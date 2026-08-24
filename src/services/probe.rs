use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use anyhow::{Context as _, Result, bail};
use serde::{Deserialize, Serialize};

use crate::domain::{
    AppConfig, ProbeCandidateResult, ProbeProfile, ProbeRole, ProbeTargetResult,
    candidate_is_valid, passing_baseline_controls, rank_candidates,
};
use crate::services::RuntimeServices;

mod candidate;
mod http;
mod storage;
mod support;
use candidate::{candidate_progress, test_candidate};
use http::discover_youtube_ggc;
pub use storage::{clear_recovery_journal, load_recovery_journal, report_path};
use storage::{write_journal, write_json_replace};
use support::{
    candidate_name, category_profile_path, classify_profile_from_baseline, ensure_not_cancelled,
    reclassify_candidate, resolve_strategies_dir, set_category_strategy,
};

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
    pub targets: Vec<ProbeTargetProgress>,
}

#[derive(Clone, Debug)]
pub struct ProbeTargetProgress {
    pub target_id: String,
    pub target_name: String,
    pub target_url: String,
    pub expected_protocol: crate::domain::ProbeProtocol,
    pub result: Option<ProbeTargetResult>,
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
    #[serde(default)]
    pub verification_urls: Vec<String>,
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
        runtime_dir,
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
    runtime_dir: &Path,
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
    let mut verification_urls = Vec::new();

    for (category_index, category_id) in request.category_ids.iter().enumerate() {
        ensure_not_cancelled(cancelled)?;
        let category = working
            .categories
            .iter()
            .find(|category| &category.id == category_id)
            .cloned()
            .with_context(|| format!("категория {category_id} не найдена"))?;
        let profile_path = category_profile_path(&strategies_dir, runtime_dir, &category.name)?;
        let mut profile = ProbeProfile::load(&profile_path)?;
        if profile.discover_youtube_ggc
            && !profile
                .targets
                .iter()
                .any(|target| target.id == "youtube-local-ggc")
            && let Some(target) = discover_youtube_ggc(&curl, &profile)
        {
            profile.targets.push(target);
        }

        let baseline_progress = candidate_progress(
            &profile,
            request.mode == ProbeMode::Full,
            ProbeProgress {
                category_name: category.name.clone(),
                candidate_name: candidate_name(&category, None),
                category_index,
                category_total: request.category_ids.len(),
                candidate_index: 0,
                candidate_total: category.strategies.len() + 1,
                phase: ProbePhase::Baseline,
                targets: Vec::new(),
            },
        );
        let baseline_full = test_candidate(
            &curl,
            runtime,
            &working,
            &profile,
            category_id,
            None,
            request.mode == ProbeMode::Full,
            1,
            cancelled,
            baseline_progress,
            on_progress,
        )?;
        classify_profile_from_baseline(&mut profile, &baseline_full);
        let baseline_full = reclassify_candidate(baseline_full, &profile);
        for target in profile
            .targets
            .iter()
            .filter(|target| target.role == ProbeRole::Required)
        {
            if !verification_urls.contains(&target.url) {
                verification_urls.push(target.url.clone());
            }
        }

        let mut candidate_ids = Vec::with_capacity(category.strategies.len() + 1);
        candidate_ids.push(None);
        candidate_ids.extend(
            category
                .strategies
                .iter()
                .map(|strategy| Some(strategy.id.clone())),
        );
        let baseline_smoke = if request.mode == ProbeMode::Full {
            let progress = candidate_progress(
                &profile,
                false,
                ProbeProgress {
                    category_name: category.name.clone(),
                    candidate_name: candidate_name(&category, None),
                    category_index,
                    category_total: request.category_ids.len(),
                    candidate_index: 0,
                    candidate_total: candidate_ids.len(),
                    phase: ProbePhase::Smoke,
                    targets: Vec::new(),
                },
            );
            reclassify_candidate(
                test_candidate(
                    &curl,
                    runtime,
                    &working,
                    &profile,
                    category_id,
                    None,
                    false,
                    1,
                    cancelled,
                    progress,
                    on_progress,
                )?,
                &profile,
            )
        } else {
            baseline_full.clone()
        };
        let mut smoke_results = Vec::with_capacity(candidate_ids.len());
        smoke_results.push(baseline_smoke);

        for (candidate_index, strategy_id) in candidate_ids.iter().enumerate().skip(1) {
            let name = candidate_name(&category, strategy_id.as_deref());
            let progress = candidate_progress(
                &profile,
                false,
                ProbeProgress {
                    category_name: category.name.clone(),
                    candidate_name: name.clone(),
                    category_index,
                    category_total: request.category_ids.len(),
                    candidate_index,
                    candidate_total: candidate_ids.len(),
                    phase: ProbePhase::Smoke,
                    targets: Vec::new(),
                },
            );
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
                progress,
                on_progress,
            )?;
            result.strategy_name = name;
            result.strategy_id.clone_from(strategy_id);
            smoke_results.push(reclassify_candidate(result, &profile));
        }

        let baseline_controls = passing_baseline_controls(&smoke_results[0]);
        let ranked = rank_candidates(&smoke_results, &baseline_controls);
        let finalist_count = ranked.len().min(3);
        if finalist_count == 0 {
            category_reports.push(ProbeCategoryReport {
                category_id: category_id.clone(),
                category_name: category.name,
                candidates: smoke_results,
            });
            continue;
        }
        let finalist_controls = if request.mode == ProbeMode::Full {
            passing_baseline_controls(&baseline_full)
        } else {
            baseline_controls.clone()
        };
        let mut finalists = Vec::with_capacity(finalist_count);
        for (finalist_index, smoke_index) in ranked.into_iter().take(finalist_count).enumerate() {
            ensure_not_cancelled(cancelled)?;
            let strategy_id = candidate_ids[smoke_index].as_deref();
            let name = candidate_name(&category, strategy_id);
            let progress = candidate_progress(
                &profile,
                request.mode == ProbeMode::Full,
                ProbeProgress {
                    category_name: category.name.clone(),
                    candidate_name: name.clone(),
                    category_index,
                    category_total: request.category_ids.len(),
                    candidate_index: finalist_index,
                    candidate_total: finalist_count,
                    phase: ProbePhase::Finalists,
                    targets: Vec::new(),
                },
            );
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
                progress,
                on_progress,
            )?;
            result.strategy_name = name;
            result.strategy_id = strategy_id.map(str::to_owned);
            finalists.push(reclassify_candidate(result, &profile));
        }

        let ranked_finalists = rank_candidates(&finalists, &finalist_controls);
        let Some(&winner_index) = ranked_finalists.first() else {
            category_reports.push(ProbeCategoryReport {
                category_id: category_id.clone(),
                category_name: category.name,
                candidates: smoke_results,
            });
            continue;
        };
        let winner = finalists[winner_index].clone();
        let progress = candidate_progress(
            &profile,
            request.mode == ProbeMode::Full,
            ProbeProgress {
                category_name: category.name.clone(),
                candidate_name: winner.strategy_name.clone(),
                category_index,
                category_total: request.category_ids.len(),
                candidate_index: 0,
                candidate_total: 1,
                phase: ProbePhase::Verifying,
                targets: Vec::new(),
            },
        );
        let verified = reclassify_candidate(
            test_candidate(
                &curl,
                runtime,
                &working,
                &profile,
                category_id,
                winner.strategy_id.as_deref(),
                request.mode == ProbeMode::Full,
                profile.verification_repeats,
                cancelled,
                progress,
                on_progress,
            )?,
            &profile,
        );
        let verify_controls = ProbeCandidateResult {
            strategy_id: winner.strategy_id.clone(),
            strategy_name: winner.strategy_name.clone(),
            attempts: verified.attempts.clone(),
        };
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
        if !candidate_is_valid(&verify_controls, &finalist_controls) {
            category_reports.push(ProbeCategoryReport {
                category_id: category_id.clone(),
                category_name: category.name,
                candidates: smoke_results,
            });
            continue;
        }

        set_category_strategy(&mut working, category_id, winner.strategy_id.as_deref())?;
        let recommendation = ProbeRecommendation {
            category_id: category_id.clone(),
            category_name: category.name.clone(),
            strategy_id: winner.strategy_id.clone(),
            strategy_name: winner.strategy_name.clone(),
            candidates_tested: category.strategies.len(),
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
        verification_urls,
    })
}
