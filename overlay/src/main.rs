#![windows_subsystem = "windows"]

mod compat;
mod config;
mod feedback;
mod layout;
mod record;
mod render;
mod settings;
mod plugin;
mod shm;
mod startup;
mod tray;
mod uninstall;
mod update;

use std::mem::size_of;
use std::ptr::null_mut;
use std::time::{Duration, Instant};

use tiny_skia::Pixmap;
use windows::core::w;
use windows::Win32::Foundation::{BOOL, CloseHandle, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    ClientToScreen, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject,
    AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS,
    HBITMAP, HDC, HGDIOBJ,
};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};
use windows::Win32::System::Threading::Sleep;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_F8, VK_F9};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateIconFromResourceEx, CreateWindowExW, DefWindowProcW, DispatchMessageW, GetClassNameW,
    GetClientRect, GetSystemMetrics, GetWindowThreadProcessId, IsWindow, LoadCursorW, LoadImageW,
    LookupIconIdFromDirectoryEx, PeekMessageW, PostQuitMessage, RegisterClassExW, SendMessageW,
    SetCursor, SetWindowPos, ShowWindow, TranslateMessage, UpdateLayeredWindow, HWND_TOPMOST,
    ICON_BIG, ICON_SMALL, IDC_ARROW, IMAGE_ICON, LR_DEFAULTCOLOR, MSG, PM_REMOVE, SM_CXSCREEN,
    SM_CYSCREEN, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_MINIMIZE, SW_SHOWMINNOACTIVE, SW_SHOWNOACTIVATE,
    ULW_ALPHA, WM_ACTIVATE, WM_CLOSE, WM_DESTROY, WM_QUIT, WM_SETCURSOR, WM_SETICON, WNDCLASSEXW,
    WS_CAPTION, WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_EX_TRANSPARENT, WS_MINIMIZEBOX, WS_OVERLAPPED, WS_POPUP, WS_SYSMENU,
};

use crate::render::Fonts;
use crate::shm::Shm;

static mut HOST: HWND = HWND(null_mut());

fn main() {
    let loaded = crate::config::HudConfig::load_file();
    if loaded.auto_update_on_launch && crate::update::apply_on_launch() {
        return;
    }
    let clock_log_path = {
        crate::record::init();
        crate::record::path()
    };
    match clock_log_path {
        Some(p) => mxbo_hud::set_status_hint(format!("Clock log: {}", p.display())),
        None => mxbo_hud::set_status_hint("Clock log failed — see AppData\\Local\\Holeshot HUD\\logs\\boot.txt"),
    }
    crate::plugin::sync();
    let family = loaded.font_family;
    *crate::config::CONFIG.lock().unwrap_or_else(|e| e.into_inner()) = loaded;
    let fonts = Fonts::for_family(family)
        .or_else(Fonts::load)
        .expect("need a HUD font (bundled or Windows\\Fonts)");
    unsafe { run(fonts, family) }
}

unsafe fn run(mut fonts: Fonts, mut font_family: crate::config::FontFamily) {
    let hinst = GetModuleHandleW(None).unwrap();
    let icon_bytes = include_bytes!("../icon.ico");
    let icon_big = load_app_icon(hinst, icon_bytes, 256);
    let icon_small = load_app_icon(hinst, icon_bytes, 32);
    let class = w!("MXBOOverlay");
    let wc = WNDCLASSEXW {
        cbSize: size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: Some(wndproc),
        hInstance: hinst.into(),
        lpszClassName: class,
        hIcon: icon_big,
        hIconSm: icon_small,
        hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
        ..Default::default()
    };
    RegisterClassExW(&wc);

    let host = CreateWindowExW(
        WS_EX_APPWINDOW,
        class,
        w!("Holeshot HUD — Settings"),
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
        80,
        80,
        800,
        700,
        None,
        None,
        hinst,
        None,
    )
    .expect("host window");
    unsafe { HOST = host; }
    crate::settings::attach(host);
    crate::startup::sync_from_config();
    apply_window_icons(host, icon_big, icon_small);
    crate::tray::add(host, icon_small);
    let start_minimized = std::env::args().any(|a| a == "--minimized");
    if start_minimized {
        let _ = ShowWindow(host, SW_SHOWMINNOACTIVE);
    } else {
        crate::settings::show(host);
    }

    let mut game = find_game_hwnd();
    let mut restart_hint = false;
    let mut compat_done = false;
    if let Some(path) = mxbikes_pid().and_then(compat::exe_path_for_pid) {
        restart_hint = compat::ensure_disable_fullscreen_optimizations(&path);
        compat_done = true;
    }
    let (mut x, mut y, mut w, mut h) = game
        .and_then(client_screen_rect)
        .unwrap_or_else(primary_screen);
    let mut hwnd = create_overlay(hinst, class, x, y, w, h);

    let mut dib = Dib::new(w, h);
    let mut pixmap = Pixmap::new(w as u32, h as u32).unwrap();
    dib.present(hwnd, w, h, x, y);
    let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);

    let mut shm = Shm::open();
    let mut last_snap = None;
    let mut freq = 0i64;
    QueryPerformanceFrequency(&mut freq).ok();
    let mut zfix = compat::FullscreenFix::new();
    let mut editor = crate::layout::Editor::default();
    let mut f8_was = false;
    let mut f9_was = false;
    let mut next_game_scan = Instant::now();
    let mut placed = false;

    let mut msg = MSG::default();
    loop {
        while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
            if msg.message == WM_QUIT || crate::update::should_quit() {
                return;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        if !IsWindow(hwnd).as_bool() {
            game = find_game_hwnd();
            let r = game
                .and_then(client_screen_rect)
                .unwrap_or_else(primary_screen);
            x = r.0;
            y = r.1;
            w = r.2;
            h = r.3;
            hwnd = create_overlay(hinst, class, x, y, w, h);
            dib = Dib::new(w, h);
            pixmap = Pixmap::new(w as u32, h as u32).unwrap();
            placed = false;
        }

        if game.is_none() || Instant::now() >= next_game_scan {
            let next_game = find_game_hwnd();
            if next_game != game {
                game = next_game;
                placed = false;
            }
            next_game_scan = Instant::now() + Duration::from_millis(500);
            if game.is_none() {
                crate::plugin::retry_if_needed();
            }
        }
        if !compat_done {
            if let Some(path) = mxbikes_pid().and_then(compat::exe_path_for_pid) {
                restart_hint = compat::ensure_disable_fullscreen_optimizations(&path);
                compat_done = true;
            }
        }
        let overlay_on = zfix.keep_overlay_above(hwnd, game);
        if overlay_on {
            crate::settings::keep_above_overlay(host);
        }
        if let Some(g) = game {
            if let Some((nx, ny, nw, nh)) = client_screen_rect(g) {
                if nw > 64 && nh > 64 && ((nx - x).abs() > 2 || (ny - y).abs() > 2 || (nw - w).abs() > 2 || (nh - h).abs() > 2)
                {
                    x = nx;
                    y = ny;
                    w = nw;
                    h = nh;
                    dib = Dib::new(w, h);
                    pixmap = Pixmap::new(w as u32, h as u32).unwrap();
                    placed = false;
                }
            }
        }

        if overlay_on && !placed {
            let _ = SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                x,
                y,
                w,
                h,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
            placed = true;
        }

        let layout_on = overlay_on && crate::layout::Editor::ctrl_down();
        zfix.set_layout_mode(hwnd, layout_on);
        if layout_on {
            let cur = LoadCursorW(None, IDC_ARROW).unwrap_or_default();
            let _ = SetCursor(cur);
        }

        let f8 = unsafe { GetAsyncKeyState(VK_F8.0 as i32) < 0 };
        if f8 && !f8_was {
            crate::settings::show(host);
        }
        f8_was = f8;
        let f9 = unsafe { GetAsyncKeyState(VK_F9.0 as i32) < 0 };
        if f9 && !f9_was {
            crate::record::rotate();
            match crate::record::path() {
                Some(p) => mxbo_hud::set_status_hint(format!("Clock log: {}", p.display())),
                None => mxbo_hud::set_status_hint("Clock log failed — see AppData\\Local\\Holeshot HUD\\logs\\boot.txt"),
            }
        }
        f9_was = f9;

        if shm.is_none() {
            shm = Shm::open();
        }
        if let Some(s) = shm.as_ref().and_then(|s| s.read()) {
            last_snap = Some(s);
        }
        let mut snap = last_snap.clone();
        if crate::update::should_quit() {
            return;
        }
        let mut cfg = crate::config::with_config(|c| c.clone());
        if cfg.font_family != font_family {
            if let Some(next) = Fonts::for_family(cfg.font_family) {
                fonts = next;
                font_family = cfg.font_family;
            }
        }
        if let Some(ref mut s) = snap {
            cfg.apply_to_snapshot(s);
        }
        editor.tick(hwnd, x, y, w, h, snap.as_ref(), &cfg);
        if let Some(ref mut s) = snap {
            editor.apply(s);
        }
        editor.apply_cfg(&mut cfg);
        let raw_age = snap
            .as_ref()
            .map(|s| qpc_age(s.tick_qpc, freq))
            .unwrap_or(999.0);
        let age = raw_age.clamp(0.0, 0.08);
        let live = raw_age < 2.5;
        let hud = if live || layout_on { snap.as_ref() } else { None };
        if live {
            if let Some(s) = snap.as_ref() {
                crate::record::tick(s);
            }
        }

        let frame_start = Instant::now();
        render::draw(
            &mut pixmap,
            &fonts,
            hud,
            &cfg,
            w as u32,
            h as u32,
            age,
            restart_hint,
            layout_on,
        );
        dib.blit_premul_bgra(pixmap.data());
        dib.present(hwnd, w, h, x, y);
        crate::settings::paint(&fonts);
        let target = Duration::from_millis(16);
        if let Some(remain) = target.checked_sub(frame_start.elapsed()) {
            Sleep(remain.as_millis() as u32);
        }
    }
}

fn qpc_age(tick: u64, freq: i64) -> f32 {
    if freq <= 0 || tick == 0 {
        return 0.0;
    }
    let mut now = 0i64;
    unsafe {
        QueryPerformanceCounter(&mut now).ok();
    }
    ((now as f64 - tick as f64) / freq as f64) as f32
}

unsafe fn load_app_icon(
    hinst: windows::Win32::Foundation::HMODULE,
    bytes: &[u8],
    size: i32,
) -> windows::Win32::UI::WindowsAndMessaging::HICON {
    if let Ok(handle) = LoadImageW(
        hinst,
        windows::core::PCWSTR(1usize as *const u16),
        IMAGE_ICON,
        size,
        size,
        LR_DEFAULTCOLOR,
    ) {
        if !handle.is_invalid() {
            return windows::Win32::UI::WindowsAndMessaging::HICON(handle.0);
        }
    }
    icon_from_ico(bytes, size)
}

unsafe fn apply_window_icons(
    hwnd: HWND,
    big: windows::Win32::UI::WindowsAndMessaging::HICON,
    small: windows::Win32::UI::WindowsAndMessaging::HICON,
) {
    let _ = SendMessageW(hwnd, WM_SETICON, WPARAM(ICON_BIG as usize), LPARAM(big.0 as isize));
    let _ = SendMessageW(
        hwnd,
        WM_SETICON,
        WPARAM(ICON_SMALL as usize),
        LPARAM(small.0 as isize),
    );
}

unsafe fn icon_from_ico(bytes: &[u8], size: i32) -> windows::Win32::UI::WindowsAndMessaging::HICON {
    let offset = LookupIconIdFromDirectoryEx(bytes.as_ptr(), true, size, size, LR_DEFAULTCOLOR);
    if offset <= 0 || offset as usize >= bytes.len() {
        return Default::default();
    }
    CreateIconFromResourceEx(
        &bytes[offset as usize..],
        true,
        0x0003_0000,
        size,
        size,
        LR_DEFAULTCOLOR,
    )
    .unwrap_or_default()
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if hwnd == HOST {
        if msg == crate::tray::callback_msg() {
            crate::tray::on_callback(lp);
            return LRESULT(0);
        }
        if msg != 0 && msg == crate::tray::taskbar_created_msg() {
            crate::tray::readd();
            return LRESULT(0);
        }
        if msg == WM_ACTIVATE && (wp.0 as u32 & 0xFFFF) != 0 {
            crate::settings::show(hwnd);
        }
        if crate::settings::handle_message(msg, wp, lp) {
            return LRESULT(0);
        }
        if msg == WM_CLOSE {
            if crate::config::with_config(|c| c.minimize_on_close) {
                let _ = ShowWindow(hwnd, SW_MINIMIZE);
                return LRESULT(0);
            }
            crate::config::update_config(|_| {});
            crate::tray::remove();
            PostQuitMessage(0);
            return LRESULT(0);
        }
        if msg == WM_DESTROY {
            crate::config::update_config(|_| {});
            crate::tray::remove();
            PostQuitMessage(0);
            return LRESULT(0);
        }
    } else if msg == WM_SETCURSOR {
        let cur = LoadCursorW(None, IDC_ARROW).unwrap_or_default();
        let _ = SetCursor(cur);
        return LRESULT(1);
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}

unsafe fn create_overlay(
    hinst: windows::Win32::Foundation::HMODULE,
    class: windows::core::PCWSTR,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) -> HWND {
    let ex = WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW;
    if let Some(hwnd) = compat::try_create_window_in_band(
        ex.0,
        class,
        w!("Holeshot HUD"),
        WS_POPUP.0,
        x,
        y,
        w,
        h,
        hinst.into(),
    ) {
        return hwnd;
    }
    CreateWindowExW(
        ex,
        class,
        w!("Holeshot HUD"),
        WS_POPUP,
        x,
        y,
        w,
        h,
        HWND::default(),
        None,
        hinst,
        None,
    )
    .expect("overlay window")
}

fn mxbikes_pid() -> Option<u32> {
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).ok()?;
        let mut pe = PROCESSENTRY32W {
            dwSize: size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut found = None;
        if Process32FirstW(snap, &mut pe).is_ok() {
            loop {
                let len = pe
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(pe.szExeFile.len());
                let name = String::from_utf16_lossy(&pe.szExeFile[..len]).to_lowercase();
                if name == "mxbikes.exe" {
                    found = Some(pe.th32ProcessID);
                    break;
                }
                if Process32NextW(snap, &mut pe).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snap);
        found
    }
}

fn find_game_hwnd() -> Option<HWND> {
    let pid = mxbikes_pid()?;
    struct St {
        pid: u32,
        best: HWND,
        area: i32,
    }
    unsafe extern "system" fn cb(hwnd: HWND, lp: LPARAM) -> BOOL {
        use windows::Win32::UI::WindowsAndMessaging::{GetWindowRect, IsWindowVisible};
        let st = unsafe { &mut *(lp.0 as *mut St) };
        if !IsWindowVisible(hwnd).as_bool() {
            return true.into();
        }
        let mut wpid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut wpid));
        if wpid != st.pid {
            return true.into();
        }
        let mut name = [0u16; 64];
        let n = GetClassNameW(hwnd, &mut name);
        if n > 0 {
            let class = String::from_utf16_lossy(&name[..n as usize]);
            if class == "MXBOOverlay" {
                return true.into();
            }
        }
        let mut r = RECT::default();
        let _ = GetWindowRect(hwnd, &mut r);
        let area = (r.right - r.left).saturating_mul(r.bottom - r.top);
        if area > st.area && (r.right - r.left) > 64 && (r.bottom - r.top) > 64 {
            st.area = area;
            st.best = hwnd;
        }
        true.into()
    }
    let mut st = St {
        pid,
        best: HWND::default(),
        area: 0,
    };
    unsafe {
        let _ = windows::Win32::UI::WindowsAndMessaging::EnumWindows(
            Some(cb),
            LPARAM(&mut st as *mut St as isize),
        );
    }
    if st.best.0.is_null() {
        None
    } else {
        Some(st.best)
    }
}

fn client_screen_rect(hwnd: HWND) -> Option<(i32, i32, i32, i32)> {
    unsafe {
        let mut cr = RECT::default();
        GetClientRect(hwnd, &mut cr).ok()?;
        let mut pt = POINT { x: 0, y: 0 };
        let _ = ClientToScreen(hwnd, &mut pt);
        let w = cr.right - cr.left;
        let h = cr.bottom - cr.top;
        if w < 64 || h < 64 {
            return None;
        }
        Some((pt.x, pt.y, w, h))
    }
}

fn primary_screen() -> (i32, i32, i32, i32) {
    unsafe {
        (
            0,
            0,
            GetSystemMetrics(SM_CXSCREEN).max(800),
            GetSystemMetrics(SM_CYSCREEN).max(600),
        )
    }
}

struct Dib {
    hdc: HDC,
    bmp: HBITMAP,
    bits: *mut u8,
    w: i32,
    h: i32,
}

impl Dib {
    fn new(w: i32, h: i32) -> Self {
        unsafe {
            let hdc = CreateCompatibleDC(None);
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w,
                    biHeight: -h,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0 as u32,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits = null_mut();
            let bmp = CreateDIBSection(hdc, &info, DIB_RGB_COLORS, &mut bits, None, 0).unwrap();
            SelectObject(hdc, HGDIOBJ(bmp.0));
            Self {
                hdc,
                bmp,
                bits: bits as *mut u8,
                w,
                h,
            }
        }
    }

    fn blit_premul_bgra(&self, rgba: &[u8]) {
        let n = (self.w * self.h * 4) as usize;
        unsafe {
            let dst = std::slice::from_raw_parts_mut(self.bits, n);
            let mut i = 0;
            while i + 16 <= n {
                dst[i] = rgba[i + 2];
                dst[i + 1] = rgba[i + 1];
                dst[i + 2] = rgba[i];
                dst[i + 3] = rgba[i + 3];
                dst[i + 4] = rgba[i + 6];
                dst[i + 5] = rgba[i + 5];
                dst[i + 6] = rgba[i + 4];
                dst[i + 7] = rgba[i + 7];
                dst[i + 8] = rgba[i + 10];
                dst[i + 9] = rgba[i + 9];
                dst[i + 10] = rgba[i + 8];
                dst[i + 11] = rgba[i + 11];
                dst[i + 12] = rgba[i + 14];
                dst[i + 13] = rgba[i + 13];
                dst[i + 14] = rgba[i + 12];
                dst[i + 15] = rgba[i + 15];
                i += 16;
            }
            while i + 4 <= n {
                dst[i] = rgba[i + 2];
                dst[i + 1] = rgba[i + 1];
                dst[i + 2] = rgba[i];
                dst[i + 3] = rgba[i + 3];
                i += 4;
            }
        }
    }

    fn present(&self, hwnd: HWND, w: i32, h: i32, x: i32, y: i32) {
        unsafe {
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            let src = POINT { x: 0, y: 0 };
            let dst = POINT { x, y };
            let size = windows::Win32::Foundation::SIZE { cx: w, cy: h };
            let _ = UpdateLayeredWindow(
                hwnd,
                None,
                Some(&dst),
                Some(&size),
                self.hdc,
                Some(&src),
                windows::Win32::Foundation::COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );
        }
    }
}

impl Drop for Dib {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(HGDIOBJ(self.bmp.0));
            let _ = DeleteDC(self.hdc);
        }
    }
}
