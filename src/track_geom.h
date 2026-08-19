#pragma once

#include "state.h"

#include <algorithm>
#include <cmath>
#include <vector>

namespace track_geom
{
    constexpr float kPi = 3.14159265358979323846f;
    constexpr float kDeg = 180.0f / kPi;
    constexpr float kMinArcRadius = 0.05f;
    constexpr float kMinSegLength = 0.01f;
    constexpr float kCenterlineStepMeters = 0.55f;
    constexpr int kMinArcSteps = 6;
    constexpr float kDedupDistSq = 0.04f;

    inline float deg2rad(float d)
    {
        return d * kPi / 180.0f;
    }

    inline void advanceAlongArc(float& x, float& y, float& angleDeg, float radius, float distance)
    {
        const float r = (std::fabs(radius) < kMinArcRadius) ? 0.0f : radius;
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

    template <typename Fn>
    void walkCenterline(const std::vector<CenterlineSeg>& segs, Fn&& emit)
    {
        if (segs.empty())
        {
            return;
        }
        float x = segs[0].startX;
        float y = segs[0].startZ;
        float heading = segs[0].angle;
        emit(x, y);
        for (const auto& s : segs)
        {
            if (s.length <= kMinSegLength)
            {
                continue;
            }
            if (s.type == 0 || std::fabs(s.radius) < kMinArcRadius)
            {
                advanceAlongArc(x, y, heading, 0.0f, s.length);
                emit(x, y);
                continue;
            }
            const int steps = std::max(kMinArcSteps, static_cast<int>(std::ceil(s.length / kCenterlineStepMeters)));
            const float step = s.length / static_cast<float>(steps);
            for (int i = 0; i < steps; ++i)
            {
                advanceAlongArc(x, y, heading, s.radius, step);
                emit(x, y);
            }
        }
    }
}
