# Dash

Gear, RPM, speed, position, and the session clock / lap counter. Race **flags** are a banner **above** this widget, not side panels. Settings subtitle: “Gear, speed, and footer stats”.

This is the most stateful widget. Clock and flag bugs almost always belong here, not in Standings.

## Code

- Draw: `draw_dash`, `draw_simple_dash`, `draw_dash_wrap`, `draw_rev_bar` in `overlay/hud/src/render.rs`
- Clock: `session_remain_ms`, `session_banner`, `is_lap_race`, overtime helpers in `overlay/hud/src/race_store.rs` (`RaceStore::tick` / `get`)
- Flags: `dash_race_flag`, `laps_left`, `leader_finished` / `race_over_for_me`
- S/F geometry: `approaching_line`, `final_lap_approach`, `note_line_progress`, `sf_frac`
- Trace: `clock_sample` → `%LOCALAPPDATA%\Holeshot HUD\logs\race.jsonl`
- Settings: `pane_dash` in `overlay/src/settings.rs`
- Footer fields: `DashField`

A `~Lapped` tag sits beside the lap/clock text whenever `lapped` is true — the focus rider's classification `gap_laps >= 1`, gated on being on track, out of warmup, and off the gate. It widens the right column, so the panel grows once when you get lapped rather than clipping. Amber (`dash_lapped_col`), a little smaller than the lap text and nudged onto its baseline.

Fixed body: gear | RPM + speed | position + lap/clock text. The plaque fills the widget rect: height scales type, extra width is column gap. Hold Ctrl and drag to resize. Default 11.1%×11.5%, bottom-centered. **Simple dash** (`dash_simple`) strips that to a compact lockup: orange skew **gear plaque** (ink-on-accent digit) plus large italic speed with a vertical `MPH`/`KPH` stack. No RPM, place, footer, or rev bar. White/checkered flags still wrap the plaque. The RPM/speed column is sized from the widest digits (`0`–`9`), not the live value, so gear and position do not shift as RPM changes. **P#** and `~Lapped` use live `RaceStore` rank/gaps during a race; session clock still uses game laps. `P#` reads `standing_pos`, so it counts an on-track pass straight away instead of at the line — see [live race order](../live-order.md). It also falls back to `local_race_num` when there is no focus rider. Footer is three slots (default Engine, Air, Best); options include **Local time** (`DashField::LocalTime`, same 12h clock as standings), **Fuel** (`DashField::Fuel`, liters or US gallons from Units), **Fuel %** (`DashField::FuelPct`), and **Setup** (`DashField::Setup`, loaded bike setup filename). Optional rev bar from `local_rpm` vs `max_rpm` / `shift_rpm`. Footer and rev stay in the ini while Simple dash is on.

## Session clock (hard-won)

Practice, gate, and race time share **one** slot (`session_banner`).

- **Lap moto:** `session_laps >= 4` is always a lap race, even if leftover warmup `10:00` or a leftover timed length (`8:00`) is still in `session_length`. 2- and 3-lap motos are lap races when length is not leftover practice or a standard timed length (5–30 min set). A leftover start board (`00:50` stored as length) is still `1 / 2`, not `0 / +2`. Leaked extras during warmup (length 0, live `05:00`) stay a countdown. If a live **5–30 min** clock has already been seen with 1–3 extras and length is unset/start-board (`TIMED_EXTRAS_HINT`), stay timed — do not flip to `1 / 2` after the gate (6:00+2).
- Lap motos show `current / N` after green, not leftover minutes. Gate boards (about 8 s–2 min) still show a countdown until the clock runs up or you move. A later 45 s / 30 s board after `00:10` stays a countdown; leftover `08:00` must not replace it until the race clock actually ticks.
- Timed race: countdown while `session_time_ms` is live (`07:32` only — no `+#` while the clock runs). When time expires, extras use `0/1` then `1/1` for +1, or `0/2` … `2/2` for +2. Crossing as a backmarker at time-zero does **not** start extras (`local_overtime_taken`). `0/1` is the uncounted lap after the clock; `1/1` is the extra. It must advance — do not stick on `1/1` for laps you still have to run.
- Clock stays `00:00` until you cross or the leader puts a lap on you (0.1.0).
- Warmup `10:00` must not stick after a race: prefer the ticking clock (0.1.4).
- Warmup/practice countdown hides when time hits zero (blank — not sticky `00:00` / `00:30`).

Plugin session fields are messy (warmup length leaking into race). Lots of atomics (`IN_GATE`, `SESSION_EXPIRED`, `OVERTIME_*`, `LAP_GREEN`) exist because the API does not send a clean mode enum. `session_kind` / `session_state` are in SHM (version 9). Logged: warmup `kind=5`; race 2 `kind=7` for **both** 8:00 +1 and a 4-lap moto. `state=16` is running; `state=256` is the start gate. Race 1 not dumped yet (likely `kind=6`). **Warmup vs race** uses kind when it is present (`>= 6` is a race, so a 15:00 +1 with unpublished extras is not warmup). Lap vs timed is still `session_laps` + `session_length`. A kind change (warmup `5` → race `7`) drops clock latches so a finished 15:00 warmup cannot inherit as `0/1`. Plugin length is **`-1` until this session writes it**; a length change drops cached length/laps so leftover `8:00` cannot stick when the new session publishes `0`. Start boards and leaked warmup extras are still real API writes, not cache.

## Flags

Banner above the dash only (`draw_dash_wrap`). No striped side panels over the widget (reverted). The **Flags** widget is a separate cloth that uses the same `dash_race_flag` result for white/checkered; turning it on does not hide this wrap. Yellow and blue never wrap the dash.

One path for lap motos and timed extras, driven by `laps_left`. Lap motos count `session_laps - laps_done`. Timed extras count down to the lap total you finish on, `OVERTIME_LOCAL_BASE + 1 + extras`, and stay `None` until the leader starts extras. Do **not** derive the timed count from `extras - local_overtime_done`: that clamps at zero and so reports the same "2 left" on the uncounted lap and on your first extra, which fires white a lap early.

- No flags on the gate, while stopped on a prestart, or while a long countdown is still running.
- **Checkered latches from one thing only:** `laps_left == 0`, which happens when you cross the line with nothing left to run. Never speed-gated, so a slow roll over the line still gets waved off. It **latches** until you leave the session (`on_track == 0` clears it) and must not carry into the next warmup (0.1.4). It also **shows** unlatched on the finish run-in (`laps_left == 1` and `line_approach`), the way a flagger has it out before you reach the line — losing the window (stopping, bad geometry) drops it again, and only the crossing makes it stick.
- **Checkered has to be earned** (`finish_earned`): you must have completed a lap since the last frame that reported laps still to run (`LAPS_TO_RUN_AT`). Session fields glitch often enough that a single frame reading `laps_left == 0` would otherwise latch the flag mid-race — a timed set momentarily looking like a lap moto puts extras (`2`) against real lap counts (`6`), and `session_laps - laps_done` clamps to `0`.
- **White is a wave, not a state.** It goes up on the run-in that starts your final lap (`laps_left == 2` and `line_approach`) and comes down `WHITE_WAVE_MS` (5 s) after the lap starts — `white_wave` stamps `WHITE_WAVE_AT` on the first frame of a lap that calls for white (`WHITE_WAVE_LAP` keys it to that lap, so the repeated `dash_layout` calls in a frame do not restamp it). The mid-lap stretch is deliberately bare; the checkered comes back for the finish run-in.
- **Lapped riders:** once `leader_finished` the race is over, so the white is waved the moment the lap you are on becomes your last, and the checkered comes on your next crossing (`race_over_for_me`), even a lap short. `leader_finished` is `session_laps <= leader_num_laps <= session_laps + 2` on a lap moto (the upper bound rejects a glitched frame where extras masquerade as a lap count; nobody runs more than a cool-down lap past the finish), or `leader_num_laps >= overtime_base + 1 + extras` on a timed set. `LEADER_FIN_LOCAL_BASE` latches your lap count at that moment.
- **The total shrinks to the race you run.** `effective_race_laps` is `LEADER_FIN_LOCAL_BASE + 1`, clamped to the distance: getting lapped in a 5-lap moto reads `4 / 4`, not `4 / 5`. The clamp matters — your own finish latches the base too, so a winner would otherwise read `5 / 6`. `effective_extra_laps` does the same for timed extras against `OVERTIME_LOCAL_BASE`, and `1/2` collapsing to a single extra shows as `0/1` then `1/1`. `laps_left` uses the effective total on lap motos, so a lapped rider's flags fall out of ordinary lap counting instead of a special case.
- **The run-in flag stays out across the line** (`hold_across_line`, `RUN_IN_FLAG`, `across_the_line`). Your lap count only catches up a frame or two after you are past the line, and until it does the lap rules still describe the lap you just finished. The approach window also stops `FLAG_LINE_MIN_M` (4 m) short of the line, so without holding that stretch the banner was None long enough for the hide animation to start and then grow again on the other side. The held flag lasts until the count catches up or you leave that window, and a latched checkered supersedes it. Coming back to the same flag after a brief None does not replay the grow (`flag_anim_step`).
- **Geometry only decides the run-ins.** `line_approach` needs the 4–80 m window (`FLAG_LINE_MIN_M` / `FLAG_LINE_M`), a mid-lap sighting (`LAP_MID_SEEN`), a closing step (`CLOSING_ON_LINE`), and a known line. The latched checkered and the white wave are pure lap counting, so bad track data costs you the run-in flags and can never latch a wrong finish.
- **Where the line is:** `sf_frac` prefers a position learned from your own lap crossings over `sf_meters`, which sits at `0` whenever the game sends no centerline and would park the window at the centerline origin instead of the timing line. Two crossings must agree within 5% of a lap (`SF_AGREE_FRAC`) before the learned value is trusted, so a rejoin or a stray lap increment cannot move the window.

## Do not regress

- Do not put flags on the sides of the dash again.
- Do not treat leftover `10:00` or leftover `08:00` session length as a timed race when `session_laps >= 4`.
- Do not keep plugin `session_length` across a session kind change; unset is `-1` until this session writes a value (`0` means the game sent 0).
- Do not treat a leftover start board (`00:50`) as timed `+2` extras when `session_laps == 2`.
- Do not treat a live 6:00+2 (or any 5–30 min +2/+3) as a 2-lap moto when length is 0 or still a start board after the gate (`TIMED_EXTRAS_HINT`).
- Do not let a leftover start-board length cap the live timed countdown (`effective_session_len_ms`).
- Do not treat leaked extras during a live warmup clock (length 0, `05:00`) as a 2-lap moto.
- Do not replace a later start board (`00:45` after `00:10`) with frozen `08:00` unless a live remaining clock has already been shown.
- Do not stick 8:00+1 on `1/1` for laps still to run. After the clock: `0/1` until you start the extra, `1/1` on that lap (clock alone while time remains — no `08:00 +1`).
- Do not lower `OVERTIME_LOCAL_BASE` / `OVERTIME_BASE_LAP` when standings reset to 0 after extras appear late on an 8:00+1. Those counts are high-water marks from the timed lap.
- Do not append `+#` to the live timed countdown; extras text is only after the clock hits zero.
- Do not start extras from a backmarker cross at time zero; wait for the leader.
- Do not count the lap you are running when the leader starts extras. It does not count, so `8:00 + 2` from a mid-lap expiry is one uncounted lap then two extras — three crossings, and `laps_left` is `3` on the uncounted lap.
- Do not let one glitched frame wave you off. Checkered needs `finish_earned` (a lap completed since laps were last known to remain), because `session_length` dropping out for a frame makes a timed set read as a lap moto.
- Do not read `OVERTIME_LOCAL_BASE` without calling `note_overtime_base` first. The pre-extras re-basing used to be a side effect of `local_overtime_done`, so the base was only correct if something happened to format the banner that frame.
- Do not **latch** checkered from the S/F approach on either race type. The run-in shows it, but only a line crossing with `laps_left == 0` sets `CHECKERED_LATCH`, so a rider who stops or crashes in the window is not finished.
- Do not let track geometry decide the latched checkered. The approach window only gates the run-in flags, so a bad `sf_meters` or missing centerline degrades to "no run-in flag", never to a wrong finish.
- Do not hold the white up for the whole final lap again. It is waved at the line for `WHITE_WAVE_MS`; the bare mid-lap stretch is the point.
- Do not read the lap rules as authoritative in the frames just after the line. The classification lags the crossing, so a flag that was out on the run-in has to be carried across it (`hold_across_line`).
- Do not show white on timed extras before the leader starts them (`laps_left` is `None` until `extras_started`).
- Do not leave a lapped rider without a checkered. When `leader_finished`, the race is over and the next crossing ends it (`race_over_for_me`), even a lap short.
- Do not count a finisher up past what they ran; once `i_finished`, the banner freezes.
- Do not let the effective total exceed the race distance. Your own finish latches `LEADER_FIN_LOCAL_BASE` at your full lap count, so an unclamped `base + 1` gives a winner `5 / 6`.
- Do not show `~Lapped` in warmup, on the gate, or off track. `gap_laps` has no meaning without a race leader.
- Do not derive `LapsLeft` from the banner text. It comes from `laps_left` and counts the lap you are on, so the final lap reads `1`.
- Do not trust a single observed lap crossing as the S/F position; two must agree (`SF_AGREE_FRAC`).
- Do not leave a sticky `00:00` / `00:30` on the dash after warmup/practice expires; hide the clock until the next session.
- Do not keep warmup `SAW` / `ARMED` when `session_laps` first go from 0 to extras after a practice-like length (`0` / 10–20 / 30+ min). That forces early `+1` with no live `MM:SS` countdown on 5:00+1 / 8:00+1. Do **not** apply that reset when extras appear on the **same** 10–30 min timed race you are already armed/expired on — that is late +1, not warmup ending, and wiping overtime bases sits on `0/1` with checkered while laps remain.
- Do not treat a race session (`session_kind >= 6`) as warmup just because length is 10/15/20/30 and extras are unpublished.
- Do not throw away a 30:00 timed length as leftover practice when extras are 1–3. 40+ min leftover practice still drops.
- Do not call `session_remain_ms` more than once per frame from `RaceStore::tick` + dash; widgets read the tick banner.
- Session clock / overtime / flags still use game classification laps, not live rank.
- Do not drop flag wrap in Simple dash. White and checkered still sit on the compact plaque.
- Do not paint Simple dash gear in lapped-red; the gear tile is Holeshot orange with dark ink.
- Do not throw away footer / rev settings when Simple dash is on; they come back when it is off.
- Do not lock dash size to a content-only visual rect. Orange handles and hit testing use the widget rect; the plaque fills that rect. Hold Ctrl and drag to scale. Default is 11.1%×11.5%, bottom-centered.
- Do not paint yellow or blue on the Dash wrap. Those exist only on the Flags widget, behind **Yellow flag** / **Blue flag**.
- Fuel footer is liters/US gallons (`Fuel`) or tank percent (`Fuel %`). Empty volume is `0.0 L` / `0.0 gal`; `--` / `--%` only when tank size is missing.
- Setup footer is the `RunInit` filename stem (path and `.xml` / `.mxb` stripped). `--` when the name has not arrived (replay / spectate never call `RunInit`). Restart MX Bikes after the V13 plugin so `setupName` is in SHM.

## Change log

- 2026-09-03 — **Setup** is a footer option (`DashField::Setup`). Filename from `RunInit` `m_szSetupFileName`. Restart MX Bikes after this plugin so SHM `Local\MXBOHudV13` loads.
- 2026-09-03 — 15:00+1 (and 10/12/20/25/30) that publishes extras late no longer resets the session clock. Those lengths looked like warmup/practice, so `+1` arriving wiped overtime bases and the dash sat on `0/1` with the checkered while you still had laps to run. Kind `>= 6` is a race even with unpublished extras; warmup `5` → race `7` still clears latches. 25:00+2 stays timed, not a 2-lap moto. 30:00 is a moto length, not leftover practice.
- 2026-08-30 — Default size is 11.1%×11.5% (the in-game lockup we settled on). Untouched 11.5%×10.8% factory rects migrate; a custom placement is left alone.
- 2026-08-30 — 8:00+1 that publishes extras late and resets standings to 0 no longer sticks on `1/1` or latches checkered three laps early. Overtime bases are high-water marks from the timed lap. Banner is `0/1` until you start the extra, `1/1` on that lap.

- 2026-08-29 — Fuel reads as liters or US gallons from Units, not percent.

- 2026-08-29 — Fuel level is a footer option (`DashField::Fuel`). Same tank percent as Standings / Relative / H-Standings.

- 2026-08-29 — Dash wrap ignores yellow/blue. Those flags are Flags-widget only (`flag_yellow` / `flag_blue`).
- 2026-08-28 — Flags widget shares `dash_race_flag` / `flag_anim_step`. Dash wrap stays when Flags is on.
- 2026-08-28 — Website demo starts Dash at 16% width so **~Lapped** fits. Simple dash snaps to a compact 9%×8% lockup (overlay default stays 11.5%×10.8%).
- 2026-08-27 — Default dash is 11.5%×10.8% (the size we settled on in-game). The plaque fills the widget rect so Ctrl-drag on the orange handles actually resizes it.
- 2026-08-26 — Simple dash is gear + speed only: orange skew gear plaque, vertical unit stack, flags still wrap. Footer and rev hide in settings but stay in the ini.

- 2026-08-25 — Checkered wrap stays around the dash (sides and bottom), using the same soft grey squares as the top. Checkers fade in from both sides and hug the caption; Font Awesome `flag` sits with the label. White flag uses the same icon.
- 2026-08-25 — The last `FLAG_LINE_MIN_M` before the line is part of the hold (`across_the_line`). That stretch used to be None, which started the hide animation and made the checkered collapse as you crossed. Same-kind resume in `flag_anim_step` does not replay the grow.
- 2026-08-25 — Flags follow the flagger. Both go up on the run-in to the line: white onto your final lap, checkered onto the finish (`Some(1)` + `line_approach`, unlatched — `final_lap_approach` is now `line_approach` since both use it). The white is a wave, held `WHITE_WAVE_MS` (5 s) from the first frame of the lap that calls for it (`white_wave`, `WHITE_WAVE_LAP` / `WHITE_WAVE_AT`) instead of sitting on the dash from the crossing to the finish.
- 2026-08-24 — **P#** and `~Lapped` follow live `RaceStore` rank/gaps during a race; session clock still uses game laps.
- 2026-08-25 — `P#` counts an on-track pass immediately (`standing_pos` → live order) instead of waiting for the game to rescore at the line, and falls back to `local_race_num` when there is no focus rider.
- 2026-08-25 — A start board republished into the session clock mid-moto no longer ends the race. `DIP_FROM_CLOCK` remembers the countdown a long single-frame drop came from; coming back within 30 s of it is the clock resuming, so the expiry rules stay off, while a real expiry still returns at the session length. The dip can still show for a frame or two. Seen on a `5:00 + 2` as `04:43` → `00:05` → `04:42`, which used to latch `SESSION_EXPIRED` and sit on `0 / 2` for the rest of the moto.
- 2026-08-25 — `build_field` is read-only for session state: it takes the `SessionClock` the tick already built instead of calling `is_warmup` / `leader_finished` / `effective_race_laps`, which note and arm latches. The clock owns them and runs first.
- 2026-08-24 — `~Lapped` tag beside the lap/clock text while a lap or more down (`lapped`, from classification `gap_laps`).
- 2026-08-24 — Getting lapped shortens the lap total instead of freezing under the moto distance: `4 / 5` is now `4 / 4` (`effective_race_laps`), and timed extras shrink the same way (`effective_extra_laps`). Clamped to the distance so a winner does not read `5 / 6`.
- 2026-08-24 — Fixed white/checkered flicker on timed extras (`8:00 + 2`). Two causes: the timed `laps_left` lost a lap to the `max(0)` clamp in `local_overtime_done`, so white fired on the run-in of the uncounted lap as well as the real one; and checkered latched from a single frame of `laps_left == 0`, which a glitched `session_length` produces by reading the extras count as a lap distance. Timed `laps_left` now counts down to `OVERTIME_LOCAL_BASE + 1 + extras`, checkered requires `finish_earned`, `leader_finished` is bounded on lap motos, and `note_overtime_base` makes the pre-extras re-basing explicit instead of a side effect of the banner. `race.jsonl` now logs `flag` and `left`.
- 2026-08-24 — Checkered now requires crossing the line on both race types, so the finish run-in stays white. Lapped riders get white once the leader finishes and checkered on their next crossing, with the banner frozen on the laps they ran. White also covers the run-in that starts the final lap. Removed the approach-timing apparatus that only existed to hold checkered back (`last_lap_cleared`, `note_last_lap`, the white hold, `LAST_LAP_READY` / `LAST_LAPS_LEFT` / `WHITE_HOLD_T0`).
- 2026-08-24 — S/F position is learned from your own lap crossings (two must agree) instead of trusting `sf_meters`, which is `0` with no centerline and would put the flag window at the centerline origin. The early white also needs a mid-lap sighting and a closing step, and is suppressed outright when the line is unknown.
- 2026-08-24 — `LapsLeft` reads from `laps_left` rather than parsing the banner, and counts the lap you are on (`1` on the final lap, was `0`).
- 2026-08-24 — Lap-moto checkered no longer fires when last lap starts (`4/4`); white shows first, checkered only after clearing S/F then approaching finish.
- 2026-08-24 — Warmup/practice clock hides after expiry (no sticky `00:00` / `00:30`).
- 2026-08-24 — Live timed countdown is clock-only (`07:32`); `+#` / `N / +M` only after expiry.
- 2026-08-24 — 6:00+2 with unset/start-board length no longer flips to `1 / 2` after the gate; sticky timed-extras hint from a live 5–20 min clock.
- 2026-08-24 — Timed white/checkered live on `RaceStore.clock.flag` from timer + extras lap count after expiry (no S/F window). Lap motos keep approach-based flags.
- 2026-08-24 — Warmup → 5:00+1 no longer sticks on `+1` only: entering a race from practice-like length clears clock latch state; tick mutates remain once; missing remain does not invent overtime text.
- 2026-08-24 — Session clock + classification live in shared `RaceStore` (`race_store.rs`). Dash still owns the heuristics; other widgets `get()` the same tick. No visual change.
- 0.1.0 — Shared practice/gate/race clock slot. Timed +2 last lap waits for the second extra crossing. Clock holds `00:00` until a real cross. White is a top banner. Flags only on a real run-in (~8–70 m then, now 4–80 m). Configurable footer. No flags while stopped on the gate.
- 0.1.2 — Lap motos (4+) show `1 / N`. 3-lap motos show lap count when length is not a standard timed session. Timed extras stay `0 / N` until the leader crosses after time expires. White ~40 m before last lap; checkered ~40 m before finish, then holds. Banner, not side panels. Website demo uses MPH / °F.
- 0.1.4 — Warmup no longer stuck at `10:00`. Checkered does not leak into next warmup. White then checkered on last-lap approach. Timed extras: no flags until extras start; +1 is white then checkered.
- 0.1.8 — Hidden until **Show on overlay**.
- 2026-08-19 — 4-lap motos ignore leftover `08:00`. Later gate boards after `00:10` stay a countdown instead of flashing frozen `08:00`.
- 2026-08-19 — RPM/speed column width is reserved from the widest digits so gear and position do not move as RPM ticks.
- 2026-08-25 — White flag banner matches checkered: stripes cover most of the band and wrap, fading to a white plaque behind the caption.
- 2026-08-20 — 8:00 +1 timed motos show `08:00 +1` / `+1`, not `1 / 1`.
- 2026-08-20 — 2-lap motos with a leftover start board show `1 / 2`, not `0 / +2`. `8:00 +2` extras are unchanged.
- 2026-08-20 — SHM publishes `session_kind` / `session_state`. F9 dump lists them; clock logic still infers warmup vs race from laps/length.
- 2026-08-20 — Warmup logged as `session_kind=5`, `session_state=16` (Maryland 30:00). Race 2 is `kind=7` (`state=256` on the gate, `state=16` once rolling). Same `kind=7` was **8:00 +1** and a **4-lap** moto (`laps=4`, `length=0`).
- 2026-08-20 — Plugin session length starts at `-1` and resets on kind change so leftover warmup time is not treated as this race. Overlay still treats `<= 0` as no clock.
