"""Shared Xbox pad tracing from gamepad-xbox-target.png (used by light + dark generators)."""

from __future__ import annotations

from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFilter, ImageFont
from scipy import ndimage

TARGET = Path(__file__).with_name("gamepad-xbox-target.png")

W, H = 1344, 1024
S = 4
MARGIN = 8
SMOOTH = 2.6
CONTOUR_SIGMA = 3.0

BODY_MIN = 225
BODY_SAT = 12
OUT_MAX = 58
OUT_SAT = 26
IN_MAX = 140
IN_SAT = 45
KEEP_AREA = 900
WINDOW = (13, 40, 694, 561)

T_LS = (187, 242)
T_RS = (440, 343)
WELL_R = 50
CAP_R = 34
T_DPAD = (268, 349)
DPAD_ARM = 48
DPAD_BAR = 32
T_FACE = {"Y": (522, 197), "B": (566, 242), "A": (522, 286), "X": (478, 242)}
FACE_R = 23
LETTER_CAP = 23
T_VIEW = (307, 243)
T_MENU = (402, 243)
DOT_R = 16
T_GUIDE = (354, 169)
GUIDE_R = 28
T_SHARE = (354, 218)
SHARE_W = 36
SHARE_H = 22
T_VENT = (354, 248)
HORN_L = (157, 48, 78, 54)
HORN_R = (474, 48, 78, 54)
BUMP_L = (123, 105, 133, 63)
BUMP_R = (452, 105, 133, 63)


def font_for_cap(cap_px: float) -> ImageFont.FreeTypeFont:
    for path in (r"C:\Windows\Fonts\segoeuib.ttf", r"C:\Windows\Fonts\arialbd.ttf"):
        try:
            probe = ImageFont.truetype(path, 100)
        except OSError:
            continue
        _, y0, _, y1 = probe.getbbox("H")
        return ImageFont.truetype(path, max(8, int(round(100 * cap_px / (y1 - y0)))))
    raise SystemExit("no bold system font found")


def font_for_label(cap_px: float) -> ImageFont.FreeTypeFont:
    for path in (r"C:\Windows\Fonts\segoeui.ttf", r"C:\Windows\Fonts\arial.ttf"):
        try:
            probe = ImageFont.truetype(path, 100)
        except OSError:
            continue
        _, y0, _, y1 = probe.getbbox("L1")
        return ImageFont.truetype(path, max(7, int(round(100 * cap_px / (y1 - y0)))))
    raise SystemExit("no system font found")


def components(mask: np.ndarray, min_area: int) -> list[tuple[np.ndarray, tuple[int, int, int, int]]]:
    seen = np.zeros(mask.shape, bool)
    out = []
    for sy, sx in zip(*np.where(mask)):
        if seen[sy, sx]:
            continue
        stack = [(sy, sx)]
        seen[sy, sx] = True
        pts = []
        while stack:
            y, x = stack.pop()
            pts.append((y, x))
            for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                ny, nx = y + dy, x + dx
                if 0 <= ny < mask.shape[0] and 0 <= nx < mask.shape[1] and mask[ny, nx] and not seen[ny, nx]:
                    seen[ny, nx] = True
                    stack.append((ny, nx))
        if len(pts) < min_area:
            continue
        ys = [p[0] for p in pts]
        xs = [p[1] for p in pts]
        comp = np.zeros(mask.shape, bool)
        comp[tuple(np.array(pts).T)] = True
        out.append((comp, (min(xs), min(ys), max(xs), max(ys))))
    return out


def fill_holes(mask: np.ndarray) -> np.ndarray:
    h, w = mask.shape
    outside = np.zeros(mask.shape, bool)
    stack = [(y, x) for x in range(w) for y in (0, h - 1) if not mask[y, x]]
    stack += [(y, x) for y in range(h) for x in (0, w - 1) if not mask[y, x]]
    while stack:
        y, x = stack.pop()
        if outside[y, x] or mask[y, x]:
            continue
        outside[y, x] = True
        for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            ny, nx = y + dy, x + dx
            if 0 <= ny < h and 0 <= nx < w and not outside[ny, nx] and not mask[ny, nx]:
                stack.append((ny, nx))
    return ~outside


def opened(mask: np.ndarray) -> np.ndarray:
    im = Image.fromarray((mask * 255).astype(np.uint8))
    im = im.filter(ImageFilter.MinFilter(3)).filter(ImageFilter.MaxFilter(3))
    return np.array(im) > 128


def dilated(mask: np.ndarray) -> np.ndarray:
    im = Image.fromarray((mask * 255).astype(np.uint8)).filter(ImageFilter.MaxFilter(3))
    return np.array(im) > 128


def grow_into(seed: np.ndarray, allowed: np.ndarray) -> np.ndarray:
    lab, _ = ndimage.label(allowed, structure=np.ones((3, 3), int))
    keep = np.unique(lab[seed & (lab > 0)])
    return seed | np.isin(lab, keep[keep > 0])


def trace_boundary(mask: np.ndarray) -> list[tuple[float, float]]:
    ys, xs = np.where(mask)
    start = (int(ys.min()), int(xs[ys == ys.min()].min()))
    h, w = mask.shape
    nbr = [(0, -1), (-1, -1), (-1, 0), (-1, 1), (0, 1), (1, 1), (1, 0), (1, -1)]
    cur, back = start, 0
    out = [(start[1] + 0.5, start[0] + 0.5)]
    seen = {(cur, back)}
    while True:
        step = None
        for k in range(1, 9):
            i = (back + k) % 8
            ny, nx = cur[0] + nbr[i][0], cur[1] + nbr[i][1]
            if 0 <= ny < h and 0 <= nx < w and mask[ny, nx]:
                step = ((ny, nx), (i + 4) % 8)
                break
        if step is None or step in seen:
            break
        cur, back = step
        seen.add(step)
        out.append((cur[1] + 0.5, cur[0] + 0.5))
    return out


def smooth_closed(pts: list[tuple[float, float]], sigma: float) -> np.ndarray:
    a = np.asarray(pts, float)
    rad = max(1, int(round(sigma * 3)))
    if len(a) < rad * 2 + 4:
        return a
    k = np.exp(-0.5 * (np.arange(-rad, rad + 1) / sigma) ** 2)
    k /= k.sum()
    out = np.empty_like(a)
    for i in (0, 1):
        wrapped = np.concatenate([a[-rad:, i], a[:, i], a[:rad, i]])
        out[:, i] = np.convolve(wrapped, k, mode="valid")
    return out


def trace_masks() -> tuple[np.ndarray, np.ndarray, np.ndarray, np.ndarray]:
    arr = np.array(Image.open(TARGET).convert("RGB")).astype(np.int16)
    mx = arr.max(2)
    mn = arr.min(2)
    sat = mx - mn

    white = (mn >= BODY_MIN) & (sat <= BODY_SAT)
    body = fill_holes(max(components(white, 2000), key=lambda c: c[0].sum())[0])

    outside = opened((mx <= OUT_MAX) & (sat <= OUT_SAT))
    x0, y0, x1, y1 = WINDOW
    outer = np.zeros(body.shape, bool)
    for comp, (cx0, cy0, cx1, cy1) in components(outside, KEEP_AREA):
        if x0 <= cx0 and cx1 <= x1 and y0 <= cy0 and cy1 <= y1:
            outer |= fill_holes(comp)
    soft = (mx <= IN_MAX) & (sat <= IN_SAT)
    inner = soft & body
    outer = grow_into(outer, soft & fill_holes(body | outer))
    outer = dilated(outer)
    inner = dilated(inner)

    gy, gx = np.ogrid[: body.shape[0], : body.shape[1]]
    disc = (gx - T_GUIDE[0]) ** 2 + (gy - T_GUIDE[1]) ** 2 <= (GUIDE_R - 2) ** 2
    guide_x = outside & disc
    outer &= ~disc
    return body, outer, inner, guide_x


class Art:
    def __init__(self, silhouette: np.ndarray):
        ys, xs = np.where(silhouette)
        self.bx0, self.by0 = int(xs.min()), int(ys.min())
        bw = int(xs.max()) - self.bx0 + 1
        bh = int(ys.max()) - self.by0 + 1
        self.k = min((W - MARGIN * 2) / bw, (H - MARGIN * 2) / bh)
        self.w = bw * self.k
        self.h = bh * self.k
        self.x0 = (W - self.w) / 2
        self.y0 = (H - self.h) / 2
        self.box = (self.bx0, self.by0, self.bx0 + bw, self.by0 + bh)

    def pt(self, x: float, y: float) -> tuple[float, float]:
        return (self.x0 + (x - self.bx0) * self.k, self.y0 + (y - self.by0) * self.k)

    def rect(self, box: tuple[float, float, float, float]) -> tuple[float, float, float, float]:
        x, y, w, h = box
        ax, ay = self.pt(x, y)
        return (ax, ay, w * self.k, h * self.k)

    def layer(self, mask: np.ndarray, smooth: bool = True) -> Image.Image:
        src = Image.fromarray((mask * 255).astype(np.uint8)).crop(self.box)
        if smooth:
            src = src.filter(ImageFilter.MedianFilter(5))
        src = src.filter(ImageFilter.GaussianBlur(SMOOTH))
        hi = src.resize((max(1, int(self.w * S)), max(1, int(self.h * S))), Image.Resampling.BICUBIC)
        out = Image.new("L", (W * S, H * S), 0)
        out.paste(hi, (int(round(self.x0 * S)), int(round(self.y0 * S))))
        return out.point(lambda v: 255 if v > 127 else 0)

    def contour_layer(self, mask: np.ndarray, min_area: int = 200) -> Image.Image:
        out = Image.new("L", (W * S, H * S), 0)
        d = ImageDraw.Draw(out)
        for comp, _ in components(mask, min_area):
            poly = smooth_closed(trace_boundary(comp), CONTOUR_SIGMA)
            if len(poly) < 3:
                continue
            d.polygon([(x * S, y * S) for x, y in (self.pt(px, py) for px, py in poly)], fill=255)
        return out


def finalize(body: Image.Image) -> Image.Image:
    out = body.resize((W, H), Image.Resampling.LANCZOS)
    arr = np.array(out)
    a = arr[..., 3]
    arr[a > 220, 3] = 255
    arr[a < 6, 3] = 0
    return Image.fromarray(arr)


def ring_radius(mask: np.ndarray, cx: float, cy: float, hi: float) -> float:
    ts = np.linspace(0, 2 * np.pi, 240, endpoint=False)
    best = 1.0
    r = 1.0
    while r <= hi:
        xs = np.clip(np.round(cx + r * np.cos(ts)).astype(int), 0, mask.shape[1] - 1)
        ys = np.clip(np.round(cy + r * np.sin(ts)).astype(int), 0, mask.shape[0] - 1)
        if mask[ys, xs].mean() >= 0.5:
            best = r
        r += 0.5
    return best


def blob_box(mask: np.ndarray, x: float, y: float) -> tuple[int, int, int, int]:
    sy, sx = int(round(y)), int(round(x))
    seen = np.zeros(mask.shape, bool)
    stack = [(sy, sx)]
    seen[sy, sx] = True
    pts = []
    while stack:
        py, px_ = stack.pop()
        pts.append((py, px_))
        for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            ny, nx = py + dy, px_ + dx
            if 0 <= ny < mask.shape[0] and 0 <= nx < mask.shape[1] and mask[ny, nx] and not seen[ny, nx]:
                seen[ny, nx] = True
                stack.append((ny, nx))
    ys = [p[0] for p in pts]
    xs = [p[1] for p in pts]
    return (min(xs), min(ys), max(xs), max(ys))


def fmt_uv(box: tuple[float, float, float, float]) -> str:
    x, y, w, h = box
    return f"[{x:.1f} / {W}.0, {y:.1f} / {H}.0, {w:.1f} / {W}.0, {h:.1f} / {H}.0]"


def fmt_pt(p: tuple[float, float]) -> str:
    return f"[{p[0]:.1f} / {W}.0, {p[1]:.1f} / {H}.0]"


def print_layout(art: Art, ink: np.ndarray) -> None:
    pad = 1.5
    ls, rs = art.pt(*T_LS), art.pt(*T_RS)
    dpad_c = art.pt(*T_DPAD)
    guide = art.pt(*T_GUIDE)
    faces = {k: art.pt(*v) for k, v in T_FACE.items()}

    well_r = ring_radius(ink, *ls, WELL_R * art.k + 12) + pad
    face_r = ring_radius(ink, *faces["A"], FACE_R * art.k + 12) + pad
    dot_r = ring_radius(ink, *art.pt(*T_VIEW), DOT_R * art.k + 12) + pad
    dx0, dy0, dx1, dy1 = blob_box(ink, *dpad_c)
    arm = (dx1 - dx0 + 1) / 2
    bar = int(ink[dy0 : dy1 + 1, dx0 + int(arm * 0.5)].sum()) + 2 * pad

    def dot(c):
        x, y = art.pt(*c)
        return (x - dot_r, y - dot_r, dot_r * 2, dot_r * 2)

    def grown(box):
        x, y, w, h = art.rect(box)
        return (x - 3.0, y - 3.0, w + 6.0, h + 6.0)

    seg = {
        "up": (dpad_c[0] - bar / 2, dpad_c[1] - arm, bar, arm),
        "right": (dpad_c[0], dpad_c[1] - bar / 2, arm, bar),
        "down": (dpad_c[0] - bar / 2, dpad_c[1], bar, arm),
        "left": (dpad_c[0] - arm, dpad_c[1] - bar / 2, arm, bar),
    }

    print(f"art {W}x{H} scale {art.k:.4f}")
    print(f"  ls: {fmt_pt(ls)}")
    print(f"  rs: {fmt_pt(rs)}")
    print(f"  well_r: {well_r:.1f}")
    print(f"  cap_r: {CAP_R * art.k + pad:.1f}")
    print(f"  l2: {fmt_uv(grown(HORN_L))}")
    print(f"  r2: {fmt_uv(grown(HORN_R))}")
    print(f"  l1: {fmt_uv(grown(BUMP_L))}")
    print(f"  r1: {fmt_uv(grown(BUMP_R))}")
    print(f"  l2_seed: {fmt_pt(art.pt(HORN_L[0] + HORN_L[2] / 2, HORN_L[1] + HORN_L[3] * 0.5))}")
    print(f"  r2_seed: {fmt_pt(art.pt(HORN_R[0] + HORN_R[2] / 2, HORN_R[1] + HORN_R[3] * 0.5))}")
    print(f"  l1_seed: {fmt_pt(art.pt(BUMP_L[0] + 60, BUMP_L[1] + 22))}")
    print(f"  r1_seed: {fmt_pt(art.pt(BUMP_R[0] + 73, BUMP_R[1] + 22))}")
    for k in ("Y", "B", "A", "X"):
        print(f"  face {k}: {fmt_pt(faces[k])}")
    print(f"  face_r: {face_r:.1f}")
    for k, v in seg.items():
        print(f"  dpad_seg {k}: {fmt_uv(v)}")
    print(f"  dpad: {fmt_pt(dpad_c)}")
    print(f"  dpad_half: {arm:.1f}")
    print(f"  back: {fmt_uv(dot(T_VIEW))}")
    print(f"  start: {fmt_uv(dot(T_MENU))}")
    print(f"  guide: {fmt_pt(guide)}")
    print(f"  guide_r: {GUIDE_R * art.k + pad:.1f}")
