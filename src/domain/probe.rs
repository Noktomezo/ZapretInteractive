use std::cmp::Ordering;
use std::collections::{BTreeSet, HashSet};
use std::path::Path;

use anyhow::{Context as _, Result, bail};
use serde::{Deserialize, Serialize};
use url::Url;

fn default_startup_delay_ms() -> u64 {
    350
}

fn default_timeout_ms() -> u64 {
    5_000
}

fn default_parallel_targets() -> usize {
    4
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum ProbeProtocol {
    #[serde(rename = "http/1.1")]
    Http11,
    #[serde(rename = "h2")]
    Http2,
    #[serde(rename = "h3")]
    Http3,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProbeRole {
    Required,
    Optional,
    Control,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProbeTier {
    Smoke,
    Full,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProbeProfile {
    pub version: u32,
    pub protocols: Vec<ProbeProtocol>,
    #[serde(default = "default_startup_delay_ms")]
    pub startup_delay_ms: u64,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default = "default_parallel_targets")]
    pub parallel_targets: usize,
    pub targets: Vec<ProbeTarget>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProbeTarget {
    pub id: String,
    pub name: String,
    pub url: String,
    pub role: ProbeRole,
    pub tier: ProbeTier,
}

impl ProbeProfile {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("не удалось прочитать {}", path.display()))?;
        let profile: Self = toml::from_str(&content)
            .with_context(|| format!("некорректный профиль {}", path.display()))?;
        profile.validate()?;
        Ok(profile)
    }

    pub fn validate(&self) -> Result<()> {
        if self.version != 1 {
            bail!("неподдерживаемая версия probe.toml: {}", self.version);
        }
        if self.protocols.is_empty() {
            bail!("probe.toml не содержит protocols");
        }
        if self.parallel_targets == 0 || self.parallel_targets > 16 {
            bail!("parallelTargets должен быть в диапазоне 1..=16");
        }
        if !(500..=60_000).contains(&self.timeout_ms) {
            bail!("timeoutMs должен быть в диапазоне 500..=60000");
        }
        if self.targets.is_empty() {
            bail!("probe.toml не содержит targets");
        }

        let mut ids = HashSet::new();
        for target in &self.targets {
            if target.id.trim().is_empty() || target.name.trim().is_empty() {
                bail!("id и name цели не могут быть пустыми");
            }
            if !ids.insert(target.id.as_str()) {
                bail!("повторяющийся id цели: {}", target.id);
            }
            let url = Url::parse(&target.url)
                .with_context(|| format!("некорректный URL цели {}", target.id))?;
            if !matches!(url.scheme(), "http" | "https")
                || !url.username().is_empty()
                || url.password().is_some()
                || url.host_str().is_none()
            {
                bail!("недопустимый URL цели {}", target.id);
            }
        }
        if !self
            .targets
            .iter()
            .any(|target| target.role == ProbeRole::Required)
        {
            bail!("probe.toml должен содержать хотя бы одну required-цель");
        }
        Ok(())
    }

    pub fn targets_for(&self, full: bool) -> impl Iterator<Item = &ProbeTarget> {
        self.targets
            .iter()
            .filter(move |target| full || target.tier == ProbeTier::Smoke)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProbeOutcome {
    Pass,
    Degraded,
    Fail,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeTargetResult {
    pub target_id: String,
    pub role: ProbeRole,
    pub outcome: ProbeOutcome,
    pub protocol: Option<String>,
    pub status_code: Option<u16>,
    pub bytes: u64,
    pub latency_ms: u128,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeCandidateResult {
    pub strategy_id: Option<String>,
    pub strategy_name: String,
    pub attempts: Vec<ProbeTargetResult>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProbeCandidateScore {
    pub controls_ok: bool,
    pub stable_required: usize,
    pub optional_passes: usize,
    pub unstable_attempts: usize,
    pub median_latency_ms: u128,
}

impl ProbeCandidateScore {
    fn for_candidate(
        candidate: &ProbeCandidateResult,
        baseline_controls: &BTreeSet<String>,
    ) -> Self {
        let controls_ok = candidate_preserves_controls(candidate, baseline_controls);

        let required_ids = candidate
            .attempts
            .iter()
            .filter(|attempt| attempt.role == ProbeRole::Required)
            .map(|attempt| attempt.target_id.as_str())
            .collect::<BTreeSet<_>>();
        let stable_required = required_ids
            .iter()
            .filter(|id| {
                candidate
                    .attempts
                    .iter()
                    .filter(|attempt| attempt.target_id == ***id)
                    .all(|attempt| attempt.outcome == ProbeOutcome::Pass)
            })
            .count();
        let optional_passes = candidate
            .attempts
            .iter()
            .filter(|attempt| {
                attempt.role == ProbeRole::Optional && attempt.outcome == ProbeOutcome::Pass
            })
            .count();
        let unstable_attempts = candidate
            .attempts
            .iter()
            .filter(|attempt| {
                attempt.role != ProbeRole::Control && attempt.outcome != ProbeOutcome::Pass
            })
            .count();
        let mut latencies = candidate
            .attempts
            .iter()
            .filter(|attempt| attempt.outcome == ProbeOutcome::Pass)
            .map(|attempt| attempt.latency_ms)
            .collect::<Vec<_>>();
        latencies.sort_unstable();
        let median_latency_ms = latencies
            .get(latencies.len().saturating_sub(1) / 2)
            .copied()
            .unwrap_or(u128::MAX);

        Self {
            controls_ok,
            stable_required,
            optional_passes,
            unstable_attempts,
            median_latency_ms,
        }
    }
}

pub fn candidate_preserves_controls(
    candidate: &ProbeCandidateResult,
    baseline_controls: &BTreeSet<String>,
) -> bool {
    baseline_controls.iter().all(|id| {
        let attempts = candidate
            .attempts
            .iter()
            .filter(|attempt| attempt.target_id == *id)
            .collect::<Vec<_>>();
        !attempts.is_empty()
            && attempts
                .iter()
                .all(|attempt| attempt.outcome == ProbeOutcome::Pass)
    })
}

pub fn passing_baseline_controls(candidate: &ProbeCandidateResult) -> BTreeSet<String> {
    candidate
        .attempts
        .iter()
        .filter(|attempt| {
            attempt.role == ProbeRole::Control && attempt.outcome == ProbeOutcome::Pass
        })
        .map(|attempt| attempt.target_id.clone())
        .collect()
}

pub fn rank_candidates(
    candidates: &[ProbeCandidateResult],
    baseline_controls: &BTreeSet<String>,
) -> Vec<usize> {
    let mut indices = (0..candidates.len()).collect::<Vec<_>>();
    indices.sort_by(|left, right| {
        let left_score = ProbeCandidateScore::for_candidate(&candidates[*left], baseline_controls);
        let right_score =
            ProbeCandidateScore::for_candidate(&candidates[*right], baseline_controls);
        compare_scores(right_score, left_score).then_with(|| {
            candidates[*left]
                .strategy_name
                .cmp(&candidates[*right].strategy_name)
        })
    });
    indices
}

fn compare_scores(left: ProbeCandidateScore, right: ProbeCandidateScore) -> Ordering {
    left.controls_ok
        .cmp(&right.controls_ok)
        .then(left.stable_required.cmp(&right.stable_required))
        .then(left.optional_passes.cmp(&right.optional_passes))
        .then_with(|| right.unstable_attempts.cmp(&left.unstable_attempts))
        .then_with(|| right.median_latency_ms.cmp(&left.median_latency_ms))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(name: &str, attempts: &[(ProbeRole, ProbeOutcome, u128)]) -> ProbeCandidateResult {
        ProbeCandidateResult {
            strategy_id: Some(name.to_owned()),
            strategy_name: name.to_owned(),
            attempts: attempts
                .iter()
                .enumerate()
                .map(|(index, (role, outcome, latency_ms))| ProbeTargetResult {
                    target_id: if *role == ProbeRole::Control {
                        "control".to_owned()
                    } else {
                        format!("target-{index}")
                    },
                    role: *role,
                    outcome: *outcome,
                    protocol: Some("2".to_owned()),
                    status_code: Some(200),
                    bytes: 1,
                    latency_ms: *latency_ms,
                    error: None,
                })
                .collect(),
        }
    }

    #[test]
    fn controls_and_required_stability_dominate_latency() {
        let baseline = BTreeSet::from(["control".to_owned()]);
        let candidates = vec![
            result(
                "fast-but-breaks-control",
                &[
                    (ProbeRole::Control, ProbeOutcome::Fail, 1),
                    (ProbeRole::Required, ProbeOutcome::Pass, 1),
                ],
            ),
            result(
                "stable",
                &[
                    (ProbeRole::Control, ProbeOutcome::Pass, 50),
                    (ProbeRole::Required, ProbeOutcome::Pass, 50),
                ],
            ),
        ];
        assert_eq!(rank_candidates(&candidates, &baseline), vec![1, 0]);
    }

    #[test]
    fn no_strategy_can_win_as_a_normal_candidate() {
        let baseline = BTreeSet::new();
        let mut no_strategy = result(
            "Без стратегии",
            &[(ProbeRole::Required, ProbeOutcome::Pass, 10)],
        );
        no_strategy.strategy_id = None;
        let candidates = vec![
            result("v1", &[(ProbeRole::Required, ProbeOutcome::Degraded, 5)]),
            no_strategy,
        ];
        assert_eq!(rank_candidates(&candidates, &baseline), vec![1, 0]);
    }

    #[test]
    fn bundled_probe_profiles_are_valid() {
        for source in [
            include_str!("../../thirdparty/strategies/HTTP/probe.toml"),
            include_str!("../../thirdparty/strategies/TCP/probe.toml"),
            include_str!("../../thirdparty/strategies/YouTube/probe.toml"),
            include_str!("../../thirdparty/strategies/QUIC/probe.toml"),
        ] {
            let profile: ProbeProfile = toml::from_str(source).expect("bundled profile parses");
            profile.validate().expect("bundled profile validates");
        }
    }
}
