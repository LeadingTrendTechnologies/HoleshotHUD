"""DualShock 4 Light skin — recolors `gamepad-ds4-dark.png` into `gamepad-ds4-light.png`.

Every stroke and hole of the dark art stays where it is, so both PlayStation skins share
`ds4_gamepad_layout`. Each charcoal region between the strokes is classified (body, panels,
controls, wells) and repainted; the dark art's anti-aliased stroke coverage is kept, and each
stroke turns black where it borders the cream body or the outside, cream where it sits wholly
inside a dark region (d-pad key outlines, face rings and symbols, SHARE/OPTIONS text). Marks
inside the stick wells are a muted grey.

Press fills depend on the luminance bands: panels and outlines stay below 30, controls
(triggers, bumpers, d-pad keys) sit in 30-78, cream is far above.
"""

from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage

SRC = Path(__file__).with_name("gamepad-ds4-dark.png")
DEST = Path(__file__).with_name("gamepad-ds4-light.png")
W, H = 1536, 1024

BODY = (242, 238, 228)
OUTLINE = (18, 18, 20)
PANEL = (16, 16, 18)
CONTROL = (58, 62, 72)
LIP = (196, 160, 120)
WELL = (16, 16, 18)
WELL_RING = (52, 54, 60)
WELL_GAP = (30, 31, 35)
CAP = (62, 64, 72)
MARK = (236, 232, 222)
WELL_MARK = (96, 98, 106)
WELL_MARK_R = 96

# Dark art palette: fill pixels are charcoal, strokes ramp up to grey.
FILL_LUM = 60
STROKE_LO, STROKE_HI = 34.0, 118.0
TOUCH = 2

# Seed points (art px) for each region class; each seed picks the fill component under it.
CONTROL_SEEDS = [
    (360, 110), (1175, 110),  # L2, R2
    (360, 200), (1175, 200),  # L1, R1
    (349, 375), (410, 435), (289, 435), (349, 495),  # d-pad keys
]
LIP_SEEDS = [(360, 182), (1175, 182)]
PANEL_SEEDS = [
    (765, 355),  # touchpad
    (250, 350), (1080, 350),  # d-pad and face panels
    (479, 262), (1054, 262),  # SHARE / OPTIONS pills
    (766, 616),  # PS disc
]
WELL_CENTERS = [(546, 626), (987, 626)]


def lum_of(rgb: np.ndarray) -> np.ndarray:
    return rgb[..., 0] * 0.30 + rgb[..., 1] * 0.59 + rgb[..., 2] * 0.11


def main() -> None:
    dest = Path(sys.argv[1]) if len(sys.argv) > 1 else DEST
    src = np.array(Image.open(SRC).convert("RGBA")).astype(np.float64)
    assert src.shape[:2] == (H, W), src.shape
    alpha = src[..., 3]
    rgb = np.where(alpha[..., None] > 0, src[..., :3], 0.0)
    lum = lum_of(rgb)
    opaque = alpha > 0
    enclosed = ndimage.binary_fill_holes(opaque) & ~opaque

    fill = opaque & (alpha > 250) & (lum < FILL_LUM)
    fill_lab, _ = ndimage.label(fill)
    body_id = np.bincount(fill_lab.ravel())[1:].argmax() + 1
    hole_lab, _ = ndimage.label(enclosed)

    def region_at(labels: np.ndarray, x: int, y: int) -> int:
        found = labels[y, x]
        if found:
            return int(found)
        near = labels[max(y - 6, 0) : y + 7, max(x - 6, 0) : x + 7]
        ids = near[near > 0]
        return int(np.bincount(ids).argmax()) if len(ids) else 0

    color = np.zeros((H, W, 3))
    color[:] = BODY
    dark = np.zeros((H, W), bool)

    assigned = fill_lab == body_id

    def paint(mask: np.ndarray, rgb_color: tuple[int, int, int], is_dark: bool) -> None:
        color[mask] = rgb_color
        assigned[mask] = True
        if is_dark:
            dark[mask] = True

    for x, y in CONTROL_SEEDS:
        paint(fill_lab == region_at(fill_lab, x, y), CONTROL, True)
    for x, y in LIP_SEEDS:
        paint(fill_lab == region_at(fill_lab, x, y), LIP, False)
    for x, y in PANEL_SEEDS:
        paint(fill_lab == region_at(fill_lab, x, y), PANEL, True)
    # Face buttons and their symbol interiors are the fills inside the face panel's bounding box.
    objects = ndimage.find_objects(fill_lab)
    face_box = objects[region_at(fill_lab, 1080, 350) - 1]
    for i, (sy, sx) in enumerate(objects):
        inside = (face_box[0].start <= sy.start and sy.stop <= face_box[0].stop
                  and face_box[1].start <= sx.start and sx.stop <= face_box[1].stop)
        if inside and i + 1 != body_id:
            paint(fill_lab == i + 1, PANEL, True)

    # Wells: holes and fill rings around each stick center, by radius from the center.
    gy, gx = np.ogrid[:H, :W]
    for cx, cy in WELL_CENTERS:
        r = np.sqrt((gx - cx) ** 2 + (gy - cy) ** 2)
        well = r <= 100
        paint(well & (r > 72), WELL, True)
        paint(well & (r > 57) & (r <= 72), WELL_RING, True)
        paint(well & (r > 51) & (r <= 57), WELL_GAP, True)
        paint(well & (r <= 51), CAP, True)
    # Grille dots and the PS logo (with the gap around its disc) are the remaining holes.
    logo = hole_lab == region_at(hole_lab, 726, 643)
    wells = np.zeros((H, W), bool)
    for cx, cy in WELL_CENTERS:
        wells |= (gx - cx) ** 2 + (gy - cy) ** 2 <= 100 ** 2
    paint(enclosed & ~logo & ~wells, PANEL, True)
    paint(logo, MARK, False)

    # Leftover small fills (letter counters, PS logo pieces) take the region around them.
    leftover_lab, leftover_count = ndimage.label(fill & ~assigned)
    for i, (sy, sx) in enumerate(ndimage.find_objects(leftover_lab)):
        box = (slice(max(sy.start - 8, 0), sy.stop + 8), slice(max(sx.start - 8, 0), sx.stop + 8))
        piece = leftover_lab[box] == i + 1
        ring = ndimage.binary_dilation(piece, iterations=8) & ~piece & assigned[box]
        if not ring.any():
            continue
        dark_share = dark[box][ring].mean()
        around = color[box][ring & (dark[box] == (dark_share >= 0.5))]
        sub_color = color[box]
        sub_color[piece] = np.median(around, axis=0)
        if dark_share >= 0.5:
            dark[box][piece] = True
        assigned[box][piece] = True

    # Strokes: coverage from the dark art, color from what they border.
    coverage = np.clip((lum - STROKE_LO) / (STROKE_HI - STROKE_LO), 0.0, 1.0)
    coverage[~opaque | enclosed] = 0.0
    classified = fill | enclosed
    light_side = (classified & ~dark) | ~ndimage.binary_fill_holes(opaque)
    # A stroke that touches the body or the outside is an outline; one that sits wholly in a
    # dark region is a mark drawn on it.
    strokes = opaque & ~enclosed & ~fill
    stroke_lab, stroke_count = ndimage.label(strokes, structure=np.ones((3, 3), int))
    touching = np.unique(stroke_lab[ndimage.binary_dilation(light_side, iterations=TOUCH) & strokes])
    is_outline = np.zeros(stroke_count + 1, bool)
    is_outline[touching] = True
    per_stroke = np.where(is_outline[stroke_lab][..., None], OUTLINE, MARK).astype(np.float64)
    in_well = np.zeros((H, W), bool)
    for cx, cy in WELL_CENTERS:
        in_well |= (gx - cx) ** 2 + (gy - cy) ** 2 <= WELL_MARK_R ** 2
    per_stroke[in_well & ~is_outline[stroke_lab]] = WELL_MARK
    _, (sy_idx, sx_idx) = ndimage.distance_transform_edt(~strokes, return_indices=True)
    stroke_color = per_stroke[sy_idx, sx_idx]

    # Stroke pixels take the fill color of the nearest classified region underneath.
    _, (iy, ix) = ndimage.distance_transform_edt(~classified, return_indices=True)
    base = color[iy, ix]
    base[classified] = color[classified]
    out_rgb = base * (1.0 - coverage[..., None]) + stroke_color * coverage[..., None]
    # The silhouette edge reads as the black outline.
    edge = opaque & (alpha < 250) & ~enclosed
    out_rgb[edge] = OUTLINE
    out_alpha = np.where(enclosed, 255.0, alpha)

    out = np.dstack([out_rgb, out_alpha]).round().clip(0, 255).astype(np.uint8)
    out[out[..., 3] == 0, :3] = 0
    Image.fromarray(out, "RGBA").save(dest, optimize=True)
    print("wrote", dest)


if __name__ == "__main__":
    main()
