use std::cell::{Cell, RefCell};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::config::{BoardField, DashField, DotLabel, FontFamily, HudConfig, LeanStyle, RelField, StField, StanceStyle, SYS_PROC_MAX, TableText, WidgetId};
use crate::shm::{cstr, Snapshot, MAX_STANDINGS};
pub use crate::race_store::{ClockSample, clock_sample};
// Re-export clock / field helpers for `render_tests` (`use super::*`).
#[allow(unused_imports)]
pub(crate) use crate::race_store::{
    class_position, extras_started, extra_laps, finish_earned, focus_num_laps,
    standing_num_laps,
    focus_standing, format_countdown, format_gap, format_lap, format_session_clock,
    i_finished, note_laps_to_run, skip_last_lap_white,
    gap_ahead_text, gap_behind_text, interval_text, interval_text_from_row, is_lap_race, is_warmup,
    lapped, laps_done, laps_left,
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
    PathBuilder, Pixmap, PixmapPaint, Point as SkPoint, PremultipliedColorU8, Rect, SpreadMode, Stroke,
    StrokeDash, Transform,
};

mod standings;
mod relative;
mod map;
mod minimap;
mod radar;
mod dash;
mod ticker;
mod sys;
mod sector;
mod delta;
mod flag;
mod stance;
mod lean;
mod gamepad;
mod telemetry;
pub(crate) use standings::*;
pub(crate) use relative::*;
pub(crate) use map::*;
pub(crate) use minimap::*;
pub(crate) use radar::*;
pub(crate) use dash::*;
pub(crate) use ticker::*;
pub(crate) use sys::*;
pub(crate) use sector::*;
pub(crate) use delta::*;
pub(crate) use flag::*;
pub(crate) use stance::*;
pub(crate) use lean::*;
pub(crate) use gamepad::*;
pub(crate) use telemetry::*;
pub use sys::{SysProc, set_sys_procs, set_sys_stats};
pub use stance::{set_stance, stance_sitting};

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

fn telemetry_steer_col() -> Color { Color::from_rgba8(214, 214, 220, 230) }

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
    static HS_SLIDE: RefCell<TableSlides> = RefCell::new(TableSlides { rows: Vec::new() });
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
    fn indices(&mut self, ids: &[i32], now: f32) -> Vec<f32> {
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
                out.push(displayed(e));
            } else {
                self.rows.push(RowSlide {
                    id,
                    from: target,
                    to: target,
                    start: now,
                });
                out.push(target);
            }
        }
        self.rows.retain(|e| ids.contains(&e.id));
        out
    }

    fn step(&mut self, ids: &[i32], body_y: f32, row_h: f32, now: f32) -> Vec<f32> {
        self.indices(ids, now)
            .into_iter()
            .map(|i| body_y + i * row_h)
            .collect()
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
        font_from(include_bytes!("../../fonts/fa-solid-900.ttf").as_slice())
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
                include_bytes!("../../fonts/Roboto-Regular.ttf"),
                None,
                icons,
            ),
            FontFamily::Exo2 => Self::embedded(
                include_bytes!("../../fonts/Exo2-ExtraBoldItalic.ttf"),
                Some(include_bytes!("../../fonts/Exo2-BlackItalic.ttf")),
                icons,
            ),
            FontFamily::Teko => Self::embedded(
                include_bytes!("../../fonts/Teko-SemiBold.ttf"),
                Some(include_bytes!("../../fonts/Teko-Bold.ttf")),
                icons,
            ),
            FontFamily::Goldman => Self::embedded(
                include_bytes!("../../fonts/Goldman-Regular.ttf"),
                Some(include_bytes!("../../fonts/Goldman-Bold.ttf")),
                icons,
            ),
            FontFamily::Montserrat => Self::embedded(
                include_bytes!("../../fonts/Montserrat-ExtraBold.ttf"),
                Some(include_bytes!("../../fonts/Montserrat-Black.ttf")),
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
    if plugin_hint && !settings_hint {
        top_banner(
            px,
            fonts,
            w,
            "Fully quit MX Bikes and start it again so the plugin can load",
        );
    } else if restart_hint {
        top_banner(px, fonts, w, "Restart MX Bikes once so the HUD stays on top while you ride");
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
        } else {
            top_banner(
                px,
                fonts,
                w,
                "Widgets appear on track. If you are racing, fully quit MX Bikes and start it again.",
            );
        }
        return;
    }
    if !cfg.any_overlay_widget() && !settings_hint {
        top_banner(px, fonts, w, "Press F8 — turn on Show on overlay");
    }

    let sw = w as f32;
    let sh = h as f32;
    RaceStore::refresh(s);
    RaceStore::with(|_| {
        let (flag, flag_grow) = if cfg[WidgetId::Dash].show || cfg[WidgetId::Flag].show {
            tick_display_flag(
                s,
                (cfg[WidgetId::Flag].show && cfg.flag_yellow)
                    || (cfg[WidgetId::Dash].show && cfg.dash_yellow),
                (cfg[WidgetId::Flag].show && cfg.flag_blue)
                    || (cfg[WidgetId::Dash].show && cfg.dash_blue),
                (cfg[WidgetId::Flag].show && cfg.flag_red)
                    || (cfg[WidgetId::Dash].show && cfg.dash_red),
            )
        } else {
            (DashFlag::None, 0.0)
        };
        draw_widgets(px, fonts, s, cfg, sw, sh, age, flag, flag_grow);
    });
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

fn draw_widgets(
    px: &mut Pixmap,
    fonts: &Fonts,
    s: &Snapshot,
    cfg: &HudConfig,
    sw: f32,
    sh: f32,
    age: f32,
    flag: DashFlag,
    flag_grow: f32,
) {
    let delta = crate::delta::view_for(cfg.delta_session);
    if s.show_standings != 0 {
        let _g = push_style(fonts, cfg[WidgetId::Standings].bold, cfg[WidgetId::Standings].font);
        draw_standings(px, fonts, s, cfg, sw, sh);
    }
    if s.show_relative != 0 {
        let _g = push_style(fonts, cfg[WidgetId::Relative].bold, cfg[WidgetId::Relative].font);
        draw_relative(px, fonts, s, cfg, sw, sh);
    }
    if s.show_map != 0 {
        let _g = push_style(fonts, cfg[WidgetId::Map].bold, cfg[WidgetId::Map].font);
        draw_map(px, fonts, s, cfg, sw, sh, age);
    }
    if cfg[WidgetId::Minimap].show {
        let _g = push_style(fonts, cfg[WidgetId::Minimap].bold, cfg[WidgetId::Minimap].font);
        draw_minimap(px, fonts, s, cfg, sw, sh, age);
    }
    if cfg[WidgetId::Radar].show {
        let _g = push_style(fonts, cfg[WidgetId::Radar].bold, cfg[WidgetId::Radar].font);
        draw_radar(px, fonts, s, cfg, sw, sh, age);
    }
    if cfg[WidgetId::Dash].show {
        let _g = push_style(fonts, cfg[WidgetId::Dash].bold, cfg[WidgetId::Dash].font);
        let (dash_flag, dash_grow) =
            dash_wrap_flag(flag, flag_grow, cfg.dash_yellow, cfg.dash_blue, cfg.dash_red);
        draw_dash(px, fonts, s, cfg, sw, sh, dash_flag, dash_grow);
    }
    if cfg[WidgetId::Ticker].show {
        let _g = push_style(fonts, cfg[WidgetId::Ticker].bold, cfg[WidgetId::Ticker].font);
        draw_ticker(px, fonts, s, cfg, sw, sh);
    }
    if cfg[WidgetId::Sys].show {
        let _g = push_style(fonts, cfg[WidgetId::Sys].bold, cfg[WidgetId::Sys].font);
        draw_sys(px, fonts, cfg, sw, sh);
    }
    if cfg.sector_visible() {
        let _g = push_style(fonts, cfg[WidgetId::Sector].bold, cfg[WidgetId::Sector].font);
        draw_sector(px, fonts, s, cfg, sw, sh);
    }
    if cfg.delta_visible() {
        let _g = push_style(fonts, cfg[WidgetId::Delta].bold, cfg[WidgetId::Delta].font);
        draw_delta(px, fonts, cfg, sw, sh, delta);
    }
    if cfg[WidgetId::Flag].show {
        let _g = push_style(fonts, cfg[WidgetId::Flag].bold, cfg[WidgetId::Flag].font);
        let (flag, flag_grow) =
            flag_widget_flag(flag, flag_grow, cfg.flag_yellow, cfg.flag_blue, cfg.flag_red);
        draw_flag(px, fonts, cfg, sw, sh, flag, flag_grow);
    }
    if cfg.stance_visible() {
        let _g = push_style(fonts, cfg[WidgetId::Stance].bold, cfg[WidgetId::Stance].font);
        draw_stance(px, fonts, cfg, sw, sh);
    }
    if cfg[WidgetId::Lean].show {
        let _g = push_style(fonts, cfg[WidgetId::Lean].bold, cfg[WidgetId::Lean].font);
        draw_lean(px, fonts, s, cfg, sw, sh);
    }
    if cfg.gamepad_visible() {
        let _g = push_style(fonts, cfg[WidgetId::Gamepad].bold, cfg[WidgetId::Gamepad].font);
        draw_gamepad(px, fonts, cfg, sw, sh);
    }
    if cfg[WidgetId::Telemetry].show {
        let _g = push_style(fonts, cfg[WidgetId::Telemetry].bold, cfg[WidgetId::Telemetry].font);
        draw_telemetry(px, fonts, s, cfg, sw, sh);
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
    ICON.get_or_init(|| Pixmap::decode_png(include_bytes!("../../../icon-48.png")).expect("icon-48.png"))
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
        let r = table_layout_rect(s, cfg, WidgetId::Standings, s.standings_rect, sw, sh);
        layout_box(px, r.x * sw, r.y * sh, r.w * sw, r.h * sh, false);
    }
    if s.show_relative != 0 {
        let r = table_layout_rect(s, cfg, WidgetId::Relative, s.relative, sw, sh);
        layout_box(px, r.x * sw, r.y * sh, r.w * sw, r.h * sh, false);
    }
    if s.show_map != 0 {
        layout_box(px, s.map.x * sw, s.map.y * sh, s.map.w * sw, s.map.h * sh, false);
    }
    if cfg[WidgetId::Radar].show {
        layout_box(px, cfg[WidgetId::Radar].rect.x * sw, cfg[WidgetId::Radar].rect.y * sh, cfg[WidgetId::Radar].rect.w * sw, cfg[WidgetId::Radar].rect.h * sh, false);
    }
    if cfg[WidgetId::Ticker].show {
        layout_box(px, cfg[WidgetId::Ticker].rect.x * sw, cfg[WidgetId::Ticker].rect.y * sh, cfg[WidgetId::Ticker].rect.w * sw, cfg[WidgetId::Ticker].rect.h * sh, true);
    }
    if cfg[WidgetId::Sys].show {
        layout_box(px, cfg[WidgetId::Sys].rect.x * sw, cfg[WidgetId::Sys].rect.y * sh, cfg[WidgetId::Sys].rect.w * sw, cfg[WidgetId::Sys].rect.h * sh, false);
    }
    if cfg.sector_visible() {
        layout_box(px, cfg[WidgetId::Sector].rect.x * sw, cfg[WidgetId::Sector].rect.y * sh, cfg[WidgetId::Sector].rect.w * sw, cfg[WidgetId::Sector].rect.h * sh, false);
    }
    if cfg.delta_visible() {
        layout_box(px, cfg[WidgetId::Delta].rect.x * sw, cfg[WidgetId::Delta].rect.y * sh, cfg[WidgetId::Delta].rect.w * sw, cfg[WidgetId::Delta].rect.h * sh, false);
    }
    if cfg.stance_visible() {
        layout_box(px, cfg[WidgetId::Stance].rect.x * sw, cfg[WidgetId::Stance].rect.y * sh, cfg[WidgetId::Stance].rect.w * sw, cfg[WidgetId::Stance].rect.h * sh, false);
    }
    if cfg[WidgetId::Lean].show {
        layout_box(px, cfg[WidgetId::Lean].rect.x * sw, cfg[WidgetId::Lean].rect.y * sh, cfg[WidgetId::Lean].rect.w * sw, cfg[WidgetId::Lean].rect.h * sh, false);
    }
    if cfg.gamepad_visible() {
        layout_box(px, cfg[WidgetId::Gamepad].rect.x * sw, cfg[WidgetId::Gamepad].rect.y * sh, cfg[WidgetId::Gamepad].rect.w * sw, cfg[WidgetId::Gamepad].rect.h * sh, false);
    }
    if cfg[WidgetId::Telemetry].show {
        layout_box(px, cfg[WidgetId::Telemetry].rect.x * sw, cfg[WidgetId::Telemetry].rect.y * sh, cfg[WidgetId::Telemetry].rect.w * sw, cfg[WidgetId::Telemetry].rect.h * sh, false);
    }
    if cfg[WidgetId::Flag].show {
        layout_box(px, cfg[WidgetId::Flag].rect.x * sw, cfg[WidgetId::Flag].rect.y * sh, cfg[WidgetId::Flag].rect.w * sw, cfg[WidgetId::Flag].rect.h * sh, false);
    }
    if cfg[WidgetId::Dash].show {
        layout_box(
            px,
            cfg[WidgetId::Dash].rect.x * sw,
            cfg[WidgetId::Dash].rect.y * sh,
            cfg[WidgetId::Dash].rect.w * sw,
            cfg[WidgetId::Dash].rect.h * sh,
            false,
        );
    }
    if cfg[WidgetId::Minimap].show {
        let rw = cfg[WidgetId::Minimap].rect.w * sw;
        let rh = cfg[WidgetId::Minimap].rect.h * sh;
        let d = rw.min(rh);
        let cx = cfg[WidgetId::Minimap].rect.x * sw + rw * 0.5;
        let cy = cfg[WidgetId::Minimap].rect.y * sh + rh * 0.5;
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

/// 1px night-ink border around glyphs. Not a drop-shadow: same 8-neighbor rim as radar numbers.
fn text_halo(
    px: &mut Pixmap,
    fonts: &Fonts,
    s: &str,
    size: f32,
    x: f32,
    y: f32,
    color: Color,
    center: bool,
    glass: bool,
) {
    if glass {
        let ink = Color::from_rgba8(10, 10, 10, 230);
        for (dx, dy) in [
            (-1.0, 0.0),
            (1.0, 0.0),
            (0.0, -1.0),
            (0.0, 1.0),
            (-1.0, -1.0),
            (1.0, -1.0),
            (-1.0, 1.0),
            (1.0, 1.0),
        ] {
            text(px, fonts, s, size, x + dx, y + dy, ink, center);
        }
    }
    text(px, fonts, s, size, x, y, color, center);
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

fn table_font_k(pct: i32) -> f32 {
    (pct.clamp(70, 160) as f32) / 100.0
}

fn table_stack_h(k: f32, vis_rows: usize, has_foot: bool) -> f32 {
    let head_h = 26.0 * k;
    let col_h = 16.0 * k;
    let track_h = 20.0 * k;
    let row_h = 22.0 * k;
    let foot_h = if has_foot { 20.0 * k } else { 0.0 };
    // Follow the row stack, not a stale widget box. Ctrl+move can save a
    // hugged 1-row height; Rows can grow without rewriting standings_h.
    head_h + col_h + track_h + vis_rows as f32 * row_h + foot_h + 8.0
}

fn standings_vis_rows(s: &Snapshot) -> usize {
    let n = (s.standing_count.max(0) as usize).min(MAX_STANDINGS);
    let max_rows = s.standings_rows.max(3) as usize;
    n.min(max_rows).max(1)
}

fn relative_vis_rows(s: &Snapshot) -> usize {
    let n = s.rider_count.max(0) as usize;
    let focus = if s.focus_race_num > 0 {
        s.focus_race_num
    } else {
        s.local_race_num
    };
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
            have = true;
        }
        uniq.push(i);
    }
    let side = s.relative_count.max(1) as usize;
    if !have || uniq.is_empty() {
        1
    } else {
        uniq.len().min(side * 2 + 1).max(1)
    }
}

/// Painted standings / relative plaque in normalized overlay space (hugged columns, row stack).
pub fn table_layout_rect(
    s: &Snapshot,
    cfg: &HudConfig,
    id: WidgetId,
    rect: crate::shm::Rect,
    sw: f32,
    sh: f32,
) -> crate::shm::Rect {
    if sw <= 0.0 || sh <= 0.0 {
        return rect;
    }
    let (widths, flex, vis, has_foot) = match id {
        WidgetId::Standings => {
            let cols = cfg.standings_cols();
            let flex = cols
                .iter()
                .position(|c| c.is_name())
                .unwrap_or(cols.len().saturating_sub(1));
            (
                cols.iter().map(|c| c.width(cfg) as f32).collect::<Vec<_>>(),
                flex,
                standings_vis_rows(s),
                BoardField::any(&cfg.st_foot),
            )
        }
        WidgetId::Relative => {
            let cols = cfg.relative_cols();
            let flex = cols
                .iter()
                .position(|c| c.is_name())
                .unwrap_or(cols.len().saturating_sub(1));
            (
                cols.iter().map(|c| c.width(cfg) as f32).collect::<Vec<_>>(),
                flex,
                relative_vis_rows(s),
                BoardField::any(&cfg.rel_foot),
            )
        }
        _ => return rect,
    };
    let k = table_font_k(cfg[id].font);
    let pad = 8.0;
    let max_w = rect.w * sw;
    let idxs: Vec<usize> = (0..widths.len()).collect();
    let slots = col_slots(0.0, pad, max_w, &idxs, |i| widths[i], |i| i == flex);
    let w = hug_board_w(0.0, pad, max_w, &slots);
    let h = table_stack_h(k, vis, has_foot);
    crate::shm::Rect {
        x: rect.x,
        y: rect.y,
        w: w / sw,
        h: h / sh,
    }
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
    RaceStore::with(|race| {
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
    })
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

trait BoardCol: Copy + PartialEq {
    fn header(self) -> &'static str;
    fn width(self, cfg: &HudConfig) -> i32;
    fn is_name(self) -> bool;
    fn is_bike(self) -> bool;
    fn is_pos(self) -> bool;
}

impl BoardCol for StField {
    fn header(self) -> &'static str {
        match self {
            Self::Pos => "P",
            Self::Num => "#",
            Self::Name => "NAME",
            Self::Laps => "Completed Laps",
            Self::Current => "Current Lap",
            Self::Best => "Fastest",
            Self::Last => "Last",
            Self::Status => "ST",
            Self::Gap => "GAP",
            Self::Interval => "INT",
            Self::Bike => "BIKE",
            Self::Penalty => "PEN",
            Self::Crashed => "CR",
        }
    }

    fn width(self, cfg: &HudConfig) -> i32 {
        StField::width(self, cfg)
    }

    fn is_name(self) -> bool {
        matches!(self, Self::Name)
    }

    fn is_bike(self) -> bool {
        matches!(self, Self::Bike)
    }

    fn is_pos(self) -> bool {
        matches!(self, Self::Pos)
    }
}

impl BoardCol for RelField {
    fn header(self) -> &'static str {
        match self {
            Self::Pos => "P",
            Self::Num => "#",
            Self::Name => "NAME",
            Self::Gap => "Gap",
            Self::Laps => "Completed Laps",
            Self::Current => "Current Lap",
            Self::Bike => "BIKE",
            Self::Penalty => "Pen",
            Self::Interval => "Int",
            Self::Crashed => "Crash",
            Self::Best => "Fastest",
            Self::Last => "Last",
        }
    }

    fn width(self, cfg: &HudConfig) -> i32 {
        RelField::width(self, cfg)
    }

    fn is_name(self) -> bool {
        matches!(self, Self::Name)
    }

    fn is_bike(self) -> bool {
        matches!(self, Self::Bike)
    }

    fn is_pos(self) -> bool {
        matches!(self, Self::Pos)
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

/// Rider you are coming to lap, ahead and close — same window as blue.
const FLAG_RED_SPAN_M: f32 = 40.0;

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
        DashFlag::Red => 5,
    }
}

fn flag_from_code(code: i32) -> DashFlag {
    match code {
        1 => DashFlag::White,
        2 => DashFlag::Checkered,
        3 => DashFlag::Yellow,
        4 => DashFlag::Blue,
        5 => DashFlag::Red,
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
/// lap of the distance you are still running — and for `WHITE_WAVE_MS` after, so it is
/// not held up all the way to the finish. A lapped wave-off skips this (`skip_last_lap_white`).
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
/// checkered, and the white comes down a few seconds into the lap. Checkered is your
/// finish — the leader taking the flag does not wave you off a lap short.
fn dash_race_flag(s: &Snapshot) -> DashFlag {
    if s.on_track == 0 {
        return reset_flag_state();
    }
    note_line_progress(s);
    RaceStore::with(|store| {
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
    let no_white = skip_last_lap_white(s, left);
    let flag = match left {
        // You crossed the line with nothing left to run. Never gated on speed, so a
        // slow roll over the line still gets waved off. `finish_earned` keeps a single
        // glitched frame from waving you off mid-race.
        Some(0) if finish_earned(s) => latch_checkered(),
        // Coming to the line with nothing left to run: the checkered is already out.
        Some(1) if line_approach(s) => DashFlag::Checkered,
        Some(0) | Some(1) if no_white => DashFlag::None,
        Some(0) | Some(1) => white_wave(s),
        Some(2) if line_approach(s) && no_white => DashFlag::None,
        Some(2) if line_approach(s) => DashFlag::White,
        _ => DashFlag::None,
    };
    hold_across_line(s, flag)
    })
}

/// Website demo: 0 none, 1 white, 2 checkered, 3 yellow, 4 blue, 5 red. Negative = live.
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

fn tick_display_flag(s: &Snapshot, yellow: bool, blue: bool, red: bool) -> (DashFlag, f32) {
    flag_anim_step(wanted_flag(s, yellow, blue, red))
}

fn wanted_flag(s: &Snapshot, yellow: bool, blue: bool, red: bool) -> DashFlag {
    match FLAG_PREVIEW.load(Ordering::Relaxed) {
        0 => DashFlag::None,
        1 => DashFlag::White,
        2 => DashFlag::Checkered,
        3 => DashFlag::Yellow,
        4 => DashFlag::Blue,
        5 => DashFlag::Red,
        _ => {
            let race = dash_race_flag(s);
            if yellow || blue || red {
                merge_caution(race, caution_flag(s, yellow, blue, red))
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
/// blue is someone a lap up closing from behind (`LapRel::LappingMe`);
/// red is someone you are coming to lap (`LapRel::LappedByMe`).
fn caution_flag(s: &Snapshot, yellow: bool, blue: bool, red: bool) -> DashFlag {
    if s.on_track == 0 || is_warmup(s) || prestart(s) {
        return DashFlag::None;
    }
    if yellow && nearby_crash(s) {
        DashFlag::Yellow
    } else if blue && being_lapped(s) {
        DashFlag::Blue
    } else if red && lapping_them(s) {
        DashFlag::Red
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

fn lapping_them(s: &Snapshot) -> bool {
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
            if lap_rel(s, r.race_num) != LapRel::LappedByMe {
                return false;
            }
            let Some(op) = rider_norm_pos(s, r.race_num) else {
                return false;
            };
            let Some(mp) = rider_norm_pos(s, focus) else {
                return false;
            };
            let w = wrap_frac(op, mp);
            let ahead_m = if w > 0.0 { closing_m(s, w) } else { 0.0 };
            ahead_m > 2.0 && ahead_m <= FLAG_RED_SPAN_M
        })
}

fn dash_wrap_flag(flag: DashFlag, grow: f32, yellow: bool, blue: bool, red: bool) -> (DashFlag, f32) {
    match flag {
        DashFlag::Yellow if yellow => (flag, grow),
        DashFlag::Blue if blue => (flag, grow),
        DashFlag::Red if red => (flag, grow),
        DashFlag::Yellow | DashFlag::Blue | DashFlag::Red => (DashFlag::None, 0.0),
        other => (other, grow),
    }
}

fn flag_widget_flag(flag: DashFlag, grow: f32, yellow: bool, blue: bool, red: bool) -> (DashFlag, f32) {
    dash_wrap_flag(flag, grow, yellow, blue, red)
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
    RaceStore::with(|race| {
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
        BoardField::Setup => {
            let name = s.setup_label();
            if name.is_empty() {
                "--".into()
            } else {
                name
            }
        }
        BoardField::GapAhead => st
            .map(|r| gap_ahead_text(s, &race.field, r))
            .unwrap_or_else(|| "---".into()),
        BoardField::GapBehind => st
            .map(|r| gap_behind_text(s, &race.field, r))
            .unwrap_or_else(|| "---".into()),
    };
    Some((field.icon(), text))
    })
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

struct TableLook<'a> {
    bg: i32,
    hl: i32,
    text: TableText,
    stripe: bool,
    head: &'a [BoardField; 3],
    foot: &'a [BoardField; 3],
}

struct TableBoard<'a, C: BoardCol> {
    px: &'a mut Pixmap,
    fonts: &'a Fonts,
    s: &'a Snapshot,
    cfg: &'a HudConfig,
    slots: Vec<(C, f32, f32)>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    row_h: f32,
    body_y: f32,
    foot_h: f32,
    a: u8,
    you_bg: Color,
    stripe_c: Color,
    stripe: bool,
    bar_end: Option<f32>,
    ink: Color,
    ink_dim: Color,
    out_c: Color,
    foot: &'a [BoardField; 3],
}

struct TableRow<'a, C: BoardCol> {
    vis_i: usize,
    cy: f32,
    x: f32,
    w: f32,
    row_h: f32,
    slots: &'a [(C, f32, f32)],
    bar_end: Option<f32>,
    you_bg: Color,
    ink: Color,
    ink_dim: Color,
    out_c: Color,
}

fn plaque_cap_a(a: u8) -> u8 {
    ((a as u16 * 240) / 200).min(255) as u8
}

fn draw_table_board<C: BoardCol>(
    px: &mut Pixmap,
    fonts: &Fonts,
    s: &Snapshot,
    cfg: &HudConfig,
    rect: crate::shm::Rect,
    sw: f32,
    sh: f32,
    cols: &[C],
    look: TableLook<'_>,
    vis_rows: usize,
    count_n: usize,
    empty: Option<&'static str>,
    body: impl FnOnce(&mut TableBoard<'_, C>),
) {
    let x = rect.x * sw;
    let y = rect.y * sh;
    let max_w = rect.w * sw;
    let k = style_k();
    let head_h = 26.0 * k;
    let col_h = 16.0 * k;
    let track_h = 20.0 * k;
    let row_h = 22.0 * k;
    let foot_h = if BoardField::any(look.foot) { 20.0 * k } else { 0.0 };
    let h = table_stack_h(k, vis_rows, BoardField::any(look.foot));
    let pad = 8.0;
    let slots = col_slots(x, pad, max_w, cols, |c| c.width(cfg) as f32, |c| c.is_name());
    let w = hug_board_w(x, pad, max_w, &slots);
    let a = bg_a(look.bg);
    if a > 0 {
        fill_round(px, x, y, w, h, 6.0, Color::from_rgba8(8, 8, 10, a));
        fill_round(px, x, y, w, head_h, 6.0, Color::from_rgba8(4, 4, 6, plaque_cap_a(a)));
        if let Some(rrt) = rr(x, y + head_h - 6.0, w, 6.0) {
            fill_rect(px, rrt, Color::from_rgba8(4, 4, 6, plaque_cap_a(a)));
        }
    }
    draw_board_bar(px, fonts, s, cfg, look.head, x, y, w, head_h);
    if let Some(msg) = empty {
        text(px, fonts, msg, 12.0, x + 12.0, y + head_h + 10.0, text_dim(), false);
        paint_table_footer(px, fonts, s, cfg, look.foot, x, y, w, h, foot_h, a);
        return;
    }

    let (ink, ink_dim, out_c, hdr_c) = table_ink(look.text);
    let bar_end = slots
        .iter()
        .find(|(c, _, _)| c.is_pos())
        .map(|(_, cx, cw)| bike_bar_end(*cx, *cw));
    let mut cy = y + head_h;
    let track = {
        let t = cstr(&s.track_name);
        if t.is_empty() { "TRACK".into() } else { t.to_uppercase() }
    };
    draw_count_track(px, fonts, x, cy, count_n, &track);
    if let Some(line) = rr(x + 8.0, cy + 18.0, w - 16.0, 1.2) {
        fill_rect(px, line, accent());
    }
    cy += track_h;
    let hdr_y = cy + 2.0;
    for (col, cx, cw) in &slots {
        let right = !col.is_name() && !col.is_bike();
        let pad = if col.is_name() { name_left_pad(*cx, bar_end) } else { 0.0 };
        col_text(px, fonts, col.header(), 10.0, *cx + pad, (*cw - pad).max(8.0), hdr_y, hdr_c, right);
    }
    cy += col_h;

    let mut board = TableBoard {
        px,
        fonts,
        s,
        cfg,
        slots,
        x,
        y,
        w,
        h,
        row_h,
        body_y: cy,
        foot_h,
        a,
        you_bg: you_row_bg(look.hl),
        stripe_c: stripe_row_bg(a),
        stripe: look.stripe,
        bar_end,
        ink,
        ink_dim,
        out_c,
        foot: look.foot,
    };
    body(&mut board);
    paint_table_footer(board.px, board.fonts, board.s, board.cfg, board.foot, board.x, board.y, board.w, board.h, board.foot_h, board.a);
}

fn paint_table_footer(
    px: &mut Pixmap,
    fonts: &Fonts,
    s: &Snapshot,
    cfg: &HudConfig,
    foot: &[BoardField; 3],
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    foot_h: f32,
    a: u8,
) {
    if foot_h <= 0.0 {
        return;
    }
    if a > 0 {
        if let Some(rrt) = rr(x, y + h - foot_h, w, foot_h) {
            fill_rect(px, rrt, Color::from_rgba8(4, 4, 6, plaque_cap_a(a)));
        }
    }
    draw_board_bar(px, fonts, s, cfg, foot, x, y + h - foot_h, w, foot_h);
}

impl<C: BoardCol> TableBoard<'_, C> {
    fn rows(
        &mut self,
        ids: &[i32],
        slides: &mut TableSlides,
        mut each: impl FnMut(&mut Pixmap, &Fonts, TableRow<'_, C>),
    ) {
        let row_ys = slides.step(ids, self.body_y, self.row_h, anim_now());
        let x = self.x;
        let w = self.w;
        let row_h = self.row_h;
        let body_y = self.body_y;
        let a = self.a;
        let stripe_c = self.stripe_c;
        let bar_end = self.bar_end;
        let you_bg = self.you_bg;
        let ink = self.ink;
        let ink_dim = self.ink_dim;
        let out_c = self.out_c;
        if self.stripe && a > 0 {
            for vis_i in 0..ids.len() {
                if vis_i % 2 == 1 {
                    fill_focus_row(self.px, x, body_y + vis_i as f32 * row_h, w, row_h, stripe_c);
                }
            }
        }
        let px = &mut *self.px;
        let fonts = self.fonts;
        let slots = self.slots.as_slice();
        for vis_i in 0..ids.len() {
            each(
                px,
                fonts,
                TableRow {
                    vis_i,
                    cy: row_ys[vis_i],
                    x,
                    w,
                    row_h,
                    slots,
                    bar_end,
                    you_bg,
                    ink,
                    ink_dim,
                    out_c,
                },
            );
        }
    }
}

impl<C: BoardCol> TableRow<'_, C> {
    fn fill_you(&self, px: &mut Pixmap) {
        fill_focus_row(px, self.x, self.cy, self.w, self.row_h, self.you_bg);
    }

    fn fill(&self, px: &mut Pixmap, c: Color) {
        fill_focus_row(px, self.x, self.cy, self.w, self.row_h, c);
    }

    fn paint(
        &self,
        px: &mut Pixmap,
        fonts: &Fonts,
        accent: Color,
        click: Option<i32>,
        mut cell: impl FnMut(C) -> (String, Color, bool),
    ) {
        paint_table_row(px, fonts, self, accent, click, &mut cell);
    }
}

fn paint_table_row<C: BoardCol>(
    px: &mut Pixmap,
    fonts: &Fonts,
    row: &TableRow<'_, C>,
    accent: Color,
    click: Option<i32>,
    cell: &mut impl FnMut(C) -> (String, Color, bool),
) {
    let mut named = false;
    for (kind, cx, cw) in row.slots {
        if kind.is_pos() {
            fill_skew(px, *cx + *cw + 1.0, row.cy + 4.0, BIKE_BAR_W, row.row_h - 8.0, BIKE_BAR_SKEW, accent);
        }
        let (val, color, right) = cell(*kind);
        let pad = if kind.is_name() { name_left_pad(*cx, row.bar_end) } else { 0.0 };
        if kind.is_name() {
            named = true;
            if let Some(num) = click {
                push_click_rider(num, *cx + pad, row.cy, (*cw - pad).max(8.0), row.row_h);
            }
        }
        if kind.is_bike() && !val.is_empty() {
            draw_bike_pill(px, fonts, &val, *cx, row.cy, *cw, row.row_h, accent);
        } else {
            col_text(px, fonts, &val, 12.0, *cx + pad, (*cw - pad).max(8.0), row.cy + 4.0, color, right);
        }
    }
    if !named {
        if let Some(num) = click {
            push_click_rider(num, row.x, row.cy, row.w, row.row_h);
        }
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

fn signed_deg(deg: f32) -> String {
    let n = deg.round();
    if n == 0.0 {
        "0°".into()
    } else {
        format!("{:+.0}°", n)
    }
}

fn stroke_smooth_series(px: &mut Pixmap, pts: &[(f32, f32)], color: Color, width: f32, y_lo: f32, y_hi: f32) {
    if pts.len() < 2 {
        return;
    }
    let n = pts.len();
    let y_at = |j: i32| pts[j.clamp(0, n as i32 - 1) as usize].1;
    let mut smooth = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as i32;
        let y = (y_at(t - 2)
            + y_at(t - 1) * 2.0
            + y_at(t) * 3.0
            + y_at(t + 1) * 2.0
            + y_at(t + 2))
            / 9.0;
        smooth.push((pts[i].0, y.clamp(y_lo, y_hi)));
    }
    let mut pb = PathBuilder::new();
    pb.move_to(smooth[0].0, smooth[0].1);
    if smooth.len() == 2 {
        pb.line_to(smooth[1].0, smooth[1].1);
    } else {
        for i in 0..smooth.len() - 1 {
            let p0 = smooth[i.saturating_sub(1)];
            let p1 = smooth[i];
            let p2 = smooth[i + 1];
            let p3 = smooth[(i + 2).min(smooth.len() - 1)];
            let c1x = p1.0 + (p2.0 - p0.0) / 6.0;
            let c1y = (p1.1 + (p2.1 - p0.1) / 6.0).clamp(y_lo, y_hi);
            let c2x = p2.0 - (p3.0 - p1.0) / 6.0;
            let c2y = (p2.1 - (p3.1 - p1.1) / 6.0).clamp(y_lo, y_hi);
            pb.cubic_to(c1x, c1y, c2x, c2y, p2.0, p2.1);
        }
    }
    if let Some(path) = pb.finish() {
        stroke_path(px, &path, color, width);
    }
}

fn stroke_circle(px: &mut Pixmap, cx: f32, cy: f32, r: f32, color: Color, width: f32) {
    if r < 1.0 {
        return;
    }
    let mut pb = PathBuilder::new();
    pb.push_circle(cx, cy, r);
    if let Some(path) = pb.finish() {
        stroke_path(px, &path, color, width);
    }
}

fn dest_lum_alpha(d: PremultipliedColorU8) -> (u8, u8) {
    let da = d.alpha();
    if da < 8 {
        return (0, da);
    }
    let ur = d.red() as u16 * 255 / da as u16;
    let ug = d.green() as u16 * 255 / da as u16;
    let ub = d.blue() as u16 * 255 / da as u16;
    ((((ur * 30 + ug * 59 + ub * 11) / 100) as u8), da)
}

fn paint_keep_alpha(r: f32, g: f32, b: f32, da: u8) -> Option<PremultipliedColorU8> {
    let a = da as f32;
    PremultipliedColorU8::from_rgba(
        (r * a / 255.0).round() as u8,
        (g * a / 255.0).round() as u8,
        (b * a / 255.0).round() as u8,
        da,
    )
}

fn art_lum_at(art: &Pixmap, x: u32, y: u32) -> u8 {
    match art.pixel(x, y) {
        Some(c) if c.alpha() >= 8 => {
            let a = c.alpha() as u16;
            let ur = c.red() as u16 * 255 / a;
            let ug = c.green() as u16 * 255 / a;
            let ub = c.blue() as u16 * 255 / a;
            ((ur * 30 + ug * 59 + ub * 11) / 100) as u8
        }
        _ => 255,
    }
}

const MAX_SAFE: usize = 40;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum DashFlag {
    None,
    White,
    Checkered,
    Yellow,
    Blue,
    Red,
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

fn max_digit_w(fonts: &Fonts, size: f32) -> f32 {
    ('0'..='9').map(|d| measure(fonts, d.encode_utf8(&mut [0; 4]), size)).fold(0.0, f32::max)
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

fn format_clock(ms: i32) -> String {
    if ms <= 0 {
        return "--:--.---".into();
    }
    let t = ms as f32 / 1000.0;
    let m = (t / 60.0) as i32;
    let s = t - m as f32 * 60.0;
    format!("{m:02}:{:06.3}", s)
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

fn draw_solid_wrap(px: &mut Pixmap, d: &DashLay, border: f32, cloth: Color) {
    let Some(frame) = dash_wrap_frame_path(d, border) else {
        return;
    };
    fill_path_rule(px, &frame, cloth, FillRule::EvenOdd, None);
}

fn draw_solid_banner(
    px: &mut Pixmap,
    fonts: &Fonts,
    band: &Path,
    ox: f32,
    top_y: f32,
    ow: f32,
    top_h: f32,
    grow: f32,
    cloth: Color,
    ink: Color,
    label: &str,
) {
    fill_path(px, band, cloth);
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

fn shift_gear_col(s: &Snapshot, on: bool, rest: Color) -> Color {
    if on && crate::telemetry::shift_warn(s) {
        Color::from_rgba8(232, 132, 138, 255)
    } else {
        rest
    }
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

fn pad_from_size(size: f32) -> f32 {
    (size * 0.10).max(8.0)
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
    stroke_path_clip(px, path, color, width, None);
}

fn stroke_path_clip(px: &mut Pixmap, path: &Path, color: Color, width: f32, clip: Option<&Mask>) {
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;
    let stroke = Stroke {
        width,
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Stroke::default()
    };
    px.stroke_path(path, &paint, &stroke, Transform::identity(), clip);
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
mod render_tests {
    include!("../tests/render.rs");
}
