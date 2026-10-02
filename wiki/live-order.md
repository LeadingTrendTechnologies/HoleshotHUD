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
| Metres into the lap | On lap 1 (`num_laps` still 0), known S1 and S2 fractions: forward distance from those gates (`split_fracs`, wrap like sectors). The arc from S2 back toward S1 counts as past S2 when the rider is closer to S2 than to S1, and for the whole arc once their sector gate is 2. Closer to S1, with no S2 gate, stays before S1 so the start grid is not a finished sector. Without saved fractions, a published sector gate (1 after S1, 2 after S2) plus travel since that gate. After a lap is complete, tracker distance since the gate or their last crossing, when armed. Otherwise metres past a known start/finish (`SF_FRAC_LEARNED`, or `sf_meters` when it is set). With neither, only a same-lap pair inside 250 m can move. |
| Their own penalty | The game only applies `penalty_ms` to the results. Live, each rider's seconds are metres at session-best pace (`track_length / best_lap`, or 15 m/s before anyone has a lap). Place does not matter: a 10 s penalty and a 5 s penalty are both subtracted from that rider, and the field is sorted once. |
| No `track_pos`, or DNS / OUT / DSQ | Stay in the scored slot. The bubble steps over them. |
| Ahead by `PASS_M` (3 m), or `HOLD_M` (0.5 m) if they already hold the place | Only a tie break so two bikes on the same stretch do not swap every frame. |
| Not both scored after the leader finished | A cool-down pass must not move the results. |

The order is rebuilt from the game order **every tick**, not carried forward, so a bad swap
cannot stick — the previous order is only read for the hysteresis margin.

## Penalty vs on-track place

While live order is on, a second ranking uses the same bubble with penalties ignored
(`TRACK_ORDER` / `track_position`). When a rider's live place and on-track place differ,
boards paint a trailing `*` on the place: green (`ahead_col`) when live is ahead of
on-track, red (`behind_col`) when behind. Dash hero `P#`, Dash/board Position (and
ClassPos), Standings, Relative, and H-Standings all use it. Map dot numbers stay plain.

## Progress tracker

`PROGRESS` keeps, per race number, metres covered (`wrap_signed` step × `track_length`
each tick) and where their current lap started:

- On the gate (`clock.in_gate`) everyone is zeroed and **armed**: lap 0 progress is metres
  since the gate. This is what fixes the start, where the game keeps the gate order and
  riders down in turn one are soon more than 250 m back.
- When a rider's `num_laps` rises the lap base moves to the current distance and they are
  armed. Spectating / joining mid-race arms each rider on their first crossing we see.
- On lap 1, a sector-gate rise moves the lap base (travel since that gate) and clears a
  corrupt pin. After a lap is complete, a gate rise clears a corrupt pin only — a healthy
  armed rider keeps their metres. The gate count dropping at the next lap does not disarm
  them.
- A step over `MAX_STEP_M` (80 m) is a reset / shortcut / teleport and adds nothing.
  Dropping out of `riders[]` freezes distance and keeps `armed`; on reappear travel
  resumes without inventing a teleport step.
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
`IN_GATE == 0` (on the gate everyone sits on the same stretch).

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

## Change log

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
