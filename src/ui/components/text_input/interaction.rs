use gpui::*;
use unicode_segmentation::UnicodeSegmentation as _;

use super::TextInputEvent;
use super::actions::*;
use super::state::{TextInputState, normalize_single_line};

impl TextInputState {
    pub(super) fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let previous = self.previous_grapheme(self.cursor_offset());
            if previous == self.cursor_offset() {
                window.play_system_bell();
                return;
            }
            self.select_to(previous, cx);
        }
        self.replace_selection("", true, cx);
    }

    pub(super) fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let next = self.next_grapheme(self.cursor_offset());
            if next == self.cursor_offset() {
                window.play_system_bell();
                return;
            }
            self.select_to(next, cx);
        }
        self.replace_selection("", true, cx);
    }

    pub(super) fn delete_previous_word(
        &mut self,
        _: &DeletePreviousWord,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selected_range.is_empty() {
            let previous = self.previous_word(self.cursor_offset());
            if previous == self.cursor_offset() {
                window.play_system_bell();
                return;
            }
            self.select_to(previous, cx);
        }
        self.replace_selection("", true, cx);
    }

    pub(super) fn delete_next_word(
        &mut self,
        _: &DeleteNextWord,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selected_range.is_empty() {
            let next = self.next_word(self.cursor_offset());
            if next == self.cursor_offset() {
                window.play_system_bell();
                return;
            }
            self.select_to(next, cx);
        }
        self.replace_selection("", true, cx);
    }

    pub(super) fn move_left(&mut self, _: &MoveLeft, cx: &mut Context<Self>) {
        let offset = if self.selected_range.is_empty() {
            self.previous_grapheme(self.cursor_offset())
        } else {
            self.selected_range.start
        };
        self.move_to(offset, cx);
    }

    pub(super) fn move_right(&mut self, _: &MoveRight, cx: &mut Context<Self>) {
        let offset = if self.selected_range.is_empty() {
            self.next_grapheme(self.cursor_offset())
        } else {
            self.selected_range.end
        };
        self.move_to(offset, cx);
    }

    pub(super) fn move_previous_word(&mut self, _: &MovePreviousWord, cx: &mut Context<Self>) {
        let offset = if self.selected_range.is_empty() {
            self.previous_word(self.cursor_offset())
        } else {
            self.selected_range.start
        };
        self.move_to(offset, cx);
    }

    pub(super) fn move_next_word(&mut self, _: &MoveNextWord, cx: &mut Context<Self>) {
        let offset = if self.selected_range.is_empty() {
            self.next_word(self.cursor_offset())
        } else {
            self.selected_range.end
        };
        self.move_to(offset, cx);
    }

    pub(super) fn select_left(&mut self, _: &SelectLeft, cx: &mut Context<Self>) {
        self.select_to(self.previous_grapheme(self.cursor_offset()), cx);
    }

    pub(super) fn select_right(&mut self, _: &SelectRight, cx: &mut Context<Self>) {
        self.select_to(self.next_grapheme(self.cursor_offset()), cx);
    }

    pub(super) fn select_previous_word(&mut self, _: &SelectPreviousWord, cx: &mut Context<Self>) {
        self.select_to(self.previous_word(self.cursor_offset()), cx);
    }

    pub(super) fn select_next_word(&mut self, _: &SelectNextWord, cx: &mut Context<Self>) {
        self.select_to(self.next_word(self.cursor_offset()), cx);
    }

    pub(super) fn move_home(&mut self, _: &MoveHome, cx: &mut Context<Self>) {
        self.move_to(0, cx);
    }

    pub(super) fn move_end(&mut self, _: &MoveEnd, cx: &mut Context<Self>) {
        self.move_to(self.content.len(), cx);
    }

    pub(super) fn select_home(&mut self, _: &SelectHome, cx: &mut Context<Self>) {
        self.select_to(0, cx);
    }

    pub(super) fn select_end(&mut self, _: &SelectEnd, cx: &mut Context<Self>) {
        self.select_to(self.content.len(), cx);
    }

    pub(super) fn select_all(&mut self, _: &SelectAll, cx: &mut Context<Self>) {
        self.selected_range = 0..self.content.len();
        self.selection_reversed = false;
        cx.notify();
    }

    pub(super) fn copy(&mut self, _: &Copy, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_owned(),
            ));
        }
    }

    pub(super) fn cut(&mut self, _: &Cut, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            return;
        }
        self.copy(&Copy, cx);
        self.replace_selection("", true, cx);
    }

    pub(super) fn paste(&mut self, _: &Paste, _: &mut Window, cx: &mut Context<Self>) {
        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            return;
        };
        self.replace_selection(&normalize_single_line(&text), true, cx);
    }

    pub(super) fn undo(&mut self, _: &Undo, cx: &mut Context<Self>) {
        let Some(snapshot) = self.undo_stack.pop() else {
            return;
        };
        self.redo_stack.push(self.snapshot());
        self.restore(snapshot, cx);
    }

    pub(super) fn redo(&mut self, _: &Redo, cx: &mut Context<Self>) {
        let Some(snapshot) = self.redo_stack.pop() else {
            return;
        };
        self.undo_stack.push(self.snapshot());
        self.restore(snapshot, cx);
    }

    pub(super) fn escape(&mut self, _: &Escape, _: &mut Window, cx: &mut Context<Self>) {
        if self.clean_on_escape && !self.content.is_empty() {
            self.set_value(String::new(), cx);
        }
    }

    pub(super) fn enter(&mut self, _: &Enter, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(TextInputEvent::PressEnter);
    }

    pub(super) fn show_character_palette(
        &mut self,
        _: &ShowCharacterPalette,
        window: &mut Window,
        _: &mut Context<Self>,
    ) {
        window.show_character_palette();
    }

    pub(super) fn mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        self.is_selecting = true;
        let offset = self.index_for_mouse_position(event.position);
        match event.click_count {
            3.. => self.select_all(&SelectAll, cx),
            2 => self.select_word_at(offset, cx),
            _ if event.modifiers.shift => self.select_to(offset, cx),
            _ => self.move_to(offset, cx),
        }
    }

    pub(super) fn mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }

    pub(super) fn mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_selecting {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        }
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        let offset = self.clamp_boundary(offset);
        self.selected_range = offset..offset;
        self.selection_reversed = false;
        self.marked_range = None;
        cx.notify();
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        let offset = self.clamp_boundary(offset);
        if self.selection_reversed {
            self.selected_range.start = offset;
        } else {
            self.selected_range.end = offset;
        }
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        self.marked_range = None;
        cx.notify();
    }

    fn select_word_at(&mut self, offset: usize, cx: &mut Context<Self>) {
        let offset = self.clamp_boundary(offset);
        let range = self
            .content
            .split_word_bound_indices()
            .find_map(|(start, word)| {
                let end = start + word.len();
                (start <= offset && offset <= end && !word.trim().is_empty()).then_some(start..end)
            })
            .unwrap_or(offset..offset);
        self.selected_range = range;
        self.selection_reversed = false;
        cx.notify();
    }

    fn previous_grapheme(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .rev()
            .find_map(|(index, _)| (index < offset).then_some(index))
            .unwrap_or(0)
    }

    fn next_grapheme(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .find_map(|(index, _)| (index > offset).then_some(index))
            .unwrap_or(self.content.len())
    }

    fn previous_word(&self, offset: usize) -> usize {
        self.content[..self.clamp_boundary(offset)]
            .split_word_bound_indices()
            .rev()
            .find_map(|(index, word)| word.chars().any(char::is_alphanumeric).then_some(index))
            .unwrap_or(0)
    }

    fn next_word(&self, offset: usize) -> usize {
        self.content
            .split_word_bound_indices()
            .find_map(|(index, word)| {
                (index > offset && word.chars().any(char::is_alphanumeric)).then_some(index)
            })
            .unwrap_or(self.content.len())
    }

    fn replace_selection(&mut self, new_text: &str, record_history: bool, cx: &mut Context<Self>) {
        let range = self
            .marked_range
            .take()
            .unwrap_or(self.selected_range.clone());
        self.replace_range(range, new_text, record_history, cx);
    }
}
