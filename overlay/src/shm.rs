use std::mem;
use std::ptr;
use std::sync::atomic::{compiler_fence, AtomicU32, Ordering};

use windows::core::w;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Memory::{
    MapViewOfFile, OpenFileMappingW, UnmapViewOfFile, FILE_MAP_READ, MEMORY_MAPPED_VIEW_ADDRESS,
};

pub use mxbo_hud::snapshot::*;

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
