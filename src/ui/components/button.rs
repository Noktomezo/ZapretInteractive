use gpui::prelude::*;
use gpui::*;
use std::time::Duration;

use crate::ui::foundation::colors;
use crate::ui::foundation::control_style::CONTROL_HEIGHT;
use crate::ui::foundation::hover_motion;
use crate::ui::foundation::motion::{mix_color, refresh_rotation};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Outline,
    Ghost,
    Destructive,
    Success,
    Orange,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ButtonSize {
    Sm,
    #[default]
    Md,
    Lg,
}

type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum IconButtonVariant {
    #[default]
    Ghost,
    Outline,
    Secondary,
    Primary,
    Destructive,
    Warning,
    Success,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum IconButtonSize {
    Sm,
    #[default]
    Md,
    Lg,
}

pub struct Button {
    id: ElementId,
    hover_key: SharedString,
    label: SharedString,
    variant: ButtonVariant,
    size: ButtonSize,
    icon_prefix: Option<&'static str>,
    icon_suffix: Option<&'static str>,
    loading: bool,
    disabled: bool,
    tooltip: Option<SharedString>,
    on_click: Option<ClickHandler>,
    hover_progress: f32,
}

impl Button {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>, cx: &App) -> Self {
        let label = label.into();
        let id_val = id.into();
        let hover_key: SharedString = SharedString::from(format!("btn-{}", id_val));
        let hover_progress = hover_motion::progress(&hover_key, cx);
        Self {
            id: id_val,
            hover_key,
            label,
            variant: ButtonVariant::default(),
            size: ButtonSize::default(),
            icon_prefix: None,
            icon_suffix: None,
            loading: false,
            disabled: false,
            tooltip: None,
            on_click: None,
            hover_progress,
        }
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn primary(mut self) -> Self {
        self.variant = ButtonVariant::Primary;
        self
    }

    pub fn secondary(mut self) -> Self {
        self.variant = ButtonVariant::Secondary;
        self
    }

    pub fn outline(mut self) -> Self {
        self.variant = ButtonVariant::Outline;
        self
    }

    pub fn ghost(mut self) -> Self {
        self.variant = ButtonVariant::Ghost;
        self
    }

    pub fn destructive(mut self) -> Self {
        self.variant = ButtonVariant::Destructive;
        self
    }

    pub fn success(mut self) -> Self {
        self.variant = ButtonVariant::Success;
        self
    }

    pub fn orange(mut self) -> Self {
        self.variant = ButtonVariant::Orange;
        self
    }

    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn small(mut self) -> Self {
        self.size = ButtonSize::Sm;
        self
    }

    pub fn large(mut self) -> Self {
        self.size = ButtonSize::Lg;
        self
    }

    pub fn icon_prefix(mut self, icon: &'static str) -> Self {
        self.icon_prefix = Some(icon);
        self
    }

    pub fn icon_suffix(mut self, icon: &'static str) -> Self {
        self.icon_suffix = Some(icon);
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    pub fn on_click(
        mut self,
        click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(click));
        self
    }
}

impl IntoElement for Button {
    type Element = AnyElement;

    fn into_element(self) -> Self::Element {
        let (height, px_pad, text_sz, icon_sz, gap_sz) = match self.size {
            ButtonSize::Sm => (CONTROL_HEIGHT, px(10.), px(12.), px(13.), px(5.)),
            ButtonSize::Md => (CONTROL_HEIGHT, px(14.), px(13.), px(14.), px(6.)),
            ButtonSize::Lg => (CONTROL_HEIGHT, px(18.), px(14.), px(16.), px(8.)),
        };

        let is_dark = colors::is_dark();
        let (base_bg, hover_bg, base_border, hover_border, base_fg, hover_fg) = match self.variant {
            ButtonVariant::Primary => (
                colors::accent(),
                mix_color(colors::accent(), colors::background(), 0.08),
                colors::accent(),
                mix_color(colors::accent(), colors::background(), 0.08),
                colors::accent_foreground(),
                colors::accent_foreground(),
            ),
            ButtonVariant::Secondary => (
                colors::input().opacity(if is_dark { 0.35 } else { 0.85 }),
                colors::secondary().opacity(if is_dark { 0.75 } else { 0.95 }),
                colors::border().opacity(0.7),
                colors::border(),
                colors::foreground(),
                colors::foreground(),
            ),
            ButtonVariant::Outline => (
                colors::card().opacity(0.4),
                colors::secondary().opacity(0.6),
                colors::border().opacity(0.8),
                colors::accent().opacity(0.7),
                colors::foreground(),
                colors::foreground(),
            ),
            ButtonVariant::Ghost => (
                rgba(0x00000000),
                colors::muted().opacity(0.35),
                rgba(0x00000000),
                rgba(0x00000000),
                colors::muted_foreground(),
                colors::foreground(),
            ),
            ButtonVariant::Destructive => (
                colors::destructive().opacity(if is_dark { 0.15 } else { 0.10 }),
                colors::destructive().opacity(if is_dark { 0.25 } else { 0.20 }),
                colors::destructive().opacity(0.35),
                colors::destructive().opacity(0.7),
                colors::destructive(),
                colors::destructive(),
            ),
            ButtonVariant::Success => (
                colors::success().opacity(0.12),
                colors::success().opacity(0.22),
                colors::success().opacity(0.35),
                colors::success().opacity(0.7),
                colors::success(),
                colors::success(),
            ),
            ButtonVariant::Orange => (
                colors::orange(),
                mix_color(colors::orange(), colors::background(), 0.08),
                colors::orange(),
                mix_color(colors::orange(), colors::background(), 0.08),
                colors::accent_foreground(),
                colors::accent_foreground(),
            ),
        };

        let effective_hover = if self.disabled {
            0.0
        } else {
            self.hover_progress
        };
        let bg_color = mix_color(base_bg, hover_bg, effective_hover);
        let border_color = mix_color(base_border, hover_border, effective_hover);
        let text_color = mix_color(base_fg, hover_fg, effective_hover);

        let hk = self.hover_key.clone();
        let source = self.id.clone();
        let is_disabled = self.disabled || self.loading;

        let mut el = div()
            .id(self.id)
            .h(height)
            .max_w_full()
            .px(px_pad)
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .gap(gap_sz)
            .rounded_md()
            .border_1()
            .border_color(border_color)
            .bg(bg_color)
            .text_size(text_sz)
            .line_height(text_sz)
            .font_weight(FontWeight::MEDIUM)
            .text_color(text_color);

        if is_disabled {
            el = el.opacity(0.6).cursor_not_allowed();
            if let Some(tooltip) = self.tooltip {
                el = crate::ui::components::cursor_tooltip::attach(el, source, tooltip);
            }
        } else if let Some(tooltip) = self.tooltip {
            el = el.cursor_pointer();
            el = crate::ui::components::cursor_tooltip::attach_with_hover_motion(
                el, source, hk, tooltip,
            );
        } else {
            el = el.cursor_pointer().on_hover(move |hovered, window, cx| {
                hover_motion::set_hovered(hk.clone(), *hovered, window, cx);
            });
        }

        if let Some(click) = self.on_click
            && !is_disabled
        {
            let hk_click = self.hover_key.clone();
            el = el.on_click(move |event, window, cx| {
                hover_motion::clear_hover(&hk_click, window, cx);
                click(event, window, cx);
            });
        }

        if self.loading {
            let anim_id: SharedString = SharedString::from(format!("btn-spin-{}", self.hover_key));
            el = el.child(
                svg()
                    .path("icons/refresh-cw.svg")
                    .size(icon_sz)
                    .text_color(text_color)
                    .with_animation(
                        anim_id,
                        Animation::new(Duration::from_millis(900)).repeat(),
                        |icon, delta| {
                            icon.with_transformation(Transformation::rotate(refresh_rotation(
                                delta,
                            )))
                        },
                    ),
            );
        } else if let Some(icon) = self.icon_prefix {
            el = el.child(svg().path(icon).size(icon_sz).text_color(text_color));
        }

        el = el.child(div().min_w_0().truncate().child(self.label));

        if !self.loading
            && let Some(icon) = self.icon_suffix
        {
            el = el.child(svg().path(icon).size(icon_sz).text_color(text_color));
        }

        el.into_any_element()
    }
}

pub struct IconButton {
    id: ElementId,
    hover_key: SharedString,
    icon: &'static str,
    variant: IconButtonVariant,
    size: IconButtonSize,
    active: bool,
    loading: bool,
    disabled: bool,
    tooltip: Option<SharedString>,
    on_click: Option<ClickHandler>,
    hover_progress: f32,
}

impl IconButton {
    pub fn new(id: impl Into<ElementId>, icon: &'static str, cx: &App) -> Self {
        let id_val = id.into();
        let hover_key: SharedString = SharedString::from(format!("icn-btn-{}", id_val));
        let hover_progress = hover_motion::progress(&hover_key, cx);
        Self {
            id: id_val,
            hover_key,
            icon,
            variant: IconButtonVariant::default(),
            size: IconButtonSize::default(),
            active: false,
            loading: false,
            disabled: false,
            tooltip: None,
            on_click: None,
            hover_progress,
        }
    }

    pub fn variant(mut self, variant: IconButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn ghost(mut self) -> Self {
        self.variant = IconButtonVariant::Ghost;
        self
    }

    pub fn outline(mut self) -> Self {
        self.variant = IconButtonVariant::Outline;
        self
    }

    pub fn secondary(mut self) -> Self {
        self.variant = IconButtonVariant::Secondary;
        self
    }

    pub fn primary(mut self) -> Self {
        self.variant = IconButtonVariant::Primary;
        self
    }

    pub fn destructive(mut self) -> Self {
        self.variant = IconButtonVariant::Destructive;
        self
    }

    pub fn warning(mut self) -> Self {
        self.variant = IconButtonVariant::Warning;
        self
    }

    pub fn success(mut self) -> Self {
        self.variant = IconButtonVariant::Success;
        self
    }

    pub fn size(mut self, size: IconButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn small(mut self) -> Self {
        self.size = IconButtonSize::Sm;
        self
    }

    pub fn large(mut self) -> Self {
        self.size = IconButtonSize::Lg;
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    pub fn on_click(
        mut self,
        click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(click));
        self
    }
}

impl IntoElement for IconButton {
    type Element = AnyElement;

    fn into_element(self) -> Self::Element {
        let (box_size, icon_sz) = match self.size {
            IconButtonSize::Sm => (CONTROL_HEIGHT, px(13.)),
            IconButtonSize::Md => (CONTROL_HEIGHT, px(14.)),
            IconButtonSize::Lg => (CONTROL_HEIGHT, px(16.)),
        };

        let is_dark = colors::is_dark();
        let (base_bg, hover_bg, base_border, hover_border, base_fg, hover_fg) = match self.variant {
            IconButtonVariant::Ghost => (
                rgba(0x00000000),
                colors::muted().opacity(0.35),
                rgba(0x00000000),
                rgba(0x00000000),
                colors::muted_foreground(),
                colors::foreground(),
            ),
            IconButtonVariant::Outline => (
                colors::input().opacity(if is_dark { 0.25 } else { 0.8 }),
                colors::secondary().opacity(if is_dark { 0.7 } else { 0.95 }),
                colors::border().opacity(0.7),
                colors::border(),
                colors::foreground().opacity(0.85),
                colors::foreground(),
            ),
            IconButtonVariant::Secondary => (
                colors::secondary().opacity(0.6),
                colors::secondary().opacity(0.9),
                colors::border().opacity(0.6),
                colors::border(),
                colors::foreground().opacity(0.85),
                colors::foreground(),
            ),
            IconButtonVariant::Primary => (
                colors::accent(),
                mix_color(colors::accent(), colors::background(), 0.08),
                colors::accent(),
                mix_color(colors::accent(), colors::background(), 0.08),
                colors::accent_foreground(),
                colors::accent_foreground(),
            ),
            IconButtonVariant::Destructive => (
                colors::destructive().opacity(0.10),
                colors::destructive().opacity(0.20),
                colors::destructive().opacity(0.35),
                colors::destructive().opacity(0.7),
                colors::destructive(),
                colors::destructive(),
            ),
            IconButtonVariant::Warning => (
                colors::warning().opacity(0.10),
                colors::warning().opacity(0.20),
                colors::warning().opacity(0.35),
                colors::warning().opacity(0.7),
                colors::warning(),
                colors::warning(),
            ),
            IconButtonVariant::Success => (
                colors::success().opacity(0.10),
                colors::success().opacity(0.20),
                colors::success().opacity(0.35),
                colors::success().opacity(0.7),
                colors::success(),
                colors::success(),
            ),
        };

        let effective_hover = if self.disabled {
            0.0
        } else {
            self.hover_progress
        };
        let bg_color = if self.active {
            colors::accent().opacity(0.15)
        } else {
            mix_color(base_bg, hover_bg, effective_hover)
        };
        let border_color = if self.active {
            colors::accent().opacity(0.5)
        } else {
            mix_color(base_border, hover_border, effective_hover)
        };
        let icon_color = if self.active {
            colors::accent()
        } else {
            mix_color(base_fg, hover_fg, effective_hover)
        };

        let hk = self.hover_key.clone();
        let source = self.id.clone();
        let is_disabled = self.disabled || self.loading;

        let mut el = div()
            .id(self.id)
            .size(box_size)
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .border_1()
            .border_color(border_color)
            .bg(bg_color);

        if is_disabled {
            el = el.opacity(0.6).cursor_not_allowed();
            if let Some(tooltip) = self.tooltip {
                el = crate::ui::components::cursor_tooltip::attach(el, source, tooltip);
            }
        } else if let Some(tooltip) = self.tooltip {
            el = el.cursor_pointer();
            el = crate::ui::components::cursor_tooltip::attach_with_hover_motion(
                el, source, hk, tooltip,
            );
        } else {
            el = el.cursor_pointer().on_hover(move |hovered, window, cx| {
                hover_motion::set_hovered(hk.clone(), *hovered, window, cx);
            });
        }

        if let Some(click) = self.on_click
            && !is_disabled
        {
            let hk_click = self.hover_key.clone();
            el = el.on_click(move |event, window, cx| {
                hover_motion::clear_hover(&hk_click, window, cx);
                click(event, window, cx);
            });
        }

        if self.loading {
            let anim_id: SharedString =
                SharedString::from(format!("icn-btn-spin-{}", self.hover_key));
            el = el.child(
                svg()
                    .path("icons/refresh-cw.svg")
                    .size(icon_sz)
                    .text_color(icon_color)
                    .with_animation(
                        anim_id,
                        Animation::new(Duration::from_millis(900)).repeat(),
                        |icon, delta| {
                            icon.with_transformation(Transformation::rotate(refresh_rotation(
                                delta,
                            )))
                        },
                    ),
            );
        } else {
            el = el.child(svg().path(self.icon).size(icon_sz).text_color(icon_color));
        }

        el.into_any_element()
    }
}

pub fn button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    variant: ButtonVariant,
    click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    Button::new(id, label, cx)
        .variant(variant)
        .on_click(click)
        .into_element()
}

pub fn icon_button(
    id: impl Into<ElementId>,
    icon: &'static str,
    variant: IconButtonVariant,
    click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    IconButton::new(id, icon, cx)
        .variant(variant)
        .on_click(click)
        .into_element()
}
