use super::*;

#[test]
fn quit_leaves_waiter_when_open_with_game_is_on() {
    assert!(should_leave_game_waiter(true, false));
}

#[test]
fn quit_does_not_leave_waiter_when_setting_is_off() {
    assert!(!should_leave_game_waiter(false, false));
}

#[test]
fn uninstall_does_not_leave_waiter() {
    assert!(!should_leave_game_waiter(true, true));
}
