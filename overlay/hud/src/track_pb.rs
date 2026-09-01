//! Per-track personal bests for Delta Bar and Sectors.
//!
//! One JSON file per `track_name` under AppData. Each file holds a tape per
//! displacement class (`250`, `450`, … in `bikes`). Load the current track only.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

pub const BINS: usize = 256;

/// Rewrite `used` at most this often when you visit a track that already has a file.
const TOUCH_SECS: i64 = 60 * 60;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrackPb {
    pub lap_ms: i32,
    pub bins: [i32; BINS],
    pub sectors: [i32; 3],
    /// S1 / S2 split line as thousandths of a lap (0 = unknown).
    pub split_milli: [i32; 2],
    /// Unix seconds when this file was last used (ridden or written). 0 = unknown.
    pub used: i64,
}

impl TrackPb {
    pub fn empty() -> Self {
        Self {
            lap_ms: 0,
            bins: [0; BINS],
            sectors: [0; 3],
            split_milli: [0; 2],
            used: 0,
        }
    }

    pub fn tape_filled(&self) -> usize {
        self.bins.iter().filter(|&&b| b > 0).count()
    }

    pub fn has_tape(&self) -> bool {
        self.lap_ms > 0 && self.tape_filled() > 0
    }
}

/// On-disk file: split lines are per track; tapes and sector times are per class.
struct TrackFile {
    bikes: BTreeMap<String, TrackPb>,
    split_milli: [i32; 2],
    used: i64,
}

impl TrackFile {
    const fn empty() -> Self {
        Self {
            bikes: BTreeMap::new(),
            split_milli: [0; 2],
            used: 0,
        }
    }

    fn has_named(&self) -> bool {
        self.bikes.keys().any(|k| !k.is_empty())
    }

    fn select(&self, bike: &str) -> TrackPb {
        let class = bike_class(bike);
        let mut pb = self
            .bikes
            .get(&class)
            .cloned()
            .or_else(|| {
                self.bikes.iter().find_map(|(k, pb)| {
                    if !k.is_empty() && bike_class(k) == class {
                        Some(pb.clone())
                    } else {
                        None
                    }
                })
            })
            .unwrap_or_else(TrackPb::empty);
        pb.split_milli = self.split_milli;
        pb.used = self.used;
        pb
    }

    /// Merge model-name keys (`YZ450F`) into displacement (`450`). Faster lap wins.
    fn fold_classes(&mut self) -> bool {
        let mut by_class: BTreeMap<String, TrackPb> = BTreeMap::new();
        let mut changed = false;
        for (k, pb) in std::mem::take(&mut self.bikes) {
            let class = if k.is_empty() {
                String::new()
            } else {
                let c = bike_class(&k);
                if c != k {
                    changed = true;
                }
                c
            };
            match by_class.get(&class) {
                None => {
                    by_class.insert(class, pb);
                }
                Some(old) => {
                    by_class.insert(class, merge_pb(old, &pb));
                    changed = true;
                }
            }
        }
        self.bikes = by_class;
        changed
    }

    /// First named bike on a v1 file takes the old tape so existing PBs are not dropped.
    fn adopt(&mut self, bike: &str) -> bool {
        if bike.is_empty() || self.bikes.contains_key(bike) || self.has_named() {
            return false;
        }
        if let Some(legacy) = self.bikes.remove("") {
            self.bikes.insert(bike.to_string(), legacy);
            true
        } else {
            false
        }
    }

    fn put_bike(&mut self, bike: &str, pb: &TrackPb) {
        let mut body = pb.clone();
        body.split_milli = [0; 2];
        body.used = 0;
        self.bikes.insert(bike_class(bike), body);
    }
}

struct Store {
    dir: Option<PathBuf>,
    track: String,
    bike: String,
    file: TrackFile,
    pb: TrackPb,
}

impl Store {
    const fn new() -> Self {
        Self {
            dir: None,
            track: String::new(),
            bike: String::new(),
            file: TrackFile::empty(),
            pb: TrackPb {
                lap_ms: 0,
                bins: [0; BINS],
                sectors: [0; 3],
                split_milli: [0; 2],
                used: 0,
            },
        }
    }
}

static STORE: Mutex<Store> = Mutex::new(Store::new());

#[cfg(test)]
thread_local! {
    static THREAD_PERSIST: std::cell::Cell<bool> = std::cell::Cell::new(false);
}

#[cfg(test)]
fn thread_persist() -> bool {
    THREAD_PERSIST.with(|c| c.get())
}

#[cfg(not(test))]
fn thread_persist() -> bool {
    true
}

#[cfg(test)]
static TEST_LOCK: Mutex<()> = Mutex::new(());

#[cfg(test)]
pub fn exclusive_test() -> std::sync::MutexGuard<'static, ()> {
    TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
pub fn reset_store() {
    *live() = Store::new();
    THREAD_PERSIST.with(|c| c.set(false));
}

fn live() -> std::sync::MutexGuard<'static, Store> {
    STORE.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn set_store_dir(dir: PathBuf) {
    let _ = fs::create_dir_all(&dir);
    let mut g = live();
    g.dir = Some(dir);
    g.track.clear();
    g.bike.clear();
    g.file = TrackFile::empty();
    g.pb = TrackPb::empty();
    #[cfg(test)]
    THREAD_PERSIST.with(|c| c.set(true));
}

pub fn slug(name: &str) -> String {
    let mut s = String::new();
    for c in name.chars() {
        match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => s.push('_'),
            c if c.is_control() => {}
            _ => s.push(c),
        }
    }
    let s = s.trim_matches(|c: char| c == ' ' || c == '.').to_string();
    if s.is_empty() {
        "_".into()
    } else {
        s
    }
}

fn merge_pb(a: &TrackPb, b: &TrackPb) -> TrackPb {
    let mut out = a.clone();
    if b.lap_ms > 0 && (out.lap_ms <= 0 || b.lap_ms < out.lap_ms) {
        out.lap_ms = b.lap_ms;
        out.bins = b.bins;
    }
    for i in 0..3 {
        if b.sectors[i] > 0 && (out.sectors[i] <= 0 || b.sectors[i] < out.sectors[i]) {
            out.sectors[i] = b.sectors[i];
        }
    }
    out
}

/// Displacement class used as the JSON key: `250`, `450`, …
/// Yamaha 250 and Honda 250 share; a 450 is separate.
pub fn bike_class(name: &str) -> String {
    let name = name.trim();
    if name.is_empty() {
        return String::new();
    }
    if let Some(cc) = displacement(name) {
        return cc.to_string();
    }
    name.to_string()
}

fn displacement(name: &str) -> Option<u32> {
    const CLASSES: &[u32] = &[50, 65, 85, 100, 110, 125, 144, 150, 250, 300, 350, 450, 500];
    let mut found = Vec::new();
    let mut n = 0u32;
    let mut digits = 0u32;
    let push = |found: &mut Vec<u32>, n: u32, digits: u32| {
        if (2..=3).contains(&digits) {
            found.push(n);
        }
    };
    for c in name.chars() {
        if c.is_ascii_digit() {
            n = n.saturating_mul(10).saturating_add(c as u32 - b'0' as u32);
            digits += 1;
        } else if digits > 0 {
            push(&mut found, n, digits);
            n = 0;
            digits = 0;
        }
    }
    if digits > 0 {
        push(&mut found, n, digits);
    }
    found
        .iter()
        .copied()
        .filter(|x| CLASSES.contains(x))
        .max()
        .or_else(|| {
            found
                .iter()
                .copied()
                .filter(|&x| (50..=550).contains(&x))
                .max()
        })
}

fn path_for(dir: &Path, track: &str) -> PathBuf {
    dir.join(format!("{}.json", slug(track)))
}

fn resolve_bike(requested: &str, last: &str) -> String {
    let raw = if requested.is_empty() { last } else { requested };
    bike_class(raw)
}

fn load_track(g: &mut Store, track: &str) -> bool {
    g.track = track.to_string();
    g.file = g
        .dir
        .as_ref()
        .and_then(|dir| load_file(&path_for(dir, track)))
        .unwrap_or_else(TrackFile::empty);
    g.file.fold_classes()
}

fn apply_bike(g: &mut Store, bike: String) -> bool {
    let adopted = g.file.adopt(&bike);
    g.bike = bike;
    g.pb = g.file.select(&g.bike);
    adopted
}

fn sync_class(g: &mut Store, track: &str, class: String) -> bool {
    if track != g.track {
        let folded = load_track(g, track);
        apply_bike(g, class) || folded
    } else if class != g.bike {
        apply_bike(g, class)
    } else {
        false
    }
}

fn sync_track(g: &mut Store, track: &str, bike: &str) -> bool {
    sync_class(g, track, resolve_bike(bike, &g.bike))
}

/// Switch to this track and bike. Empty track keeps the current cache.
/// Empty bike keeps the last known bike so a standings flicker does not drop the tape.
pub fn bind(track: &str, bike: &str) -> TrackPb {
    if track.is_empty() || !thread_persist() {
        return if thread_persist() {
            live().pb.clone()
        } else {
            TrackPb::empty()
        };
    }
    let mut g = live();
    let bike = resolve_bike(bike, &g.bike);
    if track == g.track && bike == g.bike {
        return g.pb.clone();
    }
    let on_disk = g
        .dir
        .as_ref()
        .is_some_and(|dir| path_for(dir, track).is_file());
    let adopted = sync_track(&mut g, track, &bike);
    if adopted || (on_disk && should_touch(g.file.used)) {
        persist(&mut g);
    }
    g.pb.clone()
}

/// Bind this track to an exact class key. Empty bike stays empty — does not
/// reuse the last named class (standings lag must not load a 250 onto a 450).
pub fn bind_exact(track: &str, bike: &str) -> TrackPb {
    if track.is_empty() || !thread_persist() {
        return if thread_persist() {
            live().pb.clone()
        } else {
            TrackPb::empty()
        };
    }
    let mut g = live();
    let bike = bike_class(bike);
    if track == g.track && bike == g.bike {
        return g.pb.clone();
    }
    let on_disk = g
        .dir
        .as_ref()
        .is_some_and(|dir| path_for(dir, track).is_file());
    let adopted = sync_class(&mut g, track, bike);
    if adopted || (on_disk && should_touch(g.file.used)) {
        persist(&mut g);
    }
    g.pb.clone()
}

pub fn current() -> TrackPb {
    live().pb.clone()
}

pub fn current_key() -> String {
    live().track.clone()
}

pub fn current_bike() -> String {
    live().bike.clone()
}

/// Replace the tape when this lap is faster (or the first).
pub fn commit_tape(track: &str, bike: &str, lap_ms: i32, bins: [i32; BINS]) -> bool {
    if !thread_persist() || track.is_empty() || lap_ms <= 0 {
        return false;
    }
    let mut g = live();
    sync_track(&mut g, track, bike);
    if g.pb.lap_ms > 0 && lap_ms >= g.pb.lap_ms {
        return false;
    }
    g.pb.lap_ms = lap_ms;
    g.pb.bins = bins;
    let bike = g.bike.clone();
    let pb = g.pb.clone();
    g.file.put_bike(&bike, &pb);
    persist(&mut g);
    true
}

/// Replace one sector duration when this frozen split is faster (or the first).
pub fn commit_sector(track: &str, bike: &str, i: usize, duration_ms: i32) -> bool {
    if !thread_persist() || track.is_empty() || i >= 3 || duration_ms <= 0 {
        return false;
    }
    let mut g = live();
    sync_track(&mut g, track, bike);
    if g.pb.sectors[i] > 0 && duration_ms >= g.pb.sectors[i] {
        return false;
    }
    g.pb.sectors[i] = duration_ms;
    let bike = g.bike.clone();
    let pb = g.pb.clone();
    g.file.put_bike(&bike, &pb);
    persist(&mut g);
    true
}

/// Remember where S1 / S2 fire on this track (thousandths of a lap).
pub fn note_split_pos(track: &str, bike: &str, i: usize, pos: f32) -> bool {
    if !thread_persist() || track.is_empty() || i >= 2 {
        return false;
    }
    if !(0.04..0.96).contains(&pos) {
        return false;
    }
    let milli = (pos * 1000.0).round() as i32;
    let mut g = live();
    sync_track(&mut g, track, bike);
    if (g.file.split_milli[i] - milli).abs() < 8 {
        return false;
    }
    g.file.split_milli[i] = milli;
    g.pb.split_milli[i] = milli;
    persist(&mut g);
    true
}

/// Time on the saved tape at this lap fraction.
pub fn time_at(bins: &[i32; BINS], pos: f32) -> Option<i32> {
    if !(0.0..1.0).contains(&pos) {
        return None;
    }
    let f = (pos * BINS as f32).min((BINS - 1) as f32);
    let i = f as usize;
    let mut lo = None;
    for j in (0..=i).rev() {
        if bins[j] > 0 {
            lo = Some((j, bins[j]));
            break;
        }
    }
    let mut hi = None;
    let hi_from = lo.map(|(ia, _)| ia + 1).unwrap_or(i);
    for j in hi_from..BINS {
        if bins[j] > 0 {
            hi = Some((j, bins[j]));
            break;
        }
    }
    match (lo, hi) {
        (Some((ia, ta)), Some((ib, tb))) if ib > ia => {
            if ib - ia > BINS / 4 {
                return None;
            }
            let t = ((f - ia as f32) / (ib as f32 - ia as f32)).clamp(0.0, 1.0);
            Some(ta + ((tb - ta) as f32 * t).round() as i32)
        }
        (Some((ia, t)), None) if f - ia as f32 <= 8.0 => Some(t),
        (None, Some((ib, t))) if ib as f32 - f <= 8.0 => Some(t),
        _ => None,
    }
}

/// Lap fraction where the tape first reaches `target_ms`.
pub fn pos_at_time(bins: &[i32; BINS], target: i32) -> Option<f32> {
    if target <= 0 {
        return None;
    }
    let mut prev: Option<(usize, i32)> = None;
    for (i, &ms) in bins.iter().enumerate() {
        if ms <= 0 {
            continue;
        }
        if ms >= target {
            let pos = if let Some((j, t0)) = prev {
                if ms == t0 {
                    i as f32 / BINS as f32
                } else {
                    let t = (target - t0) as f32 / (ms - t0) as f32;
                    (j as f32 + t * (i - j) as f32) / BINS as f32
                }
            } else {
                i as f32 / BINS as f32
            };
            return Some(pos.clamp(0.0, 0.999));
        }
        prev = Some((i, ms));
    }
    None
}

pub fn clear_current() {
    let mut g = live();
    if let (Some(dir), true) = (g.dir.as_ref(), !g.track.is_empty()) {
        let _ = fs::remove_file(path_for(dir, &g.track));
    }
    g.file = TrackFile::empty();
    g.pb = TrackPb::empty();
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn should_touch(used: i64) -> bool {
    used <= 0 || now_unix().saturating_sub(used) >= TOUCH_SECS
}

fn persist(g: &mut Store) {
    let now = now_unix();
    g.file.used = now;
    g.pb.used = now;
    let Some(dir) = g.dir.as_ref() else {
        return;
    };
    if g.track.is_empty() {
        return;
    }
    let _ = fs::create_dir_all(dir);
    let path = path_for(dir, &g.track);
    let _ = atomic_write(&path, encode_file(&g.file).as_bytes());
}

fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, bytes)?;
    let _ = fs::remove_file(path);
    fs::rename(&tmp, path)
}

fn encode_bins(bins: &[i32; BINS]) -> String {
    let mut o = String::from("[");
    for (i, b) in bins.iter().enumerate() {
        if i > 0 {
            o.push(',');
        }
        o.push_str(&b.to_string());
    }
    o.push(']');
    o
}

fn encode_body(pb: &TrackPb) -> String {
    let mut o = String::from("{\"ms\":");
    o.push_str(&pb.lap_ms.to_string());
    o.push_str(",\"bins\":");
    o.push_str(&encode_bins(&pb.bins));
    o.push_str(",\"s\":[");
    o.push_str(&pb.sectors[0].to_string());
    o.push(',');
    o.push_str(&pb.sectors[1].to_string());
    o.push(',');
    o.push_str(&pb.sectors[2].to_string());
    o.push_str("]}");
    o
}

fn encode_file(file: &TrackFile) -> String {
    if !file.has_named() {
        let mut pb = file.select("");
        pb.split_milli = file.split_milli;
        pb.used = file.used;
        return encode(&pb);
    }
    let mut o = String::from("{\"v\":2,\"p\":[");
    o.push_str(&file.split_milli[0].to_string());
    o.push(',');
    o.push_str(&file.split_milli[1].to_string());
    o.push_str("],\"used\":");
    o.push_str(&file.used.to_string());
    o.push_str(",\"bikes\":{");
    let mut first = true;
    for (name, pb) in &file.bikes {
        if name.is_empty() {
            continue;
        }
        if !first {
            o.push(',');
        }
        first = false;
        o.push('"');
        o.push_str(&json_escape(name));
        o.push_str("\":");
        o.push_str(&encode_body(pb));
    }
    o.push_str("}}");
    o
}

pub fn encode(pb: &TrackPb) -> String {
    let mut o = String::from("{\"v\":1,\"ms\":");
    o.push_str(&pb.lap_ms.to_string());
    o.push_str(",\"bins\":");
    o.push_str(&encode_bins(&pb.bins));
    o.push_str(",\"s\":[");
    o.push_str(&pb.sectors[0].to_string());
    o.push(',');
    o.push_str(&pb.sectors[1].to_string());
    o.push(',');
    o.push_str(&pb.sectors[2].to_string());
    o.push_str("],\"p\":[");
    o.push_str(&pb.split_milli[0].to_string());
    o.push(',');
    o.push_str(&pb.split_milli[1].to_string());
    o.push_str("],\"used\":");
    o.push_str(&pb.used.to_string());
    o.push('}');
    o
}

pub fn decode(text: &str) -> Option<TrackPb> {
    decode_body(text)
}

fn decode_body(text: &str) -> Option<TrackPb> {
    let ms = int_field(text, "\"ms\":")?;
    let bins_raw = array_field(text, "\"bins\":")?;
    let sec_raw = array_field(text, "\"s\":")?;
    let mut pb = TrackPb::empty();
    pb.lap_ms = ms;
    let mut n = 0usize;
    for part in bins_raw.split(',') {
        if n >= BINS {
            break;
        }
        pb.bins[n] = part.trim().parse().ok()?;
        n += 1;
    }
    if n != BINS {
        return None;
    }
    let secs: Vec<i32> = sec_raw
        .split(',')
        .filter_map(|p| p.trim().parse().ok())
        .collect();
    if secs.len() != 3 {
        return None;
    }
    pb.sectors = [secs[0], secs[1], secs[2]];
    if let Some(p_raw) = array_field(text, "\"p\":") {
        let ps: Vec<i32> = p_raw
            .split(',')
            .filter_map(|p| p.trim().parse().ok())
            .collect();
        if ps.len() == 2 {
            pb.split_milli = [ps[0], ps[1]];
        }
    }
    if let Some(used) = int64_field(text, "\"used\":") {
        pb.used = used;
    }
    Some(pb)
}

fn decode_file(text: &str) -> Option<TrackFile> {
    if let Some(inner) = object_inner(text, "\"bikes\":") {
        let mut file = TrackFile::empty();
        file.bikes = parse_bikes(inner);
        if let Some(p_raw) = array_field(text, "\"p\":") {
            let ps: Vec<i32> = p_raw
                .split(',')
                .filter_map(|p| p.trim().parse().ok())
                .collect();
            if ps.len() == 2 {
                file.split_milli = [ps[0], ps[1]];
            }
        }
        if let Some(used) = int64_field(text, "\"used\":") {
            file.used = used;
        }
        return Some(file);
    }
    let pb = decode_body(text)?;
    let mut file = TrackFile::empty();
    file.split_milli = pb.split_milli;
    file.used = pb.used;
    let mut body = pb;
    body.split_milli = [0; 2];
    body.used = 0;
    file.bikes.insert(String::new(), body);
    Some(file)
}

fn load_file(path: &Path) -> Option<TrackFile> {
    decode_file(&fs::read_to_string(path).ok()?)
}

fn json_escape(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if c.is_control() => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o
}

fn object_inner<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let i = text.find(key)? + key.len();
    brace_inner(text[i..].trim_start())
}

fn brace_inner(rest: &str) -> Option<&str> {
    if !rest.starts_with('{') {
        return None;
    }
    let bytes = rest.as_bytes();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    for (i, &b) in bytes.iter().enumerate() {
        if in_str {
            if esc {
                esc = false;
                continue;
            }
            if b == b'\\' {
                esc = true;
                continue;
            }
            if b == b'"' {
                in_str = false;
            }
            continue;
        }
        match b {
            b'"' => in_str = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&rest[1..i]);
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_json_string(s: &str) -> Option<(String, &str)> {
    if !s.starts_with('"') {
        return None;
    }
    let bytes = s.as_bytes();
    let mut o = String::new();
    let mut i = 1;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\\' {
            i += 1;
            if i >= bytes.len() {
                return None;
            }
            match bytes[i] {
                b'"' => o.push('"'),
                b'\\' => o.push('\\'),
                b'n' => o.push('\n'),
                b't' => o.push('\t'),
                c => o.push(c as char),
            }
            i += 1;
            continue;
        }
        if b == b'"' {
            return Some((o, &s[i + 1..]));
        }
        o.push(b as char);
        i += 1;
    }
    None
}

fn parse_bikes(inner: &str) -> BTreeMap<String, TrackPb> {
    let mut m = BTreeMap::new();
    let mut s = inner.trim();
    while !s.is_empty() {
        s = s.trim_start_matches(|c: char| c == ',' || c.is_whitespace());
        if s.is_empty() {
            break;
        }
        let Some((key, rest)) = parse_json_string(s) else {
            break;
        };
        let rest = rest.trim_start();
        if !rest.starts_with(':') {
            break;
        }
        let rest = rest[1..].trim_start();
        let Some(body) = brace_inner(rest) else {
            break;
        };
        if let Some(pb) = decode_body(&format!("{{{body}}}")) {
            m.insert(key, pb);
        }
        let span = 1 + body.len() + 1;
        s = rest[span..].trim_start();
    }
    m
}

fn int_field(text: &str, key: &str) -> Option<i32> {
    int64_field(text, key)?.try_into().ok()
}

fn int64_field(text: &str, key: &str) -> Option<i64> {
    let i = text.find(key)? + key.len();
    let rest = text[i..].trim_start();
    let end = rest
        .find(|c: char| c != '-' && !c.is_ascii_digit())
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

fn array_field<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let i = text.find(key)? + key.len();
    let rest = text[i..].trim_start();
    if !rest.starts_with('[') {
        return None;
    }
    let close = rest.find(']')?;
    Some(&rest[1..close])
}

#[cfg(test)]
#[path = "tests/track_pb.rs"]
mod tests;
