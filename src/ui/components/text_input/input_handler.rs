use std::ops::Range;

use gpui::*;

use super::state::{TextInputState, normalize_single_line};

impl EntityInputHandler for TextInputState {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.clamp_range(self.range_from_utf16(&range_utf16));
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.content[range].to_owned())
    }

    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _: &mut Window, _: &mut Context<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range| self.range_from_utf16(range))
            .or_else(|| self.marked_range.take())
            .unwrap_or_else(|| self.selected_range.clone());
        self.replace_range(range, new_text, true, cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let continuing_composition = self.marked_range.is_some();
        let range = range_utf16
            .as_ref()
            .map(|range| self.range_from_utf16(range))
            .or_else(|| self.marked_range.take())
            .unwrap_or_else(|| self.selected_range.clone());
        let range = self.clamp_range(range);
        let new_text = normalize_single_line(new_text);
        if !continuing_composition {
            self.push_undo();
        }
        self.content.replace_range(range.clone(), &new_text);
        self.marked_range =
            (!new_text.is_empty()).then_some(range.start..range.start + new_text.len());
        let relative_selection = new_selected_range_utf16
            .as_ref()
            .map(|selection| utf16_range_in_text(&new_text, selection));
        self.selected_range = relative_selection
            .map(|selection| range.start + selection.start..range.start + selection.end)
            .unwrap_or_else(|| {
                let end = range.start + new_text.len();
                end..end
            });
        self.selection_reversed = false;
        self.changed(cx);
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        _: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let layout_ref = self.last_layout.borrow();
        let line = layout_ref.as_ref()?;
        let bounds = self.last_bounds.get()?;
        let range = self.clamp_range(self.range_from_utf16(&range_utf16));
        Some(Bounds::from_corners(
            point(
                bounds.left() + line.x_for_index(range.start) - self.scroll_x.get(),
                bounds.top(),
            ),
            point(
                bounds.left() + line.x_for_index(range.end) - self.scroll_x.get(),
                bounds.bottom(),
            ),
        ))
    }

    fn character_index_for_point(
        &mut self,
        position: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        Some(self.offset_to_utf16(self.index_for_mouse_position(position)))
    }

    fn set_selected_text_range(
        &mut self,
        range_utf16: Range<usize>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.selected_range = self.clamp_range(self.range_from_utf16(&range_utf16));
        self.selection_reversed = false;
        cx.notify();
    }

    fn text_length_utf16(&mut self, _: &mut Window, _: &mut Context<Self>) -> Option<usize> {
        Some(self.content.encode_utf16().count())
    }
}

fn utf16_range_in_text(text: &str, range: &Range<usize>) -> Range<usize> {
    let mut utf8_start = text.len();
    let mut utf8_end = text.len();
    let mut utf16_offset = 0;
    let mut utf8_offset = 0;
    for character in text.chars() {
        if utf16_offset >= range.start && utf8_start == text.len() {
            utf8_start = utf8_offset;
        }
        if utf16_offset >= range.end {
            utf8_end = utf8_offset;
            break;
        }
        utf16_offset += character.len_utf16();
        utf8_offset += character.len_utf8();
    }
    if range.start >= utf16_offset {
        utf8_start = utf8_offset;
    }
    if range.end >= utf16_offset {
        utf8_end = utf8_offset;
    }
    utf8_start.min(text.len())..utf8_end.min(text.len())
}

#[cfg(test)]
mod tests {
    use super::utf16_range_in_text;

    #[test]
    fn converts_utf16_selection_inside_surrogate_pair_text() {
        assert_eq!(utf16_range_in_text("a😀b", &(1..3)), 1..5);
    }
}
