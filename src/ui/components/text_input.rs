use std::time::Duration;

use gpui::prelude::*;
use gpui::*;

mod actions;
mod element;
mod input_handler;
mod interaction;
mod state;

pub use state::TextInputState;

use actions::*;
use element::TextElement;

const CARET_BLINK_DURATION: Duration = Duration::from_millis(1_000);

#[derive(Clone, Debug)]
pub enum TextInputEvent {
    Change,
    Focus,
    Blur,
    PressEnter,
}

impl EventEmitter<TextInputEvent> for TextInputState {}

pub fn init(cx: &mut App) {
    actions::init(cx);
}

fn caret_opacity(progress: f32) -> f32 {
    0.5 + 0.5 * (std::f32::consts::TAU * progress.clamp(0.0, 1.0)).cos()
}

#[derive(IntoElement)]
pub struct TextInput {
    state: Entity<TextInputState>,
}

impl TextInput {
    pub fn new(state: &Entity<TextInputState>) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

impl RenderOnce for TextInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let focus_handle = self.state.read(cx).focus_handle().clone();
        let focused = focus_handle.is_focused(window);
        let entity_id = self.state.entity_id().as_u64();
        let state = self.state.clone();
        let root = div()
            .id(ElementId::NamedInteger(
                "zapret-text-input".into(),
                entity_id,
            ))
            .key_context(actions::CONTEXT)
            .track_focus(&focus_handle)
            .cursor(CursorStyle::IBeam)
            .size_full()
            .min_w_0()
            .flex()
            .items_center()
            .on_action({
                let state = state.clone();
                move |action: &Backspace, window, cx| {
                    state.update(cx, |state, cx| state.backspace(action, window, cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |action: &Delete, window, cx| {
                    state.update(cx, |state, cx| state.delete(action, window, cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |action: &DeletePreviousWord, window, cx| {
                    state.update(cx, |state, cx| {
                        state.delete_previous_word(action, window, cx);
                    });
                }
            })
            .on_action({
                let state = state.clone();
                move |action: &DeleteNextWord, window, cx| {
                    state.update(cx, |state, cx| state.delete_next_word(action, window, cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |action: &MoveLeft, _, cx| {
                    state.update(cx, |state, cx| state.move_left(action, cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |action: &MoveRight, _, cx| {
                    state.update(cx, |state, cx| state.move_right(action, cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |action: &MovePreviousWord, _, cx| {
                    state.update(cx, |state, cx| state.move_previous_word(action, cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |action: &MoveNextWord, _, cx| {
                    state.update(cx, |state, cx| state.move_next_word(action, cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |action: &SelectLeft, _, cx| {
                    state.update(cx, |state, cx| state.select_left(action, cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |action: &SelectRight, _, cx| {
                    state.update(cx, |state, cx| state.select_right(action, cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |action: &SelectPreviousWord, _, cx| {
                    state.update(cx, |state, cx| state.select_previous_word(action, cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |action: &SelectNextWord, _, cx| {
                    state.update(cx, |state, cx| state.select_next_word(action, cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |action: &MoveHome, _, cx| {
                    state.update(cx, |state, cx| state.move_home(action, cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |action: &MoveEnd, _, cx| {
                    state.update(cx, |state, cx| state.move_end(action, cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |action: &SelectHome, _, cx| {
                    state.update(cx, |state, cx| state.select_home(action, cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |action: &SelectEnd, _, cx| {
                    state.update(cx, |state, cx| state.select_end(action, cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |action: &SelectAll, _, cx| {
                    state.update(cx, |state, cx| state.select_all(action, cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |action: &Copy, _, cx| {
                    state.update(cx, |state, cx| state.copy(action, cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |action: &Cut, window, cx| {
                    state.update(cx, |state, cx| state.cut(action, window, cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |action: &Paste, window, cx| {
                    state.update(cx, |state, cx| state.paste(action, window, cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |action: &Undo, _, cx| {
                    state.update(cx, |state, cx| state.undo(action, cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |action: &Redo, _, cx| {
                    state.update(cx, |state, cx| state.redo(action, cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |action: &Escape, window, cx| {
                    state.update(cx, |state, cx| state.escape(action, window, cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |action: &Enter, window, cx| {
                    state.update(cx, |state, cx| state.enter(action, window, cx));
                }
            })
            .on_action({
                let state = state.clone();
                move |action: &ShowCharacterPalette, window, cx| {
                    state.update(cx, |state, cx| {
                        state.show_character_palette(action, window, cx);
                    });
                }
            })
            .on_mouse_down(MouseButton::Left, {
                let state = state.clone();
                move |event, window, cx| {
                    state.update(cx, |state, cx| state.mouse_down(event, window, cx));
                }
            })
            .on_mouse_down_out(move |_, window, _| {
                if focus_handle.is_focused(window) {
                    window.blur();
                }
            });

        if focused {
            root.with_animation(
                ElementId::NamedInteger("zapret-text-input-caret".into(), entity_id),
                Animation::new(CARET_BLINK_DURATION).repeat(),
                move |element, delta| {
                    element.child(TextElement::new(state.clone(), caret_opacity(delta)))
                },
            )
            .into_any_element()
        } else {
            root.child(TextElement::new(state, 0.0)).into_any_element()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::caret_opacity;

    #[test]
    fn caret_fades_out_and_back_in() {
        assert!((caret_opacity(0.0) - 1.0).abs() < f32::EPSILON);
        assert!(caret_opacity(0.5).abs() < f32::EPSILON);
        assert!((caret_opacity(1.0) - 1.0).abs() < f32::EPSILON);
    }
}
