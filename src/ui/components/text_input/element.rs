use std::ops::Range;

use gpui::*;

use super::TextInputState;
use crate::ui::foundation::colors;

const CARET_WIDTH: Pixels = px(2.0);

pub(super) struct TextElement {
    input: Entity<TextInputState>,
    caret_opacity: f32,
}

impl TextElement {
    pub(super) fn new(input: Entity<TextInputState>, caret_opacity: f32) -> Self {
        Self {
            input,
            caret_opacity,
        }
    }
}

pub(super) struct PrepaintState {
    line: ShapedLine,
    text_origin: Point<Pixels>,
    cursor: Option<PaintQuad>,
    selection: Option<PaintQuad>,
    scroll_x: Pixels,
}

impl IntoElement for TextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextElement {
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
        let mut style = Style::default();
        style.size.width = relative(1.0).into();
        style.size.height = relative(1.0).into();
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
        let input = self.input.read(cx);
        let content = input.content.clone();
        let display_text: SharedString = if content.is_empty() {
            input.placeholder.clone()
        } else {
            content.clone().into()
        };
        let text_style = window.text_style();
        let text_color = if content.is_empty() {
            colors::base_500().into()
        } else {
            text_style.color
        };
        let base_run = TextRun {
            len: display_text.len(),
            font: text_style.font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let runs = marked_runs(base_run, input.marked_range.as_ref(), display_text.len());
        let font_size = text_style.font_size.to_pixels(window.rem_size());
        let line = window
            .text_system()
            .shape_line(display_text, font_size, &runs, None);
        let cursor_x = if content.is_empty() {
            Pixels::ZERO
        } else {
            line.x_for_index(input.cursor_offset())
        };
        let scroll_x = adjusted_scroll(
            input.scroll_x.get(),
            cursor_x,
            line.width(),
            bounds.size.width,
        );
        let text_origin = point(bounds.left() - scroll_x, bounds.top());
        let focused = input.focus_handle.is_focused(window);
        let (selection, cursor) = selection_and_cursor(
            &line,
            &input.selected_range,
            bounds,
            scroll_x,
            focused,
            self.caret_opacity,
        );

        PrepaintState {
            line,
            text_origin,
            cursor,
            selection,
            scroll_x,
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
        let focus_handle = self.input.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );
        if let Some(selection) = prepaint.selection.take() {
            window.paint_quad(selection);
        }
        if let Err(error) = prepaint.line.paint(
            prepaint.text_origin,
            bounds.size.height,
            TextAlign::Left,
            None,
            window,
            cx,
        ) {
            eprintln!("failed to paint text input contents: {error}");
        }
        if let Some(cursor) = prepaint.cursor.take() {
            window.paint_quad(cursor);
        }

        let input = self.input.read(cx);
        *input.last_layout.borrow_mut() = Some(prepaint.line.clone());
        input.last_bounds.set(Some(bounds));
        input.scroll_x.set(prepaint.scroll_x);
    }
}

fn marked_runs(base: TextRun, marked: Option<&Range<usize>>, text_len: usize) -> Vec<TextRun> {
    let Some(marked) = marked else {
        return vec![base];
    };
    [
        TextRun {
            len: marked.start,
            ..base.clone()
        },
        TextRun {
            len: marked.end.saturating_sub(marked.start),
            underline: Some(UnderlineStyle {
                color: Some(base.color),
                thickness: px(1.0),
                wavy: false,
            }),
            ..base.clone()
        },
        TextRun {
            len: text_len.saturating_sub(marked.end),
            ..base
        },
    ]
    .into_iter()
    .filter(|run| run.len > 0)
    .collect()
}

fn adjusted_scroll(
    current: Pixels,
    cursor_x: Pixels,
    line_width: Pixels,
    viewport_width: Pixels,
) -> Pixels {
    let max_scroll = (line_width - viewport_width + CARET_WIDTH).max(Pixels::ZERO);
    if cursor_x < current {
        cursor_x.max(Pixels::ZERO)
    } else if cursor_x + CARET_WIDTH > current + viewport_width {
        (cursor_x + CARET_WIDTH - viewport_width).min(max_scroll)
    } else {
        current.min(max_scroll)
    }
}

fn selection_and_cursor(
    line: &ShapedLine,
    selected_range: &Range<usize>,
    bounds: Bounds<Pixels>,
    scroll_x: Pixels,
    focused: bool,
    caret_opacity: f32,
) -> (Option<PaintQuad>, Option<PaintQuad>) {
    if !focused {
        return (None, None);
    }
    if !selected_range.is_empty() {
        let selection = Bounds::from_corners(
            point(
                bounds.left() + line.x_for_index(selected_range.start) - scroll_x,
                bounds.top(),
            ),
            point(
                bounds.left() + line.x_for_index(selected_range.end) - scroll_x,
                bounds.bottom(),
            ),
        );
        return (Some(fill(selection, colors::accent().opacity(0.30))), None);
    }
    let cursor_x = bounds.left() + line.x_for_index(selected_range.start) - scroll_x;
    let cursor = caret_bounds(cursor_x, bounds, line.ascent + line.descent);
    (
        None,
        Some(fill(cursor, colors::accent().opacity(caret_opacity))),
    )
}

fn caret_bounds(
    cursor_x: Pixels,
    container: Bounds<Pixels>,
    text_height: Pixels,
) -> Bounds<Pixels> {
    let height = text_height.min(container.size.height);
    let top = container.top() + (container.size.height - height) / 2.0;
    Bounds::new(point(cursor_x, top), size(CARET_WIDTH, height))
}

#[cfg(test)]
mod tests {
    use super::{adjusted_scroll, caret_bounds};
    use gpui::{Bounds, point, px, size};

    #[test]
    fn horizontal_scroll_keeps_the_caret_inside_the_viewport() {
        assert_eq!(
            adjusted_scroll(px(0.0), px(120.0), px(160.0), px(100.0)),
            px(22.0)
        );
        assert_eq!(
            adjusted_scroll(px(30.0), px(10.0), px(160.0), px(100.0)),
            px(10.0)
        );
    }

    #[test]
    fn caret_is_centered_on_the_text_instead_of_filling_the_control() {
        let container = Bounds::new(point(px(5.0), px(10.0)), size(px(100.0), px(32.0)));
        let caret = caret_bounds(px(20.0), container, px(14.0));

        assert_eq!(caret.origin, point(px(20.0), px(19.0)));
        assert_eq!(caret.size, size(px(2.0), px(14.0)));
    }
}
