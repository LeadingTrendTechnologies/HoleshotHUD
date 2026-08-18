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
        self.flush();
        self.open_new();
    }

    pub fn tick(&mut self, snap: &Snapshot) {
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
            self.rotate();
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
        self.flush();
    }
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
