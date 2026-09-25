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
    recording: bool,
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
    let record_anchor = profile_record_switch(px, fonts, x, y, recording, hover, hits);
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
        if hover == Some(Hit::ReviewToggle) {
            paint_record_tip(px, fonts, x, w, record_anchor, recording);
        }
        return y + 24.0;
    }

    let gap = 12.0;
    let flank_w = ((w - gap * 2.0) * 0.22).clamp(100.0, 140.0);
    let mid_w = (w - flank_w * 2.0 - gap * 2.0).max(180.0);
    let card_h = (px.height() as f32 - y - 28.0).clamp(240.0, 420.0);
    let left_x = x;
    let mid_x = x + flank_w + gap;
    let right_x = mid_x + mid_w + gap;

    let left = [
        ("Wins", p.wins.to_string()),
        ("Podium rate", fmt_pct(p.podium_rate)),
        ("Avg finish", fmt_place(p.avg_finish)),
    ];
    let mut right = vec![
        ("Crashes per lap", fmt_crash_per_lap(p.crash_rate)),
        ("Holeshot rate", fmt_pct(p.holeshot_rate)),
        ("Avg Penalty Seconds", fmt_pen(p.avg_penalty_ms)),
    ];
    if p.dns_count > 0 {
        right.push(("Did not start", fmt_pct(p.dns_rate)));
    }
    if p.dnf_count > 0 {
        right.push(("Did not finish", fmt_pct(p.dnf_rate)));
    }
    if p.dsq_count > 0 {
        right.push(("Disqualified", fmt_pct(p.dsq_rate)));
    }
    paint_kpi_stack(px, fonts, left_x, y, flank_w, card_h, &left);
    let tip = paint_spider_card(
        px,
        fonts,
        hover,
        hits,
        mid_x,
        y,
        mid_w,
        card_h,
        &p.scores,
        p.all_time_count,
    );
    paint_kpi_stack(px, fonts, right_x, y, flank_w, card_h, &right);
    if let Some(tip) = tip {
        paint_axis_tip(
            px,
            fonts,
            tip.pane_x,
            tip.pane_w,
            tip.lx,
            tip.ly,
            tip.cy,
            tip.i,
            tip.score,
        );
    }
    if hover == Some(Hit::ReviewToggle) {
        paint_record_tip(px, fonts, x, w, record_anchor, recording);
    }
    y + card_h + 16.0
}

fn paint_kpi_stack(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    rows: &[(&str, String)],
) {
    if rows.is_empty() {
        return;
    }
    let gap = 8.0;
    let n = rows.len() as f32;
    let row_h = ((h - gap * (n - 1.0)) / n).max(48.0);
    let mut cy = y;
    for (label, value) in rows {
        outlined(px, x, cy, w, row_h, 8.0, panel());
        let label = super::ellipsize_heading(fonts, label, 11.0, (w - 24.0).max(8.0));
        text(px, fonts, &label, 11.0, x + 12.0, cy + 10.0, dim(), false);
        let vw = measure(fonts, value, 20.0);
        text(
            px,
            fonts,
            value,
            20.0,
            x + ((w - vw) * 0.5).max(12.0),
            cy + row_h * 0.45,
            text_col(),
            false,
        );
        cy += row_h + gap;
    }
}

fn paint_spider_card(
    px: &mut Pixmap,
    fonts: &Fonts,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    scores: &[Option<f32>; AXIS_COUNT],
    all_time_count: i32,
) -> Option<SpiderTip> {
    outlined(px, x, y, w, h, 10.0, panel());
    let cx = x + w * 0.5;
    let cy = y + h * 0.52;
    let radius = (w.min(h) * 0.28).clamp(70.0, 130.0);
    let tip = draw_spider(px, fonts, hover, hits, x, w, cx, cy, radius, scores);
    if all_time_count < PROFILE_LOCK_RACES {
        let msg = format!("{} of {} races", all_time_count, PROFILE_LOCK_RACES);
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
    tip
}

struct SpiderTip {
    i: u8,
    lx: f32,
    ly: f32,
    cy: f32,
    pane_x: f32,
    pane_w: f32,
    score: Option<f32>,
}

fn fmt_pct(v: Option<f32>) -> String {
    match v {
        Some(x) => format!("{}%", (x * 100.0).round() as i32),
        None => "N/A".into(),
    }
}

fn fmt_crash_per_lap(v: Option<f32>) -> String {
    match v {
        Some(x) => format!("{x:.1}"),
        None => "N/A".into(),
    }
}

fn fmt_place(v: Option<f32>) -> String {
    match v {
        Some(x) => format!("P{:.1}", x),
        None => "N/A".into(),
    }
}

fn fmt_pen(ms: Option<f32>) -> String {
    match ms {
        Some(m) if m >= 500.0 => format!("{}s", (m / 1000.0).round() as i32),
        Some(_) => "0s".into(),
        None => "N/A".into(),
    }
}

fn profile_record_switch(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    recording: bool,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> (f32, f32, f32) {
    let label = "Record";
    let label_w = measure(fonts, label, 13.0);
    let hit = Hit::ReviewToggle;
    let right_w = label_w + 14.0 + 52.0;
    text(px, fonts, label, 13.0, x, y + 6.0, text_col(), false);
    hits.push(HitBox {
        id: hit,
        x,
        y,
        w: right_w,
        h: 28.0,
    });
    switch_lg(
        px,
        x + label_w + 12.0,
        y,
        recording,
        hit,
        hover,
        hits,
    );
    (x, y, right_w)
}

pub(crate) fn profile_record_tip(recording: bool) -> &'static str {
    if recording {
        "Recording races. Turn off to stop saving new visits."
    } else {
        "Saves race visits to Motos for Analyze. Practice and spectate are skipped."
    }
}

fn paint_record_tip(
    px: &mut Pixmap,
    fonts: &Fonts,
    pane_x: f32,
    pane_w: f32,
    anchor: (f32, f32, f32),
    recording: bool,
) {
    let (ax, ay, aw) = anchor;
    let copy = profile_record_tip(recording);
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
    let tx = (ax + aw * 0.5 - tw * 0.5).clamp(pane_x, max_x);
    let ty = ay + 28.0 + 4.0;
    outlined(px, tx, ty, tw, th, 6.0, menu_fill());
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

pub(crate) fn profile_chip(
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
            fill_round(px, x, y, bw, 28.0, 8.0, hover_wash());
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
) -> Option<SpiderTip> {
    let n = AXIS_COUNT as f32;
    let ring = spider_grid();
    for step in 1..=4 {
        spider_ring(px, cx, cy, radius * step as f32 / 4.0, ring);
    }
    let mut tip: Option<SpiderTip> = None;
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
            tip = Some(SpiderTip {
                i: i as u8,
                lx,
                ly,
                cy,
                pane_x,
                pane_w,
                score: scores[i],
            });
        }
    }
    let mut pb = PathBuilder::new();
    let mut verts = [(0.0f32, 0.0f32); AXIS_COUNT];
    for i in 0..AXIS_COUNT {
        let v = scores[i].unwrap_or(0.0).clamp(0.0, 1.0);
        let (x, y) = spoke_pt(cx, cy, radius * v.max(0.04), i, n);
        verts[i] = (x, y);
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
    for i in 0..AXIS_COUNT {
        if scores[i].is_none() {
            continue;
        }
        let (x, y) = verts[i];
        fill_circle(px, x, y, 4.0, accent());
        let hit = Hit::ProfileAxis(i as u8);
        let hs = 20.0;
        hits.push(HitBox {
            id: hit,
            x: x - hs * 0.5,
            y: y - hs * 0.5,
            w: hs,
            h: hs,
        });
        if hover == Some(hit) {
            tip = Some(SpiderTip {
                i: i as u8,
                lx: x,
                ly: y,
                cy,
                pane_x,
                pane_w,
                score: scores[i],
            });
        }
    }
    tip
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
    score: Option<f32>,
) {
    let base = profile_axis_tip(i);
    if base.is_empty() {
        return;
    }
    let copy = match score {
        Some(v) => format!("{}% · {base}", (v.clamp(0.0, 1.0) * 100.0).round() as i32),
        None => base.to_string(),
    };
    let pad_x = 8.0;
    let pad_y = 6.0;
    let size = 11.0;
    let line_h = 14.0;
    let max_inner = ((pane_w - 16.0).min(240.0) - pad_x * 2.0).max(40.0);
    let lines = wrap_fb(fonts, &copy, max_inner, size);
    let inner = lines
        .iter()
        .map(|l| measure(fonts, l, size))
        .fold(0.0_f32, f32::max);
    let tw = inner + pad_x * 2.0;
    let th = lines.len() as f32 * line_h + pad_y * 2.0;
    let max_x = (pane_x + pane_w - tw).max(pane_x);
    let tx = (lx - tw * 0.5).clamp(pane_x, max_x);
    let ty = if ly < cy { ly + 14.0 } else { ly - 14.0 - th };
    outlined(px, tx, ty, tw, th, 6.0, menu_fill());
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

fn spider_grid() -> Color {
    if crate::config::settings_theme_light() {
        Color::from_rgba8(0, 0, 0, 40)
    } else {
        Color::from_rgba8(255, 255, 255, 28)
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
