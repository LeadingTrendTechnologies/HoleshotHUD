//! THESIS: Settings is a first-run on-switch, then a working board — not a Windows Settings clone.
//! OWN-WORLD: Charcoal stack, Holeshot Orange plaque, Exo 2 ExtraBold Italic, 6–10px rounds, no card shadows.
//! STORY: Rider hits F8, sees Show on overlay, turns widgets on, then edits columns and snap.
//! FIRST VIEWPORT: Top mode bar (Widgets / Profile / Settings / Groups / Feedback); Profile sub-nav Overview / Motos / MyMXB / mxb-ranked / CBR / Tracks; widget rail grouped Boards / Cockpit / Track; rail hides on Feedback; orange name plaque; Show on overlay on the right; Header/Footer are three slots.
//! FORM: Combined Show Plaque + Header Strip columns; seed settings-comp.
//! FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance.

use std::cell::RefCell;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

use tiny_skia::{
    Color, FillRule, Paint, Path, PathBuilder,
    Pixmap, PixmapPaint, Rect, Stroke, Transform,
};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{
    GetSysColor, COLOR_BTNFACE, COLOR_GRAYTEXT, COLOR_HIGHLIGHT, COLOR_HIGHLIGHTTEXT,
    COLOR_HOTLIGHT, COLOR_WINDOW, COLOR_WINDOWTEXT,
};
use windows::Win32::UI::Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW};
use windows::Win32::UI::WindowsAndMessaging::{
    SystemParametersInfoW,
    SPI_GETHIGHCONTRAST, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
};

use crate::config::{
    format_primary_color, update_config, with_config, BoardField,
    DashField, DotLabel, EditSurface, FontFamily, GamepadStyle, GamepadTheme, HudConfig, LeanStyle,
    RadarStyle, SettingsKey, SettingsTheme,
    StanceMode, StanceStyle, TableText, UnitKind, Units, WidgetId,
    DEFAULT_PRIMARY, PRIMARY_SWATCHES, RADAR_RANGE_MAX, RADAR_RANGE_MIN, SYS_PRESETS, SYS_PROC_MAX,
};
use mxbo_hud::pitboard::{add_design_slot, apply_json_names, apply_pack_colors, factory_pack, factory_places, normalize_places, pack_slot_names, pitboards_dir, remove_design_slot, slot_menu, slot_sample, write_pack, PitVar, PitWhen, FACTORY_ART, LEGACY_CATALOG, MAX_DESIGN_SLOTS};
use crate::render::{fill_rect, icon, measure, text, text_bold, Fonts};

mod app;
mod clear;
mod controls;
mod dispatch;
mod feedback;
mod groups;
mod pit_help;
mod profile;
mod cbr;
mod mymxb;
mod ranked;
mod reply;
mod review;
mod tracks;
mod whats_new;
mod widgets;
pub(crate) use app::*;
pub(crate) use clear::*;
pub(crate) use controls::*;
pub(crate) use dispatch::*;
pub(crate) use feedback::*;
pub(crate) use groups::{
    commit_group_name, dispatch_group, group_backspace, group_member_focused, group_name_focused,
    group_push, group_revert_focus, pane_groups, set_session_riders,
};
pub(crate) use pit_help::*;
pub(crate) use profile::*;
pub(crate) use reply::*;
pub(crate) use review::*;
pub(crate) use whats_new::*;
pub(crate) use widgets::*;

mod hit;
mod input;
mod paint;
mod window;
pub use input::handle_message;
pub use window::{apply_stance_bind, attach, dump_motos_shots, dump_whats_new, hide, host_create_pos, is_open, keep_above_overlay, listening_bind, open_whats_new, paint, persist_host_pos, show, toggle};
#[cfg(test)]
pub use window::dump_review_pages;
pub(crate) use hit::*;
pub(crate) use input::*;
pub(crate) use paint::*;
pub(crate) use window::*;

#[derive(Clone, Copy)]
struct Palette {
    bg: Color,
    side: Color,
    tab_on: Color,
    text: Color,
    muted: Color,
    dim: Color,
    row_line: Color,
    chip_hover: Color,
    accent: Color,
    accent_dim: Color,
    knob: Color,
    track_off: Color,
    btn_bg: Color,
    btn_border: Color,
    panel: Color,
    ink: Color,
    scrim: Color,
    menu: Color,
    menu_edge: Color,
    menu_edge_strong: Color,
    shadow: Color,
    track_idle: Color,
    nav_idle: Color,
    hover_wash: Color,
}

impl Palette {
    fn dark(rgb: [u8; 3]) -> Self {
        let [r, g, b] = rgb;
        Self {
            bg: Color::from_rgba8(24, 25, 29, 255),
            side: Color::from_rgba8(8, 8, 10, 255),
            tab_on: Color::from_rgba8(r, g, b, 28),
            text: Color::from_rgba8(244, 244, 247, 255),
            muted: Color::from_rgba8(140, 140, 148, 255),
            dim: Color::from_rgba8(132, 132, 140, 255),
            row_line: Color::from_rgba8(255, 255, 255, 12),
            chip_hover: Color::from_rgba8(46, 47, 54, 255),
            accent: Color::from_rgba8(r, g, b, 255),
            accent_dim: Color::from_rgba8(r, g, b, 36),
            knob: Color::from_rgba8(250, 250, 252, 255),
            track_off: Color::from_rgba8(112, 112, 120, 255),
            btn_bg: Color::from_rgba8(32, 32, 36, 255),
            btn_border: Color::from_rgba8(255, 255, 255, 22),
            panel: Color::from_rgba8(34, 35, 41, 255),
            ink: {
                let [ir, ig, ib] = crate::config::ink_on_rgb(rgb);
                Color::from_rgba8(ir, ig, ib, 255)
            },
            scrim: Color::from_rgba8(8, 8, 10, 204),
            menu: Color::from_rgba8(24, 24, 28, 255),
            menu_edge: Color::from_rgba8(255, 255, 255, 18),
            menu_edge_strong: Color::from_rgba8(255, 255, 255, 48),
            shadow: Color::from_rgba8(0, 0, 0, 90),
            track_idle: Color::from_rgba8(58, 58, 66, 255),
            nav_idle: Color::from_rgba8(210, 210, 216, 255),
            hover_wash: Color::from_rgba8(255, 255, 255, 10),
        }
    }

    fn light(rgb: [u8; 3]) -> Self {
        let [r, g, b] = rgb;
        Self {
            bg: Color::from_rgba8(242, 243, 245, 255),
            side: Color::from_rgba8(230, 231, 235, 255),
            tab_on: Color::from_rgba8(r, g, b, 36),
            text: Color::from_rgba8(16, 16, 18, 255),
            muted: Color::from_rgba8(72, 74, 84, 255),
            dim: Color::from_rgba8(88, 90, 100, 255),
            row_line: Color::from_rgba8(0, 0, 0, 14),
            chip_hover: Color::from_rgba8(220, 222, 228, 255),
            accent: Color::from_rgba8(r, g, b, 255),
            accent_dim: Color::from_rgba8(r, g, b, 40),
            knob: Color::from_rgba8(250, 250, 252, 255),
            track_off: Color::from_rgba8(168, 170, 178, 255),
            btn_bg: Color::from_rgba8(236, 237, 239, 255),
            btn_border: Color::from_rgba8(0, 0, 0, 22),
            panel: Color::from_rgba8(255, 255, 255, 255),
            ink: {
                let [ir, ig, ib] = crate::config::ink_on_rgb(rgb);
                Color::from_rgba8(ir, ig, ib, 255)
            },
            scrim: Color::from_rgba8(16, 16, 20, 140),
            menu: Color::from_rgba8(255, 255, 255, 255),
            menu_edge: Color::from_rgba8(0, 0, 0, 16),
            menu_edge_strong: Color::from_rgba8(0, 0, 0, 40),
            shadow: Color::from_rgba8(0, 0, 0, 50),
            track_idle: Color::from_rgba8(196, 198, 206, 255),
            nav_idle: Color::from_rgba8(48, 50, 58, 255),
            hover_wash: Color::from_rgba8(0, 0, 0, 8),
        }
    }

    fn high_contrast() -> Self {
        let window = sys_color(COLOR_WINDOW);
        let text = sys_color(COLOR_WINDOWTEXT);
        let hi = sys_color(COLOR_HIGHLIGHT);
        let hi_text = sys_color(COLOR_HIGHLIGHTTEXT);
        let btn = sys_color(COLOR_BTNFACE);
        let hot = sys_color(COLOR_HOTLIGHT);
        let gray = sys_color(COLOR_GRAYTEXT);
        Self {
            bg: window,
            side: btn,
            tab_on: hi,
            text,
            muted: text,
            dim: text,
            row_line: text,
            chip_hover: hi,
            accent: if hot.red() + hot.green() + hot.blue() > 0.01 {
                hot
            } else {
                hi
            },
            accent_dim: hi,
            knob: hi_text,
            track_off: gray,
            btn_bg: btn,
            btn_border: text,
            panel: btn,
            ink: hi_text,
            scrim: Color::from_rgba8(0, 0, 0, 160),
            menu: btn,
            menu_edge: text,
            menu_edge_strong: text,
            shadow: Color::from_rgba8(0, 0, 0, 90),
            track_idle: gray,
            nav_idle: text,
            hover_wash: hi,
        }
    }
}

fn sys_color(idx: windows::Win32::Graphics::Gdi::SYS_COLOR_INDEX) -> Color {
    let c = unsafe { GetSysColor(idx) };
    Color::from_rgba8(
        (c & 0xFF) as u8,
        ((c >> 8) & 0xFF) as u8,
        ((c >> 16) & 0xFF) as u8,
        255,
    )
}

fn high_contrast_on() -> bool {
    let mut hc = HIGHCONTRASTW {
        cbSize: std::mem::size_of::<HIGHCONTRASTW>() as u32,
        dwFlags: Default::default(),
        lpszDefaultScheme: windows::core::PWSTR::null(),
    };
    let ok = unsafe {
        SystemParametersInfoW(
            SPI_GETHIGHCONTRAST,
            hc.cbSize,
            Some((&mut hc as *mut HIGHCONTRASTW).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    ok.is_ok() && hc.dwFlags.contains(HCF_HIGHCONTRASTON)
}

thread_local! {
    static PALETTE: std::cell::Cell<Palette> = std::cell::Cell::new(Palette::dark(DEFAULT_PRIMARY));
}

fn refresh_palette() {
    let (rgb, theme) = with_config(|c| (c.primary, c.settings_theme));
    crate::config::set_accent_rgb(rgb);
    crate::config::set_settings_theme_light(theme == SettingsTheme::Light);
    PALETTE.with(|p| {
        p.set(if high_contrast_on() {
            Palette::high_contrast()
        } else if theme == SettingsTheme::Light {
            Palette::light(rgb)
        } else {
            Palette::dark(rgb)
        });
    });
}

fn palette() -> Palette {
    PALETTE.with(|p| p.get())
}

fn bg() -> Color {
    palette().bg
}

fn side() -> Color {
    palette().side
}

fn tab_on() -> Color {
    palette().tab_on
}

fn text_col() -> Color {
    palette().text
}

fn muted() -> Color {
    palette().muted
}

fn dim() -> Color {
    palette().dim
}

fn caution() -> Color {
    if high_contrast_on() {
        text_col()
    } else if crate::config::settings_theme_light() {
        // Dark amber — yellow fails contrast on light chrome.
        Color::from_rgba8(146, 86, 0, 255)
    } else {
        Color::from_rgba8(244, 214, 36, 255)
    }
}

fn row_line() -> Color {
    palette().row_line
}

fn chip_hover() -> Color {
    palette().chip_hover
}

fn accent() -> Color {
    palette().accent
}

fn accent_a(a: u8) -> Color {
    let c = accent();
    Color::from_rgba8(
        (c.red() * 255.0) as u8,
        (c.green() * 255.0) as u8,
        (c.blue() * 255.0) as u8,
        a,
    )
}

fn accent_hot() -> Color {
    let c = accent();
    let t = 0.12;
    Color::from_rgba8(
        ((c.red() * (1.0 - t) + t) * 255.0) as u8,
        ((c.green() * (1.0 - t) + t) * 255.0) as u8,
        ((c.blue() * (1.0 - t) + t) * 255.0) as u8,
        255,
    )
}

fn accent_dim() -> Color {
    palette().accent_dim
}

fn knob() -> Color {
    palette().knob
}

fn track_off() -> Color {
    palette().track_off
}

fn btn_bg() -> Color {
    palette().btn_bg
}

fn btn_border() -> Color {
    palette().btn_border
}

fn panel() -> Color {
    palette().panel
}

fn ink() -> Color {
    palette().ink
}

fn scrim() -> Color {
    palette().scrim
}

fn menu_fill() -> Color {
    palette().menu
}

fn menu_edge() -> Color {
    palette().menu_edge
}

fn menu_edge_strong() -> Color {
    palette().menu_edge_strong
}

fn shadow() -> Color {
    palette().shadow
}

fn track_idle() -> Color {
    palette().track_idle
}

fn nav_idle() -> Color {
    palette().nav_idle
}

fn hover_wash() -> Color {
    palette().hover_wash
}

fn titlebar_should_be_dark() -> bool {
    if high_contrast_on() {
        return false;
    }
    !crate::config::settings_theme_light()
}

fn sync_titlebar(hwnd: HWND) {
    unsafe {
        set_titlebar_dark(hwnd, titlebar_should_be_dark());
    }
}

const ROW_H: f32 = 48.0;

const ROW_GAP: f32 = 8.0;

const COL_GAP: f32 = 10.0;

struct PairGrid {
    x0: f32,
    x1: f32,
    cw: f32,
    y0: f32,
    y1: f32,
    left: bool,
}

impl PairGrid {
    fn new(x: f32, y: f32, w: f32) -> Self {
        let cw = ((w - COL_GAP) * 0.5).max(1.0);
        Self {
            x0: x,
            x1: x + cw + COL_GAP,
            cw,
            y0: y,
            y1: y,
            left: true,
        }
    }

    fn place(&mut self, next_y: impl FnOnce(f32, f32, f32) -> f32) {
        let (cx, cy) = if self.left {
            (self.x0, self.y0)
        } else {
            (self.x1, self.y1)
        };
        let y = next_y(cx, cy, self.cw);
        if self.left {
            self.y0 = y;
        } else {
            self.y1 = y;
        }
        self.left = !self.left;
    }

    fn end(self) -> f32 {
        self.y0.max(self.y1)
    }
}

struct SettingsUi {
    host: HWND,
    tab: Tab,
    last_widget: Tab,
    app_section: AppSection,
    profile_section: ProfileSection,
    hover: Option<Hit>,
    focus: Option<Hit>,
    hits: Vec<HitBox>,
    open_drop: Option<Drop>,
    drag: Option<ColDrag>,
    slide: Option<SlideDrag>,
    sv_drag: Option<(f32, f32, f32, f32)>,
    scroll: f32,
    content_h: f32,
    /// Max pane scroll from the last draw. Wheel uses this so it cannot overshoot
    /// and snap back when the window is taller than the old 520px viewport.
    scroll_max: f32,
    /// Motos list scroll to restore after Analyze Back.
    motos_list_scroll: f32,
    nav_scroll: f32,
    nav_content_h: f32,
    nav_top: f32,
    nav_bottom: f32,
    banner_dismissed: bool,
    whats_new_open: bool,
    pit_help_open: bool,
    whats_new_scroll: f32,
    whats_new_scroll_max: f32,
    reply_id: Option<String>,
    reply_scroll: f32,
    reply_scroll_max: f32,
    /// Pixel offset into the open dropdown's option list.
    drop_scroll: f32,
    /// Open menu hit target: x, y, w, view_h, content_h.
    drop_menu: Option<(f32, f32, f32, f32, f32)>,
    /// Sit button is waiting for the next pad press.
    bind_listen: bool,
    review_filter: crate::review::ListFilter,
    /// Back (or delete of the live moto) keeps the list until Review is opened again.
    review_stay_on_list: bool,
    analyze_id: Option<i64>,
    analyze_compare: i32,
    analyze_you_lap: i32,
    analyze_warmup: bool,
    analyze_scrub: f32,
    analyze_zoom: f32,
    analyze_pan_x: f32,
    analyze_pan_z: f32,
    analyze_scrubbing: bool,
    analyze_follow: bool,
    map_drag: Option<(f32, f32, f32, f32)>,
    profile_all_time: bool,
    profile_ranked_only: bool,
    /// Selected track name on Profile → Tracks detail.
    tracks_selected: Option<String>,
    tracks_zoom: f32,
    tracks_pan_x: f32,
    tracks_pan_z: f32,
    tracks_map_drag: Option<(f32, f32, f32, f32)>,
    clear_confirm: Option<ClearKind>,
    st_col_slides: ColSlides,
    rel_col_slides: ColSlides,
    /// Fixed Y of the column list (for drag slot hit-testing while rows animate).
    col_list_y: f32,
    col_list_n: usize,
    col_list_kind: Option<DragKind>,
}


unsafe impl Send for SettingsUi {}

static UI: Mutex<Option<SettingsUi>> = Mutex::new(None);

static RAISING: AtomicBool = AtomicBool::new(false);

struct PendingDrop {
    mx: f32,
    my: f32,
    /// Top of the control the menu is attached to. `my` is 6px under that control.
    anchor_top: f32,
    bw: f32,
    content_h: f32,
    open_hit: Hit,
    options: Vec<(Hit, String, bool)>,
}

thread_local! {
    static DROP_MENUS: RefCell<Vec<PendingDrop>> = RefCell::new(Vec::new());
    static COLOR_PICKER_ANCHOR: std::cell::Cell<Option<(f32, f32, f32, f32)>> =
        const { std::cell::Cell::new(None) };
    static PIT_DESIGN: RefCell<PitDesign> = RefCell::new(PitDesign {
        selected: 0,
        preview: None,
        dragging: false,
        name: String::new(),
        name_on: false,
        shown_board: String::new(),
        notice: String::new(),
    });
    static PIT_PLATE_GEN: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

struct PitDesign {
    selected: u8,
    preview: Option<(f32, f32, f32, f32)>,
    dragging: bool,
    name: String,
    name_on: bool,
    shown_board: String,
    notice: String,
}

fn pit_selected() -> u8 {
    PIT_DESIGN.with(|design| design.borrow().selected)
}

fn set_pit_selected(index: u8) {
    PIT_DESIGN.with(|design| design.borrow_mut().selected = index);
}

fn seed_pit_name(board: &str) {
    PIT_DESIGN.with(|design| {
        let mut design = design.borrow_mut();
        if design.name_on || design.shown_board == board {
            return;
        }
        design.name = board.to_string();
        design.shown_board = board.to_string();
    });
}

fn pit_name_draft() -> String {
    PIT_DESIGN.with(|design| design.borrow().name.clone())
}

fn pit_default_board() -> bool {
    with_config(|c| c.pit.pit_board.is_empty() && (c.pit.pit_art.is_empty() || c.pit.pit_art == FACTORY_ART))
}

fn board_name_from_picture(path: &std::path::Path) -> Result<String, String> {
    let draft = if pit_default_board() { String::new() } else { pit_name_draft() };
    mxbo_hud::pitboard::board_name_for_picture(&draft, path)
}

fn board_name_from_pack(path: &std::path::Path) -> Result<String, String> {
    let draft = if pit_default_board() { String::new() } else { pit_name_draft() };
    mxbo_hud::pitboard::board_name_for_pack(&draft, path)
}

fn pit_name_focused() -> bool {
    PIT_DESIGN.with(|design| design.borrow().name_on)
}

fn pit_notice() -> String {
    PIT_DESIGN.with(|design| design.borrow().notice.clone())
}

fn set_pit_notice(notice: &str) {
    PIT_DESIGN.with(|design| design.borrow_mut().notice = notice.to_string());
}

fn set_pit_name(name: &str) {
    PIT_DESIGN.with(|design| {
        let mut design = design.borrow_mut();
        design.name = name.to_string();
        design.shown_board = name.to_string();
    });
}

fn set_pit_name_focus(on: bool) {
    PIT_DESIGN.with(|design| design.borrow_mut().name_on = on);
}

fn commit_pit_name() -> bool {
    if pit_default_board() {
        return true;
    }
    let current = with_config(|c| c.pit.pit_board.clone());
    let Some(name) = mxbo_hud::pitboard::sanitize_board_name(&pit_name_draft()) else {
        set_pit_notice("That name cannot be used.");
        return false;
    };
    if name == current {
        set_pit_name(&current);
        set_pit_notice("");
        return true;
    }
    match mxbo_hud::pitboard::rename_saved_board(&current, &name) {
        Ok(art) => {
            update_config(|c| {
                c.pit.pit_board = name.clone();
                c.pit.pit_art = art;
            });
            set_pit_name(&name);
            set_pit_notice("");
            true
        }
        Err(msg) => {
            set_pit_notice(&msg);
            false
        }
    }
}

fn revert_pit_name() {
    let current = with_config(|c| c.pit.pit_board.clone());
    set_pit_name(&current);
    set_pit_notice("");
}

fn pit_name_push(ch: char) {
    if ch.is_control() || matches!(ch, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
        return;
    }
    PIT_DESIGN.with(|design| {
        let mut design = design.borrow_mut();
        if design.name.chars().count() >= 48 {
            return;
        }
        design.name.push(ch);
        design.notice.clear();
    });
}

fn pit_name_backspace() {
    PIT_DESIGN.with(|design| {
        let mut design = design.borrow_mut();
        design.name.pop();
        design.notice.clear();
    });
}

fn set_pit_preview(rect: Option<(f32, f32, f32, f32)>) {
    PIT_DESIGN.with(|design| design.borrow_mut().preview = rect);
}

pub(crate) fn invalidate_plate_preview() {
    PIT_PLATE_GEN.with(|gen| gen.set(gen.get().wrapping_add(1)));
}

fn plate_preview_gen() -> u32 {
    PIT_PLATE_GEN.with(|gen| gen.get())
}

pub(crate) fn queue_drop_menu(
    mx: f32,
    my: f32,
    anchor_top: f32,
    bw: f32,
    content_h: f32,
    open_hit: Hit,
    options: Vec<(Hit, String, bool)>,
) {
    DROP_MENUS.with(|menus| {
        menus.borrow_mut().push(PendingDrop {
            mx,
            my,
            anchor_top,
            bw,
            content_h,
            open_hit,
            options,
        });
    });
}

const SIDE_W: f32 = 204.0;

const TOP_H: f32 = 52.0;

const PRESET_STRIP_H: f32 = 52.0;

const PROFILE_SUBNAV_H: f32 = 44.0;

const UPDATE_BANNER_H: f32 = 46.0;

const UPDATE_BANNER_H_ADMIN: f32 = 64.0;

