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
fn radar_same_stretch_wraps_around_the_lap() {
    let mut s = live_snap();
    s.track_length = 1000.0;
    s.local_track_pos = 0.98;
    assert!(radar_same_stretch(&s, 0.02, 80.0));
    assert!(!radar_same_stretch(&s, 0.40, 80.0));
}

#[test]
fn radar_in_view_covers_blind_spots() {
    assert!(radar_in_view(-3.0, -2.0, true, false, 12.0, 6.0));
    assert!(radar_in_view(-3.0, -2.0, false, true, 12.0, 6.0));
    assert!(!radar_in_view(-3.0, -2.0, false, false, 12.0, 6.0));
    assert!(radar_in_view(0.0, 2.0, true, false, 12.0, 6.0));
    assert!(!radar_in_view(0.0, 2.0, false, true, 12.0, 6.0));
    assert!(radar_in_view(-4.0, 0.0, false, true, 12.0, 6.0));
    assert!(!radar_in_view(-4.0, 0.0, true, false, 12.0, 6.0));
    assert!(!radar_in_view(2.0, 0.0, true, true, 12.0, 6.0));
    assert!(!radar_in_view(-13.0, 0.0, true, true, 12.0, 6.0));
    assert!(!radar_in_view(0.0, 7.0, true, true, 12.0, 6.0));
}

#[test]
fn radar_in_view_honors_range() {
    assert!(!radar_in_view(-18.0, 0.0, true, true, 12.0, 6.0));
    assert!(radar_in_view(-18.0, 0.0, true, true, 24.0, 12.0));
    assert!(!radar_in_view(0.0, 10.0, true, true, 12.0, 6.0));
    assert!(radar_in_view(0.0, 10.0, true, true, 24.0, 12.0));
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
    assert_eq!(radar_blip_heat(1.0, 12.0), 1.0);
    assert!((radar_blip_heat(7.0, 12.0) - 1.0 / 7.0).abs() < 1e-6);
    assert_eq!(radar_blip_heat(8.0, 12.0), 0.0);
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
fn radar_you_outline_reads_on_a_light_sky() {
    let mut px = Pixmap::new(48, 56).expect("pixmap");
    px.fill(Color::from_rgba8(186, 214, 232, 255));
    draw_radar_you(&mut px, 24.0, 28.0, 12.0, 30.0, 160.0);
    let cream = sample_px(&px, 24.0, 28.0);
    assert!(cream[0] > 230 && cream[1] > 230 && cream[2] > 230, "fill {cream:?}");
    let ink = sample_px(&px, 17.0, 28.0);
    assert!(ink[0] < 50 && ink[1] < 50 && ink[2] < 50, "outline {ink:?}");
    let sky = sample_px(&px, 6.0, 28.0);
    assert!(sky[0] > 160 && sky[2] > 200, "sky {sky:?}");
}

#[test]
fn radar_you_stroke_stays_a_hairline() {
    assert!((radar_you_stroke(80.0) - 2.4).abs() < 1e-4);
    assert!((radar_you_stroke(200.0) - 3.6).abs() < 1e-4);
    assert_eq!(rgba8(radar_you_ink()), (14, 14, 16, 255));
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
    let s = radar_fit_scale(160.0, 160.0, 80.0, 40.0, 0.0, 0.0, 8.0, 12.0);
    assert!((s - 6.0).abs() < 1e-4);
    assert!((radar_ring_radius(12.0, s) - 72.0).abs() < 1e-4);
    assert!((radar_ring_radius(6.0, s) - 36.0).abs() < 1e-4);
}

#[test]
fn radar_rings_stay_at_6_and_12() {
    assert_eq!(radar_rings_m(), [3.0, 6.0, 12.0]);
}

#[test]
fn radar_fit_range_only_grows_past_12() {
    assert_eq!(radar_fit_range(6.0), 12.0);
    assert_eq!(radar_fit_range(12.0), 12.0);
    assert_eq!(radar_fit_range(24.0), 24.0);
}

#[test]
fn radar_far_blips_sit_outside_the_12m_ring() {
    let fit = radar_fit_range(24.0);
    let s = radar_fit_scale(160.0, 160.0, 80.0, 40.0, 0.0, 0.0, 8.0, fit);
    let ring12 = radar_ring_radius(12.0, s);
    let (bx, by) = radar_to_screen(-18.0, 0.0, 80.0, 40.0, s, s);
    let dist = ((bx - 80.0).powi(2) + (by - 40.0).powi(2)).sqrt();
    assert!(ring12 < radar_ring_radius(12.0, radar_fit_scale(160.0, 160.0, 80.0, 40.0, 0.0, 0.0, 8.0, 12.0)));
    assert!(dist > ring12);
}

#[test]
fn radar_arrow_on_edge_puts_rear_on_the_bottom() {
    let (x, y, ux, uy) = radar_arrow_on_edge(0.0, 0.0, 200.0, 100.0, -4.0, 0.0, 8.0).unwrap();
    assert!((x - 100.0).abs() < 0.5);
    assert!((y - (100.0 - 8.0)).abs() < 0.5);
    assert!(uy > 0.9);
    assert!(ux.abs() < 0.05);
}

#[test]
fn radar_arrow_on_edge_puts_back_left_on_the_bottom_left() {
    let (x, y, ux, uy) = radar_arrow_on_edge(0.0, 0.0, 200.0, 100.0, -3.0, -2.0, 8.0).unwrap();
    assert!(x < 100.0);
    assert!(y > 50.0);
    assert!(ux < 0.0);
    assert!(uy > 0.0);
}

#[test]
fn radar_arrow_on_edge_puts_right_beside_on_the_right() {
    let (x, y, ux, uy) = radar_arrow_on_edge(0.0, 0.0, 200.0, 100.0, 0.0, 4.0, 8.0).unwrap();
    assert!((x - (200.0 - 8.0)).abs() < 0.5);
    assert!((y - 50.0).abs() < 0.5);
    assert!(ux > 0.9);
    assert!(uy.abs() < 0.05);
}

#[test]
fn radar_bearing_zero_is_ahead() {
    assert!(radar_bearing(4.0, 0.0).abs() < 1e-4);
    assert!((radar_bearing(0.0, 4.0) - std::f32::consts::FRAC_PI_2).abs() < 1e-4);
    assert!(radar_bearing(-4.0, 0.0).abs() > 3.0);
}

#[test]
fn radar_arrow_size_stays_glanceable() {
    let far = radar_arrow_size(0.0, 960.0);
    let close = radar_arrow_size(1.0, 960.0);
    assert!(far >= 28.0);
    assert!(close <= 56.0);
    assert!(close > far);
    assert!((radar_arrow_size(0.0, 80.0) - 28.0).abs() < 1e-3);
    assert!((radar_arrow_size(1.0, 2000.0) - 56.0).abs() < 1e-3);
}

#[test]
fn radar_arrows_flash_on_nearby_crash_then_hide() {
    reset_arrow_crash_flash();
    let (mut s, cfg) = radar_arrow_snap_behind();
    let upright = collect_radar_arrow_blips(&s, &cfg, 0.0);
    assert_eq!(upright.len(), 1);
    assert!(!upright[0].crashed);
    s.riders[0].crashed = 1;
    let flash = collect_radar_arrow_blips(&s, &cfg, 0.0);
    assert_eq!(flash.len(), 1, "rising crash nearby arms a flash");
    assert!(flash[0].crashed);
    force_arrow_crash_flash(1, now_ms() - ARROW_CRASH_HOLD_MS - 1, -5.0, 0.0);
    let gone = collect_radar_arrow_blips(&s, &cfg, 0.0);
    assert!(
        gone.is_empty(),
        "after the hold, sustained crashed riders stay off Arrows"
    );
    let plaque = collect_radar_blips(&s, &cfg, 0.0);
    assert_eq!(plaque.len(), 1, "Plaque still shows crashed riders");
    assert!(plaque[0].crashed);
    reset_arrow_crash_flash();
}

