//! Live-then-freeze sector times vs your saved best at this point in the sector.

use crate::delta;
use crate::shm::{cstr, Snapshot};
use crate::track_pb::{self, BINS};
use std::sync::Mutex;

const CLOCK_ON: i32 = 200;
const SMOOTH: f32 = 0.12;

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
    key: String,
    saved: [i32; 3],
    bins: [i32; BINS],
    has_tape: bool,
    split_end: [f32; 2],
    freeze_time: [i32; 3],
    freeze_delta: [i32; 3],
    freeze_ok: u8,
    freeze_delta_ok: u8,
    last_cur: [i32; 3],
    last_clock: bool,
    smooth_ms: f32,
    smooth_i: i8,
}

impl SectorEngine {
    const fn new() -> Self {
        Self {
            key: String::new(),
            saved: [0; 3],
            bins: [0; BINS],
            has_tape: false,
            split_end: [0.0; 2],
            freeze_time: [0; 3],
            freeze_delta: [0; 3],
            freeze_ok: 0,
            freeze_delta_ok: 0,
            last_cur: [0; 3],
            last_clock: false,
            smooth_ms: 0.0,
            smooth_i: -1,
        }
    }
}

static STORE: Mutex<SectorEngine> = Mutex::new(SectorEngine::new());

fn live() -> std::sync::MutexGuard<'static, SectorEngine> {
    STORE.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn tick(s: &Snapshot) {
    let key = cstr(&s.track_name).to_string();
    let mut g = live();
    if !key.is_empty() {
        let pb = track_pb::bind(&key);
        if key != g.key {
            g.key = key;
            g.freeze_time = [0; 3];
            g.freeze_delta = [0; 3];
            g.freeze_ok = 0;
            g.freeze_delta_ok = 0;
            g.last_cur = [0; 3];
            g.last_clock = false;
            g.smooth_i = -1;
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
        g.smooth_i = -1;
    }
    for i in 0..3 {
        if cur[i] > 0 && g.last_cur[i] <= 0 {
            freeze_split(&mut g, s, i, cur);
        }
    }
    g.last_cur = cur;
    g.last_clock = clock;
}

/// S1 and S2 as lap fraction. `0` means that split is not known yet.
pub fn split_fracs() -> [f32; 2] {
    live().split_end
}

/// Demo / tests: pin S1 / S2 without a saved tape.
pub fn set_split_fracs(ends: [f32; 2]) {
    live().split_end = ends;
}

pub fn reload() {
    let mut g = live();
    if g.key.is_empty() {
        g.saved = [0; 3];
        g.has_tape = false;
        return;
    }
    let pb = track_pb::bind(&g.key);
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
    g.smooth_i = -1;
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
    const LABELS: [&str; 3] = ["S1", "S2", "S3"];
    let label = LABELS.get(i).copied().unwrap_or("S?");
    let hero = hero_index(s) == i as i32;
    let clock = s.current_lap_ms > CLOCK_ON;
    let cur = s.sector_cur.get(i).copied().unwrap_or(0);
    let mut g = live();
    let frozen = !g.key.is_empty()
        && cstr(&s.track_name) == g.key
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
            match vs_location(&g, s, i, t, pos) {
                Some(raw) => {
                    let d = smooth_live(&mut g, i, raw);
                    (t, d, t > CLOCK_ON, true)
                }
                None => (t, 0, false, true),
            }
        }
    } else if frozen {
        let t = g.freeze_time[i];
        let has_d = (g.freeze_delta_ok & (1 << i)) != 0;
        (t, g.freeze_delta[i], has_d, false)
    } else if cur > 0 {
        let t = split_duration(i, cur_all, s.last_lap_ms);
        if plugin_has_delta(s, i) {
            (t, plugin_delta(s, i), true, false)
        } else {
            let best = compare_best(&g, s, i);
            let has = t > 0 && best > 0;
            (t, if has { t - best } else { 0 }, has, false)
        }
    } else if !clock {
        let t = last_lap_time(s, i);
        if plugin_has_delta(s, i) {
            (t, plugin_delta(s, i), t > 0, false)
        } else {
            let best = compare_best(&g, s, i);
            let has = t > 0 && best > 0;
            (t, if has { t - best } else { 0 }, has, false)
        }
    } else {
        (0, 0, false, false)
    };
    drop(g);
    let showing = time_ms > 0;
    let has_delta = showing && has_delta;
    SectorRow {
        label,
        time_ms,
        delta_ms,
        pending: !showing,
        has_delta,
        slower: has_delta && delta_ms > 0,
        fresh: hero,
        live: is_live && showing,
    }
}

fn freeze_split(g: &mut SectorEngine, s: &Snapshot, i: usize, cur: [i32; 3]) {
    let dur = split_duration(i, cur, s.last_lap_ms);
    if dur <= 0 {
        return;
    }
    let pos = delta::lap_pos(s);
    if i < 2 && (0.04..0.96).contains(&pos) {
        g.split_end[i] = pos;
        if !g.key.is_empty() {
            track_pb::note_split_pos(&g.key, i, pos);
        }
    }
    g.freeze_time[i] = dur;
    g.freeze_ok |= 1 << i;
    g.smooth_i = -1;
    if let Some(d) = vs_location(g, s, i, dur, pos) {
        g.freeze_delta[i] = d;
        g.freeze_delta_ok |= 1 << i;
    } else {
        let best = compare_best(g, s, i);
        if best > 0 {
            g.freeze_delta[i] = dur - best;
            g.freeze_delta_ok |= 1 << i;
        } else {
            g.freeze_delta[i] = 0;
            g.freeze_delta_ok &= !(1 << i);
        }
    }
    if !g.key.is_empty() && track_pb::commit_sector(&g.key, i, dur) {
        g.saved[i] = dur;
    }
}

fn compare_best(g: &SectorEngine, s: &Snapshot, i: usize) -> i32 {
    if g.saved[i] > 0 {
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

fn vs_location(g: &SectorEngine, s: &Snapshot, i: usize, elapsed: i32, pos: f32) -> Option<i32> {
    if elapsed <= CLOCK_ON || pos < 0.0 {
        return None;
    }
    if g.has_tape {
        let now = track_pb::time_at(&g.bins, pos)?;
        let p0 = enter_pos(g, i);
        let ref0 = if p0 <= 0.001 {
            0
        } else {
            track_pb::time_at(&g.bins, p0).unwrap_or(0)
        };
        return Some(elapsed - (now - ref0));
    }
    vs_linear(g, s, i, elapsed, pos)
}

fn vs_linear(g: &SectorEngine, s: &Snapshot, i: usize, elapsed: i32, pos: f32) -> Option<i32> {
    let best = compare_best(g, s, i);
    if best <= 0 {
        return None;
    }
    let (p0, p1) = match i {
        0 if g.split_end[0] > 0.05 => (0.0, g.split_end[0]),
        1 if g.split_end[0] > 0.05 && g.split_end[1] > g.split_end[0] + 0.05 => {
            (g.split_end[0], g.split_end[1])
        }
        2 if g.split_end[1] > 0.05 && g.split_end[1] < 0.95 => (g.split_end[1], 1.0),
        _ => return None,
    };
    let span = p1 - p0;
    if span < 0.04 {
        return None;
    }
    let frac = ((pos - p0) / span).clamp(0.02, 1.2);
    let expected = (best as f32 * frac).round() as i32;
    Some(elapsed - expected)
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
    let t = s.current_lap_ms;
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
mod tests {
    use super::*;
    use crate::shm::write_name;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn snap() -> Snapshot {
        let mut s = Snapshot::default();
        s.on_track = 1;
        s.has_telemetry = 1;
        s.sector_count = 3;
        s.track_length = 1_600.0;
        write_name(&mut s.track_name, "LiveTrack");
        s
    }

    const LAP: i32 = 72_000;
    const S1: i32 = 24_000;

    fn linear_bins(lap: i32) -> [i32; BINS] {
        let mut b = [0; BINS];
        for i in 0..BINS {
            b[i] = (lap as i64 * i as i64 / BINS as i64) as i32;
            if i > 0 && b[i] <= 0 {
                b[i] = 1;
            }
        }
        b[0] = 1;
        b
    }

    fn seed_tape() {
        track_pb::bind("LiveTrack");
        track_pb::commit_tape("LiveTrack", LAP, linear_bins(LAP));
        track_pb::commit_sector("LiveTrack", 0, S1);
        track_pb::commit_sector("LiveTrack", 1, S1);
        track_pb::commit_sector("LiveTrack", 2, S1);
    }

    fn pos_for(ms: i32) -> f32 {
        ms as f32 / LAP as f32
    }

    fn tape_at(pos: f32) -> i32 {
        track_pb::time_at(&linear_bins(LAP), pos).unwrap()
    }

    fn tmp() -> std::sync::MutexGuard<'static, ()> {
        let lock = track_pb::exclusive_test();
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("mxbo-sec-{n}"));
        track_pb::set_store_dir(dir);
        *live() = SectorEngine::new();
        lock
    }

    #[test]
    fn split_fracs_can_be_pinned() {
        let _lock = tmp();
        set_split_fracs([0.31, 0.64]);
        assert!((split_fracs()[0] - 0.31).abs() < 1e-6);
        assert!((split_fracs()[1] - 0.64).abs() < 1e-6);
    }

    #[test]
    fn hero_is_current_sector() {
        let mut s = snap();
        s.current_lap_ms = 5_000;
        assert_eq!(hero_index(&s), 0);
        s.sector_cur = [24_000, 0, 0];
        assert_eq!(hero_index(&s), 1);
        s.sector_cur = [24_000, 25_000, 0];
        assert_eq!(hero_index(&s), 2);
        s.current_lap_ms = 0;
        s.sector_cur = [0, 0, 0];
        s.sector_last_lap = [24_000, 25_000, 23_000];
        assert_eq!(hero_index(&s), 2);
    }

    #[test]
    fn live_s1_then_freeze_when_leaving() {
        let _lock = tmp();
        seed_tape();
        let mut s = snap();
        let here = pos_for(20_000);
        s.current_lap_ms = 18_000;
        s.local_track_pos = here;
        tick(&s);
        let r = row(&s, 0, true);
        assert!(r.fresh);
        assert!(r.live);
        assert_eq!(r.delta_ms, 18_000 - tape_at(here));
        assert_eq!(r.time_ms, 18_000);
        let line = pos_for(S1);
        s.current_lap_ms = 26_000;
        s.local_track_pos = line;
        s.sector_cur = [24_500, 0, 0];
        tick(&s);
        let frozen = row(&s, 0, true);
        assert!(!frozen.live);
        assert!(!frozen.fresh);
        assert_eq!(frozen.time_ms, 24_500);
        assert_eq!(frozen.delta_ms, 24_500 - tape_at(line));
        let s2 = row(&s, 1, true);
        assert!(s2.fresh);
        assert!(s2.live);
    }

    #[test]
    fn live_s2_is_vs_location_in_sector() {
        let _lock = tmp();
        seed_tape();
        let mut s = snap();
        let line = pos_for(S1);
        s.current_lap_ms = 25_000;
        s.local_track_pos = line;
        s.sector_cur = [25_000, 0, 0];
        tick(&s);
        assert!(row(&s, 0, true).delta_ms > 500);
        let half = 0.5;
        s.current_lap_ms = 25_000 + (tape_at(half) - tape_at(line));
        s.local_track_pos = half;
        tick(&s);
        let s2 = row(&s, 1, true);
        assert!(s2.fresh);
        assert!(s2.live);
        assert!(
            s2.delta_ms.abs() <= 20,
            "S2 should ignore the S1 deficit, got {}",
            s2.delta_ms
        );
    }

    #[test]
    fn new_pb_freeze_is_not_zero() {
        let _lock = tmp();
        seed_tape();
        let mut s = snap();
        let line = pos_for(S1);
        s.current_lap_ms = 23_000;
        s.local_track_pos = line;
        s.sector_cur = [23_000, 0, 0];
        s.sector_best = [23_000, 0, 0];
        tick(&s);
        let r = row(&s, 0, true);
        assert_eq!(r.delta_ms, 23_000 - tape_at(line));
        assert_ne!(r.delta_ms, 0);
        assert_eq!(track_pb::bind("LiveTrack").sectors[0], 23_000);
    }

    #[test]
    fn slower_split_does_not_replace_saved() {
        let _lock = tmp();
        seed_tape();
        let mut s = snap();
        let line = pos_for(S1);
        s.current_lap_ms = 26_000;
        s.local_track_pos = line;
        s.sector_cur = [26_000, 0, 0];
        tick(&s);
        assert_eq!(track_pb::bind("LiveTrack").sectors[0], 24_000);
        assert_eq!(row(&s, 0, true).delta_ms, 26_000 - tape_at(line));
    }

    #[test]
    fn live_off_waits_until_split() {
        let _lock = tmp();
        seed_tape();
        let mut s = snap();
        s.current_lap_ms = 18_000;
        s.local_track_pos = pos_for(20_000);
        tick(&s);
        let hidden = row(&s, 0, false);
        assert!(hidden.fresh);
        assert!(!hidden.live);
        assert!(hidden.pending);
        let shown = row(&s, 0, true);
        assert!(shown.live);
        assert!(!shown.pending);
        let line = pos_for(S1);
        s.current_lap_ms = 24_500;
        s.local_track_pos = line;
        s.sector_cur = [24_500, 0, 0];
        tick(&s);
        let frozen = row(&s, 0, false);
        assert!(!frozen.pending);
        assert!(!frozen.live);
        assert_eq!(frozen.time_ms, 24_500);
        let s2 = row(&s, 1, false);
        assert!(s2.fresh);
        assert!(s2.pending);
    }

    #[test]
    fn three_ms_picks_duration_when_closer_to_s1() {
        assert_eq!(three_ms(24_000, 25_000, 72_000), 23_000);
        assert_eq!(three_ms(24_000, 49_000, 72_000), 23_000);
        assert_eq!(three_ms(0, 25_000, 72_000), 0);
        assert_eq!(three_ms(24_000, 25_000, 0), 0);
    }

    #[test]
    fn split_duration_reads_cumulative_or_raw() {
        assert_eq!(split_duration(0, [24_000, 49_000, 0], 0), 24_000);
        assert_eq!(split_duration(1, [24_000, 49_000, 0], 0), 25_000);
        assert_eq!(split_duration(1, [24_000, 20_000, 0], 0), 20_000);
        assert_eq!(split_duration(2, [24_000, 25_000, 0], 72_000), 23_000);
        assert_eq!(split_duration(2, [24_000, 25_000, 22_000], 72_000), 22_000);
    }

    #[test]
    fn plugin_delta_is_preferred_over_tape() {
        let _lock = tmp();
        seed_tape();
        let mut s = snap();
        s.current_lap_ms = 40_000;
        s.sector_cur = [24_000, 0, 0];
        s.sector_delta_valid = 0b001;
        s.sector_delta = [123, 0, 0];
        assert_eq!(row(&s, 0, true).delta_ms, 123);
        assert_eq!(row(&s, 0, true).time_ms, 24_000);
    }
}
