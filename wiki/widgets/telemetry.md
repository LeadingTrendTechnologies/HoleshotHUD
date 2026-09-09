# Telemetry

Throttle and brake over the last few seconds, live clutch / brake / throttle bars, and gear plus speed. Settings subtitle: “Throttle and brake traces, analog bars, gear and speed”. Each section (traces, bars, gear/speed) and each channel (throttle/brake/steer traces; clutch/brake/throttle/steer bars) can be turned off. Hidden sections reflow so the rest fill the plaque. Gear flushes faint red at `shift_rpm` or the limiter. Steer is off until you turn it on.

Regular Cockpit widget. Hidden until **Show on overlay**.

## Code

- Draw: `draw_telemetry` in `overlay/hud/src/render.rs`
- Trace ring: `overlay/hud/src/telemetry.rs` — sampled from `RaceStore::refresh`
- Inputs: SHM `localThrottle` / `localFrontBrake` / `localRearBrake` / `localClutch` (V14)
- Settings: `pane_telemetry` in `overlay/src/settings.rs` — Show (traces / bars / dial), then channel toggles under the open section.

Night-ink plaque, 6px left corners, circular right end. Overlaid traces: ahead-green throttle, behind-red brake, optional cream steer (zero at mid-well, right up). Analog bars: clutch slate, brake red, throttle green, optional cream steer from center. Live percent on a bar that is actually in; steer is a signed percent. Circular gear well: huge ExtraBold Italic gear (faint red at shift or limiter), speed stacked under it, dim unit under speed, Holeshot Orange RPM arc. Traces are Catmull-Rom curves, not stair-step segments. No left title. Gear and speed still live on Dash — this strip is the input history.

Brake bar is `max(front, rear)`. Riding uses BikeData (clutch + both brakes + steer). Spectate uses that rider’s throttle and front brake; clutch, rear, and steer stay off because other bikes do not send them.

Default 56%×14.5%, lower-center. Panel opacity 82. Fresh install: `show_telemetry = false`. Restart MX Bikes after the V14 plugin so `Local\MXBOHudV14` loads.

## Do not regress

- Do not put the unit between gear and speed — speed sits under gear.
- Do not draw a TELEMETRY spine or orange pip on the left.
- Do not use a dotted grid or neon lime/red. Meaning green/red and Holeshot Orange only.
- Do not steal Dash’s place / clock / flags. This is inputs + gear/speed.
- Do not treat Controller’s pad picture as this widget.
- Do not publish clutch / rear brake from other riders — they are not in `RaceVehicleData`.
- Hidden until **Show on overlay**.
- Missing ini keys for the section/channel toggles stay on. Do not treat an old ini as “all off”.
- Do not leave a gap when a section is off — remaining sections take the space.
- Steer defaults off. Do not paint it orange or meaning-green/red — cream only. Spectate has no steer.
- Gear goes faint red at `shift_rpm` or `max_rpm`, not the full brake red. Speed stays white.

## Change log

- 2026-09-09 — Settings: Show masters first; Throttle / Brake / Steer nest under the open section. Note is one spectate line.
- 2026-09-09 — Gear flushes faint red at the shift light or limiter. Optional cream steer trace (mid-zero) and center-zero steer bar; both default off.
- 2026-09-09 — Section toggles (traces / bars / gear-speed) and channel toggles (throttle/brake traces; clutch/brake/throttle bars). Hidden sections reflow.
- 2026-09-09 — Smoothed throttle/brake traces (5-tap + Catmull-Rom). Speed sits under gear; unit sits under speed.
- 2026-09-09 — First ship. Strip Tape: traces, analog bars, gear/speed dial. SHM V14 for throttle / brakes / clutch.
