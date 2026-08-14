use std::mem;
use std::ptr;
use std::sync::atomic::{compiler_fence, AtomicU32, Ordering};

use windows::core::w;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Memory::{
    MapViewOfFile, OpenFileMappingW, UnmapViewOfFile, FILE_MAP_READ, MEMORY_MAPPED_VIEW_ADDRESS,
};

pub const MAGIC: u32 = 0x4F42584D;
pub const VERSION: u32 = 1;
pub const MAX_POLY: usize = 1024;
pub const MAX_RIDERS: usize = 64;
pub const MAX_STANDINGS: usize = 40;
pub const NAME: usize = 32;
pub const TRACK_NAME: usize = 64;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Point {
    pub x: f32,
    pub z: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Rider {
    pub race_num: i32,
    pub x: f32,
    pub z: f32,
    pub yaw: f32,
    pub track_pos: f32,
    pub crashed: i32,
    pub name: [u8; NAME],
}

impl Default for Rider {
    fn default() -> Self {
        Self {
            race_num: 0,
            x: 0.0,
            z: 0.0,
            yaw: 0.0,
            track_pos: 0.0,
            crashed: 0,
            name: [0; NAME],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Standing {
    pub race_num: i32,
    pub position: i32,
    pub state: i32,
    pub best_lap_ms: i32,
    pub num_laps: i32,
    pub gap_ms: i32,
    pub gap_laps: i32,
    pub pit: i32,
    pub name: [u8; NAME],
}

impl Default for Standing {
    fn default() -> Self {
        Self {
            race_num: 0,
            position: 0,
            state: 0,
            best_lap_ms: 0,
            num_laps: 0,
            gap_ms: 0,
            gap_laps: 0,
            pit: 0,
            name: [0; NAME],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Snapshot {
    pub magic: u32,
    pub version: u32,
    pub seq: u32,
    pub size: u32,
    pub tick_qpc: u64,
    pub local_race_num: i32,
    pub focus_race_num: i32,
    pub has_telemetry: i32,
    pub local_crashed: i32,
    pub local_x: f32,
    pub local_z: f32,
    pub local_vel_x: f32,
    pub local_vel_z: f32,
    pub local_yaw: f32,
    pub local_speed: f32,
    pub local_track_pos: f32,
    pub track_name: [u8; TRACK_NAME],
    pub track_length: f32,
    pub sf_meters: f32,
    pub poly_count: i32,
    pub poly: [Point; MAX_POLY],
    pub rider_count: i32,
    pub riders: [Rider; MAX_RIDERS],
    pub standing_count: i32,
    pub standings: [Standing; MAX_STANDINGS],
    pub map: Rect,
    pub standings_rect: Rect,
    pub relative: Rect,
    pub show_map: i32,
    pub show_standings: i32,
    pub show_relative: i32,
    pub standings_rows: i32,
    pub relative_count: i32,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            magic: 0,
            version: 0,
            seq: 0,
            size: 0,
            tick_qpc: 0,
            local_race_num: -1,
            focus_race_num: -1,
            has_telemetry: 0,
            local_crashed: 0,
            local_x: 0.0,
            local_z: 0.0,
            local_vel_x: 0.0,
            local_vel_z: 0.0,
            local_yaw: 0.0,
            local_speed: 0.0,
            local_track_pos: 0.0,
            track_name: [0; TRACK_NAME],
            track_length: 0.0,
            sf_meters: 0.0,
            poly_count: 0,
            poly: [Point { x: 0.0, z: 0.0 }; MAX_POLY],
            rider_count: 0,
            riders: [Rider::default(); MAX_RIDERS],
            standing_count: 0,
            standings: [Standing::default(); MAX_STANDINGS],
            map: Rect {
                x: 0.775,
                y: 0.62,
                w: 0.21,
                h: 0.34,
            },
            standings_rect: Rect {
                x: 0.012,
                y: 0.03,
                w: 0.235,
                h: 0.42,
            },
            relative: Rect {
                x: 0.012,
                y: 0.62,
                w: 0.235,
                h: 0.33,
            },
            show_map: 1,
            show_standings: 1,
            show_relative: 1,
            standings_rows: 12,
            relative_count: 3,
        }
    }
}

pub fn cstr(buf: &[u8]) -> String {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}

pub struct Shm {
    _map: HANDLE,
    view: MEMORY_MAPPED_VIEW_ADDRESS,
}

impl Shm {
    pub fn open() -> Option<Self> {
        unsafe {
            let map = OpenFileMappingW(FILE_MAP_READ.0, false, w!("Local\\MXBOHudV1")).ok()?;
            let view = MapViewOfFile(map, FILE_MAP_READ, 0, 0, mem::size_of::<Snapshot>());
            if view.Value.is_null() {
                return None;
            }
            Some(Self { _map: map, view })
        }
    }

    pub fn read(&self) -> Option<Snapshot> {
        unsafe {
            let src = self.view.Value as *const Snapshot;
            if src.is_null() {
                return None;
            }
            for _ in 0..64 {
                let seq_ptr = ptr::addr_of!((*src).seq) as *const AtomicU32;
                let s1 = (*seq_ptr).load(Ordering::Acquire);
                if s1 & 1 != 0 {
                    continue;
                }
                let copy = ptr::read_volatile(src);
                compiler_fence(Ordering::Acquire);
                let s2 = (*seq_ptr).load(Ordering::Acquire);
                if s1 == s2 && copy.magic == MAGIC && copy.version == VERSION {
                    return Some(copy);
                }
            }
            None
        }
    }
}

impl Drop for Shm {
    fn drop(&mut self) {
        unsafe {
            if !self.view.Value.is_null() {
                let _ = UnmapViewOfFile(self.view);
            }
        }
    }
}
