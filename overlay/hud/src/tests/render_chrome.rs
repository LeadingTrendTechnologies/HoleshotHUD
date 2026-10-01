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

#[test]
fn hairline_fill_rect_does_not_panic() {
    let mut px = Pixmap::new(64, 64).expect("pixmap");
    let r = Rect::from_xywh(10.4, 10.4, 1.3, 1.3).expect("rect");
    fill_rect(&mut px, r, Color::from_rgba8(180, 180, 188, 40));
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
    cfg.map.map_sectors = false;
    let s = golden_snap(&base, &cfg);
    draw_widget_golden("map", &s, &cfg, cfg[WidgetId::Map].rect);
    cfg.map.map_sectors = true;

    hide_widgets(&mut cfg);
    cfg[WidgetId::Minimap].show = true;
    cfg.mini.mini_sectors = false;
    let s = golden_snap(&base, &cfg);
    draw_widget_golden("minimap", &s, &cfg, cfg[WidgetId::Minimap].rect);
    cfg.mini.mini_sectors = true;

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
    crate::delta::set_preview(Some(crate::delta::DeltaView {
        ready: true,
        recording: false,
        has_delta: true,
        delta_ms: 210,
        ref_lap_ms: 70_140,
        last_lap_ms: 69_967,
        cover: 100,
        new_best: false,
    }));
    let s = golden_snap(&sector_snap(base), &cfg);
    draw_widget_golden("sector", &s, &cfg, cfg[WidgetId::Sector].rect);
    crate::delta::set_preview(None);

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
    cfg.stance.stance_style = StanceStyle::Icon;
    cfg.stance.stance_show_sit = true;
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

