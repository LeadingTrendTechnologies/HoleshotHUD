"""Xbox pad — schematic Dark skin (charcoal body, cream outlines, L1/R1/L2/R2)."""

from __future__ import annotations

from pathlib import Path

import numpy as np
from PIL import Image, ImageChops, ImageDraw

from xbox_pad_trace import (
    Art,
    BUMP_L,
    BUMP_R,
    CAP_R,
    DOT_R,
    FACE_R,
    GUIDE_R,
    HORN_L,
    HORN_R,
    LETTER_CAP,
    SHARE_H,
    SHARE_W,
    T_DPAD,
    T_FACE,
    T_GUIDE,
    T_LS,
    T_MENU,
    T_RS,
    T_SHARE,
    T_VENT,
    T_VIEW,
    WELL_R,
    W,
    H,
    S,
    DPAD_ARM,
    DPAD_BAR,
    dilated,
    finalize,
    font_for_cap,
    font_for_label,
    print_layout,
    trace_masks,
)

DEST = Path(__file__).with_name("gamepad-xbox-dark.png")

BODY = (22, 22, 24, 255)
LINE = (248, 248, 252, 255)
MID = (48, 52, 64, 255)


def sw() -> int:
    return max(1, int(round(1.1 * S)))


def p(art: Art, x: float, y: float) -> tuple[float, float]:
    cx, cy = art.pt(x, y)
    return cx * S, cy * S


def paste_strokes(canvas: Image.Image, alpha: Image.Image, color: tuple[int, int, int, int]) -> None:
    canvas.paste(Image.new("RGBA", canvas.size, color), (0, 0), alpha)


def draw_dpad(d: ImageDraw.ImageDraw, art: Art) -> None:
    cx, cy = p(art, *T_DPAD)
    arm = DPAD_ARM * art.k * S
    bar = DPAD_BAR * art.k * S
    w = sw()
    d.rectangle([cx - bar / 2, cy - arm, cx + bar / 2, cy - arm + bar], outline=LINE, width=w)
    d.rectangle([cx - bar / 2, cy + arm - bar, cx + bar / 2, cy + arm], outline=LINE, width=w)
    d.rectangle([cx - arm, cy - bar / 2, cx - arm + bar, cy + bar / 2], outline=LINE, width=w)
    d.rectangle([cx + arm - bar, cy - bar / 2, cx + arm, cy + bar / 2], outline=LINE, width=w)
    tri = max(3.0, 5.0 * art.k * S)
    for dx, dy, pts in (
        (0, -1, [(cx, cy - arm + tri * 2), (cx - tri, cy - arm + tri * 4), (cx + tri, cy - arm + tri * 4)]),
        (0, 1, [(cx, cy + arm - tri * 2), (cx - tri, cy + arm - tri * 4), (cx + tri, cy + arm - tri * 4)]),
        (-1, 0, [(cx - arm + tri * 2, cy), (cx - arm + tri * 4, cy - tri), (cx - arm + tri * 4, cy + tri)]),
        (1, 0, [(cx + arm - tri * 2, cy), (cx + arm - tri * 4, cy - tri), (cx + arm - tri * 4, cy + tri)]),
    ):
        d.polygon(pts, outline=LINE, width=w)


def draw_guide(d: ImageDraw.ImageDraw, art: Art) -> None:
    cx, cy = p(art, *T_GUIDE)
    rr = GUIDE_R * art.k * S
    w = sw()
    d.ellipse([cx - rr, cy - rr, cx + rr, cy + rr], outline=LINE, width=w)
    s = rr * 0.55
    d.line([(cx - s, cy - s), (cx + s, cy + s)], fill=LINE, width=w)
    d.line([(cx - s, cy + s), (cx + s, cy - s)], fill=LINE, width=w)


def draw_view_menu(d: ImageDraw.ImageDraw, art: Art) -> None:
    w = sw()
    for center in (T_VIEW, T_MENU):
        cx, cy = p(art, *center)
        rr = DOT_R * art.k * S
        d.ellipse([cx - rr, cy - rr, cx + rr, cy + rr], outline=LINE, width=w)
    vx, vy = p(art, *T_VIEW)
    s = DOT_R * art.k * S * 0.35
    d.rectangle([vx - s * 1.2, vy - s, vx - s * 0.1, vy + s], outline=LINE, width=w)
    d.rectangle([vx + s * 0.1, vy - s * 0.6, vx + s * 1.2, vy + s * 0.4], outline=LINE, width=w)
    mx, my = p(art, *T_MENU)
    gap = s * 0.55
    for i in (-1, 0, 1):
        y = my + i * gap
        d.line([(mx - s, y), (mx + s, y)], fill=LINE, width=w)


def draw_share(d: ImageDraw.ImageDraw, art: Art) -> None:
    cx, cy = p(art, *T_SHARE)
    ww = SHARE_W * art.k * S
    hh = SHARE_H * art.k * S
    w = sw()
    r = hh * 0.35
    d.rounded_rectangle([cx - ww / 2, cy - hh / 2, cx + ww / 2, cy + hh / 2], radius=r, outline=LINE, width=w)
    ah = hh * 0.35
    d.line([(cx, cy + ah * 0.2), (cx, cy - ah * 0.5)], fill=LINE, width=w)
    d.polygon(
        [(cx, cy - ah * 0.85), (cx - ah * 0.45, cy - ah * 0.35), (cx + ah * 0.45, cy - ah * 0.35)],
        outline=LINE,
        width=w,
    )


def draw_vent(d: ImageDraw.ImageDraw, art: Art) -> None:
    cx, cy = p(art, *T_VENT)
    r = max(1.5, 2.2 * art.k * S)
    step = r * 2.8
    rows = (5, 4, 3, 2, 1)
    y = cy - step * 2
    for n in rows:
        x0 = cx - step * (n - 1) / 2
        for i in range(n):
            x = x0 + i * step
            d.ellipse([x - r, y - r, x + r, y + r], fill=LINE)
        y += step


def draw_sticks(d: ImageDraw.ImageDraw, art: Art) -> None:
    w = sw()
    for center in (T_LS, T_RS):
        cx, cy = p(art, *center)
        well = WELL_R * art.k * S
        cap = CAP_R * art.k * S
        d.ellipse([cx - well, cy - well, cx + well, cy + well], outline=LINE, width=w)
        d.ellipse([cx - cap, cy - cap, cx + cap, cy + cap], fill=MID)


def draw_faces(d: ImageDraw.ImageDraw, art: Art) -> None:
    w = sw()
    cap_px = LETTER_CAP * art.k * S
    f = font_for_cap(cap_px)
    rr = FACE_R * art.k * S
    for letter, center in T_FACE.items():
        cx, cy = p(art, *center)
        d.ellipse([cx - rr, cy - rr, cx + rr, cy + rr], outline=LINE, width=w)
        x0, y0, x1, y1 = f.getbbox(letter)
        d.text((cx - (x1 - x0) / 2 - x0, cy - (y1 - y0) / 2 - y0), letter, font=f, fill=LINE)


def draw_shoulder_labels(d: ImageDraw.ImageDraw, art: Art) -> None:
    f = font_for_label(11 * art.k * S)
    for label, box in (("L2", HORN_L), ("R2", HORN_R), ("L1", BUMP_L), ("R1", BUMP_R)):
        bx, by, bw, bh = box
        cx, cy = p(art, bx + bw / 2, by + bh / 2)
        x0, y0, x1, y1 = f.getbbox(label)
        d.text((cx - (x1 - x0) / 2 - x0, cy - (y1 - y0) / 2 - y0), label, font=f, fill=LINE)


def main() -> None:
    body_m, outer_m, inner_m, guide_x = trace_masks()
    art = Art(body_m | outer_m | inner_m | guide_x)

    body_hi = art.contour_layer(dilated(body_m))
    outer_hi = art.contour_layer(outer_m)
    ink_hi = ImageChops.lighter(outer_hi, art.layer(inner_m))
    pad_fill = art.layer(dilated(body_m | outer_m))

    canvas = Image.new("RGBA", (W * S, H * S), (0, 0, 0, 0))
    paste_strokes(canvas, pad_fill, BODY)
    paste_strokes(canvas, ImageChops.lighter(body_hi, outer_hi), LINE)

    d = ImageDraw.Draw(canvas)
    draw_sticks(d, art)
    draw_dpad(d, art)
    draw_faces(d, art)
    draw_guide(d, art)
    draw_view_menu(d, art)
    draw_share(d, art)
    draw_vent(d, art)
    draw_shoulder_labels(d, art)

    finalize(canvas).save(DEST, "PNG")
    print("wrote", DEST)
    ink_art = np.array(ink_hi.resize((W, H), Image.Resampling.LANCZOS)) > 128
    print_layout(art, ink_art)


if __name__ == "__main__":
    main()
