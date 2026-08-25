# Sectors

Your last split times and delta vs personal best. Settings subtitle: “Split times and delta vs your best”.

This widget is **behind a feature flag**. Debug `cargo run` unlocks the Sectors tab. Release (`build.bat`) keeps it hidden until **Sector times (experimental)** is on (Settings tab, Labs). `show_sector` alone is not enough in release.

## Code

- Draw: `draw_sector` in `overlay/hud/src/render.rs`
- Plugin: `RunSplit` / `RaceSplit` → `PluginState::recordSector` in `src/state.cpp`
- SHM: `sectorCur` / `sectorLastLap` / `sectorBest` / `sectorDelta` / `sectorDeltaValid` (version 8)
- Settings: `pane_sector` in `overlay/src/settings.rs`
- Flag: `HudConfig::feature_sector` (`feature_sector=1` in Holeshot-HUD.ini)

MX Bikes reports two mid-lap splits (`m_aiSplit[2]`). S1 and S2 come from those callbacks. S3 is lap time minus S1 minus S2 when `RunLap` / `RaceLap` fires.

Split index: `0` or first `1` is S1; a later `1` or `2` is S2. That covers 0-based and 1-based plugin values.

## Rows

Three rows: **S1**, **S2**, **S3**.

- Time: current lap if that sector is done, else last lap (dim). `--` if neither exists.
- Delta: vs your previous best for that sector. `--` until there is a comparison.
- Purple: new or tied personal best this lap. Red: slower. Last completed sector gets a faint orange flash.

No column picker; only show, opacity, font, bold, snap.

## Do not regress

- Keep the widget fully hidden in **release** when `feature_sector` is off, even if `show_sector` is on in the ini. Debug builds unlock the tab without that key.
- Do not draw off-track unless settings layout boxes are up (same as other race widgets).
- Do not treat a missing S3 as a live estimate; wait for the lap callback.
- `RunLap` and `RaceLap` can both fire for you. Finish S3 once per `lapNum` so a following split is not folded into last-lap.
- SHM version must stay in lockstep between `overlay/hud/src/snapshot.rs` and `src/shm/mxbo_shm.h`.

## Change log

- 2026-08-20 — Experimental sector times widget. Flagged off in release. Plugin now publishes `RunSplit` / `RaceSplit`.
- 2026-08-20 — Debug `cargo run` unlocks the Sectors tab; `build.bat` release still requires the Labs toggle.
