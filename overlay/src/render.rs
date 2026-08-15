use std::sync::Mutex;

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
        return;
    };
    if s.on_track == 0 && !settings_hint {
        return;
    }

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
    if cfg.show_dash {
        draw_dash(px, fonts, s, cfg, sw, sh);
    }
    if settings_hint {
        draw_layout(px, fonts, s, cfg, sw, sh);
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

fn draw_layout(px: &mut Pixmap, fonts: &Fonts, s: &Snapshot, cfg: &HudConfig, sw: f32, sh: f32) {
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
    if cfg.show_dash {
        let (dx, dy, dw, dh) = dash_box(fonts, s, cfg, sw, sh);
        layout_box(px, dx, dy, dw, dh);
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

fn bike_color(bike: &str, extra: &str) -> Color {
    let hay = format!("{bike} {extra}").to_ascii_lowercase();
    let tokens: Vec<&str> = hay
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .collect();
    let has = |n: &str| hay.contains(n);
    let tok = |n: &str| tokens.iter().any(|t| *t == n || t.starts_with(n));
    if has("husq") || has("husky") || tok("fc") || tok("tc") || tok("fx") {
        Color::from_rgba8(240, 240, 244, 255)
    } else if has("yamaha") || tok("yz") || tok("yzf") {
        Color::from_rgba8(0, 82, 196, 255)
    } else if has("kawasaki") || tok("kx") || tok("kxf") {
        Color::from_rgba8(80, 196, 32, 255)
    } else if has("suzuki") || tok("rmz") || has("rm z") {
        Color::from_rgba8(236, 208, 24, 255)
    } else if has("honda") || tok("crf") || tok("cr") {
        Color::from_rgba8(220, 28, 36, 255)
    } else if has("gasgas") || has("gas gas") || tok("mc") {
        Color::from_rgba8(196, 24, 40, 255)
    } else if has("ktm") || tok("sxf") || tok("xcf") || tok("exc") || has("sx f") {
        Color::from_rgba8(244, 108, 16, 255)
    } else if has("sherco") {
        Color::from_rgba8(32, 96, 196, 255)
    } else if has("beta") {
        Color::from_rgba8(196, 32, 40, 255)
    } else if has("fantic") {
        Color::from_rgba8(220, 40, 48, 255)
    } else if tokens.iter().any(|t| *t == "tm") {
        Color::from_rgba8(32, 180, 220, 255)
    } else if extra.is_empty() && bike.is_empty() {
        accent()
    } else {
        let h = hay.bytes().fold(2166136261u32, |a, b| a.wrapping_mul(16777619) ^ b as u32);
        const PAL: [(u8, u8, u8); 6] = [
            (232, 196, 48),
            (48, 208, 232),
            (232, 80, 148),
            (80, 214, 96),
            (232, 132, 48),
            (168, 128, 255),
        ];
        let (r, g, b) = PAL[h as usize % PAL.len()];
        Color::from_rgba8(r, g, b, 255)
    }
}

fn ink_on(c: Color) -> Color {
    if 0.2126 * c.red() + 0.7152 * c.green() + 0.0722 * c.blue() > 0.62 {
        Color::from_rgba8(16, 16, 18, 255)
    } else {
        Color::from_rgba8(248, 248, 250, 255)
    }
}

fn fill_skew(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, skew: f32, c: Color) {
    let mut pb = PathBuilder::new();
    pb.move_to(x + skew, y);
    pb.line_to(x + w + skew, y);
    pb.line_to(x + w, y + h);
    pb.line_to(x, y + h);
    pb.close();
    if let Some(path) = pb.finish() {
        fill_path(px, &path, c);
    }
}

fn format_board_gap(ms: i32, laps: i32, leader: bool) -> String {
    if leader {
        return "-".into();
    }
    if laps != 0 {
        return format!("{laps}L");
    }
    if ms <= 0 {
        return "-".into();
    }
    let sec = ms as f32 / 1000.0;
    if sec >= 60.0 {
        let m = (sec / 60.0) as i32;
        format!("{m}:{:04.1}", sec - m as f32 * 60.0)
    } else {
        format!("{sec:.1}")
    }
}

fn format_session_clock(ms: i32) -> String {
    if ms <= 0 {
        return "--:--:--".into();
    }
    let t = ms / 1000;
    format!("{:02}:{:02}:{:02}", t / 3600, (t / 60) % 60, t % 60)
}

fn format_session_len(len: i32) -> String {
    if len <= 0 {
        return String::new();
    }
    if len >= 60 && len % 60 == 0 {
        return format!("{:02}:{:02}m", 0, len / 60);
    }
    if len < 1000 {
        format!("{:02}:{:02}m", 0, len.max(1))
    } else {
        let s = if len > 100_000 { len / 1000 } else { len };
        format!("{:02}:{:02}m", 0, (s / 60).max(1))
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

fn draw_standings(px: &mut Pixmap, fonts: &Fonts, s: &Snapshot, cfg: &HudConfig, sw: f32, sh: f32) {
    let r = s.standings_rect;
    let x = r.x * sw;
    let y = r.y * sh;
    let w = r.w * sw;
    let rows = s.standing_count.max(0) as usize;
    let max_rows = s.standings_rows.max(3) as usize;
    let n = rows.min(MAX_SAFE);
    let focus = s.focus_race_num;
    let mut start = 0;
    if n > max_rows {
        let fi = (0..n).find(|&i| s.standings[i].race_num == focus).unwrap_or(0);
        start = fi.saturating_sub(max_rows / 2).min(n.saturating_sub(max_rows));
    }
    let end = (start + max_rows).min(n);
    let slice = if n == 0 { &s.standings[..0] } else { &s.standings[start..end] };

    let mut cats: Vec<String> = slice.iter().map(|row| cstr(&row.category)).filter(|c| !c.is_empty()).collect();
    cats.sort();
    cats.dedup();
    let show_classes = !cats.is_empty();
    let class_h = if show_classes { 20.0 } else { 0.0 };
    let class_count = if show_classes {
        let mut last = String::new();
        let mut n_hdr = 0;
        for row in slice {
            let c = cstr(&row.category);
            if c != last {
                n_hdr += 1;
                last = c;
            }
        }
        n_hdr
    } else {
        0
    };

    let head_h = 26.0;
    let col_h = 16.0;
    let row_h = 22.0;
    let vis = slice.len().max(1);
    let h = (head_h + col_h + class_count as f32 * class_h + vis as f32 * row_h + 8.0).min(r.h * sh);
    let a = bg_a(cfg.st_bg);
    if a > 0 {
        fill_round(px, x, y, w, h, 6.0, Color::from_rgba8(8, 8, 10, a));
        fill_round(px, x, y, w, head_h, 6.0, Color::from_rgba8(4, 4, 6, ((a as u16 * 240) / 200).min(255) as u8));
        if let Some(rrt) = rr(x, y + head_h - 6.0, w, 6.0) {
            fill_rect(px, rrt, Color::from_rgba8(4, 4, 6, ((a as u16 * 240) / 200).min(255) as u8));
        }
    }

    let clock = format_session_clock(s.session_time_ms);
    let len = format_session_len(s.session_length);
    let time_txt = if len.is_empty() { clock } else { format!("{clock} / {len}") };
    icon(px, fonts, '\u{f2f2}', 11.0, x + 8.0, y + 7.0, text_col(), false);
    text(px, fonts, &time_txt, 12.0, x + 24.0, y + 6.5, text_col(), false);
    let count_txt = format!("{}", rows.max(s.rider_count.max(0) as usize));
    let cw = measure(fonts, &count_txt, 12.0);
    text(px, fonts, &count_txt, 12.0, x + w - 10.0 - cw, y + 6.5, text_col(), false);
    icon(px, fonts, '\u{f553}', 11.0, x + w - 26.0 - cw, y + 7.0, text_col(), false);

    if rows == 0 {
        text(px, fonts, "Waiting for race data", 12.0, x + 12.0, y + head_h + 10.0, text_dim(), false);
        return;
    }

    let extras: Vec<StField> = standings_cols(cfg)
        .into_iter()
        .filter(|c| {
            !matches!(
                c,
                StField::Pos | StField::Name | StField::Bike | StField::Gap | StField::Best
            )
        })
        .collect();
    let gap_w = 40.0;
    let time_w = 54.0;
    let extra_w: f32 = extras.iter().map(|c| c.width(cfg) as f32).sum();
    let pad = 8.0;
    let pos_w = 22.0;
    let bar_w = 7.0;
    let right = pad + gap_w + time_w + time_w + extra_w;
    let name_x = x + pad + pos_w + bar_w + 6.0;
    let name_max = (x + w - right - name_x - 8.0).max(48.0);

    let hdr_y = y + head_h + 2.0;
    let hdr_c = Color::from_rgba8(160, 160, 168, 220);
    let mut rx = x + w - pad;
    let last_x = rx - time_w;
    text(px, fonts, "Last", 10.0, last_x + time_w - measure(fonts, "Last", 10.0), hdr_y, hdr_c, false);
    rx = last_x;
    let fast_x = rx - time_w;
    text(px, fonts, "Fastest", 10.0, fast_x + time_w - measure(fonts, "Fastest", 10.0), hdr_y, hdr_c, false);
    rx = fast_x;
    let gap_x = rx - gap_w;
    text(px, fonts, "Gap", 10.0, gap_x + gap_w - measure(fonts, "Gap", 10.0), hdr_y, hdr_c, false);
    let mut extra_xs = Vec::new();
    rx = gap_x;
    for col in extras.iter().rev() {
        let cw = col.width(cfg) as f32;
        rx -= cw;
        extra_xs.push((*col, rx, cw));
        let label = st_col_header(*col);
        text(px, fonts, label, 10.0, rx + cw - measure(fonts, label, 10.0), hdr_y, hdr_c, false);
    }
    extra_xs.reverse();

    let purple = Color::from_rgba8(196, 112, 255, 255);
    let best_ms = slice
        .iter()
        .chain(s.standings.iter().take(n))
        .map(|row| row.best_lap_ms)
        .filter(|ms| *ms > 0)
        .min()
        .unwrap_or(0);
    let gold = Color::from_rgba8(168, 118, 36, ((a as u16 * 150) / 255).max(90) as u8);
    let stripe_c = Color::from_rgba8(22, 22, 24, ((a as u16 * 40) / 255) as u8);
    let out_c = Color::from_rgba8(110, 110, 116, 255);

    let mut cy = y + head_h + col_h;
    let mut last_cat = String::from("\0");
    for (vis_i, row) in slice.iter().enumerate() {
        let cat = cstr(&row.category);
        let accent_c = bike_color(&cstr(&row.bike), &cat);
        if show_classes && cat != last_cat {
            let count = s.standings[..n].iter().filter(|st| cstr(&st.category) == cat).count();
            let label = if cat.is_empty() { "OPEN".into() } else { cat.to_uppercase() };
            fill_skew(px, x + 8.0, cy + 3.0, 28.0, 14.0, 4.0, accent_c);
            icon(px, fonts, '\u{f553}', 8.0, x + 12.0, cy + 5.0, Color::from_rgba8(12, 12, 14, 255), false);
            text(px, fonts, &format!("{count}"), 9.0, x + 22.0, cy + 4.5, Color::from_rgba8(12, 12, 14, 255), false);
            fill_skew(px, x + 40.0, cy + 3.0, (measure(fonts, &label, 10.0) + 14.0).max(36.0), 14.0, 4.0, accent_c);
            text(px, fonts, &label, 10.0, x + 48.0, cy + 4.5, Color::from_rgba8(12, 12, 14, 255), false);
            if let Some(line) = rr(x + 8.0, cy + 18.0, w - 16.0, 1.2) {
                fill_rect(px, line, accent_c);
            }
            last_cat = cat.clone();
            cy += class_h;
        }

        let is_focus = row.race_num == focus;
        let out = standing_status(row).is_some() && standing_status(row) != Some("PIT");
        if is_focus {
            if let Some(rrt) = rr(x + 2.0, cy, w - 4.0, row_h) {
                fill_rect(px, rrt, gold);
            }
        } else if vis_i % 2 == 1 && a > 0 {
            if let Some(rrt) = rr(x + 2.0, cy, w - 4.0, row_h) {
                fill_rect(px, rrt, stripe_c);
            }
        }

        let name_c = if out { out_c } else { text_col() };
        let dim = if out { out_c } else { Color::from_rgba8(210, 210, 216, 255) };
        let pos = format!("{}", row.position.max(0));
        text(
            px,
            fonts,
            &pos,
            12.0,
            x + pad + pos_w - measure(fonts, &pos, 12.0),
            cy + 4.0,
            name_c,
            false,
        );
        fill_skew(px, x + pad + pos_w + 2.0, cy + 4.0, 5.0, row_h - 8.0, 3.0, accent_c);

        let bike = cstr(&row.bike);
        let badge = if bike.is_empty() {
            String::new()
        } else {
            ellipsize(fonts, &bike, 9.0, 54.0)
        };
        let badge_w = if badge.is_empty() { 0.0 } else { measure(fonts, &badge, 9.0) + 10.0 };
        let name = ellipsize(fonts, &cstr(&row.name), 12.0, (name_max - badge_w - 6.0).max(24.0));
        text(px, fonts, &name, 12.0, name_x, cy + 4.0, name_c, false);
        if !badge.is_empty() {
            let bx = name_x + measure(fonts, &name, 12.0) + 6.0;
            fill_round(px, bx, cy + 5.0, badge_w, 13.0, 3.0, accent_c);
            text(px, fonts, &badge, 9.0, bx + 5.0, cy + 6.0, ink_on(accent_c), false);
        }

        for (kind, cx, cw) in &extra_xs {
            let status = standing_status(row);
            let (val, color) = match kind {
                StField::Num => (format!("{}", row.race_num), dim),
                StField::Laps => (format!("{}", row.num_laps.max(0)), dim),
                StField::Status => (status.unwrap_or("").to_string(), dim),
                StField::Interval => (interval_text(s, row), dim),
                StField::Penalty => (format_penalty(row.penalty_ms), dim),
                StField::Crashed => {
                    if row.crashed != 0 || rider_crashed(s, row.race_num) {
                        ("CRASH".into(), behind_col())
                    } else {
                        ("".into(), dim)
                    }
                }
                _ => (String::new(), dim),
            };
            if !val.is_empty() {
                text(px, fonts, &val, 11.0, *cx + *cw - measure(fonts, &val, 11.0), cy + 4.5, color, false);
            }
        }

        let gap = if let Some(st) = standing_status(row) {
            if cfg.st_status { format_board_gap(row.gap_ms, row.gap_laps, row.position <= 1) } else { st.to_string() }
        } else {
            format_board_gap(row.gap_ms, row.gap_laps, row.position <= 1)
        };
        text(px, fonts, &gap, 11.0, gap_x + gap_w - measure(fonts, &gap, 11.0), cy + 4.5, dim, false);
        let fastest = format_lap(row.best_lap_ms);
        let fcol = if best_ms > 0 && row.best_lap_ms == best_ms && !out { purple } else { dim };
        text(px, fonts, &fastest, 11.0, fast_x + time_w - measure(fonts, &fastest, 11.0), cy + 4.5, fcol, false);
        let last = if row.last_lap_ms > 0 {
            format_lap(row.last_lap_ms)
        } else if row.race_num == focus && s.last_lap_ms > 0 {
            format_lap(s.last_lap_ms)
        } else {
            "--".into()
        };
        text(px, fonts, &last, 11.0, last_x + time_w - measure(fonts, &last, 11.0), cy + 4.5, dim, false);
        cy += row_h;
    }
}

const MAX_SAFE: usize = 40;

fn dash_pos_col() -> Color { Color::from_rgba8(232, 120, 23, 255) }

static DASH_VIS: Mutex<crate::shm::Rect> = Mutex::new(crate::shm::Rect {
    x: 0.41,
    y: 0.82,
    w: 0.18,
    h: 0.16,
});

pub fn dash_visual() -> crate::shm::Rect {
    *DASH_VIS.lock().unwrap_or_else(|e| e.into_inner())
}

struct DashLay {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    pad: f32,
    footer_h: f32,
    rev_x: f32,
    rev_y: f32,
    rev_w: f32,
    rev_h: f32,
    gear_x: f32,
    gear_w: f32,
    main_y: f32,
    main_h: f32,
    mid_x: f32,
    mid_w: f32,
    right_x: f32,
    label: f32,
    gear_n: f32,
    val: f32,
    pos_n: f32,
    lap_sz: f32,
    icon_s: f32,
    fsz: f32,
    cut: f32,
    gear: String,
    rpm: String,
    kph: String,
    ptxt: String,
    lap_txt: String,
    foot: [(char, String); 3],
}

fn dash_box(fonts: &Fonts, s: &Snapshot, cfg: &HudConfig, sw: f32, sh: f32) -> (f32, f32, f32, f32) {
    let lay = dash_layout(fonts, s, cfg, sw, sh);
    (lay.x, lay.y, lay.w, lay.h)
}

fn dash_layout(fonts: &Fonts, s: &Snapshot, cfg: &HudConfig, sw: f32, sh: f32) -> DashLay {
    let x0 = cfg.dash.x * sw;
    let y = cfg.dash.y * sh;
    let h = (cfg.dash.h * sh).max(80.0);
    let pad = (h * 0.12).clamp(14.0, 20.0);
    let footer_h = (h * 0.20).clamp(16.0, 22.0);
    let rev_h = (h * 0.12).clamp(11.0, 15.0);
    let rev_y = y + (pad * 0.28).max(4.0);
    let main_y = (rev_y + rev_h + 6.0).max(y + pad);
    let main_h = (y + h - pad - footer_h - main_y).max(36.0);
    let label = (h * 0.095).clamp(8.5, 11.0);
    let gear_n = (main_h * 0.58).clamp(20.0, 36.0);
    let val = (main_h * 0.26).clamp(13.0, 18.0);
    let pos_n = (main_h * 0.44).clamp(18.0, 30.0);
    let lap_sz = (val * 0.88).max(10.5);
    let icon_s = (footer_h * 0.68).clamp(9.0, 12.0);
    let fsz = (footer_h * 0.52).clamp(8.5, 11.0);
    let gap = (h * 0.11).clamp(14.0, 20.0);

    let gear = if s.local_gear <= 0 {
        "N".into()
    } else {
        format!("{}", s.local_gear)
    };
    let kph_n = (s.local_speed * 3.6).round().max(0.0) as i32;
    let rpm_n = s.local_rpm.max(0);
    let rpm = format!("{rpm_n}");
    let kph = format!("{kph_n}");
    let pos = s
        .standings
        .iter()
        .take(s.standing_count.max(0) as usize)
        .find(|st| st.race_num == s.focus_race_num)
        .map(|st| st.position)
        .filter(|p| *p > 0);
    let ptxt = pos.map(|p| format!("P{p}")).unwrap_or_else(|| "P--".into());
    let lap = s.current_lap.max(0);
    let lap_txt = if s.session_laps > 0 {
        format!("LAP: {lap} / {}", s.session_laps)
    } else if lap > 0 {
        format!("LAP: {lap}")
    } else {
        "LAP: --".into()
    };
    let engine = if s.engine_temp > 0.5 { format!("{:.0}°C", s.engine_temp) } else { "--°C".into() };
    let air = if s.air_temp > 0.5 { format!("{:.0}°C", s.air_temp) } else { "--°C".into() };
    let standing_best = s
        .standings
        .iter()
        .take(s.standing_count.max(0) as usize)
        .find(|st| st.race_num == s.focus_race_num)
        .map(|st| st.best_lap_ms)
        .unwrap_or(0);
    let lap_ms = [s.best_lap_ms, standing_best]
        .into_iter()
        .filter(|ms| *ms > 0)
        .min()
        .unwrap_or(0);
    let foot = [
        ('\u{f2c9}', format!("ENGINE {engine}")),
        ('\u{f72e}', format!("AIR TEMP {air}")),
        ('\u{f2f2}', format_clock(lap_ms)),
    ];

    let gear_w = (main_h * 0.82).clamp(44.0, 56.0);
    let mid_w = (measure(fonts, "RPM", label) + 8.0 + measure(fonts, &rpm, val).max(measure(fonts, "0000", val)))
        .max(measure(fonts, "KPH", label) + 8.0 + measure(fonts, &kph, val).max(measure(fonts, "000", val)));
    let right_w = measure(fonts, "POSITION", label)
        .max(measure(fonts, &ptxt, pos_n))
        .max(measure(fonts, &lap_txt, lap_sz));
    let mut foot_w = 0.0;
    for (i, (ch, t)) in foot.iter().enumerate() {
        if i > 0 {
            foot_w += 16.0;
        }
        foot_w += fonts.icons.metrics(*ch, icon_s).advance_width + 5.0 + measure(fonts, t, fsz);
    }
    let inner = gear_w + gap + mid_w + gap + right_w;
    let w = pad * 2.0 + inner.max(foot_w) + 40.0;
    let x = x0;

    if let Ok(mut vis) = DASH_VIS.lock() {
        *vis = crate::shm::Rect {
            x: x / sw,
            y: y / sh,
            w: w / sw,
            h: h / sh,
        };
    }

    DashLay {
        x,
        y,
        w,
        h,
        pad,
        footer_h,
        rev_x: x + pad,
        rev_y,
        rev_w: w - pad * 2.0,
        rev_h,
        gear_x: x + pad,
        gear_w,
        main_y,
        main_h,
        mid_x: x + pad + gear_w + gap,
        mid_w,
        right_x: x + pad + gear_w + gap + mid_w + gap,
        label,
        gear_n,
        val,
        pos_n,
        lap_sz,
        icon_s,
        fsz,
        cut: (h * 0.14).clamp(7.0, 12.0),
        gear,
        rpm,
        kph,
        ptxt,
        lap_txt,
        foot,
    }
}

fn chamfer_path(x: f32, y: f32, w: f32, h: f32, cut: f32) -> Option<Path> {
    let cut = cut.min(w * 0.45).min(h * 0.45).max(2.0);
    let mut pb = PathBuilder::new();
    pb.move_to(x + cut, y);
    pb.line_to(x + w - cut, y);
    pb.line_to(x + w, y + cut);
    pb.line_to(x + w, y + h - cut);
    pb.line_to(x + w - cut, y + h);
    pb.line_to(x + cut, y + h);
    pb.line_to(x, y + h - cut);
    pb.line_to(x, y + cut);
    pb.close();
    pb.finish()
}

fn fill_path(px: &mut Pixmap, path: &Path, color: Color) {
    let mut p = Paint::default();
    p.set_color(color);
    p.anti_alias = true;
    px.fill_path(path, &p, FillRule::Winding, Transform::identity(), None);
}

fn draw_rev_bar(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, rpm: i32, max_rpm: i32, shift_rpm: i32) {
    if w < 24.0 || h < 6.0 {
        return;
    }
    let n = 20i32;
    let ceiling = max_rpm.max(shift_rpm).max(rpm).max(8000) as f32;
    let t = (rpm.max(0) as f32 / ceiling).clamp(0.0, 1.0);
    let lit = if rpm <= 0 {
        0
    } else {
        ((t * n as f32).ceil() as i32).clamp(1, n)
    };
    fill_round(
        px,
        x - 3.0,
        y - 2.5,
        w + 6.0,
        h + 5.0,
        (h + 5.0) * 0.42,
        Color::from_rgba8(6, 6, 8, 170),
    );
    let gap = (w * 0.014).clamp(1.8, 3.0);
    let seg_w = ((w - gap * (n - 1) as f32) / n as f32).max(2.2);
    for i in 0..n {
        let sx = x + i as f32 * (seg_w + gap);
        let on = i < lit;
        let col = if !on {
            Color::from_rgba8(48, 50, 54, 230)
        } else if i >= 13 {
            Color::from_rgba8(236, 44, 36, 255)
        } else if i >= 11 {
            Color::from_rgba8(244, 214, 36, 255)
        } else {
            Color::from_rgba8(36, 230, 68, 255)
        };
        fill_round(px, sx, y, seg_w, h, h * 0.5, col);
        if on {
            fill_round(
                px,
                sx + 0.55,
                y + 0.7,
                (seg_w - 1.1).max(0.6),
                (h * 0.36).max(1.0),
                h * 0.18,
                Color::from_rgba8(255, 255, 255, 58),
            );
        }
    }
}

fn format_clock(ms: i32) -> String {
    if ms <= 0 {
        return "--:--.---".into();
    }
    let t = ms as f32 / 1000.0;
    let m = (t / 60.0) as i32;
    let s = t - m as f32 * 60.0;
    format!("{m:02}:{:06.3}", s)
}

fn draw_dash(px: &mut Pixmap, fonts: &Fonts, s: &Snapshot, cfg: &HudConfig, sw: f32, sh: f32) {
    let d = dash_layout(fonts, s, cfg, sw, sh);
    let a = bg_a(cfg.dash_bg);
    if let Some(path) = chamfer_path(d.x, d.y, d.w, d.h, d.cut) {
        if a > 0 {
            fill_path(px, &path, Color::from_rgba8(18, 18, 20, a));
        }
        if let Some(shader) = LinearGradient::new(
            SkPoint::from_xy(d.x + d.w * 0.42, d.y),
            SkPoint::from_xy(d.x + d.w, d.y + d.h * 0.85),
            vec![
                GradientStop::new(0.0, Color::from_rgba8(255, 255, 255, 0)),
                GradientStop::new(0.58, Color::from_rgba8(255, 255, 255, 0)),
                GradientStop::new(0.74, Color::from_rgba8(255, 255, 255, ((38.0 * a as f32) / 255.0) as u8)),
                GradientStop::new(1.0, Color::from_rgba8(255, 255, 255, 0)),
            ],
            SpreadMode::Pad,
            Transform::identity(),
        ) {
            let mut paint = Paint::default();
            paint.shader = shader;
            paint.anti_alias = true;
            px.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
        }
        stroke_path(px, &path, Color::from_rgba8(220, 220, 224, ((a as u16 * 200) / 255).max(90) as u8), 1.4);
    }

    draw_rev_bar(px, d.rev_x, d.rev_y, d.rev_w, d.rev_h, s.local_rpm, s.max_rpm, s.shift_rpm);

    let white = Color::from_rgba8(248, 248, 250, 255);
    let dim = Color::from_rgba8(168, 168, 176, 255);
    if let Some(path) = chamfer_path(d.gear_x, d.main_y, d.gear_w, d.main_h, d.cut * 0.45) {
        stroke_path(px, &path, Color::from_rgba8(236, 236, 240, 210), 1.3);
    }
    text(px, fonts, "GEAR", d.label, d.gear_x + d.gear_w * 0.5, d.main_y + d.main_h * 0.10, white, true);
    text(px, fonts, &d.gear, d.gear_n, d.gear_x + d.gear_w * 0.5, d.main_y + d.main_h * 0.36, white, true);

    text(px, fonts, "RPM", d.label, d.mid_x, d.main_y + d.main_h * 0.12, dim, false);
    text(px, fonts, &d.rpm, d.val, d.mid_x + d.mid_w - measure(fonts, &d.rpm, d.val), d.main_y + d.main_h * 0.10, white, false);
    if let Some(line) = rr(d.mid_x, d.main_y + d.main_h * 0.48, d.mid_w, 1.0) {
        fill_rect(px, line, Color::from_rgba8(200, 200, 206, 70));
    }
    text(px, fonts, "KPH", d.label, d.mid_x, d.main_y + d.main_h * 0.62, dim, false);
    text(px, fonts, &d.kph, d.val, d.mid_x + d.mid_w - measure(fonts, &d.kph, d.val), d.main_y + d.main_h * 0.58, white, false);

    text(px, fonts, "POSITION", d.label, d.right_x, d.main_y + d.main_h * 0.08, white, false);
    text(px, fonts, &d.ptxt, d.pos_n, d.right_x + 1.0, d.main_y + d.main_h * 0.28 + 1.0, Color::from_rgba8(20, 12, 6, 160), false);
    text(px, fonts, &d.ptxt, d.pos_n, d.right_x, d.main_y + d.main_h * 0.28, dash_pos_col(), false);
    text(px, fonts, &d.lap_txt, d.lap_sz, d.right_x, d.main_y + d.main_h * 0.78, white, false);

    let fy = d.y + d.h - d.pad - d.footer_h + 1.0;
    let mut fx = d.x + d.pad;
    for (i, (ch, label)) in d.foot.iter().enumerate() {
        if i > 0 {
            fx += 16.0;
        }
        icon(px, fonts, *ch, d.icon_s, fx, fy, white, false);
        fx += fonts.icons.metrics(*ch, d.icon_s).advance_width + 5.0;
        text(px, fonts, label, d.fsz, fx, fy + 1.0, white, false);
        fx += measure(fonts, label, d.fsz);
    }
}

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
    let head_h = 22.0;
    let foot_h = 20.0;
    let row_h = 22.0;
    let h = (head_h + vis as f32 * row_h + foot_h + 4.0).min(r.h * sh);
    let a = bg_a(cfg.rel_bg);
    if a > 0 {
        fill_round(px, x, y, w, h, 6.0, Color::from_rgba8(8, 8, 10, a));
        fill_round(px, x, y, w, head_h, 6.0, Color::from_rgba8(4, 4, 6, ((a as u16 * 240) / 200).min(255) as u8));
        if let Some(rrt) = rr(x, y + head_h - 6.0, w, 6.0) {
            fill_rect(px, rrt, Color::from_rgba8(4, 4, 6, ((a as u16 * 240) / 200).min(255) as u8));
        }
    }
    fill_round(px, x + 8.0, y + 5.0, 28.0, 13.0, 2.0, Color::from_rgba8(48, 48, 52, 230));
    text(px, fonts, "REL", 9.0, x + 13.0, y + 6.5, Color::from_rgba8(200, 200, 206, 255), false);
    text(px, fonts, &format!("{}", n.max(s.standing_count.max(0) as usize)), 12.0, x + 40.0, y + 5.0, text_col(), false);
    if !have || n == 0 {
        text(px, fonts, "Waiting for positions", 12.0, x + 12.0, y + head_h + 8.0, text_dim(), false);
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

    let extras: Vec<RelField> = cfg
        .relative_cols()
        .into_iter()
        .filter(|c| !matches!(c, RelField::Pos | RelField::Name | RelField::Bike | RelField::Gap))
        .collect();
    let pad = 8.0;
    let pos_w = 22.0;
    let gap_w = 36.0;
    let name_x = x + pad + pos_w + 13.0;
    let gap_x = x + w - pad - gap_w;
    let mut extra_xs = Vec::new();
    let mut rx = gap_x;
    for col in extras.iter().rev() {
        let cw = col.width(cfg) as f32;
        rx -= cw;
        extra_xs.push((*col, rx, cw));
    }
    extra_xs.reverse();
    let name_max = (rx - name_x - 8.0).max(48.0);

    let track_len = if s.track_length > 1.0 { s.track_length } else { 1.0 };
    let speed = s.local_speed.max(4.0);
    let gold = Color::from_rgba8(196, 148, 40, ((a as u16 * 200) / 255).max(140) as u8);
    let stripe_c = Color::from_rgba8(22, 22, 24, ((a as u16 * 40) / 255) as u8);
    let out_c = Color::from_rgba8(110, 110, 116, 255);
    let dark = Color::from_rgba8(16, 12, 8, 255);

    let focus_st = standing_of(s, focus);
    let focus_pos_n = focus_st.map(|st| st.position).unwrap_or(0);
    if focus_pos_n > 0 {
        let ptxt = format!("P{focus_pos_n}");
        text(px, fonts, &ptxt, 12.0, x + w - 10.0 - measure(fonts, &ptxt, 12.0), y + 5.0, text_col(), false);
    }

    for (vis_i, oi) in show.iter().enumerate() {
        let (ri, wrapped) = order[*oi];
        let rider = &s.riders[ri];
        let ry = y + head_h + vis_i as f32 * row_h;
        let is_self = rider.race_num == focus;
        let st = standing_of(s, rider.race_num);
        let cat = st.map(|r| cstr(&r.category)).unwrap_or_default();
        let bike_name = st.map(|r| cstr(&r.bike)).unwrap_or_default();
        let accent_c = bike_color(&bike_name, &cat);
        let out = rider.crashed != 0
            || st.is_some_and(|r| r.crashed != 0 || matches!(r.state, 1 | 3 | 4));
        if is_self {
            if let Some(rrt) = rr(x + 2.0, ry, w - 4.0, row_h) {
                fill_rect(px, rrt, gold);
            }
        } else if vis_i % 2 == 1 && a > 0 {
            if let Some(rrt) = rr(x + 2.0, ry, w - 4.0, row_h) {
                fill_rect(px, rrt, stripe_c);
            }
        }
        if !is_self && wrapped > 0.0 {
            if let Some(rrt) = rr(x + w - 28.0, ry, 26.0, row_h) {
                fill_rect(px, rrt, Color::from_rgba8(120, 28, 24, 50));
            }
        }

        let name_c = if is_self {
            dark
        } else if out {
            out_c
        } else {
            text_col()
        };
        let dim = if is_self { dark } else if out { out_c } else { Color::from_rgba8(210, 210, 216, 255) };
        let pos = st.map(|r| r.position).unwrap_or(0);
        let pos_txt = if pos > 0 { format!("{pos}") } else { "--".into() };
        text(
            px,
            fonts,
            &pos_txt,
            12.0,
            x + pad + pos_w - measure(fonts, &pos_txt, 12.0),
            ry + 4.0,
            name_c,
            false,
        );
        fill_skew(px, x + pad + pos_w + 2.0, ry + 4.0, 5.0, row_h - 8.0, 3.0, accent_c);

        let bike = bike_name;
        let cat_lbl = if cat.is_empty() {
            String::new()
        } else {
            ellipsize(fonts, &cat.to_uppercase(), 9.0, 40.0)
        };
        let badge = if bike.is_empty() {
            String::new()
        } else {
            ellipsize(fonts, &bike, 9.0, 48.0)
        };
        let cat_w = if cat_lbl.is_empty() { 0.0 } else { measure(fonts, &cat_lbl, 9.0) + 10.0 };
        let badge_w = if badge.is_empty() { 0.0 } else { measure(fonts, &badge, 9.0) + 10.0 };
        let name = ellipsize(
            fonts,
            &cstr(&rider.name),
            12.0,
            (name_max - cat_w - badge_w - 10.0).max(24.0),
        );
        text(px, fonts, &name, 12.0, name_x, ry + 4.0, name_c, false);
        let mut bx = name_x + measure(fonts, &name, 12.0) + 6.0;
        if !cat_lbl.is_empty() {
            fill_round(px, bx, ry + 5.0, cat_w, 13.0, 3.0, accent_c);
            text(px, fonts, &cat_lbl, 9.0, bx + 5.0, ry + 6.0, dark, false);
            bx += cat_w + 4.0;
        }
        if !badge.is_empty() {
            fill_round(px, bx, ry + 5.0, badge_w, 13.0, 3.0, accent_c);
            text(px, fonts, &badge, 9.0, bx + 5.0, ry + 6.0, ink_on(accent_c), false);
        }

        for (kind, cx, cw) in &extra_xs {
            let (val, color) = match kind {
                RelField::Num => (format!("{}", rider.race_num), dim),
                RelField::Penalty => (st.map(|r| format_penalty(r.penalty_ms)).unwrap_or_else(|| "---".into()), dim),
                RelField::Interval => (st.map(|r| interval_text(s, r)).unwrap_or_else(|| "---".into()), dim),
                RelField::Crashed => {
                    if rider.crashed != 0 || st.is_some_and(|r| r.crashed != 0) {
                        ("CRASH".into(), behind_col())
                    } else {
                        ("".into(), dim)
                    }
                }
                _ => (String::new(), dim),
            };
            if !val.is_empty() {
                text(px, fonts, &val, 11.0, *cx + *cw - measure(fonts, &val, 11.0), ry + 4.5, color, false);
            }
        }

        let gap = if is_self {
            "0.0".into()
        } else {
            format!("{:.1}", ((wrapped * track_len) / speed).abs())
        };
        text(px, fonts, &gap, 12.0, gap_x + gap_w - measure(fonts, &gap, 12.0), ry + 4.0, dim, false);
    }

    let fy = y + h - foot_h + 3.0;
    let clock = format_session_clock(s.session_time_ms);
    let len = format_session_len(s.session_length);
    let race = if len.is_empty() {
        format!("RACE {clock}")
    } else {
        format!("RACE {clock} / {len}")
    };
    text(px, fonts, &race, 10.0, x + 8.0, fy, Color::from_rgba8(190, 190, 196, 255), false);
    let lap = if s.session_laps > 0 {
        format!("Lap {}/{}", s.current_lap.max(0), s.session_laps)
    } else if s.current_lap > 0 {
        format!("Lap {}", s.current_lap)
    } else {
        String::new()
    };
    if !lap.is_empty() {
        text(px, fonts, &lap, 10.0, x + w * 0.5 - measure(fonts, &lap, 10.0) * 0.5, fy, Color::from_rgba8(190, 190, 196, 255), false);
    }
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
            let fill = rider_fill(rider.race_num);
            draw_rider_dot(
                px,
                fonts,
                hx,
                hy,
                other_r,
                fill,
                rider_dot_num(s, rider.race_num, cfg.map_dot),
                cfg.map_numbers,
                false,
            );
            let (fwx, fwz) = yaw_forward(rider.yaw);
            let (sdx, sdy) = screen_dir(&to_px, rider.x, rider.z, fwx, fwz);
            draw_dot_chevron(px, hx, hy, other_r, sdx, sdy, fill, false);
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
        let (fwx, fwz) = local_forward(s);
        let (sdx, sdy) = screen_dir(&to_px, pred_x, pred_z, fwx, fwz);
        draw_dot_chevron(px, hx, hy, 8.5, sdx, sdy, you_col(), true);
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
            let fill = rider_fill(rider.race_num);
            numbered_dot(
                &mut mini,
                fonts,
                hx,
                hy,
                other_r,
                fill,
                rider_dot_num(s, rider.race_num, cfg.mini_dot),
                cfg.mini_numbers,
                false,
            );
            let (fwx, fwz) = yaw_forward(rider.yaw);
            let (sdx, sdy) = screen_dir(&to_px, rider.x, rider.z, fwx, fwz);
            draw_dot_chevron(&mut mini, hx, hy, other_r, sdx, sdy, fill, false);
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
        let (fwx, fwz) = local_forward(s);
        let (sdx, sdy) = screen_dir(&to_px, pred_x, pred_z, fwx, fwz);
        draw_dot_chevron(&mut mini, hx, hy, local_r, sdx, sdy, you_col(), true);
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

fn yaw_forward(yaw: f32) -> (f32, f32) {
    let yaw = if yaw.abs() > 6.5 { yaw.to_radians() } else { yaw };
    let (s, c) = yaw.sin_cos();
    (s, c)
}

fn local_forward(s: &Snapshot) -> (f32, f32) {
    let speed2 = s.local_vel_x * s.local_vel_x + s.local_vel_z * s.local_vel_z;
    if speed2 > 1.0 {
        let inv = 1.0 / speed2.sqrt();
        (s.local_vel_x * inv, s.local_vel_z * inv)
    } else {
        yaw_forward(s.local_yaw)
    }
}

fn screen_dir(to_px: &impl Fn(f32, f32) -> (f32, f32), wx: f32, wz: f32, hx: f32, hz: f32) -> (f32, f32) {
    let (x0, y0) = to_px(wx, wz);
    let (x1, y1) = to_px(wx + hx, wz + hz);
    let dx = x1 - x0;
    let dy = y1 - y0;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1.0e-4 {
        (0.0, -1.0)
    } else {
        (dx / len, dy / len)
    }
}

fn draw_dot_chevron(px: &mut Pixmap, x: f32, y: f32, r: f32, sdx: f32, sdy: f32, fill: Color, you: bool) {
    let ring = if you { 4.2 } else { 1.6 };
    let h = (r * 1.05).clamp(5.5, 11.0);
    let w = (r * 0.78).clamp(3.8, 8.0);
    let base = r + ring + 1.4;
    let bx = x + sdx * base;
    let by = y + sdy * base;
    let tip_x = x + sdx * (base + h);
    let tip_y = y + sdy * (base + h);
    let rx = -sdy;
    let ry = sdx;
    let mut pb = PathBuilder::new();
    pb.move_to(tip_x, tip_y);
    pb.line_to(bx + rx * w, by + ry * w);
    pb.line_to(bx - rx * w, by - ry * w);
    pb.close();
    let Some(path) = pb.finish() else {
        return;
    };
    fill_path(px, &path, fill);
    stroke_path(px, &path, Color::from_rgba8(8, 8, 10, 230), 1.4);
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
