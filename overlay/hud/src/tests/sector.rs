use super::*;
use crate::shm::write_name;
use std::time::{SystemTime, UNIX_EPOCH};

fn snap() -> Snapshot {
    let mut s = Snapshot::default();
    s.on_track = 1;
    s.has_telemetry = 1;
    s.sector_count = 3;
    s.track_length = 1_600.0;
    write_name(&mut s.track_name, "LiveTrack");
    s
}

const LAP: i32 = 72_000;
const S1: i32 = 24_000;

fn linear_bins(lap: i32) -> [i32; BINS] {
    let mut b = [0; BINS];
    for i in 0..BINS {
        b[i] = (lap as i64 * i as i64 / BINS as i64) as i32;
        if i > 0 && b[i] <= 0 {
            b[i] = 1;
        }
    }
    b[0] = 1;
    b
}

fn seed_tape() {
    track_pb::bind("LiveTrack", "");
    track_pb::commit_tape("LiveTrack", "", LAP, linear_bins(LAP));
    track_pb::commit_sector("LiveTrack", "", 0, S1);
    track_pb::commit_sector("LiveTrack", "", 1, S1);
    track_pb::commit_sector("LiveTrack", "", 2, S1);
}

fn pos_for(ms: i32) -> f32 {
    ms as f32 / LAP as f32
}

fn tape_at(pos: f32) -> i32 {
    track_pb::time_at(&linear_bins(LAP), pos).unwrap()
}

fn tmp() -> std::sync::MutexGuard<'static, ()> {
    let lock = track_pb::exclusive_test();
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("mxbo-sec-{n}"));
    track_pb::set_store_dir(dir);
    *live() = SectorEngine::new();
    lock
}

#[test]
fn split_fracs_can_be_pinned() {
    let _lock = tmp();
    set_split_fracs([0.31, 0.64]);
    assert!((split_fracs()[0] - 0.31).abs() < 1e-6);
    assert!((split_fracs()[1] - 0.64).abs() < 1e-6);
    let starts = sector_starts();
    assert_eq!(starts[0], (0.0, "S1"));
    assert!((starts[1].0 - 0.31).abs() < 1e-6 && starts[1].1 == "S2");
    assert!((starts[2].0 - 0.64).abs() < 1e-6 && starts[2].1 == "S3");
}

#[test]
fn hero_is_current_sector() {
    let mut s = snap();
    s.current_lap_ms = 5_000;
    assert_eq!(hero_index(&s), 0);
    s.sector_cur = [24_000, 0, 0];
    assert_eq!(hero_index(&s), 1);
    s.sector_cur = [24_000, 25_000, 0];
    assert_eq!(hero_index(&s), 2);
    s.current_lap_ms = 0;
    s.sector_cur = [0, 0, 0];
    s.sector_last_lap = [24_000, 25_000, 23_000];
    assert_eq!(hero_index(&s), 2);
}

#[test]
fn live_s1_then_freeze_when_leaving() {
    let _lock = tmp();
    seed_tape();
    let mut s = snap();
    let here = pos_for(20_000);
    s.current_lap_ms = 18_000;
    s.local_track_pos = here;
    tick(&s);
    let r = row(&s, 0, true);
    assert!(r.fresh);
    assert!(r.live);
    assert_eq!(r.delta_ms, 18_000 - tape_at(here));
    assert_eq!(r.time_ms, S1);
    let line = pos_for(S1);
    s.current_lap_ms = 26_000;
    s.local_track_pos = line;
    s.sector_cur = [24_500, 0, 0];
    tick(&s);
    let frozen = row(&s, 0, true);
    assert!(!frozen.live);
    assert!(!frozen.fresh);
    assert_eq!(frozen.time_ms, 24_500);
    assert_eq!(frozen.delta_ms, 500);
    let s2 = row(&s, 1, true);
    assert!(s2.fresh);
    assert!(s2.live);
}

#[test]
fn live_s2_is_vs_location_in_sector() {
    let _lock = tmp();
    seed_tape();
    let mut s = snap();
    let line = pos_for(S1);
    s.current_lap_ms = 25_000;
    s.local_track_pos = line;
    s.sector_cur = [25_000, 0, 0];
    tick(&s);
    assert!(row(&s, 0, true).delta_ms > 500);
    let half = 0.5;
    s.current_lap_ms = 25_000 + (tape_at(half) - tape_at(line));
    s.local_track_pos = half;
    tick(&s);
    let s2 = row(&s, 1, true);
    assert!(s2.fresh);
    assert!(s2.live);
    assert!(
        s2.delta_ms.abs() <= 20,
        "S2 should ignore the S1 deficit, got {}",
        s2.delta_ms
    );
}

#[test]
fn new_pb_freeze_is_not_zero() {
    let _lock = tmp();
    seed_tape();
    let mut s = snap();
    let line = pos_for(S1);
    s.current_lap_ms = 23_000;
    s.local_track_pos = line;
    s.sector_cur = [23_000, 0, 0];
    s.sector_best = [23_000, 0, 0];
    tick(&s);
    let r = row(&s, 0, true);
    assert_eq!(r.delta_ms, 23_000 - S1);
    assert_ne!(r.delta_ms, 0);
    assert_eq!(track_pb::bind("LiveTrack", "").sectors[0], 23_000);
}

#[test]
fn slower_split_does_not_replace_saved() {
    let _lock = tmp();
    seed_tape();
    let mut s = snap();
    let line = pos_for(S1);
    s.current_lap_ms = 26_000;
    s.local_track_pos = line;
    s.sector_cur = [26_000, 0, 0];
    tick(&s);
    assert_eq!(track_pb::bind("LiveTrack", "").sectors[0], 24_000);
    assert_eq!(row(&s, 0, true).delta_ms, 2_000);
}

#[test]
fn live_off_waits_until_split() {
    let _lock = tmp();
    seed_tape();
    let mut s = snap();
    s.current_lap_ms = 18_000;
    s.local_track_pos = pos_for(20_000);
    tick(&s);
    let hidden = row(&s, 0, false);
    assert!(hidden.fresh);
    assert!(!hidden.live);
    assert!(hidden.pending);
    let shown = row(&s, 0, true);
    assert!(shown.live);
    assert!(!shown.pending);
    let line = pos_for(S1);
    s.current_lap_ms = 24_500;
    s.local_track_pos = line;
    s.sector_cur = [24_500, 0, 0];
    tick(&s);
    let frozen = row(&s, 0, false);
    assert!(!frozen.pending);
    assert!(!frozen.live);
    assert_eq!(frozen.time_ms, 24_500);
    let s2 = row(&s, 1, false);
    assert!(s2.fresh);
    assert!(s2.pending);
}

#[test]
fn three_ms_picks_duration_when_closer_to_s1() {
    assert_eq!(three_ms(24_000, 25_000, 72_000), 23_000);
    assert_eq!(three_ms(24_000, 49_000, 72_000), 23_000);
    assert_eq!(three_ms(0, 25_000, 72_000), 0);
    assert_eq!(three_ms(24_000, 25_000, 0), 0);
}

#[test]
fn split_duration_reads_cumulative_or_raw() {
    assert_eq!(split_duration(0, [24_000, 49_000, 0], 0), 24_000);
    assert_eq!(split_duration(1, [24_000, 49_000, 0], 0), 25_000);
    assert_eq!(split_duration(1, [24_000, 20_000, 0], 0), 20_000);
    assert_eq!(split_duration(2, [24_000, 25_000, 0], 72_000), 23_000);
    assert_eq!(split_duration(2, [24_000, 25_000, 22_000], 72_000), 22_000);
}

#[test]
fn plugin_delta_is_preferred_over_tape() {
    let _lock = tmp();
    seed_tape();
    let mut s = snap();
    s.current_lap_ms = 40_000;
    s.sector_cur = [24_000, 0, 0];
    s.sector_delta_valid = 0b001;
    s.sector_delta = [123, 0, 0];
    assert_eq!(row(&s, 0, true).delta_ms, 123);
    assert_eq!(row(&s, 0, true).time_ms, 24_000);
}

fn with_bike(s: &mut Snapshot, bike: &str) {
    s.local_race_num = 1;
    s.standing_count = 1;
    s.standings[0].race_num = 1;
    write_name(&mut s.standings[0].bike, bike);
}

#[test]
fn other_bike_is_not_compared() {
    let _lock = tmp();
    track_pb::commit_tape("LiveTrack", "YZ450F", LAP, linear_bins(LAP));
    track_pb::commit_sector("LiveTrack", "YZ450F", 0, S1);
    let mut s = snap();
    with_bike(&mut s, "YZ250F");
    s.current_lap_ms = 18_000;
    s.local_track_pos = pos_for(20_000);
    tick(&s);
    let r = row(&s, 0, true);
    assert!(r.live);
    assert!(!r.has_delta, "250 must not use the 450 tape");
    assert_eq!(track_pb::bind("LiveTrack", "YZ250F").lap_ms, 0);
    assert_eq!(track_pb::bind("LiveTrack", "YZ450F").lap_ms, LAP);
}

#[test]
fn same_class_uses_saved_tape() {
    let _lock = tmp();
    track_pb::commit_tape("LiveTrack", "YZ250F", LAP, linear_bins(LAP));
    track_pb::commit_sector("LiveTrack", "YZ250F", 0, S1);
    let mut s = snap();
    with_bike(&mut s, "CRF250R");
    let here = pos_for(20_000);
    s.current_lap_ms = 18_000;
    s.local_track_pos = here;
    tick(&s);
    let r = row(&s, 0, true);
    assert!(r.has_delta, "Honda 250 should use the Yamaha 250 tape");
    assert_eq!(r.delta_ms, 18_000 - tape_at(here));
}

fn origin_bins(sf: f32) -> [i32; BINS] {
    let mut b = [0; BINS];
    for i in 0..BINS {
        let from_sf = (i as f32 / BINS as f32 - sf).rem_euclid(1.0);
        b[i] = 200 + (from_sf * LAP as f32) as i32;
    }
    b
}

fn from_sf_ms(pos: f32, sf: f32) -> i32 {
    200 + ((pos - sf).rem_euclid(1.0) * LAP as f32) as i32
}

#[test]
fn origin_wrap_mid_s3_keeps_live_delta() {
    let _lock = tmp();
    let sf = 0.12;
    let s1_pos = 0.45;
    let s2_pos = 0.78;
    let bins = origin_bins(sf);
    track_pb::commit_tape("Orig", "YZ450F", LAP, bins);
    let s1_ms = from_sf_ms(s1_pos, sf) - 200;
    let s2_ms = from_sf_ms(s2_pos, sf) - from_sf_ms(s1_pos, sf);
    track_pb::commit_sector("Orig", "YZ450F", 0, s1_ms);
    track_pb::commit_sector("Orig", "YZ450F", 1, s2_ms);
    track_pb::commit_sector("Orig", "YZ450F", 2, LAP - s1_ms - s2_ms);

    let mut s = snap();
    write_name(&mut s.track_name, "Orig");
    with_bike(&mut s, "YZ450F");

    s.current_lap_ms = from_sf_ms(0.20, sf);
    s.local_track_pos = 0.20;
    tick(&s);

    s.current_lap_ms = from_sf_ms(s1_pos, sf);
    s.local_track_pos = s1_pos;
    s.sector_cur = [s1_ms, 0, 0];
    tick(&s);

    s.current_lap_ms = from_sf_ms(s2_pos, sf);
    s.local_track_pos = s2_pos;
    s.sector_cur = [s1_ms, s2_ms, 0];
    tick(&s);

    s.current_lap_ms = from_sf_ms(0.90, sf);
    s.local_track_pos = 0.90;
    tick(&s);
    let before = row(&s, 2, true);
    assert!(before.fresh);
    assert!(before.live, "S3 should be live before the origin wrap");
    assert!(
        before.delta_ms.abs() <= 250,
        "on-pace S3 before wrap, got {}",
        before.delta_ms
    );

    s.local_track_pos = 0.04;
    s.current_lap_ms = from_sf_ms(0.04, sf) + 60;
    tick(&s);
    let after = row(&s, 2, true);
    assert_eq!(hero_index(&s), 2);
    assert!(after.live, "centerline origin in S3 is not S/F; S3 must keep ticking");
    assert!(!after.pending);
    assert!(
        after.delta_ms.abs() <= 250,
        "S3 delta must keep moving through origin, got {}",
        after.delta_ms
    );
    assert!(
        (after.delta_ms - before.delta_ms - 60).abs() <= 80,
        "origin wrap must not jump the S3 delta ({} -> {})",
        before.delta_ms,
        after.delta_ms
    );
    assert_eq!(live().freeze_ok & 0b100, 0, "origin wrap must not freeze S3");
}

#[test]
fn session_compare_uses_this_visit_not_saved_best() {
    let _lock = tmp();
    seed_tape();
    let mut s = snap();
    let line = pos_for(S1);
    s.current_lap_ms = 24_500;
    s.local_track_pos = line;
    s.sector_cur = [24_500, 0, 0];
    tick(&s);
    let first = row_vs(&s, 0, true, true);
    assert!(!first.has_delta, "first session split has no session best yet");
    assert!(row_vs(&s, 0, true, false).has_delta);

    s.current_lap_ms = 0;
    s.sector_cur = [0, 0, 0];
    tick(&s);
    s.current_lap_ms = 400;
    tick(&s);
    s.current_lap_ms = 27_500;
    s.local_track_pos = line;
    s.sector_cur = [27_500, 0, 0];
    tick(&s);
    let session = row_vs(&s, 0, true, true);
    let alltime = row_vs(&s, 0, true, false);
    assert!(session.has_delta);
    assert_eq!(session.delta_ms, 3_000);
    assert_eq!(alltime.delta_ms, 3_500);
    assert_ne!(session.delta_ms, alltime.delta_ms);
}

#[test]
fn freeze_delta_is_official_vs_saved_not_tape() {
    let _lock = tmp();
    track_pb::bind("LiveTrack", "");
    track_pb::commit_tape("LiveTrack", "", 80_000, linear_bins(80_000));
    track_pb::commit_sector("LiveTrack", "", 0, S1);
    let mut s = snap();
    let pos = 0.33;
    s.current_lap_ms = 25_000;
    s.local_track_pos = pos;
    s.sector_cur = [24_100, 0, 0];
    tick(&s);
    let frozen = row(&s, 0, true);
    assert!(!frozen.live);
    assert_eq!(frozen.time_ms, 24_100);
    assert_eq!(frozen.delta_ms, 100);
    let tape = track_pb::time_at(&linear_bins(80_000), pos).unwrap();
    assert_ne!(
        frozen.delta_ms,
        24_100 - tape,
        "freeze must not use tape-at-pos vs official"
    );
}

fn finish_lap(s: &mut Snapshot, splits: [i32; 3]) {
    s.current_lap_ms = splits[0] + splits[1] + splits[2];
    s.local_track_pos = 0.99;
    s.sector_cur = splits;
    s.last_lap_ms = s.current_lap_ms;
    s.sector_last_lap = splits;
    tick(s);
    s.current_lap_ms = 0;
    s.sector_cur = [0, 0, 0];
    tick(s);
}

#[test]
fn history_pushes_on_s3_and_shifts() {
    let _lock = tmp();
    seed_tape();
    let mut s = snap();
    finish_lap(&mut s, [24_500, 25_200, 22_300]);
    assert_eq!(history_times()[0], [24_500, 25_200, 22_300]);
    assert_eq!(history_times()[1], [0, 0, 0]);

    s.current_lap_ms = 400;
    tick(&s);
    finish_lap(&mut s, [24_100, 25_000, 22_000]);
    assert_eq!(history_times()[0], [24_100, 25_000, 22_000]);
    assert_eq!(history_times()[1], [24_500, 25_200, 22_300]);

    s.current_lap_ms = 400;
    tick(&s);
    finish_lap(&mut s, [24_000, 24_900, 21_800]);
    assert_eq!(history_times()[0], [24_000, 24_900, 21_800]);
    assert_eq!(history_times()[1], [24_100, 25_000, 22_000]);
    assert_eq!(history_times()[2], [24_500, 25_200, 22_300]);

    s.current_lap_ms = 400;
    tick(&s);
    finish_lap(&mut s, [23_900, 24_800, 21_700]);
    s.current_lap_ms = 400;
    tick(&s);
    finish_lap(&mut s, [23_800, 24_700, 21_600]);
    assert_eq!(history_times()[0], [23_800, 24_700, 21_600]);
    assert_eq!(history_times()[4], [24_500, 25_200, 22_300]);
}

#[test]
fn history_skips_incomplete_and_duplicate() {
    let _lock = tmp();
    seed_tape();
    let mut s = snap();
    s.current_lap_ms = 25_000;
    s.local_track_pos = pos_for(S1);
    s.sector_cur = [24_500, 0, 0];
    tick(&s);
    assert_eq!(history_times()[0], [0, 0, 0]);

    finish_lap(&mut s, [24_500, 25_200, 22_300]);
    finish_lap(&mut s, [24_500, 25_200, 22_300]);
    assert_eq!(history_times()[0], [24_500, 25_200, 22_300]);
    assert_eq!(history_times()[1], [0, 0, 0], "same lap must not shift twice");
}

#[test]
fn history_board_falls_back_to_last_lap() {
    let _lock = tmp();
    let mut s = snap();
    s.sector_last_lap = [24_310, 25_820, 23_090];
    s.sector_best = [24_180, 25_640, 22_910];
    let board = history_board(&s, false, 3);
    assert_eq!(board[0].label, "LAST");
    assert_eq!(board[0].cells[0].time_ms, 24_310);
    assert!(board[0].cells[0].slower);
    assert_eq!(board[1].cells[0].time_ms, 0);
}

#[test]
fn history_board_respects_count() {
    let _lock = tmp();
    set_history([
        [24_000, 25_000, 22_000],
        [24_100, 25_100, 22_100],
        [24_200, 25_200, 22_200],
        [24_300, 25_300, 22_300],
        [24_400, 25_400, 22_400],
    ]);
    let s = snap();
    assert_eq!(history_board(&s, false, 1).len(), 1);
    assert_eq!(history_board(&s, false, 1)[0].label, "LAST");
    let five = history_board(&s, false, 5);
    assert_eq!(five.len(), 5);
    assert_eq!(five[4].label, "-5");
    assert_eq!(five[4].cells[0].time_ms, 24_400);
    assert_eq!(history_board(&s, false, 9).len(), 5);
}

#[test]
fn history_gold_is_fastest_lap_not_always_last() {
    let _lock = tmp();
    set_history([
        [25_000, 26_000, 23_000],
        [24_000, 25_000, 22_000],
        [24_500, 25_500, 22_500],
        [0, 0, 0],
        [0, 0, 0],
    ]);
    let s = snap();
    let board = history_board(&s, false, 3);
    assert!(!board[0].fastest, "slower LAST must not take the gold wash");
    assert!(board[1].fastest, "-2 is the fastest lap in the log");
    assert!(!board[2].fastest);
}

#[test]
fn history_clears_on_track_change() {
    let _lock = tmp();
    seed_tape();
    let mut s = snap();
    finish_lap(&mut s, [24_500, 25_200, 22_300]);
    assert_eq!(history_times()[0][0], 24_500);
    write_name(&mut s.track_name, "OtherTrack");
    s.has_telemetry = 1;
    tick(&s);
    assert_eq!(history_times()[0], [0, 0, 0]);
}
