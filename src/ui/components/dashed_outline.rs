use gpui::prelude::*;
use gpui::*;

pub fn dashed_outline(color: Hsla) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let inset = px(1.);
            let radius = px(7.);
            let left = bounds.left() + inset;
            let top = bounds.top() + inset;
            let right = bounds.right() - inset;
            let bottom = bounds.bottom() - inset;
            let mut outline = PathBuilder::stroke(px(2.)).dash_array(&[px(16.), px(8.)]);
            outline.move_to(point(left + radius, top));
            outline.line_to(point(right - radius, top));
            outline.arc_to(
                point(radius, radius),
                px(0.),
                false,
                true,
                point(right, top + radius),
            );
            outline.line_to(point(right, bottom - radius));
            outline.arc_to(
                point(radius, radius),
                px(0.),
                false,
                true,
                point(right - radius, bottom),
            );
            outline.line_to(point(left + radius, bottom));
            outline.arc_to(
                point(radius, radius),
                px(0.),
                false,
                true,
                point(left, bottom - radius),
            );
            outline.line_to(point(left, top + radius));
            outline.arc_to(
                point(radius, radius),
                px(0.),
                false,
                true,
                point(left + radius, top),
            );
            outline.close();
            if let Ok(outline) = outline.build() {
                window.paint_path(outline, color);
            }
        },
    )
    .absolute()
    .inset_0()
}
