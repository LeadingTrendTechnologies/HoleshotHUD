# Overlay widgets

Agent context for every HUD widget. Plugin field inventory lives in [Home.md](Home.md). These pages track **what each widget does, why it looks the way it does, and what changed**.

When you change a widget, append a dated entry to that widget’s **Change log**. Do not only update `CHANGELOG.md`.

| Widget | Settings tab | `WidgetId` | Wiki |
| --- | --- | --- | --- |
| Standings | Standings | `Standings` | [standings.md](widgets/standings.md) |
| Relative | Relative | `Relative` | [relative.md](widgets/relative.md) |
| Map | Map | `Map` | [map.md](widgets/map.md) |
| Minimap | Minimap | `Minimap` | [minimap.md](widgets/minimap.md) |
| Radar | Radar | `Radar` | [radar.md](widgets/radar.md) |
| Dash | Dash | `Dash` | [dash.md](widgets/dash.md) |
| Flags | Flags | `Flag` | [flag.md](widgets/flag.md) |
| Horizontal Standings | H-Standings | `Ticker` | [horizontal-standings.md](widgets/horizontal-standings.md) |
| Systems | Systems | `Sys` | [systems.md](widgets/systems.md) |
| Stance | Stance | `Stance` | [stance.md](widgets/stance.md) |
| Sectors (labs) | Sectors | `Sector` | [sector.md](widgets/sector.md) |
| Delta Bar (labs) | Delta Bar | `Delta` | [delta-bar.md](widgets/delta-bar.md) |

## Shared rules

- Draw path: `overlay/hud/src/render.rs` (`draw()`). Settings: `overlay/src/settings.rs`. Layout/ini: `overlay/hud/src/config.rs`.
- Shared race view: `overlay/hud/src/race_store.rs` — `RaceStore::refresh` once per overlay frame, then `RaceStore::with` (re-entrant, no clone). `tick()` still returns a clone for tests and clock logs.
- Settings widget panes share `WidgetPaneSpec` / `open_widget_pane` (heading, Show on overlay, style rows, extras, snap). Standings/relative use `table_style_controls` so **Row highlight** / **Text color** / **Alternating rows** stay on those boards. App / Feedback / What’s new stay custom. High-contrast settings palette is still OS-aware.
- **Live race order**: the game only republishes its classification when someone crosses the line, so `race_store` re-derives places every tick — the game order with any pass we can see in `riders[].track_pos` applied on top (`live_order` / `passed`). `RaceField.rows` come back in that order with `standing.position` set to it, and `live_position` / `live_leader` serve map-style lookups. See [live race order](live-order.md).
- All widgets start **hidden** on a fresh install (`show_* = false`). Turn on with **Show on overlay**.
- **Labs**: Sectors and Delta Bar stay off the overlay widget rail until **Experimental widgets** is on (Settings → Labs). `experimental=1` in the ini; `feature_sector=1` still unlocks (legacy). Stance and Flags are regular Cockpit widgets. The website demo lists labs under an **Experimental** rail group (WASM turns the labs flag on only while those widgets are selected).
- Every widget has font size, bold, opacity, and snap-to-monitor. Hold **Ctrl** and drag to move or resize.
- Overlay font families: default **Exo 2**. Also Segoe / Arial / Tahoma / Roboto, **Teko**, **Goldman**, **Montserrat**. Old ini keys `agency` / `industry` / `faster` map to Exo 2 / Teko / Goldman. `bebas` and `impact` map to Goldman and Montserrat.
- Minimap / radar / dash / ticker / sys / stance / flag read `cfg.show_*` directly. Sector and Delta Bar also require `experimental_unlocked()`.
- All widgets draw when session data is present (`on_track`, telemetry, standings, or riders) — including replay / spectate, which never set `RunInit`. They hide when the snapshot is empty (menus, lobby, garage) and when MX Bikes is not running. A hitch that pauses plugin publish still keeps the last session HUD for 15s (`overlay/src/main.rs`); do not blank at the 2.5s live cutoff. Hold Ctrl for layout boxes; that still forces a draw. The Holeshot HUD icon stays in the top-right whenever the overlay window is on the game (proof it is compositing); a click opens settings even while Ctrl is down. The settings key toggles settings closed if it is already open. A top plaque explains a blank HUD: no widget on (F8 → Show on overlay) or no live plugin data (fully quit MX Bikes and start it again).
- Optional in-game HUD (`ingame_hud`) still draws a simpler standings + relative in C++ (`src/hud/widgets.cpp`). Overlay work does not go there.
- **Show overlay users** (Settings → App, off by default) adds an orange mark after the name on Standings / Relative and a thin orange ring on map / minimap dots for other riders in this session who also have the overlay. You are not marked. Presence is not a widget.
- **Highlight Steam friends** (Settings → App, off by default) adds a friend icon over map / minimap dots and a Friend column on Standings / Relative (same icon before the name on Horizontal Standings) when their Steam64 is in the presence room and in your Steam friend list. Both of you must run Holeshot and have Show overlay users on. In-game names are not used. Friends without the overlay stay unmarked.

## Shared rider colors (map, minimap, relative rows)

Default other-rider color is dark slate. **Blue** only if they are a lap ahead **and** closing from behind. **Red** only if you are a lap ahead **and** closing on them. Two (or more) laps down is still blue when they close from behind — `other_laps_ahead` prefers `gap_laps` over `num_laps` so a lapped rider whose completed-lap count sits on the race lap does not invert the leader to red. You are always orange. Off in warmup (`is_warmup`) — practice lap counts are not race lapping. See `lap_rel` / `rider_dot_col` in `render.rs`. Friend highlight is an icon, not a dot color.

## How to log a change

Add a bullet under **Change log** on the widget page:

```
- YYYY-MM-DD — short why. What you changed and the pitfall you are protecting.
```
