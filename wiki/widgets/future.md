# Future widgets

These are **not shipped**. When one lands, give it a real page under `wiki/widgets/` and a row in [widgets.md](../widgets.md), then drop it from here.

Plugin field status lives in [Home.md](../Home.md). **Overlay** = already in SHM. **Need SHM** = API has it, plugin/overlay do not publish it yet.

| Widget | Closest we have | Data |
| --- | --- | --- |
| [G-Force](#g-force) | — | Need SHM (`m_fAccelerationX/Y/Z`) |
| [Fuel calculator](#fuel-calculator) | Dash / table **Fuel** and **Fuel %** | Overlay (`fuel` / `maxFuel`) |
| [Ideal Lap](#ideal-lap) | [Sectors](sector.md) — shipped as IDEAL on that widget | Overlay splits + `track_pb` |
| [Lap consistency](#lap-consistency) | [Delta Bar](delta-bar.md) | Overlay last/best lap; need a lap ring |
| [Telemetry](#telemetry) | [Telemetry](telemetry.md) — shipped as traces + bars + gear/speed | Overlay (`localThrottle` / brakes / clutch). Suspension still Need SHM |
| [Pitboard](#pitboard) | [Dash](dash.md) + [Sectors](sector.md) | Overlay (lap / split / gap) |
| [Event Log](#event-log) | — | Cached / unused (`RaceCommunication`, laps, holeshot, penalties) |

---

## G-Force

Lat/long G-meter with a peak marker.

- Plugin: `SPluginsBikeData_t.m_fAccelerationX/Y/Z` — **Unused**. World axes; confirm which pair is lateral vs longitudinal before drawing.
- Not in SHM. Bump `MXBO_SHM_VERSION`.
- Peak marker is overlay state (hold last max, decay or a reset). Do not persist across sessions unless we decide to.

## Fuel calculator

Consumption tracking, not just a tank readout.

- Level is already **Overlay** (`fuel` / `maxFuel`) and shows in Dash / Standings / Relative / ticker footers. This widget is **rate**: L/lap or gal/lap, remaining laps, maybe a session total.
- Derive from `fuel` vs completed laps / `local_track_pos`. No extra plugin field.
- Do not treat a refill or a reset as a huge burn. Need a baseline after leaving pits / starting a lap.

## Ideal Lap

Shipped on [Sectors](sector.md): best S1 + S2 + S3 (possibly from different laps). Night-ink pill in the LAP column; **IDEAL** row in the lap log. Not a separate widget. No purple (standings still owns Best Lap Violet).

## Lap consistency

Lap-time trend across the session (sparkline / rolling delta vs average or best).

- Overlay today keeps **last** and **best**, not a history. Build a ring of completed laps from `RaceLap` / `RunLap` (or last-lap crossings). Invalid / crash / out-lap must not enter the trend.
- No extra plugin field beyond lap times we already see. Persist only if we want all-time; default is this session.
- Delta Bar is pace vs a saved tape **at this track position**. This widget is whole-lap scatter: are laps grouping, or swinging. Do not duplicate the hairline.

## Telemetry

Shipped as [Telemetry](telemetry.md): throttle / brake traces, clutch / brake / throttle bars, gear and speed. Suspension graphs are still future (`m_afSuspLength` / `m_afSuspVelocity` — Need SHM).

## Pitboard

Pitboard-style lap / split board.

- A crew-style plaque: last lap, delta vs PB, maybe position and a split flash. Not a new data source — Dash + Sectors already know this.
- Overlay today: last/best lap, live place, sector freeze. Gap vs PB is the Delta Bar tape / sector freeze, not the in-game ghost.
- Worth it when the layout is a **board** (large last-lap + delta), not another Dash footer.

## Event Log

Timestamped race-event feed.

- Plugin: `RaceCommunication` (**Unused**, enums unmapped), `RaceLap` / `RaceSplit` (splits already Overlay), `RaceHoleshot` (**Unused**), classification `m_iPenalty` (**Cached**, not in SHM), rider state DNS/OUT/DSQ.
- Need a small ring of events in overlay (or SHM). Map `m_iCommunication` / `m_iReason` / `m_iOffence` in-game before drawing labels — they are game-defined ints.
- Overlap with [Flags](flag.md) (white/checkered/yellow/blue) and Dash notices. This is a **scrollback**, not a flag cloth.

## Change log

- 2026-09-09 — Telemetry shipped (traces + bars + gear/speed). Suspension graphs stay here.
- 2026-09-04 — Controller (was Gamepad) is Labs-only. See [gamepad.md](gamepad.md).
- 2026-09-02 — Gamepad shipped. Moved to [gamepad.md](gamepad.md).
- 2026-09-01 — Lean shipped. Moved to [lean.md](lean.md).
- 2026-09-01 — Added Lap consistency (session lap-time trend).
- 2026-09-01 — First cut. Eight widgets only.
