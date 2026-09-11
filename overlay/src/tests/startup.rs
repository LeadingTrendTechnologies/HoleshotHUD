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

#[test]
fn hud_exe_rejects_escape_and_wrong_name() {
    assert!(sanitize_hud_exe(r"C:\foo\..\Holeshot-HUD.exe").is_none());
    assert!(sanitize_hud_exe(r"C:\Windows\notepad.exe").is_none());
    assert!(sanitize_hud_exe("").is_none());
}