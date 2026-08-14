"""Generate PiBoSo HUD assets: AA .fnt plus white/circle TGA sprites."""
from __future__ import annotations

import ctypes
import math
import pathlib
import struct
import zlib
from ctypes import wintypes

GGO_GRAY8_BITMAP = 6
GDI_ERROR = 0xFFFFFFFF
ANTIALIASED_QUALITY = 4
OUT_TT_PRECIS = 4
CLIP_DEFAULT_PRECIS = 0
FW_SEMIBOLD = 600
ANSI_CHARSET = 0
VARIABLE_PITCH = 2
FF_SWISS = 32


class POINT(ctypes.Structure):
    _fields_ = [("x", ctypes.c_long), ("y", ctypes.c_long)]


class GLYPHMETRICS(ctypes.Structure):
    _fields_ = [
        ("gmBlackBoxX", wintypes.UINT),
        ("gmBlackBoxY", wintypes.UINT),
        ("gmptGlyphOrigin", POINT),
        ("gmCellIncX", ctypes.c_short),
        ("gmCellIncY", ctypes.c_short),
    ]


class FIXED(ctypes.Structure):
    _fields_ = [("fract", wintypes.WORD), ("value", ctypes.c_short)]


class MAT2(ctypes.Structure):
    _fields_ = [
        ("eM11", FIXED),
        ("eM12", FIXED),
        ("eM21", FIXED),
        ("eM22", FIXED),
    ]


gdi32 = ctypes.windll.gdi32
gdi32.GetGlyphOutlineW.restype = ctypes.c_uint
gdi32.GetGlyphOutlineW.argtypes = [
    wintypes.HDC,
    ctypes.c_uint,
    ctypes.c_uint,
    ctypes.POINTER(GLYPHMETRICS),
    ctypes.c_uint,
    ctypes.c_void_p,
    ctypes.POINTER(MAT2),
]


def write_tga32(path: pathlib.Path, width: int, height: int, pixels_rgba: bytes) -> None:
    header = bytearray(18)
    header[2] = 2
    header[12] = width & 255
    header[13] = (width >> 8) & 255
    header[14] = height & 255
    header[15] = (height >> 8) & 255
    header[16] = 32
    header[17] = 8
    bgra = bytearray(width * height * 4)
    for i in range(width * height):
        r, g, b, a = pixels_rgba[i * 4 : i * 4 + 4]
        # TGA is bottom-up BGRA.
        y = i // width
        x = i % width
        j = ((height - 1 - y) * width + x) * 4
        bgra[j : j + 4] = bytes((b, g, r, a))
    path.write_bytes(bytes(header) + bytes(bgra))


def make_white_tga(path: pathlib.Path) -> None:
    n = 16
    pix = bytes([255, 255, 255, 255]) * (n * n)
    write_tga32(path, n, n, pix)


def make_circle_tga(path: pathlib.Path) -> None:
    n = 128
    cx = cy = (n - 1) * 0.5
    radius = n * 0.5 - 1.5
    pix = bytearray(n * n * 4)
    for y in range(n):
        for x in range(n):
            d = math.hypot(x - cx, y - cy)
            edge = radius - d
            if edge >= 1.0:
                a = 255
            elif edge <= 0.0:
                a = 0
            else:
                a = int(255.0 * edge)
            o = (y * n + x) * 4
            pix[o : o + 4] = bytes((255, 255, 255, a))
    write_tga32(path, n, n, bytes(pix))


def rasterize_font(cell: int = 48, atlas: int = 512, spacing: int = 10):
    dc = gdi32.CreateCompatibleDC(None)
    if not dc:
        raise RuntimeError("CreateCompatibleDC failed")

    font = gdi32.CreateFontW(
        -cell,
        0,
        0,
        0,
        FW_SEMIBOLD,
        False,
        False,
        False,
        ANSI_CHARSET,
        OUT_TT_PRECIS,
        CLIP_DEFAULT_PRECIS,
        ANTIALIASED_QUALITY,
        VARIABLE_PITCH | FF_SWISS,
        "Segoe UI",
    )
    if not font:
        raise RuntimeError("CreateFontW failed")
    old = gdi32.SelectObject(dc, font)

    mat = MAT2()
    mat.eM11.value = 1
    mat.eM22.value = 1

    glyphs = [(False, 0, 0, 0, 0, 0, 0, 0)] * 256
    bitmap = bytearray(atlas * atlas)
    px = spacing
    py = spacing
    placed = 0

    for code in range(32, 127):
        gm = GLYPHMETRICS()
        size = gdi32.GetGlyphOutlineW(dc, code, GGO_GRAY8_BITMAP, ctypes.byref(gm), 0, None, ctypes.byref(mat))
        adv = int(gm.gmCellIncX) if gm.gmCellIncX else cell // 2
        if code == 32:
            glyphs[code] = (True, 0, 0, adv, px, px, py, py + cell)
            continue
        if size == GDI_ERROR or size == 0 or gm.gmBlackBoxX == 0:
            glyphs[code] = (True, 0, 0, adv, px, px, py, py + cell)
            continue

        buf = (ctypes.c_ubyte * size)()
        if gdi32.GetGlyphOutlineW(dc, code, GGO_GRAY8_BITMAP, ctypes.byref(gm), size, buf, ctypes.byref(mat)) == GDI_ERROR:
            glyphs[code] = (True, 0, 0, adv, px, px, py, py + cell)
            continue

        gw = int(gm.gmBlackBoxX)
        gh = int(gm.gmBlackBoxY)
        stride = (gw + 3) & ~3
        xoff = int(gm.gmptGlyphOrigin.x)
        width = gw
        rb = adv - xoff - width
        if px + width + spacing > atlas:
            px = spacing
            py += cell + spacing
        if py + cell + spacing > atlas:
            raise RuntimeError("font atlas overflow")

        origin_y = int(gm.gmptGlyphOrigin.y)
        # Place ink so glyph origin sits on the baseline near 0.78 of the cell.
        baseline = int(cell * 0.78)
        top = baseline - origin_y
        for y in range(gh):
            ay = py + top + y
            if ay < py or ay >= py + cell or ay >= atlas:
                continue
            for x in range(gw):
                ax = px + x
                if ax < 0 or ax >= atlas:
                    continue
                v = buf[y * stride + x]
                bitmap[ay * atlas + ax] = min(255, (int(v) * 255 + 32) // 64)
        glyphs[code] = (True, xoff, width, rb, px, px + width, py, py + cell)
        px += width + spacing
        placed += 1

    gdi32.SelectObject(dc, old)
    gdi32.DeleteObject(font)
    gdi32.DeleteDC(dc)
    return cell, atlas, glyphs, bitmap, placed


def write_fnt(path: pathlib.Path, name: str, cell: int, atlas: int, glyphs, bitmap: bytes) -> None:
    header = bytearray(10508 + 24)
    header[0:4] = b"FNT\0"
    encoded = name.encode("ascii", "ignore")[:255]
    header[4 : 4 + len(encoded)] = encoded
    struct.pack_into("<i", header, 264, cell)
    for code in range(256):
        valid, xoff, width, rb, ax0, ax1, ay0, ay1 = glyphs[code]
        if not valid:
            continue
        o = 268 + code * 40
        struct.pack_into("<10i", header, o, 1, xoff, width, rb, ax0, ax1, ay0, ay1, 0, 0)

    compressor = zlib.compressobj(9, zlib.DEFLATED, -15)
    payload = compressor.compress(bytes(bitmap)) + compressor.flush()
    struct.pack_into("<i", header, 10508, 0)
    struct.pack_into("<i", header, 10512, atlas)
    struct.pack_into("<i", header, 10516, atlas)
    struct.pack_into("<i", header, 10524, 2)
    struct.pack_into("<i", header, 10528, 0)
    data = bytes(header) + payload
    struct.pack_into("<i", header, 10520, len(data) - 10524)
    path.write_bytes(bytes(header) + payload)


def main() -> None:
    root = pathlib.Path(__file__).resolve().parents[1]
    fonts = root / "assets" / "fonts"
    fonts.mkdir(parents=True, exist_ok=True)
    make_white_tga(root / "assets" / "white.tga")
    make_circle_tga(root / "assets" / "circle.tga")
    cell, atlas, glyphs, bitmap, placed = rasterize_font()
    out = fonts / "hud.fnt"
    write_fnt(out, "HUD", cell, atlas, glyphs, bitmap)
    print(f"wrote {out} ({placed} glyphs, cell {cell}px)")
    print("wrote assets/white.tga and assets/circle.tga")


if __name__ == "__main__":
    main()
