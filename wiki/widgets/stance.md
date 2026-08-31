# Stance

Sit / stand plaque for the local rider. MX Bikes does not send posture in the plugin API, so this widget **mirrors a local input** (pad, keyboard, or mouse).

Default bind: **Right bumper**, **Toggle**. That matches a PS5 R1 sit toggle through Steam’s Xbox mapping, DualSense / DualShock 4 HID if Steam is not wrapping the pad, and Xbox pads via XInput.

**Sit button** in settings is press-to-set: click the row, it turns orange (**Press a button now**), then press a pad button, key, or mouse button. Esc or click again cancels. The settings key (F8) is skipped so it cannot steal the bind. Left-click the game window (not Settings) to bind mouse left.

**Look** in settings: **Text** (STAND / SIT plaque) or **Icon** (`Rider-Standing.png` / `Rider-Sitting.png`).

**Show sitting** defaults **off**: standing draws the icon or text; sitting hides the widget. Turn it on to keep sit visible.

Starts **hidden**. Turn on **Show on overlay**. Lives under Cockpit with Dash and Systems. Hides in the menu / lobby / garage like every other widget. Settings always shows a disclaimer that this is not game telemetry.

## Code

- Draw: `draw_stance` in `overlay/hud/src/render.rs`
- Icons: `overlay/hud/assets/stance-stand.png`, `stance-sit.png` (from `assets/png/Rider-Standing.png` / `Rider-Sitting.png`)
- Poll: `overlay/src/stance.rs`. Xbox / Steam Xbox mapping via `XInputGetState` when a pad is connected. Otherwise DualSense / DualSense Edge / DualShock 4 HID. Keyboard and mouse via `GetAsyncKeyState`.
- Settings: `pane_stance` in `overlay/src/settings.rs`

## Do not regress

- This is **not** rider animation state. Settings disclaimer stays visible even when Show is off. Toggle desyncs if the game sits you without a press (crash / reset). **Reset to standing** re-syncs.
- Do not draw in the menu / lobby. Same session-data hide rule as other widgets.
- Hold-to-sit follows the button: down = sit, up = stand.
- Fresh install: `show_stance = false`. Default look is **Text**. `stance_show_sit = false` (sitting hides). Not gated by Labs.
- Sit button is press-to-set, not a dropdown. Listening replaces the row with an orange “Press a button now” plaque. Pad (face, bumpers, L2/R2, D-pad), keyboard, and mouse bind. Escape cancels listen; it does not bind.
- Do not infer sit/stand from suspension or extra telemetry bytes.

## Change log

- 2026-08-31 — Do not poll XInput/HID every overlay frame. Pad scan runs only while Stance is shown (or while listening for a bind). Disconnected XInput slots back off for 2s.

- 2026-08-26 — Listening for a bind fills the Sit button row orange with “Press a button now” so it is not a small Press… chip.
- 2026-08-26 — Disclaimer is flag-yellow (`#F4D624`), not dim gray, so it does not read as a subtitle.
- 2026-08-26 — Hides without a session. Settings disclaimer: not connected to MX Bikes; follows the bind only.
- 2026-08-26 — Promoted out of Labs. Always on the Cockpit rail. `show_stance` is enough.
- 2026-08-26 — Sit bind accepts keyboard and mouse, DualShock 4 HID, plus Xbox / DualSense. Click Sit button, then press the input.
- 2026-08-26 — Sit bind includes D-pad and L2/R2 (XInput analog triggers, DualSense digital + analog).
- 2026-08-26 — **Sit button** is press-to-set (click, then press the pad button) instead of a dropdown.
- 2026-08-26 — Added. Pad-button mirror (toggle or hold) after API extra bytes and shock-length probes found no posture field.
- 2026-08-26 — DualSense HID used Square (byte 0 bit 4) as L1. L1/R1 are byte 1. Skip HID when XInput already has the pad.
- 2026-08-26 — Behind **Experimental widgets** with Sectors.
- 2026-08-26 — Look toggle: Text plaque or rider icon (`Rider-Standing.png` / `Rider-Sitting.png`).
- 2026-08-26 — **Show sitting** off by default; sitting hides the widget.
