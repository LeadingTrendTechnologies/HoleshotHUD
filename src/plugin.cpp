#include "vendor/piboso/mxb_api.h"
#include "config.h"
#include "state.h"
#include "hud/draw_list.h"
#include "hud/map_hud.h"
#include "hud/widgets.h"
#include "hud/font_atlas.h"
#include "layout.h"
#include "shm_writer.h"

#ifndef NOMINMAX
#define NOMINMAX
#endif
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <algorithm>
#include <cstring>
#include <fstream>
#include <string>

namespace
{
    constexpr int kTelemetryHz = 50;

    PluginState g_state;
    PluginConfig g_config;
    DrawList g_draw;
    MapHud g_map;
    LayoutEditor g_layout;
    ShmWriter g_shm;
    std::string g_savePath;
    std::string g_iniPath;
    bool g_layoutDirty = true;

    template <typename T>
    bool copySized(T& dest, const void* src, int size)
    {
        dest = T{};
        if (!src || size <= 0)
        {
            return false;
        }
        const size_t n = std::min(static_cast<size_t>(size), sizeof(T));
        std::memcpy(&dest, src, n);
        return true;
    }

    std::string joinPath(const char* base, const char* file)
    {
        std::string p = base ? base : "";
        if (!p.empty() && p.back() != '\\' && p.back() != '/')
        {
            p += '\\';
        }
        p += file;
        return p;
    }

    void ensureOverlayCompat()
    {
        wchar_t exe[MAX_PATH]{};
        if (!GetModuleFileNameW(nullptr, exe, MAX_PATH))
        {
            return;
        }
        HKEY key = nullptr;
        if (RegCreateKeyExW(
                HKEY_CURRENT_USER,
                L"Software\\Microsoft\\Windows NT\\CurrentVersion\\AppCompatFlags\\Layers",
                0,
                nullptr,
                0,
                KEY_READ | KEY_WRITE,
                nullptr,
                &key,
                nullptr) != ERROR_SUCCESS)
        {
            return;
        }
        wchar_t val[512]{};
        DWORD type = 0;
        DWORD bytes = sizeof(val);
        const wchar_t* flag = L"DISABLEDXMAXIMIZEDWINDOWEDMODE";
        std::wstring next = L"~ ";
        if (RegQueryValueExW(key, exe, nullptr, &type, reinterpret_cast<LPBYTE>(val), &bytes) == ERROR_SUCCESS &&
            type == REG_SZ)
        {
            next = val;
            if (next.find(flag) != std::wstring::npos)
            {
                RegCloseKey(key);
                return;
            }
            if (!next.empty() && next.back() != L' ')
            {
                next += L' ';
            }
            next += flag;
            if (next[0] != L'~')
            {
                next = L"~ " + next;
            }
        }
        else
        {
            next += flag;
        }
        RegSetValueExW(
            key,
            exe,
            0,
            REG_SZ,
            reinterpret_cast<const BYTE*>(next.c_str()),
            static_cast<DWORD>((next.size() + 1) * sizeof(wchar_t)));
        RegCloseKey(key);
    }

    FILETIME g_iniWriteTime{};
    char g_lastCallback[32]{};

    void breadcrumb(const char* name)
    {
        if (!name)
        {
            return;
        }
        std::strncpy(g_lastCallback, name, sizeof(g_lastCallback) - 1);
        g_lastCallback[sizeof(g_lastCallback) - 1] = '\0';
    }

    void writeLastCallback()
    {
        if (g_lastCallback[0] == '\0')
        {
            return;
        }
        wchar_t local[MAX_PATH]{};
        const DWORD n = GetEnvironmentVariableW(L"LOCALAPPDATA", local, MAX_PATH);
        if (n == 0 || n >= MAX_PATH)
        {
            return;
        }
        std::wstring dir = local;
        dir += L"\\Holeshot HUD";
        CreateDirectoryW(dir.c_str(), nullptr);
        const std::wstring path = dir + L"\\last-callback.txt";
        const HANDLE file = CreateFileW(
            path.c_str(),
            GENERIC_WRITE,
            FILE_SHARE_READ,
            nullptr,
            CREATE_ALWAYS,
            FILE_ATTRIBUTE_NORMAL,
            nullptr);
        if (file == INVALID_HANDLE_VALUE)
        {
            return;
        }
        DWORD written = 0;
        WriteFile(file, g_lastCallback, static_cast<DWORD>(std::strlen(g_lastCallback)), &written, nullptr);
        CloseHandle(file);
    }

    void reloadConfigIfChanged()
    {
        if (g_iniPath.empty())
        {
            return;
        }
        WIN32_FILE_ATTRIBUTE_DATA fad{};
        if (!GetFileAttributesExA(g_iniPath.c_str(), GetFileExInfoStandard, &fad))
        {
            return;
        }
        if (CompareFileTime(&fad.ftLastWriteTime, &g_iniWriteTime) <= 0)
        {
            return;
        }
        g_iniWriteTime = fad.ftLastWriteTime;
        g_config.load(g_iniPath);
        g_layoutDirty = true;
    }

    void stampIniWriteTime()
    {
        if (g_iniPath.empty())
        {
            return;
        }
        WIN32_FILE_ATTRIBUTE_DATA fad{};
        if (GetFileAttributesExA(g_iniPath.c_str(), GetFileExInfoStandard, &fad))
        {
            g_iniWriteTime = fad.ftLastWriteTime;
        }
    }

    void publishHud()
    {
        g_shm.publish(g_state, g_config);
    }

    template <typename Fn>
    void safeCall(Fn&& fn)
    {
        try
        {
            fn();
        }
        catch (...)
        {
            writeLastCallback();
        }
    }

    template <typename T, typename Fn>
    void onCopied(void* data, int size, Fn&& fn)
    {
        safeCall([&] {
            T dest{};
            if (copySized(dest, data, size))
            {
                fn(dest);
            }
        });
    }
}

extern "C" {

__declspec(dllexport) char* GetModID()
{
    breadcrumb("GetModID");
    static char modId[] = "mxbikes";
    return modId;
}

__declspec(dllexport) int GetModDataVersion()
{
    breadcrumb("GetModDataVersion");
    return 8;
}

__declspec(dllexport) int GetInterfaceVersion()
{
    breadcrumb("GetInterfaceVersion");
    return 9;
}

__declspec(dllexport) int Startup(char* _szSavePath)
{
    breadcrumb("Startup");
    try
    {
        g_savePath = _szSavePath ? _szSavePath : "";
        g_iniPath = joinPath(g_savePath.c_str(), "Holeshot-HUD.ini");
        std::string loadPath = g_iniPath;
        {
            std::ifstream probe(g_iniPath);
            if (!probe.good())
            {
                const std::string legacy = joinPath(g_savePath.c_str(), "mxbo.ini");
                std::ifstream old(legacy);
                if (old.good())
                {
                    loadPath = legacy;
                }
            }
        }
        g_config.load(loadPath);
        stampIniWriteTime();
        g_layoutDirty = true;
        g_shm.open();
        ensureOverlayCompat();
        return kTelemetryHz;
    }
    catch (...)
    {
        return -1;
    }
}

__declspec(dllexport) void Shutdown()
{
    breadcrumb("Shutdown");
    writeLastCallback();
    safeCall([] {
        if (!g_iniPath.empty())
        {
            const std::string legacy = joinPath(g_savePath.c_str(), "mxbo.ini");
            if (_stricmp(legacy.c_str(), g_iniPath.c_str()) != 0)
            {
                DeleteFileA(legacy.c_str());
            }
        }
        g_state.clearEvent();
        g_draw.clear();
        g_shm.close();
    });
}

__declspec(dllexport) void EventInit(void* _pData, int _iDataSize)
{
    breadcrumb("EventInit");
    safeCall([&] {
        SPluginsBikeEvent_t data{};
        copySized(data, _pData, _iDataSize);
        g_state.setEvent(data);
        g_layoutDirty = true;
    });
}

__declspec(dllexport) void EventDeinit()
{
    breadcrumb("EventDeinit");
    safeCall([] {
        g_state.clearEvent();
        g_layoutDirty = true;
        publishHud();
    });
}

__declspec(dllexport) void RunInit(void* _pData, int _iDataSize)
{
    breadcrumb("RunInit");
    safeCall([&] {
        SPluginsBikeSession_t data{};
        if (copySized(data, _pData, _iDataSize))
        {
            g_state.setBikeSession(data);
        }
        g_state.beginRun();
    });
}

__declspec(dllexport) void RunDeinit()
{
    breadcrumb("RunDeinit");
    safeCall([] {
        g_state.endRun();
        publishHud();
    });
}

__declspec(dllexport) void RunStart()
{
    breadcrumb("RunStart");
    safeCall([] { g_state.beginRun(); });
}

__declspec(dllexport) void RunStop()
{
    breadcrumb("RunStop");
}

__declspec(dllexport) void RunLap(void* _pData, int _iDataSize)
{
    breadcrumb("RunLap");
    onCopied<SPluginsBikeLap_t>(_pData, _iDataSize, [](const SPluginsBikeLap_t& data) {
        g_state.setLocalLap(data.m_iLapNum, data.m_iLapTime);
        const int last = g_state.lastLapMs();
        if (last > 0)
        {
            const int focus = g_state.focusRaceNum();
            if (focus < 0 || focus == g_state.localRaceNum())
            {
                g_state.finishLapSectors(data.m_iLapNum, last, 0, 0);
            }
        }
    });
}

__declspec(dllexport) void RunSplit(void* _pData, int _iDataSize)
{
    breadcrumb("RunSplit");
    onCopied<SPluginsBikeSplit_t>(_pData, _iDataSize, [](const SPluginsBikeSplit_t& data) {
        g_state.setLocalSplit(data.m_iSplit, data.m_iSplitTime, data.m_iBestDiff);
    });
}

__declspec(dllexport) void RunTelemetry(void* _pData, int _iDataSize, float _fTime, float _fPos)
{
    breadcrumb("RunTelemetry");
    onCopied<SPluginsBikeData_t>(_pData, _iDataSize, [&](const SPluginsBikeData_t& data) {
        g_state.setTelemetry(data, _fTime, _fPos);
    });
}

__declspec(dllexport) int DrawInit(int* _piNumSprites, char** _pszSpriteName, int* _piNumFonts, char** _pszFontName)
{
    breadcrumb("DrawInit");
    if (_piNumSprites)
    {
        *_piNumSprites = 0;
    }
    if (_pszSpriteName)
    {
        *_pszSpriteName = nullptr;
    }
    if (_piNumFonts)
    {
        *_piNumFonts = 0;
    }
    if (_pszFontName)
    {
        *_pszFontName = nullptr;
    }
    FontAtlas::get().ensure();
    return 0;
}

__declspec(dllexport) void Draw(int _iState, int* _piNumQuads, void** _ppQuad, int* _piNumString, void** _ppString)
{
    breadcrumb("Draw");
    if (_piNumQuads) *_piNumQuads = 0;
    if (_ppQuad) *_ppQuad = nullptr;
    if (_piNumString) *_piNumString = 0;
    if (_ppString) *_ppString = nullptr;

    try
    {
        (void)_iState;
        reloadConfigIfChanged();
        g_layout.update(g_config, g_layoutDirty, g_iniPath);
        g_shm.publish(g_state, g_config);

        // Frozen: standings, relative, and map only. Overlay widgets stay in Rust.
        if (!g_config.ingameHud)
        {
            return;
        }

        g_draw.clear();
        if (g_draw.quads.capacity() < kDrawQuadReserve)
        {
            g_draw.quads.reserve(kDrawQuadReserve);
        }

        if (g_layoutDirty || g_state.centerlineDirty())
        {
            g_map.rebuildTrack(g_state, g_config.map);
            g_state.clearCenterlineDirty();
            g_layoutDirty = false;
        }

        if (g_config.showMap)
        {
            g_map.draw(g_draw, g_state, g_config.map);
        }
        if (g_config.showStandings)
        {
            drawStandings(g_draw, g_state, g_config.standings, g_config.standingsRows);
        }
        if (g_config.showRelative)
        {
            drawRelative(g_draw, g_state, g_config.relative, g_config.relativeCount);
        }
        g_layout.drawOverlay(g_draw, g_config);

        if (_piNumQuads)
        {
            *_piNumQuads = static_cast<int>(g_draw.quads.size());
        }
        if (_ppQuad)
        {
            *_ppQuad = g_draw.quads.empty() ? nullptr : g_draw.quads.data();
        }
        if (_piNumString)
        {
            *_piNumString = static_cast<int>(g_draw.strings.size());
        }
        if (_ppString)
        {
            *_ppString = g_draw.strings.empty() ? nullptr : g_draw.strings.data();
        }
    }
    catch (...)
    {
    }
}

__declspec(dllexport) void TrackCenterline(int _iNumSegments, SPluginsTrackSegment_t* _pasSegment, void* _pRaceData)
{
    breadcrumb("TrackCenterline");
    safeCall([&] {
        if (_iNumSegments > 0 && !_pasSegment)
        {
            return;
        }
        g_state.setCenterline(_iNumSegments, _pasSegment, static_cast<const float*>(_pRaceData));
        g_layoutDirty = true;
    });
}

__declspec(dllexport) void RaceEvent(void* _pData, int _iDataSize)
{
    breadcrumb("RaceEvent");
    onCopied<SPluginsRaceEvent_t>(_pData, _iDataSize, [](const SPluginsRaceEvent_t& data) {
        g_state.setRaceEvent(data);
    });
}

__declspec(dllexport) void RaceDeinit()
{
    breadcrumb("RaceDeinit");
    safeCall([] {
        g_state.endRun();
        g_state.clearRace();
        g_layoutDirty = true;
        publishHud();
    });
}

__declspec(dllexport) void RaceAddEntry(void* _pData, int _iDataSize)
{
    breadcrumb("RaceAddEntry");
    onCopied<SPluginsRaceAddEntry_t>(_pData, _iDataSize, [](const SPluginsRaceAddEntry_t& data) {
        g_state.addEntry(data);
    });
}

__declspec(dllexport) void RaceRemoveEntry(void* _pData, int _iDataSize)
{
    breadcrumb("RaceRemoveEntry");
    onCopied<SPluginsRaceRemoveEntry_t>(_pData, _iDataSize, [](const SPluginsRaceRemoveEntry_t& data) {
        g_state.removeEntry(data.m_iRaceNum);
    });
}

__declspec(dllexport) void RaceSession(void* _pData, int _iDataSize)
{
    breadcrumb("RaceSession");
    onCopied<SPluginsRaceSession_t>(_pData, _iDataSize, [](const SPluginsRaceSession_t& data) {
        g_state.setSession(data);
    });
}

__declspec(dllexport) void RaceSessionState(void* _pData, int _iDataSize)
{
    breadcrumb("RaceSessionState");
    onCopied<SPluginsRaceSessionState_t>(_pData, _iDataSize, [](const SPluginsRaceSessionState_t& data) {
        g_state.setSessionState(data);
    });
}

__declspec(dllexport) void RaceLap(void* _pData, int _iDataSize)
{
    breadcrumb("RaceLap");
    onCopied<SPluginsRaceLap_t>(_pData, _iDataSize, [](const SPluginsRaceLap_t& data) {
        g_state.setRaceLap(data.m_iRaceNum, data.m_iLapNum, data.m_iLapTime, data.m_aiSplit[0], data.m_aiSplit[1]);
    });
}

__declspec(dllexport) void RaceSplit(void* _pData, int _iDataSize)
{
    breadcrumb("RaceSplit");
    onCopied<SPluginsRaceSplit_t>(_pData, _iDataSize, [](const SPluginsRaceSplit_t& data) {
        g_state.setRaceSplit(data.m_iRaceNum, data.m_iSplit, data.m_iSplitTime);
    });
}

__declspec(dllexport) void RaceHoleshot(void* _pData, int _iDataSize)
{
    breadcrumb("RaceHoleshot");
    onCopied<SPluginsRaceHoleshot_t>(_pData, _iDataSize, [](const SPluginsRaceHoleshot_t& data) {
        g_state.setRaceHoleshot(data.m_iRaceNum, data.m_iTime);
    });
}

__declspec(dllexport) void RaceCommunication(void* _pData, int _iDataSize)
{
    breadcrumb("RaceCommunication");
    (void)_pData;
    (void)_iDataSize;
}

__declspec(dllexport) void RaceClassification(void* _pData, int _iDataSize, void* _pArray, int _iElemSize)
{
    breadcrumb("RaceClassification");
    safeCall([&] {
        if (_iElemSize != static_cast<int>(sizeof(SPluginsRaceClassificationEntry_t)))
        {
            return;
        }
        SPluginsRaceClassification_t header{};
        if (!copySized(header, _pData, _iDataSize))
        {
            return;
        }
        auto* entries = static_cast<SPluginsRaceClassificationEntry_t*>(_pArray);
        const int n = std::clamp(header.m_iNumEntries, 0, kMaxRaceEntries);
        if (n > 0 && !entries)
        {
            return;
        }
        g_state.setClassification(header, entries, n);
    });
}

__declspec(dllexport) void RaceTrackPosition(int _iNumVehicles, void* _pArray, int _iElemSize)
{
    breadcrumb("RaceTrackPosition");
    safeCall([&] {
        if (_iElemSize != static_cast<int>(sizeof(SPluginsRaceTrackPosition_t)))
        {
            return;
        }
        auto* entries = static_cast<SPluginsRaceTrackPosition_t*>(_pArray);
        const int n = std::clamp(_iNumVehicles, 0, kMaxRaceEntries);
        if (n > 0 && !entries)
        {
            return;
        }
        g_state.setTrackPositions(entries, n);
    });
}

__declspec(dllexport) void RaceVehicleData(void* _pData, int _iDataSize)
{
    breadcrumb("RaceVehicleData");
    safeCall([&] {
        if (!_pData || _iDataSize <= 0)
        {
            return;
        }
        const int stride = static_cast<int>(sizeof(SPluginsRaceVehicleData_t));
        int n = 0;
        bool onePartial = false;
        if (_iDataSize >= stride && _iDataSize % stride == 0)
        {
            n = _iDataSize / stride;
        }
        else
        {
            const int field = std::clamp(
                static_cast<int>(g_state.trackPositions().size()),
                0,
                kMaxRaceEntries);
            if (field > 1 && _iDataSize == field)
            {
                n = field;
            }
            else
            {
                onePartial = true;
            }
        }
        if (onePartial)
        {
            SPluginsRaceVehicleData_t one{};
            if (copySized(one, _pData, _iDataSize))
            {
                g_state.setVehicleData(one);
            }
        }
        else
        {
            n = std::clamp(n, 0, kMaxRaceEntries);
            auto* entries = static_cast<SPluginsRaceVehicleData_t*>(_pData);
            for (int i = 0; i < n; ++i)
            {
                g_state.setVehicleData(entries[i]);
            }
        }
        publishHud();
    });
}

__declspec(dllexport) int SpectateVehicles(int _iNumVehicles, void* _pVehicleData, int _iCurSelection, int* _piSelect)
{
    breadcrumb("SpectateVehicles");
    int pick = -1;
    safeCall([&] {
        g_shm.noteSpectating();
        auto* vehicles = static_cast<SPluginsSpectateVehicle_t*>(_pVehicleData);
        const int n = std::clamp(_iNumVehicles, 0, kMaxRaceEntries);
        if (n > 0 && vehicles && _iCurSelection >= 0 && _iCurSelection < n)
        {
            g_state.setSpectateSelection(vehicles[_iCurSelection].m_iRaceNum);
        }
        const int want = g_shm.takeSpectateRequest();
        if (want > 0 && n > 0 && vehicles)
        {
            for (int i = 0; i < n; ++i)
            {
                if (vehicles[i].m_iRaceNum == want)
                {
                    pick = i;
                    g_state.setSpectateSelection(want);
                    break;
                }
            }
        }
    });
    if (pick >= 0 && _piSelect)
    {
        *_piSelect = pick;
        return 1;
    }
    return 0;
}

__declspec(dllexport) int SpectateCameras(int _iNumCameras, void* _pCameraData, int _iCurSelection, int* _piSelect)
{
    breadcrumb("SpectateCameras");
    (void)_iNumCameras;
    (void)_pCameraData;
    (void)_iCurSelection;
    (void)_piSelect;
    return 0;
}

}
