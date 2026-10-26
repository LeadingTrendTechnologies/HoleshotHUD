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
    imp.units = crate::config::UnitPrefs::all(crate::config::Units::Imperial);
    assert_eq!(dash_foot_item(&s, &imp, DashField::Fuel).unwrap().1, "1.5 gal");
    assert_eq!(dash_foot_item(&s, &cfg, DashField::FuelPct).unwrap().1, "80%");
    assert_eq!(dash_foot_item(&s, &cfg, DashField::Bike).unwrap().1, "YZ450");
    assert_eq!(dash_foot_item(&s, &cfg, DashField::Class).unwrap().1, "MX1");
    assert_eq!(dash_foot_item(&s, &cfg, DashField::Setup).unwrap().1, "Washougal Soft");
    assert_eq!(dash_foot_item(&s, &cfg, DashField::Interval).unwrap().1, "10.000");
    assert_eq!(dash_foot_item(&s, &cfg, DashField::Gap).unwrap().1, "10.000");
    assert_eq!(dash_foot_item(&s, &cfg, DashField::GapBehind).unwrap().1, "---");
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
    cfg.dash.dash_simple = true;
    cfg.dash.dash_rev = true;
    cfg.units = crate::config::UnitPrefs::all(crate::config::Units::Imperial);
    let d = dash_layout(&fonts(), &s, &cfg, 1280.0, 720.0, DashFlag::None, 0.0);
    assert!(d.simple);
    assert_eq!(d.gear, "3");
    assert_eq!(d.speed, "40");
    assert_eq!(d.speed_label, "MPH");
    assert!(d.foot.is_empty());
    assert_eq!(d.rev_h, 0.0);
    assert_eq!(d.footer_h, 0.0);
    assert!((d.w - cfg[WidgetId::Dash].rect.w * 1280.0).abs() < 1.0, "simple dash should fill the widget width, got {}", d.w);
    assert!(!d.lead);
}

#[test]
fn dash_p1_sets_lead_and_p2_does_not() {
    let _g = session_lock();
    reset_session();
    let p2 = live_snap();
    let mut cfg = HudConfig::new();
    cfg[WidgetId::Dash].show = true;
    let d2 = dash_layout(&fonts(), &p2, &cfg, 1280.0, 720.0, DashFlag::None, 0.0);
    assert_eq!(d2.ptxt, "P2");
    assert!(!d2.lead);

    let mut p1 = live_snap();
    p1.standings[0] = standing(12, 1, 5);
    p1.standings[1] = standing(1, 2, 5);
    p1.riders[0].track_pos = 0.40;
    p1.riders[1].track_pos = 0.70;
    p1.local_track_pos = 0.70;
    let d1 = dash_layout(&fonts(), &p1, &cfg, 1280.0, 720.0, DashFlag::None, 0.0);
    assert_eq!(d1.ptxt, "P1");
    assert!(d1.lead);

    p1.show_standings = 0;
    p1.show_relative = 0;
    p1.show_map = 0;
    draw_widget_golden("dash-p1", &golden_snap(&p1, &cfg), &cfg, cfg[WidgetId::Dash].rect);
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
    cfg.dash.dash_simple = true;
    cfg[WidgetId::Dash].bg = 82;
    cfg.units = crate::config::UnitPrefs::all(crate::config::Units::Imperial);
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

struct GuideGuard;

impl Drop for GuideGuard {
    fn drop(&mut self) {
        crate::track_guide::reset();
    }
}

fn guide_guard() -> GuideGuard {
    crate::track_guide::reset();
    GuideGuard
}

fn guide_bike() -> Snapshot {
    let mut snap = Snapshot::default();
    snap.has_telemetry = 1;
    straight_track(&mut snap);
    snap.local_z = 50.0;
    snap
}

fn straight_track(snap: &mut Snapshot) {
    snap.poly_count = 2;
    snap.poly[0] = Point { x: 0.0, z: 0.0 };
    snap.poly[1] = Point { x: 0.0, z: 200.0 };
}

#[test]
fn guide_off_to_the_side_aims_toward_the_line_and_along_it() {
    let mut snap = guide_bike();
    snap.local_x = 30.0;
    let bearing = crate::track_guide::aim(&snap).expect("centerline");
    assert!(
        bearing.angle < -0.35,
        "should point back toward the line, got {}",
        bearing.angle
    );
    assert!(
        bearing.angle > -1.2,
        "should also point along the track, got {}",
        bearing.angle
    );
    assert!(bearing.dist_m > 12.0);
    assert!(!bearing.aligned);
}

#[test]
fn guide_facing_backward_aims_down() {
    let mut snap = guide_bike();
    snap.local_yaw = std::f32::consts::PI;
    let bearing = crate::track_guide::aim(&snap).expect("centerline");
    assert!(
        bearing.angle.abs() > 2.5,
        "facing the wrong way should aim back down, got {}",
        bearing.angle
    );
}

#[test]
fn guide_crash_stays_up_on_the_line() {
    let _guard = guide_guard();
    let mut snap = guide_bike();
    snap.local_crashed = 1;
    crate::track_guide::tick_at(&snap, std::time::Instant::now());
    let bearing = crate::track_guide::visible(&snap).expect("a crash keeps the arrow up");
    assert!(bearing.aligned);
    assert!(bearing.angle.abs() < 0.2);
}

#[test]
fn guide_aligned_join_stays_hidden() {
    let _guard = guide_guard();
    let snap = guide_bike();
    crate::track_guide::tick_at(&snap, std::time::Instant::now());
    assert!(crate::track_guide::visible(&snap).is_none());
}

#[test]
fn guide_without_a_polyline_stays_hidden() {
    let _guard = guide_guard();
    let mut snap = guide_bike();
    snap.poly_count = 0;
    snap.local_x = 40.0;
    snap.local_crashed = 1;
    crate::track_guide::tick_at(&snap, std::time::Instant::now());
    assert!(crate::track_guide::aim(&snap).is_none());
    assert!(crate::track_guide::visible(&snap).is_none());
}

#[test]
fn guide_hides_once_you_are_aligned() {
    let _guard = guide_guard();
    let mut snap = guide_bike();
    snap.local_x = 30.0;
    let now = std::time::Instant::now();
    crate::track_guide::tick_at(&snap, now);
    assert!(crate::track_guide::visible(&snap).is_some());
    snap.local_x = 0.0;
    crate::track_guide::tick_at(&snap, now);
    assert!(
        crate::track_guide::visible(&snap).is_some(),
        "the arrow holds through the first aligned frame"
    );
    crate::track_guide::tick_at(&snap, now + std::time::Duration::from_millis(400));
    assert!(crate::track_guide::visible(&snap).is_none());
}

#[test]
fn guide_off_track_mid_ride_shows() {
    let _guard = guide_guard();
    let mut snap = guide_bike();
    let now = std::time::Instant::now();
    crate::track_guide::tick_at(&snap, now);
    assert!(crate::track_guide::visible(&snap).is_none());
    snap.local_x = 30.0;
    crate::track_guide::tick_at(&snap, now);
    assert!(crate::track_guide::visible(&snap).is_some());
}

#[test]
fn guide_wrong_way_on_the_line_shows() {
    let _guard = guide_guard();
    let mut snap = guide_bike();
    let now = std::time::Instant::now();
    crate::track_guide::tick_at(&snap, now);
    snap.local_yaw = std::f32::consts::PI;
    crate::track_guide::tick_at(&snap, now);
    assert!(crate::track_guide::visible(&snap).is_some());
}

#[test]
fn guide_fold_aims_back_at_the_stretch_you_left() {
    let _guard = guide_guard();
    let mut snap = Snapshot::default();
    snap.has_telemetry = 1;
    snap.poly_count = 4;
    snap.poly[0] = Point { x: 0.0, z: 0.0 };
    snap.poly[1] = Point { x: 0.0, z: 400.0 };
    snap.poly[2] = Point { x: 10.0, z: 400.0 };
    snap.poly[3] = Point { x: 10.0, z: 0.0 };
    snap.local_z = 40.0;
    let now = std::time::Instant::now();
    crate::track_guide::tick_at(&snap, now);
    snap.local_x = 10.0;
    snap.local_z = 250.0;
    let bearing = crate::track_guide::aim(&snap).expect("folded centerline");
    assert!(
        bearing.target_x.abs() < 1.0,
        "stay on the straight you left, x {}",
        bearing.target_x
    );
    assert!(
        (bearing.target_z - 52.0).abs() < 2.0,
        "12 m ahead of the exit, z {}",
        bearing.target_z
    );
    assert!(
        bearing.dist_m > 12.0,
        "the other straight still counts as off, dist {}",
        bearing.dist_m
    );
}

#[test]
fn guide_corner_on_the_line_stays_hidden() {
    let _guard = guide_guard();
    let mut snap = guide_bike();
    let now = std::time::Instant::now();
    crate::track_guide::tick_at(&snap, now);
    snap.local_yaw = 50.0 * std::f32::consts::PI / 180.0;
    crate::track_guide::tick_at(&snap, now);
    assert!(crate::track_guide::visible(&snap).is_none());
}

#[test]
fn guide_labs_off_hides_the_chevron() {
    let _guard = guide_guard();
    let _session = session_lock();
    reset_session();
    let mut snap = guide_bike();
    snap.local_crashed = 1;
    crate::track_guide::tick_at(&snap, std::time::Instant::now());
    let cfg = HudConfig::new();
    assert!(cfg.dash.dash_guide);
    assert!(!guide_lifted(&snap, &cfg));
    let fonts = fonts();
    let red = guide_slot_red(&snap, &cfg, &fonts, 1920.0, 1080.0);
    assert!(red < 80, "labs off draws no chevron, red {red}");
}

#[test]
fn guide_toggle_off_hides_the_arrow_and_keeps_the_gear() {
    let _guard = guide_guard();
    let _session = session_lock();
    reset_session();
    let mut snap = guide_bike();
    snap.local_crashed = 1;
    crate::track_guide::tick_at(&snap, std::time::Instant::now());
    let mut cfg = HudConfig::new();
    cfg.experimental = true;
    cfg.dash.dash_guide = false;
    let fonts = fonts();
    let layout = dash_layout(&fonts, &snap, &cfg, 1920.0, 1080.0, DashFlag::None, 0.0);
    assert!(gear_text_y(&layout, true) < gear_text_y(&layout, false) - 1.0);
    assert!(!guide_lifted(&snap, &cfg));
    let red = guide_slot_red(&snap, &cfg, &fonts, 1920.0, 1080.0);
    assert!(red < 80, "toggle off does not draw the chevron, red {red}");
}

#[test]
fn guide_showing_lifts_the_gear_digit() {
    let _guard = guide_guard();
    let _session = session_lock();
    reset_session();
    let mut snap = guide_bike();
    let mut cfg = HudConfig::new();
    cfg.experimental = true;
    let fonts = fonts();
    crate::track_guide::tick_at(&snap, std::time::Instant::now());
    let resting = gear_digit_top(&snap, &cfg, &fonts);
    snap.local_crashed = 1;
    crate::track_guide::tick_at(&snap, std::time::Instant::now());
    assert!(guide_lifted(&snap, &cfg));
    let lifted = gear_digit_top(&snap, &cfg, &fonts);
    assert!(
        lifted + 2 < resting,
        "digit moves up while the arrow shows, lifted {lifted} resting {resting}"
    );
}

#[test]
fn guide_chevron_sits_under_the_gear() {
    let _guard = guide_guard();
    let _session = session_lock();
    reset_session();
    let mut snap = guide_bike();
    let mut cfg = HudConfig::new();
    cfg.experimental = true;
    let fonts = fonts();
    crate::track_guide::tick_at(&snap, std::time::Instant::now());
    let hidden = guide_slot_red(&snap, &cfg, &fonts, 1920.0, 1080.0);
    snap.local_crashed = 1;
    crate::track_guide::tick_at(&snap, std::time::Instant::now());
    let showing = guide_slot_match(&snap, &cfg, &fonts, 1920.0, 1080.0, accent_pixel);
    assert!(
        hidden < 80 && showing,
        "full dash arrow is the accent under the gear, hidden {hidden} showing {showing}",
    );

    let mut simple = cfg.clone();
    simple.dash.dash_simple = true;
    let showing = guide_slot_match(&snap, &simple, &fonts, 1280.0, 720.0, ink_pixel);
    crate::track_guide::reset();
    snap.local_crashed = 0;
    crate::track_guide::tick_at(&snap, std::time::Instant::now());
    let quiet = guide_slot_color(&snap, &simple, &fonts, 1280.0, 720.0);
    assert!(
        showing && quiet.red() > 180,
        "simple dash arrow is ink on the orange gear, showing {showing} hidden {}",
        quiet.red()
    );
}

#[test]
fn guide_widget_draws_only_while_shown_and_armed() {
    let _guard = guide_guard();
    let _session = session_lock();
    reset_session();
    let mut snap = guide_bike();
    snap.local_crashed = 1;
    crate::track_guide::tick_at(&snap, std::time::Instant::now());
    let mut cfg = HudConfig::new();
    cfg.experimental = true;
    cfg[WidgetId::Guide].show = true;
    cfg[WidgetId::Guide].rect = crate::shm::Rect {
        x: 0.25,
        y: 0.25,
        w: 0.5,
        h: 0.5,
    };
    let mut shown = Pixmap::new(200, 200).expect("pixmap");
    draw_guide(&mut shown, &snap, &cfg, 200.0, 200.0);
    let armed = shown.pixel(100, 100).expect("chevron");
    assert!(
        accent_pixel(armed),
        "shown guide is the accent chevron, rgb {} {} {}",
        armed.red(),
        armed.green(),
        armed.blue()
    );
    cfg[WidgetId::Guide].show = false;
    let mut hidden = Pixmap::new(200, 200).expect("pixmap");
    draw_guide(&mut hidden, &snap, &cfg, 200.0, 200.0);
    let quiet = hidden.pixel(100, 100).expect("empty");
    assert!(
        quiet.red() < 20,
        "hidden guide draws nothing, red {}",
        quiet.red()
    );
}

fn accent_pixel(pixel: tiny_skia::PremultipliedColorU8) -> bool {
    pixel.red() > 200 && pixel.green() > 90 && pixel.green() < 210 && pixel.blue() < 120
}

fn ink_pixel(pixel: tiny_skia::PremultipliedColorU8) -> bool {
    pixel.red() < 40 && pixel.green() < 40 && pixel.blue() < 40
}

fn gear_digit_top(snap: &Snapshot, cfg: &HudConfig, fonts: &Fonts) -> i32 {
    let layout = dash_layout(fonts, snap, cfg, 1920.0, 1080.0, DashFlag::None, 0.0);
    let mut px = Pixmap::new(1920, 1080).expect("pixmap");
    draw_dash(
        &mut px,
        fonts,
        snap,
        cfg,
        1920.0,
        1080.0,
        DashFlag::None,
        0.0,
    );
    let x = (layout.gear_x + layout.gear_w * 0.5).round() as i32;
    let y0 = (layout.main_y + 2.0).round() as i32;
    let y1 = (layout.main_y + layout.main_h - 2.0).round() as i32;
    for y in y0..y1 {
        let pixel = px.pixel(x as u32, y as u32).expect("gear column");
        if pixel.red() > 180 && pixel.green() > 180 && pixel.blue() > 180 {
            return y;
        }
    }
    y1
}

fn guide_slot_red(snap: &Snapshot, cfg: &HudConfig, fonts: &Fonts, sw: f32, sh: f32) -> u8 {
    guide_slot_color(snap, cfg, fonts, sw, sh).red()
}

fn guide_slot_color(
    snap: &Snapshot,
    cfg: &HudConfig,
    fonts: &Fonts,
    sw: f32,
    sh: f32,
) -> tiny_skia::PremultipliedColorU8 {
    let painted = guide_slot_pixmap(snap, cfg, fonts, sw, sh);
    let (center_x, center_y, _) = painted.slot;
    painted
        .px
        .pixel(center_x.round() as u32, center_y.round() as u32)
        .expect("guide pixel")
}

fn guide_slot_match(
    snap: &Snapshot,
    cfg: &HudConfig,
    fonts: &Fonts,
    sw: f32,
    sh: f32,
    matches: fn(tiny_skia::PremultipliedColorU8) -> bool,
) -> bool {
    let painted = guide_slot_pixmap(snap, cfg, fonts, sw, sh);
    let (center_x, center_y, height) = painted.slot;
    let x0 = center_x.round() as i32;
    let y0 = center_y.round() as i32;
    let radius = ((height * 0.55).ceil() as i32).clamp(2, 10);
    for y in (y0 - radius)..=(y0 + radius) {
        for x in (x0 - radius)..=(x0 + radius) {
            if painted.px.pixel(x as u32, y as u32).is_some_and(matches) {
                return true;
            }
        }
    }
    false
}

struct GuidePaint {
    px: Pixmap,
    slot: (f32, f32, f32),
}

fn guide_slot_pixmap(snap: &Snapshot, cfg: &HudConfig, fonts: &Fonts, sw: f32, sh: f32) -> GuidePaint {
    let layout = dash_layout(fonts, snap, cfg, sw, sh, DashFlag::None, 0.0);
    let mut px = Pixmap::new(sw as u32, sh as u32).expect("pixmap");
    draw_dash(&mut px, fonts, snap, cfg, sw, sh, DashFlag::None, 0.0);
    GuidePaint {
        px,
        slot: guide_slot(&layout),
    }
}

