"""Xbox Series pad — charcoal blueprint matching gamepad-ds4.png.

Silhouette from the dark body in gamepad-xbox-ref.png (blue bg + glow ignored).
Merge direction: one connected outline including detached horns; LT/RT labels in horns.
Layout constants are copied into overlay/hud/src/render.rs xbox_gamepad_layout().
"""

from __future__ import annotations

import math
from pathlib import Path

import numpy as np
from PIL import Image, ImageChops, ImageDraw, ImageFilter, ImageFont

W, H = 1536, 1024
S = 4
BODY_EDGE_BLUR = 0.75  # soften traced silhouette jaggies before supersample
FILL = (25, 27, 29, 255)
STROKE = (122, 122, 124, 255)
LABEL = (136, 136, 138, 255)
WELL = (22, 22, 24, 255)

ART = (93, 49, 1441, 976)
REF = Path(__file__).with_name("gamepad-xbox-ref.png")
GUIDE_TWIN = Path(__file__).with_name("gamepad-xbox-twin.png")
# Dark controller silhouette on blue bg — glow/fringe ignored.
REF_BODY_THRESH = 95
REF_BODY_TOP = 125
REF_EXTENT = (17, 40, 507, 451)

# Normalized to REF_EXTENT — measured from gamepad-xbox-ref.png.
N_LT = (0.267, 0.092, 0.110, 0.159)
N_RT = (0.731, 0.092, 0.114, 0.161)
# Fallback shoulder quads if mask sampling fails.
LB_SHELF_FALLBACK = ((305, 289), (579, 256), (579, 292), (305, 325))
RB_SHELF_FALLBACK = ((952, 256), (1226, 287), (1226, 323), (952, 292))
SHELF_THICK = 62
TOP_LIFT = 20  # pull bumper lip up to meet the body shoulder outline
SHELF_OUTER_DROP = 8  # outer edge drops slightly — matches the almost-flat green ref line
BUMPER_CAP_R = 10  # outer bottom bumper corner only (not the shoulder lip)
BUMPER_INNER_R = 10  # inner bumper corners
# Horn art bboxes in 1536×1024 — full detached LT/RT silhouette.
HORN_LT = (248, 47, 303, 183)
HORN_RT = (948, 47, 353, 183)
N_LS = (0.247, 0.443)
N_RS = (0.630, 0.624)
N_DPAD = (0.395, 0.619)
N_GUIDE = (0.501, 0.311)
N_VIEW = (0.444, 0.400, 0.115, 0.055)
N_MENU = (0.566, 0.400, 0.115, 0.055)
N_FACE_Y = (0.741, 0.335)
N_FACE_X = (0.690, 0.417)
N_FACE_A = (0.741, 0.505)
N_FACE_B = (0.792, 0.417)

LS_R = 92
RS_R = 92
FACE_R = 44
GUIDE_R = 22
DPAD_R = 88
DPAD_ARM = 66
DPAD_BAR = 32
DPAD_HUB = 22


def font(size: int, italic: bool = False) -> ImageFont.FreeTypeFont:
    path = (
        r"C:\Windows\Fonts\segoeuiz.ttf"
        if italic
        else r"C:\Windows\Fonts\segoeuib.ttf"
    )
    try:
        return ImageFont.truetype(path, size)
    except OSError:
        return ImageFont.truetype(r"C:\Windows\Fonts\arialbd.ttf", size)


def smooth_silhouette(mask: Image.Image) -> Image.Image:
    """Remove stair-steps from the traced ref mask — keeps shape, smooths edges."""
    m = mask.filter(ImageFilter.ModeFilter(3))
    if BODY_EDGE_BLUR > 0:
        m = m.filter(ImageFilter.GaussianBlur(BODY_EDGE_BLUR))
    return m


def mask_to_rgba(mask: Image.Image, color: tuple[int, int, int, int]) -> Image.Image:
    """Paint `color` through a soft grayscale mask."""
    a = np.array(mask, dtype=np.float32) / 255.0
    arr = np.zeros((*a.shape, 4), dtype=np.uint8)
    for i, v in enumerate(color):
        arr[..., i] = (v * a).astype(np.uint8)
    arr[..., 3] = np.clip(a * 255.0, 0, 255).astype(np.uint8)
    return Image.fromarray(arr)


def finalize_art(body: Image.Image) -> Image.Image:
    """Downscale supersampled art; keep a thin anti-aliased fringe on edges."""
    out = body.resize((W, H), Image.Resampling.LANCZOS)
    arr = np.array(out)
    a = arr[..., 3]
    arr[a > 220, 3] = 255
    arr[a < 6, 3] = 0
    return Image.fromarray(arr)


def near_color(arr: np.ndarray, rgb: tuple[int, int, int], tol: int = 12) -> np.ndarray:
    r, g, b = arr[..., 0].astype(np.int16), arr[..., 1].astype(np.int16), arr[..., 2].astype(np.int16)
    tr, tg, tb = rgb
    return (np.abs(r - tr) <= tol) & (np.abs(g - tg) <= tol) & (np.abs(b - tb) <= tol)


def guide_logo_from_twin() -> Image.Image:
    """Guide button art copied from gamepad-xbox-twin.png — circle + X only."""
    twin = Image.open(GUIDE_TWIN).convert("RGBA")
    cx, cy = map_norm_point(*N_GUIDE)
    pad = int(GUIDE_R * 1.75)
    crop = twin.crop((cx - pad, cy - pad, cx + pad, cy + pad))
    arr = np.array(crop)
    keep = (arr[..., 3] > 64) & (
        near_color(arr, FILL[:3])
        | near_color(arr, STROKE[:3])
        | near_color(arr, LABEL[:3])
    )
    out = np.zeros_like(arr)
    out[keep] = arr[keep]
    return Image.fromarray(out)


def smooth_logo_layer(logo: Image.Image, scale: int) -> Image.Image:
    """Supersample the logo stamp so arcs stay smooth after downscale."""
    w, h = logo.size
    big = logo.resize((w * scale * 2, h * scale * 2), Image.Resampling.LANCZOS)
    big = big.filter(ImageFilter.GaussianBlur(0.45))
    return big.resize((w * scale, h * scale), Image.Resampling.LANCZOS)


def stamp_guide_logo(body: Image.Image, cx: int, cy: int) -> None:
    logo = guide_logo_from_twin()
    logo_hi = smooth_logo_layer(logo, S)
    lhw, lhh = logo_hi.size
    x = cx * S - lhw // 2
    y = cy * S - lhh // 2
    body.alpha_composite(logo_hi, (x, y))


def outline_mask(mask: Image.Image, width: int) -> Image.Image:
    k = width * 2 + 1
    eroded = mask.filter(ImageFilter.MinFilter(k))
    return ImageChops.subtract(mask, eroded)


def rounded(draw: ImageDraw.ImageDraw, box, r, **kw):
    draw.rounded_rectangle(box, radius=max(1, r), **kw)


def map_norm_box(nx: float, ny: float, nw: float, nh: float) -> tuple[int, int, int, int]:
    ax0, ay0, ax1, ay1 = ART
    aw, ah = ax1 - ax0, ay1 - ay0
    x = ax0 + (nx - nw / 2) * aw
    y = ay0 + (ny - nh / 2) * ah
    return int(round(x)), int(round(y)), int(round(nw * aw)), int(round(nh * ah))


def map_norm_point(nx: float, ny: float) -> tuple[int, int]:
    ax0, ay0, ax1, ay1 = ART
    aw, ah = ax1 - ax0, ay1 - ay0
    return int(round(ax0 + nx * aw)), int(round(ay0 + ny * ah))


def shelf_bbox(pts: tuple[tuple[int, int], ...]) -> tuple[int, int, int, int]:
    xs = [p[0] for p in pts]
    ys = [p[1] for p in pts]
    x0, x1 = min(xs), max(xs)
    y0, y1 = min(ys), max(ys)
    return x0, y0, x1 - x0, y1 - y0


def shelf_center(pts: tuple[tuple[int, int], ...]) -> tuple[int, int]:
    return int(round(sum(p[0] for p in pts) / len(pts))), int(round(sum(p[1] for p in pts) / len(pts)))


def draw_dpad(d: ImageDraw.ImageDraw, px: int, py: int, st: int) -> None:
    """Outline cross in a ring — open center hub, curved arm tips (user ref)."""
    r, arm, bar = DPAD_R, DPAD_ARM, DPAD_BAR
    s = S
    gap = 7
    cr = bar * 0.42
    cap = bar * 0.46
    core = arm - gap - cap
    hub = bar * 1.15

    d.ellipse(
        [(px - r) * s, (py - r) * s, (px + r) * s, (py + r) * s],
        outline=STROKE,
        width=st,
    )
    arms = (
        [(px - bar / 2, py - core, px + bar / 2, py - hub), (0, -core, 180, 360)],
        [(px - bar / 2, py + hub, px + bar / 2, py + core), (0, core, 0, 180)],
        [(px - core, py - bar / 2, px - hub, py + bar / 2), (-core, 0, 90, 270)],
        [(px + hub, py - bar / 2, px + core, py + bar / 2), (core, 0, 270, 90)],
    )
    hw = bar / 2
    for box, (ox, oy, a0, a1) in arms:
        rounded(
            d,
            [box[0] * s, box[1] * s, box[2] * s, box[3] * s],
            cr * s,
            fill=WELL,
            outline=STROKE,
            width=st,
        )
        cx, cy = (px + ox) * s, (py + oy) * s
        d.chord(
            [cx - hw * s, cy - cap * s * 0.72, cx + hw * s, cy + cap * s * 0.72],
            a0,
            a1,
            fill=WELL,
            outline=STROKE,
            width=st,
        )
    # Open hub — punch out the cross intersection so arms do not meet.
    d.ellipse(
        [(px - hub) * s, (py - hub) * s, (px + hub) * s, (py + hub) * s],
        fill=FILL,
        outline=STROKE,
        width=max(4, st - 1),
    )


def smooth_pts(pts: list[tuple[int, int]], window: int = 5) -> list[tuple[int, int]]:
    if len(pts) <= window:
        return pts
    half = window // 2
    out: list[tuple[int, int]] = []
    for i in range(len(pts)):
        chunk = pts[max(0, i - half) : min(len(pts), i + half + 1)]
        out.append(
            (
                int(round(sum(p[0] for p in chunk) / len(chunk))),
                int(round(sum(p[1] for p in chunk) / len(chunk))),
            )
        )
    return out


def arc_pts(cx: float, cy: float, r: float, a0: float, a1: float, steps: int = 10) -> list[tuple[int, int]]:
    return [
        (
            int(round(cx + r * math.cos(math.radians(a0 + (a1 - a0) * i / steps)))),
            int(round(cy + r * math.sin(math.radians(a0 + (a1 - a0) * i / steps)))),
        )
        for i in range(steps + 1)
    ]


def fillet_outer_top(
    top: list[tuple[int, int]],
    thick: int,
    side: str,
    cap_r: int = BUMPER_CAP_R,
    inner_r: int = BUMPER_INNER_R,
) -> tuple[tuple[tuple[int, int], ...], int]:
    """Shoulder band — lip follows body outline; only bumper corners are rounded."""
    top = [(x, y - TOP_LIFT) for x, y in top]
    n = len(top)
    rev = list(reversed(top))
    bot: list[tuple[int, int]] = []
    for i, (x, y) in enumerate(rev):
        u = i / max(1, n - 1)
        bot.append((x, y + thick + int(SHELF_OUTER_DROP * u)))

    if side == "L":
        ox, oy = top[0]
        ix, iy = top[-1]
        ibx, iby = bot[0]
        obx, oby = bot[-1]
        outer_cap = arc_pts(ox + cap_r, oy + cap_r, cap_r, 180, 270, 8)
        inner_top = arc_pts(ix - inner_r, iy + inner_r, inner_r, 270, 360, 6)
        inner_bot = arc_pts(ibx - inner_r, iby - inner_r, inner_r, 0, 90, 6)
        outer_bot = arc_pts(obx + cap_r, oby - cap_r, cap_r, 90, 180, 8)
        top_mid = [p for p in top[1:-1] if p[0] > ox + cap_r - 1]
        bot_mid = [p for p in bot[1:-1] if ox + cap_r < p[0] < ix - inner_r]
        contour = tuple(outer_cap + top_mid + inner_top + inner_bot + bot_mid + outer_bot)
        top_n = len(outer_cap) + len(top_mid) + len(inner_top)
        return contour, top_n

    ox, oy = top[-1]
    ix, iy = top[0]
    ibx, iby = bot[-1]
    obx, oby = bot[0]
    outer_cap = arc_pts(ox - cap_r, oy + cap_r, cap_r, 270, 360, 8)
    inner_top = arc_pts(ix + inner_r, iy + inner_r, inner_r, 180, 270, 6)
    inner_bot = arc_pts(ibx + inner_r, iby - inner_r, inner_r, 90, 180, 6)
    outer_bot = arc_pts(obx - cap_r, oby - cap_r, cap_r, 0, 90, 8)
    top_mid = [p for p in top[1:-1] if p[0] < ox - cap_r + 1]
    bot_mid = [p for p in bot[1:-1] if ix + inner_r < p[0] < ox - cap_r]
    contour = tuple(inner_top + top_mid + outer_cap + outer_bot + bot_mid + inner_bot)
    top_n = len(inner_top) + len(top_mid) + len(outer_cap)
    return contour, top_n


def contour_from_top(
    top: list[tuple[int, int]], thick: int, side: str
) -> tuple[tuple[tuple[int, int], ...], int]:
    """Closed bumper band: curved top lip + parallel bottom, rounded outer caps."""
    return fillet_outer_top(top, thick, side)


def quad_to_contour(
    quad: tuple[tuple[int, int], ...], thick: int, side: str, steps: int = 24
) -> tuple[tuple[tuple[int, int], ...], int]:
    p0, p1, _, _ = quad
    top = [
        (
            int(round(p0[0] + (p1[0] - p0[0]) * i / (steps - 1))),
            int(round(p0[1] + (p1[1] - p0[1]) * i / (steps - 1))),
        )
        for i in range(steps)
    ]
    return contour_from_top(top, thick, side)


def shoulder_shelf_from_mask(
    mask: Image.Image, side: str, thick: int = SHELF_THICK
) -> tuple[tuple[tuple[int, int], ...], int]:
    """Slanted bumper band hugging the curved body shoulder lip under LT/RT."""
    ax0, ay0, ax1, ay1 = ART
    crop = np.array(mask)[ay0:ay1, ax0:ax1] > 128
    h, w = crop.shape
    if side == "L":
        xs = range(int(w * 0.155), int(w * 0.365), 2)
    else:
        xs = range(int(w * 0.635), int(w * 0.845), 2)
    y0, y1 = int(h * 0.17), int(h * 0.33)
    tops: list[tuple[int, int]] = []
    for x in xs:
        col = crop[y0:y1, x]
        idx = np.where(col)[0]
        if idx.size:
            tops.append((ax0 + x, ay0 + y0 + int(idx[0])))
    if len(tops) < 8:
        fb = LB_SHELF_FALLBACK if side == "L" else RB_SHELF_FALLBACK
        return quad_to_contour(fb, thick, side)
    lip_y = min(p[1] for p in tops) + 28
    lip = [p for p in tops if p[1] >= lip_y]
    if len(lip) < 8:
        lip = tops[3:-3]
    top = smooth_pts(lip, window=7)
    return contour_from_top(top, thick, side)


def horn_label_center(mask: Image.Image, box: tuple[int, int, int, int]) -> tuple[int, int]:
    """Center LT/RT labels on the horn silhouette, not the axis-aligned bbox."""
    x, y, w, h = box
    sub = np.array(mask)[y : y + h, x : x + w] > 128
    ys, xs = np.where(sub)
    if len(xs) < 8:
        return x + w // 2, y + int(h * 0.48)
    return int(round(x + xs.mean())), int(round(y + ys.mean()))


def paint_horn_wells(body: Image.Image, mask: Image.Image) -> None:
    """LT/RT horns get WELL fill so analog press can flood the whole wing."""
    arr = np.array(body)
    m = np.array(mask.resize(body.size, Image.Resampling.NEAREST))
    for x, y, w, h in (HORN_LT, HORN_RT):
        x1, y1 = min(x + w, arr.shape[1]), min(y + h, arr.shape[0])
        sub_m = m[y:y1, x:x1] > 128
        sub = arr[y:y1, x:x1]
        lum = sub[..., :3].max(axis=2)
        core = sub_m & (lum < 96)
        sub[core] = WELL
        arr[y:y1, x:x1] = sub
    body.paste(Image.fromarray(arr))


def body_mask_from_ref(ref: Image.Image) -> Image.Image:
    """Opaque merge silhouette: dark body + detached horns. Blue bg and glow stay out."""
    arr = np.array(ref.convert("RGBA"))
    r, g, b, a = arr[..., 0], arr[..., 1], arr[..., 2], arr[..., 3]
    bg = (
        (a > 128)
        & (b.astype(np.int16) > r.astype(np.int16) + 15)
        & (b > g + 8)
        & (b > 70)
    )
    visible = (a > 128) & ~bg
    mx = np.maximum(np.maximum(r, g), b)
    core = visible & (mx < REF_BODY_THRESH)
    mask = Image.fromarray((core.astype(np.uint8) * 255))
    mask = mask.filter(ImageFilter.MaxFilter(5))
    rx0, ry0, rx1, ry1 = REF_EXTENT
    crop = mask.crop((rx0, ry0, rx1 + 1, ry1 + 1))
    ax0, ay0, ax1, ay1 = ART
    aw, ah = ax1 - ax0, ay1 - ay0
    crop = crop.resize((aw, ah), Image.Resampling.LANCZOS)
    placed = Image.new("L", (W, H), 0)
    placed.paste(crop, (ax0, ay0))
    return smooth_silhouette(placed)


def layout_constants(
    lb_shelf: tuple[tuple[int, int], ...],
    lb_top: int,
    rb_shelf: tuple[tuple[int, int], ...],
    rb_top: int,
) -> dict:
    lbb = shelf_bbox(lb_shelf)
    rbb = shelf_bbox(rb_shelf)
    return {
        "LT": HORN_LT,
        "RT": HORN_RT,
        "LB": lbb,
        "RB": rbb,
        "LB_SHELF": lb_shelf,
        "LB_SHELF_TOP": lb_top,
        "RB_SHELF": rb_shelf,
        "RB_SHELF_TOP": rb_top,
        "LS": map_norm_point(*N_LS),
        "RS": map_norm_point(*N_RS),
        "FACE_Y": map_norm_point(*N_FACE_Y),
        "FACE_X": map_norm_point(*N_FACE_X),
        "FACE_A": map_norm_point(*N_FACE_A),
        "FACE_B": map_norm_point(*N_FACE_B),
        "LS_R": LS_R,
        "RS_R": RS_R,
        "FACE_R": FACE_R,
        "DPAD": map_norm_point(*N_DPAD),
        "DPAD_R": DPAD_R,
        "DPAD_ARM": DPAD_ARM,
        "GUIDE": map_norm_point(*N_GUIDE),
        "GUIDE_R": GUIDE_R,
        "VIEW": map_norm_box(*N_VIEW),
        "MENU": map_norm_box(*N_MENU),
    }


def main() -> None:
    ref = Image.open(REF)
    mask = body_mask_from_ref(ref)
    mask_bin = mask.point(lambda v: 255 if v > 128 else 0)
    lb_shelf, lb_top = shoulder_shelf_from_mask(mask_bin, "L")
    rb_shelf, rb_top = shoulder_shelf_from_mask(mask_bin, "R")
    layout = layout_constants(lb_shelf, lb_top, rb_shelf, rb_top)
    sw, sh = W * S, H * S
    mask_hi = mask.resize((sw, sh), Image.Resampling.LANCZOS)

    body = mask_to_rgba(mask_hi, FILL)
    edge_mask = mask_hi.point(lambda v: 255 if v > 120 else 0)
    edge = outline_mask(edge_mask, max(2, 6))
    stroke = Image.new("RGBA", (sw, sh), (0, 0, 0, 0))
    stroke.paste(STROKE, mask=edge)
    body = Image.alpha_composite(body, stroke)
    paint_horn_wells(body, mask_bin.resize((sw, sh), Image.Resampling.LANCZOS))

    d = ImageDraw.Draw(body)
    st = max(5, int(6))

    def ring(cx, cy, r, width=st):
        d.ellipse(
            [(cx - r) * S, (cy - r) * S, (cx + r) * S, (cy + r) * S],
            outline=STROKE,
            width=width,
        )

    def fill_circle(cx, cy, r, color):
        d.ellipse(
            [(cx - r) * S, (cy - r) * S, (cx + r) * S, (cy + r) * S],
            fill=color,
        )

    def label(text, cx, cy, size, italic=False):
        f = font(size * S, italic=italic)
        x0, y0, x1, y1 = f.getbbox(text)
        d.text(
            (cx * S - (x1 - x0) / 2, cy * S - (y1 - y0) / 2 - y0),
            text,
            font=f,
            fill=LABEL,
        )

    def pill(box, text):
        x, y, w, h = box
        rounded(
            d,
            [x * S, y * S, (x + w) * S, (y + h) * S],
            h * S * 0.45,
            outline=STROKE,
            width=st,
        )
        label(text, x + w / 2, y + h / 2, 13)

    def trigger(box, text):
        """DS4-style LT/RT tombstone — same family as L2/R2 on gamepad-ds4.png."""
        x, y, w, h = box
        rounded(
            d,
            [x * S, y * S, (x + w) * S, (y + h) * S],
            46 * S,
            fill=FILL,
            outline=STROKE,
            width=st,
        )
        lip_y = y + h * 0.78
        d.arc(
            [(x + 18) * S, (lip_y - 22) * S, (x + w - 18) * S, (lip_y + 22) * S],
            10,
            170,
            fill=STROKE,
            width=st,
        )
        label(text, x + w / 2, y + h * 0.38, 36, italic=True)

    def bumper_shelf(pts: tuple[tuple[int, int], ...], top_n: int, text: str):
        hi = [c for p in pts for c in (p[0] * S, p[1] * S)]
        d.polygon(hi, fill=WELL)
        outline = [(p[0] * S, p[1] * S) for p in pts] + [(pts[0][0] * S, pts[0][1] * S)]
        d.line(outline, fill=STROKE, width=st, joint="curve")
        cx, cy = shelf_center(pts)
        label(text, cx, cy, 15)

    LT, RT = layout["LT"], layout["RT"]
    LS, RS = layout["LS"], layout["RS"]
    for box, text in ((LT, "LT"), (RT, "RT")):
        lx, ly = horn_label_center(mask, box)
        label(text, lx, ly, 36, italic=True)
    bumper_shelf(layout["LB_SHELF"], layout["LB_SHELF_TOP"], "LB")
    bumper_shelf(layout["RB_SHELF"], layout["RB_SHELF_TOP"], "RB")

    for cx, cy, r in ((LS[0], LS[1], layout["LS_R"]), (RS[0], RS[1], layout["RS_R"])):
        fill_circle(cx, cy, r, WELL)
        ring(cx, cy, r)
        ring(cx, cy, int(r * 0.46), width=max(4, st - 1))

    for key, glyph in (("FACE_Y", "Y"), ("FACE_B", "B"), ("FACE_A", "A"), ("FACE_X", "X")):
        cx, cy = layout[key]
        fill_circle(cx, cy, layout["FACE_R"], FILL)
        ring(cx, cy, layout["FACE_R"])
        label(glyph, cx, cy + 1, 28)

    px, py = layout["DPAD"]
    draw_dpad(d, px, py, st)

    pill(layout["VIEW"], "VIEW")
    pill(layout["MENU"], "MENU")
    gx, gy = layout["GUIDE"]
    stamp_guide_logo(body, gx, gy)

    out = finalize_art(body)
    dest = Path(__file__).with_name("gamepad-xbox.png")
    out.save(dest, "PNG")
    print("wrote", dest)
    print("layout", layout)


if __name__ == "__main__":
    main()
