#pragma once

#include <string>

struct HudRect
{
    float x = 0.0f;
    float y = 0.0f;
    float w = 0.2f;
    float h = 0.3f;
};

struct PluginConfig
{
    HudRect standings{0.012f, 0.030f, 0.235f, 0.42f};
    HudRect relative{0.012f, 0.62f, 0.235f, 0.33f};
    HudRect map{0.775f, 0.62f, 0.210f, 0.340f};

    bool showStandings = true;
    bool showRelative = true;
    bool showMap = true;
    bool ingameHud = false;

    int standingsRows = 12;
    int relativeCount = 3;

    void load(const std::string& path);
    void save(const std::string& path) const;
};
