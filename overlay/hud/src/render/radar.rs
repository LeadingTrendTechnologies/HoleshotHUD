#![allow(unused_imports)]
use super::*;

pub(crate) const RADAR_FWD_AHEAD: f32 = 3.0;

pub(crate) const RADAR_SIDE_LAT: f32 = 0.4;

pub(crate) const RADAR_REAR_FWD: f32 = -0.6;

pub(crate) const RADAR_STRETCH_M: f32 = 20.0;

pub(crate) const RADAR_RINGS_M: [f32; 3] = [3.0, 6.0, 12.0];

pub(crate) const RADAR_RING_OUTER_M: f32 = 12.0;

pub(crate) fn radar_range_m(cfg: &HudConfig) -> f32 {
    cfg.radar_range.clamp(crate::config::RADAR_RANGE_MIN, crate::config::RADAR_RANGE_MAX) as f32
}

pub(crate) fn radar_extents(range: f32) -> (f32, f32) {
    let rear = range.max(1.0);
    (rear, rear * 0.5)
}

pub(crate) fn radar_fit_range(range: f32) -> f32 {
    range.max(RADAR_RING_OUTER_M)
}

pub(crate) fn radar_rings_m() -> [f32; 3] {
    RADAR_RINGS_M
}

pub(crate) fn radar_in_view(fwd: f32, lat: f32, sides: bool, rear: bool, rear_m: f32, lat_m: f32) -> bool {
    if fwd < -rear_m || fwd > RADAR_FWD_AHEAD || lat.abs() > lat_m {
        return false;
    }
    let behind = fwd < RADAR_REAR_FWD;
    let beside = lat.abs() > RADAR_SIDE_LAT;
    (rear && behind) || (sides && beside)
}

pub(crate) fn radar_you_frac(rear_m: f32) -> f32 {
    RADAR_FWD_AHEAD / (RADAR_FWD_AHEAD + rear_m.max(1.0))
}

pub(crate) fn radar_to_screen(fwd: f32, lat: f32, ox: f32, oy: f32, sx: f32, sy: f32) -> (f32, f32) {
    (ox + lat * sx, oy - fwd * sy)
}

pub(crate) fn radar_blip_heat(dist: f32, range: f32) -> f32 {
    let far = (range * (8.0 / 12.0)).max(4.0);
    ((far - dist) / (far - 1.0)).clamp(0.0, 1.0)
}

pub(crate) fn radar_blip_radius(heat: f32, size: f32) -> f32 {
    (size * (0.020 + heat * 0.014)).clamp(7.0, 15.0)
}

pub(crate) fn radar_blip_color(heat: f32) -> Color {
    let r = 228.0 + 22.0 * heat;
    let g = 197.0 + (118.0 - 197.0) * heat;
    let b = 112.0 + (2.0 - 112.0) * heat;
    Color::from_rgba8(r as u8, g as u8, b as u8, 255)
}

pub(crate) fn draw_radar_blip(px: &mut Pixmap, x: f32, y: f32, rad: f32, heat: f32) {
    let col = radar_blip_color(heat);
    let (cr, cg, cb) = (
        (col.red() * 255.0) as u8,
        (col.green() * 255.0) as u8,
        (col.blue() * 255.0) as u8,
    );
    fill_circle(px, x, y, rad + 3.4, Color::from_rgba8(cr, cg, cb, 46));
    fill_circle(px, x, y, rad + 1.5, Color::from_rgba8(cr, cg, cb, 88));
    fill_circle(px, x, y, rad, col);
}

pub(crate) fn radar_fit_scale(w: f32, h: f32, ox: f32, oy: f32, x: f32, y: f32, inset: f32, rear_m: f32) -> f32 {
    let rear = rear_m.max(1.0);
    let to_side = (ox - x - inset).min(x + w - inset - ox).max(1.0);
    let to_bottom = (y + h - inset - oy).max(1.0);
    (to_side / rear)
        .min(to_bottom / rear)
        .max(0.5)
}

pub(crate) fn radar_ring_radius(meters: f32, scale: f32) -> f32 {
    meters * scale
}

pub(crate) fn radar_ring_lift(bg_pct: i32) -> f32 {
    // Night-ink at ≥80% eats a #38383C hairline. Lift the stroke as the plaque goes solid.
    ((bg_pct as f32 - 50.0) / 50.0).clamp(0.0, 1.0)
}

pub(crate) fn radar_ring_color(bg_pct: i32) -> Color {
    let t = radar_ring_lift(bg_pct);
    let v = 96.0 + 84.0 * t;
    Color::from_rgba8(v as u8, v as u8, (v + 6.0).min(255.0) as u8, (220.0 + 35.0 * t) as u8)
}

pub(crate) fn radar_ring_label_color(bg_pct: i32) -> Color {
    let t = radar_ring_lift(bg_pct);
    let v = 160.0 + 48.0 * t;
    Color::from_rgba8(v as u8, v as u8, (v + 6.0).min(255.0) as u8, 255)
}

pub(crate) fn radar_ring_stroke(size: f32, bg_pct: i32) -> f32 {
    let t = radar_ring_lift(bg_pct);
    (size * (0.0042 + 0.0022 * t)).clamp(1.15 + 0.25 * t, 1.35 + 0.55 * t)
}

pub(crate) fn radar_you_fill() -> Color {
    Color::from_rgba8(248, 248, 252, 255)
}

pub(crate) fn radar_you_ink() -> Color {
    Color::from_rgba8(14, 14, 16, 255)
}

pub(crate) fn radar_you_stroke(size: f32) -> f32 {
    (size * 0.018).clamp(2.4, 3.6)
}

pub(crate) fn radar_you_nose(ox: f32, body_y: f32) -> Option<Path> {
    let mut nose = PathBuilder::new();
    nose.move_to(ox, body_y - 4.0);
    nose.line_to(ox - 3.6, body_y + 1.0);
    nose.line_to(ox + 3.6, body_y + 1.0);
    nose.close();
    nose.finish()
}

pub(crate) fn draw_radar_you(px: &mut Pixmap, ox: f32, oy: f32, bw: f32, bh: f32, size: f32) {
    let x = ox - bw * 0.5;
    let y = oy - bh * 0.5;
    let ink = radar_you_ink();
    let cream = radar_you_fill();
    let sw = radar_you_stroke(size);
    let mut paint = Paint::default();
    paint.set_color(cream);
    paint.anti_alias = true;
    if let Some(body) = round_rect_path(x, y, bw, bh, 2.2) {
        stroke_path(px, &body, ink, sw);
        px.fill_path(&body, &paint, FillRule::Winding, Transform::identity(), None);
    }
    if let Some(nose) = radar_you_nose(ox, y) {
        stroke_path(px, &nose, ink, sw);
        px.fill_path(&nose, &paint, FillRule::Winding, Transform::identity(), None);
    }
}

pub(crate) fn draw_radar_range_rings(
    px: &mut Pixmap,
    fonts: &Fonts,
    ox: f32,
    oy: f32,
    scale: f32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    size: f32,
    bg_pct: i32,
) {
    let clip = round_rect_path(x, y, w, h, 6.0).and_then(|path| {
        let mut m = Mask::new(px.width(), px.height())?;
        m.fill_path(&path, FillRule::Winding, true, Transform::identity());
        Some(m)
    });
    let clip_ref = clip.as_ref();
    let ring = radar_ring_color(bg_pct);
    let stroke_w = radar_ring_stroke(size, bg_pct);
    let label_sz = (size * 0.048).clamp(9.0, 13.0);
    let show_labels = size >= 72.0;
    let rings = radar_rings_m();
    let mid_label = format!("{}", rings[1] as i32);
    let outer_label = format!("{}", rings[2] as i32);
    let labeled = [(rings[1], mid_label.as_str()), (rings[2], outer_label.as_str())];
    for meters in rings {
        let r = radar_ring_radius(meters, scale);
        let mut gap = 0.0;
        if show_labels {
            if let Some((_, label)) = labeled.iter().find(|(m, _)| (*m - meters).abs() < 0.01) {
                let tw = measure(fonts, label, label_sz);
                gap = ((tw * 0.5 + 5.0) / r).clamp(0.10, 0.42);
            }
        }
        stroke_circle_gap(px, ox, oy, r, gap, ring, stroke_w, clip_ref);
    }
    if !show_labels {
        return;
    }
    let label_col = radar_ring_label_color(bg_pct);
    for (meters, label) in labeled {
        let r = radar_ring_radius(meters, scale);
        let ty = oy + r - label_sz * 0.55;
        text_bold(px, fonts, label, label_sz, ox, ty, label_col, true);
    }
}

pub(crate) fn draw_radar(px: &mut Pixmap, fonts: &Fonts, s: &Snapshot, cfg: &HudConfig, sw: f32, sh: f32, age: f32) {
    // THESIS: distance is a graphic — hairline arcs on the bike, not empty glass.
    // OWN-WORLD: night-ink 6px plaque, hairline frame, white bike with a night-ink outline, heat blips, range circles that lift off a solid plaque.
    // STORY: rider glances behind and beside; 6 and 12 say how far without reading a table.
    // FIRST VIEWPORT: square plaque, bike in the upper third, three circles inside the glass, 6/12 in stroke gaps, blips on top.
    // FORM: Range Arcs, user-locked radar-arcs.png. No wedges, no sweep, no title bar.
    // FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
    let r = cfg[WidgetId::Radar].rect;
    let x = r.x * sw;
    let y = r.y * sh;
    let w = (r.w * sw).max(48.0);
    let h = (r.h * sh).max(48.0);
    let size = w.min(h);
    let pad = pad_from_size(size);

    let a = bg_a(cfg[WidgetId::Radar].bg);
    if a > 0 {
        fill_round(px, x, y, w, h, 6.0, Color::from_rgba8(14, 14, 16, a));
        if let Some(frame) = round_rect_path(x + 0.5, y + 0.5, w - 1.0, h - 1.0, 5.5) {
            let edge = ((a as u16 * 170) / 255).max(70) as u8;
            stroke_path(px, &frame, Color::from_rgba8(58, 58, 62, edge), 1.0);
        }
    }

    let range = radar_range_m(cfg);
    let (rear_m, lat_m) = radar_extents(range);
    let fit_m = radar_fit_range(range);
    let ox = x + w * 0.5;
    let usable_h = (h - pad * 2.0).max(16.0);
    let oy = y + pad + usable_h * radar_you_frac(fit_m);
    let inset = (size * 0.028).max(4.0);
    let scale = radar_fit_scale(w, h, ox, oy, x, y, inset, fit_m);

    if cfg.radar_rings {
        draw_radar_range_rings(px, fonts, ox, oy, scale, x, y, w, h, size, cfg[WidgetId::Radar].bg);
    }

    let bw = (size * 0.075).max(6.5);
    let bh = (size * 0.185).max(13.0);
    draw_radar_you(px, ox, oy, bw, bh, size);

    if s.has_telemetry == 0 {
        return;
    }

    let pred_x = s.local_x + s.local_vel_x * age;
    let pred_z = s.local_z + s.local_vel_z * age;
    let (fx, fz, rx, rz) = radar_axes(s);
    let focus = s.focus_race_num;
    let mut blips: Vec<(f32, f32, f32, i32, bool)> = Vec::new();
    for i in 0..s.rider_count.max(0) as usize {
        let rider = &s.riders[i];
        if rider.race_num == focus {
            continue;
        }
        let dx = rider.x - pred_x;
        let dz = rider.z - pred_z;
        let fwd = dx * fx + dz * fz;
        let lat = dx * rx + dz * rz;
        if !radar_same_stretch(s, rider.track_pos, RADAR_STRETCH_M.max(range)) {
            continue;
        }
        if !radar_in_view(fwd, lat, cfg.radar_sides, cfg.radar_rear, rear_m, lat_m) {
            continue;
        }
        let dist = (fwd * fwd + lat * lat).sqrt();
        blips.push((fwd, lat, dist, rider.race_num, rider.crashed != 0));
    }
    blips.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    for (fwd, lat, dist, race_num, crashed) in blips {
        let (bx, by) = radar_to_screen(fwd, lat, ox, oy, scale, scale);
        let heat = radar_blip_heat(dist, RADAR_RING_OUTER_M);
        let rad = radar_blip_radius(heat, size);
        draw_radar_blip(px, bx, by, rad, heat);
        draw_state_mark(px, fonts, bx, by, rad.max(6.5), rider_mark(s, race_num, crashed));
    }
    let local_num = if focus > 0 { focus } else { s.local_race_num };
    draw_state_mark(
        px,
        fonts,
        ox,
        oy,
        bw.max(bh) * 0.45,
        rider_mark(s, local_num, s.local_crashed != 0),
    );
}

pub(crate) fn radar_same_stretch(s: &Snapshot, other_pos: f32, max_m: f32) -> bool {
    if s.track_length <= 1.0 {
        return true;
    }
    let mut d = other_pos - s.local_track_pos;
    if d > 0.5 {
        d -= 1.0;
    }
    if d < -0.5 {
        d += 1.0;
    }
    (d * s.track_length).abs() <= max_m
}

pub(crate) fn radar_axes(s: &Snapshot) -> (f32, f32, f32, f32) {
    let speed2 = s.local_vel_x * s.local_vel_x + s.local_vel_z * s.local_vel_z;
    let (fx, fz) = if speed2 > 4.0 {
        let inv = 1.0 / speed2.sqrt();
        (s.local_vel_x * inv, s.local_vel_z * inv)
    } else {
        let yaw = if s.local_yaw.abs() > 6.5 {
            s.local_yaw.to_radians()
        } else {
            s.local_yaw
        };
        let (sin_y, cos_y) = yaw.sin_cos();
        (sin_y, cos_y)
    };
    (fx, fz, fz, -fx)
}
