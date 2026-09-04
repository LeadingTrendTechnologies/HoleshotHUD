#ifndef NOMINMAX
#define NOMINMAX
#endif
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif

#include "shm_writer.h"
#include "shm/mxbo_shm.h"
#include "config.h"
#include "state.h"
#include "steam_friends.h"
#include "str_util.h"
#include "track_geom.h"

#include <algorithm>
#include <cstddef>
#include <cmath>
#include <cstring>
#include <vector>
#include <windows.h>

namespace
{
    void tessellateInto(const PluginState& state, std::vector<MxboShmPoint>& pts)
    {
        pts.clear();
        pts.reserve(512);

        const auto& segs = state.centerline();
        if (!segs.empty())
        {
            auto push = [&](float px, float pz) {
                if (!pts.empty())
                {
                    const float dx = px - pts.back().x;
                    const float dz = pz - pts.back().z;
                    if (dx * dx + dz * dz < track_geom::kDedupDistSq)
                    {
                        return;
                    }
                }
                pts.push_back(MxboShmPoint{px, pz});
            };
            track_geom::walkCenterline(segs, push);
        }
        else
        {
            for (const auto& p : state.trail())
            {
                pts.push_back(MxboShmPoint{p.first, p.second});
            }
            if (state.hasTelemetry() &&
                (pts.empty() ||
                 std::fabs(pts.back().x - state.localX()) + std::fabs(pts.back().z - state.localZ()) > 0.5f))
            {
                pts.push_back(MxboShmPoint{state.localX(), state.localZ()});
            }
        }

        if (pts.size() > MXBO_MAX_POLY)
        {
            const float stride = static_cast<float>(pts.size() - 1) / static_cast<float>(MXBO_MAX_POLY - 1);
            std::vector<MxboShmPoint> thin;
            thin.resize(MXBO_MAX_POLY);
            for (int i = 0; i < MXBO_MAX_POLY; ++i)
            {
                const int idx = std::min(static_cast<int>(pts.size()) - 1, static_cast<int>(i * stride));
                thin[static_cast<size_t>(i)] = pts[static_cast<size_t>(idx)];
            }
            pts.swap(thin);
        }
    }
}

bool ShmWriter::open()
{
    close();
    const DWORD bytes = static_cast<DWORD>(sizeof(MxboShmSnapshot));
    SetLastError(0);
    m_map = CreateFileMappingW(
        INVALID_HANDLE_VALUE,
        nullptr,
        PAGE_READWRITE,
        0,
        bytes,
        MXBO_SHM_NAME);
    if (!m_map)
    {
        return false;
    }
    const bool existed = GetLastError() == ERROR_ALREADY_EXISTS;
    // An existing smaller section (old plugin) must not be memset/memcpy past its
    // end — that corrupts the game process and can GS-fail in mxbikes.exe.
    if (existed)
    {
        MEMORY_BASIC_INFORMATION info{};
        m_view = MapViewOfFile(m_map, FILE_MAP_ALL_ACCESS, 0, 0, 0);
        if (!m_view || !VirtualQuery(m_view, &info, sizeof(info)) || info.RegionSize < bytes)
        {
            close();
            return false;
        }
        UnmapViewOfFile(m_view);
        m_view = nullptr;
    }
    m_view = MapViewOfFile(m_map, FILE_MAP_ALL_ACCESS, 0, 0, bytes);
    if (!m_view)
    {
        CloseHandle(m_map);
        m_map = nullptr;
        return false;
    }
    MEMORY_BASIC_INFORMATION mapped{};
    if (!VirtualQuery(m_view, &mapped, sizeof(mapped)) || mapped.RegionSize < bytes)
    {
        close();
        return false;
    }
    std::memset(m_view, 0, bytes);
    auto* snap = static_cast<MxboShmSnapshot*>(m_view);
    snap->magic = MXBO_SHM_MAGIC;
    snap->version = MXBO_SHM_VERSION;
    snap->size = static_cast<uint32_t>(sizeof(MxboShmSnapshot));
    snap->seq = 0;

    m_cmdMap = CreateFileMappingW(
        INVALID_HANDLE_VALUE,
        nullptr,
        PAGE_READWRITE,
        0,
        static_cast<DWORD>(sizeof(MxboShmCmd)),
        MXBO_CMD_NAME);
    if (m_cmdMap)
    {
        m_cmdView = MapViewOfFile(m_cmdMap, FILE_MAP_ALL_ACCESS, 0, 0, sizeof(MxboShmCmd));
        if (m_cmdView)
        {
            auto* cmd = static_cast<MxboShmCmd*>(m_cmdView);
            if (cmd->magic != MXBO_CMD_MAGIC)
            {
                std::memset(cmd, 0, sizeof(MxboShmCmd));
                cmd->magic = MXBO_CMD_MAGIC;
            }
        }
        else
        {
            CloseHandle(m_cmdMap);
            m_cmdMap = nullptr;
        }
    }
    return true;
}

void ShmWriter::close()
{
    if (m_view)
    {
        UnmapViewOfFile(m_view);
        m_view = nullptr;
    }
    if (m_map)
    {
        CloseHandle(m_map);
        m_map = nullptr;
    }
    if (m_cmdView)
    {
        UnmapViewOfFile(m_cmdView);
        m_cmdView = nullptr;
    }
    if (m_cmdMap)
    {
        CloseHandle(m_cmdMap);
        m_cmdMap = nullptr;
    }
    m_lastSpectateQpc = 0;
    m_poly.clear();
    m_polyCount = 0;
    m_polyRev = 0xFFFFFFFFu;
    m_polyTrail = static_cast<size_t>(-1);
}

void ShmWriter::fillPolyCache(const PluginState& state)
{
    const bool lined = !state.centerline().empty();
    const uint32_t rev = state.mapRev();
    const size_t trail_n = lined ? 0 : state.trail().size();
    if (m_polyRev == rev && m_polyTrail == trail_n)
    {
        return;
    }
    tessellateInto(state, m_poly);
    m_polyCount = static_cast<int32_t>(m_poly.size());
    m_polyRev = rev;
    m_polyTrail = trail_n;
}

void ShmWriter::publish(const PluginState& state, const PluginConfig& config)
{
    if (!m_view)
    {
        return;
    }

    MxboShmSnapshot local{};
    local.magic = MXBO_SHM_MAGIC;
    local.version = MXBO_SHM_VERSION;
    local.size = static_cast<uint32_t>(sizeof(MxboShmSnapshot));

    LARGE_INTEGER qpc{};
    QueryPerformanceCounter(&qpc);
    local.tickQpc = static_cast<uint64_t>(qpc.QuadPart);

    local.localRaceNum = state.localRaceNum();
    local.focusRaceNum = state.focusRaceNum();
    local.hasTelemetry = state.hasTelemetry() ? 1 : 0;
    local.onTrack = state.onTrack() ? 1 : 0;
    local.maxRpm = state.maxRpm();
    local.shiftRpm = state.shiftRpm();
    local.localCrashed = state.localCrashed();
    local.localX = state.localX();
    local.localZ = state.localZ();
    local.localVelX = state.localVelX();
    local.localVelZ = state.localVelZ();
    local.localYaw = state.localYaw();
    local.localSpeed = state.localSpeed();
    local.localTrackPos = state.localTrackPos();
    copyBounded(local.trackName, MXBO_TRACK_NAME, state.trackName().c_str());
    local.trackLength = state.trackLength();
    local.sfMeters = state.startFinishMeters();

    fillPolyCache(state);
    local.polyCount = (std::min)(m_polyCount, MXBO_MAX_POLY);
    if (local.polyCount > 0)
    {
        std::memcpy(local.poly, m_poly.data(), static_cast<size_t>(local.polyCount) * sizeof(MxboShmPoint));
    }

    const int nRiders = std::min(static_cast<int>(state.trackPositions().size()), MXBO_MAX_RIDERS);
    local.riderCount = nRiders;
    for (int i = 0; i < nRiders; ++i)
    {
        const TrackPos& p = state.trackPositions()[static_cast<size_t>(i)];
        MxboShmRider& d = local.riders[i];
        d.raceNum = p.raceNum;
        d.x = p.x;
        d.z = p.z;
        d.yaw = p.yaw;
        d.trackPos = p.trackPos;
        d.crashed = p.crashed;
        const RaceEntry* e = state.findEntry(p.raceNum);
        copyBounded(d.name, MXBO_NAME, e ? e->name.c_str() : "");
    }

    const int nStand = std::min(static_cast<int>(state.standings().size()), MXBO_MAX_STANDINGS);
    local.standingCount = nStand;
    for (int i = 0; i < nStand; ++i)
    {
        const StandingRow& s = state.standings()[static_cast<size_t>(i)];
        MxboShmStanding& d = local.standings[i];
        d.raceNum = s.raceNum;
        d.position = s.position;
        d.state = s.state;
        d.bestLapMs = s.bestLapMs;
        d.numLaps = s.numLaps;
        d.gapMs = s.gapMs;
        d.gapLaps = s.gapLaps;
        d.pit = s.pit;
        d.penaltyMs = s.penaltyMs;
        const TrackPos* tp = state.findTrackPos(s.raceNum);
        d.crashed = tp ? tp->crashed : 0;
        const RaceEntry* e = state.findEntry(s.raceNum);
        copyBounded(d.name, MXBO_NAME, e ? e->name.c_str() : "");
        copyBounded(d.bike, MXBO_NAME, e ? e->bikeShort.c_str() : "");
        d.lastLapMs = s.lastLapMs;
        copyBounded(d.category, MXBO_NAME, e ? e->category.c_str() : "");
    }

    local.map = MxboShmRect{config.map.x, config.map.y, config.map.w, config.map.h};
    local.standingsRect = MxboShmRect{config.standings.x, config.standings.y, config.standings.w, config.standings.h};
    local.relative = MxboShmRect{config.relative.x, config.relative.y, config.relative.w, config.relative.h};
    local.showMap = config.showMap ? 1 : 0;
    local.showStandings = config.showStandings ? 1 : 0;
    local.showRelative = config.showRelative ? 1 : 0;
    local.standingsRows = config.standingsRows;
    local.relativeCount = config.relativeCount;

    const int focus = state.focusRaceNum();
    const VehicleLive* live = state.findVehicle(focus);
    if (live && live->active)
    {
        local.localGear = live->gear;
        local.localRpm = live->rpm;
    }
    else
    {
        local.localGear = state.localGear();
        local.localRpm = state.localRpm();
    }
    local.engineTemp = state.engineTemp();
    local.airTemp = state.airTemp();
    local.fuel = state.fuel();
    local.maxFuel = state.maxFuel();
    copyBounded(local.guid, MXBO_GUID, state.guid().c_str());
    copyBounded(local.serverName, MXBO_SERVER_NAME, state.serverName().c_str());
    local.serverType = state.serverType();
    steamFriendsCopy(&local.localSteamId, &local.friendCount, local.friends, MXBO_MAX_FRIENDS);
    local.friendPad = 0;
    local.lastLapMs = state.lastLapMs();
    local.bestLapMs = state.bestLapMs();
    local.currentLapMs = state.currentLapMs();
    int lap = state.currentLap();
    if (const StandingRow* st = state.findStanding(focus))
    {
        lap = std::max(lap, st->numLaps);
    }
    local.currentLap = lap;
    local.sessionLaps = state.sessionLaps();
    local.sessionKind = state.sessionKind();
    local.sessionState = state.sessionState();
    local.sessionTimeMs = state.sessionTimeMs();
    // Plugin uses -1 = unset this session; never publish that into SHM.
    local.sessionLength = std::max(0, state.sessionLength());

    local.sectorCount = state.sectorCount();
    local.sectorLast = state.sectorLast();
    local.sectorDeltaValid = state.sectorDeltaValid();
    for (int i = 0; i < MXBO_MAX_SECTORS; ++i)
    {
        local.sectorCur[i] = state.sectorCur(i);
        local.sectorLastLap[i] = state.sectorLastLap(i);
        local.sectorBest[i] = state.sectorBest(i);
        local.sectorDelta[i] = state.sectorDelta(i);
    }

    auto* dst = static_cast<MxboShmSnapshot*>(m_view);
    const uint32_t odd = dst->seq | 1u;
    dst->seq = odd;
    MemoryBarrier();
    constexpr size_t kSkip = offsetof(MxboShmSnapshot, size);
    std::memcpy(reinterpret_cast<uint8_t*>(dst) + kSkip,
                reinterpret_cast<const uint8_t*>(&local) + kSkip,
                sizeof(MxboShmSnapshot) - kSkip);
    dst->magic = MXBO_SHM_MAGIC;
    dst->version = MXBO_SHM_VERSION;
    MemoryBarrier();
    dst->seq = odd + 1u;
    decaySpectating();
}

void ShmWriter::noteSpectating()
{
    LARGE_INTEGER qpc{};
    QueryPerformanceCounter(&qpc);
    m_lastSpectateQpc = qpc.QuadPart;
    if (m_cmdView)
    {
        static_cast<MxboShmCmd*>(m_cmdView)->spectating = 1;
    }
}

int ShmWriter::takeSpectateRequest()
{
    if (!m_cmdView)
    {
        return 0;
    }
    auto* cmd = static_cast<MxboShmCmd*>(m_cmdView);
    const LONG want = InterlockedExchange(reinterpret_cast<LONG*>(&cmd->spectateRaceNum), 0);
    return static_cast<int>(want);
}

void ShmWriter::decaySpectating()
{
    if (!m_cmdView)
    {
        return;
    }
    auto* cmd = static_cast<MxboShmCmd*>(m_cmdView);
    if (m_lastSpectateQpc == 0)
    {
        cmd->spectating = 0;
        return;
    }
    LARGE_INTEGER now{};
    LARGE_INTEGER freq{};
    QueryPerformanceCounter(&now);
    QueryPerformanceFrequency(&freq);
    const double age = freq.QuadPart > 0
        ? static_cast<double>(now.QuadPart - m_lastSpectateQpc) / static_cast<double>(freq.QuadPart)
        : 1.0;
    if (age > 0.25)
    {
        cmd->spectating = 0;
    }
}
