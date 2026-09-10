#![allow(unused_imports)]
use super::*;

pub(crate) fn telemetry_body_path(x: f32, y: f32, w: f32, h: f32) -> Option<Path> {
    let rl = 6.0f32.min(h * 0.4);
    let rr = h * 0.5;
    let k = 0.5522847498 * rr;
    let cx = x + w - rr;
    let cy = y + rr;
    let mut pb = PathBuilder::new();
    pb.move_to(x + rl, y);
    pb.line_to(cx, y);
    pb.cubic_to(cx + k, y, x + w, cy - k, x + w, cy);
    pb.cubic_to(x + w, cy + k, cx + k, y + h, cx, y + h);
    pb.line_to(x + rl, y + h);
    pb.quad_to(x, y + h, x, y + h - rl);
    pb.line_to(x, y + rl);
    pb.quad_to(x, y, x + rl, y);
    pb.close();
    pb.finish()
}

pub(crate) fn draw_telemetry(px: &mut Pixmap, fonts: &Fonts, s: &Snapshot, cfg: &HudConfig, sw: f32, sh: f32) {
    let r = cfg[WidgetId::Telemetry].rect;
    let x = r.x * sw;
    let y = r.y * sh;
    let w = (r.w * sw).max(220.0);
    let h = (r.h * sh).max(64.0);
    let a = bg_a(cfg[WidgetId::Telemetry].bg);
    if let Some(body) = telemetry_body_path(x, y, w, h) {
        if a > 0 {
            let mut paint = Paint::default();
            paint.set_color(Color::from_rgba8(10, 10, 10, a));
            paint.anti_alias = true;
            px.fill_path(&body, &paint, FillRule::Winding, Transform::identity(), None);
            let edge = ((a as u16 * 170) / 255).max(70) as u8;
            stroke_path(px, &body, Color::from_rgba8(42, 42, 46, edge), 1.0);
        }
    }

    let pad = (h * 0.10).clamp(6.0, 12.0);
    let show_dial = cfg.telemetry_dial;
    let show_traces = cfg.telemetry_draw_traces();
    let show_bars = cfg.telemetry_draw_bars();
    let dial_d = (h - pad * 2.0).max(40.0);
    let dial_r = dial_d * 0.5;
    let gap = (h * 0.06).clamp(4.0, 8.0);
    let well_y = y + pad;
    let well_h = h - pad * 2.0;
    let mut cursor = x + w - pad;
    let (dial_cx, dial_cy) = if show_dial {
        let cx = cursor - dial_r;
        cursor = cx - dial_r - gap;
        (cx, y + h * 0.5)
    } else {
        (0.0, 0.0)
    };
    let bar_n = [
        cfg.telemetry_bar_clutch,
        cfg.telemetry_bar_brake,
        cfg.telemetry_bar_throttle,
        cfg.telemetry_bar_steer,
    ]
    .iter()
    .filter(|on| **on)
    .count()
    .max(1) as f32;
    let bars_w = if bar_n > 3.0 {
        (h * 0.50).clamp(44.0, 76.0)
    } else {
        (h * 0.42).clamp(36.0, 58.0)
    };
    let bars_x = if show_bars {
        let left = x + pad;
        let bx = if show_traces {
            (cursor - bars_w).max(left)
        } else {
            left + ((cursor - left) - bars_w).max(0.0) * 0.5
        };
        if show_traces {
            cursor = bx - gap;
        }
        bx
    } else {
        0.0
    };
    let well_x = x + pad;
    let well_w = if show_traces { (cursor - well_x).max(48.0) } else { 0.0 };
    let well_r = 4.0f32.min(well_h * 0.12);

    if show_traces {
        if a > 0 {
            fill_round(px, well_x, well_y, well_w, well_h, well_r, Color::from_rgba8(8, 8, 10, a));
            if let Some(frame) = round_rect_path(well_x + 0.5, well_y + 0.5, well_w - 1.0, well_h - 1.0, well_r) {
                let edge = ((a as u16 * 120) / 255).max(50) as u8;
                stroke_path(px, &frame, Color::from_rgba8(42, 42, 46, edge), 1.0);
            }
        }
        let grid_a = if a > 0 { ((a as u16 * 70) / 255).max(28) as u8 } else { 40 };
        for i in 1..4 {
            let gy = well_y + well_h * (i as f32 / 4.0);
            if let Some(line) = rr(well_x + 4.0, gy, well_w - 8.0, 1.0) {
                fill_rect(px, line, Color::from_rgba8(42, 42, 46, grid_a));
            }
        }

        let ink_w = (h * 0.038).clamp(2.2, 3.6);
        let y_lo = well_y + 3.0;
        let y_hi = well_y + well_h - 3.0;
        crate::telemetry::with(|trace| {
            if trace.len() >= 2 {
                let n = trace.len();
                let mut thr_pts = Vec::with_capacity(n);
                let mut brk_pts = Vec::with_capacity(n);
                let mut str_pts = Vec::with_capacity(n);
                let span = well_w - 6.0;
                let amp = well_h - 6.0;
                let mid = well_y + well_h * 0.5;
                let steer_amp = amp * 0.5;
                for i in 0..n {
                    let sample = trace.get(i);
                    let tx = well_x + 3.0 + span * (i as f32 / (n - 1) as f32);
                    thr_pts.push((tx, y_hi - amp * sample.throttle));
                    brk_pts.push((tx, y_hi - amp * sample.brake));
                    str_pts.push((tx, mid - steer_amp * sample.steer));
                }
                if cfg.telemetry_trace_steer {
                    stroke_smooth_series(px, &str_pts, telemetry_steer_col(), ink_w * 0.92, y_lo, y_hi);
                }
                if cfg.telemetry_trace_brake {
                    stroke_smooth_series(px, &brk_pts, behind_col(), ink_w, y_lo, y_hi);
                }
                if cfg.telemetry_trace_throttle {
                    stroke_smooth_series(px, &thr_pts, ahead_col(), ink_w, y_lo, y_hi);
                }
            }
        });
    }

    let (thr, brk, clu) = crate::telemetry::inputs(s);
    let str = crate::telemetry::steer(s);
    if show_bars {
        let mut bars: Vec<(f32, Color, bool)> = Vec::with_capacity(4);
        if cfg.telemetry_bar_clutch {
            bars.push((clu, Color::from_rgba8(48, 52, 64, 255), false));
        }
        if cfg.telemetry_bar_brake {
            bars.push((brk, behind_col(), false));
        }
        if cfg.telemetry_bar_throttle {
            bars.push((thr, ahead_col(), false));
        }
        if cfg.telemetry_bar_steer {
            bars.push((str, telemetry_steer_col(), true));
        }
        let n = bars.len().max(1) as f32;
        let bar_gap = 4.0;
        let bar_w = ((bars_w - bar_gap * (n - 1.0)) / n).max(6.0);
        let bar_h = well_h * 0.78;
        let bar_y = well_y + well_h - bar_h;
        for (i, (level, fill, bipolar)) in bars.iter().enumerate() {
            let bx = bars_x + i as f32 * (bar_w + bar_gap);
            fill_round(px, bx, bar_y, bar_w, bar_h, 2.0, Color::from_rgba8(24, 24, 28, a.max(160)));
            if *bipolar {
                let mid = bar_y + bar_h * 0.5;
                let fh = (bar_h * 0.5 * level.abs()).max(if level.abs() > 0.02 { 3.0 } else { 0.0 });
                if fh > 0.5 {
                    let fy = if *level >= 0.0 { mid - fh } else { mid };
                    fill_round(px, bx, fy, bar_w, fh, 2.0, *fill);
                }
                if let Some(tick) = rr(bx + 1.0, mid - 0.5, (bar_w - 2.0).max(1.0), 1.0) {
                    fill_rect(px, tick, Color::from_rgba8(90, 90, 96, 200));
                }
                if level.abs() > 0.04 {
                    let label = format!("{:+}", (*level * 100.0).round() as i32);
                    let n = (h * 0.14).clamp(8.0, 12.0);
                    text_bold(px, fonts, &label, n, bx + bar_w * 0.5, bar_y - n - 1.0, text_col(), true);
                }
            } else {
                let fh = (bar_h * level).max(if *level > 0.02 { 3.0 } else { 0.0 });
                if fh > 0.5 {
                    fill_round(px, bx, bar_y + bar_h - fh, bar_w, fh, 2.0, *fill);
                }
                if *level > 0.04 {
                    let label = format!("{}", (*level * 100.0).round() as i32);
                    let n = (h * 0.14).clamp(8.0, 12.0);
                    text_bold(px, fonts, &label, n, bx + bar_w * 0.5, bar_y - n - 1.0, text_col(), true);
                }
            }
        }
    }

    if !show_dial {
        return;
    }
    let ring_w = (dial_r * 0.10).clamp(3.0, 6.0);
    stroke_circle(px, dial_cx, dial_cy, dial_r - ring_w * 0.5, Color::from_rgba8(42, 42, 46, a.max(140)), ring_w);
    let max_rpm = s.max_rpm.max(1) as f32;
    let rpm_n = (s.local_rpm.max(0) as f32 / max_rpm).clamp(0.0, 1.0);
    if rpm_n > 0.02 {
        draw_rpm_arc(px, dial_cx, dial_cy, dial_r - ring_w * 0.5, ring_w, rpm_n, accent());
    }

    let gear = if s.local_gear <= 0 {
        "N".to_string()
    } else {
        format!("{}", s.local_gear)
    };
    let speed = cfg.units.format_speed(s.local_speed);
    let unit = cfg.units.speed_label().to_ascii_lowercase();
    let gear_n = (dial_d * 0.40).clamp(18.0, 40.0);
    let speed_n = (dial_d * 0.20).clamp(10.0, 17.0);
    let unit_n = (dial_d * 0.10).clamp(6.0, 9.0);
    let stack_gap = (dial_d * 0.02).clamp(1.0, 2.0);
    let block = gear_n + stack_gap + speed_n;
    let gy = dial_cy - block * 0.52;
    text_bold(
        px,
        fonts,
        &gear,
        gear_n,
        dial_cx,
        gy,
        shift_gear_col(s, true, text_col()),
        true,
    );
    text_bold(px, fonts, &speed, speed_n, dial_cx, gy + gear_n + stack_gap, text_col(), true);
    text(px, fonts, &unit, unit_n, dial_cx, gy + gear_n + stack_gap + speed_n + 1.0, text_dim(), true);
}

pub(crate) fn draw_rpm_arc(px: &mut Pixmap, cx: f32, cy: f32, r: f32, width: f32, frac: f32, color: Color) {
    let start = -std::f32::consts::FRAC_PI_2 - 0.15;
    let sweep = std::f32::consts::PI * 1.15 * frac.clamp(0.0, 1.0);
    let steps = ((sweep.abs() * r).max(8.0) as usize).clamp(8, 48);
    let mut pb = PathBuilder::new();
    for i in 0..=steps {
        let a = start + sweep * (i as f32 / steps as f32);
        let px_ = cx + a.cos() * r;
        let py_ = cy + a.sin() * r;
        if i == 0 {
            pb.move_to(px_, py_);
        } else {
            pb.line_to(px_, py_);
        }
    }
    if let Some(path) = pb.finish() {
        stroke_path(px, &path, color, width);
    }
}
