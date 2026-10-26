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
fn formatters_cover_clock_gap_and_penalty() {
    assert_eq!(format_countdown(0), "00:00");
    assert_eq!(format_countdown(125_000), "02:05");
    assert_eq!(format_countdown(3600_000), "01:00:00");
    assert_eq!(format_session_clock(0), "--:--:--");
    assert_eq!(format_session_clock(3661_000), "01:01:01");
    assert_eq!(session_len_ms(8), 8 * 60_000);
    assert_eq!(session_len_ms(480), 480_000);
    assert_eq!(format_clock(0), "--:--.---");
    assert_eq!(format_clock(93_500), "01:33.500");
    assert_eq!(format_board_gap(0, 0, true), "-");
    assert_eq!(format_board_gap(0, 1, false), "1L");
    assert_eq!(format_board_gap(1500, 0, false), "1.5");
    assert_eq!(format_penalty(0), "---");
    assert_eq!(format_penalty(5000), "5s");
    assert_eq!(format_delta_ms(0), "0.000");
    assert_eq!(format_delta_ms(250), "+0.250");
    assert_eq!(format_delta_ms(-347), "-0.347");
    assert_eq!(format_local_clock(0, 5), "12:05 AM");
    assert_eq!(format_local_clock(9, 14), "9:14 AM");
    assert_eq!(format_local_clock(12, 0), "12:00 PM");
    assert_eq!(format_local_clock(21, 7), "9:07 PM");
    assert_eq!(format_memory(0.0), "0 MB");
    assert_eq!(format_memory(88.0), "88 MB");
    assert_eq!(format_memory(1800.0), "1.8 GB");
}

#[test]
fn standings_and_relative_board_fields() {
    let _g = session_lock();
    reset_session();
    let s = live_snap();
    let cfg = HudConfig::new();
    assert_eq!(board_item(&s, &cfg, BoardField::None), None);
    assert_eq!(board_item(&s, &cfg, BoardField::Position).unwrap().1, "P2");
    assert_eq!(board_item(&s, &cfg, BoardField::ClassPos).unwrap().1, "P2");
    assert_eq!(board_item(&s, &cfg, BoardField::Track).unwrap().1, "Test Track");
    assert_eq!(board_item(&s, &cfg, BoardField::Riders).unwrap().1, "2");
    assert_eq!(board_item(&s, &cfg, BoardField::SessionType).unwrap().1, "Session");
    assert_eq!(board_item(&s, &cfg, BoardField::Fuel).unwrap().1, "5.6 L");
    assert_eq!(board_item(&s, &cfg, BoardField::FuelPct).unwrap().1, "80%");
    assert_eq!(board_item(&s, &cfg, BoardField::Setup).unwrap().1, "Washougal Soft");
    let mut empty_setup = s;
    empty_setup.setup_name = [0; TRACK_NAME];
    assert_eq!(board_item(&empty_setup, &cfg, BoardField::Setup).unwrap().1, "--");
    let mut empty_fuel = s;
    empty_fuel.fuel = 0.0;
    empty_fuel.max_fuel = 0.0;
    assert_eq!(board_item(&empty_fuel, &cfg, BoardField::Fuel).unwrap().1, "-- L");
    assert_eq!(board_item(&empty_fuel, &cfg, BoardField::FuelPct).unwrap().1, "--%");
    let mut timed = s;
    timed.session_length = 8;
    timed.session_laps = 0;
    assert_eq!(board_item(&timed, &cfg, BoardField::SessionType).unwrap().1, "Timed");
    timed.session_laps = 12;
    timed.session_length = 0;
    assert_eq!(board_item(&timed, &cfg, BoardField::Lap).unwrap().1, "6 / 12");
    // Laps left counts the lap you are on: 6 / 12 means seven still to run.
    assert_eq!(board_item(&timed, &cfg, BoardField::LapsLeft).unwrap().1, "7");
    assert_eq!(board_item(&timed, &cfg, BoardField::Lap).unwrap().1, session_banner(&timed).1);
    assert_eq!(dash_foot_item(&timed, &cfg, DashField::LapCount).unwrap().1, session_banner(&timed).1);
    assert_eq!(board_item(&timed, &cfg, BoardField::SessionType).unwrap().1, "Lap race");
    for field in BoardField::ALL {
        if field == BoardField::None {
            continue;
        }
        assert!(board_item(&s, &cfg, field).is_some(), "{field:?}");
    }
    assert_eq!(board_item(&s, &cfg, BoardField::GapAhead).unwrap().1, "10.000");
    assert_eq!(board_item(&s, &cfg, BoardField::GapBehind).unwrap().1, "---");
    assert_eq!(ticker_meta_label(BoardField::LapDiff, "+0.400"), "DIFF");
    assert_eq!(ticker_meta_label(BoardField::GapAhead, "10.000"), "AHEAD");
    assert_eq!(ticker_meta_label(BoardField::GapBehind, "---"), "BEHIND");
    assert!(!cfg.standings_cols().is_empty());
    assert!(cfg.standings_cols().contains(&StField::Name));
    assert!(cfg.relative_cols().contains(&RelField::Name));
}

#[test]
fn board_gap_ahead_behind_use_live_place_neighbors() {
    let _g = session_lock();
    reset_session();
    let cfg = HudConfig::new();
    let s = live_snap();
    RaceStore::refresh(&s);
    assert_eq!(board_item(&s, &cfg, BoardField::GapAhead).unwrap(), ('\u{f062}', "10.000".into()));
    assert_eq!(board_item(&s, &cfg, BoardField::GapBehind).unwrap().0, '\u{f063}');
    assert_eq!(board_item(&s, &cfg, BoardField::GapBehind).unwrap().1, "---");
    assert_eq!(
        dash_foot_item(&s, &cfg, DashField::Interval).unwrap(),
        ('\u{f062}', "10.000".into(), None)
    );
    assert_eq!(dash_foot_item(&s, &cfg, DashField::GapBehind).unwrap().0, '\u{f063}');
    assert_eq!(dash_foot_item(&s, &cfg, DashField::Gap).unwrap().1, "10.000");
    assert_eq!(dash_foot_item(&s, &cfg, DashField::GapBehind).unwrap().1, "---");

    let mut three = s;
    three.standing_count = 3;
    three.rider_count = 3;
    three.standings[2] = standing(7, 3, 5);
    three.standings[2].gap_ms = 5_100;
    three.riders[2] = rider(7, 5.0, 2.0, 0.80);
    RaceStore::refresh(&three);
    assert_eq!(board_item(&three, &cfg, BoardField::GapAhead).unwrap().1, "10.000");
    assert_eq!(board_item(&three, &cfg, BoardField::GapBehind).unwrap().1, "6.667");
    assert_eq!(dash_foot_item(&three, &cfg, DashField::GapBehind).unwrap().1, "6.667");
}

#[test]
fn board_gap_ahead_follows_a_pass() {
    let _g = session_lock();
    reset_session();
    let cfg = HudConfig::new();
    let mut s = live_snap();
    s.local_track_pos = 0.50;
    s.riders[1].track_pos = 0.50;
    s.riders[0].track_pos = 0.70;
    s.standing_count = 3;
    s.rider_count = 3;
    s.standings[2] = standing(7, 3, 5);
    s.riders[2] = rider(7, 5.0, 2.0, 0.50);
    RaceStore::refresh(&s);
    assert_eq!(board_item(&s, &cfg, BoardField::GapAhead).unwrap().1, "11.111");

    s.riders[2].track_pos = 0.506;
    s.riders[2].x += 0.006 * s.track_length;
    RaceStore::refresh(&s);
    assert_eq!(live_position(7), 2);
    assert_eq!(live_position(12), 3);
    assert_eq!(
        board_item(&s, &cfg, BoardField::GapAhead).unwrap().1,
        "0.333",
        "a pass must switch gap ahead to the new P−1"
    );
}

#[test]
fn board_gap_ahead_behind_tick_with_track_pos() {
    let _g = session_lock();
    reset_session();
    let cfg = HudConfig::new();
    let mut s = live_snap();
    RaceStore::refresh(&s);
    assert_eq!(board_item(&s, &cfg, BoardField::GapAhead).unwrap().1, "10.000");

    s.riders[0].track_pos = 0.95;
    RaceStore::refresh(&s);
    assert_eq!(board_item(&s, &cfg, BoardField::GapAhead).unwrap().1, "1.667");

    s.standings[0].gap_laps = 0;
    s.standings[1].gap_laps = 1;
    RaceStore::refresh(&s);
    assert_eq!(
        board_item(&s, &cfg, BoardField::GapAhead).unwrap().1,
        "1.667",
        "a stale +1L must not hide the live running gap"
    );

    // Half a lap of shortest wrap (same as Relative).
    s.riders[1].track_pos = 0.25;
    s.local_track_pos = 0.25;
    s.riders[0].track_pos = 0.75;
    s.standings[1].gap_laps = 0;
    RaceStore::refresh(&s);
    assert_eq!(board_item(&s, &cfg, BoardField::GapAhead).unwrap().1, "27.778");
}

/// A crashed last-place rider sitting on the track is still P+1. Shortest wrap
/// would read them as nearby every pass; once you have a lap on them it is `-1L`.
#[test]
fn board_gap_behind_lapped_crashed_rider_is_laps_not_wrap() {
    let _g = session_lock();
    reset_session();
    let cfg = HudConfig::new();
    let mut s = live_snap();
    s.standing_count = 3;
    s.rider_count = 3;
    s.standings[2] = standing(7, 3, 5);
    s.standings[2].crashed = 1;
    s.riders[2] = rider(7, 5.0, 2.0, 0.50);
    s.riders[2].crashed = 1;
    s.local_track_pos = 0.51;
    s.riders[1].track_pos = 0.51;
    RaceStore::refresh(&s);
    assert_eq!(
        board_item(&s, &cfg, BoardField::GapBehind).unwrap().1,
        "0.556",
        "same lap: time to the crashed rider, not a lap"
    );

    s.standings[1].num_laps = 6;
    s.current_lap = 7;
    RaceStore::refresh(&s);
    assert_eq!(
        board_item(&s, &cfg, BoardField::GapBehind).unwrap().1,
        "-1L",
        "a lap up must not tick the on-track wrap through a lapping pass"
    );
    assert_eq!(dash_foot_item(&s, &cfg, DashField::GapBehind).unwrap().1, "-1L");

    s.local_track_pos = 0.40;
    s.riders[1].track_pos = 0.40;
    RaceStore::refresh(&s);
    assert_eq!(
        board_item(&s, &cfg, BoardField::GapBehind).unwrap().1,
        "-1L",
        "approaching a lapped rider is still a lap, not a shrinking wrap"
    );

    s.standings[1].num_laps = 7;
    s.current_lap = 8;
    RaceStore::refresh(&s);
    assert_eq!(board_item(&s, &cfg, BoardField::GapBehind).unwrap().1, "-2L");
}

/// Gap behind is how far they must ride to catch you, not the short way round.
#[test]
fn board_gap_behind_does_not_flip_at_half_a_lap() {
    let _g = session_lock();
    reset_session();
    let cfg = HudConfig::new();
    let mut s = live_snap();
    s.standing_count = 3;
    s.rider_count = 3;
    s.standings[2] = standing(7, 3, 5);
    s.riders[2] = rider(7, 5.0, 2.0, 0.20);
    s.local_track_pos = 0.90;
    s.riders[1].track_pos = 0.90;
    RaceStore::refresh(&s);
    assert_eq!(
        board_item(&s, &cfg, BoardField::GapBehind).unwrap().1,
        "38.889",
        "0.70 lap forward, not the 0.30 shortest wrap"
    );
}

#[test]
fn table_column_widths_follow_settings_when_they_fit() {
    let pad = 8.0;
    let avail = 400.0;
    let width = |c: StField| match c {
        StField::Pos => 26.0,
        StField::Name => 80.0,
        StField::Gap => 58.0,
        StField::Last => 54.0,
        _ => 40.0,
    };
    let flex = |c: StField| matches!(c, StField::Name);
    let few = col_slots(0.0, pad, avail, &[StField::Pos, StField::Name], width, flex);
    let many = col_slots(
        0.0,
        pad,
        avail,
        &[StField::Pos, StField::Name, StField::Gap, StField::Last],
        width,
        flex,
    );
    let name_few = few.iter().find(|(c, _, _)| *c == StField::Name).unwrap().2;
    let name_many = many.iter().find(|(c, _, _)| *c == StField::Name).unwrap().2;
    assert!((name_few - 80.0).abs() < 0.01);
    assert!((name_many - 80.0).abs() < 0.01);
    assert!((few[0].2 - 26.0).abs() < 0.01);
    assert!((many.iter().find(|(c, _, _)| *c == StField::Gap).unwrap().2 - 58.0).abs() < 0.01);

    let wide_name = |c: StField| match c {
        StField::Name => 140.0,
        _ => width(c),
    };
    let grown = col_slots(0.0, pad, avail, &[StField::Pos, StField::Name, StField::Gap], wide_name, flex);
    let name_grown = grown.iter().find(|(c, _, _)| *c == StField::Name).unwrap().2;
    assert!((name_grown - 140.0).abs() < 0.01);

    let tight = col_slots(0.0, pad, 120.0, &[StField::Pos, StField::Name, StField::Gap], width, flex);
    let name_tight = tight.iter().find(|(c, _, _)| *c == StField::Name).unwrap().2;
    assert!(name_tight < 80.0);
    assert!(name_tight >= 18.0);
    assert!((col_right(&tight) - (120.0 - pad)).abs() < 0.51);
}

#[test]
fn hug_board_w_drops_empty_glass() {
    let pad = 8.0;
    let origin = 10.0;
    let max_w = 600.0;
    let slots = col_slots(
        origin,
        pad,
        max_w,
        &[StField::Pos, StField::Name, StField::Last],
        |c| match c {
            StField::Pos => 26.0,
            StField::Name => 80.0,
            StField::Last => 54.0,
            _ => 40.0,
        },
        |c| matches!(c, StField::Name),
    );
    let hugged = hug_board_w(origin, pad, max_w, &slots);
    assert!(hugged < 220.0, "plaque should hug columns, got {hugged}");
    assert!((hugged - (col_right(&slots) - origin + pad)).abs() < 0.01);
    let tight = hug_board_w(origin, pad, 120.0, &slots);
    assert!(tight <= 120.0);
}

#[test]
fn overtime_ignores_standings_catch_up_after_expiry() {
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
    let _ = session_remain_ms(&s);
    s.session_time_ms = 8 * 60 * 1000;
    s.current_lap = 6;
    s.local_speed = 18.0;
    s.standings[0].num_laps = 5;
    s.standings[1].num_laps = 0;
    let _ = session_remain_ms(&s);
    s.session_time_ms = 8 * 60 * 1000 - 1_000;
    let _ = session_remain_ms(&s);
    s.session_time_ms = 400;
    assert_eq!(session_remain_ms(&s), Some(0));
    assert_eq!(session_banner(&s).1, "0/2");
    s.standings[1].num_laps = 5;
    assert_eq!(session_banner(&s).1, "0/2");
    s.standings[0].num_laps = 6;
    assert_eq!(session_banner(&s).1, "0/2");
    s.standings[1].num_laps = 6;
    s.current_lap = 7;
    assert_eq!(session_banner(&s).1, "1/2", "first extra cross starts the extra");
    s.standings[1].num_laps = 7;
    s.current_lap = 8;
    assert_eq!(session_banner(&s).1, "2/2");
}

#[test]
fn timed_plus_one_empty_standings_at_expiry_does_not_start_extras() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 8;
    s.session_laps = 1;
    s.session_time_ms = 30_000;
    s.current_lap = 1;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    let _ = session_remain_ms(&s);
    s.session_time_ms = 8 * 60 * 1000;
    s.current_lap = 4;
    s.local_speed = 18.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    let _ = session_remain_ms(&s);
    s.session_time_ms = 8 * 60 * 1000 - 1_000;
    let _ = session_remain_ms(&s);
    s.session_time_ms = 400;
    assert_eq!(session_remain_ms(&s), Some(0));
    assert_eq!(session_banner(&s).1, "0/1");
    s.standings[0].num_laps = 1;
    assert_eq!(session_banner(&s).1, "0/1", "leader standings recovery is not extras start");
    s.standings[1].num_laps = 1;
    s.current_lap = 2;
    assert_eq!(session_banner(&s).1, "0/1", "timed-lap finish after recovery is not an extra");
    assert_eq!(dash_race_flag(&s), DashFlag::None, "no flag until extras truly start");
    s.standings[0].num_laps = 2;
    assert_eq!(
        dash_race_flag(&s),
        DashFlag::None,
        "the lap you are on when the leader starts extras is not the extra"
    );
    assert_eq!(session_banner(&s).1, "0/1");
    s.standings[1].num_laps = 2;
    s.current_lap = 3;
    assert_eq!(dash_race_flag(&s), DashFlag::White);
    assert_eq!(session_banner(&s).1, "1/1", "last-lap start shows 1/1");
    s.standings[1].num_laps = 3;
    s.current_lap = 4;
    assert_eq!(dash_race_flag(&s), DashFlag::Checkered);
    let cfg = HudConfig::new();
    assert_eq!(board_item(&s, &cfg, BoardField::Lap).unwrap().1, "1/1");
    assert_eq!(board_item(&s, &cfg, BoardField::Session).unwrap().1, "1/1");
    assert_eq!(dash_foot_item(&s, &cfg, DashField::LapCount).unwrap().1, "1/1");
    assert_eq!(ticker_meta_label(BoardField::Lap, "1/1"), "LAPS");
    assert_eq!(ticker_meta_label(BoardField::Fuel, "5.6 L"), "FUEL");
    assert_eq!(ticker_meta_label(BoardField::Setup, "Washougal Soft"), "SETUP");
}

/// Timberline 8:00+1: extras published ~150s after expiry and standings reset to 0.
/// Overtime bases must stay on the timed-lap counts, not rebuild from lap 1.
#[test]
fn eight_minute_plus_one_late_extras_and_standings_reset() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 8;
    s.session_laps = 0;
    s.session_time_ms = 30_000;
    s.current_lap = 1;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    let _ = session_remain_ms(&s);
    s.session_time_ms = 8 * 60 * 1000;
    s.current_lap = 4;
    s.local_speed = 18.0;
    s.standings[0].num_laps = 4;
    s.standings[1].num_laps = 3;
    let remain = session_remain_ms(&s).expect("countdown while time remains");
    assert!(remain > 60_000, "expected a real countdown, got {remain}");
    s.session_time_ms = 8 * 60 * 1000 - 1_000;
    let _ = session_remain_ms(&s);
    s.session_time_ms = 400;
    assert_eq!(session_remain_ms(&s), Some(0));
    s.session_time_ms = 8 * 60 * 1000;
    let _ = session_remain_ms(&s);
    assert_eq!(session_banner(&s).1, "00:00", "extras not published yet");
    assert_eq!(dash_race_flag(&s), DashFlag::None);

    s.current_lap = 5;
    s.standings[1].num_laps = 4;
    s.standings[0].num_laps = 3;
    let _ = session_remain_ms(&s);
    assert_eq!(dash_race_flag(&s), DashFlag::None, "uncounted lap, extras still unpublished");

    s.session_laps = 1;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    s.session_time_ms = 40_000;
    assert_eq!(session_banner(&s).1, "0/1");
    assert_eq!(dash_race_flag(&s), DashFlag::None, "empty standings after reset");

    s.standings[0].num_laps = 1;
    s.standings[1].num_laps = 1;
    s.current_lap = 2;
    assert_eq!(dash_race_flag(&s), DashFlag::None);
    s.standings[0].num_laps = 2;
    s.standings[1].num_laps = 2;
    s.current_lap = 3;
    assert_eq!(
        dash_race_flag(&s),
        DashFlag::None,
        "rebuilt lap 2 is not the extra"
    );
    s.standings[0].num_laps = 3;
    s.standings[1].num_laps = 3;
    s.current_lap = 4;
    assert_ne!(
        dash_race_flag(&s),
        DashFlag::Checkered,
        "must not latch checkered three laps early"
    );

    s.standings[0].num_laps = 5;
    s.standings[1].num_laps = 4;
    s.current_lap = 5;
    assert_eq!(ride_to(&mut s, 0.40), DashFlag::None);
    s.standings[1].num_laps = 5;
    s.current_lap = 6;
    assert_eq!(dash_race_flag(&s), DashFlag::White, "last extra after the timed-lap base");
    age_white_wave();
    assert_eq!(ride_to(&mut s, 0.50), DashFlag::None);
    assert_eq!(cross_line(&mut s, 6, 7), DashFlag::Checkered);
}

#[test]
fn fifteen_minute_plus_one_late_extras_and_standings_reset() {
    let _g = session_lock();
    long_timed_plus_one_late_extras(15);
}

#[test]
fn twenty_five_minute_plus_one_late_extras_and_standings_reset() {
    let _g = session_lock();
    long_timed_plus_one_late_extras(25);
}

#[test]
fn thirty_minute_plus_one_late_extras_and_standings_reset() {
    let _g = session_lock();
    long_timed_plus_one_late_extras(30);
}

#[test]
fn forty_minute_plus_one_late_extras_and_standings_reset() {
    let _g = session_lock();
    long_timed_plus_one_late_extras(40);
}

#[test]
fn sixty_minute_plus_one_late_extras_and_standings_reset() {
    let _g = session_lock();
    long_timed_plus_one_late_extras(60);
}

/// Over a tabletop while the leader goes under: centerline projection can snap them
/// half a lap "behind" then correct ahead. That must not sticky-latch ~Lapped.
#[test]
fn tabletop_track_pos_spike_does_not_latch_lapped() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.local_speed = 18.0;
    s.standings[0].num_laps = 5;
    s.standings[1].num_laps = 5;
    s.current_lap = 6;
    ride_to(&mut s, 0.50);
    // Same lap battle: not armed, spike must not latch.
    s.riders[0].track_pos = 0.10;
    assert!(!lapped(&s), "same lap never arms the pass latch");
    s.riders[0].track_pos = 0.54;
    assert!(!lapped(&s), "projection flip on the same lap is not a lap-down");

    // Lap-up but the under-path teleports beyond PAIR_MAX then corrects ahead.
    s.standings[0].num_laps = 6;
    s.riders[0].track_pos = 0.52;
    assert!(!lapped(&s), "armed but not yet behind");
    s.riders[0].track_pos = 0.15;
    assert!(!lapped(&s), "far projection spike clears behind-state");
    s.riders[0].track_pos = 0.54;
    assert!(
        !lapped(&s),
        "teleport from far behind to ahead is not a continuous pass"
    );

    // Real continuous pass still latches.
    s.riders[0].track_pos = 0.35;
    assert!(!lapped(&s));
    s.riders[0].track_pos = 0.42;
    assert!(!lapped(&s));
    s.riders[0].track_pos = 0.54;
    assert!(lapped(&s), "a real lap-up pass still latches");
}

/// Equal completed laps during extras is the same physical lap — do not arm ~Lapped
/// on a mid-pack battle even if track_pos wraps.
#[test]
fn equal_extras_laps_tabletop_spike_does_not_latch_lapped() {
    let _g = session_lock();
    let mut s = live_snap();
    expire_timed_extras(&mut s, 1);
    s.local_speed = 18.0;
    s.standings[0].num_laps = 6;
    s.standings[1].num_laps = 6;
    s.current_lap = 7;
    assert!(extras_started(&s));
    ride_to(&mut s, 0.50);
    s.riders[0].track_pos = 0.15;
    assert!(!lapped(&s), "equal extras laps do not arm the latch");
    s.riders[0].track_pos = 0.54;
    assert!(!lapped(&s), "tabletop flip with equal extras laps is not ~Lapped");
}

#[test]
fn lap_race_starts_on_lap_one_when_current_lap_is_zero() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 0;
    s.session_laps = 4;
    s.session_time_ms = 0;
    s.current_lap = 0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    assert_eq!(session_banner(&s).1, "1 / 4");
    assert_eq!(race_lap(&s), 1);
    let cfg = HudConfig::new();
    assert_eq!(board_item(&s, &cfg, BoardField::Lap).unwrap().1, "1 / 4");
    assert_eq!(dash_foot_item(&s, &cfg, DashField::LapCount).unwrap().1, "1 / 4");
    s.standings[1].num_laps = 1;
    s.current_lap = 2;
    assert_eq!(session_banner(&s).1, "2 / 4");
    assert_eq!(race_lap(&s), 2);
}

#[test]
fn mid_race_eight_second_board_glitch_keeps_time() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 8;
    s.session_laps = 1;
    s.session_time_ms = 30_000;
    s.current_lap = 1;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    let _ = session_remain_ms(&s);
    s.session_time_ms = 8 * 60 * 1000;
    s.local_speed = 18.0;
    s.current_lap = 3;
    let _ = session_remain_ms(&s);
    s.session_time_ms = 8 * 60 * 1000 - 1_000;
    let _ = session_remain_ms(&s);
    s.session_time_ms = 7 * 60 * 1000 + 1_135;
    let remain = session_remain_ms(&s).expect("07:01");
    assert!(
        (remain - 421_135).abs() < 1_000,
        "expected ~07:01, got {remain}"
    );
    s.session_time_ms = 8_000;
    let remain = session_remain_ms(&s).expect("ignore 8s board junk");
    assert!(
        remain > 7 * 60 * 1000,
        "00:08 glitch must not replace 07:01, got {remain}"
    );
    assert!(!overtime_active(&s));
}

#[test]
fn frozen_board_between_prestart_and_gate_shows_race_time() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 8;
    s.session_laps = 1;
    s.session_time_ms = 120_000;
    s.current_lap = 1;
    s.local_speed = 12.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    assert_eq!(session_banner(&s).1, "02:00");
    s.session_time_ms = 8 * 60 * 1000;
    assert_eq!(session_banner(&s).1, "08:00");
    s.session_time_ms = 30_000;
    s.current_lap = 2;
    s.local_speed = 14.0;
    assert_eq!(session_banner(&s).1, "08:00");
    std::thread::sleep(std::time::Duration::from_millis(800));
    assert_eq!(session_banner(&s).1, "08:00");
    assert!(!overtime_active(&s));
    assert_eq!(dash_race_flag(&s), DashFlag::None);
    s.session_time_ms = 49_970;
    s.local_speed = 0.0;
    s.current_lap = 1;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    assert_eq!(session_banner(&s).1, "08:00");
    s.session_time_ms = 48_980;
    assert_eq!(session_banner(&s).1, "08:00");
    s.session_time_ms = 140_000;
    assert_eq!(session_banner(&s).1, "08:00");
    s.session_time_ms = 8 * 60 * 1000 - 1_000;
    s.local_speed = 12.0;
    assert_eq!(session_banner(&s).1, "07:59");
}

#[test]
fn fifty_second_board_does_not_expire_when_race_clock_appears() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 8;
    s.session_laps = 1;
    s.session_time_ms = 49_940;
    s.current_lap = 1;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    assert_eq!(session_banner(&s).1, "00:49");
    s.session_time_ms = 7_940;
    let remain = session_remain_ms(&s).expect("still on the board");
    assert!(
        remain < 15_000,
        "00:08 board must not become 07:52 elapsed, got {remain}"
    );
    assert_eq!(session_banner(&s).1, "00:07");
    assert!(!overtime_active(&s));
    s.session_time_ms = 8 * 60 * 1000 - 1_813;
    s.local_speed = 4.0;
    let remain = session_remain_ms(&s).expect("race clock after board");
    assert!(remain > 7 * 60 * 1000, "must not go to 0/1 at green, got {remain}");
    assert!(!overtime_active(&s));
    assert_eq!(session_banner(&s).1, "07:58");
}

#[test]
fn two_lap_moto_with_leftover_start_board_shows_laps() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 50_000;
    s.session_laps = 2;
    s.session_time_ms = 36_290;
    s.current_lap = 6;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    assert_eq!(session_banner(&s).1, "00:36");
    s.session_time_ms = 1_000;
    assert_eq!(session_banner(&s).1, "00:01");
    s.session_time_ms = 50_000;
    s.local_speed = 16.0;
    assert_eq!(session_banner(&s).1, "0 / 2");
    s.standings[1].num_laps = 1;
    s.current_lap = 2;
    assert_eq!(session_banner(&s).1, "1 / 2");
    s.standings[1].num_laps = 2;
    s.current_lap = 3;
    assert_eq!(session_banner(&s).1, "2 / 2");
    assert!(!overtime_active(&s));
}

#[test]
fn six_minute_plus_two_leftover_start_board_shows_countdown() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    // Plugin left a ~50s board in session_length while the live clock is 6:00 +2.
    s.session_length = 50_000;
    s.session_laps = 2;
    s.session_time_ms = 6 * 60 * 1000;
    s.current_lap = 1;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    assert_eq!(session_banner(&s).1, "06:00");
    assert!(!is_lap_race(&s));
    s.session_time_ms = 6 * 60 * 1000 - 2_000;
    s.local_speed = 18.0;
    s.current_lap = 2;
    s.standings[0].num_laps = 1;
    s.standings[1].num_laps = 1;
    let text = session_banner(&s).1;
    assert!(
        text.contains(':') && !text.contains('+') && !text.contains('/'),
        "leftover board length must not force 1 / 2, got {text}"
    );
}

#[test]
fn later_start_board_shows_race_length() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 480_000;
    s.session_laps = 1;
    s.session_time_ms = 43_980;
    s.current_lap = 0;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    assert_eq!(session_banner(&s).1, "00:43");
    s.session_time_ms = 1_980;
    assert_eq!(session_banner(&s).1, "00:01");
    s.session_time_ms = 45_000;
    assert_eq!(session_banner(&s).1, "08:00");
    s.session_time_ms = 30_000;
    assert_eq!(session_banner(&s).1, "08:00");
    s.session_time_ms = 479_328;
    s.local_speed = 3.8;
    let _ = session_remain_ms(&s);
    s.session_time_ms = 478_998;
    s.local_speed = 5.8;
    assert_eq!(session_banner(&s).1, "07:58");
    assert!(!overtime_active(&s));
}

#[test]
fn horizontal_standings_scrolls_from_the_leader() {
    assert_eq!(hstand_scroll_start(0, 7, 12), 0.0);
    assert_eq!(hstand_scroll_start(6, 7, 12), 0.0);
    assert_eq!(hstand_scroll_start(7, 7, 12), 1.0);
    assert_eq!(hstand_scroll_start(11, 7, 12), 5.0);
    assert_eq!(hstand_scroll_start(2, 7, 5), 0.0);
    assert_eq!(hstand_scroll_hold(4, 4, 12, 2.0), 2.0);
    assert_eq!(hstand_scroll_hold(2, 4, 12, 2.0), 2.0);
    assert_eq!(hstand_scroll_hold(5, 4, 12, 2.0), 2.0);
    assert_eq!(hstand_scroll_hold(6, 4, 12, 2.0), hstand_scroll_start(6, 4, 12));
    assert_eq!(hstand_scroll_hold(1, 4, 12, 2.0), hstand_scroll_start(1, 4, 12));
    assert!((hstand_card_x(0.0, 0.0, 0.0, 40.0) - 0.0).abs() < 0.01);
    assert!((hstand_card_x(2.0, 0.0, 0.0, 40.0) - 80.0).abs() < 0.01);
    let looped = hstand_loop_x(0.0, 0.3, 12.0, 100.0, 700.0, 97.0).unwrap();
    assert!((looped + 30.0).abs() < 0.01);
    let wrapped = hstand_loop_x(0.0, 11.7, 12.0, 100.0, 700.0, 97.0).unwrap();
    assert!((wrapped - 30.0).abs() < 0.01);
    assert!(hstand_loop_x(11.0, 0.0, 12.0, 100.0, 700.0, 97.0).is_none());
    let (vis_wide, w_wide) = hstand_layout(1400.0, 1.0, 7, 20);
    assert_eq!(vis_wide, 12);
    assert!((w_wide - 1367.0 / 12.0).abs() < 0.01);
    let (vis_mid, w_mid) = hstand_layout(7.0 * 118.0 + 6.0 * 3.0, 1.0, 7, 20);
    assert_eq!(vis_mid, 7);
    assert_eq!(w_mid, 118.0);
    let (vis_narrow, w_narrow) = hstand_layout(280.0, 1.0, 7, 20);
    assert_eq!(vis_narrow, 3);
    assert!((w_narrow - 274.0 / 3.0).abs() < 0.01);
}

#[test]
fn table_slides_eases_a_swap_into_the_new_slot() {
    let mut slides = TableSlides { rows: Vec::new() };
    assert_eq!(slides.indices(&[1, 2], 0.0), vec![0.0, 1.0]);
    let start = slides.indices(&[2, 1], 1.0);
    assert!((start[0] - 1.0).abs() < 0.01);
    assert!((start[1] - 0.0).abs() < 0.01);
    let mid = slides.indices(&[2, 1], 1.15);
    assert!(mid[0] < 1.0 && mid[0] > 0.0);
    assert!(mid[1] > 0.0 && mid[1] < 1.0);
    let done = slides.indices(&[2, 1], 1.30);
    assert!((done[0] - 0.0).abs() < 0.01);
    assert!((done[1] - 1.0).abs() < 0.01);
}

/// The game only republishes its classification when someone crosses the line, so a pass
/// has to move the places on its own.
#[test]
fn a_pass_moves_positions_before_the_line() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    pass_the_leader(&mut s, 6.0);
    let field = RaceStore::tick(&s).field;
    assert_eq!(live_leader(), 12);
    assert_eq!(live_position(12), 1);
    assert_eq!(live_position(1), 2);
    assert_eq!(field.rows[0].standing.race_num, 12);
    assert_eq!(field.rows[0].standing.position, 1);
    assert!(field.rows[0].is_leader);
    assert_eq!(field.focus, Some(0));
    // Boards iterate this order, so the row order moves with the pass.
    assert_eq!(field.board()[0].race_num, 12);
}

/// Out and back, 4 m apart. Length is 204 m on a 200 m lap, close enough to trust.
fn folded_lane_snap() -> Snapshot {
    let mut snapshot = live_snap();
    snapshot.track_length = 200.0;
    snapshot.sf_meters = 0.0;
    let points = [(0.0, 0.0), (100.0, 0.0), (100.0, 4.0), (0.0, 4.0)];
    snapshot.poly_count = points.len() as i32;
    for (index, (x, z)) in points.iter().enumerate() {
        snapshot.poly[index] = Point { x: *x, z: *z };
    }
    snapshot
}

/// The return lane sits beside the outbound lane. A lap fraction on that lane is not a pass.
#[test]
fn a_rider_in_the_next_lane_is_not_a_pass() {
    let _g = session_lock();
    reset_session();
    let mut snapshot = folded_lane_snap();
    snapshot.riders[0].track_pos = 0.25;
    snapshot.riders[1].track_pos = 0.75;
    snapshot.local_track_pos = 0.75;
    let field = RaceStore::tick(&snapshot).field;
    assert_eq!(live_position(1), 1, "the next lane is not ahead");
    assert_eq!(live_position(12), 2);
    assert_eq!(field.rows[0].standing.race_num, 1);
}

/// Same folded line, both on the outbound lane, lead matched in the world. That still passes.
#[test]
fn a_same_lane_lead_on_a_folded_track_still_passes() {
    let _g = session_lock();
    reset_session();
    let mut snapshot = folded_lane_snap();
    snapshot.riders[0].track_pos = 0.10;
    snapshot.riders[1].track_pos = 0.16;
    snapshot.local_track_pos = 0.16;
    let field = RaceStore::tick(&snapshot).field;
    assert_eq!(live_position(12), 1);
    assert_eq!(live_position(1), 2);
    assert_eq!(field.rows[0].standing.race_num, 12);
}

/// Farm14: a bike 9 m away with a lap fraction ~700 m up the track is the next lane.
#[test]
fn a_bike_nine_metres_away_is_not_a_long_lead() {
    let _g = session_lock();
    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = 1669.0;
    snapshot.sf_meters = 1669.0;
    let false_lead = 0.211 + 681.0 / 1669.0;
    snapshot.riders[0].track_pos = 0.211;
    snapshot.riders[0].x = 0.0;
    snapshot.riders[0].z = 0.0;
    snapshot.riders[1].track_pos = false_lead;
    snapshot.riders[1].x = 9.0;
    snapshot.riders[1].z = 0.0;
    snapshot.local_track_pos = false_lead;
    snapshot.local_x = 9.0;
    snapshot.local_z = 0.0;
    let held = RaceStore::tick(&snapshot).field;
    assert_eq!(live_position(1), 1, "9 m away is not a 681 m pass");
    assert_eq!(live_position(12), 2);
    assert_eq!(held.rows[0].standing.race_num, 1);

    let lead = 12.0 / 1669.0;
    snapshot.riders[0].track_pos = 0.400;
    snapshot.riders[0].x = 0.400 * 1669.0;
    snapshot.riders[1].track_pos = 0.400 + lead;
    snapshot.riders[1].x = snapshot.riders[1].track_pos * 1669.0;
    snapshot.riders[1].z = 0.0;
    snapshot.local_track_pos = snapshot.riders[1].track_pos;
    snapshot.local_x = snapshot.riders[1].x;
    let _ = RaceStore::tick(&snapshot);
    let passed = RaceStore::tick(&snapshot).field;
    assert_eq!(live_position(12), 1, "a 12 m lead on the same line still passes");
    assert_eq!(passed.rows[0].standing.race_num, 12);
}

/// Winchester: a ~700 m centerline fold with ~15 m of body movement is not a pass.
/// Riding on until the fraction and the body agree on a 30 m lead takes the place.
/// The other bike does not keep a lap from the coordinate the fold froze.
#[test]
fn a_refused_fold_does_not_keep_the_place_after_the_ride() {
    let _g = session_lock();
    reset_session();
    let length = 1898.0;
    let mut snapshot = live_snap();
    snapshot.track_length = length;
    snapshot.sf_meters = 0.0;
    snapshot.poly_count = 0;
    snapshot.session_laps = 2;
    snapshot.current_lap = 1;
    snapshot.local_speed = 18.0;
    snapshot.standing_count = 2;
    snapshot.rider_count = 2;
    snapshot.standings[0] = standing(1, 1, 0);
    snapshot.standings[1] = standing(12, 2, 0);
    let origin = 0.10;
    snapshot.riders[0] = rider(1, origin * length, 0.0, origin);
    snapshot.riders[1] = rider(12, origin * length, 0.0, origin);
    snapshot.local_track_pos = origin;
    snapshot.local_x = origin * length;
    snapshot.local_z = 0.0;
    IN_GATE.store(1, Ordering::Relaxed);
    let _ = RaceStore::tick(&snapshot);
    IN_GATE.store(0, Ordering::Relaxed);
    snapshot.holeshot_race_num = 1;
    ride_field_to(&mut snapshot, &[(1, 0.20), (12, 0.18)]);
    assert_eq!(live_position(1), 1, "ahead before the fold");
    assert_eq!(live_position(12), 2);

    let folded = snapshot.riders[1].track_pos + 700.0 / length;
    snapshot.riders[1].track_pos = folded;
    snapshot.riders[1].x += 15.0;
    snapshot.local_track_pos = folded;
    snapshot.local_x = snapshot.riders[1].x;
    let _ = RaceStore::tick(&snapshot);
    assert_ne!(
        live_position(12),
        1,
        "15 m of body movement is not a 700 m pass"
    );

    let back = (snapshot.riders[1].x / length).rem_euclid(1.0);
    snapshot.riders[1].track_pos = back;
    snapshot.local_track_pos = back;
    let leader = snapshot.riders[0].track_pos;
    ride_field_to(
        &mut snapshot,
        &[(1, leader + 0.02), (12, leader + 0.02 + 30.0 / length)],
    );
    assert_eq!(
        live_position(12),
        1,
        "30 m ahead on the fraction and in the world"
    );
    assert_eq!(
        live_position(1),
        2,
        "the frozen coordinate does not keep a lap"
    );
}

/// Millcreek: a bike 40 m away and ~200 m up the lap is still the next lane.
#[test]
fn a_bike_forty_metres_away_is_not_a_long_lead() {
    let _g = session_lock();
    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = 2092.0;
    snapshot.sf_meters = 2092.0;
    snapshot.riders[0].track_pos = 0.30;
    snapshot.riders[0].x = 0.0;
    snapshot.riders[0].z = 0.0;
    let false_lead = 0.30 + 200.0 / 2092.0;
    snapshot.riders[1].track_pos = false_lead;
    snapshot.riders[1].x = 40.0;
    snapshot.riders[1].z = 0.0;
    snapshot.local_track_pos = false_lead;
    snapshot.local_x = 40.0;
    snapshot.local_z = 0.0;
    let held = RaceStore::tick(&snapshot).field;
    assert_eq!(live_position(1), 1, "40 m away is not a 200 m pass");
    assert_eq!(live_position(12), 2);
    assert_eq!(held.rows[0].standing.race_num, 1);
}

/// A bike 250 m away and 600 m up the lap is ahead. The chord is short because the
/// lap bends, not because they are in the next lane. Same metres on a long lap and
/// a shorter one.
fn a_distant_same_lap_lead_stays_ahead(track_length: f32) {
    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = track_length;
    snapshot.sf_meters = track_length * 0.5;
    snapshot.riders[1].track_pos = 0.48;
    snapshot.riders[1].x = 0.0;
    snapshot.riders[1].z = 0.0;
    snapshot.local_track_pos = 0.48;
    snapshot.local_x = 0.0;
    snapshot.local_z = 0.0;
    snapshot.riders[0].track_pos = 0.48 + 600.0 / track_length;
    snapshot.riders[0].x = 250.0;
    snapshot.riders[0].z = 0.0;
    let held = RaceStore::tick(&snapshot).field;
    assert_eq!(
        live_position(1),
        1,
        "{track_length} m: 250 m away and 600 m up the lap stays ahead"
    );
    assert_eq!(live_position(12), 2);
    assert_eq!(held.rows[0].standing.race_num, 1);
}

#[test]
fn a_bike_two_hundred_fifty_metres_away_keeps_a_real_lead() {
    let _g = session_lock();
    a_distant_same_lap_lead_stays_ahead(1400.0);
    a_distant_same_lap_lead_stays_ahead(2480.0);
}

/// Maryland holeshot: the game already has the jumped fraction in P1. Bodies 16 m
/// apart and ~360 m apart on the lap are still the pack, so that lead is given back.
#[test]
fn a_folded_spline_lead_does_not_keep_the_game_place() {
    let _g = session_lock();
    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = 1795.0;
    snapshot.sf_meters = 1795.0;
    snapshot.riders[0].track_pos = 0.06;
    snapshot.riders[0].x = 287.0;
    snapshot.riders[0].z = 323.0;
    snapshot.riders[1].track_pos = 0.86;
    snapshot.riders[1].x = 287.0;
    snapshot.riders[1].z = 307.0;
    snapshot.local_track_pos = 0.86;
    snapshot.local_x = 287.0;
    snapshot.local_z = 307.0;
    let field = RaceStore::tick(&snapshot).field;
    assert_eq!(live_position(12), 1, "0.86 stays ahead of a 0.06 fold 16 m away");
    assert_eq!(live_position(1), 2);
    assert_eq!(field.rows[0].standing.race_num, 12);
}

/// Farm14: 44.5 m away and ~160 m up the lap is a real lead. The 45 m line was
/// flashing the bike behind into that place.
#[test]
fn a_forty_five_metre_chord_does_not_steal_a_real_lead() {
    let _g = session_lock();
    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = 1669.0;
    snapshot.sf_meters = 1669.0;
    let ahead = 0.556 + 160.0 / 1669.0;
    snapshot.riders[0].track_pos = ahead;
    snapshot.riders[0].x = 44.5;
    snapshot.riders[0].z = 0.0;
    snapshot.riders[1].track_pos = 0.556;
    snapshot.riders[1].x = 0.0;
    snapshot.riders[1].z = 0.0;
    snapshot.local_track_pos = 0.556;
    snapshot.local_x = 0.0;
    snapshot.local_z = 0.0;
    let held = RaceStore::tick(&snapshot).field;
    assert_eq!(live_position(1), 1, "44.5 m away and 160 m up the lap stays ahead");
    assert_eq!(live_position(12), 2);
    assert_eq!(held.rows[0].standing.race_num, 1);
}

/// Forest: 82 m up the lap and 20 m across the corner is still that lead. The
/// game has the trailer in front. Same metres on a short lap and a long one.
/// A second sample inside 20 m does not hand the place back.
fn a_corner_keeps_an_eighty_metre_lead(track_length: f32) {
    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = track_length;
    snapshot.sf_meters = track_length;
    snapshot.poly_count = 0;
    let trailer = 0.40;
    let leader = trailer + 82.0 / track_length;
    snapshot.standings[0].position = 2;
    snapshot.standings[1].position = 1;
    snapshot.riders[0].track_pos = leader;
    snapshot.riders[0].x = 20.0;
    snapshot.riders[0].z = 0.0;
    snapshot.riders[1].track_pos = trailer;
    snapshot.riders[1].x = 0.0;
    snapshot.riders[1].z = 0.0;
    snapshot.local_track_pos = trailer;
    snapshot.local_x = 0.0;
    snapshot.local_z = 0.0;
    let held = RaceStore::tick(&snapshot).field;
    assert_eq!(
        live_position(1),
        1,
        "{track_length} m: 20 m away and 82 m up the lap stays ahead"
    );
    assert_eq!(live_position(12), 2);
    assert_eq!(held.rows[0].standing.race_num, 1);

    snapshot.riders[0].x = 19.0;
    let still = RaceStore::tick(&snapshot).field;
    assert_eq!(
        live_position(1),
        1,
        "{track_length} m: 19 m across the same corner does not take the place"
    );
    assert_eq!(live_position(12), 2);
    assert_eq!(still.rows[0].standing.race_num, 1);
}

#[test]
fn a_corner_keeps_an_eighty_metre_lead_on_a_short_lap_and_a_long_one() {
    let _g = session_lock();
    a_corner_keeps_an_eighty_metre_lead(1256.0);
    a_corner_keeps_an_eighty_metre_lead(2480.0);
}

/// MXB Club: 128 m up the lap and 19.5 m across the corner is still that lead.
/// The 20 m strip was handing the place to the bike behind on the centerline,
/// then giving it back a sample later. A second sample inside 20 m stays put.
/// 159 m stays the other branch — that hold is the Washougal test.
fn a_corner_past_120m_keeps_the_centerline(track_length: f32) {
    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = track_length;
    snapshot.sf_meters = track_length;
    snapshot.poly_count = 0;
    let trailer = 0.40;
    let leader = trailer + 128.0 / track_length;
    snapshot.standings[0].position = 2;
    snapshot.standings[1].position = 1;
    snapshot.riders[0].track_pos = leader;
    snapshot.riders[0].x = 19.5;
    snapshot.riders[0].z = 0.0;
    snapshot.riders[1].track_pos = trailer;
    snapshot.riders[1].x = 0.0;
    snapshot.riders[1].z = 0.0;
    snapshot.local_track_pos = trailer;
    snapshot.local_x = 0.0;
    snapshot.local_z = 0.0;
    let held = RaceStore::tick(&snapshot).field;
    assert_eq!(
        live_position(1),
        1,
        "{track_length} m: 19.5 m away and 128 m up the lap stays ahead"
    );
    assert_eq!(live_position(12), 2);
    assert_eq!(held.rows[0].standing.race_num, 1);

    snapshot.riders[0].x = 19.0;
    let still = RaceStore::tick(&snapshot).field;
    assert_eq!(
        live_position(1),
        1,
        "{track_length} m: 19 m across the same corner does not take the place"
    );
    assert_eq!(live_position(12), 2);
    assert_eq!(still.rows[0].standing.race_num, 1);
}

#[test]
fn a_corner_past_120m_keeps_the_centerline_on_a_short_lap_and_a_long_one() {
    let _g = session_lock();
    a_corner_past_120m_keeps_the_centerline(1083.0);
    a_corner_past_120m_keeps_the_centerline(2480.0);
}

/// MXB Club finish: the bike at 0.736 is 27–43 m from a pack that is actually
/// at the line (0.93–0.996). That pack's own gaps match the spline. The
/// returning lane must not be scored onto them.
#[test]
fn a_bike_behind_the_finish_pack_stays_behind() {
    let _g = session_lock();
    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = 1083.0;
    snapshot.sf_meters = 1083.0;
    snapshot.poly_count = 0;
    let laps = 1;
    seat(&mut snapshot, 0, 12, 1, laps, 113.5, 162.9, 0.736);
    seat(&mut snapshot, 1, 3, 2, laps, 145.6, 192.1, 0.933);
    seat(&mut snapshot, 2, 2, 3, laps, 116.6, 207.6, 0.962);
    seat(&mut snapshot, 3, 1, 4, laps, 90.2, 175.8, 0.996);
    snapshot.standing_count = 4;
    snapshot.rider_count = 4;
    snapshot.local_race_num = 12;
    snapshot.focus_race_num = 12;
    snapshot.local_track_pos = 0.736;
    snapshot.local_x = 113.5;
    snapshot.local_z = 162.9;
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(live_position(1), 1, "the bike at the line stays P1");
    assert!(
        live_position(3) < live_position(12),
        "0.933 stays ahead of the returning lane, got P{} and P{}",
        live_position(3),
        live_position(12)
    );
}

/// Bump `num_laps` with each lap so the tracker does not pin a multi-lap ride,
/// then leave both riders on the last lap at `into`.
fn ride_with_the_lap_count(snapshot: &mut Snapshot, laps: i32, into: f32) {
    for lap in 1..=laps {
        for standing in snapshot
            .standings
            .iter_mut()
            .take(snapshot.standing_count as usize)
        {
            standing.num_laps = lap;
        }
        let target = lap as f32 + into;
        ride_field_to(snapshot, &[(1, target), (12, target)]);
    }
}

/// MXB Club, t=509: five laps listed ahead of six, 25 m apart and the world gap
/// matches the fraction. The lower lap's coordinate has already ridden the lap
/// it is short, so the score sits a lap ahead of the classification.
fn a_matching_gap_keeps_the_higher_lap(track_length: f32) {
    reset_session();
    let mut snapshot = gate_field(&[1, 12]);
    snapshot.track_length = track_length;
    snapshot.sf_meters = track_length;
    snapshot.poly_count = 0;
    let into = 0.267;
    ride_with_the_lap_count(&mut snapshot, 6, into);
    ride_field_to(&mut snapshot, &[(12, 6.0 + 0.95)]);
    snapshot
        .standings
        .iter_mut()
        .find(|standing| standing.race_num == 12)
        .unwrap()
        .num_laps = 7;
    ride_field_to(&mut snapshot, &[(12, 7.0 + 0.85)]);
    let ahead = into + 25.0 / track_length;
    if let Some(rider) = snapshot.riders.iter_mut().find(|rider| rider.race_num == 12) {
        rider.track_pos = ahead;
        rider.x = ahead * track_length;
        rider.z = 0.0;
    }
    snapshot.local_track_pos = ahead;
    snapshot.local_x = ahead * track_length;
    snapshot.local_z = 0.0;
    for standing in snapshot.standings.iter_mut() {
        if standing.race_num == 12 {
            standing.num_laps = 5;
            standing.position = 1;
        } else if standing.race_num == 1 {
            standing.position = 2;
        }
    }
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(
        live_position(1),
        1,
        "{track_length} m: six laps passes five when the bodies match the fraction"
    );
    assert_eq!(live_position(12), 2);
}

#[test]
fn a_matching_gap_keeps_the_higher_lap_on_a_short_lap_and_a_long_one() {
    let _g = session_lock();
    a_matching_gap_keeps_the_higher_lap(1083.0);
    a_matching_gap_keeps_the_higher_lap(800.0);
}

/// MXB Club, after the checkered: seven laps listed ahead of nine, 22 m apart,
/// ~460 m apart on the spline. The lower lap's coordinate has already covered
/// the lap it is short. The fold does not keep that place.
fn a_close_fold_keeps_the_higher_lap(track_length: f32) {
    reset_session();
    let mut snapshot = gate_field(&[1, 12]);
    snapshot.track_length = track_length;
    snapshot.sf_meters = track_length;
    snapshot.poly_count = 0;
    let into = 0.071;
    ride_with_the_lap_count(&mut snapshot, 2, into);
    ride_field_to(&mut snapshot, &[(12, 2.0 + 0.95)]);
    snapshot
        .standings
        .iter_mut()
        .find(|standing| standing.race_num == 12)
        .unwrap()
        .num_laps = 3;
    ride_field_to(&mut snapshot, &[(12, 3.0 + 0.90)]);
    let more = snapshot
        .riders
        .iter()
        .find(|rider| rider.race_num == 1)
        .copied()
        .unwrap();
    let folded = into + 460.0 / track_length;
    if let Some(fewer) = snapshot.riders.iter_mut().find(|rider| rider.race_num == 12) {
        fewer.track_pos = folded;
        fewer.x = more.x + 22.0;
        fewer.z = more.z;
    }
    snapshot.local_track_pos = folded;
    snapshot.local_x = more.x + 22.0;
    snapshot.local_z = more.z;
    for standing in snapshot.standings.iter_mut() {
        if standing.race_num == 12 {
            standing.num_laps = 1;
            standing.position = 1;
        } else if standing.race_num == 1 {
            standing.position = 2;
        }
    }
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(
        live_position(1),
        1,
        "{track_length} m: the higher lap passes a 22 m fold"
    );
    assert_eq!(live_position(12), 2);
}

#[test]
fn a_close_fold_keeps_the_higher_lap_on_a_short_lap_and_a_long_one() {
    let _g = session_lock();
    a_close_fold_keeps_the_higher_lap(1083.0);
    a_close_fold_keeps_the_higher_lap(2000.0);
}

/// Paleta: same lap, the bodies match the fraction, and the bike behind on that
/// fraction still holds a coordinate a lap off. The raw leader takes the place.
/// 20 m and 57 m are #5 ahead of you. 185 m is #5 ahead of #250. The pin is not
/// a lap the body rode. 185 m is past one step, so a refused jump used to keep it.
fn a_matching_gap_orders_the_raw_leader(track_length: f32, gap_m: f32) {
    reset_session();
    let mut snapshot = gate_field(&[1, 12]);
    snapshot.track_length = track_length;
    snapshot.sf_meters = track_length;
    snapshot.poly_count = 0;
    let base = 0.40;
    ride_with_the_lap_count(&mut snapshot, 2, base);
    if let Some(rider) = snapshot.riders.iter_mut().find(|rider| rider.race_num == 12) {
        rider.track_pos = -1.0;
    }
    snapshot.local_track_pos = -1.0;
    snapshot
        .standings
        .iter_mut()
        .find(|standing| standing.race_num == 12)
        .unwrap()
        .num_laps = 4;
    let _ = RaceStore::tick(&snapshot);
    let leader_frac = base + gap_m / track_length;
    if let Some(rider) = snapshot.riders.iter_mut().find(|rider| rider.race_num == 1) {
        rider.track_pos = leader_frac.rem_euclid(1.0);
        rider.x = leader_frac * track_length;
        rider.z = 0.0;
    }
    if let Some(rider) = snapshot.riders.iter_mut().find(|rider| rider.race_num == 12) {
        rider.track_pos = base;
        rider.x = base * track_length;
        rider.z = 0.0;
    }
    snapshot.local_track_pos = base;
    snapshot.local_x = base * track_length;
    snapshot.local_z = 0.0;
    for standing in snapshot.standings.iter_mut() {
        standing.num_laps = 2;
        if standing.race_num == 12 {
            standing.position = 1;
        } else if standing.race_num == 1 {
            standing.position = 2;
        }
    }
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(
        live_position(1),
        1,
        "{track_length} m: {gap_m} m matching gap passes the higher coordinate"
    );
    assert_eq!(live_position(12), 2);
}

#[test]
fn a_matching_gap_orders_the_raw_leader_on_a_short_lap_and_a_long_one() {
    let _g = session_lock();
    for track_length in [1635.0, 2000.0] {
        a_matching_gap_orders_the_raw_leader(track_length, 20.0);
        a_matching_gap_orders_the_raw_leader(track_length, 57.0);
        a_matching_gap_orders_the_raw_leader(track_length, 185.0);
    }
}

/// Wolf Creek: the trailer rode a lap the classification does not have, and is
/// now 30 m behind on a gap the bodies match. That short gap is the order.
fn a_short_matching_gap_beats_a_ridden_lap(track_length: f32) {
    reset_session();
    let mut snapshot = gate_field(&[1, 12]);
    snapshot.track_length = track_length;
    snapshot.sf_meters = track_length;
    snapshot.poly_count = 0;
    let base = 0.40;
    ride_with_the_lap_count(&mut snapshot, 2, base);
    ride_field_to(&mut snapshot, &[(12, base + 1.0)]);
    let leader_frac = base + 30.0 / track_length;
    if let Some(rider) = snapshot.riders.iter_mut().find(|rider| rider.race_num == 1) {
        rider.track_pos = leader_frac;
        rider.x = leader_frac * track_length;
        rider.z = 0.0;
    }
    if let Some(rider) = snapshot.riders.iter_mut().find(|rider| rider.race_num == 12) {
        rider.track_pos = base;
        rider.x = base * track_length;
        rider.z = 0.0;
    }
    snapshot.local_track_pos = base;
    snapshot.local_x = base * track_length;
    snapshot.local_z = 0.0;
    for standing in snapshot.standings.iter_mut().take(2) {
        standing.num_laps = 2;
        if standing.race_num == 12 {
            standing.position = 1;
        } else {
            standing.position = 2;
        }
    }
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(
        live_position(1),
        1,
        "{track_length} m: a 30 m matching gap beats a ridden lap"
    );
    assert_eq!(live_position(12), 2);
}

#[test]
fn a_short_matching_gap_beats_a_ridden_lap_on_a_short_lap_and_a_long_one() {
    let _g = session_lock();
    a_short_matching_gap_beats_a_ridden_lap(1635.0);
    a_short_matching_gap_beats_a_ridden_lap(2463.0);
}

/// Putra: the leader's coordinate stays ignored after a short chord, then the
/// bodies open to about 120 m and match the fraction. That gap is the place.
#[test]
fn a_matching_gap_past_80m_keeps_an_ignored_coordinate() {
    let _g = session_lock();
    reset_session();
    let mut snapshot = gate_field(&[1, 12]);
    let length = 1635.0;
    snapshot.track_length = length;
    snapshot.sf_meters = length;
    snapshot.poly_count = 0;
    for standing in snapshot.standings.iter_mut().take(2) {
        standing.num_laps = 4;
    }
    let lead = 120.0;
    let leader_frac = 0.50;
    let trailer_frac = leader_frac - lead / length;
    ride_field_to(&mut snapshot, &[(1, leader_frac), (12, trailer_frac)]);
    assert_eq!(live_position(1), 1, "matching lead before the corner");

    let step = |snapshot: &mut Snapshot, spline: f32, leader_dx: f32, trailer_dx: f32| {
        let length = snapshot.track_length;
        snapshot.riders[0].track_pos =
            (snapshot.riders[0].track_pos + spline / length).rem_euclid(1.0);
        snapshot.riders[0].x += leader_dx;
        snapshot.riders[1].track_pos =
            (snapshot.riders[1].track_pos + spline / length).rem_euclid(1.0);
        snapshot.riders[1].x += trailer_dx;
        snapshot.local_track_pos = snapshot.riders[1].track_pos;
        snapshot.local_x = snapshot.riders[1].x;
        snapshot.local_z = snapshot.riders[1].z;
        let _ = RaceStore::tick(snapshot);
    };
    for _ in 0..20 {
        step(&mut snapshot, 10.0, 1.0, 6.0);
    }
    for standing in snapshot.standings.iter_mut().take(2) {
        if standing.race_num == 12 {
            standing.position = 1;
        } else {
            standing.position = 2;
        }
    }
    for _ in 0..4 {
        let leader_x = snapshot.riders[0].x;
        snapshot.riders[1].x = leader_x - lead;
        snapshot.riders[1].z = snapshot.riders[0].z;
        snapshot.local_track_pos = snapshot.riders[1].track_pos;
        snapshot.local_x = snapshot.riders[1].x;
        snapshot.local_z = snapshot.riders[1].z;
        let _ = RaceStore::tick(&snapshot);
    }
    assert_eq!(
        live_position(1),
        1,
        "120 m up the lap, matching the bikes, stays ahead"
    );
    assert_eq!(live_position(12), 2);
}

/// Steps that freeze the leader: 10 m of spline on a 3 m chord, until that
/// step is past 80 m, then 2 m steps the pending jump keeps refusing.
fn ride_frozen_hairpin(
    snapshot: &mut Snapshot,
    creep_steps: i32,
    creep_chord: f32,
) {
    let length = snapshot.track_length;
    let step = |snapshot: &mut Snapshot, spline: f32, leader_chord: f32| {
        snapshot.riders[0].track_pos =
            (snapshot.riders[0].track_pos + spline / length).rem_euclid(1.0);
        snapshot.riders[0].x += leader_chord;
        snapshot.riders[1].track_pos =
            (snapshot.riders[1].track_pos + spline / length).rem_euclid(1.0);
        snapshot.riders[1].x += spline;
        snapshot.local_track_pos = snapshot.riders[1].track_pos;
        snapshot.local_x = snapshot.riders[1].x;
        snapshot.local_z = snapshot.riders[1].z;
        let _ = RaceStore::tick(snapshot);
    };
    for _ in 0..9 {
        step(snapshot, 10.0, 3.0);
    }
    for _ in 0..creep_steps {
        step(snapshot, 2.0, creep_chord);
    }
}

/// Wolf Creek finish: ~400 m up the lap, world chord about 140 m. The hairpin
/// freezes the leader's coordinate. The centerline leader stays P1.
#[test]
fn a_finish_hairpin_keeps_the_centerline_leader() {
    let _g = session_lock();
    reset_session();
    let mut snapshot = gate_field(&[1, 12]);
    let length = 2463.0;
    snapshot.track_length = length;
    snapshot.sf_meters = length;
    snapshot.poly_count = 0;
    for standing in snapshot.standings.iter_mut().take(2) {
        standing.num_laps = 4;
    }
    let lead = 400.0;
    let leader_frac = 0.50;
    let trailer_frac = leader_frac - lead / length;
    ride_field_to(&mut snapshot, &[(1, leader_frac), (12, trailer_frac)]);
    assert_eq!(live_position(1), 1, "matching lead before the hairpin");
    ride_frozen_hairpin(&mut snapshot, 180, 0.9);
    assert_eq!(
        live_position(1),
        1,
        "a 400 m centerline lead on a short chord stays ahead"
    );
    assert_eq!(live_position(12), 2);
}

/// Just after the line the same chord returns: leader near 0.20, trailer near
/// 0, spline about 480 m, world under half of that.
#[test]
fn a_post_cross_hairpin_keeps_the_centerline_leader() {
    let _g = session_lock();
    reset_session();
    let mut snapshot = gate_field(&[1, 12]);
    let length = 2463.0;
    snapshot.track_length = length;
    snapshot.sf_meters = length;
    snapshot.poly_count = 0;
    for standing in snapshot.standings.iter_mut().take(2) {
        standing.num_laps = 4;
    }
    let lead = 480.0;
    let advance = 90.0 + 220.0 * 2.0;
    let leader_end = 0.196;
    let leader_start = (leader_end - advance / length).rem_euclid(1.0);
    let trailer_start = (leader_start - lead / length).rem_euclid(1.0);
    ride_field_to(&mut snapshot, &[(1, leader_start), (12, trailer_start)]);
    assert_eq!(live_position(1), 1, "matching lead before the line");
    ride_frozen_hairpin(&mut snapshot, 220, 0.88);
    assert_eq!(
        live_position(1),
        1,
        "past the line, the centerline leader on the hairpin stays ahead"
    );
    assert_eq!(live_position(12), 2);
}

/// Forest: ~125 m up the lap. The coordinate freezes on a short chord, then the
/// world gap sits near 50 m and opens to about 100 m. Both stay with the leader.
#[test]
fn a_forest_corner_keeps_the_raw_leader_past_half_the_spline() {
    let _g = session_lock();
    reset_session();
    let mut snapshot = gate_field(&[1, 12]);
    let length = 1256.0;
    snapshot.track_length = length;
    snapshot.sf_meters = length;
    snapshot.poly_count = 0;
    for standing in snapshot.standings.iter_mut().take(2) {
        standing.num_laps = 4;
    }
    let lead = 125.0;
    let leader_frac = 0.50;
    let trailer_frac = leader_frac - lead / length;
    ride_field_to(&mut snapshot, &[(1, leader_frac), (12, trailer_frac)]);
    assert_eq!(live_position(1), 1, "matching lead before the corner");

    let step = |snapshot: &mut Snapshot, spline: f32, leader_dx: f32, trailer_dx: f32| {
        let length = snapshot.track_length;
        snapshot.riders[0].track_pos =
            (snapshot.riders[0].track_pos + spline / length).rem_euclid(1.0);
        snapshot.riders[0].x += leader_dx;
        snapshot.riders[1].track_pos =
            (snapshot.riders[1].track_pos + spline / length).rem_euclid(1.0);
        snapshot.riders[1].x += trailer_dx;
        snapshot.local_track_pos = snapshot.riders[1].track_pos;
        snapshot.local_x = snapshot.riders[1].x;
        snapshot.local_z = snapshot.riders[1].z;
        let _ = RaceStore::tick(snapshot);
    };
    for _ in 0..9 {
        step(&mut snapshot, 10.0, 3.0, 6.0);
    }
    for _ in 0..25 {
        step(&mut snapshot, 2.0, 1.0, 2.0);
    }
    for _ in 0..12 {
        step(&mut snapshot, 2.0, 0.0, 2.0);
    }
    assert_eq!(
        live_position(1),
        1,
        "125 m up the lap and about 50 m away stays ahead"
    );
    for _ in 0..13 {
        step(&mut snapshot, 2.0, 0.0, -4.0);
    }
    assert_eq!(
        live_position(1),
        1,
        "the same corner opened to about 100 m stays ahead"
    );
    assert_eq!(live_position(12), 2);
}

/// The coordinate freezes, the body walks a hook so the path passes 45 m while
/// the net chord stays under 45 m, and the other bike closes to about 25 m.
fn a_frozen_corner_keeps_the_raw_leader_inside_45m(lead: f32) {
    reset_session();
    let mut snapshot = gate_field(&[1, 12]);
    let length = 1256.0;
    snapshot.track_length = length;
    snapshot.sf_meters = length;
    snapshot.poly_count = 0;
    for standing in snapshot.standings.iter_mut().take(2) {
        standing.num_laps = 4;
    }
    let leader_frac = 0.50;
    let trailer_frac = leader_frac - lead / length;
    ride_field_to(&mut snapshot, &[(1, leader_frac), (12, trailer_frac)]);
    assert_eq!(live_position(1), 1, "matching lead before the corner");

    let step = |snapshot: &mut Snapshot,
                spline: f32,
                leader_dx: f32,
                leader_dz: f32,
                trailer_dx: f32| {
        let length = snapshot.track_length;
        snapshot.riders[0].track_pos =
            (snapshot.riders[0].track_pos + spline / length).rem_euclid(1.0);
        snapshot.riders[0].x += leader_dx;
        snapshot.riders[0].z += leader_dz;
        snapshot.riders[1].track_pos =
            (snapshot.riders[1].track_pos + spline / length).rem_euclid(1.0);
        snapshot.riders[1].x += trailer_dx;
        snapshot.local_track_pos = snapshot.riders[1].track_pos;
        snapshot.local_x = snapshot.riders[1].x;
        snapshot.local_z = snapshot.riders[1].z;
        let _ = RaceStore::tick(snapshot);
    };
    for _ in 0..9 {
        step(&mut snapshot, 10.0, 3.0, 0.0, 6.0);
    }
    for _ in 0..20 {
        step(&mut snapshot, 2.0, 0.0, 1.0, 2.0);
    }
    for _ in 0..12 {
        let length = snapshot.track_length;
        let leader_x = snapshot.riders[0].x;
        let leader_z = snapshot.riders[0].z;
        snapshot.riders[0].track_pos =
            (snapshot.riders[0].track_pos + 2.0 / length).rem_euclid(1.0);
        snapshot.riders[1].track_pos =
            (snapshot.riders[1].track_pos + 2.0 / length).rem_euclid(1.0);
        snapshot.riders[1].x = leader_x - 24.0;
        snapshot.riders[1].z = leader_z;
        snapshot.local_track_pos = snapshot.riders[1].track_pos;
        snapshot.local_x = snapshot.riders[1].x;
        snapshot.local_z = snapshot.riders[1].z;
        let _ = RaceStore::tick(&snapshot);
    }
    assert_eq!(
        live_position(1),
        1,
        "{lead:.0} m up the lap and about 25 m away stays ahead"
    );
    assert_eq!(live_position(12), 2);
}

#[test]
fn a_frozen_corner_keeps_a_120m_and_a_143m_lead_inside_45m() {
    let _g = session_lock();
    a_frozen_corner_keeps_the_raw_leader_inside_45m(120.0);
    a_frozen_corner_keeps_the_raw_leader_inside_45m(143.0);
}

/// The leader's coordinate freezes while the other bike keeps the spline. The
/// raw lead stays past the corner caps, and the chord stays most of that gap.
fn a_spread_chord_keeps_the_raw_leader(lead: f32, chord: f32) {
    reset_session();
    let mut snapshot = gate_field(&[1, 12]);
    let length = 1256.0;
    snapshot.track_length = length;
    snapshot.sf_meters = length;
    snapshot.poly_count = 0;
    for standing in snapshot.standings.iter_mut().take(2) {
        standing.num_laps = 4;
    }
    let leader_frac = 0.50;
    let trailer_frac = leader_frac - lead / length;
    ride_field_to(&mut snapshot, &[(1, leader_frac), (12, trailer_frac)]);
    assert_eq!(live_position(1), 1, "matching lead before the corner");

    let step = |snapshot: &mut Snapshot, spline: f32, leader_dx: f32, trailer_dx: f32| {
        let length = snapshot.track_length;
        snapshot.riders[0].track_pos =
            (snapshot.riders[0].track_pos + spline / length).rem_euclid(1.0);
        snapshot.riders[0].x += leader_dx;
        snapshot.riders[1].track_pos =
            (snapshot.riders[1].track_pos + spline / length).rem_euclid(1.0);
        snapshot.riders[1].x += trailer_dx;
        snapshot.local_track_pos = snapshot.riders[1].track_pos;
        snapshot.local_x = snapshot.riders[1].x;
        snapshot.local_z = snapshot.riders[1].z;
        let _ = RaceStore::tick(snapshot);
    };
    for _ in 0..26 {
        step(&mut snapshot, 10.0, 1.0, 6.0);
    }
    for _ in 0..19 {
        step(&mut snapshot, 2.0, 1.0, 2.0);
    }
    for _ in 0..12 {
        let length = snapshot.track_length;
        snapshot.riders[0].track_pos =
            (snapshot.riders[0].track_pos + 2.0 / length).rem_euclid(1.0);
        snapshot.riders[0].x += 0.5;
        snapshot.riders[1].track_pos =
            (snapshot.riders[1].track_pos + 2.0 / length).rem_euclid(1.0);
        snapshot.riders[1].x = snapshot.riders[0].x - chord;
        snapshot.riders[1].z = snapshot.riders[0].z;
        snapshot.local_track_pos = snapshot.riders[1].track_pos;
        snapshot.local_x = snapshot.riders[1].x;
        snapshot.local_z = snapshot.riders[1].z;
        let _ = RaceStore::tick(&snapshot);
    }
    assert_eq!(
        live_position(1),
        1,
        "{lead:.0} m up the lap and about {chord:.0} m away stays ahead"
    );
    assert_eq!(live_position(12), 2);
}

#[test]
fn a_spread_chord_keeps_a_160m_and_a_230m_lead() {
    let _g = session_lock();
    a_spread_chord_keeps_the_raw_leader(160.0, 110.0);
    a_spread_chord_keeps_the_raw_leader(230.0, 150.0);
}

/// Out, 44 m across, and back. Polyline length matches `track_length`.
fn folded_back_line(snapshot: &mut Snapshot, track_length: f32) -> (f32, f32) {
    let cross = 44.0;
    let leg = (track_length - cross) / 2.0;
    let points = [(0.0, 0.0), (leg, 0.0), (leg, cross), (0.0, cross)];
    snapshot.poly_count = points.len() as i32;
    for (index, (x, z)) in points.iter().enumerate() {
        snapshot.poly[index] = Point { x: *x, z: *z };
    }
    let along = (track_length - 400.0) / 2.0;
    (along, cross)
}

/// The lap comes back to 44 m. A bike 400 m further around, sitting on that line,
/// stays ahead even when the game still has them behind. Same metres on a short lap
/// and a long one.
fn a_line_fold_keeps_a_real_lead(track_length: f32) {
    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = track_length;
    snapshot.sf_meters = track_length;
    let (along, cross) = folded_back_line(&mut snapshot, track_length);
    snapshot.standings[0].position = 2;
    snapshot.standings[1].position = 1;
    snapshot.riders[1].track_pos = along / track_length;
    snapshot.riders[1].x = along;
    snapshot.riders[1].z = 0.0;
    snapshot.local_track_pos = snapshot.riders[1].track_pos;
    snapshot.local_x = along;
    snapshot.local_z = 0.0;
    snapshot.riders[0].track_pos = (track_length - along) / track_length;
    snapshot.riders[0].x = along;
    snapshot.riders[0].z = cross;
    let held = RaceStore::tick(&snapshot).field;
    assert_eq!(
        live_position(1),
        1,
        "{track_length} m: 44 m across and 400 m around the lap stays ahead"
    );
    assert_eq!(live_position(12), 2);
    assert_eq!(held.rows[0].standing.race_num, 1);
}

#[test]
fn a_folded_line_keeps_a_real_lead_on_a_short_lap_and_a_long_one() {
    let _g = session_lock();
    a_line_fold_keeps_a_real_lead(900.0);
    a_line_fold_keeps_a_real_lead(2480.0);
}

/// Out, 28 m across, and back, then the rest of the lap. The return sits ~380 m
/// along the line from its pair and 28 m away in the world.
fn fold_beside_the_pack(snapshot: &mut Snapshot, track_length: f32) -> (f32, f32, f32) {
    let cross = 28.0;
    let outbound = 220.0;
    let pack_x = 34.0;
    let points = [
        (0.0, 0.0),
        (outbound, 0.0),
        (outbound, cross),
        (0.0, cross),
        (track_length - (outbound * 2.0 + cross), cross),
    ];
    snapshot.poly_count = points.len() as i32;
    for (index, (x, z)) in points.iter().enumerate() {
        snapshot.poly[index] = Point { x: *x, z: *z };
    }
    let return_start = outbound + cross;
    let pack_along = return_start + (outbound - pack_x);
    let focus_along = pack_along + 15.0;
    let jump_along = pack_x;
    (pack_along, focus_along, jump_along)
}

/// Nevada: the fraction jumps across a fold the body does not ride, and the
/// place stays with the bikes beside it. Same metres on 821 m and on a long lap.
fn an_unridden_fold_jump_keeps_the_place(track_length: f32) {
    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = track_length;
    snapshot.sf_meters = track_length;
    let (pack_along, focus_along, jump_along) = fold_beside_the_pack(&mut snapshot, track_length);
    let laps = 1;
    let pack_x = 34.0;
    let focus_x = pack_x - 15.0;
    let focus_z = 52.0;
    let seated = [
        (1, 1, pack_along / track_length, pack_x, 28.0),
        (2, 2, (pack_along - 2.0) / track_length, pack_x + 2.0, 28.0),
        (3, 3, (pack_along + 2.0) / track_length, pack_x - 2.0, 28.0),
        (12, 4, focus_along / track_length, focus_x, focus_z),
    ];
    for (index, (race_num, position, fraction, x, z)) in seated.iter().enumerate() {
        seat(
            &mut snapshot,
            index,
            *race_num,
            *position,
            laps,
            *x,
            *z,
            *fraction,
        );
    }
    snapshot.standing_count = seated.len() as i32;
    snapshot.rider_count = seated.len() as i32;
    snapshot.local_track_pos = focus_along / track_length;
    snapshot.local_x = focus_x;
    snapshot.local_z = focus_z;
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(
        live_position(12),
        1,
        "{track_length} m: ahead of the pack before the jump"
    );

    snapshot.riders[3].track_pos = jump_along / track_length;
    snapshot.riders[3].x = focus_x + 2.0;
    snapshot.local_track_pos = jump_along / track_length;
    snapshot.local_x = focus_x + 2.0;
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(
        live_position(12),
        1,
        "{track_length} m: an unridden fold jump does not drop the place (order {:?})",
        crate::race_store::live_order_nums()
    );
    assert!(live_position(12) < live_position(1));
    assert!(live_position(12) < live_position(2));
    assert!(live_position(12) < live_position(3));
}

#[test]
fn an_unridden_fold_jump_keeps_the_place_on_a_short_lap_and_a_long_one() {
    let _g = session_lock();
    an_unridden_fold_jump_keeps_the_place(821.0);
    an_unridden_fold_jump_keeps_the_place(2000.0);
}

/// The bodies are 30 m apart, but the centerline samples at those fractions are not.
/// That is still the next lane.
#[test]
fn a_far_sample_chord_does_not_pass() {
    let _g = session_lock();
    reset_session();
    let mut snapshot = live_snap();
    let track_length = 2480.0;
    snapshot.track_length = track_length;
    snapshot.sf_meters = track_length;
    snapshot.poly_count = 2;
    snapshot.poly[0] = Point { x: 0.0, z: 0.0 };
    snapshot.poly[1] = Point {
        x: track_length,
        z: 0.0,
    };
    snapshot.standings[0].position = 2;
    snapshot.standings[1].position = 1;
    let behind = 0.20;
    snapshot.riders[1].track_pos = behind;
    snapshot.riders[1].x = behind * track_length;
    snapshot.riders[1].z = 0.0;
    snapshot.local_track_pos = behind;
    snapshot.local_x = snapshot.riders[1].x;
    snapshot.local_z = 0.0;
    snapshot.riders[0].track_pos = behind + 208.0 / track_length;
    snapshot.riders[0].x = snapshot.riders[1].x + 30.0;
    snapshot.riders[0].z = 0.0;
    let held = RaceStore::tick(&snapshot).field;
    assert_eq!(live_position(12), 1, "30 m away is not a pass when the line stays apart");
    assert_eq!(live_position(1), 2);
    assert_eq!(held.rows[0].standing.race_num, 12);
}

/// Farm14, the other way: you are ahead on the lap, and a bike 44.8 m away whose
/// fraction is ~200 m behind does not take the place.
#[test]
fn a_forty_five_metre_chord_behind_does_not_take_the_place() {
    let _g = session_lock();
    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = 1669.0;
    snapshot.sf_meters = 1669.0;
    snapshot.standings[0].position = 2;
    snapshot.standings[1].position = 1;
    snapshot.riders[1].track_pos = 0.669;
    snapshot.riders[1].x = 0.0;
    snapshot.riders[1].z = 0.0;
    snapshot.riders[0].track_pos = 0.669 - 200.0 / 1669.0;
    snapshot.riders[0].x = 44.8;
    snapshot.riders[0].z = 0.0;
    snapshot.local_track_pos = 0.669;
    snapshot.local_x = 0.0;
    snapshot.local_z = 0.0;
    let held = RaceStore::tick(&snapshot).field;
    assert_eq!(live_position(12), 1, "44.8 m away and 200 m back does not pass");
    assert_eq!(live_position(1), 2);
    assert_eq!(held.rows[0].standing.race_num, 12);
}

/// Washougal: 29.6 m away and ~208 m up the lap does not take the place, and the
/// next sample at 31 m still does not.
#[test]
fn a_thirty_metre_chord_does_not_flash_the_place() {
    let _g = session_lock();
    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = 2480.0;
    snapshot.sf_meters = 2480.0;
    let ahead = 0.766 + 208.0 / 2480.0;
    snapshot.riders[0].track_pos = ahead;
    snapshot.riders[0].x = 29.6;
    snapshot.riders[0].z = 0.0;
    snapshot.riders[1].track_pos = 0.766;
    snapshot.riders[1].x = 0.0;
    snapshot.riders[1].z = 0.0;
    snapshot.local_track_pos = 0.766;
    snapshot.local_x = 0.0;
    snapshot.local_z = 0.0;
    let held = RaceStore::tick(&snapshot).field;
    assert_eq!(live_position(1), 1, "29.6 m away and 208 m up the lap stays ahead");
    assert_eq!(live_position(12), 2);
    assert_eq!(held.rows[0].standing.race_num, 1);

    snapshot.riders[0].x = 31.0;
    let held = RaceStore::tick(&snapshot).field;
    assert_eq!(live_position(1), 1, "31 m away still stays ahead");
    assert_eq!(live_position(12), 2);
    assert_eq!(held.rows[0].standing.race_num, 1);
}

/// Washougal opening lap: a 159 m centerline lead held at 44.9 m does not jump
/// through when the bodies tick to 48.7 m. It counts once they are past 55 m.
#[test]
fn a_refused_fold_holds_until_the_bodies_clear_the_lane() {
    let _g = session_lock();
    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = 2480.0;
    snapshot.sf_meters = 2480.0;
    snapshot.standings[0].position = 2;
    snapshot.standings[1].position = 1;
    snapshot.riders[1].track_pos = 0.261;
    snapshot.riders[1].x = 0.0;
    snapshot.riders[1].z = 0.0;
    snapshot.riders[0].track_pos = 0.261 + 159.0 / 2480.0;
    snapshot.riders[0].x = 44.9;
    snapshot.riders[0].z = 0.0;
    snapshot.local_track_pos = 0.261;
    snapshot.local_x = 0.0;
    snapshot.local_z = 0.0;
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(live_position(12), 1, "44.9 m refuses the 159 m lead");

    snapshot.riders[0].x = 48.7;
    let held = RaceStore::tick(&snapshot).field;
    assert_eq!(live_position(12), 1, "48.7 m does not release the lead");
    assert_eq!(live_position(1), 2);
    assert_eq!(held.rows[0].standing.race_num, 12);

    snapshot.riders[0].x = 56.0;
    let released = RaceStore::tick(&snapshot).field;
    assert_eq!(live_position(1), 1, "56 m is past the lane and the lead counts");
    assert_eq!(released.rows[0].standing.race_num, 1);
}

fn seat(
    snapshot: &mut Snapshot,
    index: usize,
    race_num: i32,
    position: i32,
    laps: i32,
    x: f32,
    z: f32,
    fraction: f32,
) {
    snapshot.standings[index] = standing(race_num, position, laps);
    snapshot.riders[index] = rider(race_num, x, z, fraction);
}

/// Maryland holeshot: a few bikes project just past the origin while the rest of
/// the grid is still at the end of the lap. The larger group is the place.
#[test]
fn a_minority_branch_inside_the_pack_does_not_lead() {
    let _g = session_lock();
    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = 1795.0;
    snapshot.sf_meters = 1795.0;
    let laps = 5;
    let seated = [
        (1, 1, 0.003, 1.0, 14.0),
        (6, 2, 0.010, 5.0, 14.0),
        (12, 3, 0.880, 0.0, 0.0),
        (2, 4, 0.884, 4.0, 1.0),
        (3, 5, 0.888, 8.0, 0.0),
        (4, 6, 0.900, 12.0, 2.0),
        (5, 7, 0.882, 2.0, 6.0),
    ];
    for (index, (race_num, position, fraction, x, z)) in seated.iter().enumerate() {
        seat(&mut snapshot, index, *race_num, *position, laps, *x, *z, *fraction);
    }
    snapshot.standing_count = seated.len() as i32;
    snapshot.rider_count = seated.len() as i32;
    snapshot.local_track_pos = 0.880;
    snapshot.local_x = 0.0;
    snapshot.local_z = 0.0;
    let field = RaceStore::tick(&snapshot).field;
    assert_eq!(live_position(4), 1, "the bike at 0.90 leads the grid");
    assert!(live_position(1) > 1, "a 0.003 projection does not keep P1");
    assert_eq!(field.rows[0].standing.race_num, 4);
}

/// Grid of eight at `origin`, plus four bikes `along_m` up the spline and
/// `world_gap_m` past the front of that grid. The four start as the game
/// leaders when `group_starts_ahead` is set, so a missed pull leaves them P1.
fn seat_pack_and_group(
    track_length: f32,
    along_m: f32,
    world_gap_m: f32,
    group_starts_ahead: bool,
) -> (Snapshot, i32, [i32; 4]) {
    let mut snapshot = live_snap();
    snapshot.track_length = track_length;
    snapshot.sf_meters = track_length;
    let laps = 1;
    let origin = 0.40;
    let grid_count = 8usize;
    let step = 2.0 / track_length;
    let median_offset = (grid_count - 1) as f32 * 0.5 * step;
    let mut front = 0;
    let mut front_fraction = 0.0;
    let mut front_x = 0.0;
    for slot in 0..grid_count {
        let race_num = if slot + 1 == grid_count { 12 } else { slot as i32 + 1 };
        let fraction = origin + slot as f32 * step;
        let x = slot as f32 * 2.0;
        // The front of the grid sits just behind the projected group in the game
        // order, so it only has to pass that group. A 2 m grid step is inside the
        // pass margin and would not bubble on its own.
        let game_place = if group_starts_ahead {
            if slot + 1 == grid_count {
                5
            } else {
                6 + slot as i32
            }
        } else {
            slot as i32 + 1
        };
        seat(&mut snapshot, slot, race_num, game_place, laps, x, 0.0, fraction);
        if fraction >= front_fraction {
            front = race_num;
            front_fraction = fraction;
            front_x = x;
        }
    }
    let group = [21, 22, 23, 24];
    let group_x = front_x + world_gap_m;
    for (slot, race_num) in group.iter().enumerate() {
        let fraction =
            origin + median_offset + along_m / track_length + slot as f32 / track_length;
        let game_place = if group_starts_ahead {
            slot as i32 + 1
        } else if slot + 1 == group.len() {
            grid_count as i32 + 1
        } else {
            grid_count as i32 + 2 + slot as i32
        };
        seat(
            &mut snapshot,
            grid_count + slot,
            *race_num,
            game_place,
            laps,
            group_x,
            slot as f32,
            fraction,
        );
    }
    let seated = grid_count + group.len();
    snapshot.standing_count = seated as i32;
    snapshot.rider_count = seated as i32;
    snapshot.local_race_num = 12;
    snapshot.focus_race_num = 12;
    snapshot.local_track_pos = front_fraction;
    snapshot.local_x = front_x;
    snapshot.local_z = 0.0;
    (snapshot, front, group)
}

/// Maryland holeshot: four bikes share a fraction ~219 m up the lap while their
/// bodies are ~25 m from the larger pack. That is not a lead. Dropping the
/// position keeps the pack spot, so the front of the grid does not fall to P2.
fn a_folded_group_inside_the_pack_does_not_lead(track_length: f32) {
    let _g = session_lock();
    reset_session();
    let (mut snapshot, front, group) = seat_pack_and_group(track_length, 219.0, 25.0, true);
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(live_position(front), 1, "the grid leads, not the projection");
    for race_num in group {
        assert!(
            live_position(race_num) > 1,
            "a folded group bike is not P1 on {track_length} m"
        );
    }
    for rider in snapshot
        .riders
        .iter_mut()
        .take(snapshot.rider_count as usize)
    {
        if group.contains(&rider.race_num) {
            rider.track_pos = -1.0;
            rider.x = 0.0;
            rider.z = 0.0;
        }
    }
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(
        live_position(front),
        1,
        "a dropped projection does not sit in front of the grid"
    );
    for race_num in group {
        assert!(
            live_position(race_num) > 1,
            "a dropped fold does not become P1 on {track_length} m"
        );
    }
}

#[test]
fn a_folded_group_inside_the_pack_does_not_lead_on_a_short_lap_and_a_long_one() {
    a_folded_group_inside_the_pack_does_not_lead(1970.0);
    a_folded_group_inside_the_pack_does_not_lead(821.0);
}

/// Out 80 m, 25 m across, and back. The return is 185 m along the lap and 25 m
/// from the start in the world. The rest of the lap keeps the polyline at
/// `track_length`.
fn hairpin_back_beside_the_pack(snapshot: &mut Snapshot, track_length: f32) -> f32 {
    let outbound = 80.0;
    let cross = 25.0;
    let folded = outbound * 2.0 + cross;
    let points = [
        (0.0, 0.0),
        (outbound, 0.0),
        (outbound, cross),
        (0.0, cross),
        (track_length - folded, cross),
    ];
    snapshot.poly_count = points.len() as i32;
    for (index, (x, z)) in points.iter().enumerate() {
        snapshot.poly[index] = Point { x: *x, z: *z };
    }
    folded
}

/// 755: the race leader is over 120 m up the lap, and the line comes back so the
/// body is 25 m from a larger pack. That pack does not take the place. A second
/// tick does not store the pull. Same metres on a short lap and a long one.
fn a_hairpin_pack_does_not_take_the_lead(track_length: f32) {
    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = track_length;
    snapshot.sf_meters = track_length;
    let leader_along = hairpin_back_beside_the_pack(&mut snapshot, track_length);
    let laps = 1;
    let leader_fraction = leader_along / track_length;
    seat(
        &mut snapshot,
        0,
        1,
        1,
        laps,
        0.0,
        25.0,
        leader_fraction,
    );
    let pack = [(12, 0.0), (2, 6.0), (3, 12.0), (4, 18.0)];
    for (slot, (race_num, along)) in pack.iter().enumerate() {
        seat(
            &mut snapshot,
            slot + 1,
            *race_num,
            (slot + 2) as i32,
            laps,
            *along,
            0.0,
            along / track_length,
        );
    }
    snapshot.standing_count = 5;
    snapshot.rider_count = 5;
    snapshot.local_race_num = 12;
    snapshot.focus_race_num = 12;
    snapshot.local_track_pos = 0.0;
    snapshot.local_x = 0.0;
    snapshot.local_z = 0.0;
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(
        live_position(1),
        1,
        "{track_length} m: a 25 m hairpin does not hand the lead to the pack"
    );
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(
        live_position(1),
        1,
        "{track_length} m: the pack pull is not stored across the hairpin"
    );
}

#[test]
fn a_hairpin_pack_does_not_take_the_lead_on_a_short_lap_and_a_long_one() {
    let _g = session_lock();
    a_hairpin_pack_does_not_take_the_lead(2124.0);
    a_hairpin_pack_does_not_take_the_lead(821.0);
}

/// A group under 120 m, about 30 m away across the corner, keeps the lead.
#[test]
fn a_corner_group_under_120m_stays_ahead() {
    let _g = session_lock();
    reset_session();
    for track_length in [821.0, 1970.0] {
        reset_session();
        let (snapshot, _front, group) = seat_pack_and_group(track_length, 90.0, 30.0, false);
        let _ = RaceStore::tick(&snapshot);
        assert_eq!(
            live_position(group[3]),
            1,
            "90 m across a corner stays ahead on {track_length} m"
        );
    }
}

/// A group ~219 m up the lap whose bodies are that far away keeps the lead.
#[test]
fn a_group_219m_up_the_lap_stays_ahead() {
    let _g = session_lock();
    reset_session();
    for track_length in [821.0, 1970.0] {
        reset_session();
        let (snapshot, _front, group) = seat_pack_and_group(track_length, 219.0, 219.0, false);
        let _ = RaceStore::tick(&snapshot);
        assert_eq!(
            live_position(group[3]),
            1,
            "219 m of track and world stays ahead on {track_length} m"
        );
    }
}

/// Pala: the centerline folds so 0.05 and 0.45 are the same point, and the
/// spline gap is well over 120 m. Dropping the trailer does not hand them the lead.
fn a_dropped_position_does_not_take_the_place(track_length: f32) {
    let _g = session_lock();
    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = track_length;
    snapshot.sf_meters = track_length;
    let leg = track_length / 4.0;
    let line = [(0.0, 0.0), (leg, 0.0), (0.0, 0.0), (leg, 0.0), (0.0, 0.0)];
    snapshot.poly_count = line.len() as i32;
    for (index, (x, z)) in line.iter().enumerate() {
        snapshot.poly[index] = Point { x: *x, z: *z };
    }
    let laps = 1;
    seat(&mut snapshot, 0, 1, 1, laps, 0.0, 0.0, 0.45);
    seat(&mut snapshot, 1, 2, 2, laps, 100.0, 0.0, 0.05);
    snapshot.standing_count = 2;
    snapshot.rider_count = 2;
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(
        live_position(1),
        1,
        "the bike up the lap leads on {track_length} m"
    );
    snapshot.riders[1].track_pos = -1.0;
    snapshot.riders[1].x = 0.0;
    snapshot.riders[1].z = 0.0;
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(
        live_position(1),
        1,
        "a dropped position does not take the place on {track_length} m"
    );
    assert_eq!(live_position(2), 2, "the trailer stays behind on {track_length} m");
}

#[test]
fn a_dropped_position_does_not_take_the_place_on_a_short_lap_and_a_long_one() {
    a_dropped_position_does_not_take_the_place(2124.0);
    a_dropped_position_does_not_take_the_place(821.0);
}

/// RedBud holeshot: the pack's fractions flip for one sample and the dash jumps
/// several places. That order waits until the next sample still wants it. A real
/// pass that is still there then publishes. The gap is metres, so a short lap and
/// a long one match.
fn an_opening_lap_jump_waits_for_the_next_sample(track_length: f32) {
    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = track_length;
    snapshot.sf_meters = track_length;
    snapshot.poly_count = 0;
    snapshot.current_lap = 1;
    snapshot.standing_count = 4;
    snapshot.rider_count = 4;
    let gap = 10.0;
    let base = 0.40 * track_length;
    for index in 0..4 {
        let x = base + index as f32 * gap;
        seat(
            &mut snapshot,
            index,
            (index + 1) as i32,
            (4 - index) as i32,
            0,
            x,
            0.0,
            x / track_length,
        );
    }
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(live_position(4), 1, "the front of the pack leads on {track_length} m");
    assert_eq!(live_position(1), 4, "the back of the pack is last on {track_length} m");

    for index in 0..4 {
        let x = base + (3 - index) as f32 * gap;
        snapshot.riders[index].x = x;
        snapshot.riders[index].track_pos = x / track_length;
    }
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(
        live_position(4),
        1,
        "one flipped sample does not move several places on {track_length} m"
    );
    assert_eq!(live_position(1), 4, "the back stays back for that sample on {track_length} m");
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(
        live_position(1),
        1,
        "the same order on the next sample publishes on {track_length} m"
    );
    assert_eq!(live_position(4), 4, "the old leader is last once it holds on {track_length} m");
}

#[test]
fn an_opening_lap_jump_waits_on_a_short_lap_and_a_long_one() {
    let _g = session_lock();
    an_opening_lap_jump_waits_for_the_next_sample(2124.0);
    an_opening_lap_jump_waits_for_the_next_sample(821.0);
}

/// RedBud stripe: the lap count rises and `track_pos` is gone on that same sample.
/// The rider stays just past the line, ahead of a bike at fraction 0. A short lap
/// and a long one use the same 25 m pin.
fn a_lap_count_without_a_position_stays_ahead_of_the_stripe(track_length: f32) {
    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = track_length;
    snapshot.sf_meters = track_length;
    snapshot.poly_count = 0;
    snapshot.standing_count = 2;
    snapshot.rider_count = 2;
    snapshot.local_race_num = 12;
    snapshot.focus_race_num = 12;
    let laps = 2;
    seat(
        &mut snapshot,
        0,
        1,
        1,
        laps,
        0.96 * track_length,
        0.0,
        0.96,
    );
    seat(&mut snapshot, 1, 12, 2, laps, 0.0, 0.0, 0.0);
    snapshot.local_track_pos = 0.0;
    snapshot.local_x = 0.0;
    snapshot.local_z = 0.0;
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(live_position(1), 1, "the leader is ahead before the drop on {track_length} m");

    snapshot.standings[0].num_laps = laps + 1;
    snapshot.riders[0].track_pos = -1.0;
    snapshot.riders[0].x = 0.0;
    snapshot.riders[0].z = 0.0;
    snapshot.standings[1].num_laps = laps + 1;
    snapshot.riders[1].track_pos = 0.0;
    snapshot.current_lap = laps + 2;
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(
        live_position(1),
        1,
        "a missing leader stays ahead of the stripe on {track_length} m"
    );
    assert_eq!(
        live_position(12),
        2,
        "fraction 0 does not pass the dropout on {track_length} m"
    );
}

#[test]
fn a_lap_count_without_a_position_stays_ahead_on_a_short_lap_and_a_long_one() {
    let _g = session_lock();
    a_lap_count_without_a_position_stays_ahead_of_the_stripe(2124.0);
    a_lap_count_without_a_position_stays_ahead_of_the_stripe(821.0);
}

/// Meadow Valley holeshot: `track_pos` of exactly 1 is the line, not the next lap.
/// The bike that has wrapped to 0.01 stays ahead of it, and it stays ahead of 0.90.
/// The gap is metres, so a short lap and a long one match.
fn an_exact_lap_stays_at_the_line(track_length: f32) {
    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = track_length;
    snapshot.sf_meters = track_length;
    snapshot.poly_count = 0;
    snapshot.current_lap = 1;
    snapshot.standing_count = 3;
    snapshot.rider_count = 3;
    seat(&mut snapshot, 0, 1, 1, 0, 0.0, 0.0, 1.0);
    seat(
        &mut snapshot,
        1,
        2,
        2,
        0,
        0.01 * track_length,
        0.0,
        0.01,
    );
    seat(
        &mut snapshot,
        2,
        12,
        3,
        0,
        0.90 * track_length,
        0.0,
        0.90,
    );
    snapshot.local_track_pos = 0.90;
    snapshot.local_x = 0.90 * track_length;
    snapshot.local_z = 0.0;
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(
        live_position(2),
        1,
        "{track_length} m: 0.01 has crossed and leads the bike still on the line"
    );
    assert_eq!(
        live_position(1),
        2,
        "{track_length} m: track_pos 1 stays ahead of 0.90"
    );
    assert_eq!(
        live_position(12),
        3,
        "{track_length} m: 0.90 does not jump the line"
    );
}

#[test]
fn an_exact_lap_stays_at_the_line_on_a_short_lap_and_a_long_one() {
    let _g = session_lock();
    an_exact_lap_stays_at_the_line(2124.0);
    an_exact_lap_stays_at_the_line(821.0);
}

/// Monster Mountain holeshot: the raw fraction steps back beside the pack and the
/// body does not ride that jump. The stored coordinate stays up the lap. The bike
/// now ahead on the raw fraction takes the place. The gap is metres, so a short
/// lap and a long one match.
fn a_stored_lead_the_body_left_does_not_keep_the_place(track_length: f32) {
    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = track_length;
    snapshot.sf_meters = track_length;
    snapshot.poly_count = 0;
    snapshot.current_lap = 1;
    snapshot.standing_count = 2;
    snapshot.rider_count = 2;
    let ahead = 0.50 + 100.0 / track_length;
    seat(&mut snapshot, 0, 1, 1, 0, 0.0, 0.0, ahead);
    seat(&mut snapshot, 1, 12, 2, 0, -10.0, 0.0, 0.50);
    snapshot.local_track_pos = 0.50;
    snapshot.local_x = -10.0;
    snapshot.local_z = 0.0;
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(
        live_position(1),
        1,
        "{track_length} m: the first sample up the lap leads"
    );
    snapshot.riders[0].track_pos = 0.50 - 40.0 / track_length;
    snapshot.riders[0].x = 10.0;
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(
        live_position(12),
        1,
        "{track_length} m: the raw fraction ahead of the frozen coordinate leads"
    );
    assert_eq!(
        live_position(1),
        2,
        "{track_length} m: a stored lead the body left does not keep the place"
    );
}

#[test]
fn a_stored_lead_the_body_left_does_not_keep_the_place_on_a_short_lap_and_a_long_one() {
    let _g = session_lock();
    a_stored_lead_the_body_left_does_not_keep_the_place(2124.0);
    a_stored_lead_the_body_left_does_not_keep_the_place(821.0);
}

/// Farm14 holeshot: the stored coordinate still reads as the next lane, and the
/// score agrees, but the raw fraction is only a pack gap. That gap takes the
/// place. The metres match on a short lap and a long one.
fn a_raw_pack_gap_beats_a_stored_lane(track_length: f32) {
    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = track_length;
    snapshot.sf_meters = track_length;
    snapshot.poly_count = 0;
    snapshot.current_lap = 1;
    snapshot.standing_count = 2;
    snapshot.rider_count = 2;
    let origin = 0.40;
    seat(&mut snapshot, 0, 1, 1, 0, 0.0, 0.0, origin);
    let false_lead = origin + 200.0 / track_length;
    seat(&mut snapshot, 1, 12, 2, 0, 9.0, 0.0, false_lead);
    snapshot.local_track_pos = false_lead;
    snapshot.local_x = 9.0;
    snapshot.local_z = 0.0;
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(
        live_position(1),
        1,
        "{track_length} m: 9 m away is not a 200 m lead"
    );
    snapshot.riders[1].track_pos = origin + 40.0 / track_length;
    snapshot.local_track_pos = snapshot.riders[1].track_pos;
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(
        live_position(12),
        1,
        "{track_length} m: a 40 m raw lead in the pack takes the place"
    );
    assert_eq!(
        live_position(1),
        2,
        "{track_length} m: the stored lane does not keep the other bike ahead"
    );
}

/// Lake Elsinore: the opening pack is one group in the world and two branches
/// on the centerline, about 260 m apart. The far branch does not take the lead.
/// A bike 7 m up the lap and 9 m away still passes.
#[test]
fn an_opening_pack_fold_keeps_the_game_place() {
    let _g = session_lock();
    reset_session();
    let length = 1748.0;
    let mut snapshot = live_snap();
    snapshot.track_length = length;
    snapshot.sf_meters = length;
    snapshot.poly_count = 0;
    snapshot.current_lap = 1;
    snapshot.holeshot_race_num = 1;
    let you = 0.78 - 20.0 / length;
    let folded = 0.78 + 260.0 / length;
    let pack = [
        (1, 1, 0.78),
        (2, 2, 0.78 - 4.0 / length),
        (3, 3, 0.78 - 8.0 / length),
        (4, 4, 0.78 - 12.0 / length),
        (5, 5, 0.78 - 16.0 / length),
        (6, 6, 0.78 - 18.0 / length),
        (12, 7, you),
        (8, 8, folded),
    ];
    for (index, (race_num, place, fraction)) in pack.iter().enumerate() {
        let along = if *race_num == 8 { 24.0 } else { index as f32 * 3.0 };
        seat(
            &mut snapshot,
            index,
            *race_num,
            *place,
            0,
            along,
            0.0,
            *fraction,
        );
    }
    snapshot.standing_count = pack.len() as i32;
    snapshot.rider_count = pack.len() as i32;
    snapshot.local_race_num = 12;
    snapshot.focus_race_num = 12;
    snapshot.local_track_pos = you;
    snapshot.local_x = 6.0 * 3.0;
    snapshot.local_z = 0.0;
    let _ = RaceStore::tick(&snapshot);
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(live_position(12), 7, "the folded branch does not move you");
    assert_ne!(live_position(8), 1, "the far branch is not P1");
    assert_eq!(live_position(1), 1, "the game leader stays P1");

    let ahead = you + 7.0 / length;
    snapshot.riders[7].track_pos = ahead;
    snapshot.riders[7].x = snapshot.local_x + 9.0;
    snapshot.riders[7].z = 0.0;
    let _ = RaceStore::tick(&snapshot);
    let _ = RaceStore::tick(&snapshot);
    assert!(
        live_position(8) < live_position(12),
        "7 m up the lap and 9 m away takes the place, got P{} behind P{}",
        live_position(8),
        live_position(12)
    );
}

#[test]
fn a_raw_pack_gap_beats_a_stored_lane_on_a_short_lap_and_a_long_one() {
    let _g = session_lock();
    a_raw_pack_gap_beats_a_stored_lane(2124.0);
    a_raw_pack_gap_beats_a_stored_lane(821.0);
}

/// Same completed lap: the higher fraction is ahead. Wrapping across the origin
/// does not put the lower fraction in front. A few metres further up the lap
/// still passes. The fractions are the same on a short lap and a long one.
fn a_lower_fraction_does_not_pass_on_the_same_lap(track_length: f32) {
    let _g = session_lock();
    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = track_length;
    snapshot.sf_meters = track_length;
    snapshot.poly_count = 0;
    let laps = 5;
    seat(
        &mut snapshot,
        0,
        1,
        2,
        laps,
        0.80 * track_length,
        0.0,
        0.80,
    );
    seat(
        &mut snapshot,
        1,
        12,
        1,
        laps,
        0.12 * track_length,
        0.0,
        0.12,
    );
    snapshot.standing_count = 2;
    snapshot.rider_count = 2;
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(
        live_position(1),
        1,
        "the higher fraction leads on {track_length} m"
    );
    assert_eq!(
        live_position(12),
        2,
        "a lower fraction across the origin does not pass on {track_length} m"
    );

    reset_session();
    let lead = 0.40;
    let pass = lead + 12.0 / track_length;
    seat(
        &mut snapshot,
        0,
        1,
        1,
        laps,
        lead * track_length,
        0.0,
        lead,
    );
    seat(
        &mut snapshot,
        1,
        12,
        2,
        laps,
        pass * track_length,
        0.0,
        pass,
    );
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(
        live_position(12),
        1,
        "a few metres ahead still passes on {track_length} m"
    );
}

#[test]
fn a_lower_fraction_does_not_pass_on_a_short_lap_and_a_long_one() {
    a_lower_fraction_does_not_pass_on_the_same_lap(2124.0);
    a_lower_fraction_does_not_pass_on_the_same_lap(821.0);
}

/// The bikes are about 100 m apart, so a fraction still before the line is not
/// a lap lead. The rider just past the origin keeps the place. The same
/// fractions on the short way, with the bikes that far apart, still pass.
fn an_unmatched_origin_gap_does_not_pass(track_length: f32) {
    let _g = session_lock();
    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = track_length;
    snapshot.sf_meters = track_length;
    snapshot.poly_count = 0;
    let laps = 5;
    seat(&mut snapshot, 0, 1, 1, laps, 0.0, 0.0, 0.16);
    seat(&mut snapshot, 1, 12, 2, laps, 100.0, 0.0, 0.95);
    snapshot.standing_count = 2;
    snapshot.rider_count = 2;
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(
        live_position(1),
        1,
        "a gap the bikes do not match stays put on {track_length} m"
    );
    assert_eq!(
        live_position(12),
        2,
        "a fraction before the line does not pass on {track_length} m"
    );

    reset_session();
    let short_m = 0.12 * track_length;
    seat(&mut snapshot, 0, 1, 1, laps, short_m, 0.0, 0.90);
    seat(&mut snapshot, 1, 12, 2, laps, 0.0, 0.0, 0.02);
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(
        live_position(12),
        1,
        "the rider just past the origin passes when the bikes match on {track_length} m"
    );
}

#[test]
fn an_unmatched_origin_gap_does_not_pass_on_a_short_lap_and_a_long_one() {
    an_unmatched_origin_gap_does_not_pass(2124.0);
    an_unmatched_origin_gap_does_not_pass(821.0);
}

/// Temecula published a second race number on the leader's bike. The copy has the
/// worse game place, so it stays last. The leader still uses that body.
fn a_copied_body_stays_with_the_better_game_place(track_length: f32) {
    let _g = session_lock();
    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = track_length;
    snapshot.sf_meters = track_length;
    snapshot.poly_count = 0;
    let laps = 5;
    let lead = 0.50 * track_length;
    seat(&mut snapshot, 0, 1, 1, laps, lead, 0.0, 0.50);
    seat(&mut snapshot, 1, 12, 2, laps, lead - 80.0, 0.0, 0.40);
    seat(&mut snapshot, 2, 3, 3, laps, lead, 0.0, 0.50);
    snapshot.standing_count = 3;
    snapshot.rider_count = 3;
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(
        live_position(1),
        1,
        "the better game place keeps the shared body on {track_length} m"
    );
    assert_eq!(
        live_position(3),
        3,
        "a copied body does not take the place on {track_length} m"
    );
}

#[test]
fn a_copied_body_stays_with_the_better_game_place_on_a_short_lap_and_a_long_one() {
    a_copied_body_stays_with_the_better_game_place(2124.0);
    a_copied_body_stays_with_the_better_game_place(821.0);
}

/// Potecas: the raw fraction already matches the bikes beside this rider, but the
/// stored coordinate was accepted on the other branch. The next sample that matches
/// the pack is where they are, behind the bikes further up the lap.
#[test]
fn a_stuck_coordinate_follows_the_bikes_beside_it() {
    let _g = session_lock();
    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = 1586.0;
    snapshot.sf_meters = 1586.0;
    let laps = 4;
    let pack = [
        (12, 6, 0.440, 0.0, 0.0),
        (2, 7, 0.440, 3.0, 0.0),
        (3, 8, 0.440, 6.0, 1.0),
        (4, 9, 0.440, 9.0, 0.0),
    ];
    // A real gap up the lap: the bodies are as far as the spline, not folded
    // into the pack beside this rider.
    let leaders = [
        (5, 2, 0.590, 0.0, 240.0),
        (6, 3, 0.590, 3.0, 240.0),
        (7, 4, 0.590, 6.0, 240.0),
        (8, 5, 0.590, 9.0, 240.0),
    ];
    let mut index = 0;
    seat(&mut snapshot, index, 1, 1, laps, 1.5, 1.0, 0.800);
    index += 1;
    for (race_num, position, fraction, x, z) in pack {
        seat(&mut snapshot, index, race_num, position, laps, x, z, fraction);
        index += 1;
    }
    for (race_num, position, fraction, x, z) in leaders {
        seat(&mut snapshot, index, race_num, position, laps, x, z, fraction);
        index += 1;
    }
    snapshot.standing_count = index as i32;
    snapshot.rider_count = index as i32;
    snapshot.local_race_num = 12;
    snapshot.focus_race_num = 12;
    snapshot.local_track_pos = 0.440;
    snapshot.local_x = 0.0;
    snapshot.local_z = 0.0;
    let _ = RaceStore::tick(&snapshot);
    snapshot.riders[0].track_pos = 0.440;
    let field = RaceStore::tick(&snapshot).field;
    assert_eq!(live_position(5), 1, "the 0.59 bikes lead");
    assert!(
        live_position(1) > 4,
        "a coordinate stuck on the other branch does not stay ahead"
    );
    assert_eq!(field.rows[0].standing.race_num, 5);
}

/// MXB Club: the fraction jumps onto the next lane while the bike moves about 2 m.
/// Three bikes within 45 m confirm that lane, and the raw fraction sits ahead of
/// the bike in front. Join the pack. The same metres on a short lap and a long one.
fn an_unridden_jump_joins_the_pack(track_length: f32) {
    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = track_length;
    snapshot.sf_meters = track_length;
    snapshot.poly_count = 0;
    let laps = 1;
    let pack = 0.50;
    let behind = pack - 200.0 / track_length;
    let front = pack + 15.0 / track_length;
    let jumped = pack + 25.0 / track_length;
    let seated = [
        (1, 2, front, 15.0, 0.0),
        (2, 3, pack, 0.0, 0.0),
        (3, 4, pack, 4.0, 0.0),
        (4, 5, pack, 8.0, 1.0),
        (12, 1, behind, 0.0, 10.0),
    ];
    for (index, (race_num, position, fraction, x, z)) in seated.iter().enumerate() {
        seat(
            &mut snapshot,
            index,
            *race_num,
            *position,
            laps,
            *x,
            *z,
            *fraction,
        );
    }
    snapshot.standing_count = seated.len() as i32;
    snapshot.rider_count = seated.len() as i32;
    snapshot.local_track_pos = behind;
    snapshot.local_x = 0.0;
    snapshot.local_z = 10.0;
    let _ = RaceStore::tick(&snapshot);

    snapshot.riders[4].track_pos = jumped;
    snapshot.riders[4].x = 2.0;
    snapshot.local_track_pos = jumped;
    snapshot.local_x = 2.0;
    let field = RaceStore::tick(&snapshot).field;
    assert!(
        live_position(1) < live_position(12),
        "{track_length} m: an unridden jump does not pass the bike in front (front {} jumper {} order {:?})",
        live_position(1),
        live_position(12),
        crate::race_store::live_order_nums()
    );
    assert!(
        live_position(12) < seated.len() as i32,
        "{track_length} m: the jump joins the pack instead of staying 200 m back"
    );
    assert_ne!(field.rows[0].standing.race_num, 12);
}

#[test]
fn an_unridden_jump_joins_the_pack_on_a_short_lap_and_a_long_one() {
    let _g = session_lock();
    an_unridden_jump_joins_the_pack(400.0);
    an_unridden_jump_joins_the_pack(2000.0);
}

/// Ezkutu: the next lane is inside 45 m, and a bike further up this line is about as
/// far away as the spline says. That fraction stays ahead of the lane beside it.
#[test]
fn a_confirmed_fraction_is_not_replaced_by_the_lane_beside_it() {
    let _g = session_lock();
    reset_session();
    let mut snapshot = live_snap();
    let length = 2247.0;
    snapshot.track_length = length;
    snapshot.sf_meters = length;
    let laps = 1;
    let rider_fraction = 0.240;
    let leader_fraction = rider_fraction + 95.0 / length;
    let seated = [
        (2, 1, 0.100, 12.0, 8.0),
        (3, 2, 0.102, 8.0, -10.0),
        (4, 3, 0.098, -6.0, 16.0),
        (12, 4, rider_fraction, 0.0, 0.0),
        (1, 5, leader_fraction, 95.0, 0.0),
    ];
    for (index, (race_num, position, fraction, x, z)) in seated.iter().enumerate() {
        seat(&mut snapshot, index, *race_num, *position, laps, *x, *z, *fraction);
    }
    snapshot.standing_count = seated.len() as i32;
    snapshot.rider_count = seated.len() as i32;
    snapshot.local_track_pos = rider_fraction;
    snapshot.local_x = 0.0;
    snapshot.local_z = 0.0;
    let field = RaceStore::tick(&snapshot).field;
    assert_eq!(live_position(1), 1, "the bike 95 m ahead leads");
    assert_eq!(live_position(12), 2, "the confirmed fraction stays ahead of the next lane");
    assert!(live_position(2) > 2, "the 0.10 lane does not take the place");
    assert_eq!(field.rows[1].standing.race_num, 12);
}

/// Briarcliff holeshot: bikes ~12 m apart with only ~30 m of centerline between them
/// are the same pack, not a parallel lane.
#[test]
fn a_thirty_metre_pack_gap_still_passes() {
    let _g = session_lock();
    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = 1833.0;
    snapshot.sf_meters = 1833.0;
    snapshot.riders[0].track_pos = 0.50;
    snapshot.riders[0].x = 0.0;
    snapshot.riders[0].z = 0.0;
    let lead = 29.0 / 1833.0;
    snapshot.riders[1].track_pos = 0.50 + lead;
    snapshot.riders[1].x = 12.0;
    snapshot.riders[1].z = 0.0;
    snapshot.local_track_pos = snapshot.riders[1].track_pos;
    snapshot.local_x = 12.0;
    snapshot.local_z = 0.0;
    let field = RaceStore::tick(&snapshot).field;
    assert_eq!(live_position(12), 1, "12 m away and 29 m up the lap is still a pass");
    assert_eq!(field.rows[0].standing.race_num, 12);
}

/// Running alongside is not a pass. Two bikes swapping every frame would be unreadable.
#[test]
fn side_by_side_keeps_the_scored_order() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    pass_the_leader(&mut s, 1.0);
    let field = RaceStore::tick(&s).field;
    assert_eq!(live_position(12), 2);
    assert_eq!(field.rows[0].standing.race_num, 1);
}

/// Once the place is taken it is held on a smaller margin, so it only goes back when they
/// are really back in front.
#[test]
fn a_taken_place_is_held_until_they_drop_back() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    pass_the_leader(&mut s, 6.0);
    let _ = RaceStore::tick(&s);
    assert_eq!(live_position(12), 1);
    pass_the_leader(&mut s, 1.0);
    let _ = RaceStore::tick(&s);
    assert_eq!(live_position(12), 1, "still ahead, keeps the place");
    pass_the_leader(&mut s, -1.5);
    let _ = RaceStore::tick(&s);
    assert_eq!(live_position(12), 2, "back behind, gives the place up");
}

/// A practice or warmup field is ranked by lap time. Being further round the lap than
/// someone on a flying lap is not a place.
#[test]
fn warmup_keeps_the_lap_time_order() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_kind = -1;
    s.session_length = 40;
    pass_the_leader(&mut s, 20.0);
    assert!(is_warmup(&s));
    let field = RaceStore::tick(&s).field;
    assert_eq!(field.rows[0].standing.race_num, 1);
    assert_eq!(live_position(12), 2);
}

/// Open practice and a long practice with leaked extras stay on lap time. Track position
/// is not a place.
#[test]
fn open_practice_keeps_the_lap_time_order() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_kind = -1;
    s.session_laps = 0;
    s.session_length = 0;
    pass_the_leader(&mut s, 20.0);
    assert!(is_practice_session(&s));
    assert!(!is_warmup(&s));
    let field = RaceStore::tick(&s).field;
    assert_eq!(field.rows[0].standing.race_num, 1);
    assert_eq!(live_position(12), 2);
    assert_eq!(field.rows[1].standing.gap_ms, 1_500);

    reset_session();
    s.session_kind = -1;
    s.session_laps = 2;
    s.session_length = 40;
    pass_the_leader(&mut s, 20.0);
    assert!(is_practice_session(&s), "40 min stays practice when extras leak");
    assert!(!is_warmup(&s));
    let field = RaceStore::tick(&s).field;
    assert_eq!(field.rows[0].standing.race_num, 1);
    assert_eq!(live_position(12), 2);
}

/// A faster lap is the place, even when the game still has that rider further down the
/// classification and someone with no lap is further around the track.
#[test]
fn practice_and_warmup_rank_by_best_lap() {
    let _g = session_lock();
    for (kind, length, laps) in [(5, 10, 0), (-1, 0, 0)] {
        reset_session();
        let mut s = live_snap();
        s.session_kind = kind;
        s.session_length = length;
        s.session_laps = laps;
        s.standing_count = 3;
        s.rider_count = 3;
        s.standings[0] = standing(1, 1, 2);
        s.standings[0].best_lap_ms = 95_000;
        s.standings[1] = standing(12, 2, 2);
        s.standings[1].best_lap_ms = 90_000;
        s.standings[1].gap_ms = 4_000;
        s.standings[2] = standing(7, 3, 1);
        s.standings[2].best_lap_ms = 0;
        s.standings[2].gap_ms = 8_000;
        s.riders[2] = rider(7, 40.0, 0.0, 0.99);
        let field = RaceStore::tick(&s).field;
        assert_eq!(field.rows[0].standing.race_num, 12, "kind {kind}");
        assert_eq!(field.rows[0].standing.position, 1);
        assert_eq!(field.rows[0].standing.gap_ms, 0);
        assert_eq!(field.rows[1].standing.race_num, 1);
        assert_eq!(field.rows[1].standing.gap_ms, 5_000);
        assert_eq!(field.rows[1].interval_ms, 5_000);
        assert_eq!(field.rows[2].standing.race_num, 7, "no lap stays last");
        assert_eq!(field.rows[2].standing.gap_ms, 0);
        assert_eq!(field.rows[2].interval_ms, 0);
        assert_eq!(live_position(12), 1);
        assert_eq!(live_position(7), 3);
    }
}

/// On the gate everyone sits on the same stretch of track and `track_pos` noise is not a
/// holeshot.
#[test]
fn the_start_gate_keeps_the_scored_order() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    IN_GATE.store(1, Ordering::Relaxed);
    pass_the_leader(&mut s, 20.0);
    let field = RaceStore::tick(&s).field;
    assert_eq!(field.rows[0].standing.race_num, 1);
    assert!(
        field.rows.iter().all(|row| row.standing.position == 0),
        "no race place before the gate drops"
    );
    assert_eq!(live_position(1), 0);
    assert_eq!(live_position(12), 0);
    assert_eq!(super::standing_pos(&s, 12), 0);
    IN_GATE.store(0, Ordering::Relaxed);
}

/// Grid sits on the S2 side of the sector bisector. Qualifying order must not stick:
/// the rider further around the lap is ahead once the gate drops.
#[test]
fn the_open_lap_follows_the_grid_not_the_sector_bisector() {
    let _g = session_lock();
    let _splits = ResetSplits;
    reset_session();
    crate::sector::set_split_fracs([0.40, 0.70]);
    let mut s = live_snap();
    s.track_length = 2000.0;
    s.sf_meters = 0.0;
    s.current_lap = 1;
    s.standing_count = 2;
    s.rider_count = 2;
    s.standings[0] = standing(1, 1, 0);
    s.standings[1] = standing(12, 2, 0);
    s.riders[0] = rider(1, 0.0, 0.0, 0.98);
    s.riders[1] = rider(12, 1.0, 0.0, 0.02);
    s.local_track_pos = 0.02;
    IN_GATE.store(1, Ordering::Relaxed);
    let on_gate = RaceStore::tick(&s);
    assert!(on_gate.field.rows.iter().all(|row| row.standing.position == 0));
    IN_GATE.store(0, Ordering::Relaxed);
    let sitting = RaceStore::tick(&s);
    assert!(
        sitting.field.rows.iter().all(|row| row.standing.position == 0),
        "still in the gate slots"
    );
    assert_eq!(super::standing_pos(&s, 12), 0, "qualifying place does not show");
    s.holeshot_race_num = 1;
    ride_field_to(&mut s, &[(1, 0.99), (12, 0.03)]);
    let running = RaceStore::tick(&s);
    assert_eq!(live_position(12), 1, "further forward off the gate");
    assert_eq!(live_position(1), 2);
    assert_eq!(running.field.rows[0].standing.race_num, 12);
    assert_eq!(running.field.rows[0].standing.position, 1);
}

/// A rider who crashed and lost track position keeps the spot they went down.
/// Riding past that spot takes the place.
#[test]
fn passing_a_crashed_rider_on_the_open_lap_takes_the_place() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.track_length = 2000.0;
    s.sf_meters = 0.0;
    s.current_lap = 1;
    s.standing_count = 2;
    s.rider_count = 2;
    s.standings[0] = standing(1, 1, 0);
    s.standings[1] = standing(12, 2, 0);
    s.riders[0] = rider(1, 0.0, 0.0, 0.20);
    s.riders[1] = rider(12, 1.0, 0.0, 0.10);
    s.local_track_pos = 0.10;
    IN_GATE.store(1, Ordering::Relaxed);
    let _ = RaceStore::tick(&s);
    IN_GATE.store(0, Ordering::Relaxed);
    s.holeshot_race_num = 1;
    ride_field_to(&mut s, &[(1, 0.22), (12, 0.12)]);
    assert_eq!(live_position(1), 1, "ahead on the grid");
    s.riders[0].track_pos = -1.0;
    s.riders[0].crashed = 1;
    s.standings[0].crashed = 1;
    ride_field_to(&mut s, &[(12, 0.35)]);
    assert_eq!(live_position(12), 1, "rode past the crash");
    assert_eq!(live_position(1), 2);
}

/// Going down does not freeze the place you had. A rider who rides past the crash
/// takes it.
#[test]
fn a_rider_who_rides_past_your_crash_takes_the_place() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.track_length = 2000.0;
    s.sf_meters = 0.0;
    s.current_lap = 1;
    s.standing_count = 2;
    s.rider_count = 2;
    s.standings[0] = standing(1, 1, 0);
    s.standings[1] = standing(12, 2, 0);
    s.riders[0] = rider(1, 0.0, 0.0, 0.10);
    s.riders[1] = rider(12, 1.0, 0.0, 0.20);
    s.local_track_pos = 0.20;
    IN_GATE.store(1, Ordering::Relaxed);
    let _ = RaceStore::tick(&s);
    IN_GATE.store(0, Ordering::Relaxed);
    s.holeshot_race_num = 1;
    ride_field_to(&mut s, &[(1, 0.12), (12, 0.22)]);
    assert_eq!(live_position(12), 1, "ahead when the gate drops");
    s.riders[1].track_pos = -1.0;
    s.riders[1].crashed = 1;
    s.standings[1].crashed = 1;
    s.local_track_pos = -1.0;
    ride_field_to(&mut s, &[(1, 0.40)]);
    assert_eq!(live_position(1), 1, "rode past you");
    assert_eq!(live_position(12), 2);
}

/// The pre-start countdown has ended and `IN_GATE` is clear, but the game still has
/// the gate up. Qualifying places stay hidden, and they stay hidden after the drop
/// until the holeshot.
#[test]
fn the_countdown_end_does_not_show_a_place_before_the_gate_drops() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.track_length = 2000.0;
    s.sf_meters = 0.0;
    s.current_lap = 1;
    s.session_state = 256;
    s.standing_count = 2;
    s.rider_count = 2;
    s.standings[0] = standing(1, 1, 0);
    s.standings[1] = standing(12, 2, 0);
    s.riders[0] = rider(1, 0.0, 0.0, 0.10);
    s.riders[1] = rider(12, 1.0, 0.0, 0.14);
    s.local_track_pos = 0.14;
    IN_GATE.store(0, Ordering::Relaxed);
    let sitting = RaceStore::tick(&s).field;
    assert!(sitting.rows.iter().all(|row| row.standing.position == 0));
    assert_eq!(super::standing_pos(&s, 12), 0, "qualifying place does not show");
    assert_eq!(super::standing_pos(&s, 1), 0);
    assert_eq!(live_position(12), 0);
    s.session_state = 16;
    ride_field_to(&mut s, &[(1, 0.12), (12, 0.16)]);
    assert_eq!(s.holeshot_race_num, 0);
    assert_eq!(live_position(12), 0, "left the slot, still before the holeshot");
    assert_eq!(live_position(1), 0);
    assert_eq!(super::standing_pos(&s, 12), 0);
}

/// The front slot is not P1 at the drop. Places stay blank until the holeshot line,
/// then a pack that launched first is ahead of a late front slot.
#[test]
fn a_late_start_is_not_p1_before_the_holeshot() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.track_length = 2000.0;
    s.sf_meters = 0.0;
    s.current_lap = 1;
    s.standing_count = 2;
    s.rider_count = 2;
    s.standings[0] = standing(1, 1, 0);
    s.standings[1] = standing(12, 2, 0);
    s.riders[0] = rider(1, 0.0, 0.0, 0.10);
    s.riders[1] = rider(12, 1.0, 0.0, 0.14);
    s.local_track_pos = 0.14;
    IN_GATE.store(1, Ordering::Relaxed);
    let _ = RaceStore::tick(&s);
    IN_GATE.store(0, Ordering::Relaxed);
    ride_field_to(&mut s, &[(1, 0.11), (12, 0.16)]);
    assert_eq!(s.holeshot_race_num, 0);
    assert_eq!(live_position(12), 0, "left the slot, still before the holeshot");
    assert_eq!(live_position(1), 0);
    assert_eq!(super::standing_pos(&s, 12), 0);
    s.holeshot_race_num = 1;
    ride_field_to(&mut s, &[(1, 0.30), (12, 0.18)]);
    assert_eq!(live_position(1), 1, "the pack launched");
    assert_eq!(live_position(12), 2, "late off the front slot");
}

/// Briarcliff: the gate slots are more than 200 m apart on the spline and a few
/// metres apart in the world. Rolling out of the slot is not a holeshot.
fn a_folded_gate_is_not_the_holeshot(track_length: f32) {
    let _g = session_lock();
    reset_session();
    let gap_m = 205.0;
    let ahead = (0.10 + gap_m / track_length).rem_euclid(1.0);
    let mut snapshot = live_snap();
    snapshot.track_length = track_length;
    snapshot.sf_meters = 0.0;
    snapshot.current_lap = 1;
    snapshot.standing_count = 2;
    snapshot.rider_count = 2;
    snapshot.standings[0] = standing(1, 2, 0);
    snapshot.standings[1] = standing(12, 1, 0);
    snapshot.riders[0] = rider(1, 8.0, 0.0, ahead);
    snapshot.riders[1] = rider(12, 0.0, 0.0, 0.10);
    snapshot.local_track_pos = 0.10;
    snapshot.local_x = 0.0;
    snapshot.local_z = 0.0;
    IN_GATE.store(1, Ordering::Relaxed);
    let _ = RaceStore::tick(&snapshot);
    IN_GATE.store(0, Ordering::Relaxed);
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(
        live_position(12),
        0,
        "a folded gate does not publish on {track_length} m"
    );
    let nudge = 10.0;
    snapshot.riders[1].track_pos = (0.10 + nudge / track_length).rem_euclid(1.0);
    snapshot.riders[1].x = nudge;
    snapshot.local_track_pos = snapshot.riders[1].track_pos;
    snapshot.local_x = nudge;
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(snapshot.holeshot_race_num, 0);
    assert_eq!(
        live_position(12),
        0,
        "leaving the slot is still before the holeshot on {track_length} m"
    );
    snapshot.holeshot_race_num = 1;
    let _ = RaceStore::tick(&snapshot);
    assert_ne!(
        live_position(12),
        0,
        "the holeshot publishes on {track_length} m"
    );
    assert_ne!(live_position(1), 0, "the holeshot publishes on {track_length} m");

    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = track_length;
    snapshot.sf_meters = 0.0;
    snapshot.current_lap = 1;
    snapshot.standing_count = 2;
    snapshot.rider_count = 2;
    snapshot.standings[0] = standing(1, 2, 0);
    snapshot.standings[1] = standing(12, 1, 0);
    snapshot.riders[0] = rider(1, 8.0, 0.0, ahead);
    snapshot.riders[1] = rider(12, 0.0, 0.0, 0.10);
    snapshot.local_track_pos = 0.10;
    snapshot.local_x = 0.0;
    snapshot.local_z = 0.0;
    snapshot.holeshot_race_num = 0;
    IN_GATE.store(1, Ordering::Relaxed);
    let _ = RaceStore::tick(&snapshot);
    IN_GATE.store(0, Ordering::Relaxed);
    let mut travelled = 0.0f32;
    while travelled < 201.0 {
        let step = 67.0f32;
        travelled += step;
        let rider = &mut snapshot.riders[0];
        rider.track_pos = (rider.track_pos + step / track_length).rem_euclid(1.0);
        rider.x += step;
        let _ = RaceStore::tick(&snapshot);
    }
    assert_eq!(snapshot.holeshot_race_num, 0);
    assert_eq!(
        live_position(1),
        1,
        "200 m of body travel publishes without a callback on {track_length} m"
    );
}

#[test]
fn a_folded_gate_is_not_the_holeshot_on_a_short_lap_and_a_long_one() {
    a_folded_gate_is_not_the_holeshot(1833.0);
    a_folded_gate_is_not_the_holeshot(821.0);
}
/// That step must not wrap into almost a full lap.
#[test]
fn falling_behind_the_gate_does_not_keep_p1() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.track_length = 2000.0;
    s.sf_meters = 0.0;
    s.current_lap = 1;
    s.standing_count = 2;
    s.rider_count = 2;
    s.standings[0] = standing(1, 1, 0);
    s.standings[1] = standing(12, 2, 0);
    s.riders[0] = rider(1, 0.0, 0.0, 0.10);
    s.riders[1] = rider(12, 1.0, 0.0, 0.14);
    s.local_track_pos = 0.14;
    IN_GATE.store(1, Ordering::Relaxed);
    let _ = RaceStore::tick(&s);
    IN_GATE.store(0, Ordering::Relaxed);
    let sitting = RaceStore::tick(&s);
    assert!(sitting.field.rows.iter().all(|row| row.standing.position == 0));
    s.holeshot_race_num = 1;
    ride_field_to(&mut s, &[(1, 0.12), (12, 0.20)]);
    assert_eq!(live_position(12), 1, "front of the grid once the field has left");
    ride_field_to(&mut s, &[(1, 0.30), (12, 0.08)]);
    assert_eq!(live_position(1), 1, "pack rode past");
    assert_eq!(live_position(12), 2, "behind the gate is not a full lap");
}

/// Finishing the open lap before `num_laps` publishes must not drop the leader
/// behind riders still on that lap.
#[test]
fn crossing_the_line_before_num_laps_keeps_the_lead() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.track_length = 2000.0;
    s.sf_meters = 0.0;
    s.current_lap = 1;
    s.standing_count = 2;
    s.rider_count = 2;
    s.standings[0] = standing(1, 1, 0);
    s.standings[1] = standing(12, 2, 0);
    s.riders[0] = rider(1, 0.0, 0.0, 0.10);
    s.riders[1] = rider(12, 1.0, 0.0, 0.12);
    s.local_track_pos = 0.12;
    IN_GATE.store(1, Ordering::Relaxed);
    let _ = RaceStore::tick(&s);
    IN_GATE.store(0, Ordering::Relaxed);
    s.holeshot_race_num = 1;
    ride_field_to(&mut s, &[(1, 0.50), (12, 1.15)]);
    assert_eq!(s.standings[1].num_laps, 0);
    assert_eq!(live_position(12), 1, "wrapped the open lap");
    assert_eq!(live_position(1), 2);
    s.standings[1].num_laps = 1;
    let _ = RaceStore::tick(&s);
    assert_eq!(live_position(12), 1, "the lap bump does not drop the leader");
    ride_field_to(&mut s, &[(1, 0.70), (12, 0.40)]);
    assert_eq!(live_position(12), 1, "riding forward is not a pass against you");
    assert_eq!(live_position(1), 2);
}

/// Riders on different laps are ranked right by the classification already: a lapped
/// rider alongside you is not ahead of you.
#[test]
fn a_lapped_rider_alongside_does_not_take_the_place() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.standings[1].num_laps = 4;
    s.standings[1].gap_laps = 1;
    pass_the_leader(&mut s, 20.0);
    let field = RaceStore::tick(&s).field;
    assert_eq!(field.rows[0].standing.race_num, 1);
    assert_eq!(live_position(12), 2);
}

/// Lap 0 at 0.51 stays behind lap 1 at 0.38. Lap 1 at 0.86 does not take P1 from lap 2.
/// The gate sits just before the line, so the scored coordinate is the fraction from
/// that gate and a higher fraction on the earlier lap used to sort ahead.
fn a_lap_down_fraction_stays_behind(track_length: f32) {
    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = track_length;
    snapshot.sf_meters = 0.0;
    snapshot.current_lap = 1;
    snapshot.standing_count = 2;
    snapshot.rider_count = 2;
    snapshot.standings[0] = standing(1, 1, 0);
    snapshot.standings[1] = standing(12, 2, 0);
    snapshot.riders[0] = rider(1, 0.0, 0.0, 0.95);
    snapshot.riders[1] = rider(12, 10.0, 0.0, 0.95);
    snapshot.local_track_pos = 0.95;
    snapshot.local_x = 10.0;
    IN_GATE.store(1, Ordering::Relaxed);
    let _ = RaceStore::tick(&snapshot);
    IN_GATE.store(0, Ordering::Relaxed);
    snapshot.holeshot_race_num = 1;
    ride_field_to(&mut snapshot, &[(1, 1.38), (12, 1.51)]);
    snapshot.standings[0].num_laps = 1;
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(live_position(1), 1, "lap 1 at 0.38 on {track_length} m");
    assert_eq!(live_position(12), 2, "lap 0 at 0.51 on {track_length} m");
}

/// The lap-2 leader's body drops out with the coordinate still on the previous lap,
/// so a lap-1 bike at 0.86 scores further up the gate coordinate.
fn a_lap_behind_does_not_take_the_lead(track_length: f32) {
    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = track_length;
    snapshot.sf_meters = 0.0;
    snapshot.current_lap = 1;
    snapshot.standing_count = 2;
    snapshot.rider_count = 2;
    snapshot.standings[0] = standing(1, 1, 0);
    snapshot.standings[1] = standing(12, 2, 0);
    snapshot.riders[0] = rider(1, 0.0, 0.0, 0.10);
    snapshot.riders[1] = rider(12, 20.0, 0.0, 0.10);
    snapshot.local_track_pos = 0.10;
    snapshot.local_x = 20.0;
    IN_GATE.store(1, Ordering::Relaxed);
    let _ = RaceStore::tick(&snapshot);
    IN_GATE.store(0, Ordering::Relaxed);
    snapshot.holeshot_race_num = 1;
    ride_field_to(&mut snapshot, &[(1, 1.20), (12, 1.20)]);
    snapshot.standings[0].num_laps = 2;
    snapshot.rider_count = 1;
    snapshot.riders[0] = snapshot.riders[1];
    ride_field_to(&mut snapshot, &[(12, 0.86)]);
    snapshot.standings[1].num_laps = 1;
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(live_position(1), 1, "lap 2 keeps P1 on {track_length} m");
    assert_eq!(live_position(12), 2, "lap 1 at 0.86 on {track_length} m");
}

#[test]
fn a_lower_lap_fraction_does_not_take_the_place() {
    let _g = session_lock();
    for &track_length in &[900.0_f32, 2480.0] {
        a_lap_down_fraction_stays_behind(track_length);
        a_lap_behind_does_not_take_the_lead(track_length);
    }
}

fn place_on_track(snapshot: &mut Snapshot, race_num: i32, fraction: f32) {
    let count = snapshot.rider_count as usize;
    let pos = fraction.rem_euclid(1.0);
    if let Some(rider) = snapshot.riders[..count]
        .iter_mut()
        .find(|rider| rider.race_num == race_num)
    {
        rider.track_pos = pos;
        rider.x = pos * snapshot.track_length;
        rider.z = 0.0;
        if race_num == snapshot.focus_race_num {
            snapshot.local_track_pos = pos;
            snapshot.local_x = rider.x;
            snapshot.local_z = 0.0;
        }
    }
}

/// Lap 0 at 0.98 is still short of the line. The lap-1 bike at 0.02 has crossed.
/// The bodies sit on that short gap. The coordinate may already count the lap.
fn a_bike_short_of_the_line_stays_behind(track_length: f32) {
    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = track_length;
    snapshot.sf_meters = 0.0;
    snapshot.current_lap = 1;
    snapshot.standing_count = 2;
    snapshot.rider_count = 2;
    snapshot.standings[0] = standing(1, 1, 0);
    snapshot.standings[1] = standing(12, 2, 0);
    snapshot.riders[0] = rider(1, 0.0, 0.0, 0.90);
    snapshot.riders[1] = rider(12, 10.0, 0.0, 0.90);
    snapshot.local_track_pos = 0.90;
    snapshot.local_x = 10.0;
    IN_GATE.store(1, Ordering::Relaxed);
    let _ = RaceStore::tick(&snapshot);
    IN_GATE.store(0, Ordering::Relaxed);
    snapshot.holeshot_race_num = 1;
    ride_field_to(&mut snapshot, &[(1, 1.02), (12, 1.98)]);
    snapshot.standings[0].num_laps = 1;
    let gap = 0.04 * track_length;
    snapshot.riders[0].x = 0.0;
    snapshot.riders[0].z = 0.0;
    snapshot.riders[1].x = -gap;
    snapshot.riders[1].z = 0.0;
    snapshot.local_x = -gap;
    snapshot.local_z = 0.0;
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(live_position(1), 1, "lap 1 at 0.02 on {track_length} m");
    assert_eq!(live_position(12), 2, "lap 0 at 0.98 on {track_length} m");
}

/// A same-lap coordinate a lap high, without the travel to match, does not pass a bike
/// the fraction and the world already have ahead.
fn a_jumped_coordinate_does_not_pass(track_length: f32) {
    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = track_length;
    snapshot.sf_meters = 0.0;
    snapshot.current_lap = 1;
    snapshot.standing_count = 2;
    snapshot.rider_count = 2;
    snapshot.standings[0] = standing(1, 1, 1);
    snapshot.standings[1] = standing(12, 2, 1);
    snapshot.riders[0] = rider(1, 0.0, 0.0, 0.10);
    snapshot.riders[1] = rider(12, 20.0, 0.0, 0.10);
    snapshot.local_track_pos = 0.10;
    snapshot.local_x = 20.0;
    IN_GATE.store(1, Ordering::Relaxed);
    let _ = RaceStore::tick(&snapshot);
    IN_GATE.store(0, Ordering::Relaxed);
    snapshot.holeshot_race_num = 1;
    ride_field_to(&mut snapshot, &[(1, 0.90), (12, 0.83)]);
    // One jump across the round boundary adds a lap the travel does not record.
    // Holding the new fraction accepts it. Riding back to 0.83 keeps that lap.
    place_on_track(&mut snapshot, 12, 0.33);
    place_on_track(&mut snapshot, 12, 0.33);
    ride_field_to(&mut snapshot, &[(12, 0.83)]);
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(live_position(1), 1, "lap 1 at 0.90 on {track_length} m");
    assert_eq!(live_position(12), 2, "lap 1 at 0.83 on {track_length} m");
}

#[test]
fn a_coordinate_ahead_of_the_bike_does_not_take_the_place() {
    let _g = session_lock();
    for &track_length in &[900.0_f32, 2480.0] {
        a_bike_short_of_the_line_stays_behind(track_length);
        a_jumped_coordinate_does_not_pass(track_length);
    }
}

/// A lap-down coordinate a lap high, without the travel since the last counted lap,
/// stays behind a higher-lap bike at nearly the same fraction. The bodies sit on that gap.
fn a_jumped_lap_stays_behind(track_length: f32) {
    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = track_length;
    snapshot.sf_meters = 0.0;
    snapshot.current_lap = 1;
    snapshot.standing_count = 2;
    snapshot.rider_count = 2;
    snapshot.standings[0] = standing(1, 1, 0);
    snapshot.standings[1] = standing(12, 2, 0);
    snapshot.riders[0] = rider(1, 0.0, 0.0, 0.10);
    snapshot.riders[1] = rider(12, 20.0, 0.0, 0.10);
    snapshot.local_track_pos = 0.10;
    snapshot.local_x = 20.0;
    IN_GATE.store(1, Ordering::Relaxed);
    let _ = RaceStore::tick(&snapshot);
    IN_GATE.store(0, Ordering::Relaxed);
    snapshot.holeshot_race_num = 1;
    ride_field_to(&mut snapshot, &[(1, 0.76), (12, 0.80)]);
    // Each jump past the half-lap boundary adds a lap the travel does not record.
    place_on_track(&mut snapshot, 12, 0.20);
    place_on_track(&mut snapshot, 12, 0.20);
    ride_field_to(&mut snapshot, &[(12, 0.80)]);
    place_on_track(&mut snapshot, 12, 0.20);
    place_on_track(&mut snapshot, 12, 0.20);
    ride_field_to(&mut snapshot, &[(12, 0.80)]);
    snapshot.standings[0].num_laps = 2;
    snapshot.standings[1].num_laps = 1;
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(live_position(1), 1, "lap 2 at 0.76 on {track_length} m");
    assert_eq!(live_position(12), 2, "lap 1 at 0.80 on {track_length} m");
}

#[test]
fn a_lap_down_coordinate_does_not_take_the_place() {
    let _g = session_lock();
    for &track_length in &[900.0_f32, 2480.0] {
        a_jumped_lap_stays_behind(track_length);
    }
}

/// Walk `race_num` around `laps` of the spline while the body stays where it is.
fn walk_spline_hold_body(snapshot: &mut Snapshot, race_num: i32, laps: f32) {
    let count = snapshot.rider_count as usize;
    let held = snapshot.riders[..count]
        .iter()
        .find(|rider| rider.race_num == race_num)
        .map(|rider| (rider.x, rider.z, rider.track_pos))
        .unwrap_or((0.0, 0.0, 0.0));
    let steps = 50;
    for step in 1..=steps {
        let pos = (held.2 + laps * step as f32 / steps as f32).rem_euclid(1.0);
        if let Some(rider) = snapshot.riders[..count]
            .iter_mut()
            .find(|rider| rider.race_num == race_num)
        {
            rider.track_pos = pos;
            rider.x = held.0;
            rider.z = held.1;
            if race_num == snapshot.focus_race_num {
                snapshot.local_track_pos = pos;
                snapshot.local_x = held.0;
                snapshot.local_z = held.1;
            }
        }
        let _ = RaceStore::tick(snapshot);
    }
}

/// Same lap. One bike is 0.07 of the lap behind and the world gap matches. A spline
/// lap the body did not ride does not put them ahead.
fn a_spline_lap_the_body_did_not_ride_stays_behind(track_length: f32) {
    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = track_length;
    snapshot.sf_meters = 0.0;
    snapshot.current_lap = 1;
    snapshot.standing_count = 2;
    snapshot.rider_count = 2;
    snapshot.standings[0] = standing(1, 1, 1);
    snapshot.standings[1] = standing(12, 2, 1);
    snapshot.riders[0] = rider(1, 0.0, 0.0, 0.10);
    snapshot.riders[1] = rider(12, 20.0, 0.0, 0.10);
    snapshot.local_track_pos = 0.10;
    snapshot.local_x = 20.0;
    IN_GATE.store(1, Ordering::Relaxed);
    let _ = RaceStore::tick(&snapshot);
    IN_GATE.store(0, Ordering::Relaxed);
    snapshot.holeshot_race_num = 1;
    ride_field_to(&mut snapshot, &[(1, 0.87), (12, 0.80)]);
    walk_spline_hold_body(&mut snapshot, 12, 1.0);
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(live_position(1), 1, "lap 1 at 0.87 on {track_length} m");
    assert_eq!(live_position(12), 2, "lap 1 at 0.80 on {track_length} m");
}

#[test]
fn a_body_that_did_not_ride_the_lap_stays_behind() {
    let _g = session_lock();
    for &track_length in &[900.0_f32, 2480.0] {
        a_spline_lap_the_body_did_not_ride_stays_behind(track_length);
    }
}

/// `track_pos` is measured from the centerline origin, not the line. Without a known
/// start/finish, riders this far apart are not a pass.
#[test]
fn half_a_lap_apart_is_not_a_pass() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    pass_the_leader(&mut s, 400.0);
    let field = RaceStore::tick(&s).field;
    assert_eq!(field.rows[0].standing.race_num, 1);
}

/// Once the line is known, a lead anywhere on the lap is the place.
#[test]
fn a_known_line_counts_a_lead_anywhere_on_the_lap() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.sf_meters = s.track_length;
    pass_the_leader(&mut s, 400.0);
    let field = RaceStore::tick(&s).field;
    assert_eq!(field.rows[0].standing.race_num, 12);
}

/// Straight off a start: three riders scored ahead of you went down in the first turn and
/// are now 500 m back. The game still has the gate order, and they are too far away for
/// `track_pos` alone, but you are P5 on track.
#[test]
fn riders_down_on_the_start_do_not_hold_you_back() {
    let _g = session_lock();
    reset_session();
    let mut s = gate_field(&[1, 2, 3, 4, 5, 6, 7, 12]);
    let mut targets: Vec<(i32, f32)> = [1, 2, 3, 4].iter().map(|&n| (n, 0.70)).collect();
    targets.extend([5, 6, 7].iter().map(|&n| (n, 0.12)));
    targets.push((12, 0.65));
    ride_field_to(&mut s, &targets);
    assert_eq!(live_position(12), 5);
    assert_eq!(live_position(5), 6);
    assert_eq!(live_position(4), 4, "the clean riders ahead keep their places");
}

/// Same start pile-up, but the HUD never saw the gate (`IN_GATE` never latched). Cold-arm
/// on race go must still score far pairs before anyone crosses the line.
#[test]
fn missed_gate_still_passes_riders_down_on_the_start() {
    let _g = session_lock();
    reset_session();
    let nums = [1, 2, 3, 4, 5, 6, 7, 12];
    let mut s = live_snap();
    s.standing_count = nums.len() as i32;
    s.rider_count = nums.len() as i32;
    s.current_lap = 1;
    s.sf_meters = 0.0;
    for (i, &num) in nums.iter().enumerate() {
        s.standings[i] = standing(num, i as i32 + 1, 0);
        s.riders[i] = rider(num, i as f32, 0.0, 0.10);
    }
    s.local_track_pos = 0.10;
    assert_eq!(IN_GATE.load(Ordering::Relaxed), 0);
    let mut targets: Vec<(i32, f32)> = [1, 2, 3, 4].iter().map(|&n| (n, 0.70)).collect();
    targets.extend([5, 6, 7].iter().map(|&n| (n, 0.12)));
    targets.push((12, 0.65));
    ride_field_to(&mut s, &targets);
    assert_eq!(live_position(12), 5);
    assert_eq!(live_position(5), 6);
    assert_eq!(live_position(4), 4, "the clean riders ahead keep their places");
}

/// A brief drop from `riders[]` after the gate must not disarm the tracker for the rest
/// of lap 1: once they reappear stalled, a pass still shows.
#[test]
fn track_pos_flicker_after_gate_does_not_block_a_start_pass() {
    let _g = session_lock();
    reset_session();
    let mut s = gate_field(&[1, 2, 3, 4, 5, 6, 7, 12]);
    let count = s.rider_count as usize;
    if let Some(r) = s.riders[..count].iter_mut().find(|r| r.race_num == 5) {
        r.track_pos = -1.0;
        r.crashed = 1;
    }
    s.standings
        .iter_mut()
        .find(|st| st.race_num == 5)
        .unwrap()
        .crashed = 1;
    let _ = RaceStore::tick(&s);
    // Reappear where they left so resume does not invent metres they never rode.
    if let Some(r) = s.riders[..count].iter_mut().find(|r| r.race_num == 5) {
        r.track_pos = 0.10;
    }
    let _ = RaceStore::tick(&s);
    let mut targets: Vec<(i32, f32)> = [1, 2, 3, 4].iter().map(|&n| (n, 0.70)).collect();
    targets.extend([5, 6, 7].iter().map(|&n| (n, 0.12)));
    targets.push((12, 0.65));
    ride_field_to(&mut s, &targets);
    assert_eq!(live_position(12), 5);
    assert_eq!(live_position(5), 6);
}

struct ResetSplits;

impl Drop for ResetSplits {
    fn drop(&mut self) {
        crate::sector::set_split_fracs([0.0, 0.0]);
    }
}

/// Lap 1, line unknown, both splits saved: a rider past S1 outranks one 500 m back in S1.
#[test]
fn lap_one_past_s1_beats_a_rider_still_in_s1() {
    let _g = session_lock();
    let _splits = ResetSplits;
    reset_session();
    crate::sector::set_split_fracs([0.40, 0.70]);
    let mut s = live_snap();
    s.track_length = 2000.0;
    s.sf_meters = 0.0;
    s.current_lap = 1;
    s.standing_count = 2;
    s.rider_count = 2;
    s.standings[0] = standing(1, 1, 0);
    s.standings[1] = standing(12, 2, 0);
    s.riders[0] = rider(1, 0.15 * s.track_length, 0.0, 0.15);
    s.riders[1] = rider(12, 0.42 * s.track_length, 0.0, 0.42);
    s.local_track_pos = 0.42;
    s.local_x = s.riders[1].x;
    s.local_z = 0.0;
    let _ = RaceStore::tick(&s);
    assert_eq!(live_position(12), 1, "40 m past S1");
    assert_eq!(live_position(1), 2, "500 m before S1");
}

/// Inside S1 the order still follows track position, not the gate order.
#[test]
fn lap_one_pass_inside_s1_still_swaps() {
    let _g = session_lock();
    let _splits = ResetSplits;
    reset_session();
    crate::sector::set_split_fracs([0.40, 0.70]);
    let mut s = live_snap();
    s.track_length = 2000.0;
    s.sf_meters = 0.0;
    s.current_lap = 1;
    s.standing_count = 2;
    s.rider_count = 2;
    s.standings[0] = standing(1, 1, 0);
    s.standings[1] = standing(12, 2, 0);
    s.riders[0] = rider(1, 0.385 * s.track_length, 0.0, 0.385);
    s.riders[1] = rider(12, 0.395 * s.track_length, 0.0, 0.395);
    s.local_track_pos = 0.395;
    s.local_x = s.riders[1].x;
    s.local_z = 0.0;
    let _ = RaceStore::tick(&s);
    assert_eq!(live_position(12), 1, "20 m closer to S1");
    assert_eq!(live_position(1), 2);
}

/// After the drop, a published S1 split does not outrank a rider still on the same
/// stretch. Place follows how far they have ridden from the grid.
#[test]
fn a_sector_split_does_not_beat_the_open_lap_grid() {
    let _g = session_lock();
    let _splits = ResetSplits;
    reset_session();
    crate::sector::set_split_fracs([0.0, 0.0]);
    let mut s = gate_field(&[1, 12]);
    s.standings[1].sector_gate = 1;
    let _ = RaceStore::tick(&s);
    assert_eq!(live_position(1), 0, "still in the slot");
    assert_eq!(live_position(12), 0, "an S1 split is not a place");
    ride_field_to(&mut s, &[(1, 0.55), (12, 0.10)]);
    assert_eq!(live_position(1), 1, "further up the lap stays ahead");
    assert_eq!(live_position(12), 2);
    assert_eq!(s.standings[1].num_laps, 0);
}

/// A completed lap keeps the tracker. Sector fractions must not reorder it.
#[test]
fn a_finished_lap_ignores_sector_geometry() {
    let _g = session_lock();
    let _splits = ResetSplits;
    reset_session();
    crate::sector::set_split_fracs([0.40, 0.70]);
    let mut s = live_snap();
    s.track_length = 2000.0;
    s.sf_meters = 0.0;
    s.current_lap = 2;
    s.standing_count = 2;
    s.rider_count = 2;
    s.standings[0] = standing(1, 1, 1);
    s.standings[1] = standing(12, 2, 1);
    s.riders[0] = rider(1, 1.0, 0.0, 0.15);
    s.riders[1] = rider(12, 2.0, 0.0, 0.42);
    s.local_track_pos = 0.42;
    let _ = RaceStore::tick(&s);
    assert_eq!(live_position(1), 1, "sector geometry is lap 1 only");
    assert_eq!(live_position(12), 2);
}

/// Just past the learned S2, before that split publishes, still beats a rider in S1.
#[test]
fn lap_one_just_past_s2_stays_ahead_before_the_gate() {
    let _g = session_lock();
    let _splits = ResetSplits;
    reset_session();
    crate::sector::set_split_fracs([0.40, 0.70]);
    let mut s = live_snap();
    s.track_length = 2000.0;
    s.sf_meters = 0.0;
    s.current_lap = 1;
    s.standing_count = 2;
    s.rider_count = 2;
    s.standings[0] = standing(1, 1, 0);
    s.standings[1] = standing(12, 2, 0);
    s.standings[1].sector_gate = 1;
    s.riders[0] = rider(1, 0.55 * s.track_length, 0.0, 0.55);
    s.riders[1] = rider(12, 0.71 * s.track_length, 0.0, 0.71);
    s.local_track_pos = 0.71;
    s.local_x = s.riders[1].x;
    s.local_z = 0.0;
    let _ = RaceStore::tick(&s);
    assert_eq!(live_position(12), 1, "just past S2, gate still 1");
    assert_eq!(live_position(1), 2, "still between S1 and S2");
}

/// Spring a Ding: the return arc is cut in half, so a rider 40 m further up the
/// lap must not lose the place to someone on the other side of that cut.
#[test]
fn the_open_lap_bisector_does_not_reorder_a_close_pair() {
    let _g = session_lock();
    let _splits = ResetSplits;
    reset_session();
    crate::sector::set_split_fracs([0.40, 0.70]);
    let mut snapshot = live_snap();
    snapshot.track_length = 2000.0;
    snapshot.sf_meters = 0.0;
    snapshot.current_lap = 1;
    snapshot.standing_count = 2;
    snapshot.rider_count = 2;
    snapshot.standings[0] = standing(1, 1, 0);
    snapshot.standings[1] = standing(12, 2, 0);
    snapshot.riders[0] = rider(1, 0.04 * 2000.0, 0.0, 0.04);
    snapshot.riders[1] = rider(12, 0.06 * 2000.0, 0.0, 0.06);
    snapshot.local_track_pos = 0.06;
    snapshot.local_x = snapshot.riders[1].x;
    snapshot.local_z = 0.0;
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(
        live_position(12),
        1,
        "40 m further up the lap stays ahead of the bisector"
    );
    assert_eq!(live_position(1), 2);

    snapshot.riders[0].track_pos = 0.05;
    snapshot.riders[0].x = 0.05 * 2000.0;
    snapshot.riders[1].track_pos = 0.07;
    snapshot.riders[1].x = 0.07 * 2000.0;
    snapshot.local_track_pos = 0.07;
    snapshot.local_x = snapshot.riders[1].x;
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(
        live_position(12),
        1,
        "nudged forward, the fraction still orders them"
    );
    assert_eq!(live_position(1), 2);
}

/// Gate 0, just before S1, stays behind a rider who has reached S1.
#[test]
fn lap_one_before_s1_stays_behind_a_rider_past_s1() {
    let _g = session_lock();
    let _splits = ResetSplits;
    reset_session();
    crate::sector::set_split_fracs([0.40, 0.70]);
    let mut s = live_snap();
    s.track_length = 2000.0;
    s.sf_meters = 0.0;
    s.current_lap = 1;
    s.standing_count = 2;
    s.rider_count = 2;
    s.standings[0] = standing(1, 1, 0);
    s.standings[1] = standing(12, 2, 0);
    s.riders[0] = rider(1, 0.39 * s.track_length, 0.0, 0.39);
    s.riders[1] = rider(12, 0.42 * s.track_length, 0.0, 0.42);
    s.local_track_pos = 0.42;
    s.local_x = s.riders[1].x;
    s.local_z = 0.0;
    let _ = RaceStore::tick(&s);
    assert_eq!(live_position(12), 1, "40 m past S1");
    assert_eq!(live_position(1), 2, "gate 0, just before S1");
}

/// A sector gate after the first lap must not zero the leader's metres.
#[test]
fn a_sector_gate_after_the_line_does_not_drop_the_leader() {
    let _g = session_lock();
    let _splits = ResetSplits;
    reset_session();
    crate::sector::set_split_fracs([0.40, 0.70]);
    let mut s = gate_field(&[1, 12]);
    for standing in &mut s.standings[..2] {
        standing.num_laps = 1;
    }
    let _ = RaceStore::tick(&s);
    ride_field_to(&mut s, &[(1, 0.20), (12, 0.21)]);
    assert_eq!(live_position(12), 1, "10 m up the lap");
    s.standings
        .iter_mut()
        .find(|st| st.race_num == 12)
        .unwrap()
        .sector_gate = 1;
    let _ = RaceStore::tick(&s);
    assert_eq!(live_position(12), 1, "S1 gate does not rebase a finished lap");
    assert_eq!(live_position(1), 2);
}

/// A rider missing from `riders[]` keeps their scored slot, but the pass above them still
/// shows.
#[test]
fn a_rider_off_the_radar_does_not_block_a_pass() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.standing_count = 3;
    s.standings[1] = standing(5, 2, 5);
    s.standings[2] = standing(12, 3, 5);
    pass_the_leader(&mut s, 6.0);
    let field = RaceStore::tick(&s).field;
    assert_eq!(live_position(12), 1);
    assert_eq!(live_position(5), 2, "no track position, stays where scored");
    assert_eq!(field.rows[2].standing.race_num, 1);
}

/// Joined mid-race, we cannot tell where a far-apart pair's laps started. Once both have
/// crossed the line in front of us, we can.
#[test]
fn a_far_pass_shows_once_both_riders_have_crossed_the_line() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.riders[0].track_pos = 0.97;
    s.riders[1].track_pos = 0.96;
    s.local_track_pos = 0.96;
    let _ = RaceStore::tick(&s);
    ride_field_to(&mut s, &[(1, 1.02), (12, 1.01)]);
    s.standings[0].num_laps = 6;
    s.standings[1].num_laps = 6;
    // Targets are on this lap (0..1): after the wrap, ride_field_to already rem_euclid'd
    // positions, so 1.12/1.71 would integrate an extra full lap and trip the runaway cap.
    ride_field_to(&mut s, &[(1, 0.12), (12, 0.71)]);
    assert_eq!(live_position(12), 1, "600 m up the lap on the leader");
}

/// A rider who actually rides more than a lap further, while `num_laps` still lags,
/// is ahead. That place follows the bike. The lap bump does not throw them out.
#[test]
fn a_real_extra_lap_leads_before_num_laps_bumps() {
    let _g = session_lock();
    reset_session();
    let mut s = gate_field(&[1, 2, 3, 4, 5, 6, 12]);
    s.sf_meters = s.track_length;
    for standing in &mut s.standings[..7] {
        standing.num_laps = 5;
    }
    ride_field_to(
        &mut s,
        &[
            (1, 0.70),
            (2, 0.65),
            (3, 0.60),
            (4, 0.55),
            (5, 0.50),
            (6, 0.45),
            (12, 0.40),
        ],
    );
    assert_eq!(live_position(12), 7);
    let start = s
        .riders
        .iter()
        .find(|r| r.race_num == 12)
        .map(|r| r.track_pos)
        .unwrap_or(0.40);
    ride_field_to(&mut s, &[(12, start + 1.6)]);
    assert_eq!(
        live_position(12),
        1,
        "a real extra lap of track is the place, even before num_laps bumps"
    );
    s.standings
        .iter_mut()
        .find(|st| st.race_num == 12)
        .unwrap()
        .num_laps = 6;
    let _ = RaceStore::tick(&s);
    let place = live_position(12);
    assert!(
        (1..=7).contains(&place),
        "after a real lap bump place stays in the field, got {place}"
    );
}

/// Leader crosses the wrap / rides past one lap of travel while `num_laps` lags. Must not
/// fall to mid-pack via S/F metres near 0; stay P1 (armed clamp or corrupt pin to game).
#[test]
fn leader_holds_p1_when_lap_metres_wrap_before_num_laps() {
    let _g = session_lock();
    reset_session();
    let mut s = gate_field(&[12, 1, 2, 3, 4, 5, 6, 7]);
    s.sf_meters = s.track_length;
    for standing in &mut s.standings[..8] {
        standing.num_laps = 5;
    }
    ride_field_to(
        &mut s,
        &[
            (12, 0.70),
            (1, 0.55),
            (2, 0.50),
            (3, 0.45),
            (4, 0.40),
            (5, 0.35),
            (6, 0.30),
            (7, 0.25),
        ],
    );
    assert_eq!(live_position(12), 1);
    let start = s
        .riders
        .iter()
        .find(|r| r.race_num == 12)
        .map(|r| r.track_pos)
        .unwrap_or(0.70);
    ride_field_to(&mut s, &[(12, start + 1.2)]);
    assert_eq!(
        live_position(12),
        1,
        "wrap / overshoot without a num_laps bump must not drop the leader"
    );
}

/// Session best is 92 s round 1000 m, so 5 s of penalty is about 54 m of track.
#[test]
fn within_the_leaders_penalty_you_are_ahead() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.standings[0].penalty_ms = 5_000;
    pass_the_leader(&mut s, -40.0);
    let _ = RaceStore::tick(&s);
    assert_eq!(live_position(12), 1);
}

#[test]
fn beyond_the_leaders_penalty_they_keep_the_place() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.standings[0].penalty_ms = 5_000;
    pass_the_leader(&mut s, -80.0);
    let _ = RaceStore::tick(&s);
    assert_eq!(live_position(12), 2);
}

/// Getting by on track with a penalty is not the place until the penalty is cleared.
#[test]
fn a_penalised_pass_has_to_clear_the_penalty() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.standings[1].penalty_ms = 5_000;
    pass_the_leader(&mut s, 6.0);
    let _ = RaceStore::tick(&s);
    assert_eq!(live_position(12), 2);
    pass_the_leader(&mut s, 60.0);
    let _ = RaceStore::tick(&s);
    assert_eq!(live_position(12), 1);
}

/// P1 and P2 are clear. P3 has 10 s and is ~7 s up the road, P4 has 5 s and is ~1 s up
/// the road. Both penalties are yours to take, so you are P3.
#[test]
fn every_penalty_counts_against_where_they_are() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.sf_meters = s.track_length;
    s.standing_count = 5;
    s.rider_count = 5;
    s.standings[0] = standing(1, 1, 5);
    s.standings[0].best_lap_ms = 92_000;
    s.standings[1] = standing(2, 2, 5);
    s.standings[1].best_lap_ms = 92_000;
    s.standings[2] = standing(3, 3, 5);
    s.standings[2].best_lap_ms = 92_000;
    s.standings[2].penalty_ms = 10_000;
    s.standings[3] = standing(4, 4, 5);
    s.standings[3].best_lap_ms = 92_000;
    s.standings[3].penalty_ms = 5_000;
    s.standings[4] = standing(12, 5, 5);
    s.standings[4].best_lap_ms = 93_500;
    // ~10.87 m/s. 7 s ≈ 76 m, 1 s ≈ 11 m. The two clean riders are a clear lap ahead
    // on track position within the same lap count: 200 m and 180 m up the road.
    s.riders[0] = rider(1, 0.70 * s.track_length, 0.0, 0.70);
    s.riders[1] = rider(2, 0.68 * s.track_length, 0.0, 0.68);
    s.riders[2] = rider(3, 0.576 * s.track_length, 0.0, 0.576);
    s.riders[3] = rider(4, 0.511 * s.track_length, 0.0, 0.511);
    s.riders[4] = rider(12, 0.500 * s.track_length, 0.0, 0.500);
    s.local_track_pos = 0.500;
    s.local_x = s.riders[4].x;
    s.local_z = 0.0;
    let _ = RaceStore::tick(&s);
    assert_eq!(live_position(12), 3);
    assert_eq!(live_position(3), 4);
    assert_eq!(live_position(4), 5);
    assert_eq!(live_position(1), 1);
}

/// 30 s is most of a lap at this pace. Sitting 50 m up the road does not keep P1.
#[test]
fn a_large_penalty_drops_through_the_pack() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.sf_meters = s.track_length;
    s.standing_count = 4;
    s.rider_count = 4;
    s.standings[0] = standing(1, 1, 5);
    s.standings[0].best_lap_ms = 92_000;
    s.standings[0].penalty_ms = 30_000;
    s.standings[1] = standing(2, 2, 5);
    s.standings[1].best_lap_ms = 92_000;
    s.standings[2] = standing(3, 3, 5);
    s.standings[2].best_lap_ms = 92_000;
    s.standings[3] = standing(12, 4, 5);
    s.standings[3].best_lap_ms = 93_500;
    s.riders[0] = rider(1, 0.55 * s.track_length, 0.0, 0.55);
    s.riders[1] = rider(2, 0.52 * s.track_length, 0.0, 0.52);
    s.riders[2] = rider(3, 0.51 * s.track_length, 0.0, 0.51);
    s.riders[3] = rider(12, 0.50 * s.track_length, 0.0, 0.50);
    s.local_track_pos = 0.50;
    s.local_x = s.riders[3].x;
    s.local_z = 0.0;
    let _ = RaceStore::tick(&s);
    assert_eq!(live_position(1), 4);
    assert_eq!(live_position(12), 3);
}

#[test]
fn penalty_place_delta_marks_ahead_and_behind() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.sf_meters = s.track_length;
    s.standing_count = 4;
    s.rider_count = 4;
    s.standings[0] = standing(1, 1, 5);
    s.standings[0].best_lap_ms = 92_000;
    s.standings[0].penalty_ms = 30_000;
    s.standings[1] = standing(2, 2, 5);
    s.standings[1].best_lap_ms = 92_000;
    s.standings[2] = standing(3, 3, 5);
    s.standings[2].best_lap_ms = 92_000;
    s.standings[3] = standing(12, 4, 5);
    s.standings[3].best_lap_ms = 93_500;
    s.riders[0] = rider(1, 0.55 * s.track_length, 0.0, 0.55);
    s.riders[1] = rider(2, 0.52 * s.track_length, 0.0, 0.52);
    s.riders[2] = rider(3, 0.51 * s.track_length, 0.0, 0.51);
    s.riders[3] = rider(12, 0.50 * s.track_length, 0.0, 0.50);
    s.local_track_pos = 0.50;
    s.local_x = s.riders[3].x;
    s.local_z = 0.0;
    let _ = RaceStore::tick(&s);
    assert_eq!(track_position(1), 1);
    assert_eq!(live_position(1), 4);
    assert_eq!(penalty_place_delta(1), Some(-3), "penalised leader is behind on-track place");
    assert_eq!(track_position(12), 4);
    assert_eq!(live_position(12), 3);
    assert_eq!(penalty_place_delta(12), Some(1), "displaced clean rider is ahead of on-track");
}

#[test]
fn penalty_place_delta_is_none_without_a_shift() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    pass_the_leader(&mut s, 6.0);
    let _ = RaceStore::tick(&s);
    assert_eq!(live_position(12), 1);
    assert_eq!(penalty_place_delta(12), None);
    assert_eq!(penalty_place_delta(1), None);

    s.standings[0].penalty_ms = 5_000;
    s.standings[1].penalty_ms = 5_000;
    let _ = RaceStore::tick(&s);
    assert_eq!(live_position(12), 1);
    assert_eq!(
        penalty_place_delta(12),
        None,
        "equal penalties that do not change order"
    );

    s.on_track = 0;
    let _ = RaceStore::tick(&s);
    assert_eq!(penalty_place_delta(12), None, "live order inactive");
}

#[test]
fn equal_penalties_cancel_out() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.standings[0].penalty_ms = 5_000;
    s.standings[1].penalty_ms = 5_000;
    pass_the_leader(&mut s, 6.0);
    let _ = RaceStore::tick(&s);
    assert_eq!(live_position(12), 1);
    pass_the_leader(&mut s, -6.0);
    let _ = RaceStore::tick(&s);
    assert_eq!(live_position(12), 2);
}

/// Washington: the bike 12 m ahead gets `num_laps` one sample early. You stay
/// behind them, and catching up does not put you ahead while they are still
/// 12 m up the stripe. The gap is metres, so 800 m and 2000 m agree.
fn a_stripe_count_does_not_leapfrog(track_length: f32) {
    reset_session();
    let gap = 12.0;
    let ahead = gap / track_length;
    let mut snapshot = live_snap();
    snapshot.track_length = track_length;
    snapshot.sf_meters = 0.0;
    snapshot.current_lap = 1;
    snapshot.standing_count = 2;
    snapshot.rider_count = 2;
    snapshot.standings[0] = standing(1, 1, 0);
    snapshot.standings[1] = standing(12, 2, 0);
    snapshot.riders[0] = rider(1, 0.0, 0.0, 0.90);
    snapshot.riders[1] = rider(12, gap, 0.0, 0.90);
    snapshot.local_track_pos = 0.90;
    snapshot.local_x = gap;
    IN_GATE.store(1, Ordering::Relaxed);
    let _ = RaceStore::tick(&snapshot);
    IN_GATE.store(0, Ordering::Relaxed);
    snapshot.holeshot_race_num = 1;
    ride_field_to(&mut snapshot, &[(1, 1.0 + ahead), (12, 1.0 - ahead)]);
    snapshot.standings[0].num_laps = 1;
    snapshot.riders[0].x = gap;
    snapshot.riders[0].z = 0.0;
    snapshot.riders[1].x = 0.0;
    snapshot.riders[1].z = 0.0;
    snapshot.local_x = 0.0;
    snapshot.local_z = 0.0;
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(
        live_position(1),
        1,
        "12 m ahead keeps the place on {track_length} m"
    );
    assert_eq!(
        live_position(12),
        2,
        "not a lap down on {track_length} m"
    );
    snapshot.standings[1].num_laps = 1;
    snapshot.current_lap = 2;
    let you = (2.0 / track_length).rem_euclid(1.0);
    let them = (gap + 2.0) / track_length;
    snapshot.riders[1].track_pos = you;
    snapshot.riders[1].x = 2.0;
    snapshot.local_track_pos = you;
    snapshot.local_x = 2.0;
    snapshot.riders[0].track_pos = them;
    snapshot.riders[0].x = gap + 2.0;
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(
        live_position(1),
        1,
        "catch-up does not leapfrog on {track_length} m"
    );
    assert_eq!(
        live_position(12),
        2,
        "still 12 m behind on {track_length} m"
    );
}

#[test]
fn a_stripe_count_does_not_leapfrog_on_a_short_lap_and_a_long_one() {
    let _g = session_lock();
    a_stripe_count_does_not_leapfrog(800.0);
    a_stripe_count_does_not_leapfrog(2000.0);
}

/// The fraction jumps 400 m and holds, while the body only moves 50 m. That
/// second sample is not the place. The same metres on 800 m and 2000 m.
fn a_held_fraction_the_body_did_not_ride(track_length: f32) {
    reset_session();
    let mut snapshot = live_snap();
    snapshot.track_length = track_length;
    snapshot.sf_meters = track_length;
    snapshot.riders[0].track_pos = 0.40;
    snapshot.riders[0].x = 0.40 * track_length;
    snapshot.riders[0].z = 0.0;
    snapshot.riders[1].track_pos = 0.30;
    snapshot.riders[1].x = 0.30 * track_length;
    snapshot.riders[1].z = 0.0;
    snapshot.local_track_pos = 0.30;
    snapshot.local_x = snapshot.riders[1].x;
    snapshot.local_z = 0.0;
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(live_position(12), 2, "behind on {track_length} m");

    let jumped = (0.30 + 400.0 / track_length).rem_euclid(1.0);
    snapshot.riders[1].track_pos = jumped;
    snapshot.riders[1].x += 50.0;
    snapshot.local_track_pos = jumped;
    snapshot.local_x = snapshot.riders[1].x;
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(
        live_position(12),
        2,
        "the spike does not pass on {track_length} m"
    );
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(
        live_position(12),
        2,
        "a second sample the body did not ride is not the place on {track_length} m"
    );
}

#[test]
fn a_held_fraction_the_body_did_not_ride_on_a_short_lap_and_a_long_one() {
    let _g = session_lock();
    a_held_fraction_the_body_did_not_ride(800.0);
    a_held_fraction_the_body_did_not_ride(2000.0);
}

/// A 300 m jump in one tick is a reset or a teleport, not riding.
#[test]
fn a_teleport_is_not_distance_covered() {
    let _g = session_lock();
    reset_session();
    let mut s = gate_field(&[1, 12]);
    ride_field_to(&mut s, &[(1, 0.20), (12, 0.19)]);
    s.riders[1].track_pos = 0.49;
    s.riders[1].x = 0.49 * s.track_length;
    s.riders[1].z = 0.0;
    s.local_track_pos = 0.49;
    s.local_x = s.riders[1].x;
    s.local_z = 0.0;
    let _ = RaceStore::tick(&s);
    assert_eq!(live_position(12), 2);
    let _ = RaceStore::tick(&s);
    assert_eq!(live_position(12), 1, "a jump that stays is where they are");
}

/// Briarcliff: #108's lap fraction jumped ~600 m and the bike did not. That is not a pass.
#[test]
fn a_spline_jump_the_bike_does_not_follow_is_not_a_pass() {
    let _g = session_lock();
    reset_session();
    let mut snapshot = gate_field(&[1, 12]);
    ride_field_to(&mut snapshot, &[(1, 0.40), (12, 0.30)]);
    assert_eq!(live_position(1), 1);
    assert_eq!(live_position(12), 2);
    snapshot.riders[1].track_pos = 0.70;
    snapshot.local_track_pos = 0.70;
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(live_position(12), 2, "the spike tick does not take the place");
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(
        live_position(12),
        2,
        "a spline jump the body did not follow is not where they are"
    );
}

/// Millville, no gate snapshot: a ~400 m spline jump the bike does not follow
/// still does not take the place, on the spike tick or the one after.
#[test]
fn a_spline_jump_without_a_gate_is_not_a_pass() {
    let _g = session_lock();
    reset_session();
    let mut snapshot = live_snap();
    snapshot.sf_meters = snapshot.track_length;
    snapshot.riders[0].track_pos = 0.40;
    snapshot.riders[0].x = 0.40 * snapshot.track_length;
    snapshot.riders[0].z = 0.0;
    snapshot.riders[1].track_pos = 0.30;
    snapshot.riders[1].x = 0.30 * snapshot.track_length;
    snapshot.riders[1].z = 0.0;
    snapshot.local_track_pos = 0.30;
    snapshot.local_x = snapshot.riders[1].x;
    snapshot.local_z = 0.0;
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(live_position(12), 2);

    snapshot.riders[1].track_pos = 0.70;
    snapshot.local_track_pos = 0.70;
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(live_position(12), 2, "the spike tick does not take the place");
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(
        live_position(12),
        2,
        "without a gate, a spline jump the body did not follow is still not a pass"
    );
}

/// Later lap: a stored coordinate sits a sector ahead, and the other bike is ~300 m
/// further up the raw fraction with the bodies that far apart. The raw gap is the place.
#[test]
fn a_confirmed_gap_beats_a_stuck_coordinate() {
    let _g = session_lock();
    reset_session();
    let mut snapshot = live_snap();
    let length = 1000.0;
    snapshot.track_length = length;
    snapshot.sf_meters = length;
    snapshot.riders[0].track_pos = 0.90;
    snapshot.riders[0].x = 0.0;
    snapshot.riders[0].z = 0.0;
    snapshot.riders[1].track_pos = 0.50;
    snapshot.riders[1].x = 300.0;
    snapshot.riders[1].z = 0.0;
    snapshot.local_track_pos = 0.50;
    snapshot.local_x = 300.0;
    snapshot.local_z = 0.0;
    let _ = RaceStore::tick(&snapshot);
    assert_eq!(live_position(1), 1, "the first sample is a real lead");

    snapshot.riders[0].track_pos = 0.20;
    let field = RaceStore::tick(&snapshot).field;
    assert_eq!(
        live_position(12),
        1,
        "300 m up the raw fraction, with the bodies 300 m apart, takes the place"
    );
    assert_eq!(live_position(1), 2);
    assert_eq!(field.rows[0].standing.race_num, 12);
}

/// The clock owns every session latch and runs first in the tick; the field only reads what
/// it left. Asking those heuristics again arms them from mid-tick state, which moved the
/// lap counter and the flags.
#[test]
fn the_live_order_does_not_arm_the_clock_state() {
    let _g = session_lock();
    for (laps, length) in [(6, 0), (3, 0), (2, 0), (0, 8), (2, 8)] {
        let quiet = clock_trace(laps, length, false);
        let asked = clock_trace(laps, length, true);
        assert_eq!(quiet, asked, "laps={laps} length={length}");
    }
}

/// Straight off a 5:00 + 2 trace: mid-moto the game republished a five second start board
/// for seven frames, and the way back up to 4:42 read as the clock having run out, so the
/// dash sat on `0 / 2` extras for the rest of the race.
#[test]
fn a_start_board_republished_mid_race_is_not_the_clock_running_out() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 5;
    s.session_laps = 2;
    s.local_speed = 12.5;
    s.current_lap = 2;
    // Under way with 4:44 to run.
    for clock in [284_681, 284_591, 284_471, 284_381, 284_291, 283_991] {
        s.session_time_ms = clock;
        let _ = RaceStore::tick(&s);
    }
    let running = RaceStore::tick(&s);
    assert_eq!(running.clock.banner.1, "04:43");
    // A five-second board is inside the last 90 s, so it still shows. What must not
    // happen is the race ending when the real clock comes back.
    s.session_time_ms = 5_000;
    for _ in 0..7 {
        let _board = RaceStore::tick(&s);
    }
    // The real clock comes back where it left off, so no time ran out.
    s.session_time_ms = 282_161;
    let back = RaceStore::tick(&s);
    assert_eq!(back.clock.banner.1, "04:42");
    assert_eq!(
        SESSION_EXPIRED.load(Ordering::Relaxed),
        0,
        "the board read as the clock running out"
    );
    s.session_time_ms = 270_000;
    let later = RaceStore::tick(&s);
    assert_eq!(later.clock.banner.1, "04:30");
    assert_eq!(later.clock.mode, ClockMode::Timed);
    assert_eq!(later.clock.flag, RaceFlag::None);
}

/// MXB Club at the green: the clock stair-stepped 08:00, 07:10, 06:40 … 03:10, then
/// back to 08:00 inside a second. Each step is 30 s or more, which a real clock
/// does not do. The dash stays on the pre-dip time the whole way.
#[test]
fn a_green_flag_stair_step_stays_on_the_race_clock() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 8;
    s.session_laps = 2;
    s.local_speed = 12.5;
    s.current_lap = 2;
    for clock in [450_000, 449_900, 449_800, 449_700, 449_600, 449_400] {
        s.session_time_ms = clock;
        let _ = RaceStore::tick(&s);
    }
    let running = RaceStore::tick(&s);
    let held = running.clock.banner.1.clone();
    assert_eq!(held, "07:29");
    let mut holding = false;
    for clock in [
        400_000, 370_000, 340_000, 310_000, 280_000, 250_000, 220_000, 190_000,
    ] {
        s.session_time_ms = clock;
        let step = RaceStore::tick(&s);
        if holding {
            assert_eq!(
                step.clock.banner.1, held,
                "stair-step {clock} stays on the race clock"
            );
        }
        holding = true;
        assert_eq!(SESSION_EXPIRED.load(Ordering::Relaxed), 0);
    }
    assert!(holding);
    s.session_time_ms = 449_400;
    let back = RaceStore::tick(&s);
    assert_eq!(back.clock.banner.1, held);
    assert_eq!(SESSION_EXPIRED.load(Ordering::Relaxed), 0);
}

/// Leading on track puts the crown on your own dot.
#[test]
fn taking_the_lead_moves_the_crown() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    pass_the_leader(&mut s, 6.0);
    let _ = RaceStore::tick(&s);
    assert_eq!(leader_num(&s), s.focus_race_num);
}

#[test]
fn standings_name_is_clickable() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.show_relative = 0;
    s.show_map = 0;
    let mut cfg = HudConfig::new();
    cfg[WidgetId::Minimap].show = false;
    cfg[WidgetId::Radar].show = false;
    cfg[WidgetId::Dash].show = false;
    cfg[WidgetId::Ticker].show = false;
    cfg[WidgetId::Sys].show = false;
    draw_ok(&s, &cfg);
    let hits = click_rider_hits();
    assert_eq!(hit_nums_by_pos(), vec![1, 12]);
    for h in &hits {
        assert_eq!(
            click_rider_at(h.x + h.w * 0.5, h.y + h.h * 0.5),
            Some(h.race_num)
        );
    }
    assert_eq!(click_rider_at(0.0, 0.0), None);
}

#[test]
fn horizontal_standings_cards_are_clickable() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.show_standings = 0;
    s.show_relative = 0;
    s.show_map = 0;
    let mut cfg = HudConfig::new();
    cfg[WidgetId::Minimap].show = false;
    cfg[WidgetId::Radar].show = false;
    cfg[WidgetId::Dash].show = false;
    cfg[WidgetId::Ticker].show = true;
    cfg[WidgetId::Sys].show = false;
    draw_ok(&s, &cfg);
    let hits = click_rider_hits();
    assert_eq!(hit_nums_by_pos(), vec![1, 12]);
    for h in &hits {
        assert_eq!(
            click_rider_at(h.x + h.w * 0.5, h.y + h.h * 0.5),
            Some(h.race_num)
        );
    }
}

#[test]
fn replay_standings_draw_without_on_track() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.on_track = 0;
    s.has_telemetry = 0;
    s.show_relative = 0;
    s.show_map = 0;
    let mut cfg = HudConfig::new();
    cfg[WidgetId::Minimap].show = false;
    cfg[WidgetId::Radar].show = false;
    cfg[WidgetId::Dash].show = false;
    cfg[WidgetId::Ticker].show = false;
    cfg[WidgetId::Sys].show = false;
    draw_ok(&s, &cfg);
    assert_eq!(hit_nums_by_pos(), vec![1, 12]);
}

#[test]
fn standings_alternating_rows_can_turn_off() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.local_race_num = 1;
    s.focus_race_num = 1;
    s.standing_count = 4;
    s.standings[2] = standing(3, 3, 5);
    s.standings[3] = standing(4, 4, 5);
    s.show_relative = 0;
    s.show_map = 0;
    let mut cfg = HudConfig::new();
    cfg[WidgetId::Standings].show = true;
    cfg[WidgetId::Minimap].show = false;
    cfg[WidgetId::Radar].show = false;
    cfg[WidgetId::Dash].show = false;
    cfg[WidgetId::Ticker].show = false;
    cfg[WidgetId::Sys].show = false;
    let mut striped = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut striped, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
    let hits = click_rider_hits();
    let mut by_y = hits.clone();
    by_y.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap());
    assert_eq!(
        by_y.iter().map(|h| h.race_num).collect::<Vec<_>>(),
        vec![1, 12, 3, 4]
    );
    let stripe_a = sample_px(
        &striped,
        by_y[1].x + by_y[1].w * 0.5,
        by_y[1].y + by_y[1].h * 0.5,
    );
    let stripe_b = sample_px(
        &striped,
        by_y[2].x + by_y[2].w * 0.5,
        by_y[2].y + by_y[2].h * 0.5,
    );
    assert_ne!(stripe_a, stripe_b, "odd/even stripe rows must differ");
    cfg.standings.st_stripe = false;
    let mut flat = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut flat, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
    let flat_a = sample_px(
        &flat,
        by_y[1].x + by_y[1].w * 0.5,
        by_y[1].y + by_y[1].h * 0.5,
    );
    let flat_b = sample_px(
        &flat,
        by_y[2].x + by_y[2].w * 0.5,
        by_y[2].y + by_y[2].h * 0.5,
    );
    assert_eq!(flat_a, flat_b, "flat board rows must match away from focus");
}

#[test]
fn table_plaque_covers_rows_when_saved_box_is_short() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.standing_count = 8;
    s.rider_count = 8;
    s.standings_rows = 12;
    s.relative_count = 4;
    s.standings_rect.h = 0.12;
    s.relative.h = 0.10;
    s.local_race_num = 1;
    s.focus_race_num = 1;
    for i in 2..8 {
        let num = i as i32 + 10;
        s.standings[i] = standing(num, i as i32 + 1, 5);
        s.riders[i] = rider(num, 8.0, 3.0, 0.12 + i as f32 * 0.08);
    }
    s.show_map = 0;
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Standings].show = true;
    cfg[WidgetId::Relative].show = true;
    cfg.standings.st_stripe = false;
    cfg.relative.rel_stripe = false;
    let st = table_layout_rect(&s, &cfg, WidgetId::Standings, s.standings_rect, 1280.0, 720.0);
    assert!(
        st.h > s.standings_rect.h + 0.05,
        "standings plaque must grow with rows, got {} vs box {}",
        st.h,
        s.standings_rect.h
    );
    let rel = table_layout_rect(&s, &cfg, WidgetId::Relative, s.relative, 1280.0, 720.0);
    assert!(
        rel.h > s.relative.h + 0.04,
        "relative plaque must grow with rows, got {} vs box {}",
        rel.h,
        s.relative.h
    );
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut px, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
    let mut by_y = click_rider_hits();
    by_y.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap());
    assert!(by_y.len() >= 8, "standings name hits, got {}", by_y.len());
    let last = by_y.last().unwrap();
    let p = sample_px(&px, last.x + last.w * 0.85, last.y + last.h * 0.5);
    assert!(p[3] > 50, "plaque must sit behind the last standings row, got {p:?}");
}

#[test]
fn stripe_row_bg_lifts_when_panel_is_opaque() {
    assert_eq!(rgba8(stripe_row_bg(background_alpha(78))), (0, 0, 0, 54));
    assert_eq!(rgba8(stripe_row_bg(background_alpha(100))), (255, 255, 255, 34));
}

#[test]
fn row_washes_are_spider_scaled() {
    let [r, g, b] = crate::config::accent_rgb();
    assert_eq!(rgba8(you_row_bg(50, 78)), (r, g, b, 52));
    assert_eq!(rgba8(you_row_bg(50, 100)), (r, g, b, 52));
    assert_eq!(rgba8(you_row_bg(100, 78)), (r, g, b, 104));
    assert_eq!(rgba8(you_row_bg(50, 39)), (r, g, b, 91));
    assert_eq!(rgba8(you_row_bg(50, 0)), (r, g, b, 130));
    assert_eq!(rgba8(you_row_bg(0, 0)), (r, g, b, 0));
    assert_eq!(rgba8(you_row_bg(100, 0)), (r, g, b, 255));
    assert_eq!(rgba8(lapping_row_bg(50, 78)), (59, 130, 246, 52));
    assert_eq!(rgba8(lapping_row_bg(50, 0)), (59, 130, 246, 130));
    assert_eq!(rgba8(lapped_row_bg(50, 78)), (239, 68, 68, 52));
}

#[test]
fn standings_stripes_visible_on_opaque_panel() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.local_race_num = 1;
    s.focus_race_num = 1;
    s.standing_count = 4;
    s.standings[2] = standing(3, 3, 5);
    s.standings[3] = standing(4, 4, 5);
    s.show_relative = 0;
    s.show_map = 0;
    let mut cfg = HudConfig::new();
    cfg[WidgetId::Standings].show = true;
    cfg[WidgetId::Minimap].show = false;
    cfg[WidgetId::Radar].show = false;
    cfg[WidgetId::Dash].show = false;
    cfg[WidgetId::Ticker].show = false;
    cfg[WidgetId::Sys].show = false;
    cfg[WidgetId::Standings].bg = 100;
    let mut striped = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut striped, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
    let hits = click_rider_hits();
    let mut by_y = hits.clone();
    by_y.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap());
    let stripe_a = sample_px(
        &striped,
        by_y[1].x + by_y[1].w * 0.5,
        by_y[1].y + by_y[1].h * 0.5,
    );
    let stripe_b = sample_px(
        &striped,
        by_y[2].x + by_y[2].w * 0.5,
        by_y[2].y + by_y[2].h * 0.5,
    );
    assert_ne!(stripe_a, stripe_b, "opaque board must still zebra");
    cfg.standings.st_stripe = false;
    let mut flat = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut flat, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
    let flat_a = sample_px(
        &flat,
        by_y[1].x + by_y[1].w * 0.5,
        by_y[1].y + by_y[1].h * 0.5,
    );
    let flat_b = sample_px(
        &flat,
        by_y[2].x + by_y[2].w * 0.5,
        by_y[2].y + by_y[2].h * 0.5,
    );
    assert_eq!(flat_a, flat_b);
}

#[test]
fn garage_leftover_standings_hide_race_widgets() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.on_track = 1;
    s.has_telemetry = 0;
    s.rider_count = 0;
    s.show_relative = 0;
    s.show_map = 0;
    let mut cfg = HudConfig::new();
    cfg[WidgetId::Minimap].show = false;
    cfg[WidgetId::Radar].show = false;
    cfg[WidgetId::Dash].show = false;
    cfg[WidgetId::Ticker].show = false;
    cfg[WidgetId::Sys].show = true;
    cfg[WidgetId::Stance].show = true;
    draw_ok(&s, &cfg);
    assert!(
        click_rider_hits().is_empty(),
        "garage leftover standings must not keep race widgets up"
    );
}

