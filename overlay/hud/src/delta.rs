//! Time vs your best at this point on the lap.
//!
//! The plugin does not expose the in-game ghost. We record
//! `local_track_pos → current_lap_ms` on a decent lap and compare live.

use crate::race_store::norm_lap_pos;
use crate::shm::{cstr, Snapshot};
use crate::track_pb::{self, BINS};
use std::sync::Mutex;
use std::time::Instant;
const MIN_LAP_MS: i32 = 20_000;
const MAX_LAP_MS: i32 = 15 * 60 * 1000;
const MIN_COVERAGE: f32 = 0.55;
/// Displayed hairline time constant. Per-frame blend was still jumpy at 60 Hz.
const SMOOTH_TAU: f32 = 0.42;
const POS_TAU: f32 = 0.08;
/// How long the LAST slot reads NEW BEST after a PB.
const NEW_BEST_HOLD_S: f32 = 8.0;
/// Bar saturates at this |delta|.
pub const BAR_RANGE_MS: i32 = 2_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeltaView {
    /// A committed reference lap exists.
    pub ready: bool,
    /// Filling the first (or a replacement) lap.
    pub recording: bool,
    /// True when we have a comparison at this pos.
    pub has_delta: bool,
    /// Live − reference at this track pos. Negative = faster.
    pub delta_ms: i32,
    pub ref_lap_ms: i32,
    /// Last completed lap (held when the plugin zeros last_lap_ms).
    pub last_lap_ms: i32,
    /// 0..=100 of the lap currently being taped.
    pub cover: u8,
    /// True for a few seconds after a faster decent lap commits.
    pub new_best: bool,
}

impl DeltaView {
    pub fn empty() -> Self {
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

#[derive(Clone)]
struct LapTape {
    bins: [i32; BINS],
    filled: usize,
    min_pos: f32,
    max_pos: f32,
    last_pos: f32,
    last_ms: i32,
}

impl LapTape {
    fn new() -> Self {
        Self {
            bins: [0; BINS],
            filled: 0,
            min_pos: 1.0,
            max_pos: 0.0,
            last_pos: -1.0,
            last_ms: 0,
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

    fn push(&mut self, pos: f32, ms: i32) {
        let pos = if pos >= 1.0 { 0.999_999 } else { pos };
        if !(0.0..1.0).contains(&pos) || ms <= 0 {
            return;
        }
        if self.last_pos >= 0.0 {
            let d = pos - self.last_pos;
            // Backwards on the same stretch — skip. A wrap is a lap end; tick()
            // resets the tape first. If it missed, move last_pos so we do not freeze.
            if d < -0.50 {
                self.last_pos = pos;
                self.last_ms = ms;
                return;
            }
            if d < -0.02 {
                return;
            }
        }
        // Clock restarted. Do not mix laps — but do not freeze last_ms either.
        if self.last_ms > 0 && ms + 250 < self.last_ms {
            self.last_ms = ms;
            self.last_pos = pos;
            return;
        }
        let i = ((pos * BINS as f32) as usize).min(BINS - 1);
        if self.bins[i] == 0 {
            self.filled += 1;
            self.bins[i] = ms;
        } else if ms < self.bins[i] {
            self.bins[i] = ms;
        }
        self.min_pos = self.min_pos.min(pos);
        self.max_pos = self.max_pos.max(pos);
        self.last_pos = pos;
        self.last_ms = ms;
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
    }
}

pub(crate) struct DeltaEngine {
    track_key: String,
    bike_key: String,
    current: LapTape,
    reference: Option<LapTape>,
    ref_lap_ms: i32,
    last_lap_num: i32,
    last_last_lap_ms: i32,
    shown_last_ms: i32,
    last_cur_ms: i32,
    last_seen_pos: f32,
    lookup_pos: f32,
    last_tick: Option<Instant>,
    smooth_ms: f32,
    smooth_init: bool,
    /// True after S/F until the lap clock restarts. The plugin often keeps the
    /// finished-lap / out-lap clock for a frame or more at pos ~0.
    stale_clock: bool,
    /// Flying lap started (S/F cross, or lap clock on near the line). Out-lap is not this.
    armed: bool,
    new_best_at: Option<Instant>,
    last_view: DeltaView,
}

impl DeltaEngine {
    fn new() -> Self {
        Self {
            track_key: String::new(),
            bike_key: String::new(),
            current: LapTape::new(),
            reference: None,
            ref_lap_ms: 0,
            last_lap_num: 0,
            last_last_lap_ms: 0,
            shown_last_ms: 0,
            last_cur_ms: 0,
            last_seen_pos: -1.0,
            lookup_pos: -1.0,
            last_tick: None,
            smooth_ms: 0.0,
            smooth_init: false,
            stale_clock: false,
            armed: false,
            new_best_at: None,
            last_view: DeltaView::empty(),
        }
    }

    fn reset_track(&mut self, track: String, bike: String) {
        *self = Self::new();
        self.track_key = track.clone();
        self.bike_key = bike.clone();
        let pb = track_pb::bind(&track, &bike);
        if pb.has_tape() {
            self.reference = Some(LapTape::from_saved(pb.bins));
            self.ref_lap_ms = pb.lap_ms;
        }
    }

    fn adopt_bike(&mut self, bike: String) {
        self.bike_key = bike.clone();
        let pb = track_pb::bind(&self.track_key, &bike);
        if pb.has_tape() {
            self.reference = Some(LapTape::from_saved(pb.bins));
            self.ref_lap_ms = pb.lap_ms;
        }
    }

    pub fn tick(&mut self, s: &Snapshot) -> DeltaView {
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

        let wrap = wrapped(self.last_seen_pos, pos);
        let clock_drop = self.last_cur_ms > 8_000 && cur_ms + 2_500 < self.last_cur_ms;
        let clock_start_at_line = pos < 0.20 && cur_ms > 200 && self.last_cur_ms <= 200;
        let new_last = s.last_lap_ms > 0 && s.last_lap_ms != self.last_last_lap_ms;
        let lap_up = lap_num > self.last_lap_num && self.last_lap_num > 0;
        let crossed_sf = wrap || new_last || lap_up;
        let dt = tick_dt(&mut self.last_tick);
        if wrap || clock_drop {
            self.smooth_init = false;
            self.lookup_pos = pos;
        }
        // Old clock at the line only. A mid-track pos jump is not S/F — do not freeze the bar.
        if wrap && !clock_drop && !new_last && cur_ms > 4_000 && pos < 0.20 {
            self.stale_clock = true;
        }
        if self.stale_clock && (clock_drop || new_last || cur_ms < 4_000) {
            self.stale_clock = false;
            self.smooth_init = false;
        }

        let ended = lap_ended(self, s, pos);
        if ended {
            let done = completed_lap_ms(self, s);
            try_commit(self, done);
            if done > 0 {
                self.shown_last_ms = done;
            }
            self.current = LapTape::new();
            self.smooth_init = false;
            // S/F starts the next flying lap. A reset only drops the clock.
            self.armed = crossed_sf;
        } else if !self.armed
            && (crossed_sf || clock_start_at_line || (clock_drop && pos < 0.35))
        {
            self.armed = true;
            self.current = LapTape::new();
        }

        // Same-frame sample still has the old clock at pos 0 (plugin sends 1.0).
        let moving = s.on_track != 0 && s.local_speed >= 1.5 && cur_ms > 0 && pos >= 0.0;
        if moving && !ended && self.armed && !self.stale_clock {
            self.current.push(pos, cur_ms);
        }

        self.last_lap_num = lap_num;
        if s.last_lap_ms > 0 {
            self.last_last_lap_ms = s.last_lap_ms;
            self.shown_last_ms = s.last_lap_ms;
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
        let v = live_view(self, lookup, cur_ms, dt);
        self.last_view = v;
        v
    }
}

/// Plugin `local_track_pos == 1.0` is the line, not a wrap. `rem_euclid(1.0)` is 0.0
/// and would look like S/F while the clock is still the finished lap.
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

fn wrapped(prev: f32, pos: f32) -> bool {
    prev >= 0.0 && prev > 0.65 && pos < 0.35 && (prev - pos) > 0.45
}

fn tick_dt(last: &mut Option<Instant>) -> f32 {
    let now = Instant::now();
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

/// The plugin often zeros `last_lap_ms` on the crossing frame. The live clock
/// just before the drop is the lap we actually recorded.
fn completed_lap_ms(st: &DeltaEngine, s: &Snapshot) -> i32 {
    if s.last_lap_ms > 0
        && (st.last_last_lap_ms <= 0 || s.last_lap_ms != st.last_last_lap_ms)
        && (st.last_cur_ms <= 0 || (s.last_lap_ms - st.last_cur_ms).abs() < 8_000)
    {
        return s.last_lap_ms;
    }
    st.last_cur_ms
}

fn lap_ended(st: &DeltaEngine, s: &Snapshot, pos: f32) -> bool {
    if st.current.filled < 8 {
        return false;
    }
    // Plugin often zeros last_lap_ms on the crossing. The live clock drop is the tell.
    if st.last_cur_ms > 8_000 && s.current_lap_ms + 2_500 < st.last_cur_ms {
        return true;
    }
    wrapped(st.last_seen_pos, pos) && st.last_cur_ms > 8_000
}

fn try_commit(st: &mut DeltaEngine, lap_ms: i32) {
    if !st.current.decent(lap_ms) {
        return;
    }
    let better = st.ref_lap_ms <= 0 || lap_ms < st.ref_lap_ms;
    if better {
        st.reference = Some(st.current.clone());
        st.ref_lap_ms = lap_ms;
        if !st.track_key.is_empty() {
            track_pb::commit_tape(&st.track_key, &st.bike_key, lap_ms, st.current.bins);
        }
        st.new_best_at = Some(Instant::now());
    }
}

fn live_view(st: &mut DeltaEngine, pos: f32, cur_ms: i32, dt: f32) -> DeltaView {
    let ready = st.reference.is_some();
    let recording = !ready && st.armed;
    let raw = if ready && !st.stale_clock && cur_ms > 200 && pos >= 0.0 {
        st.reference.as_ref().and_then(|r| r.time_at(pos)).and_then(|t| {
            // Crossing / out-lap: old clock at pos ~0 vs tape ~0 → fake +16s, and
            // smoothing would hold it. Wait until the new lap clock matches the pos.
            if pos < 0.15 && cur_ms > t + 8_000 {
                None
            } else if !st.armed && (cur_ms - t).abs() > 10_000 {
                // Out-lap with a saved tape: clock does not match a flying lap.
                None
            } else {
                Some(cur_ms - t)
            }
        })
    } else {
        None
    };
    let delta_ms = match raw {
        Some(ms) => {
            let v = ms as f32;
            if !st.smooth_init {
                st.smooth_ms = v;
                st.smooth_init = true;
            } else {
                let a = 1.0 - (-dt / SMOOTH_TAU).exp();
                st.smooth_ms += (v - st.smooth_ms) * a;
            }
            st.smooth_ms.round() as i32
        }
        None => {
            st.smooth_init = false;
            0
        }
    };
    DeltaView {
        ready,
        recording,
        has_delta: raw.is_some(),
        delta_ms: if raw.is_some() { delta_ms } else { 0 },
        ref_lap_ms: st.ref_lap_ms,
        last_lap_ms: st.shown_last_ms,
        cover: st.current.cover(),
        new_best: st
            .new_best_at
            .is_some_and(|t| t.elapsed().as_secs_f32() < NEW_BEST_HOLD_S),
    }
}

static STORE: Mutex<DeltaEngine> = Mutex::new(DeltaEngine {
    track_key: String::new(),
    bike_key: String::new(),
    current: LapTape {
        bins: [0; BINS],
        filled: 0,
        min_pos: 1.0,
        max_pos: 0.0,
        last_pos: -1.0,
        last_ms: 0,
    },
    reference: None,
    ref_lap_ms: 0,
    last_lap_num: 0,
    last_last_lap_ms: 0,
    shown_last_ms: 0,
    last_cur_ms: 0,
    last_seen_pos: -1.0,
    lookup_pos: -1.0,
    last_tick: None,
    smooth_ms: 0.0,
    smooth_init: false,
    stale_clock: false,
    armed: false,
    new_best_at: None,
    last_view: DeltaView {
        ready: false,
        recording: false,
        has_delta: false,
        delta_ms: 0,
        ref_lap_ms: 0,
        last_lap_ms: 0,
        cover: 0,
        new_best: false,
    },
});

static PREVIEW: Mutex<Option<DeltaView>> = Mutex::new(None);

/// Once per live frame from the overlay loop. Records even when the widget is hidden.
pub fn tick(s: &Snapshot) -> DeltaView {
    let recorded = STORE.lock().map(|mut g| g.tick(s)).unwrap_or_else(|_| DeltaView::empty());
    if let Ok(g) = PREVIEW.lock() {
        if let Some(v) = *g {
            return v;
        }
    }
    recorded
}

/// Last `tick` (or preview). Draw reads this so recording is not tied to compositing.
pub fn view() -> DeltaView {
    if let Ok(g) = PREVIEW.lock() {
        if let Some(v) = *g {
            return v;
        }
    }
    STORE.lock().map(|g| g.last_view).unwrap_or_else(|_| DeltaView::empty())
}

/// Drop the in-memory tape after clearing the saved file.
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
/// Announce shots / tests: force the bar without recording a lap.
pub fn set_preview(v: Option<DeltaView>) {
    if let Ok(mut g) = PREVIEW.lock() {
        *g = v;
    }
}

#[cfg(test)]
fn snap(track: &str, pos: f32, cur_ms: i32, last_ms: i32, lap: i32) -> Snapshot {
    let mut s = Snapshot::default();
    s.on_track = 1;
    s.has_telemetry = 1;
    s.local_speed = 12.0;
    s.local_track_pos = pos;
    s.current_lap_ms = cur_ms;
    s.last_lap_ms = last_ms;
    s.current_lap = lap;
    s.track_length = 1600.0;
    crate::shm::write_name(&mut s.track_name, track);
    s
}

#[cfg(test)]
fn with_bike(s: &mut Snapshot, bike: &str) {
    s.local_race_num = 1;
    s.standing_count = 1;
    s.standings[0].race_num = 1;
    crate::shm::write_name(&mut s.standings[0].bike, bike);
}

#[cfg(test)]
fn run_lap(eng: &mut DeltaEngine, track: &str, lap_ms: i32, lap_num: i32, crashed: bool) {
    let n = 180;
    for i in 0..n {
        let t = i as f32 / n as f32;
        let mut s = snap(track, t, (lap_ms as f32 * t) as i32, 0, lap_num);
        if i + 1 == n {
            s.local_track_pos = 0.999;
            s.current_lap_ms = lap_ms;
        }
        if crashed && i > n / 2 {
            s.local_crashed = 1;
        }
        eng.tick(&s);
    }
    // Crossing often arrives with last_ms still 0 — match the plugin.
    eng.tick(&snap(track, 0.01, 180, 0, lap_num + 1));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outlap_is_not_recording() {
        let mut eng = DeltaEngine::new();
        let v = eng.tick(&snap("A", 0.4, 20_000, 0, 1));
        assert!(!v.ready);
        assert!(!v.recording);
        assert_eq!(v.delta_ms, 0);
        assert_eq!(eng.current.filled, 0);
    }

    #[test]
    fn recording_starts_after_sf_cross() {
        let mut eng = DeltaEngine::new();
        for i in 0..20 {
            let t = 0.70 + i as f32 * 0.012;
            eng.tick(&snap("A", t.min(0.98), 12_000 + i * 150, 0, 1));
        }
        assert!(!eng.armed);
        assert_eq!(eng.current.filled, 0);
        let cross = eng.tick(&snap("A", 0.02, 15_000, 0, 1));
        assert!(eng.armed);
        assert!(
            eng.current.filled == 0,
            "must not tape the out-lap clock at the line, filled={}",
            eng.current.filled
        );
        assert!(!cross.has_delta);
        let v = eng.tick(&snap("A", 0.05, 400, 0, 2));
        assert!(v.recording);
        assert_eq!(eng.current.filled, 1);
    }

    #[test]
    fn last_lap_at_sf_starts_recording() {
        let mut eng = DeltaEngine::new();
        for i in 0..12 {
            eng.tick(&snap("A", 0.40 + i as f32 * 0.01, 8_000 + i * 200, 0, 1));
        }
        assert!(!eng.armed);
        assert_eq!(eng.current.filled, 0);
        // S/F is not at pos 0. Last-lap time is the crossing.
        eng.tick(&snap("A", 0.28, 120, 14_800, 2));
        assert!(eng.armed, "finish-line last-lap must start the flying lap");
        let v = eng.tick(&snap("A", 0.30, 400, 14_800, 2));
        assert!(v.recording);
        assert_eq!(eng.current.filled, 2);
    }

    #[test]
    fn saved_tape_compares_after_sf_not_at_zero() {
        let mut eng = DeltaEngine::new();
        run_lap(&mut eng, "A", 72_000, 1, false);
        assert!(eng.reference.is_some());
        let first = eng.tick(&snap("A", 0.22, 16_000, 72_000, 2));
        assert!(first.ready);
        assert!(first.has_delta, "bar must move after S/F away from pos 0");
        let t = eng.reference.as_ref().unwrap().time_at(0.22).unwrap();
        assert_eq!(first.delta_ms, 16_000 - t);
        for i in 1..20 {
            let t = 0.22 + i as f32 * 0.01;
            eng.tick(&snap("A", t, 16_000 + i * 400, 72_000, 2));
        }
        let v = eng.tick(&snap("A", 0.45, 32_000, 72_000, 2));
        assert!(v.ready);
        assert!(v.has_delta);
    }

    #[test]
    fn reset_to_pits_waits_for_sf() {
        let mut eng = DeltaEngine::new();
        run_lap(&mut eng, "A", 72_000, 1, false);
        for i in 0..40 {
            let t = i as f32 / 200.0;
            eng.tick(&snap("A", t, 200 + i * 400, 0, 2));
        }
        assert!(eng.armed);
        assert_eq!(eng.current.filled, 40);
        // Reset: clock dies mid-track, spawn in the pits.
        eng.tick(&snap("A", 0.42, 80, 0, 2));
        assert!(!eng.armed, "reset is an out-lap");
        assert_eq!(eng.current.filled, 0);
        for i in 0..15 {
            eng.tick(&snap("A", 0.45 + i as f32 * 0.02, 1_000 + i * 200, 0, 2));
        }
        assert_eq!(eng.current.filled, 0);
        eng.tick(&snap("A", 0.02, 16_000, 0, 2));
        eng.tick(&snap("A", 0.04, 300, 0, 3));
        assert!(eng.armed);
        assert_eq!(eng.current.filled, 1);
    }

    #[test]
    fn first_lap_is_set_not_compared() {
        let mut eng = DeltaEngine::new();
        eng.tick(&snap("A", 0.02, 80, 0, 1));
        let v = eng.tick(&snap("A", 0.05, 400, 0, 1));
        assert!(!v.ready);
        assert!(v.recording);
        assert_eq!(v.delta_ms, 0);
    }

    #[test]
    fn decent_lap_becomes_reference() {
        let mut eng = DeltaEngine::new();
        run_lap(&mut eng, "A", 72_000, 1, false);
        assert!(eng.reference.is_some());
        assert_eq!(eng.ref_lap_ms, 72_000, "ref {}", eng.ref_lap_ms);
        let v = eng.tick(&snap("A", 0.5, 35_000, 72_000, 2));
        assert!(v.ready);
        let t = eng.reference.as_ref().unwrap().time_at(0.5).unwrap();
        assert_eq!(v.delta_ms, 35_000 - t);
        assert_eq!(v.last_lap_ms, 72_000);
        assert_eq!(v.ref_lap_ms, 72_000);
    }

    #[test]
    fn last_lap_held_when_plugin_zeros() {
        let mut eng = DeltaEngine::new();
        run_lap(&mut eng, "A", 72_000, 1, false);
        assert_eq!(eng.shown_last_ms, 72_000, "held {}", eng.shown_last_ms);
        let v = eng.tick(&snap("A", 0.2, 10_000, 0, 2));
        assert_eq!(v.last_lap_ms, 72_000, "last {}", v.last_lap_ms);
        assert_eq!(v.ref_lap_ms, 72_000, "best {}", v.ref_lap_ms);
    }

    #[test]
    fn slower_lap_does_not_replace_best() {
        let mut eng = DeltaEngine::new();
        run_lap(&mut eng, "A", 70_000, 1, false);
        run_lap(&mut eng, "A", 80_000, 2, false);
        assert_eq!(eng.ref_lap_ms, 70_000, "ref {}", eng.ref_lap_ms);
    }

    #[test]
    fn faster_lap_replaces_best() {
        let mut eng = DeltaEngine::new();
        run_lap(&mut eng, "A", 74_000, 1, false);
        run_lap(&mut eng, "A", 71_000, 2, false);
        assert_eq!(eng.ref_lap_ms, 71_000, "ref {}", eng.ref_lap_ms);
        assert!(eng.last_view.new_best);
    }

    #[test]
    fn first_commit_is_new_best() {
        let mut eng = DeltaEngine::new();
        run_lap(&mut eng, "A", 72_000, 1, false);
        assert!(eng.last_view.new_best);
    }

    #[test]
    fn dab_does_not_block_reference() {
        let mut eng = DeltaEngine::new();
        run_lap(&mut eng, "A", 90_000, 1, true);
        assert!(eng.reference.is_some());
        assert_eq!(eng.ref_lap_ms, 90_000);
    }

    #[test]
    fn plugin_pos_one_does_not_poison_next_lap() {
        let mut eng = DeltaEngine::new();
        let lap_ms = 90_000;
        let n = 180;
        for i in 0..n {
            let t = i as f32 / n as f32;
            let mut s = snap("A", t, (lap_ms as f32 * t) as i32, 0, 1);
            if i + 1 == n {
                s.local_track_pos = 0.999;
                s.current_lap_ms = lap_ms;
            }
            eng.tick(&s);
        }
        // Plugin sends 1.0 with the finished-lap clock still showing.
        eng.tick(&snap("A", 1.0, lap_ms, 0, 1));
        eng.tick(&snap("A", 0.0, 15, 0, 2));
        assert!(eng.reference.is_some(), "first lap should commit");
        for i in 1..60 {
            let t = i as f32 / 180.0;
            eng.tick(&snap("A", t, 300 + i * 400, 0, 2));
        }
        assert_eq!(
            eng.current.filled, 59,
            "next lap frozen, filled={}",
            eng.current.filled
        );
        let v = eng.tick(&snap("A", 60.0 / 180.0, 300 + 60 * 400, 0, 2));
        assert!(v.ready);
        assert!(v.has_delta, "should compare on lap 2");
        assert_eq!(v.delta_ms.signum(), -1);
    }

    #[test]
    fn short_outlap_is_not_reference() {
        let mut eng = DeltaEngine::new();
        for i in 0..40 {
            let t = 0.4 + i as f32 * 0.01;
            eng.tick(&snap("A", t, 8_000 + i * 200, 0, 1));
        }
        let mut done = snap("A", 0.02, 100, 12_000, 2);
        done.last_lap_ms = 12_000;
        assert_eq!(eng.current.filled, 0, "out-lap must not fill the tape");
        eng.tick(&done);
        assert!(eng.reference.is_none());
    }

    #[test]
    fn track_change_drops_reference() {
        let mut eng = DeltaEngine::new();
        run_lap(&mut eng, "A", 72_000, 1, false);
        assert!(eng.reference.is_some());
        let v = eng.tick(&snap("B", 0.2, 5_000, 0, 1));
        assert!(!v.ready);
        assert!(eng.reference.is_none());
    }

    #[test]
    fn interpolates_missing_bins() {
        let mut tape = LapTape::new();
        tape.push(10.0 / BINS as f32, 10_000);
        tape.push(20.0 / BINS as f32, 20_000);
        assert_eq!(tape.time_at(15.0 / BINS as f32), Some(15_000));
    }

    #[test]
    fn crossing_with_zero_last_ms_still_commits() {
        let mut eng = DeltaEngine::new();
        run_lap(&mut eng, "A", 90_708, 12, false);
        assert!(eng.reference.is_some(), "should commit using live clock");
        assert_eq!(eng.ref_lap_ms, 90_708, "ref {}", eng.ref_lap_ms);
        let v = eng.tick(&snap("A", 0.25, 20_000, 0, 13));
        assert!(v.ready);
        let t = eng.reference.as_ref().unwrap().time_at(0.25).unwrap();
        assert_eq!(v.delta_ms, 20_000 - t);
    }

    #[test]
    fn next_lap_keeps_recording_after_wrap() {
        let mut eng = DeltaEngine::new();
        run_lap(&mut eng, "A", 90_000, 1, false);
        assert!(eng.reference.is_some());
        for i in 0..40 {
            let t = i as f32 / 200.0;
            eng.tick(&snap("A", t, 200 + i * 400, 0, 2));
        }
        assert_eq!(eng.current.filled, 40, "filled {}", eng.current.filled);
        let v = eng.tick(&snap("A", 0.20, 18_000, 0, 2));
        assert!(v.ready);
    }

    #[test]
    fn pos_one_is_recorded() {
        let mut tape = LapTape::new();
        tape.push(1.0, 72_000);
        assert_eq!(tape.filled, 1);
        assert_eq!(tape.time_at(0.999_999), Some(72_000));
    }

    #[test]
    fn saved_tape_loads_without_riding() {
        let _lock = track_pb::exclusive_test();
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("mxbo-delta-load-{n}"));
        track_pb::set_store_dir(dir.clone());
        let mut bins = [0; BINS];
        for i in 0..BINS {
            bins[i] = 100 + (i as i32) * 280;
        }
        assert!(track_pb::commit_tape("Saved", "", 72_000, bins));
        let mut eng = DeltaEngine::new();
        let first = eng.tick(&snap("Saved", 0.05, 400, 72_000, 2));
        assert!(first.ready);
        assert!(first.has_delta);
        let t = eng.reference.as_ref().unwrap().time_at(0.05).unwrap();
        assert_eq!(first.delta_ms, 400 - t);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn wrap_with_old_clock_does_not_show_plus_sixteen() {
        let mut eng = DeltaEngine::new();
        run_lap(&mut eng, "A", 72_000, 1, false);
        assert!(eng.reference.is_some());
        for i in 0..12 {
            let t = (0.90 + i as f32 * 0.008).min(0.995);
            eng.tick(&snap("A", t, 15_200 + i * 80, 0, 2));
        }
        let poison = eng.tick(&snap("A", 0.01, 16_000, 0, 2));
        assert!(
            !poison.has_delta,
            "crossing still comparing the out-lap clock: delta={}",
            poison.delta_ms
        );
        let v = eng.tick(&snap("A", 0.03, 500, 0, 2));
        assert!(v.has_delta, "should compare once the new lap clock starts");
        let t = eng.reference.as_ref().unwrap().time_at(0.03).unwrap();
        assert_eq!(v.delta_ms, 500 - t);
    }

    #[test]
    fn mid_lap_plus_sixteen_still_shows() {
        let mut eng = DeltaEngine::new();
        run_lap(&mut eng, "A", 72_000, 1, false);
        let v = eng.tick(&snap("A", 0.50, 52_000, 72_000, 2));
        assert!(v.has_delta);
        let t = eng.reference.as_ref().unwrap().time_at(0.50).unwrap();
        assert_eq!(v.delta_ms, 52_000 - t);
    }

    #[test]
    fn decent_rejects_short_sparse_and_narrow_tapes() {
        let mut tape = LapTape::new();
        for i in 0..200 {
            tape.push(i as f32 / 200.0, 10_000 + i * 50);
        }
        assert!(!tape.decent(19_999));
        assert!(tape.decent(20_000));
        assert!(!tape.decent(900_001));

        let mut sparse = LapTape::new();
        for i in 0..100 {
            sparse.push(i as f32 / BINS as f32, 30_000 + i);
        }
        assert!(!sparse.decent(72_000));

        let mut narrow = LapTape::new();
        for i in 0..200 {
            let t = 0.10 + i as f32 / 200.0 * 0.40;
            narrow.push(t, 30_000 + i * 100);
        }
        assert!(!narrow.decent(72_000));
    }

    #[test]
    fn push_skips_soft_reverse_and_clock_restart() {
        let mut tape = LapTape::new();
        tape.push(0.50, 10_000);
        tape.push(0.47, 10_100);
        assert_eq!(tape.filled, 1);
        tape.push(0.52, 10_200);
        assert_eq!(tape.filled, 2);
        tape.push(0.60, 20_000);
        tape.push(0.61, 100);
        assert_eq!(tape.filled, 3);
        assert_eq!(tape.bins[((0.61 * BINS as f32) as usize).min(BINS - 1)], 0);
    }

    #[test]
    fn other_bike_does_not_load_saved_tape() {
        let _lock = track_pb::exclusive_test();
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("mxbo-delta-bike-{n}"));
        track_pb::set_store_dir(dir.clone());
        let mut bins = [0; BINS];
        for i in 0..BINS {
            bins[i] = 100 + (i as i32) * 280;
        }
        assert!(track_pb::commit_tape("Saved", "YZ450F", 72_000, bins));
        let mut eng = DeltaEngine::new();
        let mut two = snap("Saved", 0.05, 400, 72_000, 2);
        with_bike(&mut two, "YZ250F");
        let v = eng.tick(&two);
        assert!(!v.ready, "250 must not load the 450 tape");
        assert!(!v.has_delta);
        let mut four = snap("Saved", 0.05, 400, 72_000, 2);
        with_bike(&mut four, "YZ450F");
        let mut eng2 = DeltaEngine::new();
        let v2 = eng2.tick(&four);
        assert!(v2.ready);
        assert!(v2.has_delta);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn same_class_loads_saved_tape() {
        let _lock = track_pb::exclusive_test();
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("mxbo-delta-class-{n}"));
        track_pb::set_store_dir(dir.clone());
        let mut bins = [0; BINS];
        for i in 0..BINS {
            bins[i] = 100 + (i as i32) * 280;
        }
        assert!(track_pb::commit_tape("Saved", "YZ250F", 72_000, bins));
        let mut eng = DeltaEngine::new();
        let mut honda = snap("Saved", 0.05, 400, 72_000, 2);
        with_bike(&mut honda, "CRF250R");
        let v = eng.tick(&honda);
        assert!(v.ready, "Honda 250 should load the Yamaha 250 tape");
        assert!(v.has_delta);
        let _ = std::fs::remove_dir_all(dir);
    }
}
