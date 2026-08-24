use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Context as _, Result, bail};
use serde::{Deserialize, Serialize};

use crate::domain::{
    AppConfig, ProbeCandidateResult, ProbeOutcome, ProbeProfile, ProbeRole,
    candidate_preserves_controls, passing_baseline_controls, rank_candidates,
};
use crate::services::RuntimeServices;

mod http;
mod storage;
use http::run_targets;
pub use storage::{clear_recovery_journal, load_recovery_journal, report_path};
use storage::{write_journal, write_json_replace};

const CURL_RELATIVE_PATH: &str = "modules/curl-impersonate/curl-impersonate.exe";

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProbeMode {
    Quick,
    Full,
}

#[derive(Clone, Debug)]
pub struct ProbeRequest {
    pub category_ids: Vec<String>,
    pub mode: ProbeMode,
    pub was_connected: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ProbeProgress {
    pub category_name: String,
    pub candidate_name: String,
    pub category_index: usize,
    pub category_total: usize,
    pub candidate_index: usize,
    pub candidate_total: usize,
    pub phase: ProbePhase,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProbePhase {
    #[default]
    Preparing,
    Baseline,
    Smoke,
    Finalists,
    Verifying,
    Restoring,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeRecommendation {
    pub category_id: String,
    pub category_name: String,
    pub strategy_id: Option<String>,
    pub strategy_name: String,
    pub candidates_tested: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeReport {
    pub mode: ProbeMode,
    pub recommendations: Vec<ProbeRecommendation>,
    pub categories: Vec<ProbeCategoryReport>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeCategoryReport {
    pub category_id: String,
    pub category_name: String,
    pub candidates: Vec<ProbeCandidateResult>,
}

pub fn run_strategy_probe(
    resources_dir: &Path,
    runtime_dir: &Path,
    runtime: &RuntimeServices,
    original: &AppConfig,
    request: &ProbeRequest,
    cancelled: Arc<AtomicBool>,
    on_progress: impl Fn(ProbeProgress),
) -> Result<ProbeReport> {
    write_journal(runtime_dir, request.was_connected)?;
    let result = run_probe_inner(
        resources_dir,
        runtime,
        original,
        request,
        cancelled.as_ref(),
        &on_progress,
    );

    on_progress(ProbeProgress {
        phase: ProbePhase::Restoring,
        ..ProbeProgress::default()
    });
    let restore = if request.was_connected {
        runtime.connect(original).map(|_| ())
    } else {
        runtime.disconnect()
    };
    match (result, restore) {
        (Ok(report), Ok(())) => {
            write_json_replace(&report_path(runtime_dir), &report)?;
            clear_recovery_journal(runtime_dir)?;
            Ok(report)
        }
        (Err(error), Ok(())) => {
            clear_recovery_journal(runtime_dir)?;
            Err(error)
        }
        (Ok(_), Err(restore)) => Err(restore).context("не удалось восстановить подключение"),
        (Err(error), Err(restore)) => Err(anyhow::anyhow!(
            "{error:#}; восстановление подключения также не удалось: {restore:#}"
        )),
    }
}

fn run_probe_inner(
    resources_dir: &Path,
    runtime: &RuntimeServices,
    original: &AppConfig,
    request: &ProbeRequest,
    cancelled: &AtomicBool,
    on_progress: &impl Fn(ProbeProgress),
) -> Result<ProbeReport> {
    let curl = resources_dir.join(CURL_RELATIVE_PATH);
    if !curl.is_file() {
        bail!("curl-impersonate не найден: {}", curl.display());
    }
    let strategies_dir = resolve_strategies_dir(resources_dir);
    let mut working = original.clone();
    working.dns_module_enabled = false;
    working.tg_ws_proxy_module_enabled = false;
    working.discord_presence_enabled = false;
    let mut recommendations = Vec::new();
    let mut category_reports = Vec::new();

    for (category_index, category_id) in request.category_ids.iter().enumerate() {
        ensure_not_cancelled(cancelled)?;
        let category = working
            .categories
            .iter()
            .find(|category| &category.id == category_id)
            .cloned()
            .with_context(|| format!("категория {category_id} не найдена"))?;
        let profile_path = category_profile_path(&strategies_dir, &category.name)?;
        let profile = ProbeProfile::load(&profile_path)?;
        let mut candidate_ids = Vec::with_capacity(category.strategies.len() + 1);
        candidate_ids.push(None);
        candidate_ids.extend(
            category
                .strategies
                .iter()
                .map(|strategy| Some(strategy.id.clone())),
        );
        let mut smoke_results = Vec::with_capacity(candidate_ids.len());

        for (candidate_index, strategy_id) in candidate_ids.iter().enumerate() {
            let name = candidate_name(&category, strategy_id.as_deref());
            on_progress(ProbeProgress {
                category_name: category.name.clone(),
                candidate_name: name.clone(),
                category_index,
                category_total: request.category_ids.len(),
                candidate_index,
                candidate_total: candidate_ids.len(),
                phase: if candidate_index == 0 {
                    ProbePhase::Baseline
                } else {
                    ProbePhase::Smoke
                },
            });
            let mut result = test_candidate(
                &curl,
                runtime,
                &working,
                &profile,
                category_id,
                strategy_id.as_deref(),
                false,
                1,
                cancelled,
            )?;
            result.strategy_name = name;
            result.strategy_id.clone_from(strategy_id);
            smoke_results.push(result);
        }

        let baseline_controls = passing_baseline_controls(&smoke_results[0]);
        let ranked = rank_candidates(&smoke_results, &baseline_controls);
        let finalist_count = ranked.len().min(3);
        let finalist_controls = if request.mode == ProbeMode::Full {
            let baseline = test_candidate(
                &curl,
                runtime,
                &working,
                &profile,
                category_id,
                None,
                true,
                1,
                cancelled,
            )?;
            passing_baseline_controls(&baseline)
        } else {
            baseline_controls.clone()
        };
        let mut finalists = Vec::with_capacity(finalist_count);
        for (finalist_index, smoke_index) in ranked.into_iter().take(finalist_count).enumerate() {
            ensure_not_cancelled(cancelled)?;
            let strategy_id = candidate_ids[smoke_index].as_deref();
            let name = candidate_name(&category, strategy_id);
            on_progress(ProbeProgress {
                category_name: category.name.clone(),
                candidate_name: name.clone(),
                category_index,
                category_total: request.category_ids.len(),
                candidate_index: finalist_index,
                candidate_total: finalist_count,
                phase: ProbePhase::Finalists,
            });
            let mut result = test_candidate(
                &curl,
                runtime,
                &working,
                &profile,
                category_id,
                strategy_id,
                request.mode == ProbeMode::Full,
                3,
                cancelled,
            )?;
            result.strategy_name = name;
            result.strategy_id = strategy_id.map(str::to_owned);
            finalists.push(result);
        }

        let ranked_finalists = rank_candidates(&finalists, &finalist_controls);
        let winner_index = *ranked_finalists
            .first()
            .context("не удалось выбрать стратегию")?;
        let winner = finalists[winner_index].clone();
        on_progress(ProbeProgress {
            category_name: category.name.clone(),
            candidate_name: winner.strategy_name.clone(),
            category_index,
            category_total: request.category_ids.len(),
            candidate_index: 0,
            candidate_total: 1,
            phase: ProbePhase::Verifying,
        });
        let verified = test_candidate(
            &curl,
            runtime,
            &working,
            &profile,
            category_id,
            winner.strategy_id.as_deref(),
            request.mode == ProbeMode::Full,
            1,
            cancelled,
        )?;
        let verify_controls = ProbeCandidateResult {
            strategy_id: winner.strategy_id.clone(),
            strategy_name: winner.strategy_name.clone(),
            attempts: verified.attempts.clone(),
        };
        if !candidate_preserves_controls(&verify_controls, &finalist_controls) {
            bail!(
                "финальная проверка {} нарушила контрольные цели",
                winner.strategy_name
            );
        }

        set_category_strategy(&mut working, category_id, winner.strategy_id.as_deref())?;
        for finalist in finalists {
            if let Some(candidate) = smoke_results
                .iter_mut()
                .find(|candidate| candidate.strategy_id == finalist.strategy_id)
            {
                candidate.attempts.extend(finalist.attempts);
                if candidate.strategy_id == winner.strategy_id {
                    candidate.attempts.extend(verified.attempts.clone());
                }
            }
        }
        let recommendation = ProbeRecommendation {
            category_id: category_id.clone(),
            category_name: category.name.clone(),
            strategy_id: winner.strategy_id.clone(),
            strategy_name: winner.strategy_name.clone(),
            candidates_tested: candidate_ids.len(),
        };
        recommendations.push(recommendation);
        category_reports.push(ProbeCategoryReport {
            category_id: category_id.clone(),
            category_name: category.name,
            candidates: smoke_results,
        });
    }

    Ok(ProbeReport {
        mode: request.mode,
        recommendations,
        categories: category_reports,
    })
}

#[allow(clippy::too_many_arguments)]
fn test_candidate(
    curl: &Path,
    runtime: &RuntimeServices,
    base: &AppConfig,
    profile: &ProbeProfile,
    category_id: &str,
    strategy_id: Option<&str>,
    full: bool,
    repeats: usize,
    cancelled: &AtomicBool,
) -> Result<ProbeCandidateResult> {
    ensure_not_cancelled(cancelled)?;
    let mut config = base.clone();
    set_category_strategy(&mut config, category_id, strategy_id)?;
    if let Err(error) = runtime.connect(&config) {
        if strategy_id.is_none() {
            return Err(error).context("не удалось запустить базовую проверку");
        }
        return Ok(failed_candidate(
            strategy_id,
            profile,
            full,
            repeats,
            &format!("winws не запустился: {error:#}"),
        ));
    }
    std::thread::sleep(Duration::from_millis(profile.startup_delay_ms));
    if !runtime.winws_running()? {
        if strategy_id.is_none() {
            bail!("winws завершился при запуске базовой проверки");
        }
        return Ok(failed_candidate(
            strategy_id,
            profile,
            full,
            repeats,
            "winws завершился при запуске тестовой стратегии",
        ));
    }

    let mut attempts = Vec::new();
    for _ in 0..repeats {
        ensure_not_cancelled(cancelled)?;
        let mut current = run_targets(curl, profile, full, cancelled);
        let all_required_failed = current
            .iter()
            .filter(|result| result.role == ProbeRole::Required)
            .all(|result| result.outcome == ProbeOutcome::Fail);
        if all_required_failed && !cancelled.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(650));
            current = run_targets(curl, profile, full, cancelled);
        }
        attempts.extend(current);
    }

    Ok(ProbeCandidateResult {
        strategy_id: strategy_id.map(str::to_owned),
        strategy_name: String::new(),
        attempts,
    })
}

fn failed_candidate(
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
                .map(move |_| crate::domain::ProbeTargetResult {
                    target_id: target.id.clone(),
                    role: target.role,
                    outcome: ProbeOutcome::Fail,
                    protocol: None,
                    status_code: None,
                    bytes: 0,
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

fn set_category_strategy(
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
    for strategy in &mut category.strategies {
        strategy.active = strategy_id.is_some_and(|id| strategy.id == id);
    }
    Ok(())
}

fn candidate_name(category: &crate::domain::Category, strategy_id: Option<&str>) -> String {
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

fn resolve_strategies_dir(resources_dir: &Path) -> PathBuf {
    resources_dir.join("strategies")
}

fn category_profile_path(strategies_dir: &Path, category_name: &str) -> Result<PathBuf> {
    let path = Path::new(category_name);
    if path.components().count() != 1
        || !matches!(path.components().next(), Some(Component::Normal(_)))
    {
        bail!("небезопасное имя категории: {category_name}");
    }
    let profile = strategies_dir.join(path).join("probe.toml");
    if !profile.is_file() {
        bail!("для категории {category_name} отсутствует probe.toml");
    }
    Ok(profile)
}

fn ensure_not_cancelled(cancelled: &AtomicBool) -> Result<()> {
    if cancelled.load(Ordering::Relaxed) {
        bail!("подбор стратегий отменён");
    }
    Ok(())
}
