use super::*;
use crate::config::{BoardField, DashField, FontFamily, HudConfig, RelField, StField};
use crate::shm::{write_name, Point, Rider, Snapshot, Standing, MAGIC, VERSION};
use std::sync::{Mutex, OnceLock};
use tiny_skia::Pixmap;

fn session_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn reset_session() {
    reset_session_clock_track();
    LAST_SESSION_SIG.store(0, Ordering::Relaxed);
    LAST_CUR_LAP.store(0, Ordering::Relaxed);
}

fn standing(race_num: i32, position: i32, laps: i32) -> Standing {
    let mut row = Standing::default();
    row.race_num = race_num;
    row.position = position;
    row.num_laps = laps;
    write_name(&mut row.name, &format!("R{race_num}"));
    write_name(&mut row.bike, "YZ450");
    write_name(&mut row.category, "MX1");
    row
}

fn rider(race_num: i32, x: f32, z: f32, pos: f32) -> Rider {
    let mut r = Rider::default();
    r.race_num = race_num;
    r.x = x;
    r.z = z;
    r.track_pos = pos;
    write_name(&mut r.name, &format!("R{race_num}"));
    r
}

fn live_snap() -> Snapshot {
    let mut s = Snapshot {
        magic: MAGIC,
        version: VERSION,
        on_track: 1,
        has_telemetry: 1,
        local_race_num: 12,
        focus_race_num: 12,
        local_speed: 18.0,
        current_lap: 6,
        track_length: 1000.0,
        sf_meters: 0.0,
        local_track_pos: 0.92,
        local_x: 10.0,
        local_z: 4.0,
        standing_count: 2,
        rider_count: 2,
        show_standings: 1,
        show_relative: 1,
        show_map: 1,
        standings_rows: 12,
        relative_count: 3,
        local_gear: 3,
        local_rpm: 7200,
        engine_temp: 82.0,
        air_temp: 21.0,
        last_lap_ms: 95_000,
        current_lap_ms: 40_000,
        best_lap_ms: 93_500,
        ..Snapshot::default()
    };
    s.standings[0] = standing(1, 1, 5);
    s.standings[0].best_lap_ms = 92_000;
    s.standings[1] = standing(12, 2, 5);
    s.standings[1].gap_ms = 2_400;
    s.standings[1].best_lap_ms = 93_500;
    s.standings[1].last_lap_ms = 95_000;
    s.riders[0] = rider(1, 20.0, 8.0, 0.10);
    s.riders[1] = rider(12, 10.0, 4.0, 0.92);
    write_name(&mut s.track_name, "Test Track");
    let n = 24;
    s.poly_count = n;
    for i in 0..n as usize {
        let a = i as f32 / n as f32 * std::f32::consts::TAU;
        s.poly[i] = Point {
            x: a.cos() * 80.0,
            z: a.sin() * 50.0,
        };
    }
    s
}

fn expire_timed(s: &mut Snapshot) {
    reset_session();
    s.session_length = 8;
    s.session_laps = 2;
    s.session_time_ms = 8 * 60 * 1000;
    s.current_lap = 6;
    s.local_speed = 18.0;
    let remain = session_remain_ms(s).expect("countdown while time remains");
    assert!(remain > 60_000, "expected a real countdown, got {remain}");
    s.session_time_ms = 400;
    let remain = session_remain_ms(s).expect("expired timed session");
    assert_eq!(remain, 0);
    assert!(overtime_active(s));
}

fn fonts() -> Fonts {
    Fonts::for_family(FontFamily::Roboto).expect("bundled Roboto")
}

fn draw_ok(s: &Snapshot, cfg: &HudConfig) {
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut px, &fonts(), Some(s), cfg, 1280, 720, 0.0, false, false);
}

#[test]
fn formatters_cover_clock_gap_and_penalty() {
    assert_eq!(format_countdown(0), "00:00");
    assert_eq!(format_countdown(125_000), "02:05");
    assert_eq!(format_session_clock(0), "--:--:--");
    assert_eq!(format_session_clock(3661_000), "01:01:01");
    assert_eq!(session_len_ms(8), 8 * 60_000);
    assert_eq!(session_len_ms(480), 480_000);
    assert_eq!(format_clock(0), "--:--.---");
    assert!(format_clock(93_500).starts_with("01:"));
    assert_eq!(format_board_gap(0, 0, true), "-");
    assert_eq!(format_board_gap(0, 1, false), "1L");
    assert_eq!(format_board_gap(1500, 0, false), "1.5");
    assert_eq!(format_penalty(0), "---");
    assert_eq!(format_delta_ms(0), "0.000");
    assert!(format_delta_ms(250).starts_with('+'));
}

#[test]
fn dash_footer_fields_fill_from_live_snapshot() {
    let _g = session_lock();
    reset_session();
    let s = live_snap();
    let cfg = HudConfig::new();
    assert_eq!(dash_foot_item(&s, &cfg, DashField::None), None);
    let speed = dash_foot_item(&s, &cfg, DashField::Speed).unwrap().1;
    assert!(speed.contains("KPH"), "{speed}");
    assert_eq!(dash_foot_item(&s, &cfg, DashField::Rpm).unwrap().1, "7200");
    assert_eq!(dash_foot_item(&s, &cfg, DashField::Gear).unwrap().1, "3");
    assert_eq!(dash_foot_item(&s, &cfg, DashField::Position).unwrap().1, "P2");
    assert_eq!(dash_foot_item(&s, &cfg, DashField::Number).unwrap().1, "#12");
    assert_eq!(dash_foot_item(&s, &cfg, DashField::Air).unwrap().1, "21°C");
    assert_eq!(dash_foot_item(&s, &cfg, DashField::Engine).unwrap().1, "82°C");
    assert_eq!(dash_foot_item(&s, &cfg, DashField::Bike).unwrap().1, "YZ450");
    assert_eq!(dash_foot_item(&s, &cfg, DashField::Class).unwrap().1, "MX1");
    for field in DashField::ALL {
        if field == DashField::None {
            continue;
        }
        assert!(dash_foot_item(&s, &cfg, field).is_some(), "{field:?}");
    }
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
    let mut timed = s;
    timed.session_length = 8;
    timed.session_laps = 0;
    assert_eq!(board_item(&timed, &cfg, BoardField::SessionType).unwrap().1, "Timed");
    timed.session_laps = 12;
    timed.session_length = 0;
    assert_eq!(board_item(&timed, &cfg, BoardField::Lap).unwrap().1, "6 / 12");
    assert_eq!(board_item(&timed, &cfg, BoardField::LapsLeft).unwrap().1, "6");
    assert_eq!(board_item(&timed, &cfg, BoardField::SessionType).unwrap().1, "Lap race");
    for field in BoardField::ALL {
        if field == BoardField::None {
            continue;
        }
        assert!(board_item(&s, &cfg, field).is_some(), "{field:?}");
    }
    assert!(!cfg.standings_cols().is_empty());
    assert!(cfg.standings_cols().contains(&StField::Name));
    assert!(cfg.relative_cols().contains(&RelField::Name));
}

#[test]
fn timed_session_stays_at_zero_until_local_cross_or_leader_lap() {
    let _g = session_lock();
    let mut s = live_snap();
    expire_timed(&mut s);
    assert_eq!(session_banner(&s).1, "00:00");
    s.standings[1].num_laps = 6;
    s.current_lap = 7;
    assert_eq!(session_banner(&s).1, "1 / 2");

    let mut s = live_snap();
    expire_timed(&mut s);
    s.standings[0].num_laps = 6;
    s.standings[1].num_laps = 6;
    s.current_lap = 7;
    assert_eq!(session_banner(&s).1, "1 / 2");
    s.standings[0].num_laps = 7;
    assert_eq!(session_banner(&s).1, "2 / 2");
}

#[test]
fn white_flag_waits_for_local_last_lap_not_leader_crossing() {
    let _g = session_lock();
    let mut s = live_snap();
    expire_timed(&mut s);
    s.local_speed = 18.0;
    s.local_track_pos = 0.95;
    s.riders[1].track_pos = 0.95;
    assert_eq!(dash_race_flag(&s), DashFlag::None);
    s.standings[0].num_laps = 6;
    assert_eq!(dash_race_flag(&s), DashFlag::None);
    s.standings[0].num_laps = 7;
    s.standings[1].num_laps = 6;
    s.current_lap = 7;
    assert_eq!(dash_race_flag(&s), DashFlag::White);
    s.standings[1].num_laps = 7;
    s.current_lap = 8;
    assert_eq!(dash_race_flag(&s), DashFlag::Checkered);
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
fn approaching_sf_uses_meters_before_the_line() {
    let mut s = live_snap();
    s.track_length = 1000.0;
    s.sf_meters = 0.0;
    s.local_track_pos = 0.95;
    s.riders[1].track_pos = 0.95;
    s.has_telemetry = 1;
    assert!(approaching_sf(&s));
    s.local_track_pos = 0.80;
    s.riders[1].track_pos = 0.80;
    assert!(!approaching_sf(&s));
    s.local_track_pos = 0.001;
    s.riders[1].track_pos = 0.001;
    assert!(!approaching_sf(&s));
}

#[test]
fn radar_same_stretch_wraps_around_the_lap() {
    let mut s = live_snap();
    s.track_length = 1000.0;
    s.local_track_pos = 0.98;
    assert!(radar_same_stretch(&s, 0.02, 80.0));
    assert!(!radar_same_stretch(&s, 0.40, 80.0));
}

#[test]
fn minimap_keeps_sparse_track_segments_near_the_rider() {
    let mut s = live_snap();
    s.poly_count = 4;
    s.poly[0] = Point { x: -200.0, z: 0.0 };
    s.poly[1] = Point { x: 200.0, z: 0.0 };
    s.poly[2] = Point { x: 200.0, z: 40.0 };
    s.poly[3] = Point { x: -200.0, z: 40.0 };
    let mut pb = PathBuilder::new();
    append_visible_track(&mut pb, &s, 4, 0.0, 0.0, 90.0, &|x, z| (x, z));
    assert!(pb.finish().is_some(), "segment through the rider should stay visible");
}

#[test]
fn each_widget_renders_without_panic() {
    let _g = session_lock();
    reset_session();
    let s = live_snap();
    let mut cfg = HudConfig::new();
    draw_ok(&s, &cfg);

    cfg.show_minimap = false;
    cfg.show_radar = false;
    cfg.show_dash = false;
    let mut only = s;
    only.show_relative = 0;
    only.show_map = 0;
    only.show_standings = 1;
    draw_ok(&only, &cfg);

    only.show_standings = 0;
    only.show_relative = 1;
    draw_ok(&only, &cfg);

    only.show_relative = 0;
    only.show_map = 1;
    draw_ok(&only, &cfg);

    only.show_map = 0;
    cfg.show_minimap = true;
    draw_ok(&only, &cfg);

    cfg.show_minimap = false;
    cfg.show_radar = true;
    draw_ok(&only, &cfg);

    cfg.show_radar = false;
    cfg.show_dash = true;
    draw_ok(&only, &cfg);
}

#[test]
fn hidden_widgets_do_not_need_live_telemetry() {
    let _g = session_lock();
    reset_session();
    let mut cfg = HudConfig::new();
    cfg.show_minimap = false;
    cfg.show_radar = false;
    cfg.show_dash = false;
    let mut s = Snapshot::default();
    s.on_track = 1;
    s.show_standings = 0;
    s.show_relative = 0;
    s.show_map = 0;
    draw_ok(&s, &cfg);
}
