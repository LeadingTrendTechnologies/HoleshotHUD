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
| Horizontal Standings | H-Standings | `Ticker` | [horizontal-standings.md](widgets/horizontal-standings.md) |
| Systems | Systems | `Sys` | [systems.md](widgets/systems.md) |

## Shared rules

- Draw path: `overlay/hud/src/render.rs` (`draw()`). Settings: `overlay/src/settings.rs`. Layout/ini: `overlay/hud/src/config.rs`.
- All eight start **hidden** on a fresh install (`show_* = false`). Turn on with **Show on overlay**.
- Every widget has font size, bold, opacity, and snap-to-monitor. Hold **Ctrl** and drag to move or resize.
- Overlay font families: default **Exo 2**. Also Segoe / Arial / Tahoma / Roboto, **Teko**, **Goldman**, **Montserrat**. Old ini keys `agency` / `industry` / `faster` map to Exo 2 / Teko / Goldman. `bebas` and `impact` map to Goldman and Montserrat.
- Standings / relative / map visibility is copied onto the snapshot (`s.show_*`). Minimap / radar / dash / ticker / sys read `cfg.show_*` directly.
- Race widgets (everything except Systems) do not draw when `on_track == 0` unless settings layout boxes are up.
- Optional in-game HUD (`ingame_hud`) still draws a simpler standings + relative in C++ (`src/hud/widgets.cpp`). Overlay work does not go there.

## Shared rider colors (map, minimap, relative rows)

Default other-rider color is dark slate. **Blue** only if they are a lap ahead **and** closing from behind. **Red** only if you are a lap ahead **and** closing on them. You are always orange. Off in warmup (`is_warmup`) — practice lap counts are not race lapping. See `lap_rel` / `rider_dot_col` in `render.rs`.

## How to log a change

Add a bullet under **Change log** on the widget page:

```
- YYYY-MM-DD — short why. What you changed and the pitfall you are protecting.
```
