#include "draw_list.h"
#include "font_atlas.h"
#include "font8x8.h"

#include <algorithm>
#include <cmath>
#include <cstdio>
#include <cstring>

void DrawList::clear()
{
    quads.clear();
    strings.clear();
}

void DrawList::addQuad(float x0, float y0, float x1, float y1, unsigned long color, int sprite)
{
    SPluginQuad_t q{};
    q.m_aafPos[0][0] = x0; q.m_aafPos[0][1] = y0;
    q.m_aafPos[1][0] = x0; q.m_aafPos[1][1] = y1;
    q.m_aafPos[2][0] = x1; q.m_aafPos[2][1] = y1;
    q.m_aafPos[3][0] = x1; q.m_aafPos[3][1] = y0;
    q.m_iSprite = sprite;
    q.m_ulColor = color;
    quads.push_back(q);
}

void DrawList::addQuadPts(const float pts[4][2], unsigned long color)
{
    SPluginQuad_t q{};
    for (int i = 0; i < 4; ++i)
    {
        q.m_aafPos[i][0] = pts[i][0];
        q.m_aafPos[i][1] = pts[i][1];
    }
    q.m_iSprite = 0;
    q.m_ulColor = color;
    quads.push_back(q);
}

void DrawList::addLine(float x0, float y0, float x1, float y1, float halfW, unsigned long color, int sprite)
{
    const float dx = x1 - x0;
    const float dy = y1 - y0;
    const float len = std::sqrt(dx * dx + dy * dy);
    if (len < 1.0e-6f)
    {
        addDot(x0, y0, halfW, color, sprite);
        return;
    }
    const float step = std::max(halfW * 0.40f, 0.00115f);
    const int n = std::max(1, static_cast<int>(std::ceil(len / step)));
    const float inv = 1.0f / static_cast<float>(n);
    for (int i = 0; i <= n; ++i)
    {
        const float t = static_cast<float>(i) * inv;
        addDot(x0 + dx * t, y0 + dy * t, halfW, color, sprite);
    }
}

void DrawList::addDot(float x, float y, float r, unsigned long color, int sprite)
{
    if (sprite != 0)
    {
        addQuad(x - r, y - r, x + r, y + r, color, sprite);
        return;
    }
    const float a = r * 0.42f;
    const float b = r * 0.78f;
    addQuad(x - r, y - a, x + r, y + a, color);
    addQuad(x - a, y - r, x + a, y + r, color);
    addQuad(x - b, y - b, x + b, y + b, color);
}

void DrawList::append(const std::vector<SPluginQuad_t>& extra)
{
    quads.insert(quads.end(), extra.begin(), extra.end());
}

void DrawList::addBorder(float x0, float y0, float x1, float y1, float t, unsigned long color)
{
    addQuad(x0, y0, x1, y0 + t, color);
    addQuad(x0, y1 - t, x1, y1, color);
    addQuad(x0, y0, x0 + t, y1, color);
    addQuad(x1 - t, y0, x1, y1, color);
}

void DrawList::addString(const char* text, float x, float y, float size, int justify, unsigned long color)
{
    if (!text || !text[0] || size <= 0.0f)
    {
        return;
    }

    FontAtlas& atlas = FontAtlas::get();
    if (atlas.ensure())
    {
        const float px = size / static_cast<float>(atlas.cell());
        float cursor = x;
        if (justify == 1)
        {
            cursor -= atlas.measure(text, size) * 0.5f;
        }
        else if (justify == 2)
        {
            cursor -= atlas.measure(text, size);
        }

        const unsigned long rgb = color & 0x00FFFFFF;
        const unsigned long srcA = (color >> 24) & 0xFFul;

        for (const char* p = text; *p; ++p)
        {
            const AaGlyph& g = atlas.glyph(static_cast<unsigned char>(*p));
            const float gx = cursor + static_cast<float>(g.left) * px;
            const float gy = y + static_cast<float>(g.top) * px;
            for (int row = 0; row < g.h; ++row)
            {
                int col = 0;
                while (col < g.w)
                {
                    const std::uint8_t cov = g.pixels[static_cast<size_t>(row * g.w + col)];
                    if (cov < 12)
                    {
                        ++col;
                        continue;
                    }
                    const int start = col;
                    int sum = cov;
                    int run = 1;
                    ++col;
                    while (col < g.w)
                    {
                        const std::uint8_t c2 = g.pixels[static_cast<size_t>(row * g.w + col)];
                        int d = static_cast<int>(c2) - static_cast<int>(cov);
                        if (d < 0)
                        {
                            d = -d;
                        }
                        if (c2 < 12 || d > 28)
                        {
                            break;
                        }
                        sum += c2;
                        ++run;
                        ++col;
                    }
                    const unsigned long a = (srcA * static_cast<unsigned long>(sum / run)) / 255ul;
                    addQuad(gx + start * px,
                            gy + row * px,
                            gx + col * px,
                            gy + (row + 1) * px,
                            rgb | (a << 24));
                }
            }
            cursor += static_cast<float>(g.advance) * px;
        }
        return;
    }

    const float px = size / 8.0f;
    float cursor = x;
    if (justify == 1)
    {
        cursor -= measureText(text, size) * 0.5f;
    }
    else if (justify == 2)
    {
        cursor -= measureText(text, size);
    }

    for (const char* p = text; *p; ++p)
    {
        const unsigned char ch = static_cast<unsigned char>(*p);
        const std::uint8_t* g = glyph8x8(ch);
        for (int row = 0; row < 8; ++row)
        {
            int col = 0;
            while (col < 8)
            {
                if ((g[row] & (1u << col)) == 0)
                {
                    ++col;
                    continue;
                }
                const int start = col;
                while (col < 8 && (g[row] & (1u << col)))
                {
                    ++col;
                }
                addQuad(cursor + start * px,
                        y + row * px,
                        cursor + col * px,
                        y + (row + 1) * px,
                        color);
            }
        }
        cursor += static_cast<float>(glyphAdvancePx(ch)) * px;
    }
}

void DrawList::addPanel(float x, float y, float w, float h)
{
    addQuad(x, y, x + w, y + h, Palette::kPanelBg);
    addBorder(x, y, x + w, y + h, 0.0012f, Palette::kPanelBorder);
}

void DrawList::addHeader(float x, float y, float w, const char* title, const char* subtitle)
{
    addQuad(x, y, x + w, y + kHeaderH, Palette::kHeaderBg);
    addQuad(x, y + kHeaderH - 0.0018f, x + w, y + kHeaderH, Palette::kHeaderLine);
    addString(title, x + 0.010f, y + 0.006f, 0.0145f, 0, Palette::kAccent);
    if (subtitle && subtitle[0])
    {
        addString(subtitle, x + w - 0.010f, y + 0.007f, 0.0125f, 2, Palette::kTextDim);
    }
}

void formatGap(int gapMs, int gapLaps, char* out, int outSize)
{
    if (!out || outSize <= 0)
    {
        return;
    }
    if (gapLaps != 0)
    {
        std::snprintf(out, outSize, "+%dL", gapLaps);
        return;
    }
    if (gapMs <= 0)
    {
        std::snprintf(out, outSize, "---");
        return;
    }
    const float sec = gapMs / 1000.0f;
    if (sec >= 60.0f)
    {
        const int m = static_cast<int>(sec / 60.0f);
        std::snprintf(out, outSize, "+%d:%04.1f", m, sec - m * 60.0f);
    }
    else
    {
        std::snprintf(out, outSize, "+%.3f", sec);
    }
}

void formatEstGap(float seconds, char* out, int outSize)
{
    if (!out || outSize <= 0)
    {
        return;
    }
    const float a = std::fabs(seconds);
    if (seconds < 0.0f)
    {
        std::snprintf(out, outSize, "-%.2f", a);
    }
    else
    {
        std::snprintf(out, outSize, "+%.2f", a);
    }
}

void truncateCopy(char* dest, size_t destSize, const char* src, int maxChars)
{
    if (!dest || destSize == 0)
    {
        return;
    }
    dest[0] = '\0';
    if (!src)
    {
        return;
    }
    int n = 0;
    while (src[n] && n < maxChars && static_cast<size_t>(n) + 1 < destSize)
    {
        dest[n] = src[n];
        ++n;
    }
    dest[n] = '\0';
}
