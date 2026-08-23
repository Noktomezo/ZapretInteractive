use std::path::Path;

use anyhow::{Result, bail};

use super::{AppConfig, ListMode, Placeholder};

pub fn build_winws_args(config: &AppConfig, resources_dir: &Path) -> Vec<String> {
    let list_mode = match config.list_mode {
        ListMode::Exclude => "--hostlist-exclude={{HOSTS_USER_EXCLUDE}}",
        ListMode::Ipset => "--ipset={{IP_USER}}",
    };
    let mut args = vec![
        format!("--wf-tcp={}", config.global_ports.tcp),
        format!("--wf-udp={}", config.global_ports.udp),
    ];
    args.extend(
        config
            .filters
            .iter()
            .filter(|filter| filter.active)
            .map(|filter| {
                format!(
                    "--wf-raw-part=@{}",
                    resources_dir
                        .join("filters")
                        .join(&filter.filename)
                        .display()
                )
            }),
    );

    let strategies = config
        .categories
        .iter()
        .flat_map(|category| category.strategies.iter())
        .filter(|strategy| strategy.active)
        .map(|strategy| strategy.content.replace("<LIST_MODE>", list_mode));
    for (index, strategy) in strategies.enumerate() {
        if index > 0 {
            args.push("--new".into());
        }
        let resolved = resolve_placeholders(&strategy, &config.placeholders, resources_dir);
        args.extend(
            resolved
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_owned),
        );
    }
    args
}

pub fn validate_port_spec(value: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("список портов пуст");
    }
    for part in value.split(',').map(str::trim) {
        let mut bounds = part.split('-');
        let start = bounds
            .next()
            .and_then(|value| value.parse::<u16>().ok())
            .filter(|value| *value > 0)
            .ok_or_else(|| anyhow::anyhow!("некорректный порт: {part}"))?;
        if let Some(end) = bounds.next() {
            let end = end
                .parse::<u16>()
                .ok()
                .filter(|value| *value >= start)
                .ok_or_else(|| anyhow::anyhow!("некорректный диапазон портов: {part}"))?;
            if end == 0 || bounds.next().is_some() {
                bail!("некорректный диапазон портов: {part}");
            }
        }
    }
    Ok(())
}

pub fn resolve_placeholders(
    content: &str,
    placeholders: &[Placeholder],
    resources_dir: &Path,
) -> String {
    placeholders
        .iter()
        .fold(content.to_owned(), |result, placeholder| {
            let path = if let Some(relative) = placeholder.path.strip_prefix("@resources") {
                resources_dir.join(relative.trim_start_matches(['/', '\\']))
            } else if let Some(relative) = placeholder.path.strip_prefix('~') {
                home_dir().join(relative.trim_start_matches(['/', '\\']))
            } else {
                placeholder.path.clone().into()
            };
            result.replace(
                &format!("{{{{{}}}}}", placeholder.name),
                &path.to_string_lossy(),
            )
        })
}

fn home_dir() -> std::path::PathBuf {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(Into::into)
        .unwrap_or_else(|| ".".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strategy_arguments_keep_new_boundaries_and_resolve_resources() {
        let mut config: AppConfig =
            serde_json::from_str(include_str!("../../assets/default-config.json"))
                .expect("default config is part of the build");
        let strategies = crate::domain::strategy::load_strategies_from_dir(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("thirdparty/strategies"),
        )
        .expect("bundled strategies are readable");
        config.categories = crate::domain::strategy::group_strategies_into_categories(&strategies);
        config.categories.truncate(1);
        config.categories[0].strategies.truncate(1);
        config.placeholders = vec![Placeholder {
            name: "HOSTS_USER_EXCLUDE".into(),
            path: "@resources/lists/exclude.txt".into(),
            system: false,
            system_base_name: None,
            system_base_path: None,
        }];
        config.list_mode = ListMode::Exclude;
        let args = build_winws_args(&config, Path::new("C:/app/thirdparty"));
        assert!(args.iter().any(|arg| {
            arg.replace('/', "\\")
                .contains("thirdparty\\lists\\exclude.txt")
        }));
    }

    #[test]
    fn port_specs_reject_reversed_and_zero_ranges() {
        assert!(validate_port_spec("80,443,1000-2000").is_ok());
        assert!(validate_port_spec("2000-1000").is_err());
        assert!(validate_port_spec("0").is_err());
    }
}
