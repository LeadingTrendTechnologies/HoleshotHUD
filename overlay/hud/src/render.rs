use std::cell::{Cell, RefCell};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Mutex;

use crate::config::{BoardField, DashField, DotLabel, FontFamily, HudConfig, RelField, StField};
use crate::shm::{cstr, Snapshot};
use fontdue::Font;
use tiny_skia::{
    Color, FillRule, GradientStop, LineCap, LineJoin, LinearGradient, Paint, Path, PathBuilder,
    Pixmap, PixmapPaint, Point as SkPoint, Rect, SpreadMode, Stroke, Transform,
};

fn accent() -> Color { Color::from_rgba8(255, 148, 48, 255) }
fn text_col() -> Color { Color::from_rgba8(228, 228, 230, 255) }
fn text_dim() -> Color { Color::from_rgba8(132, 132, 138, 255) }
fn panel_col() -> Color { Color::from_rgba8(10, 10, 10, 200) }
fn track_col() -> Color { Color::from_rgba8(236, 236, 240, 255) }
fn fill_col() -> Color { Color::from_rgba8(10, 8, 8, 168) }
fn you_col() -> Color { Color::from_rgba8(255, 148, 48, 255) }
fn other_col() -> Color { Color::from_rgba8(49, 68, 120, 255) }
fn ahead_col() -> Color { Color::from_rgba8(48, 220, 88, 255) }
fn behind_col() -> Color { Color::from_rgba8(255, 64, 72, 255) }

pub struct Fonts {
    pub ui: Font,
    pub bold: Font,
    pub icons: Font,
    bold_is_fake: bool,
}

thread_local! {
    static FACE: Cell<*const Font> = Cell::new(std::ptr::null());
    static SCALE: Cell<f32> = Cell::new(1.0);
    static FAKE_BOLD: Cell<bool> = Cell::new(false);
    static MAP_LAYER: RefCell<Option<(u64, Pixmap)>> = RefCell::new(None);
    static MINI_PX: RefCell<Option<Pixmap>> = RefCell::new(None);
    static ST_SLIDE: RefCell<TableSlides> = RefCell::new(TableSlides { rows: Vec::new() });
    static REL_SLIDE: RefCell<TableSlides> = RefCell::new(TableSlides { rows: Vec::new() });
    static HS_SCROLL: RefCell<IndexSlide> = RefCell::new(IndexSlide {
        from: 0.0,
        to: 0.0,
        start: 0.0,
        init: false,
    });
}

struct IndexSlide {
    from: f32,
    to: f32,
    start: f32,
    init: bool,
}

impl IndexSlide {
    fn step(&mut self, target: f32, now: f32) -> f32 {
        const DUR: f32 = 0.38;
        if !self.init {
            self.from = target;
            self.to = target;
            self.start = now;
            self.init = true;
            return target;
        }
        if (self.to - target).abs() > 0.02 {
            let t = ((now - self.start) / DUR).clamp(0.0, 1.0);
            self.from += (self.to - self.from) * ease_out_cubic(t);
            self.to = target;
            self.start = now;
        }
        let t = ((now - self.start) / DUR).clamp(0.0, 1.0);
        self.from + (self.to - self.from) * ease_out_cubic(t)
    }
}

struct RowSlide {
    id: i32,
    from: f32,
    to: f32,
    start: f32,
}

struct TableSlides {
    rows: Vec<RowSlide>,
}

fn anim_now() -> f32 {
    #[cfg(target_arch = "wasm32")]
    {
        thread_local! {
            static T: Cell<f32> = const { Cell::new(0.0) };
        }
        T.with(|c| {
            let v = c.get() + 1.0 / 60.0;
            c.set(v);
            v
        })
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        thread_local! {
            static ORIGIN: std::time::Instant = std::time::Instant::now();
        }
        ORIGIN.with(|o| o.elapsed().as_secs_f32())
    }
}

fn ease_out_cubic(t: f32) -> f32 {
    let u = 1.0 - t.clamp(0.0, 1.0);
    1.0 - u * u * u
}

impl TableSlides {
    fn step(&mut self, ids: &[i32], body_y: f32, row_h: f32, now: f32) -> Vec<f32> {
        const DUR: f32 = 0.30;
        let displayed = |e: &RowSlide| {
            let t = ((now - e.start) / DUR).clamp(0.0, 1.0);
            e.from + (e.to - e.from) * ease_out_cubic(t)
        };
        let mut out = Vec::with_capacity(ids.len());
        for (i, &id) in ids.iter().enumerate() {
            let target = i as f32;
            if let Some(idx) = self.rows.iter().position(|e| e.id == id) {
                let cur = displayed(&self.rows[idx]);
                let e = &mut self.rows[idx];
                if (e.to - target).abs() > 0.05 {
                    e.from = cur;
                    e.to = target;
                    e.start = now;
                }
                out.push(body_y + displayed(e) * row_h);
            } else {
                self.rows.push(RowSlide {
                    id,
                    from: target,
                    to: target,
                    start: now,
                });
                out.push(body_y + target * row_h);
            }
        }
        self.rows.retain(|e| ids.contains(&e.id));
        out
    }
}

fn row_ids(ids: impl IntoIterator<Item = i32>) -> Vec<i32> {
    ids.into_iter()
        .enumerate()
        .map(|(i, id)| if id > 0 { id } else { -(i as i32 + 1) })
        .collect()
}

struct StyleGuard;

impl Drop for StyleGuard {
    fn drop(&mut self) {
        FACE.with(|c| c.set(std::ptr::null()));
        SCALE.with(|c| c.set(1.0));
        FAKE_BOLD.with(|c| c.set(false));
    }
}

fn push_style(fonts: &Fonts, bold: bool, pct: i32) -> StyleGuard {
    let face = if bold { &fonts.bold } else { &fonts.ui };
    FACE.with(|c| c.set(face as *const Font));
    SCALE.with(|c| c.set((pct.clamp(70, 160) as f32) / 100.0));
    FAKE_BOLD.with(|c| c.set(bold && fonts.bold_is_fake));
    StyleGuard
}

fn style_k() -> f32 {
    SCALE.with(|c| c.get())
}

fn style_font(fonts: &Fonts) -> &Font {
    let ptr = FACE.with(|c| c.get());
    if ptr.is_null() {
        &fonts.ui
    } else {
        unsafe { &*ptr }
    }
}

fn font_from(bytes: &[u8]) -> Option<Font> {
    Font::from_bytes(bytes, fontdue::FontSettings::default()).ok()
}

impl Fonts {
    pub fn load() -> Option<Self> {
        Self::for_family(FontFamily::Segoe)
    }

    pub fn for_family(family: FontFamily) -> Option<Self> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(loaded) = Self::from_windows(family) {
                return Some(loaded);
            }
        }
        if let Some(loaded) = Self::from_bundled(family) {
            return Some(loaded);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            for fallback in [FontFamily::Segoe, FontFamily::Arial, FontFamily::Tahoma] {
                if fallback == family {
                    continue;
                }
                if let Some(loaded) = Self::from_windows(fallback) {
                    return Some(loaded);
                }
            }
        }
        Self::from_bundled(FontFamily::Roboto)
    }

    fn icons() -> Option<Font> {
        font_from(include_bytes!("../fonts/fa-solid-900.ttf").as_slice())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn from_windows(family: FontFamily) -> Option<Self> {
        let (reg, bld) = family.windows_files()?;
        let bytes = std::fs::read(reg).ok()?;
        let ui = font_from(&bytes)?;
        let (bold, bold_is_fake) = match std::fs::read(bld).ok().and_then(|b| font_from(&b)) {
            Some(bold) => (bold, false),
            None => (font_from(&bytes)?, true),
        };
        Some(Self {
            ui,
            bold,
            icons: Self::icons()?,
            bold_is_fake,
        })
    }

    fn from_bundled(family: FontFamily) -> Option<Self> {
        let icons = Self::icons()?;
        match family {
            FontFamily::Roboto => Self::embedded(
                include_bytes!("../fonts/Roboto-Regular.ttf"),
                None,
                icons,
            ),
            FontFamily::Agency => Self::embedded(
                include_bytes!("../fonts/Michroma-Regular.ttf"),
                None,
                icons,
            ),
            FontFamily::Industry => Self::embedded(
                include_bytes!("../fonts/Oswald-Regular.ttf"),
                Some(include_bytes!("../fonts/Oswald-Bold.ttf")),
                icons,
            ),
            FontFamily::FasterOne => Self::embedded(
                include_bytes!("../fonts/FasterOne-Regular.ttf"),
                None,
                icons,
            ),
            _ => None,
        }
    }

    fn embedded(regular: &[u8], bold: Option<&[u8]>, icons: Font) -> Option<Self> {
        let ui = font_from(regular)?;
        let (bold, bold_is_fake) = match bold.and_then(|b| font_from(b)) {
            Some(bold) => (bold, false),
            None => (font_from(regular)?, true),
        };
        Some(Self {
            ui,
            bold,
            icons,
            bold_is_fake,
        })
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
        let _g = push_style(fonts, cfg.st_bold, cfg.st_font);
        draw_standings(px, fonts, s, cfg, sw, sh);
    }
    if s.show_relative != 0 {
        let _g = push_style(fonts, cfg.rel_bold, cfg.rel_font);
        draw_relative(px, fonts, s, cfg, sw, sh);
    }
    if s.show_map != 0 {
        let _g = push_style(fonts, cfg.map_bold, cfg.map_font);
        draw_map(px, fonts, s, cfg, sw, sh, age);
    }
    if cfg.show_minimap {
        let _g = push_style(fonts, cfg.mini_bold, cfg.mini_font);
        draw_minimap(px, fonts, s, cfg, sw, sh, age);
    }
    if cfg.show_radar {
        let _g = push_style(fonts, cfg.radar_bold, cfg.radar_font);
        draw_radar(px, s, cfg, sw, sh, age);
    }
    if cfg.show_dash {
        let _g = push_style(fonts, cfg.dash_bold, cfg.dash_font);
        draw_dash(px, fonts, s, cfg, sw, sh);
    }
    if cfg.show_ticker {
        let _g = push_style(fonts, cfg.ticker_bold, cfg.ticker_font);
        draw_ticker(px, fonts, s, cfg, sw, sh);
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
        layout_box(px, s.standings_rect.x * sw, s.standings_rect.y * sh, s.standings_rect.w * sw, s.standings_rect.h * sh, false);
    }
    if s.show_relative != 0 {
        layout_box(px, s.relative.x * sw, s.relative.y * sh, s.relative.w * sw, s.relative.h * sh, false);
    }
    if s.show_map != 0 {
        layout_box(px, s.map.x * sw, s.map.y * sh, s.map.w * sw, s.map.h * sh, false);
    }
    if cfg.show_radar {
        layout_box(px, cfg.radar.x * sw, cfg.radar.y * sh, cfg.radar.w * sw, cfg.radar.h * sh, false);
    }
    if cfg.show_ticker {
        layout_box(px, cfg.ticker.x * sw, cfg.ticker.y * sh, cfg.ticker.w * sw, cfg.ticker.h * sh, true);
    }
    if cfg.show_dash {
        let _g = push_style(fonts, cfg.dash_bold, cfg.dash_font);
        let (dx, dy, dw, dh) = dash_box(fonts, s, cfg, sw, sh);
        layout_box(px, dx, dy, dw, dh, false);
    }
    if cfg.show_minimap {
        let rw = cfg.minimap.w * sw;
        let rh = cfg.minimap.h * sh;
        let d = rw.min(rh);
        let cx = cfg.minimap.x * sw + rw * 0.5;
        let cy = cfg.minimap.y * sh + rh * 0.5;
        layout_box(px, cx - d * 0.5, cy - d * 0.5, d, d, false);
    }
}

fn layout_box(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, ew_only: bool) {
    let mut pb = PathBuilder::new();
    pb.move_to(x, y);
    pb.line_to(x + w, y);
    pb.line_to(x + w, y + h);
    pb.line_to(x, y + h);
    pb.close();
    if let Some(path) = pb.finish() {
        stroke_path(px, &path, Color::from_rgba8(255, 148, 48, 200), 1.4);
    }
    let handles: &[(f32, f32)] = if ew_only {
        &[(x, y + h * 0.5), (x + w, y + h * 0.5)]
    } else {
        &[
            (x, y),
            (x + w, y),
            (x, y + h),
            (x + w, y + h),
            (x + w * 0.5, y),
            (x + w * 0.5, y + h),
            (x, y + h * 0.5),
            (x + w, y + h * 0.5),
        ]
    };
    for &(hx, hy) in handles {
        if let Some(r) = rr(hx - 5.0, hy - 5.0, 10.0, 10.0) {
            fill_rect(px, r, Color::from_rgba8(8, 8, 10, 230));
        }
        if let Some(r) = rr(hx - 4.0, hy - 4.0, 8.0, 8.0) {
            fill_rect(px, r, Color::from_rgba8(255, 148, 48, 255));
        }
    }
}

pub fn fill_rect(px: &mut Pixmap, r: Rect, c: Color) {
    let mut p = Paint::default();
    p.set_color(c);
    p.anti_alias = true;
    px.fill_rect(r, &p, Transform::identity(), None);
}

fn bg_a(pct: i32) -> u8 {
    ((pct.clamp(0, 100) as f32 / 100.0) * 255.0).round() as u8
}

pub fn text(px: &mut Pixmap, fonts: &Fonts, s: &str, size: f32, x: f32, y: f32, color: Color, center: bool) {
    draw_text(px, style_font(fonts), s, size, x, y, color, center, FAKE_BOLD.with(|c| c.get()));
}

fn text_bold(px: &mut Pixmap, fonts: &Fonts, s: &str, size: f32, x: f32, y: f32, color: Color, center: bool) {
    draw_text(px, &fonts.bold, s, size, x, y, color, center, fonts.bold_is_fake);
}

fn draw_text(
    px: &mut Pixmap,
    font: &Font,
    s: &str,
    size: f32,
    mut x: f32,
    y: f32,
    color: Color,
    center: bool,
    fake: bool,
) {
    let size = size * style_k();
    if center {
        x -= measure_font(font, s, size) * 0.5;
    }
    let rgba = [
        (color.red() * 255.0) as u8,
        (color.green() * 255.0) as u8,
        (color.blue() * 255.0) as u8,
        (color.alpha() * 255.0) as u8,
    ];
    let mut pen = x;
    for ch in s.chars() {
        let (metrics, bitmap) = font.rasterize(ch, size);
        let gx = pen + metrics.xmin as f32;
        let gy = y + size - metrics.ymin as f32 - metrics.height as f32;
        blit(px, &bitmap, metrics.width, metrics.height, gx, gy, rgba);
        if fake {
            blit(px, &bitmap, metrics.width, metrics.height, gx + 0.7, gy, rgba);
        }
        pen += metrics.advance_width;
    }
}

pub(crate) fn measure(fonts: &Fonts, s: &str, size: f32) -> f32 {
    let extra = if FAKE_BOLD.with(|c| c.get()) { 0.7 } else { 0.0 };
    measure_font(style_font(fonts), s, size * style_k()) + extra
}

fn measure_bold(fonts: &Fonts, s: &str, size: f32) -> f32 {
    let extra = if fonts.bold_is_fake { 0.7 } else { 0.0 };
    measure_font(&fonts.bold, s, size * style_k()) + extra
}

fn measure_font(font: &Font, s: &str, size: f32) -> f32 {
    s.chars().map(|ch| font.metrics(ch, size).advance_width).sum()
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

fn col_slots<T: Copy>(
    origin: f32,
    pad: f32,
    avail: f32,
    cols: &[T],
    mut width: impl FnMut(T) -> f32,
    is_flex: impl Fn(T) -> bool,
) -> Vec<(T, f32, f32)> {
    if cols.is_empty() {
        return Vec::new();
    }
    const GAP: f32 = 4.0;
    const MIN_W: f32 = 18.0;
    let mut widths: Vec<f32> = cols.iter().copied().map(&mut width).collect();
    let gaps = GAP * (cols.len() - 1) as f32;
    let inner = (avail - pad * 2.0).max(0.0);
    let used: f32 = widths.iter().sum::<f32>() + gaps;
    let leftover = inner - used;
    let flex = cols.iter().position(|&c| is_flex(c)).unwrap_or(cols.len() - 1);
    if leftover > 0.5 {
        widths[flex] += leftover;
    } else if leftover < -0.5 {
        let mut remain = -leftover;
        let shrink = (widths[flex] - MIN_W).max(0.0).min(remain);
        widths[flex] -= shrink;
        remain -= shrink;
        if remain > 0.5 {
            let room: Vec<f32> = widths.iter().map(|w| (*w - MIN_W).max(0.0)).collect();
            let total_room: f32 = room.iter().sum();
            if total_room > 0.0 {
                for (w, r) in widths.iter_mut().zip(room) {
                    if r <= 0.0 {
                        continue;
                    }
                    *w = (*w - remain * (r / total_room)).max(MIN_W);
                }
            }
        }
    }
    let mut x = origin + pad;
    let mut out = Vec::with_capacity(cols.len());
    for (i, &col) in cols.iter().enumerate() {
        out.push((col, x, widths[i]));
        x += widths[i] + GAP;
    }
    out
}

fn fill_focus_row(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, c: Color) {
    if let Some(rrt) = rr(x, y, w, h) {
        fill_rect(px, rrt, c);
    }
}

fn you_row_bg() -> Color {
    Color::from_rgba8(196, 132, 36, 88)
}

fn col_text(
    px: &mut Pixmap,
    fonts: &Fonts,
    s: &str,
    size: f32,
    x: f32,
    w: f32,
    y: f32,
    color: Color,
    right: bool,
) {
    if s.is_empty() {
        return;
    }
    let t = ellipsize(fonts, s, size, w.max(8.0));
    let tx = if right {
        x + w - measure(fonts, &t, size)
    } else {
        x
    };
    text(px, fonts, &t, size, tx, y, color, false);
}

fn draw_count_track(px: &mut Pixmap, fonts: &Fonts, x: f32, cy: f32, n: usize, track: &str) {
    let count = format!("{n}");
    let track_col = accent();
    let ink = Color::from_rgba8(12, 12, 14, 255);
    let icon_x = x + 12.0;
    let num_x = icon_x + 14.0;
    let count_w = (num_x - (x + 8.0) + measure(fonts, &count, 9.0) + 7.0).max(36.0);
    fill_skew(px, x + 8.0, cy + 3.0, count_w, 14.0, 4.0, track_col);
    icon(px, fonts, '\u{f553}', 8.0, icon_x, cy + 5.0, ink, false);
    text(px, fonts, &count, 9.0, num_x, cy + 4.5, ink, false);
    let tx = x + 8.0 + count_w + 4.0;
    fill_skew(px, tx, cy + 3.0, (measure(fonts, track, 10.0) + 14.0).max(36.0), 14.0, 4.0, track_col);
    text(px, fonts, track, 10.0, tx + 8.0, cy + 4.5, ink, false);
}

fn st_col_header(col: StField) -> &'static str {
    match col {
        StField::Pos => "P",
        StField::Num => "#",
        StField::Name => "NAME",
        StField::Laps => "Completed Laps",
        StField::Current => "Current Lap",
        StField::Best => "Fastest",
        StField::Last => "Last",
        StField::Status => "ST",
        StField::Gap => "GAP",
        StField::Interval => "INT",
        StField::Bike => "BIKE",
        StField::Penalty => "PEN",
        StField::Crashed => "CR",
    }
}

fn rel_col_header(col: RelField) -> &'static str {
    match col {
        RelField::Pos => "P",
        RelField::Num => "#",
        RelField::Name => "NAME",
        RelField::Gap => "Gap",
        RelField::Laps => "Completed Laps",
        RelField::Current => "Current Lap",
        RelField::Bike => "BIKE",
        RelField::Penalty => "Pen",
        RelField::Interval => "Int",
        RelField::Crashed => "Crash",
        RelField::Best => "Fastest",
        RelField::Last => "Last",
    }
}

fn bike_color(bike: &str, extra: &str) -> Color {
    let hay = format!("{bike} {extra}")
        .to_ascii_lowercase()
        .replace(['-', '_'], " ");
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
    } else if has("ktm") || has("sx f") || has("xc f") || tok("sxf") || tok("xcf") || tok("exc") || tok("sx") {
        Color::from_rgba8(255, 96, 0, 255)
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
            (80, 214, 96),
            (232, 132, 48),
            (48, 128, 232),
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

const BIKE_BAR_W: f32 = 5.0;
const BIKE_BAR_SKEW: f32 = 3.0;
const BIKE_BAR_PAD: f32 = 5.0;

fn bike_bar_end(pos_cx: f32, pos_cw: f32) -> f32 {
    pos_cx + pos_cw + 1.0 + BIKE_BAR_W + BIKE_BAR_SKEW
}

fn name_left_pad(cx: f32, bar_end: Option<f32>) -> f32 {
    bar_end.map(|end| (end + BIKE_BAR_PAD - cx).max(0.0)).unwrap_or(0.0)
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

fn session_len_ms(len: i32) -> i32 {
    if len <= 0 {
        0
    } else if len >= 1_000 {
        len
    } else if len >= 60 {
        len * 1000
    } else {
        len * 60_000
    }
}

fn session_len_minutes(ms: i32) -> i32 {
    if ms < 60_000 {
        0
    } else {
        (ms + 30_000) / 60_000
    }
}

fn leftover_practice_len(ms: i32) -> bool {
    ms >= 30 * 60_000
}

fn race_clock_ms(clock: i32) -> bool {
    clock >= 5 * 60_000 && clock <= 20 * 60_000
}

fn wait_display_ms(total: i32, clock: i32) -> i32 {
    if total > 0 {
        total
    } else if race_clock_ms(clock) {
        clock
    } else {
        0
    }
}

fn standard_race_minutes(ms: i32) -> bool {
    matches!(session_len_minutes(ms), 5 | 6 | 8 | 10 | 12 | 15 | 20)
}

fn effective_session_len_ms(s: &Snapshot) -> i32 {
    let total = session_len_ms(s.session_length);
    if s.session_laps > 0 && leftover_practice_len(total) {
        0
    } else {
        total
    }
}

fn format_countdown(remain_ms: i32) -> String {
    let t = remain_ms.max(0) / 1000;
    if t >= 3600 {
        format!("{:02}:{:02}:{:02}", t / 3600, (t / 60) % 60, t % 60)
    } else {
        format!("{:02}:{:02}", t / 60, t % 60)
    }
}

fn race_lap(s: &Snapshot) -> i32 {
    if s.current_lap > 0 {
        return s.current_lap;
    }
    s.standings
        .iter()
        .take(s.standing_count.max(0) as usize)
        .map(|row| row.num_laps)
        .max()
        .unwrap_or(0)
        .max(0)
}

static STATUS_HINT: Mutex<String> = Mutex::new(String::new());

pub fn set_status_hint(s: impl Into<String>) {
    if let Ok(mut g) = STATUS_HINT.lock() {
        *g = s.into();
    }
}

static LAST_SESSION_CLOCK: AtomicI32 = AtomicI32::new(0);
static SESSION_CLOCK_MODE: AtomicI32 = AtomicI32::new(0);
static SAW_SESSION_TIME: AtomicI32 = AtomicI32::new(0);
static SESSION_EXPIRED: AtomicI32 = AtomicI32::new(0);
static OVERTIME_BASE_LAP: AtomicI32 = AtomicI32::new(-1);
static OVERTIME_LOCAL_BASE: AtomicI32 = AtomicI32::new(-1);
static CACHED_SESSION_LAPS: AtomicI32 = AtomicI32::new(0);
static CHECKERED_LATCH: AtomicI32 = AtomicI32::new(0);
static LAST_SESSION_SIG: AtomicI32 = AtomicI32::new(0);
static LAST_CUR_LAP: AtomicI32 = AtomicI32::new(0);
static IN_GATE: AtomicI32 = AtomicI32::new(0);
static POST_GATE: AtomicI32 = AtomicI32::new(0);
static LAP_GREEN: AtomicI32 = AtomicI32::new(0);
static LOCKED_SESSION_LEN: AtomicI32 = AtomicI32::new(0);
static RACE_ARMED: AtomicI32 = AtomicI32::new(0);

fn reset_session_clock_track() {
    LAST_SESSION_CLOCK.store(0, Ordering::Relaxed);
    SESSION_CLOCK_MODE.store(0, Ordering::Relaxed);
    SAW_SESSION_TIME.store(0, Ordering::Relaxed);
    SESSION_EXPIRED.store(0, Ordering::Relaxed);
    OVERTIME_BASE_LAP.store(-1, Ordering::Relaxed);
    OVERTIME_LOCAL_BASE.store(-1, Ordering::Relaxed);
    CACHED_SESSION_LAPS.store(0, Ordering::Relaxed);
    CHECKERED_LATCH.store(0, Ordering::Relaxed);
    IN_GATE.store(0, Ordering::Relaxed);
    POST_GATE.store(0, Ordering::Relaxed);
    LAP_GREEN.store(0, Ordering::Relaxed);
    LOCKED_SESSION_LEN.store(0, Ordering::Relaxed);
    RACE_ARMED.store(0, Ordering::Relaxed);
}

fn session_sig(s: &Snapshot) -> i32 {
    let raw = s.session_length.max(0);
    let locked = LOCKED_SESSION_LEN.load(Ordering::Relaxed);
    if raw > locked {
        LOCKED_SESSION_LEN.store(raw, Ordering::Relaxed);
    } else if raw > 0 && locked >= 60 && raw < 60 {
        LOCKED_SESSION_LEN.store(raw, Ordering::Relaxed);
    } else if raw > 0 && locked > raw && locked < 60 && raw < 60 && s.session_laps > 0 {
        LOCKED_SESSION_LEN.store(raw, Ordering::Relaxed);
    }
    LOCKED_SESSION_LEN.load(Ordering::Relaxed).max(raw)
}

fn note_session(s: &Snapshot) {
    let sig = session_sig(s);
    let prev = LAST_SESSION_SIG.swap(sig, Ordering::Relaxed);
    if prev != 0 && prev != sig {
        reset_session_clock_track();
        LOCKED_SESSION_LEN.store(s.session_length.max(0), Ordering::Relaxed);
        LAST_SESSION_SIG.store(session_sig(s), Ordering::Relaxed);
    }
    let lap = s.current_lap.max(0);
    let prev_lap = LAST_CUR_LAP.swap(lap, Ordering::Relaxed);
    if prev_lap > 1 && lap <= 1 && POST_GATE.load(Ordering::Relaxed) == 0 {
        reset_session_clock_track();
    }
}

fn extra_laps(s: &Snapshot) -> i32 {
    if is_lap_race(s) {
        return 0;
    }
    if s.session_laps > 0 {
        CACHED_SESSION_LAPS.store(s.session_laps, Ordering::Relaxed);
        s.session_laps
    } else if SESSION_EXPIRED.load(Ordering::Relaxed) == 1 {
        CACHED_SESSION_LAPS.load(Ordering::Relaxed)
    } else {
        0
    }
}

fn is_lap_race(s: &Snapshot) -> bool {
    // Extra laps are 1–2, sometimes 3. Four or more is always a lap moto,
    // even when leftover warmup (10:00) is still sitting in session length.
    if s.session_laps >= 4 {
        return true;
    }
    if s.session_laps < 3 {
        return false;
    }
    let total = session_len_ms(s.session_length);
    if leftover_practice_len(total) || standard_race_minutes(total) {
        return false;
    }
    true
}

fn lap_race_text(s: &Snapshot) -> String {
    let n = s.session_laps.max(1);
    let done = focus_num_laps(s).max(0);
    let lap = (done + 1).clamp(1, n);
    format!("{lap} / {n}")
}

fn leader_num_laps(s: &Snapshot) -> i32 {
    let rows = s.standing_count.max(0) as usize;
    s.standings
        .iter()
        .take(rows)
        .find(|row| row.position == 1)
        .map(|row| row.num_laps)
        .or_else(|| s.standings.iter().take(rows).map(|row| row.num_laps).max())
        .unwrap_or(0)
        .max(0)
}

fn overtime_base(s: &Snapshot) -> i32 {
    let mut base = OVERTIME_BASE_LAP.load(Ordering::Relaxed);
    let local = focus_num_laps(s);
    let local0 = OVERTIME_LOCAL_BASE.load(Ordering::Relaxed);
    if local0 < 0 || (local0 == 0 && local > 0) {
        OVERTIME_LOCAL_BASE.store(local, Ordering::Relaxed);
    }
    if base < 0 {
        base = leader_num_laps(s);
        OVERTIME_BASE_LAP.store(base, Ordering::Relaxed);
    }
    base
}

fn extras_started(s: &Snapshot) -> bool {
    overtime_active(s) && leader_num_laps(s) > overtime_base(s)
}

fn local_overtime_done(s: &Snapshot) -> i32 {
    let local = focus_num_laps(s);
    if local <= 0 {
        return 0;
    }
    let _ = overtime_base(s);
    // Backmarkers who cross after the clock hits 0 are still finishing the timed lap.
    // Extra laps start when the leader next crosses, then local crosses count.
    if !extras_started(s) {
        OVERTIME_LOCAL_BASE.store(local, Ordering::Relaxed);
        return 0;
    }
    let local0 = OVERTIME_LOCAL_BASE.load(Ordering::Relaxed);
    if local0 < 0 {
        OVERTIME_LOCAL_BASE.store(local, Ordering::Relaxed);
        return 0;
    }
    (local - local0).max(0)
}

/// Extra laps after time are counted from the leader's finish-line crossings.
/// Last lap is the Nth extra lap, so +2L is two crossings after time expires.
fn overtime_last_at(n: i32) -> i32 {
    n.max(1)
}

fn last_lap_arm_laps(s: &Snapshot) -> i32 {
    overtime_base(s) + overtime_last_at(extra_laps(s).max(1))
}

fn race_last_armed(s: &Snapshot) -> bool {
    if !overtime_active(s) || extra_laps(s) <= 0 {
        return false;
    }
    let _ = overtime_base(s);
    OVERTIME_BASE_LAP.load(Ordering::Relaxed) >= 0 && leader_num_laps(s) >= last_lap_arm_laps(s)
}

fn overtime_lap_text(s: &Snapshot) -> String {
    let n = extra_laps(s).max(1);
    let done = local_overtime_done(s).min(n);
    format!("{done} / {n}")
}

fn moving(s: &Snapshot) -> bool {
    s.local_speed >= 3.5
}

fn near_session_total(clock: i32, total: i32) -> bool {
    if total <= 0 {
        return false;
    }
    clock >= total || (clock - total).abs() < 30_000
}

fn is_gate_clock(clock: i32, total: i32) -> bool {
    // Start boards are 2:00 / 0:50 / 0:30 / ~15s. Practice remaining at 3:00 is not a board.
    let long_session = total > 180_000 || total <= 0;
    long_session && clock >= 8_000 && clock <= 120_000 && (total <= 0 || clock * 3 < total)
}

fn clock_stuck(clock: i32, last: i32) -> bool {
    static HOLD: Mutex<Option<(i32, f32)>> = Mutex::new(None);
    clock_held_for(clock, last, 2.5, &HOLD)
}

fn board_held(clock: i32, last: i32) -> bool {
    static HOLD: Mutex<Option<(i32, f32)>> = Mutex::new(None);
    clock_held_for(clock, last, 0.6, &HOLD)
}

fn clock_held_for(clock: i32, last: i32, secs: f32, hold: &Mutex<Option<(i32, f32)>>) -> bool {
    let Ok(mut hold) = hold.lock() else {
        return false;
    };
    let now = anim_now();
    if last > 0 && clock == last {
        match *hold {
            Some((prev, at)) if prev == clock => now - at >= secs,
            _ => {
                *hold = Some((clock, now));
                false
            }
        }
    } else {
        *hold = Some((clock, now));
        false
    }
}

fn session_remain_ms(s: &Snapshot) -> Option<i32> {
    note_session(s);
    if is_lap_race(s) {
        let clock = s.session_time_ms.max(0);
        let last = LAST_SESSION_CLOCK.load(Ordering::Relaxed);
        LAST_SESSION_CLOCK.store(clock, Ordering::Relaxed);
        let counting_up = last > 0 && clock > last + 400;
        let counting_down = last > 0
            && clock < last
            && last - clock < 5_000
            && clock <= 180_000;
        if LAP_GREEN.load(Ordering::Relaxed) == 1 {
            IN_GATE.store(0, Ordering::Relaxed);
            return None;
        }
        if counting_up || (moving(s) && counting_down && last <= 8_000) {
            LAP_GREEN.store(1, Ordering::Relaxed);
            IN_GATE.store(0, Ordering::Relaxed);
            POST_GATE.store(1, Ordering::Relaxed);
            return None;
        }
        if POST_GATE.load(Ordering::Relaxed) == 1 {
            IN_GATE.store(0, Ordering::Relaxed);
            return None;
        }
        let in_gate = IN_GATE.load(Ordering::Relaxed) == 1;
        if in_gate && last > 0 && (clock > last + 400 || clock < 500 || clock > 180_000) {
            POST_GATE.store(1, Ordering::Relaxed);
            IN_GATE.store(0, Ordering::Relaxed);
            return None;
        }
        if clock > 0 && clock <= 180_000 {
            IN_GATE.store(1, Ordering::Relaxed);
            RACE_ARMED.store(0, Ordering::Relaxed);
            SESSION_EXPIRED.store(0, Ordering::Relaxed);
            return Some(clock);
        }
        if in_gate {
            POST_GATE.store(1, Ordering::Relaxed);
        }
        if moving(s) {
            LAP_GREEN.store(1, Ordering::Relaxed);
        }
        IN_GATE.store(0, Ordering::Relaxed);
        return None;
    }
    let total = effective_session_len_ms(s);
    if total <= 0 && session_len_ms(s.session_length) <= 0 && s.session_laps <= 0 {
        if RACE_ARMED.load(Ordering::Relaxed) == 1
            && SESSION_EXPIRED.load(Ordering::Relaxed) == 1
            && extra_laps(s) > 0
        {
            return Some(0);
        }
        if s.session_time_ms > 0 {
            return Some(s.session_time_ms);
        }
        return None;
    }
    let last = LAST_SESSION_CLOCK.load(Ordering::Relaxed);
    let raw = s.session_time_ms.max(0);
    let saw = SAW_SESSION_TIME.load(Ordering::Relaxed) == 1;
    let armed = RACE_ARMED.load(Ordering::Relaxed) == 1;
    let in_gate_now = IN_GATE.load(Ordering::Relaxed) == 1;
    let mut clock = if raw > total + 30_000 && total > 0 && last > 0 && !in_gate_now && armed {
        last
    } else {
        raw
    };
    // Game republishes the 8s/10s board during a live race; keep the real remaining clock.
    if armed
        && last > 60_000
        && clock >= 8_000
        && clock <= 15_000
        && clock + 20_000 < last
    {
        clock = last;
    }
    let near_full = near_session_total(clock, total);
    let last_near_full = last > 0 && near_session_total(last, total);
    let started = moving(s) || s.current_lap > 1;
    // After a real race countdown, a snap back to session length (or 8s junk) is expiry.
    if saw && armed && last > 0 && last + 30_000 < total && extra_laps(s) > 0 && last <= 90_000 {
        let jump_to_length = (near_full || clock >= total) && clock > last + 20_000;
        let jump_off_zero = last <= 15_000 && clock > last + 4_000;
        if jump_to_length || jump_off_zero {
            LAST_SESSION_CLOCK.store(last, Ordering::Relaxed);
            SESSION_EXPIRED.store(1, Ordering::Relaxed);
            let _ = overtime_base(s);
            return Some(0);
        }
    }
    if saw
        && armed
        && last > 0
        && last + 30_000 < total
        && near_full
        && clock > last + 20_000
    {
        clock = last;
    }
    if s.session_laps <= 0
        && last > 0
        && last <= 20_000
        && near_full
        && last + 30_000 < total
    {
        LAST_SESSION_CLOCK.store(0, Ordering::Relaxed);
        IN_GATE.store(0, Ordering::Relaxed);
        return Some(0);
    }
    LAST_SESSION_CLOCK.store(clock, Ordering::Relaxed);
    let in_gate = IN_GATE.load(Ordering::Relaxed) == 1;
    let mut post = POST_GATE.load(Ordering::Relaxed) == 1;
    let prestart_sized = |t: i32| t > 0 && t <= 120_000 && total > 180_000 && t * 3 < total;
    if s.session_laps > 0 && prestart_sized(last) && near_full && !armed {
        POST_GATE.store(1, Ordering::Relaxed);
        post = true;
    }
    let short_clock = clock > 0 && clock <= 35_000 && (total <= 0 || clock * 3 < total) && total > 180_000;
    let enter_gate =
        is_gate_clock(clock, total) && !moving(s) && !post && !armed && s.session_laps > 0;
    let gate_clock = enter_gate
        || (!post && in_gate && clock > 0 && clock <= 180_000 && (total <= 0 || clock * 3 < total));
    // Frozen 00:30 wait is a race board. Practice remaining that hitch-pauses is not.
    let held_board = short_clock && board_held(clock, last) && !armed && s.session_laps > 0;
    if held_board {
        IN_GATE.store(0, Ordering::Relaxed);
        POST_GATE.store(1, Ordering::Relaxed);
        SESSION_CLOCK_MODE.store(1, Ordering::Relaxed);
        let shown = wait_display_ms(total, clock);
        LAST_SESSION_CLOCK.store(shown.max(total), Ordering::Relaxed);
        return Some(shown);
    }
    let drop_off_gate = in_gate && last >= 8_000 && clock < 500;
    let leave_gate = in_gate && !gate_clock && !near_full && clock > 180_000;
    let wait_off_gate = in_gate && !gate_clock && near_full;
    let board_restart = in_gate && !post && last > 0 && clock > last + 2_000 && clock <= 180_000;
    let gate = gate_clock && !drop_off_gate && !board_restart && !armed;
    let race_ticking = !gate_clock
        && last > 180_000
        && clock > 5_000
        && clock < last
        && last - clock >= 50
        && last - clock < 5_000;
    let waiting_for_race = (post || board_restart) && !armed && !race_ticking;

    if drop_off_gate {
        IN_GATE.store(0, Ordering::Relaxed);
        POST_GATE.store(1, Ordering::Relaxed);
        SESSION_CLOCK_MODE.store(1, Ordering::Relaxed);
        SESSION_EXPIRED.store(0, Ordering::Relaxed);
        let shown = wait_display_ms(total, clock);
        LAST_SESSION_CLOCK.store(shown.max(total), Ordering::Relaxed);
        if !armed {
            return Some(shown);
        }
        RACE_ARMED.store(1, Ordering::Relaxed);
    } else if leave_gate {
        IN_GATE.store(0, Ordering::Relaxed);
        POST_GATE.store(1, Ordering::Relaxed);
        SESSION_CLOCK_MODE.store(1, Ordering::Relaxed);
        SESSION_EXPIRED.store(0, Ordering::Relaxed);
        SAW_SESSION_TIME.store(0, Ordering::Relaxed);
        OVERTIME_BASE_LAP.store(-1, Ordering::Relaxed);
        OVERTIME_LOCAL_BASE.store(-1, Ordering::Relaxed);
        CHECKERED_LATCH.store(0, Ordering::Relaxed);
        let shown = wait_display_ms(total, clock);
        LAST_SESSION_CLOCK.store(shown.max(total), Ordering::Relaxed);
        return Some(shown);
    } else if wait_off_gate {
        IN_GATE.store(0, Ordering::Relaxed);
        POST_GATE.store(1, Ordering::Relaxed);
        SESSION_CLOCK_MODE.store(1, Ordering::Relaxed);
        let shown = wait_display_ms(total, clock);
        LAST_SESSION_CLOCK.store(shown.max(total), Ordering::Relaxed);
        return Some(shown);
    } else if waiting_for_race {
        IN_GATE.store(0, Ordering::Relaxed);
        POST_GATE.store(1, Ordering::Relaxed);
        SESSION_CLOCK_MODE.store(1, Ordering::Relaxed);
        let shown = wait_display_ms(total, clock);
        LAST_SESSION_CLOCK.store(shown.max(total), Ordering::Relaxed);
        return Some(shown);
    } else if gate {
        IN_GATE.store(1, Ordering::Relaxed);
        POST_GATE.store(0, Ordering::Relaxed);
        SESSION_CLOCK_MODE.store(1, Ordering::Relaxed);
        SESSION_EXPIRED.store(0, Ordering::Relaxed);
        SAW_SESSION_TIME.store(0, Ordering::Relaxed);
        OVERTIME_BASE_LAP.store(-1, Ordering::Relaxed);
        OVERTIME_LOCAL_BASE.store(-1, Ordering::Relaxed);
        CHECKERED_LATCH.store(0, Ordering::Relaxed);
        RACE_ARMED.store(0, Ordering::Relaxed);
    } else if race_ticking {
        RACE_ARMED.store(1, Ordering::Relaxed);
        POST_GATE.store(0, Ordering::Relaxed);
        SESSION_CLOCK_MODE.store(1, Ordering::Relaxed);
        IN_GATE.store(0, Ordering::Relaxed);
    } else if !gate
        && IN_GATE.load(Ordering::Relaxed) == 0
        && RACE_ARMED.load(Ordering::Relaxed) == 1
        && extra_laps(s) > 0
        && SAW_SESSION_TIME.load(Ordering::Relaxed) == 1
        && started
        && last > 0
        && last <= 90_000
        && !last_near_full
        && last + 30_000 < total
        && near_full
        && clock > last + 20_000
    {
        SESSION_EXPIRED.store(1, Ordering::Relaxed);
        let _ = overtime_base(s);
        return Some(0);
    } else if near_full
        && SESSION_EXPIRED.load(Ordering::Relaxed) == 0
        && !(SESSION_CLOCK_MODE.load(Ordering::Relaxed) == 2 && last_near_full && started)
    {
        SESSION_CLOCK_MODE.store(1, Ordering::Relaxed);
        SESSION_EXPIRED.store(0, Ordering::Relaxed);
        OVERTIME_BASE_LAP.store(-1, Ordering::Relaxed);
        OVERTIME_LOCAL_BASE.store(-1, Ordering::Relaxed);
        CHECKERED_LATCH.store(0, Ordering::Relaxed);
    } else if !gate && last > 0 && clock + 400 < last {
        SESSION_CLOCK_MODE.store(1, Ordering::Relaxed);
    } else if last > 0
        && clock > last + 400
        && !near_full
        && !gate
        && clock * 4 < total * 3
        && last * 4 < total * 3
        && last > 45_000
    {
        SESSION_CLOCK_MODE.store(2, Ordering::Relaxed);
    }

    let cap = |c: i32| if total > 0 { c.min(total) } else { c.max(0) };
    let remain = if gate {
        cap(clock)
    } else {
        match SESSION_CLOCK_MODE.load(Ordering::Relaxed) {
            2 => (total - clock).max(0),
            1 => {
                if clock <= 0 {
                    if started && last > 2000 && !near_full && !drop_off_gate && !leave_gate {
                        0
                    } else {
                        total
                    }
                } else {
                    cap(clock)
                }
            }
            _ => {
                if clock <= 0 {
                    total
                } else {
                    cap(clock)
                }
            }
        }
    };

    if !gate && remain > 4000 {
        SAW_SESSION_TIME.store(1, Ordering::Relaxed);
    }

    if SESSION_EXPIRED.load(Ordering::Relaxed) == 1 {
        if near_full && remain > 20_000 && !started {
            SESSION_EXPIRED.store(0, Ordering::Relaxed);
            OVERTIME_BASE_LAP.store(-1, Ordering::Relaxed);
            OVERTIME_LOCAL_BASE.store(-1, Ordering::Relaxed);
            CHECKERED_LATCH.store(0, Ordering::Relaxed);
            return Some(remain);
        }
        return Some(0);
    }

    let timed_out = remain <= 800
        || (clock_stuck(clock, last) && remain > 0 && remain <= 2_000);
    let raced = RACE_ARMED.load(Ordering::Relaxed) == 1
        || (SAW_SESSION_TIME.load(Ordering::Relaxed) == 1 && last > 60_000);
    if !gate
        && started
        && timed_out
        && extra_laps(s) > 0
        && raced
        && SAW_SESSION_TIME.load(Ordering::Relaxed) == 1
    {
        RACE_ARMED.store(1, Ordering::Relaxed);
        SESSION_EXPIRED.store(1, Ordering::Relaxed);
        let _ = overtime_base(s);
        return Some(0);
    }
    Some(remain)
}

fn overtime_active(s: &Snapshot) -> bool {
    extra_laps(s) > 0
        && RACE_ARMED.load(Ordering::Relaxed) == 1
        && SAW_SESSION_TIME.load(Ordering::Relaxed) == 1
        && SESSION_EXPIRED.load(Ordering::Relaxed) == 1
}

fn prestart(s: &Snapshot) -> bool {
    if IN_GATE.load(Ordering::Relaxed) == 1 {
        return true;
    }
    if overtime_active(s) {
        return false;
    }
    if moving(s) {
        return false;
    }
    let total = session_len_ms(s.session_length);
    let clock = s.session_time_ms.max(0);
    s.current_lap <= 1 || laps_done(s) <= 0 || (total > 0 && near_session_total(clock, total))
}

fn norm_track_pos(s: &Snapshot, pos: f32) -> f32 {
    if pos < 0.0 {
        return pos;
    }
    let mut u = pos;
    if u > 1.5 {
        if s.track_length > 10.0 {
            u = (u / s.track_length).rem_euclid(1.0);
        } else {
            u = u.rem_euclid(1.0);
        }
    } else {
        u = u.rem_euclid(1.0);
    }
    u
}

fn focus_track_pos(s: &Snapshot) -> f32 {
    let focus = if s.focus_race_num > 0 {
        s.focus_race_num
    } else {
        s.local_race_num
    };
    let raw = s
        .riders
        .iter()
        .take(s.rider_count.max(0) as usize)
        .find(|r| r.race_num == focus)
        .map(|r| r.track_pos)
        .filter(|p| *p >= 0.0)
        .unwrap_or(s.local_track_pos);
    norm_track_pos(s, raw)
}

fn sf_frac(s: &Snapshot) -> f32 {
    if s.track_length > 1.0 && s.sf_meters >= 0.0 {
        (s.sf_meters / s.track_length).rem_euclid(1.0)
    } else {
        0.0
    }
}

fn dist_to_sf(pos: f32, sf: f32) -> f32 {
    let d = sf - pos;
    if d <= 0.0 {
        d + 1.0
    } else {
        d
    }
}

fn meters_to_sf(s: &Snapshot) -> Option<f32> {
    let pos = focus_track_pos(s);
    if pos < 0.0 {
        return None;
    }
    let remain = dist_to_sf(pos, sf_frac(s));
    if s.track_length > 10.0 {
        Some(remain * s.track_length)
    } else {
        Some(remain * 1200.0)
    }
}

const FLAG_LINE_M: f32 = 40.0;
const FLAG_LINE_MIN_M: f32 = 4.0;

fn approaching_line(s: &Snapshot) -> bool {
    meters_to_sf(s).is_some_and(|m| m > FLAG_LINE_MIN_M && m <= FLAG_LINE_M)
}

#[cfg(test)]
fn approaching_sf(s: &Snapshot) -> bool {
    approaching_line(s)
}

#[cfg(test)]
fn approaching_finish(s: &Snapshot) -> bool {
    approaching_line(s)
}

fn just_after_sf(s: &Snapshot) -> bool {
    let pos = focus_track_pos(s);
    if pos < 0.0 {
        return false;
    }
    let remain = dist_to_sf(pos, sf_frac(s));
    if s.track_length > 10.0 {
        let past = (1.0 - remain) * s.track_length;
        remain > 0.55 && past <= 40.0
    } else {
        remain >= 0.96
    }
}

fn laps_done(s: &Snapshot) -> i32 {
    let st = focus_num_laps(s);
    let on = s.current_lap;
    if on > 0 && st >= on + 2 {
        (on - 1).max(0)
    } else {
        st
    }
}

fn rider_current_lap(s: &Snapshot, race_num: i32, num_laps: i32) -> i32 {
    let done = num_laps.max(0);
    let focus = if s.focus_race_num > 0 {
        s.focus_race_num
    } else {
        s.local_race_num
    };
    if race_num == focus && s.current_lap > done {
        s.current_lap
    } else {
        done + 1
    }
}

fn focus_num_laps(s: &Snapshot) -> i32 {
    let focus = if s.focus_race_num > 0 {
        s.focus_race_num
    } else {
        s.local_race_num
    };
    s.standings
        .iter()
        .take(s.standing_count.max(0) as usize)
        .find(|row| row.race_num == focus)
        .map(|row| row.num_laps)
        .unwrap_or_else(|| {
            if s.current_lap > 0 {
                (s.current_lap - 1).max(0)
            } else {
                0
            }
        })
        .max(0)
}

fn timed_clock_live(s: &Snapshot) -> bool {
    !is_lap_race(s) && !overtime_active(s) && s.session_time_ms > 1000
}

fn lap_race_racing(s: &Snapshot) -> bool {
    is_lap_race(s)
        && IN_GATE.load(Ordering::Relaxed) == 0
        && (LAP_GREEN.load(Ordering::Relaxed) == 1 || laps_done(s) > 0 || s.current_lap > 1)
}

fn i_finished(s: &Snapshot) -> bool {
    if prestart(s) || timed_clock_live(s) {
        return false;
    }
    if lap_race_racing(s) {
        return laps_done(s) >= s.session_laps;
    }
    let n = extra_laps(s);
    if n <= 0 {
        return false;
    }
    if s.session_length > 0 && !overtime_active(s) {
        return false;
    }
    if overtime_active(s) {
        return race_last_armed(s) && laps_done(s) > last_lap_arm_laps(s);
    }
    laps_done(s) >= n
}

fn laps_left(s: &Snapshot) -> Option<i32> {
    if prestart(s) || timed_clock_live(s) {
        return None;
    }
    if lap_race_racing(s) {
        return Some((s.session_laps - laps_done(s)).max(0));
    }
    if overtime_active(s) {
        let n = extra_laps(s).max(1);
        return Some((n - local_overtime_done(s)).max(0));
    }
    None
}

fn dash_race_flag(s: &Snapshot) -> DashFlag {
    let _ = session_remain_ms(s);
    if s.on_track == 0 {
        CHECKERED_LATCH.store(0, Ordering::Relaxed);
        return DashFlag::None;
    }
    if CHECKERED_LATCH.load(Ordering::Relaxed) == 1 {
        return DashFlag::Checkered;
    }
    if prestart(s) || timed_clock_live(s) {
        return DashFlag::None;
    }
    if moving(s) && approaching_line(s) {
        match laps_left(s) {
            Some(1) => {
                CHECKERED_LATCH.store(1, Ordering::Relaxed);
                return DashFlag::Checkered;
            }
            Some(2) => return DashFlag::White,
            _ => {}
        }
    }
    if i_finished(s) && moving(s) && just_after_sf(s) {
        CHECKERED_LATCH.store(1, Ordering::Relaxed);
        return DashFlag::Checkered;
    }
    DashFlag::None
}

fn session_banner(s: &Snapshot) -> (char, String) {
    if is_lap_race(s) {
        if let Some(remain) = session_remain_ms(s) {
            if remain > 0 && remain <= 180_000 {
                return ('\u{f2f2}', format_countdown(remain));
            }
        }
        OVERTIME_BASE_LAP.store(-1, Ordering::Relaxed);
        OVERTIME_LOCAL_BASE.store(-1, Ordering::Relaxed);
        return ('\u{f11e}', lap_race_text(s));
    }
    if let Some(remain) = session_remain_ms(s) {
        if remain > 1000 && SESSION_EXPIRED.load(Ordering::Relaxed) == 0 {
            OVERTIME_BASE_LAP.store(-1, Ordering::Relaxed);
            OVERTIME_LOCAL_BASE.store(-1, Ordering::Relaxed);
        }
        if overtime_active(s) {
            return ('\u{f11e}', overtime_lap_text(s));
        }
        ('\u{f2f2}', format_countdown(remain))
    } else if s.session_laps > 0 {
        OVERTIME_BASE_LAP.store(-1, Ordering::Relaxed);
        OVERTIME_LOCAL_BASE.store(-1, Ordering::Relaxed);
        ('\u{f11e}', format!("{} / {}", race_lap(s), s.session_laps))
    } else {
        OVERTIME_BASE_LAP.store(-1, Ordering::Relaxed);
        OVERTIME_LOCAL_BASE.store(-1, Ordering::Relaxed);
        ('\u{f2f2}', format_session_clock(s.session_time_ms))
    }
}

/// Compact session-clock fields for a JSONL trace the overlay writes while you ride.
pub struct ClockSample {
    pub seq: u32,
    pub session_length: i32,
    pub session_laps: i32,
    pub session_time_ms: i32,
    pub current_lap: i32,
    pub current_lap_ms: i32,
    pub last_lap_ms: i32,
    pub local_speed: f32,
    pub local_track_pos: f32,
    pub on_track: i32,
    pub local_laps: i32,
    pub leader_laps: i32,
    pub standing_count: i32,
    pub dash: String,
    pub remain_ms: i32,
    pub mode: i32,
    pub gate: i32,
    pub armed: i32,
    pub expired: i32,
    pub saw: i32,
    pub locked_len: i32,
    pub ot_local: i32,
    pub ot_lead: i32,
}

pub fn clock_sample(s: &Snapshot) -> ClockSample {
    let remain = session_remain_ms(s);
    let dash = session_banner(s).1;
    ClockSample {
        seq: s.seq,
        session_length: s.session_length,
        session_laps: s.session_laps,
        session_time_ms: s.session_time_ms,
        current_lap: s.current_lap,
        current_lap_ms: s.current_lap_ms,
        last_lap_ms: s.last_lap_ms,
        local_speed: s.local_speed,
        local_track_pos: s.local_track_pos,
        on_track: s.on_track,
        local_laps: focus_num_laps(s),
        leader_laps: leader_num_laps(s),
        standing_count: s.standing_count,
        dash,
        remain_ms: remain.unwrap_or(-1),
        mode: SESSION_CLOCK_MODE.load(Ordering::Relaxed),
        gate: IN_GATE.load(Ordering::Relaxed),
        armed: RACE_ARMED.load(Ordering::Relaxed),
        expired: SESSION_EXPIRED.load(Ordering::Relaxed),
        saw: SAW_SESSION_TIME.load(Ordering::Relaxed),
        locked_len: LOCKED_SESSION_LEN.load(Ordering::Relaxed),
        ot_local: OVERTIME_LOCAL_BASE.load(Ordering::Relaxed),
        ot_lead: OVERTIME_BASE_LAP.load(Ordering::Relaxed),
    }
}

fn session_best_ms(s: &Snapshot) -> i32 {
    s.standings
        .iter()
        .take(s.standing_count.max(0) as usize)
        .map(|row| row.best_lap_ms)
        .filter(|ms| *ms > 0)
        .min()
        .unwrap_or(0)
}

fn class_position(s: &Snapshot) -> i32 {
    let Some(st) = focus_standing(s) else {
        return 0;
    };
    let cat = cstr(&st.category);
    if cat.is_empty() {
        return st.position.max(0);
    }
    s.standings
        .iter()
        .take(s.standing_count.max(0) as usize)
        .filter(|row| cstr(&row.category) == cat)
        .position(|row| row.race_num == st.race_num)
        .map(|i| i as i32 + 1)
        .unwrap_or(st.position.max(0))
}

fn local_clock() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        let mins = (anim_now() as i32 / 60).rem_euclid(24 * 60);
        return format!("{:02}:{:02}", mins / 60, mins % 60);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        #[repr(C)]
        struct SystemTime {
            year: u16,
            month: u16,
            day_of_week: u16,
            day: u16,
            hour: u16,
            minute: u16,
            second: u16,
            milliseconds: u16,
        }
        extern "system" {
            fn GetLocalTime(lp: *mut SystemTime);
        }
        let mut st = SystemTime {
            year: 0,
            month: 0,
            day_of_week: 0,
            day: 0,
            hour: 0,
            minute: 0,
            second: 0,
            milliseconds: 0,
        };
        unsafe {
            GetLocalTime(&mut st);
        }
        format!("{:02}:{:02}", st.hour, st.minute)
    }
}

fn board_item(s: &Snapshot, cfg: &HudConfig, field: BoardField) -> Option<(char, String)> {
    if field == BoardField::None {
        return None;
    }
    let st = focus_standing(s);
    let text = match field {
        BoardField::None => return None,
        BoardField::Position => st
            .map(|r| format!("P{}", r.position.max(0)))
            .unwrap_or_else(|| "P--".into()),
        BoardField::ClassPos => {
            let pos = class_position(s);
            if pos > 0 {
                format!("P{pos}")
            } else {
                "P--".into()
            }
        }
        BoardField::Session => session_banner(s).1,
        BoardField::RaceTime => format_session_clock(s.session_time_ms),
        BoardField::Lap => {
            let lap = race_lap(s);
            if s.session_laps > 0 {
                format!("{lap} / {}", s.session_laps)
            } else if lap > 0 {
                format!("{lap}")
            } else {
                "--".into()
            }
        }
        BoardField::LapsLeft => {
            if s.session_laps > 0 {
                format!("{}", (s.session_laps - race_lap(s)).max(0))
            } else {
                "--".into()
            }
        }
        BoardField::Track => {
            let t = cstr(&s.track_name);
            if t.is_empty() {
                "TRACK".into()
            } else {
                t
            }
        }
        BoardField::Air => cfg.units.format_temp(s.air_temp),
        BoardField::Best => format_clock(dash_best_ms(s)),
        BoardField::SessionBest => format_clock(session_best_ms(s)),
        BoardField::LocalTime => local_clock(),
        BoardField::Riders => format!("{}", s.standing_count.max(s.rider_count).max(0)),
        BoardField::SessionType => {
            if s.session_laps > 0 {
                "Lap race".into()
            } else if s.session_length > 0 {
                "Timed".into()
            } else {
                "Session".into()
            }
        }
    };
    Some((field.icon(), text))
}

fn draw_board_bar(
    px: &mut Pixmap,
    fonts: &Fonts,
    s: &Snapshot,
    cfg: &HudConfig,
    fields: &[BoardField; 3],
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) {
    let pad = 8.0;
    let slot_w = ((w - pad * 2.0) / 3.0).max(1.0);
    let icon_s = (h * 0.48).clamp(9.0, 11.0);
    let fsz = (h * 0.46).clamp(10.0, 12.0);
    let ty = y + (h - fsz) * 0.42;
    for (i, field) in fields.iter().enumerate() {
        let Some((ch, label)) = board_item(s, cfg, *field) else {
            continue;
        };
        let max_tw = (slot_w - 16.0).max(12.0);
        let label = ellipsize(fonts, &label, fsz, max_tw);
        let iw = if ch != '\0' {
            fonts.icons.metrics(ch, icon_s).advance_width + 4.0
        } else {
            0.0
        };
        let used = iw + measure(fonts, &label, fsz);
        let sx = x + pad + slot_w * i as f32 + (slot_w - used) * 0.5;
        if ch != '\0' {
            icon(px, fonts, ch, icon_s, sx, ty + 0.5, text_col(), false);
        }
        text(px, fonts, &label, fsz, sx + iw, ty, text_col(), false);
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

    let k = style_k();
    let head_h = 26.0 * k;
    let col_h = 16.0 * k;
    let track_h = 20.0 * k;
    let row_h = 22.0 * k;
    let foot_h = if BoardField::any(&cfg.st_foot) { 20.0 * k } else { 0.0 };
    let vis = slice.len().max(1);
    let h = (head_h + col_h + track_h + vis as f32 * row_h + foot_h + 8.0).min(r.h * sh);
    let a = bg_a(cfg.st_bg);
    if a > 0 {
        fill_round(px, x, y, w, h, 6.0, Color::from_rgba8(8, 8, 10, a));
        fill_round(px, x, y, w, head_h, 6.0, Color::from_rgba8(4, 4, 6, ((a as u16 * 240) / 200).min(255) as u8));
        if let Some(rrt) = rr(x, y + head_h - 6.0, w, 6.0) {
            fill_rect(px, rrt, Color::from_rgba8(4, 4, 6, ((a as u16 * 240) / 200).min(255) as u8));
        }
    }

    draw_board_bar(px, fonts, s, cfg, &cfg.st_head, x, y, w, head_h);

    if rows == 0 {
        text(px, fonts, "Waiting for race data", 12.0, x + 12.0, y + head_h + 10.0, text_dim(), false);
        if foot_h > 0.0 {
            draw_board_bar(px, fonts, s, cfg, &cfg.st_foot, x, y + h - foot_h, w, foot_h);
        }
        return;
    }

    let cols = standings_cols(cfg);
    let pad = 8.0;
    let slots = col_slots(x, pad, w, &cols, |c| c.width(cfg) as f32, |c| matches!(c, StField::Name));
    let hdr_c = Color::from_rgba8(160, 160, 168, 220);
    let bar_end = slots
        .iter()
        .find(|(c, _, _)| *c == StField::Pos)
        .map(|(_, cx, cw)| bike_bar_end(*cx, *cw));

    let purple = Color::from_rgba8(196, 112, 255, 255);
    let best_ms = slice
        .iter()
        .chain(s.standings.iter().take(n))
        .map(|row| row.best_lap_ms)
        .filter(|ms| *ms > 0)
        .min()
        .unwrap_or(0);
    let you_bg = you_row_bg();
    let stripe_c = Color::from_rgba8(0, 0, 0, ((a as u16 * 70) / 255) as u8);
    let out_c = Color::from_rgba8(110, 110, 116, 255);

    let mut cy = y + head_h;
    let track = {
        let t = cstr(&s.track_name);
        if t.is_empty() { "TRACK".into() } else { t.to_uppercase() }
    };
    let track_col = accent();
    draw_count_track(px, fonts, x, cy, n, &track);
    if let Some(line) = rr(x + 8.0, cy + 18.0, w - 16.0, 1.2) {
        fill_rect(px, line, track_col);
    }
    cy += track_h;

    let hdr_y = cy + 2.0;
    for (col, cx, cw) in &slots {
        let right = !matches!(col, StField::Name | StField::Bike);
        let pad = if matches!(col, StField::Name) { name_left_pad(*cx, bar_end) } else { 0.0 };
        col_text(px, fonts, st_col_header(*col), 10.0, *cx + pad, (*cw - pad).max(8.0), hdr_y, hdr_c, right);
    }
    cy += col_h;

    let body_y = cy;
    let ids = row_ids(slice.iter().map(|row| row.race_num));
    let row_ys = ST_SLIDE.with(|a| a.borrow_mut().step(&ids, body_y, row_h, anim_now()));
    for vis_i in 0..slice.len() {
        if vis_i % 2 == 1 && a > 0 {
            fill_focus_row(px, x, body_y + vis_i as f32 * row_h, w, row_h, stripe_c);
        }
    }
    for (vis_i, row) in slice.iter().enumerate() {
        let cy = row_ys[vis_i];
        let cat = cstr(&row.category);
        let accent_c = bike_color(&cstr(&row.bike), &cat);

        let is_focus = row.race_num == focus;
        let out = standing_status(row).is_some() && standing_status(row) != Some("PIT");
        if is_focus {
            fill_focus_row(px, x, cy, w, row_h, you_bg);
        }

        let name_c = if out { out_c } else { text_col() };
        let dim = if out { out_c } else { Color::from_rgba8(210, 210, 216, 255) };
        let status = standing_status(row);
        for (kind, cx, cw) in &slots {
            if *kind == StField::Pos {
                fill_skew(px, *cx + *cw + 1.0, cy + 4.0, BIKE_BAR_W, row_h - 8.0, BIKE_BAR_SKEW, accent_c);
            }
            let (val, color, right) = match kind {
                StField::Pos => (format!("{}", row.position.max(0)), name_c, true),
                StField::Num => (format!("{}", row.race_num), dim, true),
                StField::Name => (cstr(&row.name).to_string(), name_c, false),
                StField::Bike => (cstr(&row.bike).to_string(), name_c, false),
                StField::Gap => (format_board_gap(row.gap_ms, row.gap_laps, row.position <= 1), dim, true),
                StField::Interval => (interval_text(s, row), dim, true),
                StField::Laps => (format!("{}", row.num_laps.max(0)), dim, true),
                StField::Current => (format!("{}", rider_current_lap(s, row.race_num, row.num_laps)), dim, true),
                StField::Best => (format_lap(row.best_lap_ms), if best_ms > 0 && row.best_lap_ms == best_ms && !out { purple } else { dim }, true),
                StField::Last => {
                    let ms = if row.last_lap_ms > 0 {
                        row.last_lap_ms
                    } else if row.race_num == focus {
                        s.last_lap_ms
                    } else {
                        0
                    };
                    (format_lap(ms), dim, true)
                }
                StField::Status => (status.unwrap_or("").to_string(), dim, true),
                StField::Penalty => (format_penalty(row.penalty_ms), dim, true),
                StField::Crashed => {
                    if row.crashed != 0 || rider_crashed(s, row.race_num) {
                        ("CRASH".into(), behind_col(), true)
                    } else {
                        (String::new(), dim, true)
                    }
                }
            };
            let pad = if *kind == StField::Name { name_left_pad(*cx, bar_end) } else { 0.0 };
            if *kind == StField::Bike && !val.is_empty() {
                let badge = ellipsize(fonts, &val, 9.0, (*cw - 4.0).max(12.0));
                let bw = (measure(fonts, &badge, 9.0) + 10.0).min(*cw);
                fill_round(px, *cx, cy + 5.0, bw, 13.0, 3.0, accent_c);
                text(px, fonts, &badge, 9.0, *cx + 5.0, cy + 6.0, ink_on(accent_c), false);
            } else {
                col_text(px, fonts, &val, 12.0, *cx + pad, (*cw - pad).max(8.0), cy + 4.0, color, right);
            }
        }
    }
    if foot_h > 0.0 {
        if a > 0 {
            if let Some(rrt) = rr(x, y + h - foot_h, w, foot_h) {
                fill_rect(px, rrt, Color::from_rgba8(4, 4, 6, ((a as u16 * 240) / 200).min(255) as u8));
            }
        }
        draw_board_bar(px, fonts, s, cfg, &cfg.st_foot, x, y + h - foot_h, w, foot_h);
    }
}

fn format_signed_delta(ms: i32, laps: i32) -> String {
    if laps != 0 {
        return format!("{laps:+}L");
    }
    let sec = ms as f32 / 1000.0;
    if ms.abs() < 50 {
        return "0.000".into();
    }
    if sec.abs() >= 60.0 {
        let m = (sec.abs() / 60.0) as i32;
        let s = sec.abs() - m as f32 * 60.0;
        let sign = if ms < 0 { '-' } else { '+' };
        format!("{sign}{m}:{:04.1}", s)
    } else {
        format!("{sec:+.3}")
    }
}

fn ticker_delta(focus: &crate::shm::Standing, row: &crate::shm::Standing) -> String {
    format_signed_delta(row.gap_ms - focus.gap_ms, row.gap_laps - focus.gap_laps)
}

fn ticker_meta_label(field: BoardField) -> &'static str {
    match field {
        BoardField::Lap | BoardField::LapsLeft => "LAPS",
        BoardField::Air => "TEMP",
        BoardField::Best | BoardField::SessionBest => "BEST",
        BoardField::Session | BoardField::RaceTime => "TIME",
        BoardField::Position | BoardField::ClassPos => "POS",
        BoardField::Track => "TRACK",
        BoardField::Riders => "RIDERS",
        BoardField::LocalTime => "CLOCK",
        BoardField::SessionType => "SESSION",
        BoardField::None => "",
    }
}

fn ticker_title(s: &Snapshot) -> String {
    let track = cstr(&s.track_name);
    let kind = if s.session_laps > 0 {
        "LAP RACE"
    } else if s.session_length > 0 {
        "TIMED"
    } else {
        "SESSION"
    };
    if track.is_empty() {
        kind.into()
    } else {
        format!("{kind} - {}", track.to_uppercase())
    }
}

fn draw_ticker(px: &mut Pixmap, fonts: &Fonts, s: &Snapshot, cfg: &HudConfig, sw: f32, sh: f32) {
    let r = cfg.ticker;
    let x = r.x * sw;
    let y = r.y * sh;
    let w = (r.w * sw).max(280.0);
    let h = (r.h * sh).clamp(42.0, 64.0);
    let k = style_k();
    let a = bg_a(cfg.ticker_bg);
    let cut = (h * 0.16).clamp(8.0, 14.0);
    if a > 0 {
        fill_skew(px, x, y, w, h, cut, Color::from_rgba8(16, 16, 20, a));
        let step = 7.0;
        let dot = Color::from_rgba8(180, 180, 188, ((a as u16 * 38) / 255).max(10) as u8);
        let mut dy = y + 5.0;
        while dy < y + h - 4.0 {
            let mut dx = x + 10.0 + ((dy - y) as i32 % 14) as f32;
            while dx < x + w - 10.0 {
                if let Some(rrt) = rr(dx, dy, 1.3, 1.3) {
                    fill_rect(px, rrt, dot);
                }
                dx += step;
            }
            dy += step;
        }
        fill_skew(px, x + 1.0, y, w - 2.0, 2.2, cut * 0.2, accent());
    }

    let pad_l = (h * 0.10).clamp(6.0, 12.0);
    let pad_r = (h * 0.05).clamp(3.0, 6.0);
    let show_title = cfg.ticker_title;
    let title_h = if show_title {
        (h * 0.22).clamp(11.0, 15.0)
    } else {
        0.0
    };
    let inner_x = x + pad_l + cut * 0.35;
    let inner_w = (w - pad_l - pad_r).max(120.0);
    let card_y = y + if show_title { title_h + 1.0 } else { 4.0 };
    let card_h = (y + h - 4.0 - card_y).max(22.0);
    let left_on = cfg.ticker_left != BoardField::None;
    let right_on = cfg.ticker_right != BoardField::None;
    let left_w = if left_on {
        ticker_meta_width(fonts, s, cfg, cfg.ticker_left, card_h, k)
    } else {
        0.0
    };
    let right_w = if right_on {
        ticker_meta_width(fonts, s, cfg, cfg.ticker_right, card_h, k)
    } else {
        0.0
    };
    let side_gap = (h * 0.12).clamp(8.0, 12.0);
    let cards_x = inner_x + left_w + if left_on { side_gap } else { 0.0 };
    let cards_w = (inner_x + inner_w - right_w - if right_on { side_gap } else { 0.0 } - cards_x).max(80.0);

    if left_on {
        draw_ticker_meta(
            px,
            fonts,
            s,
            cfg,
            cfg.ticker_left,
            inner_x,
            card_y,
            left_w,
            card_h,
            k,
            false,
        );
    }
    if right_on {
        draw_ticker_meta(
            px,
            fonts,
            s,
            cfg,
            cfg.ticker_right,
            inner_x + inner_w - right_w,
            card_y,
            right_w,
            card_h,
            k,
            true,
        );
    }

    if show_title {
        let title = ticker_title(s);
        let title_sz = (h * 0.16).clamp(8.5, 12.0);
        let title_max = (cards_w - 8.0).max(40.0);
        let title = ellipsize(fonts, &title, title_sz, title_max);
        text(
            px,
            fonts,
            &title,
            title_sz,
            cards_x + cards_w * 0.5,
            y + 4.0,
            Color::from_rgba8(236, 236, 240, 255),
            true,
        );
    }

    let n = (s.standing_count.max(0) as usize).min(MAX_SAFE);
    if n == 0 {
        text(
            px,
            fonts,
            "Waiting for race data",
            11.0,
            cards_x,
            card_y + card_h * 0.35,
            text_dim(),
            false,
        );
        return;
    }
    let want = cfg.ticker_count.clamp(3, 15) as usize;
    let (vis, card_w) = hstand_layout(cards_w, k, want, n);
    let focus = s.focus_race_num;
    let fi = (0..n).find(|&i| s.standings[i].race_num == focus).unwrap_or(0);
    let Some(focus_row) = s.standings.get(fi) else {
        return;
    };
    let best_ms = session_best_ms(s);
    let gap = HS_CARD_GAP;
    let stride = card_w + gap;
    let scroll = if cfg.ticker_autoscroll && n > vis {
        (anim_now() * HS_AUTO_SPEED).rem_euclid(n as f32)
    } else {
        let target = hstand_scroll_start(fi, vis, n);
        HS_SCROLL.with(|a| a.borrow_mut().step(target, anim_now()))
    };
    let lw = cards_w.ceil().max(1.0) as u32;
    let lh = card_h.ceil().max(1.0) as u32;
    if let Some(mut layer) = Pixmap::new(lw, lh) {
        for i in 0..n {
            let x = if cfg.ticker_autoscroll && n > vis {
                match hstand_loop_x(i as f32, scroll, n as f32, stride, cards_w, card_w) {
                    Some(x) => x,
                    None => continue,
                }
            } else {
                let x = hstand_card_x(i as f32, scroll, 0.0, stride);
                if x + card_w < -2.0 || x > cards_w + 2.0 {
                    continue;
                }
                x
            };
            draw_ticker_card(
                &mut layer,
                fonts,
                s,
                &s.standings[i],
                focus_row,
                best_ms,
                x,
                0.0,
                card_w,
                card_h,
                k,
            );
        }
        px.draw_pixmap(
            0,
            0,
            layer.as_ref(),
            &PixmapPaint::default(),
            Transform::from_translate(cards_x, card_y),
            None,
        );
    }
}

fn hstand_card_x(index: f32, scroll: f32, origin: f32, stride: f32) -> f32 {
    origin + (index - scroll) * stride
}

const HS_AUTO_SPEED: f32 = 0.42;
const HS_CARD_GAP: f32 = 3.0;

fn hstand_card_range(k: f32) -> (f32, f32) {
    let min_w = (86.0 * k).clamp(78.0, 96.0);
    let max_w = (124.0 * k).clamp(112.0, 140.0);
    (min_w, max_w.max(min_w + 8.0))
}

fn hstand_layout(cards_w: f32, k: f32, want: usize, n: usize) -> (usize, f32) {
    let gap = HS_CARD_GAP;
    let (min_w, max_w) = hstand_card_range(k);
    let n = n.max(1);
    let max_fit = if cards_w < min_w {
        1
    } else {
        (((cards_w + gap) / (min_w + gap)).floor() as usize).clamp(1, n)
    };
    let mut vis = want.clamp(1, max_fit);
    loop {
        let cw = (cards_w - gap * vis.saturating_sub(1) as f32) / vis as f32;
        if cw <= max_w || vis >= max_fit {
            return (vis, cw.clamp(min_w, max_w));
        }
        vis += 1;
    }
}

fn hstand_loop_x(
    index: f32,
    scroll: f32,
    n: f32,
    stride: f32,
    cards_w: f32,
    card_w: f32,
) -> Option<f32> {
    if n <= 0.0 || stride <= 0.0 {
        return None;
    }
    let period = n * stride;
    let mut x = (index - scroll) * stride;
    x = x.rem_euclid(period);
    if x > cards_w {
        x -= period;
    }
    if x + card_w < -2.0 || x > cards_w + 2.0 {
        None
    } else {
        Some(x)
    }
}

fn hstand_scroll_start(focus_idx: usize, vis: usize, n: usize) -> f32 {
    let vis = vis.max(1);
    if n <= vis {
        return 0.0;
    }
    if focus_idx + 1 <= vis {
        0.0
    } else {
        (focus_idx + 1 - vis).min(n - vis) as f32
    }
}

fn ticker_meta_copy(
    _fonts: &Fonts,
    s: &Snapshot,
    cfg: &HudConfig,
    field: BoardField,
    h: f32,
    k: f32,
) -> Option<(String, String, f32, f32)> {
    let (_, val) = board_item(s, cfg, field)?;
    let label = ticker_meta_label(field).to_string();
    let label_sz = (8.5 * k).clamp(7.5, 10.0);
    let val_sz = (h * 0.28).clamp(13.0, 20.0);
    Some((label, val, label_sz, val_sz))
}

fn ticker_meta_width(
    fonts: &Fonts,
    s: &Snapshot,
    cfg: &HudConfig,
    field: BoardField,
    h: f32,
    k: f32,
) -> f32 {
    let Some((label, val, label_sz, val_sz)) = ticker_meta_copy(fonts, s, cfg, field, h, k) else {
        return 36.0;
    };
    measure(fonts, &label, label_sz)
        .max(measure_bold(fonts, &val, val_sz))
        + 2.0
}

fn draw_ticker_meta(
    px: &mut Pixmap,
    fonts: &Fonts,
    s: &Snapshot,
    cfg: &HudConfig,
    field: BoardField,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    k: f32,
    align_right: bool,
) {
    let Some((label, val, label_sz, val_sz)) = ticker_meta_copy(fonts, s, cfg, field, h, k) else {
        return;
    };
    let lx = if align_right {
        x + w - measure(fonts, &label, label_sz)
    } else {
        x
    };
    let vx = if align_right {
        x + w - measure_bold(fonts, &val, val_sz)
    } else {
        x
    };
    text(
        px,
        fonts,
        &label,
        label_sz,
        lx,
        y + h * 0.22,
        Color::from_rgba8(150, 150, 156, 255),
        false,
    );
    text_bold(
        px,
        fonts,
        &val,
        val_sz,
        vx,
        y + h * 0.42,
        Color::from_rgba8(244, 244, 247, 255),
        false,
    );
}

fn draw_ticker_card(
    px: &mut Pixmap,
    fonts: &Fonts,
    s: &Snapshot,
    row: &crate::shm::Standing,
    focus: &crate::shm::Standing,
    best_ms: i32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    k: f32,
) {
    let is_focus = row.race_num == focus.race_num;
    let out = standing_status(row).is_some() && standing_status(row) != Some("PIT");
    if is_focus {
        fill_round(px, x, y, w, h, 3.0, you_row_bg());
    }
    let pos_s = (h * 0.38).clamp(14.0, 20.0);
    let pos_y = y + (h - pos_s) * 0.5;
    if let Some(rrt) = rr(x + 4.0, pos_y, pos_s, pos_s) {
        fill_rect(px, rrt, Color::from_rgba8(244, 244, 247, 255));
    }
    let pos = format!("{}", row.position.max(0));
    text_bold(
        px,
        fonts,
        &pos,
        (pos_s * 0.62).clamp(9.0, 13.0),
        x + 4.0 + pos_s * 0.5,
        pos_y + pos_s * 0.18,
        Color::from_rgba8(12, 12, 14, 255),
        true,
    );
    let accent_c = bike_color(&cstr(&row.bike), &cstr(&row.category));
    let bar_x = x + 4.0 + pos_s + 3.0;
    if let Some(rrt) = rr(bar_x, y + 4.0, 2.2, h - 8.0) {
        fill_rect(px, rrt, accent_c);
    }
    let text_x = bar_x + 7.0;
    let name_sz = (h * 0.28).clamp(10.5, 13.5);
    let gap_sz = (h * 0.22).clamp(8.5, 11.0);
    let name = ellipsize(fonts, &cstr(&row.name), name_sz, (w - (text_x - x) - 8.0).max(24.0));
    let name_c = if out {
        Color::from_rgba8(110, 110, 116, 255)
    } else {
        Color::from_rgba8(244, 244, 247, 255)
    };
    text_bold(px, fonts, &name, name_sz, text_x, y + h * 0.16, name_c, false);
    let gap_c = if out {
        Color::from_rgba8(110, 110, 116, 255)
    } else if is_focus {
        Color::from_rgba8(200, 200, 206, 255)
    } else {
        Color::from_rgba8(168, 168, 176, 255)
    };
    let gap = if is_focus {
        let ms = if row.last_lap_ms > 0 {
            row.last_lap_ms
        } else if s.last_lap_ms > 0 {
            s.last_lap_ms
        } else {
            row.best_lap_ms
        };
        format_lap(ms)
    } else if let Some(st) = standing_status(row) {
        st.to_string()
    } else {
        ticker_delta(focus, row)
    };
    text(px, fonts, &gap, gap_sz, text_x, y + h * 0.52, gap_c, false);
    if best_ms > 0 && row.best_lap_ms == best_ms && !out {
        let tag = "FASTEST LAP";
        let tag_sz = (7.5 * k).clamp(6.5, 8.5);
        let tw = measure(fonts, tag, tag_sz);
        text(
            px,
            fonts,
            tag,
            tag_sz,
            x + w - tw - 6.0,
            y + h - tag_sz - 5.0,
            Color::from_rgba8(232, 232, 236, 255),
            false,
        );
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DashFlag {
    None,
    White,
    Checkered,
}

struct FlagAnim {
    kind: DashFlag,
    t0: f32,
    hiding: bool,
}

static FLAG_ANIM: Mutex<FlagAnim> = Mutex::new(FlagAnim {
    kind: DashFlag::None,
    t0: 0.0,
    hiding: false,
});

fn flag_anim_step(wanted: DashFlag) -> (DashFlag, f32) {
    const IN: f32 = 0.36;
    const OUT: f32 = 0.20;
    let now = anim_now();
    let mut st = FLAG_ANIM.lock().unwrap_or_else(|e| e.into_inner());
    if wanted != DashFlag::None {
        if st.kind != wanted || st.hiding {
            st.kind = wanted;
            st.t0 = now;
            st.hiding = false;
        }
        let t = ease_out_cubic(((now - st.t0) / IN).clamp(0.0, 1.0));
        (st.kind, t)
    } else if st.kind != DashFlag::None {
        if !st.hiding {
            st.hiding = true;
            st.t0 = now;
        }
        let t = 1.0 - ease_out_cubic(((now - st.t0) / OUT).clamp(0.0, 1.0));
        if t <= 0.02 {
            st.kind = DashFlag::None;
            st.hiding = false;
            (DashFlag::None, 0.0)
        } else {
            (st.kind, t)
        }
    } else {
        (DashFlag::None, 0.0)
    }
}

struct DashLay {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    flag: DashFlag,
    flag_h: f32,
    flag_grow: f32,
    pad: f32,
    foot_pad: f32,
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
    speed: String,
    speed_label: &'static str,
    ptxt: String,
    lap_txt: String,
    foot: Vec<(char, String)>,
}

fn dash_box(fonts: &Fonts, s: &Snapshot, cfg: &HudConfig, sw: f32, sh: f32) -> (f32, f32, f32, f32) {
    let lay = dash_layout(fonts, s, cfg, sw, sh);
    (lay.x, lay.y - lay.flag_h, lay.w, lay.h + lay.flag_h)
}

fn dash_layout(fonts: &Fonts, s: &Snapshot, cfg: &HudConfig, sw: f32, sh: f32) -> DashLay {
    let x0 = cfg.dash.x * sw;
    let y = cfg.dash.y * sh;
    let h = (cfg.dash.h * sh).max(80.0);
    let wanted = dash_race_flag(s);
    let (flag, grow) = flag_anim_step(wanted);
    let flag_full = if flag != DashFlag::None {
        (h * 0.22).clamp(20.0, 28.0)
    } else {
        0.0
    };
    let flag_h = flag_full * grow;
    let pad = (h * 0.12).clamp(14.0, 20.0);
    let foot_pad = (h * 0.035).clamp(3.0, 6.0);
    let footer_h = (h * 0.20).clamp(16.0, 22.0);
    let mid_gap = (h * 0.04).clamp(4.0, 8.0);
    let rev_h = (h * 0.12).clamp(11.0, 15.0);
    let rev_y = y + (pad * 0.28).max(4.0);
    let main_y = (rev_y + rev_h + 14.0).max(y + pad);
    let main_h = (y + h - foot_pad - footer_h - mid_gap - main_y).max(36.0);
    let label = (h * 0.095).clamp(8.5, 11.0);
    let gear_n = (main_h * 0.58).clamp(20.0, 36.0);
    let val = (main_h * 0.26).clamp(13.0, 18.0);
    let pos_n = (main_h * 0.44).clamp(18.0, 30.0);
    let lap_sz = (val * 0.88).max(10.5);
    let icon_s = (footer_h * 0.68).clamp(9.0, 12.0);
    let fsz = (footer_h * 0.52).clamp(8.5, 11.0);

    let gear = if s.local_gear <= 0 {
        "N".into()
    } else {
        format!("{}", s.local_gear)
    };
    let speed = cfg.units.format_speed(s.local_speed);
    let speed_label = cfg.units.speed_label();
    let rpm_n = s.local_rpm.max(0);
    let rpm = format!("{rpm_n}");
    let pos = s
        .standings
        .iter()
        .take(s.standing_count.max(0) as usize)
        .find(|st| st.race_num == s.focus_race_num)
        .map(|st| st.position)
        .filter(|p| *p > 0);
    let ptxt = pos.map(|p| format!("P{p}")).unwrap_or_else(|| "P--".into());
    let lap = race_lap(s);
    let lap_txt = if session_remain_ms(s).is_some() {
        session_banner(s).1
    } else if s.session_laps > 0 {
        format!("{lap} / {}", s.session_laps)
    } else if lap > 0 {
        format!("{lap}")
    } else {
        "--".into()
    };
    let foot: Vec<(char, String)> = [cfg.dash_left, cfg.dash_mid, cfg.dash_right]
        .into_iter()
        .filter_map(|field| dash_foot_item(s, cfg, field))
        .collect();

    let gear_w = (main_h * 0.82).clamp(44.0, 56.0);
    let mid_w = (measure(fonts, "RPM", label) + 8.0 + measure(fonts, &rpm, val).max(measure(fonts, "0000", val)))
        .max(measure(fonts, speed_label, label) + 8.0 + measure(fonts, &speed, val).max(measure(fonts, "000", val)));
    let right_w = measure(fonts, &ptxt, pos_n).max(measure(fonts, &lap_txt, lap_sz));
    let mut foot_w = 0.0;
    for (ch, t) in &foot {
        foot_w += fonts.icons.metrics(*ch, icon_s).advance_width + 5.0 + measure(fonts, t, fsz);
    }
    let min_gap = 12.0;
    let inner_min = (gear_w + mid_w + right_w + min_gap * 2.0).max(foot_w + min_gap * 2.0);
    let w = (cfg.dash.w * sw).max(pad * 2.0 + inner_min);
    let x = x0;
    let inner = (w - pad * 2.0).max(inner_min);
    let main_gap = ((inner - gear_w - mid_w - right_w) / 4.0).max(min_gap);
    let gear_x = x + pad + main_gap;
    let mid_x = gear_x + gear_w + main_gap;
    let right_x = mid_x + mid_w + main_gap;

    if let Ok(mut vis) = DASH_VIS.lock() {
        *vis = crate::shm::Rect {
            x: x / sw,
            y: (y - flag_h) / sh,
            w: w / sw,
            h: (h + flag_h) / sh,
        };
    }

    DashLay {
        x,
        y,
        w,
        h,
        flag,
        flag_h,
        flag_grow: grow,
        pad,
        foot_pad,
        footer_h,
        rev_x: x + pad,
        rev_y,
        rev_w: w - pad * 2.0,
        rev_h,
        gear_x,
        gear_w,
        main_y,
        main_h,
        mid_x,
        mid_w,
        right_x,
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
        speed,
        speed_label,
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

fn format_delta_ms(ms: i32) -> String {
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

fn focus_standing(s: &Snapshot) -> Option<&crate::shm::Standing> {
    let focus = if s.focus_race_num > 0 { s.focus_race_num } else { s.local_race_num };
    standing_of(s, focus)
}

fn dash_best_ms(s: &Snapshot) -> i32 {
    let standing_best = focus_standing(s).map(|st| st.best_lap_ms).unwrap_or(0);
    [s.best_lap_ms, standing_best]
        .into_iter()
        .filter(|ms| *ms > 0)
        .min()
        .unwrap_or(0)
}

fn dash_foot_item(s: &Snapshot, cfg: &HudConfig, field: DashField) -> Option<(char, String)> {
    if field == DashField::None {
        return None;
    }
    let st = focus_standing(s);
    let text = match field {
        DashField::None => return None,
        DashField::Speed => format!("{} {}", cfg.units.format_speed(s.local_speed), cfg.units.speed_label()),
        DashField::Rpm => format!("{}", s.local_rpm.max(0)),
        DashField::Gear => {
            if s.local_gear <= 0 {
                "N".into()
            } else {
                format!("{}", s.local_gear)
            }
        }
        DashField::Position => st
            .map(|r| format!("P{}", r.position.max(0)))
            .unwrap_or_else(|| "P--".into()),
        DashField::Number => {
            let n = if s.focus_race_num > 0 { s.focus_race_num } else { s.local_race_num };
            if n > 0 { format!("#{n}") } else { "--".into() }
        }
        DashField::LapCount => {
            let lap = race_lap(s);
            if s.session_laps > 0 {
                format!("{lap} / {}", s.session_laps)
            } else if lap > 0 {
                format!("{lap}")
            } else {
                "--".into()
            }
        }
        DashField::LapsLeft => {
            if s.session_laps > 0 {
                format!("{}", (s.session_laps - race_lap(s)).max(0))
            } else {
                "--".into()
            }
        }
        DashField::Last => {
            let ms = st.map(|r| r.last_lap_ms).filter(|ms| *ms > 0).unwrap_or(s.last_lap_ms);
            format_clock(ms)
        }
        DashField::Best => format_clock(dash_best_ms(s)),
        DashField::Current => format_clock(s.current_lap_ms),
        DashField::Delta => {
            let best = dash_best_ms(s);
            let src = if s.current_lap_ms > 0 { s.current_lap_ms } else { s.last_lap_ms };
            if best <= 0 || src <= 0 {
                "--".into()
            } else {
                format_delta_ms(src - best)
            }
        }
        DashField::Air => cfg.units.format_temp(s.air_temp),
        DashField::Engine => cfg.units.format_temp(s.engine_temp),
        DashField::Gap => st
            .map(|r| format_board_gap(r.gap_ms, r.gap_laps, r.position <= 1))
            .unwrap_or_else(|| "--".into()),
        DashField::Interval => st.map(|r| interval_text(s, r)).unwrap_or_else(|| "--".into()),
        DashField::Penalty => format_penalty(st.map(|r| r.penalty_ms).unwrap_or(0)),
        DashField::Session => session_banner(s).1,
        DashField::Bike => st
            .map(|r| cstr(&r.bike))
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "--".into()),
        DashField::Class => st
            .map(|r| cstr(&r.category))
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "--".into()),
    };
    Some((field.icon(), text))
}

fn dash_flag_path(x: f32, y: f32, w: f32, h: f32) -> Option<Path> {
    let r = (h * 0.35).clamp(5.0, 8.0);
    let mut pb = PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.quad_to(x + w, y, x + w, y + r);
    pb.line_to(x + w, y + h);
    pb.line_to(x, y + h);
    pb.line_to(x, y + r);
    pb.quad_to(x, y, x + r, y);
    pb.close();
    pb.finish()
}

fn draw_diag_stripes(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, stripe: Color) {
    let step = (h.min(w) * 0.55).clamp(7.0, 12.0);
    let thick = step * 0.45;
    let mut t = -h;
    while t < w + h {
        let mut pb = PathBuilder::new();
        pb.move_to(x + t, y + h);
        pb.line_to(x + t + thick, y + h);
        pb.line_to(x + t + thick + h, y);
        pb.line_to(x + t + h, y);
        pb.close();
        if let Some(path) = pb.finish() {
            fill_path(px, &path, stripe);
        }
        t += step;
    }
}

fn draw_checkered(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32) {
    let cell = (w.min(h) * 0.55).clamp(5.0, 9.0);
    let cols = ((w / cell).ceil() as i32).max(1);
    let rows = ((h / cell).ceil() as i32).max(1);
    let black = Color::from_rgba8(12, 12, 14, 255);
    for row in 0..rows {
        for col in 0..cols {
            if (row + col) % 2 != 0 {
                continue;
            }
            let cx = x + col as f32 * cell;
            let cy = y + row as f32 * cell;
            if let Some(r) = rr(cx, cy, cell.min(x + w - cx).max(0.5), cell.min(y + h - cy).max(0.5)) {
                fill_rect(px, r, black);
            }
        }
    }
}

fn draw_dash_wrap(px: &mut Pixmap, fonts: &Fonts, d: &DashLay) {
    if d.flag == DashFlag::None || d.flag_grow <= 0.02 {
        return;
    }
    let (label, fg, bg) = match d.flag {
        DashFlag::White => (
            "WHITE FLAG",
            Color::from_rgba8(16, 16, 18, 255),
            Color::from_rgba8(244, 244, 246, 255),
        ),
        DashFlag::Checkered => (
            "CHECKERED FLAG",
            Color::from_rgba8(248, 248, 250, 255),
            Color::from_rgba8(236, 236, 238, 255),
        ),
        DashFlag::None => return,
    };
    let grow = d.flag_grow;
    let top_h = d.flag_h.max(2.0);
    let top_y = d.y - top_h;
    let top_x = d.x;
    let top_w = d.w;

    let Some(top) = dash_flag_path(top_x, top_y, top_w, top_h) else {
        return;
    };
    fill_path(px, &top, bg);
    if d.flag == DashFlag::Checkered {
        draw_checkered(px, top_x, top_y, top_w, top_h);
    } else {
        let stripe_w = (top_w * 0.16).min(28.0);
        draw_diag_stripes(px, top_x, top_y, stripe_w, top_h, Color::from_rgba8(210, 210, 214, 255));
        draw_diag_stripes(
            px,
            top_x + top_w - stripe_w,
            top_y,
            stripe_w,
            top_h,
            Color::from_rgba8(210, 210, 214, 255),
        );
        if let Some(r) = rr(top_x + stripe_w, top_y, (top_w - stripe_w * 2.0).max(0.0), top_h) {
            fill_rect(px, r, bg);
        }
    }

    if grow > 0.42 {
        let sz = (top_h * 0.48).clamp(10.0, 13.0) * (0.75 + 0.25 * grow);
        let tw = measure(fonts, label, sz);
        if d.flag == DashFlag::Checkered {
            fill_round(
                px,
                top_x + (top_w - tw) * 0.5 - 8.0,
                top_y + 3.0,
                tw + 16.0,
                (top_h - 6.0).max(8.0),
                4.0,
                Color::from_rgba8(8, 8, 10, 210),
            );
        }
        text_bold(px, fonts, label, sz, top_x + top_w * 0.5, top_y + (top_h - sz) * 0.42, fg, true);
    }
}

fn draw_dash(px: &mut Pixmap, fonts: &Fonts, s: &Snapshot, cfg: &HudConfig, sw: f32, sh: f32) {
    let d = dash_layout(fonts, s, cfg, sw, sh);
    draw_dash_wrap(px, fonts, &d);
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
    text_bold(px, fonts, &d.gear, d.gear_n, d.gear_x + d.gear_w * 0.5, d.main_y + (d.main_h - d.gear_n) * 0.42, white, true);

    text(px, fonts, "RPM", d.label, d.mid_x, d.main_y + d.main_h * 0.12, dim, false);
    text(px, fonts, &d.rpm, d.val, d.mid_x + d.mid_w - measure(fonts, &d.rpm, d.val), d.main_y + d.main_h * 0.10, white, false);
    if let Some(line) = rr(d.mid_x, d.main_y + d.main_h * 0.48, d.mid_w, 1.0) {
        fill_rect(px, line, Color::from_rgba8(200, 200, 206, 70));
    }
    text(px, fonts, d.speed_label, d.label, d.mid_x, d.main_y + d.main_h * 0.62, dim, false);
    text(px, fonts, &d.speed, d.val, d.mid_x + d.mid_w - measure(fonts, &d.speed, d.val), d.main_y + d.main_h * 0.58, white, false);

    text_bold(px, fonts, &d.ptxt, d.pos_n, d.right_x + 1.0, d.main_y + d.main_h * 0.10 + 1.0, Color::from_rgba8(20, 12, 6, 160), false);
    text_bold(px, fonts, &d.ptxt, d.pos_n, d.right_x, d.main_y + d.main_h * 0.10, dash_pos_col(), false);
    text(px, fonts, &d.lap_txt, d.lap_sz, d.right_x, d.main_y + d.main_h * 0.68, white, false);

    if !d.foot.is_empty() {
        let fy = d.y + d.h - d.foot_pad - d.footer_h + 1.0;
        let foot_inner = (d.w - d.pad * 2.0).max(1.0);
        let foot_used: f32 = d
            .foot
            .iter()
            .map(|(ch, label)| fonts.icons.metrics(*ch, d.icon_s).advance_width + 5.0 + measure(fonts, label, d.fsz))
            .sum();
        let foot_gap = ((foot_inner - foot_used) / (d.foot.len() as f32 + 1.0)).max(8.0);
        let mut fx = d.x + d.pad + foot_gap;
        for (ch, label) in &d.foot {
            icon(px, fonts, *ch, d.icon_s, fx, fy, white, false);
            fx += fonts.icons.metrics(*ch, d.icon_s).advance_width + 5.0;
            text(px, fonts, label, d.fsz, fx, fy + 1.0, white, false);
            fx += measure(fonts, label, d.fsz) + foot_gap;
        }
    }
}

fn draw_relative(px: &mut Pixmap, fonts: &Fonts, s: &Snapshot, cfg: &HudConfig, sw: f32, sh: f32) {
    let r = s.relative;
    let x = r.x * sw;
    let y = r.y * sh;
    let w = r.w * sw;
    let n = s.rider_count.max(0) as usize;
    let focus = if s.focus_race_num > 0 { s.focus_race_num } else { s.local_race_num };
    let mut focus_pos = s.local_track_pos;
    let mut have = s.has_telemetry != 0;
    let mut uniq: Vec<usize> = Vec::new();
    for i in 0..n {
        let rider = &s.riders[i];
        let empty = rider.race_num <= 0 && cstr(&rider.name).is_empty();
        if empty {
            continue;
        }
        if rider.race_num > 0 && uniq.iter().any(|&j| s.riders[j].race_num == rider.race_num) {
            continue;
        }
        if rider.race_num == focus {
            focus_pos = rider.track_pos;
            have = true;
        }
        uniq.push(i);
    }
    let side = s.relative_count.max(1) as usize;
    let vis = if have { uniq.len().min(side * 2 + 1).max(1) } else { 1 };
    let k = style_k();
    let head_h = 26.0 * k;
    let col_h = 16.0 * k;
    let track_h = 20.0 * k;
    let row_h = 22.0 * k;
    let foot_h = if BoardField::any(&cfg.rel_foot) { 20.0 * k } else { 0.0 };
    let h = (head_h + col_h + track_h + vis as f32 * row_h + foot_h + 8.0).min(r.h * sh);
    let a = bg_a(cfg.rel_bg);
    if a > 0 {
        fill_round(px, x, y, w, h, 6.0, Color::from_rgba8(8, 8, 10, a));
        fill_round(px, x, y, w, head_h, 6.0, Color::from_rgba8(4, 4, 6, ((a as u16 * 240) / 200).min(255) as u8));
        if let Some(rrt) = rr(x, y + head_h - 6.0, w, 6.0) {
            fill_rect(px, rrt, Color::from_rgba8(4, 4, 6, ((a as u16 * 240) / 200).min(255) as u8));
        }
    }

    draw_board_bar(px, fonts, s, cfg, &cfg.rel_head, x, y, w, head_h);

    if !have || uniq.is_empty() {
        text(px, fonts, "Waiting for positions", 12.0, x + 12.0, y + head_h + 10.0, text_dim(), false);
        if foot_h > 0.0 {
            draw_board_bar(px, fonts, s, cfg, &cfg.rel_foot, x, y + h - foot_h, w, foot_h);
        }
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

    let mut order: Vec<(usize, f32)> = uniq
        .iter()
        .map(|&i| (i, wrap(s.riders[i].track_pos, focus_pos)))
        .collect();
    order.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let self_idx = order
        .iter()
        .position(|(i, _)| s.riders[*i].race_num == focus)
        .unwrap_or(0);
    let behind = self_idx.min(side);
    let ahead = (order.len() - self_idx - 1).min(side);
    let mut show = Vec::new();
    for k in (1..=ahead).rev() {
        show.push(self_idx + k);
    }
    show.push(self_idx);
    for k in 1..=behind {
        show.push(self_idx - k);
    }

    let cols = cfg.relative_cols();
    let pad = 8.0;
    let slots = col_slots(x, pad, w, &cols, |c| c.width(cfg) as f32, |c| matches!(c, RelField::Name));
    let hdr_c = Color::from_rgba8(160, 160, 168, 220);
    let bar_end = slots
        .iter()
        .find(|(c, _, _)| *c == RelField::Pos)
        .map(|(_, cx, cw)| bike_bar_end(*cx, *cw));

    let purple = Color::from_rgba8(196, 112, 255, 255);
    let best_ms = s
        .standings
        .iter()
        .take(s.standing_count.max(0) as usize)
        .map(|row| row.best_lap_ms)
        .filter(|ms| *ms > 0)
        .min()
        .unwrap_or(0);
    let you_bg = you_row_bg();
    let stripe_c = Color::from_rgba8(0, 0, 0, ((a as u16 * 70) / 255) as u8);
    let out_c = Color::from_rgba8(110, 110, 116, 255);

    let mut cy = y + head_h;
    let track = {
        let t = cstr(&s.track_name);
        if t.is_empty() { "TRACK".into() } else { t.to_uppercase() }
    };
    let track_col = accent();
    draw_count_track(px, fonts, x, cy, n, &track);
    if let Some(line) = rr(x + 8.0, cy + 18.0, w - 16.0, 1.2) {
        fill_rect(px, line, track_col);
    }
    cy += track_h;

    let hdr_y = cy + 2.0;
    for (col, cx, cw) in &slots {
        let label = rel_col_header(*col);
        let right = !matches!(col, RelField::Name | RelField::Bike);
        let pad = if matches!(col, RelField::Name) { name_left_pad(*cx, bar_end) } else { 0.0 };
        col_text(px, fonts, label, 10.0, *cx + pad, (*cw - pad).max(8.0), hdr_y, hdr_c, right);
    }
    cy += col_h;

    let track_len = if s.track_length > 1.0 { s.track_length } else { 1.0 };
    let speed = s.local_speed.max(4.0);

    let body_y = cy;
    let ids = row_ids(show.iter().map(|oi| s.riders[order[*oi].0].race_num));
    let row_ys = REL_SLIDE.with(|a| a.borrow_mut().step(&ids, body_y, row_h, anim_now()));
    for (vis_i, _) in show.iter().enumerate() {
        if vis_i % 2 == 1 && a > 0 {
            fill_focus_row(px, x, body_y + vis_i as f32 * row_h, w, row_h, stripe_c);
        }
    }
    for (vis_i, oi) in show.iter().enumerate() {
        let cy = row_ys[vis_i];
        let (ri, wrapped) = order[*oi];
        let rider = &s.riders[ri];
        let is_self = rider.race_num == focus;
        let st = standing_of(s, rider.race_num);
        let cat = st.map(|r| cstr(&r.category)).unwrap_or_default();
        let bike_name = st.map(|r| cstr(&r.bike)).unwrap_or_default();
        let accent_c = bike_color(&bike_name, &cat);
        let out = rider.crashed != 0
            || st.is_some_and(|r| r.crashed != 0 || matches!(r.state, 1 | 3 | 4));
        if is_self {
            fill_focus_row(px, x, cy, w, row_h, you_bg);
        }

        let name_c = if out { out_c } else { text_col() };
        let dim = if out { out_c } else { Color::from_rgba8(210, 210, 216, 255) };
        let pos = st.map(|r| r.position).unwrap_or(0);
        let best = st.map(|r| r.best_lap_ms).unwrap_or(0);
        let last_ms = if st.map(|r| r.last_lap_ms).unwrap_or(0) > 0 {
            st.map(|r| r.last_lap_ms).unwrap_or(0)
        } else if is_self {
            s.last_lap_ms
        } else {
            0
        };
        for (kind, cx, cw) in &slots {
            if *kind == RelField::Pos {
                fill_skew(px, *cx + *cw + 1.0, cy + 4.0, BIKE_BAR_W, row_h - 8.0, BIKE_BAR_SKEW, accent_c);
            }
            let (val, color, right) = match kind {
                RelField::Pos => (
                    if pos > 0 { format!("{pos}") } else { String::new() },
                    name_c,
                    true,
                ),
                RelField::Num => (format!("{}", rider.race_num), dim, true),
                RelField::Name => (cstr(&rider.name).to_string(), name_c, false),
                RelField::Bike => (bike_name.to_string(), name_c, false),
                RelField::Gap => (
                    if is_self {
                        "0.0".into()
                    } else {
                        format!("{:.1}", ((wrapped * track_len) / speed).abs())
                    },
                    dim,
                    true,
                ),
                RelField::Laps => (
                    st.map(|r| format!("{}", r.num_laps.max(0))).unwrap_or_default(),
                    dim,
                    true,
                ),
                RelField::Current => (
                    format!("{}", rider_current_lap(s, rider.race_num, st.map(|r| r.num_laps).unwrap_or(0))),
                    dim,
                    true,
                ),
                RelField::Penalty => (st.map(|r| format_penalty(r.penalty_ms)).unwrap_or_default(), dim, true),
                RelField::Interval => (st.map(|r| interval_text(s, r)).unwrap_or_default(), dim, true),
                RelField::Crashed => {
                    if rider.crashed != 0 || st.is_some_and(|r| r.crashed != 0) {
                        ("CRASH".into(), behind_col(), true)
                    } else {
                        (String::new(), dim, true)
                    }
                }
                RelField::Best => (
                    format_lap(best),
                    if best_ms > 0 && best == best_ms && !out { purple } else { dim },
                    true,
                ),
                RelField::Last => (format_lap(last_ms), dim, true),
            };
            let pad = if *kind == RelField::Name { name_left_pad(*cx, bar_end) } else { 0.0 };
            if *kind == RelField::Bike && !val.is_empty() {
                let badge = ellipsize(fonts, &val, 9.0, (*cw - 4.0).max(12.0));
                let bw = (measure(fonts, &badge, 9.0) + 10.0).min(*cw);
                fill_round(px, *cx, cy + 5.0, bw, 13.0, 3.0, accent_c);
                text(px, fonts, &badge, 9.0, *cx + 5.0, cy + 6.0, ink_on(accent_c), false);
            } else {
                col_text(px, fonts, &val, 12.0, *cx + pad, (*cw - pad).max(8.0), cy + 4.0, color, right);
            }
        }
    }
    if foot_h > 0.0 {
        if a > 0 {
            if let Some(rrt) = rr(x, y + h - foot_h, w, foot_h) {
                fill_rect(px, rrt, Color::from_rgba8(4, 4, 6, ((a as u16 * 240) / 200).min(255) as u8));
            }
        }
        draw_board_bar(px, fonts, s, cfg, &cfg.rel_foot, x, y + h - foot_h, w, foot_h);
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

    let track_px = (8.0 * scale).clamp(5.5, 26.0);
    let lw = w.ceil().max(1.0) as u32;
    let lh = h.ceil().max(1.0) as u32;
    let key = track_layer_key(s, n, lw, lh, cfg.map_sf, cfg.map_arrows);
    MAP_LAYER.with(|slot| {
        let mut slot = slot.borrow_mut();
        let stale = slot.as_ref().map(|(k, _)| *k != key).unwrap_or(true);
        if stale {
            if let Some(mut layer) = Pixmap::new(lw, lh) {
                let to_local = |wx: f32, wz: f32| -> (f32, f32) {
                    let (sx, sy) = to_px(wx, wz);
                    (sx - x, sy - y)
                };
                let mut pb = PathBuilder::new();
                let (sx, sy) = to_local(s.poly[0].x, s.poly[0].z);
                pb.move_to(sx, sy);
                for p in s.poly.iter().take(n).skip(1) {
                    let (px_, py_) = to_local(p.x, p.z);
                    pb.line_to(px_, py_);
                }
                pb.close();
                if let Some(path) = pb.finish() {
                    let mut fill = Paint::default();
                    fill.set_color(fill_col());
                    fill.anti_alias = true;
                    layer.fill_path(&path, &fill, FillRule::EvenOdd, Transform::identity(), None);
                    stroke_path(&mut layer, &path, Color::from_rgba8(18, 16, 16, 240), track_px + 3.0);
                    stroke_path(&mut layer, &path, track_col(), track_px);
                }
                if n >= 2 && s.sf_meters >= 0.0 && cfg.map_sf {
                    draw_sf(&mut layer, s, n, to_local, track_px);
                }
                if cfg.map_arrows {
                    draw_track_arrows(&mut layer, s, n, to_local, track_px, None, false);
                }
                *slot = Some((key, layer));
            }
        }
        if let Some((_, layer)) = slot.as_ref() {
            px.draw_pixmap(
                0,
                0,
                layer.as_ref(),
                &PixmapPaint::default(),
                Transform::from_translate(x, y),
                None,
            );
        }
    });

    let pred_x = s.local_x + s.local_vel_x * age;
    let pred_z = s.local_z + s.local_vel_z * age;
    let focus = s.focus_race_num;
    let leader = leader_num(s);
    let other_r = (if cfg.map_numbers { 6.8 } else { 4.6 }) * style_k();

    if cfg.map_others {
        for i in 0..s.rider_count.max(0) as usize {
            let rider = &s.riders[i];
            if s.has_telemetry != 0 && rider.race_num == focus {
                continue;
            }
            let (hx, hy) = to_px(rider.x, rider.z);
            let fill = other_col();
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
        let local_r = 8.5 * style_k();
        draw_rider_dot(
            px,
            fonts,
            hx,
            hy,
            local_r,
            you_col(),
            rider_dot_num(s, local_num, cfg.map_dot),
            cfg.map_numbers,
            true,
        );
        let (fwx, fwz) = local_forward(s);
        let (sdx, sdy) = screen_dir(&to_px, pred_x, pred_z, fwx, fwz);
        draw_dot_chevron(px, hx, hy, local_r, sdx, sdy, you_col(), true);
        if cfg.map_crown && leader > 0 && focus == leader {
            crown_over_dot(px, fonts, hx, hy, local_r);
        }
        draw_state_mark(px, fonts, hx, hy, local_r, rider_mark(s, focus, s.local_crashed != 0));
    }
}

fn mini_view_radius(zoom: i32) -> f32 {
    const NEAR_M: f32 = 22.0;
    const FAR_M: f32 = 85.0;
    let t = zoom.clamp(0, 100) as f32 / 100.0;
    FAR_M + t * (NEAR_M - FAR_M)
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
    let mut held = MINI_PX
        .with(|slot| slot.borrow_mut().take())
        .filter(|p| p.width() == dim && p.height() == dim)
        .or_else(|| Pixmap::new(dim, dim));
    let n = s.poly_count.max(0) as usize;
    if n < 2 {
        if let Some(mini) = held.as_ref() {
            blit_circle(px, mini, left, top);
        }
        MINI_PX.with(|slot| *slot.borrow_mut() = held);
        return;
    }
    let Some(mini) = held.as_mut() else {
        return;
    };
    mini.fill(Color::TRANSPARENT);
    if cfg.mini_bg > 0 {
        fill_circle(
            mini,
            sdim * 0.5,
            sdim * 0.5,
            sdim * 0.5 - 0.5,
            Color::from_rgba8(18, 18, 20, bg_a(cfg.mini_bg)),
        );
    }

    let pred_x = s.local_x + s.local_vel_x * age;
    let pred_z = s.local_z + s.local_vel_z * age;
    let (fx, fz, rx, rz, scale, origin_x, origin_z, north_up) = if s.has_telemetry != 0 {
        let (fx, fz) = track_forward(s, n, pred_x, pred_z).unwrap_or_else(|| {
            let (f, z, _, _) = radar_axes(s);
            (f, z)
        });
        let radius_m = mini_view_radius(cfg.mini_zoom);
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

    let track_px = if north_up {
        let radius_m = mini_view_radius(cfg.mini_zoom);
        (sdim * 0.11 * (40.0 / radius_m)).clamp(16.0, 48.0)
    } else {
        (16.0 * scale).clamp(12.0, 28.0)
    };
    let mut pb = PathBuilder::new();
    if north_up {
        append_visible_track(&mut pb, s, n, origin_x, origin_z, mini_view_radius(cfg.mini_zoom) * 2.25, &to_px);
    } else {
        let (sx, sy) = to_px(s.poly[0].x, s.poly[0].z);
        pb.move_to(sx, sy);
        for p in s.poly.iter().take(n).skip(1) {
            let (px_, py_) = to_px(p.x, p.z);
            pb.line_to(px_, py_);
        }
    }
    if let Some(path) = pb.finish() {
        stroke_path_fast(mini, &path, Color::from_rgba8(8, 8, 10, 220), track_px + 5.0);
        stroke_path_fast(mini, &path, Color::from_rgba8(248, 248, 252, 255), track_px);
    }

    if n >= 2 && s.sf_meters >= 0.0 && cfg.mini_sf {
        draw_sf(mini, s, n, to_px, track_px);
    }
    if cfg.mini_arrows {
        draw_track_arrows(mini, s, n, to_px, track_px, Some((mc, sdim)), north_up);
    }

    let focus = s.focus_race_num;
    let leader = leader_num(s);
    let other_r = (sdim * 0.028).clamp(7.0, 11.0) * style_k();
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
            let fill = other_col();
            numbered_dot(
                mini,
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
            draw_dot_chevron(mini, hx, hy, other_r, sdx, sdy, fill, false);
            draw_rider_overhead(mini, fonts, s, rider.race_num, hx, hy, other_r, focus, leader, cfg.mini_crown, cfg.mini_place);
            draw_state_mark(mini, fonts, hx, hy, other_r, rider_mark(s, rider.race_num, rider.crashed != 0));
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
            fill_circle(mini, tx, ty, local_r * (0.55 + i as f32 * 0.04), Color::from_rgba8(255, 148, 48, a));
        }
        numbered_dot(
            mini,
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
        draw_dot_chevron(mini, hx, hy, local_r, sdx, sdy, you_col(), true);
        let local_num = if focus > 0 { focus } else { s.local_race_num };
        if cfg.mini_crown && leader > 0 && local_num == leader {
            crown_over_dot(mini, fonts, hx, hy, local_r);
        }
        draw_state_mark(mini, fonts, hx, hy, local_r, rider_mark(s, local_num, s.local_crashed != 0));
    }

    blit_circle(px, mini, left, top);
    MINI_PX.with(|slot| *slot.borrow_mut() = held);
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

fn color_bytes(c: Color) -> (u8, u8, u8, u8) {
    (
        (c.red() * 255.0 + 0.5) as u8,
        (c.green() * 255.0 + 0.5) as u8,
        (c.blue() * 255.0 + 0.5) as u8,
        (c.alpha() * 255.0 + 0.5) as u8,
    )
}

fn color_alpha(c: Color, a: u8) -> Color {
    let (r, g, b, _) = color_bytes(c);
    Color::from_rgba8(r, g, b, a)
}

fn draw_dot_chevron(px: &mut Pixmap, x: f32, y: f32, r: f32, sdx: f32, sdy: f32, fill: Color, you: bool) {
    let ring = if you { 2.4 } else { 1.8 };
    let h = (r * 1.05).clamp(5.5, 11.0);
    let w = (r * 0.78).clamp(3.8, 8.0);
    let base = r + ring + 1.05;
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
    fill_path(px, &path, color_alpha(fill, 230));
    stroke_path(px, &path, color_alpha(fill, 160), if you { 1.8 } else { 1.4 });
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

fn fill_dot_glow(px: &mut Pixmap, x: f32, y: f32, r: f32, fill: Color, you: bool) {
    fill_circle(
        px,
        x,
        y,
        r + if you { 5.6 } else { 4.2 },
        Color::from_rgba8(0, 0, 0, if you { 48 } else { 32 }),
    );
    fill_circle(px, x, y, r + if you { 4.4 } else { 3.3 }, color_alpha(fill, if you { 52 } else { 34 }));
    fill_circle(px, x, y, r + if you { 2.7 } else { 2.1 }, color_alpha(fill, if you { 86 } else { 58 }));
    fill_circle(px, x, y, r, color_alpha(fill, 230));
}

fn dot_body(px: &mut Pixmap, x: f32, y: f32, r: f32, fill: Color, you: bool) {
    fill_dot_glow(px, x, y, r, fill, you);
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
    let k = style_k().max(0.01);
    let size = if num >= 100 {
        r * 0.82
    } else if num >= 10 {
        r * 0.98
    } else {
        r * 1.12
    } / k;
    let Some((min_x, min_y, max_x, max_y)) = ink_bounds(fonts, &label, size) else {
        return;
    };
    let extra_x = if FAKE_BOLD.with(|c| c.get()) { 0.7 } else { 0.0 };
    let tx = (x - (min_x + max_x + extra_x) * 0.5).round();
    let ty = (y - (min_y + max_y) * 0.5).round();
    let ink = if you {
        ink_on(fill)
    } else {
        Color::from_rgba8(255, 255, 255, 255)
    };
    let outline = Color::from_rgba8(8, 8, 10, 180);
    text(px, fonts, &label, size, tx - 0.6, ty, outline, false);
    text(px, fonts, &label, size, tx + 0.6, ty, outline, false);
    text(px, fonts, &label, size, tx, ty - 0.6, outline, false);
    text(px, fonts, &label, size, tx, ty + 0.6, outline, false);
    text(px, fonts, &label, size, tx, ty, ink, false);
}

fn ink_bounds(fonts: &Fonts, s: &str, size: f32) -> Option<(f32, f32, f32, f32)> {
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    let mut pen = 0.0;
    let mut any = false;
    let size = size * style_k();
    let font = style_font(fonts);
    for ch in s.chars() {
        let m = font.metrics(ch, size);
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
    let size = size * style_k();
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
    icon_over_dot_scaled(px, fonts, x, y, r, ch, col, 1.0);
}

fn icon_over_dot_scaled(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    r: f32,
    ch: char,
    col: Color,
    scale: f32,
) {
    let k = style_k().max(0.01);
    let vis = (r * 1.85 * scale).clamp(11.0 * k * scale, 20.0 * k * scale);
    let size = vis / k;
    let metrics = fonts.icons.metrics(ch, vis);
    let gap = (r * 0.22).max(2.5);
    let cy = y - r - gap - vis + metrics.ymin as f32;
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
    if !show_place || mine <= 0 || theirs <= 0 {
        return;
    }
    if theirs == mine - 1 {
        draw_place_mark(px, x, y, r, true);
    } else if theirs == mine + 1 {
        draw_place_mark(px, x, y, r, false);
    }
}

fn draw_place_mark(px: &mut Pixmap, x: f32, y: f32, r: f32, ahead: bool) {
    let col = if ahead { ahead_col() } else { behind_col() };
    let mut ring = PathBuilder::new();
    ring.push_circle(x, y, r + 3.4);
    if let Some(path) = ring.finish() {
        stroke_path(px, &path, col, 3.6);
        stroke_path(px, &path, Color::from_rgba8(8, 8, 10, 200), 1.2);
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
    let k = style_k().max(0.01);
    let size = (r * 1.05).clamp(8.0 * k, 13.0 * k) / k;
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
    let cr2 = cr * cr;
    let fade2 = fade_start * fade_start;
    let fade_span = (cr - fade_start).max(0.001);
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
            let dist2 = (fx - ccx) * (fx - ccx) + (fy - ccy) * (fy - ccy);
            if dist2 >= cr2 {
                continue;
            }
            let cover = if dist2 <= fade2 {
                1.0
            } else {
                let t = ((dist2.sqrt() - fade_start) / fade_span).clamp(0.0, 1.0);
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

const RADAR_FWD_AHEAD: f32 = 3.0;
const RADAR_FWD_REAR: f32 = 12.0;
const RADAR_LAT: f32 = 6.0;
const RADAR_SIDE_LAT: f32 = 0.4;
const RADAR_REAR_FWD: f32 = -0.6;
const RADAR_STRETCH_M: f32 = 20.0;

fn radar_in_view(fwd: f32, lat: f32, sides: bool, rear: bool) -> bool {
    if fwd < -RADAR_FWD_REAR || fwd > RADAR_FWD_AHEAD || lat.abs() > RADAR_LAT {
        return false;
    }
    let behind = fwd < RADAR_REAR_FWD;
    let beside = lat.abs() > RADAR_SIDE_LAT;
    (rear && behind) || (sides && beside)
}

fn radar_you_frac() -> f32 {
    RADAR_FWD_AHEAD / (RADAR_FWD_AHEAD + RADAR_FWD_REAR)
}

fn radar_to_screen(fwd: f32, lat: f32, ox: f32, oy: f32, sx: f32, sy: f32) -> (f32, f32) {
    (ox + lat * sx, oy - fwd * sy)
}

fn radar_blip_heat(dist: f32) -> f32 {
    ((8.0 - dist) / 7.0).clamp(0.0, 1.0)
}

fn radar_blip_radius(heat: f32, size: f32) -> f32 {
    (size * (0.035 + heat * 0.045)).clamp(3.2, 8.5)
}

fn radar_blip_color(heat: f32) -> Color {
    let r = (240.0 + 15.0 * heat) as u8;
    let g = (196.0 + (64.0 - 196.0) * heat) as u8;
    let b = (40.0 + 32.0 * heat) as u8;
    let a = (160.0 + heat * 95.0) as u8;
    Color::from_rgba8(r, g, b, a)
}

fn draw_radar_wedge(px: &mut Pixmap, ox: f32, oy: f32, sx: f32, sy: f32, left: bool, a: u8) {
    let sign = if left { -1.0 } else { 1.0 };
    let pts = [
        (0.7 * sign, -0.5),
        (RADAR_LAT * sign, -1.2),
        (RADAR_LAT * sign, -RADAR_FWD_REAR),
        (0.35 * sign, -RADAR_FWD_REAR),
    ];
    let mut pb = PathBuilder::new();
    let (x0, y0) = radar_to_screen(pts[0].1, pts[0].0, ox, oy, sx, sy);
    pb.move_to(x0, y0);
    for (lat, fwd) in pts.iter().skip(1) {
        let (x, y) = radar_to_screen(*fwd, *lat, ox, oy, sx, sy);
        pb.line_to(x, y);
    }
    pb.close();
    if let Some(path) = pb.finish() {
        let mut p = Paint::default();
        p.set_color(Color::from_rgba8(255, 168, 64, a));
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
    let size = w.min(h);
    let pad = (size * 0.10).max(8.0);

    let a = bg_a(cfg.radar_bg);
    if a > 0 {
        fill_round(px, x, y, w, h, 6.0, Color::from_rgba8(22, 22, 24, a));
    }

    let ox = x + w * 0.5;
    let usable_h = (h - pad * 2.0).max(16.0);
    let oy = y + pad + usable_h * radar_you_frac();
    let sx = ((w * 0.5 - pad) / RADAR_LAT).max(0.5);
    let sy = (usable_h / (RADAR_FWD_AHEAD + RADAR_FWD_REAR)).max(0.5);

    let wedge_a = if a > 0 {
        ((a as u16 * 48) / 255).max(18) as u8
    } else {
        28
    };
    draw_radar_wedge(px, ox, oy, sx, sy, true, wedge_a);
    draw_radar_wedge(px, ox, oy, sx, sy, false, wedge_a);

    let bw = (size * 0.09).max(7.0);
    let bh = (size * 0.20).max(14.0);
    fill_round(px, ox - bw * 0.5 - 1.5, oy - bh * 0.5 - 1.5, bw + 3.0, bh + 3.0, 3.0, Color::from_rgba8(8, 8, 10, 220));
    fill_round(px, ox - bw * 0.5, oy - bh * 0.5, bw, bh, 2.2, Color::from_rgba8(248, 248, 252, 255));
    let mut nose = PathBuilder::new();
    nose.move_to(ox, oy - bh * 0.5 - 4.0);
    nose.line_to(ox - 3.6, oy - bh * 0.5 + 1.0);
    nose.line_to(ox + 3.6, oy - bh * 0.5 + 1.0);
    nose.close();
    if let Some(path) = nose.finish() {
        let mut p = Paint::default();
        p.set_color(Color::from_rgba8(248, 248, 252, 255));
        p.anti_alias = true;
        px.fill_path(&path, &p, FillRule::Winding, Transform::identity(), None);
    }

    if s.has_telemetry == 0 {
        return;
    }

    let pred_x = s.local_x + s.local_vel_x * age;
    let pred_z = s.local_z + s.local_vel_z * age;
    let (fx, fz, rx, rz) = radar_axes(s);
    let focus = s.focus_race_num;
    let mut blips: Vec<(f32, f32, f32)> = Vec::new();
    for i in 0..s.rider_count.max(0) as usize {
        let rider = &s.riders[i];
        if rider.race_num == focus {
            continue;
        }
        let dx = rider.x - pred_x;
        let dz = rider.z - pred_z;
        let fwd = dx * fx + dz * fz;
        let lat = dx * rx + dz * rz;
        if !radar_same_stretch(s, rider.track_pos, RADAR_STRETCH_M) {
            continue;
        }
        if !radar_in_view(fwd, lat, cfg.radar_sides, cfg.radar_rear) {
            continue;
        }
        let dist = (fwd * fwd + lat * lat).sqrt();
        blips.push((fwd, lat, dist));
    }
    blips.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    for (fwd, lat, dist) in blips {
        let (bx, by) = radar_to_screen(fwd, lat, ox, oy, sx, sy);
        let heat = radar_blip_heat(dist);
        let rad = radar_blip_radius(heat, size);
        fill_circle(px, bx, by, rad + 1.4, Color::from_rgba8(8, 8, 10, 220));
        fill_circle(px, bx, by, rad, radar_blip_color(heat));
    }
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

fn radar_same_stretch(s: &Snapshot, other_pos: f32, max_m: f32) -> bool {
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
        let half = (track_px * 1.15).max(7.0);
        let mut pb = PathBuilder::new();
        pb.move_to(hx - pxn * half, hy - pyn * half);
        pb.line_to(hx + pxn * half, hy + pyn * half);
        if let Some(path) = pb.finish() {
            stroke_path(px, &path, Color::from_rgba8(8, 8, 10, 220), (track_px * 0.55).clamp(5.0, 9.0));
            stroke_path(px, &path, accent(), (track_px * 0.38).clamp(3.5, 6.5));
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

fn stroke_path_fast(px: &mut Pixmap, path: &Path, color: Color, width: f32) {
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = false;
    let stroke = Stroke {
        width,
        line_cap: LineCap::Butt,
        line_join: LineJoin::Miter,
        ..Stroke::default()
    };
    px.stroke_path(path, &paint, &stroke, Transform::identity(), None);
}

fn track_layer_key(s: &Snapshot, n: usize, w: u32, h: u32, sf: bool, arrows: bool) -> u64 {
    let mut hasher = DefaultHasher::new();
    n.hash(&mut hasher);
    w.hash(&mut hasher);
    h.hash(&mut hasher);
    sf.hash(&mut hasher);
    arrows.hash(&mut hasher);
    s.sf_meters.to_bits().hash(&mut hasher);
    let pts = [0, n / 2, n - 1];
    for i in pts {
        s.poly[i].x.to_bits().hash(&mut hasher);
        s.poly[i].z.to_bits().hash(&mut hasher);
    }
    hasher.finish()
}

fn append_visible_track(
    pb: &mut PathBuilder,
    s: &Snapshot,
    n: usize,
    origin_x: f32,
    origin_z: f32,
    radius_m: f32,
    to_px: &impl Fn(f32, f32) -> (f32, f32),
) {
    if n < 2 {
        return;
    }
    let r2 = radius_m * radius_m;
    let near_pt = |x: f32, z: f32| {
        let dx = x - origin_x;
        let dz = z - origin_z;
        dx * dx + dz * dz <= r2
    };
    let near_seg = |ax: f32, az: f32, bx: f32, bz: f32| {
        if near_pt(ax, az) || near_pt(bx, bz) {
            return true;
        }
        let sx = bx - ax;
        let sz = bz - az;
        let len2 = sx * sx + sz * sz;
        if len2 < 1e-6 {
            return false;
        }
        let t = ((origin_x - ax) * sx + (origin_z - az) * sz) / len2;
        let t = t.clamp(0.0, 1.0);
        near_pt(ax + sx * t, az + sz * t)
    };
    let looped = {
        let dx = s.poly[0].x - s.poly[n - 1].x;
        let dz = s.poly[0].z - s.poly[n - 1].z;
        dx * dx + dz * dz < 400.0
    };
    let seg_count = if looped { n } else { n - 1 };
    let mut drawing = false;
    for i in 0..seg_count {
        let a = &s.poly[i];
        let b = &s.poly[(i + 1) % n];
        if near_seg(a.x, a.z, b.x, b.z) {
            let (x0, y0) = to_px(a.x, a.z);
            let (x1, y1) = to_px(b.x, b.z);
            if !drawing {
                pb.move_to(x0, y0);
                drawing = true;
            }
            pb.line_to(x1, y1);
        } else {
            drawing = false;
        }
    }
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod render_tests;
