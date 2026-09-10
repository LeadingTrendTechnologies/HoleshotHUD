#ifndef NOMINMAX
#define NOMINMAX
#endif
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif

#include "shm_publish.h"
#include "str_util.h"
#include "track_geom.h"

#include <algorithm>
#include <cstddef>
#include <cmath>
#include <cstring>
#include <windows.h>

void tessellateTrack(const PluginState& state, std::vector<MxboShmPoint>& pts)
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

void fillSnapshot(MxboShmSnapshot& local,
                  const PluginState& state,
                  const PluginConfig& config,
                  const MxboShmPoint* poly,
                  int polyCount,
                  uint64_t tickQpc)
{
    local.magic = MXBO_SHM_MAGIC;
    local.version = MXBO_SHM_VERSION;
    local.size = static_cast<uint32_t>(sizeof(MxboShmSnapshot));
    local.tickQpc = tickQpc;

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
    local.localRoll = state.localRoll();
    local.localPitch = state.localPitch();
    local.localSteer = state.localSteer();
    local.steerLock = state.steerLock();
    copyBounded(local.trackName, MXBO_TRACK_NAME, state.trackName().c_str());
    copyBounded(local.setupName, MXBO_TRACK_NAME, state.setupName().c_str());
    local.trackLength = state.trackLength();
    local.sfMeters = state.startFinishMeters();

    local.polyCount = (std::min)(polyCount, MXBO_MAX_POLY);
    if (local.polyCount > 0 && poly)
    {
        std::memcpy(local.poly, poly, static_cast<size_t>(local.polyCount) * sizeof(MxboShmPoint));
    }

    // Leftover RaceTrackPosition after spectate (garage) is not a session.
    const bool liveRiders = state.hasTelemetry() || state.spectating();
    const int nRiders = liveRiders
        ? std::min(static_cast<int>(state.trackPositions().size()), MXBO_MAX_RIDERS)
        : 0;
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
        const VehicleLive* live = state.findVehicle(p.raceNum);
        d.lean = live ? live->lean : 0.0f;
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
    // BikeData has clutch + both brakes. Other riders only publish throttle / front brake.
    if (live && live->active && focus >= 0 && focus != state.localRaceNum())
    {
        local.localThrottle = live->throttle;
        local.localFrontBrake = live->frontBrake;
        local.localRearBrake = 0.0f;
        local.localClutch = 0.0f;
    }
    else
    {
        local.localThrottle = state.localThrottle();
        local.localFrontBrake = state.localFrontBrake();
        local.localRearBrake = state.localRearBrake();
        local.localClutch = state.localClutch();
    }
    local.engineTemp = state.engineTemp();
    local.airTemp = state.airTemp();
    local.fuel = state.fuel();
    local.maxFuel = state.maxFuel();
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
}

void seqlockStore(MxboShmSnapshot& dst, const MxboShmSnapshot& local)
{
    const uint32_t odd = dst.seq | 1u;
    dst.seq = odd;
    MemoryBarrier();
    constexpr size_t kSkip = offsetof(MxboShmSnapshot, size);
    std::memcpy(reinterpret_cast<uint8_t*>(&dst) + kSkip,
                reinterpret_cast<const uint8_t*>(&local) + kSkip,
                sizeof(MxboShmSnapshot) - kSkip);
    dst.magic = MXBO_SHM_MAGIC;
    dst.version = MXBO_SHM_VERSION;
    MemoryBarrier();
    dst.seq = odd + 1u;
}
