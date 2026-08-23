use gpui::*;

use super::state::TextAreaState;
use crate::ui::foundation::colors;

pub const CARET_WIDTH: Pixels = px(2.0);
pub const CARET_HEIGHT: Pixels = px(14.0);
pub const LINE_HEIGHT: Pixels = px(18.0);
pub const PADDING: Pixels = px(8.0);

pub(super) struct TextAreaElement {
    state: Entity<TextAreaState>,
    caret_opacity: f32,
}

impl TextAreaElement {
    pub(super) fn new(state: Entity<TextAreaState>, caret_opacity: f32) -> Self {
        Self {
            state,
            caret_opacity,
        }
    }
}

pub(super) struct ShapedTextLine {
    pub(super) line_start: usize,
    pub(super) line_end: usize,
    pub(super) shaped: ShapedLine,
}

pub(super) struct PrepaintState {
    lines: Vec<ShapedTextLine>,
    selections: Vec<PaintQuad>,
    cursor: Option<PaintQuad>,
}

impl IntoElement for TextAreaElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextAreaElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let state = self.state.read(cx);
        let line_count = state.content.split('\n').count().max(1);
        let height = PADDING * 2.0 + (line_count as f32) * LINE_HEIGHT;

        let mut style = Style::default();
        style.size.width = relative(1.0).into();
        style.size.height = height.into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let state = self.state.read(cx);
        let content = state.content.clone();
        let is_empty = content.is_empty();
        let text_style = window.text_style();
        let font_size = text_style.font_size.to_pixels(window.rem_size());
        let focused = state.focus_handle.is_focused(window);
        let selected_range = state.selected_range.clone();
        let has_selection = focused && !selected_range.is_empty();
        let cursor = state.cursor;

        let mut lines = Vec::new();
        let mut selections = Vec::new();
        let mut cursor_quad = None;

        if is_empty {
            let placeholder = state.placeholder.clone();
            let run = TextRun {
                len: placeholder.len(),
                font: text_style.font(),
                color: colors::muted_foreground().into(),
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let shaped = window
                .text_system()
                .shape_line(placeholder, font_size, &[run], None);
            lines.push(ShapedTextLine {
                line_start: 0,
                line_end: 0,
                shaped,
            });

            if focused && self.caret_opacity > 0.0 {
                let cursor_origin = point(
                    bounds.left() + PADDING,
                    bounds.top() + PADDING + (LINE_HEIGHT - CARET_HEIGHT) / 2.0,
                );
                cursor_quad = Some(fill(
                    Bounds::new(cursor_origin, size(CARET_WIDTH, CARET_HEIGHT)),
                    colors::accent().opacity(self.caret_opacity),
                ));
            }
        } else {
            let base_run = TextRun {
                len: 0,
                font: text_style.font(),
                color: text_style.color,
                background_color: None,
                underline: None,
                strikethrough: None,
            };

            let mut byte_offset = 0;
            let split_lines: Vec<&str> = content.split('\n').collect();
            let total_lines = split_lines.len();

            for (idx, line_str) in split_lines.into_iter().enumerate() {
                let line_len = line_str.len();
                let line_start = byte_offset;
                let line_end = byte_offset + line_len;
                let line_top = bounds.top() + PADDING + (idx as f32) * LINE_HEIGHT;

                let run = TextRun {
                    len: line_len,
                    ..base_run.clone()
                };
                let shaped = window.text_system().shape_line(
                    line_str.to_owned().into(),
                    font_size,
                    &[run],
                    None,
                );

                if has_selection
                    && selected_range.start <= line_end
                    && selected_range.end >= line_start
                {
                    let sel_start_byte =
                        selected_range.start.max(line_start).min(line_end) - line_start;
                    let sel_end_byte =
                        selected_range.end.max(line_start).min(line_end) - line_start;

                    let x_start = shaped.x_for_index(sel_start_byte);
                    let mut x_end = if sel_end_byte > sel_start_byte {
                        shaped.x_for_index(sel_end_byte)
                    } else {
                        x_start
                    };

                    if selected_range.end > line_end && idx + 1 < total_lines {
                        x_end = (x_end + px(8.0)).max(x_start + px(8.0));
                    }

                    if x_end > x_start {
                        let sel_quad = Bounds::from_corners(
                            point(bounds.left() + PADDING + x_start, line_top),
                            point(bounds.left() + PADDING + x_end, line_top + LINE_HEIGHT),
                        );
                        selections.push(fill(sel_quad, colors::accent().opacity(0.30)));
                    }
                }

                if focused
                    && !has_selection
                    && cursor >= line_start
                    && (cursor <= line_end || (cursor == line_end + 1 && idx + 1 == total_lines))
                {
                    let cursor_byte = (cursor - line_start).min(line_len);
                    let cursor_x = bounds.left() + PADDING + shaped.x_for_index(cursor_byte);
                    let cursor_y = line_top + (LINE_HEIGHT - CARET_HEIGHT) / 2.0;
                    cursor_quad = Some(fill(
                        Bounds::new(point(cursor_x, cursor_y), size(CARET_WIDTH, CARET_HEIGHT)),
                        colors::accent().opacity(self.caret_opacity),
                    ));
                }

                lines.push(ShapedTextLine {
                    line_start,
                    line_end,
                    shaped,
                });

                byte_offset += line_len + 1;
            }
        }

        PrepaintState {
            lines,
            selections,
            cursor: cursor_quad,
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        for sel in prepaint.selections.drain(..) {
            window.paint_quad(sel);
        }

        for (idx, line) in prepaint.lines.iter().enumerate() {
            let line_top = bounds.top() + PADDING + (idx as f32) * LINE_HEIGHT;
            let text_origin = point(bounds.left() + PADDING, line_top);
            if let Err(error) =
                line.shaped
                    .paint(text_origin, LINE_HEIGHT, TextAlign::Left, None, window, cx)
            {
                eprintln!("failed to paint textarea line: {error}");
            }
        }

        if let Some(cursor) = prepaint.cursor.take() {
            window.paint_quad(cursor);
        }

        let state = self.state.read(cx);
        state.last_bounds.set(Some(bounds));
        let mut state_lines = state.last_layouts.borrow_mut();
        state_lines.clear();
        for l in &prepaint.lines {
            state_lines.push((l.line_start, l.line_end, l.shaped.clone()));
        }
    }
}
