# Map

Full-track outline in world XZ with rider dots. Settings subtitle: “Where you and others are on track”.

## Code

- Draw: `draw_map` in `overlay/hud/src/render.rs`
- Shared dots / chevrons / crowns / S/F / arrows: `draw_rider_dot`, `draw_sf`, `draw_track_arrows`, `rider_dot_col`
- Settings: `pane_map` in `overlay/src/settings.rs`
- Track poly comes from plugin tessellation (or a local XZ trail if centerline is missing). See [Home.md](../Home.md) section 5.

## Data

`poly[]` + `poly_count`, rider XZ + yaw, local XZ + velocity (for interpolation), `sf_meters`, `RaceStore` for position labels and lapping colors.

Local marker is predicted: `local + vel * age` so it stays smooth between SHM ticks.

## Behavior

- Fits the whole polyline in the rect (10% pad). Y is unused; Z is the track plane.
- Track fill + stroke is cached in `MAP_LAYER` until poly / size / S/F / arrows change. Rider dots are redrawn every frame.
- You: larger orange dot. Others: slate, or blue/red only when lapping and closing (see [widgets.md](../widgets.md)).
- Chevrons show heading. Optional: S/F, track arrows, leader crown, nearest ahead/behind marks, numbers in dots (bike number or classification position).
- Missing poly (`< 2` points) shows “No track map”.

Toggles: other riders, start/finish, track arrows, leader crown, nearest ahead/behind, numbers in dots, dot number vs position. Default background opacity is **0** (transparent over the game).

## Do not regress

- Do not flash the track blank when segments are sparse; the cache and polyline close path are what stopped that (0.1.0).
- Lapping color is **not** “anyone a lap up is blue”. They must also be behind you and inside `catch_span_m`.
- Dot **Position** labels, leader crown and the nearest ahead / behind rings use live `RaceStore` rank during a race (`standing_pos` / `leader_num` prefer `live_position` / `live_leader`). See [live race order](../live-order.md).
- No blue/red lapping dots in warmup; `lap_rel` is `Same` until the race starts.
- Map uses snapshot rect `s.map` (copied from config), not only `cfg.map` at draw time.

## Change log

- 2026-08-25 — Crown, nearest ahead / behind rings and dot position labels move with an on-track pass: `standing_pos` / `leader_num` read the live order first. Passing for the lead moves the crown to your own dot on the same frame.
- 2026-08-24 — Position labels and leader crown use live `RaceStore` rank during a race.
- 0.1.0 — Minimap no longer flashes blank on sparse segments (same poly pipeline the map uses).
- 0.1.4 — Other riders default to dark slate + white number. Blue = lap ahead and closing from behind. Red = you are a lap ahead and closing on them.
- 0.1.8 — Hidden until **Show on overlay**.
- 2026-08-18 — Wiki created. Cached track layer + predicted local marker documented.
- 2026-08-19 — Warmup keeps slate dots. Practice laps are not treated as lapping.
