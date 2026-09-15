#![allow(unused_imports)]
use super::*;
use crate::review::{self, ListFilter, RiderRow, SessionDetail};
use std::sync::OnceLock;
use tiny_skia::{FilterQuality, FillRule, StrokeDash};

thread_local! {
    static ANALYZE_TRACK: std::cell::RefCell<Option<(AnalyzeTrackKey, Pixmap)>> =
        std::cell::RefCell::new(None);
    static ANALYZE_TRACK_PENDING: std::cell::RefCell<Option<AnalyzeTrackKey>> =
        std::cell::RefCell::new(None);
    static ANALYZE_INK: std::cell::RefCell<Option<(u32, u32, Pixmap)>> =
        std::cell::RefCell::new(None);
}

#[derive(Clone, Copy, PartialEq)]
struct AnalyzeTrackKey {
    id: i64,
    mw: u32,
    mh: u32,
    zoom: u32,
    pan_x: u32,
    pan_z: u32,
    sf_m: i32,
    s2_m: i32,
    s3_m: i32,
}

fn night_ink() -> Color {
    Color::from_rgba8(10, 10, 10, 255)
}

fn ahead_green() -> Color {
    Color::from_rgba8(48, 220, 88, 255)
}

fn behind_red() -> Color {
    Color::from_rgba8(255, 64, 72, 255)
}

fn you_row() -> Color {
    Color::from_rgba8(196, 132, 36, 92)
}

fn best_violet() -> Color {
    Color::from_rgba8(196, 112, 255, 255)
}

fn well() -> Color {
    Color::from_rgba8(20, 20, 22, 255)
}

fn delta_col(v: f32, lower_better: bool) -> Color {
    if !v.is_finite() || v.abs() < 1.0e-3 {
        return muted();
    }
    let better = if lower_better { v < 0.0 } else { v > 0.0 };
    if better {
        ahead_green()
    } else {
        behind_red()
    }
}

fn rider_class(bike: &str) -> String {
    let cc = mxbo_hud::track_pb::bike_class(bike);
    if !cc.is_empty() && cc.chars().all(|c| c.is_ascii_digit()) {
        cc
    } else {
        String::new()
    }
}

pub(crate) fn pane_review(
    px: &mut Pixmap,
    fonts: &Fonts,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
    analyze_id: Option<i64>,
    filter: ListFilter,
    compare: i32,
    you_lap: i32,
    warmup: bool,
    open_drop: Option<Drop>,
    scrub: f32,
    zoom: f32,
    pan_x: f32,
    pan_z: f32,
    follow: bool,
    recording: bool,
    units: Units,
) -> f32 {
    if let Some(id) = analyze_id {
        return pane_analyze(
            px, fonts, hover, hits, x, y, w, id, compare, you_lap, warmup, open_drop, scrub, zoom,
            pan_x, pan_z, follow, units,
        );
    }
    let has_races = !review::list(ListFilter::All).is_empty();
    if !recording && !has_races {
        return pane_review_gate(px, fonts, hover, hits, x, y, w);
    }
    let sub = if recording {
        "Races on this PC. Save keeps one past two weeks"
    } else {
        "Recording is off. New motos will not be saved."
    };
    let mut y = heading(
        px,
        fonts,
        x,
        y,
        w,
        "Motos",
        sub,
        Some((recording, Hit::ReviewToggle, "Record")),
        hover,
        hits,
    );
    let chip_y = y;
    let _ = filter_chip(
        px,
        fonts,
        x,
        chip_y,
        "All",
        filter == ListFilter::All,
        Hit::ReviewFilterAll,
        hover,
        hits,
    );
    let _ = filter_chip(
        px,
        fonts,
        x + 72.0,
        chip_y,
        "Saved",
        filter == ListFilter::Saved,
        Hit::ReviewFilterSaved,
        hover,
        hits,
    );
    y = chip_y + 36.0;
    draw_review_sheet(px, fonts, hover, hits, x, y, w, filter)
}

fn pane_review_gate(
    px: &mut Pixmap,
    fonts: &Fonts,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let mut y = heading(
        px,
        fonts,
        x,
        y,
        w,
        "Motos",
        "Local races on this PC",
        None,
        hover,
        hits,
    );
    text(px, fonts, "Experimental", 13.0, x + 4.0, y, caution(), false);
    y += 22.0;
    let copy = "After a moto, Analyze shows your line on the map beside SPEED, HEIGHT, and throttle tapes. Zoom a corner, compare another rider at the same spot, and read Results. Motos last two weeks unless you Save. Nothing is recorded until you enable it.";
    for line in wrap_fb(fonts, copy, (w - 8.0).max(40.0), 13.0) {
        text(px, fonts, &line, 13.0, x + 4.0, y, muted(), false);
        y += 20.0;
    }
    y += 10.0;
    let btn_h = 36.0;
    let gap = 12.0;
    let btn_top = (px.height() as f32 - 16.0 - btn_h).max(y + 72.0);
    let well_h = (btn_top - gap - y).max(48.0);
    blit_gate_preview(px, x, y, w, well_h);
    y = y + well_h + gap;
    action_btn(
        px,
        fonts,
        x,
        y,
        128.0,
        36.0,
        "Enable",
        Hit::ReviewToggle,
        hover,
        hits,
        true,
    );
    y + 52.0
}

fn gate_preview_png() -> Option<&'static Pixmap> {
    static PNG: OnceLock<Option<Pixmap>> = OnceLock::new();
    PNG.get_or_init(|| Pixmap::decode_png(include_bytes!("review-enable.png")).ok())
        .as_ref()
}

fn blit_gate_preview(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32) {
    if w < 40.0 || h < 40.0 {
        return;
    }
    fill_round(px, x, y, w, h, 6.0, night_ink());
    let Some(src) = gate_preview_png() else {
        return;
    };
    let pad = 6.0;
    let iw = src.width() as f32;
    let ih = src.height() as f32;
    if iw < 1.0 || ih < 1.0 {
        return;
    }
    let scale = ((w - pad * 2.0) / iw)
        .min((h - pad * 2.0) / ih)
        .max(0.01);
    let dw = iw * scale;
    let dh = ih * scale;
    let dx = x + (w - dw) * 0.5;
    let dy = y + (h - dh) * 0.5;
    let mut paint = PixmapPaint::default();
    paint.quality = FilterQuality::Bicubic;
    let _ = px.draw_pixmap(
        0,
        0,
        src.as_ref(),
        &paint,
        Transform::from_row(scale, 0.0, 0.0, scale, dx, dy),
        None,
    );
}

fn pane_analyze(
    px: &mut Pixmap,
    fonts: &Fonts,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
    id: i64,
    compare: i32,
    you_lap: i32,
    warmup: bool,
    open_drop: Option<Drop>,
    scrub: f32,
    zoom: f32,
    pan_x: f32,
    pan_z: f32,
    follow: bool,
    units: Units,
) -> f32 {
    let Some(detail) = review::load(id) else {
        return heading(
            px,
            fonts,
            x,
            y,
            w,
            "Analyze",
            "Race is gone",
            None,
            hover,
            hits,
        );
    };
    let mut y = heading(
        px,
        fonts,
        x,
        y,
        w,
        "Analyze",
        &detail.row.track,
        None,
        hover,
        hits,
    );
    let back = icon_hit(
        px,
        x,
        y,
        Hit::ReviewBack,
        hover,
        hits,
        IconKind::Back,
        false,
    );
    let del = icon_hit(
        px,
        x + 36.0,
        y,
        Hit::ReviewDelete(id as u64),
        hover,
        hits,
        IconKind::Trash,
        false,
    );
    let keep = icon_hit(
        px,
        x + 72.0,
        y,
        Hit::ReviewKeep(id as u64),
        hover,
        hits,
        IconKind::Bookmark,
        detail.row.kept,
    );
    let mut hx = 116.0;
    if review::live_id() == Some(id) {
        text(px, fonts, "Live", 12.0, x + hx, y + 10.0, dim(), false);
        hx += 40.0;
    }
    if detail.row.you_won {
        draw_win_crown(px, fonts, x + hx, y + 9.0, 14.0);
    }
    if let Some((ix, iy, label)) = back.or(del).or(keep) {
        paint_icon_tip(px, fonts, x, w, ix, iy, label);
    }
    y += 36.0;

    let has_race = !your_laps(&detail, false).is_empty();
    let has_wu = !your_laps(&detail, true).is_empty();
    let warmup = if has_wu && !has_race {
        true
    } else if has_race && !warmup {
        false
    } else {
        warmup && has_wu
    };

    let other = pick_compare(&detail, compare);
    review::ensure_line(id, other);
    let detail = review::load(id).unwrap_or(detail);
    let picked = pick_you_lap(&detail, you_lap, warmup);
    let other_lap = compare_lap(&detail, other);

    let compare_w = 280.0_f32.min((w * 0.36).max(180.0));
    let lap_w = (w - compare_w - 12.0).max(220.0);
    let row_y = y;
    let _ = draw_you_lap_stepper(
        px, fonts, x, row_y, lap_w, hover, hits, &detail, you_lap, warmup, open_drop, has_wu,
        has_race,
    );
    let _ = draw_compare_drop(
        px,
        fonts,
        x + lap_w + 12.0,
        row_y,
        compare_w,
        hover,
        hits,
        &detail,
        other,
        open_drop,
    );
    y = row_y + 40.0;
    if let Some(cap) = crash_caption_for(picked) {
        text(
            px,
            fonts,
            &cap,
            12.0,
            x + 4.0,
            y,
            Color::from_rgba8(232, 56, 56, 255),
            false,
        );
        y += 18.0;
    }
    if let Some(cap) = cut_caption_for(picked) {
        text(
            px,
            fonts,
            &cap,
            12.0,
            x + 4.0,
            y,
            Color::from_rgba8(232, 56, 56, 255),
            false,
        );
        y += 18.0;
    }

    let pos = scrub.clamp(0.0, 1.0);
    let gap = 12.0;
    let side = w >= 700.0;
    let inner = if side { (w - gap).max(1.0) } else { w };
    let map_w = if side { inner * 0.50 } else { w };
    let tape_w = if side { inner * 0.50 } else { w };
    let board_h = (map_w * 0.88).clamp(220.0, 360.0);

    hits.push(HitBox {
        id: Hit::AnalyzeMap,
        x,
        y,
        w: map_w,
        h: board_h,
    });
    draw_analyze_map(
        px, fonts, x, y, map_w, board_h, &detail, compare, you_lap, warmup, scrub, zoom, pan_x,
        pan_z, follow,
    );
    if zoom > 1.0 {
        draw_follow_chip(px, fonts, x, y, map_w, follow, hover, hits);
    }
    if side {
        draw_spot_charts(
            px,
            fonts,
            x + map_w + gap,
            y,
            tape_w,
            board_h,
            picked,
            other_lap,
            pos,
            follow,
            zoom,
        );
        y += board_h + 10.0;
    } else {
        y += board_h + 10.0;
        let tape_h = 168.0;
        draw_spot_charts(
            px, fonts, x, y, w, tape_h, picked, other_lap, pos, follow, zoom,
        );
        y += tape_h + 10.0;
    }

    draw_scrubber(px, fonts, x, y, w, hover, hits, scrub);
    y += 28.0;

    y = draw_spot_stats(
        px, fonts, x, y, w, &detail, picked, other, other_lap, pos, units,
    );
    y += 8.0;

    y = section(px, fonts, x, y, "Results");
    y = draw_results(px, fonts, x, y, w, &detail);
    y + 16.0
}

const SHEET_WHEN: f32 = 84.0;
const SHEET_RIDERS: f32 = 44.0;
const SHEET_YOU: f32 = 104.0;
const SHEET_FAST: f32 = 148.0;
const SHEET_ACT: f32 = 76.0;
const SHEET_PAD: f32 = 8.0;
const SHEET_ROW: f32 = 42.0;
const SHEET_ICON: f32 = 32.0;

fn sheet_hair() -> Color {
    Color::from_rgba8(42, 42, 46, 255)
}

fn draw_review_sheet(
    px: &mut Pixmap,
    fonts: &Fonts,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
    filter: ListFilter,
) -> f32 {
    let rows = review::list(filter);
    if rows.is_empty() {
        text(
            px,
            fonts,
            "No races yet. Finish a moto and it shows up here.",
            13.0,
            x + 4.0,
            y + 8.0,
            dim(),
            false,
        );
        return y + 48.0;
    }
    let cols = sheet_columns(x, w);
    text(px, fonts, "WHEN", 10.0, cols.when, y, dim(), false);
    text(px, fonts, "TRACK", 10.0, cols.track, y, dim(), false);
    text(px, fonts, "RIDERS", 10.0, cols.riders, y, dim(), false);
    text(px, fonts, "YOU", 10.0, cols.you, y, dim(), false);
    text(px, fonts, "FASTEST", 10.0, cols.fast, y, dim(), false);
    let mut y = y + 18.0;
    if let Some(r) = Rect::from_xywh(x, y, w, 1.0) {
        fill_rect(px, r, sheet_hair());
    }
    y += 4.0;
    let mut tip = None;
    for row in rows {
        let hid = Hit::ReviewOpen(row.id as u64);
        hits.push(HitBox {
            id: hid,
            x,
            y,
            w: (cols.act - x).max(40.0),
            h: SHEET_ROW,
        });
        if hover == Some(hid)
            || hover == Some(Hit::ReviewDelete(row.id as u64))
            || hover == Some(Hit::ReviewKeep(row.id as u64))
        {
            fill_round(
                px,
                x,
                y,
                w,
                SHEET_ROW,
                6.0,
                Color::from_rgba8(255, 255, 255, 10),
            );
        }
        let ty = y + 13.0;
        text(
            px,
            fonts,
            &format_when(row.started),
            10.0,
            cols.when,
            ty + 2.0,
            dim(),
            false,
        );
        let track = ellipsize_heading(fonts, &row.track, 14.0, cols.track_w - 4.0);
        text(px, fonts, &track, 14.0, cols.track, ty, text_col(), false);
        text(
            px,
            fonts,
            &row.rider_count.to_string(),
            13.0,
            cols.riders,
            ty,
            muted(),
            false,
        );
        text(
            px,
            fonts,
            &fmt_lap(row.your_best_ms),
            13.0,
            cols.you,
            ty,
            text_col(),
            false,
        );
        if row.you_won {
            let tw = measure(fonts, &fmt_lap(row.your_best_ms), 13.0);
            draw_win_crown(px, fonts, cols.you + tw + 6.0, ty + 1.0, 13.0);
        }
        let fast_time = fmt_lap(row.fastest_ms);
        let time_w = measure(fonts, &fast_time, 13.0);
        let name_w = (SHEET_FAST - time_w - 8.0).max(20.0);
        let name = ellipsize_heading(fonts, &row.fastest_name, 11.0, name_w);
        text(px, fonts, &name, 11.0, cols.fast, ty + 2.0, muted(), false);
        let name_drawn = measure(fonts, &name, 11.0);
        text(
            px,
            fonts,
            &fast_time,
            13.0,
            cols.fast + name_drawn + 6.0,
            ty,
            text_col(),
            false,
        );
        let del = icon_hit(
            px,
            cols.act,
            y + (SHEET_ROW - SHEET_ICON) * 0.5,
            Hit::ReviewDelete(row.id as u64),
            hover,
            hits,
            IconKind::Trash,
            false,
        );
        let keep = icon_hit(
            px,
            cols.act + 36.0,
            y + (SHEET_ROW - SHEET_ICON) * 0.5,
            Hit::ReviewKeep(row.id as u64),
            hover,
            hits,
            IconKind::Bookmark,
            row.kept,
        );
        tip = del.or(keep).or(tip);
        y += SHEET_ROW;
        if let Some(r) = Rect::from_xywh(x, y, w, 1.0) {
            fill_rect(px, r, sheet_hair());
        }
    }
    if let Some((ix, iy, label)) = tip {
        paint_icon_tip(px, fonts, x, w, ix, iy, label);
    }
    y + 12.0
}

struct SheetCols {
    when: f32,
    track: f32,
    track_w: f32,
    riders: f32,
    you: f32,
    fast: f32,
    act: f32,
}

fn sheet_columns(x: f32, w: f32) -> SheetCols {
    let inner = (w - SHEET_PAD * 2.0).max(1.0);
    let fixed = SHEET_WHEN + SHEET_RIDERS + SHEET_YOU + SHEET_FAST + SHEET_ACT;
    let track_w = (inner - fixed).clamp(96.0, 220.0);
    let when = x + SHEET_PAD;
    let track = when + SHEET_WHEN;
    let riders = track + track_w;
    let you = riders + SHEET_RIDERS;
    let fast = you + SHEET_YOU;
    let act = x + w - SHEET_PAD - SHEET_ACT;
    SheetCols {
        when,
        track,
        track_w,
        riders,
        you,
        fast,
        act,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum IconKind {
    Back,
    Trash,
    Bookmark,
}

fn icon_tip_label(kind: IconKind, on: bool) -> &'static str {
    match kind {
        IconKind::Back => "Back",
        IconKind::Trash => "Delete",
        IconKind::Bookmark => {
            if on {
                "Saved"
            } else {
                "Save"
            }
        }
    }
}

fn paint_icon_tip(
    px: &mut Pixmap,
    fonts: &Fonts,
    pane_x: f32,
    pane_w: f32,
    icon_x: f32,
    icon_y: f32,
    label: &str,
) {
    let pad_x = 8.0;
    let pad_y = 6.0;
    let tw = measure(fonts, label, 11.0) + pad_x * 2.0;
    let max_x = (pane_x + pane_w - tw).max(pane_x);
    let tx = (icon_x + SHEET_ICON * 0.5 - tw * 0.5).clamp(pane_x, max_x);
    let ty = icon_y + SHEET_ICON + 4.0;
    let th = 11.0 + pad_y * 2.0;
    outlined(px, tx, ty, tw, th, 6.0, night_ink());
    text(
        px,
        fonts,
        label,
        11.0,
        tx + tw * 0.5,
        ty + pad_y,
        text_col(),
        true,
    );
}

fn icon_hit(
    px: &mut Pixmap,
    x: f32,
    y: f32,
    hit: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    kind: IconKind,
    on: bool,
) -> Option<(f32, f32, &'static str)> {
    hits.push(HitBox {
        id: hit,
        x,
        y,
        w: SHEET_ICON,
        h: SHEET_ICON,
    });
    if hover == Some(hit) {
        fill_round(
            px,
            x,
            y,
            SHEET_ICON,
            SHEET_ICON,
            8.0,
            Color::from_rgba8(255, 255, 255, 14),
        );
    }
    let cx = x + SHEET_ICON * 0.5;
    let cy = y + SHEET_ICON * 0.5;
    match kind {
        IconKind::Back | IconKind::Trash => {
            let c = if hover == Some(hit) {
                text_col()
            } else {
                muted()
            };
            if kind == IconKind::Back {
                draw_back_icon(px, cx, cy, c);
            } else {
                draw_trash_icon(px, cx, cy, c);
            }
        }
        IconKind::Bookmark => {
            if on {
                draw_bookmark_icon(px, cx, cy, true, accent());
            } else {
                draw_bookmark_icon(
                    px,
                    cx,
                    cy,
                    false,
                    if hover == Some(hit) {
                        text_col()
                    } else {
                        muted()
                    },
                );
            }
        }
    }
    if hover == Some(hit) {
        Some((x, y, icon_tip_label(kind, on)))
    } else {
        None
    }
}

fn draw_back_icon(px: &mut Pixmap, cx: f32, cy: f32, c: Color) {
    icon_stroke_line(px, cx + 4.0, cy - 6.0, cx - 4.5, cy, c, 1.5);
    icon_stroke_line(px, cx - 4.5, cy, cx + 4.0, cy + 6.0, c, 1.5);
}

fn draw_trash_icon(px: &mut Pixmap, cx: f32, cy: f32, c: Color) {
    icon_stroke_line(px, cx - 6.5, cy - 3.2, cx + 6.5, cy - 3.2, c, 1.5);
    icon_stroke_line(px, cx - 2.2, cy - 6.2, cx + 2.2, cy - 6.2, c, 1.5);
    icon_stroke_line(px, cx - 2.2, cy - 6.2, cx - 2.2, cy - 3.2, c, 1.5);
    icon_stroke_line(px, cx + 2.2, cy - 6.2, cx + 2.2, cy - 3.2, c, 1.5);
    let mut pb = PathBuilder::new();
    pb.move_to(cx - 5.0, cy - 3.2);
    pb.line_to(cx - 3.6, cy + 7.2);
    pb.line_to(cx + 3.6, cy + 7.2);
    pb.line_to(cx + 5.0, cy - 3.2);
    if let Some(path) = pb.finish() {
        icon_stroke(px, &path, c, 1.5);
    }
}

fn crown_gold() -> Color {
    Color::from_rgba8(255, 196, 48, 255)
}

fn draw_win_crown(px: &mut Pixmap, fonts: &Fonts, x: f32, y: f32, size: f32) {
    icon(px, fonts, '\u{f521}', size, x, y, crown_gold(), false);
}

fn draw_bookmark_icon(px: &mut Pixmap, cx: f32, cy: f32, filled: bool, c: Color) {
    let mut pb = PathBuilder::new();
    pb.move_to(cx - 4.6, cy - 7.0);
    pb.line_to(cx + 4.6, cy - 7.0);
    pb.line_to(cx + 4.6, cy + 7.2);
    pb.line_to(cx, cy + 3.0);
    pb.line_to(cx - 4.6, cy + 7.2);
    pb.close();
    let Some(path) = pb.finish() else {
        return;
    };
    if filled {
        let mut fill = Paint::default();
        fill.set_color(c);
        fill.anti_alias = true;
        px.fill_path(&path, &fill, FillRule::Winding, Transform::identity(), None);
    } else {
        icon_stroke(px, &path, c, 1.5);
    }
}

fn filter_chip(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    label: &str,
    on: bool,
    hit: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let bw = 64.0;
    hits.push(HitBox {
        id: hit,
        x,
        y,
        w: bw,
        h: 28.0,
    });
    if on {
        fill_skew(px, x, y, 58.0, 28.0, 6.0, accent());
        text(px, fonts, label, 13.0, x + 14.0, y + 6.0, ink(), false);
    } else {
        if hover == Some(hit) {
            fill_round(
                px,
                x,
                y,
                bw,
                28.0,
                8.0,
                Color::from_rgba8(255, 255, 255, 10),
            );
        }
        text(px, fonts, label, 13.0, x + 14.0, y + 6.0, text_col(), false);
    }
    28.0
}

fn draw_scrubber(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    scrub: f32,
) {
    hits.push(HitBox {
        id: Hit::AnalyzeScrub,
        x,
        y,
        w,
        h: 22.0,
    });
    fill_round(
        px,
        x,
        y + 8.0,
        w,
        4.0,
        2.0,
        Color::from_rgba8(42, 42, 46, 255),
    );
    let t = scrub.clamp(0.0, 1.0);
    fill_round(px, x, y + 8.0, (w * t).max(4.0), 4.0, 2.0, accent());
    fill_circle(
        px,
        x + w * t,
        y + 10.0,
        6.0,
        if hover == Some(Hit::AnalyzeScrub) {
            accent()
        } else {
            super::knob()
        },
    );
    let _ = fonts;
}

fn draw_analyze_map(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    detail: &SessionDetail,
    compare: i32,
    you_sel: i32,
    warmup: bool,
    scrub: f32,
    zoom: f32,
    pan_x: f32,
    pan_z: f32,
    follow: bool,
) {
    fill_round(px, x, y, w, h, 6.0, night_ink());
    let pts = if detail.poly.len() >= 2 {
        detail.poly.clone()
    } else {
        return;
    };
    let mut min_x = pts[0].0;
    let mut max_x = min_x;
    let mut min_z = pts[0].1;
    let mut max_z = min_z;
    for p in &pts {
        min_x = min_x.min(p.0);
        max_x = max_x.max(p.0);
        min_z = min_z.min(p.1);
        max_z = max_z.max(p.1);
    }
    let (pan_x, pan_z) = if follow && zoom > 1.0 {
        pick_you_lap(detail, you_sel, warmup)
            .and_then(|lap| follow_pan(&pts, &lap.line, scrub))
            .unwrap_or((pan_x, pan_z))
    } else {
        (pan_x, pan_z)
    };
    let z = zoom.max(1.0);
    let dx = (max_x - min_x).max(8.0) / z;
    let dz = (max_z - min_z).max(8.0) / z;
    let cx = (min_x + max_x) * 0.5 + pan_x;
    let cz = (min_z + max_z) * 0.5 + pan_z;
    let pad = 22.0;
    let sx = (w - pad * 2.0) / dx;
    let sz = (h - pad * 2.0) / dz;
    let scale = sx.min(sz);
    let (mw, mh) = (w.round().max(1.0) as u32, h.round().max(1.0) as u32);
    let starts = analyze_sector_starts(mxbo_hud::track_pb::peek_split_milli(&detail.row.track));
    let key = AnalyzeTrackKey {
        id: detail.row.id,
        mw,
        mh,
        zoom: z.to_bits(),
        pan_x: pan_x.to_bits(),
        pan_z: pan_z.to_bits(),
        sf_m: if detail.sf_meters.is_finite() {
            detail.sf_meters.round() as i32
        } else {
            i32::MIN
        },
        s2_m: (starts[1].0 * 1000.0).round() as i32,
        s3_m: (starts[2].0 * 1000.0).round() as i32,
    };
    let to_local = |wx: f32, wz: f32| -> (f32, f32) {
        (w * 0.5 + (wx - cx) * scale, h * 0.5 - (wz - cz) * scale)
    };
    let track_px = analyze_track_px(scale);
    let clip = well_mask(mw, mh);
    ANALYZE_TRACK.with(|slot| {
        let mut slot = slot.borrow_mut();
        let stale = slot.as_ref().map(|(k, _)| *k != key).unwrap_or(true);
        let had_layer = slot.is_some();
        if stale {
            let pending_same = ANALYZE_TRACK_PENDING.with(|pending| {
                let mut pending = pending.borrow_mut();
                let same = *pending == Some(key);
                if same || had_layer {
                    *pending = None;
                } else {
                    *pending = Some(key);
                }
                same
            });
            if bake_analyze_track(stale, pending_same, had_layer) {
                if let Some(mut layer) = Pixmap::new(mw, mh) {
                    layer.fill(Color::TRANSPARENT);
                    fill_track_ribbon(&mut layer, &pts, &to_local, track_px, clip.as_ref());
                    draw_analyze_sf(
                        &mut layer,
                        &pts,
                        detail.sf_meters,
                        &to_local,
                        track_px,
                        clip.as_ref(),
                    );
                    draw_analyze_sectors(
                        &mut layer,
                        fonts,
                        &pts,
                        detail.sf_meters,
                        starts,
                        &to_local,
                        track_px,
                        clip.as_ref(),
                    );
                    *slot = Some((key, layer));
                }
            }
        } else {
            ANALYZE_TRACK_PENDING.with(|pending| *pending.borrow_mut() = None);
        }
    });
    ANALYZE_INK.with(|slot| {
        let mut slot = slot.borrow_mut();
        let reuse = slot
            .as_ref()
            .is_some_and(|(iw, ih, _)| *iw == mw && *ih == mh);
        if !reuse {
            *slot = Pixmap::new(mw, mh).map(|p| (mw, mh, p));
        }
        let Some((_, _, ink)) = slot.as_mut() else {
            return;
        };
        ink.fill(Color::TRANSPARENT);
        ANALYZE_TRACK.with(|track| {
            if let Some((_, layer)) = track.borrow().as_ref() {
                let _ = ink.draw_pixmap(
                    0,
                    0,
                    layer.as_ref(),
                    &PixmapPaint::default(),
                    Transform::identity(),
                    None,
                );
            }
        });
        let other = pick_compare(detail, compare);
        if let Some(lap) = compare_lap(detail, other) {
            let line: Vec<(f32, f32)> = lap.line.iter().map(|(lx, _, lz)| (*lx, *lz)).collect();
            stroke_world(
                ink,
                &line,
                &to_local,
                Color::from_rgba8(138, 142, 154, 255),
                2.4,
                false,
                clip.as_ref(),
            );
            let you_ms = pick_you_lap(detail, you_sel, warmup)
                .map(|you| bin_at(you, scrub).ms)
                .unwrap_or(0);
            let other_pos = if you_ms > 0 {
                pos_at_ms(lap, you_ms)
            } else {
                scrub
            };
            if let Some((lx, _, lz)) = point_at(&lap.line, other_pos) {
                let (hx, hy) = to_local(lx, lz);
                if in_well(hx, hy, w, h) {
                    fill_circle(ink, hx, hy, 5.0, Color::from_rgba8(138, 142, 154, 255));
                }
            }
        }
        if let Some(lap) = pick_you_lap(detail, you_sel, warmup) {
            let line: Vec<(f32, f32)> = lap.line.iter().map(|(lx, _, lz)| (*lx, *lz)).collect();
            stroke_you_line(ink, &line, detail, lap, &to_local, clip.as_ref());
            if let Some((lx, _, lz)) = point_at(&lap.line, scrub) {
                let (hx, hy) = to_local(lx, lz);
                if in_well(hx, hy, w, h) {
                    fill_circle(ink, hx, hy, 5.0, accent());
                }
            }
            let marks: Vec<(f32, f32, &str)> = lap
                .crashes
                .iter()
                .map(|m| {
                    let (hx, hy) = to_local(m.x, m.z);
                    (hx, hy, m.contact_name.as_str())
                })
                .collect();
            for (hx, hy, label) in cluster_crash_marks(&marks) {
                if !in_well(hx, hy, w, h) {
                    continue;
                }
                draw_crash_triangle(ink, hx, hy);
                if !label.is_empty() && in_well(hx + 8.0, hy - 4.0, w, h) {
                    text(
                        ink,
                        fonts,
                        &label,
                        10.0,
                        hx + 8.0,
                        hy - 4.0,
                        Color::from_rgba8(232, 56, 56, 255),
                        false,
                    );
                }
            }
        }
        let _ = px.draw_pixmap(
            x.round() as i32,
            y.round() as i32,
            ink.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            None,
        );
    });
    text(
        px,
        fonts,
        if follow && zoom > 1.0 {
            "Follow on · drag to release"
        } else {
            "Scroll to zoom · drag to pan"
        },
        10.0,
        x + 12.0,
        y + h - 18.0,
        dim(),
        false,
    );
}

fn analyze_track_px(scale: f32) -> f32 {
    (16.0 * scale).max(12.0)
}

fn analyze_cross_half(track_px: f32) -> f32 {
    track_px * 0.5
}

fn chart_bin_value(
    lap: &crate::review::LapView,
    i: usize,
    b: &mxbo_hud::location_tape::ChannelBin,
    pick: fn(&mxbo_hud::location_tape::ChannelBin) -> f32,
    infer_speed: bool,
    infer_throttle: bool,
) -> f32 {
    if infer_speed {
        tape_speed(lap, i)
    } else if infer_throttle {
        tape_throttle(lap, i)
    } else {
        pick(b)
    }
}

fn chart_range(
    you: Option<&crate::review::LapView>,
    other: Option<&crate::review::LapView>,
    pick: fn(&mxbo_hud::location_tape::ChannelBin) -> f32,
    win_start: f32,
    win_span: f32,
    infer_speed: bool,
    infer_throttle: bool,
) -> Option<(f32, f32)> {
    let last = (BINS_CHART - 1) as f32;
    let win_end = win_start + win_span;
    let mut min = f32::MAX;
    let mut max = f32::MIN;
    for lap in [you, other].into_iter().flatten() {
        for (i, b) in lap.bins.iter().enumerate() {
            let t = i as f32 / last;
            if t < win_start || t > win_end {
                continue;
            }
            let v = chart_bin_value(lap, i, b, pick, infer_speed, infer_throttle);
            if v == 0.0 {
                continue;
            }
            min = min.min(v);
            max = max.max(v);
        }
    }
    if !min.is_finite() || (max - min).abs() < 0.001 {
        return None;
    }
    Some((min, max))
}

fn well_mask(mw: u32, mh: u32) -> Option<Mask> {
    let path = round_path(0.0, 0.0, mw as f32, mh as f32, 6.0)?;
    let mut m = Mask::new(mw, mh)?;
    m.fill_path(&path, FillRule::Winding, true, Transform::identity());
    Some(m)
}

fn in_well(lx: f32, ly: f32, w: f32, h: f32) -> bool {
    lx >= 0.0 && ly >= 0.0 && lx <= w && ly <= h
}

fn bake_analyze_track(stale: bool, pending_same: bool, had_layer: bool) -> bool {
    stale && (had_layer || pending_same)
}

fn fill_track_ribbon(
    px: &mut Pixmap,
    pts: &[(f32, f32)],
    to_px: &dyn Fn(f32, f32) -> (f32, f32),
    track_px: f32,
    clip: Option<&Mask>,
) {
    if pts.len() < 2 {
        return;
    }
    let mapped: Vec<(f32, f32)> = pts.iter().map(|p| to_px(p.0, p.1)).collect();
    let pts = thin_world_pts(&mapped, 4.0);
    let pts = smooth_poly_pts(&pts, true);
    let Some(path) = cubic_poly_path(&pts, true) else {
        return;
    };
    let mut fill = Paint::default();
    fill.set_color(Color::from_rgba8(10, 8, 8, 168));
    fill.anti_alias = true;
    px.fill_path(&path, &fill, FillRule::EvenOdd, Transform::identity(), clip);
    let stroke = |px: &mut Pixmap, col: Color, width: f32| {
        let mut paint = Paint::default();
        paint.set_color(col);
        paint.anti_alias = true;
        px.stroke_path(
            &path,
            &paint,
            &Stroke {
                width,
                line_cap: LineCap::Round,
                line_join: LineJoin::Round,
                ..Stroke::default()
            },
            Transform::identity(),
            clip,
        );
    };
    stroke(px, Color::from_rgba8(18, 16, 16, 240), track_px + 3.0);
    stroke(px, Color::from_rgba8(236, 236, 240, 255), track_px);
}

fn thin_world_pts(pts: &[(f32, f32)], min_d2: f32) -> Vec<(f32, f32)> {
    if pts.len() <= 2 {
        return pts.to_vec();
    }
    let mut out = Vec::with_capacity(64);
    out.push(pts[0]);
    let last = pts[pts.len() - 1];
    for &p in &pts[1..pts.len() - 1] {
        let prev = out[out.len() - 1];
        let dx = p.0 - prev.0;
        let dy = p.1 - prev.1;
        if dx * dx + dy * dy >= min_d2 {
            out.push(p);
        }
    }
    if out.last() != Some(&last) {
        out.push(last);
    }
    out
}

fn smooth_poly_pts(pts: &[(f32, f32)], close: bool) -> Vec<(f32, f32)> {
    if pts.len() < 3 {
        return pts.to_vec();
    }
    let n = pts.len();
    let at = |j: i32| -> (f32, f32) {
        if close {
            pts[j.rem_euclid(n as i32) as usize]
        } else {
            pts[j.clamp(0, n as i32 - 1) as usize]
        }
    };
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        if !close && (i == 0 || i == n - 1) {
            out.push(pts[i]);
            continue;
        }
        let t = i as i32;
        let a = at(t - 2);
        let b = at(t - 1);
        let c = at(t);
        let d = at(t + 1);
        let e = at(t + 2);
        out.push((
            (a.0 + b.0 * 2.0 + c.0 * 3.0 + d.0 * 2.0 + e.0) / 9.0,
            (a.1 + b.1 * 2.0 + c.1 * 3.0 + d.1 * 2.0 + e.1) / 9.0,
        ));
    }
    out
}

fn cubic_poly_path(pts: &[(f32, f32)], close: bool) -> Option<Path> {
    if pts.len() < 2 {
        return None;
    }
    let n = pts.len();
    let at = |i: isize| -> (f32, f32) {
        if close {
            pts[i.rem_euclid(n as isize) as usize]
        } else {
            pts[i.clamp(0, n as isize - 1) as usize]
        }
    };
    let mut pb = PathBuilder::new();
    pb.move_to(pts[0].0, pts[0].1);
    if n == 2 {
        pb.line_to(pts[1].0, pts[1].1);
        if close {
            pb.close();
        }
        return pb.finish();
    }
    let last = if close { n } else { n - 1 };
    for i in 0..last {
        let p0 = at(i as isize - 1);
        let p1 = at(i as isize);
        let p2 = at(i as isize + 1);
        let p3 = at(i as isize + 2);
        let c1x = p1.0 + (p2.0 - p0.0) / 6.0;
        let c1y = p1.1 + (p2.1 - p0.1) / 6.0;
        let c2x = p2.0 - (p3.0 - p1.0) / 6.0;
        let c2y = p2.1 - (p3.1 - p1.1) / 6.0;
        pb.cubic_to(c1x, c1y, c2x, c2y, p2.0, p2.1);
    }
    if close {
        pb.close();
    }
    pb.finish()
}

fn stroke_you_line(
    px: &mut Pixmap,
    line: &[(f32, f32)],
    detail: &SessionDetail,
    lap: &crate::review::LapView,
    to_px: &dyn Fn(f32, f32) -> (f32, f32),
    clip: Option<&Mask>,
) {
    let stroke = |px: &mut Pixmap, pts: &[(f32, f32)], col: Color| {
        stroke_world(px, pts, to_px, col, 2.6, false, clip);
    };
    if let Some(i) = you_holeshot_split(detail, lap) {
        if i + 1 < line.len() {
            stroke(px, &line[i..], accent());
            stroke(px, &line[..=i], best_violet());
            return;
        }
        stroke(px, line, best_violet());
        return;
    }
    stroke(px, line, accent());
}

fn stroke_world(
    px: &mut Pixmap,
    pts: &[(f32, f32)],
    to_px: &dyn Fn(f32, f32) -> (f32, f32),
    col: Color,
    width: f32,
    close: bool,
    clip: Option<&Mask>,
) {
    if pts.len() < 2 {
        return;
    }
    let mapped: Vec<(f32, f32)> = pts.iter().map(|p| to_px(p.0, p.1)).collect();
    let pts = thin_world_pts(&mapped, 1.0);
    let pts = smooth_poly_pts(&pts, close);
    let Some(path) = cubic_poly_path(&pts, close) else {
        return;
    };
    let mut paint = Paint::default();
    paint.set_color(col);
    paint.anti_alias = true;
    let stroke = Stroke {
        width,
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Stroke::default()
    };
    px.stroke_path(&path, &paint, &stroke, Transform::identity(), clip);
}

fn draw_spot_stats(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    detail: &SessionDetail,
    picked: Option<&crate::review::LapView>,
    other: i32,
    other_lap: Option<&crate::review::LapView>,
    pos: f32,
    units: Units,
) -> f32 {
    let mut y = section(px, fonts, x, y, "At this spot");
    if let (Some(a), Some(b)) = (picked, other_lap) {
        y = draw_spot_deltas(px, fonts, x, y, w, detail, a, b, other, pos, units);
    }
    y
}

fn draw_spot_charts(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    picked: Option<&crate::review::LapView>,
    other_lap: Option<&crate::review::LapView>,
    pos: f32,
    follow: bool,
    zoom: f32,
) {
    fill_round(px, x, y, w, h, 6.0, night_ink());
    let (win_start, win_span) = if follow && zoom > 1.0 {
        chart_window(pos, zoom)
    } else {
        (0.0, 1.0)
    };
    let rows = 3.0;
    let pad = 8.0;
    let inner = (h - pad * 2.0).max(48.0);
    let bar_h = ((inner / rows) - 12.0).max(16.0);
    let cx = x + pad;
    let cw = (w - pad * 2.0).max(80.0);
    let mut y = y + pad;
    y = chart_row(
        px,
        fonts,
        cx,
        y,
        cw,
        "SPEED",
        picked,
        other_lap,
        |b| b.speed,
        pos,
        bar_h,
        win_start,
        win_span,
        None,
        true,
        false,
    );
    y = chart_row(
        px,
        fonts,
        cx,
        y,
        cw,
        "HEIGHT",
        picked,
        other_lap,
        |b| b.y,
        pos,
        bar_h,
        win_start,
        win_span,
        None,
        false,
        false,
    );
    let _ = chart_row(
        px,
        fonts,
        cx,
        y,
        cw,
        "THR",
        picked,
        other_lap,
        |b| b.throttle,
        pos,
        bar_h,
        win_start,
        win_span,
        Some((0.0, 1.0)),
        false,
        true,
    );
}

fn draw_spot_deltas(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    detail: &SessionDetail,
    you: &crate::review::LapView,
    other_lap: &crate::review::LapView,
    other: i32,
    pos: f32,
    units: Units,
) -> f32 {
    let ia = bin_at(you, pos);
    let ib = bin_at(other_lap, pos);
    let i = bin_index(pos);
    let you_spd = tape_speed(you, i);
    let them_spd = tape_speed(other_lap, i);
    let you_t = fmt_lap(you.lap_ms);
    let them_t = fmt_lap(other_lap.lap_ms);
    let them_name = rider_name(detail, other);
    text(px, fonts, "YOU", 10.0, x + 4.0, y + 1.0, dim(), false);
    text(
        px,
        fonts,
        &you_t,
        12.0,
        x + 4.0 + measure(fonts, "YOU", 10.0) + 6.0,
        y,
        text_col(),
        false,
    );
    let them_lab = format!("{them_name}  {them_t}");
    let tw = measure(fonts, &them_lab, 12.0);
    text(
        px,
        fonts,
        &them_lab,
        12.0,
        x + w - tw - 4.0,
        y,
        muted(),
        false,
    );
    let mut y = y + 18.0;
    let ds = you_spd - them_spd;
    let dy = ia.y - ib.y;
    let col_w = (w / 3.0).max(70.0);
    let them_spd_empty = ib.ms <= 0 && them_spd == 0.0;
    let you_spd_col = if them_spd_empty {
        text_col()
    } else {
        delta_col(ds, false)
    };
    draw_pair_cell(
        px,
        fonts,
        x + 4.0,
        y,
        "SPD",
        &spot_speed(you_spd, ia.ms, units),
        &spot_speed(them_spd, ib.ms, units),
        you_spd_col,
    );
    draw_pair_cell(
        px,
        fonts,
        x + col_w,
        y,
        "GEAR",
        &spot_gear(ia.gear),
        &spot_gear(tape_gear(you, other_lap, i)),
        text_col(),
    );
    draw_pair_cell(
        px,
        fonts,
        x + col_w * 2.0,
        y,
        "THR",
        &spot_throttle(ia.throttle, ia.ms, true),
        &spot_throttle(
            tape_throttle(other_lap, i),
            ib.ms,
            tape_throttle_ok(other_lap, i),
        ),
        text_col(),
    );
    y += 18.0;
    draw_delta_cell(
        px,
        fonts,
        x + 4.0,
        y,
        "HT",
        &format!("{dy:+.2}"),
        delta_col(dy, true),
    );
    draw_delta_cell(
        px,
        fonts,
        x + col_w,
        y,
        "RPM",
        &spot_rpm_delta(ia.rpm, ib.rpm, tape_has_rpm(other_lap)),
        text_col(),
    );
    y += 18.0;
    y
}

fn draw_delta_cell(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    label: &str,
    value: &str,
    col: Color,
) {
    text(px, fonts, label, 10.0, x, y, dim(), false);
    let lw = measure(fonts, label, 10.0);
    text(px, fonts, value, 12.0, x + lw + 6.0, y, col, false);
}

fn draw_pair_cell(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    label: &str,
    you: &str,
    them: &str,
    you_col: Color,
) {
    text(px, fonts, label, 10.0, x, y, dim(), false);
    let lx = x + measure(fonts, label, 10.0) + 6.0;
    text(px, fonts, you, 12.0, lx, y, you_col, false);
    let sep = " | ";
    let mx = lx + measure(fonts, you, 12.0);
    text(px, fonts, sep, 12.0, mx, y, muted(), false);
    text(
        px,
        fonts,
        them,
        12.0,
        mx + measure(fonts, sep, 12.0),
        y,
        muted(),
        false,
    );
}

#[cfg(test)]
fn spot_speed_delta(ds_mps: f32, units: Units) -> String {
    let (n, unit) = match units {
        Units::Imperial => (ds_mps * 2.236936, "mph"),
        Units::Metric => (ds_mps * 3.6, "kph"),
    };
    format!("{n:+.1} {unit}")
}

fn spot_speed(mps: f32, ms: i32, units: Units) -> String {
    if ms <= 0 && mps == 0.0 {
        return "—".into();
    }
    let (n, unit) = match units {
        Units::Imperial => (mps * 2.236936, "mph"),
        Units::Metric => (mps * 3.6, "kph"),
    };
    format!("{n:.1} {unit}")
}

fn spot_gear(gear: i32) -> String {
    if gear == 0 {
        "—".into()
    } else {
        format!("{gear}")
    }
}

fn spot_throttle(throttle: f32, ms: i32, has_signal: bool) -> String {
    if !has_signal || (ms <= 0 && throttle == 0.0) {
        "—".into()
    } else {
        format!("{throttle:.2}")
    }
}

fn spot_rpm_delta(you_rpm: f32, them_rpm: f32, them_has_rpm: bool) -> String {
    if !them_has_rpm {
        return "—".into();
    }
    format!("{:+.0}", you_rpm - them_rpm)
}

fn line_len(line: &[(f32, f32, f32)]) -> f32 {
    line.windows(2)
        .map(|w| {
            let dx = w[1].0 - w[0].0;
            let dy = w[1].1 - w[0].1;
            let dz = w[1].2 - w[0].2;
            (dx * dx + dy * dy + dz * dz).sqrt()
        })
        .sum()
}

fn inferred_speed(lap: &crate::review::LapView, i: usize) -> Option<f32> {
    let len = line_len(&lap.line);
    if len <= 0.0 || i >= lap.bins.len() {
        return None;
    }
    let cur_ms = lap.bins[i].ms;
    if cur_ms <= 0 {
        return None;
    }
    let prev = (0..i).rev().find(|&j| lap.bins[j].ms > 0)?;
    let dt = (cur_ms - lap.bins[prev].ms) as f32 / 1000.0;
    if dt <= 0.001 {
        return None;
    }
    let di = (i - prev) as f32;
    Some((len / BINS_CHART as f32) * di / dt)
}

fn tape_speed(lap: &crate::review::LapView, i: usize) -> f32 {
    let Some(b) = lap.bins.get(i) else {
        return 0.0;
    };
    if b.speed != 0.0 {
        b.speed
    } else {
        inferred_speed(lap, i).unwrap_or(0.0)
    }
}

fn implied_throttle(lap: &crate::review::LapView, i: usize) -> Option<f32> {
    let v1 = tape_speed(lap, i);
    let prev = (0..i).rev().find(|&j| lap.bins[j].ms > 0)?;
    let v0 = tape_speed(lap, prev);
    let dt = (lap.bins[i].ms - lap.bins[prev].ms) as f32 / 1000.0;
    if dt <= 0.001 {
        return None;
    }
    let a = (v1 - v0) / dt;
    Some((a / 6.0 + 0.4).clamp(0.0, 1.0))
}

fn tape_throttle(lap: &crate::review::LapView, i: usize) -> f32 {
    if tape_has_throttle(lap) {
        lap.bins.get(i).map(|b| b.throttle).unwrap_or(0.0)
    } else {
        implied_throttle(lap, i).unwrap_or(0.0)
    }
}

fn tape_throttle_ok(lap: &crate::review::LapView, i: usize) -> bool {
    tape_has_throttle(lap) || implied_throttle(lap, i).is_some()
}

fn implied_gear(ref_lap: &crate::review::LapView, speed: f32) -> i32 {
    if speed <= 0.0 {
        return 0;
    }
    let mut best_g = 0;
    let mut best_d = f32::MAX;
    for g in 1..=6 {
        let mut sum = 0.0f32;
        let mut n = 0;
        for b in &ref_lap.bins {
            if b.gear == g && b.speed > 0.0 {
                sum += b.speed;
                n += 1;
            }
        }
        if n == 0 {
            continue;
        }
        let mean = sum / n as f32;
        let d = (mean - speed).abs();
        if d < best_d {
            best_d = d;
            best_g = g;
        }
    }
    best_g
}

fn tape_gear(you: &crate::review::LapView, other: &crate::review::LapView, i: usize) -> i32 {
    if tape_has_gear(other) {
        other.bins.get(i).map(|b| b.gear).unwrap_or(0)
    } else {
        implied_gear(you, tape_speed(other, i))
    }
}

fn tape_has_gear(lap: &crate::review::LapView) -> bool {
    lap.bins.iter().any(|b| b.gear != 0)
}

fn tape_has_throttle(lap: &crate::review::LapView) -> bool {
    lap.bins.iter().any(|b| b.throttle != 0.0)
}

fn tape_has_rpm(lap: &crate::review::LapView) -> bool {
    lap.bins.iter().any(|b| b.rpm != 0.0)
}

fn bin_index(pos: f32) -> usize {
    ((pos.clamp(0.0, 0.999) * BINS_CHART as f32) as usize).min(BINS_CHART - 1)
}

fn draw_results(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    detail: &SessionDetail,
) -> f32 {
    let p_w = 36.0;
    let best_w = 76.0;
    let last_w = 76.0;
    let class_w = 48.0;
    let name_w = 168.0f32.min((w * 0.28).max(96.0));
    let row_h = 24.0;
    let pad = 8.0;
    let table_w = (pad + p_w + name_w + best_w + last_w + class_w + pad).min(w);
    text(px, fonts, "P", 10.0, x + pad, y, dim(), false);
    text(px, fonts, "NAME", 10.0, x + pad + p_w, y, dim(), false);
    let best_x = x + pad + p_w + name_w;
    let last_x = best_x + best_w;
    let class_x = last_x + last_w;
    text(px, fonts, "BEST", 10.0, best_x, y, dim(), false);
    text(px, fonts, "LAST", 10.0, last_x, y, dim(), false);
    text(px, fonts, "CLASS", 10.0, class_x, y, dim(), false);
    let mut y = y + 16.0;
    let session_best = detail
        .riders
        .iter()
        .filter(|r| r.best_ms > 0)
        .map(|r| r.best_ms)
        .min();
    for r in &detail.riders {
        let you = r.race_num == detail.your_race_num;
        if you {
            fill_round(px, x, y, table_w, row_h, 6.0, you_row());
        }
        let ink = if you { text_col() } else { muted() };
        let best = fmt_lap(r.best_ms);
        let last = fmt_lap(r.last_ms);
        let class = rider_class(&r.bike);
        let class = if class.is_empty() {
            "—".into()
        } else {
            class
        };
        let name = ellipsize_heading(fonts, &r.name, 12.0, name_w - 4.0);
        text(
            px,
            fonts,
            &format!("P{}", r.position.max(1)),
            12.0,
            x + pad,
            y + 5.0,
            ink,
            false,
        );
        text(px, fonts, &name, 12.0, x + pad + p_w, y + 5.0, ink, false);
        let best_col = if session_best == Some(r.best_ms) && r.best_ms > 0 {
            best_violet()
        } else {
            ink
        };
        text(px, fonts, &best, 12.0, best_x, y + 5.0, best_col, false);
        text(px, fonts, &last, 12.0, last_x, y + 5.0, ink, false);
        text(px, fonts, &class, 12.0, class_x, y + 5.0, ink, false);
        y += row_h;
    }
    y
}

fn compare_rider_label(r: &RiderRow) -> String {
    let time = fmt_lap(r.best_ms);
    let cc = mxbo_hud::track_pb::bike_class(&r.bike);
    if !cc.is_empty() && cc.chars().all(|c| c.is_ascii_digit()) {
        format!("{} | {} | {}", r.name, cc, time)
    } else {
        format!("{} | {}", r.name, time)
    }
}

fn compare_line_lap(detail: &SessionDetail, race_num: i32) -> Option<&crate::review::LapView> {
    detail
        .laps
        .iter()
        .filter(|l| l.race_num == race_num && !l.crashed && l.line.len() >= 2 && l.lap_ms > 0)
        .min_by_key(|l| l.lap_ms)
}

fn compare_lap(detail: &SessionDetail, race_num: i32) -> Option<&crate::review::LapView> {
    best_lap(detail, race_num).or_else(|| compare_line_lap(detail, race_num))
}

fn compare_choices(detail: &SessionDetail) -> Vec<&RiderRow> {
    detail
        .riders
        .iter()
        .filter(|r| r.race_num != detail.your_race_num && r.has_line)
        .collect()
}

fn pick_compare(detail: &SessionDetail, compare: i32) -> i32 {
    let choices = compare_choices(detail);
    if compare > 0 && choices.iter().any(|r| r.race_num == compare) {
        return compare;
    }
    if choices
        .iter()
        .any(|r| r.race_num == detail.fastest_race_num)
    {
        return detail.fastest_race_num;
    }
    choices
        .first()
        .map(|r| r.race_num)
        .unwrap_or(detail.fastest_race_num)
}

fn draw_compare_drop(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    detail: &SessionDetail,
    other: i32,
    open_drop: Option<Drop>,
) -> f32 {
    let choices = compare_choices(detail);
    let label = choices
        .iter()
        .find(|r| r.race_num == other)
        .map(|r| compare_rider_label(r))
        .unwrap_or_else(|| rider_name(detail, other));
    let h = 32.0;
    let bw = choices
        .iter()
        .map(|r| measure(fonts, &compare_rider_label(r), 12.0) + 40.0)
        .fold(160.0_f32, f32::max)
        .min(w)
        .max(160.0);
    let open = open_drop == Some(Drop::AnalyzeCompare);
    let hit = Hit::AnalyzeCompareOpen;
    hits.push(HitBox {
        id: hit,
        x,
        y,
        w: bw,
        h,
    });
    fill_round(
        px,
        x,
        y,
        bw,
        h,
        6.0,
        if open || hover == Some(hit) {
            Color::from_rgba8(20, 20, 22, 255)
        } else {
            night_ink()
        },
    );
    text(
        px,
        fonts,
        &label,
        12.0,
        x + 10.0,
        y + 8.0,
        text_col(),
        false,
    );
    chevron(px, x + bw - 14.0, y + h * 0.5, open, muted());
    if open && !choices.is_empty() {
        let options: Vec<(Hit, String, bool)> = choices
            .iter()
            .map(|r| {
                (
                    Hit::AnalyzeCompare(r.race_num),
                    compare_rider_label(r),
                    r.race_num == other,
                )
            })
            .collect();
        DROP_MENUS.with(|menus| {
            menus.borrow_mut().push(PendingDrop {
                mx: x,
                my: y + h + 6.0,
                bw,
                content_h: 10.0 + 28.0 * options.len() as f32,
                open_hit: hit,
                options,
            });
        });
    }
    y + h + 8.0
}

fn chart_row(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    you: Option<&crate::review::LapView>,
    other: Option<&crate::review::LapView>,
    pick: fn(&mxbo_hud::location_tape::ChannelBin) -> f32,
    pos: f32,
    bar_h: f32,
    win_start: f32,
    win_span: f32,
    range: Option<(f32, f32)>,
    infer_speed: bool,
    infer_throttle: bool,
) -> f32 {
    let bar_h = bar_h.max(16.0);
    text(px, fonts, label, 10.0, x + 2.0, y, dim(), false);
    let gx = x + 72.0;
    let gw = (w - 76.0).max(80.0);
    fill_round(px, gx, y + 4.0, gw, bar_h, 4.0, well());
    let lw = gw.round().max(1.0) as u32;
    let lh = bar_h.round().max(1.0) as u32;
    if let Some(mut layer) = Pixmap::new(lw, lh) {
        layer.fill(Color::TRANSPARENT);
        let include_zero = range.is_some();
        let shared = range.or_else(|| {
            chart_range(
                you,
                other,
                pick,
                win_start,
                win_span,
                infer_speed,
                infer_throttle,
            )
        });
        if let Some(lap) = other {
            stroke_bins(
                &mut layer,
                0.0,
                0.0,
                gw,
                bar_h,
                lap,
                pick,
                Color::from_rgba8(138, 142, 154, 255),
                win_start,
                win_span,
                shared,
                infer_speed,
                infer_throttle,
                include_zero,
            );
        }
        if let Some(lap) = you {
            stroke_bins(
                &mut layer,
                0.0,
                0.0,
                gw,
                bar_h,
                lap,
                pick,
                accent(),
                win_start,
                win_span,
                shared,
                infer_speed,
                infer_throttle,
                include_zero,
            );
        }
        let _ = px.draw_pixmap(
            gx.round() as i32,
            (y + 4.0).round() as i32,
            layer.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            None,
        );
    }
    let t = ((pos.clamp(0.0, 1.0) - win_start) / win_span.max(0.001)).clamp(0.0, 1.0);
    let tx = gx + gw * t;
    fill_rect(
        px,
        Rect::from_xywh(tx, y + 4.0, 1.0, bar_h)
            .unwrap_or(Rect::from_ltrb(0.0, 0.0, 1.0, 1.0).unwrap()),
        text_col(),
    );
    y + bar_h + 12.0
}

fn stroke_bins(
    px: &mut Pixmap,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    lap: &crate::review::LapView,
    pick: fn(&mxbo_hud::location_tape::ChannelBin) -> f32,
    col: Color,
    win_start: f32,
    win_span: f32,
    range: Option<(f32, f32)>,
    infer_speed: bool,
    infer_throttle: bool,
    include_zero: bool,
) {
    let last = (BINS_CHART - 1) as f32;
    let win_end = win_start + win_span;
    let (min, max) = if let Some(r) = range {
        r
    } else {
        return;
    };
    let inner = (h - 2.0).max(1.0);
    let mut pb = PathBuilder::new();
    let mut started = false;
    for (i, b) in lap.bins.iter().enumerate() {
        let t = i as f32 / last;
        if t < win_start || t > win_end {
            continue;
        }
        let v = chart_bin_value(lap, i, b, pick, infer_speed, infer_throttle);
        if v == 0.0 && !started && !include_zero {
            continue;
        }
        let px_ = x + w * ((t - win_start) / win_span.max(0.001));
        let py = y + 1.0 + inner - (v - min) / (max - min) * inner;
        if !started {
            pb.move_to(px_, py);
            started = true;
        } else {
            pb.line_to(px_, py);
        }
    }
    if let Some(path) = pb.finish() {
        let mut paint = Paint::default();
        paint.set_color(col);
        paint.anti_alias = true;
        px.stroke_path(
            &path,
            &paint,
            &Stroke {
                width: 1.4,
                ..Stroke::default()
            },
            Transform::identity(),
            None,
        );
    }
}

const BINS_CHART: usize = mxbo_hud::location_tape::BINS;
const CHART_MIN_BINS: f32 = 24.0;

fn chart_window(pos: f32, zoom: f32) -> (f32, f32) {
    let span = (1.0 / zoom.max(1.0)).clamp(CHART_MIN_BINS / BINS_CHART as f32, 1.0);
    let pos = pos.clamp(0.0, 1.0);
    let start = (pos - span * 0.5).clamp(0.0, 1.0 - span);
    (start, span)
}

fn best_lap(detail: &SessionDetail, race_num: i32) -> Option<&crate::review::LapView> {
    detail
        .laps
        .iter()
        .filter(|l| l.race_num == race_num && !l.crashed && !l.cut && l.lap_ms > 0)
        .min_by_key(|l| l.lap_ms)
}

fn your_laps(detail: &SessionDetail, warmup: bool) -> Vec<&crate::review::LapView> {
    detail
        .laps
        .iter()
        .filter(|l| l.race_num == detail.your_race_num && l.warmup == warmup)
        .collect()
}

fn best_you(detail: &SessionDetail, warmup: bool) -> Option<&crate::review::LapView> {
    your_laps(detail, warmup)
        .into_iter()
        .filter(|l| !l.crashed && !l.cut && l.lap_ms > 0)
        .min_by_key(|l| l.lap_ms)
}

fn analyze_sector_starts(milli: [i32; 2]) -> [(f32, &'static str); 3] {
    [
        (0.0, "S1"),
        (milli[0] as f32 / 1000.0, "S2"),
        (milli[1] as f32 / 1000.0, "S3"),
    ]
}

fn sector_frac_known(i: usize, frac: f32) -> bool {
    i == 0 || (0.04..0.96).contains(&frac)
}

fn pick_you_lap(
    detail: &SessionDetail,
    you_sel: i32,
    warmup: bool,
) -> Option<&crate::review::LapView> {
    let laps = your_laps(detail, warmup);
    if you_sel >= 0 {
        return laps.into_iter().find(|l| l.lap_num == you_sel);
    }
    laps.into_iter().next()
}

fn is_first_race_lap(detail: &SessionDetail, picked: &crate::review::LapView) -> bool {
    !picked.warmup
        && your_laps(detail, false)
            .first()
            .is_some_and(|first| first.lap_num == picked.lap_num)
}

const HOLESHOT_NEAR_M: f32 = 14.0;

fn you_holeshot_split(detail: &SessionDetail, picked: &crate::review::LapView) -> Option<usize> {
    if !is_first_race_lap(detail, picked) || picked.line.len() < 2 {
        return None;
    }
    Some(holeshot_end(&picked.line, &detail.poly, detail.sf_meters))
}

fn holeshot_end(line: &[(f32, f32, f32)], poly: &[(f32, f32)], sf_meters: f32) -> usize {
    if line.len() < 2 {
        return line.len().saturating_sub(1);
    }
    let last = line.len() - 1;
    let Some(hit) = along_pts(poly, if sf_meters < 0.0 { 0.0 } else { sf_meters }) else {
        return last;
    };
    let (sx, sz) = (hit.4, hit.5);
    let near = HOLESHOT_NEAR_M * HOLESHOT_NEAR_M;
    let start_d2 = {
        let dx = line[0].0 - sx;
        let dz = line[0].2 - sz;
        dx * dx + dz * dz
    };
    let mut seen_far = start_d2 > near;
    for i in 1..line.len() {
        let ddx = line[i].0 - sx;
        let ddz = line[i].2 - sz;
        let d2 = ddx * ddx + ddz * ddz;
        if !seen_far {
            if d2 > near {
                seen_far = true;
            }
            continue;
        }
        if d2 <= near {
            return i;
        }
    }
    last
}

fn line_pos_near_sf(line: &[(f32, f32, f32)], poly: &[(f32, f32)], sf_meters: f32) -> f32 {
    if line.is_empty() {
        return 0.0;
    }
    let last = (line.len() - 1) as f32;
    let Some(hit) = along_pts(poly, if sf_meters < 0.0 { 0.0 } else { sf_meters }) else {
        return 1.0;
    };
    let (sx, sz) = (hit.4, hit.5);
    let mut best_i = line.len() - 1;
    let mut best_d = f32::MAX;
    for (i, &(x, _, z)) in line.iter().enumerate() {
        let dx = x - sx;
        let dz = z - sz;
        let d = dx * dx + dz * dz;
        if d < best_d {
            best_d = d;
            best_i = i;
        }
    }
    best_i as f32 / last
}

pub(crate) fn default_analyze_scrub(detail: &SessionDetail, you_lap: i32, warmup: bool) -> f32 {
    let Some(picked) = pick_you_lap(detail, you_lap, warmup) else {
        return 0.0;
    };
    if !is_first_race_lap(detail, picked) {
        return 0.0;
    }
    line_pos_near_sf(&picked.line, &detail.poly, detail.sf_meters)
}

fn step_you_lap(laps: &[&crate::review::LapView], sel: i32, dir: i32) -> Option<i32> {
    if laps.is_empty() {
        return None;
    }
    let idx = if sel >= 0 {
        laps.iter().position(|l| l.lap_num == sel).unwrap_or(0)
    } else {
        0
    };
    let next = idx as i32 + dir;
    if next < 0 || next >= laps.len() as i32 {
        return Some(laps[idx].lap_num);
    }
    Some(laps[next as usize].lap_num)
}

fn lap_chip_label(lap: &crate::review::LapView, n: usize) -> String {
    if lap.crashed {
        format!("L{} crash", n)
    } else if lap.cut {
        format!("L{} cut", n)
    } else {
        format!("L{} {}", n, fmt_lap(lap.lap_ms))
    }
}

const CRASH_CLUSTER_PX: f32 = 24.0;

fn collapse_contact_names<'a, I>(names: I) -> Vec<String>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut uniq: Vec<String> = Vec::new();
    for n in names {
        let n = n.trim();
        if n.is_empty() || uniq.iter().any(|e| e == n) {
            continue;
        }
        uniq.push(n.to_string());
    }
    uniq.iter()
        .filter(|n| !uniq.iter().any(|o| o != *n && o.contains(n.as_str())))
        .cloned()
        .collect()
}

fn cluster_crash_marks(pts: &[(f32, f32, &str)]) -> Vec<(f32, f32, String)> {
    let mut clusters: Vec<(f32, f32, f32, Vec<String>)> = Vec::new();
    let merge = CRASH_CLUSTER_PX * CRASH_CLUSTER_PX;
    for &(hx, hy, name) in pts {
        let hit = clusters.iter().position(|c| {
            let cx = c.0 / c.2;
            let cy = c.1 / c.2;
            let dx = hx - cx;
            let dy = hy - cy;
            dx * dx + dy * dy <= merge
        });
        if let Some(i) = hit {
            clusters[i].0 += hx;
            clusters[i].1 += hy;
            clusters[i].2 += 1.0;
            if !name.is_empty() {
                clusters[i].3.push(name.to_string());
            }
        } else {
            let names = if name.is_empty() {
                Vec::new()
            } else {
                vec![name.to_string()]
            };
            clusters.push((hx, hy, 1.0, names));
        }
    }
    clusters
        .into_iter()
        .map(|(xs, ys, n, names)| {
            let label = collapse_contact_names(names.iter().map(|s| s.as_str())).join(" · ");
            (xs / n, ys / n, label)
        })
        .collect()
}

fn crash_caption_for(lap: Option<&crate::review::LapView>) -> Option<String> {
    let marks = &lap?.crashes;
    if marks.is_empty() {
        return None;
    }
    let names = collapse_contact_names(marks.iter().map(|m| m.contact_name.as_str()));
    if names.is_empty() {
        Some("Crash".into())
    } else {
        Some(format!("Crash · {}", names.join(" · ")))
    }
}

fn cut_caption_for(lap: Option<&crate::review::LapView>) -> Option<String> {
    lap.filter(|l| l.cut).map(|_| "Cut".into())
}

fn draw_you_lap_stepper(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    detail: &SessionDetail,
    you_sel: i32,
    warmup: bool,
    open_drop: Option<Drop>,
    has_wu: bool,
    has_race: bool,
) -> f32 {
    let laps = your_laps(detail, warmup);
    let h = 32.0;
    let arrow = 36.0;
    let sess_w = 88.0;
    let mid_w = (w - arrow * 2.0 - sess_w - 16.0).max(120.0);
    let prev = step_you_lap(&laps, you_sel, -1);
    let next = step_you_lap(&laps, you_sel, 1);
    let cur = pick_you_lap(detail, you_sel, warmup);
    let idx = cur
        .and_then(|c| laps.iter().position(|l| l.lap_num == c.lap_num))
        .unwrap_or(0);
    let at_first = prev.is_none() || cur.is_some_and(|c| prev == Some(c.lap_num));
    let at_last = next.is_none() || cur.is_some_and(|c| next == Some(c.lap_num));

    fill_round(px, x, y, w, h, 6.0, night_ink());

    if let Some(p) = prev.filter(|_| !at_first) {
        hits.push(HitBox {
            id: Hit::AnalyzeYouLap(p),
            x,
            y,
            w: arrow,
            h,
        });
    }
    if hover == prev.map(Hit::AnalyzeYouLap) && !at_first {
        fill_round(px, x, y, arrow, h, 6.0, well());
    }
    text(
        px,
        fonts,
        "<",
        16.0,
        x + arrow * 0.5,
        y + 6.0,
        if at_first { dim() } else { text_col() },
        true,
    );

    let mx = x + arrow + 8.0;
    hits.push(HitBox {
        id: Hit::AnalyzeLapOpen,
        x: mx,
        y,
        w: mid_w,
        h,
    });
    let lap_on = open_drop == Some(Drop::AnalyzeLap);
    if hover == Some(Hit::AnalyzeLapOpen) || lap_on {
        fill_round(px, mx, y, mid_w, h, 6.0, well());
    }
    let gold =
        best_you(detail, warmup).is_some_and(|b| cur.is_some_and(|c| c.lap_num == b.lap_num));
    let prefix = format!("L{} ", idx + 1);
    let time = cur
        .map(|l| {
            if l.crashed {
                "crash".into()
            } else if l.cut {
                "cut".into()
            } else {
                fmt_lap(l.lap_ms)
            }
        })
        .unwrap_or_else(|| "—".into());
    let time_col = if cur.is_some_and(|c| c.crashed || c.cut) {
        dim()
    } else if gold {
        Color::from_rgba8(212, 176, 80, 255)
    } else {
        text_col()
    };
    let tw = measure(fonts, &prefix, 13.0) + measure(fonts, &time, 13.0);
    let tx = mx + (mid_w - tw).max(0.0) * 0.5;
    text(px, fonts, &prefix, 13.0, tx, y + 8.0, text_col(), false);
    text(
        px,
        fonts,
        &time,
        13.0,
        tx + measure(fonts, &prefix, 13.0),
        y + 8.0,
        time_col,
        false,
    );

    let nx = mx + mid_w + 8.0;
    if let Some(n) = next.filter(|_| !at_last) {
        hits.push(HitBox {
            id: Hit::AnalyzeYouLap(n),
            x: nx,
            y,
            w: arrow,
            h,
        });
    }
    if hover == next.map(Hit::AnalyzeYouLap) && !at_last {
        fill_round(px, nx, y, arrow, h, 6.0, well());
    }
    text(
        px,
        fonts,
        ">",
        16.0,
        nx + arrow * 0.5,
        y + 6.0,
        if at_last { dim() } else { text_col() },
        true,
    );

    let sx = x + w - sess_w;
    let session_on = open_drop == Some(Drop::AnalyzeSession);
    let both = has_wu && has_race;
    if both {
        hits.push(HitBox {
            id: Hit::AnalyzeSessionOpen,
            x: sx,
            y,
            w: sess_w,
            h,
        });
    }
    if session_on || hover == Some(Hit::AnalyzeSessionOpen) {
        fill_round(px, sx, y, sess_w, h, 6.0, well());
    }
    text(
        px,
        fonts,
        if warmup { "Warmup" } else { "Race" },
        12.0,
        sx + sess_w * 0.5,
        y + 8.0,
        text_col(),
        true,
    );
    if session_on && both {
        DROP_MENUS.with(|menus| {
            menus.borrow_mut().push(PendingDrop {
                mx: sx,
                my: y + h + 6.0,
                bw: sess_w,
                content_h: 10.0 + 28.0 * 2.0,
                open_hit: Hit::AnalyzeSessionOpen,
                options: vec![
                    (Hit::AnalyzeSession(0), "Race".into(), !warmup),
                    (Hit::AnalyzeSession(1), "Warmup".into(), warmup),
                ],
            });
        });
    }

    if lap_on && !laps.is_empty() {
        let options: Vec<(Hit, String, bool)> = laps
            .iter()
            .enumerate()
            .map(|(i, lap)| {
                (
                    Hit::AnalyzeYouLap(lap.lap_num),
                    lap_chip_label(lap, i + 1),
                    cur.is_some_and(|c| c.lap_num == lap.lap_num),
                )
            })
            .collect();
        DROP_MENUS.with(|menus| {
            menus.borrow_mut().push(PendingDrop {
                mx,
                my: y + h + 6.0,
                bw: mid_w,
                content_h: 10.0 + 28.0 * options.len() as f32,
                open_hit: Hit::AnalyzeLapOpen,
                options,
            });
        });
    }
    y + h + 8.0
}

fn draw_analyze_sf(
    px: &mut Pixmap,
    pts: &[(f32, f32)],
    sf_meters: f32,
    to_px: &dyn Fn(f32, f32) -> (f32, f32),
    track_px: f32,
    clip: Option<&Mask>,
) {
    let Some(hit) = along_pts(pts, if sf_meters < 0.0 { 0.0 } else { sf_meters }) else {
        return;
    };
    let (x0, y0) = to_px(hit.0, hit.1);
    let (x1, y1) = to_px(hit.2, hit.3);
    let (hx, hy) = to_px(hit.4, hit.5);
    let tx = x1 - x0;
    let ty = y1 - y0;
    let len = (tx * tx + ty * ty).sqrt().max(1.0e-4);
    let pxn = -ty / len;
    let pyn = tx / len;
    let half = analyze_cross_half(track_px);
    let mut pb = PathBuilder::new();
    pb.move_to(hx - pxn * half, hy - pyn * half);
    pb.line_to(hx + pxn * half, hy + pyn * half);
    if let Some(path) = pb.finish() {
        let mut ink = Paint::default();
        ink.set_color(Color::from_rgba8(8, 8, 10, 220));
        ink.anti_alias = true;
        px.stroke_path(
            &path,
            &ink,
            &Stroke {
                width: (track_px * 0.55).clamp(5.0, 9.0),
                line_cap: LineCap::Round,
                ..Stroke::default()
            },
            Transform::identity(),
            clip,
        );
        let mut orange = Paint::default();
        orange.set_color(accent());
        orange.anti_alias = true;
        px.stroke_path(
            &path,
            &orange,
            &Stroke {
                width: (track_px * 0.38).clamp(3.5, 6.5),
                line_cap: LineCap::Round,
                ..Stroke::default()
            },
            Transform::identity(),
            clip,
        );
    }
}

fn poly_len(pts: &[(f32, f32)]) -> f32 {
    let mut d = 0.0;
    for i in 1..pts.len() {
        let dx = pts[i].0 - pts[i - 1].0;
        let dz = pts[i].1 - pts[i - 1].1;
        d += (dx * dx + dz * dz).sqrt();
    }
    d
}

fn pts_centroid(pts: &[(f32, f32)]) -> (f32, f32) {
    let n = pts.len().max(1) as f32;
    let (sx, sz) = pts.iter().fold((0.0, 0.0), |(x, z), p| (x + p.0, z + p.1));
    (sx / n, sz / n)
}

fn pts_at_frac(
    pts: &[(f32, f32)],
    sf_meters: f32,
    frac: f32,
) -> Option<(f32, f32, f32, f32, f32, f32)> {
    let dist = poly_len(pts);
    if dist < 1.0 {
        return None;
    }
    let origin = if sf_meters >= 0.0 { sf_meters } else { 0.0 };
    along_pts(pts, origin + frac.rem_euclid(1.0) * dist)
}

fn stroke_dashed(
    px: &mut Pixmap,
    path: &Path,
    color: Color,
    width: f32,
    dash: f32,
    gap: f32,
    clip: Option<&Mask>,
) {
    let Some(dash) = StrokeDash::new(vec![dash, gap], 0.0) else {
        return;
    };
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;
    px.stroke_path(
        path,
        &paint,
        &Stroke {
            width,
            dash: Some(dash),
            line_cap: LineCap::Round,
            ..Stroke::default()
        },
        Transform::identity(),
        clip,
    );
}

fn draw_analyze_sectors(
    px: &mut Pixmap,
    fonts: &Fonts,
    pts: &[(f32, f32)],
    sf_meters: f32,
    starts: [(f32, &'static str); 3],
    to_px: &dyn Fn(f32, f32) -> (f32, f32),
    track_px: f32,
    clip: Option<&Mask>,
) {
    if pts.len() < 2 {
        return;
    }
    let (ccx, ccy) = {
        let (wx, wz) = pts_centroid(pts);
        to_px(wx, wz)
    };
    let half = analyze_cross_half(track_px);
    let dash = (half * 0.4).clamp(1.6, 3.5);
    let thick = (track_px * 0.14).clamp(1.1, 2.0);
    let violet = best_violet();
    let sz = (track_px * 0.95).clamp(10.0, 15.0);
    let pw = px.width() as f32;
    let ph = px.height() as f32;
    for (i, &(frac, label)) in starts.iter().enumerate() {
        if !sector_frac_known(i, frac) {
            continue;
        }
        let Some(hit) = pts_at_frac(pts, sf_meters, frac) else {
            continue;
        };
        let (x0, y0) = to_px(hit.0, hit.1);
        let (x1, y1) = to_px(hit.2, hit.3);
        let (hx, hy) = to_px(hit.4, hit.5);
        let tx = x1 - x0;
        let ty = y1 - y0;
        let len = (tx * tx + ty * ty).sqrt().max(1.0e-4);
        let pxn = -ty / len;
        let pyn = tx / len;
        let mut pb = PathBuilder::new();
        pb.move_to(hx - pxn * half, hy - pyn * half);
        pb.line_to(hx + pxn * half, hy + pyn * half);
        if let Some(path) = pb.finish() {
            stroke_dashed(
                px,
                &path,
                Color::from_rgba8(8, 8, 10, 200),
                thick + 1.0,
                dash,
                dash,
                clip,
            );
            stroke_dashed(px, &path, violet, thick, dash, dash, clip);
        }
        let mut side = if (hx - ccx) * pxn + (hy - ccy) * pyn >= 0.0 {
            1.0
        } else {
            -1.0
        };
        let pad = half + sz * 0.7;
        let mut lx = hx + pxn * side * pad;
        let mut ly = hy + pyn * side * pad;
        if lx < sz || ly < sz || lx > pw - sz || ly > ph - sz {
            side = -side;
            lx = hx + pxn * side * pad;
            ly = hy + pyn * side * pad;
        }
        let ink = Color::from_rgba8(8, 8, 10, 230);
        for (ox, oy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
            text(px, fonts, label, sz, lx + ox, ly + oy - sz * 0.5, ink, true);
        }
        text(px, fonts, label, sz, lx, ly - sz * 0.5, violet, true);
    }
}

fn along_pts(pts: &[(f32, f32)], meters: f32) -> Option<(f32, f32, f32, f32, f32, f32)> {
    if pts.len() < 2 {
        return None;
    }
    let mut d = vec![0.0f32; pts.len()];
    for i in 1..pts.len() {
        let dx = pts[i].0 - pts[i - 1].0;
        let dz = pts[i].1 - pts[i - 1].1;
        d[i] = d[i - 1] + (dx * dx + dz * dz).sqrt();
    }
    let dist = *d.last()?;
    if dist < 1.0 {
        return None;
    }
    let mut target = meters;
    if target > dist {
        target %= dist;
    }
    if target < 0.0 {
        target = 0.0;
    }
    for i in 1..pts.len() {
        if d[i] < target {
            continue;
        }
        let span = d[i] - d[i - 1];
        let u = if span > 0.001 {
            (target - d[i - 1]) / span
        } else {
            0.0
        };
        let ax = pts[i - 1].0;
        let az = pts[i - 1].1;
        let bx = pts[i].0;
        let bz = pts[i].1;
        return Some((ax, az, bx, bz, ax + (bx - ax) * u, az + (bz - az) * u));
    }
    None
}

fn draw_crash_triangle(px: &mut Pixmap, x: f32, y: f32) {
    let mut pb = PathBuilder::new();
    pb.move_to(x, y - 7.0);
    pb.line_to(x + 6.0, y + 5.0);
    pb.line_to(x - 6.0, y + 5.0);
    pb.close();
    if let Some(path) = pb.finish() {
        let mut fill = Paint::default();
        fill.set_color(Color::from_rgba8(232, 56, 56, 255));
        fill.anti_alias = true;
        px.fill_path(&path, &fill, FillRule::Winding, Transform::identity(), None);
    }
}

fn rider_name(detail: &SessionDetail, race_num: i32) -> String {
    detail
        .riders
        .iter()
        .find(|r| r.race_num == race_num)
        .map(|r| r.name.clone())
        .unwrap_or_else(|| format!("#{race_num}"))
}

fn bin_at(lap: &crate::review::LapView, pos: f32) -> mxbo_hud::location_tape::ChannelBin {
    lap.bins[bin_index(pos)]
}

fn pos_at_ms(lap: &crate::review::LapView, you_ms: i32) -> f32 {
    if you_ms <= 0 {
        return 0.0;
    }
    if lap.lap_ms > 0 && you_ms >= lap.lap_ms {
        return 0.999;
    }
    let last = (BINS_CHART - 1) as f32;
    let mut found = None;
    for (i, b) in lap.bins.iter().enumerate() {
        if b.ms > 0 && b.ms <= you_ms {
            found = Some(i);
        }
    }
    if let Some(i) = found {
        return (i as f32 / last).clamp(0.0, 0.999);
    }
    if lap.lap_ms > 0 {
        return (you_ms as f32 / lap.lap_ms as f32).clamp(0.0, 0.999);
    }
    0.0
}

fn draw_follow_chip(
    px: &mut Pixmap,
    fonts: &Fonts,
    map_x: f32,
    map_y: f32,
    map_w: f32,
    on: bool,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) {
    let label = "Follow";
    let bw = 76.0;
    let bh = 26.0;
    let x = map_x + map_w - bw - 10.0;
    let y = map_y + 8.0;
    let hit = Hit::AnalyzeFollow;
    hits.push(HitBox {
        id: hit,
        x,
        y,
        w: bw,
        h: bh,
    });
    if on {
        fill_skew(px, x, y, bw - 6.0, bh, 5.0, accent());
        text(px, fonts, label, 12.0, x + bw * 0.5, y + 6.0, ink(), true);
    } else {
        fill_round(
            px,
            x,
            y,
            bw,
            bh,
            8.0,
            Color::from_rgba8(8, 8, 10, if hover == Some(hit) { 200 } else { 170 }),
        );
        text(
            px,
            fonts,
            label,
            12.0,
            x + bw * 0.5,
            y + 6.0,
            text_col(),
            true,
        );
    }
}

pub(crate) fn follow_pan_for(
    id: i64,
    you_sel: i32,
    warmup: bool,
    scrub: f32,
) -> Option<(f32, f32)> {
    let detail = review::load(id)?;
    let lap = pick_you_lap(&detail, you_sel, warmup)?;
    follow_pan(&detail.poly, &lap.line, scrub)
}

fn follow_pan(poly: &[(f32, f32)], line: &[(f32, f32, f32)], scrub: f32) -> Option<(f32, f32)> {
    if poly.len() < 2 {
        return None;
    }
    let (lx, _, lz) = point_at(line, scrub)?;
    let mut min_x = poly[0].0;
    let mut max_x = min_x;
    let mut min_z = poly[0].1;
    let mut max_z = min_z;
    for p in poly {
        min_x = min_x.min(p.0);
        max_x = max_x.max(p.0);
        min_z = min_z.min(p.1);
        max_z = max_z.max(p.1);
    }
    Some((lx - (min_x + max_x) * 0.5, lz - (min_z + max_z) * 0.5))
}

fn point_at(line: &[(f32, f32, f32)], pos: f32) -> Option<(f32, f32, f32)> {
    if line.is_empty() {
        return None;
    }
    let i = ((pos.clamp(0.0, 0.999) * line.len() as f32) as usize).min(line.len() - 1);
    Some(line[i])
}

fn fmt_lap(ms: i32) -> String {
    if ms <= 0 {
        return "—".into();
    }
    let m = ms / 60000;
    let s = (ms % 60000) / 1000;
    let t = (ms % 1000) / 10;
    if m > 0 {
        format!("{m}:{s:02}.{t:02}")
    } else {
        format!("{s}.{t:02}")
    }
}

fn format_when(unix: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(unix);
    let age = (now - unix).max(0);
    if age < 3600 {
        format!("{} min ago", (age / 60).max(1))
    } else if age < 86400 {
        format!("{} h ago", age / 3600)
    } else {
        format!("{} d ago", age / 86400)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        along_pts, analyze_cross_half, analyze_sector_starts, analyze_track_px, bake_analyze_track,
        best_lap, chart_range, chart_window, cluster_crash_marks, collapse_contact_names,
        compare_choices, compare_lap, compare_rider_label, crash_caption_for, cut_caption_for,
        default_analyze_scrub, follow_pan, holeshot_end, icon_tip_label, implied_gear,
        implied_throttle, in_well, inferred_speed, pick_compare, pick_you_lap, pos_at_ms,
        sector_frac_known, smooth_poly_pts, spot_gear, spot_rpm_delta, spot_speed,
        spot_speed_delta, spot_throttle, step_you_lap, tape_gear, tape_has_rpm, tape_has_throttle,
        tape_speed, tape_throttle, tape_throttle_ok, thin_world_pts, you_holeshot_split, your_laps,
        IconKind, Units,
    };
    use crate::review::{LapView, RiderRow, SessionDetail, SessionRow};
    use mxbo_hud::location_tape::{ChannelBin, CrashMark, BINS};

    fn lap(num: i32, ms: i32, crashed: bool) -> LapView {
        LapView {
            race_num: 2,
            lap_num: num,
            lap_ms: ms,
            sectors: [0, 0, 0],
            bins: [ChannelBin::default(); BINS],
            line: Vec::new(),
            crashed,
            cut: false,
            warmup: false,
            crashes: Vec::new(),
        }
    }

    fn cut_lap(num: i32, ms: i32) -> LapView {
        let mut l = lap(num, ms, false);
        l.cut = true;
        l.line = vec![(0.0, 0.0, 0.0), (80.0, 0.0, 0.0)];
        l
    }

    fn detail(laps: Vec<LapView>) -> SessionDetail {
        SessionDetail {
            row: SessionRow {
                id: 1,
                started: 0,
                track: "Hangtown".into(),
                rider_count: 2,
                your_best_ms: 110_000,
                fastest_name: "Cole".into(),
                fastest_ms: 108_000,
                kept: false,
                you_won: false,
            },
            your_race_num: 2,
            fastest_race_num: 7,
            poly: Vec::new(),
            sf_meters: 0.0,
            riders: Vec::new(),
            laps,
        }
    }

    #[test]
    fn spot_speed_delta_uses_settings_units() {
        assert!(spot_speed_delta(14.2, Units::Imperial).contains("mph"));
        assert!(spot_speed_delta(14.2, Units::Metric).contains("kph"));
    }

    #[test]
    fn spot_speed_uses_settings_units() {
        assert!(spot_speed(14.2, 1, Units::Imperial).contains("mph"));
        assert!(spot_speed(14.2, 1, Units::Metric).contains("kph"));
        assert!(!spot_speed(14.2, 1, Units::Imperial).contains('+'));
    }

    #[test]
    fn spot_speed_empty_bin_is_dash() {
        assert_eq!(spot_speed(0.0, 0, Units::Imperial), "—");
        assert_ne!(spot_speed(0.0, 1, Units::Imperial), "—");
    }

    #[test]
    fn spot_gear_zero_is_dash() {
        assert_eq!(spot_gear(0), "—");
        assert_eq!(spot_gear(4), "4");
    }

    #[test]
    fn spot_throttle_empty_bin_is_dash() {
        assert_eq!(spot_throttle(0.0, 0, true), "—");
        assert_eq!(spot_throttle(0.0, 1, true), "0.00");
        assert_eq!(spot_throttle(0.82, 1, true), "0.82");
        assert_eq!(spot_throttle(0.0, 1, false), "—");
    }

    #[test]
    fn inferred_speed_from_line_and_ms() {
        let mut l = lap(0, 60_000, false);
        l.line = (0..=BINS).map(|i| (i as f32 * 10.0, 0.0, 0.0)).collect();
        l.bins[0].ms = 1000;
        l.bins[1].ms = 2000;
        let got = inferred_speed(&l, 1).expect("speed");
        assert!(
            (got - 10.0).abs() < 0.05,
            "expected ~10 m/s from 10 m / 1 s, got {got}"
        );
    }

    #[test]
    fn tape_speed_recorded_wins() {
        let mut l = lap(0, 60_000, false);
        l.line = (0..=BINS).map(|i| (i as f32 * 10.0, 0.0, 0.0)).collect();
        l.bins[0].ms = 1000;
        l.bins[1].ms = 2000;
        l.bins[1].speed = 20.0;
        assert_eq!(tape_speed(&l, 1), 20.0);
    }

    #[test]
    fn implied_throttle_from_speed_change() {
        let mut l = lap(0, 60_000, false);
        l.line = (0..=BINS).map(|i| (i as f32 * 10.0, 0.0, 0.0)).collect();
        l.bins[0].ms = 1000;
        l.bins[1].ms = 2000;
        let t = implied_throttle(&l, 1).expect("throttle");
        assert!(t > 0.8, "accel ~6 m/s2 should be near 1, got {t}");
        assert!(tape_throttle_ok(&l, 1));
        assert!(!tape_has_throttle(&l));
        assert!(tape_throttle(&l, 1) > 0.8);
    }

    #[test]
    fn implied_gear_from_your_speed_bands() {
        let mut you = lap(0, 60_000, false);
        you.bins[0].gear = 2;
        you.bins[0].speed = 10.0;
        you.bins[1].gear = 3;
        you.bins[1].speed = 20.0;
        assert_eq!(implied_gear(&you, 19.0), 3);
        assert_eq!(implied_gear(&you, 11.0), 2);
        let mut them = lap(0, 60_000, false);
        them.line = (0..=BINS).map(|i| (i as f32 * 10.0, 0.0, 0.0)).collect();
        them.bins[0].ms = 1000;
        them.bins[1].ms = 2000;
        them.bins[1].speed = 19.0;
        assert_eq!(tape_gear(&you, &them, 1), 3);
    }

    #[test]
    fn tape_has_throttle_false_when_all_zero() {
        let mut l = lap(0, 60_000, false);
        l.bins[3].ms = 1000;
        assert!(!tape_has_throttle(&l));
        l.bins[3].throttle = 0.4;
        assert!(tape_has_throttle(&l));
    }

    #[test]
    fn spot_rpm_delta_dash_when_them_has_no_rpm() {
        let mut l = lap(0, 60_000, false);
        assert!(!tape_has_rpm(&l));
        assert_eq!(spot_rpm_delta(11000.0, 0.0, false), "—");
        l.bins[1].rpm = 9000.0;
        assert!(tape_has_rpm(&l));
        assert_eq!(spot_rpm_delta(11000.0, 9000.0, true), "+2000");
    }

    #[test]
    fn thin_world_pts_keeps_ends_and_drops_subpixel() {
        let pts: Vec<(f32, f32)> = (0..16_000).map(|i| (i as f32 * 0.01, 0.0)).collect();
        let thin = thin_world_pts(&pts, 1.0);
        assert!(
            thin.len() < 300,
            "16k collinear samples should thin, got {}",
            thin.len()
        );
        assert_eq!(thin.first(), pts.first());
        assert_eq!(thin.last(), pts.last());
        let coarse = thin_world_pts(&pts, 4.0);
        assert!(coarse.len() < thin.len());
        assert_eq!(coarse.first(), pts.first());
        assert_eq!(coarse.last(), pts.last());
    }

    fn poly_len(pts: &[(f32, f32)]) -> f32 {
        pts.windows(2)
            .map(|w| {
                let dx = w[1].0 - w[0].0;
                let dy = w[1].1 - w[0].1;
                (dx * dx + dy * dy).sqrt()
            })
            .sum()
    }

    #[test]
    fn smooth_poly_pts_keeps_ends_and_shortens_zigzag() {
        let zig: Vec<(f32, f32)> = (0..21)
            .map(|i| (i as f32, if i % 2 == 0 { 0.0 } else { 8.0 }))
            .collect();
        let smooth = smooth_poly_pts(&zig, false);
        assert_eq!(smooth.first(), zig.first());
        assert_eq!(smooth.last(), zig.last());
        assert!(
            poly_len(&smooth) < poly_len(&zig),
            "smoothed zigzag should be shorter"
        );
    }

    #[test]
    fn bake_analyze_track_zoom_is_immediate() {
        assert!(bake_analyze_track(true, false, true));
        assert!(!bake_analyze_track(true, false, false));
        assert!(bake_analyze_track(true, true, false));
        assert!(!bake_analyze_track(false, false, true));
    }

    #[test]
    fn in_well_rejects_outside_map() {
        assert!(in_well(10.0, 10.0, 200.0, 180.0));
        assert!(!in_well(-4.0, 10.0, 200.0, 180.0));
        assert!(!in_well(10.0, 190.0, 200.0, 180.0));
    }

    #[test]
    fn analyze_track_is_at_least_twelve_px_when_zoomed_out() {
        assert!(analyze_track_px(0.4) >= 12.0);
    }

    #[test]
    fn analyze_track_grows_past_old_cap_when_zoomed_in() {
        let px = analyze_track_px(8.0);
        assert!(
            (px - 128.0).abs() < 0.01,
            "zoomed width {px} should stay ~16 m * scale, not cap at 48"
        );
    }

    #[test]
    fn analyze_cross_half_is_half_track() {
        let px = analyze_track_px(1.0);
        assert!((px - 16.0).abs() < 0.01);
        assert!((analyze_cross_half(px) - 8.0).abs() < 0.01);
    }

    #[test]
    fn analyze_sector_starts_from_track_milli() {
        let starts = analyze_sector_starts([333, 671]);
        assert_eq!(starts[0], (0.0, "S1"));
        assert!((starts[1].0 - 0.333).abs() < 0.0005);
        assert_eq!(starts[1].1, "S2");
        assert!((starts[2].0 - 0.671).abs() < 0.0005);
        assert_eq!(starts[2].1, "S3");
        assert!(sector_frac_known(1, starts[1].0));
        assert!(sector_frac_known(2, starts[2].0));
        let unknown = analyze_sector_starts([0, 0]);
        assert_eq!(unknown[1].0, 0.0);
        assert_eq!(unknown[2].0, 0.0);
        assert!(!sector_frac_known(1, unknown[1].0));
        assert!(!sector_frac_known(2, unknown[2].0));
    }

    #[test]
    fn sector_frac_skips_near_sf() {
        assert!(sector_frac_known(0, 0.0));
        assert!(!sector_frac_known(1, 0.0));
        assert!(!sector_frac_known(1, 0.02));
        assert!(sector_frac_known(1, 0.33));
        assert!(!sector_frac_known(2, 0.99));
    }

    #[test]
    fn chart_range_shared_scale_puts_faster_them_higher() {
        let mut you = lap(0, 100_000, false);
        let mut them = lap(0, 90_000, false);
        for b in you.bins.iter_mut() {
            b.speed = 35.0;
        }
        you.bins[0].speed = 20.0;
        you.bins[BINS / 2].speed = 45.0;
        you.bins[BINS - 1].speed = 50.0;
        for b in them.bins.iter_mut() {
            b.speed = 70.0;
        }
        them.bins[0].speed = 60.0;
        them.bins[BINS / 2].speed = 65.0;
        them.bins[BINS - 1].speed = 80.0;
        let (min, max) = chart_range(Some(&you), Some(&them), |b| b.speed, 0.0, 1.0, true, false)
            .expect("range");
        assert!((min - 20.0).abs() < 0.01);
        assert!((max - 80.0).abs() < 0.01);
        let you_y = (45.0 - min) / (max - min);
        let them_y = (65.0 - min) / (max - min);
        assert!(
            them_y > you_y,
            "them {them_y} should sit above you {you_y} on a shared scale"
        );
    }

    #[test]
    fn pick_you_lap_default_is_first() {
        let d = detail(vec![
            lap(0, 115_000, false),
            lap(1, 110_000, false),
            lap(2, 108_000, true),
        ]);
        let picked = pick_you_lap(&d, -1, false).expect("l1");
        assert_eq!(picked.lap_num, 0);
        assert_eq!(picked.lap_ms, 115_000);
    }

    #[test]
    fn pick_you_lap_n_is_that_lap() {
        let d = detail(vec![lap(0, 115_000, false), lap(1, 110_000, true)]);
        let picked = pick_you_lap(&d, 1, false).expect("l2");
        assert_eq!(picked.lap_num, 1);
        assert!(picked.crashed);
    }

    #[test]
    fn best_lap_skips_cut() {
        let d = detail(vec![cut_lap(0, 90_000), lap(1, 112_000, false)]);
        let best = best_lap(&d, 2).expect("clean");
        assert_eq!(best.lap_num, 1);
        assert_eq!(best.lap_ms, 112_000);
    }

    #[test]
    fn step_you_lap_does_not_wrap() {
        let owned = vec![
            lap(0, 115_000, false),
            lap(1, 110_000, false),
            lap(2, 112_000, false),
        ];
        let laps: Vec<_> = owned.iter().collect();
        assert_eq!(step_you_lap(&laps, 0, -1), Some(0));
        assert_eq!(step_you_lap(&laps, 2, 1), Some(2));
        assert_eq!(step_you_lap(&laps, 0, 1), Some(1));
        assert_eq!(step_you_lap(&laps, -1, -1), Some(0));
        assert_eq!(step_you_lap(&laps, -1, 1), Some(1));
    }

    #[test]
    fn pick_you_lap_filters_warmup() {
        let mut wu = lap(0, 120_000, false);
        wu.warmup = true;
        let d = detail(vec![wu, lap(1, 110_000, false)]);
        assert_eq!(pick_you_lap(&d, -1, false).unwrap().lap_num, 1);
        assert_eq!(pick_you_lap(&d, -1, true).unwrap().lap_num, 0);
        assert!(pick_you_lap(&d, 1, true).is_none());
        assert_eq!(your_laps(&d, false).len(), 1);
        assert_eq!(your_laps(&d, true).len(), 1);
    }

    #[test]
    fn holeshot_end_splits_at_first_sf() {
        let poly = vec![(0.0, 0.0), (200.0, 0.0)];
        let line: Vec<(f32, f32, f32)> = (0..=80).map(|i| (i as f32, 0.0, 0.0)).collect();
        let i = holeshot_end(&line, &poly, 50.0);
        let x = line[i].0;
        assert!(
            (x - 50.0).abs() <= 14.0,
            "split x {x} should be within 14m of S/F at 50"
        );
        assert!(i + 1 < line.len(), "rest of L1 stays after the dash");
    }

    #[test]
    fn holeshot_end_splits_at_sf_even_when_far() {
        let poly = vec![(0.0, 0.0), (800.0, 0.0)];
        let line: Vec<(f32, f32, f32)> = (0..=400).map(|i| (i as f32 * 2.0, 0.0, 0.0)).collect();
        let i = holeshot_end(&line, &poly, 800.0);
        let along = line[i].0;
        assert!(
            (along - 800.0).abs() <= 14.0,
            "split should be at S/F 800m, got {along}"
        );
    }

    #[test]
    fn you_holeshot_split_later_lap_and_warmup_are_none() {
        let poly_line = |n: i32, wu: bool| {
            let mut l = lap(n, 110_000, false);
            l.warmup = wu;
            l.line = (0..=80).map(|i| (i as f32, 0.0, 0.0)).collect();
            l
        };
        let mut d = detail(vec![poly_line(0, false), poly_line(1, false)]);
        d.poly = vec![(0.0, 0.0), (200.0, 0.0)];
        d.sf_meters = 50.0;
        let l2 = pick_you_lap(&d, 1, false).expect("l2");
        assert!(you_holeshot_split(&d, l2).is_none());

        let mut wu = poly_line(0, true);
        let race = poly_line(1, false);
        let mut d = detail(vec![wu.clone(), race]);
        d.poly = vec![(0.0, 0.0), (200.0, 0.0)];
        d.sf_meters = 50.0;
        wu = pick_you_lap(&d, -1, true).expect("warmup").clone();
        assert!(you_holeshot_split(&d, &wu).is_none());
        let l1 = pick_you_lap(&d, -1, false).expect("race l1");
        assert!(you_holeshot_split(&d, l1).is_some());
    }

    #[test]
    fn chart_window_mid_lap_at_zoom_two() {
        let (start, span) = chart_window(0.5, 2.0);
        assert!((span - 0.5).abs() < 0.001, "span {span}");
        assert!((start - 0.25).abs() < 0.001, "start {start}");
    }

    #[test]
    fn pos_at_ms_faster_other_is_ahead() {
        let mut other = lap(0, 50_000, false);
        let last = BINS - 1;
        for (i, b) in other.bins.iter_mut().enumerate() {
            b.ms = ((i as i32) * 50_000) / last as i32;
            if b.ms == 0 && i > 0 {
                b.ms = 1;
            }
        }
        other.bins[0].ms = 1;
        let pos = pos_at_ms(&other, 60_000);
        assert!(pos > 0.5, "faster other at 60s should be ahead, got {pos}");
        assert!((pos - 0.999).abs() < 0.001);
    }

    #[test]
    fn compare_rider_label_includes_displacement() {
        let r = RiderRow {
            race_num: 7,
            name: "Cole".into(),
            bike: "YZ450F".into(),
            position: 1,
            best_ms: 111_420,
            last_ms: 111_420,
            has_line: true,
        };
        assert_eq!(compare_rider_label(&r), "Cole | 450 | 1:51.42");
        let mut two = r.clone();
        two.bike = "YZ250F".into();
        two.best_ms = 98_000;
        assert_eq!(compare_rider_label(&two), "Cole | 250 | 1:38.00");
        let mut no_cc = r.clone();
        no_cc.bike = "Custom".into();
        assert_eq!(compare_rider_label(&no_cc), "Cole | 1:51.42");
    }

    #[test]
    fn compare_choices_lists_field_without_you() {
        let cole = RiderRow {
            race_num: 7,
            name: "Cole".into(),
            bike: "YZ450F".into(),
            position: 1,
            best_ms: 111_420,
            last_ms: 111_420,
            has_line: true,
        };
        let webb = RiderRow {
            race_num: 4,
            name: "Webb".into(),
            bike: "KX450".into(),
            position: 2,
            best_ms: 112_040,
            last_ms: 112_460,
            has_line: true,
        };
        let you = RiderRow {
            race_num: 2,
            name: "You".into(),
            bike: "CRF450".into(),
            position: 3,
            best_ms: 113_080,
            last_ms: 113_080,
            has_line: true,
        };
        let reed = RiderRow {
            race_num: 22,
            name: "Reed".into(),
            bike: "YZ450F".into(),
            position: 4,
            best_ms: 114_210,
            last_ms: 114_630,
            has_line: false,
        };
        let dnf = RiderRow {
            race_num: 99,
            name: "DNF".into(),
            bike: "YZ450F".into(),
            position: 5,
            best_ms: 0,
            last_ms: 0,
            has_line: false,
        };
        let mut cole_lap = lap(0, 111_420, false);
        cole_lap.race_num = 7;
        cole_lap.line = vec![(0.0, 0.0, 0.0), (1.0, 0.0, 1.0)];
        let mut webb_lap = lap(0, 112_040, false);
        webb_lap.race_num = 4;
        webb_lap.cut = true;
        webb_lap.line = Vec::new();
        let mut d = detail(vec![lap(0, 113_080, false), cole_lap, webb_lap]);
        d.riders = vec![cole, webb, you, reed, dnf];
        let names: Vec<_> = compare_choices(&d)
            .iter()
            .map(|r| r.name.as_str())
            .collect();
        assert_eq!(names, ["Cole", "Webb"]);
        assert_eq!(pick_compare(&d, -1), 7);
        d.fastest_race_num = 2;
        assert_eq!(pick_compare(&d, -1), 7);
        assert_eq!(pick_compare(&d, 4), 4);
        assert!(d.riders.iter().any(|r| r.race_num == 4 && r.has_line));
        assert!(d.laps.iter().any(|l| l.race_num == 4 && l.line.is_empty()));
    }

    #[test]
    fn pos_at_ms_even_pace_without_tape() {
        let other = lap(0, 50_000, false);
        let pos = pos_at_ms(&other, 30_000);
        assert!((pos - 0.6).abs() < 0.001, "even-pace {pos}");
    }

    #[test]
    fn chart_window_clamps_at_lap_start() {
        let (start, span) = chart_window(0.0, 4.0);
        assert!((span - 0.25).abs() < 0.001, "span {span}");
        assert!(start.abs() < 0.001, "start {start}");
    }

    #[test]
    fn follow_pan_centers_line_start() {
        let poly = [(0.0, 0.0), (100.0, 0.0)];
        let line = [(0.0, 0.0, 0.0), (50.0, 0.0, 0.0), (100.0, 0.0, 0.0)];
        let (px, pz) = follow_pan(&poly, &line, 0.0).expect("start");
        assert!((px + 50.0).abs() < 0.01, "pan_x {px}");
        assert!(pz.abs() < 0.01, "pan_z {pz}");
        let (mx, mz) = follow_pan(&poly, &line, 0.5).expect("mid");
        assert!(mx.abs() < 0.01, "mid pan_x {mx}");
        assert!(mz.abs() < 0.01, "mid pan_z {mz}");
    }

    #[test]
    fn along_pts_hits_mid_segment() {
        let pts = [(0.0, 0.0), (100.0, 0.0)];
        let hit = along_pts(&pts, 50.0).expect("hit");
        assert!((hit.4 - 50.0).abs() < 0.01);
        assert!(hit.5.abs() < 0.01);
    }

    fn holeshot_detail(laps: Vec<LapView>) -> SessionDetail {
        let mut d = detail(laps);
        d.poly = vec![(0.0, 0.0), (80.0, 0.0)];
        d.sf_meters = 80.0;
        d
    }

    fn with_line(mut l: LapView) -> LapView {
        l.line = vec![(0.0, 0.0, 0.0), (40.0, 0.0, 0.0), (80.0, 0.0, 0.0)];
        l
    }

    #[test]
    fn default_scrub_first_race_lap_is_finish() {
        let d = holeshot_detail(vec![
            with_line(lap(0, 115_000, false)),
            lap(1, 110_000, false),
        ]);
        let pos = default_analyze_scrub(&d, -1, false);
        assert!(
            (pos - 1.0).abs() < 0.01,
            "L1 scrub {pos} should sit on the finish"
        );
    }

    #[test]
    fn default_scrub_later_lap_is_start() {
        let d = holeshot_detail(vec![
            with_line(lap(0, 115_000, false)),
            with_line(lap(1, 110_000, false)),
        ]);
        assert_eq!(default_analyze_scrub(&d, 1, false), 0.0);
    }

    #[test]
    fn default_scrub_warmup_is_start() {
        let mut wu = with_line(lap(0, 120_000, false));
        wu.warmup = true;
        let d = holeshot_detail(vec![wu, with_line(lap(1, 110_000, false))]);
        assert_eq!(default_analyze_scrub(&d, -1, true), 0.0);
    }

    #[test]
    fn default_scrub_first_race_after_warmup_is_finish() {
        let mut wu = with_line(lap(0, 120_000, false));
        wu.warmup = true;
        let d = holeshot_detail(vec![wu, with_line(lap(1, 110_000, false))]);
        let pos = default_analyze_scrub(&d, -1, false);
        assert!(
            (pos - 1.0).abs() < 0.01,
            "first race lap {pos} should sit on the finish"
        );
        let again = default_analyze_scrub(&d, 1, false);
        assert!(
            (again - 1.0).abs() < 0.01,
            "lap_num 1 still holeshot {again}"
        );
    }

    fn mark(x: f32, z: f32, name: &str) -> CrashMark {
        CrashMark {
            x,
            y: 0.0,
            z,
            contact_num: 1,
            contact_name: name.into(),
        }
    }

    #[test]
    fn cut_caption_when_lap_is_cut() {
        let mut l = lap(0, 90_000, false);
        l.cut = true;
        assert_eq!(cut_caption_for(Some(&l)).as_deref(), Some("Cut"));
        assert_eq!(cut_caption_for(Some(&lap(0, 110_000, false))), None);
    }

    #[test]
    fn crash_caption_collapses_substring_names() {
        let mut l = lap(0, 110_000, true);
        l.crashes = vec![mark(0.0, 0.0, "813"), mark(1.0, 0.0, "Sam Hyde 813")];
        assert_eq!(
            crash_caption_for(Some(&l)).as_deref(),
            Some("Crash · Sam Hyde 813")
        );
    }

    #[test]
    fn crash_caption_unnamed_is_crash() {
        let mut l = lap(0, 110_000, true);
        l.crashes = vec![mark(0.0, 0.0, "")];
        assert_eq!(crash_caption_for(Some(&l)).as_deref(), Some("Crash"));
        assert_eq!(crash_caption_for(Some(&lap(0, 110_000, false))), None);
    }

    #[test]
    fn collapse_drops_substring_and_dupes() {
        assert_eq!(
            collapse_contact_names(["813", "Sam Hyde 813", "813"]),
            vec!["Sam Hyde 813".to_string()]
        );
    }

    #[test]
    fn cluster_merges_marks_within_24px() {
        let out = cluster_crash_marks(&[(10.0, 10.0, "813"), (14.0, 10.0, "Sam Hyde 813")]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].2, "Sam Hyde 813");
        assert!((out[0].0 - 12.0).abs() < 0.01);
        assert!((out[0].1 - 10.0).abs() < 0.01);
    }

    #[test]
    fn cluster_keeps_far_marks_apart() {
        let out = cluster_crash_marks(&[(0.0, 0.0, "A"), (50.0, 0.0, "B")]);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].2, "A");
        assert_eq!(out[1].2, "B");
    }

    #[test]
    fn icon_tip_labels_match_old_buttons() {
        assert_eq!(icon_tip_label(IconKind::Back, false), "Back");
        assert_eq!(icon_tip_label(IconKind::Trash, false), "Delete");
        assert_eq!(icon_tip_label(IconKind::Bookmark, false), "Save");
        assert_eq!(icon_tip_label(IconKind::Bookmark, true), "Saved");
    }
}
