# Minimap

Circular, zoomed track with numbered riders. Settings subtitle: “Circular track with numbered riders”.

## Code

- Draw: `draw_minimap` in `overlay/hud/src/render.rs`
- Zoom: `mini_view_radius` (0% ≈ 85 m radius, 100% ≈ 22 m)
- Shared marks with Map: dots, chevrons, crowns, S/F, arrows, `rider_dot_col`
- Settings: `pane_minimap` in `overlay/src/settings.rs`

Drawn into a square pixmap (`MINI_PX`, reused) then blitted as a circle. Layout box in settings is also forced square.

## Behavior

With live telemetry: **north-up along the track**, origin on you, only the nearby polyline (`append_visible_track`). Heading comes from `track_forward` (polyline tangent), falling back to radar axes.

Without telemetry: whole-track fit, world-up, centered on the poly bounds.

You get an orange motion trail (velocity samples). Others outside the circle are skipped. Default **Dot number** is bike **Number** (map defaults to **Position**).

Toggles match Map, plus **Zoom**. Default background 0.

## Do not regress

- Sparse centerline used to blank the widget; keep drawing with whatever poly exists (0.1.0).
- Same lapping color rules as Map. Do not invent a second palette. Off in warmup, same as Map.
- When live, keep north-up (along-track forward = up). Do not rotate the circle with bike roll/yaw as a radar.

## Change log

- 0.1.0 — No blank flash on sparse track segments.
- 0.1.4 — Slate / blue / red rider dots (lapping + closing only).
- 0.1.8 — Hidden until **Show on overlay**.
- 2026-08-18 — Wiki created. Zoom range 22–85 m and north-up live view documented.
- 2026-08-19 — Warmup keeps slate dots (shared `lap_rel` with Map).
