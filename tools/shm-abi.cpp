// Dumps C layout of MxboShmSnapshot / MxboShmCmd. Must match overlay/hud snapshot.rs
// and overlay/src/shm.rs CmdView. CI diffs this against src/shm/abi.txt.
#include "shm/mxbo_shm.h"

#include <cstddef>
#include <cstdio>

#define FIELD(T, f) \
    std::printf("%s.%s %zu %zu\n", #T, #f, offsetof(T, f), sizeof(static_cast<T *>(nullptr)->f))

int main()
{
    std::printf("MAGIC=0x%X\n", MXBO_SHM_MAGIC);
    std::printf("VERSION=%u\n", MXBO_SHM_VERSION);
    std::printf("CMD_MAGIC=0x%X\n", MXBO_CMD_MAGIC);
    std::printf("MAX_POLY=%d\n", MXBO_MAX_POLY);
    std::printf("MAX_RIDERS=%d\n", MXBO_MAX_RIDERS);
    std::printf("MAX_STANDINGS=%d\n", MXBO_MAX_STANDINGS);
    std::printf("MAX_SECTORS=%d\n", MXBO_MAX_SECTORS);
    std::printf("NAME=%d\n", MXBO_NAME);
    std::printf("TRACK_NAME=%d\n", MXBO_TRACK_NAME);

    std::printf("MxboShmPoint.size %zu\n", sizeof(MxboShmPoint));
    FIELD(MxboShmPoint, x);
    FIELD(MxboShmPoint, z);

    std::printf("MxboShmRider.size %zu\n", sizeof(MxboShmRider));
    FIELD(MxboShmRider, raceNum);
    FIELD(MxboShmRider, x);
    FIELD(MxboShmRider, z);
    FIELD(MxboShmRider, yaw);
    FIELD(MxboShmRider, trackPos);
    FIELD(MxboShmRider, crashed);
    FIELD(MxboShmRider, name);
    FIELD(MxboShmRider, lean);

    std::printf("MxboShmStanding.size %zu\n", sizeof(MxboShmStanding));
    FIELD(MxboShmStanding, raceNum);
    FIELD(MxboShmStanding, position);
    FIELD(MxboShmStanding, state);
    FIELD(MxboShmStanding, bestLapMs);
    FIELD(MxboShmStanding, numLaps);
    FIELD(MxboShmStanding, gapMs);
    FIELD(MxboShmStanding, gapLaps);
    FIELD(MxboShmStanding, pit);
    FIELD(MxboShmStanding, penaltyMs);
    FIELD(MxboShmStanding, crashed);
    FIELD(MxboShmStanding, name);
    FIELD(MxboShmStanding, bike);
    FIELD(MxboShmStanding, lastLapMs);
    FIELD(MxboShmStanding, category);

    std::printf("MxboShmRect.size %zu\n", sizeof(MxboShmRect));
    FIELD(MxboShmRect, x);
    FIELD(MxboShmRect, y);
    FIELD(MxboShmRect, w);
    FIELD(MxboShmRect, h);

    std::printf("MxboShmSnapshot.size %zu\n", sizeof(MxboShmSnapshot));
    FIELD(MxboShmSnapshot, magic);
    FIELD(MxboShmSnapshot, version);
    FIELD(MxboShmSnapshot, seq);
    FIELD(MxboShmSnapshot, size);
    FIELD(MxboShmSnapshot, tickQpc);
    FIELD(MxboShmSnapshot, localRaceNum);
    FIELD(MxboShmSnapshot, focusRaceNum);
    FIELD(MxboShmSnapshot, hasTelemetry);
    FIELD(MxboShmSnapshot, localCrashed);
    FIELD(MxboShmSnapshot, localX);
    FIELD(MxboShmSnapshot, localZ);
    FIELD(MxboShmSnapshot, localVelX);
    FIELD(MxboShmSnapshot, localVelZ);
    FIELD(MxboShmSnapshot, localYaw);
    FIELD(MxboShmSnapshot, localSpeed);
    FIELD(MxboShmSnapshot, localTrackPos);
    FIELD(MxboShmSnapshot, trackName);
    FIELD(MxboShmSnapshot, trackLength);
    FIELD(MxboShmSnapshot, sfMeters);
    FIELD(MxboShmSnapshot, polyCount);
    FIELD(MxboShmSnapshot, poly);
    FIELD(MxboShmSnapshot, riderCount);
    FIELD(MxboShmSnapshot, riders);
    FIELD(MxboShmSnapshot, standingCount);
    FIELD(MxboShmSnapshot, standings);
    FIELD(MxboShmSnapshot, map);
    FIELD(MxboShmSnapshot, standingsRect);
    FIELD(MxboShmSnapshot, relative);
    FIELD(MxboShmSnapshot, showMap);
    FIELD(MxboShmSnapshot, showStandings);
    FIELD(MxboShmSnapshot, showRelative);
    FIELD(MxboShmSnapshot, standingsRows);
    FIELD(MxboShmSnapshot, relativeCount);
    FIELD(MxboShmSnapshot, localGear);
    FIELD(MxboShmSnapshot, localRpm);
    FIELD(MxboShmSnapshot, engineTemp);
    FIELD(MxboShmSnapshot, airTemp);
    FIELD(MxboShmSnapshot, lastLapMs);
    FIELD(MxboShmSnapshot, currentLapMs);
    FIELD(MxboShmSnapshot, currentLap);
    FIELD(MxboShmSnapshot, sessionLaps);
    FIELD(MxboShmSnapshot, onTrack);
    FIELD(MxboShmSnapshot, maxRpm);
    FIELD(MxboShmSnapshot, shiftRpm);
    FIELD(MxboShmSnapshot, sessionTimeMs);
    FIELD(MxboShmSnapshot, sessionLength);
    FIELD(MxboShmSnapshot, bestLapMs);
    FIELD(MxboShmSnapshot, sectorCount);
    FIELD(MxboShmSnapshot, sectorLast);
    FIELD(MxboShmSnapshot, sectorCur);
    FIELD(MxboShmSnapshot, sectorLastLap);
    FIELD(MxboShmSnapshot, sectorBest);
    FIELD(MxboShmSnapshot, sectorDelta);
    FIELD(MxboShmSnapshot, sectorDeltaValid);
    FIELD(MxboShmSnapshot, sessionKind);
    FIELD(MxboShmSnapshot, sessionState);
    FIELD(MxboShmSnapshot, fuel);
    FIELD(MxboShmSnapshot, maxFuel);
    FIELD(MxboShmSnapshot, localRoll);
    FIELD(MxboShmSnapshot, localPitch);
    FIELD(MxboShmSnapshot, localSteer);
    FIELD(MxboShmSnapshot, steerLock);
    FIELD(MxboShmSnapshot, setupName);
    FIELD(MxboShmSnapshot, localThrottle);
    FIELD(MxboShmSnapshot, localFrontBrake);
    FIELD(MxboShmSnapshot, localRearBrake);
    FIELD(MxboShmSnapshot, localClutch);

    std::printf("MxboShmCmd.size %zu\n", sizeof(MxboShmCmd));
    FIELD(MxboShmCmd, magic);
    FIELD(MxboShmCmd, spectating);
    FIELD(MxboShmCmd, spectateRaceNum);
    return 0;
}
