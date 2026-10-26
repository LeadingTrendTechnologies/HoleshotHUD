# Guide

A filled chevron that points you back onto the track. Same aim and show/hide as the chevron under the Dash gear. Settings tab **Guide**. Settings subtitle: “Chevron back to the track. Same guide as Dash”.

## Code

- Draw: `draw_guide` in `overlay/hud/src/render/guide.rs`
- Bearing and latch: `overlay/hud/src/track_guide.rs` (`tick` / `visible`). Dash calls the same latch.

The chevron is the Look accent and fills about 0.72 of the shorter side of the widget rect. No plaque. Font, bold, and panel opacity stay on the shared pane and do not change the chevron. It draws only while Settings → Labs → Experimental features is on, **Show on overlay** is on, and the latch is up: join (unless you are already on the line and facing along it), the whole time you are crashed, farther than 12 m from the chosen centerline sample, or more than 90° from bike-forward. A ~50° corner on the line does not show it. It hides after you are up, within 12 m, and within 40° of track travel, held 0.4 s. Up is the bike’s nose, not the camera. See [dash.md](dash.md) for the fold rule (40 m along the lap, then 12 m ahead of the stretch you left).

Hold Ctrl and the layout box is there even when the chevron is hidden, so you can place it off track. Default rect is a small square just above the dash. Hidden until Labs is on and **Show on overlay**. Dash **Track guide** stays its own toggle; both can be on. Neither draws until Experimental features is on.

## Do not regress

- Do not give Guide its own aim. It reads `track_guide::visible`.
- Do not draw a plaque behind the chevron.
- Do not hide the Ctrl layout box just because the latch is down.
- Hidden until Settings → Labs → Experimental features is on, then until **Show on overlay**. Ini keys are `guide_` / `show_guide`. A missing key stays off.
- Turning Guide on does not turn the dash chevron off, and the other way around.

## Change log

- 2026-10-26 — Guide stays off until Settings → Labs → Experimental features is on.
- 2026-10-19 — Guide is its own widget: the filled chevron, same latch as Dash. Hidden until Show on overlay.
