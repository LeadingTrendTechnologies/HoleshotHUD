#pragma once

#include "vendor/piboso/mxb_api.h"
#include "color.h"
#include "str_util.h"

#include <cstddef>
#include <vector>

constexpr float kHeaderH = 0.026f;
constexpr size_t kDrawQuadReserve = 2048;

struct DrawList
{
    std::vector<SPluginQuad_t> quads;
    std::vector<SPluginString_t> strings;

    void clear();
    void addQuad(float x0, float y0, float x1, float y1, unsigned long color, int sprite = 0);
    void addQuadPts(const float pts[4][2], unsigned long color);
    void addLine(float x0, float y0, float x1, float y1, float halfW, unsigned long color, int sprite = 0);
    void addDot(float x, float y, float r, unsigned long color, int sprite = 0);
    void append(const std::vector<SPluginQuad_t>& extra);
    void addBorder(float x0, float y0, float x1, float y1, float t, unsigned long color);
    void addString(const char* text, float x, float y, float size, int justify, unsigned long color);
    void addPanel(float x, float y, float w, float h);
    void addHeader(float x, float y, float w, const char* title, const char* subtitle = nullptr);
};

void formatGap(int gapMs, int gapLaps, char* out, int outSize);
void formatEstGap(float seconds, char* out, int outSize);
