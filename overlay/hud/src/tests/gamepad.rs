use super::*;

#[test]
fn disconnected_is_none() {
    assert!(!PadState::DISCONNECTED.connected());
    assert_eq!(PadState::DISCONNECTED.kind, PadKind::None);
}

#[test]
fn demo_sony_shows_cross_right_bumper_and_r2() {
    let p = demo_sony();
    assert_eq!(p.kind, PadKind::Sony);
    assert!(p.down(SOUTH));
    assert!(p.down(RIGHT));
    assert!(p.down(LB));
    assert!(!p.down(RB));
    assert!((p.rt - 0.70).abs() < 0.001);
    assert!(p.lt < 0.01);
    assert!(p.lx < 0.0);
    assert!(p.ly < 0.0);
}

#[test]
fn demo_xbox_matches_sony_inputs() {
    let p = demo_xbox();
    assert_eq!(p.kind, PadKind::Xbox);
    assert!(p.down(SOUTH));
    assert!(p.down(RIGHT));
    assert!(p.down(LB));
    assert!((p.rt - 0.70).abs() < 0.001);
}

#[test]
fn hid_axis_center_is_zero() {
    assert!(axis_u8(128).abs() < 0.01);
    assert!(axis_u8(0) < -0.95);
    assert!(axis_u8(255) > 0.95);
}

#[test]
fn xinput_deadzone_holds_center() {
    assert_eq!(axis_i16(0, 7849), 0.0);
    assert_eq!(axis_i16(4000, 7849), 0.0);
    assert!(axis_i16(32767, 7849) > 0.99);
    assert!(axis_i16(-32768, 7849) < -0.99);
}

#[test]
fn trigger_scales() {
    assert_eq!(trigger_u8(0), 0.0);
    assert!((trigger_u8(128) - 128.0 / 255.0).abs() < 0.001);
    assert_eq!(trigger_u8(255), 1.0);
}
