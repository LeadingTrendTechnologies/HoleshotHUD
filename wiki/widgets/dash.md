# Dash

Gear, RPM, speed, position, and the session clock / lap counter. Race **flags** are a banner **above** this widget, not side panels. Settings subtitle: “Gear, speed, and footer stats”.

This is the most stateful widget. Clock and flag bugs almost always belong here, not in Standings.

## Code

- Draw: `draw_dash`, `draw_dash_wrap`, `draw_rev_bar` in `overlay/hud/src/render.rs`
- Clock: `session_remain_ms`, `session_banner`, `is_lap_race`, overtime helpers
- Flags: `dash_race_flag`, `approaching_line`, `last_lap_cleared`
- Trace: `clock_sample` → `%LOCALAPPDATA%\Holeshot HUD\logs\race.jsonl`
- Settings: `pane_dash` in `overlay/src/settings.rs`
- Footer fields: `DashField`

Fixed body: gear | RPM + speed | position + lap/clock text. Footer is three slots (default Engine, Air, Best). Optional rev bar from `local_rpm` vs `max_rpm` / `shift_rpm`.

## Session clock (hard-won)

Practice, gate, and race time share **one** slot (`session_banner`).

- **Lap moto:** `session_laps >= 4` is always a lap race, even if leftover warmup `10:00` or a leftover timed length (`8:00`) is still in `session_length`. 3-lap motos are lap races only when length is not a leftover practice or a standard timed length (5–20 min set).
- Lap motos show `current / N` after green, not leftover minutes. Gate boards (about 8 s–2 min) still show a countdown until the clock runs up or you move. A later 45 s / 30 s board after `00:10` stays a countdown; leftover `08:00` must not replace it until the race clock actually ticks.
- **Timed race:** countdown while `session_time_ms` is live. When time expires, extras use `0 / N` until the **leader** crosses, then **your** crosses count (`local_overtime_taken`). Crossing as a backmarker at time-zero does **not** start extras.
- Clock stays `00:00` until you cross or the leader puts a lap on you (0.1.0).
- Warmup `10:00` must not stick after a race: prefer the ticking clock (0.1.4).

Plugin session fields are messy (warmup length leaking into race). Lots of atomics (`IN_GATE`, `SESSION_EXPIRED`, `OVERTIME_*`, `LAP_GREEN`) exist because the API does not send a clean mode enum. Change them with a race log in hand.

## Flags

Banner above the dash only (`draw_dash_wrap`). No striped side panels over the widget (reverted).

- No flags on the gate, while stopped, or while a long countdown is still running.
- Approach window: **4–80 m** to S/F (`FLAG_LINE_MIN_M` / `FLAG_LINE_M`).
- White on last-lap approach, then checkered on the finish approach. Checkered **latches** until you leave the session (`on_track == 0` clears it). It must not carry into the next warmup (0.1.4).
- You must leave the line by **80 m** (`LAST_LAP_CLEAR_M`) before checkered can arm, so the white banner can show first.
- Timed extras: no flags until extras start. **+1 extra** is white then checkered. **+2** uses `left == 2` as the last extra (not a lap-moto penultimate lap).
- White hold is 5 s (`WHITE_HOLD_MS`).

## Do not regress

- Do not put flags on the sides of the dash again.
- Do not treat leftover `10:00` or leftover `08:00` session length as a timed race when `session_laps >= 4`.
- Do not replace a later start board (`00:45` after `00:10`) with frozen `08:00` unless a live remaining clock has already been shown.
- Do not start extras from a backmarker cross at time zero; wait for the leader.
- Do not arm checkered on +1 extras until **you** have taken that extra (`last_extra_started`).

## Change log

- 0.1.0 — Shared practice/gate/race clock slot. Timed +2 last lap waits for the second extra crossing. Clock holds `00:00` until a real cross. White is a top banner. Flags only on a real run-in (~8–70 m then, now 4–80 m). Configurable footer. No flags while stopped on the gate.
- 0.1.2 — Lap motos (4+) show `1 / N`. 3-lap motos show lap count when length is not a standard timed session. Timed extras stay `0 / N` until the leader crosses after time expires. White ~40 m before last lap; checkered ~40 m before finish, then holds. Banner, not side panels. Website demo uses MPH / °F.
- 0.1.4 — Warmup no longer stuck at `10:00`. Checkered does not leak into next warmup. White then checkered on last-lap approach. Timed extras: no flags until extras start; +1 is white then checkered.
- 0.1.8 — Hidden until **Show on overlay**.
- 2026-08-19 — 4-lap motos ignore leftover `08:00`. Later gate boards after `00:10` stay a countdown instead of flashing frozen `08:00`.
