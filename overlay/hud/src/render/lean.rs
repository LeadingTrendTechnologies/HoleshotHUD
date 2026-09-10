#![allow(unused_imports)]
use super::*;

pub(crate) fn lean_rider() -> Option<&'static Pixmap> {
    static RIDER: OnceLock<Option<Pixmap>> = OnceLock::new();
    RIDER
        .get_or_init(|| Pixmap::decode_png(include_bytes!("../../assets/lean-rider.png")).ok())
        .as_ref()
}

pub(crate) fn draw_lean(px: &mut Pixmap, fonts: &Fonts, s: &Snapshot, cfg: &HudConfig, sw: f32, sh: f32) {
    // THESIS: the rider is the angle — a rear-view MX figure, not a tach and not a toy trapezoid.
    // OWN-WORLD: night-ink 6px plaque, 1px hairline, white rider, orange skew 32° bug, 2px steer + pitch hairlines.
    // STORY: glance roll, nose, and bar. Spectate follows the camera; steer and pitch hide for other bikes.
    // FIRST VIEWPORT: white rider filling the glass, orange 32° TV bug on the hip, steer under the boots, pitch on the right while riding.
    // FORM: Figure (rear). Seed: user-locked original Figure. Do not infer sit/stand.
    // FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
    let r = cfg[WidgetId::Lean].rect;
    let x = r.x * sw;
    let y = r.y * sh;
    let w = (r.w * sw).max(64.0);
    let h = (r.h * sh).max(72.0);
    let a = bg_a(cfg[WidgetId::Lean].bg);
    if a > 0 {
        fill_round(px, x, y, w, h, 6.0, Color::from_rgba8(10, 10, 10, a));
        if let Some(frame) = round_rect_path(x + 0.5, y + 0.5, w - 1.0, h - 1.0, 5.5) {
            let edge = ((a as u16 * 170) / 255).max(70) as u8;
            stroke_path(px, &frame, Color::from_rgba8(42, 42, 46, edge), 1.0);
        }
    }

    let view = crate::lean::view(s);
    if cfg.lean_style == LeanStyle::Minimal {
        draw_lean_minimal(px, fonts, cfg, x, y, w, h, a, view);
        return;
    }
    let pad = (w.min(h) * 0.07).clamp(5.0, 10.0);
    let steer_h = if view.steer.is_some() {
        (h * 0.18).clamp(18.0, 30.0)
    } else {
        0.0
    };
    let pitch_w = if view.pitch.is_some() {
        (w * 0.16).clamp(18.0, 28.0)
    } else {
        0.0
    };
    let fig_x = x + pad;
    let fig_y = y + pad;
    let fig_w = (w - pad * 2.0 - pitch_w).max(36.0);
    let fig_h = (h - pad * 2.0 - steer_h).max(40.0);
    let needle = view.deg.clamp(-70.0, 70.0);
    draw_lean_rider(px, fig_x, fig_y, fig_w, fig_h, needle);

    let deg_txt = format!("{:.0}°", view.deg.round());
    let deg_fs = (w.min(h) * 0.11).clamp(11.0, 16.0);
    let tw = measure(fonts, &deg_txt, deg_fs);
    let bh = deg_fs + 7.0;
    let bw = (tw + 12.0).max(28.0);
    let skew = (bh * 0.28).clamp(3.0, 7.0);
    let bx = fig_x + fig_w * 0.58;
    let by = fig_y + fig_h * 0.38;
    fill_skew(px, bx, by, bw, bh, skew, accent());
    text(
        px,
        fonts,
        &deg_txt,
        deg_fs,
        bx + (bw - tw) * 0.5 + skew * 0.35,
        by + (bh - deg_fs) * 0.45,
        ink_on(accent()),
        false,
    );

    let over_game = cfg[WidgetId::Lean].bg < 40;
    if let Some(frac) = view.steer {
        let bx = fig_x;
        let bw = fig_w;
        let bar_y = y + h - pad - steer_h + steer_h * 0.28;
        draw_lean_track_h(px, bx, bar_y, bw, over_game, a);
        let mid = bx + bw * 0.5;
        let fill_w = (frac.abs() * bw * 0.5).max(if frac.abs() > 0.01 { 3.0 } else { 0.0 });
        if fill_w > 0.5 {
            let fx = if frac >= 0.0 { mid } else { mid - fill_w };
            fill_round(px, fx, bar_y - 1.0, fill_w, 4.0, 1.0, accent());
        }
        draw_lean_tick_v(px, mid, bar_y, over_game);
        let pct = format!("{:.0}%", (frac.abs() * 100.0).round());
        let fs = (steer_h * 0.42).clamp(9.0, 13.0);
        draw_lean_pct(px, fonts, &pct, fs, mid, bar_y + 8.0, over_game);
    }

    if let Some(frac) = view.pitch {
        let track_x = fig_x + fig_w + (pitch_w * 0.42).max(6.0);
        let bar_y = fig_y;
        let bar_h = fig_h;
        draw_lean_track_v(px, track_x, bar_y, bar_h, over_game, a);
        let mid = bar_y + bar_h * 0.5;
        let fill_h = (frac.abs() * bar_h * 0.5).max(if frac.abs() > 0.01 { 3.0 } else { 0.0 });
        if fill_h > 0.5 {
            let fy = if frac >= 0.0 { mid - fill_h } else { mid };
            fill_round(px, track_x - 1.0, fy, 4.0, fill_h, 1.0, accent());
        }
        draw_lean_tick_h(px, track_x, mid, over_game);
        let pct = format!("{:.0}%", (frac.abs() * 100.0).round());
        let fs = (pitch_w * 0.42).clamp(8.0, 12.0);
        let pct_x = fig_x + fig_w + pitch_w * 0.5;
        let pct_y = if view.steer.is_some() {
            y + h - pad - steer_h + steer_h * 0.28 + 8.0
        } else {
            fig_y + fig_h - fs - 1.0
        };
        draw_lean_pct(px, fonts, &pct, fs, pct_x, pct_y, over_game);
    }
}

pub(crate) fn draw_lean_minimal(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    a: u8,
    view: crate::lean::LeanView,
) {
    // Numbers, not a gyro. Huge orange lean is you. Cream pitch degrees. Steer stays a hairline.
    let over_game = cfg[WidgetId::Lean].bg < 40;
    let pad = (w.min(h) * 0.08).clamp(6.0, 12.0);
    let steer_h = if view.steer.is_some() {
        (h * 0.20).clamp(18.0, 32.0)
    } else {
        0.0
    };
    let body_h = (h - pad * 2.0 - steer_h).max(28.0);
    let has_pitch = view.pitch.is_some();
    let hero = signed_deg(view.deg);
    let hero_fs = (body_h * if has_pitch { 0.46 } else { 0.52 }).clamp(22.0, 64.0);
    let cx = x + w * 0.5;
    let hero_y = if has_pitch {
        y + pad + body_h * 0.08
    } else {
        y + pad + (body_h - hero_fs) * 0.5
    };
    let cream = Color::from_rgba8(248, 248, 252, 255);
    if over_game {
        let tw = measure(fonts, &hero, hero_fs);
        fill_night_pill(px, cx - tw * 0.5 - 10.0, hero_y - 5.0, tw + 20.0, hero_fs + 10.0);
    }
    text(px, fonts, &hero, hero_fs, cx, hero_y, accent(), true);

    if let Some(frac) = view.pitch {
        let pdeg = signed_deg(frac * 60.0);
        let pfs = (body_h * 0.20).clamp(12.0, 22.0);
        let py = hero_y + hero_fs + (body_h * 0.05).max(3.0);
        if over_game {
            let tw = measure(fonts, &pdeg, pfs);
            fill_night_pill(px, cx - tw * 0.5 - 6.0, py - 3.0, tw + 12.0, pfs + 6.0);
        }
        text(px, fonts, &pdeg, pfs, cx, py, cream, true);
    }

    if let Some(frac) = view.steer {
        let bw = (w - pad * 2.0).max(40.0);
        let bx = x + pad;
        let bar_y = y + h - pad - steer_h + steer_h * 0.22;
        draw_lean_track_h(px, bx, bar_y, bw, over_game, a);
        let mid = cx;
        let fill_w = (frac.abs() * bw * 0.5).max(if frac.abs() > 0.01 { 3.0 } else { 0.0 });
        if fill_w > 0.5 {
            let fx = if frac >= 0.0 { mid } else { mid - fill_w };
            fill_round(px, fx, bar_y - 1.0, fill_w, 4.0, 1.0, accent());
        }
        draw_lean_tick_v(px, mid, bar_y, over_game);
        let pct = format!("{:.0}%", (frac.abs() * 100.0).round());
        let fs = (steer_h * 0.42).clamp(9.0, 13.0);
        draw_lean_pct(px, fonts, &pct, fs, mid, bar_y + 8.0, over_game);
    }
}

pub(crate) fn lean_track_fill(over_game: bool, a: u8) -> Color {
    if over_game {
        Color::from_rgba8(236, 236, 240, 255)
    } else {
        Color::from_rgba8(58, 58, 62, a.max(200))
    }
}

pub(crate) fn lean_halo() -> Color {
    Color::from_rgba8(10, 10, 10, 230)
}

pub(crate) fn draw_lean_track_h(px: &mut Pixmap, x: f32, y: f32, w: f32, over_game: bool, a: u8) {
    if over_game {
        fill_round(px, x - 1.0, y - 2.0, w + 2.0, 6.0, 2.0, lean_halo());
    }
    fill_round(px, x, y, w, 2.0, 1.0, lean_track_fill(over_game, a));
}

pub(crate) fn draw_lean_track_v(px: &mut Pixmap, x: f32, y: f32, h: f32, over_game: bool, a: u8) {
    if over_game {
        fill_round(px, x - 2.0, y - 1.0, 6.0, h + 2.0, 2.0, lean_halo());
    }
    fill_round(px, x, y, 2.0, h, 1.0, lean_track_fill(over_game, a));
}

pub(crate) fn draw_lean_tick_v(px: &mut Pixmap, mid: f32, bar_y: f32, over_game: bool) {
    if over_game {
        fill_round(px, mid - 2.0, bar_y - 5.0, 4.0, 12.0, 1.0, lean_halo());
    }
    fill_round(
        px,
        mid - 1.0,
        bar_y - 3.0,
        2.0,
        8.0,
        0.8,
        Color::from_rgba8(248, 248, 252, 255),
    );
}

pub(crate) fn draw_lean_tick_h(px: &mut Pixmap, track_x: f32, mid: f32, over_game: bool) {
    if over_game {
        fill_round(px, track_x - 5.0, mid - 2.0, 12.0, 4.0, 1.0, lean_halo());
    }
    fill_round(
        px,
        track_x - 3.0,
        mid - 1.0,
        8.0,
        2.0,
        0.8,
        Color::from_rgba8(248, 248, 252, 255),
    );
}

pub(crate) fn draw_lean_pct(
    px: &mut Pixmap,
    fonts: &Fonts,
    pct: &str,
    fs: f32,
    cx: f32,
    y: f32,
    over_game: bool,
) {
    if over_game {
        let tw = measure(fonts, pct, fs);
        fill_night_pill(px, cx - tw * 0.5 - 6.0, y - 3.0, tw + 12.0, fs + 6.0);
    }
    text(
        px,
        fonts,
        pct,
        fs,
        cx,
        y,
        Color::from_rgba8(248, 248, 252, 255),
        true,
    );
}

pub(crate) fn draw_lean_rider(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, deg: f32) {
    let Some(src) = lean_rider() else {
        return;
    };
    let iw = src.width() as f32;
    let ih = src.height() as f32;
    if iw < 1.0 || ih < 1.0 {
        return;
    }
    let scale = ((w * 0.92) / iw).min((h * 0.98) / ih).max(0.01);
    let dest_x = x + w * 0.5;
    let dest_y = y + h - 4.0;
    let t = Transform::from_translate(dest_x, dest_y)
        .pre_concat(Transform::from_rotate(deg))
        .pre_concat(Transform::from_scale(scale, scale))
        .pre_concat(Transform::from_translate(-iw * 0.5, -ih * 0.92));
    let mut paint = PixmapPaint::default();
    paint.quality = FilterQuality::Bilinear;
    px.draw_pixmap(0, 0, src.as_ref(), &paint, t, None);
}
