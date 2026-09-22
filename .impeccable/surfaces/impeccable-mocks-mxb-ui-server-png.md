---
version: 1
slug: "impeccable-mocks-mxb-ui-server-png"
primary_target: ".impeccable/mocks/mxb-ui-server.png"
related_targets: [".impeccable/mocks/mxb-ui-main.png",".impeccable/mocks/mxb-ui-lobby.png"]
---

# MX Bikes server browser

## Scope
Operate · comps only for `multijoin.mnu` (LAN/World + chrome). Not the overlay HUD. Not `ingame_hud`.

## Audience / job
Rider picking a session to join. Scan the list, filter, Join.

## Direction
Locked: inset board. Same F8 world as the main dest card. Live list stays.

## Memorable moment
One session row lights orange with a left pip; Join is the only skew plaque.

## Constraints
- Same dests as MX Bikes: Host, Filter name, Hide empty / missing, 12-column list, Local / World, Back, Spectate, My Bike, Join, Refresh, Info, Password.
- Columns stay Name D P Comp Track Category Length H W Status Ping Rating.
- Design around the live list in `.impeccable/mocks/decision/mxb-ui-incumbent-server.jpg` — keep those session names and counts, do not invent a cleaner lobby.
- No F8 overlay tabs. No Holeshot wordmark.
- Keep dialog names and control ids when packed later.

## Anti-goals
- Do not reuse `.impeccable/mocks/mxb-ui-lobby.png` overlay chrome.
- Do not invent columns the game does not have.

## Direction contract
THESIS: The live session list sits on a floating night-ink board; studio stays around it. Not a full-bleed table, not overlay HUD tabs.
OWN-WORLD: Night-ink 8px plaque, hairline #2A2A2E, ExtraBold Italic, orange pip + tab-on wash, Join as the only content-sized skew.
STORY: Rider scans the real 12-col list, filters, Join. Same dests as MX Bikes.
FIRST VIEWPORT: Inset board on grey studio. Host / Filter / Hide empty / missing on top. List in the card. Local / World and Join on the card. Helmet and dirt can peek at the edges.
FORM: Inset board (user lock); specified pair, no seed.
FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance

## Unresolved
None. Inset board is packed in `game_ui.rs` (`multijoin.mnu` splice).
