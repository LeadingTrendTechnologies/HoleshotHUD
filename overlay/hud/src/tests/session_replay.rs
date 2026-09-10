use super::*;
use crate::shm::{write_name, Rider, Snapshot, Standing};

fn sessions_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/sessions")
}

fn json_i32(line: &str, key: &str) -> Option<i32> {
    let needle = format!("\"{key}\"");
    let i = line.find(&needle)?;
    let rest = line[i + needle.len()..].trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    let end = rest
        .find(|c: char| !(c.is_ascii_digit() || c == '-'))
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

fn json_f32(line: &str, key: &str) -> Option<f32> {
    let needle = format!("\"{key}\"");
    let i = line.find(&needle)?;
    let rest = line[i + needle.len()..].trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    let end = rest
        .find(|c: char| !(c.is_ascii_digit() || c == '-' || c == '.'))
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

fn json_bool(line: &str, key: &str) -> Option<bool> {
    let needle = format!("\"{key}\"");
    let i = line.find(&needle)?;
    let rest = line[i + needle.len()..].trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    if rest.starts_with("true") {
        Some(true)
    } else if rest.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

fn json_str(line: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let i = line.find(&needle)?;
    let rest = line[i + needle.len()..].trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    if !rest.starts_with('"') {
        return None;
    }
    let mut out = String::new();
    let mut chars = rest[1..].chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            out.push(chars.next()?);
        } else if ch == '"' {
            return Some(out);
        } else {
            out.push(ch);
        }
    }
    None
}

fn json_object<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{key}\"");
    let i = line.find(&needle)?;
    let rest = line[i + needle.len()..].trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    if !rest.starts_with('{') {
        return None;
    }
    let bytes = rest.as_bytes();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    for (j, &c) in bytes.iter().enumerate() {
        if in_str {
            if esc {
                esc = false;
            } else if c == b'\\' {
                esc = true;
            } else if c == b'"' {
                in_str = false;
            }
        } else if c == b'"' {
            in_str = true;
        } else if c == b'{' {
            depth += 1;
        } else if c == b'}' {
            depth -= 1;
            if depth == 0 {
                return rest.get(..=j);
            }
        }
    }
    None
}

fn standing(race_num: i32, position: i32, laps: i32) -> Standing {
    let mut row = Standing::default();
    row.race_num = race_num;
    row.position = position;
    row.num_laps = laps;
    write_name(&mut row.name, &format!("R{race_num}"));
    row
}

fn rider(race_num: i32, pos: f32) -> Rider {
    let mut r = Rider::default();
    r.race_num = race_num;
    r.track_pos = pos;
    write_name(&mut r.name, &format!("R{race_num}"));
    r
}

fn base_snap() -> Snapshot {
    let mut s = Snapshot {
        has_telemetry: 1,
        on_track: 1,
        local_race_num: 12,
        focus_race_num: 12,
        standing_count: 2,
        rider_count: 2,
        track_length: 1000.0,
        ..Snapshot::default()
    };
    s.standings[0] = standing(1, 1, 0);
    s.standings[1] = standing(12, 2, 0);
    s.riders[0] = rider(1, 0.10);
    s.riders[1] = rider(12, 0.50);
    write_name(&mut s.track_name, "Test Track");
    s
}

fn apply_line(s: &mut Snapshot, line: &str) {
    if let Some(v) = json_i32(line, "len") {
        s.session_length = v;
    }
    if let Some(v) = json_i32(line, "laps") {
        s.session_laps = v;
    }
    if let Some(v) = json_i32(line, "time") {
        s.session_time_ms = v;
    }
    if let Some(v) = json_i32(line, "cur") {
        s.current_lap = v;
    }
    if let Some(v) = json_i32(line, "lap_ms") {
        s.current_lap_ms = v;
    }
    if let Some(v) = json_i32(line, "last_ms") {
        s.last_lap_ms = v;
    }
    if let Some(v) = json_f32(line, "spd") {
        s.local_speed = v;
    }
    if let Some(v) = json_f32(line, "pos") {
        s.local_track_pos = v;
        s.riders[1].track_pos = v;
    }
    if let Some(v) = json_i32(line, "on") {
        s.on_track = v;
    }
    if let Some(v) = json_i32(line, "ll") {
        s.standings[1].num_laps = v;
    }
    if let Some(v) = json_i32(line, "ld") {
        s.standings[0].num_laps = v;
    }
    if let Some(v) = json_i32(line, "n") {
        s.standing_count = v;
        s.rider_count = v.max(s.rider_count);
    }
    if let Some(v) = json_i32(line, "kind") {
        s.session_kind = v;
    }
    if let Some(v) = json_i32(line, "state") {
        s.session_state = v;
    }
}

fn mode_name(m: ClockMode) -> &'static str {
    match m {
        ClockMode::Practice => "Practice",
        ClockMode::Gate => "Gate",
        ClockMode::LapRace => "LapRace",
        ClockMode::Timed => "Timed",
        ClockMode::Overtime => "Overtime",
    }
}

fn flag_name(f: RaceFlag) -> &'static str {
    match f {
        RaceFlag::None => "None",
        RaceFlag::White => "White",
        RaceFlag::Checkered => "Checkered",
    }
}

fn assert_expect(store: &RaceStore, expect: &str, file: &str, line_no: usize) {
    if let Some(want) = json_str(expect, "mode") {
        assert_eq!(
            mode_name(store.clock.mode),
            want,
            "{file}:{line_no} mode"
        );
    }
    if let Some(want) = json_str(expect, "banner") {
        assert_eq!(store.clock.banner.1, want, "{file}:{line_no} banner");
    }
    if let Some(want) = json_bool(expect, "gate") {
        assert_eq!(store.clock.in_gate, want, "{file}:{line_no} gate");
    }
    if let Some(want) = json_bool(expect, "expired") {
        assert_eq!(store.clock.expired, want, "{file}:{line_no} expired");
    }
    if let Some(want) = json_str(expect, "flag") {
        assert_eq!(flag_name(store.clock.flag), want, "{file}:{line_no} flag");
    }
    if let Some(want) = json_i32(expect, "remain") {
        assert_eq!(
            store.clock.remain_ms.unwrap_or(-1),
            want,
            "{file}:{line_no} remain"
        );
    }
}

fn replay_file(name: &str) {
    let _g = session_test_lock();
    reset_session_clock_track();
    LAST_SESSION_SIG.store(0, Ordering::Relaxed);
    LAST_CUR_LAP.store(0, Ordering::Relaxed);
    let path = sessions_dir().join(name);
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let mut snap = base_snap();
    for (i, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        apply_line(&mut snap, line);
        let store = RaceStore::tick(&snap);
        if let Some(expect) = json_object(line, "expect") {
            assert_expect(&store, expect, name, i + 1);
        }
    }
}

#[test]
fn replays_gate_countdown() {
    replay_file("gate-countdown.jsonl");
}

#[test]
fn replays_lap_race_laps() {
    replay_file("lap-race-laps.jsonl");
}

#[test]
fn replays_timed_green() {
    replay_file("timed-green.jsonl");
}

#[test]
fn session_fixtures_exist() {
    let dir = sessions_dir();
    assert!(dir.join("gate-countdown.jsonl").is_file());
    assert!(dir.join("lap-race-laps.jsonl").is_file());
    assert!(dir.join("timed-green.jsonl").is_file());
}
