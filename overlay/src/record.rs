use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

use mxbo_hud::snapshot::{cstr, Snapshot};
use mxbo_hud::{clock_sample, ClockSample};

const RACE_LOG: &str = "race.jsonl";
const LAST_RACE: &str = "last-race.jsonl";
const RACE_SEND: &str = "race-send.jsonl";
const MAX_LOG: usize = 2 * 1024 * 1024;

static LOG: Mutex<Option<ClockLog>> = Mutex::new(None);

fn live() -> std::sync::MutexGuard<'static, Option<ClockLog>> {
    LOG.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn init() {
    *live() = Some(ClockLog::new());
}

pub fn path() -> Option<PathBuf> {
    live().as_ref().and_then(|l| l.path().map(|p| p.to_path_buf()))
}

pub fn rotate() {
    if let Some(log) = live().as_mut() {
        log.rotate();
    }
}

pub fn tick(snap: &Snapshot) {
    if let Some(log) = live().as_mut() {
        log.tick(snap);
    }
}

pub struct ClockLog {
    writer: Option<BufWriter<File>>,
    path: PathBuf,
    last_path: PathBuf,
    body: String,
    truncated: bool,
    last_truncated: bool,
    last_ready: bool,
    last_len: usize,
    started: Instant,
    last_write: Instant,
    last_key: Option<(i32, i32, i32, i32, i32, i32, i32, i32, i32)>,
    lines: u32,
    gate: SessionGate,
}

impl ClockLog {
    pub fn new() -> Self {
        let mut log = Self {
            writer: None,
            path: PathBuf::new(),
            last_path: PathBuf::new(),
            body: String::new(),
            truncated: false,
            last_truncated: false,
            last_ready: false,
            last_len: 0,
            started: Instant::now(),
            last_write: Instant::now() - std::time::Duration::from_secs(10),
            last_key: None,
            lines: 0,
            gate: SessionGate::default(),
        };
        log.open(false);
        log
    }

    pub fn path(&self) -> Option<&Path> {
        if self.writer.is_some() {
            Some(self.path.as_path())
        } else {
            None
        }
    }

    pub fn rotate(&mut self) {
        self.reset();
    }

    pub fn tick(&mut self, snap: &Snapshot) {
        let track = cstr(&snap.track_name);
        match self.gate.update(
            &track,
            snap.session_laps,
            snap.session_length,
            snap.session_time_ms,
            snap.current_lap,
            snap.on_track,
        ) {
            SessionEvent::RaceEnded => {
                self.archive_last();
                return;
            }
            SessionEvent::NewSession => {
                self.reset();
            }
            SessionEvent::Continue => {}
        }
        if snap.session_length <= 0
            && snap.session_time_ms <= 0
            && snap.session_laps <= 0
            && snap.current_lap <= 0
        {
            return;
        }
        let sample = clock_sample(snap);
        let key = (
            sample.session_length,
            sample.session_laps,
            sample.session_time_ms / 100,
            sample.current_lap,
            sample.local_laps,
            sample.leader_laps,
            sample.remain_ms / 100,
            sample.mode * 1000 + sample.gate * 100 + sample.armed * 10 + sample.expired,
            sample.locked_len,
        );
        if self.last_key.as_ref() == Some(&key) && self.last_write.elapsed().as_millis() < 250 {
            return;
        }
        self.last_key = Some(key);
        self.last_write = Instant::now();
        self.write_line(snap, &sample);
    }

    fn reset(&mut self) {
        self.archive_last();
        self.gate.saw_race = false;
        self.open(true);
    }

    fn archive_last(&mut self) {
        self.flush();
        if !raw_has_race(&self.body) {
            return;
        }
        let Some(dir) = self.path.parent().filter(|d| !d.as_os_str().is_empty()) else {
            return;
        };
        let dest = dir.join(LAST_RACE);
        if fs::write(&dest, self.body.as_bytes()).is_ok() {
            self.last_path = dest;
            self.last_truncated = self.truncated;
            self.last_ready = true;
            self.last_len = self.body.len();
        }
    }

    fn open(&mut self, clear: bool) {
        self.flush();
        self.writer = None;
        self.last_key = None;
        self.started = Instant::now();
        self.truncated = false;
        let mut errors = String::new();
        for dir in log_dirs() {
            if let Err(e) = fs::create_dir_all(&dir) {
                errors.push_str(&format!("mkdir {}: {e}\n", dir.display()));
                continue;
            }
            remove_old_logs(&dir);
            let path = dir.join(RACE_LOG);
            let (existing, was_big) = if clear {
                (String::new(), false)
            } else {
                read_tail(&path)
            };
            let rewrite = clear || existing.trim().is_empty() || was_big;
            let mut opts = OpenOptions::new();
            opts.create(true).write(true);
            if rewrite {
                opts.truncate(true);
            } else {
                opts.append(true);
            }
            match opts.open(&path) {
                Ok(f) => {
                    let mut w = BufWriter::new(f);
                    if clear || existing.trim().is_empty() {
                        let header = format!(
                            "{{\"v\":1,\"file\":\"{}\"}}",
                            json_escape(&path.display().to_string())
                        );
                        let _ = writeln!(w, "{header}");
                        let _ = w.flush();
                        self.body = header;
                        self.body.push('\n');
                        self.lines = 1;
                    } else {
                        self.body = existing;
                        if !self.body.ends_with('\n') {
                            self.body.push('\n');
                        }
                        if trim_body(&mut self.body) {
                            self.truncated = true;
                        }
                        self.truncated |= was_big;
                        if rewrite {
                            let _ = w.write_all(self.body.as_bytes());
                            let _ = w.flush();
                        }
                        self.lines = self.body.lines().count() as u32;
                    }
                    self.path = path;
                    let last = dir.join(LAST_RACE);
                    if last.is_file() {
                        self.last_path = last.clone();
                        self.last_ready = log_has_race(&last);
                        self.last_len = file_len(&last) as usize;
                        self.last_truncated = self.last_len as u64 > MAX_LOG as u64;
                    }
                    self.writer = Some(w);
                    write_boot(&format!("ok {}", self.path.display()));
                    return;
                }
                Err(e) => errors.push_str(&format!("open {}: {e}\n", path.display())),
            }
        }
        self.body.clear();
        write_boot(&format!("failed\n{errors}"));
    }

    fn write_line(&mut self, snap: &Snapshot, s: &ClockSample) {
        let t = self.started.elapsed().as_secs_f32();
        let track = cstr(&snap.track_name);
        let line = format!(
            "{{\"t\":{t:.2},\"seq\":{},\"track\":\"{}\",\"len\":{},\"laps\":{},\"time\":{},\"cur\":{},\"lap_ms\":{},\"last_ms\":{},\"spd\":{:.1},\"pos\":{:.3},\"on\":{},\"ll\":{},\"ld\":{},\"n\":{},\"dash\":\"{}\",\"rem\":{},\"mode\":{},\"gate\":{},\"arm\":{},\"exp\":{},\"saw\":{},\"lock\":{},\"otl\":{},\"otb\":{}}}",
            s.seq,
            json_escape(&track),
            s.session_length,
            s.session_laps,
            s.session_time_ms,
            s.current_lap,
            s.current_lap_ms,
            s.last_lap_ms,
            s.local_speed,
            s.local_track_pos,
            s.on_track,
            s.local_laps,
            s.leader_laps,
            s.standing_count,
            json_escape(&s.dash),
            s.remain_ms,
            s.mode,
            s.gate,
            s.armed,
            s.expired,
            s.saw,
            s.locked_len,
            s.ot_local,
            s.ot_lead,
        );
        self.body.push_str(&line);
        self.body.push('\n');
        self.lines += 1;
        if trim_body(&mut self.body) {
            self.truncated = true;
            self.rewrite_file();
            return;
        }
        if let Some(w) = self.writer.as_mut() {
            let _ = writeln!(w, "{line}");
            let _ = w.flush();
        }
    }

    fn rewrite_file(&mut self) {
        if self.path.as_os_str().is_empty() {
            return;
        }
        self.writer = None;
        if let Ok(f) = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.path)
        {
            let mut w = BufWriter::new(f);
            let _ = w.write_all(self.body.as_bytes());
            let _ = w.flush();
            self.writer = Some(w);
        }
    }

    fn flush(&mut self) {
        if let Some(w) = self.writer.as_mut() {
            let _ = w.flush();
        }
    }
}

impl Drop for ClockLog {
    fn drop(&mut self) {
        self.flush();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SessionEvent {
    Continue,
    RaceEnded,
    NewSession,
}

#[derive(Default)]
struct SessionGate {
    track: String,
    laps: i32,
    time_ms: i32,
    saw_race: bool,
}

impl SessionGate {
    fn update(
        &mut self,
        track: &str,
        laps: i32,
        _len: i32,
        time_ms: i32,
        current_lap: i32,
        on_track: i32,
    ) -> SessionEvent {
        let empty = laps <= 0 && time_ms <= 0 && current_lap <= 0 && on_track == 0;
        if empty {
            if self.saw_race {
                self.saw_race = false;
                self.track.clear();
                self.time_ms = 0;
                self.laps = 0;
                return SessionEvent::RaceEnded;
            }
            return SessionEvent::Continue;
        }
        let track = track.trim();
        let racing = current_lap > 0 || time_ms > 8000 || (on_track != 0 && time_ms > 3000);
        let new_session = self.saw_race
            && ((!self.track.is_empty() && !track.is_empty() && self.track != track)
                || (self.laps > 0 && laps > 0 && self.laps != laps)
                || (self.time_ms >= 20_000 && time_ms < 2_500));
        self.track = if track.is_empty() {
            self.track.clone()
        } else {
            track.to_string()
        };
        self.laps = laps;
        self.time_ms = time_ms;
        if new_session {
            self.saw_race = racing;
            return SessionEvent::NewSession;
        }
        if racing {
            self.saw_race = true;
        }
        SessionEvent::Continue
    }
}

pub struct FeedbackLog {
    pub path: PathBuf,
    pub name: String,
    pub body: String,
    pub truncated: bool,
    pub track: Option<String>,
}

pub fn feedback_log() -> Option<FeedbackLog> {
    let mut g = live();
    let log = g.as_mut()?;
    log.flush();
    let mid_race = log.gate.saw_race && raw_has_race(&log.body);
    if mid_race {
        let out = snapshot_log(log.body.clone(), log.path.clone(), log.truncated);
        drop(g);
        return out;
    }
    let last_path = log.last_path.clone();
    let last_truncated = log.last_truncated;
    let body = log.body.clone();
    let path = log.path.clone();
    let truncated = log.truncated;
    drop(g);
    if let Some(raw) = read_closed_log(&last_path) {
        return snapshot_log(raw, last_path, last_truncated);
    }
    if let Some(found) = pick_feedback_log() {
        if found != path {
            if let Some(raw) = read_closed_log(&found) {
                return snapshot_log(raw, found, false);
            }
        }
    }
    snapshot_log(body, path, truncated)
}

pub fn feedback_log_label() -> String {
    match feedback_choice() {
        None => "No race log yet — finish a moto first.".into(),
        Some((name, kb)) => format!("Will send {name} ({kb} KB)"),
    }
}

pub fn has_feedback_log() -> bool {
    feedback_choice().is_some()
}

fn feedback_choice() -> Option<(&'static str, u64)> {
    let g = live();
    let log = g.as_ref()?;
    if log.gate.saw_race && raw_has_race(&log.body) {
        return Some(("this race", (log.body.len() as u64 / 1024).max(1)));
    }
    if log.last_ready {
        return Some(("last race", (log.last_len as u64 / 1024).max(1)));
    }
    if raw_has_race(&log.body) {
        return Some(("last race", (log.body.len() as u64 / 1024).max(1)));
    }
    None
}

fn pick_feedback_log() -> Option<PathBuf> {
    for dir in log_dirs() {
        for name in [LAST_RACE, RACE_LOG] {
            let path = dir.join(name);
            if log_has_race(&path) {
                return Some(path);
            }
        }
    }
    None
}

fn file_len(path: &Path) -> u64 {
    fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

fn log_has_race(path: &Path) -> bool {
    let Ok(raw) = fs::read_to_string(path) else {
        return false;
    };
    raw_has_race(&raw)
}

fn snapshot_log(raw: String, path: PathBuf, truncated: bool) -> Option<FeedbackLog> {
    if !raw_has_race(&raw) {
        return None;
    }
    let track = peek_track(&raw);
    let send = path.parent().map(|dir| dir.join(RACE_SEND));
    if let Some(send) = send.as_ref() {
        let _ = fs::write(send, raw.as_bytes());
    }
    Some(FeedbackLog {
        path: send.filter(|p| p.is_file()).unwrap_or_else(|| path.clone()),
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| RACE_LOG.to_string()),
        body: raw,
        truncated,
        track,
    })
}

fn read_closed_log(path: &Path) -> Option<String> {
    if path.as_os_str().is_empty() || !path.is_file() {
        return None;
    }
    let (raw, _) = read_tail(path);
    if raw_has_race(&raw) {
        Some(raw)
    } else {
        None
    }
}

fn read_tail(path: &Path) -> (String, bool) {
    let Ok(meta) = fs::metadata(path) else {
        return (String::new(), false);
    };
    let Ok(mut f) = File::open(path) else {
        return (String::new(), false);
    };
    if meta.len() <= MAX_LOG as u64 {
        let mut data = String::new();
        let _ = f.read_to_string(&mut data);
        return (data, false);
    }
    let _ = f.seek(SeekFrom::Start(meta.len() - MAX_LOG as u64));
    let mut buf = Vec::with_capacity(MAX_LOG);
    let _ = f.read_to_end(&mut buf);
    let start = buf.iter().position(|&b| b == b'\n').map(|i| i + 1).unwrap_or(0);
    (
        String::from_utf8_lossy(&buf[start..]).into_owned(),
        true,
    )
}

fn trim_body(body: &mut String) -> bool {
    if body.len() <= MAX_LOG {
        return false;
    }
    let start = body.len() - MAX_LOG;
    let start = body[start..].find('\n').map(|i| start + i + 1).unwrap_or(start);
    body.replace_range(0..start, "");
    true
}

fn raw_has_race(raw: &str) -> bool {
    raw.lines().any(|l| l.contains("\"cur\":") && l.contains("\"t\":"))
}

fn peek_track(raw: &str) -> Option<String> {
    for line in raw.lines().rev() {
        if let Some(track) = json_field(line, "track") {
            if !track.is_empty() && track != "null" {
                return Some(track);
            }
        }
    }
    None
}

fn json_field(line: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\":\"");
    let i = line.find(&needle)?;
    let rest = &line[i + needle.len()..];
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                if let Some(n) = chars.next() {
                    out.push(n);
                }
            }
            '"' => break,
            c => out.push(c),
        }
    }
    Some(out)
}

fn write_boot(msg: &str) {
    for dir in log_dirs() {
        let _ = fs::create_dir_all(&dir);
        let _ = fs::write(dir.join("boot.txt"), msg.as_bytes());
    }
}

fn remove_old_logs(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let stale = name == "latest.txt"
            || name == "last-race.txt"
            || name == "feedback-attach.jsonl"
            || name == "boot-from-agent.txt"
            || (name.starts_with("clock-") && name.ends_with(".jsonl"));
        if stale {
            let _ = fs::remove_file(entry.path());
        }
    }
}

fn log_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        dirs.push(PathBuf::from(local).join("Holeshot HUD").join("logs"));
    }
    dirs.push(std::env::temp_dir().join("Holeshot HUD").join("logs"));
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            dirs.push(parent.join("logs"));
        }
    }
    dirs
}

fn json_escape(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if c.is_control() => {}
            c => o.push(c),
        }
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_ticks_do_not_end_a_race() {
        let mut g = SessionGate::default();
        assert_eq!(g.update("", 0, 0, 0, 0, 0), SessionEvent::Continue);
    }

    #[test]
    fn leaving_a_session_archives_the_race() {
        let mut g = SessionGate::default();
        assert_eq!(g.update("Glen", 4, 0, 30_000, 2, 1), SessionEvent::Continue);
        assert_eq!(g.update("", 0, 0, 0, 0, 0), SessionEvent::RaceEnded);
    }

    #[test]
    fn track_change_starts_a_new_session() {
        let mut g = SessionGate::default();
        g.update("Glen", 4, 0, 40_000, 3, 1);
        assert_eq!(g.update("Hangtown", 4, 0, 1_000, 1, 1), SessionEvent::NewSession);
    }

    #[test]
    fn lap_count_change_starts_a_new_session() {
        let mut g = SessionGate::default();
        g.update("Glen", 4, 0, 40_000, 3, 1);
        assert_eq!(g.update("Glen", 6, 0, 1_000, 1, 1), SessionEvent::NewSession);
    }

    #[test]
    fn clock_reset_starts_a_new_session() {
        let mut g = SessionGate::default();
        g.update("Glen", 0, 8, 90_000, 2, 1);
        assert_eq!(g.update("Glen", 0, 8, 800, 0, 1), SessionEvent::NewSession);
    }

    #[test]
    fn same_moto_keeps_logging() {
        let mut g = SessionGate::default();
        g.update("Glen", 4, 0, 10_000, 1, 1);
        assert_eq!(g.update("Glen", 4, 0, 20_000, 2, 1), SessionEvent::Continue);
    }

    #[test]
    fn peek_track_reads_latest_name() {
        let raw = "{\"v\":1}\n{\"track\":\"Glen Helen\",\"cur\":2}\n{\"track\":\"Hangtown\",\"cur\":1}\n";
        assert_eq!(peek_track(raw).as_deref(), Some("Hangtown"));
    }

    #[test]
    fn header_only_clock_file_is_not_a_race_log() {
        let raw = "{\"v\":1,\"file\":\"C:\\\\Users\\\\troye\\\\AppData\\\\Local\\\\Holeshot HUD\\\\logs\\\\clock-20260818-204810.jsonl\"}\n";
        assert!(!raw_has_race(raw));
    }

    #[test]
    fn clock_sample_line_counts_as_a_race_log() {
        let raw = "{\"v\":1,\"file\":\"x\"}\n{\"t\":1.2,\"seq\":3,\"track\":\"Glen\",\"cur\":2,\"time\":8000}\n";
        assert!(raw_has_race(raw));
    }

    #[test]
    fn snapshot_uses_in_memory_samples_without_reading_the_live_file() {
        let dir = std::env::temp_dir().join(format!(
            "holeshot-hud-snapshot-{}",
            std::process::id()
        ));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("race.jsonl");
        let raw = "{\"v\":1}\n{\"t\":1.2,\"seq\":3,\"track\":\"Glen\",\"cur\":2,\"time\":8000}\n".into();
        let log = snapshot_log(raw, path, false).expect("samples");
        assert_eq!(log.track.as_deref(), Some("Glen"));
        assert!(log.body.contains("\"cur\":2"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn trim_body_keeps_only_a_small_tail() {
        let mut body = String::from("{\"v\":1}\n");
        while body.len() <= MAX_LOG {
            body.push_str("{\"t\":1.0,\"cur\":1}\n");
        }
        assert!(body.len() > MAX_LOG);
        assert!(trim_body(&mut body));
        assert!(body.len() <= MAX_LOG);
        assert!(raw_has_race(&body));
    }
}
