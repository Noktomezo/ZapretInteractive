use std::time::Duration;

use gpui::prelude::*;
use gpui::*;

use crate::ui::foundation::control_style::{CONTROL_FONT_FAMILY, DROPDOWN_CONTROL_HEIGHT};
use crate::ui::foundation::motion::{CONTROL_MOTION, mix_color};

const MARQUEE_DURATION: Duration = Duration::from_millis(1_800);
// The control has an 8 px gutter; keep its outer 1 px border unobscured.
const FADE_WIDTH: Pixels = px(7.0);
const BORDER_INSET: Pixels = px(1.0);
const CONTROL_FONT_SIZE: Pixels = px(12.0);

#[derive(IntoElement)]
pub struct MarqueeText {
    id: ElementId,
    text: SharedString,
    max_width: Pixels,
    active: bool,
    fade_color: Rgba,
    fade_motion: Option<FadeMotion>,
}

#[derive(Clone, Copy)]
pub struct MarqueeFade {
    resting: Rgba,
    active: Rgba,
    is_active: bool,
    animating: bool,
}

impl MarqueeFade {
    pub fn new(resting: Rgba, active: Rgba, is_active: bool, animating: bool) -> Self {
        Self {
            resting,
            active,
            is_active,
            animating,
        }
    }
}

struct FadeMotion {
    id: ElementId,
    fade: MarqueeFade,
}

impl MarqueeText {
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>, max_width: Pixels) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            max_width,
            active: false,
            fade_color: transparent_black().into(),
            fade_motion: None,
        }
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn fade_with_motion(mut self, id: impl Into<ElementId>, fade: MarqueeFade) -> Self {
        self.fade_color = if fade.is_active {
            fade.active
        } else {
            fade.resting
        };
        self.fade_motion = Some(FadeMotion {
            id: id.into(),
            fade,
        });
        self
    }
}

impl RenderOnce for MarqueeText {
    fn render(self, window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let text_width = control_text_width(&self.text, window);
        let viewport_width = text_width.min(self.max_width);
        let shift = marquee_shift(text_width, viewport_width);
        let anchor = div()
            .relative()
            .w(viewport_width)
            .h(DROPDOWN_CONTROL_HEIGHT)
            .flex_none();
        let fade_layer = animated_edge_fades(self.fade_color, self.fade_motion);

        if self.active && shift > Pixels::ZERO {
            let text = self.text;
            anchor
                .child(
                    expanded_viewport(viewport_width)
                        .child(
                            div()
                                .absolute()
                                .inset_0()
                                .flex()
                                .items_center()
                                .with_animation(
                                    self.id,
                                    Animation::new(MARQUEE_DURATION)
                                        .repeat()
                                        .with_easing(bounce(ease_in_out)),
                                    move |element, progress| {
                                        element.child(marquee_line(
                                            text.clone(),
                                            FADE_WIDTH - shift * progress,
                                        ))
                                    },
                                ),
                        )
                        .child(fade_layer),
                )
                .into_any_element()
        } else if shift > Pixels::ZERO {
            anchor
                .child(
                    expanded_viewport(viewport_width)
                        .child(marquee_line(self.text, FADE_WIDTH))
                        .child(fade_layer),
                )
                .into_any_element()
        } else {
            anchor
                .flex()
                .items_center()
                .child(marquee_line(self.text, Pixels::ZERO))
                .into_any_element()
        }
    }
}

fn animated_edge_fades(color: Rgba, motion: Option<FadeMotion>) -> AnyElement {
    let layer = div().absolute().inset_0();
    match motion {
        Some(FadeMotion { id, fade }) if fade.animating => layer
            .with_animation(
                id,
                Animation::new(CONTROL_MOTION).with_easing(ease_in_out),
                move |element, delta| {
                    let progress = if fade.is_active { delta } else { 1.0 - delta };
                    let color = mix_color(fade.resting, fade.active, progress);
                    element
                        .child(edge_fade(FadeEdge::Left, color))
                        .child(edge_fade(FadeEdge::Right, color))
                },
            )
            .into_any_element(),
        _ => layer
            .child(edge_fade(FadeEdge::Left, color))
            .child(edge_fade(FadeEdge::Right, color))
            .into_any_element(),
    }
}

fn expanded_viewport(text_width: Pixels) -> Div {
    div()
        .absolute()
        .left(-FADE_WIDTH)
        .top(BORDER_INSET)
        .bottom(BORDER_INSET)
        .w(text_width + FADE_WIDTH * 2.0)
        .flex()
        .items_center()
        .overflow_hidden()
}

fn marquee_line(text: SharedString, offset: Pixels) -> Div {
    div()
        .relative()
        .left(offset)
        .flex_none()
        .whitespace_nowrap()
        .child(text)
}

#[derive(Clone, Copy)]
enum FadeEdge {
    Left,
    Right,
}

fn edge_fade(edge: FadeEdge, color: Rgba) -> Div {
    let transparent = color.opacity(0.0);
    let background = match edge {
        FadeEdge::Left => linear_gradient(
            90.0,
            linear_color_stop(color, 0.35),
            linear_color_stop(transparent, 1.0),
        ),
        FadeEdge::Right => linear_gradient(
            90.0,
            linear_color_stop(transparent, 0.0),
            linear_color_stop(color, 0.65),
        ),
    };
    let overlay = div()
        .absolute()
        .top_0()
        .bottom_0()
        .w(FADE_WIDTH)
        .bg(background);

    match edge {
        FadeEdge::Left => overlay.left_0(),
        FadeEdge::Right => overlay.right_0(),
    }
}

pub fn control_text_width(text: &SharedString, window: &mut Window) -> Pixels {
    let mut control_font = font(CONTROL_FONT_FAMILY);
    control_font.weight = FontWeight::MEDIUM;
    let run = TextRun {
        len: text.len(),
        font: control_font,
        color: window.text_style().color,
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    window
        .text_system()
        .shape_line(text.clone(), CONTROL_FONT_SIZE, &[run], None)
        .width()
}

fn marquee_shift(text_width: Pixels, viewport_width: Pixels) -> Pixels {
    if text_width > viewport_width {
        text_width - viewport_width
    } else {
        Pixels::ZERO
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Pixels, px};

    use super::marquee_shift;

    #[test]
    fn short_text_does_not_move() {
        assert_eq!(marquee_shift(px(80.0), px(100.0)), Pixels::ZERO);
    }

    #[test]
    fn overflowing_text_uses_only_the_hidden_width_as_travel() {
        assert_eq!(marquee_shift(px(140.0), px(100.0)), px(40.0));
    }
}
