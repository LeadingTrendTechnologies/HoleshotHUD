use super::*;
use crate::config::{
    BoardField, DashField, FontFamily, HudConfig, LeanStyle, RelField, SessionPreset,
    StField, StanceStyle, WidgetId,
};
use crate::race_store::{
    effective_extra_laps, effective_race_laps, is_practice_session, live_session, session_preset,
    ClockMode,
};
use crate::shm::{write_name, Point, Rider, Snapshot, Standing, MAGIC, TRACK_NAME, VERSION};
use tiny_skia::{Color, Pixmap, Rect};

use super::render_support::*;
use super::render_support::draw;

#[test]
fn white_flag_waits_for_local_last_lap_not_leader_crossing() {
    let _g = session_lock();
    let mut s = live_snap();
    expire_timed(&mut s);
    s.local_speed = 18.0;
    assert_eq!(dash_race_flag(&s), DashFlag::None, "no flags until extras start");
    s.standings[0].num_laps = 6;
    assert_eq!(dash_race_flag(&s), DashFlag::None, "+2 still has 2 left — not white yet");
    s.standings[0].num_laps = 7;
    s.standings[1].num_laps = 5;
    s.current_lap = 6;
    assert_eq!(dash_race_flag(&s), DashFlag::None);
    // You start first extra: taken=1, done=0, left=2 → not last.
    s.standings[1].num_laps = 6;
    s.current_lap = 7;
    s.local_track_pos = 0.40;
    s.riders[1].track_pos = 0.40;
    assert_eq!(dash_race_flag(&s), DashFlag::None, "first of +2 is not white");
    // The run-in that starts your last extra is white.
    assert_eq!(ride_lap_to_line(&mut s), DashFlag::White);
    // Finish first / on last: done=1, left=1 → white.
    s.standings[1].num_laps = 7;
    s.current_lap = 8;
    assert_eq!(dash_race_flag(&s), DashFlag::White, "white on last remaining extra");
    // Complete both extras.
    s.standings[1].num_laps = 8;
    s.current_lap = 9;
    assert_eq!(dash_race_flag(&s), DashFlag::Checkered);
    s.on_track = 0;
    assert_eq!(dash_race_flag(&s), DashFlag::None);
}

/// The game only rescores you a frame or two after you are past the line, so the lap
/// rules still describe the lap you just finished. The flag that was out on the run-in has
/// to stay out across that gap instead of blinking off and back on.
#[test]
fn the_run_in_flag_stays_out_across_the_line() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 0;
    s.session_laps = 4;
    s.session_time_ms = 0;
    s.local_speed = 18.0;
    s.standings[0].num_laps = 2;
    s.standings[1].num_laps = 2;
    s.current_lap = 3;
    ride_to(&mut s, 0.50);
    assert_eq!(ride_lap_to_line(&mut s), DashFlag::White, "run-in onto the last lap");
    // Past the line, but the classification still has two laps to run.
    assert_eq!(ride_to(&mut s, 0.01), DashFlag::White, "white holds across the line");
    assert_eq!(cross_line(&mut s, 3, 4), DashFlag::White, "and the wave takes over");

    // Same across the finish.
    age_white_wave();
    assert_eq!(ride_lap_to_line(&mut s), DashFlag::Checkered);
    assert_eq!(
        ride_to(&mut s, 0.998),
        DashFlag::Checkered,
        "the last metres before the line are not a hole"
    );
    assert_eq!(
        ride_to(&mut s, 0.01),
        DashFlag::Checkered,
        "checkered holds across the line"
    );
    assert_eq!(
        CHECKERED_LATCH.load(Ordering::Relaxed),
        0,
        "held, not latched — the lap count has not confirmed the finish"
    );
    assert_eq!(cross_line(&mut s, 4, 5), DashFlag::Checkered);
    assert_eq!(CHECKERED_LATCH.load(Ordering::Relaxed), 1);
}

/// With no centerline and no learned line, the window position is a guess, so the early
/// white is suppressed. Lap counting still delivers white and checkered.
#[test]
fn no_track_geometry_still_flags_from_lap_count() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 0;
    s.session_laps = 4;
    s.session_time_ms = 0;
    s.local_speed = 18.0;
    s.poly_count = 0;
    s.sf_meters = 0.0;
    s.standings[0].num_laps = 2;
    s.standings[1].num_laps = 2;
    s.current_lap = 3;
    ride_to(&mut s, 0.50);
    assert_eq!(
        ride_lap_to_line(&mut s),
        DashFlag::None,
        "no early white without a known line"
    );
    // The last lap and the finish need no geometry at all.
    s.standings[1].num_laps = 3;
    s.current_lap = 4;
    assert_eq!(dash_race_flag(&s), DashFlag::White);
    s.standings[1].num_laps = 4;
    s.current_lap = 5;
    assert_eq!(dash_race_flag(&s), DashFlag::Checkered);
}

#[test]
fn green_flag_countdown_clock_shows_race_time_left() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 8;
    s.session_laps = 2;
    s.session_time_ms = 30_000;
    s.current_lap = 1;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    assert_eq!(session_banner(&s).1, "00:30");

    s.session_time_ms = 8 * 60 * 1000;
    s.local_speed = 18.0;
    let remain = session_remain_ms(&s).expect("race countdown after green");
    assert!(remain > 7 * 60 * 1000, "expected ~8 min left, got {remain}");
    assert_eq!(session_banner(&s).1, "08:00");
    assert!(!overtime_active(&s));
}

#[test]
fn green_flag_elapsed_clock_shows_race_time_left() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 8;
    s.session_laps = 2;
    s.session_time_ms = 30_000;
    s.current_lap = 1;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    assert_eq!(session_remain_ms(&s), Some(30_000));

    s.session_time_ms = 1_000;
    s.local_speed = 18.0;
    let remain = session_remain_ms(&s).expect("last seconds of the board");
    assert!(
        (remain - 1_000).abs() < 500,
        "00:01 gate must not become elapsed 07:59, got {remain}"
    );
    assert_eq!(session_banner(&s).1, "00:01");
    s.session_time_ms = 8 * 60 * 1000;
    let remain = session_remain_ms(&s).expect("race length after board");
    assert!(remain > 7 * 60 * 1000, "expected ~8 min left, got {remain}");
    assert_eq!(session_banner(&s).1, "08:00");
    assert!(!overtime_active(&s));
}

#[test]
fn finished_status_is_flag_even_when_crashed() {
    let _g = session_lock();
    reset_session();
    LEADER_FIN_LOCAL_BASE.store(4, Ordering::Relaxed);
    let mut s = live_snap();
    s.standing_count = 1;
    s.standings[0] = standing(12, 1, 5);
    s.standings[0].crashed = 1;
    assert_eq!(standing_mark(&s, 12, true), RiderMark::Finish);
    LEADER_FIN_LOCAL_BASE.store(-1, Ordering::Relaxed);
}

#[test]
fn flag_widget_draws_nothing_when_no_flag() {
    let _g = session_lock();
    reset_session();
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            reset_flag_display();
        }
    }
    let _pg = Guard;
    set_flag_preview(0);
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Flag].show = true;
    let s = golden_snap(&live_snap(), &cfg);
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut px, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
    let cx = (cfg[WidgetId::Flag].rect.x + cfg[WidgetId::Flag].rect.w * 0.5) * 1280.0;
    let cy = (cfg[WidgetId::Flag].rect.y + cfg[WidgetId::Flag].rect.h * 0.5) * 720.0;
    assert_eq!(sample_px(&px, cx, cy)[3], 0, "no flag should leave the slot empty");
}

#[test]
fn flag_widget_paints_checkered() {
    let _g = session_lock();
    reset_session();
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            reset_flag_display();
        }
    }
    let _pg = Guard;
    set_flag_preview(2);
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Flag].show = true;
    let s = golden_snap(&live_snap(), &cfg);
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut px, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
    let x0 = cfg[WidgetId::Flag].rect.x * 1280.0;
    let y0 = cfg[WidgetId::Flag].rect.y * 720.0;
    let x1 = x0 + cfg[WidgetId::Flag].rect.w * 1280.0;
    let y1 = y0 + cfg[WidgetId::Flag].rect.h * 720.0;
    let mut lit = 0u32;
    let mut x = x0;
    while x < x1 {
        let mut y = y0;
        while y < y1 {
            if sample_px(&px, x, y)[3] > 40 {
                lit += 1;
            }
            y += 8.0;
        }
        x += 8.0;
    }
    assert!(lit > 20, "checkered flag should fill the widget, lit={lit}");
}

#[test]
fn flag_widget_paints_white() {
    let _g = session_lock();
    reset_session();
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            reset_flag_display();
        }
    }
    let _pg = Guard;
    set_flag_preview(1);
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Flag].show = true;
    let s = golden_snap(&live_snap(), &cfg);
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut px, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
    let x0 = cfg[WidgetId::Flag].rect.x * 1280.0;
    let y0 = cfg[WidgetId::Flag].rect.y * 720.0;
    let x1 = x0 + cfg[WidgetId::Flag].rect.w * 1280.0;
    let y1 = y0 + cfg[WidgetId::Flag].rect.h * 720.0;
    assert!(
        rect_has(&px, x0, y0, x1, y1, |p| p[3] > 40 && p[0] > 180 && p[1] > 180 && p[2] > 180),
        "white flag should paint a light cloth"
    );
}

#[test]
fn flag_default_matches_saved_size() {
    let cfg = HudConfig::new();
    assert!((cfg[WidgetId::Flag].rect.w - 0.107).abs() < 0.001);
    assert!((cfg[WidgetId::Flag].rect.h - 0.019).abs() < 0.001);
}

#[test]
fn caution_off_ignores_nearby_crash() {
    let _g = session_lock();
    reset_session();
    let mut s = mid_race_snap();
    crash_ahead(&mut s);
    assert_eq!(dash_race_flag(&s), DashFlag::None);
    assert_eq!(wanted_flag(&s, false, false, false), DashFlag::None);
    assert_eq!(caution_flag(&s, true, true, false), DashFlag::Yellow);
}

#[test]
fn caution_yellow_on_nearby_crash() {
    let _g = session_lock();
    reset_session();
    let mut s = mid_race_snap();
    crash_ahead(&mut s);
    assert_eq!(wanted_flag(&s, true, false, false), DashFlag::Yellow);
    assert_eq!(wanted_flag(&s, false, true, false), DashFlag::None);
    // Clear sticky so behind / on-top / far cases are pure geometry.
    YELLOW_HOLD_AT.store(-1, Ordering::Relaxed);
    crash_behind(&mut s);
    assert_eq!(caution_flag(&s, true, true, false), DashFlag::None, "crash behind you is not a yellow");
    crash_ahead(&mut s);
    s.riders[0].track_pos = 0.399;
    assert_eq!(caution_flag(&s, true, true, false), DashFlag::None, "crash on top of you is not a yellow");
    YELLOW_HOLD_AT.store(-1, Ordering::Relaxed);
    crash_ahead(&mut s);
    s.riders[0].track_pos = 0.55;
    assert_eq!(caution_flag(&s, true, true, false), DashFlag::None, "crash far ahead is not a yellow");
}

#[test]
fn caution_yellow_sticky_across_crash_clear() {
    let _g = session_lock();
    reset_session();
    let mut s = mid_race_snap();
    crash_ahead(&mut s);
    // Same rider is also someone you are coming to lap — red would win without sticky yellow.
    s.standings[1].num_laps = 3;
    s.standings[0].num_laps = 2;
    assert_eq!(lap_rel(&s, 1), LapRel::LappedByMe);
    assert_eq!(caution_flag(&s, true, true, true), DashFlag::Yellow);
    s.riders[0].crashed = 0;
    assert_eq!(
        caution_flag(&s, true, true, true),
        DashFlag::Yellow,
        "sticky yellow holds across a remount so red cannot flash"
    );
    YELLOW_HOLD_AT.store(now_ms() - YELLOW_HOLD_MS - 1, Ordering::Relaxed);
    assert_eq!(
        caution_flag(&s, true, true, true),
        DashFlag::Red,
        "after the hold expires, red can take over"
    );
}

#[test]
fn caution_blue_and_red_skip_crashed() {
    let _g = session_lock();
    reset_session();
    let mut s = mid_race_snap();
    lapping_from_behind(&mut s);
    s.riders[0].crashed = 1;
    assert_eq!(
        caution_flag(&s, false, true, true),
        DashFlag::None,
        "crashed lapper does not wave blue"
    );
    YELLOW_HOLD_AT.store(-1, Ordering::Relaxed);
    reset_session();
    s = mid_race_snap();
    lapping_ahead(&mut s);
    s.riders[0].crashed = 1;
    assert_eq!(
        caution_flag(&s, false, true, true),
        DashFlag::None,
        "crashed backmarker does not wave red"
    );
    // Without the crash bit, red still waves.
    s.riders[0].crashed = 0;
    assert_eq!(caution_flag(&s, false, false, true), DashFlag::Red);
}

#[test]
fn caution_blue_when_being_lapped() {
    let _g = session_lock();
    reset_session();
    let mut s = mid_race_snap();
    lapping_from_behind(&mut s);
    assert_eq!(lap_rel(&s, 1), LapRel::LappingMe);
    assert_eq!(wanted_flag(&s, false, true, false), DashFlag::Blue);
    assert_eq!(wanted_flag(&s, false, false, false), DashFlag::None);
    assert_eq!(wanted_flag(&s, true, false, false), DashFlag::None);
    s.riders[0].track_pos = 0.28;
    assert_eq!(lap_rel(&s, 1), LapRel::LappingMe);
    assert_eq!(wanted_flag(&s, false, true, false), DashFlag::None, "lapper still too far for blue");
}

#[test]
fn caution_red_when_lapping_ahead() {
    let _g = session_lock();
    reset_session();
    let mut s = mid_race_snap();
    lapping_ahead(&mut s);
    assert_eq!(lap_rel(&s, 1), LapRel::LappedByMe);
    assert_eq!(wanted_flag(&s, false, false, true), DashFlag::Red);
    assert_eq!(wanted_flag(&s, false, false, false), DashFlag::None);
    assert_eq!(wanted_flag(&s, true, true, false), DashFlag::None);
    s.riders[0].track_pos = 0.50;
    assert_eq!(lap_rel(&s, 1), LapRel::LappedByMe);
    assert_eq!(wanted_flag(&s, false, false, true), DashFlag::None, "lapper still too far for red");
}

#[test]
fn caution_blue_and_red_off_in_warmup() {
    let _g = session_lock();
    reset_session();
    let mut s = mid_race_snap();
    s.session_kind = 5;
    s.session_laps = 2;
    s.session_length = 15;
    lapping_from_behind(&mut s);
    assert!(is_warmup(&s));
    assert_eq!(lap_rel(&s, 1), LapRel::Same);
    assert_eq!(wanted_flag(&s, false, true, true), DashFlag::None);
    lapping_ahead(&mut s);
    assert_eq!(lap_rel(&s, 1), LapRel::Same);
    assert_eq!(wanted_flag(&s, false, true, true), DashFlag::None);
}

#[test]
fn caution_yellow_on_in_warmup_and_practice() {
    let _g = session_lock();
    reset_session();
    let mut s = mid_race_snap();
    s.session_kind = 5;
    s.session_laps = 2;
    s.session_length = 15;
    crash_ahead(&mut s);
    assert!(is_warmup(&s));
    assert_eq!(
        caution_flag(&s, true, true, true),
        DashFlag::Yellow,
        "yellow still waves in warmup when someone crashes ahead"
    );
    assert_eq!(
        caution_flag(&s, false, true, true),
        DashFlag::None,
        "blue/red stay off in warmup"
    );

    reset_session();
    s = mid_race_snap();
    s.session_kind = -1;
    s.session_laps = 0;
    s.session_length = 0;
    crash_ahead(&mut s);
    assert!(is_practice_session(&s));
    assert!(!is_warmup(&s));
    assert_eq!(
        caution_flag(&s, true, false, false),
        DashFlag::Yellow,
        "yellow waves in Testing Setup practice"
    );

    reset_session();
    s = mid_race_snap();
    s.session_kind = -1;
    s.session_laps = 2;
    s.session_length = 40;
    crash_ahead(&mut s);
    assert!(is_practice_session(&s), "40 min stays practice when extras leak");
    assert!(!is_warmup(&s));
    assert_eq!(
        caution_flag(&s, true, false, false),
        DashFlag::Yellow,
        "yellow waves in open practice"
    );
}

#[test]
fn caution_yellow_beats_blue() {
    let _g = session_lock();
    reset_session();
    let mut s = mid_race_snap();
    lapping_from_behind(&mut s);
    s.rider_count = 3;
    s.riders[2] = rider(5, 12.0, 5.0, 0.43);
    s.riders[2].crashed = 1;
    assert_eq!(caution_flag(&s, true, true, false), DashFlag::Yellow);
    assert_eq!(caution_flag(&s, false, true, false), DashFlag::Blue);
}

#[test]
fn caution_blue_beats_red() {
    let _g = session_lock();
    reset_session();
    let mut s = mid_race_snap();
    lapping_ahead(&mut s);
    s.rider_count = 3;
    s.standing_count = 3;
    s.standings[2] = standing(5, 3, 4);
    s.riders[2] = rider(5, 8.0, 3.0, 0.365);
    assert_eq!(caution_flag(&s, false, true, true), DashFlag::Blue);
    assert_eq!(caution_flag(&s, false, false, true), DashFlag::Red);
}

#[test]
fn caution_loses_to_white_and_checkered() {
    assert_eq!(merge_caution(DashFlag::White, DashFlag::Yellow), DashFlag::White);
    assert_eq!(merge_caution(DashFlag::Checkered, DashFlag::Blue), DashFlag::Checkered);
    assert_eq!(merge_caution(DashFlag::White, DashFlag::Red), DashFlag::White);
    assert_eq!(merge_caution(DashFlag::None, DashFlag::Yellow), DashFlag::Yellow);
    assert_eq!(merge_caution(DashFlag::None, DashFlag::Red), DashFlag::Red);
}

#[test]
fn dash_wrap_skips_caution_flags() {
    assert_eq!(dash_wrap_flag(DashFlag::Yellow, 1.0, false, false, false), (DashFlag::None, 0.0));
    assert_eq!(dash_wrap_flag(DashFlag::Blue, 1.0, false, false, false), (DashFlag::None, 0.0));
    assert_eq!(dash_wrap_flag(DashFlag::Red, 1.0, false, false, false), (DashFlag::None, 0.0));
    assert_eq!(dash_wrap_flag(DashFlag::White, 0.8, false, false, false), (DashFlag::White, 0.8));
    assert_eq!(dash_wrap_flag(DashFlag::Yellow, 1.0, true, false, false), (DashFlag::Yellow, 1.0));
    assert_eq!(dash_wrap_flag(DashFlag::Blue, 0.7, false, true, false), (DashFlag::Blue, 0.7));
    assert_eq!(dash_wrap_flag(DashFlag::Red, 0.7, false, false, true), (DashFlag::Red, 0.7));
    assert_eq!(dash_wrap_flag(DashFlag::Yellow, 1.0, false, true, false), (DashFlag::None, 0.0));
    assert_eq!(dash_wrap_flag(DashFlag::Red, 1.0, true, true, false), (DashFlag::None, 0.0));
}

#[test]
fn flag_widget_paints_yellow() {
    let _g = session_lock();
    reset_session();
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            reset_flag_display();
        }
    }
    let _pg = Guard;
    set_flag_preview(3);
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Flag].show = true;
    cfg.flag.flag_yellow = true;
    let s = golden_snap(&live_snap(), &cfg);
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut px, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
    let x0 = cfg[WidgetId::Flag].rect.x * 1280.0;
    let y0 = cfg[WidgetId::Flag].rect.y * 720.0;
    let x1 = x0 + cfg[WidgetId::Flag].rect.w * 1280.0;
    let y1 = y0 + cfg[WidgetId::Flag].rect.h * 720.0;
    assert!(rect_has(&px, x0, y0, x1, y1, is_yellow), "yellow flag should paint the cloth");
}

#[test]
fn flag_caption_white_covers_the_label() {
    let _g = session_lock();
    reset_session();
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            reset_flag_display();
        }
    }
    let _pg = Guard;
    set_flag_preview(3);
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Flag].show = true;
    cfg.flag.flag_yellow = true;
    let s = golden_snap(&live_snap(), &cfg);
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut px, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
    let w = cfg[WidgetId::Flag].rect.w * 1280.0;
    let h = cfg[WidgetId::Flag].rect.h * 720.0;
    let x = cfg[WidgetId::Flag].rect.x * 1280.0;
    let y = cfg[WidgetId::Flag].rect.y * 720.0;
    let gw = flag_caption_group_w(&fonts(), h, 1.0, "YELLOW FLAG");
    let mid_y = y + h * 0.5;
    let cx = x + w * 0.5;
    let mut sx = cx - gw * 0.5 - 10.0;
    let x1 = cx + gw * 0.5 + 10.0;
    while sx < x1 {
        let p = sample_px(&px, sx, mid_y);
        let yellow_through = p[3] > 40 && p[0] > 160 && p[2] < 120 && (p[0] as i16 - p[2] as i16) > 80;
        assert!(
            !yellow_through,
            "white plaque should cover the label and side pad at {sx:.0}, got {p:?}"
        );
        sx += 1.0;
    }
}

#[test]
fn flag_caption_white_leaves_cloth_above_and_below() {
    let _g = session_lock();
    reset_session();
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            reset_flag_display();
        }
    }
    let _pg = Guard;
    set_flag_preview(3);
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Flag].show = true;
    cfg.flag.flag_yellow = true;
    let s = golden_snap(&live_snap(), &cfg);
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut px, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
    let x0 = cfg[WidgetId::Flag].rect.x * 1280.0;
    let y0 = cfg[WidgetId::Flag].rect.y * 720.0;
    let w = cfg[WidgetId::Flag].rect.w * 1280.0;
    let h = cfg[WidgetId::Flag].rect.h * 720.0;
    let cx = x0 + w * 0.5;
    let y1 = y0 + h;
    let cloth = is_yellow;
    assert!(
        (1..4).any(|dy| cloth(sample_px(&px, cx, y0 + dy as f32))),
        "cloth should show above the caption white"
    );
    assert!(
        (1..4).any(|dy| cloth(sample_px(&px, x0 + 12.0, y1 - dy as f32))),
        "cloth should show below the caption white"
    );
}

#[test]
fn flag_text_off_leaves_cloth_with_no_caption() {
    let _g = session_lock();
    reset_session();
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            reset_flag_display();
        }
    }
    let _pg = Guard;
    set_flag_preview(3);
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Flag].show = true;
    cfg.flag.flag_yellow = true;
    cfg.flag.flag_text = false;
    let s = golden_snap(&live_snap(), &cfg);
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut px, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
    let w = cfg[WidgetId::Flag].rect.w * 1280.0;
    let h = cfg[WidgetId::Flag].rect.h * 720.0;
    let x = cfg[WidgetId::Flag].rect.x * 1280.0;
    let y = cfg[WidgetId::Flag].rect.y * 720.0;
    let cx = x + w * 0.5;
    let mid_y = y + h * 0.5;
    assert!(
        is_yellow(sample_px(&px, cx, mid_y)),
        "cloth should show through the center when text is off, got {:?}",
        sample_px(&px, cx, mid_y)
    );
}

#[test]
fn flag_widget_paints_blue() {
    let _g = session_lock();
    reset_session();
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            reset_flag_display();
        }
    }
    let _pg = Guard;
    set_flag_preview(4);
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Flag].show = true;
    cfg.flag.flag_blue = true;
    let s = golden_snap(&live_snap(), &cfg);
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut px, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
    let x0 = cfg[WidgetId::Flag].rect.x * 1280.0;
    let y0 = cfg[WidgetId::Flag].rect.y * 720.0;
    let x1 = x0 + cfg[WidgetId::Flag].rect.w * 1280.0;
    let y1 = y0 + cfg[WidgetId::Flag].rect.h * 720.0;
    assert!(rect_has(&px, x0, y0, x1, y1, is_blue), "blue flag should paint the cloth");
}

#[test]
fn flag_widget_paints_red() {
    let _g = session_lock();
    reset_session();
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            reset_flag_display();
        }
    }
    let _pg = Guard;
    set_flag_preview(5);
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Flag].show = true;
    cfg.flag.flag_red = true;
    let s = golden_snap(&live_snap(), &cfg);
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut px, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
    let x0 = cfg[WidgetId::Flag].rect.x * 1280.0;
    let y0 = cfg[WidgetId::Flag].rect.y * 720.0;
    let x1 = x0 + cfg[WidgetId::Flag].rect.w * 1280.0;
    let y1 = y0 + cfg[WidgetId::Flag].rect.h * 720.0;
    assert!(rect_has(&px, x0, y0, x1, y1, is_red), "red flag should paint the cloth");
}

#[test]
fn dash_wrap_does_not_paint_yellow() {
    let _g = session_lock();
    reset_session();
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            reset_flag_display();
        }
    }
    let _pg = Guard;
    set_flag_preview(3);
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Dash].show = true;
    let s = golden_snap(&live_snap(), &cfg);
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut px, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
    let x0 = cfg[WidgetId::Dash].rect.x * 1280.0 - 8.0;
    let y0 = (cfg[WidgetId::Dash].rect.y * 720.0 - 36.0).max(0.0);
    let x1 = (cfg[WidgetId::Dash].rect.x + cfg[WidgetId::Dash].rect.w) * 1280.0 + 8.0;
    let y1 = cfg[WidgetId::Dash].rect.y * 720.0;
    assert!(
        !rect_has(&px, x0, y0, x1, y1, is_yellow),
        "dash wrap must not paint yellow"
    );
}

#[test]
fn dash_wrap_paints_yellow_when_enabled() {
    let _g = session_lock();
    reset_session();
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            reset_flag_display();
        }
    }
    let _pg = Guard;
    set_flag_preview(3);
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Dash].show = true;
    cfg.dash.dash_yellow = true;
    let s = golden_snap(&live_snap(), &cfg);
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut px, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
    let x0 = cfg[WidgetId::Dash].rect.x * 1280.0 - 8.0;
    let y0 = (cfg[WidgetId::Dash].rect.y * 720.0 - 36.0).max(0.0);
    let x1 = (cfg[WidgetId::Dash].rect.x + cfg[WidgetId::Dash].rect.w) * 1280.0 + 8.0;
    let y1 = cfg[WidgetId::Dash].rect.y * 720.0;
    assert!(
        rect_has(&px, x0, y0, x1, y1, is_yellow),
        "dash wrap should paint yellow when the toggle is on"
    );
}

#[test]
fn dash_wrap_does_not_paint_red() {
    let _g = session_lock();
    reset_session();
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            reset_flag_display();
        }
    }
    let _pg = Guard;
    set_flag_preview(5);
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Dash].show = true;
    let s = golden_snap(&live_snap(), &cfg);
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut px, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
    let x0 = cfg[WidgetId::Dash].rect.x * 1280.0 - 8.0;
    let y0 = (cfg[WidgetId::Dash].rect.y * 720.0 - 36.0).max(0.0);
    let x1 = (cfg[WidgetId::Dash].rect.x + cfg[WidgetId::Dash].rect.w) * 1280.0 + 8.0;
    let y1 = cfg[WidgetId::Dash].rect.y * 720.0;
    assert!(
        !rect_has(&px, x0, y0, x1, y1, is_red),
        "dash wrap must not paint red"
    );
}

#[test]
fn dash_wrap_paints_red_when_enabled() {
    let _g = session_lock();
    reset_session();
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            reset_flag_display();
        }
    }
    let _pg = Guard;
    set_flag_preview(5);
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Dash].show = true;
    cfg.dash.dash_red = true;
    let s = golden_snap(&live_snap(), &cfg);
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut px, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
    let x0 = cfg[WidgetId::Dash].rect.x * 1280.0 - 8.0;
    let y0 = (cfg[WidgetId::Dash].rect.y * 720.0 - 36.0).max(0.0);
    let x1 = (cfg[WidgetId::Dash].rect.x + cfg[WidgetId::Dash].rect.w) * 1280.0 + 8.0;
    let y1 = cfg[WidgetId::Dash].rect.y * 720.0;
    assert!(
        rect_has(&px, x0, y0, x1, y1, is_red),
        "dash wrap should paint red when the toggle is on"
    );
}

fn paint_again(px: &mut Pixmap, s: &Snapshot, cfg: &HudConfig) {
    draw_each(
        px,
        &fonts(),
        Some(s),
        cfg,
        1280,
        720,
        0.0,
        1280,
        720,
        None,
        |_, _| true,
    );
}

/// A second paint (the stream thread) must not step the wave, the yellow hold, or the run-in.
#[test]
fn stream_redraw_keeps_the_overlay_flag() {
    let _g = session_lock();
    reset_session();
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            reset_flag_display();
        }
    }
    let _pg = Guard;
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Flag].show = true;
    cfg.flag.flag_yellow = true;
    cfg.flag.flag_blue = true;
    cfg.flag.flag_red = true;
    let mut s = mid_race_snap();
    crash_ahead(&mut s);
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut px, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
    let shown = cached_display_flag();
    assert_eq!(shown.0, DashFlag::Yellow);
    let hold = YELLOW_HOLD_AT.load(Ordering::Relaxed);
    let wave = WHITE_WAVE_AT.load(Ordering::Relaxed);
    let run_in = RUN_IN_FLAG.load(Ordering::Relaxed);
    assert!(hold >= 0, "yellow hold should be armed");
    s.riders[0].crashed = 0;
    paint_again(&mut px, &s, &cfg);
    assert_eq!(YELLOW_HOLD_AT.load(Ordering::Relaxed), hold);
    assert_eq!(WHITE_WAVE_AT.load(Ordering::Relaxed), wave);
    assert_eq!(RUN_IN_FLAG.load(Ordering::Relaxed), run_in);
    assert_eq!(cached_display_flag(), shown);
}

/// No overlay paint: one advance still waves white, checkered, and yellow, and a
/// following paint does not step them again.
#[test]
fn stream_only_advances_each_flag_once() {
    let _g = session_lock();
    reset_session();
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            reset_flag_display();
        }
    }
    let _pg = Guard;
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Flag].show = true;
    let mut s = live_snap();
    s.session_length = 0;
    s.session_laps = 4;
    s.session_time_ms = 0;
    s.local_speed = 18.0;
    s.poly_count = 0;
    s.sf_meters = 0.0;
    s.standings[0].num_laps = 2;
    s.standings[1].num_laps = 2;
    s.current_lap = 3;
    advance_display_flag(&s, &cfg);
    s.standings[1].num_laps = 3;
    s.current_lap = 4;
    advance_display_flag(&s, &cfg);
    assert_eq!(cached_display_flag().0, DashFlag::White);
    let wave = WHITE_WAVE_AT.load(Ordering::Relaxed);
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    paint_again(&mut px, &s, &cfg);
    assert_eq!(WHITE_WAVE_AT.load(Ordering::Relaxed), wave);
    assert_eq!(cached_display_flag().0, DashFlag::White);

    s.standings[1].num_laps = 4;
    s.current_lap = 5;
    advance_display_flag(&s, &cfg);
    assert_eq!(cached_display_flag().0, DashFlag::Checkered);
    assert_eq!(CHECKERED_LATCH.load(Ordering::Relaxed), 1);
    paint_again(&mut px, &s, &cfg);
    assert_eq!(CHECKERED_LATCH.load(Ordering::Relaxed), 1);
    assert_eq!(cached_display_flag().0, DashFlag::Checkered);

    reset_session();
    reset_flag_display();
    cfg.flag.flag_yellow = true;
    cfg.flag.flag_blue = true;
    cfg.flag.flag_red = true;
    let mut s = mid_race_snap();
    crash_ahead(&mut s);
    advance_display_flag(&s, &cfg);
    assert_eq!(cached_display_flag().0, DashFlag::Yellow);
    let hold = YELLOW_HOLD_AT.load(Ordering::Relaxed);
    assert!(hold >= 0);
    s.riders[0].crashed = 0;
    paint_again(&mut px, &s, &cfg);
    assert_eq!(YELLOW_HOLD_AT.load(Ordering::Relaxed), hold);
    assert_eq!(cached_display_flag().0, DashFlag::Yellow);
}
