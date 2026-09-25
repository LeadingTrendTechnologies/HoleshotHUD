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

**Status** column (`st_status`, default off) draws Font Awesome icons (same marks as map/radar): finish (after the leader takes the flag), crash, DNS, OUT, DSQ, pit. Priority: finish → crash → DNS/OUT/DSQ → pit. Finished riders show only the flag. No column header title.

## Behavior

- Height grows with visible rows, even when the saved widget box is shorter (a Ctrl+move can write a hugged 1-row `standings_h`). If the field is larger than **Rows**, the window centers on you.
- Your row is highlighted. OUT / DNS / DSQ rows dim. **Row highlight** opacity is adjustable in settings (`st_hl`). **Text color** is White or Black (`st_text`); bike pills keep brand colors. **Alternating rows** (`st_stripe`, default on) paints every other row near-black. On a solid panel the stripe lifts to a slightly lighter charcoal so it still reads; at lower background opacity it darkens (game shows through even rows). **Plaque text** is Black or White on the orange rider-count / track-name skews (`st_plaque_text`, default Black). **Show plaques** (`st_plaque`, default on) hides those skews and collapses their band.
- In replay / spectate, clicking a rider's **name** moves the game camera to them (`SpectateVehicles`). The overlay only captures that click while hovering a name; riding is not affected.
- Best lap in the field is purple.
- Bike column is a colored badge (`bike_color` from bike name + category). A skew bar after **Position** uses the same accent.
- Header / footer are three `BoardField` slots each (session time, riders, fuel, setup, **Gap ahead**, **Gap behind**, etc.). Gap ahead / behind are live-order place neighbors (P−1 / P+1). A pass switches who. Same lap is a live running time along the track toward that rider; a lap or more is `1L` / `-1L` from completed laps, so a crashed rider you keep passing is not a ticking wrap.
- Name column uses its configured width (and only shrinks when the table is too narrow); other columns keep configured widths. The plaque hugs that column pack, so leftover widget width is not empty glass. Ctrl+resize width grows the Name column so the plaque actually gets bigger; height grows **Rows** (3–40).
- Rows slide when order changes (`ST_SLIDE`).

Default columns on: Position, Number, Name, Gap to leader, Fastest, Last lap.

## Do not regress

- Settings labels: **Gap to leader** (`StField::Gap`) uses `gap_ms` / `gap_laps` to P1. **Gap to rider ahead** (`StField::Interval`) is gap to the rider one place ahead, not to the leader. Ini keys stay `gap` / `int`.
- Header/footer **Gap ahead** / **Gap behind** are live-order P−1 / P+1. A pass switches the rider immediately. Same lap is seconds along the track toward them, not Interval’s line time and not Relative’s shortest wrap. A live lap or more is `1L` / `-1L`.
- Last lap for you can fall back to `s.last_lap_ms` when the row has no last lap yet.
- Practice / warmup **Laps** for you can follow `current_lap - 1` when classification skipped a crashed crossing. Do not do that in a moto.
- Empty field shows “Waiting for race data”, not a blank panel.
- Rows and places come from the live order, not the raw `s.standings` array. The row window and slide animation follow it.
- Click-to-follow only runs while the plugin sees `SpectateVehicles` (replay / spectate). It must not steal the camera while you are riding.
- Leaving replay must drop camera focus back to you. A stuck spectate target keeps their row highlighted and starves the dash of your telemetry.
- Leaving spectate for the garage must hide the board. Leftover rider positions are not a session once `SpectateVehicles` stops.
- Missing `st_stripe` in the ini keeps alternating rows on.
- Missing `st_plaque` / `st_plaque_text` keep plaques on with black ink.
- Alternating rows must still read at **Background** 100% (lift, not extra black on night-ink).
- Fuel header/footer is liters/US gallons (`Fuel`) or tank percent (`Fuel %`). Empty volume is `0.0`; `--` / `--%` only when tank size is missing.
- Setup header/footer is the loaded bike setup filename stem. `--` when `RunInit` has not sent it. Restart MX Bikes after the V13 plugin.
- Ctrl+resize chrome is the hugged plaque (column pack × row stack), not leftover widget glass. Dragging it larger still grows Name / Rows — do not leave the orange box as a no-op hug.
- The night-ink plaque must cover every visible row. Do not clamp stack height to a shorter saved `standings_h`.
- Finished riders show only the Status finish flag — not crash / pit / DNS / OUT / DSQ on top of it.

## Change log

- 2026-09-24 — Live order ranks the whole field by track progress minus each rider's own penalty, so several penalties stack in one tick. See `wiki/live-order.md`.
- 2026-09-25 — Trailing green/red `*` on Position when live place ≠ on-track place due to penalties (see [live race order](../live-order.md)).
- 2026-09-24 — Live order passes riders who crashed far back (start pile-ups) and steps over riders with no track position, instead of waiting for the next gate re-score. See `wiki/live-order.md`.
- 2026-09-22 — Status finish flag wins over crash/pit for done riders. Your-row wash alpha 52 at default **Row highlight** (spider-scaled).
- 2026-09-22 — Your-row highlight wash matches Profile spider fill (full chroma, alpha 40 at default **Row highlight**).
- 2026-09-22 — Settings column names: **Gap** → **Gap to leader**, **Interval** → **Gap to rider ahead**. Same columns and ini keys.
- 2026-09-22 — Removed the separate **Crashed** text column; crash is only on **Status** icons. Status header label is blank.
- 2026-09-22 — **Status** column paints crash / DNS / OUT / DSQ / finish / pit icons (map-style marks) instead of DNS/OUT/DSQ/PIT text. Finish is inferred after the leader finishes (`done_racing`).
- 2026-09-22 — Settings column drag slides neighboring rows into place (ease-out), instead of snapping on drop.
- 2026-09-22 — Penalty column shows whole seconds as `#s` (e.g. `5s`), not lap-style `5.000`.
- 2026-09-22 — **Plaque text** (Black/White, default Black) and **Show plaques** for the orange rider-count / track-name skews. Separate from row **Text color**. Hidden plaques collapse the track band height.
- 2026-09-10 — Practice **Laps** still increment after a crash when `RunLap` advanced and classification did not.
- 2026-09-09 — **Gap ahead** / **Gap behind** stay live-order P−1 / P+1. A pass switches who. Same lap ticks along the track toward that rider; a live lap or more is `1L` / `-1L` so a crashed rider you keep passing is not a shortest-wrap blip. Not the nearest on-track rider. Times have no leading `+`; ahead is an up arrow, behind a down arrow.
- 2026-09-08 — **Gap ahead** and **Gap behind** are header/footer options (`BoardField::GapAhead` / `GapBehind`). Live-order P−1 / P+1, not the riders around you on track.
- 2026-09-06 — Plaque height follows the visible row stack. A short saved widget box no longer leaves later rows on the game with no glass.
- 2026-09-06 — Spectate → garage hides the board. Leftover rider dots after `SpectateVehicles` stops are not a session (`live_session`).
- 2026-09-03 — **Setup** is a header/footer option (`BoardField::Setup`). Same `RunInit` filename as Dash / Relative / H-Standings. Restart MX Bikes after this plugin so SHM `Local\MXBOHudV13` loads.
- 2026-08-31 — Ctrl+resize of the hugged plaque grows Name width and **Rows**, so the table can get larger instead of the orange box being a no-op.
- 2026-08-31 — Ctrl+resize orange box (and grab handles) follow the hugged plaque, not leftover widget glass.
- 2026-08-31 — Plaque, header/footer, stripes, bike pill, and row slide share `draw_table_board` with Relative. Windowing and cell text stay here.

- 2026-08-29 — **Fuel %** is a separate header/footer option from volume.

- 2026-09-10 — Speed, Liquids, and Temperature units are independent in Settings. Legacy `units=` still seeds all three on upgrade.
- 2026-08-29 — Fuel reads as liters or US gallons from Liquids units, not percent.

- 2026-08-29 — Fuel level is a header/footer option (`BoardField::Fuel`). Percent of the tank from SHM `fuel` / `maxFuel`. Restart MX Bikes after this plugin so the V10 mapping is live.

- 2026-08-27 — Plaque width hugs the columns so leftover widget space is not empty glass. Fresh-install board is 20% of the screen as a max (was 30%). Existing layouts in the ini are unchanged.

- 2026-08-26 — Leaving replay drops spectate focus so your row / dash come back. `SpectateVehicles` only owns `focusRaceNum` while it is still being called.

- 2026-08-25 — Board follows the live race order, so a pass moves the row when it happens instead of at the line. Gap is still the game's number; interval is a size so a just-passed pair cannot read negative.
- 2026-08-24 — Interval / current lap / session best read from shared `RaceStore` after each frame tick. No visual change.
- 0.1.0 — Configurable header fields (shared with Relative).
- 0.1.4 — Lapping row tints live on Relative, not this table. Map/minimap got the slate / blue / red rider-dot rules that standings does not use for rows.
- 0.1.8 — Hidden until **Show on overlay**. Sidebar tab has its own icon.
- 2026-08-18 — Wiki created from current `draw_standings` / `StField` behavior.
