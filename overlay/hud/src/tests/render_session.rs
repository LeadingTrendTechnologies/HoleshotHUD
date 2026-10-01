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
fn last_lap_diff_is_against_the_previous_lap() {
    let _g = session_lock();
    reset_session();
    let cfg = HudConfig::new();
    assert!(!cfg.standings.st_lapdiff);
    assert!(!cfg.relative.rel_lapdiff);
    let mut s = live_snap();
    s.session_kind = 7;
    s.session_laps = 10;
    set_completed_lap(&mut s, 1, 72_000, 1);
    RaceStore::refresh(&s);
    assert_eq!(board_item(&s, &cfg, BoardField::LapDiff).unwrap().1, "--");
    assert_eq!(dash_foot_item(&s, &cfg, DashField::LapDiff).unwrap().1, "--");
    assert_eq!(lap_diff_ms(12), None);

    set_completed_lap(&mut s, 1, 72_400, 2);
    RaceStore::refresh(&s);
    assert_eq!(board_item(&s, &cfg, BoardField::LapDiff).unwrap().1, "+0.400");
    assert_eq!(dash_foot_item(&s, &cfg, DashField::LapDiff).unwrap().1, "+0.400");
    assert_eq!(lap_diff_ms(12), Some(400));

    set_completed_lap(&mut s, 1, 71_900, 3);
    RaceStore::refresh(&s);
    assert_eq!(board_item(&s, &cfg, BoardField::LapDiff).unwrap().1, "-0.500");
    assert_eq!(lap_diff_ms(12), Some(-500));

    s.standings[1].last_lap_ms = 0;
    s.last_lap_ms = 0;
    RaceStore::refresh(&s);
    assert_eq!(
        board_item(&s, &cfg, BoardField::LapDiff).unwrap().1,
        "-0.500",
        "a zeroed last lap must not replace the stored time"
    );

    reset_session();
    set_completed_lap(&mut s, 1, 72_000, 1);
    RaceStore::refresh(&s);
    assert_eq!(lap_diff_ms(12), None, "a new session starts with no previous lap");

    set_completed_lap(&mut s, 1, 72_000, 2);
    RaceStore::refresh(&s);
    RaceStore::refresh(&s);
    assert_eq!(board_item(&s, &cfg, BoardField::LapDiff).unwrap().1, "0.000");

    reset_session();
    s.standings[1].last_lap_ms = 0;
    s.last_lap_ms = 72_000;
    s.standings[1].num_laps = 1;
    s.current_lap = 2;
    RaceStore::refresh(&s);
    s.last_lap_ms = 72_400;
    s.standings[1].num_laps = 2;
    s.current_lap = 3;
    RaceStore::refresh(&s);
    assert_eq!(
        lap_diff_ms(12),
        Some(400),
        "your row falls back to last_lap_ms when the classification row is empty"
    );

    set_completed_lap(&mut s, 0, 70_000, 1);
    RaceStore::refresh(&s);
    set_completed_lap(&mut s, 0, 71_200, 2);
    RaceStore::refresh(&s);
    assert_eq!(lap_diff_ms(1), Some(1_200));
    assert_eq!(lap_diff_ms(12), Some(400));
}

#[test]
fn rider_current_lap_is_completed_plus_one() {
    let s = live_snap();
    assert_eq!(rider_current_lap(&s, 12, 5), 6);
    assert_eq!(rider_current_lap(&s, 1, 5), 6);
    let mut late = s;
    late.current_lap = 0;
    assert_eq!(rider_current_lap(&late, 12, 4), 5);
}

#[test]
fn lap_rel_colors_lapping_and_lapped_riders() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    let rel = |s: &Snapshot, n| {
        let _ = RaceStore::tick(s);
        lap_rel(s, n)
    };
    assert_eq!(rel(&s, 1), LapRel::Same);

    s.standings[0].num_laps = 6;
    s.riders[0].track_pos = 0.80;
    s.local_track_pos = 0.92;
    s.riders[1].track_pos = 0.92;
    assert_eq!(rel(&s, 1), LapRel::LappingMe);

    s.standings[0].num_laps = 6;
    s.riders[0].track_pos = 0.02;
    s.local_track_pos = 0.98;
    s.riders[1].track_pos = 0.98;
    assert_eq!(
        rel(&s, 1),
        LapRel::Same,
        "same-race S/F straddle is not a lap up"
    );

    // True lap-up hold: already a lap ahead, just gone by, still on the same
    // side of the line (not an S/F straddle).
    s.standings[0].num_laps = 6;
    s.riders[0].track_pos = 0.96;
    s.local_track_pos = 0.88;
    s.riders[1].track_pos = 0.88;
    assert_eq!(rel(&s, 1), LapRel::LappingMe, "lap up, just gone by, still nearby");

    s.standings[0].num_laps = 4;
    s.riders[0].track_pos = 0.97;
    s.local_track_pos = 0.90;
    s.riders[1].track_pos = 0.90;
    assert_eq!(rel(&s, 1), LapRel::LappedByMe);

    s.standings[0].num_laps = 6;
    s.riders[0].track_pos = 0.20;
    s.local_track_pos = 0.90;
    s.riders[1].track_pos = 0.90;
    assert_eq!(rel(&s, 1), LapRel::Same, "lap up but far ahead outside catch span");

    s.standings[0].num_laps = 5;
    s.standings[0].gap_laps = 0;
    s.standings[1].gap_laps = 1;
    s.riders[0].track_pos = 0.50;
    s.local_track_pos = 0.50;
    s.riders[1].track_pos = 0.50;
    assert_eq!(rel(&s, 1), LapRel::Same);

    s.riders[0].track_pos = 0.82;
    s.local_track_pos = 0.90;
    s.riders[1].track_pos = 0.90;
    assert_eq!(rel(&s, 1), LapRel::LappingMe);

    s.standings[0].gap_laps = 2;
    s.standings[1].gap_laps = 1;
    s.riders[0].track_pos = 0.96;
    s.local_track_pos = 0.88;
    s.riders[1].track_pos = 0.88;
    assert_eq!(
        rel(&s, 1),
        LapRel::Same,
        "more laps down to the leader is not red until you lap them"
    );

    // You actually gained a lap on them — red via pairwise num_laps.
    s.standings[1].num_laps = 6;
    s.riders[0].track_pos = 0.96;
    s.local_track_pos = 0.88;
    s.riders[1].track_pos = 0.88;
    assert_eq!(rel(&s, 1), LapRel::LappedByMe);
    assert_eq!(rel(&s, 12), LapRel::Same);
}

#[test]
fn lap_rel_same_lap_sf_straddle_stays_same() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    let rel = |s: &Snapshot, n| {
        let _ = RaceStore::tick(s);
        lap_rel(s, n)
    };

    // They crossed first: raw num_laps says +1, but continuous progress is ~0.
    s.standings[0].num_laps = 6;
    s.standings[1].num_laps = 5;
    s.standings[0].gap_laps = 0;
    s.standings[1].gap_laps = 0;
    s.riders[0].track_pos = 0.03;
    s.local_track_pos = 0.97;
    s.riders[1].track_pos = 0.97;
    assert_eq!(
        rel(&s, 1),
        LapRel::Same,
        "them across the line first is not lapping"
    );

    // You crossed first: raw num_laps says -1 until they cross.
    s.standings[0].num_laps = 5;
    s.standings[1].num_laps = 6;
    s.riders[0].track_pos = 0.97;
    s.local_track_pos = 0.03;
    s.riders[1].track_pos = 0.03;
    assert_eq!(
        rel(&s, 1),
        LapRel::Same,
        "you across the line first is not lapping them"
    );
}

#[test]
fn lap_rel_leader_lapped_rider_behind_stays_same() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    let rel = |s: &Snapshot, n| {
        let _ = RaceStore::tick(s);
        lap_rel(s, n)
    };

    // Leader lapped them; you have not. Equal num_laps, they sit just behind.
    s.standings[0].num_laps = 5;
    s.standings[1].num_laps = 5;
    s.standings[0].gap_laps = 1;
    s.standings[1].gap_laps = 0;
    s.riders[0].track_pos = 0.84;
    s.local_track_pos = 0.92;
    s.riders[1].track_pos = 0.92;
    assert_eq!(
        rel(&s, 1),
        LapRel::Same,
        "leader lapping them is not you lapping them"
    );
}

#[test]
fn lap_rel_leader_two_laps_up_stays_blue() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    let rel = |s: &Snapshot, n| {
        let _ = RaceStore::tick(s);
        lap_rel(s, n)
    };
    s.standings[0].num_laps = 7;
    s.standings[1].num_laps = 5;
    s.standings[0].gap_laps = 0;
    s.standings[1].gap_laps = 2;
    s.riders[0].track_pos = 0.82;
    s.local_track_pos = 0.90;
    s.riders[1].track_pos = 0.90;
    assert_eq!(rel(&s, 1), LapRel::LappingMe, "two down, closing from behind");

    s.riders[0].track_pos = 0.96;
    s.local_track_pos = 0.88;
    s.riders[1].track_pos = 0.88;
    assert_eq!(rel(&s, 1), LapRel::LappingMe, "two down, already gone by, still nearby");

    // Completed laps can sit on the race lap (or run ahead after our crossing)
    // while gap_laps still says we are two down. Must not flip the leader to red.
    s.standings[0].num_laps = 5;
    s.standings[1].num_laps = 7;
    s.riders[0].track_pos = 0.82;
    s.local_track_pos = 0.90;
    s.riders[1].track_pos = 0.90;
    assert_eq!(rel(&s, 1), LapRel::LappingMe, "gap wins over inverted num_laps");

    s.riders[0].track_pos = 0.96;
    s.local_track_pos = 0.88;
    s.riders[1].track_pos = 0.88;
    assert_eq!(
        rel(&s, 1),
        LapRel::LappingMe,
        "inverted laps after they pass stays blue, not red"
    );}

#[test]
fn lap_rel_off_in_warmup() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 10;
    s.session_laps = 0;
    s.standings[0].num_laps = 6;
    s.riders[0].track_pos = 0.80;
    s.local_track_pos = 0.92;
    s.riders[1].track_pos = 0.92;
    assert!(is_warmup(&s));
    assert_eq!(lap_rel(&s, 1), LapRel::Same);
    s.session_kind = 5;
    s.session_laps = 2;
    s.session_length = 15;
    assert!(is_warmup(&s), "kind 5 is warmup even with leaked extras");
    assert_eq!(lap_rel(&s, 1), LapRel::Same);
    s.session_kind = -1;
    s.session_length = 8;
    s.session_laps = 0;
    assert!(!is_warmup(&s));
    assert_eq!(lap_rel(&s, 1), LapRel::LappingMe);
}

#[test]
fn session_preset_from_live_state() {
    let _g = session_lock();
    reset_session();
    assert_eq!(session_preset(&Snapshot::default(), false), None);

    let mut s = live_snap();
    s.session_kind = 5;
    s.session_length = 15;
    s.session_laps = 0;
    assert_eq!(session_preset(&s, false), Some(SessionPreset::Warmup));
    assert_eq!(session_preset(&s, true), Some(SessionPreset::Spectate));

    reset_session();
    s = live_snap();
    s.session_kind = 5;
    s.session_length = 15;
    s.session_laps = 2;
    assert_eq!(
        session_preset(&s, false),
        Some(SessionPreset::Warmup),
        "kind 5 stays warmup when extras leak"
    );

    reset_session();
    s = live_snap();
    s.session_kind = 5;
    s.session_length = 30;
    s.session_laps = 0;
    assert_eq!(session_preset(&s, false), Some(SessionPreset::Warmup));

    reset_session();
    s = live_snap();
    s.session_kind = -1;
    s.session_length = 40;
    s.session_laps = 0;
    assert_eq!(session_preset(&s, false), Some(SessionPreset::Practice));

    reset_session();
    s = live_snap();
    s.session_kind = -1;
    s.session_length = 40;
    s.session_laps = 2;
    assert_eq!(
        session_preset(&s, false),
        Some(SessionPreset::Practice),
        "40+ min practice wins over leaked extras"
    );

    reset_session();
    s = live_snap();
    s.session_kind = -1;
    s.session_length = 0;
    s.session_laps = 0;
    s.session_time_ms = 90_000;
    assert_eq!(
        session_preset(&s, false),
        Some(SessionPreset::Practice),
        "open practice / Testing Setup with a live clock"
    );

    reset_session();
    s = live_snap();
    s.session_kind = 7;
    s.session_length = 8;
    s.session_laps = 0;
    assert_eq!(session_preset(&s, false), Some(SessionPreset::Race));

    reset_session();
    s = live_snap();
    s.session_kind = -1;
    s.session_length = 8;
    s.session_laps = 2;
    assert_eq!(
        session_preset(&s, false),
        Some(SessionPreset::Race),
        "short timed with extras stays Race"
    );

    reset_session();
    s = live_snap();
    s.session_kind = -1;
    s.session_length = 0;
    s.session_laps = 4;
    assert_eq!(session_preset(&s, false), Some(SessionPreset::Race));

    reset_session();
    s = live_snap();
    s.session_kind = -1;
    s.session_length = 15;
    s.session_laps = 0;
    assert_eq!(session_preset(&s, false), Some(SessionPreset::Warmup));

    reset_session();
    s = live_snap();
    s.has_telemetry = 0;
    s.rider_count = 3;
    assert!(s.has_session_data(), "replay payload is still in the snapshot");
    assert!(
        !live_session(&s, false),
        "leftover spectate dots in the garage are not a session"
    );
    assert_eq!(session_preset(&s, false), None);
    assert_eq!(session_preset(&s, true), Some(SessionPreset::Spectate));
}

#[test]
fn timed_session_shows_zero_of_extra_laps_until_local_cross() {
    let _g = session_lock();
    let mut s = live_snap();
    expire_timed(&mut s);
    assert_eq!(session_banner(&s).1, "0/2");
    s.standings[0].num_laps = 6;
    assert_eq!(session_banner(&s).1, "0/2", "leader cross must not increment");
    s.standings[1].num_laps = 6;
    s.current_lap = 7;
    assert_eq!(session_banner(&s).1, "1/2", "first extra cross starts the extra");
    s.standings[1].num_laps = 7;
    s.current_lap = 8;
    assert_eq!(session_banner(&s).1, "2/2");
}

#[test]
fn last_place_cross_at_expiry_stays_zero_until_leader_then_local() {
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
    s.local_speed = 18.0;
    s.current_lap = 3;
    s.standings[0].num_laps = 3;
    s.standings[1].num_laps = 2;
    let remain = session_remain_ms(&s).expect("countdown");
    assert!(remain > 60_000);
    s.session_time_ms = 8 * 60 * 1000 - 1_000;
    let _ = session_remain_ms(&s);
    s.session_time_ms = 400;
    assert_eq!(session_remain_ms(&s), Some(0));
    assert_eq!(session_banner(&s).1, "0/2");
    s.standings[1].num_laps = 3;
    s.current_lap = 4;
    assert_eq!(
        session_banner(&s).1,
        "0/2",
        "local cross before the leader starts extras must not count"
    );
    s.standings[0].num_laps = 4;
    assert_eq!(session_banner(&s).1, "0/2");
    s.standings[1].num_laps = 4;
    s.current_lap = 5;
    assert_eq!(session_banner(&s).1, "1/2", "first extra cross starts the extra");
    s.standings[0].num_laps = 5;
    s.standings[1].num_laps = 5;
    s.current_lap = 6;
    assert_eq!(session_banner(&s).1, "2/2");
    s.standings[0].num_laps = 6;
    s.standings[1].num_laps = 6;
    s.current_lap = 7;
    assert_eq!(session_banner(&s).1, "2/2");
}

#[test]
fn timed_plus_one_cross_before_leader_stays_zero_until_next_pass() {
    let _g = session_lock();
    let mut s = live_snap();
    expire_timed_extras(&mut s, 1);
    s.local_speed = 18.0;
    assert_eq!(session_banner(&s).1, "0/1");
    s.standings[1].num_laps = 6;
    s.current_lap = 7;
    assert_eq!(
        session_banner(&s).1,
        "0/1",
        "pass after time expire does not count until the leader starts extras"
    );
    assert_eq!(dash_race_flag(&s), DashFlag::None);
    s.standings[0].num_laps = 6;
    assert_eq!(session_banner(&s).1, "0/1", "leader start extras still 0/1 until you pass");
    // Premature cross bumped your base to 6, so the lap you are on still does not count.
    assert_eq!(ride_to(&mut s, 0.40), DashFlag::None, "one more lap before your extra");
    s.standings[1].num_laps = 7;
    s.current_lap = 8;
    assert_eq!(session_banner(&s).1, "1/1");
    assert_eq!(dash_race_flag(&s), DashFlag::White, "on the counted extra after premature cross");
    s.standings[1].num_laps = 8;
    s.current_lap = 9;
    assert_eq!(dash_race_flag(&s), DashFlag::Checkered);
}

#[test]
fn checkered_does_not_carry_into_warmup() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 10;
    s.session_laps = 4;
    s.session_time_ms = 0;
    s.current_lap = 5;
    s.local_speed = 18.0;
    s.standings[0].num_laps = 4;
    s.standings[1].num_laps = 4;
    CHECKERED_LATCH.store(1, Ordering::Relaxed);
    LAP_GREEN.store(1, Ordering::Relaxed);
    POST_GATE.store(1, Ordering::Relaxed);
    s.on_track = 1;
    assert_eq!(dash_race_flag(&s), DashFlag::Checkered);

    s.session_laps = 0;
    s.session_time_ms = 10 * 60 * 1000;
    s.current_lap = 1;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    assert_eq!(dash_race_flag(&s), DashFlag::None);
    assert_eq!(session_banner(&s).1, "10:00");
    s.session_time_ms = 9 * 60 * 1000;
    assert_eq!(session_banner(&s).1, "09:00");
}

#[test]
fn warmup_after_race_counts_down_from_ten() {
    let _g = session_lock();
    let mut s = live_snap();
    expire_timed(&mut s);
    CHECKERED_LATCH.store(1, Ordering::Relaxed);
    POST_GATE.store(1, Ordering::Relaxed);
    s.on_track = 1;
    s.session_length = 10;
    s.session_laps = 0;
    s.session_time_ms = 10 * 60 * 1000;
    s.current_lap = 1;
    s.local_speed = 16.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    assert_eq!(dash_race_flag(&s), DashFlag::None);
    assert_eq!(session_banner(&s).1, "10:00");
    s.session_time_ms = 9 * 60 * 1000 + 40_000;
    assert_eq!(session_banner(&s).1, "09:40");
}

#[test]
fn timed_plus_one_shows_white_then_checkered() {
    let _g = session_lock();
    let mut s = live_snap();
    expire_timed_extras(&mut s, 1);
    s.local_speed = 18.0;
    assert_eq!(dash_race_flag(&s), DashFlag::None, "no flags until extras start");
    // Leader starts extras. The lap you are on does not count, so your extra is still
    // one crossing away: two laps to run, no flag away from the run-in.
    s.standings[0].num_laps = 6;
    assert_eq!(ride_to(&mut s, 0.40), DashFlag::None, "the lap you are on is not the extra");
    assert_eq!(ride_lap_to_line(&mut s), DashFlag::White, "run-in onto your extra");
    // Your crossing starts the extra: the white is waved at the line, then put away.
    cross_line(&mut s, 6, 7);
    assert_eq!(dash_race_flag(&s), DashFlag::White, "white as the extra starts");
    age_white_wave();
    assert_eq!(ride_to(&mut s, 0.50), DashFlag::None, "not held up all lap");
    assert_eq!(cross_line(&mut s, 7, 8), DashFlag::Checkered);
    s.on_track = 0;
    assert_eq!(dash_race_flag(&s), DashFlag::None);
}

#[test]
fn timed_plus_one_first_extra_crossing_is_white_not_checkered() {
    let _g = session_lock();
    let mut s = live_snap();
    expire_timed_extras(&mut s, 1);
    s.local_speed = 18.0;
    s.standings[0].num_laps = 6;
    s.standings[1].num_laps = 5;
    s.current_lap = 6;
    assert_eq!(
        ride_to(&mut s, 0.40),
        DashFlag::None,
        "the lap running when the leader starts extras is not yours"
    );
    // Your first extra crossing puts you on the last lap — white, never checkered.
    cross_line(&mut s, 6, 7);
    assert_eq!(dash_race_flag(&s), DashFlag::White);
    assert_ne!(dash_race_flag(&s), DashFlag::Checkered);
    age_white_wave();
    assert_eq!(ride_to(&mut s, 0.40), DashFlag::None, "the wave is over");
    assert_eq!(
        ride_lap_to_line(&mut s),
        DashFlag::Checkered,
        "checkered out on the finish run-in"
    );
    assert_eq!(
        CHECKERED_LATCH.load(Ordering::Relaxed),
        0,
        "the run-in must not latch it"
    );
    assert_eq!(session_banner(&s).1, "1/1");
    assert_eq!(cross_line(&mut s, 7, 8), DashFlag::Checkered);
    assert_eq!(session_banner(&s).1, "1/1");
}

#[test]
fn twenty_five_minute_plus_two_is_timed_not_a_two_lap_moto() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_kind = 7;
    s.session_length = 25;
    s.session_laps = 2;
    s.session_time_ms = 30_000;
    s.current_lap = 1;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    assert_eq!(session_banner(&s).1, "00:30");
    s.session_time_ms = 25 * 60 * 1000;
    s.local_speed = 18.0;
    s.current_lap = 2;
    s.standings[0].num_laps = 1;
    s.standings[1].num_laps = 1;
    let remain = session_remain_ms(&s).expect("25:00+2 countdown");
    assert!(remain > 20 * 60 * 1000, "expected ~25 min left, got {remain}");
    assert_ne!(session_banner(&s).1, "2 / 2");
    assert!(!overtime_active(&s));
}

#[test]
fn warmup_fifteen_then_fifteen_plus_one_still_counts_down() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_kind = 5;
    s.session_length = 15;
    s.session_laps = 0;
    s.session_time_ms = 15 * 60 * 1000;
    s.current_lap = 2;
    s.local_speed = 12.0;
    s.standings[0].num_laps = 1;
    s.standings[1].num_laps = 1;
    assert_eq!(session_banner(&s).1, "15:00");
    s.session_time_ms = 1_000;
    let _ = session_remain_ms(&s);
    s.session_time_ms = 400;
    let _ = session_remain_ms(&s);

    s.session_kind = 7;
    s.session_laps = 1;
    s.session_time_ms = 15 * 60 * 1000;
    s.current_lap = 1;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    assert_eq!(session_banner(&s).1, "15:00");
    assert_ne!(session_banner(&s).1, "0/1");
    assert!(!overtime_active(&s));
}

#[test]
fn ticker_title_warmup_not_timed_or_lap_race() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 10;
    s.session_laps = 0;
    s.session_time_ms = 10 * 60 * 1000;
    s.current_lap = 1;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    assert_eq!(ticker_title(&s), "WARMUP - TEST TRACK");
    s.session_length = 40;
    assert_eq!(ticker_title(&s), "WARMUP - TEST TRACK");
    s.session_kind = 7;
    s.session_length = 15;
    s.session_laps = 0;
    assert_eq!(
        ticker_title(&s),
        "TIMED - TEST TRACK",
        "a 15:00 race with unpublished extras is not warmup"
    );
    s.session_kind = -1;
    s.session_length = 8;
    s.session_laps = 1;
    assert_eq!(ticker_title(&s), "TIMED - TEST TRACK");
    s.session_length = 0;
    s.session_laps = 4;
    assert_eq!(ticker_title(&s), "LAP RACE - TEST TRACK");
}

#[test]
fn lap_race_shows_white_then_checkered() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 7;
    s.session_laps = 3;
    s.session_time_ms = 40_000;
    s.current_lap = 1;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    s.local_track_pos = 0.88;
    s.riders[1].track_pos = 0.88;
    assert_eq!(session_banner(&s).1, "00:40");
    assert_eq!(dash_race_flag(&s), DashFlag::None);
    s.session_time_ms = 50_000;
    s.local_speed = 18.0;
    let _ = session_remain_ms(&s);

    // Lap 2 of 3.
    cross_line(&mut s, 1, 2);
    assert_eq!(session_banner(&s).1, "2 / 3");
    assert_eq!(ride_to(&mut s, 0.40), DashFlag::None);
    assert_eq!(
        ride_lap_to_line(&mut s),
        DashFlag::White,
        "white on the run-in that starts the last lap"
    );

    // Lap 3 of 3 — white as the lap starts, then away; checkered on the run-in.
    cross_line(&mut s, 2, 3);
    assert_eq!(session_banner(&s).1, "3 / 3");
    assert_eq!(dash_race_flag(&s), DashFlag::White);
    assert_eq!(ride_to(&mut s, 0.40), DashFlag::White, "still waving");
    age_white_wave();
    assert_eq!(ride_to(&mut s, 0.45), DashFlag::None, "the wave is over");
    assert_eq!(
        ride_lap_to_line(&mut s),
        DashFlag::Checkered,
        "checkered on the finish run-in"
    );

    assert_eq!(cross_line(&mut s, 3, 4), DashFlag::Checkered);
    assert_eq!(ride_to(&mut s, 0.20), DashFlag::Checkered, "checkered latches");
    s.on_track = 0;
    assert_eq!(dash_race_flag(&s), DashFlag::None);
}

/// The checkered goes up on the finish run-in, but only the crossing latches it.
#[test]
fn lap_race_shows_the_checkered_on_the_finish_run_in() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 0;
    s.session_laps = 4;
    s.session_time_ms = 0;
    s.current_lap = 3;
    s.local_speed = 18.0;
    s.standings[0].num_laps = 2;
    s.standings[1].num_laps = 2;
    assert_eq!(session_banner(&s).1, "3 / 4");
    assert_eq!(ride_to(&mut s, 0.40), DashFlag::None, "mid lap 3 of 4");
    assert_eq!(
        ride_lap_to_line(&mut s),
        DashFlag::White,
        "run-in onto the last lap"
    );

    cross_line(&mut s, 3, 4);
    assert_eq!(session_banner(&s).1, "4 / 4");
    assert_eq!(ride_to(&mut s, 0.40), DashFlag::White);
    age_white_wave();
    assert_eq!(ride_to(&mut s, 0.45), DashFlag::None, "the wave is over");
    assert_eq!(
        ride_lap_to_line(&mut s),
        DashFlag::Checkered,
        "80 m short of the finish"
    );
    assert_eq!(
        CHECKERED_LATCH.load(Ordering::Relaxed),
        0,
        "proximity must not latch the finish"
    );
    assert_eq!(cross_line(&mut s, 4, 5), DashFlag::Checkered);
    assert_eq!(ride_to(&mut s, 0.20), DashFlag::Checkered);
}

#[test]
fn four_lap_race_white_on_last_lap_start_not_checkered() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 0;
    s.session_laps = 4;
    s.session_time_ms = 0;
    s.local_speed = 18.0;
    // Two laps still to run after this one — no flags anywhere on the lap.
    s.standings[0].num_laps = 1;
    s.standings[1].num_laps = 1;
    s.current_lap = 2;
    assert_eq!(session_banner(&s).1, "2 / 4");
    assert_eq!(ride_to(&mut s, 0.40), DashFlag::None);
    assert_eq!(ride_lap_to_line(&mut s), DashFlag::None);

    // Lap 3 of 4: the run-in onto the last lap is white.
    cross_line(&mut s, 2, 3);
    assert_eq!(session_banner(&s).1, "3 / 4");
    assert_eq!(ride_to(&mut s, 0.40), DashFlag::None);
    assert_eq!(ride_lap_to_line(&mut s), DashFlag::White);
    assert_ne!(dash_race_flag(&s), DashFlag::Checkered);

    cross_line(&mut s, 3, 4);
    assert_eq!(session_banner(&s).1, "4 / 4");
    assert_eq!(
        dash_race_flag(&s),
        DashFlag::White,
        "white when last lap starts"
    );
    assert_ne!(dash_race_flag(&s), DashFlag::Checkered);
    age_white_wave();
    assert_eq!(
        ride_to(&mut s, 0.40),
        DashFlag::None,
        "the wave does not stay up for the whole last lap"
    );
}

/// A lap down when the leader takes the finish: you still run your remaining laps.
/// Checkered waits until you complete the distance, not until they take the flag.
#[test]
fn lapped_rider_checkered_when_they_finish_not_the_leader() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 0;
    s.session_laps = 5;
    s.session_time_ms = 0;
    s.local_speed = 18.0;
    // You are on lap 4 of 5, the leader is on their last lap.
    s.standings[0].num_laps = 4;
    s.standings[1].num_laps = 3;
    s.current_lap = 4;
    s.standings[1].gap_laps = 1;
    assert_eq!(session_banner(&s).1, "4 / 5");
    assert_eq!(
        ride_to(&mut s, 0.40),
        DashFlag::None,
        "two of your own laps still to run"
    );

    s.standings[0].num_laps = 5;
    assert!(leader_finished(&s));
    assert_eq!(laps_left(&s), Some(2), "leader finishing does not cut your distance");
    assert_eq!(
        ride_lap_to_line(&mut s),
        DashFlag::White,
        "run-in onto your last lap"
    );
    assert_ne!(cross_line(&mut s, 4, 5), DashFlag::Checkered);
    age_white_wave();
    assert_eq!(ride_lap_to_line(&mut s), DashFlag::Checkered);
    assert_eq!(cross_line(&mut s, 5, 6), DashFlag::Checkered);
}

/// Lapped on 4/5: that run-in is still onto your last lap, not a wave-off.
#[test]
fn lapped_on_four_of_five_run_in_is_white_not_checkered() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 0;
    s.session_laps = 5;
    s.session_time_ms = 0;
    s.local_speed = 18.0;
    s.standings[0].num_laps = 4;
    s.standings[1].num_laps = 3;
    s.current_lap = 4;
    s.standings[1].gap_laps = 1;
    ride_to(&mut s, 0.50);
    assert_eq!(session_banner(&s).1, "4 / 5");
    assert_eq!(
        ride_lap_to_line(&mut s),
        DashFlag::White,
        "lapped on 4/5: white onto your last lap"
    );

    s.standings[0].num_laps = 5;
    assert!(leader_finished(&s));
    assert_ne!(
        dash_race_flag(&s),
        DashFlag::Checkered,
        "leader finished in the window: still your last-lap white"
    );
}

/// A first-lap crash (or a frame that collapses remaining laps) must not wave white.
#[test]
fn first_lap_crash_does_not_wave_white() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 0;
    s.session_laps = 5;
    s.session_time_ms = 0;
    s.local_speed = 18.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    s.current_lap = 1;
    LAP_GREEN.store(1, Ordering::Relaxed);
    assert_eq!(ride_to(&mut s, 0.40), DashFlag::None);
    assert_eq!(laps_left(&s), Some(5));

    s.local_crashed = 1;
    s.local_speed = 12.0;
    assert_eq!(dash_race_flag(&s), DashFlag::None, "crash on lap 1 is not a last lap");

    s.standings[0].num_laps = 5;
    assert_eq!(
        dash_race_flag(&s),
        DashFlag::None,
        "collapsed laps-left after a crash must not wave white"
    );
    assert_ne!(dash_race_flag(&s), DashFlag::White);
}

/// The `~Lapped` tag follows the classification gap, and must stay off the gate, out of
/// warmup, and away from anyone who is merely being caught.
#[test]
fn lapped_tag_tracks_the_classification_gap() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 0;
    s.session_laps = 5;
    s.session_time_ms = 0;
    s.local_speed = 18.0;
    s.standings[0].num_laps = 3;
    s.standings[1].num_laps = 3;
    s.current_lap = 4;
    assert!(!lapped(&s), "same lap as the leader");

    s.standings[1].gap_laps = 1;
    assert!(lapped(&s), "a lap down");
    s.standings[1].gap_laps = 2;
    assert!(lapped(&s), "two laps down");
    // The tag widens the right column, so make sure the dash still lays out and draws.
    let cfg = HudConfig::new();
    assert!(dash_layout(&fonts(), &s, &cfg, 1280.0, 720.0, DashFlag::None, 0.0).lapped);
    draw_ok(&s, &cfg);

    // Not on the gate, and not while off track.
    IN_GATE.store(1, std::sync::atomic::Ordering::Relaxed);
    assert!(!lapped(&s), "no tag on the gate");
    IN_GATE.store(0, std::sync::atomic::Ordering::Relaxed);
    s.on_track = 0;
    assert!(!lapped(&s), "no tag off track");
    s.on_track = 1;

    // Warmup has no leader to be a lap behind.
    let mut warm = s;
    warm.session_laps = 0;
    warm.session_length = 10;
    assert!(is_warmup(&warm));
    assert!(!lapped(&warm), "no tag in warmup");
}

/// The leader taking the flag while you are still on the last lap is not a lap down —
/// they have one more completed lap because they crossed and you have not.
#[test]
fn finishing_last_lap_after_the_leader_is_not_lapped() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 0;
    s.session_laps = 4;
    s.session_time_ms = 0;
    s.local_speed = 18.0;
    s.standings[0].num_laps = 3;
    s.standings[1].num_laps = 3;
    s.current_lap = 4;
    assert_eq!(session_banner(&s).1, "4 / 4");
    assert!(!lapped(&s));

    s.standings[0].num_laps = 4;
    assert!(leader_finished(&s));
    assert!(!lapped(&s), "still on the last lap the leader just finished");
    assert_eq!(session_banner(&s).1, "4 / 4");
    let cfg = HudConfig::new();
    assert!(!dash_layout(&fonts(), &s, &cfg, 1280.0, 720.0, DashFlag::White, 0.0).lapped);
}

/// Winning must not stretch the total: your own finish latches the leader base at your
/// full lap count, and `+1` past it would read `5 / 6` on a 5-lap moto.
#[test]
fn winner_keeps_the_full_lap_total() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 0;
    s.session_laps = 5;
    s.session_time_ms = 0;
    s.local_speed = 18.0;
    s.standings[0].num_laps = 4;
    s.standings[1].num_laps = 4;
    s.current_lap = 5;
    assert_eq!(session_banner(&s).1, "5 / 5");
    assert_eq!(dash_race_flag(&s), DashFlag::White, "on the last lap");
    assert_eq!(cross_line(&mut s, 5, 6), DashFlag::Checkered);
    assert_eq!(effective_race_laps(&s), 5);
    assert_eq!(session_banner(&s).1, "5 / 5");
}

/// Leader completing extras does not wave you off. Checkered is when you finish yours.
#[test]
fn lapped_rider_checkered_when_they_finish_extras() {
    let _g = session_lock();
    let mut s = live_snap();
    expire_timed_extras(&mut s, 1);
    s.local_speed = 18.0;
    s.standings[0].num_laps = 6;
    s.standings[1].num_laps = 5;
    s.current_lap = 6;
    assert!(extras_started(&s));
    assert!(!leader_finished(&s));

    s.standings[0].num_laps = 7;
    assert!(leader_finished(&s));
    assert_eq!(laps_left(&s), Some(2), "uncounted + extra still to run");
    assert_eq!(ride_to(&mut s, 0.40), DashFlag::None);
    assert_eq!(ride_lap_to_line(&mut s), DashFlag::White, "run-in onto your extra");
    assert_ne!(cross_line(&mut s, 6, 7), DashFlag::Checkered);
    age_white_wave();
    assert_eq!(ride_lap_to_line(&mut s), DashFlag::Checkered);
    assert_eq!(cross_line(&mut s, 7, 8), DashFlag::Checkered);
}

/// The run-in white is the only flag that trusts track geometry, so it stays off until
/// the position data has actually shown a lap going past.
#[test]
fn early_white_needs_a_real_run_in() {
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

    // Dropped straight into the window with no lap behind us — nothing to go on.
    assert_eq!(ride_to(&mut s, 0.92), DashFlag::None, "no mid-lap sighting yet");
    // Half a lap out, then closing in: a real run-in.
    ride_to(&mut s, 0.50);
    assert_eq!(ride_to(&mut s, 0.96), DashFlag::White, "closing — run-in armed");
    // Still inside the window but drifting back from the line.
    assert_eq!(ride_to(&mut s, 0.94), DashFlag::None, "not closing on the line");
}

/// 8:00 + 2 where you cross after time expiry but before the leader. That crossing does
/// not count, so you run one uncounted lap and then two extras. Flags must not appear on
/// the uncounted lap, and must not flicker on either run-in.
#[test]
fn timed_plus_two_uncounted_lap_then_two_extras() {
    let _g = session_lock();
    let mut s = live_snap();
    expire_timed_extras(&mut s, 2);
    s.local_speed = 18.0;
    // You cross just after the clock hits zero; the leader has not crossed yet.
    s.standings[1].num_laps = 6;
    s.current_lap = 7;
    assert_eq!(dash_race_flag(&s), DashFlag::None);
    // Leader crosses and starts the extras. You are mid-lap, so this lap is uncounted:
    // three laps still to run.
    s.standings[0].num_laps = 6;
    assert!(extras_started(&s));
    assert_eq!(laps_left(&s), Some(3));
    assert_eq!(session_banner(&s).1, "0/2");
    assert_eq!(ride_to(&mut s, 0.40), DashFlag::None);
    assert_eq!(
        ride_lap_to_line(&mut s),
        DashFlag::None,
        "no flag on the run-in of the uncounted lap"
    );

    // First extra: two to run, so only the run-in is white.
    cross_line(&mut s, 7, 8);
    assert_eq!(laps_left(&s), Some(2));
    assert_eq!(session_banner(&s).1, "1/2");
    assert_eq!(ride_to(&mut s, 0.40), DashFlag::None);
    assert_eq!(ride_lap_to_line(&mut s), DashFlag::White, "run-in onto the last extra");

    // Last extra: white at the line, then the checkered on the finish run-in.
    cross_line(&mut s, 8, 9);
    assert_eq!(laps_left(&s), Some(1));
    assert_eq!(session_banner(&s).1, "2/2");
    assert_eq!(ride_to(&mut s, 0.40), DashFlag::White);
    age_white_wave();
    assert_eq!(ride_lap_to_line(&mut s), DashFlag::Checkered);
    assert_eq!(cross_line(&mut s, 9, 10), DashFlag::Checkered);
}

/// Leader completing extras early does not shorten yours. `0/2` still has the uncounted
/// lap and both extras; checkered is when you finish that distance.
#[test]
fn lapped_during_extras_still_runs_the_extras() {
    let _g = session_lock();
    let mut s = live_snap();
    expire_timed_extras(&mut s, 2);
    s.local_speed = 18.0;
    s.standings[0].num_laps = 6;
    assert_eq!(session_banner(&s).1, "0/2");
    assert_eq!(effective_extra_laps(&s), 2);
    assert_eq!(ride_to(&mut s, 0.40), DashFlag::None);

    s.standings[0].num_laps = 8;
    assert!(leader_finished(&s));
    assert_eq!(laps_left(&s), Some(3), "uncounted + both extras");
    assert_eq!(ride_to(&mut s, 0.40), DashFlag::None);
    assert_ne!(
        ride_lap_to_line(&mut s),
        DashFlag::Checkered,
        "leader finished: you still have extras to run"
    );
    assert_ne!(cross_line(&mut s, 6, 7), DashFlag::Checkered);
    assert_eq!(laps_left(&s), Some(2));
}

/// Lapped on `0/2`. The line into `1/2` is not the checkered. The next one, which
/// would start `2/2`, is, and you do not ride that extra.
#[test]
fn lapped_on_zero_of_plus_two_checkered_at_two_of_two() {
    let _g = session_lock();
    let mut s = live_snap();
    expire_timed_extras(&mut s, 2);
    s.local_speed = 18.0;
    s.standings[0].num_laps = 6;
    assert!(extras_started(&s));
    assert_eq!(session_banner(&s).1, "0/2");
    ride_to(&mut s, 0.50);
    leader_goes_past(&mut s);
    assert_eq!(laps_left(&s), Some(3), "still the uncounted lap");
    assert_eq!(session_banner(&s).1, "0/2");
    assert_ne!(
        ride_lap_to_line(&mut s),
        DashFlag::Checkered,
        "starting 1/2 is not the checkered"
    );
    assert_ne!(cross_line(&mut s, 6, 7), DashFlag::Checkered);
    assert_eq!(session_banner(&s).1, "1/2");
    assert_eq!(laps_left(&s), Some(1), "the next line would start 2/2");
    assert_eq!(ride_to(&mut s, 0.40), DashFlag::None, "no white flash on 1/2");
    assert_eq!(ride_lap_to_line(&mut s), DashFlag::Checkered);
    assert_eq!(cross_line(&mut s, 7, 8), DashFlag::Checkered);
    assert_eq!(laps_left(&s), Some(0));
    assert_eq!(ride_to(&mut s, 0.20), DashFlag::Checkered, "checkered latches");
}

/// Lapped on `1/2`, one lap before the final extra. That line is the checkered.
#[test]
fn lapped_on_one_of_plus_two_checkered_at_the_line() {
    let _g = session_lock();
    let mut s = live_snap();
    expire_timed_extras(&mut s, 2);
    s.local_speed = 18.0;
    s.standings[0].num_laps = 6;
    assert_eq!(session_banner(&s).1, "0/2");
    assert_ne!(cross_line(&mut s, 6, 7), DashFlag::Checkered);
    assert_eq!(session_banner(&s).1, "1/2");
    assert_eq!(laps_left(&s), Some(2));

    s.standings[0].num_laps = 7;
    assert!(!leader_finished(&s));
    ride_to(&mut s, 0.50);
    leader_goes_past(&mut s);
    assert_eq!(laps_left(&s), Some(1), "this line would start 2/2");
    assert_eq!(session_banner(&s).1, "1/2");
    assert_eq!(ride_to(&mut s, 0.40), DashFlag::None, "no white flash mid-lap");
    assert_eq!(ride_lap_to_line(&mut s), DashFlag::Checkered);
    assert_eq!(cross_line(&mut s, 7, 8), DashFlag::Checkered);
    assert_eq!(laps_left(&s), Some(0));
    assert_eq!(ride_to(&mut s, 0.20), DashFlag::Checkered, "checkered latches");
}

/// +1 extras, still on `0/1`, then someone puts a lap on you. That is ~Lapped, not a
/// wave-off. The extra still has to be run; checkered a lap early was the SX bug.
#[test]
fn lapped_on_uncounted_plus_one_still_runs_the_extra() {
    let _g = session_lock();
    let mut s = live_snap();
    expire_timed_extras(&mut s, 1);
    s.local_speed = 18.0;
    s.standings[0].num_laps = 6;
    s.standings[1].num_laps = 5;
    s.current_lap = 6;
    assert!(extras_started(&s));
    assert!(!leader_finished(&s));
    assert_eq!(session_banner(&s).1, "0/1");
    assert!(!lapped(&s));
    ride_to(&mut s, 0.50);
    assert_eq!(
        ride_lap_to_line(&mut s),
        DashFlag::White,
        "run-in onto the extra is still white until you are lapped"
    );

    s.standings[1].gap_laps = 1;
    assert!(lapped(&s));
    assert_eq!(session_banner(&s).1, "0/1", "being lapped does not start the extra");
    assert_eq!(laps_left(&s), Some(2), "uncounted + extra still to run");
    age_white_wave();
    assert_ne!(
        ride_lap_to_line(&mut s),
        DashFlag::Checkered,
        "leader has not finished — this line is not the checkered"
    );
    assert_ne!(cross_line(&mut s, 6, 7), DashFlag::Checkered);
    assert_eq!(session_banner(&s).1, "1/1");
    assert_eq!(laps_left(&s), Some(1));

    // Leader completes their extra. You still finish yours.
    s.standings[0].num_laps = 7;
    assert!(leader_finished(&s));
    assert_eq!(laps_left(&s), Some(1), "leader finishing does not end your extra");
    assert_eq!(
        ride_lap_to_line(&mut s),
        DashFlag::Checkered,
        "your extra finish is the checkered"
    );
    assert_eq!(cross_line(&mut s, 7, 8), DashFlag::Checkered);
}

/// Leader finishes the +1 on the lap they used to put you a lap down, while you are
/// still on `0/1`. That run-in is the checkered. White stays only while they have
/// not taken the flag yet.
#[test]
fn lapped_on_plus_one_final_lap_is_checkered() {
    let _g = session_lock();
    let mut s = live_snap();
    expire_timed_extras(&mut s, 1);
    s.local_speed = 18.0;
    s.standings[0].num_laps = 6;
    s.standings[1].num_laps = 5;
    s.current_lap = 6;
    assert!(extras_started(&s));
    assert!(!leader_finished(&s));
    assert_eq!(session_banner(&s).1, "0/1");
    ride_to(&mut s, 0.50);

    s.riders[0].track_pos = 0.35;
    assert!(!lapped(&s), "leader behind is not a lap yet");
    s.riders[0].track_pos = 0.42;
    assert!(!lapped(&s), "still closing from behind");
    s.riders[0].track_pos = 0.54;
    assert!(lapped(&s), "~Lapped the moment the leader goes past");
    assert_eq!(laps_left(&s), Some(2), "leader has not finished — extra still to run");
    assert_eq!(session_banner(&s).1, "0/1");

    s.standings[0].num_laps = 7;
    assert!(leader_finished(&s));
    assert_eq!(session_banner(&s).1, "0/1", "still the uncounted lap");
    assert_eq!(laps_left(&s), Some(1), "this line is the finish");
    assert_eq!(ride_to(&mut s, 0.40), DashFlag::None, "no white flash mid-lap");
    assert_eq!(
        ride_lap_to_line(&mut s),
        DashFlag::Checkered,
        "leader finished the lap that lapped you"
    );
    assert_eq!(cross_line(&mut s, 6, 7), DashFlag::Checkered);
    assert_eq!(laps_left(&s), Some(0));
    assert_eq!(ride_to(&mut s, 0.20), DashFlag::Checkered, "checkered latches");
}

/// Already a lap (or more) down when extras start, as in an 8:00+1 SX moto. The
/// uncounted crossing must not latch checkered while the leader still has the extra.
#[test]
fn already_lapped_when_plus_one_starts_is_not_checkered() {
    let _g = session_lock();
    let mut s = live_snap();
    expire_timed_extras(&mut s, 1);
    s.local_speed = 18.0;
    s.standings[1].gap_laps = 2;
    s.standings[0].num_laps = 6;
    s.standings[1].num_laps = 5;
    s.current_lap = 6;
    assert!(extras_started(&s));
    assert!(!leader_finished(&s));
    assert!(lapped(&s));
    assert_eq!(session_banner(&s).1, "0/1");
    assert_eq!(laps_left(&s), Some(2));
    assert_eq!(ride_to(&mut s, 0.40), DashFlag::None);
    assert_ne!(
        ride_lap_to_line(&mut s),
        DashFlag::Checkered,
        "extras starting is not a wave-off"
    );
    assert_ne!(cross_line(&mut s, 6, 7), DashFlag::Checkered);
    assert_eq!(session_banner(&s).1, "1/1");
    assert_eq!(laps_left(&s), Some(1));
}

/// `gap_laps` waits for a line crossing. ~Lapped must flip the moment the lap-up
/// leader goes past you. That pass does not start the extra or wave you off.
#[test]
fn lapped_by_leader_on_track_is_not_a_wave_off() {
    let _g = session_lock();
    let mut s = live_snap();
    expire_timed_extras(&mut s, 1);
    s.local_speed = 18.0;
    s.standings[0].num_laps = 6;
    s.standings[1].num_laps = 5;
    s.current_lap = 6;
    assert!(extras_started(&s));
    ride_to(&mut s, 0.50);
    // Stay inside PAIR_MAX_M (250 m on this 1000 m fixture) and move continuously.
    s.riders[0].track_pos = 0.35;
    assert_eq!(session_banner(&s).1, "0/1", "leader behind is not a lap yet");
    assert!(!lapped(&s), "extras starting is not getting lapped");

    s.riders[0].track_pos = 0.42;
    assert!(!lapped(&s), "still closing from behind");
    s.riders[0].track_pos = 0.54;
    assert!(lapped(&s), "~Lapped the moment the leader goes past");
    assert_eq!(session_banner(&s).1, "0/1", "still the uncounted lap");
    assert_eq!(laps_left(&s), Some(2), "the extra is still ahead");
    assert_eq!(s.standings[1].gap_laps, 0, "must not wait for the game gap");
}

/// Timed +2 on `1/2`: leader has finished (`lead == 1`) and goes past you. ~Lapped and
/// the waved-off `1/1` banner must flip on that pass, not at your next line.
#[test]
fn lapped_on_first_of_plus_two_latches_when_leader_finished_passes() {
    let _g = session_lock();
    let mut s = live_snap();
    expire_timed_extras(&mut s, 2);
    s.local_speed = 18.0;
    // Match timed +2 setup: your lap ticks before the leader starts extras so
    // OVERTIME_LOCAL_BASE high-waters onto this uncounted crossing.
    s.standings[1].num_laps = 6;
    s.current_lap = 7;
    let _ = session_banner(&s);
    s.standings[0].num_laps = 6;
    assert!(extras_started(&s));
    assert_eq!(session_banner(&s).1, "0/2");
    cross_line(&mut s, 7, 8);
    assert_eq!(session_banner(&s).1, "1/2");

    // Leader finished their +2; one completed lap ahead while you are still on 1/2.
    s.standings[0].num_laps = 8;
    assert!(leader_finished(&s));
    assert_eq!(leader_num_laps(&s) - s.standings[1].num_laps, 1);
    ride_to(&mut s, 0.50);
    assert_eq!(session_banner(&s).1, "1/1", "wave-off total once leader finished");
    assert!(!lapped(&s), "leader finished ahead is not ~Lapped until they pass");

    s.riders[0].track_pos = 0.35;
    assert!(!lapped(&s));
    s.riders[0].track_pos = 0.42;
    assert!(!lapped(&s), "still closing from behind");
    s.riders[0].track_pos = 0.54;
    assert!(lapped(&s), "~Lapped the moment the finished leader goes past");
    assert_eq!(session_banner(&s).1, "1/1");
    assert_eq!(s.standings[1].gap_laps, 0, "must not wait for the game gap");
}

/// On the last timed extra, +1 completed lap after the leader finishes is still the same
/// last lap — not ~Lapped until a real second lap down.
#[test]
fn finishing_last_extra_after_the_leader_is_not_lapped() {
    let _g = session_lock();
    let mut s = live_snap();
    expire_timed_extras(&mut s, 2);
    s.local_speed = 18.0;
    s.standings[1].num_laps = 6;
    s.current_lap = 7;
    let _ = session_banner(&s);
    s.standings[0].num_laps = 6;
    cross_line(&mut s, 7, 8);
    cross_line(&mut s, 8, 9);
    assert_eq!(session_banner(&s).1, "2/2");

    s.standings[0].num_laps = 9;
    assert!(leader_finished(&s));
    assert_eq!(leader_num_laps(&s) - s.standings[1].num_laps, 1);
    ride_to(&mut s, 0.50);
    assert!(!lapped(&s), "same last extra the leader just finished");
}

/// A frame where the session fields glitch into looking like a lap race puts extras (2)
/// against real lap counts, so `session_laps - laps_done` goes negative and clamps to
/// zero. That must not wave you off mid-race.
#[test]
fn glitched_lap_race_frame_does_not_latch_checkered() {
    let _g = session_lock();
    let mut s = live_snap();
    expire_timed_extras(&mut s, 2);
    s.local_speed = 18.0;
    s.standings[0].num_laps = 6;
    cross_line(&mut s, 6, 7);
    assert_eq!(laps_left(&s), Some(2), "on your first extra");
    assert!(!finish_earned(&s));

    // Session length drops out for a frame, so the clock heuristics read a lap moto.
    let mut glitch = s;
    glitch.session_length = 0;
    glitch.session_time_ms = 0;
    assert_ne!(
        dash_race_flag(&glitch),
        DashFlag::Checkered,
        "a glitched frame must not finish the race"
    );
    // Back to normal: still riding, still white on the run-in.
    assert_eq!(ride_lap_to_line(&mut s), DashFlag::White);
    assert_eq!(cross_line(&mut s, 7, 8), DashFlag::White);
    assert_eq!(cross_line(&mut s, 8, 9), DashFlag::Checkered);
}

/// A stuck checkered from a glitched finish must clear once laps remain again, so Extra
/// `2/2` mid-race does not keep looking finished while live place still moves.
#[test]
fn premature_checkered_latch_clears_while_extras_remain() {
    let _g = session_lock();
    let mut s = live_snap();
    expire_timed_extras(&mut s, 2);
    s.local_speed = 18.0;
    s.standings[0].num_laps = 6;
    cross_line(&mut s, 6, 7); // first extra
    cross_line(&mut s, 7, 8); // second extra — banner 2/2, still racing
    assert_eq!(session_banner(&s).1, "2/2");
    assert_eq!(laps_left(&s), Some(1));
    age_white_wave();
    assert_eq!(ride_to(&mut s, 0.40), DashFlag::None, "mid-lap last extra is bare");

    CHECKERED_LATCH.store(1, Ordering::Relaxed);
    assert_ne!(
        dash_race_flag(&s),
        DashFlag::Checkered,
        "latched checkered must not stick while laps remain"
    );
    assert_eq!(CHECKERED_LATCH.load(Ordering::Relaxed), 0);
    assert_eq!(session_banner(&s).1, "2/2");

    assert_eq!(ride_lap_to_line(&mut s), DashFlag::Checkered, "finish run-in still waves");
    assert_eq!(cross_line(&mut s, 8, 9), DashFlag::Checkered);
    assert_eq!(CHECKERED_LATCH.load(Ordering::Relaxed), 1);
    // True finish: latch holds even if standings keep shuffling.
    s.standings[0].position = 2;
    s.standings[1].position = 1;
    assert_eq!(dash_race_flag(&s), DashFlag::Checkered);
}

/// Two agreeing crossings pin the line down even when `sf_meters` points elsewhere.
#[test]
fn learned_line_overrides_a_wrong_sf_meters() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 0;
    s.session_laps = 6;
    s.session_time_ms = 0;
    s.local_speed = 18.0;
    // The game claims the line is a third of the way round; laps actually tick at 0.75.
    s.sf_meters = 330.0;
    s.standings[0].num_laps = 1;
    s.standings[1].num_laps = 1;
    s.current_lap = 2;
    ride_to(&mut s, 0.50);

    for (laps, lap_num) in [(2, 3), (3, 4)] {
        s.standings[1].num_laps = laps;
        s.current_lap = lap_num;
        ride_to(&mut s, 0.76);
        ride_to(&mut s, 0.50);
    }
    let learned = START_FINISH_FRACTION_LEARNED.load(Ordering::Relaxed);
    assert!(learned > 7_000 && learned < 8_000, "learned {learned}");
    // One stray crossing somewhere else must not move it.
    s.standings[1].num_laps = 4;
    s.current_lap = 5;
    ride_to(&mut s, 0.20);
    assert_eq!(START_FINISH_FRACTION_LEARNED.load(Ordering::Relaxed), learned);
}

#[test]
fn lap_race_banner_uses_lap_count() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 0;
    s.session_laps = 12;
    s.session_time_ms = 0;
    let (icon, text) = session_banner(&s);
    assert_eq!(text, "6 / 12");
    assert_ne!(icon, '\0');
}

#[test]
fn practice_countdown_uses_session_clock() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 0;
    s.session_laps = 0;
    s.session_time_ms = 7 * 60 * 1000;
    s.current_lap = 3;
    s.local_speed = 12.0;
    assert_eq!(session_remain_ms(&s), Some(7 * 60 * 1000));
    assert_eq!(session_banner(&s).1, "07:00");
}

#[test]
fn practice_crash_still_counts_the_crossing() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_kind = -1;
    s.session_length = 40;
    s.session_laps = 0;
    s.local_crashed = 1;
    // Game left completed laps on the last valid lap. RunLap still advanced.
    s.standings[1].num_laps = 2;
    s.current_lap = 4;
    assert_eq!(
        focus_num_laps(&s),
        3,
        "practice must count a crashed crossing from current_lap"
    );
    assert_eq!(standing_num_laps(&s, 12, 2), 3);
    assert_eq!(
        standing_num_laps(&s, 1, 5),
        5,
        "other riders keep the game lap count"
    );
}

#[test]
fn race_does_not_count_ahead_current_lap() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_kind = 7;
    s.session_length = 0;
    s.session_laps = 12;
    s.standings[1].num_laps = 5;
    s.current_lap = 9;
    assert_eq!(
        focus_num_laps(&s),
        5,
        "a glitched current_lap must not move moto laps-done"
    );
}

#[test]
fn warmup_countdown_shows_even_when_race_laps_are_set() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 0;
    s.session_laps = 2;
    s.session_time_ms = 5 * 60 * 1000;
    s.current_lap = 3;
    s.local_speed = 12.0;
    s.local_track_pos = 0.95;
    s.riders[1].track_pos = 0.95;
    assert_eq!(session_banner(&s).1, "05:00");
    assert!(!overtime_active(&s));
    assert_eq!(dash_race_flag(&s), DashFlag::None);
}

#[test]
fn warmup_then_gate_shows_prestart_countdown() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 8;
    s.session_laps = 2;
    s.session_time_ms = 5 * 60 * 1000;
    s.current_lap = 3;
    s.local_speed = 12.0;
    assert_eq!(session_banner(&s).1, "05:00");
    assert_eq!(dash_race_flag(&s), DashFlag::None);

    s.session_time_ms = 400;
    s.local_speed = 0.0;
    let _ = session_remain_ms(&s);
    s.session_time_ms = 30_000;
    s.current_lap = 1;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    assert_eq!(session_banner(&s).1, "00:30");
    assert!(!overtime_active(&s));
    assert_eq!(dash_race_flag(&s), DashFlag::None);
}

#[test]
fn warmup_ten_minutes_does_not_replace_eight_minute_race() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 10;
    s.session_laps = 0;
    s.session_time_ms = 10 * 60 * 1000;
    s.current_lap = 2;
    s.local_speed = 12.0;
    assert_eq!(session_banner(&s).1, "10:00");

    s.session_length = 8;
    s.session_laps = 2;
    s.session_time_ms = 8 * 60 * 1000;
    s.current_lap = 1;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    let remain = session_remain_ms(&s).expect("race length after warmup");
    assert!(
        (remain - 8 * 60 * 1000).abs() < 1_000,
        "expected ~8:00 race time, got {remain}"
    );
    assert_eq!(session_banner(&s).1, "08:00");
}

#[test]
fn gate_countdown_uses_short_clock_before_green() {
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
    assert_eq!(session_banner(&s).1, "00:30");
}

#[test]
fn two_minute_prestart_shows_instead_of_race_length() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 10;
    s.session_laps = 2;
    s.session_time_ms = 10 * 60 * 1000;
    s.current_lap = 1;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    assert_eq!(session_banner(&s).1, "10:00");
    s.session_time_ms = 120_000;
    s.local_speed = 4.0;
    assert_eq!(session_remain_ms(&s), Some(120_000));
    assert_eq!(session_banner(&s).1, "02:00");
    assert!(!overtime_active(&s));
    assert_eq!(dash_race_flag(&s), DashFlag::None);
}

#[test]
fn mid_race_countdown_decreases() {
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
    s.local_speed = 18.0;
    s.current_lap = 2;
    let _ = session_remain_ms(&s);
    s.session_time_ms = 8 * 60 * 1000 - 1_000;
    let first = session_remain_ms(&s).expect("green");
    s.session_time_ms = 7 * 60 * 1000;
    let second = session_remain_ms(&s).expect("mid race");
    assert!(second < first, "countdown should drop, {first} -> {second}");
    assert_eq!(session_banner(&s).1, "07:00");
    assert!(!overtime_active(&s));
}

#[test]
fn timed_race_keeps_remaining_clock_in_the_second_half() {
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
    s.local_speed = 18.0;
    s.current_lap = 2;
    let _ = session_remain_ms(&s);
    s.session_time_ms = 8 * 60 * 1000 - 1_000;
    let _ = session_remain_ms(&s);
    s.session_time_ms = 3 * 60 * 1000;
    let remain = session_remain_ms(&s).expect("second-half countdown");
    assert!(
        (remain - 3 * 60 * 1000).abs() < 1_000,
        "expected ~3:00 left, got {remain}"
    );
    assert_eq!(session_banner(&s).1, "03:00");
}

#[test]
fn timed_race_mid_countdown_glitch_to_length_keeps_time() {
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
    s.local_speed = 18.0;
    s.current_lap = 6;
    s.standings[0].num_laps = 5;
    s.standings[1].num_laps = 5;
    let _ = session_remain_ms(&s);
    s.session_time_ms = 8 * 60 * 1000 - 1_000;
    let _ = session_remain_ms(&s);
    s.session_time_ms = 3 * 60 * 1000;
    let _ = session_remain_ms(&s);
    s.session_time_ms = 8 * 60 * 1000;
    let remain = session_remain_ms(&s).expect("still a countdown");
    assert!(remain > 60_000, "must not switch to laps mid-race, got {remain}");
    assert!(!overtime_active(&s));
    assert_eq!(session_banner(&s).1, "03:00");
}

#[test]
fn remaining_ms_under_100s_does_not_jump_to_session_length() {
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
    s.standings[0].num_laps = 2;
    s.standings[1].num_laps = 2;
    let _ = session_remain_ms(&s);
    s.session_time_ms = 8 * 60 * 1000 - 1_000;
    let _ = session_remain_ms(&s);
    s.session_time_ms = 100_982;
    let _ = session_remain_ms(&s);
    s.session_time_ms = 99_992;
    let remain = session_remain_ms(&s).expect("keep ~1:40");
    assert!(
        (remain - 99_992).abs() < 1_000,
        "01:40 remaining must not snap to 08:00, got {remain}"
    );
    assert_eq!(session_banner(&s).1, "01:39");
    s.session_time_ms = 99_992_000;
    let remain = session_remain_ms(&s).expect("ignore second-scaled spike");
    assert!(remain < 120_000, "garbage 99992s clock must not show 08:00, got {remain}");
    assert!(!overtime_active(&s));
}

#[test]
fn sighting_jump_to_race_length_still_allows_gate() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 8;
    s.session_laps = 0;
    s.session_time_ms = 120_000;
    s.current_lap = 1;
    s.local_speed = 16.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    assert_eq!(session_banner(&s).1, "02:00");
    s.session_laps = 1;
    s.session_time_ms = 8 * 60 * 1000;
    assert_eq!(session_banner(&s).1, "08:00");
    assert!(!overtime_active(&s));
    s.session_time_ms = 30_000;
    s.local_speed = 0.0;
    assert_eq!(session_banner(&s).1, "08:00");
    s.session_time_ms = 8 * 60 * 1000;
    s.local_speed = 18.0;
    let _ = session_remain_ms(&s);
    s.session_time_ms = 8 * 60 * 1000 - 1_000;
    s.current_lap = 2;
    let remain = session_remain_ms(&s).expect("race ticking");
    assert!(remain < 8 * 60 * 1000);
    assert_eq!(session_banner(&s).1, "07:59");
    s.session_time_ms = 400;
    assert_eq!(session_remain_ms(&s), Some(0));
    assert_eq!(session_banner(&s).1, "0/1");
}

#[test]
fn gate_clock_stays_countdown_even_if_lap_already_advanced() {
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
    assert_eq!(session_banner(&s).1, "00:30");
    s.session_time_ms = 25_740;
    s.current_lap = 2;
    s.local_speed = 22.0;
    s.standings[1].num_laps = 1;
    let remain = session_remain_ms(&s).expect("still a gate clock");
    assert!(
        (remain - 25_740).abs() < 1_000,
        "00:25 gate must not become elapsed 07:34, got {remain}"
    );
    assert_eq!(session_banner(&s).1, "00:25");
    assert!(!overtime_active(&s));
}

#[test]
fn long_timed_race_does_not_expire_on_green_clock_sweep() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 8;
    s.session_laps = 2;
    s.session_time_ms = 43_940;
    s.current_lap = 1;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    assert_eq!(session_banner(&s).1, "00:43");
    s.session_time_ms = 1_970;
    assert_eq!(session_banner(&s).1, "00:01");
    s.session_time_ms = 980_000;
    assert_eq!(session_banner(&s).1, "08:00");
    assert!(!overtime_active(&s));
    s.session_time_ms = 7_990;
    assert_eq!(session_banner(&s).1, "08:00");
    s.session_time_ms = 8 * 60 * 1000;
    let _ = session_remain_ms(&s);
    s.session_time_ms = 8 * 60 * 1000 - 1_000;
    s.local_speed = 18.0;
    assert_eq!(session_banner(&s).1, "07:59");
    s.session_time_ms = 1_075;
    let _ = session_remain_ms(&s);
    s.session_time_ms = 505_000;
    assert_eq!(session_remain_ms(&s), Some(0));
    assert_eq!(session_banner(&s).1, "0/2");
}

#[test]
fn three_lap_race_ignores_practice_session_length() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 40;
    s.session_laps = 0;
    s.session_time_ms = 40 * 60 * 1000;
    s.current_lap = 1;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    assert_eq!(session_banner(&s).1, "40:00");
    s.session_length = 7;
    s.session_laps = 3;
    s.session_time_ms = 43_940;
    s.current_lap = 3;
    assert_eq!(session_banner(&s).1, "00:43");
    s.session_time_ms = 1_970;
    assert_eq!(session_banner(&s).1, "00:01");
    s.session_time_ms = 980_000;
    s.local_speed = 4.0;
    assert_eq!(session_banner(&s).1, "1 / 3");
    assert!(!overtime_active(&s));
    s.standings[1].num_laps = 1;
    s.current_lap = 2;
    assert_eq!(session_banner(&s).1, "2 / 3");
    s.standings[1].num_laps = 2;
    s.current_lap = 3;
    assert_eq!(session_banner(&s).1, "3 / 3");
}

/// 3-lap moto, length unset. After the gate the plugin can dump a 5–30 min clock
/// (MXB Test Track jumped 00:01 → 16:20) and then count elapsed. That is still `1 / 3`.
#[test]
fn three_lap_unset_length_after_gate_shows_laps_not_elapsed_clock() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 0;
    s.session_laps = 3;
    s.session_time_ms = 45_350;
    s.current_lap = 1;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    assert_eq!(session_banner(&s).1, "00:45");
    s.session_time_ms = 1_970;
    assert_eq!(session_banner(&s).1, "00:01");
    s.session_time_ms = 980_000;
    s.local_speed = 4.0;
    assert_eq!(session_banner(&s).1, "1 / 3", "glitched 16:20 after the gate is not a timed clock");
    assert!(is_lap_race(&s));
    s.session_time_ms = 453_000;
    s.standings[1].num_laps = 1;
    s.current_lap = 2;
    assert_eq!(session_banner(&s).1, "2 / 3", "elapsed clock mid-moto must not replace laps");
}

#[test]
fn eight_minute_race_with_three_extras_is_timed() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 8;
    s.session_laps = 3;
    s.session_time_ms = 49_970;
    s.current_lap = 1;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    assert_eq!(session_banner(&s).1, "00:49");
    s.session_time_ms = 8 * 60 * 1000;
    assert_eq!(session_banner(&s).1, "08:00");
    s.session_time_ms = 8 * 60 * 1000 - 1_000;
    s.local_speed = 16.0;
    assert_eq!(session_banner(&s).1, "07:59");
    assert!(!overtime_active(&s));
}

#[test]
fn leftover_practice_length_does_not_make_a_lap_race() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 40;
    s.session_laps = 3;
    s.session_time_ms = 49_730;
    s.current_lap = 1;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    assert_eq!(session_banner(&s).1, "00:49");
    s.session_time_ms = 48_980;
    assert_eq!(session_banner(&s).1, "00:48");
    s.session_time_ms = 8 * 60 * 1000;
    assert_eq!(session_banner(&s).1, "08:00");
    s.session_time_ms = 8 * 60 * 1000 - 1_000;
    s.local_speed = 16.0;
    assert_eq!(session_banner(&s).1, "07:59");
}

#[test]
fn lap_race_elapsed_clock_after_gate_shows_laps() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 7;
    s.session_laps = 4;
    s.session_time_ms = 40_060;
    s.current_lap = 1;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    assert_eq!(session_banner(&s).1, "00:40");
    s.session_time_ms = 39_820;
    assert_eq!(session_banner(&s).1, "00:39");
    s.session_time_ms = 1_990;
    assert_eq!(session_banner(&s).1, "00:01");
    s.session_time_ms = 1_021;
    s.local_speed = 4.0;
    assert_eq!(session_banner(&s).1, "1 / 4");
    s.session_time_ms = 2_011;
    assert_eq!(session_banner(&s).1, "1 / 4");
    s.session_time_ms = 60_001;
    s.local_speed = 0.0;
    assert_eq!(session_banner(&s).1, "1 / 4");
}

#[test]
fn lap_race_keeps_gate_countdown_after_one_moving_frame() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 7;
    s.session_laps = 3;
    s.session_time_ms = 50_000;
    s.current_lap = 2;
    s.local_speed = 16.3;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    assert_eq!(session_banner(&s).1, "00:50");
    s.local_speed = 0.0;
    s.session_time_ms = 16_640;
    assert_eq!(session_banner(&s).1, "00:16");
    s.session_time_ms = 15_980;
    assert_eq!(session_banner(&s).1, "00:15");
    s.session_time_ms = 1_990;
    assert_eq!(session_banner(&s).1, "00:01");
    s.local_speed = 13.7;
    s.session_time_ms = 50_000;
    assert_eq!(session_banner(&s).1, "1 / 3");
}

#[test]
fn six_minute_plus_two_unset_length_keeps_countdown_after_gate() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 0;
    s.session_laps = 2;
    s.session_time_ms = 6 * 60 * 1000;
    s.current_lap = 1;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    assert_eq!(session_banner(&s).1, "06:00");
    assert!(!is_lap_race(&s));
    // Gate board after we already saw the race clock.
    s.session_time_ms = 10_000;
    let gate = session_banner(&s).1;
    assert!(gate.starts_with("00:"), "gate board, got {gate}");
    // Live race resumes — must not become a 2-lap moto.
    s.session_time_ms = 5 * 60 * 1000 + 30_000;
    s.local_speed = 18.0;
    s.current_lap = 2;
    s.standings[0].num_laps = 1;
    s.standings[1].num_laps = 1;
    let text = session_banner(&s).1;
    assert!(!is_lap_race(&s));
    assert!(
        text.contains(':') && !text.contains('/'),
        "timed +2 must keep countdown, got {text}"
    );
    assert!(!text.contains("+"), "live timed clock must not show +extras, got {text}");
}

#[test]
fn lap_motos_two_to_eight_keep_count_after_gate_glitch() {
    let _g = session_lock();
    // Unset length, leftover 7-min junk, leftover 8:00 on a 4-lap, leftover 10:00 on a 5-lap.
    for (laps, length) in [
        (2, 0),
        (2, 7),
        (3, 0),
        (3, 7),
        (4, 0),
        (4, 8),
        (5, 0),
        (5, 10),
        (6, 0),
        (8, 0),
        (10, 0),
    ] {
        lap_moto_after_gate_glitch_shows_laps(laps, length);
    }
}

#[test]
fn timed_races_five_to_sixty_with_one_to_four_extras_keep_countdown() {
    let _g = session_lock();
    for minutes in [5, 6, 8, 10, 12, 15, 20, 25, 30, 35, 40, 45, 50, 60] {
        for extras in 1..=4 {
            timed_plus_n_keeps_countdown(minutes, extras);
        }
    }
}

#[test]
fn hour_plus_four_unset_length_keeps_countdown_after_gate() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 0;
    s.session_laps = 4;
    s.session_time_ms = 60 * 60 * 1000;
    s.current_lap = 1;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    let start = session_banner(&s).1;
    assert!(!is_lap_race(&s), "60:00 +4 unset length is timed, got {start}");
    assert!(
        start.contains(':') && !start.contains('/'),
        "60:00 +4 prestart must be a clock, got {start}"
    );
    s.session_time_ms = 10_000;
    let gate = session_banner(&s).1;
    assert!(gate.starts_with("00:"), "gate board, got {gate}");
    s.session_time_ms = 60 * 60 * 1000 - 30_000;
    s.local_speed = 18.0;
    s.current_lap = 2;
    s.standings[0].num_laps = 1;
    s.standings[1].num_laps = 1;
    let text = session_banner(&s).1;
    assert!(!is_lap_race(&s));
    assert!(
        text.contains(':') && !text.contains('/'),
        "60:00 +4 must keep countdown, got {text}"
    );
}

#[test]
fn practice_zero_does_not_restore_session_length() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 40;
    s.session_laps = 0;
    s.session_time_ms = 180_000;
    s.current_lap = 5;
    s.local_speed = 16.0;
    assert_eq!(session_banner(&s).1, "03:00");
    s.session_time_ms = 1_470;
    s.local_speed = 19.0;
    assert_eq!(session_banner(&s).1, "00:01");
    s.session_time_ms = 40 * 60 * 1000;
    assert_eq!(session_banner(&s).1, "", "warmup over — no sticky 00:00");
    // Junk 00:30 after expiry must stay blank.
    s.session_time_ms = 30_000;
    assert_eq!(session_banner(&s).1, "");
}

#[test]
fn warmup_expired_hides_clock() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 10;
    s.session_laps = 0;
    s.session_time_ms = 10 * 60 * 1000;
    s.current_lap = 2;
    s.local_speed = 14.0;
    assert_eq!(session_banner(&s).1, "10:00");
    s.session_time_ms = 2_000;
    assert_eq!(session_banner(&s).1, "00:02");
    s.session_time_ms = 400;
    assert_eq!(session_banner(&s).1, "", "practice at zero hides the clock");
    s.session_time_ms = 30_000;
    assert_eq!(session_banner(&s).1, "", "no sticky 00:30 after warmup ends");
}

#[test]
fn warmup_mid_clock_snap_to_thirty_hides_clock() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 10;
    s.session_laps = 0;
    s.session_time_ms = 2 * 60 * 1000;
    s.current_lap = 2;
    s.local_speed = 14.0;
    assert_eq!(session_banner(&s).1, "02:00");
    // Some servers jump straight to a frozen start board without counting to zero.
    s.session_time_ms = 30_000;
    assert_eq!(session_banner(&s).1, "", "no sticky 00:30 after warmup ends mid-clock");
}

#[test]
fn warmup_kind_with_leaked_laps_frozen_thirty_hides_clock() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_kind = 5;
    s.session_length = 10;
    s.session_laps = 2;
    s.session_time_ms = 2 * 60 * 1000;
    s.current_lap = 2;
    s.local_speed = 14.0;
    assert!(is_warmup(&s));
    assert_eq!(session_banner(&s).1, "02:00");
    s.session_time_ms = 30_000;
    s.local_speed = 0.0;
    assert_eq!(
        session_banner(&s).1,
        "",
        "kind 5 must not pin leaked extras as a frozen gate board"
    );
}

#[test]
fn lap_race_holds_laps_between_prestart_and_green() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 7;
    s.session_laps = 4;
    s.session_time_ms = 40_060;
    s.current_lap = 1;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    assert_eq!(session_banner(&s).1, "00:40");
    s.session_time_ms = 1_990;
    assert_eq!(session_banner(&s).1, "00:01");
    s.session_time_ms = 50_000;
    assert_eq!(session_banner(&s).1, "1 / 4");
    s.session_time_ms = 140_000;
    assert_eq!(session_banner(&s).1, "1 / 4");
    s.local_speed = 16.0;
    s.session_time_ms = 2_000;
    assert_eq!(session_banner(&s).1, "1 / 4");
}

#[test]
fn five_lap_race_ignores_leftover_ten_minute_warmup() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 600_000;
    s.session_laps = 5;
    s.session_time_ms = 28_730;
    s.current_lap = 0;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    assert_eq!(session_banner(&s).1, "00:28");
    s.session_time_ms = 1_021;
    assert_eq!(session_banner(&s).1, "00:01");
    s.session_time_ms = 395_182;
    s.current_lap = 5;
    s.local_speed = 16.0;
    s.standings[1].num_laps = 4;
    assert_eq!(session_banner(&s).1, "5 / 5");
    assert!(!overtime_active(&s));
}

#[test]
fn four_lap_race_ignores_leftover_eight_minute_length() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 480_000;
    s.session_laps = 4;
    s.session_time_ms = 20_000;
    s.current_lap = 1;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    assert_eq!(session_banner(&s).1, "00:20");
    s.session_time_ms = 1_021;
    assert_eq!(session_banner(&s).1, "00:01");
    s.session_time_ms = 45_000;
    assert_eq!(session_banner(&s).1, "1 / 4");
    s.session_time_ms = 1_027;
    s.local_speed = 8.0;
    assert_eq!(session_banner(&s).1, "1 / 4");
    s.session_time_ms = 480_000;
    assert_eq!(session_banner(&s).1, "1 / 4");
    assert!(!overtime_active(&s));
}

/// After the live clock has started ticking, a republish of session length must not
/// snap the dash back to full race time (07:58 → 08:00).
#[test]
fn armed_countdown_ignores_near_full_length_republish() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 8;
    s.session_laps = 1;
    s.session_time_ms = 43_980;
    s.current_lap = 1;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    assert_eq!(session_banner(&s).1, "00:43");
    s.session_time_ms = 1_980;
    assert_eq!(session_banner(&s).1, "00:01");
    s.session_time_ms = 8 * 60 * 1000;
    assert_eq!(session_banner(&s).1, "08:00");
    s.session_time_ms = 8 * 60 * 1000 - 2_000;
    s.local_speed = 18.0;
    assert_eq!(session_banner(&s).1, "07:58");
    // Game republishes full length while still inside the near-full 30 s band.
    s.session_time_ms = 8 * 60 * 1000;
    assert_eq!(
        session_banner(&s).1,
        "07:58",
        "near-full republish must not snap back to 08:00"
    );
    s.session_time_ms = 8 * 60 * 1000 - 3_000;
    assert_eq!(session_banner(&s).1, "07:57");
    assert!(!overtime_active(&s));
}

#[test]
fn practice_countdown_holds_through_eight_minutes() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 15;
    s.session_laps = 0;
    s.session_time_ms = 15 * 60 * 1000;
    s.current_lap = 3;
    s.local_speed = 12.0;
    assert_eq!(session_banner(&s).1, "15:00");
    s.session_time_ms = 8 * 60 * 1000;
    let remain = session_remain_ms(&s).expect("practice remaining");
    assert!(
        (remain - 8 * 60 * 1000).abs() < 1_000,
        "expected ~8:00 left, got {remain}"
    );
    assert_eq!(session_banner(&s).1, "08:00");
    assert!(!overtime_active(&s));
}

#[test]
fn practice_remaining_counts_through_three_minutes() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 10;
    s.session_laps = 0;
    s.session_time_ms = 10 * 60 * 1000;
    s.current_lap = 2;
    s.local_speed = 19.0;
    assert_eq!(session_banner(&s).1, "10:00");
    s.session_time_ms = 180_000;
    assert_eq!(session_banner(&s).1, "03:00");
    s.session_time_ms = 179_000;
    assert_eq!(session_banner(&s).1, "02:59");
    s.session_time_ms = 104_700;
    assert_eq!(session_banner(&s).1, "01:44");
    s.local_speed = 0.0;
    std::thread::sleep(std::time::Duration::from_millis(800));
    assert_eq!(session_banner(&s).1, "01:44");
}

#[test]
fn timed_race_switches_to_laps_when_clock_jumps_back_to_length() {
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
    s.local_speed = 18.0;
    s.current_lap = 6;
    s.standings[0].num_laps = 5;
    s.standings[1].num_laps = 5;
    let _ = session_remain_ms(&s);
    s.session_time_ms = 8 * 60 * 1000 - 1_000;
    let _ = session_remain_ms(&s);
    s.session_time_ms = 1_075;
    let _ = session_remain_ms(&s);
    s.session_time_ms = 8_000;
    let _ = session_remain_ms(&s);
    s.session_time_ms = 505_000;
    assert_eq!(session_remain_ms(&s), Some(0));
    assert!(overtime_active(&s));
    assert_eq!(session_banner(&s).1, "0/2");
    s.standings[0].num_laps = 6;
    s.current_lap = 7;
    s.standings[1].num_laps = 6;
    assert_eq!(session_banner(&s).1, "1/2", "first extra cross starts the extra");
    s.standings[1].num_laps = 7;
    s.current_lap = 8;
    assert_eq!(session_banner(&s).1, "2/2");
}

#[test]
fn approaching_sf_uses_meters_before_the_line() {
    let mut s = live_snap();
    s.track_length = 1000.0;
    s.sf_meters = 0.0;
    s.local_track_pos = 0.97;
    s.riders[1].track_pos = 0.97;
    s.has_telemetry = 1;
    assert!(approaching_start_finish(&s));
    assert!(approaching_finish(&s));
    s.local_track_pos = 0.70;
    s.riders[1].track_pos = 0.70;
    assert!(!approaching_start_finish(&s));
    assert!(!approaching_finish(&s));
    s.local_track_pos = 0.001;
    s.riders[1].track_pos = 0.001;
    assert!(!approaching_start_finish(&s));
    assert!(!approaching_finish(&s));
}

#[test]
fn warmup_then_five_minute_plus_one_shows_countdown_not_plus_one() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    // Warmup: length unset, live 10:00 — sets SAW (and may arm).
    s.session_length = 0;
    s.session_laps = 0;
    s.session_time_ms = 10 * 60 * 1000;
    s.current_lap = 2;
    s.local_speed = 14.0;
    s.standings[0].num_laps = 1;
    s.standings[1].num_laps = 1;
    assert_eq!(session_banner(&s).1, "10:00");
    s.session_time_ms = 9 * 60 * 1000;
    assert_eq!(session_banner(&s).1, "09:00");

    // Race 5:00 +1 — must not inherit warmup SAW/ARMED as already-expired extras.
    s.session_length = 5;
    s.session_laps = 1;
    s.session_time_ms = 5 * 60 * 1000;
    s.current_lap = 1;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    assert_eq!(session_banner(&s).1, "05:00");
    assert!(!overtime_active(&s));
    assert_ne!(session_banner(&s).1, "0/1");

    s.session_time_ms = 5 * 60 * 1000 - 1_000;
    s.local_speed = 18.0;
    s.current_lap = 2;
    s.standings[0].num_laps = 1;
    s.standings[1].num_laps = 1;
    let remain = session_remain_ms(&s).expect("race ticking");
    assert!(remain < 5 * 60 * 1000);
    assert!(remain > 4 * 60 * 1000);
    assert_eq!(session_banner(&s).1, "04:59");
    assert!(!overtime_active(&s));
}

#[test]
fn race_store_tick_once_matches_banner_for_five_plus_one() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 0;
    s.session_laps = 0;
    s.session_time_ms = 10 * 60 * 1000;
    s.current_lap = 2;
    s.local_speed = 14.0;
    let _ = RaceStore::tick(&s);
    s.session_time_ms = 9 * 60 * 1000;
    let _ = RaceStore::tick(&s);

    s.session_length = 5;
    s.session_laps = 1;
    s.session_time_ms = 5 * 60 * 1000;
    s.current_lap = 1;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    let first = RaceStore::tick(&s);
    assert_eq!(first.clock.banner.1, "05:00");
    assert!(!first.clock.expired);

    s.session_time_ms = 5 * 60 * 1000 - 2_000;
    s.local_speed = 18.0;
    s.current_lap = 2;
    s.standings[0].num_laps = 1;
    s.standings[1].num_laps = 1;
    let a = RaceStore::tick(&s);
    let b = RaceStore::get();
    assert_eq!(a.clock.banner.1, b.clock.banner.1);
    assert_eq!(a.clock.banner.1, "04:58");
    // Second get without another tick must not require re-mutating remain.
    assert_eq!(race_progress_text(&s), "04:58");
    assert!(!overtime_active(&s));
}

