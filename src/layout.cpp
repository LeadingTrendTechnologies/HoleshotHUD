#include "layout.h"

#include <algorithm>
#include <cmath>
#include <windows.h>

namespace
{
    bool keyDown(int vk)
    {
        return (GetAsyncKeyState(vk) & 0x8000) != 0;
    }

    HWND findGameWindow()
    {
        HWND fg = GetForegroundWindow();
        if (!fg)
        {
            return nullptr;
        }
        DWORD pid = 0;
        GetWindowThreadProcessId(fg, &pid);
        if (pid != GetCurrentProcessId())
        {
            return nullptr;
        }
        RECT rc{};
        if (!GetClientRect(fg, &rc))
        {
            return nullptr;
        }
        if ((rc.right - rc.left) < 640 || (rc.bottom - rc.top) < 480)
        {
            return nullptr;
        }
        return fg;
    }

    bool contains(const HudRect& r, float x, float y)
    {
        return x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h;
    }

    void clampRect(HudRect& r)
    {
        r.w = std::clamp(r.w, 0.08f, 1.2f);
        r.h = std::clamp(r.h, 0.08f, 1.2f);
        r.x = std::clamp(r.x, 0.02f - r.w, 0.98f);
        r.y = std::clamp(r.y, 0.02f - r.h, 0.98f);
    }

    void unlockCursorToMonitor(HWND hwnd)
    {
        HMONITOR mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        MONITORINFO mi{};
        mi.cbSize = sizeof(mi);
        if (GetMonitorInfoW(mon, &mi))
        {
            ClipCursor(&mi.rcMonitor);
        }
    }
}

LayoutEditor::Target LayoutEditor::hitTest(const PluginConfig& config, float mx, float my) const
{
    // Last drawn is on top: map, then relative, then standings.
    if (config.showMap && contains(config.map, mx, my))
    {
        return Target::Map;
    }
    if (config.showRelative && contains(config.relative, mx, my))
    {
        return Target::Relative;
    }
    if (config.showStandings && contains(config.standings, mx, my))
    {
        return Target::Standings;
    }
    return Target::None;
}

HudRect* LayoutEditor::rectFor(PluginConfig& config, Target t) const
{
    switch (t)
    {
    case Target::Standings: return &config.standings;
    case Target::Relative: return &config.relative;
    case Target::Map: return &config.map;
    default: return nullptr;
    }
}

const HudRect* LayoutEditor::rectFor(const PluginConfig& config, Target t) const
{
    switch (t)
    {
    case Target::Standings: return &config.standings;
    case Target::Relative: return &config.relative;
    case Target::Map: return &config.map;
    default: return nullptr;
    }
}

void LayoutEditor::update(PluginConfig& config, bool& layoutDirty, const std::string& iniPath)
{
    m_layoutMode = keyDown(VK_CONTROL);
    m_mouseValid = false;
    m_hover = Target::None;

    HWND hwnd = findGameWindow();
    if (hwnd)
    {
        m_hwnd = hwnd;
    }
    if (!m_hwnd || !IsWindow(m_hwnd))
    {
        m_dragging = false;
        m_mouseWasDown = false;
        return;
    }

    POINT screen{};
    if (!GetCursorPos(&screen))
    {
        return;
    }
    POINT client = screen;
    if (!ScreenToClient(m_hwnd, &client))
    {
        return;
    }
    RECT rc{};
    if (!GetClientRect(m_hwnd, &rc))
    {
        return;
    }
    const int w = rc.right - rc.left;
    const int h = rc.bottom - rc.top;
    if (w <= 0 || h <= 0)
    {
        return;
    }

    m_mouseX = static_cast<float>(client.x) / static_cast<float>(w);
    m_mouseY = static_cast<float>(client.y) / static_cast<float>(h);
    m_mouseValid = true;

    if (m_layoutMode || m_dragging)
    {
        ReleaseCapture();
        ClipCursor(nullptr);
        unlockCursorToMonitor(m_hwnd);
        m_cursorUnlocked = true;
    }
    else if (m_cursorUnlocked)
    {
        ClipCursor(nullptr);
        m_cursorUnlocked = false;
    }

    const bool mouseDown = keyDown(VK_LBUTTON);
    const bool pressed = mouseDown && !m_mouseWasDown;
    const bool released = !mouseDown && m_mouseWasDown;
    m_mouseWasDown = mouseDown;

    if (!m_layoutMode && !m_dragging)
    {
        return;
    }

    m_hover = hitTest(config, m_mouseX, m_mouseY);

    if (pressed && m_layoutMode && m_hover != Target::None)
    {
        HudRect* r = rectFor(config, m_hover);
        if (r)
        {
            m_dragging = true;
            m_target = m_hover;
            m_grabX = m_mouseX - r->x;
            m_grabY = m_mouseY - r->y;
        }
    }

    if (m_dragging)
    {
        HudRect* r = rectFor(config, m_target);
        if (r)
        {
            r->x = m_mouseX - m_grabX;
            r->y = m_mouseY - m_grabY;
            clampRect(*r);
            if (m_target == Target::Map)
            {
                layoutDirty = true;
            }
        }
        if (released || !mouseDown)
        {
            m_dragging = false;
            m_target = Target::None;
            config.save(iniPath);
        }
    }
}

void LayoutEditor::drawOverlay(DrawList& dl, const PluginConfig& config) const
{
    if (!m_layoutMode && !m_dragging)
    {
        return;
    }

    dl.addQuad(0.28f, 0.012f, 0.72f, 0.042f, Palette::kHeaderBg);
    dl.addString("CTRL + DRAG TO MOVE WIDGETS", 0.50f, 0.018f, 0.016f, 1, Palette::kAccent);

    const Target highlight = m_dragging ? m_target : m_hover;
    const HudRect* r = rectFor(config, highlight);
    if (r)
    {
        dl.addBorder(r->x, r->y, r->x + r->w, r->y + r->h, 0.003f, Palette::kAccent);
        dl.addQuad(r->x, r->y, r->x + r->w, r->y + kHeaderH, Palette::kAccent);
    }

    if (m_mouseValid)
    {
        const float s = 0.012f;
        const float pts[4][2] = {
            {m_mouseX, m_mouseY},
            {m_mouseX, m_mouseY + s * 1.6f},
            {m_mouseX + s * 0.45f, m_mouseY + s * 1.15f},
            {m_mouseX + s * 0.9f, m_mouseY + s * 1.15f},
        };
        dl.addQuadPts(pts, Palette::kAccent);
        dl.addQuad(m_mouseX, m_mouseY, m_mouseX + 0.003f, m_mouseY + 0.018f, Palette::kText);
    }
}
