use crate::config::{DotLabel, HudConfig, RelField, StField};
use crate::shm::{cstr, Snapshot};
use fontdue::Font;
use tiny_skia::{
    Color, FillRule, GradientStop, LineCap, LineJoin, LinearGradient, Paint, Path, PathBuilder,
    Pixmap, Point as SkPoint, RadialGradient, Rect, SpreadMode, Stroke, Transform,
};

fn accent() -> Color { Color::from_rgba8(255, 148, 48, 255) }
fn text_col() -> Color { Color::from_rgba8(228, 228, 230, 255) }
fn text_dim() -> Color { Color::from_rgba8(132, 132, 138, 255) }
fn panel_col() -> Color { Color::from_rgba8(10, 10, 10, 200) }
fn track_col() -> Color { Color::from_rgba8(236, 236, 240, 255) }
fn fill_col() -> Color { Color::from_rgba8(10, 8, 8, 168) }
fn you_col() -> Color { Color::from_rgba8(48, 214, 232, 255) }
fn ahead_col() -> Color { Color::from_rgba8(92, 196, 96, 255) }
fn behind_col() -> Color { Color::from_rgba8(232, 96, 96, 255) }

pub struct Fonts {
    pub ui: Font,
    pub icons: Font,
}

impl Fonts {
    pub fn load() -> Option<Self> {
        let icons = Font::from_bytes(
            include_bytes!("../fonts/fa-solid-900.ttf").as_slice(),
            fontdue::FontSettings::default(),
        )
        .ok()?;
        for path in [
            r"C:\Windows\Fonts\segoeui.ttf",
            r"C:\Windows\Fonts\arial.ttf",
            r"C:\Windows\Fonts\tahoma.ttf",
        ] {
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok(ui) = Font::from_bytes(bytes, fontdue::FontSettings::default()) {
                    return Some(Self { ui, icons });
                }
            }
        }
        None
    }
}

pub fn draw(px: &mut Pixmap, fonts: &Fonts, snap: Option<&Snapshot>, cfg: &HudConfig, w: u32, h: u32, age: f32, restart_hint: bool, settings_hint: bool) {
    px.fill(if settings_hint {
        Color::from_rgba8(0, 0, 0, 1)
    } else {
        Color::TRANSPARENT
    });
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
        draw_standings(px, fonts, s, cfg, sw, sh);
    }
    if s.show_relative != 0 {
        draw_relative(px, fonts, s, cfg, sw, sh);
    }
    if s.show_map != 0 {
        draw_map(px, fonts, s, cfg, sw, sh, age);
    }
    if cfg.show_minimap {
        draw_minimap(px, fonts, s, cfg, sw, sh, age);
    }
    if cfg.show_radar {
        draw_radar(px, s, cfg, sw, sh, age);
    }
    if settings_hint {
        draw_layout(px, s, cfg, sw, sh);
        text(
            px,
            fonts,
            "Ctrl+drag to move  ·  drag corners / edges to resize",
            12.0,
            16.0,
            sh - 22.0,
            text_dim(),
            false,
        );
    }
}

fn rr(x: f32, y: f32, w: f32, h: f32) -> Option<Rect> {
    Rect::from_xywh(x, y, w, h)
}

fn draw_layout(px: &mut Pixmap, s: &Snapshot, cfg: &HudConfig, sw: f32, sh: f32) {
    if s.show_standings != 0 {
        layout_box(px, s.standings_rect.x * sw, s.standings_rect.y * sh, s.standings_rect.w * sw, s.standings_rect.h * sh);
    }
    if s.show_relative != 0 {
        layout_box(px, s.relative.x * sw, s.relative.y * sh, s.relative.w * sw, s.relative.h * sh);
    }
    if s.show_map != 0 {
        layout_box(px, s.map.x * sw, s.map.y * sh, s.map.w * sw, s.map.h * sh);
    }
    if cfg.show_radar {
        layout_box(px, cfg.radar.x * sw, cfg.radar.y * sh, cfg.radar.w * sw, cfg.radar.h * sh);
    }
    if cfg.show_minimap {
        let rw = cfg.minimap.w * sw;
        let rh = cfg.minimap.h * sh;
        let d = rw.min(rh);
        let cx = cfg.minimap.x * sw + rw * 0.5;
        let cy = cfg.minimap.y * sh + rh * 0.5;
        layout_box(px, cx - d * 0.5, cy - d * 0.5, d, d);
    }
}

fn layout_box(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32) {
    let mut pb = PathBuilder::new();
    pb.move_to(x, y);
    pb.line_to(x + w, y);
    pb.line_to(x + w, y + h);
    pb.line_to(x, y + h);
    pb.close();
    if let Some(path) = pb.finish() {
        stroke_path(px, &path, Color::from_rgba8(255, 148, 48, 200), 1.4);
    }
    for (hx, hy) in [
        (x, y),
        (x + w, y),
        (x, y + h),
        (x + w, y + h),
        (x + w * 0.5, y),
        (x + w * 0.5, y + h),
        (x, y + h * 0.5),
        (x + w, y + h * 0.5),
    ] {
        if let Some(r) = rr(hx - 5.0, hy - 5.0, 10.0, 10.0) {
            fill_rect(px, r, Color::from_rgba8(8, 8, 10, 230));
        }
        if let Some(r) = rr(hx - 4.0, hy - 4.0, 8.0, 8.0) {
            fill_rect(px, r, Color::from_rgba8(255, 148, 48, 255));
        }
    }
}

pub(crate) fn fill_rect(px: &mut Pixmap, r: Rect, c: Color) {
    let mut p = Paint::default();
    p.set_color(c);
    p.anti_alias = true;
    px.fill_rect(r, &p, Transform::identity(), None);
}

fn bg_a(pct: i32) -> u8 {
    ((pct.clamp(0, 100) as f32 / 100.0) * 255.0).round() as u8
}

fn stripe(focus: bool, vis_i: usize, bg: i32, alt: bool) -> Option<Color> {
    if bg <= 0 {
        return None;
    }
    let t = bg as f32 / 100.0;
    if focus {
        Some(Color::from_rgba8(160, 72, 28, (64.0 * t).round() as u8))
    } else if alt && vis_i % 2 == 1 {
        Some(Color::from_rgba8(24, 22, 22, (28.0 * t).round() as u8))
    } else {
        None
    }
}

fn panel(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, title: &str, fonts: &Fonts, bg: i32) {
    let a = bg_a(bg);
    if a > 0 {
        if let Some(r) = rr(x, y, w, h) {
            fill_rect(px, r, Color::from_rgba8(10, 10, 10, a));
        }
        let ha = ((a as u16 * 230) / 200).min(255) as u8;
        if let Some(r) = rr(x, y, w, 28.0) {
            fill_rect(px, r, Color::from_rgba8(16, 16, 16, ha));
        }
        if let Some(r) = rr(x, y + 26.0, w, 2.0) {
            fill_rect(px, r, Color::from_rgba8(255, 148, 48, a));
        }
    }
    text(px, fonts, title, 13.0, x + 12.0, y + 8.0, accent(), false);
}

pub(crate) fn text(px: &mut Pixmap, fonts: &Fonts, s: &str, size: f32, mut x: f32, y: f32, color: Color, center: bool) {
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

pub(crate) fn measure(fonts: &Fonts, s: &str, size: f32) -> f32 {
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

fn format_lap(ms: i32) -> String {
    if ms <= 0 {
        return "--".into();
    }
    let sec = ms as f32 / 1000.0;
    if sec >= 60.0 {
        let m = (sec / 60.0) as i32;
        format!("{m}:{:06.3}", sec - m as f32 * 60.0)
    } else {
        format!("{sec:.3}")
    }
}

fn standing_status(row: &crate::shm::Standing) -> Option<&'static str> {
    match row.state {
        1 => Some("DNS"),
        3 => Some("OUT"),
        4 => Some("DSQ"),
        _ if row.pit != 0 => Some("PIT"),
        _ => None,
    }
}

fn standing_of(s: &Snapshot, race_num: i32) -> Option<&crate::shm::Standing> {
    s.standings
        .iter()
        .take(s.standing_count.max(0) as usize)
        .find(|st| st.race_num == race_num)
}

fn rider_crashed(s: &Snapshot, race_num: i32) -> bool {
    if s.focus_race_num == race_num && s.local_crashed != 0 {
        return true;
    }
    s.riders
        .iter()
        .take(s.rider_count.max(0) as usize)
        .any(|r| r.race_num == race_num && r.crashed != 0)
}

fn format_penalty(ms: i32) -> String {
    if ms <= 0 {
        "---".into()
    } else {
        format_lap(ms)
    }
}

fn dim_rel(is_self: bool) -> Color {
    if is_self {
        accent()
    } else {
        text_dim()
    }
}

fn interval_text(s: &Snapshot, row: &crate::shm::Standing) -> String {
    if row.position <= 1 {
        return "---".into();
    }
    let n = s.standing_count.max(0) as usize;
    let Some(ahead) = s.standings[..n]
        .iter()
        .find(|st| st.position == row.position - 1)
    else {
        return "---".into();
    };
    let lap_delta = row.gap_laps - ahead.gap_laps;
    if lap_delta != 0 {
        format_gap(0, lap_delta)
    } else {
        format_gap(row.gap_ms - ahead.gap_ms, 0)
    }
}

fn standings_cols(cfg: &HudConfig) -> Vec<StField> {
    cfg.standings_cols()
}

fn layout_st_cols(cfg: &HudConfig, cols: &[StField], panel_w: f32) -> Vec<(StField, f32, f32)> {
    layout_cols(cols, panel_w, |c| c.width(cfg) as f32)
}

fn st_col_header(col: StField) -> &'static str {
    match col {
        StField::Pos => "P",
        StField::Num => "#",
        StField::Name => "NAME",
        StField::Laps => "LP",
        StField::Best => "BEST",
        StField::Status => "ST",
        StField::Gap => "GAP",
        StField::Interval => "INT",
        StField::Bike => "BIKE",
        StField::Penalty => "PEN",
        StField::Crashed => "CR",
    }
}

fn st_col_right(col: StField) -> bool {
    matches!(
        col,
        StField::Laps | StField::Best | StField::Status | StField::Gap | StField::Interval | StField::Penalty | StField::Crashed
    )
}

fn layout_cols<T: Copy>(cols: &[T], panel_w: f32, width: impl Fn(T) -> f32) -> Vec<(T, f32, f32)> {
    let pad = 12.0;
    let inner = (panel_w - pad * 2.0).max(40.0);
    let mut ws: Vec<f32> = cols.iter().map(|c| width(*c).max(16.0)).collect();
    let sum: f32 = ws.iter().sum();
    if sum > inner && sum > 0.0 {
        let s = inner / sum;
        for w in &mut ws {
            *w *= s;
        }
    }
    let mut x = pad;
    cols.iter()
        .zip(ws)
        .map(|(col, w)| {
            let item = (*col, x, w);
            x += w;
            item
        })
        .collect()
}

fn draw_standings(px: &mut Pixmap, fonts: &Fonts, s: &Snapshot, cfg: &HudConfig, sw: f32, sh: f32) {
    let r = s.standings_rect;
    let x = r.x * sw;
    let y = r.y * sh;
    let w = r.w * sw;
    let rows = s.standing_count.max(0) as usize;
    let max_rows = s.standings_rows.max(3) as usize;
    let vis = rows.max(1).min(max_rows);
    let h = (32.0 + 22.0 + vis as f32 * 24.0 + 10.0).min(r.h * sh);
    panel(px, x, y, w, h, "STANDINGS", fonts, cfg.st_bg);
    if rows == 0 {
        text(px, fonts, "Waiting for race data", 13.0, x + 12.0, y + 40.0, text_dim(), false);
        return;
    }
    let cols = standings_cols(cfg);
    let laid = layout_st_cols(cfg, &cols, w);
    for (col, cx, cw) in &laid {
        let header = st_col_header(*col);
        let hx = if st_col_right(*col) {
            x + *cx + *cw - measure(fonts, header, 11.0)
        } else {
            x + *cx
        };
        text(px, fonts, header, 11.0, hx, y + 36.0, text_dim(), false);
    }

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
        if let Some(c) = stripe(is_focus, vis_i, cfg.st_bg, true) {
            if let Some(rrt) = rr(x + 2.0, ry - 2.0, w - 4.0, 22.0) {
                fill_rect(px, rrt, c);
            }
        }
        let col = if is_focus { accent() } else { text_col() };
        let dim = if is_focus { accent() } else { text_dim() };
        let name = cstr(&row.name);
        let status = standing_status(row);
        let gap = if cfg.st_status {
            if row.position == 1 {
                "---".into()
            } else {
                format_gap(row.gap_ms, row.gap_laps)
            }
        } else if let Some(st) = status {
            st.to_string()
        } else if row.position == 1 {
            "---".into()
        } else {
            format_gap(row.gap_ms, row.gap_laps)
        };
        for (kind, cx, cw) in &laid {
            let (val, color) = match kind {
                StField::Pos => (format!("{}", row.position), dim),
                StField::Num => (format!("{}", row.race_num), col),
                StField::Name => (name.clone(), col),
                StField::Laps => (format!("{}", row.num_laps.max(0)), dim),
                StField::Best => (format_lap(row.best_lap_ms), dim),
                StField::Status => (status.unwrap_or("").to_string(), dim),
                StField::Gap => (gap.clone(), dim),
                StField::Interval => (interval_text(s, row), dim),
                StField::Bike => (cstr(&row.bike), col),
                StField::Penalty => (format_penalty(row.penalty_ms), dim),
                StField::Crashed => {
                    if row.crashed != 0 || rider_crashed(s, row.race_num) {
                        ("CRASH".into(), behind_col())
                    } else {
                        ("".into(), dim)
                    }
                }
            };
            let tx = if st_col_right(*kind) {
                x + *cx + *cw - measure(fonts, &val, 13.0)
            } else {
                x + *cx
            };
            text(px, fonts, &val, 13.0, tx, ry, color, false);
        }
    }
}

const MAX_SAFE: usize = 40;

fn draw_relative(px: &mut Pixmap, fonts: &Fonts, s: &Snapshot, cfg: &HudConfig, sw: f32, sh: f32) {
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
    panel(px, x, y, w, h, "RELATIVE", fonts, cfg.rel_bg);
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
        if let Some(c) = stripe(is_self, vis_i, cfg.rel_bg, false) {
            if let Some(rrt) = rr(x + 2.0, ry - 2.0, w - 4.0, 22.0) {
                fill_rect(px, rrt, c);
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
        let pos = s
            .standings
            .iter()
            .take(s.standing_count.max(0) as usize)
            .find(|st| st.race_num == rider.race_num)
            .map(|st| st.position)
            .unwrap_or(0);
        let cols = cfg.relative_cols();
        let laid = layout_rel_cols(cfg, &cols, w);
        for (kind, cx, cw) in &laid {
            let st = standing_of(s, rider.race_num);
            let (val, color) = match kind {
                RelField::Pos => (
                    if pos > 0 { format!("{pos}") } else { "--".into() },
                    col,
                ),
                RelField::Num => (format!("{}", rider.race_num), col),
                RelField::Name => (name.clone(), col),
                RelField::Gap => (gap.clone(), gcol),
                RelField::Bike => (st.map(|r| cstr(&r.bike)).unwrap_or_default(), col),
                RelField::Penalty => (st.map(|r| format_penalty(r.penalty_ms)).unwrap_or_else(|| "---".into()), dim_rel(is_self)),
                RelField::Interval => (st.map(|r| interval_text(s, r)).unwrap_or_else(|| "---".into()), dim_rel(is_self)),
                RelField::Crashed => {
                    let crash = rider.crashed != 0 || st.is_some_and(|r| r.crashed != 0);
                    if crash {
                        ("CRASH".into(), behind_col())
                    } else {
                        ("".into(), dim_rel(is_self))
                    }
                }
            };
            let tx = if matches!(kind, RelField::Gap | RelField::Interval | RelField::Penalty | RelField::Crashed) {
                x + *cx + *cw - measure(fonts, &val, 13.0)
            } else {
                x + *cx
            };
            text(px, fonts, &val, 13.0, tx, ry, color, false);
        }
    }
}

fn layout_rel_cols(cfg: &HudConfig, cols: &[RelField], panel_w: f32) -> Vec<(RelField, f32, f32)> {
    layout_cols(cols, panel_w, |c| c.width(cfg) as f32)
}

fn draw_map(px: &mut Pixmap, fonts: &Fonts, s: &Snapshot, cfg: &HudConfig, sw: f32, sh: f32, age: f32) {
    let r = s.map;
    let x = r.x * sw;
    let y = r.y * sh;
    let w = r.w * sw;
    let h = r.h * sh;
    if cfg.map_bg > 0 {
        if let Some(rect) = rr(x, y, w, h) {
            fill_rect(px, rect, Color::from_rgba8(10, 10, 10, bg_a(cfg.map_bg)));
        }
    }
    let n = s.poly_count.max(0) as usize;
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
    let usable_h = h * (1.0 - 2.0 * pad);
    let scale = (usable_w / dx).min(usable_h / dz);
    dx = max_x - min_x;
    dz = max_z - min_z;
    let used_w = dx * scale;
    let used_h = dz * scale;
    let ox = x + (w - used_w) * 0.5;
    let oy = y + (h - used_h) * 0.5;
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
    let track_px = (8.0 * scale).clamp(5.5, 26.0);
    if let Some(path) = pb.finish() {
        let mut fill = Paint::default();
        fill.set_color(fill_col());
        fill.anti_alias = true;
        px.fill_path(&path, &fill, FillRule::EvenOdd, Transform::identity(), None);
        stroke_path(px, &path, Color::from_rgba8(18, 16, 16, 240), track_px + 3.0);
        stroke_path(px, &path, track_col(), track_px);
    }

    if n >= 2 && s.sf_meters >= 0.0 && cfg.map_sf {
        draw_sf(px, s, n, to_px, track_px);
    }
    if cfg.map_arrows {
        draw_track_arrows(px, s, n, to_px, track_px, None, false);
    }

    let pred_x = s.local_x + s.local_vel_x * age;
    let pred_z = s.local_z + s.local_vel_z * age;
    let focus = s.focus_race_num;
    let leader = leader_num(s);
    let other_r = if cfg.map_numbers { 6.8 } else { 4.6 };

    if cfg.map_others {
        for i in 0..s.rider_count.max(0) as usize {
            let rider = &s.riders[i];
            if s.has_telemetry != 0 && rider.race_num == focus {
                continue;
            }
            let (hx, hy) = to_px(rider.x, rider.z);
            draw_rider_dot(
                px,
                fonts,
                hx,
                hy,
                other_r,
                rider_fill(rider.race_num),
                rider_dot_num(s, rider.race_num, cfg.map_dot),
                cfg.map_numbers,
                false,
            );
            draw_rider_overhead(px, fonts, s, rider.race_num, hx, hy, other_r, focus, leader, cfg.map_crown, cfg.map_place);
            draw_state_mark(px, fonts, hx, hy, other_r, rider_mark(s, rider.race_num, rider.crashed != 0));
        }
    }
    if s.has_telemetry != 0 {
        let (hx, hy) = to_px(pred_x, pred_z);
        let local_num = if focus > 0 { focus } else { s.local_race_num };
        draw_rider_dot(
            px,
            fonts,
            hx,
            hy,
            8.5,
            you_col(),
            rider_dot_num(s, local_num, cfg.map_dot),
            cfg.map_numbers,
            true,
        );
        if cfg.map_crown && leader > 0 && focus == leader {
            crown_over_dot(px, fonts, hx, hy, 8.5);
        }
        draw_state_mark(px, fonts, hx, hy, 8.5, rider_mark(s, focus, s.local_crashed != 0));
    }
}

fn draw_minimap(px: &mut Pixmap, fonts: &Fonts, s: &Snapshot, cfg: &HudConfig, sw: f32, sh: f32, age: f32) {
    let r = cfg.minimap;
    let x = r.x * sw;
    let y = r.y * sh;
    let w = r.w * sw;
    let h = r.h * sh;
    let size = w.min(h).max(48.0);
    let cx = x + w * 0.5;
    let cy = y + h * 0.5;
    let dim = size.round().clamp(48.0, 900.0) as u32;
    let sdim = dim as f32;
    let left = cx - sdim * 0.5;
    let top = cy - sdim * 0.5;
    let Some(mut mini) = Pixmap::new(dim, dim) else {
        return;
    };
    if cfg.mini_bg > 0 {
        fill_circle(
            &mut mini,
            sdim * 0.5,
            sdim * 0.5,
            sdim * 0.5 - 0.5,
            Color::from_rgba8(18, 18, 20, bg_a(cfg.mini_bg)),
        );
    }

    let n = s.poly_count.max(0) as usize;
    if n < 2 {
        text(px, fonts, "No track", 12.0, cx, cy - 8.0, text_dim(), true);
        return;
    }

    let pred_x = s.local_x + s.local_vel_x * age;
    let pred_z = s.local_z + s.local_vel_z * age;
    let (fx, fz, rx, rz, scale, origin_x, origin_z, north_up) = if s.has_telemetry != 0 {
        let (fx, fz) = track_forward(s, n, pred_x, pred_z).unwrap_or_else(|| {
            let (f, z, _, _) = radar_axes(s);
            (f, z)
        });
        let radius_m = 40.0;
        (fx, fz, fz, -fx, (sdim * 0.46) / radius_m, pred_x, pred_z, true)
    } else {
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
        let usable = sdim * 0.68;
        let dx = (max_x - min_x).max(8.0);
        let dz = (max_z - min_z).max(8.0);
        let scale = (usable / dx).min(usable / dz);
        (0.0, 1.0, 1.0, 0.0, scale, (min_x + max_x) * 0.5, (min_z + max_z) * 0.5, false)
    };
    let mc = sdim * 0.5;
    let to_px = |wx: f32, wz: f32| -> (f32, f32) {
        let dx = wx - origin_x;
        let dz = wz - origin_z;
        let along = dx * fx + dz * fz;
        let right = dx * rx + dz * rz;
        if north_up {
            (mc + right * scale, mc - along * scale)
        } else {
            (mc + dx * scale, mc - dz * scale)
        }
    };

    let mut pb = PathBuilder::new();
    let (sx, sy) = to_px(s.poly[0].x, s.poly[0].z);
    pb.move_to(sx, sy);
    for p in s.poly.iter().take(n).skip(1) {
        let (px_, py_) = to_px(p.x, p.z);
        pb.line_to(px_, py_);
    }
    let track_px = if north_up {
        (sdim * 0.058).clamp(14.0, 24.0)
    } else {
        (10.0 * scale).clamp(6.5, 18.0)
    };
    if let Some(path) = pb.finish() {
        stroke_path(&mut mini, &path, Color::from_rgba8(8, 8, 10, 220), track_px + 5.0);
        stroke_path(&mut mini, &path, Color::from_rgba8(248, 248, 252, 255), track_px);
    }

    if n >= 2 && s.sf_meters >= 0.0 && cfg.mini_sf {
        draw_sf(&mut mini, s, n, to_px, track_px);
    }
    if cfg.mini_arrows {
        draw_track_arrows(&mut mini, s, n, to_px, track_px, Some((mc, sdim)), north_up);
    }

    let focus = s.focus_race_num;
    let leader = leader_num(s);
    let other_r = (sdim * 0.028).clamp(7.0, 11.0);
    let local_r = other_r * 1.22;

    if cfg.mini_others {
        for i in 0..s.rider_count.max(0) as usize {
            let rider = &s.riders[i];
            if s.has_telemetry != 0 && rider.race_num == focus {
                continue;
            }
            let (hx, hy) = to_px(rider.x, rider.z);
            if (hx - mc) * (hx - mc) + (hy - mc) * (hy - mc) > sdim * sdim * 0.27 {
                continue;
            }
            numbered_dot(
                &mut mini,
                fonts,
                hx,
                hy,
                other_r,
                rider_fill(rider.race_num),
                rider_dot_num(s, rider.race_num, cfg.mini_dot),
                cfg.mini_numbers,
                false,
            );
            draw_rider_overhead(&mut mini, fonts, s, rider.race_num, hx, hy, other_r, focus, leader, cfg.mini_crown, cfg.mini_place);
            draw_state_mark(&mut mini, fonts, hx, hy, other_r, rider_mark(s, rider.race_num, rider.crashed != 0));
        }
    }

    if s.has_telemetry != 0 {
        let (hx, hy) = to_px(pred_x, pred_z);
        let vx = s.local_vel_x;
        let vz = s.local_vel_z;
        for i in (1..5).rev() {
            let t = i as f32 * 0.07;
            let (tx, ty) = to_px(pred_x - vx * t, pred_z - vz * t);
            let a = 40u8.saturating_mul(5 - i as u8);
            fill_circle(&mut mini, tx, ty, local_r * (0.55 + i as f32 * 0.04), Color::from_rgba8(48, 214, 232, a));
        }
        numbered_dot(
            &mut mini,
            fonts,
            hx,
            hy,
            local_r,
            you_col(),
            rider_dot_num(s, if focus > 0 { focus } else { s.local_race_num }, cfg.mini_dot),
            cfg.mini_numbers,
            true,
        );
        let local_num = if focus > 0 { focus } else { s.local_race_num };
        if cfg.mini_crown && leader > 0 && local_num == leader {
            crown_over_dot(&mut mini, fonts, hx, hy, local_r);
        }
        draw_state_mark(&mut mini, fonts, hx, hy, local_r, rider_mark(s, local_num, s.local_crashed != 0));
    }

    blit_circle(px, &mini, left, top);
}

fn rider_fill(num: i32) -> Color {
    let n = num.max(0) as u32;
    let mut hue = (n as f32 * 137.508) % 360.0;
    if (168.0..214.0).contains(&hue) {
        hue = (hue + 52.0) % 360.0;
    }
    let sat = 0.68 + ((n.wrapping_mul(17)) % 18) as f32 * 0.01;
    let val = 0.90 + ((n.wrapping_mul(13)) % 10) as f32 * 0.01;
    hsv(hue, sat, val)
}

fn hsv(h: f32, s: f32, v: f32) -> Color {
    let c = v * s;
    let h6 = (h / 60.0).rem_euclid(6.0);
    let x = c * (1.0 - (h6 % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match h6 as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    Color::from_rgba8(
        ((r + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((g + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((b + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        255,
    )
}

fn rider_dot_num(s: &Snapshot, race_num: i32, mode: DotLabel) -> i32 {
    match mode {
        DotLabel::Number => race_num,
        DotLabel::Position => standing_pos(s, race_num),
    }
}

fn standing_pos(s: &Snapshot, race_num: i32) -> i32 {
    s.standings
        .iter()
        .take(s.standing_count.max(0) as usize)
        .find(|st| st.race_num == race_num)
        .map(|st| st.position)
        .unwrap_or(0)
}

fn draw_rider_dot(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    r: f32,
    fill: Color,
    num: i32,
    show_num: bool,
    you: bool,
) {
    if show_num {
        numbered_dot(px, fonts, x, y, r, fill, num, true, you);
    } else {
        dot_body(px, x, y, r, fill, you);
    }
}

fn dot_body(px: &mut Pixmap, x: f32, y: f32, r: f32, fill: Color, you: bool) {
    if you {
        fill_circle(px, x, y, r + 4.2, Color::from_rgba8(255, 255, 255, 255));
        fill_circle(px, x, y, r + 2.4, Color::from_rgba8(8, 8, 10, 255));
    } else {
        fill_circle(px, x, y, r + 1.6, Color::from_rgba8(8, 8, 10, 240));
    }
    fill_circle(px, x, y, r, fill);
}

fn numbered_dot(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    r: f32,
    fill: Color,
    num: i32,
    show_num: bool,
    you: bool,
) {
    dot_body(px, x, y, r, fill, you);
    if !show_num || num <= 0 {
        return;
    }
    let label = format!("{num}");
    let size = if num >= 100 {
        r * 0.82
    } else if num >= 10 {
        r * 0.98
    } else {
        r * 1.12
    };
    let Some((min_x, min_y, max_x, max_y)) = ink_bounds(fonts, &label, size) else {
        return;
    };
    text(
        px,
        fonts,
        &label,
        size,
        x - (min_x + max_x) * 0.5,
        y - (min_y + max_y) * 0.5,
        Color::from_rgba8(16, 16, 20, 255),
        false,
    );
}

fn ink_bounds(fonts: &Fonts, s: &str, size: f32) -> Option<(f32, f32, f32, f32)> {
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    let mut pen = 0.0;
    let mut any = false;
    for ch in s.chars() {
        let m = fonts.ui.metrics(ch, size);
        if m.width > 0 && m.height > 0 {
            any = true;
            let gx0 = pen + m.xmin as f32;
            let gy0 = size - m.ymin as f32 - m.height as f32;
            min_x = min_x.min(gx0);
            min_y = min_y.min(gy0);
            max_x = max_x.max(gx0 + m.width as f32);
            max_y = max_y.max(gy0 + m.height as f32);
        }
        pen += m.advance_width;
    }
    any.then_some((min_x, min_y, max_x, max_y))
}

fn leader_num(s: &Snapshot) -> i32 {
    s.standings
        .iter()
        .take(s.standing_count.max(0) as usize)
        .find(|st| st.position == 1)
        .map(|st| st.race_num)
        .unwrap_or(0)
}

fn icon(px: &mut Pixmap, fonts: &Fonts, ch: char, size: f32, mut x: f32, y: f32, color: Color, center: bool) {
    if center {
        x -= fonts.icons.metrics(ch, size).advance_width * 0.5;
    }
    let rgba = [
        (color.red() * 255.0) as u8,
        (color.green() * 255.0) as u8,
        (color.blue() * 255.0) as u8,
        (color.alpha() * 255.0) as u8,
    ];
    let (metrics, bitmap) = fonts.icons.rasterize(ch, size);
    if metrics.width == 0 || metrics.height == 0 {
        return;
    }
    blit(
        px,
        &bitmap,
        metrics.width,
        metrics.height,
        x + metrics.xmin as f32,
        y + size - metrics.ymin as f32 - metrics.height as f32,
        rgba,
    );
}

fn icon_over_dot(px: &mut Pixmap, fonts: &Fonts, x: f32, y: f32, r: f32, ch: char, col: Color) {
    let size = (r * 1.85).clamp(11.0, 20.0);
    let metrics = fonts.icons.metrics(ch, size);
    let gap = (r * 0.22).max(2.5);
    let cy = y - r - gap - size + metrics.ymin as f32;
    icon(px, fonts, ch, size, x + 0.8, cy + 0.8, Color::from_rgba8(8, 8, 10, 220), true);
    icon(px, fonts, ch, size, x, cy, col, true);
}

fn crown_over_dot(px: &mut Pixmap, fonts: &Fonts, x: f32, y: f32, r: f32) {
    icon_over_dot(px, fonts, x, y, r, '\u{f521}', Color::from_rgba8(255, 196, 48, 255));
}

fn draw_rider_overhead(
    px: &mut Pixmap,
    fonts: &Fonts,
    s: &Snapshot,
    race_num: i32,
    x: f32,
    y: f32,
    r: f32,
    focus: i32,
    leader: i32,
    show_crown: bool,
    show_place: bool,
) {
    let mine = standing_pos(s, if focus > 0 { focus } else { s.local_race_num });
    let theirs = standing_pos(s, race_num);
    if theirs == 1 || (leader > 0 && race_num == leader) {
        if show_crown {
            crown_over_dot(px, fonts, x, y, r);
        }
        return;
    }
    if !show_place || mine <= 0 || theirs <= 0 || theirs == mine {
        return;
    }
    if theirs < mine {
        icon_over_dot(px, fonts, x, y, r, '\u{f077}', ahead_col());
    } else {
        icon_over_dot(px, fonts, x, y, r, '\u{f078}', behind_col());
    }
}

#[derive(Clone, Copy)]
enum RiderMark {
    None,
    Crash,
    Pit,
    Dns,
    Out,
    Dsq,
}

fn rider_mark(s: &Snapshot, race_num: i32, crashed: bool) -> RiderMark {
    if crashed {
        return RiderMark::Crash;
    }
    let Some(row) = s
        .standings
        .iter()
        .take(s.standing_count.max(0) as usize)
        .find(|st| st.race_num == race_num)
    else {
        return RiderMark::None;
    };
    match row.state {
        1 => RiderMark::Dns,
        3 => RiderMark::Out,
        4 => RiderMark::Dsq,
        _ if row.pit != 0 => RiderMark::Pit,
        _ => RiderMark::None,
    }
}

fn draw_state_mark(px: &mut Pixmap, fonts: &Fonts, x: f32, y: f32, r: f32, mark: RiderMark) {
    let (ch, col) = match mark {
        RiderMark::None => return,
        RiderMark::Crash => ('\u{f071}', Color::from_rgba8(232, 56, 56, 255)),
        RiderMark::Pit => ('\u{f0ad}', Color::from_rgba8(255, 168, 48, 255)),
        RiderMark::Dns => ('\u{f017}', Color::from_rgba8(180, 180, 186, 255)),
        RiderMark::Out => ('\u{f05e}', Color::from_rgba8(200, 80, 80, 255)),
        RiderMark::Dsq => ('\u{f00d}', Color::from_rgba8(232, 64, 64, 255)),
    };
    let size = (r * 1.05).clamp(8.0, 13.0);
    let ix = x + r * 0.62;
    let iy = y + r * 0.18;
    icon(px, fonts, ch, size, ix + 0.8, iy + 0.8, Color::from_rgba8(8, 8, 10, 230), true);
    icon(px, fonts, ch, size, ix, iy, col, true);
}

fn blit_circle(dst: &mut Pixmap, src: &Pixmap, dx: f32, dy: f32) {
    let sw = src.width() as i32;
    let sh = src.height() as i32;
    let cr = sw as f32 * 0.5 - 0.5;
    let fade_start = cr * 0.58;
    let ccx = cr;
    let ccy = sh as f32 * 0.5 - 0.5;
    let dst_w = dst.width() as i32;
    let dst_h = dst.height() as i32;
    let ox = dx.round() as i32;
    let oy = dy.round() as i32;
    let src_data = src.data();
    let dst_data = dst.data_mut();
    for sy in 0..sh {
        for sx in 0..sw {
            let fx = sx as f32 + 0.5;
            let fy = sy as f32 + 0.5;
            let dist = ((fx - ccx) * (fx - ccx) + (fy - ccy) * (fy - ccy)).sqrt();
            if dist >= cr {
                continue;
            }
            let cover = if dist <= fade_start {
                1.0
            } else {
                let t = ((dist - fade_start) / (cr - fade_start)).clamp(0.0, 1.0);
                let u = 1.0 - t;
                u * u * (3.0 - 2.0 * u)
            };
            if cover <= 0.004 {
                continue;
            }
            let dx_ = ox + sx;
            let dy_ = oy + sy;
            if dx_ < 0 || dy_ < 0 || dx_ >= dst_w || dy_ >= dst_h {
                continue;
            }
            let si = ((sy * sw + sx) * 4) as usize;
            let di = ((dy_ * dst_w + dx_) * 4) as usize;
            let sa = (src_data[si + 3] as f32 / 255.0) * cover;
            let inv = 1.0 - sa;
            dst_data[di] = (src_data[si] as f32 * cover + dst_data[di] as f32 * inv) as u8;
            dst_data[di + 1] = (src_data[si + 1] as f32 * cover + dst_data[di + 1] as f32 * inv) as u8;
            dst_data[di + 2] = (src_data[si + 2] as f32 * cover + dst_data[di + 2] as f32 * inv) as u8;
            dst_data[di + 3] = (src_data[si + 3] as f32 * cover + dst_data[di + 3] as f32 * inv) as u8;
        }
    }
}

fn fill_circle(px: &mut Pixmap, x: f32, y: f32, r: f32, color: Color) {
    if r <= 0.2 {
        return;
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

fn fill_round(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, r: f32, c: Color) {
    let r = r.min(w * 0.5).min(h * 0.5);
    let mut pb = PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.quad_to(x + w, y, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.quad_to(x + w, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.quad_to(x, y + h, x, y + h - r);
    pb.line_to(x, y + r);
    pb.quad_to(x, y, x + r, y);
    pb.close();
    if let Some(path) = pb.finish() {
        let mut p = Paint::default();
        p.set_color(c);
        p.anti_alias = true;
        px.fill_path(&path, &p, FillRule::Winding, Transform::identity(), None);
    }
}

fn draw_radar(px: &mut Pixmap, s: &Snapshot, cfg: &HudConfig, sw: f32, sh: f32, age: f32) {
    let r = cfg.radar;
    let x = r.x * sw;
    let y = r.y * sh;
    let w = (r.w * sw).max(48.0);
    let h = (r.h * sh).max(48.0);
    let cx = x + w * 0.5;
    let cy = y + h * 0.5;
    let size = w.min(h);

    let a = bg_a(cfg.radar_bg);
    if a > 0 {
        fill_round(px, x, y, w, h, 6.0, Color::from_rgba8(22, 22, 24, a));
        let da = ((28.0 * cfg.radar_bg as f32) / 100.0).round() as u8;
        if da > 0 {
            let step = 8.0;
            let mut py = y + 6.0;
            while py < y + h - 5.0 {
                let mut px_ = x + 6.0;
                while px_ < x + w - 5.0 {
                    fill_circle(px, px_, py, 1.15, Color::from_rgba8(210, 210, 214, da));
                    px_ += step;
                }
                py += step;
            }
        }
    }

    let pred_x = s.local_x + s.local_vel_x * age;
    let pred_z = s.local_z + s.local_vel_z * age;
    let (fx, fz, rx, rz) = radar_axes(s);
    let focus = s.focus_race_num;

    let mut left: Vec<(f32, f32)> = Vec::new();
    let mut right: Vec<(f32, f32)> = Vec::new();
    let mut rear = 0.0f32;

    if s.has_telemetry != 0 {
        for i in 0..s.rider_count.max(0) as usize {
            let rider = &s.riders[i];
            if rider.race_num == focus {
                continue;
            }
            let dx = rider.x - pred_x;
            let dz = rider.z - pred_z;
            let fwd = dx * fx + dz * fz;
            let lat = dx * rx + dz * rz;
            if cfg.radar_rear && fwd < -0.7 && fwd > -11.0 && lat.abs() < 3.6 {
                let t = ((-fwd - 0.7) / 10.0).clamp(0.0, 1.0);
                let near = 1.0 - t;
                if near > rear {
                    rear = near;
                }
            }
            if cfg.radar_sides && lat.abs() > 0.7 && lat.abs() < 7.5 && fwd > -3.2 && fwd < 5.5 {
                let dist = lat.abs();
                let strength = ((7.5 - dist) / 6.8).clamp(0.15, 1.0);
                if lat < 0.0 {
                    left.push((fwd, strength));
                } else {
                    right.push((fwd, strength));
                }
            }
        }
    }
    left.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    right.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    if rear > 0.04 {
        let gy = y + h * 0.90;
        let radius = size * (0.34 + rear * 0.22);
        let a = (40.0 + rear * 150.0) as u8;
        let xf = Transform::from_translate(cx, gy)
            .pre_scale(1.35, 0.52)
            .pre_translate(-cx, -gy);
        if let Some(shader) = RadialGradient::new(
            SkPoint::from_xy(cx, gy),
            SkPoint::from_xy(cx, gy),
            radius,
            vec![
                GradientStop::new(0.0, Color::from_rgba8(240, 196, 40, a)),
                GradientStop::new(0.45, Color::from_rgba8(220, 160, 28, a / 3)),
                GradientStop::new(1.0, Color::from_rgba8(200, 140, 16, 0)),
            ],
            SpreadMode::Pad,
            xf,
        ) {
            let mut paint = Paint::default();
            paint.shader = shader;
            paint.anti_alias = true;
            if let Some(rect) = rr(x, y + h * 0.45, w, h * 0.55) {
                px.fill_rect(rect, &paint, Transform::identity(), None);
            }
        }
    }

    let y_of = |fwd: f32| (cy - fwd * (size * 0.055)).clamp(y + 18.0, y + h - 18.0);
    let bar_h = size * 0.10;
    let bar_w = w * 0.38;
    let bike_w = size * 0.09;
    for (fwd, strength) in left.iter().copied().take(2) {
        radar_bar(px, cx - bike_w * 0.5 - 3.0, y_of(fwd), bar_w, bar_h, strength, true);
    }
    for (fwd, strength) in right.iter().copied().take(2) {
        radar_bar(px, cx + bike_w * 0.5 + 3.0, y_of(fwd), bar_w, bar_h, strength, false);
    }

    let mut pb = PathBuilder::new();
    pb.move_to(cx, y + 10.0);
    pb.line_to(cx, y + h - 10.0);
    if let Some(path) = pb.finish() {
        stroke_path(px, &path, Color::from_rgba8(8, 8, 10, 180), 2.4);
        stroke_path(px, &path, Color::from_rgba8(248, 248, 252, 230), 1.25);
    }

    let streak = w * 0.28;
    for dy in [-5.0f32, 0.0, 5.0] {
        let mut pb = PathBuilder::new();
        pb.move_to(cx - bike_w * 0.5 - 4.0, cy + dy);
        pb.line_to(cx - bike_w * 0.5 - 4.0 - streak, cy + dy);
        if let Some(path) = pb.finish() {
            stroke_path(px, &path, Color::from_rgba8(8, 8, 10, 160), 2.2);
            stroke_path(px, &path, Color::from_rgba8(255, 255, 255, 220), 1.15);
        }
        let mut pb = PathBuilder::new();
        pb.move_to(cx + bike_w * 0.5 + 4.0, cy + dy);
        pb.line_to(cx + bike_w * 0.5 + 4.0 + streak, cy + dy);
        if let Some(path) = pb.finish() {
            stroke_path(px, &path, Color::from_rgba8(8, 8, 10, 160), 2.2);
            stroke_path(px, &path, Color::from_rgba8(255, 255, 255, 220), 1.15);
        }
    }

    let bw = bike_w.max(8.0);
    let bh = (size * 0.22).max(16.0);
    fill_round(px, cx - bw * 0.5 - 1.6, cy - bh * 0.5 - 1.6, bw + 3.2, bh + 3.2, 3.2, Color::from_rgba8(8, 8, 10, 220));
    fill_round(px, cx - bw * 0.5, cy - bh * 0.5, bw, bh, 2.4, Color::from_rgba8(248, 248, 252, 255));
}

fn track_forward(s: &Snapshot, n: usize, px: f32, pz: f32) -> Option<(f32, f32)> {
    if n < 2 {
        return None;
    }
    let looped = {
        let dx = s.poly[0].x - s.poly[n - 1].x;
        let dz = s.poly[0].z - s.poly[n - 1].z;
        dx * dx + dz * dz < 400.0
    };
    let mut best = f32::MAX;
    let mut si = 1usize;
    let mut st = 0.0f32;
    let last = if looped { n + 1 } else { n };
    for i in 1..last {
        let a = &s.poly[i - 1];
        let b = &s.poly[i % n];
        let bx = b.x - a.x;
        let bz = b.z - a.z;
        let len2 = bx * bx + bz * bz;
        if len2 < 1e-6 {
            continue;
        }
        let t = ((px - a.x) * bx + (pz - a.z) * bz) / len2;
        let t = t.clamp(0.0, 1.0);
        let dx = px - (a.x + bx * t);
        let dz = pz - (a.z + bz * t);
        let d = dx * dx + dz * dz;
        if d < best {
            best = d;
            si = i;
            st = t;
        }
    }
    if best > 90_000.0 {
        return None;
    }
    let a = &s.poly[si - 1];
    let b = &s.poly[si % n];
    let mut x = a.x + (b.x - a.x) * st;
    let mut z = a.z + (b.z - a.z) * st;
    let ox = x;
    let oz = z;
    let mut remain = 22.0;
    let mut i = si;
    for _ in 0..n + 2 {
        if remain <= 0.05 {
            break;
        }
        if i >= n && !looped {
            break;
        }
        let nxt = &s.poly[i % n];
        let dx = nxt.x - x;
        let dz = nxt.z - z;
        let len = (dx * dx + dz * dz).sqrt();
        if len < 1e-3 {
            i += 1;
            continue;
        }
        if len >= remain {
            x += dx * (remain / len);
            z += dz * (remain / len);
            break;
        }
        remain -= len;
        x = nxt.x;
        z = nxt.z;
        i += 1;
    }
    let fx = x - ox;
    let fz = z - oz;
    let len = (fx * fx + fz * fz).sqrt();
    if len < 0.4 {
        return None;
    }
    Some((fx / len, fz / len))
}

fn radar_axes(s: &Snapshot) -> (f32, f32, f32, f32) {
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

fn radar_bar(px: &mut Pixmap, inner_x: f32, mid_y: f32, w: f32, h: f32, strength: f32, left: bool) {
    let a = (50.0 + strength * 190.0) as u8;
    let (x0, x1) = if left {
        (inner_x - w, inner_x)
    } else {
        (inner_x, inner_x + w)
    };
    let (c0, c1) = if left {
        (
            Color::from_rgba8(210, 28, 32, 0),
            Color::from_rgba8(230, 36, 40, a),
        )
    } else {
        (
            Color::from_rgba8(230, 36, 40, a),
            Color::from_rgba8(210, 28, 32, 0),
        )
    };
    if let Some(shader) = LinearGradient::new(
        SkPoint::from_xy(x0, mid_y),
        SkPoint::from_xy(x1, mid_y),
        vec![GradientStop::new(0.0, c0), GradientStop::new(1.0, c1)],
        SpreadMode::Pad,
        Transform::identity(),
    ) {
        let mut paint = Paint::default();
        paint.shader = shader;
        paint.anti_alias = true;
        if let Some(rect) = rr(x0, mid_y - h * 0.5, w, h) {
            px.fill_rect(rect, &paint, Transform::identity(), None);
        }
    }
}

fn draw_track_arrows(
    px: &mut Pixmap,
    s: &Snapshot,
    n: usize,
    to_px: impl Fn(f32, f32) -> (f32, f32),
    track_px: f32,
    circle: Option<(f32, f32)>,
    zoomed: bool,
) {
    if n < 2 {
        return;
    }
    let looped = {
        let dx = s.poly[0].x - s.poly[n - 1].x;
        let dz = s.poly[0].z - s.poly[n - 1].z;
        dx * dx + dz * dz < 400.0
    };
    let mut total = 0.0f32;
    let last = if looped { n } else { n - 1 };
    for i in 0..last {
        let a = &s.poly[i];
        let b = &s.poly[(i + 1) % n];
        let dx = b.x - a.x;
        let dz = b.z - a.z;
        total += (dx * dx + dz * dz).sqrt();
    }
    if total < 8.0 {
        return;
    }
    let spacing = if zoomed {
        12.0
    } else {
        (total / 14.0).clamp(16.0, 48.0)
    };
    let clip = circle.map(|(mc, sdim)| {
        let inner2 = if zoomed { (sdim * 0.08).powi(2) } else { 0.0 };
        let outer2 = (sdim * 0.42).powi(2);
        (mc, inner2, outer2)
    });
    let mut acc = 0.0f32;
    let mut next = spacing * 0.55;
    for i in 0..last {
        let a = &s.poly[i];
        let b = &s.poly[(i + 1) % n];
        let dx = b.x - a.x;
        let dz = b.z - a.z;
        let len = (dx * dx + dz * dz).sqrt();
        if len < 0.05 {
            continue;
        }
        while next <= acc + len {
            let t = ((next - acc) / len).clamp(0.0, 1.0);
            let wx = a.x + dx * t;
            let wz = a.z + dz * t;
            let (sx, sy) = to_px(wx, wz);
            let visible = match clip {
                Some((mc, inner2, outer2)) => {
                    let d2 = (sx - mc) * (sx - mc) + (sy - mc) * (sy - mc);
                    d2 >= inner2 && d2 <= outer2
                }
                None => true,
            };
            if visible {
                let (ex, ey) = to_px(wx + dx, wz + dz);
                draw_track_chevron(px, sx, sy, ex - sx, ey - sy, track_px);
            }
            next += spacing;
        }
        acc += len;
    }
}

fn draw_track_chevron(px: &mut Pixmap, x: f32, y: f32, dx: f32, dy: f32, track_px: f32) {
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1.0e-3 {
        return;
    }
    let fx = dx / len;
    let fy = dy / len;
    let rx = -fy;
    let ry = fx;
    let w = (track_px * 0.28).clamp(3.2, 7.0);
    let h = (track_px * 0.40).clamp(4.5, 9.5);
    let tip_x = x + fx * h * 0.55;
    let tip_y = y + fy * h * 0.55;
    let bx = x - fx * h * 0.45;
    let by = y - fy * h * 0.45;
    let mut pb = PathBuilder::new();
    pb.move_to(tip_x, tip_y);
    pb.line_to(bx + rx * w, by + ry * w);
    pb.line_to(bx + fx * h * 0.12, by + fy * h * 0.12);
    pb.line_to(bx - rx * w, by - ry * w);
    pb.close();
    let Some(path) = pb.finish() else {
        return;
    };
    let mut paint = Paint::default();
    paint.set_color(Color::from_rgba8(8, 8, 10, 230));
    paint.anti_alias = true;
    px.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
}

fn draw_sf(px: &mut Pixmap, s: &Snapshot, n: usize, to_px: impl Fn(f32, f32) -> (f32, f32), track_px: f32) {
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
        let half = track_px * 0.52;
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
