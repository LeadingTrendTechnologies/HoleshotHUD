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
fn sector_glass_caption_rim_reads_on_a_light_sky() {
    let _g = session_lock();
    let _pb = crate::track_pb::exclusive_test();
    reset_session();
    crate::track_pb::reset_store();
    crate::sector::reset_engine();
    crate::delta::set_preview(None);
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Sector].show = true;
    cfg[WidgetId::Sector].bg = 0;
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
    let s = golden_snap(&sector_snap(live_snap()), &cfg);
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    px.fill(Color::from_rgba8(186, 214, 232, 255));
    draw(
        &mut px,
        &fonts(),
        Some(&s),
        &cfg,
        1280,
        720,
        0.0,
        false,
        false,
        false,
    );
    crate::delta::set_preview(None);
    let r = cfg[WidgetId::Sector].rect;
    let x0 = (r.x * 1280.0).floor() as u32;
    let y0 = (r.y * 720.0).floor() as u32;
    let w = (r.w * 1280.0).ceil() as u32;
    let mut ink = 0u32;
    for y in y0..y0 + 28 {
        for x in x0..x0 + w {
            let p = sample_px(&px, x as f32, y as f32);
            if p[0] < 40 && p[1] < 40 && p[2] < 40 && p[3] > 100 {
                ink += 1;
            }
        }
    }
    assert!(ink > 40, "caption rim missing on sky, ink pixels={ink}");
}

#[test]
fn sector_col_widths_cover_need_and_give_extra_to_hero() {
    let need = [40.0, 40.0, 50.0, 60.0];
    let w = sector_col_widths(need, 250.0, 2);
    assert!((w[0] - 40.0).abs() < 0.01);
    assert!((w[1] - 40.0).abs() < 0.01);
    assert!((w[2] - 110.0).abs() < 0.01);
    assert!((w[3] - 60.0).abs() < 0.01);
    assert!((w.iter().sum::<f32>() - 250.0).abs() < 0.01);
}

#[test]
fn sector_col_widths_never_overlap_when_tight() {
    let need = [80.0, 80.0, 80.0, 80.0];
    let w = sector_col_widths(need, 200.0, 0);
    assert!((w.iter().sum::<f32>() - 200.0).abs() < 0.01);
    for got in w {
        assert!((got - 50.0).abs() < 0.01);
    }
}

#[test]
fn sector_columns_cover_live_times_at_large_font() {
    let _g = session_lock();
    let _pb = crate::track_pb::exclusive_test();
    reset_session();
    crate::track_pb::reset_store();
    crate::sector::reset_engine();
    crate::delta::set_preview(None);
    let fonts = fonts();
    let _style = push_style(&fonts, true, 160);
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Sector].show = true;
    cfg[WidgetId::Sector].font = 160;
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
    let s = golden_snap(&sector_snap(live_snap()), &cfg);
    let live_h = 72.0;
    let hist_fs = 14.0;
    // Default sector is ~36% of 1280, minus pad/gutter — not 720.
    let cols_w = 420.0;
    let need = sector_col_need(&fonts, &s, &cfg, live_h, hist_fs, true);
    let widths = sector_col_widths(need, cols_w, sector_hero_index(&s));
    crate::delta::set_preview(None);
    for i in 0..3 {
        let row = sector_row(&s, i, true, false);
        let (_, delta_fs, time_fs) = sector_live_type(live_h, row.fresh);
        let text_max = (widths[i] - 8.0).max(8.0);
        let d_fs = sector_fit_probe(&fonts, SECTOR_DELTA_PROBES, delta_fs, text_max);
        let t_fs = sector_fit_probe(
            &fonts,
            SECTOR_SPLIT_PROBES,
            time_fs,
            (text_max - SECTOR_PILL_PAD_X).max(8.0),
        );
        let dw = measure(&fonts, SECTOR_PROBE_DELTA, d_fs);
        let tw = measure(&fonts, SECTOR_PROBE_SPLIT_LONG, t_fs);
        assert!(
            dw <= text_max + 0.5,
            "col {i} delta {} > {}",
            dw,
            text_max
        );
        assert!(
            tw <= text_max + 0.5,
            "col {i} split {} > {}",
            tw,
            text_max
        );
    }
    let lap_max = (widths[3] - 8.0).max(8.0);
    let lap_fs = sector_fit_probe(
        &fonts,
        SECTOR_LAP_PROBES,
        (live_h * 0.28).clamp(12.0, 22.0),
        lap_max,
    );
    let lap_w = measure(&fonts, SECTOR_PROBE_LAP, lap_fs);
    assert!(
        lap_w <= lap_max + 0.5,
        "LAP width {} > {}",
        lap_w,
        lap_max
    );
}

#[test]
fn sector_pills_cover_scaled_type_and_clear_the_corner() {
    let _style = push_style(&fonts(), true, 160);
    let (_, h15) = sector_pill_metrics(15.0);
    let (_, h9) = sector_pill_metrics(9.0);
    assert!(h15 >= 15.0 * style_k() + 5.9, "pill must wrap 160% glyphs, h={h15}");
    assert!(h9 >= 9.0 * style_k() + 5.9, "flank pill must wrap 160% glyphs, h={h9}");
    let widget_h = 200.0_f32;
    let pad_y = 5.0_f32;
    let live_bottom = widget_h - pad_y.max(SECTOR_CORNER + 3.0);
    let pill_bottom = live_bottom;
    assert!(
        pill_bottom <= widget_h - SECTOR_CORNER,
        "pill bottom {pill_bottom} must sit above the 6px corner"
    );
}

#[test]
fn sector_caption_clears_labels_when_font_is_large() {
    let fonts = fonts();
    let _style = push_style(&fonts, true, 160);
    let cap_fs = 16.0;
    let cap_y = 4.0;
    let body_y = cap_y + cap_fs * style_k() + 4.0;
    assert!(
        body_y - cap_y > cap_fs + 4.0,
        "160% font must push labels below the caption, body_y={body_y} k={}",
        style_k()
    );
}

#[test]
fn sector_col_need_ignores_live_digit_growth() {
    let _g = session_lock();
    let _pb = crate::track_pb::exclusive_test();
    reset_session();
    crate::track_pb::reset_store();
    crate::sector::reset_engine();
    crate::delta::set_preview(None);
    let fonts = fonts();
    let _style = push_style(&fonts, true, 100);
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Sector].show = true;
    let mut short = sector_snap(live_snap());
    short.current_lap_ms = 9_123;
    short.sector_cur = [9_123, 8_000, 0];
    short.sector_delta = [-12, 34, 0];
    let mut long = short;
    long.current_lap_ms = 130_000;
    long.sector_cur = [70_000, 40_000, 0];
    long.sector_delta = [12_345, 8_888, 0];
    crate::delta::set_preview(Some(crate::delta::DeltaView {
        ready: true,
        recording: true,
        has_delta: true,
        delta_ms: 12_345,
        ref_lap_ms: 70_140,
        last_lap_ms: 69_967,
        cover: 100,
        new_best: false,
    }));
    let a = sector_col_need(&fonts, &golden_snap(&short, &cfg), &cfg, 72.0, 14.0, true);
    let b = sector_col_need(&fonts, &golden_snap(&long, &cfg), &cfg, 72.0, 14.0, true);
    crate::delta::set_preview(None);
    assert_eq!(a, b, "columns must not follow ticking digits");
}

#[test]
fn sector_num_x_centers_in_column() {
    let fonts = fonts();
    let _style = push_style(&fonts, true, 100);
    let slot = sector_probe_max(&fonts, 14.0, SECTOR_LAP_PROBES);
    let short = sector_num_x(&fonts, "9.123", 14.0, 0.0, 200.0, slot);
    let long = sector_num_x(&fonts, "1:10.000", 14.0, 0.0, 200.0, slot);
    let short_mid = short + measure(&fonts, "9.123", 14.0) * 0.5;
    let long_mid = long + measure(&fonts, "1:10.000", 14.0) * 0.5;
    assert!(
        (short_mid - 100.0).abs() < 0.6,
        "short mid {short_mid}"
    );
    assert!((long_mid - 100.0).abs() < 0.6, "long mid {long_mid}");
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

