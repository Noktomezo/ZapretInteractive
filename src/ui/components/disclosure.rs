use gpui::*;

use crate::ui::foundation::motion::mix_color;
use crate::ui::foundation::{colors, hover_motion};

type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
pub struct DisclosureChevron {
    id: SharedString,
    expanded: bool,
    progress: f32,
    hover_key: SharedString,
    hover_progress: f32,
    on_click: Option<ClickHandler>,
}

impl DisclosureChevron {
    pub fn new(id: impl Into<SharedString>, expanded: bool, cx: &App) -> Self {
        let id = id.into();
        let hover_key = format!("{id}-hover").into();
        Self {
            progress: disclosure_progress(&id, expanded, cx),
            hover_progress: hover_motion::progress(&hover_key, cx),
            id,
            expanded,
            hover_key,
            on_click: None,
        }
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for DisclosureChevron {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let color = mix_color(
            colors::muted_foreground(),
            colors::foreground(),
            self.hover_progress,
        );
        let id = self.id.clone();
        let hover_key = self.hover_key.clone();
        let expanded = self.expanded;
        let mut element = div()
            .id(self.id)
            .size(crate::ui::foundation::control_style::CONTROL_HEIGHT)
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .text_color(color)
            .on_hover(move |hovered, window, cx| {
                hover_motion::set_hovered(hover_key.clone(), *hovered, window, cx);
            })
            .child(
                svg()
                    .path("icons/chevron-down.svg")
                    .size_4()
                    .text_color(color)
                    .with_transformation(Transformation::rotate(Radians(
                        std::f32::consts::PI * self.progress,
                    ))),
            );
        if let Some(handler) = self.on_click {
            element = element.on_click(move |event, window, cx| {
                cx.stop_propagation();
                hover_motion::set_active(id.clone(), !expanded, window, cx);
                handler(event, window, cx);
            });
        }
        element
    }
}

pub fn disclosure_progress(id: &SharedString, expanded: bool, cx: &App) -> f32 {
    hover_motion::state_progress(id, expanded, cx)
}
