# Stream broadcast

Streaming is a second layout of the same widgets. How-to stays in [streaming.md](streaming.md). Rider wishlist stays in [widgets/future.md](widgets/future.md). This page is what a broadcast still lacks.

When you change this page, append a **Change log** bullet (why, not just what).

## Already covered

Standings, Horizontal Standings, Relative, Map, Flags, and Dash already cover a normal race broadcast. Do not add a second scoreboard, interval tower, or scorebug.

## Worth exploring

The rider HUD is built to hide these. Data status matches [Home.md](Home.md): **Overlay** means it is already in shared memory.

### Spectate nameplate

When the camera cuts, a viewer needs who is on screen without reading a table.

- Lower third: place, name, gap to leader, last lap. Follows `focus_race_num`.
- **Overlay** today. No SHM bump.
- Stream Spectate layout, and the in-game Spectate preset for replay. Hide while riding.
- Already sketched in [widgets/future.md](widgets/future.md).

### Holeshot sting

Profile already counts the holeshot. Nothing draws the live call.

- Gate: compact live order while `session_state == 256`.
- After the callback: winner plaque from `holeshotRaceNum` + `holeshotTime` (SHM v16). Hold a few seconds, then hide.
- Stream Race layout. A full gate board is too big to ride with. A small in-game plaque can share the same state later.
- Also sketched in [widgets/future.md](widgets/future.md) as Holeshot / Start.

### Results hold

The overlay drops the HUD when the session ends so leftover garage standings do not stay up ([widgets.md](widgets.md)). A broadcast wants the classification to stay for a beat after the checkered.

- Stream-only behavior, not a new widget. Keep the Race-layout standings (or a frozen classification card) for a short hold after `RunDeinit` / spectate end, then hide.
- Do not change the in-game rule.

## Skip

- **Event chyron** (penalty / crash / DNF stings). The right broadcast shape, but `RaceCommunication` enums are unmapped and `m_iPenalty` is Cached, not in SHM. Park until those ints are logged in-game.
- **Battle card**. Useful, and the data is mostly Overlay, but it is a rider glance too. Not stream-only. Stays on [widgets/future.md](widgets/future.md).
- **Coaching widgets** (G-force, line, fuel rate, lap consistency, telemetry). Wrong audience.
- **Dropped ideas** (interval bar, ahead plate, hunt, finish projection). Still not worth a stream version.

## Change log

- 2026-10-01 — First cut. Broadcast gaps the rider HUD is built to hide, so streaming is not treated as “no new widgets.”
