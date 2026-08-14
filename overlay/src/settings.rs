use std::sync::Mutex;

use tiny_skia::{Color, FillRule, Paint, Path, PathBuilder, Pixmap, Rect, Transform};
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateSolidBrush, DeleteObject, EndPaint, FillRect, GetDC, ReleaseDC,
    SetDIBitsToDevice, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HGDIOBJ, PAINTSTRUCT,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, IsIconic, LoadCursorW, SetCursor, SetForegroundWindow, ShowWindow, IDC_ARROW,
    IDC_HAND, IDC_SIZEALL, SW_RESTORE, WM_ERASEBKGND, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE,
    WM_PAINT, WM_SETCURSOR,
};

use crate::config::{update_config, with_config, DotLabel, HudConfig, RelField, StField};
use crate::render::{fill_rect, text, Fonts};

fn bg() -> Color { Color::from_rgba8(18, 18, 20, 255) }
fn side() -> Color { Color::from_rgba8(12, 12, 14, 255) }
fn tab_on() -> Color { Color::from_rgba8(32, 32, 36, 255) }
fn text_col() -> Color { Color::from_rgba8(236, 236, 240, 255) }
fn muted() -> Color { Color::from_rgba8(132, 132, 140, 255) }
fn row_line() -> Color { Color::from_rgba8(255, 255, 255, 16) }
fn chip_hover() -> Color { Color::from_rgba8(48, 48, 54, 255) }
fn accent() -> Color { Color::from_rgba8(255, 148, 48, 255) }
fn knob() -> Color { Color::from_rgba8(248, 248, 250, 255) }
fn track_off() -> Color { Color::from_rgba8(58, 58, 64, 255) }
fn btn_bg() -> Color { Color::from_rgba8(40, 40, 46, 255) }

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Standings,
    Relative,
    Map,
    Minimap,
    Radar,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Hit {
    TabSt,
    TabRel,
    TabMap,
    TabMini,
    TabRadar,
    StShow,
    RelShow,
    MapShow,
    MiniShow,
    RadarShow,
    StPos,
    StNum,
    StName,
    StGap,
    StLaps,
    StBest,
    StStatus,
    StBike,
    StPenalty,
    StCrashed,
    StInterval,
    RelNum,
    RelName,
    RelGap,
    RelPos,
    RelBike,
    RelPenalty,
    RelInterval,
    RelCrashed,
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
    StBgDec,
    StBgInc,
    RelBgDec,
    RelBgInc,
    MapBgDec,
    MapBgInc,
    MiniBgDec,
    MiniBgInc,
    RadarBgDec,
    RadarBgInc,
    StDec,
    StInc,
    RelDec,
    RelInc,
    StDrag(u8),
    RelDrag(u8),
    StWDec(u8),
    StWInc(u8),
    RelWDec(u8),
    RelWInc(u8),
}

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

struct SettingsUi {
    host: HWND,
    tab: Tab,
    hover: Option<Hit>,
    hits: Vec<HitBox>,
    open_drop: Option<Drop>,
    drag: Option<ColDrag>,
}

unsafe impl Send for SettingsUi {}

static UI: Mutex<Option<SettingsUi>> = Mutex::new(None);

const SIDE_W: f32 = 176.0;

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

pub fn handle_message(msg: u32, _wp: WPARAM, lp: LPARAM) -> bool {
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
        WM_SETCURSOR => {
            let (over, dragging) = {
                let ui = UI.lock().unwrap();
                let ui = ui.as_ref();
                let hover = ui.and_then(|u| u.hover);
                let dragging = ui.and_then(|u| u.drag).is_some();
                let grip = matches!(hover, Some(Hit::StDrag(_)) | Some(Hit::RelDrag(_)));
                (hover.is_some(), dragging || grip)
            };
            unsafe {
                let idc = if dragging {
                    IDC_SIZEALL
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

fn update_drag(p: (f32, f32)) {
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
        return;
    };
    match id {
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
        Hit::MapDotOpen => {
            toggle_drop(Drop::MapDot);
            return;
        }
        Hit::MiniDotOpen => {
            toggle_drop(Drop::MiniDot);
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
        Hit::StPos => c.st_pos = !c.st_pos,
        Hit::StNum => c.st_num = !c.st_num,
        Hit::StName => c.st_name = !c.st_name,
        Hit::StGap => c.st_gap = !c.st_gap,
        Hit::StLaps => c.st_laps = !c.st_laps,
        Hit::StBest => c.st_best = !c.st_best,
        Hit::StStatus => c.st_status = !c.st_status,
        Hit::StBike => c.st_bike = !c.st_bike,
        Hit::StPenalty => c.st_penalty = !c.st_penalty,
        Hit::StCrashed => c.st_crashed = !c.st_crashed,
        Hit::StInterval => c.st_interval = !c.st_interval,
        Hit::RelNum => c.rel_num = !c.rel_num,
        Hit::RelName => c.rel_name = !c.rel_name,
        Hit::RelGap => c.rel_gap = !c.rel_gap,
        Hit::RelPos => c.rel_pos = !c.rel_pos,
        Hit::RelBike => c.rel_bike = !c.rel_bike,
        Hit::RelPenalty => c.rel_penalty = !c.rel_penalty,
        Hit::RelInterval => c.rel_interval = !c.rel_interval,
        Hit::RelCrashed => c.rel_crashed = !c.rel_crashed,
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
        Hit::StBgDec => c.st_bg = (c.st_bg - 5).max(0),
        Hit::StBgInc => c.st_bg = (c.st_bg + 5).min(100),
        Hit::RelBgDec => c.rel_bg = (c.rel_bg - 5).max(0),
        Hit::RelBgInc => c.rel_bg = (c.rel_bg + 5).min(100),
        Hit::MapBgDec => c.map_bg = (c.map_bg - 5).max(0),
        Hit::MapBgInc => c.map_bg = (c.map_bg + 5).min(100),
        Hit::MiniBgDec => c.mini_bg = (c.mini_bg - 5).max(0),
        Hit::MiniBgInc => c.mini_bg = (c.mini_bg + 5).min(100),
        Hit::RadarBgDec => c.radar_bg = (c.radar_bg - 5).max(0),
        Hit::RadarBgInc => c.radar_bg = (c.radar_bg + 5).min(100),
        Hit::StDec => c.standings_rows = (c.standings_rows - 1).max(3),
        Hit::StInc => c.standings_rows = (c.standings_rows + 1).min(40),
        Hit::RelDec => c.relative_count = (c.relative_count - 1).max(1),
        Hit::RelInc => c.relative_count = (c.relative_count + 1).min(8),
        Hit::StWDec(i) => {
            if let Some(f) = c.st_order.get(i as usize).copied() {
                f.add_width(c, -2);
            }
        }
        Hit::StWInc(i) => {
            if let Some(f) = c.st_order.get(i as usize).copied() {
                f.add_width(c, 2);
            }
        }
        Hit::RelWDec(i) => {
            if let Some(f) = c.rel_order.get(i as usize).copied() {
                f.add_width(c, -2);
            }
        }
        Hit::RelWInc(i) => {
            if let Some(f) = c.rel_order.get(i as usize).copied() {
                f.add_width(c, 2);
            }
        }
        Hit::TabSt | Hit::TabRel | Hit::TabMap | Hit::TabMini | Hit::TabRadar
        | Hit::MapDotOpen | Hit::MiniDotOpen | Hit::StDrag(_) | Hit::RelDrag(_) => {}
    });
}

fn set_tab(tab: Tab) {
    if let Some(ui) = UI.lock().unwrap().as_mut() {
        ui.tab = tab;
        ui.open_drop = None;
        ui.drag = None;
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
        fill_rect(px, r, row_line());
    }

    let cfg = with_config(|c| c.clone());
    let (tab, hover, open_drop, drag) = {
        let ui = UI.lock().unwrap();
        let ui = ui.as_ref();
        (
            ui.map(|u| u.tab).unwrap_or(Tab::Standings),
            ui.and_then(|u| u.hover),
            ui.and_then(|u| u.open_drop),
            ui.and_then(|u| u.drag),
        )
    };
    let mut hits = Vec::new();

    text(px, fonts, "MXBO", 11.0, 20.0, 18.0, accent(), false);
    text(px, fonts, "Settings", 18.0, 20.0, 36.0, text_col(), false);

    let tabs = [
        (Tab::Standings, Hit::TabSt, "Standings", "Race order", cfg.show_standings),
        (Tab::Relative, Hit::TabRel, "Relative", "Riders nearby", cfg.show_relative),
        (Tab::Map, Hit::TabMap, "Map", "Track layout", cfg.show_map),
        (Tab::Minimap, Hit::TabMini, "Minimap", "Numbered circle", cfg.show_minimap),
        (Tab::Radar, Hit::TabRadar, "Radar", "Proximity", cfg.show_radar),
    ];
    let mut ty = 78.0;
    for (t, hit, name, hint, on) in tabs {
        nav_tab(px, fonts, 8.0, ty, SIDE_W - 16.0, 42.0, t == tab, on, name, hint, hit, hover, &mut hits);
        ty += 46.0;
    }
    text(px, fonts, "F8  ·  Ctrl+drag move / resize", 11.0, 16.0, h - 28.0, muted(), false);

    let x = SIDE_W + 28.0;
    let cw = (w - x - 28.0).max(200.0);
    match tab {
        Tab::Standings => pane_standings(px, fonts, &cfg, hover, drag, &mut hits, x, 24.0, cw),
        Tab::Relative => pane_relative(px, fonts, &cfg, hover, drag, &mut hits, x, 24.0, cw),
        Tab::Map => pane_map(px, fonts, &cfg, hover, open_drop == Some(Drop::MapDot), &mut hits, x, 24.0, cw),
        Tab::Minimap => pane_minimap(px, fonts, &cfg, hover, open_drop == Some(Drop::MiniDot), &mut hits, x, 24.0, cw),
        Tab::Radar => pane_radar(px, fonts, &cfg, hover, &mut hits, x, 24.0, cw),
    }

    if let Some(ui) = UI.lock().unwrap().as_mut() {
        ui.hits = hits;
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
    hint: &str,
    hit: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) {
    hits.push(HitBox { id: hit, x, y, w, h });
    if selected {
        fill_round(px, x, y, w, h, 8.0, tab_on());
        if let Some(r) = Rect::from_xywh(x, y + 8.0, 3.0, h - 16.0) {
            fill_rect(px, r, accent());
        }
    } else if hover == Some(hit) {
        fill_round(px, x, y, w, h, 8.0, chip_hover());
    }
    let tx = x + 16.0;
    text(px, fonts, name, 13.0, tx, y + 6.0, if selected { text_col() } else { muted() }, false);
    text(px, fonts, hint, 10.0, tx, y + 22.0, muted(), false);
    fill_circle(px, x + w - 16.0, y + h * 0.5, 4.0, if visible { accent() } else { track_off() });
}

fn pane_standings(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    drag: Option<ColDrag>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) {
    heading(px, fonts, x, y, "Standings", "Who is ahead and by how much");
    let mut y = y + 56.0;
    y = toggle_row(px, fonts, x, y, w, "Show on overlay", cfg.show_standings, Hit::StShow, hover, hits);
    y = stepper_row(px, fonts, x, y, w, "Background", &format!("{}%", cfg.st_bg), Hit::StBgDec, Hit::StBgInc, hover, hits);
    y = stepper_row(px, fonts, x, y, w, "Rows", &cfg.standings_rows.to_string(), Hit::StDec, Hit::StInc, hover, hits);
    y = section(px, fonts, x, y, "Columns  ·  drag to reorder, −/+ width");
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
            Hit::StWDec(i as u8),
            Hit::StWInc(i as u8),
            i,
            hover,
            drag.filter(|d| d.kind == DragKind::St),
            hits,
        );
    }
}

fn pane_relative(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    drag: Option<ColDrag>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) {
    heading(px, fonts, x, y, "Relative", "Riders just ahead and behind you");
    let mut y = y + 56.0;
    y = toggle_row(px, fonts, x, y, w, "Show on overlay", cfg.show_relative, Hit::RelShow, hover, hits);
    y = stepper_row(px, fonts, x, y, w, "Background", &format!("{}%", cfg.rel_bg), Hit::RelBgDec, Hit::RelBgInc, hover, hits);
    y = stepper_row(px, fonts, x, y, w, "Nearby riders", &cfg.relative_count.to_string(), Hit::RelDec, Hit::RelInc, hover, hits);
    y = section(px, fonts, x, y, "Columns  ·  drag to reorder, −/+ width");
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
            Hit::RelWDec(i as u8),
            Hit::RelWInc(i as u8),
            i,
            hover,
            drag.filter(|d| d.kind == DragKind::Rel),
            hits,
        );
    }
}

fn pane_map(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    drop_open: bool,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) {
    heading(px, fonts, x, y, "Map", "Where you and others are on track");
    let mut y = y + 56.0;
    y = toggle_row(px, fonts, x, y, w, "Show on overlay", cfg.show_map, Hit::MapShow, hover, hits);
    y = stepper_row(px, fonts, x, y, w, "Background", &format!("{}%", cfg.map_bg), Hit::MapBgDec, Hit::MapBgInc, hover, hits);
    y = section(px, fonts, x, y, "On the map");
    y = toggle_row(px, fonts, x, y, w, "Other riders", cfg.map_others, Hit::MapOthers, hover, hits);
    y = toggle_row(px, fonts, x, y, w, "Start / finish", cfg.map_sf, Hit::MapSf, hover, hits);
    y = toggle_row(px, fonts, x, y, w, "Track arrows", cfg.map_arrows, Hit::MapArrows, hover, hits);
    y = toggle_row(px, fonts, x, y, w, "Leader crown", cfg.map_crown, Hit::MapCrown, hover, hits);
    y = toggle_row(px, fonts, x, y, w, "Ahead / behind", cfg.map_place, Hit::MapPlace, hover, hits);
    y = toggle_row(px, fonts, x, y, w, "Numbers in dots", cfg.map_numbers, Hit::MapNumbers, hover, hits);
    dropdown_row(
        px,
        fonts,
        x,
        y,
        w,
        "Dot number",
        cfg.map_dot.label(),
        drop_open,
        Hit::MapDotOpen,
        &[
            (Hit::MapDotNum, "Number", cfg.map_dot == DotLabel::Number),
            (Hit::MapDotPos, "Position", cfg.map_dot == DotLabel::Position),
        ],
        hover,
        hits,
    );
}

fn pane_minimap(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    drop_open: bool,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) {
    heading(px, fonts, x, y, "Minimap", "Circular track with numbered riders");
    let mut y = y + 56.0;
    y = toggle_row(px, fonts, x, y, w, "Show on overlay", cfg.show_minimap, Hit::MiniShow, hover, hits);
    y = stepper_row(px, fonts, x, y, w, "Background", &format!("{}%", cfg.mini_bg), Hit::MiniBgDec, Hit::MiniBgInc, hover, hits);
    y = section(px, fonts, x, y, "On the minimap");
    y = toggle_row(px, fonts, x, y, w, "Other riders", cfg.mini_others, Hit::MiniOthers, hover, hits);
    y = toggle_row(px, fonts, x, y, w, "Start / finish", cfg.mini_sf, Hit::MiniSf, hover, hits);
    y = toggle_row(px, fonts, x, y, w, "Track arrows", cfg.mini_arrows, Hit::MiniArrows, hover, hits);
    y = toggle_row(px, fonts, x, y, w, "Leader crown", cfg.mini_crown, Hit::MiniCrown, hover, hits);
    y = toggle_row(px, fonts, x, y, w, "Ahead / behind", cfg.mini_place, Hit::MiniPlace, hover, hits);
    y = toggle_row(px, fonts, x, y, w, "Numbers in dots", cfg.mini_numbers, Hit::MiniNumbers, hover, hits);
    dropdown_row(
        px,
        fonts,
        x,
        y,
        w,
        "Dot number",
        cfg.mini_dot.label(),
        drop_open,
        Hit::MiniDotOpen,
        &[
            (Hit::MiniDotNum, "Number", cfg.mini_dot == DotLabel::Number),
            (Hit::MiniDotPos, "Position", cfg.mini_dot == DotLabel::Position),
        ],
        hover,
        hits,
    );
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
) {
    heading(px, fonts, x, y, "Radar", "Riders beside and behind you");
    let mut y = y + 56.0;
    y = toggle_row(px, fonts, x, y, w, "Show on overlay", cfg.show_radar, Hit::RadarShow, hover, hits);
    y = stepper_row(px, fonts, x, y, w, "Panel opacity", &format!("{}%", cfg.radar_bg), Hit::RadarBgDec, Hit::RadarBgInc, hover, hits);
    y = section(px, fonts, x, y, "On the radar");
    y = toggle_row(px, fonts, x, y, w, "Side proximity", cfg.radar_sides, Hit::RadarSides, hover, hits);
    toggle_row(px, fonts, x, y, w, "Rear proximity", cfg.radar_rear, Hit::RadarRear, hover, hits);
}

fn heading(px: &mut Pixmap, fonts: &Fonts, x: f32, y: f32, title: &str, sub: &str) {
    text(px, fonts, title, 22.0, x, y, text_col(), false);
    text(px, fonts, sub, 13.0, x, y + 28.0, muted(), false);
}

fn section(px: &mut Pixmap, fonts: &Fonts, x: f32, y: f32, label: &str) -> f32 {
    text(px, fonts, label, 11.0, x, y + 18.0, muted(), false);
    y + 40.0
}

fn st_toggle(f: StField) -> Hit {
    match f {
        StField::Pos => Hit::StPos,
        StField::Num => Hit::StNum,
        StField::Name => Hit::StName,
        StField::Gap => Hit::StGap,
        StField::Laps => Hit::StLaps,
        StField::Best => Hit::StBest,
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
        RelField::Pos => Hit::RelPos,
        RelField::Bike => Hit::RelBike,
        RelField::Penalty => Hit::RelPenalty,
        RelField::Interval => Hit::RelInterval,
        RelField::Crashed => Hit::RelCrashed,
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
    wdec: Hit,
    winc: Hit,
    i: usize,
    hover: Option<Hit>,
    col_drag: Option<ColDrag>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let h = 44.0;
    let cluster = 148.0;
    let grabbed = col_drag.is_some_and(|d| d.from as usize == i);
    let drop = col_drag.is_some_and(|d| d.over as usize == i && d.from as usize != i);
    let hot = hover == Some(drag) || hover == Some(toggle) || hover == Some(wdec) || hover == Some(winc);
    if grabbed {
        fill_round(px, x - 8.0, y, w + 16.0, h, 8.0, tab_on());
        if let Some(r) = Rect::from_xywh(x - 8.0, y + 8.0, 3.0, h - 16.0) {
            fill_rect(px, r, accent());
        }
    } else if drop {
        fill_round(px, x - 8.0, y, w + 16.0, h, 8.0, Color::from_rgba8(255, 148, 48, 28));
    } else if hot {
        fill_round(px, x - 8.0, y, w + 16.0, h, 8.0, chip_hover());
    }
    hits.push(HitBox {
        id: drag,
        x,
        y,
        w: (w - cluster).max(48.0),
        h,
    });
    draw_grip(px, x + 2.0, y + h * 0.5);
    text(px, fonts, label, 14.0, x + 28.0, y + 13.0, text_col(), false);
    let bw = 22.0;
    let bh = 28.0;
    let by = y + 8.0;
    let ix = x + w - 40.0 - 8.0 - bw;
    let vx = ix - 8.0 - 32.0;
    let dx = vx - 8.0 - bw;
    arrow_btn(px, fonts, dx, by, bw, bh, "−", wdec, hover, hits);
    text(px, fonts, &width.to_string(), 13.0, vx + 16.0, y + 13.0, muted(), true);
    arrow_btn(px, fonts, ix, by, bw, bh, "+", winc, hover, hits);
    switch(px, x + w - 40.0, y + 11.0, on, toggle, hover, hits);
    if drop {
        let from = col_drag.map(|d| d.from as usize).unwrap_or(i);
        let ly = if from > i { y } else { y + h - 2.0 };
        if let Some(r) = Rect::from_xywh(x, ly, w, 2.0) {
            fill_rect(px, r, accent());
        }
    } else if let Some(r) = Rect::from_xywh(x, y + h - 1.0, w, 1.0) {
        fill_rect(px, r, row_line());
    }
    y + h
}

fn draw_grip(px: &mut Pixmap, x: f32, cy: f32) {
    let c = Color::from_rgba8(108, 108, 116, 255);
    for row in 0..3 {
        for col in 0..2 {
            fill_circle(px, x + col as f32 * 6.0, cy - 6.0 + row as f32 * 6.0, 1.6, c);
        }
    }
}

fn arrow_btn(
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
    let bg = if hover == Some(hit) { chip_hover() } else { btn_bg() };
    fill_round(px, x, y, w, h, 6.0, bg);
    text(px, fonts, label, 11.0, x + w * 0.5, y + 7.0, text_col(), true);
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
    let h = 44.0;
    hits.push(HitBox { id: hit, x, y, w, h });
    if hover == Some(hit) {
        fill_round(px, x - 8.0, y, w + 16.0, h, 8.0, chip_hover());
    }
    text(px, fonts, label, 14.0, x, y + 13.0, text_col(), false);
    switch(px, x + w - 40.0, y + 11.0, on, hit, hover, hits);
    if let Some(r) = Rect::from_xywh(x, y + h - 1.0, w, 1.0) {
        fill_rect(px, r, row_line());
    }
    y + h
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
    let h = 44.0;
    text(px, fonts, label, 14.0, x, y + 13.0, text_col(), false);
    let bw = 30.0;
    let bh = 28.0;
    let by = y + 8.0;
    let ix = x + w - bw;
    let dx = ix - 86.0 - bw;
    btn(px, fonts, dx, by, bw, bh, "−", dec, hover, hits);
    text(px, fonts, value, 15.0, dx + bw + 43.0, y + 12.0, text_col(), true);
    btn(px, fonts, ix, by, bw, bh, "+", inc, hover, hits);
    if let Some(r) = Rect::from_xywh(x, y + h - 1.0, w, 1.0) {
        fill_rect(px, r, row_line());
    }
    y + h
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
    let h = 44.0;
    text(px, fonts, label, 14.0, x, y + 13.0, text_col(), false);
    let bw = 136.0;
    let bh = 28.0;
    let bx = x + w - bw;
    let by = y + 8.0;
    hits.push(HitBox { id: open_hit, x: bx, y: by, w: bw, h: bh });
    let hot = open || hover == Some(open_hit);
    fill_round(px, bx, by, bw, bh, 6.0, if hot { chip_hover() } else { btn_bg() });
    text(px, fonts, value, 13.0, bx + 10.0, by + 6.0, text_col(), false);
    chevron(px, bx + bw - 14.0, by + bh * 0.5, open, muted());
    if let Some(r) = Rect::from_xywh(x, y + h - 1.0, w, 1.0) {
        fill_rect(px, r, row_line());
    }
    if open {
        let item_h = 30.0;
        let pad = 4.0;
        let mh = pad * 2.0 + item_h * options.len() as f32;
        let mx = bx;
        let my = by + bh + 4.0;
        fill_round(px, mx - 1.0, my - 1.0, bw + 2.0, mh + 2.0, 9.0, Color::from_rgba8(8, 8, 10, 255));
        fill_round(px, mx, my, bw, mh, 8.0, Color::from_rgba8(28, 28, 32, 255));
        for (i, (hit, name, selected)) in options.iter().enumerate() {
            let iy = my + pad + i as f32 * item_h;
            hits.push(HitBox { id: *hit, x: mx, y: iy, w: bw, h: item_h });
            if hover == Some(*hit) {
                fill_round(px, mx + 4.0, iy, bw - 8.0, item_h, 6.0, chip_hover());
            } else if *selected {
                fill_round(px, mx + 4.0, iy, bw - 8.0, item_h, 6.0, tab_on());
            }
            let col = if *selected { accent() } else { text_col() };
            text(px, fonts, name, 13.0, mx + 12.0, iy + 7.0, col, false);
        }
    }
    y + h
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
    let w = 40.0;
    let h = 22.0;
    hits.push(HitBox { id: hit, x, y, w, h });
    let mut track = if on { accent() } else { track_off() };
    if hover == Some(hit) && !on {
        track = chip_hover();
    }
    fill_round(px, x, y, w, h, 11.0, track);
    let kx = if on { x + w - 11.0 } else { x + 11.0 };
    fill_circle(px, kx, y + h * 0.5, 8.0, knob());
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
    let bg = if hover == Some(hit) { chip_hover() } else { btn_bg() };
    fill_round(px, x, y, w, h, 6.0, bg);
    text(px, fonts, label, 16.0, x + w * 0.5, y + 4.0, text_col(), true);
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
