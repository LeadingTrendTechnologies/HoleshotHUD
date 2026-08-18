use std::mem::size_of;
use std::sync::atomic::{AtomicIsize, AtomicU32, Ordering};

use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIM_ADD, NIM_DELETE,
    NIM_SETVERSION, NIN_SELECT, NOTIFYICONDATAW, NOTIFYICON_VERSION_4,
};
use windows::Win32::UI::WindowsAndMessaging::{
    RegisterWindowMessageW, HICON, WM_APP, WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_RBUTTONUP,
};

const ID: u32 = 1;
const WM_TRAY: u32 = WM_APP + 1;
const NIN_KEYSELECT: u32 = NIN_SELECT | 1;

static HOST: AtomicIsize = AtomicIsize::new(0);
static ICON: AtomicIsize = AtomicIsize::new(0);
static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);

pub fn callback_msg() -> u32 {
    WM_TRAY
}

pub fn taskbar_created_msg() -> u32 {
    TASKBAR_CREATED.load(Ordering::Relaxed)
}

pub fn add(host: HWND, icon: HICON) {
    HOST.store(host.0 as isize, Ordering::SeqCst);
    ICON.store(icon.0 as isize, Ordering::SeqCst);
    unsafe {
        if TASKBAR_CREATED.load(Ordering::Relaxed) == 0 {
            TASKBAR_CREATED.store(RegisterWindowMessageW(w!("TaskbarCreated")), Ordering::Relaxed);
        }
        let nid = data();
        let _ = Shell_NotifyIconW(NIM_ADD, &nid);
        let mut ver = nid;
        ver.Anonymous.uVersion = NOTIFYICON_VERSION_4;
        let _ = Shell_NotifyIconW(NIM_SETVERSION, &ver);
    }
}

pub fn remove() {
    if HOST.load(Ordering::SeqCst) == 0 {
        return;
    }
    unsafe {
        let _ = Shell_NotifyIconW(NIM_DELETE, &data());
    }
    HOST.store(0, Ordering::SeqCst);
}

pub fn readd() {
    if HOST.load(Ordering::SeqCst) == 0 {
        return;
    }
    unsafe {
        let nid = data();
        let _ = Shell_NotifyIconW(NIM_ADD, &nid);
        let mut ver = nid;
        ver.Anonymous.uVersion = NOTIFYICON_VERSION_4;
        let _ = Shell_NotifyIconW(NIM_SETVERSION, &ver);
    }
}

pub fn on_callback(lp: LPARAM) {
    let event = lp.0 as u32 & 0xFFFF;
    if matches!(
        event,
        WM_LBUTTONUP | WM_LBUTTONDBLCLK | WM_RBUTTONUP | NIN_SELECT | NIN_KEYSELECT
    ) {
        let host = HWND(HOST.load(Ordering::SeqCst) as *mut _);
        if !host.0.is_null() {
            crate::settings::show(host);
        }
    }
}

fn data() -> NOTIFYICONDATAW {
    let mut nid = NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: HWND(HOST.load(Ordering::SeqCst) as *mut _),
        uID: ID,
        uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP | NIF_SHOWTIP,
        uCallbackMessage: WM_TRAY,
        hIcon: HICON(ICON.load(Ordering::SeqCst) as *mut _),
        ..Default::default()
    };
    let tip: Vec<u16> = "Holeshot HUD\0".encode_utf16().collect();
    let n = tip.len().min(nid.szTip.len());
    nid.szTip[..n].copy_from_slice(&tip[..n]);
    nid
}
