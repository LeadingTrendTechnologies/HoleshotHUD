//! Re-render marketing announce shots from the live HUD.
//!
//!   cargo run -p mxbo-hud --example dump_announce_shots --release

#[path = "../../../web-preview/src/demo_track.rs"]
mod demo_track;

use mxbo_hud::config::{FontFamily, HudConfig, SnapAlign, Units, WidgetId};
use mxbo_hud::render::{draw, Fonts};
use mxbo_hud::shm::{
    write_name, Point, Rider, Snapshot, Standing, MAGIC, VERSION,
};
use mxbo_hud::{set_sys_procs, set_sys_stats, SysProc};
use tiny_skia::Pixmap;

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
const W: u32 = 1912;
const H: u32 = 1078;
const HERO_H: u32 = 1482;

fn main() {
    let fonts = Fonts::for_family(FontFamily::Exo2)
        .or_else(|| Fonts::for_family(FontFamily::Roboto))
        .or_else(Fonts::load)
        .expect("fonts");
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../announce-shots");
    std::fs::create_dir_all(&out).expect("announce-shots dir");

    let shots: &[(&str, u32, u32, fn(&mut HudConfig))] = &[
        ("standings.png", W, H, |c| size_show(c, "standings", 0.28, 0.70)),
        ("relative.png", W, H, |c| size_show(c, "relative", 0.26, 0.52)),
        ("map.png", W, H, |c| size_show(c, "map", 0.48, 0.72)),
        ("minimap.png", W, H, |c| size_show(c, "minimap", 0.34, 0.58)),
        ("radar.png", W, H, |c| size_show(c, "radar", 0.24, 0.42)),
        ("dash.png", W, H, |c| size_show(c, "dash", 0.22, 0.14)),
        ("ticker.png", W, H, |c| size_show(c, "ticker", 0.90, 0.10)),
        ("sys.png", W, H, |c| size_show(c, "sys", 0.30, 0.56)),
        ("sector.png", W, H, |c| {
            size_show(c, "sector", 0.42, 0.18);
            c.experimental = true;
            c.show_sector = true;
            c.sector_live = true;
        }),
        ("delta.png", W, H, |c| {
            size_show(c, "delta", 0.42, 0.12);
            c.experimental = true;
            c.show_delta = true;
            c.delta_bg = 0;
            mxbo_hud::delta::set_preview(Some(mxbo_hud::delta::DeltaView {
                ready: true,
                recording: false,
                has_delta: true,
                delta_ms: -347,
                ref_lap_ms: 72_140,
                cover: 100,
                last_lap_ms: 72_480,
                new_best: false,
            }));
        }),
        ("hero.png", W, HERO_H, layout_hero),
    ];

    for &(name, w, h, setup) in shots {
        mxbo_hud::delta::set_preview(None);
        mxbo_hud::sector::reload();
        let mut cfg = base_cfg();
        setup(&mut cfg);
        let mut snap = demo_snapshot();
        if name == "sector.png" {
            snap.current_lap_ms = 70_000;
            snap.sector_last = 1;
            snap.sector_cur = [24_093, 25_760, 0];
            snap.sector_last_lap = [24_310, 25_820, 23_090];
            snap.sector_best = [24_180, 25_640, 22_910];
            snap.sector_delta = [-87, 120, 0];
            snap.sector_delta_valid = 0b011;
        }
        if name == "sys.png" {
            set_sys_stats(48.0, 62.0, 91.0, 11.0);
            set_sys_procs([
                SysProc { cpu: 12.0, mem_mb: 420.0, mem_pct: 2.6, on: true },
                SysProc { cpu: 41.0, mem_mb: 1800.0, mem_pct: 11.0, on: true },
                SysProc { cpu: 8.0, mem_mb: 180.0, mem_pct: 1.1, on: true },
                SysProc { cpu: -1.0, mem_mb: 44.0, mem_pct: 0.3, on: true },
            ]);
        }
        cfg.apply_to_snapshot(&mut snap);
        let mut px = Pixmap::new(w, h).expect("pixmap");
        fill_backdrop(&mut px);
        // Warm layout / race store, then draw for real on a clean plate.
        draw(&mut px, &fonts, Some(&snap), &cfg, w, h, 0.35, false, false, false);
        fill_backdrop(&mut px);
        draw(&mut px, &fonts, Some(&snap), &cfg, w, h, 0.35, false, false, false);
        let path = out.join(name);
        std::fs::write(&path, px.encode_png().expect("png")).expect("write");
        println!("wrote {}", path.display());
    }
}

fn base_cfg() -> HudConfig {
    let mut cfg = HudConfig::new();
    cfg.font_family = FontFamily::Exo2;
    cfg.units = Units::Imperial;
    // Showcase latest table styling.
    cfg.st_bike = true;
    cfg.rel_bike = true;
    cfg.st_hl = 50;
    cfg.rel_hl = 50;
    cfg.st_stripe = true;
    cfg.rel_stripe = true;
    cfg.map_sectors = true;
    cfg.st_w_name = 100;
    cfg.rel_w_name = 100;
    cfg.st_w_bike = 64;
    cfg.rel_w_bike = 64;
    cfg
}

fn show_only(cfg: &mut HudConfig, name: &str) {
    cfg.show_standings = name == "standings";
    cfg.show_relative = name == "relative";
    cfg.show_dash = name == "dash";
    cfg.show_map = name == "map";
    cfg.show_minimap = name == "minimap";
    cfg.show_radar = name == "radar";
    cfg.show_ticker = name == "ticker";
    cfg.show_sys = name == "sys";
    cfg.show_sector = name == "sector";
    cfg.show_delta = name == "delta";
    cfg.experimental = name == "sector" || name == "delta";
}

fn size_show(cfg: &mut HudConfig, name: &str, w: f32, h: f32) {
    show_only(cfg, name);
    let id = match name {
        "standings" => WidgetId::Standings,
        "relative" => WidgetId::Relative,
        "map" => WidgetId::Map,
        "minimap" => WidgetId::Minimap,
        "radar" => WidgetId::Radar,
        "dash" => WidgetId::Dash,
        "ticker" => WidgetId::Ticker,
        "sys" => WidgetId::Sys,
        "sector" => WidgetId::Sector,
        "delta" => WidgetId::Delta,
        _ => return,
    };
    {
        let r = match id {
            WidgetId::Standings => &mut cfg.standings,
            WidgetId::Relative => &mut cfg.relative,
            WidgetId::Map => &mut cfg.map,
            WidgetId::Minimap => &mut cfg.minimap,
            WidgetId::Radar => &mut cfg.radar,
            WidgetId::Dash => &mut cfg.dash,
            WidgetId::Ticker => &mut cfg.ticker,
            WidgetId::Sys => &mut cfg.sys,
            WidgetId::Sector => &mut cfg.sector,
            WidgetId::Delta => &mut cfg.delta,
            WidgetId::Stance => &mut cfg.stance,
        };
        r.w = w;
        r.h = h;
    }
    cfg.snap(id, SnapAlign::Center);
}

fn layout_hero(cfg: &mut HudConfig) {
    cfg.show_standings = true;
    cfg.show_relative = false;
    cfg.show_map = true;
    cfg.show_minimap = false;
    cfg.show_radar = true;
    cfg.show_dash = true;
    cfg.show_ticker = true;
    cfg.show_sys = false;
    cfg.show_sector = false;
    cfg.show_delta = false;
    cfg.standings = mxbo_hud::shm::Rect {
        x: 0.04,
        y: 0.10,
        w: 0.28,
        h: 0.58,
    };
    cfg.map = mxbo_hud::shm::Rect {
        x: 0.52,
        y: 0.12,
        w: 0.42,
        h: 0.52,
    };
    cfg.radar = mxbo_hud::shm::Rect {
        x: 0.58,
        y: 0.72,
        w: 0.16,
        h: 0.20,
    };
    cfg.dash = mxbo_hud::shm::Rect {
        x: 0.80,
        y: 0.78,
        w: 0.14,
        h: 0.10,
    };
    cfg.ticker = mxbo_hud::shm::Rect {
        x: 0.06,
        y: 0.02,
        w: 0.88,
        h: 0.06,
    };
}

fn fill_backdrop(px: &mut Pixmap) {
    let w = px.width() as i32;
    let h = px.height() as i32;
    let cx = (w as f32) * 0.5;
    let cy = (h as f32) * 0.5;
    let max_d = (cx * cx + cy * cy).sqrt();
    let data = px.data_mut();
    for y in 0..h {
        for x in 0..w {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let t = ((dx * dx + dy * dy).sqrt() / max_d).clamp(0.0, 1.0);
            let shade = (21.0 + (12.0 - 21.0) * t) as u8;
            let i = ((y * w + x) * 4) as usize;
            data[i] = shade;
            data[i + 1] = shade.saturating_sub(1);
            data[i + 2] = shade.saturating_sub(2);
            data[i + 3] = 255;
        }
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
    s.local_rpm = 9666;
    s.local_speed = 16.5;
    s.current_lap_ms = 12_000;
    s.sector_count = 3;
    s.sector_last = 2;
    s.sector_cur = [0, 0, 0];
    s.sector_last_lap = [24_093, 25_760, 23_090];
    s.sector_best = [24_180, 25_640, 22_910];
    s.sector_delta = [-87, 120, -40];
    s.sector_delta_valid = 0b111;

    let poly: Vec<Point> = demo_track::POLY
        .iter()
        .map(|(x, z)| Point { x: *x, z: *z })
        .collect();
    write_name(&mut s.track_name, demo_track::TRACK_NAME);
    s.poly_count = poly.len() as i32;
    for (i, p) in poly.iter().enumerate() {
        s.poly[i] = *p;
    }
    s.track_length = demo_track::TRACK_LENGTH;
    s.sf_meters = demo_track::SF_METERS;
    mxbo_hud::sector::set_split_fracs([0.31, 0.64]);

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
    apply_radar_pack(&mut s);
    refresh_standings(&mut s);

    if let Some(fi) = (0..s.rider_count.max(0) as usize).find(|&i| s.riders[i].race_num == FOCUS) {
        s.local_x = s.riders[fi].x;
        s.local_z = s.riders[fi].z;
        s.local_yaw = s.riders[fi].yaw;
        s.local_track_pos = s.riders[fi].track_pos;
        s.local_vel_x = s.riders[fi].yaw.sin() * s.local_speed;
        s.local_vel_z = s.riders[fi].yaw.cos() * s.local_speed;
    }
    s
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

fn offset_from_track(s: &Snapshot, pos: f32, along_m: f32, lat_m: f32) -> (f32, f32, f32, f32) {
    let dt = along_m / s.track_length.max(1.0);
    let t = (pos + dt).rem_euclid(1.0);
    let (x, z, yaw) = sample_track(s, t);
    let rx = yaw.cos();
    let rz = -yaw.sin();
    (x + rx * lat_m, z + rz * lat_m, yaw, t)
}

fn apply_radar_pack(s: &mut Snapshot) {
    const PACK: &[(usize, f32, f32)] = &[
        (6, -1.4, -2.3),
        (7, -2.2, 2.0),
        (8, 2.4, -1.6),
        (9, 5.2, 0.8),
    ];
    let n = s.rider_count.max(0) as usize;
    let Some(fi) = (0..n).find(|&i| s.riders[i].race_num == FOCUS) else {
        return;
    };
    let focus_pos = s.riders[fi].track_pos;
    for &(i, along, lat) in PACK {
        if i >= n || i == fi {
            continue;
        }
        let (x, z, yaw, pos) = offset_from_track(s, focus_pos, along, lat);
        s.riders[i].x = x;
        s.riders[i].z = z;
        s.riders[i].yaw = yaw;
        s.riders[i].track_pos = pos;
    }
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
        let num_laps = match ri {
            6 | 7 => 8,
            8 | 9 => 6,
            _ => 7,
        };
        s.standings[i] = Standing {
            race_num: r.race_num,
            position: i as i32 + 1,
            state: 0,
            best_lap_ms: 71_800 + ri as i32 * 210,
            num_laps,
            gap_ms: gap,
            gap_laps: (8 - num_laps).max(0),
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
