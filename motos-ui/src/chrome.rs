//! Settings chrome shared by Motos list + Analyze (no Windows deps).

use std::cell::RefCell;

use mxbo_hud::render::{self, Fonts};
use tiny_skia::{
    Color, FillRule, LineCap, LineJoin, Paint, Path, PathBuilder, Pixmap, Rect, Stroke, Transform,
};

pub(crate) use mxbo_hud::render::icon;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    ReviewFilterAll,
    ReviewFilterRace,
    ReviewFilterPractice,
    ReviewFilterSaved,
    ReviewOpen(u64),
    ReviewKeep(u64),
    ReviewDelete(u64),
    ReviewBack,
    ReviewToggle,
    AnalyzeCompare(i32),
    AnalyzeCompareOpen,
    AnalyzeYouLap(i32),
    AnalyzeLapOpen,
    AnalyzeScrub,
    AnalyzeMap,
    AnalyzeFollow,
    TabWidgets,
    TabMotos,
}

#[derive(Clone, Copy)]
pub struct HitBox {
    pub id: Hit,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drop {
    AnalyzeCompare,
    AnalyzeLap,
}

pub(crate) fn ink() -> Color {
    Color::from_rgba8(10, 10, 10, 255)
}

pub(crate) fn accent() -> Color {
    Color::from_rgba8(232, 106, 21, 255)
}

pub(crate) fn muted() -> Color {
    Color::from_rgba8(160, 160, 168, 255)
}

pub(crate) fn caution() -> Color {
    Color::from_rgba8(255, 196, 72, 255)
}

pub(crate) fn text_col() -> Color {
    Color::from_rgba8(240, 240, 242, 255)
}

pub(crate) fn side() -> Color {
    Color::from_rgba8(28, 28, 32, 255)
}

pub(crate) fn text(
    px: &mut Pixmap,
    fonts: &Fonts,
    s: &str,
    sz: f32,
    x: f32,
    y: f32,
    c: Color,
    bold: bool,
) {
    render::text(px, fonts, s, sz, x, y, c, bold);
}

pub(crate) fn measure(fonts: &Fonts, s: &str, sz: f32) -> f32 {
    render::measure(fonts, s, sz)
}

pub(crate) fn fill_rect(px: &mut Pixmap, r: Rect, c: Color) {
    render::fill_rect(px, r, c);
}

pub(crate) fn wrap_fb(fonts: &Fonts, s: &str, max_w: f32, size: f32) -> Vec<String> {
    let mut lines = Vec::new();
    if s.is_empty() {
        lines.push(String::new());
        return lines;
    }
    for para in s.split('\n') {
        if para.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut line = String::new();
        let mut line_w = 0.0;
        for token in para.split_inclusive(' ') {
            push_wrap_token(
                fonts,
                &mut lines,
                &mut line,
                &mut line_w,
                token,
                max_w,
                size,
            );
        }
        lines.push(line);
    }
    lines
}

fn push_wrap_token(
    fonts: &Fonts,
    lines: &mut Vec<String>,
    line: &mut String,
    line_w: &mut f32,
    token: &str,
    max_w: f32,
    size: f32,
) {
    let token_w = measure(fonts, token, size);
    if !line.is_empty() && *line_w + token_w > max_w {
        lines.push(std::mem::take(line));
        *line_w = 0.0;
    }
    if token_w <= max_w {
        line.push_str(token);
        *line_w += token_w;
        return;
    }
    for ch in token.chars() {
        let ch_w = measure(fonts, ch.encode_utf8(&mut [0; 4]), size);
        if !line.is_empty() && *line_w + ch_w > max_w {
            lines.push(std::mem::take(line));
            *line_w = 0.0;
        }
        line.push(ch);
        *line_w += ch_w;
    }
}

pub(crate) fn ellipsize_heading(fonts: &Fonts, s: &str, size: f32, max_w: f32) -> String {
    if measure(fonts, s, size) <= max_w {
        return s.to_string();
    }
    let mut t = s.to_string();
    while t.len() > 2 && measure(fonts, &format!("{t}…"), size) > max_w {
        t.pop();
    }
    format!("{t}…")
}

pub(crate) fn fill_skew(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, skew: f32, c: Color) {
    let mut pb = PathBuilder::new();
    pb.move_to(x + skew, y);
    pb.line_to(x + w + skew, y);
    pb.line_to(x + w, y + h);
    pb.line_to(x, y + h);
    pb.close();
    if let Some(p) = pb.finish() {
        let mut paint = Paint::default();
        paint.set_color(c);
        px.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
    }
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
    fill_round(px, x, y, w, h, 14.0, Color::from_rgba8(48, 48, 52, 255));
    let knob_x = if on { x + w - 24.0 } else { x + 4.0 };
    fill_round(px, knob_x, y + 4.0, 20.0, 20.0, 10.0, accent());
    hits.push(HitBox {
        id: hit,
        x,
        y,
        w,
        h,
    });
    let _ = hover;
}

pub(crate) fn heading(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    title: &str,
    sub: &str,
    show: Option<(bool, Hit, &'static str)>,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let title_sz = 22.0;
    let h = 48.0;
    let skew = 8.0;
    let ink_c = ink();
    let mut right_w = 0.0;
    if show.is_some() {
        let label = show.map(|(_, _, label)| label).unwrap_or("");
        let label_w = measure(fonts, label, 13.0);
        right_w = label_w + 14.0 + 52.0 + 4.0;
    }
    let title_w = measure(fonts, title, title_sz);
    let max_plaque = (w - right_w - 12.0).max(72.0);
    let plaque_w = (title_w + 36.0).min(max_plaque);
    fill_skew(px, x, y, (plaque_w - skew).max(48.0), h, skew, accent());
    let label = ellipsize_heading(fonts, title, title_sz, plaque_w - 28.0);
    text(
        px,
        fonts,
        &label,
        title_sz,
        x + 16.0,
        y + 12.0,
        ink_c,
        false,
    );
    if let Some((on, hit, switch_label)) = show {
        let lx = x + w - right_w;
        text(
            px,
            fonts,
            switch_label,
            13.0,
            lx,
            y + 16.0,
            text_col(),
            false,
        );
        hits.push(HitBox {
            id: hit,
            x: lx,
            y,
            w: right_w,
            h,
        });
        switch_lg(
            px,
            lx + measure(fonts, switch_label, 13.0) + 12.0,
            y + (h - 28.0) * 0.5,
            on,
            hit,
            hover,
            hits,
        );
    }
    text(px, fonts, sub, 13.0, x, y + h + 8.0, muted(), false);
    y + h + 8.0 + 46.0
}

pub(crate) fn action_btn(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    label: &str,
    hit: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    primary: bool,
) -> f32 {
    let _ = hover;
    let bg = if primary {
        accent()
    } else {
        Color::from_rgba8(48, 48, 52, 255)
    };
    fill_round(px, x, y, w, h, 6.0, bg);
    let tw = measure(fonts, label, 14.0);
    text(
        px,
        fonts,
        label,
        14.0,
        x + (w - tw) * 0.5,
        y + (h - 16.0) * 0.5,
        if primary { ink() } else { text_col() },
        false,
    );
    hits.push(HitBox {
        id: hit,
        x,
        y,
        w,
        h,
    });
    y + h
}

pub(crate) fn knob() -> Color {
    Color::from_rgba8(240, 240, 242, 255)
}

pub(crate) fn dim() -> Color {
    Color::from_rgba8(120, 120, 128, 255)
}

pub(crate) fn btn_border() -> Color {
    Color::from_rgba8(255, 255, 255, 18)
}

#[derive(Clone)]
pub struct PendingDrop {
    pub mx: f32,
    pub my: f32,
    pub bw: f32,
    pub content_h: f32,
    pub open_hit: Hit,
    pub options: Vec<(Hit, String, bool)>,
}

thread_local! {
    pub(crate) static DROP_MENUS: RefCell<Vec<PendingDrop>> = RefCell::new(Vec::new());
}

pub fn take_pending_drops() -> Vec<PendingDrop> {
    DROP_MENUS.with(|m| std::mem::take(&mut *m.borrow_mut()))
}

pub(crate) fn section(px: &mut Pixmap, fonts: &Fonts, x: f32, y: f32, label: &str) -> f32 {
    text(
        px,
        fonts,
        &label.to_ascii_uppercase(),
        10.0,
        x + 2.0,
        y + 10.0,
        dim(),
        false,
    );
    y + 28.0
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

pub(crate) fn icon_stroke(px: &mut Pixmap, path: &Path, c: Color, width: f32) {
    let mut p = Paint::default();
    p.set_color(c);
    p.anti_alias = true;
    px.stroke_path(
        path,
        &p,
        &Stroke {
            width,
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            ..Stroke::default()
        },
        Transform::identity(),
        None,
    );
}

pub(crate) fn icon_stroke_line(
    px: &mut Pixmap,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    c: Color,
    width: f32,
) {
    let mut pb = PathBuilder::new();
    pb.move_to(x0, y0);
    pb.line_to(x1, y1);
    if let Some(path) = pb.finish() {
        icon_stroke(px, &path, c, width);
    }
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

/// F8-style mode tabs for the web pit box.
pub fn paint_mode_bar(
    px: &mut Pixmap,
    fonts: &Fonts,
    w: f32,
    widgets_on: bool,
    motos_on: bool,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) {
    let h = 52.0;
    fill_rect(px, Rect::from_xywh(0.0, 0.0, w, h).unwrap(), side());
    let tab_w = 108.0;
    let y = 8.0;
    for (i, (label, on, hit)) in [
        ("Widgets", widgets_on, Hit::TabWidgets),
        ("Motos", motos_on, Hit::TabMotos),
    ]
    .iter()
    .enumerate()
    {
        let x = 16.0 + i as f32 * (tab_w + 8.0);
        if *on {
            fill_round(
                px,
                x,
                y,
                tab_w,
                36.0,
                8.0,
                Color::from_rgba8(255, 255, 255, 12),
            );
            fill_round(px, x, y + 8.0, 3.0, 20.0, 1.5, accent());
        } else if hover == Some(*hit) {
            fill_round(
                px,
                x,
                y,
                tab_w,
                36.0,
                8.0,
                Color::from_rgba8(255, 255, 255, 8),
            );
        }
        let tw = measure(fonts, label, 14.0);
        text(
            px,
            fonts,
            label,
            14.0,
            x + (tab_w - tw) * 0.5,
            y + 10.0,
            if *on { accent() } else { text_col() },
            *on,
        );
        hits.push(HitBox {
            id: *hit,
            x,
            y,
            w: tab_w,
            h: 36.0,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn take_pending_drops_drains_queue() {
        DROP_MENUS.with(|m| {
            m.borrow_mut().push(PendingDrop {
                mx: 1.0,
                my: 2.0,
                bw: 3.0,
                content_h: 4.0,
                open_hit: Hit::AnalyzeLapOpen,
                options: vec![(Hit::AnalyzeYouLap(1), "L1".into(), true)],
            });
        });
        let got = take_pending_drops();
        assert_eq!(got.len(), 1);
        assert!(matches!(got[0].open_hit, Hit::AnalyzeLapOpen));
        assert!(take_pending_drops().is_empty());
    }
}
