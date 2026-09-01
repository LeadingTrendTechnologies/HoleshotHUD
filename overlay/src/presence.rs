use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use mxbo_hud::shm::{cstr, Snapshot, MAX_RIDERS, MAX_STANDINGS};
use mxbo_hud::config::HudConfig;

use crate::util::{json_array_slice, json_escape, json_string};

const DEFAULT_URL: &str = "https://holeshot-presence.holeshot-hud.workers.dev";
const WARMUP_KIND: i32 = 5;
const GATE_STATE: i32 = 256;
const ROLLING_STATE: i32 = 16;
const WARMUP_EVERY: Duration = Duration::from_secs(180);
const GATE_REMAIN_MS: i32 = 2500;
const GATE_COUNTDOWN_MAX_MS: i32 = 180_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardRider {
    pub race_num: i32,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteRider {
    pub race_num: i32,
    pub name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pulse {
    Silent,
    Publish,
    Leave,
}

#[derive(Debug)]
pub struct PulseState {
    last_session: String,
    last_kind: i32,
    last_state: i32,
    warmup_at: Option<Instant>,
    gate_fired: bool,
    published: bool,
}

impl Default for PulseState {
    fn default() -> Self {
        Self {
            last_session: String::new(),
            last_kind: -1,
            last_state: -1,
            warmup_at: None,
            gate_fired: false,
            published: false,
        }
    }
}

pub fn session_key(guid: &str, server: &str, track: &str, race_nums: &[i32]) -> Option<String> {
    let g = guid.trim();
    if !g.is_empty() {
        return Some(g.to_string());
    }
    let srv = server.trim();
    if srv.is_empty() {
        return None;
    }
    let mut nums: Vec<i32> = race_nums.iter().copied().filter(|n| *n > 0).collect();
    nums.sort_unstable();
    nums.dedup();
    Some(format!("fb:{:016x}", fingerprint(srv, track.trim(), &nums)))
}

pub fn match_room(board: &[BoardRider], remote: &[RemoteRider]) -> Vec<i32> {
    let mut out = Vec::new();
    for r in remote {
        if let Some(n) = match_one(board, r) {
            if !out.contains(&n) {
                out.push(n);
            }
        }
    }
    out
}

fn match_one(board: &[BoardRider], remote: &RemoteRider) -> Option<i32> {
    let want = normalize_name(&remote.name);
    if want.is_empty() && remote.race_num <= 0 {
        return None;
    }
    if remote.race_num > 0 {
        if let Some(row) = board.iter().find(|b| b.race_num == remote.race_num) {
            let have = normalize_name(&row.name);
            if have.is_empty() || have == want {
                return Some(row.race_num);
            }
        }
    }
    if want.is_empty() {
        return None;
    }
    let hits: Vec<i32> = board
        .iter()
        .filter(|b| normalize_name(&b.name) == want)
        .map(|b| b.race_num)
        .collect();
    if hits.len() == 1 {
        return Some(hits[0]);
    }
    None
}

fn normalize_name(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").to_ascii_lowercase()
}

fn fingerprint(server: &str, track: &str, nums: &[i32]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2965u64;
    for b in server.bytes().chain([0]).chain(track.bytes()).chain([0]) {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    for n in nums {
        h ^= *n as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

pub fn next_pulse(
    state: &mut PulseState,
    enabled: bool,
    session: Option<&str>,
    kind: i32,
    sess_state: i32,
    remain_ms: i32,
    now: Instant,
) -> Pulse {
    if !enabled {
        return finish_leave(state);
    }
    let Some(session) = session.filter(|s| !s.is_empty()) else {
        return finish_leave(state);
    };
    if state.last_session != session {
        state.last_session = session.to_string();
        state.last_kind = -1;
        state.last_state = -1;
        state.warmup_at = None;
        state.gate_fired = false;
        state.published = false;
        return apply_session(state, kind, sess_state, remain_ms, now);
    }
    apply_session(state, kind, sess_state, remain_ms, now)
}

fn apply_session(
    state: &mut PulseState,
    kind: i32,
    sess_state: i32,
    remain_ms: i32,
    now: Instant,
) -> Pulse {
    let warmup = kind == WARMUP_KIND;
    let in_gate = sess_state == GATE_STATE;
    let rolling = sess_state == ROLLING_STATE && kind != WARMUP_KIND;
    let kind_changed = state.last_kind != kind && state.last_kind >= 0;
    let entered_warmup = warmup && state.last_kind != WARMUP_KIND;
    let left_gate = state.last_state == GATE_STATE && rolling;

    if in_gate && !state.gate_fired && remain_ms > 0 && remain_ms <= GATE_REMAIN_MS {
        state.gate_fired = true;
        state.last_kind = kind;
        state.last_state = sess_state;
        state.published = true;
        return Pulse::Publish;
    }
    if left_gate && !state.gate_fired {
        state.gate_fired = true;
        state.last_kind = kind;
        state.last_state = sess_state;
        state.published = true;
        return Pulse::Publish;
    }
    if rolling {
        if sess_state != GATE_STATE && state.last_state != GATE_STATE {
            state.gate_fired = false;
        }
        state.last_kind = kind;
        state.last_state = sess_state;
        return Pulse::Silent;
    }
    if sess_state != GATE_STATE {
        state.gate_fired = false;
    }
    if warmup && (entered_warmup || kind_changed || state.warmup_at.is_none()) {
        state.warmup_at = Some(now);
        state.last_kind = kind;
        state.last_state = sess_state;
        state.published = true;
        return Pulse::Publish;
    }
    if warmup {
        let due = match state.warmup_at {
            None => true,
            Some(t) => now.saturating_duration_since(t) >= WARMUP_EVERY,
        };
        if due {
            state.warmup_at = Some(now);
            state.last_kind = kind;
            state.last_state = sess_state;
            state.published = true;
            return Pulse::Publish;
        }
    }
    state.last_kind = kind;
    state.last_state = sess_state;
    Pulse::Silent
}

fn finish_leave(state: &mut PulseState) -> Pulse {
    let was = state.published;
    *state = PulseState::default();
    if was {
        Pulse::Leave
    } else {
        Pulse::Silent
    }
}

pub fn board_from_snap(s: &Snapshot) -> Vec<BoardRider> {
    let mut out = Vec::new();
    let n = s.standing_count.clamp(0, MAX_STANDINGS as i32) as usize;
    for row in &s.standings[..n] {
        if row.race_num <= 0 {
            continue;
        }
        out.push(BoardRider {
            race_num: row.race_num,
            name: cstr(&row.name),
        });
    }
    if out.is_empty() {
        let n = s.rider_count.clamp(0, MAX_RIDERS as i32) as usize;
        for row in &s.riders[..n] {
            if row.race_num <= 0 {
                continue;
            }
            out.push(BoardRider {
                race_num: row.race_num,
                name: cstr(&row.name),
            });
        }
    }
    out
}

pub fn local_identity(s: &Snapshot) -> Option<(i32, String)> {
    let num = s.local_race_num;
    if num <= 0 {
        return None;
    }
    let name = board_from_snap(s)
        .into_iter()
        .find(|r| r.race_num == num)
        .map(|r| r.name)
        .filter(|n| !n.is_empty())
        .unwrap_or_default();
    if name.is_empty() {
        None
    } else {
        Some((num, name))
    }
}

pub fn gate_remain_ms(s: &Snapshot) -> i32 {
    if s.session_state != GATE_STATE {
        return i32::MAX;
    }
    let t = s.session_time_ms;
    if t > 0 && t <= GATE_COUNTDOWN_MAX_MS {
        t
    } else {
        i32::MAX
    }
}

static SENDING: AtomicBool = AtomicBool::new(false);

struct Live {
    pulse: PulseState,
    posted: String,
}

fn live() -> &'static Mutex<Live> {
    static LIVE: OnceLock<Mutex<Live>> = OnceLock::new();
    LIVE.get_or_init(|| {
        Mutex::new(Live {
            pulse: PulseState::default(),
            posted: String::new(),
        })
    })
}

pub fn tick(cfg: &HudConfig, snap: Option<&Snapshot>) {
    ensure_id(cfg);
    let enabled = cfg.show_presence;
    let (session, kind, state, remain, identity, board) = match snap {
        Some(s) if enabled => {
            let nums: Vec<i32> = board_from_snap(s).iter().map(|r| r.race_num).collect();
            let key = session_key(
                &cstr(&s.guid),
                &cstr(&s.server_name),
                &cstr(&s.track_name),
                &nums,
            );
            (
                key,
                s.session_kind,
                s.session_state,
                gate_remain_ms(s),
                local_identity(s),
                board_from_snap(s),
            )
        }
        _ => (None, -1, -1, i32::MAX, None, Vec::new()),
    };
    if enabled && session.is_some() && identity.is_none() {
        return;
    }
    let client_id = crate::config::with_config(|c| c.presence_id.clone());
    let pulse = {
        let mut g = live().lock().unwrap_or_else(|e| e.into_inner());
        next_pulse(
            &mut g.pulse,
            enabled,
            session.as_deref(),
            kind,
            state,
            remain,
            Instant::now(),
        )
    };
    match pulse {
        Pulse::Silent => {
            let old = {
                let mut g = live().lock().unwrap_or_else(|e| e.into_inner());
                match session.as_deref() {
                    Some(s) if !g.posted.is_empty() && g.posted != s => {
                        let prev = g.posted.clone();
                        g.posted.clear();
                        prev
                    }
                    _ => String::new(),
                }
            };
            if !old.is_empty() {
                mxbo_hud::set_presence_marks(&[]);
                post(None, old, client_id);
            }
        }
        Pulse::Leave => {
            let old = {
                let mut g = live().lock().unwrap_or_else(|e| e.into_inner());
                let s = g.posted.clone();
                g.posted.clear();
                s
            };
            mxbo_hud::set_presence_marks(&[]);
            if !old.is_empty() {
                post(None, old, client_id);
            }
        }
        Pulse::Publish => {
            let Some(session) = session else {
                return;
            };
            let Some((race_num, name)) = identity else {
                return;
            };
            let old = {
                let mut g = live().lock().unwrap_or_else(|e| e.into_inner());
                let prev = g.posted.clone();
                g.posted = session.clone();
                if prev.is_empty() || prev == session {
                    String::new()
                } else {
                    prev
                }
            };
            post(
                Some(Join {
                    session,
                    race_num,
                    name,
                    board,
                }),
                old,
                client_id,
            );
        }
    }
}

pub fn leave_now() {
    let session = {
        let mut g = live().lock().unwrap_or_else(|e| e.into_inner());
        let s = g.posted.clone();
        g.pulse = PulseState::default();
        g.posted.clear();
        s
    };
    mxbo_hud::set_presence_marks(&[]);
    if session.is_empty() {
        return;
    }
    let id = crate::config::with_config(|c| c.presence_id.clone());
    post(None, session, id);
}

fn ensure_id(cfg: &HudConfig) {
    if !cfg.presence_id.is_empty() {
        return;
    }
    crate::config::update_config(|c| {
        if c.presence_id.is_empty() {
            c.presence_id = new_presence_id();
        }
    });
}

fn new_presence_id() -> String {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(1);
    let mix = t ^ (u128::from(std::process::id()) << 80);
    format!("{mix:032x}")
}

struct Join {
    session: String,
    race_num: i32,
    name: String,
    board: Vec<BoardRider>,
}

fn post(join: Option<Join>, leave: String, client_id: String) {
    if client_id.is_empty() {
        return;
    }
    if SENDING.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(move || {
        let url = presence_url();
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(8))
            .build();
        if !leave.is_empty() {
            let body = format!(
                "{{\"session\":\"{}\",\"client_id\":\"{}\",\"leave\":true}}",
                json_escape(&leave),
                json_escape(&client_id)
            );
            let _ = agent
                .post(&url)
                .set("Content-Type", "application/json")
                .send_string(&body);
        }
        if let Some(join) = join {
            let body = format!(
                "{{\"session\":\"{}\",\"client_id\":\"{}\",\"race_num\":{},\"name\":\"{}\"}}",
                json_escape(&join.session),
                json_escape(&client_id),
                join.race_num,
                json_escape(&join.name)
            );
            if let Ok(resp) = agent
                .post(&url)
                .set("Content-Type", "application/json")
                .send_string(&body)
            {
                if let Ok(text) = resp.into_string() {
                    let remote = parse_riders(&text);
                    let mut marks = match_room(&join.board, &remote);
                    marks.retain(|n| *n != join.race_num);
                    mxbo_hud::set_presence_marks(&marks);
                }
            }
        }
        SENDING.store(false, Ordering::SeqCst);
    });
}

fn parse_riders(body: &str) -> Vec<RemoteRider> {
    let Some(arr) = json_array_slice(body, "riders") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut rest = arr;
    while let Some(start) = rest.find('{') {
        let chunk = &rest[start..];
        let Some(end) = chunk.find('}') else {
            break;
        };
        let obj = &chunk[..=end];
        let race_num = json_i32(obj, "race_num").unwrap_or(0);
        let name = json_string(obj, "name").unwrap_or_default();
        if race_num > 0 || !name.is_empty() {
            out.push(RemoteRider { race_num, name });
        }
        rest = &chunk[end + 1..];
    }
    out
}

fn json_i32(body: &str, key: &str) -> Option<i32> {
    let needle = format!("\"{key}\"");
    let i = body.find(&needle)?;
    let rest = &body[i + needle.len()..];
    let colon = rest.find(':')?;
    let rest = rest[colon + 1..].trim_start();
    let digits: String = rest
        .chars()
        .take_while(|c| *c == '-' || c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

fn presence_url() -> String {
    if let Ok(u) = std::env::var("HOLESHOT_PRESENCE_URL") {
        let t = u.trim();
        if !t.is_empty() {
            return t.to_string();
        }
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        let path = std::path::PathBuf::from(local)
            .join("Holeshot HUD")
            .join("presence-url.txt");
        if let Ok(s) = std::fs::read_to_string(&path) {
            let t = s.trim();
            if !t.is_empty() {
                return t.to_string();
            }
        }
    }
    DEFAULT_URL.to_string()
}

#[cfg(test)]
#[path = "tests/presence.rs"]
mod tests;
