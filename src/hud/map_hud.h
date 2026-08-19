#pragma once

#include "draw_list.h"
#include "../config.h"
#include "../state.h"

#include <vector>

class MapHud
{
public:
    void rebuildTrack(const PluginState& state, const HudRect& rect);
    void draw(DrawList& dl, const PluginState& state, const HudRect& rect);

private:
    struct Vec2
    {
        float x = 0.0f;
        float z = 0.0f;
    };

    struct MapXform
    {
        float minX = 0.0f;
        float minZ = 0.0f;
        float maxZ = 0.0f;
        float scale = 1.0f;
        float ox = 0.0f;
        float oy = 0.0f;
        float innerX = 0.0f;
        float innerY = 0.0f;
        float innerW = 0.0f;
        float innerH = 0.0f;
        float usedW = 0.0f;
        float usedH = 0.0f;
    };

    void tessellate(const PluginState& state);
    void tessellateWorld(const PluginState& state);
    void fit(const HudRect& rect, const PluginState* extras = nullptr);
    void worldToHud(float wx, float wz, float& hx, float& hy) const;
    bool sampleTrackPos(float trackPos, float& hx, float& hy) const;
    bool sampleDistance(float dist, float& hx, float& hy) const;
    bool projectOnTrack(float wx, float wz, float& dist, float& hx, float& hy) const;
    bool smoothLocal(const PluginState& state, float& hx, float& hy);
    void addInteriorFill(DrawList& dl) const;
    void addRibbon(DrawList& dl, float halfW, unsigned long color) const;
    void addStartFinish(DrawList& dl) const;
    void addMarker(DrawList& dl, float hx, float hy, float size, unsigned long color, int pos = 0) const;
    void bakeStatic();

    std::vector<Vec2> m_poly;
    std::vector<float> m_dist;
    std::vector<SPluginQuad_t> m_staticQuads;
    MapXform m_xf;
    bool m_ready = false;
    float m_sfX = 0.0f;
    float m_sfZ = 0.0f;
    float m_sfTx = 0.0f;
    float m_sfTz = 0.0f;
    bool m_hasSf = false;
    float m_smoothDist = 0.0f;
    float m_smoothHx = 0.0f;
    float m_smoothHy = 0.0f;
    bool m_hasSmooth = false;
    double m_lastDrawSec = 0.0;
    size_t m_trailBakeCount = static_cast<size_t>(-1);
};
