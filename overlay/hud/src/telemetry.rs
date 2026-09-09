use std::cell::RefCell;

use crate::snapshot::Snapshot;

pub const TRACE_LEN: usize = 128;

#[derive(Clone, Copy, Default)]
pub struct Sample {
    pub throttle: f32,
    pub brake: f32,
    /// −1..1, right positive (same as Lean). 0 when spectating.
    pub steer: f32,
}

#[derive(Clone)]
pub struct Trace {
    buf: [Sample; TRACE_LEN],
    head: usize,
    filled: usize,
}

impl Default for Trace {
    fn default() -> Self {
        Self {
            buf: [Sample::default(); TRACE_LEN],
            head: 0,
            filled: 0,
        }
    }
}

impl Trace {
    pub fn push(&mut self, throttle: f32, brake: f32, steer: f32) {
        self.buf[self.head] = Sample {
            throttle: throttle.clamp(0.0, 1.0),
            brake: brake.clamp(0.0, 1.0),
            steer: steer.clamp(-1.0, 1.0),
        };
        self.head = (self.head + 1) % TRACE_LEN;
        self.filled = (self.filled + 1).min(TRACE_LEN);
    }

    pub fn len(&self) -> usize {
        self.filled
    }

    pub fn get(&self, i: usize) -> Sample {
        let start = (self.head + TRACE_LEN - self.filled) % TRACE_LEN;
        self.buf[(start + i) % TRACE_LEN]
    }
}

thread_local! {
    static TRACE: RefCell<Trace> = RefCell::new(Trace::default());
}

pub fn note(throttle: f32, brake: f32, steer: f32) {
    TRACE.with(|t| t.borrow_mut().push(throttle, brake, steer));
}

pub fn reset() {
    TRACE.with(|t| *t.borrow_mut() = Trace::default());
}

pub fn with<R>(f: impl FnOnce(&Trace) -> R) -> R {
    TRACE.with(|t| f(&t.borrow()))
}

pub fn inputs(s: &Snapshot) -> (f32, f32, f32) {
    let thr = s.local_throttle.clamp(0.0, 1.0);
    let brk = s.local_front_brake.max(s.local_rear_brake).clamp(0.0, 1.0);
    let clu = s.local_clutch.clamp(0.0, 1.0);
    (thr, brk, clu)
}

/// Right-positive bar angle while riding. 0 in spectate / replay.
pub fn steer(s: &Snapshot) -> f32 {
    crate::lean::view(s).steer.unwrap_or(0.0)
}

/// Shift light or limiter: gear should flush faint red.
pub fn shift_warn(s: &Snapshot) -> bool {
    let rpm = s.local_rpm;
    if rpm <= 200 {
        return false;
    }
    let shift = s.shift_rpm;
    let max = s.max_rpm;
    (shift > 0 && rpm >= shift) || (max > 0 && rpm >= max)
}

/// Scripted MX corner for goldens: gas, lift, brake, back on it.
pub fn seed_corner() {
    reset();
    for i in 0..TRACE_LEN {
        let u = i as f32 / (TRACE_LEN - 1) as f32;
        let (thr, brk) = if u < 0.42 {
            (0.92 - 0.06 * (u * 10.0).sin().abs(), 0.02)
        } else if u < 0.56 {
            let t = (u - 0.42) / 0.14;
            (0.90 * (1.0 - t), 0.04 + 0.86 * t)
        } else if u < 0.70 {
            let t = (u - 0.56) / 0.14;
            (0.10 * t, 0.90 * (1.0 - t))
        } else {
            let t = (u - 0.70) / 0.30;
            ((0.18 + 0.72 * t).min(0.95), 0.02)
        };
        let steer = if u < 0.42 {
            0.04 * (u * 8.0).sin()
        } else if u < 0.70 {
            0.62 * (((u - 0.42) / 0.28) * std::f32::consts::PI).sin()
        } else {
            0.06
        };
        note(thr, brk, steer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::Snapshot;

    #[test]
    fn shift_warn_at_shift_or_limiter() {
        let mut s = Snapshot::default();
        s.local_rpm = 11_800;
        s.shift_rpm = 11_800;
        s.max_rpm = 13_500;
        assert!(shift_warn(&s));
        s.local_rpm = 11_700;
        assert!(!shift_warn(&s));
        s.shift_rpm = 0;
        s.local_rpm = 13_500;
        assert!(shift_warn(&s));
        s.local_rpm = 0;
        assert!(!shift_warn(&s));
    }
}
