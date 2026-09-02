# Systems

Host PC meters: CPU, memory, FPS, ping, GPU, plus per-app CPU/mem/GPU. Settings subtitle: “CPU, memory, FPS, ping, GPU, and per-app load”.

Same as every other widget: it **hides** in the menu / lobby / garage and when MX Bikes is not running. Hold Ctrl to place the box anyway.

## Code

- Draw: `draw_sys` in `overlay/hud/src/render.rs`
- Sample: `overlay/src/sys.rs` calls `set_sys_stats` / `set_sys_procs`. GPU load is `overlay/src/gpu.rs`. Ping is `overlay/src/ping.rs` (background ICMP).
- Settings: `pane_sys` in `overlay/src/settings.rs`

## Layout

Twin columns inside a night-ink 6px plaque with a 1px hairline frame and a hairline split. Content stops ~10–16px short of that split so italic CPU percents and MEM labels do not kiss the line.

- **Left:** CPU — huge percent, gold heat track, process CPU rows
- **Right:** MEM — huge percent, gold heat track, process mem rows
- **Footer:** FPS left (number only, no bar), ping under it, GPU right with the same apps. Same hairline split.

Default process apps: **HUD**, **MX Bikes**, **MXB App**, **ReShade**, **OBS**. Settings can hide any of those, add more (Discord, Steam, NVIDIA, Afterburner, RTSS, Medal, Spotify, Game Bar), or pick an `.exe`. The overlay paints at most 8 shown apps. Missing process shows `—` and a dim bar. Mem sub-bars scale to the heaviest of those processes, not total RAM.

Watching is by running process **basename** (`obs64.exe` / `obs32.exe`), not the install folder. Steam, portable, and Program Files all count. Browse stores that filename only, so moving the app later still works.

Heat is gold (`#FAB430`), red at ≥90. FPS has no heat track. Ping is ICMP ms. No green bars. No Holeshot orange (that color is you).

No column picker. Show, opacity, font, bold, snap, plus the app list (show/hide, add preset, browse `.exe`, remove extras).

Default box matches a compact overlay size (`0.086 × 0.25`). Existing layouts in the INI keep their size.

INI: `sys_apps=` under `[Sys]`. Missing key keeps the five defaults. Built-ins cannot be removed.

## Ping

ICMP round-trip in ms, sampled on a background thread (`holeshot-ping`, ~2s). Prefers an established IPv4 TCP peer of `mxbikes.exe` (public address over LAN), else `1.1.1.1`. Blocked ICMP or no reply is `—`. Do not ping on the overlay thread.

## GPU

One percent: Task Manager's GPU graph — 3D / Graphics on each card, Compute only if that card has no 3D. NVIDIA via NVML (`nvml.dll`), AMD via ADL (`atiadlxx.dll`, PM log then Overdrive N/5), and Windows `\GPU Engine(*)\Utilization Percentage` so either card still reads when the vendor DLL is missing. Copy / video engines are ignored. Per-process GPU Engine instances are summed on the same engine. App rows use that pid's 3D / Graphics, else Compute. ReShade as a DLL inside MX Bikes has no separate GPU figure (`—`).

## Do not regress

- Do not draw with `snap == None` or an empty session. Same hide rule as race widgets. Layout boxes (`settings_hint`) still force a draw.
- Labels come from the watch list (`sys_apps`). Overlay paints at most `SYS_PROC_MAX` (8) shown apps. Changing who is sampled is `sys.rs` plus that list; GPU rows use the same set. ReShade as a DLL is `—` for GPU. Defaults include OBS. Match running processes by exe basename, never an install path.
- Do not paint load with ahead-green or Holeshot orange.
- FPS is the game’s Draw publish rate, not the overlay loop. SHM `seq` is a seqlock (advances by 2 per publish); divide by 2. Do not mix seq FPS with overlay frame counting while a snapshot is live.
- Do not Toolhelp-snapshot MX Bikes modules every sample. That stalls the game. ReShade DLL size is cached (~30s). CPU/mem/GPU sampling only runs while this widget is shown in a session.
- Do not sleep the overlay thread to prime PDH. The first GPU sample after show may be 0 until the next 500ms tick.
- Do not ICMP on the overlay thread. Ping lives on `holeshot-ping`; ICMP blocked or offline shows `—`.
- Do not take Compute over 3D / Graphics on the same card. Task Manager's GPU graph is 3D; Compute can sit at 100% with hardware-accelerated GPU scheduling.

## Change log

- 2026-09-02 — Columns leave a gutter at the split so left percents are not on the hairline.
- 2026-09-02 — OBS is on the default list. Apps are matched by `.exe` name wherever they are installed.
- 2026-09-02 — Settings can show/hide each app, add common MX Bikes tools (OBS, Discord, …), or pick an `.exe`. Overlay still caps at 8 rows. Built-ins stay on the list.
- 2026-09-02 — FPS is a number only. Ping sits under it (ICMP to MX Bikes' TCP peer, else 1.1.1.1). `—` when ICMP is blocked.
- 2026-09-02 — GPU percent is 3D / Graphics like Task Manager, not a Compute engine stuck at 100% (HAGS). App rows follow the same engine.
- 2026-09-02 — GPU lists the same four apps as CPU and memory. Per-pid GPU Engine counters; ReShade as a DLL shows `—`. Default height `0.25` so the footer rows fit. Existing INI sizes stay.
- 2026-09-02 — Footer is GPU instead of network. NVIDIA NVML, AMD ADL, and Windows GPU Engine counters so either card works.
- 2026-08-31 — Stopped walking `mxbikes.exe` modules every 250ms (that can freeze the game). Meters sample only while Systems is on; ReShade size is cached for 30s.
- 2026-08-30 — FPS was bouncing (~38 overlay loop vs ~140 seqlock units) because `note_fps` mixed both meters and treated seqlock `seq` as a frame counter (it steps by 2 per Draw). Game FPS is now `seq_delta / 2 / dt`; overlay FPS only when there is no snapshot.
- 2026-08-29 — Twin Columns: CPU | MEM boards, process rows under each, FPS/NET footer. Gold heat instead of a green task-manager list. Default size `0.086 × 0.186`.
- 2026-08-27 — A blank HUD with Systems off still follows the shared hide rule; the overlay plaque (not this widget) explains plugin-missing / widgets-off.
- 2026-08-26 — Hides without a session, same as the rest of the HUD. Menu / lobby no longer show CPU meters.
- Overlay Systems widget added for overlay vs game vs ReShade load.
- 0.1.8 — Hidden until **Show on overlay**.
- 2026-08-18 — Wiki created. Off-track visibility and process list documented.
