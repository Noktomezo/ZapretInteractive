use std::time::Duration;

use gpui::prelude::*;
use gpui::*;

use crate::ui::foundation::colors;
use crate::ui::foundation::element_ext::ElementPrepaintExt as _;
use crate::ui::foundation::motion::TOOLTIP_MOTION;

const SHOW_DELAY: Duration = Duration::from_millis(80);
const CURSOR_OFFSET_X: Pixels = px(14.0);
const CURSOR_OFFSET_Y: Pixels = px(18.0);
const CURSOR_FLIP_GAP: Pixels = px(12.0);
const WINDOW_MARGIN: Pixels = px(8.0);

#[derive(Default)]
struct CursorTooltipState {
    source: Option<ElementId>,
    text: Option<SharedString>,
    position: Point<Pixels>,
    visible: bool,
    source_painted: bool,
    revision: u64,
    show_task: Option<Task<()>>,
}

impl Global for CursorTooltipState {}

impl CursorTooltipState {
    fn clear(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.source = None;
        self.text = None;
        self.visible = false;
        self.source_painted = false;
        self.show_task = None;
    }

    fn clear_source(&mut self, source: &ElementId) -> bool {
        if self.source.as_ref() != Some(source) {
            return false;
        }

        self.clear();
        true
    }

    fn consume_source_heartbeat(&mut self) -> bool {
        if std::mem::take(&mut self.source_painted) {
            true
        } else {
            self.clear();
            false
        }
    }
}

pub fn init(cx: &mut App) {
    cx.set_global(CursorTooltipState::default());
}

pub fn attach(
    element: Stateful<Div>,
    source: ElementId,
    text: impl Into<SharedString>,
) -> Stateful<Div> {
    let text = text.into();
    let hovered_source = source.clone();
    let pressed_source = source.clone();
    element
        .on_hover(move |hovered, window, cx| {
            set_hovered(hovered_source.clone(), text.clone(), *hovered, window, cx);
        })
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            hide_source(&pressed_source, window, cx);
        })
        .on_prepaint(move |_, _, cx| mark_source_painted(&source, cx))
}

pub fn attach_with_hover_motion(
    element: Stateful<Div>,
    source: ElementId,
    hover_key: SharedString,
    text: impl Into<SharedString>,
) -> Stateful<Div> {
    let text = text.into();
    let hovered_source = source.clone();
    let pressed_source = source.clone();
    element
        .on_hover(move |hovered, window, cx| {
            crate::ui::foundation::hover_motion::set_hovered(
                hover_key.clone(),
                *hovered,
                window,
                cx,
            );
            set_hovered(hovered_source.clone(), text.clone(), *hovered, window, cx);
        })
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            hide_source(&pressed_source, window, cx);
        })
        .on_prepaint(move |_, _, cx| mark_source_painted(&source, cx))
}

pub fn mark_source_painted(source: &ElementId, cx: &mut App) {
    let state = cx.global_mut::<CursorTooltipState>();
    if state.source.as_ref() == Some(source) {
        state.source_painted = true;
    }
}

pub fn set_hovered(
    source: ElementId,
    text: SharedString,
    hovered: bool,
    window: &mut Window,
    cx: &mut App,
) {
    if !hovered {
        hide_source(&source, window, cx);
        return;
    }

    let revision = {
        let state = cx.global_mut::<CursorTooltipState>();
        state.revision = state.revision.wrapping_add(1);
        state.source = Some(source);
        state.text = Some(text);
        state.position = window.mouse_position();
        state.visible = false;
        state.source_painted = true;
        state.show_task = None;
        state.revision
    };

    let task = window.spawn(cx, async move |cx| {
        cx.background_executor().timer(SHOW_DELAY).await;
        if let Err(error) = cx.update(|window, cx| {
            let state = cx.global_mut::<CursorTooltipState>();
            if state.revision != revision || state.text.is_none() {
                return;
            }
            state.visible = true;
            window.refresh();
        }) {
            eprintln!("failed to show cursor tooltip: {error}");
        }
    });
    cx.global_mut::<CursorTooltipState>().show_task = Some(task);
}

pub fn update_position(position: Point<Pixels>, cx: &mut App) -> bool {
    let state = cx.global_mut::<CursorTooltipState>();
    if state.source.is_none() {
        return false;
    }
    state.position = position;
    state.visible
}

pub fn hide_source(source: &ElementId, window: &mut Window, cx: &mut App) {
    let state = cx.global_mut::<CursorTooltipState>();
    let was_visible = state.visible;
    if state.clear_source(source) && was_visible {
        window.refresh();
    }
}

pub fn hide(window: &mut Window, cx: &mut App) {
    let state = cx.global_mut::<CursorTooltipState>();
    let was_visible = state.visible;
    state.clear();
    if was_visible {
        window.refresh();
    }
}

pub fn overlay(cx: &App) -> AnyElement {
    let state = cx.global::<CursorTooltipState>();
    if state.source.is_none() {
        return div().into_any_element();
    }
    let Some(text) = state.text.clone().filter(|_| state.visible) else {
        return liveness_guard().into_any_element();
    };

    let revision = state.revision;
    let tooltip = div()
        .px_2p5()
        .py_1()
        .bg(colors::card())
        .border_1()
        .border_color(colors::border())
        .rounded_md()
        .shadow_lg()
        .text_xs()
        .line_height(px(16.))
        .text_color(colors::foreground())
        .child(text)
        .with_animation(
            ElementId::NamedInteger("cursor-tooltip-enter".into(), revision),
            Animation::new(TOOLTIP_MOTION).with_easing(ease_in_out),
            |tooltip, delta| tooltip.opacity(delta).mt(px(4.0 * (1.0 - delta))),
        );

    deferred(CursorTooltip::new(
        state.position,
        tooltip.into_any_element(),
    ))
    .with_priority(4)
    .into_any_element()
}

fn liveness_guard() -> impl IntoElement {
    canvas(
        |_, _, _| (),
        |_, _, window, cx| {
            consume_source_heartbeat(window, cx);
        },
    )
    .absolute()
    .size_full()
}

fn consume_source_heartbeat(window: &mut Window, cx: &mut App) -> bool {
    let alive = cx
        .global_mut::<CursorTooltipState>()
        .consume_source_heartbeat();
    if !alive {
        window.refresh();
    }
    alive
}

struct CursorTooltip {
    cursor: Point<Pixels>,
    child: AnyElement,
}

impl CursorTooltip {
    fn new(cursor: Point<Pixels>, child: AnyElement) -> Self {
        Self { cursor, child }
    }
}

impl IntoElement for CursorTooltip {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for CursorTooltip {
    type RequestLayoutState = LayoutId;
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
        let child_layout = self.child.request_layout(window, cx);
        let layout = window.request_layout(
            Style {
                position: Position::Absolute,
                display: Display::Flex,
                ..Style::default()
            },
            [child_layout],
            cx,
        );
        (layout, child_layout)
    }

    fn prepaint(
        &mut self,
        _global_id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        child_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let tooltip_size = window.layout_bounds(*child_layout).size;
        let origin = tooltip_origin(self.cursor, tooltip_size, window.viewport_size());
        let offset = origin - bounds.origin;
        window.with_element_offset(offset, |window| self.child.prepaint(window, cx));
    }

    fn paint(
        &mut self,
        _global_id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if consume_source_heartbeat(window, cx) {
            self.child.paint(window, cx);
        }
    }
}

fn tooltip_origin(
    cursor: Point<Pixels>,
    tooltip: Size<Pixels>,
    viewport: Size<Pixels>,
) -> Point<Pixels> {
    let preferred_x = cursor.x + CURSOR_OFFSET_X;
    let x = if preferred_x + tooltip.width <= viewport.width - WINDOW_MARGIN {
        preferred_x
    } else {
        cursor.x - CURSOR_OFFSET_X - tooltip.width
    };
    let preferred_y = cursor.y + CURSOR_OFFSET_Y;
    let y = if preferred_y + tooltip.height <= viewport.height - WINDOW_MARGIN {
        preferred_y
    } else {
        cursor.y - CURSOR_FLIP_GAP - tooltip.height
    };
    let max_x = (viewport.width - WINDOW_MARGIN - tooltip.width).max(WINDOW_MARGIN);
    let max_y = (viewport.height - WINDOW_MARGIN - tooltip.height).max(WINDOW_MARGIN);
    point(x.clamp(WINDOW_MARGIN, max_x), y.clamp(WINDOW_MARGIN, max_y))
}

#[cfg(test)]
mod tests {
    use super::{CursorTooltipState, tooltip_origin};
    use gpui::{ElementId, SharedString, point, px, size};

    #[test]
    fn clearing_source_cancels_a_pending_or_visible_tooltip() {
        let source = ElementId::Name("delete-plugin".into());
        let other = ElementId::Name("other-button".into());
        let mut state = CursorTooltipState {
            source: Some(source.clone()),
            text: Some(SharedString::from("Delete")),
            visible: true,
            revision: 7,
            ..CursorTooltipState::default()
        };

        assert!(!state.clear_source(&other));
        assert!(state.visible);
        assert!(state.clear_source(&source));
        assert_eq!(state.revision, 8);
        assert!(state.source.is_none());
        assert!(state.text.is_none());
        assert!(!state.visible);
    }

    #[test]
    fn missing_source_heartbeat_clears_the_tooltip() {
        let mut state = CursorTooltipState {
            source: Some(ElementId::Name("remove-plugin".into())),
            text: Some(SharedString::from("Remove")),
            visible: true,
            source_painted: true,
            ..CursorTooltipState::default()
        };

        assert!(state.consume_source_heartbeat());
        assert!(state.source.is_some());
        assert!(!state.consume_source_heartbeat());
        assert!(state.source.is_none());
        assert!(!state.visible);
    }

    #[test]
    fn tooltip_tracks_cursor_and_flips_at_window_edges() {
        let viewport = size(px(800.0), px(600.0));
        let tooltip = size(px(120.0), px(30.0));

        assert_eq!(
            tooltip_origin(point(px(100.0), px(100.0)), tooltip, viewport),
            point(px(114.0), px(118.0))
        );
        assert_eq!(
            tooltip_origin(point(px(790.0), px(590.0)), tooltip, viewport),
            point(px(656.0), px(548.0))
        );
    }
}
