use std::cell::{Cell, RefCell};
use std::ops::Range;

use gpui::*;
use unicode_segmentation::UnicodeSegmentation;

use super::element::{LINE_HEIGHT, PADDING};

pub struct TextAreaState {
    pub(super) focus_handle: FocusHandle,
    pub(super) content: String,
    pub(super) cursor: usize,
    pub(super) selected_range: Range<usize>,
    pub(super) selection_anchor: Option<usize>,
    pub(super) is_dragging: bool,
    pub(super) placeholder: SharedString,
    pub(super) last_bounds: Cell<Option<Bounds<Pixels>>>,
    pub(super) last_layouts: RefCell<Vec<(usize, usize, ShapedLine)>>,
    _focus_subscription: Subscription,
    _blur_subscription: Subscription,
}

impl TextAreaState {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let focus_subscription = cx.on_focus(&focus_handle, window, |_, _, cx| {
            cx.notify();
        });
        let blur_subscription = cx.on_blur(&focus_handle, window, |this, _, cx| {
            this.selected_range = this.cursor..this.cursor;
            this.selection_anchor = None;
            this.is_dragging = false;
            cx.notify();
        });
        Self {
            focus_handle,
            content: String::new(),
            cursor: 0,
            selected_range: 0..0,
            selection_anchor: None,
            is_dragging: false,
            placeholder: SharedString::default(),
            last_bounds: Cell::new(None),
            last_layouts: RefCell::new(Vec::new()),
            _focus_subscription: focus_subscription,
            _blur_subscription: blur_subscription,
        }
    }

    pub fn value(&self) -> &str {
        &self.content
    }

    pub fn set_value(&mut self, value: impl Into<String>, cx: &mut Context<Self>) {
        self.content = value.into();
        self.cursor = self.content.len();
        self.selected_range = self.cursor..self.cursor;
        self.selection_anchor = None;
        cx.notify();
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

    pub fn selected_text(&self) -> &str {
        if self.selected_range.is_empty() {
            ""
        } else {
            let start = self.selected_range.start.min(self.content.len());
            let end = self.selected_range.end.min(self.content.len());
            &self.content[start..end]
        }
    }

    pub(super) fn delete_selection(&mut self) -> bool {
        if !self.selected_range.is_empty() {
            let start = self.selected_range.start.min(self.content.len());
            let end = self.selected_range.end.min(self.content.len());
            self.content.drain(start..end);
            self.cursor = start;
            self.selected_range = start..start;
            self.selection_anchor = None;
            true
        } else {
            false
        }
    }

    pub(super) fn insert_char(&mut self, ch: char, cx: &mut Context<Self>) {
        self.delete_selection();
        let byte_pos = self.clamp_cursor();
        self.content.insert(byte_pos, ch);
        self.cursor = byte_pos + ch.len_utf8();
        self.selected_range = self.cursor..self.cursor;
        self.selection_anchor = None;
        cx.notify();
    }

    pub(super) fn insert_str(&mut self, s: &str, cx: &mut Context<Self>) {
        self.delete_selection();
        let byte_pos = self.clamp_cursor();
        self.content.insert_str(byte_pos, s);
        self.cursor = byte_pos + s.len();
        self.selected_range = self.cursor..self.cursor;
        self.selection_anchor = None;
        cx.notify();
    }

    pub(super) fn backspace(&mut self, cx: &mut Context<Self>) {
        if self.delete_selection() {
            cx.notify();
            return;
        }
        let pos = self.clamp_cursor();
        if pos == 0 {
            return;
        }
        let prev = self.content[..pos]
            .grapheme_indices(true)
            .next_back()
            .map(|(idx, _)| idx)
            .unwrap_or(0);
        self.content.drain(prev..pos);
        self.cursor = prev;
        self.selected_range = prev..prev;
        self.selection_anchor = None;
        cx.notify();
    }

    pub(super) fn delete(&mut self, cx: &mut Context<Self>) {
        if self.delete_selection() {
            cx.notify();
            return;
        }
        let pos = self.clamp_cursor();
        if pos >= self.content.len() {
            return;
        }
        let next = self.content[pos..]
            .grapheme_indices(true)
            .nth(1)
            .map(|(idx, _)| pos + idx)
            .unwrap_or(self.content.len());
        self.content.drain(pos..next);
        self.selected_range = pos..pos;
        self.selection_anchor = None;
        cx.notify();
    }

    pub(super) fn select_to(&mut self, target: usize, cx: &mut Context<Self>) {
        let anchor = self.selection_anchor.unwrap_or(self.cursor);
        self.selection_anchor = Some(anchor);
        self.cursor = target.min(self.content.len());
        let start = anchor.min(self.cursor);
        let end = anchor.max(self.cursor);
        self.selected_range = start..end;
        cx.notify();
    }

    pub(super) fn move_left(&mut self, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            self.cursor = self.selected_range.start;
            self.selected_range = self.cursor..self.cursor;
            self.selection_anchor = None;
            cx.notify();
            return;
        }
        let pos = self.clamp_cursor();
        if pos > 0 {
            let prev = self.content[..pos]
                .grapheme_indices(true)
                .next_back()
                .map(|(idx, _)| idx)
                .unwrap_or(0);
            self.cursor = prev;
            self.selected_range = prev..prev;
            self.selection_anchor = None;
            cx.notify();
        }
    }

    pub(super) fn move_right(&mut self, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            self.cursor = self.selected_range.end;
            self.selected_range = self.cursor..self.cursor;
            self.selection_anchor = None;
            cx.notify();
            return;
        }
        let pos = self.clamp_cursor();
        if pos < self.content.len() {
            let next = self.content[pos..]
                .grapheme_indices(true)
                .nth(1)
                .map(|(idx, _)| pos + idx)
                .unwrap_or(self.content.len());
            self.cursor = next;
            self.selected_range = next..next;
            self.selection_anchor = None;
            cx.notify();
        }
    }

    pub(super) fn select_left(&mut self, cx: &mut Context<Self>) {
        let pos = self.clamp_cursor();
        if pos > 0 {
            let prev = self.content[..pos]
                .grapheme_indices(true)
                .next_back()
                .map(|(idx, _)| idx)
                .unwrap_or(0);
            self.select_to(prev, cx);
        }
    }

    pub(super) fn select_right(&mut self, cx: &mut Context<Self>) {
        let pos = self.clamp_cursor();
        if pos < self.content.len() {
            let next = self.content[pos..]
                .grapheme_indices(true)
                .nth(1)
                .map(|(idx, _)| pos + idx)
                .unwrap_or(self.content.len());
            self.select_to(next, cx);
        }
    }

    fn target_up_offset(&self) -> Option<usize> {
        let pos = self.cursor.min(self.content.len());
        let before = &self.content[..pos];
        let current_line_start = before.rfind('\n').map(|idx| idx + 1).unwrap_or(0);
        let col = pos - current_line_start;

        if current_line_start > 0 {
            let prev_line_content = &self.content[..current_line_start - 1];
            let prev_line_start = prev_line_content
                .rfind('\n')
                .map(|idx| idx + 1)
                .unwrap_or(0);
            let prev_line_len = (current_line_start - 1) - prev_line_start;
            let target_col = col.min(prev_line_len);
            Some(prev_line_start + target_col)
        } else {
            None
        }
    }

    fn target_down_offset(&self) -> Option<usize> {
        let pos = self.cursor.min(self.content.len());
        let before = &self.content[..pos];
        let current_line_start = before.rfind('\n').map(|idx| idx + 1).unwrap_or(0);
        let col = pos - current_line_start;

        if let Some(next_newline) = self.content[pos..].find('\n') {
            let next_line_start = pos + next_newline + 1;
            let next_line_content = &self.content[next_line_start..];
            let next_line_len = next_line_content
                .find('\n')
                .unwrap_or(next_line_content.len());
            let target_col = col.min(next_line_len);
            Some(next_line_start + target_col)
        } else {
            None
        }
    }

    fn line_start_offset(&self) -> usize {
        let pos = self.cursor.min(self.content.len());
        self.content[..pos]
            .rfind('\n')
            .map(|idx| idx + 1)
            .unwrap_or(0)
    }

    fn line_end_offset(&self) -> usize {
        let pos = self.cursor.min(self.content.len());
        if let Some(nl) = self.content[pos..].find('\n') {
            pos + nl
        } else {
            self.content.len()
        }
    }

    pub(super) fn move_up(&mut self, cx: &mut Context<Self>) {
        if let Some(target) = self.target_up_offset() {
            self.cursor = target;
            self.selected_range = target..target;
            self.selection_anchor = None;
            self.clamp_cursor();
            cx.notify();
        }
    }

    pub(super) fn move_down(&mut self, cx: &mut Context<Self>) {
        if let Some(target) = self.target_down_offset() {
            self.cursor = target;
            self.selected_range = target..target;
            self.selection_anchor = None;
            self.clamp_cursor();
            cx.notify();
        }
    }

    pub(super) fn move_home(&mut self, cx: &mut Context<Self>) {
        let target = self.line_start_offset();
        self.cursor = target;
        self.selected_range = target..target;
        self.selection_anchor = None;
        cx.notify();
    }

    pub(super) fn move_end(&mut self, cx: &mut Context<Self>) {
        let target = self.line_end_offset();
        self.cursor = target;
        self.selected_range = target..target;
        self.selection_anchor = None;
        cx.notify();
    }

    pub(super) fn select_up(&mut self, cx: &mut Context<Self>) {
        if let Some(target) = self.target_up_offset() {
            self.select_to(target, cx);
        }
    }

    pub(super) fn select_down(&mut self, cx: &mut Context<Self>) {
        if let Some(target) = self.target_down_offset() {
            self.select_to(target, cx);
        }
    }

    pub(super) fn select_home(&mut self, cx: &mut Context<Self>) {
        let target = self.line_start_offset();
        self.select_to(target, cx);
    }

    pub(super) fn select_end(&mut self, cx: &mut Context<Self>) {
        let target = self.line_end_offset();
        self.select_to(target, cx);
    }

    pub(super) fn select_all(&mut self, cx: &mut Context<Self>) {
        self.selected_range = 0..self.content.len();
        self.selection_anchor = Some(0);
        self.cursor = self.content.len();
        cx.notify();
    }

    pub(super) fn offset_for_position(&self, mouse_pos: Point<Pixels>) -> usize {
        if self.content.is_empty() {
            return 0;
        }
        let Some(bounds) = self.last_bounds.get() else {
            return self.cursor.min(self.content.len());
        };
        let layouts = self.last_layouts.borrow();
        if layouts.is_empty() {
            return self.cursor.min(self.content.len());
        }

        let rel_y = (mouse_pos.y - bounds.top() - PADDING).max(px(0.));
        let line_ix = (f32::from(rel_y) / f32::from(LINE_HEIGHT)).floor() as usize;
        let target_ix = line_ix.min(layouts.len() - 1);
        let (line_start, line_end, ref shaped) = layouts[target_ix];

        let rel_x = (mouse_pos.x - bounds.left() - PADDING).max(px(0.));
        let byte_in_line = shaped.closest_index_for_x(rel_x).min(line_end - line_start);
        (line_start + byte_in_line).min(self.content.len())
    }

    pub(super) fn mouse_down(
        &mut self,
        pos: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.focus_handle.focus(window, cx);
        let offset = self.offset_for_position(pos);
        self.cursor = offset;
        self.selection_anchor = Some(offset);
        self.selected_range = offset..offset;
        self.is_dragging = true;
        cx.notify();
    }

    pub(super) fn mouse_move(&mut self, pos: Point<Pixels>, cx: &mut Context<Self>) {
        if self.is_dragging {
            let current_offset = self.offset_for_position(pos);
            self.cursor = current_offset;
            if let Some(anchor) = self.selection_anchor {
                let start = anchor.min(current_offset);
                let end = anchor.max(current_offset);
                self.selected_range = start..end;
            }
            cx.notify();
        }
    }

    pub(super) fn mouse_up(&mut self, cx: &mut Context<Self>) {
        self.is_dragging = false;
        cx.notify();
    }

    fn clamp_cursor(&mut self) -> usize {
        self.cursor = self.cursor.min(self.content.len());
        while !self.content.is_char_boundary(self.cursor) && self.cursor > 0 {
            self.cursor -= 1;
        }
        self.cursor
    }
}
