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
fn map_subject_pose_follows_spectated_rider_without_telemetry() {
    let mut s = live_snap();
    s.has_telemetry = 0;
    s.focus_race_num = 1;
    let pose = subject_pose(&s, 0.0).expect("spectate pose");
    assert!(!pose.from_local);
    let mapped = rider_map_pose(&s, &s.riders[0]);
    assert!((pose.x - mapped.x).abs() < 0.01);
    assert!((pose.z - mapped.z).abs() < 0.01);
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
fn telemetry_golden() {
    let _g = session_lock();
    reset_session();
    crate::telemetry::seed_corner();
    let mut s = live_snap();
    s.local_gear = 4;
    s.local_speed = 124.0 / 3.6;
    s.local_rpm = 9800;
    s.max_rpm = 13500;
    s.local_throttle = 0.75;
    s.local_front_brake = 0.18;
    s.local_rear_brake = 0.0;
    s.local_clutch = 0.0;
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Telemetry].show = true;
    cfg[WidgetId::Telemetry].bg = 82;
    cfg[WidgetId::Telemetry].rect.x = 0.18;
    cfg[WidgetId::Telemetry].rect.y = 0.38;
    cfg[WidgetId::Telemetry].rect.w = 0.64;
    cfg[WidgetId::Telemetry].rect.h = 0.20;
    draw_widget_golden("telemetry", &golden_snap(&s, &cfg), &cfg, cfg[WidgetId::Telemetry].rect);
}

#[test]
fn telemetry_can_hide_each_section() {
    let _g = session_lock();
    reset_session();
    crate::telemetry::seed_corner();
    let mut s = live_snap();
    s.local_gear = 2;
    s.local_speed = 20.0;
    s.local_throttle = 0.5;
    s.local_front_brake = 0.2;
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Telemetry].show = true;
    cfg.telemetry.telemetry_traces = false;
    cfg.telemetry.telemetry_bars = false;
    cfg.telemetry.telemetry_dial = false;
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut px, &fonts(), Some(&golden_snap(&s, &cfg)), &cfg, 1280, 720, 0.0, false, false, false);
    cfg.telemetry.telemetry_traces = true;
    cfg.telemetry.telemetry_trace_throttle = false;
    cfg.telemetry.telemetry_bars = true;
    cfg.telemetry.telemetry_bar_clutch = false;
    cfg.telemetry.telemetry_bar_brake = false;
    cfg.telemetry.telemetry_dial = true;
    cfg.telemetry.telemetry_trace_steer = true;
    cfg.telemetry.telemetry_bar_steer = true;
    s.local_steer = -0.22;
    s.steer_lock = 0.40;
    s.shift_rpm = 9000;
    s.max_rpm = 13500;
    s.local_rpm = 9800;
    draw(&mut px, &fonts(), Some(&golden_snap(&s, &cfg)), &cfg, 1280, 720, 0.0, false, false, false);
}

