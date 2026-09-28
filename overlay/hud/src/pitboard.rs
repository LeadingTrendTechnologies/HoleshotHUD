use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::config::{ini_path, BoardField, TableText};

/// Old ini dump of every catalog stat. Row count follows the pack, not this list.
pub const LEGACY_CATALOG: usize = 29;
use crate::shm::Snapshot;

pub const FACTORY_ART: &str = "holeshot.png";
/// Baked plate for Browse. Same art as the factory, colors not live.
pub const SAMPLE_ART: &str = "sample.png";
/// Plate yellow `#F5BE07`.
pub const PLATE_YELLOW: [u8; 3] = [0xF5, 0xBE, 0x07];
/// Plate navy `#0C2A5A`. The word uses this ink. The banner badge is the app logo.
pub const PLATE_NAVY: [u8; 3] = [0x0C, 0x2A, 0x5A];
const FACTORY_PNG: &[u8] = include_bytes!("../assets/holeshot.png");
const SAMPLE_PNG: &[u8] = include_bytes!("../assets/sample.png");
const FACTORY_JSON: &str = include_str!("../assets/board.json");

/// Factory plate (empty name or `holeshot.png`). A browsed PNG is not recolored.
pub fn factory_plate(name: &str) -> bool {
    let name = name.trim();
    name.is_empty() || name == FACTORY_ART
}

/// Shift plate yellow and navy. White and clear pixels stay. Defaults are a no-op.
pub fn recolor_plate_pixel(rgba: [u8; 4], yellow: [u8; 3], navy: [u8; 3]) -> [u8; 4] {
    if rgba[3] == 0 {
        return rgba;
    }
    let rgb = [rgba[0], rgba[1], rgba[2]];
    let yellow_d = ink_distance(rgb, PLATE_YELLOW, 30.0, 0.22);
    let navy_d = ink_distance(rgb, PLATE_NAVY, 34.0, 0.15);
    let out = match (yellow_d, navy_d) {
        (Some(dy), Some(dn)) if dy <= dn => shift_ink(rgb, PLATE_YELLOW, yellow),
        (Some(_), Some(_)) => shift_ink(rgb, PLATE_NAVY, navy),
        (Some(_), None) => shift_ink(rgb, PLATE_YELLOW, yellow),
        (None, Some(_)) => shift_ink(rgb, PLATE_NAVY, navy),
        (None, None) => rgb,
    };
    [out[0], out[1], out[2], rgba[3]]
}

fn ink_distance(rgb: [u8; 3], key: [u8; 3], window: f32, min_sat: f32) -> Option<f32> {
    let (hue, sat, _) = crate::config::rgb_to_hsv(rgb);
    if sat < min_sat {
        return None;
    }
    let (key_hue, _, _) = crate::config::rgb_to_hsv(key);
    let delta = hue_delta(hue, key_hue);
    if delta <= window {
        Some(delta)
    } else {
        None
    }
}

fn hue_delta(a: f32, b: f32) -> f32 {
    let d = (a - b).abs() % 360.0;
    d.min(360.0 - d)
}

fn shift_ink(rgb: [u8; 3], from: [u8; 3], to: [u8; 3]) -> [u8; 3] {
    if from == to {
        return rgb;
    }
    let (_, sat, val) = crate::config::rgb_to_hsv(rgb);
    let (_, from_sat, from_val) = crate::config::rgb_to_hsv(from);
    let (to_hue, to_sat, to_val) = crate::config::rgb_to_hsv(to);
    let sat = if from_sat < 0.001 {
        to_sat
    } else {
        (sat * to_sat / from_sat).clamp(0.0, 1.0)
    };
    let val = if from_val < 0.001 {
        to_val
    } else {
        (val * to_val / from_val).clamp(0.0, 1.0)
    };
    crate::config::hsv_to_rgb(to_hue, sat, val)
}

const HOLD: Duration = Duration::from_secs(5);
const CLOCK_ON: i32 = 200;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PitWhen {
    Always,
    Sector,
    Lap,
}

impl PitWhen {
    pub fn label(self) -> &'static str {
        match self {
            Self::Always => "Always",
            Self::Sector => "End of each sector",
            Self::Lap => "End of each lap",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Always => "always",
            Self::Sector => "sector",
            Self::Lap => "lap",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "sector" | "sectors" => Self::Sector,
            "lap" | "laps" => Self::Lap,
            _ => Self::Always,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FlashSnap {
    pub time_ms: i32,
    pub delta_ms: i32,
    pub has_delta: bool,
}

struct Gate {
    primed: bool,
    last_clock: bool,
    last_cur: [i32; 3],
    last_last_lap: i32,
    sector_at: Option<Instant>,
    lap_at: Option<Instant>,
    sector_snap: Option<FlashSnap>,
    lap_snap: Option<FlashSnap>,
    preview: bool,
}

impl Gate {
    const fn new() -> Self {
        Self {
            primed: false,
            last_clock: false,
            last_cur: [0; 3],
            last_last_lap: 0,
            sector_at: None,
            lap_at: None,
            sector_snap: None,
            lap_snap: None,
            preview: false,
        }
    }
}

static GATE: Mutex<Gate> = Mutex::new(Gate::new());

fn gate() -> std::sync::MutexGuard<'static, Gate> {
    GATE.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn set_preview(on: bool) {
    gate().preview = on;
}

/// `cur` just got an official split, or the lap clock stopped (S3 / line).
fn sector_edge(last_clock: bool, last_cur: [i32; 3], clock: bool, cur: [i32; 3]) -> bool {
    let split = (0..3).any(|i| cur[i] > 0 && last_cur[i] <= 0);
    let line = last_clock && !clock;
    split || line
}

/// The lap that just finished: `last_lap` posted, or the clock stopped at the line.
fn lap_done(prev_last: i32, last: i32, last_clock: bool, clock: bool) -> bool {
    (last > 0 && last != prev_last) || (last_clock && !clock)
}

pub fn tick(s: &Snapshot) {
    if s.has_telemetry == 0 {
        return;
    }
    let clock = s.current_lap_ms > CLOCK_ON;
    let cur = [
        s.sector_cur.first().copied().unwrap_or(0),
        s.sector_cur.get(1).copied().unwrap_or(0),
        s.sector_cur.get(2).copied().unwrap_or(0),
    ];
    let mut g = gate();
    if !g.primed {
        g.primed = true;
        g.last_clock = clock;
        g.last_cur = cur;
        g.last_last_lap = s.last_lap_ms;
        return;
    }
    let now = Instant::now();
    if sector_edge(g.last_clock, g.last_cur, clock, cur) {
        let idx = (0..3)
            .find(|&i| cur[i] > 0 && g.last_cur[i] <= 0)
            .unwrap_or(2);
        g.sector_snap = Some(capture_sector(s, idx, cur[idx]));
        g.sector_at = Some(now);
    }
    if lap_done(g.last_last_lap, s.last_lap_ms, g.last_clock, clock) {
        g.lap_snap = Some(capture_lap(s));
        g.lap_at = Some(now);
    }
    g.last_clock = clock;
    g.last_cur = cur;
    g.last_last_lap = s.last_lap_ms;
}

fn capture_sector(s: &Snapshot, idx: usize, fallback: i32) -> FlashSnap {
    let session = crate::config::with_config(|c| c.sector_session);
    let row = crate::sector::row_vs(s, idx, false, session);
    let time_ms = if row.time_ms > 0 { row.time_ms } else { fallback };
    FlashSnap {
        time_ms,
        delta_ms: row.delta_ms,
        has_delta: row.has_delta && time_ms > 0,
    }
}

fn capture_lap(s: &Snapshot) -> FlashSnap {
    let session = crate::config::with_config(|c| c.delta_session);
    let view = crate::delta::view_for(session);
    let time_ms = if view.last_lap_ms > 0 {
        view.last_lap_ms
    } else {
        s.last_lap_ms
    };
    let (delta_ms, has_delta) = if time_ms > 0 && view.ref_lap_ms > 0 {
        (time_ms - view.ref_lap_ms, true)
    } else {
        (0, false)
    };
    FlashSnap {
        time_ms,
        delta_ms,
        has_delta,
    }
}

fn held(at: Option<Instant>) -> bool {
    at.is_some_and(|t| t.elapsed() < HOLD)
}

/// Frozen sector or last-lap times while a flash is up. Always / F8 stay live.
pub fn time_snap(when: PitWhen) -> Option<FlashSnap> {
    let g = gate();
    if g.preview {
        return None;
    }
    match when {
        PitWhen::Always => None,
        PitWhen::Sector if held(g.sector_at) => g.sector_snap,
        PitWhen::Lap if held(g.lap_at) => g.lap_snap,
        _ => None,
    }
}

pub fn drawing(show: bool, when: PitWhen) -> bool {
    if !show {
        return false;
    }
    let g = gate();
    if g.preview {
        return true;
    }
    match when {
        PitWhen::Always => true,
        PitWhen::Sector => held(g.sector_at),
        PitWhen::Lap => held(g.lap_at),
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum PitVar {
    Name,
    Num,
    Sponsor,
    Pos,
    ClassPos,
    Laps,
    Left,
    Gap,
    Int,
    GapBehind,
    GapAhead,
    Pen,
    Last,
    Best,
    Cur,
    Delta,
    Sess,
    Local,
    Track,
    Bike,
    Class,
    Setup,
    Speed,
    Rpm,
    Gear,
    Fuel,
    FuelPct,
    Air,
    Eng,
    Lap,
    RaceTime,
    SessionBest,
    Riders,
    SessionType,
    LapDiff,
    Server,
}

impl PitVar {
    pub const ALL: [Self; 36] = [
        Self::Name,
        Self::Num,
        Self::Sponsor,
        Self::Pos,
        Self::ClassPos,
        Self::Laps,
        Self::Left,
        Self::Gap,
        Self::Int,
        Self::GapBehind,
        Self::GapAhead,
        Self::Pen,
        Self::Last,
        Self::Best,
        Self::Cur,
        Self::Delta,
        Self::Sess,
        Self::Local,
        Self::Track,
        Self::Bike,
        Self::Class,
        Self::Setup,
        Self::Speed,
        Self::Rpm,
        Self::Gear,
        Self::Fuel,
        Self::FuelPct,
        Self::Air,
        Self::Eng,
        Self::Lap,
        Self::RaceTime,
        Self::SessionBest,
        Self::Riders,
        Self::SessionType,
        Self::LapDiff,
        Self::Server,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Num => "num",
            Self::Sponsor => "sponsor",
            Self::Pos => "pos",
            Self::ClassPos => "classpos",
            Self::Laps => "laps",
            Self::Left => "left",
            Self::Gap => "gap",
            Self::Int => "int",
            Self::GapBehind => "gapbehind",
            Self::GapAhead => "gapahead",
            Self::Pen => "pen",
            Self::Last => "last",
            Self::Best => "best",
            Self::Cur => "cur",
            Self::Delta => "delta",
            Self::Sess => "sess",
            Self::Local => "local",
            Self::Track => "track",
            Self::Bike => "bike",
            Self::Class => "class",
            Self::Setup => "setup",
            Self::Speed => "speed",
            Self::Rpm => "rpm",
            Self::Gear => "gear",
            Self::Fuel => "fuel",
            Self::FuelPct => "fuelpct",
            Self::Air => "air",
            Self::Eng => "eng",
            Self::Lap => "lap",
            Self::RaceTime => "race",
            Self::SessionBest => "sbest",
            Self::Riders => "riders",
            Self::SessionType => "stype",
            Self::LapDiff => "lapdiff",
            Self::Server => "server",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Num => "Bike number",
            Self::Sponsor => "Sponsor",
            Self::Pos => "Position",
            Self::ClassPos => "Class position",
            Self::Laps => "Lap count",
            Self::Left => "Laps remaining",
            Self::Gap => "Gap",
            Self::Int => "Interval",
            Self::GapBehind => "Gap behind",
            Self::GapAhead => "Gap ahead",
            Self::Pen => "Penalty",
            Self::Last => "Last lap",
            Self::Best => "Best lap",
            Self::Cur => "Current lap",
            Self::Delta => "Delta",
            Self::Sess => "Session time",
            Self::Local => "Local time",
            Self::Track => "Track name",
            Self::Bike => "Bike",
            Self::Class => "Class",
            Self::Setup => "Setup",
            Self::Speed => "Speed",
            Self::Rpm => "RPM",
            Self::Gear => "Gear",
            Self::Fuel => "Fuel",
            Self::FuelPct => "Fuel %",
            Self::Air => "Air temp",
            Self::Eng => "Engine temp",
            Self::Lap => "Lap",
            Self::RaceTime => "Race time",
            Self::SessionBest => "Session best",
            Self::Riders => "Riders",
            Self::SessionType => "Session type",
            Self::LapDiff => "Last lap diff",
            Self::Server => "Server",
        }
    }

    pub fn group(self) -> &'static str {
        match self {
            Self::Name | Self::Num | Self::Sponsor => "Identity",
            Self::Pos | Self::ClassPos | Self::Laps | Self::Left | Self::Gap | Self::Int
            | Self::GapBehind | Self::GapAhead | Self::Pen => "Race",
            Self::Last | Self::Best | Self::Cur | Self::Delta | Self::LapDiff => "Laps",
            Self::Sess | Self::Local | Self::Track | Self::Lap | Self::RaceTime
            | Self::SessionBest | Self::Riders | Self::SessionType | Self::Server => "Session",
            Self::Bike | Self::Class | Self::Setup | Self::Speed | Self::Rpm | Self::Gear
            | Self::Fuel | Self::FuelPct | Self::Air | Self::Eng => "Bike",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s.trim() {
            "name" => Self::Name,
            "num" | "number" => Self::Num,
            "sponsor" => Self::Sponsor,
            "pos" | "position" => Self::Pos,
            "classpos" | "class_pos" => Self::ClassPos,
            "lap" => Self::Lap,
            "laps" | "lapcount" => Self::Laps,
            "left" | "lapsleft" => Self::Left,
            "gap" => Self::Gap,
            "int" | "interval" => Self::Int,
            "gapbehind" | "gap_behind" => Self::GapBehind,
            "gapahead" | "gap_ahead" => Self::GapAhead,
            "pen" | "penalty" => Self::Pen,
            "last" => Self::Last,
            "best" => Self::Best,
            "cur" | "current" => Self::Cur,
            "delta" => Self::Delta,
            "sess" | "session" => Self::Sess,
            "local" | "localtime" => Self::Local,
            "track" => Self::Track,
            "bike" => Self::Bike,
            "class" => Self::Class,
            "setup" => Self::Setup,
            "speed" => Self::Speed,
            "rpm" => Self::Rpm,
            "gear" => Self::Gear,
            "fuel" => Self::Fuel,
            "fuelpct" | "fuel%" => Self::FuelPct,
            "air" => Self::Air,
            "eng" | "engine" => Self::Eng,
            "race" | "racetime" => Self::RaceTime,
            "sbest" | "sessionbest" => Self::SessionBest,
            "riders" | "count" => Self::Riders,
            "stype" | "sessiontype" => Self::SessionType,
            "lapdiff" | "lastlapdiff" => Self::LapDiff,
            "server" => Self::Server,
            _ => return None,
        })
    }

    pub fn idx(self) -> u8 {
        Self::ALL.iter().position(|v| *v == self).unwrap_or(0) as u8
    }

    pub fn from_idx(i: u8) -> Option<Self> {
        Self::ALL.get(i as usize).copied()
    }

    pub fn from_board(field: BoardField) -> Option<Self> {
        Some(match field {
            BoardField::None => return None,
            BoardField::Position => Self::Pos,
            BoardField::ClassPos => Self::ClassPos,
            BoardField::Session => Self::Sess,
            BoardField::RaceTime => Self::RaceTime,
            BoardField::Lap => Self::Lap,
            BoardField::LapsLeft => Self::Left,
            BoardField::Track => Self::Track,
            BoardField::Air => Self::Air,
            BoardField::Best => Self::Best,
            BoardField::SessionBest => Self::SessionBest,
            BoardField::LocalTime => Self::Local,
            BoardField::Riders => Self::Riders,
            BoardField::SessionType => Self::SessionType,
            BoardField::Fuel => Self::Fuel,
            BoardField::FuelPct => Self::FuelPct,
            BoardField::Setup => Self::Setup,
            BoardField::GapAhead => Self::GapAhead,
            BoardField::GapBehind => Self::GapBehind,
            BoardField::Delta => Self::Delta,
            BoardField::Last => Self::Last,
            BoardField::LapDiff => Self::LapDiff,
            BoardField::Current => Self::Cur,
            BoardField::Gap => Self::Gap,
            BoardField::Engine => Self::Eng,
            BoardField::Penalty => Self::Pen,
            BoardField::Server => Self::Server,
        })
    }
}

/// Same labels and order as Standings / Relative header and footer, then the other pit stats.
pub fn slot_menu() -> Vec<(PitVar, &'static str)> {
    let mut out: Vec<(PitVar, &'static str)> = BoardField::ALL
        .iter()
        .filter_map(|&field| PitVar::from_board(field).map(|var| (var, field.label())))
        .collect();
    for var in PitVar::ALL {
        if !out.iter().any(|(v, _)| *v == var) {
            out.push((var, var.label()));
        }
    }
    out
}

#[derive(Clone, Debug)]
pub struct PitPlace {
    pub var: PitVar,
    pub default: PitVar,
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub show: bool,
    pub label: String,
    pub color: Option<[u8; 3]>,
}

impl PitPlace {
    fn slot(default: PitVar, x: f32, y: f32, size: f32) -> Self {
        Self {
            var: default,
            default,
            x: x.clamp(0.04, 0.96),
            y: y.clamp(0.04, 0.96),
            size: size.clamp(10.0, 56.0),
            show: true,
            label: String::new(),
            color: None,
        }
    }

    pub fn row_label(&self) -> String {
        if !self.label.is_empty() {
            self.label.clone()
        } else {
            self.default.key().into()
        }
    }
}

pub fn factory_pack() -> (TableText, String, Vec<PitPlace>) {
    if let Some((_, text, _, places)) = read_pack_json(FACTORY_JSON) {
        if !places.is_empty() {
            return (text, String::new(), places);
        }
    }
    (TableText::White, String::new(), hardcoded_places())
}

/// Hero Stack slots from the shipped `board.json`.
pub fn factory_places() -> Vec<PitPlace> {
    factory_pack().2
}

fn hardcoded_places() -> Vec<PitPlace> {
    [
        (PitVar::Name, 0.50, 0.18, 12.0, "Slot 1", false),
        (PitVar::Pos, 0.38, 0.30, 14.0, "Slot 2", true),
        (PitVar::Laps, 0.62, 0.30, 14.0, "Slot 3", true),
        (PitVar::Last, 0.50, 0.42, 28.0, "Slot 4", true),
        (PitVar::Delta, 0.50, 0.56, 15.0, "Slot 5", true),
    ]
    .into_iter()
    .map(|(var, x, y, size, name, show)| {
        let mut p = PitPlace::slot(var, x, y, size);
        p.label = name.into();
        p.show = show;
        p
    })
    .collect()
}

/// Ink for a slot. A `board.json` color wins. No color is black.
pub fn place_ink(color: Option<[u8; 3]>) -> [u8; 3] {
    color.unwrap_or([16, 16, 18])
}

/// `color` on one slot of the current pack. Missing means black.
pub fn pack_slot_color(art: &str, slot: usize) -> Option<[u8; 3]> {
    pack_named_places(art).get(slot).and_then(|place| place.color)
}
pub fn pack_slot_names(art: &str) -> Vec<String> {
    pack_named_places(art)
        .iter()
        .map(|p| p.row_label())
        .collect()
}

/// F8 row title for a slot: `name` from the current pack JSON.
pub fn slot_row_name(art: &str, slot: usize, place: &PitPlace) -> String {
    pack_slot_names(art)
        .get(slot)
        .cloned()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| place.row_label())
}

/// Copy `name` from the pack JSON onto each settings row.
pub fn apply_json_names(art: &str, places: &mut [PitPlace]) {
    let named = pack_named_places(art);
    for (i, p) in places.iter_mut().enumerate() {
        if let Some(n) = named.get(i) {
            p.label = n.row_label();
        }
    }
}

fn pack_named_places(art: &str) -> Vec<PitPlace> {
    let factory = factory_places();
    let name = if art.trim().is_empty() {
        FACTORY_ART
    } else {
        art.trim()
    };
    let mut places = art_path(name)
        .and_then(|path| load_sidecar(&path))
        .map(|(_, _, p)| p)
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| factory.clone());
    if name == FACTORY_ART {
        for (i, p) in places.iter_mut().enumerate() {
            if !p.label.is_empty() {
                continue;
            }
            let from_factory = factory
                .get(i)
                .filter(|f| f.default == p.default)
                .or_else(|| factory.iter().find(|f| f.default == p.default));
            if let Some(f) = from_factory {
                p.label = f.label.clone();
            }
        }
    }
    places
}

/// Stats the current pack defined. F8 menus stay on this list, not the full catalog.
pub fn pack_defaults(places: &[PitPlace]) -> Vec<PitVar> {
    let mut vars: Vec<PitVar> = places.iter().map(|p| p.default).collect();
    vars.sort_by_key(|v| v.idx());
    vars.dedup();
    vars
}

pub fn set_slot_var(places: &mut [PitPlace], slot: usize, var: Option<PitVar>) {
    let Some(p) = places.get_mut(slot) else {
        return;
    };
    match var {
        Some(v) => {
            p.var = v;
            p.show = true;
        }
        None => p.show = false,
    }
}

/// Ini / packed form: `default:x,y,size,current` (optional `,label`).
/// Old `var:x,y,on,size` still loads.
pub fn encode_places(places: &[PitPlace]) -> String {
    places
        .iter()
        .map(|p| {
            let current = if p.show { p.var.key() } else { "none" };
            let mut s = format!(
                "{}:{:.3},{:.3},{:.0},{}",
                p.default.key(),
                p.x,
                p.y,
                p.size,
                current
            );
            if !p.label.is_empty() {
                s.push(',');
                s.push_str(&p.label.replace(',', " "));
            }
            s
        })
        .collect::<Vec<_>>()
        .join(";")
}

pub fn parse_places(s: &str) -> Vec<PitPlace> {
    let mut places = parse_place_tokens(s, false);
    normalize_places(&mut places);
    places
}

/// Drop the old 29-stat catalog dump. Settings rows follow the pack, not PitVar::ALL.
pub fn normalize_places(places: &mut Vec<PitPlace>) {
    if places.len() >= LEGACY_CATALOG || places.is_empty() {
        *places = factory_places();
        return;
    }
    places.retain(|p| p.show || (p.x - 0.5).abs() >= 0.03 || (p.y - 0.5).abs() >= 0.03);
    if places.is_empty() {
        *places = factory_places();
    }
}

fn parse_v1_vars(s: &str) -> Vec<PitPlace> {
    parse_place_tokens(s, true)
}

fn parse_place_tokens(s: &str, drop_v1_fillers: bool) -> Vec<PitPlace> {
    let mut places: Vec<PitPlace> = Vec::new();
    for token in s.split(';') {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        let Some((key, rest)) = token.split_once(':') else {
            continue;
        };
        let Some(default) = PitVar::parse(key) else {
            continue;
        };
        let mut parts = rest.split(',');
        let x: f32 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0.5);
        let y: f32 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0.5);
        let third = parts.next().unwrap_or("");
        let fourth = parts.next().unwrap_or("");
        let label = parts.collect::<Vec<_>>().join(",").trim().to_string();
        let (var, size, show) = if is_old_show_flag(third) {
            let show = third == "1" || third.eq_ignore_ascii_case("true");
            let size = fourth.parse().unwrap_or(16.0);
            (default, size, show)
        } else {
            let size = third.parse().unwrap_or(16.0);
            match PitVar::parse(fourth) {
                Some(v) => (v, size, true),
                None => (default, size, false),
            }
        };
        if drop_v1_fillers && !show && (x - 0.5).abs() < 0.03_f32 && (y - 0.5).abs() < 0.03_f32 {
            continue;
        }
        let mut place = PitPlace::slot(default, x, y, size);
        place.var = var;
        place.show = show;
        place.label = label;
        places.push(place);
    }
    places
}

fn is_old_show_flag(s: &str) -> bool {
    matches!(
        s.trim().to_ascii_lowercase().as_str(),
        "0" | "1" | "true" | "false"
    )
}

pub fn pitboards_dir() -> PathBuf {
    ini_path()
        .parent()
        .map(|p| p.join("Holeshot-HUD").join("pitboards"))
        .unwrap_or_else(|| PathBuf::from("pitboards"))
}

/// Glass hero-stack that shipped before the light coaching plate.
fn previous_glass_slot(var: PitVar) -> Option<(f32, f32, f32)> {
    match var {
        PitVar::Name => Some((0.50, 0.28, 16.0)),
        PitVar::Pos => Some((0.22, 0.46, 16.0)),
        PitVar::Laps => Some((0.78, 0.46, 16.0)),
        PitVar::Last => Some((0.50, 0.68, 40.0)),
        PitVar::Delta => Some((0.50, 0.86, 18.0)),
        _ => None,
    }
}

/// White text on the old five-slot glass plate. A moved slot or another PNG stays put.
pub fn previous_factory_slots(art: &str, text: TableText, places: &[PitPlace]) -> bool {
    let art = art.trim();
    if !art.is_empty() && art != FACTORY_ART {
        return false;
    }
    if text != TableText::White || places.len() != 5 {
        return false;
    }
    places.iter().all(|place| {
        let Some((x, y, size)) = previous_glass_slot(place.default) else {
            return false;
        };
        (place.x - x).abs() < 0.02 && (place.y - y).abs() < 0.02 && (place.size - size).abs() < 0.6
    })
}

/// A factory five that predates Slot 1–5. A moved slot or a different stat stays put.
pub fn stale_light_factory(places: &[PitPlace]) -> bool {
    const PACKS: [[(PitVar, f32, f32, f32); 5]; 2] = [
        [
            (PitVar::Name, 0.50, 0.26, 13.0),
            (PitVar::Pos, 0.36, 0.36, 13.0),
            (PitVar::Laps, 0.64, 0.36, 13.0),
            (PitVar::Last, 0.50, 0.50, 26.0),
            (PitVar::Delta, 0.50, 0.64, 14.0),
        ],
        [
            (PitVar::Name, 0.50, 0.28, 12.0),
            (PitVar::Pos, 0.38, 0.40, 14.0),
            (PitVar::Laps, 0.62, 0.40, 14.0),
            (PitVar::Last, 0.50, 0.52, 28.0),
            (PitVar::Delta, 0.50, 0.66, 15.0),
        ],
    ];
    if places.len() != 5 {
        return false;
    }
    PACKS.iter().any(|pack| {
        places.iter().zip(pack).all(|(place, (var, x, y, size))| {
            place.default == *var
                && place.show
                && place.var == *var
                && (place.x - x).abs() < 0.02
                && (place.y - y).abs() < 0.02
                && (place.size - size).abs() < 0.6
        })
    })
}

/// Factory plate whose five row titles are still a pre-Slot set, and whose slots
/// have not been moved. A renamed slot or a moved slot stays as saved.
pub fn stale_factory_row_names(art: &str, places: &[PitPlace]) -> bool {
    const TITLES: [[&str; 5]; 2] = [
        ["Name", "Position", "Laps", "Last", "Delta"],
        ["Top", "Position", "Lap", "Lap time", "Lap delta"],
    ];
    const PACKS: [[(PitVar, f32, f32, f32); 5]; 2] = [
        [
            (PitVar::Name, 0.50, 0.26, 13.0),
            (PitVar::Pos, 0.36, 0.36, 13.0),
            (PitVar::Laps, 0.64, 0.36, 13.0),
            (PitVar::Last, 0.50, 0.50, 26.0),
            (PitVar::Delta, 0.50, 0.64, 14.0),
        ],
        [
            (PitVar::Name, 0.50, 0.28, 12.0),
            (PitVar::Pos, 0.38, 0.40, 14.0),
            (PitVar::Laps, 0.62, 0.40, 14.0),
            (PitVar::Last, 0.50, 0.52, 28.0),
            (PitVar::Delta, 0.50, 0.66, 15.0),
        ],
    ];
    if !factory_plate(art) || places.len() != 5 {
        return false;
    }
    let titles = TITLES.iter().any(|titles| {
        places
            .iter()
            .zip(titles)
            .all(|(place, title)| place.label == *title)
    });
    let unmoved = PACKS.iter().any(|pack| {
        places.iter().zip(pack).all(|(place, (var, x, y, size))| {
            place.default == *var
                && (place.x - x).abs() < 0.02
                && (place.y - y).abs() < 0.02
                && (place.size - size).abs() < 0.6
        })
    });
    titles && unmoved
}

fn rename_factory_rows(places: &mut [PitPlace]) {
    for (index, place) in places.iter_mut().enumerate() {
        place.label = format!("Slot {}", index + 1);
    }
}

/// A factory five still sitting on the stripe band. Raises `y` by 0.10.
/// The chosen stat, `x`, and size stay. A moved slot stays.
pub fn lift_low_factory(places: &mut [PitPlace]) -> bool {
    const PACKS: [[(PitVar, f32, f32, f32); 5]; 2] = [
        [
            (PitVar::Name, 0.50, 0.26, 13.0),
            (PitVar::Pos, 0.36, 0.36, 13.0),
            (PitVar::Laps, 0.64, 0.36, 13.0),
            (PitVar::Last, 0.50, 0.50, 26.0),
            (PitVar::Delta, 0.50, 0.64, 14.0),
        ],
        [
            (PitVar::Name, 0.50, 0.28, 12.0),
            (PitVar::Pos, 0.38, 0.40, 14.0),
            (PitVar::Laps, 0.62, 0.40, 14.0),
            (PitVar::Last, 0.50, 0.52, 28.0),
            (PitVar::Delta, 0.50, 0.66, 15.0),
        ],
    ];
    if places.len() != 5 {
        return false;
    }
    let Some(pack) = PACKS.iter().find(|pack| {
        places.iter().zip(pack.iter()).all(|(place, (var, x, y, size))| {
            place.default == *var
                && (place.x - x).abs() < 0.02
                && (place.y - y).abs() < 0.02
                && (place.size - size).abs() < 0.6
        })
    }) else {
        return false;
    };
    for (place, (_, _, y, _)) in places.iter_mut().zip(pack.iter()) {
        place.y = (y - 0.10).clamp(0.04, 0.96);
    }
    true
}

pub fn ensure_factory_pack() {
    let dir = pitboards_dir();
    let _ = fs::create_dir_all(&dir);
    let _ = fs::write(dir.join(FACTORY_ART), FACTORY_PNG);
    let sample = dir.join(SAMPLE_ART);
    if !sample.exists() {
        let _ = fs::write(sample, SAMPLE_PNG);
    }
    let json = dir.join("board.json");
    match fs::read_to_string(&json) {
        Ok(body) => {
            let Some((art, text, _, mut places)) = read_pack_json(&body) else {
                return;
            };
            if previous_factory_slots(&art, text, &places) {
                let _ = fs::write(json, FACTORY_JSON);
            } else if factory_plate(&art) {
                let renamed = if stale_factory_row_names(&art, &places) {
                    rename_factory_rows(&mut places);
                    true
                } else {
                    false
                };
                let lifted = lift_low_factory(&mut places);
                if renamed || lifted {
                    let _ = fs::write(json, pack_json(&art, &places, text));
                }
            }
        }
        Err(_) => {
            let _ = fs::write(json, FACTORY_JSON);
        }
    }
}

pub fn art_bytes(name: &str) -> Option<Vec<u8>> {
    let name = name.trim();
    if let Some(path) = art_path(name) {
        if let Ok(bytes) = fs::read(path) {
            return Some(bytes);
        }
    }
    if name.is_empty() || name == FACTORY_ART {
        return Some(FACTORY_PNG.to_vec());
    }
    if name == SAMPLE_ART {
        return Some(SAMPLE_PNG.to_vec());
    }
    None
}

pub fn art_path(name: &str) -> Option<PathBuf> {
    let name = name.trim();
    if name.is_empty() || name.contains("..") || name.contains('/') || name.contains('\\') {
        return None;
    }
    let lower = name.to_ascii_lowercase();
    if !lower.ends_with(".png") {
        return None;
    }
    Some(pitboards_dir().join(name))
}

pub fn import_png(src: &Path) -> Result<String, String> {
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if ext != "png" {
        return Err("Pick a PNG picture.".into());
    }
    let bytes = fs::read(src).map_err(|_| "Could not read that picture.".to_string())?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err("Picture is too large (8 MB max).".into());
    }
    let dir = pitboards_dir();
    fs::create_dir_all(&dir).map_err(|_| "Could not create the designs folder.".to_string())?;
    let stem = src
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("plate")
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>();
    let stem = if stem.is_empty() { "plate".into() } else { stem };
    let name = format!("{stem}.png");
    fs::write(dir.join(&name), bytes).map_err(|_| "Could not save that picture.".to_string())?;
    Ok(name)
}

/// Slot positions from `board.json` sitting next to a plate PNG.
pub fn load_sidecar(png: &Path) -> Option<(TableText, String, Vec<PitPlace>)> {
    let json = png.parent()?.join("board.json");
    let s = fs::read_to_string(json).ok()?;
    let (_, text, sponsor, places) = read_pack_json(&s)?;
    if places.is_empty() {
        return None;
    }
    Some((text, sponsor, places))
}

pub fn pack_json(art: &str, places: &[PitPlace], text: TableText) -> String {
    let mut slots = String::from("[\n");
    for (i, p) in places.iter().enumerate() {
        if i > 0 {
            slots.push_str(",\n");
        }
        slots.push_str("    { ");
        if !p.label.is_empty() {
            slots.push_str(&format!("\"name\": {}, ", json_str(&p.label)));
        }
        slots.push_str(&format!(
            "\"default\": {}, \"x\": {:.2}, \"y\": {:.2}, \"size\": {:.0}",
            json_str(p.default.key()),
            p.x,
            p.y,
            p.size
        ));
        if !p.show || p.var != p.default {
            let current = if p.show { p.var.key() } else { "none" };
            slots.push_str(&format!(", \"current\": {}", json_str(current)));
        }
        slots.push_str(" }");
    }
    slots.push_str("\n  ]");
    format!(
        "{{\n  \"version\": 2,\n  \"background\": {},\n  \"text\": {},\n  \"slots\": {}\n}}\n",
        json_str(art),
        json_str(text.key()),
        slots
    )
}

pub fn write_pack(art: &str, places: &[PitPlace], text: TableText) -> Result<(), String> {
    ensure_factory_pack();
    let dir = pitboards_dir();
    fs::create_dir_all(&dir).map_err(|_| "Could not create the designs folder.".to_string())?;
    fs::write(dir.join("board.json"), pack_json(art, places, text))
        .map_err(|_| "Could not write board.json.".to_string())
}

pub fn read_pack_json(s: &str) -> Option<(String, TableText, String, Vec<PitPlace>)> {
    let art = json_field(s, "background").unwrap_or_default();
    let text = TableText::parse(&json_field(s, "text").unwrap_or_default());
    let sponsor = json_field(s, "sponsor").unwrap_or_default();
    if let Some(places) = parse_slots_array(s) {
        if !places.is_empty() {
            return Some((art, text, sponsor, places));
        }
    }
    let vars = json_field(s, "vars")?;
    let places = parse_v1_vars(&vars);
    if places.is_empty() {
        return None;
    }
    Some((art, text, sponsor, places))
}

fn parse_slots_array(s: &str) -> Option<Vec<PitPlace>> {
    let objs = json_array_objects(s, "slots")?;
    let mut places = Vec::new();
    for obj in objs {
        let Some(default) = json_field(&obj, "default").and_then(|k| PitVar::parse(&k)) else {
            continue;
        };
        let x = json_num(&obj, "x").unwrap_or(0.5);
        let y = json_num(&obj, "y").unwrap_or(0.5);
        let size = json_num(&obj, "size").unwrap_or(16.0);
        let mut place = PitPlace::slot(default, x, y, size);
        if let Some(name) = json_field(&obj, "name").or_else(|| json_field(&obj, "label")) {
            place.label = name;
        }
        if let Some(color) = json_field(&obj, "color") {
            place.color = crate::config::parse_primary_color(&color);
        }
        if let Some(cur) = json_field(&obj, "current") {
            match PitVar::parse(&cur) {
                Some(v) => {
                    place.var = v;
                    place.show = true;
                }
                None => place.show = false,
            }
        }
        places.push(place);
    }
    Some(places)
}

fn json_array_objects(s: &str, key: &str) -> Option<Vec<String>> {
    let needle = format!("\"{key}\"");
    let after = s.get(s.find(&needle)? + needle.len()..)?;
    let after = after.trim_start().strip_prefix(':')?.trim_start();
    let after = after.strip_prefix('[')?;
    let mut objs = Vec::new();
    let mut depth = 0i32;
    let mut start = None;
    let mut in_str = false;
    let mut esc = false;
    for (i, c) in after.char_indices() {
        if in_str {
            if esc {
                esc = false;
            } else if c == '\\' {
                esc = true;
            } else if c == '"' {
                in_str = false;
            }
            continue;
        }
        match c {
            '"' => in_str = true,
            '{' => {
                if depth == 0 {
                    start = Some(i);
                }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    if let Some(st) = start.take() {
                        objs.push(after[st..=i].to_string());
                    }
                }
            }
            ']' if depth == 0 => break,
            _ => {}
        }
    }
    Some(objs)
}

fn json_num(s: &str, key: &str) -> Option<f32> {
    let needle = format!("\"{key}\"");
    let rest = s.get(s.find(&needle)? + needle.len()..)?;
    let rest = rest.trim_start().strip_prefix(':')?.trim_start();
    let num: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();
    num.parse().ok()
}

fn json_str(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn json_field(s: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let rest = s.get(s.find(&needle)? + needle.len()..)?;
    let rest = rest.trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    let rest = rest.strip_prefix('"')?;
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            out.push(chars.next()?);
        } else if c == '"' {
            break;
        } else {
            out.push(c);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_round_trip() {
        for var in PitVar::ALL {
            assert_eq!(PitVar::parse(var.key()), Some(var), "{}", var.key());
        }
    }

    #[test]
    fn slot_menu_matches_header_and_footer() {
        let menu = slot_menu();
        let board: Vec<&str> = crate::config::BoardField::ALL
            .iter()
            .filter(|f| **f != crate::config::BoardField::None)
            .map(|f| f.label())
            .collect();
        let head: Vec<&str> = menu.iter().take(board.len()).map(|(_, l)| *l).collect();
        assert_eq!(head, board);
        assert!(menu.iter().any(|(v, _)| *v == PitVar::Last));
        assert!(menu.iter().any(|(v, _)| *v == PitVar::Name));
        assert!(menu.iter().any(|(v, _)| *v == PitVar::Delta));
    }

    #[test]
    fn old_catalog_ini_becomes_factory_slots() {
        let places: Vec<PitPlace> = PitVar::ALL
            .iter()
            .map(|&var| PitPlace::slot(var, 0.50, 0.50, 16.0))
            .collect();
        assert_eq!(places.len(), PitVar::ALL.len());
        let again = parse_places(&encode_places(&places));
        assert_eq!(again.len(), factory_places().len());
        assert_eq!(again.len(), 5);
    }

    #[test]
    fn factory_has_heroes() {
        let places = factory_places();
        assert_eq!(places.len(), 5);
        let name = places.iter().find(|p| p.default == PitVar::Name).unwrap();
        assert!(!name.show);
        assert!(places.iter().filter(|p| p.default != PitVar::Name).all(|p| p.show));
        assert!(!places.iter().any(|p| p.default == PitVar::Num));
        assert!(!places.iter().any(|p| p.default == PitVar::Speed));
        let last = places.iter().find(|p| p.default == PitVar::Last).unwrap();
        assert!(last.size > name.size);
        assert!(places.iter().any(|p| p.default == PitVar::Delta && p.show));
        assert_eq!(name.row_label(), "Slot 1");
        assert_eq!(
            places
                .iter()
                .find(|p| p.default == PitVar::Laps)
                .unwrap()
                .row_label(),
            "Slot 3"
        );
        assert_eq!(last.row_label(), "Slot 4");
        assert_eq!(
            places
                .iter()
                .find(|p| p.default == PitVar::Delta)
                .unwrap()
                .row_label(),
            "Slot 5"
        );
        assert_eq!(
            places
                .iter()
                .find(|p| p.default == PitVar::Pos)
                .unwrap()
                .row_label(),
            "Slot 2"
        );
    }

    #[test]
    fn slot_can_show_a_different_var() {
        let mut places = factory_places();
        let name = places.iter().position(|p| p.default == PitVar::Name).unwrap();
        let (x, y, size) = (places[name].x, places[name].y, places[name].size);
        set_slot_var(&mut places, name, Some(PitVar::Best));
        assert_eq!(places[name].var, PitVar::Best);
        assert_eq!(places[name].default, PitVar::Name);
        assert_eq!(places[name].row_label(), "Slot 1");
        assert!((places[name].x - x).abs() < f32::EPSILON);
        assert!((places[name].y - y).abs() < f32::EPSILON);
        assert!((places[name].size - size).abs() < f32::EPSILON);
        set_slot_var(&mut places, name, None);
        assert!(!places[name].show);
        assert_eq!(places[name].default, PitVar::Name);
    }

    #[test]
    fn v2_slots_round_trip() {
        let places = factory_places();
        let json = pack_json("holeshot.png", &places, TableText::White);
        assert!(json.contains("\"version\": 2"));
        assert!(json.contains("\"slots\""));
        assert!(!json.contains("\"sponsor\""));
        let (art, text, sponsor, again) = read_pack_json(&json).unwrap();
        assert_eq!(art, "holeshot.png");
        assert_eq!(text, TableText::White);
        assert!(sponsor.is_empty());
        assert_eq!(again.len(), 5);
        let last = again.iter().find(|p| p.default == PitVar::Last).unwrap();
        assert!(last.show && last.var == PitVar::Last);
        assert!((last.y - 0.42).abs() < 0.002);
        assert_eq!(last.row_label(), "Slot 4");
        assert!(json.contains("\"name\": \"Slot 4\""));
        let top = again.iter().find(|p| p.default == PitVar::Name).unwrap();
        assert!(!top.show);
    }

    #[test]
    fn v1_vars_keep_only_shown_slots() {
        let json = format!(
            "{{\n  \"version\": 1,\n  \"background\": \"x.png\",\n  \"text\": \"white\",\n  \"sponsor\": \"\",\n  \"vars\": {}\n}}\n",
            json_str("name:0.50,0.28,1,16;num:0.50,0.50,0,16;speed:0.50,0.50,0,16;last:0.50,0.68,1,40"),
        );
        let (_, _, _, places) = read_pack_json(&json).unwrap();
        assert_eq!(places.len(), 2);
        assert!(places.iter().any(|p| p.default == PitVar::Name && p.show));
        assert!(places.iter().any(|p| p.default == PitVar::Last && p.show));
        assert!(!places.iter().any(|p| p.default == PitVar::Num));
    }

    #[test]
    fn json_name_is_the_settings_row() {
        let json = r#"{
  "version": 2,
  "background": "x.png",
  "text": "white",
  "slots": [
    { "name": "Rider", "default": "name", "x": 0.50, "y": 0.28, "size": 16 },
    { "name": "Gate", "default": "pos", "x": 0.22, "y": 0.46, "size": 16 }
  ]
}"#;
        let (_, _, _, mut places) = read_pack_json(json).unwrap();
        assert_eq!(places[0].row_label(), "Rider");
        assert_eq!(places[1].row_label(), "Gate");
        places[0].var = PitVar::Best;
        assert_eq!(places[0].row_label(), "Rider");
    }

    #[test]
    fn slot_label_stays_on_the_default() {
        let mut p = PitPlace::slot(PitVar::Name, 0.50, 0.28, 16.0);
        p.label = "Hole 1".into();
        assert_eq!(p.row_label(), "Hole 1");
        p.var = PitVar::Best;
        assert_eq!(p.row_label(), "Hole 1");
        p.label.clear();
        assert_eq!(p.row_label(), "name");
    }

    #[test]
    fn parse_keeps_two_slots_on_the_same_var() {
        let places = parse_places("name:0.50,0.28,1,16;name:0.18,0.46,1,16");
        let names: Vec<_> = places.iter().filter(|p| p.var == PitVar::Name && p.show).collect();
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn encode_parse_keeps_move() {
        let mut places = factory_places();
        if let Some(p) = places.iter_mut().find(|p| p.default == PitVar::Last) {
            p.x = 0.33;
            p.y = 0.71;
        }
        let again = parse_places(&encode_places(&places));
        let last = again.iter().find(|p| p.default == PitVar::Last).unwrap();
        assert!((last.x - 0.33).abs() < 0.002);
        assert!((last.y - 0.71).abs() < 0.002);
    }

    #[test]
    fn sidecar_locks_places() {
        let dir = std::env::temp_dir().join("mxbo-pit-sidecar-test");
        let _ = fs::create_dir_all(&dir);
        let vars = "last:0.220,0.800,1,40";
        let json = format!(
            "{{\n  \"version\": 1,\n  \"background\": \"x.png\",\n  \"text\": \"black\",\n  \"sponsor\": \"ACME\",\n  \"vars\": {}\n}}\n",
            json_str(vars),
        );
        fs::write(dir.join("board.json"), json).unwrap();
        let (text, sponsor, places) = load_sidecar(&dir.join("x.png")).unwrap();
        assert_eq!(text, TableText::Black);
        let _ = sponsor;
        let last = places.iter().find(|p| p.var == PitVar::Last).unwrap();
        assert!(last.show);
        assert!((last.x - 0.22).abs() < 0.002);
        assert!((last.y - 0.80).abs() < 0.002);
    }

    #[test]
    fn write_factory_pack_files() {
        if std::env::var("WRITE_PIT_FACTORY").is_err() {
            return;
        }
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
        fs::create_dir_all(&dir).unwrap();
        let places = factory_places();
        let _ = crate::render::factory_plate_png();
        fs::write(
            dir.join("board.json"),
            pack_json("holeshot.png", &places, factory_pack().0),
        )
        .unwrap();
    }

    #[test]
    fn rejects_path_escape() {
        assert!(art_path("../secret.png").is_none());
        assert!(art_path("ok.png").is_some());
        assert!(art_path("ok.jpg").is_none());
    }

    #[test]
    fn factory_pack_is_png_and_json() {
        let (text, sponsor, places) = factory_pack();
        assert_eq!(text, TableText::Black);
        assert!(sponsor.is_empty());
        assert!(places.iter().any(|p| p.var == PitVar::Last && p.show));
        assert!(!places.iter().any(|p| p.default == PitVar::Sponsor));
        assert!(!FACTORY_PNG.is_empty());
        assert!(FACTORY_JSON.contains("holeshot.png"));
        assert!(FACTORY_JSON.contains("\"slots\""));
        assert!(FACTORY_JSON.contains("\"name\""));
        assert!(FACTORY_JSON.contains("\"text\": \"black\""));
        assert!(!FACTORY_JSON.contains("\"sponsor\""));
        assert_eq!(places.len(), 5);
        assert_eq!(pack_defaults(&places).len(), 5);
        assert!(art_bytes(FACTORY_ART).is_some());
        assert!(art_bytes("").is_some());
    }

    #[test]
    fn previous_glass_pack_upgrades_to_the_light_plate() {
        let old = [
            (PitVar::Name, 0.50, 0.28, 16.0),
            (PitVar::Pos, 0.22, 0.46, 16.0),
            (PitVar::Laps, 0.78, 0.46, 16.0),
            (PitVar::Last, 0.50, 0.68, 40.0),
            (PitVar::Delta, 0.50, 0.86, 18.0),
        ]
        .map(|(var, x, y, size)| PitPlace::slot(var, x, y, size));
        assert!(previous_factory_slots("holeshot.png", TableText::White, &old));
        assert!(previous_factory_slots("", TableText::White, &old));
        assert!(!previous_factory_slots("custom.png", TableText::White, &old));
        let (text, _, now) = factory_pack();
        assert_eq!(text, TableText::Black);
        assert!(!previous_factory_slots(FACTORY_ART, text, &now));
        let last = now.iter().find(|p| p.default == PitVar::Last).unwrap();
        assert!((last.y - 0.42).abs() < 0.002);
    }

    #[test]
    fn earlier_light_pack_is_stale() {
        let places = [
            (PitVar::Name, 0.50, 0.26, 13.0),
            (PitVar::Pos, 0.36, 0.36, 13.0),
            (PitVar::Laps, 0.64, 0.36, 13.0),
            (PitVar::Last, 0.50, 0.50, 26.0),
            (PitVar::Delta, 0.50, 0.64, 14.0),
        ]
        .map(|(var, x, y, size)| PitPlace::slot(var, x, y, size));
        assert!(stale_light_factory(&places));
        let mut moved = places;
        moved[1].x = 0.10;
        assert!(!stale_light_factory(&moved));
    }

    #[test]
    fn named_light_pack_is_stale() {
        let places = [
            (PitVar::Name, 0.50, 0.28, 12.0),
            (PitVar::Pos, 0.38, 0.40, 14.0),
            (PitVar::Laps, 0.62, 0.40, 14.0),
            (PitVar::Last, 0.50, 0.52, 28.0),
            (PitVar::Delta, 0.50, 0.66, 15.0),
        ]
        .map(|(var, x, y, size)| PitPlace::slot(var, x, y, size));
        assert!(stale_light_factory(&places));
        let mut moved = places;
        moved[3].y = 0.80;
        assert!(!stale_light_factory(&moved));
    }

    #[test]
    fn low_factory_five_lifts_and_keeps_fuel() {
        let mut places = [
            (PitVar::Name, 0.50, 0.28, 12.0),
            (PitVar::Pos, 0.38, 0.40, 14.0),
            (PitVar::Laps, 0.62, 0.40, 14.0),
            (PitVar::Last, 0.50, 0.52, 28.0),
            (PitVar::Delta, 0.50, 0.66, 15.0),
        ]
        .map(|(var, x, y, size)| PitPlace::slot(var, x, y, size));
        places[3].var = PitVar::FuelPct;
        assert!(lift_low_factory(&mut places));
        let last = places.iter().find(|p| p.default == PitVar::Last).unwrap();
        assert_eq!(last.var, PitVar::FuelPct);
        assert!((last.y - 0.42).abs() < 0.002);
        assert!((last.x - 0.50).abs() < 0.002);
        let delta = places.iter().find(|p| p.default == PitVar::Delta).unwrap();
        assert!((delta.y - 0.56).abs() < 0.002);
        assert!(!lift_low_factory(&mut places));
    }

    #[test]
    fn moved_slot_is_not_lifted() {
        let mut places = [
            (PitVar::Name, 0.50, 0.28, 12.0),
            (PitVar::Pos, 0.38, 0.40, 14.0),
            (PitVar::Laps, 0.62, 0.40, 14.0),
            (PitVar::Last, 0.50, 0.52, 28.0),
            (PitVar::Delta, 0.50, 0.66, 15.0),
        ]
        .map(|(var, x, y, size)| PitPlace::slot(var, x, y, size));
        places[4].y = 0.80;
        assert!(!lift_low_factory(&mut places));
        assert!((places[4].y - 0.80).abs() < 0.002);
        assert!((places[3].y - 0.52).abs() < 0.002);
    }

    #[test]
    fn slot_color_or_black() {
        let json = r##"{
  "version": 2,
  "background": "holeshot.png",
  "text": "black",
  "slots": [
    { "name": "Slot 2", "default": "pos", "color": "#FF0000", "x": 0.38, "y": 0.30, "size": 14 },
    { "name": "Slot 3", "default": "laps", "x": 0.62, "y": 0.30, "size": 14 }
  ]
}"##;
        let (_, _, _, places) = read_pack_json(json).unwrap();
        assert_eq!(place_ink(places[0].color), [255, 0, 0]);
        assert_eq!(place_ink(places[1].color), [16, 16, 18]);
    }

    #[test]
    fn old_factory_row_titles_rename_on_launch() {
        let dir = std::env::temp_dir().join(format!("mxbo-pit-rename-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let ini = dir.join("Holeshot-HUD.ini");
        std::env::set_var("MXBO_TEST_INI", &ini);
        let boards = dir.join("Holeshot-HUD").join("pitboards");
        fs::create_dir_all(&boards).unwrap();
        let json = boards.join("board.json");

        let mut places = [
            (PitVar::Name, 0.50, 0.26, 13.0, "Name"),
            (PitVar::Pos, 0.36, 0.36, 13.0, "Position"),
            (PitVar::Laps, 0.64, 0.36, 13.0, "Laps"),
            (PitVar::Last, 0.50, 0.50, 26.0, "Last"),
            (PitVar::Delta, 0.50, 0.64, 14.0, "Delta"),
        ]
        .map(|(var, x, y, size, name)| {
            let mut place = PitPlace::slot(var, x, y, size);
            place.label = name.into();
            place
        });
        places[3].var = PitVar::FuelPct;
        fs::write(&json, pack_json(FACTORY_ART, &places, TableText::Black)).unwrap();
        ensure_factory_pack();
        let (art, text, _, renamed) = read_pack_json(&fs::read_to_string(&json).unwrap()).unwrap();

        places[3].var = PitVar::Last;
        fs::write(&json, pack_json("custom.png", &places, TableText::Black)).unwrap();
        ensure_factory_pack();
        let (_, _, _, custom) = read_pack_json(&fs::read_to_string(&json).unwrap()).unwrap();

        places[3].y = 0.80;
        fs::write(&json, pack_json(FACTORY_ART, &places, TableText::Black)).unwrap();
        ensure_factory_pack();
        let (_, _, _, moved) = read_pack_json(&fs::read_to_string(&json).unwrap()).unwrap();

        std::env::remove_var("MXBO_TEST_INI");
        let _ = fs::remove_dir_all(&dir);

        assert_eq!(art, FACTORY_ART);
        assert_eq!(text, TableText::Black);
        assert_eq!(
            renamed.iter().map(|p| p.row_label()).collect::<Vec<_>>(),
            ["Slot 1", "Slot 2", "Slot 3", "Slot 4", "Slot 5"]
        );
        let last = renamed.iter().find(|p| p.default == PitVar::Last).unwrap();
        assert_eq!(last.var, PitVar::FuelPct);
        assert!((last.x - 0.50).abs() < 0.002);
        assert!((last.y - 0.50).abs() < 0.002);
        assert!((last.size - 26.0).abs() < 0.6);
        assert_eq!(custom[0].row_label(), "Name");
        assert_eq!(moved[0].row_label(), "Name");
        assert!((moved[3].y - 0.80).abs() < 0.002);
    }

    #[test]
    fn factory_plate_recolors_yellow_and_keeps_white() {
        let red = [255, 0, 0];
        let yellow = recolor_plate_pixel([245, 190, 7, 255], red, PLATE_NAVY);
        assert_eq!(&yellow[0..3], &red);
        assert_eq!(yellow[3], 255);
        let white = recolor_plate_pixel([255, 255, 255, 255], red, [0, 80, 255]);
        assert_eq!(white, [255, 255, 255, 255]);
        let clear = recolor_plate_pixel([0, 0, 0, 0], red, [0, 80, 255]);
        assert_eq!(clear, [0, 0, 0, 0]);
        let navy = recolor_plate_pixel([PLATE_NAVY[0], PLATE_NAVY[1], PLATE_NAVY[2], 255], red, PLATE_NAVY);
        assert_eq!(&navy[0..3], &PLATE_NAVY);
        let same = recolor_plate_pixel([245, 190, 7, 255], PLATE_YELLOW, PLATE_NAVY);
        assert_eq!(same, [245, 190, 7, 255]);
        let accent = [255, 148, 48];
        let main = recolor_plate_pixel([245, 190, 7, 255], accent, [0, 0, 0]);
        assert_eq!(&main[0..3], &accent);
        let ink = recolor_plate_pixel(
            [PLATE_NAVY[0], PLATE_NAVY[1], PLATE_NAVY[2], 255],
            accent,
            [0, 0, 0],
        );
        assert_eq!(&ink[0..3], &[0, 0, 0]);
        assert!(factory_plate("holeshot.png"));
        assert!(factory_plate(""));
        assert!(!factory_plate("sample.png"));
    }

    #[test]
    fn when_keys_round_trip() {
        for when in [PitWhen::Always, PitWhen::Sector, PitWhen::Lap] {
            assert_eq!(PitWhen::parse(when.key()), when);
        }
        assert_eq!(PitWhen::parse(""), PitWhen::Always);
    }

    #[test]
    fn sector_end_is_a_new_split_or_the_line() {
        assert!(sector_edge(true, [0, 0, 0], true, [1_200, 0, 0]));
        assert!(sector_edge(true, [1_200, 0, 0], false, [1_200, 0, 0]));
        assert!(!sector_edge(true, [1_200, 0, 0], true, [1_200, 0, 0]));
    }

    #[test]
    fn lap_done_is_last_lap_or_the_line() {
        assert!(lap_done(0, 95_000, false, true));
        assert!(lap_done(90_000, 95_000, true, true));
        assert!(lap_done(95_000, 95_000, true, false));
        assert!(!lap_done(95_000, 95_000, true, true));
        assert!(!lap_done(0, 0, false, true));
    }

    #[test]
    fn sector_mode_holds_after_split() {
        *gate() = Gate::new();
        set_preview(false);
        let mut s = Snapshot::default();
        s.has_telemetry = 1;
        s.current_lap_ms = 5_000;
        tick(&s);
        s.sector_cur[0] = 12_000;
        tick(&s);
        assert!(drawing(true, PitWhen::Sector));
        assert!(!drawing(true, PitWhen::Lap));
        assert!(drawing(true, PitWhen::Always));
        assert!(!drawing(false, PitWhen::Always));
    }

    #[test]
    fn sector_flash_snaps_that_split() {
        *gate() = Gate::new();
        set_preview(false);
        let mut s = Snapshot::default();
        s.has_telemetry = 1;
        s.current_lap_ms = 5_000;
        tick(&s);
        s.sector_cur[0] = 22_140;
        tick(&s);
        let snap = time_snap(PitWhen::Sector).expect("sector snap");
        assert_eq!(snap.time_ms, 22_140);
        assert!(time_snap(PitWhen::Always).is_none());
    }

    #[test]
    fn lap_flash_snaps_last_lap() {
        *gate() = Gate::new();
        set_preview(false);
        let mut s = Snapshot::default();
        s.has_telemetry = 1;
        s.current_lap_ms = 5_000;
        tick(&s);
        s.last_lap_ms = 95_000;
        tick(&s);
        let snap = time_snap(PitWhen::Lap).expect("lap snap");
        assert_eq!(snap.time_ms, 95_000);
        assert!(time_snap(PitWhen::Always).is_none());
    }

    #[test]
    fn lap_mode_holds_when_the_lap_completes() {
        *gate() = Gate::new();
        set_preview(false);
        let mut s = Snapshot::default();
        s.has_telemetry = 1;
        s.current_lap_ms = 5_000;
        tick(&s);
        assert!(!drawing(true, PitWhen::Lap));
        s.last_lap_ms = 88_000;
        tick(&s);
        assert!(drawing(true, PitWhen::Lap));
        assert!(!drawing(true, PitWhen::Sector));
    }
}
