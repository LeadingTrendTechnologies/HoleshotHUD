//! Profile → Tracks: per-track best lap + persistent ideal line.

use super::*;
use mxbo_hud::render::Fonts;
use tiny_skia::{Color, Pixmap};

const ROW_H: f32 = 44.0;

pub(crate) fn pane_tracks(
    px: &mut Pixmap,
    fonts: &Fonts,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
    selected: &Option<String>,
    zoom: f32,
    pan_x: f32,
    pan_z: f32,
) -> f32 {
    let mut y = y + 4.0;
    if let Some(track) = selected.as_ref() {
        return pane_track_detail(px, fonts, hover, hits, x, y, w, track, zoom, pan_x, pan_z);
    }
    let rows = crate::review::track_bank_rows();
    if rows.is_empty() {
        let copy =
            "No track lines yet. Enable Motos Record, finish clean laps, and they bank here.";
        for line in wrap_fb(fonts, copy, (w - 24.0).max(40.0), 13.0) {
            text(px, fonts, &line, 13.0, x + 4.0, y, muted(), false);
            y += 18.0;
        }
        return y + 24.0;
    }
    text(px, fonts, "TRACK", 10.0, x + 4.0, y, dim(), false);
    text(px, fonts, "BEST", 10.0, x + w * 0.46, y, dim(), false);
    text(px, fonts, "IDEAL", 10.0, x + w * 0.62, y, dim(), false);
    text(px, fonts, "GAP", 10.0, x + w * 0.78, y, dim(), false);
    y += 18.0;
    if let Some(r) = Rect::from_xywh(x, y, w, 1.0) {
        fill_rect(px, r, row_line());
    }
    y += 6.0;
    for (i, row) in rows.iter().enumerate() {
        if i > u16::MAX as usize {
            break;
        }
        let hit = Hit::TrackOpen(i as u16);
        hits.push(HitBox {
            id: hit,
            x,
            y,
            w,
            h: ROW_H,
        });
        if hover == Some(hit) {
            fill_round(px, x, y, w, ROW_H, 6.0, hover_wash());
        }
        let ty = y + 12.0;
        let name = ellipsize(fonts, &row.track, 14.0, (w * 0.42).max(40.0));
        text(px, fonts, &name, 14.0, x + 4.0, ty, text_col(), false);
        text(
            px,
            fonts,
            &fmt_lap(row.best_ms),
            13.0,
            x + w * 0.46,
            ty,
            text_col(),
            false,
        );
        text(
            px,
            fonts,
            &fmt_lap(row.ideal_ms),
            13.0,
            x + w * 0.62,
            ty,
            text_col(),
            false,
        );
        let gap = if row.best_ms > 0 && row.ideal_ms > 0 && row.best_ms > row.ideal_ms {
            format!("−{}", fmt_lap(row.best_ms - row.ideal_ms))
        } else {
            "—".into()
        };
        text(px, fonts, &gap, 13.0, x + w * 0.78, ty, muted(), false);
        y += ROW_H;
    }
    y + 12.0
}

fn pane_track_detail(
    px: &mut Pixmap,
    fonts: &Fonts,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
    track: &str,
    zoom: f32,
    pan_x: f32,
    pan_z: f32,
) -> f32 {
    let mut y = y;
    let back_w = (measure(fonts, "Back", 13.0) + 28.0).max(64.0);
    profile::profile_chip(
        px,
        fonts,
        x,
        y,
        "Back",
        false,
        Hit::TrackBack,
        hover,
        hits,
    );
    let title = ellipsize(fonts, track, 18.0, (w - back_w - 24.0).max(40.0));
    text(
        px,
        fonts,
        &title,
        18.0,
        x + back_w + 12.0,
        y + 4.0,
        text_col(),
        false,
    );
    y += 36.0;

    let Some(detail) = crate::review::track_bank_detail(track) else {
        text(
            px,
            fonts,
            "No line stored for this track.",
            13.0,
            x + 4.0,
            y,
            muted(),
            false,
        );
        return y + 40.0;
    };

    let best = fmt_lap(detail.best_ms);
    let ideal = fmt_lap(detail.ideal_ms);
    let gap = if detail.best_ms > 0 && detail.ideal_ms > 0 && detail.best_ms > detail.ideal_ms {
        format!("gap {}", fmt_lap(detail.best_ms - detail.ideal_ms))
    } else {
        "gap —".into()
    };
    let header = format!(
        "Best {best}   Ideal {ideal}   {gap}   {} laps merged",
        detail.merged_laps
    );
    text(px, fonts, &header, 12.0, x + 4.0, y, dim(), false);
    y += 22.0;

    text(px, fonts, "Best", 11.0, x + 4.0, y, accent(), false);
    text(
        px,
        fonts,
        "Ideal",
        11.0,
        x + 56.0,
        y,
        Color::from_rgba8(196, 112, 255, 255),
        false,
    );
    y += 18.0;

    let map_h = (w * 0.55).clamp(220.0, 360.0);
    hits.push(HitBox {
        id: Hit::TrackMap,
        x,
        y,
        w,
        h: map_h,
    });
    mxbo_motos_ui::paint_track_bank_map(
        px,
        fonts,
        x,
        y,
        w,
        map_h,
        &detail.poly,
        &detail.best_line,
        &detail.ideal_line,
        zoom,
        pan_x,
        pan_z,
    );
    y + map_h + 16.0
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

fn ellipsize(fonts: &Fonts, s: &str, size: f32, max_w: f32) -> String {
    if measure(fonts, s, size) <= max_w {
        return s.to_string();
    }
    let mut out = String::new();
    for ch in s.chars() {
        let next = format!("{out}{ch}…");
        if measure(fonts, &next, size) > max_w {
            if out.is_empty() {
                return "…".into();
            }
            out.push('…');
            return out;
        }
        out.push(ch);
    }
    out
}
