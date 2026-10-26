//! Last practice visit per track, class, and setup. Not a Motos session.
//!
//! Each completed practice lap stores its time and the liters burned since the
//! previous start/finish. A pit refill (fuel went up) or a zero/crash lap is dropped.
//! The fill recommendation is median burn times the race, plus a lap and a half, or about
//! 15% on a timed race, clamped to the tank.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use mxbo_hud::config::SessionPreset;
use mxbo_hud::snapshot::{bytes_as_text, Snapshot};
use mxbo_hud::track_pb::bike_class;
use mxbo_hud::{published_race_distance, session_preset, RaceDistance};

const READY_LAPS: usize = 3;

const MIN_LAP_MS: i32 = 20_000;
const MIN_BURN: f32 = 0.02;
const FILE_NAME: &str = "strategy.txt";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PracticeLap {
    pub ms: i32,
    pub liters: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RaceLength {
    Laps(i32),
    Minutes(i32),
}

impl RaceLength {
    fn is_laps(self) -> bool {
        matches!(self, Self::Laps(_))
    }

    fn from_distance(distance: RaceDistance) -> Self {
        match distance {
            RaceDistance::Laps(n) => Self::Laps(n),
            RaceDistance::Minutes(n) => Self::Minutes(n),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FillAdvice {
    pub median_lap_ms: i32,
    pub best_lap_ms: i32,
    pub spread_ms: i32,
    pub liters_per_lap: f32,
    pub liters_per_minute: f32,
    pub recommended_liters: f32,
    pub needed_liters: f32,
    pub max_fuel: f32,
    /// Race lap you run dry on, when a full tank is short of the race.
    pub dry_at_lap: Option<i32>,
    /// Race minute you run dry in, when a full tank is short of the clock.
    pub dry_at_minute: Option<i32>,
    /// The race fits in the tank. The extra lap and a half, or 15%, does not.
    pub short_margin: bool,
    pub projected_race_ms: Option<i32>,
    pub timed: bool,
    /// Laps a full tank finishes at the fill rate. None when the tank size is unknown.
    pub tank_laps: Option<f32>,
    /// Minutes a full tank lasts at the fill rate.
    pub tank_minutes: Option<f32>,
    /// Timed race length as a lap count at the median lap.
    pub timed_laps: Option<i32>,
    /// The fill used laps within a second of your best, because they burned more.
    pub used_push: bool,
    /// Last three laps' median minus the session median. Negative is quicker.
    pub recent_delta_ms: Option<i32>,
    /// Fastest-to-slowest after dropping the one slow lap, when that lap was the outlier.
    pub clean_spread_ms: Option<i32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StrategyPage {
    pub track: String,
    pub class_name: String,
    pub setup: String,
    pub server: String,
    pub laps: Vec<PracticeLap>,
    pub max_fuel: f32,
    pub laps_mode: bool,
    pub length: Option<RaceLength>,
    pub from_last_race: bool,
    pub advice: Option<FillAdvice>,
    /// Your best lap on the newest Motos race at this track.
    pub last_race_best_ms: Option<i32>,
    /// The laps on screen are a warmup, not open practice.
    pub from_warmup: bool,
    /// True once the shown visit has three laps. The fill stays off until then.
    pub ready: bool,
    /// Warmup laps so far, while a ready practice visit is still the fill.
    pub warmup_pending: Option<usize>,
}

#[derive(Clone, Debug)]
struct Visit {
    track: String,
    class_name: String,
    setup: String,
    server: String,
    max_fuel: f32,
    updated: u64,
    warmup: bool,
    laps: Vec<PracticeLap>,
}

#[derive(Clone, Debug)]
struct LengthRow {
    server: String,
    track: String,
    length: RaceLength,
}

struct OpenPractice {
    track: String,
    class_name: String,
    setup: String,
    server: String,
    max_fuel: f32,
    fuel_at_line: f32,
    saw_line: bool,
    current_lap: i32,
    last_lap_ms: i32,
    current_lap_ms: i32,
    warmup: bool,
    laps: Vec<PracticeLap>,
}

struct State {
    dir: PathBuf,
    visits: Vec<Visit>,
    races: Vec<LengthRow>,
    chosen: Vec<LengthRow>,
    open: Option<OpenPractice>,
    focus_track: String,
    focus_class: String,
    focus_setup: String,
    focus_server: String,
    /// Chip the rider picked when no length is stored yet.
    prefer_laps: Option<bool>,
}

impl State {
    fn empty() -> Self {
        Self {
            dir: PathBuf::new(),
            visits: Vec::new(),
            races: Vec::new(),
            chosen: Vec::new(),
            open: None,
            focus_track: String::new(),
            focus_class: String::new(),
            focus_setup: String::new(),
            focus_server: String::new(),
            prefer_laps: None,
        }
    }
}

static STATE: Mutex<State> = Mutex::new(State {
    dir: PathBuf::new(),
    visits: Vec::new(),
    races: Vec::new(),
    chosen: Vec::new(),
    open: None,
    focus_track: String::new(),
    focus_class: String::new(),
    focus_setup: String::new(),
    focus_server: String::new(),
    prefer_laps: None,
});

fn lock() -> std::sync::MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(|err| err.into_inner())
}

pub fn init(dir: PathBuf) {
    let mut st = lock();
    *st = load_state(&dir);
}

pub fn tick(s: &Snapshot) {
    let mut st = lock();
    if s.has_telemetry == 0 {
        finish_open(&mut st);
        return;
    }
    let Some(warmup) = sample_warmup(s) else {
        if let Some(distance) = published_race_distance(s) {
            remember_race(&mut st, s, distance);
        }
        finish_open(&mut st);
        return;
    };
    let track = bytes_as_text(&s.track_name);
    if track.is_empty() {
        return;
    }
    let class_name = bike_class(&s.local_bike());
    let setup = s.setup_label();
    let server = bytes_as_text(&s.server_name);
    let continue_visit = st.open.as_ref().is_some_and(|open| {
        open.warmup == warmup
            && open.track == track
            && (open.class_name == class_name || (open.class_name.is_empty() && !class_name.is_empty()))
            && (open.setup == setup || (open.setup.is_empty() && !setup.is_empty()))
    });
    if !continue_visit {
        finish_open(&mut st);
        st.open = Some(OpenPractice {
            track: track.to_string(),
            class_name: class_name.clone(),
            setup: setup.clone(),
            server: server.to_string(),
            max_fuel: s.max_fuel,
            fuel_at_line: s.fuel,
            saw_line: false,
            current_lap: s.current_lap,
            last_lap_ms: s.last_lap_ms,
            current_lap_ms: s.current_lap_ms,
            warmup,
            laps: Vec::new(),
        });
        st.focus_track = track.to_string();
        st.focus_class = class_name.clone();
        st.focus_setup = setup.clone();
        st.focus_server = server.to_string();
        st.prefer_laps = None;
    }
    let visit = {
        let Some(open) = st.open.as_mut() else {
            return;
        };
        if open.class_name.is_empty() && !class_name.is_empty() {
            open.class_name = class_name;
        }
        if open.setup.is_empty() && !setup.is_empty() {
            open.setup = setup;
        }
        if !server.is_empty() {
            open.server = server;
        }
        if s.max_fuel > 0.05 {
            open.max_fuel = s.max_fuel;
        }
        if !open.saw_line {
            if reached_line(open, s) || (s.current_lap_ms < 1_500 && s.fuel > 0.05) {
                note_line(open, s);
            }
            None
        } else if !reached_line(open, s) {
            None
        } else {
            if s.local_crashed == 0 {
                let burn = open.fuel_at_line - s.fuel;
                if s.last_lap_ms >= MIN_LAP_MS && burn >= MIN_BURN {
                    open.laps.push(PracticeLap {
                        ms: s.last_lap_ms,
                        liters: burn,
                    });
                }
            }
            note_line(open, s);
            Some(Visit {
                track: open.track.clone(),
                class_name: open.class_name.clone(),
                setup: open.setup.clone(),
                server: open.server.clone(),
                max_fuel: open.max_fuel,
                updated: unix_secs(),
                warmup: open.warmup,
                laps: open.laps.clone(),
            })
        }
    };
    if let Some(visit) = visit.filter(|visit| !visit.laps.is_empty()) {
        store_visit(&mut st, visit);
    }
}

pub fn page() -> StrategyPage {
    let st = lock();
    let (track, class_name, setup, server) = focus_of(&st);
    let (visit, warmup_pending) = choose_visit(&st, &track, &class_name, &setup);
    let from_warmup = visit.as_ref().is_some_and(|v| v.warmup);
    let laps = visit.as_ref().map(|v| v.laps.clone()).unwrap_or_default();
    let max_fuel = visit.as_ref().map(|v| v.max_fuel).unwrap_or(0.0);
    let server = if server.is_empty() {
        visit.map(|v| v.server).unwrap_or_default()
    } else {
        server
    };
    let ready = laps.len() >= READY_LAPS;
    let (laps_mode, length, from_last_race) = displayed_length(&st, &server, &track);
    let advice = ready
        .then(|| length.and_then(|length| advise(&laps, length, max_fuel)))
        .flatten();
    let last_race_best_ms = if track.is_empty() {
        None
    } else {
        crate::last_race_best_ms(&track)
    };
    StrategyPage {
        track,
        class_name,
        setup,
        server,
        laps,
        max_fuel,
        laps_mode,
        length,
        from_last_race,
        advice,
        last_race_best_ms,
        from_warmup,
        ready,
        warmup_pending,
    }
}

/// `Some(true)` is warmup, `Some(false)` is practice. Anything else is not sampled.
fn sample_warmup(s: &Snapshot) -> Option<bool> {
    match session_preset(s, false) {
        Some(SessionPreset::Warmup) => Some(true),
        Some(SessionPreset::Practice) => Some(false),
        _ => None,
    }
}

pub fn select_mode(laps: bool) {
    let mut st = lock();
    let (track, _, _, server) = focus_of(&st);
    if track.is_empty() {
        st.prefer_laps = Some(laps);
        return;
    }
    let remembered = find_length(&st.races, &server, &track);
    if remembered.is_some_and(|length| length.is_laps() == laps) {
        st.chosen.retain(|row| !(row.server == server && row.track == track));
        st.prefer_laps = Some(laps);
        save_state(&st);
        return;
    }
    let length = if laps {
        RaceLength::Laps(5)
    } else {
        RaceLength::Minutes(10)
    };
    upsert_length(&mut st.chosen, &server, &track, length);
    st.prefer_laps = Some(laps);
    save_state(&st);
}

pub fn nudge(delta: i32) {
    if delta == 0 {
        return;
    }
    let mut st = lock();
    let (track, _, _, server) = focus_of(&st);
    if track.is_empty() {
        return;
    }
    let (laps_mode, length, _) = displayed_length(&st, &server, &track);
    let next = match length {
        Some(RaceLength::Laps(n)) => RaceLength::Laps((n + delta).clamp(1, 40)),
        Some(RaceLength::Minutes(n)) => RaceLength::Minutes((n + delta).clamp(1, 120)),
        None if delta < 0 => return,
        None if laps_mode => RaceLength::Laps(5),
        None => RaceLength::Minutes(10),
    };
    upsert_length(&mut st.chosen, &server, &track, next);
    st.prefer_laps = Some(next.is_laps());
    save_state(&st);
}

pub fn advise(laps: &[PracticeLap], length: RaceLength, max_fuel: f32) -> Option<FillAdvice> {
    if laps.is_empty() {
        return None;
    }
    let mut times: Vec<i32> = laps.iter().map(|lap| lap.ms).filter(|ms| *ms > 0).collect();
    let mut burns: Vec<f32> = laps.iter().map(|lap| lap.liters).filter(|liters| *liters > 0.0).collect();
    if times.is_empty() || burns.is_empty() {
        return None;
    }
    times.sort_unstable();
    burns.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut rates: Vec<f32> = laps
        .iter()
        .filter_map(|lap| {
            let minutes = lap.ms as f32 / 60_000.0;
            (minutes > 0.05 && lap.liters > 0.0).then_some(lap.liters / minutes)
        })
        .collect();
    rates.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_lap_ms = median_i32(&times);
    let best_lap_ms = times[0];
    let spread_ms = times[times.len() - 1] - times[0];
    let clean_spread_ms = clean_spread(&times);
    let liters_per_lap = median_f32(&burns);
    let liters_per_minute = if rates.is_empty() {
        0.0
    } else {
        median_f32(&rates)
    };
    if liters_per_lap <= 0.0 {
        return None;
    }
    let (fill_per_lap, fill_per_minute, used_push) =
        push_fill(laps, best_lap_ms, liters_per_lap, liters_per_minute);
    let recent_delta_ms = recent_delta(laps, median_lap_ms);
    let (needed_liters, projected_race_ms, timed, timed_laps) = match length {
        RaceLength::Laps(n) => {
            let n = n.max(1);
            (
                fill_per_lap * (n as f32 + 1.5),
                Some(median_lap_ms.saturating_mul(n)),
                false,
                None,
            )
        }
        RaceLength::Minutes(minutes) => {
            let minutes_n = minutes.max(1);
            let minutes = minutes_n as f32;
            let laps_est = ((minutes * 60_000.0) / median_lap_ms.max(1) as f32).round() as i32;
            (
                fill_per_minute * minutes * 1.15,
                None,
                true,
                Some(laps_est.max(1)),
            )
        }
    };
    let tank = max_fuel > 0.05;
    let (tank_laps, tank_minutes) = if tank && fill_per_lap > 0.0 && fill_per_minute > 0.0 {
        (
            Some(max_fuel / fill_per_lap),
            Some(max_fuel / fill_per_minute),
        )
    } else {
        (None, None)
    };
    let recommended_liters = if tank {
        needed_liters.min(max_fuel)
    } else {
        needed_liters
    };
    let mut dry_at_lap = None;
    let mut dry_at_minute = None;
    let mut short_margin = false;
    if tank && needed_liters > max_fuel + 0.05 {
        match length {
            RaceLength::Laps(n) => {
                let cover = max_fuel / fill_per_lap;
                if cover < n.max(1) as f32 {
                    dry_at_lap = Some((cover.floor() as i32).max(0) + 1);
                } else {
                    short_margin = true;
                }
            }
            RaceLength::Minutes(minutes) if fill_per_minute > 0.0 => {
                let cover = max_fuel / fill_per_minute;
                if cover < minutes.max(1) as f32 {
                    dry_at_minute = Some((cover.floor() as i32).max(0) + 1);
                } else {
                    short_margin = true;
                }
            }
            RaceLength::Minutes(_) => {}
        }
    }
    Some(FillAdvice {
        median_lap_ms,
        best_lap_ms,
        spread_ms,
        liters_per_lap,
        liters_per_minute,
        recommended_liters,
        needed_liters,
        max_fuel,
        dry_at_lap,
        dry_at_minute,
        short_margin,
        projected_race_ms,
        timed,
        tank_laps,
        tank_minutes,
        timed_laps,
        used_push,
        recent_delta_ms,
        clean_spread_ms,
    })
}

const PUSH_WINDOW_MS: i32 = 1_000;

/// Fast laps are within a second of the best. Use their burn for the fill when it is higher.
fn push_fill(
    laps: &[PracticeLap],
    best_lap_ms: i32,
    liters_per_lap: f32,
    liters_per_minute: f32,
) -> (f32, f32, bool) {
    let push: Vec<&PracticeLap> = laps
        .iter()
        .filter(|lap| lap.ms > 0 && lap.ms <= best_lap_ms.saturating_add(PUSH_WINDOW_MS))
        .collect();
    let rest = laps.len().saturating_sub(push.len());
    if push.is_empty() || rest == 0 {
        return (liters_per_lap, liters_per_minute, false);
    }
    let mut burns: Vec<f32> = push.iter().map(|lap| lap.liters).filter(|n| *n > 0.0).collect();
    if burns.is_empty() {
        return (liters_per_lap, liters_per_minute, false);
    }
    burns.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let push_per_lap = median_f32(&burns);
    if push_per_lap <= liters_per_lap + MIN_BURN {
        return (liters_per_lap, liters_per_minute, false);
    }
    let mut rates: Vec<f32> = push
        .iter()
        .filter_map(|lap| {
            let minutes = lap.ms as f32 / 60_000.0;
            (minutes > 0.05 && lap.liters > 0.0).then_some(lap.liters / minutes)
        })
        .collect();
    rates.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let push_per_minute = if rates.is_empty() {
        liters_per_minute
    } else {
        median_f32(&rates)
    };
    (push_per_lap, push_per_minute, true)
}

fn recent_delta(laps: &[PracticeLap], median_lap_ms: i32) -> Option<i32> {
    if laps.len() < 4 {
        return None;
    }
    let mut last: Vec<i32> = laps.iter().rev().take(3).map(|lap| lap.ms).filter(|ms| *ms > 0).collect();
    if last.len() < 3 {
        return None;
    }
    last.sort_unstable();
    let delta = median_i32(&last) - median_lap_ms;
    (delta.abs() >= 50).then_some(delta)
}

fn clean_spread(sorted_times: &[i32]) -> Option<i32> {
    if sorted_times.len() < 3 {
        return None;
    }
    let full = sorted_times[sorted_times.len() - 1] - sorted_times[0];
    let tight = sorted_times[sorted_times.len() - 2] - sorted_times[0];
    (full - tight >= 200).then_some(tight)
}

/// Practice median minus the last race best. Negative means practice is quicker.
pub fn race_gap_ms(practice_median_ms: i32, race_best_ms: i32) -> Option<i32> {
    if practice_median_ms <= 0 || race_best_ms < MIN_LAP_MS {
        return None;
    }
    let delta = practice_median_ms - race_best_ms;
    (delta.abs() >= 50).then_some(delta)
}

fn reached_line(open: &OpenPractice, s: &Snapshot) -> bool {
    if s.current_lap > open.current_lap {
        return true;
    }
    open.current_lap_ms > 8_000
        && s.current_lap_ms < 1_500
        && s.last_lap_ms != open.last_lap_ms
}

fn note_line(open: &mut OpenPractice, s: &Snapshot) {
    open.fuel_at_line = s.fuel.max(0.0);
    open.saw_line = true;
    open.current_lap = s.current_lap;
    open.last_lap_ms = s.last_lap_ms;
    open.current_lap_ms = s.current_lap_ms;
}

fn finish_open(st: &mut State) {
    let Some(open) = st.open.take() else {
        return;
    };
    if open.laps.is_empty() {
        return;
    }
    store_visit(
        st,
        Visit {
            track: open.track,
            class_name: open.class_name,
            setup: open.setup,
            server: open.server,
            max_fuel: open.max_fuel,
            updated: unix_secs(),
            warmup: open.warmup,
            laps: open.laps,
        },
    );
}

fn remember_race(st: &mut State, s: &Snapshot, distance: RaceDistance) {
    let track = bytes_as_text(&s.track_name);
    if track.is_empty() {
        return;
    }
    let server = bytes_as_text(&s.server_name);
    let length = RaceLength::from_distance(distance);
    st.focus_track = track.to_string();
    if st.focus_server.is_empty() {
        st.focus_server = server.to_string();
    }
    if find_length(&st.races, &server, &track) == Some(length) {
        return;
    }
    upsert_length(&mut st.races, &server, &track, length);
    save_state(st);
}

fn store_visit(st: &mut State, visit: Visit) {
    st.focus_track = visit.track.clone();
    st.focus_class = visit.class_name.clone();
    st.focus_setup = visit.setup.clone();
    if !visit.server.is_empty() {
        st.focus_server = visit.server.clone();
    }
    if let Some(slot) = st.visits.iter_mut().find(|row| same_key(row, &visit)) {
        *slot = visit;
    } else {
        st.visits.push(visit);
    }
    save_state(st);
}

fn focus_of(st: &State) -> (String, String, String, String) {
    if !st.focus_track.is_empty() {
        return (
            st.focus_track.clone(),
            st.focus_class.clone(),
            st.focus_setup.clone(),
            st.focus_server.clone(),
        );
    }
    st.visits
        .iter()
        .max_by_key(|visit| visit.updated)
        .map(|visit| {
            (
                visit.track.clone(),
                visit.class_name.clone(),
                visit.setup.clone(),
                visit.server.clone(),
            )
        })
        .unwrap_or_default()
}

fn same_key(row: &Visit, visit: &Visit) -> bool {
    row.track == visit.track
        && row.class_name == visit.class_name
        && row.setup == visit.setup
        && row.warmup == visit.warmup
}

fn find_visit<'a>(
    st: &'a State,
    track: &str,
    class_name: &str,
    setup: &str,
    warmup: bool,
) -> Option<&'a Visit> {
    if track.is_empty() {
        return None;
    }
    st.visits.iter().find(|visit| {
        visit.track == track
            && visit.class_name == class_name
            && visit.setup == setup
            && visit.warmup == warmup
    })
}

/// Warmup with three laps wins. Otherwise a ready practice visit stays, and a short warmup is noted.
fn choose_visit(st: &State, track: &str, class_name: &str, setup: &str) -> (Option<Visit>, Option<usize>) {
    let practice = find_visit(st, track, class_name, setup, false).cloned();
    let warmup = find_visit(st, track, class_name, setup, true).cloned();
    let practice_n = practice.as_ref().map(|v| v.laps.len()).unwrap_or(0);
    let warmup_n = warmup.as_ref().map(|v| v.laps.len()).unwrap_or(0);
    if warmup_n >= READY_LAPS {
        return (warmup, None);
    }
    if practice_n >= READY_LAPS {
        let pending = (warmup_n > 0).then_some(warmup_n);
        return (practice, pending);
    }
    if warmup_n > 0 {
        return (warmup, None);
    }
    (practice, None)
}

fn displayed_length(st: &State, server: &str, track: &str) -> (bool, Option<RaceLength>, bool) {
    if track.is_empty() {
        return (st.prefer_laps.unwrap_or(true), None, false);
    }
    if let Some(length) = find_length(&st.chosen, server, track) {
        return (length.is_laps(), Some(length), false);
    }
    if let Some(length) = find_length(&st.races, server, track) {
        let mode = st.prefer_laps.unwrap_or(length.is_laps());
        if mode == length.is_laps() {
            return (mode, Some(length), true);
        }
        return (mode, None, false);
    }
    (st.prefer_laps.unwrap_or(true), None, false)
}

fn find_length(rows: &[LengthRow], server: &str, track: &str) -> Option<RaceLength> {
    rows.iter()
        .find(|row| row.server == server && row.track == track)
        .map(|row| row.length)
}

fn upsert_length(rows: &mut Vec<LengthRow>, server: &str, track: &str, length: RaceLength) {
    if let Some(row) = rows
        .iter_mut()
        .find(|row| row.server == server && row.track == track)
    {
        row.length = length;
        return;
    }
    rows.push(LengthRow {
        server: server.to_string(),
        track: track.to_string(),
        length,
    });
}

fn median_i32(sorted: &[i32]) -> i32 {
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        sorted[mid - 1].saturating_add(sorted[mid]) / 2
    } else {
        sorted[mid]
    }
}

fn median_f32(sorted: &[f32]) -> f32 {
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        (sorted[mid - 1] + sorted[mid]) * 0.5
    } else {
        sorted[mid]
    }
}

fn unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn load_state(dir: &Path) -> State {
    let mut st = State::empty();
    st.dir = dir.to_path_buf();
    let Ok(text) = fs::read_to_string(dir.join(FILE_NAME)) else {
        return st;
    };
    for line in text.lines() {
        let mut parts = line.split('\t');
        match parts.next() {
            Some(tag @ ("P" | "W")) => {
                let Some(mut visit) = parse_visit(&mut parts) else {
                    continue;
                };
                visit.warmup = tag == "W";
                st.visits.push(visit);
            }
            Some("R") => {
                if let Some(row) = parse_length(&mut parts) {
                    st.races.push(row);
                }
            }
            Some("C") => {
                if let Some(row) = parse_length(&mut parts) {
                    st.chosen.push(row);
                }
            }
            _ => {}
        }
    }
    if let Some(visit) = st.visits.iter().max_by_key(|visit| visit.updated) {
        st.focus_track = visit.track.clone();
        st.focus_class = visit.class_name.clone();
        st.focus_setup = visit.setup.clone();
        st.focus_server = visit.server.clone();
    }
    st
}

fn parse_visit<'a>(parts: &mut impl Iterator<Item = &'a str>) -> Option<Visit> {
    let track = parts.next()?.to_string();
    let class_name = parts.next()?.to_string();
    let setup = parts.next()?.to_string();
    let server = parts.next()?.to_string();
    let max_fuel = parts.next()?.parse().ok()?;
    let updated = parts.next()?.parse().ok()?;
    let laps = parts
        .next()
        .unwrap_or("")
        .split(',')
        .filter_map(|pair| {
            let (ms, liters) = pair.split_once(':')?;
            Some(PracticeLap {
                ms: ms.parse().ok()?,
                liters: liters.parse().ok()?,
            })
        })
        .filter(|lap| lap.ms >= MIN_LAP_MS && lap.liters >= MIN_BURN)
        .collect();
    Some(Visit {
        track,
        class_name,
        setup,
        server,
        max_fuel,
        updated,
        warmup: false,
        laps,
    })
}

fn parse_length<'a>(parts: &mut impl Iterator<Item = &'a str>) -> Option<LengthRow> {
    let server = parts.next()?.to_string();
    let track = parts.next()?.to_string();
    let kind = parts.next()?;
    let n: i32 = parts.next()?.parse().ok()?;
    let length = match kind {
        "L" if (1..=40).contains(&n) => RaceLength::Laps(n),
        "M" if (1..=120).contains(&n) => RaceLength::Minutes(n),
        _ => return None,
    };
    Some(LengthRow {
        server,
        track,
        length,
    })
}

fn save_state(st: &State) {
    if st.dir.as_os_str().is_empty() {
        return;
    }
    let _ = fs::create_dir_all(&st.dir);
    let mut out = String::from("v1\n");
    for visit in &st.visits {
        if visit.laps.is_empty() {
            continue;
        }
        let laps = visit
            .laps
            .iter()
            .map(|lap| format!("{}:{:.3}", lap.ms, lap.liters))
            .collect::<Vec<_>>()
            .join(",");
        let tag = if visit.warmup { "W" } else { "P" };
        out.push_str(&format!(
            "{tag}\t{}\t{}\t{}\t{}\t{:.3}\t{}\t{laps}\n",
            field(&visit.track),
            field(&visit.class_name),
            field(&visit.setup),
            field(&visit.server),
            visit.max_fuel,
            visit.updated,
        ));
    }
    write_lengths(&mut out, "R", &st.races);
    write_lengths(&mut out, "C", &st.chosen);
    let _ = fs::write(st.dir.join(FILE_NAME), out);
}

fn write_lengths(out: &mut String, tag: &str, rows: &[LengthRow]) {
    for row in rows {
        let (kind, n) = match row.length {
            RaceLength::Laps(n) => ("L", n),
            RaceLength::Minutes(n) => ("M", n),
        };
        out.push_str(&format!(
            "{tag}\t{}\t{}\t{kind}\t{n}\n",
            field(&row.server),
            field(&row.track),
        ));
    }
}

fn field(text: &str) -> String {
    text.replace(['\t', '\n', '\r'], " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use mxbo_hud::snapshot::Standing;

    fn gate() -> std::sync::MutexGuard<'static, ()> {
        static GATE: Mutex<()> = Mutex::new(());
        GATE.lock().unwrap_or_else(|err| err.into_inner())
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mxbo-strategy-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn put(dest: &mut [u8], text: &str) {
        let bytes = text.as_bytes();
        let n = bytes.len().min(dest.len().saturating_sub(1));
        dest[..n].copy_from_slice(&bytes[..n]);
    }

    fn practice(track: &str) -> Snapshot {
        let mut snap = Snapshot::default();
        snap.has_telemetry = 1;
        snap.session_kind = -1;
        snap.session_laps = 0;
        snap.session_length = 0;
        snap.local_race_num = 1;
        snap.standing_count = 1;
        snap.standings[0] = Standing {
            race_num: 1,
            ..Standing::default()
        };
        put(&mut snap.standings[0].bike, "YZ450F");
        put(&mut snap.track_name, track);
        put(&mut snap.setup_name, "race.xml");
        put(&mut snap.server_name, "Practice Server");
        snap.max_fuel = 7.0;
        snap.fuel = 6.0;
        snap.current_lap_ms = 200;
        snap
    }

    #[test]
    fn lap_race_adds_one_spare_lap_and_clamps() {
        let laps = [
            PracticeLap { ms: 90_000, liters: 0.40 },
            PracticeLap { ms: 92_000, liters: 0.50 },
            PracticeLap { ms: 88_000, liters: 0.30 },
        ];
        let advice = advise(&laps, RaceLength::Laps(8), 3.0).unwrap();
        assert!((advice.liters_per_lap - 0.40).abs() < 0.001);
        assert_eq!(advice.best_lap_ms, 88_000);
        assert_eq!(advice.median_lap_ms, 90_000);
        assert_eq!(advice.spread_ms, 4_000);
        assert!((advice.needed_liters - 3.8).abs() < 0.001);
        assert!((advice.recommended_liters - 3.0).abs() < 0.001);
        assert_eq!(advice.dry_at_lap, Some(8));
        assert!(!advice.short_margin);
        assert_eq!(advice.projected_race_ms, Some(720_000));
        assert!(!advice.used_push);
        assert_eq!(advice.timed_laps, None);
        assert!((advice.tank_laps.unwrap() - 7.5).abs() < 0.01);
        assert_eq!(advice.clean_spread_ms, Some(2_000));
        assert!(advice.recent_delta_ms.is_none());
    }

    #[test]
    fn spare_lap_short_does_not_call_the_race_dry() {
        let laps = [PracticeLap { ms: 90_000, liters: 0.40 }];
        let advice = advise(&laps, RaceLength::Laps(6), 2.5).unwrap();
        assert!(advice.dry_at_lap.is_none());
        assert!(advice.short_margin);
        assert!((advice.needed_liters - 3.0).abs() < 0.001);
    }

    #[test]
    fn timed_race_adds_ten_percent() {
        let laps = [PracticeLap { ms: 60_000, liters: 0.50 }];
        let advice = advise(&laps, RaceLength::Minutes(20), 12.0).unwrap();
        assert!((advice.liters_per_minute - 0.50).abs() < 0.001);
        assert!((advice.needed_liters - 11.5).abs() < 0.001);
        assert!((advice.recommended_liters - 11.5).abs() < 0.001);
        assert!(advice.projected_race_ms.is_none());
        assert!(advice.dry_at_minute.is_none());
        assert_eq!(advice.timed_laps, Some(20));
        assert!((advice.tank_laps.unwrap() - 24.0).abs() < 0.01);
        assert!((advice.tank_minutes.unwrap() - 24.0).abs() < 0.01);
    }

    #[test]
    fn timed_race_names_the_dry_minute() {
        let laps = [PracticeLap { ms: 60_000, liters: 0.50 }];
        let advice = advise(&laps, RaceLength::Minutes(20), 8.0).unwrap();
        assert_eq!(advice.dry_at_minute, Some(17));
        assert!((advice.recommended_liters - 8.0).abs() < 0.001);
    }

    #[test]
    fn push_laps_raise_the_fill_when_they_burn_more() {
        let laps = [
            PracticeLap { ms: 90_000, liters: 0.55 },
            PracticeLap { ms: 90_400, liters: 0.50 },
            PracticeLap { ms: 95_000, liters: 0.30 },
            PracticeLap { ms: 96_000, liters: 0.28 },
        ];
        let advice = advise(&laps, RaceLength::Laps(8), 10.0).unwrap();
        assert!(advice.used_push);
        assert!((advice.needed_liters - 4.9875).abs() < 0.02);
        assert_eq!(advice.recent_delta_ms, Some(2_300));
        assert_eq!(advice.clean_spread_ms, Some(5_000));
        assert!((advice.tank_laps.unwrap() - (10.0 / 0.525)).abs() < 0.05);
    }

    #[test]
    fn race_gap_ignores_a_tie() {
        assert_eq!(race_gap_ms(90_000, 90_000), None);
        assert_eq!(race_gap_ms(90_000, 90_600), Some(-600));
        assert_eq!(race_gap_ms(91_000, 90_000), Some(1_000));
        assert_eq!(race_gap_ms(90_000, 0), None);
    }

    #[test]
    fn practice_keeps_full_laps_and_drops_a_refill() {
        let _gate = gate();
        let dir = temp_dir("laps");
        init(dir);
        let mut snap = practice("Unadilla");
        tick(&snap);
        snap.current_lap = 1;
        snap.current_lap_ms = 120;
        snap.last_lap_ms = 90_000;
        snap.fuel = 5.6;
        tick(&snap);
        snap.current_lap = 2;
        snap.current_lap_ms = 80;
        snap.last_lap_ms = 88_000;
        snap.fuel = 5.2;
        tick(&snap);
        snap.current_lap = 3;
        snap.last_lap_ms = 91_000;
        snap.fuel = 6.4;
        tick(&snap);
        snap.current_lap = 4;
        snap.last_lap_ms = 0;
        snap.fuel = 6.0;
        snap.local_crashed = 1;
        tick(&snap);
        let shown = page();
        assert_eq!(shown.track, "Unadilla");
        assert_eq!(shown.class_name, "450");
        assert_eq!(shown.setup, "race");
        assert_eq!(shown.laps.len(), 2);
        assert!((shown.laps[0].liters - 0.4).abs() < 0.001);
        assert_eq!(shown.laps[1].ms, 88_000);
    }

    #[test]
    fn joining_mid_lap_skips_the_partial() {
        let _gate = gate();
        init(temp_dir("partial"));
        let mut snap = practice("Unadilla");
        snap.current_lap_ms = 40_000;
        snap.fuel = 5.0;
        tick(&snap);
        snap.current_lap = 1;
        snap.current_lap_ms = 100;
        snap.last_lap_ms = 90_000;
        snap.fuel = 4.6;
        tick(&snap);
        assert!(page().laps.is_empty());
        snap.current_lap = 2;
        snap.last_lap_ms = 89_000;
        snap.fuel = 4.2;
        tick(&snap);
        let shown = page();
        assert_eq!(shown.laps.len(), 1);
        assert!((shown.laps[0].liters - 0.4).abs() < 0.001);
    }

    #[test]
    fn last_race_prefills_until_the_rider_changes_it() {
        let _gate = gate();
        let dir = temp_dir("race");
        init(dir.clone());
        let mut snap = practice("Unadilla");
        tick(&snap);
        snap.current_lap = 1;
        snap.current_lap_ms = 100;
        snap.last_lap_ms = 90_000;
        snap.fuel = 5.5;
        tick(&snap);
        snap.has_telemetry = 0;
        tick(&snap);
        let mut race = Snapshot::default();
        race.has_telemetry = 1;
        race.session_kind = 6;
        race.session_laps = 8;
        put(&mut race.track_name, "Unadilla");
        put(&mut race.server_name, "Practice Server");
        tick(&race);
        let shown = page();
        assert_eq!(shown.length, Some(RaceLength::Laps(8)));
        assert!(shown.from_last_race);
        nudge(1);
        let shown = page();
        assert_eq!(shown.length, Some(RaceLength::Laps(9)));
        assert!(!shown.from_last_race);
        select_mode(true);
        assert_eq!(page().length, Some(RaceLength::Laps(8)));
        assert!(page().from_last_race);
        let text = fs::read_to_string(dir.join(FILE_NAME)).unwrap();
        assert!(text.contains("Unadilla"));
        assert!(!text.contains("motos"));
    }

    #[test]
    fn a_new_practice_replaces_the_previous_visit() {
        let _gate = gate();
        init(temp_dir("replace"));
        let mut snap = practice("Unadilla");
        tick(&snap);
        snap.current_lap = 1;
        snap.current_lap_ms = 100;
        snap.last_lap_ms = 90_000;
        snap.fuel = 5.5;
        tick(&snap);
        snap.has_telemetry = 0;
        tick(&snap);
        let mut again = practice("Unadilla");
        again.fuel = 6.5;
        tick(&again);
        again.current_lap = 1;
        again.current_lap_ms = 90;
        again.last_lap_ms = 91_000;
        again.fuel = 6.0;
        tick(&again);
        let shown = page();
        assert_eq!(shown.laps.len(), 1);
        assert_eq!(shown.laps[0].ms, 91_000);
    }

    fn warmup(track: &str) -> Snapshot {
        let mut snap = practice(track);
        snap.session_kind = 5;
        snap
    }

    fn cross(snap: &mut Snapshot, lap: i32, ms: i32, fuel: f32) {
        snap.current_lap = lap;
        snap.current_lap_ms = 80;
        snap.last_lap_ms = ms;
        snap.fuel = fuel;
        tick(snap);
    }

    #[test]
    fn two_warmup_laps_give_no_advice_and_the_third_does() {
        let _gate = gate();
        init(temp_dir("warmup-gate"));
        let mut snap = warmup("Unadilla");
        tick(&snap);
        cross(&mut snap, 1, 90_000, 5.6);
        let shown = page();
        assert!(shown.from_warmup);
        assert_eq!(shown.laps.len(), 1);
        cross(&mut snap, 2, 90_000, 5.2);
        nudge(1);
        let shown = page();
        assert_eq!(shown.laps.len(), 2);
        assert!(!shown.ready);
        assert!(shown.advice.is_none());
        cross(&mut snap, 3, 90_000, 4.8);
        let shown = page();
        assert!(shown.ready);
        assert!(shown.from_warmup);
        assert!(shown.advice.is_some());
    }

    #[test]
    fn warmup_does_not_replace_a_stored_practice_visit() {
        let _gate = gate();
        let dir = temp_dir("warmup-keeps-practice");
        init(dir.clone());
        let mut snap = practice("Unadilla");
        tick(&snap);
        cross(&mut snap, 1, 90_000, 5.6);
        cross(&mut snap, 2, 90_000, 5.2);
        cross(&mut snap, 3, 90_000, 4.8);
        nudge(1);
        assert!(page().advice.is_some());
        assert!(!page().from_warmup);

        let mut warm = warmup("Unadilla");
        tick(&warm);
        cross(&mut warm, 1, 91_000, 5.5);
        cross(&mut warm, 2, 91_000, 5.1);
        let shown = page();
        assert!(!shown.from_warmup);
        assert_eq!(shown.laps.len(), 3);
        assert_eq!(shown.warmup_pending, Some(2));
        assert!(shown.advice.is_some());

        cross(&mut warm, 3, 91_000, 4.7);
        let shown = page();
        assert!(shown.from_warmup);
        assert!(shown.warmup_pending.is_none());
        assert_eq!(shown.laps.len(), 3);
        assert_eq!(shown.laps[0].ms, 91_000);

        let text = fs::read_to_string(dir.join(FILE_NAME)).unwrap();
        assert!(text.lines().any(|line| line.starts_with("P\t")));
        assert!(text.lines().any(|line| line.starts_with("W\t")));
        let practice_laps = text
            .lines()
            .find(|line| line.starts_with("P\t"))
            .unwrap()
            .split('\t')
            .last()
            .unwrap();
        assert_eq!(practice_laps.matches(':').count(), 3);
    }

    #[test]
    fn an_old_practice_line_stays_practice() {
        let _gate = gate();
        let dir = temp_dir("old-practice");
        fs::write(
            dir.join(FILE_NAME),
            "v1\nP\tUnadilla\t450\trace\tPractice Server\t7.000\t1\t90000:0.400,91000:0.400,89000:0.400\n",
        )
        .unwrap();
        init(dir);
        let shown = page();
        assert!(!shown.from_warmup);
        assert!(shown.ready);
        assert_eq!(shown.laps.len(), 3);
    }
}
