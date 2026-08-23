use super::coalesced_target;
use gpui::px;

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
