use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};

use super::{AppConfig, Filter, Strategy};

const DEFAULT_CONFIG: &str = include_str!("../../assets/default-config.json");

#[derive(Clone)]
pub struct ConfigRepository {
    runtime_dir: PathBuf,
    resources_dir: PathBuf,
}

impl ConfigRepository {
    pub fn portable() -> Result<Self> {
        let runtime_dir = std::env::current_exe()
            .context("не удалось определить путь ZapretInteractive.exe")?
            .parent()
            .context("у ZapretInteractive.exe нет родительского каталога")?
            .to_path_buf();
        let packaged_resources = runtime_dir.join("resources");
        let packaged_thirdparty = runtime_dir.join("thirdparty");
        let resources_dir = if packaged_resources.is_dir() {
            packaged_resources
        } else if packaged_thirdparty.is_dir() {
            packaged_thirdparty
        } else if let Ok(cwd) = std::env::current_dir() {
            if cwd.join("resources").is_dir() {
                cwd.join("resources")
            } else {
                cwd.join("thirdparty")
            }
        } else {
            packaged_resources
        };
        Ok(Self {
            resources_dir,
            runtime_dir,
        })
    }

    pub fn resources_dir(&self) -> &Path {
        &self.resources_dir
    }

    pub fn config_path(&self) -> PathBuf {
        self.runtime_dir.join("config.json")
    }

    pub fn filters_dir(&self) -> PathBuf {
        self.resources_dir.join("filters")
    }

    pub fn strategies_dir(&self) -> PathBuf {
        let packaged = self.resources_dir.join("strategies");
        if packaged.is_dir() {
            packaged
        } else if self.runtime_dir.join("strategies").is_dir() {
            self.runtime_dir.join("strategies")
        } else if let Ok(cwd) = std::env::current_dir() {
            if cwd.join("thirdparty").join("strategies").is_dir() {
                cwd.join("thirdparty").join("strategies")
            } else if cwd.join("resources").join("strategies").is_dir() {
                cwd.join("resources").join("strategies")
            } else {
                packaged
            }
        } else {
            packaged
        }
    }

    pub fn builtin(&self) -> Result<AppConfig> {
        let mut config: AppConfig =
            serde_json::from_str(DEFAULT_CONFIG).context("встроенный config.json повреждён")?;
        config.binaries_path = self.resources_dir.to_string_lossy().into_owned();
        config.system_sync_initialized = true;

        let strat_dir = self.strategies_dir();
        let loaded_strategies = super::strategy::load_strategies_from_dir(&strat_dir)?;
        if loaded_strategies.is_empty() {
            super::strategy::sync_builtin_strategies(&strat_dir, &config.categories)
                .context("не удалось записать встроенные стратегии")?;
        } else {
            config.categories =
                super::strategy::group_strategies_into_categories(&loaded_strategies);
        }

        annotate_system_items(&mut config, &self.filters_dir())?;
        Ok(config)
    }

    pub fn load_or_create(&self) -> Result<AppConfig> {
        let path = self.config_path();
        if !path.exists() {
            let config = self.builtin()?;
            self.save(&config)?;
            return Ok(config);
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("не удалось прочитать {}", path.display()))?;
        let mut config: AppConfig = serde_json::from_str(&content)
            .with_context(|| format!("некорректный JSON в {}", path.display()))?;
        let migrate_embedded_strategies = config
            .categories
            .iter()
            .any(|category| !category.strategies.is_empty());
        config.binaries_path = self.resources_dir.to_string_lossy().into_owned();

        let strat_dir = self.strategies_dir();
        let loaded_strategies = super::strategy::load_strategies_from_dir(&strat_dir)?;
        if !loaded_strategies.is_empty() {
            let mut file_categories =
                super::strategy::group_strategies_into_categories(&loaded_strategies);
            file_categories.retain(|cat| !config.system_removed_category_ids.contains(&cat.id));
            for cat in &mut file_categories {
                let cat_id = &cat.id;
                cat.strategies.retain(|s| {
                    let key = format!("{cat_id}::{}", s.id);
                    !config.system_removed_strategy_keys.contains(&key)
                });
            }
            // Reapply active states and ordering from existing config if available
            for cat in &mut file_categories {
                if let Some(existing_cat) = config.categories.iter().find(|c| c.id == cat.id) {
                    for strat in &mut cat.strategies {
                        if let Some(existing_strat) =
                            existing_cat.strategies.iter().find(|s| s.id == strat.id)
                        {
                            strat.active = existing_strat.active;
                        }
                    }
                }
            }
            config.categories = file_categories;
        } else if !config.categories.is_empty() {
            super::strategy::sync_builtin_strategies(&strat_dir, &config.categories)
                .context("не удалось экспортировать стратегии из config.json")?;
        }

        if migrate_embedded_strategies {
            if !loaded_strategies.is_empty() {
                for strategy in config
                    .categories
                    .iter()
                    .flat_map(|category| &category.strategies)
                {
                    self.save_strategy(strategy).with_context(|| {
                        format!(
                            "не удалось вынести стратегию {} из config.json",
                            strategy.id
                        )
                    })?;
                }
            }
            self.save(&config)
                .context("не удалось удалить встроенные стратегии из config.json")?;
        }

        Ok(config)
    }

    pub fn save(&self, config: &AppConfig) -> Result<()> {
        let path = self.config_path();
        let temporary = path.with_extension("json.tmp");
        let backup = path.with_extension("json.bak");
        let content =
            serde_json::to_vec_pretty(config).context("не удалось сериализовать конфиг")?;
        fs::write(&temporary, content)
            .with_context(|| format!("не удалось записать {}", temporary.display()))?;

        if path.exists() {
            fs::copy(&path, &backup)
                .with_context(|| format!("не удалось создать {}", backup.display()))?;
            fs::remove_file(&path)
                .with_context(|| format!("не удалось заменить {}", path.display()))?;
        }
        if let Err(error) = fs::rename(&temporary, &path) {
            if backup.exists() {
                fs::copy(&backup, &path).with_context(|| {
                    format!("не удалось восстановить backup для {}", path.display())
                })?;
            }
            bail!("не удалось сохранить {}: {error}", path.display());
        }
        Ok(())
    }

    pub fn reset(&self) -> Result<AppConfig> {
        let config = self.builtin()?;
        self.save(&config)?;
        Ok(config)
    }

    pub fn repair_filter_files(&self, config: &AppConfig) -> Result<Vec<String>> {
        let mut restored = Vec::new();
        for filter in &config.filters {
            validate_filter_filename(&filter.filename)?;
            let path = self.filters_dir().join(&filter.filename);
            let healthy = fs::read(&path).is_ok_and(|bytes| bytes == filter.content.as_bytes());
            if healthy {
                continue;
            }
            self.save_filter(filter)?;
            restored.push(format!("filters/{}", filter.filename));
        }
        Ok(restored)
    }

    pub fn save_strategy(&self, strategy: &Strategy) -> Result<PathBuf> {
        super::strategy::save_strategy_to_file(&self.strategies_dir(), strategy)
    }

    pub fn delete_strategy(&self, strategy_id: &str) -> Result<()> {
        super::strategy::delete_strategy_from_file(&self.strategies_dir(), strategy_id)
    }

    pub fn save_filter(&self, filter: &Filter) -> Result<()> {
        validate_filter_filename(&filter.filename)?;
        fs::create_dir_all(self.filters_dir()).context("не удалось создать папку filters")?;
        fs::write(self.filters_dir().join(&filter.filename), &filter.content)
            .with_context(|| format!("не удалось сохранить фильтр {}", filter.filename))
    }

    pub fn delete_filter(&self, filename: &str) -> Result<()> {
        validate_filter_filename(filename)?;
        let path = self.filters_dir().join(filename);
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => {
                Err(error).with_context(|| format!("не удалось удалить {}", path.display()))
            }
        }
    }
}

fn annotate_system_items(config: &mut AppConfig, filters_dir: &Path) -> Result<()> {
    for category in &mut config.categories {
        category.system = true;
        category.system_base_name = Some(category.name.clone());
        for strategy in &mut category.strategies {
            strategy.system = true;
            strategy.system_base_name = Some(strategy.name.clone());
            strategy.system_base_content = Some(strategy.content.clone());
        }
    }
    for placeholder in &mut config.placeholders {
        placeholder.system = true;
        placeholder.system_base_name = Some(placeholder.name.clone());
        placeholder.system_base_path = Some(placeholder.path.clone());
    }
    for filter in &mut config.filters {
        let filter_path = filters_dir.join(&filter.filename);
        let content = fs::read_to_string(&filter_path)
            .with_context(|| format!("не удалось прочитать {}", filter_path.display()))?;
        filter.content = content.clone();
        filter.system = true;
        filter.system_base_name = Some(filter.name.clone());
        filter.system_base_filename = Some(filter.filename.clone());
        filter.system_base_content = Some(content);
        filter.system_base_active = Some(filter.active);
    }
    Ok(())
}

fn validate_filter_filename(filename: &str) -> Result<()> {
    let path = Path::new(filename);
    if filename.is_empty()
        || path.components().count() != 1
        || path.extension().and_then(|value| value.to_str()) != Some("txt")
    {
        bail!("имя фильтра должно быть безопасным .txt-файлом");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_filename_cannot_escape_resources() {
        assert!(validate_filter_filename("custom.txt").is_ok());
        assert!(validate_filter_filename("list-general.txt").is_ok());
        assert!(validate_filter_filename("..\\config.json").is_err());
        assert!(validate_filter_filename("foo/bar.txt").is_err());
        assert!(validate_filter_filename("").is_err());
        assert!(validate_filter_filename("file.exe").is_err());
    }

    #[test]
    fn test_app_config_defaults_and_json_roundtrip() {
        let mut config: AppConfig =
            serde_json::from_str(DEFAULT_CONFIG).expect("valid default config json");
        assert!(config.categories.is_empty());
        assert!(!config.placeholders.is_empty());
        assert!(config.acrylic_material);

        config.categories.push(
            serde_json::from_value(serde_json::json!({
                "id": "custom",
                "name": "Custom",
                "strategies": [{
                    "id": "custom-v1",
                    "name": "v1",
                    "content": "--filter-tcp=443"
                }]
            }))
            .expect("test category is valid"),
        );

        let json_str = serde_json::to_string(&config).expect("failed to serialize default config");
        assert!(!json_str.contains("\"strategies\""));
        let deserialized: AppConfig =
            serde_json::from_str(&json_str).expect("failed to deserialize config");
        assert_eq!(deserialized.categories.len(), config.categories.len());
        assert!(
            deserialized
                .categories
                .iter()
                .all(|category| category.strategies.is_empty())
        );
        assert_eq!(deserialized.placeholders.len(), config.placeholders.len());
        assert_eq!(deserialized.acrylic_material, config.acrylic_material);
    }

    #[test]
    fn legacy_config_moves_strategies_to_toml_files() {
        let root = std::env::temp_dir().join(format!(
            "zapret_strategy_migration_{}",
            uuid::Uuid::new_v4().simple()
        ));
        let repository = ConfigRepository {
            runtime_dir: root.clone(),
            resources_dir: root.join("thirdparty"),
        };
        fs::create_dir_all(root.join("thirdparty/strategies"))
            .expect("temporary strategy directory is created");
        let mut legacy: serde_json::Value =
            serde_json::from_str(DEFAULT_CONFIG).expect("valid default config json");
        legacy["categories"] = serde_json::json!([{
            "id": "HTTP",
            "name": "HTTP",
            "strategies": [{
                "id": "http-v1",
                "name": "v1",
                "content": "--filter-l7=http",
                "active": true
            }]
        }]);
        fs::write(
            repository.config_path(),
            serde_json::to_vec_pretty(&legacy).expect("legacy config is serialized"),
        )
        .expect("legacy config is written");

        let loaded = repository
            .load_or_create()
            .expect("legacy config is migrated");
        assert!(!loaded.categories.is_empty());
        assert!(repository.strategies_dir().join("HTTP").is_dir());
        assert!(
            super::super::strategy::load_strategies_from_dir(&repository.strategies_dir())
                .expect("migrated strategies are readable")
                .iter()
                .any(|strategy| strategy.id == "http-v1" && strategy.active)
        );
        let saved = fs::read_to_string(repository.config_path()).expect("config is readable");
        assert!(!saved.contains("\"strategies\""));

        let _cleanup_result = fs::remove_dir_all(root);
    }

    #[test]
    fn configured_filter_files_are_restored_from_config() {
        let root =
            std::env::temp_dir().join(format!("zapret_filters_{}", uuid::Uuid::new_v4().simple()));
        let repository = ConfigRepository {
            runtime_dir: root.clone(),
            resources_dir: root.join("thirdparty"),
        };
        let mut config: AppConfig =
            serde_json::from_str(DEFAULT_CONFIG).expect("valid embedded config");
        config.filters.truncate(1);
        let filter = &config.filters[0];

        let restored = repository
            .repair_filter_files(&config)
            .expect("filter repair succeeds");
        assert_eq!(restored, vec![format!("filters/{}", filter.filename)]);
        assert_eq!(
            fs::read_to_string(repository.filters_dir().join(&filter.filename))
                .expect("restored filter is readable"),
            filter.content
        );

        let _cleanup_result = fs::remove_dir_all(root);
    }
}
