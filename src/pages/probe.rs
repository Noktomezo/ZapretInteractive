use super::*;
use crate::app_state::StrategyProbeState;
use crate::services::probe::{ProbeMode, ProbePhase};

const SUPPORTED_CATEGORIES: [&str; 6] = [
    "HTTP",
    "YouTube",
    "TCP",
    "QUIC",
    "Discord + Stun",
    "Discord Media",
];

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
        let actions = div()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_end()
            .gap_2()
            .child(
                Button::new("probe-profiles", t!("probe.profiles"), cx)
                    .outline()
                    .icon_prefix("icons/folder-open.svg")
                    .disabled(running)
                    .on_click(move |_, _, cx| {
                        state.update(cx, |state, cx| state.open_probe_profiles_directory(cx));
                    }),
            );
        let actions = actions.child(primary_action);

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

        let description = if let Some(progress) = progress {
            format!(
                "{} · {}",
                t!(
                    "probe.progress_summary",
                    category = progress.category_index.saturating_add(1),
                    categories = progress.category_total,
                    strategy = progress.candidate_name.as_str(),
                    current = progress.candidate_index.saturating_add(1),
                    total = progress.candidate_total
                ),
                phase_label(progress.phase)
            )
        } else if let Some((report, recommendations)) = report {
            let (passed, total) = super::probe_results::category_counts(report);
            let recommendation = recommendations
                .iter()
                .find(|recommendation| recommendation.category_id == category.id);
            recommendation.map_or_else(
                || {
                    rust_i18n::t!("probe.no_best_strategy", passed = passed, total = total)
                        .to_string()
                },
                |recommendation| {
                    rust_i18n::t!(
                        "probe.best_strategy",
                        name = recommendation.strategy_name.as_str(),
                        passed = passed,
                        total = total
                    )
                    .to_string()
                },
            )
        } else if let Some(message) = error {
            message.to_owned()
        } else {
            if supported {
                t!("probe.supported").to_string()
            } else {
                t!("probe.unsupported").to_string()
            }
        };

        let category_id_quick = category.id.clone();
        let state_quick = self.state.clone();
        let quick = Button::new(
            SharedString::from(format!("probe-category-quick-{}", category.id)),
            t!("probe.quick"),
            cx,
        )
        .outline()
        .icon_prefix("icons/flask-round.svg")
        .disabled(running || !supported)
        .on_click(move |_, _, cx| {
            state_quick.update(cx, |state, cx| {
                state.start_strategy_probe(vec![category_id_quick.clone()], ProbeMode::Quick, cx)
            });
        });

        let category_id_full = category.id.clone();
        let state_full = self.state.clone();
        let full = Button::new(
            SharedString::from(format!("probe-category-full-{}", category.id)),
            t!("probe.full_test"),
            cx,
        )
        .outline()
        .icon_prefix("icons/flask-conical.svg")
        .disabled(running || !supported)
        .on_click(move |_, _, cx| {
            state_full.update(cx, |state, cx| {
                state.start_strategy_probe(vec![category_id_full.clone()], ProbeMode::Full, cx)
            });
        });

        let mut header_actions = div().flex().items_center().gap_2();

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

        header_actions = header_actions.child(quick).child(full);

        if let Some((_, recommendations)) = report
            && let Some(recommendation) = recommendations
                .iter()
                .find(|recommendation| recommendation.category_id == category.id)
        {
            let is_already_applied = category
                .strategies
                .iter()
                .find(|strategy| strategy.active)
                .map(|strategy| strategy.id.as_str())
                == recommendation.strategy_id.as_deref();

            let category_id = category.id.clone();
            let strategy_id = recommendation.strategy_id.clone();
            let state = self.state.clone();
            let apply_best_btn = if is_already_applied {
                Button::new(
                    SharedString::from(format!("applied-best-probe-{}", category.id)),
                    t!("probe.applied"),
                    cx,
                )
                .secondary()
                .disabled(true)
                .icon_prefix("icons/check.svg")
            } else {
                Button::new(
                    SharedString::from(format!("apply-best-probe-{}", category.id)),
                    t!("probe.apply_best"),
                    cx,
                )
                .primary()
                .icon_prefix("icons/check.svg")
                .on_click(move |_, _, cx| {
                    state.update(cx, |state, cx| {
                        state.apply_strategy_probe_choice(&category_id, strategy_id.as_deref(), cx);
                    });
                })
            };
            header_actions = header_actions.child(apply_best_btn);
        }

        let body = (has_body && reveal > 0.001).then(|| {
            let body = if let Some(progress) = progress {
                super::probe_results::progress_body(progress, &category.strategies)
            } else if let Some((report, recommendations)) = report {
                let candidates = super::probe_results::category_counts(report).1;
                let list_state = self
                    .probe_results_list_states
                    .entry(category.id.clone())
                    .or_insert_with(|| {
                        ListState::new(candidates, ListAlignment::Top, px(240.)).measure_all()
                    })
                    .clone();
                if list_state.item_count() != candidates {
                    list_state.reset(candidates);
                    let _measurement_task = list_state.clone().measure_all();
                }
                super::probe_results::category_report_body(
                    report,
                    recommendations,
                    list_state,
                    self.state.clone(),
                )
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
