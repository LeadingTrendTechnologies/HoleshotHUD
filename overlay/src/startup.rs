use windows::core::w;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegSetValueExW, HKEY_CURRENT_USER,
    KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
};

const VALUE_NAME: windows::core::PCWSTR = w!("Holeshot HUD");
const RUN_KEY: windows::core::PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");

pub fn set_enabled(on: bool) {
    if on {
        add();
    } else {
        remove();
    }
}

pub fn sync_from_config() {
    set_enabled(crate::config::with_config(|c| c.start_with_windows));
}

fn add() {
    let Some(cmd) = exe_cmd() else {
        return;
    };
    with_run_key(|key| {
        let wide: Vec<u16> = cmd.encode_utf16().chain(std::iter::once(0)).collect();
        let bytes = unsafe {
            std::slice::from_raw_parts(wide.as_ptr() as *const u8, wide.len() * 2)
        };
        let _ = unsafe { RegSetValueExW(key, VALUE_NAME, 0, REG_SZ, Some(bytes)) };
    });
}

fn remove() {
    with_run_key(|key| {
        let _ = unsafe { RegDeleteValueW(key, VALUE_NAME) };
    });
}

fn exe_cmd() -> Option<String> {
    let path = std::env::current_exe().ok()?;
    Some(format!("\"{}\" --minimized", path.display()))
}

fn with_run_key(f: impl FnOnce(windows::Win32::System::Registry::HKEY)) {
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
