# Radar

Proximity blips beside and behind you. Settings subtitle: “Riders beside and behind you”. No track outline.

## Code

- Draw: `draw_radar` in `overlay/hud/src/render.rs`
- Axes: `radar_axes` (velocity if moving, else yaw). Yaw in radians if `|yaw| > 6.5`, else already radians.
- Settings: `pane_radar` in `overlay/src/settings.rs`

## Geometry (meters, bike frame)

| Constant | Value | Meaning |
| --- | --- | --- |
| `RADAR_FWD_AHEAD` | 3 | Almost nothing in front (this is rear/side radar) |
| `RADAR_FWD_REAR` | 12 | How far behind |
| `RADAR_LAT` | 6 | How far beside |
| `RADAR_SIDE_LAT` | 0.4 | Deadband: closer than this is “in line”, not beside |
| `RADAR_REAR_FWD` | −0.6 | Behind threshold |
| `RADAR_STRETCH_M` | 20 | Must be on the same stretch of `track_pos` |

`radar_in_view`: rear blips if `radar_rear` and behind; side blips if `radar_sides` and `|lat| > 0.4`. You are a white bike silhouette near the top third (`radar_you_frac`).

Blips heat by distance (closer = larger, more orange). Farther blips draw first so near ones sit on top. Crashed riders (and pit / DNS / out / DSQ) use the same `draw_state_mark` triangle as map/minimap — crash is the common one on radar. You get the mark on the white bike if you are down.

Local position is predicted with `age`, same as map/minimap. Requires telemetry; otherwise only the empty panel + wedges draw.

## Do not regress

- Filter with `radar_same_stretch` or riders on the other side of the track light the radar.
- Do not show far-ahead traffic; the forward cap is 3 m on purpose.
- Panel opacity default is 86, unlike map/minimap.
- Nearby crashed riders keep the map crash triangle (`\u{f071}`), not a color-only blip.

## Change log

- Overlay radar added as a separate widget from map/minimap (side + rear only).
- 0.1.8 — Hidden until **Show on overlay**.
- 2026-08-18 — Wiki created. Stretch filter and meter caps documented.
- 2026-08-19 — Radar blips (and you) show the map crash / state icon so a downed rider beside or behind you is obvious.
