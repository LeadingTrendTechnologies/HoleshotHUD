use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use windows::core::{w, PCWSTR};
use windows::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ,
    REG_SZ,
};

const EMBEDDED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/mxbo.dlo"));

static NEED_RETRY: AtomicBool = AtomicBool::new(false);

pub fn sync() {
    match install_once() {
        Ok(true) | Err(_) => NEED_RETRY.store(true, Ordering::Relaxed),
        Ok(false) => NEED_RETRY.store(false, Ordering::Relaxed),
    }
}

pub fn retry_if_needed() {
    if NEED_RETRY.load(Ordering::Relaxed) {
        sync();
    }
}

pub fn remove() {
    if let Some(dest) = plugin_dest() {
        let _ = fs::remove_file(dest);
    }
}

pub fn dest_path() -> Option<PathBuf> {
    plugin_dest()
}

fn install_once() -> Result<bool, ()> {
    let bytes = plugin_bytes().ok_or(())?;
    let dest = plugin_dest().ok_or(())?;
    if dest.is_file() {
        if let Ok(existing) = fs::read(&dest) {
            if existing == bytes {
                if let Some(game) = dest.parent().and_then(|p| p.parent()) {
                    save_game_dir(game);
                }
                return Ok(false);
            }
        }
    }
    if let Some(parent) = dest.parent() {
        let _ = fs::create_dir_all(parent);
    }
    match fs::write(&dest, bytes) {
        Ok(()) => {
            if let Some(game) = dest.parent().and_then(|p| p.parent()) {
                save_game_dir(game);
            }
            Ok(false)
        }
        Err(_) => Ok(true),
    }
}

fn plugin_bytes() -> Option<Vec<u8>> {
    if let Some(path) = sidecar_plugin() {
        if let Ok(bytes) = fs::read(path) {
            if bytes.len() > 1024 {
                return Some(bytes);
            }
        }
    }
    if EMBEDDED.len() > 1024 {
        return Some(EMBEDDED.to_vec());
    }
    None
}

fn sidecar_plugin() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let names = [
        dir.join("mxbo.dlo"),
        dir.join("../../../out/Release/mxbo.dlo"),
        dir.join("../../out/Release/mxbo.dlo"),
    ];
    names.into_iter().find(|p| p.is_file())
}

fn plugin_dest() -> Option<PathBuf> {
    Some(game_dir()?.join("plugins").join("mxbo.dlo"))
}

fn game_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("MXBIKES_DIR") {
        let p = PathBuf::from(dir);
        if looks_like_game(&p) {
            save_game_dir(&p);
            return Some(p);
        }
    }
    if let Some(p) = saved_game_dir() {
        if looks_like_game(&p) {
            return Some(p);
        }
    }
    for lib in steam_libraries() {
        let p = lib.join("steamapps").join("common").join("MX Bikes");
        if looks_like_game(&p) {
            save_game_dir(&p);
            return Some(p);
        }
    }
    for p in [
        r"C:\Program Files (x86)\Steam\steamapps\common\MX Bikes",
        r"C:\Steam\steamapps\common\MX Bikes",
        r"D:\Steam\steamapps\common\MX Bikes",
        r"E:\Steam\steamapps\common\MX Bikes",
    ] {
        let p = PathBuf::from(p);
        if looks_like_game(&p) {
            save_game_dir(&p);
            return Some(p);
        }
    }
    None
}

fn looks_like_game(p: &Path) -> bool {
    p.join("plugins").is_dir() || p.join("mxbikes.exe").is_file()
}

fn app_dir() -> Option<PathBuf> {
    Some(PathBuf::from(std::env::var("LOCALAPPDATA").ok()?).join("Holeshot HUD"))
}

fn saved_game_dir() -> Option<PathBuf> {
    let text = fs::read_to_string(app_dir()?.join("gamedir.txt")).ok()?;
    let p = PathBuf::from(text.trim());
    if p.as_os_str().is_empty() {
        None
    } else {
        Some(p)
    }
}

fn save_game_dir(p: &Path) {
    let Some(dir) = app_dir() else {
        return;
    };
    let _ = fs::create_dir_all(&dir);
    let _ = fs::write(dir.join("gamedir.txt"), p.to_string_lossy().as_bytes());
}

fn steam_libraries() -> Vec<PathBuf> {
    let mut libs = Vec::new();
    for root in steam_roots() {
        if !root.is_dir() {
            continue;
        }
        libs.push(root.clone());
        let vdf = root.join("steamapps").join("libraryfolders.vdf");
        if let Ok(text) = fs::read_to_string(vdf) {
            for line in text.lines() {
                if let Some(path) = vdf_path(line) {
                    if Path::new(&path).is_dir() {
                        libs.push(PathBuf::from(path));
                    }
                }
            }
        }
    }
    libs.sort();
    libs.dedup();
    libs
}

fn vdf_path(line: &str) -> Option<String> {
    let rest = line.split("\"path\"").nth(1)?;
    let start = rest.find('"')? + 1;
    let end = rest[start..].find('"')? + start;
    Some(rest[start..end].replace("\\\\", "\\"))
}

fn steam_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for (hive, key) in [
        (HKEY_CURRENT_USER, w!("Software\\Valve\\Steam")),
        (HKEY_LOCAL_MACHINE, w!("SOFTWARE\\WOW6432Node\\Valve\\Steam")),
        (HKEY_LOCAL_MACHINE, w!("SOFTWARE\\Valve\\Steam")),
    ] {
        if let Some(p) = reg_install_path(hive, key) {
            roots.push(p);
        }
    }
    roots.sort();
    roots.dedup();
    roots
}

fn reg_install_path(hive: windows::Win32::System::Registry::HKEY, subkey: PCWSTR) -> Option<PathBuf> {
    unsafe {
        let mut key = Default::default();
        if RegOpenKeyExW(hive, subkey, 0, KEY_READ, &mut key).is_err() {
            return None;
        }
        let mut buf = [0u16; 520];
        let mut bytes = (buf.len() * 2) as u32;
        let mut ty = REG_SZ;
        let ok = RegQueryValueExW(
            key,
            w!("InstallPath"),
            None,
            Some(&mut ty),
            Some(buf.as_mut_ptr() as *mut u8),
            Some(&mut bytes),
        )
        .is_ok();
        let _ = RegCloseKey(key);
        if !ok {
            return None;
        }
        let n = (bytes as usize / 2).saturating_sub(1).min(buf.len());
        let s = String::from_utf16_lossy(&buf[..n]);
        if s.is_empty() {
            None
        } else {
            Some(PathBuf::from(s))
        }
    }
}
