#include "config.h"
#include "shm_publish.h"
#include "vendor/piboso/mxb_api.h"

#include <cmath>
#include <cstdio>
#include <cstring>
#include <fstream>
#include <string>
#include <vector>

static int g_fails = 0;

#define CHECK(cond)                                                                              \
    do                                                                                           \
    {                                                                                            \
        if (!(cond))                                                                             \
        {                                                                                        \
            std::fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #cond);                 \
            ++g_fails;                                                                           \
        }                                                                                        \
    } while (0)

static void copyField(char* dest, size_t n, const char* src)
{
    std::strncpy(dest, src, n - 1);
    dest[n - 1] = '\0';
}

static MxboShmSnapshot publish(const PluginState& state, const std::vector<MxboShmPoint>& poly)
{
    PluginConfig cfg{};
    MxboShmSnapshot local{};
    fillSnapshot(local, state, cfg, poly.empty() ? nullptr : poly.data(),
                 static_cast<int>(poly.size()), 12345);
    return local;
}

static void testStraightCenterline()
{
    PluginState state;
    SPluginsTrackSegment_t seg{};
    seg.m_iType = 0;
    seg.m_fLength = 10.0f;
    seg.m_fRadius = 0.0f;
    seg.m_fAngle = 0.0f;
    state.setCenterline(1, &seg, nullptr);

    std::vector<MxboShmPoint> pts;
    tessellateTrack(state, pts);
    CHECK(pts.size() == 2);
    CHECK(std::fabs(pts[0].x) < 1e-4f && std::fabs(pts[0].z) < 1e-4f);
    CHECK(std::fabs(pts[1].x) < 1e-4f && std::fabs(pts[1].z - 10.0f) < 1e-3f);

    const MxboShmSnapshot snap = publish(state, pts);
    CHECK(snap.polyCount == 2);
    CHECK(std::fabs(snap.poly[1].z - 10.0f) < 1e-3f);
}

static void testThinToMaxPoly()
{
    PluginState state;
    std::vector<SPluginsTrackSegment_t> segs(2000);
    for (auto& seg : segs)
    {
        seg.m_iType = 0;
        seg.m_fLength = 1.0f;
        seg.m_fAngle = 0.0f;
    }
    state.setCenterline(static_cast<int>(segs.size()), segs.data(), nullptr);

    std::vector<MxboShmPoint> pts;
    tessellateTrack(state, pts);
    CHECK(pts.size() == static_cast<size_t>(MXBO_MAX_POLY));
}

static void testStandingsCopy()
{
    PluginState state;
    SPluginsRaceAddEntry_t e{};
    e.m_iRaceNum = 7;
    copyField(e.m_szName, sizeof(e.m_szName), "Jett Lawrence");
    copyField(e.m_szBikeShortName, sizeof(e.m_szBikeShortName), "CRF450R");
    copyField(e.m_szCategory, sizeof(e.m_szCategory), "MX1");
    state.addEntry(e);

    SPluginsRaceClassificationEntry_t row{};
    row.m_iRaceNum = 7;
    row.m_iBestLap = 95000;
    row.m_iNumLaps = 3;
    SPluginsRaceClassification_t hdr{};
    hdr.m_iNumEntries = 1;
    state.setClassification(hdr, &row, 1);

    SPluginsRaceTrackPosition_t tp{};
    tp.m_iRaceNum = 7;
    tp.m_fPosX = 12.0f;
    tp.m_fPosZ = -4.0f;
    tp.m_iCrashed = 1;
    state.setTrackPositions(&tp, 1);

    const MxboShmSnapshot snap = publish(state, {});
    CHECK(snap.standingCount == 1);
    CHECK(snap.standings[0].raceNum == 7);
    CHECK(snap.standings[0].position == 1);
    CHECK(snap.standings[0].bestLapMs == 95000);
    CHECK(snap.standings[0].crashed == 1);
    CHECK(std::strcmp(snap.standings[0].name, "Jett Lawrence") == 0);
    CHECK(std::strcmp(snap.standings[0].bike, "CRF450R") == 0);
    CHECK(std::strcmp(snap.standings[0].category, "MX1") == 0);
    CHECK(snap.riderCount == 0);
}

static void testGarageHidesRidersUntilTelemetry()
{
    PluginState state;
    SPluginsRaceTrackPosition_t tp{};
    tp.m_iRaceNum = 3;
    state.setTrackPositions(&tp, 1);
    CHECK(publish(state, {}).riderCount == 0);

    state.beginRun();
    SPluginsBikeData_t bike{};
    bike.m_fPosX = 1.0f;
    bike.m_fPosZ = 2.0f;
    state.setTelemetry(bike, 1.0f, 0.1f);
    CHECK(publish(state, {}).riderCount == 1);
}

static void testUnsetSessionLengthPublishesZero()
{
    PluginState state;
    CHECK(state.sessionLength() == kSessionLengthUnset);
    CHECK(publish(state, {}).sessionLength == 0);
}

static void testSeqlockStore()
{
    PluginState state;
    SPluginsRaceAddEntry_t e{};
    e.m_iRaceNum = 1;
    copyField(e.m_szName, sizeof(e.m_szName), "You");
    state.addEntry(e);
    SPluginsRaceClassificationEntry_t row{};
    row.m_iRaceNum = 1;
    SPluginsRaceClassification_t hdr{};
    state.setClassification(hdr, &row, 1);

    MxboShmSnapshot local = publish(state, {});
    MxboShmSnapshot dst{};
    dst.seq = 6;
    seqlockStore(dst, local);
    CHECK((dst.seq & 1u) == 0);
    CHECK(dst.seq == 8);
    CHECK(dst.magic == MXBO_SHM_MAGIC);
    CHECK(dst.version == MXBO_SHM_VERSION);
    CHECK(dst.standingCount == 1);
    CHECK(std::strcmp(dst.standings[0].name, "You") == 0);
}

static void testMissingIniDoesNotCreateFile()
{
    const char* path = "shm-publish-test-missing.ini";
    std::remove(path);
    PluginConfig cfg;
    cfg.load(path);
    std::ifstream in(path);
    CHECK(!in.good());
    std::remove(path);
}

int main()
{
    testStraightCenterline();
    testThinToMaxPoly();
    testStandingsCopy();
    testGarageHidesRidersUntilTelemetry();
    testUnsetSessionLengthPublishesZero();
    testSeqlockStore();
    testMissingIniDoesNotCreateFile();
    if (g_fails)
    {
        std::fprintf(stderr, "%d check(s) failed\n", g_fails);
        return 1;
    }
    std::printf("shm-publish-test: ok\n");
    return 0;
}
