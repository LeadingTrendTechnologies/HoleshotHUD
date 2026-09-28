#![allow(unused_imports)]
use super::*;
use crate::config::BoardField;
use crate::pitboard::{art_bytes, PitVar};

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
    let a = bg_a(cfg[WidgetId::Pitboard].bg);
    let glass = a < 102;
    let ink = match cfg.pit_text {
        TableText::Black => Color::from_rgba8(16, 16, 18, 255),
        TableText::White => text_col(),
    };
    let dim = match cfg.pit_text {
        TableText::Black => Color::from_rgba8(48, 48, 52, 255),
        TableText::White => text_dim(),
    };
    if !draw_art(px, cfg, x, y, w, h) {
        draw_glass_plate(px, x, y, w, h, a);
    }
    let k = style_k();
    let halo = matches!(cfg.pit_text, TableText::White) && glass;
    for place in cfg.pit_vars.iter().filter(|p| p.show) {
        let Some((label, col)) = pit_value(s, cfg, place.var, ink, dim) else {
            continue;
        };
        if label.is_empty() || pit_blank(&label) {
            continue;
        }
        let fs = (place.size * k).clamp(9.0, 56.0);
        let cx = x + place.x * w;
        let cy = y + place.y * h - fs * 0.55;
        text_halo(px, fonts, &label, fs, cx, cy, col, true, halo);
    }
}

fn draw_art(px: &mut Pixmap, cfg: &HudConfig, x: f32, y: f32, w: f32, h: f32) -> bool {
    let Some(bytes) = art_bytes(&cfg.pit_art) else {
        return false;
    };
    let Ok(img) = Pixmap::decode_png(&bytes) else {
        return false;
    };
    if img.width() == 0 || img.height() == 0 {
        return false;
    }
    let sx = w / img.width() as f32;
    let sy = h / img.height() as f32;
    let paint = PixmapPaint {
        opacity: (cfg[WidgetId::Pitboard].bg.clamp(0, 100) as f32 / 100.0).max(0.15),
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
    true
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
        "--" | "---" | "P--" | "L--" | "C--" | "--:--" | "--:--.---"
    )
}

fn pit_value(
    s: &Snapshot,
    cfg: &HudConfig,
    var: PitVar,
    ink: Color,
    _dim: Color,
) -> Option<(String, Color)> {
    if let Some(snap) = crate::pitboard::time_snap(cfg.pit_when) {
        match var {
            PitVar::Last | PitVar::Cur => {
                return if snap.time_ms > 0 {
                    Some((snap_time_label(cfg.pit_when, snap.time_ms), ink))
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
            let t = cfg.pit_sponsor.trim();
            if t.is_empty() {
                None
            } else {
                Some((t.to_ascii_uppercase(), ink))
            }
        }
        PitVar::Name => {
            let st = focus_standing(s)?;
            let n = crate::shm::cstr(&st.name);
            if n.is_empty() {
                None
            } else {
                Some((n, ink))
            }
        }
        PitVar::Delta => {
            let view = crate::delta::view_for(cfg.delta_session);
            if view.ready && view.has_delta {
                Some((format_delta_ms(view.delta_ms), snap_delta_col(view.delta_ms, ink)))
            } else {
                None
            }
        }
        PitVar::Track => {
            let n = crate::shm::cstr(&s.track_name);
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

