#![allow(unused_imports)]
use super::*;

pub(crate) static STANCE_SIT: AtomicI32 = AtomicI32::new(0);

pub fn set_stance(sitting: bool) {
    STANCE_SIT.store(i32::from(sitting), Ordering::Relaxed);
}

pub fn stance_sitting() -> bool {
    STANCE_SIT.load(Ordering::Relaxed) != 0
}

pub(crate) fn stance_glyph(sitting: bool) -> Option<&'static Pixmap> {
    static STAND: OnceLock<Option<Pixmap>> = OnceLock::new();
    static SIT: OnceLock<Option<Pixmap>> = OnceLock::new();
    let slot = if sitting { &SIT } else { &STAND };
    slot.get_or_init(|| {
        let bytes: &[u8] = if sitting {
            include_bytes!("../../assets/stance-sit.png")
        } else {
            include_bytes!("../../assets/stance-stand.png")
        };
        Pixmap::decode_png(bytes).ok()
    })
    .as_ref()
}

pub(crate) fn draw_stance_icon(px: &mut Pixmap, cfg: &HudConfig, x: f32, y: f32, w: f32, h: f32) -> bool {
    let sitting = stance_sitting();
    let Some(src) = stance_glyph(sitting) else {
        return false;
    };
    let a = bg_a(cfg[WidgetId::Stance].bg);
    if a > 0 {
        fill_round(px, x, y, w, h, 6.0, Color::from_rgba8(10, 10, 12, a));
    }
    let pad = (w.min(h) * 0.10).max(4.0);
    let iw = src.width() as f32;
    let ih = src.height() as f32;
    if iw < 1.0 || ih < 1.0 {
        return false;
    }
    let scale = ((w - pad * 2.0) / iw).min((h - pad * 2.0) / ih).max(0.01);
    let dw = iw * scale;
    let dh = ih * scale;
    let dx = x + (w - dw) * 0.5;
    let dy = y + (h - dh) * 0.5;
    let mut paint = PixmapPaint::default();
    paint.quality = FilterQuality::Bicubic;
    px.draw_pixmap(
        0,
        0,
        src.as_ref(),
        &paint,
        Transform::from_row(scale, 0.0, 0.0, scale, dx, dy),
        None,
    );
    true
}

pub(crate) fn draw_stance(px: &mut Pixmap, fonts: &Fonts, cfg: &HudConfig, sw: f32, sh: f32) {
    let r = cfg[WidgetId::Stance].rect;
    let x = r.x * sw;
    let y = r.y * sh;
    let w = r.w * sw;
    let h = r.h * sh;
    if w < 36.0 || h < 22.0 {
        return;
    }
    if stance_sitting() && !cfg.stance_show_sit {
        return;
    }
    if cfg.stance_style == StanceStyle::Icon && draw_stance_icon(px, cfg, x, y, w, h) {
        return;
    }
    let sitting = stance_sitting();
    let a = bg_a(cfg[WidgetId::Stance].bg);
    let stand = !sitting;
    let fill = if stand {
        Color::from_rgba8(255, 148, 48, a.max(220))
    } else {
        Color::from_rgba8(10, 10, 12, a)
    };
    let skew = (h * 0.12).clamp(3.0, 6.0);
    fill_skew(px, x, y, w, h, skew, fill);
    let word = if sitting { "SIT" } else { "STAND" };
    let ink = if stand {
        Color::from_rgba8(12, 12, 14, 255)
    } else {
        Color::from_rgba8(228, 228, 230, 255)
    };
    let size = (h * 0.42).clamp(12.0, 28.0);
    text(px, fonts, word, size, x + w * 0.5, y + (h - size) * 0.42, ink, true);
}
