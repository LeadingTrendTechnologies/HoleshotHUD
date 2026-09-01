#include "state.h"
#include "str_util.h"

#include <algorithm>
#include <cstring>
#include <string>

namespace
{
    constexpr float kTrailMinDistSq = 2.25f;
    constexpr size_t kTrailMaxPoints = 1200;
    constexpr size_t kTrailTrimBatch = 200;

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

    int sectorThreeMs(int s1, int s2, int lap)
    {
        if (s1 <= 0 || s2 <= 0 || lap <= 0)
        {
            return 0;
        }
        const int dur = lap - s1 - s2;
        const int cum = lap - s2;
        if (dur <= 0)
        {
            return cum > 0 ? cum : 0;
        }
        if (cum <= 0)
        {
            return dur;
        }
        const int dDur = dur > s1 ? dur - s1 : s1 - dur;
        const int dCum = cum > s1 ? cum - s1 : s1 - cum;
        return dDur <= dCum ? dur : cum;
    }
}

void PluginState::clearEvent()
{
    m_localName.clear();
    m_guid.clear();
    m_serverName.clear();
    m_serverType = 0;
    m_trackName.clear();
    m_trackLength = 0.0f;
    m_hasTelemetry = false;
    m_localVelX = 0.0f;
    m_localVelZ = 0.0f;
    m_telemetryStamp = 0.0;
    m_trackPosStamp = 0.0;
    m_localRaceNum = -1;
    m_centerline.clear();
    m_centerlineDirty = true;
    m_mapRev++;
    m_sfMeters = 0.0f;
    m_trail.clear();
    m_trailStarted = false;
    m_inRun = false;
    m_maxRpm = 0;
    m_shiftRpm = 0;
    m_maxFuel = 0.0f;
    clearRace();
}

void PluginState::clearRace()
{
    m_entries.clear();
    m_vehicles.clear();
    m_standings.clear();
    m_trackPos.clear();
    m_focusRaceNum = -1;
    m_lastSpectate = 0.0;
    m_localGear = 0;
    m_localRpm = 0;
    m_engineTemp = 0.0f;
    m_airTemp = 0.0f;
    m_fuel = 0.0f;
    m_sessionTime = 0.0f;
    m_lastLapEndTime = 0.0f;
    m_lastLapMs = 0;
    m_bestLapMs = 0;
    m_currentLap = 0;
    m_sessionLaps = 0;
    m_sessionKind = -1;
    m_sessionState = -1;
    m_sessionClock = 0;
    m_sessionLength = kSessionLengthUnset;
    m_sessionRemain = 0;
    m_lastLaps.clear();
    m_inRun = false;
    for (int i = 0; i < 3; ++i)
    {
        m_sectorCur[i] = 0;
        m_sectorLastLap[i] = 0;
        m_sectorBest[i] = 0;
        m_sectorDelta[i] = 0;
    }
    m_sectorDeltaValid = 0;
    m_sectorLast = -1;
    m_sectorFinishedLap = -1;
}

void PluginState::setEvent(const SPluginsBikeEvent_t& ev)
{
    m_localName = copyCString(ev.m_szRiderName, sizeof(ev.m_szRiderName));
    m_guid = copyCString(ev.m_szGUID, sizeof(ev.m_szGUID));
    m_serverName = copyCString(ev.m_szServerName, sizeof(ev.m_szServerName));
    m_serverType = ev.m_iServerType;
    if (ev.m_szTrackName[0])
    {
        m_trackName = copyCString(ev.m_szTrackName, sizeof(ev.m_szTrackName));
    }
    if (ev.m_fTrackLength > 0.0f)
    {
        m_trackLength = ev.m_fTrackLength;
    }
    if (ev.m_iMaxRPM > 0)
    {
        m_maxRpm = ev.m_iMaxRPM;
    }
    if (ev.m_iShiftRPM > 0)
    {
        m_shiftRpm = ev.m_iShiftRPM;
    }
    else if (ev.m_iLimiter > 0)
    {
        m_shiftRpm = ev.m_iLimiter;
    }
    if (ev.m_fMaxFuel > 0.0f)
    {
        m_maxFuel = ev.m_fMaxFuel;
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

void PluginState::beginRun()
{
    m_inRun = true;
    clearSpectateSelection();
    for (int i = 0; i < 3; ++i)
    {
        m_sectorCur[i] = 0;
    }
    m_sectorLast = -1;
    m_sectorFinishedLap = -1;
    m_lastLapEndTime = m_sessionTime;
}

void PluginState::endRun()
{
    m_inRun = false;
    m_hasTelemetry = false;
    m_telemetryStamp = 0.0;
    m_trackPosStamp = 0.0;
    m_trackPos.clear();
    clearSpectateSelection();
}

bool PluginState::onTrack() const
{
    // Replay / spectate never call RunInit, but classification and positions still stream.
    return m_inRun || m_hasTelemetry || !m_standings.empty() || !m_trackPos.empty();
}

namespace
{
    bool likelyStartCountdown(int len)
    {
        if (len >= 15 && len <= 180)
        {
            return true;
        }
        // Remaining time published as milliseconds (00:50 board = 50000).
        return len >= 1000 && len <= 180 * 1000;
    }

    bool likelySessionMinutes(int len)
    {
        return len >= 3 && len < 15;
    }

    bool practiceSized(int len)
    {
        if (len >= 30 * 60000)
        {
            return true;
        }
        return len >= 30 && len < 60;
    }

    bool raceMinutes(int len)
    {
        if (len >= 3 * 60000 && len <= 20 * 60000)
        {
            return true;
        }
        return len >= 3 && len <= 20 && len < 60;
    }

    bool warmupSized(int len)
    {
        if (practiceSized(len))
        {
            return true;
        }
        return len == 10 || len == 10 * 60000;
    }

    // Leftover warmup / previous timed session. A 4+ lap moto must not inherit 8:00.
    bool leftoverTimedLength(int len)
    {
        return warmupSized(len) || raceMinutes(len);
    }

    int lengthToMs(int len, int totalLen)
    {
        if (len <= 0)
        {
            return 0;
        }
        if (totalLen < 0)
        {
            totalLen = 0;
        }
        // 1000–100000 is already milliseconds (e.g. 01:40 remaining = 99992).
        // Treating that as seconds made the overlay jump back to 08:00.
        if (len >= 1000)
        {
            return len;
        }
        if (len >= 60)
        {
            return len * 1000;
        }
        if (totalLen >= 60 || (totalLen > 0 && len > totalLen))
        {
            return len * 1000;
        }
        return len * 60000;
    }
}

void PluginState::noteSessionKind(int kind)
{
    if (m_sessionKind >= 0 && kind != m_sessionKind)
    {
        m_sessionLength = kSessionLengthUnset;
        m_sessionRemain = 0;
        m_sessionLaps = 0;
    }
    m_sessionKind = kind;
}

void PluginState::applySessionLength(int len)
{
    if (len <= 0)
    {
        if (m_sessionLength < 0)
        {
            m_sessionLength = 0;
        }
        return;
    }
    if (m_sessionLength <= 0)
    {
        if (likelyStartCountdown(len))
        {
            m_sessionRemain = len;
            return;
        }
        if (m_sessionLaps >= 4 && leftoverTimedLength(len))
        {
            return;
        }
        m_sessionLength = len;
        return;
    }
    if (len > m_sessionLength)
    {
        // Locked minutes + a larger 15–180 value is the gate / remaining seconds.
        if (m_sessionLength < 60 && !likelyStartCountdown(m_sessionLength) && likelyStartCountdown(len))
        {
            m_sessionRemain = len;
            return;
        }
        if (m_sessionLength < 60 && !likelyStartCountdown(m_sessionLength) && len >= 60)
        {
            m_sessionRemain = len;
            return;
        }
        m_sessionLength = len;
        return;
    }
    // Shrinking is remaining time. Only steal the locked total when we captured
    // a gate first (30s) and the real race minutes (8) arrive later — not when
    // practice remaining ticks 15, 14, …, 8.
    if (likelyStartCountdown(m_sessionLength)
        && likelySessionMinutes(len)
        && (m_sessionRemain <= 0 || m_sessionRemain == m_sessionLength))
    {
        m_sessionRemain = m_sessionLength;
        m_sessionLength = len;
        return;
    }
    // 40:00 practice leftover must not block 8:00 when extras / laps arrive.
    if (m_sessionLaps > 0 && practiceSized(m_sessionLength) && raceMinutes(len))
    {
        m_sessionLength = len;
        m_sessionRemain = len;
    }
}

void PluginState::setSession(const SPluginsRaceSession_t& s)
{
    m_airTemp = s.m_fAirTemperature;
    const int len = s.m_iSessionLength;
    const int laps = s.m_iSessionNumLaps;
    const bool newKind = m_sessionKind >= 0 && s.m_iSession != m_sessionKind;
    const bool newFormat = laps != m_sessionLaps
        || (len > 0 && m_sessionLength > 0 && len != m_sessionLength && len < 60
            && m_sessionLength < 60 && !likelyStartCountdown(len)
            && !likelyStartCountdown(m_sessionLength));
    // Kind change drops cached length/laps so leftover 8:00 is not locked
    // when this session publishes 0. -1 = not written yet; 0 = game sent 0.
    noteSessionKind(s.m_iSession);
    m_sessionState = s.m_iSessionState;
    // Don't let extras (1–3) replace a 4+ lap moto unless the session kind changed.
    if (!(m_sessionLaps >= 4 && laps > 0 && laps < 4 && !newKind))
    {
        m_sessionLaps = laps;
    }
    if (laps > 0 && practiceSized(m_sessionLength)
        && (len <= 0 || practiceSized(len) || len == m_sessionLength))
    {
        m_sessionLength = 0;
        m_sessionRemain = 0;
    }
    // Leftover 00:50 start board must not become the length of a 2-lap moto.
    if (m_sessionLaps >= 2 && likelyStartCountdown(m_sessionLength)
        && (len <= 0 || likelyStartCountdown(len) || len == m_sessionLength))
    {
        m_sessionLength = 0;
        m_sessionRemain = len > 0 ? len : m_sessionRemain;
    }
    // Leftover 8:00 / 10:00 must not become the length of a 4+ lap moto.
    if (m_sessionLaps >= 4 && leftoverTimedLength(m_sessionLength)
        && (len <= 0 || leftoverTimedLength(len) || len == m_sessionLength))
    {
        m_sessionLength = 0;
        m_sessionRemain = 0;
    }
    // Warmup 10:00 then race 8:00 + 2L is a new format — don't keep the 10 minute lock.
    // A 4+ lap moto still ignores leftover 8:00 / 10:00 even if extras republish.
    if ((newKind || newFormat) && len > 0 && !likelyStartCountdown(len) && !practiceSized(len)
        && !(m_sessionLaps >= 4 && leftoverTimedLength(len)))
    {
        m_sessionLength = len;
        m_sessionRemain = len;
        return;
    }
    applySessionLength(len);
    if (m_sessionLength < 0 && (len <= 0 || !likelyStartCountdown(len)))
    {
        m_sessionLength = 0;
    }
    if (len > 0 && m_sessionRemain <= 0 && m_sessionLength > 0)
    {
        m_sessionRemain = len;
    }
}

void PluginState::setSessionState(const SPluginsRaceSessionState_t& s)
{
    noteSessionKind(s.m_iSession);
    m_sessionState = s.m_iSessionState;
    applySessionLength(s.m_iSessionLength);
    if (m_sessionLength < 0 && (s.m_iSessionLength <= 0 || !likelyStartCountdown(s.m_iSessionLength)))
    {
        m_sessionLength = 0;
    }
    if (s.m_iSessionLength <= 0 || m_sessionLength <= 0)
    {
        return;
    }
    // Don't pin remaining to a republished warmup/race total (10:00) if we already
    // have a clock. Update when the value looks like remaining or a new length.
    if (m_sessionRemain <= 0 || s.m_iSessionLength != m_sessionLength)
    {
        m_sessionRemain = s.m_iSessionLength;
    }
}

int PluginState::remainToMs() const
{
    return lengthToMs(m_sessionRemain, m_sessionLength);
}

void PluginState::setLocalLap(int lapNum, int lapMs)
{
    m_lastLapMs = lapMs;
    if (lapMs > 0 && (m_bestLapMs <= 0 || lapMs < m_bestLapMs))
    {
        m_bestLapMs = lapMs;
    }
    m_currentLap = lapNum + 1;
    m_lastLapEndTime = m_sessionTime;
    if (m_localRaceNum >= 0)
    {
        m_lastLaps[m_localRaceNum] = lapMs;
    }
}

void PluginState::setRaceLap(int raceNum, int lapNum, int lapMs, int split0, int split1)
{
    if (lapMs > 0)
    {
        m_lastLaps[raceNum] = lapMs;
    }
    const int focus = focusRaceNum();
    if (raceNum != focus && raceNum != m_localRaceNum)
    {
        return;
    }
    setLocalLap(lapNum, lapMs);
    if (lapMs > 0)
    {
        finishLapSectors(lapNum, lapMs, split0, split1);
    }
}

void PluginState::setLocalSplit(int split, int timeMs, int bestDiff)
{
    if (m_localRaceNum >= 0 && focusRaceNum() != m_localRaceNum)
    {
        return;
    }
    recordSector(mapSplitIndex(split), timeMs, bestDiff);
}

void PluginState::setRaceSplit(int raceNum, int split, int timeMs)
{
    const int focus = focusRaceNum();
    if (raceNum != focus && raceNum != m_localRaceNum)
    {
        return;
    }
    recordSector(mapSplitIndex(split), timeMs, 0);
}

int PluginState::sectorAt(const int* values, int i)
{
    if (i < 0 || i >= 3)
    {
        return 0;
    }
    return values[i];
}

int PluginState::mapSplitIndex(int split) const
{
    if (m_sectorCur[0] <= 0)
    {
        return 0;
    }
    if (m_sectorCur[1] <= 0)
    {
        return split <= 0 ? 0 : 1;
    }
    if (m_sectorCur[2] <= 0 && split >= 2)
    {
        return 2;
    }
    return 1;
}

void PluginState::recordSector(int idx, int timeMs, int bestDiff)
{
    if (idx < 0 || idx >= 3 || timeMs <= 0)
    {
        return;
    }
    // RunSplit and RaceSplit both fire for the same split. A second write after
    // the session best was already updated stores delta 0 on a new PB.
    if (m_sectorCur[idx] > 0)
    {
        return;
    }
    const int oldBest = m_sectorBest[idx];
    m_sectorCur[idx] = timeMs;
    m_sectorLast = idx;
    if (idx == 0)
    {
        m_sectorCur[1] = 0;
        m_sectorCur[2] = 0;
        m_sectorLastLap[1] = 0;
        m_sectorLastLap[2] = 0;
        m_sectorDelta[1] = 0;
        m_sectorDelta[2] = 0;
        m_sectorDeltaValid &= ~0x6;
    }
    if (oldBest > 0)
    {
        m_sectorDelta[idx] = timeMs - oldBest;
        m_sectorDeltaValid |= (1 << idx);
    }
    else if (bestDiff != 0)
    {
        m_sectorDelta[idx] = bestDiff;
        m_sectorDeltaValid |= (1 << idx);
    }
    else
    {
        m_sectorDelta[idx] = 0;
        m_sectorDeltaValid &= ~(1 << idx);
    }
    if (oldBest <= 0 || timeMs < oldBest)
    {
        m_sectorBest[idx] = timeMs;
    }
}

void PluginState::finishLapSectors(int lapNum, int lapMs, int split0, int split1)
{
    if (lapNum == m_sectorFinishedLap && m_sectorLastLap[2] > 0)
    {
        return;
    }
    if (m_sectorCur[0] <= 0 && split0 > 0)
    {
        m_sectorCur[0] = split0;
    }
    if (m_sectorCur[1] <= 0 && split1 > 0)
    {
        m_sectorCur[1] = split1;
    }
    if (m_sectorCur[0] <= 0)
    {
        m_sectorCur[0] = m_sectorLastLap[0];
    }
    if (m_sectorCur[1] <= 0)
    {
        m_sectorCur[1] = m_sectorLastLap[1];
    }
    m_sectorLastLap[2] = 0;
    int s3 = sectorThreeMs(m_sectorCur[0], m_sectorCur[1], lapMs);
    if (s3 <= 0 && m_sectorCur[1] > 0 && lapMs > m_sectorCur[1])
    {
        s3 = lapMs - m_sectorCur[1];
    }
    if (s3 > 0)
    {
        recordSector(2, s3, 0);
    }
    for (int i = 0; i < 3; ++i)
    {
        if (m_sectorCur[i] > 0)
        {
            m_sectorLastLap[i] = m_sectorCur[i];
        }
        m_sectorCur[i] = 0;
    }
    if (m_sectorLastLap[2] > 0)
    {
        m_sectorLast = 2;
        m_sectorFinishedLap = lapNum;
    }
}

int PluginState::sessionTimeMs() const
{
    const int remainMs = remainToMs();
    const int totalMs = lengthToMs(m_sessionLength, m_sessionLength);
    const bool racing = m_currentLap > 1 || m_localSpeed >= 3.5f;
    const bool shortRemain = remainMs > 0 && remainMs <= 180000 && (totalMs <= 0 || remainMs * 3 < totalMs);
    int clockMs = 0;
    if (m_sessionClock > 0)
    {
        clockMs = lengthToMs(m_sessionClock, m_sessionLength);
        if (totalMs > 0 && clockMs > totalMs + 30000 && clockMs > 180000)
        {
            clockMs = 0;
        }
    }
    const bool clockLooksPrestart = clockMs > 0 && clockMs <= 180000
        && (totalMs <= 0 || clockMs * 3 < totalMs);
    const bool remainLooksRace = remainMs > 0 && totalMs > 0 && remainMs + 15000 >= totalMs;
    // Classification often holds the 2:00/0:30 board while session length stays at 10:00.
    if (!racing && clockLooksPrestart && (remainLooksRace || remainMs <= 0 || remainMs > 180000))
    {
        return clockMs;
    }
    if (!racing && shortRemain && (clockMs <= 0 || remainMs + 2000 < clockMs))
    {
        return remainMs;
    }
    // Session state often republishes the 10:00 warmup length while classification
    // already has the ticking remaining clock.
    if (clockMs > 0 && totalMs > 0 && remainMs + 20000 >= totalMs
        && clockMs + 1500 <= remainMs && clockMs <= totalMs + 2000)
    {
        return clockMs;
    }
    if (remainMs > 0 && (totalMs <= 0 || remainMs <= totalMs + 2000) && !shortRemain && !clockLooksPrestart)
    {
        return remainMs;
    }
    if (clockMs > 0)
    {
        return clockMs;
    }
    if (remainMs > 0)
    {
        return remainMs;
    }
    if (m_sessionTime > 0.0f)
    {
        return static_cast<int>(m_sessionTime * 1000.0f);
    }
    return 0;
}

int PluginState::currentLapMs() const
{
    if (m_lastLapEndTime > 0.0f && m_sessionTime >= m_lastLapEndTime)
    {
        const float dt = m_sessionTime - m_lastLapEndTime;
        if (dt > 200.0f)
        {
            return static_cast<int>(dt);
        }
        return static_cast<int>(dt * 1000.0f);
    }
    return m_lastLapMs;
}

void PluginState::addEntry(const SPluginsRaceAddEntry_t& e)
{
    RaceEntry& row = m_entries[e.m_iRaceNum];
    row.raceNum = e.m_iRaceNum;
    row.name = copyCString(e.m_szName, sizeof(e.m_szName));
    row.bikeShort = copyCString(e.m_szBikeShortName, sizeof(e.m_szBikeShortName));
    row.category = copyCString(e.m_szCategory, sizeof(e.m_szCategory));
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

void PluginState::setClassification(const SPluginsRaceClassification_t& header,
                                    const SPluginsRaceClassificationEntry_t* entries,
                                    int count)
{
    if (header.m_iSessionTime > 0)
    {
        m_sessionClock = header.m_iSessionTime;
    }
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
        auto it = m_lastLaps.find(row.raceNum);
        row.lastLapMs = it != m_lastLaps.end() ? it->second : 0;
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
    if (count > 0)
    {
        m_trackPosStamp = pluginNowSeconds();
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
        m_mapRev++;
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
    m_mapRev++;
}

void PluginState::setTelemetry(const SPluginsBikeData_t& data, float time, float pos)
{
    m_hasTelemetry = true;
    m_sessionTime = time;
    if (m_inRun && m_lastLapEndTime <= 0.0f && time > 0.0f)
    {
        m_lastLapEndTime = time;
    }
    m_localGear = data.m_iGear;
    m_localRpm = data.m_iRPM;
    m_engineTemp = data.m_fWaterTemperature > 1.0f ? data.m_fWaterTemperature : data.m_fEngineTemperature;
    m_fuel = data.m_fFuel;
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
        if (dx * dx + dz * dz > kTrailMinDistSq)
        {
            m_trail.push_back({m_localX, m_localZ});
            m_trailLastX = m_localX;
            m_trailLastZ = m_localZ;
            if (m_trail.size() > kTrailMaxPoints)
            {
                m_trail.erase(m_trail.begin(), m_trail.begin() + kTrailTrimBatch);
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
    m_lastSpectate = pluginNowSeconds();
}

void PluginState::clearSpectateSelection()
{
    m_focusRaceNum = -1;
    m_lastSpectate = 0.0;
}

bool PluginState::spectating() const
{
    // Replay calls SpectateVehicles every frame, so this stays true. Riding does
    // not: drop camera focus as soon as the callback stops so the map follows you.
    return m_lastSpectate > 0.0 && (pluginNowSeconds() - m_lastSpectate) < 0.25;
}

int PluginState::focusRaceNum() const
{
    if (m_focusRaceNum >= 0 && spectating())
    {
        return m_focusRaceNum;
    }
    return m_localRaceNum;
}

const VehicleLive* PluginState::findVehicle(int raceNum) const
{
    const auto it = m_vehicles.find(raceNum);
    if (it == m_vehicles.end())
    {
        return nullptr;
    }
    return &it->second;
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
