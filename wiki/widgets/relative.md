# Relative

Riders immediately ahead and behind you on track position, not classification order. Settings subtitle: “Riders just ahead and behind you”.

## Code

- Draw: `draw_relative` in `overlay/hud/src/render.rs` (chrome via `draw_table_board`)
- Columns: `RelField` in `overlay/hud/src/config.rs`
- Settings: `pane_relative` in `overlay/src/settings.rs`
- In-game HUD copy: `drawRelative` in `src/hud/widgets.cpp`

## Data

Sorts `riders[].track_pos` wrapped around you (`wrap` so +0.5 / −0.5 is the far side of the loop). Focus is `focus_race_num`, else `local_race_num`.

**Nearby riders** (`relative_count`) is count *each side*, not total rows. Visible set is up to `2 * count + 1` (ahead + you + behind). Ctrl+resize height changes that count (1–8).

Classification is joined by race number for position, laps, bike, best/last, penalty, interval, status. The **Position** column is the live place (`RaceField` rows), so a pass shows there even though the row order is track order. A trailing green/red `*` marks when that live place differs from on-track place because of penalties. See [live race order](../live-order.md).

**Status** column (`rel_status`, default off) uses the same icons as Standings Status (finish, crash, DNS/OUT/DSQ, pit). Finished riders show only the flag. No column header title. Out / crashed riders keep bright **Position**, **Number**, and **Name**; other cells can still dim.

## Behavior

- Same chrome as Standings (header bar, track name, column headers, optional footer). Header/footer slots include **Fuel**, **Setup**, **Gap ahead**, **Gap behind**, **Delta**, **Last** / **Current**, **Gap to leader**, **Engine**, **Penalty**, and **Server**. Those place gaps are race place (P−1 / P+1 / P1), not the riders in this table. Same lap ticks along the track toward that rider; a live lap or more is `1L` / `-1L`.
- Optional **Category** (`rel_category`) and **Speed** (`rel_speed`) columns, default off. Speed uses rival `Rider.speed` (your row falls back to `local_speed`).
- Your row is highlighted. Lapping colors on **other** rows: blue if they are a lap ahead and within catch span (either side), red if you are a lap ahead and within catch span (either side) — colors hold through a pass while still nearby (`lap_rel` / `lap_row_bg`). Off in warmup. **Row highlight** opacity (`rel_hl`) scales your row and the blue/red lapping tints. **Text color** is White or Black (`rel_text`); bike pills keep brand colors. **Alternating rows** (`rel_stripe`, default on) paints every other row near-black. Same opaque-panel lift as Standings. **Plaque text** is Black or White on the orange rider-count / track-name skews (`rel_plaque_text`, default Black). **Show plaques** (`rel_plaque`, default on) hides those skews and collapses their band.
- **Gap column is not classification gap.** It is `|wrapped_frac * track_length / local_speed|` in seconds (you show `0.0`). Speed floor is 4 so a stopped rider does not explode the number.
- Rows slide when the nearby set changes (`REL_SLIDE`).
- Duplicate race numbers are skipped. Empty names with `race_num <= 0` are skipped.

Default columns on: Number, Name, Gap, Fastest, Last lap.

## Do not regress

- Do not sort Relative by standings position. It is on-track neighbors.
- Header/footer **Gap ahead** / **Gap behind** are live-order place neighbors (P−1 / P+1). Same lap is seconds along the track toward that rider; a live lap or more is `1L` / `-1L`.
- Header/footer **Gap to leader** is live-order gap to P1. **Server** is `--` when `server_name` is empty.
- Optional **Category** / **Speed** columns stay off by default.
- Keep the wrap (`d > 0.5` subtract 1, `d < -0.5` add 1) or the “nearest” set jumps across S/F.
- Empty / no telemetry shows “Waiting for positions”.
- No blue/red lapping row tints in warmup. `session_kind` 5 wins even when extras leak.
- Two laps down must not tint a better-placed rider red. `gap_laps` wins for blue when they are ahead of you.
- Red only when you gained a lap on them (pairwise `num_laps`). Leader lapping someone behind you is not red.
- Same-race S/F straddles must not tint blue/red. When `num_laps` differ, continuous `num_laps + track_pos` must also round non-zero.
- Blue/red lapping tints hold through a pass while still inside catch span (either side).
- Missing `rel_stripe` in the ini keeps alternating rows on.
- Missing `rel_plaque` / `rel_plaque_text` keep plaques on with black ink.
- Alternating rows must still read at **Background** 100% (lift, not extra black on night-ink).
- Fuel header/footer is liters/US gallons (`Fuel`) or tank percent (`Fuel %`). Empty volume is `0.0`; `--` / `--%` only when tank size is missing.
- Setup header/footer is the loaded bike setup filename stem. `--` when `RunInit` has not sent it. Restart MX Bikes after the V13 plugin.
- Ctrl+resize chrome is the hugged plaque (column pack × row stack), not leftover widget glass. Dragging it larger grows Name / nearby count.
- The night-ink plaque must cover every visible row. Do not clamp stack height to a shorter saved `relative_h`.
- Finished riders show only the Status finish flag — not crash / pit / DNS / OUT / DSQ on top of it.

## Change log

- 2026-09-25 — Shared board chrome gains Delta / Last / Current / Gap to leader / Engine / Penalty / Server. Optional **Category** and **Speed** columns (`rel_category` / `rel_speed`, default off).
- 2026-09-24 — Red is pairwise only: leader lapping someone behind you no longer tints them red.
- 2026-09-24 — Same-race S/F straddles no longer tint blue/red (`other_laps_ahead` continuous progress when `gap_laps` match).
- 2026-09-22 — Status finish flag wins over crash/pit for done riders. Your-row / lapping wash alpha 52 at default **Row highlight** (spider-scaled).
- 2026-09-22 — Your-row and blue/red lapping washes match Profile spider fill (full chroma, alpha 40 at default **Row highlight**).
- 2026-09-22 — Blue/red lapping row tints hold through a pass while still within catch span (either side).
- 2026-09-22 — Out / crashed **Position** and **Number** stay normal ink (same as Name), not greyed.
- 2026-09-22 — Removed the separate **Crashed** text column. Status header is blank. Out / crashed names stay full ink (not greyed).
- 2026-09-22 — **Status** column (`rel_status`, default off) with the same crash / finish / DNS / OUT / DSQ / pit icons as Standings.
- 2026-09-22 — Settings column drag slides neighboring rows into place (ease-out), instead of snapping on drop.
- 2026-09-22 — Penalty column shows whole seconds as `#s` (e.g. `5s`), not lap-style `5.000`.
- 2026-09-22 — **Plaque text** (Black/White, default Black) and **Show plaques** for the orange rider-count / track-name skews. Same controls as Standings. Hidden plaques collapse the track band height.
- 2026-09-10 — Practice **Laps** for you follows a crashed crossing the same way Standings does.
- 2026-09-09 — Header/footer **Gap ahead** / **Gap behind** are live-order P−1 / P+1. Same lap ticks along the track toward that rider; a live lap or more is `1L` / `-1L`. A pass switches the rider. Times have no leading `+`; ahead is an up arrow, behind a down arrow.
- 2026-09-08 — **Gap ahead** and **Gap behind** are header/footer options (shared `BoardField` with Standings). They use classification place, not the nearby riders in this table.
- 2026-09-07 — Warmup (`session_kind` 5) keeps rows slate even when leaked extras make the lap field look like a race.
- 2026-09-06 — Plaque height follows the visible nearby set. A short saved widget box no longer leaves later rows on the game with no glass.
- 2026-09-03 — **Setup** is a header/footer option (shared `BoardField::Setup` with Standings). Restart MX Bikes after this plugin so SHM `Local\MXBOHudV13` loads.
- 2026-08-31 — Ctrl+resize of the hugged plaque grows Name width and nearby-rider count, so the table can get larger instead of the orange box being a no-op.
- 2026-08-31 — Ctrl+resize orange box (and grab handles) follow the hugged plaque, not leftover widget glass.
- 2026-08-31 — Plaque, header/footer, stripes, bike pill, and row slide share `draw_table_board` with Standings. Neighbor selection and gap-from-track-pos stay here.

- 2026-08-29 — Two laps down no longer tints the leader’s row red. Same `gap_laps` preference as Map.
- 2026-08-29 — **Fuel %** is a separate header/footer option from volume.

- 2026-09-10 — Speed, Liquids, and Temperature units are independent in Settings. Legacy `units=` still seeds all three on upgrade.
- 2026-08-29 — Fuel reads as liters or US gallons from Liquids units, not percent.

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
