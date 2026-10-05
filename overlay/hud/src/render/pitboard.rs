#![allow(unused_imports)]
use std::cell::RefCell;

use super::*;
use crate::config::BoardField;
use crate::pitboard::{art_bytes, factory_plate, recolor_plate_pixel, PitVar, PLATE_NAVY, PLATE_YELLOW};

struct FactoryPlateCache {
    art: String,
    source: Option<Pixmap>,
    main: [u8; 3],
    secondary: [u8; 3],
    painted: Option<Pixmap>,
}

thread_local! {
    static FACTORY_PLATE: RefCell<FactoryPlateCache> = RefCell::new(FactoryPlateCache {
        art: String::new(),
        source: None,
        main: [0, 0, 0],
        secondary: [0, 0, 0],
        painted: None,
    });
}

/// Recolored Holeshot plate, logo included.
pub(crate) fn factory_plate_image(art: &str, main: [u8; 3], secondary: [u8; 3]) -> Option<Pixmap> {
    FACTORY_PLATE.with(|slot| {
        let mut cache = slot.borrow_mut();
        if !cache.ensure(art, main, secondary) {
            return None;
        }
        cache.painted.clone()
    })
}

pub(crate) fn draw_pitboard(
    px: &mut Pixmap,
    fonts: &Fonts,
    s: &Snapshot,
    cfg: &HudConfig,
    sw: f32,
    sh: f32,
) {
    let r = cfg[WidgetId::Pitboard].rect;
    let x = r.x * sw;
    let y = r.y * sh;
    let w = r.w * sw;
    let h = r.h * sh;
    if w < 80.0 || h < 48.0 {
        return;
    }
    let a = background_alpha(100);
    let ink_black = Color::from_rgba8(16, 16, 18, 255);
    if !draw_art(px, cfg, x, y, w, h) {
        draw_glass_plate(px, x, y, w, h, a);
    }
    for (index, place) in cfg.pit.pit_vars.iter().enumerate().filter(|(_, place)| place.show) {
        let authored = place
            .color
            .or_else(|| crate::pitboard::pack_slot_color(&cfg.pit.pit_art, index));
        let rgb = crate::pitboard::place_ink(authored);
        let ink = Color::from_rgba8(rgb[0], rgb[1], rgb[2], 255);
        let (label, mut col) = shown_text(pit_value(s, cfg, place.var, ink_black, ink_black), ink_black);
        if authored.is_some() {
            col = ink;
        }
        let fs = place.size.clamp(9.0, 56.0);
        let cx = x + place.x * w;
        let cy = y + place.y * h - fs * 0.55;
        if place.bold {
            text_bold(px, fonts, &label, fs, cx, cy, col, true);
        } else {
            text_halo(px, fonts, &label, fs, cx, cy, col, true, false);
        }
    }
}

fn draw_art(px: &mut Pixmap, cfg: &HudConfig, x: f32, y: f32, w: f32, h: f32) -> bool {
    if factory_plate(&cfg.pit.pit_art) {
        return FACTORY_PLATE.with(|slot| {
            let mut cache = slot.borrow_mut();
            if !cache.ensure(&cfg.pit.pit_art, cfg.pit.pit_yellow, cfg.pit.pit_blue) {
                return false;
            }
            let img = cache.painted.as_ref().unwrap();
            blit_plate(px, cfg, img, x, y, w, h);
            true
        });
    }
    let Some(bytes) = art_bytes(&cfg.pit.pit_art) else {
        return false;
    };
    let Ok(img) = Pixmap::decode_png(&bytes) else {
        return false;
    };
    if img.width() == 0 || img.height() == 0 {
        return false;
    }
    blit_plate(px, cfg, &img, x, y, w, h);
    true
}

impl FactoryPlateCache {
    fn ensure(&mut self, art: &str, main: [u8; 3], secondary: [u8; 3]) -> bool {
        if self.source.is_none() || self.art != art {
            let Some(bytes) = art_bytes(art) else {
                return false;
            };
            let Ok(source) = Pixmap::decode_png(&bytes) else {
                return false;
            };
            if source.width() == 0 || source.height() == 0 {
                return false;
            }
            self.art = art.to_string();
            self.source = Some(source);
            self.painted = None;
        }
        if self.painted.is_some() && self.main == main && self.secondary == secondary {
            return true;
        }
        let mut painted = self.source.clone().unwrap();
        if main != PLATE_YELLOW || secondary != PLATE_NAVY {
            recolor_plate(&mut painted, main, secondary);
        }
        smooth_plate_edges(&mut painted);
        stamp_logo(&mut painted);
        self.main = main;
        self.secondary = secondary;
        self.painted = Some(painted);
        true
    }
}

fn blit_plate(px: &mut Pixmap, _cfg: &HudConfig, img: &Pixmap, x: f32, y: f32, w: f32, h: f32) {
    let sx = w / img.width() as f32;
    let sy = h / img.height() as f32;
    let paint = PixmapPaint {
        opacity: 1.0,
        quality: FilterQuality::Bilinear,
        ..PixmapPaint::default()
    };
    px.draw_pixmap(
        0,
        0,
        img.as_ref(),
        &paint,
        Transform::from_row(sx, 0.0, 0.0, sy, x, y),
        None,
    );
}

const LOGO_X: f32 = 408.0;
const LOGO_Y: f32 = 18.0;
const LOGO_SIZE: f32 = 100.0;

/// Soften stair-stepped plate edges. The logo rectangle stays sharp.
pub(crate) fn smooth_plate_edges(img: &mut Pixmap) {
    let w = img.width() as i32;
    let h = img.height() as i32;
    if w == 0 || h == 0 {
        return;
    }
    let src: Vec<PremultipliedColorU8> = img.pixels().to_vec();
    let in_logo = |x: i32, y: i32| {
        x >= LOGO_X as i32
            && x < (LOGO_X + LOGO_SIZE) as i32
            && y >= LOGO_Y as i32
            && y < (LOGO_Y + LOGO_SIZE) as i32
    };
    let straight = |px: PremultipliedColorU8| -> Option<[u8; 3]> {
        let alpha = px.alpha();
        if alpha < 200 {
            return None;
        }
        let alpha_u = alpha as u16;
        Some([
            ((px.red() as u16 * 255) / alpha_u).min(255) as u8,
            ((px.green() as u16 * 255) / alpha_u).min(255) as u8,
            ((px.blue() as u16 * 255) / alpha_u).min(255) as u8,
        ])
    };
    let mut edge = vec![false; src.len()];
    for y in 0..h {
        for x in 0..w {
            if in_logo(x, y) {
                continue;
            }
            let Some(self_rgb) = straight(src[(y * w + x) as usize]) else {
                continue;
            };
            let mut contrast = false;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let nx = x + dx;
                    let ny = y + dy;
                    if nx < 0 || ny < 0 || nx >= w || ny >= h || in_logo(nx, ny) {
                        continue;
                    }
                    let Some(near) = straight(src[(ny * w + nx) as usize]) else {
                        continue;
                    };
                    let delta = self_rgb
                        .iter()
                        .zip(near)
                        .map(|(a, b)| a.abs_diff(b))
                        .max()
                        .unwrap_or(0);
                    if delta >= 72 {
                        contrast = true;
                    }
                }
            }
            if contrast {
                edge[(y * w + x) as usize] = true;
            }
        }
    }
    let mut band = edge.clone();
    for y in 0..h {
        for x in 0..w {
            if in_logo(x, y) || edge[(y * w + x) as usize] {
                continue;
            }
            let mut near_edge = false;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let nx = x + dx;
                    let ny = y + dy;
                    if nx < 0 || ny < 0 || nx >= w || ny >= h {
                        continue;
                    }
                    if edge[(ny * w + nx) as usize] {
                        near_edge = true;
                    }
                }
            }
            if near_edge {
                band[(y * w + x) as usize] = true;
            }
        }
    }
    for y in 0..h {
        for x in 0..w {
            if in_logo(x, y) || !band[(y * w + x) as usize] {
                continue;
            }
            let here = src[(y * w + x) as usize];
            if straight(here).is_none() {
                continue;
            }
            let mut sum = [0u32; 3];
            let mut count = 0u32;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let nx = x + dx;
                    let ny = y + dy;
                    if nx < 0 || ny < 0 || nx >= w || ny >= h || in_logo(nx, ny) {
                        continue;
                    }
                    let Some(near) = straight(src[(ny * w + nx) as usize]) else {
                        continue;
                    };
                    sum[0] += near[0] as u32;
                    sum[1] += near[1] as u32;
                    sum[2] += near[2] as u32;
                    count += 1;
                }
            }
            if count == 0 {
                continue;
            }
            let mixed = [
                (sum[0] / count) as u8,
                (sum[1] / count) as u8,
                (sum[2] / count) as u8,
            ];
            if let Some(painted) =
                PremultipliedColorU8::from_rgba(mixed[0], mixed[1], mixed[2], here.alpha())
            {
                img.pixels_mut()[(y * w + x) as usize] = painted;
            }
        }
    }
}

fn stamp_logo(plate: &mut Pixmap) {
    let Some(logo) = Pixmap::decode_png(include_bytes!("../../../../web/logo.png")).ok() else {
        return;
    };
    if logo.width() == 0 {
        return;
    }
    let scale = LOGO_SIZE / logo.width() as f32;
    let paint = PixmapPaint {
        quality: FilterQuality::Bilinear,
        ..PixmapPaint::default()
    };
    plate.draw_pixmap(
        0,
        0,
        logo.as_ref(),
        &paint,
        Transform::from_row(scale, 0.0, 0.0, scale, LOGO_X, LOGO_Y),
        None,
    );
}

fn recolor_plate(img: &mut Pixmap, yellow: [u8; 3], navy: [u8; 3]) {
    for px in img.pixels_mut() {
        let alpha = px.alpha();
        if alpha == 0 {
            continue;
        }
        let alpha_u = alpha as u16;
        let straight = |channel: u8| ((channel as u16 * 255) / alpha_u).min(255) as u8;
        let rgb = recolor_plate_pixel(
            [
                straight(px.red()),
                straight(px.green()),
                straight(px.blue()),
                alpha,
            ],
            yellow,
            navy,
        );
        if let Some(painted) =
            PremultipliedColorU8::from_rgba(rgb[0], rgb[1], rgb[2], rgb[3])
        {
            *px = painted;
        }
    }
}

fn draw_glass_plate(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, a: u8) {
    if a == 0 {
        return;
    }
    let r = (h.min(w) * 0.18).clamp(14.0, 36.0);
    let hole = (h.min(w) * 0.038).clamp(5.0, 9.0);
    let inset_x = hole * 2.5;
    let inset_y = hole * 2.0;
    let Some(plate) = hero_plate_path(x, y, w, h, r, hole, inset_x, inset_y) else {
        return;
    };
    fill_path_rule(
        px,
        &plate,
        Color::from_rgba8(10, 10, 10, a),
        FillRule::EvenOdd,
        None,
    );
    let rim = (a as u16 * 80 / 255).max(64) as u8;
    let mut paint = Paint::default();
    paint.set_color(Color::from_rgba8(255, 148, 48, rim));
    paint.anti_alias = true;
    if let Some(outer) = round_rect_path(x, y, w, h, r) {
        px.stroke_path(
            &outer,
            &paint,
            &Stroke {
                width: 1.4,
                ..Stroke::default()
            },
            Transform::identity(),
            None,
        );
    }
    for cx in [x + inset_x, x + w - inset_x] {
        let mut pb = PathBuilder::new();
        pb.push_circle(cx, y + inset_y, hole);
        if let Some(path) = pb.finish() {
            px.stroke_path(
                &path,
                &paint,
                &Stroke {
                    width: 1.1,
                    ..Stroke::default()
                },
                Transform::identity(),
                None,
            );
        }
    }
}

fn hero_plate_path(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: f32,
    hole: f32,
    inset_x: f32,
    inset_y: f32,
) -> Option<Path> {
    let mut pb = PathBuilder::new();
    push_round_poly(&mut pb, &[[x, y], [x + w, y], [x + w, y + h], [x, y + h]], r);
    pb.push_circle(x + inset_x, y + inset_y, hole);
    pb.push_circle(x + w - inset_x, y + inset_y, hole);
    pb.finish()
}

fn push_round_poly(pb: &mut PathBuilder, pts: &[[f32; 2]], r: f32) {
    let n = pts.len();
    if n < 3 {
        return;
    }
    let mut start = [[0.0; 2]; 8];
    let mut end = [[0.0; 2]; 8];
    let mut corner = [[0.0; 2]; 8];
    if n > 8 {
        return;
    }
    for i in 0..n {
        let prev = pts[(i + n - 1) % n];
        let c = pts[i];
        let next = pts[(i + 1) % n];
        let in_dx = c[0] - prev[0];
        let in_dy = c[1] - prev[1];
        let out_dx = next[0] - c[0];
        let out_dy = next[1] - c[1];
        let in_len = (in_dx * in_dx + in_dy * in_dy).sqrt().max(0.001);
        let out_len = (out_dx * out_dx + out_dy * out_dy).sqrt().max(0.001);
        let rr = r.min(in_len * 0.42).min(out_len * 0.42).max(0.5);
        start[i] = [c[0] - in_dx / in_len * rr, c[1] - in_dy / in_len * rr];
        end[i] = [c[0] + out_dx / out_len * rr, c[1] + out_dy / out_len * rr];
        corner[i] = c;
    }
    pb.move_to(end[n - 1][0], end[n - 1][1]);
    for i in 0..n {
        pb.line_to(start[i][0], start[i][1]);
        pb.quad_to(corner[i][0], corner[i][1], end[i][0], end[i][1]);
    }
    pb.close();
}

#[cfg(test)]
pub(crate) fn factory_plate_png() -> Vec<u8> {
    let w: f32 = 960.0;
    let h: f32 = 660.0;
    let mut px = Pixmap::new(w as u32, h as u32).expect("plate");
    let r = h.min(w) * 0.18;
    let hole = h.min(w) * 0.038;
    let inset_x = hole * 2.5;
    let inset_y = hole * 2.0;
    let Some(plate) = hero_plate_path(0.0, 0.0, w, h, r, hole, inset_x, inset_y) else {
        return Vec::new();
    };
    fill_path_rule(
        &mut px,
        &plate,
        Color::from_rgba8(10, 10, 10, 255),
        FillRule::EvenOdd,
        None,
    );
    let mut paint = Paint::default();
    paint.set_color(Color::from_rgba8(255, 148, 48, 255));
    paint.anti_alias = true;
    if let Some(outer) = round_rect_path(0.0, 0.0, w, h, r) {
        px.stroke_path(
            &outer,
            &paint,
            &Stroke {
                width: 3.0,
                ..Stroke::default()
            },
            Transform::identity(),
            None,
        );
    }
    for cx in [inset_x, w - inset_x] {
        let mut pb = PathBuilder::new();
        pb.push_circle(cx, inset_y, hole);
        if let Some(path) = pb.finish() {
            px.stroke_path(
                &path,
                &paint,
                &Stroke {
                    width: 2.4,
                    ..Stroke::default()
                },
                Transform::identity(),
                None,
            );
        }
    }
    px.encode_png().expect("png")
}

fn pit_blank(t: &str) -> bool {
    matches!(
        t,
        "-" | "--" | "---" | "P--" | "L--" | "C--" | "--:--" | "--:--.---"
    )
}

fn shown_text(value: Option<(String, Color)>, ink: Color) -> (String, Color) {
    match value {
        Some((label, col)) if !label.is_empty() && !pit_blank(&label) => (label, col),
        _ => ("---".to_string(), ink),
    }
}

#[cfg(test)]
mod smooth_tests {
    use super::*;

    #[test]
    fn bake_sample_plate_edges() {
        if std::env::var("BAKE_PLATE").is_err() {
            return;
        }
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
        let bytes = std::fs::read(dir.join("holeshot.png")).unwrap();
        let mut img = Pixmap::decode_png(&bytes).unwrap();
        recolor_plate(&mut img, [255, 148, 48], [0, 0, 0]);
        smooth_plate_edges(&mut img);
        stamp_logo(&mut img);
        img.save_png(dir.join("sample.png")).unwrap();
    }
}

#[cfg(test)]
mod shown_text_tests {
    use super::*;

    #[test]
    fn missing_last_lap_is_dashes() {
        let ink = Color::from_rgba8(16, 16, 18, 255);
        assert_eq!(shown_text(None, ink).0, "---");
        assert_eq!(shown_text(Some(("--:--.---".into(), ink)), ink).0, "---");
        assert_eq!(shown_text(Some(("01:35.000".into(), ink)), ink).0, "01:35.000");
    }
}

fn pit_value(
    s: &Snapshot,
    cfg: &HudConfig,
    var: PitVar,
    ink: Color,
    _dim: Color,
) -> Option<(String, Color)> {
    if let Some(snap) = crate::pitboard::time_snap(cfg.pit.pit_when) {
        match var {
            PitVar::Last | PitVar::Cur => {
                return if snap.time_ms > 0 {
                    Some((snap_time_label(cfg.pit.pit_when, snap.time_ms), ink))
                } else {
                    None
                };
            }
            PitVar::Delta => {
                return if snap.has_delta {
                    Some((format_delta_ms(snap.delta_ms), snap_delta_col(snap.delta_ms, ink)))
                } else {
                    None
                };
            }
            _ => {}
        }
    }
    match var {
        PitVar::Sponsor => {
            let t = cfg.pit.pit_sponsor.trim();
            if t.is_empty() {
                None
            } else {
                Some((t.to_ascii_uppercase(), ink))
            }
        }
        PitVar::Name => {
            let st = focus_standing(s)?;
            let n = crate::shm::bytes_as_text(&st.name);
            if n.is_empty() {
                None
            } else {
                Some((n, ink))
            }
        }
        PitVar::Delta => {
            let view = crate::delta::view_for(cfg.delta.delta_session);
            if view.ready && view.has_delta {
                Some((format_delta_ms(view.delta_ms), snap_delta_col(view.delta_ms, ink)))
            } else {
                None
            }
        }
        PitVar::Track => {
            let n = crate::shm::bytes_as_text(&s.track_name);
            if n.is_empty() {
                None
            } else {
                Some((n, ink))
            }
        }
        PitVar::Laps => {
            let n = focus_standing(s)
                .map(|st| st.num_laps)
                .filter(|&n| n > 0)
                .unwrap_or(s.current_lap);
            if n > 0 {
                Some((format!("L{n}"), ink))
            } else {
                None
            }
        }
        PitVar::Sess => {
            let t = session_banner(s).1;
            if pit_blank(&t) {
                None
            } else {
                Some((t, ink))
            }
        }
        PitVar::Pos => dash_field_for(PitVar::Pos).and_then(|f| {
            dash_foot_item(s, cfg, f).and_then(|(_, t, _)| {
                if pit_blank(&t) {
                    None
                } else {
                    Some((t, ink))
                }
            })
        }),
        PitVar::ClassPos => {
            let p = class_position(s);
            if p > 0 {
                Some((format!("C{p}"), ink))
            } else {
                None
            }
        }
        PitVar::GapAhead => {
            let text = RaceStore::with(|race| {
                race.field
                    .focus
                    .and_then(|i| race.field.rows.get(i))
                    .map(|r| gap_ahead_text(s, &race.field, &r.standing))
                    .unwrap_or_default()
            });
            if pit_blank(&text) {
                None
            } else {
                Some((text, ink))
            }
        }
        PitVar::Lap | PitVar::RaceTime | PitVar::SessionBest | PitVar::Riders | PitVar::SessionType
        | PitVar::Server => {
            let field = match var {
                PitVar::Lap => BoardField::Lap,
                PitVar::RaceTime => BoardField::RaceTime,
                PitVar::SessionBest => BoardField::SessionBest,
                PitVar::Riders => BoardField::Riders,
                PitVar::Server => BoardField::Server,
                _ => BoardField::SessionType,
            };
            board_item(s, cfg, field).and_then(|(_, t)| {
                if t.is_empty() || pit_blank(&t) {
                    None
                } else {
                    Some((t, ink))
                }
            })
        }
        other => {
            dash_field_for(other).and_then(|f| {
                dash_foot_item(s, cfg, f).and_then(|(_, t, _)| {
                    if pit_blank(&t) {
                        None
                    } else {
                        Some((t, ink))
                    }
                })
            })
        }
    }
}

fn snap_time_label(when: crate::pitboard::PitWhen, ms: i32) -> String {
    match when {
        crate::pitboard::PitWhen::Sector => crate::race_store::format_lap(ms),
        _ => format_clock(ms),
    }
}

fn snap_delta_col(ms: i32, ink: Color) -> Color {
    if ms > 0 {
        behind_col()
    } else if ms < 0 {
        ahead_col()
    } else {
        ink
    }
}

fn dash_field_for(var: PitVar) -> Option<DashField> {
    Some(match var {
        PitVar::Num => DashField::Number,
        PitVar::Pos => DashField::Position,
        PitVar::Left => DashField::LapsLeft,
        PitVar::Gap => DashField::Gap,
        PitVar::Int => DashField::Interval,
        PitVar::GapBehind => DashField::GapBehind,
        PitVar::Pen => DashField::Penalty,
        PitVar::Last => DashField::Last,
        PitVar::Best => DashField::Best,
        PitVar::Cur => DashField::Current,
        PitVar::Local => DashField::LocalTime,
        PitVar::Bike => DashField::Bike,
        PitVar::Class => DashField::Class,
        PitVar::Setup => DashField::Setup,
        PitVar::Speed => DashField::Speed,
        PitVar::Rpm => DashField::Rpm,
        PitVar::Gear => DashField::Gear,
        PitVar::Fuel => DashField::Fuel,
        PitVar::FuelPct => DashField::FuelPct,
        PitVar::Air => DashField::Air,
        PitVar::Eng => DashField::Engine,
        PitVar::LapDiff => DashField::LapDiff,
        _ => return None,
    })
}

