# Standings

Vertical classification board: place, number, name, and timing columns. Settings subtitle: “Who is ahead and by how much”.

## Code

- Draw: `draw_standings` in `overlay/hud/src/render.rs` (chrome via `draw_table_board`)
- Classification view: `RaceStore` / `RaceField` in `overlay/hud/src/race_store.rs` (interval, current lap, session best)
- Columns / header / footer: `StField`, `BoardField`, `HudConfig` in `overlay/hud/src/config.rs`
- Settings pane: `pane_standings` in `overlay/src/settings.rs`
- In-game HUD copy (not the overlay): `drawStandings` in `src/hud/widgets.cpp`

## Data

Classification array order **is race order**; `position = index + 1`. Names come from the entry list, joined onto standings rows.

Rows are drawn from `RaceField::board()` — the game order with on-track passes applied, places renumbered from it. See [live race order](../live-order.md). The game only republishes its classification at the line, so iterating `s.standings` directly would hold a stale board for up to a lap.

Primary SHM fields: race num, position, state, best / last lap, laps, gap ms / laps, pit, penalty, bike, category, crashed.

Status labels: `1` DNS, `3` OUT, `4` DSQ, else PIT if `pit != 0`.

## Behavior

- Height grows with visible rows. If the field is larger than **Rows**, the window centers on you.
- Your row is highlighted. OUT / DNS / DSQ rows dim. **Row highlight** opacity is adjustable in settings (`st_hl`). **Text color** is White or Black (`st_text`); bike pills keep brand colors. **Alternating rows** (`st_stripe`, default on) paints every other row near-black. On a solid panel the stripe lifts to a slightly lighter charcoal so it still reads; at lower background opacity it darkens (game shows through even rows).
- In replay / spectate, clicking a rider's **name** moves the game camera to them (`SpectateVehicles`). The overlay only captures that click while hovering a name; riding is not affected.
- Best lap in the field is purple.
- Bike column is a colored badge (`bike_color` from bike name + category). A skew bar after **Position** uses the same accent.
- Header / footer are three `BoardField` slots each (session time, riders, fuel, etc.).
- Name column uses its configured width (and only shrinks when the table is too narrow); other columns keep configured widths. The plaque hugs that column pack, so leftover widget width is not empty glass. Ctrl+resize width grows the Name column so the plaque actually gets bigger; height grows **Rows** (3–40).
- Rows slide when order changes (`ST_SLIDE`).

Default columns on: Position, Number, Name, Gap, Fastest, Last lap.

## Do not regress

- Gap to P1 uses `gap_ms` / `gap_laps`. Interval is gap to the rider one place ahead, not to the leader.
- Last lap for you can fall back to `s.last_lap_ms` when the row has no last lap yet.
- Empty field shows “Waiting for race data”, not a blank panel.
- Rows and places come from the live order, not the raw `s.standings` array. The row window and slide animation follow it.
- Click-to-follow only runs while the plugin sees `SpectateVehicles` (replay / spectate). It must not steal the camera while you are riding.
- Leaving replay must drop camera focus back to you. A stuck spectate target keeps their row highlighted and starves the dash of your telemetry.
- Missing `st_stripe` in the ini keeps alternating rows on.
- Alternating rows must still read at **Background** 100% (lift, not extra black on night-ink).
- Fuel header/footer is liters/US gallons (`Fuel`) or tank percent (`Fuel %`). Empty volume is `0.0`; `--` / `--%` only when tank size is missing.
- Ctrl+resize chrome is the hugged plaque (column pack × row stack), not leftover widget glass. Dragging it larger still grows Name / Rows — do not leave the orange box as a no-op hug.

- Presence marks are opt-in (`show_presence`). Orange overlay icon before the name, not a dot. Do not paint a mark on a rider who is not on this board, or on yourself.
- Friend column is opt-in (`highlight_friends`), needs Show overlay users, and only appears when a friend is in this race. Inserted before Name. Do not mark yourself. Same in-game name without a Steam64 is not a friend.

## Change log

- 2026-09-04 — Steam friends who also run Holeshot get a Friend column (`highlight_friends`). ID join only. Website demo turns this on with a few marked riders.
- 2026-08-31 — Opt-in Holeshot mark after the name when that rider is in this session and also running the overlay (`show_presence`).

- 2026-08-31 — Ctrl+resize of the hugged plaque grows Name width and **Rows**, so the table can get larger instead of the orange box being a no-op.
- 2026-08-31 — Ctrl+resize orange box (and grab handles) follow the hugged plaque, not leftover widget glass.
- 2026-08-31 — Plaque, header/footer, stripes, bike pill, and row slide share `draw_table_board` with Relative. Windowing and cell text stay here.

- 2026-08-29 — **Fuel %** is a separate header/footer option from volume.

- 2026-08-29 — Fuel reads as liters or US gallons from Units, not percent.

- 2026-08-29 — Fuel level is a header/footer option (`BoardField::Fuel`). Percent of the tank from SHM `fuel` / `maxFuel`. Restart MX Bikes after this plugin so the V10 mapping is live.

- 2026-08-27 — Plaque width hugs the columns so leftover widget space is not empty glass. Fresh-install board is 20% of the screen as a max (was 30%). Existing layouts in the ini are unchanged.

- 2026-08-26 — Leaving replay drops spectate focus so your row / dash come back. `SpectateVehicles` only owns `focusRaceNum` while it is still being called.

- 2026-08-25 — Board follows the live race order, so a pass moves the row when it happens instead of at the line. Gap is still the game's number; interval is a size so a just-passed pair cannot read negative.
- 2026-08-24 — Interval / current lap / session best read from shared `RaceStore` after each frame tick. No visual change.
- 0.1.0 — Configurable header fields (shared with Relative).
- 0.1.4 — Lapping row tints live on Relative, not this table. Map/minimap got the slate / blue / red rider-dot rules that standings does not use for rows.
- 0.1.8 — Hidden until **Show on overlay**. Sidebar tab has its own icon.
- 2026-08-18 — Wiki created from current `draw_standings` / `StField` behavior.
