use std::sync::atomic::Ordering;

use tiny_skia::Pixmap;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
};
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, GetClientRect, GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId,
    IsIconic, IsWindow, IsWindowVisible, SetForegroundWindow, SetWindowPos, ShowWindow,
    HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SW_HIDE,
    SW_RESTORE, SW_SHOW,
};

use crate::config::{update_config, with_config, EditSurface, FontFamily, StanceBind};
use crate::render::Fonts;

use super::*;

pub fn attach(host: HWND) {
    sync_titlebar(host);
    *UI.lock().unwrap() = Some(SettingsUi {
        host,
        tab: Tab::Standings,
        last_widget: Tab::Standings,
        app_section: AppSection::Look,
        profile_section: ProfileSection::Overview,
        hover: None,
        focus: None,
        hits: Vec::new(),
        open_drop: None,
        drag: None,
        slide: None,
        sv_drag: None,
        scroll: 0.0,
        content_h: 0.0,
        scroll_max: 0.0,
        motos_list_scroll: 0.0,
        nav_scroll: 0.0,
        nav_content_h: 0.0,
        nav_top: 0.0,
        nav_bottom: 0.0,
        banner_dismissed: false,
        whats_new_open: false,
        pit_help_open: false,
        whats_new_scroll: 0.0,
        whats_new_scroll_max: 0.0,
        reply_id: None,
        reply_scroll: 0.0,
        reply_scroll_max: 0.0,
        drop_scroll: 0.0,
        drop_menu: None,
        bind_listen: false,
        review_filter: crate::review::ListFilter::All,
        review_stay_on_list: false,
        analyze_id: None,
        analyze_compare: -1,
        analyze_you_lap: -1,
        analyze_warmup: false,
        analyze_scrub: 0.0,
        analyze_zoom: 1.0,
        analyze_pan_x: 0.0,
        analyze_pan_z: 0.0,
        analyze_scrubbing: false,
        analyze_follow: false,
        map_drag: None,
        profile_all_time: true,
        profile_ranked_only: false,
        tracks_selected: None,
        tracks_zoom: 1.0,
        tracks_pan_x: 0.0,
        tracks_pan_z: 0.0,
        tracks_map_drag: None,
        clear_confirm: None,
        st_col_slides: ColSlides::new(),
        rel_col_slides: ColSlides::new(),
        col_list_y: 0.0,
        col_list_n: 0,
        col_list_kind: None,
    });
}

pub fn show(host: HWND) {
    unsafe {
        force_to_front(host);
    }
    crate::feedback::refresh();
    let on_motos = UI.lock().unwrap().as_ref().is_some_and(|u| {
        u.tab == Tab::Review
            || (u.tab == Tab::Profile && u.profile_section == ProfileSection::Motos)
    });
    if on_motos {
        open_live_analyze(true);
    }
}

pub fn hide(host: HWND) {
    persist_host_pos(host);
    update_config(|c| c.edit_surface = EditSurface::Game);
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

pub(crate) const SETTINGS_W: i32 = 1000;
pub(crate) const SETTINGS_H: i32 = 720;

/// How much of the title bar must stay on a work area to keep the saved origin.
const TITLE_KEEP_W: i32 = 80;
const TITLE_H: i32 = 32;

/// Settings host origin for `CreateWindowExW`. Virtual-screen coords, clamped per-monitor.
pub fn host_create_pos() -> (i32, i32) {
    let (x, y) = with_config(|c| (c.settings_x, c.settings_y));
    clamp_settings_origin(x, y, SETTINGS_W, SETTINGS_H, &work_areas())
}

/// Write the host's current virtual-screen origin. Skips minimized windows.
pub fn persist_host_pos(host: HWND) {
    unsafe {
        if host.0.is_null() || !IsWindow(host).as_bool() || IsIconic(host).as_bool() {
            return;
        }
        let mut r = RECT::default();
        if GetWindowRect(host, &mut r).is_err() {
            return;
        }
        let x = r.left;
        let y = r.top;
        if with_config(|c| c.settings_x == x && c.settings_y == y) {
            return;
        }
        update_config(|c| {
            c.settings_x = x;
            c.settings_y = y;
        });
    }
}

/// Keep the saved origin when its title bar is still on a live monitor.
/// If that monitor is gone, snap into the nearest work area — not the primary unless that is nearest.
pub(crate) fn clamp_settings_origin(
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    works: &[[i32; 4]],
) -> (i32, i32) {
    if works.iter().any(|&wa| title_bar_on_work(x, y, w, wa)) {
        return (x, y);
    }
    let Some(wa) = nearest_work(x, y, w, h, works) else {
        return (80, 80);
    };
    snap_into_work(x, y, w, h, wa)
}

/// Analyze / Tracks map zoom: wheel scale stays in 1…12.
pub(crate) fn clamp_map_zoom(z: f32) -> f32 {
    z.clamp(1.0, 12.0)
}

pub(crate) fn title_bar_on_work(x: i32, y: i32, w: i32, wa: [i32; 4]) -> bool {
    let [l, t, r, b] = wa;
    let overlap_l = x.max(l);
    let overlap_r = (x + w).min(r);
    let overlap_w = overlap_r.saturating_sub(overlap_l);
    let top_in = y < b && y + TITLE_H > t;
    overlap_w >= TITLE_KEEP_W && top_in
}

pub(crate) fn nearest_work(x: i32, y: i32, w: i32, h: i32, works: &[[i32; 4]]) -> Option<[i32; 4]> {
    let cx = x.saturating_add(w / 2);
    let cy = y.saturating_add(h / 2);
    works.iter().copied().min_by_key(|&[l, t, r, b]| {
        let nx = cx.clamp(l, r.saturating_sub(1));
        let ny = cy.clamp(t, b.saturating_sub(1));
        let dx = (cx - nx) as i64;
        let dy = (cy - ny) as i64;
        dx * dx + dy * dy
    })
}

pub(crate) fn snap_into_work(x: i32, y: i32, w: i32, h: i32, wa: [i32; 4]) -> (i32, i32) {
    let [l, t, r, b] = wa;
    let max_x = (r - w).max(l);
    let max_y = (b - h).max(t);
    (x.clamp(l, max_x), y.clamp(t, max_y))
}

pub(crate) fn work_areas() -> Vec<[i32; 4]> {
    let mut list = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(
            HDC::default(),
            None,
            Some(enum_work_areas),
            LPARAM(&mut list as *mut Vec<[i32; 4]> as isize),
        );
    }
    list
}

unsafe extern "system" fn enum_work_areas(
    hmon: HMONITOR,
    _hdc: HDC,
    _rc: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let list = unsafe { &mut *(lparam.0 as *mut Vec<[i32; 4]>) };
    let mut mi = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if unsafe { GetMonitorInfoW(hmon, &mut mi) }.as_bool() {
        let w = mi.rcWork;
        list.push([w.left, w.top, w.right, w.bottom]);
    }
    BOOL(1)
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
    let notes = crate::changelog::next_notes()
        .ok_or_else(|| "No Unreleased or current notes.".to_string())?;
    refresh_palette();
    let fonts = Fonts::for_family(FontFamily::Exo2)
        .or_else(Fonts::load)
        .ok_or_else(|| "Need Exo 2 to paint What's new.".to_string())?;
    *UI.lock().unwrap() = Some(SettingsUi {
        host: HWND::default(),
        tab: Tab::App,
        last_widget: Tab::Standings,
        app_section: AppSection::Look,
        profile_section: ProfileSection::Overview,
        hover: None,
        focus: None,
        hits: Vec::new(),
        open_drop: None,
        drag: None,
        slide: None,
        sv_drag: None,
        scroll: 0.0,
        content_h: 0.0,
        scroll_max: 0.0,
        motos_list_scroll: 0.0,
        nav_scroll: 0.0,
        nav_content_h: 0.0,
        nav_top: 0.0,
        nav_bottom: 0.0,
        banner_dismissed: false,
        whats_new_open: true,
        pit_help_open: false,
        whats_new_scroll: 0.0,
        whats_new_scroll_max: 0.0,
        reply_id: None,
        reply_scroll: 0.0,
        reply_scroll_max: 0.0,
        drop_scroll: 0.0,
        drop_menu: None,
        bind_listen: false,
        review_filter: crate::review::ListFilter::All,
        review_stay_on_list: false,
        analyze_id: None,
        analyze_compare: -1,
        analyze_you_lap: -1,
        analyze_warmup: false,
        analyze_scrub: 0.0,
        analyze_zoom: 1.0,
        analyze_pan_x: 0.0,
        analyze_pan_z: 0.0,
        analyze_scrubbing: false,
        analyze_follow: false,
        map_drag: None,
        profile_all_time: true,
        profile_ranked_only: false,
        tracks_selected: None,
        tracks_zoom: 1.0,
        tracks_pan_x: 0.0,
        tracks_pan_z: 0.0,
        tracks_map_drag: None,
        clear_confirm: None,
        st_col_slides: ColSlides::new(),
        rel_col_slides: ColSlides::new(),
        col_list_y: 0.0,
        col_list_n: 0,
        col_list_kind: None,
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

/// Paint Motos list + Analyze to PNGs (demo sessions).
pub fn dump_motos_shots(
    dir: &std::path::Path,
) -> Result<(std::path::PathBuf, std::path::PathBuf), String> {
    refresh_palette();
    let fonts = Fonts::for_family(FontFamily::Exo2)
        .or_else(Fonts::load)
        .ok_or_else(|| "Need Exo 2 to paint Motos.".to_string())?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let tmp = std::env::temp_dir().join(format!("mxbo-motos-shots-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    crate::review::reset();
    crate::review::init(tmp);
    crate::config::update_config(|c| c.review = true);
    let id = crate::review::seed_demo()
        .ok_or_else(|| "Could not seed Motos demo sessions.".to_string())?;

    let list = dir.join("motos-list.png");
    paint_review_tab(&fonts, None, 1000, 720, &list)?;
    let analyze = dir.join("motos-analyze.png");
    paint_review_tab(&fonts, Some(id), 1000, 920, &analyze)?;
    crate::review::reset();
    Ok((list, analyze))
}

#[cfg(test)]
pub fn dump_review_pages(
    dir: &std::path::Path,
) -> Result<(i64, std::path::PathBuf, std::path::PathBuf), String> {
    let tmp = std::env::temp_dir().join(format!("mxbo-review-mocks-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    crate::review::reset();
    crate::review::init(tmp);
    crate::config::update_config(|c| c.review = true);
    let id = crate::review::seed_demo()
        .ok_or_else(|| "Could not seed Motos demo sessions.".to_string())?;
    refresh_palette();
    let fonts = Fonts::for_family(FontFamily::Exo2)
        .or_else(Fonts::load)
        .ok_or_else(|| "Need Exo 2 to paint Motos.".to_string())?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let list = dir.join("motos-list.png");
    paint_review_tab(&fonts, None, 1000, 720, &list)?;
    let analyze = dir.join("motos-analyze.png");
    paint_review_tab(&fonts, Some(id), 1000, 920, &analyze)?;
    crate::review::reset();
    Ok((id, list, analyze))
}

pub(crate) fn paint_review_tab(
    fonts: &Fonts,
    analyze_id: Option<i64>,
    w: u32,
    h: u32,
    path: &std::path::Path,
) -> Result<(), String> {
    *UI.lock().unwrap() = Some(SettingsUi {
        host: HWND::default(),
        tab: Tab::Profile,
        last_widget: Tab::Standings,
        app_section: AppSection::Look,
        profile_section: ProfileSection::Motos,
        hover: None,
        focus: None,
        hits: Vec::new(),
        open_drop: None,
        drag: None,
        slide: None,
        sv_drag: None,
        scroll: 0.0,
        content_h: 0.0,
        scroll_max: 0.0,
        motos_list_scroll: 0.0,
        nav_scroll: 0.0,
        nav_content_h: 0.0,
        nav_top: 0.0,
        nav_bottom: 0.0,
        banner_dismissed: false,
        whats_new_open: false,
        pit_help_open: false,
        whats_new_scroll: 0.0,
        whats_new_scroll_max: 0.0,
        reply_id: None,
        reply_scroll: 0.0,
        reply_scroll_max: 0.0,
        drop_scroll: 0.0,
        drop_menu: None,
        bind_listen: false,
        review_filter: crate::review::ListFilter::All,
        review_stay_on_list: false,
        analyze_id,
        analyze_compare: 0,
        analyze_you_lap: -1,
        analyze_warmup: false,
        analyze_scrub: if analyze_id.is_some() { 0.38 } else { 0.0 },
        analyze_zoom: 1.0,
        analyze_pan_x: 0.0,
        analyze_pan_z: 0.0,
        analyze_scrubbing: false,
        analyze_follow: false,
        map_drag: None,
        profile_all_time: true,
        profile_ranked_only: false,
        tracks_selected: None,
        tracks_zoom: 1.0,
        tracks_pan_x: 0.0,
        tracks_pan_z: 0.0,
        tracks_map_drag: None,
        clear_confirm: None,
        st_col_slides: ColSlides::new(),
        rel_col_slides: ColSlides::new(),
        col_list_y: 0.0,
        col_list_n: 0,
        col_list_kind: None,
    });
    let mut px =
        Pixmap::new(w, h).ok_or_else(|| "Could not allocate Review preview.".to_string())?;
    draw(&mut px, fonts, w as f32, h as f32);
    if analyze_id.is_some() {
        draw(&mut px, fonts, w as f32, h as f32);
    }
    *UI.lock().unwrap() = None;
    let png = px.encode_png().map_err(|e| e.to_string())?;
    std::fs::write(path, png).map_err(|e| e.to_string())?;
    Ok(())
}

pub(crate) fn dismiss_whats_new() {
    let ver = crate::update::current_version().to_string();
    crate::config::update_config(|c| c.whats_new_seen = ver);
    if let Some(ui) = UI.lock().unwrap().as_mut() {
        ui.whats_new_open = false;
        ui.whats_new_scroll = 0.0;
        ui.focus = None;
    }
}

pub(crate) fn open_pit_help() {
    if let Some(ui) = UI.lock().unwrap().as_mut() {
        ui.pit_help_open = true;
        ui.focus = Some(Hit::PitHelpDismiss);
        ui.open_drop = None;
        ui.drop_scroll = 0.0;
        ui.drop_menu = None;
        ui.bind_listen = false;
    }
}

pub(crate) fn dismiss_pit_help() {
    if let Some(ui) = UI.lock().unwrap().as_mut() {
        ui.pit_help_open = false;
        ui.focus = None;
    }
}

pub(crate) fn dismiss_reply() {
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

pub(crate) fn dismiss_clear_confirm() {
    if let Some(ui) = UI.lock().unwrap().as_mut() {
        ui.clear_confirm = None;
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
    let attached =
        fg_tid != 0 && fg_tid != this_tid && AttachThreadInput(this_tid, fg_tid, BOOL(1)).as_bool();
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
