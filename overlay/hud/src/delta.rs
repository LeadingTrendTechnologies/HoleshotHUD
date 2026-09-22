//! Time vs your best at this point on the lap.
//!
//! Live elapsed is a wall-clock lap anchor reset at S/F / lap-complete and
//! resynced at official S1/S2. Plugin `_fTime` is only used to detect finishes.
//! Best position→time tapes still persist via `track_pb`.

use crate::race_store::norm_lap_pos;
use crate::shm::{cstr, Snapshot};
use crate::track_pb::{self, BINS};
use std::sync::Mutex;
use std::time::Instant;

const MIN_LAP_MS: i32 = 20_000;
const MAX_LAP_MS: i32 = 15 * 60 * 1000;
const MIN_COVERAGE: f32 = 0.55;
const CUT_METERS: f32 = 100.0;
const MAX_SPLINE_MPS: f32 = 62.0;
const PACE_SLACK_MS: i32 = 400;
const JUMP_MIN_MS: i32 = 900;
const JUMP_MAX_MS: i32 = 3_000;
const JUMP_MAX_M: f32 = 200.0;
const DEFAULT_TRACK_M: f32 = 1500.0;
const SMOOTH_TAU: f32 = 0.42;
const POS_TAU: f32 = 0.08;
const NEW_BEST_HOLD_S: f32 = 8.0;
/// Official split/lap freeze hold. Off for Hair for now.
const DEFAULT_FREEZE_MS: i32 = 0;
/// Track-pos jump larger than this counts as an S/F wrap.
const WRAP_THRESHOLD: f32 = 0.5;
/// Bar saturates at this |delta|.
pub const BAR_RANGE_MS: i32 = 2_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeltaView {
    pub ready: bool,
    pub recording: bool,
    pub has_delta: bool,
    pub delta_ms: i32,
    pub ref_lap_ms: i32,
    pub last_lap_ms: i32,
    pub cover: u8,
    pub new_best: bool,
}

impl DeltaView {
    pub const fn empty() -> Self {
        Self {
            ready: false,
            recording: false,
            has_delta: false,
            delta_ms: 0,
            ref_lap_ms: 0,
            last_lap_ms: 0,
            cover: 0,
            new_best: false,
        }
    }
}

/// Wall-clock lap timer (anchor + accumulated official split time).
#[derive(Clone, Copy, Debug)]
pub(crate) struct LapAnchor {
    pub(crate) wall: Option<Instant>,
    pub(crate) accum_ms: i32,
    pub(crate) valid: bool,
}

impl LapAnchor {
    const fn new() -> Self {
        Self {
            wall: None,
            accum_ms: 0,
            valid: false,
        }
    }

    fn reset(&mut self) {
        *self = Self::new();
    }

    fn set(&mut self, accum: i32, now: Instant) {
        self.wall = Some(now);
        self.accum_ms = accum.max(0);
        self.valid = true;
    }

    fn elapsed_ms(&self, now: Instant) -> i32 {
        if !self.valid {
            return 0;
        }
        let wall = self.wall.unwrap_or(now);
        let delta = now.saturating_duration_since(wall).as_millis() as i32;
        self.accum_ms.saturating_add(delta.max(0))
    }
}

#[derive(Clone)]
struct LapTape {
    bins: [i32; BINS],
    filled: usize,
    min_pos: f32,
    max_pos: f32,
    last_pos: f32,
    last_ms: i32,
    last_move_pos: f32,
    last_move_ms: i32,
    dirty: bool,
    track_m: f32,
}

impl LapTape {
    const fn new() -> Self {
        Self {
            bins: [0; BINS],
            filled: 0,
            min_pos: 1.0,
            max_pos: 0.0,
            last_pos: -1.0,
            last_ms: 0,
            last_move_pos: -1.0,
            last_move_ms: 0,
            dirty: false,
            track_m: DEFAULT_TRACK_M,
        }
    }

    fn from_saved(bins: [i32; BINS]) -> Self {
        let mut t = Self::new();
        t.bins = bins;
        for (i, &ms) in bins.iter().enumerate() {
            if ms > 0 {
                t.filled += 1;
                let pos = i as f32 / BINS as f32;
                t.min_pos = t.min_pos.min(pos);
                t.max_pos = t.max_pos.max(pos);
            }
        }
        t
    }

    fn cover(&self) -> u8 {
        ((self.filled as f32 / BINS as f32) * 100.0).round() as u8
    }

    fn recorded_ms(&self) -> i32 {
        self.bins
            .iter()
            .copied()
            .max()
            .unwrap_or(0)
            .max(self.last_ms)
    }

    #[cfg(test)]
    fn push(&mut self, pos: f32, ms: i32) {
        self.push_at(pos, ms, self.track_m);
    }

    #[cfg(test)]
    fn push_at(&mut self, pos: f32, ms: i32, track_m: f32) {
        self.push_at_opt(pos, ms, track_m, false);
    }

    fn push_at_opt(&mut self, pos: f32, ms: i32, track_m: f32, ignore_cut: bool) {
        let pos = if pos >= 1.0 { 0.999_999 } else { pos };
        if !(0.0..1.0).contains(&pos) || ms <= 0 {
            return;
        }
        self.track_m = if track_m > 80.0 {
            track_m
        } else {
            DEFAULT_TRACK_M
        };
        if self.last_pos >= 0.0 {
            let d = pos - self.last_pos;
            if d < -WRAP_THRESHOLD {
                self.last_pos = pos;
                self.last_ms = ms;
                return;
            }
            if d < -0.02 {
                return;
            }
        }
        if self.last_ms > 0 && ms + 250 < self.last_ms {
            if self.last_ms < 8_000 {
                let track_m = self.track_m;
                *self = Self::new();
                self.track_m = track_m;
            } else {
                self.last_ms = ms;
                self.last_pos = pos;
                self.last_move_pos = -1.0;
                return;
            }
        }
        if ignore_cut {
            self.last_move_pos = pos;
            self.last_move_ms = ms;
        } else {
            self.note_progress(pos, ms);
        }
        let i = ((pos * BINS as f32) as usize).min(BINS - 1);
        // Fill skipped bins (hitch / sparse samples) so coverage stays high
        // without requiring hundreds of unique ticks.
        if self.last_pos >= 0.0 {
            let prev_i = ((self.last_pos * BINS as f32) as usize).min(BINS - 1);
            if i > prev_i + 1 && self.last_ms > 0 {
                let span = (i - prev_i) as f32;
                for k in (prev_i + 1)..i {
                    let t = (k - prev_i) as f32 / span;
                    let lerp = self.last_ms + ((ms - self.last_ms) as f32 * t).round() as i32;
                    if self.bins[k] == 0 {
                        self.filled += 1;
                    }
                    self.bins[k] = lerp;
                }
            }
        }
        // Overwrite the bin with current elapsed each tick.
        if self.bins[i] == 0 {
            self.filled += 1;
        }
        self.bins[i] = ms;
        self.min_pos = self.min_pos.min(pos);
        self.max_pos = self.max_pos.max(pos);
        self.last_pos = pos;
        self.last_ms = ms;
    }

    fn note_progress(&mut self, pos: f32, ms: i32) {
        if self.last_move_pos < 0.0 {
            self.last_move_pos = pos;
            self.last_move_ms = ms;
            return;
        }
        let d_pos = pos - self.last_move_pos;
        if d_pos <= 0.0 {
            return;
        }
        let meters = d_pos * self.track_m;
        if meters < CUT_METERS {
            return;
        }
        if too_fast(meters, ms - self.last_move_ms) {
            self.dirty = true;
        }
        self.last_move_pos = pos;
        self.last_move_ms = ms;
    }

    fn time_at(&self, pos: f32) -> Option<i32> {
        track_pb::time_at(&self.bins, pos)
    }

    fn decent(&self, lap_ms: i32) -> bool {
        let cover = self.filled as f32 / BINS as f32;
        lap_ms >= MIN_LAP_MS
            && lap_ms <= MAX_LAP_MS
            && cover >= MIN_COVERAGE
            && (self.max_pos - self.min_pos) >= 0.50
            && !self.dirty
            && self.spline_ok()
    }

    fn spline_ok(&self) -> bool {
        let mut prev: Option<(usize, i32)> = None;
        let mut saw_line = false;
        for (i, &t) in self.bins.iter().enumerate() {
            if t <= 0 {
                continue;
            }
            if let Some((j, t0)) = prev {
                let d_pos = (i - j) as f32 / BINS as f32;
                let meters = d_pos * self.track_m;
                let d_ms = t - t0;
                if d_ms < -150 {
                    if saw_line {
                        if t < 8_000 && t0 > 8_000 {
                            continue;
                        }
                        return false;
                    }
                    saw_line = true;
                } else if too_fast(meters, d_ms) {
                    return false;
                }
            }
            prev = Some((i, t));
        }
        true
    }
}

pub(crate) struct DeltaEngine {
    track_key: String,
    bike_key: String,
    current: LapTape,
    reference: Option<LapTape>,
    ref_lap_ms: i32,
    session: Option<LapTape>,
    session_lap_ms: i32,
    last_lap_num: i32,
    last_last_lap_ms: i32,
    shown_last_ms: i32,
    last_cur_ms: i32,
    last_seen_pos: f32,
    lookup_pos: f32,
    last_tick: Option<Instant>,
    smooth_ms: f32,
    smooth_init: bool,
    session_smooth_ms: f32,
    session_smooth_init: bool,
    armed: bool,
    observed_lap_start: bool,
    crashed_this_lap: bool,
    remount_at: Option<Instant>,
    pub(crate) anchor: LapAnchor,
    last_s1: i32,
    last_s2: i32,
    pub(crate)     last_wall_ms: i32,
    freeze_at: Option<Instant>,
    frozen_gap: i32,
    new_best_at: Option<Instant>,
    session_new_best_at: Option<Instant>,
    last_view: DeltaView,
    session_view: DeltaView,
}

impl DeltaEngine {
    const fn new() -> Self {
        Self {
            track_key: String::new(),
            bike_key: String::new(),
            current: LapTape::new(),
            reference: None,
            ref_lap_ms: 0,
            session: None,
            session_lap_ms: 0,
            last_lap_num: 0,
            last_last_lap_ms: 0,
            shown_last_ms: 0,
            last_cur_ms: 0,
            last_seen_pos: -1.0,
            lookup_pos: -1.0,
            last_tick: None,
            smooth_ms: 0.0,
            smooth_init: false,
            session_smooth_ms: 0.0,
            session_smooth_init: false,
            armed: false,
            observed_lap_start: false,
            crashed_this_lap: false,
            remount_at: None,
            anchor: LapAnchor::new(),
            last_s1: 0,
            last_s2: 0,
            last_wall_ms: 0,
            freeze_at: None,
            frozen_gap: 0,
            new_best_at: None,
            session_new_best_at: None,
            last_view: DeltaView::empty(),
            session_view: DeltaView::empty(),
        }
    }

    fn reset_track(&mut self, track: String, bike: String) {
        *self = Self::new();
        self.track_key = track.clone();
        self.bike_key = bike.clone();
        if bike.is_empty() {
            let pb = track_pb::bind_exact(&track, "");
            if pb.has_tape() {
                self.reference = Some(LapTape::from_saved(pb.bins));
                self.ref_lap_ms = pb.lap_ms;
            }
            return;
        }
        let pb = track_pb::bind(&track, &bike);
        if pb.has_tape() {
            self.reference = Some(LapTape::from_saved(pb.bins));
            self.ref_lap_ms = pb.lap_ms;
        }
    }

    fn adopt_bike(&mut self, bike: String) {
        self.bike_key = bike.clone();
        if let Some(tape) = self.reference.as_ref() {
            if self.ref_lap_ms > 0 && !self.track_key.is_empty() {
                track_pb::commit_tape(&self.track_key, &bike, self.ref_lap_ms, tape.bins);
            }
        }
        let pb = track_pb::bind(&self.track_key, &bike);
        if pb.has_tape() && (self.ref_lap_ms <= 0 || pb.lap_ms < self.ref_lap_ms) {
            self.reference = Some(LapTape::from_saved(pb.bins));
            self.ref_lap_ms = pb.lap_ms;
        }
    }

    /// Soft reset current lap (pit / spectate); keep saved best.
    fn soft_reset_current(&mut self) {
        self.anchor.reset();
        self.current = LapTape::new();
        self.armed = false;
        self.observed_lap_start = false;
        self.last_s1 = 0;
        self.last_s2 = 0;
        self.last_wall_ms = 0;
        self.freeze_at = None;
        self.smooth_init = false;
        self.session_smooth_init = false;
        self.lookup_pos = -1.0;
    }

    pub fn tick(&mut self, s: &Snapshot) -> DeltaView {
        self.tick_at(s, Instant::now())
    }

    pub fn tick_at(&mut self, s: &Snapshot, now: Instant) -> DeltaView {
        if s.has_telemetry == 0 {
            return self.last_view;
        }
        let track = cstr(&s.track_name).to_string();
        let bike_now = track_pb::bike_class(&s.local_bike());
        if !track.is_empty() {
            let bike = if !bike_now.is_empty() {
                bike_now
            } else {
                self.bike_key.clone()
            };
            if track != self.track_key || bike != self.bike_key {
                if track == self.track_key && self.bike_key.is_empty() && !bike.is_empty() {
                    self.adopt_bike(bike);
                } else {
                    self.reset_track(track, bike);
                }
            }
        }

        let pos = lap_pos(s);
        let cur_ms = s.current_lap_ms;
        let lap_num = s.current_lap;

        if s.local_crashed != 0 && self.armed {
            self.crashed_this_lap = true;
            self.remount_at = Some(now);
        }
        let remounting = self
            .remount_at
            .is_some_and(|t| now.saturating_duration_since(t).as_secs_f32() < 2.0);

        let wrap = sf_wrap(self.last_seen_pos, pos);
        let clock_drop = self.last_cur_ms > 8_000 && cur_ms + 2_500 < self.last_cur_ms;
        let new_last = s.last_lap_ms > 0 && s.last_lap_ms != self.last_last_lap_ms;
        let lap_up = lap_num > self.last_lap_num && self.last_lap_num > 0;
        let finish_wrap = wrap_is_finish(self, pos, cur_ms);
        let start_flying = !self.armed && wrap && pos < 0.20;
        let start_flying_clock = !self.armed
            && clock_drop
            && cur_ms < 4_000
            && pos >= 0.18
            && self.last_cur_ms < 180_000;
        let crossed_sf = start_flying || finish_wrap || new_last || lap_up;

        let dt = tick_dt(&mut self.last_tick, now);
        if wrap || clock_drop {
            self.smooth_init = false;
            self.session_smooth_init = false;
            self.lookup_pos = pos;
        }

        let mut just_armed = false;
        // Reset to pits: clock dies mid-lap without a finish. A full flying lap
        // that drops the clock at the line is a finish (see lap_ended), not a reset.
        if self.armed
            && clock_drop
            && !wrap
            && !new_last
            && !lap_up
            && !finish_wrap
            && self.current.filled > 0
            && self.current.recorded_ms() < MIN_LAP_MS
        {
            self.soft_reset_current();
        }
        // Wrap starts/resets the wall-clock anchor (not plugin `_fTime`).
        if wrap && (!self.anchor.valid || lap_up || !self.armed) {
            self.anchor.set(0, now);
            self.observed_lap_start = true;
            just_armed = true;
            if !self.armed {
                self.armed = true;
                self.current = LapTape::new();
                self.last_s1 = 0;
                self.last_s2 = 0;
            }
        }

        let ended = lap_ended(self, s, pos);
        if ended {
            let done = completed_lap_ms(self, s);
            try_commit(self, done, now);
            if done > 0 {
                self.shown_last_ms = done;
            }
            if DEFAULT_FREEZE_MS > 0 && done > 0 && self.ref_lap_ms > 0 {
                self.frozen_gap = done - self.ref_lap_ms;
                // Real clock for hold length — tick_at may use a synthetic Instant.
                self.freeze_at = Some(Instant::now());
            }
            self.current = LapTape::new();
            self.smooth_init = false;
            self.session_smooth_init = false;
            self.crashed_this_lap = false;
            // Lap-complete always arms the next flying lap with a fresh wall clock.
            self.anchor.set(0, now);
            self.observed_lap_start = true;
            self.armed = crossed_sf || self.armed;
            just_armed = true;
            self.last_s1 = 0;
            self.last_s2 = 0;
            self.last_wall_ms = 0;
        } else if !self.armed && (crossed_sf || start_flying_clock) {
            self.armed = true;
            self.observed_lap_start = true;
            self.current = LapTape::new();
            self.anchor.set(0, now);
            just_armed = true;
            self.last_s1 = 0;
            self.last_s2 = 0;
            self.last_wall_ms = 0;
        }

        if !ended {
            self.note_splits(s, now);
        }

        let wall_ms = if self.anchor.valid {
            self.anchor.elapsed_ms(now)
        } else {
            0
        };
        self.last_wall_ms = wall_ms;

        let moving = s.on_track != 0 && s.local_speed >= 1.5 && pos >= 0.0;
        // Same frame as arm still has the finished-lap / out-lap context — wait one tick.
        if moving && !ended && self.armed && self.anchor.valid && wall_ms > 0 && !just_armed {
            self.current
                .push_at_opt(pos, wall_ms, track_m(s), remounting);
        }

        self.last_lap_num = lap_num;
        if s.last_lap_ms > 0 {
            self.last_last_lap_ms = s.last_lap_ms;
            if !ended || self.shown_last_ms <= 0 || s.last_lap_ms <= self.shown_last_ms + 400 {
                self.shown_last_ms = s.last_lap_ms;
            }
        }
        self.last_cur_ms = cur_ms;
        if pos >= 0.0 {
            self.last_seen_pos = pos;
        }

        let lookup = if self.armed && pos >= 0.0 {
            self.lookup_pos = follow_pos(self.lookup_pos, pos, dt);
            self.lookup_pos
        } else {
            self.lookup_pos = -1.0;
            pos
        };
        let v = live_view(self, lookup, wall_ms, dt, false, now);
        self.last_view = v;
        self.session_view = live_view(self, lookup, wall_ms, dt, true, now);
        v
    }

    fn note_splits(&mut self, s: &Snapshot, now: Instant) {
        if !self.armed || !self.anchor.valid {
            return;
        }
        let s1 = s.sector_cur.first().copied().unwrap_or(0);
        let s2 = s.sector_cur.get(1).copied().unwrap_or(0);
        if s1 > 0 && s1 != self.last_s1 {
            if self.last_s1 <= 0 {
                let best = self.ref_lap_ms; // freeze vs full lap unused; use sector if known
                let _ = best;
                if DEFAULT_FREEZE_MS > 0 {
                    // Official gap at S1 vs tape at current pos when available.
                    let gap = if let Some(t) = self.reference.as_ref().and_then(|r| {
                        r.time_at(lap_pos(s).max(0.0))
                    }) {
                        s1 - t
                    } else {
                        0
                    };
                    self.frozen_gap = gap;
                    self.freeze_at = Some(Instant::now());
                }
                self.anchor.set(s1, now);
            }
            self.last_s1 = s1;
        }
        let to_s2 = accum_to_s2(s1, s2);
        if s2 > 0 && s2 != self.last_s2 {
            if self.last_s2 <= 0 && to_s2 > 0 {
                if DEFAULT_FREEZE_MS > 0 {
                    let gap = if let Some(t) = self.reference.as_ref().and_then(|r| {
                        r.time_at(lap_pos(s).max(0.0))
                    }) {
                        to_s2 - t
                    } else {
                        0
                    };
                    self.frozen_gap = gap;
                    self.freeze_at = Some(Instant::now());
                }
                self.anchor.set(to_s2, now);
            }
            self.last_s2 = s2;
        }
    }
}

/// Plugin `local_track_pos == 1.0` is the line, not a wrap.
pub(crate) fn lap_pos(s: &Snapshot) -> f32 {
    let raw = s.local_track_pos;
    if raw < 0.0 {
        return -1.0;
    }
    if (1.0..1.5).contains(&raw) {
        return 0.999_999;
    }
    norm_lap_pos(s, raw)
}

fn track_m(s: &Snapshot) -> f32 {
    if s.track_length.is_finite() && s.track_length > 80.0 {
        s.track_length
    } else {
        DEFAULT_TRACK_M
    }
}

fn too_fast(meters: f32, d_ms: i32) -> bool {
    if meters < CUT_METERS {
        return false;
    }
    let min_ms = ((meters / MAX_SPLINE_MPS) * 1000.0).ceil() as i32;
    if d_ms + PACE_SLACK_MS >= min_ms {
        return false;
    }
    if d_ms >= JUMP_MIN_MS && d_ms <= JUMP_MAX_MS && meters <= JUMP_MAX_M {
        return false;
    }
    true
}

/// S/F wrap: large negative track-pos delta.
fn sf_wrap(prev: f32, pos: f32) -> bool {
    prev >= 0.0 && (pos - prev) < -WRAP_THRESHOLD
}

fn wrapped(prev: f32, pos: f32) -> bool {
    sf_wrap(prev, pos)
}

fn wrap_is_finish(st: &DeltaEngine, pos: f32, cur_ms: i32) -> bool {
    if !wrapped(st.last_seen_pos, pos) {
        return false;
    }
    if st.last_cur_ms > 8_000 && cur_ms + 2_500 < st.last_cur_ms {
        return true;
    }
    // Mid-lap origin wrap: wall clock still fits the tape near pos 0.
    if let Some(t) = st.reference.as_ref().and_then(|r| r.time_at(pos.max(0.0))) {
        if (st.last_wall_ms - t).abs() <= 8_000 && st.last_wall_ms > 4_000 {
            return false;
        }
    }
    if st.reference.is_some() && pos < 0.20 && st.last_wall_ms > 4_000 {
        return true;
    }
    st.crashed_this_lap
        && pos < 0.20
        && st.last_seen_pos > 0.85
        && st.last_wall_ms > 8_000
        && st.current.recorded_ms() >= MIN_LAP_MS
}

fn tick_dt(last: &mut Option<Instant>, now: Instant) -> f32 {
    let raw = match *last {
        Some(t) => now.saturating_duration_since(t).as_secs_f32(),
        None => 1.0 / 60.0,
    };
    *last = Some(now);
    if raw < 0.001 {
        1.0 / 60.0
    } else {
        raw.min(0.05)
    }
}

fn follow_pos(prev: f32, pos: f32, dt: f32) -> f32 {
    if prev < 0.0 || wrapped(prev, pos) || (pos - prev).abs() > 0.05 {
        return pos;
    }
    let a = 1.0 - (-dt / POS_TAU).exp();
    prev + (pos - prev) * a
}

fn accum_to_s2(s1: i32, s2: i32) -> i32 {
    if s1 > 0 && s2 > s1 + s1 / 2 {
        s2
    } else {
        s1.saturating_add(s2)
    }
}

fn completed_lap_ms(st: &DeltaEngine, s: &Snapshot) -> i32 {
    let taped = st.current.recorded_ms();
    let live = st.last_wall_ms.max(taped);
    if s.last_lap_ms > 0 && (st.last_last_lap_ms <= 0 || s.last_lap_ms != st.last_last_lap_ms) {
        if live >= MIN_LAP_MS && live + 400 < s.last_lap_ms {
            return live;
        }
        if live <= 0 || (s.last_lap_ms - live).abs() < 8_000 {
            return s.last_lap_ms;
        }
        if live < 8_000 && s.last_lap_ms >= MIN_LAP_MS {
            if taped >= MIN_LAP_MS && taped + 400 < s.last_lap_ms {
                return taped;
            }
            return s.last_lap_ms;
        }
    }
    if live >= MIN_LAP_MS {
        return live;
    }
    st.last_cur_ms
}

fn lap_ended(st: &DeltaEngine, s: &Snapshot, pos: f32) -> bool {
    if st.current.filled < 8 {
        return false;
    }
    if st.last_cur_ms > 8_000 && s.current_lap_ms + 2_500 < st.last_cur_ms {
        return true;
    }
    let new_last = s.last_lap_ms > 0 && s.last_lap_ms != st.last_last_lap_ms;
    if new_last && (st.last_wall_ms > 8_000 || st.last_cur_ms > 8_000 || s.last_lap_ms >= MIN_LAP_MS)
    {
        return true;
    }
    let lap_up = s.current_lap > st.last_lap_num && st.last_lap_num > 0;
    if lap_up
        && (st.last_wall_ms > 8_000
            || st.last_cur_ms > 8_000
            || new_last
            || st.current.recorded_ms() >= MIN_LAP_MS)
    {
        return true;
    }
    if wrap_is_finish(st, pos, s.current_lap_ms) && st.last_wall_ms > 8_000 {
        return true;
    }
    wrapped(st.last_seen_pos, pos)
        && s.current_lap_ms < 4_000
        && st.current.recorded_ms() >= MIN_LAP_MS
}

fn try_commit(st: &mut DeltaEngine, lap_ms: i32, now: Instant) {
    if !st.observed_lap_start {
        return;
    }
    if !st.current.decent(lap_ms) {
        return;
    }
    if st.session_lap_ms <= 0 || lap_ms < st.session_lap_ms {
        st.session = Some(st.current.clone());
        st.session_lap_ms = lap_ms;
        st.session_new_best_at = Some(now);
    }
    let better = st.ref_lap_ms <= 0 || lap_ms < st.ref_lap_ms;
    if better {
        st.reference = Some(st.current.clone());
        st.ref_lap_ms = lap_ms;
        if !st.track_key.is_empty() && !st.bike_key.is_empty() {
            track_pb::commit_tape(&st.track_key, &st.bike_key, lap_ms, st.current.bins);
        }
        st.new_best_at = Some(now);
    }
}

fn live_view(
    st: &mut DeltaEngine,
    pos: f32,
    wall_ms: i32,
    dt: f32,
    session: bool,
    now: Instant,
) -> DeltaView {
    let ready = if session {
        st.session.is_some()
    } else {
        st.reference.is_some()
    };
    let recording = !ready && st.armed;
    let ref_lap_ms = if session {
        st.session_lap_ms
    } else {
        st.ref_lap_ms
    };
    let new_best_at = if session {
        st.session_new_best_at
    } else {
        st.new_best_at
    };

    let frozen = st.freeze_at.is_some_and(|t| {
        t.elapsed().as_millis() < DEFAULT_FREEZE_MS as u128
    });
    let raw = if frozen && ready {
        Some(st.frozen_gap)
    } else if ready && st.anchor.valid && wall_ms > 200 && pos >= 0.0 && st.armed {
        let t = if session {
            st.session.as_ref().and_then(|r| r.time_at(pos))
        } else {
            st.reference.as_ref().and_then(|r| r.time_at(pos))
        };
        t.map(|t| wall_ms - t)
    } else {
        None
    };

    let (smooth_ms, smooth_init) = if session {
        (&mut st.session_smooth_ms, &mut st.session_smooth_init)
    } else {
        (&mut st.smooth_ms, &mut st.smooth_init)
    };
    let delta_ms = match raw {
        Some(ms) => {
            let v = ms as f32;
            if !*smooth_init {
                *smooth_ms = v;
                *smooth_init = true;
            } else {
                let a = 1.0 - (-dt / SMOOTH_TAU).exp();
                *smooth_ms += (v - *smooth_ms) * a;
            }
            smooth_ms.round() as i32
        }
        None => {
            *smooth_init = false;
            0
        }
    };
    DeltaView {
        ready,
        recording,
        has_delta: raw.is_some(),
        delta_ms: if raw.is_some() { delta_ms } else { 0 },
        ref_lap_ms,
        last_lap_ms: st.shown_last_ms,
        cover: st.current.cover(),
        new_best: new_best_at.is_some_and(|t| now.saturating_duration_since(t).as_secs_f32() < NEW_BEST_HOLD_S),
    }
}

static STORE: Mutex<DeltaEngine> = Mutex::new(DeltaEngine::new());

static PREVIEW: Mutex<Option<DeltaView>> = Mutex::new(None);

/// Wall-clock lap elapsed from the last `tick`.
pub fn lap_clock(s: &Snapshot) -> i32 {
    STORE
        .lock()
        .map(|g| {
            if g.anchor.valid {
                g.last_wall_ms
            } else {
                s.current_lap_ms.max(0)
            }
        })
        .unwrap_or_else(|_| s.current_lap_ms.max(0))
}

/// Once per live frame from the overlay loop. Records even when the widget is hidden.
pub fn tick(s: &Snapshot) -> DeltaView {
    let recorded = STORE
        .lock()
        .map(|mut g| g.tick(s))
        .unwrap_or_else(|_| DeltaView::empty());
    if let Ok(g) = PREVIEW.lock() {
        if let Some(v) = *g {
            return v;
        }
    }
    recorded
}

pub fn view() -> DeltaView {
    view_for(false)
}

pub fn view_for(session: bool) -> DeltaView {
    if let Ok(g) = PREVIEW.lock() {
        if let Some(v) = *g {
            return v;
        }
    }
    STORE
        .lock()
        .map(|g| if session { g.session_view } else { g.last_view })
        .unwrap_or_else(|_| DeltaView::empty())
}

pub fn session_tape() -> Option<([i32; BINS], i32)> {
    STORE.lock().ok().and_then(|g| {
        g.session
            .as_ref()
            .map(|t| (t.bins, g.session_lap_ms))
            .filter(|(_, ms)| *ms > 0)
    })
}

pub fn reload_saved() {
    if let Ok(mut g) = STORE.lock() {
        let track = g.track_key.clone();
        let bike = g.bike_key.clone();
        if !track.is_empty() {
            g.reset_track(track, bike);
        } else {
            g.reference = None;
            g.ref_lap_ms = 0;
            g.last_view = DeltaView::empty();
        }
    }
}

pub fn set_preview(v: Option<DeltaView>) {
    if let Ok(mut g) = PREVIEW.lock() {
        *g = v;
    }
}

#[cfg(test)]
#[path = "tests/delta.rs"]
mod tests;
