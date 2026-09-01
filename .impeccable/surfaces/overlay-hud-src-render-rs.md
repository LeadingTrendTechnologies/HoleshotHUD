---
version: 1
slug: "overlay-hud-src-render-rs"
primary_target: "overlay/hud/src/render.rs"
related_targets: ["overlay/hud/src/render_tests.rs","wiki/widgets/radar.md","wiki/widgets/sector.md","wiki/widgets/systems.md"]
---

# Radar widget

## Scope
Operate · overlay HUD widget `draw_radar` in `overlay/hud/src/render.rs`.

## Audience / job
MX Bikes racer mid-session. Glance: who is beside or behind me, and how close. Eyes stay on the track.

## Direction
Range Arcs. Night-ink 6px plaque with a 1px hairline frame, white bike in the upper third, heat-colored circular blips. Hairline circles at 3 / 6 / 12 m fitted inside the glass (12 m to the sides). Dim ExtraBold Italic 6 and 12 sit in gaps on the lower arcs. No wedges, no sweep, no title bar.

## Memorable moment
A rider 6 m back sits on the labeled ring — distance is a graphic, not a guess.

## Approved comp
`.impeccable/mocks/radar-arcs.png`

## Unresolved
None.

## Inventory
| Region | Medium | Notes |
| --- | --- | --- |
| Panel | tiny-skia fill_round | night-ink #0E0E10 @ radar_bg, 6px |
| Frame | 1px stroke round_rect | #3A3A3E, alpha from panel |
| Range rings | stroke circle with bottom gap | 3 / 6 / 12 m fitted to plaque; 6/12 gapped |
| 6 / 12 labels | Exo 2 Black Italic | dim #84848A in the stroke gap |
| Bike | fill_round + nose path | white #F8F8FC, dark halo |
| Blips | fill_circle | heat orange; closer = larger |
| Crash mark | draw_state_mark | same triangle as map |
| Type | existing HUD face | ExtraBold Italic via push_style |

# Systems widget

## Scope
Operate · overlay HUD widget `draw_sys` in `overlay/hud/src/render.rs`.

## Audience / job
MX Bikes racer mid-session. Glance: is the PC the problem, and is it the HUD or the game. Eyes stay on the track.

## Direction
Twin Columns. Night-ink 6px plaque, 1px hairline frame and split. CPU left, MEM right, huge ExtraBold Italic percents, gold heat tracks (red ≥90). Four process rows under each column. Footer: FPS (cream when healthy) and NET. No green, no orange.

## Memorable moment
MX Bikes 22% / 1.8 GB sitting under the two big numbers — the hog is obvious without reading a 12-row list.

## Approved comp
`.impeccable/mocks/sys-twin.png`

## Unresolved
None.

## Inventory
| Region | Medium | Notes |
| --- | --- | --- |
| Panel | tiny-skia fill_round | night-ink #0A0A0A @ sys_bg, 6px |
| Frame | 1px stroke round_rect | #2A2A2E, alpha from panel |
| Split | 1px fill_rect | vertical + footer hairline |
| Main numbers | Exo 2 ExtraBold Italic | #F8F8FC, ~22% of body height |
| Heat tracks | fill_round 1.5px | gold #FAB430, red #EF4444 at ≥90; FPS cream when fill ≥70 |
| Process rows | type + thin grey tracks | HUD / MX Bikes / MXB App / ReShade; `—` when missing |
| Type | existing HUD face | ExtraBold Italic via push_style |

## Sectors

## Scope
Operate · overlay HUD widget `draw_sector` in `overlay/hud/src/render.rs`.

## Audience / job
MX Bikes racer mid-session. Glance: am I up or down vs my best in this sector. Drop eyes for LAST / -2 / -3 times.

## Direction
Underboard. Night-ink 6px plaque. Live three-column strip on top (current sector ~56% hero, orange skew S#). Hairline. LAST / -2 / -3 completed laps aligned under the same columns. You-row gold on the fastest log lap. History times green/red vs best. Short boxes stay live-only.

## Memorable moment
LAST S2 in green sitting under this-lap's red +0.120 — the sector you just lost, you had last lap.

## Approved comp
`.impeccable/mocks/sector-history-underboard.png`

## Unresolved
None.

## Inventory
| Region | Medium | Notes |
| --- | --- | --- |
| Panel | tiny-skia fill_round | night-ink #0A0A0A @ sector_bg, 6px |
| Frame | 1px stroke round_rect | hairline #2A2A2E, alpha from panel |
| Live columns | fill_rect splits | S1/S2/S3; current ~56% |
| Hero wash | fill_rect | orange #FF9430 @ 28 |
| S# plaque | fill_skew | Holeshot Orange parallelogram |
| Live delta | Exo 2 ExtraBold Italic | green / red / dim -- |
| Split pills | fill_night_pill | this-lap times |
| History rows | type + you-row wash | LAST / -2 / -3; LAST @ 72% you-row |
| Type | existing HUD face | ExtraBold Italic via push_style |
