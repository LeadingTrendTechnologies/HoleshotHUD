# Relative

Riders immediately ahead and behind you on track position, not classification order. Settings subtitle: “Riders just ahead and behind you”.

## Code

- Draw: `draw_relative` in `overlay/hud/src/render.rs`
- Columns: `RelField` in `overlay/hud/src/config.rs`
- Settings: `pane_relative` in `overlay/src/settings.rs`
- In-game HUD copy: `drawRelative` in `src/hud/widgets.cpp`

## Data

Sorts `riders[].track_pos` wrapped around you (`wrap` so +0.5 / −0.5 is the far side of the loop). Focus is `focus_race_num`, else `local_race_num`.

**Nearby riders** (`relative_count`) is count *each side*, not total rows. Visible set is up to `2 * count + 1` (ahead + you + behind).

Classification is joined by race number for position, laps, bike, best/last, penalty, interval, status. The **Position** column is the live place (`RaceField` rows), so a pass shows there even though the row order is track order. See [live race order](../live-order.md).

## Behavior

- Same chrome as Standings (header bar, track name, column headers, optional footer). Header/footer slots include **Fuel**.
- Your row is highlighted. Lapping colors on **other** rows: blue if they are a lap ahead and closing from behind, red if you are a lap ahead and closing on them (`lap_rel` / `lap_row_bg`). Off in warmup. **Row highlight** opacity (`rel_hl`) scales your row and the blue/red lapping tints. **Text color** is White or Black (`rel_text`); bike pills keep brand colors. **Alternating rows** (`rel_stripe`, default on) paints every other row near-black. Same opaque-panel lift as Standings.
- **Gap column is not classification gap.** It is `|wrapped_frac * track_length / local_speed|` in seconds (you show `0.0`). Speed floor is 4 so a stopped rider does not explode the number.
- Rows slide when the nearby set changes (`REL_SLIDE`).
- Duplicate race numbers are skipped. Empty names with `race_num <= 0` are skipped.

Default columns on: Number, Name, Gap, Fastest, Last lap.

## Do not regress

- Do not sort Relative by standings position. It is on-track neighbors.
- Keep the wrap (`d > 0.5` subtract 1, `d < -0.5` add 1) or the “nearest” set jumps across S/F.
- Empty / no telemetry shows “Waiting for positions”.
- No blue/red lapping row tints in warmup.
- Two laps down must not tint a better-placed rider red. `gap_laps` wins over `num_laps`.
- Missing `rel_stripe` in the ini keeps alternating rows on.
- Alternating rows must still read at **Background** 100% (lift, not extra black on night-ink).
- Fuel header/footer is liters/US gallons (`Fuel`) or tank percent (`Fuel %`). Empty volume is `0.0`; `--` / `--%` only when tank size is missing.

## Change log

- 2026-08-29 — Two laps down no longer tints the leader’s row red. Same `gap_laps` preference as Map.
- 2026-08-29 — **Fuel %** is a separate header/footer option from volume.

- 2026-08-29 — Fuel reads as liters or US gallons from Units, not percent.

- 2026-08-29 — Fuel level is a header/footer option (shared `BoardField::Fuel` with Standings). Tank percent; `--` if max fuel is missing.

- 2026-08-27 — Plaque width hugs the columns so leftover widget space is not empty glass. Fresh-install board is 20% of the screen as a max (was 30%). Existing layouts in the ini are unchanged.

- 2026-08-27 — **Alternating rows** toggle (`rel_stripe`, default on) turns the near-black zebra stripes off. Opaque **Background** lifts the stripe so 100% still zebras.

- 2026-08-27 — **Alternating rows** toggle (`rel_stripe`, default on) turns the near-black zebra stripes off. Opaque **Background** lifts the stripe so 100% still zebras.

- 2026-08-25 — Position column is the live place, so passing the rider on the row above changes the number without waiting for the line. Row order is still track order.
- 2026-08-24 — Classification join (position / laps / interval) reads shared `RaceStore`; still sorts by track_pos. No visual change.
- Early overlay — Track-pos relative board with count-each-side.
- 0.1.0 — Configurable header fields.
- 0.1.4 — Lapping row backgrounds (blue / red) using the same close-and-a-lap-ahead rule as map dots.
- 0.1.8 — Hidden until **Show on overlay**.
- 2026-08-18 — Wiki created. Gap remains estimated time from track delta / speed, not `gap_ms`.
- 2026-08-19 — Warmup drops lapping row tints; practice laps are not race lapping.
