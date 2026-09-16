# Controller

Live local controller on the overlay. DualShock / DualSense HID draws PlayStation glyphs; Steam Xbox mapping and native XInput draw Xbox. Settings subtitle: “Live pad — sticks, triggers, bumpers, and buttons”. Settings tab name is **Controller**. `WidgetId` stays `Gamepad`.

Behind **Experimental widgets** (Settings → Labs). Then turn it on with **Show on overlay**. Lives under Labs, not Cockpit.

This is **not** plugin telemetry. Other riders’ inputs are not available.

## Layout

DualShock 4 and Xbox One pads over the game. **Pad** (Auto / PlayStation / Xbox) picks the silhouette; skin follows platform for now: **PlayStation = dark schematic** (cream outlines, L1/R1/L2/R2), **Xbox = light filled** (cream body, colored ABXY, unlabeled shoulders). The Theme setting is hidden until both skins are user-selectable again. No plaque by default — Panel opacity 0.

- **Idle** is just that pad drawing. Orange tints live inputs (it does not stamp HUD capsules on top).
- **Triggers** (L2/R2 or LT/RT): analog squeeze fills from the curved bottom lip. Past 80% of the pull it reads as a full fill, so a held trigger never leaves a dark sliver at the top.
- **Bumpers** (L1/R1 or LB/RB): shoulder bars light solid orange while held.
- **Sticks**: discs leave their wells with your X/Y. Wells are opaque. An orange ring on the well when the stick is off-center. L3/R3 fills the disc orange.
- **Face / D-pad / system**: Cross/A, D-pad right, etc. fill orange when pressed. Xbox ABXY oranges the disc and keeps the colored letter. The Xbox guide is a light disc with a dark X; pressing it oranges the disc.
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
- XInput / Steam Xbox mapping draws the filled Xbox One pad (A/B/X/Y letters, plus d-pad, LB/RB, LT/RT). Sony HID draws DualShock glyphs (△○×□, L1/R1, L2/R2). Do not put Xbox labels on the DualShock body.
- DualShock keeps its proportions in the widget box. A wide short widget letterboxes the pad; it does not pancake the silhouette. Same aspect-fit for Xbox.
- Art: `gamepad-ds4-dark.png` / `gamepad-ds4-light.png` (1536×1024), `gamepad-xbox-dark.png` / `gamepad-xbox-light.png` (1344×1024). Do not stretch Xbox aspect. Orange tints the drawing's interiors and well rings; it does not draw HUD capsules over the pad.
- Xbox art is traced 1:1 from `overlay/hud/assets/gamepad-xbox-target.png` by `gen_gamepad_xbox.py` (Light) and `gen_gamepad_xbox_dark.py` (Dark schematic); shared tracing lives in `xbox_pad_trace.py`. Regenerate rather than nudging UVs by hand.
- Xbox idle is cream body (`#E4E4E6`), night-ink controls, field-slate (`#303440`) stick caps, and app-colored ABXY (Y yellow, X blue, B red, A green) on night-ink discs. No LT/RT/LB/RB/VIEW/MENU labels. View/Menu are unlabeled dots.
- Fresh install: `gamepad_bg = 0` (no night-ink plaque — just the pad). Panel opacity in settings still brings the plaque back.
- Triggers are analog 0–1 fill (DualShock wings / Xbox horns), snapping to a full fill at 80%. Bumpers are digital on/off shoulder bars.
- A press covers its whole control — no ring of ink around the orange. The Xbox layout radii and boxes are measured off the rendered art by `gen_gamepad_xbox.py`, not off the target, because the traced masks grow when they are smoothed. `xbox_press_covers_the_whole_control` pins this.
- Sticks follow X/Y. Wells stay opaque — do not punch through to the game (including the trough between the well rings). Do not only light a ring without moving the disc.
- Face / D-pad orange fills out to that pad’s outline. Xbox face press is an orange disc on cream that keeps the colored letter. Bumpers fill the rounded shoulder bar, not a square stamp.
- Pulled trigger orange follows the rounded bottom lip (not a flat cut). Analog squeeze is from that lip up. Bumpers light the shoulder bar while held.
- DualShock pressed labels stay 1px cream strokes on orange. Do not thicken them, ink them black, or redraw outlines.
- No pad is the **No controller** pill, not an idle pad.
- Do not poll XInput/HID every overlay frame unless Controller is visible or Stance is shown (or bind listen). Disconnected XInput slots still back off for 2s.
- Sit / stand stays Stance. This widget must not change sitting.
- Other riders’ throttle / brake are not this widget.

## Change log

- 2026-09-16 — Controller skin is platform-locked (PS dark, Xbox light); Theme setting removed temporarily. Four PNG skins remain embedded for a future Theme control.
- 2026-09-16 — A press now covers its whole control (the layout is measured off the rendered art), a trigger held past 80% reads as full, and the crown-to-body outline is traced and low-passed along its length instead of pixel-filtered, so the shoulder seam no longer waves.
- 2026-09-16 — Xbox art is traced 1:1 from the reference render (`gamepad-xbox-target.png`) instead of the old stretched blueprint: real Xbox One outline at its own 1344×1024 aspect, ABXY on night-ink discs, light guide disc with a dark X, unlabeled View/Menu dots, field-slate stick caps. Stick cap radius comes from the layout now, so each pad's art drives it.
- 2026-09-16 — Xbox is a filled Xbox One two-tone (cream body, night-ink controls, colored ABXY, plus d-pad). DualShock drawing and UVs stay as they were.
- 2026-09-04 — Rider-facing name is Controller. Behind Settings → Labs → Experimental widgets. Ini keys stay `show_gamepad` / `gamepad_*`.
- 2026-09-03 — Xbox is its own Series silhouette (LS up-left, d-pad down-left, ABXY, LT/RT). DualShock drawing and UVs stay as they were.
- 2026-09-03 — Trigger orange follows the rounded bottom lip per column, not a flat row. Full pull still fills the wing.
- 2026-09-02 — Pressed labels are dark ink on orange. Bumpers fill the rounded shoulder. Face and D-pad fills reach the outline.
- 2026-09-02 — Stick wells are opaque (no dirt through the trough or ring dashes). Pulled trigger orange follows the DualShock wing outline instead of a stair-stepped cap.
- 2026-09-02 — Added. Glass DualShock/Xbox over the game (no plaque by default). Analog trigger squeeze, bumper lights, sticks leave wells. No controller pill.
