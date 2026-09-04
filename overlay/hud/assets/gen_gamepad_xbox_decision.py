"""Decision comps for Xbox pad art — aligned to gamepad-xbox-ref.png."""

from __future__ import annotations

from pathlib import Path

from PIL import Image, ImageDraw

from gen_gamepad_xbox import (
    FILL,
    STROKE,
    W,
    WELL,
    font,
    layout_constants,
    outline_mask,
    ref_body_bbox,
    rounded,
    silhouette_from_ref,
)
from gen_gamepad_xbox import REF  # noqa: E402

ORANGE = (255, 148, 48, 255)

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / ".impeccable" / "mocks" / "decision"
COMP_W, COMP_H = 960, 540
BG = (10, 10, 10, 255)
LABEL = (136, 136, 138, 255)
S = 4


def draw_pad(stick_out: bool = True, press_a: bool = True) -> Image.Image:
    from PIL import Image

    ref = Image.open(REF)
    ref_bbox = ref_body_bbox(ref)
    layout = layout_constants()
    mask = silhouette_from_ref(ref, ref_bbox)
    sw, sh = W * S, 1024 * S
    mask = mask.resize((sw, sh), Image.Resampling.NEAREST)

    body = Image.new("RGBA", (sw, sh), (0, 0, 0, 0))
    body.paste(FILL, mask=mask)
    edge = outline_mask(mask, 6)
    stroke = Image.new("RGBA", (sw, sh), (0, 0, 0, 0))
    stroke.paste(STROKE, mask=edge)
    body = Image.alpha_composite(body, stroke)
    d = ImageDraw.Draw(body)
    st = 6

    def ring(cx, cy, r, width=st):
        d.ellipse([(cx - r) * S, (cy - r) * S, (cx + r) * S, (cy + r) * S], outline=STROKE, width=width)

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

    def bumper(box, text):
        x, y, w, h = box
        rounded(d, [x * S, y * S, (x + w) * S, (y + h) * S], h * S * 0.42, outline=STROKE, width=st)
        label(text, x + w / 2, y + h / 2, 16)

    LT, RT, LB, RB = layout["LT"], layout["RT"], layout["LB"], layout["RB"]
    for box, text, italic in ((LT, "LT", True), (RT, "RT", True), (LB, "LB", False), (RB, "RB", False)):
        x, y, w, h = box
        label(text, x + w / 2, y + h * (0.42 if italic else 0.5), 34 if italic else 16, italic=italic)

    lx, ly = layout["LS"]
    rx, ry = layout["RS"]
    if stick_out:
        lx -= 28
        ly -= 36
    for cx, cy, r in ((lx, ly, layout["LS_R"]), (rx, ry, layout["RS_R"])):
        fill_circle(cx, cy, r, WELL)
        ring(cx, cy, r)
        ring(cx, cy, int(r * 0.46), width=4)

    for key, glyph in (("FACE_Y", "Y"), ("FACE_B", "B"), ("FACE_A", "A"), ("FACE_X", "X")):
        cx, cy = layout[key]
        col = ORANGE if press_a and glyph == "A" else FILL
        fill_circle(cx, cy, layout["FACE_R"], col)
        ring(cx, cy, layout["FACE_R"])
        label(glyph, cx, cy + 1, 28)

    px, py = layout["DPAD"]
    ring(px, py, layout["DPAD_R"])
    from gen_gamepad_xbox import DPAD_ARM, DPAD_BAR

    rounded(d, [(px - DPAD_BAR / 2) * S, (py - DPAD_ARM) * S, (px + DPAD_BAR / 2) * S, (py + DPAD_ARM) * S], 12 * S, fill=WELL, outline=STROKE, width=st)
    rounded(d, [(px - DPAD_ARM) * S, (py - DPAD_BAR / 2) * S, (px + DPAD_ARM) * S, (py + DPAD_BAR / 2) * S], 12 * S, fill=WELL, outline=STROKE, width=st)

    pill(layout["VIEW"], "VIEW")
    pill(layout["MENU"], "MENU")
    gx, gy = layout["GUIDE"]
    gr = layout["GUIDE_R"]
    fill_circle(gx, gy, gr, FILL)
    ring(gx, gy, gr)
    d.arc([(gx - gr * 0.55) * S, (gy - gr * 0.55) * S, (gx + gr * 0.55) * S, (gy + gr * 0.55) * S], 200, 340, fill=LABEL, width=st)
    d.arc([(gx - gr * 0.55) * S, (gy - gr * 0.55) * S, (gx + gr * 0.55) * S, (gy + gr * 0.55) * S], 20, 160, fill=LABEL, width=st)

    return body.resize((W, 1024), Image.Resampling.LANCZOS)


def frame(pad: Image.Image, title: str) -> Image.Image:
    comp = Image.new("RGBA", (COMP_W, COMP_H), BG)
    alpha = pad.split()[3]
    bbox = alpha.getbbox()
    if bbox:
        crop = pad.crop(bbox)
        tw = int(COMP_W * 0.72)
        scale = tw / crop.width
        th = int(crop.height * scale)
        crop = crop.resize((tw, th), Image.Resampling.LANCZOS)
        comp.paste(crop, ((COMP_W - tw) // 2, (COMP_H - th) // 2 + 10), crop)
    d = ImageDraw.Draw(comp)
    d.text((32, 28), title, font=font(22), fill=(228, 228, 230, 255))
    d.text((32, 56), "Measured from your reference outline", font=font(14), fill=(132, 132, 138, 255))
    return comp


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    pad = draw_pad()
    for slug, title in (
        ("gamepad-xbox-twin", "Twin"),
        ("gamepad-xbox-horns", "Horns"),
        ("gamepad-xbox-over-game", "Over game"),
    ):
        comp = frame(pad, title)
        path = OUT / f"{slug}.png"
        comp.save(path, "PNG")
        print("wrote", path)


if __name__ == "__main__":
    main()
