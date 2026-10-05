# Map

Full-track outline in world XZ with rider dots. Settings subtitle: “Where you and others are on track”.

## Code

- Draw: `draw_map` in `overlay/hud/src/render.rs`
- Shared dots / chevrons / crowns / S/F / sector lines / arrows: `draw_rider_dot`, `draw_sf`, `draw_sector_lines`, `draw_track_arrows`, `rider_dot_col`
- Settings: `pane_map` in `overlay/src/settings.rs`
- Track poly comes from plugin tessellation (or a local XZ trail if centerline is missing). See [Home.md](../Home.md) section 5.

## Data

`poly[]` + `poly_count`, rider world XZ + yaw (centerline from **`track_pos`** when XZ is stuck), local XZ + velocity (for interpolation), `sf_meters`, `RaceStore` for position labels and lapping colors.

Dots and chevrons place at world XZ via `rider_map_pose` when that coordinate is moving, so a cut or an off-track bike shows. If XZ sits still while `track_pos` keeps advancing (~8 m), the dot falls back to the centerline (`norm_lap_pos` → `poly_at_frac`) so a frozen coord does not pin the icon. Missing poly still uses world XZ. Orange marker is `subject_pose`: predicted `local + vel * age` while riding; spectate uses the same placement for the focused rider. Overlay clears leftover telemetry while `SpectateVehicles` is live so spectate follows the camera. When that callback stops, telemetry is yours again and the marker snaps back to you even if focus is still stale.

## Behavior

- Fits the whole polyline in the rect (10% pad). Y is unused; Z is the track plane.
- **Follow me** (default off): pins the camera subject to the center. North is the centerline direction about 22 m ahead (`track_forward`), eased so a kink does not shake the track. A scrub or whip leaves the map put; the orange chevron still shows bike heading and can sit off north. At zoom 0, scale is the unrotated fit (10% pad) and does not change as you turn. **Zoom** (0–100%, only while Follow me is on) tightens that fit. 100% shows 80 m across the shorter side — about twice the minimap's closest view, not its 22 m close-up. A track that already fits inside 80 m does not zoom further. Overflow past the widget is clipped. Off, or no subject pose, keeps the fixed world-up fit and ignores zoom.
- Track fill + stroke is cached in `MAP_LAYER` until poly / size / S/F / arrows change. Rider dots and sector lines are redrawn every frame.
- You: larger orange dot on the camera subject (you while riding, the spectated rider in replay). Others: slate, or blue/red when lap-delta and within catch span either side (see [widgets.md](../widgets.md)). Two laps down is still blue if they are nearby. The disc is mostly solid, with a 1px ring in that color. **Dot opacity** (`map_dot_opacity`, default 78) scales that fill; 78 is today’s look and 100 is solid. Numbers in the dots use the app font at semibold (Exo 2 SemiBold Italic by default). When live place differs from on-track place, the digits and a trailing `*` are centered together: green when that rider is ahead of on-track, red when behind (same `penalty_place_delta` mark as Standings). The label drops one size step so the star fits. No star when numbers are off, places match, or live order is off. Bike number and position labels both get it. **My number** (`map_you_text`, default Black) sets only your number to white or black; the `*` stays green or red. Other riders keep automatic contrast. The minimap keeps the default fill and automatic numbers, and shares this star.
- Chevrons show heading and stay a solid fill. Optional: S/F, **sector lines**, track arrows, leader crown, nearest ahead/behind marks, numbers in dots (bike number or classification position). Ahead / behind marks are a light green or red wash just outside the dot with a 1px ring. Minimap shares those marks.
- **Sector lines** are thin violet dotted gates at where each sector **starts** (same tape as Sectors). **S1** is the start/finish line. **S2** / **S3** appear after those splits are known for this track. Do not mark the split that *ends* S1 as S1.
- Missing poly (`< 2` points) shows “No track map”.

Toggles: **Follow me** (default off), **Zoom** (shown only while Follow me is on, default 0), other riders, start/finish, sector lines, track arrows, leader crown, nearest ahead/behind, numbers in dots, dot number vs position, **My number** (White/Black, only while numbers are on), **Dot opacity** (0–100%, default 78). Default background opacity is **0** (transparent over the game). Sector lines default on, like S/F.

## Do not regress

- **Follow me** stays off unless the toggle is on. Off keeps the fixed world-up map and `MAP_LAYER` cache. Zoom is hidden and ignored while Follow me is off.
- When **Follow me** is on, north is travel along the centerline (`track_forward`) while `subject_pose` exists. No pose keeps the fixed map. Do not drive that rotation from yaw or velocity, or a scrub spins the track.
- Follow me scale at zoom 0 stays the unrotated fit. Zoom raises that scale up to an 80 m window and does not change as you turn. Do not zoom tighter than that, and do not zoom to keep the whole track inside while you are centered. Clip overflow to the widget.
- Do not flash the track blank when segments are sparse; the cache and polyline close path are what stopped that (0.1.0). Follow mode redraws that same closed path each frame instead of the cache.
- Place other-rider (and spectate) dots on live world XZ so a cut or an off-track bike shows. If that XZ sits still while `track_pos` advances about 8 m, fall back to the centerline so a frozen coord does not pin the icon. At the gate / prestart, keep world XZ so stalls do not stack on one centerline point.
- Leader crown, ahead / behind rings, and dot position numbers stay on live race order. Moving a dot off the racing line does not change who leads or who is ahead.
- Lapping color is **not** “anyone a lap up is blue”. They must also be inside `catch_span_m` (either side — holds through a pass while nearby).
- Do not trust `num_laps` over `gap_laps` for **blue**. A rider two down can have a completed-lap count that looks a lap *up*; that used to paint the leader red the second time they went by. `other_laps_ahead` uses `gap_laps` only when they are ahead of you.
- Red only when you gained a lap on them (pairwise). Leader lapping someone behind you is not red.
- Same-race S/F straddles must not paint blue/red. When `num_laps` differ, `other_laps_ahead` requires continuous `num_laps + track_pos` to round non-zero.
- Dot **Position** labels, leader crown and the nearest ahead / behind rings use live `RaceStore` rank during a race (`standing_pos` / `leader_num` prefer `live_position` / `live_leader`). See [live race order](../live-order.md).
- Dot numbers use the app semibold face (Exo 2 SemiBold Italic by default), even when the Map or Minimap bold toggle is on. The widget UI face is ExtraBold.
- A numbered dot trails a green or red `*` when live place differs from on-track place. No star when numbers are off, places match, or live order is off. Nearest ahead / behind rings stay the separate wash outside the dot.
- Missing `map_you_text` stays Black. Missing `map_dot_opacity` stays 78. Neither setting changes the minimap.
- Nearest ahead / behind rings stay off in warmup and practice (`place_rings_for_session`). Crowns and the settings toggle are unchanged.
- No blue/red lapping dots in warmup; `lap_rel` is `Same` until the race starts. `session_kind` 5 wins even when extras leak.
- **Show on maps** (Groups, default off) recolors another rider's dot and chevron with the first group's color. Riders in no group stay slate, blue, or red. The orange you-dot stays orange. Minimap shares it.
- Map uses snapshot rect `s.map` (copied from config), not only `cfg.map` at draw time.
- In spectate/replay, do not leave the orange marker on leftover local telemetry; overlay drops `has_telemetry` while `SpectateVehicles` is live so `subject_pose` uses the focused rider’s `track_pos` (XZ fallback).
- Leaving spectate / going back on the bike must put the orange marker on you. Live telemetry wins over a stale `focus_race_num`.
- Sector lines are thin best-lap-violet dashes at the **start** of S1 / S2 / S3, not orange. They span only the track stroke — do not stick out into the grass. Orange stays the S/F bar and the you-dot. The S/F bar spans only the track stroke. S1 sits on the line; S2 / S3 wait for learned splits. Do not invent equal-third gates when splits are unknown.

## Change log

- 2026-10-05 — Factory box is square on 16:9 (19.125%×34%), right edge unchanged. The track uses the shorter side, so the old 21% width was unused glass. An untouched 21%×34% box picks that up. A dragged box stays.
- 2026-10-04 — Place numbers and the crown stay off on the gate. After the drop they follow track location through the first lap. See [live race order](../live-order.md).
- 2026-10-04 — **Show on maps** (Groups, default off) recolors another rider's dot and chevron with the first group's color. The orange you-dot stays orange. Minimap shares it.
- 2026-10-02 — Numbered dots trail a green or red `*` when a penalty puts live place ahead of or behind on-track place. Minimap shares it. Nearest ahead / behind rings stay outside the dot.
- 2026-10-02 — Ahead / behind marks are a light wash and a 1px ring outside the dot. Chevrons stay solid. Minimap shares the marks.
- 2026-10-02 — **My number** sets your map number to white or black (default Black). **Dot opacity** scales the map dots (default 78, today’s fill). Minimap is unchanged.
- 2026-10-02 — Dot numbers use the app font at semibold (Exo 2 SemiBold Italic by default), not Roboto. Minimap shares them.
- 2026-10-01 — Rider dots are a translucent disc with a 1px color ring. Minimap shares them. Chevrons stay solid.
- 2026-10-01 — Lap 1 place labels move at S1 and S2, and between those gates, before the finish. See [live race order](../live-order.md).
- 2026-10-01 — **Zoom** only while Follow me is on. 0% is the whole-track fit. 100% shows 80 m across the shorter side, wider than the minimap's closest view.
- 2026-10-01 — The S/F bar spans only the track stroke. Minimap shares the same bar.
- 2026-10-01 — Other riders and the spectate marker use world XZ when it is live, so a cut or an off-track bike shows. A stuck XZ still falls back to the centerline. Place, crown, and ahead/behind rings stay on live race order.
- 2026-09-25 — Nearest ahead / behind rings stay off in warmup and practice; race motos keep them.
- 2026-09-25 — **Follow me** north is the centerline ahead, not the bike. Scale stays the unrotated fit and overflow is clipped. A scrub or whip no longer spins or resizes the map.
- 2026-09-25 — **Follow me** (default off) pins you to the center and rotates the map with you.
- 2026-09-25 — Gate / prestart dots stay on world XZ so starting stalls do not pile onto one centerline point; race still uses `track_pos`.
- 2026-09-25 — Other-rider and spectate dots follow live `track_pos` on the centerline (`rider_map_pose`); world XZ is fallback only so icons move with standings/dash.
- 2026-09-24 — Red is pairwise only: leader lapping someone behind you no longer paints them red.
- 2026-09-24 — Same-race S/F straddles no longer paint blue/red (`other_laps_ahead` continuous progress when `gap_laps` match).
- 2026-09-22 — Blue/red dots hold through a pass while still within catch span (either side).
- 2026-09-13 — Review lines stay off the live map; they only draw in Analyze.
- 2026-09-12 — Location tape records for Review (not drawn live).
- 2026-09-07 — Warmup (`session_kind` 5) keeps dots slate even when leaked extras make the lap field look like a race.
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
