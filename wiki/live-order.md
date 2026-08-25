# Live race order

The game's classification (`RaceClassification`) is only republished when a rider crosses
the line, so an on-track pass used to sit invisible until the next lap: standings, dash,
map crown and the nearest ahead/behind rings all kept the old places. `race_store.rs`
re-derives the order every tick instead.

## How it works

`live_order` starts from the game classification (`Standing.position`, P1 first) and
bubbles pairs while there is evidence of a pass, then publishes:

- `RaceField.rows` in that order, each `standing.position` overwritten with the live place
  (so `RaceField::board()` is what the boards iterate)
- `LIVE_ORDER` (race numbers) behind `live_position()` / `live_leader()` for map-style
  lookups that only have a race number

A pair swaps only when **all** of this holds (`passed`):

| Rule | Why |
| --- | --- |
| Same `num_laps` | Across lap counts the classification is right by definition, and two riders straddling the line always read a lap apart. |
| Both have a `track_pos` | Riders missing from `riders[]` keep their scored place. |
| Within `PAIR_MAX_M` (250 m) | `track_pos` is a fraction from the centerline origin, **not** from the line, so only the shortest wrap between close riders is trustworthy. |
| Ahead by `PASS_M` (3 m), or `HOLD_M` (0.5 m) if they already hold the place | Hysteresis: running alongside must not swap the row every frame, and `track_pos` is a centerline projection that moves a metre or two on a wide line or a jump. |
| Neither DNS / OUT / DSQ | Not in the running order. |
| Not both scored after the leader finished | A cool-down pass must not move the results. |

The order is rebuilt from the game order **every tick**, not carried forward, so a bad swap
cannot stick — the previous order is only read for the hysteresis margin.

## When it is off

`live_order_active`: needs `on_track`, a real `track_length`, two or more riders, not
`is_warmup` (a practice field is ranked by lap time, not by track progress), and
`IN_GATE == 0` (on the gate everyone sits on the same stretch).

## Do not regress

- Gaps stay the game's numbers. Only `interval` is derived, as a size (`abs`), because a
  fresh pass can leave the pair's gap to the leader the wrong way round for a moment.
- Do not resync the live order to the game whenever the classification changes: it is
  republished when *any* rider crosses the line, and that would undo a pass made since the
  pair's own crossing.
- Do not compare riders more than half a lap apart without knowing where the line is.

## Change log

- 2026-08-25 — Added. Passes now show on standings, relative, dash, H-standings, and the
  map/minimap dot labels, crown and ahead/behind rings without waiting for the line.
