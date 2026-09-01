# Sectors

Your last split times and delta vs. **your best at this point in the sector** (same saved tape as Delta Bar). **Lap log** (default on) puts LAST / -2 / … under the live strip; **Laps back** is 1–5 (default 3). Settings subtitle: “Split times vs your best, plus a lap log under the strip”.

This widget is a regular Cockpit widget. Turn it on with **Show on overlay**. Same tape as Delta Bar.

## Code

- Draw: `draw_sector` in `overlay/hud/src/render.rs`
- Live/freeze: `overlay/hud/src/sector.rs` — current sector ticks; completed splits freeze; LAST / -2 / -3 ring
- Persist: `overlay/hud/src/track_pb.rs` — same JSON as Delta Bar, `bikes.<class>.s` is S1/S2/S3 durations for that displacement (`250`, `450`, …), `p` is S1/S2 split positions (track-level), `used` is unix seconds last ridden/written
- Plugin: `RunSplit` / `RaceSplit` → `PluginState::recordSector` in `src/state.cpp`
- SHM: `sectorCur` / `sectorLastLap` / `sectorBest` / `sectorDelta` / `sectorDeltaValid` (version 8)
- Settings: `pane_sector` in `overlay/src/settings.rs`
- Flag: none. `HudConfig::experimental` still round-trips in the ini for older builds.

MX Bikes reports two mid-lap splits (`m_aiSplit[2]`). S1 and S2 come from those callbacks. S3 is lap time minus S1 minus S2 when `RunLap` / `RaceLap` fires.

Split index: `0` or first `1` is S1; a later `1` or `2` is S2. That covers 0-based and 1-based plugin values.

## Layout

Three columns: **S1**, **S2**, **S3**. The **current** sector is the **hero** (~56% width) with an orange skew **S#** plaque and a large delta. The other two stay quiet. When the box is tall enough, **Lap log** (default on) draws **LAST**, **-2**, … under a hairline, same columns, up to **Laps back** (1–5, default 3). You-row gold is only on the fastest lap in that log. History times tint green/red vs the same comparison as the live strip. Short boxes stay live-only. Off hides the log; splits still record.

- Current sector: live elapsed time and live delta vs your saved tape **at this `local_track_pos`** (S2/S3 ignore time already lost or gained in earlier sectors). S1 until the first split, then S2, then S3 until the line. **Live sector** (default on) in Settings. Off: `--` until that split completes.
- Leave a sector: that cell shrinks and **freezes** the delta from the split line. Do not keep ticking it.
- Not yet reached this lap: `--`.
- After you cross: S3 stays hero (frozen last lap) until the next lap clock runs, then S1 is hero again.
- Split times at the bottom of each cell sit on night-ink pills (same chip as Delta Bar BEST / LAST).
- Green: faster. Red: slower. Dim `--` until there is a comparison. Caption: **vs. your best**.
- No purple on this widget (standings still uses violet for session-best lap).

No column picker; show, **Live sector**, **Compare to session best**, **Lap log**, **Laps back**, opacity, font, bold, snap. **Clear this track** deletes the shared PB file (every class, sectors + delta tape).

## Do not regress

- Keep the widget off until **Show on overlay** is on. Experimental widgets does not gate it.
- **Compare to session best** (`sector_session`) is this visit’s fastest splits, not the saved tape. Independent of Delta Bar’s toggle. Do not use plugin sector delta in that mode.
- Do not draw off-track unless settings layout boxes are up (same as other race widgets).
- Hero is the sector you are in, not last completed.
- Live delta is time vs location in **this** sector (tape at `local_track_pos` minus tape at the sector start). Do not compare live elapsed to the full sector duration — that always looks too fast mid-sector. A wrap at the centerline origin mid-sector (often S3 when origin ≠ the line) is not S/F — keep the live cell ticking.
- Do not require S2's split to sit after S1 in raw `track_pos`. Origin wrap can put S2's end before S1's end on the 0..1 line.
- **Live sector** off still records and freezes on leave; the current cell stays `--` until the split. Default on (`sector_live=1`).
- Freeze on leave. Do not write the PB file every live frame — only when a frozen duration is faster than saved **for this class**, or when visiting a track whose file is missing/stale `used` (at most hourly). Do not create a file just to stamp a date.
- LAST / -2 / … are completed laps, newest first, up to five. Push when S3 freezes. Do not shift the same lap twice. Empty first laps stay `--` and hide the underboard until LAST exists. You-row gold is only on the fastest complete lap in the log, not always LAST.
- **Lap log** (`sector_hist`, default on) only hides the underboard. Keep recording. **Laps back** (`sector_hist_laps`) is 1–5, default 3.
- Short widgets stay live-only. Do not squash the live delta to fit history. If the box is shorter than the chosen lap count, draw as many rows as fit.
- History green/red is vs the same tape as the live strip (saved or session). No purple. Orange is still the sector you are in.
- Do not compare a 250 split to a 450 saved best. Yamaha 250 and Honda 250 share. Same per-class tape as Delta Bar.
- Do not treat a missing S3 as a live estimate while the lap is open. When the lap completes, fill S3 from lap time minus S1/S2 (durations or cumulative). If the plugin left S3 empty, the overlay infers it from last lap time so the S3 cell can still go orange.
- After `finishLapSectors`, keep last-lap times and deltas on screen until the next lap clock is running. Completing S1 of the new lap overwrites S1 and S2/S3 go pending until those splits.
- `RunLap` and `RaceLap` can both fire for you. Finish S3 once per `lapNum` so a following split is not folded into last-lap.
- `RunSplit` and `RaceSplit` both fire for the same split. Record it once. A second write after the session best is updated stores `0.000` on a faster sector. Overlay freeze vs the **old** saved best so a new PB is negative, not `0.000`.
- SHM version must stay in lockstep between `overlay/hud/src/snapshot.rs` and `src/shm/mxbo_shm.h`.
- Split times at the bottom sit on night-ink pills. Do not leave those captions floating on the game.

## Change log

- 2026-09-01 — Lap-log you-row gold is only on the fastest lap in the log, not always LAST.
- 2026-09-01 — **Lap log** (default on) and **Laps back** (1–5, default 3). Underboard still LAST / -2 / … vs your best. A short box stays live-only.
- 2026-09-01 — Underboard: LAST / -2 / -3 completed split times sit under the live strip. Green/red vs your best. Factory box is taller; a short box stays live-only.
- 2026-09-01 — Settings **Compare to session best**. This visit’s fastest splits; independent of Delta Bar’s toggle.
- 2026-09-01 — Live S3 keeps ticking through a centerline origin wrap (same tape as Delta Bar). S2/S3 span wraps at pos 0; do not require split positions to be in increasing order.
- 2026-09-01 — Left Labs. Regular Cockpit widget. Same tape as Delta Bar.
- 2026-09-01 — Labs toggle is Sectors only. Delta Bar is a regular Cockpit widget.
- 2026-08-30 — Sector PBs are per class (`250` / `450`) in the shared track JSON. Yamaha 250 and Honda 250 share. Switching 450 → 250 starts a fresh comparison.
- 2026-08-28 — Demo lap clock follows track position so S2/S3 live time keeps ticking (clock was stuck at ~18s, so S2 elapsed went to 0).
- 2026-08-28 — Website demo lists Sectors under **Experimental**. WASM turns the labs flag on only while that widget is selected.
- 2026-08-28 — Shared track JSON stores `used` (unix seconds). Stamped on sector PB write and when you visit a track that already has a file (at most hourly). Old files without it still load.
- 2026-08-28 — Split times at the bottom sit on night-ink pills so they read when the panel is thin.
- 2026-08-27 — Settings **Live sector**. On is the location delta while you are in the sector. Off only shows a time after you leave that split.
- 2026-08-27 — Live sector delta is time vs location in that sector (same tape as Delta Bar). Mid-S1 no longer compares elapsed to the full S1 time.
- 2026-08-27 — Live delta in the current sector; freeze when you leave. Wide orange cell is the current sector. Persist S1–S3 durations per track with the delta tape.
- 2026-08-20 — Experimental sector times widget. Flagged off in release. Plugin now publishes `RunSplit` / `RaceSplit`.
- 2026-08-20 — Debug `cargo run` unlocks the Sectors tab; `build.bat` release still requires the Labs toggle.
- 2026-08-27 — Hero cell on the last completed split (orange S# plaque, large delta). Green is faster / PB; red is slower. No purple on Sectors.
- 2026-08-27 — Splits show when they complete (no live count). S3 is filled from lap time when you cross the line.
- 2026-08-27 — Completing a lap fills S3 and makes it the orange cell. Overlay infers S3 from last lap if the plugin omitted it.
- 2026-08-27 — Ignore the second of RunSplit/RaceSplit so a new PB still shows a negative delta instead of 0.000.
- 2026-08-27 — Labs toggle also unlocks Delta Bar.
- 2026-08-26 — Stance left Labs. This toggle is Sectors only. Debug no longer auto-unlocks.
