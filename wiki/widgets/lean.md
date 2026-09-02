# Lean

Bike roll, pitch, and steering for the rider the camera is on. Settings subtitle: “Bike roll, pitch, and steering — follows the camera”.

Turn it on with **Show on overlay**. Lives under Cockpit with Dash and Systems.

**Look** in settings: **Figure** (default, rear-view rider) or **Minimal** (numbers).

## Layout

Night-ink 6px plaque, 1px hairline.

**Figure:** white rear-view MX rider that rolls with the bike, an orange skew 32° bug on the hip, a 2px steer hairline under the boots, and a 2px pitch hairline on the right while you are riding.

**Minimal:** huge orange signed lean (`+32°`). Cream pitch degrees under it while riding (`+18°`). Steer hairline and percent under that. No rider, no gyro. Spectate is the lean number only.

- Riding: `m_fRoll` on the ground, shown up to ±90°. In the air, chassis roll often pegs around 50–62° — use that rider’s `m_fLean` so a scrub does not snap. Pitch is `m_fPitch` (plugin `+` is nose down; HUD flips so nose up is `+`), percent of a ±60° clamp for the hairline. Do not snap Euler wraps to 0, and do not cap a real 70°+ lean at 60.
- When **Panel opacity** is under 40% (including 0), steer (and Figure pitch) tracks get a night-ink halo and a light hairline, and the percents sit on night-ink pills. Orange fill is 4px. Minimal numbers get night-ink pills behind the type.
- Spectate / replay: camera rider’s `RaceVehicleData.m_fLean`. Steer and pitch hide — other bikes do not send bar angle or chassis pitch. Minimal shows the lean number only.
- Orange is you (the camera subject). Right lean / right steer are positive. Pitch fill up (Figure) / cream `+°` (Minimal) is nose up.
- Do not infer sit / stand from this. That is [Stance](stance.md).

Starts **hidden**. Restart MX Bikes after this plugin so SHM `Local\MXBOHudV12` loads.

## Code

- Draw: `draw_lean` / `draw_lean_minimal` in `overlay/hud/src/render.rs`
- View: `overlay/hud/src/lean.rs`
- Settings: `pane_lean` in `overlay/src/settings.rs`
- Ini: `lean_style=figure` (default) or `minimal`

## Do not regress

- Keep the widget off until **Show on overlay** is on.
- Default Look is Figure. Minimal is opt-in.
- Riding uses local roll + steer on the ground, including 70°+. When chassis roll is stuck around 50–62°, use that rider’s `m_fLean` so a jump/scrub does not snap. Euler inversions clamp at ±90, not 60.
- Pitch is local `m_fPitch` while riding. Plugin positive is nose down. The HUD flips it so nose up is `+` and nose down is `−`. Spectate has no pitch.
- Spectate follows `focus_race_num` lean on `riders[]`. Do not treat Stance as lean.
- Fresh install: `show_lean = false`.
- Roll / lean / pitch are degrees. A 1° upright lean is not 1 rad.
- Plugin positive roll / steer is left. Plugin positive pitch is nose down. The HUD flips both so the rider, numbers, and bar match the bike (right from behind, nose up is `+`).
- `m_fSteerLock` from EventInit scales the bar. Do not assume 0–1 if lock is present.
- When Panel opacity is under 40%, steer and Figure pitch need a night-ink halo, a light hairline, and percent pills. Do not leave 2px charcoal tracks on the game.
- Minimal: type only. Huge orange signed lean, cream pitch degrees, steer hairline. Not a gyro.

## Change log

- 2026-09-02 — Pitch follows the nose: HUD `+` is nose up, `−` is nose down (plugin sign flipped).
- 2026-09-02 — Lean display goes to ±90°. A 70°+ chassis lean is not capped at 60. Air peg (~50–62) still uses rider lean.
- 2026-09-02 — Minimal Look is numbers: huge orange lean, cream pitch degrees, steer hairline. Gyro dropped.
- 2026-09-02 — Steer / pitch: night-ink halo, light track, and percent pills when panel opacity is under 40%. Orange fill is 4px.
- 2026-09-02 — Pitch: vertical hairline on the right while riding (`m_fPitch`, SHM v12).
- 2026-09-02 — Scrub / jump: chassis roll past 60° no longer snaps to 0; rider lean takes over in the air.
- 2026-09-01 — Figure: rear-view rider, orange 32° bug, steer hairline while riding.
- 2026-09-01 — Added. Camera-follow lean. SHM v11 (`m_fRoll`, `m_fSteer`, `m_fSteerLock`, per-rider `m_fLean`).
