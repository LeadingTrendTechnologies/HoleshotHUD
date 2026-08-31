use super::*;

fn snap(track: &str, pos: f32, cur_ms: i32, last_ms: i32, lap: i32) -> Snapshot {
    let mut s = Snapshot::default();
    s.on_track = 1;
    s.has_telemetry = 1;
    s.local_speed = 12.0;
    s.local_track_pos = pos;
    s.current_lap_ms = cur_ms;
    s.last_lap_ms = last_ms;
    s.current_lap = lap;
    s.track_length = 1600.0;
    crate::shm::write_name(&mut s.track_name, track);
    s
}

fn with_bike(s: &mut Snapshot, bike: &str) {
    s.local_race_num = 1;
    s.standing_count = 1;
    s.standings[0].race_num = 1;
    crate::shm::write_name(&mut s.standings[0].bike, bike);
}

fn run_lap(eng: &mut DeltaEngine, track: &str, lap_ms: i32, lap_num: i32, crashed: bool) {
    let n = 180;
    for i in 0..n {
        let t = i as f32 / n as f32;
        let mut s = snap(track, t, (lap_ms as f32 * t) as i32, 0, lap_num);
        if i + 1 == n {
            s.local_track_pos = 0.999;
            s.current_lap_ms = lap_ms;
        }
        if crashed && i > n / 2 {
            s.local_crashed = 1;
        }
        eng.tick(&s);
    }
    // Crossing often arrives with last_ms still 0 — match the plugin.
    eng.tick(&snap(track, 0.01, 180, 0, lap_num + 1));
}

#[test]
fn outlap_is_not_recording() {
    let mut eng = DeltaEngine::new();
    let v = eng.tick(&snap("A", 0.4, 20_000, 0, 1));
    assert!(!v.ready);
    assert!(!v.recording);
    assert_eq!(v.delta_ms, 0);
    assert_eq!(eng.current.filled, 0);
}

#[test]
fn recording_starts_after_sf_cross() {
    let mut eng = DeltaEngine::new();
    for i in 0..20 {
        let t = 0.70 + i as f32 * 0.012;
        eng.tick(&snap("A", t.min(0.98), 12_000 + i * 150, 0, 1));
    }
    assert!(!eng.armed);
    assert_eq!(eng.current.filled, 0);
    let cross = eng.tick(&snap("A", 0.02, 15_000, 0, 1));
    assert!(eng.armed);
    assert!(
        eng.current.filled == 0,
        "must not tape the out-lap clock at the line, filled={}",
        eng.current.filled
    );
    assert!(!cross.has_delta);
    let v = eng.tick(&snap("A", 0.05, 400, 0, 2));
    assert!(v.recording);
    assert_eq!(eng.current.filled, 1);
}

#[test]
fn last_lap_at_sf_starts_recording() {
    let mut eng = DeltaEngine::new();
    for i in 0..12 {
        eng.tick(&snap("A", 0.40 + i as f32 * 0.01, 8_000 + i * 200, 0, 1));
    }
    assert!(!eng.armed);
    assert_eq!(eng.current.filled, 0);
    // S/F is not at pos 0. Last-lap time is the crossing.
    eng.tick(&snap("A", 0.28, 120, 14_800, 2));
    assert!(eng.armed, "finish-line last-lap must start the flying lap");
    let v = eng.tick(&snap("A", 0.30, 400, 14_800, 2));
    assert!(v.recording);
    assert_eq!(eng.current.filled, 2);
}

#[test]
fn saved_tape_compares_after_sf_not_at_zero() {
    let mut eng = DeltaEngine::new();
    run_lap(&mut eng, "A", 72_000, 1, false);
    assert!(eng.reference.is_some());
    let first = eng.tick(&snap("A", 0.22, 16_000, 72_000, 2));
    assert!(first.ready);
    assert!(first.has_delta, "bar must move after S/F away from pos 0");
    let t = eng.reference.as_ref().unwrap().time_at(0.22).unwrap();
    assert_eq!(first.delta_ms, 16_000 - t);
    for i in 1..20 {
        let t = 0.22 + i as f32 * 0.01;
        eng.tick(&snap("A", t, 16_000 + i * 400, 72_000, 2));
    }
    let v = eng.tick(&snap("A", 0.45, 32_000, 72_000, 2));
    assert!(v.ready);
    assert!(v.has_delta);
}

#[test]
fn reset_to_pits_waits_for_sf() {
    let mut eng = DeltaEngine::new();
    run_lap(&mut eng, "A", 72_000, 1, false);
    for i in 0..40 {
        let t = i as f32 / 200.0;
        eng.tick(&snap("A", t, 200 + i * 400, 0, 2));
    }
    assert!(eng.armed);
    assert_eq!(eng.current.filled, 40);
    // Reset: clock dies mid-track, spawn in the pits.
    eng.tick(&snap("A", 0.42, 80, 0, 2));
    assert!(!eng.armed, "reset is an out-lap");
    assert_eq!(eng.current.filled, 0);
    for i in 0..15 {
        eng.tick(&snap("A", 0.45 + i as f32 * 0.02, 1_000 + i * 200, 0, 2));
    }
    assert_eq!(eng.current.filled, 0);
    eng.tick(&snap("A", 0.02, 16_000, 0, 2));
    eng.tick(&snap("A", 0.04, 300, 0, 3));
    assert!(eng.armed);
    assert_eq!(eng.current.filled, 1);
}

#[test]
fn first_lap_is_set_not_compared() {
    let mut eng = DeltaEngine::new();
    eng.tick(&snap("A", 0.02, 80, 0, 1));
    let v = eng.tick(&snap("A", 0.05, 400, 0, 1));
    assert!(!v.ready);
    assert!(v.recording);
    assert_eq!(v.delta_ms, 0);
}

#[test]
fn decent_lap_becomes_reference() {
    let mut eng = DeltaEngine::new();
    run_lap(&mut eng, "A", 72_000, 1, false);
    assert!(eng.reference.is_some());
    assert_eq!(eng.ref_lap_ms, 72_000, "ref {}", eng.ref_lap_ms);
    let v = eng.tick(&snap("A", 0.5, 35_000, 72_000, 2));
    assert!(v.ready);
    let t = eng.reference.as_ref().unwrap().time_at(0.5).unwrap();
    assert_eq!(v.delta_ms, 35_000 - t);
    assert_eq!(v.last_lap_ms, 72_000);
    assert_eq!(v.ref_lap_ms, 72_000);
}

#[test]
fn last_lap_held_when_plugin_zeros() {
    let mut eng = DeltaEngine::new();
    run_lap(&mut eng, "A", 72_000, 1, false);
    assert_eq!(eng.shown_last_ms, 72_000, "held {}", eng.shown_last_ms);
    let v = eng.tick(&snap("A", 0.2, 10_000, 0, 2));
    assert_eq!(v.last_lap_ms, 72_000, "last {}", v.last_lap_ms);
    assert_eq!(v.ref_lap_ms, 72_000, "best {}", v.ref_lap_ms);
}

#[test]
fn slower_lap_does_not_replace_best() {
    let mut eng = DeltaEngine::new();
    run_lap(&mut eng, "A", 70_000, 1, false);
    run_lap(&mut eng, "A", 80_000, 2, false);
    assert_eq!(eng.ref_lap_ms, 70_000, "ref {}", eng.ref_lap_ms);
}

#[test]
fn faster_lap_replaces_best() {
    let mut eng = DeltaEngine::new();
    run_lap(&mut eng, "A", 74_000, 1, false);
    run_lap(&mut eng, "A", 71_000, 2, false);
    assert_eq!(eng.ref_lap_ms, 71_000, "ref {}", eng.ref_lap_ms);
    assert!(eng.last_view.new_best);
}

#[test]
fn first_commit_is_new_best() {
    let mut eng = DeltaEngine::new();
    run_lap(&mut eng, "A", 72_000, 1, false);
    assert!(eng.last_view.new_best);
}

#[test]
fn dab_does_not_block_reference() {
    let mut eng = DeltaEngine::new();
    run_lap(&mut eng, "A", 90_000, 1, true);
    assert!(eng.reference.is_some());
    assert_eq!(eng.ref_lap_ms, 90_000);
}

#[test]
fn plugin_pos_one_does_not_poison_next_lap() {
    let mut eng = DeltaEngine::new();
    let lap_ms = 90_000;
    let n = 180;
    for i in 0..n {
        let t = i as f32 / n as f32;
        let mut s = snap("A", t, (lap_ms as f32 * t) as i32, 0, 1);
        if i + 1 == n {
            s.local_track_pos = 0.999;
            s.current_lap_ms = lap_ms;
        }
        eng.tick(&s);
    }
    // Plugin sends 1.0 with the finished-lap clock still showing.
    eng.tick(&snap("A", 1.0, lap_ms, 0, 1));
    eng.tick(&snap("A", 0.0, 15, 0, 2));
    assert!(eng.reference.is_some(), "first lap should commit");
    for i in 1..60 {
        let t = i as f32 / 180.0;
        eng.tick(&snap("A", t, 300 + i * 400, 0, 2));
    }
    assert_eq!(
        eng.current.filled, 59,
        "next lap frozen, filled={}",
        eng.current.filled
    );
    let v = eng.tick(&snap("A", 60.0 / 180.0, 300 + 60 * 400, 0, 2));
    assert!(v.ready);
    assert!(v.has_delta, "should compare on lap 2");
    assert_eq!(v.delta_ms.signum(), -1);
}

#[test]
fn short_outlap_is_not_reference() {
    let mut eng = DeltaEngine::new();
    for i in 0..40 {
        let t = 0.4 + i as f32 * 0.01;
        eng.tick(&snap("A", t, 8_000 + i * 200, 0, 1));
    }
    let mut done = snap("A", 0.02, 100, 12_000, 2);
    done.last_lap_ms = 12_000;
    assert_eq!(eng.current.filled, 0, "out-lap must not fill the tape");
    eng.tick(&done);
    assert!(eng.reference.is_none());
}

#[test]
fn track_change_drops_reference() {
    let mut eng = DeltaEngine::new();
    run_lap(&mut eng, "A", 72_000, 1, false);
    assert!(eng.reference.is_some());
    let v = eng.tick(&snap("B", 0.2, 5_000, 0, 1));
    assert!(!v.ready);
    assert!(eng.reference.is_none());
}

#[test]
fn interpolates_missing_bins() {
    let mut tape = LapTape::new();
    tape.push(10.0 / BINS as f32, 10_000);
    tape.push(20.0 / BINS as f32, 20_000);
    assert_eq!(tape.time_at(15.0 / BINS as f32), Some(15_000));
}

#[test]
fn crossing_with_zero_last_ms_still_commits() {
    let mut eng = DeltaEngine::new();
    run_lap(&mut eng, "A", 90_708, 12, false);
    assert!(eng.reference.is_some(), "should commit using live clock");
    assert_eq!(eng.ref_lap_ms, 90_708, "ref {}", eng.ref_lap_ms);
    let v = eng.tick(&snap("A", 0.25, 20_000, 0, 13));
    assert!(v.ready);
    let t = eng.reference.as_ref().unwrap().time_at(0.25).unwrap();
    assert_eq!(v.delta_ms, 20_000 - t);
}

#[test]
fn next_lap_keeps_recording_after_wrap() {
    let mut eng = DeltaEngine::new();
    run_lap(&mut eng, "A", 90_000, 1, false);
    assert!(eng.reference.is_some());
    for i in 0..40 {
        let t = i as f32 / 200.0;
        eng.tick(&snap("A", t, 200 + i * 400, 0, 2));
    }
    assert_eq!(eng.current.filled, 40, "filled {}", eng.current.filled);
    let v = eng.tick(&snap("A", 0.20, 18_000, 0, 2));
    assert!(v.ready);
}

#[test]
fn pos_one_is_recorded() {
    let mut tape = LapTape::new();
    tape.push(1.0, 72_000);
    assert_eq!(tape.filled, 1);
    assert_eq!(tape.time_at(0.999_999), Some(72_000));
}

#[test]
fn saved_tape_loads_without_riding() {
    let _lock = track_pb::exclusive_test();
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("mxbo-delta-load-{n}"));
    track_pb::set_store_dir(dir.clone());
    let mut bins = [0; BINS];
    for i in 0..BINS {
        bins[i] = 100 + (i as i32) * 280;
    }
    assert!(track_pb::commit_tape("Saved", "", 72_000, bins));
    let mut eng = DeltaEngine::new();
    let first = eng.tick(&snap("Saved", 0.05, 400, 72_000, 2));
    assert!(first.ready);
    assert!(first.has_delta);
    let t = eng.reference.as_ref().unwrap().time_at(0.05).unwrap();
    assert_eq!(first.delta_ms, 400 - t);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn wrap_with_old_clock_does_not_show_plus_sixteen() {
    let mut eng = DeltaEngine::new();
    run_lap(&mut eng, "A", 72_000, 1, false);
    assert!(eng.reference.is_some());
    for i in 0..12 {
        let t = (0.90 + i as f32 * 0.008).min(0.995);
        eng.tick(&snap("A", t, 15_200 + i * 80, 0, 2));
    }
    let poison = eng.tick(&snap("A", 0.01, 16_000, 0, 2));
    assert!(
        !poison.has_delta,
        "crossing still comparing the out-lap clock: delta={}",
        poison.delta_ms
    );
    let v = eng.tick(&snap("A", 0.03, 500, 0, 2));
    assert!(v.has_delta, "should compare once the new lap clock starts");
    let t = eng.reference.as_ref().unwrap().time_at(0.03).unwrap();
    assert_eq!(v.delta_ms, 500 - t);
}

#[test]
fn mid_lap_plus_sixteen_still_shows() {
    let mut eng = DeltaEngine::new();
    run_lap(&mut eng, "A", 72_000, 1, false);
    let v = eng.tick(&snap("A", 0.50, 52_000, 72_000, 2));
    assert!(v.has_delta);
    let t = eng.reference.as_ref().unwrap().time_at(0.50).unwrap();
    assert_eq!(v.delta_ms, 52_000 - t);
}

#[test]
fn decent_rejects_short_sparse_and_narrow_tapes() {
    let mut tape = LapTape::new();
    for i in 0..200 {
        tape.push(i as f32 / 200.0, 10_000 + i * 50);
    }
    assert!(!tape.decent(19_999));
    assert!(tape.decent(20_000));
    assert!(!tape.decent(900_001));

    let mut sparse = LapTape::new();
    for i in 0..100 {
        sparse.push(i as f32 / BINS as f32, 30_000 + i);
    }
    assert!(!sparse.decent(72_000));

    let mut narrow = LapTape::new();
    for i in 0..200 {
        let t = 0.10 + i as f32 / 200.0 * 0.40;
        narrow.push(t, 30_000 + i * 100);
    }
    assert!(!narrow.decent(72_000));
}

#[test]
fn push_skips_soft_reverse_and_clock_restart() {
    let mut tape = LapTape::new();
    tape.push(0.50, 10_000);
    tape.push(0.47, 10_100);
    assert_eq!(tape.filled, 1);
    tape.push(0.52, 10_200);
    assert_eq!(tape.filled, 2);
    tape.push(0.60, 20_000);
    tape.push(0.61, 100);
    assert_eq!(tape.filled, 3);
    assert_eq!(tape.bins[((0.61 * BINS as f32) as usize).min(BINS - 1)], 0);
}

#[test]
fn other_bike_does_not_load_saved_tape() {
    let _lock = track_pb::exclusive_test();
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("mxbo-delta-bike-{n}"));
    track_pb::set_store_dir(dir.clone());
    let mut bins = [0; BINS];
    for i in 0..BINS {
        bins[i] = 100 + (i as i32) * 280;
    }
    assert!(track_pb::commit_tape("Saved", "YZ450F", 72_000, bins));
    let mut eng = DeltaEngine::new();
    let mut two = snap("Saved", 0.05, 400, 72_000, 2);
    with_bike(&mut two, "YZ250F");
    let v = eng.tick(&two);
    assert!(!v.ready, "250 must not load the 450 tape");
    assert!(!v.has_delta);
    let mut four = snap("Saved", 0.05, 400, 72_000, 2);
    with_bike(&mut four, "YZ450F");
    let mut eng2 = DeltaEngine::new();
    let v2 = eng2.tick(&four);
    assert!(v2.ready);
    assert!(v2.has_delta);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn same_class_loads_saved_tape() {
    let _lock = track_pb::exclusive_test();
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("mxbo-delta-class-{n}"));
    track_pb::set_store_dir(dir.clone());
    let mut bins = [0; BINS];
    for i in 0..BINS {
        bins[i] = 100 + (i as i32) * 280;
    }
    assert!(track_pb::commit_tape("Saved", "YZ250F", 72_000, bins));
    let mut eng = DeltaEngine::new();
    let mut honda = snap("Saved", 0.05, 400, 72_000, 2);
    with_bike(&mut honda, "CRF250R");
    let v = eng.tick(&honda);
    assert!(v.ready, "Honda 250 should load the Yamaha 250 tape");
    assert!(v.has_delta);
    let _ = std::fs::remove_dir_all(dir);
}
