//! Per-track personal bests for Delta Bar and Sectors.
//!
//! One JSON file per `track_name` under AppData. Load the current track only.

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

struct Store {
    dir: Option<PathBuf>,
    key: String,
    pb: TrackPb,
}

impl Store {
    const fn new() -> Self {
        Self {
            dir: None,
            key: String::new(),
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

fn live() -> std::sync::MutexGuard<'static, Store> {
    STORE.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn set_store_dir(dir: PathBuf) {
    let _ = fs::create_dir_all(&dir);
    let mut g = live();
    g.dir = Some(dir);
    g.key.clear();
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

fn path_for(dir: &Path, key: &str) -> PathBuf {
    dir.join(format!("{}.json", slug(key)))
}

/// Switch to this track name. Empty name keeps the current cache.
pub fn bind(key: &str) -> TrackPb {
    if key.is_empty() || !thread_persist() {
        return if thread_persist() {
            live().pb.clone()
        } else {
            TrackPb::empty()
        };
    }
    let mut g = live();
    if key == g.key {
        return g.pb.clone();
    }
    g.key = key.to_string();
    let loaded = g
        .dir
        .as_ref()
        .and_then(|dir| load_file(&path_for(dir, key)));
    let existed = loaded.is_some();
    g.pb = loaded.unwrap_or_else(TrackPb::empty);
    if existed && should_touch(g.pb.used) {
        persist(&mut g);
    }
    g.pb.clone()
}

pub fn current() -> TrackPb {
    live().pb.clone()
}

pub fn current_key() -> String {
    live().key.clone()
}

/// Replace the tape when this lap is faster (or the first).
pub fn commit_tape(key: &str, lap_ms: i32, bins: [i32; BINS]) -> bool {
    if !thread_persist() || key.is_empty() || lap_ms <= 0 {
        return false;
    }
    let mut g = live();
    if key != g.key {
        g.key = key.to_string();
        g.pb = g
            .dir
            .as_ref()
            .and_then(|dir| load_file(&path_for(dir, key)))
            .unwrap_or_else(TrackPb::empty);
    }
    if g.pb.lap_ms > 0 && lap_ms >= g.pb.lap_ms {
        return false;
    }
    g.pb.lap_ms = lap_ms;
    g.pb.bins = bins;
    persist(&mut g);
    true
}

/// Replace one sector duration when this frozen split is faster (or the first).
pub fn commit_sector(key: &str, i: usize, duration_ms: i32) -> bool {
    if !thread_persist() || key.is_empty() || i >= 3 || duration_ms <= 0 {
        return false;
    }
    let mut g = live();
    if key != g.key {
        g.key = key.to_string();
        g.pb = g
            .dir
            .as_ref()
            .and_then(|dir| load_file(&path_for(dir, key)))
            .unwrap_or_else(TrackPb::empty);
    }
    if g.pb.sectors[i] > 0 && duration_ms >= g.pb.sectors[i] {
        return false;
    }
    g.pb.sectors[i] = duration_ms;
    persist(&mut g);
    true
}

/// Remember where S1 / S2 fire on this track (thousandths of a lap).
pub fn note_split_pos(key: &str, i: usize, pos: f32) -> bool {
    if !thread_persist() || key.is_empty() || i >= 2 {
        return false;
    }
    if !(0.04..0.96).contains(&pos) {
        return false;
    }
    let milli = (pos * 1000.0).round() as i32;
    let mut g = live();
    if key != g.key {
        g.key = key.to_string();
        g.pb = g
            .dir
            .as_ref()
            .and_then(|dir| load_file(&path_for(dir, key)))
            .unwrap_or_else(TrackPb::empty);
    }
    if (g.pb.split_milli[i] - milli).abs() < 8 {
        return false;
    }
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
    if let (Some(dir), true) = (g.dir.as_ref(), !g.key.is_empty()) {
        let _ = fs::remove_file(path_for(dir, &g.key));
    }
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
    g.pb.used = now_unix();
    let Some(dir) = g.dir.as_ref() else {
        return;
    };
    if g.key.is_empty() {
        return;
    }
    let _ = fs::create_dir_all(dir);
    let path = path_for(dir, &g.key);
    let _ = atomic_write(&path, encode(&g.pb).as_bytes());
}

fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, bytes)?;
    let _ = fs::remove_file(path);
    fs::rename(&tmp, path)
}

pub fn encode(pb: &TrackPb) -> String {
    let mut o = String::from("{\"v\":1,\"ms\":");
    o.push_str(&pb.lap_ms.to_string());
    o.push_str(",\"bins\":[");
    for (i, b) in pb.bins.iter().enumerate() {
        if i > 0 {
            o.push(',');
        }
        o.push_str(&b.to_string());
    }
    o.push_str("],\"s\":[");
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

fn load_file(path: &Path) -> Option<TrackPb> {
    decode(&fs::read_to_string(path).ok()?)
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
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp_dir() -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("mxbo-pb-{n}"));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn slug_strips_windows_illegal() {
        assert_eq!(slug("Paleta Raceway v2"), "Paleta Raceway v2");
        assert_eq!(slug("a/b:c*"), "a_b_c_");
        assert_eq!(slug("..."), "_");
    }

    #[test]
    fn round_trip_json() {
        let mut pb = TrackPb::empty();
        pb.lap_ms = 71_234;
        pb.bins[0] = 10;
        pb.bins[255] = 71_200;
        pb.sectors = [24_093, 25_640, 22_910];
        pb.split_milli = [333, 671];
        pb.used = 1_700_000_000;
        let back = decode(&encode(&pb)).unwrap();
        assert_eq!(back, pb);
    }

    fn bins_json() -> String {
        let mut bins = String::from("[");
        for i in 0..BINS {
            if i > 0 {
                bins.push(',');
            }
            bins.push('0');
        }
        bins.push(']');
        bins
    }

    #[test]
    fn old_json_without_split_pos_loads() {
        let text = format!(
            "{{\"v\":1,\"ms\":1,\"bins\":{},\"s\":[1,2,3]}}",
            bins_json()
        );
        let pb = decode(&text).unwrap();
        assert_eq!(pb.sectors, [1, 2, 3]);
        assert_eq!(pb.split_milli, [0, 0]);
        assert_eq!(pb.used, 0);
    }

    #[test]
    fn old_json_without_used_loads() {
        let text = format!(
            "{{\"v\":1,\"ms\":1,\"bins\":{},\"s\":[1,2,3],\"p\":[333,671]}}",
            bins_json()
        );
        let pb = decode(&text).unwrap();
        assert_eq!(pb.split_milli, [333, 671]);
        assert_eq!(pb.used, 0);
    }

    #[test]
    fn time_at_reads_bin_and_interpolates() {
        let mut bins = [0; BINS];
        bins[10] = 1_000;
        bins[20] = 2_000;
        assert_eq!(time_at(&bins, 10.0 / BINS as f32), Some(1_000));
        assert_eq!(time_at(&bins, 15.0 / BINS as f32), Some(1_500));
        bins[11] = 1_100;
        assert_eq!(time_at(&bins, 10.5 / BINS as f32), Some(1_050));
        bins[11] = 0;
        bins[12] = 1_200;
        assert_eq!(time_at(&bins, 10.5 / BINS as f32), Some(1_050));
        let a = time_at(&bins, 10.2 / BINS as f32).unwrap();
        let b = time_at(&bins, 10.8 / BINS as f32).unwrap();
        assert!(b > a, "fractional walk must move, {a} then {b}");
        assert_eq!(pos_at_time(&bins, 0), None);
        let p = pos_at_time(&bins, 1_500).unwrap();
        assert!((p * BINS as f32 - 15.0).abs() < 0.6, "pos {p}");
    }

    #[test]
    fn time_at_rejects_large_gaps() {
        let mut bins = [0; BINS];
        bins[0] = 1_000;
        bins[BINS / 4 + 2] = 10_000;
        assert_eq!(time_at(&bins, 0.2), None);
    }

    #[test]
    fn note_split_pos_rounds_and_skips_noise() {
        let _lock = exclusive_test();
        let dir = tmp_dir();
        set_store_dir(dir.clone());
        assert!(!note_split_pos("A", 0, 0.03));
        assert!(!note_split_pos("A", 0, 0.97));
        assert!(note_split_pos("A", 0, 0.33));
        assert_eq!(bind("A").split_milli[0], 330);
        assert!(!note_split_pos("A", 0, 0.333));
        assert!(note_split_pos("A", 1, 0.67));
        assert_eq!(bind("A").split_milli[1], 670);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn clear_current_drops_loaded_tape() {
        let _lock = exclusive_test();
        let dir = tmp_dir();
        set_store_dir(dir.clone());
        let mut bins = [0; BINS];
        bins[0] = 100;
        bins[200] = 80_000;
        assert!(commit_tape("A", 80_000, bins));
        assert!(bind("A").has_tape());
        clear_current();
        assert!(!bind("A").has_tape());
        assert_eq!(bind("A").lap_ms, 0);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn corrupt_file_is_ignored() {
        assert!(decode("{nope}").is_none());
        assert!(decode("{\"v\":1,\"ms\":1,\"bins\":[1],\"s\":[1,2,3]}").is_none());
    }

    #[test]
    fn faster_tape_overwrites_slower_does_not() {
        let _lock = exclusive_test();
        let dir = tmp_dir();
        set_store_dir(dir.clone());
        let mut bins = [0; BINS];
        bins[0] = 100;
        bins[200] = 80_000;
        assert!(commit_tape("A", 80_000, bins));
        let mut slower = bins;
        slower[200] = 90_000;
        assert!(!commit_tape("A", 90_000, slower));
        assert_eq!(bind("A").lap_ms, 80_000);
        let mut faster = bins;
        faster[200] = 70_000;
        assert!(commit_tape("A", 70_000, faster));
        assert_eq!(bind("A").lap_ms, 70_000);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn tracks_stay_isolated() {
        let _lock = exclusive_test();
        let dir = tmp_dir();
        set_store_dir(dir.clone());
        let mut a = [0; BINS];
        a[1] = 1;
        commit_tape("A", 60_000, a);
        commit_sector("A", 0, 20_000);
        let mut b = [0; BINS];
        b[1] = 2;
        commit_tape("B", 90_000, b);
        commit_sector("B", 0, 30_000);
        assert_eq!(bind("A").lap_ms, 60_000);
        assert_eq!(bind("A").sectors[0], 20_000);
        assert_eq!(bind("B").lap_ms, 90_000);
        assert_eq!(bind("B").sectors[0], 30_000);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn faster_sector_updates_file() {
        let _lock = exclusive_test();
        let dir = tmp_dir();
        set_store_dir(dir.clone());
        assert!(commit_sector("A", 0, 24_093));
        assert!(!commit_sector("A", 0, 25_000));
        assert!(commit_sector("A", 0, 23_000));
        assert_eq!(bind("A").sectors[0], 23_000);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn commit_tape_stamps_used() {
        let _lock = exclusive_test();
        let dir = tmp_dir();
        set_store_dir(dir.clone());
        let mut bins = [0; BINS];
        bins[0] = 100;
        bins[200] = 80_000;
        let before = now_unix();
        assert!(commit_tape("A", 80_000, bins));
        let after = now_unix();
        let used = bind("A").used;
        assert!(used >= before && used <= after, "used {used} not in [{before}, {after}]");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn bind_does_not_create_file_for_unknown_track() {
        let _lock = exclusive_test();
        let dir = tmp_dir();
        set_store_dir(dir.clone());
        assert_eq!(bind("Ghost").used, 0);
        assert!(!dir.join("Ghost.json").exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn bind_stamps_existing_file_missing_used() {
        let _lock = exclusive_test();
        let dir = tmp_dir();
        let _ = fs::create_dir_all(&dir);
        let text = format!(
            "{{\"v\":1,\"ms\":80000,\"bins\":{},\"s\":[1,2,3],\"p\":[0,0]}}",
            bins_json()
        );
        fs::write(dir.join("A.json"), text).unwrap();
        set_store_dir(dir.clone());
        let before = now_unix();
        let pb = bind("A");
        let after = now_unix();
        assert_eq!(pb.lap_ms, 80_000);
        assert!(
            pb.used >= before && pb.used <= after,
            "used {} not in [{before}, {after}]",
            pb.used
        );
        let on_disk = fs::read_to_string(dir.join("A.json")).unwrap();
        assert!(on_disk.contains(&format!("\"used\":{}", pb.used)));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn bind_skips_rewrite_when_used_is_fresh() {
        let _lock = exclusive_test();
        let dir = tmp_dir();
        set_store_dir(dir.clone());
        let mut bins = [0; BINS];
        bins[0] = 100;
        bins[200] = 80_000;
        assert!(commit_tape("A", 80_000, bins));
        let stamped = bind("A").used;
        bind("B");
        assert_eq!(bind("A").used, stamped);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn bind_restamps_stale_used() {
        let _lock = exclusive_test();
        let dir = tmp_dir();
        let _ = fs::create_dir_all(&dir);
        let stale = now_unix() - TOUCH_SECS - 1;
        let text = format!(
            "{{\"v\":1,\"ms\":80000,\"bins\":{},\"s\":[0,0,0],\"p\":[0,0],\"used\":{stale}}}",
            bins_json()
        );
        fs::write(dir.join("A.json"), text).unwrap();
        set_store_dir(dir.clone());
        let before = now_unix();
        let used = bind("A").used;
        assert!(used >= before, "used {used} should refresh past stale {stale}");
        let _ = fs::remove_dir_all(dir);
    }
}
