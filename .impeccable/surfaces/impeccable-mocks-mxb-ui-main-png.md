---
version: 1
slug: "impeccable-mocks-mxb-ui-main-png"
primary_target: ".impeccable/mocks/mxb-ui-main.png"
related_targets: [".impeccable/mocks/mxb-ui-garage.png",".impeccable/mocks/mxb-ui-options.png",".impeccable/mocks/mxb-ui-lobby.png",".impeccable/mocks/mxb-ui-pit.png",".impeccable/mocks/mxb-ui-replay.png"]
---

# MX Bikes native menus

## Scope
Operate · full PiBoSo `ui/` pack (15 menus). Comps only. Not the overlay HUD. Not `ingame_hud`.

## Audience / job
Rider in the MX Bikes menus. Pick a dest, tune, join, go to track. Eyes stay on their rider and bike.

## Direction
Modern left rail over track. Floating F8 night-ink plaque, 8px rows, orange pip. Same 3D rider/bike stage as today.

## Memorable moment
Single Player lights orange on the rail while #77 and the rider stay put.

## Approved comp
`.impeccable/mocks/mxb-ui-main.png`

## System comps
- Main: `.impeccable/mocks/mxb-ui-main.png`
- Garage: `.impeccable/mocks/mxb-ui-garage.png`
- Options: `.impeccable/mocks/mxb-ui-options.png`
- Lobby: `.impeccable/mocks/mxb-ui-lobby.png`
- Pit: `.impeccable/mocks/mxb-ui-pit.png`
- Replay: `.impeccable/mocks/mxb-ui-replay.png`
- Incumbent: `.impeccable/mocks/decision/mxb-ui-incumbent-main.jpg`

## Direction contract
THESIS: Keep today's rider stage; restyle dests as a modern F8 rail, not a stock grey list or a Holeshot-branded app.
OWN-WORLD: Night-ink 8px floating plaque, hairline #2A2A2E, ExtraBold Italic, orange pip + tab-on wash, content-sized orange skew for the one action. No Holeshot mark or HUD wordmark.
STORY: Same dests as MX Bikes. Rider and bike stay visible on main/pit/replay.
FIRST VIEWPORT: Left inset rail; grey studio, #77 bike, standing rider on the right.
FORM: Left rail over track (list 7 of 7) modernized; seed 25da8c40.
FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance

## Constraints
- Keep dialog names, control ids, `textid` keys.
- Restyle colors, TGA, fonts, some rects.
- No HTML in-game. Bake Exo 2 / rounds into TGA and `.fnt` later.

## Sprite / token map
| Game asset | Holeshot token |
| --- | --- |
| `item_darkbox` 37 37 37 | night-ink #0A0A0A / charcoal #18191D |
| `main2/3.tga` dest rows | 8px rail row, pip on selected |
| `button1/2/3.tga`, `b_button` | charcoal fill, hairline |
| `done1/2.tga` To Track / Done / Join | orange skew plaque, ink #0C0C0E |
| `back1/2.tga` | dim ExtraBold Italic on charcoal |
| `tabh/tabv` | F8 tab, orange pip |
| `dialog*.tga` | night-ink 8px, hairline |
| `logo_ui.tga` | omit or keep MX Bikes only — never Holeshot mark |
| `main.fnt` | bake Exo 2 ExtraBold Italic |
| tooltip in `ui.ui` | text #E4E4E6, back night-ink, border hairline |
| selected / you | #FF9430 / you-row #C48424 |
| list backcolor | panel #141416 |

## Screen types
Left-rail over 3D: `main.mnu`. Form-dense: `garage.mnu`, `testsetup.mnu`. Tabbed: `options.mnu`. List: `multijoin.mnu`, `multiclient.mnu`, `viewreplays.mnu`, `profiles.mnu`. Bottom-bar pit: `test.mnu`, `testday.mnu`, `straightrhythm.mnu`. Replay: `replay.mnu`. Modals: `connection.mnu`, `export.mnu`.

## Unresolved
None for comps. Pack translation is a later pass.
