use std::cell::Cell;

use fontdue::Font;
use tiny_skia::{Color, Pixmap};

use crate::config::FontFamily;

thread_local! {
    static FACE: Cell<*const Font> = Cell::new(std::ptr::null());
    static SCALE: Cell<f32> = Cell::new(1.0);
    static FAKE_BOLD: Cell<bool> = Cell::new(false);
}

pub struct Fonts {
    pub ui: Font,
    pub bold: Font,
    /// App family at semibold when that file exists. Otherwise the UI face.
    pub semibold: Font,
    pub icons: Font,
    bold_is_fake: bool,
}
pub(crate) struct StyleGuard;

impl Drop for StyleGuard {
    fn drop(&mut self) {
        FACE.with(|c| c.set(std::ptr::null()));
        SCALE.with(|c| c.set(1.0));
        FAKE_BOLD.with(|c| c.set(false));
    }
}

pub(crate) fn push_style(fonts: &Fonts, bold: bool, pct: i32) -> StyleGuard {
    let face = if bold { &fonts.bold } else { &fonts.ui };
    FACE.with(|c| c.set(face as *const Font));
    SCALE.with(|c| c.set((pct.clamp(70, 160) as f32) / 100.0));
    FAKE_BOLD.with(|c| c.set(bold && fonts.bold_is_fake));
    StyleGuard
}

pub(crate) fn style_k() -> f32 {
    SCALE.with(|c| c.get())
}

pub(crate) fn style_font(fonts: &Fonts) -> &Font {
    let ptr = FACE.with(|c| c.get());
    if ptr.is_null() {
        &fonts.ui
    } else {
        unsafe { &*ptr }
    }
}

pub(crate) fn font_from(bytes: &[u8]) -> Option<Font> {
    Font::from_bytes(bytes, fontdue::FontSettings::default()).ok()
}

impl Fonts {
    pub fn load() -> Option<Self> {
        Self::for_family(FontFamily::Exo2)
    }

    pub fn for_family(family: FontFamily) -> Option<Self> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(loaded) = Self::from_windows(family) {
                return Some(loaded);
            }
        }
        if let Some(loaded) = Self::from_bundled(family) {
            return Some(loaded);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            for fallback in [FontFamily::Segoe, FontFamily::Arial, FontFamily::Tahoma] {
                if fallback == family {
                    continue;
                }
                if let Some(loaded) = Self::from_windows(fallback) {
                    return Some(loaded);
                }
            }
        }
        Self::from_bundled(FontFamily::Roboto)
    }

    fn icons() -> Option<Font> {
        font_from(include_bytes!("../../fonts/fa-solid-900.ttf").as_slice())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn from_windows(family: FontFamily) -> Option<Self> {
        let (reg, bld) = family.windows_files()?;
        let bytes = std::fs::read(reg).ok()?;
        let ui = font_from(&bytes)?;
        let (bold, bold_is_fake) = match std::fs::read(bld).ok().and_then(|b| font_from(&b)) {
            Some(bold) => (bold, false),
            None => (font_from(&bytes)?, true),
        };
        let semibold = if family == FontFamily::Segoe {
            std::fs::read(r"C:\Windows\Fonts\segoeuisb.ttf")
                .ok()
                .and_then(|b| font_from(&b))
        } else {
            None
        };
        let semibold = match semibold {
            Some(face) => face,
            None => font_from(&bytes)?,
        };
        Some(Self {
            ui,
            bold,
            semibold,
            icons: Self::icons()?,
            bold_is_fake,
        })
    }

    fn from_bundled(family: FontFamily) -> Option<Self> {
        let icons = Self::icons()?;
        match family {
            FontFamily::Roboto => Self::embedded(
                include_bytes!("../../fonts/Roboto-Regular.ttf"),
                None,
                None,
                icons,
            ),
            FontFamily::Exo2 => Self::embedded(
                include_bytes!("../../fonts/Exo2-ExtraBoldItalic.ttf"),
                Some(include_bytes!("../../fonts/Exo2-BlackItalic.ttf")),
                Some(include_bytes!("../../fonts/Exo2-SemiBoldItalic.ttf")),
                icons,
            ),
            FontFamily::Teko => Self::embedded(
                include_bytes!("../../fonts/Teko-SemiBold.ttf"),
                Some(include_bytes!("../../fonts/Teko-Bold.ttf")),
                None,
                icons,
            ),
            FontFamily::Goldman => Self::embedded(
                include_bytes!("../../fonts/Goldman-Regular.ttf"),
                Some(include_bytes!("../../fonts/Goldman-Bold.ttf")),
                None,
                icons,
            ),
            FontFamily::Montserrat => Self::embedded(
                include_bytes!("../../fonts/Montserrat-ExtraBold.ttf"),
                Some(include_bytes!("../../fonts/Montserrat-Black.ttf")),
                None,
                icons,
            ),
            _ => None,
        }
    }

    fn embedded(
        regular: &[u8],
        bold: Option<&[u8]>,
        semibold: Option<&[u8]>,
        icons: Font,
    ) -> Option<Self> {
        let ui = font_from(regular)?;
        let (bold, bold_is_fake) = match bold.and_then(|b| font_from(b)) {
            Some(bold) => (bold, false),
            None => (font_from(regular)?, true),
        };
        let semibold = match semibold.and_then(font_from) {
            Some(face) => face,
            None => font_from(regular)?,
        };
        Some(Self {
            ui,
            bold,
            semibold,
            icons,
            bold_is_fake,
        })
    }
}
pub fn text(
    px: &mut Pixmap,
    fonts: &Fonts,
    s: &str,
    size: f32,
    x: f32,
    y: f32,
    color: Color,
    center: bool,
) {
    draw_text(
        px,
        style_font(fonts),
        s,
        size,
        x,
        y,
        color,
        center,
        FAKE_BOLD.with(|c| c.get()),
    );
}

/// Round night-ink rim. Copies sit on two circles so glyph anti-aliasing blends the edge.
pub(crate) fn text_halo(
    px: &mut Pixmap,
    fonts: &Fonts,
    s: &str,
    size: f32,
    x: f32,
    y: f32,
    color: Color,
    center: bool,
    glass: bool,
) {
    if glass {
        for (radius, alpha, steps) in [(1.0, 210u8, 12u32), (1.85, 90, 16)] {
            let ink = Color::from_rgba8(10, 10, 10, alpha);
            for step in 0..steps {
                let angle = (step as f32) * std::f32::consts::TAU / steps as f32;
                text(
                    px,
                    fonts,
                    s,
                    size,
                    x + angle.cos() * radius,
                    y + angle.sin() * radius,
                    ink,
                    center,
                );
            }
        }
    }
    text(px, fonts, s, size, x, y, color, center);
}

pub fn text_bold(
    px: &mut Pixmap,
    fonts: &Fonts,
    s: &str,
    size: f32,
    x: f32,
    y: f32,
    color: Color,
    center: bool,
) {
    draw_text(
        px,
        &fonts.bold,
        s,
        size,
        x,
        y,
        color,
        center,
        fonts.bold_is_fake,
    );
}

pub(crate) fn draw_text(
    px: &mut Pixmap,
    font: &Font,
    s: &str,
    size: f32,
    mut x: f32,
    y: f32,
    color: Color,
    center: bool,
    fake: bool,
) {
    let size = size * style_k();
    if center {
        x -= measure_font(font, s, size) * 0.5;
    }
    let rgba = [
        (color.red() * 255.0) as u8,
        (color.green() * 255.0) as u8,
        (color.blue() * 255.0) as u8,
        (color.alpha() * 255.0) as u8,
    ];
    let mut pen = x;
    for ch in s.chars() {
        if ch != ' ' && ch != '\t' && !font.has_glyph(ch) {
            continue;
        }
        let (metrics, bitmap) = font.rasterize(ch, size);
        let gx = pen + metrics.xmin as f32;
        let gy = y + size - metrics.ymin as f32 - metrics.height as f32;
        blit(px, &bitmap, metrics.width, metrics.height, gx, gy, rgba);
        if fake {
            blit(
                px,
                &bitmap,
                metrics.width,
                metrics.height,
                gx + 0.7,
                gy,
                rgba,
            );
        }
        pen += metrics.advance_width;
    }
}

pub fn measure(fonts: &Fonts, s: &str, size: f32) -> f32 {
    let extra = if FAKE_BOLD.with(|c| c.get()) {
        0.7
    } else {
        0.0
    };
    measure_font(style_font(fonts), s, size * style_k()) + extra
}

pub(crate) fn measure_bold(fonts: &Fonts, s: &str, size: f32) -> f32 {
    let extra = if fonts.bold_is_fake { 0.7 } else { 0.0 };
    measure_font(&fonts.bold, s, size * style_k()) + extra
}

pub(crate) fn measure_font(font: &Font, s: &str, size: f32) -> f32 {
    s.chars()
        .filter(|ch| *ch == ' ' || *ch == '\t' || font.has_glyph(*ch))
        .map(|ch| font.metrics(ch, size).advance_width)
        .sum()
}

pub(crate) fn blit(px: &mut Pixmap, bitmap: &[u8], gw: usize, gh: usize, x: f32, y: f32, rgba: [u8; 4]) {
    let pw = px.width() as i32;
    let ph = px.height() as i32;
    let data = px.data_mut();
    for row in 0..gh {
        for col in 0..gw {
            let cov = bitmap[row * gw + col];
            if cov < 8 {
                continue;
            }
            let dx = x as i32 + col as i32;
            let dy = y as i32 + row as i32;
            if dx < 0 || dy < 0 || dx >= pw || dy >= ph {
                continue;
            }
            let i = ((dy * pw + dx) * 4) as usize;
            let a = (rgba[3] as u16 * cov as u16) / 255;
            let ia = 255 - a;
            data[i] = ((data[i] as u16 * ia + rgba[0] as u16 * a) / 255) as u8;
            data[i + 1] = ((data[i + 1] as u16 * ia + rgba[1] as u16 * a) / 255) as u8;
            data[i + 2] = ((data[i + 2] as u16 * ia + rgba[2] as u16 * a) / 255) as u8;
            data[i + 3] = data[i + 3].saturating_add(a as u8);
        }
    }
}
pub(crate) fn ellipsize(fonts: &Fonts, s: &str, size: f32, max_w: f32) -> String {
    if measure(fonts, s, size) <= max_w {
        return s.to_string();
    }
    let mut out = String::new();
    for ch in s.chars() {
        let next = format!("{out}{ch}…");
        if measure(fonts, &next, size) > max_w {
            if out.is_empty() {
                return "…".into();
            }
            out.push('…');
            return out;
        }
        out.push(ch);
    }
    out
}
pub(crate) fn max_digit_w(fonts: &Fonts, size: f32) -> f32 {
    ('0'..='9')
        .map(|d| measure(fonts, d.encode_utf8(&mut [0; 4]), size))
        .fold(0.0, f32::max)
}
pub(crate) fn format_clock(ms: i32) -> String {
    if ms <= 0 {
        return "--:--.---".into();
    }
    let t = ms as f32 / 1000.0;
    let m = (t / 60.0) as i32;
    let s = t - m as f32 * 60.0;
    format!("{m:02}:{:06.3}", s)
}
pub(crate) fn ink_bounds(font: &Font, s: &str, size: f32) -> Option<(f32, f32, f32, f32)> {
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    let mut pen = 0.0;
    let mut any = false;
    let size = size * style_k();
    for ch in s.chars() {
        let m = font.metrics(ch, size);
        if m.width > 0 && m.height > 0 {
            any = true;
            let gx0 = pen + m.xmin as f32;
            let gy0 = size - m.ymin as f32 - m.height as f32;
            min_x = min_x.min(gx0);
            min_y = min_y.min(gy0);
            max_x = max_x.max(gx0 + m.width as f32);
            max_y = max_y.max(gy0 + m.height as f32);
        }
        pen += m.advance_width;
    }
    any.then_some((min_x, min_y, max_x, max_y))
}

pub fn icon(
    px: &mut Pixmap,
    fonts: &Fonts,
    ch: char,
    size: f32,
    mut x: f32,
    y: f32,
    color: Color,
    center: bool,
) {
    let size = size * style_k();
    if center {
        x -= fonts.icons.metrics(ch, size).advance_width * 0.5;
    }
    let rgba = [
        (color.red() * 255.0) as u8,
        (color.green() * 255.0) as u8,
        (color.blue() * 255.0) as u8,
        (color.alpha() * 255.0) as u8,
    ];
    let (metrics, bitmap) = fonts.icons.rasterize(ch, size);
    if metrics.width == 0 || metrics.height == 0 {
        return;
    }
    blit(
        px,
        &bitmap,
        metrics.width,
        metrics.height,
        x + metrics.xmin as f32,
        y + size - metrics.ymin as f32 - metrics.height as f32,
        rgba,
    );
}

pub(crate) fn icon_over_dot(px: &mut Pixmap, fonts: &Fonts, x: f32, y: f32, r: f32, ch: char, col: Color) {
    icon_over_dot_scaled(px, fonts, x, y, r, ch, col, 1.0);
}

pub(crate) fn icon_over_dot_scaled(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    r: f32,
    ch: char,
    col: Color,
    scale: f32,
) {
    let k = style_k().max(0.01);
    let vis = (r * 1.85 * scale).clamp(11.0 * k * scale, 20.0 * k * scale);
    let size = vis / k;
    let metrics = fonts.icons.metrics(ch, vis);
    let gap = (r * 0.22).max(2.5);
    let cy = y - r - gap - vis + metrics.ymin as f32;
    icon(
        px,
        fonts,
        ch,
        size,
        x + 0.8,
        cy + 0.8,
        Color::from_rgba8(8, 8, 10, 220),
        true,
    );
    icon(px, fonts, ch, size, x, cy, col, true);
}

pub(crate) fn crown_over_dot(px: &mut Pixmap, fonts: &Fonts, x: f32, y: f32, r: f32) {
    icon_over_dot(
        px,
        fonts,
        x,
        y,
        r,
        '\u{f521}',
        Color::from_rgba8(255, 196, 48, 255),
    );
}
