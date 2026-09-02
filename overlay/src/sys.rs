use std::collections::HashMap;
use std::mem::size_of;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{CloseHandle, FILETIME};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Module32FirstW, Module32NextW, Process32FirstW, Process32NextW,
    MODULEENTRY32W, PROCESSENTRY32W, TH32CS_SNAPMODULE, TH32CS_SNAPMODULE32, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetProcessTimes, GetSystemTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
};

use crate::gpu::Gpu;
use crate::ping::PingWatch;
use mxbo_hud::config::{SysApp, SysAppKind, SYS_PROC_MAX};
use mxbo_hud::SysProc;

#[derive(Clone, Copy, Default)]
struct ProcStat {
    pid: u32,
    last_cpu: u64,
    cpu: f32,
    gpu: f32,
    mem_mb: f32,
    mem_pct: f32,
    on: bool,
}

impl ProcStat {
    fn pack(self, label: &str) -> SysProc {
        SysProc {
            label: label.into(),
            cpu: self.cpu,
            gpu: self.gpu,
            mem_mb: self.mem_mb,
            mem_pct: self.mem_pct,
            on: self.on,
        }
    }
}

pub struct Sampler {
    last_idle: u64,
    last_kern: u64,
    last_user: u64,
    last_sample: Option<Instant>,
    last_seq: u32,
    last_seq_at: Option<Instant>,
    frames: u32,
    frames_at: Instant,
    cpu: f32,
    mem: f32,
    fps: f32,
    gpu: f32,
    ncpu: f32,
    gpu_src: Gpu,
    ping: PingWatch,
    slots: HashMap<String, ProcStat>,
    watch_keys: Vec<String>,
    reshade_pid: u32,
    reshade_at: Option<Instant>,
    reshade_mb: Option<f32>,
}

impl Default for Sampler {
    fn default() -> Self {
        Self {
            last_idle: 0,
            last_kern: 0,
            last_user: 0,
            last_sample: None,
            last_seq: 0,
            last_seq_at: None,
            frames: 0,
            frames_at: Instant::now(),
            cpu: 0.0,
            mem: 0.0,
            fps: 0.0,
            gpu: 0.0,
            ncpu: std::thread::available_parallelism()
                .map(|n| n.get() as f32)
                .unwrap_or(1.0)
                .max(1.0),
            gpu_src: Gpu::default(),
            ping: PingWatch::new(),
            slots: HashMap::new(),
            watch_keys: Vec::new(),
            reshade_pid: 0,
            reshade_at: None,
            reshade_mb: None,
        }
    }
}

impl Sampler {
    pub fn tick(&mut self, game_seq: Option<u32>, want_meters: bool, apps: &[SysApp]) {
        self.note_fps(game_seq);
        self.ping.set_live(want_meters);
        if !want_meters {
            return;
        }
        let shown: Vec<&SysApp> = apps.iter().filter(|a| a.show).take(SYS_PROC_MAX).collect();
        let keys: Vec<String> = shown.iter().map(|a| a.key.clone()).collect();
        let now = Instant::now();
        let stale = self.last_sample.is_some_and(|t| now.duration_since(t) < Duration::from_millis(500));
        if stale && keys == self.watch_keys {
            self.push(&shown);
            return;
        }
        let dt = self
            .last_sample
            .map(|t| now.duration_since(t).as_secs_f32())
            .unwrap_or(0.25)
            .max(0.05);
        self.last_sample = Some(now);
        self.watch_keys = keys;
        let (mem, total_mb) = memory();
        self.mem = mem;
        self.cpu = self.cpu_pct();
        let gpu = self.gpu_src.sample();
        self.gpu = gpu.pct;
        self.sample_procs(dt, total_mb, &gpu.by_pid, gpu.pid_ok, &shown);
        self.push(&shown);
    }

    fn push(&self, shown: &[&SysApp]) {
        mxbo_hud::set_sys_stats(self.cpu, self.mem, self.fps, self.gpu, self.ping.ms());
        let procs = shown
            .iter()
            .map(|a| {
                self.slots
                    .get(&a.key)
                    .copied()
                    .unwrap_or_default()
                    .pack(&a.label)
            })
            .collect();
        mxbo_hud::set_sys_procs(procs);
    }

    fn sample_procs(
        &mut self,
        dt: f32,
        total_mb: f32,
        gpu_by_pid: &HashMap<u32, f32>,
        gpu_ok: bool,
        shown: &[&SysApp],
    ) {
        let ncpu = self.ncpu;
        let self_pid = std::process::id();
        let procs = snapshot_procs();
        let mx = procs
            .iter()
            .find(|(_, name)| name == "mxbikes.exe")
            .map(|(pid, _)| *pid);
        self.ping.set_mx(mx);

        let mut live = std::collections::HashSet::new();
        for app in shown {
            live.insert(app.key.clone());
            let pid = pid_for(app, self_pid, &procs);
            let dll_mb = if app.kind == SysAppKind::Reshade {
                mx.and_then(|p| self.reshade_cached(p)).unwrap_or(0.0)
            } else {
                0.0
            };
            let slot = self.slots.entry(app.key.clone()).or_default();
            if app.kind == SysAppKind::Reshade && dll_mb > 0.05 {
                *slot = ProcStat {
                    pid: 0,
                    last_cpu: 0,
                    cpu: -1.0,
                    gpu: -1.0,
                    mem_mb: dll_mb,
                    mem_pct: if total_mb > 0.0 {
                        (dll_mb / total_mb) * 100.0
                    } else {
                        0.0
                    },
                    on: true,
                };
            } else {
                refresh_slot(slot, pid, dt, ncpu, total_mb);
            }
            apply_gpu(slot, gpu_by_pid, gpu_ok);
        }
        self.slots.retain(|k, _| live.contains(k));
    }

    /// Toolhelp `SNAPMODULE` on mxbikes.exe can stall the game. Probe at most every 30s.
    fn reshade_cached(&mut self, pid: u32) -> Option<f32> {
        const TTL: Duration = Duration::from_secs(30);
        if self.reshade_pid == pid && self.reshade_at.is_some_and(|t| t.elapsed() < TTL) {
            return self.reshade_mb;
        }
        let mb = reshade_mb(pid);
        self.reshade_pid = pid;
        self.reshade_at = Some(Instant::now());
        self.reshade_mb = mb;
        mb
    }

    fn note_fps(&mut self, game_seq: Option<u32>) {
        let now = Instant::now();
        if let Some(seq) = game_seq {
            // SHM `seq` is a seqlock, not a frame counter: each plugin Draw
            // publish does odd (lock) then even (unlock), so it advances by 2.
            // Using the raw delta made 70 game FPS read as 140, and falling
            // through to the overlay frame counter then overwrote that with
            // the HUD loop (~38), so the meter bounced between the two.
            if let Some(prev) = self.last_seq_at {
                let dt = now.duration_since(prev).as_secs_f32();
                if dt >= 0.35 {
                    let n = shm_publishes(self.last_seq, seq);
                    if n > 0 && n < 400 {
                        self.fps = n as f32 / dt;
                    }
                    self.last_seq = seq;
                    self.last_seq_at = Some(now);
                }
            } else {
                self.last_seq = seq;
                self.last_seq_at = Some(now);
            }
            return;
        }
        self.last_seq_at = None;
        self.frames += 1;
        let dt = now.duration_since(self.frames_at).as_secs_f32();
        if dt >= 0.4 {
            self.fps = self.frames as f32 / dt;
            self.frames = 0;
            self.frames_at = now;
        }
    }

    fn cpu_pct(&mut self) -> f32 {
        let mut idle = FILETIME::default();
        let mut kern = FILETIME::default();
        let mut user = FILETIME::default();
        unsafe {
            if GetSystemTimes(Some(&mut idle), Some(&mut kern), Some(&mut user)).is_err() {
                return self.cpu;
            }
        }
        let idle = ft(idle);
        let kern = ft(kern);
        let user = ft(user);
        let di = idle.saturating_sub(self.last_idle);
        let dk = kern.saturating_sub(self.last_kern);
        let du = user.saturating_sub(self.last_user);
        self.last_idle = idle;
        self.last_kern = kern;
        self.last_user = user;
        let total = dk.saturating_add(du);
        if total == 0 {
            return self.cpu;
        }
        let busy = total.saturating_sub(di) as f32 / total as f32;
        (busy * 100.0).clamp(0.0, 100.0)
    }
}

/// Seqlock `seq` advances by 2 per plugin Draw publish (odd lock, even unlock).
fn shm_publishes(prev: u32, next: u32) -> u32 {
    next.wrapping_sub(prev) / 2
}

fn is_mxb_app_main(name: &str) -> bool {
    name == "frost.exe" || name == "mxb app.exe" || name == "mxb-app.exe" || name == "mxbapp.exe"
}

fn snapshot_procs() -> Vec<(u32, String)> {
    let mut out = Vec::new();
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return out;
        };
        let mut pe = PROCESSENTRY32W {
            dwSize: size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        if Process32FirstW(snap, &mut pe).is_ok() {
            loop {
                let pid = pe.th32ProcessID;
                if pid != 0 {
                    let len = pe
                        .szExeFile
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(pe.szExeFile.len());
                    let name = String::from_utf16_lossy(&pe.szExeFile[..len]).to_ascii_lowercase();
                    out.push((pid, name));
                }
                if Process32NextW(snap, &mut pe).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snap);
    }
    out
}

pub(crate) fn pid_for(app: &SysApp, self_pid: u32, procs: &[(u32, String)]) -> Option<u32> {
    match app.kind {
        SysAppKind::Hud => Some(self_pid),
        SysAppKind::Mxbikes => procs
            .iter()
            .find(|(_, name)| name == "mxbikes.exe")
            .map(|(pid, _)| *pid),
        SysAppKind::MxbApp => {
            let main = procs
                .iter()
                .find(|(_, name)| is_mxb_app_main(name))
                .map(|(pid, _)| *pid);
            let frostmod = procs
                .iter()
                .find(|(_, name)| name == "frostmod.exe")
                .map(|(pid, _)| *pid);
            main.or(frostmod)
        }
        SysAppKind::Reshade => procs
            .iter()
            .find(|(_, name)| name.contains("reshade") && name.ends_with(".exe"))
            .map(|(pid, _)| *pid),
        // Toolhelp `szExeFile` is the basename only, so Program Files vs a
        // portable folder vs Steam still match. We never store an install path.
        SysAppKind::Exe => procs
            .iter()
            .find(|(_, name)| app.names.iter().any(|want| want == name))
            .map(|(pid, _)| *pid),
    }
}

fn apply_gpu(slot: &mut ProcStat, by_pid: &HashMap<u32, f32>, pid_ok: bool) {
    if !slot.on {
        slot.gpu = 0.0;
        return;
    }
    if !pid_ok || slot.pid == 0 {
        slot.gpu = -1.0;
        return;
    }
    slot.gpu = by_pid.get(&slot.pid).copied().unwrap_or(0.0);
}

fn refresh_slot(slot: &mut ProcStat, pid: Option<u32>, dt: f32, ncpu: f32, total_mb: f32) {
    let Some(pid) = pid else {
        *slot = ProcStat::default();
        return;
    };
    if slot.pid != pid {
        slot.pid = pid;
        slot.last_cpu = 0;
        slot.cpu = 0.0;
    }
    let Some((cpu_time, mem_mb)) = query_pid(pid) else {
        slot.on = false;
        return;
    };
    slot.on = true;
    slot.mem_mb = mem_mb;
    slot.mem_pct = if total_mb > 0.0 {
        (mem_mb / total_mb) * 100.0
    } else {
        0.0
    };
    if slot.last_cpu == 0 {
        slot.last_cpu = cpu_time;
        return;
    }
    let delta = cpu_time.saturating_sub(slot.last_cpu) as f32;
    slot.last_cpu = cpu_time;
    let wall = dt * 10_000_000.0 * ncpu.max(1.0);
    if wall > 0.0 {
        slot.cpu = (delta / wall * 100.0).clamp(0.0, 100.0);
    }
}

fn query_pid(pid: u32) -> Option<(u64, f32)> {
    unsafe {
        if pid == std::process::id() {
            return query_handle(GetCurrentProcess());
        }
        let proc = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let out = query_handle(proc);
        let _ = CloseHandle(proc);
        out
    }
}

fn query_handle(proc: windows::Win32::Foundation::HANDLE) -> Option<(u64, f32)> {
    unsafe {
        let mut create = FILETIME::default();
        let mut exit = FILETIME::default();
        let mut kern = FILETIME::default();
        let mut user = FILETIME::default();
        GetProcessTimes(proc, &mut create, &mut exit, &mut kern, &mut user).ok()?;
        let mut mem = PROCESS_MEMORY_COUNTERS {
            cb: size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
            ..Default::default()
        };
        if !K32GetProcessMemoryInfo(proc, &mut mem, mem.cb).as_bool() {
            return None;
        }
        Some((
            ft(kern).saturating_add(ft(user)),
            mem.WorkingSetSize as f32 / (1024.0 * 1024.0),
        ))
    }
}

fn reshade_mb(pid: u32) -> Option<f32> {
    reshade_dll_mb(pid).or_else(|| reshade_game_dir_mb(pid))
}

fn is_windows_dir(path: &str) -> bool {
    let p = path.replace('/', "\\").to_ascii_lowercase();
    p.contains("\\windows\\system32\\")
        || p.contains("\\windows\\syswow64\\")
        || p.contains("\\windows\\winsxs\\")
}

fn is_reshade_module(name: &str, path: &str) -> bool {
    let name = name.to_ascii_lowercase();
    if name.contains("reshade") {
        return true;
    }
    let hook = matches!(
        name.as_str(),
        "opengl32.dll" | "dxgi.dll" | "d3d9.dll" | "d3d10.dll" | "d3d11.dll" | "d3d12.dll"
    );
    hook && !path.is_empty() && !is_windows_dir(path)
}

fn reshade_dll_mb(pid: u32) -> Option<f32> {
    unsafe {
        let flags = TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32;
        let snap = CreateToolhelp32Snapshot(flags, pid).ok()?;
        let mut me = MODULEENTRY32W {
            dwSize: size_of::<MODULEENTRY32W>() as u32,
            ..Default::default()
        };
        let mut bytes = 0u64;
        if Module32FirstW(snap, &mut me).is_ok() {
            loop {
                let nlen = me
                    .szModule
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(me.szModule.len());
                let plen = me
                    .szExePath
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(me.szExePath.len());
                let name = String::from_utf16_lossy(&me.szModule[..nlen]);
                let path = String::from_utf16_lossy(&me.szExePath[..plen]);
                if is_reshade_module(&name, &path) {
                    bytes = bytes.saturating_add(me.modBaseSize as u64);
                }
                if Module32NextW(snap, &mut me).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snap);
        if bytes == 0 {
            None
        } else {
            Some(bytes as f32 / (1024.0 * 1024.0))
        }
    }
}

fn reshade_game_dir_mb(pid: u32) -> Option<f32> {
    let exe = crate::compat::exe_path_for_pid(pid)?;
    let dir = std::path::Path::new(&exe).parent()?;
    let mut bytes = 0u64;
    let mut found = false;
    if dir.join("ReShade.ini").is_file() || dir.join("reshade.ini").is_file() {
        found = true;
    }
    if let Ok(rd) = std::fs::read_dir(dir) {
        for ent in rd.flatten() {
            let name = ent.file_name().to_string_lossy().to_ascii_lowercase();
            let hook = name.contains("reshade") && name.ends_with(".dll")
                || matches!(
                    name.as_str(),
                    "opengl32.dll" | "dxgi.dll" | "d3d9.dll" | "d3d10.dll" | "d3d11.dll" | "d3d12.dll"
                );
            if !hook {
                continue;
            }
            found = true;
            if let Ok(meta) = ent.metadata() {
                bytes = bytes.saturating_add(meta.len());
            }
        }
    }
    if !found {
        return None;
    }
    Some((bytes as f32 / (1024.0 * 1024.0)).max(0.1))
}

fn ft(t: FILETIME) -> u64 {
    ((t.dwHighDateTime as u64) << 32) | t.dwLowDateTime as u64
}

fn memory() -> (f32, f32) {
    let mut info = MEMORYSTATUSEX {
        dwLength: size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    unsafe {
        if GlobalMemoryStatusEx(&mut info).is_err() {
            return (0.0, 0.0);
        }
    }
    (
        info.dwMemoryLoad as f32,
        info.ullTotalPhys as f32 / (1024.0 * 1024.0),
    )
}

#[cfg(test)]
#[path = "tests/sys.rs"]
mod tests;
