use crate::shm::{cstr, Snapshot};
use fontdue::Font;
use tiny_skia::{Color, FillRule, LineCap, LineJoin, Paint, Path, PathBuilder, Pixmap, Rect, Stroke, Transform};

fn accent() -> Color { Color::from_rgba8(255, 148, 48, 255) }
fn text_col() -> Color { Color::from_rgba8(228, 228, 230, 255) }
fn text_dim() -> Color { Color::from_rgba8(132, 132, 138, 255) }
fn panel_col() -> Color { Color::from_rgba8(10, 10, 10, 200) }
fn header_col() -> Color { Color::from_rgba8(16, 16, 16, 230) }
fn row_alt() -> Color { Color::from_rgba8(24, 22, 22, 28) }
fn local_row() -> Color { Color::from_rgba8(160, 72, 28, 64) }
fn track_col() -> Color { Color::from_rgba8(236, 236, 240, 255) }
fn fill_col() -> Color { Color::from_rgba8(10, 8, 8, 168) }
fn rider_col() -> Color { Color::from_rgba8(255, 220, 36, 255) }
fn local_col() -> Color { Color::from_rgba8(255, 148, 48, 255) }
fn crash_col() -> Color { Color::from_rgba8(220, 64, 64, 255) }
fn ahead_col() -> Color { Color::from_rgba8(92, 196, 96, 255) }
fn behind_col() -> Color { Color::from_rgba8(232, 96, 96, 255) }

pub struct Fonts {
    pub ui: Font,
}

impl Fonts {
    pub fn load() -> Option<Self> {
        for path in [
            r"C:\Windows\Fonts\segoeui.ttf",
            r"C:\Windows\Fonts\arial.ttf",
            r"C:\Windows\Fonts\tahoma.ttf",
        ] {
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok(ui) = Font::from_bytes(bytes, fontdue::FontSettings::default()) {
                    return Some(Self { ui });
                }
            }
        }
        None
    }
}

pub fn draw(px: &mut Pixmap, fonts: &Fonts, snap: Option<&Snapshot>, w: u32, h: u32, age: f32, restart_hint: bool) {
    px.fill(Color::TRANSPARENT);
    if restart_hint {
        let cx = w as f32 * 0.5;
        if let Some(r) = rr(cx - 280.0, 8.0, 560.0, 40.0) {
            fill_rect(px, r, panel_col());
        }
        text(px, fonts, "Restart MX Bikes once so the HUD stays on top while you ride", 13.0, cx, 18.0, accent(), true);
    }
    let Some(s) = snap else {
        let cx = w as f32 * 0.5;
        if let Some(r) = rr(cx - 220.0, 36.0, 440.0, 52.0) {
            fill_rect(px, r, panel_col());
        }
        text(px, fonts, "MXBO overlay", 16.0, cx, 42.0, accent(), true);
        text(px, fonts, "Start MX Bikes in borderless / windowed", 14.0, cx, 64.0, text_col(), true);
        return;
    };

    let sw = w as f32;
    let sh = h as f32;
    if s.show_standings != 0 {
        draw_standings(px, fonts, s, sw, sh);
    }
    if s.show_relative != 0 {
        draw_relative(px, fonts, s, sw, sh);
    }
    if s.show_map != 0 {
        draw_map(px, fonts, s, sw, sh, age);
    }
}

fn rr(x: f32, y: f32, w: f32, h: f32) -> Option<Rect> {
    Rect::from_xywh(x, y, w, h)
}

fn fill_rect(px: &mut Pixmap, r: Rect, c: Color) {
    let mut p = Paint::default();
    p.set_color(c);
    p.anti_alias = true;
    px.fill_rect(r, &p, Transform::identity(), None);
}

fn panel(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, title: &str, fonts: &Fonts) {
    if let Some(r) = rr(x, y, w, h) {
        fill_rect(px, r, panel_col());
    }
    if let Some(r) = rr(x, y, w, 28.0) {
        fill_rect(px, r, header_col());
    }
    if let Some(r) = rr(x, y + 26.0, w, 2.0) {
        fill_rect(px, r, accent());
    }
    text(px, fonts, title, 13.0, x + 12.0, y + 8.0, accent(), false);
}

fn text(px: &mut Pixmap, fonts: &Fonts, s: &str, size: f32, mut x: f32, y: f32, color: Color, center: bool) {
    if center {
        x -= measure(fonts, s, size) * 0.5;
    }
    let rgba = [
        (color.red() * 255.0) as u8,
        (color.green() * 255.0) as u8,
        (color.blue() * 255.0) as u8,
        (color.alpha() * 255.0) as u8,
    ];
    for ch in s.chars() {
        let (metrics, bitmap) = fonts.ui.rasterize(ch, size);
        blit(px, &bitmap, metrics.width, metrics.height, x + metrics.xmin as f32, y + size - metrics.ymin as f32 - metrics.height as f32, rgba);
        x += metrics.advance_width;
    }
}

fn measure(fonts: &Fonts, s: &str, size: f32) -> f32 {
    s.chars().map(|ch| fonts.ui.metrics(ch, size).advance_width).sum()
}

fn blit(px: &mut Pixmap, bitmap: &[u8], gw: usize, gh: usize, x: f32, y: f32, rgba: [u8; 4]) {
    let pw = px.width() as i32;
    let ph = px.height() as i32;
    let data = px.data_mut();
    for row in 0..gh {
        for col in 0..gw {
            let cov = bitmap[row * gw + col];
            if cov < 8 {
                continue;
            }
            let dx = x as i32 + col as i32;
            let dy = y as i32 + row as i32;
            if dx < 0 || dy < 0 || dx >= pw || dy >= ph {
                continue;
            }
            let i = ((dy * pw + dx) * 4) as usize;
            let a = (rgba[3] as u16 * cov as u16) / 255;
            let ia = 255 - a;
            data[i] = ((data[i] as u16 * ia + rgba[0] as u16 * a) / 255) as u8;
            data[i + 1] = ((data[i + 1] as u16 * ia + rgba[1] as u16 * a) / 255) as u8;
            data[i + 2] = ((data[i + 2] as u16 * ia + rgba[2] as u16 * a) / 255) as u8;
            data[i + 3] = data[i + 3].saturating_add(a as u8);
        }
    }
}

fn format_gap(ms: i32, laps: i32) -> String {
    if laps != 0 {
        return format!("+{laps}L");
    }
    if ms <= 0 {
        return "---".into();
    }
    let sec = ms as f32 / 1000.0;
    if sec >= 60.0 {
        let m = (sec / 60.0) as i32;
        format!("+{m}:{:04.1}", sec - m as f32 * 60.0)
    } else {
        format!("+{sec:.3}")
    }
}

fn draw_standings(px: &mut Pixmap, fonts: &Fonts, s: &Snapshot, sw: f32, sh: f32) {
    let r = s.standings_rect;
    let x = r.x * sw;
    let y = r.y * sh;
    let w = r.w * sw;
    let rows = s.standing_count.max(0) as usize;
    let max_rows = s.standings_rows.max(3) as usize;
    let vis = rows.max(1).min(max_rows);
    let h = (32.0 + 22.0 + vis as f32 * 24.0 + 10.0).min(r.h * sh);
    panel(px, x, y, w, h, "STANDINGS", fonts);
    if rows == 0 {
        text(px, fonts, "Waiting for race data", 13.0, x + 12.0, y + 40.0, text_dim(), false);
        return;
    }
    text(px, fonts, "P", 11.0, x + 12.0, y + 36.0, text_dim(), false);
    text(px, fonts, "#", 11.0, x + 36.0, y + 36.0, text_dim(), false);
    text(px, fonts, "NAME", 11.0, x + 62.0, y + 36.0, text_dim(), false);
    let gap_x = x + w - 12.0;
    text(px, fonts, "GAP", 11.0, gap_x - measure(fonts, "GAP", 11.0), y + 36.0, text_dim(), false);

    let focus = s.focus_race_num;
    let n = rows.min(MAX_SAFE);
    let mut start = 0;
    if n > max_rows {
        let mut fi = 0;
        for i in 0..n {
            if s.standings[i].race_num == focus {
                fi = i;
                break;
            }
        }
        start = fi.saturating_sub(max_rows / 2).min(n - max_rows);
    }
    let end = (start + max_rows).min(n);
    for (vis_i, i) in (start..end).enumerate() {
        let row = &s.standings[i];
        let ry = y + 54.0 + vis_i as f32 * 24.0;
        let is_focus = row.race_num == focus;
        if is_focus {
            if let Some(rrt) = rr(x + 2.0, ry - 2.0, w - 4.0, 22.0) {
                fill_rect(px, rrt, local_row());
            }
        } else if vis_i % 2 == 1 {
            if let Some(rrt) = rr(x + 2.0, ry - 2.0, w - 4.0, 22.0) {
                fill_rect(px, rrt, row_alt());
            }
        }
        let col = if is_focus { accent() } else { text_col() };
        let name = cstr(&row.name);
        let gap = if row.state == 1 {
            "DNS".into()
        } else if row.state == 3 {
            "OUT".into()
        } else if row.state == 4 {
            "DSQ".into()
        } else if row.pit != 0 {
            "PIT".into()
        } else if row.position == 1 {
            "---".into()
        } else {
            format_gap(row.gap_ms, row.gap_laps)
        };
        text(px, fonts, &format!("{}", row.position), 13.0, x + 12.0, ry, text_dim(), false);
        text(px, fonts, &format!("{}", row.race_num), 13.0, x + 36.0, ry, col, false);
        text(px, fonts, &name, 13.0, x + 62.0, ry, col, false);
        let gw = measure(fonts, &gap, 13.0);
        text(px, fonts, &gap, 13.0, gap_x - gw, ry, if is_focus { accent() } else { text_dim() }, false);
    }
}

const MAX_SAFE: usize = 40;

fn draw_relative(px: &mut Pixmap, fonts: &Fonts, s: &Snapshot, sw: f32, sh: f32) {
    let r = s.relative;
    let x = r.x * sw;
    let y = r.y * sh;
    let w = r.w * sw;
    let n = s.rider_count.max(0) as usize;
    let focus = s.focus_race_num;
    let mut focus_pos = s.local_track_pos;
    let mut have = s.has_telemetry != 0;
    for i in 0..n {
        if s.riders[i].race_num == focus {
            focus_pos = s.riders[i].track_pos;
            have = true;
        }
    }
    let side = s.relative_count.max(1) as usize;
    let vis = if have { (side * 2 + 1).min(n.max(1)) } else { 1 };
    let h = (32.0 + vis as f32 * 24.0 + 12.0).min(r.h * sh);
    panel(px, x, y, w, h, "RELATIVE", fonts);
    if !have || n == 0 {
        text(px, fonts, "Waiting for positions", 13.0, x + 12.0, y + 40.0, text_dim(), false);
        return;
    }

    fn wrap(other: f32, self_p: f32) -> f32 {
        let mut d = other - self_p;
        if d > 0.5 {
            d -= 1.0;
        }
        if d < -0.5 {
            d += 1.0;
        }
        d
    }

    let mut order: Vec<(usize, f32)> = (0..n)
        .map(|i| (i, wrap(s.riders[i].track_pos, focus_pos)))
        .collect();
    order.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let self_idx = order
        .iter()
        .position(|(i, _)| s.riders[*i].race_num == focus)
        .unwrap_or(0);
    let mut show = Vec::new();
    for k in (1..=side).rev() {
        show.push((self_idx + n - k) % n);
    }
    show.push(self_idx);
    for k in 1..=side {
        show.push((self_idx + k) % n);
    }

    let track_len = if s.track_length > 1.0 { s.track_length } else { 1.0 };
    let speed = s.local_speed.max(4.0);
    for (vis_i, oi) in show.iter().enumerate() {
        let (ri, wrapped) = order[*oi];
        let rider = &s.riders[ri];
        let ry = y + 40.0 + vis_i as f32 * 24.0;
        let is_self = rider.race_num == focus;
        if is_self {
            if let Some(rrt) = rr(x + 2.0, ry - 2.0, w - 4.0, 22.0) {
                fill_rect(px, rrt, local_row());
            }
        }
        let name = if is_self {
            "YOU".into()
        } else {
            cstr(&rider.name)
        };
        let (gap, gcol) = if is_self {
            ("---".into(), accent())
        } else {
            let est = (wrapped * track_len) / speed;
            let g = if est < 0.0 {
                format!("-{:.2}", est.abs())
            } else {
                format!("+{est:.2}")
            };
            (g, if wrapped >= 0.0 { ahead_col() } else { behind_col() })
        };
        let col = if is_self { accent() } else { text_col() };
        text(px, fonts, &format!("{}", rider.race_num), 13.0, x + 12.0, ry, col, false);
        text(px, fonts, &name, 13.0, x + 48.0, ry, col, false);
        let gw = measure(fonts, &gap, 13.0);
        text(px, fonts, &gap, 13.0, x + w - 12.0 - gw, ry, gcol, false);
    }
}

fn draw_map(px: &mut Pixmap, fonts: &Fonts, s: &Snapshot, sw: f32, sh: f32, age: f32) {
    let r = s.map;
    let x = r.x * sw;
    let y = r.y * sh;
    let w = r.w * sw;
    let h = r.h * sh;
    let n = s.poly_count.max(0) as usize;
    text(px, fonts, "MAP", 13.0, x + 8.0, y + 6.0, accent(), false);
    let tname = cstr(&s.track_name);
    if !tname.is_empty() {
        let tw = measure(fonts, &tname, 12.0);
        text(px, fonts, &tname, 12.0, x + w - 8.0 - tw, y + 7.0, text_dim(), false);
    }
    if n < 2 {
        text(px, fonts, "No track map", 13.0, x + w * 0.5, y + h * 0.5, text_dim(), true);
        return;
    }

    let mut min_x = s.poly[0].x;
    let mut max_x = min_x;
    let mut min_z = s.poly[0].z;
    let mut max_z = min_z;
    for p in s.poly.iter().take(n) {
        min_x = min_x.min(p.x);
        max_x = max_x.max(p.x);
        min_z = min_z.min(p.z);
        max_z = max_z.max(p.z);
    }
    let mut dx = (max_x - min_x).max(8.0);
    let mut dz = (max_z - min_z).max(8.0);
    let pad = 0.10;
    let usable_w = w * (1.0 - 2.0 * pad);
    let usable_h = (h - 28.0) * (1.0 - 2.0 * pad);
    let scale = (usable_w / dx).min(usable_h / dz);
    dx = max_x - min_x;
    dz = max_z - min_z;
    let used_w = dx * scale;
    let used_h = dz * scale;
    let ox = x + (w - used_w) * 0.5;
    let oy = y + 28.0 + ((h - 28.0) - used_h) * 0.5;
    let to_px = |wx: f32, wz: f32| -> (f32, f32) {
        (ox + (wx - min_x) * scale, oy + (max_z - wz) * scale)
    };

    let mut pb = PathBuilder::new();
    let (sx, sy) = to_px(s.poly[0].x, s.poly[0].z);
    pb.move_to(sx, sy);
    for p in s.poly.iter().take(n).skip(1) {
        let (px_, py_) = to_px(p.x, p.z);
        pb.line_to(px_, py_);
    }
    pb.close();
    if let Some(path) = pb.finish() {
        let mut fill = Paint::default();
        fill.set_color(fill_col());
        fill.anti_alias = true;
        px.fill_path(&path, &fill, FillRule::EvenOdd, Transform::identity(), None);
        stroke_path(px, &path, track_col(), 2.4);
    }

    if n >= 2 && s.sf_meters >= 0.0 {
        draw_sf(px, s, n, to_px);
    }

    let pred_x = s.local_x + s.local_vel_x * age;
    let pred_z = s.local_z + s.local_vel_z * age;
    let focus = s.focus_race_num;
    let mut local_pos = 0;
    for st in s.standings.iter().take(s.standing_count.max(0) as usize) {
        if st.race_num == focus {
            local_pos = st.position;
        }
    }

    for i in 0..s.rider_count.max(0) as usize {
        let rider = &s.riders[i];
        if s.has_telemetry != 0 && rider.race_num == focus {
            continue;
        }
        let (hx, hy) = to_px(rider.x, rider.z);
        let c = if rider.crashed != 0 { crash_col() } else { rider_col() };
        dot(px, hx, hy, 4.0, c);
    }
    if s.has_telemetry != 0 {
        let (hx, hy) = to_px(pred_x, pred_z);
        dot(px, hx, hy, 8.5, local_col());
        if local_pos > 0 {
            text(
                px,
                fonts,
                &format!("{local_pos}"),
                11.0,
                hx,
                hy - 6.0,
                Color::from_rgba8(12, 12, 16, 255),
                true,
            );
        }
    }
}

fn draw_sf(px: &mut Pixmap, s: &Snapshot, n: usize, to_px: impl Fn(f32, f32) -> (f32, f32)) {
    let mut dist = 0.0f32;
    let mut d = vec![0.0; n];
    for i in 1..n {
        let dx = s.poly[i].x - s.poly[i - 1].x;
        let dz = s.poly[i].z - s.poly[i - 1].z;
        dist += (dx * dx + dz * dz).sqrt();
        d[i] = dist;
    }
    if dist < 1.0 {
        return;
    }
    let mut target = s.sf_meters;
    if target > dist {
        target %= dist;
    }
    for i in 1..n {
        if d[i] < target {
            continue;
        }
        let span = d[i] - d[i - 1];
        let u = if span > 0.001 { (target - d[i - 1]) / span } else { 0.0 };
        let wx = s.poly[i - 1].x + (s.poly[i].x - s.poly[i - 1].x) * u;
        let wz = s.poly[i - 1].z + (s.poly[i].z - s.poly[i - 1].z) * u;
        let (x0, y0) = to_px(s.poly[i - 1].x, s.poly[i - 1].z);
        let (x1, y1) = to_px(s.poly[i].x, s.poly[i].z);
        let (hx, hy) = to_px(wx, wz);
        let tx = x1 - x0;
        let ty = y1 - y0;
        let len = (tx * tx + ty * ty).sqrt().max(1.0e-4);
        let pxn = -ty / len;
        let pyn = tx / len;
        let half = 5.0;
        let mut pb = PathBuilder::new();
        pb.move_to(hx - pxn * half, hy - pyn * half);
        pb.line_to(hx + pxn * half, hy + pyn * half);
        if let Some(path) = pb.finish() {
            stroke_path(px, &path, accent(), 2.0);
        }
        break;
    }
}

fn stroke_path(px: &mut Pixmap, path: &Path, color: Color, width: f32) {
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;
    let stroke = Stroke {
        width,
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Stroke::default()
    };
    px.stroke_path(path, &paint, &stroke, Transform::identity(), None);
}

fn dot(px: &mut Pixmap, x: f32, y: f32, r: f32, color: Color) {
    let mut pb = PathBuilder::new();
    pb.push_circle(x, y, r + 1.4);
    if let Some(path) = pb.finish() {
        let mut p = Paint::default();
        p.set_color(Color::from_rgba8(8, 8, 8, 240));
        p.anti_alias = true;
        px.fill_path(&path, &p, FillRule::Winding, Transform::identity(), None);
    }
    let mut pb = PathBuilder::new();
    pb.push_circle(x, y, r);
    if let Some(path) = pb.finish() {
        let mut p = Paint::default();
        p.set_color(color);
        p.anti_alias = true;
        px.fill_path(&path, &p, FillRule::Winding, Transform::identity(), None);
    }
}
