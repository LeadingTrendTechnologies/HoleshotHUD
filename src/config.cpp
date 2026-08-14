#include "config.h"

#include <algorithm>
#include <cstdlib>
#include <fstream>
#include <string>

namespace
{
    std::string trim(const std::string& s)
    {
        const auto a = s.find_first_not_of(" \t\r\n");
        if (a == std::string::npos)
        {
            return {};
        }
        const auto b = s.find_last_not_of(" \t\r\n");
        return s.substr(a, b - a + 1);
    }

    bool parseBool(const std::string& v)
    {
        return v == "1" || v == "true" || v == "True" || v == "yes";
    }

    void writeRect(std::ofstream& out, const char* prefix, const HudRect& r)
    {
        out << prefix << "_x=" << r.x << "\n";
        out << prefix << "_y=" << r.y << "\n";
        out << prefix << "_w=" << r.w << "\n";
        out << prefix << "_h=" << r.h << "\n";
    }

    bool applyRect(const std::string& key, const std::string& prefix, HudRect& r, float value)
    {
        if (key == prefix + "_x") { r.x = value; return true; }
        if (key == prefix + "_y") { r.y = value; return true; }
        if (key == prefix + "_w") { r.w = value; return true; }
        if (key == prefix + "_h") { r.h = value; return true; }
        return false;
    }
}

void PluginConfig::load(const std::string& path)
{
    std::ifstream in(path);
    if (!in)
    {
        save(path);
        return;
    }

    std::string line;
    while (std::getline(in, line))
    {
        line = trim(line);
        if (line.empty() || line[0] == '#' || line[0] == ';' || line[0] == '[')
        {
            continue;
        }
        const auto eq = line.find('=');
        if (eq == std::string::npos)
        {
            continue;
        }
        const std::string key = trim(line.substr(0, eq));
        const std::string val = trim(line.substr(eq + 1));
        const float f = static_cast<float>(std::atof(val.c_str()));

        if (applyRect(key, "standings", standings, f)) continue;
        if (applyRect(key, "relative", relative, f)) continue;
        if (applyRect(key, "map", map, f)) continue;
        if (key == "show_standings") showStandings = parseBool(val);
        else if (key == "show_relative") showRelative = parseBool(val);
        else if (key == "show_map") showMap = parseBool(val);
        else if (key == "ingame_hud") ingameHud = parseBool(val);
        else if (key == "standings_rows") standingsRows = std::max(3, std::atoi(val.c_str()));
        else if (key == "relative_count") relativeCount = std::max(1, std::atoi(val.c_str()));
    }
}

void PluginConfig::save(const std::string& path) const
{
    std::ofstream out(path);
    if (!out)
    {
        return;
    }
    out << "# mxbo HUD layout (normalized 0..1, origin top-left)\n";
    out << "[Layout]\n";
    writeRect(out, "standings", standings);
    writeRect(out, "relative", relative);
    writeRect(out, "map", map);
    out << "\n[Widgets]\n";
    out << "show_standings=" << (showStandings ? 1 : 0) << "\n";
    out << "show_relative=" << (showRelative ? 1 : 0) << "\n";
    out << "show_map=" << (showMap ? 1 : 0) << "\n";
    out << "ingame_hud=" << (ingameHud ? 1 : 0) << "\n";
    out << "standings_rows=" << standingsRows << "\n";
    out << "relative_count=" << relativeCount << "\n";
}
