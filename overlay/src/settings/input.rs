
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateSolidBrush, DeleteObject, EndPaint, FillRect, ScreenToClient, HGDIOBJ, PAINTSTRUCT,
};
use windows::Win32::UI::Accessibility::NotifyWinEvent;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, ReleaseCapture, SetCapture, VK_BACK, VK_CONTROL, VK_DOWN, VK_ESCAPE, VK_LEFT,
    VK_RETURN, VK_RIGHT, VK_SHIFT, VK_SPACE, VK_TAB, VK_UP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, LoadCursorW, SetCursor,
    SetWindowTextW, EVENT_OBJECT_FOCUS, EVENT_OBJECT_NAMECHANGE, IDC_ARROW, IDC_HAND, IDC_IBEAM, IDC_SIZEALL, OBJID_CLIENT, WM_CHAR, WM_ERASEBKGND,
    WM_EXITSIZEMOVE, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL,
    WM_PAINT, WM_SETCURSOR,
};

use crate::config::{
    hsv_to_rgb, rgb_to_hsv, update_config, with_config, WidgetId, COL_W_MAX, COL_W_MIN, RADAR_RANGE_MAX, RADAR_RANGE_MIN,
};
use mxbo_hud::pitboard::{write_pack, PitVar};

use super::*;

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
            let p = cursor_from_lparam(lp);
            set_hover(p);
            update_drag(p);
            true
        }
        WM_LBUTTONDOWN => {
            press(cursor_from_lparam(lp));
            true
        }
        WM_LBUTTONUP => {
            release(cursor_from_lparam(lp));
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
                } else if ui.pit_help_open {
                } else if ui.reply_id.is_some() {
                    ui.reply_scroll =
                        (ui.reply_scroll - delta * 0.4).clamp(0.0, ui.reply_scroll_max);
                } else {
                    let over_nav = in_client
                        && ui.nav_bottom > ui.nav_top
                        && px < SIDE_W
                        && py >= ui.nav_top
                        && py < ui.nav_bottom;
                    if ui.hover == Some(Hit::AnalyzeMap) && ui.analyze_id.is_some() {
                        let z = ui.analyze_zoom * if delta > 0.0 { 1.12 } else { 0.89 };
                        ui.analyze_zoom = clamp_map_zoom(z);
                    } else if ui.hover == Some(Hit::TrackMap) && ui.tracks_selected.is_some() {
                        let z = ui.tracks_zoom * if delta > 0.0 { 1.12 } else { 0.89 };
                        ui.tracks_zoom = clamp_map_zoom(z);
                    } else if over_nav {
                        let max = (ui.nav_content_h - (ui.nav_bottom - ui.nav_top)).max(0.0);
                        ui.nav_scroll = (ui.nav_scroll - delta * 0.4).clamp(0.0, max);
                    } else {
                        ui.scroll = (ui.scroll - delta * 0.4).clamp(0.0, ui.scroll_max);
                    }
                }
            }
            true
        }
        WM_CHAR => {
            let ch = char::from_u32(wp.0 as u32).unwrap_or('\0');
            if pit_name_focused() {
                pit_name_push(ch);
                true
            } else if group_name_focused() || group_member_focused() {
                group_push(ch);
                true
            } else {
                crate::feedback::on_char(ch)
            }
        }
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
        WM_EXITSIZEMOVE => {
            let host = UI.lock().unwrap().as_ref().map(|u| u.host);
            if let Some(host) = host {
                persist_host_pos(host);
            }
            false
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

pub(crate) fn cursor_from_lparam(lp: LPARAM) -> (f32, f32) {
    let x = (lp.0 as i32) as i16 as f32;
    let y = ((lp.0 as i32) >> 16) as i16 as f32;
    (x, y)
}

pub(crate) fn set_hover(p: (f32, f32)) {
    let mut ui = UI.lock().unwrap();
    let Some(ui) = ui.as_mut() else {
        return;
    };
    ui.hover = hit_at(&ui.hits, p.0, p.1);
}

pub(crate) fn press(p: (f32, f32)) {
    let (id, host) = {
        let ui = UI.lock().unwrap();
        let Some(ui) = ui.as_ref() else {
            return;
        };
        (hit_at(&ui.hits, p.0, p.1), ui.host)
    };
    match id {
        Some(Hit::AnalyzeScrub) => {
            if let Some(ui) = UI.lock().unwrap().as_mut() {
                if let Some(h) = ui.hits.iter().rev().find(|h| h.id == Hit::AnalyzeScrub) {
                    ui.analyze_scrub = ((p.0 - h.x) / h.w.max(1.0)).clamp(0.0, 1.0);
                }
                ui.analyze_scrubbing = true;
            }
            unsafe {
                let _ = SetCapture(host);
            }
        }
        Some(Hit::AnalyzeMap) => {
            let follow = {
                let ui = UI.lock().unwrap();
                ui.as_ref().and_then(|u| {
                    if !u.analyze_follow {
                        return None;
                    }
                    Some((
                        u.analyze_id?,
                        u.analyze_you_lap,
                        u.analyze_warmup,
                        u.analyze_scrub,
                    ))
                })
            };
            let pan =
                follow.and_then(|(id, lap, wu, scrub)| review::follow_pan_for(id, lap, wu, scrub));
            if let Some(ui) = UI.lock().unwrap().as_mut() {
                if let Some((px, pz)) = pan {
                    ui.analyze_pan_x = px;
                    ui.analyze_pan_z = pz;
                }
                if follow.is_some() {
                    ui.analyze_follow = false;
                }
                ui.map_drag = Some((p.0, p.1, ui.analyze_pan_x, ui.analyze_pan_z));
            }
            unsafe {
                let _ = SetCapture(host);
            }
        }
        Some(Hit::TrackMap) => {
            if let Some(ui) = UI.lock().unwrap().as_mut() {
                ui.tracks_map_drag = Some((p.0, p.1, ui.tracks_pan_x, ui.tracks_pan_z));
            }
            unsafe {
                let _ = SetCapture(host);
            }
        }
        Some(Hit::PrimarySaturation) | Some(Hit::GameUiPrimarySaturation) | Some(Hit::PitYellowSaturation)
        | Some(Hit::PitBlueSaturation)
        | Some(Hit::PitSlotColorSaturation) => {
            start_sv_drag(p, host);
        }
        Some(Hit::PitSlotGrab(i)) => {
            set_pit_selected(i);
            PIT_DESIGN.with(|design| design.borrow_mut().dragging = true);
            unsafe {
                let _ = SetCapture(host);
            }
            place_pit_slot_at(p.0, p.1);
        }
        Some(Hit::StDrag(i)) => start_drag(DragKind::St, i, host),
        Some(Hit::RelDrag(i)) => start_drag(DragKind::Rel, i, host),
        Some(hit) if is_slider(hit) => {
            let on_track = {
                let ui = UI.lock().unwrap();
                ui.as_ref().is_some_and(|u| {
                    u.hits.iter().rev().any(|h| {
                        h.id == hit
                            && h.h <= 24.0
                            && p.0 >= h.x
                            && p.0 <= h.x + h.w
                            && p.1 >= h.y
                            && p.1 <= h.y + h.h
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

pub(crate) fn start_drag(kind: DragKind, i: u8, host: HWND) {
    close_drop();
    if let Some(ui) = UI.lock().unwrap().as_mut() {
        ui.drag = Some(ColDrag {
            kind,
            from: i,
            over: i,
        });
    }
    unsafe {
        let _ = SetCapture(host);
    }
}

pub(crate) fn is_slider(hit: Hit) -> bool {
    matches!(
        hit,
        Hit::StBg
            | Hit::StHl
            | Hit::RelBg
            | Hit::RelHl
            | Hit::MapBg
            | Hit::MapZoom
            | Hit::MapDotOpacity
            | Hit::MiniBg
            | Hit::MiniZoom
            | Hit::RadarRange
            | Hit::RadarBg
            | Hit::DashBg
            | Hit::TickerBg
            | Hit::TickerHl
            | Hit::SysBg
            | Hit::SectorBg
            | Hit::DeltaBg
            | Hit::StanceBg
            | Hit::FlagBg
            | Hit::LeanBg
            | Hit::GamepadBg
            | Hit::TelemetryBg
            | Hit::PitBg
            | Hit::TimerBg
            | Hit::StW(_)
            | Hit::RelW(_)
            | Hit::Font(_)
            | Hit::PrimaryHue
            | Hit::GameUiPrimaryHue
            | Hit::PitYellowHue
            | Hit::PitBlueHue
            | Hit::PitSlotSize
            | Hit::PitSlotColorHue
    )
}

pub(crate) fn slide_range(hit: Hit) -> (i32, i32) {
    match hit {
        Hit::StW(i) => {
            let max = with_config(|c| {
                c.standings.st_order
                    .get(i as usize)
                    .map(|f| f.width_max())
                    .unwrap_or(COL_W_MAX)
            });
            (COL_W_MIN, max)
        }
        Hit::RelW(i) => {
            let max = with_config(|c| {
                c.relative.rel_order
                    .get(i as usize)
                    .map(|f| f.width_max())
                    .unwrap_or(COL_W_MAX)
            });
            (COL_W_MIN, max)
        }
        Hit::Font(_) => (70, 160),
        Hit::RadarRange => (RADAR_RANGE_MIN, RADAR_RANGE_MAX),
        Hit::PrimaryHue | Hit::GameUiPrimaryHue | Hit::PitYellowHue | Hit::PitBlueHue
        | Hit::PitSlotColorHue => (0, 360),
        Hit::PitSlotSize => (10, 56),
        _ => (0, 100),
    }
}

pub(crate) fn start_slide(hit: Hit, mx: f32, host: HWND) {
    if color_kind_from_hue(hit).is_none() {
        close_drop();
    }
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

pub(crate) fn start_sv_drag(p: (f32, f32), host: HWND) {
    let box_ = {
        let ui = UI.lock().unwrap();
        ui.as_ref().and_then(|u| {
            u.hits
                .iter()
                .rev()
                .find(|h| {
                    matches!(
                        h.id,
                        Hit::PrimarySaturation | Hit::GameUiPrimarySaturation | Hit::PitYellowSaturation | Hit::PitBlueSaturation
                    )
                })
                .copied()
        })
    };
    let Some(hb) = box_ else {
        return;
    };
    if let Some(ui) = UI.lock().unwrap().as_mut() {
        ui.sv_drag = Some((hb.x, hb.y, hb.w, hb.h));
    }
    apply_sv(p.0, p.1, hb.x, hb.y, hb.w, hb.h);
    unsafe {
        let _ = SetCapture(host);
    }
}

pub(crate) fn apply_sv(mx: f32, my: f32, x: f32, y: f32, w: f32, h: f32) {
    let s = if w <= 1.0 {
        0.0
    } else {
        ((mx - x) / w).clamp(0.0, 1.0)
    };
    let v = if h <= 1.0 {
        1.0
    } else {
        (1.0 - (my - y) / h).clamp(0.0, 1.0)
    };
    let kind = color_kind_from_drop(UI.lock().unwrap().as_ref().and_then(|u| u.open_drop))
        .unwrap_or(ColorPickKind::App);
    update_config(|c| {
        let (hue, _, _) = rgb_to_hsv(read_kind_color(c, kind));
        write_kind_color(c, kind, hsv_to_rgb(hue, s, v));
    });
}

pub(crate) fn apply_slide(hit: Hit, mx: f32, x: f32, w: f32, min: i32, max: i32) {
    let t = if w <= 1.0 {
        0.0
    } else {
        ((mx - x) / w).clamp(0.0, 1.0)
    };
    let v = min + ((max - min) as f32 * t).round() as i32;
    update_config(|c| match hit {
        Hit::StBg => c[WidgetId::Standings].bg = v,
        Hit::StHl => c.standings.st_hl = v,
        Hit::RelBg => c[WidgetId::Relative].bg = v,
        Hit::RelHl => c.relative.rel_hl = v,
        Hit::MapBg => c[WidgetId::Map].bg = v,
        Hit::MapZoom => c.map.map_zoom = v,
        Hit::MapDotOpacity => c.map.map_dot_opacity = v,
        Hit::MiniBg => c[WidgetId::Minimap].bg = v,
        Hit::MiniZoom => c.mini.mini_zoom = v,
        Hit::RadarRange => c.radar.radar_range = v,
        Hit::RadarBg => c[WidgetId::Radar].bg = v,
        Hit::DashBg => c[WidgetId::Dash].bg = v,
        Hit::TickerBg => c[WidgetId::Ticker].bg = v,
        Hit::TickerHl => c.ticker.ticker_hl = v,
        Hit::SysBg => c[WidgetId::Sys].bg = v,
        Hit::SectorBg => c[WidgetId::Sector].bg = v,
        Hit::DeltaBg => c[WidgetId::Delta].bg = v,
        Hit::StanceBg => c[WidgetId::Stance].bg = v,
        Hit::FlagBg => c[WidgetId::Flag].bg = v,
        Hit::LeanBg => c[WidgetId::Lean].bg = v,
        Hit::GamepadBg => c[WidgetId::Gamepad].bg = v,
        Hit::TelemetryBg => c[WidgetId::Telemetry].bg = v,
        Hit::PitBg => c[WidgetId::Pitboard].bg = v,
        Hit::TimerBg => c[WidgetId::Timer].bg = v,
        Hit::StW(i) => {
            if let Some(f) = c.standings.st_order.get(i as usize).copied() {
                f.set_width(c, v);
            }
        }
        Hit::RelW(i) => {
            if let Some(f) = c.relative.rel_order.get(i as usize).copied() {
                f.set_width(c, v);
            }
        }
        Hit::Font(id) => c.set_font_pct(id, v),
        Hit::PitSlotSize => {
            if let Some(place) = c.pit.pit_vars.get_mut(pit_selected() as usize) {
                place.size = v as f32;
            }
            let _ = write_pack(&c.pit.pit_art, &c.pit.pit_vars, c.pit.pit_text);
        }
        hit if color_kind_from_hue(hit).is_some() => {
            let kind = color_kind_from_hue(hit).unwrap();
            let (_, s, val) = rgb_to_hsv(read_kind_color(c, kind));
            write_kind_color(c, kind, hsv_to_rgb(v as f32, s, val));
        }
        _ => {}
    });
}

pub(crate) fn place_pit_slot_at(mx: f32, my: f32) {
    let (preview, slot) = PIT_DESIGN.with(|design| {
        let design = design.borrow();
        (design.preview, design.selected as usize)
    });
    let Some((x, y, w, h)) = preview else {
        return;
    };
    if w < 1.0 || h < 1.0 {
        return;
    }
    let nx = ((mx - x) / w).clamp(0.04, 0.96);
    let ny = ((my - y) / h).clamp(0.04, 0.96);
    let mut cfg = mxbo_hud::config::CONFIG
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    if let Some(place) = cfg.pit.pit_vars.get_mut(slot) {
        place.x = nx;
        place.y = ny;
    }
}

pub(crate) fn update_drag(p: (f32, f32)) {
    if PIT_DESIGN.with(|design| design.borrow().dragging) {
        place_pit_slot_at(p.0, p.1);
        return;
    }
    {
        let mut ui = UI.lock().unwrap();
        if let Some(ui) = ui.as_mut() {
            if let Some((x, y, w, h)) = ui.sv_drag {
                apply_sv(p.0, p.1, x, y, w, h);
                return;
            }
            if ui.analyze_scrubbing {
                if let Some(h) = ui.hits.iter().rev().find(|h| h.id == Hit::AnalyzeScrub) {
                    ui.analyze_scrub = ((p.0 - h.x) / h.w.max(1.0)).clamp(0.0, 1.0);
                }
                return;
            }
            if let Some((sx, sy, px0, pz0)) = ui.tracks_map_drag {
                ui.tracks_pan_x = px0 - (p.0 - sx) * 0.8 / ui.tracks_zoom.max(1.0);
                ui.tracks_pan_z = pz0 + (p.1 - sy) * 0.8 / ui.tracks_zoom.max(1.0);
                return;
            }
            if let Some((sx, sy, px0, pz0)) = ui.map_drag {
                ui.analyze_pan_x = px0 - (p.0 - sx) * 0.8 / ui.analyze_zoom.max(1.0);
                ui.analyze_pan_z = pz0 + (p.1 - sy) * 0.8 / ui.analyze_zoom.max(1.0);
                return;
            }
        }
    }
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
    let over = if ui.col_list_n > 0 && ui.col_list_kind == Some(drag.kind) {
        drag_over_slots(ui.col_list_y, ui.col_list_n, p.1)
    } else {
        drag_over(&ui.hits, drag.kind, p.1)
    };
    if let Some(over) = over {
        if let Some(d) = ui.drag.as_mut() {
            d.over = over;
        }
    }
}

pub(crate) fn drag_over(hits: &[HitBox], kind: DragKind, y: f32) -> Option<u8> {
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

pub(crate) fn release(p: (f32, f32)) {
    update_drag(p);
    let was_pit = PIT_DESIGN.with(|design| {
        let mut design = design.borrow_mut();
        let on = design.dragging;
        design.dragging = false;
        on
    });
    if was_pit {
        update_config(|c| {
            let _ = write_pack(&c.pit.pit_art, &c.pit.pit_vars, c.pit.pit_text);
        });
        unsafe {
            let _ = ReleaseCapture();
        }
        return;
    }
    let was_sv = {
        let mut ui = UI.lock().unwrap();
        ui.as_mut().is_some_and(|u| {
            u.analyze_scrubbing = false;
            u.map_drag = None;
            u.tracks_map_drag = None;
            u.sv_drag.take().is_some()
        })
    };
    let slide_hit = {
        let mut ui = UI.lock().unwrap();
        ui.as_mut().and_then(|u| u.slide.take()).map(|s| s.hit)
    };
    if was_sv
        || matches!(slide_hit, Some(hit) if color_kind_from_hue(hit).is_some())
    {
        if was_sv {
            if let Some(kind) =
                color_kind_from_drop(UI.lock().unwrap().as_ref().and_then(|u| u.open_drop))
            {
                sync_after_kind(kind);
            }
        } else if let Some(hit) = slide_hit {
            if let Some(kind) = color_kind_from_hue(hit) {
                sync_after_kind(kind);
            }
        }
    }
    if slide_hit.is_some() {
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

pub(crate) fn click(p: (f32, f32)) {
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

pub(crate) fn is_drop_pick(hit: Hit) -> bool {
    matches!(
        hit,
        Hit::MapDotNum
            | Hit::MapDotPos
            | Hit::MapYouTextWhite
            | Hit::MapYouTextBlack
            | Hit::MiniDotNum
            | Hit::MiniDotPos
            | Hit::StTextWhite
            | Hit::StTextBlack
            | Hit::RelTextWhite
            | Hit::RelTextBlack
            | Hit::StPlaqueTextWhite
            | Hit::StPlaqueTextBlack
            | Hit::RelPlaqueTextWhite
            | Hit::RelPlaqueTextBlack
            | Hit::FontSegoe
            | Hit::FontArial
            | Hit::FontTahoma
            | Hit::FontRoboto
            | Hit::FontExo2
            | Hit::FontTeko
            | Hit::FontGoldman
            | Hit::FontMontserrat
            | Hit::UnitsPick(_, _)
            | Hit::SettingsKeyPick(_)
            | Hit::LanguagePick(_)
            | Hit::ThemePick(_)
            | Hit::StanceModePick(_)
            | Hit::StanceStylePick(_)
            | Hit::LeanStylePick(_)
            | Hit::RadarStylePick(_)
            | Hit::GamepadStylePick(_)
            | Hit::GamepadThemePick(_)
            | Hit::SysAddPick(_)
            | Hit::DashFootPick(_, _)
            | Hit::TickerFootPick(_, _)
            | Hit::InfoPick(_, _, _)
            | Hit::PitSlotPick(_, _)
            | Hit::PitSlotNone(_)
            | Hit::PitWhenAlways
            | Hit::PitWhenSector
            | Hit::PitWhenLap
            | Hit::PresetCopyTo(_)
            | Hit::PresetCopyAll
            | Hit::AnalyzeYouLap(_)
            | Hit::AnalyzeCompare(_)
    )
}

pub(crate) fn is_focusable(hit: Hit) -> bool {
    !matches!(
        hit,
        Hit::StDrag(_)
            | Hit::RelDrag(_)
            | Hit::UpdateBanner
            | Hit::WhatsNewScrim
            | Hit::WhatsNewPanel
            | Hit::PitHelpScrim
            | Hit::PitHelpPanel
            | Hit::ReplyScrim
            | Hit::ReplyPanel
            | Hit::ClearScrim
            | Hit::ClearPanel
            | Hit::PrimaryPanel
            | Hit::PrimarySaturation
            | Hit::GameUiPrimaryPanel
            | Hit::GameUiPrimarySaturation
            | Hit::PitYellowPanel
            | Hit::PitYellowSaturation
            | Hit::PitBluePanel
            | Hit::PitBlueSaturation
            | Hit::PitSlotColorPanel
            | Hit::PitSlotColorSaturation
            | Hit::PitSlotGrab(_)
    )
}

pub(crate) fn set_kb_focus(id: Hit) {
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

pub(crate) fn hit_label(hit: Hit) -> String {
    let english = match hit {
        Hit::TabWidgets => "Widgets".into(),
        Hit::TabApp => "Settings".into(),
        Hit::AppLook => "Look".into(),
        Hit::AppMenus => "In game HUD".into(),
        Hit::AppInstall => "Install".into(),
        Hit::AppStartup => "Startup".into(),
        Hit::AppStream => "Stream".into(),
        Hit::AppLabs => "Labs".into(),
        Hit::AppUpdates => "Updates".into(),
        Hit::AppDiagnostics => "Diagnostics".into(),
        Hit::TabGroups => "Groups".into(),
        Hit::GroupSelect(_) => "Group".into(),
        Hit::GroupUp(_) => "Move group up".into(),
        Hit::GroupDown(_) => "Move group down".into(),
        Hit::GroupNew => "New group".into(),
        Hit::GroupDelete => "Delete group".into(),
        Hit::GroupsOnMaps => "Show on maps".into(),
        Hit::GroupIcon(_) => "Group icon".into(),
        Hit::GroupColor(_) => "Group color".into(),
        Hit::GroupMemberRemove(_) => "Remove rider".into(),
        Hit::GroupCheck(_) => "Select rider".into(),
        Hit::GroupAddSelected => "Add selected".into(),
        Hit::GroupAddName => "Add name".into(),
        Hit::GroupName => "Group name".into(),
        Hit::GroupMember => "Rider name".into(),
        Hit::DiagCopy => "Copy crash report".into(),
        Hit::TabProfile => "Profile".into(),
        Hit::TabFeedback => "Feedback".into(),
        Hit::ProfileNavOverview => "Overview".into(),
        Hit::ProfileNavMotos => "Motos".into(),
        Hit::ProfileNavMyMxb => "MyMXB".into(),
        Hit::ProfileNavRanked => "MXB-Ranked".into(),
        Hit::ProfileNavCbr => "CBR".into(),
        Hit::ProfileNavTracks => "Tracks".into(),
        Hit::MyMxbRefresh | Hit::RankedRefresh | Hit::CbrRefresh => "Refresh".into(),
        Hit::TrackOpen(_) => "Open track".into(),
        Hit::TrackBack => "Back".into(),
        Hit::TrackMap => "Track map".into(),
        Hit::ProfileAllTime => "All time".into(),
        Hit::ProfileTwoWeeks => "Last 14 days".into(),
        Hit::ProfileRanked => "Ranked motos".into(),
        Hit::ProfileClear => "Clear Profile".into(),
        Hit::ReviewClear => "Clear Motos".into(),
        Hit::ClearCancel => "Cancel".into(),
        Hit::ClearConfirm => "Clear".into(),
        Hit::ProfileAxis(i) => profile_axis_tip(i).into(),
        Hit::ReviewFilterAll => "All motos".into(),
        Hit::ReviewFilterMxbRanked => "MXB-Ranked motos".into(),
        Hit::ReviewFilterCbr => "CBR motos".into(),
        Hit::ReviewFilterSaved => "Saved motos".into(),
        Hit::ReviewOpen(_) => "Open race".into(),
        Hit::ReviewKeep(_) => "Save race".into(),
        Hit::ReviewDelete(_) => "Delete race".into(),
        Hit::ReviewBack => "Back".into(),
        Hit::ReviewToggle => profile_record_tip(with_config(|c| c.review)).into(),
        Hit::AnalyzeCompare(0) => "No compare".into(),
        Hit::AnalyzeCompare(_) => "Compare rider".into(),
        Hit::AnalyzeCompareOpen => "Choose rider".into(),
        Hit::AnalyzeYouLap(_) => "Your lap".into(),
        Hit::AnalyzeLapOpen => "Choose lap".into(),
        Hit::AnalyzeScrub => "Lap position".into(),
        Hit::AnalyzeMap => "Race map".into(),
        Hit::AnalyzeFollow => "Follow scrubber".into(),
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
        Hit::TabLean => "Lean".into(),
        Hit::TabGamepad => "Controller".into(),
        Hit::TabTelemetry => "Telemetry".into(),
        Hit::TabPitboard => "Pit Board".into(),
        Hit::TabTimer => "Session".into(),
        Hit::PitBrowse => "PNG or JPEG. An empty name uses the file name. A picture with no board.json starts as one slot.".into(),
        Hit::PitExport => "Save this pit board folder, plate.png and board.json".into(),
        Hit::PitImport => "Open a board.json that has plate.png beside it".into(),
        Hit::PitDemo => "Save the demo plate as a PNG, then upload it".into(),
        Hit::PitDelete => "Delete this saved pit board".into(),
        Hit::PitName => "Name this pit board".into(),
        Hit::PitBoardOpen => "Saved pit boards".into(),
        Hit::PitBoardFactory => "Holeshot plate".into(),
        Hit::PitBoardPick(_) => "Use this pit board".into(),
        Hit::PitSlotGrab(_) => "Drag this slot".into(),
        Hit::PitSlotSelect(_) => "Select this slot".into(),
        Hit::PitSlotAdd => "Add a slot".into(),
        Hit::PitSlotRemove => "Remove the selected slot".into(),
        Hit::PitSlotSize => "Text size".into(),
        Hit::PitSlotBold => "Bold this slot".into(),
        Hit::PitSlotCenterX => "Center horizontally".into(),
        Hit::PitSlotCenterY => "Center vertically".into(),
        Hit::PitSlotColorOpen => "Text color".into(),
        Hit::PitSlotColorReset => "Black. Lap delta stays green or red.".into(),
        Hit::PitReset => "Restore the Holeshot slots".into(),
        Hit::PitOpenFolder => "Plates folder, plus a starter board.json".into(),
        Hit::PitWhenAlways => "Show the board all the time with live values".into(),
        Hit::PitWhenSector => "5-second snapshot of the sector that just finished".into(),
        Hit::PitWhenLap => "5-second snapshot of the lap that just finished".into(),
        Hit::PitWhenOpen => "When the board shows values".into(),
        Hit::PitSlotOpen(_) => "What this spot shows".into(),
        Hit::PitSlotPick(_, i) => PitVar::from_idx(i).map(|v| v.label().into()).unwrap_or_else(|| "Stat".into()),
        Hit::PitSlotNone(_) => "None".into(),
        Hit::Preset(p) => {
            let (live, active, editing) =
                with_config(|c| (c.session_live, c.active_preset, c.settings_preset));
            if live && p == active && p != editing {
                mxbo_hud::i18n::t_fmt(
                    "{preset} — live HUD",
                    &[("preset", mxbo_hud::i18n::t(p.label()))],
                )
            } else {
                p.label().into()
            }
        }
        Hit::PresetCopyOpen => {
            let src = with_config(|c| c.settings_preset);
            mxbo_hud::i18n::t_fmt(
                "Copy {preset} to another preset",
                &[("preset", mxbo_hud::i18n::t(src.label()))],
            )
        }
        Hit::PresetCopyTo(p) => mxbo_hud::i18n::t_fmt(
            "Replace {preset} with this layout",
            &[("preset", mxbo_hud::i18n::t(p.label()))],
        ),
        Hit::PresetCopyAll => "Replace all presets with this layout".into(),
        Hit::FeatureSector => "Experimental features (Tracks)".into(),
        Hit::GameUi => "MX Bikes menus".into(),
        Hit::StShow
        | Hit::RelShow
        | Hit::MapShow
        | Hit::MiniShow
        | Hit::RadarShow
        | Hit::DashShow
        | Hit::TickerShow
        | Hit::SysShow
        | Hit::SectorShow
        | Hit::DeltaShow
        | Hit::StanceShow
        | Hit::FlagShow
        | Hit::LeanShow
        | Hit::GamepadShow
        | Hit::TelemetryShow
        | Hit::PitShow
        | Hit::TimerShow => "Show on overlay".into(),
        Hit::TimerOf => "Out of riders".into(),
        Hit::QuitApp => "Quit overlay".into(),
        Hit::Font(_) => "Font size".into(),
        Hit::Bold(_) => "Bold text".into(),
        Hit::StBg | Hit::RelBg | Hit::MapBg | Hit::MiniBg => "Background".into(),
        Hit::RadarBg
        | Hit::DashBg
        | Hit::TickerBg
        | Hit::SysBg
        | Hit::SectorBg
        | Hit::DeltaBg
        | Hit::StanceBg
        | Hit::FlagBg
        | Hit::LeanBg
        | Hit::GamepadBg
        | Hit::TelemetryBg
        | Hit::PitBg
        | Hit::TimerBg => "Panel opacity".into(),
        Hit::StHl | Hit::RelHl | Hit::TickerHl => "Row highlight".into(),
        Hit::StStripe | Hit::RelStripe => "Alternating rows".into(),
        Hit::StPlaque | Hit::RelPlaque => "Show plaques".into(),
        Hit::StTextOpen | Hit::RelTextOpen | Hit::MapYouTextOpen => "Text color".into(),
        Hit::StPlaqueTextOpen | Hit::RelPlaqueTextOpen => "Plaque text".into(),
        Hit::StTextWhite
        | Hit::StTextBlack
        | Hit::RelTextWhite
        | Hit::RelTextBlack
        | Hit::StPlaqueTextWhite
        | Hit::StPlaqueTextBlack
        | Hit::RelPlaqueTextWhite
        | Hit::RelPlaqueTextBlack
        | Hit::MapYouTextWhite
        | Hit::MapYouTextBlack => "Color".into(),
        Hit::StDec | Hit::StInc => "Rows".into(),
        Hit::RelDec | Hit::RelInc => "Nearby riders".into(),
        Hit::TickerDec | Hit::TickerInc => "Riders shown".into(),
        Hit::TickerSlide => "Slide on pass".into(),
        Hit::Snap(_, align) => snap_align_label(align).into(),
        Hit::StW(_) | Hit::RelW(_) => "Column width".into(),
        Hit::FontOpen => "Font".into(),
        Hit::PrimaryOpen | Hit::PrimaryPanel => "Primary color".into(),
        Hit::PrimarySaturation => "Saturation and brightness".into(),
        Hit::PrimaryHue => "Hue".into(),
        Hit::PrimarySwatch(_) => "Color swatch".into(),
        Hit::PrimaryReset => "Reset primary color".into(),
        Hit::GameUiMatchPrimary => "Match app accent".into(),
        Hit::GameUiPrimaryOpen | Hit::GameUiPrimaryPanel => "Menu color".into(),
        Hit::GameUiPrimarySaturation => "Saturation and brightness".into(),
        Hit::GameUiPrimaryHue => "Hue".into(),
        Hit::GameUiPrimarySwatch(_) => "Color swatch".into(),
        Hit::GameUiPrimaryReset => "Reset menu color".into(),
        Hit::PitYellowOpen | Hit::PitYellowPanel => "Main".into(),
        Hit::PitYellowSaturation => "Saturation and brightness".into(),
        Hit::PitYellowHue => "Hue".into(),
        Hit::PitYellowSwatch(_) => "Color swatch".into(),
        Hit::PitYellowReset => "Reset main".into(),
        Hit::PitBlueOpen | Hit::PitBluePanel => "Secondary".into(),
        Hit::PitBlueSaturation => "Saturation and brightness".into(),
        Hit::PitBlueHue => "Hue".into(),
        Hit::PitBlueSwatch(_) => "Color swatch".into(),
        Hit::PitBlueReset => "Reset secondary".into(),
        Hit::GameUiSplashBrowse => "Browse opening screen image".into(),
        Hit::GameUiSplashDefault => "Use default opening screen".into(),
        Hit::GameUiLoadingBrowse => "Browse loading screen image".into(),
        Hit::GameUiLoadingDefault => "Use default loading screen".into(),
        Hit::UnitsOpen(kind) => kind.label().into(),
        Hit::UnitsPick(_, units) => units.label().into(),
        Hit::SettingsKeyOpen => "Settings key".into(),
        Hit::LanguageOpen => "Language".into(),
        Hit::LanguagePick(lang) => lang.endonym().into(),
        Hit::ThemeOpen => "Theme".into(),
        Hit::ThemePick(theme) => theme.label().into(),
        Hit::StanceBindOpen => {
            if UI.lock().unwrap().as_ref().is_some_and(|u| u.bind_listen) {
                "Press a pad button, key, or mouse button now. Escape cancels.".into()
            } else {
                "Sit button".into()
            }
        }
        Hit::StanceModeOpen => "Sit mode".into(),
        Hit::StanceStyleOpen | Hit::LeanStyleOpen | Hit::RadarStyleOpen => "Look".into(),
        Hit::GamepadStyleOpen => "Pad".into(),
        Hit::GamepadThemeOpen => "Theme".into(),
        Hit::SysAddOpen => "Add app".into(),
        Hit::SysAppBrowse => "Browse .exe".into(),
        Hit::SysAppShow(_) => "Show on Systems".into(),
        Hit::SysAppRemove(_) => "Remove app".into(),
        Hit::SysAddPick(_) => "Add app".into(),
        Hit::StanceShowSit => "Show sitting".into(),
        Hit::FlagYellow => "Yellow flag".into(),
        Hit::FlagBlue => "Blue flag".into(),
        Hit::FlagRed => "Red flag".into(),
        Hit::FlagText => "Text".into(),
        Hit::RadarSides => "Side proximity".into(),
        Hit::RadarRear => "Rear proximity".into(),
        Hit::RadarRings => "Range rings".into(),
        Hit::RadarRange => "Range".into(),
        Hit::DashRev => "Rev indicator".into(),
        Hit::DashYellow => "Yellow flag".into(),
        Hit::DashBlue => "Blue flag".into(),
        Hit::DashRed => "Red flag".into(),
        Hit::DashSimple => "Simple dash".into(),
        Hit::DashShiftColor => "Shift color".into(),
        Hit::MapSectors => "Sector lines".into(),
        Hit::MapZoom => "Zoom".into(),
        Hit::MapDotOpacity => "Dot opacity".into(),
        Hit::MiniSectors => "Sector lines".into(),
        Hit::TelemetryTraces => "Traces".into(),
        Hit::TelemetryTraceThrottle => "Throttle trace".into(),
        Hit::TelemetryTraceBrake => "Brake trace".into(),
        Hit::TelemetryTraceSteer => "Steer trace".into(),
        Hit::TelemetryBars => "Analog bars".into(),
        Hit::TelemetryBarClutch => "Clutch bar".into(),
        Hit::TelemetryBarBrake => "Brake bar".into(),
        Hit::TelemetryBarThrottle => "Throttle bar".into(),
        Hit::TelemetryBarSteer => "Steer bar".into(),
        Hit::TelemetryDial => "Gear / speed".into(),
        Hit::SectorLive => "Live sector".into(),
        Hit::SectorSession | Hit::DeltaSession => "Compare to session best".into(),
        Hit::SectorHist => "Lap log".into(),
        Hit::SectorHistDec | Hit::SectorHistInc => "Laps back".into(),
        Hit::StanceReset => "Reset to standing".into(),
        Hit::TrackPbClear => "Clear this track".into(),
        Hit::FbText => "Feedback message".into(),
        Hit::FbSend => "Send feedback".into(),
        Hit::Uninstall => "Uninstall".into(),
        Hit::GameFolder => "MX Bikes folder".into(),
        Hit::StreamEnabled => "Browser Source".into(),
        Hit::StreamCopyUrl => "Copy OBS URL".into(),
        Hit::StreamCopyEditUrl => "Copy layout editor URL".into(),
        Hit::StreamOpenUrl => "Open OBS URL in the browser".into(),
        Hit::StreamOpenEditUrl => "Updates in your browser".into(),
        Hit::StreamCopyGameToStream => "Copy this preset's game layout onto its stream slot".into(),
        Hit::UpdateCheck => "Check for updates".into(),
        Hit::UpdateInstall => "Install update".into(),
        Hit::WhatsNewOpen => {
            if crate::changelog::previewing() {
                "Preview next".into()
            } else {
                "What's new".into()
            }
        }
        Hit::WhatsNewDismiss | Hit::ReplyDismiss | Hit::PitHelpDismiss => "Got it".into(),
        Hit::WhatsNewLink => "Open link".into(),
        Hit::PitHelpOpen => "How to make a pit board".into(),
        Hit::ReplySend => "Send".into(),
        Hit::ReplyText => "Write a reply".into(),
        _ => "Control".into(),
    };
    mxbo_hud::i18n::t(&english).to_string()
}

pub(crate) fn announce(host: HWND, label: &str) {
    let title = format!(
        "Holeshot HUD — {} — {label}",
        mxbo_hud::i18n::t("Settings")
    );
    let mut buf: Vec<u16> = title.encode_utf16().collect();
    buf.push(0);
    unsafe {
        let _ = SetWindowTextW(host, PCWSTR(buf.as_ptr()));
        NotifyWinEvent(EVENT_OBJECT_NAMECHANGE, host, OBJID_CLIENT.0, 0);
        NotifyWinEvent(EVENT_OBJECT_FOCUS, host, OBJID_CLIENT.0, 0);
    }
}

pub(crate) fn handle_key(vk: u16, shift: bool, _ctrl: bool) -> bool {
    if pit_name_focused() {
        if vk == VK_ESCAPE.0 {
            revert_pit_name();
            set_pit_name_focus(false);
            return true;
        }
        if vk == VK_RETURN.0 {
            if commit_pit_name() {
                set_pit_name_focus(false);
            }
            return true;
        }
        if vk == VK_BACK.0 {
            pit_name_backspace();
            return true;
        }
        return true;
    }
    if group_name_focused() || group_member_focused() {
        if vk == VK_ESCAPE.0 {
            group_revert_focus();
            return true;
        }
        if vk == VK_RETURN.0 {
            if group_name_focused() {
                commit_group_name();
                group_revert_focus();
            } else {
                dispatch(Hit::GroupAddName, (0.0, 0.0));
            }
            return true;
        }
        if vk == VK_BACK.0 {
            group_backspace();
            return true;
        }
        return true;
    }
    let fb = crate::feedback::is_focused();
    if fb && vk != VK_TAB.0 && vk != VK_ESCAPE.0 {
        return false;
    }
    if vk == VK_ESCAPE.0 {
        let whats_new = UI
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|u| u.whats_new_open);
        if whats_new {
            dismiss_whats_new();
            return true;
        }
        let pit_help = UI
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|u| u.pit_help_open);
        if pit_help {
            dismiss_pit_help();
            return true;
        }
        let clear_open = UI
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|u| u.clear_confirm.is_some());
        if clear_open {
            dismiss_clear_confirm();
            return true;
        }
        let reply_open = UI
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|u| u.reply_id.is_some());
        if reply_open {
            if crate::feedback::compose_snapshot().focused {
                crate::feedback::set_compose_focus(false);
                return true;
            }
            dismiss_reply();
            return true;
        }
        let open = UI
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|u| u.open_drop.is_some() || u.bind_listen);
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

pub(crate) fn cycle_focus(forward: bool) {
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

pub(crate) fn activate_hit(id: Hit) {
    let p = {
        let ui = UI.lock().unwrap();
        ui.as_ref().and_then(|u| {
            u.hits
                .iter()
                .filter(|h| h.id == id)
                .max_by(|a, b| {
                    (a.w * a.h)
                        .partial_cmp(&(b.w * b.h))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|h| (h.x + h.w * 0.5, h.y + h.h * 0.5))
        })
    };
    let Some(p) = p else {
        return;
    };
    dispatch(id, p);
}

pub(crate) fn move_drop_option(dir: i32) -> bool {
    let picks: Vec<Hit> = {
        let ui = UI.lock().unwrap();
        let Some(ui) = ui.as_ref() else {
            return false;
        };
        if ui.open_drop.is_none() {
            return false;
        }
        ui.hits
            .iter()
            .map(|h| h.id)
            .filter(|id| is_drop_pick(*id))
            .collect::<Vec<_>>()
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
    let cur = UI.lock().unwrap().as_ref().and_then(|u| u.hover).or(UI
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|u| u.focus));
    let idx = cur
        .and_then(|c| picks.iter().position(|h| *h == c))
        .unwrap_or(0);
    let next = (idx as i32 + dir).rem_euclid(picks.len() as i32) as usize;
    let id = picks[next];
    if let Some(ui) = UI.lock().unwrap().as_mut() {
        ui.hover = Some(id);
        ui.focus = Some(id);
    }
    true
}

pub(crate) fn nudge_slider(hit: Hit, delta: i32) {
    let (min, max) = slide_range(hit);
    let v = with_config(|c| match hit {
        Hit::StBg => c[WidgetId::Standings].bg,
        Hit::StHl => c.standings.st_hl,
        Hit::RelBg => c[WidgetId::Relative].bg,
        Hit::RelHl => c.relative.rel_hl,
        Hit::MapBg => c[WidgetId::Map].bg,
        Hit::MapZoom => c.map.map_zoom,
        Hit::MapDotOpacity => c.map.map_dot_opacity,
        Hit::MiniBg => c[WidgetId::Minimap].bg,
        Hit::MiniZoom => c.mini.mini_zoom,
        Hit::RadarRange => c.radar.radar_range,
        Hit::RadarBg => c[WidgetId::Radar].bg,
        Hit::DashBg => c[WidgetId::Dash].bg,
        Hit::TickerBg => c[WidgetId::Ticker].bg,
        Hit::TickerHl => c.ticker.ticker_hl,
        Hit::SysBg => c[WidgetId::Sys].bg,
        Hit::SectorBg => c[WidgetId::Sector].bg,
        Hit::DeltaBg => c[WidgetId::Delta].bg,
        Hit::StanceBg => c[WidgetId::Stance].bg,
        Hit::FlagBg => c[WidgetId::Flag].bg,
        Hit::LeanBg => c[WidgetId::Lean].bg,
        Hit::GamepadBg => c[WidgetId::Gamepad].bg,
        Hit::TelemetryBg => c[WidgetId::Telemetry].bg,
        Hit::PitBg => c[WidgetId::Pitboard].bg,
        Hit::TimerBg => c[WidgetId::Timer].bg,
        Hit::StW(i) => c
            .standings.st_order
            .get(i as usize)
            .map(|f| f.width(c))
            .unwrap_or(min),
        Hit::RelW(i) => c
            .relative.rel_order
            .get(i as usize)
            .map(|f| f.width(c))
            .unwrap_or(min),
        Hit::Font(id) => c.font_pct(id),
        Hit::PitSlotSize => c
            .pit.pit_vars
            .get(pit_selected() as usize)
            .map(|place| place.size.round() as i32)
            .unwrap_or(18),
        hit if color_kind_from_hue(hit).is_some() => {
            rgb_to_hsv(read_kind_color(c, color_kind_from_hue(hit).unwrap()))
                .0
                .round() as i32
        }
        _ => min,
    });
    let v = (v + delta).clamp(min, max);
    update_config(|c| match hit {
        Hit::StBg => c[WidgetId::Standings].bg = v,
        Hit::StHl => c.standings.st_hl = v,
        Hit::RelBg => c[WidgetId::Relative].bg = v,
        Hit::RelHl => c.relative.rel_hl = v,
        Hit::MapBg => c[WidgetId::Map].bg = v,
        Hit::MapZoom => c.map.map_zoom = v,
        Hit::MapDotOpacity => c.map.map_dot_opacity = v,
        Hit::MiniBg => c[WidgetId::Minimap].bg = v,
        Hit::MiniZoom => c.mini.mini_zoom = v,
        Hit::RadarRange => c.radar.radar_range = v,
        Hit::RadarBg => c[WidgetId::Radar].bg = v,
        Hit::DashBg => c[WidgetId::Dash].bg = v,
        Hit::TickerBg => c[WidgetId::Ticker].bg = v,
        Hit::TickerHl => c.ticker.ticker_hl = v,
        Hit::SysBg => c[WidgetId::Sys].bg = v,
        Hit::SectorBg => c[WidgetId::Sector].bg = v,
        Hit::DeltaBg => c[WidgetId::Delta].bg = v,
        Hit::StanceBg => c[WidgetId::Stance].bg = v,
        Hit::FlagBg => c[WidgetId::Flag].bg = v,
        Hit::LeanBg => c[WidgetId::Lean].bg = v,
        Hit::GamepadBg => c[WidgetId::Gamepad].bg = v,
        Hit::TelemetryBg => c[WidgetId::Telemetry].bg = v,
        Hit::PitBg => c[WidgetId::Pitboard].bg = v,
        Hit::TimerBg => c[WidgetId::Timer].bg = v,
        Hit::StW(i) => {
            if let Some(f) = c.standings.st_order.get(i as usize).copied() {
                f.set_width(c, v);
            }
        }
        Hit::RelW(i) => {
            if let Some(f) = c.relative.rel_order.get(i as usize).copied() {
                f.set_width(c, v);
            }
        }
        Hit::Font(id) => c.set_font_pct(id, v),
        Hit::PitSlotSize => {
            if let Some(place) = c.pit.pit_vars.get_mut(pit_selected() as usize) {
                place.size = v as f32;
            }
            let _ = write_pack(&c.pit.pit_art, &c.pit.pit_vars, c.pit.pit_text);
        }
        hit if color_kind_from_hue(hit).is_some() => {
            let kind = color_kind_from_hue(hit).unwrap();
            let (_, s, val) = rgb_to_hsv(read_kind_color(c, kind));
            write_kind_color(c, kind, hsv_to_rgb(v as f32, s, val));
        }
        _ => {}
    });
    if let Some(kind) = color_kind_from_hue(hit) {
        sync_after_kind(kind);
    }
}

pub(crate) fn set_tab(tab: Tab) {
    crate::feedback::set_focus(false);
    let (tab, force_motos) = match tab {
        Tab::Review => (Tab::Profile, true),
        other => (other, false),
    };
    if let Some(ui) = UI.lock().unwrap().as_mut() {
        if tab.is_widget() {
            ui.last_widget = tab;
        }
        ui.tab = tab;
        if force_motos {
            ui.profile_section = ProfileSection::Motos;
        }
        ui.open_drop = None;
        ui.drop_scroll = 0.0;
        ui.drop_menu = None;
        ui.drag = None;
        ui.slide = None;
        ui.scroll = 0.0;
        ui.bind_listen = false;
    }
}

pub(crate) fn set_app_section(section: AppSection) {
    crate::feedback::set_focus(false);
    if let Some(ui) = UI.lock().unwrap().as_mut() {
        ui.app_section = section;
        ui.open_drop = None;
        ui.drop_scroll = 0.0;
        ui.drop_menu = None;
        ui.drag = None;
        ui.slide = None;
        ui.scroll = 0.0;
        ui.bind_listen = false;
    }
}

pub(crate) fn set_profile_section(section: ProfileSection) {
    crate::feedback::set_focus(false);
    if let Some(ui) = UI.lock().unwrap().as_mut() {
        ui.tab = Tab::Profile;
        ui.profile_section = section;
        ui.open_drop = None;
        ui.drop_scroll = 0.0;
        ui.drop_menu = None;
        ui.drag = None;
        ui.slide = None;
        ui.scroll = 0.0;
        ui.bind_listen = false;
    }
}

pub(crate) fn toggle_drop(drop: Drop) {
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

pub(crate) fn close_drop() {
    if let Some(ui) = UI.lock().unwrap().as_mut() {
        ui.open_drop = None;
        ui.drop_scroll = 0.0;
        ui.drop_menu = None;
        ui.bind_listen = false;
    }
}

pub(crate) fn hit_at(hits: &[HitBox], x: f32, y: f32) -> Option<Hit> {
    hits.iter()
        .rev()
        .find(|h| x >= h.x && x <= h.x + h.w && y >= h.y && y <= h.y + h.h)
        .map(|h| h.id)
}

pub(crate) fn persist_analyze_follow_pan(
    id: Option<i64>,
    you_lap: i32,
    warmup: bool,
    scrub: f32,
    zoom: f32,
    follow: bool,
) {
    if !follow || zoom <= 1.0 {
        return;
    }
    let Some(id) = id else {
        return;
    };
    let Some((px, pz)) = review::follow_pan_for(id, you_lap, warmup, scrub) else {
        return;
    };
    if let Some(ui) = UI.lock().unwrap().as_mut() {
        if ui.analyze_follow && ui.analyze_id == Some(id) {
            ui.analyze_pan_x = px;
            ui.analyze_pan_z = pz;
        }
    }
}

