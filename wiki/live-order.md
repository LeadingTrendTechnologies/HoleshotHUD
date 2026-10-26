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
| Metres into the lap | If we saw that rider on the gate: one continuous coordinate, set each tick to where they are now (the unwrap of the current fraction closest to the last coordinate), measured from the back of the stored gate fractions. It is not a sum of steps. A short step behind the origin lands just below 0. A step across the line lands just above the next integer, so the place does not collapse while `num_laps` is still the old value. `num_laps` rising does not reset the coordinate. It only lifts a rider whose coordinate is a full lap behind the classification. A jump over 80 m does not move the coordinate that tick; the next tick that stays there accepts it, unless the bike's world position did not follow the jump — that projection never becomes the place. Once the raw fraction is moving and the body is riding that small step, the frozen coordinate snaps to that fraction. It does not have to cover the gap back to the winding the fold left behind, and it does not short-wrap onto that winding. When three bikes within 45 m sit on the jumped fraction, the coordinate joins their median on the same winding instead of keeping the raw fraction ahead of them. Bikes are on one branch inside the smaller of 80 m and 15% of the lap. That scored fraction starts at the first sample for every rider, gate or not. Sector metres, the start/finish fallback, and the lane check read it. A rider who loses `track_pos` keeps that coordinate, so riding past a crash takes the place. While every stored gate rider is still within 5 m of their slot, or the holeshot has not been called and nobody is 200 m past the back of the grid, there is no place, even if `IN_GATE` has already cleared, and the qualifying position is not shown. Sector geometry does not override a rider we saw on the gate. With no gate snapshot, known S1 and S2 fractions: forward distance from those gates (`split_fracs`, wrap like sectors). The arc from S2 back toward S1 counts as past S2 when the rider is closer to S2 than to S1, and for the whole arc once their sector gate is 2. Closer to S1, with no S2 gate, stays before S1 so the start grid is not a finished sector. A pair under 80 m apart on that arc is ordered by the lap fraction, not by which side of the cut they sit on. On every lap, a world gap that matches the raw fraction is the place, even when the stored coordinate is a sector ahead. Without saved fractions, a published sector gate (1 after S1, 2 after S2) plus travel since that gate. After a lap is complete, tracker distance since the gate or their last crossing, when armed. Otherwise metres past a known start/finish (`SF_FRAC_LEARNED`, or `sf_meters` when it is set). With neither, only a same-lap pair inside 250 m can move. |
| Their own penalty | The game only applies `penalty_ms` to the results. Live, each rider's seconds are metres at session-best pace (`track_length / best_lap`, or 15 m/s before anyone has a lap). Place does not matter: a 10 s penalty and a 5 s penalty are both subtracted from that rider, and the field is sorted once. |
| No `track_pos` | Stay scored from the last coordinate, or from armed lap metres. The bubble can pass them. DNS / OUT / DSQ stay in the scored slot. |
| Ahead by `PASS_M` (3 m), or `HOLD_M` (0.5 m) if they already hold the place | Only a tie break so two bikes on the same stretch do not swap every frame. |
| Centerline samples closer than 12 m, or rider bodies closer than 45 m, and under 45% of an along-track gap of at least 80 m | The next lane, or an over/under. A shorter gap is the pack. `track_pos` is a nearest-point projection, so a bike beside you can look a long way ahead. Bodies are checked even when the published line is missing or more than 25% off `track_length`. Samples are the fallback when a body is missing. On the same lap that rider does not take the place and does not keep it when the game already listed them ahead. A penalty still reorders two riders whose positions are a real distance apart. When three or more same-lap bikes are within 45 m, a fraction on a smaller branch is scored as the larger group's fraction, and a stored coordinate a lap ahead of the bikes on that rider's own branch is pulled back. A group of three or more keeps a corner under 140 m that does not cross the start line. A coordinate behind that pack is not pulled forward onto it. A longer gap, with those bodies still inside the larger pack, is scored as that pack, and a dropped position keeps that spot. That pair is then ordered by the corrected fraction. A fraction a bike further up the lap agrees with, when the world gap matches that spline gap, is not replaced by the lane inside 45 m. |
| Not both scored after the leader finished | A cool-down pass must not move the results. |

The order is rebuilt from the game order **every tick**, not carried forward, so a bad swap
cannot stick — the previous order is only read for the hysteresis margin.

When that order changes, `race.jsonl` gets one `place` line: track, your race number and
live place, then each rider's game place, live place, laps, `track_pos`, and world `x`/`z`.
It is skipped while the place is blank. A change inside 250 ms is written once the window
opens, as the order still in effect. After the moto, that line is in
`%LOCALAPPDATA%\Holeshot HUD\logs\last-race.jsonl`. Each file is one race: a new moto replaces `race.jsonl`, and the finished one replaces `last-race.jsonl`. Leaving the session, loading a different track, or switching between a lap race and a timed session starts that new file. A dropout on the same track keeps this one. That race is kept in full while testing. The feedback upload still clips to 700 KB. Riders a few metres apart in `x`/`z`
but far apart on `track_pos` are the next lane. A swap where those distances agree is a pass.

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
  a one-frame spike is ignored and a sparse update is not dropped. If the bike's world
  position does not follow that jump, do not keep the raw fraction. When three bikes
  within 45 m sit on the new fraction, join their median on the same winding as the
  coordinate you had. Otherwise drop the jump. Bikes on one branch are within the
  smaller of 80 m and 15% of the lap. Dropping out of
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
`ClockMode::Practice` (practice and warmup, including open practice and a long practice with leaked extras, ranked by best lap rather than track progress), and
`IN_GATE == 0` (on the gate everyone sits on the same stretch, and the place stays blank).

## Do not regress

- A same-lap fraction that matches the bikes within 45 m is the place, even when the stored coordinate or the S2–S1 cut says they are a sector ahead. A smaller group on the other branch does not outvote that pack. A fraction a bike further up the lap agrees with, world gap matching the spline, is not replaced by the parallel lane inside 45 m. When the world gap matches the raw lap fraction, that gap is the place on every lap, even if the stored coordinate is a sector ahead. An earlier ignored fold does not keep blocking that gap. A raw fraction the body is riding snaps the frozen coordinate onto that fraction; it does not short-wrap back onto the fold. On the same completed lap the higher fraction is ahead when the bikes match that gap. A gap across the origin counts only when the bikes match it: the direct lead, or the short way once a rider has taken the line. A lead the bikes do not match does not swap the place, on a short lap or a long one. Bikes inside 45 m stay with the pack. A lap the body has already ridden still leads before `num_laps` bumps. A different lap still uses the short way. A second race number on the same bike, under 1 m with the same fraction, does not take the place. That body stays with the better game place. A pass a few metres apart still does. A larger pack does not pull a rider back when the published line between them comes back with the bikes. The higher fraction stays the place.
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
- A pair whose bodies are under 45 m apart (or whose centerline samples are under 12 m
  apart, when a body is missing) and under 45% of an along-track gap of at least 80 m
  is the next lane, not a pass. A shorter gap is the pack. Do not let that projection
  take the place. It only loses a place it already holds when the bodies are under 20 m.
  A gap under 140 m that does not cross the start line keeps the centerline order, including
  under 20 m: that is one corner, not the other side of the track. A 159 m lead stays the other branch.
  A chord near 30 m or 45 m keeps the centerline order. A chord outside 45 m and under
  half the spline keeps the raw lap gap once that leader's body has moved at least 45 m
  from the frozen coordinate: the lap bent, the bike is still ahead. A corner under 140 m
  keeps that raw lead too when the chord is past 45 m and the body has moved, even past
  half the spline. A refused coordinate under 155 m keeps the raw lead once the body
  has walked 45 m, including when the chord folds inside 45 m. A body that stayed
  put is still a projection spike. A chord under 45 m is the next lane only when the centerline samples
  are still farther apart than the bikes. When the line comes back with them, the lead stays.
  A refusal already in the previous
  order holds until the bodies are past 55 m, or the spline gap drops under 80 m.
  Same lap only: a lapped bike beside the leader is not this case. A
  real same-lane lead still moves. A penalty still reorders riders whose positions
  match the gap.
- Two bikes inside 25 m on the stripe, in the world and on the short way, are ordered by those metres when the coordinates already have them together. A classification lap does not make that a lap down. A stored coordinate a lap away does not beat that gap. A real lap up stays ahead: the coordinate gap is about one track length, on a short track or a long one. When the bodies match the raw fraction, that order stands on the same lap too, including past 80 m and past the stripe. A coordinate a lap off that the body did not ride does not veto it, and a stored fold does not either. A matching gap inside one step is the order even when a lap was ridden. A longer raw gap still keeps that ridden lap. Pack pull does not take the earned coordinate back. The higher lap count stays ahead. A close fold does not let the fewer-lap bike pass.
- A spline jump over 80 m that the bike's world position does not follow is not a pass. A second sample of that fraction is still not the place unless the body rode the step.
  When three bikes within 45 m sit on the new fraction, the coordinate joins their median
  on the same winding, and the raw fraction does not take the place. Otherwise the jump
  is dropped, including on the next tick and after a gate snapshot. Bikes count as one
  branch inside the smaller of 80 m and 15% of the lap. The scored fraction starts at the
  first sample. Sector metres, the start/finish fallback, and the lane check read it. A
  jump the body also makes still counts once it stays.
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
- On lap 1, two riders under 80 m apart are ordered by the lap fraction when the
  sector-arc cut scores them the other way. Do not let that cut flip a pack.
- Do not publish a place while `in_gate`, while `session_state` is 256, while every stored gate rider is still within
  5 m of their slot, or after that gate until the holeshot is called (or the lead is 200 m
  past the back of the grid). Qualifying order is not a seed for the first lap, and it is not the
  place shown when live order is blank.
- Falling behind the rear gate slot lands just below 0. Do not wrap that step into almost a full lap.
- Do not reset the gate coordinate when `num_laps` rises, and do not switch that rider to
  `travelled_m - lap_base_m`. A line cross before the bump keeps the lead.
- A crashed rider who loses `track_pos` keeps their last coordinate and can be passed, or
  can be the one passed. Do not pin them in the classification slot.
- On the opening lap (`num_laps == 0`), a place change of more than one spot does not publish until the next sample still wants that order. A one-spot pass publishes immediately.
- A lap count that arrives with no `track_pos` stays just past the line (`STRIPE_M`) until a fraction returns. Pack pull does not move that pin. A bike at fraction ~0 does not pass them.
- A close pair on the long way around the lap does not take that place until the start/finish line is known. Once the line is known, bodies inside the 20 m strip still give that place to the rider the short wrap has behind.
- An exact `track_pos` of 1, or exactly one track length in metres, stays at the end of this lap. A bike at 0.90 does not pass it. A real wrap to about 0.01 stays ahead.
- On the opening lap a raw fraction under 80 m takes the place over a stored coordinate the body did not ride. A bike that stepped back beside the pack does not keep that lead.
- On the opening lap a raw gap under 80 m takes the place even when the score already agrees. A stored reading of the next lane does not cancel that pack gap. A gap of 700 m with the bikes 9 m apart is still the next lane.
- On the opening lap a same-side branch at least 80 m up the spline does not take the place while the bodies are still inside 45 m. The game order holds for that pair. A wrap across the start line is not that branch. A gap under 80 m whose bodies match still passes, on a short lap and on a 1748 m one.

## Change log

- 2026-10-20 — Lake Elsinore's opening lap flashed P2 and P10 while the game held P7. The pack was inside 35 m and `track_pos` was split by about 260 m. That branch does not take the place. A bike 7 m up the lap and 9 m away still passes.
- 2026-10-19 — Putra left you behind #934 for the extra lap while you were 96–124 m up the lap and the bikes matched that gap. The coordinate was still ignored from a 20 m chord. A matching gap is the place past 80 m too. A lap the bodies do not match still leads. A penalty still moves the place.
- 2026-10-15 — The Island dropped the race leader for about 3 s while he was 143–246 m up the lap and 90–180 m away. A chord of at least 80 m and more than 0.55 of that gap keeps the raw lead once the body has moved, past the 140 m corner. A 45 m chord at 159 m stays the other branch.
- 2026-10-15 — Forest dropped the race leader for about 2 s while he was 114–143 m up the lap and 22–66 m away. The coordinate was frozen and the chord had folded inside 45 m. A refused coordinate under 155 m keeps that raw lead once the body has walked 45 m. A lead from 159 m up stays the other branch.
- 2026-10-15 — Forest flashed P1 for about 2 s while the leader was 119–127 m up the lap and 51–102 m away. A corner under 140 m keeps that raw lead when the chord is past 45 m and the body has moved, even past half the spline. A chord inside 45 m stays a fold. A lead from 159 m up stays the other branch.
- 2026-10-14 — Wolf Creek dropped the race leader to P8 on the run to the line: about 400 m up the lap, 110–180 m away in the world, because the hairpin froze the coordinate. A chord outside 45 m and under half the spline keeps that raw lead once the body has moved 45 m. A same-lap gap the bodies match, inside one step, is the order even when the other coordinate is a lap the body rode. A longer raw gap still keeps that lap.
- 2026-10-14 — Paleta left a bike 20–185 m ahead, with the bodies on that fraction, ranked behind for many seconds. The other bike's coordinate was a lap off. A same-lap gap the bodies match is the order, including past 80 m. A coordinate the body did not ride to, and a stored fold, do not veto it. A lap the body rode still leads. A 45 m chord on a 185 m spline stays the other branch.
- 2026-10-14 — MXB Club flashed a place on a 125–131 m corner when the bikes crossed 20 m, pulled a bike on the returning lane onto the finish pack, and ranked a lap-down rider ahead for several seconds. A corner under 140 m keeps the centerline order. A coordinate behind the pack stays behind. The higher lap count stays ahead when the bodies match the fraction, and on a close fold.
- 2026-10-10 — Farm14's holeshot jumped from P16 to P9 while the bike moved a few metres and the pack was still inside 70 m. The raw gap agreed with the score, and the stored lane cancelled it. On the opening lap a raw gap under 80 m takes the place. A short lap and a long one match.
- 2026-10-09 — Monster Mountain's holeshot jumped from P7 to P17 while the pack was still inside 30 m and the bike was still at speed. The raw fraction had stepped back, and the stored coordinate kept the old lead. On the opening lap that raw gap takes the place. A short lap and a long one match.
- 2026-10-09 — Meadow Valley's opening lap jumped several places in the last 50 m when a bike published `track_pos` of exactly 1. That frame stays at the end of the lap, so a rider at 0.90 does not pass it, and a real wrap to 0.01 stays ahead. A short lap and a long one match.
- 2026-10-09 — A close pair does not take the long way around the lap until the start/finish line is known. The standings fixture was handing P1 to the bike at fraction 0.92 while the line was still unknown.
- 2026-10-09 — RedBud's first minute was jumping several places while the bike moved a few metres. On the opening lap a change of more than one spot waits for the next sample. A pass that is still there publishes. One spot still moves immediately.
- 2026-10-09 — RedBud's leaders lost `track_pos` on the lap the count rose and sorted behind a bike at the stripe. That dropout stays just past the line until a fraction returns, and the pack pull does not move it.
- 2026-10-09 — A larger pack does not pull a rider back when the published line between them comes back with the bikes. 755 Compound stored the race leader on the pack for a long stretch of the first lap: the hairpin chord was under 45 m while the fraction was still over 120 m up the lap. The higher fraction stays the place, on a short lap or a long one.
- 2026-10-09 — A body shared by two race numbers stays with the better game place. Temecula published a backmarker on the leader for the whole moto, and the copy sat in P2 while the lap counts matched. The other rider stays on the game order. A side-by-side pass is further than a metre, on a short lap or a long one.
- 2026-10-09 — A same-lap gap across the origin takes the place only when the bikes match that lead. Ezkutu put riders still before the line in front for several seconds after a crossing, about 100–200 m away. The higher fraction still leads when the bikes are that far apart. The rider just past the origin leads when the bikes match the short way. A short lap and a long one match.
- 2026-10-09 — On the same completed lap the higher fraction is ahead. The gap is that fraction delta, so a rider at 0.12 does not pass a rider at 0.80 by wrapping across the origin. A pass of a few metres still takes the place. A short lap and a long one match. A rider who has taken the line is still a lap up.
- 2026-10-09 — A fold that freezes the coordinate does not keep that bike behind once the fraction and the bodies agree. Winchester showed P10 while the lap fraction and the bikes 28 m ahead both said P2. The raw fraction snaps onto the step the body is riding, and that gap is the place even if an earlier jump was ignored.
- 2026-10-08 — A classification lap on the stripe does not move the place by a lap. Washington dropped a place when the bike 12 m ahead got `num_laps` first, then jumped back ahead of that bike once the count caught up. The window is 25 m in the world and on the short way, so a short track and a long one match. A real lap up stays ahead. A fraction that holds for a second sample is the place only when the body rode the step.
- 2026-10-08 — A folded gate is not the holeshot. Briarcliff showed P2, and a bike with no position in P1, while the field was still together.
- 2026-10-08 — A dropped position does not take the place. Pala put a bike at P1 when its position vanished, ahead of riders about 200 m up the lap.
- 2026-10-08 — A group of bikes 120 m or more up the lap, still inside the larger pack, is scored with that pack. Maryland's holeshot put four of them in P1–P4, then froze that spot when the position dropped and showed P2 from the back of the grid.
- 2026-10-08 — A corner of about 80 m keeps the place when the bikes are 20–40 m apart across it and the gap does not cross the start line. Forest was flashing one place for about a second.
- 2026-10-08 — An unridden jump across a fold does not drop the place while the bike is still beside that pack. Nevada was flashing from the pack to the back for a few seconds when the fraction jumped ~400 m and the body moved ~2 m.
- 2026-10-08 — A lap race and a timed session on the same track are two files. A clock dip at the finish of a lap race still stays in that race's file.
- 2026-10-08 — An unridden spline jump that lands on three bikes within 45 m joins their median, on the same winding, instead of keeping the raw fraction ahead of the pack. One branch is the smaller of 80 m and 15% of the lap, so a short track and a long one agree. MXB Club was flashing P1 off the gate when the fraction jumped ~200 m and the bike moved ~2 m.
- 2026-10-07 — A spline lap the body did not ride does not take the place. The crossing still counts when the body travel matches the coordinate, within a quarter of that lap.
- 2026-10-07 — A lap-down coordinate does not take the place unless the travel since the last counted lap covers the laps they are short. A short track and a long one use a quarter of that lap.
- 2026-10-07 — A coordinate a lap ahead of the bike does not take the place. The higher lap keeps it while that rider is still a short gap ahead around the lap, and a same-lap jump counts only when the travel matches.
- 2026-10-07 — A rider with fewer completed laps does not take a place by a higher fraction on the earlier lap. The place still moves when their coordinate has reached the higher lap.
- 2026-10-07 — A chord under 45 m is the next lane only when the centerline samples are still farther apart than the bikes. When the line comes back with them, that spline lead stays, on a short lap or a long one. A different track after the session ends starts a new race file.
- 2026-10-07 — A same-lap bike outside 45 m keeps a real `track_pos` lead. A short chord on a looping lap was throwing that gap away and sorting them behind you for a moment.
- 2026-10-07 — A folded centerline gap only takes a place away when the bodies are under 20 m. Washougal was flashing a place when a bike sat about 30 m away and 80–210 m up the lap. A lead already refused near 45 m stays refused until the bodies are past 55 m.
- 2026-10-07 — A folded centerline gap only takes a place away when the bodies are under 30 m. Farm14 was flashing P2 or P4 when a bike sat about 45 m away and 120–330 m up the lap, then snapping back. A bike under 45 m still cannot take a place with a long false `track_pos`.
- 2026-10-06 — When the world gap matches the raw lap fraction, that gap is the place on every lap, even if the stored coordinate is a sector ahead. Wilderland was leaving riders several places back while the bikes ahead of them were 70–300 m up the lap and about that far away.
- 2026-10-06 — A fraction a bike further up the lap agrees with, world gap matching the spline, is not replaced by the parallel lane inside 45 m. Ezkutu was dropping a place to that lane for a few seconds while the leader sat ~95 m ahead.
- 2026-10-06 — Live place uses the lap fraction the same-lap bikes within 45 m agree on. A stored coordinate or an S2–S1 cut a sector ahead of that pack is pulled back, on every track.
- 2026-10-06 — A same-lap fold does not keep the place when the game already listed that rider ahead. Maryland's holeshot showed a few bikes as the leaders while their lap fraction sat ~360 m away and the body stayed in the pack.
- 2026-10-06 — On lap 1, riders under 80 m apart stay in lap-fraction order when the sector-arc cut scores them a sector apart. Spring a Ding was flipping a place between P6 and P13 while the bike moved a few metres.
- 2026-10-06 — A bike under 45 m away does not take the place when `track_pos` says they are much further up the lap. Millcreek was dropping a place to someone 40 m away and 120–260 m ahead on the centerline.
- 2026-10-06 — The scored lap fraction is kept from the first sample, with or without a gate. A spline jump the bike does not follow no longer moves a place on a later lap. `race.jsonl` and `last-race.jsonl` keep the whole session while testing; the feedback upload still clips to 700 KB.
- 2026-10-06 — A centerline gap under 80 m is still a pass, even when the bikes are close in the world. A spline jump the bike does not follow never becomes the place. Briarcliff was dropping places in the start pack and flicking one place when a fraction jumped ~600 m.
- 2026-10-06 — A bike under 40 m away does not take the place when `track_pos` says they are much further up the lap. Farm14 was dropping a place to someone 9–24 m away and ~700 m ahead on the centerline.
- 2026-10-06 — A `place` line in `race.jsonl` records the live order when it changes (game place, live place, laps, `track_pos`, world `x`/`z`). Send `last-race.jsonl`.
- 2026-10-06 — A rider on the next lane of a folded track does not take the place. The centerline samples have to be about as far apart as the lead.
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
