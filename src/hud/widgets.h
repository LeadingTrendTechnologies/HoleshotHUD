#pragma once

#include "draw_list.h"
#include "../config.h"
#include "../state.h"

// Frozen in-game HUD. Standings + relative only. Do not add overlay widgets.

void drawStandings(DrawList& dl, const PluginState& state, const HudRect& rect, int maxRows);
void drawRelative(DrawList& dl, const PluginState& state, const HudRect& rect, int countEachSide);
