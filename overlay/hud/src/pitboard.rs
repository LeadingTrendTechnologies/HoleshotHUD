use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::config::{ini_path, BoardField, TableText};

/// Old ini dump of every catalog stat. Row count follows the pack, not this list.
pub const LEGACY_CATALOG: usize = 29;
use crate::shm::Snapshot;

pub const FACTORY_ART: &str = "holeshot.png";
const FACTORY_PNG: &[u8] = include_bytes!("../assets/holeshot.png");
const FACTORY_JSON: &str = include_str!("../assets/board.json");

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
}

impl PitVar {
    pub const ALL: [Self; 34] = [
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
        }
    }

    pub fn group(self) -> &'static str {
        match self {
            Self::Name | Self::Num | Self::Sponsor => "Identity",
            Self::Pos | Self::ClassPos | Self::Laps | Self::Left | Self::Gap | Self::Int
            | Self::GapBehind | Self::GapAhead | Self::Pen => "Race",
            Self::Last | Self::Best | Self::Cur | Self::Delta => "Laps",
            Self::Sess | Self::Local | Self::Track | Self::Lap | Self::RaceTime
            | Self::SessionBest | Self::Riders | Self::SessionType => "Session",
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
        (PitVar::Name, 0.50, 0.28, 16.0, "Name"),
        (PitVar::Pos, 0.22, 0.46, 16.0, "Position"),
        (PitVar::Laps, 0.78, 0.46, 16.0, "Laps"),
        (PitVar::Last, 0.50, 0.68, 40.0, "Last"),
        (PitVar::Delta, 0.50, 0.86, 18.0, "Delta"),
    ]
    .into_iter()
    .map(|(var, x, y, size, name)| {
        let mut p = PitPlace::slot(var, x, y, size);
        p.label = name.into();
        p
    })
    .collect()
}

/// F8 row titles from the current pack JSON (`name` on each slot).
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

pub fn ensure_factory_pack() {
    let dir = pitboards_dir();
    let _ = fs::create_dir_all(&dir);
    let _ = fs::write(dir.join(FACTORY_ART), FACTORY_PNG);
    let json = dir.join("board.json");
    if !json.exists() {
        let _ = fs::write(json, FACTORY_JSON);
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
        if !p.label.is_empty() {
            slots.push_str(&format!(", \"name\": {}", json_str(&p.label)));
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
        assert!(places.iter().all(|p| p.show));
        assert!(!places.iter().any(|p| p.default == PitVar::Num));
        assert!(!places.iter().any(|p| p.default == PitVar::Speed));
        let last = places.iter().find(|p| p.default == PitVar::Last).unwrap();
        let name = places.iter().find(|p| p.default == PitVar::Name).unwrap();
        assert!(last.size > name.size);
        assert!(places.iter().any(|p| p.default == PitVar::Delta && p.show));
        assert_eq!(name.row_label(), "Name");
        assert_eq!(
            places
                .iter()
                .find(|p| p.default == PitVar::Pos)
                .unwrap()
                .row_label(),
            "Position"
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
        assert_eq!(places[name].row_label(), "Name");
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
        assert!((last.y - 0.68).abs() < 0.002);
        assert_eq!(last.row_label(), "Last");
        assert!(json.contains("\"name\": \"Last\""));
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
        fs::write(dir.join("holeshot.png"), crate::render::factory_plate_png()).unwrap();
        let places = factory_places();
        fs::write(
            dir.join("board.json"),
            pack_json("holeshot.png", &places, TableText::White),
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
        assert_eq!(text, TableText::White);
        assert!(sponsor.is_empty());
        assert!(places.iter().any(|p| p.var == PitVar::Last && p.show));
        assert!(!places.iter().any(|p| p.default == PitVar::Sponsor));
        assert!(!FACTORY_PNG.is_empty());
        assert!(FACTORY_JSON.contains("holeshot.png"));
        assert!(FACTORY_JSON.contains("\"slots\""));
        assert!(FACTORY_JSON.contains("\"name\""));
        assert!(!FACTORY_JSON.contains("\"sponsor\""));
        assert_eq!(places.len(), 5);
        assert_eq!(pack_defaults(&places).len(), 5);
        assert!(art_bytes(FACTORY_ART).is_some());
        assert!(art_bytes("").is_some());
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
