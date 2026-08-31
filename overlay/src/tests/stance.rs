use super::*;

#[test]
fn toggle_flips_on_press() {
    let (sit, prev) = apply_edge(false, false, true, StanceMode::Toggle);
    assert!(sit);
    assert!(prev);
    let (sit, prev) = apply_edge(true, true, true, StanceMode::Toggle);
    assert!(sit);
    assert!(prev);
    let (sit, prev) = apply_edge(true, true, false, StanceMode::Toggle);
    assert!(sit);
    assert!(!prev);
    let (sit, _) = apply_edge(true, false, true, StanceMode::Toggle);
    assert!(!sit);
}

#[test]
fn hold_follows_button() {
    assert_eq!(apply_edge(false, false, true, StanceMode::Hold), (true, true));
    assert_eq!(apply_edge(true, true, true, StanceMode::Hold), (true, true));
    assert_eq!(apply_edge(true, true, false, StanceMode::Hold), (false, false));
}

#[test]
fn capture_takes_first_new_press() {
    let rb = bind_bit(StanceBind::PadRb);
    let a = bind_bit(StanceBind::PadA);
    assert_eq!(rising_bind(0, a), Some(StanceBind::PadA));
    assert_eq!(rising_bind(a, a), None);
    assert_eq!(rising_bind(rb, rb | a), Some(StanceBind::PadA));
    assert_eq!(rising_bind(0, rb | a), Some(StanceBind::PadRb));
}

#[test]
fn capture_prefers_pad_then_mouse_then_key() {
    let mut prev = Snap::default();
    let mut now = Snap::default();
    now.mouse = 1;
    now.keys[0x20 / 8] |= 1 << (0x20 % 8);
    now.pad = bind_bit(StanceBind::PadA);
    assert_eq!(rising_input(&prev, &now), Some(StanceBind::PadA));
    now.pad = 0;
    assert_eq!(rising_input(&prev, &now), Some(StanceBind::MouseLeft));
    now.mouse = 0;
    assert_eq!(rising_input(&prev, &now), Some(StanceBind::Key(0x20)));
    prev.keys = now.keys;
    assert_eq!(rising_input(&prev, &now), None);
}

#[test]
fn dualsense_usb_square_is_not_l1() {
    let mut usb = [0u8; 64];
    usb[0] = 0x01;
    usb[8] = 0x08 | (1 << 4);
    let pad = ds_buttons(&usb).unwrap();
    assert!(ds_held(pad, StanceBind::PadX));
    assert!(!ds_held(pad, StanceBind::PadLb));
    assert!(!ds_held(pad, StanceBind::PadDpadUp));
    usb[8] = 0x08;
    usb[9] = 1;
    let pad = ds_buttons(&usb).unwrap();
    assert!(ds_held(pad, StanceBind::PadLb));
    assert!(!ds_held(pad, StanceBind::PadX));
    assert!(!ds_held(pad, StanceBind::PadRb));
}

#[test]
fn dualsense_hat_and_l2() {
    let mut usb = [0u8; 64];
    usb[0] = 0x01;
    usb[8] = 0;
    let pad = ds_buttons(&usb).unwrap();
    assert!(ds_held(pad, StanceBind::PadDpadUp));
    assert!(!ds_held(pad, StanceBind::PadDpadDown));
    usb[8] = 2;
    let pad = ds_buttons(&usb).unwrap();
    assert!(ds_held(pad, StanceBind::PadDpadRight));
    usb[8] = 0x08;
    usb[9] = 1 << 2;
    let pad = ds_buttons(&usb).unwrap();
    assert!(ds_held(pad, StanceBind::PadLt));
    assert!(!ds_held(pad, StanceBind::PadRt));
    usb[9] = 1 << 3;
    let pad = ds_buttons(&usb).unwrap();
    assert!(ds_held(pad, StanceBind::PadRt));
    usb[9] = 0;
    usb[5] = 40;
    let pad = ds_buttons(&usb).unwrap();
    assert!(ds_held(pad, StanceBind::PadLt));
}

#[test]
fn ds4_usb_square_and_l2() {
    let mut usb = [0u8; 64];
    usb[0] = 0x01;
    usb[5] = 0x08 | (1 << 4);
    let pad = ds4_buttons(&usb).unwrap();
    assert!(ds_held(pad, StanceBind::PadX));
    assert!(!ds_held(pad, StanceBind::PadDpadUp));
    usb[5] = 0x08;
    usb[6] = 1 << 2;
    usb[8] = 40;
    let pad = ds4_buttons(&usb).unwrap();
    assert!(ds_held(pad, StanceBind::PadLt));
    assert!(!ds_held(pad, StanceBind::PadRt));
}

#[test]
fn xinput_trigger_and_dpad() {
    assert!(xinput_held(XINPUT_GAMEPAD_DPAD_LEFT.0, 0, 0, StanceBind::PadDpadLeft));
    assert!(!xinput_held(0, 0, 0, StanceBind::PadLt));
    assert!(xinput_held(0, 40, 0, StanceBind::PadLt));
    assert!(xinput_held(0, 0, 40, StanceBind::PadRt));
    assert!(!xinput_held(0, TRIGGER_DOWN, 0, StanceBind::PadLt));
    assert!(!xinput_held(0, 0, 0, StanceBind::Key(0x20)));
    assert!(!xinput_held(0, 0, 0, StanceBind::MouseLeft));
}

#[test]
fn dualsense_bt_square_is_not_l1() {
    let mut bt = [0u8; 64];
    bt[0] = 0x31;
    bt[9] = 0x08 | (1 << 4);
    let pad = ds_buttons(&bt).unwrap();
    assert!(ds_held(pad, StanceBind::PadX));
    assert!(!ds_held(pad, StanceBind::PadLb));
    bt[9] = 0x08;
    bt[10] = 1;
    let pad = ds_buttons(&bt).unwrap();
    assert!(ds_held(pad, StanceBind::PadLb));
    assert!(!ds_held(pad, StanceBind::PadX));
}

#[test]
fn ds4_bt_square_and_l2() {
    let mut bt = [0u8; 64];
    bt[0] = 0x11;
    bt[7] = 0x08 | (1 << 4);
    let pad = ds4_buttons(&bt).unwrap();
    assert!(ds_held(pad, StanceBind::PadX));
    bt[7] = 0x08;
    bt[10] = 40;
    let pad = ds4_buttons(&bt).unwrap();
    assert!(ds_held(pad, StanceBind::PadLt));
    assert!(!ds_held(pad, StanceBind::PadRt));
}

#[test]
fn ds_buttons_rejects_empty() {
    assert!(ds_buttons(&[]).is_none());
    assert!(ds4_buttons(&[]).is_none());
}
