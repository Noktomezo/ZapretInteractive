use std::cell::{Cell, RefCell};
use std::ops::Range;

use gpui::*;

use super::TextInputEvent;

const HISTORY_LIMIT: usize = 100;

#[derive(Clone)]
pub(super) struct Snapshot {
    content: String,
    selected_range: Range<usize>,
    selection_reversed: bool,
}

pub struct TextInputState {
    pub(super) focus_handle: FocusHandle,
    pub(super) content: String,
    pub(super) placeholder: SharedString,
    pub(super) selected_range: Range<usize>,
    pub(super) selection_reversed: bool,
    pub(super) marked_range: Option<Range<usize>>,
    pub(super) last_layout: RefCell<Option<ShapedLine>>,
    pub(super) last_bounds: Cell<Option<Bounds<Pixels>>>,
    pub(super) scroll_x: Cell<Pixels>,
    pub(super) is_selecting: bool,
    pub(super) clean_on_escape: bool,
    pub(super) undo_stack: Vec<Snapshot>,
    pub(super) redo_stack: Vec<Snapshot>,
    _focus_subscription: Subscription,
    _blur_subscription: Subscription,
}

impl TextInputState {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let focus_subscription = cx.on_focus(&focus_handle, window, |_, _, cx| {
            cx.emit(TextInputEvent::Focus);
            cx.notify();
        });
        let blur_subscription = cx.on_blur(&focus_handle, window, |this, _, cx| {
            let cursor = this.cursor_offset();
            this.selected_range = cursor..cursor;
            cx.emit(TextInputEvent::Blur);
            cx.notify();
        });
        Self {
            focus_handle,
            content: String::new(),
            placeholder: SharedString::default(),
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            last_layout: RefCell::new(None),
            last_bounds: Cell::new(None),
            scroll_x: Cell::new(Pixels::ZERO),
            is_selecting: false,
            clean_on_escape: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            _focus_subscription: focus_subscription,
            _blur_subscription: blur_subscription,
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn clean_on_escape(mut self) -> Self {
        self.clean_on_escape = true;
        self
    }

    pub fn value(&self) -> &str {
        &self.content
    }

    pub fn set_value(&mut self, value: impl Into<String>, cx: &mut Context<Self>) {
        let value = normalize_single_line(&value.into());
        if self.content == value {
            return;
        }
        self.push_undo();
        self.content = value;
        let end = self.content.len();
        self.selected_range = end..end;
        self.selection_reversed = false;
        self.marked_range = None;
        self.scroll_x.set(Pixels::ZERO);
        self.changed(cx);
    }

    pub fn set_placeholder(
        &mut self,
        placeholder: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        self.placeholder = placeholder.into();
        cx.notify();
    }

    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus_handle
    }

    pub(super) fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    pub(super) fn replace_range(
        &mut self,
        range: Range<usize>,
        new_text: &str,
        record_history: bool,
        cx: &mut Context<Self>,
    ) {
        let range = self.clamp_range(range);
        let new_text = normalize_single_line(new_text);
        if record_history {
            self.push_undo();
        }
        self.content.replace_range(range.clone(), &new_text);
        let cursor = range.start + new_text.len();
        self.selected_range = cursor..cursor;
        self.selection_reversed = false;
        self.changed(cx);
    }

    pub(super) fn snapshot(&self) -> Snapshot {
        Snapshot {
            content: self.content.clone(),
            selected_range: self.selected_range.clone(),
            selection_reversed: self.selection_reversed,
        }
    }

    pub(super) fn push_undo(&mut self) {
        self.undo_stack.push(self.snapshot());
        if self.undo_stack.len() > HISTORY_LIMIT {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    pub(super) fn restore(&mut self, snapshot: Snapshot, cx: &mut Context<Self>) {
        self.content = snapshot.content;
        self.selected_range = snapshot.selected_range;
        self.selection_reversed = snapshot.selection_reversed;
        self.marked_range = None;
        self.changed(cx);
    }

    pub(super) fn changed(&mut self, cx: &mut Context<Self>) {
        cx.emit(TextInputEvent::Change);
        cx.notify();
    }

    pub(super) fn clamp_boundary(&self, offset: usize) -> usize {
        let mut offset = offset.min(self.content.len());
        while offset > 0 && !self.content.is_char_boundary(offset) {
            offset -= 1;
        }
        offset
    }

    pub(super) fn clamp_range(&self, range: Range<usize>) -> Range<usize> {
        self.clamp_boundary(range.start)..self.clamp_boundary(range.end)
    }

    pub(super) fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_offset = 0;
        for character in self.content.chars() {
            if utf16_offset >= offset {
                break;
            }
            utf16_offset += character.len_utf16();
            utf8_offset += character.len_utf8();
        }
        utf8_offset
    }

    pub(super) fn offset_to_utf16(&self, offset: usize) -> usize {
        self.content[..self.clamp_boundary(offset)]
            .encode_utf16()
            .count()
    }

    pub(super) fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    pub(super) fn range_from_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range.start)..self.offset_from_utf16(range.end)
    }

    pub(super) fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        if self.content.is_empty() {
            return 0;
        }
        let Some(bounds) = self.last_bounds.get() else {
            return 0;
        };
        let layout_ref = self.last_layout.borrow();
        let Some(line) = layout_ref.as_ref() else {
            return 0;
        };
        if position.x <= bounds.left() {
            return 0;
        }
        if position.x >= bounds.right() {
            return self.content.len();
        }
        line.closest_index_for_x(position.x - bounds.left() + self.scroll_x.get())
    }
}

impl Focusable for TextInputState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

pub(super) fn normalize_single_line(text: &str) -> String {
    text.replace("\r\n", " ").replace(['\r', '\n'], " ")
}

#[cfg(test)]
mod tests {
    use super::normalize_single_line;

    #[test]
    fn normalizes_multiline_clipboard_text() {
        assert_eq!(normalize_single_line("one\r\ntwo"), "one two");
    }
}
