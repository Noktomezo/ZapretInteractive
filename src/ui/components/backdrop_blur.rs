use gpui::prelude::*;
use gpui::*;

pub fn backdrop_blur(
    tint: Hsla,
    blur_radius: Pixels,
    corner_radius: Pixels,
    noise: f32,
) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            window.paint_backdrop_blur(PaintBackdropBlur {
                bounds,
                blur_radius,
                tint,
                corner_radii: Corners::all(corner_radius),
                noise,
            });
        },
    )
    .absolute()
    .inset_0()
}
