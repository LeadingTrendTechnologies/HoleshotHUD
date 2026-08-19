use std::mem::size_of;
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicIsize, Ordering};
use std::time::Duration;

use windows::core::w;
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, ERROR_SUCCESS, HANDLE,
};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
    KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
};
use windows::Win32::System::Threading::CreateMutexW;

const RUN_KEY: windows::core::PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const RUN_HUD: windows::core::PCWSTR = w!("Holeshot HUD");
const RUN_GAME: windows::core::PCWSTR = w!("Holeshot HUD game");
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const DETACHED_PROCESS: u32 = 0x0000_0008;
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

static HUD_MUTEX: AtomicIsize = AtomicIsize::new(0);
static WAIT_MUTEX: AtomicIsize = AtomicIsize::new(0);

pub fn set_enabled(on: bool) {
    if on {
        sync_from_config();
    } else {
        remove_value(RUN_HUD);
        remove_value(RUN_GAME);
    }
}

pub fn sync_from_config() {
    let (start, follow) = crate::config::with_config(|c| (c.start_with_windows, c.open_with_game));
    set_run_value(RUN_HUD, start, "--minimized");
    set_run_value(RUN_GAME, follow, "--wait-for-game");
}

pub fn wait_for_mx_bikes() {
    if !claim_named(w!("Local\\HoleshotHUD-wait"), &WAIT_MUTEX) {
        std::process::exit(0);
    }
    loop {
        if mx_bikes_pid().is_some() {
            break;
        }
        if !mxbo_hud::config::HudConfig::load_file().open_with_game {
            release(&WAIT_MUTEX);
            std::process::exit(0);
        }
        std::thread::sleep(Duration::from_millis(400));
    }
    release(&WAIT_MUTEX);
}

pub fn spawn_game_waiter() {
    if mutex_held(w!("Local\\HoleshotHUD-wait")) {
        return;
    }
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let _ = Command::new(exe)
        .arg("--wait-for-game")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(DETACHED_PROCESS | CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP)
        .spawn();
}

pub fn claim_hud_instance() -> bool {
    claim_named(w!("Local\\HoleshotHUD"), &HUD_MUTEX)
}

pub fn mx_bikes_pid() -> Option<u32> {
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

fn set_run_value(name: windows::core::PCWSTR, on: bool, flag: &str) {
    if on {
        if let Some(cmd) = exe_cmd(flag) {
            write_value(name, &cmd);
        }
    } else {
        remove_value(name);
    }
}

fn exe_cmd(flag: &str) -> Option<String> {
    let path = std::env::current_exe().ok()?;
    Some(format!("\"{}\" {flag}", path.display()))
}

fn write_value(name: windows::core::PCWSTR, cmd: &str) {
    with_run_key(|key| {
        let wide: Vec<u16> = cmd.encode_utf16().chain(std::iter::once(0)).collect();
        let bytes = unsafe { std::slice::from_raw_parts(wide.as_ptr() as *const u8, wide.len() * 2) };
        let _ = unsafe { RegSetValueExW(key, name, 0, REG_SZ, Some(bytes)) };
    });
}

fn remove_value(name: windows::core::PCWSTR) {
    with_run_key(|key| {
        let _ = unsafe { RegDeleteValueW(key, name) };
    });
}

fn with_run_key(f: impl FnOnce(HKEY)) {
    unsafe {
        let mut key = Default::default();
        let err = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            RUN_KEY,
            0,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &mut key,
            None,
        );
        if err != ERROR_SUCCESS {
            return;
        }
        f(key);
        let _ = RegCloseKey(key);
    }
}

fn claim_named(name: windows::core::PCWSTR, slot: &AtomicIsize) -> bool {
    unsafe {
        let Ok(h) = CreateMutexW(None, true, name) else {
            return false;
        };
        if GetLastError() == ERROR_ALREADY_EXISTS {
            let _ = CloseHandle(h);
            return false;
        }
        slot.store(h.0 as isize, Ordering::SeqCst);
        true
    }
}

fn mutex_held(name: windows::core::PCWSTR) -> bool {
    unsafe {
        let Ok(h) = CreateMutexW(None, false, name) else {
            return false;
        };
        let held = GetLastError() == ERROR_ALREADY_EXISTS;
        let _ = CloseHandle(h);
        held
    }
}

fn release(slot: &AtomicIsize) {
    let raw = slot.swap(0, Ordering::SeqCst);
    if raw != 0 {
        unsafe {
            let _ = CloseHandle(HANDLE(raw as *mut _));
        }
    }
}
