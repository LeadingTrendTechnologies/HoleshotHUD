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
    // Out-lap to the line, then a wrap starts the flying lap. Clock-start near
    // pos 0 is pits, not S/F.
    for i in 0..8 {
        let t = (0.80 + i as f32 * 0.02).min(0.98);
        eng.tick(&snap(track, t, 10_000 + i * 200, 0, lap_num.max(1)));
    }
    eng.tick(&snap(track, 0.01, 80, 0, lap_num.max(1)));
    let n = 180;
    for i in 1..n {
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

/// Out-lap is not a timed lap. MX starts the flying clock at S/F with last-lap still 0.
#[test]
fn outlap_clock_drop_at_sf_starts_first_flying_lap() {
    let mut eng = DeltaEngine::new();
    let sf = 0.28;
    for i in 0..12 {
        eng.tick(&snap("A", 0.10 + i as f32 * 0.01, 8_000 + i * 200, 0, 1));
    }
    assert!(!eng.armed);
    eng.tick(&snap("A", sf, 120, 0, 1));
    assert!(eng.armed, "clock drop at the line must start the first flying lap");
    let mut ms = 400;
    let mut p = sf + 0.01;
    while p < 0.98 {
        eng.tick(&snap("A", p, ms, 0, 1));
        ms += 280;
        p += 0.004;
    }
    p = 0.04;
    while p < sf - 0.01 {
        eng.tick(&snap("A", p, ms, 0, 1));
        ms += 280;
        p += 0.004;
    }
    let done = eng.tick(&snap("A", sf + 0.01, 180, 0, 2));
    assert!(
        eng.reference.is_some(),
        "first flying lap after an untimed out-lap must save like MX Bikes"
    );
    assert!(done.ready);
}

#[test]
fn clock_collapse_mid_lap_does_not_start_flying() {
    let mut eng = DeltaEngine::new();
    eng.tick(&snap("A", 0.90, 199_000, 0, 0));
    eng.tick(&snap("A", 0.91, 200, 0, 0));
    assert!(!eng.armed, "3:20 clock collapse is not S/F");
    assert_eq!(eng.current.filled, 0);
}

/// Plugin last-lap can be the previous PB while the live clock was faster (Timberline 1:38 vs 1:40).
#[test]
fn faster_live_clock_beats_stale_plugin_last() {
    let mut eng = DeltaEngine::new();
    run_lap(&mut eng, "A", 100_501, 1, false);
    assert_eq!(eng.ref_lap_ms, 100_501);
    for i in 0..8 {
        let t = (0.80 + i as f32 * 0.02).min(0.98);
        eng.tick(&snap("A", t, 10_000 + i * 200, 100_501, 2));
    }
    eng.tick(&snap("A", 0.01, 80, 100_501, 2));
    fly_to_line(&mut eng, "A", 97_000, 100_501, 2);
    let v = eng.tick(&snap("A", 0.0, 10, 100_501, 3));
    assert_eq!(
        eng.ref_lap_ms, 97_000,
        "1:37 live clock must beat a republished 1:40 last-lap, ref={}",
        eng.ref_lap_ms
    );
    assert!(v.new_best);
    assert_eq!(v.last_lap_ms, 97_000);
}

/// Crossing often zeros last-lap. The tape we just filled is the lap.
#[test]
fn zeroed_last_lap_still_saves_faster_tape() {
    let mut eng = DeltaEngine::new();
    run_lap(&mut eng, "A", 100_501, 1, false);
    for i in 0..8 {
        let t = (0.80 + i as f32 * 0.02).min(0.98);
        eng.tick(&snap("A", t, 10_000 + i * 200, 100_501, 2));
    }
    eng.tick(&snap("A", 0.01, 80, 100_501, 2));
    fly_to_line(&mut eng, "A", 97_899, 100_501, 2);
    let v = eng.tick(&snap("A", 0.0, 10, 0, 3));
    assert_eq!(
        eng.ref_lap_ms, 97_899,
        "zeroed last-lap must not keep the 1:40 tape, ref={}",
        eng.ref_lap_ms
    );
    assert_eq!(v.last_lap_ms, 97_899);
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
    assert!(!v.recording, "pits near S/F must not start the tape");
    assert_eq!(v.delta_ms, 0);
    assert_eq!(eng.current.filled, 0);
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
fn session_compare_uses_this_visit_not_saved_tape() {
    let _lock = track_pb::exclusive_test();
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("mxbo-delta-session-{n}"));
    track_pb::set_store_dir(dir.clone());
    let mut bins = [0; BINS];
    for i in 0..BINS {
        bins[i] = (60_000i64 * i as i64 / BINS as i64) as i32;
        if i > 0 && bins[i] <= 0 {
            bins[i] = 1;
        }
    }
    bins[0] = 1;
    assert!(track_pb::commit_tape("Sess", "YZ450F", 60_000, bins));

    let mut eng = DeltaEngine::new();
    let mut s = snap("Sess", 0.4, 24_000, 0, 2);
    with_bike(&mut s, "YZ450F");
    eng.tick(&s);
    assert_eq!(eng.ref_lap_ms, 60_000);
    assert_eq!(eng.session_lap_ms, 0);
    assert!(eng.last_view.ready);
    assert!(!eng.session_view.ready, "session compare waits for a lap this visit");

    run_lap(&mut eng, "Sess", 72_000, 2, false);
    assert_eq!(eng.ref_lap_ms, 60_000, "saved tape stays the all-time best");
    assert_eq!(eng.session_lap_ms, 72_000);
    let mut mid = snap("Sess", 0.5, 36_000, 72_000, 3);
    with_bike(&mut mid, "YZ450F");
    eng.tick(&mid);
    assert_eq!(eng.last_view.ref_lap_ms, 60_000);
    assert_eq!(eng.session_view.ref_lap_ms, 72_000);
    assert!(eng.session_view.ready);
    let _ = std::fs::remove_dir_all(dir);
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
    for i in 0..8 {
        let t = (0.80 + i as f32 * 0.02).min(0.98);
        eng.tick(&snap("A", t, 10_000 + i * 200, 0, 1));
    }
    eng.tick(&snap("A", 0.01, 80, 0, 1));
    let n = 180;
    for i in 1..n {
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
fn origin_wrap_mid_lap_keeps_delta() {
    let _lock = track_pb::exclusive_test();
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("mxbo-delta-origin-wrap-{n}"));
    track_pb::set_store_dir(dir.clone());
    // S/F at 0.5 so pos 0 is mid-lap — wrapping origin is sector 3, not the line.
    let mut bins = [0; BINS];
    for i in 0..BINS {
        let from_sf = (i as f32 / BINS as f32 + 0.5).rem_euclid(1.0);
        bins[i] = 200 + (from_sf * 72_000.0) as i32;
    }
    assert!(track_pb::commit_tape("Orig", "YZ450F", 72_000, bins));

    let mut eng = DeltaEngine::new();
    let t = |p: f32| track_pb::time_at(&bins, p).unwrap();
    for i in 0..10 {
        let p = 0.88 + i as f32 * 0.01;
        let mut s = snap("Orig", p, t(p), 72_000, 2);
        with_bike(&mut s, "YZ450F");
        eng.tick(&s);
    }
    let mut wrap = snap("Orig", 0.04, t(0.04) + 80, 72_000, 2);
    with_bike(&mut wrap, "YZ450F");
    let v = eng.tick(&wrap);
    assert!(
        v.has_delta,
        "centerline origin in S3 is not S/F; delta must keep moving"
    );
    assert!(!eng.stale_clock);
    assert_eq!(eng.ref_lap_ms, 72_000, "origin wrap must not commit a short tape");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn origin_wrap_does_not_end_the_lap() {
    let _lock = track_pb::exclusive_test();
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("mxbo-delta-origin-end-{n}"));
    track_pb::set_store_dir(dir.clone());
    let mut bins = [0; BINS];
    for i in 0..BINS {
        let from_sf = (i as f32 / BINS as f32 + 0.5).rem_euclid(1.0);
        bins[i] = 200 + (from_sf * 72_000.0) as i32;
    }
    assert!(track_pb::commit_tape("Orig", "YZ450F", 72_000, bins));

    let mut eng = DeltaEngine::new();
    let t = |p: f32| track_pb::time_at(&bins, p).unwrap();
    for i in 0..12 {
        let p = 0.86 + i as f32 * 0.01;
        let mut s = snap("Orig", p.min(0.97), t(p.min(0.97)), 72_000, 2);
        with_bike(&mut s, "YZ450F");
        eng.tick(&s);
    }
    let filled = eng.current.filled;
    assert!(filled >= 8, "filled {filled}");
    let mut wrap = snap("Orig", 0.03, t(0.03) + 100, 72_000, 2);
    with_bike(&mut wrap, "YZ450F");
    eng.tick(&wrap);
    assert!(eng.current.filled >= filled, "keep taping through origin");
    assert_eq!(eng.ref_lap_ms, 72_000);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn first_flying_lap_origin_wrap_then_sf_commits() {
    let mut eng = DeltaEngine::new();
    for i in 0..8 {
        let t = (0.80 + i as f32 * 0.02).min(0.98);
        eng.tick(&snap("A", t, 10_000 + i * 200, 0, 1));
    }
    eng.tick(&snap("A", 0.01, 80, 0, 1));
    let mut ms = 400;
    let mut p = 0.40;
    while p < 0.98 {
        eng.tick(&snap("A", p, ms, 0, 1));
        ms += 300;
        p += 0.005;
    }
    assert!(eng.reference.is_none());
    let filled = eng.current.filled;
    eng.tick(&snap("A", 0.03, ms, 0, 1));
    ms += 300;
    assert!(eng.reference.is_none(), "origin wrap must not end the first flying lap");
    assert!(
        eng.current.filled >= filled,
        "keep REC through origin, filled {} -> {}",
        filled,
        eng.current.filled
    );
    p = 0.04;
    while p < 0.34 {
        eng.tick(&snap("A", p, ms, 0, 1));
        ms += 300;
        p += 0.005;
    }
    eng.tick(&snap("A", 0.35, 180, 0, 2));
    assert!(
        eng.reference.is_some(),
        "first flying lap must become the tape after the real line"
    );
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
        tape.push(i as f32 / 200.0, 10_000 + i * 100);
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

#[test]
fn leftover_replay_telemetry_does_not_record() {
    let mut eng = DeltaEngine::new();
    for i in 0..40 {
        let t = 0.70 + i as f32 * 0.006;
        eng.tick(&snap("A", t.min(0.98), 12_000 + i * 150, 0, 1));
    }
    let mut leftover = snap("A", 0.02, 400, 0, 2);
    leftover.has_telemetry = 0;
    leftover.local_speed = 18.0;
    leftover.on_track = 1;
    eng.tick(&leftover);
    assert!(!eng.armed);
    assert_eq!(eng.current.filled, 0);
    for i in 1..80 {
        let mut s = snap("A", i as f32 / 180.0, 300 + i * 400, 0, 2);
        s.has_telemetry = 0;
        s.local_speed = 18.0;
        eng.tick(&s);
    }
    assert_eq!(eng.current.filled, 0);
    assert!(eng.reference.is_none());
}

#[test]
fn pits_spawn_near_sf_is_not_recording() {
    let mut eng = DeltaEngine::new();
    for i in 0..30 {
        let v = eng.tick(&snap("A", 0.05 + i as f32 * 0.002, 200 + i * 400, 0, 1));
        assert!(!v.recording, "spawn in pits at pos 0.05 must stay SET LAP");
        assert_eq!(eng.current.filled, 0);
    }
}

#[test]
fn reset_to_pits_near_sf_waits_for_wrap() {
    let mut eng = DeltaEngine::new();
    run_lap(&mut eng, "A", 72_000, 1, false);
    for i in 0..40 {
        let t = i as f32 / 200.0;
        eng.tick(&snap("A", t, 200 + i * 400, 0, 2));
    }
    assert!(eng.armed);
    eng.tick(&snap("A", 0.12, 80, 0, 2));
    assert!(!eng.armed, "reset into pits near S/F is an out-lap");
    assert_eq!(eng.current.filled, 0);
    for i in 0..20 {
        eng.tick(&snap("A", 0.10 + i as f32 * 0.004, 300 + i * 200, 0, 2));
        assert!(!eng.armed);
        assert_eq!(eng.current.filled, 0);
    }
    for i in 0..12 {
        let t = (0.90 + i as f32 * 0.008).min(0.995);
        eng.tick(&snap("A", t, 15_200 + i * 80, 0, 2));
        assert!(!eng.armed);
        assert_eq!(eng.current.filled, 0);
    }
    eng.tick(&snap("A", 0.02, 16_000, 0, 2));
    eng.tick(&snap("A", 0.04, 300, 0, 3));
    assert!(eng.armed);
    assert_eq!(eng.current.filled, 1);
}

#[test]
fn hitch_across_sf_still_commits() {
    let mut eng = DeltaEngine::new();
    let lap_ms = 72_000;
    for i in 0..8 {
        let t = (0.80 + i as f32 * 0.02).min(0.98);
        eng.tick(&snap("A", t, 10_000 + i * 200, 0, 1));
    }
    eng.tick(&snap("A", 0.01, 80, 0, 1));
    let n = 180;
    for i in 1..n {
        let t = i as f32 / n as f32;
        if t > 0.92 {
            break;
        }
        eng.tick(&snap("A", t, (lap_ms as f32 * t) as i32, 0, 1));
    }
    assert!(eng.reference.is_none());
    eng.tick(&snap("A", 0.08, 600, 0, 2));
    assert!(eng.reference.is_some(), "hitch across S/F must still commit");
    assert!(eng.ref_lap_ms >= 60_000, "ref {}", eng.ref_lap_ms);
}

#[test]
fn empty_bike_commit_persists_when_class_arrives() {
    let _lock = track_pb::exclusive_test();
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("mxbo-delta-empty-bike-{n}"));
    track_pb::set_store_dir(dir.clone());
    let mut bins250 = [0; BINS];
    for i in 0..BINS {
        bins250[i] = 100 + (i as i32) * 280;
    }
    assert!(track_pb::commit_tape("Saved", "YZ250F", 80_000, bins250));

    let mut eng = DeltaEngine::new();
    run_lap(&mut eng, "Saved", 72_000, 1, false);
    assert!(eng.reference.is_some());
    assert_eq!(eng.ref_lap_ms, 72_000);
    assert_eq!(
        track_pb::bind("Saved", "YZ250F").lap_ms,
        80_000,
        "250 tape must stay on disk until the class is known"
    );

    let mut four = snap("Saved", 0.10, 8_000, 72_000, 2);
    with_bike(&mut four, "YZ450F");
    let v = eng.tick(&four);
    assert!(v.ready);
    assert_eq!(eng.bike_key, "450");
    assert_eq!(eng.ref_lap_ms, 72_000);
    assert_eq!(track_pb::bind("Saved", "YZ450F").lap_ms, 72_000);
    assert_eq!(
        track_pb::bind("Saved", "YZ250F").lap_ms,
        80_000,
        "250 must be untouched"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn cut_jump_does_not_commit() {
    let mut eng = DeltaEngine::new();
    for i in 0..8 {
        let t = (0.80 + i as f32 * 0.02).min(0.98);
        eng.tick(&snap("A", t, 10_000 + i * 200, 0, 1));
    }
    eng.tick(&snap("A", 0.01, 80, 0, 1));
    let n = 256;
    for i in 1..n {
        let t = i as f32 / n as f32;
        if t > 0.38 && t < 0.70 {
            continue;
        }
        let mut ms = (72_000.0 * t) as i32;
        if t >= 0.70 {
            ms = (72_000.0 * 0.38) as i32 + 180 + ((t - 0.70) * 72_000.0) as i32;
        }
        let mut s = snap("A", t, ms, 0, 1);
        if i + 1 == n {
            s.local_track_pos = 0.999;
            s.current_lap_ms = ms;
        }
        eng.tick(&s);
    }
    eng.tick(&snap("A", 0.01, 180, 0, 2));
    assert!(eng.reference.is_none(), "a cut must not become the tape");
}

#[test]
fn cut_does_not_replace_a_real_pb() {
    let mut eng = DeltaEngine::new();
    run_lap(&mut eng, "A", 72_000, 1, false);
    assert_eq!(eng.ref_lap_ms, 72_000);
    for i in 0..8 {
        let t = (0.80 + i as f32 * 0.02).min(0.98);
        eng.tick(&snap("A", t, 10_000 + i * 200, 72_000, 2));
    }
    eng.tick(&snap("A", 0.01, 80, 72_000, 2));
    let n = 256;
    for i in 1..n {
        let t = i as f32 / n as f32;
        if t > 0.38 && t < 0.70 {
            continue;
        }
        let mut ms = (40_000.0 * t) as i32;
        if t >= 0.70 {
            ms = (40_000.0 * 0.38) as i32 + 180 + ((t - 0.70) * 40_000.0) as i32;
        }
        eng.tick(&snap("A", t, ms, 72_000, 2));
    }
    eng.tick(&snap("A", 0.01, 180, 0, 3));
    assert_eq!(eng.ref_lap_ms, 72_000, "cut lap must not replace a real best");
}

#[test]
fn hitch_mid_lap_still_commits() {
    let mut eng = DeltaEngine::new();
    for i in 0..8 {
        let t = (0.80 + i as f32 * 0.02).min(0.98);
        eng.tick(&snap("A", t, 10_000 + i * 200, 0, 1));
    }
    eng.tick(&snap("A", 0.01, 80, 0, 1));
    let n = 256;
    for i in 1..n {
        let t = i as f32 / n as f32;
        if t > 0.40 && t < 0.58 {
            continue;
        }
        let mut s = snap("A", t, (72_000.0 * t) as i32, 0, 1);
        if i + 1 == n {
            s.local_track_pos = 0.999;
            s.current_lap_ms = 72_000;
        }
        eng.tick(&s);
    }
    eng.tick(&snap("A", 0.01, 180, 0, 2));
    assert!(eng.reference.is_some(), "hitch that still covers the ground must commit");
    assert!(eng.ref_lap_ms >= 60_000, "ref {}", eng.ref_lap_ms);
}

#[test]
fn decent_rejects_a_cut_tape() {
    let mut tape = LapTape::new();
    for i in 0..80 {
        tape.push_at(i as f32 / 200.0, 200 + i * 360, 1600.0);
    }
    for i in 160..200 {
        tape.push_at(
            i as f32 / 200.0,
            200 + 80 * 360 + 150 + (i - 160) * 360,
            1600.0,
        );
    }
    assert!(tape.dirty, "100m+ skip in 150ms is a cut");
    assert!(!tape.decent(72_000));
}

#[test]
fn jump_after_airtime_is_not_a_cut() {
    let mut tape = LapTape::new();
    let mut ms = 200;
    let mut pos = 0.01;
    while pos < 0.45 {
        tape.push_at(pos, ms, 1600.0);
        pos += 1.0 / 180.0;
        ms += 400;
    }
    ms += 1_500;
    pos += 60.0 / 1600.0;
    tape.push_at(pos, ms, 1600.0);
    while pos < 0.98 {
        pos += 1.0 / 180.0;
        ms += 400;
        tape.push_at(pos.min(0.99), ms, 1600.0);
    }
    assert!(!tape.dirty, "airtime then a short land is a jump, not a cut");
    assert!(tape.decent(ms));
}

#[test]
fn big_jump_airtime_still_commits() {
    let mut tape = LapTape::new();
    let mut ms = 200;
    let mut pos = 0.01;
    while pos < 0.45 {
        tape.push_at(pos, ms, 1600.0);
        pos += 1.0 / 180.0;
        ms += 400;
    }
    // ~140 m of centerline under a 1.2 s flight — a triple, not a cut.
    ms += 1_200;
    pos += 140.0 / 1600.0;
    tape.push_at(pos, ms, 1600.0);
    while pos < 0.98 {
        pos += 1.0 / 180.0;
        ms += 400;
        tape.push_at(pos.min(0.99), ms, 1600.0);
    }
    assert!(!tape.dirty, "a jump that skips 140 m in air is not a cut");
    assert!(tape.decent(ms), "first flying lap with a jump must still save");
}

#[test]
fn first_flying_lap_with_a_jump_becomes_the_tape() {
    let mut eng = DeltaEngine::new();
    for i in 0..8 {
        let t = (0.80 + i as f32 * 0.02).min(0.98);
        eng.tick(&snap("A", t, 10_000 + i * 200, 0, 1));
    }
    eng.tick(&snap("A", 0.01, 80, 0, 1));
    let lap_ms = 72_000;
    let jump_from = 0.45;
    let jump_to = jump_from + 140.0 / 1600.0;
    let n = 180;
    for i in 1..n {
        let t = i as f32 / n as f32;
        if t > jump_from && t < jump_to {
            continue;
        }
        let mut ms = (lap_ms as f32 * t) as i32;
        if t >= jump_to {
            ms = (lap_ms as f32 * jump_from) as i32
                + 1_200
                + ((t - jump_to) * lap_ms as f32) as i32;
        }
        let mut s = snap("A", t, ms, 0, 1);
        if i + 1 == n {
            s.local_track_pos = 0.999;
            s.current_lap_ms = ms;
        }
        eng.tick(&s);
    }
    let v = eng.tick(&snap("A", 0.01, 180, 0, 2));
    assert!(
        eng.reference.is_some(),
        "REC must become the tape after a lap with a jump"
    );
    assert!(v.ready, "must not reset to SET LAP at the line");
}

#[test]
fn rec_lap_clock_drop_at_sf_not_origin_commits() {
    let mut eng = DeltaEngine::new();
    let sf = 0.28;
    for i in 0..12 {
        eng.tick(&snap("A", 0.10 + i as f32 * 0.01, 8_000 + i * 200, 0, 1));
    }
    eng.tick(&snap("A", sf, 120, 14_800, 2));
    let mut ms = 400;
    let mut p = sf + 0.01;
    while p < 0.98 {
        eng.tick(&snap("A", p, ms, 14_800, 2));
        ms += 280;
        p += 0.004;
    }
    let filled = eng.current.filled;
    eng.tick(&snap("A", 0.03, ms, 14_800, 2));
    assert!(eng.reference.is_none(), "origin wrap is not the finish");
    assert!(eng.current.filled >= filled);
    ms += 280;
    p = 0.04;
    while p < sf - 0.01 {
        eng.tick(&snap("A", p, ms, 14_800, 2));
        ms += 280;
        p += 0.004;
    }
    // Plugin often zeros last-lap and holds the lap number on the crossing.
    let done = eng.tick(&snap("A", sf + 0.01, 180, 0, 2));
    assert!(
        eng.reference.is_some(),
        "REC lap must save when the clock drops at the real line"
    );
    assert!(done.ready, "must not fall back to SET LAP");
    let v = eng.tick(&snap("A", sf + 0.04, 900, 0, 2));
    assert!(
        v.has_delta,
        "next lap must compare, ready={} recording={} cover={}",
        v.ready, v.recording, v.cover
    );
}

fn fly_to_line(eng: &mut DeltaEngine, track: &str, lap_ms: i32, last_ms: i32, lap: i32) {
    let n = 180;
    for i in 1..n {
        let t = i as f32 / n as f32;
        let mut s = snap(track, t, (lap_ms as f32 * t).max(80.0) as i32, last_ms, lap);
        if i + 1 == n {
            s.local_track_pos = 1.0;
            s.current_lap_ms = lap_ms;
        }
        eng.tick(&s);
    }
}

/// Plugin sends `local_track_pos == 1.0` on the line, then 0.0 with the new clock
/// and last-lap together. Standings already have the time; REC must become the tape.
#[test]
fn pos_one_then_zero_with_last_lap_commits() {
    let mut eng = DeltaEngine::new();
    for i in 0..8 {
        let t = (0.80 + i as f32 * 0.02).min(0.98);
        eng.tick(&snap("Ezkutu", t, 10_000 + i * 200, 0, 1));
    }
    eng.tick(&snap("Ezkutu", 0.01, 80, 0, 1));
    fly_to_line(&mut eng, "Ezkutu", 150_848, 0, 2);
    assert!(eng.reference.is_none());
    let v = eng.tick(&snap("Ezkutu", 0.0, 10, 150_848, 3));
    assert!(
        eng.reference.is_some(),
        "REC must save when pos 1.0 jumps to 0 with last-lap, ready={} recording={} cover={} filled={}",
        v.ready,
        v.recording,
        v.cover,
        eng.current.filled
    );
    assert!(v.ready);
}

/// Clock already restarted on the previous frame (last_cur < 8s). Next tick
/// publishes last-lap / lap bump — standings and sectors freeze, delta must too.
#[test]
fn last_lap_after_clock_already_low_commits() {
    let mut eng = DeltaEngine::new();
    for i in 0..8 {
        let t = (0.80 + i as f32 * 0.02).min(0.98);
        eng.tick(&snap("Ezkutu", t, 10_000 + i * 200, 0, 1));
    }
    eng.tick(&snap("Ezkutu", 0.01, 80, 0, 1));
    fly_to_line(&mut eng, "Ezkutu", 151_540, 146_602, 2);
    // Crossing frame the overlay missed: clock is already the new lap.
    eng.tick(&snap("Ezkutu", 1.0, 10, 146_602, 2));
    let v = eng.tick(&snap("Ezkutu", 0.0, 80, 150_848, 3));
    assert!(
        eng.reference.is_some(),
        "last-lap after a restarted clock must still save, ready={} rec={} cover={} filled={} ref={}",
        v.ready,
        v.recording,
        v.cover,
        eng.current.filled,
        eng.ref_lap_ms
    );
}

/// Out-lap clock runs past 200s and collapses to ~200 ms (plugin `_fTime` heuristic),
/// then last-lap arrives at pos 1.0. The next flying lap must still become the tape.
#[test]
fn clock_collapse_then_flying_lap_commits() {
    let mut eng = DeltaEngine::new();
    for i in 0..8 {
        let t = (0.80 + i as f32 * 0.02).min(0.98);
        eng.tick(&snap("Ezkutu", t, 50_000 + i * 200, 0, 0));
    }
    // Origin wrap mid out-lap: arms, stale, does not tape.
    eng.tick(&snap("Ezkutu", 0.0, 59_028, 0, 0));
    assert!(eng.armed);
    assert_eq!(eng.current.filled, 0);
    for i in 1..40 {
        let t = i as f32 / 50.0;
        eng.tick(&snap("Ezkutu", t.min(0.95), 59_028 + i * 3_500, 0, 0));
    }
    // dt > 200s: current_lap_ms jumps to ~200.
    eng.tick(&snap("Ezkutu", 0.957, 200, 0, 0));
    eng.tick(&snap("Ezkutu", 1.0, 205, 0, 0));
    eng.tick(&snap("Ezkutu", 1.0, 9, 146_602, 2));
    eng.tick(&snap("Ezkutu", 0.0, 80, 146_602, 2));
    assert!(
        eng.reference.is_none(),
        "collapsed-clock last-lap is not a taped flying lap"
    );
    fly_to_line(&mut eng, "Ezkutu", 150_848, 146_602, 2);
    let v = eng.tick(&snap("Ezkutu", 0.0, 10, 150_848, 3));
    assert!(
        eng.reference.is_some(),
        "next flying lap after a collapsed clock must save, ready={} rec={} cover={} filled={}",
        v.ready,
        v.recording,
        v.cover,
        eng.current.filled
    );
}
