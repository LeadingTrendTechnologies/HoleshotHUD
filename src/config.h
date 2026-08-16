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
    HudRect standings{0.012f, 0.030f, 0.30f, 0.46f};
    HudRect relative{0.012f, 0.62f, 0.30f, 0.36f};
    HudRect map{0.775f, 0.62f, 0.210f, 0.340f};
    HudRect minimap{0.815f, 0.035f, 0.165f, 0.295f};
    HudRect radar{0.438f, 0.755f, 0.124f, 0.220f};
    HudRect dash{0.41f, 0.82f, 0.18f, 0.16f};

    bool showStandings = true;
    bool showRelative = true;
    bool showMap = true;
    bool showMinimap = true;
    bool showRadar = true;
    bool showDash = true;
    bool ingameHud = false;

    int standingsRows = 12;
    int relativeCount = 3;

    bool stPos = true;
    bool stNum = true;
    bool stName = true;
    bool stGap = true;
    bool stInterval = false;
    bool stLaps = false;
    bool stBest = false;
    bool stStatus = false;
    bool stBike = false;
    bool stPenalty = false;
    bool stCrashed = false;

    bool relNum = true;
    bool relName = true;
    bool relGap = true;
    bool relPos = false;
    bool relBike = false;
    bool relPenalty = false;
    bool relInterval = false;
    bool relCrashed = false;

    bool mapOthers = true;
    bool mapSf = true;
    bool mapName = true;
    bool mapNumbers = true;
    bool mapArrows = true;
    bool mapCrown = true;
    bool mapPlace = true;
    std::string mapDot = "pos";

    bool miniOthers = true;
    bool miniSf = true;
    bool miniNumbers = true;
    bool miniArrows = true;
    bool miniCrown = true;
    bool miniPlace = true;
    std::string miniDot = "num";

    bool radarSides = true;
    bool radarRear = true;

    int stBg = 78;
    int relBg = 78;
    int mapBg = 0;
    int miniBg = 0;
    int radarBg = 86;
    int dashBg = 82;
    std::string dashLeft = "eng";
    std::string dashMid = "air";
    std::string dashRight = "best";

    std::string stOrder = "pos,num,name,gap,int,laps,best,status,bike,pen,crash";
    std::string relOrder = "num,name,gap,pos,bike,pen,int,crash";
    std::string stHead = "sess,none,riders";
    std::string stFoot = "none,none,none";
    std::string relHead = "sess,none,riders";
    std::string relFoot = "none,none,none";

    int stWPos = 26;
    int stWNum = 30;
    int stWName = 80;
    int stWGap = 58;
    int stWInterval = 58;
    int stWLaps = 32;
    int stWBest = 58;
    int stWStatus = 40;
    int stWBike = 56;
    int stWPenalty = 48;
    int stWCrashed = 44;
    int relWNum = 32;
    int relWName = 80;
    int relWGap = 58;
    int relWPos = 28;
    int relWBike = 56;
    int relWPenalty = 48;
    int relWInterval = 58;
    int relWCrashed = 44;

    void load(const std::string& path);
    void save(const std::string& path) const;
};
