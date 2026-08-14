#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define MXBO_SHM_NAME L"Local\\MXBOHudV1"
#define MXBO_SHM_NAME_A "Local\\MXBOHudV1"
#define MXBO_SHM_MAGIC 0x4F42584Du /* 'MXBO' */
#define MXBO_SHM_VERSION 2
#define MXBO_MAX_POLY 1024
#define MXBO_MAX_RIDERS 64
#define MXBO_MAX_STANDINGS 40
#define MXBO_NAME 32
#define MXBO_TRACK_NAME 64

typedef struct MxboShmPoint
{
    float x;
    float z;
} MxboShmPoint;

typedef struct MxboShmRider
{
    int32_t raceNum;
    float x;
    float z;
    float yaw;
    float trackPos;
    int32_t crashed;
    char name[MXBO_NAME];
} MxboShmRider;

typedef struct MxboShmStanding
{
    int32_t raceNum;
    int32_t position;
    int32_t state;
    int32_t bestLapMs;
    int32_t numLaps;
    int32_t gapMs;
    int32_t gapLaps;
    int32_t pit;
    int32_t penaltyMs;
    int32_t crashed;
    char name[MXBO_NAME];
    char bike[MXBO_NAME];
} MxboShmStanding;

typedef struct MxboShmRect
{
    float x;
    float y;
    float w;
    float h;
} MxboShmRect;

typedef struct MxboShmSnapshot
{
    uint32_t magic;
    uint32_t version;
    uint32_t seq;
    uint32_t size;
    uint64_t tickQpc;

    int32_t localRaceNum;
    int32_t focusRaceNum;
    int32_t hasTelemetry;
    int32_t localCrashed;
    float localX;
    float localZ;
    float localVelX;
    float localVelZ;
    float localYaw;
    float localSpeed;
    float localTrackPos;

    char trackName[MXBO_TRACK_NAME];
    float trackLength;
    float sfMeters;

    int32_t polyCount;
    MxboShmPoint poly[MXBO_MAX_POLY];

    int32_t riderCount;
    MxboShmRider riders[MXBO_MAX_RIDERS];

    int32_t standingCount;
    MxboShmStanding standings[MXBO_MAX_STANDINGS];

    MxboShmRect map;
    MxboShmRect standingsRect;
    MxboShmRect relative;
    int32_t showMap;
    int32_t showStandings;
    int32_t showRelative;
    int32_t standingsRows;
    int32_t relativeCount;
} MxboShmSnapshot;

#ifdef __cplusplus
}
#endif
