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
    assert_eq!(session_banner(&s).1, "1 / 2");
    s.standings[1].num_laps = 1;
    s.current_lap = 2;
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
    s.session_length = 40;
    pass_the_leader(&mut s, 20.0);
    assert!(is_warmup(&s));
    let field = RaceStore::tick(&s).field;
    assert_eq!(field.rows[0].standing.race_num, 1);
    assert_eq!(live_position(12), 2);
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
    IN_GATE.store(0, Ordering::Relaxed);
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
    s.riders[0] = rider(1, 1.0, 0.0, 0.15);
    s.riders[1] = rider(12, 2.0, 0.0, 0.42);
    s.local_track_pos = 0.42;
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
    s.riders[0] = rider(1, 1.0, 0.0, 0.385);
    s.riders[1] = rider(12, 2.0, 0.0, 0.395);
    s.local_track_pos = 0.395;
    let _ = RaceStore::tick(&s);
    assert_eq!(live_position(12), 1, "20 m closer to S1");
    assert_eq!(live_position(1), 2);
}

/// No saved splits: the other rider's S1 split moves them up, then track travel
/// passes them back before the finish.
#[test]
fn a_sector_split_moves_then_track_pos_passes_back() {
    let _g = session_lock();
    let _splits = ResetSplits;
    reset_session();
    crate::sector::set_split_fracs([0.0, 0.0]);
    let mut s = gate_field(&[1, 12]);
    s.standings[1].sector_gate = 1;
    let _ = RaceStore::tick(&s);
    assert_eq!(live_position(12), 1, "S1 split before the finish");
    ride_field_to(&mut s, &[(1, 0.55), (12, 0.10)]);
    assert_eq!(live_position(1), 1, "track travel after the gate takes the place back");
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
    s.riders[0] = rider(1, 1.0, 0.0, 0.55);
    s.riders[1] = rider(12, 2.0, 0.0, 0.71);
    s.local_track_pos = 0.71;
    let _ = RaceStore::tick(&s);
    assert_eq!(live_position(12), 1, "just past S2, gate still 1");
    assert_eq!(live_position(1), 2, "still between S1 and S2");
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
    s.riders[0] = rider(1, 1.0, 0.0, 0.39);
    s.riders[1] = rider(12, 2.0, 0.0, 0.42);
    s.local_track_pos = 0.42;
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

/// If `num_laps` lags while the tracker keeps integrating, lap metres used to grow past a
/// full lap and invent P1 until the line republished. Past one lap without a bump must not.
#[test]
fn runaway_lap_metres_without_a_lap_bump_do_not_invent_p1() {
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
    assert_ne!(
        live_position(12),
        1,
        "extra lap of metres without a num_laps bump must not invent P1"
    );
    assert!(
        live_position(12) >= 4,
        "still behind riders who are truly ahead on track"
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
    s.riders[0] = rider(1, 0.0, 0.0, 0.70);
    s.riders[1] = rider(2, 0.0, 0.0, 0.68);
    s.riders[2] = rider(3, 0.0, 0.0, 0.576);
    s.riders[3] = rider(4, 0.0, 0.0, 0.511);
    s.riders[4] = rider(12, 0.0, 0.0, 0.500);
    s.local_track_pos = 0.500;
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
    s.riders[0] = rider(1, 0.0, 0.0, 0.55);
    s.riders[1] = rider(2, 0.0, 0.0, 0.52);
    s.riders[2] = rider(3, 0.0, 0.0, 0.51);
    s.riders[3] = rider(12, 0.0, 0.0, 0.50);
    s.local_track_pos = 0.50;
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
    s.riders[0] = rider(1, 0.0, 0.0, 0.55);
    s.riders[1] = rider(2, 0.0, 0.0, 0.52);
    s.riders[2] = rider(3, 0.0, 0.0, 0.51);
    s.riders[3] = rider(12, 0.0, 0.0, 0.50);
    s.local_track_pos = 0.50;
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

/// A 300 m jump in one tick is a reset or a teleport, not riding.
#[test]
fn a_teleport_is_not_distance_covered() {
    let _g = session_lock();
    reset_session();
    let mut s = gate_field(&[1, 12]);
    ride_field_to(&mut s, &[(1, 0.20), (12, 0.19)]);
    s.riders[1].track_pos = 0.49;
    s.local_track_pos = 0.49;
    let _ = RaceStore::tick(&s);
    assert_eq!(live_position(12), 2);
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
    // The board lands on the clock for a few frames. It shows, because a long drop in one
    // frame is also how a real countdown is fast-forwarded, and one frame cannot tell them
    // apart. What must not happen is the race ending on it.
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
    s.session_time_ms = 240_000;
    let later = RaceStore::tick(&s);
    assert_eq!(later.clock.banner.1, "04:00");
    assert_eq!(later.clock.mode, ClockMode::Timed);
    assert_eq!(later.clock.flag, RaceFlag::None);
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
    assert_eq!(rgba8(you_row_bg(50)), (r, g, b, 52));
    assert_eq!(rgba8(you_row_bg(100)), (r, g, b, 104));
    assert_eq!(rgba8(lapping_row_bg(50)), (59, 130, 246, 52));
    assert_eq!(rgba8(lapped_row_bg(50)), (239, 68, 68, 52));
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

