use super::*;
use crate::app_state::StrategyProbeState;
use crate::services::probe::{ProbeMode, ProbePhase};

const SUPPORTED_CATEGORIES: [&str; 4] = ["HTTP", "YouTube", "TCP", "QUIC"];

impl AppView {
    pub(crate) fn strategy_probe_page(&self, cx: &mut Context<Self>) -> AnyElement {
        let (categories, probe_state) = {
            let state = self.state.read(cx);
            (
                state.config.categories.clone(),
                state.strategy_probe.clone(),
            )
        };
        let running = matches!(probe_state, StrategyProbeState::Running(_));
        let supported_ids = SUPPORTED_CATEGORIES
            .iter()
            .filter_map(|name| {
                categories
                    .iter()
                    .find(|category| category.name == *name)
                    .map(|category| category.id.clone())
            })
            .collect::<Vec<_>>();

        let primary_action = if running {
            let state = self.state.clone();
            Button::new("cancel-probe", t!("probe.cancel"), cx)
                .destructive()
                .icon_prefix("icons/square-stop.svg")
                .on_click(move |_, _, cx| {
                    state.update(cx, |state, cx| state.cancel_strategy_probe(cx));
                })
                .into_any_element()
        } else {
            let state = self.state.clone();
            Button::new("probe-all", t!("probe.full"), cx)
                .primary()
                .icon_prefix("icons/flask-conical.svg")
                .disabled(supported_ids.is_empty())
                .on_click(move |_, _, cx| {
                    state.update(cx, |state, cx| {
                        state.start_strategy_probe(supported_ids.clone(), ProbeMode::Full, cx)
                    });
                })
                .into_any_element()
        };
        let state = self.state.clone();
        let actions = div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                Button::new("probe-profiles", t!("probe.profiles"), cx)
                    .outline()
                    .icon_prefix("icons/folder-open.svg")
                    .disabled(running)
                    .on_click(move |_, _, cx| {
                        state.update(cx, |state, cx| state.open_probe_profiles_directory(cx));
                    }),
            )
            .child(primary_action);

        let mut content = div().flex().flex_col().gap_3();
        if let StrategyProbeState::Running(progress) = &probe_state {
            content = content.child(status_card(
                "icons/refresh-cw.svg",
                t!("probe.running"),
                progress_text(progress),
                accent(),
            ));
        } else if let StrategyProbeState::Error(error) = &probe_state {
            content = content.child(status_card(
                "icons/circle-alert.svg",
                t!("probe.failed"),
                error.clone(),
                danger(),
            ));
        }

        for category in &categories {
            let supported = SUPPORTED_CATEGORIES.contains(&category.name.as_str());
            let category_id = category.id.clone();
            let state = self.state.clone();
            let action = Button::new(
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
            content = content.child(module_card(
                div()
                    .min_h(px(72.))
                    .px_4()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(card_icon(
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
                    ))
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(div().text_sm().child(category.name.clone()))
                            .child(div().text_xs().text_color(muted_foreground()).child(
                                if supported {
                                    t!("probe.supported").to_string()
                                } else {
                                    t!("probe.unsupported").to_string()
                                },
                            )),
                    )
                    .child(action),
                None,
            ));
        }

        if let StrategyProbeState::Complete(report) = probe_state {
            content = content.child(super::probe_results::probe_report_card(
                report,
                self.state.clone(),
                cx,
            ));
        }

        page_with_actions(t!("probe.title"), actions, content)
    }
}

fn progress_text(progress: &crate::services::probe::ProbeProgress) -> String {
    let phase = match progress.phase {
        ProbePhase::Preparing => t!("probe.phase_preparing"),
        ProbePhase::Baseline => t!("probe.phase_baseline"),
        ProbePhase::Smoke => t!("probe.phase_smoke"),
        ProbePhase::Finalists => t!("probe.phase_finalists"),
        ProbePhase::Verifying => t!("probe.phase_verifying"),
        ProbePhase::Restoring => t!("probe.phase_restoring"),
    };
    if progress.category_name.is_empty() {
        return phase.to_string();
    }
    format!(
        "{} · {} ({}/{}) · {} ({}/{})",
        phase,
        progress.category_name,
        progress.category_index + 1,
        progress.category_total,
        progress.candidate_name,
        progress.candidate_index + 1,
        progress.candidate_total
    )
}

fn status_card(
    icon: &'static str,
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
    color: Rgba,
) -> Div {
    let title = title.into();
    let description = description.into();
    module_card(
        div()
            .min_h(px(72.))
            .px_4()
            .flex()
            .items_center()
            .gap_3()
            .child(card_icon(icon, color))
            .child(
                div()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(title))
                    .child(
                        div()
                            .text_xs()
                            .text_color(muted_foreground())
                            .child(description),
                    ),
            ),
        None,
    )
}
