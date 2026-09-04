use super::*;
use crate::config::{BoardField, DashField, FontFamily, HudConfig, LeanStyle, RelField, StField, StanceStyle, WidgetId};
use crate::race_store::{effective_extra_laps, effective_race_laps, ClockMode};
use crate::shm::{write_name, Point, Rider, Snapshot, Standing, MAGIC, TRACK_NAME, VERSION};
use std::sync::{Mutex, OnceLock};
use tiny_skia::{Color, Pixmap, Rect};

fn session_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn reset_session() {
    reset_session_clock_track();
    reset_flag_display();
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
    ST_SLIDE.with(|a| *a.borrow_mut() = TableSlides { rows: Vec::new() });
    REL_SLIDE.with(|a| *a.borrow_mut() = TableSlides { rows: Vec::new() });
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
        fuel: 5.6,
        max_fuel: 7.0,
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
    write_name(&mut s.setup_name, r"C:\Setups\Washougal Soft.xml");
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

/// Put the focus rider at `pos` round the lap and let the flag logic see the step.
fn ride_to(s: &mut Snapshot, pos: f32) -> DashFlag {
    s.local_track_pos = pos;
    s.riders[1].track_pos = pos;
    dash_race_flag(s)
}

/// Walk from just after the line round into the run-in window. The geometry guards need a
/// mid-lap sighting and a closing step before a run-in flag can arm.
fn ride_lap_to_line(s: &mut Snapshot) -> DashFlag {
    for pos in [0.10, 0.30, 0.50, 0.70, 0.85, 0.92] {
        ride_to(s, pos);
    }
    ride_to(s, 0.96)
}

/// Cross the line: laps tick over just past S/F, which is also where the overlay learns
/// where the line is.
fn cross_line(s: &mut Snapshot, laps: i32, lap_num: i32) -> DashFlag {
    s.standings[1].num_laps = laps;
    s.current_lap = lap_num;
    ride_to(s, 0.01)
}

/// Run the white-flag wave out without waiting on the clock.
fn age_white_wave() {
    WHITE_WAVE_AT.fetch_sub(WHITE_WAVE_MS + 1_000, Ordering::Relaxed);
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

fn golden_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/goldens")
}

fn update_goldens() -> bool {
    matches!(
        std::env::var("UPDATE_GOLDENS").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE")
    )
}

fn crop_px(src: &Pixmap, x: f32, y: f32, w: f32, h: f32, pad: f32) -> Pixmap {
    let x0 = (x - pad).floor().max(0.0) as i32;
    let y0 = (y - pad).floor().max(0.0) as i32;
    let x1 = ((x + w + pad).ceil() as i32).min(src.width() as i32);
    let y1 = ((y + h + pad).ceil() as i32).min(src.height() as i32);
    let cw = (x1 - x0).max(1) as u32;
    let ch = (y1 - y0).max(1) as u32;
    let mut out = Pixmap::new(cw, ch).expect("crop");
    out.fill(Color::TRANSPARENT);
    out.draw_pixmap(
        -x0,
        -y0,
        src.as_ref(),
        &tiny_skia::PixmapPaint::default(),
        tiny_skia::Transform::identity(),
        None,
    );
    out
}

fn assert_golden(name: &str, px: &Pixmap) {
    let dir = golden_dir();
    let path = dir.join(format!("{name}.png"));
    if update_goldens() {
        std::fs::create_dir_all(&dir).expect("goldens dir");
        std::fs::write(&path, px.encode_png().expect("png")).expect("write golden");
        return;
    }
    let bytes = std::fs::read(&path).unwrap_or_else(|_| {
        panic!("missing golden {name}.png — run with UPDATE_GOLDENS=1")
    });
    let expected = Pixmap::decode_png(&bytes).expect("decode golden");
    if expected.width() != px.width()
        || expected.height() != px.height()
        || expected.data() != px.data()
    {
        let actual_path = dir.join(format!("{name}.actual.png"));
        let _ = std::fs::write(&actual_path, px.encode_png().expect("png"));
        panic!(
            "golden mismatch {name} (wrote {})",
            actual_path.display()
        );
    }
}

fn hide_widgets(cfg: &mut HudConfig) {
    cfg[WidgetId::Standings].show = false;
    cfg[WidgetId::Relative].show = false;
    cfg[WidgetId::Map].show = false;
    cfg[WidgetId::Minimap].show = false;
    cfg[WidgetId::Radar].show = false;
    cfg[WidgetId::Dash].show = false;
    cfg[WidgetId::Ticker].show = false;
    cfg[WidgetId::Sys].show = false;
    cfg[WidgetId::Sector].show = false;
    cfg[WidgetId::Delta].show = false;
    cfg[WidgetId::Stance].show = false;
    cfg[WidgetId::Flag].show = false;
    cfg[WidgetId::Lean].show = false;
    cfg[WidgetId::Gamepad].show = false;
}

fn golden_snap(s: &Snapshot, cfg: &HudConfig) -> Snapshot {
    let mut s = *s;
    cfg.apply_to_snapshot(&mut s);
    s
}

fn draw_widget_golden(name: &str, s: &Snapshot, cfg: &HudConfig, rect: crate::shm::Rect) {
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    draw(
        &mut px,
        &fonts(),
        Some(s),
        cfg,
        1280,
        720,
        0.0,
        false,
        false,
        false,
    );
    let crop = crop_px(
        &px,
        rect.x * 1280.0,
        rect.y * 720.0,
        rect.w * 1280.0,
        rect.h * 720.0,
        8.0,
    );
    assert_golden(name, &crop);
}

fn rgba8(c: Color) -> (u8, u8, u8, u8) {
    (
        (c.red() * 255.0).round() as u8,
        (c.green() * 255.0).round() as u8,
        (c.blue() * 255.0).round() as u8,
        (c.alpha() * 255.0).round() as u8,
    )
}

fn sample_px(px: &Pixmap, x: f32, y: f32) -> [u8; 4] {
    let xi = (x.floor() as i32).clamp(0, px.width() as i32 - 1) as u32;
    let yi = (y.floor() as i32).clamp(0, px.height() as i32 - 1) as u32;
    let i = ((yi * px.width() + xi) * 4) as usize;
    let d = px.data();
    [d[i], d[i + 1], d[i + 2], d[i + 3]]
}

fn hit_nums_by_pos() -> Vec<i32> {
    let mut hits = click_rider_hits();
    hits.sort_by(|a, b| {
        a.y.partial_cmp(&b.y)
            .unwrap()
            .then(a.x.partial_cmp(&b.x).unwrap())
    });
    hits.iter().map(|h| h.race_num).collect()
}

#[test]
fn bundled_race_fonts_load() {
    for family in [
        FontFamily::Roboto,
        FontFamily::Exo2,
        FontFamily::Teko,
        FontFamily::Goldman,
        FontFamily::Montserrat,
    ] {
        Fonts::for_family(family).unwrap_or_else(|| panic!("load {}", family.label()));
    }
}

fn draw_ok(s: &Snapshot, cfg: &HudConfig) {
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut px, &fonts(), Some(s), cfg, 1280, 720, 0.0, false, false, false);
}

#[test]
fn hairline_fill_rect_does_not_panic() {
    let mut px = Pixmap::new(64, 64).expect("pixmap");
    let r = Rect::from_xywh(10.4, 10.4, 1.3, 1.3).expect("rect");
    fill_rect(&mut px, r, Color::from_rgba8(180, 180, 188, 40));
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
    assert_eq!(format_clock(93_500), "01:33.500");
    assert_eq!(format_board_gap(0, 0, true), "-");
    assert_eq!(format_board_gap(0, 1, false), "1L");
    assert_eq!(format_board_gap(1500, 0, false), "1.5");
    assert_eq!(format_penalty(0), "---");
    assert_eq!(format_delta_ms(0), "0.000");
    assert_eq!(format_delta_ms(250), "+0.250");
    assert_eq!(format_delta_ms(-347), "-0.347");
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
    assert_eq!(rgba8(bike_color("450 SX-F", "MX1")), (255, 96, 0, 255));
    assert_eq!(rgba8(bike_color("FC 450", "MX1")), (240, 240, 244, 255));
    assert_eq!(rgba8(bike_color("YZ450F", "MX1")), (0, 82, 196, 255));
    assert_eq!(rgba8(bike_color("CRF450R", "MX1")), (220, 28, 36, 255));
    assert_eq!(rgba8(bike_color("KX450", "MX1")), (80, 196, 32, 255));
    assert_eq!(rgba8(bike_color("RM-Z450", "MX1")), (236, 208, 24, 255));
    assert_eq!(rgba8(bike_color("MC 450", "MX1")), (196, 24, 40, 255));
}

#[test]
fn dash_footer_fields_fill_from_live_snapshot() {
    let _g = session_lock();
    reset_session();
    let s = live_snap();
    let cfg = HudConfig::new();
    assert_eq!(dash_foot_item(&s, &cfg, DashField::None), None);
    let speed = dash_foot_item(&s, &cfg, DashField::Speed).unwrap().1;
    assert_eq!(speed, "65 KPH");
    assert_eq!(dash_foot_item(&s, &cfg, DashField::Rpm).unwrap().1, "7200");
    assert_eq!(dash_foot_item(&s, &cfg, DashField::Gear).unwrap().1, "3");
    assert_eq!(dash_foot_item(&s, &cfg, DashField::Position).unwrap().1, "P2");
    assert_eq!(dash_foot_item(&s, &cfg, DashField::Number).unwrap().1, "#12");
    assert_eq!(dash_foot_item(&s, &cfg, DashField::Air).unwrap().1, "21°C");
    assert_eq!(dash_foot_item(&s, &cfg, DashField::Engine).unwrap().1, "82°C");
    assert_eq!(dash_foot_item(&s, &cfg, DashField::Fuel).unwrap().1, "5.6 L");
    let mut imp = cfg.clone();
    imp.units = crate::config::Units::Imperial;
    assert_eq!(dash_foot_item(&s, &imp, DashField::Fuel).unwrap().1, "1.5 gal");
    assert_eq!(dash_foot_item(&s, &cfg, DashField::FuelPct).unwrap().1, "80%");
    assert_eq!(dash_foot_item(&s, &cfg, DashField::Bike).unwrap().1, "YZ450");
    assert_eq!(dash_foot_item(&s, &cfg, DashField::Class).unwrap().1, "MX1");
    assert_eq!(dash_foot_item(&s, &cfg, DashField::Setup).unwrap().1, "Washougal Soft");
    let clock = dash_foot_item(&s, &cfg, DashField::LocalTime).unwrap().1;
    assert!(
        clock.contains("AM") || clock.contains("PM"),
        "local time footer should be 12h clock, got {clock}"
    );
    for field in DashField::ALL {
        if field == DashField::None {
            continue;
        }
        assert!(dash_foot_item(&s, &cfg, field).is_some(), "{field:?}");
    }
}

#[test]
fn default_dash_is_compact() {
    let _g = session_lock();
    reset_session();
    let s = live_snap();
    let cfg = HudConfig::new();
    let d = dash_layout(&fonts(), &s, &cfg, 1920.0, 1080.0, DashFlag::None, 0.0);
    assert!(!d.simple);
    assert!((d.w - cfg[WidgetId::Dash].rect.w * 1920.0).abs() < 1.0, "plaque should fill the widget width, got {}", d.w);
    assert!((d.h - cfg[WidgetId::Dash].rect.h * 1080.0).abs() < 1.0, "plaque should fill the widget height, got {}", d.h);
    assert!((cfg[WidgetId::Dash].rect.w - 0.111).abs() < 0.001);
    assert!((cfg[WidgetId::Dash].rect.h - 0.115).abs() < 0.001);
}

#[test]
fn dash_follows_widget_width() {
    let _g = session_lock();
    reset_session();
    let s = live_snap();
    let tight = dash_layout(&fonts(), &s, &HudConfig::new(), 1920.0, 1080.0, DashFlag::None, 0.0);
    let mut cfg = HudConfig::new();
    cfg[WidgetId::Dash].rect.w = 0.40;
    let d = dash_layout(&fonts(), &s, &cfg, 1920.0, 1080.0, DashFlag::None, 0.0);
    assert_eq!(tight.w, 0.111 * 1920.0);
    assert_eq!(d.w, 0.40 * 1920.0);
}

#[test]
fn dash_follows_widget_height() {
    let _g = session_lock();
    reset_session();
    let s = live_snap();
    let tight = dash_layout(&fonts(), &s, &HudConfig::new(), 1920.0, 1080.0, DashFlag::None, 0.0);
    let mut cfg = HudConfig::new();
    cfg[WidgetId::Dash].rect.h = 0.28;
    let d = dash_layout(&fonts(), &s, &cfg, 1920.0, 1080.0, DashFlag::None, 0.0);
    assert_eq!(tight.h, 0.115 * 1080.0);
    assert_eq!(d.h, 0.28 * 1080.0);
}

#[test]
fn simple_dash_is_gear_and_speed_only() {
    let _g = session_lock();
    reset_session();
    let s = live_snap();
    let mut cfg = HudConfig::new();
    cfg.dash_simple = true;
    cfg.dash_rev = true;
    cfg.units = crate::config::Units::Imperial;
    let d = dash_layout(&fonts(), &s, &cfg, 1280.0, 720.0, DashFlag::None, 0.0);
    assert!(d.simple);
    assert_eq!(d.gear, "3");
    assert_eq!(d.speed, "40");
    assert_eq!(d.speed_label, "MPH");
    assert!(d.foot.is_empty());
    assert_eq!(d.rev_h, 0.0);
    assert_eq!(d.footer_h, 0.0);
    assert!((d.w - cfg[WidgetId::Dash].rect.w * 1280.0).abs() < 1.0, "simple dash should fill the widget width, got {}", d.w);
}

#[test]
fn simple_dash_renders() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.show_standings = 0;
    s.show_relative = 0;
    s.show_map = 0;
    let mut cfg = HudConfig::new();
    cfg[WidgetId::Dash].show = true;
    cfg.dash_simple = true;
    cfg[WidgetId::Dash].bg = 82;
    cfg.units = crate::config::Units::Imperial;
    cfg[WidgetId::Dash].rect.x = 0.08;
    cfg[WidgetId::Dash].rect.y = 0.22;
    cfg[WidgetId::Dash].rect.h = 0.52;
    let (fw, fh) = (640u32, 280u32);
    let mut hud = Pixmap::new(fw, fh).expect("pixmap");
    draw(&mut hud, &fonts(), Some(&s), &cfg, fw, fh, 0.0, false, false, false);
    let lay = dash_layout(&fonts(), &s, &cfg, fw as f32, fh as f32, DashFlag::None, 0.0);
    let pad = 28.0;
    let cx = (lay.x - pad).max(0.0) as u32;
    let cy = (lay.y - pad).max(0.0) as u32;
    let cw = ((lay.w + pad * 2.0).min(fw as f32 - cx as f32)).max(1.0) as u32;
    let ch = ((lay.h + pad * 2.0).min(fh as f32 - cy as f32)).max(1.0) as u32;
    let mut px = Pixmap::new(cw, ch).expect("crop");
    px.fill(Color::from_rgba8(16, 16, 18, 255));
    px.draw_pixmap(
        -(cx as i32),
        -(cy as i32),
        hud.as_ref(),
        &tiny_skia::PixmapPaint::default(),
        tiny_skia::Transform::identity(),
        None,
    );
    assert_golden("dash-simple", &px);
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
    assert_eq!(rel(&s, 1), LapRel::Same);

    s.standings[0].num_laps = 4;
    s.riders[0].track_pos = 0.97;
    s.local_track_pos = 0.90;
    s.riders[1].track_pos = 0.90;
    assert_eq!(rel(&s, 1), LapRel::LappedByMe);

    s.standings[0].num_laps = 6;
    s.riders[0].track_pos = 0.20;
    s.local_track_pos = 0.90;
    s.riders[1].track_pos = 0.90;
    assert_eq!(rel(&s, 1), LapRel::Same, "lap up but not closing from behind");

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
    assert_eq!(rel(&s, 1), LapRel::LappedByMe);
    assert_eq!(rel(&s, 12), LapRel::Same);
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
    assert_eq!(rel(&s, 1), LapRel::Same, "two down, already gone by");

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
    assert_eq!(rel(&s, 1), LapRel::Same, "inverted laps after they pass is not red");
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
fn white_flag_waits_for_local_last_lap_not_leader_crossing() {
    let _g = session_lock();
    let mut s = live_snap();
    expire_timed(&mut s);
    s.local_speed = 18.0;
    assert_eq!(dash_race_flag(&s), DashFlag::None, "no flags until extras start");
    s.standings[0].num_laps = 6;
    assert_eq!(dash_race_flag(&s), DashFlag::None, "+2 still has 2 left — not white yet");
    s.standings[0].num_laps = 7;
    s.standings[1].num_laps = 5;
    s.current_lap = 6;
    assert_eq!(dash_race_flag(&s), DashFlag::None);
    // You start first extra: taken=1, done=0, left=2 → not last.
    s.standings[1].num_laps = 6;
    s.current_lap = 7;
    s.local_track_pos = 0.40;
    s.riders[1].track_pos = 0.40;
    assert_eq!(dash_race_flag(&s), DashFlag::None, "first of +2 is not white");
    // The run-in that starts your last extra is white.
    assert_eq!(ride_lap_to_line(&mut s), DashFlag::White);
    // Finish first / on last: done=1, left=1 → white.
    s.standings[1].num_laps = 7;
    s.current_lap = 8;
    assert_eq!(dash_race_flag(&s), DashFlag::White, "white on last remaining extra");
    // Complete both extras.
    s.standings[1].num_laps = 8;
    s.current_lap = 9;
    assert_eq!(dash_race_flag(&s), DashFlag::Checkered);
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

/// 10–30 min +1 looks like warmup/practice. Publishing extras after expiry must not
/// reset the session clock, or the dash sits on `0/1` with checkered while you
/// still have the uncounted lap and the extra to run.
fn long_timed_plus_one_late_extras(minutes: i32) {
    reset_session();
    let mut s = live_snap();
    s.session_kind = 7;
    s.session_length = minutes;
    s.session_laps = 0;
    s.session_time_ms = 30_000;
    s.current_lap = 1;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    let _ = session_remain_ms(&s);
    let len_ms = minutes * 60 * 1000;
    s.session_time_ms = len_ms;
    s.current_lap = 8;
    s.local_speed = 18.0;
    s.standings[0].num_laps = 8;
    s.standings[1].num_laps = 7;
    let remain = session_remain_ms(&s).expect("countdown while time remains");
    assert!(remain > 60_000, "{minutes}:00 expected a real countdown, got {remain}");
    s.session_time_ms = len_ms - 1_000;
    let _ = session_remain_ms(&s);
    s.session_time_ms = 400;
    assert_eq!(session_remain_ms(&s), Some(0));
    s.session_time_ms = len_ms;
    let _ = session_remain_ms(&s);
    assert_eq!(session_banner(&s).1, "00:00", "extras not published yet");
    assert_eq!(dash_race_flag(&s), DashFlag::None);

    s.current_lap = 9;
    s.standings[1].num_laps = 8;
    s.standings[0].num_laps = 7;
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
    s.standings[0].num_laps = 9;
    s.standings[1].num_laps = 8;
    s.current_lap = 9;
    assert_ne!(
        dash_race_flag(&s),
        DashFlag::Checkered,
        "must not latch checkered while the extra is still ahead"
    );
    assert_eq!(session_banner(&s).1, "0/1", "still the uncounted lap after recovery");

    s.standings[0].num_laps = 9;
    s.standings[1].num_laps = 8;
    s.current_lap = 9;
    assert_eq!(ride_to(&mut s, 0.40), DashFlag::None);
    s.standings[1].num_laps = 9;
    s.current_lap = 10;
    assert_eq!(dash_race_flag(&s), DashFlag::White, "last extra after the timed-lap base");
    age_white_wave();
    assert_eq!(ride_to(&mut s, 0.50), DashFlag::None);
    assert_eq!(cross_line(&mut s, 10, 11), DashFlag::Checkered);
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

/// A lap down when the leader takes the finish: the race is over, so you are waved off
/// at the line even though you never completed the distance.
#[test]
fn lapped_rider_gets_checkered_when_leader_finishes() {
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

    // Leader crosses to finish while you are still out on track. The lap you are on is
    // now your last, so the total drops to the four laps you will actually run.
    s.standings[0].num_laps = 5;
    assert!(leader_finished(&s));
    assert!(!race_over_for_me(&s), "not until you reach the line");
    assert_eq!(effective_race_laps(&s), 4);
    assert_eq!(session_banner(&s).1, "4 / 4");
    // The lap you are on has just become your last, so the white is waved there and then.
    assert_eq!(ride_to(&mut s, 0.60), DashFlag::White);
    age_white_wave();
    assert_eq!(
        ride_lap_to_line(&mut s),
        DashFlag::Checkered,
        "your next crossing ends it, so the run-in is checkered"
    );

    // Your crossing ends it, one lap short of the moto but the full shortened race.
    assert_eq!(cross_line(&mut s, 4, 5), DashFlag::Checkered);
    assert!(race_over_for_me(&s));
    assert!(i_finished(&s));
    assert_eq!(
        session_banner(&s).1,
        "4 / 4",
        "the total follows the race you ran, not the moto distance"
    );
    assert_eq!(race_laps_left_text(&s), "0");
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

/// Same rule on a timed race: the leader completing their extras ends it for everyone.
#[test]
fn lapped_rider_gets_checkered_when_leader_finishes_extras() {
    let _g = session_lock();
    let mut s = live_snap();
    expire_timed_extras(&mut s, 1);
    s.local_speed = 18.0;
    // Leader starts their extra a lap up on you; you are still on the timed lap.
    s.standings[0].num_laps = 6;
    s.standings[1].num_laps = 5;
    s.current_lap = 6;
    assert!(extras_started(&s));
    assert!(!leader_finished(&s));

    // Leader completes the extra and takes the finish.
    s.standings[0].num_laps = 7;
    assert!(leader_finished(&s));
    assert!(!race_over_for_me(&s));
    assert_eq!(ride_to(&mut s, 0.40), DashFlag::White);
    assert_eq!(cross_line(&mut s, 6, 7), DashFlag::Checkered);
    assert!(i_finished(&s));
}

/// The game only rescores you a frame or two after you are past the line, so the lap
/// rules still describe the lap you just finished. The flag that was out on the run-in has
/// to stay out across that gap instead of blinking off and back on.
#[test]
fn the_run_in_flag_stays_out_across_the_line() {
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
    ride_to(&mut s, 0.50);
    assert_eq!(ride_lap_to_line(&mut s), DashFlag::White, "run-in onto the last lap");
    // Past the line, but the classification still has two laps to run.
    assert_eq!(ride_to(&mut s, 0.01), DashFlag::White, "white holds across the line");
    assert_eq!(cross_line(&mut s, 3, 4), DashFlag::White, "and the wave takes over");

    // Same across the finish.
    age_white_wave();
    assert_eq!(ride_lap_to_line(&mut s), DashFlag::Checkered);
    assert_eq!(
        ride_to(&mut s, 0.998),
        DashFlag::Checkered,
        "the last metres before the line are not a hole"
    );
    assert_eq!(
        ride_to(&mut s, 0.01),
        DashFlag::Checkered,
        "checkered holds across the line"
    );
    assert_eq!(
        CHECKERED_LATCH.load(Ordering::Relaxed),
        0,
        "held, not latched — the lap count has not confirmed the finish"
    );
    assert_eq!(cross_line(&mut s, 4, 5), DashFlag::Checkered);
    assert_eq!(CHECKERED_LATCH.load(Ordering::Relaxed), 1);
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

/// With no centerline and no learned line, the window position is a guess, so the early
/// white is suppressed. Lap counting still delivers white and checkered.
#[test]
fn no_track_geometry_still_flags_from_lap_count() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_length = 0;
    s.session_laps = 4;
    s.session_time_ms = 0;
    s.local_speed = 18.0;
    s.poly_count = 0;
    s.sf_meters = 0.0;
    s.standings[0].num_laps = 2;
    s.standings[1].num_laps = 2;
    s.current_lap = 3;
    ride_to(&mut s, 0.50);
    assert_eq!(
        ride_lap_to_line(&mut s),
        DashFlag::None,
        "no early white without a known line"
    );
    // The last lap and the finish need no geometry at all.
    s.standings[1].num_laps = 3;
    s.current_lap = 4;
    assert_eq!(dash_race_flag(&s), DashFlag::White);
    s.standings[1].num_laps = 4;
    s.current_lap = 5;
    assert_eq!(dash_race_flag(&s), DashFlag::Checkered);
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

/// Lapped during the extras of a timed race: the total extras drop the same way a lap
/// moto's total does, so `1/2` becomes the single extra you will actually run.
#[test]
fn lapped_during_extras_shortens_the_extra_count() {
    let _g = session_lock();
    let mut s = live_snap();
    expire_timed_extras(&mut s, 2);
    s.local_speed = 18.0;
    // Leader starts the extras while you are mid-lap, so you have three laps to run.
    s.standings[0].num_laps = 6;
    assert_eq!(session_banner(&s).1, "0/2");
    assert_eq!(effective_extra_laps(&s), 2);
    assert_eq!(ride_to(&mut s, 0.40), DashFlag::None);

    // The leader completes both extras before you have taken either. You are waved off
    // at your next crossing, so only one extra is yours.
    s.standings[0].num_laps = 8;
    assert!(leader_finished(&s));
    assert_eq!(effective_extra_laps(&s), 1);
    assert_eq!(session_banner(&s).1, "0/1", "a single remaining extra still uses n/n");
    assert_eq!(dash_race_flag(&s), DashFlag::White);
    assert_eq!(cross_line(&mut s, 6, 7), DashFlag::Checkered);
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
    let learned = SF_FRAC_LEARNED.load(Ordering::Relaxed);
    assert!(learned > 7_000 && learned < 8_000, "learned {learned}");
    // One stray crossing somewhere else must not move it.
    s.standings[1].num_laps = 4;
    s.current_lap = 5;
    ride_to(&mut s, 0.20);
    assert_eq!(SF_FRAC_LEARNED.load(Ordering::Relaxed), learned);
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
    assert_eq!(session_banner(&s).1, "0/1");
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
    assert_eq!((you_x, you_y), (50.0, 40.0));
    assert_eq!((lx, ly), (40.0, 56.0));
    let (rx, ry) = radar_to_screen(0.0, 2.5, ox, oy, sx, sy);
    assert_eq!((rx, ry), (62.5, 40.0));
}

#[test]
fn radar_blip_heat_rises_when_closer() {
    assert_eq!(radar_blip_heat(1.0), 1.0);
    assert!((radar_blip_heat(7.0) - 1.0 / 7.0).abs() < 1e-6);
    assert_eq!(radar_blip_heat(8.0), 0.0);
}

#[test]
fn radar_blip_color_matches_the_arcs_mock() {
    let close = radar_blip_color(1.0);
    let far = radar_blip_color(0.0);
    assert!((close.red() - 250.0 / 255.0).abs() < 0.02);
    assert!(close.green() < far.green());
    assert_eq!(close.alpha(), 1.0);
    assert_eq!(far.alpha(), 1.0);
}

#[test]
fn radar_blip_radius_grows_with_heat_and_widget_size() {
    let small_far = radar_blip_radius(0.0, 160.0);
    let small_close = radar_blip_radius(1.0, 160.0);
    let large_far = radar_blip_radius(0.0, 400.0);
    let large_close = radar_blip_radius(1.0, 400.0);
    assert!((small_far - 7.0).abs() < 1e-4);
    assert!(small_close >= small_far);
    assert!(large_close > large_far);
    assert!(large_close > 12.0);
    assert!(large_close <= 15.0);
}

#[test]
fn radar_rings_lift_off_a_solid_plaque() {
    let glass = radar_ring_color(0);
    let solid = radar_ring_color(100);
    assert!(solid.red() > glass.red() + 0.2);
    assert!(solid.alpha() >= glass.alpha());
    assert!(radar_ring_stroke(200.0, 100) > radar_ring_stroke(200.0, 0));
}

#[test]
fn radar_fit_scale_keeps_the_12m_ring_inside_the_plaque() {
    let s = radar_fit_scale(160.0, 160.0, 80.0, 40.0, 0.0, 0.0, 8.0);
    assert!((s - 6.0).abs() < 1e-4);
    assert!((radar_ring_radius(12.0, s) - 72.0).abs() < 1e-4);
    assert!((radar_ring_radius(6.0, s) - 36.0).abs() < 1e-4);
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
    assert_eq!(mini_view_radius(100), 22.0);
    assert_eq!(mini_view_radius(0), 85.0);
    assert!((mini_view_radius(70) - 40.9).abs() < 0.001);
}

#[test]
fn poly_at_frac_walks_centerline_distance() {
    let mut s = Snapshot::default();
    s.poly_count = 3;
    s.track_length = 200.0;
    s.poly[0] = Point { x: 0.0, z: 0.0 };
    s.poly[1] = Point { x: 100.0, z: 0.0 };
    s.poly[2] = Point { x: 100.0, z: 100.0 };
    let a = poly_at_frac(&s, 3, 0.25).expect("s1");
    assert!((a.wx - 50.0).abs() < 0.5, "wx {}", a.wx);
    assert!(a.wz.abs() < 0.5, "wz {}", a.wz);
    let b = poly_at_frac(&s, 3, 0.75).expect("s2");
    assert!((b.wx - 100.0).abs() < 0.5, "wx {}", b.wx);
    assert!((b.wz - 50.0).abs() < 0.5, "wz {}", b.wz);
    s.sf_meters = 50.0;
    let line = poly_at_frac(&s, 3, 0.0).expect("s1 starts at s/f");
    assert!((line.wx - 50.0).abs() < 0.5, "sf wx {}", line.wx);
    let s2 = poly_at_frac(&s, 3, 0.25).expect("s2 starts at first split");
    assert!((s2.wx - 100.0).abs() < 0.5, "s2 wx {}", s2.wx);
    assert!(s2.wz.abs() < 0.5, "s2 wz {}", s2.wz);
}

#[test]
fn sector_row_formats_plugin_deltas() {
    let mut sector = live_snap();
    sector.show_standings = 0;
    sector.show_relative = 0;
    sector.show_map = 0;
    sector.sector_count = 3;
    sector.current_lap_ms = 70_000;
    sector.sector_last = 1;
    sector.sector_cur = [24_093, 25_760, 0];
    sector.sector_last_lap = [24_310, 25_820, 23_090];
    sector.sector_best = [24_180, 25_640, 22_910];
    sector.sector_delta = [-87, 120, 0];
    sector.sector_delta_valid = 0b011;
    let mid_s1 = super::sector_row(&sector, 0, true, false);
    let mid_s3 = super::sector_row(&sector, 2, true, false);
    assert!(!mid_s1.pending);
    assert!(!mid_s1.fresh);
    assert_eq!(mid_s1.delta, "-0.087");
    assert!(mid_s3.fresh);
    assert!(!mid_s3.pending);
    let mut pb = sector;
    pb.sector_best = [24_093, 25_640, 22_910];
    pb.sector_delta = [-87, 120, 0];
    pb.sector_delta_valid = 0b011;
    let pb_s1 = super::sector_row(&pb, 0, true, false);
    assert_eq!(pb_s1.delta, "-0.087");
    assert_ne!(pb_s1.delta, "0.000");
    let mut held = sector;
    held.current_lap_ms = 0;
    held.sector_cur = [0, 0, 0];
    held.sector_last = 2;
    held.sector_last_lap = [24_093, 25_760, 23_090];
    held.sector_delta = [-87, 120, -40];
    held.sector_delta_valid = 0b111;
    let held_s3 = super::sector_row(&held, 2, true, false);
    assert!(!held_s3.pending);
    assert_eq!(held_s3.time, "23.090");
    assert_eq!(held_s3.delta, "-0.040");
    assert!(held_s3.fresh);
    let mut inferred = held;
    inferred.sector_last = 1;
    inferred.sector_last_lap = [24_093, 25_760, 0];
    inferred.sector_delta_valid = 0b011;
    inferred.last_lap_ms = 24_093 + 25_760 + 23_090;
    let inf_s3 = super::sector_row(&inferred, 2, true, false);
    assert!(!inf_s3.pending);
    assert_eq!(inf_s3.time, "23.090");
    assert!(inf_s3.fresh);
    let mut next_s1 = held;
    next_s1.current_lap_ms = 40_000;
    next_s1.sector_cur = [24_200, 0, 0];
    next_s1.sector_last = 0;
    next_s1.sector_last_lap = [24_093, 0, 0];
    next_s1.sector_delta = [20, 0, 0];
    next_s1.sector_delta_valid = 0b001;
    let frozen_s1 = super::sector_row(&next_s1, 0, true, false);
    let next_s2 = super::sector_row(&next_s1, 1, true, false);
    let next_s3 = super::sector_row(&next_s1, 2, true, false);
    assert_eq!(frozen_s1.delta, "+0.020");
    assert!(!frozen_s1.fresh);
    assert!(next_s2.fresh);
    assert!(next_s3.pending);
}

#[test]
fn hidden_widgets_do_not_need_live_telemetry() {
    let _g = session_lock();
    reset_session();
    let mut cfg = HudConfig::new();
    cfg[WidgetId::Minimap].show = false;
    cfg[WidgetId::Radar].show = false;
    cfg[WidgetId::Dash].show = false;
    cfg[WidgetId::Ticker].show = false;
    cfg[WidgetId::Sys].show = false;
    let mut s = Snapshot::default();
    s.on_track = 1;
    s.show_standings = 0;
    s.show_relative = 0;
    s.show_map = 0;
    draw_ok(&s, &cfg);
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

/// Both on the same lap, focus (#12) scored second but `along` metres up the track on the
/// leader. Track length is 1000 m in the fixture, so a metre is 0.001 of a lap.
fn pass_the_leader(s: &mut Snapshot, along: f32) {
    s.riders[0].track_pos = 0.500;
    s.riders[1].track_pos = 0.500 + along / 1000.0;
    s.local_track_pos = s.riders[1].track_pos;
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

/// `track_pos` is measured from the centerline origin, not the line, so only riders close
/// together can be compared.
#[test]
fn half_a_lap_apart_is_not_a_pass() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    pass_the_leader(&mut s, 400.0);
    let field = RaceStore::tick(&s).field;
    assert_eq!(field.rows[0].standing.race_num, 1);
}

/// Map / minimap dot labels, the crown and the nearest ahead / behind rings all read
/// `standing_pos` and `leader_num`, so they move with a pass too.
#[test]
fn map_marks_follow_the_live_order() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.standing_count = 3;
    s.rider_count = 3;
    s.standings[2] = standing(7, 3, 5);
    s.riders[2] = rider(7, 30.0, 12.0, 0.0);
    // #7 was behind you; now they are 6 m up the track on you. The leader is clear.
    s.riders[0].track_pos = 0.600;
    s.riders[1].track_pos = 0.500;
    s.local_track_pos = 0.500;
    s.riders[2].track_pos = 0.506;
    let _ = RaceStore::tick(&s);
    assert_eq!(leader_num(&s), 1);
    assert_eq!(standing_pos(&s, 7), 2);
    assert_eq!(standing_pos(&s, 12), 3);
    // The ring on #7 is now the green "ahead" mark, not the red one behind you.
    assert_eq!(standing_pos(&s, 7), standing_pos(&s, 12) - 1);
}

/// Walk a session and record what the dash would show each frame. `extra` replays what the
/// live-order code used to do: ask the session heuristics again after the clock had already
/// decided the frame.
fn clock_trace(laps: i32, length: i32, extra: bool) -> Vec<(ClockMode, String, RaceFlag)> {
    reset_session();
    let mut s = live_snap();
    s.session_length = length;
    s.session_laps = laps;
    s.session_time_ms = 50_000;
    s.current_lap = 1;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    let mut out = Vec::new();
    let step = |s: &Snapshot, out: &mut Vec<_>| {
        if extra {
            // What `build_field` used to call, right after `build_clock` ran.
            let _ = is_warmup(s);
            let _ = leader_finished(s);
            let _ = effective_race_laps(s);
        }
        let store = RaceStore::tick(s);
        out.push((store.clock.mode, store.clock.banner.1.clone(), store.clock.flag));
    };
    step(&s, &mut out);
    // Green, then a lap at a time to the finish.
    s.local_speed = 18.0;
    for lap in 1..=laps.max(1) + 1 {
        s.session_time_ms = 60_000 * lap;
        s.current_lap = lap + 1;
        s.standings[0].num_laps = lap;
        s.standings[1].num_laps = lap;
        step(&s, &mut out);
        s.local_track_pos = 0.5;
        step(&s, &mut out);
    }
    out
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
    cfg.st_stripe = false;
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
fn stripe_row_bg_lifts_when_panel_is_opaque() {
    assert_eq!(rgba8(stripe_row_bg(bg_a(78))), (0, 0, 0, 54));
    assert_eq!(rgba8(stripe_row_bg(bg_a(100))), (255, 255, 255, 34));
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
    cfg.st_stripe = false;
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
fn blank_hud_paints_enable_hint_when_no_widget_is_on() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.on_track = 0;
    s.has_telemetry = 0;
    s.standing_count = 0;
    s.rider_count = 0;
    let cfg = HudConfig::new();
    assert!(!cfg.any_overlay_widget());
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut px, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
    let crop = crop_px(&px, 1280.0 * 0.5 - 340.0, 8.0, 680.0, 40.0, 4.0);
    assert_golden("blank-hint", &crop);
}

#[test]
fn blank_hud_explains_on_track_when_widgets_are_on() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.on_track = 0;
    s.has_telemetry = 0;
    s.standing_count = 0;
    s.rider_count = 0;
    let mut cfg = HudConfig::new();
    cfg[WidgetId::Dash].show = true;
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut px, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
    let crop = crop_px(&px, 1280.0 * 0.5 - 340.0, 8.0, 680.0, 40.0, 4.0);
    let opaque = crop.pixels().iter().filter(|p| p.alpha() > 20).count();
    assert!(opaque > 50, "widgets-on garage / no plugin data must still show a plaque");
}

#[test]
fn live_mark_paints_in_the_top_right() {
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    px.fill(Color::TRANSPARENT);
    let (x, y, bw, bh) = draw_live_mark(&mut px, 1280, 720, false);
    assert_eq!((x, y, bw, bh), (1230.0, 10.0, 40.0, 40.0));
    let crop = crop_px(&px, x, y, bw, bh, 6.0);
    assert_golden("live-mark", &crop);
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

#[test]
fn empty_session_hides_race_widgets() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.on_track = 0;
    s.has_telemetry = 0;
    s.standing_count = 0;
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
    assert!(click_rider_hits().is_empty());
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut px, &fonts(), None, &cfg, 1280, 720, 0.0, false, false, false);
}

#[test]
fn no_snapshot_does_not_ask_to_restart() {
    let cfg = HudConfig::new();
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut px, &fonts(), None, &cfg, 1280, 720, 0.0, false, false, false);
    let opaque = px.pixels().iter().filter(|p| p.alpha() > 20).count();
    assert_eq!(opaque, 0, "menus / boot with a copied plugin must not show a restart plaque");
}

#[test]
fn plugin_hint_wins_over_fullscreen_restart() {
    let cfg = HudConfig::new();
    let mut both = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut both, &fonts(), None, &cfg, 1280, 720, 0.0, true, true, false);
    let mut plugin = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut plugin, &fonts(), None, &cfg, 1280, 720, 0.0, false, true, false);
    let mut fso = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut fso, &fonts(), None, &cfg, 1280, 720, 0.0, true, false, false);
    assert_eq!(
        both.data(),
        plugin.data(),
        "plugin restart plaque wins when both hints are on"
    );
    assert_ne!(both.data(), fso.data());
}

#[test]
fn map_subject_pose_follows_spectated_rider_without_telemetry() {
    let mut s = live_snap();
    s.has_telemetry = 0;
    s.focus_race_num = 1;
    let pose = subject_pose(&s, 0.0).expect("spectate pose");
    assert!(!pose.from_local);
    assert_eq!(pose.x, 20.0);
    assert_eq!(pose.z, 8.0);
    assert_eq!(camera_subject(&s), 1);

    s.has_telemetry = 1;
    s.focus_race_num = 12;
    let pose = subject_pose(&s, 0.0).expect("back on the bike");
    assert!(pose.from_local);
    assert_eq!(pose.x, 10.0);
    assert_eq!(pose.z, 4.0);
}

#[test]
fn map_subject_pose_uses_local_telemetry_while_riding() {
    let s = live_snap();
    let pose = subject_pose(&s, 0.0).expect("ride pose");
    assert!(pose.from_local);
    assert_eq!(pose.x, 10.0);
    assert_eq!(pose.z, 4.0);
}

#[test]
fn map_subject_pose_returns_to_me_when_riding_even_if_focus_is_stale() {
    let mut s = live_snap();
    s.has_telemetry = 1;
    s.focus_race_num = 1;
    let pose = subject_pose(&s, 0.0).expect("ride pose");
    assert!(pose.from_local);
    assert_eq!(pose.x, 10.0);
    assert_eq!(pose.z, 4.0);
}

fn sys_procs_fixture() -> Vec<SysProc> {
    vec![
        SysProc {
            label: "HUD".into(),
            cpu: 4.0,
            gpu: 3.0,
            mem_mb: 88.0,
            mem_pct: 0.5,
            on: true,
        },
        SysProc {
            label: "MX Bikes".into(),
            cpu: 22.0,
            gpu: 38.0,
            mem_mb: 1800.0,
            mem_pct: 11.0,
            on: true,
        },
        SysProc {
            label: "MXB App".into(),
            cpu: 8.0,
            gpu: 1.0,
            mem_mb: 180.0,
            mem_pct: 1.1,
            on: true,
        },
        SysProc {
            label: "ReShade".into(),
            cpu: -1.0,
            gpu: -1.0,
            mem_mb: 44.0,
            mem_pct: 0.3,
            on: true,
        },
    ]
}

fn sector_snap(base: Snapshot) -> Snapshot {
    let mut s = base;
    s.sector_count = 3;
    s.current_lap_ms = 70_000;
    s.sector_last = 1;
    s.sector_cur = [24_093, 25_760, 0];
    s.sector_last_lap = [24_310, 25_820, 23_090];
    s.sector_best = [24_180, 25_640, 22_910];
    s.sector_delta = [-87, 120, 0];
    s.sector_delta_valid = 0b011;
    s
}

#[test]
fn widget_goldens_pin_paint() {
    let _g = session_lock();
    let _pb = crate::track_pb::exclusive_test();
    reset_session();
    crate::track_pb::reset_store();
    crate::sector::reset_engine();
    crate::delta::set_preview(None);
    let base = live_snap();
    let mut rel = base;
    rel.standings[0].num_laps = 6;

    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Standings].show = true;
    let s = golden_snap(&base, &cfg);
    draw_widget_golden("standings", &s, &cfg, cfg[WidgetId::Standings].rect);

    hide_widgets(&mut cfg);
    cfg[WidgetId::Relative].show = true;
    let s = golden_snap(&rel, &cfg);
    draw_widget_golden("relative", &s, &cfg, cfg[WidgetId::Relative].rect);

    hide_widgets(&mut cfg);
    cfg[WidgetId::Map].show = true;
    cfg.map_sectors = false;
    let s = golden_snap(&base, &cfg);
    draw_widget_golden("map", &s, &cfg, cfg[WidgetId::Map].rect);
    cfg.map_sectors = true;

    hide_widgets(&mut cfg);
    cfg[WidgetId::Minimap].show = true;
    cfg.mini_sectors = false;
    let s = golden_snap(&base, &cfg);
    draw_widget_golden("minimap", &s, &cfg, cfg[WidgetId::Minimap].rect);
    cfg.mini_sectors = true;

    hide_widgets(&mut cfg);
    cfg[WidgetId::Radar].show = true;
    let s = golden_snap(&base, &cfg);
    draw_widget_golden("radar", &s, &cfg, cfg[WidgetId::Radar].rect);

    hide_widgets(&mut cfg);
    cfg[WidgetId::Dash].show = true;
    let s = golden_snap(&base, &cfg);
    draw_widget_golden("dash", &s, &cfg, cfg[WidgetId::Dash].rect);

    hide_widgets(&mut cfg);
    cfg[WidgetId::Ticker].show = true;
    let s = golden_snap(&base, &cfg);
    draw_widget_golden("ticker", &s, &cfg, cfg[WidgetId::Ticker].rect);

    hide_widgets(&mut cfg);
    cfg[WidgetId::Sys].show = true;
    set_sys_stats(48.0, 62.0, 91.0, 41.0, 24);
    set_sys_procs(sys_procs_fixture());
    let s = golden_snap(&base, &cfg);
    draw_widget_golden("sys", &s, &cfg, cfg[WidgetId::Sys].rect);

    hide_widgets(&mut cfg);
    cfg[WidgetId::Sector].show = true;
    crate::sector::reset_engine();
    crate::sector::set_history([
        [24_180, 25_640, 20_147],
        [24_410, 25_890, 20_400],
        [24_250, 25_710, 20_220],
        [24_500, 26_010, 20_550],
        [24_330, 25_800, 20_310],
    ]);
    let s = golden_snap(&sector_snap(base), &cfg);
    draw_widget_golden("sector", &s, &cfg, cfg[WidgetId::Sector].rect);

    hide_widgets(&mut cfg);
    cfg[WidgetId::Delta].show = true;
    crate::delta::set_preview(Some(crate::delta::DeltaView {
        ready: true,
        recording: false,
        has_delta: true,
        delta_ms: -347,
        ref_lap_ms: 72_140,
        cover: 100,
        last_lap_ms: 72_480,
        new_best: false,
    }));
    let s = golden_snap(&base, &cfg);
    draw_widget_golden("delta-ahead", &s, &cfg, cfg[WidgetId::Delta].rect);
    crate::delta::set_preview(Some(crate::delta::DeltaView {
        ready: true,
        recording: false,
        has_delta: true,
        delta_ms: 812,
        ref_lap_ms: 72_140,
        cover: 100,
        last_lap_ms: 72_480,
        new_best: false,
    }));
    draw_widget_golden("delta-behind", &s, &cfg, cfg[WidgetId::Delta].rect);
    crate::delta::set_preview(Some(crate::delta::DeltaView {
        ready: false,
        recording: true,
        has_delta: false,
        delta_ms: 0,
        ref_lap_ms: 0,
        cover: 40,
        last_lap_ms: 0,
        new_best: false,
    }));
    draw_widget_golden("delta-rec", &s, &cfg, cfg[WidgetId::Delta].rect);
    crate::delta::set_preview(None);

    hide_widgets(&mut cfg);
    cfg.experimental = false;
    cfg[WidgetId::Stance].show = true;
    cfg.stance_style = StanceStyle::Icon;
    cfg.stance_show_sit = true;
    set_stance(true);
    let s = golden_snap(&base, &cfg);
    draw_widget_golden("stance-stand", &s, &cfg, cfg[WidgetId::Stance].rect);
    set_stance(false);
    draw_widget_golden("stance-sit", &s, &cfg, cfg[WidgetId::Stance].rect);

    hide_widgets(&mut cfg);
    cfg[WidgetId::Lean].show = true;
    cfg[WidgetId::Lean].bg = 0;
    let mut lean = base;
    lean.local_roll = -32.0;
    lean.local_pitch = -18.0;
    lean.local_steer = -0.12;
    lean.steer_lock = 0.40;
    let s = golden_snap(&lean, &cfg);
    draw_widget_golden("lean", &s, &cfg, cfg[WidgetId::Lean].rect);
    lean.has_telemetry = 0;
    lean.focus_race_num = 1;
    lean.riders[0].lean = -18.0;
    let s = golden_snap(&lean, &cfg);
    draw_widget_golden("lean-spectate", &s, &cfg, cfg[WidgetId::Lean].rect);
}

#[test]
fn lean_min_golden() {
    let _g = session_lock();
    reset_session();
    let base = live_snap();
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Lean].show = true;
    cfg[WidgetId::Lean].bg = 0;
    cfg.lean_style = LeanStyle::Minimal;
    let mut lean = base;
    lean.local_roll = -32.0;
    lean.local_pitch = -18.0;
    lean.local_steer = -0.12;
    lean.steer_lock = 0.40;
    let s = golden_snap(&lean, &cfg);
    draw_widget_golden("lean-min", &s, &cfg, cfg[WidgetId::Lean].rect);
}

#[test]
fn gamepad_goldens() {
    let _g = session_lock();
    reset_session();
    let base = live_snap();
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Gamepad].show = true;
    crate::gamepad::set(crate::gamepad::demo_sony());
    let s = golden_snap(&base, &cfg);
    draw_widget_golden("gamepad", &s, &cfg, cfg[WidgetId::Gamepad].rect);
    crate::gamepad::set(crate::gamepad::demo_xbox());
    draw_widget_golden("gamepad-xbox", &s, &cfg, cfg[WidgetId::Gamepad].rect);
    crate::gamepad::set(crate::gamepad::PadState::DISCONNECTED);
    draw_widget_golden("gamepad-none", &s, &cfg, cfg[WidgetId::Gamepad].rect);
}

#[test]
fn flag_widget_draws_nothing_when_no_flag() {
    let _g = session_lock();
    reset_session();
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            reset_flag_display();
        }
    }
    let _pg = Guard;
    set_flag_preview(0);
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Flag].show = true;
    let s = golden_snap(&live_snap(), &cfg);
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut px, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
    let cx = (cfg[WidgetId::Flag].rect.x + cfg[WidgetId::Flag].rect.w * 0.5) * 1280.0;
    let cy = (cfg[WidgetId::Flag].rect.y + cfg[WidgetId::Flag].rect.h * 0.5) * 720.0;
    assert_eq!(sample_px(&px, cx, cy)[3], 0, "no flag should leave the slot empty");
}

#[test]
fn flag_widget_paints_checkered() {
    let _g = session_lock();
    reset_session();
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            reset_flag_display();
        }
    }
    let _pg = Guard;
    set_flag_preview(2);
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Flag].show = true;
    let s = golden_snap(&live_snap(), &cfg);
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut px, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
    let x0 = cfg[WidgetId::Flag].rect.x * 1280.0;
    let y0 = cfg[WidgetId::Flag].rect.y * 720.0;
    let x1 = x0 + cfg[WidgetId::Flag].rect.w * 1280.0;
    let y1 = y0 + cfg[WidgetId::Flag].rect.h * 720.0;
    let mut lit = 0u32;
    let mut x = x0;
    while x < x1 {
        let mut y = y0;
        while y < y1 {
            if sample_px(&px, x, y)[3] > 40 {
                lit += 1;
            }
            y += 8.0;
        }
        x += 8.0;
    }
    assert!(lit > 20, "checkered flag should fill the widget, lit={lit}");
}

#[test]
fn flag_widget_paints_white() {
    let _g = session_lock();
    reset_session();
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            reset_flag_display();
        }
    }
    let _pg = Guard;
    set_flag_preview(1);
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Flag].show = true;
    let s = golden_snap(&live_snap(), &cfg);
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut px, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
    let x0 = cfg[WidgetId::Flag].rect.x * 1280.0;
    let y0 = cfg[WidgetId::Flag].rect.y * 720.0;
    let x1 = x0 + cfg[WidgetId::Flag].rect.w * 1280.0;
    let y1 = y0 + cfg[WidgetId::Flag].rect.h * 720.0;
    assert!(
        rect_has(&px, x0, y0, x1, y1, |p| p[3] > 40 && p[0] > 180 && p[1] > 180 && p[2] > 180),
        "white flag should paint a light cloth"
    );
}

#[test]
fn flag_default_matches_saved_size() {
    let cfg = HudConfig::new();
    assert!((cfg[WidgetId::Flag].rect.w - 0.107).abs() < 0.001);
    assert!((cfg[WidgetId::Flag].rect.h - 0.019).abs() < 0.001);
}

fn mid_race_snap() -> Snapshot {
    let mut s = live_snap();
    s.session_laps = 10;
    s.session_length = 0;
    s.current_lap = 3;
    s.standings[0].num_laps = 2;
    s.standings[1].num_laps = 2;
    s.local_track_pos = 0.40;
    s.riders[1].track_pos = 0.40;
    s.riders[0].track_pos = 0.33;
    s.local_speed = 18.0;
    s
}

fn crash_ahead(s: &mut Snapshot) {
    s.riders[0].crashed = 1;
    s.riders[0].track_pos = 0.43;
    s.local_track_pos = 0.40;
    s.riders[1].track_pos = 0.40;
}

fn crash_behind(s: &mut Snapshot) {
    s.riders[0].crashed = 1;
    s.riders[0].track_pos = 0.33;
    s.local_track_pos = 0.40;
    s.riders[1].track_pos = 0.40;
}

fn lapping_from_behind(s: &mut Snapshot) {
    s.standings[0].num_laps = 3;
    s.riders[0].track_pos = 0.365;
    s.local_track_pos = 0.40;
    s.riders[1].track_pos = 0.40;
}

fn rect_has(px: &Pixmap, x0: f32, y0: f32, x1: f32, y1: f32, pred: impl Fn([u8; 4]) -> bool) -> bool {
    let mut x = x0;
    while x < x1 {
        let mut y = y0;
        while y < y1 {
            if pred(sample_px(px, x, y)) {
                return true;
            }
            y += 6.0;
        }
        x += 6.0;
    }
    false
}

fn is_yellow(p: [u8; 4]) -> bool {
    p[3] > 40 && p[0] > 200 && p[1] > 180 && p[2] < 80
}

fn is_blue(p: [u8; 4]) -> bool {
    p[3] > 40 && p[2] > 170 && p[0] < 130 && p[1] > 80
}

#[test]
fn caution_off_ignores_nearby_crash() {
    let _g = session_lock();
    reset_session();
    let mut s = mid_race_snap();
    crash_ahead(&mut s);
    assert_eq!(dash_race_flag(&s), DashFlag::None);
    assert_eq!(wanted_flag(&s, false, false), DashFlag::None);
    assert_eq!(caution_flag(&s, true, true), DashFlag::Yellow);
}

#[test]
fn caution_yellow_on_nearby_crash() {
    let _g = session_lock();
    reset_session();
    let mut s = mid_race_snap();
    crash_ahead(&mut s);
    assert_eq!(wanted_flag(&s, true, false), DashFlag::Yellow);
    assert_eq!(wanted_flag(&s, false, true), DashFlag::None);
    crash_behind(&mut s);
    assert_eq!(caution_flag(&s, true, true), DashFlag::None, "crash behind you is not a yellow");
    crash_ahead(&mut s);
    s.riders[0].track_pos = 0.399;
    assert_eq!(caution_flag(&s, true, true), DashFlag::None, "crash on top of you is not a yellow");
    crash_ahead(&mut s);
    s.riders[0].track_pos = 0.55;
    assert_eq!(caution_flag(&s, true, true), DashFlag::None, "crash far ahead is not a yellow");
}

#[test]
fn caution_blue_when_being_lapped() {
    let _g = session_lock();
    reset_session();
    let mut s = mid_race_snap();
    lapping_from_behind(&mut s);
    assert_eq!(lap_rel(&s, 1), LapRel::LappingMe);
    assert_eq!(wanted_flag(&s, false, true), DashFlag::Blue);
    assert_eq!(wanted_flag(&s, false, false), DashFlag::None);
    assert_eq!(wanted_flag(&s, true, false), DashFlag::None);
    s.riders[0].track_pos = 0.28;
    assert_eq!(lap_rel(&s, 1), LapRel::LappingMe);
    assert_eq!(wanted_flag(&s, false, true), DashFlag::None, "lapper still too far for blue");
}

#[test]
fn caution_yellow_beats_blue() {
    let _g = session_lock();
    reset_session();
    let mut s = mid_race_snap();
    lapping_from_behind(&mut s);
    s.rider_count = 3;
    s.riders[2] = rider(5, 12.0, 5.0, 0.43);
    s.riders[2].crashed = 1;
    assert_eq!(caution_flag(&s, true, true), DashFlag::Yellow);
    assert_eq!(caution_flag(&s, false, true), DashFlag::Blue);
}

#[test]
fn caution_loses_to_white_and_checkered() {
    assert_eq!(merge_caution(DashFlag::White, DashFlag::Yellow), DashFlag::White);
    assert_eq!(merge_caution(DashFlag::Checkered, DashFlag::Blue), DashFlag::Checkered);
    assert_eq!(merge_caution(DashFlag::None, DashFlag::Yellow), DashFlag::Yellow);
}

#[test]
fn dash_wrap_skips_caution_flags() {
    assert_eq!(dash_wrap_flag(DashFlag::Yellow, 1.0), (DashFlag::None, 0.0));
    assert_eq!(dash_wrap_flag(DashFlag::Blue, 1.0), (DashFlag::None, 0.0));
    assert_eq!(dash_wrap_flag(DashFlag::White, 0.8), (DashFlag::White, 0.8));
}

#[test]
fn flag_widget_paints_yellow() {
    let _g = session_lock();
    reset_session();
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            reset_flag_display();
        }
    }
    let _pg = Guard;
    set_flag_preview(3);
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Flag].show = true;
    let s = golden_snap(&live_snap(), &cfg);
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut px, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
    let x0 = cfg[WidgetId::Flag].rect.x * 1280.0;
    let y0 = cfg[WidgetId::Flag].rect.y * 720.0;
    let x1 = x0 + cfg[WidgetId::Flag].rect.w * 1280.0;
    let y1 = y0 + cfg[WidgetId::Flag].rect.h * 720.0;
    assert!(rect_has(&px, x0, y0, x1, y1, is_yellow), "yellow flag should paint the cloth");
}

#[test]
fn flag_caption_white_covers_the_label() {
    let _g = session_lock();
    reset_session();
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            reset_flag_display();
        }
    }
    let _pg = Guard;
    set_flag_preview(3);
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Flag].show = true;
    let s = golden_snap(&live_snap(), &cfg);
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut px, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
    let w = cfg[WidgetId::Flag].rect.w * 1280.0;
    let h = cfg[WidgetId::Flag].rect.h * 720.0;
    let x = cfg[WidgetId::Flag].rect.x * 1280.0;
    let y = cfg[WidgetId::Flag].rect.y * 720.0;
    let gw = flag_caption_group_w(&fonts(), h, 1.0, "YELLOW FLAG");
    let mid_y = y + h * 0.5;
    let cx = x + w * 0.5;
    let mut sx = cx - gw * 0.5 - 10.0;
    let x1 = cx + gw * 0.5 + 10.0;
    while sx < x1 {
        let p = sample_px(&px, sx, mid_y);
        let yellow_through = p[3] > 40 && p[0] > 160 && p[2] < 120 && (p[0] as i16 - p[2] as i16) > 80;
        assert!(
            !yellow_through,
            "white plaque should cover the label and side pad at {sx:.0}, got {p:?}"
        );
        sx += 1.0;
    }
}

#[test]
fn flag_caption_white_leaves_cloth_above_and_below() {
    let _g = session_lock();
    reset_session();
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            reset_flag_display();
        }
    }
    let _pg = Guard;
    set_flag_preview(3);
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Flag].show = true;
    let s = golden_snap(&live_snap(), &cfg);
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut px, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
    let x0 = cfg[WidgetId::Flag].rect.x * 1280.0;
    let y0 = cfg[WidgetId::Flag].rect.y * 720.0;
    let w = cfg[WidgetId::Flag].rect.w * 1280.0;
    let h = cfg[WidgetId::Flag].rect.h * 720.0;
    let cx = x0 + w * 0.5;
    let y1 = y0 + h;
    let cloth = |p: [u8; 4]| p[3] > 40 && p[0] > 200 && p[1] > 180 && p[2] < 80;
    assert!(
        (1..4).any(|dy| cloth(sample_px(&px, cx, y0 + dy as f32))),
        "cloth should show above the caption white"
    );
    assert!(
        (1..4).any(|dy| cloth(sample_px(&px, x0 + 12.0, y1 - dy as f32))),
        "cloth should show below the caption white"
    );
}

#[test]
fn flag_text_off_leaves_cloth_with_no_caption() {
    let _g = session_lock();
    reset_session();
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            reset_flag_display();
        }
    }
    let _pg = Guard;
    set_flag_preview(3);
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Flag].show = true;
    cfg.flag_text = false;
    let s = golden_snap(&live_snap(), &cfg);
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut px, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
    let w = cfg[WidgetId::Flag].rect.w * 1280.0;
    let h = cfg[WidgetId::Flag].rect.h * 720.0;
    let x = cfg[WidgetId::Flag].rect.x * 1280.0;
    let y = cfg[WidgetId::Flag].rect.y * 720.0;
    let cx = x + w * 0.5;
    let mid_y = y + h * 0.5;
    assert!(
        is_yellow(sample_px(&px, cx, mid_y)),
        "cloth should show through the center when text is off, got {:?}",
        sample_px(&px, cx, mid_y)
    );
}

#[test]
fn flag_widget_paints_blue() {
    let _g = session_lock();
    reset_session();
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            reset_flag_display();
        }
    }
    let _pg = Guard;
    set_flag_preview(4);
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Flag].show = true;
    let s = golden_snap(&live_snap(), &cfg);
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut px, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
    let x0 = cfg[WidgetId::Flag].rect.x * 1280.0;
    let y0 = cfg[WidgetId::Flag].rect.y * 720.0;
    let x1 = x0 + cfg[WidgetId::Flag].rect.w * 1280.0;
    let y1 = y0 + cfg[WidgetId::Flag].rect.h * 720.0;
    assert!(rect_has(&px, x0, y0, x1, y1, is_blue), "blue flag should paint the cloth");
}

#[test]
fn dash_wrap_does_not_paint_yellow() {
    let _g = session_lock();
    reset_session();
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            reset_flag_display();
        }
    }
    let _pg = Guard;
    set_flag_preview(3);
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Dash].show = true;
    let s = golden_snap(&live_snap(), &cfg);
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut px, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
    let x0 = cfg[WidgetId::Dash].rect.x * 1280.0 - 8.0;
    let y0 = (cfg[WidgetId::Dash].rect.y * 720.0 - 36.0).max(0.0);
    let x1 = (cfg[WidgetId::Dash].rect.x + cfg[WidgetId::Dash].rect.w) * 1280.0 + 8.0;
    let y1 = cfg[WidgetId::Dash].rect.y * 720.0;
    assert!(
        !rect_has(&px, x0, y0, x1, y1, is_yellow),
        "dash wrap must not paint yellow"
    );
}
