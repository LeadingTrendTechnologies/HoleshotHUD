//! Full-screen opening (`splash.tga`) and loading (`bkgrnd.tga`) images.

use std::fs;
use std::path::Path;

use tiny_skia::{FillRule, FilterQuality, Paint, PathBuilder, Pixmap, PixmapPaint, Transform};

use super::sprites::{decode_tga_bgra, write_tga};

const SPLASH_PNG: &[u8] = include_bytes!("assets/splash_default.png");
const LOADING_PNG: &[u8] = include_bytes!("assets/loading_default.png");
/// Official PiBoSo wordmark (from piboso.com), not the MX Bikes `logo_ui.tga` mark.
const PIBOSO_LOGO_PNG: &[u8] = include_bytes!("assets/piboso_logo.png");

/// Floor for written TGA size — stock bak is often 1080p; ship 4K so common
/// high-res MX Bikes displays aren't nearest-neighbor stretched (looks pixelated).
#[cfg(not(test))]
const DEFAULT_W: u32 = 3840;
#[cfg(not(test))]
const DEFAULT_H: u32 = 2160;
/// Tests keep a 1080p floor so apply() stays fast (4K TGA ≈ 33MB each).
#[cfg(test)]
const DEFAULT_W: u32 = 1920;
#[cfg(test)]
const DEFAULT_H: u32 = 1080;

pub(crate) const SPLASH_TGA: &str = "splash.tga";
pub(crate) const LOADING_TGA: &str = "bkgrnd.tga";

/// Write opening + loading TGAs from optional user PNG paths (empty = bundled default).
pub(crate) fn write_screen_images(
    ui: &Path,
    bak: Option<&Path>,
    splash_path: &str,
    loading_path: &str,
) -> Result<(), String> {
    write_one(ui, bak, SPLASH_TGA, splash_path, SPLASH_PNG, true)?;
    write_one(ui, bak, LOADING_TGA, loading_path, LOADING_PNG, false)?;
    Ok(())
}

fn write_one(
    ui: &Path,
    bak: Option<&Path>,
    dest_name: &str,
    custom_path: &str,
    default_png: &[u8],
    with_piboso_logo: bool,
) -> Result<(), String> {
    let (dw, dh) = target_size(ui, bak, dest_name);
    let src = load_source(custom_path, default_png)?;
    let mut out = cover_scale(&src, dw, dh)?;
    soft_edge_fade(&mut out);
    if with_piboso_logo {
        // Bake into the TGA — connection wait blits splash.tga without .mnu logo items.
        composite_piboso_logo(&mut out);
    }
    write_tga(&ui.join(dest_name), &out)
}

fn target_size(ui: &Path, bak: Option<&Path>, name: &str) -> (u32, u32) {
    for base in [Some(ui), bak].into_iter().flatten() {
        if let Some((w, h)) = tga_dims(&base.join(name)) {
            // Keep stock aspect only when it is already at least 4K-class.
            if w >= DEFAULT_W && h >= DEFAULT_H {
                return (w, h);
            }
        }
    }
    (DEFAULT_W, DEFAULT_H)
}

fn tga_dims(path: &Path) -> Option<(u32, u32)> {
    let data = fs::read(path).ok()?;
    if data.len() < 18 {
        return None;
    }
    let w = u16::from_le_bytes([data[12], data[13]]) as u32;
    let h = u16::from_le_bytes([data[14], data[15]]) as u32;
    if w > 0 && h > 0 && w <= 8192 && h <= 8192 {
        return Some((w, h));
    }
    let px = decode_tga_bgra(&data).ok().flatten()?;
    Some((px.width().max(1), px.height().max(1)))
}

fn load_source(custom_path: &str, default_png: &[u8]) -> Result<Pixmap, String> {
    let path = custom_path.trim();
    if !path.is_empty() {
        if let Ok(bytes) = fs::read(path) {
            if let Some(px) = decode_image_bytes(&bytes) {
                return Ok(px);
            }
        }
    }
    Pixmap::decode_png(default_png).map_err(|e| e.to_string())
}

/// Decode PNG or JPEG bytes into a pixmap (custom Browse paths).
fn decode_image_bytes(bytes: &[u8]) -> Option<Pixmap> {
    if let Ok(px) = Pixmap::decode_png(bytes) {
        return Some(px);
    }
    let img = image::load_from_memory(bytes).ok()?.into_rgba8();
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 || w > 8192 || h > 8192 {
        return None;
    }
    let mut px = Pixmap::new(w, h)?;
    px.data_mut().copy_from_slice(img.as_raw());
    Some(px)
}

/// Center the bundled PiBoSo company wordmark on a soft night-ink plaque.
fn composite_piboso_logo(dest: &mut Pixmap) {
    let Ok(logo) = Pixmap::decode_png(PIBOSO_LOGO_PNG) else {
        return;
    };
    if logo.width() == 0 || logo.height() == 0 {
        return;
    }
    let dw = dest.width() as f32;
    let dh = dest.height() as f32;
    let target_w = dw * 0.36;
    let scale = target_w / logo.width().max(1) as f32;
    let lw = logo.width() as f32 * scale;
    let lh = logo.height() as f32 * scale;
    let lx = (dw - lw) * 0.5;
    let ly = (dh - lh) * 0.5;

    // Soft plaque behind the mark so it reads on sunset / dust highlights.
    let pad_x = lw * 0.12;
    let pad_y = lh * 0.35;
    let mut paint = Paint::default();
    paint.set_color_rgba8(0, 0, 0, 235);
    paint.anti_alias = true;
    let mut pb = PathBuilder::new();
    let rx = lx - pad_x;
    let ry = ly - pad_y;
    let rw = lw + pad_x * 2.0;
    let rh = lh + pad_y * 2.0;
    let rad = rh * 0.35;
    rounded_rect_path(&mut pb, rx, ry, rw, rh, rad);
    if let Some(path) = pb.finish() {
        dest.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }

    let t = Transform::from_translate(lx, ly).pre_scale(scale, scale);
    let mut logo_paint = PixmapPaint::default();
    logo_paint.quality = FilterQuality::Bicubic;
    dest.draw_pixmap(0, 0, logo.as_ref(), &logo_paint, t, None);
}

fn rounded_rect_path(pb: &mut PathBuilder, x: f32, y: f32, w: f32, h: f32, r: f32) {
    let r = r.min(w * 0.5).min(h * 0.5).max(0.0);
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
}

/// Cover-scale `src` into `dw`×`dh` (center crop) with bicubic filtering.
fn cover_scale(src: &Pixmap, dw: u32, dh: u32) -> Result<Pixmap, String> {
    let mut out = Pixmap::new(dw, dh).ok_or("pixmap")?;
    let sw = src.width() as f32;
    let sh = src.height() as f32;
    if sw < 1.0 || sh < 1.0 {
        return Err("empty image".into());
    }
    let scale = (dw as f32 / sw).max(dh as f32 / sh);
    let ox = (dw as f32 - sw * scale) * 0.5;
    let oy = (dh as f32 - sh * scale) * 0.5;
    let t = Transform::from_translate(ox, oy).pre_scale(scale, scale);
    let mut paint = PixmapPaint::default();
    paint.quality = FilterQuality::Bicubic;
    out.draw_pixmap(0, 0, src.as_ref(), &paint, t, None);
    Ok(out)
}

/// Soft rounded-rect falloff into black — reads as a rounded plate, not a square crop.
fn soft_edge_fade(px: &mut Pixmap) {
    let w = px.width();
    let h = px.height();
    if w < 8 || h < 8 {
        return;
    }
    let wf = w as f32;
    let hf = h as f32;
    let short = wf.min(hf);
    // Pull in from the frame so the plate is clearly rounded, not full-bleed square.
    let inset_x = wf * 0.04;
    let inset_y = hf * 0.05;
    // Large corner radius + soft blur band.
    let radius = (short * 0.10).max(64.0);
    let band = (short * 0.08).max(36.0);
    let ix0 = inset_x;
    let iy0 = inset_y;
    let ix1 = wf - inset_x;
    let iy1 = hf - inset_y;
    let rw = (ix1 - ix0).max(1.0);
    let rh = (iy1 - iy0).max(1.0);
    let r = radius.min(rw * 0.5).min(rh * 0.5);
    let cx = (ix0 + ix1) * 0.5;
    let cy = (iy0 + iy1) * 0.5;
    let hw = (rw * 0.5 - r).max(0.0);
    let hh = (rh * 0.5 - r).max(0.0);

    let data = px.data_mut();
    for y in 0..h {
        let py = y as f32 + 0.5;
        for x in 0..w {
            let px_ = x as f32 + 0.5;
            // SDF of rounded rect: negative inside, positive outside.
            let qx = (px_ - cx).abs() - hw;
            let qy = (py - cy).abs() - hh;
            let sdf = {
                let ox = qx.max(0.0);
                let oy = qy.max(0.0);
                (ox * ox + oy * oy).sqrt() + qx.min(0.0).max(qy.min(0.0)) - r
            };
            // 1 inside → 0 through the soft outer band.
            let t = smoothstep(1.0 - sdf / band);
            if t >= 0.999 {
                continue;
            }
            let i = ((y * w + x) * 4) as usize;
            data[i] = (data[i] as f32 * t).round() as u8;
            data[i + 1] = (data[i + 1] as f32 * t).round() as u8;
            data[i + 2] = (data[i + 2] as f32 * t).round() as u8;
        }
    }
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
