use std::time::Instant;

use gpui::prelude::*;
use gpui::*;

mod scrollbar;

pub use scrollbar::PageScrollbar;

const SETTLE_DISTANCE: Pixels = px(0.5);
const RESPONSE_SECONDS: f32 = 0.065;

#[derive(IntoElement)]
pub struct ScrollableColumn {
    id: ElementId,
    max_height: Pixels,
    child: AnyElement,
    base: Div,
}

impl ScrollableColumn {
    pub fn new(id: impl Into<ElementId>, max_height: Pixels, child: impl IntoElement) -> Self {
        Self {
            id: id.into(),
            max_height,
            child: child.into_any_element(),
            base: div(),
        }
    }
}

impl Styled for ScrollableColumn {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for ScrollableColumn {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let handle = window
            .use_keyed_state((self.id.clone(), "scroll-handle"), cx, |_, _| {
                ScrollHandle::new()
            })
            .read(cx)
            .clone();
        self.base
            .relative()
            .overflow_hidden()
            .max_h(self.max_height)
            .child(
                div()
                    .id((self.id.clone(), "area"))
                    .w_full()
                    .max_h(self.max_height)
                    .track_scroll(&handle)
                    .overflow_y_scroll()
                    .child(self.child),
            )
            .child(PageScrollbar::new(self.id, handle))
    }
}

struct SmoothListState {
    target_y: Pixels,
    running: bool,
    last_frame: Instant,
}

impl SmoothListState {
    fn new() -> Self {
        Self {
            target_y: Pixels::ZERO,
            running: false,
            last_frame: Instant::now(),
        }
    }
}

use crate::ui::foundation::colors;
use crate::ui::foundation::hover_motion;
use crate::ui::foundation::motion::mix_color;

/// A vertically scrollable area whose wheel input eases toward an accumulated target.
#[derive(IntoElement)]
pub struct SmoothVerticalScroll {
    id: ElementId,
    child: AnyElement,
    wheel_enabled: bool,
    scroll_to_top: bool,
}

impl SmoothVerticalScroll {
    /// Create a smooth vertical scroll area with a stable element id.
    pub fn new(id: impl Into<ElementId>, child: impl IntoElement) -> Self {
        Self {
            id: id.into(),
            child: child.into_any_element(),
            wheel_enabled: true,
            scroll_to_top: false,
        }
    }

    pub fn wheel_enabled(mut self, enabled: bool) -> Self {
        self.wheel_enabled = enabled;
        self
    }

    pub fn scroll_to_top(mut self, enabled: bool) -> Self {
        self.scroll_to_top = enabled;
        self
    }
}

impl RenderOnce for SmoothVerticalScroll {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window
            .use_keyed_state((self.id.clone(), "smooth-list-state"), cx, |_, _| {
                SmoothListState::new()
            })
            .clone();
        let handle = window
            .use_keyed_state((self.id.clone(), "scroll-handle"), cx, |_, _| {
                ScrollHandle::new()
            })
            .read(cx)
            .clone();

        let area = div()
            .id((self.id.clone(), "area"))
            .size_full()
            .flex()
            .flex_col()
            .track_scroll(&handle)
            .overflow_y_scroll()
            .child(div().w_full().flex_none().child(self.child));

        let captured = CaptureListWheel {
            child: area.into_any_element(),
            state: state.clone(),
            handle: handle.clone(),
            wheel_enabled: self.wheel_enabled,
        };

        let container = div()
            .id(self.id.clone())
            .relative()
            .size_full()
            .child(captured)
            .child(PageScrollbar::new(self.id.clone(), handle.clone()));

        with_scroll_to_top(container, self.id, state, handle, self.scroll_to_top, cx)
    }
}

/// Adds eased wheel scrolling to GPUI's fixed-height virtualized list.
#[derive(IntoElement)]
pub struct SmoothUniformListScroll {
    id: ElementId,
    handle: UniformListScrollHandle,
    child: AnyElement,
    wheel_enabled: bool,
    scroll_to_top: bool,
}

impl SmoothUniformListScroll {
    pub fn new(
        id: impl Into<ElementId>,
        handle: UniformListScrollHandle,
        child: impl IntoElement,
    ) -> Self {
        Self {
            id: id.into(),
            handle,
            child: child.into_any_element(),
            wheel_enabled: true,
            scroll_to_top: false,
        }
    }

    pub fn wheel_enabled(mut self, enabled: bool) -> Self {
        self.wheel_enabled = enabled;
        self
    }

    pub fn scroll_to_top(mut self, enabled: bool) -> Self {
        self.scroll_to_top = enabled;
        self
    }
}

impl RenderOnce for SmoothUniformListScroll {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window
            .use_keyed_state((self.id.clone(), "smooth-list-state"), cx, |_, _| {
                SmoothListState::new()
            })
            .clone();

        let handle = UniformScrollHandle(self.handle);
        let captured = CaptureListWheel {
            child: div()
                .id((self.id.clone(), "area"))
                .size_full()
                .child(self.child)
                .into_any_element(),
            state: state.clone(),
            handle: handle.clone(),
            wheel_enabled: self.wheel_enabled,
        };

        let container = div()
            .id(self.id.clone())
            .relative()
            .size_full()
            .child(captured);

        with_scroll_to_top(container, self.id, state, handle, self.scroll_to_top, cx)
    }
}

struct CaptureListWheel<H: SmoothVirtualHandle> {
    child: AnyElement,
    state: Entity<SmoothListState>,
    handle: H,
    wheel_enabled: bool,
}

impl<H: SmoothVirtualHandle> IntoElement for CaptureListWheel<H> {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl<H: SmoothVirtualHandle> Element for CaptureListWheel<H> {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _global_id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        (self.child.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _global_id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.child.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _global_id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if self.wheel_enabled {
            let state = self.state.clone();
            let handle = self.handle.clone();
            window.on_mouse_event(move |event: &ScrollWheelEvent, phase, window, cx| {
                if phase != DispatchPhase::Bubble || !bounds.contains(&event.position) {
                    return;
                }

                let delta = event.delta.pixel_delta(px(20.0));
                let delta_y = if delta.y.is_zero() { delta.x } else { delta.y };
                if delta_y.is_zero() {
                    return;
                }

                handle_list_wheel(&state, &handle, delta_y, window, cx);
                cx.stop_propagation();
            });
        }

        // Register outer handlers before painting children so nested scroll areas
        // receive bubble-phase wheel events first.
        self.child.paint(window, cx);
    }
}

/// Adds eased wheel scrolling to GPUI's dynamic-height virtualized list (ListState).
#[derive(IntoElement)]
pub struct SmoothListScroll {
    id: ElementId,
    state: ListState,
    child: AnyElement,
    wheel_enabled: bool,
    scroll_to_top: bool,
}

impl SmoothListScroll {
    pub fn new(id: impl Into<ElementId>, state: ListState, child: impl IntoElement) -> Self {
        Self {
            id: id.into(),
            state,
            child: child.into_any_element(),
            wheel_enabled: true,
            scroll_to_top: false,
        }
    }

    pub fn wheel_enabled(mut self, enabled: bool) -> Self {
        self.wheel_enabled = enabled;
        self
    }

    pub fn scroll_to_top(mut self, enabled: bool) -> Self {
        self.scroll_to_top = enabled;
        self
    }
}

impl RenderOnce for SmoothListScroll {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window
            .use_keyed_state((self.id.clone(), "smooth-list-state"), cx, |_, _| {
                SmoothListState::new()
            })
            .clone();

        let handle = self.state;
        let captured = CaptureListWheel {
            child: div()
                .id((self.id.clone(), "area"))
                .size_full()
                .flex()
                .flex_col()
                .child(self.child)
                .into_any_element(),
            state: state.clone(),
            handle: handle.clone(),
            wheel_enabled: self.wheel_enabled,
        };

        let container = div()
            .id(self.id.clone())
            .relative()
            .size_full()
            .child(captured);

        with_scroll_to_top(container, self.id, state, handle, self.scroll_to_top, cx)
    }
}

trait SmoothVirtualHandle: Clone + 'static {
    fn offset(&self) -> Point<Pixels>;
    fn max_scroll_y(&self) -> Pixels;
    fn set_offset(&self, offset: Point<Pixels>);
}

impl SmoothVirtualHandle for ListState {
    fn offset(&self) -> Point<Pixels> {
        self.scroll_px_offset_for_scrollbar()
    }

    fn max_scroll_y(&self) -> Pixels {
        self.max_offset_for_scrollbar().y
    }

    fn set_offset(&self, offset: Point<Pixels>) {
        self.set_offset_from_scrollbar(offset);
    }
}

impl SmoothVirtualHandle for ScrollHandle {
    fn offset(&self) -> Point<Pixels> {
        self.offset()
    }

    fn max_scroll_y(&self) -> Pixels {
        self.max_offset().y
    }

    fn set_offset(&self, offset: Point<Pixels>) {
        self.set_offset(offset);
    }
}

#[derive(Clone)]
struct UniformScrollHandle(UniformListScrollHandle);

impl SmoothVirtualHandle for UniformScrollHandle {
    fn offset(&self) -> Point<Pixels> {
        self.0.0.borrow().base_handle.offset()
    }

    fn max_scroll_y(&self) -> Pixels {
        self.0.0.borrow().base_handle.max_offset().y
    }

    fn set_offset(&self, offset: Point<Pixels>) {
        self.0.0.borrow().base_handle.set_offset(offset);
    }
}

fn with_scroll_to_top<H: SmoothVirtualHandle>(
    container: Stateful<Div>,
    id: ElementId,
    state: Entity<SmoothListState>,
    handle: H,
    enabled: bool,
    cx: &mut App,
) -> Stateful<Div> {
    if !enabled || handle.offset().y >= -px(160.0) {
        return container;
    }

    let hover_key = SharedString::from(format!("{id}-scroll-to-top-btn"));
    let hover = hover_motion::progress(&hover_key, cx);
    let border_color = mix_color(
        colors::border().opacity(0.8),
        colors::accent().opacity(0.7),
        hover,
    );
    let background_color = mix_color(
        colors::card().opacity(0.94),
        colors::secondary().opacity(0.96),
        hover,
    );
    let text_color = mix_color(colors::foreground(), colors::accent(), hover);
    let icon_color = mix_color(colors::muted_foreground(), colors::accent(), hover);
    let scroll_handle = handle.clone();
    let scroll_state = state.clone();
    let hover_key_for_event = hover_key.clone();

    container.child(
        div()
            .absolute()
            .bottom(px(20.))
            .left_0()
            .right_0()
            .flex()
            .justify_center()
            .child(
                div()
                    .id((id, "scroll-to-top-btn"))
                    .flex_none()
                    .h(crate::ui::foundation::control_style::CONTROL_HEIGHT)
                    .pl(px(10.))
                    .pr(px(12.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(6.))
                    .rounded_md()
                    .border_1()
                    .border_color(border_color)
                    .bg(background_color)
                    .shadow_lg()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(text_color)
                    .cursor_pointer()
                    .active(|style| style.opacity(0.85))
                    .on_hover(move |hovered, window, cx| {
                        hover_motion::set_hovered(
                            hover_key_for_event.clone(),
                            *hovered,
                            window,
                            cx,
                        );
                    })
                    .on_mouse_down(MouseButton::Left, |_, window, cx| {
                        window.prevent_default();
                        cx.stop_propagation();
                    })
                    .on_click(move |_, window, cx| {
                        let reduce_motion = cx.reduce_motion();
                        let should_schedule = scroll_state.update(cx, |state, _| {
                            let applied_offset = scroll_handle.offset();
                            state.target_y = Pixels::ZERO;
                            if reduce_motion {
                                scroll_handle.set_offset(point(applied_offset.x, Pixels::ZERO));
                                state.running = false;
                                false
                            } else {
                                state.running = true;
                                state.last_frame = Instant::now();
                                true
                            }
                        });

                        window.refresh();
                        if should_schedule {
                            schedule_list_frame(
                                scroll_state.clone(),
                                scroll_handle.clone(),
                                window,
                            );
                        }
                    })
                    .child(
                        svg()
                            .path("icons/arrow-up.svg")
                            .size(px(14.))
                            .flex_none()
                            .text_color(icon_color),
                    )
                    .child(
                        div()
                            .whitespace_nowrap()
                            .flex_none()
                            .child(t!("common.scroll_to_top")),
                    ),
            ),
    )
}

fn handle_list_wheel(
    state: &Entity<SmoothListState>,
    handle: &impl SmoothVirtualHandle,
    delta_y: Pixels,
    window: &mut Window,
    cx: &mut App,
) {
    let reduce_motion = cx.reduce_motion();
    let should_schedule = state.update(cx, |state, _| {
        let applied_offset = handle.offset();
        let max_scroll = handle.max_scroll_y();
        let current_y = applied_offset.y.clamp(-max_scroll, Pixels::ZERO);

        if !state.running {
            state.target_y = current_y;
        }
        state.target_y = coalesced_target(current_y, state.target_y, delta_y, max_scroll);

        if reduce_motion {
            handle.set_offset(point(applied_offset.x, state.target_y));
            state.running = false;
            false
        } else if state.running {
            false
        } else {
            state.running = true;
            state.last_frame = Instant::now();
            true
        }
    });

    window.refresh();
    if should_schedule {
        schedule_list_frame(state.clone(), handle.clone(), window);
    }
}

fn schedule_list_frame(
    state: Entity<SmoothListState>,
    handle: impl SmoothVirtualHandle,
    window: &Window,
) {
    window.on_next_frame(move |window, cx| advance_list_frame(state, handle, window, cx));
}

fn advance_list_frame(
    state: Entity<SmoothListState>,
    handle: impl SmoothVirtualHandle,
    window: &mut Window,
    cx: &mut App,
) {
    let keep_running = state.update(cx, |state, _| {
        let now = Instant::now();
        let elapsed = now.duration_since(state.last_frame).as_secs_f32();
        state.last_frame = now;

        let current = handle.offset();
        let max_scroll = handle.max_scroll_y();
        state.target_y = state.target_y.clamp(-max_scroll, Pixels::ZERO);
        let distance = state.target_y - current.y;
        if distance.abs() <= SETTLE_DISTANCE {
            handle.set_offset(point(current.x, state.target_y));
            state.running = false;
            return false;
        }

        let frame_seconds = elapsed.clamp(1.0 / 240.0, 1.0 / 30.0);
        let progress = 1.0 - (-frame_seconds / RESPONSE_SECONDS).exp();
        handle.set_offset(point(current.x, current.y + distance * progress));
        true
    });

    window.refresh();
    if keep_running {
        schedule_list_frame(state, handle, window);
    }
}

fn coalesced_target(current: Pixels, target: Pixels, delta: Pixels, max_scroll: Pixels) -> Pixels {
    let pending = target - current;
    let reverses_direction = (pending < Pixels::ZERO && delta > Pixels::ZERO)
        || (pending > Pixels::ZERO && delta < Pixels::ZERO);
    let next = if reverses_direction {
        current + delta
    } else {
        target + delta
    };
    next.clamp(-max_scroll, Pixels::ZERO)
}

#[cfg(test)]
mod tests;
