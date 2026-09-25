"""Xbox pad — Dark skin in the DualShock dark style: charcoal body, thin grey outlines.

Derived from `gamepad-xbox-light.png` so every control keeps its light-skin position and
`xbox_gamepad_layout` stays valid. Each boundary between drawn regions (silhouette edge,
ink shapes, stick caps, guide disc, face letters) becomes a grey stroke; everything else is
charcoal. Rendered at 4x and box-averaged down so the strokes stay continuous.
"""

from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageFilter
from scipy import ndimage

SOURCE = Path(__file__).with_name("gamepad-xbox-light.png")
DEST = Path(__file__).with_name("gamepad-xbox-dark.png")

SUPERSAMPLE = 4
BODY = np.array([29, 31, 34], float)
LINE = np.array([128, 130, 136], float)
CAP = np.array([54, 56, 70], float)
STROKE_W = 3.2
EDGE_W = 4.0
CLASS_BLUR = 0.8

GUIDE = (672.0, 251.5)
GUIDE_R = 57.4
DPAD = (500.3, 610.8)
DPAD_RING_R = 124.0

TRANSPARENT, BODY_CLASS, INK, CAP_CLASS, PAPER, LETTER = range(6)


def classify(rgba: np.ndarray) -> np.ndarray:
    rgb = rgba[..., :3].astype(np.int32)
    alpha = rgba[..., 3]
    hi = rgb.max(2)
    lo = rgb.min(2)
    sat = hi - lo
    h, w = alpha.shape
    gy, gx = np.ogrid[:h, :w]
    in_guide = (gx + 0.5 - GUIDE[0]) ** 2 + (gy + 0.5 - GUIDE[1]) ** 2 <= (GUIDE_R + 2) ** 2

    label = np.full((h, w), INK, np.uint8)
    label[(lo >= 150) & (sat <= 24)] = BODY_CLASS
    label[(lo >= 150) & (sat <= 24) & in_guide] = PAPER
    label[(hi < 110) & (rgb[..., 2] - rgb[..., 0] >= 8) & (sat <= 30)] = CAP_CLASS
    label[(sat >= 60) & (hi >= 90)] = LETTER
    label[alpha < 128] = TRANSPARENT
    return label


def upsampled(mask: np.ndarray, size: tuple[int, int]) -> np.ndarray:
    im = Image.fromarray((mask * 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(CLASS_BLUR))
    return np.array(im.resize(size, Image.Resampling.BICUBIC)) > 127


def boundary_stroke(mask: np.ndarray, width: float) -> np.ndarray:
    inside = ndimage.distance_transform_edt(mask)
    outside = ndimage.distance_transform_edt(~mask)
    distance = np.where(mask, inside - 0.5, outside - 0.5)
    return np.clip(width / 2 + 0.5 - distance, 0.0, 1.0)


def ring_stroke(shape: tuple[int, int], cx: float, cy: float, r: float, width: float) -> np.ndarray:
    gy, gx = np.ogrid[: shape[0], : shape[1]]
    distance = np.abs(np.sqrt((gx + 0.5 - cx) ** 2 + (gy + 0.5 - cy) ** 2) - r)
    return np.clip(width / 2 + 0.5 - distance, 0.0, 1.0)


def box_average(a: np.ndarray, k: int) -> np.ndarray:
    h, w = a.shape[0] // k, a.shape[1] // k
    return a[: h * k, : w * k].reshape(h, k, w, k, *a.shape[2:]).mean(axis=(1, 3))


def main() -> None:
    dest = Path(sys.argv[1]) if len(sys.argv) > 1 else DEST
    src = np.array(Image.open(SOURCE).convert("RGBA"))
    h, w = src.shape[:2]
    k = SUPERSAMPLE
    size = (w * k, h * k)
    label = classify(src)

    silhouette = upsampled(label != TRANSPARENT, size)
    cap = upsampled(label == CAP_CLASS, size)
    stroke = np.clip(EDGE_W * k + 0.5 - ndimage.distance_transform_edt(silhouette), 0.0, 1.0)
    for region in (INK, CAP_CLASS, PAPER, LETTER):
        stroke = np.maximum(stroke, boundary_stroke(upsampled(label == region, size), STROKE_W * k))
    ring = ring_stroke(silhouette.shape, DPAD[0] * k, DPAD[1] * k, DPAD_RING_R * k, STROKE_W * k)
    stroke = np.maximum(stroke, ring)
    stroke *= silhouette

    color = np.where(cap[..., None], CAP, BODY)
    color = color * (1.0 - stroke[..., None]) + LINE * stroke[..., None]
    alpha = silhouette.astype(float)
    premultiplied = np.concatenate([color * alpha[..., None], alpha[..., None] * 255.0], axis=2)
    low = box_average(premultiplied, k)
    a = low[..., 3]
    rgb = np.where(a[..., None] > 0, low[..., :3] / np.maximum(a[..., None], 1e-6) * 255.0, 0.0)
    out = np.concatenate([rgb, a[..., None]], axis=2).round().clip(0, 255).astype(np.uint8)
    Image.fromarray(out, "RGBA").save(dest, "PNG")
    print("wrote", dest)


if __name__ == "__main__":
    main()
