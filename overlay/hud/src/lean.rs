//! Lean + steer + pitch for the camera subject.

use crate::shm::Snapshot;

/// Chassis lean past 60 is real. Euler jumps sit near ±180 — clamp there, not at 60.
const ROLL_MAX: f32 = 90.0;
/// Pitch hairline is still scaled to ±60. Whoops past that peg the fill.
const PITCH_MAX: f32 = 60.0;
/// In the air, chassis roll often sticks around 50–62 while the rider is still leaned.
const AIR_PEG_LO: f32 = 50.0;
const AIR_PEG_HI: f32 = 62.0;

fn wrap_deg(raw: f32) -> f32 {
    let mut d = raw % 360.0;
    if d > 180.0 {
        d -= 360.0;
    }
    if d < -180.0 {
        d += 360.0;
    }
    d
}

fn finite_wrap(raw: f32) -> Option<f32> {
    if !raw.is_finite() {
        None
    } else {
        Some(wrap_deg(raw))
    }
}

/// `m_fRoll` / `m_fLean` are degrees. Wrap, then clamp at ±90 so an inverted
/// Euler does not become 170°. Do not snap to 0, and do not cap real 70° leans.
pub fn angle_deg(raw: f32) -> f32 {
    finite_wrap(raw)
        .map(|d| d.clamp(-ROLL_MAX, ROLL_MAX))
        .unwrap_or(0.0)
}

fn riding_deg(s: &Snapshot) -> f32 {
    let roll = angle_deg(s.local_roll);
    let body = angle_deg(rider_lean(s, s.local_race_num));
    let a = roll.abs();
    let body_ok = body.abs() > 2.0 && body.abs() < 50.0;
    // Air peg (~50–62) or an inverted Euler at the 90 wall — rider lean keeps the scrub.
    // A 70°+ chassis angle on the ground is the lean; do not replace it with body.
    if body_ok && (a >= ROLL_MAX || (a >= AIR_PEG_LO && a <= AIR_PEG_HI)) {
        body
    } else {
        roll
    }
}

/// Steer as −1..1 (right positive, from behind). `None` when there is no
/// usable lock and the value is not already a unit fraction.
pub fn steer_frac(steer: f32, lock: f32) -> Option<f32> {
    if !steer.is_finite() {
        return None;
    }
    let frac = if lock.is_finite() && lock.abs() > 0.15 {
        if steer.abs() <= 1.0 && lock.abs() > 1.5 {
            steer
        } else {
            let lock_deg = if lock.abs() > 3.0 {
                lock.abs()
            } else {
                lock.abs().to_degrees().max(12.0)
            };
            let steer_deg = if steer.abs() > 3.0 || lock.abs() > 3.0 {
                steer
            } else {
                steer.to_degrees()
            };
            steer_deg / lock_deg
        }
    } else if steer.abs() <= 1.0 {
        steer
    } else {
        return None;
    };
    Some(frac.clamp(-1.0, 1.0))
}

/// Pitch as −1..1 of the ±60° hairline, plugin sign (`+` is nose down).
/// `from_behind` flips it so HUD nose-up is positive.
pub fn pitch_frac(raw: f32) -> f32 {
    finite_wrap(raw)
        .map(|d| d.clamp(-PITCH_MAX, PITCH_MAX) / PITCH_MAX)
        .unwrap_or(0.0)
}

#[derive(Clone, Copy, Debug)]
pub struct LeanView {
    /// Signed degrees, right-hand lean positive (from behind the bike).
    pub deg: f32,
    /// −1..1, right positive. `None` in spectate / replay (no steer).
    pub steer: Option<f32>,
    /// −1..1, nose up positive. `None` in spectate / replay (no pitch).
    pub pitch: Option<f32>,
}

fn rider_lean(s: &Snapshot, race_num: i32) -> f32 {
    if race_num <= 0 {
        return 0.0;
    }
    let n = s.rider_count.clamp(0, crate::shm::MAX_RIDERS as i32) as usize;
    s.riders[..n]
        .iter()
        .find(|r| r.race_num == race_num)
        .map(|r| r.lean)
        .unwrap_or(0.0)
}

/// Plugin roll/steer/pitch are the opposite of the HUD. Flip so right lean,
/// right bar, and nose-up are positive. Nose down is negative.
fn from_behind(deg: f32, steer: Option<f32>, pitch: Option<f32>) -> LeanView {
    LeanView {
        deg: -deg,
        steer: steer.map(|f| -f),
        pitch: pitch.map(|f| -f),
    }
}

/// Riding uses local roll, pitch, and steer. Spectate / replay
/// (`has_telemetry == 0`) follows the camera rider's `lean` and drops
/// steer and pitch.
pub fn view(s: &Snapshot) -> LeanView {
    if s.has_telemetry != 0 {
        from_behind(
            riding_deg(s),
            steer_frac(s.local_steer, s.steer_lock),
            Some(pitch_frac(s.local_pitch)),
        )
    } else {
        let focus = if s.focus_race_num > 0 {
            s.focus_race_num
        } else {
            s.local_race_num
        };
        from_behind(angle_deg(rider_lean(s, focus)), None, None)
    }
}

#[cfg(test)]
#[path = "tests/lean.rs"]
mod tests;
