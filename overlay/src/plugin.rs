use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use windows::core::{w, PCWSTR};
use windows::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ,
    REG_SZ,
};

const PLUGIN_FILE: &str = "Holeshot-HUD.dlo";
const LEGACY_PLUGIN_FILE: &str = "mxbo.dlo";

const EMBEDDED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/Holeshot-HUD.dlo"));

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

/// True when the plugin file could not be replaced (usually because MX Bikes still has it open).
pub fn needs_restart() -> bool {
    NEED_RETRY.load(Ordering::Relaxed)
}

pub fn game_exe() -> Option<String> {
    let p = game_dir()?.join("mxbikes.exe");
    p.is_file().then(|| p.to_string_lossy().into_owned())
}

pub fn remove() {
    if let Some(dir) = game_dir().map(|g| g.join("plugins")) {
        let _ = fs::remove_file(dir.join(PLUGIN_FILE));
        let _ = fs::remove_file(dir.join(LEGACY_PLUGIN_FILE));
    }
}

pub fn dest_path() -> Option<PathBuf> {
    Some(game_dir()?.join("plugins").join(PLUGIN_FILE))
}

pub fn plugin_installed() -> bool {
    dest_path().is_some_and(|p| p.is_file())
}

/// Folder picker. Saves the path and copies the plugin when the folder looks like MX Bikes.
pub fn pick_game_folder(host: windows::Win32::Foundation::HWND) -> bool {
    let Some(picked) = browse_folder(host) else {
        return false;
    };
    if !looks_like_game(&picked) {
        unsafe {
            let _ = windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
                host,
                w!("That folder does not look like MX Bikes (missing mxbikes.exe / plugins)."),
                w!("Holeshot HUD"),
                windows::Win32::UI::WindowsAndMessaging::MB_OK
                    | windows::Win32::UI::WindowsAndMessaging::MB_ICONWARNING,
            );
        }
        return false;
    }
    save_game_dir(&picked);
    NEED_RETRY.store(true, Ordering::Relaxed);
    sync();
    true
}

fn browse_folder(host: windows::Win32::Foundation::HWND) -> Option<PathBuf> {
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::{
        FileOpenDialog, IFileOpenDialog, FOS_FORCEFILESYSTEM, FOS_PICKFOLDERS, SIGDN_FILESYSPATH,
    };
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let dlg: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_ALL).ok()?;
        dlg.SetOptions(FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM).ok()?;
        dlg.SetTitle(w!("Select the MX Bikes folder (the one that contains mxbikes.exe)"))
            .ok()?;
        dlg.Show(host).ok()?;
        let item = dlg.GetResult().ok()?;
        let name = item.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
        let path = name.to_string().ok().filter(|s| !s.is_empty());
        CoTaskMemFree(Some(name.0 as _));
        path.map(PathBuf::from)
    }
}

fn install_once() -> Result<bool, ()> {
    let bytes = plugin_bytes().ok_or(())?;
    refresh_sidecar(&bytes);
    let dest = plugin_dest().ok_or(())?;
    if dest.is_file() {
        if let Ok(existing) = fs::read(&dest) {
            if existing == bytes {
                if let Some(game) = dest.parent().and_then(|p| p.parent()) {
                    save_game_dir(game);
                    remove_legacy_plugin(game);
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
                remove_legacy_plugin(game);
            }
            Ok(false)
        }
        Err(_) => Ok(true),
    }
}

fn remove_legacy_plugin(game: &Path) {
    let _ = fs::remove_file(game.join("plugins").join(LEGACY_PLUGIN_FILE));
}

fn plugin_bytes() -> Option<Vec<u8>> {
    let sidecar = sidecar_plugin().and_then(|p| fs::read(p).ok());
    pick_plugin_bytes(EMBEDDED, sidecar.as_deref())
}

/// Prefer the plugin baked into this exe. A leftover sidecar from an older install
/// used to win after an update, so MX Bikes kept publishing into the wrong SHM name.
fn pick_plugin_bytes(embedded: &[u8], sidecar: Option<&[u8]>) -> Option<Vec<u8>> {
    if embedded.len() > 1024 {
        return Some(embedded.to_vec());
    }
    sidecar.filter(|b| b.len() > 1024).map(|b| b.to_vec())
}

fn refresh_sidecar(bytes: &[u8]) {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let Some(dir) = exe.parent() else {
        return;
    };
    let path = dir.join(PLUGIN_FILE);
    if fs::read(&path).ok().as_deref() == Some(bytes) {
        return;
    }
    let _ = fs::write(&path, bytes);
}

fn sidecar_plugin() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let names = [
        dir.join(PLUGIN_FILE),
        dir.join("../../../out/Release").join(PLUGIN_FILE),
        dir.join("../../out/Release").join(PLUGIN_FILE),
        // Dev / old packs until everything is rebuilt.
        dir.join(LEGACY_PLUGIN_FILE),
        dir.join("../../../out/Release").join(LEGACY_PLUGIN_FILE),
    ];
    names.into_iter().find(|p| p.is_file())
}

fn plugin_dest() -> Option<PathBuf> {
    Some(game_dir()?.join("plugins").join(PLUGIN_FILE))
}

pub fn game_dir() -> Option<PathBuf> {
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

#[cfg(test)]
mod tests {
    use super::pick_plugin_bytes;

    #[test]
    fn embedded_plugin_wins_over_sidecar() {
        let embedded = vec![1u8; 2000];
        let sidecar = vec![2u8; 2000];
        assert_eq!(pick_plugin_bytes(&embedded, Some(&sidecar)).unwrap()[0], 1);
    }

    #[test]
    fn sidecar_used_when_embed_is_placeholder() {
        let sidecar = vec![3u8; 2000];
        assert_eq!(pick_plugin_bytes(&[], Some(&sidecar)).unwrap()[0], 3);
        assert!(pick_plugin_bytes(&[], Some(&[1, 2, 3])).is_none());
    }
}
