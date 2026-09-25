#![allow(unused_imports)]
use super::*;

pub(crate) struct GamepadLayout {
    ls: [f32; 2],
    rs: [f32; 2],
    well_r: f32,
    /// Radius in art pixels of the dark skin's drawn well ring.
    well_stroke_r: f32,
    /// Stick cap radius in art pixels.
    cap_r: f32,
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
    /// Per-direction arm UV rects `[u, v, w, h]` (Xbox cross). `None` = DS4 flood fill.
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
        well_stroke_r: 100.0,
        cap_r: 48.0,
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

/// Both DualShock skins share one drawing, so one layout serves Light and Dark.
pub(crate) fn gamepad_layout(sony: bool) -> GamepadLayout {
    if sony {
        ds4_gamepad_layout()
    } else {
        xbox_gamepad_layout()
    }
}

/// `overlay/hud/assets/gamepad-xbox.png` (1344×1024). Layout from gen_gamepad_xbox.py,
/// traced 1:1 from `gamepad-xbox-target.png`.
pub(crate) fn xbox_gamepad_layout() -> GamepadLayout {
    GamepadLayout {
        ls: [338.7 / 1344.0, 397.2 / 1024.0],
        rs: [843.7 / 1344.0, 598.8 / 1024.0],
        well_r: 104.0,
        well_stroke_r: 103.0,
        cap_r: 69.4,
        l2: [275.8 / 1344.0, 7.0 / 1024.0, 161.7 / 1344.0, 113.8 / 1024.0],
        r2: [908.5 / 1344.0, 7.0 / 1024.0, 161.7 / 1344.0, 113.8 / 1024.0],
        l1: [
            207.9 / 1344.0,
            120.8 / 1024.0,
            271.5 / 1344.0,
            131.8 / 1024.0,
        ],
        r1: [
            864.6 / 1344.0,
            120.8 / 1024.0,
            271.5 / 1344.0,
            131.8 / 1024.0,
        ],
        l2_seed: [356.6 / 1344.0, 63.9 / 1024.0],
        r2_seed: [989.4 / 1344.0, 63.9 / 1024.0],
        l1_seed: [330.7 / 1344.0, 167.7 / 1024.0],
        r1_seed: [1013.3 / 1344.0, 167.7 / 1024.0],
        face: [
            [1007.3 / 1344.0, 307.4 / 1024.0],
            [1095.2 / 1344.0, 397.2 / 1024.0],
            [1007.3 / 1344.0, 485.1 / 1024.0],
            [919.5 / 1344.0, 397.2 / 1024.0],
        ],
        face_r: 50.0,
        dpad: [
            [500.3 / 1344.0, 561.3 / 1024.0],
            [549.8 / 1344.0, 610.8 / 1024.0],
            [500.3 / 1344.0, 660.3 / 1024.0],
            [450.8 / 1344.0, 610.8 / 1024.0],
        ],
        dpad_half: [99.0 / 1344.0, 99.0 / 1024.0],
        dpad_seg: Some([
            [466.8 / 1344.0, 511.8 / 1024.0, 67.0 / 1344.0, 99.0 / 1024.0],
            [500.3 / 1344.0, 577.3 / 1024.0, 99.0 / 1344.0, 67.0 / 1024.0],
            [466.8 / 1344.0, 610.8 / 1024.0, 67.0 / 1344.0, 99.0 / 1024.0],
            [401.3 / 1344.0, 577.3 / 1024.0, 99.0 / 1344.0, 67.0 / 1024.0],
        ]),
        back: [542.2 / 1344.0, 363.2 / 1024.0, 72.0 / 1344.0, 72.0 / 1024.0],
        start: [731.8 / 1344.0, 363.2 / 1024.0, 72.0 / 1344.0, 72.0 / 1024.0],
        guide: [672.0 / 1344.0, 251.5 / 1024.0],
        guide_r: 57.4,
        touch: None,
    }
}

pub(crate) fn draw_gamepad(px: &mut Pixmap, fonts: &Fonts, cfg: &HudConfig, sw: f32, sh: f32) {
    // THESIS: a live DualShock over the dirt. Primary is the press. Sticks leave their wells. Triggers fill with squeeze. Bumpers light when held.
    // OWN-WORLD: Broadcast Booth Glass — DualShock drawing, no night-ink plaque by default, Look primary live, Exo 2 ExtraBold Italic labels.
    // STORY: glance which inputs are live without looking at the pad. No pad is a small No controller pill.
    // FIRST VIEWPORT: DualShock 4 keeps its proportions. L2/R2 are trigger wings on the silhouette (analog fill from the bottom). L1/R1 are DualShock shoulder bars. Cross/A primary. Left stick translated.
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
            fill_night_pill(
                px,
                x + (w - tw) * 0.5 - 8.0,
                y + (h - fs) * 0.48 - 4.0,
                tw + 16.0,
                fs + 8.0,
            );
        }
        text(
            px,
            fonts,
            label,
            fs,
            x + w * 0.5,
            y + (h - fs) * 0.48,
            text_col(),
            true,
        );
        return;
    }

    let sony = cfg.gamepad_style.sony_art(pad.kind);
    let filled = cfg.gamepad_theme.filled();
    let well = Color::from_rgba8(22, 22, 24, 255);
    let cream = Color::from_rgba8(248, 248, 252, 255);
    let Some(art) = gamepad_art(sony, filled) else {
        return;
    };
    let layout = gamepad_layout(sony);
    // Every skin is pressed in art space before it is scaled, so fills get the same
    // area-averaged edges as the idle outlines.
    let Some((dx, dy, dw, dh)) =
        blit_gamepad_art(px, art, sony, filled, x, y, w, h, Some((pad, &layout)))
    else {
        return;
    };

    let (lsx, lsy) = (dx + layout.ls[0] * dw, dy + layout.ls[1] * dh);
    let (rsx, rsy) = (dx + layout.rs[0] * dw, dy + layout.rs[1] * dh);
    let well_r = layout.well_r / art.height() as f32 * dh;
    plug_stick_well(px, lsx, lsy, well_r, well);
    plug_stick_well(px, rsx, rsy, well_r, well);

    draw_stick_live(
        px,
        art,
        dx,
        dy,
        dw,
        dh,
        lsx,
        lsy,
        pad.lx,
        pad.ly,
        pad.down(crate::gamepad::LS),
        well,
        cream,
        &layout,
        filled,
    );
    draw_stick_live(
        px,
        art,
        dx,
        dy,
        dw,
        dh,
        rsx,
        rsy,
        pad.rx,
        pad.ry,
        pad.down(crate::gamepad::RS),
        well,
        cream,
        &layout,
        filled,
    );
}

pub(crate) fn gamepad_art(sony: bool, filled: bool) -> Option<&'static Pixmap> {
    static DS4_DARK: OnceLock<Option<Pixmap>> = OnceLock::new();
    static DS4_LIGHT: OnceLock<Option<Pixmap>> = OnceLock::new();
    static XB_LIGHT: OnceLock<Option<Pixmap>> = OnceLock::new();
    static XB_DARK: OnceLock<Option<Pixmap>> = OnceLock::new();
    let slot = match (sony, filled) {
        (true, false) => &DS4_DARK,
        (true, true) => &DS4_LIGHT,
        (false, true) => &XB_LIGHT,
        (false, false) => &XB_DARK,
    };
    slot.get_or_init(|| {
        Pixmap::decode_png(match (sony, filled) {
            (true, false) => include_bytes!("../../assets/gamepad-ds4-dark.png").as_slice(),
            (true, true) => include_bytes!("../../assets/gamepad-ds4-light.png").as_slice(),
            (false, true) => include_bytes!("../../assets/gamepad-xbox-light.png").as_slice(),
            (false, false) => include_bytes!("../../assets/gamepad-xbox-dark.png").as_slice(),
        })
        .ok()
    })
    .as_ref()
}

pub(crate) fn blit_gamepad_art(
    px: &mut Pixmap,
    src: &Pixmap,
    sony: bool,
    filled: bool,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    art_press: Option<(crate::gamepad::PadState, &GamepadLayout)>,
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
    let Some(mut hi) = scaled_gamepad_art(src, sony, filled, tw, th) else {
        return None;
    };
    if let Some((pad, layout)) = art_press {
        hi = pad_pressed_frame(src, hi, sony, filled, pad, layout, dh);
    }
    let mut paint = PixmapPaint::default();
    paint.quality = FilterQuality::Bicubic;
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

pub(crate) fn scaled_gamepad_art(
    src: &Pixmap,
    sony: bool,
    filled: bool,
    tw: u32,
    th: u32,
) -> Option<Pixmap> {
    thread_local! {
        static CACHE: RefCell<Option<(bool, bool, u32, u32, Pixmap)>> = const { RefCell::new(None) };
    }
    CACHE.with(|slot| {
        if let Some((s, f, w, h, px)) = slot.borrow().as_ref() {
            if *s == sony && *f == filled && *w == tw && *h == th {
                return Some(px.clone());
            }
        }
        // Pressed skins re-downscale dirty regions with the area average, so their idle frame
        // must use it too or the pressed regions would seam.
        let lo = if tw <= src.width() && th <= src.height() {
            area_downscale(src, tw, th)?
        } else {
            supersampled_resample(src, tw, th)?
        };
        *slot.borrow_mut() = Some((sony, filled, tw, th, lo.clone()));
        Some(lo)
    })
}

fn supersampled_resample(src: &Pixmap, tw: u32, th: u32) -> Option<Pixmap> {
    let sw = tw.saturating_mul(4).max(4);
    let sh = th.saturating_mul(4).max(4);
    let mut hi = Pixmap::new(sw, sh)?;
    let mut paint = PixmapPaint::default();
    paint.quality = FilterQuality::Bilinear;
    hi.draw_pixmap(
        0,
        0,
        src.as_ref(),
        &paint,
        Transform::from_scale(sw as f32 / src.width() as f32, sh as f32 / src.height() as f32),
        None,
    );
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
    Some(lo)
}

/// Per-destination-pixel source coverage `(first, weights)` for a 1-D box downscale.
fn box_taps(src_len: u32, dst_len: u32) -> Vec<(usize, Vec<f32>)> {
    let scale = src_len as f32 / dst_len as f32;
    (0..dst_len)
        .map(|i| {
            let lo = i as f32 * scale;
            let hi = (lo + scale).min(src_len as f32);
            let first = lo.floor() as usize;
            let last = (hi.ceil() as usize).min(src_len as usize);
            let weights = (first..last)
                .map(|j| ((j + 1) as f32).min(hi) - (j as f32).max(lo))
                .map(|coverage| coverage / scale)
                .collect();
            (first, weights)
        })
        .collect()
}

/// Area-average downscale on premultiplied pixels. Thin 1px strokes stay continuous
/// instead of aliasing into dashes the way a single resample does.
pub(crate) fn area_downscale(src: &Pixmap, tw: u32, th: u32) -> Option<Pixmap> {
    let mut out = Pixmap::new(tw, th)?;
    area_downscale_into(src, &mut out, [0, 0, tw, th]);
    Some(out)
}

/// Recompute only `rect` (`[x0, y0, x1, y1)` in `dst` pixels) of an area-average downscale
/// of `src` to `dst`'s size.
pub(crate) fn area_downscale_into(src: &Pixmap, dst: &mut Pixmap, rect: [u32; 4]) {
    let (tw, th) = (dst.width(), dst.height());
    let [x0, y0, x1, y1] = [rect[0], rect[1], rect[2].min(tw), rect[3].min(th)];
    if x0 >= x1 || y0 >= y1 || tw > src.width() || th > src.height() {
        return;
    }
    let src_w = src.width() as usize;
    let dst_w = tw as usize;
    let cols = &box_taps(src.width(), tw)[x0 as usize..x1 as usize];
    let rows = &box_taps(src.height(), th)[y0 as usize..y1 as usize];
    let row_lo = rows[0].0;
    let row_hi = rows.last().map_or(row_lo, |(first, weights)| first + weights.len());
    let span_w = cols.len();
    let pixels = src.pixels();
    let mut horizontal = vec![[0.0f32; 4]; (row_hi - row_lo) * span_w];
    for y in row_lo..row_hi {
        let row = &pixels[y * src_w..(y + 1) * src_w];
        for (x, (first, weights)) in cols.iter().enumerate() {
            let mut sum = [0.0f32; 4];
            for (k, weight) in weights.iter().enumerate() {
                let p = row[first + k];
                sum[0] += p.red() as f32 * weight;
                sum[1] += p.green() as f32 * weight;
                sum[2] += p.blue() as f32 * weight;
                sum[3] += p.alpha() as f32 * weight;
            }
            horizontal[(y - row_lo) * span_w + x] = sum;
        }
    }
    let dest = dst.pixels_mut();
    for (ry, (first, weights)) in rows.iter().enumerate() {
        let y = y0 as usize + ry;
        for rx in 0..span_w {
            let x = x0 as usize + rx;
            let mut sum = [0.0f32; 4];
            for (k, weight) in weights.iter().enumerate() {
                let p = horizontal[(first + k - row_lo) * span_w + rx];
                for channel in 0..4 {
                    sum[channel] += p[channel] * weight;
                }
            }
            let alpha = sum[3].round().clamp(0.0, 255.0) as u8;
            let channel = |v: f32| (v.round().clamp(0.0, 255.0) as u8).min(alpha);
            if let Some(p) = PremultipliedColorU8::from_rgba(
                channel(sum[0]),
                channel(sum[1]),
                channel(sum[2]),
                alpha,
            ) {
                dest[y * dst_w + x] = p;
            }
        }
    }
}

/// How a skin's art mixes its ink with the lighter marks drawn over it.
#[derive(Clone, Copy)]
struct PressPalette {
    ink: [f32; 3],
    /// Coverage ramp: 0 at `lo` (pure ink), 1 at `hi` (a full stroke or paper).
    lo: f32,
    hi: f32,
    /// Ramp on the max channel instead of luminance, so saturated letters count as marks.
    max_channel: bool,
}

impl PressPalette {
    fn coverage(&self, rgb: [f32; 3]) -> f32 {
        let level = if self.max_channel {
            rgb[0].max(rgb[1]).max(rgb[2])
        } else {
            rgb[0] * 0.30 + rgb[1] * 0.59 + rgb[2] * 0.11
        };
        ((level - self.lo) / (self.hi - self.lo)).clamp(0.0, 1.0)
    }
}

const DS4_PALETTE: PressPalette = PressPalette {
    ink: [29.0, 31.0, 34.0],
    lo: 34.0,
    hi: 118.0,
    max_channel: false,
};
const XBOX_PALETTE: PressPalette = PressPalette {
    ink: [10.0, 10.0, 10.0],
    lo: 10.0,
    hi: 200.0,
    max_channel: true,
};
/// Drawn with the DualShock dark body and stroke colors.
const XBOX_DARK_PALETTE: PressPalette = DS4_PALETTE;
/// Light DualShock: slate controls and black panels under cream outlines and marks.
const DS4_LIGHT_PALETTE: PressPalette = PressPalette {
    ink: [58.0, 62.0, 72.0],
    lo: 62.0,
    hi: 200.0,
    max_channel: false,
};
/// Light DualShock floods stop at its black panels, which sit below this luminance.
const DS4_LIGHT_FLOOD_MIN_LUM: u8 = 30;
const PRESS_CREAM: [f32; 3] = [248.0, 248.0, 252.0];
const PRESS_BITS: u32 = crate::gamepad::LB
    | crate::gamepad::RB
    | crate::gamepad::BACK
    | crate::gamepad::START
    | crate::gamepad::TOUCH
    | crate::gamepad::GUIDE
    | crate::gamepad::UP
    | crate::gamepad::RIGHT
    | crate::gamepad::DOWN
    | crate::gamepad::LEFT
    | crate::gamepad::NORTH
    | crate::gamepad::EAST
    | crate::gamepad::SOUTH
    | crate::gamepad::WEST;

#[derive(Clone, Copy)]
enum PressFill {
    /// Ink turns accent; outlines, letters and paper keep their drawn color.
    Keep,
    /// Ink turns accent; glyph strokes read as cream on it (DualShock face symbols).
    Cream,
    /// Paper turns accent; the ink mark on it stays (Xbox guide: light disc, dark X).
    Paper,
}

#[derive(Clone, Copy, PartialEq)]
struct PressKey {
    sony: bool,
    filled: bool,
    buttons: u32,
    lt: u8,
    rt: u8,
    accent: [u8; 3],
    size: (u32, u32),
}

struct PressState {
    /// Full-resolution art with this frame's presses applied.
    pressed: Pixmap,
    /// Art rects in `pressed` that differ from the original art.
    dirty: Vec<[u32; 4]>,
    key: PressKey,
    frame: Pixmap,
}

fn analog_key(v: f32) -> u8 {
    if v < 0.03 {
        0
    } else {
        (v.clamp(0.0, 1.0) * 255.0).round().max(1.0) as u8
    }
}

/// The pad with presses composited in art space, then area-averaged down over only the
/// changed regions of the cached idle frame.
pub(crate) fn pad_pressed_frame(
    art: &Pixmap,
    idle: Pixmap,
    sony: bool,
    filled: bool,
    pad: crate::gamepad::PadState,
    layout: &GamepadLayout,
    dh: f32,
) -> Pixmap {
    thread_local! {
        static STATE: RefCell<Option<PressState>> = const { RefCell::new(None) };
    }
    let key = PressKey {
        sony,
        filled,
        buttons: pad.buttons & PRESS_BITS,
        lt: analog_key(pad.lt),
        rt: analog_key(pad.rt),
        accent: accent_rgb(),
        size: (idle.width(), idle.height()),
    };
    let idle_key = PressKey {
        buttons: 0,
        lt: 0,
        rt: 0,
        ..key
    };
    STATE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let stale = slot.as_ref().map_or(true, |state| {
            state.key.sony != key.sony
                || state.key.filled != key.filled
                || state.key.size != key.size
                || state.pressed.width() != art.width()
        });
        if stale {
            if key == idle_key {
                *slot = None;
                return idle;
            }
            *slot = Some(PressState {
                pressed: art.clone(),
                dirty: Vec::new(),
                key: idle_key,
                frame: idle.clone(),
            });
        }
        let Some(state) = slot.as_mut() else {
            return idle;
        };
        if state.key == key {
            return state.frame.clone();
        }
        let previous = std::mem::take(&mut state.dirty);
        for &rect in &previous {
            copy_art_rect(art, &mut state.pressed, rect);
        }
        state.dirty = apply_presses(art, &mut state.pressed, sony, filled, pad, layout, dh);
        let (tw, th) = key.size;
        let (sx, sy) = (tw as f32 / art.width() as f32, th as f32 / art.height() as f32);
        for rect in previous.iter().chain(state.dirty.iter()) {
            let dest = [
                (rect[0] as f32 * sx).floor() as u32,
                (rect[1] as f32 * sy).floor() as u32,
                ((rect[2] as f32 * sx).ceil() as u32 + 1).min(tw),
                ((rect[3] as f32 * sy).ceil() as u32 + 1).min(th),
            ];
            area_downscale_into(&state.pressed, &mut state.frame, dest);
        }
        state.key = key;
        state.frame.clone()
    })
}

fn copy_art_rect(art: &Pixmap, dst: &mut Pixmap, rect: [u32; 4]) {
    let w = art.width() as usize;
    let src = art.pixels();
    let dest = dst.pixels_mut();
    for y in rect[1] as usize..rect[3] as usize {
        let row = y * w;
        dest[row + rect[0] as usize..row + rect[2] as usize]
            .copy_from_slice(&src[row + rect[0] as usize..row + rect[2] as usize]);
    }
}

fn apply_presses(
    art: &Pixmap,
    pressed: &mut Pixmap,
    sony: bool,
    filled: bool,
    pad: crate::gamepad::PadState,
    layout: &GamepadLayout,
    dh: f32,
) -> Vec<[u32; 4]> {
    let xbox_dark = !sony && !filled;
    let palette = match (sony, filled) {
        (true, true) => DS4_LIGHT_PALETTE,
        (true, false) => DS4_PALETTE,
        (false, true) => XBOX_PALETTE,
        (false, false) => XBOX_DARK_PALETTE,
    };
    let flood_min_lum = if sony && filled {
        DS4_LIGHT_FLOOD_MIN_LUM
    } else {
        0
    };
    let accent = accent_rgb().map(f32::from);
    let fade = (art.height() as f32 / dh.max(1.0)).max(1.0);
    let (iw, ih) = (art.width() as f32, art.height() as f32);
    let art_rect = |uv: [f32; 4]| {
        [
            (uv[0] * iw).floor().max(0.0) as u32,
            (uv[1] * ih).floor().max(0.0) as u32,
            ((uv[0] + uv[2]) * iw).ceil().min(iw) as u32,
            ((uv[1] + uv[3]) * ih).ceil().min(ih) as u32,
        ]
    };
    let mut dirty = Vec::new();
    let mut paint = |rect: [u32; 4], fill: PressFill, weight: &dyn Fn(u32, u32) -> f32| {
        paint_art_region(art, pressed, rect, palette, accent, fill, weight);
        dirty.push(rect);
    };

    let flood = |seed: [f32; 2], uv: [f32; 4], squeeze: f32| {
        flood_mask(art, seed[0], seed[1], uv, squeeze, sony, flood_min_lum)
    };
    let mut floods = Vec::new();
    if pad.lt >= 0.03 {
        floods.extend(flood(layout.l2_seed, layout.l2, pad.lt));
    }
    if pad.rt >= 0.03 {
        floods.extend(flood(layout.r2_seed, layout.r2, pad.rt));
    }
    if pad.down(crate::gamepad::LB) {
        floods.extend(flood(layout.l1_seed, layout.l1, 1.0));
    }
    if pad.down(crate::gamepad::RB) {
        floods.extend(flood(layout.r1_seed, layout.r1, 1.0));
    }
    let dpad_bits = [
        crate::gamepad::UP,
        crate::gamepad::RIGHT,
        crate::gamepad::DOWN,
        crate::gamepad::LEFT,
    ];
    let mut rects = Vec::new();
    let mut dpad_rects = Vec::new();
    let [hu, hv] = layout.dpad_half;
    for (i, &bit) in dpad_bits.iter().enumerate() {
        if pad.down(bit) {
            match layout.dpad_seg {
                Some(segs) => dpad_rects.push(segs[i]),
                None => {
                    let [u, v] = layout.dpad[i];
                    floods.extend(flood([u, v], [u - hu, v - hv, hu * 2.0, hv * 2.0], 1.0));
                }
            }
        }
    }
    // The dark Xbox body is ink too: arm rects stop at the cross outline, and View/Menu fill
    // only inside their rings.
    let dpad_cross = if xbox_dark && !dpad_rects.is_empty() {
        let (center_u, center_v) = (layout.dpad[0][0], layout.dpad[1][1]);
        flood(
            [center_u, center_v],
            [center_u - hu, center_v - hv, hu * 2.0, hv * 2.0],
            1.0,
        )
    } else {
        None
    };
    let mut buttons = Vec::new();
    if pad.down(crate::gamepad::BACK) {
        buttons.push(layout.back);
    }
    if pad.down(crate::gamepad::START) {
        buttons.push(layout.start);
    }
    if xbox_dark {
        for uv in buttons.drain(..) {
            floods.extend(flood([uv[0] + uv[2] * 0.5, uv[1] + uv[3] * 0.5], uv, 1.0));
        }
    }
    for mask in &floods {
        paint([mask.ax0, mask.ay0, mask.ax1, mask.ay1], PressFill::Keep, &|x, y| {
            if !mask.fill[(y - mask.ay0) as usize * mask.bw + (x - mask.ax0) as usize] {
                return 0.0;
            }
            ((y as f32 + 0.5 - mask.cut) / fade).clamp(0.0, 1.0)
        });
    }

    if let Some(touch) = layout.touch {
        if pad.down(crate::gamepad::TOUCH) {
            rects.push(touch);
        }
    }
    rects.extend(buttons);
    for uv in dpad_rects {
        match &dpad_cross {
            Some(cross) => paint(art_rect(uv), PressFill::Keep, &|x, y| {
                if cross.contains(x, y) {
                    1.0
                } else {
                    0.0
                }
            }),
            None => paint(art_rect(uv), PressFill::Keep, &|_, _| 1.0),
        }
    }
    for uv in rects {
        paint(art_rect(uv), PressFill::Keep, &|_, _| 1.0);
    }

    let mut disc = |[u, v]: [f32; 2], r: f32, fill: PressFill| {
        let (cx, cy) = (u * iw, v * ih);
        let side = r * 2.0 + 4.0;
        let rect = art_rect([(cx - r - 2.0) / iw, (cy - r - 2.0) / ih, side / iw, side / ih]);
        paint(rect, fill, &|x, y| {
            let ddx = x as f32 + 0.5 - cx;
            let ddy = y as f32 + 0.5 - cy;
            (r + 0.5 - (ddx * ddx + ddy * ddy).sqrt()).clamp(0.0, 1.0)
        });
    };
    let face_bits = [
        crate::gamepad::NORTH,
        crate::gamepad::EAST,
        crate::gamepad::SOUTH,
        crate::gamepad::WEST,
    ];
    let face_fill = if sony || xbox_dark {
        PressFill::Cream
    } else {
        PressFill::Keep
    };
    for (i, &bit) in face_bits.iter().enumerate() {
        if pad.down(bit) {
            disc(layout.face[i], layout.face_r, face_fill);
        }
    }
    if pad.down(crate::gamepad::GUIDE) {
        let guide_fill = if filled && !sony {
            PressFill::Paper
        } else {
            PressFill::Keep
        };
        disc(layout.guide, layout.guide_r, guide_fill);
    }
    dirty
}

/// Lay accent into the art inside `rect`, weighted per pixel by `weight` (0 keeps the art).
/// Ink/mark coverage comes from the full-resolution art, so edges and glyphs stay anti-aliased.
fn paint_art_region(
    art: &Pixmap,
    pressed: &mut Pixmap,
    rect: [u32; 4],
    palette: PressPalette,
    accent: [f32; 3],
    fill: PressFill,
    weight: &dyn Fn(u32, u32) -> f32,
) {
    let w = art.width();
    let src = art.pixels();
    let dest = pressed.pixels_mut();
    for y in rect[1]..rect[3].min(art.height()) {
        for x in rect[0]..rect[2].min(w) {
            let t = weight(x, y);
            if t <= 0.0 {
                continue;
            }
            let i = (y * w + x) as usize;
            let p = src[i];
            let alpha = p.alpha();
            if alpha < 8 {
                continue;
            }
            let a = alpha as f32 / 255.0;
            let rgb = [p.red() as f32 / a, p.green() as f32 / a, p.blue() as f32 / a];
            let coverage = palette.coverage(rgb);
            let out: [f32; 3] = std::array::from_fn(|c| {
                let lit = match fill {
                    PressFill::Paper => palette.ink[c] * (1.0 - coverage) + accent[c] * coverage,
                    PressFill::Cream => accent[c] * (1.0 - coverage) + PRESS_CREAM[c] * coverage,
                    PressFill::Keep => {
                        let mark = if coverage > 0.0 {
                            ((rgb[c] - palette.ink[c] * (1.0 - coverage)) / coverage)
                                .clamp(0.0, 255.0)
                        } else {
                            0.0
                        };
                        accent[c] * (1.0 - coverage) + mark * coverage
                    }
                };
                (rgb[c] * (1.0 - t) + lit * t) * a
            });
            let channel = |v: f32| (v.round().clamp(0.0, 255.0) as u8).min(alpha);
            if let Some(q) =
                PremultipliedColorU8::from_rgba(channel(out[0]), channel(out[1]), channel(out[2]), alpha)
            {
                dest[i] = q;
            }
        }
    }
}

/// Cover every opaque art pixel in `uv` with `color` (stick cap cover).
/// `y_cut` 0 paints the whole box; analog uses `1 - squeeze`.
pub(crate) fn shade_art(
    px: &mut Pixmap,
    art: &Pixmap,
    dx: f32,
    dy: f32,
    dw: f32,
    dh: f32,
    uv: [f32; 4],
    y_cut: f32,
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
            if src.alpha() >= 24 {
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
        color,
        |u, v| {
            let ddx = (u - cx) * iw;
            let ddy = (v - cy) * ih;
            ddx * ddx + ddy * ddy <= r_px * r_px
        },
    );
}

/// Art-space fill of one control: `fill` over `[ax0, ax1) x [ay0, ay1)` (row stride `bw`),
/// and the art row `cut` the analog squeeze fills up to.
pub(crate) struct FloodMask {
    pub(crate) ax0: u32,
    pub(crate) ay0: u32,
    pub(crate) ax1: u32,
    pub(crate) ay1: u32,
    pub(crate) bw: usize,
    pub(crate) fill: Vec<bool>,
    pub(crate) cut: f32,
}

impl FloodMask {
    fn contains(&self, x: u32, y: u32) -> bool {
        x >= self.ax0
            && y >= self.ay0
            && x < self.ax1
            && y < self.ay1
            && self.fill[(y - self.ay0) as usize * self.bw + (x - self.ax0) as usize]
    }
}

pub(crate) fn flood_mask(
    art: &Pixmap,
    seed_u: f32,
    seed_v: f32,
    uv: [f32; 4],
    squeeze: f32,
    ds4_lip: bool,
    min_lum: u8,
) -> Option<FloodMask> {
    let iw = art.width();
    let ih = art.height();
    if iw == 0 || ih == 0 || uv[2] <= 0.0 || uv[3] <= 0.0 {
        return None;
    }
    let ax0 = ((uv[0] * iw as f32).floor() as i32).max(0) as u32;
    let ay0 = ((uv[1] * ih as f32).floor() as i32).max(0) as u32;
    let ax1 = ((uv[0] + uv[2]) * iw as f32).ceil() as u32;
    let ay1 = ((uv[1] + uv[3]) * ih as f32).ceil() as u32;
    let ax1 = ax1.min(iw);
    let ay1 = ay1.min(ih);
    if ax1 <= ax0 || ay1 <= ay0 {
        return None;
    }
    let bw = (ax1 - ax0) as usize;
    let bh = (ay1 - ay0) as usize;
    let mut sx = ((seed_u * iw as f32).floor() as i32).clamp(ax0 as i32, ax1 as i32 - 1) as u32;
    let mut sy = ((seed_v * ih as f32).floor() as i32).clamp(ay0 as i32, ay1 as i32 - 1) as u32;
    let fillable = |x: u32, y: u32| -> bool { (min_lum..78).contains(&art_lum_at(art, x, y)) };
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
            None => return None,
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
        return None;
    }
    // A held trigger should read as fully lit; the last fifth of the pull is not
    // worth a black sliver at the top of the tab.
    let squeeze = if squeeze >= 0.80 {
        1.0
    } else {
        squeeze.clamp(0.0, 1.0)
    };
    let span = (ymax.saturating_sub(ymin)).max(1) as f32;
    let cut = ymin as f32 + (1.0 - squeeze) * span;
    Some(FloodMask {
        ax0,
        ay0,
        ax1,
        ay1,
        bw,
        fill,
        cut,
    })
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
    layout: &GamepadLayout,
    filled: bool,
) {
    let mag = (lx * lx + ly * ly).sqrt();
    if mag < 0.08 && !click {
        return;
    }
    let ih = art.height() as f32;
    let well_r = layout.well_r;
    let cu = (cx - dx) / dw;
    let cv = (cy - dy) / dh;
    let ring_r = well_r / ih * dh;
    fill_circle(px, cx, cy, ring_r, well);
    if !filled {
        // The dark drawing's well stroke is ~3 art px wide.
        let mut pb = PathBuilder::new();
        pb.push_circle(cx, cy, layout.well_stroke_r / ih * dh);
        if let Some(path) = pb.finish() {
            stroke_path(px, &path, accent(), (3.0 / ih * dh).max(1.5));
        }
    } else {
        let mut pb = PathBuilder::new();
        pb.push_circle(cx, cy, ring_r);
        if let Some(path) = pb.finish() {
            stroke_path(px, &path, accent(), 2.0);
        }
    }
    let cap_r = layout.cap_r / ih * dh;
    let travel = 0.048 * dh;
    shade_disc(
        px,
        art,
        dx,
        dy,
        dw,
        dh,
        cu,
        cv,
        layout.cap_r,
        well,
    );
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
        if click {
            accent()
        } else {
            Color::from_rgba8(14, 14, 16, 255)
        },
    );
    if let Some(mut pb) = Some(PathBuilder::new()) {
        pb.push_circle(cx + ox, cy + oy, cap_r);
        if let Some(path) = pb.finish() {
            stroke_path(px, &path, cream, 1.0);
        }
    }
}
