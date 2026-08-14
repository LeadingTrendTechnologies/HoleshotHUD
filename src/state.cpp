#include "state.h"

#include <algorithm>
#include <cmath>
#include <cstring>
#include <string>

namespace
{
    std::string copyCString(const char* s, size_t maxLen)
    {
        if (!s)
        {
            return {};
        }
        size_t n = 0;
        while (n < maxLen && s[n] != '\0')
        {
            ++n;
        }
        return std::string(s, s + n);
    }

    int clampCount(int n)
    {
        if (n < 0)
        {
            return 0;
        }
        if (n > kMaxRaceEntries)
        {
            return kMaxRaceEntries;
        }
        return n;
    }
}

void PluginState::clearEvent()
{
    m_localName.clear();
    m_trackName.clear();
    m_trackLength = 0.0f;
    m_hasTelemetry = false;
    m_localVelX = 0.0f;
    m_localVelZ = 0.0f;
    m_telemetryStamp = 0.0;
    m_localRaceNum = -1;
    m_centerline.clear();
    m_centerlineDirty = true;
    m_sfMeters = 0.0f;
    m_trail.clear();
    m_trailStarted = false;
    clearRace();
}

void PluginState::clearRace()
{
    m_entries.clear();
    m_vehicles.clear();
    m_standings.clear();
    m_trackPos.clear();
    m_focusRaceNum = -1;
}

void PluginState::setEvent(const SPluginsBikeEvent_t& ev)
{
    m_localName = copyCString(ev.m_szRiderName, sizeof(ev.m_szRiderName));
    if (ev.m_szTrackName[0])
    {
        m_trackName = copyCString(ev.m_szTrackName, sizeof(ev.m_szTrackName));
    }
    if (ev.m_fTrackLength > 0.0f)
    {
        m_trackLength = ev.m_fTrackLength;
    }
    resolveLocalRaceNum();
}

void PluginState::setRaceEvent(const SPluginsRaceEvent_t& ev)
{
    if (ev.m_szTrackName[0])
    {
        m_trackName = copyCString(ev.m_szTrackName, sizeof(ev.m_szTrackName));
    }
    if (ev.m_fTrackLength > 0.0f)
    {
        m_trackLength = ev.m_fTrackLength;
    }
}

void PluginState::setSession(const SPluginsRaceSession_t& s)
{
    (void)s;
}

void PluginState::addEntry(const SPluginsRaceAddEntry_t& e)
{
    RaceEntry& row = m_entries[e.m_iRaceNum];
    row.raceNum = e.m_iRaceNum;
    row.name = copyCString(e.m_szName, sizeof(e.m_szName));
    row.bikeShort = copyCString(e.m_szBikeShortName, sizeof(e.m_szBikeShortName));
    row.unactive = e.m_iUnactive != 0;
    resolveLocalRaceNum();
}

void PluginState::removeEntry(int raceNum)
{
    m_entries.erase(raceNum);
    m_vehicles.erase(raceNum);
    if (m_localRaceNum == raceNum)
    {
        m_localRaceNum = -1;
    }
    if (m_focusRaceNum == raceNum)
    {
        m_focusRaceNum = -1;
    }
}

void PluginState::setClassification(const SPluginsRaceClassification_t&,
                                    const SPluginsRaceClassificationEntry_t* entries,
                                    int count)
{
    count = clampCount(count);
    m_standings.clear();
    m_standings.reserve(static_cast<size_t>(count));
    for (int i = 0; i < count; ++i)
    {
        StandingRow row;
        row.raceNum = entries[i].m_iRaceNum;
        row.position = i + 1;
        row.state = entries[i].m_iState;
        row.bestLapMs = entries[i].m_iBestLap;
        row.numLaps = entries[i].m_iNumLaps;
        row.gapMs = entries[i].m_iGap;
        row.gapLaps = entries[i].m_iGapLaps;
        row.penaltyMs = entries[i].m_iPenalty;
        row.pit = entries[i].m_iPit;
        m_standings.push_back(row);
    }
}

void PluginState::setTrackPositions(const SPluginsRaceTrackPosition_t* entries, int count)
{
    count = clampCount(count);
    m_trackPos.clear();
    m_trackPos.reserve(static_cast<size_t>(count));
    for (int i = 0; i < count; ++i)
    {
        TrackPos p;
        p.raceNum = entries[i].m_iRaceNum;
        p.x = entries[i].m_fPosX;
        p.y = entries[i].m_fPosY;
        p.z = entries[i].m_fPosZ;
        p.yaw = entries[i].m_fYaw;
        p.trackPos = entries[i].m_fTrackPos;
        p.crashed = entries[i].m_iCrashed;
        m_trackPos.push_back(p);
    }
    resolveLocalRaceNum();
}

void PluginState::setCenterline(int numSegments, const SPluginsTrackSegment_t* segments, const float* raceData)
{
    m_centerline.clear();
    if (numSegments < 0)
    {
        numSegments = 0;
    }
    constexpr int kMaxSegs = 4096;
    if (numSegments > kMaxSegs)
    {
        numSegments = kMaxSegs;
    }
    if (numSegments > 0 && !segments)
    {
        m_centerlineDirty = true;
        return;
    }
    m_centerline.reserve(static_cast<size_t>(numSegments));
    for (int i = 0; i < numSegments; ++i)
    {
        CenterlineSeg s;
        s.type = segments[i].m_iType;
        s.length = segments[i].m_fLength;
        s.radius = segments[i].m_fRadius;
        s.angle = segments[i].m_fAngle;
        s.startX = segments[i].m_afStart[0];
        s.startZ = segments[i].m_afStart[1];
        s.height = segments[i].m_fHeight;
        m_centerline.push_back(s);
    }
    if (raceData)
    {
        m_sfMeters = raceData[0];
    }
    m_centerlineDirty = true;
}

void PluginState::setTelemetry(const SPluginsBikeData_t& data, float, float pos)
{
    m_hasTelemetry = true;
    m_localSpeed = data.m_fSpeedometer;
    m_localTrackPos = pos;
    m_localX = data.m_fPosX;
    m_localZ = data.m_fPosZ;
    m_localVelX = data.m_fVelocityX;
    m_localVelZ = data.m_fVelocityZ;
    m_localYaw = data.m_fYaw;
    m_localCrashed = data.m_iCrashed;
    m_telemetryStamp = pluginNowSeconds();
    resolveLocalRaceNum();

    if (!m_trailStarted)
    {
        m_trail.push_back({m_localX, m_localZ});
        m_trailLastX = m_localX;
        m_trailLastZ = m_localZ;
        m_trailStarted = true;
    }
    else
    {
        const float dx = m_localX - m_trailLastX;
        const float dz = m_localZ - m_trailLastZ;
        if (dx * dx + dz * dz > 2.25f)
        {
            m_trail.push_back({m_localX, m_localZ});
            m_trailLastX = m_localX;
            m_trailLastZ = m_localZ;
            if (m_trail.size() > 1200)
            {
                m_trail.erase(m_trail.begin(), m_trail.begin() + 200);
            }
        }
    }
}

void PluginState::setVehicleData(const SPluginsRaceVehicleData_t& data)
{
    VehicleLive& v = m_vehicles[data.m_iRaceNum];
    v.active = data.m_iActive != 0;
    v.rpm = data.m_iRPM;
    v.gear = data.m_iGear;
    v.speed = data.m_fSpeedometer;
    v.throttle = data.m_fThrottle;
    v.frontBrake = data.m_fFrontBrake;
    v.lean = data.m_fLean;
}

void PluginState::setSpectateSelection(int raceNum)
{
    m_focusRaceNum = raceNum;
}

float PluginState::riderSpeed(int raceNum) const
{
    const auto it = m_vehicles.find(raceNum);
    if (it != m_vehicles.end() && it->second.active && it->second.speed > 0.5f)
    {
        return it->second.speed;
    }
    if (raceNum == m_localRaceNum && m_hasTelemetry)
    {
        return m_localSpeed;
    }
    return m_localSpeed;
}

const RaceEntry* PluginState::findEntry(int raceNum) const
{
    const auto it = m_entries.find(raceNum);
    return it == m_entries.end() ? nullptr : &it->second;
}

const StandingRow* PluginState::findStanding(int raceNum) const
{
    for (const auto& row : m_standings)
    {
        if (row.raceNum == raceNum)
        {
            return &row;
        }
    }
    return nullptr;
}

const TrackPos* PluginState::findTrackPos(int raceNum) const
{
    for (const auto& p : m_trackPos)
    {
        if (p.raceNum == raceNum)
        {
            return &p;
        }
    }
    return nullptr;
}

void PluginState::resolveLocalRaceNum()
{
    if (m_localRaceNum >= 0)
    {
        return;
    }
    if (!m_localName.empty())
    {
        for (const auto& kv : m_entries)
        {
            if (_stricmp(kv.second.name.c_str(), m_localName.c_str()) == 0)
            {
                m_localRaceNum = kv.first;
                return;
            }
        }
    }
    if (m_hasTelemetry && !m_trackPos.empty())
    {
        float best = 1.0e9f;
        int bestNum = -1;
        for (const auto& p : m_trackPos)
        {
            const float dx = p.x - m_localX;
            const float dz = p.z - m_localZ;
            const float d2 = dx * dx + dz * dz;
            if (d2 < best)
            {
                best = d2;
                bestNum = p.raceNum;
            }
        }
        if (bestNum >= 0 && best < 4.0f)
        {
            m_localRaceNum = bestNum;
        }
    }
}
