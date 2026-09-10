use std::fs;
use std::io;
use std::os::windows::process::CommandExt;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use sha2::{Digest, Sha256};
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE};

use crate::util::{json_array_slice, json_string};

const REPO: &str = "LeadingTrendTechnologies/HoleshotHUD";
const UA: &str = "mxbo-overlay";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const DETACHED_PROCESS: u32 = 0x0000_0008;
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;
const OVERLAY_EXE: &str = "Holeshot-HUD.exe";
const PLUGIN_DLO: &str = "Holeshot-HUD.dlo";

static QUIT: AtomicBool = AtomicBool::new(false);
static STATE: Mutex<UpdateState> = Mutex::new(UpdateState::Idle);

#[derive(Clone)]
pub enum UpdateState {
    Idle,
    Checking,
    Current,
    Available {
        version: String,
        url: String,
        digest: String,
    },
    Downloading,
    Failed(String),
}

pub fn state() -> UpdateState {
    STATE.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Banner on the settings window when auto-update is off and a newer build exists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ManualBanner {
    Available { version: String },
    Installing,
}

pub fn manual_banner(auto_update: bool, dismissed: bool, state: &UpdateState) -> Option<ManualBanner> {
    if auto_update || dismissed {
        return None;
    }
    match state {
        UpdateState::Available { version, .. } => Some(ManualBanner::Available {
            version: version.clone(),
        }),
        UpdateState::Downloading => Some(ManualBanner::Installing),
        _ => None,
    }
}

pub fn should_quit() -> bool {
    QUIT.load(Ordering::SeqCst)
}

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Folder that contains the running overlay exe (the install location).
pub fn install_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
}

pub fn install_dir_display() -> String {
    install_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "(unknown)".into())
}

/// True when replacing the exe may fail without elevation (e.g. Program Files).
pub fn update_may_need_admin() -> bool {
    static NEED_ADMIN: LazyLock<bool> = LazyLock::new(|| {
        let Some(dir) = install_dir() else {
            return false;
        };
        if looks_protected(&dir) {
            return true;
        }
        !dir_is_writable(&dir)
    });
    *NEED_ADMIN
}

fn looks_protected(dir: &Path) -> bool {
    let s = dir.to_string_lossy().to_ascii_lowercase();
    let markers = [
        "\\program files\\",
        "\\program files (x86)\\",
        "\\windows\\",
        "/program files/",
        "/program files (x86)/",
        "/windows/",
    ];
    markers.iter().any(|m| s.contains(m))
}

fn dir_is_writable(dir: &Path) -> bool {
    let probe = dir.join(".holeshot-write-test");
    match fs::OpenOptions::new().write(true).create(true).truncate(true).open(&probe) {
        Ok(_) => {
            let _ = fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

pub const ADMIN_UPDATE_HINT: &str =
    "This install folder may need admin approval to update.";

/// Check GitHub and install a newer build before the UI opens.
/// Returns true if this process should exit so the updater can relaunch.
pub fn apply_on_launch() -> bool {
    if std::env::var_os("HOLESHOT_SKIP_UPDATE").is_some() {
        return false;
    }
    if std::env::args().any(|a| a == "--skip-update") {
        return false;
    }
    let Ok(rel) = latest_release_in(Duration::from_secs(10)) else {
        return false;
    };
    if !version_newer(&rel.version, current_version()) {
        return false;
    }
    apply(&rel).is_ok()
}

pub fn check() {
    {
        let mut g = STATE.lock().unwrap_or_else(|e| e.into_inner());
        if matches!(*g, UpdateState::Checking | UpdateState::Downloading) {
            return;
        }
        *g = UpdateState::Checking;
    }
    std::thread::spawn(|| {
        let next = match latest_release_in(Duration::from_secs(25)) {
            Ok(rel) => {
                if version_newer(&rel.version, current_version()) {
                    UpdateState::Available {
                        version: rel.version,
                        url: rel.url,
                        digest: rel.digest,
                    }
                } else {
                    UpdateState::Current
                }
            }
            Err(e) => UpdateState::Failed(e),
        };
        *STATE.lock().unwrap_or_else(|e| e.into_inner()) = next;
    });
}

pub fn install() {
    let rel = {
        let g = STATE.lock().unwrap_or_else(|e| e.into_inner());
        match &*g {
            UpdateState::Available {
                version,
                url,
                digest,
            } => ReleaseZip {
                version: version.clone(),
                url: url.clone(),
                digest: digest.clone(),
            },
            _ => return,
        }
    };
    *STATE.lock().unwrap_or_else(|e| e.into_inner()) = UpdateState::Downloading;
    std::thread::spawn(move || {
        match apply(&rel) {
            Ok(()) => QUIT.store(true, Ordering::SeqCst),
            Err(e) => *STATE.lock().unwrap_or_else(|e| e.into_inner()) = UpdateState::Failed(e),
        }
    });
}

struct ReleaseZip {
    version: String,
    url: String,
    digest: String,
}

fn latest_release_in(timeout: Duration) -> Result<ReleaseZip, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(timeout)
        .user_agent(UA)
        .build();
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let body = agent
        .get(&url)
        .set("Accept", "application/vnd.github+json")
        .set("X-GitHub-Api-Version", "2022-11-28")
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(404, _) => "No release has been published yet.".into(),
            other => format!("Could not reach GitHub: {other}"),
        })?
        .into_string()
        .map_err(|e| format!("Could not read GitHub response: {e}"))?;
    parse_latest_release(&body)
}

fn parse_latest_release(body: &str) -> Result<ReleaseZip, String> {
    let tag = json_string(body, "tag_name").ok_or_else(|| "Release is missing a version tag.".to_string())?;
    let zip = parse_windows_zip_asset(body).ok_or_else(|| "Release has no Windows zip.".to_string())?;
    if !release_zip_url_ok(&zip.url) {
        return Err("Release zip URL is not the Holeshot HUD GitHub download.".into());
    }
    let digest = parse_sha256_digest(&zip.digest).ok_or_else(|| "Release zip is missing a SHA-256 digest.".to_string())?;
    Ok(ReleaseZip {
        version: tag.trim_start_matches('v').to_string(),
        url: zip.url,
        digest,
    })
}

struct ZipAsset {
    url: String,
    digest: String,
}

fn parse_windows_zip_asset(body: &str) -> Option<ZipAsset> {
    let assets = json_array_slice(body, "assets")?;
    let mut search = assets;
    while let Some(rel) = search.find('{') {
        let Some(obj) = json_object_at(&search[rel..]) else {
            search = &search[rel + 1..];
            continue;
        };
        if let Some(url) = json_string(obj, "browser_download_url") {
            if url.contains("windows-x64.zip") && !url.to_ascii_lowercase().contains("setup") {
                return Some(ZipAsset {
                    url,
                    digest: json_string(obj, "digest").unwrap_or_default(),
                });
            }
        }
        search = &search[rel + 1..];
    }
    None
}

fn json_object_at(s: &str) -> Option<&str> {
    let bytes = s.as_bytes();
    if !s.starts_with('{') {
        return None;
    }
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    for (i, &c) in bytes.iter().enumerate() {
        if in_str {
            if esc {
                esc = false;
            } else if c == b'\\' {
                esc = true;
            } else if c == b'"' {
                in_str = false;
            }
        } else if c == b'"' {
            in_str = true;
        } else if c == b'{' {
            depth += 1;
        } else if c == b'}' {
            depth -= 1;
            if depth == 0 {
                return s.get(..=i);
            }
        }
    }
    None
}

fn release_zip_url_ok(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    if !lower.starts_with("https://github.com/leadingtrendtechnologies/holeshothud/releases/download/") {
        return false;
    }
    if lower.contains("..") || lower.contains('\\') {
        return false;
    }
    lower.contains("windows-x64.zip") && !lower.contains("setup")
}

fn parse_sha256_digest(digest: &str) -> Option<String> {
    let s = digest.trim();
    let hex = s
        .strip_prefix("sha256:")
        .or_else(|| s.strip_prefix("SHA256:"))?;
    let hex = hex.trim();
    if hex.len() != 64 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    Some(hex.to_ascii_lowercase())
}

fn apply(rel: &ReleaseZip) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("Could not find this app: {e}"))?;
    let work = std::env::temp_dir().join(format!("mxbo-update-{}", rel.version));
    let _ = fs::remove_dir_all(&work);
    fs::create_dir_all(&work).map_err(|e| format!("Could not create temp folder: {e}"))?;
    let zip = work.join("update.zip");
    download(&rel.url, &zip)?;
    let got = sha256_hex(&zip)?;
    if got != rel.digest {
        let _ = fs::remove_dir_all(&work);
        return Err("Update zip did not match the GitHub SHA-256 digest.".into());
    }
    let extracted = work.join("extracted");
    fs::create_dir_all(&extracted).map_err(|e| format!("Could not create extract folder: {e}"))?;
    unzip(&zip, &extracted)?;
    let src_exe = find_file(&extracted, is_overlay_exe)
        .ok_or_else(|| "Update zip is missing Holeshot-HUD.exe.".to_string())?;
    let src_dlo = find_file(&extracted, is_plugin_dlo);
    let plugin_changed = src_dlo.as_ref().is_some_and(|p| zip_plugin_differs_from_installed(p));
    spawn_apply_helper(&work, &src_exe, &exe, src_dlo.as_deref(), plugin_changed)?;
    Ok(())
}

fn spawn_apply_helper(
    work: &Path,
    src_exe: &Path,
    dst_exe: &Path,
    src_dlo: Option<&Path>,
    plugin_changed: bool,
) -> Result<(), String> {
    let helper = work.join("holeshot-apply.exe");
    let self_exe = std::env::current_exe().map_err(|e| format!("Could not find this app: {e}"))?;
    fs::copy(&self_exe, &helper).map_err(|e| format!("Could not stage updater: {e}"))?;
    let dlo = src_dlo
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "-".into());
    let mut cmd = Command::new(&helper);
    cmd.arg("--apply-update")
        .arg(std::process::id().to_string())
        .arg(src_exe)
        .arg(dst_exe)
        .arg(&dlo)
        .args(relaunch_args(plugin_changed))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(
            DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW | CREATE_BREAKAWAY_FROM_JOB,
        );
    cmd.spawn()
        .map_err(|e| format!("Could not start updater: {e}"))?;
    Ok(())
}

/// Helper process: wait for the UI pid, copy the new exe/plugin, relaunch.
pub fn apply_staged_from_args() -> Result<(), String> {
    let mut args = std::env::args();
    let _ = args.next();
    if args.next().as_deref() != Some("--apply-update") {
        return Err("missing --apply-update".into());
    }
    let pid: u32 = args
        .next()
        .ok_or_else(|| "missing pid".to_string())?
        .parse()
        .map_err(|_| "bad pid".to_string())?;
    let src_exe = PathBuf::from(args.next().ok_or_else(|| "missing source exe".to_string())?);
    let dst_exe = PathBuf::from(args.next().ok_or_else(|| "missing dest exe".to_string())?);
    let dlo_arg = args.next().unwrap_or_else(|| "-".into());
    let relaunch: Vec<String> = args.collect();
    if !is_overlay_exe(
        src_exe
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default(),
    ) {
        return Err("update source is not Holeshot-HUD.exe".into());
    }
    if !is_overlay_exe(
        dst_exe
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default(),
    ) {
        return Err("update dest is not Holeshot-HUD.exe".into());
    }
    let src_dlo = if dlo_arg.is_empty() || dlo_arg == "-" {
        None
    } else {
        let p = PathBuf::from(&dlo_arg);
        if !is_plugin_dlo(p.file_name().and_then(|n| n.to_str()).unwrap_or_default()) {
            return Err("update plugin is not Holeshot-HUD.dlo".into());
        }
        Some(p)
    };
    wait_for_pid(pid, 40_000);
    std::thread::sleep(Duration::from_millis(400));
    fs::copy(&src_exe, &dst_exe).map_err(|e| format!("Could not replace overlay: {e}"))?;
    if let Some(dlo) = src_dlo {
        if let Some(dir) = dst_exe.parent() {
            let _ = fs::copy(&dlo, dir.join(PLUGIN_DLO));
        }
    }
    let mut child = Command::new(&dst_exe);
    child
        .args(&relaunch)
        .env("HOLESHOT_SKIP_UPDATE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(
            DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW | CREATE_BREAKAWAY_FROM_JOB,
        );
    child
        .spawn()
        .map_err(|e| format!("Could not relaunch overlay: {e}"))?;
    Ok(())
}

fn wait_for_pid(pid: u32, timeout_ms: u32) {
    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) else {
            return;
        };
        let _ = WaitForSingleObject(handle, timeout_ms);
        let _ = CloseHandle(handle);
    }
}

fn sha256_hex(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|e| format!("Could not read update zip: {e}"))?;
    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut hasher).map_err(|e| format!("Could not hash update zip: {e}"))?;
    Ok(format!("{:x}", hasher.finalize()))
}

fn unzip(src: &Path, dest: &Path) -> Result<(), String> {
    let file = fs::File::open(src).map_err(|e| format!("Could not open update zip: {e}"))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Could not read update zip: {e}"))?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| format!("Could not read zip entry: {e}"))?;
        let name = entry.mangled_name();
        let Some(out) = safe_extract_path(dest, &name) else {
            continue;
        };
        if entry.is_dir() {
            fs::create_dir_all(&out).map_err(|e| format!("Could not create extract folder: {e}"))?;
            continue;
        }
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Could not create extract folder: {e}"))?;
        }
        let mut dest_file = fs::File::create(&out).map_err(|e| format!("Could not extract update: {e}"))?;
        io::copy(&mut entry, &mut dest_file).map_err(|e| format!("Could not extract update: {e}"))?;
    }
    Ok(())
}

fn safe_extract_path(dest: &Path, name: &Path) -> Option<PathBuf> {
    if name.as_os_str().is_empty() {
        return None;
    }
    if name.components().any(|c| {
        matches!(
            c,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return None;
    }
    let out = dest.join(name);
    out.starts_with(dest).then_some(out)
}

fn download(url: &str, dest: &Path) -> Result<(), String> {
    if !release_zip_url_ok(url) {
        return Err("Update URL is not the Holeshot HUD GitHub download.".into());
    }
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(120))
        .user_agent(UA)
        .build();
    let mut reader = agent
        .get(url)
        .call()
        .map_err(|e| format!("Download failed: {e}"))?
        .into_reader();
    let mut file = fs::File::create(dest).map_err(|e| format!("Could not save update: {e}"))?;
    std::io::copy(&mut reader, &mut file).map_err(|e| format!("Could not save update: {e}"))?;
    Ok(())
}

fn is_overlay_exe(name: &str) -> bool {
    name.eq_ignore_ascii_case(OVERLAY_EXE)
}

fn is_plugin_dlo(name: &str) -> bool {
    name.eq_ignore_ascii_case(PLUGIN_DLO)
}

fn find_file(root: &Path, pred: impl Fn(&str) -> bool) -> Option<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(&pred)
            {
                return Some(path);
            }
        }
    }
    None
}

fn version_newer(latest: &str, current: &str) -> bool {
    parse_ver(latest) > parse_ver(current)
}

fn parse_ver(s: &str) -> [u32; 3] {
    let s = s.trim().trim_start_matches('v');
    let mut out = [0u32; 3];
    for (i, part) in s.split('.').take(3).enumerate() {
        let digits: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
        out[i] = digits.parse().unwrap_or(0);
    }
    out
}

fn zip_plugin_differs_from_installed(src_dlo: &Path) -> bool {
    let Ok(new) = fs::read(src_dlo) else {
        return false;
    };
    match crate::plugin::dest_path() {
        Some(dest) if dest.is_file() => fs::read(&dest).map(|old| old != new).unwrap_or(true),
        Some(_) => true,
        None => false,
    }
}

#[cfg(test)]
fn dlo_differs(new_bytes: &[u8], existing: Option<&[u8]>) -> bool {
    match existing {
        Some(old) => old != new_bytes,
        None => true,
    }
}

fn relaunch_args(plugin_changed: bool) -> Vec<&'static str> {
    if plugin_changed {
        vec!["--skip-update", "--whats-new", "--plugin-changed"]
    } else {
        vec!["--skip-update", "--whats-new"]
    }
}

#[cfg(test)]
#[path = "tests/update.rs"]
mod tests;
