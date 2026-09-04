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
| Delta Bar | Delta Bar | `Delta` | [delta-bar.md](widgets/delta-bar.md) |
| Sectors | Sectors | `Sector` | [sector.md](widgets/sector.md) |
| Lean | Lean | `Lean` | [lean.md](widgets/lean.md) |
| Controller | Controller | `Gamepad` | [gamepad.md](widgets/gamepad.md) |

## Shared rules

- Draw path: `overlay/hud/src/render.rs` (`draw()`). Settings: `overlay/src/settings.rs`. Layout/ini: `overlay/hud/src/config.rs`.
- Shared race view: `overlay/hud/src/race_store.rs` — `RaceStore::refresh` once per overlay frame, then `RaceStore::with` (re-entrant, no clone). `tick()` still returns a clone for tests and clock logs.
- Settings widget panes share `WidgetPaneSpec` / `open_widget_pane` (heading, Show on overlay, style rows, extras, snap). Standings/relative use `table_style_controls` so **Row highlight** / **Text color** / **Alternating rows** stay on those boards. App / Feedback / What’s new stay custom. High-contrast settings palette is still OS-aware.
- **Live race order**: the game only republishes its classification when someone crosses the line, so `race_store` re-derives places every tick — the game order with any pass we can see in `riders[].track_pos` applied on top (`live_order` / `passed`). `RaceField.rows` come back in that order with `standing.position` set to it, and `live_position` / `live_leader` serve map-style lookups. See [live race order](live-order.md).
- All widgets start **hidden** on a fresh install (`show_* = false`). Turn on with **Show on overlay**.
- Every widget has font size, bold, opacity, and snap-to-monitor. Hold **Ctrl** and drag to move or resize.
- Overlay font families: default **Exo 2**. Also Segoe / Arial / Tahoma / Roboto, **Teko**, **Goldman**, **Montserrat**. Old ini keys `agency` / `industry` / `faster` map to Exo 2 / Teko / Goldman. `bebas` and `impact` map to Goldman and Montserrat.
- Minimap / radar / dash / ticker / sys / stance / flag / delta / sector / lean read `cfg.show_*` directly. Controller (`Gamepad`) also needs Settings → Labs → Experimental widgets (`cfg.gamepad_visible()`).
- All widgets draw when telemetry or rider positions are present — including replay / spectate, which never set `RunInit`. Standings alone after `RunDeinit` are leftover garage / lobby data and must not keep the HUD up. They hide in the menus, lobby, and garage, and when MX Bikes is not running. Only the taskbar on MX Bikes' monitor is hidden (`overlay/src/compat.rs`); other screens keep Start and the taskbar. Clicking those screens must not hide or freeze the HUD. A hitch that pauses plugin publish still keeps the last session HUD for 15s (`overlay/src/main.rs`); do not blank at the 2.5s live cutoff. Delta and Sectors keep ticking in that window so a wrap across S/F can still commit. Hold Ctrl for layout boxes; that still forces a draw. The Holeshot HUD icon stays in the top-right of the MX Bikes monitor only — not on other screens — whenever the overlay is on the game (proof it is compositing); a click opens settings even while Ctrl is down. The settings key toggles settings closed if it is already open. A top plaque explains a blank HUD: no widget on (F8 → Show on overlay), widgets on but you are in the garage/menus (appear on track), or no live plugin data for 2s (fully quit MX Bikes and start it again). That plugin restart plaque wins over the fullscreen-optimizations “stay on top” line when both apply. After an in-app update that changed the plugin while MX Bikes was open, What’s new and Settings → Install say the same until the game process is gone. Overlay SHM maps the whole `Local\\MXBOHudV13` section so a leftover smaller mapping still opens.
- Overlay copies `Holeshot-HUD.dlo` into the game `plugins` folder from the plugin **baked into the running exe**. A leftover sidecar next to the exe must not win after an in-app update. Dev `cargo` builds with an empty embed still fall back to `out/Release`.

## Shared rider colors (map, minimap, relative rows)

Default other-rider color is dark slate. **Blue** only if they are a lap ahead **and** closing from behind. **Red** only if you are a lap ahead **and** closing on them. Two (or more) laps down is still blue when they close from behind — `other_laps_ahead` prefers `gap_laps` over `num_laps` so a lapped rider whose completed-lap count sits on the race lap does not invert the leader to red. You are always orange. Off in warmup (`is_warmup`) — practice lap counts are not race lapping. See `lap_rel` / `rider_dot_col` in `render.rs`.

## How to log a change

Add a bullet under **Change log** on the widget page:

```
- YYYY-MM-DD — short why. What you changed and the pitfall you are protecting.
```
