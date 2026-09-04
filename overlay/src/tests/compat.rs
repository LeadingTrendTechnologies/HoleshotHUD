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
fn only_the_game_monitor_taskbar_hides() {
    assert!(should_hide_taskbar(true));
    assert!(!should_hide_taskbar(false));
}

#[test]
fn overlay_stays_when_another_monitor_or_taskbar_is_in_front() {
    assert!(overlay_stays_up(true, false, false, false));
    assert!(overlay_stays_up(false, true, false, false));
    assert!(overlay_stays_up(false, false, true, false));
    assert!(overlay_stays_up(false, false, false, true));
    assert!(!overlay_stays_up(false, false, false, false));
}

#[test]
fn overlay_stays_on_the_game_monitor() {
    let game = rect(0, 0, 1920, 1080);
    assert_eq!(
        overlay_rect_on_monitor(rect(0, 0, 1920, 1080), game),
        Some((0, 0, 1920, 1080))
    );
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