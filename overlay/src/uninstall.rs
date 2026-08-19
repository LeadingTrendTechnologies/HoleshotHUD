use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::util::hidden_powershell;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY_CURRENT_USER, KEY_READ, REG_SZ,
};
use windows::Win32::UI::WindowsAndMessaging::{
    MessageBoxW, IDYES, MB_DEFBUTTON2, MB_ICONWARNING, MB_OK, MB_YESNO,
};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const UNINSTALL_KEY: PCWSTR =
    w!("Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{A7C4E2B1-9F18-4D3A-8C21-6B4E9F2A1C08}_is1");

pub fn confirm(host: HWND) -> bool {
    unsafe {
        MessageBoxW(
            host,
            w!("This removes Holeshot HUD, the MX Bikes plugin, and shortcuts. Continue?"),
            w!("Uninstall Holeshot HUD"),
            MB_YESNO | MB_ICONWARNING | MB_DEFBUTTON2,
        ) == IDYES
    }
}

/// Start uninstall and return true if the overlay should quit.
pub fn start(host: HWND) -> bool {
    crate::startup::set_enabled(false);
    crate::plugin::remove();
    crate::tray::remove();
    if launch_inno() || launch_sidecar_script() || launch_fallback() {
        true
    } else {
        unsafe {
            let _ = MessageBoxW(
                host,
                w!("Could not start uninstall. Use Windows Settings → Apps, or Uninstall Holeshot HUD in the Start menu."),
                w!("Uninstall Holeshot HUD"),
                MB_OK | MB_ICONWARNING,
            );
        }
        false
    }
}

fn launch_inno() -> bool {
    let raw = reg_sz(w!("QuietUninstallString")).or_else(|| reg_sz(w!("UninstallString")));
    let Some(raw) = raw else {
        if let Some(exe) = sidecar("unins000.exe") {
            return spawn_exe(&exe, &["/VERYSILENT".into(), "/NORESTART".into()]);
        }
        return false;
    };
    let Some((exe, mut args)) = split_cmd(&raw) else {
        return false;
    };
    if args.iter().all(|a| !a.eq_ignore_ascii_case("/SILENT") && !a.eq_ignore_ascii_case("/VERYSILENT")) {
        args.push("/VERYSILENT".into());
        args.push("/NORESTART".into());
    }
    spawn_exe(&exe, &args)
}

fn launch_sidecar_script() -> bool {
    let Some(script) = sidecar("Uninstall.ps1") else {
        return false;
    };
    hidden_powershell()
        .args(["-File", &script, "-Silent"])
        .spawn()
        .is_ok()
}

fn launch_fallback() -> bool {
    let exe = std::env::current_exe().ok();
    let pid = std::process::id();
    let plugin = crate::plugin::dest_path()
        .map(|p| p.display().to_string().replace('\'', "''"))
        .unwrap_or_default();
    let app = std::env::var("LOCALAPPDATA")
        .ok()
        .map(|p| PathBuf::from(p).join("Holeshot HUD"));
    let install = app
        .as_ref()
        .map(|p| p.display().to_string().replace('\'', "''"))
        .unwrap_or_default();
    let wipe_install = match (exe.as_ref().and_then(|p| p.parent()), app.as_ref()) {
        (Some(dir), Some(app_dir)) => dir == app_dir,
        _ => false,
    };
    let wipe = if wipe_install { "$true" } else { "$false" };
    let body = format!(
        r#"$ErrorActionPreference = 'Continue'
try {{ Wait-Process -Id {pid} -Timeout 40 -ErrorAction SilentlyContinue }} catch {{}}
Start-Sleep -Milliseconds 600
if ('{plugin}') {{ Remove-Item -LiteralPath '{plugin}' -Force -ErrorAction SilentlyContinue }}
Remove-Item -LiteralPath 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -Name 'Holeshot HUD' -Force -ErrorAction SilentlyContinue
$desktop = [Environment]::GetFolderPath('Desktop')
$start = Join-Path ([Environment]::GetFolderPath('StartMenu')) 'Programs'
foreach ($name in @('Holeshot HUD', 'MXBO Overlay')) {{
  foreach ($lnk in @((Join-Path $desktop "$name.lnk"), (Join-Path $start "$name.lnk"), (Join-Path $start "$name\$name.lnk"))) {{
    if (Test-Path $lnk) {{ Remove-Item -LiteralPath $lnk -Force -ErrorAction SilentlyContinue }}
  }}
}}
if ({wipe}) {{
  Remove-Item -LiteralPath '{install}' -Recurse -Force -ErrorAction SilentlyContinue
}} elseif ('{install}') {{
  Remove-Item -LiteralPath (Join-Path '{install}' 'logs') -Recurse -Force -ErrorAction SilentlyContinue
  Remove-Item -LiteralPath (Join-Path '{install}' 'mxbo.ini') -Force -ErrorAction SilentlyContinue
  Remove-Item -LiteralPath (Join-Path '{install}' 'gamedir.txt') -Force -ErrorAction SilentlyContinue
}}
"#
    );
    let script = std::env::temp_dir().join("holeshot-uninstall.ps1");
    if std::fs::write(&script, body).is_err() {
        return false;
    }
    hidden_powershell()
        .args(["-File", script.to_str().unwrap_or_default()])
        .spawn()
        .is_ok()
}

fn sidecar(name: &str) -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let path = exe.parent()?.join(name);
    path.is_file().then(|| path.display().to_string())
}

fn spawn_exe(exe: &str, args: &[String]) -> bool {
    Command::new(exe)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok()
}

fn split_cmd(s: &str) -> Option<(String, Vec<String>)> {
    let s = s.trim();
    if s.starts_with('"') {
        let rest = &s[1..];
        let end = rest.find('"')?;
        let exe = rest[..end].to_string();
        let args = rest[end + 1..]
            .split_whitespace()
            .map(str::to_string)
            .collect();
        Some((exe, args))
    } else {
        let mut parts = s.split_whitespace();
        let exe = parts.next()?.to_string();
        Some((exe, parts.map(str::to_string).collect()))
    }
}

fn reg_sz(value: PCWSTR) -> Option<String> {
    unsafe {
        let mut key = Default::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, UNINSTALL_KEY, 0, KEY_READ, &mut key).is_err() {
            return None;
        }
        let mut buf = [0u16; 520];
        let mut bytes = (buf.len() * 2) as u32;
        let mut ty = REG_SZ;
        let ok = RegQueryValueExW(
            key,
            value,
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
            Some(s)
        }
    }
}
