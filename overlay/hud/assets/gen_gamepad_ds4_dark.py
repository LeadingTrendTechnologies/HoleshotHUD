"""DualShock 4 dark schematic skin — cleans `gamepad-ds4.png` into `gamepad-ds4-dark.png`.

The source has a hard 0/255 alpha silhouette with notches and transparent pinholes along its
strokes, which read as broken, dashed lines once the widget scales the pad down. This keeps every
stroke and control pixel where it is (the `ds4_gamepad_layout` UVs stay valid) and only:
  - fills pinholes smaller than PINHOLE_MAX with the nearest opaque colour,
  - replaces the outer silhouette with a traced, low-passed contour rendered at S x and
    box-averaged back down, so the body edge is anti-aliased.
Intentional holes (stick wells, speaker grille, SHARE/OPTIONS counters) stay transparent.
The L2/R2/L1/R1 labels are erased to body charcoal; the wing and bar outlines stay.
"""

from __future__ import annotations

from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw
from scipy import ndimage

from xbox_pad_trace import smooth_closed, trace_boundary

SRC = Path(__file__).with_name("gamepad-ds4.png")
DEST = Path(__file__).with_name("gamepad-ds4-dark.png")
W, H = 1536, 1024
S = 4
PINHOLE_MAX = 40
CLOSE_R = 2
CONTOUR_SIGMA = 2.5
LABEL_LUM = 60
LABEL_PAD = 4
# Search windows (x0, y0, x1, y1). Only stroke components wholly inside one are erased.
LABEL_WINDOWS = {
    "L2": (320, 100, 400, 150),
    "R2": (1135, 100, 1215, 150),
    "L1": (340, 188, 382, 212),
    "R1": (1152, 188, 1196, 212),
}


def erase_labels(rgb: np.ndarray, opaque: np.ndarray) -> None:
    wide = rgb.astype(np.int32)
    lum = (wide[..., 0] * 30 + wide[..., 1] * 59 + wide[..., 2] * 11) // 100
    stroke = opaque & (lum >= LABEL_LUM)
    lab, _ = ndimage.label(stroke, structure=np.ones((3, 3), int))
    objects = ndimage.find_objects(lab)
    for name, (x0, y0, x1, y1) in LABEL_WINDOWS.items():
        letters = np.zeros(stroke.shape, bool)
        others = np.zeros(stroke.shape, bool)
        for i, (sy, sx) in enumerate(objects):
            inside = x0 <= sx.start and sx.stop <= x1 and y0 <= sy.start and sy.stop <= y1
            if inside:
                letters |= lab == i + 1
            elif sy.start < y1 and sy.stop > y0 and sx.start < x1 and sx.stop > x0:
                others |= lab == i + 1
        ys, xs = np.where(letters)
        if len(xs) == 0:
            raise SystemExit(f"no {name} label found in {LABEL_WINDOWS[name]}")
        print(f"{name}: erased x {xs.min()}-{xs.max()} y {ys.min()}-{ys.max()} ({len(xs)} px)")
        # The letters carry an emboss shadow darker than the body, so clear the padded box,
        # sparing any outline stroke (and its soft edge) that passes through it.
        box = np.zeros(stroke.shape, bool)
        box[max(ys.min() - LABEL_PAD, 0) : ys.max() + LABEL_PAD + 1,
            max(xs.min() - LABEL_PAD, 0) : xs.max() + LABEL_PAD + 1] = True
        spare = ndimage.binary_dilation(others, iterations=2)
        window = (slice(y0, y1), slice(x0, x1))
        body = opaque[window] & (lum[window] < 45) & ~box[window] & ~spare[window]
        rgb[box & opaque & ~spare] = np.median(rgb[window][body], axis=0).astype(rgb.dtype)


def disc(r: int) -> np.ndarray:
    y, x = np.ogrid[-r : r + 1, -r : r + 1]
    return x * x + y * y <= r * r


def main() -> None:
    src = np.array(Image.open(SRC).convert("RGBA"))
    assert src.shape[:2] == (H, W), src.shape
    opaque = src[..., 3] > 0

    enclosed = ndimage.binary_fill_holes(opaque) & ~opaque
    lab, n = ndimage.label(enclosed)
    sizes = ndimage.sum(enclosed, lab, range(1, n + 1))
    keep_holes = np.isin(lab, [i + 1 for i in range(n) if sizes[i] >= PINHOLE_MAX])

    padded = np.pad(opaque, CLOSE_R * 2)
    closed = ndimage.binary_closing(padded, structure=disc(CLOSE_R))[
        CLOSE_R * 2 : -CLOSE_R * 2, CLOSE_R * 2 : -CLOSE_R * 2
    ]
    body = ndimage.binary_fill_holes(closed | opaque)
    body_lab, _ = ndimage.label(body)
    body = body_lab == np.bincount(body_lab.ravel())[1:].argmax() + 1

    hi = Image.new("L", (W * S, H * S), 0)
    poly = smooth_closed(trace_boundary(body), CONTOUR_SIGMA)
    ImageDraw.Draw(hi).polygon([(x * S, y * S) for x, y in poly], fill=255)
    edge = np.asarray(hi, np.float32).reshape(H, S, W, S).mean(axis=(1, 3))

    alpha = edge.copy()
    alpha[keep_holes] = 0.0
    alpha[opaque & ~body] = 255.0
    alpha[(alpha > 250)] = 255.0
    alpha[(alpha < 4)] = 0.0

    _, (iy, ix) = ndimage.distance_transform_edt(~opaque, return_indices=True)
    rgb = src[..., :3][iy, ix]
    rgb[opaque] = src[..., :3][opaque]
    erase_labels(rgb, body & ~keep_holes)

    out = np.dstack([rgb, alpha.round().astype(np.uint8)])
    out[out[..., 3] == 0, :3] = 0
    Image.fromarray(out, "RGBA").save(DEST, optimize=True)

    partial = int(((out[..., 3] > 0) & (out[..., 3] < 255)).sum())
    filled = int((~opaque & (out[..., 3] == 255)).sum())
    print(f"wrote {DEST.name}: {partial} anti-aliased edge px, {filled} pinhole/notch px filled")


if __name__ == "__main__":
    main()
