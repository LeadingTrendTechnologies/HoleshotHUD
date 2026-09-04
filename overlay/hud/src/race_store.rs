//! Shared race view: session clock + classification field.
//!
//! [`RaceStore::refresh`] once per overlay frame. Drawers use [`RaceStore::with`]
//! (re-entrant) so they do not clone the store. [`RaceStore::tick`] still returns
//! a clone for tests and clock logs.

use crate::shm::{cstr, Snapshot, Standing, MAX_STANDINGS};
use std::cell::Cell;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicI32, Ordering};
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
        } else if is_warmup(s) {
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
});

thread_local! {
    static WITH: Cell<Option<NonNull<RaceStore>>> = const { Cell::new(None) };
}

/// Race order we published last tick, P1 first, as race numbers. The classification the
/// game sends only moves when someone crosses the line, so this is also the hysteresis
/// state that keeps a pass from flickering while two riders run side by side.
static LIVE_ORDER: Mutex<Vec<i32>> = Mutex::new(Vec::new());

/// A rider has to be this far up the track before they take the place. About a bike and a
/// half: `track_pos` is a centerline projection, so a rider taking a wide line or landing
/// off a jump moves a metre or two on its own.
const PASS_M: f32 = 3.0;
/// Once they have it they keep it until they drop back inside this.
const HOLD_M: f32 = 0.5;
/// Further apart than this and we cannot say who is ahead from `track_pos` alone: the
/// fraction is measured from the centerline origin, not from the start/finish line, so
/// only riders close together compare safely.
const PAIR_MAX_M: f32 = 250.0;

/// `track_pos` as a 0..1 lap fraction. The plugin sends metres on some tracks.
pub(crate) fn norm_lap_pos(s: &Snapshot, pos: f32) -> f32 {
    if pos < 0.0 {
        return pos;
    }
    if pos > 1.5 && s.track_length > 10.0 {
        (pos / s.track_length).rem_euclid(1.0)
    } else {
        pos.rem_euclid(1.0)
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
/// ranked by lap time, and on the gate everyone sits on the same stretch of track.
///
/// Read from the clock this tick already built rather than from `is_warmup` / `IN_GATE`:
/// the session heuristics latch state as they answer, and re-asking them here would rearm
/// the flag and lap machine after the banner for this frame was decided.
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
fn done_racing(st: &Standing) -> bool {
    let base = LEADER_FIN_LOCAL_BASE.load(Ordering::Relaxed);
    base >= 0 && st.num_laps > base
}

fn prev_rank(prev: &[i32], race_num: i32) -> usize {
    prev.iter().position(|&n| n == race_num).unwrap_or(usize::MAX)
}

/// True when `b`, currently scored behind `a`, is clearly up the track on them.
///
/// Only riders on the same lap are compared: across lap counts the classification is
/// right by definition, and two riders straddling the line always read a lap apart.
fn passed(s: &Snapshot, prev: &[i32], a: &Standing, b: &Standing) -> bool {
    if a.num_laps != b.num_laps || out_of_race(a) || out_of_race(b) {
        return false;
    }
    if done_racing(a) && done_racing(b) {
        return false;
    }
    let (Some(pa), Some(pb)) = (rider_lap_pos(s, a.race_num), rider_lap_pos(s, b.race_num)) else {
        return false;
    };
    let ahead_m = wrap_signed(pb - pa) * s.track_length;
    if ahead_m.abs() > PAIR_MAX_M {
        return false;
    }
    let held = prev_rank(prev, b.race_num) < prev_rank(prev, a.race_num);
    ahead_m > if held { HOLD_M } else { PASS_M }
}

/// Indices into `s.standings[..n]`, P1 first: the game classification with every pass we
/// can see on track applied on top. Rebuilt from the game order each tick so a bad swap
/// cannot stick, and bubbled so a rider can gain several places through a pack.
fn live_order(s: &Snapshot, clock: &SessionClock, n: usize) -> Vec<usize> {
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|&i| {
        let p = s.standings[i].position;
        if p > 0 {
            p
        } else {
            i as i32 + MAX_STANDINGS as i32
        }
    });
    if n > 1 && live_order_active(s, clock) {
        let prev = LIVE_ORDER.lock().map(|g| g.clone()).unwrap_or_default();
        for _ in 0..n {
            let mut moved = false;
            for i in 0..n - 1 {
                if passed(s, &prev, &s.standings[order[i]], &s.standings[order[i + 1]]) {
                    order.swap(i, i + 1);
                    moved = true;
                }
            }
            if !moved {
                break;
            }
        }
    }
    if let Ok(mut g) = LIVE_ORDER.lock() {
        g.clear();
        g.extend(order.iter().map(|&i| s.standings[i].race_num));
    }
    order
}

/// Live place for a race number, `0` when we have no order yet.
pub fn live_position(race_num: i32) -> i32 {
    if race_num <= 0 {
        return 0;
    }
    LIVE_ORDER
        .lock()
        .ok()
        .and_then(|g| g.iter().position(|&n| n == race_num))
        .map(|i| i as i32 + 1)
        .unwrap_or(0)
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
    let focus_st = s.standings[..n].iter().find(|r| r.race_num == focus_num).copied();

    for (i, &si) in order.iter().enumerate() {
        let mut st = s.standings[si];
        // Live place, so a pass shows on every board without waiting for the game to
        // republish its classification at the line.
        st.position = i as i32 + 1;
        let is_focus = st.race_num == focus_num;
        let is_leader = i == 0;
        if is_focus {
            focus = Some(i);
        }
        if is_leader {
            leader = Some(i);
        }
        // Gaps are still the game's, so a fresh pass can leave the pair's gap to the
        // leader the wrong way round for a moment: interval is a size, never negative.
        let (interval_ms, interval_laps) = if i == 0 {
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
        let (gap_to_focus_ms, gap_to_focus_laps) = match focus_st {
            Some(f) => (st.gap_ms - f.gap_ms, st.gap_laps - f.gap_laps),
            None => (0, 0),
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

fn build_clock(s: &Snapshot) -> SessionClock {
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
    let flag = timed_race_flag(s);
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
    note_session(s);
    if s.on_track == 0 {
        CHECKERED_LATCH.store(0, Ordering::Relaxed);
        return RaceFlag::None;
    }
    if is_lap_race(s) || timed_clock_live(s) || prestart(s) || !overtime_active(s) {
        return RaceFlag::None;
    }
    if CHECKERED_LATCH.load(Ordering::Relaxed) == 1 {
        return RaceFlag::Checkered;
    }
    // Crossing the line after the leader finished ends your race a lap down.
    if race_over_for_me(s) && finish_earned(s) {
        CHECKERED_LATCH.store(1, Ordering::Relaxed);
        return RaceFlag::Checkered;
    }
    if !extras_started(s) {
        return RaceFlag::None;
    }
    let left = laps_left(s).unwrap_or(1);
    note_laps_to_run(s, Some(left));
    if left == 0 && finish_earned(s) {
        CHECKERED_LATCH.store(1, Ordering::Relaxed);
        RaceFlag::Checkered
    } else if left <= 1 || leader_finished(s) {
        // Your last remaining extra, or the leader is already done.
        RaceFlag::White
    } else {
        RaceFlag::None
    }
}

impl RaceStore {
    /// Fill `VIEW` without cloning. Overlay `draw` uses this, then [`with`].
    pub fn refresh(s: &Snapshot) {
        // Clock first, and only it may mutate session state: the field reads the result.
        let clock = build_clock(s);
        let field = build_field(s, &clock);
        if let Ok(mut g) = VIEW.lock() {
            g.clock = clock;
            g.field = field;
        }
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

pub(crate) fn leftover_practice_len(ms: i32) -> bool {
    ms >= 30 * 60_000
}

pub(crate) fn leftover_warmup_len(ms: i32) -> bool {
    leftover_practice_len(ms) || standard_race_minutes(ms)
}

pub(crate) fn race_clock_ms(clock: i32) -> bool {
    clock >= 5 * 60_000 && clock <= 30 * 60_000
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
    matches!(session_len_minutes(ms), 5 | 6 | 8 | 10 | 12 | 15 | 20 | 25 | 30)
}

pub(crate) fn effective_session_len_ms(s: &Snapshot) -> i32 {
    let total = session_len_ms(s.session_length);
    if s.session_laps >= 4 && leftover_warmup_len(total) {
        return 0;
    }
    // Leftover 40+ min practice must not cap a race. 30:00 is a real moto length.
    if s.session_laps > 0 && leftover_practice_len(total) && !standard_race_minutes(total) {
        return 0;
    }
    // Leftover start board (~50s) must not cap a live 5–30 min timed +N countdown.
    note_timed_extras_hint(s);
    if s.session_laps > 0
        && s.session_laps < 4
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
/// Start/finish position in ten-thousandths of a lap, learned from your own crossings.
/// `-1` until confirmed; beats `sf_meters`, which is 0 without a centerline.
pub(crate) static SF_FRAC_LEARNED: AtomicI32 = AtomicI32::new(-1);
/// First unconfirmed sighting of the line. Two crossings must agree before we trust it,
/// so a rejoin or a stray lap increment cannot move the flag window.
pub(crate) static SF_FRAC_CAND: AtomicI32 = AtomicI32::new(-1);
/// Your completed laps last frame — detects a crossing for the S/F calibration.
pub(crate) static SF_LEARN_LAPS: AtomicI32 = AtomicI32::new(-1);
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
/// Sticky: 1–3 "laps" field is timed extras once we see a 5–30 min race clock / length.
/// Stops 6:00+2 with unset/start-board length flipping to a 2-lap moto after the gate.
pub(crate) static TIMED_EXTRAS_HINT: AtomicI32 = AtomicI32::new(0);
/// The clock we were counting down from when it dropped a long way in one frame, so the
/// climb back out of a republished start board is not read as the race clock expiring.
/// `-1` when the clock is running normally.
static DIP_FROM_CLOCK: AtomicI32 = AtomicI32::new(-1);

pub(crate) fn reset_session_clock_track() {
    LAST_SESSION_CLOCK.store(0, Ordering::Relaxed);
    DIP_FROM_CLOCK.store(-1, Ordering::Relaxed);
    SESSION_CLOCK_MODE.store(0, Ordering::Relaxed);
    SAW_SESSION_TIME.store(0, Ordering::Relaxed);
    SESSION_EXPIRED.store(0, Ordering::Relaxed);
    OVERTIME_BASE_LAP.store(-1, Ordering::Relaxed);
    OVERTIME_LOCAL_BASE.store(-1, Ordering::Relaxed);
    CACHED_SESSION_LAPS.store(0, Ordering::Relaxed);
    CHECKERED_LATCH.store(0, Ordering::Relaxed);
    WHITE_WAVE_LAP.store(-1, Ordering::Relaxed);
    RUN_IN_FLAG.store(0, Ordering::Relaxed);
    LEADER_FIN_LOCAL_BASE.store(-1, Ordering::Relaxed);
    SF_FRAC_LEARNED.store(-1, Ordering::Relaxed);
    SF_FRAC_CAND.store(-1, Ordering::Relaxed);
    SF_LEARN_LAPS.store(-1, Ordering::Relaxed);
    LAST_SF_METERS.store(-1, Ordering::Relaxed);
    LAP_MID_SEEN.store(0, Ordering::Relaxed);
    CLOSING_ON_LINE.store(0, Ordering::Relaxed);
    LAPS_TO_RUN_AT.store(-1, Ordering::Relaxed);
    IN_GATE.store(0, Ordering::Relaxed);
    POST_GATE.store(0, Ordering::Relaxed);
    LAP_GREEN.store(0, Ordering::Relaxed);
    LOCKED_SESSION_LEN.store(0, Ordering::Relaxed);
    RACE_ARMED.store(0, Ordering::Relaxed);
    LAST_SESSION_LAPS.store(-1, Ordering::Relaxed);
    LAST_RAW_SESSION_LEN.store(0, Ordering::Relaxed);
    LAST_SESSION_KIND.store(0, Ordering::Relaxed);
    TIMED_EXTRAS_HINT.store(0, Ordering::Relaxed);
    if let Ok(mut g) = LIVE_ORDER.lock() {
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

fn note_timed_extras_hint(s: &Snapshot) {
    let laps = s.session_laps;
    if !(1..=3).contains(&laps) {
        return;
    }
    let total = session_len_ms(s.session_length);
    if standard_race_minutes(total) {
        TIMED_EXTRAS_HINT.store(1, Ordering::Relaxed);
        return;
    }
    // Only when length is unset or a leftover start board — not a 7-min 3-lap leftover.
    let ambiguous = total <= 0 || total < 3 * 60_000;
    if !ambiguous {
        return;
    }
    let clock = s.session_time_ms.max(0);
    let last = LAST_SESSION_CLOCK.load(Ordering::Relaxed);
    let saw_race_clock = race_clock_ms(clock) || race_clock_ms(last);
    if !saw_race_clock && RACE_ARMED.load(Ordering::Relaxed) == 0 {
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
    // 10–30 min with unpublished extras looks like warmup/practice. When +1 appears on
    // that same timed race, keep overtime bases — a reset sits on `0/1` and can wave
    // the checkered while you still have laps to run.
    let timed_extras_arriving = prev_laps == 0
        && (1..=3).contains(&laps)
        && standard_race_minutes(prev_len_ms)
        && session_len_ms(s.session_length) == prev_len_ms
        && (RACE_ARMED.load(Ordering::Relaxed) == 1
            || SESSION_EXPIRED.load(Ordering::Relaxed) == 1);
    let entered_race = prev_laps == 0 && laps > 0 && from_practice && !timed_extras_arriving;
    let kind_changed = prev_laps > 0 && laps > 0 && (prev_laps >= 4) != (laps >= 4);
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
    if overtime_active(s) || is_lap_race(s) {
        return false;
    }
    // 1–3 is extras on a timed moto, not practice.
    if s.session_laps > 0 && s.session_laps < 4 {
        return false;
    }
    // A race session at 10–30 min with unpublished extras is not warmup.
    if is_race_session_kind(s.session_kind) {
        return false;
    }
    let total = session_len_ms(s.session_length);
    leftover_practice_len(total) || matches!(session_len_minutes(total), 10 | 12 | 15 | 20)
}

pub(crate) fn is_lap_race(s: &Snapshot) -> bool {
    // Extra laps are 1–3 on a timed set. Four or more is always a lap moto,
    // even when leftover warmup (10:00) is still sitting in session length.
    if s.session_laps >= 4 {
        return true;
    }
    if s.session_laps <= 1 {
        return false;
    }
    note_timed_extras_hint(s);
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
    // gate / leftover start board, not a 5–30 min clock, until green.
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
    format!("{} / {}", race_lap(s).min(n), n)
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

pub(crate) fn overtime_base(s: &Snapshot) -> i32 {
    let mut base = OVERTIME_BASE_LAP.load(Ordering::Relaxed);
    let local = focus_num_laps(s);
    let lead = leader_num_laps(s);
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

pub(crate) fn local_overtime_taken(s: &Snapshot) -> i32 {
    let _ = local_overtime_done(s);
    if !extras_started(s) {
        return 0;
    }
    let local = focus_num_laps(s);
    let local0 = OVERTIME_LOCAL_BASE.load(Ordering::Relaxed);
    if local <= 0 || local0 < 0 {
        return 0;
    }
    (local - local0).max(0)
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

pub(crate) fn clock_held_for(clock: i32, last: i32, secs: f32, hold: &Mutex<Option<(i32, f32)>>) -> bool {
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
        let counting_down = last > 0
            && clock < last
            && last - clock < 5_000
            && clock <= 180_000;
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
    // Game republishes the 8s/10s board during a live race; keep the real remaining clock.
    if armed
        && last > 60_000
        && clock >= 8_000
        && clock <= 15_000
        && clock + 20_000 < last
    {
        clock = last;
    }
    // A start board can land anywhere, not just the 8–15s window: 04:43 → 00:05 → 04:42
    // has been seen mid-moto. Remember where such a dip started, because the way back up
    // out of it is the frame that otherwise reads as the clock having run out.
    let dip_from = DIP_FROM_CLOCK.load(Ordering::Relaxed);
    if dip_from < 0 && armed && !in_gate_now && last > 0 && clock > 0 && last - clock > 30_000 {
        DIP_FROM_CLOCK.store(last, Ordering::Relaxed);
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
    // The dip itself still reaches the dash: in one frame it is not tellable from a clock
    // that genuinely ran down that far. Only the climb back out gives it away.
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
    if !board_dip
        && saw
        && armed
        && last > 0
        && last + 30_000 < total
        && near_full
        && clock > last + 20_000
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
    let short_clock = clock > 0 && clock <= 35_000 && (total <= 0 || clock * 3 < total) && total > 180_000;
    let enter_gate =
        is_gate_clock(clock, total) && !moving(s) && !post && !armed && s.session_laps > 0;
    let gate_clock = enter_gate
        || (!post && in_gate && clock > 0 && clock <= 180_000 && (total <= 0 || clock * 3 < total));
    // Frozen 00:30 wait is a race board. Practice remaining that hitch-pauses is not.
    let held_board = short_clock && board_held(clock, last) && !armed && s.session_laps > 0;
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
    let board_restart = in_gate && !post && last > 0 && clock > last + 2_000 && clock <= 180_000;
    // A later 45s/30s board after 00:10 must stay a countdown. Don't swap in leftover 08:00
    // until we've actually seen the race clock tick (Maryland 4-lap / 8:00 leftover).
    let hold_gate_board = gate_clock
        && !moving(s)
        && !armed
        && SAW_SESSION_TIME.load(Ordering::Relaxed) == 0;
    let gate = gate_clock && !drop_off_gate && !armed && (!board_restart || hold_gate_board);
    let race_ticking = !gate_clock
        && last > 180_000
        && clock > 5_000
        && clock < last
        && last - clock >= 50
        && last - clock < 5_000;
    let waiting_for_race = (post || board_restart)
        && !armed
        && !race_ticking
        && clock <= 180_000
        && !hold_gate_board;

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
            CHECKERED_LATCH.store(0, Ordering::Relaxed);
            return Some(remain);
        }
        let _ = overtime_base(s);
        return Some(0);
    }

    let timed_out = remain <= 800
        || (clock_stuck(clock, last) && remain > 0 && remain <= 2_000);
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
    // Warmup / practice: once the countdown hits zero, stay blank (ignore 00:30 junk).
    if s.session_laps <= 0
        && !gate
        && SAW_SESSION_TIME.load(Ordering::Relaxed) == 1
        && (remain <= 800 || (started && timed_out))
    {
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

/// 5–30 min race clock already expired, extras field still 0. MX Bikes often
/// publishes `session_laps = 1` a lap later and resets standings when it does.
fn timed_race_awaiting_extras(s: &Snapshot) -> bool {
    RACE_ARMED.load(Ordering::Relaxed) == 1
        && SESSION_EXPIRED.load(Ordering::Relaxed) == 1
        && !is_lap_race(s)
        && s.session_laps <= 0
        && standard_race_minutes(session_len_ms(s.session_length))
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
    s.standings
        .iter()
        .take(s.standing_count.max(0) as usize)
        .find(|row| row.race_num == focus)
        .map(|row| row.num_laps)
        .unwrap_or_else(|| {
            if s.current_lap > 0 {
                (s.current_lap - 1).max(0)
            } else {
                0
            }
        })
        .max(0)
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
    if LEADER_FIN_LOCAL_BASE.load(Ordering::Relaxed) >= 0 || !leader_finished(s) {
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
/// Counted against `OVERTIME_LOCAL_BASE` because the banner's first extra is the crossing
/// that ends the lap the leader's extras started on.
pub(crate) fn effective_extra_laps(s: &Snapshot) -> i32 {
    let n = extra_laps(s).max(1);
    note_leader_finish(s);
    let base = LEADER_FIN_LOCAL_BASE.load(Ordering::Relaxed);
    let local0 = OVERTIME_LOCAL_BASE.load(Ordering::Relaxed);
    if base < 0 || local0 < 0 || !extras_started(s) {
        return n;
    }
    (base + 1 - local0).clamp(1, n)
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
    if race_over_for_me(s) {
        return Some(0);
    }
    if prestart(s) || timed_clock_live(s) {
        return None;
    }
    if lap_race_racing(s) {
        // Effective, so a lapped rider counts down to the shorter race they will run and
        // the flags fall out of the same lap count as everyone else.
        return Some((effective_race_laps(s) - laps_done(s)).max(0));
    }
    if overtime_active(s) {
        note_overtime_base(s);
        // Until the leader crosses after time expiry you are still on the timed lap,
        // so how many laps remain is not decided yet.
        if !extras_started(s) {
            return None;
        }
        // Count down to the lap total you finish on. Deriving this from
        // `local_overtime_done` instead would lose a lap: its `max(0)` clamp reports 0
        // both on the lap that does not count and on your first extra.
        let local0 = OVERTIME_LOCAL_BASE.load(Ordering::Relaxed);
        if local0 < 0 {
            return None;
        }
        let target = local0 + 1 + extra_laps(s).max(1);
        return Some((target - focus_num_laps(s)).max(0));
    }
    None
}

/// Your lap count while there were still laps to run. Tick once per frame.
pub(crate) fn note_laps_to_run(s: &Snapshot, left: Option<i32>) {
    if left.is_some_and(|n| n > 0) {
        LAPS_TO_RUN_AT.store(focus_num_laps(s), Ordering::Relaxed);
    }
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
        return ('\u{f11e}', lap_race_text(s));
    }
    if let Some(remain) = remain {
        if remain > 1000 && SESSION_EXPIRED.load(Ordering::Relaxed) == 0 {
            OVERTIME_BASE_LAP.store(-1, Ordering::Relaxed);
            OVERTIME_LOCAL_BASE.store(-1, Ordering::Relaxed);
        }
        if overtime_active(s) {
            return ('\u{f11e}', overtime_lap_text(s));
        }
        // Timed race over, extras not published yet. Don't look like warmup ended.
        if s.session_laps <= 0
            && (remain <= 0 || SESSION_EXPIRED.load(Ordering::Relaxed) == 1)
        {
            if timed_race_awaiting_extras(s) {
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
        ('\u{f11e}', overtime_lap_text(s))
    } else if s.session_laps > 0 {
        OVERTIME_BASE_LAP.store(-1, Ordering::Relaxed);
        OVERTIME_LOCAL_BASE.store(-1, Ordering::Relaxed);
        ('\u{f11e}', format!("{} / {}", race_lap(s), s.session_laps))
    } else if SESSION_EXPIRED.load(Ordering::Relaxed) == 1 {
        ('\u{f2f2}', String::new())
    } else {
        OVERTIME_BASE_LAP.store(-1, Ordering::Relaxed);
        OVERTIME_LOCAL_BASE.store(-1, Ordering::Relaxed);
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
    /// 0 none, 1 white, 2 checkered.
    pub flag: i32,
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
    let cat = cstr(&st.category);
    if cat.is_empty() {
        return overall;
    }
    ordered_standings(s)
        .iter()
        .filter(|row| cstr(&row.category) == cat)
        .position(|row| row.race_num == st.race_num)
        .map(|i| i as i32 + 1)
        .unwrap_or(overall)
}

pub(crate) fn focus_standing(s: &Snapshot) -> Option<&Standing> {
    let focus = if s.focus_race_num > 0 { s.focus_race_num } else { s.local_race_num };
    standing_of(s, focus)
}

/// A lap or more behind the leader in the classification. `gap_laps` counts laps behind
/// the leader, so the leader's own is 0. Gated on a running race because a practice or
/// warmup field has no leader to be a lap behind.
pub(crate) fn lapped(s: &Snapshot) -> bool {
    if s.on_track == 0 || is_warmup(s) || prestart(s) {
        return false;
    }
    focus_standing(s).is_some_and(|st| st.gap_laps >= 1)
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

pub(crate) fn ticker_delta_from_row(row: &RaceRow) -> String {
    format_signed_delta(row.gap_to_focus_ms, row.gap_to_focus_laps)
}

fn format_signed_delta(ms: i32, laps: i32) -> String {
    if laps != 0 {
        let sign = if laps > 0 { '+' } else { '-' };
        return format!("{sign}{}L", laps.abs());
    }
    let sec = ms as f32 / 1000.0;
    if sec.abs() >= 60.0 {
        let m = (sec.abs() / 60.0) as i32;
        let s = sec.abs() - m as f32 * 60.0;
        let sign = if ms < 0 { '-' } else { '+' };
        format!("{sign}{m}:{:04.1}", s)
    } else {
        format!("{sec:+.3}")
    }
}
