use super::{CaptureListWheel, SmoothListState, coalesced_target};
use gpui::prelude::*;
use gpui::{
    AppContext, Context, Entity, IntoElement, Render, ScrollDelta, ScrollHandle, ScrollWheelEvent,
    StatefulInteractiveElement, TestAppContext, TouchPhase, Window, div, point, px, size,
};

#[test]
fn nested_wheel_is_applied_once_to_innermost_scroll() {
    let mut cx = TestAppContext::single();
    let inner_handle = ScrollHandle::new();
    let inner_state = cx.new(|_| SmoothListState::new());
    let outer_handle = ScrollHandle::new();
    let outer_state = cx.new(|_| SmoothListState::new());

    struct TestView {
        inner_handle: ScrollHandle,
        inner_state: Entity<SmoothListState>,
        outer_handle: ScrollHandle,
        outer_state: Entity<SmoothListState>,
    }

    impl Render for TestView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let inner = CaptureListWheel {
                child: div()
                    .id("test-inner-native-scroll")
                    .size_full()
                    .track_scroll(&self.inner_handle)
                    .overflow_y_scroll()
                    .child(div().h(px(1_000.)))
                    .into_any_element(),
                state: self.inner_state.clone(),
                handle: self.inner_handle.clone(),
                wheel_enabled: true,
            };

            CaptureListWheel {
                child: div()
                    .id("test-outer-native-scroll")
                    .size_full()
                    .track_scroll(&self.outer_handle)
                    .overflow_y_scroll()
                    .child(div().h(px(1_000.)).child(div().h(px(100.)).child(inner)))
                    .into_any_element(),
                state: self.outer_state.clone(),
                handle: self.outer_handle.clone(),
                wheel_enabled: true,
            }
        }
    }

    let draw_inner_state = inner_state.clone();
    let draw_outer_state = outer_state.clone();
    let cx = cx.add_empty_window();
    cx.draw(
        point(px(0.), px(0.)),
        size(px(100.), px(100.)),
        move |_, cx| {
            cx.new(|_| TestView {
                inner_handle: inner_handle.clone(),
                inner_state: draw_inner_state.clone(),
                outer_handle: outer_handle.clone(),
                outer_state: draw_outer_state.clone(),
            })
            .into_any_element()
        },
    );

    cx.simulate_event(ScrollWheelEvent {
        position: point(px(50.), px(50.)),
        delta: ScrollDelta::Pixels(point(px(0.), px(-20.))),
        modifiers: Default::default(),
        touch_phase: TouchPhase::Moved,
    });

    let (inner_target, outer_target) =
        cx.update(|_, cx| (inner_state.read(cx).target_y, outer_state.read(cx).target_y));
    assert_eq!(inner_target, px(-20.));
    assert_eq!(outer_target, px(0.));
}

#[test]
fn wheel_targets_accumulate_and_clamp() {
    assert_eq!(
        coalesced_target(px(-20.0), px(-40.0), px(-30.0), px(100.0)),
        px(-70.0)
    );
    assert_eq!(
        coalesced_target(px(-80.0), px(-90.0), px(-30.0), px(100.0)),
        px(-100.0)
    );
}

#[test]
fn reversing_wheel_direction_discards_old_momentum() {
    assert_eq!(
        coalesced_target(px(-40.0), px(-80.0), px(15.0), px(100.0)),
        px(-25.0)
    );
}
