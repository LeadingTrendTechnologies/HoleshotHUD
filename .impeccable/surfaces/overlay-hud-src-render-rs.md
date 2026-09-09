---
version: 1
slug: "overlay-hud-src-render-rs"
primary_target: "overlay/hud/src/render.rs"
related_targets: ["overlay/hud/src/render_tests.rs","wiki/widgets/radar.md","wiki/widgets/sector.md","wiki/widgets/systems.md","wiki/widgets/telemetry.md"]
---

---
version: 1
slug: "overlay-hud-src-render-rs"
primary_target: "overlay/hud/src/render.rs"
related_targets: ["overlay/hud/src/render_tests.rs","wiki/widgets/radar.md","wiki/widgets/sector.md","wiki/widgets/systems.md","wiki/widgets/telemetry.md"]
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
Twin Columns. Night-ink 6px plaque, 1px hairline frame and split. CPU left, MEM right, huge ExtraBold Italic percents, gold heat tracks (red ≥90). Watched-app rows under each column (defaults HUD / MX Bikes / MXB App / ReShade / OBS; at most 8). Footer: FPS number (no bar) with ping under it, GPU with the same process rows. No green, no orange.

## Memorable moment
MX Bikes 22% / 1.8 GB / 38% GPU sitting on the three lists — the hog is obvious without reading a 12-row list.

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
| Heat tracks | fill_round 1.5px | gold #FAB430, red #EF4444 at ≥90; FPS has no track |
| Ping | type | under FPS; ICMP ms or `—` |
| Process rows | type + thin grey tracks | watched apps under CPU, MEM, and GPU (default HUD / MX Bikes / MXB App / ReShade / OBS); `—` when missing |
| Type | existing HUD face | ExtraBold Italic via push_style |

## Sectors

## Scope
Operate · overlay HUD widget `draw_sector` in `overlay/hud/src/render.rs`.

## Audience / job
MX Bikes racer mid-session. Glance: am I up or down vs my best in this sector. Drop eyes for LAST / -2 / -3 times and the lap total.

## Direction
Underboard. Night-ink 6px plaque. Live four-column strip on top (columns size to their times, leftover on the current sector, orange skew S#, LAP stacked time/delta, ideal pill). Hairline. IDEAL then LAST / -2 / -3 aligned under the same columns. You-row gold on the fastest log lap, not IDEAL. History times green/red vs best. Short boxes stay live-only.

## Memorable moment
LAST lap total in green sitting under this-lap's red live LAP delta — you were faster last time. IDEAL is the stitched best sectors.

## Approved comp
`.impeccable/mocks/sector-lap-column.png`

Glass rim: `.impeccable/mocks/sector-glass-halo.png`

## Unresolved
None.

## Inventory
| Region | Medium | Notes |
| --- | --- | --- |
| Panel | tiny-skia fill_round | night-ink #0A0A0A @ sector_bg, 6px |
| Frame | 1px stroke round_rect | hairline #2A2A2E, alpha from panel |
| Live columns | fill_rect splits | S1/S2/S3/LAP sized to measured type; leftover on current sector; LAP never hero |
| Hero wash | fill_rect | orange #FF9430 @ 28 |
| S# plaque | fill_skew | Holeshot Orange parallelogram |
| Live delta | Exo 2 ExtraBold Italic | green / red / dim -- ; 1px night-ink rim when bg < 40 |
| Split pills | fill_night_pill | this-lap sector times |
| Live LAP | stacked type | running lap time over full-lap delta; no pill on the stack; rim when bg < 40 |
| Ideal pill | fill_night_pill | best S1+S2+S3 in the LAP column |
| History rows | type + you-row wash | IDEAL then LAST / -2 / -3; LAP stacks total over delta; rim when bg < 40 |
| Type | existing HUD face | ExtraBold Italic via push_style |

# Lean widget

## Scope
Operate · overlay HUD widget `draw_lean` in `overlay/hud/src/render.rs`.

## Audience / job
MX Bikes racer mid-session. Glance: how far am I leaned, how much bar, and whether the nose is up or down.

## Direction
Figure (default). Night-ink 6px plaque, 1px hairline. White rear-view MX rider that rolls with the bike. Orange skew 32° TV bug on the hip. 2px steer hairline under the boots and 2px pitch hairline on the right while riding; spectate hides steer and pitch.

Minimal (opt-in Look). Same plaque. Type, not a gyro. Huge orange ExtraBold Italic signed lean (`+32°`). Cream pitch degrees under it while riding. Steer hairline and percent along the bottom. Spectate is the lean number only. No labels.

## Memorable moment
Figure: the white rider is over at 32° and the orange bug sits on the hip — lean is a body, not a tach. Pitch fill climbs the right hairline when the nose comes up.
Minimal: `+32°` fills the glass in orange — lean is a number you can read without decoding an instrument.

## Approved comp
Figure: `.impeccable/mocks/decision/lean-figure.png`
Minimal: `.impeccable/mocks/decision/lean-min-numbers.png`

## Unresolved
None.

## Inventory
| Region | Medium | Notes |
| --- | --- | --- |
| Panel | tiny-skia fill_round | night-ink #0A0A0A @ lean_bg, 6px |
| Frame | 1px stroke round_rect | hairline #2A2A2E |
| Rider (Figure) | raster `lean-rider.png` rotated | white #F8F8FC silhouette, pivot at rear tire |
| Lean (Minimal) | ExtraBold Italic | Holeshot Orange #FF9430, signed `+32°`, ~46% of body |
| Pitch (Minimal) | ExtraBold Italic | cream #F8F8FC, signed degrees of ±60; hidden in spectate |
| Degree bug (Figure) | fill_skew | Holeshot Orange #FF9430, ink-on-accent 32° on the hip |
| Steer | 2px hairline + type | orange 4px fill; night-ink halo + percent pill when bg < 40; hidden in spectate |
| Pitch (Figure) | 2px vertical hairline + type | orange up = nose up, percent of ±60°; same halo/pill; hidden in spectate |
| Type | existing HUD face | ExtraBold Italic via push_style |

# Telemetry widget

## Scope
Operate · overlay HUD widget `draw_telemetry` in `overlay/hud/src/render.rs`. New Cockpit widget. Throttle / brake / clutch need SHM (V14).

## Audience / job
MX Bikes racer mid-session. Glance: how have I been on the gas and brake, what are the live analog levels, and what gear/speed am I in.

## Direction contract
THESIS: A horizontal input strip whose glance is the throttle/brake traces — not a second Dash and not a generic sim-HUD neon tape.
OWN-WORLD: Night-ink 6px plaque, 1px hairline. Exo 2 ExtraBold Italic. Ahead-green throttle, behind-red brake, dim clutch. Orange is you (label pip, RPM arc). No dotted grid, no neon, no drop shadow.
STORY: The rider reads recent throttle/brake without looking at a pad picture, then the live bars and gear/speed.
FIRST VIEWPORT: No left label. Night-ink well with overlaid green/red traces opens the strip. Three analog bars (clutch / brake / throttle) with the live % on the active one. Right circular gear well hugging the rounded end — huge gear, dim unit, speed, orange RPM arc.
FORM: Local extension of Broadcast Booth Glass. Reference strip topology, Holeshot language. Narrow request — no concept-seed.
FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance

## Memorable moment
A green throttle drop and a red brake spike sit on the same well — the last corner is a picture, not a guess.

## Approved comp
`.impeccable/mocks/telemetry-strip.png` — Strip Tape, left TELEMETRY spine removed.

## Unresolved
Comp approval. Brake bar uses front+rear max. Local rider only (spectate uses focus vehicle throttle/front brake when present).
