#![allow(unused_imports)]
use super::*;

pub(crate) fn pane_feedback_tab(
    px: &mut Pixmap,
    fonts: &Fonts,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let y = heading(
        px,
        fonts,
        x,
        y,
        w,
        "Feedback",
        "Rate the app, report a bug, or ask for a feature",
        None,
        hover,
        hits,
    );
    pane_feedback(px, fonts, hover, hits, x, y, w)
}

pub(crate) fn pane_feedback(
    px: &mut Pixmap,
    fonts: &Fonts,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let fb = crate::feedback::snapshot();
    let bug = fb.kind == crate::feedback::Kind::Bug;
    let feature = fb.kind == crate::feedback::Kind::Feature;
    let show_stars = !feature;
    let attach_h = if bug { 52.0 } else { 0.0 };
    let stars_h = if show_stars { 56.0 } else { 8.0 };
    let card_h = 224.0 + stars_h + attach_h;
    outlined(px, x, y, w, card_h, 10.0, panel());

    let gap = 8.0;
    let chip_w = (w - 32.0 - gap * 2.0) / 3.0;
    let cy = y + 14.0;
    kind_chip(px, fonts, x + 16.0, cy, chip_w, "Rate", Hit::FbRate, fb.kind == crate::feedback::Kind::Rate, hover, hits);
    kind_chip(px, fonts, x + 16.0 + chip_w + gap, cy, chip_w, "Bug", Hit::FbBug, bug, hover, hits);
    kind_chip(px, fonts, x + 16.0 + (chip_w + gap) * 2.0, cy, chip_w, "Feature", Hit::FbFeature, feature, hover, hits);

    let prompt = match fb.kind {
        crate::feedback::Kind::Bug => "How bad is it? (optional)",
        crate::feedback::Kind::Feature => "What should we add?",
        crate::feedback::Kind::Rate => "How is it going?",
    };
    let sy = y + 56.0;
    text(px, fonts, prompt, 11.0, x + 16.0, sy, dim(), false);
    let mut box_y = sy + 24.0;
    if show_stars {
        let star_y = sy + 20.0;
        for i in 1u8..=5 {
            let sx = x + 12.0 + (i as f32 - 1.0) * 34.0;
            hits.push(HitBox { id: Hit::FbStar(i), x: sx, y: star_y, w: 32.0, h: 28.0 });
            let on = fb.rating >= i;
            let hot = hover == Some(Hit::FbStar(i));
            let col = if on {
                accent()
            } else if hot {
                Color::from_rgba8(255, 140, 36, 140)
            } else {
                Color::from_rgba8(72, 72, 80, 255)
            };
            fill_star(px, sx + 16.0, star_y + 14.0, 10.0, col);
        }
        box_y = star_y + 36.0;
    }
    let box_h = 88.0;
    crate::feedback::set_text_rect(x + 16.0, box_y, w - 32.0, box_h);
    hits.push(HitBox { id: Hit::FbText, x: x + 16.0, y: box_y, w: w - 32.0, h: box_h });
    let box_fill = if fb.focused {
        Color::from_rgba8(20, 20, 24, 255)
    } else {
        btn_bg()
    };
    outlined(px, x + 16.0, box_y, w - 32.0, box_h, 8.0, box_fill);
    let placeholder = match fb.kind {
        crate::feedback::Kind::Bug => "What went wrong?",
        crate::feedback::Kind::Feature => "Describe the feature.",
        crate::feedback::Kind::Rate => "Anything you want to add? (optional)",
    };
    let tx = x + 28.0;
    let ty = box_y + 10.0;
    let tw = w - 56.0;
    if fb.message.is_empty() && !fb.focused {
        text(px, fonts, placeholder, 12.0, tx, ty + 2.0, dim(), false);
        crate::feedback::set_caret_layout(tx, ty, 16.0, vec![vec![(0, 0.0)]]);
    } else {
        draw_fb_text(px, fonts, &fb.message, fb.cursor, tx, ty, tw, box_h - 16.0);
    }

    let mut iy = box_y + box_h + 12.0;
    if bug {
        hits.push(HitBox { id: Hit::FbAttach, x: x + 16.0, y: iy, w: w - 32.0, h: 44.0 });
        let check_col = if fb.attach_log { accent() } else { track_off() };
        fill_round(px, x + 16.0, iy + 6.0, 18.0, 18.0, 4.0, check_col);
        if fb.attach_log {
            let mut pb = PathBuilder::new();
            pb.move_to(x + 20.0, iy + 15.0);
            pb.line_to(x + 24.0, iy + 19.0);
            pb.line_to(x + 30.0, iy + 11.0);
            if let Some(path) = pb.finish() {
                let mut p = Paint::default();
                p.set_color(Color::from_rgba8(20, 12, 4, 255));
                p.anti_alias = true;
                px.stroke_path(
                    &path,
                    &p,
                    &Stroke { width: 2.0, ..Stroke::default() },
                    Transform::identity(),
                    None,
                );
            }
        }
        let log_c = if crate::feedback::has_log() { muted() } else { dim() };
        text(px, fonts, "Include last race log", 13.0, x + 42.0, iy + 8.0, text_col(), false);
        text(px, fonts, &crate::feedback::log_label(), 11.0, x + 42.0, iy + 24.0, log_c, false);
        iy += 52.0;
    }

    let sending = matches!(fb.status, crate::feedback::Status::Sending);
    action_btn(px, fonts, x + 16.0, iy, 120.0, 32.0, if sending { "Sending…" } else { "Send" }, Hit::FbSend, hover, hits, true);
    let (status, status_c) = match &fb.status {
        crate::feedback::Status::Idle => ("", muted()),
        crate::feedback::Status::Sending => ("Sending…", muted()),
        crate::feedback::Status::Sent => ("Sent. Thank you.", accent()),
        crate::feedback::Status::Error(msg) => (msg.as_str(), Color::from_rgba8(255, 120, 100, 255)),
    };
    if !status.is_empty() {
        text(px, fonts, status, 11.0, x + 148.0, iy + 10.0, status_c, false);
    }
    y + card_h + 14.0
}
