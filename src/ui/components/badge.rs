use gpui::prelude::*;
use gpui::*;
use std::time::Duration;

use crate::ui::foundation::colors;
use crate::ui::foundation::motion::{UPDATE_PULSE_MOTION, refresh_rotation, update_pulse_opacity};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BadgeVariant {
    #[default]
    Neutral,
    Accent,
    Success,
    Warning,
    Orange,
    Destructive,
    Outline,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BadgeSize {
    #[default]
    Sm,
    Md,
    Lg,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BadgeStyle {
    Neutral,
    Purple,
    Cyan,
    Green,
    Red,
    Orange,
}

/// Unified Badge component for tags, status indicators, and metadata labels.
pub struct Badge {
    label: SharedString,
    variant: BadgeVariant,
    size: BadgeSize,
    monospace: bool,
    icon: Option<(&'static str, Option<Rgba>)>,
    pulse_id: Option<SharedString>,
    fade_id: Option<SharedString>,
    spinner_id: Option<SharedString>,
}

impl Badge {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            variant: BadgeVariant::Neutral,
            size: BadgeSize::Sm,
            monospace: false,
            icon: None,
            pulse_id: None,
            fade_id: None,
            spinner_id: None,
        }
    }

    pub fn variant(mut self, variant: BadgeVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn neutral(mut self) -> Self {
        self.variant = BadgeVariant::Neutral;
        self
    }

    pub fn accent(mut self) -> Self {
        self.variant = BadgeVariant::Accent;
        self
    }

    pub fn success(mut self) -> Self {
        self.variant = BadgeVariant::Success;
        self
    }

    pub fn warning(mut self) -> Self {
        self.variant = BadgeVariant::Warning;
        self
    }

    pub fn orange(mut self) -> Self {
        self.variant = BadgeVariant::Orange;
        self
    }

    pub fn destructive(mut self) -> Self {
        self.variant = BadgeVariant::Destructive;
        self
    }

    pub fn outline(mut self) -> Self {
        self.variant = BadgeVariant::Outline;
        self
    }

    pub fn size(mut self, size: BadgeSize) -> Self {
        self.size = size;
        self
    }

    pub fn small(mut self) -> Self {
        self.size = BadgeSize::Sm;
        self
    }

    pub fn medium(mut self) -> Self {
        self.size = BadgeSize::Md;
        self
    }

    pub fn large(mut self) -> Self {
        self.size = BadgeSize::Lg;
        self
    }

    pub fn monospace(mut self) -> Self {
        self.monospace = true;
        self
    }

    pub fn icon(mut self, path: &'static str) -> Self {
        self.icon = Some((path, None));
        self
    }

    pub fn icon_colored(mut self, path: &'static str, color: Rgba) -> Self {
        self.icon = Some((path, Some(color)));
        self
    }

    pub fn pulse(mut self, anim_id: impl Into<SharedString>) -> Self {
        self.pulse_id = Some(anim_id.into());
        self
    }

    pub fn fade_in(mut self, anim_id: impl Into<SharedString>) -> Self {
        self.fade_id = Some(anim_id.into());
        self
    }

    pub fn spinner(mut self, anim_id: impl Into<SharedString>) -> Self {
        self.spinner_id = Some(anim_id.into());
        self
    }
}

impl IntoElement for Badge {
    type Element = AnyElement;

    fn into_element(self) -> Self::Element {
        let (bg, border, fg) = match self.variant {
            BadgeVariant::Neutral => (
                colors::muted().opacity(0.3),
                colors::border().opacity(0.6),
                colors::muted_foreground(),
            ),
            BadgeVariant::Accent => (
                colors::accent().opacity(0.12),
                colors::accent().opacity(0.35),
                colors::accent(),
            ),
            BadgeVariant::Success => (
                colors::success().opacity(0.10),
                colors::success().opacity(0.35),
                colors::success(),
            ),
            BadgeVariant::Warning => (
                colors::warning().opacity(0.15),
                colors::warning().opacity(0.40),
                colors::warning(),
            ),
            BadgeVariant::Orange => (
                colors::orange().opacity(0.12),
                colors::orange().opacity(0.40),
                colors::orange(),
            ),
            BadgeVariant::Destructive => (
                colors::destructive().opacity(0.15),
                colors::destructive().opacity(0.40),
                colors::destructive(),
            ),
            BadgeVariant::Outline => (
                rgba(0x00000000),
                colors::border().opacity(0.8),
                colors::foreground(),
            ),
        };

        let (px_pad, py_pad, text_sz, line_h, gap_sz, icon_sz, radius) = match self.size {
            BadgeSize::Sm => (px(4.), px(1.5), px(10.), px(10.), px(3.), px(10.), px(3.)),
            BadgeSize::Md => (px(6.), px(2.), px(11.), px(12.), px(4.), px(12.), px(4.)),
            BadgeSize::Lg => (px(8.), px(3.), px(12.), px(14.), px(5.), px(14.), px(5.)),
        };

        let mut el = div()
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .gap(gap_sz)
            .px(px_pad)
            .py(py_pad)
            .rounded(radius)
            .border_1()
            .border_color(border)
            .bg(bg)
            .text_size(text_sz)
            .line_height(line_h)
            .font_weight(FontWeight::MEDIUM)
            .text_color(fg);

        if self.monospace {
            el = el.font_family("IBM Plex Mono");
        }

        if let Some(spin_id) = self.spinner_id {
            el = el.child(
                svg()
                    .path("icons/refresh-cw.svg")
                    .size(icon_sz)
                    .text_color(fg)
                    .with_animation(
                        spin_id,
                        Animation::new(Duration::from_millis(850)).repeat(),
                        |icon, delta| {
                            icon.with_transformation(Transformation::rotate(refresh_rotation(
                                delta,
                            )))
                        },
                    ),
            );
        } else if let Some((icon_path, icon_color)) = self.icon {
            let color = icon_color.unwrap_or(fg);
            el = el.child(svg().path(icon_path).size(icon_sz).text_color(color));
        }

        el = el.child(self.label);

        if let Some(pulse_id) = self.pulse_id {
            el.with_animation(
                pulse_id,
                Animation::new(UPDATE_PULSE_MOTION).repeat(),
                |badge, delta| badge.opacity(update_pulse_opacity(delta)),
            )
            .into_any_element()
        } else if let Some(fade_id) = self.fade_id {
            el.with_animation(
                fade_id,
                Animation::new(Duration::from_millis(250)),
                |badge, delta| badge.opacity(delta),
            )
            .into_any_element()
        } else {
            el.into_any_element()
        }
    }
}

/// Convenience builder function for creating a Badge.
pub fn badge(label: impl Into<SharedString>) -> Badge {
    Badge::new(label)
}

/// Backward compatibility function
pub fn loading_badge(text: impl Into<SharedString>) -> AnyElement {
    Badge::new(text)
        .accent()
        .spinner("loading-badge-spinner")
        .into_any_element()
}

pub fn progress_badge(text: impl Into<SharedString>, progress: f32) -> AnyElement {
    let foreground = colors::accent();
    div()
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .gap(px(5.))
        .px(px(6.))
        .py(px(2.))
        .rounded(px(4.))
        .border_1()
        .border_color(foreground.opacity(0.4))
        .bg(foreground.opacity(0.12))
        .text_size(px(11.))
        .line_height(px(12.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(foreground)
        .child(progress_ring(progress))
        .child(text.into())
        .into_any_element()
}

pub fn progress_ring(progress: f32) -> AnyElement {
    let progress = progress.clamp(0.0, 1.0);
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let center = point(
                bounds.origin.x + bounds.size.width / 2.0,
                bounds.origin.y + bounds.size.height / 2.0,
            );
            let radius = px(6.0);
            let top = point(center.x, center.y - radius);
            let bottom = point(center.x, center.y + radius);
            let radii = point(radius, radius);

            let mut track = PathBuilder::stroke(px(2.0));
            track.move_to(top);
            track.arc_to(radii, px(0.0), false, true, bottom);
            track.arc_to(radii, px(0.0), false, true, top);
            if let Ok(track) = track.build() {
                window.paint_path(track, colors::accent().opacity(0.25));
            }

            if progress <= f32::EPSILON {
                return;
            }

            let mut arc = PathBuilder::stroke(px(2.0));
            arc.move_to(top);
            if progress >= 1.0 - f32::EPSILON {
                arc.arc_to(radii, px(0.0), false, true, bottom);
                arc.arc_to(radii, px(0.0), false, true, top);
            } else {
                let angle = -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * progress;
                let end = point(
                    center.x + radius * angle.cos(),
                    center.y + radius * angle.sin(),
                );
                arc.arc_to(radii, px(0.0), progress > 0.5, true, end);
            }
            if let Ok(arc) = arc.build() {
                window.paint_path(arc, colors::accent());
            }
        },
    )
    .size_4()
    .into_any_element()
}
