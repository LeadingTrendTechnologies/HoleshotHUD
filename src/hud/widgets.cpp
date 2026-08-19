#include "widgets.h"

#include <algorithm>
#include <cmath>
#include <cstdio>
#include <vector>

namespace
{
    const char* riderName(const PluginState& state, int raceNum)
    {
        const RaceEntry* e = state.findEntry(raceNum);
        return e ? e->name.c_str() : "???";
    }

    const char* stateLabel(int s)
    {
        switch (s)
        {
        case 1: return "DNS";
        case 3: return "OUT";
        case 4: return "DSQ";
        default: return nullptr;
        }
    }

    void drawFocusRow(DrawList& dl, float x, float y, float w, float h)
    {
        dl.addQuad(x, y, x + w, y + h, Palette::kLocalRow);
        dl.addQuad(x, y, x + 0.0028f, y + h, Palette::kLocalPip);
    }
}

void drawStandings(DrawList& dl, const PluginState& state, const HudRect& rect, int maxRows)
{
    const auto& rows = state.standings();
    maxRows = std::max(3, maxRows);

    const int n = static_cast<int>(rows.size());
    const int focus = state.focusRaceNum();
    int focusIdx = 0;
    for (int i = 0; i < n; ++i)
    {
        if (rows[i].raceNum == focus)
        {
            focusIdx = i;
            break;
        }
    }

    int visCount = n > 0 ? std::min(n, maxRows) : 1;
    int start = 0;
    if (n > maxRows)
    {
        start = std::max(0, focusIdx - maxRows / 2);
        start = std::min(start, n - maxRows);
    }
    const int end = n > 0 ? std::min(n, start + maxRows) : 0;
    visCount = n > 0 ? end - start : 1;

    const float pad = 0.008f;
    const float colH = 0.018f;
    const float rowH = 0.022f;
    const float h = std::min(rect.h, kHeaderH + colH + static_cast<float>(visCount) * rowH + pad);
    const float font = 0.0145f;

    dl.addPanel(rect.x, rect.y, rect.w, h);
    dl.addHeader(rect.x, rect.y, rect.w, "STANDINGS");

    if (rows.empty())
    {
        dl.addString("Waiting for race data", rect.x + 0.012f, rect.y + kHeaderH + 0.006f, font, 0, Palette::kTextDim);
        return;
    }

    const float headerY = rect.y + kHeaderH + 0.003f;
    const float xPos = rect.x + 0.012f;
    const float xNum = rect.x + 0.036f;
    const float xName = rect.x + 0.062f;
    const float xGap = rect.x + rect.w - 0.012f;

    dl.addString("P", xPos, headerY, 0.0115f, 0, Palette::kTextDim);
    dl.addString("#", xNum, headerY, 0.0115f, 0, Palette::kTextDim);
    dl.addString("NAME", xName, headerY, 0.0115f, 0, Palette::kTextDim);
    dl.addString("GAP", xGap, headerY, 0.0115f, 2, Palette::kTextDim);
    dl.addQuad(rect.x + 0.008f, rect.y + kHeaderH + colH - 0.0012f,
               rect.x + rect.w - 0.008f, rect.y + kHeaderH + colH, Palette::kPanelBorder);

    for (int i = start; i < end; ++i)
    {
        const StandingRow& row = rows[i];
        const int vis = i - start;
        const float y = rect.y + kHeaderH + colH + static_cast<float>(vis) * rowH;
        const bool isFocus = row.raceNum == focus;
        if (isFocus)
        {
            drawFocusRow(dl, rect.x + 0.002f, y, rect.w - 0.004f, rowH - 0.0015f);
        }
        else if (vis % 2)
        {
            dl.addQuad(rect.x + 0.002f, y, rect.x + rect.w - 0.002f, y + rowH - 0.0015f, Palette::kRowAlt);
        }

        char posBuf[8];
        std::snprintf(posBuf, sizeof(posBuf), "%d", row.position);
        char numBuf[8];
        std::snprintf(numBuf, sizeof(numBuf), "%d", row.raceNum);

        char nameBuf[40];
        truncateCopy(nameBuf, sizeof(nameBuf), riderName(state, row.raceNum), 14);

        char gapBuf[16];
        const char* status = stateLabel(row.state);
        if (status)
        {
            std::snprintf(gapBuf, sizeof(gapBuf), "%s", status);
        }
        else if (row.pit)
        {
            std::snprintf(gapBuf, sizeof(gapBuf), "PIT");
        }
        else if (row.position == 1)
        {
            std::snprintf(gapBuf, sizeof(gapBuf), "---");
        }
        else
        {
            formatGap(row.gapMs, row.gapLaps, gapBuf, sizeof(gapBuf));
        }

        const float ty = y + 0.0035f;
        const unsigned long col = isFocus ? Palette::kAccent : Palette::kText;
        dl.addString(posBuf, xPos, ty, font, 0, isFocus ? Palette::kAccent : Palette::kTextDim);
        dl.addString(numBuf, xNum, ty, font, 0, col);
        dl.addString(nameBuf, xName, ty, font, 0, col);
        dl.addString(gapBuf, xGap, ty, font, 2, isFocus ? Palette::kAccent : Palette::kTextDim);
    }
}

void drawRelative(DrawList& dl, const PluginState& state, const HudRect& rect, int countEachSide)
{
    struct Rel
    {
        int raceNum = 0;
        float trackPos = 0.0f;
        float wrapped = 0.0f;
    };

    const int focus = state.focusRaceNum();
    float focusPos = 0.0f;
    bool haveFocus = false;

    std::vector<Rel> riders;
    riders.reserve(state.trackPositions().size());
    for (const auto& p : state.trackPositions())
    {
        Rel r;
        r.raceNum = p.raceNum;
        r.trackPos = p.trackPos;
        riders.push_back(r);
        if (p.raceNum == focus)
        {
            focusPos = p.trackPos;
            haveFocus = true;
        }
    }

    if (!haveFocus && state.hasTelemetry())
    {
        focusPos = state.localTrackPos();
        haveFocus = true;
        if (focus < 0 && riders.empty())
        {
            Rel self;
            self.raceNum = -1;
            self.trackPos = focusPos;
            riders.push_back(self);
        }
    }

    std::vector<int> order;
    if (haveFocus && !riders.empty())
    {
        auto wrapDelta = [](float other, float self) {
            float d = other - self;
            if (d > 0.5f) d -= 1.0f;
            if (d < -0.5f) d += 1.0f;
            return d;
        };

        for (auto& r : riders)
        {
            r.wrapped = wrapDelta(r.trackPos, focusPos);
        }

        std::sort(riders.begin(), riders.end(), [](const Rel& a, const Rel& b) {
            return a.wrapped < b.wrapped;
        });

        int selfIdx = 0;
        for (size_t i = 0; i < riders.size(); ++i)
        {
            if (riders[i].raceNum == focus || (focus < 0 && riders[i].raceNum == -1))
            {
                selfIdx = static_cast<int>(i);
                break;
            }
            if (std::fabs(riders[i].wrapped) < std::fabs(riders[selfIdx].wrapped))
            {
                selfIdx = static_cast<int>(i);
            }
        }

        countEachSide = std::max(1, countEachSide);
        const int n = static_cast<int>(riders.size());
        std::vector<char> used(static_cast<size_t>(n), 0);
        used[static_cast<size_t>(selfIdx)] = 1;

        std::vector<int> ahead;
        for (int k = 1; k <= countEachSide && k < n; ++k)
        {
            const int idx = (selfIdx + k) % n;
            if (!used[static_cast<size_t>(idx)])
            {
                used[static_cast<size_t>(idx)] = 1;
                ahead.push_back(idx);
            }
        }
        for (auto it = ahead.rbegin(); it != ahead.rend(); ++it)
        {
            order.push_back(*it);
        }
        order.push_back(selfIdx);
        for (int k = 1; k <= countEachSide && k < n; ++k)
        {
            const int idx = (selfIdx - k + n) % n;
            if (!used[static_cast<size_t>(idx)])
            {
                used[static_cast<size_t>(idx)] = 1;
                order.push_back(idx);
            }
        }
    }

    const int visCount = order.empty() ? 1 : static_cast<int>(order.size());
    const float pad = 0.008f;
    const float rowH = 0.024f;
    const float h = std::min(rect.h, kHeaderH + static_cast<float>(visCount) * rowH + pad);
    const float font = 0.0145f;

    dl.addPanel(rect.x, rect.y, rect.w, h);
    dl.addHeader(rect.x, rect.y, rect.w, "RELATIVE");

    if (order.empty())
    {
        dl.addString("Waiting for positions", rect.x + 0.012f, rect.y + kHeaderH + 0.006f, font, 0, Palette::kTextDim);
        return;
    }

    const float xNum = rect.x + 0.012f;
    const float xName = rect.x + 0.046f;
    const float xGap = rect.x + rect.w - 0.012f;
    const float trackLen = state.trackLength() > 1.0f ? state.trackLength() : 1.0f;
    const float speed = std::max(state.localSpeed(), 4.0f);

    for (size_t vis = 0; vis < order.size(); ++vis)
    {
        const Rel& r = riders[order[vis]];
        const float y = rect.y + kHeaderH + 0.004f + static_cast<float>(vis) * rowH;
        const bool isSelf = (r.raceNum == focus) || (focus < 0 && r.raceNum == -1);

        if (isSelf)
        {
            drawFocusRow(dl, rect.x + 0.002f, y - 0.0015f, rect.w - 0.004f, rowH - 0.0015f);
        }
        else if (vis % 2)
        {
            dl.addQuad(rect.x + 0.002f, y - 0.0015f, rect.x + rect.w - 0.002f, y + rowH - 0.003f, Palette::kRowAlt);
        }

        char numBuf[8];
        if (r.raceNum >= 0)
        {
            std::snprintf(numBuf, sizeof(numBuf), "%d", r.raceNum);
        }
        else
        {
            std::snprintf(numBuf, sizeof(numBuf), "--");
        }

        char nameBuf[40];
        if (isSelf)
        {
            truncateCopy(nameBuf, sizeof(nameBuf), "YOU", 14);
        }
        else
        {
            truncateCopy(nameBuf, sizeof(nameBuf), riderName(state, r.raceNum), 14);
        }

        char gapBuf[16];
        unsigned long gapCol = Palette::kTextDim;
        if (isSelf)
        {
            std::snprintf(gapBuf, sizeof(gapBuf), "---");
            gapCol = Palette::kAccent;
        }
        else
        {
            float est = (r.wrapped * trackLen) / speed;
            if (std::fabs(est) < 0.01f && r.wrapped != 0.0f)
            {
                est = r.wrapped >= 0.0f ? 0.01f : -0.01f;
            }
            formatEstGap(est, gapBuf, sizeof(gapBuf));
            gapCol = r.wrapped >= 0.0f ? Palette::kAhead : Palette::kBehind;
        }

        const float ty = y + 0.003f;
        const unsigned long col = isSelf ? Palette::kAccent : Palette::kText;
        dl.addString(numBuf, xNum, ty, font, 0, col);
        dl.addString(nameBuf, xName, ty, font, 0, col);
        dl.addString(gapBuf, xGap, ty, font, 2, gapCol);
    }
}
