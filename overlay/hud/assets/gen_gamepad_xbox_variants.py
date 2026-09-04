"""Three Impeccable Xbox pad comps — Broadcast Booth Glass, ref outline, glow ignored."""

from __future__ import annotations

from pathlib import Path

from gen_gamepad_xbox import draw_dpad, shelf_bbox, shoulder_shelf_from_mask

import numpy as np
from PIL import Image, ImageChops, ImageDraw, ImageFilter, ImageFont

W, H = 1536, 1024
S = 4
FILL = (25, 27, 29, 255)
PLATEAU = (14, 15, 17, 255)
STROKE = (122, 122, 124, 255)
LABEL = (136, 136, 138, 255)
WELL = (22, 22, 24, 255)
ORANGE = (255, 148, 48, 255)

ART = (93, 49, 1441, 976)
REF = Path(__file__).with_name("gamepad-xbox-ref.png")
# Dark controller silhouette on blue bg — glow/fringe ignored.
REF_BODY_THRESH = 95
REF_BODY_TOP = 125  # main body below detached horns
REF_EXTENT = (17, 40, 507, 451)
ROOT = Path(__file__).resolve().parents[3]
DECISION = ROOT / ".impeccable" / "mocks" / "decision"
ASSETS = Path(__file__).parent

# Measured from ref (extent-normalized). Detached horns y≈40–118.
N_LT = (0.267, 0.092, 0.110, 0.159)
N_RT = (0.731, 0.092, 0.114, 0.161)
LB_SHELF_FALLBACK = ((305, 289), (579, 256), (579, 292), (305, 325))
RB_SHELF_FALLBACK = ((952, 256), (1226, 287), (1226, 323), (952, 292))
N_LS = (0.247, 0.443)
N_RS = (0.630, 0.624)
N_DPAD = (0.395, 0.619)
N_GUIDE = (0.501, 0.311)
N_VIEW = (0.444, 0.400, 0.115, 0.055)
N_MENU = (0.566, 0.400, 0.115, 0.055)
N_PLATEAU = (0.501, 0.280, 0.720, 0.140)
N_FACE_Y = (0.741, 0.335)
N_FACE_X = (0.690, 0.417)
N_FACE_A = (0.741, 0.505)
N_FACE_B = (0.792, 0.417)

LS_R, RS_R, FACE_R, GUIDE_R = 78, 78, 44, 22
DPAD_R, DPAD_ARM, DPAD_BAR, DPAD_HUB = 72, 54, 26, 18


def font(size: int, italic: bool = False) -> ImageFont.FreeTypeFont:
    path = r"C:\Windows\Fonts\segoeuiz.ttf" if italic else r"C:\Windows\Fonts\segoeuib.ttf"
    try:
        return ImageFont.truetype(path, size)
    except OSError:
        return ImageFont.truetype(r"C:\Windows\Fonts\arialbd.ttf", size)


def outline_mask(mask: Image.Image, width: int) -> Image.Image:
    k = width * 2 + 1
    return ImageChops.subtract(mask, mask.filter(ImageFilter.MinFilter(k)))


def rounded(draw: ImageDraw.ImageDraw, box, r, **kw):
    draw.rounded_rectangle(box, radius=max(1, r), **kw)


def map_norm_box(nx: float, ny: float, nw: float, nh: float) -> tuple[int, int, int, int]:
    ax0, ay0, ax1, ay1 = ART
    aw, ah = ax1 - ax0, ay1 - ay0
    return (
        int(round(ax0 + (nx - nw / 2) * aw)),
        int(round(ay0 + (ny - nh / 2) * ah)),
        int(round(nw * aw)),
        int(round(nh * ah)),
    )


def map_norm_point(nx: float, ny: float) -> tuple[int, int]:
    ax0, ay0, ax1, ay1 = ART
    aw, ah = ax1 - ax0, ay1 - ay0
    return int(round(ax0 + nx * aw)), int(round(ay0 + ny * ah))


def shelf_center(pts: tuple[tuple[int, int], ...]) -> tuple[int, int]:
    return int(round(sum(p[0] for p in pts) / len(pts))), int(round(sum(p[1] for p in pts) / len(pts)))


def layout(mask: Image.Image | None = None) -> dict:
    if mask:
        lb, _ = shoulder_shelf_from_mask(mask, "L")
        rb, _ = shoulder_shelf_from_mask(mask, "R")
    else:
        lb, rb = LB_SHELF_FALLBACK, RB_SHELF_FALLBACK
    return {
        "LT": map_norm_box(*N_LT),
        "RT": map_norm_box(*N_RT),
        "LB": shelf_bbox(lb),
        "RB": shelf_bbox(rb),
        "LB_SHELF": lb,
        "RB_SHELF": rb,
        "PLATEAU": map_norm_box(*N_PLATEAU),
        "LS": map_norm_point(*N_LS),
        "RS": map_norm_point(*N_RS),
        "FACE_Y": map_norm_point(*N_FACE_Y),
        "FACE_X": map_norm_point(*N_FACE_X),
        "FACE_A": map_norm_point(*N_FACE_A),
        "FACE_B": map_norm_point(*N_FACE_B),
        "DPAD": map_norm_point(*N_DPAD),
        "GUIDE": map_norm_point(*N_GUIDE),
        "VIEW": map_norm_box(*N_VIEW),
        "MENU": map_norm_box(*N_MENU),
    }


def body_mask(ref: Image.Image, include_horns: bool) -> Image.Image:
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
    dark = visible & (mx < REF_BODY_THRESH)
    if include_horns:
        core = dark
    else:
        core = dark & (np.arange(arr.shape[0])[:, None] >= REF_BODY_TOP)
    mask = Image.fromarray((core.astype(np.uint8) * 255))
    mask = mask.filter(ImageFilter.MaxFilter(5 if include_horns else 3))
    rx0, ry0, rx1, ry1 = REF_EXTENT
    crop = mask.crop((rx0, ry0, rx1 + 1, ry1 + 1))
    ax0, ay0, ax1, ay1 = ART
    crop = crop.resize((ax1 - ax0, ay1 - ay0), Image.Resampling.LANCZOS)
    placed = Image.new("L", (W, H), 0)
    placed.paste(crop, (ax0, ay0))
    return placed.point(lambda v: 255 if v > 64 else 0)


def draw_pad(variant: str, press: bool = True) -> Image.Image:
    ref = Image.open(REF)
    include_horns = variant == "merge"
    mask = body_mask(ref, include_horns)
    lay = layout(mask)
    sw, sh = W * S, H * S
    mask_hi = mask.resize((sw, sh), Image.Resampling.NEAREST)

    body = Image.new("RGBA", (sw, sh), (0, 0, 0, 0))
    body.paste(FILL, mask=mask_hi)
    edge = outline_mask(mask_hi, 6)
    stroke = Image.new("RGBA", (sw, sh), (0, 0, 0, 0))
    stroke.paste(STROKE, mask=edge)
    body = Image.alpha_composite(body, stroke)
    d = ImageDraw.Draw(body)
    st = 6

    def ring(cx, cy, r, w=st):
        d.ellipse([(cx - r) * S, (cy - r) * S, (cx + r) * S, (cy + r) * S], outline=STROKE, width=w)

    def fill_circle(cx, cy, r, color):
        d.ellipse([(cx - r) * S, (cy - r) * S, (cx + r) * S, (cy + r) * S], fill=color)

    def label(text, cx, cy, size, italic=False):
        f = font(size * S, italic=italic)
        x0, y0, x1, y1 = f.getbbox(text)
        d.text((cx * S - (x1 - x0) / 2, cy * S - (y1 - y0) / 2 - y0), text, font=f, fill=LABEL)

    def pill(box, text):
        x, y, w, h = box
        rounded(d, [x * S, y * S, (x + w) * S, (y + h) * S], h * S * 0.45, outline=STROKE, width=st)
        label(text, x + w / 2, y + h / 2, 13)

    def trigger(box, text):
        x, y, w, h = box
        label(text, x + w / 2, y + h * 0.42, 36, italic=True)

    def bumper_shelf(pts: tuple[tuple[int, int], ...], text: str):
        flat = [c for p in pts for c in (p[0] * S, p[1] * S)]
        d.polygon(flat, fill=WELL)
        for i in range(len(pts)):
            x0, y0 = pts[i]
            x1, y1 = pts[(i + 1) % len(pts)]
            d.line([(x0 * S, y0 * S), (x1 * S, y1 * S)], fill=STROKE, width=st)
        cx, cy = shelf_center(pts)
        label(text, cx, cy, 15)

    LT, RT = lay["LT"], lay["RT"]

    if variant == "twin":
        x, y, w, h = LT
        rounded(d, [x * S, y * S, (x + w) * S, (y + h) * S], 46 * S, fill=FILL, outline=STROKE, width=st)
        label("LT", x + w / 2, y + h * 0.38, 36, italic=True)
        x, y, w, h = RT
        rounded(d, [x * S, y * S, (x + w) * S, (y + h) * S], 46 * S, fill=FILL, outline=STROKE, width=st)
        label("RT", x + w / 2, y + h * 0.38, 36, italic=True)
    else:
        for box, text in ((LT, "LT"), (RT, "RT")):
            trigger(box, text)

    bumper_shelf(lay["LB_SHELF"], "LB")
    bumper_shelf(lay["RB_SHELF"], "RB")

    if variant == "plateau":
        px, py, pw, ph = lay["PLATEAU"]
        rounded(d, [px * S, py * S, (px + pw) * S, (py + ph) * S], 12 * S, fill=PLATEAU, outline=STROKE, width=st)

    lx, ly = lay["LS"]
    rx, ry = lay["RS"]
    if press:
        lx -= 26
        ly -= 34
    for cx, cy, r in ((lx, ly, LS_R), (rx, ry, RS_R)):
        fill_circle(cx, cy, r, WELL)
        ring(cx, cy, r)
        ring(cx, cy, int(r * 0.46), 4)

    for key, glyph in (("FACE_Y", "Y"), ("FACE_B", "B"), ("FACE_A", "A"), ("FACE_X", "X")):
        cx, cy = lay[key]
        col = ORANGE if press and glyph == "A" else FILL
        fill_circle(cx, cy, FACE_R, col)
        ring(cx, cy, FACE_R)
        label(glyph, cx, cy + 1, 28)

    if press:
        x, y, w, h = RT
        squeeze = Image.new("RGBA", (sw, sh), (0, 0, 0, 0))
        sd = ImageDraw.Draw(squeeze)
        sd.pieslice([x * S, (y + int(h * 0.55)) * S, (x + w) * S, (y + h) * S], 180, 0, fill=(*ORANGE[:3], 180))
        body = Image.alpha_composite(body, squeeze)

    dpx, dpy = lay["DPAD"]
    draw_dpad(d, dpx, dpy, st)

    pill(lay["VIEW"], "VIEW")
    pill(lay["MENU"], "MENU")
    gx, gy = lay["GUIDE"]
    fill_circle(gx, gy, GUIDE_R, PLATEAU if variant == "plateau" else FILL)
    ring(gx, gy, GUIDE_R)
    gr = GUIDE_R * 0.55
    d.arc([(gx - gr) * S, (gy - gr) * S, (gx + gr) * S, (gy + gr) * S], 200, 340, fill=LABEL, width=st)
    d.arc([(gx - gr) * S, (gy - gr) * S, (gx + gr) * S, (gy + gr) * S], 20, 160, fill=LABEL, width=st)

    out = body.resize((W, H), Image.Resampling.LANCZOS)
    arr = np.array(out)
    opaque = arr[..., 3] > 0
    arr[opaque, 3] = 255
    return Image.fromarray(arr)


def frame_comp(pad: Image.Image, title: str, subtitle: str) -> Image.Image:
    comp = Image.new("RGBA", (960, 540), (10, 10, 10, 255))
    alpha = pad.split()[3]
    bbox = alpha.getbbox()
    if bbox:
        crop = pad.crop(bbox)
        tw = int(960 * 0.78)
        scale = tw / crop.width
        th = int(crop.height * scale)
        crop = crop.resize((tw, th), Image.Resampling.LANCZOS)
        comp.paste(crop, ((960 - tw) // 2, (540 - th) // 2 + 18), crop)
    d = ImageDraw.Draw(comp)
    d.text((28, 22), title, font=font(24), fill=(228, 228, 230, 255))
    d.text((28, 54), subtitle, font=font(14), fill=(132, 132, 138, 255))
    return comp


def main() -> None:
    DECISION.mkdir(parents=True, exist_ok=True)
    variants = [
        ("twin", "Twin", "DS4 blueprint. LT/RT tombstones + LB/RB bars. Opaque charcoal body from your outline (no glow)."),
        ("merge", "Merge", "One silhouette: horns merged into the body stroke. LT/RT labels inside the horns."),
        ("plateau", "Plateau", "Reference black top band for guide / view / menu. Horns + shoulders match ref placement."),
    ]
    for slug, title, subtitle in variants:
        pad = draw_pad(slug)
        pad.save(ASSETS / f"gamepad-xbox-{slug}.png", "PNG")
        comp = frame_comp(pad, title, subtitle)
        comp.save(DECISION / f"gamepad-xbox-{slug}.png", "PNG")
        print("wrote", slug)


if __name__ == "__main__":
    main()
