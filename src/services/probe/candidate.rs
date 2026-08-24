use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Context as _, Result, bail};

use super::http::run_targets;
use super::support::{ensure_not_cancelled, failed_candidate, set_category_strategy};
use super::{ProbeProgress, ProbeTargetProgress};
use crate::domain::{
    AppConfig, ProbeCandidateResult, ProbeOutcome, ProbeProfile, ProbeTargetResult,
};
use crate::services::RuntimeServices;

#[allow(clippy::too_many_arguments)]
pub(super) fn test_candidate(
    curl: &Path,
    runtime: &RuntimeServices,
    base: &AppConfig,
    profile: &ProbeProfile,
    category_id: &str,
    strategy_id: Option<&str>,
    full: bool,
    repeats: usize,
    cancelled: &AtomicBool,
    progress: ProbeProgress,
    on_progress: &impl Fn(ProbeProgress),
) -> Result<ProbeCandidateResult> {
    ensure_not_cancelled(cancelled)?;
    on_progress(progress.clone());
    let publish_results = |results: &[ProbeTargetResult]| {
        let mut update = progress.clone();
        for target in &mut update.targets {
            target.result = results
                .iter()
                .rev()
                .find(|result| {
                    result.target_id == target.target_id
                        && result.expected_protocol == target.expected_protocol
                })
                .cloned();
        }
        on_progress(update);
    };
    let mut config = base.clone();
    set_category_strategy(&mut config, category_id, strategy_id)?;
    if let Err(error) = runtime.connect(&config) {
        if strategy_id.is_none() {
            return Err(error).context("не удалось запустить базовую проверку");
        }
        let failed = failed_candidate(
            strategy_id,
            profile,
            full,
            repeats,
            &format!("winws не запустился: {error:#}"),
        );
        publish_results(&failed.attempts);
        return Ok(failed);
    }
    std::thread::sleep(Duration::from_millis(profile.startup_delay_ms));
    if !runtime.winws_running()? {
        if strategy_id.is_none() {
            bail!("winws завершился при запуске базовой проверки");
        }
        let failed = failed_candidate(
            strategy_id,
            profile,
            full,
            repeats,
            "winws завершился при запуске тестовой стратегии",
        );
        publish_results(&failed.attempts);
        return Ok(failed);
    }

    let mut attempts = Vec::new();
    for _ in 0..repeats {
        ensure_not_cancelled(cancelled)?;
        let mut current = run_targets(curl, profile, full, cancelled, &publish_results);
        ensure_not_cancelled(cancelled)?;
        let all_targets_failed = !current.is_empty()
            && current
                .iter()
                .all(|result| result.outcome == ProbeOutcome::Fail);
        if all_targets_failed && !cancelled.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(650));
            current = run_targets(curl, profile, full, cancelled, &publish_results);
            ensure_not_cancelled(cancelled)?;
        }
        attempts.extend(current);
    }

    Ok(ProbeCandidateResult {
        strategy_id: strategy_id.map(str::to_owned),
        strategy_name: String::new(),
        attempts,
    })
}

pub(super) fn candidate_progress(
    profile: &ProbeProfile,
    full: bool,
    mut progress: ProbeProgress,
) -> ProbeProgress {
    progress.targets = profile
        .targets_for(full)
        .flat_map(|target| {
            profile
                .protocols
                .iter()
                .copied()
                .map(move |expected_protocol| ProbeTargetProgress {
                    target_id: target.id.clone(),
                    target_name: target.name.clone(),
                    target_url: target.url.clone(),
                    expected_protocol,
                    result: None,
                })
        })
        .collect();
    progress
}
