#![allow(unused_imports)]
use super::*;

pub(crate) fn draw_flag(px: &mut Pixmap, fonts: &Fonts, cfg: &HudConfig, sw: f32, sh: f32, flag: DashFlag, grow: f32) {
    if flag == DashFlag::None || grow <= 0.02 {
        return;
    }
    let r = cfg[WidgetId::Flag].rect;
    let x = r.x * sw;
    let y = r.y * sh;
    let w = r.w * sw;
    let h = r.h * sh;
    if w < 48.0 || h < 10.0 {
        return;
    }
    let a = ((bg_a(cfg[WidgetId::Flag].bg) as f32) * grow.clamp(0.0, 1.0)).round() as u8;
    if a == 0 {
        return;
    }
    let skew = (h * 0.32).clamp(6.0, 12.0);
    let inner_w = (w - skew).max(48.0);
    let mut pb = PathBuilder::new();
    pb.move_to(x + skew, y);
    pb.line_to(x + inner_w + skew, y);
    pb.line_to(x + inner_w, y + h);
    pb.line_to(x, y + h);
    pb.close();
    let Some(path) = pb.finish() else {
        return;
    };
    paint_flag_fill(px, &path, x, y, inner_w + skew, h, flag, a);
    if !cfg.flag_text {
        return;
    }
    let cx = x + skew * 0.35;
    let label = flag_label(flag);
    let pad = (inner_w * 0.10).max(h * 1.2).clamp(18.0, 36.0);
    let white_w = flag_caption_group_w(fonts, h, 1.0, label) + pad * 2.0;
    let inset = (h * 0.20).clamp(3.0, 5.0);
    let band = flag_caption_band_path(x, y, inner_w, h, skew, inset);
    draw_flag_caption_fade(px, band.as_ref().unwrap_or(&path), cx, y, inner_w, white_w, a, None);
    let ink = Color::from_rgba8(16, 16, 18, a.max(220));
    draw_flag_caption(px, fonts, cx, y, inner_w, h, 1.0, label, ink);
}

/// Caption white sits in a shorter band so cloth shows above and below.
pub(crate) fn flag_caption_band_path(x: f32, y: f32, inner_w: f32, h: f32, skew: f32, inset: f32) -> Option<Path> {
    let inset = inset.min((h - 4.0) * 0.5).max(1.0);
    let t0 = (inset / h).clamp(0.0, 0.4);
    let t1 = 1.0 - t0;
    let left = |t: f32| x + skew * (1.0 - t);
    let right = |t: f32| x + inner_w + skew * (1.0 - t);
    let mut pb = PathBuilder::new();
    pb.move_to(left(t0), y + inset);
    pb.line_to(right(t0), y + inset);
    pb.line_to(right(t1), y + h - inset);
    pb.line_to(left(t1), y + h - inset);
    pb.close();
    pb.finish()
}

pub(crate) fn flag_label(flag: DashFlag) -> &'static str {
    match flag {
        DashFlag::White => "WHITE FLAG",
        DashFlag::Yellow => "YELLOW FLAG",
        DashFlag::Blue => "BLUE FLAG",
        DashFlag::Red => "RED FLAG",
        _ => "Checkered Flag",
    }
}

pub(crate) fn flag_yellow_cloth(a: u8) -> Color {
    Color::from_rgba8(204, 176, 70, a)
}

pub(crate) fn flag_blue_cloth(a: u8) -> Color {
    Color::from_rgba8(82, 118, 172, a)
}

pub(crate) fn flag_red_cloth(a: u8) -> Color {
    Color::from_rgba8(186, 82, 82, a)
}

pub(crate) fn paint_flag_fill(px: &mut Pixmap, path: &Path, x: f32, y: f32, w: f32, h: f32, flag: DashFlag, a: u8) {
    let clip = Mask::new(px.width(), px.height()).map(|mut m| {
        m.fill_path(path, FillRule::Winding, true, Transform::identity());
        m
    });
    match flag {
        DashFlag::White => {
            fill_path(px, path, Color::from_rgba8(248, 248, 250, a));
            draw_diag_stripes_masked(
                px,
                x,
                y,
                w,
                h,
                Color::from_rgba8(210, 210, 214, a),
                clip.as_ref(),
            );
        }
        DashFlag::Yellow => fill_path(px, path, flag_yellow_cloth(a)),
        DashFlag::Blue => fill_path(px, path, flag_blue_cloth(a)),
        DashFlag::Red => fill_path(px, path, flag_red_cloth(a)),
        _ => {
            fill_path(px, path, Color::from_rgba8(236, 236, 240, a));
            let rows = ((h / 16.0).round() as i32).clamp(3, 8);
            draw_checkered_cells(
                px,
                x,
                y,
                w,
                h,
                Color::from_rgba8(176, 176, 182, a),
                Color::from_rgba8(236, 236, 240, a),
                rows,
                clip.as_ref(),
            );
        }
    }
    stroke_path(
        px,
        path,
        Color::from_rgba8(16, 16, 18, ((a as u16 * 90) / 255) as u8),
        1.2,
    );
}

/// White behind the caption, same 5% edge fade as the Dash wrap banners.
pub(crate) fn draw_flag_caption_fade(
    px: &mut Pixmap,
    path: &Path,
    ox: f32,
    top_y: f32,
    ow: f32,
    white_w: f32,
    a: u8,
    clip: Option<&Mask>,
) {
    if ow <= 1.0 {
        return;
    }
    let fade = 0.05;
    let t0 = ((ow - white_w) * 0.5 / ow).clamp(fade + 0.02, 0.46);
    let on = Color::from_rgba8(248, 248, 250, a);
    let off = Color::from_rgba8(248, 248, 250, 0);
    let Some(shader) = LinearGradient::new(
        SkPoint::from_xy(ox, top_y),
        SkPoint::from_xy(ox + ow, top_y),
        vec![
            GradientStop::new(0.0, off),
            GradientStop::new((t0 - fade).max(0.0), off),
            GradientStop::new(t0, on),
            GradientStop::new(1.0 - t0, on),
            GradientStop::new((1.0 - t0 + fade).min(1.0), off),
            GradientStop::new(1.0, off),
        ],
        SpreadMode::Pad,
        Transform::identity(),
    ) else {
        return;
    };
    let mut paint = Paint::default();
    paint.shader = shader;
    paint.anti_alias = true;
    px.fill_path(path, &paint, FillRule::Winding, Transform::identity(), clip);
}

pub(crate) fn flag_icon_w(fonts: &Fonts, size: f32) -> f32 {
    fonts.icons.metrics('\u{f024}', size * style_k()).advance_width
}

/// Icon + label, centered. Returns the caption width so the checker fade can hug it.
pub(crate) fn draw_flag_caption(
    px: &mut Pixmap,
    fonts: &Fonts,
    ox: f32,
    top_y: f32,
    ow: f32,
    top_h: f32,
    grow: f32,
    label: &str,
    ink: Color,
) -> f32 {
    let sz = (top_h * 0.44).clamp(10.0, 13.0) * (0.75 + 0.25 * grow);
    let icon_sz = (sz * 1.12).clamp(11.0, 16.0);
    let gap = (icon_sz * 0.28).clamp(4.0, 7.0);
    let iw = flag_icon_w(fonts, icon_sz);
    let tw = measure_bold(fonts, label, sz);
    let group = iw + gap + tw;
    let x0 = ox + (ow - group) * 0.5;
    let ty = top_y + (top_h - sz) * 0.42;
    icon(
        px,
        fonts,
        '\u{f024}',
        icon_sz,
        x0,
        ty + (sz - icon_sz) * 0.5,
        ink,
        false,
    );
    text_bold(px, fonts, label, sz, x0 + iw + gap, ty, ink, false);
    group
}

pub(crate) fn flag_caption_group_w(fonts: &Fonts, top_h: f32, grow: f32, label: &str) -> f32 {
    let sz = (top_h * 0.44).clamp(10.0, 13.0) * (0.75 + 0.25 * grow);
    let icon_sz = (sz * 1.12).clamp(11.0, 16.0);
    let gap = (icon_sz * 0.28).clamp(4.0, 7.0);
    flag_icon_w(fonts, icon_sz) + gap + measure_bold(fonts, label, sz)
}
