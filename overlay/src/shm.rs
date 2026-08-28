use std::mem;
use std::ptr;
use std::sync::atomic::{compiler_fence, AtomicI32, AtomicU32, Ordering};

use windows::core::w;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Memory::{
    MapViewOfFile, OpenFileMappingW, UnmapViewOfFile, FILE_MAP_ALL_ACCESS, FILE_MAP_READ,
    MEMORY_MAPPED_VIEW_ADDRESS,
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
                self.view.Value = ptr::null_mut();
            }
            if !self._map.is_invalid() {
                let _ = CloseHandle(self._map);
                self._map = HANDLE::default();
            }
        }
    }
}

#[allow(dead_code)]
const CMD_MAGIC: u32 = 0x4342_584D;

#[repr(C)]
#[allow(dead_code)]
struct CmdView {
    magic: u32,
    spectating: i32,
    spectate_race_num: i32,
}

/// Overlay → plugin camera request. Created by the plugin as `Local\MXBOHudCmdV1`.
#[allow(dead_code)]
pub struct Cmd {
    _map: HANDLE,
    view: MEMORY_MAPPED_VIEW_ADDRESS,
}

#[allow(dead_code)]
impl Cmd {
    pub fn open() -> Option<Self> {
        unsafe {
            let map = OpenFileMappingW(FILE_MAP_ALL_ACCESS.0, false, w!("Local\\MXBOHudCmdV1")).ok()?;
            let view = MapViewOfFile(map, FILE_MAP_ALL_ACCESS, 0, 0, mem::size_of::<CmdView>());
            if view.Value.is_null() {
                return None;
            }
            let src = view.Value as *const CmdView;
            if (*src).magic != CMD_MAGIC {
                return None;
            }
            Some(Self { _map: map, view })
        }
    }

    pub fn spectating(&self) -> bool {
        unsafe {
            let src = self.view.Value as *const CmdView;
            if src.is_null() {
                return false;
            }
            (*src).spectating != 0
        }
    }

    pub fn request(&self, race_num: i32) {
        if race_num <= 0 {
            return;
        }
        unsafe {
            let src = self.view.Value as *mut CmdView;
            if src.is_null() {
                return;
            }
            let slot = ptr::addr_of_mut!((*src).spectate_race_num) as *const AtomicI32;
            (*slot).store(race_num, Ordering::SeqCst);
        }
    }
}

impl Drop for Cmd {
    fn drop(&mut self) {
        unsafe {
            if !self.view.Value.is_null() {
                let _ = UnmapViewOfFile(self.view);
                self.view.Value = ptr::null_mut();
            }
            if !self._map.is_invalid() {
                let _ = CloseHandle(self._map);
                self._map = HANDLE::default();
            }
        }
    }
}
