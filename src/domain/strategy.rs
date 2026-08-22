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
    format!("{}.toml", slugify(&strategy.name))
}

pub fn save_strategy_to_file(dir: &Path, strategy: &Strategy) -> Result<PathBuf> {
    let category_dir = dir.join(category_directory_name(strategy));
    fs::create_dir_all(&category_dir).with_context(|| {
        format!(
            "Failed to create strategy category directory {}",
            category_dir.display()
        )
    })?;
    let target_path = category_dir.join(strategy_filename(strategy));
    let previous_path = find_strategy_file(dir, &strategy.id)?;
    if target_path.is_file() && previous_path.as_deref() != Some(&target_path) {
        anyhow::bail!(
            "Strategy file {} is already occupied by another strategy",
            target_path.display()
        );
    }

    let toml_str = toml::to_string_pretty(strategy)
        .with_context(|| format!("Failed to serialize strategy {}", strategy.id))?;

    fs::write(&target_path, toml_str)
        .with_context(|| format!("Failed to write strategy file {}", target_path.display()))?;

    if let Some(previous_path) = previous_path
        && previous_path != target_path
    {
        fs::remove_file(&previous_path)
            .with_context(|| format!("Failed to remove {}", previous_path.display()))?;
        if let Some(parent) = previous_path.parent()
            && parent != dir
        {
            let _remove_empty_category = fs::remove_dir(parent);
        }
    }

    Ok(target_path)
}

pub fn delete_strategy_from_file(dir: &Path, strategy_id: &str) -> Result<()> {
    if let Some(path) = find_strategy_file(dir, strategy_id)? {
        fs::remove_file(&path).with_context(|| format!("Failed to delete {}", path.display()))?;
        if let Some(parent) = path.parent()
            && parent != dir
        {
            let _remove_empty_category = fs::remove_dir(parent);
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

            let path = dir
                .join(category_directory_name(&s))
                .join(strategy_filename(&s));
            if !path.exists() {
                save_strategy_to_file(dir, &s)?;
            }
        }
    }
    Ok(())
}

fn find_strategy_file(dir: &Path, strategy_id: &str) -> Result<Option<PathBuf>> {
    if !dir.is_dir() {
        return Ok(None);
    }

    let entries =
        fs::read_dir(dir).with_context(|| format!("Failed to read directory {}", dir.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_strategy_file(&path, strategy_id)? {
                return Ok(Some(found));
            }
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("toml") {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read strategy file {}", path.display()))?;
            if toml::from_str::<Strategy>(&content).is_ok_and(|item| item.id == strategy_id) {
                return Ok(Some(path));
            }
        }
    }

    Ok(None)
}

fn category_directory_name(strategy: &Strategy) -> String {
    let category = strategy.category.trim();
    let is_reserved = matches!(
        category.to_ascii_uppercase().as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    );
    let is_safe = !category.is_empty()
        && category != "."
        && category != ".."
        && !category.ends_with(['.', ' '])
        && !category
            .chars()
            .any(|character| character.is_control() || r#"<>:"/\|?*"#.contains(character));

    if is_safe && !is_reserved {
        category.to_owned()
    } else {
        slugify(&strategy.category_id)
    }
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
        let bundled_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("thirdparty")
            .join("strategies");
        let bundled = load_strategies_from_dir(&bundled_dir).unwrap();
        let categories = group_strategies_into_categories(&bundled);
        assert_eq!(
            categories
                .iter()
                .map(|category| category.name.as_str())
                .collect::<Vec<_>>(),
            [
                "HTTP",
                "YouTube",
                "TCP",
                "QUIC",
                "Discord + Stun",
                "Discord Media",
                "Game TCP",
                "Game UDP",
            ]
        );

        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("zapret-test-strat-{unique_id}"));
        sync_builtin_strategies(&temp_dir, &categories).unwrap();

        let loaded = load_strategies_from_dir(&temp_dir).unwrap();
        assert_eq!(loaded.len(), 161);
        assert!(temp_dir.join("HTTP").join("v1.toml").is_file());
        assert!(
            fs::read_dir(&temp_dir)
                .unwrap()
                .flatten()
                .all(|entry| entry.path().is_dir())
        );

        let mut moved = loaded
            .iter()
            .find(|strategy| strategy.id == "preset-http-1")
            .unwrap()
            .clone();
        moved.category = "Renamed HTTP".to_string();
        moved.name = "renamed".to_string();
        save_strategy_to_file(&temp_dir, &moved).unwrap();
        assert!(!temp_dir.join("HTTP").join("v1.toml").exists());
        assert!(temp_dir.join("Renamed HTTP").join("renamed.toml").is_file());

        moved.category = "../escape".to_string();
        moved.category_id = "safe-category".to_string();
        assert_eq!(category_directory_name(&moved), "safe-category");

        let bundled_category_dirs = fs::read_dir(&bundled_dir)
            .unwrap()
            .flatten()
            .filter(|entry| entry.path().is_dir())
            .collect::<Vec<_>>();
        assert_eq!(bundled_category_dirs.len(), 8);
        for entry in bundled_category_dirs {
            let category_name = entry.file_name().to_string_lossy().into_owned();
            assert!(
                load_strategies_from_dir(&entry.path())
                    .unwrap()
                    .iter()
                    .all(|strategy| strategy.category == category_name)
            );
        }
        assert_eq!(bundled.len(), 161);
        assert!(
            group_strategies_into_categories(&bundled)
                .iter()
                .all(|category| category
                    .strategies
                    .iter()
                    .filter(|strategy| strategy.active)
                    .count()
                    == 1)
        );

        let _cleanup_result = fs::remove_dir_all(&temp_dir);
    }
}
