//! Shared race view: session clock + classification field.
//!
//! [`RaceStore::refresh`] once per overlay frame. Drawers use [`RaceStore::with`]
//! (re-entrant) so they do not clone the store. [`RaceStore::tick`] still returns
//! a clone for tests and clock logs.

use crate::config::SessionPreset;
use crate::shm::{bytes_as_text, Snapshot, Standing, MAX_STANDINGS};
use std::cell::Cell;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Mutex;

fn anim_now() -> f32 {
    #[cfg(target_arch = "wasm32")]
    {
        thread_local! {
            static T: Cell<f32> = const { Cell::new(0.0) };
        }
        T.with(|c| {
            let v = c.get() + 1.0 / 60.0;
            c.set(v);
            v
        })
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        thread_local! {
            static ORIGIN: std::time::Instant = std::time::Instant::now();
        }
        ORIGIN.with(|o| o.elapsed().as_secs_f32())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockMode {
    Practice,
    Gate,
    LapRace,
    Timed,
    Overtime,
}

impl ClockMode {
    fn from_state(s: &Snapshot, remain: Option<i32>, banner: &str) -> Self {
        if overtime_active(s) {
            ClockMode::Overtime
        } else if IN_GATE.load(Ordering::Relaxed) == 1 {
            ClockMode::Gate
        } else if is_lap_race(s) {
            ClockMode::LapRace
        } else if is_warmup(s) || is_practice_session(s) {
            ClockMode::Practice
        } else if remain.is_some() || banner.contains(':') {
            ClockMode::Timed
        } else {
            ClockMode::LapRace
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RaceFlag {
    #[default]
    None,
    White,
    Checkered,
}

#[derive(Debug, Clone)]
pub struct SessionClock {
    pub mode: ClockMode,
    pub remain_ms: Option<i32>,
    pub banner: (char, String),
    pub in_gate: bool,
    pub expired: bool,
    pub extras_taken: i32,
    pub extras_total: i32,
    /// Timed-extras flag from timer + lap count (lap motos use approach logic in render).
    pub flag: RaceFlag,
}

impl Default for SessionClock {
    fn default() -> Self {
        Self {
            mode: ClockMode::Practice,
            remain_ms: None,
            banner: ('\u{f2f2}', "--:--".into()),
            in_gate: false,
            expired: false,
            extras_taken: 0,
            extras_total: 0,
            flag: RaceFlag::None,
        }
    }
}

#[derive(Clone)]
pub struct RaceRow {
    pub standing: Standing,
    pub current_lap: i32,
    pub is_focus: bool,
    pub is_leader: bool,
    pub interval_ms: i32,
    pub interval_laps: i32,
    pub gap_to_focus_ms: i32,
    pub gap_to_focus_laps: i32,
}

#[derive(Clone, Default)]
pub struct RaceField {
    pub rows: Vec<RaceRow>,
    pub focus: Option<usize>,
    pub leader: Option<usize>,
    pub session_best_ms: i32,
}

impl RaceField {
    pub fn row_by_num(&self, race_num: i32) -> Option<&RaceRow> {
        self.rows.iter().find(|r| r.standing.race_num == race_num)
    }

    /// Classification rows in live order for the boards that iterate standings.
    pub fn board(&self) -> Vec<Standing> {
        self.rows.iter().map(|r| r.standing).collect()
    }
}

#[derive(Clone, Default)]
pub struct RaceStore {
    pub clock: SessionClock,
    pub field: RaceField,
    /// Race flag from the last advancing [`RaceStore::refresh`]. A paint-only refresh
    /// leaves it. Render draws this; it does not re-run the start/finish machine.
    pub race_flag: i32,
}

static VIEW: Mutex<RaceStore> = Mutex::new(RaceStore {
    clock: SessionClock {
        mode: ClockMode::Practice,
        remain_ms: None,
        banner: ('\u{f2f2}', String::new()),
        in_gate: false,
        expired: false,
        extras_taken: 0,
        extras_total: 0,
        flag: RaceFlag::None,
    },
    field: RaceField {
        rows: Vec::new(),
        focus: None,
        leader: None,
        session_best_ms: 0,
    },
    race_flag: 0,
});

thread_local! {
    static WITH: Cell<Option<NonNull<RaceStore>>> = const { Cell::new(None) };
}

/// Race order we published last tick, P1 first, as race numbers. The classification the
/// game sends only moves when someone crosses the line, so this is also the hysteresis
/// state that keeps a pass from flickering while two riders run side by side.
static LIVE_ORDER: Mutex<Vec<i32>> = Mutex::new(Vec::new());

/// Same bubble as [`LIVE_ORDER`] but ranked on track progress only (no penalties). Used to
/// mark when live place and on-track place disagree.
static TRACK_ORDER: Mutex<Vec<i32>> = Mutex::new(Vec::new());

/// A rider has to be this far up the track before they take the place. About a bike and a
/// half: `track_pos` is a centerline projection, so a rider taking a wide line or landing
/// off a jump moves a metre or two on its own.
const PASS_M: f32 = 3.0;
/// Two published samples of one bike. A side-by-side pass is further apart than this.
const COPIED_BODY_M: f32 = 1.0;
/// Once they have it they keep it until they drop back inside this.
const HOLD_M: f32 = 0.5;
/// Two centerline samples closer than this in the world, while `track_pos` says they
/// are much further apart, are parallel lanes or an over/under. Not a pass.
const FOLD_WORLD_M: f32 = 12.0;
/// Rider bodies closer than this, with a much larger along-track gap, are in the next
/// lane. Wider than the sample cap: Farm14 put a bike 24 m away 700 m up the lap.
/// Millcreek still took the place at 40.0–40.3 m.
const BODY_CLOSE_M: f32 = 45.0;
/// A folded short wrap only takes a place the centerline already has when the bodies
/// are this close. Washougal sits on a 30 m chord (about 29–31 m, 80–210 m up the lap)
/// and was flashing a place. Maryland's holeshot is 16 m.
const FOLD_STRIP_M: f32 = 20.0;
/// A refused fold stays refused until the bodies are this far apart. Washougal released
/// a 159 m centerline lead when the gap ticked from 44.9 m to 48.7 m.
const FOLD_RELEASE_M: f32 = 55.0;
/// Chord over arc. A 180° hairpin is still about 0.64; a lane running back beside
/// itself is far below this.
const FOLD_RATIO: f32 = 0.45;
/// A chord outside [`BODY_CLOSE_M`] and under half the spline is the lap bending
/// back. Wolf Creek's finish chord was 229 m on a 478 m spline.
const BEND_RATIO: f32 = 0.5;
/// Spline gap below this is a start pack or a short straight, not the next lane.
/// Briarcliff dropped places on a ~30 m centerline gap with the bikes ~12 m apart.
const FOLD_ALONG_M: f32 = 80.0;
/// Same start/finish stripe. A classification lap one sample early is this far,
/// in metres, on a short track or a long one. Inside the next-lane body cap, so
/// a 45 m fold stays on that rule.
const STRIPE_M: f32 = 25.0;
/// A same-direction gap under this is one corner, not the other side of the track.
/// Forest flashed a place at 80–90 m with the bikes 20–40 m apart. MXB Club flashed
/// one at 125–131 m when the bodies crossed 20 m. A wrap across the start, and the
/// locked false leads from 159 m up, stay on the other branch.
const FOLD_CORNER_M: f32 = 140.0;
/// A refused coordinate's raw lead under this is still that corner once the body
/// has walked [`BODY_CLOSE_M`] along the refusal. The straight chord can fold
/// back inside 45 m. A lead from 159 m up stays the other branch.
const FROZEN_CORNER_M: f32 = 155.0;
/// A direct gap past [`FOLD_CORNER_M`] whose bikes are still this far apart.
/// The Island dropped the leader at 143–246 m with the chord still 90–180 m.
const SPREAD_CHORD_M: f32 = 80.0;
/// Chord over that direct gap. The Island chords were about 0.6 of the spline.
/// A 45 m chord at 159 m stays the other branch.
const SPREAD_RATIO: f32 = 0.55;
/// The published line has to be this close to `track_length` or the samples are not
/// the lap the fraction is measured on. A short fixture line keeps the old rule.
const POLY_LENGTH_SLACK: f32 = 0.25;
/// Further apart than this and we cannot say who is ahead from `track_pos` alone: the
/// fraction is measured from the centerline origin, not from the start/finish line, so
/// only riders close together compare safely.
const PAIR_MAX_M: f32 = 250.0;
/// One tick moving further than this is a reset, a shortcut or a network teleport, not
/// riding, so it does not count as distance covered.
const MAX_STEP_M: f32 = 80.0;
/// The leader was already this close to the line when the timed clock died. That
/// crossing finishes the lap in progress. Metres, so a short track and a long one
/// treat the same approach the same way.
const EXPIRY_LINE_M: f32 = 200.0;
/// Pace a penalty is converted at before anyone has set a lap.
const PENALTY_FALLBACK_MPS: f32 = 15.0;
/// Small overshoot past one lap of armed travel is noise (centerline wobble near the
/// line). Past this, the tracker rode more than a lap without a `num_laps` bump.
const LAP_OVERFLOW_SLACK_M: f32 = 50.0;
/// Still in the start slot. Further than this off the stored gate fraction and the
/// field has left the slots. The place stays blank until the holeshot line anyway.
const GATE_SLOT_M: f32 = 5.0;
/// No `RaceHoleshot` yet. Once the furthest rider is this far past the back of the
/// grid, publish anyway so a missed callback does not hide the lap.
const HOLESHOT_FALLBACK_M: f32 = 200.0;

/// A time penalty as metres of track. The game only applies penalties to the results, and
/// the live order compares distance, so the seconds are ridden off at the session best
/// pace. One pace for the whole field keeps the order consistent from pair to pair.
fn penalty_m(s: &Snapshot, penalty_ms: i32) -> f32 {
    let best_ms = session_best_ms(s);
    let metres_per_second = if best_ms > 0 {
        s.track_length / (best_ms as f32 / 1000.0)
    } else {
        PENALTY_FALLBACK_MPS
    };
    penalty_ms as f32 / 1000.0 * metres_per_second
}

/// Distance a rider has covered since the gate dropped or since their last line crossing.
/// On the open lap the grid snapshot orders them by location, including a bike that
/// crashed and lost `track_pos`.
#[derive(Clone, Copy)]
struct RiderProgress {
    race_num: i32,
    /// Lap fraction last tick, `None` while they were missing from `riders[]`.
    last_frac: Option<f32>,
    /// Lap fraction while the gate was up. `None` if we never saw them on the gate.
    gate_frac: Option<f32>,
    /// Continuous lap coordinate. Seeded from the first sample, gate or not, and
    /// moved only when the spline step and the body agree.
    unwrapped: Option<f32>,
    /// A step over `MAX_STEP_M` we have not accepted yet. The next tick that stays
    /// there takes it, so one glitch does not move the place and a real jump is not lost.
    pending_frac: Option<f32>,
    /// World position when `unwrapped` last moved. A spline jump that leaves this
    /// put is a projection glitch, not a pass.
    last_world: Option<(f32, f32)>,
    /// World position on the previous sample, including ticks that refused the
    /// coordinate. The step from here is the path, not the chord back to `last_world`.
    sample_world: Option<(f32, f32)>,
    /// Body path walked while the raw coordinate is refused. Reset when a sample
    /// is accepted. A corner that folds the chord back still adds up here.
    refused_path_m: f32,
    travelled_m: f32,
    /// Body movement on steps the rider actually rode. A spline lap the body did not
    /// follow is not in here, and one tick over `MAX_STEP_M` is not a lap of it.
    body_travelled_m: f32,
    lap_base_m: f32,
    laps_at_base: i32,
    /// We saw where this lap started: gate, race-go cold arm, or a line crossing.
    armed: bool,
    /// Armed travel grew more than a lap + slack without a `num_laps` rise. Pin to the
    /// game place (no S/F fallback) until the next crossing or sector gate.
    corrupt: bool,
    /// Highest sector gate applied to this lap's base. A rise rebases travel.
    gates_seen: i32,
    /// This sample's `track_pos` was not ridden. Ordering must not treat that
    /// raw fraction as a pass over the coordinate we kept.
    raw_ignored: bool,
    /// `num_laps` rose on a sample with no `track_pos`. The coordinate stays
    /// just past the line until a real fraction comes back. Pack pull must not
    /// move it.
    stripe_pin: bool,
}

static PROGRESS: Mutex<Vec<RiderProgress>> = Mutex::new(Vec::new());

/// The field is still sitting on the stored gate slots, so there is no race place yet
/// even if `IN_GATE` has already cleared.
static HOLDING_GATE: AtomicBool = AtomicBool::new(false);

/// Game session state while the start gate is still up. The pre-start countdown
/// clears `IN_GATE` before the gate drops; this stays 256 until it does.
const START_GATE_STATE: i32 = 256;

pub(crate) fn start_gate_up(s: &Snapshot) -> bool {
    s.session_state == START_GATE_STATE
}

pub(crate) fn holding_gate() -> bool {
    HOLDING_GATE.load(Ordering::Relaxed)
}

fn note_progress(s: &Snapshot, clock: &SessionClock, n: usize) {
    if s.track_length <= 10.0 {
        return;
    }
    let Ok(mut tracked) = PROGRESS.lock() else {
        return;
    };
    // Cold-arm on the first real step when the race is live and we never saw the gate:
    // without a known line, far pairs only score through the tracker. Skip when the line
    // is known so a mid-race join still uses S/F metres. Do not arm on a zero step — that
    // would invent equal travel for riders who are already spread out.
    let cold_arm = live_order_active(s, clock) && !line_known(s);
    let on_gate = clock.in_gate || start_gate_up(s);
    for standing in &s.standings[..n] {
        let frac = rider_lap_pos(s, standing.race_num);
        let index = match tracked.iter().position(|p| p.race_num == standing.race_num) {
            Some(index) => index,
            None => {
                tracked.push(RiderProgress {
                    race_num: standing.race_num,
                    last_frac: frac,
                    gate_frac: None,
                    unwrapped: None,
                    pending_frac: None,
                    last_world: None,
                    sample_world: None,
                    refused_path_m: 0.0,
                    travelled_m: 0.0,
                    body_travelled_m: 0.0,
                    lap_base_m: 0.0,
                    laps_at_base: standing.num_laps,
                    armed: false,
                    corrupt: false,
                    gates_seen: 0,
                    raw_ignored: false,
                    stripe_pin: false,
                });
                tracked.len() - 1
            }
        };
        let progress = &mut tracked[index];
        if on_gate {
            let kept_gate = frac.or(progress.gate_frac);
            *progress = RiderProgress {
                race_num: standing.race_num,
                last_frac: frac,
                gate_frac: kept_gate,
                unwrapped: progress.unwrapped,
                pending_frac: None,
                last_world: rider_world(s, standing.race_num).or(progress.last_world),
                sample_world: rider_world(s, standing.race_num).or(progress.sample_world),
                refused_path_m: 0.0,
                travelled_m: 0.0,
                body_travelled_m: 0.0,
                lap_base_m: 0.0,
                laps_at_base: standing.num_laps,
                armed: true,
                corrupt: false,
                gates_seen: 0,
                raw_ignored: false,
                stripe_pin: false,
            };
            continue;
        }
        // The lap count arrived without a fraction. Keep them just past the
        // line, ahead of a bike still at ~0, until `track_pos` returns.
        if standing.num_laps > progress.laps_at_base && frac.is_none() {
            let pin_frac = (STRIPE_M / s.track_length).min(0.49);
            let into = STRIPE_M.min(s.track_length);
            progress.stripe_pin = true;
            progress.unwrapped = Some(standing.num_laps as f32 + pin_frac);
            progress.pending_frac = None;
            progress.raw_ignored = false;
            progress.refused_path_m = 0.0;
            progress.last_frac = Some(pin_frac);
            progress.lap_base_m = progress.travelled_m - into;
            progress.laps_at_base = standing.num_laps;
            progress.armed = true;
            progress.corrupt = false;
            progress.gates_seen = standing.sector_gate.max(0);
            continue;
        }
        if frac.is_some() {
            progress.stripe_pin = false;
        }
        let world = rider_world(s, standing.race_num);
        let step_m = match (progress.last_frac, frac) {
            (Some(last), Some(now)) => wrap_signed(now - last) * s.track_length,
            _ => 0.0,
        };
        // A step the body did not ride is not travel and not a new coordinate. A
        // missing body does not freeze the rider. Leave the last fraction so the
        // next tick still sees the whole unmatched step.
        let body_missed_step = frac.is_some()
            && step_m.abs() > PASS_M
            && step_m.abs() <= MAX_STEP_M
            && !body_followed(progress, world, step_m);
        if !body_missed_step {
            if progress.last_frac.is_some() && frac.is_some() && step_m.abs() <= MAX_STEP_M {
                    progress.travelled_m += step_m;
                    if let Some(moved) = body_step_m(progress, world) {
                        // One tick over the step cap is not a lap the body rode.
                        progress.body_travelled_m += moved.min(MAX_STEP_M);
                    }
                    if cold_arm
                        && !progress.armed
                        && !progress.corrupt
                        && step_m.abs() > 0.01
                    {
                        progress.armed = true;
                    }
            }
            progress.last_frac = frac;
            if !clock.in_gate {
                let pack_fraction = frac.and_then(|fraction| confirming_fraction(s, standing, fraction));
                note_unwrapped(progress, frac, s.track_length, world, pack_fraction);
            }
            if progress.raw_ignored {
                add_refused_path(progress, world);
            }
        } else {
            add_refused_path(progress, world);
        }
        if world.is_some() {
            progress.sample_world = world;
        }
        if standing.num_laps > progress.laps_at_base {
            progress.lap_base_m = progress.travelled_m;
            progress.laps_at_base = standing.num_laps;
            progress.armed = progress.last_frac.is_some();
            progress.corrupt = false;
            progress.gates_seen = standing.sector_gate.max(0);
        } else if standing.num_laps < progress.laps_at_base {
            progress.stripe_pin = false;
            progress.laps_at_base = standing.num_laps;
            progress.armed = false;
            progress.corrupt = false;
            progress.gates_seen = standing.sector_gate.max(0);
        } else if standing.sector_gate > progress.gates_seen {
            // Lap 1 scores travel since this gate. A corrupt pin starts again from
            // here too. A healthy armed rider already has the right metres; moving
            // the base would drop them to 0 m and swap the field at every split.
            if standing.num_laps == 0 || progress.corrupt {
                progress.lap_base_m = progress.travelled_m;
                progress.armed = progress.last_frac.is_some();
                progress.corrupt = false;
            }
            progress.gates_seen = standing.sector_gate;
        } else if standing.sector_gate < progress.gates_seen {
            progress.gates_seen = standing.sector_gate.max(0);
        }
        let into_lap = progress.travelled_m - progress.lap_base_m;
        if progress.armed && into_lap > s.track_length + LAP_OVERFLOW_SLACK_M {
            // Multi-lap invent without a bump: pin to game place (no S/F score).
            progress.armed = false;
            progress.corrupt = true;
        }
    }
    if on_gate {
        if let Some(origin) = gate_origin_frac(&tracked) {
            for progress in tracked.iter_mut() {
                if let Some(gate) = progress.gate_frac {
                    progress.unwrapped = Some(origin + forward_frac(origin, gate));
                    progress.pending_frac = None;
                }
            }
        }
    } else {
        // A folded group beside a larger pack is scored with that pack. Store the
        // pull while the body is here, so a dropped position cannot freeze the
        // projection at the front.
        let mut pulls = Vec::new();
        for standing in &s.standings[..n] {
            if tracked
                .iter()
                .any(|progress| progress.race_num == standing.race_num && progress.stripe_pin)
            {
                continue;
            }
            if rider_world(s, standing.race_num).is_none() {
                continue;
            }
            let Some(stored) = tracked
                .iter()
                .find(|progress| progress.race_num == standing.race_num)
                .and_then(|progress| progress.unwrapped)
            else {
                continue;
            };
            let Some(pulled) = scored_coordinate(s, &tracked, standing) else {
                continue;
            };
            if spline_gap(
                stored.rem_euclid(1.0),
                pulled.rem_euclid(1.0),
                s.track_length,
            ) < FOLD_ALONG_M
            {
                continue;
            }
            // A short gap on the raw fraction can order this pair the other way.
            // That order does not pull back a lap the body actually rode.
            if earned_lap_beside(s, &tracked, standing) {
                continue;
            }
            if let Some(index) = tracked
                .iter()
                .position(|progress| progress.race_num == standing.race_num)
            {
                pulls.push((index, pulled));
            }
        }
        for (index, pulled) in pulls {
            tracked[index].unwrapped = Some(pulled);
            tracked[index].raw_ignored = true;
            tracked[index].pending_frac = None;
        }
    }
}

/// Closest continuous coordinate to `prev` whose fractional part is `now`.
fn unwrap_near(prev: f32, now: f32) -> f32 {
    let now = now.rem_euclid(1.0);
    now + (prev - now).round()
}

/// Move the scored coordinate to where the bike is. The first sample is accepted.
/// A jump over `MAX_STEP_M` waits one tick; the next sample becomes the place only
/// when the body rode that step. Two samples of a fraction the bike did not ride
/// stay dropped. When that fraction matches the bikes beside them, they join that
/// pack's median instead of leaping ahead of it. Once the raw fraction is moving
/// continuously and the body is riding that small step, the frozen coordinate
/// snaps to that fraction. It does not have to cover the gap back to the winding
/// the fold left behind.
fn note_unwrapped(
    progress: &mut RiderProgress,
    now: Option<f32>,
    track_length: f32,
    world: Option<(f32, f32)>,
    pack_fraction: Option<f32>,
) {
    let Some(now) = now else {
        return;
    };
    let Some(prev) = progress.unwrapped else {
        progress.unwrapped = Some(now.rem_euclid(1.0));
        progress.refused_path_m = 0.0;
        if world.is_some() {
            progress.last_world = world;
        }
        return;
    };
    let candidate = unwrap_near(prev, now);
    let step_m = (candidate - prev).abs() * track_length;
    if step_m <= MAX_STEP_M {
        // The body is still on the previous sample. Do not walk the coordinate
        // along a spline step it did not ride.
        if step_m > PASS_M && !body_followed(progress, world, step_m) {
            progress.raw_ignored = true;
            return;
        }
        progress.unwrapped = Some(candidate);
        progress.raw_ignored = false;
        progress.refused_path_m = 0.0;
        progress.pending_frac = None;
        if world.is_some() {
            progress.last_world = world;
        }
        return;
    }
    if body_stayed(progress, world, step_m) {
        if let Some(median) = pack_fraction {
            // The raw fraction can sit ahead of the bikes the body is beside.
            // Join their median, on the same winding as the coordinate we had.
            progress.unwrapped = Some(unwrap_near(prev, median));
            progress.raw_ignored = true;
            progress.pending_frac = None;
            if world.is_some() {
                progress.last_world = world;
            }
            return;
        }
        progress.raw_ignored = true;
        progress.pending_frac = None;
        return;
    }
    if let Some(pending) = progress.pending_frac {
        let stay_m = wrap_signed(now - pending).abs() * track_length;
        if stay_m <= MAX_STEP_M {
            // The raw fraction is continuous. Accept it when the body rode this
            // small step. The gap back to the frozen coordinate is the fold
            // already refused — do not ask the body to cover it, and do not
            // short-wrap onto that winding. A fraction that sits still, while
            // the body has not ridden that gap, is still the jump.
            let held_jump = stay_m <= PASS_M && !body_followed(progress, world, step_m);
            if held_jump || !body_followed(progress, world, stay_m) {
                progress.raw_ignored = true;
                progress.pending_frac = None;
                return;
            }
            progress.unwrapped = Some(now.rem_euclid(1.0));
            progress.raw_ignored = false;
            progress.refused_path_m = 0.0;
            progress.pending_frac = None;
            if world.is_some() {
                progress.last_world = world;
            }
            return;
        }
    }
    progress.raw_ignored = true;
    progress.pending_frac = Some(now);
}

/// Metres the body moved since the previous sample.
fn sample_step_m(progress: &RiderProgress, world: Option<(f32, f32)>) -> Option<f32> {
    let (previous_x, previous_z) = progress.sample_world?;
    let (x, z) = world?;
    let dx = x - previous_x;
    let dz = z - previous_z;
    Some((dx * dx + dz * dz).sqrt())
}

/// Add this tick's body step onto the path walked while the coordinate is refused.
fn add_refused_path(progress: &mut RiderProgress, world: Option<(f32, f32)>) {
    let Some(moved) = sample_step_m(progress, world) else {
        return;
    };
    progress.refused_path_m += moved.min(MAX_STEP_M);
}

/// The spline stepped a long way and the bike is still where it was.
fn body_stayed(progress: &RiderProgress, world: Option<(f32, f32)>, step_m: f32) -> bool {
    let (Some((previous_x, previous_z)), Some((x, z))) = (progress.last_world, world) else {
        return false;
    };
    let dx = x - previous_x;
    let dz = z - previous_z;
    let moved = (dx * dx + dz * dz).sqrt();
    beside_on_track(moved, step_m, BODY_CLOSE_M)
}

/// The body rode this spline step. A missing body does not freeze the rider.
/// A step inside the pass margin is too small to call a fold.
fn body_followed(progress: &RiderProgress, world: Option<(f32, f32)>, step_m: f32) -> bool {
    if step_m.abs() <= PASS_M {
        return true;
    }
    let Some(moved) = body_step_m(progress, world) else {
        return true;
    };
    moved >= step_m.abs() * FOLD_RATIO
}

/// Metres the body moved since the coordinate last accepted a sample.
fn body_step_m(progress: &RiderProgress, world: Option<(f32, f32)>) -> Option<f32> {
    let (previous_x, previous_z) = progress.last_world?;
    let (x, z) = world?;
    let dx = x - previous_x;
    let dz = z - previous_z;
    Some((dx * dx + dz * dz).sqrt())
}

/// Metres into the lap they are scored on, when we saw that lap start.
/// Capped to one lap: raw travel past `track_length` without a `num_laps` rise is clamped.
fn lap_progress_m(tracked: &[RiderProgress], st: &Standing, track_length: f32) -> Option<f32> {
    tracked
        .iter()
        .find(|p| p.race_num == st.race_num)
        .filter(|p| p.armed && !p.corrupt && p.laps_at_base == st.num_laps)
        .map(|p| (p.travelled_m - p.lap_base_m).clamp(0.0, track_length))
}

fn progress_corrupt(tracked: &[RiderProgress], race_num: i32) -> bool {
    tracked
        .iter()
        .find(|p| p.race_num == race_num)
        .is_some_and(|p| p.corrupt)
}

/// `track_pos` as a 0..1 lap fraction. The plugin sends metres on some tracks.
/// An exact lap (`1`, or one track length in metres) stays at the end of this lap.
/// `rem_euclid` would turn it into `0`, and the open-lap arc then scores that bike
/// behind the pack still at `0.90`.
pub(crate) fn norm_lap_pos(s: &Snapshot, pos: f32) -> f32 {
    if pos < 0.0 {
        return pos;
    }
    let laps = if pos > 1.5 && s.track_length > 10.0 {
        pos / s.track_length
    } else {
        pos
    };
    let wrapped = laps.rem_euclid(1.0);
    if wrapped == 0.0 && laps > 0.0 {
        1.0 - f32::EPSILON
    } else {
        wrapped
    }
}

/// Signed shortest way round the lap, in lap fractions. Positive means `d` is ahead.
fn wrap_signed(d: f32) -> f32 {
    let d = d.rem_euclid(1.0);
    if d > 0.5 {
        d - 1.0
    } else {
        d
    }
}

fn rider_lap_pos(s: &Snapshot, race_num: i32) -> Option<f32> {
    let live = s
        .riders
        .iter()
        .take(s.rider_count.max(0) as usize)
        .find(|r| r.race_num == race_num)
        .map(|r| r.track_pos)
        .filter(|p| *p >= 0.0);
    let focus = if s.focus_race_num > 0 {
        s.focus_race_num
    } else {
        s.local_race_num
    };
    let raw = match live {
        Some(p) => p,
        None if race_num == focus && s.local_track_pos >= 0.0 => s.local_track_pos,
        None => return None,
    };
    Some(norm_lap_pos(s, raw))
}

/// Live order only makes sense in a race that is running. A practice or warmup field is
/// ranked by best lap, and on the gate everyone sits on the same stretch of track.
///
/// Read from the clock this tick already built rather than from `is_warmup` /
/// `is_practice_session` / `IN_GATE`: the session heuristics latch state as they answer,
/// and re-asking them here would rearm the flag and lap machine after the banner for
/// this frame was decided. `ClockMode::from_state` already folds both practice presets
/// into [`ClockMode::Practice`].
fn live_order_active(s: &Snapshot, clock: &SessionClock) -> bool {
    s.on_track != 0
        && s.track_length > 10.0
        && s.rider_count >= 2
        && !clock.in_gate
        && !matches!(clock.mode, ClockMode::Practice | ClockMode::Gate)
}

fn out_of_race(st: &Standing) -> bool {
    matches!(st.state, 1 | 3 | 4)
}

/// Scored and cruising: after the leader takes the flag a cool-down pass must not move
/// anyone in the results. Reads the latch `build_clock` already set this tick — calling
/// `leader_finished` / `effective_race_laps` here would move it.
pub(crate) fn done_racing(st: &Standing) -> bool {
    let base = LEADER_FIN_LOCAL_BASE.load(Ordering::Relaxed);
    base >= 0 && st.num_laps > base
}

fn prev_rank(prev: &[i32], race_num: i32) -> usize {
    prev.iter()
        .position(|&n| n == race_num)
        .unwrap_or(usize::MAX)
}

/// Start/finish as a lap fraction. A crossing we watched beats `sf_meters`.
/// `sf_meters == 0` with nothing learned is "unknown", not the origin: the centerline
/// fraction is not measured from the line.
fn line_known(s: &Snapshot) -> bool {
    START_FINISH_FRACTION_LEARNED.load(Ordering::Relaxed) >= 0 || s.sf_meters > 0.0
}

fn start_finish_fraction(s: &Snapshot) -> f32 {
    let learned = START_FINISH_FRACTION_LEARNED.load(Ordering::Relaxed);
    if learned >= 0 {
        return (learned as f32 / 10_000.0).rem_euclid(1.0);
    }
    if s.track_length > 1.0 && s.sf_meters > 0.0 {
        (s.sf_meters / s.track_length).rem_euclid(1.0)
    } else {
        0.0
    }
}

/// Forward distance along the lap, wrapping at the centerline origin. Same rule as sectors.
fn forward_frac(from: f32, to: f32) -> f32 {
    (to - from).rem_euclid(1.0)
}

/// Lap 1, before a finish arms the tracker. Known S1/S2 fractions order the whole
/// field from track position. Without those fractions, a published sector gate plus
/// travel since that gate does the same job until the line.
fn open_lap_sector_metres(s: &Snapshot, tracked: &[RiderProgress], st: &Standing) -> Option<f32> {
    if st.num_laps > 0 || out_of_race(st) {
        return None;
    }
    let ends = crate::sector::split_fracs();
    if ends[0] > 0.05 && ends[1] > 0.05 {
        let frac = pack_lap_pos(s, tracked, st)?;
        return Some(geometric_open_lap_metres(
            s.track_length,
            st.sector_gate,
            frac,
            ends[0],
            ends[1],
        ));
    }
    let gate = st.sector_gate.clamp(0, 2);
    if gate <= 0 {
        return None;
    }
    let span = s.track_length / 3.0;
    let since = lap_progress_m(tracked, st, s.track_length)
        .unwrap_or(0.0)
        .clamp(0.0, (span - PASS_M).max(0.0));
    Some(gate as f32 * span + since)
}

/// Metres from the start of the open lap using S1 (and S2 when known). The arc from
/// S2 back to S1 also holds the run to the line, and the origin is not the line.
/// Once the sector gate is 2, that whole arc counts as past S2. Until the gate
/// publishes, only the half closer to S2 does — the half closer to S1 stays before
/// S1, so the start grid is not scored as a finished sector. Those two halves meet
/// the S1–S2 score at each gate, so a late split cannot drop a rider a full lap.
fn geometric_open_lap_metres(
    track_length: f32,
    published: i32,
    frac: f32,
    s1: f32,
    s2: f32,
) -> f32 {
    let published = published.clamp(0, 2);
    let s1_to_s2 = forward_frac(s1, s2);
    let from_s1 = forward_frac(s1, frac);
    if from_s1 > 0.0 && from_s1 < s1_to_s2 {
        return from_s1 * track_length;
    }
    let after_s2 = forward_frac(s2, frac);
    if published >= 2 || after_s2 <= forward_frac(frac, s1) {
        return (s1_to_s2 + after_s2) * track_length;
    }
    -forward_frac(frac, s1) * track_length
}

/// Fraction at the back of the gate. The candidate that puts every stored grid
/// fraction in the smallest forward span, so a grid that wraps past 0 still has a back.
fn gate_origin_frac(tracked: &[RiderProgress]) -> Option<f32> {
    let fracs: Vec<f32> = tracked.iter().filter_map(|p| p.gate_frac).collect();
    let mut best = *fracs.first()?;
    let mut best_span = f32::MAX;
    for &candidate in &fracs {
        let span = fracs
            .iter()
            .map(|frac| forward_frac(candidate, *frac))
            .fold(0.0f32, f32::max);
        if span < best_span {
            best_span = span;
            best = candidate;
        }
    }
    Some(best)
}

/// Where a rider we saw on the gate is, in metres from the back of that grid.
/// The coordinate is not reset when `num_laps` rises. `num_laps` only lifts a rider
/// whose coordinate is a full lap behind the classification. A missing `track_pos`
/// keeps the last coordinate, so a crash can still be passed.
fn gate_place_metres(s: &Snapshot, tracked: &[RiderProgress], st: &Standing) -> Option<f32> {
    if out_of_race(st) {
        return None;
    }
    let origin = gate_origin_frac(tracked)?;
    let progress = tracked.iter().find(|p| p.race_num == st.race_num)?;
    if progress.gate_frac.is_none() {
        return None;
    }
    let stored = progress.unwrapped?;
    let signed = scored_coordinate(s, tracked, st).unwrap_or(stored) - origin;
    let class_laps = st.num_laps as f32;
    let laps = if signed + 1.0 < class_laps {
        class_laps + signed.rem_euclid(1.0)
    } else {
        signed
    };
    Some(laps * s.track_length)
}

/// Every stored gate rider is still within [`GATE_SLOT_M`] of the slot we recorded.
/// No gate snapshot means the field is not being held.
fn field_still_in_slots(s: &Snapshot, tracked: &[RiderProgress]) -> bool {
    let Some(origin) = gate_origin_frac(tracked) else {
        return false;
    };
    let mut saw = false;
    for progress in tracked {
        let Some(gate) = progress.gate_frac else {
            continue;
        };
        saw = true;
        let slot = origin + forward_frac(origin, gate);
        let now = progress.unwrapped.unwrap_or(slot);
        if (now - slot).abs() * s.track_length > GATE_SLOT_M {
            return false;
        }
    }
    saw
}

/// The gate was seen and the game has not called the holeshot yet. The front slot is
/// not a place. A long lead with no callback stops holding. The metres are what the
/// body has ridden since the gate: a folded slot can sit 200 m up the lap while the
/// bike is still in the grid.
fn awaiting_holeshot(s: &Snapshot, tracked: &[RiderProgress]) -> bool {
    if s.holeshot_race_num > 0 {
        return false;
    }
    if gate_origin_frac(tracked).is_none() {
        return false;
    }
    let mut saw = false;
    let mut furthest = 0.0f32;
    for progress in tracked {
        if progress.gate_frac.is_none() {
            continue;
        }
        saw = true;
        furthest = furthest.max(progress.body_travelled_m);
    }
    saw && furthest < HOLESHOT_FALLBACK_M
}

/// Metres into the lap when we know where it started: the tracker after the gate or a
/// crossing, otherwise metres past a known start/finish.
/// Runaway/corrupt riders return `None` so they keep their game place (no S/F collapse
/// to ~0 just after a wrap before `num_laps` bumps). The open lap uses the grid
/// snapshot when we saw the gate, otherwise sector geometry, so a pin cannot freeze
/// the start.
fn lap_metres(s: &Snapshot, tracked: &[RiderProgress], st: &Standing) -> Option<f32> {
    if st.num_laps == 0 {
        if let Some(metres) = open_lap_sector_metres(s, tracked, st) {
            return Some(metres);
        }
    }
    if progress_corrupt(tracked, st.race_num) {
        return None;
    }
    if let Some(progress) = lap_progress_m(tracked, st, s.track_length) {
        return Some(progress);
    }
    if !line_known(s) {
        return None;
    }
    let frac = pack_lap_pos(s, tracked, st)?;
    Some((frac - start_finish_fraction(s)).rem_euclid(1.0) * s.track_length)
}

/// Laps completed, plus metres into this lap, minus this rider's own penalty.
/// Higher is further up the race. `None` when we cannot place them on track.
fn rider_score(s: &Snapshot, tracked: &[RiderProgress], st: &Standing) -> Option<f32> {
    let metres = track_score(s, tracked, st)?;
    Some(metres - penalty_m(s, st.penalty_ms))
}

/// Laps completed plus metres into this lap, ignoring penalties.
fn track_score(s: &Snapshot, tracked: &[RiderProgress], st: &Standing) -> Option<f32> {
    if out_of_race(st) {
        return None;
    }
    if let Some(metres) = gate_place_metres(s, tracked, st) {
        return Some(metres);
    }
    let metres = lap_metres(s, tracked, st)?;
    Some(st.num_laps as f32 * s.track_length + metres)
}

/// Length of the published centerline when it is the same lap `track_pos` is measured
/// on. `None` when the line is missing or too short to trust.
fn centerline_length(snapshot: &Snapshot) -> Option<f32> {
    let count = snapshot.poly_count.max(0) as usize;
    if count < 2 || count > snapshot.poly.len() || snapshot.track_length <= 10.0 {
        return None;
    }
    let mut length = 0.0f32;
    for index in 1..count {
        let dx = snapshot.poly[index].x - snapshot.poly[index - 1].x;
        let dz = snapshot.poly[index].z - snapshot.poly[index - 1].z;
        length += (dx * dx + dz * dz).sqrt();
    }
    let slack = (length - snapshot.track_length).abs() / snapshot.track_length;
    if length < 1.0 || slack > POLY_LENGTH_SLACK {
        return None;
    }
    Some(length)
}

/// World point at `fraction` along the published centerline. `fraction` 0 is the
/// first point, the centerline origin `track_pos` is measured from.
fn centerline_sample(snapshot: &Snapshot, length: f32, fraction: f32) -> Option<(f32, f32)> {
    let count = snapshot.poly_count.max(0) as usize;
    if count < 2 || count > snapshot.poly.len() || length < 1.0 {
        return None;
    }
    let target = fraction.rem_euclid(1.0) * length;
    let mut walked = 0.0f32;
    for index in 1..count {
        let start = snapshot.poly[index - 1];
        let end = snapshot.poly[index];
        let dx = end.x - start.x;
        let dz = end.z - start.z;
        let span = (dx * dx + dz * dz).sqrt();
        if walked + span >= target || index + 1 == count {
            let blend = if span > 0.001 {
                ((target - walked) / span).clamp(0.0, 1.0)
            } else {
                0.0
            };
            return Some((start.x + dx * blend, start.z + dz * blend));
        }
        walked += span;
    }
    None
}

/// World `x`/`z` for a rider who still has a `track_pos`. Missing when they have
/// dropped off the bike list, unless they are the focus rider.
fn rider_world(snapshot: &Snapshot, race_num: i32) -> Option<(f32, f32)> {
    let count = snapshot.rider_count.max(0) as usize;
    if let Some(rider) = snapshot.riders[..count.min(snapshot.riders.len())]
        .iter()
        .find(|rider| rider.race_num == race_num)
    {
        if rider.track_pos < 0.0 {
            return None;
        }
        return Some((rider.x, rider.z));
    }
    let focus = if snapshot.focus_race_num > 0 {
        snapshot.focus_race_num
    } else {
        snapshot.local_race_num
    };
    if race_num == focus && snapshot.local_track_pos >= 0.0 {
        return Some((snapshot.local_x, snapshot.local_z));
    }
    None
}

fn beside_on_track(world: f32, along: f32, cap: f32) -> bool {
    world < cap && world < along * FOLD_RATIO
}

struct NearbyRider {
    frac: f32,
    world: f32,
    coordinate: f32,
}

fn spline_gap(from: f32, to: f32, track_length: f32) -> f32 {
    wrap_signed(from - to).abs() * track_length
}

/// The short way from `from` to `to` crosses the centerline origin.
fn fractions_wrap_across_origin(from: f32, to: f32) -> bool {
    (from.rem_euclid(1.0) - to.rem_euclid(1.0)).abs() > 0.5
}

fn circular_median(fractions: &[f32]) -> Option<f32> {
    let mut best = *fractions.first()?;
    let mut best_cost = f32::MAX;
    for &candidate in fractions {
        let cost = fractions.iter().fold(0.0f32, |sum, fraction| {
            sum + wrap_signed(fraction - candidate).abs()
        });
        if cost < best_cost {
            best_cost = cost;
            best = candidate;
        }
    }
    Some(best)
}

fn linear_median(values: &[f32]) -> Option<f32> {
    let mut best = *values.first()?;
    let mut best_cost = f32::MAX;
    for &candidate in values {
        let cost = values
            .iter()
            .fold(0.0f32, |sum, value| sum + (value - candidate).abs());
        if cost < best_cost {
            best_cost = cost;
            best = candidate;
        }
    }
    Some(best)
}

/// Same-lap bikes within [`BODY_CLOSE_M`]. A lapped rider beside the leader is not
/// in this set.
fn nearby_riders(
    snapshot: &Snapshot,
    tracked: &[RiderProgress],
    standing: &Standing,
) -> Vec<NearbyRider> {
    let Some((origin_x, origin_z)) = rider_world(snapshot, standing.race_num) else {
        return Vec::new();
    };
    let count = snapshot.standing_count.max(0) as usize;
    let mut nearby = Vec::new();
    for other in &snapshot.standings[..count.min(snapshot.standings.len())] {
        if other.race_num == standing.race_num
            || other.num_laps != standing.num_laps
            || out_of_race(other)
        {
            continue;
        }
        let Some(fraction) = rider_lap_pos(snapshot, other.race_num) else {
            continue;
        };
        let Some((x, z)) = rider_world(snapshot, other.race_num) else {
            continue;
        };
        let dx = x - origin_x;
        let dz = z - origin_z;
        let world = (dx * dx + dz * dz).sqrt();
        if world >= BODY_CLOSE_M {
            continue;
        }
        let coordinate = tracked
            .iter()
            .find(|progress| progress.race_num == other.race_num)
            .and_then(|progress| progress.unwrapped)
            .unwrap_or(fraction);
        nearby.push(NearbyRider {
            frac: fraction,
            world,
            coordinate,
        });
    }
    nearby
}

/// One branch. 15% of a short lap, and never more than [`FOLD_ALONG_M`], so a
/// 200 m track does not treat a third of the lap as one straight. On 1083 m
/// this is still 80 m.
fn same_branch_m(track_length: f32) -> f32 {
    FOLD_ALONG_M.min(track_length * 0.15)
}

fn on_same_branch(fraction: f32, other: f32, track_length: f32) -> bool {
    spline_gap(fraction, other, track_length) < same_branch_m(track_length)
}

/// The new fraction sits with at least three bikes beside this rider.
fn confirming_fraction(snapshot: &Snapshot, standing: &Standing, fraction: f32) -> Option<f32> {
    if snapshot.track_length <= 10.0 {
        return None;
    }
    let nearby = nearby_riders(snapshot, &[], standing);
    if nearby.len() < 3 {
        return None;
    }
    let agreeing: Vec<f32> = nearby
        .iter()
        .filter(|rider| on_same_branch(rider.frac, fraction, snapshot.track_length))
        .map(|rider| rider.frac)
        .collect();
    if agreeing.len() < 3 {
        return None;
    }
    circular_median(&agreeing)
}

/// A same-lap bike further up this line, far enough that the bodies match the spline.
/// The parallel lane inside [`BODY_CLOSE_M`] is not that bike.
fn fraction_confirmed(snapshot: &Snapshot, standing: &Standing, fraction: f32) -> bool {
    if snapshot.track_length <= 10.0 {
        return false;
    }
    let Some((origin_x, origin_z)) = rider_world(snapshot, standing.race_num) else {
        return false;
    };
    let count = snapshot.standing_count.max(0) as usize;
    let length = snapshot.track_length;
    snapshot.standings[..count.min(snapshot.standings.len())]
        .iter()
        .filter(|other| {
            other.race_num != standing.race_num
                && other.num_laps == standing.num_laps
                && !out_of_race(other)
        })
        .any(|other| {
            let Some(other_fraction) = rider_lap_pos(snapshot, other.race_num) else {
                return false;
            };
            let along = spline_gap(fraction, other_fraction, length);
            if along < FOLD_ALONG_M {
                return false;
            }
            let Some((x, z)) = rider_world(snapshot, other.race_num) else {
                return false;
            };
            let dx = x - origin_x;
            let dz = z - origin_z;
            let world = (dx * dx + dz * dz).sqrt();
            world >= along * FOLD_RATIO
        })
}

fn pull_to_pack(stored: f32, fraction: f32, pack: &[&NearbyRider], track_length: f32) -> Option<f32> {
    let coordinates: Vec<f32> = pack.iter().map(|rider| rider.coordinate).collect();
    let level = linear_median(&coordinates)?;
    let snapped = unwrap_near(level, fraction);
    let along = (snapped - stored).abs() * track_length;
    let world = pack.iter().map(|rider| rider.world).fold(f32::MAX, f32::min);
    if along >= FOLD_ALONG_M && beside_on_track(world, along, BODY_CLOSE_M) {
        Some(snapped)
    } else {
        None
    }
}

/// A larger same-lap group on another branch. `None` when this rider already sits
/// with a group at least that big, or the published line between them comes back
/// with the bikes. That chord keeps the higher fraction.
fn dominant_branch(
    snapshot: &Snapshot,
    stored_fraction: f32,
    nearby: &[NearbyRider],
    track_length: f32,
) -> Option<f32> {
    let own_count = 1 + nearby
        .iter()
        .filter(|rider| on_same_branch(rider.frac, stored_fraction, track_length))
        .count();
    let mut best_seed = 0.0f32;
    let mut best_count = 0usize;
    let mut best_world = f32::MAX;
    for rider in nearby {
        if on_same_branch(rider.frac, stored_fraction, track_length) {
            continue;
        }
        let count = nearby
            .iter()
            .filter(|other| on_same_branch(other.frac, rider.frac, track_length))
            .count();
        let world = nearby
            .iter()
            .filter(|other| on_same_branch(other.frac, rider.frac, track_length))
            .map(|other| other.world)
            .fold(f32::MAX, f32::min);
        if count > best_count || (count == best_count && world < best_world) {
            best_count = count;
            best_seed = rider.frac;
            best_world = world;
        }
    }
    if best_count < 3 || best_count <= own_count {
        return None;
    }
    let branch: Vec<&NearbyRider> = nearby
        .iter()
        .filter(|rider| on_same_branch(rider.frac, best_seed, track_length))
        .collect();
    let fractions: Vec<f32> = branch.iter().map(|rider| rider.frac).collect();
    let median = circular_median(&fractions)?;
    let along = spline_gap(median, stored_fraction, track_length);
    let world = branch.iter().map(|rider| rider.world).fold(f32::MAX, f32::min);
    if along < FOLD_ALONG_M || !beside_on_track(world, along, BODY_CLOSE_M) {
        return None;
    }
    // The line between this fraction and the pack comes back as close as the bikes.
    // A hairpin chord is that lap, so the higher fraction is not stored on the pack.
    if lap_folded_back(snapshot, stored_fraction, median, world) {
        return None;
    }
    // A projection sits ahead of the pack. A bike still behind that pack stays
    // on its own fraction — pulling it forward invents a lead on the returning lane.
    let ahead_of_pack = wrap_signed(stored_fraction - median) * track_length;
    if ahead_of_pack <= 0.0 {
        return None;
    }
    // A corner under FOLD_CORNER_M stays with the group already up the lap.
    // A longer gap whose bodies are still inside the larger pack is the other
    // branch. Wrapping across the start line is that projection either way.
    // A group that is actually further up the lap already returned above:
    // beside_on_track is false when the world gap matches the spline.
    if own_count >= 3
        && along < FOLD_CORNER_M
        && !fractions_wrap_across_origin(stored_fraction, median)
    {
        return None;
    }
    let coordinates: Vec<f32> = branch.iter().map(|rider| rider.coordinate).collect();
    let level = linear_median(&coordinates)?;
    Some(unwrap_near(level, median))
}

/// Stored coordinate, pulled back to the same-lap pack when it is a fold away from
/// the bikes beside this rider. The fraction and the lap integer both move.
fn scored_coordinate(
    snapshot: &Snapshot,
    tracked: &[RiderProgress],
    standing: &Standing,
) -> Option<f32> {
    let stored = tracked
        .iter()
        .find(|progress| progress.race_num == standing.race_num)
        .and_then(|progress| progress.unwrapped)
        .or_else(|| rider_lap_pos(snapshot, standing.race_num))?;
    if snapshot.track_length <= 10.0 {
        return Some(stored);
    }
    let nearby = nearby_riders(snapshot, tracked, standing);
    if nearby.len() < 3 {
        return Some(stored);
    }
    let stored_fraction = stored.rem_euclid(1.0);
    let length = snapshot.track_length;
    let own: Vec<&NearbyRider> = nearby
        .iter()
        .filter(|rider| on_same_branch(rider.frac, stored_fraction, length))
        .collect();
    if own.len() >= 3 {
        if let Some(pulled) = pull_to_pack(stored, stored_fraction, &own, length) {
            return Some(pulled);
        }
    }
    if !fraction_confirmed(snapshot, standing, stored_fraction) {
        if let Some(pulled) = dominant_branch(snapshot, stored_fraction, &nearby, length) {
            return Some(pulled);
        }
    }
    Some(stored)
}

/// Lap fraction the order is allowed to score. The pack median replaces a stored
/// fraction that the bikes within [`BODY_CLOSE_M`] do not share.
fn pack_lap_pos(
    snapshot: &Snapshot,
    tracked: &[RiderProgress],
    standing: &Standing,
) -> Option<f32> {
    Some(scored_coordinate(snapshot, tracked, standing)?.rem_euclid(1.0))
}

fn has_same_lap_pack(snapshot: &Snapshot, standing: &Standing) -> bool {
    nearby_riders(snapshot, &[], standing).len() >= 3
}

/// The published line at these two fractions comes back about as close as the bikes.
/// The spline gap is then the long way around the lap. Bodies inside [`FOLD_STRIP_M`]
/// are still beside each other. `false` when the line is missing or off the lap.
fn lap_folded_back(snapshot: &Snapshot, pos_a: f32, pos_b: f32, body_gap: f32) -> bool {
    if body_gap < FOLD_STRIP_M {
        return false;
    }
    let Some(length) = centerline_length(snapshot) else {
        return false;
    };
    let (Some((ax, az)), Some((bx, bz))) = (
        centerline_sample(snapshot, length, pos_a),
        centerline_sample(snapshot, length, pos_b),
    ) else {
        return false;
    };
    let dx = bx - ax;
    let dz = bz - az;
    let sample_gap = (dx * dx + dz * dz).sqrt();
    (sample_gap - body_gap).abs() <= FOLD_WORLD_M
}

/// The two lap fractions sit on lanes that run beside each other. `track_pos` says
/// one bike is well ahead, but the bikes themselves — or, if a body is missing, the
/// centerline samples — are only a short distance apart. When the line itself comes
/// back with the bikes, that gap is the lap folding, not the next lane.
fn different_lane(
    snapshot: &Snapshot,
    tracked: &[RiderProgress],
    behind: &Standing,
    ahead: &Standing,
) -> bool {
    let (Some(behind_pos), Some(ahead_pos)) = (
        pack_lap_pos(snapshot, tracked, behind),
        pack_lap_pos(snapshot, tracked, ahead),
    ) else {
        return false;
    };
    let along = (wrap_signed(ahead_pos - behind_pos) * snapshot.track_length).abs();
    if along <= PASS_M || along < FOLD_ALONG_M {
        return false;
    }
    if let (Some((behind_x, behind_z)), Some((ahead_x, ahead_z))) = (
        rider_world(snapshot, behind.race_num),
        rider_world(snapshot, ahead.race_num),
    ) {
        let dx = ahead_x - behind_x;
        let dz = ahead_z - behind_z;
        let world = (dx * dx + dz * dz).sqrt();
        if lap_folded_back(snapshot, behind_pos, ahead_pos, world) {
            return false;
        }
        return beside_on_track(world, along, BODY_CLOSE_M);
    }
    let Some(length) = centerline_length(snapshot) else {
        return false;
    };
    let (Some((behind_x, behind_z)), Some((ahead_x, ahead_z))) = (
        centerline_sample(snapshot, length, behind_pos),
        centerline_sample(snapshot, length, ahead_pos),
    ) else {
        return false;
    };
    let dx = ahead_x - behind_x;
    let dz = ahead_z - behind_z;
    let world = (dx * dx + dz * dz).sqrt();
    beside_on_track(world, along, FOLD_WORLD_M)
}

/// World distance between two riders who still have a body. `None` when either
/// body is missing.
fn body_gap(snapshot: &Snapshot, race_a: i32, race_b: i32) -> Option<f32> {
    let (ax, az) = rider_world(snapshot, race_a)?;
    let (bx, bz) = rider_world(snapshot, race_b)?;
    let dx = ax - bx;
    let dz = az - bz;
    Some((dx * dx + dz * dz).sqrt())
}

/// The rider ahead on the short wrap was already behind in `prev`, and the bodies
/// are not yet clearly past the lane. Keep that refusal so a metre of world gap
/// cannot release the lead.
fn refused_fold_holds(
    snapshot: &Snapshot,
    prev: &[i32],
    a: &Standing,
    b: &Standing,
    along_m: f32,
) -> bool {
    let Some(world) = body_gap(snapshot, a.race_num, b.race_num) else {
        return false;
    };
    if world < FOLD_STRIP_M || world >= FOLD_RELEASE_M || world >= along_m.abs() * FOLD_RATIO {
        return false;
    }
    if let (Some(pos_a), Some(pos_b)) = (
        rider_lap_pos(snapshot, a.race_num),
        rider_lap_pos(snapshot, b.race_num),
    ) {
        if lap_folded_back(snapshot, pos_a, pos_b, world) {
            return false;
        }
    }
    let (long_num, other_num) = if along_m > 0.0 {
        (b.race_num, a.race_num)
    } else {
        (a.race_num, b.race_num)
    };
    let long_rank = prev_rank(prev, long_num);
    let other_rank = prev_rank(prev, other_num);
    long_rank != usize::MAX && other_rank != usize::MAX && long_rank > other_rank
}

/// Same-lap bodies in the next lane.
/// `Some(true)` gives the place to `b`.
///
/// With a pack beside either rider, the corrected lap fraction is the order: the
/// bike that fraction says is ahead takes the place. Two bikes with no pack keep
/// the short-wrap lead from counting, so a projection onto the other branch does
/// not pass. That projection loses a place it already holds only when the bodies
/// are inside [`FOLD_STRIP_M`]. A chord near [`BODY_CLOSE_M`] keeps the centerline
/// order, unless the published line comes back as close as the bikes: that lead stays.
/// If that refusal is already the previous order, it holds until the bodies
/// are past [`FOLD_RELEASE_M`]. A gap under [`FOLD_CORNER_M`] that does not cross
/// the start line is one corner: the centerline order stands, including under 20 m.
fn folded_spline_lead(
    s: &Snapshot,
    prev: &[i32],
    tracked: &[RiderProgress],
    a: &Standing,
    b: &Standing,
    margin: f32,
) -> Option<bool> {
    let pos_a = pack_lap_pos(s, tracked, a)?;
    let pos_b = pack_lap_pos(s, tracked, b)?;
    let along_m = wrap_signed(pos_b - pos_a) * s.track_length;
    if along_m.abs() < FOLD_ALONG_M {
        return None;
    }
    // A stored coordinate a corner ahead is not the place when the raw fraction
    // has already stepped back inside the pack. The open-lap score uses that raw gap.
    if a.num_laps == 0 && b.num_laps == 0 && !out_of_race(a) && !out_of_race(b) {
        if let (Some(raw_a), Some(raw_b)) = (
            rider_lap_pos(s, a.race_num),
            rider_lap_pos(s, b.race_num),
        ) {
            let raw_m = same_lap_along_m(raw_a, raw_b, s.track_length);
            // The stored gap can be the next lane while the raw gap is still the
            // pack, in the same direction. Leave that pair to the raw fraction.
            if raw_m.abs() < FOLD_ALONG_M && raw_m.abs() > margin {
                return None;
            }
        }
    }
    // One corner on this lap. A wrap onto the other side, or a gap from
    // [`FOLD_CORNER_M`] up, is still the branch the short chord must not promote.
    let corner =
        along_m.abs() < FOLD_CORNER_M && !fractions_wrap_across_origin(pos_a, pos_b);
    let packed = has_same_lap_pack(s, a) || has_same_lap_pack(s, b);
    if !packed && !corner && refused_fold_holds(s, prev, a, b, along_m) {
        return Some(along_m < 0.0);
    }
    if !different_lane(s, tracked, a, b) {
        return None;
    }
    if packed || corner {
        return Some(along_m > margin);
    }
    if along_m >= FOLD_ALONG_M {
        Some(false)
    } else if along_m <= -FOLD_ALONG_M && line_known(s) {
        // A missing body is not inside the strip. The last coordinate stays,
        // so a dropped position cannot jump the riders still on the lap.
        // Until the line is known, that close pair is not a fold either.
        match body_gap(s, a.race_num, b.race_num) {
            Some(world) if world < FOLD_STRIP_M => Some(true),
            _ => None,
        }
    } else {
        None
    }
}

/// Metres from `from` to `to` on the same completed lap. The higher fraction is
/// ahead. A gap across the origin is decided by [`origin_crossing_ahead`].
fn same_lap_along_m(from: f32, to: f32, track_length: f32) -> f32 {
    (to - from) * track_length
}

/// Same completed lap, when the fraction gap crosses the origin.
/// `Some(true)` gives the place to `b`. `None` when the pair stays on one side,
/// the bikes match the direct lead, or they are inside a close fold.
///
/// The rider just past the origin is ahead when the bikes match the short way.
/// A longer gap the bikes do not match does not swap, on a short lap or a long one.
/// A direct lead the bikes have ridden stays with the score, so an extra lap of
/// travel still counts before `num_laps` bumps.
fn origin_crossing_ahead(
    snapshot: &Snapshot,
    a: &Standing,
    b: &Standing,
    margin: f32,
    use_penalty: bool,
) -> Option<bool> {
    let pos_a = rider_lap_pos(snapshot, a.race_num)?;
    let pos_b = rider_lap_pos(snapshot, b.race_num)?;
    let direct = pos_b - pos_a;
    if direct.abs() <= 0.5 {
        return None;
    }
    let direct_m = direct * snapshot.track_length;
    let short_m = wrap_signed(direct) * snapshot.track_length;
    let Some(world) = body_gap(snapshot, a.race_num, b.race_num) else {
        return None;
    };
    if gap_matches_bodies(world, short_m) && !gap_matches_bodies(world, direct_m) {
        let ahead_m = if use_penalty {
            short_m + penalty_m(snapshot, a.penalty_ms - b.penalty_ms)
        } else {
            short_m
        };
        return Some(ahead_m > margin);
    }
    if gap_matches_bodies(world, direct_m) || world < BODY_CLOSE_M {
        return None;
    }
    Some(false)
}

/// Raw gap from `behind` to `ahead` when the bodies are at least [`FOLD_RATIO`] of that
/// distance apart. A fold, where the bikes are close and the spline gap is long, is `None`.
/// Same completed lap: the gap is the direct fraction delta.
fn confirmed_raw_gap(snapshot: &Snapshot, behind: &Standing, ahead: &Standing) -> Option<f32> {
    let pos_behind = rider_lap_pos(snapshot, behind.race_num)?;
    let pos_ahead = rider_lap_pos(snapshot, ahead.race_num)?;
    let (behind_x, behind_z) = rider_world(snapshot, behind.race_num)?;
    let (ahead_x, ahead_z) = rider_world(snapshot, ahead.race_num)?;
    let along_m = same_lap_along_m(pos_behind, pos_ahead, snapshot.track_length);
    let dx = ahead_x - behind_x;
    let dz = ahead_z - behind_z;
    let world = (dx * dx + dz * dz).sqrt();
    // A short chord is the next lane only when the bodies are actually beside each
    // other. A long chord on a looping lap is a real lead on a short track or a long one.
    // When the published line comes back with the bikes, the spline gap is that lead.
    if world < BODY_CLOSE_M
        && world < along_m.abs() * FOLD_RATIO
        && !lap_folded_back(snapshot, pos_behind, pos_ahead, world)
    {
        return None;
    }
    Some(along_m)
}

/// The bikes are about as far apart as the lap fraction says. A few metres of
/// line width is noise. A jump that only lands near the next bike is not.
fn gap_matches_bodies(world: f32, along: f32) -> bool {
    let along = along.abs();
    let slack = PASS_M.max(along * 0.15);
    (world - along).abs() <= slack
}

/// The chord is outside [`BODY_CLOSE_M`] and under half the spline, and the
/// centerline leader has moved at least [`BODY_CLOSE_M`] since the coordinate
/// last accepted a sample. A projection whose body stayed put is not this bend.
fn bent_chord_the_body_rode(
    snapshot: &Snapshot,
    tracked: &[RiderProgress],
    raw_ahead: i32,
    world: f32,
    along_m: f32,
) -> bool {
    let along = along_m.abs();
    if world < BODY_CLOSE_M || world >= along * BEND_RATIO {
        return false;
    }
    let Some(progress) = tracked
        .iter()
        .find(|progress| progress.race_num == raw_ahead)
    else {
        return false;
    };
    let Some(moved) = body_step_m(progress, rider_world(snapshot, raw_ahead)) else {
        return false;
    };
    moved >= BODY_CLOSE_M
}

/// A corner under [`FOLD_CORNER_M`]. The chord is outside the next lane, the gap
/// does not cross the start line, and the raw leader has moved at least
/// [`BODY_CLOSE_M`] since the coordinate last accepted a sample. A matched gap
/// is not this corner, and a body that stayed put is still a projection.
fn corner_the_body_rode(
    snapshot: &Snapshot,
    tracked: &[RiderProgress],
    raw_ahead: i32,
    behind: &Standing,
    ahead: &Standing,
    world: f32,
    along_m: f32,
) -> bool {
    let along = along_m.abs();
    if along >= FOLD_CORNER_M || world < BODY_CLOSE_M || gap_matches_bodies(world, along) {
        return false;
    }
    let Some(pos_behind) = rider_lap_pos(snapshot, behind.race_num) else {
        return false;
    };
    let Some(pos_ahead) = rider_lap_pos(snapshot, ahead.race_num) else {
        return false;
    };
    if fractions_wrap_across_origin(pos_behind, pos_ahead) {
        return false;
    }
    let Some(progress) = tracked
        .iter()
        .find(|progress| progress.race_num == raw_ahead)
    else {
        return false;
    };
    let Some(moved) = body_step_m(progress, rider_world(snapshot, raw_ahead)) else {
        return false;
    };
    moved >= BODY_CLOSE_M
}

/// A direct gap past [`FOLD_CORNER_M`] and under half a lap. The chord is at least
/// [`SPREAD_CHORD_M`] and more than [`SPREAD_RATIO`] of the spline, so the bikes
/// are not folded into the next lane. The gap does not cross the start line, and
/// the raw leader has moved at least [`BODY_CLOSE_M`] since the coordinate last
/// accepted a sample. A matched gap is not this lead. A 45 m chord at 159 m
/// stays the other branch.
fn spread_chord_the_body_rode(
    snapshot: &Snapshot,
    tracked: &[RiderProgress],
    raw_ahead: i32,
    behind: &Standing,
    ahead: &Standing,
    world: f32,
    along_m: f32,
) -> bool {
    let along = along_m.abs();
    if along < FOLD_CORNER_M || along >= snapshot.track_length * 0.5 {
        return false;
    }
    if world < SPREAD_CHORD_M || world <= along * SPREAD_RATIO || gap_matches_bodies(world, along)
    {
        return false;
    }
    let Some(pos_behind) = rider_lap_pos(snapshot, behind.race_num) else {
        return false;
    };
    let Some(pos_ahead) = rider_lap_pos(snapshot, ahead.race_num) else {
        return false;
    };
    if fractions_wrap_across_origin(pos_behind, pos_ahead) {
        return false;
    }
    let Some(progress) = tracked
        .iter()
        .find(|progress| progress.race_num == raw_ahead)
    else {
        return false;
    };
    let Some(moved) = body_step_m(progress, rider_world(snapshot, raw_ahead)) else {
        return false;
    };
    moved >= BODY_CLOSE_M
}

/// The raw coordinate is refused, the gap is under [`FROZEN_CORNER_M`] and does
/// not cross the start line, and the body has walked at least [`BODY_CLOSE_M`]
/// since that refusal. The straight chord can still be inside the next lane.
/// A matched gap is not this corner, and a body that stayed put is still a spike.
fn frozen_corner_the_body_rode(
    snapshot: &Snapshot,
    tracked: &[RiderProgress],
    raw_ahead: i32,
    behind: &Standing,
    ahead: &Standing,
    world: f32,
    along_m: f32,
) -> bool {
    let along = along_m.abs();
    if along >= FROZEN_CORNER_M || gap_matches_bodies(world, along) {
        return false;
    }
    let Some(pos_behind) = rider_lap_pos(snapshot, behind.race_num) else {
        return false;
    };
    let Some(pos_ahead) = rider_lap_pos(snapshot, ahead.race_num) else {
        return false;
    };
    if fractions_wrap_across_origin(pos_behind, pos_ahead) {
        return false;
    }
    let Some(progress) = tracked
        .iter()
        .find(|progress| progress.race_num == raw_ahead)
    else {
        return false;
    };
    progress.raw_ignored && progress.refused_path_m >= BODY_CLOSE_M
}

/// `Some(true)` when `b` keeps the raw lead on a refused corner. `None` when
/// this pair is not that corner.
fn frozen_corner_order(
    snapshot: &Snapshot,
    tracked: &[RiderProgress],
    a: &Standing,
    b: &Standing,
    margin: f32,
    use_penalty: bool,
    along_m: f32,
    world: f32,
) -> Option<bool> {
    if along_m.abs() <= margin {
        return None;
    }
    let raw_ahead = if along_m > 0.0 { b.race_num } else { a.race_num };
    let raw_behind = if along_m > 0.0 { a } else { b };
    let raw_leader = if along_m > 0.0 { b } else { a };
    if !frozen_corner_the_body_rode(
        snapshot,
        tracked,
        raw_ahead,
        raw_behind,
        raw_leader,
        world,
        along_m,
    ) {
        return None;
    }
    let ahead_m = if use_penalty {
        along_m + penalty_m(snapshot, a.penalty_ms - b.penalty_ms)
    } else {
        along_m
    };
    Some(ahead_m > margin)
}

/// The stored coordinate is about a lap ahead of a bike beside this rider, and
/// the body travel matches that lap. Pack pull does not take the lap back.
fn earned_lap_beside(
    snapshot: &Snapshot,
    tracked: &[RiderProgress],
    standing: &Standing,
) -> bool {
    let count = snapshot.standing_count.max(0) as usize;
    snapshot.standings[..count.min(snapshot.standings.len())]
        .iter()
        .filter(|other| {
            other.race_num != standing.race_num
                && other.num_laps == standing.num_laps
                && !out_of_race(other)
        })
        .any(|other| {
            let Some(world) = body_gap(snapshot, standing.race_num, other.race_num) else {
                return false;
            };
            if world >= BODY_CLOSE_M {
                return false;
            }
            let Some(pos_standing) = rider_lap_pos(snapshot, standing.race_num) else {
                return false;
            };
            let Some(pos_other) = rider_lap_pos(snapshot, other.race_num) else {
                return false;
            };
            let along_m = same_lap_along_m(pos_standing, pos_other, snapshot.track_length);
            stored_lap_crossed(tracked, standing, other, along_m, snapshot.track_length)
        })
}

/// The stored coordinate and the wrapped fraction disagree by a full lap, and the
/// rider with the higher coordinate has body travel about that much further. A
/// spline lap the body did not ride is not a crossing.
fn stored_lap_crossed(
    tracked: &[RiderProgress],
    a: &Standing,
    b: &Standing,
    along_m: f32,
    track_length: f32,
) -> bool {
    let Some(progress_a) = tracked.iter().find(|progress| progress.race_num == a.race_num) else {
        return false;
    };
    let Some(progress_b) = tracked.iter().find(|progress| progress.race_num == b.race_num) else {
        return false;
    };
    let (Some(coord_a), Some(coord_b)) = (progress_a.unwrapped, progress_b.unwrapped) else {
        return false;
    };
    let coord_m = (coord_b - coord_a) * track_length;
    if (coord_m - along_m).abs() < track_length - LAP_OVERFLOW_SLACK_M {
        return false;
    }
    let travelled_m = if coord_b >= coord_a {
        progress_b.body_travelled_m - progress_a.body_travelled_m
    } else {
        progress_a.body_travelled_m - progress_b.body_travelled_m
    };
    // A missed lap is most of that lap, on a short track or a long one. The slack is
    // the same noise the tracker already allows past one lap.
    (travelled_m - coord_m.abs()).abs() <= track_length * 0.25
}

/// The laps the lower classification is short have been ridden since the last counted
/// lap. A coordinate that has reached the higher lap without that travel is not a
/// crossing. The slack is a quarter of this lap, on a short track or a long one.
fn rode_the_laps_short(
    tracked: &[RiderProgress],
    fewer: &Standing,
    more_laps: i32,
    track_length: f32,
) -> bool {
    let Some(progress) = tracked
        .iter()
        .find(|progress| progress.race_num == fewer.race_num)
    else {
        return false;
    };
    let laps_short = (more_laps - fewer.num_laps) as f32;
    if laps_short <= 0.0 {
        return false;
    }
    let into_lap = progress.travelled_m - progress.lap_base_m;
    into_lap + track_length * 0.25 >= laps_short * track_length
}

/// Short-way metres from `a` to `b` when that gap and the bodies are both inside
/// [`STRIPE_M`]. Positive means `b` is ahead. `None` when either gap is wider, or a
/// body is missing. The window is metres, so a short track and a long one match.
fn stripe_along(snapshot: &Snapshot, a: &Standing, b: &Standing) -> Option<f32> {
    let pos_a = rider_lap_pos(snapshot, a.race_num)?;
    let pos_b = rider_lap_pos(snapshot, b.race_num)?;
    let along_m = wrap_signed(pos_b - pos_a) * snapshot.track_length;
    if along_m.abs() > STRIPE_M {
        return None;
    }
    let world = body_gap(snapshot, a.race_num, b.race_num)?;
    if world > STRIPE_M {
        return None;
    }
    Some(along_m)
}

/// `b` is the extra copy of a body that already belongs to a better game place.
/// That copy does not take the place. A side-by-side pass is further than a metre.
fn copied_behind_owner(snapshot: &Snapshot, b: &Standing) -> bool {
    let Some(pos_b) = rider_lap_pos(snapshot, b.race_num) else {
        return false;
    };
    let count = snapshot.standing_count.max(0) as usize;
    let limit = count.min(snapshot.standings.len());
    snapshot.standings[..limit].iter().any(|other| {
        other.race_num != b.race_num
            && other.position < b.position
            && other.num_laps == b.num_laps
            && !out_of_race(other)
            && rider_lap_pos(snapshot, other.race_num)
                .is_some_and(|pos| (pos - pos_b).abs() <= 0.001)
            && body_gap(snapshot, other.race_num, b.race_num)
                .is_some_and(|world| world < COPIED_BODY_M)
    })
}

/// The classification lap is the place. The bodies agree with the raw fraction, so a
/// coordinate a lap ahead of `num_laps` is not a pass. A close fold is the other
/// branch: the fewer-lap bike is beside them, not ahead.
fn classified_lap_holds(snapshot: &Snapshot, fewer: &Standing, more: &Standing) -> bool {
    let Some(fewer_pos) = rider_lap_pos(snapshot, fewer.race_num) else {
        return false;
    };
    let Some(more_pos) = rider_lap_pos(snapshot, more.race_num) else {
        return false;
    };
    let Some(world) = body_gap(snapshot, fewer.race_num, more.race_num) else {
        return false;
    };
    let along = wrap_signed(fewer_pos - more_pos) * snapshot.track_length;
    if gap_matches_bodies(world, along) {
        return true;
    }
    beside_on_track(world, along.abs(), BODY_CLOSE_M) && along.abs() >= FOLD_ALONG_M
}

/// Opening lap, same side of the lap: the spline is at least a pack length apart
/// and the bodies are still inside the next-lane cap. That branch does not take
/// the place. `Some(true)` gives it to `b` only when the game already has `b` ahead.
fn opening_pack_fold(s: &Snapshot, a: &Standing, b: &Standing) -> Option<bool> {
    if a.num_laps != 0 || b.num_laps != 0 {
        return None;
    }
    if a.position <= 0 || b.position <= 0 {
        return None;
    }
    let pos_a = rider_lap_pos(s, a.race_num)?;
    let pos_b = rider_lap_pos(s, b.race_num)?;
    let direct = pos_b - pos_a;
    if direct.abs() > 0.5 {
        return None;
    }
    let along = direct.abs() * s.track_length;
    if along < FOLD_ALONG_M {
        return None;
    }
    let world = body_gap(s, a.race_num, b.race_num)?;
    if world > BODY_CLOSE_M {
        return None;
    }
    Some(b.position < a.position)
}

/// `b` is ahead of `a` on score by enough to take the place. The margin is only so two
/// bikes on the same stretch do not swap every frame: `HOLD_M` once `b` already holds it,
/// `PASS_M` to take it. Lap count is always in the score; `use_penalty` subtracts each
/// rider's own time penalty (live order) or ignores it (on-track order).
///
/// With no tracker and no known line, only a same-lap pair inside `PAIR_MAX_M` can move.
/// Further apart than that is not trustworthy. A rider with fewer completed laps does not
/// take the place by a higher fraction on the earlier lap, or while the higher lap is
/// still a short gap ahead on the lap ring. Their coordinate still can once they have
/// crossed and are further into that lap, and the travel since the last counted lap
/// covers the laps they are short. A lead whose bikes sit close together,
/// or whose centerline samples do when a body is missing, is the next lane, not a pass.
/// That rider does not keep the place when the game already had them ahead. A penalty
/// still reorders a pair whose positions are a real distance apart.
fn score_ahead(
    s: &Snapshot,
    prev: &[i32],
    tracked: &[RiderProgress],
    a: &Standing,
    b: &Standing,
    use_penalty: bool,
) -> bool {
    if done_racing(a) && done_racing(b) {
        return false;
    }
    let margin = if prev_rank(prev, b.race_num) < prev_rank(prev, a.race_num) {
        HOLD_M
    } else {
        PASS_M
    };
    if a.num_laps == b.num_laps && !out_of_race(a) && !out_of_race(b) {
        if copied_behind_owner(s, b) {
            return false;
        }
        // Opening lap: a branch a pack-length up the spline, bodies still one
        // group, is not a pass. The game order holds until the branches meet.
        // A gap under 80 m still falls through and takes the place.
        if let Some(give_place_to_b) = opening_pack_fold(s, a, b) {
            return give_place_to_b;
        }
        // The bodies are on this fraction, including past one step and past the
        // stripe. A stored fold does not get a vote, and a coordinate a lap off
        // does not either unless that rider's body rode the lap. A jump the body
        // did not ride is not this gap, unless the bikes already match it. A
        // matching gap is the place past 80 m too, even when that coordinate is
        // still ignored. Inside one step the bodies are the order even when that
        // lap was ridden. A longer raw gap still keeps a lap the bodies do not
        // match.
        // A chord outside 45 m and under half the spline is the lap bending back,
        // once the centerline leader has ridden away from the frozen coordinate.
        // A corner under 140 m is that lead too, even when the chord is past
        // half the spline. A refused coordinate under 155 m keeps the raw lead
        // once the body has walked 45 m, including when the chord folds inside
        // 45 m. Past that cap, a chord of at least 80 m and more than 0.55 of
        // the gap keeps the raw lead once the body has moved. A 45 m chord at
        // 159 m stays the other branch.
        if let Some(along_m) = confirmed_raw_gap(s, a, b) {
            let raw_ahead = if along_m > 0.0 { b.race_num } else { a.race_num };
            let world = body_gap(s, a.race_num, b.race_num);
            let ridden_lap_veto = along_m.abs() > MAX_STEP_M
                && stored_lap_crossed(tracked, a, b, along_m, s.track_length);
            if world.is_some_and(|gap| gap_matches_bodies(gap, along_m))
                && along_m.abs() > margin
                && !ridden_lap_veto
            {
                let ahead_m = if use_penalty {
                    along_m + penalty_m(s, a.penalty_ms - b.penalty_ms)
                } else {
                    along_m
                };
                return ahead_m > margin;
            }
            let raw_behind = if along_m > 0.0 { a } else { b };
            let raw_leader = if along_m > 0.0 { b } else { a };
            if along_m.abs() > margin
                && world.is_some_and(|gap| {
                    bent_chord_the_body_rode(s, tracked, raw_ahead, gap, along_m)
                        || corner_the_body_rode(
                            s,
                            tracked,
                            raw_ahead,
                            raw_behind,
                            raw_leader,
                            gap,
                            along_m,
                        )
                        || spread_chord_the_body_rode(
                            s,
                            tracked,
                            raw_ahead,
                            raw_behind,
                            raw_leader,
                            gap,
                            along_m,
                        )
                })
            {
                let ahead_m = if use_penalty {
                    along_m + penalty_m(s, a.penalty_ms - b.penalty_ms)
                } else {
                    along_m
                };
                return ahead_m > margin;
            }
            let chord_under_45 = tracked
                .iter()
                .find(|progress| progress.race_num == raw_ahead)
                .and_then(|progress| body_step_m(progress, rider_world(s, raw_ahead)))
                .is_some_and(|moved| moved < BODY_CLOSE_M);
            if chord_under_45 {
                if let Some(gap) = world {
                    if let Some(give_place_to_b) = frozen_corner_order(
                        s, tracked, a, b, margin, use_penalty, along_m, gap,
                    ) {
                        return give_place_to_b;
                    }
                }
            }
        } else if let (Some(pos_a), Some(pos_b)) = (
            rider_lap_pos(s, a.race_num),
            rider_lap_pos(s, b.race_num),
        ) {
            let along_m = same_lap_along_m(pos_a, pos_b, s.track_length);
            if let Some(gap) = body_gap(s, a.race_num, b.race_num) {
                if let Some(give_place_to_b) = frozen_corner_order(
                    s, tracked, a, b, margin, use_penalty, along_m, gap,
                ) {
                    return give_place_to_b;
                }
            }
        }
        if let Some(give_place_to_b) = folded_spline_lead(s, prev, tracked, a, b, margin) {
            return give_place_to_b;
        }
        if let Some(give_place_to_b) = origin_crossing_ahead(s, a, b, margin, use_penalty) {
            return give_place_to_b;
        }
    }
    let (score_a, score_b) = if use_penalty {
        (rider_score(s, tracked, a), rider_score(s, tracked, b))
    } else {
        (track_score(s, tracked, a), track_score(s, tracked, b))
    };
    if let (Some(score_a), Some(score_b)) = (score_a, score_b) {
        let delta = score_b - score_a;
        // The bodies match the raw fraction, on any lap. A stored coordinate a sector
        // ahead of that gap does not keep the place.
        if a.num_laps == b.num_laps && !out_of_race(a) && !out_of_race(b) {
            if let Some(along_m) = confirmed_raw_gap(s, a, b) {
                let raw_ahead = if along_m > 0.0 { b } else { a };
                let raw_behind = if along_m > 0.0 { a } else { b };
                let raw_was_ignored = |race_num: i32| {
                    tracked.iter().any(|progress| {
                        progress.race_num == race_num && progress.raw_ignored
                    })
                };
                // A matching gap already returned above, before a stored fold can
                // veto it. A jump the body did not ride can still land on the
                // other side of a fold. The line coming back makes that gap look
                // real, and the jumped rider is the one it puts behind.
                let behind_in_the_fold = raw_was_ignored(raw_behind.race_num)
                    && body_gap(s, a.race_num, b.race_num).is_some_and(|world| {
                        beside_on_track(world, along_m.abs(), BODY_CLOSE_M)
                    });
                // A matching gap already returned above. What is left is a short
                // step, or a jump the body did not ride. `confirmed_raw_gap`
                // still returns that short gap when a jump lands just ahead of
                // a bike a few metres away. The bodies match only inside one
                // step here. An earlier fold that left `raw_ignored` set does
                // not block that lead.
                let bodies_match_fraction = along_m.abs() <= MAX_STEP_M
                    && body_gap(s, a.race_num, b.race_num)
                        .is_some_and(|world| gap_matches_bodies(world, along_m));
                if (!raw_was_ignored(raw_ahead.race_num) || bodies_match_fraction)
                    && !behind_in_the_fold
                    && along_m.abs() > margin
                    && delta.abs() > margin
                    && delta.signum() != along_m.signum()
                {
                    // The stored coordinate is a full lap off the wrapped fraction only
                    // when that rider crossed. A start/finish score can look the same
                    // without the coordinate having moved. On the stripe the bodies are
                    // only a few metres apart, so that lap does not beat the short way.
                    let lap_crossed = stored_lap_crossed(tracked, a, b, along_m, s.track_length);
                    let on_stripe = along_m.abs() <= STRIPE_M
                        && body_gap(s, a.race_num, b.race_num)
                            .is_some_and(|world| world <= STRIPE_M);
                    if !lap_crossed || on_stripe {
                        let ahead_m = if use_penalty {
                            along_m + penalty_m(s, a.penalty_ms - b.penalty_ms)
                        } else {
                            along_m
                        };
                        return ahead_m > margin;
                    }
                }
            }
        }
        // The open-lap arc from S2 back to S1 is cut in half. Two bikes a few
        // metres apart can score a sector apart. Under a pack gap, the raw
        // fraction wins. A stored coordinate the body did not ride is not that gap.
        if a.num_laps == 0 && b.num_laps == 0 && !out_of_race(a) && !out_of_race(b) {
            if let (Some(pos_a), Some(pos_b)) = (
                rider_lap_pos(s, a.race_num),
                rider_lap_pos(s, b.race_num),
            ) {
                let along_m = same_lap_along_m(pos_a, pos_b, s.track_length);
                // Same direction as the score still counts. The stored-lane veto
                // below would cancel a pack gap the body is actually riding.
                if along_m.abs() < FOLD_ALONG_M && along_m.abs() > margin && delta.abs() > margin
                {
                    let ahead_m = if use_penalty {
                        along_m + penalty_m(s, a.penalty_ms - b.penalty_ms)
                    } else {
                        along_m
                    };
                    // The raw gap is under a pack length, so it is not the next lane.
                    return ahead_m > margin;
                }
            }
        }
        // A higher fraction on an earlier lap is not that lap. Reaching it in the
        // coordinate is not enough while the higher lap is still ahead the short way
        // round the ring, or while the travel since the last counted lap does not
        // cover the laps they are short. Half a lap is the flip, and the slack is a
        // quarter of this lap, so a short track and a long one agree.
        if a.num_laps != b.num_laps && !out_of_race(a) && !out_of_race(b) {
            let (fewer, more) = if b.num_laps < a.num_laps {
                (b, a)
            } else {
                (a, b)
            };
            // Bodies on the same stretch, or a close fold, keep the lap count.
            // A coordinate that ran ahead of the classification does not pass.
            if classified_lap_holds(s, fewer, more) {
                return b.num_laps > a.num_laps;
            }
            // One classification lap, bodies and the short way both inside the
            // stripe. The coordinates already have them together, so the place is
            // those metres. A real lap up is about one track length apart and
            // still takes the lap.
            if (a.num_laps - b.num_laps).abs() == 1 {
                if let Some(along_m) = stripe_along(s, a, b) {
                    let score_gap = (score_a - score_b).abs();
                    if score_gap + STRIPE_M < s.track_length {
                        let ahead_m = if use_penalty {
                            along_m + penalty_m(s, a.penalty_ms - b.penalty_ms)
                        } else {
                            along_m
                        };
                        return ahead_m > margin;
                    }
                }
            }
            let (fewer_score, more_laps, fewer, more) = if b.num_laps < a.num_laps {
                (score_b, a.num_laps, b, a)
            } else {
                (score_a, b.num_laps, a, b)
            };
            let reached = fewer_score >= more_laps as f32 * s.track_length
                && rode_the_laps_short(tracked, fewer, more_laps, s.track_length);
            let higher_lap_still_ahead = rider_lap_pos(s, fewer.race_num)
                .zip(rider_lap_pos(s, more.race_num))
                .is_some_and(|(fewer_frac, more_frac)| {
                    wrap_signed(more_frac - fewer_frac) * s.track_length > margin
                });
            if !reached || higher_lap_still_ahead {
                return b.num_laps > a.num_laps;
            }
        }
        // Clear lead on the tracker. Within the pass margin, fall through so a
        // close wrap (cold-arm with equal travel, side-by-side) can still decide.
        if delta > margin {
            return !different_lane(s, tracked, a, b);
        }
        if delta < -margin {
            return false;
        }
    }
    if a.num_laps != b.num_laps || out_of_race(a) || out_of_race(b) {
        return false;
    }
    if let Some(give_place_to_b) = origin_crossing_ahead(s, a, b, margin, use_penalty) {
        return give_place_to_b;
    }
    let (Some(pos_a), Some(pos_b)) = (pack_lap_pos(s, tracked, a), pack_lap_pos(s, tracked, b))
    else {
        return false;
    };
    let along_m = same_lap_along_m(pos_a, pos_b, s.track_length);
    if along_m.abs() > PAIR_MAX_M {
        return false;
    }
    let ahead_m = if use_penalty {
        along_m + penalty_m(s, a.penalty_ms - b.penalty_ms)
    } else {
        along_m
    };
    ahead_m > margin && !different_lane(s, tracked, a, b)
}

/// A crashed bike with no live `track_pos` can still be passed when we held a fraction
/// or an armed lap. DNS / OUT / DSQ stay pinned.
fn rider_can_move(s: &Snapshot, tracked: &[RiderProgress], st: &Standing) -> bool {
    if out_of_race(st) {
        return false;
    }
    rider_lap_pos(s, st.race_num).is_some() || track_score(s, tracked, st).is_some()
}

static LAP0_PENDING_LIVE: Mutex<Vec<i32>> = Mutex::new(Vec::new());
static LAP0_PENDING_TRACK: Mutex<Vec<i32>> = Mutex::new(Vec::new());

fn opening_lap(s: &Snapshot, n: usize) -> bool {
    n > 0 && s.standings[..n].iter().all(|row| row.num_laps <= 0)
}

fn place_shift(prev: &[i32], next: &[i32]) -> i32 {
    let mut worst = 0i32;
    for (index, race_num) in next.iter().enumerate() {
        if let Some(old) = prev.iter().position(|num| num == race_num) {
            worst = worst.max((index as i32 - old as i32).abs());
        }
    }
    worst
}

/// On the opening lap a jump of more than one spot waits until the next sample
/// still wants that order. One projection flip does not move the dash. A pass
/// that is still there on the next tick does.
fn hold_opening_lap(prev: &[i32], candidate: &[i32], pending: &mut Vec<i32>) -> Vec<i32> {
    if prev.is_empty() || place_shift(prev, candidate) <= 1 {
        pending.clear();
        return candidate.to_vec();
    }
    if pending.as_slice() == candidate {
        pending.clear();
        return candidate.to_vec();
    }
    *pending = candidate.to_vec();
    prev.to_vec()
}

fn order_from_nums(s: &Snapshot, n: usize, nums: &[i32], fallback: &[usize]) -> Vec<usize> {
    if nums.len() != n {
        return fallback.to_vec();
    }
    let mut order = Vec::with_capacity(n);
    for race_num in nums {
        match s.standings[..n]
            .iter()
            .position(|row| row.race_num == *race_num)
        {
            Some(index) => order.push(index),
            None => return fallback.to_vec(),
        }
    }
    order
}

fn clear_opening_lap_pending() {
    if let Ok(mut pending) = LAP0_PENDING_LIVE.lock() {
        pending.clear();
    }
    if let Ok(mut pending) = LAP0_PENDING_TRACK.lock() {
        pending.clear();
    }
}

/// Bubble movable slots in `order` by [`score_ahead`]. `use_penalty` selects live vs
/// on-track ranking. Pinned rows (no score, out of race) stay put.
fn bubble_live_slots(
    s: &Snapshot,
    order: &mut [usize],
    prev: &[i32],
    tracked: &[RiderProgress],
    use_penalty: bool,
) {
    let n = order.len();
    let movable: Vec<usize> = (0..n)
        .filter(|&slot| rider_can_move(s, tracked, &s.standings[order[slot]]))
        .collect();
    for _ in 0..n {
        let mut moved = false;
        for pair in movable.windows(2) {
            let (ahead, behind) = (pair[0], pair[1]);
            let (a, b) = (&s.standings[order[ahead]], &s.standings[order[behind]]);
            if score_ahead(s, prev, tracked, a, b, use_penalty) {
                order.swap(ahead, behind);
                moved = true;
            }
        }
        if !moved {
            break;
        }
    }
}

/// Game classification place, with unranked rows after the scored field.
fn game_place_key(st: &Standing, index: usize) -> i32 {
    if st.position > 0 {
        st.position
    } else {
        index as i32 + MAX_STANDINGS as i32
    }
}

/// Practice and warmup: fastest valid lap first. Riders with no time stay in the game's
/// order behind everyone who has one.
fn order_by_best_lap(s: &Snapshot, order: &mut [usize]) {
    order.sort_by(|&left_index, &right_index| {
        let left = &s.standings[left_index];
        let right = &s.standings[right_index];
        let left_timed = left.best_lap_ms > 0;
        let right_timed = right.best_lap_ms > 0;
        match (left_timed, right_timed) {
            (true, true) => left
                .best_lap_ms
                .cmp(&right.best_lap_ms)
                .then_with(|| {
                    game_place_key(left, left_index).cmp(&game_place_key(right, right_index))
                })
                .then(left_index.cmp(&right_index)),
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            (false, false) => game_place_key(left, left_index)
                .cmp(&game_place_key(right, right_index))
                .then(left_index.cmp(&right_index)),
        }
    });
}

/// Indices into `s.standings[..n]`, P1 first. In a race, running riders are ordered by
/// [`rider_score`] (track progress minus their own penalty), bubbled so one tick can move
/// a rider through the whole pack. Riders we cannot place stay in their scored slot.
/// Also publishes [`TRACK_ORDER`] (same bubble without penalties) while live order is on.
/// Practice and warmup sort by best lap instead, and do not bubble.
fn live_order(s: &Snapshot, clock: &SessionClock, n: usize) -> Vec<usize> {
    note_progress(s, clock, n);
    let (in_slots, awaiting_line) = PROGRESS
        .lock()
        .map(|tracked| {
            (
                field_still_in_slots(s, &tracked),
                awaiting_holeshot(s, &tracked),
            )
        })
        .unwrap_or((false, false));
    let holding = in_slots || awaiting_line || start_gate_up(s);
    HOLDING_GATE.store(holding, Ordering::Relaxed);
    let mut order: Vec<usize> = (0..n).collect();
    if matches!(clock.mode, ClockMode::Practice) {
        order_by_best_lap(s, &mut order);
    } else {
        order.sort_by_key(|&i| game_place_key(&s.standings[i], i));
    }
    if n > 1 && live_order_active(s, clock) && !holding {
        let prev_live = LIVE_ORDER.lock().map(|g| g.clone()).unwrap_or_default();
        let prev_track = TRACK_ORDER.lock().map(|g| g.clone()).unwrap_or_default();
        let tracked = PROGRESS.lock().map(|g| g.clone()).unwrap_or_default();
        let mut track = order.clone();
        bubble_live_slots(s, &mut order, &prev_live, &tracked, true);
        bubble_live_slots(s, &mut track, &prev_track, &tracked, false);
        if opening_lap(s, n) {
            let live_nums: Vec<i32> = order.iter().map(|&i| s.standings[i].race_num).collect();
            let track_nums: Vec<i32> = track.iter().map(|&i| s.standings[i].race_num).collect();
            let held_live = match LAP0_PENDING_LIVE.lock() {
                Ok(mut pending) => hold_opening_lap(&prev_live, &live_nums, &mut pending),
                Err(_) => live_nums,
            };
            let held_track = match LAP0_PENDING_TRACK.lock() {
                Ok(mut pending) => hold_opening_lap(&prev_track, &track_nums, &mut pending),
                Err(_) => track_nums,
            };
            order = order_from_nums(s, n, &held_live, &order);
            track = order_from_nums(s, n, &held_track, &track);
        } else {
            clear_opening_lap_pending();
        }
        if let Ok(mut g) = TRACK_ORDER.lock() {
            g.clear();
            g.extend(track.iter().map(|&i| s.standings[i].race_num));
        }
    } else if let Ok(mut g) = TRACK_ORDER.lock() {
        g.clear();
    }
    if clock.in_gate || holding {
        // The gate order is not a race place. Publishing it made the next tick
        // "correct" qualifying positions instead of starting from location.
        // The same hold covers the slots, the game's gate state, and the run to the holeshot line.
        clear_opening_lap_pending();
        if let Ok(mut g) = LIVE_ORDER.lock() {
            g.clear();
        }
    } else if let Ok(mut g) = LIVE_ORDER.lock() {
        g.clear();
        g.extend(order.iter().map(|&i| s.standings[i].race_num));
    }
    order
}

fn place_in_order(order: &[i32], race_num: i32) -> i32 {
    if race_num <= 0 {
        return 0;
    }
    order
        .iter()
        .position(|&n| n == race_num)
        .map(|i| i as i32 + 1)
        .unwrap_or(0)
}

/// Live place for a race number, `0` when we have no order yet.
pub fn live_position(race_num: i32) -> i32 {
    LIVE_ORDER
        .lock()
        .ok()
        .map(|g| place_in_order(&g, race_num))
        .unwrap_or(0)
}

/// Published live order, P1 first, as race numbers. Empty on the gate and until the holeshot.
pub fn live_order_nums() -> Vec<i32> {
    LIVE_ORDER.lock().map(|g| g.clone()).unwrap_or_default()
}

/// On-track place (penalties ignored), `0` when live order is off or unknown.
pub fn track_position(race_num: i32) -> i32 {
    TRACK_ORDER
        .lock()
        .ok()
        .map(|g| place_in_order(&g, race_num))
        .unwrap_or(0)
}

/// How live place compares to on-track place: `Some(track − live)`.
/// Positive = ahead of on-track (green `*`), negative = behind (red `*`).
/// `None` when places match, either place is unknown, or live order is off.
pub fn penalty_place_delta(race_num: i32) -> Option<i32> {
    let live = live_position(race_num);
    let track = track_position(race_num);
    if live <= 0 || track <= 0 || live == track {
        return None;
    }
    Some(track - live)
}

fn class_rank_in_order(s: &Snapshot, order: &[i32], race_num: i32, category: &str) -> i32 {
    let mut rank = 0i32;
    for &num in order {
        let Some(row) = standing_of(s, num) else {
            continue;
        };
        if bytes_as_text(&row.category) != category {
            continue;
        }
        rank += 1;
        if num == race_num {
            return rank;
        }
    }
    0
}

/// Same as [`penalty_place_delta`] but within the rider's class.
pub fn penalty_class_place_delta(s: &Snapshot, race_num: i32) -> Option<i32> {
    let Some(st) = standing_of(s, race_num) else {
        return None;
    };
    let cat = bytes_as_text(&st.category);
    if cat.is_empty() {
        return penalty_place_delta(race_num);
    }
    let live_order = LIVE_ORDER.lock().ok()?;
    let track_order = TRACK_ORDER.lock().ok()?;
    if live_order.is_empty() || track_order.is_empty() {
        return None;
    }
    let live = class_rank_in_order(s, &live_order, race_num, &cat);
    let track = class_rank_in_order(s, &track_order, race_num, &cat);
    if live <= 0 || track <= 0 || live == track {
        return None;
    }
    Some(track - live)
}

/// Race number leading right now, `0` when we have no order yet.
pub fn live_leader() -> i32 {
    LIVE_ORDER
        .lock()
        .ok()
        .and_then(|g| g.first().copied())
        .unwrap_or(0)
}

/// Classification rows in live order. Falls back to the game array before the first tick.
pub(crate) fn ordered_standings(s: &Snapshot) -> Vec<Standing> {
    let n = (s.standing_count.max(0) as usize).min(MAX_STANDINGS);
    let order = LIVE_ORDER.lock().map(|g| g.clone()).unwrap_or_default();
    let mut out: Vec<Standing> = Vec::with_capacity(n);
    for num in &order {
        if let Some(st) = s.standings[..n].iter().find(|r| r.race_num == *num) {
            out.push(*st);
        }
    }
    if out.len() != n {
        out.clear();
        out.extend_from_slice(&s.standings[..n]);
    }
    out
}

/// Read-only: the clock owns every session latch, so nothing here may call a helper that
/// notes or arms state (`is_warmup`, `leader_finished`, `effective_race_laps`, ...).
fn build_field(s: &Snapshot, clock: &SessionClock) -> RaceField {
    let n = s.standing_count.max(0) as usize;
    let n = n.min(MAX_STANDINGS);
    let focus_num = if s.focus_race_num > 0 {
        s.focus_race_num
    } else {
        s.local_race_num
    };
    let order = live_order(s, clock, n);
    let mut rows = Vec::with_capacity(n);
    let mut focus = None;
    let mut leader = None;
    let focus_st = s.standings[..n]
        .iter()
        .find(|r| r.race_num == focus_num)
        .copied();
    let practice = matches!(clock.mode, ClockMode::Practice);
    let leader_best_ms = order.iter().find_map(|&si| {
        let ms = s.standings[si].best_lap_ms;
        (ms > 0).then_some(ms)
    });

    for (i, &si) in order.iter().enumerate() {
        let mut st = s.standings[si];
        // Live place, so a pass shows on every board without waiting for the game to
        // republish its classification at the line. On the gate there is no place yet.
        st.position = if clock.in_gate || holding_gate() { 0 } else { i as i32 + 1 };
        let is_focus = st.race_num == focus_num;
        let is_leader = i == 0;
        if is_focus {
            focus = Some(i);
        }
        if is_leader {
            leader = Some(i);
        }
        // Race gaps stay the game's, so a fresh pass can leave the pair's gap to the
        // leader the wrong way round for a moment: interval is a size, never negative.
        // Practice and warmup gaps are best-lap deltas, matching the lap-time order.
        let (interval_ms, interval_laps, gap_to_focus_ms, gap_to_focus_laps) = if practice {
            if let Some(leader_best) = leader_best_ms.filter(|_| st.best_lap_ms > 0) {
                st.gap_ms = st.best_lap_ms - leader_best;
                st.gap_laps = 0;
            } else {
                st.gap_ms = 0;
                st.gap_laps = 0;
            }
            let interval = if i == 0 {
                (0, 0)
            } else {
                let ahead_ms = s.standings[order[i - 1]].best_lap_ms;
                if st.best_lap_ms > 0 && ahead_ms > 0 {
                    ((st.best_lap_ms - ahead_ms).abs(), 0)
                } else {
                    (0, 0)
                }
            };
            let to_focus = match focus_st {
                Some(f) if st.best_lap_ms > 0 && f.best_lap_ms > 0 => {
                    (st.best_lap_ms - f.best_lap_ms, 0)
                }
                _ => (0, 0),
            };
            (interval.0, interval.1, to_focus.0, to_focus.1)
        } else {
            let interval = if i == 0 {
                (0, 0)
            } else {
                let ahead = &s.standings[order[i - 1]];
                let lap_delta = st.gap_laps - ahead.gap_laps;
                if lap_delta != 0 {
                    (0, lap_delta.abs())
                } else {
                    ((st.gap_ms - ahead.gap_ms).abs(), 0)
                }
            };
            let to_focus = match focus_st {
                Some(f) => (st.gap_ms - f.gap_ms, st.gap_laps - f.gap_laps),
                None => (0, 0),
            };
            (interval.0, interval.1, to_focus.0, to_focus.1)
        };
        rows.push(RaceRow {
            standing: st,
            current_lap: rider_current_lap(s, st.race_num, st.num_laps),
            is_focus,
            is_leader,
            interval_ms,
            interval_laps,
            gap_to_focus_ms,
            gap_to_focus_laps,
        });
    }

    RaceField {
        session_best_ms: session_best_ms(s),
        rows,
        focus,
        leader,
    }
}

fn build_clock_with(s: &Snapshot, write_flag: bool) -> SessionClock {
    // One mutation per tick: banner formatting must not call session_remain_ms again.
    let remain_ms = session_remain_ms(s);
    note_leader_finish(s);
    let banner = format_session_banner(s, remain_ms);
    let extras_total = extra_laps(s);
    let extras_taken = if overtime_active(s) {
        local_overtime_taken(s)
    } else {
        0
    };
    let flag = timed_race_flag_with(s, write_flag);
    SessionClock {
        mode: ClockMode::from_state(s, remain_ms, &banner.1),
        remain_ms,
        banner,
        in_gate: IN_GATE.load(Ordering::Relaxed) == 1,
        expired: SESSION_EXPIRED.load(Ordering::Relaxed) == 1,
        extras_taken,
        extras_total,
        flag,
    }
}

/// White / checkered for timed extras from the race timer + your extras lap count.
/// Lap motos and the early white on the run-in need track geometry, so the dash
/// refines this in render; this is the count-only value for tracing and fallback.
pub(crate) fn timed_race_flag(s: &Snapshot) -> RaceFlag {
    timed_race_flag_with(s, true)
}

/// `write` is the overlay (or a stream-only) step. A second paint of the same sample
/// must not clear the checkered latch or the laps-to-run note.
fn timed_race_flag_with(s: &Snapshot, write: bool) -> RaceFlag {
    note_session(s);
    if s.on_track == 0 {
        if write {
            CHECKERED_LATCH.store(0, Ordering::Relaxed);
        }
        return RaceFlag::None;
    }
    if is_lap_race(s) || timed_clock_live(s) || prestart(s) || !overtime_active(s) {
        return RaceFlag::None;
    }
    // Same recovery as `dash_race_flag`: do not keep a latched finish while laps remain.
    if CHECKERED_LATCH.load(Ordering::Relaxed) == 1 {
        if extras_started(s) && laps_left(s).is_some_and(|n| n > 0) {
            if write {
                CHECKERED_LATCH.store(0, Ordering::Relaxed);
            }
        } else {
            return RaceFlag::Checkered;
        }
    }
    if !extras_started(s) {
        return RaceFlag::None;
    }
    let left = laps_left(s).unwrap_or(1);
    if write {
        note_laps_to_run(s, Some(left));
    }
    if left == 0 && finish_earned(s) {
        if write {
            CHECKERED_LATCH.store(1, Ordering::Relaxed);
        }
        RaceFlag::Checkered
    } else if skip_last_lap_white(s, Some(left)) {
        RaceFlag::None
    } else if left <= 1 {
        // Your last remaining extra of the distance you are still running.
        RaceFlag::White
    } else {
        RaceFlag::None
    }
}

impl RaceStore {
    /// Fill `VIEW` without cloning. Overlay `draw` and [`tick`] use this, then [`with`].
    /// Steps the race flag once. Stream paint uses [`refresh_without_flags`].
    pub fn refresh(s: &Snapshot) {
        fill_view(s, true);
    }

    /// Clock and field for a second paint of a sample the overlay already flagged.
    /// Does not step white, checkered, the run-in, or the checkered latch.
    pub fn refresh_without_flags(s: &Snapshot) {
        fill_view(s, false);
    }

    /// Once per frame for tests and clock logs. Overlay draw prefers [`refresh`] + [`with`].
    pub fn tick(s: &Snapshot) -> RaceStore {
        Self::refresh(s);
        Self::get()
    }

    /// Borrow the last refresh. Re-entrant: nested `with` / [`get`] reuse the same borrow.
    /// Do not lock `VIEW` in the callback (`reset_session_clock_track` skips it).
    pub fn with<R>(f: impl FnOnce(&RaceStore) -> R) -> R {
        if let Some(p) = WITH.with(|c| c.get()) {
            return f(unsafe { p.as_ref() });
        }
        let g = VIEW.lock().unwrap_or_else(|e| e.into_inner());
        WITH.with(|c| c.set(Some(NonNull::from(&*g))));
        struct Clear;
        impl Drop for Clear {
            fn drop(&mut self) {
                WITH.with(|c| c.set(None));
            }
        }
        let _c = Clear;
        f(&g)
    }

    /// Clone of the last refresh. Prefer [`with`] on the draw path.
    pub fn get() -> RaceStore {
        Self::with(|s| s.clone())
    }
}

fn fill_view(s: &Snapshot, advance_flags: bool) {
    // Clock first, and only it may mutate session state: the field reads the result.
    let clock = build_clock_with(s, advance_flags);
    note_field_laps(s);
    let field = build_field(s, &clock);
    if s.has_telemetry != 0 {
        let (thr, brk, _) = crate::telemetry::inputs(s);
        crate::telemetry::note(thr, brk, crate::telemetry::steer(s));
    }
    if let Ok(mut g) = VIEW.lock() {
        g.clock = clock;
        g.field = field;
    }
    if !advance_flags {
        return;
    }
    // Flag, start/finish, and run-in advance once with the session, after the clock
    // is visible to `dash_race_flag`.
    let race_flag = crate::render::session_race_flag(s);
    if let Ok(mut g) = VIEW.lock() {
        g.race_flag = race_flag;
    }
}

pub(crate) fn format_gap(ms: i32, laps: i32) -> String {
    if laps != 0 {
        return format!("+{laps}L");
    }
    if ms <= 0 {
        return "---".into();
    }
    let sec = ms as f32 / 1000.0;
    if sec >= 60.0 {
        let m = (sec / 60.0) as i32;
        format!("+{m}:{:04.1}", sec - m as f32 * 60.0)
    } else {
        format!("+{sec:.3}")
    }
}

pub(crate) fn format_lap(ms: i32) -> String {
    if ms <= 0 {
        return "--".into();
    }
    let sec = ms as f32 / 1000.0;
    if sec >= 60.0 {
        let m = (sec / 60.0) as i32;
        format!("{m}:{:06.3}", sec - m as f32 * 60.0)
    } else {
        format!("{sec:.3}")
    }
}

pub(crate) fn standing_of(s: &Snapshot, race_num: i32) -> Option<&Standing> {
    s.standings
        .iter()
        .take(s.standing_count.max(0) as usize)
        .find(|st| st.race_num == race_num)
}

pub(crate) fn interval_text(s: &Snapshot, row: &Standing) -> String {
    if row.position <= 1 {
        return "---".into();
    }
    let n = s.standing_count.max(0) as usize;
    let Some(ahead) = s.standings[..n]
        .iter()
        .find(|st| st.position == row.position - 1)
    else {
        return "---".into();
    };
    let lap_delta = row.gap_laps - ahead.gap_laps;
    if lap_delta != 0 {
        format_gap(0, lap_delta)
    } else {
        format_gap(row.gap_ms - ahead.gap_ms, 0)
    }
}

pub(crate) fn format_session_clock(ms: i32) -> String {
    if ms <= 0 {
        return "--:--:--".into();
    }
    let t = ms / 1000;
    format!("{:02}:{:02}:{:02}", t / 3600, (t / 60) % 60, t % 60)
}

pub(crate) fn session_len_ms(len: i32) -> i32 {
    // Plugin uses -1 until this session writes a length; same as 0 (no clock).
    // 1–59 is minutes, 60–999 is seconds (a 00:60 gate, not a 60:00 race),
    // 1000+ is already milliseconds (a 60:00 race is 3_600_000).
    if len <= 0 {
        0
    } else if len >= 1_000 {
        len
    } else if len >= 60 {
        len * 1000
    } else {
        len * 60_000
    }
}

pub(crate) fn session_len_minutes(ms: i32) -> i32 {
    if ms < 60_000 {
        0
    } else {
        (ms + 30_000) / 60_000
    }
}

/// Timed extras the plugin publishes as `session_laps`. Five or more is a lap moto.
pub(crate) const EXTRA_LAPS_MAX: i32 = 4;

pub(crate) fn extra_laps_field(n: i32) -> bool {
    (1..=EXTRA_LAPS_MAX).contains(&n)
}

pub(crate) fn leftover_practice_len(ms: i32) -> bool {
    ms >= 30 * 60_000
}

pub(crate) fn leftover_warmup_len(ms: i32) -> bool {
    leftover_practice_len(ms) || standard_race_minutes(ms)
}

pub(crate) fn race_clock_ms(clock: i32) -> bool {
    clock >= 5 * 60_000 && clock <= 60 * 60_000
}

pub(crate) fn wait_display_ms(total: i32, clock: i32) -> i32 {
    if total > 0 {
        total
    } else if race_clock_ms(clock) {
        clock
    } else {
        0
    }
}

pub(crate) fn standard_race_minutes(ms: i32) -> bool {
    matches!(
        session_len_minutes(ms),
        5 | 6 | 8 | 10 | 12 | 15 | 20 | 25 | 30 | 60
    )
}

pub(crate) fn effective_session_len_ms(s: &Snapshot) -> i32 {
    let total = session_len_ms(s.session_length);
    if is_lap_race(s) && leftover_warmup_len(total) {
        return 0;
    }
    // Leftover practice (not a 5–60 min race length) must not cap a moto.
    if s.session_laps > 0 && leftover_practice_len(total) && !standard_race_minutes(total) {
        return 0;
    }
    // Leftover start board (~50s) must not cap a live 5–60 min timed +N countdown.
    note_timed_extras_hint(s);
    if extra_laps_field(s.session_laps)
        && total > 0
        && total < 3 * 60_000
        && (TIMED_EXTRAS_HINT.load(Ordering::Relaxed) == 1
            || race_clock_ms(s.session_time_ms.max(0))
            || race_clock_ms(LAST_SESSION_CLOCK.load(Ordering::Relaxed)))
    {
        return 0;
    }
    total
}

pub(crate) fn format_countdown(remain_ms: i32) -> String {
    let t = remain_ms.max(0) / 1000;
    if t >= 3600 {
        format!("{:02}:{:02}:{:02}", t / 3600, (t / 60) % 60, t % 60)
    } else {
        format!("{:02}:{:02}", t / 60, t % 60)
    }
}

pub(crate) fn race_lap(s: &Snapshot) -> i32 {
    if is_lap_race(s) {
        let n = s.session_laps.max(1);
        let done = focus_num_laps(s).max(0);
        return (done + 1).clamp(1, n);
    }
    if s.current_lap > 0 {
        return s.current_lap;
    }
    s.standings
        .iter()
        .take(s.standing_count.max(0) as usize)
        .map(|row| row.num_laps)
        .max()
        .unwrap_or(0)
        .max(0)
}

pub(crate) static LAST_SESSION_CLOCK: AtomicI32 = AtomicI32::new(0);
pub(crate) static SESSION_CLOCK_MODE: AtomicI32 = AtomicI32::new(0);
pub(crate) static SAW_SESSION_TIME: AtomicI32 = AtomicI32::new(0);
pub(crate) static SESSION_EXPIRED: AtomicI32 = AtomicI32::new(0);
pub(crate) static OVERTIME_BASE_LAP: AtomicI32 = AtomicI32::new(-1);
pub(crate) static OVERTIME_LOCAL_BASE: AtomicI32 = AtomicI32::new(-1);
/// `1` when the frozen base should swallow the leader's next single lap. They were
/// already within [`EXPIRY_LINE_M`] of the line when the clock died, so that crossing
/// finishes the lap in progress and does not start extras.
pub(crate) static OVERTIME_LINE_ARM: AtomicI32 = AtomicI32::new(0);
pub(crate) static CACHED_SESSION_LAPS: AtomicI32 = AtomicI32::new(0);
pub(crate) static CHECKERED_LATCH: AtomicI32 = AtomicI32::new(0);
/// The lap the white flag was waved on, and when it went up, in `anim_now` ms. The wave
/// is a moment at the line, not a state, so the dash drops it a few seconds into the lap.
/// `-1` when no wave is running.
pub(crate) static WHITE_WAVE_LAP: AtomicI32 = AtomicI32::new(-1);
pub(crate) static WHITE_WAVE_AT: AtomicI32 = AtomicI32::new(0);
/// The flag that was out on the run-in, kept out across the line. The game only rescores
/// you a frame or two after you are past it, so `laps_left` lags the crossing and the
/// banner would blink off and back on. 0 none, 1 white, 2 checkered.
pub(crate) static RUN_IN_FLAG: AtomicI32 = AtomicI32::new(0);
/// Your completed laps when the leader took the finish. The race is over for you on
/// your next crossing, even a lap down. `-1` until the leader finishes.
pub(crate) static LEADER_FIN_LOCAL_BASE: AtomicI32 = AtomicI32::new(-1);
/// Timed +1, still on the uncounted lap, and a lap down when the leader finished.
/// That line is the checkered. Stays set so the crossing still reads `laps_left == 0`
/// after their lead shrinks by one. `0` until then.
static PLUS_ONE_WAVE_OFF: AtomicI32 = AtomicI32::new(0);
/// Timed +2, lapped before `2/2` starts. The line that would start that extra is the
/// checkered. `0` until a real pass on `0/2` or `1/2`.
static PLUS_TWO_WAVE_OFF: AtomicI32 = AtomicI32::new(0);
/// Start/finish position in ten-thousandths of a lap, learned from your own crossings.
/// `-1` until confirmed; beats `sf_meters`, which is 0 without a centerline.
pub(crate) static START_FINISH_FRACTION_LEARNED: AtomicI32 = AtomicI32::new(-1);
/// First unconfirmed sighting of the line. Two crossings must agree before we trust it,
/// so a rejoin or a stray lap increment cannot move the flag window.
pub(crate) static START_FINISH_FRACTION_CANDIDATE: AtomicI32 = AtomicI32::new(-1);
/// Your completed laps last frame — detects a crossing for the S/F calibration.
pub(crate) static START_FINISH_LEARN_LAPS: AtomicI32 = AtomicI32::new(-1);
/// Prior distance to S/F in metres, so the run-in must actually be closing.
pub(crate) static LAST_SF_METERS: AtomicI32 = AtomicI32::new(-1);
/// Set once you are ~half a lap from S/F, cleared on each crossing. Stops track-position
/// jitter or a reset-to-track near the line from re-arming the run-in.
pub(crate) static LAP_MID_SEEN: AtomicI32 = AtomicI32::new(0);
/// Whether the last frame moved you closer to S/F.
pub(crate) static CLOSING_ON_LINE: AtomicI32 = AtomicI32::new(0);
/// Your lap count while laps still remained, so the checkered has to be earned by a
/// crossing rather than by one glitched frame. `-1` until the race is under way.
pub(crate) static LAPS_TO_RUN_AT: AtomicI32 = AtomicI32::new(-1);
/// Last stable `laps_left` of 2 or more. A drop of two or more in one step is a
/// glitched `session_laps`, not a last lap — that used to wave white for a second.
static LAST_LAPS_REMAINING: AtomicI32 = AtomicI32::new(-1);
pub(crate) static LAST_SESSION_SIG: AtomicI32 = AtomicI32::new(0);
pub(crate) static LAST_CUR_LAP: AtomicI32 = AtomicI32::new(0);
pub(crate) static IN_GATE: AtomicI32 = AtomicI32::new(0);
pub(crate) static POST_GATE: AtomicI32 = AtomicI32::new(0);
pub(crate) static LAP_GREEN: AtomicI32 = AtomicI32::new(0);
pub(crate) static LOCKED_SESSION_LEN: AtomicI32 = AtomicI32::new(0);
pub(crate) static RACE_ARMED: AtomicI32 = AtomicI32::new(0);
pub(crate) static LAST_SESSION_LAPS: AtomicI32 = AtomicI32::new(-1);
pub(crate) static LAST_RAW_SESSION_LEN: AtomicI32 = AtomicI32::new(0);
/// Last `session_kind` (warmup 5, race 1/2 = 6/7). `0` until we have seen one.
static LAST_SESSION_KIND: AtomicI32 = AtomicI32::new(0);
/// Sticky: 1–4 "laps" field is timed extras once we see a 5–60 min race clock / length.
/// Stops 6:00+2 with unset/start-board length flipping to a 2-lap moto after the gate.
pub(crate) static TIMED_EXTRAS_HINT: AtomicI32 = AtomicI32::new(0);
/// A 2–4-lap moto already ran its gate board. Sticky so a later 5–60 min
/// `session_time_ms` dump cannot flip the dash to a timed clock.
static LAP_MOTO_GATE: AtomicI32 = AtomicI32::new(0);
/// The lap-up leader has gone past you this session. Classification `gap_laps` only
/// moves at the line, so this is what makes `~Lapped` and `0/1` → `1/1` instant.
static LAPPED_LATCH: AtomicI32 = AtomicI32::new(0);
/// Last sample: the lap-up leader was behind you, so the next time they are ahead is the pass.
static LEADER_BEHIND: AtomicI32 = AtomicI32::new(0);
/// Leader lap fraction last sample, ten-thousandths. The pass has to be them moving, not you.
static LAST_LEAD_FRAC: AtomicI32 = AtomicI32::new(-1);
/// The clock we were counting down from when it dropped a long way in one frame, so the
/// climb back out of a republished start board is not read as the race clock expiring.
/// `-1` when the clock is running normally.
static DIP_FROM_CLOCK: AtomicI32 = AtomicI32::new(-1);
/// A second long drop inside one dip. The dash stays on the pre-dip clock until
/// the raw time comes back, instead of walking down the stair.
static CLOCK_STAIR: AtomicBool = AtomicBool::new(false);
/// Remain captured when the game republished the 8–15 s start board. 0 when the clock is live.
static BOARD_HOLD_REMAIN: AtomicI32 = AtomicI32::new(0);
/// `anim_now` in milliseconds when that remain was captured.
static BOARD_HOLD_AT_MS: AtomicI32 = AtomicI32::new(0);

#[cfg(test)]
pub(crate) fn session_test_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Last completed lap and the one before it, per race number.
/// `prior_ms` stays 0 until a second lap is real.
struct LapPace {
    seen_ms: i32,
    prior_ms: i32,
    laps: i32,
    /// Completed-lap count seen once with the same time. A second sample at
    /// that count is an equal lap. One stale frame before the new time arrives
    /// does not count.
    pending_laps: i32,
}

static LAP_PACE: Mutex<Vec<(i32, LapPace)>> = Mutex::new(Vec::new());

fn focus_id(s: &Snapshot) -> i32 {
    if s.focus_race_num > 0 {
        s.focus_race_num
    } else {
        s.local_race_num
    }
}

/// Same last-lap the Last column shows: the row, or your `last_lap_ms` when
/// classification has not filled the row yet.
fn resolved_last_lap_ms(s: &Snapshot, standing: &Standing) -> i32 {
    if standing.last_lap_ms > 0 {
        standing.last_lap_ms
    } else if standing.race_num == focus_id(s) {
        s.last_lap_ms
    } else {
        0
    }
}

fn note_lap_pace(row: &mut LapPace, last_ms: i32, completed_laps: i32) {
    if last_ms != row.seen_ms {
        row.prior_ms = row.seen_ms;
        row.seen_ms = last_ms;
        row.laps = completed_laps.max(row.laps);
        row.pending_laps = 0;
        return;
    }
    if completed_laps > row.laps {
        if row.pending_laps == completed_laps {
            row.prior_ms = row.seen_ms;
            row.laps = completed_laps;
            row.pending_laps = 0;
        } else {
            row.pending_laps = completed_laps;
        }
    }
}

fn note_field_laps(s: &Snapshot) {
    let n = (s.standing_count.max(0) as usize).min(MAX_STANDINGS);
    let focus = focus_id(s);
    let mut saw_focus = false;
    let Ok(mut pace) = LAP_PACE.lock() else {
        return;
    };
    for standing in &s.standings[..n] {
        if standing.race_num <= 0 {
            continue;
        }
        if standing.race_num == focus {
            saw_focus = true;
        }
        let last_ms = resolved_last_lap_ms(s, standing);
        if last_ms <= 0 {
            continue;
        }
        let completed = standing_num_laps(s, standing.race_num, standing.num_laps);
        remember_lap(&mut pace, standing.race_num, last_ms, completed);
    }
    if focus > 0 && !saw_focus && s.last_lap_ms > 0 {
        let completed = standing_num_laps(s, focus, 0);
        remember_lap(&mut pace, focus, s.last_lap_ms, completed);
    }
}

fn remember_lap(pace: &mut Vec<(i32, LapPace)>, race_num: i32, last_ms: i32, completed: i32) {
    if let Some((_, row)) = pace.iter_mut().find(|(num, _)| *num == race_num) {
        note_lap_pace(row, last_ms, completed);
        return;
    }
    pace.push((
        race_num,
        LapPace {
            seen_ms: last_ms,
            prior_ms: 0,
            laps: completed.max(0),
            pending_laps: 0,
        },
    ));
}

/// Last completed lap minus the one before it. `None` until that rider has two
/// laps this session. Negative is faster.
pub(crate) fn lap_diff_ms(race_num: i32) -> Option<i32> {
    let pace = LAP_PACE.lock().unwrap_or_else(|e| e.into_inner());
    let row = pace
        .iter()
        .find(|(num, _)| *num == race_num)
        .map(|(_, row)| row)?;
    if row.prior_ms <= 0 || row.seen_ms <= 0 {
        return None;
    }
    Some(row.seen_ms - row.prior_ms)
}

pub(crate) fn reset_session_clock_track() {
    LAST_SESSION_CLOCK.store(0, Ordering::Relaxed);
    DIP_FROM_CLOCK.store(-1, Ordering::Relaxed);
    CLOCK_STAIR.store(false, Ordering::Relaxed);
    BOARD_HOLD_REMAIN.store(0, Ordering::Relaxed);
    BOARD_HOLD_AT_MS.store(0, Ordering::Relaxed);
    SESSION_CLOCK_MODE.store(0, Ordering::Relaxed);
    SAW_SESSION_TIME.store(0, Ordering::Relaxed);
    SESSION_EXPIRED.store(0, Ordering::Relaxed);
    OVERTIME_BASE_LAP.store(-1, Ordering::Relaxed);
    OVERTIME_LOCAL_BASE.store(-1, Ordering::Relaxed);
    OVERTIME_LINE_ARM.store(0, Ordering::Relaxed);
    CACHED_SESSION_LAPS.store(0, Ordering::Relaxed);
    CHECKERED_LATCH.store(0, Ordering::Relaxed);
    WHITE_WAVE_LAP.store(-1, Ordering::Relaxed);
    RUN_IN_FLAG.store(0, Ordering::Relaxed);
    LEADER_FIN_LOCAL_BASE.store(-1, Ordering::Relaxed);
    START_FINISH_FRACTION_LEARNED.store(-1, Ordering::Relaxed);
    START_FINISH_FRACTION_CANDIDATE.store(-1, Ordering::Relaxed);
    START_FINISH_LEARN_LAPS.store(-1, Ordering::Relaxed);
    LAST_SF_METERS.store(-1, Ordering::Relaxed);
    LAP_MID_SEEN.store(0, Ordering::Relaxed);
    CLOSING_ON_LINE.store(0, Ordering::Relaxed);
    LAPS_TO_RUN_AT.store(-1, Ordering::Relaxed);
    LAST_LAPS_REMAINING.store(-1, Ordering::Relaxed);
    IN_GATE.store(0, Ordering::Relaxed);
    HOLDING_GATE.store(false, Ordering::Relaxed);
    POST_GATE.store(0, Ordering::Relaxed);
    LAP_GREEN.store(0, Ordering::Relaxed);
    LOCKED_SESSION_LEN.store(0, Ordering::Relaxed);
    RACE_ARMED.store(0, Ordering::Relaxed);
    LAST_SESSION_LAPS.store(-1, Ordering::Relaxed);
    LAST_RAW_SESSION_LEN.store(0, Ordering::Relaxed);
    LAST_SESSION_KIND.store(0, Ordering::Relaxed);
    TIMED_EXTRAS_HINT.store(0, Ordering::Relaxed);
    LAP_MOTO_GATE.store(0, Ordering::Relaxed);
    LAPPED_LATCH.store(0, Ordering::Relaxed);
    PLUS_ONE_WAVE_OFF.store(0, Ordering::Relaxed);
    PLUS_TWO_WAVE_OFF.store(0, Ordering::Relaxed);
    LEADER_BEHIND.store(0, Ordering::Relaxed);
    LAST_LEAD_FRAC.store(-1, Ordering::Relaxed);
    if let Ok(mut g) = LIVE_ORDER.lock() {
        g.clear();
    }
    if let Ok(mut g) = TRACK_ORDER.lock() {
        g.clear();
    }
    clear_opening_lap_pending();
    if let Ok(mut g) = PROGRESS.lock() {
        g.clear();
    }
    if let Ok(mut g) = LAP_PACE.lock() {
        g.clear();
    }
    // `with()` holds VIEW. Clearing it here would deadlock, and mutating through
    // the published `&RaceStore` would be unsound. Next `refresh` overwrites.
    if WITH.with(|c| c.get()).is_none() {
        if let Ok(mut g) = VIEW.lock() {
            *g = RaceStore::default();
        }
    }
}

/// Keep the captured remain moving while the game holds the 8–15 s board.
/// Elapsed time is wall time from the first junk sample, so a one-frame board
/// still shows the last second and a longer stop does not freeze then skip.
fn held_board_remain(anchor: i32) -> i32 {
    let now_ms = (anim_now() * 1000.0) as i32;
    let captured = BOARD_HOLD_REMAIN.load(Ordering::Relaxed);
    let (remain, at_ms) = if captured > 0 {
        (captured, BOARD_HOLD_AT_MS.load(Ordering::Relaxed))
    } else {
        BOARD_HOLD_REMAIN.store(anchor, Ordering::Relaxed);
        BOARD_HOLD_AT_MS.store(now_ms, Ordering::Relaxed);
        (anchor, now_ms)
    };
    (remain - now_ms.saturating_sub(at_ms)).max(0)
}

fn note_timed_extras_hint(s: &Snapshot) {
    let laps = s.session_laps;
    if !extra_laps_field(laps) {
        return;
    }
    let total = session_len_ms(s.session_length);
    let clock = s.session_time_ms.max(0);
    let last = LAST_SESSION_CLOCK.load(Ordering::Relaxed);
    if standard_race_minutes(total) {
        // +1..+3 with a published 5–60 min length is extras. +4 looks like a 4-lap
        // leftover 8:00 until a live race clock proves it.
        if laps <= 3 || race_clock_ms(clock) || race_clock_ms(last) {
            TIMED_EXTRAS_HINT.store(1, Ordering::Relaxed);
        }
        return;
    }
    // 35–50 min is leftover practice in the integer encoding, but a live 35:00–60:00
    // clock with extras is a long timed race (including +4).
    if leftover_practice_len(total) {
        if LAP_MOTO_GATE.load(Ordering::Relaxed) == 1 {
            return;
        }
        if race_clock_ms(clock) || race_clock_ms(last) {
            TIMED_EXTRAS_HINT.store(1, Ordering::Relaxed);
        }
        return;
    }
    // Only when length is unset or a leftover start board — not a 7-min 3-lap leftover.
    let ambiguous = total <= 0 || total < 3 * 60_000;
    if !ambiguous {
        return;
    }
    let saw_race_clock = race_clock_ms(clock) || race_clock_ms(last);
    if !saw_race_clock && RACE_ARMED.load(Ordering::Relaxed) == 0 {
        return;
    }
    // Already counted this down as a 2–4-lap gate. A later 5–60 min dump is garbage.
    if LAP_MOTO_GATE.load(Ordering::Relaxed) == 1 {
        return;
    }
    // A 2–4-lap gate board (≤2 min) that jumps into 5–60 min is plugin garbage, not
    // 6:00+2. Real timed extras already stamped the hint from the race clock.
    if last > 0 && last <= 180_000 && !race_clock_ms(last) && race_clock_ms(clock) {
        return;
    }
    // Warmup can leak extras + a live 5:00 while you're already lapping — that is not
    // a timed +2 prestart. Real 6:00+2 with unset length is lap≤1 and stopped, or past gate.
    let timed_prestart = saw_race_clock && s.current_lap <= 1 && !moving(s);
    let past_prestart = RACE_ARMED.load(Ordering::Relaxed) == 1
        || POST_GATE.load(Ordering::Relaxed) == 1
        || IN_GATE.load(Ordering::Relaxed) == 1;
    if timed_prestart || past_prestart {
        TIMED_EXTRAS_HINT.store(1, Ordering::Relaxed);
    }
}

pub(crate) fn session_sig(s: &Snapshot) -> i32 {
    let raw = s.session_length.max(0);
    let locked = LOCKED_SESSION_LEN.load(Ordering::Relaxed);
    if raw > locked {
        LOCKED_SESSION_LEN.store(raw, Ordering::Relaxed);
    } else if raw > 0 && locked >= 60 && raw < 60 {
        LOCKED_SESSION_LEN.store(raw, Ordering::Relaxed);
    } else if raw > 0 && locked > raw && locked < 60 && raw < 60 && s.session_laps > 0 {
        LOCKED_SESSION_LEN.store(raw, Ordering::Relaxed);
    }
    LOCKED_SESSION_LEN.load(Ordering::Relaxed).max(raw)
}

pub(crate) fn note_session(s: &Snapshot) {
    let sig = session_sig(s);
    let prev = LAST_SESSION_SIG.swap(sig, Ordering::Relaxed);
    if prev != 0 && prev != sig {
        reset_session_clock_track();
        LOCKED_SESSION_LEN.store(s.session_length.max(0), Ordering::Relaxed);
        LAST_SESSION_SIG.store(session_sig(s), Ordering::Relaxed);
    }
    note_timed_extras_hint(s);
    let laps = s.session_laps.max(0);
    let prev_laps = LAST_SESSION_LAPS.swap(laps, Ordering::Relaxed);
    let left_race = prev_laps > 0 && laps == 0;
    // Warmup often keeps length 0 so LAST_SESSION_SIG never bumps. When extras/laps
    // first appear after a practice-like length, clear leaked SAW/ARMED. Sighting with
    // an 8:00 length already locked must not reset (02:00 → 08:00 +1).
    let prev_raw_len = LAST_RAW_SESSION_LEN.swap(s.session_length.max(0), Ordering::Relaxed);
    let prev_len_ms = session_len_ms(prev_raw_len);
    let from_practice = prev_raw_len <= 0
        || leftover_practice_len(prev_len_ms)
        || matches!(session_len_minutes(prev_len_ms), 10 | 12 | 15 | 20);
    // 10–60 min with unpublished extras looks like warmup/practice. When +1 appears on
    // that same timed race, keep overtime bases — a reset sits on `0/1` and can wave
    // the checkered while you still have laps to run.
    let timed_extras_arriving = prev_laps == 0
        && extra_laps_field(laps)
        && (standard_race_minutes(prev_len_ms) || leftover_practice_len(prev_len_ms))
        && session_len_ms(s.session_length) == prev_len_ms
        && (RACE_ARMED.load(Ordering::Relaxed) == 1
            || SESSION_EXPIRED.load(Ordering::Relaxed) == 1);
    let entered_race = prev_laps == 0 && laps > 0 && from_practice && !timed_extras_arriving;
    let kind_changed =
        prev_laps > 0 && laps > 0 && (prev_laps > EXTRA_LAPS_MAX) != (laps > EXTRA_LAPS_MAX);
    let kind = s.session_kind;
    let prev_kind = LAST_SESSION_KIND.swap(kind, Ordering::Relaxed);
    let session_kind_changed = prev_kind > 0 && kind > 0 && prev_kind != kind;
    if left_race || entered_race || kind_changed || session_kind_changed {
        reset_session_clock_track();
        LOCKED_SESSION_LEN.store(s.session_length.max(0), Ordering::Relaxed);
        LAST_SESSION_SIG.store(session_sig(s), Ordering::Relaxed);
        LAST_SESSION_LAPS.store(laps, Ordering::Relaxed);
        LAST_SESSION_KIND.store(kind, Ordering::Relaxed);
        note_timed_extras_hint(s);
    }
    let lap = s.current_lap.max(0);
    let prev_lap = LAST_CUR_LAP.swap(lap, Ordering::Relaxed);
    let race_over = CHECKERED_LATCH.load(Ordering::Relaxed) == 1
        || SESSION_EXPIRED.load(Ordering::Relaxed) == 1
        || LAP_GREEN.load(Ordering::Relaxed) == 1;
    if prev_lap > 1 && lap <= 1 && (POST_GATE.load(Ordering::Relaxed) == 0 || race_over) {
        reset_session_clock_track();
        LOCKED_SESSION_LEN.store(s.session_length.max(0), Ordering::Relaxed);
        LAST_SESSION_SIG.store(session_sig(s), Ordering::Relaxed);
        LAST_SESSION_LAPS.store(laps, Ordering::Relaxed);
        LAST_CUR_LAP.store(lap, Ordering::Relaxed);
        note_timed_extras_hint(s);
    }
}

pub(crate) fn extra_laps(s: &Snapshot) -> i32 {
    if is_lap_race(s) {
        return 0;
    }
    if s.session_laps > 0 {
        CACHED_SESSION_LAPS.store(s.session_laps, Ordering::Relaxed);
        s.session_laps
    } else if SESSION_EXPIRED.load(Ordering::Relaxed) == 1 {
        CACHED_SESSION_LAPS.load(Ordering::Relaxed)
    } else {
        0
    }
}

/// Race 1 is `6`, race 2 is `7`. Warmup is `5`. Unknown (`-1` / `0`) stays inferred.
fn is_race_session_kind(kind: i32) -> bool {
    kind >= 6
}

pub(crate) fn is_warmup(s: &Snapshot) -> bool {
    // Kind 5 is warmup even when extras / lap counts leak — those fields are for race motos.
    if s.session_kind == 5 {
        return true;
    }
    if overtime_active(s) || is_lap_race(s) {
        return false;
    }
    // 1–4 is extras on a timed moto, not practice.
    if extra_laps_field(s.session_laps) {
        return false;
    }
    // A race session at 10–30 min with unpublished extras is not warmup.
    if is_race_session_kind(s.session_kind) {
        return false;
    }
    let total = session_len_ms(s.session_length);
    leftover_practice_len(total) || matches!(session_len_minutes(total), 10 | 12 | 15 | 20)
}

/// Riding telemetry, or rider positions while `SpectateVehicles` is live.
/// Leftover dots after spectate (garage) are not a session — same as leftover standings.
pub fn live_session(s: &Snapshot, spectating: bool) -> bool {
    s.has_telemetry != 0 || (spectating && s.rider_count > 0)
}

/// Live HUD preset from session state. `None` in the garage/menus so the last slot is held.
pub fn session_preset(s: &Snapshot, spectating: bool) -> Option<SessionPreset> {
    if !live_session(s, spectating) {
        return None;
    }
    if spectating {
        return Some(SessionPreset::Spectate);
    }
    if s.session_kind == 5 {
        return Some(SessionPreset::Warmup);
    }
    if is_race_session_kind(s.session_kind) || is_lap_race(s) || overtime_active(s) {
        return Some(SessionPreset::Race);
    }
    let total = session_len_ms(s.session_length);
    // Practice length before leaked extras — kind 5 already returned Warmup above.
    if session_len_minutes(total) >= 40 {
        return Some(SessionPreset::Practice);
    }
    if extra_laps_field(s.session_laps) {
        return Some(SessionPreset::Race);
    }
    if is_warmup(s) {
        return Some(SessionPreset::Warmup);
    }
    // Open practice / Testing Setup: no race length and no extras.
    if s.session_laps <= 0 && s.session_length <= 0 {
        return Some(SessionPreset::Practice);
    }
    // Unpublished timed moto (standard race minutes) still maps to Race.
    if standard_race_minutes(total) || s.session_length > 0 || prestart(s) {
        return Some(SessionPreset::Race);
    }
    None
}

/// Open practice (40+ min or Testing Setup with no race length), not a race moto and not short warmup.
pub fn is_practice_session(s: &Snapshot) -> bool {
    matches!(session_preset(s, false), Some(SessionPreset::Practice))
}

/// Lap count or race minutes once a race moto has published a length.
/// Practice, warmup, and spectate stay empty.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RaceDistance {
    Laps(i32),
    Minutes(i32),
}

pub fn published_race_distance(s: &Snapshot) -> Option<RaceDistance> {
    if session_preset(s, false) != Some(SessionPreset::Race) {
        return None;
    }
    if is_lap_race(s) {
        let laps = s.session_laps;
        return (laps >= 2).then_some(RaceDistance::Laps(laps));
    }
    let total = session_len_ms(s.session_length);
    let minutes = session_len_minutes(total);
    if minutes >= 1 && standard_race_minutes(total) {
        Some(RaceDistance::Minutes(minutes))
    } else {
        None
    }
}

/// Short warmup (not Practice). Motos stores those laps on the race visit.
pub(crate) fn skip_warmup_laps(s: &Snapshot) -> bool {
    is_warmup(s) && !is_practice_session(s)
}

/// A 4–29 minute length the standard set skips (7:00+2). Timed when the live
/// clock is sitting on that length. A later dump to 16:20 is not this clock.
fn published_length_countdown(s: &Snapshot) -> bool {
    if !extra_laps_field(s.session_laps) {
        return false;
    }
    let total = session_len_ms(s.session_length);
    let minutes = session_len_minutes(total);
    if !(4..=29).contains(&minutes) || standard_race_minutes(total) {
        return false;
    }
    let clock = s.session_time_ms.max(0);
    (clock - total).abs() < 30_000
}

pub(crate) fn is_lap_race(s: &Snapshot) -> bool {
    // Extra laps are 1–4 on a timed set. Five or more is always a lap moto,
    // even when leftover warmup (10:00) is still sitting in session length.
    if s.session_laps > EXTRA_LAPS_MAX {
        return true;
    }
    if s.session_laps <= 1 {
        return false;
    }
    note_timed_extras_hint(s);
    // 7:00+2 publishes a 4 s board before the length. That latches a lap moto.
    // A clock on the published length is the race, and the hint keeps the
    // countdown after it leaves that band.
    if published_length_countdown(s) {
        LAP_MOTO_GATE.store(0, Ordering::Relaxed);
        TIMED_EXTRAS_HINT.store(1, Ordering::Relaxed);
        return false;
    }
    // A 2–4-lap moto that already ran its gate stays a lap race even if extras-hint
    // later sees a glitched 5–60 min clock.
    if LAP_MOTO_GATE.load(Ordering::Relaxed) == 1 {
        return true;
    }
    // 4-lap leftover 8:00/10:00 is a lap moto. 8:00+4 needs a live race clock hint.
    if s.session_laps == 4 {
        return TIMED_EXTRAS_HINT.load(Ordering::Relaxed) != 1;
    }
    let total = session_len_ms(s.session_length);
    if leftover_practice_len(total) || standard_race_minutes(total) {
        return false;
    }
    // Once we have seen a real timed race clock with unset/start-board length, stick
    // to extras — do not flip to a 2/3-lap moto after the gate.
    if TIMED_EXTRAS_HINT.load(Ordering::Relaxed) == 1 {
        return false;
    }
    let clock = s.session_time_ms.max(0);
    // 2 extras leak into warmup (length 0, live mid clock). A 2-lap moto has a
    // gate / leftover start board, not a 5–60 min clock, until green.
    if s.session_laps == 2
        && total <= 0
        && clock > 180_000
        && LAP_GREEN.load(Ordering::Relaxed) == 0
        && POST_GATE.load(Ordering::Relaxed) == 0
    {
        return false;
    }
    true
}

pub(crate) fn lap_race_text(s: &Snapshot) -> String {
    let n = effective_race_laps(s);
    // Once you are done, freeze on what you actually covered.
    if i_finished(s) {
        return format!("{} / {}", laps_done(s).min(n), n);
    }
    // A 2-lap moto counts crossings: the opening lap is 0/2, and the finish
    // line starts 1/2. Longer motos still show the lap you are on.
    let shown = if s.session_laps == 2 {
        laps_done(s).clamp(0, n)
    } else {
        race_lap(s).min(n)
    };
    format!("{} / {}", shown, n)
}

pub(crate) fn leader_num_laps(s: &Snapshot) -> i32 {
    let rows = s.standing_count.max(0) as usize;
    s.standings
        .iter()
        .take(rows)
        .find(|row| row.position == 1)
        .map(|row| row.num_laps)
        .or_else(|| s.standings.iter().take(rows).map(|row| row.num_laps).max())
        .unwrap_or(0)
        .max(0)
}

/// Metres still to the line for the race leader. `None` when the line or the
/// leader's position is unknown.
fn leader_metres_to_line(s: &Snapshot) -> Option<f32> {
    if !line_known(s) || s.track_length <= 10.0 {
        return None;
    }
    let race_num = leader_race_num(s)?;
    let pos = rider_lap_pos(s, race_num)?;
    Some(forward_frac(pos, start_finish_fraction(s)) * s.track_length)
}

pub(crate) fn overtime_base(s: &Snapshot) -> i32 {
    let mut base = OVERTIME_BASE_LAP.load(Ordering::Relaxed);
    let local = focus_num_laps(s);
    let lead = leader_num_laps(s);
    // A near-line expiry crossing finishes the lap already under way. Swallow that
    // single step before it can count as extras. A jump of more than one is not it.
    if OVERTIME_LINE_ARM.load(Ordering::Relaxed) == 1 && base > 0 {
        if lead == base + 1 {
            OVERTIME_BASE_LAP.store(lead, Ordering::Relaxed);
            OVERTIME_LINE_ARM.store(0, Ordering::Relaxed);
            base = lead;
        } else if lead > base + 1 {
            OVERTIME_LINE_ARM.store(0, Ordering::Relaxed);
        }
    }
    let local0 = OVERTIME_LOCAL_BASE.load(Ordering::Relaxed);
    // extras_started without recursing: leader past the frozen expiry base.
    let started = overtime_active(s) && base > 0 && lead > base;
    // High-water the timed-lap count. Standings often reset to 0 when +1 is published
    // late; following that drop would fire white/checkered three laps early.
    if local > 0 && (local0 < 0 || (local > local0 && !started)) {
        OVERTIME_LOCAL_BASE.store(local, Ordering::Relaxed);
    }
    if base < 0 || base == 0 {
        if lead > 0 {
            OVERTIME_BASE_LAP.store(lead, Ordering::Relaxed);
            base = lead;
            let near_line = leader_metres_to_line(s).is_some_and(|metres| metres <= EXPIRY_LINE_M);
            OVERTIME_LINE_ARM.store(i32::from(near_line), Ordering::Relaxed);
        } else if base < 0 {
            return -1;
        }
    }
    base
}

pub(crate) fn extras_started(s: &Snapshot) -> bool {
    let base = overtime_base(s);
    overtime_active(s) && base > 0 && leader_num_laps(s) > base
}

/// Keep the overtime bases current. Backmarkers who cross after the clock hits zero are
/// still finishing the timed lap, so those crossings do not count and your base follows
/// you until the leader starts the extras. Call this before reading either base — it used
/// to happen as a side effect of `local_overtime_done`, which meant the bases were only
/// right if something happened to format the banner first.
pub(crate) fn note_overtime_base(s: &Snapshot) {
    if !overtime_active(s) {
        return;
    }
    let _ = overtime_base(s);
    let local = focus_num_laps(s);
    if local > 0 && !extras_started(s) {
        let prev = OVERTIME_LOCAL_BASE.load(Ordering::Relaxed);
        if local > prev {
            OVERTIME_LOCAL_BASE.store(local, Ordering::Relaxed);
        }
    }
}

pub(crate) fn local_overtime_done(s: &Snapshot) -> i32 {
    let local = focus_num_laps(s);
    if local <= 0 {
        return 0;
    }
    note_overtime_base(s);
    // Your next crossing after the leader starts extras begins your first extra; the one
    // after that completes it.
    if !extras_started(s) {
        return 0;
    }
    let local0 = OVERTIME_LOCAL_BASE.load(Ordering::Relaxed);
    if local0 < 0 {
        OVERTIME_LOCAL_BASE.store(local, Ordering::Relaxed);
        return 0;
    }
    (local - local0 - 1).max(0)
}

fn raw_overtime_taken(s: &Snapshot) -> i32 {
    let local = focus_num_laps(s);
    let local0 = OVERTIME_LOCAL_BASE.load(Ordering::Relaxed);
    if local <= 0 || local0 < 0 {
        return 0;
    }
    (local - local0).max(0)
}

fn leader_race_num(s: &Snapshot) -> Option<i32> {
    s.standings
        .iter()
        .take(s.standing_count.max(0) as usize)
        .find(|row| row.position == 1)
        .map(|row| row.race_num)
        .filter(|&n| n > 0)
}

fn leader_lap_lead(s: &Snapshot) -> i32 {
    leader_num_laps(s) - focus_num_laps(s)
}

fn leader_is_lap_up(s: &Snapshot) -> bool {
    let lead = leader_lap_lead(s);
    // After the leader takes the flag, +1 completed lap is usually "they finished,
    // you have not crossed yet" — same last lap, not lapped (`need = 2`). Timed
    // extras are different: on `1/2` while they have finished `2/2`, lead is often
    // only 1 and you are a full extra behind, not on their finishing lap.
    let need = if leader_finished(s) {
        if extras_started(s) && raw_overtime_taken(s) < extra_laps(s).max(1) {
            1
        } else {
            2
        }
    } else {
        1
    };
    if lead >= need {
        return true;
    }
    // Extras started and you have not taken one: the leader is a lap up when they
    // hold at least one completed lap more. Equal `num_laps` during extras is the
    // same physical lap — do not arm the pass latch on a mid-pack battle.
    !leader_finished(s) && extras_started(s) && raw_overtime_taken(s) == 0 && lead >= 1
}

/// Latch the moment the lap-up leader goes past you. `gap_laps` waits for a line crossing.
fn note_lapped_by_leader(s: &Snapshot) {
    if s.on_track == 0 || is_warmup(s) || prestart(s) {
        LEADER_BEHIND.store(0, Ordering::Relaxed);
        return;
    }
    if LAPPED_LATCH.load(Ordering::Relaxed) == 1 {
        return;
    }
    let focus = if s.focus_race_num > 0 {
        s.focus_race_num
    } else {
        s.local_race_num
    };
    let Some(lead) = leader_race_num(s) else {
        return;
    };
    if lead == focus || !leader_is_lap_up(s) {
        LEADER_BEHIND.store(0, Ordering::Relaxed);
        return;
    }
    let (Some(lp), Some(fp)) = (rider_lap_pos(s, lead), rider_lap_pos(s, focus)) else {
        return;
    };
    let len = if s.track_length > 10.0 {
        s.track_length
    } else {
        1000.0
    };
    let along = wrap_signed(lp - fp) * len;
    // Same as live order: centerline projection is only trustworthy nearby. An
    // over/under tabletop can snap one bike to a wrong spline segment half a lap away.
    if along.abs() > PAIR_MAX_M {
        LEADER_BEHIND.store(0, Ordering::Relaxed);
        LAST_LEAD_FRAC.store((lp * 10_000.0).round() as i32, Ordering::Relaxed);
        return;
    }
    let prev = LAST_LEAD_FRAC.load(Ordering::Relaxed);
    LAST_LEAD_FRAC.store((lp * 10_000.0).round() as i32, Ordering::Relaxed);
    let lead_delta_m = if prev >= 0 {
        wrap_signed(lp - prev as f32 / 10_000.0) * len
    } else {
        0.0
    };
    // A real pass is continuous. A tabletop under-path teleport must not count as moving.
    if lead_delta_m.abs() > PAIR_MAX_M {
        LEADER_BEHIND.store(0, Ordering::Relaxed);
        return;
    }
    let lead_moved = prev >= 0 && lead_delta_m > PASS_M;
    if along < -PASS_M {
        LEADER_BEHIND.store(1, Ordering::Relaxed);
    } else if along > PASS_M {
        if lead_moved && LEADER_BEHIND.swap(0, Ordering::Relaxed) == 1 {
            LAPPED_LATCH.store(1, Ordering::Relaxed);
        } else if !lead_moved {
            // You rode around; the wrap flipped. That is not them going past you.
            LEADER_BEHIND.store(0, Ordering::Relaxed);
        }
    }
}

pub(crate) fn local_overtime_taken(s: &Snapshot) -> i32 {
    let _ = local_overtime_done(s);
    if !extras_started(s) {
        return 0;
    }
    raw_overtime_taken(s)
}

pub(crate) fn overtime_lap_text(s: &Snapshot) -> String {
    let n = effective_extra_laps(s);
    let taken = local_overtime_taken(s).min(n);
    format!("{taken}/{n}")
}

pub(crate) fn timed_clock_text(_s: &Snapshot, remain: i32) -> String {
    // Live timed race: clock only. Extras (`0/1` / `0/2`) show after expiry.
    format_countdown(remain)
}

pub(crate) fn moving(s: &Snapshot) -> bool {
    s.local_speed >= 3.5
}

pub(crate) fn near_session_total(clock: i32, total: i32) -> bool {
    if total <= 0 {
        return false;
    }
    clock >= total || (clock - total).abs() < 30_000
}

pub(crate) fn is_gate_clock(clock: i32, total: i32) -> bool {
    // Start boards are 2:00 / 0:50 / 0:30 / ~15s. Practice remaining at 3:00 is not a board.
    let long_session = total > 180_000 || total <= 0;
    long_session && clock >= 8_000 && clock <= 120_000 && (total <= 0 || clock * 3 < total)
}

pub(crate) fn clock_stuck(clock: i32, last: i32) -> bool {
    static HOLD: Mutex<Option<(i32, f32)>> = Mutex::new(None);
    clock_held_for(clock, last, 2.5, &HOLD)
}

pub(crate) fn board_held(clock: i32, last: i32) -> bool {
    static HOLD: Mutex<Option<(i32, f32)>> = Mutex::new(None);
    clock_held_for(clock, last, 0.6, &HOLD)
}

pub(crate) fn clock_held_for(
    clock: i32,
    last: i32,
    secs: f32,
    hold: &Mutex<Option<(i32, f32)>>,
) -> bool {
    let Ok(mut hold) = hold.lock() else {
        return false;
    };
    let now = anim_now();
    if last > 0 && clock == last {
        match *hold {
            Some((prev, at)) if prev == clock => now - at >= secs,
            _ => {
                *hold = Some((clock, now));
                false
            }
        }
    } else {
        *hold = Some((clock, now));
        false
    }
}

pub(crate) fn session_remain_ms(s: &Snapshot) -> Option<i32> {
    note_session(s);
    if is_lap_race(s) {
        let clock = s.session_time_ms.max(0);
        let last = LAST_SESSION_CLOCK.load(Ordering::Relaxed);
        LAST_SESSION_CLOCK.store(clock, Ordering::Relaxed);
        let counting_up = last > 0 && clock > last + 400;
        let counting_down = last > 0 && clock < last && last - clock < 5_000 && clock <= 180_000;
        if LAP_GREEN.load(Ordering::Relaxed) == 1 {
            IN_GATE.store(0, Ordering::Relaxed);
            return None;
        }
        if counting_up || (moving(s) && counting_down && last <= 8_000) {
            LAP_GREEN.store(1, Ordering::Relaxed);
            IN_GATE.store(0, Ordering::Relaxed);
            POST_GATE.store(1, Ordering::Relaxed);
            return None;
        }
        if POST_GATE.load(Ordering::Relaxed) == 1 {
            IN_GATE.store(0, Ordering::Relaxed);
            return None;
        }
        let in_gate = IN_GATE.load(Ordering::Relaxed) == 1;
        if in_gate && last > 0 && (clock > last + 400 || clock < 500 || clock > 180_000) {
            POST_GATE.store(1, Ordering::Relaxed);
            IN_GATE.store(0, Ordering::Relaxed);
            return None;
        }
        if clock > 0 && clock <= 180_000 {
            IN_GATE.store(1, Ordering::Relaxed);
            LAP_MOTO_GATE.store(1, Ordering::Relaxed);
            RACE_ARMED.store(0, Ordering::Relaxed);
            SESSION_EXPIRED.store(0, Ordering::Relaxed);
            return Some(clock);
        }
        if in_gate {
            POST_GATE.store(1, Ordering::Relaxed);
        }
        if moving(s) {
            LAP_GREEN.store(1, Ordering::Relaxed);
        }
        IN_GATE.store(0, Ordering::Relaxed);
        return None;
    }
    let total = effective_session_len_ms(s);
    if total <= 0 && session_len_ms(s.session_length) <= 0 && s.session_laps <= 0 {
        if RACE_ARMED.load(Ordering::Relaxed) == 1
            && SESSION_EXPIRED.load(Ordering::Relaxed) == 1
            && extra_laps(s) > 0
        {
            return Some(0);
        }
        if s.session_time_ms > 0 {
            return Some(s.session_time_ms);
        }
        return None;
    }
    let last = LAST_SESSION_CLOCK.load(Ordering::Relaxed);
    let raw = s.session_time_ms.max(0);
    let saw = SAW_SESSION_TIME.load(Ordering::Relaxed) == 1;
    let armed = RACE_ARMED.load(Ordering::Relaxed) == 1;
    let in_gate_now = IN_GATE.load(Ordering::Relaxed) == 1;
    let mut clock = if raw > total + 30_000 && total > 0 && last > 0 && !in_gate_now && armed {
        last
    } else {
        raw
    };
    // Game republishes the 8s/10s board during a live race. Keep counting the
    // captured remain by wall time until the real clock comes back.
    let board_anchor = {
        let held = BOARD_HOLD_REMAIN.load(Ordering::Relaxed);
        if held > 60_000 { held } else { last }
    };
    if armed
        && !in_gate_now
        && clock >= 8_000
        && clock <= 15_000
        && clock + 20_000 < board_anchor
        && board_anchor > 60_000
    {
        clock = held_board_remain(board_anchor);
    } else {
        BOARD_HOLD_REMAIN.store(0, Ordering::Relaxed);
    }
    // A start board can land anywhere, not just the 8–15s window: 04:43 → 00:05 → 04:42
    // has been seen mid-moto. Remember where such a dip started, because the way back up
    // out of it is the frame that otherwise reads as the clock having run out.
    let mut dip_from = DIP_FROM_CLOCK.load(Ordering::Relaxed);
    if dip_from < 0 && armed && !in_gate_now && last > 0 && clock > 0 && last - clock > 30_000 {
        DIP_FROM_CLOCK.store(last, Ordering::Relaxed);
        dip_from = last;
    }
    // Still down in the dip, and back out of it: back within half a minute of where the dip
    // began means the board was never the clock and no time ran out. A clock that really
    // expired comes back at the session length instead, above where it went down.
    let dipping = dip_from > 0 && clock > 0 && dip_from - clock > 30_000;
    let resumed = dip_from > 0 && clock > 0 && clock <= dip_from && dip_from - clock <= 30_000;
    let board_dip = dipping || resumed;
    if resumed || (!dipping && last > 0 && clock < last && last - clock <= 5_000) {
        DIP_FROM_CLOCK.store(-1, Ordering::Relaxed);
    }
    // One drop of more than 30 s can be a real countdown between two samples, so
    // that sample still shows. A further drop inside the same dip is the green-flag
    // stair: keep the pre-dip clock, and show the raw time again once it is back
    // within 30 s. That return is not the clock running out.
    let another_step =
        dipping && last > 0 && last != dip_from && last > clock && last - clock > 15_000;
    if another_step {
        CLOCK_STAIR.store(true, Ordering::Relaxed);
    }
    if !dipping {
        CLOCK_STAIR.store(false, Ordering::Relaxed);
    }
    if dipping && CLOCK_STAIR.load(Ordering::Relaxed) {
        clock = dip_from;
    }
    let near_full = near_session_total(clock, total);
    let last_near_full = last > 0 && near_session_total(last, total);
    let started = moving(s) || s.current_lap > 1;
    // After a real race countdown, a snap back to session length (or 8s junk) is expiry.
    if !board_dip
        && saw
        && armed
        && last > 0
        && last + 30_000 < total
        && extra_laps(s) > 0
        && last <= 90_000
    {
        let jump_to_length = (near_full || clock >= total) && clock > last + 20_000;
        let jump_off_zero = last <= 15_000 && clock > last + 4_000;
        if jump_to_length || jump_off_zero {
            LAST_SESSION_CLOCK.store(last, Ordering::Relaxed);
            SESSION_EXPIRED.store(1, Ordering::Relaxed);
            let _ = overtime_base(s);
            return Some(0);
        }
    }
    // Once armed and counting down, a republish of session length must not snap
    // the dash back to full race time (07:59 → 08:00). Near-full is a 30 s band, so
    // the opening minute of an 8:00 race is still "near full" on every tick.
    if !board_dip
        && armed
        && !in_gate_now
        && last > 0
        && last + 1_000 < total
        && near_full
        && clock > last
    {
        clock = last;
    }
    if !board_dip
        && s.session_laps <= 0
        && last > 0
        && last <= 20_000
        && near_full
        && last + 30_000 < total
    {
        LAST_SESSION_CLOCK.store(0, Ordering::Relaxed);
        IN_GATE.store(0, Ordering::Relaxed);
        SESSION_EXPIRED.store(1, Ordering::Relaxed);
        let _ = overtime_base(s);
        return Some(0);
    }
    LAST_SESSION_CLOCK.store(clock, Ordering::Relaxed);
    let in_gate = IN_GATE.load(Ordering::Relaxed) == 1;
    let mut post = POST_GATE.load(Ordering::Relaxed) == 1;
    let prestart_sized = |t: i32| t > 0 && t <= 120_000 && total > 180_000 && t * 3 < total;
    if s.session_laps > 0 && prestart_sized(last) && near_full && !armed {
        POST_GATE.store(1, Ordering::Relaxed);
        post = true;
    }
    let short_clock =
        clock > 0 && clock <= 35_000 && (total <= 0 || clock * 3 < total) && total > 180_000;
    let warmup = is_warmup(s);
    // Kind 5 with leaked extras must not pin a frozen start board as a live gate.
    let enter_gate = is_gate_clock(clock, total)
        && !moving(s)
        && !post
        && !armed
        && s.session_laps > 0
        && !warmup;
    let gate_clock = enter_gate
        || (!post && in_gate && clock > 0 && clock <= 180_000 && (total <= 0 || clock * 3 < total));
    // Frozen 00:30 wait is a race board. Practice remaining that hitch-pauses is not.
    let held_board =
        !warmup && short_clock && board_held(clock, last) && !armed && s.session_laps > 0;
    if held_board {
        IN_GATE.store(0, Ordering::Relaxed);
        POST_GATE.store(1, Ordering::Relaxed);
        SESSION_CLOCK_MODE.store(1, Ordering::Relaxed);
        let shown = wait_display_ms(total, clock);
        LAST_SESSION_CLOCK.store(shown.max(total), Ordering::Relaxed);
        return Some(shown);
    }
    let drop_off_gate = in_gate && last >= 8_000 && clock < 500;
    let leave_gate = in_gate && !gate_clock && !near_full && clock > 180_000;
    let wait_off_gate = in_gate && !gate_clock && near_full;
    // A timed race has already left the only short board. A 30 s stair above 3 minutes
    // is still that board, not the race clock. A lap moto never reaches this branch.
    let timed_hold = s.session_laps > 0 && clock > 180_000;
    let board_restart = in_gate
        && !post
        && last > 0
        && clock > last + 2_000
        && (clock <= 180_000 || timed_hold);
    // Timed: first gate board only. After it runs down or jumps up (45s/30s),
    // show race length — do not re-enter a short countdown.
    let gate = gate_clock && !drop_off_gate && !armed && !board_restart;
    let race_ticking = !gate_clock
        && last > 180_000
        && clock > 5_000
        && clock < last
        && last - clock >= 50
        && last - clock < 5_000;
    let waiting_for_race = (post || board_restart)
        && !armed
        && !race_ticking
        && (clock <= 180_000 || timed_hold);

    if drop_off_gate {
        IN_GATE.store(0, Ordering::Relaxed);
        POST_GATE.store(1, Ordering::Relaxed);
        SESSION_CLOCK_MODE.store(1, Ordering::Relaxed);
        SESSION_EXPIRED.store(0, Ordering::Relaxed);
        let shown = wait_display_ms(total, clock);
        LAST_SESSION_CLOCK.store(shown.max(total), Ordering::Relaxed);
        if !armed {
            return Some(shown);
        }
        RACE_ARMED.store(1, Ordering::Relaxed);
    } else if leave_gate {
        IN_GATE.store(0, Ordering::Relaxed);
        POST_GATE.store(1, Ordering::Relaxed);
        SESSION_CLOCK_MODE.store(1, Ordering::Relaxed);
        SESSION_EXPIRED.store(0, Ordering::Relaxed);
        SAW_SESSION_TIME.store(0, Ordering::Relaxed);
        OVERTIME_BASE_LAP.store(-1, Ordering::Relaxed);
        OVERTIME_LOCAL_BASE.store(-1, Ordering::Relaxed);
        OVERTIME_LINE_ARM.store(0, Ordering::Relaxed);
        CHECKERED_LATCH.store(0, Ordering::Relaxed);
        let shown = wait_display_ms(total, clock);
        LAST_SESSION_CLOCK.store(shown.max(total), Ordering::Relaxed);
        return Some(shown);
    } else if wait_off_gate {
        IN_GATE.store(0, Ordering::Relaxed);
        POST_GATE.store(1, Ordering::Relaxed);
        SESSION_CLOCK_MODE.store(1, Ordering::Relaxed);
        let shown = wait_display_ms(total, clock);
        LAST_SESSION_CLOCK.store(shown.max(total), Ordering::Relaxed);
        return Some(shown);
    } else if waiting_for_race {
        IN_GATE.store(0, Ordering::Relaxed);
        POST_GATE.store(1, Ordering::Relaxed);
        SESSION_CLOCK_MODE.store(1, Ordering::Relaxed);
        if is_lap_race(s) {
            return None;
        }
        let shown = wait_display_ms(total, clock);
        LAST_SESSION_CLOCK.store(shown.max(total), Ordering::Relaxed);
        return Some(shown);
    } else if gate {
        IN_GATE.store(1, Ordering::Relaxed);
        POST_GATE.store(0, Ordering::Relaxed);
        SESSION_CLOCK_MODE.store(1, Ordering::Relaxed);
        SESSION_EXPIRED.store(0, Ordering::Relaxed);
        SAW_SESSION_TIME.store(0, Ordering::Relaxed);
        OVERTIME_BASE_LAP.store(-1, Ordering::Relaxed);
        OVERTIME_LOCAL_BASE.store(-1, Ordering::Relaxed);
        OVERTIME_LINE_ARM.store(0, Ordering::Relaxed);
        CHECKERED_LATCH.store(0, Ordering::Relaxed);
        RACE_ARMED.store(0, Ordering::Relaxed);
    } else if race_ticking {
        RACE_ARMED.store(1, Ordering::Relaxed);
        POST_GATE.store(0, Ordering::Relaxed);
        SESSION_CLOCK_MODE.store(1, Ordering::Relaxed);
        IN_GATE.store(0, Ordering::Relaxed);
    } else if !board_dip
        && !gate
        && IN_GATE.load(Ordering::Relaxed) == 0
        && RACE_ARMED.load(Ordering::Relaxed) == 1
        && extra_laps(s) > 0
        && SAW_SESSION_TIME.load(Ordering::Relaxed) == 1
        && started
        && last > 0
        && last <= 90_000
        && !last_near_full
        && last + 30_000 < total
        && near_full
        && clock > last + 20_000
    {
        SESSION_EXPIRED.store(1, Ordering::Relaxed);
        let _ = overtime_base(s);
        return Some(0);
    } else if near_full
        && SESSION_EXPIRED.load(Ordering::Relaxed) == 0
        && !(SESSION_CLOCK_MODE.load(Ordering::Relaxed) == 2 && last_near_full && started)
    {
        SESSION_CLOCK_MODE.store(1, Ordering::Relaxed);
        SESSION_EXPIRED.store(0, Ordering::Relaxed);
        OVERTIME_BASE_LAP.store(-1, Ordering::Relaxed);
        OVERTIME_LOCAL_BASE.store(-1, Ordering::Relaxed);
        OVERTIME_LINE_ARM.store(0, Ordering::Relaxed);
        CHECKERED_LATCH.store(0, Ordering::Relaxed);
    } else if !gate && last > 0 && clock + 400 < last {
        SESSION_CLOCK_MODE.store(1, Ordering::Relaxed);
    } else if last > 0
        && clock > last + 400
        && !near_full
        && !gate
        && clock * 4 < total * 3
        && last * 4 < total * 3
        && last > 45_000
    {
        SESSION_CLOCK_MODE.store(2, Ordering::Relaxed);
    }

    let cap = |c: i32| if total > 0 { c.min(total) } else { c.max(0) };
    let remain = if gate {
        cap(clock)
    } else {
        match SESSION_CLOCK_MODE.load(Ordering::Relaxed) {
            2 => (total - clock).max(0),
            1 => {
                if clock <= 0 {
                    if started && last > 2000 && !near_full && !drop_off_gate && !leave_gate {
                        0
                    } else {
                        total
                    }
                } else {
                    cap(clock)
                }
            }
            _ => {
                if clock <= 0 {
                    total
                } else {
                    cap(clock)
                }
            }
        }
    };

    if !gate && remain > 4000 {
        SAW_SESSION_TIME.store(1, Ordering::Relaxed);
    }

    if SESSION_EXPIRED.load(Ordering::Relaxed) == 1 {
        if near_full && remain > 20_000 && !started {
            SESSION_EXPIRED.store(0, Ordering::Relaxed);
            OVERTIME_BASE_LAP.store(-1, Ordering::Relaxed);
            OVERTIME_LOCAL_BASE.store(-1, Ordering::Relaxed);
            OVERTIME_LINE_ARM.store(0, Ordering::Relaxed);
            CHECKERED_LATCH.store(0, Ordering::Relaxed);
            return Some(remain);
        }
        let _ = overtime_base(s);
        return Some(0);
    }

    let timed_out = remain <= 800 || (clock_stuck(clock, last) && remain > 0 && remain <= 2_000);
    let raced = RACE_ARMED.load(Ordering::Relaxed) == 1
        || (SAW_SESSION_TIME.load(Ordering::Relaxed) == 1 && last > 60_000);
    if !gate
        && started
        && timed_out
        && extra_laps(s) > 0
        && raced
        && SAW_SESSION_TIME.load(Ordering::Relaxed) == 1
    {
        RACE_ARMED.store(1, Ordering::Relaxed);
        SESSION_EXPIRED.store(1, Ordering::Relaxed);
        let _ = overtime_base(s);
        return Some(0);
    }
    // Warmup / practice: blank after zero, or when the game jumps to / freezes a
    // start board (~00:30) without ever counting down through zero. Near-zero
    // frames (00:02) are not boards — those still show until the zero path.
    let snap_to_board =
        last > 60_000 && clock >= 8_000 && clock <= 35_000 && last - clock > 30_000;
    let stuck_board = clock >= 8_000
        && clock <= 35_000
        && (total <= 0 || clock * 3 < total)
        && (board_held(clock, last) || clock_stuck(clock, last));
    let junk_board = snap_to_board || stuck_board;
    if (s.session_laps <= 0 || warmup)
        && !gate
        && SAW_SESSION_TIME.load(Ordering::Relaxed) == 1
        && (remain <= 800 || (started && timed_out) || junk_board)
    {
        IN_GATE.store(0, Ordering::Relaxed);
        SESSION_EXPIRED.store(1, Ordering::Relaxed);
        let _ = overtime_base(s);
        return Some(0);
    }
    Some(remain)
}

pub(crate) fn overtime_active(s: &Snapshot) -> bool {
    extra_laps(s) > 0
        && RACE_ARMED.load(Ordering::Relaxed) == 1
        && SAW_SESSION_TIME.load(Ordering::Relaxed) == 1
        && SESSION_EXPIRED.load(Ordering::Relaxed) == 1
}

/// 5–60 min race clock already expired, extras field still 0. MX Bikes often
/// publishes `session_laps = 1` a lap later and resets standings when it does.
fn timed_race_awaiting_extras(s: &Snapshot) -> bool {
    let total = session_len_ms(s.session_length);
    RACE_ARMED.load(Ordering::Relaxed) == 1
        && SESSION_EXPIRED.load(Ordering::Relaxed) == 1
        && !is_lap_race(s)
        && s.session_laps <= 0
        && (standard_race_minutes(total)
            || (leftover_practice_len(total) && is_race_session_kind(s.session_kind)))
}

pub(crate) fn prestart(s: &Snapshot) -> bool {
    if IN_GATE.load(Ordering::Relaxed) == 1 {
        return true;
    }
    if overtime_active(s) {
        return false;
    }
    if moving(s) {
        return false;
    }
    let total = session_len_ms(s.session_length);
    let clock = s.session_time_ms.max(0);
    s.current_lap <= 1 || laps_done(s) <= 0 || (total > 0 && near_session_total(clock, total))
}

pub(crate) fn laps_done(s: &Snapshot) -> i32 {
    // The plugin publishes `current_lap = max(current_lap, num_laps)`, so standings
    // laps are the completed count and never run ahead of the lap you are on.
    focus_num_laps(s)
}

/// Practice / warmup: classification often skips a crashed lap. `RunLap` still
/// advances `current_lap`. Do not use that in a moto — a glitched current_lap
/// would move flags and laps-left.
fn practice_open_laps(s: &Snapshot) -> bool {
    s.session_laps <= 0 && !is_race_session_kind(s.session_kind)
}

/// Completed laps for a standings / relative row. Practice uses the plugin
/// crossing when the game left `num_laps` on the last valid lap.
pub(crate) fn standing_num_laps(s: &Snapshot, race_num: i32, num_laps: i32) -> i32 {
    let n = num_laps.max(0);
    let focus = if s.focus_race_num > 0 {
        s.focus_race_num
    } else {
        s.local_race_num
    };
    if race_num == focus && practice_open_laps(s) && s.current_lap > 0 {
        n.max((s.current_lap - 1).max(0))
    } else {
        n
    }
}

pub(crate) fn rider_current_lap(s: &Snapshot, race_num: i32, num_laps: i32) -> i32 {
    let done = num_laps.max(0);
    let focus = if s.focus_race_num > 0 {
        s.focus_race_num
    } else {
        s.local_race_num
    };
    if race_num == focus && s.current_lap > done {
        s.current_lap
    } else {
        done + 1
    }
}

pub(crate) fn focus_num_laps(s: &Snapshot) -> i32 {
    let focus = if s.focus_race_num > 0 {
        s.focus_race_num
    } else {
        s.local_race_num
    };
    let from_plugin = if s.current_lap > 0 {
        (s.current_lap - 1).max(0)
    } else {
        0
    };
    let standing = s
        .standings
        .iter()
        .take(s.standing_count.max(0) as usize)
        .find(|row| row.race_num == focus)
        .map(|row| row.num_laps);
    match standing {
        Some(n) => standing_num_laps(s, focus, n),
        None => from_plugin,
    }
}

pub(crate) fn timed_clock_live(s: &Snapshot) -> bool {
    !is_lap_race(s) && !overtime_active(s) && s.session_time_ms > 1000
}

pub(crate) fn lap_race_racing(s: &Snapshot) -> bool {
    is_lap_race(s)
        && IN_GATE.load(Ordering::Relaxed) == 0
        && (LAP_GREEN.load(Ordering::Relaxed) == 1 || laps_done(s) > 0 || s.current_lap > 1)
}

/// The leader has taken the finish, so the race is over and everyone still out is
/// waved off on their next crossing.
pub(crate) fn leader_finished(s: &Snapshot) -> bool {
    if lap_race_racing(s) {
        let n = s.session_laps.max(1);
        let lead = leader_num_laps(s);
        // A leader miles past the distance means this is not really an `n`-lap race —
        // a timed set glitching to `is_lap_race` puts extras (2) up against real lap
        // counts (6). Nobody runs more than a cool-down lap after the finish.
        return lead >= n && lead <= n + 2;
    }
    if overtime_active(s) && extras_started(s) {
        let base = overtime_base(s);
        // The leader's own extras also start one crossing after expiry.
        return base > 0 && leader_num_laps(s) >= base + 1 + extra_laps(s).max(1);
    }
    false
}

fn note_leader_finish(s: &Snapshot) {
    note_lapped_by_leader(s);
    if LEADER_FIN_LOCAL_BASE.load(Ordering::Relaxed) >= 0 {
        return;
    }
    if !leader_finished(s) {
        return;
    }
    LEADER_FIN_LOCAL_BASE.store(focus_num_laps(s).max(0), Ordering::Relaxed);
}

/// The lap total you will actually be scored over. Normally the race distance, but once
/// the leader takes the finish you are waved off on your next crossing, so a lapped rider
/// runs fewer laps than the moto: their counter reads `4 / 4`, not `4 / 5`. Never longer
/// than the distance, so a winner is not credited an extra lap for their own finish.
pub(crate) fn effective_race_laps(s: &Snapshot) -> i32 {
    let n = s.session_laps.max(1);
    note_leader_finish(s);
    let base = LEADER_FIN_LOCAL_BASE.load(Ordering::Relaxed);
    if base < 0 {
        return n;
    }
    (base + 1).clamp(1, n)
}

/// Extras you will actually be scored over, on the same rule as `effective_race_laps`.
/// Counted against `OVERTIME_LOCAL_BASE`. `base - local0` is extras already started
/// when the leader finished; the uncounted lap (`0`) still clamps to one extra so a
/// rider waved off on `0/2` reads `1/1`, not `2/2`. Being a lap down on `0/1` does
/// not latch this — only `leader_finished` does.
pub(crate) fn effective_extra_laps(s: &Snapshot) -> i32 {
    let n = extra_laps(s).max(1);
    note_leader_finish(s);
    let base = LEADER_FIN_LOCAL_BASE.load(Ordering::Relaxed);
    let local0 = OVERTIME_LOCAL_BASE.load(Ordering::Relaxed);
    if base < 0 || local0 < 0 || !extras_started(s) {
        return n;
    }
    (base - local0).clamp(1, n)
}

/// Your race is done: you covered the distance, or you crossed the line after the
/// leader finished while still a lap down.
pub(crate) fn race_over_for_me(s: &Snapshot) -> bool {
    note_leader_finish(s);
    let base = LEADER_FIN_LOCAL_BASE.load(Ordering::Relaxed);
    base >= 0 && focus_num_laps(s) > base
}

pub(crate) fn i_finished(s: &Snapshot) -> bool {
    if prestart(s) || timed_clock_live(s) {
        return false;
    }
    if race_over_for_me(s) {
        return true;
    }
    if lap_race_racing(s) {
        return laps_done(s) >= s.session_laps;
    }
    let n = extra_laps(s);
    if n <= 0 {
        return false;
    }
    if s.session_length > 0 && !overtime_active(s) {
        return false;
    }
    if overtime_active(s) {
        return extras_started(s) && local_overtime_done(s) >= extra_laps(s).max(1);
    }
    laps_done(s) >= n
}

pub(crate) fn laps_left(s: &Snapshot) -> Option<i32> {
    // Checked before `prestart` so stopping after the finish cannot drop the checkered.
    if prestart(s) || timed_clock_live(s) {
        return None;
    }
    if lap_race_racing(s) {
        // Your published distance, not a wave-off. Checkered waits until you
        // complete these laps, even if the leader has already taken the flag.
        return Some((s.session_laps.max(1) - laps_done(s)).max(0));
    }
    if overtime_active(s) {
        note_overtime_base(s);
        // Until the leader crosses after time expiry you are still on the timed lap,
        // so how many laps remain is not decided yet.
        if !extras_started(s) {
            return None;
        }
        // Count down to the extras you will run. Deriving this from
        // `local_overtime_done` instead would lose a lap: its `max(0)` clamp reports 0
        // both on the lap that does not count and on your first extra.
        // The leader finishing does not cut a lap you have already started.
        // Timed +1 still on `0/1` is the checkered once a real pass happened and
        // the leader has finished. Timed +2 lapped before `2/2` ends on the line
        // that would start that extra.
        let extras = extra_laps(s).max(1);
        let taken = local_overtime_taken(s);
        note_plus_one_wave_off(s);
        note_plus_two_wave_off(s);
        if PLUS_ONE_WAVE_OFF.load(Ordering::Relaxed) == 1 {
            return Some((1 - raw_overtime_taken(s)).max(0));
        }
        if PLUS_TWO_WAVE_OFF.load(Ordering::Relaxed) == 1 && raw_overtime_taken(s) > 0 {
            return Some((2 - raw_overtime_taken(s)).max(0));
        }
        return Some((extras + 1 - taken).max(0));
    }
    None
}

/// Your lap count while there were still laps to run. Tick once per frame.
pub(crate) fn note_laps_to_run(s: &Snapshot, left: Option<i32>) {
    if left.is_some_and(|n| n > 0) {
        LAPS_TO_RUN_AT.store(focus_num_laps(s), Ordering::Relaxed);
    }
    if let Some(n) = left {
        if n >= 2 && !remaining_laps_collapsed(n) {
            LAST_LAPS_REMAINING.store(n, Ordering::Relaxed);
        }
    }
}

fn remaining_laps_collapsed(left: i32) -> bool {
    let prev = LAST_LAPS_REMAINING.load(Ordering::Relaxed);
    prev >= 0 && left < prev && prev - left >= 2
}

/// The leader actually went past, or classification already has a lap on you.
/// A leader finish with a lead of 2 on `0/1` is their extra crossing, not this.
fn passed_a_lap(s: &Snapshot) -> bool {
    let _ = lapped(s);
    LAPPED_LATCH.load(Ordering::Relaxed) == 1
        || focus_standing(s).is_some_and(|row| row.gap_laps >= 1)
}

/// Timed +1: the leader finished the lap that put you a lap down before you started
/// the extra. Latch it — after you cross, their lead shrinks by one and `lapped` can
/// go quiet, but this line is still the finish.
fn note_plus_one_wave_off(s: &Snapshot) {
    if PLUS_ONE_WAVE_OFF.load(Ordering::Relaxed) == 1 || extra_laps(s) != 1 {
        return;
    }
    note_leader_finish(s);
    let fin = LEADER_FIN_LOCAL_BASE.load(Ordering::Relaxed);
    let local0 = OVERTIME_LOCAL_BASE.load(Ordering::Relaxed);
    if fin < 0 || local0 < 0 || fin > local0 {
        return;
    }
    if passed_a_lap(s) {
        PLUS_ONE_WAVE_OFF.store(1, Ordering::Relaxed);
    }
}

/// Timed +2: a real lap-down on `0/2` or `1/2`. The line that would start `2/2` is
/// the checkered. Stays set after the lead shrinks so that crossing still finishes.
fn note_plus_two_wave_off(s: &Snapshot) {
    if PLUS_TWO_WAVE_OFF.load(Ordering::Relaxed) == 1 || extra_laps(s) != 2 {
        return;
    }
    if raw_overtime_taken(s) > 1 {
        return;
    }
    if passed_a_lap(s) {
        PLUS_TWO_WAVE_OFF.store(1, Ordering::Relaxed);
    }
}

/// White is the last-lap wave for a rider still running the full distance.
/// A first-lap `session_laps` drop that collapses remaining laps is not a last lap.
/// A +1 wave-off (`PLUS_ONE_WAVE_OFF`) or a +2 wave-off onto the cancelled `2/2`
/// (`PLUS_TWO_WAVE_OFF`) is checkered on the run-in only — no white flash mid-lap.
pub(crate) fn skip_last_lap_white(s: &Snapshot, left: Option<i32>) -> bool {
    let Some(n) = left else {
        return false;
    };
    if n <= 1
        && (PLUS_ONE_WAVE_OFF.load(Ordering::Relaxed) == 1
            || PLUS_TWO_WAVE_OFF.load(Ordering::Relaxed) == 1)
    {
        return true;
    }
    if remaining_laps_collapsed(n) {
        return true;
    }
    n <= 1 && laps_done(s) <= 0 && is_lap_race(s) && s.session_laps > 1
}

/// You have completed a lap since we last knew you had laps to run, so reaching zero
/// really was a line crossing. The session fields glitch often enough that a lone frame
/// claiming no laps remain must not wave you off.
pub(crate) fn finish_earned(s: &Snapshot) -> bool {
    let at = LAPS_TO_RUN_AT.load(Ordering::Relaxed);
    at >= 0 && focus_num_laps(s) > at
}

pub(crate) fn race_progress_text(s: &Snapshot) -> String {
    race_progress_from_store_or_snap(s)
}

/// Prefer the last `RaceStore::tick` banner so draw does not mutate the clock again.
fn race_progress_from_store_or_snap(s: &Snapshot) -> String {
    RaceStore::with(|store| {
        let b = &store.clock.banner.1;
        if b.is_empty() || b == "--:--" {
            session_banner(s).1
        } else {
            b.clone()
        }
    })
}

/// Laps still to run, counting the one you are on, so the final lap reads `1`.
/// Same source as the flags — do not re-derive it from the banner text.
pub(crate) fn race_laps_left_text(s: &Snapshot) -> String {
    laps_left(s)
        .map(|n| format!("{n}"))
        .unwrap_or_else(|| "--".into())
}

pub(crate) fn session_banner(s: &Snapshot) -> (char, String) {
    format_session_banner(s, session_remain_ms(s))
}

fn format_session_banner(s: &Snapshot, remain: Option<i32>) -> (char, String) {
    if is_lap_race(s) {
        if let Some(remain) = remain {
            if remain > 0 && remain <= 180_000 {
                return ('\u{f2f2}', format_countdown(remain));
            }
        }
        OVERTIME_BASE_LAP.store(-1, Ordering::Relaxed);
        OVERTIME_LOCAL_BASE.store(-1, Ordering::Relaxed);
        OVERTIME_LINE_ARM.store(0, Ordering::Relaxed);
        return ('\u{f11e}', lap_race_text(s));
    }
    if let Some(remain) = remain {
        if remain > 1000 && SESSION_EXPIRED.load(Ordering::Relaxed) == 0 {
            OVERTIME_BASE_LAP.store(-1, Ordering::Relaxed);
            OVERTIME_LOCAL_BASE.store(-1, Ordering::Relaxed);
            OVERTIME_LINE_ARM.store(0, Ordering::Relaxed);
        }
        if overtime_active(s) {
            return ('\u{f11e}', overtime_lap_text(s));
        }
        // Timed race over, extras not published yet. Don't look like warmup ended.
        // Kind 5 with leaked extras still blanks — that is not a race awaiting +N.
        if (s.session_laps <= 0 || is_warmup(s))
            && (remain <= 0 || SESSION_EXPIRED.load(Ordering::Relaxed) == 1)
        {
            if !is_warmup(s) && timed_race_awaiting_extras(s) {
                let _ = overtime_base(s);
                return ('\u{f2f2}', "00:00".into());
            }
            return ('\u{f2f2}', String::new());
        }
        ('\u{f2f2}', timed_clock_text(s, remain))
    } else if overtime_active(s) {
        // Only show extras text when the timed clock has actually expired.
        OVERTIME_BASE_LAP.store(-1, Ordering::Relaxed);
        OVERTIME_LOCAL_BASE.store(-1, Ordering::Relaxed);
        OVERTIME_LINE_ARM.store(0, Ordering::Relaxed);
        ('\u{f11e}', overtime_lap_text(s))
    } else if s.session_laps > 0 {
        OVERTIME_BASE_LAP.store(-1, Ordering::Relaxed);
        OVERTIME_LOCAL_BASE.store(-1, Ordering::Relaxed);
        OVERTIME_LINE_ARM.store(0, Ordering::Relaxed);
        ('\u{f11e}', format!("{} / {}", race_lap(s), s.session_laps))
    } else if SESSION_EXPIRED.load(Ordering::Relaxed) == 1 {
        ('\u{f2f2}', String::new())
    } else {
        OVERTIME_BASE_LAP.store(-1, Ordering::Relaxed);
        OVERTIME_LOCAL_BASE.store(-1, Ordering::Relaxed);
        OVERTIME_LINE_ARM.store(0, Ordering::Relaxed);
        ('\u{f2f2}', format_session_clock(s.session_time_ms))
    }
}

/// Compact session-clock fields for a JSONL trace the overlay writes while you ride.
pub struct ClockSample {
    pub seq: u32,
    pub session_length: i32,
    pub session_laps: i32,
    pub session_time_ms: i32,
    pub current_lap: i32,
    pub current_lap_ms: i32,
    pub last_lap_ms: i32,
    pub local_speed: f32,
    pub local_track_pos: f32,
    pub on_track: i32,
    pub local_laps: i32,
    pub leader_laps: i32,
    pub standing_count: i32,
    pub dash: String,
    pub remain_ms: i32,
    pub mode: i32,
    pub gate: i32,
    pub armed: i32,
    pub expired: i32,
    pub saw: i32,
    pub locked_len: i32,
    pub ot_local: i32,
    pub ot_lead: i32,
    /// Count-only white/checkered: 0 none, 1 white, 2 checkered. Stays white for
    /// the whole last lap. The dash wave is `shown`.
    pub flag: i32,
    /// Flag the plaque draws: 0 none, 1 white, 2 checkered, 3 yellow, 4 blue, 5 red.
    pub shown: i32,
    pub laps_left: i32,
}

pub fn clock_sample(s: &Snapshot) -> ClockSample {
    let store = RaceStore::tick(s);
    let remain = store.clock.remain_ms;
    let flag = match store.clock.flag {
        RaceFlag::None => 0,
        RaceFlag::White => 1,
        RaceFlag::Checkered => 2,
    };
    let dash = store.clock.banner.1;
    ClockSample {
        seq: s.seq,
        session_length: s.session_length,
        session_laps: s.session_laps,
        session_time_ms: s.session_time_ms,
        current_lap: s.current_lap,
        current_lap_ms: s.current_lap_ms,
        last_lap_ms: s.last_lap_ms,
        local_speed: s.local_speed,
        local_track_pos: s.local_track_pos,
        on_track: s.on_track,
        local_laps: focus_num_laps(s),
        leader_laps: leader_num_laps(s),
        standing_count: s.standing_count,
        dash,
        remain_ms: remain.unwrap_or(-1),
        mode: SESSION_CLOCK_MODE.load(Ordering::Relaxed),
        gate: IN_GATE.load(Ordering::Relaxed),
        armed: RACE_ARMED.load(Ordering::Relaxed),
        expired: SESSION_EXPIRED.load(Ordering::Relaxed),
        saw: SAW_SESSION_TIME.load(Ordering::Relaxed),
        locked_len: LOCKED_SESSION_LEN.load(Ordering::Relaxed),
        ot_local: OVERTIME_LOCAL_BASE.load(Ordering::Relaxed),
        ot_lead: OVERTIME_BASE_LAP.load(Ordering::Relaxed),
        flag,
        shown: store.race_flag,
        laps_left: laps_left(s).unwrap_or(-1),
    }
}

pub(crate) fn session_best_ms(s: &Snapshot) -> i32 {
    s.standings
        .iter()
        .take(s.standing_count.max(0) as usize)
        .map(|row| row.best_lap_ms)
        .filter(|ms| *ms > 0)
        .min()
        .unwrap_or(0)
}

pub(crate) fn class_position(s: &Snapshot) -> i32 {
    let Some(st) = focus_standing(s) else {
        return 0;
    };
    let live = live_position(st.race_num);
    let overall = if live > 0 { live } else { st.position.max(0) };
    let cat = bytes_as_text(&st.category);
    if cat.is_empty() {
        return overall;
    }
    ordered_standings(s)
        .iter()
        .filter(|row| bytes_as_text(&row.category) == cat)
        .position(|row| row.race_num == st.race_num)
        .map(|i| i as i32 + 1)
        .unwrap_or(overall)
}

pub(crate) fn focus_standing(s: &Snapshot) -> Option<&Standing> {
    let focus = if s.focus_race_num > 0 {
        s.focus_race_num
    } else {
        s.local_race_num
    };
    standing_of(s, focus)
}

/// A lap or more behind the leader. Classification `gap_laps` only moves at the line, so
/// this also latches the moment the lap-up leader goes past you on track. Gated on a
/// running race because a practice or warmup field has no leader to be a lap behind.
/// The game often leaves `gap_laps` at 0 when you are waved off a lap short, so
/// `leader_finished` with a lap deficit also counts.
pub(crate) fn lapped(s: &Snapshot) -> bool {
    if s.on_track == 0 || is_warmup(s) || prestart(s) {
        return false;
    }
    note_lapped_by_leader(s);
    if LAPPED_LATCH.load(Ordering::Relaxed) == 1 {
        return true;
    }
    if focus_standing(s).is_some_and(|st| st.gap_laps >= 1) {
        return true;
    }
    // Two or more completed laps behind after the finish: a single lap is you still
    // out on the last lap the leader just took.
    leader_finished(s) && leader_lap_lead(s) >= 2
}

pub(crate) fn interval_text_from_row(row: &RaceRow) -> String {
    if row.standing.position <= 1 {
        return "---".into();
    }
    if row.interval_laps != 0 {
        format_gap(0, row.interval_laps)
    } else {
        format_gap(row.interval_ms, 0)
    }
}

/// Live seconds from a wrapped lap fraction (Relative's Gap math).
fn live_track_gap_ms(s: &Snapshot, wrap_frac: f32) -> Option<i32> {
    if s.track_length <= 10.0 {
        return None;
    }
    let meters = wrap_frac.abs() * s.track_length;
    let speed = s.local_speed.max(4.0);
    Some((meters / speed * 1000.0).round() as i32)
}

fn format_live_wrap(s: &Snapshot, wrap: f32) -> String {
    match live_track_gap_ms(s, wrap) {
        Some(ms) if ms <= 0 => "0.000".into(),
        Some(ms) => format_gap(ms, 0).trim_start_matches('+').to_string(),
        None => "---".into(),
    }
}

/// Distance along the lap from `from` to `to` in race direction, 0..1. Not the
/// shortest wrap: a rider a long way behind must not flip to looking close.
fn wrap_along(s: &Snapshot, from: i32, to: i32) -> Option<f32> {
    let (Some(pa), Some(pb)) = (rider_lap_pos(s, from), rider_lap_pos(s, to)) else {
        return None;
    };
    Some((pb - pa).rem_euclid(1.0))
}

fn rider_laps_on(s: &Snapshot, field: &RaceField, race_num: i32) -> i32 {
    if let Some(r) = field.row_by_num(race_num) {
        return r.current_lap;
    }
    standing_of(s, race_num)
        .map(|st| rider_current_lap(s, race_num, st.num_laps))
        .unwrap_or(0)
}

/// Place-neighbor gap: live laps when they differ, else seconds along the track
/// toward that rider. Shortest wrap would treat a crashed last-place rider as
/// nearby every time you come around to pass them.
fn gap_to_num(s: &Snapshot, field: &RaceField, me: i32, other: i32, behind: bool) -> String {
    let d = rider_laps_on(s, field, me) - rider_laps_on(s, field, other);
    if behind && d >= 1 {
        return format!("-{d}L");
    }
    if !behind && d <= -1 {
        return format!("{}L", -d);
    }
    let (from, to) = if behind { (other, me) } else { (me, other) };
    match wrap_along(s, from, to) {
        Some(w) => format_live_wrap(s, w),
        None => "---".into(),
    }
}

fn classified_neighbor<'a>(s: &'a Snapshot, position: i32) -> Option<&'a Standing> {
    if position <= 0 {
        return None;
    }
    let n = s.standing_count.max(0) as usize;
    s.standings[..n].iter().find(|st| st.position == position)
}

/// Gap to the live-order leader (P1). Time is a live running gap.
pub(crate) fn gap_leader_text(s: &Snapshot, field: &RaceField, row: &Standing) -> String {
    if let Some(leader) = field.rows.first() {
        if leader.standing.race_num == row.race_num {
            return "---".into();
        }
        return gap_to_num(s, field, row.race_num, leader.standing.race_num, false);
    }
    if row.position <= 1 {
        return "---".into();
    }
    format_gap(row.gap_ms, row.gap_laps)
        .trim_start_matches('+')
        .to_string()
}

/// Gap to the rider one live-order place ahead (P−1). Time is a live running gap.
pub(crate) fn gap_ahead_text(s: &Snapshot, field: &RaceField, row: &Standing) -> String {
    if let Some(i) = field
        .rows
        .iter()
        .position(|r| r.standing.race_num == row.race_num)
    {
        if i == 0 {
            return "---".into();
        }
        return gap_to_num(
            s,
            field,
            row.race_num,
            field.rows[i - 1].standing.race_num,
            false,
        );
    }
    if row.position <= 1 {
        return "---".into();
    }
    classified_neighbor(s, row.position - 1)
        .map(|ahead| gap_to_num(s, field, row.race_num, ahead.race_num, false))
        .unwrap_or_else(|| "---".into())
}

/// Gap to the rider one live-order place behind (P+1). Time is a live running gap.
pub(crate) fn gap_behind_text(s: &Snapshot, field: &RaceField, row: &Standing) -> String {
    if let Some(i) = field
        .rows
        .iter()
        .position(|r| r.standing.race_num == row.race_num)
    {
        return match field.rows.get(i + 1) {
            Some(behind) => gap_to_num(s, field, row.race_num, behind.standing.race_num, true),
            None => "---".into(),
        };
    }
    classified_neighbor(s, row.position + 1)
        .map(|behind| gap_to_num(s, field, row.race_num, behind.race_num, true))
        .unwrap_or_else(|| "---".into())
}

#[cfg(test)]
#[path = "tests/session_replay.rs"]
mod session_replay_tests;
