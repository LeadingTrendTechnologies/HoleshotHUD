# Controller

Live local controller on the overlay. DualShock / DualSense HID draws PlayStation glyphs; Steam Xbox mapping and native XInput draw Xbox. Settings subtitle: “Live pad — sticks, triggers, bumpers, and buttons”. Settings tab name is **Controller**. `WidgetId` stays `Gamepad`.

This widget is a regular Cockpit widget. Turn it on with **Show on overlay**. Lives under Cockpit with Dash and Systems.

This is **not** plugin telemetry. Other riders’ inputs are not available.

## Layout

DualShock 4 and Xbox One pads over the game. **Pad** (Auto / PlayStation / Xbox) picks the silhouette. **Theme** (Light / Dark, default Light) picks the skin for both pads. Xbox **Light** is the filled pad (cream body, colored ABXY, unlabeled shoulders); Xbox **Dark** is the DualShock-style schematic (charcoal body, thin grey outlines, outlined ABXY). PlayStation **Dark** is the charcoal schematic; PlayStation **Light** is a cream body with black outlines, black d-pad / face / touchpad panels, slate d-pad keys and shoulders with cream outlines, and cream symbols (`gen_gamepad_ds4_light.py` recolors the dark art, so every control stays put). No plaque by default — Panel opacity 0.

- **Idle** is just that pad drawing. Primary color tints live inputs (it does not stamp HUD capsules on top).
- **Triggers** (L2/R2 or LT/RT): analog squeeze fills from the curved bottom lip. Past 80% of the pull it reads as a full fill, so a held trigger never leaves a dark sliver at the top.
- **Bumpers** (L1/R1 or LB/RB): shoulder bars light solid primary while held.
- **Sticks**: discs leave their wells with your X/Y. Wells are opaque. A primary ring on the well when the stick is off-center. L3/R3 fills the disc in primary.
- **Face / D-pad / system**: Cross/A, D-pad right, etc. fill in primary when pressed. Xbox ABXY fills the disc in primary and keeps the colored letter (Dark: the outlined letter turns cream). The Xbox Light guide is a light disc with a dark X; pressing it fills the disc in primary.
- **No pad** (keyboard, disconnected): **No controller** on a small night-ink pill. It does not draw an empty pad.

Starts **hidden**.

## Code

- Draw: `draw_gamepad` in `overlay/hud/src/render.rs`
- State: `overlay/hud/src/gamepad.rs`
- Poll: `overlay/src/stance.rs` (`Tracker::tick` `gamepad_on`). Reuses XInput / DualSense / DS4 HID from Stance. Pad scan runs while Controller is visible (`gamepad_visible`) **or** Stance is shown (or while listening for a bind).
- Settings: `pane_gamepad` in `overlay/src/settings.rs`

## Do not regress

- Keep the widget off until **Show on overlay** is on. Experimental features does not gate it. `show_gamepad` alone draws.
- Fresh install: `show_gamepad = false`, `experimental = false` (Tracks still Labs).
- XInput / Steam Xbox mapping draws the filled Xbox One pad (A/B/X/Y letters, plus d-pad, LB/RB, LT/RT). Sony HID draws DualShock glyphs (△○×□, SHARE/OPTIONS, PS logo). The DualShock shoulders (L1/R1, L2/R2) are unlabeled. Do not put Xbox labels on the DualShock body.
- DualShock keeps its proportions in the widget box. A wide short widget letterboxes the pad; it does not pancake the silhouette. Same aspect-fit for Xbox.
- Art: `gamepad-ds4-dark.png` / `gamepad-ds4-light.png` (1536×1024), `gamepad-xbox-dark.png` / `gamepad-xbox-light.png` (1344×1024). Do not stretch Xbox aspect. Primary color tints the drawing's interiors and well rings; it does not draw HUD capsules over the pad.
- DualShock dark art (`gamepad-ds4-dark.png`) is generated from `gamepad-ds4.png` by `gen_gamepad_ds4_dark.py` (fills pinholes, anti-aliases the silhouette, erases the L1/L2/R1/R2 labels, keeps every control pixel in place). Regenerate rather than hand-edit.
- Theme switches both pads (`gamepad_theme_switches_playstation`), and the Theme row opens for every Pad setting (`gamepad_theme_row_opens_for_every_pad`). `gamepad_theme` is saved per preset; unknown values load as Light.
- Both DualShock skins share `ds4_gamepad_layout` and are pressed in art space. The light DualShock floods only its slate controls (luminance 30–78), so a press stops at the black panels; face presses keep cream symbols. `ds4_light_press_fills_stay_inside_their_outlines` pins R2 to the same trigger as the dark pad, Cross keeping its symbol, and d-pad up lighting only its key.
- The shown skins (DualShock dark, Xbox light, Xbox dark) are area-averaged down to widget size (`area_downscale`) — a single resample aliases thin outlines into dashes.
- The shown skins press in art space and are area-averaged down (`pad_pressed_frame`), only over the changed regions; the idle frame must use the same downscale or pressed regions seam. Do not shade their presses after the blit: a per-screen-pixel fill stair-steps the edges and shreds the glyphs. DualShock face symbols and Xbox dark letters read as cream on the fill; Xbox light ABXY keep their colored letter; outlines keep their drawn color. The Xbox light guide press turns the light disc primary and keeps the dark X. Only the hidden DualShock light skin keeps the dest-space path (`shade_presses`).
- Xbox dark body is ink too, so its d-pad arm press is clipped to the cross outline and View/Menu flood inside their rings; a plain rect press would light charcoal outside the control. LB/RB light the same band as on the light Xbox pad (no bumper dividers). `xbox_dark_press_fills_stay_inside_their_outlines` pins this.
- Xbox light art is traced 1:1 from `overlay/hud/assets/gamepad-xbox-target.png` by `gen_gamepad_xbox.py`; shared tracing lives in `xbox_pad_trace.py`. Xbox dark (`gamepad-xbox-dark.png`) is generated from `gamepad-xbox-light.png` by `gen_gamepad_xbox_dark.py`, so both skins share one layout (`xbox_gamepad_layout`). Regenerate rather than nudging UVs by hand.
- Xbox idle is cream body (`#E4E4E6`), night-ink controls, field-slate (`#303440`) stick caps, and app-colored ABXY (Y yellow, X blue, B red, A green) on night-ink discs. No LT/RT/LB/RB/VIEW/MENU labels. View/Menu are unlabeled dots.
- Fresh install: `gamepad_bg = 0` (no night-ink plaque — just the pad). Panel opacity in settings still brings the plaque back.
- Triggers are analog 0–1 fill (DualShock wings / Xbox horns), snapping to a full fill at 80%. Bumpers are digital on/off shoulder bars.
- A press covers its whole control — no ring of ink around the fill. The Xbox layout radii and boxes are measured off the rendered art by `gen_gamepad_xbox.py`, not off the target, because the traced masks grow when they are smoothed. `xbox_press_covers_the_whole_control` pins this (it counts only pixels the press turned orange, since anti-aliased Y edges also read as orange).
- Sticks follow X/Y. Wells stay opaque — do not punch through to the game (including the trough between the well rings). Do not only light a ring without moving the disc.
- Face / D-pad primary fills out to that pad’s outline. Xbox face press is a primary disc on cream that keeps the colored letter. Bumpers fill the rounded shoulder bar, not a square stamp.
- Pulled trigger fill follows the rounded bottom lip (not a flat cut). Analog squeeze is from that lip up. Bumpers light the shoulder bar while held.
- DualShock pressed face symbols stay thin cream strokes on the primary fill. Do not thicken them, ink them black, or redraw outlines.
- No pad is the **No controller** pill, not an idle pad.
- Do not poll XInput/HID every overlay frame unless Controller is visible or Stance is shown (or bind listen). Disconnected XInput slots still back off for 2s.
- Sit / stand stays Stance. This widget must not change sitting.
- Other riders’ throttle / brake are not this widget.

## Change log

- 2026-09-25 — Left Labs. Regular Cockpit widget. `show_gamepad` is enough. Experimental features still gates Profile → Tracks.
- 2026-09-24 — Light PlayStation pad. Theme now switches both pads (default Light for everyone): the light DualShock is a cream body with black outlines and panels, slate d-pad keys and shoulders, and no L1/L2/R1/R2 labels. The "PlayStation: dark only" lock is gone. Xbox dark bumpers lose their dividers so they match the light pad.
- 2026-09-24 — Theme is back for Xbox. Controller → **Theme** picks Light (the filled pad, default) or Dark, a new DualShock-style Xbox schematic: charcoal body, thin grey outlines, outlined ABXY, a ring around the d-pad, and a divider so LB/RB light only the bumper. The Dark art is regenerated from the Light art so every control stays put. PlayStation stays dark; the row shows "PlayStation: dark only" and is locked while Pad is PlayStation.

- 2026-09-24 — Xbox press fills are smooth too. Xbox presses (LT/RT, LB/RB, d-pad, ABXY, View/Menu, guide) are laid into the full-size art and area-averaged down like the DualShock, so the primary fill no longer stair-steps or leaves dark specks on the A rim. The idle Xbox pad moved to the same area-average downscale so pressed regions do not seam. The DualShock PS button now presses the same way.

- 2026-09-24 — DualShock press fills are smooth. Presses used to be painted per screen pixel after the pad was shrunk, so the primary fill stair-stepped and the L2/R2 labels broke up on it. Presses are now laid into the full-size art and area-averaged down with the rest of the pad, and the L1/L2/R1/R2 labels are gone from the art (the shoulders read fine unlabeled). Pressed ×/△/○/□ stay cream on the fill.

- 2026-09-24 — DualShock lines smoothed. The dark art had transparent pinholes along its strokes and a hard stair-step edge, and the pad was shrunk with a single resample, so outlines broke into dashes at widget size. It is now regenerated clean (`gen_gamepad_ds4_dark.py`) and area-averaged down. The stick ring is a solid anti-aliased circle, and press fills keep partially covered glyph pixels so × / △ stay readable.

- 2026-09-16 — Press fills (face, D-pad, triggers, bumpers) use Look primary color, not baked Holeshot orange.
- 2026-09-16 — Controller skin is platform-locked (PS dark, Xbox light); Theme setting removed temporarily. Four PNG skins remain embedded for a future Theme control.
- 2026-09-16 — A press now covers its whole control (the layout is measured off the rendered art), a trigger held past 80% reads as full, and the crown-to-body outline is traced and low-passed along its length instead of pixel-filtered, so the shoulder seam no longer waves.
- 2026-09-16 — Xbox art is traced 1:1 from the reference render (`gamepad-xbox-target.png`) instead of the old stretched blueprint: real Xbox One outline at its own 1344×1024 aspect, ABXY on night-ink discs, light guide disc with a dark X, unlabeled View/Menu dots, field-slate stick caps. Stick cap radius comes from the layout now, so each pad's art drives it.
- 2026-09-16 — Xbox is a filled Xbox One two-tone (cream body, night-ink controls, colored ABXY, plus d-pad). DualShock drawing and UVs stay as they were.
- 2026-09-04 — Rider-facing name is Controller. Behind Settings → Labs → Experimental features. Ini keys stay `show_gamepad` / `gamepad_*`.
- 2026-09-03 — Xbox is its own Series silhouette (LS up-left, d-pad down-left, ABXY, LT/RT). DualShock drawing and UVs stay as they were.
- 2026-09-03 — Trigger orange follows the rounded bottom lip per column, not a flat row. Full pull still fills the wing.
- 2026-09-02 — Pressed labels are dark ink on orange. Bumpers fill the rounded shoulder. Face and D-pad fills reach the outline.
- 2026-09-02 — Stick wells are opaque (no dirt through the trough or ring dashes). Pulled trigger orange follows the DualShock wing outline instead of a stair-stepped cap.
- 2026-09-02 — Added. Glass DualShock/Xbox over the game (no plaque by default). Analog trigger squeeze, bumper lights, sticks leave wells. No controller pill.
