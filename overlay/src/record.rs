use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use mxbo_hud::snapshot::{cstr, Snapshot};
use mxbo_hud::{clock_sample, ClockSample};

pub struct ClockLog {
    writer: Option<BufWriter<File>>,
    path: PathBuf,
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
            started: Instant::now(),
            last_write: Instant::now() - std::time::Duration::from_secs(10),
            last_key: None,
            lines: 0,
            gate: SessionGate::default(),
        };
        log.open_new();
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
        if self.gate.saw_race {
            self.archive_last_race();
            self.gate.saw_race = false;
        }
        self.flush();
        self.open_new();
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
                self.archive_last_race();
                self.flush();
                self.open_new();
                return;
            }
            SessionEvent::NewSession => {
                self.archive_last_race();
                self.flush();
                self.open_new();
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
        if self.lines >= 80_000 {
            self.flush();
            self.open_new();
        }
    }

    fn archive_last_race(&mut self) {
        self.flush();
        if self.lines < 1 || self.path.as_os_str().is_empty() || !self.path.is_file() {
            return;
        }
        let Some(dir) = self.path.parent() else {
            return;
        };
        let dest = dir.join("last-race.jsonl");
        if dest == self.path {
            return;
        }
        if fs::copy(&self.path, &dest).is_ok() {
            let _ = fs::write(dir.join("last-race.txt"), dest.to_string_lossy().as_bytes());
        }
    }

    fn open_new(&mut self) {
        self.writer = None;
        self.lines = 0;
        self.last_key = None;
        self.started = Instant::now();
        let stamp = chrono_stamp();
        let name = format!("clock-{stamp}.jsonl");
        let mut errors = String::new();
        for dir in log_dirs() {
            if let Err(e) = fs::create_dir_all(&dir) {
                errors.push_str(&format!("mkdir {}: {e}\n", dir.display()));
                continue;
            }
            let path = dir.join(&name);
            match OpenOptions::new().create(true).append(true).open(&path) {
                Ok(f) => {
                    let mut w = BufWriter::new(f);
                    let _ = writeln!(
                        w,
                        "{{\"v\":1,\"file\":\"{}\"}}",
                        json_escape(&path.display().to_string())
                    );
                    let _ = w.flush();
                    let _ = fs::write(dir.join("latest.txt"), path.to_string_lossy().as_bytes());
                    self.path = path;
                    self.writer = Some(w);
                    write_boot(&format!("ok {}", self.path.display()));
                    return;
                }
                Err(e) => errors.push_str(&format!("open {}: {e}\n", path.display())),
            }
        }
        write_boot(&format!("failed\n{errors}"));
    }

    fn write_line(&mut self, snap: &Snapshot, s: &ClockSample) {
        let Some(w) = self.writer.as_mut() else {
            return;
        };
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
        if writeln!(w, "{line}").is_ok() {
            self.lines += 1;
            let _ = w.flush();
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
        if self.gate.saw_race {
            self.archive_last_race();
        }
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
    let path = pick_feedback_log()?;
    let dest = path.parent().unwrap_or(path.as_path()).join("feedback-attach.jsonl");
    let src = if fs::copy(&path, &dest).is_ok() { dest } else { path.clone() };
    let raw = fs::read_to_string(&src).ok()?;
    if raw.trim().is_empty() {
        return None;
    }
    let track = peek_track(&raw);
    const MAX: usize = 400_000;
    let (body, truncated) = if raw.len() <= MAX {
        (raw, false)
    } else {
        let start = raw.len() - MAX;
        let start = raw[start..].find('\n').map(|i| start + i + 1).unwrap_or(start);
        (format!("… truncated …\n{}", &raw[start..]), true)
    };
    Some(FeedbackLog {
        path: src,
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "last-race.jsonl".into()),
        body,
        truncated,
        track,
    })
}

pub fn feedback_log_label() -> String {
    match pick_feedback_log() {
        None => "No race log yet — finish a moto first.".into(),
        Some(path) => {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "last-race.jsonl".into());
            let kb = (file_len(&path) / 1024).max(1);
            format!("Will send {name} ({kb} KB)")
        }
    }
}

pub fn has_feedback_log() -> bool {
    pick_feedback_log().is_some_and(|p| file_len(&p) > 40)
}

fn pick_feedback_log() -> Option<PathBuf> {
    for dir in log_dirs() {
        let current = latest_in(&dir);
        let last = dir.join("last-race.jsonl");
        let current_live = current.as_ref().is_some_and(|p| log_has_race(p));
        if current_live {
            return current;
        }
        if last.is_file() && file_len(&last) > 40 {
            return Some(last);
        }
        if let Some(p) = newest_clock(&dir) {
            if file_len(&p) > 40 {
                return Some(p);
            }
        }
        if let Some(p) = current {
            if file_len(&p) > 0 {
                return Some(p);
            }
        }
    }
    None
}

fn newest_clock(dir: &Path) -> Option<PathBuf> {
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension().and_then(|e| e.to_str()) == Some("jsonl")
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("clock-") || n == "last-race.jsonl")
        })
        .filter_map(|p| {
            let t = fs::metadata(&p).and_then(|m| m.modified()).ok()?;
            Some((t, p))
        })
        .collect();
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files.pop().map(|(_, p)| p)
}

fn file_len(path: &Path) -> u64 {
    fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

fn latest_in(dir: &Path) -> Option<PathBuf> {
    let text = fs::read_to_string(dir.join("latest.txt")).ok()?;
    let path = PathBuf::from(text.trim());
    path.is_file().then_some(path)
}

fn log_has_race(path: &Path) -> bool {
    let Ok(raw) = fs::read_to_string(path) else {
        return false;
    };
    raw.lines().filter(|l| l.contains("\"cur\":") || l.contains("\"time\":")).count() >= 1
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

fn chrono_stamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let secs = now as i64;
    let days = secs / 86400;
    let tod = secs % 86400;
    let (y, m, d) = civil_from_days(days);
    let hh = tod / 3600;
    let mm = (tod % 3600) / 60;
    let ss = tod % 60;
    format!("{y:04}{m:02}{d:02}-{hh:02}{mm:02}{ss:02}")
}

fn civil_from_days(z: i64) -> (i32, i32, i32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as i32, d as i32)
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
}
