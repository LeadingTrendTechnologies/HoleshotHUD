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
            // Must match MXBO_SHM_NAME in src/shm/mxbo_shm.h (versioned with SHM layout).
            let map = OpenFileMappingW(FILE_MAP_READ.0, false, w!("Local\\MXBOHudV9")).ok()?;
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
                let magic = ptr::read_volatile(ptr::addr_of!((*src).magic));
                let version = ptr::read_volatile(ptr::addr_of!((*src).version));
                let size = ptr::read_volatile(ptr::addr_of!((*src).size)) as usize;
                let max = mem::size_of::<Snapshot>();
                if magic != MAGIC || version < 8 || version > VERSION || size < 64 || size > max {
                    compiler_fence(Ordering::Acquire);
                    let s2 = (*seq_ptr).load(Ordering::Acquire);
                    if s1 == s2 {
                        return None;
                    }
                    continue;
                }
                let mut copy = Snapshot::default();
                ptr::copy_nonoverlapping(src as *const u8, (&mut copy as *mut Snapshot).cast(), size);
                compiler_fence(Ordering::Acquire);
                let s2 = (*seq_ptr).load(Ordering::Acquire);
                if s1 == s2 {
                    if version < VERSION {
                        copy.session_kind = -1;
                        copy.session_state = -1;
                    }
                    return Some(copy);
                }
            }
            None
        }
    }

    /// Peek SHM magic/version/seq/size without a full seqlock copy.
    /// Used by Holeshot-HUD; dump-track links this module but does not call it.
    #[allow(dead_code)]
    pub fn header(&self) -> Option<(u32, u32, u32, u32)> {
        unsafe {
            let src = self.view.Value as *const Snapshot;
            if src.is_null() {
                return None;
            }
            Some(((*src).magic, (*src).version, (*src).seq, (*src).size))
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
