//! Rider-type scores from eligible race motos (live tapes or compact rows).

use mxbo_hud::location_tape::{ChannelBin, BINS};

pub const PROFILE_LOCK_RACES: i32 = 5;
pub const PROFILE_WINDOW_SECS: i64 = 14 * 24 * 3600;
pub const WARMUP_KIND: i32 = 5;
pub const AXIS_COUNT: usize = 8;
pub const AXIS_LABELS: [&str; AXIS_COUNT] = [
    "PACE", "SMOOTH", "CLEAN", "CLUTCH", "ATTACK", "AIR", "RESULTS", "OPENING",
];

/// Game classification `Standing.state` values (HUD: DNS / OUT / DSQ).
pub const STATE_DNS: i32 = 1;
pub const STATE_OUT: i32 = 3; // Profile label: DNF
pub const STATE_DSQ: i32 = 4;

pub fn out_of_race_state(state: i32) -> bool {
    matches!(state, STATE_DNS | STATE_OUT | STATE_DSQ)
}

/// Legacy / unknown (`0`) counts as classified so old rollups keep working.
pub fn classified_state(state: i32) -> bool {
    !out_of_race_state(state)
}

pub fn you_won_race(position: i32, rider_count: i32) -> bool {
    position == 1 && rider_count > 1
}

/// Motos crown / Profile Wins: P1 with a field on an eligible race moto (not practice/warmup/solo).
pub fn counts_as_profile_win(
    practice: bool,
    kind: i32,
    rider_count: i32,
    position: i32,
    has_non_warmup_lap: bool,
) -> bool {
    you_won_race(position, rider_count)
        && has_non_warmup_lap
        && !practice
        && kind != WARMUP_KIND
        && rider_count >= 2
}

const PACE_FLOOR: f32 = 0.88;
const CV_CAP: f32 = 0.06;
const ATTACK_HOT: f32 = 0.85;
const ATTACK_LO: f32 = 0.15;
const ATTACK_HI: f32 = 0.55;
const AIR_M: f32 = 1.5;
const AIR_WIN: usize = 16;
const AIR_HI: f32 = 0.08;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileWindow {
    AllTime,
    TwoWeeks,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RiderProfile {
    pub name: String,
    pub race_count: i32,
    pub all_time_count: i32,
    pub holeshots: i32,
    pub wins: i32,
    pub scores: [Option<f32>; AXIS_COUNT],
    pub win_rate: Option<f32>,
    pub podium_rate: Option<f32>,
    pub avg_finish: Option<f32>,
    pub median_finish: Option<f32>,
    pub finish_pct: Option<f32>,
    pub clean_rate: Option<f32>,
    pub crash_rate: Option<f32>,
    pub cut_rate: Option<f32>,
    pub holeshot_rate: Option<f32>,
    pub dns_count: i32,
    pub dnf_count: i32,
    pub dsq_count: i32,
    pub dns_rate: Option<f32>,
    pub dnf_rate: Option<f32>,
    pub dsq_rate: Option<f32>,
    pub avg_penalty_ms: Option<f32>,
}

impl RiderProfile {
    pub fn empty() -> Self {
        Self {
            name: String::new(),
            race_count: 0,
            all_time_count: 0,
            holeshots: 0,
            wins: 0,
            scores: [None; AXIS_COUNT],
            win_rate: None,
            podium_rate: None,
            avg_finish: None,
            median_finish: None,
            finish_pct: None,
            clean_rate: None,
            crash_rate: None,
            cut_rate: None,
            holeshot_rate: None,
            dns_count: 0,
            dnf_count: 0,
            dsq_count: 0,
            dns_rate: None,
            dnf_rate: None,
            dsq_rate: None,
            avg_penalty_ms: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CompactLap {
    pub ms: i32,
    pub crashed: bool,
    pub cut: bool,
}

#[derive(Clone, Debug)]
pub struct CompactRace {
    pub session_id: i64,
    pub started: i64,
    pub rider_count: i32,
    pub position: i32,
    pub your_best_ms: i32,
    pub fastest_ms: i32,
    pub laps: Vec<CompactLap>,
    pub attack_hot: i32,
    pub attack_n: i32,
    pub air_hot: i32,
    pub air_n: i32,
    pub name: String,
    pub holeshot: Option<i32>,
    pub state: i32,
    pub penalty_ms: i32,
}

#[derive(Clone, Debug)]
pub struct RaceLapInput {
    pub lap_num: i32,
    pub ms: i32,
    pub warmup: bool,
    pub crashed: bool,
    pub cut: bool,
    pub bins: [ChannelBin; BINS],
}

#[derive(Clone, Debug)]
pub struct RaceInput {
    pub session_id: i64,
    pub started: i64,
    pub practice: bool,
    pub kind: i32,
    pub rider_count: i32,
    pub your_race_num: i32,
    pub position: i32,
    pub your_best_ms: i32,
    pub fastest_ms: i32,
    pub laps: Vec<RaceLapInput>,
    pub name: String,
    pub holeshot: Option<i32>,
    pub state: i32,
    pub penalty_ms: i32,
}

pub fn eligible(practice: bool, kind: i32, rider_count: i32, laps: &[RaceLapInput]) -> bool {
    if practice || kind == WARMUP_KIND || rider_count < 2 {
        return false;
    }
    laps.iter().any(|l| !l.warmup && l.ms > 0)
}

pub fn compact_from_input(input: &RaceInput) -> Option<CompactRace> {
    if !eligible(input.practice, input.kind, input.rider_count, &input.laps) {
        return None;
    }
    let mut laps = Vec::new();
    let mut attack_hot = 0;
    let mut attack_n = 0;
    let mut air_hot = 0;
    let mut air_n = 0;
    let mut your_best = 0;
    for lap in input.laps.iter().filter(|l| !l.warmup) {
        laps.push(CompactLap {
            ms: lap.ms,
            crashed: lap.crashed,
            cut: lap.cut,
        });
        let clean = lap.ms > 0 && !lap.crashed && !lap.cut;
        if clean && lap.ms > 0 && (your_best == 0 || lap.ms < your_best) {
            your_best = lap.ms;
        }
        if clean && bins_signal(&lap.bins) {
            let (ah, an) = attack_counts(&lap.bins);
            attack_hot += ah;
            attack_n += an;
            let (ih, inn) = air_counts(&lap.bins);
            air_hot += ih;
            air_n += inn;
        }
    }
    if laps.is_empty() {
        return None;
    }
    let your_best_ms = if input.your_best_ms > 0 {
        input.your_best_ms
    } else {
        your_best
    };
    Some(CompactRace {
        session_id: input.session_id,
        started: input.started,
        rider_count: input.rider_count,
        position: input.position,
        your_best_ms,
        fastest_ms: input.fastest_ms,
        laps,
        attack_hot,
        attack_n,
        air_hot,
        air_n,
        name: input.name.clone(),
        holeshot: input.holeshot,
        state: input.state,
        penalty_ms: input.penalty_ms.max(0),
    })
}

/// Fill name / race_count / all_time_count / scores separately; this fills rates.
pub fn apply_window_stats(profile: &mut RiderProfile, races: &[&CompactRace]) {
    let n = races.len() as i32;
    if n <= 0 {
        profile.wins = 0;
        profile.win_rate = None;
        profile.podium_rate = None;
        profile.avg_finish = None;
        profile.median_finish = None;
        profile.finish_pct = None;
        profile.clean_rate = None;
        profile.crash_rate = None;
        profile.cut_rate = None;
        profile.holeshot_rate = None;
        profile.dns_count = 0;
        profile.dnf_count = 0;
        profile.dsq_count = 0;
        profile.dns_rate = None;
        profile.dnf_rate = None;
        profile.dsq_rate = None;
        profile.avg_penalty_ms = None;
        return;
    }

    let wins = races
        .iter()
        .filter(|r| you_won_race(r.position, r.rider_count))
        .count() as i32;
    let podiums = races
        .iter()
        .filter(|r| {
            classified_state(r.state) && r.position >= 1 && r.position <= 3 && r.rider_count > 1
        })
        .count() as i32;
    profile.wins = wins;
    profile.win_rate = Some(wins as f32 / n as f32);
    profile.podium_rate = Some(podiums as f32 / n as f32);

    let mut finishes: Vec<i32> = races
        .iter()
        .filter(|r| classified_state(r.state) && r.position > 0 && r.rider_count > 1)
        .map(|r| r.position)
        .collect();
    if finishes.is_empty() {
        profile.avg_finish = None;
        profile.median_finish = None;
        profile.finish_pct = None;
    } else {
        let sum: i32 = finishes.iter().sum();
        profile.avg_finish = Some(sum as f32 / finishes.len() as f32);
        finishes.sort_unstable();
        let mid = finishes.len() / 2;
        profile.median_finish = Some(if finishes.len() % 2 == 0 {
            (finishes[mid - 1] + finishes[mid]) as f32 / 2.0
        } else {
            finishes[mid] as f32
        });
        let mut pct_sum = 0.0f32;
        let mut pct_n = 0i32;
        for r in races
            .iter()
            .filter(|r| classified_state(r.state) && r.position > 0 && r.rider_count > 1)
        {
            if let Some(p) = results_score(r.position, r.rider_count) {
                pct_sum += p;
                pct_n += 1;
            }
        }
        profile.finish_pct = if pct_n > 0 {
            Some(pct_sum / pct_n as f32)
        } else {
            None
        };
    }

    let mut timed = 0i32;
    let mut clean = 0i32;
    let mut crashed = 0i32;
    let mut cut = 0i32;
    for race in races {
        for lap in race.laps.iter().filter(|l| l.ms > 0) {
            timed += 1;
            if lap.crashed {
                crashed += 1;
            }
            if lap.cut {
                cut += 1;
            }
            if !lap.crashed && !lap.cut {
                clean += 1;
            }
        }
    }
    if timed > 0 {
        profile.clean_rate = Some(clean as f32 / timed as f32);
        profile.crash_rate = Some(crashed as f32 / timed as f32);
        profile.cut_rate = Some(cut as f32 / timed as f32);
    } else {
        profile.clean_rate = None;
        profile.crash_rate = None;
        profile.cut_rate = None;
    }

    let known_hs = races.iter().filter(|r| r.holeshot.is_some()).count() as i32;
    let hs_wins = races.iter().filter(|r| r.holeshot == Some(1)).count() as i32;
    profile.holeshot_rate = if known_hs > 0 {
        Some(hs_wins as f32 / known_hs as f32)
    } else {
        None
    };

    profile.dns_count = races.iter().filter(|r| r.state == STATE_DNS).count() as i32;
    profile.dnf_count = races.iter().filter(|r| r.state == STATE_OUT).count() as i32;
    profile.dsq_count = races.iter().filter(|r| r.state == STATE_DSQ).count() as i32;
    profile.dns_rate = Some(profile.dns_count as f32 / n as f32);
    profile.dnf_rate = Some(profile.dnf_count as f32 / n as f32);
    profile.dsq_rate = Some(profile.dsq_count as f32 / n as f32);

    let pen_sum: i64 = races.iter().map(|r| r.penalty_ms.max(0) as i64).sum();
    profile.avg_penalty_ms = Some(pen_sum as f32 / n as f32);
}

pub fn scores_from_compact(race: &CompactRace) -> [Option<f32>; AXIS_COUNT] {
    let mut out = [None; AXIS_COUNT];
    let race_laps: Vec<&CompactLap> = race.laps.iter().filter(|l| l.ms > 0).collect();
    if race_laps.is_empty() {
        return out;
    }
    let clean: Vec<i32> = race_laps
        .iter()
        .filter(|l| !l.crashed && !l.cut)
        .map(|l| l.ms)
        .collect();

    out[0] = pace_score(race.your_best_ms, race.fastest_ms);
    out[1] = consistency_score(&clean);
    out[2] = Some(clean.len() as f32 / race_laps.len() as f32);
    out[3] = closer_score(&clean);
    out[4] = attack_score(race.attack_hot, race.attack_n);
    out[5] = air_score(race.air_hot, race.air_n);
    out[6] = results_score(race.position, race.rider_count);
    out[7] = opening_score(&race.laps);
    out
}

pub fn mean_scores(races: &[[Option<f32>; AXIS_COUNT]]) -> [Option<f32>; AXIS_COUNT] {
    let mut out = [None; AXIS_COUNT];
    for axis in 0..AXIS_COUNT {
        let mut sum = 0.0;
        let mut n = 0;
        for race in races {
            if let Some(v) = race[axis] {
                sum += v;
                n += 1;
            }
        }
        if n > 0 {
            out[axis] = Some(sum / n as f32);
        }
    }
    out
}

pub fn pack_laps(laps: &[CompactLap]) -> Vec<u8> {
    let mut o = Vec::with_capacity(2 + laps.len() * 5);
    o.extend_from_slice(&(laps.len() as u16).to_le_bytes());
    for lap in laps {
        o.extend_from_slice(&lap.ms.to_le_bytes());
        let mut flags = 0u8;
        if lap.crashed {
            flags |= 1;
        }
        if lap.cut {
            flags |= 2;
        }
        o.push(flags);
    }
    o
}

pub fn unpack_laps(raw: &[u8]) -> Vec<CompactLap> {
    if raw.len() < 2 {
        return Vec::new();
    }
    let n = u16::from_le_bytes(raw[0..2].try_into().unwrap()) as usize;
    let mut laps = Vec::with_capacity(n);
    let mut o = 2;
    for _ in 0..n {
        if o + 5 > raw.len() {
            break;
        }
        let ms = i32::from_le_bytes(raw[o..o + 4].try_into().unwrap());
        let flags = raw[o + 4];
        laps.push(CompactLap {
            ms,
            crashed: flags & 1 != 0,
            cut: flags & 2 != 0,
        });
        o += 5;
    }
    laps
}

fn stretch(v: f32, lo: f32, hi: f32) -> f32 {
    if hi <= lo {
        return 0.0;
    }
    ((v - lo) / (hi - lo)).clamp(0.0, 1.0)
}

fn pace_score(your_best: i32, fastest: i32) -> Option<f32> {
    if your_best <= 0 || fastest <= 0 {
        return None;
    }
    Some(stretch(fastest as f32 / your_best as f32, PACE_FLOOR, 1.0))
}

fn consistency_score(clean: &[i32]) -> Option<f32> {
    if clean.len() < 3 {
        return None;
    }
    let n = clean.len() as f32;
    let mean = clean.iter().map(|ms| *ms as f32).sum::<f32>() / n;
    if mean <= 0.0 {
        return None;
    }
    let var = clean
        .iter()
        .map(|ms| {
            let d = *ms as f32 - mean;
            d * d
        })
        .sum::<f32>()
        / n;
    let cv = var.sqrt() / mean;
    Some((1.0 - cv / CV_CAP).clamp(0.0, 1.0))
}

fn closer_score(clean: &[i32]) -> Option<f32> {
    if clean.len() < 4 {
        return None;
    }
    let third = (clean.len() / 3).max(1);
    let first: f32 = clean.iter().take(third).map(|ms| *ms as f32).sum::<f32>() / third as f32;
    let last: f32 = clean
        .iter()
        .rev()
        .take(third)
        .map(|ms| *ms as f32)
        .sum::<f32>()
        / third as f32;
    if last <= 0.0 {
        return None;
    }
    Some(stretch(first / last, 0.96, 1.04))
}

fn attack_score(hot: i32, n: i32) -> Option<f32> {
    if n <= 0 {
        return None;
    }
    Some(stretch(hot as f32 / n as f32, ATTACK_LO, ATTACK_HI))
}

fn air_score(hot: i32, n: i32) -> Option<f32> {
    if n <= 0 {
        return None;
    }
    Some(stretch(hot as f32 / n as f32, 0.0, AIR_HI))
}

fn results_score(position: i32, rider_count: i32) -> Option<f32> {
    if rider_count < 2 || position <= 0 {
        return None;
    }
    Some(((rider_count - position) as f32 / (rider_count - 1) as f32).clamp(0.0, 1.0))
}

fn opening_score(laps: &[CompactLap]) -> Option<f32> {
    let first = laps.iter().find(|l| l.ms > 0)?;
    if first.crashed || first.cut {
        return Some(0.0);
    }
    let later: Vec<i32> = laps
        .iter()
        .skip(1)
        .filter(|l| l.ms > 0 && !l.crashed && !l.cut)
        .map(|l| l.ms)
        .collect();
    if later.is_empty() {
        return None;
    }
    let mut sorted = later;
    sorted.sort_unstable();
    let mid = sorted[sorted.len() / 2];
    if first.ms <= 0 {
        return None;
    }
    Some(stretch(mid as f32 / first.ms as f32, PACE_FLOOR, 1.0))
}

fn attack_counts(bins: &[ChannelBin; BINS]) -> (i32, i32) {
    let hot = bins.iter().filter(|b| b.throttle >= ATTACK_HOT).count() as i32;
    (hot, BINS as i32)
}

fn air_counts(bins: &[ChannelBin; BINS]) -> (i32, i32) {
    let mut hot = 0;
    for i in 0..BINS {
        let lo = i.saturating_sub(AIR_WIN / 2);
        let hi = (i + AIR_WIN / 2 + 1).min(BINS);
        let mut win: Vec<f32> = bins[lo..hi].iter().map(|b| b.y).collect();
        win.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let med = win[win.len() / 2];
        if bins[i].y > med + AIR_M {
            hot += 1;
        }
    }
    (hot, BINS as i32)
}

fn bins_signal(bins: &[ChannelBin; BINS]) -> bool {
    bins.iter()
        .any(|b| b.speed > 0.5 || b.throttle > 0.01 || b.y.abs() > 0.01)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lap(ms: i32, crashed: bool, cut: bool) -> CompactLap {
        CompactLap { ms, crashed, cut }
    }

    fn race(laps: Vec<CompactLap>) -> CompactRace {
        CompactRace {
            session_id: 1,
            started: 1,
            rider_count: 10,
            position: 3,
            your_best_ms: laps
                .iter()
                .filter(|l| !l.crashed && !l.cut && l.ms > 0)
                .map(|l| l.ms)
                .min()
                .unwrap_or(0),
            fastest_ms: 100_000,
            laps,
            attack_hot: 0,
            attack_n: 0,
            air_hot: 0,
            air_n: 0,
            name: String::new(),
            holeshot: None,
            state: 0,
            penalty_ms: 0,
        }
    }

    #[test]
    fn practice_and_solo_are_not_eligible() {
        let laps = vec![RaceLapInput {
            lap_num: 0,
            ms: 110_000,
            warmup: false,
            crashed: false,
            cut: false,
            bins: [ChannelBin::default(); BINS],
        }];
        assert!(!eligible(true, 7, 20, &laps));
        assert!(!eligible(false, WARMUP_KIND, 20, &laps));
        assert!(!eligible(false, 7, 1, &laps));
        assert!(eligible(false, 7, 8, &laps));
    }

    #[test]
    fn warmup_only_is_not_eligible() {
        let laps = vec![RaceLapInput {
            lap_num: 0,
            ms: 118_000,
            warmup: true,
            crashed: false,
            cut: false,
            bins: [ChannelBin::default(); BINS],
        }];
        assert!(!eligible(false, 7, 12, &laps));
        assert!(compact_from_input(&RaceInput {
            session_id: 1,
            started: 1,
            practice: false,
            kind: 7,
            rider_count: 12,
            your_race_num: 2,
            position: 4,
            your_best_ms: 118_000,
            fastest_ms: 110_000,
            laps,
            name: String::new(),
            holeshot: None,
            state: 0,
            penalty_ms: 0,
        })
        .is_none());
    }

    #[test]
    fn pace_is_one_when_you_are_fastest() {
        let s = scores_from_compact(&race(vec![lap(100_000, false, false)]));
        assert!((s[0].unwrap() - 1.0).abs() < 0.001);
    }

    #[test]
    fn consistency_needs_three_clean_laps() {
        let two = scores_from_compact(&race(vec![
            lap(100_000, false, false),
            lap(101_000, false, false),
        ]));
        assert!(two[1].is_none());
        let even = scores_from_compact(&race(vec![
            lap(100_000, false, false),
            lap(100_000, false, false),
            lap(100_000, false, false),
        ]));
        assert!((even[1].unwrap() - 1.0).abs() < 0.001);
    }

    #[test]
    fn crashed_l1_opening_is_zero() {
        let s = scores_from_compact(&race(vec![
            lap(140_000, true, false),
            lap(100_000, false, false),
            lap(101_000, false, false),
        ]));
        assert_eq!(s[7], Some(0.0));
    }

    #[test]
    fn results_percentile() {
        let mut r = race(vec![lap(100_000, false, false)]);
        r.position = 1;
        r.rider_count = 10;
        let s = scores_from_compact(&r);
        assert!((s[6].unwrap() - 1.0).abs() < 0.001);
        r.position = 10;
        let s = scores_from_compact(&r);
        assert!((s[6].unwrap() - 0.0).abs() < 0.001);
    }

    #[test]
    fn pack_roundtrip() {
        let laps = vec![lap(100, false, true), lap(110, true, false)];
        let back = unpack_laps(&pack_laps(&laps));
        assert_eq!(back.len(), 2);
        assert!(back[0].cut && !back[0].crashed);
        assert!(back[1].crashed && !back[1].cut);
        assert_eq!(back[0].ms, 100);
    }

    #[test]
    fn mean_skips_missing_axes() {
        let a = {
            let mut s = [None; AXIS_COUNT];
            s[0] = Some(1.0);
            s
        };
        let b = {
            let mut s = [None; AXIS_COUNT];
            s[0] = Some(0.5);
            s[2] = Some(1.0);
            s
        };
        let m = mean_scores(&[a, b]);
        assert!((m[0].unwrap() - 0.75).abs() < 0.001);
        assert_eq!(m[2], Some(1.0));
        assert!(m[1].is_none());
    }

    #[test]
    fn window_stats_win_podium_finish_and_laps() {
        let mut a = race(vec![
            lap(100_000, false, false),
            lap(101_000, true, false),
            lap(102_000, false, true),
        ]);
        a.position = 1;
        a.holeshot = Some(1);
        a.penalty_ms = 2000;
        let mut b = race(vec![lap(110_000, false, false), lap(111_000, false, false)]);
        b.position = 5;
        b.holeshot = Some(0);
        b.penalty_ms = 0;
        let mut c = race(vec![lap(120_000, false, false)]);
        c.position = 2;
        c.state = STATE_OUT;
        c.holeshot = None;
        c.penalty_ms = 1000;
        let races = [&a, &b, &c];
        let mut p = RiderProfile::empty();
        apply_window_stats(&mut p, &races);
        assert_eq!(p.wins, 1);
        assert!((p.win_rate.unwrap() - 1.0 / 3.0).abs() < 0.001);
        assert!((p.podium_rate.unwrap() - 1.0 / 3.0).abs() < 0.001); // DNF P2 excluded
        // classified only: P1 and P5
        assert!((p.avg_finish.unwrap() - 3.0).abs() < 0.001);
        assert!((p.median_finish.unwrap() - 3.0).abs() < 0.001);
        assert!(p.finish_pct.is_some());
        // 6 timed laps: clean 4, crash 1, cut 1
        assert!((p.clean_rate.unwrap() - 4.0 / 6.0).abs() < 0.001);
        assert!((p.crash_rate.unwrap() - 1.0 / 6.0).abs() < 0.001);
        assert!((p.cut_rate.unwrap() - 1.0 / 6.0).abs() < 0.001);
        assert!((p.holeshot_rate.unwrap() - 0.5).abs() < 0.001);
        assert_eq!(p.dnf_count, 1);
        assert!((p.dnf_rate.unwrap() - 1.0 / 3.0).abs() < 0.001);
        assert!((p.avg_penalty_ms.unwrap() - 1000.0).abs() < 0.001);
    }

    #[test]
    fn legacy_unknown_state_counts_as_classified() {
        let mut r = race(vec![lap(100_000, false, false)]);
        r.position = 4;
        r.state = 0;
        let mut p = RiderProfile::empty();
        apply_window_stats(&mut p, &[&r]);
        assert_eq!(p.avg_finish, Some(4.0));
        assert_eq!(p.dns_count, 0);
        assert_eq!(p.dnf_count, 0);
    }

    #[test]
    fn wins_follow_you_won_race_not_state() {
        let mut r = race(vec![lap(100_000, false, false)]);
        r.position = 1;
        r.state = STATE_OUT;
        let mut p = RiderProfile::empty();
        apply_window_stats(&mut p, &[&r]);
        assert!(you_won_race(1, r.rider_count));
        assert_eq!(p.wins, 1);
    }

    #[test]
    fn holeshot_rate_none_when_all_unknown() {
        let mut r = race(vec![lap(100_000, false, false)]);
        r.holeshot = None;
        let mut p = RiderProfile::empty();
        apply_window_stats(&mut p, &[&r]);
        assert!(p.holeshot_rate.is_none());
    }
}
