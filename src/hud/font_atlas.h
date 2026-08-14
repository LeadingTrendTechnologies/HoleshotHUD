#pragma once

#include <cstdint>
#include <vector>

struct AaGlyph
{
    int w = 0;
    int h = 0;
    int left = 0;
    int top = 0;
    int advance = 0;
    std::vector<std::uint8_t> pixels;
};

struct FontAtlas
{
    static FontAtlas& get();
    bool ensure();
    float measure(const char* text, float size) const;
    int cell() const { return m_cell; }
    const AaGlyph& glyph(unsigned char c) const;

private:
    bool rasterize();
    AaGlyph m_glyphs[96];
    AaGlyph m_missing;
    int m_cell = 16;
    int m_ascent = 13;
    bool m_ready = false;
    bool m_tried = false;
};
