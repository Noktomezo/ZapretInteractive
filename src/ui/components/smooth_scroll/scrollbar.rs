use std::time::Instant;

use gpui::prelude::*;
use gpui::*;

use crate::ui::foundation::colors;
use crate::ui::foundation::motion::mix_color;

const THIN_WIDTH: Pixels = px(6.0);
const THICK_WIDTH: Pixels = px(8.0);
const THIN_INSET: Pixels = px(5.0);
const THICK_INSET: Pixels = px(4.0);
const SETTLE_DISTANCE: Pixels = px(0.25);
const VISUAL_RESPONSE_SECONDS: f32 = 0.045;

pub trait PageScrollHandle: Clone + 'static {
    fn bounds(&self) -> Bounds<Pixels>;
    fn max_scroll_y(&self) -> Pixels;
    fn offset_y(&self) -> Pixels;
    fn set_offset_y(&self, offset_y: Pixels);

    fn viewport_height(&self) -> Pixels {
        self.bounds().size.height
    }
}

impl PageScrollHandle for ScrollHandle {
    fn bounds(&self) -> Bounds<Pixels> {
        self.bounds()
    }

    fn max_scroll_y(&self) -> Pixels {
        self.max_offset().y
    }

    fn offset_y(&self) -> Pixels {
        self.offset().y
    }

    fn set_offset_y(&self, offset_y: Pixels) {
        let current = self.offset();
        self.set_offset(point(current.x, offset_y));
    }
}

impl PageScrollHandle for UniformListScrollHandle {
    fn bounds(&self) -> Bounds<Pixels> {
        self.0.borrow().base_handle.bounds()
    }

    fn max_scroll_y(&self) -> Pixels {
        self.0.borrow().base_handle.max_offset().y
    }

    fn offset_y(&self) -> Pixels {
        self.0.borrow().base_handle.offset().y
    }

    fn set_offset_y(&self, offset_y: Pixels) {
        let handle = self.0.borrow().base_handle.clone();
        let current = handle.offset();
        handle.set_offset(point(current.x, offset_y));
    }
}

impl PageScrollHandle for ListState {
    fn bounds(&self) -> Bounds<Pixels> {
        self.viewport_bounds()
    }

    fn max_scroll_y(&self) -> Pixels {
        self.max_offset_for_scrollbar().y
    }

    fn offset_y(&self) -> Pixels {
        self.scroll_px_offset_for_scrollbar().y
    }

    fn set_offset_y(&self, offset_y: Pixels) {
        self.set_offset_from_scrollbar(point(px(0.), offset_y));
    }
}

struct PageScrollbarState {
    hovered: bool,
    dragging: bool,
    expansion: f32,
    thumb_height: Option<Pixels>,
    drag_origin_y: Pixels,
    drag_start_offset_y: Pixels,
    last_frame: Instant,
}

impl PageScrollbarState {
    fn new() -> Self {
        Self {
            hovered: false,
            dragging: false,
            expansion: 0.0,
            thumb_height: None,
            drag_origin_y: Pixels::ZERO,
            drag_start_offset_y: Pixels::ZERO,
            last_frame: Instant::now(),
        }
    }

    fn advance(
        &mut self,
        target_height: Option<Pixels>,
        reduce_motion: bool,
    ) -> (Option<Pixels>, f32, bool) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;
        let target_expansion = if self.hovered || self.dragging {
            1.0
        } else {
            0.0
        };

        if reduce_motion {
            self.expansion = target_expansion;
            self.thumb_height = target_height;
            return (self.thumb_height, self.expansion, false);
        }

        let frame_seconds = elapsed.clamp(1.0 / 240.0, 1.0 / 30.0);
        let progress = 1.0 - (-frame_seconds / VISUAL_RESPONSE_SECONDS).exp();
        self.expansion = approach(self.expansion, target_expansion, progress, 0.01);
        self.thumb_height = match (self.thumb_height, target_height) {
            (None, Some(target)) => Some(target),
            (Some(current), Some(target)) => Some(approach_pixels(current, target, progress)),
            (Some(current), None) => {
                let next = approach_pixels(current, Pixels::ZERO, progress);
                (next > SETTLE_DISTANCE).then_some(next)
            }
            (None, None) => None,
        };

        let height_animating = match (self.thumb_height, target_height) {
            (Some(current), Some(target)) => (current - target).abs() > SETTLE_DISTANCE,
            (Some(_), None) | (None, Some(_)) => true,
            (None, None) => false,
        };
        let expansion_animating = (self.expansion - target_expansion).abs() > 0.01;
        (
            self.thumb_height,
            self.expansion,
            height_animating || expansion_animating,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ThumbTarget {
    container_height: Pixels,
    height: Pixels,
    progress: f32,
}

#[derive(IntoElement)]
pub struct PageScrollbar<H: PageScrollHandle> {
    id: ElementId,
    handle: H,
}

impl<H: PageScrollHandle> PageScrollbar<H> {
    pub fn new(id: impl Into<ElementId>, handle: H) -> Self {
        Self {
            id: id.into(),
            handle,
        }
    }
}

impl<H: PageScrollHandle> RenderOnce for PageScrollbar<H> {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window
            .use_keyed_state((self.id.clone(), "scrollbar-state"), cx, |_, _| {
                PageScrollbarState::new()
            })
            .clone();
        let target = thumb_target(
            self.handle.viewport_height(),
            self.handle.max_scroll_y(),
            self.handle.offset_y(),
        );
        let target_height = target.map(|target| target.height);
        let reduce_motion = cx.reduce_motion();
        let (thumb_height, expansion, animating) =
            state.update(cx, |state, _| state.advance(target_height, reduce_motion));
        if animating {
            window.request_animation_frame();
        }

        let hover_state = state.clone();
        let click_handle = self.handle.clone();
        let click_state = state.clone();
        let thumb = target.zip(thumb_height).map(|(target, height)| {
            let height = height.min(target.container_height);
            let top = (target.container_height - height) * target.progress;
            let width = THIN_WIDTH + (THICK_WIDTH - THIN_WIDTH) * expansion;
            let inset = THIN_INSET + (THICK_INSET - THIN_INSET) * expansion;
            let drag_state = state.clone();
            let drag_handle = self.handle.clone();
            div()
                .id((self.id.clone(), "scrollbar-thumb"))
                .absolute()
                .top(top)
                .right(inset)
                .w(width)
                .h(height)
                .rounded(width / 2.0)
                .bg(mix_color(colors::card(), colors::muted(), expansion))
                .border_1()
                .border_color(colors::border().opacity(0.6))
                .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                    drag_state.update(cx, |state, cx| {
                        state.dragging = true;
                        state.drag_origin_y = event.position.y;
                        state.drag_start_offset_y = drag_handle.offset_y();
                        state.last_frame = Instant::now();
                        cx.notify();
                    });
                    cx.stop_propagation();
                    window.refresh();
                })
        });
        let is_dragging = state.read(cx).dragging;
        let drag_overlay = is_dragging.then(|| {
            let capture_state = state.clone();
            let capture_handle = self.handle.clone();
            div()
                .id((self.id.clone(), "scrollbar-drag-overlay"))
                .absolute()
                .inset_0()
                .cursor(CursorStyle::Arrow)
                .on_mouse_move({
                    let capture_state = capture_state.clone();
                    move |event: &MouseMoveEvent, window, cx| {
                        if !event.dragging() {
                            return;
                        }
                        let (dragging, origin_y, start_offset) =
                            capture_state.read_with(cx, |state, _| {
                                (
                                    state.dragging,
                                    state.drag_origin_y,
                                    state.drag_start_offset_y,
                                )
                            });
                        let Some(target) = target.filter(|_| dragging) else {
                            return;
                        };
                        capture_handle.set_offset_y(dragged_offset(
                            target,
                            capture_handle.max_scroll_y(),
                            start_offset,
                            origin_y,
                            event.position.y,
                        ));
                        cx.stop_propagation();
                        window.refresh();
                    }
                })
                .on_mouse_up(MouseButton::Left, move |_, window, cx| {
                    let dragging = capture_state.read_with(cx, |state, _| state.dragging);
                    if dragging {
                        release_drag(&capture_state, window, cx);
                        cx.stop_propagation();
                    }
                })
        });

        div().absolute().inset_0().children(drag_overlay).child(
            div()
                .id((self.id.clone(), "scrollbar-zone"))
                .absolute()
                .top_0()
                .right_0()
                .bottom_0()
                .w(px(16.0))
                .on_hover(move |hovered, window, cx| {
                    hover_state.update(cx, |state, cx| {
                        if state.hovered == *hovered {
                            return;
                        }
                        state.hovered = *hovered;
                        state.last_frame = Instant::now();
                        cx.notify();
                    });
                    window.refresh();
                })
                .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                    let Some(target) = target else {
                        return;
                    };
                    let bounds = click_handle.bounds();
                    let track = (target.container_height - target.height).max(px(1.0));
                    let local_y = event.position.y - bounds.top() - target.height / 2.0;
                    let progress = (local_y / track).clamp(0.0, 1.0);
                    click_handle.set_offset_y(-click_handle.max_scroll_y() * progress);
                    click_state.update(cx, |state, cx| {
                        state.dragging = true;
                        state.drag_origin_y = event.position.y;
                        state.drag_start_offset_y = click_handle.offset_y();
                        cx.notify();
                    });
                    window.refresh();
                })
                .when_some(thumb, |zone, thumb| zone.child(thumb)),
        )
    }
}

fn release_drag(state: &Entity<PageScrollbarState>, window: &mut Window, cx: &mut App) {
    state.update(cx, |state, cx| {
        state.dragging = false;
        state.last_frame = Instant::now();
        cx.notify();
    });
    window.refresh();
}

fn dragged_offset(
    target: ThumbTarget,
    max_scroll: Pixels,
    start_offset: Pixels,
    origin_y: Pixels,
    current_y: Pixels,
) -> Pixels {
    let track = (target.container_height - target.height).max(px(1.0));
    let start_progress = (-start_offset / max_scroll).clamp(0.0, 1.0);
    let progress = (start_progress + (current_y - origin_y) / track).clamp(0.0, 1.0);
    -max_scroll * progress
}

fn thumb_target(
    container_height: Pixels,
    max_scroll: Pixels,
    offset_y: Pixels,
) -> Option<ThumbTarget> {
    if container_height <= Pixels::ZERO || max_scroll <= Pixels::ZERO {
        return None;
    }
    let content_height = container_height + max_scroll;
    let height = (container_height / content_height * container_height)
        .max(px(48.0))
        .min(container_height);
    let progress = (-offset_y / max_scroll).clamp(0.0, 1.0);
    Some(ThumbTarget {
        container_height,
        height,
        progress,
    })
}

fn approach(current: f32, target: f32, progress: f32, settle: f32) -> f32 {
    let next = current + (target - current) * progress;
    if (next - target).abs() <= settle {
        target
    } else {
        next
    }
}

fn approach_pixels(current: Pixels, target: Pixels, progress: f32) -> Pixels {
    let next = current + (target - current) * progress;
    if (next - target).abs() <= SETTLE_DISTANCE {
        target
    } else {
        next
    }
}

#[cfg(test)]
mod tests {
    use super::{PageScrollbarState, ThumbTarget, dragged_offset, thumb_target};
    use gpui::px;

    #[test]
    fn thumb_tracks_position_and_hides_without_overflow() {
        let top = thumb_target(px(100.0), px(300.0), px(0.0));
        let bottom = thumb_target(px(100.0), px(300.0), px(-300.0));
        assert_eq!(top.map(|target| target.progress), Some(0.0));
        assert_eq!(bottom.map(|target| target.progress), Some(1.0));
        assert!(thumb_target(px(100.0), px(0.0), px(0.0)).is_none());
    }

    #[test]
    fn visual_values_approach_without_overshooting() {
        let mut state = PageScrollbarState::new();
        state.hovered = true;
        state.last_frame -= std::time::Duration::from_millis(16);
        let (_, expansion, _) = state.advance(Some(px(80.0)), false);
        assert!((0.0..=1.0).contains(&expansion));
    }

    #[test]
    fn thumb_drag_uses_window_position_outside_the_scrollbar_zone() {
        let target = ThumbTarget {
            container_height: px(100.0),
            height: px(50.0),
            progress: 0.0,
        };
        assert_eq!(
            dragged_offset(target, px(300.0), px(0.0), px(20.0), px(70.0)),
            px(-300.0)
        );
    }
}
