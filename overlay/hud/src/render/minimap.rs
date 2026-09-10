#![allow(unused_imports)]
use super::*;

pub(crate) fn mini_view_radius(zoom: i32) -> f32 {
    const NEAR_M: f32 = 22.0;
    const FAR_M: f32 = 85.0;
    let t = zoom.clamp(0, 100) as f32 / 100.0;
    FAR_M + t * (NEAR_M - FAR_M)
}

pub(crate) fn draw_minimap(px: &mut Pixmap, fonts: &Fonts, s: &Snapshot, cfg: &HudConfig, sw: f32, sh: f32, age: f32) {
    let r = cfg[WidgetId::Minimap].rect;
    let x = r.x * sw;
    let y = r.y * sh;
    let w = r.w * sw;
    let h = r.h * sh;
    let size = w.min(h).max(48.0);
    let cx = x + w * 0.5;
    let cy = y + h * 0.5;
    let dim = size.round().clamp(48.0, 900.0) as u32;
    let sdim = dim as f32;
    let left = cx - sdim * 0.5;
    let top = cy - sdim * 0.5;
    let mut held = MINI_PX
        .with(|slot| slot.borrow_mut().take())
        .filter(|p| p.width() == dim && p.height() == dim)
        .or_else(|| Pixmap::new(dim, dim));
    let n = s.poly_count.max(0) as usize;
    if n < 2 {
        if let Some(mini) = held.as_ref() {
            blit_circle(px, mini, left, top);
        }
        MINI_PX.with(|slot| *slot.borrow_mut() = held);
        return;
    }
    let Some(mini) = held.as_mut() else {
        return;
    };
    mini.fill(Color::TRANSPARENT);
    if cfg[WidgetId::Minimap].bg > 0 {
        fill_circle(
            mini,
            sdim * 0.5,
            sdim * 0.5,
            sdim * 0.5 - 0.5,
            Color::from_rgba8(18, 18, 20, bg_a(cfg[WidgetId::Minimap].bg)),
        );
    }

    let subject = camera_subject(s);
    let you = subject_pose(s, age);
    let (fx, fz, rx, rz, scale, origin_x, origin_z, north_up) = if let Some(pose) = you.as_ref() {
        let (fx, fz) = track_forward(s, n, pose.x, pose.z).unwrap_or_else(|| {
            if pose.from_local {
                let (f, z, _, _) = radar_axes(s);
                (f, z)
            } else {
                yaw_forward(pose.yaw)
            }
        });
        let radius_m = mini_view_radius(cfg.mini_zoom);
        (fx, fz, fz, -fx, (sdim * 0.46) / radius_m, pose.x, pose.z, true)
    } else {
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
        let usable = sdim * 0.68;
        let dx = (max_x - min_x).max(8.0);
        let dz = (max_z - min_z).max(8.0);
        let scale = (usable / dx).min(usable / dz);
        (0.0, 1.0, 1.0, 0.0, scale, (min_x + max_x) * 0.5, (min_z + max_z) * 0.5, false)
    };
    let mc = sdim * 0.5;
    let to_px = |wx: f32, wz: f32| -> (f32, f32) {
        let dx = wx - origin_x;
        let dz = wz - origin_z;
        let along = dx * fx + dz * fz;
        let right = dx * rx + dz * rz;
        if north_up {
            (mc + right * scale, mc - along * scale)
        } else {
            (mc + dx * scale, mc - dz * scale)
        }
    };

    let track_px = if north_up {
        let radius_m = mini_view_radius(cfg.mini_zoom);
        (sdim * 0.11 * (40.0 / radius_m)).clamp(16.0, 48.0)
    } else {
        (16.0 * scale).clamp(12.0, 28.0)
    };
    let mut pb = PathBuilder::new();
    if north_up {
        append_visible_track(&mut pb, s, n, origin_x, origin_z, mini_view_radius(cfg.mini_zoom) * 2.25, &to_px);
    } else {
        let (sx, sy) = to_px(s.poly[0].x, s.poly[0].z);
        pb.move_to(sx, sy);
        for p in s.poly.iter().take(n).skip(1) {
            let (px_, py_) = to_px(p.x, p.z);
            pb.line_to(px_, py_);
        }
    }
    if let Some(path) = pb.finish() {
        stroke_path_fast(mini, &path, Color::from_rgba8(8, 8, 10, 220), track_px + 5.0);
        stroke_path_fast(mini, &path, Color::from_rgba8(248, 248, 252, 255), track_px);
    }

    if n >= 2 && s.sf_meters >= 0.0 && cfg.mini_sf {
        draw_sf(mini, s, n, to_px, track_px);
    }
    if cfg.mini_sectors {
        draw_sector_lines(mini, fonts, s, n, to_px, track_px, Some((mc, mc, sdim * 0.46)));
    }
    if cfg.mini_arrows {
        draw_track_arrows(mini, s, n, to_px, track_px, Some((mc, sdim)), north_up);
    }

    let leader = leader_num(s);
    let other_r = (sdim * 0.028).clamp(7.0, 11.0) * style_k();
    let local_r = other_r * 1.22;

    if cfg.mini_others {
        for i in 0..s.rider_count.max(0) as usize {
            let rider = &s.riders[i];
            if you.is_some() && rider.race_num == subject {
                continue;
            }
            let (hx, hy) = to_px(rider.x, rider.z);
            if (hx - mc) * (hx - mc) + (hy - mc) * (hy - mc) > sdim * sdim * 0.27 {
                continue;
            }
            let fill = rider_dot_col(s, rider.race_num);
            numbered_dot(
                mini,
                fonts,
                hx,
                hy,
                other_r,
                fill,
                rider_dot_num(s, rider.race_num, cfg.mini_dot),
                cfg.mini_numbers,
                false,
            );
            let (fwx, fwz) = yaw_forward(rider.yaw);
            let (sdx, sdy) = screen_dir(&to_px, rider.x, rider.z, fwx, fwz);
            draw_dot_chevron(mini, hx, hy, other_r, sdx, sdy, fill, false);
            draw_rider_overhead(mini, fonts, s, rider.race_num, hx, hy, other_r, subject, leader, cfg.mini_crown, cfg.mini_place);
            draw_state_mark(mini, fonts, hx, hy, other_r, rider_mark(s, rider.race_num, rider.crashed != 0));
        }
    }

    if let Some(pose) = you {
        let (hx, hy) = to_px(pose.x, pose.z);
        if pose.from_local {
            let vx = pose.vel_x;
            let vz = pose.vel_z;
            for i in (1..5).rev() {
                let t = i as f32 * 0.07;
                let (tx, ty) = to_px(pose.x - vx * t, pose.z - vz * t);
                let a = 40u8.saturating_mul(5 - i as u8);
                fill_circle(mini, tx, ty, local_r * (0.55 + i as f32 * 0.04), Color::from_rgba8(255, 148, 48, a));
            }
        }
        numbered_dot(
            mini,
            fonts,
            hx,
            hy,
            local_r,
            you_col(),
            rider_dot_num(s, subject, cfg.mini_dot),
            cfg.mini_numbers,
            true,
        );
        let (fwx, fwz) = if pose.from_local {
            local_forward(s)
        } else {
            yaw_forward(pose.yaw)
        };
        let (sdx, sdy) = screen_dir(&to_px, pose.x, pose.z, fwx, fwz);
        draw_dot_chevron(mini, hx, hy, local_r, sdx, sdy, you_col(), true);
        if cfg.mini_crown && leader > 0 && subject == leader {
            crown_over_dot(mini, fonts, hx, hy, local_r);
        }
        draw_state_mark(mini, fonts, hx, hy, local_r, rider_mark(s, subject, pose.crashed));
    }

    blit_circle(px, mini, left, top);
    MINI_PX.with(|slot| *slot.borrow_mut() = held);
}
