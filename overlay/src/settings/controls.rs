//! Shared Settings form controls (F8 host).
//!
//! Reuse contract for new Settings / Profile panes:
//! 1. Pass data into these helpers (label, value, options, hits) — do not re-paint row chrome.
//! 2. Add a new helper here only if the pattern appears 3+ times or is a standard form control.
//! 3. One-shot compositions (Profile spider, Motos sheet) stay screen-local; they may still use
//!    low-level `outlined` / `fill_round` / tokens from the parent module.
//!
//! Row APIs: `toggle_row`, `toggle_nested`, `slider_row`, `stepper_row`, `dropdown_row`,
//! `color_row`, `btn_icon`. Paint: `row_card`, `switch`, `switch_lg`, `draw_slider`, `chevron`,
//! `outlined`, `fill_round`, `fill_circle`, `round_path`.

#![allow(unused_imports)]
use super::*;

pub(crate) fn row_card(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, hot: bool) {
    let fill = if hot { chip_hover() } else { panel() };
    fill_round(px, x, y, w, h, 10.0, fill);
}


pub(crate) fn toggle_row(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    on: bool,
    hit: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let h = ROW_H;
    hits.push(HitBox {
        id: hit,
        x,
        y,
        w,
        h,
    });
    row_card(px, x, y, w, h, hover == Some(hit));
    text(
        px,
        fonts,
        label,
        13.0,
        x + 16.0,
        y + 16.0,
        text_col(),
        false,
    );
    switch(px, x + w - 52.0, y + 14.0, on, hit, hover, hits);
    y + h + ROW_GAP
}


pub(crate) fn toggle_nested(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    on: bool,
    hit: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    toggle_row(
        px,
        fonts,
        x + 16.0,
        y,
        (w - 16.0).max(80.0),
        label,
        on,
        hit,
        hover,
        hits,
    )
}


pub(crate) fn slider_row(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    value: i32,
    min: i32,
    max: i32,
    suffix: &str,
    hit: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let h = ROW_H;
    hits.push(HitBox {
        id: hit,
        x,
        y,
        w,
        h,
    });
    row_card(px, x, y, w, h, hover == Some(hit));
    text(
        px,
        fonts,
        label,
        13.0,
        x + 16.0,
        y + 16.0,
        text_col(),
        false,
    );
    let val_w = 44.0;
    let label_end = x + 16.0 + measure(fonts, label, 13.0) + 12.0;
    let max_end = x + w - 14.0;
    let track_w = 148.0_f32.min((max_end - val_w - label_end).max(40.0));
    let track_x = max_end - val_w - track_w;
    draw_slider(
        px,
        track_x,
        y + 16.0,
        track_w,
        16.0,
        value,
        min,
        max,
        hit,
        hover,
        hits,
    );
    text(
        px,
        fonts,
        &format!("{value}{suffix}"),
        12.0,
        track_x + track_w + 8.0,
        y + 16.0,
        muted(),
        false,
    );
    y + h + ROW_GAP
}


pub(crate) fn draw_slider(
    px: &mut Pixmap,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    value: i32,
    min: i32,
    max: i32,
    hit: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) {
    hits.push(HitBox {
        id: hit,
        x,
        y,
        w,
        h,
    });
    let span = (max - min).max(1) as f32;
    let t = ((value - min) as f32 / span).clamp(0.0, 1.0);
    let cy = y + h * 0.5;
    let track_h = 4.0;
    fill_round(px, x, cy - track_h * 0.5, w, track_h, 2.0, track_off());
    let fill_w = (w * t).max(4.0);
    fill_round(px, x, cy - track_h * 0.5, fill_w, track_h, 2.0, accent());
    let kx = x + w * t;
    let kr = if hover == Some(hit) { 7.0 } else { 6.0 };
    fill_circle(px, kx, cy + 1.0, kr + 1.0, Color::from_rgba8(0, 0, 0, 70));
    fill_circle(px, kx, cy, kr, knob());
}


pub(crate) fn stepper_row(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    value: &str,
    dec: Hit,
    inc: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let h = ROW_H;
    row_card(px, x, y, w, h, hover == Some(dec) || hover == Some(inc));
    text(
        px,
        fonts,
        label,
        13.0,
        x + 16.0,
        y + 16.0,
        text_col(),
        false,
    );
    let bw = 36.0;
    let bh = 36.0;
    let by = y + 6.0;
    let ix = x + w - bw - 14.0;
    let dx = ix - 86.0 - bw;
    btn_icon(px, fonts, dx, by, bw, bh, '\u{f068}', dec, hover, hits);
    text(
        px,
        fonts,
        value,
        13.0,
        dx + bw + 43.0,
        y + 16.0,
        text_col(),
        true,
    );
    btn_icon(px, fonts, ix, by, bw, bh, '\u{f067}', inc, hover, hits);
    y + h + ROW_GAP
}


pub(crate) fn dropdown_row(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    value: &str,
    open: bool,
    open_hit: Hit,
    options: &[(Hit, &'static str, bool)],
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let h = ROW_H;
    hits.push(HitBox {
        id: open_hit,
        x,
        y,
        w,
        h,
    });
    row_card(px, x, y, w, h, open || hover == Some(open_hit));
    text(
        px,
        fonts,
        label,
        13.0,
        x + 16.0,
        y + 16.0,
        text_col(),
        false,
    );
    let label_w = measure(fonts, label, 13.0);
    let bw = (w - 30.0 - label_w - 16.0).clamp(88.0, 160.0);
    let bh = 28.0;
    let bx = x + w - bw - 14.0;
    let by = y + 10.0;
    hits.push(HitBox {
        id: open_hit,
        x: bx,
        y: by,
        w: bw,
        h: bh,
    });
    let hot = open || hover == Some(open_hit);
    outlined(
        px,
        bx,
        by,
        bw,
        bh,
        7.0,
        if hot { chip_hover() } else { bg() },
    );
    text(
        px,
        fonts,
        value,
        12.0,
        bx + 10.0,
        by + 6.0,
        text_col(),
        false,
    );
    chevron(px, bx + bw - 14.0, by + bh * 0.5, open, muted());
    if open {
        let options = sorted_drop_options(options);
        let item_h = 28.0;
        let pad = 5.0;
        let content_h = pad * 2.0 + item_h * options.len() as f32;
        DROP_MENUS.with(|menus| {
            menus.borrow_mut().push(PendingDrop {
                mx: bx,
                my: by + bh + 6.0,
                bw,
                content_h,
                open_hit,
                options,
            });
        });
    }
    y + h + ROW_GAP
}

pub(crate) fn color_row(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    rgb: [u8; 3],
    open: bool,
    kind: ColorPickKind,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let h = ROW_H;
    let open_hit = kind.open();
    let reset_hit = kind.reset();
    hits.push(HitBox {
        id: open_hit,
        x,
        y,
        w,
        h,
    });
    row_card(
        px,
        x,
        y,
        w,
        h,
        open || hover == Some(open_hit) || hover == Some(reset_hit),
    );
    text(
        px,
        fonts,
        kind.label(),
        13.0,
        x + 16.0,
        y + 16.0,
        text_col(),
        false,
    );
    let hex = format_primary_color(rgb);
    let label_w = measure(fonts, kind.label(), 13.0);
    let bh = 28.0;
    let by = y + 10.0;
    let reset_label = "Reset";
    let rw = (measure(fonts, reset_label, 12.0) + 20.0).max(56.0);
    let gap = 8.0;
    let label_end = x + 16.0 + label_w + 12.0;
    let right = x + w - 14.0;
    let bw = (right - label_end - rw - gap).clamp(108.0, 160.0);
    let bx = right - bw;
    let rx = bx - gap - rw;
    hits.push(HitBox {
        id: open_hit,
        x: bx,
        y: by,
        w: bw,
        h: bh,
    });
    let hot = open || hover == Some(open_hit);
    outlined(
        px,
        bx,
        by,
        bw,
        bh,
        7.0,
        if hot { chip_hover() } else { bg() },
    );
    fill_round(
        px,
        bx + 8.0,
        by + 5.0,
        18.0,
        18.0,
        5.0,
        Color::from_rgba8(rgb[0], rgb[1], rgb[2], 255),
    );
    if let Some(path) = round_path(bx + 8.0, by + 5.0, 18.0, 18.0, 5.0) {
        let mut p = Paint::default();
        p.set_color(Color::from_rgba8(255, 255, 255, 40));
        p.anti_alias = true;
        px.stroke_path(
            &path,
            &p,
            &Stroke {
                width: 1.0,
                ..Stroke::default()
            },
            Transform::identity(),
            None,
        );
    }
    text(
        px,
        fonts,
        &hex,
        12.0,
        bx + 32.0,
        by + 6.0,
        text_col(),
        false,
    );
    chevron(px, bx + bw - 14.0, by + bh * 0.5, open, muted());
    hits.push(HitBox {
        id: reset_hit,
        x: rx,
        y: by,
        w: rw,
        h: bh,
    });
    outlined(
        px,
        rx,
        by,
        rw,
        bh,
        7.0,
        if hover == Some(reset_hit) {
            chip_hover()
        } else {
            bg()
        },
    );
    text(
        px,
        fonts,
        reset_label,
        12.0,
        rx + 10.0,
        by + 6.0,
        text_col(),
        false,
    );
    if open {
        COLOR_PICKER_ANCHOR.with(|a| a.set(Some((bx, by, bw, bh))));
    }
    y + h + ROW_GAP
}


pub(crate) fn sorted_drop_options(options: &[(Hit, &'static str, bool)]) -> Vec<(Hit, String, bool)> {
    let mut options: Vec<(Hit, String, bool)> = options
        .iter()
        .map(|(hit, name, on)| (*hit, (*name).to_string(), *on))
        .collect();
    options.sort_by(|a, b| {
        match (
            a.1.eq_ignore_ascii_case("none"),
            b.1.eq_ignore_ascii_case("none"),
        ) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.1.to_ascii_lowercase().cmp(&b.1.to_ascii_lowercase()),
        }
    });
    options
}


pub(crate) fn chevron(px: &mut Pixmap, cx: f32, cy: f32, open: bool, c: Color) {
    let mut pb = PathBuilder::new();
    if open {
        pb.move_to(cx - 4.5, cy + 2.0);
        pb.line_to(cx + 4.5, cy + 2.0);
        pb.line_to(cx, cy - 3.0);
    } else {
        pb.move_to(cx - 4.5, cy - 2.0);
        pb.line_to(cx + 4.5, cy - 2.0);
        pb.line_to(cx, cy + 3.0);
    }
    pb.close();
    let Some(path) = pb.finish() else {
        return;
    };
    let mut p = Paint::default();
    p.set_color(c);
    p.anti_alias = true;
    px.fill_path(&path, &p, FillRule::Winding, Transform::identity(), None);
}


pub(crate) fn switch_lg(
    px: &mut Pixmap,
    x: f32,
    y: f32,
    on: bool,
    hit: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) {
    let w = 52.0;
    let h = 28.0;
    hits.push(HitBox {
        id: hit,
        x,
        y,
        w,
        h,
    });
    let mut track = if on { accent() } else { track_off() };
    if hover == Some(hit) && !on {
        track = track_idle();
    }
    fill_round(px, x, y, w, h, 14.0, track);
    let kx = if on { x + w - 14.0 } else { x + 14.0 };
    fill_circle(
        px,
        kx,
        y + h * 0.5 + 0.8,
        9.0,
        Color::from_rgba8(0, 0, 0, 50),
    );
    fill_circle(px, kx, y + h * 0.5, 8.5, knob());
}


pub(crate) fn switch(
    px: &mut Pixmap,
    x: f32,
    y: f32,
    on: bool,
    hit: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) {
    let w = 38.0;
    let h = 20.0;
    hits.push(HitBox {
        id: hit,
        x,
        y,
        w,
        h,
    });
    let mut track = if on { accent() } else { track_off() };
    if hover == Some(hit) && !on {
        track = track_idle();
    }
    fill_round(px, x, y, w, h, 10.0, track);
    let kx = if on { x + w - 10.0 } else { x + 10.0 };
    fill_circle(
        px,
        kx,
        y + h * 0.5 + 0.8,
        7.2,
        Color::from_rgba8(0, 0, 0, 50),
    );
    fill_circle(px, kx, y + h * 0.5, 7.0, knob());
}


pub(crate) fn btn_icon(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    ch: char,
    hit: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) {
    hits.push(HitBox {
        id: hit,
        x,
        y,
        w,
        h,
    });
    let fill = if hover == Some(hit) {
        chip_hover()
    } else {
        panel()
    };
    outlined(px, x, y, w, h, 7.0, fill);
    icon(px, fonts, ch, 13.0, x + w * 0.5, y + 5.5, text_col(), true);
}


pub(crate) fn outlined(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, r: f32, fill: Color) {
    fill_round(px, x, y, w, h, r, btn_border());
    fill_round(
        px,
        x + 1.0,
        y + 1.0,
        (w - 2.0).max(1.0),
        (h - 2.0).max(1.0),
        (r - 1.0).max(0.0),
        fill,
    );
}


pub(crate) fn fill_round(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, r: f32, c: Color) {
    let Some(path) = round_path(x, y, w, h, r) else {
        return;
    };
    let mut p = Paint::default();
    p.set_color(c);
    p.anti_alias = true;
    px.fill_path(&path, &p, FillRule::Winding, Transform::identity(), None);
}


pub(crate) fn fill_circle(px: &mut Pixmap, cx: f32, cy: f32, r: f32, c: Color) {
    let mut pb = PathBuilder::new();
    pb.push_circle(cx, cy, r);
    let Some(path) = pb.finish() else {
        return;
    };
    let mut p = Paint::default();
    p.set_color(c);
    p.anti_alias = true;
    px.fill_path(&path, &p, FillRule::Winding, Transform::identity(), None);
}


pub(crate) fn round_path(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<Path> {
    let r = r.min(w * 0.5).min(h * 0.5);
    let mut pb = PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.quad_to(x + w, y, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.quad_to(x + w, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.quad_to(x, y + h, x, y + h - r);
    pb.line_to(x, y + r);
    pb.quad_to(x, y, x + r, y);
    pb.close();
    pb.finish()
}

