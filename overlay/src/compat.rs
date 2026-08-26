use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use windows::core::{w, PCSTR, PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegQueryValueExW, RegSetValueExW, HKEY_CURRENT_USER, KEY_READ,
    KEY_WRITE, REG_SZ,
};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Shell::{ABM_WINDOWPOSCHANGED, APPBARDATA, SHAppBarMessage};
use windows::Win32::UI::WindowsAndMessaging::{
    ClipCursor, FindWindowExW, FindWindowW, GetForegroundWindow, GetWindowLongPtrW, GetWindowRect,
    GetWindowThreadProcessId, IsIconic, IsWindowVisible, SetWindowLongPtrW, SetWindowPos, ShowWindow,
    GWL_EXSTYLE, HWND_NOTOPMOST, HWND_TOPMOST, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, SWP_SHOWWINDOW, SW_HIDE, SW_SHOWNOACTIVATE, SW_SHOWNORMAL, WS_EX_TRANSPARENT,
};

const FLAG: &str = "DISABLEDXMAXIMIZEDWINDOWEDMODE";
const ZBID_IMMERSIVE_NOTIFICATION: u32 = 4;

static UNCLIP: AtomicBool = AtomicBool::new(false);
static BG_RUN: AtomicBool = AtomicBool::new(true);

fn spawn_unclip_thread() {
    thread::spawn(|| {
        while BG_RUN.load(Ordering::Relaxed) {
            if UNCLIP.load(Ordering::Relaxed) {
                unsafe {
                    let _ = ClipCursor(None);
                }
                thread::sleep(Duration::from_millis(1));
            } else {
                thread::sleep(Duration::from_millis(16));
            }
        }
    });
}

pub fn stop_background_threads() {
    BG_RUN.store(false, Ordering::Relaxed);
    UNCLIP.store(false, Ordering::Relaxed);
}

type CreateWindowInBandFn = unsafe extern "system" fn(
    dwexstyle: u32,
    lpclassname: PCWSTR,
    lpwindowname: PCWSTR,
    dwstyle: u32,
    x: i32,
    y: i32,
    nwidth: i32,
    nheight: i32,
    hwndparent: HWND,
    hmenu: isize,
    hinstance: windows::Win32::Foundation::HINSTANCE,
    lpparam: *const core::ffi::c_void,
    dwband: u32,
) -> HWND;

pub fn exe_path_for_pid(pid: u32) -> Option<String> {
    unsafe {
        let proc = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 520];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(
            proc,
            PROCESS_NAME_FORMAT(0),
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        )
        .is_ok();
        let _ = CloseHandle(proc);
        if !ok || len == 0 {
            return None;
        }
        Some(String::from_utf16_lossy(&buf[..len as usize]))
    }
}

/// Returns true if the flag was missing and we just wrote it (game must restart).
pub fn ensure_disable_fullscreen_optimizations(exe_path: &str) -> bool {
    let mut name: Vec<u16> = exe_path.encode_utf16().collect();
    name.push(0);
    unsafe {
        let mut key = Default::default();
        if RegCreateKeyExW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows NT\\CurrentVersion\\AppCompatFlags\\Layers"),
            0,
            None,
            Default::default(),
            KEY_READ | KEY_WRITE,
            None,
            &mut key,
            None,
        )
        .is_err()
        {
            return false;
        }
        let mut buf = [0u16; 512];
        let mut bytes = (buf.len() * 2) as u32;
        let mut ty = REG_SZ;
        let existing = if RegQueryValueExW(
            key,
            PCWSTR(name.as_ptr()),
            None,
            Some(&mut ty),
            Some(buf.as_mut_ptr() as *mut u8),
            Some(&mut bytes),
        )
        .is_ok()
        {
            let n = (bytes as usize / 2).saturating_sub(1).min(buf.len());
            String::from_utf16_lossy(&buf[..n])
        } else {
            String::new()
        };
        if existing.to_uppercase().contains(FLAG) {
            let _ = RegCloseKey(key);
            return false;
        }
        let mut next = if existing.trim().is_empty() {
            format!("~ {FLAG}")
        } else {
            format!("{} {FLAG}", existing.trim())
        };
        if !next.starts_with('~') {
            next = format!("~ {next}");
        }
        let mut wide: Vec<u16> = next.encode_utf16().collect();
        wide.push(0);
        let _ = RegSetValueExW(
            key,
            PCWSTR(name.as_ptr()),
            0,
            REG_SZ,
            Some(std::slice::from_raw_parts(
                wide.as_ptr() as *const u8,
                wide.len() * size_of::<u16>(),
            )),
        );
        let _ = RegCloseKey(key);
        true
    }
}

pub unsafe fn try_create_window_in_band(
    ex: u32,
    class: PCWSTR,
    title: PCWSTR,
    style: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    hinst: windows::Win32::Foundation::HINSTANCE,
) -> Option<HWND> {
    let user32 = GetModuleHandleW(w!("user32")).ok()?;
    let proc = GetProcAddress(user32, PCSTR::from_raw(b"CreateWindowInBand\0".as_ptr()))?;
    let create: CreateWindowInBandFn = std::mem::transmute(proc);
    let hwnd = create(
        ex,
        class,
        title,
        style,
        x,
        y,
        w,
        h,
        HWND::default(),
        0,
        hinst,
        std::ptr::null(),
        ZBID_IMMERSIVE_NOTIFICATION,
    );
    if hwnd.0.is_null() {
        None
    } else {
        Some(hwnd)
    }
}

type SetWindowBandFn = unsafe extern "system" fn(HWND, HWND, u32) -> i32;

pub struct FullscreenFix {
    set_window_band: Option<SetWindowBandFn>,
    last_playing: bool,
    hide_after: Option<Instant>,
    last_raise: Instant,
    layout_on: bool,
}

impl FullscreenFix {
    pub fn new() -> Self {
        unsafe {
            let set_window_band = (|| {
                let user32 = GetModuleHandleW(w!("user32")).ok()?;
                let proc = GetProcAddress(user32, PCSTR::from_raw(b"SetWindowBand\0".as_ptr()))?;
                Some(std::mem::transmute::<_, SetWindowBandFn>(proc))
            })();
            spawn_unclip_thread();
            show_taskbars();
            Self {
                set_window_band,
                last_playing: false,
                hide_after: None,
                last_raise: Instant::now() - Duration::from_secs(10),
                layout_on: false,
            }
        }
    }

    pub fn set_layout_mode(&mut self, overlay: HWND, on: bool) {
        if self.layout_on == on {
            return;
        }
        self.layout_on = on;
        UNCLIP.store(on, Ordering::Relaxed);
        unsafe {
            set_click_through(overlay, !on);
            if on {
                let _ = ClipCursor(None);
            }
        }
    }

    pub fn keep_overlay_above(&mut self, overlay: HWND, game: Option<HWND>, settings: HWND) -> bool {
        unsafe {
            // Settings steals foreground from the game; keep the HUD up so riders can
            // see widget tweaks live. Alt-tab away from both still hides it.
            let playing = game.is_some_and(|g| game_is_foreground(g))
                || (game.is_some() && window_is_foreground(settings));
            let now = Instant::now();
            if playing {
                self.hide_after = None;
                let became = !self.last_playing;
                self.last_playing = true;
                if became || now.duration_since(self.last_raise) > Duration::from_secs(2) {
                    if let Some(game) = game {
                        hide_taskbars();
                        if became {
                            keep_just_shy_of_fullscreen(game);
                        }
                    }
                    if let Some(set_band) = self.set_window_band {
                        for band in [ZBID_IMMERSIVE_NOTIFICATION, 2u32, 16u32] {
                            if set_band(overlay, HWND::default(), band) != 0 {
                                break;
                            }
                        }
                    }
                    let _ = ShowWindow(overlay, SW_SHOWNOACTIVATE);
                    let _ = SetWindowPos(
                        overlay,
                        HWND_TOPMOST,
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                    );
                    self.last_raise = now;
                }
                true
            } else {
                let game_gone = game.is_none();
                if self.last_playing {
                    self.last_playing = false;
                    if game_gone {
                        restore_desktop(overlay);
                        self.hide_after = None;
                        return false;
                    }
                    self.hide_after = Some(now + Duration::from_millis(1500));
                    return true;
                }
                if let Some(until) = self.hide_after {
                    if now < until && !game_gone {
                        return true;
                    }
                    restore_desktop(overlay);
                    self.hide_after = None;
                }
                false
            }
        }
    }
}

impl Drop for FullscreenFix {
    fn drop(&mut self) {
        show_taskbars();
    }
}

unsafe fn restore_desktop(overlay: HWND) {
    show_taskbars();
    let _ = ShowWindow(overlay, SW_HIDE);
}

fn window_is_foreground(hwnd: HWND) -> bool {
    unsafe {
        if hwnd.0.is_null() || IsIconic(hwnd).as_bool() || !IsWindowVisible(hwnd).as_bool() {
            return false;
        }
        GetForegroundWindow() == hwnd
    }
}

fn game_is_foreground(game: HWND) -> bool {
    unsafe {
        let fg = GetForegroundWindow();
        if fg == game {
            return true;
        }
        let mut game_pid = 0u32;
        let mut fg_pid = 0u32;
        GetWindowThreadProcessId(game, Some(&mut game_pid));
        GetWindowThreadProcessId(fg, Some(&mut fg_pid));
        game_pid != 0 && game_pid == fg_pid
    }
}

fn hide_taskbars() {
    set_taskbars_visible(false);
}

fn show_taskbars() {
    set_taskbars_visible(true);
}

fn set_taskbars_visible(show: bool) {
    unsafe {
        for_each_taskbar(|hwnd| {
            if show {
                restore_taskbar(hwnd);
            } else {
                let _ = ShowWindow(hwnd, SW_HIDE);
            }
        });
    }
}

unsafe fn for_each_taskbar(mut f: impl FnMut(HWND)) {
    if let Ok(hwnd) = FindWindowW(w!("Shell_TrayWnd"), None) {
        if !hwnd.is_invalid() {
            f(hwnd);
        }
    }
    let mut prev = HWND::default();
    loop {
        let hwnd = FindWindowExW(None, prev, w!("Shell_SecondaryTrayWnd"), None).unwrap_or_default();
        if hwnd.is_invalid() || hwnd.0.is_null() {
            break;
        }
        f(hwnd);
        prev = hwnd;
    }
}

unsafe fn restore_taskbar(hwnd: HWND) {
    let _ = ShowWindow(hwnd, SW_SHOWNORMAL);
    let _ = SetWindowPos(
        hwnd,
        HWND_TOPMOST,
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW | SWP_NOACTIVATE,
    );
    let mut abd = APPBARDATA {
        cbSize: std::mem::size_of::<APPBARDATA>() as u32,
        hWnd: hwnd,
        ..Default::default()
    };
    let _ = SHAppBarMessage(ABM_WINDOWPOSCHANGED, &mut abd);
}

unsafe fn set_click_through(hwnd: HWND, through: bool) {
    let mut ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
    if through {
        ex |= WS_EX_TRANSPARENT.0 as isize;
    } else {
        ex &= !(WS_EX_TRANSPARENT.0 as isize);
    }
    SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex);
    let _ = SetWindowPos(
        hwnd,
        HWND_TOPMOST,
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
    );
}

unsafe fn keep_just_shy_of_fullscreen(game: HWND) {
    let mut wr = RECT::default();
    let _ = GetWindowRect(game, &mut wr);
    let mon = MonitorFromWindow(game, MONITOR_DEFAULTTONEAREST);
    let mut mi = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !GetMonitorInfoW(mon, &mut mi).as_bool() {
        return;
    }
    let mr = mi.rcMonitor;
    let w = mr.right - mr.left;
    let h = (mr.bottom - mr.top - 1).max(600);
    if wr.left != mr.left || wr.top != mr.top || wr.right - wr.left != w || wr.bottom - wr.top != h
    {
        let _ = SetWindowPos(game, HWND_NOTOPMOST, mr.left, mr.top, w, h, SWP_NOACTIVATE);
    }
}
