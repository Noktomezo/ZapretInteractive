use std::cell::Cell;
use std::rc::Rc;

use gpui::prelude::*;
use gpui::*;

use crate::ui::foundation::element_ext::ElementPrepaintExt as _;

use crate::ui::foundation::motion::{DropdownMotion, set_dropdown_open};

const WINDOW_MARGIN: Pixels = px(8.0);
const TRIGGER_GAP: Pixels = px(4.0);

pub fn adaptive_dropdown(
    id: &'static str,
    trigger: impl IntoElement,
    menu: impl IntoElement,
    motion: Entity<DropdownMotion>,
    cx: &App,
) -> AnyElement {
    let trigger_bounds = Rc::new(Cell::new(Bounds::default()));
    let measured_trigger_bounds = Rc::clone(&trigger_bounds);
    let toggle_motion = motion.clone();
    let open = motion.read(cx).open();
    let visible = motion.read(cx).visible();
    let trigger = div()
        .id(SharedString::from(format!("{id}-adaptive-trigger")))
        .on_prepaint(move |bounds, _, _| measured_trigger_bounds.set(bounds))
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            set_dropdown_open(&toggle_motion, !open, window, cx);
            cx.stop_propagation();
        })
        .child(trigger);

    let close_motion = motion;
    let outside_trigger_bounds = Rc::clone(&trigger_bounds);
    let menu = div()
        .id(SharedString::from(format!("{id}-adaptive-menu")))
        .on_mouse_down_out(move |event, window, cx| {
            if outside_trigger_bounds.get().contains(&event.position) {
                return;
            }
            set_dropdown_open(&close_motion, false, window, cx);
        })
        .child(menu);

    div()
        .child(trigger)
        .when(visible, |element| {
            element.child(
                deferred(AdaptiveMenu::new(trigger_bounds, menu.into_any_element()))
                    .with_priority(2),
            )
        })
        .into_any_element()
}

struct AdaptiveMenu {
    trigger_bounds: Rc<Cell<Bounds<Pixels>>>,
    child: AnyElement,
}

impl AdaptiveMenu {
    fn new(trigger_bounds: Rc<Cell<Bounds<Pixels>>>, child: AnyElement) -> Self {
        Self {
            trigger_bounds,
            child,
        }
    }
}

impl IntoElement for AdaptiveMenu {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for AdaptiveMenu {
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
        let menu_size = window.layout_bounds(*child_layout).size;
        let origin = dropdown_origin(self.trigger_bounds.get(), menu_size, window.viewport_size());
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
        self.child.paint(window, cx);
    }
}

fn dropdown_origin(
    trigger: Bounds<Pixels>,
    menu: Size<Pixels>,
    viewport: Size<Pixels>,
) -> Point<Pixels> {
    let room_below = viewport.height - WINDOW_MARGIN - trigger.bottom();
    let y = if menu.height + TRIGGER_GAP <= room_below {
        trigger.bottom() + TRIGGER_GAP
    } else {
        trigger.top() - TRIGGER_GAP - menu.height
    };
    let max_x = (viewport.width - WINDOW_MARGIN - menu.width).max(WINDOW_MARGIN);
    let max_y = (viewport.height - WINDOW_MARGIN - menu.height).max(WINDOW_MARGIN);
    point(
        trigger.left().clamp(WINDOW_MARGIN, max_x),
        y.clamp(WINDOW_MARGIN, max_y),
    )
}

#[cfg(test)]
mod tests {
    use super::dropdown_origin;
    use gpui::{Bounds, point, px, size};

    #[test]
    fn dropdown_flips_above_instead_of_overlapping_trigger() {
        let viewport = size(px(800.0), px(600.0));
        let menu = size(px(220.0), px(280.0));
        let low_trigger = Bounds::new(point(px(500.0), px(540.0)), size(px(220.0), px(34.0)));
        assert_eq!(
            dropdown_origin(low_trigger, menu, viewport),
            point(px(500.0), px(256.0))
        );

        let high_trigger = Bounds::new(point(px(500.0), px(40.0)), size(px(220.0), px(34.0)));
        assert_eq!(
            dropdown_origin(high_trigger, menu, viewport),
            point(px(500.0), px(78.0))
        );
    }
}
