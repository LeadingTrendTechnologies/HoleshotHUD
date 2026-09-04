# Minimap

Circular, zoomed track with numbered riders. Settings subtitle: “Circular track with numbered riders”.

## Code

- Draw: `draw_minimap` in `overlay/hud/src/render.rs`
- Zoom: `mini_view_radius` (0% ≈ 85 m radius, 100% ≈ 22 m)
- Shared marks with Map: dots, chevrons, crowns, S/F, sector lines, arrows, `rider_dot_col`
- Settings: `pane_minimap` in `overlay/src/settings.rs`

Drawn into a square pixmap (`MINI_PX`, reused) then blitted as a circle. Layout box in settings is also forced square.

## Behavior

With a camera-subject pose (live telemetry while riding, or the spectated rider’s XZ): **north-up along the track**, origin on them, only the nearby polyline (`append_visible_track`). Heading comes from `track_forward` (polyline tangent), falling back to radar axes while riding or the rider’s yaw while spectating.

Without a pose: whole-track fit, world-up, centered on the poly bounds.

While riding you get an orange motion trail (velocity samples). Spectate has no local vel, so no trail. Others outside the circle are skipped. Default **Dot number** is bike **Number** (map defaults to **Position**). **Sector lines** match Map (dotted S1 / S2 / S3 at sector starts); a gate outside the zoomed circle is skipped.

Toggles match Map, plus **Zoom**. Default background 0.

## Do not regress

- Sparse centerline used to blank the widget; keep drawing with whatever poly exists (0.1.0).
- Same lapping color rules as Map. Do not invent a second palette. Off in warmup, same as Map. Two laps down stays blue when they close from behind.
- Position labels, leader crown and ahead / behind rings use live `RaceStore` rank during a race (same as Map). See [live race order](../live-order.md).
- When live, keep north-up (along-track forward = up). Do not rotate the circle with bike roll/yaw as a radar.
- Follow / north-up must use `subject_pose`, not `has_telemetry` alone, or spectate falls back to a whole-track fit with no orange you-dot.
- After spectate, live telemetry must put the origin back on you. Do not keep following a stale camera target.
- Sector lines use the same sector-start gates as Map (S1 at S/F, S2 / S3 at learned splits). Do not paint them orange. Skip a gate that is outside the circle.
- Presence rings share Map (`show_presence`). Do not wrap the you-dot. Overlay does not change the fill.
- Friend teal other-dots share Map (`highlight_friends`). Do not recolor the you-dot.

## Change log

- 2026-09-04 — Shares Map friend teal other-dots (`highlight_friends`).
- 2026-09-04 — Shares Map overlay rings (`show_presence`).
- 2026-08-31 — Shares the Map presence ring on other dots (`show_presence`).

- 2026-08-30 — Shares the Map fix: sector lines mark where each sector starts, not where the previous one ended.
- 2026-08-29 — Shares the Map fix: a rider two laps up stays blue (not red) when closing from behind.
- 2026-08-28 — Thin violet dotted S1 / S2 sector lines, same toggle and splits as Map. Hidden when that split is outside the zoomed circle.
- 2026-08-27 — Spectate follows the watched rider with the orange you-dot and north-up origin. Overlay drops leftover telemetry while spectating; riding turns it back on so the view snaps to you.
- 2026-08-25 — Shares the Map's live-order marks: crown and ahead / behind rings follow a pass immediately.
- 2026-08-24 — Position labels and leader crown use live `RaceStore` rank during a race.
- 0.1.0 — No blank flash on sparse track segments.
- 0.1.4 — Slate / blue / red rider dots (lapping + closing only).
- 0.1.8 — Hidden until **Show on overlay**.
- 2026-08-18 — Wiki created. Zoom range 22–85 m and north-up live view documented.
- 2026-08-19 — Warmup keeps slate dots (shared `lap_rel` with Map).
