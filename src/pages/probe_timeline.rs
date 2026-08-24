use super::*;
use crate::domain::Strategy;
use crate::services::probe::ProbeProgress;
use crate::ui::components::badge::Badge;
use std::ops::Range;
use std::time::Duration;

const TIMELINE_MOTION: Duration = Duration::from_millis(220);

pub(super) fn strategy_timeline(strategies: &[Strategy], progress: &ProbeProgress) -> Div {
    let mut names = strategies
        .iter()
        .map(|strategy| strategy.name.as_str())
        .collect::<Vec<_>>();
    let current = names
        .iter()
        .position(|name| *name == progress.candidate_name)
        .unwrap_or_else(|| {
            names.insert(0, progress.candidate_name.as_str());
            0
        });
    let visible = timeline_window(names.len(), current);

    div()
        .rounded(px(8.))
        .border_1()
        .border_color(accent().opacity(0.45))
        .bg(card_color())
        .p_3()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .child(progress.candidate_name.clone()),
                )
                .child(
                    Badge::new(super::probe::phase_label(progress.phase))
                        .warning()
                        .spinner(format!("probe-phase-{}", progress.category_name)),
                )
                .child(
                    div()
                        .ml_auto()
                        .text_xs()
                        .text_color(muted_foreground())
                        .child(format!(
                            "{}/{}",
                            progress.candidate_index.saturating_add(1),
                            progress.candidate_total
                        )),
                ),
        )
        .child(
            div()
                .relative()
                .h(px(62.))
                .overflow_hidden()
                .child(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .top(px(29.))
                        .h(px(1.))
                        .bg(border()),
                )
                .child(
                    div()
                        .relative()
                        .h_full()
                        .flex()
                        .justify_center()
                        .children(names[visible.clone()].iter().enumerate().map(
                            |(visible_index, name)| {
                                let index = visible.start + visible_index;
                                let active = index == current;
                                let scope = format!("{}-{name}", progress.category_name);
                                div()
                                    .w(px(120.))
                                    .h_full()
                                    .flex_none()
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .child(if index.is_multiple_of(2) {
                                        timeline_label(name, active, &scope)
                                    } else {
                                        div().h(px(20.)).into_any_element()
                                    })
                                    .child(
                                        div()
                                            .h(px(22.))
                                            .flex()
                                            .items_center()
                                            .child(timeline_dot(active, &scope)),
                                    )
                                    .child(if index.is_multiple_of(2) {
                                        div().h(px(20.)).into_any_element()
                                    } else {
                                        timeline_label(name, active, &scope)
                                    })
                            },
                        ))
                        .with_animation(
                            SharedString::from(format!(
                                "probe-timeline-window-{}-{}",
                                progress.category_name, progress.candidate_name
                            )),
                            Animation::new(TIMELINE_MOTION),
                            |strip, progress| {
                                let eased = timeline_easing(progress);
                                strip
                                    .opacity(0.55 + 0.45 * eased)
                                    .left(px(8. * (1. - eased)))
                            },
                        ),
                ),
        )
}

fn timeline_window(total: usize, current: usize) -> Range<usize> {
    let end = (current + 3).min(total);
    let start = end.saturating_sub(5).min(current.saturating_sub(2));
    start..(start + 5).min(total)
}

fn timeline_label(name: &str, active: bool, scope: &str) -> AnyElement {
    let label = div()
        .h(px(20.))
        .max_w(px(112.))
        .truncate()
        .text_xs()
        .text_center()
        .text_color(if active { accent() } else { muted_foreground() })
        .when(active, |label| label.font_weight(FontWeight::MEDIUM))
        .child(name.to_owned());
    if !active {
        return label.into_any_element();
    }
    label
        .with_animation(
            SharedString::from(format!("probe-timeline-label-{scope}-{active}")),
            Animation::new(TIMELINE_MOTION),
            |label, progress| {
                let progress = timeline_easing(progress);
                label
                    .text_color(crate::ui::foundation::motion::mix_color(
                        muted_foreground(),
                        accent(),
                        progress,
                    ))
                    .opacity(0.72 + 0.28 * progress)
            },
        )
        .into_any_element()
}

fn timeline_dot(active: bool, scope: &str) -> AnyElement {
    let dot = div().rounded_full().border_1();
    if !active {
        return dot
            .size(px(6.))
            .border_color(border())
            .bg(card_color())
            .into_any_element();
    }
    dot.with_animation(
        SharedString::from(format!("probe-timeline-dot-{scope}")),
        Animation::new(TIMELINE_MOTION),
        |dot, progress| {
            let progress = timeline_easing(progress);
            dot.size(px(6. + 4. * progress))
                .border_color(crate::ui::foundation::motion::mix_color(
                    border(),
                    accent(),
                    progress,
                ))
                .bg(crate::ui::foundation::motion::mix_color(
                    card_color(),
                    accent(),
                    progress,
                ))
        },
    )
    .into_any_element()
}

fn timeline_easing(progress: f32) -> f32 {
    let progress = progress.clamp(0.0, 1.0);
    1.0 - (1.0 - progress).powi(4)
}

#[cfg(test)]
mod tests {
    use super::{timeline_easing, timeline_window};

    #[test]
    fn timeline_keeps_current_strategy_in_a_five_item_window() {
        assert_eq!(timeline_window(40, 0), 0..5);
        assert_eq!(timeline_window(40, 20), 18..23);
        assert_eq!(timeline_window(40, 39), 35..40);
    }

    #[test]
    fn timeline_motion_reaches_both_endpoints() {
        assert_eq!(timeline_easing(0.0), 0.0);
        assert_eq!(timeline_easing(1.0), 1.0);
    }
}
