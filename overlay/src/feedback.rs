use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{HANDLE, HGLOBAL, HWND};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

use crate::record::{self, FeedbackLog};
use crate::update;

const FEEDBACK_URL: &str = "https://holeshot-hud.vercel.app/api/feedback";
const MAX_MESSAGE: usize = 1500;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Rate,
    Bug,
}

#[derive(Clone)]
pub enum Status {
    Idle,
    Sending,
    Sent,
    Error(String),
}

#[derive(Clone)]
pub struct Form {
    pub kind: Kind,
    pub rating: u8,
    pub message: String,
    pub cursor: usize,
    pub attach_log: bool,
    pub focused: bool,
    pub caret_at: Instant,
    pub status: Status,
    pub text_rect: (f32, f32, f32, f32),
}

impl Default for Form {
    fn default() -> Self {
        Self {
            kind: Kind::Rate,
            rating: 0,
            message: String::new(),
            cursor: 0,
            attach_log: true,
            focused: false,
            caret_at: Instant::now(),
            status: Status::Idle,
            text_rect: (0.0, 0.0, 0.0, 0.0),
        }
    }
}

static FORM: OnceLock<Mutex<Form>> = OnceLock::new();

fn lock() -> std::sync::MutexGuard<'static, Form> {
    FORM.get_or_init(|| Mutex::new(Form::default()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

pub fn snapshot() -> Form {
    lock().clone()
}

pub fn set_kind(kind: Kind) {
    let mut f = lock();
    f.kind = kind;
    f.status = Status::Idle;
}

pub fn set_rating(rating: u8) {
    let mut f = lock();
    f.rating = rating.clamp(1, 5);
    f.status = Status::Idle;
}

pub fn toggle_attach() {
    let mut f = lock();
    f.attach_log = !f.attach_log;
}

pub fn set_focus(on: bool) {
    let mut f = lock();
    f.focused = on;
    if on {
        f.caret_at = Instant::now();
    }
}

pub fn set_text_rect(x: f32, y: f32, w: f32, h: f32) {
    lock().text_rect = (x, y, w, h);
}

pub fn click_text(px: f32, py: f32) {
    let mut f = lock();
    f.focused = true;
    f.caret_at = Instant::now();
    let (x, y, w, _h) = f.text_rect;
    let col = ((px - x - 12.0).max(0.0) / 7.2) as usize;
    let row = ((py - y - 10.0).max(0.0) / 16.0) as usize;
    let wrap = wrap_cols(w);
    f.cursor = cursor_from_row_col(&f.message, row, col, wrap);
}

pub fn on_char(ch: char) -> bool {
    let mut f = lock();
    if !f.focused {
        return false;
    }
    if ch == '\r' || ch == '\n' {
        insert(&mut f, '\n');
        return true;
    }
    if ch.is_control() {
        return true;
    }
    insert(&mut f, ch);
    true
}

pub fn on_key(vk: u16, ctrl: bool) -> bool {
    let mut f = lock();
    if !f.focused {
        return false;
    }
    f.caret_at = Instant::now();
    match vk {
        0x08 => {
            if f.cursor > 0 {
                let prev = prev_char(&f.message, f.cursor);
                let cur = f.cursor;
                f.message.replace_range(prev..cur, "");
                f.cursor = prev;
            }
            true
        }
        0x2E => {
            if f.cursor < f.message.len() {
                let cur = f.cursor;
                let next = next_char(&f.message, cur);
                f.message.replace_range(cur..next, "");
            }
            true
        }
        0x25 => {
            f.cursor = prev_char(&f.message, f.cursor);
            true
        }
        0x27 => {
            f.cursor = next_char(&f.message, f.cursor);
            true
        }
        0x24 => {
            f.cursor = 0;
            true
        }
        0x23 => {
            f.cursor = f.message.len();
            true
        }
        0x1B => {
            f.focused = false;
            true
        }
        0x56 if ctrl => {
            drop(f);
            if let Some(text) = clipboard_text() {
                let mut f = lock();
                for ch in text.chars() {
                    if ch == '\r' {
                        continue;
                    }
                    insert(&mut f, ch);
                }
            }
            true
        }
        _ => true,
    }
}

pub fn caret_on() -> bool {
    let f = lock();
    f.focused && f.caret_at.elapsed().as_millis() % 1000 < 500
}

pub fn send() {
    let (kind, rating, message, attach) = {
        let mut f = lock();
        if matches!(f.status, Status::Sending) {
            return;
        }
        match f.kind {
            Kind::Rate if f.rating == 0 => {
                f.status = Status::Error("Pick a star rating first.".into());
                return;
            }
            Kind::Bug if f.message.trim().is_empty() => {
                f.status = Status::Error("Describe the bug first.".into());
                return;
            }
            _ => {}
        }
        f.status = Status::Sending;
        f.focused = false;
        (f.kind, f.rating, f.message.trim().to_string(), f.attach_log && f.kind == Kind::Bug)
    };
    std::thread::spawn(move || {
        let log = if attach { record::feedback_log() } else { None };
        let result = submit(kind, rating, &message, log.as_ref());
        let mut f = lock();
        f.status = result;
        if matches!(f.status, Status::Sent) {
            f.message.clear();
            f.cursor = 0;
            f.rating = 0;
        }
    });
}

pub fn log_label() -> String {
    record::feedback_log_label()
}

pub fn has_log() -> bool {
    record::has_feedback_log()
}

fn insert(f: &mut Form, ch: char) {
    if f.message.chars().count() >= MAX_MESSAGE {
        return;
    }
    f.message.insert(f.cursor, ch);
    f.cursor += ch.len_utf8();
    f.caret_at = Instant::now();
    f.status = Status::Idle;
}

fn wrap_cols(w: f32) -> usize {
    ((w - 24.0) / 7.2).max(8.0) as usize
}

fn cursor_from_row_col(s: &str, row: usize, col: usize, wrap: usize) -> usize {
    let mut r = 0usize;
    let mut c = 0usize;
    for (i, ch) in s.char_indices() {
        if r == row && c >= col {
            return i;
        }
        if ch == '\n' {
            r += 1;
            c = 0;
            if r > row {
                return i;
            }
        } else {
            c += 1;
            if c >= wrap {
                r += 1;
                c = 0;
                if r > row {
                    return i + ch.len_utf8();
                }
            }
        }
    }
    s.len()
}

fn prev_char(s: &str, i: usize) -> usize {
    if i == 0 {
        return 0;
    }
    s.char_indices()
        .rev()
        .find(|(idx, _)| *idx < i)
        .map(|(idx, _)| idx)
        .unwrap_or(0)
}

fn next_char(s: &str, i: usize) -> usize {
    s.char_indices()
        .find(|(idx, _)| *idx > i)
        .map(|(idx, _)| idx)
        .unwrap_or(s.len())
}

fn submit(kind: Kind, rating: u8, message: &str, log: Option<&FeedbackLog>) -> Status {
    match post(kind, rating, message, log) {
        Ok(()) => Status::Sent,
        Err(e) => {
            copy_report(kind, rating, message, log);
            Status::Error(send_error(&e))
        }
    }
}

fn send_error(e: &str) -> String {
    let e = e.to_ascii_lowercase();
    if e.contains("503") {
        "Couldn't send. Add FEEDBACK_GITHUB_TOKEN on Vercel.".into()
    } else if e.contains("404") || e.contains("not found") {
        "Couldn't send. Deploy /api/feedback to Vercel.".into()
    } else if e.contains("could not") || e.contains("timed out") || e.contains("dns") {
        "Couldn't send. No connection to the server.".into()
    } else {
        "Couldn't send. Report and log file copied.".into()
    }
}

fn copy_report(kind: Kind, rating: u8, message: &str, log: Option<&FeedbackLog>) {
    let text = plain_report(kind, rating, message, log);
    let path = log.map(|l| l.path.as_path());
    let _ = set_clipboard(&text, path);
}

fn post(kind: Kind, rating: u8, message: &str, log: Option<&FeedbackLog>) -> Result<(), String> {
    let url = feedback_url();
    let body = payload_json(kind, rating, message, log);
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(60))
        .user_agent("mxbo-overlay")
        .build();
    let resp = agent
        .post(&url)
        .set("Content-Type", "application/json")
        .send_string(&body)
        .map_err(|e| format!("{e}"))?;
    if resp.status() >= 200 && resp.status() < 300 {
        Ok(())
    } else {
        Err(format!("HTTP {}", resp.status()))
    }
}

fn feedback_url() -> String {
    if let Ok(url) = std::env::var("HOLESHOT_FEEDBACK_URL") {
        let url = url.trim();
        if !url.is_empty() {
            return url.to_string();
        }
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        if let Ok(text) = std::fs::read_to_string(
            std::path::PathBuf::from(local)
                .join("Holeshot HUD")
                .join("feedback-url.txt"),
        ) {
            let url = text.trim();
            if !url.is_empty() {
                return url.to_string();
            }
        }
    }
    FEEDBACK_URL.to_string()
}

fn payload_json(kind: Kind, rating: u8, message: &str, log: Option<&FeedbackLog>) -> String {
    let kind_s = match kind {
        Kind::Rate => "rating",
        Kind::Bug => "bug",
    };
    let rating_s = if rating == 0 {
        "null".into()
    } else {
        rating.to_string()
    };
    let mut s = format!(
        "{{\"kind\":\"{kind_s}\",\"rating\":{rating_s},\"message\":\"{}\",\"version\":\"{}\",\"os\":\"{} {}\"",
        json_escape(message),
        json_escape(update::current_version()),
        json_escape(std::env::consts::OS),
        json_escape(std::env::consts::ARCH),
    );
    if let Some(log) = log {
        s.push_str(&format!(
            ",\"log_name\":\"{}\",\"log_truncated\":{},\"log\":\"{}\"",
            json_escape(&log.name),
            if log.truncated { "true" } else { "false" },
            json_escape(&log.body),
        ));
        if let Some(track) = log.track.as_deref() {
            s.push_str(&format!(",\"track\":\"{}\"", json_escape(track)));
        }
    }
    s.push('}');
    s
}

fn plain_report(kind: Kind, rating: u8, message: &str, log: Option<&FeedbackLog>) -> String {
    let mut body = String::new();
    body.push_str(&format!(
        "Holeshot HUD {}\n{} {}\n\n",
        update::current_version(),
        std::env::consts::OS,
        std::env::consts::ARCH
    ));
    match kind {
        Kind::Rate => body.push_str("Kind: rating\n"),
        Kind::Bug => body.push_str("Kind: bug\n"),
    }
    if rating > 0 {
        body.push_str(&format!("Rating: {rating}/5\n"));
    }
    if !message.is_empty() {
        body.push('\n');
        body.push_str(message);
        body.push('\n');
    }
    if let Some(log) = log {
        if let Some(track) = log.track.as_deref() {
            body.push_str(&format!("\nTrack: {track}\n"));
        }
        body.push_str("\n--- last race log ---\n");
        body.push_str(&log_excerpt(&log.body, 12_000));
    }
    body
}

fn log_excerpt(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let start = s.len() - max;
    let start = s[start..].find('\n').map(|i| start + i + 1).unwrap_or(start);
    format!("… truncated …\n{}", &s[start..])
}

fn json_escape(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if c.is_control() => {}
            c => o.push(c),
        }
    }
    o
}

fn clipboard_text() -> Option<String> {
    unsafe {
        OpenClipboard(HWND::default()).ok()?;
        let handle = GetClipboardData(13).ok()?;
        let hg = HGLOBAL(handle.0);
        let ptr = GlobalLock(hg) as *const u16;
        let text = if ptr.is_null() {
            None
        } else {
            let mut len = 0usize;
            while *ptr.add(len) != 0 {
                len += 1;
                if len > 50_000 {
                    break;
                }
            }
            let slice = std::slice::from_raw_parts(ptr, len);
            Some(String::from_utf16_lossy(slice))
        };
        let _ = GlobalUnlock(hg);
        let _ = CloseClipboard();
        text
    }
}

fn set_clipboard(text: &str, file: Option<&Path>) -> Result<(), ()> {
    let mut wide: Vec<u16> = text.encode_utf16().collect();
    wide.push(0);
    let bytes = wide.len() * 2;
    unsafe {
        OpenClipboard(HWND::default()).map_err(|_| ())?;
        let _ = EmptyClipboard();
        let hg = GlobalAlloc(GMEM_MOVEABLE, bytes).map_err(|_| {
            let _ = CloseClipboard();
            ()
        })?;
        let ptr = GlobalLock(hg) as *mut u16;
        if ptr.is_null() {
            let _ = CloseClipboard();
            return Err(());
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr, wide.len());
        let _ = GlobalUnlock(hg);
        let _ = SetClipboardData(13, HANDLE(hg.0));
        if let Some(path) = file {
            let _ = set_clipboard_file(path);
        }
        let _ = CloseClipboard();
    }
    Ok(())
}

fn set_clipboard_file(path: &Path) -> Result<(), ()> {
    let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let mut s = abs.to_string_lossy().into_owned();
    if let Some(stripped) = s.strip_prefix(r"\\?\") {
        s = stripped.to_string();
    }
    let mut wide: Vec<u16> = s.encode_utf16().collect();
    wide.push(0);
    wide.push(0);
    let header = 20usize;
    let bytes = header + wide.len() * 2;
    unsafe {
        let hg = GlobalAlloc(GMEM_MOVEABLE, bytes).map_err(|_| ())?;
        let ptr = GlobalLock(hg) as *mut u8;
        if ptr.is_null() {
            return Err(());
        }
        std::ptr::write(ptr as *mut u32, 20);
        std::ptr::write(ptr.add(4) as *mut i32, 0);
        std::ptr::write(ptr.add(8) as *mut i32, 0);
        std::ptr::write(ptr.add(12) as *mut i32, 0);
        std::ptr::write(ptr.add(16) as *mut i32, 1);
        std::ptr::copy_nonoverlapping(wide.as_ptr() as *const u8, ptr.add(20), wide.len() * 2);
        let _ = GlobalUnlock(hg);
        let _ = SetClipboardData(15, HANDLE(hg.0));
    }
    Ok(())
}
