//! Live local pad for the Controller widget. Not plugin telemetry.

use std::sync::Mutex;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum PadKind {
    #[default]
    None,
    Xbox,
    Sony,
}

pub const SOUTH: u32 = 1 << 0;
pub const EAST: u32 = 1 << 1;
pub const WEST: u32 = 1 << 2;
pub const NORTH: u32 = 1 << 3;
pub const LB: u32 = 1 << 4;
pub const RB: u32 = 1 << 5;
pub const BACK: u32 = 1 << 6;
pub const START: u32 = 1 << 7;
pub const LS: u32 = 1 << 8;
pub const RS: u32 = 1 << 9;
pub const UP: u32 = 1 << 10;
pub const DOWN: u32 = 1 << 11;
pub const LEFT: u32 = 1 << 12;
pub const RIGHT: u32 = 1 << 13;
pub const GUIDE: u32 = 1 << 14;
pub const TOUCH: u32 = 1 << 15;

#[derive(Clone, Copy, Debug)]
pub struct PadState {
    pub kind: PadKind,
    /// −1 left … +1 right.
    pub lx: f32,
    /// −1 up … +1 down (screen).
    pub ly: f32,
    pub rx: f32,
    pub ry: f32,
    /// Analog squeeze 0…1.
    pub lt: f32,
    pub rt: f32,
    pub buttons: u32,
}

impl PadState {
    pub const DISCONNECTED: Self = Self {
        kind: PadKind::None,
        lx: 0.0,
        ly: 0.0,
        rx: 0.0,
        ry: 0.0,
        lt: 0.0,
        rt: 0.0,
        buttons: 0,
    };

    pub fn down(self, bit: u32) -> bool {
        self.buttons & bit != 0
    }

    pub fn connected(self) -> bool {
        self.kind != PadKind::None
    }
}

static PAD: Mutex<PadState> = Mutex::new(PadState::DISCONNECTED);

pub fn set(state: PadState) {
    if let Ok(mut g) = PAD.lock() {
        *g = state;
    }
}

pub fn current() -> PadState {
    PAD.lock().map(|g| *g).unwrap_or(PadState::DISCONNECTED)
}

/// Stick axis from a 0–255 HID byte. 128 is center. +Y is down.
pub fn axis_u8(v: u8) -> f32 {
    ((v as f32 - 127.5) / 127.5).clamp(-1.0, 1.0)
}

/// XInput thumb. +Y from the API is up; we flip so +Y is down on screen.
pub fn axis_i16(v: i16, dead: i16) -> f32 {
    let dead = dead.max(1) as f32;
    let n = v as f32;
    if n.abs() <= dead {
        0.0
    } else {
        let sign = n.signum();
        let mag = ((n.abs() - dead) / (32767.0 - dead)).clamp(0.0, 1.0);
        sign * mag
    }
}

pub fn trigger_u8(v: u8) -> f32 {
    v as f32 / 255.0
}

/// Demo DualShock: Cross, D-pad right, L1, left stick out, R2 squeezed.
pub fn demo_sony() -> PadState {
    PadState {
        kind: PadKind::Sony,
        lx: -0.42,
        ly: -0.55,
        rx: 0.0,
        ry: 0.0,
        lt: 0.0,
        rt: 0.70,
        buttons: SOUTH | RIGHT | LB,
    }
}

/// Demo Xbox: A, D-pad right, LB, left stick out, RT squeezed.
pub fn demo_xbox() -> PadState {
    PadState {
        kind: PadKind::Xbox,
        lx: -0.42,
        ly: -0.55,
        rx: 0.0,
        ry: 0.0,
        lt: 0.0,
        rt: 0.70,
        buttons: SOUTH | RIGHT | LB,
    }
}

#[cfg(test)]
#[path = "tests/gamepad.rs"]
mod tests;
