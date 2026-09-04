# Controller

Live local controller on the overlay. DualShock / DualSense HID draws PlayStation glyphs; Steam Xbox mapping and native XInput draw Xbox. Settings subtitle: “Live pad — sticks, triggers, bumpers, and buttons”. Settings tab name is **Controller**. `WidgetId` stays `Gamepad`.

Behind **Experimental widgets** (Settings → Labs). Then turn it on with **Show on overlay**. Lives under Labs, not Cockpit.

This is **not** plugin telemetry. Other riders’ inputs are not available.

## Layout

Night-ink pad drawing over the game (same family as Lean’s rider). No plaque by default — Panel opacity 0. DualShock HID draws the DualShock 4. Steam Xbox / XInput draws an Xbox Series pad (asymmetric sticks, ABXY, LT/RT tombstones) — not a DualShock with Xbox labels.

- **Idle** is just that pad drawing. Orange tints the drawing on live inputs (it does not stamp HUD capsules on top).
- **Triggers** (L2/R2 or LT/RT): analog squeeze fills from the curved bottom lip.
- **Bumpers** (L1/R1 or LB/RB): shoulder bars light solid orange while held.
- **Sticks**: discs leave their wells with your X/Y. Wells are opaque. An orange ring on the well when the stick is off-center. L3/R3 fills the disc orange.
- **Face / D-pad / system**: Cross/A, D-pad right, etc. fill orange when pressed.
- **No pad** (keyboard, disconnected): **No controller** on a small night-ink pill. It does not draw an empty pad.

Starts **hidden**.

## Code

- Draw: `draw_gamepad` in `overlay/hud/src/render.rs`
- State: `overlay/hud/src/gamepad.rs`
- Poll: `overlay/src/stance.rs` (`Tracker::tick` `gamepad_on`). Reuses XInput / DualSense / DS4 HID from Stance. Pad scan runs while Controller is visible (`gamepad_visible`) **or** Stance is shown (or while listening for a bind).
- Settings: `pane_gamepad` in `overlay/src/settings.rs`

## Do not regress

- Keep the widget off until Labs → **Experimental widgets** and **Show on overlay** are on. `show_gamepad` alone must not draw.
- Fresh install: `show_gamepad = false`, `experimental = false`.
- XInput / Steam Xbox mapping draws the Xbox Series pad (A/B/X/Y, LB/RB, LT/RT). Sony HID draws DualShock glyphs (△○×□, L1/R1, L2/R2). Do not put Xbox labels on the DualShock body.
- DualShock keeps its proportions in the widget box. A wide short widget letterboxes the pad; it does not pancake the silhouette. Same aspect-fit for Xbox.
- DualShock art is `overlay/hud/assets/gamepad-ds4.png`. Xbox art is `overlay/hud/assets/gamepad-xbox.png`. Do not rewrite DualShock UVs or `gamepad-ds4.png` to ship Xbox. Orange tints the drawing's interiors and well rings; it does not draw HUD capsules over the pad.
- Fresh install: `gamepad_bg = 0` (no night-ink plaque — just the pad). Panel opacity in settings still brings the plaque back.
- Triggers are analog 0–1 fill (DualShock wings / Xbox tombstones). Bumpers are digital on/off shoulder bars.
- Sticks follow X/Y. Wells stay opaque — do not punch through to the game (including the trough between the well rings). Do not only light a ring without moving the disc.
- Face / D-pad orange fills out to that pad’s outline. Bumpers fill the rounded shoulder bar, not a square stamp.
- Pulled trigger orange follows the rounded bottom lip (not a flat cut). Analog squeeze is from that lip up. Bumpers light the shoulder bar while held.
- Pressed labels and glyphs stay that pad’s 1px cream strokes on orange. Do not thicken them, ink them black, or redraw outlines.
- No pad is the **No controller** pill, not an idle pad.
- Do not poll XInput/HID every overlay frame unless Controller is visible or Stance is shown (or bind listen). Disconnected XInput slots still back off for 2s.
- Sit / stand stays Stance. This widget must not change sitting.
- Other riders’ throttle / brake are not this widget.

## Change log

- 2026-09-04 — Rider-facing name is Controller. Behind Settings → Labs → Experimental widgets. Ini keys stay `show_gamepad` / `gamepad_*`.
- 2026-09-03 — Xbox is its own Series silhouette (LS up-left, d-pad down-left, ABXY, LT/RT). DualShock drawing and UVs stay as they were.
- 2026-09-03 — Trigger orange follows the rounded bottom lip per column, not a flat row. Full pull still fills the wing.
- 2026-09-02 — Pressed labels are dark ink on orange. Bumpers fill the rounded shoulder. Face and D-pad fills reach the outline.
- 2026-09-02 — Stick wells are opaque (no dirt through the trough or ring dashes). Pulled trigger orange follows the DualShock wing outline instead of a stair-stepped cap.
- 2026-09-02 — Added. Glass DualShock/Xbox over the game (no plaque by default). Analog trigger squeeze, bumper lights, sticks leave wells. No controller pill.
