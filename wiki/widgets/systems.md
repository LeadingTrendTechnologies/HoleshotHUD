# Systems

Host PC meters: CPU, memory, FPS, network, plus per-app CPU/mem. Settings subtitle: “CPU, memory, FPS, network, and per-app load”.

Same as every other widget: it **hides** in the menu / lobby / garage and when MX Bikes is not running. Hold Ctrl to place the box anyway.

## Code

- Draw: `draw_sys` in `overlay/hud/src/render.rs`
- Sample: `overlay/src/sys.rs` calls `set_sys_stats` / `set_sys_procs`
- Settings: `pane_sys` in `overlay/src/settings.rs`

## Layout

Twin columns inside a night-ink 6px plaque with a 1px hairline frame and a hairline split.

- **Left:** CPU — huge percent, gold heat track, four process CPU rows
- **Right:** MEM — huge percent, gold heat track, four process mem rows
- **Footer:** FPS left, NET right, same hairline split

Process apps: **HUD**, **MX Bikes**, **MXB App**, **ReShade**. Missing process shows `—` and a dim bar. Mem sub-bars scale to the heaviest of those processes, not total RAM.

Heat is gold (`#FAB430`), red at ≥90. FPS uses cream when the fill is healthy (high FPS), gold/red when low. No green bars. No Holeshot orange (that color is you).

No column picker; only show, opacity, font, bold, snap.

Default box matches a compact overlay size (`0.086 × 0.186`). Existing layouts in the INI keep their size.

## Do not regress

- Do not draw with `snap == None` or an empty session. Same hide rule as race widgets. Layout boxes (`settings_hint`) still force a draw.
- Labels are fixed length-4 (`SYS_PROC_N`). Changing the set means `sys.rs` and the atomics together.
- Do not paint load with ahead-green or Holeshot orange.
- FPS is the game’s Draw publish rate, not the overlay loop. SHM `seq` is a seqlock (advances by 2 per publish); divide by 2. Do not mix seq FPS with overlay frame counting while a snapshot is live.

## Change log

- 2026-08-30 — FPS was bouncing (~38 overlay loop vs ~140 seqlock units) because `note_fps` mixed both meters and treated seqlock `seq` as a frame counter (it steps by 2 per Draw). Game FPS is now `seq_delta / 2 / dt`; overlay FPS only when there is no snapshot.
- 2026-08-29 — Twin Columns: CPU | MEM boards, process rows under each, FPS/NET footer. Gold heat instead of a green task-manager list. Default size `0.086 × 0.186`.
- 2026-08-27 — A blank HUD with Systems off still follows the shared hide rule; the overlay plaque (not this widget) explains plugin-missing / widgets-off.
- 2026-08-26 — Hides without a session, same as the rest of the HUD. Menu / lobby no longer show CPU meters.
- Overlay Systems widget added for overlay vs game vs ReShade load.
- 0.1.8 — Hidden until **Show on overlay**.
- 2026-08-18 — Wiki created. Off-track visibility and process list documented.
