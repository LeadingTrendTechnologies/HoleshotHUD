#![allow(unused_imports)]
use super::*;

/// Orange Δ mark. Bundled Exo 2 has no Delta glyph, so this is drawn.
pub(crate) fn fill_delta_mark(px: &mut Pixmap, x: f32, y: f32, h: f32, color: Color) -> f32 {
    let w = h * 0.92;
    let skew = h * 0.22;
    let mut pb = PathBuilder::new();
    pb.move_to(x + skew * 0.12, y + h);
    pb.line_to(x + w + skew * 0.88, y + h);
    pb.line_to(x + w * 0.5 + skew * 0.5, y);
    pb.close();
    if let Some(path) = pb.finish() {
        fill_path(px, &path, color);
    }
    w + skew * 0.5
}

pub(crate) fn draw_delta(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    sw: f32,
    sh: f32,
    view: crate::delta::DeltaView,
) {
    // THESIS: one glance — am I up or down vs my best at this spot on the lap.
    // OWN-WORLD: Hair — orange Δ letter, ExtraBold Italic delta, 2px center-zero line. No plaque.
    // STORY: after one decent lap the line is live; it is not the in-game ghost.
    // FIRST VIEWPORT: Δ -0.840 over a hairline that fills left of a center tick; BEST and LAST cap the ends.
    // FORM: Hair · approved .impeccable/mocks/delta-hair.png
    let r = cfg[WidgetId::Delta].rect;
    let x = r.x * sw;
    let y = r.y * sh;
    let w = r.w * sw;
    let h = r.h * sh;
    if w < 160.0 || h < 44.0 {
        return;
    }
    let a = bg_a(cfg[WidgetId::Delta].bg);
    if a > 0 {
        fill_round(px, x, y, w, h, 6.0, Color::from_rgba8(10, 10, 10, a));
    }
    let pad_x = (w * 0.04).clamp(8.0, 16.0);
    let num_fs = (h * 0.40).clamp(18.0, 44.0);
    let mark_fs = (num_fs * 0.36).clamp(11.0, 18.0);
    let times_fs = (h * 0.18).clamp(13.0, 20.0);
    let cap_fs = (h * 0.155).clamp(12.0, 17.0);
    let line_gap = (h * 0.09).clamp(6.0, 12.0);
    let foot_gap = (h * 0.08).clamp(6.0, 10.0);
    let line_h = 2.0;
    let stack_h = num_fs + line_gap + line_h + foot_gap + times_fs;
    let gy = y + (h - stack_h) * 0.5;
    let label = if view.ready && view.has_delta {
        format_delta_ms(view.delta_ms)
    } else if view.ready {
        "—".into()
    } else if view.recording {
        "REC".into()
    } else {
        "SET LAP".into()
    };
    let col = if !view.ready || !view.has_delta {
        text_dim()
    } else if view.delta_ms > 0 {
        behind_col()
    } else if view.delta_ms < 0 {
        ahead_col()
    } else {
        text_col()
    };
    let gap = (num_fs * 0.18).clamp(6.0, 10.0);
    let mark_h = mark_fs;
    let mark_w = mark_h * 0.92 + mark_h * 0.22 * 0.5;
    let num_w = measure(fonts, &label, num_fs);
    let cluster = mark_w + gap + num_w;
    let left = x + (w - cluster) * 0.5;
    let mark_y = gy + (num_fs - mark_h) * 0.78;
    fill_delta_mark(px, left, mark_y, mark_h, accent());
    text(px, fonts, &label, num_fs, left + mark_w + gap, gy, col, false);

    let line_x = x + pad_x;
    let line_w = (w - pad_x * 2.0).max(48.0);
    let line_y = gy + num_fs + line_gap;
    let mid = line_x + line_w * 0.5;
    if let Some(track) = rr(line_x, line_y, line_w, line_h) {
        fill_rect(px, track, Color::from_rgba8(58, 58, 64, 220));
    }
    if view.ready && view.has_delta {
        let t = (view.delta_ms as f32 / crate::delta::BAR_RANGE_MS as f32).clamp(-1.0, 1.0);
        let fill_w = (line_w * 0.5 * t.abs()).max(if t.abs() > 0.02 { 3.0 } else { 0.0 });
        if fill_w > 0.5 {
            let fill_col = if view.delta_ms > 0 {
                Color::from_rgba8(255, 64, 72, 255)
            } else {
                Color::from_rgba8(48, 220, 88, 255)
            };
            let fx = if t < 0.0 { mid - fill_w } else { mid };
            if let Some(fill) = rr(fx, line_y - 1.0, fill_w, 4.0) {
                fill_rect(px, fill, fill_col);
            }
        }
    } else if view.recording && view.cover > 0 {
        let fill_w = (line_w * (view.cover as f32 / 100.0)).max(3.0);
        if let Some(fill) = rr(line_x, line_y - 1.0, fill_w, 4.0) {
            fill_rect(px, fill, Color::from_rgba8(255, 148, 48, 200));
        }
    }
    if let Some(tick) = rr(mid - 0.5, line_y - 4.0, 1.0, 10.0) {
        fill_rect(px, tick, Color::from_rgba8(200, 200, 204, 230));
    }

    let times_y = line_y + line_h + foot_gap;
    let cap_y = times_y + (times_fs - cap_fs) * 0.45;
    let pills = cfg[WidgetId::Delta].bg < 40;
    if view.recording {
        let hint = "complete a flying lap";
        let hw = measure(fonts, hint, times_fs);
        let hx = mid - hw * 0.5;
        if pills {
            fill_night_pill(
                px,
                hx - 7.0,
                times_y - 3.0,
                hw + 14.0,
                times_fs + 6.0,
            );
        }
        text(px, fonts, hint, times_fs, mid, times_y, text_dim(), true);
    } else {
        if view.ref_lap_ms > 0 {
            let best = format_lap(view.ref_lap_ms);
            draw_delta_lap_chip(
                px, fonts, if cfg.delta_session { "SESSION" } else { "BEST" }, &best, line_x, cap_y, times_y, cap_fs, times_fs, pills,
                text_dim(), text_col(),
            );
        }
        if view.new_best {
            let pb = format_lap(if view.last_lap_ms > 0 {
                view.last_lap_ms
            } else {
                view.ref_lap_ms
            });
            let cap = "NEW BEST";
            let cap_w = measure(fonts, cap, cap_fs) + 6.0;
            let chip_w = cap_w + measure(fonts, &pb, times_fs);
            let lx = line_x + line_w - chip_w;
            draw_delta_lap_chip(
                px, fonts, cap, &pb, lx, cap_y, times_y, cap_fs, times_fs, pills,
                accent(), accent(),
            );
        } else if view.last_lap_ms > 0 {
            let last = format_lap(view.last_lap_ms);
            let cap_w = measure(fonts, "LAST", cap_fs) + 6.0;
            let last_w = cap_w + measure(fonts, &last, times_fs);
            let lx = line_x + line_w - last_w;
            draw_delta_lap_chip(
                px, fonts, "LAST", &last, lx, cap_y, times_y, cap_fs, times_fs, pills,
                text_dim(), text_col(),
            );
        }
    }
}

/// Night-ink stadium behind BEST / LAST when the panel is too thin to read on the game.
pub(crate) fn draw_delta_lap_chip(
    px: &mut Pixmap,
    fonts: &Fonts,
    cap: &str,
    time: &str,
    x: f32,
    cap_y: f32,
    time_y: f32,
    cap_fs: f32,
    times_fs: f32,
    pill: bool,
    cap_col: Color,
    time_col: Color,
) {
    let cap_w = measure(fonts, cap, cap_fs) + 6.0;
    let tw = cap_w + measure(fonts, time, times_fs);
    if pill {
        let pad_x = 8.0;
        let pad_y = 3.0;
        let top = cap_y.min(time_y);
        let bot = (time_y + times_fs).max(cap_y + cap_fs);
        let ph = (bot - top) + pad_y * 2.0;
        fill_night_pill(px, x - pad_x, top - pad_y, tw + pad_x * 2.0, ph);
    }
    text(px, fonts, cap, cap_fs, x, cap_y, cap_col, false);
    text(px, fonts, time, times_fs, x + cap_w, time_y, time_col, false);
}

pub(crate) fn format_delta_ms(ms: i32) -> String {
    if ms == 0 {
        return "0.000".into();
    }
    let sign = if ms > 0 { '+' } else { '-' };
    let sec = ms.unsigned_abs() as f32 / 1000.0;
    if sec >= 60.0 {
        let m = (sec / 60.0) as i32;
        format!("{sign}{m}:{:04.1}", sec - m as f32 * 60.0)
    } else {
        format!("{sign}{sec:.3}")
    }
}
