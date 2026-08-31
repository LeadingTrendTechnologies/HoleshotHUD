#pragma once

#include "shm/mxbo_shm.h"

#include <cstdint>
#include <vector>

class PluginState;
struct PluginConfig;

class ShmWriter
{
public:
    bool open();
    void close();
    void publish(const PluginState& state, const PluginConfig& config);
    void noteSpectating();
    int takeSpectateRequest();

private:
    void decaySpectating();
    void fillPolyCache(const PluginState& state);

    void* m_map = nullptr;
    void* m_view = nullptr;
    void* m_cmdMap = nullptr;
    void* m_cmdView = nullptr;
    long long m_lastSpectateQpc = 0;
    std::vector<MxboShmPoint> m_poly;
    int32_t m_polyCount = 0;
    uint32_t m_polyRev = 0xFFFFFFFFu;
    size_t m_polyTrail = static_cast<size_t>(-1);
};
