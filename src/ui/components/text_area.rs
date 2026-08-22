use std::time::Duration;

use gpui::prelude::*;
use gpui::*;

mod actions;
mod element;
mod state;

pub use actions::init;
pub use state::TextAreaState;

use crate::ui::foundation::colors::{border, input};
use actions::*;
use element::TextAreaElement;

const CARET_BLINK_DURATION: Duration = Duration::from_millis(1_000);

fn caret_opacity(progress: f32) -> f32 {
    0.5 + 0.5 * (std::f32::consts::TAU * progress.clamp(0.0, 1.0)).cos()
}

#[derive(IntoElement)]
pub struct TextArea {
    state: Entity<TextAreaState>,
}

impl TextArea {
    pub fn new(state: &Entity<TextAreaState>) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

impl RenderOnce for TextArea {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let focus_handle = self.state.read(cx).focus_handle.clone();
        let focused = focus_handle.is_focused(window);
        let entity_id = self.state.entity_id().as_u64();
        let state = self.state.clone();

        let root = div()
            .id(ElementId::NamedInteger(
                "zapret-text-area".into(),
                entity_id,
            ))
            .key_context(actions::CONTEXT)
            .track_focus(&focus_handle)
            .cursor(CursorStyle::IBeam)
            .size_full()
            .min_h(px(120.))
            .max_h(px(220.))
            .overflow_y_scroll()
            .rounded_md()
            .border_1()
            .border_color(border().opacity(0.8))
            .bg(input().opacity(0.3))
            .text_size(px(12.))
            .line_height(px(18.))
            .font_family("IBM Plex Mono")
            .on_action({
                let state = state.clone();
                move |_: &Backspace, _, cx| {
                    state.update(cx, |s, cx| s.backspace(cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |_: &Delete, _, cx| {
                    state.update(cx, |s, cx| s.delete(cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |_: &Enter, _, cx| {
                    state.update(cx, |s, cx| s.insert_char('\n', cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |_: &MoveLeft, _, cx| {
                    state.update(cx, |s, cx| s.move_left(cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |_: &MoveRight, _, cx| {
                    state.update(cx, |s, cx| s.move_right(cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |_: &MoveUp, _, cx| {
                    state.update(cx, |s, cx| s.move_up(cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |_: &MoveDown, _, cx| {
                    state.update(cx, |s, cx| s.move_down(cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |_: &MoveHome, _, cx| {
                    state.update(cx, |s, cx| s.move_home(cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |_: &MoveEnd, _, cx| {
                    state.update(cx, |s, cx| s.move_end(cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |_: &SelectLeft, _, cx| {
                    state.update(cx, |s, cx| s.select_left(cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |_: &SelectRight, _, cx| {
                    state.update(cx, |s, cx| s.select_right(cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |_: &SelectUp, _, cx| {
                    state.update(cx, |s, cx| s.select_up(cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |_: &SelectDown, _, cx| {
                    state.update(cx, |s, cx| s.select_down(cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |_: &SelectHome, _, cx| {
                    state.update(cx, |s, cx| s.select_home(cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |_: &SelectEnd, _, cx| {
                    state.update(cx, |s, cx| s.select_end(cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |_: &SelectAll, _, cx| {
                    state.update(cx, |s, cx| s.select_all(cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |_: &Copy, _, cx| {
                    let sel = state.read(cx).selected_text().to_owned();
                    let text = if sel.is_empty() {
                        state.read(cx).content.clone()
                    } else {
                        sel
                    };
                    if !text.is_empty() {
                        cx.write_to_clipboard(ClipboardItem::new_string(text));
                    }
                }
            })
            .on_action({
                let state = state.clone();
                move |_: &Cut, _, cx| {
                    let sel = state.read(cx).selected_text().to_owned();
                    if !sel.is_empty() {
                        cx.write_to_clipboard(ClipboardItem::new_string(sel));
                        state.update(cx, |s, cx| {
                            s.delete_selection();
                            cx.notify();
                        });
                    } else {
                        let text = state.read(cx).content.clone();
                        if !text.is_empty() {
                            cx.write_to_clipboard(ClipboardItem::new_string(text));
                            state.update(cx, |s, cx| s.set_value(String::new(), cx));
                        }
                    }
                }
            })
            .on_action({
                let state = state.clone();
                move |_: &Paste, _, cx| {
                    if let Some(text) = cx
                        .read_from_clipboard()
                        .and_then(|item| item.text().map(|s| s.to_string()))
                    {
                        state.update(cx, |s, cx| s.insert_str(&text, cx));
                    }
                }
            })
            .on_mouse_down(MouseButton::Left, {
                let state = state.clone();
                move |event, window, cx| {
                    state.update(cx, |s, cx| s.mouse_down(event.position, window, cx));
                }
            })
            .on_mouse_move({
                let state = state.clone();
                move |event, _, cx| {
                    state.update(cx, |s, cx| s.mouse_move(event.position, cx));
                }
            })
            .on_mouse_up(MouseButton::Left, {
                let state = state.clone();
                move |_, _, cx| {
                    state.update(cx, |s, cx| s.mouse_up(cx));
                }
            })
            .on_mouse_up_out(MouseButton::Left, {
                let state = state.clone();
                move |_, _, cx| {
                    state.update(cx, |s, cx| s.mouse_up(cx));
                }
            })
            .on_mouse_down_out({
                let state = state.clone();
                let focus_handle = focus_handle.clone();
                move |_, window, cx| {
                    if focus_handle.is_focused(window) {
                        window.blur();
                        state.update(cx, |s, cx| {
                            s.selected_range = s.cursor..s.cursor;
                            s.selection_anchor = None;
                            s.is_dragging = false;
                            cx.notify();
                        });
                    }
                }
            })
            .on_key_down({
                let state = state.clone();
                move |event, _, cx| {
                    if let Some(ref text) = event.keystroke.key_char
                        && !event.keystroke.modifiers.control
                        && !event.keystroke.modifiers.alt
                        && !event.keystroke.modifiers.platform
                        && let Some(ch) = text.chars().next()
                        && !ch.is_control()
                    {
                        state.update(cx, |s, cx| s.insert_char(ch, cx));
                    }
                }
            });

        if focused {
            root.with_animation(
                ElementId::NamedInteger("zapret-text-area-caret".into(), entity_id),
                Animation::new(CARET_BLINK_DURATION).repeat(),
                move |element, delta| {
                    element.child(TextAreaElement::new(state.clone(), caret_opacity(delta)))
                },
            )
            .into_any_element()
        } else {
            root.child(TextAreaElement::new(state, 0.0))
                .into_any_element()
        }
    }
}
