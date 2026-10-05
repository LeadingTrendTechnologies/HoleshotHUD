# Unread signals

Fields the game sends, or that we can derive, that the rider HUD does not show. Plugin field status stays in [Home.md](../Home.md). Widget sketches stay in [widgets.md](widgets.md).

**Overlay** means the value is already in shared memory. **Need SHM** means the API has it and the plugin drops it.

Already optional on Dash, Pit Board, Standings, or Relative: combined engine temp, air temp, fuel, fuel %, penalty, and setup. Those are not hidden.

## Discarded in `RunTelemetry`

`PluginState::setTelemetry` keeps gear, RPM, inputs, one temperature, fuel, speed, pose, and lean. It does not copy:

| Signal | Why a rider would care |
| --- | --- |
| Front and rear wheel speed (`m_afWheelSpeed`) | Slip against chassis speed. Throttle and brake traction. |
| Suspension length and velocity, plus max travel from `EventInit` | Bottoming, pack, and travel left. Setup riders live on this. |
| Acceleration X/Y/Z | Braking G and landing impacts. Sketched as [G-Force](widgets.md#g-force). |
| Brake pressure, front and rear | What the calipers did versus how hard the lever was pulled. |
| Steer torque | Bar feedback, separate from steer angle. Lean already shows the angle. |
| Yaw, pitch, and roll rate | How fast the bike is rotating, not just the lean angle. |
| Vertical velocity | Jump apex and landing speed. Height is already Overlay, so this can also be derived. |
| Wheel surface material | Dirt, concrete, or off-track under each wheel. Explains a dead sector. |

One temperature is kept: water if it is above 1°, otherwise engine. Optimal temp and the alarm band from `EventInit` are not copied, so there is a reading and no overheat warning.

Also dropped:

- **Invalid lap** (`m_iInvalid`). A 0-time lap is reconstructed from the live clock and shown as LAST. The game’s invalid flag is never read.
- **Which lap was the best** (`m_iBestLapNum`).
- **`RaceCommunication`**. The callback runs and ignores the payload. Penalty, warning, and offence codes are in there. The enums are not mapped. Sketched as the [Event Log](widgets.md#event-log).
- Session **conditions**, event type, gear count, and the full bike name.
- Other riders never get clutch, rear brake, fuel, temps, or suspension. `RaceVehicleData` is RPM, gear, speed, throttle, front brake, and lean.

## In shared memory, not drawn

- **Other riders’ RPM, gear, throttle, and front brake** (SHM v15). Relative can show their speed. Nothing shows that the rider ahead is pinned in third while you are short-shifting.
- **Height** (`local_y` and each rider’s `y`). Air time, jump height, and who is still in the air.
- **Holeshot winner and time** (SHM v16). The profile counts them. The live plaque is [Holeshot / Start](widgets.md#holeshot--start).
- **Centerline elevation** (`m_fHeight`). Kept on the plugin segment, dropped when `poly[]` is built. The map is plan-view only.

## Derivable with no new plugin field

- **Fuel rate.** Liters or gallons per lap, and laps left in the tank, from `fuel` against completed laps and track position. A pit refill has to be ignored or it looks like a huge burn. Sketched as the [fuel calculator](widgets.md#fuel-calculator).
- **Lap consistency.** We keep last lap, the one before it (lap diff), and best. A ring of valid laps would show whether times are grouping or swinging. Delta Bar is pace at a track position. Sketched as [lap consistency](widgets.md#lap-consistency).
- **Closing rate.** Gap change over a second, or your speed versus the published rival speed. That is the [battle card](widgets.md#battle-card): one rival, live gap, who is faster.
- **Line offset.** Meters left or right of the centerline from `poly[]` plus your X/Z. This needs a few in-game dumps first. The sign, jumps, and off-track teleports are unproven, and a missing centerline would always read about 0. Sketched as [Line](widgets.md#line).

## Change log

- 2026-10-03 — First cut. Signals the API or shared memory already has that the rider HUD does not show.
