#![allow(unused_imports)]
use super::*;
use mxbo_review::{ProfileWindow, AXIS_COUNT, AXIS_LABELS, PROFILE_LOCK_RACES};

pub(crate) fn pane_profile(
    px: &mut Pixmap,
    fonts: &Fonts,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
    all_time: bool,
) -> f32 {
    let window = if all_time {
        ProfileWindow::AllTime
    } else {
        ProfileWindow::TwoWeeks
    };
    let p = crate::review::profile(window);
    let mut y = y + 8.0;
    let name = if p.name.is_empty() {
        "YOU"
    } else {
        p.name.as_str()
    };
    let title = 32.0;
    let tw = measure(fonts, name, title);
    text(
        px,
        fonts,
        name,
        title,
        x + ((w - tw) * 0.5).max(0.0),
        y,
        text_col(),
        false,
    );
    y += 42.0;
    let sub = if p.race_count == 1 {
        "1 race moto".to_string()
    } else {
        format!("{} race motos", p.race_count)
    };
    let sw = measure(fonts, &sub, 13.0);
    text(
        px,
        fonts,
        &sub,
        13.0,
        x + ((w - sw) * 0.5).max(0.0),
        y,
        dim(),
        false,
    );
    y += 28.0;
    if p.holeshots > 0 {
        let hs = if p.holeshots == 1 {
            "1 holeshot".to_string()
        } else {
            format!("{} holeshots", p.holeshots)
        };
        let hw = measure(fonts, &hs, 13.0);
        text(
            px,
            fonts,
            &hs,
            13.0,
            x + ((w - hw) * 0.5).max(0.0),
            y,
            dim(),
            false,
        );
        y += 22.0;
    }
    let chips = [
        ("All time", all_time, Hit::ProfileAllTime),
        ("14 days", !all_time, Hit::ProfileTwoWeeks),
    ];
    let mut total = 0.0;
    for (label, _, _) in &chips {
        total += (measure(fonts, label, 13.0) + 28.0).max(64.0) + 8.0;
    }
    total = (total - 8.0).max(0.0);
    let mut cx = x + ((w - total) * 0.5).max(0.0);
    for (label, on, hit) in chips {
        cx += profile_chip(px, fonts, cx, y, label, on, hit, hover, hits) + 8.0;
    }
    if p.all_time_count > 0 {
        let bw = (measure(fonts, "Clear", 13.0) + 28.0).max(64.0);
        profile_chip(
            px,
            fonts,
            x + (w - bw).max(0.0),
            y,
            "Clear",
            false,
            Hit::ProfileClear,
            hover,
            hits,
        );
    }
    y += 44.0;
    if p.all_time_count == 0 {
        let copy = "Finish a race with a field. Practice, warmup, and solo sessions do not count.";
        for line in wrap_fb(fonts, copy, (w - 24.0).max(40.0), 13.0) {
            let lw = measure(fonts, &line, 13.0);
            text(
                px,
                fonts,
                &line,
                13.0,
                x + ((w - lw) * 0.5).max(0.0),
                y,
                muted(),
                false,
            );
            y += 20.0;
        }
        return y + 24.0;
    }
    let chart_h = (px.height() as f32 - y - 36.0).clamp(220.0, 420.0);
    let cx = x + w * 0.5;
    let cy = y + chart_h * 0.48;
    let radius = (w.min(chart_h) * 0.32).clamp(90.0, 150.0);
    draw_spider(px, fonts, hover, hits, x, w, cx, cy, radius, &p.scores);
    if p.all_time_count < PROFILE_LOCK_RACES {
        let msg = format!("{} of {} races", p.all_time_count, PROFILE_LOCK_RACES);
        let mw = measure(fonts, &msg, 14.0);
        text(
            px,
            fonts,
            &msg,
            14.0,
            cx - mw * 0.5,
            cy - 8.0,
            text_col(),
            false,
        );
    }
    y + chart_h
}

fn profile_chip(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    label: &str,
    on: bool,
    hit: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let bw = (measure(fonts, label, 13.0) + 28.0).max(64.0);
    hits.push(HitBox {
        id: hit,
        x,
        y,
        w: bw,
        h: 28.0,
    });
    if on {
        fill_skew(px, x, y, bw - 6.0, 28.0, 6.0, accent());
        text(px, fonts, label, 13.0, x + 14.0, y + 6.0, ink(), false);
    } else {
        if hover == Some(hit) {
            fill_round(
                px,
                x,
                y,
                bw,
                28.0,
                8.0,
                Color::from_rgba8(255, 255, 255, 10),
            );
        }
        text(px, fonts, label, 13.0, x + 14.0, y + 6.0, text_col(), false);
    }
    bw
}

pub(crate) fn profile_axis_tip(i: u8) -> &'static str {
    match i {
        0 => "Your best lap vs the fastest in that race.",
        1 => "How even your lap times stay.",
        2 => "Laps finished without a crash or cut.",
        3 => "Whether you get faster at the end of the moto.",
        4 => "How often the throttle is pinned.",
        5 => "How much time you spend in the air.",
        6 => "Where you finish against the field.",
        7 => "First lap vs later laps, not holeshot.",
        _ => "",
    }
}

fn draw_spider(
    px: &mut Pixmap,
    fonts: &Fonts,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    pane_x: f32,
    pane_w: f32,
    cx: f32,
    cy: f32,
    radius: f32,
    scores: &[Option<f32>; AXIS_COUNT],
) {
    let n = AXIS_COUNT as f32;
    let ring = Color::from_rgba8(255, 255, 255, 28);
    for step in 1..=4 {
        spider_ring(px, cx, cy, radius * step as f32 / 4.0, ring);
    }
    let mut tip: Option<(u8, f32, f32)> = None;
    for i in 0..AXIS_COUNT {
        let (ox, oy) = spoke_pt(cx, cy, radius, i, n);
        icon_stroke_line(px, cx, cy, ox, oy, ring, 1.0);
        let (lx, ly) = spoke_pt(cx, cy, radius + 22.0, i, n);
        let label = AXIS_LABELS[i];
        let hit = Hit::ProfileAxis(i as u8);
        let hot = hover == Some(hit);
        let lw = measure(fonts, label, 10.0);
        let col = if hot { accent() } else { text_col() };
        text(px, fonts, label, 10.0, lx - lw * 0.5, ly - 5.0, col, false);
        let pad = 14.0;
        let x0 = lx.min(ox) - pad;
        let y0 = ly.min(oy) - pad;
        let x1 = lx.max(ox) + pad.max(lw * 0.5);
        let y1 = ly.max(oy) + pad;
        hits.push(HitBox {
            id: hit,
            x: x0,
            y: y0,
            w: (x1 - x0).max(36.0),
            h: (y1 - y0).max(22.0),
        });
        if hot {
            tip = Some((i as u8, lx, ly));
        }
    }
    let mut pb = PathBuilder::new();
    for i in 0..AXIS_COUNT {
        let v = scores[i].unwrap_or(0.0).clamp(0.0, 1.0);
        let (x, y) = spoke_pt(cx, cy, radius * v.max(0.04), i, n);
        if i == 0 {
            pb.move_to(x, y);
        } else {
            pb.line_to(x, y);
        }
    }
    pb.close();
    if let Some(path) = pb.finish() {
        let mut p = Paint::default();
        p.set_color(accent_a(40));
        p.anti_alias = true;
        px.fill_path(&path, &p, FillRule::Winding, Transform::identity(), None);
        icon_stroke(px, &path, accent(), 2.0);
    }
    if let Some((i, lx, ly)) = tip {
        paint_axis_tip(px, fonts, pane_x, pane_w, lx, ly, cy, i);
    }
}

fn paint_axis_tip(
    px: &mut Pixmap,
    fonts: &Fonts,
    pane_x: f32,
    pane_w: f32,
    lx: f32,
    ly: f32,
    cy: f32,
    i: u8,
) {
    let copy = profile_axis_tip(i);
    if copy.is_empty() {
        return;
    }
    let pad_x = 8.0;
    let pad_y = 6.0;
    let size = 11.0;
    let line_h = 14.0;
    let max_inner = ((pane_w - 16.0).min(240.0) - pad_x * 2.0).max(40.0);
    let lines = wrap_fb(fonts, copy, max_inner, size);
    let inner = lines
        .iter()
        .map(|l| measure(fonts, l, size))
        .fold(0.0_f32, f32::max);
    let tw = inner + pad_x * 2.0;
    let th = lines.len() as f32 * line_h + pad_y * 2.0;
    let max_x = (pane_x + pane_w - tw).max(pane_x);
    let tx = (lx - tw * 0.5).clamp(pane_x, max_x);
    let ty = if ly < cy { ly + 14.0 } else { ly - 14.0 - th };
    outlined(px, tx, ty, tw, th, 6.0, Color::from_rgba8(10, 10, 10, 255));
    for (n, line) in lines.iter().enumerate() {
        text(
            px,
            fonts,
            line,
            size,
            tx + tw * 0.5,
            ty + pad_y + n as f32 * line_h,
            text_col(),
            true,
        );
    }
}

fn spider_ring(px: &mut Pixmap, cx: f32, cy: f32, r: f32, c: Color) {
    let n = AXIS_COUNT as f32;
    let mut pb = PathBuilder::new();
    for i in 0..AXIS_COUNT {
        let (x, y) = spoke_pt(cx, cy, r, i, n);
        if i == 0 {
            pb.move_to(x, y);
        } else {
            pb.line_to(x, y);
        }
    }
    pb.close();
    if let Some(path) = pb.finish() {
        icon_stroke(px, &path, c, 1.0);
    }
}

fn spoke_pt(cx: f32, cy: f32, r: f32, i: usize, n: f32) -> (f32, f32) {
    let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::TAU / n;
    (cx + r * a.cos(), cy + r * a.sin())
}
