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
    pub rider_count: i32,
    pub your_best_ms: i32,
    pub fastest_name: String,
    pub fastest_ms: i32,
    pub kept: bool,
    pub you_won: bool,
}

pub fn you_won_race(position: i32, rider_count: i32) -> bool {
    position == 1 && rider_count > 1
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListFilter {
    All,
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

fn bins_blank(bins: &[ChannelBin; BINS]) -> bool {
    !bins
        .iter()
        .any(|b| b.ms > 0 || b.speed != 0.0 || b.y != 0.0)
}

fn poly_from_snap(s: &Snapshot) -> Vec<(f32, f32)> {
    let n = s.poly_count.clamp(0, mxbo_hud::shm::MAX_POLY as i32) as usize;
    s.poly.iter().take(n).map(|p| (p.x, p.z)).collect()
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
    d.poly = poly_from_snap(s);
    d.row.rider_count = n as i32;
    let has_line: Vec<(i32, bool)> = d.riders.iter().map(|r| (r.race_num, r.has_line)).collect();
    d.riders = s
        .standings
        .iter()
        .take(n)
        .filter(|row| row.race_num > 0)
        .map(|row| RiderRow {
            race_num: row.race_num,
            name: cstr(&row.name),
            bike: cstr(&row.bike),
            position: row.position,
            best_ms: row.best_lap_ms,
            last_ms: row.last_lap_ms,
            has_line: has_line
                .iter()
                .find(|(n, _)| *n == row.race_num)
                .map(|(_, h)| *h)
                .unwrap_or_else(|| {
                    d.laps
                        .iter()
                        .any(|l| l.race_num == row.race_num && !l.line.is_empty())
                }),
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

#[cfg(test)]
pub fn set_live(id: i64, track: &str) {
    live().live = Some(Live {
        id,
        track: track.to_string(),
    });
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
pub fn reset() {
    *live() = Store::empty();
}

#[cfg(test)]
pub fn serial() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
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
    let mut st = live();
    st.conn = Some(conn);
    drop(st);
    prune();
}

pub fn tick(s: &Snapshot, in_session: bool) {
    if !in_session {
        let mut st = live();
        if let Some(open) = st.live.take() {
            if let Some(c) = st.conn.as_ref() {
                delete_if_empty(c, open.id);
                compact_and_prune(c);
            }
            location_tape::reset();
            bump(&mut st, Some(open.id));
        }
        return;
    }
    location_tape::tick(s);
    let commits = location_tape::take_commits();
    let track = cstr(&s.track_name);
    if track.is_empty() {
        return;
    }
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
            Some(l) => l.track != track,
        };
        if need_new {
            if let Some(old) = st.live.take() {
                if let Some(c) = st.conn.as_ref() {
                    delete_if_empty(c, old.id);
                }
                bump(&mut st, Some(old.id));
            }
            if let Some(c) = st.conn.as_ref() {
                if let Ok(id) = open_session(c, &track, s.session_kind, you, fastest) {
                    st.live = Some(Live {
                        id,
                        track: track.clone(),
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
        ListFilter::All => {
            "SELECT s.id, s.started, s.track, s.rider_count, s.fastest_race_num, s.your_race_num, s.kept,
                    (SELECT MIN(best_ms) FROM riders r WHERE r.session_id = s.id AND r.race_num = s.your_race_num),
                    (SELECT name FROM riders r WHERE r.session_id = s.id AND r.race_num = s.fastest_race_num),
                    (SELECT MIN(best_ms) FROM riders r WHERE r.session_id = s.id AND r.race_num = s.fastest_race_num),
                    (SELECT position FROM riders r WHERE r.session_id = s.id AND r.race_num = s.your_race_num)
             FROM sessions s
             WHERE EXISTS (SELECT 1 FROM laps l WHERE l.session_id = s.id)
             ORDER BY s.started DESC LIMIT 200"
        }
        ListFilter::Saved => {
            "SELECT s.id, s.started, s.track, s.rider_count, s.fastest_race_num, s.your_race_num, s.kept,
                    (SELECT MIN(best_ms) FROM riders r WHERE r.session_id = s.id AND r.race_num = s.your_race_num),
                    (SELECT name FROM riders r WHERE r.session_id = s.id AND r.race_num = s.fastest_race_num),
                    (SELECT MIN(best_ms) FROM riders r WHERE r.session_id = s.id AND r.race_num = s.fastest_race_num),
                    (SELECT position FROM riders r WHERE r.session_id = s.id AND r.race_num = s.your_race_num)
             FROM sessions s
             WHERE s.kept = 1 AND EXISTS (SELECT 1 FROM laps l WHERE l.session_id = s.id)
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
            fastest_name: row.get::<_, Option<String>>(8)?.unwrap_or_default(),
            fastest_ms: row.get::<_, Option<i32>>(9)?.unwrap_or(0),
            your_best_ms: row.get::<_, Option<i32>>(7)?.unwrap_or(0),
            kept: row.get::<_, i32>(6)? != 0,
            you_won: you_won_race(
                row.get::<_, Option<i32>>(10)?.unwrap_or(0),
                row.get(3)?,
            ),
        })
    });
    match rows {
        Ok(it) => it.filter_map(|r| r.ok()).collect(),
        Err(_) => Vec::new(),
    }
}

pub fn load(id: i64) -> Option<Arc<SessionDetail>> {
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
            "SELECT id, started, track, rider_count, fastest_race_num, your_race_num, kept, poly, sf_meters
             FROM sessions WHERE id = ?1",
            params![id],
            |r| {
                Ok((
                    SessionRow {
                        id: r.get(0)?,
                        started: r.get(1)?,
                        track: r.get(2)?,
                        rider_count: r.get(3)?,
                        fastest_name: String::new(),
                        fastest_ms: 0,
                        your_best_ms: 0,
                        kept: r.get::<_, i32>(6)? != 0,
                        you_won: false,
                    },
                    r.get::<_, i32>(4)?,
                    r.get::<_, i32>(5)?,
                    r.get::<_, Option<Vec<u8>>>(7)?,
                    r.get::<_, f32>(8).unwrap_or(-1.0),
                ))
            },
        )
        .optional()
        .ok()
        .flatten()?;
    let (mut row, fastest_race_num, your_race_num, poly_blob, sf_meters) = row;
    let mut riders = Vec::new();
    if let Ok(mut stmt) = c.prepare(
        "SELECT race_num, name, bike, position, best_ms, last_ms,
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
                has_line: r.get::<_, i32>(6)? != 0,
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
        row.you_won = you_won_race(y.position, row.rider_count);
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
        let _ = c.execute("DELETE FROM crashes WHERE session_id = ?1", params![id]);
        let _ = c.execute("DELETE FROM riders WHERE session_id = ?1", params![id]);
        let _ = c.execute("DELETE FROM laps WHERE session_id = ?1", params![id]);
        let _ = c.execute("DELETE FROM sessions WHERE id = ?1", params![id]);
    }
    if st.live.as_ref().is_some_and(|l| l.id == id) {
        st.live = None;
    }
    bump(&mut st, Some(id));
}

pub fn prune() {
    let mut st = live();
    if let Some(c) = st.conn.as_ref() {
        compact_and_prune(c);
    }
    let cached = st.cached.as_ref().map(|(id, _, _)| *id);
    bump(&mut st, cached);
}

/// Synthetic motos for Review list / Analyze dumps. Returns the first session id.
#[cfg(test)]
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
        }
    }
    bump(&mut st, first);
    first
}

#[cfg(test)]
fn kidney_poly(n: usize, rx: f32, rz: f32) -> Vec<(f32, f32)> {
    (0..n)
        .map(|i| {
            let t = i as f32 / n as f32 * std::f32::consts::TAU;
            let k = 0.72 + 0.28 * (2.0 * t).sin();
            (rx * t.cos(), rz * t.sin() * k)
        })
        .collect()
}

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
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
) -> rusqlite::Result<i64> {
    let recent = c
        .query_row(
            "SELECT id, started FROM sessions WHERE track = ?1 ORDER BY started DESC LIMIT 1",
            params![track],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)),
        )
        .optional()?;
    if let Some((id, started)) = recent {
        if now_secs() - started <= RESUME_SECS {
            return Ok(id);
        }
    }
    insert_session(c, track, kind, you, fastest)
}

fn insert_session(
    c: &Connection,
    track: &str,
    kind: i32,
    you: i32,
    fastest: i32,
) -> rusqlite::Result<i64> {
    c.execute(
        "INSERT INTO sessions (started, track, kind, your_race_num, fastest_race_num) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![now_secs(), track, kind, you, fastest],
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

fn delete_if_empty(c: &Connection, id: i64) {
    if session_has_laps(c, id) {
        return;
    }
    let _ = c.execute("DELETE FROM crashes WHERE session_id = ?1", params![id]);
    let _ = c.execute("DELETE FROM riders WHERE session_id = ?1", params![id]);
    let _ = c.execute("DELETE FROM laps WHERE session_id = ?1", params![id]);
    let _ = c.execute(
        "DELETE FROM sessions WHERE id = ?1 AND kept = 0",
        params![id],
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
        "UPDATE sessions SET rider_count = ?1, your_race_num = ?2, fastest_race_num = ?3, poly = ?4, sf_meters = ?5 WHERE id = ?6",
        params![n as i32, you, fastest, pack_poly(s), s.sf_meters, id],
    )?;
    for row in s.standings.iter().take(n) {
        if row.race_num <= 0 {
            continue;
        }
        c.execute(
            "INSERT INTO riders (session_id, race_num, name, bike, position, best_ms, last_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(session_id, race_num) DO UPDATE SET
               name = excluded.name, bike = excluded.bike, position = excluded.position,
               best_ms = excluded.best_ms, last_ms = excluded.last_ms",
            params![
                id,
                row.race_num,
                cstr(&row.name),
                cstr(&row.bike),
                row.position,
                row.best_lap_ms,
                row.last_lap_ms
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
        params![now - FORTNIGHT_SECS],
    );
    let _ = c.execute(
        "DELETE FROM crashes WHERE session_id IN (SELECT id FROM sessions WHERE kept = 0 AND started < ?1)
         OR session_id NOT IN (SELECT id FROM sessions)
         OR (session_id, race_num, lap_num) NOT IN (SELECT session_id, race_num, lap_num FROM laps)",
        params![now - FORTNIGHT_SECS],
    );
    let _ = c.execute(
        "DELETE FROM laps WHERE session_id IN (SELECT id FROM sessions WHERE kept = 0 AND started < ?1)",
        params![now - FORTNIGHT_SECS],
    );
    let _ = c.execute(
        "DELETE FROM sessions WHERE kept = 0 AND started < ?1",
        params![now - FORTNIGHT_SECS],
    );
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
    fn you_won_race_p1_with_field() {
        assert!(you_won_race(1, 8));
        assert!(!you_won_race(1, 1));
        assert!(!you_won_race(2, 8));
        assert!(!you_won_race(0, 8));
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
            open_session(c, "Pala", 7, 14, 9).expect("resume")
        };
        assert_eq!(first, again);
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
}
