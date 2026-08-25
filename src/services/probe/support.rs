use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Context as _, Result, bail};

use crate::domain::{
    AppConfig, Category, ProbeCandidateResult, ProbeOutcome, ProbeProfile, ProbeTargetResult,
};

pub(super) fn failed_candidate(
    strategy_id: Option<&str>,
    profile: &ProbeProfile,
    full: bool,
    repeats: usize,
    message: &str,
) -> ProbeCandidateResult {
    let attempts = (0..repeats)
        .flat_map(|_| profile.targets_for(full))
        .flat_map(|target| {
            profile
                .protocols
                .iter()
                .map(move |protocol| ProbeTargetResult {
                    target_id: target.id.clone(),
                    target_name: target.name.clone(),
                    target_url: target.url.clone(),
                    expected_protocol: *protocol,
                    outcome: ProbeOutcome::Fail,
                    protocol: None,
                    status_code: None,
                    bytes: 0,
                    remote_ip: None,
                    latency_ms: 0,
                    error: Some(message.to_owned()),
                })
        })
        .collect();
    ProbeCandidateResult {
        strategy_id: strategy_id.map(str::to_owned),
        strategy_name: String::new(),
        attempts,
    }
}

pub(super) fn cache_baseline_addresses(
    profile: &mut ProbeProfile,
    baseline: &ProbeCandidateResult,
) {
    for target in &mut profile.targets {
        let attempts = baseline
            .attempts
            .iter()
            .filter(|attempt| attempt.target_id == target.id)
            .collect::<Vec<_>>();
        if target.connect_ip.is_none() {
            target.connect_ip = attempts
                .iter()
                .find(|attempt| attempt.outcome == ProbeOutcome::Pass)
                .and_then(|attempt| attempt.remote_ip.clone());
        }
    }
}

pub(super) fn set_category_strategy(
    config: &mut AppConfig,
    category_id: &str,
    strategy_id: Option<&str>,
) -> Result<()> {
    let category = config
        .categories
        .iter_mut()
        .find(|category| category.id == category_id)
        .with_context(|| format!("категория {category_id} не найдена"))?;
    if let Some(strategy_id) = strategy_id
        && !category
            .strategies
            .iter()
            .any(|strategy| strategy.id == strategy_id)
    {
        bail!("стратегия {strategy_id} не найдена в {}", category.name);
    }
    for cat in &mut config.categories {
        if cat.id == category_id {
            for strategy in &mut cat.strategies {
                strategy.active = strategy_id.is_some_and(|id| strategy.id == id);
            }
        } else {
            for strategy in &mut cat.strategies {
                strategy.active = false;
            }
        }
    }
    Ok(())
}

pub(super) fn candidate_name(category: &Category, strategy_id: Option<&str>) -> String {
    strategy_id
        .and_then(|id| {
            category
                .strategies
                .iter()
                .find(|strategy| strategy.id == id)
        })
        .map(|strategy| strategy.name.clone())
        .unwrap_or_else(|| "Без стратегии".to_owned())
}

pub(super) fn resolve_strategies_dir(resources_dir: &Path) -> PathBuf {
    resources_dir.join("strategies")
}

pub(super) fn category_profile_path(
    strategies_dir: &Path,
    runtime_dir: &Path,
    category_name: &str,
) -> Result<PathBuf> {
    let path = Path::new(category_name);
    if path.components().count() != 1
        || !matches!(path.components().next(), Some(Component::Normal(_)))
    {
        bail!("небезопасное имя категории: {category_name}");
    }
    let override_profile = runtime_dir
        .join("probe-profiles")
        .join(path)
        .join("probe.toml");
    if override_profile.is_file() {
        return Ok(override_profile);
    }
    let profile = strategies_dir.join(path).join("probe.toml");
    if !profile.is_file() {
        bail!("для категории {category_name} отсутствует probe.toml");
    }
    Ok(profile)
}

pub(super) fn ensure_not_cancelled(cancelled: &AtomicBool) -> Result<()> {
    if cancelled.load(Ordering::Relaxed) {
        bail!("подбор стратегий отменён");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ProbeProtocol, ProbeTier};

    fn attempt(id: &str, outcome: ProbeOutcome, remote_ip: Option<&str>) -> ProbeTargetResult {
        ProbeTargetResult {
            target_id: id.to_owned(),
            target_name: id.to_owned(),
            target_url: format!("https://{id}.example"),
            expected_protocol: ProbeProtocol::Auto,
            outcome,
            protocol: Some("2".to_owned()),
            status_code: Some(200),
            bytes: 69_632,
            remote_ip: remote_ip.map(str::to_owned),
            latency_ms: 10,
            error: None,
        }
    }

    #[test]
    fn baseline_caches_working_ip() {
        let mut profile = ProbeProfile {
            version: 1,
            protocols: vec![ProbeProtocol::Auto],
            startup_delay_ms: 350,
            timeout_ms: 5_000,
            parallel_targets: 4,
            download_bytes: 69_632,
            verification_repeats: 1,
            follow_redirects: true,
            impersonate: crate::domain::ProbeImpersonation::Chrome150,
            doh_url: None,
            discover_youtube_ggc: false,
            targets: vec![
                crate::domain::ProbeTarget {
                    id: "open".to_owned(),
                    name: "Open".to_owned(),
                    url: "https://example.com".to_owned(),
                    _legacy_role: None,
                    tier: ProbeTier::Smoke,
                    min_bytes: 0,
                    connect_ip: None,
                },
                crate::domain::ProbeTarget {
                    id: "blocked".to_owned(),
                    name: "Blocked".to_owned(),
                    url: "https://blocked.example".to_owned(),
                    _legacy_role: None,
                    tier: ProbeTier::Smoke,
                    min_bytes: 0,
                    connect_ip: None,
                },
            ],
        };
        let baseline = ProbeCandidateResult {
            strategy_id: None,
            strategy_name: "Без стратегии".to_owned(),
            attempts: vec![
                attempt("open", ProbeOutcome::Pass, Some("192.0.2.1")),
                attempt("blocked", ProbeOutcome::Fail, None),
            ],
        };

        cache_baseline_addresses(&mut profile, &baseline);

        assert_eq!(profile.targets[0].connect_ip.as_deref(), Some("192.0.2.1"));
        assert_eq!(profile.targets[1].connect_ip, None);
    }

    #[test]
    fn set_category_strategy_isolates_active_strategy() {
        use crate::domain::Strategy;

        let mut config: AppConfig =
            serde_json::from_str(include_str!("../../../assets/default-config.json")).unwrap();
        let make_strat = |id: &str, active: bool| Strategy {
            id: id.to_owned(),
            name: id.to_owned(),
            category: "Test".to_owned(),
            category_id: "test".to_owned(),
            category_order: None,
            order: None,
            description: None,
            content: String::new(),
            active,
            system: false,
            system_base_name: None,
            system_base_content: None,
        };
        config.categories = vec![
            Category {
                id: "cat1".to_owned(),
                name: "Cat1".to_owned(),
                strategies: vec![make_strat("cat1-s1", true), make_strat("cat1-s2", false)],
                system: false,
                system_base_name: None,
            },
            Category {
                id: "cat2".to_owned(),
                name: "Cat2".to_owned(),
                strategies: vec![make_strat("cat2-s1", true)],
                system: false,
                system_base_name: None,
            },
        ];

        set_category_strategy(&mut config, "cat1", Some("cat1-s2")).unwrap();
        assert!(!config.categories[0].strategies[0].active);
        assert!(config.categories[0].strategies[1].active);
        assert!(!config.categories[1].strategies[0].active);

        set_category_strategy(&mut config, "cat1", None).unwrap();
        assert!(!config.categories[0].strategies[0].active);
        assert!(!config.categories[0].strategies[1].active);
        assert!(!config.categories[1].strategies[0].active);
    }
}
