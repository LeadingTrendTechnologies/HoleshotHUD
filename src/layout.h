#pragma once

#ifndef NOMINMAX
#define NOMINMAX
#endif
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif

#include "config.h"
#include "hud/draw_list.h"

#include <string>
#include <windows.h>

struct LayoutEditor
{
    void update(PluginConfig& config, bool& layoutDirty, const std::string& iniPath);
    void drawOverlay(DrawList& dl, const PluginConfig& config) const;

private:
    enum class Target
    {
        None,
        Standings,
        Relative,
        Map,
    };

    Target hitTest(const PluginConfig& config, float mx, float my) const;
    HudRect* rectFor(PluginConfig& config, Target t) const;
    const HudRect* rectFor(const PluginConfig& config, Target t) const;

    HWND m_hwnd = nullptr;
    bool m_mouseWasDown = false;
    bool m_dragging = false;
    bool m_layoutMode = false;
    Target m_target = Target::None;
    Target m_hover = Target::None;
    float m_grabX = 0.0f;
    float m_grabY = 0.0f;
    float m_mouseX = 0.0f;
    float m_mouseY = 0.0f;
    bool m_mouseValid = false;
    bool m_cursorUnlocked = false;
};
