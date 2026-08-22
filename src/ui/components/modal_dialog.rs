use std::time::Duration;

use gpui::prelude::*;
use gpui::*;

use crate::ui::foundation::colors::{
    black as flex_black, border, card as card_color, foreground, muted_foreground,
};

/// Unified modal dialog container for confirmation, forms, and popups.
pub struct ModalDialog {
    width: Pixels,
    max_height: Option<Pixels>,
    title: Option<SharedString>,
    description: Option<SharedString>,
    children: Vec<AnyElement>,
    actions: Vec<AnyElement>,
    anim_id: SharedString,
    closing_progress: Option<f32>,
}

impl ModalDialog {
    pub fn new() -> Self {
        Self {
            width: px(460.),
            max_height: None,
            title: None,
            description: None,
            children: Vec::new(),
            actions: Vec::new(),
            anim_id: SharedString::from("modal-dialog-appear"),
            closing_progress: None,
        }
    }

    pub fn width(mut self, width: Pixels) -> Self {
        self.width = width;
        self
    }

    pub fn max_height(mut self, max_h: Pixels) -> Self {
        self.max_height = Some(max_h);
        self
    }

    pub fn anim_id(mut self, id: impl Into<SharedString>) -> Self {
        self.anim_id = id.into();
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

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }

    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.actions.push(action.into_any_element());
        self
    }

    pub fn closing_progress(mut self, progress: Option<f32>) -> Self {
        self.closing_progress = progress;
        self
    }
}

impl Default for ModalDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoElement for ModalDialog {
    type Element = AnyElement;

    fn into_element(self) -> Self::Element {
        let card_id = SharedString::from(format!("{}-card", self.anim_id));
        let backdrop_id = SharedString::from(format!("{}-backdrop", self.anim_id));

        let mut card = div()
            .id(card_id)
            .relative()
            .w(self.width)
            .p_5()
            .rounded(px(8.))
            .border_1()
            .border_color(border().opacity(0.7))
            .bg(card_color())
            .shadow_lg()
            .flex()
            .flex_col()
            .gap_3()
            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                cx.stop_propagation();
            })
            .on_mouse_down(MouseButton::Right, |_, _, cx| {
                cx.stop_propagation();
            })
            .on_mouse_up(MouseButton::Left, |_, _, cx| {
                cx.stop_propagation();
            })
            .on_click(|_, _, cx| {
                cx.stop_propagation();
            })
            .on_scroll_wheel(|_, _, cx| {
                cx.stop_propagation();
            });

        if let Some(max_h) = self.max_height {
            card = card.max_h(max_h);
        }

        if let Some(title) = self.title {
            let mut header_col = div().flex().flex_col().gap_1().child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(foreground())
                    .child(title),
            );

            if let Some(desc) = self.description {
                header_col = header_col.child(
                    div()
                        .text_xs()
                        .line_height(px(18.))
                        .text_color(muted_foreground())
                        .child(desc),
                );
            }

            card = card.child(header_col);
        }

        if !self.children.is_empty() {
            card = card.children(self.children);
        }

        if !self.actions.is_empty() {
            let footer = div()
                .flex()
                .justify_end()
                .gap_2()
                .pt_2()
                .children(self.actions);
            card = card.child(footer);
        }

        let (animated_card, bg_opacity) = if let Some(progress) = self.closing_progress {
            let progress = progress.clamp(0.0, 1.0);
            (
                card.opacity(progress)
                    .top(px(-6.0 * (1.0 - progress)))
                    .into_any_element(),
                0.60 * progress,
            )
        } else {
            (
                card.with_animation(
                    self.anim_id,
                    Animation::new(Duration::from_millis(160))
                        .with_easing(|p| 1.0 - (1.0 - p).powi(4)),
                    |card, delta| card.opacity(delta).top(px(-6.0 * (1.0 - delta))),
                )
                .into_any_element(),
                0.60,
            )
        };

        div()
            .id(backdrop_id)
            .absolute()
            .inset_0()
            .bg(flex_black().opacity(bg_opacity))
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                cx.stop_propagation();
            })
            .on_mouse_down(MouseButton::Right, |_, _, cx| {
                cx.stop_propagation();
            })
            .on_mouse_up(MouseButton::Left, |_, _, cx| {
                cx.stop_propagation();
            })
            .on_click(|_, _, cx| {
                cx.stop_propagation();
            })
            .on_scroll_wheel(|_, _, cx| {
                cx.stop_propagation();
            })
            .child(animated_card)
            .into_any_element()
    }
}
