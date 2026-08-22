use gpui::{App, Entity, Radians, Rgba, Window};
use std::time::{Duration, Instant};

pub const CONTROL_MOTION: Duration = Duration::from_millis(150);
pub const MENU_MOTION: Duration = Duration::from_millis(140);
pub const DIALOG_MOTION: Duration = Duration::from_millis(180);
pub const MODE_MOTION: Duration = Duration::from_millis(300);
pub const TOOLTIP_MOTION: Duration = Duration::from_millis(130);
pub const UPDATE_PULSE_MOTION: Duration = Duration::from_millis(1_400);
pub const PULSE_MOTION: Duration = Duration::from_millis(1_800);

pub struct ScalarTransition {
    from: f32,
    to: f32,
    changed_at: Option<Instant>,
    duration: Duration,
}

impl ScalarTransition {
    pub fn new(value: f32, duration: Duration) -> Self {
        Self {
            from: value,
            to: value,
            changed_at: None,
            duration,
        }
    }

    pub fn sample(&self) -> (f32, bool) {
        let Some(changed_at) = self.changed_at else {
            return (self.to, false);
        };
        let linear = changed_at.elapsed().as_secs_f32() / self.duration.as_secs_f32();
        if linear >= 1.0 {
            return (self.to, false);
        }
        let eased = if linear < 0.5 {
            4.0 * linear.powi(3)
        } else {
            1.0 - (-2.0 * linear + 2.0).powi(3) / 2.0
        };
        (self.from + (self.to - self.from) * eased, true)
    }

    pub fn set_target(&mut self, target: f32) {
        if (self.to - target).abs() < f32::EPSILON {
            return;
        }
        let (current, _) = self.sample();
        self.from = current;
        self.to = target;
        self.changed_at = ((current - target).abs() > f32::EPSILON).then(Instant::now);
    }
}

pub fn stepped_pulse(progress: f32, steps: usize) -> f32 {
    let steps = steps.max(1) as f32;
    let step_progress = (progress.clamp(0.0, 1.0) * steps).floor() / steps;
    (1.0 - (std::f32::consts::TAU * step_progress).cos()) * 0.5
}

pub fn update_pulse_opacity(progress: f32) -> f32 {
    let wave = (1.0 - (std::f32::consts::TAU * progress.clamp(0.0, 1.0)).cos()) / 2.0;
    0.7 + 0.3 * wave
}

pub fn refresh_rotation(progress: f32) -> Radians {
    Radians(std::f32::consts::TAU * progress)
}

#[derive(Default)]
pub struct DropdownMotion {
    hovered: bool,
    hovered_item: Option<usize>,
    open: bool,
    closing: bool,
    menu_revision: u64,
    surface_changed_at: Option<Instant>,
    open_changed_at: Option<Instant>,
    item_transition: Option<ItemTransition>,
}

struct ItemTransition {
    from: Option<usize>,
    to: Option<usize>,
    changed_at: Instant,
}

impl ItemTransition {
    fn affects(&self, index: usize) -> bool {
        self.from == Some(index) || self.to == Some(index)
    }
}

impl DropdownMotion {
    pub fn hovered(&self) -> bool {
        self.hovered
    }

    pub fn open(&self) -> bool {
        self.open
    }

    pub fn closing(&self) -> bool {
        self.closing
    }

    pub fn visible(&self) -> bool {
        self.open || self.closing
    }

    pub fn hovered_item(&self) -> Option<usize> {
        self.hovered_item
    }

    pub fn menu_revision(&self) -> u64 {
        self.menu_revision
    }

    pub fn surface_animating(&self) -> bool {
        changed_recently(self.surface_changed_at, CONTROL_MOTION)
    }

    pub fn open_animating(&self) -> bool {
        changed_recently(self.open_changed_at, CONTROL_MOTION)
    }

    pub fn item_animating(&self, index: usize) -> bool {
        self.item_transition.as_ref().is_some_and(|transition| {
            changed_recently(Some(transition.changed_at), CONTROL_MOTION)
                && transition.affects(index)
        })
    }
}

pub fn set_dropdown_hovered(
    motion: &Entity<DropdownMotion>,
    hovered: bool,
    window: &mut Window,
    cx: &mut App,
) {
    if motion.read(cx).hovered == hovered {
        return;
    }
    motion.update(cx, |state, cx| {
        state.hovered = hovered;
        state.surface_changed_at = Some(Instant::now());
        cx.notify();
    });
    window.refresh();
}

pub fn set_dropdown_item_hovered(
    motion: &Entity<DropdownMotion>,
    index: usize,
    hovered: bool,
    window: &mut Window,
    cx: &mut App,
) {
    let previous = motion.read(cx).hovered_item;
    let next = if hovered {
        Some(index)
    } else if previous == Some(index) {
        None
    } else {
        return;
    };
    if previous == next {
        return;
    }
    motion.update(cx, |state, cx| {
        state.hovered_item = next;
        state.item_transition = Some(ItemTransition {
            from: previous,
            to: next,
            changed_at: Instant::now(),
        });
        cx.notify();
    });
    window.refresh();
}

pub fn set_dropdown_open(
    motion: &Entity<DropdownMotion>,
    open: bool,
    window: &mut Window,
    cx: &mut App,
) {
    let should_update = {
        let state = motion.read(cx);
        state.open != open || (open && state.closing)
    };
    if !should_update {
        return;
    }

    let revision = motion.update(cx, |state, cx| {
        state.open = open;
        state.closing = !open;
        state.hovered_item = None;
        state.surface_changed_at = Some(Instant::now());
        state.open_changed_at = Some(Instant::now());
        state.item_transition = None;
        state.menu_revision = state.menu_revision.wrapping_add(1);
        cx.notify();
        state.menu_revision
    });
    window.refresh();

    if open {
        return;
    }

    let motion = motion.clone();
    cx.spawn(async move |cx| {
        cx.background_executor().timer(MENU_MOTION).await;
        motion.update(cx, |state, cx| {
            if !state.open && state.menu_revision == revision {
                state.closing = false;
                cx.notify();
            }
        });
        cx.refresh();
    })
    .detach();
}

pub fn reset_dropdown_interaction(motion: &Entity<DropdownMotion>, cx: &mut App) {
    motion.update(cx, |state, cx| {
        state.hovered = false;
        state.hovered_item = None;
        state.open = false;
        state.closing = false;
        state.surface_changed_at = None;
        state.open_changed_at = None;
        state.item_transition = None;
        state.menu_revision = state.menu_revision.wrapping_add(1);
        cx.notify();
    });
}

pub fn changed_recently(changed_at: Option<Instant>, duration: Duration) -> bool {
    changed_at.is_some_and(|changed_at| changed_at.elapsed() < duration)
}

pub fn mix_color(from: Rgba, to: Rgba, progress: f32) -> Rgba {
    let progress = progress.clamp(0.0, 1.0);
    Rgba {
        r: from.r + (to.r - from.r) * progress,
        g: from.g + (to.g - from.g) * progress,
        b: from.b + (to.b - from.b) * progress,
        a: from.a + (to.a - from.a) * progress,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DropdownMotion, ItemTransition, ScalarTransition, mix_color, refresh_rotation,
        stepped_pulse, update_pulse_opacity,
    };
    use gpui::rgba;
    use std::time::{Duration, Instant};

    #[test]
    fn scalar_transition_starts_at_rest_and_ignores_same_target() {
        let mut transition = ScalarTransition::new(1.0, Duration::from_millis(250));
        assert_eq!(transition.sample(), (1.0, false));
        transition.set_target(1.0);
        assert_eq!(transition.sample(), (1.0, false));
    }

    #[test]
    fn color_mix_clamps_progress() {
        assert_eq!(
            mix_color(rgba(0x000000ff), rgba(0xffffffff), -1.0),
            rgba(0x000000ff)
        );
        assert_eq!(
            mix_color(rgba(0x000000ff), rgba(0xffffffff), 2.0),
            rgba(0xffffffff)
        );
    }

    #[test]
    fn update_pulse_returns_smooth_loop_endpoints() {
        assert!((update_pulse_opacity(0.0) - 0.7).abs() < f32::EPSILON);
        assert!((update_pulse_opacity(0.5) - 1.0).abs() < f32::EPSILON);
        assert!((update_pulse_opacity(1.0) - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn stepped_pulse_quantizes_wave_properly() {
        assert!((stepped_pulse(0.0, 30) - 0.0).abs() < f32::EPSILON);
        assert!((stepped_pulse(0.5, 30) - 1.0).abs() < f32::EPSILON);
        assert!((stepped_pulse(1.0, 30) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn refresh_icon_rotates_clockwise() {
        assert!(refresh_rotation(0.25).0 > 0.0);
    }

    #[test]
    fn untouched_controls_do_not_animate_on_mount() {
        let motion = DropdownMotion::default();
        assert!(!motion.surface_animating());
        assert!(!motion.open_animating());
        assert!(!motion.item_animating(0));
    }

    #[test]
    fn item_transition_only_affects_changed_items() {
        let transition = ItemTransition {
            from: Some(1),
            to: Some(3),
            changed_at: Instant::now(),
        };

        assert!(transition.affects(1));
        assert!(transition.affects(3));
        assert!(!transition.affects(0));
        assert!(!transition.affects(2));
    }
}
