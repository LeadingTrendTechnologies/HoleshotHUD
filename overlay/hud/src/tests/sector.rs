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
    assert_eq!(r.time_ms, 18_000);
    let line = pos_for(S1);
    s.current_lap_ms = 26_000;
    s.local_track_pos = line;
    s.sector_cur = [24_500, 0, 0];
    tick(&s);
    let frozen = row(&s, 0, true);
    assert!(!frozen.live);
    assert!(!frozen.fresh);
    assert_eq!(frozen.time_ms, 24_500);
    assert_eq!(frozen.delta_ms, 24_500 - tape_at(line));
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
    assert_eq!(r.delta_ms, 23_000 - tape_at(line));
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
    assert_eq!(row(&s, 0, true).delta_ms, 26_000 - tape_at(line));
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
