#include "map_hud.h"

#include <algorithm>
#include <cmath>
#include <cstdio>

namespace
{
    constexpr float kPi = 3.14159265358979323846f;
    constexpr float kDeg = 180.0f / kPi;

    float deg2rad(float d)
    {
        return d * kPi / 180.0f;
    }

    double nowSeconds()
    {
        return pluginNowSeconds();
    }

    float wrapDelta(float a, float b, float len)
    {
        float d = a - b;
        const float half = len * 0.5f;
        while (d > half)
        {
            d -= len;
        }
        while (d < -half)
        {
            d += len;
        }
        return d;
    }

    void advanceAlongArc(float& x, float& y, float& angleDeg, float radius, float distance)
    {
        const float r = (std::fabs(radius) < 0.05f) ? 0.0f : radius;
        if (r == 0.0f)
        {
            const float a = deg2rad(angleDeg);
            x += std::sin(a) * distance;
            y += std::cos(a) * distance;
            return;
        }
        const float a = deg2rad(angleDeg);
        const float dTheta = distance / r;
        const float cx = x + std::cos(a) * r;
        const float cy = y - std::sin(a) * r;
        const float n = a + dTheta;
        x = cx - std::cos(n) * r;
        y = cy + std::sin(n) * r;
        angleDeg += dTheta * kDeg;
    }
}

void MapHud::tessellate(const PluginState& state)
{
    m_poly.clear();
    m_dist.clear();
    m_hasSf = false;
    m_ready = false;
    m_hasSmooth = false;

    const auto& segs = state.centerline();
    if (segs.empty())
    {
        tessellateWorld(state);
        return;
    }

    float x = segs[0].startX;
    float y = segs[0].startZ;
    float heading = segs[0].angle;
    float dist = 0.0f;
    auto push = [&](float px, float pz) {
        if (!m_poly.empty())
        {
            const float dx = px - m_poly.back().x;
            const float dz = pz - m_poly.back().z;
            dist += std::sqrt(dx * dx + dz * dz);
        }
        m_poly.push_back(Vec2{px, pz});
        m_dist.push_back(dist);
    };
    push(x, y);

    for (const auto& s : segs)
    {
        if (s.length <= 0.01f)
        {
            continue;
        }
        if (s.type == 0 || std::fabs(s.radius) < 0.05f)
        {
            advanceAlongArc(x, y, heading, 0.0f, s.length);
            push(x, y);
            continue;
        }
        const int steps = std::max(6, static_cast<int>(std::ceil(s.length / 0.55f)));
        const float step = s.length / static_cast<float>(steps);
        for (int i = 0; i < steps; ++i)
        {
            advanceAlongArc(x, y, heading, s.radius, step);
            push(x, y);
        }
    }

    if (m_poly.size() < 2)
    {
        m_poly.clear();
        m_dist.clear();
        for (const auto& s : segs)
        {
            m_poly.push_back(Vec2{s.startX, s.startZ});
        }
    }

    if (m_poly.size() < 2)
    {
        tessellateWorld(state);
        return;
    }

    float minX = m_poly[0].x, maxX = m_poly[0].x;
    float minZ = m_poly[0].z, maxZ = m_poly[0].z;
    for (const auto& p : m_poly)
    {
        minX = std::min(minX, p.x);
        maxX = std::max(maxX, p.x);
        minZ = std::min(minZ, p.z);
        maxZ = std::max(maxZ, p.z);
    }
    if ((maxX - minX) < 1.0f && (maxZ - minZ) < 1.0f)
    {
        tessellateWorld(state);
        return;
    }

    const float sf = state.startFinishMeters();
    if (!m_dist.empty() && m_dist.size() == m_poly.size() && m_dist.back() > 1.0f && sf >= 0.0f)
    {
        float target = sf;
        if (target > m_dist.back())
        {
            target = std::fmod(target, m_dist.back());
        }
        for (size_t i = 1; i < m_dist.size(); ++i)
        {
            if (m_dist[i] >= target)
            {
                const float span = m_dist[i] - m_dist[i - 1];
                const float u = span > 0.001f ? (target - m_dist[i - 1]) / span : 0.0f;
                m_sfX = m_poly[i - 1].x + (m_poly[i].x - m_poly[i - 1].x) * u;
                m_sfZ = m_poly[i - 1].z + (m_poly[i].z - m_poly[i - 1].z) * u;
                m_sfTx = m_poly[i].x - m_poly[i - 1].x;
                m_sfTz = m_poly[i].z - m_poly[i - 1].z;
                m_hasSf = true;
                break;
            }
        }
    }

    m_ready = true;
}

void MapHud::tessellateWorld(const PluginState& state)
{
    m_poly.clear();
    m_dist.clear();
    m_hasSf = false;

    for (const auto& p : state.trail())
    {
        m_poly.push_back(Vec2{p.first, p.second});
    }
    if (state.hasTelemetry())
    {
        if (m_poly.empty() ||
            std::fabs(m_poly.back().x - state.localX()) + std::fabs(m_poly.back().z - state.localZ()) > 0.5f)
        {
            m_poly.push_back(Vec2{state.localX(), state.localZ()});
        }
    }

    m_ready = m_poly.size() >= 2 || state.hasTelemetry() || !state.trackPositions().empty();
}

void MapHud::fit(const HudRect& rect, const PluginState* extras)
{
    bool any = false;
    float minX = 0.0f, maxX = 0.0f, minZ = 0.0f, maxZ = 0.0f;
    auto consider = [&](float x, float z) {
        if (!any)
        {
            minX = maxX = x;
            minZ = maxZ = z;
            any = true;
            return;
        }
        minX = std::min(minX, x);
        maxX = std::max(maxX, x);
        minZ = std::min(minZ, z);
        maxZ = std::max(maxZ, z);
    };
    for (const auto& p : m_poly)
    {
        consider(p.x, p.z);
    }
    if (extras)
    {
        for (const auto& p : extras->trackPositions())
        {
            consider(p.x, p.z);
        }
        if (extras->hasTelemetry())
        {
            consider(extras->localX(), extras->localZ());
        }
    }
    if (!any)
    {
        return;
    }

    const float pad = 0.08f;
    m_xf.innerX = rect.x;
    m_xf.innerY = rect.y + kHeaderH;
    m_xf.innerW = rect.w;
    m_xf.innerH = rect.h - kHeaderH;

    float dx = maxX - minX;
    float dz = maxZ - minZ;
    if (dx < 8.0f)
    {
        const float mid = (minX + maxX) * 0.5f;
        minX = mid - 4.0f;
        maxX = mid + 4.0f;
        dx = 8.0f;
    }
    if (dz < 8.0f)
    {
        const float mid = (minZ + maxZ) * 0.5f;
        minZ = mid - 4.0f;
        maxZ = mid + 4.0f;
        dz = 8.0f;
    }

    const float usableW = m_xf.innerW * (1.0f - 2.0f * pad);
    const float usableH = m_xf.innerH * (1.0f - 2.0f * pad);
    m_xf.scale = std::min(usableW / dx, usableH / dz);
    m_xf.minX = minX;
    m_xf.minZ = minZ;
    m_xf.maxZ = maxZ;
    const float usedW = dx * m_xf.scale;
    const float usedH = dz * m_xf.scale;
    m_xf.usedW = usedW;
    m_xf.usedH = usedH;
    m_xf.ox = m_xf.innerX + (m_xf.innerW - usedW) * 0.5f;
    m_xf.oy = m_xf.innerY + (m_xf.innerH - usedH) * 0.5f;
}

void MapHud::worldToHud(float wx, float wz, float& hx, float& hy) const
{
    hx = m_xf.ox + (wx - m_xf.minX) * m_xf.scale;
    hy = m_xf.oy + (m_xf.maxZ - wz) * m_xf.scale;
}

bool MapHud::sampleTrackPos(float trackPos, float& hx, float& hy) const
{
    if (m_poly.size() < 2 || m_dist.size() != m_poly.size() || m_dist.back() <= 0.01f)
    {
        return false;
    }
    float u = trackPos;
    if (u > 1.5f)
    {
        u = std::fmod(u, m_dist.back()) / m_dist.back();
    }
    u = std::fmod(u, 1.0f);
    if (u < 0.0f)
    {
        u += 1.0f;
    }
    return sampleDistance(u * m_dist.back(), hx, hy);
}

bool MapHud::sampleDistance(float dist, float& hx, float& hy) const
{
    if (m_poly.size() < 2 || m_dist.size() != m_poly.size() || m_dist.back() <= 0.01f)
    {
        return false;
    }
    const float len = m_dist.back();
    float target = std::fmod(dist, len);
    if (target < 0.0f)
    {
        target += len;
    }
    for (size_t i = 1; i < m_dist.size(); ++i)
    {
        if (m_dist[i] >= target)
        {
            const float span = m_dist[i] - m_dist[i - 1];
            const float t = span > 0.001f ? (target - m_dist[i - 1]) / span : 0.0f;
            const float wx = m_poly[i - 1].x + (m_poly[i].x - m_poly[i - 1].x) * t;
            const float wz = m_poly[i - 1].z + (m_poly[i].z - m_poly[i - 1].z) * t;
            worldToHud(wx, wz, hx, hy);
            return true;
        }
    }
    worldToHud(m_poly.back().x, m_poly.back().z, hx, hy);
    return true;
}

bool MapHud::projectOnTrack(float wx, float wz, float& dist, float& hx, float& hy) const
{
    if (m_poly.size() < 2 || m_dist.size() != m_poly.size() || m_dist.back() <= 0.01f)
    {
        return false;
    }
    float bestD2 = 1.0e30f;
    float bestWx = m_poly[0].x;
    float bestWz = m_poly[0].z;
    float bestDist = 0.0f;
    for (size_t i = 1; i < m_poly.size(); ++i)
    {
        const float ax = m_poly[i - 1].x;
        const float az = m_poly[i - 1].z;
        const float bx = m_poly[i].x - ax;
        const float bz = m_poly[i].z - az;
        const float seg2 = bx * bx + bz * bz;
        float t = 0.0f;
        if (seg2 > 1.0e-6f)
        {
            t = ((wx - ax) * bx + (wz - az) * bz) / seg2;
            t = std::max(0.0f, std::min(1.0f, t));
        }
        const float px = ax + bx * t;
        const float pz = az + bz * t;
        const float dx = wx - px;
        const float dz = wz - pz;
        const float d2 = dx * dx + dz * dz;
        if (d2 < bestD2)
        {
            bestD2 = d2;
            bestWx = px;
            bestWz = pz;
            bestDist = m_dist[i - 1] + std::sqrt(seg2) * t;
        }
    }
    dist = bestDist;
    worldToHud(bestWx, bestWz, hx, hy);
    return true;
}

bool MapHud::smoothLocal(const PluginState& state, float& hx, float& hy)
{
    const double now = nowSeconds();
    float dt = static_cast<float>(now - m_lastDrawSec);
    m_lastDrawSec = now;
    if (!(dt > 0.0f) || dt > 0.25f)
    {
        dt = 1.0f / 60.0f;
    }

    float age = static_cast<float>(now - state.telemetryStamp());
    if (!(age >= 0.0f) || age > 0.25f)
    {
        age = 0.0f;
    }

    float predX = state.localX();
    float predZ = state.localZ();
    if (!state.localCrashed())
    {
        predX += state.localVelX() * age;
        predZ += state.localVelZ() * age;
    }

    float targetDist = 0.0f;
    float thx = 0.0f;
    float thy = 0.0f;
    bool onPoly = projectOnTrack(predX, predZ, targetDist, thx, thy);
    if (!onPoly && sampleTrackPos(state.localTrackPos(), thx, thy))
    {
        onPoly = true;
        if (m_dist.size() == m_poly.size() && m_dist.back() > 0.01f)
        {
            float u = state.localTrackPos();
            if (u > 1.5f)
            {
                u = std::fmod(u, m_dist.back()) / m_dist.back();
            }
            u = std::fmod(u, 1.0f);
            if (u < 0.0f)
            {
                u += 1.0f;
            }
            targetDist = u * m_dist.back();
        }
    }
    if (!onPoly)
    {
        worldToHud(predX, predZ, thx, thy);
    }

    const bool canWrap = onPoly && m_dist.size() == m_poly.size() && m_dist.back() > 1.0f;
    const float len = canWrap ? m_dist.back() : 0.0f;
    const bool snap = !m_hasSmooth || dt > 0.20f ||
                      (canWrap && std::fabs(wrapDelta(targetDist, m_smoothDist, len)) > 35.0f) ||
                      (!canWrap && (std::fabs(thx - m_smoothHx) + std::fabs(thy - m_smoothHy) > 0.08f));

    if (snap)
    {
        m_smoothDist = targetDist;
        m_smoothHx = thx;
        m_smoothHy = thy;
        m_hasSmooth = true;
    }
    else
    {
        const float a = 1.0f - std::exp(-dt / 0.050f);
        if (canWrap)
        {
            m_smoothDist += wrapDelta(targetDist, m_smoothDist, len) * a;
            if (m_smoothDist < 0.0f)
            {
                m_smoothDist += len;
            }
            else if (m_smoothDist >= len)
            {
                m_smoothDist -= len;
            }
            sampleDistance(m_smoothDist, m_smoothHx, m_smoothHy);
        }
        else
        {
            m_smoothHx += (thx - m_smoothHx) * a;
            m_smoothHy += (thy - m_smoothHy) * a;
        }
    }

    hx = m_smoothHx;
    hy = m_smoothHy;
    return std::isfinite(hx) && std::isfinite(hy);
}

void MapHud::addInteriorFill(DrawList& dl) const
{
    if (m_poly.size() < 8)
    {
        return;
    }

    std::vector<Vec2> hud;
    hud.reserve(m_poly.size());
    float minY = 1.0e9f;
    float maxY = -1.0e9f;
    for (const auto& p : m_poly)
    {
        float hx, hy;
        worldToHud(p.x, p.z, hx, hy);
        if (!std::isfinite(hx) || !std::isfinite(hy))
        {
            continue;
        }
        hud.push_back(Vec2{hx, hy});
        minY = std::min(minY, hy);
        maxY = std::max(maxY, hy);
    }
    if (hud.size() < 8 || maxY - minY < 0.01f)
    {
        return;
    }

    const float step = 0.0018f;
    const size_t n = hud.size();
    std::vector<float> xs;
    xs.reserve(32);
    std::vector<float> prevX0, prevX1;
    float bandY = minY;
    float bandH = 0.0f;
    bool havePrev = false;

    auto flush = [&]() {
        if (!havePrev || bandH <= 0.0f)
        {
            return;
        }
        for (size_t i = 0; i < prevX0.size(); ++i)
        {
            dl.addQuad(prevX0[i], bandY, prevX1[i], bandY + bandH, Palette::kMapFill);
        }
    };

    for (float y = minY; y < maxY + step * 0.5f; y += step)
    {
        const float yMid = std::min(y + step * 0.5f, maxY - 1.0e-5f);
        xs.clear();
        for (size_t i = 0; i < n; ++i)
        {
            const float y0 = hud[i].z;
            const float y1 = hud[(i + 1) % n].z;
            if (!((y0 < yMid && y1 >= yMid) || (y1 < yMid && y0 >= yMid)))
            {
                continue;
            }
            const float x0 = hud[i].x;
            const float x1 = hud[(i + 1) % n].x;
            const float t = (yMid - y0) / (y1 - y0);
            xs.push_back(x0 + (x1 - x0) * t);
        }
        std::vector<float> cur0, cur1;
        if (xs.size() >= 2)
        {
            std::sort(xs.begin(), xs.end());
            for (size_t i = 0; i + 1 < xs.size(); i += 2)
            {
                if (xs[i + 1] - xs[i] > 0.0007f)
                {
                    cur0.push_back(xs[i]);
                    cur1.push_back(xs[i + 1]);
                }
            }
        }

        bool same = havePrev && cur0.size() == prevX0.size();
        if (same)
        {
            for (size_t i = 0; i < cur0.size(); ++i)
            {
                if (std::fabs(cur0[i] - prevX0[i]) > 0.00045f ||
                    std::fabs(cur1[i] - prevX1[i]) > 0.00045f)
                {
                    same = false;
                    break;
                }
            }
        }
        if (same)
        {
            bandH += step;
        }
        else
        {
            flush();
            prevX0.swap(cur0);
            prevX1.swap(cur1);
            bandY = y;
            bandH = step;
            havePrev = true;
        }
    }
    flush();
}

void MapHud::addRibbon(DrawList& dl, float halfW, unsigned long color) const
{
    if (m_poly.size() < 2)
    {
        return;
    }

    std::vector<Vec2> hud;
    hud.reserve(m_poly.size());
    float total = 0.0f;
    for (size_t i = 0; i < m_poly.size(); ++i)
    {
        float hx, hy;
        worldToHud(m_poly[i].x, m_poly[i].z, hx, hy);
        if (!std::isfinite(hx) || !std::isfinite(hy))
        {
            continue;
        }
        if (!hud.empty())
        {
            const float dx = hx - hud.back().x;
            const float dy = hy - hud.back().z;
            total += std::sqrt(dx * dx + dy * dy);
        }
        hud.push_back(Vec2{hx, hy});
    }
    if (hud.size() < 2)
    {
        return;
    }

    float step = std::max(halfW * 0.38f, 0.00115f);
    if (total > step * 380.0f)
    {
        step = total / 380.0f;
    }

    dl.addDot(hud[0].x, hud[0].z, halfW, color);
    float acc = 0.0f;
    for (size_t i = 1; i < hud.size(); ++i)
    {
        const float x0 = hud[i - 1].x;
        const float y0 = hud[i - 1].z;
        const float dx = hud[i].x - x0;
        const float dy = hud[i].z - y0;
        const float len = std::sqrt(dx * dx + dy * dy);
        if (len < 1.0e-6f)
        {
            continue;
        }
        float d = step - acc;
        while (d <= len)
        {
            const float t = d / len;
            dl.addDot(x0 + dx * t, y0 + dy * t, halfW, color);
            d += step;
        }
        acc = len - (d - step);
        if (acc < 0.0f)
        {
            acc = 0.0f;
        }
    }
}

void MapHud::addStartFinish(DrawList& dl) const
{
    if (!m_hasSf)
    {
        return;
    }
    float hx, hy;
    worldToHud(m_sfX, m_sfZ, hx, hy);
    float hx2, hy2;
    worldToHud(m_sfX + m_sfTx, m_sfZ + m_sfTz, hx2, hy2);
    float tx = hx2 - hx;
    float ty = hy2 - hy;
    const float tlen = std::sqrt(tx * tx + ty * ty);
    if (tlen < 1.0e-6f)
    {
        tx = 1.0f;
        ty = 0.0f;
    }
    else
    {
        tx /= tlen;
        ty /= tlen;
    }
    const float px = -ty;
    const float py = tx;
    const float half = 0.0040f;
    const float x0 = hx - px * half;
    const float y0 = hy - py * half;
    const float x1 = hx + px * half;
    const float y1 = hy + py * half;
    dl.addLine(x0, y0, x1, y1, 0.00155f, Palette::kDotRing);
    dl.addLine(x0, y0, x1, y1, 0.00085f, Palette::kAccent);
}

void MapHud::addMarker(DrawList& dl, float hx, float hy, float size, unsigned long color, int pos) const
{
    if (!std::isfinite(hx) || !std::isfinite(hy))
    {
        return;
    }
    if (hx < m_xf.innerX - 0.01f || hx > m_xf.innerX + m_xf.innerW + 0.01f ||
        hy < m_xf.innerY - 0.01f || hy > m_xf.innerY + m_xf.innerH + 0.01f)
    {
        return;
    }
    dl.addDot(hx, hy, size + 0.0016f, Palette::kDotRing);
    dl.addDot(hx, hy, size, color);
    if (pos > 0)
    {
        char buf[8]{};
        std::snprintf(buf, sizeof(buf), "%d", pos);
        const float fs = pos >= 10 ? size * 1.35f : size * 1.55f;
        dl.addString(buf, hx, hy - fs * 0.38f, fs, 1, Palette::kDotRing);
    }
}

void MapHud::rebuildTrack(const PluginState& state, const HudRect& rect)
{
    tessellate(state);
    fit(rect);
    bakeStatic();
}

void MapHud::bakeStatic()
{
    m_staticQuads.clear();
    if (!m_ready)
    {
        return;
    }
    DrawList tmp;
    tmp.quads.reserve(2048);
    addInteriorFill(tmp);
    addRibbon(tmp, 0.0040f, Palette::kTrackEdge);
    addRibbon(tmp, 0.0024f, Palette::kTrack);
    addStartFinish(tmp);
    m_staticQuads.swap(tmp.quads);
}

bool MapHud::ridersOnMap(const PluginState& state) const
{
    auto inside = [this](float hx, float hy) {
        return hx >= m_xf.innerX && hx <= m_xf.innerX + m_xf.innerW &&
               hy >= m_xf.innerY && hy <= m_xf.innerY + m_xf.innerH;
    };
    if (state.hasTelemetry())
    {
        float hx, hy;
        worldToHud(state.localX(), state.localZ(), hx, hy);
        if (inside(hx, hy))
        {
            return true;
        }
    }
    for (const auto& p : state.trackPositions())
    {
        float hx, hy;
        worldToHud(p.x, p.z, hx, hy);
        if (inside(hx, hy))
        {
            return true;
        }
    }
    return !state.hasTelemetry() && state.trackPositions().empty();
}

void MapHud::draw(DrawList& dl, const PluginState& state, const HudRect& rect)
{
    char shortName[40]{};
    const char* title = state.trackName().empty() ? "" : state.trackName().c_str();
    if (title[0])
    {
        truncateCopy(shortName, sizeof(shortName), title, 16);
    }

    if (!state.hasCenterline())
    {
        const size_t n = state.trail().size();
        if (n != m_trailBakeCount)
        {
            tessellateWorld(state);
            fit(rect, &state);
            bakeStatic();
            m_trailBakeCount = n;
        }
    }

    dl.addString("MAP", rect.x + 0.004f, rect.y + 0.005f, 0.0145f, 0, Palette::kAccent);
    if (title[0])
    {
        dl.addString(shortName, rect.x + rect.w - 0.004f, rect.y + 0.006f, 0.0125f, 2, Palette::kTextDim);
    }

    if (!m_ready)
    {
        dl.addString("No track map", rect.x + rect.w * 0.5f, rect.y + rect.h * 0.52f, 0.0135f, 1, Palette::kTextDim);
        return;
    }

    dl.append(m_staticQuads);

    const int focus = state.focusRaceNum();
    for (const auto& p : state.trackPositions())
    {
        if (state.hasTelemetry() && p.raceNum == focus)
        {
            continue;
        }
        float hx, hy;
        if (!sampleTrackPos(p.trackPos, hx, hy))
        {
            worldToHud(p.x, p.z, hx, hy);
        }
        unsigned long col = Palette::kRider;
        if (p.crashed)
        {
            col = Palette::kRiderCrash;
        }
        addMarker(dl, hx, hy, 0.0028f, col);
    }

    if (state.hasTelemetry())
    {
        float hx, hy;
        if (!smoothLocal(state, hx, hy))
        {
            worldToHud(state.localX(), state.localZ(), hx, hy);
        }
        int pos = 0;
        const StandingRow* row = state.findStanding(state.focusRaceNum());
        if (row)
        {
            pos = row->position;
        }
        addMarker(dl, hx, hy, 0.0062f, Palette::kRiderLocal, pos);
    }
}
