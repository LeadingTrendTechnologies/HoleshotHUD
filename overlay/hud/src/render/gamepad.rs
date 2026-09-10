#![allow(unused_imports)]
use super::*;

pub(crate) struct GamepadLayout {
    ls: [f32; 2],
    rs: [f32; 2],
    well_r: f32,
    l2: [f32; 4],
    r2: [f32; 4],
    l1: [f32; 4],
    r1: [f32; 4],
    l2_seed: [f32; 2],
    r2_seed: [f32; 2],
    l1_seed: [f32; 2],
    r1_seed: [f32; 2],
    face: [[f32; 2]; 4],
    face_r: f32,
    dpad: [[f32; 2]; 4],
    dpad_half: [f32; 2],
    /// Per-direction arm UV rects `[u, v, w, h]` for `shade_art` (Xbox cross). `None` = DS4 flood fill.
    dpad_seg: Option<[[f32; 4]; 4]>,
    back: [f32; 4],
    start: [f32; 4],
    guide: [f32; 2],
    guide_r: f32,
    touch: Option<[f32; 4]>,
}

pub(crate) fn ds4_gamepad_layout() -> GamepadLayout {
    GamepadLayout {
        ls: [0.3554, 0.6117],
        rs: [0.6425, 0.6117],
        well_r: 108.0,
        l2: [0.168, 0.036, 0.132, 0.145],
        r2: [0.698, 0.036, 0.132, 0.145],
        l1: [0.170, 0.162, 0.132, 0.055],
        r1: [0.698, 0.162, 0.132, 0.055],
        l2_seed: [0.235, 0.105],
        r2_seed: [0.764, 0.088],
        l1_seed: [0.235, 0.193],
        r1_seed: [0.764, 0.193],
        face: [
            [0.7669, 0.3389],
            [0.8249, 0.4258],
            [0.7689, 0.5107],
            [0.7103, 0.4229],
        ],
        face_r: 39.0,
        dpad: [
            [0.2272, 0.3617],
            [0.2697, 0.4243],
            [0.2275, 0.4872],
            [0.1850, 0.4245],
        ],
        dpad_half: [0.055, 0.065],
        dpad_seg: None,
        back: [0.255, 0.255, 0.085, 0.040],
        start: [0.660, 0.255, 0.085, 0.040],
        guide: [0.4996, 0.6261],
        guide_r: 13.0,
        touch: Some([0.390, 0.280, 0.220, 0.160]),
    }
}

/// `overlay/hud/assets/gamepad-xbox.png` (1536×1024). Layout measured from gamepad-xbox-ref.png.
pub(crate) fn xbox_gamepad_layout() -> GamepadLayout {
    GamepadLayout {
        ls: [426.0 / 1536.0, 460.0 / 1024.0],
        rs: [942.0 / 1536.0, 627.0 / 1024.0],
        well_r: 92.0,
        l2: [248.0 / 1536.0, 47.0 / 1024.0, 303.0 / 1536.0, 183.0 / 1024.0],
        r2: [948.0 / 1536.0, 47.0 / 1024.0, 353.0 / 1536.0, 183.0 / 1024.0],
        l1: [304.0 / 1536.0, 218.0 / 1024.0, 276.0 / 1536.0, 123.0 / 1024.0],
        r1: [951.0 / 1536.0, 216.0 / 1024.0, 276.0 / 1536.0, 114.0 / 1024.0],
        l2_seed: [399.0 / 1536.0, 210.0 / 1024.0],
        r2_seed: [1124.0 / 1536.0, 210.0 / 1024.0],
        l1_seed: [437.0 / 1536.0, 274.0 / 1024.0],
        r1_seed: [1081.0 / 1536.0, 272.0 / 1024.0],
        face: [
            [1092.0 / 1536.0, 360.0 / 1024.0],
            [1161.0 / 1536.0, 436.0 / 1024.0],
            [1092.0 / 1536.0, 517.0 / 1024.0],
            [1023.0 / 1536.0, 436.0 / 1024.0],
        ],
        face_r: 44.0,
        dpad: [
            [625.0 / 1536.0, 557.0 / 1024.0],
            [691.0 / 1536.0, 623.0 / 1024.0],
            [625.0 / 1536.0, 689.0 / 1024.0],
            [559.0 / 1536.0, 623.0 / 1024.0],
        ],
        dpad_half: [66.0 / 1536.0, 66.0 / 1024.0],
        dpad_seg: Some([
            [609.0 / 1536.0, 557.0 / 1024.0, 32.0 / 1536.0, 66.0 / 1024.0],
            [625.0 / 1536.0, 607.0 / 1024.0, 66.0 / 1536.0, 32.0 / 1024.0],
            [609.0 / 1536.0, 623.0 / 1024.0, 32.0 / 1536.0, 66.0 / 1024.0],
            [559.0 / 1536.0, 607.0 / 1024.0, 66.0 / 1536.0, 32.0 / 1024.0],
        ]),
        back: [614.0 / 1536.0, 394.0 / 1024.0, 155.0 / 1536.0, 51.0 / 1024.0],
        start: [778.0 / 1536.0, 394.0 / 1024.0, 155.0 / 1536.0, 51.0 / 1024.0],
        guide: [768.0 / 1536.0, 337.0 / 1024.0],
        guide_r: 22.0,
        touch: None,
    }
}

pub(crate) fn draw_gamepad(px: &mut Pixmap, fonts: &Fonts, cfg: &HudConfig, sw: f32, sh: f32) {
    // THESIS: a live DualShock over the dirt. Orange is the press. Sticks leave their wells. Triggers fill with squeeze. Bumpers light when held.
    // OWN-WORLD: Broadcast Booth Glass — DualShock drawing, no night-ink plaque by default, Holeshot orange live, Exo 2 ExtraBold Italic labels.
    // STORY: glance which inputs are live without looking at the pad. No pad is a small No controller pill.
    // FIRST VIEWPORT: DualShock 4 keeps its proportions. L2/R2 are trigger wings on the silhouette (analog orange from the bottom). L1/R1 are DualShock shoulder bars. Cross/A orange. Left stick translated.
    // FORM: Glass. Seed: user-locked gamepad-glass.png plus analog triggers and bumpers. Panel opacity 0: just the pad.
    // FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
    let r = cfg[WidgetId::Gamepad].rect;
    let x = r.x * sw;
    let y = r.y * sh;
    let w = (r.w * sw).max(140.0);
    let h = (r.h * sh).max(88.0);
    let a = bg_a(cfg[WidgetId::Gamepad].bg);
    if a > 0 {
        fill_round(px, x, y, w, h, 6.0, Color::from_rgba8(10, 10, 10, a));
        if let Some(frame) = round_rect_path(x + 0.5, y + 0.5, w - 1.0, h - 1.0, 5.5) {
            let edge = ((a as u16 * 170) / 255).max(70) as u8;
            stroke_path(px, &frame, Color::from_rgba8(42, 42, 46, edge), 1.0);
        }
    }

    let pad = crate::gamepad::current();
    if !pad.connected() {
        let fs = (h * 0.22).clamp(11.0, 18.0);
        let label = "No controller";
        if a == 0 {
            let tw = measure(fonts, label, fs);
            fill_night_pill(px, x + (w - tw) * 0.5 - 8.0, y + (h - fs) * 0.48 - 4.0, tw + 16.0, fs + 8.0);
        }
        text(px, fonts, label, fs, x + w * 0.5, y + (h - fs) * 0.48, text_col(), true);
        return;
    }

    let sony = cfg.gamepad_style.sony_art(pad.kind);
    let well = Color::from_rgba8(22, 22, 24, 255);
    let cream = Color::from_rgba8(248, 248, 252, 255);
    let Some(art) = gamepad_art(sony) else {
        return;
    };
    let Some((dx, dy, dw, dh)) = blit_gamepad_art(px, art, sony, x, y, w, h) else {
        return;
    };

    let layout = if sony {
        ds4_gamepad_layout()
    } else {
        xbox_gamepad_layout()
    };
    let (lsx, lsy) = (dx + layout.ls[0] * dw, dy + layout.ls[1] * dh);
    let (rsx, rsy) = (dx + layout.rs[0] * dw, dy + layout.rs[1] * dh);
    let well_r = layout.well_r / art.height() as f32 * dh;
    plug_stick_well(px, lsx, lsy, well_r, well);
    plug_stick_well(px, rsx, rsy, well_r, well);

    if pad.lt >= 0.03 {
        shade_press_flood(
            px, art, dx, dy, dw, dh, layout.l2_seed[0], layout.l2_seed[1], layout.l2, pad.lt, sony,
        );
    }
    if pad.rt >= 0.03 {
        shade_press_flood(
            px, art, dx, dy, dw, dh, layout.r2_seed[0], layout.r2_seed[1], layout.r2, pad.rt, sony,
        );
    }
    if pad.down(crate::gamepad::LB) {
        shade_press_flood(
            px, art, dx, dy, dw, dh, layout.l1_seed[0], layout.l1_seed[1], layout.l1, 1.0, sony,
        );
    }
    if pad.down(crate::gamepad::RB) {
        shade_press_flood(
            px, art, dx, dy, dw, dh, layout.r1_seed[0], layout.r1_seed[1], layout.r1, 1.0, sony,
        );
    }
    if let Some(touch) = layout.touch {
        if pad.down(crate::gamepad::TOUCH) {
            shade_art(px, art, dx, dy, dw, dh, touch, 0.0, ShadeMode::Interior, accent(), |_, _| true);
        }
    }
    if pad.down(crate::gamepad::BACK) {
        shade_art(px, art, dx, dy, dw, dh, layout.back, 0.0, ShadeMode::Interior, accent(), |_, _| true);
    }
    if pad.down(crate::gamepad::START) {
        shade_art(px, art, dx, dy, dw, dh, layout.start, 0.0, ShadeMode::Interior, accent(), |_, _| true);
    }

    shade_dpad(px, art, dx, dy, dw, dh, pad, &layout);
    shade_face(px, dx, dy, dw, dh, pad, &layout);

    draw_stick_live(px, art, dx, dy, dw, dh, lsx, lsy, pad.lx, pad.ly, pad.down(crate::gamepad::LS), well, cream, layout.well_r);
    draw_stick_live(px, art, dx, dy, dw, dh, rsx, rsy, pad.rx, pad.ry, pad.down(crate::gamepad::RS), well, cream, layout.well_r);

    if pad.down(crate::gamepad::GUIDE) {
        shade_disc(px, art, dx, dy, dw, dh, layout.guide[0], layout.guide[1], layout.guide_r, ShadeMode::Interior, accent());
    }
}

pub(crate) fn gamepad_art(sony: bool) -> Option<&'static Pixmap> {
    static DS4: OnceLock<Option<Pixmap>> = OnceLock::new();
    static XB: OnceLock<Option<Pixmap>> = OnceLock::new();
    if sony {
        DS4.get_or_init(|| Pixmap::decode_png(include_bytes!("../../assets/gamepad-ds4.png")).ok())
            .as_ref()
    } else {
        XB.get_or_init(|| Pixmap::decode_png(include_bytes!("../../assets/gamepad-xbox.png")).ok())
            .as_ref()
    }
}

pub(crate) fn blit_gamepad_art(
    px: &mut Pixmap,
    src: &Pixmap,
    sony: bool,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) -> Option<(f32, f32, f32, f32)> {
    let iw = src.width() as f32;
    let ih = src.height() as f32;
    if iw < 1.0 || ih < 1.0 {
        return None;
    }
    let inset = (w.min(h) * 0.03).clamp(2.0, 8.0);
    let aw = (w - inset * 2.0).max(48.0);
    let ah = (h - inset * 2.0).max(36.0);
    let scale = (aw / iw).min(ah / ih).max(0.01);
    let dw = iw * scale;
    let dh = ih * scale;
    let dx = x + (w - dw) * 0.5;
    let dy = y + (h - dh) * 0.5;
    let tw = dw.round().max(1.0) as u32;
    let th = dh.round().max(1.0) as u32;
    let Some(hi) = scaled_gamepad_art(src, sony, tw, th) else {
        return None;
    };
    let mut paint = PixmapPaint::default();
    paint.quality = if sony {
        FilterQuality::Bilinear
    } else {
        FilterQuality::Bicubic
    };
    px.draw_pixmap(
        0,
        0,
        hi.as_ref(),
        &paint,
        Transform::from_translate(dx, dy),
        None,
    );
    Some((dx, dy, dw, dh))
}

pub(crate) fn scaled_gamepad_art(src: &Pixmap, sony: bool, tw: u32, th: u32) -> Option<Pixmap> {
    thread_local! {
        static CACHE: RefCell<Option<(bool, u32, u32, Pixmap)>> = const { RefCell::new(None) };
    }
    CACHE.with(|slot| {
        if let Some((s, w, h, px)) = slot.borrow().as_ref() {
            if *s == sony && *w == tw && *h == th {
                return Some(px.clone());
            }
        }
        let sw = tw.saturating_mul(4).max(4);
        let sh = th.saturating_mul(4).max(4);
        let mut hi = Pixmap::new(sw, sh)?;
        let sx = sw as f32 / src.width() as f32;
        let sy = sh as f32 / src.height() as f32;
        let mut paint = PixmapPaint::default();
        paint.quality = FilterQuality::Bilinear;
        hi.draw_pixmap(0, 0, src.as_ref(), &paint, Transform::from_scale(sx, sy), None);
        let mut lo = Pixmap::new(tw, th)?;
        let mut down = PixmapPaint::default();
        down.quality = FilterQuality::Bicubic;
        lo.draw_pixmap(
            0,
            0,
            hi.as_ref(),
            &down,
            Transform::from_scale(tw as f32 / sw as f32, th as f32 / sh as f32),
            None,
        );
        *slot.borrow_mut() = Some((sony, tw, th, lo.clone()));
        Some(lo)
    })
}

#[derive(Clone, Copy)]
pub(crate) enum ShadeMode {
    /// Dark DualShock fill. Leaves the drawing's outlines and baked labels.
    Interior,
    /// Every opaque art pixel (cap cover).
    All,
    /// The drawing's light strokes (stick well ring).
    Stroke,
}

/// Recolor DualShock art. `y_cut` 0 paints the whole box; analog uses `1 - squeeze`.
pub(crate) fn shade_art(
    px: &mut Pixmap,
    art: &Pixmap,
    dx: f32,
    dy: f32,
    dw: f32,
    dh: f32,
    uv: [f32; 4],
    y_cut: f32,
    mode: ShadeMode,
    color: Color,
    pred: impl Fn(f32, f32) -> bool,
) {
    let iw = art.width();
    let ih = art.height();
    if iw == 0 || ih == 0 || dw < 1.0 || dh < 1.0 || uv[2] <= 0.0 || uv[3] <= 0.0 {
        return;
    }
    let u8c = color.to_color_u8();
    let Some(paint) = PremultipliedColorU8::from_rgba(
        ((u8c.red() as u16 * u8c.alpha() as u16) / 255) as u8,
        ((u8c.green() as u16 * u8c.alpha() as u16) / 255) as u8,
        ((u8c.blue() as u16 * u8c.alpha() as u16) / 255) as u8,
        u8c.alpha(),
    ) else {
        return;
    };
    let x0 = ((dx + uv[0] * dw).floor() as i32).max(0) as u32;
    let y0 = ((dy + uv[1] * dh).floor() as i32).max(0) as u32;
    let x1 = ((dx + (uv[0] + uv[2]) * dw).ceil() as u32).min(px.width());
    let y1 = ((dy + (uv[1] + uv[3]) * dh).ceil() as u32).min(px.height());
    let cut = y_cut.clamp(0.0, 1.0);
    let dw_px = px.width();
    let dest = px.pixels_mut();
    for py in y0..y1 {
        let v = (py as f32 + 0.5 - dy) / dh;
        let local_y = (v - uv[1]) / uv[3];
        if local_y < cut {
            continue;
        }
        for px_ in x0..x1 {
            let u = (px_ as f32 + 0.5 - dx) / dw;
            if !pred(u, v) {
                continue;
            }
            let ax = (u * iw as f32).floor() as i32;
            let ay = (v * ih as f32).floor() as i32;
            if ax < 0 || ay < 0 || ax as u32 >= iw || ay as u32 >= ih {
                continue;
            }
            let Some(src) = art.pixel(ax as u32, ay as u32) else {
                continue;
            };
            let a = src.alpha();
            if a < 24 {
                continue;
            }
            let lum = ((src.red() as u16 * 30 + src.green() as u16 * 59 + src.blue() as u16 * 11) / 100) as u8;
            let hit = match mode {
                ShadeMode::Interior => lum < 78,
                ShadeMode::All => true,
                ShadeMode::Stroke => lum >= 78,
            };
            if hit {
                dest[(py * dw_px + px_) as usize] = paint;
            }
        }
    }
}

/// Opaque charcoal behind any hole in a stick well. Leaves the DualShock rings and cap.
pub(crate) fn plug_stick_well(px: &mut Pixmap, cx: f32, cy: f32, r: f32, well: Color) {
    if r < 1.0 {
        return;
    }
    let wr = well.to_color_u8();
    let (wr, wg, wb) = (wr.red() as f32, wr.green() as f32, wr.blue() as f32);
    let x0 = ((cx - r).floor() as i32).max(0) as u32;
    let y0 = ((cy - r).floor() as i32).max(0) as u32;
    let x1 = ((cx + r).ceil() as u32).min(px.width());
    let y1 = ((cy + r).ceil() as u32).min(px.height());
    let dw = px.width();
    let dest = px.pixels_mut();
    let rr = r * r;
    for py in y0..y1 {
        for px_ in x0..x1 {
            let dx = px_ as f32 + 0.5 - cx;
            let dy = py as f32 + 0.5 - cy;
            if dx * dx + dy * dy > rr {
                continue;
            }
            let i = (py * dw + px_) as usize;
            let d = dest[i];
            let a = d.alpha();
            if a >= 250 {
                continue;
            }
            let da = a as f32 / 255.0;
            let ur = if a == 0 { 0.0 } else { d.red() as f32 / da };
            let ug = if a == 0 { 0.0 } else { d.green() as f32 / da };
            let ub = if a == 0 { 0.0 } else { d.blue() as f32 / da };
            let nr = (ur * da + wr * (1.0 - da)).round().clamp(0.0, 255.0) as u8;
            let ng = (ug * da + wg * (1.0 - da)).round().clamp(0.0, 255.0) as u8;
            let nb = (ub * da + wb * (1.0 - da)).round().clamp(0.0, 255.0) as u8;
            if let Some(p) = PremultipliedColorU8::from_rgba(nr, ng, nb, 255) {
                dest[i] = p;
            }
        }
    }
}

pub(crate) fn shade_disc(
    px: &mut Pixmap,
    art: &Pixmap,
    dx: f32,
    dy: f32,
    dw: f32,
    dh: f32,
    cx: f32,
    cy: f32,
    r_px: f32,
    mode: ShadeMode,
    color: Color,
) {
    let iw = art.width() as f32;
    let ih = art.height() as f32;
    if iw < 1.0 || ih < 1.0 {
        return;
    }
    let ru = (r_px + 1.0) / iw;
    let rv = (r_px + 1.0) / ih;
    shade_art(
        px,
        art,
        dx,
        dy,
        dw,
        dh,
        [cx - ru, cy - rv, ru * 2.0, rv * 2.0],
        0.0,
        mode,
        color,
        |u, v| {
            let ddx = (u - cx) * iw;
            let ddy = (v - cy) * ih;
            ddx * ddx + ddy * ddy <= r_px * r_px
        },
    );
}

pub(crate) fn shade_face(px: &mut Pixmap, dx: f32, dy: f32, dw: f32, dh: f32, pad: crate::gamepad::PadState, layout: &GamepadLayout) {
    let bits = [
        crate::gamepad::NORTH,
        crate::gamepad::EAST,
        crate::gamepad::SOUTH,
        crate::gamepad::WEST,
    ];
    for (i, &bit) in bits.iter().enumerate() {
        if pad.down(bit) {
            let [u, v] = layout.face[i];
            shade_press_disc(px, dx, dy, dw, dh, u, v, layout.face_r);
        }
    }
}

pub(crate) fn shade_dpad(px: &mut Pixmap, art: &Pixmap, dx: f32, dy: f32, dw: f32, dh: f32, pad: crate::gamepad::PadState, layout: &GamepadLayout) {
    let bits = [
        crate::gamepad::UP,
        crate::gamepad::RIGHT,
        crate::gamepad::DOWN,
        crate::gamepad::LEFT,
    ];
    if let Some(segs) = layout.dpad_seg {
        for (i, &bit) in bits.iter().enumerate() {
            if pad.down(bit) {
                shade_art(px, art, dx, dy, dw, dh, segs[i], 0.0, ShadeMode::Interior, accent(), |_, _| true);
            }
        }
        return;
    }
    let [hu, hv] = layout.dpad_half;
    for (i, &bit) in bits.iter().enumerate() {
        if pad.down(bit) {
            let [u, v] = layout.dpad[i];
            let uv = [u - hu, v - hv, hu * 2.0, hv * 2.0];
            shade_press_flood(px, art, dx, dy, dw, dh, u, v, uv, 1.0, true);
        }
    }
}

/// Orange fill out to a circular outline. Cream glyphs and the outline stay 1px — not thickened.
pub(crate) fn shade_press_disc(
    px: &mut Pixmap,
    dx: f32,
    dy: f32,
    dw: f32,
    dh: f32,
    cu: f32,
    cv: f32,
    r_art: f32,
) {
    if dw < 1.0 || dh < 1.0 {
        return;
    }
    let iw = 1536.0;
    let ih = 1024.0;
    let ru = (r_art + 2.0) / iw;
    let rv = (r_art + 2.0) / ih;
    let x0 = ((dx + (cu - ru) * dw).floor() as i32).max(0) as u32;
    let y0 = ((dy + (cv - rv) * dh).floor() as i32).max(0) as u32;
    let x1 = ((dx + (cu + ru) * dw).ceil() as u32).min(px.width());
    let y1 = ((dy + (cv + rv) * dh).ceil() as u32).min(px.height());
    let dw_px = px.width();
    let dest = px.pixels_mut();
    for py in y0..y1 {
        for px_ in x0..x1 {
            let u = (px_ as f32 + 0.5 - dx) / dw;
            let v = (py as f32 + 0.5 - dy) / dh;
            let ddx = (u - cu) * iw;
            let ddy = (v - cv) * ih;
            let r = (ddx * ddx + ddy * ddy).sqrt();
            if r > r_art {
                continue;
            }
            let i = (py * dw_px + px_) as usize;
            let (lum, da) = dest_lum_alpha(dest[i]);
            if da < 8 {
                continue;
            }
            // Leave the DualShock cream ring and glyph. Orange only the dark interior.
            if lum > 92 {
                continue;
            }
            if let Some(p) = paint_keep_alpha(255.0, 148.0, 48.0, da) {
                dest[i] = p;
            }
        }
    }
}

/// Orange fill that follows a DualShock control in art space.
/// `squeeze` 0…1 fills from the curved bottom of the control (triggers); 1 is a full press.
pub(crate) fn shade_press_flood(
    px: &mut Pixmap,
    art: &Pixmap,
    dx: f32,
    dy: f32,
    dw: f32,
    dh: f32,
    seed_u: f32,
    seed_v: f32,
    uv: [f32; 4],
    squeeze: f32,
    ds4_lip: bool,
) {
    let iw = art.width();
    let ih = art.height();
    if iw == 0 || ih == 0 || dw < 1.0 || dh < 1.0 || uv[2] <= 0.0 || uv[3] <= 0.0 {
        return;
    }
    let ax0 = ((uv[0] * iw as f32).floor() as i32).max(0) as u32;
    let ay0 = ((uv[1] * ih as f32).floor() as i32).max(0) as u32;
    let ax1 = ((uv[0] + uv[2]) * iw as f32).ceil() as u32;
    let ay1 = ((uv[1] + uv[3]) * ih as f32).ceil() as u32;
    let ax1 = ax1.min(iw);
    let ay1 = ay1.min(ih);
    if ax1 <= ax0 || ay1 <= ay0 {
        return;
    }
    let bw = (ax1 - ax0) as usize;
    let bh = (ay1 - ay0) as usize;
    let mut sx = ((seed_u * iw as f32).floor() as i32).clamp(ax0 as i32, ax1 as i32 - 1) as u32;
    let mut sy = ((seed_v * ih as f32).floor() as i32).clamp(ay0 as i32, ay1 as i32 - 1) as u32;
    let fillable = |x: u32, y: u32| -> bool { art_lum_at(art, x, y) < 78 };
    if !fillable(sx, sy) {
        let mut found = None;
        'hunt: for d in 1..=16u32 {
            for oy in 0..=d {
                let ox = d - oy;
                for &(ix, iy) in &[
                    (sx as i32 + ox as i32, sy as i32 + oy as i32),
                    (sx as i32 - ox as i32, sy as i32 + oy as i32),
                    (sx as i32 + ox as i32, sy as i32 - oy as i32),
                    (sx as i32 - ox as i32, sy as i32 - oy as i32),
                ] {
                    if ix < ax0 as i32 || iy < ay0 as i32 || ix >= ax1 as i32 || iy >= ay1 as i32 {
                        continue;
                    }
                    let (x, y) = (ix as u32, iy as u32);
                    if fillable(x, y) {
                        found = Some((x, y));
                        break 'hunt;
                    }
                }
            }
        }
        match found {
            Some((x, y)) => {
                sx = x;
                sy = y;
            }
            None => return,
        }
    }
    let idx = |x: u32, y: u32| ((y - ay0) as usize) * bw + (x - ax0) as usize;
    let mut fill = vec![false; bw * bh];
    let mut stack = vec![(sx, sy)];
    while let Some((x, y)) = stack.pop() {
        if x < ax0 || y < ay0 || x >= ax1 || y >= ay1 {
            continue;
        }
        let i = idx(x, y);
        if fill[i] || !fillable(x, y) {
            continue;
        }
        fill[i] = true;
        if x > ax0 {
            stack.push((x - 1, y));
        }
        if x + 1 < ax1 {
            stack.push((x + 1, y));
        }
        if y > ay0 {
            stack.push((x, y - 1));
        }
        if y + 1 < ay1 {
            stack.push((x, y + 1));
        }
    }
    // DS4 tombstones: labels sit in the wing; cream lip below them caps the analog fill.
    let lip = if ds4_lip {
        let skip = (sy + 40).min(ay1);
        let mut found = None;
        for y in skip..ay1 {
            let mut cream = 0u32;
            let mut fill_w = 0u32;
            for x in ax0..ax1 {
                if art_lum_at(art, x, y) >= 96 {
                    cream += 1;
                }
                if fill[idx(x, y)] {
                    fill_w += 1;
                }
            }
            if cream > fill_w && cream > 40 {
                found = Some(y);
                break;
            }
        }
        found
    } else {
        None
    };
    if ds4_lip {
        if let Some(lip_y) = lip {
            // Follow the rounded bottom: each column stops at its own cream lip, not one flat row.
            let y_lo = lip_y.saturating_sub(6).max(ay0);
            let y_hi = (lip_y + 12).min(ay1);
            for x in ax0..ax1 {
                let mut interior = false;
                for y in y_lo..lip_y {
                    if fill[idx(x, y)] {
                        interior = true;
                        break;
                    }
                }
                if !interior {
                    continue;
                }
                let mut bottom = lip_y;
                for y in y_lo..y_hi {
                    if art_lum_at(art, x, y) < 96 {
                        continue;
                    }
                    let left = x > ax0 && art_lum_at(art, x - 1, y) >= 96;
                    let right = x + 1 < ax1 && art_lum_at(art, x + 1, y) >= 96;
                    if left || right {
                        bottom = y;
                        break;
                    }
                }
                for y in bottom..ay1 {
                    fill[idx(x, y)] = false;
                }
            }
        }
    }
    let mut ymin = ay1;
    let mut ymax = ay0;
    for y in ay0..ay1 {
        for x in ax0..ax1 {
            if fill[idx(x, y)] {
                ymin = ymin.min(y);
                ymax = ymax.max(y);
            }
        }
    }
    if ymin > ymax {
        return;
    }
    let squeeze = if squeeze >= 0.92 { 1.0 } else { squeeze.clamp(0.0, 1.0) };
    let span = (ymax.saturating_sub(ymin)).max(1) as f32;
    let cut = ymin as f32 + (1.0 - squeeze) * span;
    let fade = (ih as f32 / dh).max(1.0);
    let x0 = ((dx + uv[0] * dw).floor() as i32).max(0) as u32;
    let y0 = ((dy + uv[1] * dh).floor() as i32).max(0) as u32;
    let x1 = ((dx + (uv[0] + uv[2]) * dw).ceil() as u32).min(px.width());
    let y1 = ((dy + (uv[1] + uv[3]) * dh).ceil() as u32).min(px.height());
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    let dw_px = px.width();
    let dest = px.pixels_mut();
    for py in y0..y1 {
        for px_ in x0..x1 {
            let u = (px_ as f32 + 0.5 - dx) / dw;
            let v = (py as f32 + 0.5 - dy) / dh;
            let ax = (u * iw as f32).floor() as i32;
            let ay = (v * ih as f32).floor() as i32;
            if ax < ax0 as i32 || ay < ay0 as i32 || ax as u32 >= ax1 || ay as u32 >= ay1 {
                continue;
            }
            let (ax, ay) = (ax as u32, ay as u32);
            let ai = idx(ax, ay);
            let di = (py * dw_px + px_) as usize;
            let (lum, da) = dest_lum_alpha(dest[di]);
            if da < 8 {
                continue;
            }
            // Keep the drawing's 1px cream labels and outlines — do not ink or thicken them.
            if lum > 92 {
                continue;
            }
            let rise = ((ay as f32 + 0.5 - cut) / fade).clamp(0.0, 1.0);
            if fill[ai] && rise > 0.02 {
                let t = rise;
                let a = da as f32;
                let inv = 1.0 - t;
                let r = (255.0 * (a / 255.0) * t + dest[di].red() as f32 * inv).round() as u8;
                let g = (148.0 * (a / 255.0) * t + dest[di].green() as f32 * inv).round() as u8;
                let b = (48.0 * (a / 255.0) * t + dest[di].blue() as f32 * inv).round() as u8;
                if let Some(p) = PremultipliedColorU8::from_rgba(r, g, b, da) {
                    dest[di] = p;
                }
            }
        }
    }
}

pub(crate) fn draw_stick_live(
    px: &mut Pixmap,
    art: &Pixmap,
    dx: f32,
    dy: f32,
    dw: f32,
    dh: f32,
    cx: f32,
    cy: f32,
    lx: f32,
    ly: f32,
    click: bool,
    well: Color,
    cream: Color,
    well_r: f32,
) {
    let mag = (lx * lx + ly * ly).sqrt();
    if mag < 0.08 && !click {
        return;
    }
    let iw = art.width() as f32;
    let ih = art.height() as f32;
    let cu = (cx - dx) / dw;
    let cv = (cy - dy) / dh;
    fill_circle(px, cx, cy, well_r / ih * dh, well);
    shade_art(
        px,
        art,
        dx,
        dy,
        dw,
        dh,
        [cu - 0.08, cv - 0.12, 0.16, 0.24],
        0.0,
        ShadeMode::Stroke,
        accent(),
        |u, v| {
            let ddx = (u - cu) * iw;
            let ddy = (v - cv) * ih;
            let d = (ddx * ddx + ddy * ddy).sqrt();
            d >= (well_r - 20.0).max(1.0) && d <= well_r
        },
    );
    let cap_r = 0.0469 * dh;
    let travel = 0.048 * dh;
    shade_disc(px, art, dx, dy, dw, dh, cu, cv, 48.0, ShadeMode::All, well);
    let (ox, oy) = if mag < 0.08 {
        (0.0, 0.0)
    } else {
        (lx.clamp(-1.0, 1.0) * travel, ly.clamp(-1.0, 1.0) * travel)
    };
    fill_circle(
        px,
        cx + ox,
        cy + oy,
        cap_r,
        if click { accent() } else { Color::from_rgba8(14, 14, 16, 255) },
    );
    if let Some(mut pb) = Some(PathBuilder::new()) {
        pb.push_circle(cx + ox, cy + oy, cap_r);
        if let Some(path) = pb.finish() {
            stroke_path(px, &path, cream, 1.0);
        }
    }
}
