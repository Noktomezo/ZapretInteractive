use super::*;
use crate::app_state::StrategyProbeState;
use crate::services::probe::{ProbeMode, ProbePhase};
use crate::ui::components::badge::Badge;

const SUPPORTED_CATEGORIES: [&str; 4] = ["HTTP", "YouTube", "TCP", "QUIC"];

impl AppView {
    pub(crate) fn strategy_probe_page(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let (categories, probe_state) = {
            let state = self.state.read(cx);
            (
                state.config.categories.clone(),
                state.strategy_probe.clone(),
            )
        };
        let running = matches!(probe_state, StrategyProbeState::Running(_));
        let complete = matches!(probe_state, StrategyProbeState::Complete(_));
        let supported_ids = SUPPORTED_CATEGORIES
            .iter()
            .filter_map(|name| {
                categories
                    .iter()
                    .find(|category| category.name == *name)
                    .map(|category| category.id.clone())
            })
            .collect::<Vec<_>>();

        let state = self.state.clone();
        let full_button = Button::new("probe-all", t!("probe.full"), cx)
            .icon_prefix("icons/flask-conical.svg")
            .disabled(running || supported_ids.is_empty())
            .on_click(move |_, _, cx| {
                state.update(cx, |state, cx| {
                    state.start_strategy_probe(supported_ids.clone(), ProbeMode::Full, cx)
                });
            });
        let primary_action = if running {
            let state = self.state.clone();
            Button::new("cancel-probe", t!("probe.cancel"), cx)
                .destructive()
                .icon_prefix("icons/square-stop.svg")
                .on_click(move |_, _, cx| {
                    state.update(cx, |state, cx| state.cancel_strategy_probe(cx));
                })
                .into_any_element()
        } else if complete {
            full_button.outline().into_any_element()
        } else {
            full_button.primary().into_any_element()
        };

        let state = self.state.clone();
        let mut actions = div().flex().items_center().gap_2().child(
            Button::new("probe-profiles", t!("probe.profiles"), cx)
                .outline()
                .icon_prefix("icons/folder-open.svg")
                .disabled(running)
                .on_click(move |_, _, cx| {
                    state.update(cx, |state, cx| state.open_probe_profiles_directory(cx));
                }),
        );
        if let StrategyProbeState::Complete(report) = &probe_state {
            let verification_urls = report.verification_urls.clone();
            let state = self.state.clone();
            actions = actions.child(
                Button::new("verify-probe", t!("probe.verify_browser"), cx)
                    .outline()
                    .icon_prefix("icons/external-link.svg")
                    .disabled(verification_urls.is_empty())
                    .on_click(move |_, _, cx| {
                        state.update(cx, |state, cx| {
                            state.open_probe_verification_urls(&verification_urls, cx)
                        });
                    }),
            );
            let state = self.state.clone();
            actions = actions.child(
                Button::new("apply-probe", t!("probe.apply"), cx)
                    .primary()
                    .icon_prefix("icons/check.svg")
                    .disabled(report.recommendations.is_empty())
                    .on_click(move |_, _, cx| {
                        state.update(cx, |state, cx| state.apply_strategy_probe_report(cx));
                    }),
            );
        }
        actions = actions.child(primary_action);

        let mut content = div().flex().flex_col().gap_3();
        for category in &categories {
            let supported = SUPPORTED_CATEGORIES.contains(&category.name.as_str());
            let running_progress = match &probe_state {
                StrategyProbeState::Running(progress)
                    if progress.category_name == category.name =>
                {
                    Some(progress)
                }
                _ => None,
            };
            let error = match &probe_state {
                StrategyProbeState::Error { message, progress }
                    if progress.category_name == category.name =>
                {
                    Some(message.as_str())
                }
                _ => None,
            };
            let category_report = match &probe_state {
                StrategyProbeState::Complete(report) => report
                    .categories
                    .iter()
                    .find(|item| item.category_id == category.id)
                    .map(|item| (item, report.recommendations.as_slice())),
                _ => None,
            };
            content = content.child(self.probe_category_card(
                category,
                supported,
                running,
                running_progress,
                error,
                category_report,
                cx,
            ));
        }

        page_with_actions(t!("probe.title"), actions, content)
    }

    #[allow(clippy::too_many_arguments)]
    fn probe_category_card(
        &mut self,
        category: &crate::domain::Category,
        supported: bool,
        running: bool,
        progress: Option<&crate::services::probe::ProbeProgress>,
        error: Option<&str>,
        report: Option<(
            &crate::services::probe::ProbeCategoryReport,
            &[crate::services::probe::ProbeRecommendation],
        )>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let disclosure_id: SharedString = format!("probe-disclosure-{}", category.id).into();
        let has_body = progress.is_some() || error.is_some() || report.is_some();
        let forced_open = progress.is_some() || error.is_some();
        let expanded =
            forced_open || self.probe_expanded_category.as_deref() == Some(category.id.as_str());
        let reveal = disclosure_progress(&disclosure_id, expanded, cx);

        let (description, status) = if let Some(progress) = progress {
            (
                t!(
                    "probe.progress_summary",
                    category = progress.category_index.saturating_add(1),
                    categories = progress.category_total,
                    strategy = progress.candidate_name.as_str(),
                    current = progress.candidate_index.saturating_add(1),
                    total = progress.candidate_total
                )
                .to_string(),
                Some(
                    Badge::new(phase_label(progress.phase))
                        .warning()
                        .spinner(format!("probe-header-{}", category.id))
                        .into_any_element(),
                ),
            )
        } else if let Some((report, recommendations)) = report {
            let (passed, total) = super::probe_results::category_counts(report);
            let recommendation = recommendations
                .iter()
                .find(|recommendation| recommendation.category_id == category.id);
            let description = recommendation.map_or_else(
                || t!("probe.no_recommendation").to_string(),
                |recommendation| {
                    format!(
                        "{}: {}",
                        t!("probe.recommendation"),
                        recommendation.strategy_name
                    )
                },
            );
            let badge = Badge::new(format!("{passed}/{total}"))
                .fade_in(format!("probe-result-{}", category.id));
            (
                description,
                Some(if recommendation.is_some() {
                    badge.success().into_any_element()
                } else {
                    badge.destructive().into_any_element()
                }),
            )
        } else if let Some(message) = error {
            (
                message.to_owned(),
                Some(
                    Badge::new(t!("probe.outcome_fail"))
                        .destructive()
                        .into_any_element(),
                ),
            )
        } else {
            (
                if supported {
                    t!("probe.supported").to_string()
                } else {
                    t!("probe.unsupported").to_string()
                },
                None,
            )
        };

        let category_id = category.id.clone();
        let state = self.state.clone();
        let quick = Button::new(
            SharedString::from(format!("probe-category-{}", category.id)),
            t!("probe.quick"),
            cx,
        )
        .outline()
        .icon_prefix("icons/flask-round.svg")
        .disabled(running || !supported)
        .on_click(move |_, _, cx| {
            state.update(cx, |state, cx| {
                state.start_strategy_probe(vec![category_id.clone()], ProbeMode::Quick, cx)
            });
        });

        let mut header_actions = div()
            .flex()
            .items_center()
            .gap_2()
            .children(status)
            .child(quick);
        if has_body {
            let category_id = category.id.clone();
            header_actions = header_actions.child(
                DisclosureChevron::new(disclosure_id, expanded, cx).on_click(cx.listener(
                    move |this, _, _, cx| {
                        if this.probe_expanded_category.as_deref() == Some(category_id.as_str()) {
                            this.probe_expanded_category = None;
                        } else {
                            this.probe_expanded_category = Some(category_id.clone());
                        }
                        cx.notify();
                    },
                )),
            );
        }

        let body = (has_body && reveal > 0.001).then(|| {
            let body = if let Some(progress) = progress {
                super::probe_results::progress_body(progress)
            } else if let Some((report, recommendations)) = report {
                super::probe_results::category_report_body(report, recommendations)
            } else {
                super::probe_results::error_body(error.unwrap_or_default())
            };
            div()
                .overflow_hidden()
                .opacity(reveal)
                .mt(px(-8. * (1. - reveal)))
                .child(body)
        });

        module_card(
            module_header(
                (
                    if supported {
                        "icons/flask-conical.svg"
                    } else {
                        "icons/flask-conical-off.svg"
                    },
                    if supported {
                        accent()
                    } else {
                        muted_foreground()
                    },
                ),
                category.name.clone(),
                description,
                Some(header_actions.into_any_element()),
                body.is_some(),
            ),
            body,
        )
        .into_any_element()
    }
}

pub(super) fn phase_label(phase: ProbePhase) -> SharedString {
    match phase {
        ProbePhase::Preparing => t!("probe.phase_preparing"),
        ProbePhase::Baseline => t!("probe.phase_baseline"),
        ProbePhase::Smoke => t!("probe.phase_smoke"),
        ProbePhase::Finalists => t!("probe.phase_finalists"),
        ProbePhase::Verifying => t!("probe.phase_verifying"),
        ProbePhase::Restoring => t!("probe.phase_restoring"),
    }
    .into()
}
