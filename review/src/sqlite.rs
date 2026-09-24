//! Session review library (`%LOCALAPPDATA%\Holeshot HUD\reviews\reviews.sqlite`).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const FIELD_EVERY_MS: u64 = 2000;

use mxbo_hud::location_tape::{self, ChannelBin, CommittedLap, CrashMark, BINS};
use mxbo_hud::snapshot::{cstr, Snapshot};
use rusqlite::{params, Connection, OptionalExtension};

const WEEK_SECS: i64 = 7 * 24 * 3600;
const FORTNIGHT_SECS: i64 = 14 * 24 * 3600;
const RESUME_SECS: i64 = 10 * 60;

#[derive(Clone, Debug)]
pub struct SessionRow {
    pub id: i64,
    pub started: i64,
    pub track: String,
    /// Online server name from SHM (empty offline / unknown). Used for Ranked chip / trophy.
    pub server_name: String,
    /// True when `server_name` contains a `#lobby_id` present in `ranked_servers`.
    pub ranked: bool,
    pub rider_count: i32,
    pub your_position: i32,
    pub your_best_ms: i32,
    pub fastest_name: String,
    pub fastest_ms: i32,
    pub kept: bool,
    pub you_won: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListFilter {
    All,
    Ranked,
    Saved,
}

#[derive(Clone, Debug)]
pub struct RiderRow {
    pub race_num: i32,
    pub name: String,
    pub bike: String,
    pub position: i32,
    pub best_ms: i32,
    pub last_ms: i32,
    pub state: i32,
    pub penalty_ms: i32,
    #[allow(dead_code)]
    pub has_line: bool,
}

#[derive(Clone, Debug)]
pub struct LapView {
    pub race_num: i32,
    pub lap_num: i32,
    pub lap_ms: i32,
    #[allow(dead_code)]
    pub sectors: [i32; 3],
    pub bins: [ChannelBin; BINS],
    pub line: Vec<(f32, f32, f32)>,
    pub crashed: bool,
    pub cut: bool,
    pub warmup: bool,
    pub crashes: Vec<CrashMark>,
}

#[derive(Clone, Debug)]
pub struct SessionDetail {
    pub row: SessionRow,
    pub your_race_num: i32,
    pub fastest_race_num: i32,
    pub poly: Vec<(f32, f32)>,
    pub sf_meters: f32,
    pub riders: Vec<RiderRow>,
    pub laps: Vec<LapView>,
}

struct Live {
    id: i64,
    track: String,
    practice: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FieldKey {
    track: String,
    you: i32,
    fastest: i32,
    riders: i32,
}

struct Store {
    conn: Option<Connection>,
    live: Option<Live>,
    field_at: Option<Instant>,
    field_key: Option<FieldKey>,
    rev: u64,
    cached: Option<(i64, u64, Arc<SessionDetail>)>,
}

impl Store {
    const fn empty() -> Self {
        Self {
            conn: None,
            live: None,
            field_at: None,
            field_key: None,
            rev: 0,
            cached: None,
        }
    }
}

fn bump(st: &mut Store, id: Option<i64>) {
    let Some(id) = id else {
        return;
    };
    if st.cached.as_ref().is_some_and(|(cid, _, _)| *cid == id) {
        st.rev = st.rev.wrapping_add(1);
    }
}

fn invalidate(st: &mut Store) {
    st.rev = st.rev.wrapping_add(1);
    st.cached = None;
}

fn bins_blank(bins: &[ChannelBin; BINS]) -> bool {
    !bins
        .iter()
        .any(|b| b.ms > 0 || b.speed != 0.0 || b.y != 0.0)
}

fn poly_from_snap(s: &Snapshot) -> Vec<(f32, f32)> {
    let n = s.poly_count.clamp(0, mxbo_hud::shm::MAX_POLY as i32) as usize;
    s.poly.iter().take(n).map(|p| (p.x, p.z)).collect()
}

fn should_replace_poly(have: usize, next: usize) -> bool {
    have < 2 || next > have
}

fn patch_cached_field(st: &mut Store, id: i64, s: &Snapshot, you: i32, fastest: i32) {
    let Some((cid, _, arc)) = &mut st.cached else {
        return;
    };
    if *cid != id {
        return;
    }
    apply_field(Arc::make_mut(arc), s, you, fastest);
}

fn apply_field(d: &mut SessionDetail, s: &Snapshot, you: i32, fastest: i32) {
    let n = s
        .standing_count
        .clamp(0, mxbo_hud::shm::MAX_STANDINGS as i32) as usize;
    d.your_race_num = you;
    d.fastest_race_num = fastest;
    d.sf_meters = s.sf_meters;
    let next = poly_from_snap(s);
    if should_replace_poly(d.poly.len(), next.len()) {
        d.poly = next;
    }
    d.row.rider_count = n as i32;
    d.row.server_name = cstr(&s.server_name);
    d.row.ranked = lobby_id_from_server_name(&d.row.server_name)
        .is_some_and(|id| RANKED_LOBBY_IDS.contains(&id));
    let prev: Vec<(i32, bool, i32)> = d
        .riders
        .iter()
        .map(|r| (r.race_num, r.has_line, r.best_ms))
        .collect();
    d.riders = s
        .standings
        .iter()
        .take(n)
        .filter(|row| row.race_num > 0)
        .map(|row| {
            let stored = prev
                .iter()
                .find(|(n, _, _)| *n == row.race_num)
                .map(|(_, _, ms)| *ms)
                .unwrap_or(0);
            let live = row.best_lap_ms;
            let best_ms = if live > 0 && (stored == 0 || live < stored) {
                live
            } else {
                stored
            };
            RiderRow {
                race_num: row.race_num,
                name: cstr(&row.name),
                bike: cstr(&row.bike),
                position: row.position,
                best_ms,
                last_ms: row.last_lap_ms,
                state: row.state,
                penalty_ms: row.penalty_ms.max(0),
                has_line: prev
                    .iter()
                    .find(|(n, _, _)| *n == row.race_num)
                    .map(|(_, h, _)| *h)
                    .unwrap_or_else(|| {
                        d.laps
                            .iter()
                            .any(|l| l.race_num == row.race_num && !l.line.is_empty())
                    }),
            }
        })
        .collect();
    d.riders.sort_by_key(|r| (r.position, r.race_num));
    if let Some(f) = d.riders.iter().find(|r| r.race_num == fastest) {
        d.row.fastest_name = f.name.clone();
        d.row.fastest_ms = f.best_ms;
    }
    if let Some(y) = d.riders.iter().find(|r| r.race_num == you) {
        d.row.your_best_ms = y.best_ms;
    }
}

fn field_key(track: &str, you: i32, fastest: i32, riders: i32) -> FieldKey {
    FieldKey {
        track: track.to_string(),
        you,
        fastest,
        riders,
    }
}

fn field_due(same: bool, elapsed_ms: Option<u64>) -> bool {
    if !same {
        return true;
    }
    elapsed_ms.is_none_or(|ms| ms >= FIELD_EVERY_MS)
}

static STORE: Mutex<Store> = Mutex::new(Store::empty());

fn live() -> std::sync::MutexGuard<'static, Store> {
    STORE.lock().unwrap_or_else(|e| e.into_inner())
}

/// Session currently being recorded, if the overlay is in a moto.
pub fn live_id() -> Option<i64> {
    live().live.as_ref().map(|l| l.id)
}

pub fn set_live(id: i64, track: &str) {
    live().live = Some(Live {
        id,
        track: track.to_string(),
        practice: false,
    });
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn reset() {
    *live() = Store::empty();
}

pub fn serial() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Known MXB-Ranked lobby ids (allowlist for the Motos Ranked chip).
const RANKED_LOBBY_IDS: &[i64] = &[
    105004, 105010, 105001, 105005, 105019, 105020, 105014, 105017, 105013, 104785, 104847,
    104840, 104982, 104986, 105016, 105009, 104964, 104981, 105012, 105011, 104819, 104945,
    104999, 104996, 105008, 105018, 105015,
];

/// First `#` + digits in a server name (e.g. `… | #105019`).
pub fn lobby_id_from_server_name(s: &str) -> Option<i64> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'#' {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                end += 1;
            }
            if end > start {
                return s[start..end].parse().ok();
            }
        }
        i += 1;
    }
    None
}

fn ranked_lobby_set(c: &Connection) -> std::collections::HashSet<i64> {
    let mut set = std::collections::HashSet::new();
    if let Ok(mut stmt) = c.prepare("SELECT lobby_id FROM ranked_servers") {
        if let Ok(rows) = stmt.query_map([], |r| r.get::<_, i64>(0)) {
            for id in rows.flatten() {
                set.insert(id);
            }
        }
    }
    if set.is_empty() {
        set.extend(RANKED_LOBBY_IDS.iter().copied());
    }
    set
}

fn server_name_is_ranked(ids: &std::collections::HashSet<i64>, server_name: &str) -> bool {
    lobby_id_from_server_name(server_name).is_some_and(|id| ids.contains(&id))
}

fn seed_ranked_servers(c: &Connection) {
    let _ = c.execute_batch(
        "CREATE TABLE IF NOT EXISTS ranked_servers (
           lobby_id INTEGER PRIMARY KEY,
           host TEXT NOT NULL DEFAULT '',
           series TEXT NOT NULL DEFAULT '',
           class TEXT NOT NULL DEFAULT '',
           split TEXT NOT NULL DEFAULT '',
           region TEXT NOT NULL DEFAULT ''
         );",
    );
    for id in RANKED_LOBBY_IDS {
        let _ = c.execute(
            "INSERT OR IGNORE INTO ranked_servers (lobby_id) VALUES (?1)",
            params![id],
        );
    }
    // Freebies 250 S3- regional lobbies (metadata known).
    for (id, region) in [(105019i64, "EU"), (105020, "NA"), (105005, "NZ")] {
        let _ = c.execute(
            "UPDATE ranked_servers
             SET host = 'MXB-Ranked.com', series = 'MX Freebies', class = '250',
                 split = 'S3-', region = ?1
             WHERE lobby_id = ?2",
            params![region, id],
        );
    }
}

pub fn init(dir: PathBuf) {
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("reviews.sqlite");
    let conn = match Connection::open(&path) {
        Ok(c) => c,
        Err(_) => return,
    };
    let _ = conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         CREATE TABLE IF NOT EXISTS sessions (
           id INTEGER PRIMARY KEY,
           started INTEGER NOT NULL,
           track TEXT NOT NULL,
           kind INTEGER NOT NULL,
           rider_count INTEGER NOT NULL DEFAULT 0,
           fastest_race_num INTEGER NOT NULL DEFAULT -1,
           your_race_num INTEGER NOT NULL DEFAULT -1,
           kept INTEGER NOT NULL DEFAULT 0,
           poly BLOB
         );
         CREATE TABLE IF NOT EXISTS riders (
           session_id INTEGER NOT NULL,
           race_num INTEGER NOT NULL,
           name TEXT NOT NULL,
           bike TEXT NOT NULL,
           position INTEGER NOT NULL,
           best_ms INTEGER NOT NULL,
           last_ms INTEGER NOT NULL,
           PRIMARY KEY (session_id, race_num)
         );
         CREATE TABLE IF NOT EXISTS laps (
           session_id INTEGER NOT NULL,
           race_num INTEGER NOT NULL,
           lap_num INTEGER NOT NULL,
           ms INTEGER NOT NULL,
           s1 INTEGER NOT NULL,
           s2 INTEGER NOT NULL,
           s3 INTEGER NOT NULL,
           channels BLOB,
           line BLOB,
           PRIMARY KEY (session_id, race_num, lap_num)
         );
         CREATE TABLE IF NOT EXISTS crashes (
           session_id INTEGER NOT NULL,
           race_num INTEGER NOT NULL,
           lap_num INTEGER NOT NULL,
           x REAL NOT NULL,
           y REAL NOT NULL,
           z REAL NOT NULL,
           contact_num INTEGER NOT NULL DEFAULT -1,
           contact_name TEXT NOT NULL DEFAULT ''
         );
         CREATE INDEX IF NOT EXISTS sessions_started ON sessions(started);
         CREATE INDEX IF NOT EXISTS sessions_track ON sessions(track, started);
         CREATE INDEX IF NOT EXISTS crashes_lap ON crashes(session_id, race_num, lap_num);",
    );
    let _ = conn.execute(
        "ALTER TABLE laps ADD COLUMN warmup INTEGER NOT NULL DEFAULT 0",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE sessions ADD COLUMN sf_meters REAL NOT NULL DEFAULT -1",
        [],
    );
    let added_practice = conn
        .execute(
            "ALTER TABLE sessions ADD COLUMN practice INTEGER NOT NULL DEFAULT 0",
            [],
        )
        .is_ok();
    if added_practice {
        let _ = conn.execute("UPDATE sessions SET practice = 1 WHERE kind < 6", []);
    }
    let _ = conn.execute("ALTER TABLE sessions ADD COLUMN holeshot INTEGER", []);
    let _ = conn.execute(
        "ALTER TABLE sessions ADD COLUMN server_name TEXT NOT NULL DEFAULT ''",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE profile_races ADD COLUMN name TEXT NOT NULL DEFAULT ''",
        [],
    );
    let _ = conn.execute("ALTER TABLE profile_races ADD COLUMN holeshot INTEGER", []);
    let _ = conn.execute(
        "ALTER TABLE riders ADD COLUMN state INTEGER NOT NULL DEFAULT 0",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE riders ADD COLUMN penalty_ms INTEGER NOT NULL DEFAULT 0",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE profile_races ADD COLUMN state INTEGER NOT NULL DEFAULT 0",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE profile_races ADD COLUMN penalty_ms INTEGER NOT NULL DEFAULT 0",
        [],
    );
    let _ = conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS profile_races (
           session_id INTEGER PRIMARY KEY,
           started INTEGER NOT NULL,
           rider_count INTEGER NOT NULL,
           position INTEGER NOT NULL,
           your_best_ms INTEGER NOT NULL,
           fastest_ms INTEGER NOT NULL,
           laps BLOB NOT NULL,
           attack_hot INTEGER NOT NULL,
           attack_n INTEGER NOT NULL,
           air_hot INTEGER NOT NULL,
           air_n INTEGER NOT NULL,
           name TEXT NOT NULL DEFAULT '',
           holeshot INTEGER,
           state INTEGER NOT NULL DEFAULT 0,
           penalty_ms INTEGER NOT NULL DEFAULT 0
         );
         CREATE TABLE IF NOT EXISTS profile_meta (
           k TEXT PRIMARY KEY,
           v INTEGER NOT NULL
         );",
    );
    seed_ranked_servers(&conn);
    let mut st = live();
    st.conn = Some(conn);
    drop(st);
    prune();
}

pub fn tick(s: &Snapshot, in_session: bool) {
    if !in_session || s.has_telemetry == 0 {
        let mut st = live();
        if let Some(open) = st.live.take() {
            if let Some(c) = st.conn.as_ref() {
                finish_visit(c, open.id);
                compact_and_prune(c);
            }
            location_tape::reset();
            bump(&mut st, Some(open.id));
        }
        return;
    }
    let track = cstr(&s.track_name);
    if track.is_empty() {
        return;
    }
    // Practice is never stored; close any open race visit and leave Motos idle.
    if mxbo_hud::is_practice_session(s) {
        let mut st = live();
        if let Some(open) = st.live.take() {
            if let Some(c) = st.conn.as_ref() {
                if open.practice {
                    wipe_session(c, open.id);
                } else {
                    finish_visit(c, open.id);
                    compact_and_prune(c);
                }
            }
            location_tape::reset();
            bump(&mut st, Some(open.id));
        } else {
            location_tape::reset();
        }
        return;
    }
    location_tape::tick(s);
    let commits = location_tape::take_commits();
    let you = if s.local_race_num >= 0 {
        s.local_race_num
    } else {
        s.focus_race_num
    };
    let fastest = location_tape::fastest_num();
    {
        let mut st = live();
        let need_new = match &st.live {
            None => true,
            Some(l) => l.track != track || l.practice,
        };
        if need_new {
            if let Some(old) = st.live.take() {
                if let Some(c) = st.conn.as_ref() {
                    if old.practice {
                        wipe_session(c, old.id);
                    } else {
                        finish_visit(c, old.id);
                    }
                }
                bump(&mut st, Some(old.id));
            }
            if let Some(c) = st.conn.as_ref() {
                let server = cstr(&s.server_name);
                if let Ok(id) =
                    open_session(c, &track, s.session_kind, you, fastest, false, &server)
                {
                    st.live = Some(Live {
                        id,
                        track: track.clone(),
                        practice: false,
                    });
                    st.field_at = None;
                    st.field_key = None;
                }
            }
        }
        let id = st.live.as_ref().map(|l| l.id);
        let n = s
            .standing_count
            .clamp(0, mxbo_hud::shm::MAX_STANDINGS as i32);
        let key = field_key(&track, you, fastest, n);
        let same = st.field_key.as_ref() == Some(&key);
        let elapsed = st.field_at.map(|t| t.elapsed().as_millis() as u64);
        let due = field_due(same, elapsed);
        if let (Some(id), Some(c)) = (id, st.conn.as_ref()) {
            if due {
                let _ = upsert_field(c, id, s, you, fastest);
            }
            for lap in &commits {
                let _ = upsert_lap(c, id, lap);
            }
            note_holeshot(c, id, s, you, false);
        }
        if due && st.conn.is_some() && id.is_some() {
            st.field_at = Some(Instant::now());
            st.field_key = Some(key);
        }
        if need_new || !commits.is_empty() {
            bump(&mut st, id);
        } else if due {
            if let Some(id) = id {
                patch_cached_field(&mut st, id, s, you, fastest);
            }
        }
    }
}

pub fn list(filter: ListFilter) -> Vec<SessionRow> {
    let st = live();
    let Some(c) = st.conn.as_ref() else {
        return Vec::new();
    };
    let sql = match filter {
        ListFilter::All | ListFilter::Ranked => {
            "SELECT s.id, s.started, s.track, s.rider_count, s.fastest_race_num, s.your_race_num, s.kept,
                    (SELECT MIN(best_ms) FROM riders r WHERE r.session_id = s.id AND r.race_num = s.your_race_num),
                    (SELECT name FROM riders r WHERE r.session_id = s.id AND r.race_num = s.fastest_race_num),
                    (SELECT MIN(best_ms) FROM riders r WHERE r.session_id = s.id AND r.race_num = s.fastest_race_num),
                    (SELECT position FROM riders r WHERE r.session_id = s.id AND r.race_num = s.your_race_num),
                    s.server_name
             FROM sessions s
             WHERE s.practice = 0 AND EXISTS (SELECT 1 FROM laps l WHERE l.session_id = s.id)
             ORDER BY s.started DESC LIMIT 200"
        }
        ListFilter::Saved => {
            "SELECT s.id, s.started, s.track, s.rider_count, s.fastest_race_num, s.your_race_num, s.kept,
                    (SELECT MIN(best_ms) FROM riders r WHERE r.session_id = s.id AND r.race_num = s.your_race_num),
                    (SELECT name FROM riders r WHERE r.session_id = s.id AND r.race_num = s.fastest_race_num),
                    (SELECT MIN(best_ms) FROM riders r WHERE r.session_id = s.id AND r.race_num = s.fastest_race_num),
                    (SELECT position FROM riders r WHERE r.session_id = s.id AND r.race_num = s.your_race_num),
                    s.server_name
             FROM sessions s
             WHERE s.kept = 1 AND s.practice = 0 AND EXISTS (SELECT 1 FROM laps l WHERE l.session_id = s.id)
             ORDER BY s.started DESC LIMIT 200"
        }
    };
    let Ok(mut stmt) = c.prepare(sql) else {
        return Vec::new();
    };
    let rows = stmt.query_map([], |row| {
        Ok(SessionRow {
            id: row.get(0)?,
            started: row.get(1)?,
            track: row.get(2)?,
            rider_count: row.get(3)?,
            your_position: row.get::<_, Option<i32>>(10)?.unwrap_or(0),
            fastest_name: row.get::<_, Option<String>>(8)?.unwrap_or_default(),
            fastest_ms: row.get::<_, Option<i32>>(9)?.unwrap_or(0),
            your_best_ms: row.get::<_, Option<i32>>(7)?.unwrap_or(0),
            kept: row.get::<_, i32>(6)? != 0,
            you_won: false,
            server_name: row.get::<_, Option<String>>(11)?.unwrap_or_default(),
            ranked: false,
        })
    });
    let mut rows: Vec<SessionRow> = match rows {
        Ok(it) => it.filter_map(|r| r.ok()).collect(),
        Err(_) => return Vec::new(),
    };
    let ranked_ids = ranked_lobby_set(c);
    for row in &mut rows {
        row.you_won = session_you_won(c, row.id, row.rider_count);
        row.ranked = server_name_is_ranked(&ranked_ids, &row.server_name);
    }
    if filter == ListFilter::Ranked {
        rows.retain(|row| row.ranked);
    }
    rows
}

fn session_you_won(c: &Connection, id: i64, rider_count: i32) -> bool {
    let Ok((practice, kind, your_race_num)) = c.query_row(
        "SELECT practice, kind, your_race_num FROM sessions WHERE id = ?1",
        params![id],
        |r| Ok((r.get::<_, i32>(0)?, r.get::<_, i32>(1)?, r.get::<_, i32>(2)?)),
    ) else {
        return false;
    };
    let position: i32 = c
        .query_row(
            "SELECT position FROM riders WHERE session_id = ?1 AND race_num = ?2",
            params![id, your_race_num],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let has_race_lap: bool = c
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM laps
                WHERE session_id = ?1 AND race_num = ?2 AND COALESCE(warmup,0)=0 AND ms > 0
             )",
            params![id, your_race_num],
            |r| r.get::<_, i32>(0),
        )
        .ok()
        .is_some_and(|n| n != 0);
    crate::counts_as_profile_win(practice != 0, kind, rider_count, position, has_race_lap)
}

static WEB_DEMO: std::sync::OnceLock<SessionDetail> = std::sync::OnceLock::new();

/// In-memory session for WASM / marketing paint (no sqlite).
pub fn install_web_demo(detail: SessionDetail) {
    let _ = WEB_DEMO.set(detail);
}

pub fn load(id: i64) -> Option<Arc<SessionDetail>> {
    if let Some(d) = WEB_DEMO.get() {
        if d.row.id == id {
            return Some(Arc::new(d.clone()));
        }
    }
    let mut st = live();
    if let Some((cid, crev, detail)) = &st.cached {
        if *cid == id && *crev == st.rev {
            return Some(Arc::clone(detail));
        }
    }
    let detail = {
        let c = st.conn.as_ref()?;
        load_from(c, id)
    }?;
    let detail = Arc::new(detail);
    st.cached = Some((id, st.rev, Arc::clone(&detail)));
    Some(detail)
}

fn load_from(c: &Connection, id: i64) -> Option<SessionDetail> {
    let row = c
        .query_row(
            "SELECT id, started, track, rider_count, fastest_race_num, your_race_num, kept, poly, sf_meters,
                    practice, kind, server_name
             FROM sessions WHERE id = ?1",
            params![id],
            |r| {
                Ok((
                    SessionRow {
                        id: r.get(0)?,
                        started: r.get(1)?,
                        track: r.get(2)?,
                        rider_count: r.get(3)?,
                        your_position: 0,
                        fastest_name: String::new(),
                        fastest_ms: 0,
                        your_best_ms: 0,
                        kept: r.get::<_, i32>(6)? != 0,
                        you_won: false,
                        server_name: r.get::<_, Option<String>>(11)?.unwrap_or_default(),
                        ranked: false,
                    },
                    r.get::<_, i32>(4)?,
                    r.get::<_, i32>(5)?,
                    r.get::<_, Option<Vec<u8>>>(7)?,
                    r.get::<_, f32>(8).unwrap_or(-1.0),
                    r.get::<_, i32>(9).unwrap_or(0) != 0,
                    r.get::<_, i32>(10).unwrap_or(0),
                ))
            },
        )
        .optional()
        .ok()
        .flatten()?;
    let (mut row, fastest_race_num, your_race_num, poly_blob, sf_meters, practice, kind) = row;
    row.ranked = server_name_is_ranked(&ranked_lobby_set(c), &row.server_name);
    let mut riders = Vec::new();
    if let Ok(mut stmt) = c.prepare(
        "SELECT race_num, name, bike, position, best_ms, last_ms, state, penalty_ms,
                EXISTS(SELECT 1 FROM laps l WHERE l.session_id = riders.session_id AND l.race_num = riders.race_num)
         FROM riders WHERE session_id = ?1 ORDER BY position, race_num",
    ) {
        if let Ok(it) = stmt.query_map(params![id], |r| {
            Ok(RiderRow {
                race_num: r.get(0)?,
                name: r.get(1)?,
                bike: r.get(2)?,
                position: r.get(3)?,
                best_ms: r.get(4)?,
                last_ms: r.get(5)?,
                state: r.get::<_, i32>(6).unwrap_or(0),
                penalty_ms: r.get::<_, i32>(7).unwrap_or(0),
                has_line: r.get::<_, i32>(8)? != 0,
            })
        }) {
            riders = it.filter_map(|r| r.ok()).collect();
        }
    }
    if let Some(f) = riders.iter().find(|r| r.race_num == fastest_race_num) {
        row.fastest_name = f.name.clone();
        row.fastest_ms = f.best_ms;
    }
    if let Some(y) = riders.iter().find(|r| r.race_num == your_race_num) {
        row.your_best_ms = y.best_ms;
        row.your_position = y.position;
    }
    let mut laps = Vec::new();
    if let Ok(mut stmt) = c.prepare(
        "SELECT race_num, lap_num, ms, s1, s2, s3,
                CASE WHEN race_num IN (?2, ?3) THEN channels ELSE NULL END,
                CASE WHEN race_num IN (?2, ?3) THEN line ELSE NULL END, warmup
         FROM laps WHERE session_id = ?1 ORDER BY race_num, lap_num",
    ) {
        if let Ok(it) = stmt.query_map(params![id, your_race_num, fastest_race_num], |r| {
            let channels: Option<Vec<u8>> = r.get(6)?;
            let line: Option<Vec<u8>> = r.get(7)?;
            Ok((
                r.get::<_, i32>(1)?,
                LapView {
                    race_num: r.get(0)?,
                    lap_num: r.get(1)?,
                    lap_ms: r.get(2)?,
                    sectors: [r.get(3)?, r.get(4)?, r.get(5)?],
                    bins: unpack_channels(channels.as_deref()),
                    line: unpack_line(line.as_deref()),
                    crashed: false,
                    cut: false,
                    warmup: r.get::<_, i32>(8).unwrap_or(0) != 0,
                    crashes: Vec::new(),
                },
            ))
        }) {
            let numbered: Vec<(i32, LapView)> = it.filter_map(|r| r.ok()).collect();
            let marks = load_crashes(c, id);
            laps = numbered
                .into_iter()
                .map(|(lap_num, mut lap)| {
                    lap.crashes = marks
                        .iter()
                        .filter(|m| m.0 == lap.race_num && m.1 == lap_num)
                        .map(|m| m.2.clone())
                        .collect();
                    lap.crashed = !lap.crashes.is_empty();
                    lap.cut = if lap.line.is_empty() && bins_blank(&lap.bins) {
                        false
                    } else {
                        location_tape::infer_cut(&lap.line, &lap.bins, lap.lap_ms)
                    };
                    lap
                })
                .collect();
        }
    }
    if let Some(y) = riders.iter().find(|r| r.race_num == your_race_num) {
        let has_race_lap = laps
            .iter()
            .any(|l| l.race_num == your_race_num && !l.warmup && l.lap_ms > 0);
        row.you_won = crate::counts_as_profile_win(
            practice,
            kind,
            row.rider_count,
            y.position,
            has_race_lap,
        );
    }
    Some(SessionDetail {
        row,
        your_race_num,
        fastest_race_num,
        poly: unpack_poly(poly_blob.as_deref()),
        sf_meters,
        riders,
        laps,
    })
}

/// Fill a compare rider's line and channel tapes if `load` skipped unpacking them.
pub fn ensure_line(id: i64, race_num: i32) {
    let mut st = live();
    let Some((cid, _, arc)) = &st.cached else {
        return;
    };
    if *cid != id {
        return;
    }
    if !arc.laps.iter().any(|l| l.race_num == race_num) {
        return;
    }
    if arc
        .laps
        .iter()
        .any(|l| l.race_num == race_num && !l.line.is_empty() && !bins_blank(&l.bins))
    {
        return;
    }
    let blobs: Vec<(i32, Option<Vec<u8>>, Option<Vec<u8>>)> = {
        let Some(c) = st.conn.as_ref() else {
            return;
        };
        let Ok(mut stmt) = c.prepare(
            "SELECT lap_num, line, channels FROM laps WHERE session_id = ?1 AND race_num = ?2",
        ) else {
            return;
        };
        let rows = match stmt.query_map(params![id, race_num], |r| {
            Ok((
                r.get::<_, i32>(0)?,
                r.get::<_, Option<Vec<u8>>>(1)?,
                r.get::<_, Option<Vec<u8>>>(2)?,
            ))
        }) {
            Ok(it) => it.filter_map(|r| r.ok()).collect(),
            Err(_) => return,
        };
        rows
    };
    if blobs.is_empty() {
        return;
    }
    let unpacked: Vec<(i32, Vec<(f32, f32, f32)>, [ChannelBin; BINS])> = blobs
        .into_iter()
        .map(|(lap_num, line, channels)| {
            (
                lap_num,
                unpack_line(line.as_deref()),
                unpack_channels(channels.as_deref()),
            )
        })
        .collect();
    let Some((cid, _, arc)) = &mut st.cached else {
        return;
    };
    if *cid != id {
        return;
    }
    let detail = Arc::make_mut(arc);
    for lap in &mut detail.laps {
        if lap.race_num != race_num {
            continue;
        }
        let Some((_, line, bins)) = unpacked.iter().find(|(n, _, _)| *n == lap.lap_num) else {
            continue;
        };
        if lap.line.is_empty() {
            lap.line = line.clone();
        }
        if bins_blank(&lap.bins) {
            lap.bins = *bins;
        }
        if !lap.line.is_empty() || !bins_blank(&lap.bins) {
            lap.cut = location_tape::infer_cut(&lap.line, &lap.bins, lap.lap_ms);
        }
    }
}

pub fn set_kept(id: i64, kept: bool) {
    let mut st = live();
    if let Some(c) = st.conn.as_ref() {
        let _ = c.execute(
            "UPDATE sessions SET kept = ?1 WHERE id = ?2",
            params![if kept { 1 } else { 0 }, id],
        );
    }
    bump(&mut st, Some(id));
}

pub fn delete(id: i64) {
    let mut st = live();
    if let Some(c) = st.conn.as_ref() {
        wipe_session(c, id);
    }
    if st.live.as_ref().is_some_and(|l| l.id == id) {
        st.live = None;
    }
    bump(&mut st, Some(id));
}

pub fn clear_profile() {
    let mut st = live();
    if let Some(c) = st.conn.as_ref() {
        let _ = c.execute("DELETE FROM profile_races", []);
        let _ = c.execute(
            "INSERT INTO profile_meta (k, v) VALUES ('cleared_at', ?1)
             ON CONFLICT(k) DO UPDATE SET v = excluded.v",
            params![now_secs()],
        );
    }
    invalidate(&mut st);
}

pub fn clear_motos() {
    let mut st = live();
    let keep = st.live.as_ref().map(|l| l.id);
    if let Some(c) = st.conn.as_ref() {
        if let Some(id) = keep {
            let _ = c.execute("DELETE FROM crashes WHERE session_id != ?1", params![id]);
            let _ = c.execute("DELETE FROM riders WHERE session_id != ?1", params![id]);
            let _ = c.execute("DELETE FROM laps WHERE session_id != ?1", params![id]);
            let _ = c.execute("DELETE FROM sessions WHERE id != ?1", params![id]);
        } else {
            let _ = c.execute("DELETE FROM crashes", []);
            let _ = c.execute("DELETE FROM riders", []);
            let _ = c.execute("DELETE FROM laps", []);
            let _ = c.execute("DELETE FROM sessions", []);
        }
    }
    invalidate(&mut st);
}

pub fn prune() {
    let mut st = live();
    if let Some(c) = st.conn.as_ref() {
        purge_practice(c);
        compact_and_prune(c);
    }
    let cached = st.cached.as_ref().map(|(id, _, _)| *id);
    bump(&mut st, cached);
}

/// Synthetic motos for list / Analyze dumps. Returns the first session id.
pub fn seed_demo() -> Option<i64> {
    let mut st = live();
    let c = st.conn.as_ref()?;
    let now = now_secs();
    let poly = kidney_poly(96, 92.0, 58.0);
    let poly_blob = pack_poly_pts(&poly);
    let you_line = offset_line(&poly, 3.4, 0.4);
    let other_line = offset_line(&poly, -3.6, 0.9);
    let you_bins = demo_bins(48.2, 1.4, 0.08);
    let other_bins = demo_bins(50.6, 1.1, -0.05);

    let motos: &[(
        &str,
        i64,
        i32,
        bool,
        i32,
        i32,
        &[(&str, i32, i32, i32, &str)],
    )] = &[
        (
            "Hangtown",
            now - 40 * 60,
            12,
            true,
            2,
            7,
            &[
                ("Cole", 7, 1, 111_420, "YZ450F"),
                ("You", 2, 3, 113_080, "FC 450"),
                ("Reed", 21, 4, 114_210, "CRF450R"),
                ("Webb", 1, 2, 112_040, "KX450"),
                ("Barcia", 51, 6, 116_900, "MC450F"),
                ("Plessinger", 32, 5, 115_330, "KX450"),
            ],
        ),
        (
            "Pala",
            now - 5 * 3600,
            22,
            false,
            14,
            9,
            &[
                ("Anderson", 9, 1, 98_640, "KX450"),
                ("You", 14, 5, 101_220, "FC 450"),
                ("Ferrandis", 4, 2, 99_180, "YZ450F"),
                ("Craig", 32, 8, 103_400, "CRF450R"),
            ],
        ),
        (
            "Thunder Valley",
            now - 26 * 3600,
            18,
            false,
            2,
            2,
            &[
                ("You", 2, 2, 124_500, "FC 450"),
                ("Sexton", 23, 1, 122_880, "CRF450R"),
                ("Lawrence", 96, 3, 125_010, "YZ450F"),
            ],
        ),
        (
            "RedBud",
            now - 4 * 86400,
            28,
            false,
            41,
            3,
            &[
                ("Jett", 3, 1, 104_110, "CRF450R"),
                ("You", 41, 9, 108_770, "FC 450"),
                ("Hunter", 96, 2, 104_880, "YZ450F"),
            ],
        ),
        (
            "Southwick",
            now - 9 * 86400,
            16,
            false,
            2,
            18,
            &[
                ("Tomac", 18, 1, 119_200, "YZ450F"),
                ("You", 2, 4, 121_640, "FC 450"),
                ("Febvre", 8, 3, 120_900, "KX450"),
            ],
        ),
    ];

    let mut first = None;
    for (i, (track, started, riders, kept, you, fastest, field)) in motos.iter().enumerate() {
        c.execute(
            "INSERT INTO sessions (started, track, kind, rider_count, fastest_race_num, your_race_num, kept, poly)
             VALUES (?1, ?2, 2, ?3, ?4, ?5, ?6, ?7)",
            params![started, track, riders, fastest, you, if *kept { 1 } else { 0 }, poly_blob],
        )
        .ok()?;
        let id = c.last_insert_rowid();
        if first.is_none() {
            first = Some(id);
        }
        for (name, num, pos, best, bike) in *field {
            c.execute(
                "INSERT INTO riders (session_id, race_num, name, bike, position, best_ms, last_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![id, num, name, bike, pos, best, best + 420],
            )
            .ok()?;
        }
        if i == 0 {
            let you_wu = CommittedLap {
                race_num: *you,
                name: "You".into(),
                bike: "FC 450".into(),
                lap_ms: 118_500,
                sectors: [38_200, 40_100, 40_200],
                bins: you_bins,
                line: you_line.clone(),
                is_you: true,
                crashed: false,
                cut: false,
                warmup: true,
                crashes: Vec::new(),
            };
            let _ = upsert_lap(c, id, &you_wu);
            let you_lap = CommittedLap {
                race_num: *you,
                name: "You".into(),
                bike: "FC 450".into(),
                lap_ms: 113_080,
                sectors: [36_420, 38_110, 38_550],
                bins: you_bins,
                line: you_line.clone(),
                is_you: true,
                crashed: false,
                cut: false,
                warmup: false,
                crashes: Vec::new(),
            };
            let other_lap = CommittedLap {
                race_num: *fastest,
                name: "Cole".into(),
                bike: "YZ450F".into(),
                lap_ms: 111_420,
                sectors: [35_880, 37_640, 37_900],
                bins: other_bins,
                line: other_line.clone(),
                is_you: false,
                crashed: false,
                cut: false,
                warmup: false,
                crashes: Vec::new(),
            };
            let _ = upsert_lap(c, id, &you_lap);
            let _ = upsert_lap(c, id, &other_lap);
            for extra in [113_400, 114_100, 112_900] {
                let mut more = you_lap.clone();
                more.lap_ms = extra;
                let _ = upsert_lap(c, id, &more);
            }
        } else {
            let you_best = field
                .iter()
                .find(|r| r.1 == *you)
                .map(|r| r.3)
                .unwrap_or(120_000);
            let fast_best = field
                .iter()
                .find(|r| r.1 == *fastest)
                .map(|r| r.3)
                .unwrap_or(you_best);
            let you_name = field
                .iter()
                .find(|r| r.1 == *you)
                .map(|r| r.0)
                .unwrap_or("You");
            let fast_name = field
                .iter()
                .find(|r| r.1 == *fastest)
                .map(|r| r.0)
                .unwrap_or("Cole");
            for extra in [0, 800, 1_400, -400] {
                let mut lap = CommittedLap {
                    race_num: *you,
                    name: you_name.into(),
                    bike: "FC 450".into(),
                    lap_ms: you_best + extra,
                    sectors: [0, 0, 0],
                    bins: you_bins,
                    line: you_line.clone(),
                    is_you: true,
                    crashed: false,
                    cut: false,
                    warmup: false,
                    crashes: Vec::new(),
                };
                if extra < 0 {
                    lap.lap_ms = you_best + extra.abs();
                }
                let _ = upsert_lap(c, id, &lap);
            }
            let _ = upsert_lap(
                c,
                id,
                &CommittedLap {
                    race_num: *fastest,
                    name: fast_name.into(),
                    bike: "YZ450F".into(),
                    lap_ms: fast_best,
                    sectors: [0, 0, 0],
                    bins: other_bins,
                    line: other_line.clone(),
                    is_you: false,
                    crashed: false,
                    cut: false,
                    warmup: false,
                    crashes: Vec::new(),
                },
            );
        }
    }
    if let Some(id) = first {
        let _ = c.execute(
            "UPDATE sessions SET holeshot = 1 WHERE id = ?1",
            params![id],
        );
    }
    let ids: Vec<i64> = c
        .prepare("SELECT id FROM sessions")
        .ok()
        .and_then(|mut stmt| {
            stmt.query_map([], |r| r.get(0))
                .ok()
                .map(|it| it.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_default();
    for id in ids {
        upsert_profile_race(c, id);
    }
    bump(&mut st, first);
    first
}

/// Hangtown demo for web Analyze (matches first `seed_demo` row).
pub fn demo_session() -> SessionDetail {
    let now = now_secs();
    let poly = kidney_poly(96, 92.0, 58.0);
    let you_line = offset_line(&poly, 3.4, 0.4);
    let other_line = offset_line(&poly, -3.6, 0.9);
    let you_bins = demo_bins(48.2, 1.4, 0.08);
    let other_bins = demo_bins(50.6, 1.1, -0.05);
    let riders = vec![
        RiderRow {
            race_num: 7,
            name: "Cole".into(),
            bike: "YZ450F".into(),
            position: 1,
            best_ms: 111_420,
            last_ms: 111_840,
            state: 0,
            penalty_ms: 0,
            has_line: true,
        },
        RiderRow {
            race_num: 2,
            name: "You".into(),
            bike: "FC 450".into(),
            position: 3,
            best_ms: 113_080,
            last_ms: 113_500,
            state: 0,
            penalty_ms: 0,
            has_line: true,
        },
        RiderRow {
            race_num: 21,
            name: "Reed".into(),
            bike: "CRF450R".into(),
            position: 4,
            best_ms: 114_210,
            last_ms: 114_630,
            state: 0,
            penalty_ms: 0,
            has_line: false,
        },
        RiderRow {
            race_num: 1,
            name: "Webb".into(),
            bike: "KX450".into(),
            position: 2,
            best_ms: 112_040,
            last_ms: 112_460,
            state: 0,
            penalty_ms: 0,
            has_line: true,
        },
    ];
    let you_lap = LapView {
        race_num: 2,
        lap_num: 1,
        lap_ms: 113_080,
        sectors: [36_420, 38_110, 38_550],
        bins: you_bins,
        line: you_line,
        crashed: false,
        cut: false,
        warmup: false,
        crashes: Vec::new(),
    };
    let other_lap = LapView {
        race_num: 1,
        lap_num: 1,
        lap_ms: 112_040,
        sectors: [35_880, 37_640, 37_900],
        bins: other_bins,
        line: other_line,
        crashed: false,
        cut: false,
        warmup: false,
        crashes: Vec::new(),
    };
    SessionDetail {
        row: SessionRow {
            id: 1,
            started: now - 40 * 60,
            track: "Hangtown".into(),
            server_name: String::new(),
            ranked: false,
            rider_count: 12,
            your_position: 3,
            your_best_ms: 113_080,
            fastest_name: "Cole".into(),
            fastest_ms: 111_420,
            kept: true,
            you_won: false,
        },
        your_race_num: 2,
        fastest_race_num: 7,
        poly,
        sf_meters: -1.0,
        riders,
        laps: vec![you_lap, other_lap],
    }
}

fn kidney_poly(n: usize, rx: f32, rz: f32) -> Vec<(f32, f32)> {
    (0..n)
        .map(|i| {
            let t = i as f32 / n as f32 * std::f32::consts::TAU;
            let k = 0.72 + 0.28 * (2.0 * t).sin();
            (rx * t.cos(), rz * t.sin() * k)
        })
        .collect()
}

fn offset_line(poly: &[(f32, f32)], n: f32, phase: f32) -> Vec<(f32, f32, f32)> {
    let len = poly.len();
    poly.iter()
        .enumerate()
        .map(|(i, (x, z))| {
            let j = (i + 1) % len;
            let dx = poly[j].0 - x;
            let dz = poly[j].1 - z;
            let m = (dx * dx + dz * dz).sqrt().max(0.001);
            let wobble = n + 0.55 * ((i as f32 * 0.18) + phase).sin();
            (
                x - dz / m * wobble,
                1.1 + 0.35 * (i as f32 * 0.11).sin(),
                z + dx / m * wobble,
            )
        })
        .collect()
}

fn demo_bins(base_speed: f32, base_y: f32, time_bias: f32) -> [ChannelBin; BINS] {
    let mut bins = [ChannelBin::default(); BINS];
    let mut ms = 0;
    for (i, b) in bins.iter_mut().enumerate() {
        let t = i as f32 / BINS as f32;
        let corner = (t * std::f32::consts::TAU * 2.0 + 0.4).sin();
        b.speed = base_speed + corner * 11.0;
        b.y = base_y + (t * 9.0).sin() * 0.55 + corner.abs() * 0.2;
        b.rpm = 8200.0 + corner * 1400.0;
        b.throttle = (0.55 + corner * 0.42).clamp(0.05, 1.0);
        b.brake = if corner < -0.35 {
            (-corner - 0.35) * 1.4
        } else {
            0.0
        };
        b.lean = corner * 18.0;
        b.yaw = t * std::f32::consts::TAU;
        ms += (420.0 + time_bias * 80.0 - corner * 40.0) as i32;
        b.ms = ms;
        b.gear = 2 + ((t * 3.0).floor() as i32).clamp(0, 3);
    }
    bins
}

fn pack_poly_pts(pts: &[(f32, f32)]) -> Vec<u8> {
    let mut o = Vec::with_capacity(4 + pts.len() * 8);
    o.extend_from_slice(&(pts.len() as u32).to_le_bytes());
    for (x, z) in pts {
        o.extend_from_slice(&x.to_le_bytes());
        o.extend_from_slice(&z.to_le_bytes());
    }
    o
}

fn open_session(
    c: &Connection,
    track: &str,
    kind: i32,
    you: i32,
    fastest: i32,
    practice: bool,
    server_name: &str,
) -> rusqlite::Result<i64> {
    let recent = c
        .query_row(
            "SELECT id, started, practice FROM sessions WHERE track = ?1 ORDER BY started DESC LIMIT 1",
            params![track],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, i32>(2)? != 0)),
        )
        .optional()?;
    if let Some((id, started, was_practice)) = recent {
        if now_secs() - started <= RESUME_SECS && was_practice == practice {
            if !server_name.is_empty() {
                let _ = c.execute(
                    "UPDATE sessions SET server_name = ?1 WHERE id = ?2 AND server_name = ''",
                    params![server_name, id],
                );
            }
            return Ok(id);
        }
    }
    insert_typed(c, track, kind, you, fastest, practice, server_name)
}

#[cfg(test)]
fn insert_session(
    c: &Connection,
    track: &str,
    kind: i32,
    you: i32,
    fastest: i32,
) -> rusqlite::Result<i64> {
    insert_typed(c, track, kind, you, fastest, false, "")
}

fn insert_typed(
    c: &Connection,
    track: &str,
    kind: i32,
    you: i32,
    fastest: i32,
    practice: bool,
    server_name: &str,
) -> rusqlite::Result<i64> {
    c.execute(
        "INSERT INTO sessions (started, track, kind, your_race_num, fastest_race_num, practice, server_name)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            now_secs(),
            track,
            kind,
            you,
            fastest,
            if practice { 1 } else { 0 },
            server_name
        ],
    )?;
    Ok(c.last_insert_rowid())
}

fn session_has_laps(c: &Connection, id: i64) -> bool {
    c.query_row(
        "SELECT EXISTS(SELECT 1 FROM laps WHERE session_id = ?1)",
        params![id],
        |r| r.get::<_, i32>(0),
    )
    .ok()
    .is_some_and(|n| n != 0)
}

fn wipe_session(c: &Connection, id: i64) {
    let _ = c.execute("DELETE FROM crashes WHERE session_id = ?1", params![id]);
    let _ = c.execute("DELETE FROM riders WHERE session_id = ?1", params![id]);
    let _ = c.execute("DELETE FROM laps WHERE session_id = ?1", params![id]);
    let _ = c.execute(
        "DELETE FROM profile_races WHERE session_id = ?1",
        params![id],
    );
    let _ = c.execute("DELETE FROM sessions WHERE id = ?1", params![id]);
}

fn purge_practice(c: &Connection) {
    let ids: Vec<i64> = c
        .prepare("SELECT id FROM sessions WHERE practice = 1")
        .ok()
        .and_then(|mut stmt| {
            stmt.query_map([], |r| r.get(0))
                .ok()
                .map(|it| it.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_default();
    for id in ids {
        wipe_session(c, id);
    }
}

fn delete_if_empty(c: &Connection, id: i64) {
    if session_has_laps(c, id) {
        return;
    }
    let _ = c.execute("DELETE FROM crashes WHERE session_id = ?1", params![id]);
    let _ = c.execute("DELETE FROM riders WHERE session_id = ?1", params![id]);
    let _ = c.execute("DELETE FROM laps WHERE session_id = ?1", params![id]);
    let _ = c.execute(
        "DELETE FROM profile_races WHERE session_id = ?1",
        params![id],
    );
    let _ = c.execute(
        "DELETE FROM sessions WHERE id = ?1 AND kept = 0",
        params![id],
    );
}

fn finish_visit(c: &Connection, id: i64) {
    upsert_profile_race(c, id);
    delete_if_empty(c, id);
}

fn note_holeshot(c: &Connection, id: i64, s: &Snapshot, you: i32, practice: bool) {
    if practice || s.session_kind == crate::WARMUP_KIND || s.holeshot_race_num <= 0 {
        return;
    }
    let v = if s.holeshot_race_num == you { 1 } else { 0 };
    let _ = c.execute(
        "UPDATE sessions SET holeshot = ?1 WHERE id = ?2 AND holeshot IS NULL",
        params![v, id],
    );
}

fn upsert_field(
    c: &Connection,
    id: i64,
    s: &Snapshot,
    you: i32,
    fastest: i32,
) -> rusqlite::Result<()> {
    let n = s
        .standing_count
        .clamp(0, mxbo_hud::shm::MAX_STANDINGS as i32) as usize;
    c.execute(
        "UPDATE sessions SET rider_count = ?1, your_race_num = ?2, fastest_race_num = ?3, poly = ?4, sf_meters = ?5, practice = ?6, server_name = ?7 WHERE id = ?8",
        params![
            n as i32,
            you,
            fastest,
            pack_poly(s),
            s.sf_meters,
            if mxbo_hud::is_practice_session(s) { 1 } else { 0 },
            cstr(&s.server_name),
            id
        ],
    )?;
    for row in s.standings.iter().take(n) {
        if row.race_num <= 0 {
            continue;
        }
        c.execute(
            "INSERT INTO riders (session_id, race_num, name, bike, position, best_ms, last_ms, state, penalty_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(session_id, race_num) DO UPDATE SET
               name = excluded.name, bike = excluded.bike, position = excluded.position,
               best_ms = CASE
                 WHEN excluded.best_ms > 0 AND (riders.best_ms = 0 OR excluded.best_ms < riders.best_ms)
                 THEN excluded.best_ms ELSE riders.best_ms END,
               last_ms = excluded.last_ms,
               state = excluded.state,
               penalty_ms = excluded.penalty_ms",
            params![
                id,
                row.race_num,
                cstr(&row.name),
                cstr(&row.bike),
                row.position,
                row.best_lap_ms,
                row.last_lap_ms,
                row.state,
                row.penalty_ms.max(0)
            ],
        )?;
    }
    Ok(())
}

fn upsert_lap(c: &Connection, id: i64, lap: &CommittedLap) -> rusqlite::Result<()> {
    if !lap.is_you {
        if lap.crashed {
            return Ok(());
        }
        if let Some(ms) = other_stored_ms(c, id, lap.race_num) {
            if ms > 0 && ms <= lap.lap_ms {
                return Ok(());
            }
        }
    }
    let lap_num = if lap.is_you {
        next_you_lap(c, id, lap.race_num)
    } else {
        0
    };
    c.execute(
        "INSERT INTO laps (session_id, race_num, lap_num, ms, s1, s2, s3, channels, line, warmup)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(session_id, race_num, lap_num) DO UPDATE SET
           ms = excluded.ms, s1 = excluded.s1, s2 = excluded.s2, s3 = excluded.s3,
           channels = excluded.channels, line = excluded.line, warmup = excluded.warmup",
        params![
            id,
            lap.race_num,
            lap_num,
            lap.lap_ms,
            lap.sectors[0],
            lap.sectors[1],
            lap.sectors[2],
            pack_channels(&lap.bins),
            pack_line(&lap.line),
            if lap.warmup { 1 } else { 0 }
        ],
    )?;
    let best = if lap.crashed { 0 } else { lap.lap_ms };
    c.execute(
        "INSERT INTO riders (session_id, race_num, name, bike, position, best_ms, last_ms)
         VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6)
         ON CONFLICT(session_id, race_num) DO UPDATE SET
           name = excluded.name, bike = excluded.bike,
           best_ms = CASE
             WHEN excluded.best_ms > 0 AND (riders.best_ms = 0 OR excluded.best_ms < riders.best_ms)
             THEN excluded.best_ms ELSE riders.best_ms END,
           last_ms = excluded.last_ms",
        params![id, lap.race_num, lap.name, lap.bike, best, lap.lap_ms],
    )?;
    let _ = c.execute(
        "DELETE FROM crashes WHERE session_id = ?1 AND race_num = ?2 AND lap_num = ?3",
        params![id, lap.race_num, lap_num],
    );
    for mark in &lap.crashes {
        c.execute(
            "INSERT INTO crashes (session_id, race_num, lap_num, x, y, z, contact_num, contact_name)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id,
                lap.race_num,
                lap_num,
                mark.x,
                mark.y,
                mark.z,
                mark.contact_num,
                mark.contact_name
            ],
        )?;
    }
    Ok(())
}

fn load_crashes(c: &Connection, id: i64) -> Vec<(i32, i32, CrashMark)> {
    let Ok(mut stmt) = c.prepare(
        "SELECT race_num, lap_num, x, y, z, contact_num, contact_name FROM crashes WHERE session_id = ?1",
    ) else {
        return Vec::new();
    };
    let Ok(it) = stmt.query_map(params![id], |r| {
        Ok((
            r.get::<_, i32>(0)?,
            r.get::<_, i32>(1)?,
            CrashMark {
                x: r.get(2)?,
                y: r.get(3)?,
                z: r.get(4)?,
                contact_num: r.get(5)?,
                contact_name: r.get(6)?,
            },
        ))
    }) else {
        return Vec::new();
    };
    it.filter_map(|r| r.ok()).collect()
}

fn other_stored_ms(c: &Connection, id: i64, race_num: i32) -> Option<i32> {
    c.query_row(
        "SELECT ms FROM laps WHERE session_id = ?1 AND race_num = ?2 AND lap_num = 0",
        params![id, race_num],
        |r| r.get::<_, i32>(0),
    )
    .ok()
}

fn next_you_lap(c: &Connection, id: i64, race_num: i32) -> i32 {
    c.query_row(
        "SELECT COALESCE(MAX(lap_num), -1) FROM laps WHERE session_id = ?1 AND race_num = ?2",
        params![id, race_num],
        |r| r.get::<_, i32>(0),
    )
    .unwrap_or(-1)
        + 1
}

fn compact_and_prune(c: &Connection) {
    let now = now_secs();
    let cutoff = now - FORTNIGHT_SECS;
    if let Ok(mut stmt) = c.prepare("SELECT id FROM sessions WHERE kept = 0 AND started < ?1") {
        let ids: Vec<i64> = stmt
            .query_map(params![cutoff], |r| r.get(0))
            .ok()
            .map(|it| it.filter_map(|r| r.ok()).collect())
            .unwrap_or_default();
        drop(stmt);
        for id in ids {
            upsert_profile_race(c, id);
        }
    }
    let _ = c.execute(
        "DELETE FROM laps WHERE session_id IN (
            SELECT id FROM sessions WHERE kept = 0 AND started < ?1
         ) AND race_num NOT IN (
            SELECT your_race_num FROM sessions WHERE sessions.id = laps.session_id
            UNION
            SELECT fastest_race_num FROM sessions WHERE sessions.id = laps.session_id
         )",
        params![now - WEEK_SECS],
    );
    let _ = c.execute(
        "DELETE FROM riders WHERE session_id IN (SELECT id FROM sessions WHERE kept = 0 AND started < ?1)",
        params![cutoff],
    );
    let _ = c.execute(
        "DELETE FROM crashes WHERE session_id IN (SELECT id FROM sessions WHERE kept = 0 AND started < ?1)
         OR session_id NOT IN (SELECT id FROM sessions)
         OR (session_id, race_num, lap_num) NOT IN (SELECT session_id, race_num, lap_num FROM laps)",
        params![cutoff],
    );
    let _ = c.execute(
        "DELETE FROM laps WHERE session_id IN (SELECT id FROM sessions WHERE kept = 0 AND started < ?1)",
        params![cutoff],
    );
    let _ = c.execute(
        "DELETE FROM sessions WHERE kept = 0 AND started < ?1",
        params![cutoff],
    );
}

fn upsert_profile_race(c: &Connection, id: i64) {
    let Some(compact) = race_input_from(c, id).and_then(|i| crate::compact_from_input(&i)) else {
        let _ = c.execute(
            "DELETE FROM profile_races WHERE session_id = ?1",
            params![id],
        );
        return;
    };
    let _ = c.execute(
        "INSERT INTO profile_races (
            session_id, started, rider_count, position, your_best_ms, fastest_ms,
            laps, attack_hot, attack_n, air_hot, air_n, name, holeshot, state, penalty_ms
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
         ON CONFLICT(session_id) DO UPDATE SET
            started = excluded.started,
            rider_count = excluded.rider_count,
            position = excluded.position,
            your_best_ms = excluded.your_best_ms,
            fastest_ms = excluded.fastest_ms,
            laps = excluded.laps,
            attack_hot = excluded.attack_hot,
            attack_n = excluded.attack_n,
            air_hot = excluded.air_hot,
            air_n = excluded.air_n,
            name = excluded.name,
            holeshot = excluded.holeshot,
            state = excluded.state,
            penalty_ms = excluded.penalty_ms",
        params![
            compact.session_id,
            compact.started,
            compact.rider_count,
            compact.position,
            compact.your_best_ms,
            compact.fastest_ms,
            crate::pack_laps(&compact.laps),
            compact.attack_hot,
            compact.attack_n,
            compact.air_hot,
            compact.air_n,
            compact.name,
            compact.holeshot,
            compact.state,
            compact.penalty_ms
        ],
    );
}

fn race_input_from(c: &Connection, id: i64) -> Option<crate::RaceInput> {
    let (practice, kind, holeshot): (i32, i32, Option<i32>) = c
        .query_row(
            "SELECT practice, kind, holeshot FROM sessions WHERE id = ?1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .ok()?;
    let detail = load_from(c, id)?;
    let you = detail
        .riders
        .iter()
        .find(|r| r.race_num == detail.your_race_num);
    Some(crate::RaceInput {
        session_id: id,
        started: detail.row.started,
        practice: practice != 0,
        kind,
        rider_count: detail.row.rider_count,
        your_race_num: detail.your_race_num,
        position: you.map(|r| r.position).unwrap_or(0),
        your_best_ms: you.map(|r| r.best_ms).unwrap_or(detail.row.your_best_ms),
        fastest_ms: detail.row.fastest_ms,
        laps: detail
            .laps
            .iter()
            .filter(|l| l.race_num == detail.your_race_num)
            .map(|l| crate::RaceLapInput {
                lap_num: l.lap_num,
                ms: l.lap_ms,
                warmup: l.warmup,
                crashed: l.crashed,
                cut: l.cut,
                bins: l.bins,
            })
            .collect(),
        name: you.map(|r| r.name.clone()).unwrap_or_default(),
        holeshot,
        state: you.map(|r| r.state).unwrap_or(0),
        penalty_ms: you.map(|r| r.penalty_ms.max(0)).unwrap_or(0),
    })
}

fn load_compact_row(
    session_id: i64,
    started: i64,
    rider_count: i32,
    position: i32,
    your_best_ms: i32,
    fastest_ms: i32,
    laps: Vec<u8>,
    attack_hot: i32,
    attack_n: i32,
    air_hot: i32,
    air_n: i32,
    name: String,
    holeshot: Option<i32>,
    state: i32,
    penalty_ms: i32,
) -> crate::CompactRace {
    crate::CompactRace {
        session_id,
        started,
        rider_count,
        position,
        your_best_ms,
        fastest_ms,
        laps: crate::unpack_laps(&laps),
        attack_hot,
        attack_n,
        air_hot,
        air_n,
        name,
        holeshot,
        state,
        penalty_ms,
    }
}

pub fn profile(window: crate::ProfileWindow) -> crate::RiderProfile {
    let st = live();
    let Some(c) = st.conn.as_ref() else {
        return crate::RiderProfile::empty();
    };
    profile_from(c, window, now_secs())
}

fn profile_from(c: &Connection, window: crate::ProfileWindow, now: i64) -> crate::RiderProfile {
    backfill_profile_races(c);
    let cutoff = now - crate::PROFILE_WINDOW_SECS;
    let mut races = Vec::new();
    if let Ok(mut stmt) = c.prepare(
        "SELECT session_id, started, rider_count, position, your_best_ms, fastest_ms,
                laps, attack_hot, attack_n, air_hot, air_n, name, holeshot, state, penalty_ms
         FROM profile_races",
    ) {
        if let Ok(it) = stmt.query_map([], |r| {
            Ok(load_compact_row(
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
                r.get(7)?,
                r.get(8)?,
                r.get(9)?,
                r.get(10)?,
                r.get(11)?,
                r.get(12)?,
                r.get::<_, i32>(13).unwrap_or(0),
                r.get::<_, i32>(14).unwrap_or(0),
            ))
        }) {
            races.extend(it.filter_map(|r| r.ok()));
        }
    }
    let all_time_count = races.len() as i32;
    let mut name = String::new();
    let mut newest = i64::MIN;
    for race in &races {
        if race.started >= newest && !race.name.is_empty() {
            newest = race.started;
            name = race.name.clone();
        }
    }
    let window_races: Vec<&crate::CompactRace> = match window {
        crate::ProfileWindow::TwoWeeks => races.iter().filter(|r| r.started >= cutoff).collect(),
        crate::ProfileWindow::AllTime => races.iter().collect(),
    };
    let holeshots = window_races
        .iter()
        .filter(|r| r.holeshot == Some(1))
        .count() as i32;
    let scores: Vec<[Option<f32>; crate::AXIS_COUNT]> = window_races
        .iter()
        .map(|r| crate::scores_from_compact(r))
        .collect();
    let mut profile = crate::RiderProfile {
        name,
        race_count: window_races.len() as i32,
        all_time_count,
        holeshots,
        scores: crate::mean_scores(&scores),
        ..crate::RiderProfile::empty()
    };
    crate::apply_window_stats(&mut profile, &window_races);
    let cleared_at = profile_cleared_at(c);
    let mut win_ids: std::collections::HashSet<i64> = window_races
        .iter()
        .filter(|r| crate::you_won_race(r.position, r.rider_count))
        .map(|r| r.session_id)
        .collect();
    win_ids.extend(live_eligible_win_ids(
        c,
        cleared_at,
        cutoff,
        matches!(window, crate::ProfileWindow::AllTime),
    ));
    profile.wins = win_ids.len() as i32;
    let rate_n = profile.race_count.max(profile.wins);
    if rate_n > 0 {
        profile.win_rate = Some(profile.wins as f32 / rate_n as f32);
    }
    profile
}

fn live_eligible_win_ids(
    c: &Connection,
    cleared_at: i64,
    cutoff: i64,
    all_time: bool,
) -> std::collections::HashSet<i64> {
    let rows: Vec<(i64, i64)> = c
        .prepare("SELECT id, started FROM sessions")
        .ok()
        .and_then(|mut stmt| {
            stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
                .ok()
                .map(|it| it.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_default();
    let mut out = std::collections::HashSet::new();
    for (id, started) in rows {
        if started <= cleared_at {
            continue;
        }
        if !all_time && started < cutoff {
            continue;
        }
        let Some(input) = race_input_from(c, id) else {
            continue;
        };
        let has_lap = input.laps.iter().any(|l| !l.warmup && l.ms > 0);
        if crate::counts_as_profile_win(
            input.practice,
            input.kind,
            input.rider_count,
            input.position,
            has_lap,
        ) {
            out.insert(id);
        }
    }
    out
}

fn profile_cleared_at(c: &Connection) -> i64 {
    c.query_row(
        "SELECT v FROM profile_meta WHERE k = 'cleared_at'",
        [],
        |r| r.get(0),
    )
    .unwrap_or(0)
}

fn backfill_profile_races(c: &Connection) {
    let cleared_at = profile_cleared_at(c);
    let rows: Vec<(i64, i64)> = c
        .prepare("SELECT id, started FROM sessions")
        .ok()
        .and_then(|mut stmt| {
            stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
                .ok()
                .map(|it| it.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_default();
    for (id, started) in rows {
        if started <= cleared_at {
            continue;
        }
        upsert_profile_race(c, id);
    }
}

fn pack_channels(bins: &[ChannelBin; BINS]) -> Vec<u8> {
    let mut o = Vec::with_capacity(BINS * 36);
    for b in bins {
        o.extend_from_slice(&b.speed.to_le_bytes());
        o.extend_from_slice(&b.y.to_le_bytes());
        o.extend_from_slice(&b.rpm.to_le_bytes());
        o.extend_from_slice(&b.throttle.to_le_bytes());
        o.extend_from_slice(&b.brake.to_le_bytes());
        o.extend_from_slice(&b.lean.to_le_bytes());
        o.extend_from_slice(&b.yaw.to_le_bytes());
        o.extend_from_slice(&b.ms.to_le_bytes());
        o.extend_from_slice(&b.gear.to_le_bytes());
    }
    o
}

fn unpack_channels(raw: Option<&[u8]>) -> [ChannelBin; BINS] {
    let mut bins = [ChannelBin::default(); BINS];
    let Some(raw) = raw else {
        return bins;
    };
    let stride = if raw.len() >= BINS * 36 {
        36
    } else if raw.len() >= BINS * 32 {
        32
    } else {
        return bins;
    };
    for (i, b) in bins.iter_mut().enumerate() {
        let o = i * stride;
        b.speed = f32::from_le_bytes(raw[o..o + 4].try_into().unwrap());
        b.y = f32::from_le_bytes(raw[o + 4..o + 8].try_into().unwrap());
        b.rpm = f32::from_le_bytes(raw[o + 8..o + 12].try_into().unwrap());
        b.throttle = f32::from_le_bytes(raw[o + 12..o + 16].try_into().unwrap());
        b.brake = f32::from_le_bytes(raw[o + 16..o + 20].try_into().unwrap());
        b.lean = f32::from_le_bytes(raw[o + 20..o + 24].try_into().unwrap());
        b.yaw = f32::from_le_bytes(raw[o + 24..o + 28].try_into().unwrap());
        b.ms = i32::from_le_bytes(raw[o + 28..o + 32].try_into().unwrap());
        if stride >= 36 {
            b.gear = i32::from_le_bytes(raw[o + 32..o + 36].try_into().unwrap());
        }
    }
    bins
}

fn pack_line(line: &[(f32, f32, f32)]) -> Vec<u8> {
    let mut o = Vec::with_capacity(4 + line.len() * 12);
    o.extend_from_slice(&(line.len() as u32).to_le_bytes());
    for (x, y, z) in line {
        o.extend_from_slice(&x.to_le_bytes());
        o.extend_from_slice(&y.to_le_bytes());
        o.extend_from_slice(&z.to_le_bytes());
    }
    o
}

fn unpack_line(raw: Option<&[u8]>) -> Vec<(f32, f32, f32)> {
    let Some(raw) = raw else {
        return Vec::new();
    };
    if raw.len() < 4 {
        return Vec::new();
    }
    let n = (u32::from_le_bytes(raw[0..4].try_into().unwrap()) as usize)
        .min(mxbo_hud::location_tape::MAX_LINE);
    let mut v = Vec::with_capacity(n);
    for i in 0..n {
        let o = 4 + i * 12;
        if o + 12 > raw.len() {
            break;
        }
        v.push((
            f32::from_le_bytes(raw[o..o + 4].try_into().unwrap()),
            f32::from_le_bytes(raw[o + 4..o + 8].try_into().unwrap()),
            f32::from_le_bytes(raw[o + 8..o + 12].try_into().unwrap()),
        ));
    }
    v
}

fn pack_poly(s: &Snapshot) -> Vec<u8> {
    let n = s.poly_count.clamp(0, mxbo_hud::shm::MAX_POLY as i32) as usize;
    let mut o = Vec::with_capacity(4 + n * 8);
    o.extend_from_slice(&(n as u32).to_le_bytes());
    for p in s.poly.iter().take(n) {
        o.extend_from_slice(&p.x.to_le_bytes());
        o.extend_from_slice(&p.z.to_le_bytes());
    }
    o
}

fn unpack_poly(raw: Option<&[u8]>) -> Vec<(f32, f32)> {
    let Some(raw) = raw else {
        return Vec::new();
    };
    if raw.len() < 4 {
        return Vec::new();
    }
    let n =
        (u32::from_le_bytes(raw[0..4].try_into().unwrap()) as usize).min(mxbo_hud::shm::MAX_POLY);
    let mut v = Vec::with_capacity(n);
    for i in 0..n {
        let o = 4 + i * 8;
        if o + 8 > raw.len() {
            break;
        }
        v.push((
            f32::from_le_bytes(raw[o..o + 4].try_into().unwrap()),
            f32::from_le_bytes(raw[o + 4..o + 8].try_into().unwrap()),
        ));
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn practice_p1_is_not_profile_win() {
        assert!(!crate::counts_as_profile_win(true, 7, 8, 1, true));
        assert!(crate::counts_as_profile_win(false, 7, 8, 1, true));
        assert!(!crate::counts_as_profile_win(false, crate::WARMUP_KIND, 8, 1, true));
        assert!(!crate::counts_as_profile_win(false, 7, 1, 1, true));
        assert!(!crate::counts_as_profile_win(false, 7, 8, 1, false));
    }

    #[test]
    fn you_won_race_p1_with_field() {
        assert!(crate::you_won_race(1, 8));
        assert!(!crate::you_won_race(1, 1));
        assert!(!crate::you_won_race(2, 8));
        assert!(!crate::you_won_race(0, 8));
    }

    #[test]
    fn live_poly_keeps_same_length_outline() {
        assert!(!should_replace_poly(8, 8));
        assert!(!should_replace_poly(8, 7));
        assert!(!should_replace_poly(2, 2));
        assert!(should_replace_poly(0, 2));
        assert!(should_replace_poly(1, 2));
        assert!(should_replace_poly(8, 9));
    }

    #[test]
    fn pack_channels_roundtrip_includes_gear() {
        let mut bins = [ChannelBin::default(); BINS];
        bins[3].speed = 41.5;
        bins[3].ms = 12_000;
        bins[3].gear = 4;
        let back = unpack_channels(Some(&pack_channels(&bins)));
        assert_eq!(back[3].speed, 41.5);
        assert_eq!(back[3].ms, 12_000);
        assert_eq!(back[3].gear, 4);
    }

    #[test]
    fn unpack_channels_old_32_byte_bins_have_gear_zero() {
        let mut raw = vec![0u8; BINS * 32];
        raw[3 * 32 + 28..3 * 32 + 32].copy_from_slice(&9_000i32.to_le_bytes());
        let back = unpack_channels(Some(&raw));
        assert_eq!(back[3].ms, 9_000);
        assert_eq!(back[3].gear, 0);
    }

    #[test]
    fn pack_roundtrip_line() {
        let src = vec![(1.0, 2.0, 3.0), (4.0, 5.0, 6.0)];
        let back = unpack_line(Some(&pack_line(&src)));
        assert_eq!(back, src);
    }

    #[test]
    fn live_id_none_after_reset_some_after_session_insert() {
        let _g = serial();
        reset();
        assert_eq!(live_id(), None);
        let dir = std::env::temp_dir().join(format!("mxbo-review-live-id-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        let id = {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            insert_session(c, "Hangtown", 5, 2, 7).expect("insert")
        };
        set_live(id, "Hangtown");
        assert_eq!(live_id(), Some(id));
        reset();
        assert_eq!(live_id(), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_hides_sessions_with_no_laps() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-review-list-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            let id = insert_session(c, "Hangtown", 5, 2, 7).expect("insert");
            assert!(!session_has_laps(c, id));
        }
        assert!(list(ListFilter::All).is_empty());
        {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            let id = insert_session(c, "Hangtown", 7, 2, 7).expect("insert");
            let lap = CommittedLap {
                race_num: 2,
                name: "You".into(),
                bike: "FC 450".into(),
                lap_ms: 113_000,
                sectors: [0, 0, 0],
                bins: [ChannelBin::default(); BINS],
                line: vec![(0.0, 0.0, 0.0)],
                is_you: true,
                crashed: false,
                cut: false,
                warmup: false,
                crashes: Vec::new(),
            };
            upsert_lap(c, id, &lap).expect("lap");
        }
        assert_eq!(list(ListFilter::All).len(), 1);
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn same_track_reopens_within_ten_minutes() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-review-resume-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        let first = {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            insert_session(c, "Pala", 5, 14, 9).expect("insert")
        };
        let again = {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            open_session(c, "Pala", 7, 14, 9, false, "").expect("resume")
        };
        assert_eq!(first, again);
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn practice_does_not_resume_into_race() {
        let _g = serial();
        reset();
        let dir =
            std::env::temp_dir().join(format!("mxbo-review-prac-resume-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        let first = {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            insert_typed(c, "Pala", -1, 14, 9, true, "").expect("practice")
        };
        let again = {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            open_session(c, "Pala", 7, 14, 9, false, "").expect("race")
        };
        assert_ne!(first, again);
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_filters_exclude_practice() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-review-filter-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            let race = insert_typed(c, "Hangtown", 7, 2, 7, false, "").expect("race");
            upsert_lap(c, race, &dummy_lap(2, 113_000, true, false)).expect("race lap");
            let prac = insert_typed(c, "Pala", -1, 2, 7, true, "").expect("practice");
            upsert_lap(c, prac, &dummy_lap(2, 120_000, true, false)).expect("prac lap");
            c.execute("UPDATE sessions SET kept = 1 WHERE id = ?1", params![prac])
                .expect("keep practice");
        }
        let all = list(ListFilter::All);
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].track, "Hangtown");
        assert!(list(ListFilter::Ranked).is_empty());
        assert!(list(ListFilter::Saved).is_empty());
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn purge_removes_legacy_practice() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-review-backfill-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            let id = insert_session(c, "Hangtown", 5, 2, 7).expect("old");
            upsert_lap(c, id, &dummy_lap(2, 113_000, true, false)).expect("lap");
            c.execute("UPDATE sessions SET practice = 1 WHERE id = ?1", params![id])
                .expect("mark practice");
        }
        prune();
        assert!(list(ListFilter::All).is_empty());
        assert!(list(ListFilter::Ranked).is_empty());
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tick_practice_does_not_record() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-review-prac-tick-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        let mut s = Snapshot::default();
        let name = b"Hangtown";
        s.track_name[..name.len()].copy_from_slice(name);
        s.has_telemetry = 1;
        s.session_laps = 0;
        s.session_length = 0;
        s.local_race_num = 2;
        s.focus_race_num = 2;
        tick(&s, true);
        assert_eq!(live_id(), None);
        assert!(list(ListFilter::All).is_empty());
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tick_without_telemetry_closes_live() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-review-telem-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        let mut s = Snapshot::default();
        let name = b"Hangtown";
        s.track_name[..name.len()].copy_from_slice(name);
        s.has_telemetry = 1;
        s.session_kind = 7;
        tick(&s, true);
        assert!(live_id().is_some());
        s.has_telemetry = 0;
        tick(&s, true);
        assert_eq!(live_id(), None);
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn dummy_lap(race_num: i32, ms: i32, is_you: bool, crashed: bool) -> CommittedLap {
        CommittedLap {
            race_num,
            name: if is_you { "You" } else { "Cole" }.into(),
            bike: "FC 450".into(),
            lap_ms: ms,
            sectors: [0, 0, 0],
            bins: [ChannelBin::default(); BINS],
            line: vec![(0.0, 0.0, 0.0), (1.0, 0.0, 1.0)],
            is_you,
            crashed,
            cut: false,
            warmup: false,
            crashes: Vec::new(),
        }
    }

    #[test]
    fn your_laps_both_load() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-review-yours-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        let id = {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            let id = insert_session(c, "Hangtown", 7, 2, 7).expect("insert");
            upsert_lap(c, id, &dummy_lap(2, 113_000, true, false)).expect("l1");
            upsert_lap(c, id, &dummy_lap(2, 115_000, true, false)).expect("l2");
            id
        };
        let detail = load(id).expect("load");
        let yours: Vec<i32> = detail
            .laps
            .iter()
            .filter(|l| l.race_num == 2)
            .map(|l| l.lap_ms)
            .collect();
        assert_eq!(yours, vec![113_000, 115_000]);
        assert_eq!(detail.laps[0].lap_num, 0);
        assert_eq!(detail.laps[1].lap_num, 1);
        let again = load(id).expect("load again");
        let yours_again: Vec<i32> = again
            .laps
            .iter()
            .filter(|l| l.race_num == 2)
            .map(|l| l.lap_ms)
            .collect();
        assert_eq!(yours_again, yours);
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bump_other_session_keeps_cached_laps() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-review-bump-iso-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        let (a, b) = {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            let a = insert_session(c, "Hangtown", 7, 2, 7).expect("a");
            upsert_lap(c, a, &dummy_lap(2, 113_000, true, false)).expect("a lap");
            let b = insert_session(c, "Pala", 7, 2, 7).expect("b");
            upsert_lap(c, b, &dummy_lap(2, 101_000, true, false)).expect("b1");
            upsert_lap(c, b, &dummy_lap(2, 99_000, true, false)).expect("b2");
            (a, b)
        };
        let first = load(b).expect("load b");
        assert_eq!(first.laps.len(), 2);
        set_kept(a, true);
        let second = load(b).expect("load b again");
        assert_eq!(second.laps.len(), first.laps.len());
        assert!(
            std::sync::Arc::ptr_eq(&first, &second),
            "live write to A must not unpack B again"
        );
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn other_rider_keeps_fastest_only() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-review-other-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        let id = {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            let id = insert_session(c, "Hangtown", 7, 2, 7).expect("insert");
            upsert_lap(c, id, &dummy_lap(7, 110_000, false, false)).expect("fast");
            upsert_lap(c, id, &dummy_lap(7, 118_000, false, false)).expect("slow");
            upsert_lap(c, id, &dummy_lap(7, 112_000, false, true)).expect("crash");
            id
        };
        let detail = load(id).expect("load");
        let other: Vec<i32> = detail
            .laps
            .iter()
            .filter(|l| l.race_num == 7)
            .map(|l| l.lap_ms)
            .collect();
        assert_eq!(other, vec![110_000]);
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn field_due_same_within_two_seconds_is_false() {
        assert!(!field_due(true, Some(100)));
        assert!(field_due(true, None));
        assert!(field_due(true, Some(2000)));
    }

    #[test]
    fn field_due_rider_count_change_is_true() {
        assert!(field_due(false, Some(50)));
    }

    #[test]
    fn other_rider_cut_is_written_slower_does_not_replace() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-review-cut-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        let id = {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            let id = insert_session(c, "Hangtown", 7, 2, 7).expect("insert");
            let mut cut = dummy_lap(7, 80_000, false, false);
            cut.cut = true;
            upsert_lap(c, id, &cut).expect("cut");
            upsert_lap(c, id, &dummy_lap(7, 110_000, false, false)).expect("slow");
            id
        };
        let detail = load(id).expect("load");
        let other: Vec<i32> = detail
            .laps
            .iter()
            .filter(|l| l.race_num == 7)
            .map(|l| l.lap_ms)
            .collect();
        assert_eq!(other, vec![80_000]);
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn warmup_and_race_laps_split() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-review-wu-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        let id = {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            let id = insert_session(c, "Hangtown", 7, 2, 7).expect("insert");
            let mut wu = dummy_lap(2, 120_000, true, false);
            wu.warmup = true;
            upsert_lap(c, id, &wu).expect("wu");
            upsert_lap(c, id, &dummy_lap(2, 113_000, true, false)).expect("race");
            id
        };
        let detail = load(id).expect("load");
        let wu: Vec<i32> = detail
            .laps
            .iter()
            .filter(|l| l.warmup)
            .map(|l| l.lap_ms)
            .collect();
        let race: Vec<i32> = detail
            .laps
            .iter()
            .filter(|l| !l.warmup)
            .map(|l| l.lap_ms)
            .collect();
        assert_eq!(wu, vec![120_000]);
        assert_eq!(race, vec![113_000]);
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn warmup_writes_your_best_ms() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-review-wu-best-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        let id = {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            let id = insert_session(c, "Hangtown", 7, 2, 7).expect("insert");
            let mut wu = dummy_lap(2, 101_000, true, false);
            wu.warmup = true;
            upsert_lap(c, id, &wu).expect("wu");
            id
        };
        assert_eq!(list(ListFilter::All)[0].your_best_ms, 101_000);
        {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            upsert_lap(c, id, &dummy_lap(2, 113_000, true, false)).expect("slow race");
        }
        assert_eq!(list(ListFilter::All)[0].your_best_ms, 101_000);
        {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            upsert_lap(c, id, &dummy_lap(2, 99_000, true, false)).expect("fast race");
        }
        assert_eq!(list(ListFilter::All)[0].your_best_ms, 99_000);
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn slower_race_keeps_other_warmup_line() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-review-wu-other-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        let id = {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            let id = insert_session(c, "Hangtown", 7, 2, 7).expect("insert");
            let mut wu = dummy_lap(7, 101_000, false, false);
            wu.warmup = true;
            upsert_lap(c, id, &wu).expect("wu");
            upsert_lap(c, id, &dummy_lap(7, 113_000, false, false)).expect("slow race");
            id
        };
        let detail = load(id).expect("load");
        let other: Vec<_> = detail
            .laps
            .iter()
            .filter(|l| l.race_num == 7)
            .map(|l| (l.lap_ms, l.warmup))
            .collect();
        assert_eq!(other, vec![(101_000, true)]);
        assert_eq!(
            detail
                .riders
                .iter()
                .find(|r| r.race_num == 7)
                .map(|r| r.best_ms),
            Some(101_000)
        );
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn faster_race_replaces_other_warmup_line() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-review-wu-rep-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        let id = {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            let id = insert_session(c, "Hangtown", 7, 2, 7).expect("insert");
            let mut wu = dummy_lap(7, 101_000, false, false);
            wu.warmup = true;
            upsert_lap(c, id, &wu).expect("wu");
            upsert_lap(c, id, &dummy_lap(7, 99_000, false, false)).expect("fast race");
            id
        };
        let detail = load(id).expect("load");
        let other: Vec<_> = detail
            .laps
            .iter()
            .filter(|l| l.race_num == 7)
            .map(|l| (l.lap_ms, l.warmup))
            .collect();
        assert_eq!(other, vec![(99_000, false)]);
        assert_eq!(
            detail
                .riders
                .iter()
                .find(|r| r.race_num == 7)
                .map(|r| r.best_ms),
            Some(99_000)
        );
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn delete_wipes_kept_session() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-review-delete-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        let id = {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            let id = insert_session(c, "Hangtown", 7, 2, 7).expect("insert");
            upsert_lap(c, id, &dummy_lap(2, 113_000, true, false)).expect("lap");
            id
        };
        set_kept(id, true);
        assert_eq!(list(ListFilter::All).len(), 1);
        assert_eq!(list(ListFilter::Saved).len(), 1);
        delete(id);
        assert!(list(ListFilter::All).is_empty());
        assert!(load(id).is_none());
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tick_off_session_does_not_open_live() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-review-tick-off-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        let mut s = Snapshot::default();
        let name = b"Hangtown";
        s.track_name[..name.len()].copy_from_slice(name);
        s.has_telemetry = 1;
        s.session_kind = 7;
        tick(&s, false);
        assert_eq!(live_id(), None);
        assert!(list(ListFilter::All).is_empty());
        tick(&s, true);
        assert!(live_id().is_some());
        tick(&s, false);
        assert_eq!(live_id(), None);
        assert!(list(ListFilter::All).is_empty());
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn field_tick_does_not_reload_cached_laps() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-review-field-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        let id = {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            let id = insert_session(c, "Hangtown", 7, 2, 7).expect("insert");
            upsert_lap(c, id, &dummy_lap(2, 113_000, true, false)).expect("you");
            id
        };
        let first = load(id).expect("load");
        assert_eq!(first.laps.len(), 1);
        drop(first);
        set_live(id, "Hangtown");
        let mut s = Snapshot::default();
        let name = b"Hangtown";
        s.track_name[..name.len()].copy_from_slice(name);
        s.has_telemetry = 1;
        s.session_kind = 7;
        s.local_race_num = 2;
        s.standing_count = 2;
        s.standings[0].race_num = 2;
        s.standings[0].position = 2;
        s.standings[0].best_lap_ms = 113_000;
        mxbo_hud::snapshot::write_name(&mut s.standings[0].name, "You");
        s.standings[1].race_num = 7;
        s.standings[1].position = 1;
        s.standings[1].best_lap_ms = 108_000;
        mxbo_hud::snapshot::write_name(&mut s.standings[1].name, "Cole");
        s.poly_count = 2;
        s.poly[1].x = 40.0;
        s.poly[1].z = 10.0;
        s.sf_meters = 12.4;
        tick(&s, true);
        let second = load(id).expect("after field");
        let third = load(id).expect("again");
        assert!(
            std::sync::Arc::ptr_eq(&second, &third),
            "field tick must not drop the Analyze cache"
        );
        assert_eq!(second.laps.len(), 1);
        assert_eq!(second.row.rider_count, 2);
        assert_eq!(second.row.fastest_ms, 108_000);
        assert_eq!(second.poly.len(), 2);
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn field_tick_keeps_same_length_poly() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-review-poly-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        let id = {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            let id = insert_session(c, "Hangtown", 7, 2, 7).expect("insert");
            upsert_lap(c, id, &dummy_lap(2, 113_000, true, false)).expect("you");
            id
        };
        set_live(id, "Hangtown");
        let mut s = Snapshot::default();
        let name = b"Hangtown";
        s.track_name[..name.len()].copy_from_slice(name);
        s.has_telemetry = 1;
        s.session_kind = 7;
        s.local_race_num = 2;
        s.standing_count = 2;
        s.standings[0].race_num = 2;
        s.standings[0].position = 2;
        s.standings[0].best_lap_ms = 113_000;
        mxbo_hud::snapshot::write_name(&mut s.standings[0].name, "You");
        s.standings[1].race_num = 7;
        s.standings[1].position = 1;
        s.standings[1].best_lap_ms = 108_000;
        mxbo_hud::snapshot::write_name(&mut s.standings[1].name, "Cole");
        s.poly_count = 2;
        s.poly[1].x = 40.0;
        s.poly[1].z = 10.0;
        tick(&s, true);
        {
            let d = load(id).expect("first poly");
            assert_eq!(d.poly, vec![(0.0, 0.0), (40.0, 10.0)]);
        }
        s.poly[1].x = 41.0;
        s.standing_count = 3;
        tick(&s, true);
        {
            let d = load(id).expect("jitter");
            assert_eq!(
                d.poly,
                vec![(0.0, 0.0), (40.0, 10.0)],
                "same-length SHM jitter must not replace the outline"
            );
            assert_eq!(d.row.rider_count, 3);
        }
        s.poly_count = 3;
        s.poly[2].x = 80.0;
        s.standing_count = 2;
        tick(&s, true);
        {
            let d = load(id).expect("longer");
            assert_eq!(d.poly.len(), 3);
            assert_eq!(d.poly[2], (80.0, 0.0));
        }
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_skips_other_rider_channels() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-review-ch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        let id = {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            let id = insert_session(c, "Hangtown", 7, 2, 7).expect("insert");
            upsert_lap(c, id, &dummy_lap(2, 113_000, true, false)).expect("you");
            upsert_lap(c, id, &dummy_lap(7, 110_000, false, false)).expect("fast");
            let mut other = dummy_lap(9, 120_000, false, false);
            other.name = "Third".into();
            other.bins = demo_bins(40.0, 1.0, 0.0);
            upsert_lap(c, id, &other).expect("third");
            id
        };
        let detail = load(id).expect("load");
        let third = detail
            .laps
            .iter()
            .find(|l| l.race_num == 9)
            .expect("third lap");
        assert!(bins_blank(&third.bins), "other rider bins stay packed");
        assert!(third.line.is_empty(), "other rider line stays packed");
        drop(detail);
        ensure_line(id, 9);
        let detail = load(id).expect("ensure");
        let third = detail
            .laps
            .iter()
            .find(|l| l.race_num == 9)
            .expect("third after ensure");
        assert!(!bins_blank(&third.bins));
        assert!(!third.line.is_empty());
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unpack_poly_caps_at_max_poly() {
        let n = mxbo_hud::shm::MAX_POLY + 8;
        let mut raw = Vec::with_capacity(4 + n * 8);
        raw.extend_from_slice(&(n as u32).to_le_bytes());
        for i in 0..n {
            raw.extend_from_slice(&(i as f32).to_le_bytes());
            raw.extend_from_slice(&0f32.to_le_bytes());
        }
        let pts = unpack_poly(Some(&raw));
        assert_eq!(pts.len(), mxbo_hud::shm::MAX_POLY);
    }

    fn put_eligible_race(c: &Connection, started: i64, you: i32, laps: &[i32]) -> i64 {
        let id = insert_typed(c, "Hangtown", 7, you, 7, false, "").expect("session");
        c.execute(
            "UPDATE sessions SET rider_count = 8, started = ?1 WHERE id = ?2",
            params![started, id],
        )
        .expect("field");
        for ms in laps {
            upsert_lap(c, id, &dummy_lap(you, *ms, true, false)).expect("lap");
        }
        upsert_lap(
            c,
            id,
            &dummy_lap(
                7,
                laps.iter().min().copied().unwrap_or(100_000) - 500,
                false,
                false,
            ),
        )
        .expect("fastest");
        c.execute(
            "UPDATE riders SET position = 3 WHERE session_id = ?1 AND race_num = ?2",
            params![id, you],
        )
        .ok();
        c.execute(
            "UPDATE riders SET position = 1 WHERE session_id = ?1 AND race_num = 7",
            params![id],
        )
        .ok();
        id
    }

    #[test]
    fn profile_skips_practice_warmup_and_solo() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-profile-skip-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            let prac = insert_typed(c, "Pala", -1, 2, 7, true, "").expect("prac");
            c.execute(
                "UPDATE sessions SET rider_count = 12 WHERE id = ?1",
                params![prac],
            )
            .ok();
            upsert_lap(c, prac, &dummy_lap(2, 110_000, true, false)).expect("p");
            let wu = insert_typed(c, "Pala", crate::WARMUP_KIND, 2, 7, false, "").expect("wu");
            c.execute(
                "UPDATE sessions SET rider_count = 12 WHERE id = ?1",
                params![wu],
            )
            .ok();
            upsert_lap(c, wu, &dummy_lap(2, 110_000, true, false)).expect("w");
            let solo = insert_typed(c, "Pala", 7, 2, 2, false, "").expect("solo");
            c.execute(
                "UPDATE sessions SET rider_count = 1 WHERE id = ?1",
                params![solo],
            )
            .ok();
            upsert_lap(c, solo, &dummy_lap(2, 110_000, true, false)).expect("s");
        }
        let p = profile(crate::ProfileWindow::AllTime);
        assert_eq!(p.all_time_count, 0);
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn prune_compacts_eligible_race_for_all_time() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-profile-prune-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        let old = now_secs() - FORTNIGHT_SECS - 60;
        {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            put_eligible_race(c, old, 2, &[113_000, 114_000, 112_500, 113_400]);
        }
        prune();
        assert!(list(ListFilter::All).is_empty());
        let all = profile(crate::ProfileWindow::AllTime);
        assert_eq!(all.all_time_count, 1);
        assert_eq!(all.race_count, 1);
        assert!(all.scores[0].is_some());
        let two = profile(crate::ProfileWindow::TwoWeeks);
        assert_eq!(two.race_count, 0);
        assert_eq!(two.all_time_count, 1);
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn motos_delete_drops_profile_rollup() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-profile-del-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        let old = now_secs() - FORTNIGHT_SECS - 60;
        let id = {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            put_eligible_race(c, old, 2, &[113_000, 114_000, 112_500, 113_400])
        };
        prune();
        assert_eq!(profile(crate::ProfileWindow::AllTime).all_time_count, 1);
        delete(id);
        assert_eq!(profile(crate::ProfileWindow::AllTime).all_time_count, 0);
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn profile_counts_holeshot_from_compact() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-profile-hs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            let id = put_eligible_race(c, now_secs(), 2, &[113_000, 114_000, 112_500, 113_400]);
            c.execute(
                "UPDATE sessions SET holeshot = 1 WHERE id = ?1",
                params![id],
            )
            .expect("hs");
        }
        let p = profile(crate::ProfileWindow::AllTime);
        assert_eq!(p.holeshots, 1);
        assert_eq!(p.name, "You");
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_keeps_eligible_race_with_you_won_post_pass() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-list-win-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            let id = put_eligible_race(c, now_secs(), 2, &[113_000, 114_000, 112_500, 113_400]);
            c.execute(
                "UPDATE riders SET position = 1 WHERE session_id = ?1 AND race_num = 2",
                params![id],
            )
            .expect("p1");
        }
        let rows = list(ListFilter::All);
        assert_eq!(rows.len(), 1);
        assert!(rows[0].you_won);
        assert_eq!(rows[0].your_position, 1);
        assert_eq!(profile(crate::ProfileWindow::AllTime).wins, 1);
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn practice_p1_list_has_no_crown() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-prac-crown-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            let id = insert_typed(c, "Pala", 7, 2, 7, true, "").expect("prac");
            c.execute(
                "UPDATE sessions SET rider_count = 8 WHERE id = ?1",
                params![id],
            )
            .ok();
            upsert_lap(c, id, &dummy_lap(2, 110_000, true, false)).expect("lap");
            c.execute(
                "UPDATE riders SET position = 1 WHERE session_id = ?1 AND race_num = 2",
                params![id],
            )
            .expect("p1");
            assert!(!session_you_won(c, id, 8));
        }
        assert!(list(ListFilter::All).is_empty());
        assert_eq!(profile(crate::ProfileWindow::AllTime).wins, 0);
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn profile_backfill_refreshes_stale_win() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-profile-refresh-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        let id = {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            let id = put_eligible_race(c, now_secs(), 2, &[113_000, 114_000, 112_500, 113_400]);
            c.execute(
                "UPDATE riders SET position = 2 WHERE session_id = ?1 AND race_num = 2",
                params![id],
            )
            .expect("p2");
            upsert_profile_race(c, id);
            c.execute(
                "UPDATE riders SET position = 1 WHERE session_id = ?1 AND race_num = 2",
                params![id],
            )
            .expect("p1");
            id
        };
        assert!(crate::you_won_race(1, 8));
        let _ = id;
        let p = profile(crate::ProfileWindow::AllTime);
        assert_eq!(p.wins, 1);
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn profile_aggregates_results_state_and_penalty() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-profile-agg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            let win = put_eligible_race(c, now_secs(), 2, &[113_000, 114_000, 112_500, 113_400]);
            c.execute(
                "UPDATE riders SET position = 1, state = 0, penalty_ms = 2000
                 WHERE session_id = ?1 AND race_num = 2",
                params![win],
            )
            .expect("win");
            c.execute(
                "UPDATE sessions SET holeshot = 1 WHERE id = ?1",
                params![win],
            )
            .ok();
            upsert_profile_race(c, win);

            let dnf = put_eligible_race(
                c,
                now_secs() - 10,
                2,
                &[115_000, 116_000, 114_500, 115_400],
            );
            c.execute(
                "UPDATE riders SET position = 8, state = 3, penalty_ms = 0
                 WHERE session_id = ?1 AND race_num = 2",
                params![dnf],
            )
            .expect("dnf");
            c.execute(
                "UPDATE sessions SET holeshot = 0 WHERE id = ?1",
                params![dnf],
            )
            .ok();
            upsert_profile_race(c, dnf);
        }
        let p = profile(crate::ProfileWindow::AllTime);
        assert_eq!(p.race_count, 2);
        assert_eq!(p.wins, 1);
        assert!((p.win_rate.unwrap() - 0.5).abs() < 0.001);
        assert_eq!(p.dnf_count, 1);
        assert!((p.holeshot_rate.unwrap() - 0.5).abs() < 0.001);
        assert!((p.avg_penalty_ms.unwrap() - 1000.0).abs() < 0.001);
        assert_eq!(p.avg_finish, Some(1.0)); // DNF excluded from finish avg
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn upsert_field_persists_server_name() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-server-name-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            let id = insert_typed(c, "Hangtown", 7, 2, 7, false, "").expect("session");
            upsert_lap(c, id, &dummy_lap(2, 113_000, true, false)).expect("lap");
        }
        assert_eq!(list(ListFilter::All)[0].server_name, "");
        let mut s = Snapshot::default();
        let name = b"Hangtown";
        s.track_name[..name.len()].copy_from_slice(name);
        let server = b"Ranked MX #1";
        s.server_name[..server.len()].copy_from_slice(server);
        s.has_telemetry = 1;
        s.local_race_num = 2;
        s.session_kind = 7;
        s.standing_count = 2;
        s.standings[0].race_num = 2;
        s.standings[1].race_num = 7;
        tick(&s, true);
        let id = live_id().expect("live");
        let stored: String = {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            c.query_row(
                "SELECT server_name FROM sessions WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .expect("server")
        };
        assert_eq!(stored, "Ranked MX #1");
        let detail = load(id).expect("load");
        assert_eq!(detail.row.server_name, "Ranked MX #1");
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn init_seeds_ranked_servers_allowlist() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-ranked-servers-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        let st = live();
        let c = st.conn.as_ref().expect("conn");
        let count: i64 = c
            .query_row("SELECT COUNT(*) FROM ranked_servers", [], |r| r.get(0))
            .expect("count");
        assert_eq!(count, RANKED_LOBBY_IDS.len() as i64);
        for id in RANKED_LOBBY_IDS {
            let found: i64 = c
                .query_row(
                    "SELECT lobby_id FROM ranked_servers WHERE lobby_id = ?1",
                    params![id],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            assert_eq!(found, *id, "missing lobby {id}");
        }
        let freebies: Vec<(i64, String, String, String)> = c
            .prepare(
                "SELECT lobby_id, series, class, region FROM ranked_servers
                 WHERE lobby_id IN (105019, 105020, 105005) ORDER BY lobby_id",
            )
            .expect("prep")
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .expect("q")
            .map(|r| r.expect("row"))
            .collect();
        assert_eq!(
            freebies,
            vec![
                (105005, "MX Freebies".into(), "250".into(), "NZ".into()),
                (105019, "MX Freebies".into(), "250".into(), "EU".into()),
                (105020, "MX Freebies".into(), "250".into(), "NA".into()),
            ]
        );
        drop(st);
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn lobby_id_from_server_name_parses_hash() {
        assert_eq!(
            lobby_id_from_server_name("MXB-Ranked.com | MX Freebies | 250 | S3- | EU | #105019"),
            Some(105019)
        );
        assert_eq!(lobby_id_from_server_name("#105020"), Some(105020));
        assert_eq!(lobby_id_from_server_name(""), None);
        assert_eq!(lobby_id_from_server_name("offline practice"), None);
        assert_eq!(lobby_id_from_server_name("no hash 105019"), None);
    }

    #[test]
    fn list_marks_ranked_when_server_lobby_in_allowlist() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-ranked-flag-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            let ranked = insert_typed(
                c,
                "Norwood",
                7,
                2,
                7,
                false,
                "MXB-Ranked.com | MX Freebies | 250 | S3- | EU | #105019",
            )
            .expect("ranked session");
            upsert_lap(c, ranked, &dummy_lap(2, 113_000, true, false)).expect("lap");
            let casual = insert_typed(c, "Hangtown", 7, 2, 7, false, "Some casual server").expect("casual");
            upsert_lap(c, casual, &dummy_lap(2, 114_000, true, false)).expect("lap");
            let offline = insert_typed(c, "Millville", 7, 2, 7, false, "").expect("offline");
            upsert_lap(c, offline, &dummy_lap(2, 115_000, true, false)).expect("lap");
        }
        let rows = list(ListFilter::All);
        let by_track = |t: &str| rows.iter().find(|r| r.track == t).expect(t);
        assert!(by_track("Norwood").ranked);
        assert!(!by_track("Hangtown").ranked);
        assert!(!by_track("Millville").ranked);
        let detail = load(by_track("Norwood").id).expect("load");
        assert!(detail.row.ranked);
        let ranked_only = list(ListFilter::Ranked);
        assert_eq!(ranked_only.len(), 1);
        assert_eq!(ranked_only[0].track, "Norwood");
        assert!(ranked_only[0].ranked);
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn upsert_field_persists_state_and_penalty() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-profile-pen-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        let mut s = Snapshot::default();
        let name = b"Hangtown";
        s.track_name[..name.len()].copy_from_slice(name);
        s.has_telemetry = 1;
        s.local_race_num = 2;
        s.session_kind = 7;
        s.standing_count = 2;
        s.standings[0].race_num = 2;
        s.standings[0].position = 1;
        s.standings[0].state = 0;
        s.standings[0].penalty_ms = 5000;
        s.standings[1].race_num = 7;
        s.standings[1].position = 2;
        tick(&s, true);
        let id = live_id().expect("live");
        let (state, pen): (i32, i32) = {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            c.query_row(
                "SELECT state, penalty_ms FROM riders WHERE session_id = ?1 AND race_num = 2",
                params![id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .expect("row")
        };
        assert_eq!(state, 0);
        assert_eq!(pen, 5000);
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tick_freezes_holeshot_once() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-profile-hs-tick-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        let mut s = Snapshot::default();
        let name = b"Hangtown";
        s.track_name[..name.len()].copy_from_slice(name);
        s.has_telemetry = 1;
        s.local_race_num = 2;
        s.session_kind = 7;
        s.standing_count = 2;
        s.standings[0].race_num = 2;
        s.standings[1].race_num = 7;
        s.holeshot_race_num = 2;
        tick(&s, true);
        let id = live_id().expect("live");
        let hs: Option<i32> = {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            c.query_row(
                "SELECT holeshot FROM sessions WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .expect("hs")
        };
        assert_eq!(hs, Some(1));
        s.holeshot_race_num = 7;
        tick(&s, true);
        let hs2: Option<i32> = {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            c.query_row(
                "SELECT holeshot FROM sessions WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .expect("hs2")
        };
        assert_eq!(hs2, Some(1));
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn profile_reads_compact_after_sessions_gone() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-profile-gone-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        let old = now_secs() - FORTNIGHT_SECS - 60;
        {
            let st = live();
            let c = st.conn.as_ref().expect("conn");
            put_eligible_race(c, old, 2, &[113_000, 114_000, 112_500, 113_400]);
        }
        prune();
        assert!(list(ListFilter::All).is_empty());
        let p = profile(crate::ProfileWindow::AllTime);
        assert_eq!(p.all_time_count, 1);
        assert!(p.scores[0].is_some());
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn clear_profile_leaves_motos() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-clear-profile-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        seed_demo().expect("demo");
        assert!(profile(crate::ProfileWindow::AllTime).all_time_count > 0);
        assert!(!list(ListFilter::All).is_empty());
        clear_profile();
        assert_eq!(profile(crate::ProfileWindow::AllTime).all_time_count, 0);
        assert!(!list(ListFilter::All).is_empty());
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn clear_motos_leaves_profile() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-clear-motos-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        seed_demo().expect("demo");
        let n = profile(crate::ProfileWindow::AllTime).all_time_count;
        assert!(n > 0);
        clear_motos();
        assert!(list(ListFilter::All).is_empty());
        assert_eq!(profile(crate::ProfileWindow::AllTime).all_time_count, n);
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn clear_motos_keeps_live() {
        let _g = serial();
        reset();
        let dir = std::env::temp_dir().join(format!("mxbo-clear-live-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        let id = seed_demo().expect("demo");
        set_live(id, "Hangtown");
        let before = list(ListFilter::All).len();
        assert!(before > 1);
        clear_motos();
        let rows = list(ListFilter::All);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, id);
        assert_eq!(live_id(), Some(id));
        assert!(profile(crate::ProfileWindow::AllTime).all_time_count > 0);
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }
}
