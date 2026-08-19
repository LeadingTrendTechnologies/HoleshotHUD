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
    HS_SCROLL.with(|a| {
        *a.borrow_mut() = IndexSlide {
            from: 0.0,
            to: 0.0,
            start: 0.0,
            init: false,
        };
    });
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
    expire_timed_extras(s, 2);
}

fn expire_timed_extras(s: &mut Snapshot, extras: i32) {
    reset_session();
    s.session_length = 8;
    s.session_laps = extras;
    s.session_time_ms = 30_000;
    s.current_lap = 1;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    let _ = session_remain_ms(s);
    s.session_time_ms = 8 * 60 * 1000;
    s.current_lap = 6;
    s.local_speed = 18.0;
    s.standings[0].num_laps = 5;
    s.standings[1].num_laps = 5;
    let remain = session_remain_ms(s).expect("countdown while time remains");
    assert!(remain > 60_000, "expected a real countdown, got {remain}");
    s.session_time_ms = 8 * 60 * 1000 - 1_000;
    let remain = session_remain_ms(s).expect("ticking race clock");
    assert!(remain < 8 * 60 * 1000);
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
    assert_eq!(format_local_clock(0, 5), "12:05 AM");
    assert_eq!(format_local_clock(9, 14), "9:14 AM");
    assert_eq!(format_local_clock(12, 0), "12:00 PM");
    assert_eq!(format_local_clock(21, 7), "9:07 PM");
    assert_eq!(fmt_sys_mem(0.0), "0 MB");
    assert_eq!(fmt_sys_mem(88.0), "88 MB");
    assert_eq!(fmt_sys_mem(1800.0), "1.8 GB");
}

#[test]
fn bike_colors_match_factory_brands() {
    let ktm = bike_color("450 SX-F", "MX1");
    assert!(ktm.red() > 0.9 && ktm.green() > 0.2 && ktm.blue() < 0.12, "KTM should be orange");
    let husky = bike_color("FC 450", "MX1");
    assert!(husky.red() > 0.9 && husky.green() > 0.9, "Husqvarna should be white");
    let yamaha = bike_color("YZ450F", "MX1");
    assert!(yamaha.blue() > yamaha.red() && yamaha.blue() > yamaha.green(), "Yamaha should be blue");
    let honda = bike_color("CRF450R", "MX1");
    assert!(honda.red() > 0.7 && honda.green() < 0.25, "Honda should be red");
    let kawi = bike_color("KX450", "MX1");
    assert!(kawi.green() > kawi.red() && kawi.green() > kawi.blue(), "Kawasaki should be green");
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
    assert_eq!(board_item(&timed, &cfg, BoardField::Lap).unwrap().1, session_banner(&timed).1);
    assert_eq!(dash_foot_item(&timed, &cfg, DashField::LapCount).unwrap().1, session_banner(&timed).1);
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
    let mut s = live_snap();
    assert_eq!(lap_rel(&s, 1), LapRel::Same);

    s.standings[0].num_laps = 6;
    s.riders[0].track_pos = 0.80;
    s.local_track_pos = 0.92;
    s.riders[1].track_pos = 0.92;
    assert_eq!(lap_rel(&s, 1), LapRel::LappingMe);

    s.standings[0].num_laps = 6;
    s.riders[0].track_pos = 0.02;
    s.local_track_pos = 0.98;
    s.riders[1].track_pos = 0.98;
    assert_eq!(lap_rel(&s, 1), LapRel::Same);

    s.standings[0].num_laps = 4;
    s.riders[0].track_pos = 0.97;
    s.local_track_pos = 0.90;
    s.riders[1].track_pos = 0.90;
    assert_eq!(lap_rel(&s, 1), LapRel::LappedByMe);

    s.standings[0].num_laps = 6;
    s.riders[0].track_pos = 0.20;
    s.local_track_pos = 0.90;
    s.riders[1].track_pos = 0.90;
    assert_eq!(lap_rel(&s, 1), LapRel::Same, "lap up but not closing from behind");

    s.standings[0].num_laps = 5;
    s.standings[0].gap_laps = 0;
    s.standings[1].gap_laps = 1;
    s.riders[0].track_pos = 0.50;
    s.local_track_pos = 0.50;
    s.riders[1].track_pos = 0.50;
    assert_eq!(lap_rel(&s, 1), LapRel::Same);

    s.riders[0].track_pos = 0.82;
    s.local_track_pos = 0.90;
    s.riders[1].track_pos = 0.90;
    assert_eq!(lap_rel(&s, 1), LapRel::LappingMe);

    s.standings[0].gap_laps = 2;
    s.standings[1].gap_laps = 1;
    s.riders[0].track_pos = 0.96;
    s.local_track_pos = 0.88;
    s.riders[1].track_pos = 0.88;
    assert_eq!(lap_rel(&s, 1), LapRel::LappedByMe);
    assert_eq!(lap_rel(&s, 12), LapRel::Same);
}

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
    s.session_length = 8;
    s.session_laps = 0;
    assert!(!is_warmup(&s));
    assert_eq!(lap_rel(&s, 1), LapRel::LappingMe);
}

fn col_right(slots: &[(StField, f32, f32)]) -> f32 {
    slots.last().map(|(_, x, w)| x + w).unwrap_or(0.0)
}

#[test]
fn table_columns_fill_the_row_when_fields_change() {
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
    assert!((col_right(&few) - (avail - pad)).abs() < 0.51);
    assert!((col_right(&many) - (avail - pad)).abs() < 0.51);
    let name_few = few.iter().find(|(c, _, _)| *c == StField::Name).unwrap().2;
    let name_many = many.iter().find(|(c, _, _)| *c == StField::Name).unwrap().2;
    assert!(name_few > name_many);
}

#[test]
fn timed_session_shows_zero_of_extra_laps_until_local_cross() {
    let _g = session_lock();
    let mut s = live_snap();
    expire_timed(&mut s);
    assert_eq!(session_banner(&s).1, "0 / 2");
    s.standings[0].num_laps = 6;
    assert_eq!(session_banner(&s).1, "0 / 2", "leader cross must not increment");
    s.standings[1].num_laps = 6;
    s.current_lap = 7;
    assert_eq!(session_banner(&s).1, "1 / 2", "first extra cross starts the extra");
    s.standings[1].num_laps = 7;
    s.current_lap = 8;
    assert_eq!(session_banner(&s).1, "2 / 2");
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
    assert_eq!(session_banner(&s).1, "0 / 2");
    s.standings[1].num_laps = 3;
    s.current_lap = 4;
    assert_eq!(
        session_banner(&s).1,
        "0 / 2",
        "local cross before the leader starts extras must not count"
    );
    s.standings[0].num_laps = 4;
    assert_eq!(session_banner(&s).1, "0 / 2");
    s.standings[1].num_laps = 4;
    s.current_lap = 5;
    assert_eq!(session_banner(&s).1, "1 / 2", "first extra cross starts the extra");
    s.standings[0].num_laps = 5;
    s.standings[1].num_laps = 5;
    s.current_lap = 6;
    assert_eq!(session_banner(&s).1, "2 / 2");
    s.standings[0].num_laps = 6;
    s.standings[1].num_laps = 6;
    s.current_lap = 7;
    assert_eq!(session_banner(&s).1, "2 / 2");
}

#[test]
fn timed_plus_one_cross_before_leader_stays_zero_until_next_pass() {
    let _g = session_lock();
    let mut s = live_snap();
    expire_timed_extras(&mut s, 1);
    s.local_speed = 18.0;
    s.local_track_pos = 0.97;
    s.riders[1].track_pos = 0.97;
    assert_eq!(session_banner(&s).1, "0 / 1");
    s.standings[1].num_laps = 6;
    s.current_lap = 7;
    assert_eq!(
        session_banner(&s).1,
        "0 / 1",
        "pass after time expire does not count until the leader starts extras"
    );
    s.standings[0].num_laps = 6;
    assert_eq!(session_banner(&s).1, "0 / 1", "leader start extras still 0 / 1 until you pass");
    s.local_track_pos = 0.40;
    s.riders[1].track_pos = 0.40;
    assert_eq!(dash_race_flag(&s), DashFlag::None);
    s.local_track_pos = 0.97;
    s.riders[1].track_pos = 0.97;
    assert_eq!(dash_race_flag(&s), DashFlag::White);
    s.standings[1].num_laps = 7;
    s.current_lap = 8;
    s.local_track_pos = 0.02;
    s.riders[1].track_pos = 0.02;
    expire_white_hold();
    assert_eq!(session_banner(&s).1, "1 / 1");
    assert_ne!(dash_race_flag(&s), DashFlag::Checkered);
    s.local_track_pos = 0.40;
    s.riders[1].track_pos = 0.40;
    let _ = dash_race_flag(&s);
    s.local_track_pos = 0.97;
    s.riders[1].track_pos = 0.97;
    assert_eq!(dash_race_flag(&s), DashFlag::Checkered);
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
    assert_eq!(session_banner(&s).1, "0 / 2");
    s.standings[1].num_laps = 5;
    assert_eq!(session_banner(&s).1, "0 / 2");
    s.standings[0].num_laps = 6;
    assert_eq!(session_banner(&s).1, "0 / 2");
    s.standings[1].num_laps = 6;
    s.current_lap = 7;
    assert_eq!(session_banner(&s).1, "1 / 2", "first extra cross starts the extra");
    s.standings[1].num_laps = 7;
    s.current_lap = 8;
    assert_eq!(session_banner(&s).1, "2 / 2");
}

#[test]
fn white_flag_waits_for_local_last_lap_not_leader_crossing() {
    let _g = session_lock();
    let mut s = live_snap();
    expire_timed(&mut s);
    s.local_speed = 18.0;
    s.local_track_pos = 0.97;
    s.riders[1].track_pos = 0.97;
    assert_eq!(dash_race_flag(&s), DashFlag::None, "no flags until extras start");
    s.local_track_pos = 0.88;
    s.riders[1].track_pos = 0.88;
    assert_eq!(dash_race_flag(&s), DashFlag::None);
    s.standings[0].num_laps = 6;
    assert_eq!(dash_race_flag(&s), DashFlag::None);
    s.standings[0].num_laps = 7;
    s.standings[1].num_laps = 5;
    s.current_lap = 6;
    s.local_track_pos = 0.40;
    s.riders[1].track_pos = 0.40;
    assert_eq!(dash_race_flag(&s), DashFlag::None);
    s.local_track_pos = 0.97;
    s.riders[1].track_pos = 0.97;
    assert_eq!(dash_race_flag(&s), DashFlag::White);
    s.standings[1].num_laps = 6;
    s.current_lap = 7;
    assert_eq!(dash_race_flag(&s), DashFlag::White, "white holds through the extra start");
    s.local_track_pos = 0.40;
    s.riders[1].track_pos = 0.40;
    assert_eq!(dash_race_flag(&s), DashFlag::White);
    expire_white_hold();
    assert_eq!(dash_race_flag(&s), DashFlag::None);
    s.local_track_pos = 0.97;
    s.riders[1].track_pos = 0.97;
    assert_eq!(dash_race_flag(&s), DashFlag::White, "first extra finish is still not checkered on +2");
    s.standings[1].num_laps = 7;
    s.current_lap = 8;
    assert_eq!(dash_race_flag(&s), DashFlag::White);
    s.local_track_pos = 0.40;
    s.riders[1].track_pos = 0.40;
    expire_white_hold();
    let _ = dash_race_flag(&s);
    s.local_track_pos = 0.97;
    s.riders[1].track_pos = 0.97;
    assert_eq!(dash_race_flag(&s), DashFlag::Checkered);
    s.standings[1].num_laps = 8;
    s.current_lap = 9;
    s.local_track_pos = 0.80;
    s.riders[1].track_pos = 0.80;
    assert_eq!(dash_race_flag(&s), DashFlag::Checkered, "checkered holds until you leave the session");
    s.on_track = 0;
    assert_eq!(dash_race_flag(&s), DashFlag::None);
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
    s.local_track_pos = 0.40;
    s.riders[1].track_pos = 0.40;
    assert_eq!(dash_race_flag(&s), DashFlag::None);
    s.local_track_pos = 0.97;
    s.riders[1].track_pos = 0.97;
    assert_eq!(dash_race_flag(&s), DashFlag::White, "white as you start the last extra");
    s.standings[0].num_laps = 6;
    s.standings[1].num_laps = 5;
    s.current_lap = 6;
    assert_eq!(dash_race_flag(&s), DashFlag::White);
    s.standings[1].num_laps = 6;
    s.current_lap = 7;
    s.local_track_pos = 0.40;
    s.riders[1].track_pos = 0.40;
    assert_eq!(dash_race_flag(&s), DashFlag::White);
    expire_white_hold();
    assert_eq!(dash_race_flag(&s), DashFlag::None);
    s.local_track_pos = 0.97;
    s.riders[1].track_pos = 0.97;
    assert_eq!(dash_race_flag(&s), DashFlag::Checkered);
    s.standings[1].num_laps = 6;
    s.current_lap = 7;
    s.local_track_pos = 0.20;
    s.riders[1].track_pos = 0.20;
    assert_eq!(dash_race_flag(&s), DashFlag::Checkered);
    s.on_track = 0;
    assert_eq!(dash_race_flag(&s), DashFlag::None);
}

#[test]
fn timed_plus_one_first_extra_crossing_is_white_not_checkered() {
    let _g = session_lock();
    let mut s = live_snap();
    expire_timed_extras(&mut s, 1);
    s.local_speed = 18.0;
    s.local_track_pos = 0.40;
    s.riders[1].track_pos = 0.40;
    s.standings[0].num_laps = 6;
    s.standings[1].num_laps = 5;
    s.current_lap = 6;
    assert_eq!(dash_race_flag(&s), DashFlag::None);
    s.local_track_pos = 0.97;
    s.riders[1].track_pos = 0.97;
    assert_eq!(
        dash_race_flag(&s),
        DashFlag::White,
        "first extra approach after extras start is white"
    );
    assert_ne!(dash_race_flag(&s), DashFlag::Checkered);
    s.standings[1].num_laps = 6;
    s.current_lap = 7;
    s.local_track_pos = 0.02;
    s.riders[1].track_pos = 0.02;
    expire_white_hold();
    assert_eq!(session_banner(&s).1, "1 / 1", "last-lap start shows 1 / 1");
    assert_ne!(dash_race_flag(&s), DashFlag::Checkered, "checkered waits for the next approach");
    s.local_track_pos = 0.40;
    s.riders[1].track_pos = 0.40;
    let _ = dash_race_flag(&s);
    s.local_track_pos = 0.97;
    s.riders[1].track_pos = 0.97;
    assert_eq!(dash_race_flag(&s), DashFlag::Checkered);
    assert_eq!(session_banner(&s).1, "1 / 1", "last-lap start shows 1 / 1");
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
    assert_eq!(session_banner(&s).1, "0 / 1");
    s.standings[0].num_laps = 1;
    assert_eq!(session_banner(&s).1, "0 / 1", "leader standings recovery is not extras start");
    s.standings[1].num_laps = 1;
    s.current_lap = 2;
    assert_eq!(session_banner(&s).1, "0 / 1", "timed-lap finish after recovery is not an extra");
    s.local_track_pos = 0.97;
    s.riders[1].track_pos = 0.97;
    s.standings[0].num_laps = 2;
    assert_eq!(dash_race_flag(&s), DashFlag::White);
    assert_eq!(session_banner(&s).1, "0 / 1");
    s.standings[1].num_laps = 2;
    s.current_lap = 3;
    s.local_track_pos = 0.02;
    s.riders[1].track_pos = 0.02;
    expire_white_hold();
    assert_eq!(session_banner(&s).1, "1 / 1", "last-lap start shows 1 / 1");
    assert_ne!(dash_race_flag(&s), DashFlag::Checkered);
    s.local_track_pos = 0.40;
    s.riders[1].track_pos = 0.40;
    let _ = dash_race_flag(&s);
    s.local_track_pos = 0.97;
    s.riders[1].track_pos = 0.97;
    assert_eq!(dash_race_flag(&s), DashFlag::Checkered);
    s.standings[1].num_laps = 3;
    s.current_lap = 4;
    assert_eq!(session_banner(&s).1, "1 / 1");
    let cfg = HudConfig::new();
    assert_eq!(board_item(&s, &cfg, BoardField::Lap).unwrap().1, "1 / 1");
    assert_eq!(board_item(&s, &cfg, BoardField::Session).unwrap().1, "1 / 1");
    assert_eq!(dash_foot_item(&s, &cfg, DashField::LapCount).unwrap().1, "1 / 1");
    assert_eq!(ticker_meta_label(BoardField::Lap, "1 / 1"), "LAPS");
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
    s.standings[1].num_laps = 1;
    s.current_lap = 2;
    s.local_track_pos = 0.40;
    s.riders[1].track_pos = 0.40;
    assert_eq!(dash_race_flag(&s), DashFlag::None);
    s.local_track_pos = 0.97;
    s.riders[1].track_pos = 0.97;
    assert_eq!(dash_race_flag(&s), DashFlag::None, "penultimate approach is not white");
    assert_eq!(session_banner(&s).1, "2 / 3");
    s.standings[1].num_laps = 2;
    s.current_lap = 3;
    assert_eq!(session_banner(&s).1, "3 / 3");
    assert_eq!(dash_race_flag(&s), DashFlag::White, "white when last lap starts");
    s.local_track_pos = 0.40;
    s.riders[1].track_pos = 0.40;
    assert_eq!(dash_race_flag(&s), DashFlag::White);
    expire_white_hold();
    assert_eq!(dash_race_flag(&s), DashFlag::None);
    s.local_track_pos = 0.97;
    s.riders[1].track_pos = 0.97;
    assert_eq!(dash_race_flag(&s), DashFlag::Checkered);
    s.standings[1].num_laps = 3;
    s.current_lap = 4;
    s.local_track_pos = 0.80;
    s.riders[1].track_pos = 0.80;
    assert_eq!(dash_race_flag(&s), DashFlag::Checkered);
    s.on_track = 0;
    assert_eq!(dash_race_flag(&s), DashFlag::None);
}

#[test]
fn lap_race_checkered_waits_until_finish_approach() {
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
    s.local_track_pos = 0.97;
    s.riders[1].track_pos = 0.97;
    assert_eq!(session_banner(&s).1, "3 / 4");
    assert_eq!(dash_race_flag(&s), DashFlag::None, "3 / 4 approach is not last lap");
    s.standings[1].num_laps = 3;
    s.current_lap = 4;
    assert_eq!(session_banner(&s).1, "4 / 4");
    assert_eq!(dash_race_flag(&s), DashFlag::White);
    assert_ne!(dash_race_flag(&s), DashFlag::Checkered);
    expire_white_hold();
    s.local_track_pos = 0.40;
    s.riders[1].track_pos = 0.40;
    assert_eq!(dash_race_flag(&s), DashFlag::None);
    s.local_track_pos = 0.97;
    s.riders[1].track_pos = 0.97;
    assert_eq!(dash_race_flag(&s), DashFlag::Checkered);
    s.local_track_pos = 0.20;
    s.riders[1].track_pos = 0.20;
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
    assert_eq!(session_banner(&s).1, "0 / 1");
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
    assert_eq!(session_banner(&s).1, "0 / 2");
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
    assert_eq!(session_banner(&s).1, "00:00");
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

#[test]
fn later_start_board_does_not_show_locked_eight_minutes() {
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
    assert_eq!(session_banner(&s).1, "00:45");
    s.session_time_ms = 479_328;
    s.local_speed = 3.8;
    let _ = session_remain_ms(&s);
    s.session_time_ms = 478_998;
    s.local_speed = 5.8;
    assert_eq!(session_banner(&s).1, "07:58");
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
    assert_eq!(session_banner(&s).1, "0 / 2");
    s.standings[0].num_laps = 6;
    s.current_lap = 7;
    s.standings[1].num_laps = 6;
    assert_eq!(session_banner(&s).1, "1 / 2", "first extra cross starts the extra");
    s.standings[1].num_laps = 7;
    s.current_lap = 8;
    assert_eq!(session_banner(&s).1, "2 / 2");
}

#[test]
fn approaching_sf_uses_meters_before_the_line() {
    let mut s = live_snap();
    s.track_length = 1000.0;
    s.sf_meters = 0.0;
    s.local_track_pos = 0.97;
    s.riders[1].track_pos = 0.97;
    s.has_telemetry = 1;
    assert!(approaching_sf(&s));
    assert!(approaching_finish(&s));
    s.local_track_pos = 0.70;
    s.riders[1].track_pos = 0.70;
    assert!(!approaching_sf(&s));
    assert!(!approaching_finish(&s));
    s.local_track_pos = 0.001;
    s.riders[1].track_pos = 0.001;
    assert!(!approaching_sf(&s));
    assert!(!approaching_finish(&s));
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
fn radar_in_view_covers_blind_spots() {
    assert!(radar_in_view(-3.0, -2.0, true, false));
    assert!(radar_in_view(-3.0, -2.0, false, true));
    assert!(!radar_in_view(-3.0, -2.0, false, false));
    assert!(radar_in_view(0.0, 2.0, true, false));
    assert!(!radar_in_view(0.0, 2.0, false, true));
    assert!(radar_in_view(-4.0, 0.0, false, true));
    assert!(!radar_in_view(-4.0, 0.0, true, false));
    assert!(!radar_in_view(2.0, 0.0, true, true));
    assert!(!radar_in_view(-13.0, 0.0, true, true));
    assert!(!radar_in_view(0.0, 7.0, true, true));
}

#[test]
fn radar_to_screen_puts_left_rear_below_and_left() {
    let (ox, oy, sx, sy) = (50.0, 40.0, 5.0, 4.0);
    let (you_x, you_y) = radar_to_screen(0.0, 0.0, ox, oy, sx, sy);
    let (lx, ly) = radar_to_screen(-4.0, -2.0, ox, oy, sx, sy);
    assert!(lx < you_x);
    assert!(ly > you_y);
    let (rx, ry) = radar_to_screen(0.0, 2.5, ox, oy, sx, sy);
    assert!(rx > you_x);
    assert!((ry - you_y).abs() < 0.01);
}

#[test]
fn radar_blip_heat_rises_when_closer() {
    assert!(radar_blip_heat(1.0) > radar_blip_heat(7.0));
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
fn minimap_zoom_stays_on_a_local_section() {
    let near = mini_view_radius(100);
    let far = mini_view_radius(0);
    let mid = mini_view_radius(70);
    assert!(near >= 20.0 && near <= 26.0);
    assert!(far >= 80.0 && far <= 90.0);
    assert!((mid - 40.0).abs() < 2.0);
    assert!(far > near);
    assert!(far < 120.0);
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
    cfg.show_ticker = false;
    cfg.show_sys = false;
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

    cfg.show_dash = false;
    cfg.show_ticker = true;
    draw_ok(&only, &cfg);
    cfg.ticker_autoscroll = true;
    draw_ok(&only, &cfg);

    cfg.show_ticker = false;
    cfg.show_sys = true;
    set_sys_stats(48.0, 62.0, 91.0, 11.0);
    set_sys_procs([
        SysProc {
            cpu: 4.0,
            mem_mb: 88.0,
            mem_pct: 0.5,
            on: true,
        },
        SysProc {
            cpu: 22.0,
            mem_mb: 1800.0,
            mem_pct: 11.0,
            on: true,
        },
        SysProc {
            cpu: 8.0,
            mem_mb: 180.0,
            mem_pct: 1.1,
            on: true,
        },
        SysProc {
            cpu: -1.0,
            mem_mb: 44.0,
            mem_pct: 0.3,
            on: true,
        },
    ]);
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
    cfg.show_ticker = false;
    cfg.show_sys = false;
    let mut s = Snapshot::default();
    s.on_track = 1;
    s.show_standings = 0;
    s.show_relative = 0;
    s.show_map = 0;
    draw_ok(&s, &cfg);
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
    assert!(vis_wide > 7);
    assert!(w_wide <= 140.0);
    let (vis_mid, w_mid) = hstand_layout(7.0 * 118.0 + 6.0 * 3.0, 1.0, 7, 20);
    assert_eq!(vis_mid, 7);
    assert!(w_mid > 86.0 && w_mid <= 140.0);
    let (vis_narrow, w_narrow) = hstand_layout(280.0, 1.0, 7, 20);
    assert!(vis_narrow < 7);
    assert!(w_narrow >= 78.0);
}
