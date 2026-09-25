//! Per-rider line + channel tapes at track position (live ghosts + review commits).

use crate::race_store::{norm_lap_pos, skip_warmup_laps};
use crate::shm::{cstr, Snapshot};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

pub const BINS: usize = 256;
pub const MAX_LINE: usize = 16_000;
const MIN_LAP_MS: i32 = 20_000;
const MAX_LAP_MS: i32 = 15 * 60 * 1000;
const MIN_COVER: f32 = 0.45;
const LINE_MIN_M: f32 = 0.6;
pub const LINE_CUT_M2: f32 = 30.0 * 30.0;
const CUT_M2: f32 = LINE_CUT_M2;
const HITCH_MS: u128 = 200;
const GATE_STATE: i32 = 256;
const CONTACT_M2: f32 = 16.0;

#[derive(Clone, Copy, Debug, Default)]
pub struct ChannelBin {
    pub speed: f32,
    pub y: f32,
    pub rpm: f32,
    pub throttle: f32,
    pub brake: f32,
    pub lean: f32,
    pub yaw: f32,
    pub ms: i32,
    pub gear: i32,
}

#[derive(Clone, Debug, Default)]
pub struct CrashMark {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub contact_num: i32,
    pub contact_name: String,
}

#[derive(Clone, Debug)]
pub struct CommittedLap {
    pub race_num: i32,
    pub name: String,
    pub bike: String,
    pub lap_ms: i32,
    pub sectors: [i32; 3],
    pub bins: [ChannelBin; BINS],
    pub line: Vec<(f32, f32, f32)>,
    pub is_you: bool,
    pub crashed: bool,
    pub cut: bool,
    pub warmup: bool,
    /// Standing-start first race lap after gate (not a flying lap).
    pub from_gate: bool,
    pub crashes: Vec<CrashMark>,
}

#[derive(Clone, Debug, Default)]
pub struct CompareLines {
    pub you: Vec<(f32, f32, f32)>,
    pub other: Vec<(f32, f32, f32)>,
    pub other_num: i32,
}

struct RiderWork {
    last_lap_ms: i32,
    last_x: f32,
    last_z: f32,
    line: Vec<(f32, f32, f32)>,
    bins: [ChannelBin; BINS],
    filled: usize,
    best: Option<CommittedLap>,
    crashed: bool,
    was_crashed: bool,
    first_after_gate: bool,
    cut: bool,
    crashes: Vec<CrashMark>,
    last_pos: f32,
    lap_t0: Option<Instant>,
    last_at: Option<Instant>,
    gap: bool,
}

impl RiderWork {
    fn new() -> Self {
        Self {
            last_lap_ms: 0,
            last_x: 0.0,
            last_z: 0.0,
            line: Vec::new(),
            bins: [ChannelBin::default(); BINS],
            filled: 0,
            best: None,
            crashed: false,
            was_crashed: false,
            first_after_gate: false,
            cut: false,
            crashes: Vec::new(),
            last_pos: -1.0,
            lap_t0: None,
            last_at: None,
            gap: false,
        }
    }

    fn reset(&mut self) {
        self.line.clear();
        self.bins = [ChannelBin::default(); BINS];
        self.filled = 0;
        self.crashed = false;
        self.was_crashed = false;
        self.cut = false;
        self.crashes.clear();
        self.last_x = 0.0;
        self.last_z = 0.0;
        self.last_pos = -1.0;
        self.lap_t0 = None;
        self.last_at = None;
        self.gap = false;
    }

    fn note_pos(&mut self, pos: f32) {
        if self.last_pos >= 0.0 && self.last_pos > 0.85 && pos < 0.15 {
            self.lap_t0 = Some(Instant::now());
        } else if self.lap_t0.is_none() {
            self.lap_t0 = Some(Instant::now());
        }
        self.last_pos = pos;
    }

    fn elapsed_ms(&self) -> i32 {
        self.lap_t0
            .map(|t| t.elapsed().as_millis().min(i32::MAX as u128) as i32)
            .unwrap_or(0)
    }

    fn cover(&self) -> f32 {
        self.filled as f32 / BINS as f32
    }
}

pub fn line_is_cut(line: &[(f32, f32, f32)]) -> bool {
    line.windows(2).any(|w| {
        if !w[0].0.is_finite() || !w[1].0.is_finite() {
            return false;
        }
        let dx = w[1].0 - w[0].0;
        let dz = w[1].2 - w[0].2;
        dx * dx + dz * dz >= CUT_M2
    })
}

/// Half-open ranges of finite XZ samples with no ≥30 m jump.
pub fn line_run_ranges(pts: &[(f32, f32)]) -> Vec<(usize, usize)> {
    let mut runs = Vec::new();
    let mut start = None;
    let mut prev: Option<(f32, f32)> = None;
    for (i, p) in pts.iter().enumerate() {
        if !p.0.is_finite() || !p.1.is_finite() {
            if let Some(s) = start.take() {
                if i - s >= 2 {
                    runs.push((s, i));
                }
            }
            prev = None;
            continue;
        }
        if let Some(q) = prev {
            let dx = p.0 - q.0;
            let dy = p.1 - q.1;
            if dx * dx + dy * dy >= CUT_M2 {
                if let Some(s) = start.take() {
                    if i - s >= 2 {
                        runs.push((s, i));
                    }
                }
                start = Some(i);
            }
        } else {
            start = Some(i);
        }
        prev = Some(*p);
    }
    if let Some(s) = start {
        if pts.len() - s >= 2 {
            runs.push((s, pts.len()));
        }
    }
    runs
}

fn jump_is_cut(d2: f32, since_ms: Option<u128>) -> bool {
    if d2 < CUT_M2 {
        return false;
    }
    match since_ms {
        Some(ms) if ms > HITCH_MS => false,
        _ => true,
    }
}

fn line_min_d2(track_len: f32) -> f32 {
    let spacing = if track_len > 10.0 {
        (track_len / MAX_LINE as f32).max(LINE_MIN_M)
    } else {
        LINE_MIN_M
    };
    spacing * spacing
}

pub fn infer_cut(line: &[(f32, f32, f32)], bins: &[ChannelBin; BINS], lap_ms: i32) -> bool {
    if line_is_cut(line) {
        return true;
    }
    let filled = bins
        .iter()
        .filter(|b| b.ms > 0 || b.speed != 0.0 || b.y != 0.0)
        .count();
    if filled == 0 {
        return false;
    }
    let cover = filled as f32 / BINS as f32;
    cover < MIN_COVER && lap_ms >= MIN_LAP_MS
}

fn lap_cut(first_race: bool, jumped: bool, cover: f32) -> bool {
    jumped || (!first_race && cover < MIN_COVER)
}

struct Store {
    riders: HashMap<i32, RiderWork>,
    commits: Vec<CommittedLap>,
    fastest_num: i32,
    you_num: i32,
    on_gate: bool,
    gate_dropped: bool,
}

impl Store {
    fn new() -> Self {
        Self {
            riders: HashMap::new(),
            commits: Vec::new(),
            fastest_num: -1,
            you_num: -1,
            on_gate: false,
            gate_dropped: false,
        }
    }
}

static STORE: Mutex<Option<Store>> = Mutex::new(None);

fn with_store<R>(f: impl FnOnce(&mut Store) -> R) -> R {
    let mut g = STORE.lock().unwrap_or_else(|e| e.into_inner());
    if g.is_none() {
        *g = Some(Store::new());
    }
    f(g.as_mut().unwrap())
}

pub fn reset() {
    with_store(|st| *st = Store::new());
}

pub fn take_commits() -> Vec<CommittedLap> {
    with_store(|st| std::mem::take(&mut st.commits))
}

pub fn fastest_num() -> i32 {
    with_store(|st| st.fastest_num)
}

pub fn live_compare() -> CompareLines {
    with_store(|st| {
        let you = st
            .riders
            .get(&st.you_num)
            .map(|w| w.line.clone())
            .unwrap_or_default();
        let other_num = if st.fastest_num >= 0 && st.fastest_num != st.you_num {
            st.fastest_num
        } else {
            -1
        };
        let other = st
            .riders
            .get(&other_num)
            .and_then(|w| w.best.as_ref().map(|b| b.line.clone()))
            .unwrap_or_default();
        CompareLines {
            you,
            other,
            other_num,
        }
    })
}

pub fn tick(s: &Snapshot) {
    let you = if s.local_race_num >= 0 {
        s.local_race_num
    } else {
        s.focus_race_num
    };
    let warmup = skip_warmup_laps(s);
    with_store(|st| {
        let mut pending = Vec::new();
        st.you_num = you;
        refresh_fastest(s, st);
        note_gate(s, warmup, st);

        if s.session_state == GATE_STATE && !warmup {
            return;
        }

        let min_d2 = line_min_d2(s.track_length);
        let n = s.rider_count.clamp(0, crate::shm::MAX_RIDERS as i32) as usize;
        for i in 0..n {
            let r = &s.riders[i];
            if r.race_num <= 0 {
                continue;
            }
            let pos = norm_lap_pos(s, r.track_pos);
            if !(0.0..1.0).contains(&pos) {
                continue;
            }
            let y = if r.race_num == you && s.has_telemetry != 0 {
                s.local_y
            } else {
                r.y
            };
            let speed = if r.race_num == you && s.has_telemetry != 0 {
                s.local_speed
            } else {
                r.speed
            };
            let rpm = if r.race_num == you && s.has_telemetry != 0 {
                s.local_rpm as f32
            } else {
                r.rpm as f32
            };
            let throttle = if r.race_num == you && s.has_telemetry != 0 {
                s.local_throttle
            } else {
                r.throttle
            };
            let brake = if r.race_num == you && s.has_telemetry != 0 {
                s.local_front_brake.max(s.local_rear_brake)
            } else {
                r.front_brake
            };
            let gear = if r.race_num == you && s.has_telemetry != 0 {
                s.local_gear
            } else {
                r.gear
            };
            let now_crashed = if r.race_num == you {
                s.local_crashed != 0 || r.crashed != 0
            } else {
                r.crashed != 0
            };
            let work = st.riders.entry(r.race_num).or_insert_with(RiderWork::new);
            if st.gate_dropped && !warmup {
                work.first_after_gate = true;
            }
            if r.race_num == you && now_crashed && !work.was_crashed {
                work.crashes.push(crash_mark(s, you, r.x, y, r.z));
            }
            work.was_crashed = now_crashed;
            if now_crashed {
                work.crashed = true;
            }
            work.note_pos(pos);
            let bin = ((pos * BINS as f32).floor() as usize).min(BINS - 1);
            if work.bins[bin].ms <= 0 {
                work.filled += 1;
            }
            work.bins[bin] = ChannelBin {
                speed,
                y,
                rpm,
                throttle,
                brake,
                lean: r.lean,
                yaw: r.yaw,
                ms: if r.race_num == you {
                    s.current_lap_ms
                } else {
                    work.elapsed_ms()
                },
                gear,
            };
            if work.line.is_empty() {
                work.line.push((r.x, y, r.z));
                work.last_x = r.x;
                work.last_z = r.z;
                work.last_at = Some(Instant::now());
            } else {
                let dx = r.x - work.last_x;
                let dz = r.z - work.last_z;
                let d2 = dx * dx + dz * dz;
                let since = work.last_at.map(|t| t.elapsed().as_millis());
                if jump_is_cut(d2, since) {
                    work.cut = true;
                    work.last_x = r.x;
                    work.last_z = r.z;
                    work.last_at = Some(Instant::now());
                } else if d2 >= CUT_M2 {
                    work.gap = true;
                    work.last_x = r.x;
                    work.last_z = r.z;
                    work.last_at = Some(Instant::now());
                } else if !work.cut && d2 >= min_d2 && work.line.len() < MAX_LINE {
                    if work.gap {
                        if work.line.len() + 1 < MAX_LINE {
                            work.line.push((f32::NAN, y, f32::NAN));
                        }
                        work.gap = false;
                    }
                    work.line.push((r.x, y, r.z));
                    work.last_x = r.x;
                    work.last_z = r.z;
                    work.last_at = Some(Instant::now());
                }
            }

            let last = standing_last(s, r.race_num);
            if last > 0 && last != work.last_lap_ms {
                let first_race = !warmup && work.first_after_gate;
                let keep = should_commit(
                    warmup,
                    first_race,
                    work.crashed,
                    last,
                    work.cover(),
                    work.line.len(),
                );
                if keep {
                    let name = cstr(&r.name);
                    let bike = standing_bike(s, r.race_num);
                    let sectors = if r.race_num == you {
                        s.sector_last_lap
                    } else {
                        [0, 0, 0]
                    };
                    let cut = lap_cut(first_race, work.cut, work.cover());
                    let lap = CommittedLap {
                        race_num: r.race_num,
                        name,
                        bike,
                        lap_ms: last,
                        sectors,
                        bins: work.bins,
                        line: work.line.clone(),
                        is_you: r.race_num == you,
                        crashed: work.crashed,
                        cut,
                        warmup,
                        from_gate: first_race,
                        crashes: std::mem::take(&mut work.crashes),
                    };
                    if !lap.crashed
                        && !lap.cut
                        && work.best.as_ref().is_none_or(|b| last < b.lap_ms)
                    {
                        work.best = Some(lap.clone());
                    }
                    pending.push(lap);
                }
                work.reset();
                work.last_lap_ms = last;
                if first_race {
                    work.first_after_gate = false;
                }
            } else if work.last_lap_ms == 0 && last > 0 {
                work.last_lap_ms = last;
            }
        }
        if st.gate_dropped {
            st.gate_dropped = false;
        }
        st.commits.extend(pending);
        refresh_fastest_from_works(st);
    });
}

fn note_gate(s: &Snapshot, warmup: bool, st: &mut Store) {
    if warmup {
        st.on_gate = false;
        return;
    }
    if s.session_state == GATE_STATE {
        st.on_gate = true;
        return;
    }
    if st.on_gate {
        st.on_gate = false;
        st.gate_dropped = true;
        for w in st.riders.values_mut() {
            w.first_after_gate = true;
        }
    }
}

fn should_commit(
    warmup: bool,
    first_race: bool,
    crashed: bool,
    last: i32,
    _cover: f32,
    line_len: usize,
) -> bool {
    if last <= 0 || last > MAX_LAP_MS || line_len < 2 {
        return false;
    }
    if warmup {
        return !crashed && last >= MIN_LAP_MS;
    }
    if first_race {
        return true;
    }
    true
}

fn crash_mark(s: &Snapshot, you: i32, x: f32, y: f32, z: f32) -> CrashMark {
    let n = s.rider_count.clamp(0, crate::shm::MAX_RIDERS as i32) as usize;
    let mut best_d2 = CONTACT_M2;
    let mut contact_num = -1;
    let mut contact_name = String::new();
    for r in s.riders.iter().take(n) {
        if r.race_num <= 0 || r.race_num == you {
            continue;
        }
        let dx = r.x - x;
        let dz = r.z - z;
        let d2 = dx * dx + dz * dz;
        if d2 <= best_d2 {
            best_d2 = d2;
            contact_num = r.race_num;
            contact_name = cstr(&r.name);
        }
    }
    CrashMark {
        x,
        y,
        z,
        contact_num,
        contact_name,
    }
}

fn standing_last(s: &Snapshot, race_num: i32) -> i32 {
    let n = s.standing_count.clamp(0, crate::shm::MAX_STANDINGS as i32) as usize;
    s.standings
        .iter()
        .take(n)
        .find(|st| st.race_num == race_num)
        .map(|st| st.last_lap_ms)
        .unwrap_or(0)
}

fn standing_bike(s: &Snapshot, race_num: i32) -> String {
    let n = s.standing_count.clamp(0, crate::shm::MAX_STANDINGS as i32) as usize;
    s.standings
        .iter()
        .take(n)
        .find(|st| st.race_num == race_num)
        .map(|st| cstr(&st.bike))
        .unwrap_or_default()
}

fn refresh_fastest(s: &Snapshot, st: &mut Store) {
    let n = s.standing_count.clamp(0, crate::shm::MAX_STANDINGS as i32) as usize;
    let mut best_n = -1;
    let mut best_ms = i32::MAX;
    for row in s.standings.iter().take(n) {
        if row.best_lap_ms > 0 && row.best_lap_ms < best_ms {
            best_ms = row.best_lap_ms;
            best_n = row.race_num;
        }
    }
    if best_n >= 0 {
        st.fastest_num = best_n;
    }
}

fn refresh_fastest_from_works(st: &mut Store) {
    let mut best_n = st.fastest_num;
    let mut best_ms = i32::MAX;
    for (num, w) in &st.riders {
        if let Some(b) = &w.best {
            if b.lap_ms > 0 && b.lap_ms < best_ms {
                best_ms = b.lap_ms;
                best_n = *num;
            }
        }
    }
    st.fastest_num = best_n;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shm::{Snapshot, Standing, NAME};

    static TEST: Mutex<()> = Mutex::new(());

    fn isolated(f: impl FnOnce()) {
        let _g = TEST.lock().unwrap_or_else(|e| e.into_inner());
        reset();
        f();
        reset();
    }

    fn name_bytes(s: &str) -> [u8; NAME] {
        let mut b = [0u8; NAME];
        let raw = s.as_bytes();
        let n = raw.len().min(NAME - 1);
        b[..n].copy_from_slice(&raw[..n]);
        b
    }

    fn rider(num: i32, name: &str, x: f32, z: f32, pos: f32, crashed: i32) -> crate::shm::Rider {
        crate::shm::Rider {
            race_num: num,
            x,
            z,
            track_pos: pos,
            crashed,
            name: name_bytes(name),
            ..crate::shm::Rider::default()
        }
    }

    fn standing(num: i32, last: i32) -> Standing {
        Standing {
            race_num: num,
            last_lap_ms: last,
            ..Standing::default()
        }
    }

    fn base(kind: i32, state: i32) -> Snapshot {
        let mut s = Snapshot::default();
        s.session_kind = kind;
        s.session_state = state;
        s.local_race_num = 2;
        s.has_telemetry = 1;
        s.rider_count = 1;
        s.standing_count = 1;
        s.riders[0] = rider(2, "You", 0.0, 0.0, 0.1, 0);
        s.standings[0] = standing(2, 0);
        s
    }

    fn drive_cover(s: &mut Snapshot, last: i32) {
        for i in 0..140 {
            s.riders[0].track_pos = i as f32 / 140.0;
            s.riders[0].x = i as f32;
            s.standings[0].last_lap_ms = 0;
            tick(s);
        }
        s.standings[0].last_lap_ms = last;
        tick(s);
    }

    #[test]
    fn reset_clears_live_compare() {
        isolated(|| {
            let empty = live_compare();
            assert!(empty.you.is_empty());
            assert!(empty.other.is_empty());
            assert_eq!(empty.other_num, -1);
            tick(&Snapshot::default());
            assert!(live_compare().you.is_empty());
        });
    }

    #[test]
    fn warmup_rejects_crash_and_short() {
        isolated(|| {
            let mut s = base(5, 16);
            s.riders[0].x = 1.0;
            tick(&s);
            s.riders[0].x = 3.0;
            s.riders[0].track_pos = 0.2;
            s.standings[0].last_lap_ms = 12_000;
            tick(&s);
            assert!(take_commits().is_empty());

            reset();
            let mut s = base(5, 16);
            s.riders[0].crashed = 1;
            s.local_crashed = 1;
            drive_cover(&mut s, 45_000);
            assert!(take_commits().is_empty());
        });
    }

    #[test]
    fn warmup_commits_clean_full_lap() {
        isolated(|| {
            let mut s = base(5, 16);
            drive_cover(&mut s, 45_000);
            let commits = take_commits();
            assert_eq!(commits.len(), 1);
            assert!(commits[0].warmup);
            assert!(commits[0].is_you);
        });
    }

    #[test]
    fn warmup_commits_other_riders_clean_full_lap() {
        isolated(|| {
            let mut s = base(5, 16);
            s.rider_count = 2;
            s.standing_count = 2;
            s.riders[1] = rider(7, "Cole", 0.0, 0.0, 0.1, 0);
            s.standings[1] = standing(7, 0);
            for i in 0..140 {
                s.riders[0].track_pos = i as f32 / 140.0;
                s.riders[0].x = i as f32;
                s.riders[1].track_pos = i as f32 / 140.0;
                s.riders[1].x = i as f32 + 2.0;
                s.standings[0].last_lap_ms = 0;
                s.standings[1].last_lap_ms = 0;
                tick(&s);
            }
            s.standings[0].last_lap_ms = 45_000;
            s.standings[1].last_lap_ms = 44_000;
            tick(&s);
            let commits = take_commits();
            let cole = commits
                .iter()
                .find(|c| c.race_num == 7)
                .expect("other warmup");
            assert!(cole.warmup);
            assert!(!cole.is_you);
            assert_eq!(cole.lap_ms, 44_000);
        });
    }

    #[test]
    fn practice_commits_clean_full_lap() {
        isolated(|| {
            let mut s = base(-1, 16);
            s.session_length = 40;
            drive_cover(&mut s, 45_000);
            let commits = take_commits();
            assert_eq!(commits.len(), 1);
            assert!(!commits[0].warmup);
        });
    }

    #[test]
    fn race_first_crossing_after_gate_commits_short() {
        isolated(|| {
            let mut s = base(7, GATE_STATE);
            tick(&s);
            s.session_state = 16;
            s.riders[0].x = 0.0;
            tick(&s);
            s.riders[0].x = 4.0;
            s.riders[0].track_pos = 0.3;
            s.standings[0].last_lap_ms = 8_500;
            tick(&s);
            let commits = take_commits();
            assert_eq!(commits.len(), 1);
            assert_eq!(commits[0].lap_ms, 8_500);
            assert!(!commits[0].cut);
        });
    }

    #[test]
    fn jump_is_cut_and_drops_the_chord() {
        isolated(|| {
            let mut s = base(7, 16);
            s.riders[0].x = 0.0;
            tick(&s);
            s.riders[0].x = 4.0;
            s.riders[0].track_pos = 0.2;
            tick(&s);
            s.riders[0].x = 90.0;
            s.riders[0].track_pos = 0.4;
            tick(&s);
            s.riders[0].x = 92.0;
            s.riders[0].track_pos = 0.5;
            s.standings[0].last_lap_ms = 28_000;
            tick(&s);
            let commits = take_commits();
            assert_eq!(commits.len(), 1);
            assert!(commits[0].cut);
            assert!(!line_is_cut(&commits[0].line));
        });
    }

    #[test]
    fn later_race_low_cover_is_cut() {
        isolated(|| {
            let mut s = base(7, 16);
            s.riders[0].x = 0.0;
            tick(&s);
            s.riders[0].x = 3.0;
            s.riders[0].track_pos = 0.1;
            s.standings[0].last_lap_ms = 40_000;
            tick(&s);
            let commits = take_commits();
            assert_eq!(commits.len(), 1);
            assert!(commits[0].cut);
        });
    }

    #[test]
    fn jump_is_cut_ignores_stale_gap() {
        assert!(jump_is_cut(900.0, Some(50)));
        assert!(jump_is_cut(900.0, None));
        assert!(!jump_is_cut(900.0, Some(250)));
        assert!(!jump_is_cut(1.0, Some(50)));
    }

    #[test]
    fn line_min_d2_follows_track_length() {
        assert!((line_min_d2(0.0) - 0.36).abs() < 1e-6);
        assert!((line_min_d2(2000.0) - 0.36).abs() < 1e-6);
        assert!((line_min_d2(16_000.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn long_track_line_reaches_the_end() {
        isolated(|| {
            let mut s = base(7, 16);
            s.track_length = 2000.0;
            let step = 0.6_f32;
            let n = (2000.0 / step) as i32 + 50;
            for i in 0..n {
                let x = i as f32 * step;
                s.riders[0].x = x;
                s.riders[0].track_pos = (x / 2000.0).min(0.99);
                s.standings[0].last_lap_ms = 0;
                tick(&s);
            }
            s.standings[0].last_lap_ms = 90_000;
            tick(&s);
            let lap = take_commits().into_iter().next().expect("lap");
            let last_x = lap.line.last().expect("pt").0;
            assert!(
                last_x > 1800.0,
                "line should reach the end of a 2km lap, last_x={last_x} n={}",
                lap.line.len()
            );
            assert!(lap.line.len() <= MAX_LINE);
        });
    }

    #[test]
    fn eight_km_line_keeps_density_and_reaches_the_end() {
        isolated(|| {
            let mut s = base(7, 16);
            s.track_length = 8000.0;
            let step = 0.6_f32;
            let n = (8000.0 / step) as i32 + 50;
            for i in 0..n {
                let x = i as f32 * step;
                s.riders[0].x = x;
                s.riders[0].track_pos = (x / 8000.0).min(0.99);
                s.standings[0].last_lap_ms = 0;
                tick(&s);
            }
            s.standings[0].last_lap_ms = 180_000;
            tick(&s);
            let lap = take_commits().into_iter().next().expect("lap");
            let last_x = lap.line.last().expect("pt").0;
            assert!(
                last_x > 7200.0,
                "line should reach the end of an 8km lap, last_x={last_x} n={}",
                lap.line.len()
            );
            assert!(
                lap.line.len() > 2000,
                "8km should keep ~0.6m spacing, n={}",
                lap.line.len()
            );
            assert!(lap.line.len() <= MAX_LINE);
        });
    }

    #[test]
    fn hitch_jump_other_rider_still_commits() {
        isolated(|| {
            let mut s = base(7, 256);
            s.rider_count = 2;
            s.standing_count = 2;
            s.riders[1] = rider(7, "Cole", 0.0, 0.0, 0.1, 0);
            s.standings[1] = standing(7, 0);
            tick(&s);
            s.session_state = 16;
            tick(&s);
            s.riders[1].x = 4.0;
            s.riders[1].track_pos = 0.2;
            tick(&s);
            std::thread::sleep(std::time::Duration::from_millis(250));
            s.riders[1].x = 50.0;
            s.riders[1].track_pos = 0.5;
            tick(&s);
            s.riders[1].x = 52.0;
            s.riders[1].track_pos = 0.55;
            s.standings[1].last_lap_ms = 8_500;
            tick(&s);
            let cole = take_commits()
                .into_iter()
                .find(|c| c.race_num == 7)
                .expect("cole lap");
            assert!(!cole.cut, "stale 30m step should not mark cut");
            assert!(cole.line.len() >= 2);
            assert!(
                !line_is_cut(&cole.line),
                "hitch must not store a grass hypotenuse"
            );
        });
    }

    #[test]
    fn frozen_then_resume_does_not_store_a_chord() {
        isolated(|| {
            let mut s = base(7, 256);
            s.rider_count = 2;
            s.standing_count = 2;
            s.riders[1] = rider(7, "Cole", 0.0, 0.0, 0.1, 0);
            s.standings[1] = standing(7, 0);
            tick(&s);
            s.session_state = 16;
            tick(&s);
            s.riders[1].x = 4.0;
            s.riders[1].track_pos = 0.2;
            tick(&s);
            for _ in 0..8 {
                tick(&s);
            }
            std::thread::sleep(std::time::Duration::from_millis(250));
            s.riders[1].x = 54.0;
            s.riders[1].track_pos = 0.5;
            tick(&s);
            s.riders[1].x = 56.0;
            s.riders[1].track_pos = 0.55;
            s.standings[1].last_lap_ms = 8_500;
            tick(&s);
            let cole = take_commits()
                .into_iter()
                .find(|c| c.race_num == 7)
                .expect("cole lap");
            assert!(!cole.cut, "frozen SHM then a jump is a hitch, not a cut");
            assert!(!line_is_cut(&cole.line));
        });
    }

    #[test]
    fn line_run_ranges_splits_a_long_chord() {
        let pts = [(0.0, 0.0), (0.0, 8.0), (80.0, 8.0), (80.0, 16.0)];
        assert_eq!(line_run_ranges(&pts), vec![(0, 2), (2, 4)]);
        let with_nan = [
            (0.0, 0.0),
            (0.0, 8.0),
            (f32::NAN, f32::NAN),
            (80.0, 8.0),
            (80.0, 16.0),
        ];
        assert_eq!(line_run_ranges(&with_nan), vec![(0, 2), (3, 5)]);
    }

    #[test]
    fn other_rider_bins_get_elapsed_ms() {
        isolated(|| {
            let mut s = base(7, 16);
            s.rider_count = 2;
            s.standing_count = 2;
            s.riders[1] = rider(7, "Cole", 0.0, 0.0, 0.1, 0);
            s.standings[1] = standing(7, 0);
            tick(&s);
            std::thread::sleep(std::time::Duration::from_millis(8));
            s.riders[1].x = 4.0;
            s.riders[1].track_pos = 0.25;
            tick(&s);
            s.standings[1].last_lap_ms = 30_000;
            tick(&s);
            let cole = take_commits().into_iter().find(|c| c.race_num == 7);
            let cole = cole.expect("cole lap");
            assert!(
                cole.bins.iter().any(|b| b.ms > 0),
                "other rider should stamp elapsed ms"
            );
        });
    }

    #[test]
    fn crash_names_nearest_within_four_meters() {
        isolated(|| {
            let mut s = base(7, 16);
            s.rider_count = 2;
            s.riders[1] = rider(7, "Webb", 1.0, 0.0, 0.1, 0);
            tick(&s);
            s.local_crashed = 1;
            s.riders[0].crashed = 1;
            tick(&s);
            s.riders[0].x = 2.0;
            s.standings[0].last_lap_ms = 30_000;
            tick(&s);
            let commits = take_commits();
            assert_eq!(commits.len(), 1);
            assert!(commits[0].crashed);
            assert_eq!(commits[0].crashes[0].contact_name, "Webb");

            reset();
            let mut s = base(7, 16);
            s.rider_count = 2;
            s.riders[1] = rider(7, "Webb", 20.0, 0.0, 0.1, 0);
            tick(&s);
            s.local_crashed = 1;
            s.riders[0].crashed = 1;
            tick(&s);
            s.riders[0].x = 2.0;
            s.standings[0].last_lap_ms = 30_000;
            tick(&s);
            let commits = take_commits();
            assert_eq!(commits.len(), 1);
            assert!(commits[0].crashes[0].contact_name.is_empty());
        });
    }
}
