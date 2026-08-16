#include "vendor/piboso/mxb_api.h"
#include "color.h"
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

    void publishHud()
    {
        g_shm.publish(g_state, g_config);
    }
}

extern "C" {

__declspec(dllexport) char* GetModID()
{
    static char modId[] = "mxbikes";
    return modId;
}

__declspec(dllexport) int GetModDataVersion()
{
    return 8;
}

__declspec(dllexport) int GetInterfaceVersion()
{
    return 9;
}

__declspec(dllexport) int Startup(char* _szSavePath)
{
    try
    {
        g_savePath = _szSavePath ? _szSavePath : "";
        g_iniPath = joinPath(g_savePath.c_str(), "mxbo.ini");
        g_config.load(g_iniPath);
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
    try
    {
        if (!g_iniPath.empty())
        {
            g_config.save(g_iniPath);
        }
        g_state.clearEvent();
        g_draw.clear();
        g_shm.close();
    }
    catch (...)
    {
    }
}

__declspec(dllexport) void EventInit(void* _pData, int _iDataSize)
{
    try
    {
        SPluginsBikeEvent_t data{};
        copySized(data, _pData, _iDataSize);
        g_state.setEvent(data);
        g_layoutDirty = true;
    }
    catch (...)
    {
    }
}

__declspec(dllexport) void EventDeinit()
{
    try
    {
        g_state.clearEvent();
        g_layoutDirty = true;
        publishHud();
    }
    catch (...)
    {
    }
}

__declspec(dllexport) void RunInit(void* _pData, int _iDataSize)
{
    (void)_pData;
    (void)_iDataSize;
    try
    {
        g_state.beginRun();
    }
    catch (...)
    {
    }
}

__declspec(dllexport) void RunDeinit()
{
    try
    {
        g_state.endRun();
        publishHud();
    }
    catch (...)
    {
    }
}

__declspec(dllexport) void RunStart()
{
    try
    {
        g_state.beginRun();
    }
    catch (...)
    {
    }
}

__declspec(dllexport) void RunStop() {}

__declspec(dllexport) void RunLap(void* _pData, int _iDataSize)
{
    try
    {
        SPluginsBikeLap_t data{};
        if (copySized(data, _pData, _iDataSize))
        {
            g_state.setLocalLap(data.m_iLapNum, data.m_iLapTime);
        }
    }
    catch (...)
    {
    }
}

__declspec(dllexport) void RunSplit(void* _pData, int _iDataSize)
{
    (void)_pData;
    (void)_iDataSize;
}

__declspec(dllexport) void RunTelemetry(void* _pData, int _iDataSize, float _fTime, float _fPos)
{
    try
    {
        SPluginsBikeData_t data{};
        if (copySized(data, _pData, _iDataSize))
        {
            g_state.setTelemetry(data, _fTime, _fPos);
        }
    }
    catch (...)
    {
    }
}

__declspec(dllexport) int DrawInit(int* _piNumSprites, char** _pszSpriteName, int* _piNumFonts, char** _pszFontName)
{
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

        if (!g_config.ingameHud)
        {
            return;
        }

        g_draw.clear();
        if (g_draw.quads.capacity() < 2048)
        {
            g_draw.quads.reserve(2048);
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
    try
    {
        if (_iNumSegments > 0 && !_pasSegment)
        {
            return;
        }
        g_state.setCenterline(_iNumSegments, _pasSegment, static_cast<const float*>(_pRaceData));
        g_layoutDirty = true;
    }
    catch (...)
    {
    }
}

__declspec(dllexport) void RaceEvent(void* _pData, int _iDataSize)
{
    try
    {
        SPluginsRaceEvent_t data{};
        if (copySized(data, _pData, _iDataSize))
        {
            g_state.setRaceEvent(data);
        }
    }
    catch (...)
    {
    }
}

__declspec(dllexport) void RaceDeinit()
{
    try
    {
        g_state.endRun();
        g_state.clearRace();
        g_layoutDirty = true;
        publishHud();
    }
    catch (...)
    {
    }
}

__declspec(dllexport) void RaceAddEntry(void* _pData, int _iDataSize)
{
    try
    {
        SPluginsRaceAddEntry_t data{};
        if (copySized(data, _pData, _iDataSize))
        {
            g_state.addEntry(data);
        }
    }
    catch (...)
    {
    }
}

__declspec(dllexport) void RaceRemoveEntry(void* _pData, int _iDataSize)
{
    try
    {
        SPluginsRaceRemoveEntry_t data{};
        if (copySized(data, _pData, _iDataSize))
        {
            g_state.removeEntry(data.m_iRaceNum);
        }
    }
    catch (...)
    {
    }
}

__declspec(dllexport) void RaceSession(void* _pData, int _iDataSize)
{
    try
    {
        SPluginsRaceSession_t data{};
        if (copySized(data, _pData, _iDataSize))
        {
            g_state.setSession(data);
        }
    }
    catch (...)
    {
    }
}

__declspec(dllexport) void RaceSessionState(void* _pData, int _iDataSize)
{
    try
    {
        SPluginsRaceSessionState_t data{};
        if (copySized(data, _pData, _iDataSize))
        {
            g_state.setSessionState(data);
        }
    }
    catch (...)
    {
    }
}

__declspec(dllexport) void RaceLap(void* _pData, int _iDataSize)
{
    try
    {
        SPluginsRaceLap_t data{};
        if (copySized(data, _pData, _iDataSize))
        {
            g_state.setRaceLap(data.m_iRaceNum, data.m_iLapNum, data.m_iLapTime);
        }
    }
    catch (...)
    {
    }
}

__declspec(dllexport) void RaceSplit(void* _pData, int _iDataSize)
{
    (void)_pData;
    (void)_iDataSize;
}

__declspec(dllexport) void RaceHoleshot(void* _pData, int _iDataSize)
{
    (void)_pData;
    (void)_iDataSize;
}

__declspec(dllexport) void RaceCommunication(void* _pData, int _iDataSize)
{
    (void)_pData;
    (void)_iDataSize;
}

__declspec(dllexport) void RaceClassification(void* _pData, int _iDataSize, void* _pArray, int _iElemSize)
{
    try
    {
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
    }
    catch (...)
    {
    }
}

__declspec(dllexport) void RaceTrackPosition(int _iNumVehicles, void* _pArray, int _iElemSize)
{
    try
    {
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
    }
    catch (...)
    {
    }
}

__declspec(dllexport) void RaceVehicleData(void* _pData, int _iDataSize)
{
    try
    {
        SPluginsRaceVehicleData_t data{};
        if (copySized(data, _pData, _iDataSize))
        {
            g_state.setVehicleData(data);
        }
    }
    catch (...)
    {
    }
}

__declspec(dllexport) int SpectateVehicles(int _iNumVehicles, void* _pVehicleData, int _iCurSelection, int* _piSelect)
{
    try
    {
        auto* vehicles = static_cast<SPluginsSpectateVehicle_t*>(_pVehicleData);
        const int n = std::clamp(_iNumVehicles, 0, kMaxRaceEntries);
        if (n > 0 && vehicles && _iCurSelection >= 0 && _iCurSelection < n)
        {
            g_state.setSpectateSelection(vehicles[_iCurSelection].m_iRaceNum);
        }
        (void)_piSelect;
        return 0;
    }
    catch (...)
    {
        return 0;
    }
}

__declspec(dllexport) int SpectateCameras(int _iNumCameras, void* _pCameraData, int _iCurSelection, int* _piSelect)
{
    (void)_iNumCameras;
    (void)_pCameraData;
    (void)_iCurSelection;
    (void)_piSelect;
    return 0;
}

}
