use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

const REPO: &str = "LeadingTrendTechnologies/HoleshotHUD";
const UA: &str = "mxbo-overlay";

static QUIT: AtomicBool = AtomicBool::new(false);
static STATE: Mutex<UpdateState> = Mutex::new(UpdateState::Idle);

#[derive(Clone)]
pub enum UpdateState {
    Idle,
    Checking,
    Current,
    Available { version: String, url: String },
    Downloading,
    Failed(String),
}

pub fn state() -> UpdateState {
    STATE.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

pub fn should_quit() -> bool {
    QUIT.load(Ordering::SeqCst)
}

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
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
        let next = match latest_release() {
            Ok((ver, url)) => {
                if version_newer(&ver, current_version()) {
                    UpdateState::Available { version: ver, url }
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
    let (ver, url) = {
        let g = STATE.lock().unwrap_or_else(|e| e.into_inner());
        match &*g {
            UpdateState::Available { version, url } => (version.clone(), url.clone()),
            _ => return,
        }
    };
    *STATE.lock().unwrap_or_else(|e| e.into_inner()) = UpdateState::Downloading;
    std::thread::spawn(move || {
        match apply(&ver, &url) {
            Ok(()) => QUIT.store(true, Ordering::SeqCst),
            Err(e) => *STATE.lock().unwrap_or_else(|e| e.into_inner()) = UpdateState::Failed(e),
        }
    });
}

fn latest_release() -> Result<(String, String), String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(25))
        .user_agent(UA)
        .build();
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let body = agent
        .get(&url)
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(404, _) => "No release has been published yet.".into(),
            other => format!("Could not reach GitHub: {other}"),
        })?
        .into_string()
        .map_err(|e| format!("Could not read GitHub response: {e}"))?;
    let tag = json_string(&body, "tag_name").ok_or_else(|| "Release is missing a version tag.".to_string())?;
    let zip = json_download_url(&body).ok_or_else(|| "Release has no Windows zip.".to_string())?;
    Ok((tag.trim_start_matches('v').to_string(), zip))
}

fn apply(version: &str, url: &str) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("Could not find this app: {e}"))?;
    let work = std::env::temp_dir().join(format!("mxbo-update-{version}"));
    let _ = fs::remove_dir_all(&work);
    fs::create_dir_all(&work).map_err(|e| format!("Could not create temp folder: {e}"))?;
    let zip = work.join("update.zip");
    download(url, &zip)?;
    let extracted = work.join("extracted");
    fs::create_dir_all(&extracted).map_err(|e| format!("Could not create extract folder: {e}"))?;
    let status = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &format!(
                "Expand-Archive -LiteralPath '{}' -DestinationPath '{}' -Force",
                zip.display(),
                extracted.display()
            ),
        ])
        .status()
        .map_err(|e| format!("Could not unzip the update: {e}"))?;
    if !status.success() {
        return Err("Could not unzip the update.".into());
    }
    let src_exe = find_file(&extracted, |n| n.ends_with(".exe"))
        .ok_or_else(|| "Update zip is missing the overlay.".to_string())?;
    let src_dlo = find_file(&extracted, |n| n.eq_ignore_ascii_case("mxbo.dlo"));
    let script = work.join("apply.ps1");
    let pid = std::process::id();
    let plugin = src_dlo
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let body = format!(
        r#"$ErrorActionPreference = 'Continue'
$pidToWait = {pid}
$srcExe = '{}'
$dstExe = '{}'
$srcDlo = '{plugin}'
try {{ Wait-Process -Id $pidToWait -Timeout 40 -ErrorAction SilentlyContinue }} catch {{}}
Start-Sleep -Milliseconds 800
Copy-Item -LiteralPath $srcExe -Destination $dstExe -Force
if ($srcDlo -and (Test-Path -LiteralPath $srcDlo)) {{
  $game = $null
  foreach ($key in @('HKCU:\Software\Valve\Steam','HKLM:\SOFTWARE\WOW6432Node\Valve\Steam','HKLM:\SOFTWARE\Valve\Steam')) {{
    try {{ $steam = (Get-ItemProperty -Path $key -ErrorAction Stop).InstallPath }} catch {{ $steam = $null }}
    if ($steam) {{
      $c = Join-Path $steam 'steamapps\common\MX Bikes\plugins\mxbo.dlo'
      if (Test-Path (Split-Path $c)) {{ $game = $c; break }}
    }}
  }}
  foreach ($drive in @('C','D','E')) {{
    $c = "${{drive}}:\Steam\steamapps\common\MX Bikes\plugins\mxbo.dlo"
    if (-not $game -and (Test-Path (Split-Path $c))) {{ $game = $c }}
  }}
  if ($game) {{
    try {{ Copy-Item -LiteralPath $srcDlo -Destination $game -Force }} catch {{}}
  }}
}}
Start-Process -FilePath $dstExe
"#,
        src_exe.display(),
        exe.display(),
    );
    let mut f = fs::File::create(&script).map_err(|e| format!("Could not write updater: {e}"))?;
    f.write_all(body.as_bytes()).map_err(|e| format!("Could not write updater: {e}"))?;
    Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-File",
            script.to_str().ok_or("Bad updater path")?,
        ])
        .spawn()
        .map_err(|e| format!("Could not start updater: {e}"))?;
    Ok(())
}

fn download(url: &str, dest: &Path) -> Result<(), String> {
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

fn json_string(body: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let i = body.find(&needle)?;
    let rest = &body[i + needle.len()..];
    let colon = rest.find(':')?;
    let rest = rest[colon + 1..].trim_start();
    if !rest.starts_with('"') {
        return None;
    }
    let mut out = String::new();
    let mut chars = rest[1..].chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(n) = chars.next() {
                out.push(n);
            }
        } else if ch == '"' {
            break;
        } else {
            out.push(ch);
        }
    }
    Some(out)
}

fn json_download_url(body: &str) -> Option<String> {
    let mut search = body;
    while let Some(rel) = search.find("\"browser_download_url\"") {
        let slice = &search[rel..];
        if let Some(url) = json_string(slice, "browser_download_url") {
            if url.contains("windows-x64.zip") {
                return Some(url);
            }
        }
        search = &search[rel + 20..];
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
