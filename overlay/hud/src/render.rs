use std::cell::{Cell, RefCell};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::config::{BoardField, DashField, DotLabel, FontFamily, HudConfig, RelField, StField, StanceStyle, TableText};
use crate::shm::{cstr, Snapshot};
pub use crate::race_store::{ClockSample, clock_sample};
// Re-export clock / field helpers for `render_tests` (`use super::*`).
#[allow(unused_imports)]
pub(crate) use crate::race_store::{
    class_position, extras_started, extra_laps, finish_earned, focus_num_laps,
    focus_standing, format_countdown, format_gap, format_lap, format_session_clock,
    i_finished, note_laps_to_run,
    interval_text, interval_text_from_row, is_lap_race, is_warmup, lapped, laps_done, laps_left,
    leader_finished, leader_num_laps, live_leader, live_position, local_overtime_done,
    local_overtime_taken, moving, norm_lap_pos as norm_track_pos,
    overtime_active, prestart, race_lap, race_laps_left_text, race_over_for_me,
    race_progress_text, reset_session_clock_track, rider_current_lap, session_banner,
    session_best_ms, session_len_ms, session_remain_ms, standing_of, ticker_delta_from_row,
    timed_clock_live, timed_race_flag, CHECKERED_LATCH, IN_GATE, LAP_GREEN, LAST_CUR_LAP,
    CLOSING_ON_LINE, LAP_MID_SEEN, LAST_SESSION_SIG, LAST_SF_METERS, LEADER_FIN_LOCAL_BASE,
    OVERTIME_LOCAL_BASE, POST_GATE, RaceFlag, RaceStore, SESSION_EXPIRED, SF_FRAC_CAND,
    SF_FRAC_LEARNED, SF_LEARN_LAPS, LAPS_TO_RUN_AT, RUN_IN_FLAG, WHITE_WAVE_AT,
    WHITE_WAVE_LAP,
};
use fontdue::Font;
use tiny_skia::{
    Color, FillRule, FilterQuality, GradientStop, LineCap, LineJoin, LinearGradient, Mask, Paint, Path,
    PathBuilder, Pixmap, PixmapPaint, Point as SkPoint, Rect, SpreadMode, Stroke, StrokeDash, Transform,
};

fn accent() -> Color { Color::from_rgba8(255, 148, 48, 255) }
fn text_col() -> Color { Color::from_rgba8(228, 228, 230, 255) }
fn text_dim() -> Color { Color::from_rgba8(132, 132, 138, 255) }
fn panel_col() -> Color { Color::from_rgba8(10, 10, 10, 200) }
fn track_col() -> Color { Color::from_rgba8(236, 236, 240, 255) }
fn fill_col() -> Color { Color::from_rgba8(10, 8, 8, 168) }
fn you_col() -> Color { Color::from_rgba8(255, 148, 48, 255) }
fn other_col() -> Color { Color::from_rgba8(48, 52, 64, 255) }
fn lapping_col() -> Color { Color::from_rgba8(59, 130, 246, 255) }
fn lapped_col() -> Color { Color::from_rgba8(239, 68, 68, 255) }
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
    static CLICK_RIDERS: RefCell<Vec<ClickRider>> = RefCell::new(Vec::new());
}

struct IndexSlide {
    from: f32,
    to: f32,
    start: f32,
    init: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct ClickRider {
    pub race_num: i32,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

fn push_click_rider(race_num: i32, x: f32, y: f32, w: f32, h: f32) {
    if race_num <= 0 || w <= 1.0 || h <= 1.0 {
        return;
    }
    CLICK_RIDERS.with(|v| {
        v.borrow_mut().push(ClickRider {
            race_num,
            x,
            y,
            w,
            h,
        });
    });
}

fn clear_click_riders() {
    CLICK_RIDERS.with(|v| v.borrow_mut().clear());
}

pub fn click_rider_hits() -> Vec<ClickRider> {
    CLICK_RIDERS.with(|v| v.borrow().clone())
}

pub fn click_rider_at(px: f32, py: f32) -> Option<i32> {
    CLICK_RIDERS.with(|v| {
        v.borrow()
            .iter()
            .rev()
            .find(|h| px >= h.x && px < h.x + h.w && py >= h.y && py < h.y + h.h)
            .map(|h| h.race_num)
    })
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
        Self::for_family(FontFamily::Exo2)
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
            FontFamily::Exo2 => Self::embedded(
                include_bytes!("../fonts/Exo2-ExtraBoldItalic.ttf"),
                Some(include_bytes!("../fonts/Exo2-BlackItalic.ttf")),
                icons,
            ),
            FontFamily::Teko => Self::embedded(
                include_bytes!("../fonts/Teko-SemiBold.ttf"),
                Some(include_bytes!("../fonts/Teko-Bold.ttf")),
                icons,
            ),
            FontFamily::Goldman => Self::embedded(
                include_bytes!("../fonts/Goldman-Regular.ttf"),
                Some(include_bytes!("../fonts/Goldman-Bold.ttf")),
                icons,
            ),
            FontFamily::Montserrat => Self::embedded(
                include_bytes!("../fonts/Montserrat-ExtraBold.ttf"),
                Some(include_bytes!("../fonts/Montserrat-Black.ttf")),
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

pub fn draw(px: &mut Pixmap, fonts: &Fonts, snap: Option<&Snapshot>, cfg: &HudConfig, w: u32, h: u32, age: f32, restart_hint: bool, plugin_hint: bool, settings_hint: bool) {
    clear_click_riders();
    px.fill(if settings_hint {
        Color::from_rgba8(0, 0, 0, 1)
    } else {
        Color::TRANSPARENT
    });
    if restart_hint {
        top_banner(px, fonts, w, "Restart MX Bikes once so the HUD stays on top while you ride");
    } else if plugin_hint && !settings_hint {
        top_banner(
            px,
            fonts,
            w,
            "Fully quit MX Bikes and start it again so the plugin can load",
        );
    }
    let Some(s) = snap else {
        return;
    };
    if !s.has_session_data() && !settings_hint {
        if !cfg.any_overlay_widget() {
            top_banner(
                px,
                fonts,
                w,
                "Press F8 — turn on Show on overlay. Widgets appear on track.",
            );
        }
        return;
    }
    if !cfg.any_overlay_widget() && !settings_hint {
        top_banner(px, fonts, w, "Press F8 — turn on Show on overlay");
    }

    let sw = w as f32;
    let sh = h as f32;
    let _race = RaceStore::tick(s);
    let delta = crate::delta::view();
    let (flag, flag_grow) = if cfg.show_dash || cfg.show_flag {
        tick_display_flag(s, cfg.show_flag && cfg.flag_yellow, cfg.show_flag && cfg.flag_blue)
    } else {
        (DashFlag::None, 0.0)
    };
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
        draw_radar(px, fonts, s, cfg, sw, sh, age);
    }
    if cfg.show_dash {
        let _g = push_style(fonts, cfg.dash_bold, cfg.dash_font);
        let (dash_flag, dash_grow) = dash_wrap_flag(flag, flag_grow);
        draw_dash(px, fonts, s, cfg, sw, sh, dash_flag, dash_grow);
    }
    if cfg.show_ticker {
        let _g = push_style(fonts, cfg.ticker_bold, cfg.ticker_font);
        draw_ticker(px, fonts, s, cfg, sw, sh);
    }
    if cfg.show_sys {
        let _g = push_style(fonts, cfg.sys_bold, cfg.sys_font);
        draw_sys(px, fonts, cfg, sw, sh);
    }
    if cfg.sector_visible() {
        let _g = push_style(fonts, cfg.sector_bold, cfg.sector_font);
        draw_sector(px, fonts, s, cfg, sw, sh);
    }
    if cfg.delta_visible() {
        let _g = push_style(fonts, cfg.delta_bold, cfg.delta_font);
        draw_delta(px, fonts, cfg, sw, sh, delta);
    }
    if cfg.show_flag {
        let _g = push_style(fonts, cfg.flag_bold, cfg.flag_font);
        draw_flag(px, fonts, cfg, sw, sh, flag, flag_grow);
    }
    if cfg.stance_visible() {
        let _g = push_style(fonts, cfg.stance_bold, cfg.stance_font);
        draw_stance(px, fonts, cfg, sw, sh);
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

fn top_banner(px: &mut Pixmap, fonts: &Fonts, w: u32, msg: &str) {
    let cx = w as f32 * 0.5;
    let bw = 680.0;
    if let Some(r) = rr(cx - bw * 0.5, 8.0, bw, 40.0) {
        fill_rect(px, r, panel_col());
    }
    text(px, fonts, msg, 13.0, cx, 18.0, accent(), true);
}

/// App mark in the top-right while the overlay is on MX Bikes. Click it to open settings.
/// Returns the hit box in overlay pixels.
pub fn draw_live_mark(px: &mut Pixmap, w: u32, _h: u32, hot: bool) -> (f32, f32, f32, f32) {
    let icon = app_icon();
    let size = 40.0;
    let pad = 10.0;
    let x = (w as f32 - pad - size).max(8.0);
    let y = pad;
    // icon-48 art uses a ~10px corner on 48px; keep that ratio so the orange border is not clipped.
    let r = size * 10.0 / 48.0;
    if hot {
        if let Some(glow) = round_rect_path(x - 3.0, y - 3.0, size + 6.0, size + 6.0, r + 3.0) {
            fill_path(px, &glow, Color::from_rgba8(255, 148, 48, 90));
        }
    }
    let scale = size / icon.width().max(1) as f32;
    let mut paint = PixmapPaint::default();
    paint.quality = FilterQuality::Bicubic;
    let clip = round_rect_path(x, y, size, size, r).and_then(|path| {
        let mut m = Mask::new(px.width(), px.height())?;
        m.fill_path(&path, FillRule::Winding, true, Transform::identity());
        Some(m)
    });
    px.draw_pixmap(
        0,
        0,
        icon.as_ref(),
        &paint,
        Transform::from_row(scale, 0.0, 0.0, scale, x, y),
        clip.as_ref(),
    );
    (x, y, size, size)
}

fn app_icon() -> &'static Pixmap {
    static ICON: OnceLock<Pixmap> = OnceLock::new();
    ICON.get_or_init(|| Pixmap::decode_png(include_bytes!("../../icon-48.png")).expect("icon-48.png"))
}

/// Circular rounded rect (cubic approximation). Quadratic corners look pointed at this size.
fn round_rect_path(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<Path> {
    let r = r.min(w * 0.5).min(h * 0.5);
    let k = 0.55228475 * r;
    let mut pb = PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.cubic_to(x + w - r + k, y, x + w, y + r - k, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.cubic_to(x + w, y + h - r + k, x + w - r + k, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.cubic_to(x + r - k, y + h, x, y + h - r + k, x, y + h - r);
    pb.line_to(x, y + r);
    pb.cubic_to(x, y + r - k, x + r - k, y, x + r, y);
    pb.close();
    pb.finish()
}

fn draw_layout(px: &mut Pixmap, s: &Snapshot, cfg: &HudConfig, sw: f32, sh: f32) {
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
    if cfg.show_sys {
        layout_box(px, cfg.sys.x * sw, cfg.sys.y * sh, cfg.sys.w * sw, cfg.sys.h * sh, false);
    }
    if cfg.sector_visible() {
        layout_box(px, cfg.sector.x * sw, cfg.sector.y * sh, cfg.sector.w * sw, cfg.sector.h * sh, false);
    }
    if cfg.delta_visible() {
        layout_box(px, cfg.delta.x * sw, cfg.delta.y * sh, cfg.delta.w * sw, cfg.delta.h * sh, false);
    }
    if cfg.stance_visible() {
        layout_box(px, cfg.stance.x * sw, cfg.stance.y * sh, cfg.stance.w * sw, cfg.stance.h * sh, false);
    }
    if cfg.show_flag {
        layout_box(px, cfg.flag.x * sw, cfg.flag.y * sh, cfg.flag.w * sw, cfg.flag.h * sh, false);
    }
    if cfg.show_dash {
        layout_box(
            px,
            cfg.dash.x * sw,
            cfg.dash.y * sh,
            cfg.dash.w * sw,
            cfg.dash.h * sh,
            false,
        );
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
    // tiny-skia debug-asserts on subpixel hairline AA rects; the integer path is also cheaper.
    p.anti_alias = r.width() >= 2.0 && r.height() >= 2.0;
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
        if ch != ' ' && ch != '\t' && !font.has_glyph(ch) {
            continue;
        }
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

pub fn measure(fonts: &Fonts, s: &str, size: f32) -> f32 {
    let extra = if FAKE_BOLD.with(|c| c.get()) { 0.7 } else { 0.0 };
    measure_font(style_font(fonts), s, size * style_k()) + extra
}

fn measure_bold(fonts: &Fonts, s: &str, size: f32) -> f32 {
    let extra = if fonts.bold_is_fake { 0.7 } else { 0.0 };
    measure_font(&fonts.bold, s, size * style_k()) + extra
}

fn measure_font(font: &Font, s: &str, size: f32) -> f32 {
    s.chars()
        .filter(|ch| *ch == ' ' || *ch == '\t' || font.has_glyph(*ch))
        .map(|ch| font.metrics(ch, size).advance_width)
        .sum()
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


fn standing_status(row: &crate::shm::Standing) -> Option<&'static str> {
    match row.state {
        1 => Some("DNS"),
        3 => Some("OUT"),
        4 => Some("DSQ"),
        _ if row.pit != 0 => Some("PIT"),
        _ => None,
    }
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
    // Honor configured widths when they fit. Only shrink on overflow so width
    // sliders (especially Name) actually change how wide each column draws.
    if leftover < -0.5 {
        let flex = cols.iter().position(|&c| is_flex(c)).unwrap_or(cols.len() - 1);
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

fn hug_board_w<T>(origin: f32, pad: f32, max_w: f32, slots: &[(T, f32, f32)]) -> f32 {
    let Some((_, x, cw)) = slots.last() else {
        return max_w;
    };
    (x + cw - origin + pad).clamp(pad * 2.0 + 40.0, max_w)
}

fn fill_focus_row(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, c: Color) {
    if let Some(rrt) = rr(x, y, w, h) {
        fill_rect(px, rrt, c);
    }
}

fn scale_a(opacity_pct: i32) -> u8 {
    ((255u32 * opacity_pct.clamp(0, 100) as u32) / 100).min(255) as u8
}

fn you_row_bg(opacity_pct: i32) -> Color {
    Color::from_rgba8(196, 132, 36, scale_a(opacity_pct))
}

/// Extra black only reads while the game still shows through. Opaque night-ink
/// cannot get darker, so the stripe lifts to a slightly lighter charcoal.
fn stripe_row_bg(panel_a: u8) -> Color {
    if panel_a == 0 {
        return Color::from_rgba8(0, 0, 0, 0);
    }
    // Stay on the black wash through the default ~78% Background; crossfade
    // to white from ~80% so 100% still zebras.
    let lift = panel_a.saturating_sub(204) as u32;
    let dark = 51u32.saturating_sub(lift);
    let rgb = ((255u32 * lift) / 51) as u8;
    let dark_a = (panel_a as u32 * 70 / 255) * dark / 51;
    let light_a = (34u32 * lift * panel_a as u32) / (51 * 255);
    Color::from_rgba8(rgb, rgb, rgb, (dark_a + light_a) as u8)
}

fn lapping_row_bg(opacity_pct: i32) -> Color {
    Color::from_rgba8(59, 130, 246, scale_a(opacity_pct))
}

fn lapped_row_bg(opacity_pct: i32) -> Color {
    Color::from_rgba8(239, 68, 68, scale_a(opacity_pct))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LapRel {
    Same,
    LappingMe,
    LappedByMe,
}

fn wrap_frac(other: f32, self_p: f32) -> f32 {
    let mut d = other - self_p;
    if d > 0.5 {
        d -= 1.0;
    }
    if d < -0.5 {
        d += 1.0;
    }
    d
}

fn rider_norm_pos(s: &Snapshot, race_num: i32) -> Option<f32> {
    let focus = if s.focus_race_num > 0 {
        s.focus_race_num
    } else {
        s.local_race_num
    };
    if race_num == focus {
        let p = focus_track_pos(s);
        return if p < 0.0 { None } else { Some(p) };
    }
    s.riders
        .iter()
        .take(s.rider_count.max(0) as usize)
        .find(|r| r.race_num == race_num)
        .map(|r| r.track_pos)
        .filter(|p| *p >= 0.0)
        .map(|p| norm_track_pos(s, p))
}

/// Integer laps `other` is ahead of `me`.
///
/// `gap_laps` is laps behind the leader (0 for P1). Prefer it over raw
/// `num_laps`: MX Bikes can keep a lapped rider on the race lap, so after you
/// complete another crossing you look a lap *up* while still two down. That
/// used to paint the leader red once they went by the second time.
fn other_laps_ahead(other: &crate::shm::Standing, me: &crate::shm::Standing) -> i32 {
    let by_gap = me.gap_laps - other.gap_laps;
    if by_gap != 0 {
        by_gap
    } else {
        other.num_laps - me.num_laps
    }
}

fn catch_span_m(s: &Snapshot) -> f32 {
    let len = if s.track_length > 10.0 {
        s.track_length
    } else {
        1200.0
    };
    (len * 0.18).clamp(80.0, 200.0)
}

fn closing_m(s: &Snapshot, along: f32) -> f32 {
    let len = if s.track_length > 10.0 {
        s.track_length
    } else {
        1200.0
    };
    along * len
}

fn lap_rel(s: &Snapshot, race_num: i32) -> LapRel {
    if is_warmup(s) {
        return LapRel::Same;
    }
    let focus = if s.focus_race_num > 0 {
        s.focus_race_num
    } else {
        s.local_race_num
    };
    if race_num <= 0 || race_num == focus {
        return LapRel::Same;
    }
    let race = RaceStore::get();
    let me = race
        .field
        .row_by_num(focus)
        .map(|r| &r.standing)
        .or_else(|| standing_of(s, focus));
    let other = race
        .field
        .row_by_num(race_num)
        .map(|r| &r.standing)
        .or_else(|| standing_of(s, race_num));
    let (Some(me), Some(other)) = (me, other) else {
        return LapRel::Same;
    };
    let Some(op) = rider_norm_pos(s, race_num) else {
        return LapRel::Same;
    };
    let Some(mp) = rider_norm_pos(s, focus) else {
        return LapRel::Same;
    };
    let ahead = other_laps_ahead(other, me);
    let w = wrap_frac(op, mp);
    let behind_m = if w < 0.0 { closing_m(s, -w) } else { 0.0 };
    let ahead_m = if w > 0.0 { closing_m(s, w) } else { 0.0 };
    let span = catch_span_m(s);
    if ahead >= 1 && behind_m > 2.0 && behind_m <= span {
        LapRel::LappingMe
    } else if ahead <= -1 && ahead_m > 2.0 && ahead_m <= span {
        LapRel::LappedByMe
    } else {
        LapRel::Same
    }
}

fn rider_dot_col(s: &Snapshot, race_num: i32) -> Color {
    match lap_rel(s, race_num) {
        LapRel::LappingMe => lapping_col(),
        LapRel::LappedByMe => lapped_col(),
        LapRel::Same => other_col(),
    }
}

fn lap_row_bg(rel: LapRel, opacity_pct: i32) -> Option<Color> {
    match rel {
        LapRel::Same => None,
        LapRel::LappingMe => Some(lapping_row_bg(opacity_pct)),
        LapRel::LappedByMe => Some(lapped_row_bg(opacity_pct)),
    }
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

fn table_ink(mode: TableText) -> (Color, Color, Color, Color) {
    match mode {
        TableText::White => (
            text_col(),
            Color::from_rgba8(210, 210, 216, 255),
            Color::from_rgba8(110, 110, 116, 255),
            Color::from_rgba8(160, 160, 168, 220),
        ),
        TableText::Black => (
            Color::from_rgba8(16, 16, 18, 255),
            Color::from_rgba8(48, 48, 54, 255),
            Color::from_rgba8(90, 90, 98, 255),
            Color::from_rgba8(64, 64, 72, 220),
        ),
    }
}

const BIKE_BAR_W: f32 = 5.0;
const BIKE_BAR_SKEW: f32 = 3.0;
const BIKE_BAR_PAD: f32 = 5.0;
const BIKE_PILL_PAD_X: f32 = 10.0;
const BIKE_PILL_PAD_Y: f32 = 4.0;

fn bike_bar_end(pos_cx: f32, pos_cw: f32) -> f32 {
    pos_cx + pos_cw + 1.0 + BIKE_BAR_W + BIKE_BAR_SKEW
}

fn name_left_pad(cx: f32, bar_end: Option<f32>) -> f32 {
    bar_end.map(|end| (end + BIKE_BAR_PAD - cx).max(0.0)).unwrap_or(0.0)
}

fn draw_bike_pill(
    px: &mut Pixmap,
    fonts: &Fonts,
    label: &str,
    cx: f32,
    cy: f32,
    cw: f32,
    row_h: f32,
    accent_c: Color,
) {
    let font_sz = 9.0;
    let max_inner = (cw - BIKE_PILL_PAD_X * 2.0).max(8.0);
    let badge = ellipsize(fonts, label, font_sz, max_inner);
    let tw = measure(fonts, &badge, font_sz);
    let bw = (tw + BIKE_PILL_PAD_X * 2.0).min(cw);
    let bh = font_sz + BIKE_PILL_PAD_Y * 2.0;
    let bx = cx + ((cw - bw) * 0.5).max(0.0);
    let by = cy + ((row_h - bh) * 0.5).max(0.0);
    fill_round(px, bx, by, bw, bh, 4.0, accent_c);
    text(
        px,
        fonts,
        &badge,
        font_sz,
        bx + BIKE_PILL_PAD_X,
        by + BIKE_PILL_PAD_Y - 1.0,
        ink_on(accent_c),
        false,
    );
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


static STATUS_HINT: Mutex<String> = Mutex::new(String::new());

pub fn set_status_hint(s: impl Into<String>) {
    if let Ok(mut g) = STATUS_HINT.lock() {
        *g = s.into();
    }
}

static SYS_CPU: AtomicI32 = AtomicI32::new(0);
static SYS_MEM: AtomicI32 = AtomicI32::new(0);
static SYS_FPS: AtomicI32 = AtomicI32::new(0);
static SYS_NET: AtomicI32 = AtomicI32::new(0);
const SYS_PROC_N: usize = 4;
const SYS_PROC_LABELS: [&str; SYS_PROC_N] = ["HUD", "MX Bikes", "MXB App", "ReShade"];
static SYS_PROC_CPU: [AtomicI32; SYS_PROC_N] = [
    AtomicI32::new(i32::MIN),
    AtomicI32::new(i32::MIN),
    AtomicI32::new(i32::MIN),
    AtomicI32::new(i32::MIN),
];
static SYS_PROC_MEM: [AtomicI32; SYS_PROC_N] = [
    AtomicI32::new(0),
    AtomicI32::new(0),
    AtomicI32::new(0),
    AtomicI32::new(0),
];
static SYS_PROC_MEMP: [AtomicI32; SYS_PROC_N] = [
    AtomicI32::new(0),
    AtomicI32::new(0),
    AtomicI32::new(0),
    AtomicI32::new(0),
];
static SYS_PROC_ON: [AtomicI32; SYS_PROC_N] = [
    AtomicI32::new(0),
    AtomicI32::new(0),
    AtomicI32::new(0),
    AtomicI32::new(0),
];

#[derive(Clone, Copy, Default)]
pub struct SysProc {
    pub cpu: f32,
    pub mem_mb: f32,
    pub mem_pct: f32,
    pub on: bool,
}

pub fn set_sys_stats(cpu: f32, mem: f32, fps: f32, net: f32) {
    SYS_CPU.store((cpu.clamp(0.0, 100.0) * 10.0).round() as i32, Ordering::Relaxed);
    SYS_MEM.store((mem.clamp(0.0, 100.0) * 10.0).round() as i32, Ordering::Relaxed);
    SYS_FPS.store((fps.clamp(0.0, 999.0) * 10.0).round() as i32, Ordering::Relaxed);
    SYS_NET.store((net.clamp(0.0, 100.0) * 10.0).round() as i32, Ordering::Relaxed);
}

pub fn set_sys_procs(procs: [SysProc; SYS_PROC_N]) {
    for (i, p) in procs.iter().enumerate() {
        let cpu = if !p.on || p.cpu < 0.0 {
            i32::MIN
        } else {
            (p.cpu.clamp(0.0, 100.0) * 10.0).round() as i32
        };
        SYS_PROC_CPU[i].store(cpu, Ordering::Relaxed);
        SYS_PROC_MEM[i].store((p.mem_mb.clamp(0.0, 1_000_000.0) * 10.0).round() as i32, Ordering::Relaxed);
        SYS_PROC_MEMP[i].store((p.mem_pct.clamp(0.0, 100.0) * 10.0).round() as i32, Ordering::Relaxed);
        SYS_PROC_ON[i].store(i32::from(p.on), Ordering::Relaxed);
    }
}

static STANCE_SIT: AtomicI32 = AtomicI32::new(0);

pub fn set_stance(sitting: bool) {
    STANCE_SIT.store(i32::from(sitting), Ordering::Relaxed);
}

pub fn stance_sitting() -> bool {
    STANCE_SIT.load(Ordering::Relaxed) != 0
}

fn sys_stats() -> (f32, f32, f32, f32) {
    (
        SYS_CPU.load(Ordering::Relaxed) as f32 / 10.0,
        SYS_MEM.load(Ordering::Relaxed) as f32 / 10.0,
        SYS_FPS.load(Ordering::Relaxed) as f32 / 10.0,
        SYS_NET.load(Ordering::Relaxed) as f32 / 10.0,
    )
}

fn sys_procs() -> [SysProc; SYS_PROC_N] {
    std::array::from_fn(|i| {
        let cpu_raw = SYS_PROC_CPU[i].load(Ordering::Relaxed);
        SysProc {
            cpu: if cpu_raw == i32::MIN {
                -1.0
            } else {
                cpu_raw as f32 / 10.0
            },
            mem_mb: SYS_PROC_MEM[i].load(Ordering::Relaxed) as f32 / 10.0,
            mem_pct: SYS_PROC_MEMP[i].load(Ordering::Relaxed) as f32 / 10.0,
            on: SYS_PROC_ON[i].load(Ordering::Relaxed) != 0,
        }
    })
}


fn camera_subject(s: &Snapshot) -> i32 {
    if s.focus_race_num > 0 {
        s.focus_race_num
    } else {
        s.local_race_num
    }
}

struct SubjectPose {
    x: f32,
    z: f32,
    yaw: f32,
    vel_x: f32,
    vel_z: f32,
    crashed: bool,
    from_local: bool,
}

/// Where the orange "you" marker lives: live bike data while riding, the spectated
/// rider's XZ while watching someone else. Overlay clears leftover telemetry while
/// `SpectateVehicles` is live so spectate can follow the camera; once that stops,
/// telemetry belongs to you again even if focus is still stale.
fn subject_pose(s: &Snapshot, age: f32) -> Option<SubjectPose> {
    if s.has_telemetry != 0 {
        return Some(SubjectPose {
            x: s.local_x + s.local_vel_x * age,
            z: s.local_z + s.local_vel_z * age,
            yaw: s.local_yaw,
            vel_x: s.local_vel_x,
            vel_z: s.local_vel_z,
            crashed: s.local_crashed != 0,
            from_local: true,
        });
    }
    let subject = camera_subject(s);
    let n = s.rider_count.max(0) as usize;
    s.riders.iter().take(n).find(|r| r.race_num == subject).map(|r| SubjectPose {
        x: r.x,
        z: r.z,
        yaw: r.yaw,
        vel_x: 0.0,
        vel_z: 0.0,
        crashed: r.crashed != 0,
        from_local: false,
    })
}

fn focus_track_pos(s: &Snapshot) -> f32 {
    let focus = camera_subject(s);
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

fn lap_meters(s: &Snapshot) -> f32 {
    if s.track_length > 10.0 {
        s.track_length
    } else {
        1200.0
    }
}

fn sf_frac(s: &Snapshot) -> f32 {
    // A crossing we watched beats `sf_meters`, which stays 0 when the game never
    // sends a centerline — that would park the flag window at the centerline origin.
    let learned = SF_FRAC_LEARNED.load(Ordering::Relaxed);
    if learned >= 0 {
        return (learned as f32 / 10_000.0).rem_euclid(1.0);
    }
    if s.track_length > 1.0 && s.sf_meters >= 0.0 {
        (s.sf_meters / s.track_length).rem_euclid(1.0)
    } else {
        0.0
    }
}

/// True when we have no idea where the line is: nothing learned, no centerline, and
/// `sf_meters` still at its default.
fn sf_uncalibrated(s: &Snapshot) -> bool {
    SF_FRAC_LEARNED.load(Ordering::Relaxed) < 0 && s.poly_count <= 0 && s.sf_meters <= 0.0
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
    Some(dist_to_sf(pos, sf_frac(s)) * lap_meters(s))
}

const FLAG_LINE_M: f32 = 80.0;
const FLAG_LINE_MIN_M: f32 = 4.0;
/// Crash ahead of you, close enough to need a yellow.
const FLAG_YELLOW_SPAN_M: f32 = 50.0;
/// Lapper closing from behind — shorter than `catch_span_m` so blue is not a whole-stretch warning.
const FLAG_BLUE_SPAN_M: f32 = 40.0;

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

const SF_AGREE_FRAC: f32 = 0.05;

/// A lap counted at `frac` of the way round. Two sightings within 5% of a lap of each
/// other confirm the line; a lone odd one only replaces the candidate.
fn note_line_sighting(frac: f32) {
    let ticks = (frac * 10_000.0).round() as i32;
    let cand = SF_FRAC_CAND.swap(ticks, Ordering::Relaxed);
    if cand < 0 {
        return;
    }
    let d = (frac - cand as f32 / 10_000.0).abs();
    if d.min(1.0 - d) <= SF_AGREE_FRAC {
        SF_FRAC_LEARNED.store(ticks, Ordering::Relaxed);
    }
}

/// Watch the S/F line: learn where it is from your own crossings, remember whether you
/// have been round the far side, and keep the previous distance so the run-in must close.
fn note_line_progress(s: &Snapshot) {
    let Some(remain) = meters_to_sf(s) else {
        return;
    };
    let len = lap_meters(s);
    let laps = focus_num_laps(s);
    let prev_laps = SF_LEARN_LAPS.swap(laps, Ordering::Relaxed);
    if prev_laps >= 0 && laps > prev_laps {
        LAP_MID_SEEN.store(0, Ordering::Relaxed);
        CLOSING_ON_LINE.store(0, Ordering::Relaxed);
        if moving(s) {
            // We are a frame or so past the line; back out that much travel.
            let overshoot = (s.local_speed / 60.0 / len).clamp(0.0, 0.02);
            let frac = (focus_track_pos(s) - overshoot).rem_euclid(1.0);
            note_line_sighting(frac);
        }
    }
    let frac = remain / len;
    if (0.4..=0.6).contains(&frac) {
        LAP_MID_SEEN.store(1, Ordering::Relaxed);
    }
    let prev = LAST_SF_METERS.swap(remain.round() as i32, Ordering::Relaxed);
    // Sticky while the distance is still falling or flat — only clear when you
    // clearly open the gap again (jitter used to flicker white open/closed).
    if prev >= 0 && remain < prev as f32 - 0.5 {
        CLOSING_ON_LINE.store(1, Ordering::Relaxed);
    } else if prev >= 0 && remain > prev as f32 + 1.5 {
        CLOSING_ON_LINE.store(0, Ordering::Relaxed);
    }
}

/// You are closing on the line, not sitting in the window or heading away from it.
fn closing_on_line() -> bool {
    CLOSING_ON_LINE.load(Ordering::Relaxed) == 1
}

/// The run-in to the line, where a flagger would be standing. This is the only flag
/// decision that depends on track geometry, so it is heavily guarded and simply does not
/// fire when the data is bad.
fn line_approach(s: &Snapshot) -> bool {
    if sf_uncalibrated(s) || !moving(s) || !approaching_line(s) {
        return false;
    }
    LAP_MID_SEEN.load(Ordering::Relaxed) == 1 && closing_on_line()
}

/// The last metres before the line, plus the stretch just after it. `approaching_line`
/// stops at `FLAG_LINE_MIN_M`, so without this the banner is None for ~4 m (and a
/// classification-lag beat past the line) — long enough for the hide animation to start.
fn across_the_line(s: &Snapshot) -> bool {
    if sf_uncalibrated(s) {
        return false;
    }
    meters_to_sf(s).is_some_and(|m| m <= FLAG_LINE_MIN_M || m >= lap_meters(s) - FLAG_LINE_M)
}

fn reset_flag_state() -> DashFlag {
    CHECKERED_LATCH.store(0, Ordering::Relaxed);
    WHITE_WAVE_LAP.store(-1, Ordering::Relaxed);
    RUN_IN_FLAG.store(0, Ordering::Relaxed);
    DashFlag::None
}

fn flag_code(flag: DashFlag) -> i32 {
    match flag {
        DashFlag::None => 0,
        DashFlag::White => 1,
        DashFlag::Checkered => 2,
        DashFlag::Yellow => 3,
        DashFlag::Blue => 4,
    }
}

fn flag_from_code(code: i32) -> DashFlag {
    match code {
        1 => DashFlag::White,
        2 => DashFlag::Checkered,
        3 => DashFlag::Yellow,
        4 => DashFlag::Blue,
        _ => DashFlag::None,
    }
}

/// Keep the run-in flag out across the line. Your lap count only catches up a frame or
/// two after you are past it, and until it does the lap rules describe the lap you have
/// already finished. The last `FLAG_LINE_MIN_M` before the line is the same gap: the
/// approach window has closed and the wrap-around hold has not opened yet.
fn hold_across_line(s: &Snapshot, flag: DashFlag) -> DashFlag {
    if line_approach(s) {
        if flag != DashFlag::None {
            RUN_IN_FLAG.store(flag_code(flag), Ordering::Relaxed);
        }
        return flag;
    }
    if !across_the_line(s) {
        RUN_IN_FLAG.store(0, Ordering::Relaxed);
        return flag;
    }
    let held = flag_from_code(RUN_IN_FLAG.load(Ordering::Relaxed));
    if held == DashFlag::None {
        return flag;
    }
    if flag == DashFlag::Checkered {
        RUN_IN_FLAG.store(0, Ordering::Relaxed);
        return flag;
    }
    held
}

/// How long the white stays up after it is waved.
const WHITE_WAVE_MS: i32 = 5_000;

fn now_ms() -> i32 {
    (anim_now() * 1000.0) as i32
}

/// White from the first frame this lap that calls for it — the crossing onto your last
/// lap, or the moment the leader's finish makes the lap you are on your last — and for
/// `WHITE_WAVE_MS` after, so it is not held up all the way to the finish.
fn white_wave(s: &Snapshot) -> DashFlag {
    let lap = focus_num_laps(s);
    let now = now_ms();
    if WHITE_WAVE_LAP.swap(lap, Ordering::Relaxed) != lap {
        WHITE_WAVE_AT.store(now, Ordering::Relaxed);
        return DashFlag::White;
    }
    if now - WHITE_WAVE_AT.load(Ordering::Relaxed) <= WHITE_WAVE_MS {
        DashFlag::White
    } else {
        DashFlag::None
    }
}

fn latch_checkered() -> DashFlag {
    CHECKERED_LATCH.store(1, Ordering::Relaxed);
    DashFlag::Checkered
}

/// One path for lap motos and timed extras. Both flags go up on the run-in to the line:
/// white onto your final lap, checkered onto the finish. Only the crossing latches the
/// checkered, and the white comes down a few seconds into the lap.
fn dash_race_flag(s: &Snapshot) -> DashFlag {
    if s.on_track == 0 {
        return reset_flag_state();
    }
    note_line_progress(s);
    let store = RaceStore::get();
    let stale = {
        let b = &store.clock.banner.1;
        b.is_empty() || b == "--:--"
    };
    if is_lap_race(s) {
        // Gate boards run a countdown; no flags until the race is actually under way.
        let remain = if stale {
            session_remain_ms(s)
        } else {
            store.clock.remain_ms
        };
        if remain.is_some_and(|r| r > 60_000) {
            return reset_flag_state();
        }
    } else if stale {
        // Keep the count-only flag latch in step when the store has not ticked yet.
        let _ = timed_race_flag(s);
    }
    if prestart(s) {
        return reset_flag_state();
    }
    if CHECKERED_LATCH.load(Ordering::Relaxed) == 1 {
        return DashFlag::Checkered;
    }
    let left = laps_left(s);
    note_laps_to_run(s, left);
    let flag = match left {
        // You crossed the line with nothing left to run. Never gated on speed, so a
        // slow roll over the line still gets waved off. `finish_earned` keeps a single
        // glitched frame from waving you off mid-race.
        Some(0) if finish_earned(s) => latch_checkered(),
        // Coming to the line with nothing left to run: the checkered is already out.
        Some(1) if line_approach(s) => DashFlag::Checkered,
        Some(0) | Some(1) => white_wave(s),
        Some(2) if line_approach(s) => DashFlag::White,
        // The leader is done, so whatever lap you are on is your last.
        _ if leader_finished(s) => white_wave(s),
        _ => DashFlag::None,
    };
    hold_across_line(s, flag)
}

/// Website demo: 0 none, 1 white, 2 checkered, 3 yellow, 4 blue. Negative = live.
static FLAG_PREVIEW: AtomicI32 = AtomicI32::new(-1);

pub fn set_flag_preview(code: i32) {
    FLAG_PREVIEW.store(code, Ordering::Relaxed);
    if code > 0 {
        let mut st = FLAG_ANIM.lock().unwrap_or_else(|e| e.into_inner());
        st.kind = flag_from_code(code);
        st.t0 = anim_now() - 1.0;
        st.hiding = false;
        st.none_since = -1.0;
    }
}

#[allow(dead_code)]
pub(crate) fn reset_flag_display() {
    FLAG_PREVIEW.store(-1, Ordering::Relaxed);
    let mut st = FLAG_ANIM.lock().unwrap_or_else(|e| e.into_inner());
    *st = FlagAnim {
        kind: DashFlag::None,
        t0: 0.0,
        hiding: false,
        none_since: -1.0,
    };
}

fn tick_display_flag(s: &Snapshot, yellow: bool, blue: bool) -> (DashFlag, f32) {
    flag_anim_step(wanted_flag(s, yellow, blue))
}

fn wanted_flag(s: &Snapshot, yellow: bool, blue: bool) -> DashFlag {
    match FLAG_PREVIEW.load(Ordering::Relaxed) {
        0 => DashFlag::None,
        1 => DashFlag::White,
        2 => DashFlag::Checkered,
        3 => DashFlag::Yellow,
        4 => DashFlag::Blue,
        _ => {
            let race = dash_race_flag(s);
            if yellow || blue {
                merge_caution(race, caution_flag(s, yellow, blue))
            } else {
                race
            }
        }
    }
}

fn merge_caution(race: DashFlag, caution: DashFlag) -> DashFlag {
    match race {
        DashFlag::White | DashFlag::Checkered => race,
        _ => caution,
    }
}

/// Inferred only. The game has no marshal flag field. Yellow is a nearby crash;
/// blue is someone a lap up closing from behind (`LapRel::LappingMe`).
fn caution_flag(s: &Snapshot, yellow: bool, blue: bool) -> DashFlag {
    if s.on_track == 0 || is_warmup(s) || prestart(s) {
        return DashFlag::None;
    }
    if yellow && nearby_crash(s) {
        DashFlag::Yellow
    } else if blue && being_lapped(s) {
        DashFlag::Blue
    } else {
        DashFlag::None
    }
}

fn nearby_crash(s: &Snapshot) -> bool {
    let focus = if s.focus_race_num > 0 {
        s.focus_race_num
    } else {
        s.local_race_num
    };
    let Some(mp) = rider_norm_pos(s, focus) else {
        return false;
    };
    s.riders
        .iter()
        .take(s.rider_count.max(0) as usize)
        .any(|r| {
            if r.race_num <= 0 || r.race_num == focus || r.crashed == 0 {
                return false;
            }
            let Some(op) = rider_norm_pos(s, r.race_num) else {
                return false;
            };
            let along = closing_m(s, wrap_frac(op, mp));
            along > FLAG_LINE_MIN_M && along <= FLAG_YELLOW_SPAN_M
        })
}

fn being_lapped(s: &Snapshot) -> bool {
    let focus = if s.focus_race_num > 0 {
        s.focus_race_num
    } else {
        s.local_race_num
    };
    s.riders
        .iter()
        .take(s.rider_count.max(0) as usize)
        .any(|r| {
            if r.race_num <= 0 || r.race_num == focus {
                return false;
            }
            if lap_rel(s, r.race_num) != LapRel::LappingMe {
                return false;
            }
            let Some(op) = rider_norm_pos(s, r.race_num) else {
                return false;
            };
            let Some(mp) = rider_norm_pos(s, focus) else {
                return false;
            };
            let w = wrap_frac(op, mp);
            let behind_m = if w < 0.0 { closing_m(s, -w) } else { 0.0 };
            behind_m > 2.0 && behind_m <= FLAG_BLUE_SPAN_M
        })
}

fn dash_wrap_flag(flag: DashFlag, grow: f32) -> (DashFlag, f32) {
    match flag {
        DashFlag::Yellow | DashFlag::Blue => (DashFlag::None, 0.0),
        other => (other, grow),
    }
}




fn format_local_clock(hour: u16, minute: u16) -> String {
    let h24 = hour % 24;
    let h12 = match h24 {
        0 => 12,
        13..=23 => h24 - 12,
        _ => h24,
    };
    let ampm = if h24 < 12 { "AM" } else { "PM" };
    format!("{h12}:{minute:02} {ampm}")
}

fn local_clock() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        let mins = (anim_now() as i32 / 60).rem_euclid(24 * 60);
        return format_local_clock((mins / 60) as u16, (mins % 60) as u16);
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
        format_local_clock(st.hour, st.minute)
    }
}

fn format_fuel_pct(fuel: f32, max_fuel: f32) -> String {
    if max_fuel > 0.01 {
        let pct = (fuel.max(0.0) / max_fuel * 100.0).round().clamp(0.0, 100.0);
        format!("{pct:.0}%")
    } else {
        "--%".into()
    }
}

fn board_item(s: &Snapshot, cfg: &HudConfig, field: BoardField) -> Option<(char, String)> {
    if field == BoardField::None {
        return None;
    }
    let race = RaceStore::get();
    let st = race
        .field
        .focus
        .and_then(|i| race.field.rows.get(i))
        .map(|r| &r.standing)
        .or_else(|| focus_standing(s));
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
        BoardField::Session | BoardField::RaceTime | BoardField::Lap => race_progress_text(s),
        BoardField::LapsLeft => race_laps_left_text(s),
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
        BoardField::SessionBest => {
            let best = if race.field.session_best_ms > 0 {
                race.field.session_best_ms
            } else {
                session_best_ms(s)
            };
            format_clock(best)
        }
        BoardField::LocalTime => local_clock(),
        BoardField::Riders => format!("{}", s.standing_count.max(s.rider_count).max(0)),
        BoardField::SessionType => {
            if is_lap_race(s) {
                "Lap race".into()
            } else if overtime_active(s) {
                "Extra".into()
            } else if s.session_length > 0 {
                "Timed".into()
            } else {
                "Session".into()
            }
        }
        BoardField::Fuel => cfg.units.format_fuel(s.fuel, s.max_fuel),
        BoardField::FuelPct => format_fuel_pct(s.fuel, s.max_fuel),
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
    let max_w = r.w * sw;
    let race = RaceStore::get();
    // Live order and live places, so a pass moves a row now instead of at the line.
    let mut board = race.field.board();
    if board.is_empty() {
        let count = (s.standing_count.max(0) as usize).min(MAX_SAFE);
        board.extend_from_slice(&s.standings[..count]);
    }
    let rows = board.len();
    let max_rows = s.standings_rows.max(3) as usize;
    let n = rows.min(MAX_SAFE);
    let focus = s.focus_race_num;
    let mut start = 0;
    if n > max_rows {
        let fi = (0..n).find(|&i| board[i].race_num == focus).unwrap_or(0);
        start = fi.saturating_sub(max_rows / 2).min(n.saturating_sub(max_rows));
    }
    let end = (start + max_rows).min(n);
    let slice = &board[start..end];

    let k = style_k();
    let head_h = 26.0 * k;
    let col_h = 16.0 * k;
    let track_h = 20.0 * k;
    let row_h = 22.0 * k;
    let foot_h = if BoardField::any(&cfg.st_foot) { 20.0 * k } else { 0.0 };
    let vis = slice.len().max(1);
    let h = (head_h + col_h + track_h + vis as f32 * row_h + foot_h + 8.0).min(r.h * sh);
    let cols = cfg.standings_cols();
    let pad = 8.0;
    let slots = col_slots(x, pad, max_w, &cols, |c| c.width(cfg) as f32, |c| matches!(c, StField::Name));
    let w = hug_board_w(x, pad, max_w, &slots);
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

    let (ink, ink_dim, out_c, hdr_c) = table_ink(cfg.st_text);
    let bar_end = slots
        .iter()
        .find(|(c, _, _)| *c == StField::Pos)
        .map(|(_, cx, cw)| bike_bar_end(*cx, *cw));

    let purple = Color::from_rgba8(196, 112, 255, 255);
    let best_ms = if race.field.session_best_ms > 0 {
        race.field.session_best_ms
    } else {
        board
            .iter()
            .map(|row| row.best_lap_ms)
            .filter(|ms| *ms > 0)
            .min()
            .unwrap_or(0)
    };
    let you_bg = you_row_bg(cfg.st_hl);
    let stripe_c = stripe_row_bg(a);

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
        if cfg.st_stripe && vis_i % 2 == 1 && a > 0 {
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

        let name_c = if out { out_c } else { ink };
        let dim = if out { out_c } else { ink_dim };
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
                StField::Interval => {
                    let txt = race
                        .field
                        .row_by_num(row.race_num)
                        .map(interval_text_from_row)
                        .unwrap_or_else(|| interval_text(s, row));
                    (txt, dim, true)
                }
                StField::Laps => (format!("{}", row.num_laps.max(0)), dim, true),
                StField::Current => {
                    let lap = race
                        .field
                        .row_by_num(row.race_num)
                        .map(|r| r.current_lap)
                        .unwrap_or_else(|| rider_current_lap(s, row.race_num, row.num_laps));
                    (format!("{lap}"), dim, true)
                }
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
            if *kind == StField::Name {
                push_click_rider(row.race_num, *cx + pad, cy, (*cw - pad).max(8.0), row_h);
            }
            if *kind == StField::Bike && !val.is_empty() {
                draw_bike_pill(px, fonts, &val, *cx, cy, *cw, row_h, accent_c);
            } else {
                col_text(px, fonts, &val, 12.0, *cx + pad, (*cw - pad).max(8.0), cy + 4.0, color, right);
            }
        }
        if !slots.iter().any(|(kind, _, _)| *kind == StField::Name) {
            push_click_rider(row.race_num, x, cy, w, row_h);
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

fn ticker_meta_label(field: BoardField, val: &str) -> &'static str {
    match field {
        BoardField::Lap | BoardField::LapsLeft | BoardField::Session | BoardField::RaceTime => {
            if val.contains('/') || val.starts_with('+') {
                "LAPS"
            } else {
                "TIME"
            }
        }
        BoardField::Air => "TEMP",
        BoardField::Best | BoardField::SessionBest => "BEST",
        BoardField::Position | BoardField::ClassPos => "POS",
        BoardField::Track => "TRACK",
        BoardField::Riders => "RIDERS",
        BoardField::LocalTime => "CLOCK",
        BoardField::SessionType => "SESSION",
        BoardField::Fuel | BoardField::FuelPct => "FUEL",
        BoardField::None => "",
    }
}

fn ticker_title(s: &Snapshot) -> String {
    let track = cstr(&s.track_name);
    let kind = if is_warmup(s) {
        "WARMUP"
    } else if is_lap_race(s) {
        "LAP RACE"
    } else if overtime_active(s) {
        "EXTRA"
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

fn sector_col_fracs(fresh: i32) -> [f32; 3] {
    if (0..=2).contains(&fresh) {
        let mut f = [0.22, 0.22, 0.22];
        f[fresh as usize] = 0.56;
        f
    } else {
        [1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0]
    }
}

fn sector_hero_index(s: &Snapshot) -> i32 {
    crate::sector::hero_index(s)
}

fn draw_sector(px: &mut Pixmap, fonts: &Fonts, s: &Snapshot, cfg: &HudConfig, sw: f32, sh: f32) {
    // THESIS: the sector you are in owns the board — live delta vs your best at this point, freeze when you leave.
    // OWN-WORLD: night-ink 6px plaque, hairline columns, ExtraBold Italic deltas, orange skew S# plaque on the current cell.
    // STORY: rider glances at this stretch, reads up/down vs their best, keeps eyes on the track.
    // FIRST VIEWPORT: three columns; current ~56% wide with huge live delta and orange S#. Flanks stay quiet. Pending is "--".
    // FORM: sector-comp-fresh, user-locked; PB is Ahead Green instead of Best Lap Violet. No glow.
    // FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
    let r = cfg.sector;
    let x = r.x * sw;
    let y = r.y * sh;
    let w = r.w * sw;
    let h = r.h * sh;
    if w < 88.0 || h < 44.0 {
        return;
    }
    let a = bg_a(cfg.sector_bg);
    let n = 3usize;
    let pad_x = (w * 0.03).clamp(6.0, 12.0);
    let pad_y = (h * 0.08).clamp(5.0, 10.0);
    if a > 0 {
        fill_round(px, x, y, w, h, 6.0, Color::from_rgba8(10, 10, 10, a));
        let rad = 6.0f32.min(w * 0.5).min(h * 0.5);
        let mut pb = PathBuilder::new();
        pb.move_to(x + rad, y);
        pb.line_to(x + w - rad, y);
        pb.quad_to(x + w, y, x + w, y + rad);
        pb.line_to(x + w, y + h - rad);
        pb.quad_to(x + w, y + h, x + w - rad, y + h);
        pb.line_to(x + rad, y + h);
        pb.quad_to(x, y + h, x, y + h - rad);
        pb.line_to(x, y + rad);
        pb.quad_to(x, y, x + rad, y);
        pb.close();
        if let Some(path) = pb.finish() {
            let edge = ((a as u16 * 70) / 255).max(36) as u8;
            stroke_path(px, &path, Color::from_rgba8(220, 220, 224, edge), 1.0);
        }
    }
    let inner_w = (w - pad_x * 2.0).max(60.0);
    let cap_fs = (h * 0.16).clamp(13.0, 20.0);
    let cap_y = y + (pad_y * 0.25).max(2.0);
    text(
        px,
        fonts,
        "vs. your best",
        cap_fs,
        x + w * 0.5,
        cap_y,
        text_dim(),
        true,
    );
    let body_y = cap_y + cap_fs + 4.0;
    let fracs = sector_col_fracs(sector_hero_index(s));
    let mut col_x = [0.0; 3];
    let mut col_w = [0.0; 3];
    let mut acc = x + pad_x;
    for i in 0..n {
        col_x[i] = acc;
        col_w[i] = inner_w * fracs[i];
        acc += col_w[i];
    }
    for i in 1..n {
        if let Some(line) = rr(col_x[i], body_y, 1.0, (y + h - pad_y - body_y).max(8.0)) {
            fill_rect(px, line, Color::from_rgba8(42, 42, 46, a.max(90)));
        }
    }
    let ink = Color::from_rgba8(12, 12, 14, 255);
    for i in 0..n {
        let row = sector_row(s, i, cfg.sector_live);
        let cx = col_x[i];
        let cw = col_w[i];
        let hero = row.fresh;
        if hero {
            if let Some(wash) = rr(cx + 1.0, body_y, (cw - 2.0).max(4.0), (y + h - pad_y - body_y).max(8.0)) {
                fill_rect(px, wash, Color::from_rgba8(255, 148, 48, 28));
            }
        }
        let delta_c = if row.pending || !row.has_delta {
            text_dim()
        } else if row.slower {
            behind_col()
        } else {
            ahead_col()
        };
        let time_c = if hero { text_col() } else { text_dim() };
        let label_fs = if hero {
            (h * 0.13).clamp(10.0, 14.0)
        } else {
            (h * 0.12).clamp(9.0, 12.0)
        };
        let delta_fs = if hero {
            (h * 0.38).max(18.0)
        } else {
            (h * 0.22).max(12.0)
        };
        let time_fs = if hero {
            (h * 0.14).clamp(10.0, 15.0)
        } else {
            (h * 0.12).clamp(9.0, 12.0)
        };
        let mid = cx + cw * 0.5;
        let label_y = body_y;
        if hero {
            let plaque_h = (label_fs + 8.0).min(h * 0.28);
            let plaque_w = (measure(fonts, row.label, label_fs) + 16.0).min(cw - 12.0);
            let skew = (plaque_h * 0.18).clamp(3.0, 6.0);
            let px0 = cx + 8.0;
            fill_skew(px, px0, label_y, plaque_w, plaque_h, skew, accent());
            text(
                px,
                fonts,
                row.label,
                label_fs,
                px0 + plaque_w * 0.5 + skew * 0.5,
                label_y + (plaque_h - label_fs) * 0.42,
                ink,
                true,
            );
        } else {
            text(px, fonts, row.label, label_fs, mid, label_y, text_dim(), true);
        }
        let delta_y = y + (h - delta_fs) * 0.42;
        text(px, fonts, &row.delta, delta_fs, mid, delta_y, delta_c, true);
        let time_y = y + h - pad_y - time_fs - 2.0;
        let tw = measure(fonts, &row.time, time_fs);
        let chip_x = 7.0;
        let chip_y = 3.0;
        fill_night_pill(
            px,
            mid - tw * 0.5 - chip_x,
            time_y - chip_y,
            tw + chip_x * 2.0,
            time_fs + chip_y * 2.0,
        );
        text(px, fonts, &row.time, time_fs, mid, time_y, time_c, true);
    }
}

struct SectorRowView {
    label: &'static str,
    time: String,
    delta: String,
    pending: bool,
    has_delta: bool,
    slower: bool,
    fresh: bool,
}

fn sector_row(s: &Snapshot, i: usize, live: bool) -> SectorRowView {
    let src = crate::sector::row(s, i, live);
    let showing = src.time_ms > 0;
    SectorRowView {
        label: src.label,
        time: if showing {
            format_lap(src.time_ms)
        } else {
            "--".into()
        },
        delta: if !showing || !src.has_delta {
            "--".into()
        } else {
            format_delta_ms(src.delta_ms)
        },
        pending: src.pending,
        has_delta: src.has_delta,
        slower: src.slower,
        fresh: src.fresh,
    }
}

/// Orange Δ mark. Bundled Exo 2 has no Delta glyph, so this is drawn.
fn fill_delta_mark(px: &mut Pixmap, x: f32, y: f32, h: f32, color: Color) -> f32 {
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

fn draw_delta(
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
    let r = cfg.delta;
    let x = r.x * sw;
    let y = r.y * sh;
    let w = r.w * sw;
    let h = r.h * sh;
    if w < 160.0 || h < 44.0 {
        return;
    }
    let a = bg_a(cfg.delta_bg);
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
    let pills = cfg.delta_bg < 40;
    if view.recording {
        let hint = "complete two full laps";
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
                px, fonts, "BEST", &best, line_x, cap_y, times_y, cap_fs, times_fs, pills,
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
fn draw_delta_lap_chip(
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

fn fmt_sys_mem(mb: f32) -> String {
    if mb < 0.05 {
        "0 MB".into()
    } else if mb < 9.95 {
        format!("{mb:.1} MB")
    } else if mb < 1024.0 {
        format!("{:.0} MB", mb.round())
    } else {
        let gb = mb / 1024.0;
        if gb < 9.95 {
            format!("{gb:.1} GB")
        } else {
            format!("{:.0} GB", gb.round())
        }
    }
}

fn sys_heat(hot: f32) -> Color {
    if hot >= 90.0 {
        Color::from_rgba8(239, 68, 68, 230)
    } else {
        Color::from_rgba8(250, 180, 48, 230)
    }
}

fn sys_fps_fill(fill: f32) -> Color {
    if fill >= 70.0 {
        Color::from_rgba8(232, 228, 214, 230)
    } else {
        sys_heat(100.0 - fill)
    }
}

fn draw_sys_track(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, fill: f32, col: Color) {
    let w = w.max(8.0);
    let h = h.clamp(2.0, 6.0);
    fill_round(px, x, y, w, h, 1.5, Color::from_rgba8(42, 42, 46, 160));
    let fw = (w * (fill / 100.0).clamp(0.0, 1.0)).max(if fill > 0.5 { h } else { 0.0 });
    if fw > 0.5 {
        fill_round(px, x, y, fw, h, 1.5, col);
    }
}

fn draw_sys_meter(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    value: &str,
    label_fs: f32,
    value_fs: f32,
    fill: f32,
    col: Color,
    track_h: f32,
) -> f32 {
    text(px, fonts, label, label_fs, x, y, text_dim(), false);
    let vy = y + label_fs * 1.12;
    text(px, fonts, value, value_fs, x, vy, Color::from_rgba8(248, 248, 252, 255), false);
    let ty = vy + value_fs * 1.08;
    draw_sys_track(px, x, ty, w, track_h, fill, col);
    ty + track_h
}

fn draw_sys_procs(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    row_h: f32,
    fs: f32,
    label_w: f32,
    procs: &[SysProc; SYS_PROC_N],
    mem: bool,
    mem_scale: f32,
) {
    let mute = Color::from_rgba8(132, 132, 138, 255);
    let bar = Color::from_rgba8(150, 150, 158, 110);
    let bar_dim = Color::from_rgba8(150, 150, 158, 40);
    for (i, p) in procs.iter().enumerate() {
        let ry = y + row_h * i as f32;
        let (value, fill, dim) = if mem {
            if p.on {
                (fmt_sys_mem(p.mem_mb), (p.mem_mb / mem_scale * 100.0).clamp(0.0, 100.0), false)
            } else {
                ("—".into(), 0.0, true)
            }
        } else {
            let known = p.on && p.cpu >= 0.0;
            if known {
                (format!("{:.0}%", p.cpu.round()), p.cpu.clamp(0.0, 100.0), false)
            } else {
                ("—".into(), 0.0, true)
            }
        };
        let ink = if dim { Color::from_rgba8(108, 108, 114, 220) } else { mute };
        text(px, fonts, SYS_PROC_LABELS[i], fs, x, ry + (row_h - fs) * 0.22, ink, false);
        let val_w = measure(fonts, &value, fs);
        let bx = x + label_w + 4.0;
        let bw = (x + w - val_w - 5.0 - bx).max(12.0);
        let bh = (row_h * 0.28).clamp(2.0, 4.0);
        let by = ry + (row_h - bh) * 0.42;
        draw_sys_track(px, bx, by, bw, bh, fill, if dim { bar_dim } else { bar });
        text(px, fonts, &value, fs, x + w - val_w, ry + (row_h - fs) * 0.22, ink, false);
    }
}

fn stance_glyph(sitting: bool) -> Option<&'static Pixmap> {
    static STAND: OnceLock<Option<Pixmap>> = OnceLock::new();
    static SIT: OnceLock<Option<Pixmap>> = OnceLock::new();
    let slot = if sitting { &SIT } else { &STAND };
    slot.get_or_init(|| {
        let bytes: &[u8] = if sitting {
            include_bytes!("../assets/stance-sit.png")
        } else {
            include_bytes!("../assets/stance-stand.png")
        };
        Pixmap::decode_png(bytes).ok()
    })
    .as_ref()
}

fn draw_stance_icon(px: &mut Pixmap, cfg: &HudConfig, x: f32, y: f32, w: f32, h: f32) -> bool {
    let sitting = stance_sitting();
    let Some(src) = stance_glyph(sitting) else {
        return false;
    };
    let a = bg_a(cfg.stance_bg);
    if a > 0 {
        fill_round(px, x, y, w, h, 6.0, Color::from_rgba8(10, 10, 12, a));
    }
    let pad = (w.min(h) * 0.10).max(4.0);
    let iw = src.width() as f32;
    let ih = src.height() as f32;
    if iw < 1.0 || ih < 1.0 {
        return false;
    }
    let scale = ((w - pad * 2.0) / iw).min((h - pad * 2.0) / ih).max(0.01);
    let dw = iw * scale;
    let dh = ih * scale;
    let dx = x + (w - dw) * 0.5;
    let dy = y + (h - dh) * 0.5;
    let mut paint = PixmapPaint::default();
    paint.quality = FilterQuality::Bicubic;
    px.draw_pixmap(
        0,
        0,
        src.as_ref(),
        &paint,
        Transform::from_row(scale, 0.0, 0.0, scale, dx, dy),
        None,
    );
    true
}

fn draw_stance(px: &mut Pixmap, fonts: &Fonts, cfg: &HudConfig, sw: f32, sh: f32) {
    let r = cfg.stance;
    let x = r.x * sw;
    let y = r.y * sh;
    let w = r.w * sw;
    let h = r.h * sh;
    if w < 36.0 || h < 22.0 {
        return;
    }
    if stance_sitting() && !cfg.stance_show_sit {
        return;
    }
    if cfg.stance_style == StanceStyle::Icon && draw_stance_icon(px, cfg, x, y, w, h) {
        return;
    }
    let sitting = stance_sitting();
    let a = bg_a(cfg.stance_bg);
    let stand = !sitting;
    let fill = if stand {
        Color::from_rgba8(255, 148, 48, a.max(220))
    } else {
        Color::from_rgba8(10, 10, 12, a)
    };
    let skew = (h * 0.12).clamp(3.0, 6.0);
    fill_skew(px, x, y, w, h, skew, fill);
    let word = if sitting { "SIT" } else { "STAND" };
    let ink = if stand {
        Color::from_rgba8(12, 12, 14, 255)
    } else {
        Color::from_rgba8(228, 228, 230, 255)
    };
    let size = (h * 0.42).clamp(12.0, 28.0);
    text(px, fonts, word, size, x + w * 0.5, y + (h - size) * 0.42, ink, true);
}

fn draw_flag(px: &mut Pixmap, fonts: &Fonts, cfg: &HudConfig, sw: f32, sh: f32, flag: DashFlag, grow: f32) {
    if flag == DashFlag::None || grow <= 0.02 {
        return;
    }
    let r = cfg.flag;
    let x = r.x * sw;
    let y = r.y * sh;
    let w = r.w * sw;
    let h = r.h * sh;
    if w < 48.0 || h < 10.0 {
        return;
    }
    let a = ((bg_a(cfg.flag_bg) as f32) * grow.clamp(0.0, 1.0)).round() as u8;
    if a == 0 {
        return;
    }
    let skew = (h * 0.32).clamp(6.0, 12.0);
    let inner_w = (w - skew).max(48.0);
    let mut pb = PathBuilder::new();
    pb.move_to(x + skew, y);
    pb.line_to(x + inner_w + skew, y);
    pb.line_to(x + inner_w, y + h);
    pb.line_to(x, y + h);
    pb.close();
    let Some(path) = pb.finish() else {
        return;
    };
    paint_flag_fill(px, &path, x, y, inner_w + skew, h, flag, a);
    let cx = x + skew * 0.35;
    let label = flag_label(flag);
    let pad = (inner_w * 0.10).max(h * 1.2).clamp(18.0, 36.0);
    let white_w = flag_caption_group_w(fonts, h, 1.0, label) + pad * 2.0;
    let inset = (h * 0.20).clamp(3.0, 5.0);
    let band = flag_caption_band_path(x, y, inner_w, h, skew, inset);
    draw_flag_caption_fade(px, band.as_ref().unwrap_or(&path), cx, y, inner_w, white_w, a, None);
    let ink = Color::from_rgba8(16, 16, 18, a.max(220));
    draw_flag_caption(px, fonts, cx, y, inner_w, h, 1.0, label, ink);
}

/// Caption white sits in a shorter band so cloth shows above and below.
fn flag_caption_band_path(x: f32, y: f32, inner_w: f32, h: f32, skew: f32, inset: f32) -> Option<Path> {
    let inset = inset.min((h - 4.0) * 0.5).max(1.0);
    let t0 = (inset / h).clamp(0.0, 0.4);
    let t1 = 1.0 - t0;
    let left = |t: f32| x + skew * (1.0 - t);
    let right = |t: f32| x + inner_w + skew * (1.0 - t);
    let mut pb = PathBuilder::new();
    pb.move_to(left(t0), y + inset);
    pb.line_to(right(t0), y + inset);
    pb.line_to(right(t1), y + h - inset);
    pb.line_to(left(t1), y + h - inset);
    pb.close();
    pb.finish()
}

fn flag_label(flag: DashFlag) -> &'static str {
    match flag {
        DashFlag::White => "WHITE FLAG",
        DashFlag::Yellow => "YELLOW FLAG",
        DashFlag::Blue => "BLUE FLAG",
        _ => "Checkered Flag",
    }
}

fn paint_flag_fill(px: &mut Pixmap, path: &Path, x: f32, y: f32, w: f32, h: f32, flag: DashFlag, a: u8) {
    let clip = Mask::new(px.width(), px.height()).map(|mut m| {
        m.fill_path(path, FillRule::Winding, true, Transform::identity());
        m
    });
    match flag {
        DashFlag::White => {
            fill_path(px, path, Color::from_rgba8(248, 248, 250, a));
            draw_diag_stripes_masked(
                px,
                x,
                y,
                w,
                h,
                Color::from_rgba8(210, 210, 214, a),
                clip.as_ref(),
            );
        }
        DashFlag::Yellow => fill_path(px, path, Color::from_rgba8(244, 214, 36, a)),
        DashFlag::Blue => fill_path(px, path, Color::from_rgba8(59, 130, 246, a)),
        _ => {
            fill_path(px, path, Color::from_rgba8(236, 236, 240, a));
            let rows = ((h / 16.0).round() as i32).clamp(3, 8);
            draw_checkered_cells(
                px,
                x,
                y,
                w,
                h,
                Color::from_rgba8(176, 176, 182, a),
                Color::from_rgba8(236, 236, 240, a),
                rows,
                clip.as_ref(),
            );
        }
    }
    stroke_path(
        px,
        path,
        Color::from_rgba8(16, 16, 18, ((a as u16 * 90) / 255) as u8),
        1.2,
    );
}

/// White behind the caption, same 5% edge fade as the Dash wrap banners.
fn draw_flag_caption_fade(
    px: &mut Pixmap,
    path: &Path,
    ox: f32,
    top_y: f32,
    ow: f32,
    white_w: f32,
    a: u8,
    clip: Option<&Mask>,
) {
    if ow <= 1.0 {
        return;
    }
    let fade = 0.05;
    let t0 = ((ow - white_w) * 0.5 / ow).clamp(fade + 0.02, 0.46);
    let on = Color::from_rgba8(248, 248, 250, a);
    let off = Color::from_rgba8(248, 248, 250, 0);
    let Some(shader) = LinearGradient::new(
        SkPoint::from_xy(ox, top_y),
        SkPoint::from_xy(ox + ow, top_y),
        vec![
            GradientStop::new(0.0, off),
            GradientStop::new((t0 - fade).max(0.0), off),
            GradientStop::new(t0, on),
            GradientStop::new(1.0 - t0, on),
            GradientStop::new((1.0 - t0 + fade).min(1.0), off),
            GradientStop::new(1.0, off),
        ],
        SpreadMode::Pad,
        Transform::identity(),
    ) else {
        return;
    };
    let mut paint = Paint::default();
    paint.shader = shader;
    paint.anti_alias = true;
    px.fill_path(path, &paint, FillRule::Winding, Transform::identity(), clip);
}

fn draw_sys(px: &mut Pixmap, fonts: &Fonts, cfg: &HudConfig, sw: f32, sh: f32) {
    // THESIS: CPU and MEM are two boards so process load is a glance, not a 12-row scroll.
    // OWN-WORLD: night-ink 6px plaque, hairline split, huge ExtraBold Italic numbers, gold heat tracks (red when hot). No green, no orange.
    // STORY: rider glances — is the machine the problem, and is it the HUD or the game.
    // FIRST VIEWPORT: tall plaque, CPU left / MEM right, four process rows under each, FPS 91 and NET 11% in a footer.
    // FORM: Twin Columns, user-locked sys-twin.png.
    // FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
    let r = cfg.sys;
    let x = r.x * sw;
    let y = r.y * sh;
    let w = r.w * sw;
    let h = r.h * sh;
    if w < 88.0 || h < 72.0 {
        return;
    }
    let a = bg_a(cfg.sys_bg);
    fill_round(px, x, y, w, h, 6.0, Color::from_rgba8(10, 10, 10, a));
    if a > 0 {
        if let Some(frame) = round_rect_path(x + 0.5, y + 0.5, w - 1.0, h - 1.0, 5.5) {
            let edge = ((a as u16 * 170) / 255).max(70) as u8;
            stroke_path(px, &frame, Color::from_rgba8(42, 42, 46, edge), 1.0);
        }
    }

    let (cpu, mem, fps, net) = sys_stats();
    let procs = sys_procs();
    let mem_scale = procs
        .iter()
        .filter(|p| p.on)
        .map(|p| p.mem_mb)
        .fold(0.0f32, f32::max)
        .max(1.0);

    let pad = (w * 0.045).clamp(8.0, 12.0);
    let gap = 1.0;
    let inner_w = (w - pad * 2.0).max(40.0);
    let inner_h = (h - pad * 2.0).max(56.0);
    let col_w = ((inner_w - gap) * 0.5).max(36.0);
    let foot_h = (inner_h * 0.30).clamp(36.0, 68.0).min(inner_h - 48.0);
    let body_h = (inner_h - foot_h - 1.0).max(40.0);
    let mid = x + pad + col_w + gap * 0.5;
    let hair = Color::from_rgba8(42, 42, 46, a.max(90));
    if let Some(line) = rr(mid.floor(), y + pad, 1.0, body_h + foot_h + 1.0) {
        fill_rect(px, line, hair);
    }
    let foot_y = y + pad + body_h;
    if let Some(line) = rr(x + pad, foot_y.floor(), inner_w, 1.0) {
        fill_rect(px, line, hair);
    }

    let label_fs = (body_h * 0.09).clamp(8.0, 11.0);
    let value_fs = (body_h * 0.22).clamp(18.0, 34.0);
    let proc_fs = (body_h * 0.075).clamp(8.0, 11.0);
    let track_h = (body_h * 0.028).clamp(3.0, 5.0);
    let cpu_s = format!("{:.0}%", cpu.round());
    let mem_s = format!("{:.0}%", mem.round());
    let fps_s = format!("{:.0}", fps.round());
    let net_s = format!("{:.0}%", net.round());
    let fps_fill = (fps / 1.2).clamp(0.0, 100.0);

    let left_x = x + pad;
    let right_x = mid + 1.0 + gap * 0.5;
    let head_y = y + pad * 0.35 + 4.0;
    let after_l = draw_sys_meter(
        px,
        fonts,
        left_x,
        head_y,
        col_w,
        "CPU",
        &cpu_s,
        label_fs,
        value_fs,
        cpu.clamp(0.0, 100.0),
        sys_heat(cpu),
        track_h,
    );
    let after_r = draw_sys_meter(
        px,
        fonts,
        right_x,
        head_y,
        col_w,
        "MEM",
        &mem_s,
        label_fs,
        value_fs,
        mem.clamp(0.0, 100.0),
        sys_heat(mem),
        track_h,
    );
    let proc_top = after_l.max(after_r) + (body_h * 0.04).clamp(4.0, 8.0);
    let proc_bot = y + pad + body_h - 4.0;
    let proc_h = (proc_bot - proc_top).max(28.0);
    let row_h = proc_h / SYS_PROC_N as f32;
    let label_w = SYS_PROC_LABELS
        .iter()
        .map(|s| measure(fonts, s, proc_fs))
        .fold(0.0f32, f32::max)
        + 4.0;
    draw_sys_procs(
        px, fonts, left_x, proc_top, col_w, row_h, proc_fs, label_w, &procs, false, 1.0,
    );
    draw_sys_procs(
        px, fonts, right_x, proc_top, col_w, row_h, proc_fs, label_w, &procs, true, mem_scale,
    );

    let foot_label = (foot_h * 0.18).clamp(8.0, 11.0);
    let foot_value = (foot_h * 0.38).clamp(16.0, 28.0);
    let foot_track = (foot_h * 0.06).clamp(3.0, 5.0);
    let fy = foot_y + (foot_h * 0.12).clamp(4.0, 8.0);
    draw_sys_meter(
        px,
        fonts,
        left_x,
        fy,
        col_w,
        "FPS",
        &fps_s,
        foot_label,
        foot_value,
        fps_fill,
        sys_fps_fill(fps_fill),
        foot_track,
    );
    draw_sys_meter(
        px,
        fonts,
        right_x,
        fy,
        col_w,
        "NET",
        &net_s,
        foot_label,
        foot_value,
        net.clamp(0.0, 100.0),
        sys_heat(net),
        foot_track,
    );
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

    let race = RaceStore::get();
    // Cards follow the live order, so a pass slides the field now, not at the line.
    let mut board = race.field.board();
    if board.is_empty() {
        let count = (s.standing_count.max(0) as usize).min(MAX_SAFE);
        board.extend_from_slice(&s.standings[..count]);
    }
    let n = board.len().min(MAX_SAFE);
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
    let fi = race
        .field
        .focus
        .filter(|i| *i < n)
        .or_else(|| (0..n).find(|&i| board[i].race_num == focus))
        .unwrap_or(0);
    let Some(focus_row) = board.get(fi) else {
        return;
    };
    let best_ms = if race.field.session_best_ms > 0 {
        race.field.session_best_ms
    } else {
        session_best_ms(s)
    };
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
        for (i, card) in board.iter().enumerate().take(n) {
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
                card,
                focus_row,
                best_ms,
                x,
                0.0,
                card_w,
                card_h,
                k,
            );
            push_click_rider(card.race_num, cards_x + x, card_y, card_w, card_h);
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
    let label = ticker_meta_label(field, &val).to_string();
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
        fill_round(px, x, y, w, h, 3.0, you_row_bg(100));
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
        RaceStore::get()
            .field
            .row_by_num(row.race_num)
            .map(ticker_delta_from_row)
            .unwrap_or_else(|| ticker_delta(focus, row))
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

/// Amber for the lapped tag: reads as a warning without competing with the orange position.
fn dash_lapped_col() -> Color { Color::from_rgba8(226, 186, 74, 255) }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DashFlag {
    None,
    White,
    Checkered,
    Yellow,
    Blue,
}

struct FlagAnim {
    kind: DashFlag,
    t0: f32,
    hiding: bool,
    /// When `wanted` first went None; hide only after a short hold so one-frame
    /// flicker on the run-in cannot slam the banner shut.
    none_since: f32,
}

static FLAG_ANIM: Mutex<FlagAnim> = Mutex::new(FlagAnim {
    kind: DashFlag::None,
    t0: 0.0,
    hiding: false,
    none_since: -1.0,
});

fn flag_anim_step(wanted: DashFlag) -> (DashFlag, f32) {
    const IN: f32 = 0.36;
    const OUT: f32 = 0.20;
    const HIDE_HOLD: f32 = 0.14;
    let now = anim_now();
    let mut st = FLAG_ANIM.lock().unwrap_or_else(|e| e.into_inner());
    if wanted != DashFlag::None {
        st.none_since = -1.0;
        if st.kind == wanted {
            // Same flag coming back after a brief None: stay open. Replaying the grow
            // is what made the checkered collapse as you crossed the line.
            if st.hiding {
                st.hiding = false;
                st.t0 = now - IN;
            }
        } else {
            st.kind = wanted;
            st.t0 = now;
            st.hiding = false;
        }
        let t = ease_out_cubic(((now - st.t0) / IN).clamp(0.0, 1.0));
        (st.kind, t)
    } else if st.kind != DashFlag::None {
        if st.none_since < 0.0 {
            st.none_since = now;
        }
        // Hold the open flag briefly so approach jitter cannot flap it.
        if !st.hiding && now - st.none_since < HIDE_HOLD {
            return (st.kind, 1.0);
        }
        if !st.hiding {
            st.hiding = true;
            st.t0 = now;
        }
        let t = 1.0 - ease_out_cubic(((now - st.t0) / OUT).clamp(0.0, 1.0));
        if t <= 0.02 {
            st.kind = DashFlag::None;
            st.hiding = false;
            st.none_since = -1.0;
            (DashFlag::None, 0.0)
        } else {
            (st.kind, t)
        }
    } else {
        st.none_since = -1.0;
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
    simple: bool,
    gear: String,
    rpm: String,
    speed: String,
    speed_label: &'static str,
    ptxt: String,
    lap_txt: String,
    lapped: bool,
    tag_sz: f32,
    foot: Vec<(char, String)>,
}

/// Shown beside the lap/clock text when you are a lap or more down.
const LAPPED_TAG: &str = "~Lapped";
const LAPPED_GAP: f32 = 6.0;

fn max_digit_w(fonts: &Fonts, size: f32) -> f32 {
    ('0'..='9').map(|d| measure(fonts, d.encode_utf8(&mut [0; 4]), size)).fold(0.0, f32::max)
}

fn dash_layout(fonts: &Fonts, s: &Snapshot, cfg: &HudConfig, sw: f32, sh: f32, flag: DashFlag, grow: f32) -> DashLay {
    let x0 = cfg.dash.x * sw;
    let y = cfg.dash.y * sh;
    if cfg.dash_simple {
        return dash_layout_simple(fonts, s, cfg, sw, sh, x0, y, flag, grow);
    }

    let gear = if s.local_gear <= 0 {
        "N".into()
    } else {
        format!("{}", s.local_gear)
    };
    let speed = cfg.units.format_speed(s.local_speed);
    let speed_label = cfg.units.speed_label();
    let rpm_n = s.local_rpm.max(0);
    let rpm = format!("{rpm_n}");
    let focus_num = if s.focus_race_num > 0 {
        s.focus_race_num
    } else {
        s.local_race_num
    };
    let pos = Some(standing_pos(s, focus_num)).filter(|p| *p > 0);
    let ptxt = pos.map(|p| format!("P{p}")).unwrap_or_else(|| "P--".into());
    let lap_txt = race_progress_text(s);
    let lapped = lapped(s);
    let foot: Vec<(char, String)> = [cfg.dash_left, cfg.dash_mid, cfg.dash_right]
        .into_iter()
        .filter_map(|field| dash_foot_item(s, cfg, field))
        .collect();

    const NAT_H: f32 = 147.0;
    const NAT_W: f32 = 280.0;
    let slot_w = (cfg.dash.w * sw).max(1.0);
    let slot_h = (cfg.dash.h * sh).max(1.0);
    let k = (slot_h / NAT_H).min(slot_w / NAT_W).clamp(0.35, 2.4);
    let pad = 15.0 * k;
    let foot_pad = 5.0 * k;
    let has_foot = !foot.is_empty();
    let footer_h = if has_foot { 23.0 * k } else { 0.0 };
    let mid_gap = if has_foot { 5.0 * k } else { 0.0 };
    let (rev_h, rev_lead, rev_to_main) = if cfg.dash_rev {
        (16.0 * k, 5.0 * k, 9.0 * k)
    } else {
        (0.0, pad, 0.0)
    };
    let h = slot_h;
    let w = slot_w;
    let x = x0;
    let chrome = rev_lead + rev_h + rev_to_main + mid_gap + footer_h + foot_pad;
    let main_h = (h - chrome).max(28.0 * k);
    let flag_full = if flag != DashFlag::None {
        (h * 0.22).clamp(20.0 * k, 28.0 * k)
    } else {
        0.0
    };
    let flag_h = flag_full * grow;
    let rev_y = y + rev_lead;
    let main_y = if cfg.dash_rev {
        rev_y + rev_h + rev_to_main
    } else {
        y + pad
    };
    let label = 14.0 * k;
    let gear_n = (main_h * 0.58).clamp(24.0 * k, 48.0 * k);
    let val = (main_h * 0.26).clamp(15.0 * k, 22.0 * k);
    let pos_n = (main_h * 0.44).clamp(22.0 * k, 38.0 * k);
    let lap_sz = (val * 0.88).max(13.0 * k);
    let icon_s = if has_foot { 15.0 * k } else { 0.0 };
    let fsz = if has_foot { 14.0 * k } else { 0.0 };
    let tag_sz = (lap_sz * 0.82).max(12.0 * k);
    let lap_row_w = measure(fonts, &lap_txt, lap_sz)
        + if lapped {
            LAPPED_GAP + measure(fonts, LAPPED_TAG, tag_sz)
        } else {
            0.0
        };

    let gear_w = (main_h * 0.82).clamp(52.0 * k, 70.0 * k);
    let digit = max_digit_w(fonts, val);
    let mid_w = (measure(fonts, "RPM", label) + 8.0 * k + digit * 5.0)
        .max(measure(fonts, speed_label, label) + 8.0 * k + digit * 3.0);
    let right_w = measure(fonts, &ptxt, pos_n).max(lap_row_w);
    let base_gap = 18.0 * k;
    let extra = (w - pad * 2.0 - gear_w - mid_w - right_w).max(0.0);
    let col_gap = (extra / 2.0).max(base_gap * 0.4);
    let col_origin = x + pad + ((w - pad * 2.0 - gear_w - mid_w - right_w - col_gap * 2.0) * 0.5).max(0.0);
    let gear_x = col_origin;
    let mid_x = gear_x + gear_w + col_gap;
    let right_x = mid_x + mid_w + col_gap;

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
        cut: (h * 0.14).clamp(8.0, 14.0),
        simple: false,
        gear,
        rpm,
        speed,
        speed_label,
        ptxt,
        lap_txt,
        lapped,
        tag_sz,
        foot,
    }
}

fn dash_layout_simple(
    fonts: &Fonts,
    s: &Snapshot,
    cfg: &HudConfig,
    sw: f32,
    sh: f32,
    x0: f32,
    y: f32,
    flag: DashFlag,
    grow: f32,
) -> DashLay {
    const NAT_H: f32 = 92.0;
    const NAT_W: f32 = 220.0;
    let k = ((cfg.dash.h * sh) / NAT_H)
        .min((cfg.dash.w * sw) / NAT_W)
        .clamp(0.35, 2.4);
    let pad = 10.0 * k;
    let main_h = (cfg.dash.h * sh - pad * 2.0).max(28.0 * k);
    let h = cfg.dash.h * sh;
    let w = cfg.dash.w * sw;
    let x = x0;
    let flag_full = if flag != DashFlag::None {
        (h * 0.22).clamp(20.0 * k, 28.0 * k)
    } else {
        0.0
    };
    let flag_h = flag_full * grow;
    let cut = (h * 0.14).clamp(8.0 * k, 14.0 * k);
    let skew = (cut * 1.1).max(6.0 * k);
    let main_y = y + pad;
    let gear_n = (main_h * 0.78).clamp(32.0 * k, 56.0 * k);
    let val = (main_h * 0.62).clamp(28.0 * k, 46.0 * k);
    let unit_sz = (val * 0.22).clamp(8.0 * k, 13.0 * k);

    let gear = if s.local_gear <= 0 {
        "N".into()
    } else {
        format!("{}", s.local_gear)
    };
    let speed = cfg.units.format_speed(s.local_speed);
    let speed_label = cfg.units.speed_label();

    let gear_digit = measure(fonts, &gear, gear_n).max(max_digit_w(fonts, gear_n));
    let gear_w = (gear_digit + 18.0 * k + skew).clamp(56.0 * k, 110.0 * k);
    let speed_w = measure(fonts, &speed, val).max(max_digit_w(fonts, val) * 2.0);
    let gap = (h * 0.10).clamp(12.0 * k, 20.0 * k);
    let unit_gap = 8.0 * k;
    let gear_x = x + pad;
    let mid_x = gear_x + gear_w + gap;

    DashLay {
        x,
        y,
        w,
        h,
        flag,
        flag_h,
        flag_grow: grow,
        pad,
        foot_pad: 0.0,
        footer_h: 0.0,
        rev_x: x + pad,
        rev_y: y,
        rev_w: 0.0,
        rev_h: 0.0,
        gear_x,
        gear_w,
        main_y,
        main_h,
        mid_x,
        mid_w: speed_w,
        right_x: mid_x + speed_w + unit_gap,
        label: unit_sz,
        gear_n,
        val,
        pos_n: 0.0,
        lap_sz: 0.0,
        icon_s: 0.0,
        fsz: unit_sz,
        cut,
        simple: true,
        gear,
        rpm: String::new(),
        speed,
        speed_label,
        ptxt: String::new(),
        lap_txt: String::new(),
        lapped: false,
        tag_sz: 0.0,
        foot: Vec::new(),
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
    fill_path_rule(px, path, color, FillRule::Winding, None);
}

fn fill_path_rule(
    px: &mut Pixmap,
    path: &Path,
    color: Color,
    rule: FillRule,
    mask: Option<&Mask>,
) {
    let mut p = Paint::default();
    p.set_color(color);
    p.anti_alias = true;
    px.fill_path(path, &p, rule, Transform::identity(), mask);
}

fn push_rect(pb: &mut PathBuilder, x: f32, y: f32, w: f32, h: f32) {
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    pb.move_to(x, y);
    pb.line_to(x + w, y);
    pb.line_to(x + w, y + h);
    pb.line_to(x, y + h);
    pb.close();
}

fn push_chamfer(pb: &mut PathBuilder, x: f32, y: f32, w: f32, h: f32, cut: f32) {
    push_chamfer_tb(pb, x, y, w, h, cut, cut);
}

/// Chamfered rect with independent top / bottom corner cuts (flag wrap vs dash body).
fn push_chamfer_tb(pb: &mut PathBuilder, x: f32, y: f32, w: f32, h: f32, top_cut: f32, bot_cut: f32) {
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    let top = top_cut.min(w * 0.45).min(h * 0.45).max(0.0);
    let bot = bot_cut.min(w * 0.45).min(h * 0.45).max(0.0);
    if top <= 0.5 && bot <= 0.5 {
        push_rect(pb, x, y, w, h);
        return;
    }
    pb.move_to(x + top, y);
    pb.line_to(x + w - top, y);
    pb.line_to(x + w, y + top);
    pb.line_to(x + w, y + h - bot);
    pb.line_to(x + w - bot, y + h);
    pb.line_to(x + bot, y + h);
    pb.line_to(x, y + h - bot);
    pb.line_to(x, y + top);
    pb.close();
}

/// Top banner: chamfered top corners (same language as the dash), square bottom flush on the body.
fn dash_flag_top_path(x: f32, y: f32, w: f32, h: f32, cut: f32) -> Option<Path> {
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    let cut = cut.min(w * 0.45).min(h * 0.9).max(0.0);
    let mut pb = PathBuilder::new();
    if cut <= 0.5 {
        push_rect(&mut pb, x, y, w, h);
    } else {
        pb.move_to(x + cut, y);
        pb.line_to(x + w - cut, y);
        pb.line_to(x + w, y + cut);
        pb.line_to(x + w, y + h);
        pb.line_to(x, y + h);
        pb.line_to(x, y + cut);
        pb.close();
    }
    pb.finish()
}

/// Flag wrap sits on the *outside* of the dash so the body cannot cover it.
fn dash_wrap_border(d: &DashLay) -> f32 {
    if d.flag == DashFlag::None || d.flag_grow <= 0.02 {
        0.0
    } else {
        (6.0 * d.flag_grow).clamp(0.0, 6.0)
    }
}

fn dash_wrap_outer(d: &DashLay, border: f32) -> (f32, f32, f32, f32) {
    let top_h = d.flag_h.max(1.0);
    (
        d.x - border,
        d.y - top_h,
        d.w + border * 2.0,
        top_h + d.h + border,
    )
}

fn dash_wrap_frame_path(d: &DashLay, border: f32) -> Option<Path> {
    if border <= 0.5 {
        return None;
    }
    let (ox, oy, ow, oh) = dash_wrap_outer(d, border);
    let top_h = d.flag_h.max(1.0);
    // Match flag top corners; bottom follows the dash body chamfer (not a round blob).
    let top_cut = d.cut.min(top_h * 0.9);
    let bot_cut = d.cut;
    let mut pb = PathBuilder::new();
    push_chamfer_tb(&mut pb, ox, oy, ow, oh, top_cut, bot_cut);
    // Hole matches the dash body exactly so top corners meet with no gap.
    push_chamfer(&mut pb, d.x, d.y, d.w, d.h, d.cut);
    pb.finish()
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


fn dash_best_ms(s: &Snapshot) -> i32 {
    let race = RaceStore::get();
    let standing_best = race
        .field
        .focus
        .and_then(|i| race.field.rows.get(i))
        .map(|r| r.standing.best_lap_ms)
        .or_else(|| focus_standing(s).map(|st| st.best_lap_ms))
        .unwrap_or(0);
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
    let race = RaceStore::get();
    let st = race
        .field
        .focus
        .and_then(|i| race.field.rows.get(i))
        .map(|r| &r.standing)
        .or_else(|| focus_standing(s));
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
        DashField::LapCount => race_progress_text(s),
        DashField::LapsLeft => race_laps_left_text(s),
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
        DashField::Interval => st
            .and_then(|r| {
                race.field
                    .row_by_num(r.race_num)
                    .map(interval_text_from_row)
                    .or_else(|| Some(interval_text(s, r)))
            })
            .unwrap_or_else(|| "--".into()),
        DashField::Penalty => format_penalty(st.map(|r| r.penalty_ms).unwrap_or(0)),
        DashField::Session => race_progress_text(s),
        DashField::LocalTime => local_clock(),
        DashField::Bike => st
            .map(|r| cstr(&r.bike))
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "--".into()),
        DashField::Class => st
            .map(|r| cstr(&r.category))
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "--".into()),
        DashField::Fuel => cfg.units.format_fuel(s.fuel, s.max_fuel),
        DashField::FuelPct => format_fuel_pct(s.fuel, s.max_fuel),
    };
    Some((field.icon(), text))
}

fn draw_diag_stripes_masked(
    px: &mut Pixmap,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    stripe: Color,
    mask: Option<&Mask>,
) {
    if w <= 0.0 || h <= 0.0 {
        return;
    }
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
            fill_path_rule(px, &path, stripe, FillRule::Winding, mask);
        }
        t += step;
    }
}

fn fill_cell(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, c: Color, mask: Option<&Mask>) {
    if w <= 0.5 || h <= 0.5 {
        return;
    }
    let mut pb = PathBuilder::new();
    push_rect(&mut pb, x, y, w, h);
    if let Some(path) = pb.finish() {
        fill_path_rule(px, &path, c, FillRule::Winding, mask);
    }
}

fn flag_icon_w(fonts: &Fonts, size: f32) -> f32 {
    fonts.icons.metrics('\u{f024}', size * style_k()).advance_width
}

/// Icon + label, centered. Returns the caption width so the checker fade can hug it.
fn draw_flag_caption(
    px: &mut Pixmap,
    fonts: &Fonts,
    ox: f32,
    top_y: f32,
    ow: f32,
    top_h: f32,
    grow: f32,
    label: &str,
    ink: Color,
) -> f32 {
    let sz = (top_h * 0.44).clamp(10.0, 13.0) * (0.75 + 0.25 * grow);
    let icon_sz = (sz * 1.12).clamp(11.0, 16.0);
    let gap = (icon_sz * 0.28).clamp(4.0, 7.0);
    let iw = flag_icon_w(fonts, icon_sz);
    let tw = measure_bold(fonts, label, sz);
    let group = iw + gap + tw;
    let x0 = ox + (ow - group) * 0.5;
    let ty = top_y + (top_h - sz) * 0.42;
    icon(
        px,
        fonts,
        '\u{f024}',
        icon_sz,
        x0,
        ty + (sz - icon_sz) * 0.5,
        ink,
        false,
    );
    text_bold(px, fonts, label, sz, x0 + iw + gap, ty, ink, false);
    group
}

fn flag_caption_group_w(fonts: &Fonts, top_h: f32, grow: f32, label: &str) -> f32 {
    let sz = (top_h * 0.44).clamp(10.0, 13.0) * (0.75 + 0.25 * grow);
    let icon_sz = (sz * 1.12).clamp(11.0, 16.0);
    let gap = (icon_sz * 0.28).clamp(4.0, 7.0);
    flag_icon_w(fonts, icon_sz) + gap + measure_bold(fonts, label, sz)
}

fn draw_checkered_masked(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, mask: Option<&Mask>) {
    draw_checkered_cells(
        px,
        x,
        y,
        w,
        h,
        Color::from_rgba8(176, 176, 182, 255),
        Color::from_rgba8(236, 236, 240, 255),
        if h < 12.0 { 2 } else { 3 },
        mask,
    );
}

fn draw_checkered_cells(
    px: &mut Pixmap,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    dark: Color,
    light: Color,
    rows: i32,
    mask: Option<&Mask>,
) {
    if w <= 0.0 || h <= 0.0 || rows < 1 {
        return;
    }
    let cell_h = h / rows as f32;
    let cols = ((w / cell_h).round() as i32).max(1);
    let cell_w = w / cols as f32;
    for row in 0..rows {
        for col in 0..cols {
            let c = if (row + col) % 2 == 0 { dark } else { light };
            fill_cell(
                px,
                x + col as f32 * cell_w,
                y + row as f32 * cell_h,
                cell_w + 0.4,
                cell_h + 0.4,
                c,
                mask,
            );
        }
    }
}

/// Sides and bottom of the dash wrap. One square wide, clipped to the frame so it
/// cannot paint into the body. The top banner covers the rest.
fn draw_checkered_wrap(px: &mut Pixmap, d: &DashLay, border: f32, ox: f32, ow: f32) {
    let Some(frame) = dash_wrap_frame_path(d, border) else {
        return;
    };
    fill_path_rule(px, &frame, Color::from_rgba8(248, 248, 250, 255), FillRule::EvenOdd, None);
    let clip = Mask::new(px.width(), px.height()).map(|mut m| {
        m.fill_path(&frame, FillRule::EvenOdd, true, Transform::identity());
        m
    });
    let dark = Color::from_rgba8(176, 176, 182, 255);
    let light = Color::from_rgba8(236, 236, 240, 255);
    let side_rows = ((d.h / border).round() as i32).max(1);
    draw_checkered_cells(px, ox, d.y, border, d.h, dark, light, side_rows, clip.as_ref());
    draw_checkered_cells(px, d.x + d.w, d.y, border, d.h, dark, light, side_rows, clip.as_ref());
    draw_checkered_cells(px, ox, d.y + d.h, ow, border, dark, light, 1, clip.as_ref());
}

fn draw_checkered_banner(px: &mut Pixmap, fonts: &Fonts, band: &Path, ox: f32, top_y: f32, ow: f32, top_h: f32, grow: f32) {
    let white = Color::from_rgba8(248, 248, 250, 255);
    let ink = Color::from_rgba8(22, 22, 26, 255);
    let label = "Checkered Flag";
    fill_path(px, band, white);
    let clip = Mask::new(px.width(), px.height()).map(|mut m| {
        m.fill_path(band, FillRule::Winding, true, Transform::identity());
        m
    });
    draw_checkered_masked(px, ox, top_y, ow, top_h, clip.as_ref());
    let pad = (top_h * 0.22).clamp(5.0, 8.0);
    let white_w = flag_caption_group_w(fonts, top_h, grow.max(0.42), label) + pad * 2.0;
    draw_flag_caption_fade(px, band, ox, top_y, ow, white_w, 255, clip.as_ref());
    if grow > 0.42 {
        draw_flag_caption(px, fonts, ox, top_y, ow, top_h, grow, label, ink);
    }
}

/// Sides and bottom of the dash wrap for white flag — diagonal stripes, same frame as checkered.
fn draw_white_wrap(px: &mut Pixmap, d: &DashLay, border: f32, ox: f32, ow: f32) {
    let Some(frame) = dash_wrap_frame_path(d, border) else {
        return;
    };
    let bg = Color::from_rgba8(248, 248, 250, 255);
    let stripe = Color::from_rgba8(210, 210, 214, 255);
    fill_path_rule(px, &frame, bg, FillRule::EvenOdd, None);
    let clip = Mask::new(px.width(), px.height()).map(|mut m| {
        m.fill_path(&frame, FillRule::EvenOdd, true, Transform::identity());
        m
    });
    draw_diag_stripes_masked(px, ox, d.y, border, d.h, stripe, clip.as_ref());
    draw_diag_stripes_masked(px, d.x + d.w, d.y, border, d.h, stripe, clip.as_ref());
    draw_diag_stripes_masked(px, ox, d.y + d.h, ow, border, stripe, clip.as_ref());
}

/// Top banner: stripes across most of the band, fading to a white plaque behind the caption.
fn draw_white_banner(px: &mut Pixmap, fonts: &Fonts, band: &Path, ox: f32, top_y: f32, ow: f32, top_h: f32, grow: f32) {
    let white = Color::from_rgba8(248, 248, 250, 255);
    let stripe = Color::from_rgba8(210, 210, 214, 255);
    let ink = Color::from_rgba8(16, 16, 18, 255);
    let label = "WHITE FLAG";
    fill_path(px, band, white);
    let clip = Mask::new(px.width(), px.height()).map(|mut m| {
        m.fill_path(band, FillRule::Winding, true, Transform::identity());
        m
    });
    draw_diag_stripes_masked(px, ox, top_y, ow, top_h, stripe, clip.as_ref());
    let pad = (top_h * 0.22).clamp(5.0, 8.0);
    let white_w = flag_caption_group_w(fonts, top_h, grow.max(0.42), label) + pad * 2.0;
    draw_flag_caption_fade(px, band, ox, top_y, ow, white_w, 255, clip.as_ref());
    if grow > 0.42 {
        draw_flag_caption(px, fonts, ox, top_y, ow, top_h, grow, label, ink);
    }
}

fn draw_dash_wrap(px: &mut Pixmap, fonts: &Fonts, d: &DashLay) {
    if d.flag == DashFlag::None || d.flag_grow <= 0.02 {
        return;
    }
    let grow = d.flag_grow;
    let border = dash_wrap_border(d);
    let top_h = d.flag_h.max(2.0);
    let (ox, _oy, ow, _) = dash_wrap_outer(d, border);
    let top_y = d.y - top_h;
    let top_cut = d.cut.min(top_h * 0.9);

    let Some(band) = dash_flag_top_path(ox, top_y, ow, top_h + 0.75, top_cut) else {
        return;
    };
    if d.flag == DashFlag::White {
        draw_white_wrap(px, d, border, ox, ow);
        draw_white_banner(px, fonts, &band, ox, top_y, ow, top_h, grow);
        return;
    }
    if d.flag != DashFlag::Checkered {
        return;
    }

    draw_checkered_wrap(px, d, border, ox, ow);
    draw_checkered_banner(px, fonts, &band, ox, top_y, ow, top_h, grow);
}

fn draw_unit_stack(
    px: &mut Pixmap,
    fonts: &Fonts,
    label: &str,
    size: f32,
    x: f32,
    y: f32,
    h: f32,
    color: Color,
) {
    let n = label.chars().count().max(1) as f32;
    let step = (h / n).min(size * 1.05);
    let stack_h = step * n;
    let mut cy = y + (h - stack_h) * 0.5;
    let mut buf = [0u8; 4];
    for ch in label.chars() {
        text(px, fonts, ch.encode_utf8(&mut buf), size, x, cy, color, true);
        cy += step;
    }
}

fn draw_simple_dash(px: &mut Pixmap, fonts: &Fonts, d: &DashLay, a: u8) {
    if let Some(path) = chamfer_path(d.x, d.y, d.w, d.h, d.cut) {
        if a > 0 {
            fill_path(px, &path, Color::from_rgba8(18, 18, 20, a));
        }
        if (d.flag == DashFlag::None || d.flag_grow <= 0.02) && a > 40 {
            stroke_path(
                px,
                &path,
                Color::from_rgba8(220, 220, 224, ((a as u16 * 200) / 255).max(90) as u8),
                1.4,
            );
        }
    }

    let skew = (d.cut * 1.1).max(6.0);
    let tile_a = a.max(220);
    fill_skew(
        px,
        d.gear_x,
        d.main_y,
        (d.gear_w - skew).max(28.0),
        d.main_h,
        skew,
        Color::from_rgba8(255, 148, 48, tile_a),
    );
    let ink = ink_on(accent());
    text_bold(
        px,
        fonts,
        &d.gear,
        d.gear_n,
        d.gear_x + d.gear_w * 0.5,
        d.main_y + (d.main_h - d.gear_n) * 0.45,
        ink,
        true,
    );

    let white = Color::from_rgba8(248, 248, 250, 255);
    let unit = Color::from_rgba8(176, 176, 184, 255);
    let speed_y = d.main_y + (d.main_h - d.val) * 0.38;
    text_bold(px, fonts, &d.speed, d.val, d.mid_x, speed_y, white, false);
    draw_unit_stack(
        px,
        fonts,
        d.speed_label,
        d.fsz,
        d.right_x + d.fsz * 0.45,
        d.main_y,
        d.main_h,
        unit,
    );

    draw_dash_wrap(px, fonts, d);
}

fn draw_dash(px: &mut Pixmap, fonts: &Fonts, s: &Snapshot, cfg: &HudConfig, sw: f32, sh: f32, flag: DashFlag, grow: f32) {
    let d = dash_layout(fonts, s, cfg, sw, sh, flag, grow);
    if d.simple {
        draw_simple_dash(px, fonts, &d, bg_a(cfg.dash_bg));
        return;
    }
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
        if d.flag == DashFlag::None || d.flag_grow <= 0.02 {
            stroke_path(px, &path, Color::from_rgba8(220, 220, 224, ((a as u16 * 200) / 255).max(90) as u8), 1.4);
        }
    }

    if cfg.dash_rev {
        draw_rev_bar(px, d.rev_x, d.rev_y, d.rev_w, d.rev_h, s.local_rpm, s.max_rpm, s.shift_rpm);
    }

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
    let lap_y = d.main_y + d.main_h * 0.68;
    text(px, fonts, &d.lap_txt, d.lap_sz, d.right_x, lap_y, white, false);
    if d.lapped {
        // Baselines differ with the smaller size, so nudge down to sit on the lap text.
        let tag_x = d.right_x + measure(fonts, &d.lap_txt, d.lap_sz) + LAPPED_GAP;
        let tag_y = lap_y + (d.lap_sz - d.tag_sz) * 0.72;
        text(px, fonts, LAPPED_TAG, d.tag_sz, tag_x, tag_y, dash_lapped_col(), false);
    }

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

    // Last: wrap sits on the outer edge so the body cannot cover it.
    draw_dash_wrap(px, fonts, &d);
}

fn draw_relative(px: &mut Pixmap, fonts: &Fonts, s: &Snapshot, cfg: &HudConfig, sw: f32, sh: f32) {
    let r = s.relative;
    let x = r.x * sw;
    let y = r.y * sh;
    let max_w = r.w * sw;
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
    let cols = cfg.relative_cols();
    let pad = 8.0;
    let slots = col_slots(x, pad, max_w, &cols, |c| c.width(cfg) as f32, |c| matches!(c, RelField::Name));
    let w = hug_board_w(x, pad, max_w, &slots);
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

    let (ink, ink_dim, out_c, hdr_c) = table_ink(cfg.rel_text);
    let bar_end = slots
        .iter()
        .find(|(c, _, _)| *c == RelField::Pos)
        .map(|(_, cx, cw)| bike_bar_end(*cx, *cw));

    let purple = Color::from_rgba8(196, 112, 255, 255);
    let race = RaceStore::get();
    let best_ms = if race.field.session_best_ms > 0 {
        race.field.session_best_ms
    } else {
        s.standings
            .iter()
            .take(s.standing_count.max(0) as usize)
            .map(|row| row.best_lap_ms)
            .filter(|ms| *ms > 0)
            .min()
            .unwrap_or(0)
    };
    let you_bg = you_row_bg(cfg.rel_hl);
    let stripe_c = stripe_row_bg(a);

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
        if cfg.rel_stripe && vis_i % 2 == 1 && a > 0 {
            fill_focus_row(px, x, body_y + vis_i as f32 * row_h, w, row_h, stripe_c);
        }
    }
    for (vis_i, oi) in show.iter().enumerate() {
        let cy = row_ys[vis_i];
        let (ri, wrapped) = order[*oi];
        let rider = &s.riders[ri];
        let is_self = rider.race_num == focus;
        let race_row = race.field.row_by_num(rider.race_num);
        let st = race_row.map(|r| &r.standing).or_else(|| standing_of(s, rider.race_num));
        let cat = st.map(|r| cstr(&r.category)).unwrap_or_default();
        let bike_name = st.map(|r| cstr(&r.bike)).unwrap_or_default();
        let accent_c = bike_color(&bike_name, &cat);
        let out = rider.crashed != 0
            || st.is_some_and(|r| r.crashed != 0 || matches!(r.state, 1 | 3 | 4));
        if is_self {
            fill_focus_row(px, x, cy, w, row_h, you_bg);
        } else if let Some(bg) = lap_row_bg(lap_rel(s, rider.race_num), cfg.rel_hl) {
            fill_focus_row(px, x, cy, w, row_h, bg);
        }

        let name_c = if out { out_c } else { ink };
        let dim = if out { out_c } else { ink_dim };
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
                    format!(
                        "{}",
                        race_row
                            .map(|r| r.current_lap)
                            .unwrap_or_else(|| {
                                rider_current_lap(s, rider.race_num, st.map(|r| r.num_laps).unwrap_or(0))
                            })
                    ),
                    dim,
                    true,
                ),
                RelField::Penalty => (st.map(|r| format_penalty(r.penalty_ms)).unwrap_or_default(), dim, true),
                RelField::Interval => (
                    race_row
                        .map(interval_text_from_row)
                        .or_else(|| st.map(|r| interval_text(s, r)))
                        .unwrap_or_default(),
                    dim,
                    true,
                ),
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
                draw_bike_pill(px, fonts, &val, *cx, cy, *cw, row_h, accent_c);
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

    if cfg.map_sectors {
        draw_sector_lines(px, fonts, s, n, to_px, track_px, None);
    }

    let subject = camera_subject(s);
    let you = subject_pose(s, age);
    let leader = leader_num(s);
    let other_r = (if cfg.map_numbers { 6.8 } else { 4.6 }) * style_k();

    if cfg.map_others {
        for i in 0..s.rider_count.max(0) as usize {
            let rider = &s.riders[i];
            if you.is_some() && rider.race_num == subject {
                continue;
            }
            let (hx, hy) = to_px(rider.x, rider.z);
            let fill = rider_dot_col(s, rider.race_num);
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
            draw_rider_overhead(px, fonts, s, rider.race_num, hx, hy, other_r, subject, leader, cfg.map_crown, cfg.map_place);
            draw_state_mark(px, fonts, hx, hy, other_r, rider_mark(s, rider.race_num, rider.crashed != 0));
        }
    }
    if let Some(pose) = you {
        let (hx, hy) = to_px(pose.x, pose.z);
        let local_r = 8.5 * style_k();
        draw_rider_dot(
            px,
            fonts,
            hx,
            hy,
            local_r,
            you_col(),
            rider_dot_num(s, subject, cfg.map_dot),
            cfg.map_numbers,
            true,
        );
        let (fwx, fwz) = if pose.from_local {
            local_forward(s)
        } else {
            yaw_forward(pose.yaw)
        };
        let (sdx, sdy) = screen_dir(&to_px, pose.x, pose.z, fwx, fwz);
        draw_dot_chevron(px, hx, hy, local_r, sdx, sdy, you_col(), true);
        if cfg.map_crown && leader > 0 && subject == leader {
            crown_over_dot(px, fonts, hx, hy, local_r);
        }
        draw_state_mark(px, fonts, hx, hy, local_r, rider_mark(s, subject, pose.crashed));
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

    let subject = camera_subject(s);
    let you = subject_pose(s, age);
    let (fx, fz, rx, rz, scale, origin_x, origin_z, north_up) = if let Some(pose) = you.as_ref() {
        let (fx, fz) = track_forward(s, n, pose.x, pose.z).unwrap_or_else(|| {
            if pose.from_local {
                let (f, z, _, _) = radar_axes(s);
                (f, z)
            } else {
                yaw_forward(pose.yaw)
            }
        });
        let radius_m = mini_view_radius(cfg.mini_zoom);
        (fx, fz, fz, -fx, (sdim * 0.46) / radius_m, pose.x, pose.z, true)
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
    if cfg.mini_sectors {
        draw_sector_lines(mini, fonts, s, n, to_px, track_px, Some((mc, mc, sdim * 0.46)));
    }
    if cfg.mini_arrows {
        draw_track_arrows(mini, s, n, to_px, track_px, Some((mc, sdim)), north_up);
    }

    let leader = leader_num(s);
    let other_r = (sdim * 0.028).clamp(7.0, 11.0) * style_k();
    let local_r = other_r * 1.22;

    if cfg.mini_others {
        for i in 0..s.rider_count.max(0) as usize {
            let rider = &s.riders[i];
            if you.is_some() && rider.race_num == subject {
                continue;
            }
            let (hx, hy) = to_px(rider.x, rider.z);
            if (hx - mc) * (hx - mc) + (hy - mc) * (hy - mc) > sdim * sdim * 0.27 {
                continue;
            }
            let fill = rider_dot_col(s, rider.race_num);
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
            draw_rider_overhead(mini, fonts, s, rider.race_num, hx, hy, other_r, subject, leader, cfg.mini_crown, cfg.mini_place);
            draw_state_mark(mini, fonts, hx, hy, other_r, rider_mark(s, rider.race_num, rider.crashed != 0));
        }
    }

    if let Some(pose) = you {
        let (hx, hy) = to_px(pose.x, pose.z);
        if pose.from_local {
            let vx = pose.vel_x;
            let vz = pose.vel_z;
            for i in (1..5).rev() {
                let t = i as f32 * 0.07;
                let (tx, ty) = to_px(pose.x - vx * t, pose.z - vz * t);
                let a = 40u8.saturating_mul(5 - i as u8);
                fill_circle(mini, tx, ty, local_r * (0.55 + i as f32 * 0.04), Color::from_rgba8(255, 148, 48, a));
            }
        }
        numbered_dot(
            mini,
            fonts,
            hx,
            hy,
            local_r,
            you_col(),
            rider_dot_num(s, subject, cfg.mini_dot),
            cfg.mini_numbers,
            true,
        );
        let (fwx, fwz) = if pose.from_local {
            local_forward(s)
        } else {
            yaw_forward(pose.yaw)
        };
        let (sdx, sdy) = screen_dir(&to_px, pose.x, pose.z, fwx, fwz);
        draw_dot_chevron(mini, hx, hy, local_r, sdx, sdy, you_col(), true);
        if cfg.mini_crown && leader > 0 && subject == leader {
            crown_over_dot(mini, fonts, hx, hy, local_r);
        }
        draw_state_mark(mini, fonts, hx, hy, local_r, rider_mark(s, subject, pose.crashed));
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

fn leader_num(s: &Snapshot) -> i32 {
    // Live rank first: the crown has to move with an on-track pass for the lead, not
    // wait for the game to republish its classification at the line.
    let live = live_leader();
    if live > 0 {
        return live;
    }
    s.standings
        .iter()
        .take(s.standing_count.max(0) as usize)
        .find(|st| st.position == 1)
        .map(|st| st.race_num)
        .unwrap_or(0)
}

fn standing_pos(s: &Snapshot, race_num: i32) -> i32 {
    let live = live_position(race_num);
    if live > 0 {
        return live;
    }
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
    let ink = ink_on(fill);
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

pub fn icon(px: &mut Pixmap, fonts: &Fonts, ch: char, size: f32, mut x: f32, y: f32, color: Color, center: bool) {
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

fn fill_night_pill(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32) {
    fill_round(px, x, y, w, h, h * 0.5, Color::from_rgba8(10, 10, 10, 220));
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
    (size * (0.020 + heat * 0.014)).clamp(7.0, 15.0)
}

fn radar_blip_color(heat: f32) -> Color {
    let r = 228.0 + 22.0 * heat;
    let g = 197.0 + (118.0 - 197.0) * heat;
    let b = 112.0 + (2.0 - 112.0) * heat;
    Color::from_rgba8(r as u8, g as u8, b as u8, 255)
}

fn draw_radar_blip(px: &mut Pixmap, x: f32, y: f32, rad: f32, heat: f32) {
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

fn radar_fit_scale(w: f32, h: f32, ox: f32, oy: f32, x: f32, y: f32, inset: f32) -> f32 {
    let to_side = (ox - x - inset).min(x + w - inset - ox).max(1.0);
    let to_bottom = (y + h - inset - oy).max(1.0);
    (to_side / RADAR_FWD_REAR)
        .min(to_bottom / RADAR_FWD_REAR)
        .max(0.5)
}

fn radar_ring_radius(meters: f32, scale: f32) -> f32 {
    meters * scale
}

fn radar_ring_lift(bg_pct: i32) -> f32 {
    // Night-ink at ≥80% eats a #38383C hairline. Lift the stroke as the plaque goes solid.
    ((bg_pct as f32 - 50.0) / 50.0).clamp(0.0, 1.0)
}

fn radar_ring_color(bg_pct: i32) -> Color {
    let t = radar_ring_lift(bg_pct);
    let v = 96.0 + 84.0 * t;
    Color::from_rgba8(v as u8, v as u8, (v + 6.0).min(255.0) as u8, (220.0 + 35.0 * t) as u8)
}

fn radar_ring_label_color(bg_pct: i32) -> Color {
    let t = radar_ring_lift(bg_pct);
    let v = 160.0 + 48.0 * t;
    Color::from_rgba8(v as u8, v as u8, (v + 6.0).min(255.0) as u8, 255)
}

fn radar_ring_stroke(size: f32, bg_pct: i32) -> f32 {
    let t = radar_ring_lift(bg_pct);
    (size * (0.0042 + 0.0022 * t)).clamp(1.15 + 0.25 * t, 1.35 + 0.55 * t)
}

fn stroke_circle_gap(
    px: &mut Pixmap,
    cx: f32,
    cy: f32,
    r: f32,
    gap_half: f32,
    color: Color,
    width: f32,
    clip: Option<&Mask>,
) {
    if r < 2.0 {
        return;
    }
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;
    let stroke = Stroke {
        width,
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Stroke::default()
    };
    if gap_half < 0.04 {
        let mut pb = PathBuilder::new();
        pb.push_circle(cx, cy, r);
        if let Some(path) = pb.finish() {
            px.stroke_path(&path, &paint, &stroke, Transform::identity(), clip);
        }
        return;
    }
    let tau = std::f32::consts::TAU;
    let down = std::f32::consts::FRAC_PI_2;
    let start = down + gap_half;
    let sweep = tau - 2.0 * gap_half;
    if sweep <= 0.2 {
        return;
    }
    let steps = (r * 0.7).clamp(28.0, 72.0) as i32;
    let mut pb = PathBuilder::new();
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let a = start + sweep * t;
        let x = cx + r * a.cos();
        let y = cy + r * a.sin();
        if i == 0 {
            pb.move_to(x, y);
        } else {
            pb.line_to(x, y);
        }
    }
    if let Some(path) = pb.finish() {
        px.stroke_path(&path, &paint, &stroke, Transform::identity(), clip);
    }
}

fn draw_radar_range_rings(
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
    let labeled = [(6.0_f32, "6"), (12.0, "12")];
    for meters in [3.0_f32, 6.0, 12.0] {
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

fn pad_from_size(size: f32) -> f32 {
    (size * 0.10).max(8.0)
}

fn draw_radar(px: &mut Pixmap, fonts: &Fonts, s: &Snapshot, cfg: &HudConfig, sw: f32, sh: f32, age: f32) {
    // THESIS: distance is a graphic — hairline arcs on the bike, not empty glass.
    // OWN-WORLD: night-ink 6px plaque, hairline frame, white bike, heat blips, range circles that lift off a solid plaque.
    // STORY: rider glances behind and beside; 6 and 12 say how far without reading a table.
    // FIRST VIEWPORT: square plaque, bike in the upper third, three circles inside the glass, 6/12 in stroke gaps, blips on top.
    // FORM: Range Arcs, user-locked radar-arcs.png. No wedges, no sweep, no title bar.
    // FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
    let r = cfg.radar;
    let x = r.x * sw;
    let y = r.y * sh;
    let w = (r.w * sw).max(48.0);
    let h = (r.h * sh).max(48.0);
    let size = w.min(h);
    let pad = pad_from_size(size);

    let a = bg_a(cfg.radar_bg);
    if a > 0 {
        fill_round(px, x, y, w, h, 6.0, Color::from_rgba8(14, 14, 16, a));
        if let Some(frame) = round_rect_path(x + 0.5, y + 0.5, w - 1.0, h - 1.0, 5.5) {
            let edge = ((a as u16 * 170) / 255).max(70) as u8;
            stroke_path(px, &frame, Color::from_rgba8(58, 58, 62, edge), 1.0);
        }
    }

    let ox = x + w * 0.5;
    let usable_h = (h - pad * 2.0).max(16.0);
    let oy = y + pad + usable_h * radar_you_frac();
    let inset = (size * 0.028).max(4.0);
    let scale = radar_fit_scale(w, h, ox, oy, x, y, inset);

    if cfg.radar_rings {
        draw_radar_range_rings(px, fonts, ox, oy, scale, x, y, w, h, size, cfg.radar_bg);
    }

    let bw = (size * 0.075).max(6.5);
    let bh = (size * 0.185).max(13.0);
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
        if !radar_same_stretch(s, rider.track_pos, RADAR_STRETCH_M) {
            continue;
        }
        if !radar_in_view(fwd, lat, cfg.radar_sides, cfg.radar_rear) {
            continue;
        }
        let dist = (fwd * fwd + lat * lat).sqrt();
        blips.push((fwd, lat, dist, rider.race_num, rider.crashed != 0));
    }
    blips.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    for (fwd, lat, dist, race_num, crashed) in blips {
        let (bx, by) = radar_to_screen(fwd, lat, ox, oy, scale, scale);
        let heat = radar_blip_heat(dist);
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

struct PolyHit {
    wx: f32,
    wz: f32,
    ax: f32,
    az: f32,
    bx: f32,
    bz: f32,
}

fn poly_length(s: &Snapshot, n: usize) -> (f32, Vec<f32>) {
    let mut dist = 0.0f32;
    let mut d = vec![0.0; n];
    for i in 1..n {
        let dx = s.poly[i].x - s.poly[i - 1].x;
        let dz = s.poly[i].z - s.poly[i - 1].z;
        dist += (dx * dx + dz * dz).sqrt();
        d[i] = dist;
    }
    (dist, d)
}

fn along_poly(s: &Snapshot, n: usize, meters: f32) -> Option<PolyHit> {
    if n < 2 {
        return None;
    }
    let (dist, d) = poly_length(s, n);
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
    for i in 1..n {
        if d[i] < target {
            continue;
        }
        let span = d[i] - d[i - 1];
        let u = if span > 0.001 { (target - d[i - 1]) / span } else { 0.0 };
        let ax = s.poly[i - 1].x;
        let az = s.poly[i - 1].z;
        let bx = s.poly[i].x;
        let bz = s.poly[i].z;
        return Some(PolyHit {
            wx: ax + (bx - ax) * u,
            wz: az + (bz - az) * u,
            ax,
            az,
            bx,
            bz,
        });
    }
    None
}

fn poly_at_frac(s: &Snapshot, n: usize, frac: f32) -> Option<PolyHit> {
    let (dist, _) = poly_length(s, n);
    if dist < 1.0 {
        return None;
    }
    let lap = if s.track_length > 1.0 { s.track_length } else { dist };
    let origin = if s.sf_meters >= 0.0 { s.sf_meters } else { 0.0 };
    along_poly(s, n, origin + frac.rem_euclid(1.0) * lap)
}

fn poly_centroid(s: &Snapshot, n: usize) -> (f32, f32) {
    let mut cx = 0.0;
    let mut cz = 0.0;
    let n = n.max(1);
    for p in s.poly.iter().take(n) {
        cx += p.x;
        cz += p.z;
    }
    let inv = 1.0 / n as f32;
    (cx * inv, cz * inv)
}

fn draw_sf(px: &mut Pixmap, s: &Snapshot, n: usize, to_px: impl Fn(f32, f32) -> (f32, f32), track_px: f32) {
    let Some(hit) = along_poly(s, n, s.sf_meters) else {
        return;
    };
    let (x0, y0) = to_px(hit.ax, hit.az);
    let (x1, y1) = to_px(hit.bx, hit.bz);
    let (hx, hy) = to_px(hit.wx, hit.wz);
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
}

fn draw_sector_lines(
    px: &mut Pixmap,
    fonts: &Fonts,
    s: &Snapshot,
    n: usize,
    to_px: impl Fn(f32, f32) -> (f32, f32),
    track_px: f32,
    clip: Option<(f32, f32, f32)>,
) {
    if n < 2 {
        return;
    }
    let (ccx, ccy) = {
        let (wx, wz) = poly_centroid(s, n);
        to_px(wx, wz)
    };
    let half = track_px * 0.5;
    let dash = (half * 0.4).clamp(1.6, 3.5);
    let thick = (track_px * 0.14).clamp(1.1, 2.0);
    let violet = Color::from_rgba8(196, 112, 255, 255);
    let sz = (track_px * 0.95).clamp(10.0, 15.0);
    let pw = px.width() as f32;
    let ph = px.height() as f32;
    for (i, &(frac, label)) in crate::sector::sector_starts().iter().enumerate() {
        // S1 is the line. S2 / S3 wait until that split is known (not noise near S/F).
        if i > 0 && !(0.04..0.96).contains(&frac) {
            continue;
        }
        let Some(hit) = poly_at_frac(s, n, frac) else {
            continue;
        };
        let (hx, hy) = to_px(hit.wx, hit.wz);
        if let Some((cx, cy, r)) = clip {
            let dx = hx - cx;
            let dy = hy - cy;
            if dx * dx + dy * dy > r * r {
                continue;
            }
        }
        let (x0, y0) = to_px(hit.ax, hit.az);
        let (x1, y1) = to_px(hit.bx, hit.bz);
        let tx = x1 - x0;
        let ty = y1 - y0;
        let len = (tx * tx + ty * ty).sqrt().max(1.0e-4);
        let pxn = -ty / len;
        let pyn = tx / len;
        let mut pb = PathBuilder::new();
        pb.move_to(hx - pxn * half, hy - pyn * half);
        pb.line_to(hx + pxn * half, hy + pyn * half);
        if let Some(path) = pb.finish() {
            stroke_dashed(px, &path, Color::from_rgba8(8, 8, 10, 200), thick + 1.0, dash, dash);
            stroke_dashed(px, &path, violet, thick, dash, dash);
        }
        let mut side = if (hx - ccx) * pxn + (hy - ccy) * pyn >= 0.0 { 1.0 } else { -1.0 };
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
        text_bold(px, fonts, label, sz, lx, ly - sz * 0.5, violet, true);
    }
}

fn stroke_dashed(px: &mut Pixmap, path: &Path, color: Color, width: f32, dash: f32, gap: f32) {
    let Some(dash) = StrokeDash::new(vec![dash, gap], 0.0) else {
        return;
    };
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;
    let stroke = Stroke {
        width,
        line_cap: LineCap::Butt,
        line_join: LineJoin::Miter,
        dash: Some(dash),
        ..Stroke::default()
    };
    px.stroke_path(path, &paint, &stroke, Transform::identity(), None);
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
