use std::sync::Mutex;

use tiny_skia::{Color, FillRule, Paint, Path, PathBuilder, Pixmap, PixmapPaint, Rect, Stroke, Transform};
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateSolidBrush, DeleteObject, EndPaint, FillRect, GetDC, ReleaseDC,
    SetDIBitsToDevice, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HGDIOBJ, PAINTSTRUCT,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, ReleaseCapture, SetCapture, VK_CONTROL};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, IsIconic, LoadCursorW, SetCursor, SetForegroundWindow, ShowWindow, IDC_ARROW,
    IDC_HAND, IDC_IBEAM, IDC_SIZEALL, SW_RESTORE, WM_CHAR, WM_ERASEBKGND, WM_KEYDOWN, WM_LBUTTONDOWN,
    WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_PAINT, WM_SETCURSOR,
};

use crate::config::{
    update_config, with_config, BoardField, DashField, DotLabel, FontFamily, HudConfig, RelField,
    SnapAlign, StField, Units, WidgetId,
};
use crate::render::{fill_rect, measure, text, Fonts};

fn bg() -> Color { Color::from_rgba8(24, 25, 29, 255) }
fn side() -> Color { Color::from_rgba8(8, 8, 10, 255) }
fn tab_on() -> Color { Color::from_rgba8(255, 140, 36, 28) }
fn text_col() -> Color { Color::from_rgba8(244, 244, 247, 255) }
fn muted() -> Color { Color::from_rgba8(140, 140, 148, 255) }
fn dim() -> Color { Color::from_rgba8(96, 96, 104, 255) }
fn row_line() -> Color { Color::from_rgba8(255, 255, 255, 12) }
fn chip_hover() -> Color { Color::from_rgba8(46, 47, 54, 255) }
fn accent() -> Color { Color::from_rgba8(255, 140, 36, 255) }
fn accent_dim() -> Color { Color::from_rgba8(255, 140, 36, 36) }
fn knob() -> Color { Color::from_rgba8(250, 250, 252, 255) }
fn track_off() -> Color { Color::from_rgba8(46, 46, 52, 255) }
fn btn_bg() -> Color { Color::from_rgba8(32, 32, 36, 255) }
fn btn_border() -> Color { Color::from_rgba8(255, 255, 255, 22) }
fn panel() -> Color { Color::from_rgba8(34, 35, 41, 255) }
const ROW_H: f32 = 48.0;
const ROW_GAP: f32 = 8.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    App,
    Standings,
    Relative,
    Map,
    Minimap,
    Radar,
    Dash,
    Ticker,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Hit {
    TabApp,
    TabSt,
    TabRel,
    TabMap,
    TabMini,
    TabRadar,
    TabDash,
    TabTicker,
    StShow,
    RelShow,
    MapShow,
    MiniShow,
    RadarShow,
    DashShow,
    TickerShow,
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
    MapArrows,
    MapCrown,
    MapPlace,
    MapNumbers,
    MapDotOpen,
    MapDotNum,
    MapDotPos,
    MiniOthers,
    MiniSf,
    MiniArrows,
    MiniCrown,
    MiniPlace,
    MiniNumbers,
    MiniDotOpen,
    MiniDotNum,
    MiniDotPos,
    RadarSides,
    RadarRear,
    StBg,
    RelBg,
    MapBg,
    MiniBg,
    MiniZoom,
    RadarBg,
    DashBg,
    TickerBg,
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
    FontAgency,
    FontIndustry,
    FontFaster,
    UnitsOpen,
    UnitsMetric,
    UnitsImperial,
    DashFootOpen(u8),
    DashFootPick(u8, DashField),
    TickerFootOpen(u8),
    TickerFootPick(u8, BoardField),
    InfoOpen(InfoBar, u8),
    InfoPick(InfoBar, u8, BoardField),
    UpdateCheck,
    UpdateInstall,
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
    hover: Option<Hit>,
    hits: Vec<HitBox>,
    open_drop: Option<Drop>,
    drag: Option<ColDrag>,
    slide: Option<SlideDrag>,
    scroll: f32,
    content_h: f32,
}

unsafe impl Send for SettingsUi {}

static UI: Mutex<Option<SettingsUi>> = Mutex::new(None);

const SIDE_W: f32 = 204.0;

pub fn attach(host: HWND) {
    unsafe {
        dark_titlebar(host);
    }
    *UI.lock().unwrap() = Some(SettingsUi {
        host,
        tab: Tab::Standings,
        hover: None,
        hits: Vec::new(),
        open_drop: None,
        drag: None,
        slide: None,
        scroll: 0.0,
        content_h: 0.0,
    });
}

pub fn show(host: HWND) {
    unsafe {
        let _ = ShowWindow(host, SW_RESTORE);
        let _ = SetForegroundWindow(host);
    }
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
        if IsIconic(host).as_bool() {
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
                let max = (ui.content_h - 520.0).max(0.0);
                ui.scroll = (ui.scroll - delta * 0.4).clamp(0.0, max);
            }
            true
        }
        WM_CHAR => crate::feedback::on_char(char::from_u32(wp.0 as u32).unwrap_or('\0')),
        WM_KEYDOWN => {
            let ctrl = unsafe { GetAsyncKeyState(VK_CONTROL.0 as i32) } < 0;
            crate::feedback::on_key(wp.0 as u16, ctrl)
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
                    hover == Some(Hit::FbText),
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
        Some(hit) if is_slider(hit) => start_slide(hit, p.0, host),
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
            | Hit::RelBg
            | Hit::MapBg
            | Hit::MiniBg
            | Hit::MiniZoom
            | Hit::RadarBg
            | Hit::DashBg
            | Hit::TickerBg
            | Hit::StW(_)
            | Hit::RelW(_)
            | Hit::Font(_)
    )
}

fn slide_range(hit: Hit) -> (i32, i32) {
    match hit {
        Hit::StW(_) | Hit::RelW(_) => (18, 160),
        Hit::Font(_) => (70, 160),
        _ => (0, 100),
    }
}

fn start_slide(hit: Hit, mx: f32, host: HWND) {
    close_drop();
    let box_ = {
        let ui = UI.lock().unwrap();
        ui.as_ref().and_then(|u| u.hits.iter().rev().find(|h| h.id == hit).copied())
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
        Hit::StBg => c.st_bg = v,
        Hit::RelBg => c.rel_bg = v,
        Hit::MapBg => c.map_bg = v,
        Hit::MiniBg => c.mini_bg = v,
        Hit::MiniZoom => c.mini_zoom = v,
        Hit::RadarBg => c.radar_bg = v,
        Hit::DashBg => c.dash_bg = v,
        Hit::TickerBg => c.ticker_bg = v,
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
    if !matches!(id, Hit::FbText) {
        crate::feedback::set_focus(false);
    }
    match id {
        Hit::TabApp => {
            set_tab(Tab::App);
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
        Hit::StShow => c.show_standings = !c.show_standings,
        Hit::RelShow => c.show_relative = !c.show_relative,
        Hit::MapShow => c.show_map = !c.show_map,
        Hit::MiniShow => c.show_minimap = !c.show_minimap,
        Hit::RadarShow => c.show_radar = !c.show_radar,
        Hit::DashShow => c.show_dash = !c.show_dash,
        Hit::TickerShow => c.show_ticker = !c.show_ticker,
        Hit::TickerTitle => c.ticker_title = !c.ticker_title,
        Hit::TickerAutoscroll => c.ticker_autoscroll = !c.ticker_autoscroll,
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
        Hit::MapArrows => c.map_arrows = !c.map_arrows,
        Hit::MapCrown => c.map_crown = !c.map_crown,
        Hit::MapPlace => c.map_place = !c.map_place,
        Hit::MapNumbers => c.map_numbers = !c.map_numbers,
        Hit::MapDotNum => c.map_dot = DotLabel::Number,
        Hit::MapDotPos => c.map_dot = DotLabel::Position,
        Hit::MiniOthers => c.mini_others = !c.mini_others,
        Hit::MiniSf => c.mini_sf = !c.mini_sf,
        Hit::MiniArrows => c.mini_arrows = !c.mini_arrows,
        Hit::MiniCrown => c.mini_crown = !c.mini_crown,
        Hit::MiniPlace => c.mini_place = !c.mini_place,
        Hit::MiniNumbers => c.mini_numbers = !c.mini_numbers,
        Hit::MiniDotNum => c.mini_dot = DotLabel::Number,
        Hit::MiniDotPos => c.mini_dot = DotLabel::Position,
        Hit::RadarSides => c.radar_sides = !c.radar_sides,
        Hit::RadarRear => c.radar_rear = !c.radar_rear,
        Hit::Bold(id) => {
            let on = !c.bold(id);
            c.set_bold(id, on);
        }
        Hit::Snap(id, align) => c.snap(id, align),
        Hit::FontSegoe => c.font_family = FontFamily::Segoe,
        Hit::FontArial => c.font_family = FontFamily::Arial,
        Hit::FontTahoma => c.font_family = FontFamily::Tahoma,
        Hit::FontRoboto => c.font_family = FontFamily::Roboto,
        Hit::FontAgency => c.font_family = FontFamily::Agency,
        Hit::FontIndustry => c.font_family = FontFamily::Industry,
        Hit::FontFaster => c.font_family = FontFamily::FasterOne,
        Hit::UnitsMetric => c.units = Units::Metric,
        Hit::UnitsImperial => c.units = Units::Imperial,
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
        Hit::TabApp | Hit::TabSt | Hit::TabRel | Hit::TabMap | Hit::TabMini | Hit::TabRadar | Hit::TabDash
        | Hit::TabTicker
        | Hit::MapDotOpen | Hit::MiniDotOpen | Hit::FontOpen | Hit::UnitsOpen | Hit::DashFootOpen(_)
        | Hit::TickerFootOpen(_)
        | Hit::InfoOpen(_, _)
        | Hit::UpdateCheck | Hit::UpdateInstall
        | Hit::FbRate | Hit::FbBug | Hit::FbFeature | Hit::FbStar(_) | Hit::FbText | Hit::FbAttach | Hit::FbSend
        | Hit::StDrag(_) | Hit::RelDrag(_)
        | Hit::StBg | Hit::RelBg | Hit::MapBg | Hit::MiniBg | Hit::MiniZoom | Hit::RadarBg | Hit::DashBg | Hit::TickerBg
        | Hit::StW(_) | Hit::RelW(_) | Hit::Font(_) => {}
    });
}

fn set_tab(tab: Tab) {
    crate::feedback::set_focus(false);
    if let Some(ui) = UI.lock().unwrap().as_mut() {
        ui.tab = tab;
        ui.open_drop = None;
        ui.drag = None;
        ui.slide = None;
        ui.scroll = 0.0;
    }
}

fn toggle_drop(drop: Drop) {
    if let Some(ui) = UI.lock().unwrap().as_mut() {
        ui.open_drop = if ui.open_drop == Some(drop) { None } else { Some(drop) };
    }
}

fn close_drop() {
    if let Some(ui) = UI.lock().unwrap().as_mut() {
        ui.open_drop = None;
    }
}

fn hit_at(hits: &[HitBox], x: f32, y: f32) -> Option<Hit> {
    hits.iter()
        .rev()
        .find(|h| x >= h.x && x <= h.x + h.w && y >= h.y && y <= h.y + h.h)
        .map(|h| h.id)
}

fn draw(px: &mut Pixmap, fonts: &Fonts, w: f32, h: f32) {
    px.fill(bg());
    if let Some(r) = Rect::from_xywh(0.0, 0.0, SIDE_W, h) {
        fill_rect(px, r, side());
    }
    if let Some(r) = Rect::from_xywh(SIDE_W, 0.0, 1.0, h) {
        fill_rect(px, r, Color::from_rgba8(255, 255, 255, 10));
    }

    let cfg = with_config(|c| c.clone());
    let (tab, hover, open_drop, drag, scroll) = {
        let ui = UI.lock().unwrap();
        let ui = ui.as_ref();
        (
            ui.map(|u| u.tab).unwrap_or(Tab::Standings),
            ui.and_then(|u| u.hover),
            ui.and_then(|u| u.open_drop),
            ui.and_then(|u| u.drag),
            ui.map(|u| u.scroll).unwrap_or(0.0),
        )
    };
    let mut hits = Vec::new();

    draw_brand(px, fonts);
    if let Some(r) = Rect::from_xywh(18.0, 72.0, SIDE_W - 36.0, 1.0) {
        fill_rect(px, r, row_line());
    }

    let tabs = [
        (Tab::App, Hit::TabApp, "App", true),
        (Tab::Standings, Hit::TabSt, "Standings", cfg.show_standings),
        (Tab::Relative, Hit::TabRel, "Relative", cfg.show_relative),
        (Tab::Map, Hit::TabMap, "Map", cfg.show_map),
        (Tab::Minimap, Hit::TabMini, "Minimap", cfg.show_minimap),
        (Tab::Radar, Hit::TabRadar, "Radar", cfg.show_radar),
        (Tab::Dash, Hit::TabDash, "Dash", cfg.show_dash),
        (Tab::Ticker, Hit::TabTicker, "H-Standings", cfg.show_ticker),
    ];
    let mut ty = 84.0;
    for (t, hit, name, on) in tabs {
        nav_tab(px, fonts, 12.0, ty, SIDE_W - 24.0, 36.0, t == tab, on, name, hit, hover, &mut hits);
        ty += 40.0;
    }
    fill_round(px, 12.0, h - 56.0, SIDE_W - 24.0, 40.0, 10.0, panel());
    text(px, fonts, "F8  settings", 10.0, 22.0, h - 50.0, dim(), false);
    text(px, fonts, "Ctrl + drag to move", 10.0, 22.0, h - 34.0, dim(), false);

    let x = SIDE_W + 28.0;
    let cw = (w - x - 28.0).max(200.0);
    let py = 24.0 - scroll;
    let bottom = match tab {
        Tab::App => pane_app(px, fonts, &cfg, hover, open_drop, &mut hits, x, py, cw),
        Tab::Standings => pane_standings(px, fonts, &cfg, hover, open_drop, drag, &mut hits, x, py, cw),
        Tab::Relative => pane_relative(px, fonts, &cfg, hover, open_drop, drag, &mut hits, x, py, cw),
        Tab::Map => pane_map(px, fonts, &cfg, hover, open_drop, &mut hits, x, py, cw),
        Tab::Minimap => pane_minimap(px, fonts, &cfg, hover, open_drop, &mut hits, x, py, cw),
        Tab::Radar => pane_radar(px, fonts, &cfg, hover, &mut hits, x, py, cw),
        Tab::Dash => pane_dash(px, fonts, &cfg, hover, open_drop, &mut hits, x, py, cw),
        Tab::Ticker => pane_ticker(px, fonts, &cfg, hover, open_drop, &mut hits, x, py, cw),
    };

    if let Some(ui) = UI.lock().unwrap().as_mut() {
        ui.hits = hits;
        ui.content_h = bottom + scroll;
        let max = (ui.content_h - h + 24.0).max(0.0);
        ui.scroll = ui.scroll.clamp(0.0, max);
    }
}

fn brand_logo() -> &'static Pixmap {
    static LOGO: std::sync::OnceLock<Pixmap> = std::sync::OnceLock::new();
    LOGO.get_or_init(|| Pixmap::decode_png(include_bytes!("../icon-48.png")).expect("icon-48.png"))
}

fn draw_brand(px: &mut Pixmap, fonts: &Fonts) {
    let logo = brand_logo();
    let x = 12.0;
    let y = 14.0;
    let _ = px.draw_pixmap(
        x as i32,
        y as i32,
        logo.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
    let tx = x + logo.width() as f32 + 10.0;
    text(px, fonts, "HOLESHOT", 13.0, tx, y + 8.0, text_col(), false);
    text_tracked(px, fonts, "HUD", 10.0, tx, y + 26.0, accent(), 2.6);
}

fn text_tracked(
    px: &mut Pixmap,
    fonts: &Fonts,
    s: &str,
    size: f32,
    mut x: f32,
    y: f32,
    color: Color,
    tracking: f32,
) {
    for ch in s.chars() {
        let g = ch.to_string();
        text(px, fonts, &g, size, x, y, color, false);
        x += size * 0.62 + tracking;
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
) {
    hits.push(HitBox { id: hit, x, y, w, h });
    if selected {
        fill_round(px, x, y, w, h, 8.0, tab_on());
    } else if hover == Some(hit) {
        fill_round(px, x, y, w, h, 8.0, Color::from_rgba8(255, 255, 255, 10));
    }
    let name_c = if selected { accent() } else { Color::from_rgba8(210, 210, 216, 255) };
    text(px, fonts, name, 13.0, x + 14.0, y + 10.0, name_c, false);
    let dx = x + w - 16.0;
    let dy = y + h * 0.5;
    if visible {
        fill_circle(px, dx, dy, 3.5, accent());
    } else {
        fill_circle(px, dx, dy, 3.5, track_off());
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
    heading(px, fonts, x, y, "App", "Font and units apply to every widget");
    let mut y = y + 64.0;
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
            (Hit::FontAgency, "Agency FB", cfg.font_family == FontFamily::Agency),
            (Hit::FontIndustry, "Industry", cfg.font_family == FontFamily::Industry),
            (Hit::FontFaster, "Faster One", cfg.font_family == FontFamily::FasterOne),
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
    y = section(px, fonts, x, y, "Updates");
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
    let card_h = if extra.is_some() { 168.0 } else { 148.0 };
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
    let btn_w = 156.0;
    if show_check {
        action_btn(px, fonts, x + 16.0, iy, btn_w, 32.0, "Check for updates", Hit::UpdateCheck, hover, hits, false);
    }
    if show_install {
        action_btn(
            px,
            fonts,
            x + 16.0 + btn_w + 10.0,
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
    y = section(px, fonts, x, y, "Feedback");
    y = pane_feedback(px, fonts, hover, hits, x, y, w);
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
    if on {
        fill_round(px, x, y, w, 32.0, 8.0, accent_dim());
        text(px, fonts, label, 13.0, x + w * 0.5, y + 8.0, accent(), true);
    } else {
        let fill = if hover == Some(hit) { chip_hover() } else { btn_bg() };
        outlined(px, x, y, w, 32.0, 8.0, fill);
        text(px, fonts, label, 13.0, x + w * 0.5, y + 8.0, text_col(), true);
    }
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
    for (pi, para) in s.split('\n').enumerate() {
        if pi > 0 && para.is_empty() {
            lines.push(String::new());
            continue;
        }
        if para.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut line = String::new();
        let mut line_w = 0.0;
        for ch in para.chars() {
            let ch_w = measure(fonts, ch.encode_utf8(&mut [0; 4]), size);
            if !line.is_empty() && line_w + ch_w > max_w {
                lines.push(std::mem::take(&mut line));
                line_w = 0.0;
            }
            line.push(ch);
            line_w += ch_w;
        }
        lines.push(line);
    }
    lines
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
    heading(px, fonts, x, y, "Standings", "Who is ahead and by how much");
    let mut y = y + 64.0;
    y = toggle_row(px, fonts, x, y, w, "Show on overlay", cfg.show_standings, Hit::StShow, hover, hits);
    y = board_slots_section(px, fonts, x, y, w, "Header", InfoBar::StHead, cfg.st_head, open_drop, hover, hits);
    y = board_slots_section(px, fonts, x, y, w, "Footer", InfoBar::StFoot, cfg.st_foot, open_drop, hover, hits);
    y = stepper_row(px, fonts, x, y, w, "Rows", &cfg.standings_rows.to_string(), Hit::StDec, Hit::StInc, hover, hits);
    y = section(px, fonts, x, y, "Columns  ·  drag to reorder, slide width");
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
            i,
            hover,
            drag.filter(|d| d.kind == DragKind::St),
            hits,
        );
    }
    y = slider_row(px, fonts, x, y, w, "Background", cfg.st_bg, 0, 100, "%", Hit::StBg, hover, hits);
    look_section(px, fonts, x, y, w, WidgetId::Standings, cfg, hover, hits)
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
    heading(px, fonts, x, y, "Relative", "Riders just ahead and behind you");
    let mut y = y + 64.0;
    y = toggle_row(px, fonts, x, y, w, "Show on overlay", cfg.show_relative, Hit::RelShow, hover, hits);
    y = board_slots_section(px, fonts, x, y, w, "Header", InfoBar::RelHead, cfg.rel_head, open_drop, hover, hits);
    y = board_slots_section(px, fonts, x, y, w, "Footer", InfoBar::RelFoot, cfg.rel_foot, open_drop, hover, hits);
    y = stepper_row(px, fonts, x, y, w, "Nearby riders", &cfg.relative_count.to_string(), Hit::RelDec, Hit::RelInc, hover, hits);
    y = section(px, fonts, x, y, "Columns  ·  drag to reorder, slide width");
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
            i,
            hover,
            drag.filter(|d| d.kind == DragKind::Rel),
            hits,
        );
    }
    y = slider_row(px, fonts, x, y, w, "Background", cfg.rel_bg, 0, 100, "%", Hit::RelBg, hover, hits);
    look_section(px, fonts, x, y, w, WidgetId::Relative, cfg, hover, hits)
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
    heading(px, fonts, x, y, "Map", "Where you and others are on track");
    let mut y = y + 64.0;
    y = toggle_row(px, fonts, x, y, w, "Show on overlay", cfg.show_map, Hit::MapShow, hover, hits);
    y = section(px, fonts, x, y, "On the map");
    y = toggle_row(px, fonts, x, y, w, "Other riders", cfg.map_others, Hit::MapOthers, hover, hits);
    y = toggle_row(px, fonts, x, y, w, "Start / finish", cfg.map_sf, Hit::MapSf, hover, hits);
    y = toggle_row(px, fonts, x, y, w, "Track arrows", cfg.map_arrows, Hit::MapArrows, hover, hits);
    y = toggle_row(px, fonts, x, y, w, "Leader crown", cfg.map_crown, Hit::MapCrown, hover, hits);
    y = toggle_row(px, fonts, x, y, w, "Nearest ahead / behind", cfg.map_place, Hit::MapPlace, hover, hits);
    y = toggle_row(px, fonts, x, y, w, "Numbers in dots", cfg.map_numbers, Hit::MapNumbers, hover, hits);
    y = dropdown_row(
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
    );
    y = slider_row(px, fonts, x, y, w, "Background", cfg.map_bg, 0, 100, "%", Hit::MapBg, hover, hits);
    look_section(px, fonts, x, y, w, WidgetId::Map, cfg, hover, hits)
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
    heading(px, fonts, x, y, "Minimap", "Circular track with numbered riders");
    let mut y = y + 64.0;
    y = toggle_row(px, fonts, x, y, w, "Show on overlay", cfg.show_minimap, Hit::MiniShow, hover, hits);
    y = section(px, fonts, x, y, "On the minimap");
    y = toggle_row(px, fonts, x, y, w, "Other riders", cfg.mini_others, Hit::MiniOthers, hover, hits);
    y = toggle_row(px, fonts, x, y, w, "Start / finish", cfg.mini_sf, Hit::MiniSf, hover, hits);
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
    y = slider_row(px, fonts, x, y, w, "Zoom", cfg.mini_zoom, 0, 100, "%", Hit::MiniZoom, hover, hits);
    y = slider_row(px, fonts, x, y, w, "Background", cfg.mini_bg, 0, 100, "%", Hit::MiniBg, hover, hits);
    look_section(px, fonts, x, y, w, WidgetId::Minimap, cfg, hover, hits)
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
    heading(px, fonts, x, y, "Radar", "Riders beside and behind you");
    let mut y = y + 64.0;
    y = toggle_row(px, fonts, x, y, w, "Show on overlay", cfg.show_radar, Hit::RadarShow, hover, hits);
    y = section(px, fonts, x, y, "On the radar");
    y = toggle_row(px, fonts, x, y, w, "Side proximity", cfg.radar_sides, Hit::RadarSides, hover, hits);
    y = toggle_row(px, fonts, x, y, w, "Rear proximity", cfg.radar_rear, Hit::RadarRear, hover, hits);
    y = slider_row(px, fonts, x, y, w, "Panel opacity", cfg.radar_bg, 0, 100, "%", Hit::RadarBg, hover, hits);
    look_section(px, fonts, x, y, w, WidgetId::Radar, cfg, hover, hits)
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
    heading(px, fonts, x, y, "Dash", "Gear, speed, and footer stats");
    let mut y = y + 64.0;
    y = toggle_row(px, fonts, x, y, w, "Show on overlay", cfg.show_dash, Hit::DashShow, hover, hits);
    y = section(px, fonts, x, y, "Footer");
    y = dash_field_row(px, fonts, x, y, w, "Left", cfg.dash_left, 0, open_drop, hover, hits);
    y = dash_field_row(px, fonts, x, y, w, "Middle", cfg.dash_mid, 1, open_drop, hover, hits);
    y = dash_field_row(px, fonts, x, y, w, "Right", cfg.dash_right, 2, open_drop, hover, hits);
    y = slider_row(px, fonts, x, y, w, "Panel opacity", cfg.dash_bg, 0, 100, "%", Hit::DashBg, hover, hits);
    look_section(px, fonts, x, y, w, WidgetId::Dash, cfg, hover, hits)
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
    heading(px, fonts, x, y, "Horizontal Standings", "Your name is highlighted in the field");
    let mut y = y + 64.0;
    y = toggle_row(px, fonts, x, y, w, "Show on overlay", cfg.show_ticker, Hit::TickerShow, hover, hits);
    y = toggle_row(px, fonts, x, y, w, "Track name", cfg.ticker_title, Hit::TickerTitle, hover, hits);
    y = toggle_row(px, fonts, x, y, w, "Autoscroll", cfg.ticker_autoscroll, Hit::TickerAutoscroll, hover, hits);
    y = section(px, fonts, x, y, "Side info");
    y = ticker_field_row(px, fonts, x, y, w, "Left", cfg.ticker_left, 0, open_drop, hover, hits);
    y = ticker_field_row(px, fonts, x, y, w, "Right", cfg.ticker_right, 1, open_drop, hover, hits);
    y = stepper_row(px, fonts, x, y, w, "Riders shown", &cfg.ticker_count.to_string(), Hit::TickerDec, Hit::TickerInc, hover, hits);
    y = slider_row(px, fonts, x, y, w, "Panel opacity", cfg.ticker_bg, 0, 100, "%", Hit::TickerBg, hover, hits);
    look_section(px, fonts, x, y, w, WidgetId::Ticker, cfg, hover, hits)
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

fn dash_field_row(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    value: DashField,
    slot: u8,
    open_drop: Option<Drop>,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let options: Vec<(Hit, &'static str, bool)> = DashField::ALL
        .iter()
        .map(|&field| (Hit::DashFootPick(slot, field), field.label(), field == value))
        .collect();
    dropdown_row(
        px,
        fonts,
        x,
        y,
        w,
        label,
        value.label(),
        open_drop == Some(Drop::DashFoot(slot)),
        Hit::DashFootOpen(slot),
        &options,
        hover,
        hits,
    )
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
    let mut y = section(px, fonts, x, y, title);
    for (slot, (label, value)) in ["Left", "Middle", "Right"].iter().zip(values).enumerate() {
        y = board_field_row(px, fonts, x, y, w, label, value, bar, slot as u8, open_drop, hover, hits);
    }
    y
}

fn board_field_row(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    value: BoardField,
    bar: InfoBar,
    slot: u8,
    open_drop: Option<Drop>,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let options: Vec<(Hit, &'static str, bool)> = BoardField::ALL
        .iter()
        .map(|&field| (Hit::InfoPick(bar, slot, field), field.label(), field == value))
        .collect();
    dropdown_row(
        px,
        fonts,
        x,
        y,
        w,
        label,
        value.label(),
        open_drop == Some(Drop::Info(bar, slot)),
        Hit::InfoOpen(bar, slot),
        &options,
        hover,
        hits,
    )
}

fn heading(px: &mut Pixmap, fonts: &Fonts, x: f32, y: f32, title: &str, sub: &str) {
    text(px, fonts, title, 26.0, x, y, text_col(), false);
    text(px, fonts, sub, 13.0, x, y + 32.0, muted(), false);
}

fn section(px: &mut Pixmap, fonts: &Fonts, x: f32, y: f32, label: &str) -> f32 {
    text(px, fonts, &label.to_ascii_uppercase(), 10.0, x + 2.0, y + 10.0, dim(), false);
    y + 28.0
}

fn row_card(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, hot: bool) {
    let fill = if hot { chip_hover() } else { panel() };
    fill_round(px, x, y, w, h, 10.0, fill);
}

fn look_section(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    id: WidgetId,
    cfg: &HudConfig,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let mut y = slider_row(
        px,
        fonts,
        x,
        y,
        w,
        "Font size",
        cfg.font_pct(id),
        70,
        160,
        "%",
        Hit::Font(id),
        hover,
        hits,
    );
    y = toggle_row(px, fonts, x, y, w, "Bold text", cfg.bold(id), Hit::Bold(id), hover, hits);
    y = section(px, fonts, x, y, "Position on screen");
    let snap_h = 224.0;
    row_card(px, x, y, w, snap_h, false);
    text(px, fonts, "Snap to the monitor this widget is on. Size stays the same.", 12.0, x + 16.0, y + 14.0, muted(), false);
    y += 40.0;
    let cell = 36.0;
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
    text(px, fonts, label, 13.0, x + 38.0, y + 16.0, text_col(), false);
    let switch_x = x + w - 54.0;
    let slider_w = 88.0;
    let slider_x = switch_x - 10.0 - slider_w;
    text(px, fonts, &width.to_string(), 12.0, slider_x - 18.0, y + 16.0, muted(), true);
    draw_slider(px, slider_x, y + 16.0, slider_w, 16.0, width, 18, 160, wslide, hover, hits);
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
    switch(px, x + w - 54.0, y + 14.0, on, hit, hover, hits);
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
    row_card(px, x, y, w, h, hover == Some(hit));
    text(px, fonts, label, 13.0, x + 16.0, y + 16.0, text_col(), false);
    let val_w = 44.0;
    let track_w = 148.0;
    let track_x = x + w - val_w - track_w - 16.0;
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
    let bw = 28.0;
    let bh = 26.0;
    let by = y + 11.0;
    let ix = x + w - bw - 14.0;
    let dx = ix - 86.0 - bw;
    btn(px, fonts, dx, by, bw, bh, "−", dec, hover, hits);
    text(px, fonts, value, 13.0, dx + bw + 43.0, y + 16.0, text_col(), true);
    btn(px, fonts, ix, by, bw, bh, "+", inc, hover, hits);
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
    row_card(px, x, y, w, h, open || hover == Some(open_hit));
    text(px, fonts, label, 13.0, x + 16.0, y + 16.0, text_col(), false);
    let bw = 160.0;
    let bh = 28.0;
    let bx = x + w - bw - 14.0;
    let by = y + 10.0;
    hits.push(HitBox { id: open_hit, x: bx, y: by, w: bw, h: bh });
    let hot = open || hover == Some(open_hit);
    outlined(px, bx, by, bw, bh, 7.0, if hot { chip_hover() } else { bg() });
    text(px, fonts, value, 12.0, bx + 10.0, by + 6.0, text_col(), false);
    chevron(px, bx + bw - 14.0, by + bh * 0.5, open, muted());
    if !open {
        return y + h + ROW_GAP;
    }
    let item_h = 28.0;
    let pad = 5.0;
    let mh = pad * 2.0 + item_h * options.len() as f32;
    let mx = bx;
    let my = by + bh + 6.0;
    fill_round(px, mx - 1.0, my - 1.0, bw + 2.0, mh + 2.0, 10.0, Color::from_rgba8(0, 0, 0, 90));
    outlined(px, mx, my, bw, mh, 9.0, Color::from_rgba8(24, 24, 28, 255));
    for (i, (hit, name, selected)) in options.iter().enumerate() {
        let iy = my + pad + i as f32 * item_h;
        hits.push(HitBox { id: *hit, x: mx, y: iy, w: bw, h: item_h });
        if hover == Some(*hit) {
            fill_round(px, mx + 5.0, iy, bw - 10.0, item_h, 6.0, chip_hover());
        } else if *selected {
            fill_round(px, mx + 5.0, iy, bw - 10.0, item_h, 6.0, accent_dim());
        }
        let col = if *selected { accent() } else { text_col() };
        text(px, fonts, name, 12.0, mx + 12.0, iy + 6.0, col, false);
    }
    y + h + ROW_GAP + mh + 8.0
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

fn btn(
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
) {
    hits.push(HitBox { id: hit, x, y, w, h });
    let fill = if hover == Some(hit) { chip_hover() } else { panel() };
    outlined(px, x, y, w, h, 7.0, fill);
    text(px, fonts, label, 15.0, x + w * 0.5, y + 4.0, text_col(), true);
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
