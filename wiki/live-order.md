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
| Metres into the lap | Tracker distance since the gate or their last crossing, when armed. Otherwise metres past a known start/finish (`SF_FRAC_LEARNED`, or `sf_meters` when it is set). With neither, only a same-lap pair inside 250 m can move. |
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
- A step over `MAX_STEP_M` (80 m) is a reset / shortcut / teleport and adds nothing.
  Dropping out of `riders[]` disarms until the next crossing.
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

## Change log

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
