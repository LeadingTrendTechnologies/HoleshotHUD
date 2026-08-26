#pragma once

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

    void* m_map = nullptr;
    void* m_view = nullptr;
    void* m_cmdMap = nullptr;
    void* m_cmdView = nullptr;
    long long m_lastSpectateQpc = 0;
};
