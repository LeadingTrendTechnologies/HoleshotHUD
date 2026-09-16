"""Xbox One pad — traced 1:1 from gamepad-xbox-target.png, painted in Holeshot tokens."""

from __future__ import annotations

from pathlib import Path

import numpy as np
from PIL import Image, ImageChops, ImageDraw

from xbox_pad_trace import (
    Art,
    CAP_R,
    FACE_R,
    GUIDE_R,
    LETTER_CAP,
    T_FACE,
    T_GUIDE,
    T_LS,
    T_RS,
    W,
    H,
    S,
    dilated,
    finalize,
    font_for_cap,
    print_layout,
    trace_masks,
)

DEST = Path(__file__).with_name("gamepad-xbox.png")

CREAM = (228, 228, 230, 255)
INK = (10, 10, 10, 255)
CAP = (48, 52, 64, 255)
KNOB = (250, 250, 252, 255)
FACE_COLOR = {
    "Y": (244, 214, 36, 255),
    "X": (59, 130, 246, 255),
    "B": (255, 64, 72, 255),
    "A": (48, 220, 88, 255),
}


def main() -> None:
    body_m, outer_m, inner_m, guide_x = trace_masks()
    art = Art(body_m | outer_m | inner_m | guide_x)

    body_hi = art.contour_layer(dilated(body_m))
    outer_hi = art.contour_layer(outer_m)
    ink_hi = ImageChops.lighter(outer_hi, art.layer(inner_m))
    cream_hi = ImageChops.lighter(body_hi, outer_hi)

    canvas = Image.new("RGBA", (W * S, H * S), (0, 0, 0, 0))
    for alpha, color in ((cream_hi, CREAM), (ink_hi, INK)):
        canvas.paste(Image.new("RGBA", canvas.size, color), (0, 0), alpha)

    d = ImageDraw.Draw(canvas)

    def disc(center, r, color):
        cx, cy = art.pt(*center)
        rr = r * art.k * S
        d.ellipse([cx * S - rr, cy * S - rr, cx * S + rr, cy * S + rr], fill=color)

    disc(T_GUIDE, GUIDE_R, KNOB)
    canvas.paste(Image.new("RGBA", canvas.size, INK), (0, 0), art.layer(guide_x, smooth=False))

    for center in (T_LS, T_RS):
        disc(center, CAP_R, CAP)

    cap_px = LETTER_CAP * art.k * S
    f = font_for_cap(cap_px)
    for letter, center in T_FACE.items():
        cx, cy = art.pt(*center)
        x0, y0, x1, y1 = f.getbbox(letter)
        d.text((cx * S - (x1 - x0) / 2 - x0, cy * S - (y1 - y0) / 2 - y0), letter, font=f, fill=FACE_COLOR[letter])

    finalize(canvas).save(DEST, "PNG")
    print("wrote", DEST)
    ink_art = np.array(ink_hi.resize((W, H), Image.Resampling.LANCZOS)) > 128
    print_layout(art, ink_art)


if __name__ == "__main__":
    main()
