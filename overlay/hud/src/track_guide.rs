//! Arrow under the dash gear. Arms when you join, crash, leave the centerline,
//! or ride against track travel. Aims about 25 m ahead on the same stretch, or
//! 12 m ahead of the stretch you left when the nearest point is a different
//! part of the lap. Hides once you are up, close to that point, and facing along it.

use std::cell::RefCell;
use std::time::{Duration, Instant};

use crate::render::{line_ahead, line_fix, radar_axes, LineFix};
use crate::shm::{bytes_as_text, Snapshot, MAX_POLY};

const LOOKAHEAD_M: f32 = 25.0;
/// How far ahead of the remembered point to aim after a fold.
const REJOIN_M: f32 = 12.0;
const ON_LINE_M: f32 = 12.0;
/// Along-track gap that still counts as the stretch you are on.
const SAME_STRETCH_M: f32 = 40.0;
const ALIGN_RAD: f32 = 40.0 * std::f32::consts::PI / 180.0;
/// Track travel behind the bike. A corner (~50°) is still the right way.
const WRONG_WAY_RAD: f32 = std::f32::consts::FRAC_PI_2;
const HOLD: Duration = Duration::from_millis(400);

/// Bike-frame bearing of the track guide. `angle` is 0 when the aim is
/// bike-forward (screen up) and positive when it is bike-right.
#[derive(Clone, Copy)]
pub(crate) struct GuideAim {
    pub angle: f32,
    /// Meters from the bike to the sample the arrow is using.
    #[allow(dead_code)]
    pub dist_m: f32,
    /// World point the arrow aims at. Read by the folded-track test.
    #[allow(dead_code)]
    pub target_x: f32,
    #[allow(dead_code)]
    pub target_z: f32,
    pub aligned: bool,
}

/// Last centerline sample taken while within [`ON_LINE_M`].
#[derive(Clone, Copy)]
struct Depart {
    x: f32,
    z: f32,
    along_m: f32,
    lap_m: f32,
    looped: bool,
    segment_end: usize,
}

struct Latch {
    had_telemetry: bool,
    crashed: bool,
    track: String,
    showing: bool,
    aligned_at: Option<Instant>,
    depart: Option<Depart>,
}

impl Default for Latch {
    fn default() -> Self {
        Self {
            had_telemetry: false,
            crashed: false,
            track: String::new(),
            showing: false,
            aligned_at: None,
            depart: None,
        }
    }
}

thread_local! {
    static LATCH: RefCell<Latch> = RefCell::new(Latch::default());
}

pub fn tick(s: &Snapshot) {
    tick_at(s, Instant::now());
}

#[cfg(test)]
pub(crate) fn reset() {
    LATCH.with(|latch| *latch.borrow_mut() = Latch::default());
}

pub(crate) fn tick_at(s: &Snapshot, now: Instant) {
    LATCH.with(|latch| {
        let mut state = latch.borrow_mut();
        if s.has_telemetry == 0 {
            *state = Latch::default();
            return;
        }
        let track = bytes_as_text(&s.track_name);
        let crashed = s.local_crashed != 0;
        let joined = !state.had_telemetry
            || (!track.is_empty() && !state.track.is_empty() && track != state.track);
        let crash_rose = crashed && !state.crashed;
        let depart = state.depart;
        let looked = look(s, depart);
        if let Some((_, next)) = looked {
            if let Some(next) = next {
                state.depart = Some(next);
            }
        }
        let bearing = looked.map(|(bearing, _)| bearing);
        let can_aim = bearing.is_some();
        let aligned = bearing.as_ref().is_some_and(|bearing| bearing.aligned);
        let off_track = bearing.as_ref().is_some_and(|bearing| bearing.dist_m > ON_LINE_M);
        let wrong_way = bearing
            .as_ref()
            .is_some_and(|bearing| bearing.angle.abs() > WRONG_WAY_RAD);
        let lost = crashed || off_track || wrong_way;

        if can_aim && (lost || ((joined || crash_rose) && !aligned)) {
            state.showing = true;
            state.aligned_at = None;
        }
        if state.showing {
            if !can_aim || crashed {
                if !can_aim {
                    state.showing = false;
                }
                state.aligned_at = None;
            } else if aligned {
                match state.aligned_at {
                    Some(since) if now.saturating_duration_since(since) >= HOLD => {
                        state.showing = false;
                        state.aligned_at = None;
                    }
                    Some(_) => {}
                    None => state.aligned_at = Some(now),
                }
            } else {
                state.aligned_at = None;
            }
        }
        state.had_telemetry = true;
        state.crashed = crashed;
        if !track.is_empty() {
            state.track = track;
        }
    });
}

/// Where the arrow should point, independent of the show/hide latch.
pub(crate) fn aim(s: &Snapshot) -> Option<GuideAim> {
    LATCH.with(|latch| look(s, latch.borrow().depart).map(|(bearing, _)| bearing))
}

/// `(bearing, next depart)`. `next` is set only while the nearest sample is the
/// stretch you are on and within [`ON_LINE_M`].
fn look(s: &Snapshot, depart: Option<Depart>) -> Option<(GuideAim, Option<Depart>)> {
    if s.has_telemetry == 0 {
        return None;
    }
    let n = s.poly_count.clamp(0, MAX_POLY as i32) as usize;
    if n < 2 {
        return None;
    }
    let nearest = line_fix(s, n, s.local_x, s.local_z)?;
    let folded = depart.is_some_and(|depart| {
        along_apart(nearest.along_m, depart.along_m, depart.lap_m, depart.looped) > SAME_STRETCH_M
    });
    let (fix, ahead_m, dist_m, next) = if folded {
        let depart = depart.unwrap();
        let dx = s.local_x - depart.x;
        let dz = s.local_z - depart.z;
        let fix = LineFix {
            x: depart.x,
            z: depart.z,
            dist_m: (dx * dx + dz * dz).sqrt(),
            along_m: depart.along_m,
            lap_m: depart.lap_m,
            looped: depart.looped,
            segment_end: depart.segment_end,
        };
        (fix, REJOIN_M, (dx * dx + dz * dz).sqrt(), None)
    } else {
        let next = (nearest.dist_m <= ON_LINE_M).then_some(Depart {
            x: nearest.x,
            z: nearest.z,
            along_m: nearest.along_m,
            lap_m: nearest.lap_m,
            looped: nearest.looped,
            segment_end: nearest.segment_end,
        });
        let dist_m = nearest.dist_m;
        (nearest, LOOKAHEAD_M, dist_m, next)
    };
    let (target_x, target_z) = line_ahead(s, n, &fix, ahead_m);
    let aim_x = target_x - s.local_x;
    let aim_z = target_z - s.local_z;
    let (forward_x, forward_z, lateral_x, lateral_z) = radar_axes(s);
    let ahead = aim_x * forward_x + aim_z * forward_z;
    let lateral = aim_x * lateral_x + aim_z * lateral_z;
    let angle = if ahead * ahead + lateral * lateral < 0.16 {
        0.0
    } else {
        lateral.atan2(ahead)
    };
    let aligned = dist_m <= ON_LINE_M && angle.abs() <= ALIGN_RAD;
    Some((
        GuideAim {
            angle,
            dist_m,
            target_x,
            target_z,
            aligned,
        },
        next,
    ))
}

fn along_apart(here: f32, remembered: f32, lap_m: f32, looped: bool) -> f32 {
    let delta = (here - remembered).abs();
    if looped && lap_m > delta {
        delta.min(lap_m - delta)
    } else {
        delta
    }
}

/// Bearing while the latch is up. `None` when the arrow should not draw.
pub(crate) fn visible(s: &Snapshot) -> Option<GuideAim> {
    let showing = LATCH.with(|latch| latch.borrow().showing);
    if !showing {
        return None;
    }
    aim(s)
}
