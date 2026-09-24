//! WASM / web: in-memory demo session only (no sqlite).

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
#[cfg(not(target_arch = "wasm32"))]
use std::time::{SystemTime, UNIX_EPOCH};

use mxbo_hud::location_tape::{ChannelBin, BINS};
use mxbo_hud::snapshot::Snapshot;

#[derive(Clone, Debug)]
pub struct SessionRow {
    pub id: i64,
    pub started: i64,
    pub track: String,
    pub server_name: String,
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
    pub has_line: bool,
}

#[derive(Clone, Debug)]
pub struct LapView {
    pub race_num: i32,
    pub lap_num: i32,
    pub lap_ms: i32,
    pub sectors: [i32; 3],
    pub bins: [ChannelBin; BINS],
    pub line: Vec<(f32, f32, f32)>,
    pub crashed: bool,
    pub cut: bool,
    pub warmup: bool,
    pub crashes: Vec<mxbo_hud::location_tape::CrashMark>,
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

static WEB_DEMO: OnceLock<SessionDetail> = OnceLock::new();

pub fn install_web_demo(detail: SessionDetail) {
    let _ = WEB_DEMO.set(detail);
}

pub fn init(_dir: PathBuf) {}

pub fn reset() {}

pub fn tick(_s: &Snapshot, _in_session: bool) {}

pub fn live_id() -> Option<i64> {
    None
}

pub fn list(_filter: ListFilter) -> Vec<SessionRow> {
    Vec::new()
}

pub fn profile(_window: crate::ProfileWindow) -> crate::RiderProfile {
    crate::RiderProfile::empty()
}

pub fn clear_profile() {}

pub fn clear_motos() {}

pub fn load(id: i64) -> Option<Arc<SessionDetail>> {
    WEB_DEMO
        .get()
        .filter(|d| d.row.id == id)
        .map(|d| Arc::new(d.clone()))
}

pub fn ensure_line(_id: i64, _race_num: i32) {}

fn demo_now_secs() -> i64 {
    #[cfg(target_arch = "wasm32")]
    {
        return 1_704_067_200; // fixed UTC for web demo (no std::time on wasm32)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }
}

pub fn demo_session() -> SessionDetail {
    let now = demo_now_secs();
    let poly = kidney_poly(96, 92.0, 58.0);
    let you_line = offset_line(&poly, 3.4, 0.4);
    let other_line = offset_line(&poly, -3.6, 0.9);
    let you_bins = demo_bins(48.2, 1.4, 0.08);
    let other_bins = demo_bins(50.6, 1.1, -0.05);
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
        riders: vec![
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
        ],
        laps: vec![
            LapView {
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
            },
            LapView {
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
            },
        ],
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
