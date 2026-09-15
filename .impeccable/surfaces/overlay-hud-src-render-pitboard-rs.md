---
version: 1
slug: "overlay-hud-src-render-pitboard-rs"
primary_target: "overlay/hud/src/render/pitboard.rs"
related_targets: ["overlay/hud/src/pitboard.rs","wiki/widgets/pitboard.md"]
---

# Pit Board

## Scope
Operate · overlay HUD widget `draw_pitboard` in `overlay/hud/src/render/pitboard.rs`.

## Audience / job
MX Bikes racer mid-session. Glance last lap and signed delta at race speed. Name and place confirm whose board. Eyes stay on the track.

## Direction
Hero Stack. Rounded night-ink glass pit board. Last lap is the board; signed delta is the call. Identity and the three-across row are support.

## Memorable moment
Huge last lap on glass, green delta under it, two handle holes in the top corners.

## Approved comp
`.impeccable/mocks/decision/pitboard-pick.png`

## Unresolved
None.

## Direction contract

THESIS: A glass pit board: identity, then place, then huge last and signed delta. Refuses a white number-plate sim HUD and equal-cell dash grids.

OWN-WORLD: Night-ink glass, Holeshot Orange hairline, Exo 2 ExtraBold Italic, cream last, ahead-green / behind-red delta. Two top handle holes.

STORY: Rider and stream glance last and signed delta; name and P# confirm whose board.

FIRST VIEWPORT: Landscape rounded plaque. Holes top-left and top-right. Sponsor and name at the top. P# · session · laps across the middle. Huge last lap, signed delta under it.

FORM: Hero Stack (1 of 7), seed aefb5ee4.

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
