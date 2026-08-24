# Standings

Vertical classification board: place, number, name, and timing columns. Settings subtitle: “Who is ahead and by how much”.

## Code

- Draw: `draw_standings` in `overlay/hud/src/render.rs`
- Classification view: `RaceStore` / `RaceField` in `overlay/hud/src/race_store.rs` (interval, current lap, session best)
- Columns / header / footer: `StField`, `BoardField`, `HudConfig` in `overlay/hud/src/config.rs`
- Settings pane: `pane_standings` in `overlay/src/settings.rs`
- In-game HUD copy (not the overlay): `drawStandings` in `src/hud/widgets.cpp`

## Data

Classification array order **is race order**; `position = index + 1`. Names come from the entry list, joined onto standings rows.

Primary SHM fields: race num, position, state, best / last lap, laps, gap ms / laps, pit, penalty, bike, category, crashed.

Status labels: `1` DNS, `3` OUT, `4` DSQ, else PIT if `pit != 0`.

## Behavior

- Height grows with visible rows. If the field is larger than **Rows**, the window centers on you.
- Your row is highlighted. OUT / DNS / DSQ rows dim.
- Best lap in the field is purple.
- Bike column is a colored badge (`bike_color` from bike name + category). A skew bar after **Position** uses the same accent.
- Header / footer are three `BoardField` slots each (session time, riders, etc.).
- Name column flexes; other columns keep configured widths.
- Rows slide when order changes (`ST_SLIDE`).

Default columns on: Position, Number, Name, Gap, Fastest, Last lap.

## Do not regress

- Gap to P1 uses `gap_ms` / `gap_laps`. Interval is gap to the rider one place ahead, not to the leader.
- Last lap for you can fall back to `s.last_lap_ms` when the row has no last lap yet.
- Empty field shows “Waiting for race data”, not a blank panel.

## Change log

- 2026-08-24 — Interval / current lap / session best read from shared `RaceStore` after each frame tick. No visual change.
- 0.1.0 — Configurable header fields (shared with Relative).
- 0.1.4 — Lapping row tints live on Relative, not this table. Map/minimap got the slate / blue / red rider-dot rules that standings does not use for rows.
- 0.1.8 — Hidden until **Show on overlay**. Sidebar tab has its own icon.
- 2026-08-18 — Wiki created from current `draw_standings` / `StField` behavior.
