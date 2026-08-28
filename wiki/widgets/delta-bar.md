# Delta Bar

Time vs your best at this point on the lap. Settings subtitle: “Time vs your best at this point on the lap”.

This widget is **behind the Labs flag**. Settings → Labs → **Experimental widgets**. `show_delta` alone is not enough.

## Code

- Draw: `draw_delta` in `overlay/hud/src/render.rs`
- Store: `overlay/hud/src/delta.rs` — records `local_track_pos → current_lap_ms` on a decent lap, then compares live
- Persist: `overlay/hud/src/track_pb.rs` — `%LOCALAPPDATA%\Holeshot HUD\track-pbs\<track>.json` keyed by `track_name`. Field `used` is unix seconds last ridden/written.
- Tick: `delta::tick` from `overlay/src/main.rs` (not only from `draw`)
- Settings: `pane_delta` in `overlay/src/settings.rs`
- Flag: `HudConfig::experimental` (`experimental=1` in Holeshot-HUD.ini)

The plugin does **not** expose the in-game ghost. A saved tape from a previous visit is the reference; a faster decent lap replaces it.

## Layout

Hair lockup (approved `.impeccable/mocks/delta-hair.png`): no Δ plaque, no border. Orange **Δ** as a letter, live signed time, a 2px center-zero hairline, **BEST** and **LAST** capping the ends. New installs start with panel opacity 0 (type floats). Riders can still raise **Panel opacity**.

- Green fill left of the center tick + negative time: faster than your recorded best at this `local_track_pos`.
- Red fill right + positive time: slower.
- Hairline saturates at ±2.0 s.
- **BEST** and **LAST** are large ExtraBold captions capping the hairline. After a faster decent lap, LAST is replaced by orange **NEW BEST** and the time for eight seconds, then LAST returns.
- When **Panel opacity** is under 40% (including 0), BEST, LAST, and NEW BEST sit on night-ink pills so they read on the game. A solid panel drops the pills.
- **SET LAP** until you cross S/F to start a flying lap; **REC** while that first lap fills (no saved tape), with **complete two full laps** in the foot; a time after a tape exists (saved or this session). Orange fill on the hairline tracks coverage while REC. The out-lap does not record.

A decent lap: ~20 s–15 min, coverage across most of 0..1. A dab does not throw the tape away. A faster later lap replaces the reference and the file. Track rename in MX Bikes is a new file (no track id). **Clear this track** deletes the shared file (delta + sectors).

## Do not regress

- Keep the widget fully hidden when **Experimental widgets** is off, even if `show_delta` is on in the ini.
- Do not treat this as the in-game ghost. We only compare to a lap we recorded.
- Do not record the out-lap. The tape starts when they cross S/F to begin a flying lap: a new last-lap time, a lap-number bump, a pos wrap, or a lap clock start near the line. Do not require the line to sit at `track_pos` 0 — MX Bikes origin is not always S/F. A reset to pits is another out-lap (clock drop, last-lap unchanged).
- Do not commit a short out-lap as the reference.
- Recording must run from the overlay loop (`delta::tick` in `main`), not only from `draw`. Overlay-off still records.
- Do not require `last_lap_ms` to commit. The plugin often zeros it on the crossing; use the live clock drop / pos wrap and the time we just recorded.
- Plugin `local_track_pos == 1.0` is the line, not a wrap. `rem_euclid(1.0)` is 0.0 and poisons the next tape if you record the old clock at pos 0.
- After S/F, do not compare until the lap clock restarts. The plugin often keeps the out-lap / finished-lap clock at pos ~0; that used to show a fake +16s (and smoothing held it).
- Do not push onto a fresh tape on the same frame as a lap end — that sample still has the finished-lap clock.
- A wrap at S/F (0.99 → 0.01) must start a new tape. Skipping wrap samples without moving `last_pos` freezes recording for the next lap.
- Green is faster; red is slower. No purple on this widget.
- Do not draw the orange Δ as a skew plaque. It is a drawn triangle (bundled Exo 2 has no Delta glyph), same orange as you.
- Do not draw a hairline border around the panel. Opacity 0 is the Hair look; a fill is optional.
- Hairline is 2px, fill grows from the center tick. Do not bring back the fat capsule bar.
- Read the tape by interpolating the nearest filled bins at the fractional index. Do not hold a bin’s time across empty bins, and do not step the live bar from bin to bin.
- Hairline damping is a ~0.4 s time constant, not a per-frame blend. Do not drop back to a 0.12/frame mix.
- Persist is one JSON per `track_name` (no MySQL). Write when a faster decent lap commits, or when you visit a track that already has a file and `used` is missing or older than an hour. Load only the current track. Do not write every frame. Do not create a file just to stamp a date (no PB → no file).
- Shared file with Sectors: `ms`/`bins` plus `s` (sector durations) and `used` (unix seconds). **Clear this track** on either pane wipes both.
- While **REC**, the foot must say **complete two full laps**. Do not leave REC as the only cue.
- After a faster decent lap, LAST becomes orange **NEW BEST** plus the time for eight seconds. Do not keep showing LAST on that hold.
- When Panel opacity is under 40%, BEST and LAST need night-ink pills. Do not leave those captions floating on the game at the Hair default of 0.

## Change log

- 2026-08-28 — Track JSON stores `used` (unix seconds). Stamped on PB write and when you visit a track that already has a file (at most hourly). Old files without it still load. No empty files for tracks with no PB.
- 2026-08-28 — BEST / LAST type is larger. A new PB replaces LAST with orange **NEW BEST** and the time for eight seconds.
- 2026-08-28 — While **REC**, the foot says **complete two full laps** (night-ink pill when opacity is under 40%).
- 2026-08-28 — Hairline lerps across empty bins (no flat hold) and damps on a 0.4 s time constant so plugin pos noise does not step the line.
- 2026-08-28 — Flying lap starts when last-lap time appears at S/F, even if that line is not at track_pos 0. Requiring a wrap at 0 left the bar stuck after real laps.
- 2026-08-28 — Out-lap does not fill the tape. Recording starts when you cross S/F (or the lap clock starts near the line). A reset to pits waits for the next cross.
- 2026-08-28 — BEST / LAST sit on night-ink pills when Panel opacity is under 40%, so they read at the Hair default of 0.
- 2026-08-28 — First flying lap after a saved tape no longer opens at +16s. Crossing used the out-lap clock at pos 0; wait for the new lap timer.
- 2026-08-27 — Hair lockup: Δ as a letter, 2px center-zero line, BEST/LAST on the ends. No plaque. New installs start with panel opacity 0.
- 2026-08-27 — Show BEST under the bar whenever a tape exists. Show LAST after you finish a lap (held when the plugin zeros last-lap ms).
- 2026-08-27 — Persist best tape per track in AppData. Coming back loads it; a faster lap updates the file.
- 2026-08-27 — `pos=1.0` no longer wraps to 0 while the clock is the old lap. That froze the next tape on **SET LAP**. REC while filling; dabs do not veto.
- 2026-08-27 — Commit the tape on live-clock drop / pos wrap. Plugin often zeros last-lap ms at the line; that used to leave SET LAP on the next lap.
- 2026-08-27 — Experimental delta bar vs a recorded personal-best tape at `local_track_pos`. Not the in-game ghost.
