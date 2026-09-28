#![allow(unused_imports)]
use super::*;

pub(crate) fn draw_map(
    px: &mut Pixmap,
    fonts: &Fonts,
    s: &Snapshot,
    cfg: &HudConfig,
    sw: f32,
    sh: f32,
    age: f32,
) {
    let r = s.map;
    let x = r.x * sw;
    let y = r.y * sh;
    let w = r.w * sw;
    let h = r.h * sh;
    if cfg[WidgetId::Map].bg > 0 {
        if let Some(rect) = rr(x, y, w, h) {
            fill_rect(
                px,
                rect,
                Color::from_rgba8(10, 10, 10, bg_a(cfg[WidgetId::Map].bg)),
            );
        }
    }
    let n = s.poly_count.max(0) as usize;
    if n < 2 {
        reset_map_follow();
        text(
            px,
            fonts,
            "No track map",
            13.0,
            x + w * 0.5,
            y + h * 0.5,
            text_dim(),
            true,
        );
        return;
    }

    let subject = camera_subject(s);
    let you = subject_pose(s, age);
    let follow = if cfg.map_follow {
        you.as_ref()
            .map(|pose| follow_frame(s, n, pose, subject, w, h))
    } else {
        None
    };

    if let Some(frame) = follow {
        let track_px = (8.0 * frame.scale).clamp(5.5, 26.0);
        let lw = w.ceil().max(1.0) as u32;
        let lh = h.ceil().max(1.0) as u32;
        if let Some(mut layer) = Pixmap::new(lw, lh) {
            paint_follow_track(&mut layer, s, cfg, n, &frame, track_px);
            let to_px = |wx: f32, wz: f32| frame.to_px(wx, wz);
            if cfg.map_sectors {
                draw_sector_lines(&mut layer, fonts, s, n, &to_px, track_px, None);
            }
            paint_map_riders(&mut layer, fonts, s, cfg, you.as_ref(), subject, &to_px);
            px.draw_pixmap(
                0,
                0,
                layer.as_ref(),
                &PixmapPaint::default(),
                Transform::from_translate(x, y),
                None,
            );
        }
        return;
    }

    reset_map_follow();
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
    let to_px =
        |wx: f32, wz: f32| -> (f32, f32) { (ox + (wx - min_x) * scale, oy + (max_z - wz) * scale) };

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
                    stroke_path(
                        &mut layer,
                        &path,
                        Color::from_rgba8(18, 16, 16, 240),
                        track_px + 3.0,
                    );
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
        draw_sector_lines(px, fonts, s, n, &to_px, track_px, None);
    }
    paint_map_riders(px, fonts, s, cfg, you.as_ref(), subject, &to_px);
}

struct FollowFrame {
    fx: f32,
    fz: f32,
    rx: f32,
    rz: f32,
    scale: f32,
    origin_x: f32,
    origin_z: f32,
    cx: f32,
    cy: f32,
}

impl FollowFrame {
    fn to_px(&self, wx: f32, wz: f32) -> (f32, f32) {
        let dx = wx - self.origin_x;
        let dz = wz - self.origin_z;
        let along = dx * self.fx + dz * self.fz;
        let right = dx * self.rx + dz * self.rz;
        (self.cx + right * self.scale, self.cy - along * self.scale)
    }
}

fn paint_map_riders(
    px: &mut Pixmap,
    fonts: &Fonts,
    s: &Snapshot,
    cfg: &HudConfig,
    you: Option<&SubjectPose>,
    subject: i32,
    to_px: &impl Fn(f32, f32) -> (f32, f32),
) {
    let leader = leader_num(s);
    let other_r = (if cfg.map_numbers { 6.8 } else { 4.6 }) * style_k();

    if cfg.map_others {
        for i in 0..s.rider_count.max(0) as usize {
            let rider = &s.riders[i];
            if you.is_some() && rider.race_num == subject {
                continue;
            }
            let pose = rider_map_pose(s, rider);
            let (hx, hy) = to_px(pose.x, pose.z);
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
            let (fwx, fwz) = yaw_forward(pose.yaw);
            let (sdx, sdy) = screen_dir(to_px, pose.x, pose.z, fwx, fwz);
            draw_dot_chevron(px, hx, hy, other_r, sdx, sdy, fill, false);
            draw_rider_overhead(
                px,
                fonts,
                s,
                rider.race_num,
                hx,
                hy,
                other_r,
                subject,
                leader,
                cfg.map_crown,
                cfg.map_place,
            );
            draw_state_mark(
                px,
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
        let (sdx, sdy) = screen_dir(to_px, pose.x, pose.z, fwx, fwz);
        draw_dot_chevron(px, hx, hy, local_r, sdx, sdy, you_col(), true);
        if cfg.map_crown && leader > 0 && subject == leader {
            crown_over_dot(px, fonts, hx, hy, local_r);
        }
        draw_state_mark(
            px,
            fonts,
            hx,
            hy,
            local_r,
            rider_mark(s, subject, pose.crashed),
        );
    }
}

fn reset_map_follow() {
    MAP_FOLLOW.with(|cell| {
        let mut ease = cell.get();
        ease.live = false;
        cell.set(ease);
    });
}

fn angle_delta(from: f32, to: f32) -> f32 {
    let pi = std::f32::consts::PI;
    (to - from + pi).rem_euclid(2.0 * pi) - pi
}

fn unrotated_fit_scale(s: &Snapshot, n: usize, w: f32, h: f32) -> f32 {
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
    let dx = (max_x - min_x).max(8.0);
    let dz = (max_z - min_z).max(8.0);
    let pad = 0.10;
    let usable_w = w * (1.0 - 2.0 * pad);
    let usable_h = h * (1.0 - 2.0 * pad);
    (usable_w / dx).min(usable_h / dz)
}

fn follow_frame(
    s: &Snapshot,
    n: usize,
    pose: &SubjectPose,
    subject: i32,
    w: f32,
    h: f32,
) -> FollowFrame {
    let travel = track_forward(s, n, pose.x, pose.z);
    let scale = unrotated_fit_scale(s, n, w, h);
    MAP_FOLLOW.with(|cell| {
        let prev = cell.get();
        let subject_changed = !prev.live || prev.subject != subject;
        let target = if let Some((fx, fz)) = travel {
            Some(fx.atan2(fz))
        } else if prev.live && !subject_changed {
            None
        } else if pose.from_local {
            let (fx, fz) = local_forward(s);
            Some(fx.atan2(fz))
        } else {
            let (fx, fz) = yaw_forward(pose.yaw);
            Some(fx.atan2(fz))
        };
        let angle = match target {
            None => prev.angle,
            Some(target_angle) => {
                let delta = angle_delta(prev.angle, target_angle);
                if subject_changed || delta.abs() > 2.2 {
                    target_angle
                } else {
                    prev.angle + delta * 0.2
                }
            }
        };
        let fx = angle.sin();
        let fz = angle.cos();
        cell.set(MapFollowEase {
            angle,
            subject,
            live: true,
        });
        FollowFrame {
            fx,
            fz,
            rx: fz,
            rz: -fx,
            scale,
            origin_x: pose.x,
            origin_z: pose.z,
            cx: w * 0.5,
            cy: h * 0.5,
        }
    })
}

fn paint_follow_track(
    px: &mut Pixmap,
    s: &Snapshot,
    cfg: &HudConfig,
    n: usize,
    frame: &FollowFrame,
    track_px: f32,
) {
    let mut pb = PathBuilder::new();
    let (sx, sy) = frame.to_px(s.poly[0].x, s.poly[0].z);
    pb.move_to(sx, sy);
    for p in s.poly.iter().take(n).skip(1) {
        let (px_, py_) = frame.to_px(p.x, p.z);
        pb.line_to(px_, py_);
    }
    pb.close();
    if let Some(path) = pb.finish() {
        let mut fill = Paint::default();
        fill.set_color(fill_col());
        fill.anti_alias = true;
        px.fill_path(&path, &fill, FillRule::EvenOdd, Transform::identity(), None);
        stroke_path(
            px,
            &path,
            Color::from_rgba8(18, 16, 16, 240),
            track_px + 3.0,
        );
        stroke_path(px, &path, track_col(), track_px);
    }
    if n >= 2 && s.sf_meters >= 0.0 && cfg.map_sf {
        draw_sf(px, s, n, |wx, wz| frame.to_px(wx, wz), track_px);
    }
    if cfg.map_arrows {
        draw_track_arrows(
            px,
            s,
            n,
            |wx, wz| frame.to_px(wx, wz),
            track_px,
            None,
            false,
        );
    }
}
