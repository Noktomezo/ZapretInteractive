use gpui::prelude::*;
use gpui::*;

use crate::ui::foundation::colors;
use crate::ui::foundation::hover_motion;
use crate::ui::foundation::motion::mix_color;

const SWITCH_TRAVEL: Pixels = px(16.0);
type ToggleHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// Unified interactive Switch (toggle) component.
pub struct Switch {
    id: SharedString,
    checked: bool,
    disabled: bool,
    progress: f32,
    hover_key: SharedString,
    hover_progress: f32,
    tooltip: Option<SharedString>,
    on_toggle: Option<ToggleHandler>,
}

impl Switch {
    pub fn new(id: impl Into<SharedString>, checked: bool, cx: &App) -> Self {
        let key: SharedString = id.into();
        let progress = hover_motion::state_progress(&key, checked, cx);
        let hover_key: SharedString = SharedString::from(format!("{key}-hover"));
        let hover_progress = hover_motion::progress(&hover_key, cx);
        Self {
            id: key,
            checked,
            disabled: false,
            progress,
            hover_key,
            hover_progress,
            tooltip: None,
            on_toggle: None,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    pub fn on_toggle(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle = Some(Box::new(handler));
        self
    }
}

impl IntoElement for Switch {
    type Element = AnyElement;

    fn into_element(self) -> Self::Element {
        let key = self.id.clone();
        let checked = self.checked;
        let is_disabled = self.disabled;
        let source_id: SharedString = SharedString::from(format!("{key}-switch"));
        let state_bg = mix_color(
            colors::red().opacity(0.85),
            colors::green().opacity(0.85),
            self.progress,
        );
        let state_border = mix_color(colors::red(), colors::green(), self.progress);
        let hover_progress = if is_disabled {
            0.0
        } else {
            self.hover_progress
        };

        let mut track = div()
            .id(source_id.clone())
            .w(px(38.0))
            .h(px(22.0))
            .p(px(2.0))
            .flex_none()
            .flex()
            .items_center()
            .rounded(px(7.0))
            .border_1()
            .bg(mix_color(
                state_bg,
                colors::foreground(),
                0.06 * hover_progress,
            ))
            .border_color(mix_color(
                state_border,
                colors::foreground(),
                0.18 * hover_progress,
            ))
            .child(
                div()
                    .size(px(16.0))
                    .ml(SWITCH_TRAVEL * self.progress)
                    .rounded(px(5.0))
                    .bg(colors::black())
                    .shadow_sm(),
            );

        if is_disabled {
            track = track.opacity(0.5).cursor_not_allowed();
            if let Some(tooltip) = self.tooltip {
                track = crate::ui::components::cursor_tooltip::attach(
                    track,
                    ElementId::from(source_id),
                    tooltip,
                );
            }
        } else {
            track = track.cursor_pointer();
            if let Some(tooltip) = self.tooltip {
                track = crate::ui::components::cursor_tooltip::attach_with_hover_motion(
                    track,
                    ElementId::from(source_id),
                    self.hover_key,
                    tooltip,
                );
            } else {
                let hover_key = self.hover_key;
                track = track.on_hover(move |hovered, window, cx| {
                    hover_motion::set_hovered(hover_key.clone(), *hovered, window, cx);
                });
            }
            if let Some(on_toggle) = self.on_toggle {
                let switch_id = key.clone();
                track = track.on_click(move |event, window, cx| {
                    cx.stop_propagation();
                    let new_state = !checked;
                    animate_toggle(switch_id.clone(), new_state, window, cx);
                    on_toggle(event, window, cx);
                });
            }
        }

        track.into_any_element()
    }
}

/// Convenience builder function for Switch.
pub fn switch(id: impl Into<SharedString>, checked: bool, cx: &App) -> Switch {
    Switch::new(id, checked, cx)
}

/// Animate switch state transition.
pub fn animate_toggle(
    id: impl Into<SharedString>,
    checked: bool,
    window: &mut Window,
    cx: &mut App,
) {
    hover_motion::set_active(id.into(), checked, window, cx);
}
