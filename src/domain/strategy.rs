use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::model::{Category, Strategy};

pub fn load_strategies_from_dir(dir: &Path) -> Result<Vec<Strategy>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut strategies = Vec::new();
    let entries =
        fs::read_dir(dir).with_context(|| format!("Failed to read directory {}", dir.display()))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("toml") {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read strategy file {}", path.display()))?;
            match toml::from_str::<Strategy>(&content) {
                Ok(mut strategy) => {
                    if strategy.id.trim().is_empty()
                        && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                    {
                        strategy.id = stem.to_string();
                    }
                    if strategy.category_id.is_empty() {
                        strategy.category_id = slugify(&strategy.category);
                    }
                    strategies.push(strategy);
                }
                Err(error) => {
                    eprintln!(
                        "Warning: failed to parse strategy from {}: {error}",
                        path.display()
                    );
                }
            }
        } else if path.is_dir() {
            // Also support subdirectories (e.g. strategies/http/v1.toml)
            let mut sub_strategies = load_strategies_from_dir(&path)?;
            strategies.append(&mut sub_strategies);
        }
    }

    strategies.sort_by(|a, b| {
        let cat_ord_a = a.category_order.unwrap_or(999);
        let cat_ord_b = b.category_order.unwrap_or(999);
        cat_ord_a
            .cmp(&cat_ord_b)
            .then_with(|| a.category.cmp(&b.category))
            .then_with(|| a.order.unwrap_or(999).cmp(&b.order.unwrap_or(999)))
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(strategies)
}

pub fn group_strategies_into_categories(strategies: &[Strategy]) -> Vec<Category> {
    let mut categories: Vec<Category> = Vec::new();

    for strategy in strategies {
        let cat_id = if strategy.category_id.is_empty() {
            slugify(&strategy.category)
        } else {
            strategy.category_id.clone()
        };
        let cat_name = if strategy.category.is_empty() {
            cat_id.clone()
        } else {
            strategy.category.clone()
        };

        if let Some(category) = categories.iter_mut().find(|c| c.id == cat_id) {
            category.strategies.push(strategy.clone());
            if strategy.system {
                category.system = true;
            }
        } else {
            categories.push(Category {
                id: cat_id,
                name: cat_name,
                strategies: vec![strategy.clone()],
                system: strategy.system,
                system_base_name: None,
            });
        }
    }

    categories
}

pub fn strategy_filename(strategy: &Strategy) -> String {
    let safe_id = slugify(&strategy.id);
    format!("{safe_id}.toml")
}

pub fn save_strategy_to_file(dir: &Path, strategy: &Strategy) -> Result<PathBuf> {
    fs::create_dir_all(dir)
        .with_context(|| format!("Failed to create strategies directory {}", dir.display()))?;
    let file_name = strategy_filename(strategy);
    let target_path = dir.join(&file_name);

    let toml_str = toml::to_string_pretty(strategy)
        .with_context(|| format!("Failed to serialize strategy {}", strategy.id))?;

    fs::write(&target_path, toml_str)
        .with_context(|| format!("Failed to write strategy file {}", target_path.display()))?;

    Ok(target_path)
}

pub fn delete_strategy_from_file(dir: &Path, strategy_id: &str) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    let direct_file = dir.join(format!("{}.toml", slugify(strategy_id)));
    if direct_file.is_file() {
        fs::remove_file(&direct_file)
            .with_context(|| format!("Failed to delete {}", direct_file.display()))?;
        return Ok(());
    }

    // Search recursively if file name is different from ID
    let entries =
        fs::read_dir(dir).with_context(|| format!("Failed to read directory {}", dir.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("toml") {
            if let Ok(content) = fs::read_to_string(&path)
                && let Ok(strat) = toml::from_str::<Strategy>(&content)
                && strat.id == strategy_id
            {
                fs::remove_file(&path)
                    .with_context(|| format!("Failed to delete {}", path.display()))?;
                return Ok(());
            }
        } else if path.is_dir() {
            delete_strategy_from_file(&path, strategy_id)?;
        }
    }

    Ok(())
}

pub fn sync_builtin_strategies(dir: &Path, builtin_categories: &[Category]) -> Result<()> {
    fs::create_dir_all(dir)
        .with_context(|| format!("Failed to create strategies dir {}", dir.display()))?;

    for (cat_idx, category) in builtin_categories.iter().enumerate() {
        for (strat_idx, strategy) in category.strategies.iter().enumerate() {
            let mut s = strategy.clone();
            if s.category.is_empty() {
                s.category = category.name.clone();
            }
            if s.category_id.is_empty() {
                s.category_id = category.id.clone();
            }
            s.category_order = Some((cat_idx + 1) as i32);
            s.order = Some((strat_idx + 1) as i32);
            s.system = true;

            let path = dir.join(strategy_filename(&s));
            if !path.exists() {
                save_strategy_to_file(dir, &s)?;
            }
        }
    }
    Ok(())
}

fn slugify(input: &str) -> String {
    let mut slug = String::with_capacity(input.len());
    let mut last_dash = true;

    for c in input.chars() {
        if c.is_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
            last_dash = false;
        } else if (c == '-' || c == '_' || c.is_whitespace()) && !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }

    if slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() {
        "strategy".to_string()
    } else {
        slug
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_toml_roundtrip() {
        let strategy = Strategy {
            id: "youtube-v15".to_string(),
            name: "v15".to_string(),
            category: "YouTube".to_string(),
            category_id: "preset-youtube".to_string(),
            category_order: Some(2),
            order: Some(15),
            description: Some("Multi-split TLS".to_string()),
            content: "--filter-l7=tls\n--dpi-desync=fake".to_string(),
            active: true,
            system: true,
            system_base_name: Some("v15".to_string()),
            system_base_content: Some("--filter-l7=tls\n--dpi-desync=fake".to_string()),
        };

        let serialized = toml::to_string_pretty(&strategy).unwrap();
        assert!(serialized.contains("category = \"YouTube\""));
        assert!(serialized.contains("id = \"youtube-v15\""));

        let deserialized: Strategy = toml::from_str(&serialized).unwrap();
        assert_eq!(strategy, deserialized);
    }

    #[test]
    fn test_populate_thirdparty_strategies() {
        let default_config_json = include_str!("../../assets/default-config.json");
        let parsed: serde_json::Value = serde_json::from_str(default_config_json).unwrap();
        let categories: Vec<Category> =
            serde_json::from_value(parsed["categories"].clone()).unwrap();

        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("zapret-test-strat-{unique_id}"));
        sync_builtin_strategies(&temp_dir, &categories).unwrap();

        let loaded = load_strategies_from_dir(&temp_dir).unwrap();
        assert_eq!(loaded.len(), 49);

        let _cleanup_result = fs::remove_dir_all(&temp_dir);
    }
}
