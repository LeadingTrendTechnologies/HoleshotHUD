use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{HANDLE, HGLOBAL, HWND};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

use crate::record::{self, FeedbackLog};
use crate::update;
use crate::util::{json_array_slice, json_bool, json_escape, json_string};

const FEEDBACK_URL: &str = "https://holeshot-hud.vercel.app/api/feedback";
const MAX_MESSAGE: usize = 1500;
const MAX_TICKETS: usize = 20;
const POLL_EVERY: Duration = Duration::from_secs(60);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Rate,
    Bug,
    Feature,
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

struct CaretLayout {
    x0: f32,
    y0: f32,
    line_h: f32,
    rows: Vec<Vec<(usize, f32)>>,
}

static FORM: OnceLock<Mutex<Form>> = OnceLock::new();
static COMPOSE: OnceLock<Mutex<Compose>> = OnceLock::new();
static CARET: OnceLock<Mutex<CaretLayout>> = OnceLock::new();
static TICKETS: OnceLock<Mutex<Vec<Ticket>>> = OnceLock::new();
static POLL_AT: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();
static POLLING: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatLine {
    pub from_dev: bool,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplyView {
    pub id: String,
    pub kind_label: &'static str,
    pub lines: Vec<ChatLine>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Msg {
    from_dev: bool,
    text: String,
    at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Ticket {
    id: String,
    kind: String,
    summary: String,
    sent_at: String,
    reply: String,
    replied_at: String,
    thread: Vec<Msg>,
    seen_reply: bool,
}

#[derive(Clone)]
pub struct Compose {
    pub id: String,
    pub message: String,
    pub cursor: usize,
    pub focused: bool,
    pub caret_at: Instant,
    pub status: Status,
    pub text_rect: (f32, f32, f32, f32),
}

impl Default for Compose {
    fn default() -> Self {
        Self {
            id: String::new(),
            message: String::new(),
            cursor: 0,
            focused: false,
            caret_at: Instant::now(),
            status: Status::Idle,
            text_rect: (0.0, 0.0, 0.0, 0.0),
        }
    }
}

fn caret_lock() -> std::sync::MutexGuard<'static, CaretLayout> {
    CARET
        .get_or_init(|| {
            Mutex::new(CaretLayout {
                x0: 0.0,
                y0: 0.0,
                line_h: 16.0,
                rows: Vec::new(),
            })
        })
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

pub fn set_caret_layout(x0: f32, y0: f32, line_h: f32, rows: Vec<Vec<(usize, f32)>>) {
    *caret_lock() = CaretLayout {
        x0,
        y0,
        line_h,
        rows,
    };
}

fn lock() -> std::sync::MutexGuard<'static, Form> {
    FORM.get_or_init(|| Mutex::new(Form::default()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn tickets_lock() -> std::sync::MutexGuard<'static, Vec<Ticket>> {
    TICKETS
        .get_or_init(|| Mutex::new(load_tickets()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn compose_lock() -> std::sync::MutexGuard<'static, Compose> {
    COMPOSE
        .get_or_init(|| Mutex::new(Compose::default()))
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

pub fn is_focused() -> bool {
    lock().focused || compose_lock().focused
}

pub fn set_focus(on: bool) {
    {
        let mut f = lock();
        f.focused = on;
        if on {
            f.caret_at = Instant::now();
        }
    }
    if on {
        compose_lock().focused = false;
    }
}

pub fn compose_snapshot() -> Compose {
    compose_lock().clone()
}

pub fn prepare_compose(id: &str) {
    let mut c = compose_lock();
    if c.id != id {
        *c = Compose {
            id: id.into(),
            caret_at: Instant::now(),
            ..Compose::default()
        };
        c.id = id.into();
    }
}

pub fn set_compose_focus(on: bool) {
    {
        let mut c = compose_lock();
        c.focused = on;
        if on {
            c.caret_at = Instant::now();
        }
    }
    if on {
        lock().focused = false;
    }
}

pub fn set_compose_rect(x: f32, y: f32, w: f32, h: f32) {
    compose_lock().text_rect = (x, y, w, h);
}

pub fn click_compose(px: f32, py: f32) {
    set_compose_focus(true);
    let mut c = compose_lock();
    c.caret_at = Instant::now();
    let lay = caret_lock();
    if lay.rows.is_empty() {
        c.cursor = c.message.len();
        return;
    }
    let row = ((py - lay.y0).max(0.0) / lay.line_h.max(1.0)) as usize;
    let row = row.min(lay.rows.len() - 1);
    let x = px - lay.x0;
    let stops = &lay.rows[row];
    let mut best = stops.first().map(|s| s.0).unwrap_or(0);
    for pair in stops.windows(2) {
        let mid = (pair[0].1 + pair[1].1) * 0.5;
        if x >= mid {
            best = pair[1].0;
        }
    }
    c.cursor = best.min(c.message.len());
}

pub fn set_text_rect(x: f32, y: f32, w: f32, h: f32) {
    lock().text_rect = (x, y, w, h);
}

pub fn click_text(px: f32, py: f32) {
    let mut f = lock();
    f.focused = true;
    f.caret_at = Instant::now();
    let lay = caret_lock();
    if lay.rows.is_empty() {
        f.cursor = f.message.len();
        return;
    }
    let row = ((py - lay.y0).max(0.0) / lay.line_h.max(1.0)) as usize;
    let row = row.min(lay.rows.len() - 1);
    let x = px - lay.x0;
    let stops = &lay.rows[row];
    let mut best = stops.first().map(|s| s.0).unwrap_or(0);
    for pair in stops.windows(2) {
        let mid = (pair[0].1 + pair[1].1) * 0.5;
        if x >= mid {
            best = pair[1].0;
        }
    }
    f.cursor = best.min(f.message.len());
}

pub fn on_char(ch: char) -> bool {
    if compose_lock().focused {
        return compose_char(ch);
    }
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
    if compose_lock().focused {
        return compose_key(vk, ctrl);
    }
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
    let form = lock();
    if form.focused && form.caret_at.elapsed().as_millis() % 1000 < 500 {
        return true;
    }
    drop(form);
    let c = compose_lock();
    c.focused && c.caret_at.elapsed().as_millis() % 1000 < 500
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
            Kind::Feature if f.message.trim().is_empty() => {
                f.status = Status::Error("Describe the feature first.".into());
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
        let result = submit(kind, rating, &message, log.as_ref(), attach);
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

fn insert_compose(c: &mut Compose, ch: char) {
    if c.message.chars().count() >= MAX_MESSAGE {
        return;
    }
    c.message.insert(c.cursor, ch);
    c.cursor += ch.len_utf8();
    c.caret_at = Instant::now();
    c.status = Status::Idle;
}

fn compose_char(ch: char) -> bool {
    let mut c = compose_lock();
    if !c.focused {
        return false;
    }
    if ch == '\r' || ch == '\n' {
        insert_compose(&mut c, '\n');
        return true;
    }
    if ch.is_control() {
        return true;
    }
    insert_compose(&mut c, ch);
    true
}

fn compose_key(vk: u16, ctrl: bool) -> bool {
    let mut c = compose_lock();
    if !c.focused {
        return false;
    }
    c.caret_at = Instant::now();
    match vk {
        0x08 => {
            if c.cursor > 0 {
                let prev = prev_char(&c.message, c.cursor);
                let cur = c.cursor;
                c.message.replace_range(prev..cur, "");
                c.cursor = prev;
            }
            true
        }
        0x2E => {
            if c.cursor < c.message.len() {
                let cur = c.cursor;
                let next = next_char(&c.message, cur);
                c.message.replace_range(cur..next, "");
            }
            true
        }
        0x25 => {
            c.cursor = prev_char(&c.message, c.cursor);
            true
        }
        0x27 => {
            c.cursor = next_char(&c.message, c.cursor);
            true
        }
        0x24 => {
            c.cursor = 0;
            true
        }
        0x23 => {
            c.cursor = c.message.len();
            true
        }
        0x1B => {
            c.focused = false;
            true
        }
        0x56 if ctrl => {
            drop(c);
            if let Some(text) = clipboard_text() {
                let mut c = compose_lock();
                for ch in text.chars() {
                    if ch == '\r' {
                        continue;
                    }
                    insert_compose(&mut c, ch);
                }
            }
            true
        }
        _ => true,
    }
}

pub fn send_compose() {
    let (id, message) = {
        let mut c = compose_lock();
        if matches!(c.status, Status::Sending) {
            return;
        }
        if c.id.is_empty() {
            return;
        }
        if c.message.trim().is_empty() {
            c.status = Status::Error("Write a reply first.".into());
            return;
        }
        c.status = Status::Sending;
        c.focused = false;
        (c.id.clone(), c.message.trim().to_string())
    };
    std::thread::spawn(move || {
        let result = post_followup(&id, &message);
        let mut c = compose_lock();
        match result {
            Ok(()) => {
                append_user_msg(&id, &message);
                c.message.clear();
                c.cursor = 0;
                c.status = Status::Sent;
            }
            Err(e) => c.status = Status::Error(followup_error(&e)),
        }
    });
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

fn submit(kind: Kind, rating: u8, message: &str, log: Option<&FeedbackLog>, attach: bool) -> Status {
    match post(kind, rating, message, log, attach) {
        Ok(id) => {
            if let Some(id) = id {
                remember_ticket(id, kind, rating, message);
            }
            Status::Sent
        }
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
    } else if e.contains("413") || e.contains("422") || e.contains("too large") {
        "Couldn't send. Race log was too large.".into()
    } else if e.contains("gist") || e.contains("502") {
        "Couldn't send. Token needs gist access on Vercel.".into()
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

fn post(kind: Kind, rating: u8, message: &str, log: Option<&FeedbackLog>, attach: bool) -> Result<Option<String>, String> {
    let url = feedback_url();
    let body = payload_json(kind, rating, message, log, attach);
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(60))
        .user_agent("mxbo-overlay")
        .build();
    let resp = agent
        .post(&url)
        .set("Content-Type", "application/json")
        .send_string(&body)
        .map_err(|e| format!("{e}"))?;
    let status = resp.status();
    let text = resp.into_string().unwrap_or_default();
    if status >= 200 && status < 300 {
        Ok(json_string(&text, "id").filter(|id| !id.is_empty()))
    } else {
        Err(format!("HTTP {status}"))
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

fn first_install_version() -> String {
    crate::config::with_config(|c| {
        let v = c.first_install_version.trim();
        if v.is_empty() {
            "unknown".into()
        } else {
            v.to_string()
        }
    })
}

fn payload_json(kind: Kind, rating: u8, message: &str, log: Option<&FeedbackLog>, attach: bool) -> String {
    let kind_s = match kind {
        Kind::Rate => "rating",
        Kind::Bug => "bug",
        Kind::Feature => "feature",
    };
    let rating_s = if rating == 0 {
        "null".into()
    } else {
        rating.to_string()
    };
    let first_version = first_install_version();
    let mut s = format!(
        "{{\"kind\":\"{kind_s}\",\"rating\":{rating_s},\"message\":\"{}\",\"version\":\"{}\",\"first_version\":\"{}\",\"os\":\"{} {}\"",
        json_escape(message),
        json_escape(update::current_version()),
        json_escape(&first_version),
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
    } else if attach {
        s.push_str(",\"log_skipped\":true");
    }
    s.push('}');
    s
}

fn plain_report(kind: Kind, rating: u8, message: &str, log: Option<&FeedbackLog>) -> String {
    let mut body = String::new();
    body.push_str(&format!(
        "Holeshot HUD {}\nFirst install {}\n{} {}\n\n",
        update::current_version(),
        first_install_version(),
        std::env::consts::OS,
        std::env::consts::ARCH
    ));
    match kind {
        Kind::Rate => body.push_str("Kind: rating\n"),
        Kind::Bug => body.push_str("Kind: bug\n"),
        Kind::Feature => body.push_str("Kind: feature\n"),
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

pub fn ticket_view(id: &str) -> Option<ReplyView> {
    let tickets = tickets_lock();
    let t = tickets.iter().find(|t| t.id == id)?;
    Some(view_of(t))
}

pub fn pending_reply() -> Option<ReplyView> {
    tickets_lock().iter().find_map(|t| {
        if !unread_dev(t) {
            return None;
        }
        Some(view_of(t))
    })
}

fn view_of(t: &Ticket) -> ReplyView {
    let mut lines = Vec::new();
    if !t.summary.is_empty() {
        lines.push(ChatLine {
            from_dev: false,
            text: t.summary.clone(),
        });
    }
    for m in &t.thread {
        lines.push(ChatLine {
            from_dev: m.from_dev,
            text: m.text.clone(),
        });
    }
    if lines.len() == 1 && !t.reply.is_empty() && t.thread.is_empty() {
        lines.push(ChatLine {
            from_dev: true,
            text: t.reply.clone(),
        });
    }
    ReplyView {
        id: t.id.clone(),
        kind_label: kind_label(&t.kind),
        lines,
    }
}

fn unread_dev(t: &Ticket) -> bool {
    if t.seen_reply {
        return false;
    }
    if let Some(last) = t.thread.last() {
        return last.from_dev;
    }
    !t.reply.is_empty()
}

pub fn dismiss_reply(id: &str) {
    {
        let mut tickets = tickets_lock();
        if let Some(t) = tickets.iter_mut().find(|t| t.id == id) {
            t.seen_reply = true;
        }
        save_tickets(&tickets);
    }
}

/// Poll for replies while settings is on screen. No-op every frame unless due.
pub fn tick(settings_open: bool) {
    if !settings_open {
        return;
    }
    if tickets_lock().is_empty() {
        return;
    }
    let mut last = POLL_AT
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let due = last.map(|t| t.elapsed() >= POLL_EVERY).unwrap_or(true);
    if !due {
        return;
    }
    *last = Some(Instant::now());
    drop(last);
    spawn_poll();
}

/// Settings just came to the front — fetch replies now.
pub fn refresh() {
    if tickets_lock().is_empty() {
        return;
    }
    if let Ok(mut last) = POLL_AT
        .get_or_init(|| Mutex::new(None))
        .lock()
    {
        *last = Some(Instant::now());
    }
    spawn_poll();
}

fn spawn_poll() {
    if POLLING.swap(true, Ordering::SeqCst) {
        return;
    }
    let ids: Vec<String> = tickets_lock().iter().map(|t| t.id.clone()).collect();
    if ids.is_empty() {
        POLLING.store(false, Ordering::SeqCst);
        return;
    }
    std::thread::spawn(move || {
        if let Ok(remote) = fetch_tickets(&ids) {
            merge_remote(&remote);
        }
        POLLING.store(false, Ordering::SeqCst);
    });
}

fn kind_label(kind: &str) -> &'static str {
    match kind {
        "bug" => "Bug",
        "feature" => "Feature",
        "rating" => "Rating",
        _ => "Feedback",
    }
}

fn kind_key(kind: Kind) -> &'static str {
    match kind {
        Kind::Rate => "rating",
        Kind::Bug => "bug",
        Kind::Feature => "feature",
    }
}

fn clip_summary(s: &str) -> String {
    let t = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.chars().count() <= 72 {
        t
    } else {
        format!("{}…", t.chars().take(71).collect::<String>())
    }
}

fn remember_ticket(id: String, kind: Kind, rating: u8, message: &str) {
    let summary = if kind == Kind::Rate && message.trim().is_empty() {
        format!("{rating}/5")
    } else {
        clip_summary(message)
    };
    let ticket = Ticket {
        id,
        kind: kind_key(kind).into(),
        summary,
        sent_at: String::new(),
        reply: String::new(),
        replied_at: String::new(),
        thread: Vec::new(),
        seen_reply: false,
    };
    let mut tickets = tickets_lock();
    tickets.retain(|t| t.id != ticket.id);
    tickets.insert(0, ticket);
    if tickets.len() > MAX_TICKETS {
        tickets.truncate(MAX_TICKETS);
    }
    save_tickets(&tickets);
}

fn merge_remote(remote: &[Ticket]) {
    let mut tickets = tickets_lock();
    for incoming in remote {
        if let Some(local) = tickets.iter_mut().find(|t| t.id == incoming.id) {
            if !incoming.kind.is_empty() {
                local.kind = incoming.kind.clone();
            }
            if !incoming.summary.is_empty() {
                local.summary = incoming.summary.clone();
            }
            let thread_changed = incoming.thread != local.thread;
            if incoming.reply != local.reply || thread_changed {
                if unread_dev(incoming) {
                    local.seen_reply = false;
                }
            }
            local.reply = incoming.reply.clone();
            local.replied_at = incoming.replied_at.clone();
            local.thread = incoming.thread.clone();
        }
    }
    save_tickets(&tickets);
}

fn append_user_msg(id: &str, text: &str) {
    let mut tickets = tickets_lock();
    if let Some(local) = tickets.iter_mut().find(|t| t.id == id) {
        local.thread.push(Msg {
            from_dev: false,
            text: text.into(),
            at: String::new(),
        });
        local.seen_reply = true;
    }
    save_tickets(&tickets);
}

fn post_followup(id: &str, message: &str) -> Result<(), String> {
    let url = tickets_url();
    let body = format!(
        "{{\"id\":\"{}\",\"message\":\"{}\"}}",
        json_escape(id),
        json_escape(message)
    );
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(30))
        .user_agent("mxbo-overlay")
        .build();
    let resp = agent
        .post(&url)
        .set("Content-Type", "application/json")
        .send_string(&body)
        .map_err(|e| format!("{e}"))?;
    let status = resp.status();
    if status >= 200 && status < 300 {
        Ok(())
    } else {
        Err(format!("HTTP {status}"))
    }
}

fn followup_error(e: &str) -> String {
    let e = e.to_ascii_lowercase();
    if e.contains("503") {
        "Couldn't send. Add FEEDBACK_GITHUB_TOKEN on Vercel.".into()
    } else if e.contains("404") || e.contains("not found") {
        "Couldn't send. Deploy /api/tickets to Vercel.".into()
    } else if e.contains("could not") || e.contains("timed out") || e.contains("dns") {
        "Couldn't send. No connection to the server.".into()
    } else {
        "Couldn't send. Try again.".into()
    }
}

fn fetch_tickets(ids: &[String]) -> Result<Vec<Ticket>, String> {
    let url = format!("{}?ids={}", tickets_url(), ids.join(","));
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(20))
        .user_agent("mxbo-overlay")
        .build();
    let text = agent
        .get(&url)
        .call()
        .map_err(|e| format!("{e}"))?
        .into_string()
        .map_err(|e| format!("{e}"))?;
    Ok(parse_remote_tickets(&text))
}

fn tickets_url() -> String {
    feedback_url().replacen("/api/feedback", "/api/tickets", 1)
}

fn app_dir() -> Option<PathBuf> {
    Some(PathBuf::from(std::env::var("LOCALAPPDATA").ok()?).join("Holeshot HUD"))
}

fn tickets_path() -> Option<PathBuf> {
    Some(app_dir()?.join("tickets.json"))
}

fn load_tickets() -> Vec<Ticket> {
    let Some(path) = tickets_path() else {
        return Vec::new();
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    parse_local_tickets(&text)
}

#[cfg(not(test))]
fn save_tickets(tickets: &[Ticket]) {
    let Some(path) = tickets_path() else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, serialize_tickets(tickets));
}

#[cfg(test)]
fn save_tickets(_tickets: &[Ticket]) {}

fn serialize_tickets(tickets: &[Ticket]) -> String {
    let mut s = String::from("[\n");
    for (i, t) in tickets.iter().enumerate() {
        if i > 0 {
            s.push_str(",\n");
        }
        s.push_str("  {");
        s.push_str(&format!(
            "\"id\":\"{}\",\"kind\":\"{}\",\"summary\":\"{}\",\"sent_at\":\"{}\",\"reply\":\"{}\",\"replied_at\":\"{}\",\"thread\":{},\"seen_reply\":{}",
            json_escape(&t.id),
            json_escape(&t.kind),
            json_escape(&t.summary),
            json_escape(&t.sent_at),
            json_escape(&t.reply),
            json_escape(&t.replied_at),
            serialize_thread(&t.thread),
            if t.seen_reply { "true" } else { "false" },
        ));
        s.push('}');
    }
    s.push_str("\n]\n");
    s
}

fn parse_local_tickets(text: &str) -> Vec<Ticket> {
    json_objects(text)
        .into_iter()
        .filter_map(|obj| {
            let id = json_string(obj, "id").filter(|s| !s.is_empty())?;
            Some(Ticket {
                id,
                kind: json_string(obj, "kind").unwrap_or_default(),
                summary: json_string(obj, "summary").unwrap_or_default(),
                sent_at: json_string(obj, "sent_at").unwrap_or_default(),
                reply: json_string(obj, "reply").unwrap_or_default(),
                replied_at: json_string(obj, "replied_at").unwrap_or_default(),
                thread: parse_thread(obj),
                seen_reply: json_bool(obj, "seen_reply").unwrap_or(false),
            })
        })
        .take(MAX_TICKETS)
        .collect()
}

fn parse_remote_tickets(text: &str) -> Vec<Ticket> {
    let slice = json_array_slice(text, "tickets").unwrap_or(text);
    json_objects(slice)
        .into_iter()
        .filter_map(|obj| {
            let id = json_string(obj, "id").filter(|s| !s.is_empty())?;
            if json_array_slice(obj, "tickets").is_some() {
                return None;
            }
            Some(Ticket {
                id,
                kind: json_string(obj, "kind").unwrap_or_default(),
                summary: json_string(obj, "summary").unwrap_or_default(),
                sent_at: json_string(obj, "at").unwrap_or_default(),
                reply: json_string(obj, "reply").unwrap_or_default(),
                replied_at: json_string(obj, "replied_at").unwrap_or_default(),
                thread: parse_thread(obj),
                seen_reply: false,
            })
        })
        .collect()
}

fn serialize_thread(thread: &[Msg]) -> String {
    let mut s = String::from("[");
    for (i, m) in thread.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!(
            "{{\"from\":\"{}\",\"text\":\"{}\",\"at\":\"{}\"}}",
            if m.from_dev { "dev" } else { "user" },
            json_escape(&m.text),
            json_escape(&m.at),
        ));
    }
    s.push(']');
    s
}

fn parse_thread(obj: &str) -> Vec<Msg> {
    if let Some(arr) = json_array_slice(obj, "thread") {
        let mut out = Vec::new();
        for m in json_objects(arr) {
            let text = json_string(m, "text").unwrap_or_default();
            if text.is_empty() {
                continue;
            }
            let from = json_string(m, "from").unwrap_or_default();
            out.push(Msg {
                from_dev: from == "dev",
                text,
                at: json_string(m, "at").unwrap_or_default(),
            });
        }
        return out;
    }
    if let Some(reply) = json_string(obj, "reply").filter(|s| !s.is_empty()) {
        return vec![Msg {
            from_dev: true,
            text: reply,
            at: json_string(obj, "replied_at").unwrap_or_default(),
        }];
    }
    Vec::new()
}

fn json_objects(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'{' {
            i += 1;
            continue;
        }
        let start = i;
        let mut depth = 0i32;
        let mut in_str = false;
        let mut esc = false;
        while i < bytes.len() {
            let c = bytes[i];
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
                    if let Ok(slice) = std::str::from_utf8(&bytes[start..=i]) {
                        out.push(slice);
                    }
                    i += 1;
                    break;
                }
            }
            i += 1;
        }
    }
    out
}

#[cfg(test)]
#[path = "tests/feedback.rs"]
mod tests;
