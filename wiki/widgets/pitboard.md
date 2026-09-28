# Pit Board

A light coaching-board pit plate (Hero Stack): last lap and delta as the heroes, identity and place as support. Settings subtitle: “Slots from the plate, pick what each shows”. The overlay only **passes variables**. `board.json` lists the slots (default, size, location); F8 remaps the value.

Regular Cockpit widget. Hidden until **Show on overlay**.

## How riders use it

F8 opens with a Make your own note:

1. Draw a PNG of the plate and leave room for the numbers
2. Put `board.json` in that same folder. **Open folder** writes a v2 starter
3. `slots` is an array. Each slot is `name`, `default`, `x`, `y`, and `size` (0 to 1). `name` is the F8 row title. Optional `color` is `#RRGGBB` for that slot's text. No color means black. As many slots as you want
4. **Browse…** the PNG. Sibling `board.json` becomes the slot list. No `sponsor` field — put brand art in the PNG
5. F8 lists one menu per slot. Every menu is the same as Standings header and footer (plus name, last, delta, and the other pit stats). **None** hides the spot. Size and position stay
6. **White** or **Black** text
7. **When**: Always (live values), end of each sector (5 s snapshot of that sector), or end of each lap (5 s snapshot of that lap)
8. **Reset layout** restores the Holeshot PNG, factory slots, black text, Main (the current accent), and Secondary (black)

Nothing on the plate can be dragged. Ctrl+drag still moves and resizes the **whole widget**. Custom slot positions are authored in the design (`board.json` beside the PNG), not by dragging on the overlay.

## Code

- Draw: `draw_pitboard` in `overlay/hud/src/render/pitboard.rs`
- Catalog / pack: `overlay/hud/src/pitboard.rs` (`PitVar`, placements, PNG copy, `board.json`, `load_sidecar`)
- Data: `RaceStore` / Dash formatters / `delta::view_for` — no plugin or SHM bump
- Settings: `pane_pitboard` in `overlay/src/settings/widgets.rs`

Factory look is the shipped pack `overlay/hud/assets/holeshot.png` + `board.json` (Hero Stack slots, black text on the light plate). The banner carries the Holeshot app logo and the word **HOLESHOT**. Main and Secondary in F8 recolor that plate. Main starts as the accent. Secondary starts black. A saved layout that still has the old plate yellow and navy is treated as unset. `sample.png` is the same art with the colors baked in, so Browse can be tried without touching the factory colors. On launch the overlay writes `holeshot.png` over `Documents\PiBoSo\MX Bikes\Holeshot-HUD\pitboards\holeshot.png`, copies `sample.png` when that file is missing, and rewrites `board.json` when the file is missing or still the previous glass pack (white text, the old five coordinates). A factory plate whose rows are still Name, Position, Laps, Last, and Delta — or Top, Position, Lap, Lap time, and Lap delta — is renamed to Slot 1–5. A moved slot, a renamed slot, or another plate stays. Custom art is a different PNG in the same folder. Browse applies a sibling `board.json` when present. Procedural glass is only a fallback if the PNG cannot be decoded.

Default 32%×22%, upper-center. Panel opacity 100 so the white plate stays solid. Fresh install: `show_pitboard = false`. Factory rows are **Slot 1** through **Slot 5**. Slot 1 draws nothing until you pick a stat. Slot 2 is position and Slot 3 is lap (`P#` left, `L#` right). Slot 4 is lap time, the only large line. Slot 5 is the lap delta (green ahead, red behind). A shown stat with no value draws `---`. Default text is **Black**. Default **When** is Always (`pit_when=always`): slots show live values (Last is last lap, Current is the running clock, Delta is live). Sector holds 5 seconds with a snapshot of the sector that just finished. Lap holds 5 seconds with a snapshot of the lap that just finished. F8 or Ctrl-drag still shows the board live so you can place it.

## Do not regress

- Hidden until **Show on overlay**.
- Do not let riders drag individual stats on the overlay. Slot positions are locked to the design.
- Do not treat this as a second Dash footer — last + delta stay the heroes on the factory layout.
- Factory default is the shipped `holeshot.png` + `board.json` pack. The plate is the light coaching board. The banner mark is the app logo. Do not paint a second bike, a URL, or the word RIDER on it. Procedural glass is only the decode fallback.
- Do not load JPEG/SVG/URLs. PNG only, no `..` / `/` / `\` in the art name, 8 MB max.
- F8 must keep the Make your own steps (PNG, sibling `board.json` slots array, Browse, no drag).
- Factory default is five slots (Slot 1 through Slot 5). Slot 1 draws nothing until a stat is picked. Do not pad unused catalog stats. F8 lists one row per `board.json` slot — never the 29-stat catalog. An old ini dump of every PitVar must normalize to the pack.
- A shown stat with no value draws `---`. **None** stays blank. Do not draw `P--` / `L--`.
- Each slot has a menu labeled by `name` from `board.json` (else the `default` key). Every menu is Standings header/footer (`BoardField`) plus the remaining pit stats. Changing the value must not move or resize the slot. **None** hides that spot.
- Browse a PNG with a sibling `board.json` must apply those placements. A PNG alone keeps the current slots.
- Black text is the factory default on the light plate. White text stays available for a dark custom plate.
- Main and Secondary recolor only the Holeshot plate. Main defaults to the accent and stays put if the accent changes later. Secondary defaults to black. A browsed PNG, including `sample.png`, keeps the colors in the file.
- Ctrl+drag still owns the widget box.
- Delta green/red comes from the tape (`delta::view_for`), same as the Delta Bar.
- Do not put sponsor in `board.json` or F8. Brand mark lives in the PNG.
- **When** default is Always (live field values). Sector flashes 5 seconds with the sector that just finished (time + sector delta). Lap flashes 5 seconds with the lap that just finished (last lap + delta vs best). Last / Current / Delta use that snapshot; other slots stay live. Do not require the Sectors widget to be on. A new split or completed lap refreshes the 5 s. F8 and Ctrl-drag still show the board live.

## Change log

- 2026-09-28 — When is one dropdown: Always, end of each sector, or end of each lap. Panel opacity and the black/white text toggles are gone. The plate stays solid. Slot text is black unless that slot in `board.json` has `color`. A lap delta with no color stays green ahead and red behind.
- 2026-09-28 — Factory slots sit higher so the bottom value clears the stripes. A save still on the old factory spots is lifted on load. The chosen stat stays. A moved slot stays. Plate edges are anti-aliased after recolor. The logo stays sharp.
- 2026-09-28 — A factory `board.json` still titled Name, Position, Laps, Last, and Delta, or Top, Position, Lap, Lap time, and Lap delta, is renamed to Slot 1–5 on launch. The chosen stat stays. A moved slot, a renamed slot, or another plate stays.
- 2026-09-28 — F8 rows are Slot 1 through Slot 5. A shown stat with no value draws `---`. A save that is still an older factory five is replaced with this pack on load.
- 2026-09-28 — Factory default is the Lap layout. Top draws nothing until you pick a stat. Position, lap, lap time, and lap delta are on. A save that still shows the name in that top spot hides it on load.
- 2026-09-28 — Factory text is a stack in the white field: quiet name, place and laps on one line, a large last lap, and the signed delta under it. No captions.
- 2026-09-28 — The banner mark is the Holeshot app logo. The small bike on the lower spear is gone. The logo is stamped after recolor so Main and Secondary do not wash it out.
- 2026-09-28 — Factory colors are Main and Secondary. Main starts as the accent. Secondary starts black. Reset on the row and Reset layout restore those. A save that still has the old plate yellow and navy is treated as unset. The recolored plate is cached so a color click does not walk the image every frame.
- 2026-09-28 — Factory yellow and blue are settings (`pit_yellow`, `pit_blue`). They recolor the Holeshot plate, including the word and the bike. A browsed PNG is left as painted. `sample.png` is that plate with the baked colors, copied into the pitboards folder when missing, for trying Browse.
- 2026-09-28 — Factory plate is the light coaching board. Only the Holeshot bike and the word HOLESHOT are painted on it. Slots sit in the white center in black. Panel opacity 100. A still-glass `board.json` (white text, the old five coordinates) is rewritten on launch.
- 2026-09-11 — Always is live values. Sector/lap flashes snapshot the sector or lap that just finished (not the next lap start).
- 2026-09-11 — Every F8 slot menu is the Standings header/footer list, plus name, last, delta, and the other pit stats.
- 2026-09-11 — F8 slot row titles come from `board.json` `name`, not the catalog label.
- 2026-09-11 — F8 Slots is one row per `board.json` slot. Old 29-stat ini lists collapse to the pack.
- 2026-09-11 — No sponsor in JSON. HOLESHOT is on the PNG. F8 slot menus match the pack’s slots only.
- 2026-09-11 — Factory default is five slots. Placeholders (`--`, `P--`, `L--`) do not draw.
- 2026-09-11 — `board.json` v2 `slots` array is the source of truth (name, default, x, y, size). F8 one menu per slot; the row title is `name`.
- 2026-09-11 — Each location has a menu. Name, P1, L, and the rest can show any stat, or None.
- 2026-09-11 — Always keeps running times. Sector/lap flashes snapshot that sector or the last lap.
- 2026-09-11 — Factory default is `holeshot.png` + `board.json`. Reset layout restores that pack.
- 2026-09-11 — When: Always, end of each sector (5 s), or start of each lap (5 s).
- 2026-09-11 — Removed the F8 sponsor Set/Clear row. Packs can still set sponsor in `board.json`.
- 2026-09-11 — F8 note walks through making a plate: PNG, sibling `board.json`, Browse. Slots stay locked.
- 2026-09-11 — Slot positions lock to the design. No overlay drag. Browse applies sibling `board.json`.
- 2026-09-11 — Factory plate is Hero Stack: glass plaque, top holes, last and delta as the heroes.
- 2026-09-11 — First ship. Glass plate plus optional PNG; tick-and-drag catalog from F8.
