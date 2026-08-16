mod demo_track;

use mxbo_hud::config::HudConfig;
use mxbo_hud::render::{draw, Fonts};
use mxbo_hud::snapshot::{
    write_name, Point, Rider, Snapshot, Standing, MAGIC, MAX_POLY, VERSION,
};
use tiny_skia::Pixmap;
use wasm_bindgen::prelude::*;

const RIDERS: &[(&str, &str, &str)] = &[
    ("Eli Tomac", "YZ450F", "MX1"),
    ("Jett Lawrence", "CRF450R", "MX1"),
    ("Chase Sexton", "FC 450", "MX1"),
    ("Cooper Webb", "450 SX-F", "MX1"),
    ("Aaron Plessinger", "KX450", "MX1"),
    ("Ken Roczen", "RM-Z450", "MX1"),
    ("Justin Barcia", "MC 450", "MX1"),
    ("Dylan Ferrandis", "YZ450F", "MX1"),
    ("Hunter Lawrence", "CRF450R", "MX1"),
    ("Jason Anderson", "KX450", "MX1"),
    ("You", "FC 450", "MX1"),
    ("Haiden Deegan", "YZ250F", "MX2"),
];

const FOCUS: i32 = 11;

#[wasm_bindgen]
pub struct Preview {
    fonts: Fonts,
    cfg: HudConfig,
    snap: Snapshot,
    t: f32,
}

#[wasm_bindgen]
impl Preview {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<Preview, JsValue> {
        let fonts = Fonts::load().ok_or_else(|| JsValue::from_str("failed to load fonts"))?;
        let cfg = HudConfig::new();
        let mut snap = demo_snapshot();
        cfg.apply_to_snapshot(&mut snap);
        Ok(Self {
            fonts,
            cfg,
            snap,
            t: 0.0,
        })
    }

    pub fn set_widget(&mut self, name: &str, on: bool) {
        match name {
            "standings" => self.cfg.show_standings = on,
            "relative" => self.cfg.show_relative = on,
            "dash" => self.cfg.show_dash = on,
            "map" => self.cfg.show_map = on,
            "minimap" => self.cfg.show_minimap = on,
            "radar" => self.cfg.show_radar = on,
            _ => {}
        }
        self.cfg.apply_to_snapshot(&mut self.snap);
    }

    pub fn widget_on(&self, name: &str) -> bool {
        match name {
            "standings" => self.cfg.show_standings,
            "relative" => self.cfg.show_relative,
            "dash" => self.cfg.show_dash,
            "map" => self.cfg.show_map,
            "minimap" => self.cfg.show_minimap,
            "radar" => self.cfg.show_radar,
            _ => false,
        }
    }

    pub fn tick(&mut self, dt: f32) {
        self.t += dt.max(0.0).min(0.08);
        animate(&mut self.snap, self.t, dt.max(0.0).min(0.08));
        self.cfg.apply_to_snapshot(&mut self.snap);
    }

    pub fn frame(&mut self, width: u32, height: u32) -> Vec<u8> {
        let w = width.clamp(320, 1920);
        let h = height.clamp(180, 1080);
        let mut px = Pixmap::new(w, h).expect("pixmap");
        draw(
            &mut px,
            &self.fonts,
            Some(&self.snap),
            &self.cfg,
            w,
            h,
            0.0,
            false,
            false,
        );
        px.data().to_vec()
    }
}

fn demo_snapshot() -> Snapshot {
    let mut s = Snapshot::default();
    s.magic = MAGIC;
    s.version = VERSION;
    s.on_track = 1;
    s.has_telemetry = 1;
    s.local_race_num = FOCUS;
    s.focus_race_num = FOCUS;
    s.max_rpm = 13500;
    s.shift_rpm = 11800;
    s.engine_temp = 89.0;
    s.air_temp = 24.0;
    s.session_laps = 12;
    s.session_length = 45 * 60;
    s.session_time_ms = 20 * 60 * 1000;
    s.best_lap_ms = 72_140;
    s.last_lap_ms = 73_220;
    s.current_lap = 8;
    s.local_gear = 3;
    s.local_speed = 18.0;
    let (poly, length, sf, name) = captured_track();
    write_name(&mut s.track_name, &name);
    s.poly_count = poly.len() as i32;
    for (i, p) in poly.iter().enumerate() {
        s.poly[i] = *p;
    }
    s.track_length = length;
    s.sf_meters = sf;

    s.rider_count = RIDERS.len() as i32;
    for (i, (name, _, _)) in RIDERS.iter().enumerate() {
        let pos = (i as f32 / RIDERS.len() as f32 + 0.08).fract();
        let (x, z, yaw) = sample_track(&s, pos);
        s.riders[i] = Rider {
            race_num: i as i32 + 1,
            x,
            z,
            yaw,
            track_pos: pos,
            crashed: 0,
            name: [0; 32],
        };
        write_name(&mut s.riders[i].name, name);
    }
    refresh_standings(&mut s);
    s
}

fn captured_track() -> (Vec<Point>, f32, f32, String) {
    if demo_track::POLY.len() >= 8 {
        let poly = demo_track::POLY
            .iter()
            .map(|(x, z)| Point { x: *x, z: *z })
            .collect();
        let name = if demo_track::TRACK_NAME.is_empty() {
            "RedBud".into()
        } else {
            demo_track::TRACK_NAME.into()
        };
        return (poly, demo_track::TRACK_LENGTH, demo_track::SF_METERS, name);
    }
    let (poly, length) = redbud_poly();
    (poly, length, 0.0, "RedBud".into())
}

fn redbud_poly() -> (Vec<Point>, f32) {
    let keys = [
        (2.0, -78.0),
        (10.0, -42.0),
        (16.0, 2.0),
        (18.0, 48.0),
        (8.0, 82.0),
        (-22.0, 92.0),
        (-54.0, 78.0),
        (-78.0, 46.0),
        (-86.0, 8.0),
        (-80.0, -28.0),
        (-58.0, -56.0),
        (-24.0, -70.0),
        (12.0, -52.0),
        (36.0, -22.0),
        (48.0, 16.0),
        (40.0, 52.0),
        (14.0, 64.0),
        (-12.0, 44.0),
        (-28.0, 14.0),
        (-18.0, -16.0),
        (4.0, -38.0),
        (2.0, -78.0),
    ];
    let n = keys.len();
    let samples = 220.min(MAX_POLY);
    let mut poly = Vec::with_capacity(samples);
    for i in 0..samples {
        let t = i as f32 / samples as f32 * n as f32;
        let i1 = t.floor() as usize % n;
        let i0 = (i1 + n - 1) % n;
        let i2 = (i1 + 1) % n;
        let i3 = (i1 + 2) % n;
        let f = t.fract();
        poly.push(Point {
            x: catmull(keys[i0].0, keys[i1].0, keys[i2].0, keys[i3].0, f),
            z: catmull(keys[i0].1, keys[i1].1, keys[i2].1, keys[i3].1, f),
        });
    }
    let mut length = 0.0f32;
    for i in 0..poly.len() {
        let a = &poly[i];
        let b = &poly[(i + 1) % poly.len()];
        let dx = b.x - a.x;
        let dz = b.z - a.z;
        length += (dx * dx + dz * dz).sqrt();
    }
    (poly, length)
}

fn catmull(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * (2.0 * p1
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

fn sample_track(s: &Snapshot, t: f32) -> (f32, f32, f32) {
    let n = s.poly_count.max(2) as usize;
    let u = t.fract().rem_euclid(1.0) * (n as f32);
    let i = u.floor() as usize % n;
    let j = (i + 1) % n;
    let f = u.fract();
    let x = s.poly[i].x + (s.poly[j].x - s.poly[i].x) * f;
    let z = s.poly[i].z + (s.poly[j].z - s.poly[i].z) * f;
    let yaw = (s.poly[j].x - s.poly[i].x).atan2(s.poly[j].z - s.poly[i].z);
    (x, z, yaw)
}

fn animate(s: &mut Snapshot, t: f32, dt: f32) {
    s.session_time_ms += (dt * 1000.0) as i32;
    s.local_rpm = (8200.0 + 3800.0 * (t * 2.4).sin()) as i32;
    s.local_gear = 2 + ((t * 0.35).sin() * 1.6 + 1.6) as i32;
    s.local_speed = 14.0 + 7.0 * (0.5 + 0.5 * (t * 1.1).sin());
    s.current_lap_ms = 18_000 + ((t * 40.0) as i32 % 55_000);

    for i in 0..s.rider_count.max(0) as usize {
        let speed = 0.006 + (i as f32) * 0.00022 + 0.0012 * ((t * 0.35) + i as f32).sin();
        let next = (s.riders[i].track_pos + speed * dt).fract();
        s.riders[i].track_pos = if next < 0.0 { next + 1.0 } else { next };
        let (x, z, yaw) = sample_track(s, s.riders[i].track_pos);
        s.riders[i].x = x;
        s.riders[i].z = z;
        s.riders[i].yaw = yaw;
        if s.riders[i].race_num == FOCUS {
            s.local_x = x;
            s.local_z = z;
            s.local_yaw = yaw;
            s.local_track_pos = s.riders[i].track_pos;
            s.local_vel_x = yaw.sin() * s.local_speed;
            s.local_vel_z = yaw.cos() * s.local_speed;
        }
    }
    refresh_standings(s);
}

fn refresh_standings(s: &mut Snapshot) {
    let n = s.rider_count.max(0) as usize;
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|a, b| {
        s.riders[*b]
            .track_pos
            .partial_cmp(&s.riders[*a].track_pos)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    s.standing_count = n as i32;
    let leader_pos = s.riders[order[0]].track_pos;
    for (i, &ri) in order.iter().enumerate() {
        let r = &s.riders[ri];
        let (name, bike, cat) = RIDERS[ri];
        let gap = if i == 0 {
            0
        } else {
            let d = (leader_pos - r.track_pos).rem_euclid(1.0);
            (d * s.track_length / s.local_speed.max(8.0) * 1000.0) as i32
        };
        s.standings[i] = Standing {
            race_num: r.race_num,
            position: i as i32 + 1,
            state: 0,
            best_lap_ms: 71_800 + ri as i32 * 210,
            num_laps: 7 + ((ri + 3) % 2) as i32,
            gap_ms: gap,
            gap_laps: 0,
            pit: 0,
            penalty_ms: 0,
            crashed: 0,
            name: [0; 32],
            bike: [0; 32],
            last_lap_ms: 72_400 + ri as i32 * 180,
            category: [0; 32],
        };
        write_name(&mut s.standings[i].name, name);
        write_name(&mut s.standings[i].bike, bike);
        write_name(&mut s.standings[i].category, cat);
    }
}
