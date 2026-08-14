#pragma once

class PluginState;
struct PluginConfig;

class ShmWriter
{
public:
    bool open();
    void close();
    void publish(const PluginState& state, const PluginConfig& config);

private:
    void* m_map = nullptr;
    void* m_view = nullptr;
};
