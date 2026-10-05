# Pit Board

A light coaching-board pit plate (Hero Stack): last lap and delta as the heroes, identity and place as support. Settings subtitle: “Slots from the plate, pick what each shows”. The overlay only **passes variables**. F8 places the slots on the plate (count, location, size, color) and writes `board.json`. Each row still picks the stat.

Regular Cockpit widget. Hidden until **Show on overlay**.

## How riders use it

F8 opens with the plate designer:

1. **Browse…** a PNG or JPEG. JPEG is stored as PNG in the pitboards folder. The overlay still draws PNG only
2. A picture with a sibling `board.json` uses those slots. A picture alone starts as one blank slot
3. The preview shows the plate. Drag a slot to place it. The type bar on the selected slot sets **Size** (10–56) and **Text** (the letters, not a background). **Add slot** / **Remove slot** change the count, up to 12. New slots are named Slot N, stat None, black, until you pick a stat
4. Under the plate, F8 lists one menu per slot in two columns. Every menu is the same as Standings header and footer (plus name, last, delta, and the other pit stats). **None** hides the spot. Size and position stay. The type bar follows the selected slot
5. **When**: Always (live values), end of each sector (5 s snapshot of that sector), or end of each lap (5 s snapshot of that lap)
6. **Reset layout** restores the Holeshot PNG, factory slots, black text, Main (the current accent), and Secondary (black)

Nothing on the overlay plate can be dragged. Ctrl+drag still moves and resizes the **whole widget**. Slot position, count, and color are authored in settings and written to `board.json`.

## Code

- Draw: `draw_pitboard` in `overlay/hud/src/render/pitboard.rs`
- Catalog / pack: `overlay/hud/src/pitboard.rs` (`PitVar`, placements, PNG copy, `board.json`, `load_sidecar`)
- Data: `RaceStore` / Dash formatters / `delta::view_for` — no plugin or SHM bump
- Settings: `pane_pitboard` in `overlay/src/settings/widgets.rs`

Factory look is the shipped pack `overlay/hud/assets/holeshot.png` + `board.json` (Hero Stack slots, black text on the light plate). The banner carries the Holeshot app logo and the word **HOLESHOT**. Main and Secondary in F8 recolor that plate. Main starts as the accent. Secondary starts black. A saved layout that still has the old plate yellow and navy is treated as unset. `sample.png` is the same art with the colors baked in, so Browse can be tried without touching the factory colors. On launch the overlay writes `holeshot.png` over `Documents\PiBoSo\MX Bikes\Holeshot-HUD\pitboards\holeshot.png`, copies `sample.png` when that file is missing, and rewrites `board.json` when the file is missing or still the previous glass pack (white text, the old five coordinates). A factory plate whose rows are still Name, Position, Laps, Last, and Delta — or Top, Position, Lap, Lap time, and Lap delta — is renamed to Slot 1–5. A moved slot, a renamed slot, or another plate stays. Custom art is a different PNG in the same folder. Browse accepts PNG or JPEG and stores PNG. A sibling `board.json` sets the slots. A picture alone starts as one blank slot. Procedural glass is only a fallback if the PNG cannot be decoded.

Default 18%×22%, upper-center, the holeshot.png aspect on a 16:9 screen. Panel opacity 100 so the white plate stays solid. Fresh install: `show_pitboard = false`. Factory rows are **Slot 1** through **Slot 5**. Slot 1 draws nothing until you pick a stat. Slot 2 is position and Slot 3 is lap (`P#` left, `L#` right). Slot 4 is lap time, the only large line. Slot 5 is the lap delta (green ahead, red behind). A shown stat with no value draws `---`. Default text is **Black**. Default **When** is Always (`pit_when=always`): slots show live values (Last is last lap, Current is the running clock, Delta is live). Sector holds 5 seconds with a snapshot of the sector that just finished. Lap holds 5 seconds with a snapshot of the lap that just finished. F8 or Ctrl-drag still shows the board live so you can place it.

## Do not regress

- Hidden until **Show on overlay**.
- While any preset has Show on overlay, an empty `pitboard.cfg` in the game `misc/hud` hides the stock 2D board. A sibling `holeshot-pitboard` marker means we created it. Do not delete a `pitboard.cfg` that has no marker. The small 3D board on the track stays.
- Do not let riders drag individual stats on the overlay. Slot positions are locked to the design.
- Do not treat this as a second Dash footer — last + delta stay the heroes on the factory layout.
- Factory default is the shipped `holeshot.png` + `board.json` pack. The plate is the light coaching board. The banner mark is the app logo. Do not paint a second bike, a URL, or the word RIDER on it. Procedural glass is only the decode fallback.
- A saved board is `pitboards/<Name>/plate.png` plus that folder’s `board.json`. The menu loads both. Reset layout returns to Holeshot and does not delete those folders. The default board’s name is **Holeshot** and cannot be edited. Upload and Import from that board use the picture or folder name. A saved board keeps an editable name. Leaving that field renames the folder. Enter saves. Escape restores the saved name. A refused or already used name stays in the field.
- Download demo saves the chevron plate as `holeshot-demo.png` only. It does not write `board.json`.
- Make your own explains how to build a plate. It does not save or change the live board. When sits beside Board. Delete sits beside Name.
- Pit Board has no font-size or bold row. Bold is per slot and is written on that slot in `board.json`. A slot with no `bold` stays regular.
- Upload Pitboard Image takes a picture only. Import opens a `board.json` and requires `plate.png` in that same folder. An empty import name uses the folder name. Delete is hidden while Holeshot is the live board. Holeshot cannot be exported or deleted.
- Browse accepts PNG or JPEG and stores PNG. Do not draw JPEG in the HUD. No SVG/URLs. No `..` / `/` / `\` in the art name. 8 MB max.
- F8 authors slots on the plate preview (drag, size, color, add/remove, cap 12). Do not let riders drag individual stats on the overlay.
- Factory default is five slots (Slot 1 through Slot 5). Slot 1 draws nothing until a stat is picked. Do not pad unused catalog stats. F8 lists one row per `board.json` slot — never the 29-stat catalog. An old ini dump of every PitVar must normalize to the pack.
- A shown stat with no value draws `---`. **None** stays blank. Do not draw `P--` / `L--`.
- Each slot’s name comes from `board.json` (else the `default` key). The rows under the plate show that slot’s stat name, or None, and are not menus. Clicking a row selects that slot. The plate stays blank for None. A chosen stat with no value shows `---` on the plate. Removing a slot rewrites later `Slot N` names to the new index. A name that is not `Slot N` stays. The type bar menu is Standings header/footer (`BoardField`) plus the remaining pit stats. Changing the value must not move or resize the slot. **None** hides that spot.
- Browse a picture with a sibling `board.json` must apply those placements. A picture alone starts as one blank slot.
- Black text is the factory default on the light plate. White text stays available for a dark custom plate.
- Main and Secondary recolor only the Holeshot plate. Main defaults to the accent and stays put if the accent changes later. Secondary defaults to black. A browsed PNG, including `sample.png`, keeps the colors in the file.
- Ctrl+drag still owns the widget box.
- Delta green/red comes from the tape (`delta::view_for`), same as the Delta Bar.
- Do not put sponsor in `board.json` or F8. Brand mark lives in the PNG.
- **When** default is Always (live field values). Sector flashes 5 seconds with the sector that just finished (time + sector delta). Lap flashes 5 seconds with the lap that just finished (last lap + delta vs best). Last / Current / Delta use that snapshot; other slots stay live. Do not require the Sectors widget to be on. A new split or completed lap refreshes the 5 s. F8 and Ctrl-drag still show the board live.

## Change log

- 2026-10-05 — Factory box is 18%×22% so holeshot.png (1600×1100) is not stretched on 16:9. An untouched 32%×22% box picks that up. A dragged box stays.
- 2026-09-29 — Leaving the name field renames the saved board folder. Enter saves. Escape restores the previous name. The Board menu shows the new name.
- 2026-09-29 — The sentence under When is gone. Hover still explains Always, end of sector, and end of lap.
- 2026-09-29 — Clicking a row under the plate selects that slot. The row stays a readout. Size, Bold, and Text follow it.
- 2026-09-29 — A None slot shows the word None on the row under the plate. The plate stays blank.
- 2026-09-29 — The rows under the plate name the stat, such as Class Position, not the sample. None stays blank.
- 2026-09-29 — The rows under the plate show the sample for each slot. They are not menus. None is blank on the plate and in those rows. A chosen stat with no value shows `---`.
- 2026-09-29 — Removing a slot renames later Slot N rows. Slot 1 stays. Slot 3 becomes Slot 2. A name that is not Slot N stays. Download demo and Make your own sit above Upload, Export, and Import.
- 2026-09-29 — When sits beside Board. Delete sits beside Name. Pit Board no longer has a font-size or bold row. Bold is a toggle on the selected slot.
- 2026-09-29 — Board sits above Name and When. Make your own opens the steps for a custom plate. Download demo saves `holeshot-demo.png`.
- 2026-09-28 — Download demo saves the chevron plate as `holeshot-demo.png` only. No `board.json`. Upload that file to try a picture with one blank slot.
- 2026-09-28 — The slot center buttons show a tooltip: Center horizontally / Won't move up or down, and Center vertically / Won't move left or right.
- 2026-09-28 — Center horizontal and Center vertical use the same axis marks as snapping a widget to the monitor.
- 2026-09-28 — Delete sits on the Board row for a saved board. Center horizontal and Center vertical snap the selected slot to the middle of the plate.
- 2026-09-28 — Upload Pitboard image, Export, and Import sit at the top of the Pit Board page. Reset layout shows only while the default Holeshot board is on screen. Delete stays under the board menu for a saved board.
- 2026-09-28 — Show on overlay on any preset writes an empty `pitboard.cfg` (and a `holeshot-pitboard` marker) into the game `misc/hud`, which hides the stock 2D pitboard. Turning it off on every preset deletes that pair only. A `pitboard.cfg` we did not create stays. The 3D board on the track stays. MX Bikes must be fully quit before the stock board updates.
- 2026-09-28 — The default board name is Holeshot and cannot be edited. Upload and Import from that board use the picture or folder name. The file row says Upload Pitboard image. Remove is Remove slot. Open folder is gone.
- 2026-09-28 — Pit Board settings lead with a type bar for the selected slot: its stat, Size, and Text. Text opens the color picker and colors the letters. The plate sits under the bar. Slot menus are two columns. Name and When share the row above the board menu.
- 2026-09-28 — A named board is a folder under `pitboards` (`plate.png` and `board.json`). Browse requires a name and saves that folder. The board menu loads one saved design, plate and slots together. An empty name does not save. Reset layout returns to Holeshot and does not delete saved folders. A picture with no sibling `board.json` still starts as one blank slot.
- 2026-09-28 — Export saves the live plate and its `board.json` beside the PNG. Import opens a PNG only when that sibling file is present, and the name row is required, including on Holeshot. Delete removes one saved folder. Deleting the live board returns to Holeshot. Holeshot cannot be exported or deleted.
- 2026-09-28 — Export copies the board folder (`plate.png` and `board.json`) into a folder you pick. Browse and Import open while Holeshot is selected. An empty name uses the picture's file name. `holeshot.png` is never replaced.
- 2026-09-28 — Upload Pitboard Image opens a picture. Import opens `board.json` and needs `plate.png` beside it. An empty import name uses that folder's name. Delete is hidden on Holeshot.
- 2026-09-28 — When sits at the top of the Pit Board page. The settings preview of the Holeshot plate uses Main and Secondary, so the default plate is orange and black rather than the yellow and navy in the PNG.
- 2026-09-28 — Settings can place slots on a browsed PNG or JPEG. Drag sets location. Size, text color, and add/remove (up to 12) write `board.json`. JPEG is stored as PNG. A picture with no sibling `board.json` starts as one blank slot.
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
