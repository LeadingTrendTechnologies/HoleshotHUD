# Live race order

The game's classification (`RaceClassification`) is only republished when a rider crosses
the line, so an on-track pass used to sit invisible until the next lap: standings, dash,
map crown and the nearest ahead/behind rings all kept the old places. `race_store.rs`
re-derives the order every tick instead.

## How it works

`live_order` starts from the game classification (`Standing.position`, P1 first) and
reorders every rider it can place by one score, then publishes:

- `RaceField.rows` in that order, each `standing.position` overwritten with the live place
  (so `RaceField::board()` is what the boards iterate)
- `LIVE_ORDER` (race numbers) behind `live_position()` / `live_leader()` for map-style
  lookups that only have a race number

Each placed rider's score, high to low:

`num_laps × track_length + metres into this lap − penalty_m(their own penalty_ms)`

| Rule | Why |
| --- | --- |
| Metres into the lap | If we saw that rider on the gate: one continuous coordinate, set each tick to where they are now (the unwrap of the current fraction closest to the last coordinate), measured from the back of the stored gate fractions. It is not a sum of steps. A short step behind the origin lands just below 0. A step across the line lands just above the next integer, so the place does not collapse while `num_laps` is still the old value. `num_laps` rising does not reset the coordinate. It only lifts a rider whose coordinate is a full lap behind the classification. A jump over 80 m does not move the coordinate that tick; the next tick that stays there accepts it. A rider who loses `track_pos` keeps that coordinate, so riding past a crash takes the place. While every stored gate rider is still within 5 m of their slot, or the holeshot has not been called and nobody is 200 m past the back of the grid, there is no place, even if `IN_GATE` has already cleared, and the qualifying position is not shown. Sector geometry does not override a rider we saw on the gate. With no gate snapshot, known S1 and S2 fractions: forward distance from those gates (`split_fracs`, wrap like sectors). The arc from S2 back toward S1 counts as past S2 when the rider is closer to S2 than to S1, and for the whole arc once their sector gate is 2. Closer to S1, with no S2 gate, stays before S1 so the start grid is not a finished sector. Without saved fractions, a published sector gate (1 after S1, 2 after S2) plus travel since that gate. After a lap is complete, tracker distance since the gate or their last crossing, when armed. Otherwise metres past a known start/finish (`SF_FRAC_LEARNED`, or `sf_meters` when it is set). With neither, only a same-lap pair inside 250 m can move. |
| Their own penalty | The game only applies `penalty_ms` to the results. Live, each rider's seconds are metres at session-best pace (`track_length / best_lap`, or 15 m/s before anyone has a lap). Place does not matter: a 10 s penalty and a 5 s penalty are both subtracted from that rider, and the field is sorted once. |
| No `track_pos` | Stay scored from the last coordinate, or from armed lap metres. The bubble can pass them. DNS / OUT / DSQ stay in the scored slot. |
| Ahead by `PASS_M` (3 m), or `HOLD_M` (0.5 m) if they already hold the place | Only a tie break so two bikes on the same stretch do not swap every frame. |
| Not both scored after the leader finished | A cool-down pass must not move the results. |

The order is rebuilt from the game order **every tick**, not carried forward, so a bad swap
cannot stick — the previous order is only read for the hysteresis margin.

## Penalty vs on-track place

While live order is on, a second ranking uses the same bubble with penalties ignored
(`TRACK_ORDER` / `track_position`). When a rider's live place and on-track place differ,
boards paint a trailing `*` on the place: green (`ahead_col`) when live is ahead of
on-track, red (`behind_col`) when behind. Dash hero `P#`, Dash/board Position (and
ClassPos), Standings, Relative, and H-Standings all use it. Map and Minimap numbered
dots center the same `*` after the digits. No star when numbers are off.

## Progress tracker

`PROGRESS` keeps, per race number, metres covered (`wrap_signed` step × `track_length`
each tick) and where their current lap started:

- On the gate (`clock.in_gate`, or `session_state` 256 after the pre-start countdown
  has already cleared that latch) everyone is zeroed and **armed**, and their lap fraction
  is stored as the start of a continuous coordinate. No race place is published (boards
  show a blank or `-`) until the holeshot line. Leaving the slot is not enough: the front
  of the grid is not a place. If the game never calls the holeshot, the place appears once
  the furthest rider is 200 m past the back of the grid. Qualifying
  position is not a fallback in that window. After that, place is where the bike is.
- When a rider's `num_laps` rises the lap base moves to the current distance and they are
  armed. Spectating / joining mid-race arms each rider on their first crossing we see.
- On lap 1, a sector-gate rise moves the lap base (travel since that gate) and clears a
  corrupt pin. After a lap is complete, a gate rise clears a corrupt pin only — a healthy
  armed rider keeps their metres. The gate count dropping at the next lap does not disarm
  them.
- A step over `MAX_STEP_M` (80 m) is a reset / shortcut / teleport and does not move the
  gate coordinate that tick. The next tick that stays at the new fraction accepts it, so
  a one-frame spike is ignored and a sparse update is not dropped. Dropping out of
  `riders[]` freezes the coordinate and keeps `armed`; on reappear travel resumes
  without inventing a teleport step.
- If the race goes live without a gate tick and the line is still unknown, the first
  real motion step cold-arms that rider so far pairs still score on lap 1.
- Armed tracker distance is the lap metres in the score. Unarmed riders use metres past
  the line instead. Armed metres into the lap are capped to one lap. A small overshoot
  past one lap only clamps; past one lap plus slack without a `num_laps` rise the rider
  is marked corrupt and keeps their game place (no S/F fallback — that would read ~0 m
  just after a wrap and drop a leader to mid-pack until the line republishes).

## When it is off

`live_order_active`: needs `on_track` (a run, or positions / telemetry — so replay counts; leftover garage standings and leftover spectate dots do not), a real `track_length`, two or more riders, not
`is_warmup` (a practice field is ranked by lap time, not by track progress), and
`IN_GATE == 0` (on the gate everyone sits on the same stretch, and the place stays blank).

## Do not regress

- Gaps stay the game's numbers. Only `interval` is derived, as a size (`abs`), because a
  fresh pass can leave the pair's gap to the leader the wrong way round for a moment.
- Do not resync the live order to the game whenever the classification changes: it is
  republished when *any* rider crosses the line, and that would undo a pass made since the
  pair's own crossing.
- Every running rider is scored from their own track progress and their own penalty.
  Do not gate that on the game place they started the tick in, or on being within 250 m.
- Do not bubble adjacent pairs only: a pinned row (no `track_pos`, out of race) must not
  stop passes above it.
- Gaps and intervals stay the game's numbers. The penalty is only in the order.
- Do not let armed lap metres exceed one lap without a `num_laps` rise: that invents a
  free lap of score and can flash P1 until the line republishes. Past one lap + slack,
  pin to the game place with no S/F fallback — do not disarm into S/F metres near 0
  after a wrap (that drops a true leader to mid-pack until the line).
- The same `PAIR_MAX_M` / continuous-motion rule applies to the Dash `~Lapped` pass latch
  (`note_lapped_by_leader`): an over/under tabletop projection spike must not sticky-latch.
- Do not clear `armed` when a rider briefly drops from `riders[]`; freeze travel only.
- When the line is unknown, cold-arm on the first real motion step after the race goes
  live (missed gate). Do not arm on a zero step — that invents equal travel for riders
  already spread out.
- Lap 1 uses sector gates so far pairs move before the finish. Do not treat track
  fraction 0 as S1. After `num_laps` has risen, the armed tracker wins — do not keep
  scoring that lap from sector geometry.
- Do not rebase a healthy armed rider when their sector gate rises after `num_laps` has
  increased. That zeroes the lap and swaps places until the rest of the field reaches
  the same gate.
- On lap 1, a rider just past the learned S2 fraction stays ahead of riders still between
  S1 and S2 even if `sector_gate` has not reached 2 yet. Do not score that stretch as a
  full lap behind S1. The start grid, just before S1 with no S2 gate, stays behind S1.
  That geometry is only the fallback when the gate was never seen. Do not let it override
  the unwrapped coordinate.
- Do not publish a place while `in_gate`, while `session_state` is 256, while every stored gate rider is still within
  5 m of their slot, or after that gate until the holeshot is called (or the lead is 200 m
  past the back of the grid). Qualifying order is not a seed for the first lap, and it is not the
  place shown when live order is blank.
- Falling behind the rear gate slot lands just below 0. Do not wrap that step into almost a full lap.
- Do not reset the gate coordinate when `num_laps` rises, and do not switch that rider to
  `travelled_m - lap_base_m`. A line cross before the bump keeps the lead.
- A crashed rider who loses `track_pos` keeps their last coordinate and can be passed, or
  can be the one passed. Do not pin them in the classification slot.

## Change log

- 2026-10-05 — Place for a rider seen on the gate is where they are now. A finish does not drop them before `num_laps` updates, and no place is shown from the drop until the holeshot line. The front slot is not P1 for sitting there. The pre-start countdown ending does not show a place while `session_state` is still 256.
- 2026-10-05 — Open lap after the drop is the grid stagger plus signed metres since the gate. Falling behind the rear slot no longer scores as almost a full lap.
- 2026-10-04 — Open lap after the drop is forward metres from the back of the grid. No place on the gate. A crash without `track_pos` still moves when someone rides past.
- 2026-10-02 — Map and Minimap numbered dots trail the same green/red `*` after the digits.
- 2026-10-02 — A sector gate after the first lap no longer zeroes a healthy rider's metres,
  so places stop swapping as the pack reaches S1 and S2. On lap 1, the stretch just past
  the learned S2 stays ahead of S1 until the split publishes, instead of dropping a full lap.
- 2026-10-01 — Lap 1 places use S1/S2 when those gates are known, and every rider's sector
  split when they are not. A pass between gates still follows track position. After a
  lap is complete the tracker wins. A later sector gate clears a rider pinned to the
  game place.
- 2026-09-26 — First-lap places after start crashes: keep `armed` across brief track-pos
  gaps; cold-arm on first real step when the race is live and the line is unknown so
  far passes show without waiting for S/F.
- 2026-09-25 — Overflow past one lap + slack pins to game place (no S/F fallback); small
  overshoot only clamps. Stops P1→mid-pack when wrap beats `num_laps`.
- 2026-09-25 — Armed lap metres capped to one lap; over that without a `num_laps` bump
  disarms the rider so a lagged line publish cannot invent P1.
- 2026-09-25 — Trailing green/red `*` when live place ≠ on-track place due to penalties
  (Dash, Standings, Relative, H-Standings, board Position / ClassPos).
- 2026-09-24 — Live order is one score per rider: laps and metres on track, minus that
  rider's own penalty. Any penalty size, any place in the field.
- 2026-09-24 — Progress tracker for far pairs (start crashes held riders behind until the
  next gate) and the bubble steps over pinned rows instead of stopping at them.
- 2026-09-24 — `~Lapped` pass latch shares `PAIR_MAX_M` and rejects discontinuous leader
  teleports; see Dash wiki.
- 2026-08-25 — Added. Passes now show on standings, relative, dash, H-standings, and the
  map/minimap dot labels, crown and ahead/behind rings without waiting for the line.
