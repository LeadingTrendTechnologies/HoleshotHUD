"""Build overlay/icon.ico (BMP small frames + PNG 256) from the demo rider art."""
from __future__ import annotations

import struct
from io import BytesIO
from pathlib import Path

from PIL import Image, ImageEnhance, ImageFilter

ROOT = Path(__file__).resolve().parents[1]
OUT_ICO = ROOT / "overlay" / "icon.ico"
OUT_PNG = ROOT / "overlay" / "icon.png"
OUT_UI = ROOT / "overlay" / "icon-48.png"
WEB_PNG = ROOT / "web" / "logo.png"
WEB_ICO = ROOT / "web" / "favicon.ico"
ART = ROOT / "overlay" / "icon-art.png"
DARK = (18, 16, 16, 255)
SIZES = (16, 20, 24, 32, 40, 48, 64, 256)


def draw_mark(size: int) -> Image.Image:
    from PIL import ImageDraw

    im = Image.new("RGBA", (size, size), DARK)
    d = ImageDraw.Draw(im)
    rad = max(3, size * 16 // 64)
    d.rounded_rectangle((0, 0, size - 1, size - 1), radius=rad, outline=(255, 148, 48, 255), width=max(1, size // 16))
    return im


def load_art(size: int) -> Image.Image:
    if not ART.exists():
        return draw_mark(size)
    art = Image.open(ART).convert("RGBA")
    im = art.resize((size, size), Image.Resampling.LANCZOS)
    if size <= 48:
        im = ImageEnhance.Contrast(im).enhance(1.12)
        im = im.filter(ImageFilter.UnsharpMask(radius=0.8, percent=90, threshold=2))
    return im


def png_bytes(im: Image.Image) -> bytes:
    buf = BytesIO()
    im.save(buf, format="PNG")
    return buf.getvalue()


def dib_bytes(im: Image.Image) -> bytes:
    """32bpp ICO DIB: BITMAPINFOHEADER + BGRA XOR (bottom-up) + 1bpp AND mask."""
    w, h = im.size
    pix = im.load()
    xor = bytearray()
    for y in range(h - 1, -1, -1):
        for x in range(w):
            r, g, b, a = pix[x, y]
            xor.extend((b, g, r, a))
    stride = ((w + 31) // 32) * 4
    mask = bytearray(stride * h)
    for y in range(h - 1, -1, -1):
        row = h - 1 - y
        for x in range(w):
            if pix[x, y][3] < 128:
                mask[row * stride + (x >> 3)] |= 0x80 >> (x & 7)
    header = struct.pack("<IIIHHIIIIII", 40, w, h * 2, 1, 32, 0, len(xor), 0, 0, 0, 0)
    return header + xor + mask


def write_ico(path: Path, frames: dict[int, Image.Image]) -> None:
    sizes = list(frames)
    images: list[bytes] = []
    for n in sizes:
        im = frames[n]
        images.append(png_bytes(im) if n >= 256 else dib_bytes(im))
    count = len(sizes)
    offset = 6 + 16 * count
    buf = bytearray(struct.pack("<HHH", 0, 1, count))
    for n, data in zip(sizes, images):
        buf.extend(
            struct.pack(
                "<BBBBHHII",
                0 if n >= 256 else n,
                0 if n >= 256 else n,
                0,
                0,
                1,
                32,
                len(data),
                offset,
            )
        )
        offset += len(data)
    for data in images:
        buf.extend(data)
    path.write_bytes(buf)


def main() -> None:
    frames = {n: load_art(n) for n in SIZES}
    OUT_ICO.parent.mkdir(parents=True, exist_ok=True)
    write_ico(OUT_ICO, frames)
    frames[256].save(OUT_PNG)
    frames[256].resize((48, 48), Image.Resampling.LANCZOS).save(OUT_UI)
    WEB_PNG.parent.mkdir(parents=True, exist_ok=True)
    frames[256].save(WEB_PNG)
    write_ico(WEB_ICO, frames)
    print(f"wrote {OUT_ICO}, {OUT_PNG}, {OUT_UI}, {WEB_PNG}, and {WEB_ICO}")


if __name__ == "__main__":
    main()
