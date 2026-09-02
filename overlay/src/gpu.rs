use std::collections::HashMap;
use std::ffi::c_void;
use std::mem::size_of;

use windows::core::{w, PCSTR, PCWSTR, PWSTR};
use windows::Win32::Foundation::{FreeLibrary, HMODULE};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::Win32::System::Performance::{
    PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
    PdhOpenQueryW, PDH_CSTATUS_NEW_DATA, PDH_CSTATUS_VALID_DATA, PDH_FMT_COUNTERVALUE_ITEM_W,
    PDH_FMT_DOUBLE, PDH_MORE_DATA,
};

/// Host GPU engine load 0–100 plus per-pid 3D/Graphics/Compute when PDH has them.
pub struct Gpu {
    nvidia: Option<Nvml>,
    amd: Option<Adl>,
    pdh: Option<PdhGpu>,
    probed: bool,
}

#[derive(Default)]
pub struct GpuSample {
    pub pct: f32,
    pub by_pid: HashMap<u32, f32>,
    pub pid_ok: bool,
}

impl Default for Gpu {
    fn default() -> Self {
        Self {
            nvidia: None,
            amd: None,
            pdh: None,
            probed: false,
        }
    }
}

impl Gpu {
    pub fn sample(&mut self) -> GpuSample {
        if !self.probed {
            self.nvidia = Nvml::open();
            self.amd = Adl::open();
            self.pdh = PdhGpu::open();
            self.probed = true;
        }
        let mut best = 0.0f32;
        if let Some(n) = &self.nvidia {
            if let Some(v) = n.pct() {
                best = best.max(v);
            }
        }
        if let Some(a) = &self.amd {
            if let Some(v) = a.pct() {
                best = best.max(v);
            }
        }
        let (pdh_pct, by_pid, pid_ok, pdh_render) = match &mut self.pdh {
            Some(p) => {
                let samples = p.samples();
                let pid_ok = samples
                    .iter()
                    .any(|(n, _)| engine_is_per_pid(n) && engine_key(n).is_some());
                let pdh_render = samples.iter().any(|(n, _)| engine_is_render(n));
                (
                    gpu_engine_pct(samples.iter().map(|(n, v)| (n.as_str(), *v))),
                    gpu_pid_pcts(samples.iter().map(|(n, v)| (n.as_str(), *v))),
                    pid_ok,
                    pdh_render,
                )
            }
            None => (0.0, HashMap::new(), false, false),
        };
        // Task Manager's GPU graph is 3D. Compute can sit at 100% with HAGS.
        // Prefer that PDH 3D/Graphics reading over vendor or Compute.
        let pct = if pdh_render {
            pdh_pct
        } else {
            best.max(pdh_pct)
        };
        GpuSample {
            pct: pct.clamp(0.0, 100.0),
            by_pid,
            pid_ok,
        }
    }
}

/// Per card: 3D / Graphics if that card has one, else Compute. Hottest card
/// wins. Per-process instances are summed per engine; a pid-less instance is
/// already the engine total. Copy / video ignored.
pub(crate) fn gpu_engine_pct<I, S>(samples: I) -> f32
where
    I: IntoIterator<Item = (S, f32)>,
    S: AsRef<str>,
{
    struct Acc {
        total: Option<f32>,
        pid_sum: f32,
        render: bool,
    }
    let mut by_engine: HashMap<String, Acc> = HashMap::new();
    for (name, pct) in samples {
        let name = name.as_ref();
        let Some(key) = engine_key(name) else {
            continue;
        };
        let render = engine_is_render(name);
        let acc = by_engine.entry(key).or_insert(Acc {
            total: None,
            pid_sum: 0.0,
            render,
        });
        if engine_is_per_pid(name) {
            acc.pid_sum += pct;
        } else {
            acc.total = Some(acc.total.unwrap_or(0.0).max(pct));
        }
    }
    struct Card {
        render: Option<f32>,
        compute: Option<f32>,
    }
    let mut by_card: HashMap<String, Card> = HashMap::new();
    for (key, acc) in by_engine {
        let v = acc.total.unwrap_or(acc.pid_sum.clamp(0.0, 100.0));
        let card = by_card.entry(card_key(&key).to_string()).or_insert(Card {
            render: None,
            compute: None,
        });
        if acc.render {
            card.render = Some(card.render.unwrap_or(0.0).max(v));
        } else {
            card.compute = Some(card.compute.unwrap_or(0.0).max(v));
        }
    }
    by_card
        .values()
        .map(|c| c.render.unwrap_or(c.compute.unwrap_or(0.0)))
        .fold(0.0f32, f32::max)
        .clamp(0.0, 100.0)
}

/// 3D / Graphics per pid when that pid has one; else Compute. Copy / video ignored.
pub(crate) fn gpu_pid_pcts<I, S>(samples: I) -> HashMap<u32, f32>
where
    I: IntoIterator<Item = (S, f32)>,
    S: AsRef<str>,
{
    struct Acc {
        render: Option<f32>,
        compute: Option<f32>,
    }
    let mut by_pid: HashMap<u32, Acc> = HashMap::new();
    for (name, pct) in samples {
        let name = name.as_ref();
        if engine_key(name).is_none() {
            continue;
        }
        let Some(pid) = parse_engine_pid(name) else {
            continue;
        };
        let pct = pct.clamp(0.0, 100.0);
        let acc = by_pid.entry(pid).or_insert(Acc {
            render: None,
            compute: None,
        });
        if engine_is_render(name) {
            acc.render = Some(acc.render.unwrap_or(0.0).max(pct));
        } else {
            acc.compute = Some(acc.compute.unwrap_or(0.0).max(pct));
        }
    }
    by_pid
        .into_iter()
        .map(|(pid, acc)| (pid, acc.render.unwrap_or(acc.compute.unwrap_or(0.0))))
        .collect()
}

fn parse_engine_pid(name: &str) -> Option<u32> {
    if !engine_is_per_pid(name) {
        return None;
    }
    let rest = &name[4..];
    let end = rest.find('_').unwrap_or(rest.len());
    rest[..end].parse().ok()
}

fn engine_is_per_pid(name: &str) -> bool {
    name.len() >= 4 && name[..4].eq_ignore_ascii_case("pid_")
}

fn engine_is_render(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("engtype_3d") || lower.contains("engtype_graphics")
}

fn card_key(engine: &str) -> &str {
    engine.split_once("_eng_").map(|(c, _)| c).unwrap_or(engine)
}

fn engine_key(name: &str) -> Option<String> {
    let lower = name.to_ascii_lowercase();
    let work = lower.contains("engtype_3d")
        || lower.contains("engtype_graphics")
        || lower.contains("engtype_compute");
    if !work {
        return None;
    }
    let rest = if let Some(after) = lower.strip_prefix("pid_") {
        after
            .find('_')
            .map(|i| &after[i + 1..])
            .unwrap_or(after)
    } else {
        lower.as_str()
    };
    Some(rest.to_string())
}

fn load_proc<T>(lib: HMODULE, name: &[u8]) -> Option<T> {
    unsafe {
        let p = GetProcAddress(lib, PCSTR::from_raw(name.as_ptr()))?;
        Some(std::mem::transmute_copy(&p))
    }
}

fn try_load(path: PCWSTR) -> Option<HMODULE> {
    unsafe { LoadLibraryW(path).ok() }
}

type NvmlInit = unsafe extern "C" fn() -> i32;
type NvmlShutdown = unsafe extern "C" fn() -> i32;
type NvmlCount = unsafe extern "C" fn(*mut u32) -> i32;
type NvmlHandle = unsafe extern "C" fn(u32, *mut *mut c_void) -> i32;
type NvmlUtil = unsafe extern "C" fn(*mut c_void, *mut NvmlUtilization) -> i32;

#[repr(C)]
struct NvmlUtilization {
    gpu: u32,
    memory: u32,
}

struct Nvml {
    lib: HMODULE,
    shutdown: NvmlShutdown,
    count: NvmlCount,
    handle: NvmlHandle,
    util: NvmlUtil,
}

impl Nvml {
    fn open() -> Option<Self> {
        let lib = try_load(w!("nvml.dll"))
            .or_else(|| try_load(w!("C:\\Windows\\System32\\nvml.dll")))
            .or_else(|| {
                try_load(w!(
                    "C:\\Program Files\\NVIDIA Corporation\\NVSMI\\nvml.dll"
                ))
            })?;
        let close = |lib: HMODULE| unsafe {
            let _ = FreeLibrary(lib);
        };
        let Some(init) = load_proc::<NvmlInit>(lib, b"nvmlInit_v2\0")
            .or_else(|| load_proc(lib, b"nvmlInit\0"))
        else {
            close(lib);
            return None;
        };
        let Some(shutdown) = load_proc::<NvmlShutdown>(lib, b"nvmlShutdown\0") else {
            close(lib);
            return None;
        };
        let Some(count) = load_proc::<NvmlCount>(lib, b"nvmlDeviceGetCount_v2\0")
            .or_else(|| load_proc(lib, b"nvmlDeviceGetCount\0"))
        else {
            close(lib);
            return None;
        };
        let Some(handle) = load_proc::<NvmlHandle>(lib, b"nvmlDeviceGetHandleByIndex_v2\0")
            .or_else(|| load_proc(lib, b"nvmlDeviceGetHandleByIndex\0"))
        else {
            close(lib);
            return None;
        };
        let Some(util) = load_proc::<NvmlUtil>(lib, b"nvmlDeviceGetUtilizationRates\0") else {
            close(lib);
            return None;
        };
        if unsafe { init() } != 0 {
            close(lib);
            return None;
        }
        Some(Self {
            lib,
            shutdown,
            count,
            handle,
            util,
        })
    }

    fn pct(&self) -> Option<f32> {
        let mut n = 0u32;
        if unsafe { (self.count)(&mut n) } != 0 || n == 0 {
            return None;
        }
        let mut best = None;
        for i in 0..n {
            let mut dev = std::ptr::null_mut();
            if unsafe { (self.handle)(i, &mut dev) } != 0 || dev.is_null() {
                continue;
            }
            let mut u = NvmlUtilization { gpu: 0, memory: 0 };
            if unsafe { (self.util)(dev, &mut u) } != 0 {
                continue;
            }
            let v = u.gpu as f32;
            best = Some(best.unwrap_or(0.0f32).max(v));
        }
        best
    }
}

impl Drop for Nvml {
    fn drop(&mut self) {
        unsafe {
            let _ = (self.shutdown)();
            let _ = FreeLibrary(self.lib);
        }
    }
}

const AMD_OK: i32 = 0;
const AMD_PMLOG_GFX: usize = 16;
const AMD_PMLOG_SENSORS: usize = 256;

type AdlMalloc = unsafe extern "system" fn(i32) -> *mut c_void;
type AdlCreate = unsafe extern "C" fn(AdlMalloc, i32, *mut *mut c_void) -> i32;
type AdlDestroy = unsafe extern "C" fn(*mut c_void) -> i32;
type AdlNum = unsafe extern "C" fn(*mut c_void, *mut i32) -> i32;
type AdlInfo = unsafe extern "C" fn(*mut c_void, *mut AdapterInfo, i32) -> i32;
type AdlActive = unsafe extern "C" fn(*mut c_void, i32, *mut i32) -> i32;
type AdlPmLog = unsafe extern "C" fn(*mut c_void, i32, *mut AdlPmLogOut) -> i32;
type AdlOdn = unsafe extern "C" fn(*mut c_void, i32, *mut AdlOdnStatus) -> i32;
type AdlOd5 = unsafe extern "C" fn(*mut c_void, i32, *mut AdlOd5Activity) -> i32;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

unsafe extern "system" fn adl_malloc(size: i32) -> *mut c_void {
    if size <= 0 {
        std::ptr::null_mut()
    } else {
        malloc(size as usize)
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct AdapterInfo {
    size: i32,
    adapter_index: i32,
    _udid: [u8; 256],
    _bus: i32,
    _device: i32,
    _function: i32,
    vendor_id: i32,
    _adapter_name: [u8; 256],
    _display_name: [u8; 256],
    present: i32,
    exist: i32,
    _driver_path: [u8; 256],
    _driver_path_ext: [u8; 256],
    _pnp: [u8; 256],
    _os_display_index: i32,
}

impl Default for AdapterInfo {
    fn default() -> Self {
        Self {
            size: size_of::<Self>() as i32,
            adapter_index: 0,
            _udid: [0; 256],
            _bus: 0,
            _device: 0,
            _function: 0,
            vendor_id: 0,
            _adapter_name: [0; 256],
            _display_name: [0; 256],
            present: 0,
            exist: 0,
            _driver_path: [0; 256],
            _driver_path_ext: [0; 256],
            _pnp: [0; 256],
            _os_display_index: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct AdlSensor {
    supported: i32,
    value: i32,
}

#[repr(C)]
struct AdlPmLogOut {
    size: i32,
    sensors: [AdlSensor; AMD_PMLOG_SENSORS],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct AdlOdnStatus {
    _core: i32,
    _mem: i32,
    _dcef: i32,
    _gfx: i32,
    _uvd: i32,
    _vce: i32,
    gpu_activity: i32,
    _rest: [i32; 11],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct AdlOd5Activity {
    size: i32,
    _engine: i32,
    _mem: i32,
    _vddc: i32,
    activity: i32,
    _rest: [i32; 5],
}

struct Adl {
    lib: HMODULE,
    ctx: *mut c_void,
    destroy: AdlDestroy,
    num: AdlNum,
    info: AdlInfo,
    active: Option<AdlActive>,
    pmlog: Option<AdlPmLog>,
    odn: Option<AdlOdn>,
    od5: Option<AdlOd5>,
}

impl Adl {
    fn open() -> Option<Self> {
        let lib = try_load(w!("atiadlxx.dll"))?;
        let create: AdlCreate = match load_proc(lib, b"ADL2_Main_Control_Create\0") {
            Some(f) => f,
            None => {
                unsafe {
                    let _ = FreeLibrary(lib);
                }
                return None;
            }
        };
        let destroy: AdlDestroy = match load_proc(lib, b"ADL2_Main_Control_Destroy\0") {
            Some(f) => f,
            None => {
                unsafe {
                    let _ = FreeLibrary(lib);
                }
                return None;
            }
        };
        let num: AdlNum = match load_proc(lib, b"ADL2_Adapter_NumberOfAdapters_Get\0") {
            Some(f) => f,
            None => {
                unsafe {
                    let _ = FreeLibrary(lib);
                }
                return None;
            }
        };
        let info: AdlInfo = match load_proc(lib, b"ADL2_Adapter_AdapterInfo_Get\0") {
            Some(f) => f,
            None => {
                unsafe {
                    let _ = FreeLibrary(lib);
                }
                return None;
            }
        };
        let mut ctx = std::ptr::null_mut();
        if unsafe { create(adl_malloc, 1, &mut ctx) } != AMD_OK || ctx.is_null() {
            unsafe {
                let _ = FreeLibrary(lib);
            }
            return None;
        }
        Some(Self {
            lib,
            ctx,
            destroy,
            num,
            info,
            active: load_proc(lib, b"ADL2_Adapter_Active_Get\0"),
            pmlog: load_proc(lib, b"ADL2_New_QueryPMLogData_Get\0"),
            odn: load_proc(lib, b"ADL2_OverdriveN_PerformanceStatus_Get\0"),
            od5: load_proc(lib, b"ADL2_Overdrive5_CurrentActivity_Get\0"),
        })
    }

    fn pct(&self) -> Option<f32> {
        let mut n = 0i32;
        if unsafe { (self.num)(self.ctx, &mut n) } != AMD_OK || n <= 0 {
            return None;
        }
        let n = n as usize;
        let mut infos = vec![AdapterInfo::default(); n];
        let bytes = (n * size_of::<AdapterInfo>()) as i32;
        if unsafe { (self.info)(self.ctx, infos.as_mut_ptr(), bytes) } != AMD_OK {
            return None;
        }
        let mut best = None;
        for row in &infos {
            if row.present == 0 && row.exist == 0 {
                continue;
            }
            if row.vendor_id != 0 && row.vendor_id != 0x1002 && row.vendor_id != 1002 {
                continue;
            }
            if let Some(active) = self.active {
                let mut on = 0i32;
                if unsafe { active(self.ctx, row.adapter_index, &mut on) } == AMD_OK && on == 0 {
                    continue;
                }
            }
            if let Some(v) = self.adapter_pct(row.adapter_index) {
                best = Some(best.unwrap_or(0.0f32).max(v));
            }
        }
        best
    }

    fn adapter_pct(&self, index: i32) -> Option<f32> {
        if let Some(pmlog) = self.pmlog {
            let mut out = AdlPmLogOut {
                size: size_of::<AdlPmLogOut>() as i32,
                sensors: [AdlSensor {
                    supported: 0,
                    value: 0,
                }; AMD_PMLOG_SENSORS],
            };
            if unsafe { pmlog(self.ctx, index, &mut out) } == AMD_OK {
                if let Some(s) = out.sensors.get(AMD_PMLOG_GFX) {
                    if s.supported != 0 {
                        return Some((s.value as f32).clamp(0.0, 100.0));
                    }
                }
            }
        }
        if let Some(odn) = self.odn {
            let mut st = AdlOdnStatus {
                _core: 0,
                _mem: 0,
                _dcef: 0,
                _gfx: 0,
                _uvd: 0,
                _vce: 0,
                gpu_activity: 0,
                _rest: [0; 11],
            };
            if unsafe { odn(self.ctx, index, &mut st) } == AMD_OK && st.gpu_activity >= 0 {
                return Some((st.gpu_activity as f32).clamp(0.0, 100.0));
            }
        }
        if let Some(od5) = self.od5 {
            let mut act = AdlOd5Activity {
                size: size_of::<AdlOd5Activity>() as i32,
                _engine: 0,
                _mem: 0,
                _vddc: 0,
                activity: 0,
                _rest: [0; 5],
            };
            if unsafe { od5(self.ctx, index, &mut act) } == AMD_OK && act.activity >= 0 {
                return Some((act.activity as f32).clamp(0.0, 100.0));
            }
        }
        None
    }
}

impl Drop for Adl {
    fn drop(&mut self) {
        unsafe {
            let _ = (self.destroy)(self.ctx);
            let _ = FreeLibrary(self.lib);
        }
    }
}

struct PdhGpu {
    query: isize,
    counter: isize,
}

impl PdhGpu {
    fn open() -> Option<Self> {
        let mut query = 0isize;
        if unsafe { PdhOpenQueryW(PCWSTR::null(), 0, &mut query) } != 0 {
            return None;
        }
        let mut counter = 0isize;
        let st = unsafe {
            PdhAddEnglishCounterW(
                query,
                w!("\\GPU Engine(*)\\Utilization Percentage"),
                0,
                &mut counter,
            )
        };
        if st != 0 {
            unsafe {
                PdhCloseQuery(query);
            }
            return None;
        }
        unsafe {
            PdhCollectQueryData(query);
        }
        Some(Self { query, counter })
    }

    fn samples(&mut self) -> Vec<(String, f32)> {
        unsafe {
            if PdhCollectQueryData(self.query) != 0 {
                return Vec::new();
            }
            let mut size = 0u32;
            let mut count = 0u32;
            let st = PdhGetFormattedCounterArrayW(
                self.counter,
                PDH_FMT_DOUBLE,
                &mut size,
                &mut count,
                None,
            );
            if st != PDH_MORE_DATA && st != 0 {
                return Vec::new();
            }
            if size == 0 {
                return Vec::new();
            }
            let mut buf = vec![0u8; size as usize];
            let st = PdhGetFormattedCounterArrayW(
                self.counter,
                PDH_FMT_DOUBLE,
                &mut size,
                &mut count,
                Some(buf.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W),
            );
            if st != 0 {
                return Vec::new();
            }
            let items = std::slice::from_raw_parts(
                buf.as_ptr() as *const PDH_FMT_COUNTERVALUE_ITEM_W,
                count as usize,
            );
            let mut out = Vec::with_capacity(count as usize);
            for it in items {
                if it.FmtValue.CStatus != PDH_CSTATUS_VALID_DATA
                    && it.FmtValue.CStatus != PDH_CSTATUS_NEW_DATA
                {
                    continue;
                }
                if it.szName.is_null() {
                    continue;
                }
                let Ok(name) = PWSTR::to_string(&it.szName) else {
                    continue;
                };
                let pct = it.FmtValue.Anonymous.doubleValue as f32;
                if pct.is_finite() {
                    out.push((name, pct));
                }
            }
            out
        }
    }
}

impl Drop for PdhGpu {
    fn drop(&mut self) {
        unsafe {
            PdhCloseQuery(self.query);
        }
    }
}

#[cfg(test)]
#[path = "tests/gpu.rs"]
mod tests;
