#![windows_subsystem = "windows"]

mod compat;
mod config;
mod layout;
mod render;
mod settings;
mod shm;

use std::mem::size_of;
use std::ptr::null_mut;

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
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_F8};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateIconFromResourceEx, CreateWindowExW, DefWindowProcW, DispatchMessageW, GetClassNameW,
    GetClientRect, GetSystemMetrics, GetWindowThreadProcessId, IsWindow, LoadCursorW,
    LookupIconIdFromDirectoryEx, PeekMessageW, PostQuitMessage, RegisterClassExW, SetCursor,
    SetWindowPos, ShowWindow, TranslateMessage, UpdateLayeredWindow, HWND_TOPMOST, IDC_ARROW,
    LR_DEFAULTCOLOR, MSG, PM_REMOVE, SM_CXSCREEN, SM_CYSCREEN, SWP_NOACTIVATE, SWP_SHOWWINDOW,
    SW_SHOWMINNOACTIVE, SW_SHOWNOACTIVATE, ULW_ALPHA, WM_CLOSE, WM_DESTROY, WM_QUIT, WM_SETCURSOR,
    WNDCLASSEXW, WS_CAPTION, WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_MINIMIZEBOX, WS_OVERLAPPED, WS_POPUP, WS_SYSMENU,
};

use crate::render::Fonts;
use crate::shm::Shm;

static mut HOST: HWND = HWND(null_mut());

fn main() {
    let fonts = Fonts::load().expect("need Segoe UI / Arial / Tahoma in Windows\\Fonts");
    unsafe { run(fonts) }
}

unsafe fn run(fonts: Fonts) {
    let hinst = GetModuleHandleW(None).unwrap();
    let icon_bytes = include_bytes!("../icon.ico");
    let icon_big = icon_from_ico(icon_bytes, 32);
    let icon_small = icon_from_ico(icon_bytes, 16);
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
        w!("MXBO Overlay — Settings"),
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
        80,
        80,
        720,
        640,
        None,
        None,
        hinst,
        None,
    )
    .expect("host window");
    unsafe { HOST = host; }
    {
        let loaded = crate::config::HudConfig::load_file();
        *crate::config::CONFIG.lock().unwrap_or_else(|e| e.into_inner()) = loaded;
    }
    crate::settings::attach(host);
    let _ = ShowWindow(host, SW_SHOWMINNOACTIVE);

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
    let mut freq = 0i64;
    QueryPerformanceFrequency(&mut freq).ok();
    let zfix = compat::FullscreenFix::new();
    let mut editor = crate::layout::Editor::default();
    let mut f8_was = false;

    let mut msg = MSG::default();
    loop {
        while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
            if msg.message == WM_QUIT {
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
        }

        let next_game = find_game_hwnd();
        if next_game != game {
            game = next_game;
        }
        if !compat_done {
            if let Some(path) = mxbikes_pid().and_then(compat::exe_path_for_pid) {
                restart_hint = compat::ensure_disable_fullscreen_optimizations(&path);
                compat_done = true;
            }
        }
        let overlay_on = zfix.keep_overlay_above(hwnd, game);
        if let Some(g) = game {
            if let Some((nx, ny, nw, nh)) = client_screen_rect(g) {
                if (nx, ny, nw, nh) != (x, y, w, h) && nw > 64 && nh > 64 {
                    x = nx;
                    y = ny;
                    w = nw;
                    h = nh;
                    dib = Dib::new(w, h);
                    pixmap = Pixmap::new(w as u32, h as u32).unwrap();
                }
            }
        }

        if overlay_on {
            let _ = SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                x,
                y,
                w,
                h,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
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

        if shm.is_none() {
            shm = Shm::open();
        }
        let mut snap = shm.as_ref().and_then(|s| s.read());
        let mut cfg = crate::config::with_config(|c| c.clone());
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
        let live = raw_age < 0.6;
        let hud = if live || layout_on { snap.as_ref() } else { None };

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
        Sleep(8);
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
        if crate::settings::handle_message(msg, wp, lp) {
            return LRESULT(0);
        }
        if msg == WM_CLOSE || msg == WM_DESTROY {
            crate::config::update_config(|_| {});
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
        w!("mxbo overlay"),
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
        w!("mxbo overlay"),
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
            for i in (0..n).step_by(4) {
                let r = rgba[i];
                let g = rgba[i + 1];
                let b = rgba[i + 2];
                let a = rgba[i + 3];
                dst[i] = b;
                dst[i + 1] = g;
                dst[i + 2] = r;
                dst[i + 3] = a;
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
