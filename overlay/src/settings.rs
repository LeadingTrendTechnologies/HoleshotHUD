//! THESIS: Settings is a first-run on-switch, then a working board — not a Windows Settings clone.
//! OWN-WORLD: Charcoal stack, Holeshot Orange plaque, Exo 2 ExtraBold Italic, 6–10px rounds, no card shadows.
//! STORY: Rider hits F8, sees Show on overlay, turns widgets on, then edits columns and snap.
//! FIRST VIEWPORT: Top mode bar (Widgets / Settings / Feedback); widget rail grouped Boards / Track / Cockpit; rail hides on Settings and Feedback; orange name plaque; Show on overlay on the right; Header/Footer are three slots.
//! FORM: Combined Show Plaque + Header Strip columns; seed settings-comp.
//! FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance.

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tiny_skia::{Color, FillRule, LineCap, LineJoin, Paint, Path, PathBuilder, Pixmap, PixmapPaint, Rect, Stroke, Transform};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateSolidBrush, DeleteObject, EndPaint, FillRect, GetDC, GetSysColor, ReleaseDC, ScreenToClient,
    SetDIBitsToDevice, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, COLOR_BTNFACE, COLOR_GRAYTEXT, COLOR_HIGHLIGHT,
    COLOR_HIGHLIGHTTEXT, COLOR_HOTLIGHT, COLOR_WINDOW, COLOR_WINDOWTEXT, DIB_RGB_COLORS, HGDIOBJ, PAINTSTRUCT,
};
use windows::Win32::UI::Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW, NotifyWinEvent};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, ReleaseCapture, SetCapture, VK_CONTROL, VK_DOWN, VK_ESCAPE, VK_LEFT, VK_RETURN, VK_RIGHT,
    VK_SHIFT, VK_SPACE, VK_TAB, VK_UP,
};
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, GetClientRect, GetForegroundWindow, GetWindowThreadProcessId, IsIconic,
    IsWindowVisible, LoadCursorW, SetCursor, SetForegroundWindow, SetWindowPos, SetWindowTextW,
    ShowWindow,
    SystemParametersInfoW, EVENT_OBJECT_FOCUS, EVENT_OBJECT_NAMECHANGE, HWND_NOTOPMOST, HWND_TOPMOST,
    IDC_ARROW, IDC_HAND, IDC_IBEAM, IDC_SIZEALL, OBJID_CLIENT, SPI_GETHIGHCONTRAST, SW_HIDE, SW_RESTORE, SW_SHOW,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, WM_CHAR,
    WM_ERASEBKGND, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_PAINT, WM_SETCURSOR,
};

use crate::config::{
    update_config, with_config, BoardField, DashField, DotLabel, FontFamily, HudConfig, RelField,
    SettingsKey, SnapAlign, StField, StanceBind, StanceMode, StanceStyle, TableText, Units, WidgetId, COL_W_MAX,
    COL_W_MIN,
};
use crate::render::{fill_rect, icon, measure, text, Fonts};

#[derive(Clone, Copy)]
struct Pal {
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
}

impl Pal {
    fn dark() -> Self {
        Self {
            bg: Color::from_rgba8(24, 25, 29, 255),
            side: Color::from_rgba8(8, 8, 10, 255),
            tab_on: Color::from_rgba8(255, 140, 36, 28),
            text: Color::from_rgba8(244, 244, 247, 255),
            muted: Color::from_rgba8(140, 140, 148, 255),
            dim: Color::from_rgba8(132, 132, 140, 255),
            row_line: Color::from_rgba8(255, 255, 255, 12),
            chip_hover: Color::from_rgba8(46, 47, 54, 255),
            accent: Color::from_rgba8(255, 140, 36, 255),
            accent_dim: Color::from_rgba8(255, 140, 36, 36),
            knob: Color::from_rgba8(250, 250, 252, 255),
            track_off: Color::from_rgba8(112, 112, 120, 255),
            btn_bg: Color::from_rgba8(32, 32, 36, 255),
            btn_border: Color::from_rgba8(255, 255, 255, 22),
            panel: Color::from_rgba8(34, 35, 41, 255),
            ink: Color::from_rgba8(20, 12, 4, 255),
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
            accent: if hot.red() + hot.green() + hot.blue() > 0.01 { hot } else { hi },
            accent_dim: hi,
            knob: hi_text,
            track_off: gray,
            btn_bg: btn,
            btn_border: text,
            panel: btn,
            ink: hi_text,
        }
    }
}

fn sys_color(idx: windows::Win32::Graphics::Gdi::SYS_COLOR_INDEX) -> Color {
    let c = unsafe { GetSysColor(idx) };
    Color::from_rgba8((c & 0xFF) as u8, ((c >> 8) & 0xFF) as u8, ((c >> 16) & 0xFF) as u8, 255)
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
    static PAL: std::cell::Cell<Pal> = std::cell::Cell::new(Pal::dark());
}

fn refresh_palette() {
    PAL.with(|p| {
        p.set(if high_contrast_on() {
            Pal::high_contrast()
        } else {
            Pal::dark()
        });
    });
}

fn pal() -> Pal {
    PAL.with(|p| p.get())
}

fn bg() -> Color { pal().bg }
fn side() -> Color { pal().side }
fn tab_on() -> Color { pal().tab_on }
fn text_col() -> Color { pal().text }
fn muted() -> Color { pal().muted }
fn dim() -> Color { pal().dim }
fn caution() -> Color {
    if high_contrast_on() {
        text_col()
    } else {
        Color::from_rgba8(244, 214, 36, 255)
    }
}
fn row_line() -> Color { pal().row_line }
fn chip_hover() -> Color { pal().chip_hover }
fn accent() -> Color { pal().accent }
fn accent_dim() -> Color { pal().accent_dim }
fn knob() -> Color { pal().knob }
fn track_off() -> Color { pal().track_off }
fn btn_bg() -> Color { pal().btn_bg }
fn btn_border() -> Color { pal().btn_border }
fn panel() -> Color { pal().panel }
fn ink() -> Color { pal().ink }
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    App,
    Feedback,
    Standings,
    Relative,
    Map,
    Minimap,
    Radar,
    Dash,
    Ticker,
    Sys,
    Sector,
    Delta,
    Stance,
    Flag,
}

impl Tab {
    fn is_widget(self) -> bool {
        !matches!(self, Tab::App | Tab::Feedback)
    }

    fn is_labs(self) -> bool {
        matches!(self, Tab::Sector | Tab::Delta)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Hit {
    TabWidgets,
    TabApp,
    TabFeedback,
    TabSt,
    TabRel,
    TabMap,
    TabMini,
    TabRadar,
    TabDash,
    TabTicker,
    TabSys,
    TabSector,
    TabDelta,
    TabStance,
    TabFlag,
    StShow,
    RelShow,
    MapShow,
    MiniShow,
    RadarShow,
    DashShow,
    DashRev,
    DashSimple,
    TickerShow,
    SysShow,
    SectorShow,
    SectorLive,
    DeltaShow,
    TrackPbClear,
    StanceShow,
    StanceShowSit,
    FlagShow,
    FlagYellow,
    FlagBlue,
    FeatureSector,
    ShowPresence,
    HighlightFriends,
    TickerTitle,
    TickerAutoscroll,
    StPos,
    StNum,
    StName,
    StGap,
    StLaps,
    StCurrent,
    StBest,
    StLast,
    StStatus,
    StBike,
    StPenalty,
    StCrashed,
    StInterval,
    RelNum,
    RelName,
    RelGap,
    RelLaps,
    RelCurrent,
    RelPos,
    RelBike,
    RelPenalty,
    RelInterval,
    RelCrashed,
    RelBest,
    RelLast,
    MapOthers,
    MapSf,
    MapSectors,
    MapArrows,
    MapCrown,
    MapPlace,
    MapNumbers,
    MapDotOpen,
    MapDotNum,
    MapDotPos,
    MiniOthers,
    MiniSf,
    MiniSectors,
    MiniArrows,
    MiniCrown,
    MiniPlace,
    MiniNumbers,
    MiniDotOpen,
    MiniDotNum,
    MiniDotPos,
    RadarSides,
    RadarRear,
    RadarRings,
    StBg,
    StHl,
    StStripe,
    StTextOpen,
    StTextWhite,
    StTextBlack,
    RelBg,
    RelHl,
    RelStripe,
    RelTextOpen,
    RelTextWhite,
    RelTextBlack,
    MapBg,
    MiniBg,
    MiniZoom,
    RadarBg,
    DashBg,
    TickerBg,
    SysBg,
    SectorBg,
    DeltaBg,
    StanceBg,
    FlagBg,
    StDec,
    StInc,
    RelDec,
    RelInc,
    TickerDec,
    TickerInc,
    StDrag(u8),
    RelDrag(u8),
    StW(u8),
    RelW(u8),
    Font(WidgetId),
    Bold(WidgetId),
    Snap(WidgetId, SnapAlign),
    FontOpen,
    FontSegoe,
    FontArial,
    FontTahoma,
    FontRoboto,
    FontExo2,
    FontTeko,
    FontGoldman,
    FontMontserrat,
    UnitsOpen,
    UnitsMetric,
    UnitsImperial,
    SettingsKeyOpen,
    SettingsKeyPick(SettingsKey),
    StanceBindOpen,
    StanceModeOpen,
    StanceModePick(StanceMode),
    StanceStyleOpen,
    StanceStylePick(StanceStyle),
    StanceReset,
    DashFootOpen(u8),
    DashFootPick(u8, DashField),
    TickerFootOpen(u8),
    TickerFootPick(u8, BoardField),
    InfoOpen(InfoBar, u8),
    InfoPick(InfoBar, u8, BoardField),
    UpdateCheck,
    UpdateInstall,
    UpdateBanner,
    UpdateBannerDismiss,
    WhatsNewOpen,
    WhatsNewDismiss,
    WhatsNewScrim,
    WhatsNewPanel,
    ReplyDismiss,
    ReplySend,
    ReplyText,
    ReplyScrim,
    ReplyPanel,
    StartWithWindows,
    MinimizeOnClose,
    CloseWithGame,
    OpenWithGame,
    AutoUpdateOnLaunch,
    QuitApp,
    Uninstall,
    GameFolder,
    FbRate,
    FbBug,
    FbFeature,
    FbStar(u8),
    FbText,
    FbAttach,
    FbSend,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum InfoBar {
    StHead,
    StFoot,
    RelHead,
    RelFoot,
}

#[derive(Clone, Copy)]
struct HitBox {
    id: Hit,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Drop {
    MapDot,
    MiniDot,
    FontFamily,
    Units,
    SettingsKey,
    StanceMode,
    StanceStyle,
    StText,
    RelText,
    DashFoot(u8),
    TickerFoot(u8),
    Info(InfoBar, u8),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DragKind {
    St,
    Rel,
}

#[derive(Clone, Copy)]
struct ColDrag {
    kind: DragKind,
    from: u8,
    over: u8,
}

#[derive(Clone, Copy)]
struct SlideDrag {
    hit: Hit,
    x: f32,
    w: f32,
    min: i32,
    max: i32,
}

struct SettingsUi {
    host: HWND,
    tab: Tab,
    last_widget: Tab,
    hover: Option<Hit>,
    focus: Option<Hit>,
    hits: Vec<HitBox>,
    open_drop: Option<Drop>,
    drag: Option<ColDrag>,
    slide: Option<SlideDrag>,
    scroll: f32,
    content_h: f32,
    /// Max pane scroll from the last draw. Wheel uses this so it cannot overshoot
    /// and snap back when the window is taller than the old 520px viewport.
    scroll_max: f32,
    nav_scroll: f32,
    nav_content_h: f32,
    nav_top: f32,
    nav_bottom: f32,
    banner_dismissed: bool,
    whats_new_open: bool,
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
}

unsafe impl Send for SettingsUi {}

static UI: Mutex<Option<SettingsUi>> = Mutex::new(None);
static RAISING: AtomicBool = AtomicBool::new(false);

struct PendingDrop {
    mx: f32,
    my: f32,
    bw: f32,
    content_h: f32,
    open_hit: Hit,
    options: Vec<(Hit, &'static str, bool)>,
}

thread_local! {
    static DROP_MENUS: RefCell<Vec<PendingDrop>> = RefCell::new(Vec::new());
}

const SIDE_W: f32 = 204.0;
const TOP_H: f32 = 52.0;
const UPDATE_BANNER_H: f32 = 46.0;
const UPDATE_BANNER_H_ADMIN: f32 = 64.0;

pub fn attach(host: HWND) {
    unsafe {
        dark_titlebar(host);
    }
    *UI.lock().unwrap() = Some(SettingsUi {
        host,
        tab: Tab::Standings,
        last_widget: Tab::Standings,
        hover: None,
        focus: None,
        hits: Vec::new(),
        open_drop: None,
        drag: None,
        slide: None,
        scroll: 0.0,
        content_h: 0.0,
        scroll_max: 0.0,
        nav_scroll: 0.0,
        nav_content_h: 0.0,
        nav_top: 0.0,
        nav_bottom: 0.0,
        banner_dismissed: false,
        whats_new_open: false,
        whats_new_scroll: 0.0,
        whats_new_scroll_max: 0.0,
        reply_id: None,
        reply_scroll: 0.0,
        reply_scroll_max: 0.0,
        drop_scroll: 0.0,
        drop_menu: None,
        bind_listen: false,
    });
}

pub fn show(host: HWND) {
    unsafe {
        force_to_front(host);
    }
    crate::feedback::refresh();
}

pub fn hide(host: HWND) {
    unsafe {
        let _ = ShowWindow(host, SW_HIDE);
    }
}

pub fn toggle(host: HWND) {
    if is_open() {
        hide(host);
    } else {
        show(host);
    }
}

/// Open the What's new modal for this build's changelog. No-op if there are no notes.
pub fn open_whats_new() {
    if crate::changelog::modal_notes().is_none() {
        return;
    }
    if let Some(ui) = UI.lock().unwrap().as_mut() {
        ui.whats_new_open = true;
        ui.whats_new_scroll = 0.0;
        ui.focus = Some(Hit::WhatsNewDismiss);
        ui.open_drop = None;
        ui.drop_scroll = 0.0;
        ui.drop_menu = None;
        ui.bind_listen = false;
    }
}

/// Paint the next What's new board to a PNG (Settings chrome + modal).
pub fn dump_whats_new(path: &std::path::Path) -> Result<crate::changelog::Notes, String> {
    let notes = crate::changelog::next_notes().ok_or_else(|| "No Unreleased or current notes.".to_string())?;
    refresh_palette();
    let fonts = Fonts::for_family(FontFamily::Exo2)
        .or_else(Fonts::load)
        .ok_or_else(|| "Need Exo 2 to paint What's new.".to_string())?;
    *UI.lock().unwrap() = Some(SettingsUi {
        host: HWND::default(),
        tab: Tab::App,
        last_widget: Tab::Standings,
        hover: None,
        focus: None,
        hits: Vec::new(),
        open_drop: None,
        drag: None,
        slide: None,
        scroll: 0.0,
        content_h: 0.0,
        scroll_max: 0.0,
        nav_scroll: 0.0,
        nav_content_h: 0.0,
        nav_top: 0.0,
        nav_bottom: 0.0,
        banner_dismissed: false,
        whats_new_open: true,
        whats_new_scroll: 0.0,
        whats_new_scroll_max: 0.0,
        reply_id: None,
        reply_scroll: 0.0,
        reply_scroll_max: 0.0,
        drop_scroll: 0.0,
        drop_menu: None,
        bind_listen: false,
    });
    let mut px = Pixmap::new(1000, 720).ok_or_else(|| "Could not allocate preview.".to_string())?;
    draw(&mut px, &fonts, 1000.0, 720.0);
    *UI.lock().unwrap() = None;
    if let Some(dir) = path.parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
    }
    let png = px.encode_png().map_err(|e| e.to_string())?;
    std::fs::write(path, png).map_err(|e| e.to_string())?;
    Ok(notes)
}

fn dismiss_whats_new() {
    let ver = crate::update::current_version().to_string();
    crate::config::update_config(|c| c.whats_new_seen = ver);
    if let Some(ui) = UI.lock().unwrap().as_mut() {
        ui.whats_new_open = false;
        ui.whats_new_scroll = 0.0;
        ui.focus = None;
    }
}

fn dismiss_reply() {
    let id = UI.lock().unwrap().as_ref().and_then(|u| u.reply_id.clone());
    if let Some(id) = id {
        crate::feedback::dismiss_reply(&id);
    }
    crate::feedback::set_compose_focus(false);
    if let Some(ui) = UI.lock().unwrap().as_mut() {
        ui.reply_id = None;
        ui.reply_scroll = 0.0;
        ui.focus = None;
    }
}

/// Settings is restored and on screen (not minimized to the tray).
pub fn is_open() -> bool {
    let host = {
        let ui = UI.lock().unwrap();
        let Some(ui) = ui.as_ref() else {
            return false;
        };
        ui.host
    };
    unsafe { !IsIconic(host).as_bool() && IsWindowVisible(host).as_bool() }
}

/// Sit-button row is waiting for a pad press.
pub fn listening_bind() -> bool {
    let (host, listen) = {
        let ui = UI.lock().unwrap();
        let Some(ui) = ui.as_ref() else {
            return false;
        };
        (ui.host, ui.bind_listen)
    };
    if !listen {
        return false;
    }
    let visible = unsafe { !IsIconic(host).as_bool() && IsWindowVisible(host).as_bool() };
    if !visible {
        if let Some(ui) = UI.lock().unwrap().as_mut() {
            ui.bind_listen = false;
        }
        return false;
    }
    true
}

pub fn apply_stance_bind(bind: StanceBind) {
    update_config(|c| c.stance_bind = bind);
    if let Some(ui) = UI.lock().unwrap().as_mut() {
        ui.bind_listen = false;
    }
}

/// Keep settings above the game overlay while the window is open.
pub fn keep_above_overlay(host: HWND) {
    unsafe {
        if IsIconic(host).as_bool() || !IsWindowVisible(host).as_bool() {
            return;
        }
        let _ = SetWindowPos(
            host,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW | SWP_NOACTIVATE,
        );
    }
}

unsafe fn force_to_front(host: HWND) {
    if RAISING.swap(true, Ordering::SeqCst) {
        return;
    }
    if IsIconic(host).as_bool() {
        let _ = ShowWindow(host, SW_RESTORE);
    } else {
        let _ = ShowWindow(host, SW_SHOW);
    }
    let fg = GetForegroundWindow();
    let fg_tid = GetWindowThreadProcessId(fg, None);
    let this_tid = GetCurrentThreadId();
    let attached = fg_tid != 0
        && fg_tid != this_tid
        && AttachThreadInput(this_tid, fg_tid, BOOL(1)).as_bool();
    let _ = BringWindowToTop(host);
    let _ = SetForegroundWindow(host);
    let _ = SetWindowPos(
        host,
        HWND_TOPMOST,
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
    );
    let _ = SetWindowPos(
        host,
        HWND_NOTOPMOST,
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
    );
    if attached {
        let _ = AttachThreadInput(this_tid, fg_tid, BOOL(0));
    }
    let _ = SetForegroundWindow(host);
    RAISING.store(false, Ordering::SeqCst);
}

pub fn paint(fonts: &Fonts) {
    let host = {
        let ui = UI.lock().unwrap();
        let Some(ui) = ui.as_ref() else {
            return;
        };
        ui.host
    };
    unsafe {
        if IsIconic(host).as_bool() || !IsWindowVisible(host).as_bool() {
            return;
        }
        let mut rc = windows::Win32::Foundation::RECT::default();
        if GetClientRect(host, &mut rc).is_err() {
            return;
        }
        let w = (rc.right - rc.left).max(1);
        let h = (rc.bottom - rc.top).max(1);
        let Some(mut px) = Pixmap::new(w as u32, h as u32) else {
            return;
        };
        draw(&mut px, fonts, w as f32, h as f32);
        present(host, &px);
    }
}

pub fn handle_message(msg: u32, wp: WPARAM, lp: LPARAM) -> bool {
    match msg {
        WM_ERASEBKGND => true,
        WM_PAINT => {
            let host = UI.lock().unwrap().as_ref().map(|u| u.host);
            let Some(host) = host else {
                return false;
            };
            unsafe {
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(host, &mut ps);
                let mut rc = windows::Win32::Foundation::RECT::default();
                let _ = GetClientRect(host, &mut rc);
                let brush = CreateSolidBrush(windows::Win32::Foundation::COLORREF(0x00121210));
                let _ = FillRect(hdc, &rc, brush);
                let _ = DeleteObject(HGDIOBJ(brush.0));
                let _ = EndPaint(host, &ps);
            }
            true
        }
        WM_MOUSEMOVE => {
            let p = lp_xy(lp);
            set_hover(p);
            update_drag(p);
            true
        }
        WM_LBUTTONDOWN => {
            press(lp_xy(lp));
            true
        }
        WM_LBUTTONUP => {
            release(lp_xy(lp));
            true
        }
        WM_MOUSEWHEEL => {
            let delta = ((wp.0 as u32 >> 16) as i16) as f32;
            if let Some(ui) = UI.lock().unwrap().as_mut() {
                let mut pt = POINT {
                    x: (lp.0 as i32) as i16 as i32,
                    y: ((lp.0 as i32) >> 16) as i16 as i32,
                };
                let in_client = unsafe { ScreenToClient(ui.host, &mut pt) }.as_bool();
                let px = pt.x as f32;
                let py = pt.y as f32;
                let over_drop = in_client
                    && ui.open_drop.is_some()
                    && ui.drop_menu.is_some_and(|(mx, my, mw, mh, _)| {
                        px >= mx && px <= mx + mw && py >= my && py <= my + mh
                    });
                if over_drop {
                    if let Some((_, _, _, view_h, content_h)) = ui.drop_menu {
                        let max = (content_h - view_h).max(0.0);
                        ui.drop_scroll = (ui.drop_scroll - delta * 0.35).clamp(0.0, max);
                    }
                } else if ui.whats_new_open {
                    ui.whats_new_scroll =
                        (ui.whats_new_scroll - delta * 0.4).clamp(0.0, ui.whats_new_scroll_max);
                } else if ui.reply_id.is_some() {
                    ui.reply_scroll =
                        (ui.reply_scroll - delta * 0.4).clamp(0.0, ui.reply_scroll_max);
                } else {
                    let over_nav = in_client
                        && ui.nav_bottom > ui.nav_top
                        && px < SIDE_W
                        && py >= ui.nav_top
                        && py < ui.nav_bottom;
                    if over_nav {
                        let max = (ui.nav_content_h - (ui.nav_bottom - ui.nav_top)).max(0.0);
                        ui.nav_scroll = (ui.nav_scroll - delta * 0.4).clamp(0.0, max);
                    } else {
                        ui.scroll = (ui.scroll - delta * 0.4).clamp(0.0, ui.scroll_max);
                    }
                }
            }
            true
        }
        WM_CHAR => crate::feedback::on_char(char::from_u32(wp.0 as u32).unwrap_or('\0')),
        WM_KEYDOWN => {
            let vk = wp.0 as u16;
            let shift = unsafe { GetAsyncKeyState(VK_SHIFT.0 as i32) } < 0;
            let ctrl = unsafe { GetAsyncKeyState(VK_CONTROL.0 as i32) } < 0;
            if handle_key(vk, shift, ctrl) {
                true
            } else {
                crate::feedback::on_key(vk, ctrl)
            }
        }
        WM_SETCURSOR => {
            let (over, dragging, text) = {
                let ui = UI.lock().unwrap();
                let ui = ui.as_ref();
                let hover = ui.and_then(|u| u.hover);
                let dragging = ui.and_then(|u| u.drag).is_some();
                let sliding = ui.and_then(|u| u.slide).is_some() || hover.is_some_and(is_slider);
                let grip = matches!(hover, Some(Hit::StDrag(_)) | Some(Hit::RelDrag(_)));
                (
                    hover.is_some(),
                    dragging || grip || sliding,
                    hover == Some(Hit::FbText) || hover == Some(Hit::ReplyText),
                )
            };
            unsafe {
                let idc = if dragging {
                    IDC_SIZEALL
                } else if text {
                    IDC_IBEAM
                } else if over {
                    IDC_HAND
                } else {
                    IDC_ARROW
                };
                let cur = LoadCursorW(None, idc).unwrap_or_default();
                let _ = SetCursor(cur);
            }
            true
        }
        _ => false,
    }
}

fn lp_xy(lp: LPARAM) -> (f32, f32) {
    let x = (lp.0 as i32) as i16 as f32;
    let y = ((lp.0 as i32) >> 16) as i16 as f32;
    (x, y)
}

fn set_hover(p: (f32, f32)) {
    let mut ui = UI.lock().unwrap();
    let Some(ui) = ui.as_mut() else {
        return;
    };
    ui.hover = hit_at(&ui.hits, p.0, p.1);
}

fn press(p: (f32, f32)) {
    let (id, host) = {
        let ui = UI.lock().unwrap();
        let Some(ui) = ui.as_ref() else {
            return;
        };
        (hit_at(&ui.hits, p.0, p.1), ui.host)
    };
    match id {
        Some(Hit::StDrag(i)) => start_drag(DragKind::St, i, host),
        Some(Hit::RelDrag(i)) => start_drag(DragKind::Rel, i, host),
        Some(hit) if is_slider(hit) => {
            let on_track = {
                let ui = UI.lock().unwrap();
                ui.as_ref().is_some_and(|u| {
                    u.hits.iter().rev().any(|h| {
                        h.id == hit && h.h <= 24.0 && p.0 >= h.x && p.0 <= h.x + h.w && p.1 >= h.y && p.1 <= h.y + h.h
                    })
                })
            };
            set_kb_focus(hit);
            if on_track {
                start_slide(hit, p.0, host);
            }
        }
        _ => click(p),
    }
}

fn start_drag(kind: DragKind, i: u8, host: HWND) {
    close_drop();
    if let Some(ui) = UI.lock().unwrap().as_mut() {
        ui.drag = Some(ColDrag { kind, from: i, over: i });
    }
    unsafe {
        let _ = SetCapture(host);
    }
}

fn is_slider(hit: Hit) -> bool {
    matches!(
        hit,
        Hit::StBg
            | Hit::StHl
            | Hit::RelBg
            | Hit::RelHl
            | Hit::MapBg
            | Hit::MiniBg
            | Hit::MiniZoom
            | Hit::RadarBg
            | Hit::DashBg
            | Hit::TickerBg
            | Hit::SysBg
            | Hit::SectorBg
            | Hit::DeltaBg
            | Hit::StanceBg
            | Hit::FlagBg
            | Hit::StW(_)
            | Hit::RelW(_)
            | Hit::Font(_)
    )
}

fn slide_range(hit: Hit) -> (i32, i32) {
    match hit {
        Hit::StW(i) => {
            let max = with_config(|c| {
                c.st_order
                    .get(i as usize)
                    .map(|f| f.width_max())
                    .unwrap_or(COL_W_MAX)
            });
            (COL_W_MIN, max)
        }
        Hit::RelW(i) => {
            let max = with_config(|c| {
                c.rel_order
                    .get(i as usize)
                    .map(|f| f.width_max())
                    .unwrap_or(COL_W_MAX)
            });
            (COL_W_MIN, max)
        }
        Hit::Font(_) => (70, 160),
        _ => (0, 100),
    }
}

fn start_slide(hit: Hit, mx: f32, host: HWND) {
    close_drop();
    let box_ = {
        let ui = UI.lock().unwrap();
        ui.as_ref().and_then(|u| {
            u.hits
                .iter()
                .rev()
                .find(|h| h.id == hit && h.h <= 24.0)
                .copied()
                .or_else(|| u.hits.iter().rev().find(|h| h.id == hit).copied())
        })
    };
    let Some(hb) = box_ else {
        return;
    };
    let (min, max) = slide_range(hit);
    if let Some(ui) = UI.lock().unwrap().as_mut() {
        ui.slide = Some(SlideDrag {
            hit,
            x: hb.x,
            w: hb.w,
            min,
            max,
        });
    }
    apply_slide(hit, mx, hb.x, hb.w, min, max);
    unsafe {
        let _ = SetCapture(host);
    }
}

fn apply_slide(hit: Hit, mx: f32, x: f32, w: f32, min: i32, max: i32) {
    let t = if w <= 1.0 { 0.0 } else { ((mx - x) / w).clamp(0.0, 1.0) };
    let v = min + ((max - min) as f32 * t).round() as i32;
    update_config(|c| match hit {
        Hit::StBg => c[WidgetId::Standings].bg = v,
        Hit::StHl => c.st_hl = v,
        Hit::RelBg => c[WidgetId::Relative].bg = v,
        Hit::RelHl => c.rel_hl = v,
        Hit::MapBg => c[WidgetId::Map].bg = v,
        Hit::MiniBg => c[WidgetId::Minimap].bg = v,
        Hit::MiniZoom => c.mini_zoom = v,
        Hit::RadarBg => c[WidgetId::Radar].bg = v,
        Hit::DashBg => c[WidgetId::Dash].bg = v,
        Hit::TickerBg => c[WidgetId::Ticker].bg = v,
        Hit::SysBg => c[WidgetId::Sys].bg = v,
        Hit::SectorBg => c[WidgetId::Sector].bg = v,
        Hit::DeltaBg => c[WidgetId::Delta].bg = v,
        Hit::StanceBg => c[WidgetId::Stance].bg = v,
        Hit::FlagBg => c[WidgetId::Flag].bg = v,
        Hit::StW(i) => {
            if let Some(f) = c.st_order.get(i as usize).copied() {
                f.set_width(c, v);
            }
        }
        Hit::RelW(i) => {
            if let Some(f) = c.rel_order.get(i as usize).copied() {
                f.set_width(c, v);
            }
        }
        Hit::Font(id) => c.set_font_pct(id, v),
        _ => {}
    });
}

fn update_drag(p: (f32, f32)) {
    let slide = {
        let ui = UI.lock().unwrap();
        ui.as_ref().and_then(|u| u.slide)
    };
    if let Some(s) = slide {
        apply_slide(s.hit, p.0, s.x, s.w, s.min, s.max);
        return;
    }
    let mut ui = UI.lock().unwrap();
    let Some(ui) = ui.as_mut() else {
        return;
    };
    let Some(drag) = ui.drag else {
        return;
    };
    if let Some(over) = drag_over(&ui.hits, drag.kind, p.1) {
        if let Some(d) = ui.drag.as_mut() {
            d.over = over;
        }
    }
}

fn drag_over(hits: &[HitBox], kind: DragKind, y: f32) -> Option<u8> {
    let mut rows: Vec<(u8, f32, f32)> = hits
        .iter()
        .filter_map(|h| match (kind, h.id) {
            (DragKind::St, Hit::StDrag(i)) | (DragKind::Rel, Hit::RelDrag(i)) => {
                Some((i, h.y, h.y + h.h))
            }
            _ => None,
        })
        .collect();
    rows.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let first = rows.first()?;
    if y < first.1 {
        return Some(first.0);
    }
    let last = rows.last()?;
    if y >= last.2 {
        return Some(last.0);
    }
    rows.iter()
        .find(|(_, a, b)| y >= *a && y < *b)
        .map(|(i, _, _)| *i)
}

fn release(p: (f32, f32)) {
    update_drag(p);
    let sliding = {
        let mut ui = UI.lock().unwrap();
        ui.as_mut().and_then(|u| u.slide.take()).is_some()
    };
    if sliding {
        unsafe {
            let _ = ReleaseCapture();
        }
        return;
    }
    let drag = {
        let mut ui = UI.lock().unwrap();
        ui.as_mut().and_then(|u| u.drag.take())
    };
    unsafe {
        let _ = ReleaseCapture();
    }
    let Some(drag) = drag else {
        return;
    };
    if drag.from == drag.over {
        return;
    }
    update_config(|c| match drag.kind {
        DragKind::St => c.move_st_to(drag.from as usize, drag.over as usize),
        DragKind::Rel => c.move_rel_to(drag.from as usize, drag.over as usize),
    });
}

fn click(p: (f32, f32)) {
    let id = {
        let ui = UI.lock().unwrap();
        ui.as_ref().and_then(|u| hit_at(&u.hits, p.0, p.1))
    };
    let Some(id) = id else {
        close_drop();
        crate::feedback::set_focus(false);
        return;
    };
    if !matches!(id, Hit::FbText | Hit::ReplyText) {
        crate::feedback::set_focus(false);
        crate::feedback::set_compose_focus(false);
    }
    set_kb_focus(id);
    dispatch(id, p);
}

fn dispatch(id: Hit, p: (f32, f32)) {
    match id {
        Hit::TabWidgets => {
            let last = UI.lock().unwrap().as_ref().map(|u| u.last_widget).unwrap_or(Tab::Standings);
            let last = if last.is_labs() && !with_config(|c| c.experimental_unlocked()) {
                Tab::Standings
            } else {
                last
            };
            set_tab(last);
            return;
        }
        Hit::TabApp => {
            set_tab(Tab::App);
            return;
        }
        Hit::TabFeedback => {
            set_tab(Tab::Feedback);
            return;
        }
        Hit::TabSt => {
            set_tab(Tab::Standings);
            return;
        }
        Hit::TabRel => {
            set_tab(Tab::Relative);
            return;
        }
        Hit::TabMap => {
            set_tab(Tab::Map);
            return;
        }
        Hit::TabMini => {
            set_tab(Tab::Minimap);
            return;
        }
        Hit::TabRadar => {
            set_tab(Tab::Radar);
            return;
        }
        Hit::TabDash => {
            set_tab(Tab::Dash);
            return;
        }
        Hit::TabTicker => {
            set_tab(Tab::Ticker);
            return;
        }
        Hit::TabSys => {
            set_tab(Tab::Sys);
            return;
        }
        Hit::TabSector => {
            set_tab(Tab::Sector);
            return;
        }
        Hit::TabDelta => {
            set_tab(Tab::Delta);
            return;
        }
        Hit::TabStance => {
            set_tab(Tab::Stance);
            return;
        }
        Hit::TabFlag => {
            set_tab(Tab::Flag);
            return;
        }
        Hit::MapDotOpen => {
            toggle_drop(Drop::MapDot);
            return;
        }
        Hit::MiniDotOpen => {
            toggle_drop(Drop::MiniDot);
            return;
        }
        Hit::FontOpen => {
            toggle_drop(Drop::FontFamily);
            return;
        }
        Hit::UnitsOpen => {
            toggle_drop(Drop::Units);
            return;
        }
        Hit::StTextOpen => {
            toggle_drop(Drop::StText);
            return;
        }
        Hit::RelTextOpen => {
            toggle_drop(Drop::RelText);
            return;
        }
        Hit::SettingsKeyOpen => {
            toggle_drop(Drop::SettingsKey);
            return;
        }
        Hit::StanceBindOpen => {
            let was = UI.lock().unwrap().as_ref().is_some_and(|u| u.bind_listen);
            close_drop();
            if !was {
                let host = {
                    let mut ui = UI.lock().unwrap();
                    ui.as_mut().map(|u| {
                        u.bind_listen = true;
                        u.host
                    })
                };
                if let Some(host) = host {
                    announce(
                        host,
                        "Press a pad button, key, or mouse button now. Escape cancels.",
                    );
                }
            }
            return;
        }
        Hit::StanceModeOpen => {
            toggle_drop(Drop::StanceMode);
            return;
        }
        Hit::StanceStyleOpen => {
            toggle_drop(Drop::StanceStyle);
            return;
        }
        Hit::StanceReset => {
            close_drop();
            crate::stance::reset_standing();
            return;
        }
        Hit::TrackPbClear => {
            close_drop();
            mxbo_hud::track_pb::clear_current();
            mxbo_hud::delta::reload_saved();
            mxbo_hud::sector::reload();
            return;
        }
        Hit::DashFootOpen(slot) => {
            toggle_drop(Drop::DashFoot(slot));
            return;
        }
        Hit::TickerFootOpen(slot) => {
            toggle_drop(Drop::TickerFoot(slot));
            return;
        }
        Hit::InfoOpen(bar, slot) => {
            toggle_drop(Drop::Info(bar, slot));
            return;
        }
        Hit::UpdateCheck => {
            close_drop();
            crate::update::check();
            return;
        }
        Hit::UpdateInstall => {
            close_drop();
            crate::update::install();
            return;
        }
        Hit::UpdateBanner => {
            close_drop();
            return;
        }
        Hit::UpdateBannerDismiss => {
            close_drop();
            if let Some(ui) = UI.lock().unwrap().as_mut() {
                ui.banner_dismissed = true;
            }
            return;
        }
        Hit::WhatsNewOpen => {
            close_drop();
            open_whats_new();
            return;
        }
        Hit::WhatsNewDismiss => {
            close_drop();
            dismiss_whats_new();
            return;
        }
        Hit::WhatsNewScrim | Hit::WhatsNewPanel => {
            close_drop();
            return;
        }
        Hit::ReplyDismiss => {
            close_drop();
            dismiss_reply();
            return;
        }
        Hit::ReplySend => {
            close_drop();
            crate::feedback::send_compose();
            return;
        }
        Hit::ReplyText => {
            close_drop();
            crate::feedback::click_compose(p.0, p.1);
            return;
        }
        Hit::ReplyScrim | Hit::ReplyPanel => {
            close_drop();
            return;
        }
        Hit::StartWithWindows => {
            close_drop();
            let on = crate::config::with_config(|c| !c.start_with_windows);
            crate::config::update_config(|c| c.start_with_windows = on);
            crate::startup::sync_from_config();
            return;
        }
        Hit::MinimizeOnClose => {
            close_drop();
            crate::config::update_config(|c| c.minimize_on_close = !c.minimize_on_close);
            return;
        }
        Hit::CloseWithGame => {
            close_drop();
            crate::config::update_config(|c| c.close_with_game = !c.close_with_game);
            return;
        }
        Hit::OpenWithGame => {
            close_drop();
            let on = crate::config::with_config(|c| !c.open_with_game);
            crate::config::update_config(|c| c.open_with_game = on);
            crate::startup::sync_from_config();
            if on {
                crate::startup::spawn_game_waiter();
            } else {
                // Turning off: stop leftover HUD waiters from an older build.
                crate::startup::kill_other_hud_processes();
            }
            return;
        }
        Hit::AutoUpdateOnLaunch => {
            close_drop();
            let on = crate::config::with_config(|c| !c.auto_update_on_launch);
            crate::config::update_config(|c| c.auto_update_on_launch = on);
            if !on {
                crate::update::check();
            }
            return;
        }
        Hit::QuitApp => {
            close_drop();
            crate::config::update_config(|_| {});
            if crate::config::with_config(|c| c.open_with_game) {
                crate::startup::spawn_game_waiter();
            }
            crate::quit_app();
            return;
        }
        Hit::Uninstall => {
            close_drop();
            let host = UI.lock().unwrap().as_ref().map(|u| u.host);
            let Some(host) = host else {
                return;
            };
            if crate::uninstall::confirm(host) && crate::uninstall::start(host) {
                crate::config::update_config(|_| {});
                crate::quit_app();
            }
            return;
        }
        Hit::GameFolder => {
            close_drop();
            let host = UI.lock().unwrap().as_ref().map(|u| u.host);
            let Some(host) = host else {
                return;
            };
            crate::plugin::pick_game_folder(host);
            return;
        }
        Hit::FbRate => {
            close_drop();
            crate::feedback::set_kind(crate::feedback::Kind::Rate);
            return;
        }
        Hit::FbBug => {
            close_drop();
            crate::feedback::set_kind(crate::feedback::Kind::Bug);
            return;
        }
        Hit::FbFeature => {
            close_drop();
            crate::feedback::set_kind(crate::feedback::Kind::Feature);
            return;
        }
        Hit::FbStar(n) => {
            close_drop();
            crate::feedback::set_rating(n);
            return;
        }
        Hit::FbText => {
            close_drop();
            crate::feedback::click_text(p.0, p.1);
            return;
        }
        Hit::FbAttach => {
            close_drop();
            crate::feedback::toggle_attach();
            return;
        }
        Hit::FbSend => {
            close_drop();
            crate::feedback::send();
            return;
        }
        _ => close_drop(),
    }
    update_config(|c| match id {
        Hit::StShow => c[WidgetId::Standings].show ^= true,
        Hit::RelShow => c[WidgetId::Relative].show ^= true,
        Hit::MapShow => c[WidgetId::Map].show ^= true,
        Hit::MiniShow => c[WidgetId::Minimap].show ^= true,
        Hit::RadarShow => c[WidgetId::Radar].show ^= true,
        Hit::DashShow => c[WidgetId::Dash].show ^= true,
        Hit::DashRev => c.dash_rev = !c.dash_rev,
        Hit::DashSimple => c.dash_simple = !c.dash_simple,
        Hit::TickerShow => c[WidgetId::Ticker].show ^= true,
        Hit::SysShow => c[WidgetId::Sys].show ^= true,
        Hit::SectorShow => c[WidgetId::Sector].show ^= true,
        Hit::SectorLive => c.sector_live = !c.sector_live,
        Hit::DeltaShow => c[WidgetId::Delta].show ^= true,
        Hit::StanceShow => c[WidgetId::Stance].show ^= true,
        Hit::FlagShow => c[WidgetId::Flag].show ^= true,
        Hit::FlagYellow => c.flag_yellow = !c.flag_yellow,
        Hit::FlagBlue => c.flag_blue = !c.flag_blue,
        Hit::StanceShowSit => c.stance_show_sit = !c.stance_show_sit,
        Hit::FeatureSector => {
            c.experimental = !c.experimental;
            if !c.experimental {
                c[WidgetId::Sector].show = false;
                c[WidgetId::Delta].show = false;
            }
        },
        Hit::ShowPresence => c.show_presence = !c.show_presence,
        Hit::HighlightFriends => c.highlight_friends = !c.highlight_friends,
        Hit::TickerTitle => c.ticker_title = !c.ticker_title,
        Hit::TickerAutoscroll => c.ticker_autoscroll = !c.ticker_autoscroll,
        Hit::StStripe => c.st_stripe = !c.st_stripe,
        Hit::RelStripe => c.rel_stripe = !c.rel_stripe,
        Hit::StPos => c.st_pos = !c.st_pos,
        Hit::StNum => c.st_num = !c.st_num,
        Hit::StName => c.st_name = !c.st_name,
        Hit::StGap => c.st_gap = !c.st_gap,
        Hit::StLaps => c.st_laps = !c.st_laps,
        Hit::StCurrent => c.st_current = !c.st_current,
        Hit::StBest => c.st_best = !c.st_best,
        Hit::StLast => c.st_last = !c.st_last,
        Hit::StStatus => c.st_status = !c.st_status,
        Hit::StBike => c.st_bike = !c.st_bike,
        Hit::StPenalty => c.st_penalty = !c.st_penalty,
        Hit::StCrashed => c.st_crashed = !c.st_crashed,
        Hit::StInterval => c.st_interval = !c.st_interval,
        Hit::RelNum => c.rel_num = !c.rel_num,
        Hit::RelName => c.rel_name = !c.rel_name,
        Hit::RelGap => c.rel_gap = !c.rel_gap,
        Hit::RelLaps => c.rel_laps = !c.rel_laps,
        Hit::RelCurrent => c.rel_current = !c.rel_current,
        Hit::RelPos => c.rel_pos = !c.rel_pos,
        Hit::RelBike => c.rel_bike = !c.rel_bike,
        Hit::RelPenalty => c.rel_penalty = !c.rel_penalty,
        Hit::RelInterval => c.rel_interval = !c.rel_interval,
        Hit::RelCrashed => c.rel_crashed = !c.rel_crashed,
        Hit::RelBest => c.rel_best = !c.rel_best,
        Hit::RelLast => c.rel_last = !c.rel_last,
        Hit::MapOthers => c.map_others = !c.map_others,
        Hit::MapSf => c.map_sf = !c.map_sf,
        Hit::MapSectors => c.map_sectors = !c.map_sectors,
        Hit::MapArrows => c.map_arrows = !c.map_arrows,
        Hit::MapCrown => c.map_crown = !c.map_crown,
        Hit::MapPlace => c.map_place = !c.map_place,
        Hit::MapNumbers => c.map_numbers = !c.map_numbers,
        Hit::MapDotNum => c.map_dot = DotLabel::Number,
        Hit::MapDotPos => c.map_dot = DotLabel::Position,
        Hit::MiniOthers => c.mini_others = !c.mini_others,
        Hit::MiniSf => c.mini_sf = !c.mini_sf,
        Hit::MiniSectors => c.mini_sectors = !c.mini_sectors,
        Hit::MiniArrows => c.mini_arrows = !c.mini_arrows,
        Hit::MiniCrown => c.mini_crown = !c.mini_crown,
        Hit::MiniPlace => c.mini_place = !c.mini_place,
        Hit::MiniNumbers => c.mini_numbers = !c.mini_numbers,
        Hit::MiniDotNum => c.mini_dot = DotLabel::Number,
        Hit::MiniDotPos => c.mini_dot = DotLabel::Position,
        Hit::RadarSides => c.radar_sides = !c.radar_sides,
        Hit::RadarRear => c.radar_rear = !c.radar_rear,
        Hit::RadarRings => c.radar_rings = !c.radar_rings,
        Hit::Bold(id) => {
            let on = !c.bold(id);
            c.set_bold(id, on);
        }
        Hit::Snap(id, align) => c.snap(id, align),
        Hit::FontSegoe => c.font_family = FontFamily::Segoe,
        Hit::FontArial => c.font_family = FontFamily::Arial,
        Hit::FontTahoma => c.font_family = FontFamily::Tahoma,
        Hit::FontRoboto => c.font_family = FontFamily::Roboto,
        Hit::FontExo2 => c.font_family = FontFamily::Exo2,
        Hit::FontTeko => c.font_family = FontFamily::Teko,
        Hit::FontGoldman => c.font_family = FontFamily::Goldman,
        Hit::FontMontserrat => c.font_family = FontFamily::Montserrat,
        Hit::UnitsMetric => c.units = Units::Metric,
        Hit::UnitsImperial => c.units = Units::Imperial,
        Hit::StTextWhite => c.st_text = TableText::White,
        Hit::StTextBlack => c.st_text = TableText::Black,
        Hit::RelTextWhite => c.rel_text = TableText::White,
        Hit::RelTextBlack => c.rel_text = TableText::Black,
        Hit::SettingsKeyPick(key) => c.settings_key = key,
        Hit::StanceModePick(mode) => c.stance_mode = mode,
        Hit::StanceStylePick(style) => c.stance_style = style,
        Hit::DashFootPick(slot, field) => match slot {
            0 => c.dash_left = field,
            1 => c.dash_mid = field,
            2 => c.dash_right = field,
            _ => {}
        },
        Hit::TickerFootPick(slot, field) => match slot {
            0 => c.ticker_left = field,
            1 => c.ticker_right = field,
            _ => {}
        },
        Hit::InfoPick(bar, slot, field) => set_info_slot(c, bar, slot, field),
        Hit::StDec => c.standings_rows = (c.standings_rows - 1).max(3),
        Hit::StInc => c.standings_rows = (c.standings_rows + 1).min(40),
        Hit::RelDec => c.relative_count = (c.relative_count - 1).max(1),
        Hit::RelInc => c.relative_count = (c.relative_count + 1).min(8),
        Hit::TickerDec => c.ticker_count = (c.ticker_count - 1).max(3),
        Hit::TickerInc => c.ticker_count = (c.ticker_count + 1).min(15),
        Hit::TabWidgets | Hit::TabApp | Hit::TabFeedback | Hit::TabSt | Hit::TabRel | Hit::TabMap | Hit::TabMini | Hit::TabRadar | Hit::TabDash
        | Hit::TabTicker | Hit::TabSys | Hit::TabSector | Hit::TabDelta | Hit::TabStance | Hit::TabFlag
        | Hit::MapDotOpen | Hit::MiniDotOpen | Hit::FontOpen | Hit::UnitsOpen | Hit::StTextOpen | Hit::RelTextOpen
        | Hit::SettingsKeyOpen
        | Hit::StanceBindOpen
        | Hit::StanceModeOpen
        | Hit::StanceStyleOpen
        | Hit::DashFootOpen(_)
        | Hit::TickerFootOpen(_)
        | Hit::InfoOpen(_, _)
        | Hit::UpdateCheck | Hit::UpdateInstall | Hit::UpdateBanner | Hit::UpdateBannerDismiss
        | Hit::WhatsNewOpen | Hit::WhatsNewDismiss | Hit::WhatsNewScrim | Hit::WhatsNewPanel
        | Hit::ReplyDismiss | Hit::ReplySend | Hit::ReplyText | Hit::ReplyScrim | Hit::ReplyPanel
        | Hit::StartWithWindows | Hit::MinimizeOnClose
        | Hit::CloseWithGame | Hit::OpenWithGame
        | Hit::AutoUpdateOnLaunch | Hit::QuitApp | Hit::Uninstall | Hit::GameFolder
        | Hit::FbRate | Hit::FbBug | Hit::FbFeature | Hit::FbStar(_) | Hit::FbText | Hit::FbAttach | Hit::FbSend
        | Hit::StDrag(_) | Hit::RelDrag(_)
        | Hit::StBg | Hit::StHl | Hit::RelBg | Hit::RelHl | Hit::MapBg | Hit::MiniBg | Hit::MiniZoom | Hit::RadarBg | Hit::DashBg | Hit::TickerBg | Hit::SysBg | Hit::SectorBg | Hit::DeltaBg | Hit::StanceBg | Hit::FlagBg
        | Hit::StW(_) | Hit::RelW(_) | Hit::Font(_) | Hit::StanceReset | Hit::TrackPbClear => {}
    });
    if id == Hit::FeatureSector && !with_config(|c| c.experimental_unlocked()) {
        let on_labs = UI.lock().unwrap().as_ref().is_some_and(|u| u.tab.is_labs());
        if on_labs {
            set_tab(Tab::App);
        }
    }
}

fn is_drop_pick(hit: Hit) -> bool {
    matches!(
        hit,
        Hit::MapDotNum
            | Hit::MapDotPos
            | Hit::MiniDotNum
            | Hit::MiniDotPos
            | Hit::StTextWhite
            | Hit::StTextBlack
            | Hit::RelTextWhite
            | Hit::RelTextBlack
            | Hit::FontSegoe
            | Hit::FontArial
            | Hit::FontTahoma
            | Hit::FontRoboto
            | Hit::FontExo2
            | Hit::FontTeko
            | Hit::FontGoldman
            | Hit::FontMontserrat
            | Hit::UnitsMetric
            | Hit::UnitsImperial
            | Hit::SettingsKeyPick(_)
            | Hit::StanceModePick(_)
            | Hit::StanceStylePick(_)
            | Hit::DashFootPick(_, _)
            | Hit::TickerFootPick(_, _)
            | Hit::InfoPick(_, _, _)
    )
}

fn is_focusable(hit: Hit) -> bool {
    !matches!(
        hit,
        Hit::StDrag(_) | Hit::RelDrag(_) | Hit::UpdateBanner | Hit::WhatsNewScrim | Hit::WhatsNewPanel
            | Hit::ReplyScrim | Hit::ReplyPanel
    )
}

fn set_kb_focus(id: Hit) {
    let host = {
        let mut ui = UI.lock().unwrap();
        let Some(ui) = ui.as_mut() else {
            return;
        };
        if ui.focus == Some(id) {
            return;
        }
        ui.focus = Some(id);
        ui.host
    };
    announce(host, &hit_label(id));
}

fn hit_label(hit: Hit) -> String {
    match hit {
        Hit::TabWidgets => "Widgets".into(),
        Hit::TabApp => "Settings".into(),
        Hit::TabFeedback => "Feedback".into(),
        Hit::TabSt => "Standings".into(),
        Hit::TabRel => "Relative".into(),
        Hit::TabMap => "Map".into(),
        Hit::TabMini => "Minimap".into(),
        Hit::TabRadar => "Radar".into(),
        Hit::TabDash => "Dash".into(),
        Hit::TabTicker => "Horizontal Standings".into(),
        Hit::TabSys => "Systems".into(),
        Hit::TabSector => "Sectors".into(),
        Hit::TabDelta => "Delta Bar".into(),
        Hit::TabStance => "Stance".into(),
        Hit::TabFlag => "Flags".into(),
        Hit::StShow | Hit::RelShow | Hit::MapShow | Hit::MiniShow | Hit::RadarShow | Hit::DashShow
        | Hit::TickerShow | Hit::SysShow | Hit::SectorShow | Hit::DeltaShow | Hit::StanceShow | Hit::FlagShow => "Show on overlay".into(),
        Hit::QuitApp => "Quit overlay".into(),
        Hit::Font(_) => "Font size".into(),
        Hit::Bold(_) => "Bold text".into(),
        Hit::StBg | Hit::RelBg | Hit::MapBg | Hit::MiniBg => "Background".into(),
        Hit::RadarBg | Hit::DashBg | Hit::TickerBg | Hit::SysBg | Hit::SectorBg | Hit::DeltaBg | Hit::StanceBg | Hit::FlagBg => "Panel opacity".into(),
        Hit::StHl | Hit::RelHl => "Row highlight".into(),
        Hit::StStripe | Hit::RelStripe => "Alternating rows".into(),
        Hit::StDec | Hit::StInc => "Rows".into(),
        Hit::RelDec | Hit::RelInc => "Nearby riders".into(),
        Hit::TickerDec | Hit::TickerInc => "Riders shown".into(),
        Hit::Snap(_, align) => snap_align_label(align).into(),
        Hit::StW(_) | Hit::RelW(_) => "Column width".into(),
        Hit::FontOpen => "Font".into(),
        Hit::UnitsOpen => "Units".into(),
        Hit::SettingsKeyOpen => "Settings key".into(),
        Hit::FeatureSector => "Experimental widgets".into(),
        Hit::ShowPresence => "Show overlay users".into(),
        Hit::HighlightFriends => "Highlight Steam friends".into(),
        Hit::StanceBindOpen => {
            if UI.lock().unwrap().as_ref().is_some_and(|u| u.bind_listen) {
                "Press a pad button, key, or mouse button now. Escape cancels.".into()
            } else {
                "Sit button".into()
            }
        }
        Hit::StanceModeOpen => "Sit mode".into(),
        Hit::StanceStyleOpen => "Look".into(),
        Hit::StanceShowSit => "Show sitting".into(),
        Hit::FlagYellow => "Yellow flag".into(),
        Hit::FlagBlue => "Blue flag".into(),
        Hit::RadarSides => "Side proximity".into(),
        Hit::RadarRear => "Rear proximity".into(),
        Hit::RadarRings => "Range rings".into(),
        Hit::DashRev => "Rev indicator".into(),
        Hit::DashSimple => "Simple dash".into(),
        Hit::MapSectors => "Sector lines".into(),
        Hit::MiniSectors => "Sector lines".into(),
        Hit::SectorLive => "Live sector".into(),
        Hit::StanceReset => "Reset to standing".into(),
        Hit::TrackPbClear => "Clear this track".into(),
        Hit::FbText => "Feedback message".into(),
        Hit::FbSend => "Send feedback".into(),
        Hit::Uninstall => "Uninstall".into(),
        Hit::GameFolder => "MX Bikes folder".into(),
        Hit::UpdateCheck => "Check for updates".into(),
        Hit::UpdateInstall => "Install update".into(),
        Hit::WhatsNewOpen => {
            if crate::changelog::previewing() {
                "Preview next".into()
            } else {
                "What's new".into()
            }
        }
        Hit::WhatsNewDismiss | Hit::ReplyDismiss => "Got it".into(),
        Hit::ReplySend => "Send".into(),
        Hit::ReplyText => "Write a reply".into(),
        _ => "Control".into(),
    }
}

fn announce(host: HWND, label: &str) {
    let title = format!("Holeshot HUD — Settings — {label}");
    let mut buf: Vec<u16> = title.encode_utf16().collect();
    buf.push(0);
    unsafe {
        let _ = SetWindowTextW(host, PCWSTR(buf.as_ptr()));
        NotifyWinEvent(EVENT_OBJECT_NAMECHANGE, host, OBJID_CLIENT.0, 0);
        NotifyWinEvent(EVENT_OBJECT_FOCUS, host, OBJID_CLIENT.0, 0);
    }
}

fn handle_key(vk: u16, shift: bool, _ctrl: bool) -> bool {
    let fb = crate::feedback::is_focused();
    if fb && vk != VK_TAB.0 && vk != VK_ESCAPE.0 {
        return false;
    }
    if vk == VK_ESCAPE.0 {
        let whats_new = UI.lock().unwrap().as_ref().is_some_and(|u| u.whats_new_open);
        if whats_new {
            dismiss_whats_new();
            return true;
        }
        let reply_open = UI.lock().unwrap().as_ref().is_some_and(|u| u.reply_id.is_some());
        if reply_open {
            if crate::feedback::compose_snapshot().focused {
                crate::feedback::set_compose_focus(false);
                return true;
            }
            dismiss_reply();
            return true;
        }
        let open = UI.lock().unwrap().as_ref().is_some_and(|u| u.open_drop.is_some() || u.bind_listen);
        if open {
            close_drop();
            return true;
        }
        if fb {
            crate::feedback::set_focus(false);
            return true;
        }
        return true;
    }
    if UI.lock().unwrap().as_ref().is_some_and(|u| u.bind_listen) {
        return true;
    }
    if vk == VK_TAB.0 {
        close_drop();
        cycle_focus(!shift);
        return true;
    }
    if vk == VK_LEFT.0 || vk == VK_RIGHT.0 {
        let focus = UI.lock().unwrap().as_ref().and_then(|u| u.focus);
        if let Some(hit) = focus {
            if is_slider(hit) {
                nudge_slider(hit, if vk == VK_RIGHT.0 { 1 } else { -1 });
                return true;
            }
        }
        if move_drop_option(if vk == VK_RIGHT.0 { 1 } else { -1 }) {
            return true;
        }
        return false;
    }
    if vk == VK_UP.0 || vk == VK_DOWN.0 {
        if move_drop_option(if vk == VK_DOWN.0 { 1 } else { -1 }) {
            return true;
        }
        return false;
    }
    if vk == VK_SPACE.0 || vk == VK_RETURN.0 {
        if fb {
            return false;
        }
        let focus = UI.lock().unwrap().as_ref().and_then(|u| u.focus);
        let Some(hit) = focus else {
            return false;
        };
        if is_slider(hit) {
            return true;
        }
        activate_hit(hit);
        return true;
    }
    false
}

fn cycle_focus(forward: bool) {
    let (hits, cur, host) = {
        let ui = UI.lock().unwrap();
        let Some(ui) = ui.as_ref() else {
            return;
        };
        (ui.hits.clone(), ui.focus, ui.host)
    };
    let mut list: Vec<Hit> = Vec::new();
    for h in &hits {
        if !is_focusable(h.id) || is_drop_pick(h.id) {
            continue;
        }
        if !list.contains(&h.id) {
            list.push(h.id);
        }
    }
    if list.is_empty() {
        return;
    }
    let next = if let Some(cur) = cur.and_then(|c| list.iter().position(|h| *h == c)) {
        if forward {
            (cur + 1) % list.len()
        } else {
            (cur + list.len() - 1) % list.len()
        }
    } else if forward {
        0
    } else {
        list.len() - 1
    };
    let id = list[next];
    if let Some(ui) = UI.lock().unwrap().as_mut() {
        ui.focus = Some(id);
    }
    if matches!(id, Hit::FbText) {
        crate::feedback::set_focus(true);
    } else if matches!(id, Hit::ReplyText) {
        crate::feedback::set_compose_focus(true);
    } else {
        crate::feedback::set_focus(false);
        crate::feedback::set_compose_focus(false);
    }
    announce(host, &hit_label(id));
}

fn activate_hit(id: Hit) {
    let p = {
        let ui = UI.lock().unwrap();
        ui.as_ref().and_then(|u| {
            u.hits
                .iter()
                .filter(|h| h.id == id)
                .max_by(|a, b| (a.w * a.h).partial_cmp(&(b.w * b.h)).unwrap_or(std::cmp::Ordering::Equal))
                .map(|h| (h.x + h.w * 0.5, h.y + h.h * 0.5))
        })
    };
    let Some(p) = p else {
        return;
    };
    dispatch(id, p);
}

fn move_drop_option(dir: i32) -> bool {
    let picks: Vec<Hit> = {
        let ui = UI.lock().unwrap();
        let Some(ui) = ui.as_ref() else {
            return false;
        };
        if ui.open_drop.is_none() {
            return false;
        }
        ui.hits.iter().map(|h| h.id).filter(|id| is_drop_pick(*id)).collect::<Vec<_>>()
            .into_iter()
            .fold(Vec::new(), |mut acc, id| {
                if !acc.contains(&id) {
                    acc.push(id);
                }
                acc
            })
    };
    if picks.is_empty() {
        return false;
    }
    let cur = UI.lock().unwrap().as_ref().and_then(|u| u.hover).or(UI.lock().unwrap().as_ref().and_then(|u| u.focus));
    let idx = cur.and_then(|c| picks.iter().position(|h| *h == c)).unwrap_or(0);
    let next = (idx as i32 + dir).rem_euclid(picks.len() as i32) as usize;
    let id = picks[next];
    if let Some(ui) = UI.lock().unwrap().as_mut() {
        ui.hover = Some(id);
        ui.focus = Some(id);
    }
    true
}

fn nudge_slider(hit: Hit, delta: i32) {
    let (min, max) = slide_range(hit);
    let v = with_config(|c| match hit {
        Hit::StBg => c[WidgetId::Standings].bg,
        Hit::StHl => c.st_hl,
        Hit::RelBg => c[WidgetId::Relative].bg,
        Hit::RelHl => c.rel_hl,
        Hit::MapBg => c[WidgetId::Map].bg,
        Hit::MiniBg => c[WidgetId::Minimap].bg,
        Hit::MiniZoom => c.mini_zoom,
        Hit::RadarBg => c[WidgetId::Radar].bg,
        Hit::DashBg => c[WidgetId::Dash].bg,
        Hit::TickerBg => c[WidgetId::Ticker].bg,
        Hit::SysBg => c[WidgetId::Sys].bg,
        Hit::SectorBg => c[WidgetId::Sector].bg,
        Hit::DeltaBg => c[WidgetId::Delta].bg,
        Hit::StanceBg => c[WidgetId::Stance].bg,
        Hit::FlagBg => c[WidgetId::Flag].bg,
        Hit::StW(i) => c.st_order.get(i as usize).map(|f| f.width(c)).unwrap_or(min),
        Hit::RelW(i) => c.rel_order.get(i as usize).map(|f| f.width(c)).unwrap_or(min),
        Hit::Font(id) => c.font_pct(id),
        _ => min,
    });
    let v = (v + delta).clamp(min, max);
    update_config(|c| match hit {
        Hit::StBg => c[WidgetId::Standings].bg = v,
        Hit::StHl => c.st_hl = v,
        Hit::RelBg => c[WidgetId::Relative].bg = v,
        Hit::RelHl => c.rel_hl = v,
        Hit::MapBg => c[WidgetId::Map].bg = v,
        Hit::MiniBg => c[WidgetId::Minimap].bg = v,
        Hit::MiniZoom => c.mini_zoom = v,
        Hit::RadarBg => c[WidgetId::Radar].bg = v,
        Hit::DashBg => c[WidgetId::Dash].bg = v,
        Hit::TickerBg => c[WidgetId::Ticker].bg = v,
        Hit::SysBg => c[WidgetId::Sys].bg = v,
        Hit::SectorBg => c[WidgetId::Sector].bg = v,
        Hit::DeltaBg => c[WidgetId::Delta].bg = v,
        Hit::StanceBg => c[WidgetId::Stance].bg = v,
        Hit::FlagBg => c[WidgetId::Flag].bg = v,
        Hit::StW(i) => {
            if let Some(f) = c.st_order.get(i as usize).copied() {
                f.set_width(c, v);
            }
        }
        Hit::RelW(i) => {
            if let Some(f) = c.rel_order.get(i as usize).copied() {
                f.set_width(c, v);
            }
        }
        Hit::Font(id) => c.set_font_pct(id, v),
        _ => {}
    });
}

fn set_tab(tab: Tab) {
    crate::feedback::set_focus(false);
    if let Some(ui) = UI.lock().unwrap().as_mut() {
        if tab.is_widget() {
            ui.last_widget = tab;
        }
        ui.tab = tab;
        ui.open_drop = None;
        ui.drop_scroll = 0.0;
        ui.drop_menu = None;
        ui.drag = None;
        ui.slide = None;
        ui.scroll = 0.0;
        ui.bind_listen = false;
    }
}

fn toggle_drop(drop: Drop) {
    if let Some(ui) = UI.lock().unwrap().as_mut() {
        ui.bind_listen = false;
        if ui.open_drop == Some(drop) {
            ui.open_drop = None;
            ui.drop_scroll = 0.0;
            ui.drop_menu = None;
        } else {
            ui.open_drop = Some(drop);
            ui.drop_scroll = 0.0;
            ui.drop_menu = None;
        }
    }
}

fn close_drop() {
    if let Some(ui) = UI.lock().unwrap().as_mut() {
        ui.open_drop = None;
        ui.drop_scroll = 0.0;
        ui.drop_menu = None;
        ui.bind_listen = false;
    }
}

fn hit_at(hits: &[HitBox], x: f32, y: f32) -> Option<Hit> {
    hits.iter()
        .rev()
        .find(|h| x >= h.x && x <= h.x + h.w && y >= h.y && y <= h.y + h.h)
        .map(|h| h.id)
}

fn draw(px: &mut Pixmap, fonts: &Fonts, w: f32, h: f32) {
    refresh_palette();
    px.fill(bg());
    with_config(|cfg| draw_with_cfg(px, fonts, w, h, cfg));
}

fn draw_with_cfg(px: &mut Pixmap, fonts: &Fonts, w: f32, h: f32, cfg: &HudConfig) {
    let pending = crate::feedback::pending_reply();
    let (tab, hover, focus, open_drop, drag, scroll, nav_scroll, banner_dismissed, whats_new_open, whats_new_scroll, bind_listen, reply_scroll, reply_id) = {
        let mut ui = UI.lock().unwrap();
        if let Some(u) = ui.as_mut() {
            if !u.whats_new_open && u.reply_id.is_none() {
                if let Some(view) = pending.as_ref() {
                    u.reply_id = Some(view.id.clone());
                    u.reply_scroll = 0.0;
                    u.focus = Some(Hit::ReplyText);
                }
            }
        }
        let ui = ui.as_ref();
        (
            ui.map(|u| u.tab).unwrap_or(Tab::Standings),
            ui.and_then(|u| u.hover),
            ui.and_then(|u| u.focus),
            ui.and_then(|u| u.open_drop),
            ui.and_then(|u| u.drag),
            ui.map(|u| u.scroll).unwrap_or(0.0),
            ui.map(|u| u.nav_scroll).unwrap_or(0.0),
            ui.map(|u| u.banner_dismissed).unwrap_or(false),
            ui.map(|u| u.whats_new_open).unwrap_or(false),
            ui.map(|u| u.whats_new_scroll).unwrap_or(0.0),
            ui.map(|u| u.bind_listen).unwrap_or(false),
            ui.map(|u| u.reply_scroll).unwrap_or(0.0),
            ui.and_then(|u| u.reply_id.clone()),
        )
    };
    let tab = if tab.is_labs() && !cfg.experimental_unlocked() {
        Tab::App
    } else {
        tab
    };
    let banner = crate::update::manual_banner(cfg.auto_update_on_launch, banner_dismissed, &crate::update::state());
    let banner_h = if banner.is_some() {
        if crate::update::update_may_need_admin() {
            UPDATE_BANNER_H_ADMIN
        } else {
            UPDATE_BANNER_H
        }
    } else {
        0.0
    };
    let mut hits = Vec::new();
    DROP_MENUS.with(|menus| menus.borrow_mut().clear());

    let top_y = banner_h;
    let clip_top = top_y + TOP_H;
    let clip_bottom = h;
    let widgets = tab.is_widget();
    let side_w = if widgets { SIDE_W } else { 0.0 };

    let mut nav_content_h = 0.0;
    let mut nav_scroll = nav_scroll;
    if widgets {
        if let Some(r) = Rect::from_xywh(0.0, clip_top, SIDE_W, (h - clip_top).max(0.0)) {
            fill_rect(px, r, side());
        }
        if let Some(r) = Rect::from_xywh(SIDE_W, clip_top, 1.0, (h - clip_top).max(0.0)) {
            fill_rect(px, r, row_line());
        }
        nav_content_h = widget_rail_height(&cfg);
        let view_h = (clip_bottom - clip_top).max(0.0);
        let nav_max = (nav_content_h - view_h).max(0.0);
        nav_scroll = nav_scroll.clamp(0.0, nav_max);
        draw_widget_rail(px, fonts, &cfg, tab, hover, &mut hits, clip_top, clip_bottom, nav_scroll);
        if nav_max > 1.0 && view_h > 8.0 {
            let track_x = SIDE_W - 7.0;
            let thumb_h = (view_h * view_h / nav_content_h).clamp(16.0, view_h);
            let thumb_y = clip_top + nav_scroll / nav_max * (view_h - thumb_h);
            fill_round(px, track_x, clip_top + 4.0, 3.0, (view_h - 8.0).max(4.0), 1.5, Color::from_rgba8(255, 255, 255, 18));
            fill_round(px, track_x, thumb_y, 3.0, thumb_h, 1.5, Color::from_rgba8(255, 255, 255, 48));
        }
    }

    let x = side_w + 28.0;
    let cw = (w - x - 28.0).max(200.0);
    let py = clip_top + 20.0 - scroll;
    let bottom = match tab {
        Tab::App => pane_app(px, fonts, &cfg, hover, open_drop, &mut hits, x, py, cw),
        Tab::Feedback => pane_feedback_tab(px, fonts, hover, &mut hits, x, py, cw),
        Tab::Standings => pane_standings(px, fonts, &cfg, hover, open_drop, drag, &mut hits, x, py, cw),
        Tab::Relative => pane_relative(px, fonts, &cfg, hover, open_drop, drag, &mut hits, x, py, cw),
        Tab::Map => pane_map(px, fonts, &cfg, hover, open_drop, &mut hits, x, py, cw),
        Tab::Minimap => pane_minimap(px, fonts, &cfg, hover, open_drop, &mut hits, x, py, cw),
        Tab::Radar => pane_radar(px, fonts, &cfg, hover, &mut hits, x, py, cw),
        Tab::Dash => pane_dash(px, fonts, &cfg, hover, open_drop, &mut hits, x, py, cw),
        Tab::Ticker => pane_ticker(px, fonts, &cfg, hover, open_drop, &mut hits, x, py, cw),
        Tab::Sys => pane_sys(px, fonts, &cfg, hover, &mut hits, x, py, cw),
        Tab::Sector => pane_sector(px, fonts, &cfg, hover, &mut hits, x, py, cw),
        Tab::Delta => pane_delta(px, fonts, &cfg, hover, &mut hits, x, py, cw),
        Tab::Stance => pane_stance(px, fonts, &cfg, hover, open_drop, bind_listen, &mut hits, x, py, cw),
        Tab::Flag => pane_flag(px, fonts, &cfg, hover, &mut hits, x, py, cw),
    };
    draw_top_bar(px, fonts, w, top_y, tab, cfg.settings_key.label(), hover, &mut hits);

    if let Some(kind) = banner {
        draw_update_banner(px, fonts, w, kind, hover, &mut hits);
    }
    let mut whats_new_scroll_max = 0.0;
    let mut reply_scroll_max = 0.0;
    let reply_view = reply_id
        .as_deref()
        .and_then(crate::feedback::ticket_view)
        .or_else(|| pending.clone());
    if let Some(view) = reply_view.as_ref() {
        crate::feedback::prepare_compose(&view.id);
    }
    if whats_new_open {
        if let Some(notes) = crate::changelog::modal_notes() {
            hits.clear();
            whats_new_scroll_max = draw_whats_new(px, fonts, w, h, &notes, hover, whats_new_scroll, &mut hits);
            paint_focus(px, &hits, focus);
        }
    } else if let Some(view) = reply_view.as_ref() {
        hits.clear();
        reply_scroll_max = draw_reply(px, fonts, w, h, view, hover, reply_scroll, &mut hits);
        paint_focus(px, &hits, focus);
    } else {
        paint_drop_menus(px, fonts, hover, &mut hits);
        paint_focus(px, &hits, focus);
        paint_snap_tooltip(px, fonts, &cfg, hover, focus, &hits, w, h, clip_top);
    }

    if let Some(ui) = UI.lock().unwrap().as_mut() {
        ui.hits = hits;
        ui.content_h = bottom + scroll;
        let max = (ui.content_h - h + 24.0).max(0.0);
        ui.scroll_max = max;
        ui.scroll = ui.scroll.clamp(0.0, max);
        ui.whats_new_scroll_max = whats_new_scroll_max;
        ui.whats_new_scroll = ui.whats_new_scroll.clamp(0.0, whats_new_scroll_max);
        ui.reply_scroll_max = reply_scroll_max;
        ui.reply_scroll = ui.reply_scroll.clamp(0.0, reply_scroll_max);
        if widgets {
            ui.nav_top = clip_top;
            ui.nav_bottom = clip_bottom;
            ui.nav_content_h = nav_content_h;
            ui.nav_scroll = nav_scroll;
        } else {
            ui.nav_top = 0.0;
            ui.nav_bottom = 0.0;
            ui.nav_content_h = 0.0;
            ui.nav_scroll = 0.0;
        }
    }
}

fn snap_align_label(align: SnapAlign) -> &'static str {
    match align {
        SnapAlign::TopLeft => "Top left",
        SnapAlign::Top => "Top center",
        SnapAlign::TopRight => "Top right",
        SnapAlign::Left => "Middle left",
        SnapAlign::Center => "Center",
        SnapAlign::Right => "Middle right",
        SnapAlign::BottomLeft => "Bottom left",
        SnapAlign::Bottom => "Bottom center",
        SnapAlign::BottomRight => "Bottom right",
        SnapAlign::HCenter => "Center horizontally",
        SnapAlign::VCenter => "Center vertically",
    }
}

fn widget_short_name(id: WidgetId) -> &'static str {
    match id {
        WidgetId::Standings => "Standings",
        WidgetId::Relative => "Relative",
        WidgetId::Map => "Map",
        WidgetId::Minimap => "Minimap",
        WidgetId::Radar => "Radar",
        WidgetId::Dash => "Dash",
        WidgetId::Ticker => "H-Standings",
        WidgetId::Sys => "Systems",
        WidgetId::Sector => "Sectors",
        WidgetId::Delta => "Delta Bar",
        WidgetId::Stance => "Stance",
        WidgetId::Flag => "Flags",
    }
}

fn paint_snap_tooltip(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    focus: Option<Hit>,
    hits: &[HitBox],
    win_w: f32,
    win_h: f32,
    clip_top: f32,
) {
    let Some(Hit::Snap(id, align)) = hover.or(focus) else {
        return;
    };
    let mut cluster_l = f32::MAX;
    let mut cluster_r = 0.0f32;
    let mut cluster_t = f32::MAX;
    let mut found = false;
    for hb in hits {
        if matches!(hb.id, Hit::Snap(wid, _) if wid == id) {
            found = true;
            cluster_l = cluster_l.min(hb.x);
            cluster_r = cluster_r.max(hb.x + hb.w);
            cluster_t = cluster_t.min(hb.y);
        }
    }
    if !found {
        return;
    }

    let hint = match align {
        SnapAlign::HCenter => Some("Won't move up or down"),
        SnapAlign::VCenter => Some("Won't move left or right"),
        _ => None,
    };
    let title = snap_align_label(align);
    let name = widget_short_name(id);
    let pad = 12.0;
    let screen_w = 176.0;
    let screen_h = 99.0;
    let tw = screen_w + pad * 2.0;
    let th = pad + 16.0 + 16.0 + if hint.is_some() { 15.0 } else { 0.0 } + 8.0 + screen_h + pad;
    let mut tx = cluster_r + 16.0;
    let mut ty = cluster_t;
    if tx + tw > win_w - 16.0 {
        tx = (cluster_l - 16.0 - tw).max(16.0);
    }
    if ty < clip_top + 8.0 {
        ty = clip_top + 8.0;
    }
    if ty + th > win_h - 10.0 {
        ty = (win_h - th - 10.0).max(clip_top + 8.0);
    }

    fill_round(px, tx - 1.0, ty - 1.0, tw + 2.0, th + 2.0, 12.0, Color::from_rgba8(0, 0, 0, 90));
    outlined(px, tx, ty, tw, th, 11.0, Color::from_rgba8(22, 22, 26, 255));
    text(px, fonts, name, 11.0, tx + pad, ty + pad, muted(), false);
    text(px, fonts, title, 14.0, tx + pad, ty + pad + 15.0, text_col(), false);
    let mut sy = ty + pad + 34.0;
    if let Some(hint) = hint {
        text(px, fonts, hint, 11.0, tx + pad, sy, dim(), false);
        sy += 15.0;
    }
    let sx = tx + pad;
    outlined(px, sx, sy, screen_w, screen_h, 6.0, Color::from_rgba8(12, 12, 16, 255));
    let inset = 5.0;
    let iw = screen_w - inset * 2.0;
    let ih = screen_h - inset * 2.0;
    let ix = sx + inset;
    let iy = sy + inset;
    let before = cfg.widget_rect(id);
    let after = cfg.snapped_rect(id, align);
    paint_snap_preview_blob(px, ix, iy, iw, ih, before, Color::from_rgba8(255, 255, 255, 36));
    paint_snap_preview_blob(px, ix, iy, iw, ih, after, accent());
}

fn paint_snap_preview_blob(
    px: &mut Pixmap,
    ix: f32,
    iy: f32,
    iw: f32,
    ih: f32,
    r: crate::shm::Rect,
    c: Color,
) {
    let x = ix + r.x.clamp(0.0, 1.0) * iw;
    let y = iy + r.y.clamp(0.0, 1.0) * ih;
    let w = (r.w.clamp(0.02, 1.0) * iw).max(10.0).min((ix + iw - x).max(4.0));
    let h = (r.h.clamp(0.02, 1.0) * ih).max(8.0).min((iy + ih - y).max(4.0));
    fill_round(px, x, y, w, h, 3.0, c);
}

fn paint_focus(px: &mut Pixmap, hits: &[HitBox], focus: Option<Hit>) {
    let Some(id) = focus else {
        return;
    };
    let Some(hb) = hits
        .iter()
        .filter(|h| h.id == id)
        .max_by(|a, b| (a.w * a.h).partial_cmp(&(b.w * b.h)).unwrap_or(std::cmp::Ordering::Equal))
    else {
        return;
    };
    let pad = 3.0;
    let mut pb = PathBuilder::new();
    let x = hb.x - pad;
    let y = hb.y - pad;
    let w = hb.w + pad * 2.0;
    let h = hb.h + pad * 2.0;
    let r = 10.0;
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
        p.set_color(accent());
        p.anti_alias = true;
        px.stroke_path(
            &path,
            &p,
            &Stroke {
                width: 2.0,
                line_cap: LineCap::Round,
                line_join: LineJoin::Round,
                ..Stroke::default()
            },
            Transform::identity(),
            None,
        );
    }
}

fn widget_groups(cfg: &HudConfig) -> Vec<(&'static str, Vec<(Tab, Hit, &'static str, bool)>)> {
    let cockpit = vec![
        (Tab::Dash, Hit::TabDash, "Dash", cfg[WidgetId::Dash].show),
        (Tab::Flag, Hit::TabFlag, "Flags", cfg[WidgetId::Flag].show),
        (Tab::Sys, Hit::TabSys, "Systems", cfg[WidgetId::Sys].show),
        (Tab::Stance, Hit::TabStance, "Stance", cfg[WidgetId::Stance].show),
    ];
    let mut groups = vec![
        (
            "Boards",
            vec![
                (Tab::Standings, Hit::TabSt, "Standings", cfg[WidgetId::Standings].show),
                (Tab::Relative, Hit::TabRel, "Relative", cfg[WidgetId::Relative].show),
                (Tab::Ticker, Hit::TabTicker, "H-Standings", cfg[WidgetId::Ticker].show),
            ],
        ),
        (
            "Track",
            vec![
                (Tab::Map, Hit::TabMap, "Map", cfg[WidgetId::Map].show),
                (Tab::Minimap, Hit::TabMini, "Minimap", cfg[WidgetId::Minimap].show),
                (Tab::Radar, Hit::TabRadar, "Radar", cfg[WidgetId::Radar].show),
            ],
        ),
        ("Cockpit", cockpit),
    ];
    if cfg.experimental_unlocked() {
        groups.push((
            "Labs",
            vec![
                (Tab::Sector, Hit::TabSector, "Sectors", cfg[WidgetId::Sector].show),
                (Tab::Delta, Hit::TabDelta, "Delta Bar", cfg[WidgetId::Delta].show),
            ],
        ));
    }
    groups
}

fn widget_rail_height(cfg: &HudConfig) -> f32 {
    let mut h = 10.0;
    for (_, items) in widget_groups(cfg) {
        h += 24.0 + items.len() as f32 * 40.0 + 6.0;
    }
    h
}

fn draw_widget_rail(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    tab: Tab,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    clip_top: f32,
    clip_bottom: f32,
    nav_scroll: f32,
) {
    let clip = Some((clip_top, clip_bottom));
    let mut y = clip_top + 10.0 - nav_scroll;
    for (title, items) in widget_groups(cfg) {
        y = nav_group(px, fonts, 12.0, y, title);
        for (t, hit, name, on) in items {
            nav_tab(px, fonts, 12.0, y, SIDE_W - 24.0, 36.0, t == tab, on, name, hit, hover, hits, clip);
            y += 40.0;
        }
        y += 6.0;
    }
}

fn draw_top_bar(
    px: &mut Pixmap,
    fonts: &Fonts,
    w: f32,
    y: f32,
    tab: Tab,
    key_label: &str,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) {
    if let Some(r) = Rect::from_xywh(0.0, y, w, TOP_H) {
        fill_rect(px, r, side());
    }
    if let Some(r) = Rect::from_xywh(0.0, y + TOP_H, w, 1.0) {
        fill_rect(px, r, row_line());
    }
    let logo = brand_logo();
    let lx = 12.0;
    let ly = y + ((TOP_H - logo.height() as f32) * 0.5).max(0.0);
    let _ = px.draw_pixmap(
        lx as i32,
        ly as i32,
        logo.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
    let mut mx = lx + logo.width() as f32 + 14.0;
    let ty = y + (TOP_H - 32.0) * 0.5;
    mx += mode_tab(px, fonts, mx, ty, "Widgets", tab.is_widget(), Hit::TabWidgets, hover, hits);
    mx += mode_tab(px, fonts, mx, ty, "Settings", tab == Tab::App, Hit::TabApp, hover, hits);
    let _ = mode_tab(px, fonts, mx, ty, "Feedback", tab == Tab::Feedback, Hit::TabFeedback, hover, hits);

    let quit_w = 148.0;
    let quit_h = 32.0;
    let qx = w - 14.0 - quit_w;
    let qy = y + (TOP_H - quit_h) * 0.5;
    let hint = format!("{key_label}  settings");
    let hw = measure(fonts, &hint, 10.0);
    text(px, fonts, &hint, 10.0, qx - 16.0 - hw, y + 20.0, dim(), false);
    sidebar_quit(px, fonts, qx, qy, quit_w, quit_h, hover, hits);
}

fn mode_tab(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    label: &str,
    selected: bool,
    hit: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let size = 14.0;
    let tw = measure(fonts, label, size);
    let h = 32.0;
    let skew = 6.0;
    let bw = (tw + 28.0).max(80.0);
    hits.push(HitBox { id: hit, x, y, w: bw + skew, h });
    if selected {
        fill_skew(px, x, y, (bw - skew).max(48.0), h, skew, accent());
        text(px, fonts, label, size, x + 14.0, y + 8.0, Color::from_rgba8(20, 12, 4, 255), false);
    } else {
        if hover == Some(hit) {
            fill_round(px, x, y, bw, h, 8.0, Color::from_rgba8(255, 255, 255, 10));
        }
        text(px, fonts, label, size, x + 14.0, y + 8.0, Color::from_rgba8(210, 210, 216, 255), false);
    }
    bw + 8.0
}

fn brand_logo() -> &'static Pixmap {
    static LOGO: std::sync::OnceLock<Pixmap> = std::sync::OnceLock::new();
    LOGO.get_or_init(|| Pixmap::decode_png(include_bytes!("../icon-48.png")).expect("icon-48.png"))
}

fn draw_update_banner(
    px: &mut Pixmap,
    fonts: &Fonts,
    w: f32,
    kind: crate::update::ManualBanner,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) {
    let need_admin = crate::update::update_may_need_admin();
    let h = if need_admin { UPDATE_BANNER_H_ADMIN } else { UPDATE_BANNER_H };
    if let Some(r) = Rect::from_xywh(0.0, 0.0, w, h) {
        fill_rect(px, r, Color::from_rgba8(48, 36, 22, 255));
    }
    if let Some(r) = Rect::from_xywh(0.0, 0.0, 3.0, h) {
        fill_rect(px, r, accent());
    }
    if let Some(r) = Rect::from_xywh(0.0, h, w, 1.0) {
        fill_rect(px, r, Color::from_rgba8(255, 140, 36, 50));
    }
    hits.push(HitBox {
        id: Hit::UpdateBanner,
        x: 0.0,
        y: 0.0,
        w,
        h,
    });
    let (line, show_update) = match &kind {
        crate::update::ManualBanner::Available { version } => {
            (format!("Version {version} is available."), true)
        }
        crate::update::ManualBanner::Installing => {
            ("Downloading and installing… the app will restart.".to_string(), false)
        }
    };
    let later_w = 108.0;
    let later_x = w - 16.0 - later_w;
    let btn_h = 28.0;
    let by = (h - btn_h) * 0.5;
    text(px, fonts, &line, 13.0, 16.0, if need_admin { 10.0 } else { 16.0 }, text_col(), false);
    if need_admin {
        text(
            px,
            fonts,
            crate::update::ADMIN_UPDATE_HINT,
            11.0,
            16.0,
            30.0,
            muted(),
            false,
        );
    }
    if show_update {
        action_btn(
            px,
            fonts,
            later_x - 8.0 - 92.0,
            by,
            92.0,
            btn_h,
            "Update",
            Hit::UpdateInstall,
            hover,
            hits,
            true,
        );
    }
    action_btn(
        px,
        fonts,
        later_x,
        by,
        later_w,
        btn_h,
        "Not now",
        Hit::UpdateBannerDismiss,
        hover,
        hits,
        false,
    );
}

// THESIS: After an in-app update, a centered TV plaque names the version and the change — not a generic changelog sheet.
// OWN-WORLD: Charcoal board, orange skew version plaque, Exo 2 ExtraBold Italic, Got it as the only orange fill.
// STORY: Rider sees what this build changed, hits Got it, returns to Settings.
// FIRST VIEWPORT: Dimmed Settings; centered ~520px board; orange skew 0.1.x heading; headline; section bullets; full-width Got it.
// FORM: Center Plaque; approved .impeccable/mocks/whats-new-center.png
// FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
fn draw_whats_new(
    px: &mut Pixmap,
    fonts: &Fonts,
    win_w: f32,
    win_h: f32,
    notes: &crate::changelog::Notes,
    hover: Option<Hit>,
    scroll: f32,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let scrim = Color::from_rgba8(8, 8, 10, 204);
    if let Some(r) = Rect::from_xywh(0.0, 0.0, win_w, win_h) {
        fill_rect(px, r, scrim);
    }
    hits.push(HitBox {
        id: Hit::WhatsNewScrim,
        x: 0.0,
        y: 0.0,
        w: win_w,
        h: win_h,
    });

    let pad = 22.0;
    let plaque_h = 40.0;
    let skew = 6.0;
    let btn_h = 40.0;
    let footer_h = 16.0 + btn_h + 16.0;
    let panel_w = 520.0_f32.min(win_w - 48.0).max(320.0);
    let inner_w = (panel_w - pad * 2.0).max(200.0);
    let head_size = 16.0;
    let body_size = 12.0;
    let line_h = 18.0;

    let heads = wrap_fb(fonts, &notes.headline, inner_w, head_size);
    let mut body_h = 0.0;
    if !notes.headline.is_empty() {
        body_h += heads.len() as f32 * 22.0 + 10.0;
    }
    let mut wrapped: Vec<Vec<Vec<String>>> = Vec::new();
    for sec in &notes.sections {
        if !sec.title.is_empty() {
            body_h += 26.0;
        }
        let mut bullets = Vec::new();
        for b in &sec.bullets {
            let lines = wrap_fb(fonts, b, inner_w - 18.0, body_size);
            body_h += lines.len() as f32 * line_h + 6.0;
            bullets.push(lines);
        }
        wrapped.push(bullets);
    }

    let header_h = pad + plaque_h + 16.0;
    let want = header_h + body_h + footer_h;
    let panel_h = want.min(win_h - 48.0).max(header_h + footer_h + 24.0);
    let panel_x = ((win_w - panel_w) * 0.5).max(16.0);
    let panel_y = ((win_h - panel_h) * 0.5).max(16.0);
    let board = if high_contrast_on() {
        panel()
    } else {
        Color::from_rgba8(20, 20, 22, 255)
    };
    fill_round(px, panel_x, panel_y, panel_w, panel_h, 10.0, board);
    hits.push(HitBox {
        id: Hit::WhatsNewPanel,
        x: panel_x,
        y: panel_y,
        w: panel_w,
        h: panel_h,
    });

    let plaque_x = panel_x + pad;
    let plaque_y = panel_y + pad;
    let ver_sz = 18.0;
    let ver_w = measure(fonts, &notes.version, ver_sz);
    let plaque_w = (ver_w + 36.0).min(inner_w).max(72.0);
    fill_skew(px, plaque_x, plaque_y, (plaque_w - skew).max(48.0), plaque_h, skew, accent());
    text(
        px,
        fonts,
        &notes.version,
        ver_sz,
        plaque_x + 16.0,
        plaque_y + 10.0,
        ink(),
        false,
    );

    let body_top = plaque_y + plaque_h + 16.0;
    let body_bot = panel_y + panel_h - footer_h;
    let view_h = (body_bot - body_top).max(8.0);
    let scroll_max = (body_h - view_h).max(0.0);
    let scroll = scroll.clamp(0.0, scroll_max);

    if view_h > 8.0 && inner_w > 8.0 {
        if let Some(mut body) = Pixmap::new(inner_w.ceil() as u32, view_h.ceil() as u32) {
            body.fill(board);
            let mut y = -scroll;
            if !notes.headline.is_empty() {
                for line in &heads {
                    if y + 22.0 > 0.0 && y < view_h {
                        text(&mut body, fonts, line, head_size, 0.0, y, text_col(), false);
                    }
                    y += 22.0;
                }
                y += 10.0;
            }
            for (sec, bullets) in notes.sections.iter().zip(wrapped.iter()) {
                if !sec.title.is_empty() {
                    if y + 22.0 > 0.0 && y < view_h {
                        text(
                            &mut body,
                            fonts,
                            &sec.title.to_ascii_uppercase(),
                            10.0,
                            0.0,
                            y + 4.0,
                            dim(),
                            false,
                        );
                    }
                    y += 26.0;
                }
                for lines in bullets {
                    let block_h = lines.len() as f32 * line_h;
                    if y + block_h > 0.0 && y < view_h {
                        fill_circle(&mut body, 4.0, y + 8.0, 2.2, accent());
                        let mut ly = y;
                        for line in lines {
                            text(&mut body, fonts, line, body_size, 14.0, ly, text_col(), false);
                            ly += line_h;
                        }
                    }
                    y += block_h + 6.0;
                }
            }
            px.draw_pixmap(
                panel_x as i32 + pad as i32,
                body_top as i32,
                body.as_ref(),
                &PixmapPaint::default(),
                Transform::identity(),
                None,
            );
        }
    }

    if scroll_max > 1.0 && view_h > 16.0 {
        let track_x = panel_x + panel_w - 10.0;
        let thumb_h = (view_h * view_h / body_h.max(view_h)).clamp(16.0, view_h);
        let thumb_y = body_top
            + if scroll_max > 0.0 {
                scroll / scroll_max * (view_h - thumb_h)
            } else {
                0.0
            };
        fill_round(
            px,
            track_x,
            body_top,
            3.0,
            view_h,
            1.5,
            Color::from_rgba8(255, 255, 255, 18),
        );
        fill_round(
            px,
            track_x,
            thumb_y,
            3.0,
            thumb_h,
            1.5,
            Color::from_rgba8(255, 255, 255, 48),
        );
    }

    let btn_w = panel_w - pad * 2.0;
    let btn_x = panel_x + pad;
    let btn_y = panel_y + panel_h - 16.0 - btn_h;
    action_btn(
        px,
        fonts,
        btn_x,
        btn_y,
        btn_w,
        btn_h,
        "Got it",
        Hit::WhatsNewDismiss,
        hover,
        hits,
        true,
    );
    scroll_max
}

fn draw_reply(
    px: &mut Pixmap,
    fonts: &Fonts,
    win_w: f32,
    win_h: f32,
    view: &crate::feedback::ReplyView,
    hover: Option<Hit>,
    scroll: f32,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let scrim = Color::from_rgba8(8, 8, 10, 204);
    if let Some(r) = Rect::from_xywh(0.0, 0.0, win_w, win_h) {
        fill_rect(px, r, scrim);
    }
    hits.push(HitBox {
        id: Hit::ReplyScrim,
        x: 0.0,
        y: 0.0,
        w: win_w,
        h: win_h,
    });

    let compose = crate::feedback::compose_snapshot();
    let pad = 22.0;
    let plaque_h = 40.0;
    let skew = 6.0;
    let btn_h = 40.0;
    let box_h = 72.0;
    let footer_h = 16.0 + box_h + 10.0 + 18.0 + 8.0 + btn_h + 16.0;
    let panel_w = 520.0_f32.min(win_w - 48.0).max(320.0);
    let inner_w = (panel_w - pad * 2.0).max(200.0);
    let head_size = 16.0;
    let body_size = 12.0;
    let line_h = 18.0;
    let last_dev = view.lines.last().is_some_and(|l| l.from_dev);
    let headline = if last_dev {
        "Can you tell us more?"
    } else {
        "We wrote back"
    };
    let heads = wrap_fb(fonts, headline, inner_w, head_size);
    let wrapped: Vec<(&str, bool, Vec<String>)> = view
        .lines
        .iter()
        .map(|l| {
            (
                if l.from_dev { "Holeshot" } else { "You" },
                l.from_dev,
                wrap_fb(fonts, &l.text, inner_w, body_size),
            )
        })
        .collect();
    let mut body_h = heads.len() as f32 * 22.0 + 10.0;
    for (_, _, lines) in &wrapped {
        body_h += 16.0 + lines.len() as f32 * line_h + 10.0;
    }

    let header_h = pad + plaque_h + 16.0;
    let want = header_h + body_h + footer_h;
    let panel_h = want.min(win_h - 48.0).max(header_h + footer_h + 24.0);
    let panel_x = ((win_w - panel_w) * 0.5).max(16.0);
    let panel_y = ((win_h - panel_h) * 0.5).max(16.0);
    let board = if high_contrast_on() {
        panel()
    } else {
        Color::from_rgba8(20, 20, 22, 255)
    };
    fill_round(px, panel_x, panel_y, panel_w, panel_h, 10.0, board);
    hits.push(HitBox {
        id: Hit::ReplyPanel,
        x: panel_x,
        y: panel_y,
        w: panel_w,
        h: panel_h,
    });

    let plaque_x = panel_x + pad;
    let plaque_y = panel_y + pad;
    let kind_sz = 18.0;
    let kind_w = measure(fonts, view.kind_label, kind_sz);
    let plaque_w = (kind_w + 36.0).min(inner_w).max(72.0);
    fill_skew(px, plaque_x, plaque_y, (plaque_w - skew).max(48.0), plaque_h, skew, accent());
    text(
        px,
        fonts,
        view.kind_label,
        kind_sz,
        plaque_x + 16.0,
        plaque_y + 10.0,
        ink(),
        false,
    );

    let body_top = plaque_y + plaque_h + 16.0;
    let body_bot = panel_y + panel_h - footer_h;
    let view_h = (body_bot - body_top).max(8.0);
    let scroll_max = (body_h - view_h).max(0.0);
    let scroll = scroll.clamp(0.0, scroll_max);

    if view_h > 8.0 && inner_w > 8.0 {
        if let Some(mut body) = Pixmap::new(inner_w.ceil() as u32, view_h.ceil() as u32) {
            body.fill(board);
            let mut y = -scroll;
            for line in &heads {
                if y + 22.0 > 0.0 && y < view_h {
                    text(&mut body, fonts, line, head_size, 0.0, y, text_col(), false);
                }
                y += 22.0;
            }
            y += 10.0;
            for (who, from_dev, lines) in &wrapped {
                if y + 16.0 > 0.0 && y < view_h {
                    text(&mut body, fonts, &who.to_ascii_uppercase(), 10.0, 0.0, y, dim(), false);
                }
                y += 16.0;
                let col = if *from_dev { text_col() } else { muted() };
                for line in lines {
                    if y + line_h > 0.0 && y < view_h {
                        text(&mut body, fonts, line, body_size, 0.0, y, col, false);
                    }
                    y += line_h;
                }
                y += 10.0;
            }
            px.draw_pixmap(
                panel_x as i32 + pad as i32,
                body_top as i32,
                body.as_ref(),
                &PixmapPaint::default(),
                Transform::identity(),
                None,
            );
        }
    }

    if scroll_max > 1.0 && view_h > 16.0 {
        let track_x = panel_x + panel_w - 10.0;
        let thumb_h = (view_h * view_h / body_h.max(view_h)).clamp(16.0, view_h);
        let thumb_y = body_top
            + if scroll_max > 0.0 {
                scroll / scroll_max * (view_h - thumb_h)
            } else {
                0.0
            };
        fill_round(
            px,
            track_x,
            body_top,
            3.0,
            view_h,
            1.5,
            Color::from_rgba8(255, 255, 255, 18),
        );
        fill_round(
            px,
            track_x,
            thumb_y,
            3.0,
            thumb_h,
            1.5,
            Color::from_rgba8(255, 255, 255, 48),
        );
    }

    let box_x = panel_x + pad;
    let box_y = panel_y + panel_h - footer_h + 16.0;
    crate::feedback::set_compose_rect(box_x, box_y, inner_w, box_h);
    hits.push(HitBox {
        id: Hit::ReplyText,
        x: box_x,
        y: box_y,
        w: inner_w,
        h: box_h,
    });
    let box_fill = if compose.focused {
        Color::from_rgba8(20, 20, 24, 255)
    } else {
        btn_bg()
    };
    outlined(px, box_x, box_y, inner_w, box_h, 8.0, box_fill);
    let tx = box_x + 12.0;
    let ty = box_y + 10.0;
    let tw = (inner_w - 24.0).max(40.0);
    if compose.message.is_empty() && !compose.focused {
        text(px, fonts, "Write a reply", 12.0, tx, ty + 2.0, dim(), false);
        crate::feedback::set_caret_layout(tx, ty, 16.0, vec![vec![(0, 0.0)]]);
    } else {
        draw_fb_text(px, fonts, &compose.message, compose.cursor, tx, ty, tw, box_h - 16.0);
    }

    let status_y = box_y + box_h + 8.0;
    let (status, status_c) = match &compose.status {
        crate::feedback::Status::Idle => ("", muted()),
        crate::feedback::Status::Sending => ("Sending…", muted()),
        crate::feedback::Status::Sent => ("Sent.", accent()),
        crate::feedback::Status::Error(msg) => (msg.as_str(), Color::from_rgba8(255, 120, 100, 255)),
    };
    if !status.is_empty() {
        text(px, fonts, status, 11.0, box_x, status_y, status_c, false);
    }

    let btn_y = panel_y + panel_h - 16.0 - btn_h;
    let got_w = 120.0;
    action_btn(px, fonts, box_x, btn_y, got_w, btn_h, "Got it", Hit::ReplyDismiss, hover, hits, false);
    let sending = matches!(compose.status, crate::feedback::Status::Sending);
    action_btn(
        px,
        fonts,
        box_x + got_w + 10.0,
        btn_y,
        (inner_w - got_w - 10.0).max(100.0),
        btn_h,
        if sending { "Sending…" } else { "Send" },
        Hit::ReplySend,
        hover,
        hits,
        true,
    );
    scroll_max
}

fn nav_group(px: &mut Pixmap, fonts: &Fonts, x: f32, y: f32, label: &str) -> f32 {
    text(px, fonts, &label.to_ascii_uppercase(), 10.0, x + 14.0, y + 6.0, dim(), false);
    y + 24.0
}

fn nav_icon(px: &mut Pixmap, hit: Hit, cx: f32, cy: f32, c: Color) {
    match hit {
        Hit::TabApp => {
            icon_stroke_line(px, cx - 6.0, cy - 4.5, cx + 6.0, cy - 4.5, c, 1.6);
            icon_stroke_line(px, cx - 6.0, cy, cx + 6.0, cy, c, 1.6);
            icon_stroke_line(px, cx - 6.0, cy + 4.5, cx + 6.0, cy + 4.5, c, 1.6);
            fill_circle(px, cx - 2.0, cy - 4.5, 2.1, c);
            fill_circle(px, cx + 2.5, cy, 2.1, c);
            fill_circle(px, cx - 1.0, cy + 4.5, 2.1, c);
        }
        Hit::TabFeedback => {
            if let Some(path) = round_path(cx - 6.5, cy - 5.5, 13.0, 9.5, 2.5) {
                icon_stroke(px, &path, c, 1.5);
            }
            let mut pb = PathBuilder::new();
            pb.move_to(cx - 3.0, cy + 4.0);
            pb.line_to(cx - 5.5, cy + 7.5);
            pb.line_to(cx + 0.5, cy + 4.0);
            if let Some(path) = pb.finish() {
                icon_stroke(px, &path, c, 1.5);
            }
        }
        Hit::TabSt => {
            fill_round(px, cx - 6.8, cy + 0.5, 4.0, 6.0, 1.0, c);
            fill_round(px, cx - 2.0, cy - 4.8, 4.0, 11.3, 1.0, c);
            fill_round(px, cx + 2.8, cy + 2.2, 4.0, 4.3, 1.0, c);
        }
        Hit::TabRel => {
            icon_stroke_circle(px, cx, cy - 5.4, 2.2, c);
            fill_circle(px, cx, cy, 2.7, c);
            icon_stroke_circle(px, cx, cy + 5.4, 2.2, c);
        }
        Hit::TabMap => {
            if let Some(path) = round_path(cx - 7.0, cy - 5.2, 14.0, 10.4, 5.0) {
                icon_stroke(px, &path, c, 1.7);
            }
            if let Some(path) = round_path(cx - 3.4, cy - 2.0, 6.8, 4.0, 2.0) {
                icon_stroke(px, &path, c, 1.3);
            }
        }
        Hit::TabMini => {
            icon_stroke_circle(px, cx, cy, 6.8, c);
            let mut pb = PathBuilder::new();
            pb.move_to(cx - 3.8, cy - 1.5);
            pb.quad_to(cx + 0.5, cy - 5.2, cx + 4.2, cy - 0.5);
            if let Some(path) = pb.finish() {
                icon_stroke(px, &path, c, 1.4);
            }
            fill_circle(px, cx + 1.2, cy + 2.2, 1.8, c);
        }
        Hit::TabRadar => {
            fill_round(px, cx - 2.1, cy - 4.2, 4.2, 8.4, 1.4, c);
            let mut nose = PathBuilder::new();
            nose.move_to(cx, cy - 6.6);
            nose.line_to(cx - 2.4, cy - 3.6);
            nose.line_to(cx + 2.4, cy - 3.6);
            nose.close();
            if let Some(path) = nose.finish() {
                let mut p = Paint::default();
                p.set_color(c);
                p.anti_alias = true;
                px.fill_path(&path, &p, FillRule::Winding, Transform::identity(), None);
            }
            let mut left = PathBuilder::new();
            left.move_to(cx - 1.6, cy + 1.5);
            left.line_to(cx - 7.2, cy + 6.6);
            left.line_to(cx - 2.2, cy + 6.6);
            if let Some(path) = left.finish() {
                icon_stroke(px, &path, c, 1.4);
            }
            let mut right = PathBuilder::new();
            right.move_to(cx + 1.6, cy + 1.5);
            right.line_to(cx + 7.2, cy + 6.6);
            right.line_to(cx + 2.2, cy + 6.6);
            if let Some(path) = right.finish() {
                icon_stroke(px, &path, c, 1.4);
            }
        }
        Hit::TabDash => {
            if let Some(path) = round_path(cx - 7.2, cy - 3.6, 4.2, 7.2, 1.2) {
                icon_stroke(px, &path, c, 1.4);
            }
            if let Some(path) = round_path(cx - 2.1, cy - 3.6, 4.2, 7.2, 1.2) {
                icon_stroke(px, &path, c, 1.4);
            }
            if let Some(path) = round_path(cx + 3.0, cy - 3.6, 4.2, 7.2, 1.2) {
                icon_stroke(px, &path, c, 1.4);
            }
        }
        Hit::TabTicker => {
            if let Some(path) = round_path(cx - 7.0, cy - 4.2, 6.2, 8.4, 1.5) {
                icon_stroke(px, &path, c, 1.4);
            }
            fill_round(px, cx - 2.2, cy - 4.2, 6.2, 8.4, 1.5, c);
            if let Some(path) = round_path(cx + 2.6, cy - 4.2, 6.2, 8.4, 1.5) {
                icon_stroke(px, &path, c, 1.4);
            }
        }
        Hit::TabSys => {
            fill_round(px, cx - 7.0, cy - 5.6, 14.0, 2.2, 1.1, c);
            fill_round(px, cx - 7.0, cy - 1.1, 10.0, 2.2, 1.1, c);
            fill_round(px, cx - 7.0, cy + 3.4, 7.0, 2.2, 1.1, c);
        }
        Hit::TabSector => {
            fill_round(px, cx - 6.8, cy - 5.4, 13.6, 3.2, 1.0, c);
            fill_round(px, cx - 6.8, cy - 1.1, 13.6, 3.2, 1.0, c);
            fill_round(px, cx - 6.8, cy + 3.2, 13.6, 3.2, 1.0, c);
        }
        Hit::TabDelta => {
            fill_round(px, cx - 7.0, cy - 1.6, 14.0, 3.2, 1.4, c);
            fill_round(px, cx - 0.7, cy - 5.4, 1.4, 10.8, 0.6, c);
        }
        Hit::TabStance => {
            fill_round(px, cx - 5.4, cy + 1.4, 4.4, 4.2, 1.0, c);
            fill_round(px, cx + 1.0, cy - 5.2, 4.4, 10.8, 1.0, c);
        }
        Hit::TabFlag => {
            icon_stroke_line(px, cx - 5.6, cy - 6.2, cx - 5.6, cy + 6.2, c, 1.6);
            let mut pb = PathBuilder::new();
            pb.move_to(cx - 5.0, cy - 5.8);
            pb.line_to(cx + 6.2, cy - 3.4);
            pb.line_to(cx + 4.4, cy + 0.6);
            pb.line_to(cx - 5.0, cy - 1.2);
            pb.close();
            if let Some(path) = pb.finish() {
                icon_stroke(px, &path, c, 1.4);
            }
        }
        Hit::QuitApp => {
            icon_stroke_circle(px, cx, cy, 6.2, c);
            icon_stroke_line(px, cx, cy - 7.2, cx, cy - 1.2, c, 1.7);
        }
        Hit::FbRate => fill_star(px, cx, cy, 6.2, c),
        Hit::FbBug => {
            let mut pb = PathBuilder::new();
            pb.move_to(cx, cy - 6.4);
            pb.line_to(cx + 6.6, cy + 5.4);
            pb.line_to(cx - 6.6, cy + 5.4);
            pb.close();
            if let Some(path) = pb.finish() {
                icon_stroke(px, &path, c, 1.5);
            }
            icon_stroke_line(px, cx, cy - 2.2, cx, cy + 1.4, c, 1.6);
            fill_circle(px, cx, cy + 3.4, 1.05, c);
        }
        Hit::FbFeature => {
            icon_stroke_circle(px, cx, cy - 1.4, 4.0, c);
            icon_stroke_line(px, cx, cy + 2.6, cx, cy + 6.2, c, 1.6);
            icon_stroke_line(px, cx - 2.4, cy + 6.2, cx + 2.4, cy + 6.2, c, 1.6);
            icon_stroke_line(px, cx - 1.6, cy + 4.4, cx + 1.6, cy + 4.4, c, 1.5);
        }
        _ => {}
    }
}

fn icon_stroke(px: &mut Pixmap, path: &Path, c: Color, width: f32) {
    let mut p = Paint::default();
    p.set_color(c);
    p.anti_alias = true;
    px.stroke_path(
        path,
        &p,
        &Stroke {
            width,
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            ..Stroke::default()
        },
        Transform::identity(),
        None,
    );
}

fn icon_stroke_line(px: &mut Pixmap, x0: f32, y0: f32, x1: f32, y1: f32, c: Color, width: f32) {
    let mut pb = PathBuilder::new();
    pb.move_to(x0, y0);
    pb.line_to(x1, y1);
    if let Some(path) = pb.finish() {
        icon_stroke(px, &path, c, width);
    }
}

fn icon_stroke_circle(px: &mut Pixmap, cx: f32, cy: f32, r: f32, c: Color) {
    let mut pb = PathBuilder::new();
    pb.push_circle(cx, cy, r);
    if let Some(path) = pb.finish() {
        icon_stroke(px, &path, c, 1.5);
    }
}

fn nav_tab(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    selected: bool,
    visible: bool,
    name: &str,
    hit: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    clip: Option<(f32, f32)>,
) {
    if let Some((top, bot)) = clip {
        if y + h <= top || y >= bot {
            return;
        }
        let hy = y.max(top);
        let hh = (y + h).min(bot) - hy;
        if hh > 1.0 {
            hits.push(HitBox { id: hit, x, y: hy, w, h: hh });
        }
    } else {
        hits.push(HitBox { id: hit, x, y, w, h });
    }
    if selected {
        fill_round(px, x, y, w, h, 8.0, tab_on());
        fill_round(px, x, y + 8.0, 3.0, h - 16.0, 1.5, accent());
    } else if hover == Some(hit) {
        fill_round(px, x, y, w, h, 8.0, Color::from_rgba8(255, 255, 255, 10));
    }
    let name_c = if selected { accent() } else { Color::from_rgba8(210, 210, 216, 255) };
    nav_icon(px, hit, x + 18.0, y + h * 0.5, name_c);
    text(px, fonts, name, 13.0, x + 32.0, y + 10.0, name_c, false);
    let dx = x + w - 16.0;
    let dy = y + h * 0.5;
    if visible {
        fill_circle(px, dx, dy, 3.5, accent());
    } else {
        icon_stroke_circle(px, dx, dy, 3.5, muted());
    }
}

fn pane_app(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    open_drop: Option<Drop>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let mut y = heading(
        px,
        fonts,
        x,
        y,
        w,
        "Settings",
        "Font and units apply to every widget",
        None,
        hover,
        hits,
    );
    y = section(px, fonts, x, y, "Install");
    text(px, fonts, "Installed to", 10.0, x + 2.0, y + 2.0, dim(), false);
    y += 18.0;
    let install = fit_path(fonts, &crate::update::install_dir_display(), 12.0, w - 8.0);
    text(px, fonts, &install, 13.0, x + 4.0, y, text_col(), false);
    y += 22.0;
    text(px, fonts, "MX Bikes", 10.0, x + 2.0, y + 2.0, dim(), false);
    y += 18.0;
    let game = crate::plugin::game_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "Not found".into());
    let game = fit_path(fonts, &game, 12.0, w - 148.0);
    text(px, fonts, &game, 13.0, x + 4.0, y, text_col(), false);
    action_btn(px, fonts, x + w - 132.0, y - 6.0, 132.0, 28.0, "Change folder", Hit::GameFolder, hover, hits, false);
    y += 28.0;
    let plugin_line = if crate::plugin::plugin_installed() {
        "Plugin is in the game plugins folder."
    } else {
        "Plugin is missing. Fully quit MX Bikes, then Change folder or restart the overlay."
    };
    let plugin_col = if crate::plugin::plugin_installed() {
        muted()
    } else {
        accent()
    };
    text(px, fonts, plugin_line, 11.0, x + 4.0, y, plugin_col, false);
    y += 22.0;
    if crate::update::update_may_need_admin() {
        text(px, fonts, crate::update::ADMIN_UPDATE_HINT, 11.0, x + 4.0, y, accent(), false);
        y += 20.0;
    }
    y = section(px, fonts, x, y, "Look");
    y = dropdown_row(
        px,
        fonts,
        x,
        y,
        w,
        "Font",
        cfg.font_family.label(),
        open_drop == Some(Drop::FontFamily),
        Hit::FontOpen,
        &[
            (Hit::FontSegoe, "Segoe UI", cfg.font_family == FontFamily::Segoe),
            (Hit::FontArial, "Arial", cfg.font_family == FontFamily::Arial),
            (Hit::FontTahoma, "Tahoma", cfg.font_family == FontFamily::Tahoma),
            (Hit::FontRoboto, "Roboto", cfg.font_family == FontFamily::Roboto),
            (Hit::FontExo2, "Exo 2", cfg.font_family == FontFamily::Exo2),
            (Hit::FontTeko, "Teko", cfg.font_family == FontFamily::Teko),
            (Hit::FontGoldman, "Goldman", cfg.font_family == FontFamily::Goldman),
            (Hit::FontMontserrat, "Montserrat", cfg.font_family == FontFamily::Montserrat),
        ],
        hover,
        hits,
    );
    y = dropdown_row(
        px,
        fonts,
        x,
        y,
        w,
        "Units",
        cfg.units.label(),
        open_drop == Some(Drop::Units),
        Hit::UnitsOpen,
        &[
            (Hit::UnitsMetric, "Metric", cfg.units == Units::Metric),
            (Hit::UnitsImperial, "Imperial", cfg.units == Units::Imperial),
        ],
        hover,
        hits,
    );
    y = dropdown_row(
        px,
        fonts,
        x,
        y,
        w,
        "Settings key",
        cfg.settings_key.label(),
        open_drop == Some(Drop::SettingsKey),
        Hit::SettingsKeyOpen,
        &SettingsKey::ALL.map(|key| (Hit::SettingsKeyPick(key), key.label(), cfg.settings_key == key)),
        hover,
        hits,
    );
    text(px, fonts, "Press again to close. Medal and other clip apps use F8. F9 still rotates the clock log.", 11.0, x + 4.0, y + 2.0, dim(), false);
    y += 22.0;
    y = section(px, fonts, x, y, "Startup");
    y = toggle_row(px, fonts, x, y, w, "Open when Windows starts", cfg.start_with_windows, Hit::StartWithWindows, hover, hits);
    if cfg.start_with_windows {
        text(
            px,
            fonts,
            "Starts in the tray at login. The tray icon or settings key opens settings.",
            11.0,
            x + 4.0,
            y + 2.0,
            dim(),
            false,
        );
        y += 22.0;
    }
    y = toggle_row(px, fonts, x, y, w, "Minimize on close", cfg.minimize_on_close, Hit::MinimizeOnClose, hover, hits);
    if cfg.minimize_on_close {
        text(
            px,
            fonts,
            &format!(
                "Close, Open when Windows starts, and Open when MX Bikes opens hide to the tray. {} or the tray icon brings settings back. Quit overlay exits.",
                cfg.settings_key.label()
            ),
            11.0,
            x + 4.0,
            y + 2.0,
            dim(),
            false,
        );
        y += 22.0;
    }
    y = toggle_row(px, fonts, x, y, w, "Close when MX Bikes closes", cfg.close_with_game, Hit::CloseWithGame, hover, hits);
    if cfg.close_with_game {
        text(
            px,
            fonts,
            if cfg.open_with_game {
                "Hides to the tray. Opening MX Bikes brings the HUD back."
            } else {
                "Quits the overlay a few seconds after the game exits."
            },
            11.0,
            x + 4.0,
            y + 2.0,
            dim(),
            false,
        );
        y += 22.0;
    }
    y = toggle_row(px, fonts, x, y, w, "Open when MX Bikes opens", cfg.open_with_game, Hit::OpenWithGame, hover, hits);
    if cfg.open_with_game {
        text(px, fonts, "Starts the overlay in the tray when MX Bikes launches, including after you close the game. F8 or the HUD mark opens settings.", 11.0, x + 4.0, y + 2.0, dim(), false);
        y += 22.0;
    }
    y = section(px, fonts, x, y, "Session");
    y = toggle_row(px, fonts, x, y, w, "Show overlay users", cfg.show_presence, Hit::ShowPresence, hover, hits);
    text(px, fonts, "Holeshot mark before the name on tables and an orange ring on the map when they also run Holeshot. Off until you turn it on.", 11.0, x + 4.0, y + 2.0, dim(), false);
    y += 36.0;
    y = toggle_row(px, fonts, x, y, w, "Highlight Steam friends", cfg.highlight_friends, Hit::HighlightFriends, hover, hits);
    text(px, fonts, "Teal map dots and a Friend column on tables when a Steam friend also runs Holeshot. Needs Show overlay users. Friends without the overlay cannot be marked. Off until you turn it on.", 11.0, x + 4.0, y + 2.0, dim(), false);
    y += 48.0;
    y = section(px, fonts, x, y, "Labs");
    y = toggle_row(px, fonts, x, y, w, "Experimental widgets", cfg.experimental, Hit::FeatureSector, hover, hits);
    text(px, fonts, "Adds Sectors and Delta Bar. Off until you turn this on.", 11.0, x + 4.0, y + 2.0, dim(), false);
    y += 22.0;
    y = section(px, fonts, x, y, "Updates");
    let need_admin = crate::update::update_may_need_admin();
    y = toggle_row(px, fonts, x, y, w, "Update automatically on launch", cfg.auto_update_on_launch, Hit::AutoUpdateOnLaunch, hover, hits);
    if cfg.auto_update_on_launch {
        text(px, fonts, "Checks GitHub before opening and installs if a newer version is out.", 11.0, x + 4.0, y + 2.0, dim(), false);
        y += 22.0;
    } else {
        text(px, fonts, "When a newer version is out, a banner at the top can install it.", 11.0, x + 4.0, y + 2.0, dim(), false);
        y += 22.0;
    }
    if need_admin {
        text(px, fonts, crate::update::ADMIN_UPDATE_HINT, 11.0, x + 4.0, y + 2.0, accent(), false);
        y += 22.0;
    }
    let update = crate::update::state();
    let (status, extra, show_check, show_install) = match &update {
        crate::update::UpdateState::Idle => ("Check GitHub for a newer build.", None, true, false),
        crate::update::UpdateState::Checking => ("Checking…", None, false, false),
        crate::update::UpdateState::Current => ("You already have the latest version.", None, true, false),
        crate::update::UpdateState::Available { version, .. } => (
            "A newer version is ready to install.",
            Some(format!("Version {version} is available.")),
            true,
            true,
        ),
        crate::update::UpdateState::Downloading => {
            ("Downloading and installing… the app will restart.", None, false, false)
        }
        crate::update::UpdateState::Failed(msg) => (msg.as_str(), None, true, false),
    };
    let mut card_h = if extra.is_some() { 168.0 } else { 148.0 };
    if need_admin && show_install {
        card_h += 18.0;
    }
    outlined(px, x, y, w, card_h, 10.0, panel());
    text(px, fonts, "Installed", 10.0, x + 16.0, y + 14.0, dim(), false);
    text(
        px,
        fonts,
        crate::update::current_version(),
        16.0,
        x + 16.0,
        y + 30.0,
        text_col(),
        false,
    );
    let mut iy = y + 58.0;
    if let Some(line) = extra.as_deref() {
        text(px, fonts, line, 13.0, x + 16.0, iy, accent(), false);
        iy += 22.0;
    }
    text(px, fonts, status, 12.0, x + 16.0, iy, muted(), false);
    iy += 26.0;
    if need_admin && show_install {
        text(px, fonts, crate::update::ADMIN_UPDATE_HINT, 11.0, x + 16.0, iy, accent(), false);
        iy += 20.0;
    }
    let btn_w = 156.0;
    let mut bx = x + 16.0;
    if show_check {
        action_btn(px, fonts, bx, iy, btn_w, 32.0, "Check for updates", Hit::UpdateCheck, hover, hits, false);
        bx += btn_w + 10.0;
    }
    if crate::changelog::modal_notes().is_some() {
        let label = if crate::changelog::previewing()
            && crate::changelog::next_notes().as_ref().map(|n| n.version.as_str())
                != crate::changelog::current_notes().as_ref().map(|n| n.version.as_str())
        {
            "Preview next"
        } else {
            "What's new"
        };
        action_btn(px, fonts, bx, iy, 132.0, 32.0, label, Hit::WhatsNewOpen, hover, hits, false);
        bx += 142.0;
    }
    if show_install {
        action_btn(
            px,
            fonts,
            bx,
            iy,
            168.0,
            32.0,
            "Download and install",
            Hit::UpdateInstall,
            hover,
            hits,
            true,
        );
    }
    y += card_h + 14.0;
    action_btn(px, fonts, x, y, 120.0, 32.0, "Quit overlay", Hit::QuitApp, hover, hits, false);
    action_btn(px, fonts, x + 132.0, y, 120.0, 32.0, "Uninstall", Hit::Uninstall, hover, hits, false);
    y += 46.0;
    text(
        px,
        fonts,
        "Uninstall removes the overlay, MX Bikes plugin, and shortcuts.",
        11.0,
        x,
        y,
        dim(),
        false,
    );
    y += 18.0;
    text(
        px,
        fonts,
        "The overlay installs the MX Bikes plugin when you start it. Restart the game only if it was already open.",
        11.0,
        x,
        y,
        dim(),
        false,
    );
    y + 28.0
}

fn pane_feedback_tab(
    px: &mut Pixmap,
    fonts: &Fonts,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let y = heading(
        px,
        fonts,
        x,
        y,
        w,
        "Feedback",
        "Rate the app, report a bug, or ask for a feature",
        None,
        hover,
        hits,
    );
    pane_feedback(px, fonts, hover, hits, x, y, w)
}

fn pane_feedback(
    px: &mut Pixmap,
    fonts: &Fonts,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let fb = crate::feedback::snapshot();
    let bug = fb.kind == crate::feedback::Kind::Bug;
    let feature = fb.kind == crate::feedback::Kind::Feature;
    let show_stars = !feature;
    let attach_h = if bug { 52.0 } else { 0.0 };
    let stars_h = if show_stars { 56.0 } else { 8.0 };
    let card_h = 224.0 + stars_h + attach_h;
    outlined(px, x, y, w, card_h, 10.0, panel());

    let gap = 8.0;
    let chip_w = (w - 32.0 - gap * 2.0) / 3.0;
    let cy = y + 14.0;
    kind_chip(px, fonts, x + 16.0, cy, chip_w, "Rate", Hit::FbRate, fb.kind == crate::feedback::Kind::Rate, hover, hits);
    kind_chip(px, fonts, x + 16.0 + chip_w + gap, cy, chip_w, "Bug", Hit::FbBug, bug, hover, hits);
    kind_chip(px, fonts, x + 16.0 + (chip_w + gap) * 2.0, cy, chip_w, "Feature", Hit::FbFeature, feature, hover, hits);

    let prompt = match fb.kind {
        crate::feedback::Kind::Bug => "How bad is it? (optional)",
        crate::feedback::Kind::Feature => "What should we add?",
        crate::feedback::Kind::Rate => "How is it going?",
    };
    let sy = y + 56.0;
    text(px, fonts, prompt, 11.0, x + 16.0, sy, dim(), false);
    let mut box_y = sy + 24.0;
    if show_stars {
        let star_y = sy + 20.0;
        for i in 1u8..=5 {
            let sx = x + 12.0 + (i as f32 - 1.0) * 34.0;
            hits.push(HitBox { id: Hit::FbStar(i), x: sx, y: star_y, w: 32.0, h: 28.0 });
            let on = fb.rating >= i;
            let hot = hover == Some(Hit::FbStar(i));
            let col = if on {
                accent()
            } else if hot {
                Color::from_rgba8(255, 140, 36, 140)
            } else {
                Color::from_rgba8(72, 72, 80, 255)
            };
            fill_star(px, sx + 16.0, star_y + 14.0, 10.0, col);
        }
        box_y = star_y + 36.0;
    }
    let box_h = 88.0;
    crate::feedback::set_text_rect(x + 16.0, box_y, w - 32.0, box_h);
    hits.push(HitBox { id: Hit::FbText, x: x + 16.0, y: box_y, w: w - 32.0, h: box_h });
    let box_fill = if fb.focused {
        Color::from_rgba8(20, 20, 24, 255)
    } else {
        btn_bg()
    };
    outlined(px, x + 16.0, box_y, w - 32.0, box_h, 8.0, box_fill);
    let placeholder = match fb.kind {
        crate::feedback::Kind::Bug => "What went wrong?",
        crate::feedback::Kind::Feature => "Describe the feature.",
        crate::feedback::Kind::Rate => "Anything you want to add? (optional)",
    };
    let tx = x + 28.0;
    let ty = box_y + 10.0;
    let tw = w - 56.0;
    if fb.message.is_empty() && !fb.focused {
        text(px, fonts, placeholder, 12.0, tx, ty + 2.0, dim(), false);
        crate::feedback::set_caret_layout(tx, ty, 16.0, vec![vec![(0, 0.0)]]);
    } else {
        draw_fb_text(px, fonts, &fb.message, fb.cursor, tx, ty, tw, box_h - 16.0);
    }

    let mut iy = box_y + box_h + 12.0;
    if bug {
        hits.push(HitBox { id: Hit::FbAttach, x: x + 16.0, y: iy, w: w - 32.0, h: 44.0 });
        let check_col = if fb.attach_log { accent() } else { track_off() };
        fill_round(px, x + 16.0, iy + 6.0, 18.0, 18.0, 4.0, check_col);
        if fb.attach_log {
            let mut pb = PathBuilder::new();
            pb.move_to(x + 20.0, iy + 15.0);
            pb.line_to(x + 24.0, iy + 19.0);
            pb.line_to(x + 30.0, iy + 11.0);
            if let Some(path) = pb.finish() {
                let mut p = Paint::default();
                p.set_color(Color::from_rgba8(20, 12, 4, 255));
                p.anti_alias = true;
                px.stroke_path(
                    &path,
                    &p,
                    &Stroke { width: 2.0, ..Stroke::default() },
                    Transform::identity(),
                    None,
                );
            }
        }
        let log_c = if crate::feedback::has_log() { muted() } else { dim() };
        text(px, fonts, "Include last race log", 13.0, x + 42.0, iy + 8.0, text_col(), false);
        text(px, fonts, &crate::feedback::log_label(), 11.0, x + 42.0, iy + 24.0, log_c, false);
        iy += 52.0;
    }

    let sending = matches!(fb.status, crate::feedback::Status::Sending);
    action_btn(px, fonts, x + 16.0, iy, 120.0, 32.0, if sending { "Sending…" } else { "Send" }, Hit::FbSend, hover, hits, true);
    let (status, status_c) = match &fb.status {
        crate::feedback::Status::Idle => ("", muted()),
        crate::feedback::Status::Sending => ("Sending…", muted()),
        crate::feedback::Status::Sent => ("Sent. Thank you.", accent()),
        crate::feedback::Status::Error(msg) => (msg.as_str(), Color::from_rgba8(255, 120, 100, 255)),
    };
    if !status.is_empty() {
        text(px, fonts, status, 11.0, x + 148.0, iy + 10.0, status_c, false);
    }
    y + card_h + 14.0
}

fn kind_chip(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    hit: Hit,
    on: bool,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) {
    hits.push(HitBox { id: hit, x, y, w, h: 32.0 });
    let col = if on { accent() } else { text_col() };
    if on {
        fill_round(px, x, y, w, 32.0, 8.0, accent_dim());
    } else {
        let fill = if hover == Some(hit) { chip_hover() } else { btn_bg() };
        outlined(px, x, y, w, 32.0, 8.0, fill);
    }
    let tw = measure(fonts, label, 13.0);
    let total = 18.0 + tw;
    let start = x + ((w - total) * 0.5).max(8.0);
    nav_icon(px, hit, start + 6.0, y + 16.0, col);
    text(px, fonts, label, 13.0, start + 16.0, y + 8.0, col, false);
}

fn draw_fb_text(
    px: &mut Pixmap,
    fonts: &Fonts,
    s: &str,
    cursor: usize,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) {
    const SIZE: f32 = 12.0;
    const LINE: f32 = 16.0;
    let lines = wrap_fb(fonts, s, w, SIZE);
    let max_rows = (h / LINE).max(1.0) as usize;
    let caret_line = {
        let mut i = 0usize;
        let mut line = 0usize;
        for (li, line_s) in lines.iter().enumerate() {
            let end = i + line_s.len();
            if cursor <= end {
                line = li;
                break;
            }
            i = end;
            if s.get(i..).is_some_and(|r| r.starts_with('\n')) {
                i += 1;
            }
            line = li + 1;
        }
        line
    };
    let start = caret_line.saturating_sub(max_rows.saturating_sub(1));
    let mut drawn = 0usize;
    let mut idx = 0usize;
    let mut rows = Vec::new();
    for (li, line) in lines.iter().enumerate() {
        if li >= start && drawn < max_rows {
            text(px, fonts, line, SIZE, x, y + drawn as f32 * LINE, text_col(), false);
            let mut stops = vec![(idx, 0.0)];
            let mut prefix = String::new();
            for (off, ch) in line.char_indices() {
                prefix.push(ch);
                stops.push((idx + off + ch.len_utf8(), measure(fonts, &prefix, SIZE)));
            }
            if crate::feedback::caret_on() && li == caret_line {
                let cx = x + stops
                    .iter()
                    .rev()
                    .find(|(b, _)| *b <= cursor)
                    .map(|s| s.1)
                    .unwrap_or(0.0);
                if let Some(r) = Rect::from_xywh(cx, y + drawn as f32 * LINE, 1.5, 14.0) {
                    fill_rect(px, r, accent());
                }
            }
            rows.push(stops);
            drawn += 1;
        }
        idx += line.len();
        if s.get(idx..).is_some_and(|r| r.starts_with('\n')) {
            idx += 1;
        }
    }
    if s.is_empty() {
        rows.push(vec![(0, 0.0)]);
        if crate::feedback::caret_on() {
            if let Some(r) = Rect::from_xywh(x, y, 1.5, 14.0) {
                fill_rect(px, r, accent());
            }
        }
    }
    crate::feedback::set_caret_layout(x, y, LINE, rows);
}

fn wrap_fb(fonts: &Fonts, s: &str, max_w: f32, size: f32) -> Vec<String> {
    let mut lines = Vec::new();
    if s.is_empty() {
        lines.push(String::new());
        return lines;
    }
    for para in s.split('\n') {
        if para.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut line = String::new();
        let mut line_w = 0.0;
        for token in para.split_inclusive(' ') {
            push_wrap_token(fonts, &mut lines, &mut line, &mut line_w, token, max_w, size);
        }
        lines.push(line);
    }
    lines
}

fn push_wrap_token(
    fonts: &Fonts,
    lines: &mut Vec<String>,
    line: &mut String,
    line_w: &mut f32,
    token: &str,
    max_w: f32,
    size: f32,
) {
    let token_w = measure(fonts, token, size);
    if !line.is_empty() && *line_w + token_w > max_w {
        lines.push(std::mem::take(line));
        *line_w = 0.0;
    }
    if token_w <= max_w {
        line.push_str(token);
        *line_w += token_w;
        return;
    }
    for ch in token.chars() {
        let ch_w = measure(fonts, ch.encode_utf8(&mut [0; 4]), size);
        if !line.is_empty() && *line_w + ch_w > max_w {
            lines.push(std::mem::take(line));
            *line_w = 0.0;
        }
        line.push(ch);
        *line_w += ch_w;
    }
}

fn fill_star(px: &mut Pixmap, cx: f32, cy: f32, r: f32, c: Color) {
    let mut pb = PathBuilder::new();
    for i in 0..10 {
        let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
        let rad = if i % 2 == 0 { r } else { r * 0.42 };
        let px_ = cx + a.cos() * rad;
        let py = cy + a.sin() * rad;
        if i == 0 {
            pb.move_to(px_, py);
        } else {
            pb.line_to(px_, py);
        }
    }
    pb.close();
    let Some(path) = pb.finish() else {
        return;
    };
    let mut p = Paint::default();
    p.set_color(c);
    p.anti_alias = true;
    px.fill_path(&path, &p, FillRule::Winding, Transform::identity(), None);
}

fn sidebar_quit(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) {
    hits.push(HitBox { id: Hit::QuitApp, x, y, w, h });
    let fill = if hover == Some(Hit::QuitApp) { chip_hover() } else { btn_bg() };
    outlined(px, x, y, w, h, 8.0, fill);
    let c = if hover == Some(Hit::QuitApp) { accent() } else { text_col() };
    nav_icon(px, Hit::QuitApp, x + 18.0, y + h * 0.5, c);
    text(px, fonts, "Quit overlay", 13.0, x + 32.0, y + 10.0, c, false);
}

fn action_btn(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    label: &str,
    hit: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    primary: bool,
) {
    hits.push(HitBox { id: hit, x, y, w, h });
    if primary {
        let fill = if hover == Some(hit) {
            Color::from_rgba8(255, 156, 56, 255)
        } else {
            accent()
        };
        fill_round(px, x, y, w, h, 8.0, fill);
        text(px, fonts, label, 13.0, x + w * 0.5, y + 8.0, Color::from_rgba8(20, 12, 4, 255), true);
    } else {
        let fill = if hover == Some(hit) { chip_hover() } else { btn_bg() };
        outlined(px, x, y, w, h, 8.0, fill);
        text(px, fonts, label, 13.0, x + w * 0.5, y + 8.0, text_col(), true);
    }
}

#[derive(Clone, Copy)]
struct WidgetPaneSpec {
    id: WidgetId,
    title: &'static str,
    subtitle: &'static str,
    show: Hit,
    bg: Hit,
    bg_label: &'static str,
}

fn widget_pane_spec(id: WidgetId) -> WidgetPaneSpec {
    match id {
        WidgetId::Standings => WidgetPaneSpec {
            id,
            title: "Standings",
            subtitle: "Who is ahead and by how much",
            show: Hit::StShow,
            bg: Hit::StBg,
            bg_label: "Background",
        },
        WidgetId::Relative => WidgetPaneSpec {
            id,
            title: "Relative",
            subtitle: "Riders just ahead and behind you",
            show: Hit::RelShow,
            bg: Hit::RelBg,
            bg_label: "Background",
        },
        WidgetId::Map => WidgetPaneSpec {
            id,
            title: "Map",
            subtitle: "Where you and others are on track",
            show: Hit::MapShow,
            bg: Hit::MapBg,
            bg_label: "Background",
        },
        WidgetId::Minimap => WidgetPaneSpec {
            id,
            title: "Minimap",
            subtitle: "Circular track with numbered riders",
            show: Hit::MiniShow,
            bg: Hit::MiniBg,
            bg_label: "Background",
        },
        WidgetId::Radar => WidgetPaneSpec {
            id,
            title: "Radar",
            subtitle: "Riders beside and behind you",
            show: Hit::RadarShow,
            bg: Hit::RadarBg,
            bg_label: "Panel opacity",
        },
        WidgetId::Dash => WidgetPaneSpec {
            id,
            title: "Dash",
            subtitle: "Gear, speed, and footer stats",
            show: Hit::DashShow,
            bg: Hit::DashBg,
            bg_label: "Panel opacity",
        },
        WidgetId::Ticker => WidgetPaneSpec {
            id,
            title: "Horizontal Standings",
            subtitle: "Your name is highlighted in the field",
            show: Hit::TickerShow,
            bg: Hit::TickerBg,
            bg_label: "Panel opacity",
        },
        WidgetId::Sys => WidgetPaneSpec {
            id,
            title: "Systems",
            subtitle: "CPU, memory, FPS, network, and per-app load",
            show: Hit::SysShow,
            bg: Hit::SysBg,
            bg_label: "Panel opacity",
        },
        WidgetId::Sector => WidgetPaneSpec {
            id,
            title: "Sectors",
            subtitle: "Split times vs your best at this point in the sector",
            show: Hit::SectorShow,
            bg: Hit::SectorBg,
            bg_label: "Panel opacity",
        },
        WidgetId::Delta => WidgetPaneSpec {
            id,
            title: "Delta Bar",
            subtitle: "Time vs your best at this point on the lap",
            show: Hit::DeltaShow,
            bg: Hit::DeltaBg,
            bg_label: "Panel opacity",
        },
        WidgetId::Stance => WidgetPaneSpec {
            id,
            title: "Stance",
            subtitle: "Sit / stand from a bind you set",
            show: Hit::StanceShow,
            bg: Hit::StanceBg,
            bg_label: "Panel opacity",
        },
        WidgetId::Flag => WidgetPaneSpec {
            id,
            title: "Flags",
            subtitle: "White and checkered — same timing as Dash",
            show: Hit::FlagShow,
            bg: Hit::FlagBg,
            bg_label: "Panel opacity",
        },
    }
}

fn open_widget_pane(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
    spec: WidgetPaneSpec,
    body: impl FnOnce(&mut Pixmap, &Fonts, f32, bool, &mut Vec<HitBox>) -> f32,
) -> f32 {
    let mut y = heading(
        px,
        fonts,
        x,
        y,
        w,
        spec.title,
        spec.subtitle,
        Some((cfg[spec.id].show, spec.show)),
        hover,
        hits,
    );
    let shown = cfg[spec.id].show;
    y = body(px, fonts, y, shown, hits);
    if shown {
        y = look_section(px, fonts, x, y, w, spec.id, hover, hits);
    }
    y
}

fn pane_style(
    px: &mut Pixmap,
    fonts: &Fonts,
    spec: WidgetPaneSpec,
    cfg: &HudConfig,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    style_controls(
        px,
        fonts,
        x,
        y,
        w,
        spec.id,
        cfg,
        spec.bg_label,
        cfg[spec.id].bg,
        spec.bg,
        hover,
        hits,
    )
}

fn table_style_controls(
    px: &mut Pixmap,
    fonts: &Fonts,
    spec: WidgetPaneSpec,
    cfg: &HudConfig,
    hover: Option<Hit>,
    open_drop: Option<Drop>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
    hl: i32,
    hl_hit: Hit,
    text: TableText,
    text_drop: Drop,
    text_open: Hit,
    text_white: Hit,
    text_black: Hit,
    stripe: bool,
    stripe_hit: Hit,
) -> f32 {
    let mut g = PairGrid::new(x, y, w);
    g.place(|cx, cy, cw| {
        slider_row(
            px,
            fonts,
            cx,
            cy,
            cw,
            "Font size",
            cfg.font_pct(spec.id),
            70,
            160,
            "%",
            Hit::Font(spec.id),
            hover,
            hits,
        )
    });
    g.place(|cx, cy, cw| {
        slider_row(
            px,
            fonts,
            cx,
            cy,
            cw,
            spec.bg_label,
            cfg[spec.id].bg,
            0,
            100,
            "%",
            spec.bg,
            hover,
            hits,
        )
    });
    g.place(|cx, cy, cw| {
        toggle_row(
            px,
            fonts,
            cx,
            cy,
            cw,
            "Bold text",
            cfg.bold(spec.id),
            Hit::Bold(spec.id),
            hover,
            hits,
        )
    });
    g.place(|cx, cy, cw| {
        slider_row(px, fonts, cx, cy, cw, "Row highlight", hl, 0, 100, "%", hl_hit, hover, hits)
    });
    g.place(|cx, cy, cw| {
        dropdown_row(
            px,
            fonts,
            cx,
            cy,
            cw,
            "Text color",
            text.label(),
            open_drop == Some(text_drop),
            text_open,
            &[
                (text_white, "White", text == TableText::White),
                (text_black, "Black", text == TableText::Black),
            ],
            hover,
            hits,
        )
    });
    g.place(|cx, cy, cw| {
        toggle_row(px, fonts, cx, cy, cw, "Alternating rows", stripe, stripe_hit, hover, hits)
    });
    g.end()
}

fn pane_standings(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    open_drop: Option<Drop>,
    drag: Option<ColDrag>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let spec = widget_pane_spec(WidgetId::Standings);
    open_widget_pane(px, fonts, cfg, hover, hits, x, y, w, spec, |px, fonts, y, shown, hits| {
        if !shown {
            return y;
        }
        let mut y = table_style_controls(
            px,
            fonts,
            spec,
            cfg,
            hover,
            open_drop,
            hits,
            x,
            y,
            w,
            cfg.st_hl,
            Hit::StHl,
            cfg.st_text,
            Drop::StText,
            Hit::StTextOpen,
            Hit::StTextWhite,
            Hit::StTextBlack,
            cfg.st_stripe,
            Hit::StStripe,
        );
        let mut g = PairGrid::new(x, y, w);
        g.place(|cx, cy, cw| {
            stepper_row(px, fonts, cx, cy, cw, "Rows", &cfg.standings_rows.to_string(), Hit::StDec, Hit::StInc, hover, hits)
        });
        y = g.end();
        y = board_slots_section(px, fonts, x, y, w, "Header", InfoBar::StHead, cfg.st_head, open_drop, hover, hits);
        y = board_slots_section(px, fonts, x, y, w, "Footer", InfoBar::StFoot, cfg.st_foot, open_drop, hover, hits);
        y = section(px, fonts, x, y, "Columns  ·  drag to reorder · slide width · toggle to show");
        for (i, field) in cfg.st_order.iter().enumerate() {
            y = field_row(
                px,
                fonts,
                x,
                y,
                w,
                field.label(),
                field.enabled(cfg),
                field.width(cfg),
                Hit::StDrag(i as u8),
                st_toggle(*field),
                Hit::StW(i as u8),
                field.width_max(),
                i,
                hover,
                drag.filter(|d| d.kind == DragKind::St),
                hits,
            );
        }
        y
    })
}

fn pane_relative(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    open_drop: Option<Drop>,
    drag: Option<ColDrag>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let spec = widget_pane_spec(WidgetId::Relative);
    open_widget_pane(px, fonts, cfg, hover, hits, x, y, w, spec, |px, fonts, y, shown, hits| {
        if !shown {
            return y;
        }
        let mut y = table_style_controls(
            px,
            fonts,
            spec,
            cfg,
            hover,
            open_drop,
            hits,
            x,
            y,
            w,
            cfg.rel_hl,
            Hit::RelHl,
            cfg.rel_text,
            Drop::RelText,
            Hit::RelTextOpen,
            Hit::RelTextWhite,
            Hit::RelTextBlack,
            cfg.rel_stripe,
            Hit::RelStripe,
        );
        let mut g = PairGrid::new(x, y, w);
        g.place(|cx, cy, cw| {
            stepper_row(
                px,
                fonts,
                cx,
                cy,
                cw,
                "Nearby riders",
                &cfg.relative_count.to_string(),
                Hit::RelDec,
                Hit::RelInc,
                hover,
                hits,
            )
        });
        y = g.end();
        y = board_slots_section(px, fonts, x, y, w, "Header", InfoBar::RelHead, cfg.rel_head, open_drop, hover, hits);
        y = board_slots_section(px, fonts, x, y, w, "Footer", InfoBar::RelFoot, cfg.rel_foot, open_drop, hover, hits);
        y = section(px, fonts, x, y, "Columns  ·  drag to reorder · slide width · toggle to show");
        for (i, field) in cfg.rel_order.iter().enumerate() {
            y = field_row(
                px,
                fonts,
                x,
                y,
                w,
                field.label(),
                field.enabled(cfg),
                field.width(cfg),
                Hit::RelDrag(i as u8),
                rel_toggle(*field),
                Hit::RelW(i as u8),
                field.width_max(),
                i,
                hover,
                drag.filter(|d| d.kind == DragKind::Rel),
                hits,
            );
        }
        y
    })
}

fn pane_map(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    open_drop: Option<Drop>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let spec = widget_pane_spec(WidgetId::Map);
    open_widget_pane(px, fonts, cfg, hover, hits, x, y, w, spec, |px, fonts, y, shown, hits| {
        if !shown {
            return y;
        }
        let mut y = pane_style(px, fonts, spec, cfg, hover, hits, x, y, w);
        y = section(px, fonts, x, y, "On the map");
        y = toggle_row(px, fonts, x, y, w, "Other riders", cfg.map_others, Hit::MapOthers, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Start / finish", cfg.map_sf, Hit::MapSf, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Sector lines", cfg.map_sectors, Hit::MapSectors, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Track arrows", cfg.map_arrows, Hit::MapArrows, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Leader crown", cfg.map_crown, Hit::MapCrown, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Nearest ahead / behind", cfg.map_place, Hit::MapPlace, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Numbers in dots", cfg.map_numbers, Hit::MapNumbers, hover, hits);
        dropdown_row(
            px,
            fonts,
            x,
            y,
            w,
            "Dot number",
            cfg.map_dot.label(),
            open_drop == Some(Drop::MapDot),
            Hit::MapDotOpen,
            &[
                (Hit::MapDotNum, "Number", cfg.map_dot == DotLabel::Number),
                (Hit::MapDotPos, "Position", cfg.map_dot == DotLabel::Position),
            ],
            hover,
            hits,
        )
    })
}

fn pane_minimap(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    open_drop: Option<Drop>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let spec = widget_pane_spec(WidgetId::Minimap);
    open_widget_pane(px, fonts, cfg, hover, hits, x, y, w, spec, |px, fonts, y, shown, hits| {
        if !shown {
            return y;
        }
        let mut y = pane_style(px, fonts, spec, cfg, hover, hits, x, y, w);
        y = section(px, fonts, x, y, "On the minimap");
        y = toggle_row(px, fonts, x, y, w, "Other riders", cfg.mini_others, Hit::MiniOthers, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Start / finish", cfg.mini_sf, Hit::MiniSf, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Sector lines", cfg.mini_sectors, Hit::MiniSectors, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Track arrows", cfg.mini_arrows, Hit::MiniArrows, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Leader crown", cfg.mini_crown, Hit::MiniCrown, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Nearest ahead / behind", cfg.mini_place, Hit::MiniPlace, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Numbers in dots", cfg.mini_numbers, Hit::MiniNumbers, hover, hits);
        y = dropdown_row(
            px,
            fonts,
            x,
            y,
            w,
            "Dot number",
            cfg.mini_dot.label(),
            open_drop == Some(Drop::MiniDot),
            Hit::MiniDotOpen,
            &[
                (Hit::MiniDotNum, "Number", cfg.mini_dot == DotLabel::Number),
                (Hit::MiniDotPos, "Position", cfg.mini_dot == DotLabel::Position),
            ],
            hover,
            hits,
        );
        slider_row(px, fonts, x, y, w, "Zoom", cfg.mini_zoom, 0, 100, "%", Hit::MiniZoom, hover, hits)
    })
}

fn pane_radar(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let spec = widget_pane_spec(WidgetId::Radar);
    open_widget_pane(px, fonts, cfg, hover, hits, x, y, w, spec, |px, fonts, y, shown, hits| {
        if !shown {
            return y;
        }
        let mut y = pane_style(px, fonts, spec, cfg, hover, hits, x, y, w);
        y = section(px, fonts, x, y, "On the radar");
        y = toggle_row(px, fonts, x, y, w, "Side proximity", cfg.radar_sides, Hit::RadarSides, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Rear proximity", cfg.radar_rear, Hit::RadarRear, hover, hits);
        toggle_row(px, fonts, x, y, w, "Range rings", cfg.radar_rings, Hit::RadarRings, hover, hits)
    })
}

fn pane_dash(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    open_drop: Option<Drop>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let spec = widget_pane_spec(WidgetId::Dash);
    open_widget_pane(px, fonts, cfg, hover, hits, x, y, w, spec, |px, fonts, y, shown, hits| {
        if !shown {
            return y;
        }
        let mut y = pane_style(px, fonts, spec, cfg, hover, hits, x, y, w);
        y = toggle_row(px, fonts, x, y, w, "Simple dash", cfg.dash_simple, Hit::DashSimple, hover, hits);
        if !cfg.dash_simple {
            y = toggle_row(px, fonts, x, y, w, "Rev indicator", cfg.dash_rev, Hit::DashRev, hover, hits);
            y = slots_section(
                px,
                fonts,
                x,
                y,
                w,
                "Footer  ·  3 slots",
                [
                    dash_slot(0, cfg.dash_left, open_drop),
                    dash_slot(1, cfg.dash_mid, open_drop),
                    dash_slot(2, cfg.dash_right, open_drop),
                ],
                hover,
                hits,
            );
        }
        y
    })
}

fn pane_ticker(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    open_drop: Option<Drop>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let spec = widget_pane_spec(WidgetId::Ticker);
    open_widget_pane(px, fonts, cfg, hover, hits, x, y, w, spec, |px, fonts, y, shown, hits| {
        if !shown {
            return y;
        }
        let mut y = pane_style(px, fonts, spec, cfg, hover, hits, x, y, w);
        y = toggle_row(px, fonts, x, y, w, "Track name", cfg.ticker_title, Hit::TickerTitle, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Autoscroll", cfg.ticker_autoscroll, Hit::TickerAutoscroll, hover, hits);
        y = section(px, fonts, x, y, "Side info");
        y = ticker_field_row(px, fonts, x, y, w, "Left", cfg.ticker_left, 0, open_drop, hover, hits);
        y = ticker_field_row(px, fonts, x, y, w, "Right", cfg.ticker_right, 1, open_drop, hover, hits);
        stepper_row(px, fonts, x, y, w, "Riders shown", &cfg.ticker_count.to_string(), Hit::TickerDec, Hit::TickerInc, hover, hits)
    })
}

fn pane_sys(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let spec = widget_pane_spec(WidgetId::Sys);
    open_widget_pane(px, fonts, cfg, hover, hits, x, y, w, spec, |px, fonts, y, shown, hits| {
        if !shown {
            return y;
        }
        pane_style(px, fonts, spec, cfg, hover, hits, x, y, w)
    })
}

fn pane_sector(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let spec = widget_pane_spec(WidgetId::Sector);
    open_widget_pane(px, fonts, cfg, hover, hits, x, y, w, spec, |px, fonts, y, shown, hits| {
        let mut y = note_lines(
            px,
            fonts,
            x,
            y,
            w,
            "The wide cell is the sector you are in. Live sector ticks vs your best at this point; off waits until the split. Same tape as Delta Bar. Saved per track and class (250 vs 450).",
        );
        y = toggle_row(px, fonts, x, y, w, "Live sector", cfg.sector_live, Hit::SectorLive, hover, hits);
        action_btn(px, fonts, x, y, 168.0, 32.0, "Clear this track", Hit::TrackPbClear, hover, hits, false);
        y += 40.0;
        if !shown {
            return y;
        }
        pane_style(px, fonts, spec, cfg, hover, hits, x, y, w)
    })
}

fn pane_delta(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let spec = widget_pane_spec(WidgetId::Delta);
    open_widget_pane(px, fonts, cfg, hover, hits, x, y, w, spec, |px, fonts, y, shown, hits| {
        let mut y = note_lines(
            px,
            fonts,
            x,
            y,
            w,
            "Compares this lap to a lap we recorded — not the in-game ghost. Saved per track and class (250 vs 450). REC while the first lap fills if none is saved; it says to complete two full laps.",
        );
        action_btn(px, fonts, x, y, 168.0, 32.0, "Clear this track", Hit::TrackPbClear, hover, hits, false);
        y += 40.0;
        if !shown {
            return y;
        }
        pane_style(px, fonts, spec, cfg, hover, hits, x, y, w)
    })
}

fn pane_stance(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    open_drop: Option<Drop>,
    bind_listen: bool,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let spec = widget_pane_spec(WidgetId::Stance);
    open_widget_pane(px, fonts, cfg, hover, hits, x, y, w, spec, |px, fonts, y, shown, hits| {
        let mut y = note_lines(
            px,
            fonts,
            x,
            y,
            w,
            "Not connected to MX Bikes. Sit and stand only follow the pad, key, or mouse you bind here — not rider animation.",
        );
        if !shown {
            return y;
        }
        let now = if crate::render::stance_sitting() {
            "Now: SIT"
        } else {
            "Now: STAND"
        };
        text(px, fonts, now, 13.0, x + 4.0, y + 2.0, text_col(), false);
        y += 22.0;
        let bind_name = cfg.stance_bind.label();
        y = bind_row(
            px,
            fonts,
            x,
            y,
            w,
            "Sit button",
            bind_name.as_ref(),
            bind_listen,
            Hit::StanceBindOpen,
            hover,
            hits,
        );
        y = dropdown_row(
            px,
            fonts,
            x,
            y,
            w,
            "Sit mode",
            cfg.stance_mode.label(),
            open_drop == Some(Drop::StanceMode),
            Hit::StanceModeOpen,
            &[
                (Hit::StanceModePick(StanceMode::Toggle), "Toggle", cfg.stance_mode == StanceMode::Toggle),
                (Hit::StanceModePick(StanceMode::Hold), "Hold to sit", cfg.stance_mode == StanceMode::Hold),
            ],
            hover,
            hits,
        );
        y = dropdown_row(
            px,
            fonts,
            x,
            y,
            w,
            "Look",
            cfg.stance_style.label(),
            open_drop == Some(Drop::StanceStyle),
            Hit::StanceStyleOpen,
            &[
                (Hit::StanceStylePick(StanceStyle::Text), "Text", cfg.stance_style == StanceStyle::Text),
                (Hit::StanceStylePick(StanceStyle::Icon), "Icon", cfg.stance_style == StanceStyle::Icon),
            ],
            hover,
            hits,
        );
        y = toggle_row(px, fonts, x, y, w, "Show sitting", cfg.stance_show_sit, Hit::StanceShowSit, hover, hits);
        text(
            px,
            fonts,
            "Toggle counts presses. Crash or reset can desync — use Reset to standing.",
            11.0,
            x + 4.0,
            y + 2.0,
            dim(),
            false,
        );
        y += 24.0;
        action_btn(px, fonts, x, y, 168.0, 32.0, "Reset to standing", Hit::StanceReset, hover, hits, false);
        y += 40.0;
        pane_style(px, fonts, spec, cfg, hover, hits, x, y, w)
    })
}

fn pane_flag(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let spec = widget_pane_spec(WidgetId::Flag);
    open_widget_pane(px, fonts, cfg, hover, hits, x, y, w, spec, |px, fonts, y, shown, hits| {
        if !shown {
            return y;
        }
        let mut y = toggle_row(px, fonts, x, y, w, "Yellow flag", cfg.flag_yellow, Hit::FlagYellow, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Blue flag", cfg.flag_blue, Hit::FlagBlue, hover, hits);
        pane_style(px, fonts, spec, cfg, hover, hits, x, y, w)
    })
}

fn ticker_field_row(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    value: BoardField,
    slot: u8,
    open_drop: Option<Drop>,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let options: Vec<(Hit, &'static str, bool)> = BoardField::ALL
        .iter()
        .map(|&field| (Hit::TickerFootPick(slot, field), field.label(), field == value))
        .collect();
    dropdown_row(
        px,
        fonts,
        x,
        y,
        w,
        label,
        value.label(),
        open_drop == Some(Drop::TickerFoot(slot)),
        Hit::TickerFootOpen(slot),
        &options,
        hover,
        hits,
    )
}

struct SlotDrop {
    open_hit: Hit,
    value: &'static str,
    open: bool,
    options: Vec<(Hit, &'static str, bool)>,
}

fn dash_slot(slot: u8, value: DashField, open_drop: Option<Drop>) -> SlotDrop {
    SlotDrop {
        open_hit: Hit::DashFootOpen(slot),
        value: value.label(),
        open: open_drop == Some(Drop::DashFoot(slot)),
        options: DashField::ALL
            .iter()
            .map(|&field| (Hit::DashFootPick(slot, field), field.label(), field == value))
            .collect(),
    }
}

fn set_info_slot(c: &mut HudConfig, bar: InfoBar, slot: u8, field: BoardField) {
    let slots = match bar {
        InfoBar::StHead => &mut c.st_head,
        InfoBar::StFoot => &mut c.st_foot,
        InfoBar::RelHead => &mut c.rel_head,
        InfoBar::RelFoot => &mut c.rel_foot,
    };
    if let Some(dst) = slots.get_mut(slot as usize) {
        *dst = field;
    }
}

fn board_slots_section(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    title: &str,
    bar: InfoBar,
    values: [BoardField; 3],
    open_drop: Option<Drop>,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    slots_section(
        px,
        fonts,
        x,
        y,
        w,
        &format!("{title}  ·  3 slots"),
        [0, 1, 2].map(|slot| {
            let value = values[slot];
            SlotDrop {
                open_hit: Hit::InfoOpen(bar, slot as u8),
                value: value.label(),
                open: open_drop == Some(Drop::Info(bar, slot as u8)),
                options: BoardField::ALL
                    .iter()
                    .map(|&field| (Hit::InfoPick(bar, slot as u8, field), field.label(), field == value))
                    .collect(),
            }
        }),
        hover,
        hits,
    )
}

fn slots_section(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    title: &str,
    slots: [SlotDrop; 3],
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let y = section(px, fonts, x, y, title);
    let h = 70.0;
    row_card(px, x, y, w, h, false);
    let col = (w - 16.0) / 3.0;
    for (slot, (label, drop)) in ["Left", "Middle", "Right"].iter().zip(slots).enumerate() {
        let cx = x + 8.0 + slot as f32 * col;
        text(px, fonts, label, 11.0, cx + 4.0, y + 8.0, dim(), false);
        let bw = (col - 12.0).max(80.0);
        let bh = 28.0;
        let bx = cx + 4.0;
        let by = y + 30.0;
        hits.push(HitBox { id: drop.open_hit, x: bx, y: by, w: bw, h: bh });
        let hot = drop.open || hover == Some(drop.open_hit);
        outlined(px, bx, by, bw, bh, 7.0, if hot { chip_hover() } else { bg() });
        text(px, fonts, drop.value, 12.0, bx + 8.0, by + 6.0, text_col(), false);
        chevron(px, bx + bw - 14.0, by + bh * 0.5, drop.open, muted());
        if drop.open {
            let options = sorted_drop_options(&drop.options);
            let item_h = 28.0;
            let pad = 5.0;
            let content_h = pad * 2.0 + item_h * options.len() as f32;
            DROP_MENUS.with(|menus| {
                menus.borrow_mut().push(PendingDrop {
                    mx: bx,
                    my: by + bh + 6.0,
                    bw,
                    content_h,
                    open_hit: drop.open_hit,
                    options,
                });
            });
        }
    }
    y + h + ROW_GAP
}

fn heading(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    title: &str,
    sub: &str,
    show: Option<(bool, Hit)>,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let title_sz = 22.0;
    let h = 48.0;
    let skew = 8.0;
    let ink = ink();
    let mut right_w = 0.0;
    if show.is_some() {
        let label_w = measure(fonts, "Show on overlay", 13.0);
        right_w = label_w + 14.0 + 52.0 + 4.0;
    }
    let title_w = measure(fonts, title, title_sz);
    let max_plaque = (w - right_w - 12.0).max(72.0);
    let plaque_w = (title_w + 36.0).min(max_plaque);
    fill_skew(px, x, y, (plaque_w - skew).max(48.0), h, skew, accent());
    let label = ellipsize_heading(fonts, title, title_sz, plaque_w - 28.0);
    text(px, fonts, &label, title_sz, x + 16.0, y + 12.0, ink, false);
    if let Some((on, hit)) = show {
        let lx = x + w - right_w;
        text(px, fonts, "Show on overlay", 13.0, lx, y + 16.0, text_col(), false);
        hits.push(HitBox {
            id: hit,
            x: lx,
            y,
            w: right_w,
            h,
        });
        switch_lg(
            px,
            lx + measure(fonts, "Show on overlay", 13.0) + 12.0,
            y + (h - 28.0) * 0.5,
            on,
            hit,
            hover,
            hits,
        );
    }
    text(px, fonts, sub, 13.0, x, y + h + 8.0, muted(), false);
    y + h + 8.0 + 46.0
}

fn note_lines(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    msg: &str,
) -> f32 {
    let lines = wrap_fb(fonts, msg, (w - 8.0).max(40.0), 12.0);
    let mut y = y;
    for line in &lines {
        text(px, fonts, line, 12.0, x + 4.0, y + 2.0, caution(), false);
        y += 18.0;
    }
    y + 10.0
}

fn ellipsize_heading(fonts: &Fonts, s: &str, size: f32, max_w: f32) -> String {
    if measure(fonts, s, size) <= max_w {
        return s.to_string();
    }
    let mut t = s.to_string();
    while t.len() > 2 && measure(fonts, &format!("{t}…"), size) > max_w {
        t.pop();
    }
    format!("{t}…")
}

fn fit_path(fonts: &Fonts, path: &str, size: f32, max_w: f32) -> String {
    if measure(fonts, path, size) <= max_w {
        return path.to_string();
    }
    let chars: Vec<char> = path.chars().collect();
    let mut take = chars.len().saturating_sub(3);
    while take >= 8 {
        let left = take / 2;
        let right = take - left;
        let candidate = format!(
            "{}...{}",
            chars[..left].iter().collect::<String>(),
            chars[chars.len() - right..].iter().collect::<String>()
        );
        if measure(fonts, &candidate, size) <= max_w {
            return candidate;
        }
        take -= 1;
    }
    "...".into()
}

fn section(px: &mut Pixmap, fonts: &Fonts, x: f32, y: f32, label: &str) -> f32 {
    text(px, fonts, &label.to_ascii_uppercase(), 10.0, x + 2.0, y + 10.0, dim(), false);
    y + 28.0
}

fn row_card(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, hot: bool) {
    let fill = if hot { chip_hover() } else { panel() };
    fill_round(px, x, y, w, h, 10.0, fill);
}

fn style_controls(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    id: WidgetId,
    cfg: &HudConfig,
    bg_label: &str,
    bg: i32,
    bg_hit: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let mut g = PairGrid::new(x, y, w);
    g.place(|cx, cy, cw| {
        slider_row(px, fonts, cx, cy, cw, "Font size", cfg.font_pct(id), 70, 160, "%", Hit::Font(id), hover, hits)
    });
    g.place(|cx, cy, cw| {
        slider_row(px, fonts, cx, cy, cw, bg_label, bg, 0, 100, "%", bg_hit, hover, hits)
    });
    g.place(|cx, cy, cw| {
        toggle_row(px, fonts, cx, cy, cw, "Bold text", cfg.bold(id), Hit::Bold(id), hover, hits)
    });
    g.end()
}

fn look_section(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    id: WidgetId,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let mut y = section(px, fonts, x, y, "Position on screen");
    let snap_h = 268.0;
    row_card(px, x, y, w, snap_h, false);
    text(px, fonts, "Snap to the monitor this widget is on. Size stays the same.", 12.0, x + 16.0, y + 14.0, muted(), false);
    y += 40.0;
    let cell = 44.0;
    let gap = 6.0;
    let grid = cell * 3.0 + gap * 2.0;
    let gx = x + 16.0;
    let aligns = [
        [SnapAlign::TopLeft, SnapAlign::Top, SnapAlign::TopRight],
        [SnapAlign::Left, SnapAlign::Center, SnapAlign::Right],
        [SnapAlign::BottomLeft, SnapAlign::Bottom, SnapAlign::BottomRight],
    ];
    for (row, line) in aligns.iter().enumerate() {
        for (col, align) in line.iter().enumerate() {
            let bx = gx + col as f32 * (cell + gap);
            let by = y + row as f32 * (cell + gap);
            snap_cell(px, bx, by, cell, *align, Hit::Snap(id, *align), hover, hits);
        }
    }
    let bar_y = y + grid + 8.0;
    let bar_w = (grid - gap) * 0.5;
    snap_axis(px, gx, bar_y, bar_w, cell, true, Hit::Snap(id, SnapAlign::HCenter), hover, hits);
    snap_axis(
        px,
        gx + bar_w + gap,
        bar_y,
        bar_w,
        cell,
        false,
        Hit::Snap(id, SnapAlign::VCenter),
        hover,
        hits,
    );
    y + grid + cell + 28.0
}

fn snap_cell(
    px: &mut Pixmap,
    x: f32,
    y: f32,
    s: f32,
    align: SnapAlign,
    hit: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) {
    hits.push(HitBox { id: hit, x, y, w: s, h: s });
    let fill = if hover == Some(hit) { chip_hover() } else { panel() };
    outlined(px, x, y, s, s, 8.0, fill);
    let pad = 7.0;
    let (dx, dy) = match align {
        SnapAlign::TopLeft => (pad, pad),
        SnapAlign::Top => (s * 0.5, pad),
        SnapAlign::TopRight => (s - pad, pad),
        SnapAlign::Left => (pad, s * 0.5),
        SnapAlign::Center => (s * 0.5, s * 0.5),
        SnapAlign::Right => (s - pad, s * 0.5),
        SnapAlign::BottomLeft => (pad, s - pad),
        SnapAlign::Bottom => (s * 0.5, s - pad),
        SnapAlign::BottomRight => (s - pad, s - pad),
        _ => (s * 0.5, s * 0.5),
    };
    fill_circle(px, x + dx, y + dy, 3.2, text_col());
}

fn snap_axis(
    px: &mut Pixmap,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    horizontal: bool,
    hit: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) {
    hits.push(HitBox { id: hit, x, y, w, h });
    let fill = if hover == Some(hit) { chip_hover() } else { panel() };
    outlined(px, x, y, w, h, 8.0, fill);
    if horizontal {
        fill_round(px, x + w * 0.22, y + h * 0.5 - 3.0, w * 0.56, 6.0, 3.0, text_col());
    } else {
        fill_round(px, x + w * 0.5 - 3.0, y + h * 0.18, 6.0, h * 0.64, 3.0, text_col());
    }
}

fn st_toggle(f: StField) -> Hit {
    match f {
        StField::Pos => Hit::StPos,
        StField::Num => Hit::StNum,
        StField::Name => Hit::StName,
        StField::Gap => Hit::StGap,
        StField::Laps => Hit::StLaps,
        StField::Current => Hit::StCurrent,
        StField::Best => Hit::StBest,
        StField::Last => Hit::StLast,
        StField::Status => Hit::StStatus,
        StField::Bike => Hit::StBike,
        StField::Penalty => Hit::StPenalty,
        StField::Crashed => Hit::StCrashed,
        StField::Interval => Hit::StInterval,
        StField::Friend => Hit::HighlightFriends,
    }
}

fn rel_toggle(f: RelField) -> Hit {
    match f {
        RelField::Num => Hit::RelNum,
        RelField::Name => Hit::RelName,
        RelField::Gap => Hit::RelGap,
        RelField::Laps => Hit::RelLaps,
        RelField::Current => Hit::RelCurrent,
        RelField::Pos => Hit::RelPos,
        RelField::Bike => Hit::RelBike,
        RelField::Penalty => Hit::RelPenalty,
        RelField::Interval => Hit::RelInterval,
        RelField::Crashed => Hit::RelCrashed,
        RelField::Best => Hit::RelBest,
        RelField::Last => Hit::RelLast,
        RelField::Friend => Hit::HighlightFriends,
    }
}

fn field_row(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    on: bool,
    width: i32,
    drag: Hit,
    toggle: Hit,
    wslide: Hit,
    wmax: i32,
    i: usize,
    hover: Option<Hit>,
    col_drag: Option<ColDrag>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let h = ROW_H;
    let cluster = 188.0;
    let grabbed = col_drag.is_some_and(|d| d.from as usize == i);
    let drop = col_drag.is_some_and(|d| d.over as usize == i && d.from as usize != i);
    let hot = hover == Some(drag) || hover == Some(toggle) || hover == Some(wslide);
    if grabbed {
        fill_round(px, x, y, w, h, 10.0, tab_on());
        fill_round(px, x + 4.0, y + 12.0, 3.0, h - 24.0, 1.5, accent());
    } else if drop {
        fill_round(px, x, y, w, h, 10.0, accent_dim());
    } else {
        row_card(px, x, y, w, h, hot);
    }
    hits.push(HitBox {
        id: drag,
        x,
        y,
        w: (w - cluster).max(48.0),
        h,
    });
    draw_grip(px, x + 12.0, y + h * 0.5);
    let label_c = if on { text_col() } else { muted() };
    text(px, fonts, label, 13.0, x + 38.0, y + 16.0, label_c, false);
    let switch_x = x + w - 54.0;
    let slider_w = 88.0;
    let slider_x = switch_x - 10.0 - slider_w;
    text(px, fonts, &width.to_string(), 12.0, slider_x - 18.0, y + 16.0, muted(), true);
    draw_slider(px, slider_x, y + 16.0, slider_w, 16.0, width, COL_W_MIN, wmax, wslide, hover, hits);
    switch(px, switch_x, y + 14.0, on, toggle, hover, hits);
    if drop {
        let from = col_drag.map(|d| d.from as usize).unwrap_or(i);
        let ly = if from > i { y } else { y + h - 2.0 };
        if let Some(r) = Rect::from_xywh(x, ly, w, 2.0) {
            fill_rect(px, r, accent());
        }
    }
    y + h + ROW_GAP
}

fn draw_grip(px: &mut Pixmap, x: f32, cy: f32) {
    let c = Color::from_rgba8(108, 108, 116, 255);
    for row in 0..3 {
        for col in 0..2 {
            fill_circle(px, x + col as f32 * 6.0, cy - 6.0 + row as f32 * 6.0, 1.6, c);
        }
    }
}

fn fill_skew(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, skew: f32, c: Color) {
    let mut pb = PathBuilder::new();
    pb.move_to(x + skew, y);
    pb.line_to(x + w + skew, y);
    pb.line_to(x + w, y + h);
    pb.line_to(x, y + h);
    pb.close();
    let Some(path) = pb.finish() else {
        return;
    };
    let mut p = Paint::default();
    p.set_color(c);
    p.anti_alias = true;
    px.fill_path(&path, &p, FillRule::Winding, Transform::identity(), None);
}

fn toggle_row(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    on: bool,
    hit: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let h = ROW_H;
    hits.push(HitBox { id: hit, x, y, w, h });
    row_card(px, x, y, w, h, hover == Some(hit));
    text(px, fonts, label, 13.0, x + 16.0, y + 16.0, text_col(), false);
    switch(px, x + w - 52.0, y + 14.0, on, hit, hover, hits);
    y + h + ROW_GAP
}

fn slider_row(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    value: i32,
    min: i32,
    max: i32,
    suffix: &str,
    hit: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let h = ROW_H;
    hits.push(HitBox { id: hit, x, y, w, h });
    row_card(px, x, y, w, h, hover == Some(hit));
    text(px, fonts, label, 13.0, x + 16.0, y + 16.0, text_col(), false);
    let val_w = 44.0;
    let label_end = x + 16.0 + measure(fonts, label, 13.0) + 12.0;
    let max_end = x + w - 14.0;
    let track_w = 148.0_f32.min((max_end - val_w - label_end).max(40.0));
    let track_x = max_end - val_w - track_w;
    draw_slider(px, track_x, y + 16.0, track_w, 16.0, value, min, max, hit, hover, hits);
    text(px, fonts, &format!("{value}{suffix}"), 12.0, track_x + track_w + 8.0, y + 16.0, muted(), false);
    y + h + ROW_GAP
}

fn draw_slider(
    px: &mut Pixmap,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    value: i32,
    min: i32,
    max: i32,
    hit: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) {
    hits.push(HitBox { id: hit, x, y, w, h });
    let span = (max - min).max(1) as f32;
    let t = ((value - min) as f32 / span).clamp(0.0, 1.0);
    let cy = y + h * 0.5;
    let track_h = 4.0;
    fill_round(px, x, cy - track_h * 0.5, w, track_h, 2.0, track_off());
    let fill_w = (w * t).max(4.0);
    fill_round(px, x, cy - track_h * 0.5, fill_w, track_h, 2.0, accent());
    let kx = x + w * t;
    let kr = if hover == Some(hit) { 7.0 } else { 6.0 };
    fill_circle(px, kx, cy + 1.0, kr + 1.0, Color::from_rgba8(0, 0, 0, 70));
    fill_circle(px, kx, cy, kr, knob());
}

fn stepper_row(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    value: &str,
    dec: Hit,
    inc: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let h = ROW_H;
    row_card(px, x, y, w, h, hover == Some(dec) || hover == Some(inc));
    text(px, fonts, label, 13.0, x + 16.0, y + 16.0, text_col(), false);
    let bw = 36.0;
    let bh = 36.0;
    let by = y + 6.0;
    let ix = x + w - bw - 14.0;
    let dx = ix - 86.0 - bw;
    btn_icon(px, fonts, dx, by, bw, bh, '\u{f068}', dec, hover, hits);
    text(px, fonts, value, 13.0, dx + bw + 43.0, y + 16.0, text_col(), true);
    btn_icon(px, fonts, ix, by, bw, bh, '\u{f067}', inc, hover, hits);
    y + h + ROW_GAP
}

fn bind_row(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    value: &str,
    listen: bool,
    hit: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    if listen {
        let h = 64.0;
        hits.push(HitBox { id: hit, x, y, w, h });
        fill_round(px, x, y, w, h, 10.0, accent());
        let max_w = (w - 32.0).max(40.0);
        let title = ellipsize_heading(fonts, "Press a button now", 16.0, max_w);
        let sub = ellipsize_heading(fonts, "Pad, key, or mouse  ·  Esc cancels", 12.0, max_w);
        text(px, fonts, &title, 16.0, x + 16.0, y + 12.0, ink(), false);
        text(px, fonts, &sub, 12.0, x + 16.0, y + 36.0, ink(), false);
        return y + h + ROW_GAP;
    }
    let h = ROW_H;
    hits.push(HitBox { id: hit, x, y, w, h });
    row_card(px, x, y, w, h, hover == Some(hit));
    text(px, fonts, label, 13.0, x + 16.0, y + 16.0, text_col(), false);
    let label_w = measure(fonts, label, 13.0);
    let bw = (w - 30.0 - label_w - 16.0).clamp(108.0, 176.0);
    let bh = 28.0;
    let bx = x + w - bw - 14.0;
    let by = y + 10.0;
    hits.push(HitBox { id: hit, x: bx, y: by, w: bw, h: bh });
    let hot = hover == Some(hit);
    outlined(px, bx, by, bw, bh, 7.0, if hot { chip_hover() } else { bg() });
    text(px, fonts, value, 12.0, bx + 10.0, by + 6.0, text_col(), false);
    y + h + ROW_GAP
}

fn dropdown_row(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    value: &str,
    open: bool,
    open_hit: Hit,
    options: &[(Hit, &'static str, bool)],
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let h = ROW_H;
    hits.push(HitBox { id: open_hit, x, y, w, h });
    row_card(px, x, y, w, h, open || hover == Some(open_hit));
    text(px, fonts, label, 13.0, x + 16.0, y + 16.0, text_col(), false);
    let label_w = measure(fonts, label, 13.0);
    let bw = (w - 30.0 - label_w - 16.0).clamp(88.0, 160.0);
    let bh = 28.0;
    let bx = x + w - bw - 14.0;
    let by = y + 10.0;
    hits.push(HitBox { id: open_hit, x: bx, y: by, w: bw, h: bh });
    let hot = open || hover == Some(open_hit);
    outlined(px, bx, by, bw, bh, 7.0, if hot { chip_hover() } else { bg() });
    text(px, fonts, value, 12.0, bx + 10.0, by + 6.0, text_col(), false);
    chevron(px, bx + bw - 14.0, by + bh * 0.5, open, muted());
    if open {
        let options = sorted_drop_options(options);
        let item_h = 28.0;
        let pad = 5.0;
        let content_h = pad * 2.0 + item_h * options.len() as f32;
        DROP_MENUS.with(|menus| {
            menus.borrow_mut().push(PendingDrop {
                mx: bx,
                my: by + bh + 6.0,
                bw,
                content_h,
                open_hit,
                options,
            });
        });
    }
    y + h + ROW_GAP
}

fn sorted_drop_options(options: &[(Hit, &'static str, bool)]) -> Vec<(Hit, &'static str, bool)> {
    let mut options = options.to_vec();
    options.sort_by(|a, b| {
        match (a.1.eq_ignore_ascii_case("none"), b.1.eq_ignore_ascii_case("none")) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.1.to_ascii_lowercase().cmp(&b.1.to_ascii_lowercase()),
        }
    });
    options
}

fn paint_drop_menus(px: &mut Pixmap, fonts: &Fonts, hover: Option<Hit>, hits: &mut Vec<HitBox>) {
    let menus = DROP_MENUS.with(|menus| std::mem::take(&mut *menus.borrow_mut()));
    let (mut drop_scroll, mut drop_menu) = {
        let ui = UI.lock().unwrap();
        let ui = ui.as_ref();
        (
            ui.map(|u| u.drop_scroll).unwrap_or(0.0),
            None::<(f32, f32, f32, f32, f32)>,
        )
    };
    let item_h = 28.0;
    let pad = 5.0;
    let max_visible = 9.0;
    let win_h = px.height() as f32;

    for menu in menus {
        let mx = menu.mx;
        let my = menu.my;
        let bw = menu.bw;
        let content_h = menu.content_h;
        let space_below = (win_h - my - 10.0).max(item_h + pad * 2.0);
        let view_h = content_h
            .min(pad * 2.0 + item_h * max_visible)
            .min(space_below);
        let max_scroll = (content_h - view_h).max(0.0);
        drop_scroll = drop_scroll.clamp(0.0, max_scroll);
        drop_menu = Some((mx, my, bw, view_h, content_h));

        hits.push(HitBox { id: menu.open_hit, x: mx, y: my, w: bw, h: view_h });
        fill_round(px, mx - 1.0, my - 1.0, bw + 2.0, view_h + 2.0, 10.0, Color::from_rgba8(0, 0, 0, 90));
        outlined(px, mx, my, bw, view_h, 9.0, Color::from_rgba8(24, 24, 28, 255));

        let view_top = my + pad;
        let view_bot = my + view_h - pad;
        for (i, (hit, name, selected)) in menu.options.iter().enumerate() {
            let iy = my + pad + i as f32 * item_h - drop_scroll;
            if iy + item_h <= view_top || iy >= view_bot {
                continue;
            }
            let hit_y = iy.max(my);
            let hit_b = (iy + item_h).min(my + view_h);
            if hit_b > hit_y {
                hits.push(HitBox {
                    id: *hit,
                    x: mx,
                    y: hit_y,
                    w: bw,
                    h: hit_b - hit_y,
                });
            }
            if hover == Some(*hit) {
                fill_round(px, mx + 5.0, iy, bw - 10.0, item_h, 6.0, chip_hover());
            } else if *selected {
                fill_round(px, mx + 5.0, iy, bw - 10.0, item_h, 6.0, accent_dim());
            }
            let col = if *selected { accent() } else { text_col() };
            text(px, fonts, name, 12.0, mx + 12.0, iy + 6.0, col, false);
        }

        // Cover padding so partially scrolled rows don't spill into the chrome.
        let menu_fill = Color::from_rgba8(24, 24, 28, 255);
        if let Some(r) = Rect::from_xywh(mx + 2.0, my + 2.0, bw - 4.0, (pad - 1.0).max(1.0)) {
            fill_rect(px, r, menu_fill);
        }
        if let Some(r) = Rect::from_xywh(
            mx + 2.0,
            my + view_h - pad,
            bw - 4.0,
            (pad - 1.0).max(1.0),
        ) {
            fill_rect(px, r, menu_fill);
        }

        if max_scroll > 0.5 {
            let track_h = (view_h - 12.0).max(8.0);
            let thumb_h = (view_h / content_h * track_h).clamp(12.0, track_h);
            let thumb_y = my + 6.0 + (drop_scroll / max_scroll) * (track_h - thumb_h);
            fill_round(
                px,
                mx + bw - 7.0,
                my + 6.0,
                3.0,
                track_h,
                1.5,
                Color::from_rgba8(255, 255, 255, 18),
            );
            fill_round(
                px,
                mx + bw - 7.0,
                thumb_y,
                3.0,
                thumb_h,
                1.5,
                Color::from_rgba8(255, 255, 255, 70),
            );
        }
    }

    if let Some(ui) = UI.lock().unwrap().as_mut() {
        ui.drop_scroll = drop_scroll;
        ui.drop_menu = drop_menu;
    }
}

fn chevron(px: &mut Pixmap, cx: f32, cy: f32, open: bool, c: Color) {
    let mut pb = PathBuilder::new();
    if open {
        pb.move_to(cx - 4.5, cy + 2.0);
        pb.line_to(cx + 4.5, cy + 2.0);
        pb.line_to(cx, cy - 3.0);
    } else {
        pb.move_to(cx - 4.5, cy - 2.0);
        pb.line_to(cx + 4.5, cy - 2.0);
        pb.line_to(cx, cy + 3.0);
    }
    pb.close();
    let Some(path) = pb.finish() else {
        return;
    };
    let mut p = Paint::default();
    p.set_color(c);
    p.anti_alias = true;
    px.fill_path(&path, &p, FillRule::Winding, Transform::identity(), None);
}

fn switch_lg(px: &mut Pixmap, x: f32, y: f32, on: bool, hit: Hit, hover: Option<Hit>, hits: &mut Vec<HitBox>) {
    let w = 52.0;
    let h = 28.0;
    hits.push(HitBox { id: hit, x, y, w, h });
    let mut track = if on { accent() } else { track_off() };
    if hover == Some(hit) && !on {
        track = Color::from_rgba8(58, 58, 66, 255);
    }
    fill_round(px, x, y, w, h, 14.0, track);
    let kx = if on { x + w - 14.0 } else { x + 14.0 };
    fill_circle(px, kx, y + h * 0.5 + 0.8, 9.0, Color::from_rgba8(0, 0, 0, 50));
    fill_circle(px, kx, y + h * 0.5, 8.5, knob());
}

fn switch(px: &mut Pixmap, x: f32, y: f32, on: bool, hit: Hit, hover: Option<Hit>, hits: &mut Vec<HitBox>) {
    let w = 38.0;
    let h = 20.0;
    hits.push(HitBox { id: hit, x, y, w, h });
    let mut track = if on { accent() } else { track_off() };
    if hover == Some(hit) && !on {
        track = Color::from_rgba8(58, 58, 66, 255);
    }
    fill_round(px, x, y, w, h, 10.0, track);
    let kx = if on { x + w - 10.0 } else { x + 10.0 };
    fill_circle(px, kx, y + h * 0.5 + 0.8, 7.2, Color::from_rgba8(0, 0, 0, 50));
    fill_circle(px, kx, y + h * 0.5, 7.0, knob());
}

fn btn_icon(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    ch: char,
    hit: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) {
    hits.push(HitBox { id: hit, x, y, w, h });
    let fill = if hover == Some(hit) { chip_hover() } else { panel() };
    outlined(px, x, y, w, h, 7.0, fill);
    icon(px, fonts, ch, 13.0, x + w * 0.5, y + 5.5, text_col(), true);
}

fn outlined(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, r: f32, fill: Color) {
    fill_round(px, x, y, w, h, r, btn_border());
    fill_round(px, x + 1.0, y + 1.0, (w - 2.0).max(1.0), (h - 2.0).max(1.0), (r - 1.0).max(0.0), fill);
}

fn fill_round(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, r: f32, c: Color) {
    let Some(path) = round_path(x, y, w, h, r) else {
        return;
    };
    let mut p = Paint::default();
    p.set_color(c);
    p.anti_alias = true;
    px.fill_path(&path, &p, FillRule::Winding, Transform::identity(), None);
}

fn fill_circle(px: &mut Pixmap, cx: f32, cy: f32, r: f32, c: Color) {
    let mut pb = PathBuilder::new();
    pb.push_circle(cx, cy, r);
    let Some(path) = pb.finish() else {
        return;
    };
    let mut p = Paint::default();
    p.set_color(c);
    p.anti_alias = true;
    px.fill_path(&path, &p, FillRule::Winding, Transform::identity(), None);
}

fn round_path(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<Path> {
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
    pb.finish()
}

unsafe fn present(hwnd: HWND, px: &Pixmap) {
    let hdc = GetDC(hwnd);
    if hdc.is_invalid() {
        return;
    }
    let w = px.width() as i32;
    let h = px.height() as i32;
    let rgba = px.data();
    let mut bgra = vec![0u8; rgba.len()];
    for i in (0..rgba.len()).step_by(4) {
        bgra[i] = rgba[i + 2];
        bgra[i + 1] = rgba[i + 1];
        bgra[i + 2] = rgba[i];
        bgra[i + 3] = rgba[i + 3];
    }
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0 as u32,
            ..Default::default()
        },
        ..Default::default()
    };
    let _ = SetDIBitsToDevice(
        hdc,
        0,
        0,
        w as u32,
        h as u32,
        0,
        0,
        0,
        h as u32,
        bgra.as_ptr() as *const core::ffi::c_void,
        &info,
        DIB_RGB_COLORS,
    );
    let _ = ReleaseDC(hwnd, hdc);
}

unsafe fn dark_titlebar(hwnd: HWND) {
    let on = BOOL(1);
    let _ = windows::Win32::Graphics::Dwm::DwmSetWindowAttribute(
        hwnd,
        windows::Win32::Graphics::Dwm::DWMWINDOWATTRIBUTE(20),
        &on as *const BOOL as *const core::ffi::c_void,
        std::mem::size_of::<BOOL>() as u32,
    );
}

#[cfg(test)]
#[path = "tests/settings.rs"]
mod tests;
