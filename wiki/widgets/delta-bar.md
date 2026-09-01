# Delta Bar

Time vs your best at this point on the lap. Settings subtitle: “Time vs your best at this point on the lap”.

This widget is a regular Cockpit widget. Turn it on with **Show on overlay**.

## Code

- Draw: `draw_delta` in `overlay/hud/src/render.rs`
- Store: `overlay/hud/src/delta.rs` — records `local_track_pos → current_lap_ms` on a decent lap, then compares live
- Persist: `overlay/hud/src/track_pb.rs` — `%LOCALAPPDATA%\Holeshot HUD\track-pbs\<track>.json` keyed by `track_name`. Field `bikes` is one tape per displacement (`250`, `450`, …). Field `used` is unix seconds last ridden/written. Split positions (`p`) are per track.
- Tick: `delta::tick` from `overlay/src/main.rs` (not only from `draw`). Skip when `has_telemetry == 0` (replay leftover). Hitch-hold still ticks so a skipped S/F can commit.
- Settings: `pane_delta` in `overlay/src/settings.rs`

The plugin does **not** expose the in-game ghost. A saved tape from a previous visit is the reference; a faster decent lap replaces it.

## Layout

Hair lockup (approved `.impeccable/mocks/delta-hair.png`): no Δ plaque, no border. Orange **Δ** as a letter, live signed time, a 2px center-zero hairline, **BEST** and **LAST** capping the ends. New installs start with panel opacity 0 (type floats). Riders can still raise **Panel opacity**.

- Green fill left of the center tick + negative time: faster than your recorded best at this `local_track_pos`.
- Red fill right + positive time: slower.
- Hairline saturates at ±2.0 s.
- **BEST** and **LAST** are large ExtraBold captions capping the hairline. After a faster decent lap, LAST is replaced by orange **NEW BEST** and the time for eight seconds, then LAST returns.
- When **Panel opacity** is under 40% (including 0), BEST, LAST, and NEW BEST sit on night-ink pills so they read on the game. A solid panel drops the pills.
- **SET LAP** until you cross S/F to start a flying lap; **REC** while that first lap fills (no saved tape), with **complete a flying lap** in the foot; a time after a tape exists (saved or this session). Orange fill on the hairline tracks coverage while REC. The out-lap does not record. A clock drop at the line (away from the pits) starts that first flying lap even when MX has not published last-lap yet. **Compare to session best** (off by default) uses this visit’s fastest decent lap instead of the saved tape: REC until that first flying lap commits, then the left chip reads **SESSION**. The saved file still updates in the background.

A decent lap: ~20 s–15 min, coverage across most of 0..1. A dab does not throw the tape away. A cut or shortcut (skipping ~100 m of the centerline faster than a bike can ride it) does not become the reference. A jump that is in the air about a second over ~100–200 m of winding centerline still counts. A faster later lap replaces the reference for **this class** (every 250 shares; a 450 is separate). Track rename in MX Bikes is a new file (no track id). **Clear this track** deletes the shared file (every class, delta + sectors).

## Do not regress

- Keep the widget off until **Show on overlay** is on. Experimental widgets does not gate it.
- **Compare to session best** (`delta_session`) is this visit only. Do not load the saved tape as the session reference. Keep recording the all-time file even when this is on.
- Do not treat this as the in-game ghost. We only compare to a lap we recorded.
- Do not record the out-lap. The tape starts when they cross S/F to begin a flying lap: a new last-lap time, a lap-number bump, a pos wrap, or a clock drop away from the pits (`pos >= 0.18`, new clock under 4 s, old clock under 3 min). Do not arm on a lap-clock start near pos 0 (pits often sit there). Do not arm on a 3:20 `_fTime` collapse. Do not require the line to sit at `track_pos` 0 — MX Bikes origin is not always S/F. A reset to pits is another out-lap (clock drop, last-lap unchanged).
- Do not tape leftover replay telemetry (`has_telemetry == 0`). Spectate zeros that flag before `delta::tick`.
- Do not persist a PB under an empty class. Keep it in memory until the 250/450 name arrives, then write that class. A 250 file already on disk stays put.
- A hitch that skips the crossing frame must still commit from the live clock / last recorded time.
- Do not commit a short out-lap as the reference.
- Do not commit a cut. If `local_track_pos` skips ~100 m of centerline faster than ~140 mph, the lap is dirty. A hitch with proportional clock still commits. Do not flag a jump: ~0.9–3 s in the air covering up to ~200 m of winding centerline is a triple, not a cut.
- After S/F, do not compare until the lap clock restarts. The plugin often keeps the out-lap / finished-lap clock at pos ~0; that used to show a fake +16s (and smoothing held it). A wrap at the centerline origin mid-lap (often in S3 when origin ≠ the line) is not S/F — keep the delta moving until a real clock drop / last-lap / lap bump. On the first flying lap there is no tape yet: do not treat that origin wrap as a finish or the REC lap is thrown away.
- Recording must run from the overlay loop (`delta::tick` in `main`), not only from `draw`. Overlay-off still records.
- Do not require `last_lap_ms` to commit. The plugin often zeros it on the crossing; use the live clock drop / pos wrap and the time we just recorded. The inverse also: when last-lap *does* arrive after the clock already restarted (`last_cur` under 8s), still commit. If last-lap is slower than the live clock / tape by more than ~0.4 s (plugin republished the old PB), use the clock. Do not clobber LAST/BEST with that slower last-lap.
- Do not keep bins whose times are a few hundred ms at `pos=1.0` after a collapsed or restarted clock. They fail spline_ok on the next flying lap.
- Plugin `RunTelemetry` `_fTime` is seconds. `currentLapMs` must be `dt * 1000`. Do not treat `dt > 200` as already-milliseconds — that collapses the clock at 3:20 and poisons REC.
- Plugin `local_track_pos == 1.0` is the line, not a wrap. `rem_euclid(1.0)` is 0.0 and poisons the next tape if you record the old clock at pos 0.
- Do not push onto a fresh tape on the same frame as a lap end — that sample still has the finished-lap clock.
- A wrap at S/F (0.99 → 0.01) must start a new tape. Skipping wrap samples without moving `last_pos` freezes recording for the next lap.
- Green is faster; red is slower. No purple on this widget.
- Do not draw the orange Δ as a skew plaque. It is a drawn triangle (bundled Exo 2 has no Delta glyph), same orange as you.
- Do not draw a hairline border around the panel. Opacity 0 is the Hair look; a fill is optional.
- Hairline is 2px, fill grows from the center tick. Do not bring back the fat capsule bar.
- Read the tape by interpolating the nearest filled bins at the fractional index. Do not hold a bin’s time across empty bins, and do not step the live bar from bin to bin.
- Hairline damping is a ~0.4 s time constant, not a per-frame blend. Do not drop back to a 0.12/frame mix.
- Do not share a tape across classes. Key by displacement in the bike short name (`250`, `450`). A Yamaha 250 and a Honda 250 share; a 450 PB must not be the 250 reference.
- Persist is one JSON per `track_name` (no MySQL). Inside it, `bikes` holds one tape per class. Write when a faster decent lap commits, or when you visit a track that already has a file and `used` is missing or older than an hour. Load only the current track. Do not write every frame. Do not create a file just to stamp a date (no PB → no file).
- Shared file with Sectors: v2 `bikes` map (`ms`/`bins`/`s` per class) plus track-level `p` (sector split positions) and `used` (unix seconds). A v1 file (no `bikes`) is adopted into the first class you ride after the update. Model-name keys (`YZ450F`) fold into `450`. **Clear this track** on either pane wipes the whole file.
- While **REC**, the foot must say **complete a flying lap**. Do not leave REC as the only cue.
- After a faster decent lap, LAST becomes orange **NEW BEST** plus the time for eight seconds. Do not keep showing LAST on that hold.
- When Panel opacity is under 40%, BEST and LAST need night-ink pills. Do not leave those captions floating on the game at the Hair default of 0.

## Change log

- 2026-09-01 — Faster live clock / tape beats a slower last-lap the plugin republishes (old PB) or zeros at the line. BEST and LAST follow that lap.
- 2026-09-01 — First flying lap after an untimed out-lap: clock drop at the line starts REC. Foot says **complete a flying lap**.
- 2026-09-01 — Last-lap after a restarted or collapsed clock still saves the tape. Collapsed `_fTime` at 3:20 used to leave REC up while standings already had times.
- 2026-09-01 — A jump (~1 s in the air over ~100–200 m of centerline) still commits. That used to dump REC at the line.
- 2026-09-01 — Settings **Compare to session best**. This visit’s fastest decent lap; the saved tape still updates.
- 2026-09-01 — First flying lap: wrapping origin mid-lap does not throw away REC. The tape saves when the real line is crossed.
- 2026-09-01 — Sectors left Labs too. Both are regular Cockpit widgets.
- 2026-09-01 — Left Labs. Regular Cockpit widget. Replay leftover is not taped. Pits near S/F do not start the first flying lap. Empty bike class holds a PB until the name arrives. Hitch across S/F still commits.
- 2026-08-30 — Tape is per class (`250` / `450`) in the track JSON. Yamaha 250 and Honda 250 share. A 450 PB is not the 250 reference. Old v1 files go to the first class you ride.
- 2026-08-28 — Website demo lists Delta Bar under **Experimental**. WASM turns the labs flag on only while that widget is selected.
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
