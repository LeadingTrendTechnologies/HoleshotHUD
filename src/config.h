#pragma once

#include <string>

// Read-only subset of Holeshot-HUD.ini for the plugin. Overlay HudConfig is
// the settings object and the only writer. fillSnapshot reads the rects,
// show flags, and row counts below. The frozen in-game draw (standings,
// relative, map) is compiled only with MXBO_INGAME_HUD=1.

struct HudRect
{
    float x = 0.0f;
    float y = 0.0f;
    float w = 0.2f;
    float h = 0.3f;
};

struct PluginConfig
{
    HudRect standings{0.012f, 0.030f, 0.20f, 0.46f};
    HudRect relative{0.012f, 0.62f, 0.20f, 0.36f};
    HudRect map{0.775f, 0.62f, 0.210f, 0.340f};

    bool showStandings = false;
    bool showRelative = false;
    bool showMap = false;
    bool ingameHud = false;

    int standingsRows = 12;
    int relativeCount = 3;

    void load(const std::string& path);
};
