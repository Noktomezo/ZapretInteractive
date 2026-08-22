use gpui::prelude::*;
use gpui::*;

use crate::ui::foundation::colors::{self, border, foreground, muted_foreground};
use crate::ui::foundation::motion::{UPDATE_PULSE_MOTION, update_pulse_opacity};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BannerVariant {
    #[default]
    Info,
    Warning,
    Destructive,
    Success,
    Outline,
}

/// Unified Banner component for callouts, warnings, downloads, and update notifications.
pub struct Banner {
    variant: BannerVariant,
    icon: Option<&'static str>,
    icon_pulsing: Option<SharedString>,
    title: Option<SharedString>,
    description: Option<SharedString>,
    actions: Vec<AnyElement>,
    width: Option<Pixels>,
    children: Vec<AnyElement>,
}

impl Banner {
    pub fn new() -> Self {
        Self {
            variant: BannerVariant::Info,
            icon: None,
            icon_pulsing: None,
            title: None,
            description: None,
            actions: Vec::new(),
            width: None,
            children: Vec::new(),
        }
    }

    pub fn info() -> Self {
        Self::new().variant(BannerVariant::Info)
    }

    pub fn warning() -> Self {
        Self::new().variant(BannerVariant::Warning)
    }

    pub fn destructive() -> Self {
        Self::new().variant(BannerVariant::Destructive)
    }

    pub fn success() -> Self {
        Self::new().variant(BannerVariant::Success)
    }

    pub fn outline() -> Self {
        Self::new().variant(BannerVariant::Outline)
    }

    pub fn variant(mut self, variant: BannerVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn icon(mut self, path: &'static str) -> Self {
        self.icon = Some(path);
        self
    }

    pub fn icon_pulsing(mut self, path: &'static str, anim_id: impl Into<SharedString>) -> Self {
        self.icon = Some(path);
        self.icon_pulsing = Some(anim_id.into());
        self
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn description(mut self, desc: impl Into<SharedString>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.actions.push(action.into_any_element());
        self
    }

    pub fn width(mut self, width: Pixels) -> Self {
        self.width = Some(width);
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }
}

impl Default for Banner {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoElement for Banner {
    type Element = AnyElement;

    fn into_element(self) -> Self::Element {
        let (bg, border_color, accent_color) = match self.variant {
            BannerVariant::Info => (
                colors::accent().opacity(0.08),
                colors::accent().opacity(0.35),
                colors::accent(),
            ),
            BannerVariant::Warning => (
                colors::warning().opacity(0.10),
                colors::warning().opacity(0.40),
                colors::warning(),
            ),
            BannerVariant::Destructive => (
                colors::destructive().opacity(0.12),
                colors::destructive().opacity(0.40),
                colors::destructive(),
            ),
            BannerVariant::Success => (
                colors::success().opacity(0.10),
                colors::success().opacity(0.35),
                colors::success(),
            ),
            BannerVariant::Outline => (
                colors::card().opacity(0.5),
                border().opacity(0.8),
                foreground(),
            ),
        };

        let mut el = div()
            .p_3()
            .rounded_lg()
            .border_1()
            .border_color(border_color)
            .bg(bg)
            .flex()
            .flex_col()
            .gap_2();

        if let Some(w) = self.width {
            el = el.w(w);
        }

        let has_content = self.title.is_some()
            || self.description.is_some()
            || self.icon.is_some()
            || !self.actions.is_empty();

        if has_content {
            let mut top_row = div().flex().items_center().justify_between().gap_3();

            let mut left_side = div().min_w_0().flex_1().flex().items_center().gap_3();

            if let Some(icon_path) = self.icon {
                let base_icon = svg()
                    .path(icon_path)
                    .size_5()
                    .flex_none()
                    .text_color(accent_color);

                let icon_el: AnyElement = if let Some(anim_id) = self.icon_pulsing {
                    base_icon
                        .with_animation(
                            anim_id,
                            Animation::new(UPDATE_PULSE_MOTION).repeat(),
                            |icon, delta| icon.opacity(update_pulse_opacity(delta)),
                        )
                        .into_any_element()
                } else {
                    base_icon.into_any_element()
                };

                left_side = left_side.child(icon_el);
            }

            let mut text_col = div().min_w_0().flex_1().flex_col();

            if let Some(title) = self.title {
                text_col = text_col.child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(foreground())
                        .child(title),
                );
            }

            if let Some(desc) = self.description {
                text_col = text_col.child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_xs()
                        .text_color(muted_foreground())
                        .child(desc),
                );
            }

            left_side = left_side.child(text_col);
            top_row = top_row.child(left_side);

            if !self.actions.is_empty() {
                let actions_row = div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap_2()
                    .children(self.actions);
                top_row = top_row.child(actions_row);
            }

            el = el.child(top_row);
        }

        if !self.children.is_empty() {
            el = el.children(self.children);
        }

        el.into_any_element()
    }
}
