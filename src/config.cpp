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
        if (applyRect(key, "minimap", minimap, f)) continue;
        if (applyRect(key, "radar", radar, f)) continue;
        if (applyRect(key, "dash", dash, f)) continue;
        if (key == "show_standings") showStandings = parseBool(val);
        else if (key == "show_relative") showRelative = parseBool(val);
        else if (key == "show_map") showMap = parseBool(val);
        else if (key == "show_minimap") showMinimap = parseBool(val);
        else if (key == "show_radar") showRadar = parseBool(val);
        else if (key == "show_dash") showDash = parseBool(val);
        else if (key == "ingame_hud") ingameHud = parseBool(val);
        else if (key == "standings_rows") standingsRows = std::max(3, std::atoi(val.c_str()));
        else if (key == "relative_count") relativeCount = std::max(1, std::atoi(val.c_str()));
        else if (key == "st_pos") stPos = parseBool(val);
        else if (key == "st_num") stNum = parseBool(val);
        else if (key == "st_name") stName = parseBool(val);
        else if (key == "st_gap") stGap = parseBool(val);
        else if (key == "st_interval") stInterval = parseBool(val);
        else if (key == "st_laps") stLaps = parseBool(val);
        else if (key == "st_best") stBest = parseBool(val);
        else if (key == "st_status") stStatus = parseBool(val);
        else if (key == "st_bike") stBike = parseBool(val);
        else if (key == "st_penalty") stPenalty = parseBool(val);
        else if (key == "st_crashed") stCrashed = parseBool(val);
        else if (key == "rel_num") relNum = parseBool(val);
        else if (key == "rel_name") relName = parseBool(val);
        else if (key == "rel_gap") relGap = parseBool(val);
        else if (key == "rel_pos") relPos = parseBool(val);
        else if (key == "rel_bike") relBike = parseBool(val);
        else if (key == "rel_penalty") relPenalty = parseBool(val);
        else if (key == "rel_interval") relInterval = parseBool(val);
        else if (key == "rel_crashed") relCrashed = parseBool(val);
        else if (key == "map_others") mapOthers = parseBool(val);
        else if (key == "map_sf") mapSf = parseBool(val);
        else if (key == "map_name") mapName = parseBool(val);
        else if (key == "map_numbers") mapNumbers = parseBool(val);
        else if (key == "map_arrows") mapArrows = parseBool(val);
        else if (key == "map_crown") mapCrown = parseBool(val);
        else if (key == "map_place") mapPlace = parseBool(val);
        else if (key == "map_dot") mapDot = val;
        else if (key == "mini_others") miniOthers = parseBool(val);
        else if (key == "mini_sf") miniSf = parseBool(val);
        else if (key == "mini_numbers") miniNumbers = parseBool(val);
        else if (key == "mini_arrows") miniArrows = parseBool(val);
        else if (key == "mini_crown") miniCrown = parseBool(val);
        else if (key == "mini_place") miniPlace = parseBool(val);
        else if (key == "mini_dot") miniDot = val;
        else if (key == "radar_sides") radarSides = parseBool(val);
        else if (key == "radar_rear") radarRear = parseBool(val);
        else if (key == "st_bg") stBg = std::max(0, std::min(100, std::atoi(val.c_str())));
        else if (key == "rel_bg") relBg = std::max(0, std::min(100, std::atoi(val.c_str())));
        else if (key == "map_bg") mapBg = std::max(0, std::min(100, std::atoi(val.c_str())));
        else if (key == "mini_bg") miniBg = std::max(0, std::min(100, std::atoi(val.c_str())));
        else if (key == "radar_bg") radarBg = std::max(0, std::min(100, std::atoi(val.c_str())));
        else if (key == "dash_bg") dashBg = std::max(0, std::min(100, std::atoi(val.c_str())));
        else if (key == "dash_left") dashLeft = val;
        else if (key == "dash_mid") dashMid = val;
        else if (key == "dash_right") dashRight = val;
        else if (key == "st_order") stOrder = val;
        else if (key == "rel_order") relOrder = val;
        else if (key == "st_head") stHead = val;
        else if (key == "st_foot") stFoot = val;
        else if (key == "rel_head") relHead = val;
        else if (key == "rel_foot") relFoot = val;
        else if (key == "st_w_pos") stWPos = std::max(18, std::atoi(val.c_str()));
        else if (key == "st_w_num") stWNum = std::max(18, std::atoi(val.c_str()));
        else if (key == "st_w_name") stWName = std::max(18, std::atoi(val.c_str()));
        else if (key == "st_w_gap") stWGap = std::max(18, std::atoi(val.c_str()));
        else if (key == "st_w_interval") stWInterval = std::max(18, std::atoi(val.c_str()));
        else if (key == "st_w_laps") stWLaps = std::max(18, std::atoi(val.c_str()));
        else if (key == "st_w_best") stWBest = std::max(18, std::atoi(val.c_str()));
        else if (key == "st_w_status") stWStatus = std::max(18, std::atoi(val.c_str()));
        else if (key == "st_w_bike") stWBike = std::max(18, std::atoi(val.c_str()));
        else if (key == "st_w_penalty") stWPenalty = std::max(18, std::atoi(val.c_str()));
        else if (key == "st_w_crashed") stWCrashed = std::max(18, std::atoi(val.c_str()));
        else if (key == "rel_w_num") relWNum = std::max(18, std::atoi(val.c_str()));
        else if (key == "rel_w_name") relWName = std::max(18, std::atoi(val.c_str()));
        else if (key == "rel_w_gap") relWGap = std::max(18, std::atoi(val.c_str()));
        else if (key == "rel_w_pos") relWPos = std::max(18, std::atoi(val.c_str()));
        else if (key == "rel_w_bike") relWBike = std::max(18, std::atoi(val.c_str()));
        else if (key == "rel_w_penalty") relWPenalty = std::max(18, std::atoi(val.c_str()));
        else if (key == "rel_w_interval") relWInterval = std::max(18, std::atoi(val.c_str()));
        else if (key == "rel_w_crashed") relWCrashed = std::max(18, std::atoi(val.c_str()));
    }
}
