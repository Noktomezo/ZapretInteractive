use super::*;
use crate::AppState;
use crate::domain::{ProbeCandidateResult, ProbeOutcome, ProbeProtocol, ProbeTargetResult};
use crate::services::probe::{
    ProbeCategoryReport, ProbeProgress, ProbeRecommendation, ProbeTargetProgress,
};
use crate::ui::components::badge::{Badge, BadgeVariant};
use crate::ui::components::smooth_scroll::{PageScrollbar, SmoothListScroll};

struct TargetSummary {
    target_id: String,
    target_name: String,
    target_url: String,
    protocol: ProbeProtocol,
    outcome: ProbeOutcome,
    passed_attempts: usize,
    attempts: usize,
    status_code: Option<u16>,
    actual_protocol: Option<String>,
    bytes: u64,
    latency_ms: u128,
    error: Option<String>,
}

pub(super) fn category_counts(category: &ProbeCategoryReport) -> (usize, usize) {
    let candidates = strategy_candidates(category).collect::<Vec<_>>();
    let passed = candidates
        .iter()
        .filter(|candidate| candidate_passed(candidate))
        .count();
    (passed, candidates.len())
}

pub(super) fn category_report_body(
    category: &ProbeCategoryReport,
    recommendations: &[ProbeRecommendation],
    list_state: ListState,
    state: Entity<AppState>,
) -> Div {
    let candidates = strategy_candidates(category).cloned().collect::<Vec<_>>();
    let item_count = candidates.len();
    let recommended_strategy = recommendations
        .iter()
        .find(|recommendation| recommendation.category_id == category.category_id)
        .and_then(|recommendation| recommendation.strategy_id.clone());
    let list_id: SharedString = format!("probe-results-{}", category.category_id).into();
    let scrollbar_id: SharedString =
        format!("probe-results-scrollbar-{}", category.category_id).into();
    let category_id = category.category_id.clone();
    let list_element = list(list_state.clone(), move |index, _, cx| {
        let candidate = &candidates[index];
        div()
            .w_full()
            .px_4()
            .pb_4()
            .child(candidate_results(
                candidate,
                recommended_strategy.as_deref() == candidate.strategy_id.as_deref(),
                state.clone(),
                category_id.clone(),
                cx,
            ))
            .into_any_element()
    });
    let list_height = px((item_count as f32 * 190.).clamp(190., 420.));
    div().pb_4().pt_4().child(
        div()
            .relative()
            .w_full()
            .h(list_height)
            .overflow_hidden()
            .child(SmoothListScroll::new(
                list_id,
                list_state.clone(),
                list_element.size_full(),
            ))
            .child(PageScrollbar::new(scrollbar_id, list_state)),
    )
}

pub(super) fn progress_body(
    progress: &ProbeProgress,
    strategies: &[crate::domain::Strategy],
) -> Div {
    div()
        .px_4()
        .pb_4()
        .pt_4()
        .flex()
        .flex_col()
        .gap_2()
        .child(super::probe_timeline::strategy_timeline(
            strategies, progress,
        ))
        .children(
            progress
                .targets
                .iter()
                .enumerate()
                .map(|(index, target)| progress_target(target, index)),
        )
}

pub(super) fn error_body(message: &str) -> Div {
    div()
        .p_4()
        .text_xs()
        .text_color(danger())
        .whitespace_normal()
        .child(message.to_owned())
}

fn progress_target(target: &ProbeTargetProgress, index: usize) -> AnyElement {
    let (label, variant) = match target.result.as_ref().map(|result| result.outcome) {
        Some(ProbeOutcome::Pass) => (t!("probe.outcome_pass"), BadgeVariant::Success),
        Some(ProbeOutcome::Degraded) => (t!("probe.outcome_degraded"), BadgeVariant::Warning),
        Some(ProbeOutcome::Fail) => (t!("probe.outcome_fail"), BadgeVariant::Destructive),
        None => (t!("probe.outcome_testing"), BadgeVariant::Warning),
    };
    let status = Badge::new(label).variant(variant);
    let status = if target.result.is_none() {
        status.spinner(format!("probe-target-{}-{index}", target.target_id))
    } else {
        status
    };
    div()
        .px_3()
        .py_2()
        .rounded(px(8.))
        .border_1()
        .border_color(border())
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(div().text_xs().child(target.target_name.clone()))
                        .child(status),
                )
                .child(
                    div()
                        .truncate()
                        .text_xs()
                        .text_color(muted_foreground())
                        .child(format!(
                            "{} · {}",
                            protocol_label(target.expected_protocol),
                            target.target_url
                        )),
                ),
        )
        .into_any_element()
}

fn candidate_results(
    candidate: &ProbeCandidateResult,
    recommended: bool,
    state: Entity<AppState>,
    category_id: String,
    cx: &App,
) -> AnyElement {
    let targets = summarize_targets(candidate);
    let passed = targets
        .iter()
        .filter(|target| target.outcome == ProbeOutcome::Pass)
        .count();
    let candidate_passed = !targets.is_empty() && passed == targets.len();
    let is_active = state
        .read(cx)
        .config
        .categories
        .iter()
        .find(|category| category.id == category_id)
        .and_then(|category| category.strategies.iter().find(|strategy| strategy.active))
        .map(|strategy| strategy.id.as_str())
        == candidate.strategy_id.as_deref();
    let strategy_id = candidate.strategy_id.clone();
    let apply_button = if is_active {
        Button::new(
            SharedString::from(format!(
                "applied-probe-candidate-{}-{}",
                category_id,
                strategy_id.as_deref().unwrap_or("none")
            )),
            t!("probe.applied"),
            cx,
        )
        .secondary()
        .disabled(true)
        .icon_prefix("icons/check.svg")
    } else {
        Button::new(
            SharedString::from(format!(
                "apply-probe-candidate-{}-{}",
                category_id,
                strategy_id.as_deref().unwrap_or("none")
            )),
            t!("probe.apply_strategy"),
            cx,
        )
        .outline()
        .icon_prefix("icons/check.svg")
        .on_click(move |_, _, cx| {
            state.update(cx, |state, cx| {
                state.apply_strategy_probe_choice(&category_id, strategy_id.as_deref(), cx);
            });
        })
    };

    div()
        .rounded(px(8.))
        .border_1()
        .border_color(border())
        .overflow_hidden()
        .child(
            div()
                .px_3()
                .py_2()
                .flex()
                .items_center()
                .flex_wrap()
                .gap_1()
                .child(
                    div()
                        .min_w_0()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .child(candidate.strategy_name.clone()),
                )
                .when(recommended, |header| {
                    header.child(Badge::new(t!("probe.recommendation")).success())
                })
                .child(
                    Badge::new(t!(
                        "probe.targets_summary",
                        passed = passed,
                        total = targets.len()
                    ))
                    .variant(if candidate_passed {
                        BadgeVariant::Success
                    } else {
                        BadgeVariant::Destructive
                    }),
                )
                .child(div().ml_auto().flex_none().child(apply_button)),
        )
        .child(
            div()
                .border_t_1()
                .border_color(border())
                .children(targets.into_iter().map(target_result)),
        )
        .into_any_element()
}

fn target_result(target: TargetSummary) -> AnyElement {
    let details = target_metadata(&target);
    div()
        .px_3()
        .py_2()
        .border_b_1()
        .border_color(border())
        .flex()
        .items_start()
        .justify_between()
        .gap_3()
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(div().text_xs().child(if target.target_name.is_empty() {
                            target.target_id.clone()
                        } else {
                            target.target_name.clone()
                        }))
                        .child(
                            Badge::new(outcome_label(target.outcome))
                                .variant(outcome_variant(target.outcome)),
                        ),
                )
                .when(!target.target_url.is_empty(), |this| {
                    this.child(
                        div()
                            .text_xs()
                            .text_color(muted_foreground())
                            .whitespace_normal()
                            .child(format!("{} · {}", details, target.target_url)),
                    )
                })
                .when_some(target.error.clone(), |this, error| {
                    this.child(
                        div()
                            .text_xs()
                            .text_color(danger())
                            .whitespace_normal()
                            .child(error),
                    )
                }),
        )
        .into_any_element()
}

fn strategy_candidates(
    category: &ProbeCategoryReport,
) -> impl Iterator<Item = &ProbeCandidateResult> {
    category
        .candidates
        .iter()
        .filter(|candidate| candidate.strategy_id.is_some())
}

fn candidate_passed(candidate: &ProbeCandidateResult) -> bool {
    let targets = summarize_targets(candidate);
    !targets.is_empty()
        && targets
            .iter()
            .all(|target| target.outcome == ProbeOutcome::Pass)
}

fn summarize_targets(candidate: &ProbeCandidateResult) -> Vec<TargetSummary> {
    let mut summaries = Vec::<TargetSummary>::new();
    for attempt in &candidate.attempts {
        if let Some(summary) = summaries.iter_mut().find(|summary| {
            summary.target_id == attempt.target_id && summary.protocol == attempt.expected_protocol
        }) {
            merge_attempt(summary, attempt);
        } else {
            summaries.push(TargetSummary {
                target_id: attempt.target_id.clone(),
                target_name: attempt.target_name.clone(),
                target_url: attempt.target_url.clone(),
                protocol: attempt.expected_protocol,
                outcome: attempt.outcome,
                passed_attempts: usize::from(attempt.outcome == ProbeOutcome::Pass),
                attempts: 1,
                status_code: attempt.status_code,
                actual_protocol: attempt.protocol.clone(),
                bytes: attempt.bytes,
                latency_ms: attempt.latency_ms,
                error: attempt.error.clone(),
            });
        }
    }
    summaries
}

fn merge_attempt(summary: &mut TargetSummary, attempt: &ProbeTargetResult) {
    summary.attempts += 1;
    summary.passed_attempts += usize::from(attempt.outcome == ProbeOutcome::Pass);
    summary.outcome = worse_outcome(summary.outcome, attempt.outcome);
    summary.status_code = attempt.status_code.or(summary.status_code);
    if attempt.protocol.is_some() {
        summary.actual_protocol.clone_from(&attempt.protocol);
    }
    summary.bytes = summary.bytes.max(attempt.bytes);
    summary.latency_ms += attempt.latency_ms;
    if summary.error.is_none() {
        summary.error.clone_from(&attempt.error);
    }
}

fn worse_outcome(left: ProbeOutcome, right: ProbeOutcome) -> ProbeOutcome {
    match (left, right) {
        (ProbeOutcome::Fail, _) | (_, ProbeOutcome::Fail) => ProbeOutcome::Fail,
        (ProbeOutcome::Degraded, _) | (_, ProbeOutcome::Degraded) => ProbeOutcome::Degraded,
        _ => ProbeOutcome::Pass,
    }
}

fn target_metadata(target: &TargetSummary) -> String {
    let mut parts = vec![protocol_label(target.protocol).to_owned()];
    if let Some(status_code) = target.status_code {
        parts.push(format!("HTTP {status_code}"));
    }
    if let Some(protocol) = &target.actual_protocol
        && protocol != "STUN"
        && target.protocol != ProbeProtocol::Stun
    {
        parts.push(format!("h{protocol}"));
    }
    parts.push(format!(
        "{} ms",
        target.latency_ms / target.attempts.max(1) as u128
    ));
    parts.push(format!("{}/{}", target.passed_attempts, target.attempts));
    parts.join(" · ")
}

fn protocol_label(protocol: ProbeProtocol) -> &'static str {
    match protocol {
        ProbeProtocol::Auto => "auto",
        ProbeProtocol::Http11 => "HTTP/1.1",
        ProbeProtocol::Http2 => "HTTP/2",
        ProbeProtocol::Http3 => "HTTP/3",
        ProbeProtocol::Stun => "STUN",
    }
}

fn outcome_label(outcome: ProbeOutcome) -> SharedString {
    match outcome {
        ProbeOutcome::Pass => t!("probe.outcome_pass"),
        ProbeOutcome::Degraded => t!("probe.outcome_degraded"),
        ProbeOutcome::Fail => t!("probe.outcome_fail"),
    }
    .into()
}

fn outcome_variant(outcome: ProbeOutcome) -> BadgeVariant {
    match outcome {
        ProbeOutcome::Pass => BadgeVariant::Success,
        ProbeOutcome::Degraded => BadgeVariant::Warning,
        ProbeOutcome::Fail => BadgeVariant::Destructive,
    }
}

#[cfg(test)]
mod tests {
    use super::summarize_targets;
    use crate::domain::{ProbeCandidateResult, ProbeOutcome, ProbeProtocol, ProbeTargetResult};

    #[test]
    fn repeated_target_uses_worst_outcome() {
        let result = ProbeCandidateResult {
            strategy_id: Some("v1".to_owned()),
            strategy_name: "v1".to_owned(),
            attempts: [ProbeOutcome::Pass, ProbeOutcome::Fail]
                .into_iter()
                .map(|outcome| ProbeTargetResult {
                    target_id: "example".to_owned(),
                    target_name: "Example".to_owned(),
                    target_url: "https://example.com".to_owned(),
                    expected_protocol: ProbeProtocol::Http3,
                    outcome,
                    protocol: Some("3".to_owned()),
                    status_code: Some(200),
                    bytes: 1_024,
                    remote_ip: None,
                    latency_ms: 10,
                    error: None,
                })
                .collect(),
        };

        let summary = summarize_targets(&result);
        assert_eq!(summary.len(), 1);
        assert_eq!(summary[0].outcome, ProbeOutcome::Fail);
        assert_eq!(summary[0].passed_attempts, 1);
        assert_eq!(summary[0].attempts, 2);
    }
}
