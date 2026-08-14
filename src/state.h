#pragma once

#include "vendor/piboso/mxb_api.h"

#include <chrono>
#include <string>
#include <unordered_map>
#include <utility>
#include <vector>

inline double pluginNowSeconds()
{
    using Clock = std::chrono::steady_clock;
    static const auto t0 = Clock::now();
    return std::chrono::duration<double>(Clock::now() - t0).count();
}

constexpr int kMaxRaceEntries = 128;

struct RaceEntry
{
    int raceNum = 0;
    std::string name;
    std::string bikeShort;
    bool unactive = false;
};

struct StandingRow
{
    int raceNum = 0;
    int position = 0;
    int state = 0;
    int bestLapMs = 0;
    int numLaps = 0;
    int gapMs = 0;
    int gapLaps = 0;
    int penaltyMs = 0;
    int pit = 0;
};

struct TrackPos
{
    int raceNum = 0;
    float x = 0.0f;
    float y = 0.0f;
    float z = 0.0f;
    float yaw = 0.0f;
    float trackPos = 0.0f;
    int crashed = 0;
};

struct CenterlineSeg
{
    int type = 0;
    float length = 0.0f;
    float radius = 0.0f;
    float angle = 0.0f;
    float startX = 0.0f;
    float startZ = 0.0f;
    float height = 0.0f;
};

struct VehicleLive
{
    int rpm = 0;
    int gear = 0;
    float speed = 0.0f;
    float throttle = 0.0f;
    float frontBrake = 0.0f;
    float lean = 0.0f;
    bool active = false;
};

class PluginState
{
public:
    void clearEvent();
    void clearRace();

    void setEvent(const SPluginsBikeEvent_t& ev);
    void setRaceEvent(const SPluginsRaceEvent_t& ev);
    void setSession(const SPluginsRaceSession_t& s);
    void addEntry(const SPluginsRaceAddEntry_t& e);
    void removeEntry(int raceNum);
    void setClassification(const SPluginsRaceClassification_t& header,
                           const SPluginsRaceClassificationEntry_t* entries,
                           int count);
    void setTrackPositions(const SPluginsRaceTrackPosition_t* entries, int count);
    void setCenterline(int numSegments, const SPluginsTrackSegment_t* segments, const float* raceData);
    void setTelemetry(const SPluginsBikeData_t& data, float time, float pos);
    void setVehicleData(const SPluginsRaceVehicleData_t& data);
    void setSpectateSelection(int raceNum);

    const std::unordered_map<int, RaceEntry>& entries() const { return m_entries; }
    const std::vector<StandingRow>& standings() const { return m_standings; }
    const std::vector<TrackPos>& trackPositions() const { return m_trackPos; }
    const std::vector<CenterlineSeg>& centerline() const { return m_centerline; }
    const std::vector<std::pair<float, float>>& trail() const { return m_trail; }

    float trackLength() const { return m_trackLength; }
    const std::string& trackName() const { return m_trackName; }
    float startFinishMeters() const { return m_sfMeters; }
    bool hasCenterline() const { return !m_centerline.empty(); }
    bool hasStandings() const { return !m_standings.empty(); }

    int localRaceNum() const { return m_localRaceNum; }
    int focusRaceNum() const { return m_focusRaceNum >= 0 ? m_focusRaceNum : m_localRaceNum; }
    const std::string& localName() const { return m_localName; }

    bool hasTelemetry() const { return m_hasTelemetry; }
    float localSpeed() const { return m_localSpeed; }
    float localTrackPos() const { return m_localTrackPos; }
    float localX() const { return m_localX; }
    float localZ() const { return m_localZ; }
    float localVelX() const { return m_localVelX; }
    float localVelZ() const { return m_localVelZ; }
    float localYaw() const { return m_localYaw; }
    int localCrashed() const { return m_localCrashed; }
    double telemetryStamp() const { return m_telemetryStamp; }

    float riderSpeed(int raceNum) const;
    const RaceEntry* findEntry(int raceNum) const;
    const StandingRow* findStanding(int raceNum) const;
    const TrackPos* findTrackPos(int raceNum) const;

    bool centerlineDirty() const { return m_centerlineDirty; }
    void clearCenterlineDirty() { m_centerlineDirty = false; }

private:
    void resolveLocalRaceNum();

    std::unordered_map<int, RaceEntry> m_entries;
    std::unordered_map<int, VehicleLive> m_vehicles;
    std::vector<StandingRow> m_standings;
    std::vector<TrackPos> m_trackPos;
    std::vector<CenterlineSeg> m_centerline;
    std::vector<std::pair<float, float>> m_trail;
    float m_trailLastX = 0.0f;
    float m_trailLastZ = 0.0f;
    bool m_trailStarted = false;

    std::string m_localName;
    std::string m_trackName;
    float m_trackLength = 0.0f;
    float m_sfMeters = 0.0f;
    int m_localRaceNum = -1;
    int m_focusRaceNum = -1;

    bool m_hasTelemetry = false;
    float m_localSpeed = 0.0f;
    float m_localTrackPos = 0.0f;
    float m_localX = 0.0f;
    float m_localZ = 0.0f;
    float m_localVelX = 0.0f;
    float m_localVelZ = 0.0f;
    float m_localYaw = 0.0f;
    int m_localCrashed = 0;
    double m_telemetryStamp = 0.0;

    bool m_centerlineDirty = false;
};
