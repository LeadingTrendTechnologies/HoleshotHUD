use super::*;

#[test]
fn empty_ticks_do_not_end_a_race() {
    let mut g = SessionGate::default();
    assert_eq!(g.update("", 0, 0, 0, 0, 0), SessionEvent::Continue);
}

#[test]
fn leaving_a_session_archives_the_race() {
    let mut g = SessionGate::default();
    assert_eq!(g.update("Glen", 4, 0, 30_000, 2, 1), SessionEvent::Continue);
    assert_eq!(g.update("", 0, 0, 0, 0, 0), SessionEvent::RaceEnded);
}

#[test]
fn track_change_starts_a_new_session() {
    let mut g = SessionGate::default();
    g.update("Glen", 4, 0, 40_000, 3, 1);
    assert_eq!(
        g.update("Hangtown", 4, 0, 1_000, 1, 1),
        SessionEvent::NewSession
    );
}

#[test]
fn track_change_after_the_session_ends_starts_a_new_session() {
    let mut g = SessionGate::default();
    assert_eq!(g.update("Meadow", 0, 600_000, 30_000, 4, 1), SessionEvent::Continue);
    assert_eq!(g.update("", 0, 0, 0, 0, 0), SessionEvent::RaceEnded);
    assert_eq!(
        g.update("Farm14", 1, 480_000, 40_000, 0, 1),
        SessionEvent::NewSession
    );
}

#[test]
fn same_track_after_a_dropout_keeps_the_session() {
    let mut g = SessionGate::default();
    assert_eq!(g.update("Glen", 4, 0, 40_000, 3, 1), SessionEvent::Continue);
    assert_eq!(g.update("", 0, 0, 0, 0, 0), SessionEvent::RaceEnded);
    assert_eq!(g.update("Glen", 4, 0, 39_000, 3, 1), SessionEvent::Continue);
}

#[test]
fn lap_count_change_starts_a_new_session() {
    let mut g = SessionGate::default();
    g.update("Glen", 4, 0, 40_000, 3, 1);
    assert_eq!(
        g.update("Glen", 6, 0, 1_000, 1, 1),
        SessionEvent::NewSession
    );
}

#[test]
fn clock_reset_starts_a_new_session() {
    let mut g = SessionGate::default();
    g.update("Glen", 0, 8, 90_000, 2, 1);
    assert_eq!(g.update("Glen", 0, 8, 800, 0, 1), SessionEvent::NewSession);
}

#[test]
fn same_track_lap_drop_starts_a_new_session() {
    let mut g = SessionGate::default();
    g.update("Glen", 1, 480_000, 90_000, 5, 1);
    assert_eq!(
        g.update("Glen", 1, 480_000, 800, 0, 1),
        SessionEvent::NewSession
    );
}

#[test]
fn timed_plus_one_expiry_keeps_the_same_session() {
    let mut g = SessionGate::default();
    g.update("Timberline", 1, 480_000, 30_000, 6, 1);
    assert_eq!(
        g.update("Timberline", 1, 480_000, 1_020, 7, 1),
        SessionEvent::Continue
    );
}

#[test]
fn six_lap_finish_clock_dip_keeps_the_same_session() {
    let mut g = SessionGate::default();
    g.update("Monster Mountain", 6, 0, 30_000, 6, 1);
    assert_eq!(
        g.update("Monster Mountain", 6, 0, 1_020, 7, 1),
        SessionEvent::Continue
    );
}

#[test]
fn lap_race_then_timed_session_starts_a_new_session() {
    let mut g = SessionGate::default();
    g.update("Monster Mountain", 6, 0, 30_000, 7, 1);
    assert_eq!(
        g.update("Monster Mountain", 0, 600_000, 540_630, 0, 1),
        SessionEvent::NewSession
    );
}

#[test]
fn timed_session_then_lap_race_starts_a_new_session() {
    let mut g = SessionGate::default();
    g.update("Monster Mountain", 0, 600_000, 30_000, 0, 1);
    assert_eq!(
        g.update("Monster Mountain", 6, 0, 24_060, 0, 1),
        SessionEvent::NewSession
    );
}

#[test]
fn same_moto_keeps_logging() {
    let mut g = SessionGate::default();
    g.update("Glen", 4, 0, 10_000, 1, 1);
    assert_eq!(g.update("Glen", 4, 0, 20_000, 2, 1), SessionEvent::Continue);
}

#[test]
fn peek_track_reads_latest_name() {
    let raw =
        "{\"v\":1}\n{\"track\":\"Glen Helen\",\"cur\":2}\n{\"track\":\"Hangtown\",\"cur\":1}\n";
    assert_eq!(peek_track(raw).as_deref(), Some("Hangtown"));
}

#[test]
fn header_only_clock_file_is_not_a_race_log() {
    let raw = "{\"v\":1,\"file\":\"C:\\\\Users\\\\troye\\\\AppData\\\\Local\\\\Holeshot HUD\\\\logs\\\\clock-20260818-204810.jsonl\"}\n";
    assert!(!raw_has_race(raw));
}

#[test]
fn clock_sample_line_counts_as_a_race_log() {
    let raw = "{\"v\":1,\"file\":\"x\"}\n{\"t\":1.2,\"seq\":3,\"track\":\"Glen\",\"cur\":2,\"time\":8000}\n";
    assert!(raw_has_race(raw));
}

#[test]
fn snapshot_uses_in_memory_samples_without_reading_the_live_file() {
    let dir = std::env::temp_dir().join(format!("holeshot-hud-snapshot-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("race.jsonl");
    let raw =
        "{\"v\":1}\n{\"t\":1.2,\"seq\":3,\"track\":\"Glen\",\"cur\":2,\"time\":8000}\n".into();
    let log = snapshot_log(raw, path, false).expect("samples");
    assert_eq!(log.track.as_deref(), Some("Glen"));
    assert_eq!(
        log.body,
        "{\"v\":1}\n{\"t\":1.2,\"seq\":3,\"track\":\"Glen\",\"cur\":2,\"time\":8000}\n"
    );
    assert!(!log.truncated);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn trim_body_keeps_only_a_small_tail() {
    let mut body = String::from("{\"v\":1}\n");
    while body.len() <= MAX_LOG {
        body.push_str("{\"t\":1.0,\"cur\":1}\n");
    }
    assert!(body.len() > MAX_LOG);
    assert!(trim_body(&mut body));
    assert!(body.len() <= MAX_LOG);
    assert!(raw_has_race(&body));
    assert!(!body.starts_with("{\"v\":1}"));
    assert!(body.lines().last().unwrap().contains("\"cur\":1"));
}

#[test]
fn clip_send_keeps_the_latest_tail() {
    let mut body = String::from("{\"v\":1}\n");
    while body.len() <= MAX_SEND_LOG {
        body.push_str("{\"t\":1.0,\"cur\":1}\n");
    }
    body.push_str("{\"t\":9.0,\"cur\":2,\"track\":\"Glen\"}\n");
    let (clipped, truncated) = clip_send(body, false);
    assert!(truncated);
    assert!(clipped.len() <= MAX_SEND_LOG);
    assert!(clipped.ends_with("{\"t\":9.0,\"cur\":2,\"track\":\"Glen\"}\n"));
    assert!(!clipped.contains("{\"v\":1}"));
    assert!(raw_has_race(&clipped));
}

#[test]
fn clip_send_passthrough_keeps_truncated_flag() {
    let body = "{\"v\":1}\n{\"t\":1.0,\"cur\":1}\n".to_string();
    let (clipped, truncated) = clip_send(body.clone(), true);
    assert_eq!(clipped, body);
    assert!(truncated);
}

#[test]
fn second_empty_after_race_ended_stays_continue() {
    let mut g = SessionGate::default();
    assert_eq!(g.update("Glen", 4, 0, 30_000, 2, 1), SessionEvent::Continue);
    assert_eq!(g.update("", 0, 0, 0, 0, 0), SessionEvent::RaceEnded);
    assert_eq!(g.update("", 0, 0, 0, 0, 0), SessionEvent::Continue);
}

#[test]
fn racing_thresholds_need_clock_or_lap() {
    let mut g = SessionGate::default();
    assert_eq!(g.update("Glen", 4, 0, 7_000, 0, 0), SessionEvent::Continue);
    assert!(!g.saw_race);
    assert_eq!(g.update("Glen", 4, 0, 8_001, 0, 0), SessionEvent::Continue);
    assert!(g.saw_race);
    let mut g = SessionGate::default();
    assert_eq!(g.update("Glen", 4, 0, 3_001, 0, 1), SessionEvent::Continue);
    assert!(g.saw_race);
}

fn folded_place_snap() -> mxbo_hud::shm::Snapshot {
    use mxbo_hud::shm::{write_name, Rider, Snapshot, Standing};
    let mut snap = Snapshot::default();
    snap.track_length = 200.0;
    snap.focus_race_num = 12;
    snap.local_race_num = 12;
    snap.standing_count = 2;
    snap.rider_count = 2;
    snap.standings[0] = Standing {
        race_num: 1,
        position: 1,
        num_laps: 5,
        ..Standing::default()
    };
    snap.standings[1] = Standing {
        race_num: 12,
        position: 2,
        num_laps: 5,
        ..Standing::default()
    };
    snap.riders[0] = Rider {
        race_num: 1,
        x: 50.0,
        z: 0.0,
        track_pos: 0.25,
        ..Rider::default()
    };
    snap.riders[1] = Rider {
        race_num: 12,
        x: 50.0,
        z: 4.0,
        track_pos: 0.75,
        ..Rider::default()
    };
    write_name(&mut snap.track_name, "SX");
    snap
}

#[test]
fn place_line_logs_a_folded_pair_once() {
    let snap = folded_place_snap();
    let mut trace = PlaceTrace::default();
    let line = trace
        .consider(&snap, &[1, 12], 1_500)
        .expect("first order");
    assert!(line.contains("\"evt\":\"place\""), "{line}");
    assert!(line.contains("\"track\":\"SX\""), "{line}");
    assert!(line.contains("\"you\":12"), "{line}");
    assert!(line.contains("\"live\":2"), "{line}");
    assert!(line.contains("\"frac\":0.250"), "{line}");
    assert!(line.contains("\"frac\":0.750"), "{line}");
    assert!(line.contains("\"x\":50.0"), "{line}");
    assert!(line.contains("\"z\":4.0"), "{line}");
    assert!(trace.consider(&snap, &[1, 12], 2_000).is_none());
    assert!(trace.consider(&snap, &[], 2_000).is_none());
}
