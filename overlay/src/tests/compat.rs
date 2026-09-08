use super::*;
use windows::Win32::Foundation::RECT;

fn rect(left: i32, top: i32, right: i32, bottom: i32) -> RECT {
    RECT {
        left,
        top,
        right,
        bottom,
    }
}

#[test]
fn shy_window_is_one_pixel_short_of_the_monitor() {
    let monitor = rect(0, 0, 1920, 1080);
    assert!(is_one_px_shy(rect(0, 0, 1920, 1079), monitor));
    assert!(!is_one_px_shy(rect(0, 0, 1920, 1080), monitor));
    assert!(!is_one_px_shy(rect(100, 100, 1380, 820), monitor));
}

#[test]
fn covers_monitor_is_full_or_one_pixel_shy_not_windowed() {
    let monitor = rect(0, 0, 1920, 1080);
    assert!(is_full_monitor(rect(0, 0, 1920, 1080), monitor));
    assert!(covers_monitor(rect(0, 0, 1920, 1080), monitor));
    assert!(covers_monitor(rect(0, 0, 1920, 1079), monitor));
    assert!(!covers_monitor(rect(100, 100, 1380, 820), monitor));
    assert!(!is_full_monitor(rect(100, 100, 1380, 820), monitor));
    assert!(!is_full_monitor(rect(0, 0, 1920, 1079), monitor));
}

#[test]
fn quit_defers_taskbar_restore_only_while_the_game_is_up() {
    assert!(should_defer_taskbar_restore(true, true));
    assert!(!should_defer_taskbar_restore(true, false));
    assert!(!should_defer_taskbar_restore(false, true));
    assert!(!should_defer_taskbar_restore(false, false));
}

#[test]
fn restore_taskbar_pid_reads_flag_and_value() {
    assert_eq!(
        restore_taskbar_pid(["Holeshot-HUD.exe", "--restore-taskbar", "4242"]),
        Some(4242)
    );
    assert_eq!(restore_taskbar_pid(["--restore-taskbar=99"]), Some(99));
    assert_eq!(restore_taskbar_pid(["--wait-for-game"]), None);
    assert_eq!(restore_taskbar_pid(["--restore-taskbar", "nope"]), None);
}

#[test]
fn only_the_game_monitor_taskbar_hides_while_the_pointer_is_there() {
    assert!(should_hide_game_taskbar(true, true, false, true));
    assert!(!should_hide_game_taskbar(false, true, false, true));
    assert!(!should_hide_game_taskbar(true, false, false, true));
    assert!(!should_hide_game_taskbar(true, true, true, true));
    assert!(!should_hide_game_taskbar(true, true, false, false));
}

#[test]
fn overlay_stays_when_another_monitor_or_start_is_in_front() {
    assert!(overlay_stays_up(true, false, false, false, false));
    assert!(overlay_stays_up(false, true, false, false, false));
    assert!(overlay_stays_up(false, false, true, false, false));
    assert!(overlay_stays_up(false, false, false, true, false));
    assert!(overlay_stays_up(false, false, false, false, true));
    assert!(!overlay_stays_up(false, false, false, false, false));
}

#[test]
fn start_and_search_hosts_count_as_shell_ui() {
    assert!(is_shell_ui_class("Shell_TrayWnd"));
    assert!(is_shell_ui_class("Shell_SecondaryTrayWnd"));
    assert!(is_shell_ui_class("ImmersiveLauncher"));
    assert!(!is_shell_ui_class("MXBOOverlay"));
    assert!(is_shell_ui_process(r"C:\Windows\SystemApps\StartMenuExperienceHost.exe"));
    assert!(is_shell_ui_process(r"C:\Windows\System32\SearchHost.exe"));
    assert!(is_shell_ui_process(r"C:\Windows\System32\ShellHost.exe"));
    assert!(!is_shell_ui_process(r"C:\Games\MX Bikes\mxbikes.exe"));
    assert!(!is_shell_ui_process(r"C:\Windows\explorer.exe"));
}

#[test]
fn explorer_flyouts_count_as_shell_ui_not_file_explorer() {
    let explorer = r"C:\Windows\explorer.exe";
    assert!(is_shell_ui("XamlExplorerHostIslandWindow", explorer));
    assert!(is_shell_ui("MultitaskingViewFrame", explorer));
    assert!(is_shell_ui("TaskListThumbnailWnd", explorer));
    assert!(is_shell_ui("NotifyIconOverflowWindow", explorer));
    assert!(is_shell_ui(
        "Windows.UI.Core.CoreWindow",
        r"C:\Windows\SystemApps\ShellExperienceHost.exe"
    ));
    assert!(!is_shell_ui("CabinetWClass", explorer));
    assert!(!is_shell_ui(
        "Windows.UI.Core.CoreWindow",
        r"C:\Program Files\WindowsApps\Calculator.exe"
    ));
    assert!(!is_shell_ui("MXBOOverlay", r"C:\Games\MX Bikes\mxbikes.exe"));
}

#[test]
fn overlay_stays_on_the_game_monitor() {
    let game = rect(0, 0, 1920, 1080);
    assert_eq!(
        overlay_rect_on_monitor(rect(0, 0, 1920, 1080), game),
        Some((0, 0, 1920, 1080))
    );
    assert_eq!(
        overlay_rect_on_monitor(rect(0, 0, 1920, 1079), game),
        Some((0, 0, 1920, 1080))
    );
    assert!(flush_to_monitor_except_bottom_gap(
        rect(0, 0, 1920, 1079),
        game
    ));
    assert!(!flush_to_monitor_except_bottom_gap(
        rect(100, 80, 1380, 900),
        game
    ));
    assert_eq!(
        overlay_rect_on_monitor(rect(100, 80, 1380, 900), game),
        Some((100, 80, 1280, 820))
    );
    assert_eq!(
        overlay_rect_on_monitor(rect(-1920, 0, 1920, 1080), game),
        Some((0, 0, 1920, 1080))
    );
    assert_eq!(
        overlay_rect_on_monitor(rect(-1920, 0, 1920, 1080), rect(-1920, 0, 0, 1080)),
        Some((-1920, 0, 1920, 1080))
    );
    assert_eq!(
        overlay_rect_on_monitor(rect(0, 0, 50, 50), game),
        None
    );
}