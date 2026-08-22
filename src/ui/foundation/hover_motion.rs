use std::collections::HashMap;
use std::time::{Duration, Instant};

use gpui::{App, Global, SharedString, Window};

use super::motion::CONTROL_MOTION;

#[derive(Clone, Copy)]
struct HoverTransition {
    hovered: bool,
    from: f32,
    changed_at: Instant,
}

#[derive(Default)]
struct HoverMotionState {
    transitions: HashMap<SharedString, HoverTransition>,
    states: HashMap<SharedString, HoverTransition>,
}

impl Global for HoverMotionState {}

pub fn init(cx: &mut App) {
    cx.set_global(HoverMotionState::default());
}

pub fn progress(key: &SharedString, cx: &App) -> f32 {
    let Some(transition) = cx.global::<HoverMotionState>().transitions.get(key) else {
        return 0.0;
    };
    transition_progress(*transition, Instant::now())
}

pub fn set_hovered(key: SharedString, hovered: bool, window: &mut Window, cx: &mut App) {
    let now = Instant::now();
    let previous = cx
        .global::<HoverMotionState>()
        .transitions
        .get(&key)
        .copied();
    if previous.is_some_and(|transition| transition.hovered == hovered) {
        return;
    }
    let from = previous.map_or(0.0, |transition| transition_progress(transition, now));
    cx.global_mut::<HoverMotionState>().transitions.insert(
        key,
        HoverTransition {
            hovered,
            from,
            changed_at: now,
        },
    );
    window.refresh();

    refresh_for_transition(cx);
}

pub fn clear_hover(key: &SharedString, window: &mut Window, cx: &mut App) {
    if cx
        .global_mut::<HoverMotionState>()
        .transitions
        .remove(key)
        .is_some()
    {
        window.refresh();
    }
}

pub fn clear_all_hovers_app(cx: &mut App) {
    let transitions = &mut cx.global_mut::<HoverMotionState>().transitions;
    transitions.clear();
}

pub fn state_progress(key: &SharedString, active: bool, cx: &App) -> f32 {
    cx.global::<HoverMotionState>()
        .states
        .get(key)
        .copied()
        .map_or_else(
            || f32::from(active),
            |transition| {
                if transition.hovered == active {
                    transition_progress(transition, Instant::now())
                } else {
                    f32::from(active)
                }
            },
        )
}

pub fn set_active(key: SharedString, active: bool, window: &mut Window, cx: &mut App) {
    let now = Instant::now();
    let previous = cx.global::<HoverMotionState>().states.get(&key).copied();
    let from = previous.map_or_else(
        || f32::from(!active),
        |transition| transition_progress(transition, now),
    );
    cx.global_mut::<HoverMotionState>().states.insert(
        key,
        HoverTransition {
            hovered: active,
            from,
            changed_at: now,
        },
    );
    window.refresh();
    refresh_for_transition(cx);
}

fn refresh_for_transition(cx: &mut App) {
    cx.spawn(async move |cx| {
        let frame = Duration::from_millis(16);
        let frames = CONTROL_MOTION.as_millis().div_ceil(frame.as_millis());
        for _ in 0..frames {
            cx.background_executor().timer(frame).await;
            cx.refresh();
        }
    })
    .detach();
}

fn transition_progress(transition: HoverTransition, now: Instant) -> f32 {
    let elapsed = now.saturating_duration_since(transition.changed_at);
    let time = (elapsed.as_secs_f32() / CONTROL_MOTION.as_secs_f32()).clamp(0.0, 1.0);
    let eased = time * time * (3.0 - 2.0 * time);
    let target = f32::from(transition.hovered);
    transition.from + (target - transition.from) * eased
}

#[cfg(test)]
mod tests {
    use super::{HoverTransition, transition_progress};
    use crate::ui::foundation::motion::CONTROL_MOTION;
    use std::time::Instant;

    #[test]
    fn hover_transition_reaches_target() {
        let changed_at = Instant::now();
        let transition = HoverTransition {
            hovered: true,
            from: 0.25,
            changed_at,
        };
        assert_eq!(transition_progress(transition, changed_at), 0.25);
        assert_eq!(
            transition_progress(transition, changed_at + CONTROL_MOTION),
            1.0
        );

        let unhover = HoverTransition {
            hovered: false,
            from: 0.8,
            changed_at,
        };
        assert_eq!(
            transition_progress(unhover, changed_at + CONTROL_MOTION),
            0.0
        );
    }
}
