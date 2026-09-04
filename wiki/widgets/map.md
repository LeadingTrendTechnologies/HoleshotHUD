# Map

Full-track outline in world XZ with rider dots. Settings subtitle: “Where you and others are on track”.

## Code

- Draw: `draw_map` in `overlay/hud/src/render.rs`
- Shared dots / chevrons / crowns / S/F / sector lines / arrows: `draw_rider_dot`, `draw_sf`, `draw_sector_lines`, `draw_track_arrows`, `rider_dot_col`
- Settings: `pane_map` in `overlay/src/settings.rs`
- Track poly comes from plugin tessellation (or a local XZ trail if centerline is missing). See [Home.md](../Home.md) section 5.

## Data

`poly[]` + `poly_count`, rider XZ + yaw, local XZ + velocity (for interpolation), `sf_meters`, `RaceStore` for position labels and lapping colors.

Orange marker is `subject_pose`: predicted `local + vel * age` while riding. Overlay clears leftover telemetry while `SpectateVehicles` is live so spectate uses the focused rider’s XZ. When that callback stops, telemetry is yours again and the marker snaps back to you even if focus is still stale.

## Behavior

- Fits the whole polyline in the rect (10% pad). Y is unused; Z is the track plane.
- Track fill + stroke is cached in `MAP_LAYER` until poly / size / S/F / arrows change. Rider dots and sector lines are redrawn every frame.
- You: larger orange dot on the camera subject (you while riding, the spectated rider in replay). Others: slate, or blue/red only when lapping and closing (see [widgets.md](../widgets.md)). Two laps down is still blue if they are closing from behind.
- Chevrons show heading. Optional: S/F, **sector lines**, track arrows, leader crown, nearest ahead/behind marks, numbers in dots (bike number or classification position).
- **Sector lines** are thin violet dotted gates at where each sector **starts** (same tape as Sectors). **S1** is the start/finish line. **S2** / **S3** appear after those splits are known for this track. Do not mark the split that *ends* S1 as S1.
- Missing poly (`< 2` points) shows “No track map”.

Toggles: other riders, start/finish, sector lines, track arrows, leader crown, nearest ahead/behind, numbers in dots, dot number vs position. Default background opacity is **0** (transparent over the game). Sector lines default on, like S/F.

## Do not regress

- Do not flash the track blank when segments are sparse; the cache and polyline close path are what stopped that (0.1.0).
- Lapping color is **not** “anyone a lap up is blue”. They must also be behind you and inside `catch_span_m`.
- Do not trust `num_laps` over `gap_laps` for blue/red. A rider two down can have a completed-lap count that looks a lap *up*; that used to paint the leader red the second time they went by. `other_laps_ahead` prefers `gap_laps`.
- Dot **Position** labels, leader crown and the nearest ahead / behind rings use live `RaceStore` rank during a race (`standing_pos` / `leader_num` prefer `live_position` / `live_leader`). See [live race order](../live-order.md).
- No blue/red lapping dots in warmup; `lap_rel` is `Same` until the race starts.
- Map uses snapshot rect `s.map` (copied from config), not only `cfg.map` at draw time.
- In spectate/replay, do not leave the orange marker on leftover local telemetry; overlay drops `has_telemetry` while `SpectateVehicles` is live so `subject_pose` uses the focused rider’s XZ.
- Leaving spectate / going back on the bike must put the orange marker on you. Live telemetry wins over a stale `focus_race_num`.
- Sector lines are thin best-lap-violet dashes at the **start** of S1 / S2 / S3, not orange. They span only the track stroke — do not stick out into the grass. Orange stays the S/F bar and the you-dot. S1 sits on the line; S2 / S3 wait for learned splits. Do not invent equal-third gates when splits are unknown.

- Presence rings on other dots are opt-in (`show_presence`) and must not wrap the you-dot.
- Friend icon over other dots is opt-in (`highlight_friends`) and Steam64-only. Same placement as the leader crown; offset left if both show. Do not mark the you-dot.

## Change log

- 2026-09-04 — Steam friends who also run Holeshot get a friend icon over the dot (`highlight_friends`). ID join only; in-game names are ignored.
- 2026-08-31 — Thin orange ring on other dots when **Show overlay users** is on and that rider is in the session presence list.

- 2026-08-30 — Sector lines mark where each sector starts (S1 at S/F, S2 / S3 at the learned splits). The old S1 / S2 labels sat on the split that *ended* that sector.
- 2026-08-29 — Second time the leader laps you no longer paints them red. Lap-down color prefers `gap_laps` over completed-lap counts.
- 2026-08-28 — Thin violet dotted S1 / S2 sector lines on the full-track map (**Sector lines**, default on). Same learned split positions as Sectors. Hidden until those splits exist for the track.
- 2026-08-27 — Spectate draws the orange you-dot on the watched rider (`subject_pose` / `camera_subject`). Overlay drops leftover telemetry while `SpectateVehicles` is live. Live telemetry after that snaps the marker back to you even if focus is still the last camera target.
- 2026-08-25 — Crown, nearest ahead / behind rings and dot position labels move with an on-track pass: `standing_pos` / `leader_num` read the live order first. Passing for the lead moves the crown to your own dot on the same frame.
- 2026-08-24 — Position labels and leader crown use live `RaceStore` rank during a race.
- 0.1.0 — Minimap no longer flashes blank on sparse segments (same poly pipeline the map uses).
- 0.1.4 — Other riders default to dark slate + white number. Blue = lap ahead and closing from behind. Red = you are a lap ahead and closing on them.
- 0.1.8 — Hidden until **Show on overlay**.
- 2026-08-18 — Wiki created. Cached track layer + predicted local marker documented.
- 2026-08-19 — Warmup keeps slate dots. Practice laps are not treated as lapping.
