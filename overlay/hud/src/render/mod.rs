use std::cell::{Cell, RefCell};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::config::{
    accent_rgb, BoardField, DashField, DotLabel, EditSurface, HudConfig, LeanStyle,
    MAP_DOT_OPACITY_DEFAULT,
    RelField, StField, StanceStyle, TableText, WidgetId, SYS_PROC_MAX,
};
pub use crate::race_store::{clock_sample, ClockSample};
use crate::shm::{bytes_as_text, Rider, Snapshot, MAX_STANDINGS};
// Re-export clock / field helpers for `render_tests` (`use super::*`).
#[allow(unused_imports)]
pub(crate) use crate::race_store::{
    class_position, extra_laps, extras_started, finish_earned, focus_num_laps, focus_standing,
    format_countdown, format_gap, format_lap, format_session_clock, gap_ahead_text,
    gap_behind_text, gap_leader_text, i_finished, interval_text, interval_text_from_row,
    is_lap_race, is_practice_session, is_warmup, lap_diff_ms, lapped, laps_done, laps_left,
    leader_finished,
    leader_num_laps, live_leader, live_position, local_overtime_done, local_overtime_taken, moving,
    norm_lap_pos as norm_track_pos, note_laps_to_run, overtime_active, penalty_class_place_delta,
    penalty_place_delta, prestart, race_lap, race_laps_left_text, race_over_for_me,
    race_progress_text, reset_session_clock_track, rider_current_lap, session_banner,
    session_best_ms, session_len_ms, session_remain_ms, skip_last_lap_white, standing_num_laps,
    standing_of, timed_clock_live, timed_race_flag, track_position,
    RaceFlag, RaceStore, CHECKERED_LATCH, CLOSING_ON_LINE, IN_GATE, LAPS_TO_RUN_AT, LAP_GREEN,
    LAP_MID_SEEN, LAST_CUR_LAP, LAST_SESSION_SIG, LAST_SF_METERS, LEADER_FIN_LOCAL_BASE,
    OVERTIME_LOCAL_BASE, POST_GATE, RUN_IN_FLAG, SESSION_EXPIRED, START_FINISH_FRACTION_CANDIDATE, START_FINISH_FRACTION_LEARNED,
    START_FINISH_LEARN_LAPS, WHITE_WAVE_AT, WHITE_WAVE_LAP,
};
use tiny_skia::{
    Color, FillRule, FilterQuality, GradientStop, LineCap, LineJoin, LinearGradient, Mask, Paint,
    Path, PathBuilder, Pixmap, PixmapPaint, Point as SkPoint, PremultipliedColorU8, Rect,
    SpreadMode, Stroke, StrokeDash, Transform,
};

mod dash;
mod delta;
mod flag;
mod gamepad;
mod lean;
mod map;
mod minimap;
mod radar;
mod relative;
mod sector;
mod stance;
mod standings;
mod sys;
mod telemetry;
mod ticker;
mod pitboard;
mod timer;
pub(crate) use dash::*;
pub(crate) use delta::*;
pub(crate) use flag::*;
pub(crate) use gamepad::*;
pub(crate) use lean::*;
pub(crate) use map::*;
pub(crate) use minimap::*;
pub(crate) use radar::*;
pub(crate) use relative::*;
pub(crate) use sector::*;
pub(crate) use stance::*;
pub use stance::{set_stance, stance_sitting};
pub(crate) use standings::*;
pub(crate) use sys::*;
pub use sys::{set_sys_procs, set_sys_stats, SysProc};
pub(crate) use telemetry::*;
pub(crate) use ticker::*;
pub(crate) use pitboard::*;
pub(crate) use timer::*;

mod table;
mod text;
pub use table::{click_rider_at, click_rider_hits, table_layout_rect, ClickRider};
pub(crate) use table::*;
pub use text::{icon, measure, text, text_bold, Fonts};
pub(crate) use text::*;

pub fn painted_factory_plate(art: &str, main: [u8; 3], secondary: [u8; 3]) -> Option<Pixmap> {
    pitboard::factory_plate_image(art, main, secondary)
}

fn accent() -> Color {
    let [r, g, b] = accent_rgb();
    Color::from_rgba8(r, g, b, 255)
}

fn accent_a(a: u8) -> Color {
    let [r, g, b] = accent_rgb();
    Color::from_rgba8(r, g, b, a)
}

fn text_col() -> Color {
    Color::from_rgba8(228, 228, 230, 255)
}

fn text_dim() -> Color {
    Color::from_rgba8(132, 132, 138, 255)
}

fn panel_col() -> Color {
    Color::from_rgba8(10, 10, 10, 200)
}

fn track_col() -> Color {
    Color::from_rgba8(236, 236, 240, 255)
}

fn fill_col() -> Color {
    Color::from_rgba8(10, 8, 8, 168)
}

fn you_col() -> Color {
    accent()
}

fn other_col() -> Color {
    Color::from_rgba8(48, 52, 64, 255)
}

fn lapping_col() -> Color {
    Color::from_rgba8(59, 130, 246, 255)
}

fn lapped_col() -> Color {
    Color::from_rgba8(239, 68, 68, 255)
}

fn ahead_col() -> Color {
    Color::from_rgba8(48, 220, 88, 255)
}

fn behind_col() -> Color {
    Color::from_rgba8(255, 64, 72, 255)
}

fn lap_diff_text(race_num: i32) -> String {
    match lap_diff_ms(race_num) {
        Some(ms) => format_delta_ms(ms),
        None => "--".into(),
    }
}

/// Green when the last lap was faster than the one before it, red when slower.
fn lap_diff_ink(race_num: i32, neutral: Color) -> Color {
    match lap_diff_ms(race_num) {
        Some(ms) if ms < 0 => ahead_col(),
        Some(ms) if ms > 0 => behind_col(),
        _ => neutral,
    }
}

/// Green `*` when live place is ahead of on-track, red when behind.
fn place_star_col(delta: Option<i32>) -> Option<Color> {
    match delta {
        Some(d) if d > 0 => Some(ahead_col()),
        Some(d) if d < 0 => Some(behind_col()),
        _ => None,
    }
}

fn format_place_digits(pos: i32, with_p: bool) -> String {
    if with_p {
        if pos > 0 {
            format!("P{pos}")
        } else {
            "P--".into()
        }
    } else if pos > 0 {
        format!("{pos}")
    } else {
        String::new()
    }
}

fn place_width(fonts: &Fonts, digits: &str, size: f32, star: Option<Color>) -> f32 {
    measure(fonts, digits, size)
        + if star.is_some() {
            measure(fonts, "*", size)
        } else {
            0.0
        }
}

fn paint_place_at(
    px: &mut Pixmap,
    fonts: &Fonts,
    digits: &str,
    size: f32,
    x: f32,
    y: f32,
    digit_col: Color,
    star: Option<Color>,
    bold: bool,
) {
    if bold {
        text_bold(px, fonts, digits, size, x, y, digit_col, false);
    } else {
        text(px, fonts, digits, size, x, y, digit_col, false);
    }
    if let Some(star_col) = star {
        let sx = x + measure(fonts, digits, size);
        if bold {
            text_bold(px, fonts, "*", size, sx, y, star_col, false);
        } else {
            text(px, fonts, "*", size, sx, y, star_col, false);
        }
    }
}

fn col_place_text(
    px: &mut Pixmap,
    fonts: &Fonts,
    digits: &str,
    size: f32,
    x: f32,
    w: f32,
    y: f32,
    digit_col: Color,
    star: Option<Color>,
    right: bool,
) {
    if digits.is_empty() && star.is_none() {
        return;
    }
    let star_s = if star.is_some() { "*" } else { "" };
    let full = format!("{digits}{star_s}");
    let shown = ellipsize(fonts, &full, size, w.max(8.0));
    // If the column is too narrow for the star, drop it rather than paint a wrong color.
    let (digits_draw, star_draw) = if shown.ends_with('*') && star.is_some() {
        let mut d = shown;
        d.pop();
        (d, star)
    } else if shown.contains('*') {
        (shown.replace('*', ""), None)
    } else {
        (shown, None)
    };
    let tw = place_width(fonts, &digits_draw, size, star_draw);
    let tx = if right { x + w - tw } else { x };
    paint_place_at(
        px,
        fonts,
        &digits_draw,
        size,
        tx,
        y,
        digit_col,
        star_draw,
        false,
    );
}

fn telemetry_steer_col() -> Color {
    Color::from_rgba8(214, 214, 220, 230)
}


#[derive(Clone, Copy)]
struct MapFollowEase {
    angle: f32,
    subject: i32,
    live: bool,
}

thread_local! {
    static MAP_LAYER: RefCell<Option<(u64, Pixmap)>> = RefCell::new(None);
    static MAP_FOLLOW: Cell<MapFollowEase> = Cell::new(MapFollowEase {
        angle: 0.0,
        subject: 0,
        live: false,
    });
    static MINI_PX: RefCell<Option<Pixmap>> = RefCell::new(None);
}



pub fn draw(
    px: &mut Pixmap,
    fonts: &Fonts,
    snap: Option<&Snapshot>,
    cfg: &HudConfig,
    w: u32,
    h: u32,
    age: f32,
    restart_hint: bool,
    plugin_hint: bool,
    settings_hint: bool,
    after_widget: &mut dyn FnMut(WidgetId, &mut Pixmap),
) {
    crate::config::set_accent_rgb(cfg.primary);
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
            crate::i18n::t("Fully quit MX Bikes and start it again so the plugin can load"),
        );
    } else if restart_hint {
        top_banner(
            px,
            fonts,
            w,
            crate::i18n::t("Restart MX Bikes once so the HUD stays on top while you ride"),
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
                crate::i18n::t("Press F8 — turn on Show on overlay. Widgets appear on track."),
            );
        } else {
            top_banner(
                px,
                fonts,
                w,
                crate::i18n::t(
                    "Widgets appear on track. If you are racing, fully quit MX Bikes and start it again.",
                ),
            );
        }
        return;
    }
    if !cfg.any_overlay_widget() && !settings_hint {
        top_banner(
            px,
            fonts,
            w,
            crate::i18n::t("Press F8 — turn on Show on overlay"),
        );
    }

    let sw = w as f32;
    let sh = h as f32;
    // Stream surface: orange Ctrl-drag chrome only — never paint stream widgets on the game HWND.
    if cfg.edit_surface != EditSurface::Stream {
        RaceStore::refresh(s);
        RaceStore::with(|_| {
            let (flag, flag_grow) = if cfg[WidgetId::Dash].show || cfg[WidgetId::Flag].show {
                tick_display_flag(
                    s,
                    (cfg[WidgetId::Flag].show && cfg.flag.flag_yellow)
                        || (cfg[WidgetId::Dash].show && cfg.dash.dash_yellow),
                    (cfg[WidgetId::Flag].show && cfg.flag.flag_blue)
                        || (cfg[WidgetId::Dash].show && cfg.dash.dash_blue),
                    (cfg[WidgetId::Flag].show && cfg.flag.flag_red)
                        || (cfg[WidgetId::Dash].show && cfg.dash.dash_red),
                )
            } else {
                (DashFlag::None, 0.0)
            };
            draw_widgets(
                px,
                fonts,
                s,
                cfg,
                sw,
                sh,
                age,
                flag,
                flag_grow,
                sw,
                sh,
                None,
                |id, pixmap| {
                    after_widget(id, pixmap);
                    true
                },
            );
        });
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

/// Stream paint. Same widgets as [`draw`], and `after_widget` runs before the next one starts.
/// The game overlay keeps a single [`draw`] call.
pub fn draw_each<After>(
    px: &mut Pixmap,
    fonts: &Fonts,
    snap: Option<&Snapshot>,
    cfg: &HudConfig,
    w: u32,
    h: u32,
    age: f32,
    window_w: u32,
    window_h: u32,
    include: Option<&[bool; WidgetId::COUNT]>,
    after_widget: After,
) where
    After: FnMut(WidgetId, &mut Pixmap) -> bool,
{
    crate::config::set_accent_rgb(cfg.primary);
    clear_click_riders();
    px.fill(Color::TRANSPARENT);
    let Some(s) = snap else {
        return;
    };
    if !s.has_session_data() {
        if !cfg.any_overlay_widget() {
            top_banner(
                px,
                fonts,
                w,
                crate::i18n::t("Press F8 — turn on Show on overlay. Widgets appear on track."),
            );
        } else {
            top_banner(
                px,
                fonts,
                w,
                crate::i18n::t(
                    "Widgets appear on track. If you are racing, fully quit MX Bikes and start it again.",
                ),
            );
        }
        return;
    }
    if !cfg.any_overlay_widget() {
        top_banner(
            px,
            fonts,
            w,
            crate::i18n::t("Press F8 — turn on Show on overlay"),
        );
    }
    if cfg.edit_surface == EditSurface::Stream {
        return;
    }
    let sw = w as f32;
    let sh = h as f32;
    let raster_w = if window_w >= 64 { window_w as f32 } else { sw };
    let raster_h = if window_h >= 64 { window_h as f32 } else { sh };
    RaceStore::refresh(s);
    let (flag, flag_grow) = if cfg[WidgetId::Dash].show || cfg[WidgetId::Flag].show {
        tick_display_flag(
            s,
            (cfg[WidgetId::Flag].show && cfg.flag.flag_yellow)
                || (cfg[WidgetId::Dash].show && cfg.dash.dash_yellow),
            (cfg[WidgetId::Flag].show && cfg.flag.flag_blue)
                || (cfg[WidgetId::Dash].show && cfg.dash.dash_blue),
            (cfg[WidgetId::Flag].show && cfg.flag.flag_red)
                || (cfg[WidgetId::Dash].show && cfg.dash.dash_red),
        )
    } else {
        (DashFlag::None, 0.0)
    };
    draw_widgets(
        px,
        fonts,
        s,
        cfg,
        sw,
        sh,
        age,
        flag,
        flag_grow,
        raster_w,
        raster_h,
        include,
        after_widget,
    );
}

fn draw_widgets<After>(
    px: &mut Pixmap,
    fonts: &Fonts,
    s: &Snapshot,
    cfg: &HudConfig,
    sw: f32,
    sh: f32,
    age: f32,
    flag: DashFlag,
    flag_grow: f32,
    raster_w: f32,
    raster_h: f32,
    include: Option<&[bool; WidgetId::COUNT]>,
    mut after_widget: After,
) where
    After: FnMut(WidgetId, &mut Pixmap) -> bool,
{
    let included = |id: WidgetId| include.is_none_or(|mask| mask[id.idx()]);
    let delta = crate::delta::view_for(cfg.delta.delta_session);
    if s.show_standings != 0 && included(WidgetId::Standings) {
        RaceStore::with(|_| {
            let _style = push_style(
                fonts,
                cfg[WidgetId::Standings].bold,
                cfg[WidgetId::Standings].font,
            );
            draw_standings(px, fonts, s, cfg, sw, sh);
        });
        if !after_widget(WidgetId::Standings, px) {
            return;
        }
    }
    if s.show_relative != 0 && included(WidgetId::Relative) {
        RaceStore::with(|_| {
            let _style = push_style(
                fonts,
                cfg[WidgetId::Relative].bold,
                cfg[WidgetId::Relative].font,
            );
            draw_relative(px, fonts, s, cfg, sw, sh);
        });
        if !after_widget(WidgetId::Relative, px) {
            return;
        }
    }
    if s.show_map != 0 && included(WidgetId::Map) {
        RaceStore::with(|_| {
            let _style = push_style(fonts, cfg[WidgetId::Map].bold, cfg[WidgetId::Map].font);
            draw_map(px, fonts, s, cfg, sw, sh, age);
        });
        if !after_widget(WidgetId::Map, px) {
            return;
        }
    }
    if cfg[WidgetId::Minimap].show && included(WidgetId::Minimap) {
        RaceStore::with(|_| {
            let _style = push_style(
                fonts,
                cfg[WidgetId::Minimap].bold,
                cfg[WidgetId::Minimap].font,
            );
            draw_minimap(px, fonts, s, cfg, sw, sh, age, raster_w, raster_h);
        });
        if !after_widget(WidgetId::Minimap, px) {
            return;
        }
    }
    if cfg[WidgetId::Radar].show && included(WidgetId::Radar) {
        RaceStore::with(|_| {
            let _style = push_style(fonts, cfg[WidgetId::Radar].bold, cfg[WidgetId::Radar].font);
            draw_radar(px, fonts, s, cfg, sw, sh, age);
        });
        if !after_widget(WidgetId::Radar, px) {
            return;
        }
    }
    if cfg[WidgetId::Dash].show && included(WidgetId::Dash) {
        RaceStore::with(|_| {
            let _style = push_style(fonts, cfg[WidgetId::Dash].bold, cfg[WidgetId::Dash].font);
            let (dash_flag, dash_grow) = dash_wrap_flag(
                flag,
                flag_grow,
                cfg.dash.dash_yellow,
                cfg.dash.dash_blue,
                cfg.dash.dash_red,
            );
            draw_dash(px, fonts, s, cfg, sw, sh, dash_flag, dash_grow);
        });
        if !after_widget(WidgetId::Dash, px) {
            return;
        }
    }
    if cfg[WidgetId::Ticker].show && included(WidgetId::Ticker) {
        RaceStore::with(|_| {
            let _style = push_style(
                fonts,
                cfg[WidgetId::Ticker].bold,
                cfg[WidgetId::Ticker].font,
            );
            draw_ticker(px, fonts, s, cfg, sw, sh);
        });
        if !after_widget(WidgetId::Ticker, px) {
            return;
        }
    }
    if cfg[WidgetId::Sys].show && included(WidgetId::Sys) {
        RaceStore::with(|_| {
            let _style = push_style(fonts, cfg[WidgetId::Sys].bold, cfg[WidgetId::Sys].font);
            draw_sys(px, fonts, cfg, sw, sh);
        });
        if !after_widget(WidgetId::Sys, px) {
            return;
        }
    }
    if cfg.sector_visible() && included(WidgetId::Sector) {
        RaceStore::with(|_| {
            let _style = push_style(
                fonts,
                cfg[WidgetId::Sector].bold,
                cfg[WidgetId::Sector].font,
            );
            draw_sector(px, fonts, s, cfg, sw, sh);
        });
        if !after_widget(WidgetId::Sector, px) {
            return;
        }
    }
    if cfg.delta_visible() && included(WidgetId::Delta) {
        RaceStore::with(|_| {
            let _style = push_style(fonts, cfg[WidgetId::Delta].bold, cfg[WidgetId::Delta].font);
            draw_delta(px, fonts, cfg, sw, sh, delta);
        });
        if !after_widget(WidgetId::Delta, px) {
            return;
        }
    }
    if cfg[WidgetId::Flag].show && included(WidgetId::Flag) {
        RaceStore::with(|_| {
            let _style = push_style(fonts, cfg[WidgetId::Flag].bold, cfg[WidgetId::Flag].font);
            let (flag, flag_grow) = flag_widget_flag(
                flag,
                flag_grow,
                cfg.flag.flag_yellow,
                cfg.flag.flag_blue,
                cfg.flag.flag_red,
            );
            draw_flag(px, fonts, cfg, sw, sh, flag, flag_grow);
        });
        if !after_widget(WidgetId::Flag, px) {
            return;
        }
    }
    if cfg.stance_visible() && included(WidgetId::Stance) {
        RaceStore::with(|_| {
            let _style = push_style(
                fonts,
                cfg[WidgetId::Stance].bold,
                cfg[WidgetId::Stance].font,
            );
            draw_stance(px, fonts, cfg, sw, sh);
        });
        if !after_widget(WidgetId::Stance, px) {
            return;
        }
    }
    if cfg[WidgetId::Lean].show && included(WidgetId::Lean) {
        RaceStore::with(|_| {
            let _style = push_style(fonts, cfg[WidgetId::Lean].bold, cfg[WidgetId::Lean].font);
            draw_lean(px, fonts, s, cfg, sw, sh);
        });
        if !after_widget(WidgetId::Lean, px) {
            return;
        }
    }
    if cfg.gamepad_visible() && included(WidgetId::Gamepad) {
        RaceStore::with(|_| {
            let _style = push_style(
                fonts,
                cfg[WidgetId::Gamepad].bold,
                cfg[WidgetId::Gamepad].font,
            );
            draw_gamepad(px, fonts, cfg, sw, sh);
        });
        if !after_widget(WidgetId::Gamepad, px) {
            return;
        }
    }
    if cfg[WidgetId::Telemetry].show && included(WidgetId::Telemetry) {
        RaceStore::with(|_| {
            let _style = push_style(
                fonts,
                cfg[WidgetId::Telemetry].bold,
                cfg[WidgetId::Telemetry].font,
            );
            draw_telemetry(px, fonts, s, cfg, sw, sh);
        });
        if !after_widget(WidgetId::Telemetry, px) {
            return;
        }
    }
    if crate::pitboard::drawing(cfg[WidgetId::Pitboard].show, cfg.pit.pit_when)
        && included(WidgetId::Pitboard)
    {
        RaceStore::with(|_| {
            let _style = push_style(fonts, false, 100);
            draw_pitboard(px, fonts, s, cfg, sw, sh);
        });
        if !after_widget(WidgetId::Pitboard, px) {
            return;
        }
    }
    if cfg[WidgetId::Timer].show && included(WidgetId::Timer) {
        RaceStore::with(|_| {
            let _style = push_style(fonts, cfg[WidgetId::Timer].bold, cfg[WidgetId::Timer].font);
            draw_timer(px, fonts, s, cfg, sw, sh);
        });
        if !after_widget(WidgetId::Timer, px) {
            return;
        }
    }
}

fn try_rect(x: f32, y: f32, w: f32, h: f32) -> Option<Rect> {
    Rect::from_xywh(x, y, w, h)
}

fn top_banner(px: &mut Pixmap, fonts: &Fonts, w: u32, msg: &str) {
    let cx = w as f32 * 0.5;
    let bw = 680.0;
    if let Some(r) = try_rect(cx - bw * 0.5, 8.0, bw, 40.0) {
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
            fill_path(px, &glow, accent_a(90));
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
    ICON.get_or_init(|| {
        Pixmap::decode_png(include_bytes!("../../../icon-48.png")).expect("icon-48.png")
    })
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
        layout_box(
            px,
            s.map.x * sw,
            s.map.y * sh,
            s.map.w * sw,
            s.map.h * sh,
            false,
        );
    }
    if cfg[WidgetId::Radar].show {
        layout_box(
            px,
            cfg[WidgetId::Radar].rect.x * sw,
            cfg[WidgetId::Radar].rect.y * sh,
            cfg[WidgetId::Radar].rect.w * sw,
            cfg[WidgetId::Radar].rect.h * sh,
            false,
        );
    }
    if cfg[WidgetId::Ticker].show {
        layout_box(
            px,
            cfg[WidgetId::Ticker].rect.x * sw,
            cfg[WidgetId::Ticker].rect.y * sh,
            cfg[WidgetId::Ticker].rect.w * sw,
            cfg[WidgetId::Ticker].rect.h * sh,
            true,
        );
    }
    if cfg[WidgetId::Sys].show {
        layout_box(
            px,
            cfg[WidgetId::Sys].rect.x * sw,
            cfg[WidgetId::Sys].rect.y * sh,
            cfg[WidgetId::Sys].rect.w * sw,
            cfg[WidgetId::Sys].rect.h * sh,
            false,
        );
    }
    if cfg.sector_visible() {
        layout_box(
            px,
            cfg[WidgetId::Sector].rect.x * sw,
            cfg[WidgetId::Sector].rect.y * sh,
            cfg[WidgetId::Sector].rect.w * sw,
            cfg[WidgetId::Sector].rect.h * sh,
            false,
        );
    }
    if cfg.delta_visible() {
        layout_box(
            px,
            cfg[WidgetId::Delta].rect.x * sw,
            cfg[WidgetId::Delta].rect.y * sh,
            cfg[WidgetId::Delta].rect.w * sw,
            cfg[WidgetId::Delta].rect.h * sh,
            false,
        );
    }
    if cfg.stance_visible() {
        layout_box(
            px,
            cfg[WidgetId::Stance].rect.x * sw,
            cfg[WidgetId::Stance].rect.y * sh,
            cfg[WidgetId::Stance].rect.w * sw,
            cfg[WidgetId::Stance].rect.h * sh,
            false,
        );
    }
    if cfg[WidgetId::Lean].show {
        layout_box(
            px,
            cfg[WidgetId::Lean].rect.x * sw,
            cfg[WidgetId::Lean].rect.y * sh,
            cfg[WidgetId::Lean].rect.w * sw,
            cfg[WidgetId::Lean].rect.h * sh,
            false,
        );
    }
    if cfg.gamepad_visible() {
        layout_box(
            px,
            cfg[WidgetId::Gamepad].rect.x * sw,
            cfg[WidgetId::Gamepad].rect.y * sh,
            cfg[WidgetId::Gamepad].rect.w * sw,
            cfg[WidgetId::Gamepad].rect.h * sh,
            false,
        );
    }
    if cfg[WidgetId::Telemetry].show {
        layout_box(
            px,
            cfg[WidgetId::Telemetry].rect.x * sw,
            cfg[WidgetId::Telemetry].rect.y * sh,
            cfg[WidgetId::Telemetry].rect.w * sw,
            cfg[WidgetId::Telemetry].rect.h * sh,
            false,
        );
    }
    if cfg[WidgetId::Pitboard].show {
        layout_box(px, cfg[WidgetId::Pitboard].rect.x * sw, cfg[WidgetId::Pitboard].rect.y * sh, cfg[WidgetId::Pitboard].rect.w * sw, cfg[WidgetId::Pitboard].rect.h * sh, false);
    }
    if cfg[WidgetId::Timer].show {
        layout_box(
            px,
            cfg[WidgetId::Timer].rect.x * sw,
            cfg[WidgetId::Timer].rect.y * sh,
            cfg[WidgetId::Timer].rect.w * sw,
            cfg[WidgetId::Timer].rect.h * sh,
            false,
        );
    }
    if cfg[WidgetId::Flag].show {
        layout_box(
            px,
            cfg[WidgetId::Flag].rect.x * sw,
            cfg[WidgetId::Flag].rect.y * sh,
            cfg[WidgetId::Flag].rect.w * sw,
            cfg[WidgetId::Flag].rect.h * sh,
            false,
        );
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
        stroke_path(px, &path, accent_a(200), 1.4);
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
        if let Some(r) = try_rect(hx - 5.0, hy - 5.0, 10.0, 10.0) {
            fill_rect(px, r, Color::from_rgba8(8, 8, 10, 230));
        }
        if let Some(r) = try_rect(hx - 4.0, hy - 4.0, 8.0, 8.0) {
            fill_rect(px, r, accent());
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

fn background_alpha(pct: i32) -> u8 {
    ((pct.clamp(0, 100) as f32 / 100.0) * 255.0).round() as u8
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
        format!("{}s", ms / 1000)
    }
}


fn fill_focus_row(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, c: Color) {
    if let Some(rrt) = try_rect(x, y, w, h) {
        fill_rect(px, rrt, c);
    }
}

/// Colored row: 1px lines on the top and bottom, interior fading left to right.
/// `c`'s alpha is the left-edge wash (0 draws nothing). Lines are stronger and stay visible at the right.
fn fill_fade_row(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, c: Color) {
    let (r, g, b, wash) = color_bytes(c);
    if wash == 0 || w < 1.0 || h < 1.0 {
        return;
    }
    fill_h_fade(px, x, y, w, h, r, g, b, wash, 0);
    let edge = 1.0;
    let border_left = (wash as u32 * 3).min(255) as u8;
    let border_right = (wash as u32 * 24 / 10).min(255) as u8;
    fill_h_fade(px, x, y, w, edge, r, g, b, border_left, border_right);
    fill_h_fade(
        px,
        x,
        y + h - edge,
        w,
        edge,
        r,
        g,
        b,
        border_left,
        border_right,
    );
}

/// H-Standings focus card: same wash as a row, 1px frame, 4px rounded corners.
fn fill_fade_card(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, c: Color) {
    let (r, g, b, wash) = color_bytes(c);
    if wash == 0 || w < 2.0 || h < 2.0 {
        return;
    }
    let radius = 4.0_f32.min(w * 0.5).min(h * 0.5);
    let Some(path) = round_rect_path(x, y, w, h, radius) else {
        return;
    };
    let Some(mut mask) = Mask::new(px.width(), px.height()) else {
        return;
    };
    mask.fill_path(&path, FillRule::Winding, true, Transform::identity());
    fill_h_fade_mask(px, x, y, w, h, r, g, b, wash, 0, Some(&mask));
    // 1px strips clipped by the mask leave a square gap at a 4px arc. Stroke the
    // centerline inset so the arc stays inside the card layer (cards sit on y = 0).
    let border_left = (wash as u32 * 3).min(255) as u8;
    let border_right = (wash as u32 * 24 / 10).min(255) as u8;
    let inset = 0.5;
    let stroke_r = (radius - inset).max(0.0);
    let Some(frame) = round_rect_path(x + inset, y + inset, w - 1.0, h - 1.0, stroke_r) else {
        return;
    };
    let Some(shader) = LinearGradient::new(
        SkPoint::from_xy(x, y),
        SkPoint::from_xy(x + w, y),
        vec![
            GradientStop::new(0.0, Color::from_rgba8(r, g, b, border_left)),
            GradientStop::new(1.0, Color::from_rgba8(r, g, b, border_right)),
        ],
        SpreadMode::Pad,
        Transform::identity(),
    ) else {
        return;
    };
    let mut paint = Paint::default();
    paint.shader = shader;
    paint.anti_alias = true;
    let stroke = Stroke {
        width: 1.0,
        ..Stroke::default()
    };
    px.stroke_path(&frame, &paint, &stroke, Transform::identity(), None);
}

fn fill_h_fade(
    px: &mut Pixmap,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: u8,
    g: u8,
    b: u8,
    left: u8,
    right: u8,
) {
    let Some(rrt) = try_rect(x, y, w, h) else {
        return;
    };
    let Some(shader) = LinearGradient::new(
        SkPoint::from_xy(x, y),
        SkPoint::from_xy(x + w, y),
        vec![
            GradientStop::new(0.0, Color::from_rgba8(r, g, b, left)),
            GradientStop::new(1.0, Color::from_rgba8(r, g, b, right)),
        ],
        SpreadMode::Pad,
        Transform::identity(),
    ) else {
        return;
    };
    let mut paint = Paint::default();
    paint.shader = shader;
    paint.anti_alias = w >= 2.0 && h >= 2.0;
    px.fill_rect(rrt, &paint, Transform::identity(), None);
}

fn fill_h_fade_mask(
    px: &mut Pixmap,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: u8,
    g: u8,
    b: u8,
    left: u8,
    right: u8,
    mask: Option<&Mask>,
) {
    let Some(rrt) = try_rect(x, y, w, h) else {
        return;
    };
    let Some(shader) = LinearGradient::new(
        SkPoint::from_xy(x, y),
        SkPoint::from_xy(x + w, y),
        vec![
            GradientStop::new(0.0, Color::from_rgba8(r, g, b, left)),
            GradientStop::new(1.0, Color::from_rgba8(r, g, b, right)),
        ],
        SpreadMode::Pad,
        Transform::identity(),
    ) else {
        return;
    };
    let mut paint = Paint::default();
    paint.shader = shader;
    paint.anti_alias = w >= 2.0 && h >= 2.0;
    px.fill_rect(rrt, &paint, Transform::identity(), mask);
}

/// Spider-scaled wash: full chroma, alpha 52 at default 50% highlight.
fn highlight_alpha(opacity_pct: i32) -> u8 {
    ((52u32 * opacity_pct.clamp(0, 100) as u32) / 50).min(255) as u8
}

fn you_row_bg(opacity_pct: i32) -> Color {
    let [r, g, b] = accent_rgb();
    Color::from_rgba8(r, g, b, highlight_alpha(opacity_pct))
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
    Color::from_rgba8(59, 130, 246, highlight_alpha(opacity_pct))
}

fn lapped_row_bg(opacity_pct: i32) -> Color {
    Color::from_rgba8(239, 68, 68, highlight_alpha(opacity_pct))
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
/// `gap_laps` is laps behind the leader (0 for P1). Use it only when it says
/// they are ahead of you (`by_gap >= 1`): MX Bikes can keep a lapped rider on
/// the race lap, so after you complete another crossing you look a lap *up*
/// while still two down — that used to paint the leader red the second time
/// they went by. Do not use a negative gap alone for red: the leader can lap
/// someone behind you who you have not lapped yet.
///
/// Otherwise use pairwise `num_laps`, confirmed with continuous progress
/// (`num_laps + track_pos`) so an S/F straddle is not a lap.
fn other_laps_ahead(
    other: &crate::shm::Standing,
    me: &crate::shm::Standing,
    other_pos: f32,
    me_pos: f32,
) -> i32 {
    let by_gap = me.gap_laps - other.gap_laps;
    let by_laps = other.num_laps - me.num_laps;
    let by_progress = if by_laps == 0 {
        0
    } else {
        let delta = (other.num_laps as f32 + other_pos) - (me.num_laps as f32 + me_pos);
        if delta.round() as i32 == 0 {
            0
        } else {
            by_laps
        }
    };
    if by_gap >= 1 {
        by_gap
    } else {
        by_progress
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
        let ahead = other_laps_ahead(other, me, op, mp);
        let w = wrap_frac(op, mp);
        let dist_m = closing_m(s, w.abs());
        let span = catch_span_m(s);
        // Either side of the bike: keep blue/red through a pass while still nearby.
        if ahead >= 1 && dist_m > 2.0 && dist_m <= span {
            LapRel::LappingMe
        } else if ahead <= -1 && dist_m > 2.0 && dist_m <= span {
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

fn group_map_fill(cfg: &HudConfig, s: &Snapshot, rider: &Rider) -> Color {
    if cfg.groups_on_maps {
        if let Some((_, rgb)) = cfg.group_badge(&bytes_as_text(&rider.name)) {
            return Color::from_rgba8(rgb[0], rgb[1], rgb[2], 255);
        }
    }
    rider_dot_col(s, rider.race_num)
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

fn draw_count_track(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    cy: f32,
    n: usize,
    track: &str,
    ink: Color,
) {
    let count = format!("{n}");
    let track_col = accent();
    let icon_x = x + 12.0;
    let num_x = icon_x + 14.0;
    let count_w = (num_x - (x + 8.0) + measure(fonts, &count, 9.0) + 7.0).max(36.0);
    fill_skew(px, x + 8.0, cy + 3.0, count_w, 14.0, 4.0, track_col);
    icon(px, fonts, '\u{f553}', 8.0, icon_x, cy + 5.0, ink, false);
    text(px, fonts, &count, 9.0, num_x, cy + 4.5, ink, false);
    let tx = x + 8.0 + count_w + 4.0;
    fill_skew(
        px,
        tx,
        cy + 3.0,
        (measure(fonts, track, 10.0) + 14.0).max(36.0),
        14.0,
        4.0,
        track_col,
    );
    text(px, fonts, track, 10.0, tx + 8.0, cy + 4.5, ink, false);
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
    } else if has("ktm")
        || has("sx f")
        || has("xc f")
        || tok("sxf")
        || tok("xcf")
        || tok("exc")
        || tok("sx")
    {
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
        let h = hay
            .bytes()
            .fold(2166136261u32, |a, b| a.wrapping_mul(16777619) ^ b as u32);
        const PALETTE: [(u8, u8, u8); 6] = [
            (232, 196, 48),
            (48, 208, 232),
            (80, 214, 96),
            (232, 132, 48),
            (48, 128, 232),
            (168, 128, 255),
        ];
        let (r, g, b) = PALETTE[h as usize % PALETTE.len()];
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
    s.riders
        .iter()
        .take(n)
        .find(|r| r.race_num == subject)
        .map(|r| {
            let pose = rider_map_pose(s, r);
            SubjectPose {
                x: pose.x,
                z: pose.z,
                yaw: pose.yaw,
                vel_x: 0.0,
                vel_z: 0.0,
                crashed: r.crashed != 0,
                from_local: false,
            }
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

fn start_finish_fraction(s: &Snapshot) -> f32 {
    // A crossing we watched beats `sf_meters`, which stays 0 when the game never
    // sends a centerline — that would park the flag window at the centerline origin.
    let learned = START_FINISH_FRACTION_LEARNED.load(Ordering::Relaxed);
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
fn start_finish_uncalibrated(s: &Snapshot) -> bool {
    START_FINISH_FRACTION_LEARNED.load(Ordering::Relaxed) < 0 && s.poly_count <= 0 && s.sf_meters <= 0.0
}

fn distance_to_start_finish(pos: f32, sf: f32) -> f32 {
    let d = sf - pos;
    if d <= 0.0 {
        d + 1.0
    } else {
        d
    }
}

fn meters_to_start_finish(s: &Snapshot) -> Option<f32> {
    let pos = focus_track_pos(s);
    if pos < 0.0 {
        return None;
    }
    Some(distance_to_start_finish(pos, start_finish_fraction(s)) * lap_meters(s))
}

const FLAG_LINE_M: f32 = 80.0;

const FLAG_LINE_MIN_M: f32 = 4.0;

/// Crash ahead of you, close enough to need a yellow.
const FLAG_YELLOW_SPAN_M: f32 = 50.0;
/// Keep yellow up briefly after the crash bit or span edge flickers off, so blue/red
/// cannot flash during the same incident.
const YELLOW_HOLD_MS: i32 = 1_750;
/// `anim_now` ms of the last live nearby crash. `-1` when no hold is running.
static YELLOW_HOLD_AT: AtomicI32 = AtomicI32::new(-1);

/// Lapper closing from behind — shorter than `catch_span_m` so blue is not a whole-stretch warning.
const FLAG_BLUE_SPAN_M: f32 = 40.0;

/// Rider you are coming to lap, ahead and close — same window as blue.
const FLAG_RED_SPAN_M: f32 = 40.0;

fn approaching_line(s: &Snapshot) -> bool {
    meters_to_start_finish(s).is_some_and(|m| m > FLAG_LINE_MIN_M && m <= FLAG_LINE_M)
}

#[cfg(test)]
fn approaching_start_finish(s: &Snapshot) -> bool {
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
    let cand = START_FINISH_FRACTION_CANDIDATE.swap(ticks, Ordering::Relaxed);
    if cand < 0 {
        return;
    }
    let d = (frac - cand as f32 / 10_000.0).abs();
    if d.min(1.0 - d) <= SF_AGREE_FRAC {
        START_FINISH_FRACTION_LEARNED.store(ticks, Ordering::Relaxed);
    }
}

/// Watch the S/F line: learn where it is from your own crossings, remember whether you
/// have been round the far side, and keep the previous distance so the run-in must close.
fn note_line_progress(s: &Snapshot) {
    let Some(remain) = meters_to_start_finish(s) else {
        return;
    };
    let len = lap_meters(s);
    let laps = focus_num_laps(s);
    let prev_laps = START_FINISH_LEARN_LAPS.swap(laps, Ordering::Relaxed);
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
    if start_finish_uncalibrated(s) || !moving(s) || !approaching_line(s) {
        return false;
    }
    LAP_MID_SEEN.load(Ordering::Relaxed) == 1 && closing_on_line()
}

/// The last metres before the line, plus the stretch just after it. `approaching_line`
/// stops at `FLAG_LINE_MIN_M`, so without this the banner is None for ~4 m (and a
/// classification-lag beat past the line) — long enough for the hide animation to start.
fn across_the_line(s: &Snapshot) -> bool {
    if start_finish_uncalibrated(s) {
        return false;
    }
    meters_to_start_finish(s).is_some_and(|m| m <= FLAG_LINE_MIN_M || m >= lap_meters(s) - FLAG_LINE_M)
}

fn reset_flag_state() -> DashFlag {
    CHECKERED_LATCH.store(0, Ordering::Relaxed);
    WHITE_WAVE_LAP.store(-1, Ordering::Relaxed);
    RUN_IN_FLAG.store(0, Ordering::Relaxed);
    YELLOW_HOLD_AT.store(-1, Ordering::Relaxed);
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
/// finish — the leader taking the flag does not wave you off a lap you have already
/// started. Timed +1 still on the uncounted lap, and timed +2 before `2/2`, are the
/// exceptions (`laps_left`).
pub(crate) fn dash_race_flag(s: &Snapshot) -> DashFlag {
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
        let left = laps_left(s);
        // A glitched finish can latch checkered while extras still remain. Drop it so
        // live place updates are not paired with a stuck finished flag.
        if CHECKERED_LATCH.load(Ordering::Relaxed) == 1 {
            if left.is_some_and(|n| n > 0) {
                CHECKERED_LATCH.store(0, Ordering::Relaxed);
            } else {
                return DashFlag::Checkered;
            }
        }
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

/// Flag code stored on [`crate::race_store::RaceStore::race_flag`].
pub(crate) fn session_race_flag(s: &Snapshot) -> i32 {
    flag_code(dash_race_flag(s))
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
    YELLOW_HOLD_AT.store(-1, Ordering::Relaxed);
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
            let race = flag_from_code(RaceStore::with(|store| store.race_flag));
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

/// Inferred only. The game has no marshal flag field. Yellow is a nearby crash
/// (every live session — practice, warmup, race); blue is someone a lap up
/// closing from behind (`LapRel::LappingMe`); red is someone you are coming to
/// lap (`LapRel::LappedByMe`). Blue/red stay race-only.
fn caution_flag(s: &Snapshot, yellow: bool, blue: bool, red: bool) -> DashFlag {
    if s.on_track == 0 || prestart(s) {
        YELLOW_HOLD_AT.store(-1, Ordering::Relaxed);
        return DashFlag::None;
    }
    if yellow && yellow_held(nearby_crash(s)) {
        return DashFlag::Yellow;
    }
    if is_warmup(s) {
        return DashFlag::None;
    }
    if blue && being_lapped(s) {
        DashFlag::Blue
    } else if red && lapping_them(s) {
        DashFlag::Red
    } else {
        DashFlag::None
    }
}

/// Sticky yellow: arm on a live nearby crash, hold for `YELLOW_HOLD_MS` after it
/// drops so remount / span-edge jitter cannot hand the cloth to blue or red.
fn yellow_held(live: bool) -> bool {
    let now = now_ms();
    if live {
        YELLOW_HOLD_AT.store(now, Ordering::Relaxed);
        return true;
    }
    let at = YELLOW_HOLD_AT.load(Ordering::Relaxed);
    if at >= 0 && now - at <= YELLOW_HOLD_MS {
        return true;
    }
    if at >= 0 {
        YELLOW_HOLD_AT.store(-1, Ordering::Relaxed);
    }
    false
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
            if r.race_num <= 0 || r.race_num == focus || r.crashed != 0 {
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
            if r.race_num <= 0 || r.race_num == focus || r.crashed != 0 {
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

fn dash_wrap_flag(
    flag: DashFlag,
    grow: f32,
    yellow: bool,
    blue: bool,
    red: bool,
) -> (DashFlag, f32) {
    match flag {
        DashFlag::Yellow if yellow => (flag, grow),
        DashFlag::Blue if blue => (flag, grow),
        DashFlag::Red if red => (flag, grow),
        DashFlag::Yellow | DashFlag::Blue | DashFlag::Red => (DashFlag::None, 0.0),
        other => (other, grow),
    }
}

fn flag_widget_flag(
    flag: DashFlag,
    grow: f32,
    yellow: bool,
    blue: bool,
    red: bool,
) -> (DashFlag, f32) {
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

pub(crate) fn board_item(s: &Snapshot, cfg: &HudConfig, field: BoardField) -> Option<(char, String)> {
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
            BoardField::Position => {
                let pos = st.map(|r| r.position.max(0)).unwrap_or(0);
                format_place_digits(pos, true)
            }
            BoardField::ClassPos => {
                let pos = class_position(s);
                format_place_digits(pos, true)
            }
            BoardField::Session | BoardField::RaceTime | BoardField::Lap => race_progress_text(s),
            BoardField::LapsLeft => race_laps_left_text(s),
            BoardField::Track => {
                let t = bytes_as_text(&s.track_name);
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
            BoardField::Delta => {
                let best = dash_best_ms(s);
                let src = if s.current_lap_ms > 0 {
                    s.current_lap_ms
                } else {
                    s.last_lap_ms
                };
                if best <= 0 || src <= 0 {
                    "--".into()
                } else {
                    format_delta_ms(src - best)
                }
            }
            BoardField::Last => {
                let ms = st
                    .map(|r| r.last_lap_ms)
                    .filter(|ms| *ms > 0)
                    .unwrap_or(s.last_lap_ms);
                format_clock(ms)
            }
            BoardField::LapDiff => {
                let num = st.map(|r| r.race_num).unwrap_or_else(|| {
                    if s.focus_race_num > 0 {
                        s.focus_race_num
                    } else {
                        s.local_race_num
                    }
                });
                lap_diff_text(num)
            }
            BoardField::Current => format_clock(s.current_lap_ms),
            BoardField::Gap => st
                .map(|r| gap_leader_text(s, &race.field, r))
                .unwrap_or_else(|| "---".into()),
            BoardField::Engine => cfg.units.format_temp(s.engine_temp),
            BoardField::Penalty => format_penalty(st.map(|r| r.penalty_ms).unwrap_or(0)),
            BoardField::Server => {
                let name = bytes_as_text(&s.server_name);
                if name.is_empty() {
                    "--".into()
                } else {
                    name
                }
            }
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
    let focus_num = if s.focus_race_num > 0 {
        s.focus_race_num
    } else {
        s.local_race_num
    };
    for (i, field) in fields.iter().enumerate() {
        let Some((ch, label)) = board_item(s, cfg, *field) else {
            continue;
        };
        let star = match *field {
            BoardField::Position => place_star_col(penalty_place_delta(focus_num)),
            BoardField::ClassPos => place_star_col(penalty_class_place_delta(s, focus_num)),
            _ => None,
        };
        let ink = if *field == BoardField::LapDiff {
            lap_diff_ink(focus_num, text_col())
        } else {
            text_col()
        };
        let max_tw = (slot_w - 16.0).max(12.0);
        let label = ellipsize(fonts, &label, fsz, max_tw);
        let iw = if ch != '\0' {
            fonts.icons.metrics(ch, icon_s).advance_width + 4.0
        } else {
            0.0
        };
        let used = iw + place_width(fonts, &label, fsz, star);
        let sx = x + pad + slot_w * i as f32 + (slot_w - used) * 0.5;
        if ch != '\0' {
            icon(px, fonts, ch, icon_s, sx, ty + 0.5, text_col(), false);
        }
        paint_place_at(px, fonts, &label, fsz, sx + iw, ty, ink, star, false);
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

fn stroke_smooth_series(
    px: &mut Pixmap,
    pts: &[(f32, f32)],
    color: Color,
    width: f32,
    y_lo: f32,
    y_hi: f32,
) {
    if pts.len() < 2 {
        return;
    }
    let n = pts.len();
    let y_at = |j: i32| pts[j.clamp(0, n as i32 - 1) as usize].1;
    let mut smooth = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as i32;
        let y = (y_at(t - 2) + y_at(t - 1) * 2.0 + y_at(t) * 3.0 + y_at(t + 1) * 2.0 + y_at(t + 2))
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

fn fill_path_rule(px: &mut Pixmap, path: &Path, color: Color, rule: FillRule, mask: Option<&Mask>) {
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
fn push_chamfer_tb(
    pb: &mut PathBuilder,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    top_cut: f32,
    bot_cut: f32,
) {
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
    fill_path_rule(
        px,
        &frame,
        Color::from_rgba8(248, 248, 250, 255),
        FillRule::EvenOdd,
        None,
    );
    let clip = Mask::new(px.width(), px.height()).map(|mut m| {
        m.fill_path(&frame, FillRule::EvenOdd, true, Transform::identity());
        m
    });
    let dark = Color::from_rgba8(176, 176, 182, 255);
    let light = Color::from_rgba8(236, 236, 240, 255);
    let side_rows = ((d.h / border).round() as i32).max(1);
    draw_checkered_cells(
        px,
        ox,
        d.y,
        border,
        d.h,
        dark,
        light,
        side_rows,
        clip.as_ref(),
    );
    draw_checkered_cells(
        px,
        d.x + d.w,
        d.y,
        border,
        d.h,
        dark,
        light,
        side_rows,
        clip.as_ref(),
    );
    draw_checkered_cells(px, ox, d.y + d.h, ow, border, dark, light, 1, clip.as_ref());
}

fn draw_checkered_banner(
    px: &mut Pixmap,
    fonts: &Fonts,
    band: &Path,
    ox: f32,
    top_y: f32,
    ow: f32,
    top_h: f32,
    grow: f32,
) {
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
fn draw_white_banner(
    px: &mut Pixmap,
    fonts: &Fonts,
    band: &Path,
    ox: f32,
    top_y: f32,
    ow: f32,
    top_h: f32,
    grow: f32,
) {
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
    let yaw = if yaw.abs() > 6.5 {
        yaw.to_radians()
    } else {
        yaw
    };
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

fn screen_dir(
    to_px: &impl Fn(f32, f32) -> (f32, f32),
    wx: f32,
    wz: f32,
    hx: f32,
    hz: f32,
) -> (f32, f32) {
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

fn draw_dot_chevron(
    px: &mut Pixmap,
    x: f32,
    y: f32,
    r: f32,
    sdx: f32,
    sdy: f32,
    fill: Color,
    you: bool,
) {
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
    stroke_path(
        px,
        &path,
        color_alpha(fill, 160),
        if you { 1.8 } else { 1.4 },
    );
}

fn draw_rider_dot(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    r: f32,
    fill: Color,
    num: i32,
    race_num: i32,
    show_num: bool,
    you: bool,
    opacity: i32,
    num_ink: Option<Color>,
) {
    if show_num {
        numbered_dot(
            px, fonts, x, y, r, fill, num, race_num, true, you, opacity, num_ink,
        );
    } else {
        dot_body(px, x, y, r, fill, you, opacity);
    }
}

/// `opacity` 78 reproduces `base`. 100 clamps toward solid.
fn scaled_dot_alpha(base: u8, opacity: i32) -> u8 {
    let pct = opacity.clamp(0, 100) as u32;
    ((base as u32 * pct) / MAP_DOT_OPACITY_DEFAULT as u32).min(255) as u8
}

fn fill_dot_glow(px: &mut Pixmap, x: f32, y: f32, r: f32, fill: Color, you: bool, opacity: i32) {
    let wash = scaled_dot_alpha(if you { 200 } else { 165 }, opacity);
    let halo = scaled_dot_alpha(if you { 120 } else { 96 }, opacity);
    let ring = scaled_dot_alpha(if you { 255 } else { 230 }, opacity);
    fill_circle(
        px,
        x,
        y,
        r + if you { 4.4 } else { 3.3 },
        color_alpha(fill, halo),
    );
    fill_circle(px, x, y, r, color_alpha(fill, wash));
    stroke_circle(px, x, y, r, color_alpha(fill, ring), 1.0);
}

fn dot_body(px: &mut Pixmap, x: f32, y: f32, r: f32, fill: Color, you: bool, opacity: i32) {
    fill_dot_glow(px, x, y, r, fill, you, opacity);
}

fn you_dot_ink(mode: TableText) -> Color {
    match mode {
        TableText::White => Color::from_rgba8(248, 248, 250, 255),
        TableText::Black => Color::from_rgba8(16, 16, 18, 255),
    }
}

/// One step smaller once a penalty `*` shares the disc with the digits.
fn dot_label_scale(num: i32, with_star: bool) -> f32 {
    let step = if num >= 100 {
        2
    } else if num >= 10 {
        1
    } else {
        0
    };
    let step = if with_star { step + 1 } else { step };
    match step {
        0 => 1.22,
        1 => 1.08,
        2 => 0.92,
        _ => 0.78,
    }
}

fn numbered_dot(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    r: f32,
    fill: Color,
    num: i32,
    race_num: i32,
    show_num: bool,
    you: bool,
    opacity: i32,
    num_ink: Option<Color>,
) {
    dot_body(px, x, y, r, fill, you, opacity);
    if !show_num || num <= 0 {
        return;
    }
    let label = format!("{num}");
    let face = &fonts.semibold;
    let k = style_k().max(0.01);
    let star = place_star_col(penalty_place_delta(race_num));
    let size = dot_label_scale(num, star.is_some()) * r / k;
    let Some((digit_min_x, digit_min_y, digit_max_x, digit_max_y)) = ink_bounds(face, &label, size)
    else {
        return;
    };
    let (min_x, min_y, max_x, max_y, star_pen) = if star.is_some() {
        let advance = measure_font(face, &label, size * style_k());
        match ink_bounds(face, "*", size) {
            Some((star_min_x, star_min_y, star_max_x, star_max_y)) => (
                digit_min_x.min(advance + star_min_x),
                digit_min_y.min(star_min_y),
                digit_max_x.max(advance + star_max_x),
                digit_max_y.max(star_max_y),
                Some(advance),
            ),
            None => (digit_min_x, digit_min_y, digit_max_x, digit_max_y, None),
        }
    } else {
        (digit_min_x, digit_min_y, digit_max_x, digit_max_y, None)
    };
    let tx = (x - (min_x + max_x) * 0.5).round();
    let ty = (y - (min_y + max_y) * 0.5).round();
    draw_text(
        px,
        face,
        &label,
        size,
        tx,
        ty,
        num_ink.unwrap_or_else(|| ink_on(fill)),
        false,
        false,
    );
    if let (Some(star_col), Some(pen)) = (star, star_pen) {
        draw_text(px, face, "*", size, tx + pen, ty, star_col, false, false);
    }
}


fn place_rings_for_session(s: &Snapshot) -> bool {
    !is_warmup(s) && !is_practice_session(s)
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
    if !show_place || !place_rings_for_session(s) || mine <= 0 || theirs <= 0 {
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
    stroke_circle(px, x, y, r + 2.0, color_alpha(col, 70), 2.2);
    stroke_circle(px, x, y, r + 3.4, color_alpha(col, 220), 1.0);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RiderMark {
    None,
    Crash,
    Pit,
    Dns,
    Out,
    Dsq,
    Finish,
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

/// Status column / H-Standings trailing mark: finish, crash, DNS/OUT/DSQ, then pit.
fn standing_mark(s: &Snapshot, race_num: i32, crashed: bool) -> RiderMark {
    let Some(row) = s
        .standings
        .iter()
        .take(s.standing_count.max(0) as usize)
        .find(|st| st.race_num == race_num)
    else {
        if crashed || rider_crashed(s, race_num) {
            return RiderMark::Crash;
        }
        return RiderMark::None;
    };
    if crate::race_store::done_racing(row) {
        return RiderMark::Finish;
    }
    if crashed || rider_crashed(s, race_num) {
        return RiderMark::Crash;
    }
    match row.state {
        1 => RiderMark::Dns,
        3 => RiderMark::Out,
        4 => RiderMark::Dsq,
        _ if row.pit != 0 => RiderMark::Pit,
        _ => RiderMark::None,
    }
}

fn mark_key(mark: RiderMark) -> String {
    match mark {
        RiderMark::None => String::new(),
        RiderMark::Crash => "crash".into(),
        RiderMark::Pit => "pit".into(),
        RiderMark::Dns => "dns".into(),
        RiderMark::Out => "out".into(),
        RiderMark::Dsq => "dsq".into(),
        RiderMark::Finish => "fin".into(),
    }
}

fn mark_from_key(key: &str) -> RiderMark {
    match key {
        "crash" => RiderMark::Crash,
        "pit" => RiderMark::Pit,
        "dns" => RiderMark::Dns,
        "out" => RiderMark::Out,
        "dsq" => RiderMark::Dsq,
        "fin" => RiderMark::Finish,
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
        RiderMark::Finish => ('\u{f11e}', Color::from_rgba8(232, 232, 236, 255)),
    };
    let k = style_k().max(0.01);
    let size = (r * 1.05).clamp(8.0 * k, 13.0 * k) / k;
    let ix = x + r * 0.62;
    let iy = y + r * 0.18;
    icon(
        px,
        fonts,
        ch,
        size,
        ix + 0.8,
        iy + 0.8,
        Color::from_rgba8(8, 8, 10, 230),
        true,
    );
    icon(px, fonts, ch, size, ix, iy, col, true);
}

fn blit_circle_fit(dst: &mut Pixmap, src: &Pixmap, dx: f32, dy: f32, dest: f32) {
    let dest_px = dest.round().max(1.0) as i32;
    let src_w = src.width() as i32;
    let src_h = src.height() as i32;
    if src_w < 1 || src_h < 1 || dest_px < 1 {
        return;
    }
    let cr = dest_px as f32 * 0.5 - 0.5;
    let fade_start = cr * 0.58;
    let cr2 = cr * cr;
    let fade2 = fade_start * fade_start;
    let fade_span = (cr - fade_start).max(0.001);
    let center = dest_px as f32 * 0.5 - 0.5;
    let origin_x = dx.round() as i32;
    let origin_y = dy.round() as i32;
    let src_data = src.data();
    let dst_w = dst.width() as i32;
    let dst_h = dst.height() as i32;
    let dst_data = dst.data_mut();
    let scale_x = src_w as f32 / dest_px as f32;
    let scale_y = src_h as f32 / dest_px as f32;
    for sy in 0..dest_px {
        for sx in 0..dest_px {
            let fx = sx as f32 + 0.5;
            let fy = sy as f32 + 0.5;
            let dist2 = (fx - center) * (fx - center) + (fy - center) * (fy - center);
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
            let dx_ = origin_x + sx;
            let dy_ = origin_y + sy;
            if dx_ < 0 || dy_ < 0 || dx_ >= dst_w || dy_ >= dst_h {
                continue;
            }
            let sample = sample_pixmap(src_data, src_w, src_h, (fx - 0.5) * scale_x, (fy - 0.5) * scale_y);
            let di = ((dy_ * dst_w + dx_) * 4) as usize;
            let sa = (sample[3] as f32 / 255.0) * cover;
            let inv = 1.0 - sa;
            dst_data[di] = (sample[0] as f32 * cover + dst_data[di] as f32 * inv) as u8;
            dst_data[di + 1] = (sample[1] as f32 * cover + dst_data[di + 1] as f32 * inv) as u8;
            dst_data[di + 2] = (sample[2] as f32 * cover + dst_data[di + 2] as f32 * inv) as u8;
            dst_data[di + 3] = (sample[3] as f32 * cover + dst_data[di + 3] as f32 * inv) as u8;
        }
    }
}

fn sample_pixmap(data: &[u8], width: i32, height: i32, x: f32, y: f32) -> [u8; 4] {
    let x = x.clamp(0.0, (width - 1) as f32);
    let y = y.clamp(0.0, (height - 1) as f32);
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let x1 = (x0 + 1).min(width - 1);
    let y1 = (y0 + 1).min(height - 1);
    let tx = x - x0 as f32;
    let ty = y - y0 as f32;
    let texel = |px: i32, py: i32| {
        let index = ((py * width + px) * 4) as usize;
        [data[index], data[index + 1], data[index + 2], data[index + 3]]
    };
    let top_left = texel(x0, y0);
    let top_right = texel(x1, y0);
    let bottom_left = texel(x0, y1);
    let bottom_right = texel(x1, y1);
    let mut sample = [0u8; 4];
    for channel in 0..4 {
        let top = top_left[channel] as f32 * (1.0 - tx) + top_right[channel] as f32 * tx;
        let bottom = bottom_left[channel] as f32 * (1.0 - tx) + bottom_right[channel] as f32 * tx;
        sample[channel] = (top * (1.0 - ty) + bottom * ty).round() as u8;
    }
    sample
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
            dst_data[di + 1] =
                (src_data[si + 1] as f32 * cover + dst_data[di + 1] as f32 * inv) as u8;
            dst_data[di + 2] =
                (src_data[si + 2] as f32 * cover + dst_data[di + 2] as f32 * inv) as u8;
            dst_data[di + 3] =
                (src_data[si + 3] as f32 * cover + dst_data[di + 3] as f32 * inv) as u8;
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

struct CenterlineHit {
    x: f32,
    z: f32,
    forward: Option<(f32, f32)>,
    dist2: f32,
}

/// Nearest point on the centerline, plus a tangent 22 m ahead. `dist2` is meters squared.
/// Farther than 300 m still returns a hit so the minimap can fall back to the whole track.
fn nearest_centerline(s: &Snapshot, n: usize, px: f32, pz: f32) -> Option<CenterlineHit> {
    if n < 2 {
        return None;
    }
    let looped = {
        let dx = s.poly[0].x - s.poly[n - 1].x;
        let dz = s.poly[0].z - s.poly[n - 1].z;
        dx * dx + dz * dz < 400.0
    };
    let mut best = f32::MAX;
    let mut segment_end = 1usize;
    let mut along = 0.0f32;
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
            segment_end = i;
            along = t;
        }
    }
    if best == f32::MAX {
        return None;
    }
    let a = &s.poly[segment_end - 1];
    let b = &s.poly[segment_end % n];
    let x = a.x + (b.x - a.x) * along;
    let z = a.z + (b.z - a.z) * along;
    let forward = centerline_forward(s, n, looped, segment_end, x, z);
    Some(CenterlineHit {
        x,
        z,
        forward,
        dist2: best,
    })
}

fn centerline_forward(
    s: &Snapshot,
    n: usize,
    looped: bool,
    segment_end: usize,
    mut x: f32,
    mut z: f32,
) -> Option<(f32, f32)> {
    let origin_x = x;
    let origin_z = z;
    let mut remain = 22.0;
    let mut i = segment_end;
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
    let fx = x - origin_x;
    let fz = z - origin_z;
    let len = (fx * fx + fz * fz).sqrt();
    if len < 0.4 {
        return None;
    }
    Some((fx / len, fz / len))
}

fn track_forward(s: &Snapshot, n: usize, px: f32, pz: f32) -> Option<(f32, f32)> {
    let hit = nearest_centerline(s, n, px, pz)?;
    if hit.dist2 > 90_000.0 {
        return None;
    }
    hit.forward
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
    px.fill_path(
        &path,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}

struct PolyHit {
    wx: f32,
    wz: f32,
    ax: f32,
    az: f32,
    bx: f32,
    bz: f32,
}

struct RiderMapPose {
    x: f32,
    z: f32,
    yaw: f32,
}

/// Last world XZ that actually moved, and the `track_pos` at that moment.
#[derive(Clone, Copy)]
struct MapXzSample {
    x: f32,
    z: f32,
    track_pos: f32,
}

thread_local! {
    static MAP_XZ_WATCH: RefCell<HashMap<i32, MapXzSample>> = RefCell::new(HashMap::new());
}

/// World XZ has to move this far before the dot trusts a new coordinate.
const MAP_XZ_MOVE_M: f32 = 0.5;
/// Stuck XZ with this much along-track progress goes back to the centerline.
const MAP_XZ_STUCK_M: f32 = 8.0;

#[cfg(test)]
fn clear_map_xz_watch() {
    MAP_XZ_WATCH.with(|watch| watch.borrow_mut().clear());
}

fn world_map_pose(rider: &Rider) -> RiderMapPose {
    RiderMapPose {
        x: rider.x,
        z: rider.z,
        yaw: rider.yaw,
    }
}

fn centerline_map_pose(snapshot: &Snapshot, rider: &Rider) -> Option<RiderMapPose> {
    let point_count = snapshot.poly_count.max(0) as usize;
    let fraction = norm_track_pos(snapshot, rider.track_pos);
    let hit = poly_at_frac(snapshot, point_count, fraction)?;
    let step_x = hit.bx - hit.ax;
    let step_z = hit.bz - hit.az;
    let yaw = if step_x * step_x + step_z * step_z > 1.0e-8 {
        step_x.atan2(step_z)
    } else {
        rider.yaw
    };
    Some(RiderMapPose {
        x: hit.wx,
        z: hit.wz,
        yaw,
    })
}

/// Metres of lap between two `track_pos` samples, the short way around.
fn map_progress_m(snapshot: &Snapshot, from_pos: f32, to_pos: f32) -> f32 {
    let from_frac = norm_track_pos(snapshot, from_pos);
    let to_frac = norm_track_pos(snapshot, to_pos);
    if from_frac < 0.0 || to_frac < 0.0 {
        return 0.0;
    }
    let mut lap_fraction = (to_frac - from_frac).rem_euclid(1.0);
    if lap_fraction > 0.5 {
        lap_fraction = 1.0 - lap_fraction;
    }
    lap_fraction * lap_meters(snapshot)
}

/// True when world XZ is still a believable bike position. A coordinate that
/// sits still while `track_pos` runs on is frozen, and the dot should follow
/// the centerline instead.
fn map_xz_is_live(snapshot: &Snapshot, rider: &Rider) -> bool {
    MAP_XZ_WATCH.with(|watch| {
        let mut samples = watch.borrow_mut();
        if let Some(sample) = samples.get(&rider.race_num).copied() {
            let step_x = rider.x - sample.x;
            let step_z = rider.z - sample.z;
            let moved = (step_x * step_x + step_z * step_z).sqrt() >= MAP_XZ_MOVE_M;
            if !moved {
                return map_progress_m(snapshot, sample.track_pos, rider.track_pos) < MAP_XZ_STUCK_M;
            }
        }
        samples.insert(
            rider.race_num,
            MapXzSample {
                x: rider.x,
                z: rider.z,
                track_pos: rider.track_pos,
            },
        );
        true
    })
}

/// World XZ for a map/minimap dot when that coordinate is live, so a cut or an
/// off-track bike shows. Centerline from `track_pos` only when XZ is stuck
/// while progress keeps moving. At the gate / prestart, keep world XZ so
/// stalls do not pile onto one poly point.
fn rider_map_pose(snapshot: &Snapshot, rider: &Rider) -> RiderMapPose {
    let live = map_xz_is_live(snapshot, rider);
    if rider.track_pos < 0.0 || prestart(snapshot) || live {
        return world_map_pose(rider);
    }
    centerline_map_pose(snapshot, rider).unwrap_or_else(|| world_map_pose(rider))
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
        let u = if span > 0.001 {
            (target - d[i - 1]) / span
        } else {
            0.0
        };
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
    let lap = if s.track_length > 1.0 {
        s.track_length
    } else {
        dist
    };
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

fn draw_sf(
    px: &mut Pixmap,
    s: &Snapshot,
    n: usize,
    to_px: impl Fn(f32, f32) -> (f32, f32),
    track_px: f32,
) {
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
    let half = track_px * 0.5;
    let mut pb = PathBuilder::new();
    pb.move_to(hx - pxn * half, hy - pyn * half);
    pb.line_to(hx + pxn * half, hy + pyn * half);
    if let Some(path) = pb.finish() {
        stroke_path_butt(
            px,
            &path,
            Color::from_rgba8(8, 8, 10, 220),
            (track_px * 0.55).clamp(5.0, 9.0),
        );
        stroke_path_butt(px, &path, accent(), (track_px * 0.38).clamp(3.5, 6.5));
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
            stroke_dashed(
                px,
                &path,
                Color::from_rgba8(8, 8, 10, 200),
                thick + 1.0,
                dash,
                dash,
            );
            stroke_dashed(px, &path, violet, thick, dash, dash);
        }
        let mut side = if (hx - ccx) * pxn + (hy - ccy) * pyn >= 0.0 {
            1.0
        } else {
            -1.0
        };
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

fn stroke_path_butt(px: &mut Pixmap, path: &Path, color: Color, width: f32) {
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;
    let stroke = Stroke {
        width,
        line_cap: LineCap::Butt,
        line_join: LineJoin::Miter,
        ..Stroke::default()
    };
    px.stroke_path(path, &paint, &stroke, Transform::identity(), None);
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

fn thin_poly_pts(pts: &[(f32, f32)], min_d2: f32) -> Vec<(f32, f32)> {
    if pts.len() <= 2 {
        return pts.to_vec();
    }
    let mut out = Vec::with_capacity(64);
    out.push(pts[0]);
    let last = pts[pts.len() - 1];
    for &p in &pts[1..pts.len() - 1] {
        let prev = out[out.len() - 1];
        let dx = p.0 - prev.0;
        let dy = p.1 - prev.1;
        if dx * dx + dy * dy >= min_d2 {
            out.push(p);
        }
    }
    if out.last() != Some(&last) {
        out.push(last);
    }
    out
}

fn smooth_poly_pts(pts: &[(f32, f32)], close: bool) -> Vec<(f32, f32)> {
    if pts.len() < 3 {
        return pts.to_vec();
    }
    let n = pts.len();
    let at = |j: i32| -> (f32, f32) {
        if close {
            pts[j.rem_euclid(n as i32) as usize]
        } else {
            pts[j.clamp(0, n as i32 - 1) as usize]
        }
    };
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        if !close && (i == 0 || i == n - 1) {
            out.push(pts[i]);
            continue;
        }
        let t = i as i32;
        let a = at(t - 2);
        let b = at(t - 1);
        let c = at(t);
        let d = at(t + 1);
        let e = at(t + 2);
        out.push((
            (a.0 + b.0 * 2.0 + c.0 * 3.0 + d.0 * 2.0 + e.0) / 9.0,
            (a.1 + b.1 * 2.0 + c.1 * 3.0 + d.1 * 2.0 + e.1) / 9.0,
        ));
    }
    out
}

fn cubic_poly_path(pts: &[(f32, f32)], close: bool) -> Option<Path> {
    if pts.len() < 2 {
        return None;
    }
    let n = pts.len();
    let at = |i: isize| -> (f32, f32) {
        if close {
            pts[i.rem_euclid(n as isize) as usize]
        } else {
            pts[i.clamp(0, n as isize - 1) as usize]
        }
    };
    let mut pb = PathBuilder::new();
    pb.move_to(pts[0].0, pts[0].1);
    if n == 2 {
        pb.line_to(pts[1].0, pts[1].1);
        if close {
            pb.close();
        }
        return pb.finish();
    }
    let last = if close { n } else { n - 1 };
    for i in 0..last {
        let p0 = at(i as isize - 1);
        let p1 = at(i as isize);
        let p2 = at(i as isize + 1);
        let p3 = at(i as isize + 2);
        let c1x = p1.0 + (p2.0 - p0.0) / 6.0;
        let c1y = p1.1 + (p2.1 - p0.1) / 6.0;
        let c2x = p2.0 - (p3.0 - p1.0) / 6.0;
        let c2y = p2.1 - (p3.1 - p1.1) / 6.0;
        pb.cubic_to(c1x, c1y, c2x, c2y, p2.0, p2.1);
    }
    if close {
        pb.close();
    }
    pb.finish()
}

fn poly_is_looped(s: &Snapshot, n: usize) -> bool {
    if n < 2 {
        return false;
    }
    let dx = s.poly[0].x - s.poly[n - 1].x;
    let dz = s.poly[0].z - s.poly[n - 1].z;
    dx * dx + dz * dz < 400.0
}

fn stroke_smooth_track_run(px: &mut Pixmap, pts: &[(f32, f32)], close: bool, width: f32) {
    let thinned = thin_poly_pts(pts, 16.0);
    let smoothed = smooth_poly_pts(&thinned, close);
    if let Some(path) = cubic_poly_path(&smoothed, close) {
        stroke_path(px, &path, Color::from_rgba8(8, 8, 10, 220), width + 5.0);
        stroke_path(px, &path, Color::from_rgba8(248, 248, 252, 255), width);
    }
}

fn stroke_smooth_minimap_track(
    px: &mut Pixmap,
    s: &Snapshot,
    n: usize,
    origin_x: f32,
    origin_z: f32,
    radius_m: Option<f32>,
    track_px: f32,
    to_px: &impl Fn(f32, f32) -> (f32, f32),
) {
    if n < 2 {
        return;
    }
    let looped = poly_is_looped(s, n);
    if let Some(radius_m) = radius_m {
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
        let seg_count = if looped { n } else { n - 1 };
        let mut run: Vec<(f32, f32)> = Vec::new();
        for i in 0..seg_count {
            let a = &s.poly[i];
            let b = &s.poly[(i + 1) % n];
            if near_seg(a.x, a.z, b.x, b.z) {
                let (x0, y0) = to_px(a.x, a.z);
                let (x1, y1) = to_px(b.x, b.z);
                if run.is_empty() {
                    run.push((x0, y0));
                }
                run.push((x1, y1));
            } else if run.len() >= 2 {
                stroke_smooth_track_run(px, &run, false, track_px);
                run.clear();
            } else {
                run.clear();
            }
        }
        if run.len() >= 2 {
            stroke_smooth_track_run(px, &run, false, track_px);
        }
    } else {
        let mut pts = Vec::with_capacity(n);
        for p in s.poly.iter().take(n) {
            pts.push(to_px(p.x, p.z));
        }
        stroke_smooth_track_run(px, &pts, looped, track_px);
    }
}

#[cfg(test)]
#[path = "../tests/render_support.rs"]
mod render_support;
#[cfg(test)]
#[path = "../tests/render_chrome.rs"]
mod render_chrome;
#[cfg(test)]
#[path = "../tests/render_dash.rs"]
mod render_dash;
#[cfg(test)]
#[path = "../tests/render_standings.rs"]
mod render_standings;
#[cfg(test)]
#[path = "../tests/render_session.rs"]
mod render_session;
#[cfg(test)]
#[path = "../tests/render_flag.rs"]
mod render_flag;
#[cfg(test)]
#[path = "../tests/render_map.rs"]
mod render_map;
#[cfg(test)]
#[path = "../tests/render_radar.rs"]
mod render_radar;
#[cfg(test)]
#[path = "../tests/render_sector.rs"]
mod render_sector;
#[cfg(test)]
#[path = "../tests/render_gamepad.rs"]
mod render_gamepad;
#[cfg(test)]
#[path = "../tests/render_telemetry.rs"]
mod render_telemetry;
#[cfg(test)]
#[path = "../tests/render_lean.rs"]
mod render_lean;
#[cfg(test)]
#[path = "../tests/render_pitboard.rs"]
mod render_pitboard;
