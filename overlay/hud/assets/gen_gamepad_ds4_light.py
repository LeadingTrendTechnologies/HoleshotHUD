"""DualShock 4 filled (Light) skin — cream body, ink controls, colored symbols."""

from __future__ import annotations

from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFilter, ImageFont

SRC = Path(__file__).with_name("gamepad-ds4.png")
DEST = Path(__file__).with_name("gamepad-ds4-light.png")
W, H = 1536, 1024

CREAM = (228, 228, 230, 255)
INK = (10, 10, 10, 255)
CAP = (48, 52, 64, 255)
SYMBOL = {
    "tri": (244, 214, 36, 255),
    "circ": (255, 64, 72, 255),
    "cross": (59, 130, 246, 255),
    "sq": (48, 220, 88, 255),
}

LS = (0.3554 * W, 0.6117 * H)
RS = (0.6425 * W, 0.6117 * H)
WELL_R = 108.0
CAP_R = 48.0
FACE = {
    "tri": (0.7669 * W, 0.3389 * H),
    "circ": (0.8249 * W, 0.4258 * H),
    "cross": (0.7689 * W, 0.5107 * H),
    "sq": (0.7103 * W, 0.4229 * H),
}
FACE_R = 39.0
DPAD_C = (0.2272 * W, 0.4243 * H)
DPAD_ARM = 0.055 * W
DPAD_BAR = 0.065 * H
DOT = {"back": (0.255 * W, 0.275 * H), "start": (0.660 * W, 0.275 * H)}
DOT_R = 22.0
TOUCH = (0.390 * W, 0.280 * H, 0.220 * W, 0.160 * H)
GUIDE = (0.4996 * W, 0.6261 * H)
GUIDE_R = 13.0


def silhouette() -> np.ndarray:
    arr = np.array(Image.open(SRC).convert("RGBA"))
    al = arr[..., 3] > 128
    im = Image.fromarray((al * 255).astype(np.uint8))
    im = im.filter(ImageFilter.MaxFilter(5)).filter(ImageFilter.GaussianBlur(1.2))
    return np.array(im) > 128


def main() -> None:
    sil = silhouette()
    out = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    out.paste(Image.new("RGBA", (W, H), CREAM), (0, 0), Image.fromarray((sil * 255).astype(np.uint8)))
    d = ImageDraw.Draw(out)

    def disc(cx, cy, r, color):
        d.ellipse([cx - r, cy - r, cx + r, cy + r], fill=color)

    def rect_uv(u, v, w, h, color):
        d.rectangle([u, v, u + w, v + h], fill=color)

    rect_uv(*TOUCH, INK)
    for c in (LS, RS):
        disc(*c, WELL_R, INK)
        disc(*c, CAP_R, CAP)
    cx, cy = DPAD_C
    d.rectangle([cx - DPAD_BAR / 2, cy - DPAD_ARM, cx + DPAD_BAR / 2, cy], fill=INK)
    d.rectangle([cx - DPAD_ARM, cy - DPAD_BAR / 2, cx, cy + DPAD_BAR / 2], fill=INK)
    d.rectangle([cx, cy - DPAD_BAR / 2, cx + DPAD_ARM, cy + DPAD_BAR / 2], fill=INK)
    d.rectangle([cx - DPAD_BAR / 2, cy, cx + DPAD_BAR / 2, cy + DPAD_ARM], fill=INK)
    for center in FACE.values():
        disc(*center, FACE_R, INK)
    for c in DOT.values():
        disc(*c, DOT_R, INK)
    disc(*GUIDE, GUIDE_R, INK)

    try:
        f = ImageFont.truetype(r"C:\Windows\Fonts\segoeuisymbol.ttf", 36)
    except OSError:
        f = ImageFont.load_default()
    glyphs = {"tri": "\u25b2", "circ": "\u25cb", "cross": "\u2715", "sq": "\u25a1"}
    for name, center in FACE.items():
        ch = glyphs[name]
        cx, cy = center
        x0, y0, x1, y1 = f.getbbox(ch)
        d.text((cx - (x1 - x0) / 2 - x0, cy - (y1 - y0) / 2 - y0), ch, font=f, fill=SYMBOL[name])

    out.save(DEST, "PNG")
    print("wrote", DEST)


if __name__ == "__main__":
    main()
