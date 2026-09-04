//! Live-then-freeze sector times vs your saved best at this point in the sector.

use crate::delta;
use crate::shm::{cstr, Snapshot};
use crate::track_pb::{self, BINS};
use std::sync::Mutex;

const CLOCK_ON: i32 = 200;
const SMOOTH: f32 = 0.12;
const HIST_MAX: usize = 5;
const HIST_LABELS: [&str; HIST_MAX] = ["LAST", "-2", "-3", "-4", "-5"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SectorRow {
    pub label: &'static str,
    pub time_ms: i32,
    pub delta_ms: i32,
    pub pending: bool,
    pub has_delta: bool,
    pub slower: bool,
    pub fresh: bool,
    pub live: bool,
}

struct SectorEngine {
    track: String,
    bike: String,
    saved: [i32; 3],
    bins: [i32; BINS],
    has_tape: bool,
    session_saved: [i32; 3],
    session_bins: [i32; BINS],
    session_has_tape: bool,
    split_end: [f32; 2],
    freeze_time: [i32; 3],
    freeze_delta: [i32; 3],
    freeze_ok: u8,
    freeze_delta_ok: u8,
    session_freeze_delta: [i32; 3],
    session_freeze_delta_ok: u8,
    last_cur: [i32; 3],
    last_clock: bool,
    smooth_ms: f32,
    smooth_i: i8,
    /// Completed laps, newest first: LAST, -2 … -5.
    hist: [[i32; 3]; HIST_MAX],
}

impl SectorEngine {
    const fn new() -> Self {
        Self {
            track: String::new(),
            bike: String::new(),
            saved: [0; 3],
            bins: [0; BINS],
            has_tape: false,
            session_saved: [0; 3],
            session_bins: [0; BINS],
            session_has_tape: false,
            split_end: [0.0; 2],
            freeze_time: [0; 3],
            freeze_delta: [0; 3],
            freeze_ok: 0,
            freeze_delta_ok: 0,
            session_freeze_delta: [0; 3],
            session_freeze_delta_ok: 0,
            last_cur: [0; 3],
            last_clock: false,
            smooth_ms: 0.0,
            smooth_i: -1,
            hist: [[0; 3]; HIST_MAX],
        }
    }
}

static STORE: Mutex<SectorEngine> = Mutex::new(SectorEngine::new());

fn live() -> std::sync::MutexGuard<'static, SectorEngine> {
    STORE.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn tick(s: &Snapshot) {
    if s.has_telemetry == 0 {
        return;
    }
    let track = cstr(&s.track_name).to_string();
    let mut g = live();
    if !track.is_empty() {
        let bike_now = track_pb::bike_class(&s.local_bike());
        let pb = track_pb::bind(&track, &bike_now);
        let bike = if bike_now.is_empty() {
            g.bike.clone()
        } else {
            bike_now
        };
        if track != g.track || bike != g.bike {
            let learned = track == g.track && g.bike.is_empty() && !bike.is_empty();
            g.track = track;
            g.bike = bike;
            if !learned {
                g.freeze_time = [0; 3];
                g.freeze_delta = [0; 3];
                g.freeze_ok = 0;
                g.freeze_delta_ok = 0;
                g.session_saved = [0; 3];
                g.session_bins = [0; BINS];
                g.session_has_tape = false;
                g.session_freeze_delta = [0; 3];
                g.session_freeze_delta_ok = 0;
                g.last_cur = [0; 3];
                g.last_clock = false;
                g.smooth_i = -1;
                g.hist = [[0; 3]; HIST_MAX];
            }
            g.split_end = [
                pb.split_milli[0] as f32 / 1000.0,
                pb.split_milli[1] as f32 / 1000.0,
            ];
        }
        g.saved = pb.sectors;
        g.bins = pb.bins;
        g.has_tape = pb.has_tape();
        infer_split_ends(&mut g);
    }
    if let Some((bins, _)) = delta::session_tape() {
        g.session_bins = bins;
        g.session_has_tape = true;
    }

    let clock = s.current_lap_ms > CLOCK_ON;
    let cur = [
        s.sector_cur.first().copied().unwrap_or(0),
        s.sector_cur.get(1).copied().unwrap_or(0),
        s.sector_cur.get(2).copied().unwrap_or(0),
    ];
    if clock && !g.last_clock && cur.iter().all(|&c| c <= 0) {
        g.freeze_time = [0; 3];
        g.freeze_delta = [0; 3];
        g.freeze_ok = 0;
        g.freeze_delta_ok = 0;
        g.session_freeze_delta = [0; 3];
        g.session_freeze_delta_ok = 0;
        g.smooth_i = -1;
    }
    for i in 0..3 {
        if cur[i] > 0 && g.last_cur[i] <= 0 {
            freeze_split(&mut g, s, i, cur);
        }
    }
    if g.last_clock && !clock {
        push_hist(
            &mut g,
            [
                last_lap_time(s, 0),
                last_lap_time(s, 1),
                last_lap_time(s, 2),
            ],
        );
    }
    g.last_cur = cur;
    g.last_clock = clock;
}

/// S1 and S2 *ends* as lap fraction. `0` means that split is not known yet.
pub fn split_fracs() -> [f32; 2] {
    live().split_end
}

/// Where each sector begins. S1 is the line (`0`). S2 / S3 are `0` until that split is known.
pub fn sector_starts() -> [(f32, &'static str); 3] {
    let e = live().split_end;
    [(0.0, "S1"), (e[0], "S2"), (e[1], "S3")]
}

/// Demo / tests: pin S1 / S2 without a saved tape.
pub fn set_split_fracs(ends: [f32; 2]) {
    live().split_end = ends;
}

#[cfg(test)]
pub(crate) fn reset_engine() {
    *live() = SectorEngine::new();
}

pub fn reload() {
    let mut g = live();
    if g.track.is_empty() {
        g.saved = [0; 3];
        g.has_tape = false;
        g.session_saved = [0; 3];
        g.session_has_tape = false;
        g.hist = [[0; 3]; HIST_MAX];
        return;
    }
    let pb = track_pb::bind(&g.track, &g.bike);
    g.saved = pb.sectors;
    g.bins = pb.bins;
    g.has_tape = pb.has_tape();
    g.split_end = [
        pb.split_milli[0] as f32 / 1000.0,
        pb.split_milli[1] as f32 / 1000.0,
    ];
    infer_split_ends(&mut g);
    g.freeze_time = [0; 3];
    g.freeze_delta = [0; 3];
    g.freeze_ok = 0;
    g.freeze_delta_ok = 0;
    g.session_saved = [0; 3];
    g.session_bins = [0; BINS];
    g.session_has_tape = false;
    g.session_freeze_delta = [0; 3];
    g.session_freeze_delta_ok = 0;
    g.smooth_i = -1;
    g.hist = [[0; 3]; HIST_MAX];
}

/// Which cell is wide: the sector you are in (or S3 after the line).
pub fn hero_index(s: &Snapshot) -> i32 {
    let clock = s.current_lap_ms > CLOCK_ON;
    let c0 = s.sector_cur.first().copied().unwrap_or(0) > 0;
    let c1 = s.sector_cur.get(1).copied().unwrap_or(0) > 0;
    if clock {
        if !c0 {
            return 0;
        }
        if !c1 {
            return 1;
        }
        return 2;
    }
    if last_lap_time(s, 2) > 0 || last_lap_time(s, 0) > 0 {
        2
    } else {
        0
    }
}

pub fn row(s: &Snapshot, i: usize, live_on: bool) -> SectorRow {
    row_vs(s, i, live_on, false)
}

pub fn row_vs(s: &Snapshot, i: usize, live_on: bool, session: bool) -> SectorRow {
    const LABELS: [&str; 3] = ["S1", "S2", "S3"];
    let label = LABELS.get(i).copied().unwrap_or("S?");
    let hero = hero_index(s) == i as i32;
    let clock = s.current_lap_ms > CLOCK_ON;
    let cur = s.sector_cur.get(i).copied().unwrap_or(0);
    let mut g = live();
    let frozen = !g.track.is_empty()
        && cstr(&s.track_name) == g.track
        && (g.freeze_ok & (1 << i)) != 0;
    let cur_all = [
        s.sector_cur.first().copied().unwrap_or(0),
        s.sector_cur.get(1).copied().unwrap_or(0),
        s.sector_cur.get(2).copied().unwrap_or(0),
    ];
    let (time_ms, delta_ms, has_delta, is_live) = if clock && hero && cur <= 0 {
        if !live_on {
            (0, 0, false, false)
        } else {
            let t = live_elapsed(s, i);
            let pos = delta::lap_pos(s);
            match vs_location(&g, s, i, t, pos, session) {
                Some(raw) => {
                    let d = smooth_live(&mut g, i, raw);
                    let best = compare_best(&g, s, i, session);
                    (best, d, true, true)
                }
                None => {
                    let best = compare_best(&g, s, i, session);
                    (best, 0, false, true)
                }
            }
        }
    } else if frozen {
        let t = g.freeze_time[i];
        let (d, has_d) = if session {
            (
                g.session_freeze_delta[i],
                (g.session_freeze_delta_ok & (1 << i)) != 0,
            )
        } else {
            (g.freeze_delta[i], (g.freeze_delta_ok & (1 << i)) != 0)
        };
        (t, d, has_d, false)
    } else if cur > 0 {
        let t = split_duration(i, cur_all, s.last_lap_ms);
        if !session && plugin_has_delta(s, i) {
            (t, plugin_delta(s, i), true, false)
        } else {
            let best = compare_best(&g, s, i, session);
            let has = t > 0 && best > 0;
            (t, if has { t - best } else { 0 }, has, false)
        }
    } else if !clock {
        let t = last_lap_time(s, i);
        if !session && plugin_has_delta(s, i) {
            (t, plugin_delta(s, i), t > 0, false)
        } else {
            let best = compare_best(&g, s, i, session);
            let has = t > 0 && best > 0;
            (t, if has { t - best } else { 0 }, has, false)
        }
    } else {
        (0, 0, false, false)
    };
    drop(g);
    SectorRow {
        label,
        time_ms,
        delta_ms,
        pending: !is_live && time_ms <= 0,
        has_delta,
        slower: has_delta && delta_ms > 0,
        fresh: hero,
        live: is_live,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HistoryCell {
    pub time_ms: i32,
    pub has_compare: bool,
    pub slower: bool,
    pub faster: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HistoryRow {
    pub label: &'static str,
    pub cells: [HistoryCell; 3],
    /// You-row gold: this lap's total is the fastest in the log (and vs saved/session best).
    pub fastest: bool,
}

/// Demo / tests: pin LAST … -5 completed laps.
pub fn set_history(laps: [[i32; 3]; HIST_MAX]) {
    live().hist = laps;
}

pub fn history_times() -> [[i32; 3]; HIST_MAX] {
    live().hist
}

/// Newest completed laps vs the same comparison as the live strip. `n` is 1..=5.
pub fn history_board(s: &Snapshot, session: bool, n: usize) -> Vec<HistoryRow> {
    let n = n.clamp(1, HIST_MAX);
    let g = live();
    let mut laps = g.hist;
    if laps[0].iter().all(|&t| t <= 0) {
        laps[0] = [
            last_lap_time(s, 0),
            last_lap_time(s, 1),
            last_lap_time(s, 2),
        ];
    }
    let best = [
        compare_best(&g, s, 0, session),
        compare_best(&g, s, 1, session),
        compare_best(&g, s, 2, session),
    ];
    let totals: Vec<i32> = laps.iter().map(|lap| lap_total(*lap)).collect();
    let best_lap = totals.iter().copied().filter(|&t| t > 0).min().unwrap_or(0);
    (0..n)
        .map(|row| {
            let cells = std::array::from_fn(|i| {
                let t = laps[row][i];
                let has = t > 0 && best[i] > 0;
                HistoryCell {
                    time_ms: t,
                    has_compare: has,
                    slower: has && t > best[i],
                    faster: has && t < best[i],
                }
            });
            HistoryRow {
                label: HIST_LABELS[row],
                cells,
                fastest: best_lap > 0 && totals[row] == best_lap,
            }
        })
        .collect()
}

fn lap_total(lap: [i32; 3]) -> i32 {
    if lap.iter().any(|&t| t <= 0) {
        0
    } else {
        lap[0].saturating_add(lap[1]).saturating_add(lap[2])
    }
}

fn push_hist(g: &mut SectorEngine, lap: [i32; 3]) {
    if lap.iter().any(|&t| t <= 0) {
        return;
    }
    if g.hist[0] == lap {
        return;
    }
    g.hist.copy_within(0..HIST_MAX - 1, 1);
    g.hist[0] = lap;
}

fn freeze_split(g: &mut SectorEngine, s: &Snapshot, i: usize, cur: [i32; 3]) {
    let dur = split_duration(i, cur, s.last_lap_ms);
    if dur <= 0 {
        return;
    }
    let pos = delta::lap_pos(s);
    if i < 2 && (0.04..0.96).contains(&pos) {
        g.split_end[i] = pos;
        if !g.track.is_empty() {
            track_pb::note_split_pos(&g.track, &g.bike, i, pos);
        }
    }
    g.freeze_time[i] = dur;
    g.freeze_ok |= 1 << i;
    g.smooth_i = -1;
    write_freeze_delta(g, s, i, dur, false);
    write_freeze_delta(g, s, i, dur, true);
    if !g.track.is_empty() && track_pb::commit_sector(&g.track, &g.bike, i, dur) {
        g.saved[i] = dur;
    }
    if g.session_saved[i] <= 0 || dur < g.session_saved[i] {
        g.session_saved[i] = dur;
    }
    if i == 2 && g.freeze_time.iter().all(|&t| t > 0) {
        push_hist(g, g.freeze_time);
    }
}

fn write_freeze_delta(g: &mut SectorEngine, s: &Snapshot, i: usize, dur: i32, session: bool) {
    let best = compare_best(g, s, i, session);
    let d = (best > 0).then_some(dur - best);
    if session {
        if let Some(d) = d {
            g.session_freeze_delta[i] = d;
            g.session_freeze_delta_ok |= 1 << i;
        } else {
            g.session_freeze_delta[i] = 0;
            g.session_freeze_delta_ok &= !(1 << i);
        }
    } else if let Some(d) = d {
        g.freeze_delta[i] = d;
        g.freeze_delta_ok |= 1 << i;
    } else {
        g.freeze_delta[i] = 0;
        g.freeze_delta_ok &= !(1 << i);
    }
}

fn compare_best(g: &SectorEngine, s: &Snapshot, i: usize, session: bool) -> i32 {
    if session {
        g.session_saved[i]
    } else if g.saved[i] > 0 {
        g.saved[i]
    } else {
        s.sector_best.get(i).copied().unwrap_or(0)
    }
}

fn infer_split_ends(g: &mut SectorEngine) {
    if !g.has_tape {
        return;
    }
    if g.split_end[0] < 0.04 && g.saved[0] > 0 {
        if let Some(p) = track_pb::pos_at_time(&g.bins, g.saved[0]) {
            g.split_end[0] = p;
        }
    }
    if g.split_end[1] < 0.04 && g.saved[0] > 0 && g.saved[1] > 0 {
        if let Some(p) = track_pb::pos_at_time(&g.bins, g.saved[0].saturating_add(g.saved[1])) {
            g.split_end[1] = p;
        }
    }
}

fn enter_pos(g: &SectorEngine, i: usize) -> f32 {
    match i {
        1 => g.split_end[0],
        2 => g.split_end[1],
        _ => 0.0,
    }
}

fn vs_location(
    g: &SectorEngine,
    s: &Snapshot,
    i: usize,
    elapsed: i32,
    pos: f32,
    session: bool,
) -> Option<i32> {
    if elapsed <= CLOCK_ON || pos < 0.0 {
        return None;
    }
    let (has_tape, bins) = if session {
        (g.session_has_tape, &g.session_bins)
    } else {
        (g.has_tape, &g.bins)
    };
    if has_tape {
        let now = track_pb::time_at(bins, pos)?;
        let p0 = enter_pos(g, i);
        let ref0 = if p0 <= 0.001 {
            0
        } else {
            track_pb::time_at(bins, p0).unwrap_or(0)
        };
        return Some(elapsed - (now - ref0));
    }
    vs_linear(g, s, i, elapsed, pos, session)
}

fn vs_linear(
    g: &SectorEngine,
    s: &Snapshot,
    i: usize,
    elapsed: i32,
    pos: f32,
    session: bool,
) -> Option<i32> {
    let best = compare_best(g, s, i, session);
    if best <= 0 {
        return None;
    }
    let (p0, p1) = match i {
        0 if g.split_end[0] > 0.05 => (0.0, g.split_end[0]),
        1 if g.split_end[0] > 0.05 && g.split_end[1] > 0.05 => (g.split_end[0], g.split_end[1]),
        2 if g.split_end[1] > 0.05 => (g.split_end[1], 1.0),
        _ => return None,
    };
    let span = lap_along(p0, p1);
    if span < 0.04 {
        return None;
    }
    let frac = (lap_along(p0, pos) / span).clamp(0.02, 1.2);
    let expected = (best as f32 * frac).round() as i32;
    Some(elapsed - expected)
}

/// Forward distance along the lap, wrapping at the centerline origin.
fn lap_along(from: f32, to: f32) -> f32 {
    (to - from).rem_euclid(1.0)
}

fn smooth_live(g: &mut SectorEngine, i: usize, raw: i32) -> i32 {
    let i = i as i8;
    if g.smooth_i != i {
        g.smooth_ms = raw as f32;
        g.smooth_i = i;
        return raw;
    }
    g.smooth_ms += (raw as f32 - g.smooth_ms) * SMOOTH;
    g.smooth_ms.round() as i32
}

fn plugin_delta(s: &Snapshot, i: usize) -> i32 {
    s.sector_delta.get(i).copied().unwrap_or(0)
}

fn plugin_has_delta(s: &Snapshot, i: usize) -> bool {
    (s.sector_delta_valid & (1 << i)) != 0
}

fn live_elapsed(s: &Snapshot, i: usize) -> i32 {
    let t = delta::lap_clock(s);
    if t <= 0 {
        return 0;
    }
    let s1 = s.sector_cur.first().copied().unwrap_or(0);
    let s2 = s.sector_cur.get(1).copied().unwrap_or(0);
    match i {
        0 => t,
        1 => (t - s1).max(0),
        2 => (t - time_to_s2(s1, s2)).max(0),
        _ => 0,
    }
}

fn last_lap_time(s: &Snapshot, i: usize) -> i32 {
    let last = s.sector_last_lap.get(i).copied().unwrap_or(0);
    if last > 0 {
        return last;
    }
    if i == 2 {
        return three_ms(
            s.sector_last_lap.first().copied().unwrap_or(0),
            s.sector_last_lap.get(1).copied().unwrap_or(0),
            s.last_lap_ms,
        );
    }
    0
}

fn split_duration(i: usize, cur: [i32; 3], last_lap_ms: i32) -> i32 {
    match i {
        0 => cur[0],
        1 => {
            if cur[0] > 0 && cur[1] > cur[0] + cur[0] / 2 {
                cur[1] - cur[0]
            } else {
                cur[1]
            }
        }
        2 => {
            if cur[2] > 0 {
                cur[2]
            } else {
                three_ms(cur[0], cur[1], last_lap_ms)
            }
        }
        _ => 0,
    }
}

fn time_to_s2(s1: i32, s2: i32) -> i32 {
    if s1 > 0 && s2 > s1 + s1 / 2 {
        s2
    } else {
        s1.saturating_add(s2)
    }
}

pub fn three_ms(s1: i32, s2: i32, lap: i32) -> i32 {
    if s1 <= 0 || s2 <= 0 || lap <= 0 {
        return 0;
    }
    let dur = lap - s1 - s2;
    let cum = lap - s2;
    if dur <= 0 {
        return cum.max(0);
    }
    if cum <= 0 {
        return dur;
    }
    let d_dur = (dur - s1).abs();
    let d_cum = (cum - s1).abs();
    if d_dur <= d_cum {
        dur
    } else {
        cum
    }
}

#[cfg(test)]
#[path = "tests/sector.rs"]
mod tests;
