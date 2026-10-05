# Session

Place and the session clock or lap count on one line. No gear, speed, footer, or flags. Settings subtitle: “Position and time or laps left”.

## Code

- Draw: `draw_timer` in `overlay/hud/src/render/timer.rs`
- Clock text: `race_progress_text` (the banner `RaceStore::tick` already stored). Do not call `session_remain_ms` from the draw.
- Place: `standing_pos` for the focus rider, same live order as Dash. Primary color. Gold crown on P1. Green or red `*` when live place disagrees with on-track place.
- `~Lapped`: dash amber, smaller, on the same baseline as the clock, only when `lapped` is true.

One night-ink chamfer. Place and the clock or laps (`07:32`, `2 / 5`, `0/1`) are the same size, with a gap of about half the type size between them. The plaque fills the widget rect. Hold Ctrl and drag to resize. A new layout starts at the Warmup plaque, top center, hidden until **Show on overlay**. Saved layouts keep the size they already have. **Panel opacity** (default 82) is on the Session pane with font size and bold, after Show on overlay. At 0 the chamfer border is not drawn. Under 40% the place, penalty `*`, clock or laps, and `~Lapped` get a soft night-ink rim (copies on two circles, so the edge is round). The crown is not rimmed.

**Out of riders** is off. When it is on, place reads `P3/12`. The count is the classified field (`standing_count`), or the riders on track when that field is empty. A zero count stays `P3`. The penalty `*` still follows the whole token.

## Do not regress

- Do not stack the place over the clock, and do not put the place on an orange chip.
- Do not call `session_remain_ms` from this widget. Read `race_progress_text`.
- Do not wrap flags on this plaque. White, checkered, yellow, blue, and red stay on Dash and Flags.
- Do not show a drop shadow. The approved still had one; the HUD plaque does not.
- `~Lapped` stays dash amber, not orange, so it does not read as a second place color.
- Hidden by default. Ini keys are `timer_` / `show_timer` / `timer_of`, not `session_`, so they do not collide with Practice / Warmup / Race / Spectate.
- `timer_of` missing from an ini stays off. Do not turn it on for existing layouts.
- At panel opacity 0, do not draw the plaque fill or border. Under 40%, rim the type with the soft circle outline. Do not rim the crown, and do not use a drop shadow. Do not switch Session back to the hard 8-neighbor rim.

## Change log

- 2026-10-03 — Opacity 0 drops the plaque border. Under 40% the place, clock, and ~Lapped get a soft round night-ink rim.
- 2026-10-03 — Place and the clock sit farther apart. Panel opacity, font size, and bold show on the Session pane once the widget is on.
- 2026-10-03 — Mouse-up paints the dragged rect. The preview stays through that frame, so the plaque does not flash back to the old size.
- 2026-10-03 — Ctrl-drag follows the pointer. On a short plaque the handle hit shrinks so the center stays a move.
- 2026-10-03 — Default plaque is the Warmup size. Out of riders appends `/{count}` to the place token before the row is measured.
- 2026-10-02 — Session is the dash’s place and clock on one line, so a rider can leave the full dash off. The clock is the existing banner, not a second countdown.
