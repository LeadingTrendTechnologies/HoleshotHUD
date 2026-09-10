#![allow(unused_imports)]
use super::*;

pub(crate) fn draw_map(px: &mut Pixmap, fonts: &Fonts, s: &Snapshot, cfg: &HudConfig, sw: f32, sh: f32, age: f32) {
    let r = s.map;
    let x = r.x * sw;
    let y = r.y * sh;
    let w = r.w * sw;
    let h = r.h * sh;
    if cfg[WidgetId::Map].bg > 0 {
        if let Some(rect) = rr(x, y, w, h) {
            fill_rect(px, rect, Color::from_rgba8(10, 10, 10, bg_a(cfg[WidgetId::Map].bg)));
        }
    }
    let n = s.poly_count.max(0) as usize;
    if n < 2 {
        text(px, fonts, "No track map", 13.0, x + w * 0.5, y + h * 0.5, text_dim(), true);
        return;
    }

    let mut min_x = s.poly[0].x;
    let mut max_x = min_x;
    let mut min_z = s.poly[0].z;
    let mut max_z = min_z;
    for p in s.poly.iter().take(n) {
        min_x = min_x.min(p.x);
        max_x = max_x.max(p.x);
        min_z = min_z.min(p.z);
        max_z = max_z.max(p.z);
    }
    let mut dx = (max_x - min_x).max(8.0);
    let mut dz = (max_z - min_z).max(8.0);
    let pad = 0.10;
    let usable_w = w * (1.0 - 2.0 * pad);
    let usable_h = h * (1.0 - 2.0 * pad);
    let scale = (usable_w / dx).min(usable_h / dz);
    dx = max_x - min_x;
    dz = max_z - min_z;
    let used_w = dx * scale;
    let used_h = dz * scale;
    let ox = x + (w - used_w) * 0.5;
    let oy = y + (h - used_h) * 0.5;
    let to_px = |wx: f32, wz: f32| -> (f32, f32) {
        (ox + (wx - min_x) * scale, oy + (max_z - wz) * scale)
    };

    let track_px = (8.0 * scale).clamp(5.5, 26.0);
    let lw = w.ceil().max(1.0) as u32;
    let lh = h.ceil().max(1.0) as u32;
    let key = track_layer_key(s, n, lw, lh, cfg.map_sf, cfg.map_arrows);
    MAP_LAYER.with(|slot| {
        let mut slot = slot.borrow_mut();
        let stale = slot.as_ref().map(|(k, _)| *k != key).unwrap_or(true);
        if stale {
            if let Some(mut layer) = Pixmap::new(lw, lh) {
                let to_local = |wx: f32, wz: f32| -> (f32, f32) {
                    let (sx, sy) = to_px(wx, wz);
                    (sx - x, sy - y)
                };
                let mut pb = PathBuilder::new();
                let (sx, sy) = to_local(s.poly[0].x, s.poly[0].z);
                pb.move_to(sx, sy);
                for p in s.poly.iter().take(n).skip(1) {
                    let (px_, py_) = to_local(p.x, p.z);
                    pb.line_to(px_, py_);
                }
                pb.close();
                if let Some(path) = pb.finish() {
                    let mut fill = Paint::default();
                    fill.set_color(fill_col());
                    fill.anti_alias = true;
                    layer.fill_path(&path, &fill, FillRule::EvenOdd, Transform::identity(), None);
                    stroke_path(&mut layer, &path, Color::from_rgba8(18, 16, 16, 240), track_px + 3.0);
                    stroke_path(&mut layer, &path, track_col(), track_px);
                }
                if n >= 2 && s.sf_meters >= 0.0 && cfg.map_sf {
                    draw_sf(&mut layer, s, n, to_local, track_px);
                }
                if cfg.map_arrows {
                    draw_track_arrows(&mut layer, s, n, to_local, track_px, None, false);
                }
                *slot = Some((key, layer));
            }
        }
        if let Some((_, layer)) = slot.as_ref() {
            px.draw_pixmap(
                0,
                0,
                layer.as_ref(),
                &PixmapPaint::default(),
                Transform::from_translate(x, y),
                None,
            );
        }
    });

    if cfg.map_sectors {
        draw_sector_lines(px, fonts, s, n, to_px, track_px, None);
    }

    let subject = camera_subject(s);
    let you = subject_pose(s, age);
    let leader = leader_num(s);
    let other_r = (if cfg.map_numbers { 6.8 } else { 4.6 }) * style_k();

    if cfg.map_others {
        for i in 0..s.rider_count.max(0) as usize {
            let rider = &s.riders[i];
            if you.is_some() && rider.race_num == subject {
                continue;
            }
            let (hx, hy) = to_px(rider.x, rider.z);
            let fill = rider_dot_col(s, rider.race_num);
            draw_rider_dot(
                px,
                fonts,
                hx,
                hy,
                other_r,
                fill,
                rider_dot_num(s, rider.race_num, cfg.map_dot),
                cfg.map_numbers,
                false,
            );
            let (fwx, fwz) = yaw_forward(rider.yaw);
            let (sdx, sdy) = screen_dir(&to_px, rider.x, rider.z, fwx, fwz);
            draw_dot_chevron(px, hx, hy, other_r, sdx, sdy, fill, false);
            draw_rider_overhead(px, fonts, s, rider.race_num, hx, hy, other_r, subject, leader, cfg.map_crown, cfg.map_place);
            draw_state_mark(px, fonts, hx, hy, other_r, rider_mark(s, rider.race_num, rider.crashed != 0));
        }
    }
    if let Some(pose) = you {
        let (hx, hy) = to_px(pose.x, pose.z);
        let local_r = 8.5 * style_k();
        draw_rider_dot(
            px,
            fonts,
            hx,
            hy,
            local_r,
            you_col(),
            rider_dot_num(s, subject, cfg.map_dot),
            cfg.map_numbers,
            true,
        );
        let (fwx, fwz) = if pose.from_local {
            local_forward(s)
        } else {
            yaw_forward(pose.yaw)
        };
        let (sdx, sdy) = screen_dir(&to_px, pose.x, pose.z, fwx, fwz);
        draw_dot_chevron(px, hx, hy, local_r, sdx, sdy, you_col(), true);
        if cfg.map_crown && leader > 0 && subject == leader {
            crown_over_dot(px, fonts, hx, hy, local_r);
        }
        draw_state_mark(px, fonts, hx, hy, local_r, rider_mark(s, subject, pose.crashed));
    }
}
