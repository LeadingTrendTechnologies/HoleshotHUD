#![allow(unused_imports)]
use super::*;

pub(crate) fn mini_view_radius(zoom: i32) -> f32 {
    const NEAR_M: f32 = 22.0;
    const FAR_M: f32 = 85.0;
    let t = zoom.clamp(0, 100) as f32 / 100.0;
    FAR_M + t * (NEAR_M - FAR_M)
}

pub(crate) fn draw_minimap(
    px: &mut Pixmap,
    fonts: &Fonts,
    s: &Snapshot,
    cfg: &HudConfig,
    sw: f32,
    sh: f32,
    age: f32,
    raster_w: f32,
    raster_h: f32,
) {
    let r = cfg[WidgetId::Minimap].rect;
    let x = r.x * sw;
    let y = r.y * sh;
    let w = r.w * sw;
    let h = r.h * sh;
    let place = w.min(h).max(48.0);
    let place_dim = place.round().clamp(48.0, 900.0);
    let cx = x + w * 0.5;
    let cy = y + h * 0.5;
    let raster = (r.w * raster_w).min(r.h * raster_h).max(48.0);
    let dim = raster.round().clamp(48.0, 900.0).max(place_dim) as u32;
    let sdim = dim as f32;
    let left = cx - place_dim * 0.5;
    let top = cy - place_dim * 0.5;
    let mut held = MINI_PX
        .with(|slot| slot.borrow_mut().take())
        .filter(|p| p.width() == dim && p.height() == dim)
        .or_else(|| Pixmap::new(dim, dim));
    let n = s.poly_count.max(0) as usize;
    if n < 2 {
        if let Some(mini) = held.as_ref() {
            blit_minimap(px, mini, left, top, place_dim);
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
            Color::from_rgba8(18, 18, 20, background_alpha(cfg[WidgetId::Minimap].bg)),
        );
    }

    let subject = camera_subject(s);
    let you = subject_pose(s, age);
    let radius_m = mini_view_radius(cfg.mini.mini_zoom);
    // On the line, keep the bike as the origin. Off the line, center the circle on the
    // nearest centerline so the stroke is in the view. Past 300 m, fit the whole track.
    let on_track = you.as_ref().and_then(|pose| {
        let hit = nearest_centerline(s, n, pose.x, pose.z)?;
        if hit.dist2 > 90_000.0 {
            return None;
        }
        let (fx, fz) = hit.forward.unwrap_or_else(|| {
            if pose.from_local {
                let (forward_x, forward_z, _, _) = radar_axes(s);
                (forward_x, forward_z)
            } else {
                yaw_forward(pose.yaw)
            }
        });
        let (origin_x, origin_z) = if hit.dist2 > radius_m * radius_m {
            (hit.x, hit.z)
        } else {
            (pose.x, pose.z)
        };
        Some((fx, fz, origin_x, origin_z))
    });
    let (fx, fz, rx, rz, scale, origin_x, origin_z, north_up) = if let Some((fx, fz, origin_x, origin_z)) =
        on_track
    {
        (
            fx,
            fz,
            fz,
            -fx,
            (sdim * 0.46) / radius_m,
            origin_x,
            origin_z,
            true,
        )
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
        (
            0.0,
            1.0,
            1.0,
            0.0,
            scale,
            (min_x + max_x) * 0.5,
            (min_z + max_z) * 0.5,
            false,
        )
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
        let radius_m = mini_view_radius(cfg.mini.mini_zoom);
        (sdim * 0.11 * (40.0 / radius_m)).clamp(16.0, 48.0)
    } else {
        (16.0 * scale).clamp(12.0, 28.0)
    };
    let visible_radius = if north_up {
        Some(mini_view_radius(cfg.mini.mini_zoom) * 2.25)
    } else {
        None
    };
    stroke_smooth_minimap_track(
        mini,
        s,
        n,
        origin_x,
        origin_z,
        visible_radius,
        track_px,
        &to_px,
    );

    if n >= 2 && s.sf_meters >= 0.0 && cfg.mini.mini_sf {
        draw_sf(mini, s, n, to_px, track_px);
    }
    if cfg.mini.mini_sectors {
        draw_sector_lines(
            mini,
            fonts,
            s,
            n,
            to_px,
            track_px,
            Some((mc, mc, sdim * 0.46)),
        );
    }
    if cfg.mini.mini_arrows {
        draw_track_arrows(mini, s, n, to_px, track_px, Some((mc, sdim)), north_up);
    }

    let leader = leader_num(s);
    let other_r = (sdim * 0.028).clamp(7.0, 11.0) * style_k();
    let local_r = other_r * 1.22;

    if cfg.mini.mini_others {
        for i in 0..s.rider_count.max(0) as usize {
            let rider = &s.riders[i];
            if you.is_some() && rider.race_num == subject {
                continue;
            }
            let pose = rider_map_pose(s, rider);
            let (hx, hy) = to_px(pose.x, pose.z);
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
                rider_dot_num(s, rider.race_num, cfg.mini.mini_dot),
                cfg.mini.mini_numbers,
                false,
                MAP_DOT_OPACITY_DEFAULT,
                None,
            );
            let (fwx, fwz) = yaw_forward(pose.yaw);
            let (sdx, sdy) = screen_dir(&to_px, pose.x, pose.z, fwx, fwz);
            draw_dot_chevron(mini, hx, hy, other_r, sdx, sdy, fill, false);
            draw_rider_overhead(
                mini,
                fonts,
                s,
                rider.race_num,
                hx,
                hy,
                other_r,
                subject,
                leader,
                cfg.mini.mini_crown,
                cfg.mini.mini_place,
            );
            draw_state_mark(
                mini,
                fonts,
                hx,
                hy,
                other_r,
                rider_mark(s, rider.race_num, rider.crashed != 0),
            );
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
                fill_circle(
                    mini,
                    tx,
                    ty,
                    local_r * (0.55 + i as f32 * 0.04),
                    accent_a(a),
                );
            }
        }
        numbered_dot(
            mini,
            fonts,
            hx,
            hy,
            local_r,
            you_col(),
            rider_dot_num(s, subject, cfg.mini.mini_dot),
            cfg.mini.mini_numbers,
            true,
            MAP_DOT_OPACITY_DEFAULT,
            None,
        );
        let (fwx, fwz) = if pose.from_local {
            local_forward(s)
        } else {
            yaw_forward(pose.yaw)
        };
        let (sdx, sdy) = screen_dir(&to_px, pose.x, pose.z, fwx, fwz);
        draw_dot_chevron(mini, hx, hy, local_r, sdx, sdy, you_col(), true);
        if cfg.mini.mini_crown && leader > 0 && subject == leader {
            crown_over_dot(mini, fonts, hx, hy, local_r);
        }
        draw_state_mark(
            mini,
            fonts,
            hx,
            hy,
            local_r,
            rider_mark(s, subject, pose.crashed),
        );
    }

    blit_minimap(px, mini, left, top, place_dim);
    MINI_PX.with(|slot| *slot.borrow_mut() = held);
}

fn blit_minimap(dst: &mut Pixmap, src: &Pixmap, left: f32, top: f32, place_dim: f32) {
    if src.width() as f32 == place_dim && src.height() as f32 == place_dim {
        blit_circle(dst, src, left, top);
    } else {
        blit_circle_fit(dst, src, left, top, place_dim);
    }
}
