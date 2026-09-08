# Radar

Proximity blips beside and behind you. Settings subtitle: “Riders beside and behind you”. No track outline. Hairline range arcs at **3 / 6 / 12 m**, centered on the bike, with dim labels on the outer two arcs (hidden when the widget is under 72 px). **Range** only changes how far dots show — rings stay 6 and 12.

## Code

- Draw: `draw_radar` in `overlay/hud/src/render.rs`
- Axes: `radar_axes` (velocity if moving, else yaw). Yaw in radians if `|yaw| > 6.5`, else already radians.
- Settings: `pane_radar` in `overlay/src/settings.rs`

## Geometry (meters, bike frame)

| Constant / setting | Value | Meaning |
| --- | --- | --- |
| `radar_range` | 6–30 m, default 12 | How far behind (and half of that beside) dots show. Rings stay 3 / 6 / 12; past 12 m, dots sit outside the 12 m ring |
| `RADAR_FWD_AHEAD` | 3 | Almost nothing in front (this is rear/side radar) |
| `RADAR_SIDE_LAT` | 0.4 | Deadband: closer than this is “in line”, not beside |
| `RADAR_REAR_FWD` | −0.6 | Behind threshold |
| `RADAR_STRETCH_M` | max(20, range) | Must be on the same stretch of `track_pos` |

`radar_in_view`: rear blips if `radar_rear` and behind; side blips if `radar_sides` and `|lat| > 0.4`. You are a white bike silhouette near the top (`radar_you_frac` from the 3 m forward cap vs range), with a night-ink outline so the mark still reads on a light sky when the panel is glass.

Blips heat by distance (closer = larger, more orange). Farther blips draw first so near ones sit on top. Colors follow the Range Arcs mock: close `#FA7602`, far cream `#E4C670`, both opaque with a same-hue glow (no dark halo). Size is `0.020 + heat×0.014` of the widget, clamped 7–15 px. Crashed riders (and pit / DNS / out / DSQ) use the same `draw_state_mark` triangle as map/minimap — crash is the common one on radar. You get the mark on the white bike if you are down.

**Range rings** (default on) can be toggled in settings / the demo. Off: panel, bike, and blips only. Stroke and the two outer labels lift with panel opacity so they still read on a solid `#0E0E10` plaque (100% background). Rings are always 3 / 6 / 12 m. Raising **Range** past 12 m leaves those rings in place and puts farther blips outside the 12 m ring.

Local position is predicted with `age`, same as map/minimap. Requires telemetry; otherwise only the empty panel + your bike mark draw.

## Do not regress

- Filter with `radar_same_stretch` or riders on the other side of the track light the radar.
- Do not show far-ahead traffic; the forward cap is 3 m on purpose.
- Rings stay 3 / 6 / 12 m at every **Range**. Do not relabel them to 6 / 12 / 24 when the slider goes up.
- Default `radar_range` is 12 m so the 12 m ring still fills the plaque. Past 12 m, only the view grows so extra dots sit outside that ring.
- Panel opacity default is 86, unlike map/minimap.
- Nearby crashed riders keep the map crash triangle (`\u{f071}`), not a color-only blip.
- Do not draw the old orange side/rear zone wedges again; the panel is range arcs + blips + your bike.
- Range rings are circles fitted inside the plaque (at 12 m the 12 m ring touches the sides or the bottom, not stretched to fill height). Past 12 m, that extra distance is what touches the edge. The two outer rings sit in a gap on the stroke. Do not draw ovals, a sci-fi sweep, or a compass.
- Range rings default on; `radar_rings` off still draws the panel, bike, and blips.
- Solid panel (high `radar_bg`) must keep the rings lighter than `#0E0E10` — do not use the hairline token `#2A2A2E` for the arcs.
- The white bike keeps a night-ink outline (`#0E0E10`) so it does not vanish on a light sky when `radar_bg` is low.

## Change log

- Overlay radar added as a separate widget from map/minimap (side + rear only).
- 0.1.8 — Hidden until **Show on overlay**.
- 2026-08-18 — Wiki created. Stretch filter and meter caps documented.
- 2026-08-19 — Radar blips (and you) show the map crash / state icon so a downed rider beside or behind you is obvious.
- 2026-08-25 — Removed the cream/orange blind-spot wedges behind the bike mark.
- 2026-08-29 — **Range rings** toggle (default on). Rings lift with panel opacity so they stay readable at 100% background. Blips are larger (7–15 px), orange→cream with glow.
- 2026-09-06 — **Range** slider (6–30 m, default 12) sets how far dots show. Rings stay 3 / 6 / 12; past 12 m, dots sit further out from the 12 m ring. Forward cap stays 3 m. Heat still reads against 12 m.
- 2026-09-08 — White bike mark gets a night-ink outline so it still reads on a light sky when the panel is glass.
