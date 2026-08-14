#ifndef NOMINMAX
#define NOMINMAX
#endif
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif

#include "font_atlas.h"

#include <algorithm>
#include <cstring>
#include <vector>
#include <windows.h>

FontAtlas& FontAtlas::get()
{
    static FontAtlas atlas;
    return atlas;
}

bool FontAtlas::ensure()
{
    if (m_ready)
    {
        return true;
    }
    if (m_tried)
    {
        return false;
    }
    m_tried = true;
    m_ready = rasterize();
    return m_ready;
}

const AaGlyph& FontAtlas::glyph(unsigned char c) const
{
    if (c < 32 || c > 126)
    {
        c = '?';
    }
    return m_glyphs[c - 32];
}

float FontAtlas::measure(const char* text, float size) const
{
    if (!text || m_cell <= 0)
    {
        return 0.0f;
    }
    const float px = size / static_cast<float>(m_cell);
    float w = 0.0f;
    for (const char* p = text; *p; ++p)
    {
        w += static_cast<float>(glyph(static_cast<unsigned char>(*p)).advance) * px;
    }
    return w;
}

namespace
{
    HFONT makeFont(int height)
    {
        const wchar_t* faces[] = {L"Segoe UI", L"Arial", L"Tahoma"};
        for (const wchar_t* face : faces)
        {
            HFONT font = CreateFontW(
                -height, 0, 0, 0, FW_SEMIBOLD, FALSE, FALSE, FALSE,
                ANSI_CHARSET, OUT_TT_PRECIS, CLIP_DEFAULT_PRECIS,
                ANTIALIASED_QUALITY, VARIABLE_PITCH | FF_SWISS, face);
            if (font)
            {
                return font;
            }
        }
        return nullptr;
    }
}

bool FontAtlas::rasterize()
{
    HDC dc = CreateCompatibleDC(nullptr);
    if (!dc)
    {
        return false;
    }

    HFONT font = makeFont(20);
    if (!font)
    {
        DeleteDC(dc);
        return false;
    }

    HGDIOBJ oldFont = SelectObject(dc, font);
    TEXTMETRICW tm{};
    GetTextMetricsW(dc, &tm);
    m_cell = tm.tmHeight;
    m_ascent = tm.tmAscent;
    if (m_cell < 10)
    {
        m_cell = 20;
        m_ascent = 16;
    }

    MAT2 mat{};
    mat.eM11.value = 1;
    mat.eM22.value = 1;

    const int pad = 3;
    const int bmpW = m_cell * 2 + pad * 2;
    const int bmpH = m_cell + pad * 2;
    BITMAPINFO bmi{};
    bmi.bmiHeader.biSize = sizeof(BITMAPINFOHEADER);
    bmi.bmiHeader.biWidth = bmpW;
    bmi.bmiHeader.biHeight = -bmpH;
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB;
    void* bits = nullptr;
    HBITMAP dib = CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &bits, nullptr, 0);
    HGDIOBJ oldBmp = nullptr;
    if (dib)
    {
        oldBmp = SelectObject(dc, dib);
        SetBkMode(dc, TRANSPARENT);
        SetTextColor(dc, RGB(255, 255, 255));
    }

    int ok = 0;
    for (int c = 32; c <= 126; ++c)
    {
        AaGlyph g;
        GLYPHMETRICS gm{};
        const UINT ch = static_cast<UINT>(c);
        const DWORD bytes = GetGlyphOutlineW(dc, ch, GGO_GRAY8_BITMAP, &gm, 0, nullptr, &mat);
        g.advance = gm.gmCellIncX != 0 ? gm.gmCellIncX : tm.tmAveCharWidth;

        if (c == 32)
        {
            g.advance = std::max(g.advance, m_cell / 3);
            m_glyphs[0] = std::move(g);
            ++ok;
            continue;
        }

        bool got = false;
        if (bytes != GDI_ERROR && bytes > 0 && gm.gmBlackBoxX > 0 && gm.gmBlackBoxY > 0)
        {
            std::vector<std::uint8_t> raw(bytes);
            if (GetGlyphOutlineW(dc, ch, GGO_GRAY8_BITMAP, &gm, bytes, raw.data(), &mat) != GDI_ERROR)
            {
                const int gw = static_cast<int>(gm.gmBlackBoxX);
                const int gh = static_cast<int>(gm.gmBlackBoxY);
                const int stride = (gw + 3) & ~3;
                g.w = gw;
                g.h = gh;
                g.left = gm.gmptGlyphOrigin.x;
                g.top = m_ascent - gm.gmptGlyphOrigin.y;
                g.pixels.resize(static_cast<size_t>(gw * gh));
                for (int y = 0; y < gh; ++y)
                {
                    for (int x = 0; x < gw; ++x)
                    {
                        const int v = raw[static_cast<size_t>(y * stride + x)];
                        g.pixels[static_cast<size_t>(y * gw + x)] =
                            static_cast<std::uint8_t>(std::min(255, (v * 255 + 32) / 64));
                    }
                }
                got = true;
            }
        }

        if (!got && dib && bits)
        {
            std::memset(bits, 0, static_cast<size_t>(bmpW * bmpH * 4));
            wchar_t wc = static_cast<wchar_t>(c);
            ABC abc{};
            GetCharABCWidthsW(dc, wc, wc, &abc);
            g.advance = abc.abcA + static_cast<int>(abc.abcB) + abc.abcC;
            TextOutW(dc, pad - abc.abcA, pad, &wc, 1);
            int minX = bmpW, minY = bmpH, maxX = 0, maxY = 0;
            auto* px = static_cast<const std::uint8_t*>(bits);
            for (int y = 0; y < bmpH; ++y)
            {
                for (int x = 0; x < bmpW; ++x)
                {
                    const std::uint8_t* p = px + (y * bmpW + x) * 4;
                    const int a = (static_cast<int>(p[0]) + p[1] + p[2]) / 3;
                    if (a > 10)
                    {
                        minX = std::min(minX, x);
                        minY = std::min(minY, y);
                        maxX = std::max(maxX, x);
                        maxY = std::max(maxY, y);
                    }
                }
            }
            if (maxX >= minX && maxY >= minY)
            {
                g.w = maxX - minX + 1;
                g.h = maxY - minY + 1;
                g.left = minX - pad;
                g.top = minY - pad;
                g.pixels.resize(static_cast<size_t>(g.w * g.h));
                for (int y = 0; y < g.h; ++y)
                {
                    for (int x = 0; x < g.w; ++x)
                    {
                        const std::uint8_t* p = px + ((minY + y) * bmpW + (minX + x)) * 4;
                        g.pixels[static_cast<size_t>(y * g.w + x)] =
                            static_cast<std::uint8_t>((static_cast<int>(p[0]) + p[1] + p[2]) / 3);
                    }
                }
                got = true;
            }
        }

        if (got)
        {
            ++ok;
        }
        m_glyphs[c - 32] = std::move(g);
    }

    if (oldBmp)
    {
        SelectObject(dc, oldBmp);
    }
    if (dib)
    {
        DeleteObject(dib);
    }
    SelectObject(dc, oldFont);
    DeleteObject(font);
    DeleteDC(dc);
    return ok > 20;
}
