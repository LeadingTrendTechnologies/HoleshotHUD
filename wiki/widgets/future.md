# Future widgets

These are **not shipped**. When one lands, give it a real page under `wiki/widgets/` and a row in [widgets.md](../widgets.md), then drop it from here.

Plugin field status lives in [Home.md](../Home.md). **Overlay** = already in SHM. **Need SHM** = API has it, plugin/overlay do not publish it yet.

Streaming (OBS Browser Source, stream-only layout) is not a widget — see [streaming.md](../streaming.md).

| Widget | Closest we have | Data |
| --- | --- | --- |
| [Holeshot / Start](#holeshot--start) | — | Overlay gate (`IN_GATE` / `session_state` 256) + live `track_pos`. `RaceHoleshot` Unused |
| [Battle card](#battle-card) | [Relative](relative.md) | Overlay gaps. Rival speed Need SHM (`RaceVehicleData`) |
| [Spectate nameplate](#spectate-nameplate) | Dash / Standings | Overlay (`focus_race_num`, live order, last lap) |
| [Line](#line) | [Map](map.md) centerline | Overlay `poly[]` + XZ. **Needs in-game dumps** |
| [Radar lappers](#radar-lappers) | [Radar](radar.md) + map `lap_rel` | Overlay — not a new widget |
| [G-Force](#g-force) | — | Need SHM (`m_fAccelerationX/Y/Z`) |
| [Fuel calculator](#fuel-calculator) | Dash / table **Fuel** and **Fuel %** | Overlay (`fuel` / `maxFuel`) |
| [Ideal Lap](#ideal-lap) | [Sectors](sector.md) — shipped as IDEAL | Overlay splits + `track_pb` |
| [Lap consistency](#lap-consistency) | [Delta Bar](delta-bar.md) | Overlay last/best lap; need a lap ring |
| [Telemetry](#telemetry) | [Telemetry](telemetry.md) — traces + bars + gear/speed | Overlay inputs. Suspension still Need SHM |
| [Pitboard](#pitboard) | [Dash](dash.md) + [Sectors](sector.md) | Overlay (lap / split / gap) |
| [Event Log](#event-log) | — | Cached / unused (`RaceCommunication`, laps, penalties) |

---

## Holeshot / Start

Gate lineup, live chase into turn 1, then the official holeshot call.

- **Gate** — `IN_GATE` / `session_state == 256`. Compact live order, you highlighted. Hide in practice / warmup.
- **Drop** — live chase from `riders[].track_pos` (same as Relative / live order) until the callback or a short track-pos threshold.
- **Call** — unused `RaceHoleshot` (`m_iRaceNum` + `m_iTime`) in `src/plugin.cpp`. Winner plaque; your gap if you lost. Hold a few seconds, then hide.
- Plugin stub today. Publish winner + time on SHM (bump `MXBO_SHM_VERSION`).
- Glanceable plaque, not a second standings table. `RaceHoleshot` is no longer only an Event Log leftover.

## Battle card

One rival at race speed. Relative is a table; this is the 40 mph glance. Can also sit on a stream layout.

- Closest on track, or live-order P−1 / P+1. Name, number, live gap.
- Speed delta needs `RaceVehicleData` speed on SHM (Cached today).
- Optional lean tick. Does not replace Relative.

## Spectate nameplate

In-game lower third while spectating: P#, name, gap, last lap. Follows `focus_race_num`.

- League casters and replay watchers who stay in the game. Complements [streaming.md](../streaming.md) (stream-only layout) — this one can also sit on the Spectate preset.

## Line

Meters left/right of the track centerline. Sketch only: `L 1.2` / `R 0.4`. Not a heading compass.

**Do not spec or build until we dump this in-game** on a few tracks.

What we know: map already has tessellated `poly[]` from `TrackCenterline` ([Home.md](../Home.md) §5). Rider XZ is Overlay. If the plugin sends no centerline, the fallback is *your* XZ trail — that offset would always read ~0 and must not be shown as Line.

Unknown (log before a widget page):

- Left vs right: cross the nearest poly segment with track-forward; confirm the sign matches the rider’s left.
- Legal / racing line vs a rough ribbon. Wide MX lines can sit 3–8 m off and still be the ruts.
- Jumps / air: hide, freeze, or spike?
- Off-track, crash, pits, remount teleports.
- Height is on the segment (`m_fHeight`) but unused — plan-view only unless we decide otherwise.
- Units, update rate, numeric plaque vs a L\|R bar.

## Radar lappers

Not a new widget. Map and minimap already color with `lap_rel` / `rider_dot_col`: **blue** = they are a lap up and closing from behind, **red** = you are a lap up and closing on them. Off in warmup. `gap_laps` wins.

Radar blips are heat only (orange → cream). A lapper looks like anyone else.

Future Radar pane toggle (**Lappers**, default on): when `lap_rel` is set, paint that blip blue or red; keep heat for size. Same-lap blips stay the heat gradient. Same warmup / `gap_laps` rules as the map. When it ships, log it on [radar.md](radar.md) and drop this section.

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

- Plugin: `RaceCommunication` (**Unused**, enums unmapped), `RaceLap` / `RaceSplit` (splits already Overlay), classification `m_iPenalty` (**Cached**, not in SHM), rider state DNS/OUT/DSQ.
- Holeshot winner is [Holeshot / Start](#holeshot--start), not this scrollback.
- Need a small ring of events in overlay (or SHM). Map `m_iCommunication` / `m_iReason` / `m_iOffence` in-game before drawing labels — they are game-defined ints.
- Overlap with [Flags](flag.md) (white/checkered/yellow/blue) and Dash notices. This is a **scrollback**, not a flag cloth.

## Change log

- 2026-09-10 — Dropped unapproved ideas (ahead plate, interval bar, hunt, remount, finish projection, bike health, and the parked list).
- 2026-09-10 — Added Holeshot / Start, Battle card, Spectate nameplate, Line (needs dumps), Radar lappers. Streaming moved to [streaming.md](../streaming.md). Event Log no longer owns holeshot.
- 2026-09-09 — Telemetry shipped (traces + bars + gear/speed). Suspension graphs stay here.
- 2026-09-04 — Controller (was Gamepad) is Labs-only. See [gamepad.md](gamepad.md).
- 2026-09-02 — Gamepad shipped. Moved to [gamepad.md](gamepad.md).
- 2026-09-01 — Lean shipped. Moved to [lean.md](lean.md).
- 2026-09-01 — Added Lap consistency (session lap-time trend).
- 2026-09-01 — First cut. Eight widgets only.
