use std::mem::size_of;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{CloseHandle, FILETIME};
use windows::Win32::NetworkManagement::IpHelper::{FreeMibTable, GetIfTable2, IF_TYPE_SOFTWARE_LOOPBACK};
use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Module32FirstW, Module32NextW, Process32FirstW, Process32NextW,
    MODULEENTRY32W, PROCESSENTRY32W, TH32CS_SNAPMODULE, TH32CS_SNAPMODULE32, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetProcessTimes, GetSystemTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
};

use mxbo_hud::SysProc;

#[derive(Clone, Copy, Default)]
struct ProcStat {
    pid: u32,
    last_cpu: u64,
    cpu: f32,
    mem_mb: f32,
    mem_pct: f32,
    on: bool,
}

impl ProcStat {
    fn pack(self) -> SysProc {
        SysProc {
            cpu: self.cpu,
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
    last_octets: u64,
    last_sample: Option<Instant>,
    last_seq: u32,
    last_seq_at: Option<Instant>,
    frames: u32,
    frames_at: Instant,
    cpu: f32,
    mem: f32,
    fps: f32,
    net: f32,
    ncpu: f32,
    hud: ProcStat,
    mx: ProcStat,
    mxb_app: ProcStat,
    reshade: ProcStat,
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
            last_octets: 0,
            last_sample: None,
            last_seq: 0,
            last_seq_at: None,
            frames: 0,
            frames_at: Instant::now(),
            cpu: 0.0,
            mem: 0.0,
            fps: 0.0,
            net: 0.0,
            ncpu: std::thread::available_parallelism()
                .map(|n| n.get() as f32)
                .unwrap_or(1.0)
                .max(1.0),
            hud: ProcStat::default(),
            mx: ProcStat::default(),
            mxb_app: ProcStat::default(),
            reshade: ProcStat::default(),
            reshade_pid: 0,
            reshade_at: None,
            reshade_mb: None,
        }
    }
}

impl Sampler {
    pub fn tick(&mut self, game_seq: Option<u32>, want_meters: bool) {
        self.note_fps(game_seq);
        if !want_meters {
            return;
        }
        let now = Instant::now();
        if self
            .last_sample
            .is_some_and(|t| now.duration_since(t) < Duration::from_millis(500))
        {
            self.push();
            return;
        }
        let dt = self
            .last_sample
            .map(|t| now.duration_since(t).as_secs_f32())
            .unwrap_or(0.25)
            .max(0.05);
        self.last_sample = Some(now);
        let (mem, total_mb) = memory();
        self.mem = mem;
        self.cpu = self.cpu_pct();
        self.net = self.net_pct(dt);
        self.sample_procs(dt, total_mb);
        self.push();
    }

    fn push(&self) {
        mxbo_hud::set_sys_stats(self.cpu, self.mem, self.fps, self.net);
        mxbo_hud::set_sys_procs([
            self.hud.pack(),
            self.mx.pack(),
            self.mxb_app.pack(),
            self.reshade.pack(),
        ]);
    }

    fn sample_procs(&mut self, dt: f32, total_mb: f32) {
        let ncpu = self.ncpu;
        let self_pid = std::process::id();
        let found = find_pids(self_pid);
        refresh_slot(&mut self.hud, Some(self_pid), dt, ncpu, total_mb);
        refresh_slot(&mut self.mx, found.mx, dt, ncpu, total_mb);
        refresh_slot(&mut self.mxb_app, found.mxb_app, dt, ncpu, total_mb);

        let mut dll_mb = 0.0f32;
        if let Some(pid) = found.mx {
            if let Some(mb) = self.reshade_cached(pid) {
                dll_mb = mb;
            }
        }
        if dll_mb > 0.05 {
            self.reshade = ProcStat {
                pid: 0,
                last_cpu: 0,
                cpu: -1.0,
                mem_mb: dll_mb,
                mem_pct: if total_mb > 0.0 {
                    (dll_mb / total_mb) * 100.0
                } else {
                    0.0
                },
                on: true,
            };
        } else {
            refresh_slot(&mut self.reshade, found.reshade, dt, ncpu, total_mb);
        }
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

    fn net_pct(&mut self, dt: f32) -> f32 {
        let (octets, speed) = net_totals();
        let prev = self.last_octets;
        self.last_octets = octets;
        if prev == 0 || speed == 0 || octets < prev {
            return self.net;
        }
        let bits = (octets - prev) as f32 * 8.0 / dt;
        ((bits / speed as f32) * 100.0).clamp(0.0, 100.0)
    }
}

/// Seqlock `seq` advances by 2 per plugin Draw publish (odd lock, even unlock).
fn shm_publishes(prev: u32, next: u32) -> u32 {
    next.wrapping_sub(prev) / 2
}

struct FoundPids {
    mx: Option<u32>,
    mxb_app: Option<u32>,
    reshade: Option<u32>,
}

fn is_mxb_app_main(name: &str) -> bool {
    name == "frost.exe" || name == "mxb app.exe" || name == "mxb-app.exe" || name == "mxbapp.exe"
}

fn find_pids(self_pid: u32) -> FoundPids {
    let mut found = FoundPids {
        mx: None,
        mxb_app: None,
        reshade: None,
    };
    let mut frostmod = None;
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return found;
        };
        let mut pe = PROCESSENTRY32W {
            dwSize: size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        if Process32FirstW(snap, &mut pe).is_ok() {
            loop {
                let pid = pe.th32ProcessID;
                if pid != 0 && pid != self_pid {
                    let len = pe
                        .szExeFile
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(pe.szExeFile.len());
                    let name = String::from_utf16_lossy(&pe.szExeFile[..len]).to_ascii_lowercase();
                    if found.mx.is_none() && name == "mxbikes.exe" {
                        found.mx = Some(pid);
                    } else if is_mxb_app_main(&name) {
                        found.mxb_app = Some(pid);
                    } else if frostmod.is_none() && name == "frostmod.exe" {
                        frostmod = Some(pid);
                    } else if found.reshade.is_none()
                        && name.contains("reshade")
                        && name.ends_with(".exe")
                    {
                        found.reshade = Some(pid);
                    }
                }
                if Process32NextW(snap, &mut pe).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snap);
    }
    if found.mxb_app.is_none() {
        found.mxb_app = frostmod;
    }
    found
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

fn net_totals() -> (u64, u64) {
    unsafe {
        let mut table = std::ptr::null_mut();
        if GetIfTable2(&mut table).is_err() || table.is_null() {
            return (0, 0);
        }
        let n = (*table).NumEntries as usize;
        let rows = std::slice::from_raw_parts((*table).Table.as_ptr(), n);
        let mut best_octets = 0u64;
        let mut best_speed = 0u64;
        for row in rows {
            if row.Type == IF_TYPE_SOFTWARE_LOOPBACK || row.OperStatus != IfOperStatusUp {
                continue;
            }
            if row.TransmitLinkSpeed == 0 {
                continue;
            }
            let octets = row.InOctets.saturating_add(row.OutOctets);
            if octets >= best_octets {
                best_octets = octets;
                best_speed = row.TransmitLinkSpeed;
            }
        }
        FreeMibTable(table as *const _);
        (best_octets, best_speed)
    }
}

#[cfg(test)]
#[path = "tests/sys.rs"]
mod tests;
