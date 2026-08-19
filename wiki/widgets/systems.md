# Systems

Host PC meters: CPU, memory, FPS, network, plus per-app CPU/mem. Settings subtitle: “CPU, memory, FPS, network, and per-app load”.

Unlike the other widgets, this one **still draws when you are not on track**, and even when there is no SHM snapshot (so you can see load in the menu / lobby).

## Code

- Draw: `draw_sys` in `overlay/hud/src/render.rs`
- Sample: `overlay/src/sys.rs` calls `set_sys_stats` / `set_sys_procs`
- Settings: `pane_sys` in `overlay/src/settings.rs`

## Rows

Mains: **CPU**, **MEM**, **FPS**, **NET** (percent bars; FPS bar is inverted so low FPS reads hot).

Under CPU and MEM, four apps: **HUD**, **MX Bikes**, **MXB App**, **ReShade**. Missing process shows `—` and a dim bar. Mem sub-bars scale to the heaviest of those processes, not total RAM.

No column picker; only show, opacity, font, bold, snap.

## Do not regress

- Keep drawing with `snap == None` and `on_track == 0`. Hiding it off-track makes the widget useless for diagnosing overlay cost.
- Labels are fixed length-4 (`SYS_PROC_N`). Changing the set means `sys.rs` and the atomics together.

## Change log

- Overlay Systems widget added for overlay vs game vs ReShade load.
- 0.1.8 — Hidden until **Show on overlay**.
- 2026-08-18 — Wiki created. Off-track visibility and process list documented.
