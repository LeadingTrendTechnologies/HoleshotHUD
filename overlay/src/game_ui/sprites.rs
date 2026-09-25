use std::fs;
use std::path::Path;

use tiny_skia::{BlendMode, Color, FillRule, Paint, PathBuilder, Pixmap, Transform};

use crate::config::ink_on_rgb;
use super::shell::{CHARCOAL, FIELD, HAIR, NIGHT, TEXT};

pub(crate) const SPRITES: &[&str] = &[
    "mainbox.tga",
    "main1.tga",
    "main2.tga",
    "main3.tga",
    "mainexit2.tga",
    "mainexit3.tga",
    "button1.tga",
    "button2.tga",
    "button3.tga",
    "b_button2.tga",
    "b_button3.tga",
    "done1.tga",
    "done2.tga",
    "back1.tga",
    "back2.tga",
    "fieldbox.tga",
    "tabh1.tga",
    "tabh2.tga",
    "tabv1.tga",
    "tabv2.tga",
    "segtrack.tga",
    "segl1.tga",
    "segl2.tga",
    "segr1.tga",
    "segr2.tga",
    "segm1.tga",
    "segm2.tga",
    "vsegtrack.tga",
    "sevt1.tga",
    "sevt2.tga",
    "sevm1.tga",
    "sevm2.tga",
    "sevb1.tga",
    "sevb2.tga",
    "optionstabtrack.tga",
    "dialog600x200.tga",
    "dialog800x500.tga",
    "dialog440x170.tga",
    "dialog600x190.tga",
    "dialog940x700.tga",
    "dialog1580x730.tga",
    "dialog1580x700t.tga",
    "dialog940x670.tga",
    "dialog620x670.tga",
    "setupboard_l.tga",
    "setupboard_r.tga",
    "setupbar.tga",
    "trackinfoboard.tga",
    "bikeboard.tga",
    "biketrackboard.tga",
    "bikeinfoboard.tga",
    "profilesboard.tga",
    "profilemodifyboard.tga",
    "profiledeleteboard.tga",
    "raceinfoboard.tga",
    "testpanelboard.tga",
    "trackmapmask.tga",
    "replayboard.tga",
    "replaypanel.tga",
    "garageboard.tga",
    "garagepanel_t.tga",
    "garagepanel_r.tga",
    "dialog600x70.tga",
    "dialog600x130.tga",
    "dialog600x600.tga",
    "serverboard.tga",
    "optionsboard.tga",
    "check1.tga",
    "check2.tga",
];

/// Stock chrome recolored on apply (not pack-generated sprites).

/// Stock chrome recolored on apply (not pack-generated sprites).
pub(crate) const STOCK_CHROME: &[&str] = &[
    "pointer.tga",
    "arrow_lf1.tga",
    "arrow_lf2.tga",
    "arrow_lf3.tga",
    "arrow_rg1.tga",
    "arrow_rg2.tga",
    "arrow_rg3.tga",
];

/// Stock pointer is hard-blue. Retint to Look primary so chrome matches the menus.
pub(crate) fn write_accent_pointer(ui: &Path, bak: Option<&Path>, accent: [u8; 3]) -> Result<(), String> {
    let src = bak
        .map(|b| b.join("pointer.tga"))
        .filter(|p| p.is_file())
        .or_else(|| {
            let p = ui.join("pointer.tga");
            p.is_file().then_some(p)
        });
    let Some(src) = src else {
        return Ok(());
    };
    let raw = fs::read(&src).map_err(|e| e.to_string())?;
    let Some(mut px) = decode_tga_bgra(&raw)? else {
        // Unreadable — keep whatever ensure_stock copied.
        return Ok(());
    };
    tint_blue_to_accent(&mut px, accent);
    write_tga(&ui.join("pointer.tga"), &px)?;
    // Shadow stays stock if present.
    if let Some(bak) = bak {
        let shadow = bak.join("pointer_shadow.tga");
        if shadow.is_file() && !ui.join("pointer_shadow.tga").is_file() {
            fs::copy(&shadow, ui.join("pointer_shadow.tga")).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Stock spin chevrons are ~RGB 60 — invisible on glass. Keep alpha, paint TEXT / accent.

/// Stock spin chevrons are ~RGB 60 — invisible on glass. Keep alpha, paint TEXT / accent.
pub(crate) fn write_spin_arrows(ui: &Path, bak: Option<&Path>, accent: [u8; 3]) -> Result<(), String> {
    let bright = [255u8, 255, 255];
    let jobs = [
        ("arrow_lf1.tga", TEXT),
        ("arrow_lf2.tga", bright),
        ("arrow_lf3.tga", accent),
        ("arrow_rg1.tga", TEXT),
        ("arrow_rg2.tga", bright),
        ("arrow_rg3.tga", accent),
    ];
    for (name, rgb) in jobs {
        let src = bak
            .map(|b| b.join(name))
            .filter(|p| p.is_file())
            .or_else(|| {
                let p = ui.join(name);
                p.is_file().then_some(p)
            });
        let Some(src) = src else {
            continue;
        };
        let raw = fs::read(&src).map_err(|e| e.to_string())?;
        let Some(mut px) = decode_tga_bgra(&raw)? else {
            continue;
        };
        recolor_keep_alpha(&mut px, rgb);
        write_tga(&ui.join(name), &px)?;
    }
    Ok(())
}

pub(crate) fn recolor_keep_alpha(px: &mut Pixmap, rgb: [u8; 3]) {
    let d = px.data_mut();
    for i in (0..d.len()).step_by(4) {
        if d[i + 3] == 0 {
            continue;
        }
        d[i] = rgb[0];
        d[i + 1] = rgb[1];
        d[i + 2] = rgb[2];
    }
}

pub(crate) fn decode_tga_bgra(data: &[u8]) -> Result<Option<Pixmap>, String> {
    if data.len() < 18 {
        return Ok(None);
    }
    let id_len = data[0] as usize;
    let itype = data[2];
    let w = u16::from_le_bytes([data[12], data[13]]) as u32;
    let h = u16::from_le_bytes([data[14], data[15]]) as u32;
    let bpp = data[16];
    let desc = data[17];
    if w == 0 || h == 0 || bpp != 32 || (itype != 2 && itype != 10) {
        return Ok(None);
    }
    let mut off = 18 + id_len;
    let mut bgra = vec![0u8; (w * h * 4) as usize];
    if itype == 2 {
        let need = bgra.len();
        if data.len() < off + need {
            return Ok(None);
        }
        bgra.copy_from_slice(&data[off..off + need]);
    } else {
        let mut i = 0usize;
        let px = (w * h) as usize;
        while i < px {
            if off >= data.len() {
                return Ok(None);
            }
            let pkt = data[off];
            off += 1;
            let count = (pkt & 0x7f) as usize + 1;
            if pkt & 0x80 != 0 {
                if off + 4 > data.len() {
                    return Ok(None);
                }
                let pix = &data[off..off + 4];
                off += 4;
                for _ in 0..count {
                    if i >= px {
                        break;
                    }
                    let o = i * 4;
                    bgra[o..o + 4].copy_from_slice(pix);
                    i += 1;
                }
            } else {
                for _ in 0..count {
                    if i >= px || off + 4 > data.len() {
                        return Ok(None);
                    }
                    let o = i * 4;
                    bgra[o..o + 4].copy_from_slice(&data[off..off + 4]);
                    off += 4;
                    i += 1;
                }
            }
        }
    }
    let top = desc & 0x20 != 0;
    let mut px = Pixmap::new(w, h).ok_or("pixmap")?;
    let out = px.data_mut();
    for y in 0..h as usize {
        let src_y = if top { y } else { h as usize - 1 - y };
        for x in 0..w as usize {
            let si = (src_y * w as usize + x) * 4;
            let di = (y * w as usize + x) * 4;
            // TGA BGRA → RGBA
            out[di] = bgra[si + 2];
            out[di + 1] = bgra[si + 1];
            out[di + 2] = bgra[si];
            out[di + 3] = bgra[si + 3];
        }
    }
    Ok(Some(px))
}

pub(crate) fn tint_blue_to_accent(px: &mut Pixmap, accent: [u8; 3]) {
    let w = px.width() as usize;
    let h = px.height() as usize;
    let data = px.data_mut();
    for i in 0..w * h {
        let o = i * 4;
        let r = data[o];
        let g = data[o + 1];
        let b = data[o + 2];
        let a = data[o + 3];
        if a == 0 {
            continue;
        }
        // Stock pointer is blue chrome; retint those pixels, leave near-black alone.
        if b > r.saturating_add(20) && b > g.saturating_add(10) {
            let lum = (b as u16 * 2 + g as u16 + r as u16) / 4;
            let t = (lum as f32 / 255.0).clamp(0.15, 1.0);
            data[o] = (accent[0] as f32 * t).round().clamp(0.0, 255.0) as u8;
            data[o + 1] = (accent[1] as f32 * t).round().clamp(0.0, 255.0) as u8;
            data[o + 2] = (accent[2] as f32 * t).round().clamp(0.0, 255.0) as u8;
        }
    }
}



pub(crate) fn write_sprites(ui: &Path, accent: [u8; 3]) -> Result<(), String> {
    let wash = Color::from_rgba8(accent[0], accent[1], accent[2], 28);
    let pip = rgb(accent);
    let night = rgb(NIGHT);
    let charcoal = rgb(CHARCOAL);
    let hair = rgb(HAIR);
    let ink = rgb(ink_on_rgb(accent));

    // Idle dest: clear (text on glass). Hairline boxes read as hollow frames
    // once the card is translucent; hover/selected keep the filled F8 row.
    write_row(&ui.join("main1.tga"), None, None, None, None)?;
    write_card(&ui.join("mainbox.tga"))?;
    write_row(
        &ui.join("main2.tga"),
        Some(rgb([24, 25, 29])),
        Some(wash),
        Some(pip),
        None,
    )?;
    write_row(
        &ui.join("main3.tga"),
        Some(rgb([24, 25, 29])),
        Some(wash),
        Some(pip),
        None,
    )?;
    write_row(
        &ui.join("mainexit2.tga"),
        Some(night),
        Some(wash),
        None,
        Some(hair),
    )?;
    write_row(
        &ui.join("mainexit3.tga"),
        Some(night),
        Some(wash),
        None,
        Some(hair),
    )?;
    write_round(&ui.join("button1.tga"), 160, 36, charcoal, hair)?;
    write_round(&ui.join("button2.tga"), 160, 36, charcoal, pip)?;
    write_round(&ui.join("button3.tga"), 160, 36, charcoal, pip)?;
    write_round(&ui.join("b_button2.tga"), 160, 36, charcoal, pip)?;
    write_round(&ui.join("b_button3.tga"), 160, 36, charcoal, pip)?;
    write_skew(&ui.join("done1.tga"), 200, 40, pip, ink)?;
    write_skew(&ui.join("done2.tga"), 200, 40, pip, ink)?;
    write_round(&ui.join("back1.tga"), 160, 36, charcoal, hair)?;
    write_round(&ui.join("back2.tga"), 160, 36, charcoal, pip)?;
    write_round(&ui.join("fieldbox.tga"), 200, 36, rgb(FIELD), hair)?;
    write_tab(&ui.join("tabh1.tga"), Some(charcoal), None, None, Some(hair))?;
    write_tab(
        &ui.join("tabh2.tga"),
        Some(rgb([24, 25, 29])),
        Some(wash),
        Some(pip),
        Some(pip),
    )?;
    write_round(&ui.join("tabv1.tga"), 200, 36, charcoal, hair)?;
    write_round(&ui.join("tabv2.tga"), 200, 36, charcoal, pip)?;
    write_seg_track(&ui.join("segtrack.tga"), charcoal, hair)?;
    write_round(&ui.join("optionstabtrack.tga"), 900, 36, charcoal, hair)?;
    write_seg(
        &ui.join("segl1.tga"),
        SegCap::Left,
        None,
        None,
    )?;
    write_seg(
        &ui.join("segl2.tga"),
        SegCap::Left,
        Some(wash),
        Some(pip),
    )?;
    write_seg(
        &ui.join("segr1.tga"),
        SegCap::Right,
        None,
        None,
    )?;
    write_seg(
        &ui.join("segr2.tga"),
        SegCap::Right,
        Some(wash),
        Some(pip),
    )?;
    write_seg(
        &ui.join("segm1.tga"),
        SegCap::Mid,
        None,
        None,
    )?;
    write_seg(
        &ui.join("segm2.tga"),
        SegCap::Mid,
        Some(wash),
        Some(pip),
    )?;
    // Vertical side-tab group (Event Info / Results / Track Info).
    // Idle clear like horizontal segs — vsegtrack is the continuous rail.
    write_round(&ui.join("vsegtrack.tga"), 110, 220, charcoal, hair)?;
    write_seg(&ui.join("sevt1.tga"), SegCap::Top, None, None)?;
    write_seg(&ui.join("sevt2.tga"), SegCap::Top, Some(wash), Some(pip))?;
    write_seg(&ui.join("sevm1.tga"), SegCap::Mid, None, None)?;
    write_seg(&ui.join("sevm2.tga"), SegCap::Mid, Some(wash), Some(pip))?;
    write_seg(&ui.join("sevb1.tga"), SegCap::Bottom, None, None)?;
    write_seg(&ui.join("sevb2.tga"), SegCap::Bottom, Some(wash), Some(pip))?;

    for (name, w, h) in [
        ("dialog600x200.tga", 600, 200),
        ("dialog800x500.tga", 800, 500),
        ("dialog440x170.tga", 440, 170),
        ("dialog600x190.tga", 600, 190),
        ("dialog940x700.tga", 940, 700),
        ("dialog940x670.tga", 940, 670),
        ("dialog620x670.tga", 620, 670),
        ("dialog600x70.tga", 600, 70),
        ("dialog600x130.tga", 600, 130),
        ("dialog600x600.tga", 600, 600),
    ] {
        write_round(&ui.join(name), w, h, night, hair)?;
    }
    write_frame(&ui.join("dialog1580x730.tga"), 1580, 730, night, hair)?;
    write_frame(&ui.join("dialog1580x700t.tga"), 1580, 700, night, hair)?;
    write_glass_alpha(&ui.join("serverboard.tga"), 1580, 780, 12.0, 160)?;
    write_glass_alpha(&ui.join("setupboard_l.tga"), 620, 670, 12.0, 160)?;
    write_glass_alpha(&ui.join("setupboard_r.tga"), 940, 670, 12.0, 160)?;
    // Wide short track strip — same glass as boards; radius capped by height.
    write_glass_alpha(&ui.join("setupbar.tga"), 1580, 56, 12.0, 160)?;
    write_glass_alpha(&ui.join("trackinfoboard.tga"), 940, 700, 12.0, 160)?;
    write_glass_alpha(&ui.join("bikeboard.tga"), 600, 460, 12.0, 160)?;
    write_glass_alpha(&ui.join("biketrackboard.tga"), 320, 70, 12.0, 160)?;
    write_glass_alpha(&ui.join("bikeinfoboard.tga"), 420, 200, 12.0, 160)?;
    write_glass_alpha(&ui.join("profilesboard.tga"), 1580, 730, 12.0, 160)?;
    write_glass_alpha(&ui.join("profilemodifyboard.tga"), 600, 300, 12.0, 160)?;
    write_glass_alpha(&ui.join("profiledeleteboard.tga"), 600, 200, 12.0, 160)?;
    write_glass_alpha(&ui.join("raceinfoboard.tga"), 1380, 670, 12.0, 160)?;
    write_glass_alpha(&ui.join("testpanelboard.tga"), 400, 200, 12.0, 160)?;
    // Corner caps over live track bitmaps (square TGA → rounded look).
    write_round_mask(&ui.join("trackmapmask.tga"), 650, 650, 28.0, 160)?;
    write_glass_alpha(&ui.join("replayboard.tga"), 600, 600, 12.0, 160)?;
    write_glass_alpha(&ui.join("replaypanel.tga"), 520, 130, 12.0, 160)?;
    write_glass_alpha(&ui.join("garageboard.tga"), 820, 570, 12.0, 160)?;
    write_glass_alpha(&ui.join("garagepanel_t.tga"), 440, 170, 12.0, 160)?;
    write_glass_alpha(&ui.join("garagepanel_r.tga"), 440, 470, 12.0, 160)?;
    write_glass_alpha(&ui.join("optionsboard.tga"), 1580, 700, 12.0, 210)?;
    write_check(&ui.join("check1.tga"), charcoal, hair, false, pip)?;
    write_check(&ui.join("check2.tga"), charcoal, pip, true, pip)?;
    Ok(())
}

pub(crate) fn rgb(c: [u8; 3]) -> Color {
    Color::from_rgba8(c[0], c[1], c[2], 255)
}

pub(crate) fn write_frame(path: &Path, w: u32, h: u32, fill: Color, stroke: Color) -> Result<(), String> {
    let mut px = Pixmap::new(w, h).ok_or("pixmap")?;
    let r = 8.0_f32.min(h as f32 * 0.2);
    fill_round(&mut px, 0.0, 0.0, w as f32, h as f32, r, fill);
    let inset = 8.0;
    let mut pb = PathBuilder::new();
    rounded_rect(
        &mut pb,
        inset,
        inset,
        w as f32 - inset * 2.0,
        h as f32 - inset * 2.0,
        r.max(1.0),
    );
    if let Some(inner) = pb.finish() {
        let mut paint = Paint::default();
        paint.blend_mode = BlendMode::Clear;
        paint.anti_alias = true;
        px.fill_path(
            &inner,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
    stroke_round(&mut px, 0.5, 0.5, w as f32 - 1.0, h as f32 - 1.0, r, stroke, 1.0);
    write_tga(path, &px)
}

pub(crate) fn write_card(path: &Path) -> Result<(), String> {
    let w = 360u32;
    let h = 920u32;
    // Rounded glass matching stock pointer_shadow / darkbox: pure black A=100.
    // Title wordmark is stock logo_ui.tga (item_bitmap ID_LOGO), not baked text.
    let mut px = Pixmap::new(w, h).ok_or("pixmap")?;
    fill_glass(&mut px, 12.0);
    write_tga(path, &px)
}

pub(crate) fn write_glass_alpha(path: &Path, w: u32, h: u32, radius: f32, alpha: u8) -> Result<(), String> {
    let mut px = Pixmap::new(w, h).ok_or("pixmap")?;
    fill_glass_alpha(&mut px, radius, alpha);
    write_tga(path, &px)
}

/// Opaque square with a rounded hole — covers square corners of a track map bitmap.

/// Opaque square with a rounded hole — covers square corners of a track map bitmap.
pub(crate) fn write_round_mask(path: &Path, w: u32, h: u32, radius: f32, alpha: u8) -> Result<(), String> {
    let mut px = Pixmap::new(w, h).ok_or("pixmap")?;
    px.fill(Color::from_rgba8(0, 0, 0, alpha));
    let mut pb = PathBuilder::new();
    rounded_rect(&mut pb, 0.0, 0.0, w as f32, h as f32, radius);
    if let Some(inner) = pb.finish() {
        let mut paint = Paint::default();
        paint.blend_mode = BlendMode::Clear;
        paint.anti_alias = true;
        px.fill_path(
            &inner,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
    write_tga(path, &px)
}

pub(crate) fn fill_glass(px: &mut Pixmap, radius: f32) {
    fill_glass_alpha(px, radius, 100);
}

pub(crate) fn fill_glass_alpha(px: &mut Pixmap, radius: f32, alpha: u8) {
    let glass = Color::from_rgba8(0, 0, 0, alpha);
    fill_round(
        px,
        0.0,
        0.0,
        px.width() as f32,
        px.height() as f32,
        radius,
        glass,
    );
}

pub(crate) fn write_row(
    path: &Path,
    fill: Option<Color>,
    wash: Option<Color>,
    pip: Option<Color>,
    stroke: Option<Color>,
) -> Result<(), String> {
    let w = 512u32;
    let h = 48u32;
    let mut px = Pixmap::new(w, h).ok_or("pixmap")?;
    if let Some(fill) = fill {
        fill_round(&mut px, 0.0, 0.0, w as f32, h as f32, 8.0, fill);
    }
    if let Some(wash) = wash {
        fill_round(&mut px, 0.0, 0.0, w as f32, h as f32, 8.0, wash);
    }
    if let Some(pip) = pip {
        fill_round(&mut px, 6.0, 10.0, 3.0, 28.0, 1.5, pip);
    }
    if let Some(stroke) = stroke {
        stroke_round(&mut px, 0.5, 0.5, w as f32 - 1.0, h as f32 - 1.0, 8.0, stroke, 1.0);
    }
    write_tga(path, &px)
}

pub(crate) fn write_tab(
    path: &Path,
    fill: Option<Color>,
    wash: Option<Color>,
    pip: Option<Color>,
    stroke: Option<Color>,
) -> Result<(), String> {
    let w = 200u32;
    let h = 36u32;
    let mut px = Pixmap::new(w, h).ok_or("pixmap")?;
    if let Some(fill) = fill {
        fill_round(&mut px, 0.0, 0.0, w as f32, h as f32, 6.0, fill);
    }
    if let Some(wash) = wash {
        fill_round(&mut px, 0.0, 0.0, w as f32, h as f32, 6.0, wash);
    }
    if let Some(pip) = pip {
        fill_round(&mut px, 4.0, 8.0, 3.0, 20.0, 1.5, pip);
    }
    if let Some(stroke) = stroke {
        stroke_round(&mut px, 0.5, 0.5, w as f32 - 1.0, h as f32 - 1.0, 6.0, stroke, 1.0);
    }
    write_tga(path, &px)
}

pub(crate) fn write_check(
    path: &Path,
    fill: Color,
    stroke: Color,
    checked: bool,
    mark: Color,
) -> Result<(), String> {
    let w = 32u32;
    let h = 32u32;
    let mut px = Pixmap::new(w, h).ok_or("pixmap")?;
    fill_round(&mut px, 1.0, 1.0, 30.0, 30.0, 4.0, fill);
    stroke_round(&mut px, 1.5, 1.5, 29.0, 29.0, 4.0, stroke, 1.5);
    if checked {
        let mut stroke_p = Paint::default();
        stroke_p.set_color(mark);
        stroke_p.anti_alias = true;
        let mut pb = PathBuilder::new();
        pb.move_to(8.0, 16.0);
        pb.line_to(13.0, 22.0);
        pb.line_to(24.0, 10.0);
        if let Some(path) = pb.finish() {
            px.stroke_path(
                &path,
                &stroke_p,
                &tiny_skia::Stroke {
                    width: 3.0,
                    ..Default::default()
                },
                Transform::identity(),
                None,
            );
        }
    }
    write_tga(path, &px)
}

pub(crate) fn write_round(path: &Path, w: u32, h: u32, fill: Color, stroke: Color) -> Result<(), String> {
    let mut px = Pixmap::new(w, h).ok_or("pixmap")?;
    let r = 8.0_f32.min(h as f32 * 0.2);
    fill_round(&mut px, 0.0, 0.0, w as f32, h as f32, r, fill);
    stroke_round(&mut px, 0.5, 0.5, w as f32 - 1.0, h as f32 - 1.0, r, stroke, 1.0);
    write_tga(path, &px)
}

#[derive(Clone, Copy)]
pub(crate) enum SegCap {
    Left,
    Right,
    Mid,
    Top,
    Bottom,
}

pub(crate) fn fill_seg(
    px: &mut Pixmap,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: f32,
    cap: SegCap,
    color: Color,
) {
    let mut pb = PathBuilder::new();
    seg_rect(&mut pb, x, y, w, h, r, cap);
    let Some(path) = pb.finish() else {
        return;
    };
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;
    px.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
}

pub(crate) fn stroke_seg(
    px: &mut Pixmap,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: f32,
    cap: SegCap,
    color: Color,
) {
    let mut pb = PathBuilder::new();
    seg_rect(&mut pb, x, y, w, h, r, cap);
    let Some(path) = pb.finish() else {
        return;
    };
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;
    let stroke = tiny_skia::Stroke {
        width: 1.0,
        ..tiny_skia::Stroke::default()
    };
    px.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
}

pub(crate) fn seg_rect(pb: &mut PathBuilder, x: f32, y: f32, w: f32, h: f32, r: f32, cap: SegCap) {
    let r = r.min(w * 0.5).min(h * 0.5).max(0.0);
    match cap {
        SegCap::Left => {
            pb.move_to(x + r, y);
            pb.line_to(x + w, y);
            pb.line_to(x + w, y + h);
            pb.line_to(x + r, y + h);
            pb.quad_to(x, y + h, x, y + h - r);
            pb.line_to(x, y + r);
            pb.quad_to(x, y, x + r, y);
        }
        SegCap::Right => {
            pb.move_to(x, y);
            pb.line_to(x + w - r, y);
            pb.quad_to(x + w, y, x + w, y + r);
            pb.line_to(x + w, y + h - r);
            pb.quad_to(x + w, y + h, x + w - r, y + h);
            pb.line_to(x, y + h);
            pb.line_to(x, y);
        }
        SegCap::Mid => {
            pb.move_to(x, y);
            pb.line_to(x + w, y);
            pb.line_to(x + w, y + h);
            pb.line_to(x, y + h);
            pb.line_to(x, y);
        }
        SegCap::Top => {
            pb.move_to(x + r, y);
            pb.line_to(x + w - r, y);
            pb.quad_to(x + w, y, x + w, y + r);
            pb.line_to(x + w, y + h);
            pb.line_to(x, y + h);
            pb.line_to(x, y + r);
            pb.quad_to(x, y, x + r, y);
        }
        SegCap::Bottom => {
            pb.move_to(x, y);
            pb.line_to(x + w, y);
            pb.line_to(x + w, y + h - r);
            pb.quad_to(x + w, y + h, x + w - r, y + h);
            pb.line_to(x + r, y + h);
            pb.quad_to(x, y + h, x, y + h - r);
            pb.line_to(x, y);
        }
    }
    pb.close();
}

pub(crate) fn fill_round(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, r: f32, color: Color) {
    let mut pb = PathBuilder::new();
    rounded_rect(&mut pb, x, y, w, h, r);
    let Some(path) = pb.finish() else {
        return;
    };
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;
    px.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
}

pub(crate) fn stroke_round(
    px: &mut Pixmap,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: f32,
    color: Color,
    width: f32,
) {
    let mut pb = PathBuilder::new();
    rounded_rect(&mut pb, x, y, w, h, r);
    let Some(path) = pb.finish() else {
        return;
    };
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;
    let stroke = tiny_skia::Stroke {
        width,
        ..tiny_skia::Stroke::default()
    };
    px.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
}

pub(crate) fn write_seg_track(path: &Path, fill: Color, stroke: Color) -> Result<(), String> {
    write_round(path, 220, 36, fill, stroke)
}

pub(crate) fn write_seg(
    path: &Path,
    cap: SegCap,
    wash: Option<Color>,
    stroke: Option<Color>,
) -> Result<(), String> {
    let w = 110u32;
    let h = 36u32;
    let mut px = Pixmap::new(w, h).ok_or("pixmap")?;
    let inset = 2.0;
    let rw = w as f32 - inset * 2.0;
    let rh = h as f32 - inset * 2.0;
    let r = 6.0;
    if let Some(wash) = wash {
        fill_seg(&mut px, inset, inset, rw, rh, r, cap, wash);
    }
    if let Some(stroke) = stroke {
        stroke_seg(&mut px, inset, inset, rw, rh, r, cap, stroke);
    }
    write_tga(path, &px)
}

pub(crate) fn write_skew(path: &Path, w: u32, h: u32, fill: Color, _ink: Color) -> Result<(), String> {
    let mut px = Pixmap::new(w, h).ok_or("pixmap")?;
    let skew = 10.0;
    let mut pb = PathBuilder::new();
    pb.move_to(skew, 0.0);
    pb.line_to(w as f32, 0.0);
    pb.line_to(w as f32 - skew, h as f32);
    pb.line_to(0.0, h as f32);
    pb.close();
    if let Some(path) = pb.finish() {
        let mut paint = Paint::default();
        paint.set_color(fill);
        paint.anti_alias = true;
        px.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
    }
    write_tga(path, &px)
}

pub(crate) fn rounded_rect(pb: &mut PathBuilder, x: f32, y: f32, w: f32, h: f32, r: f32) {
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

pub(crate) fn write_tga(path: &Path, px: &Pixmap) -> Result<(), String> {
    let w = px.width() as u16;
    let h = px.height() as u16;
    let src = px.data();
    let mut out = Vec::with_capacity(18 + src.len());
    out.extend_from_slice(&[
        0u8,
        0,
        2,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        (w & 0xff) as u8,
        (w >> 8) as u8,
        (h & 0xff) as u8,
        (h >> 8) as u8,
        32,
        8,
    ]);
    // bottom-up BGRA
    for y in (0..h as usize).rev() {
        let row = y * w as usize * 4;
        for x in 0..w as usize {
            let i = row + x * 4;
            out.push(src[i + 2]);
            out.push(src[i + 1]);
            out.push(src[i]);
            out.push(src[i + 3]);
        }
    }
    fs::write(path, out).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_ui() -> std::path::PathBuf {
        let p = env::temp_dir().join(format!(
            "mxbo-vseg-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    /// Alpha at top-down (x, y) in a bottom-up BGRA TGA.
    fn tga_a(data: &[u8], w: usize, h: usize, x: usize, y: usize) -> u8 {
        let file_y = h - 1 - y;
        data[18 + (file_y * w + x) * 4 + 3]
    }

    #[test]
    fn vertical_seg_caps_have_correct_corner_geometry() {
        let ui = temp_ui();
        let fill = Color::from_rgba8(40, 40, 44, 255);
        let stroke = Color::from_rgba8(80, 80, 84, 255);
        write_seg(&ui.join("sevt1.tga"), SegCap::Top, Some(fill), Some(stroke)).unwrap();
        write_seg(&ui.join("sevm1.tga"), SegCap::Mid, Some(fill), Some(stroke)).unwrap();
        write_seg(&ui.join("sevb1.tga"), SegCap::Bottom, Some(fill), Some(stroke)).unwrap();

        let w = 110usize;
        let h = 36usize;
        let inset = 2usize;
        let top = fs::read(ui.join("sevt1.tga")).unwrap();
        let mid = fs::read(ui.join("sevm1.tga")).unwrap();
        let bot = fs::read(ui.join("sevb1.tga")).unwrap();

        // Mid: opaque near all four inset corners (square).
        for (x, y) in [
            (inset + 1, inset + 1),
            (w - inset - 2, inset + 1),
            (inset + 1, h - inset - 2),
            (w - inset - 2, h - inset - 2),
        ] {
            assert!(
                tga_a(&mid, w, h, x, y) > 200,
                "mid should be opaque at ({x},{y})"
            );
        }

        // Top: opaque under rounded top; opaque at flat bottom corners; clear at
        // extreme pixmap top corners (outside the round).
        assert!(tga_a(&top, w, h, w / 2, inset + 1) > 200, "top center under top edge");
        assert!(
            tga_a(&top, w, h, inset + 1, h - inset - 2) > 200,
            "top flat bottom-left corner must be filled"
        );
        assert!(
            tga_a(&top, w, h, w - inset - 2, h - inset - 2) > 200,
            "top flat bottom-right corner must be filled"
        );
        assert_eq!(tga_a(&top, w, h, 0, 0), 0, "top pixmap TL clear");
        assert_eq!(tga_a(&top, w, h, w - 1, 0), 0, "top pixmap TR clear");

        // Bottom: opaque above rounded bottom; opaque at flat top corners.
        assert!(
            tga_a(&bot, w, h, w / 2, h - inset - 2) > 200,
            "bottom center above bottom edge"
        );
        assert!(
            tga_a(&bot, w, h, inset + 1, inset + 1) > 200,
            "bottom flat top-left corner must be filled"
        );
        assert!(
            tga_a(&bot, w, h, w - inset - 2, inset + 1) > 200,
            "bottom flat top-right corner must be filled"
        );
        assert_eq!(tga_a(&bot, w, h, 0, h - 1), 0, "bottom pixmap BL clear");
        assert_eq!(tga_a(&bot, w, h, w - 1, h - 1), 0, "bottom pixmap BR clear");

        let _ = fs::remove_dir_all(&ui);
    }
}

