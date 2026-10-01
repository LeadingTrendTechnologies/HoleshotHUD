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

