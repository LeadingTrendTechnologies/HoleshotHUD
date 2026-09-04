# MX Bikes plugin data wiki

Everything the PiBoSo plugin API can send this project, and whether we already keep it.

Source of truth: `src/vendor/piboso/mxb_api.h` (data version **8**, interface **9**).  
The game loads `Holeshot-HUD.dlo` and calls the exported functions below. The plugin may copy fields into `PluginState`, then into shared memory `Local\MXBOHudV12` for the Rust overlay.

**Status**

| Tag | Meaning |
| --- | --- |
| Overlay | In shared memory today — overlay can draw it with no plugin change |
| Cached | Plugin stores it in `PluginState`, not published to SHM yet |
| Received | Callback runs and copies the struct, then throws most fields away |
| Unused | Callback exists; we ignore the payload today (still available) |
| Draw-only | Used to draw inside the game, not a data feed |

Times from classification / laps are **milliseconds** unless noted.  
Arrays `[2]` are **front = 0, rear = 1** (PiBoSo convention; matches how we treat suspension).  
World XZ is the track plane; Y is height. HUD draw space is **0,0 top-left → 1,1 bottom-right**.

---

## Pipeline

```
MX Bikes
  → plugin callbacks (this wiki)
    → PluginState  (src/state.h)
      → shared memory MxboShmSnapshot  (src/shm/mxbo_shm.h)
        → overlay widgets  (overlay/src)
```

Telemetry rate is whatever `Startup` returns (**50 Hz** now). Classification / track positions / vehicle data arrive on the game’s race tick, not necessarily 50 Hz.

To add a widget: if the field is **Overlay**, draw it. If **Cached** / **Received** / **Unused**, add it to `PluginState` + `MxboShmSnapshot` (bump `MXBO_SHM_VERSION`) + overlay `shm.rs`, then draw. Add `wiki/widgets/<name>.md` and a row in [widgets.md](widgets.md).

---

## Overlay widget wikis

Per-widget behavior, pitfalls, and change history for agents: **[widgets.md](widgets.md)**.

Rust overlay structure and possible refactors (suggestions only): **[rust-patterns.md](rust-patterns.md)**.

## What the overlay already shows

- [Standings](widgets/standings.md): place, `#`, name, gap (ms or laps)
- [Horizontal Standings](widgets/horizontal-standings.md): leader on the left, your name highlighted; lap/temp ends
- [Relative](widgets/relative.md): riders near you on `m_fTrackPos`
- [Map](widgets/map.md): tessellated centerline, rider XZ + yaw, crash, S/F meters, track name
- [Minimap](widgets/minimap.md): circular zoomed track
- [Radar](widgets/radar.md): side / rear proximity
- [Dash](widgets/dash.md): gear, speed, session clock, flags (optional simple gear+speed lockup)
- [Sectors](widgets/sector.md): S1–S3 times (labs flag)
- [Delta Bar](widgets/delta-bar.md): time vs your best at this track position (labs flag; recorded lap, not the ghost)
- [Systems](widgets/systems.md): CPU / mem / FPS
- [Stance](widgets/stance.md): sit / stand from a local bind (not plugin telemetry)

Local speed / yaw / crash / track pos are in SHM for the moving marker, not as their own widgets yet.

---

## Lifecycle callbacks

| Export | Payload | Status | Notes |
| --- | --- | --- | --- |
| `GetModID` | `"mxbikes"` | — | Must match |
| `GetModDataVersion` | `8` | — | Struct layout |
| `GetInterfaceVersion` | `9` | — | Callback set |
| `Startup` | save path string | — | Returns telemetry Hz. Ini + SHM opened here |
| `Shutdown` | — | — | Saves ini, closes SHM |
| `EventInit` | `SPluginsBikeEvent_t` | Received | Rider/bike/track constants for **your** bike |
| `EventDeinit` | — | Cached | Clears event + race |
| `RunInit` | `SPluginsBikeSession_t` | Unused | Session at run start |
| `RunDeinit` | — | Unused | |
| `RunStart` / `RunStop` | — | Unused | Green flag / pause style events |
| `DrawInit` | sprite/font names | Draw-only | We request **0** sprites/fonts |
| `Draw` | — | Draw-only | Publishes SHM every frame; in-game HUD optional |
| `SpectateVehicles` | `SPluginsSpectateVehicle_t[]` | Overlay | Reads the current camera target; overlay name/card clicks can change it in replay/spectate |
| `SpectateCameras` | camera list | Unused | Names only; not a video feed |

---

## 1. Your bike — event (`EventInit`)

`SPluginsBikeEvent_t` — once per event, not per frame.

| Field | Type | Status | Widget ideas |
| --- | --- | --- | --- |
| `m_szRiderName` | char[100] | Cached | “You” label; used to match local race number |
| `m_szBikeID` | char[100] | Unused | Bike internal id |
| `m_szBikeName` | char[100] | Unused | Bike title |
| `m_iNumberOfGears` | int | Unused | Gearbox widget scale |
| `m_iMaxRPM` | int | Unused | RPM gauge max |
| `m_iLimiter` | int | Unused | Rev limiter |
| `m_iShiftRPM` | int | Unused | Shift light |
| `m_fEngineOptTemperature` | float | Unused | Temp target |
| `m_afEngineTemperatureAlarm[2]` | float | Unused | Temp warning band |
| `m_fMaxFuel` | float | Overlay | Tank size in liters |
| `m_afSuspMaxTravel[2]` | float | Unused | Suspension travel % |
| `m_fSteerLock` | float | Unused | Steer gauge scale |
| `m_szCategory` | char[100] | Unused | Class (MX, SX, …) |
| `m_szTrackID` | char[100] | Unused | Track id |
| `m_szTrackName` | char[100] | Overlay | Map header (also from race event) |
| `m_fTrackLength` | float | Overlay | Length (meters). Relative wrap |
| `m_iType` | int | Unused | Event type; **enum not in header** |
| `m_szServerName` | char[64] | Overlay | Online server name (presence fallback key) |
| `m_iServerType` | int | Overlay | Online/offline style flag |
| `m_szGUID` | char[100] | Overlay | Event/session id (presence join key; F9 dump) |

---

## 2. Your bike — session (`RunInit`)

`SPluginsBikeSession_t` — **Unused** (callback does not copy).

| Field | Type | Widget ideas |
| --- | --- | --- |
| `m_iSession` | int | Practice / qual / race (game-defined int) |
| `m_iConditions` | int | Weather/conditions |
| `m_fAirTemperature` | float | Air temp |
| `m_szSetupFileName` | char[100] | Setup name |

Same session fields also arrive on `RaceSession` (also not stored).

---

## 3. Your bike — live telemetry (`RunTelemetry`)

`SPluginsBikeData_t` plus extra args `_fTime`, `_fPos`. Rate = Startup Hz (50).

This is the richest per-frame feed. **Only a handful of fields are kept.**

| Field | Type | Status | Notes / widgets |
| --- | --- | --- | --- |
| `m_iRPM` | int | Unused | Tach, shift light with `m_iShiftRPM` |
| `m_fEngineTemperature` | float | Unused | Engine temp |
| `m_fWaterTemperature` | float | Unused | Water temp |
| `m_iGear` | int | Unused | Typically 0 = N, 1+ = gears (confirm in-game) |
| `m_fFuel` | float | Overlay | Liters. Header/footer: L or US gal from Units |
| `m_fSpeedometer` | float | Overlay | Speed (game units; treated as speedometer) |
| `m_fPosX/Y/Z` | float | Overlay (X,Z) | World position. Y unused. Map marker |
| `m_fVelocityX/Y/Z` | float | Overlay (X,Z) | Used to interpolate the local marker |
| `m_fAccelerationX/Y/Z` | float | Unused | G-meter |
| `m_aafRot[3][3]` | float | Unused | Rotation matrix |
| `m_fYaw` | float | Overlay | Heading |
| `m_fPitch` | float | Unused | Wheelie / nose |
| `m_fRoll` | float | Unused | Lean (also on `RaceVehicleData.m_fLean`) |
| `m_fYawVelocity` | float | Unused | Spin rates |
| `m_fPitchVelocity` | float | Unused | |
| `m_fRollVelocity` | float | Unused | |
| `m_afSuspLength[2]` | float | Unused | Fork / shock length |
| `m_afSuspVelocity[2]` | float | Unused | Compression speed |
| `m_iCrashed` | int | Overlay | Crash flag on local marker |
| `m_fSteer` | float | Unused | Steer input |
| `m_fThrottle` | float | Unused | 0–1 style input bar |
| `m_fFrontBrake` | float | Unused | Front brake input |
| `m_fRearBrake` | float | Unused | Rear brake input |
| `m_fClutch` | float | Unused | Clutch |
| `m_afWheelSpeed[2]` | float | Unused | Wheel speed vs GPS → slip |
| `m_aiWheelMaterial[2]` | int | Unused | Surface type under each wheel |
| `m_afBrakePressure[2]` | float | Unused | Hydraulic pressure |
| `m_fSteerTorque` | float | Unused | Bar torque |
| `_fTime` (callback arg) | float | Unused | Session/run time |
| `_fPos` (callback arg) | float | Overlay | Local track position (`localTrackPos`) |

---

## 4. Your bike — laps & splits

### `RunLap` → `SPluginsBikeLap_t` — **Unused**

| Field | Type | Widget ideas |
| --- | --- | --- |
| `m_iLapNum` | int | Lap counter |
| `m_iInvalid` | int | Invalid lap flash |
| `m_iLapTime` | int | Last lap time (ms) |
| `m_iBest` | int | Best lap (ms) |

### `RunSplit` → `SPluginsBikeSplit_t` — **Overlay**

| Field | Type | Widget ideas |
| --- | --- | --- |
| `m_iSplit` | int | Sector index |
| `m_iSplitTime` | int | Sector time (ms) |
| `m_iBestDiff` | int | Delta vs best (ms) |

Race-wide versions exist too (`RaceLap`, `RaceSplit`) with `m_iRaceNum` so you can show **any** rider’s lap, not just yours.

---

## 5. Track centerline (`TrackCenterline`)

`SPluginsTrackSegment_t[]` plus `_pRaceData`.

| Field | Type | Status | Notes |
| --- | --- | --- | --- |
| `m_iType` | int | Cached | `0` (or radius ≈ 0) = straight; else curve. Tessellated into the map poly |
| `m_fLength` | float | Cached | Segment length (meters) |
| `m_fRadius` | float | Cached | Curve radius; ~0 = straight |
| `m_fAngle` | float | Cached | Heading (degrees) at start |
| `m_afStart[2]` | float | Cached | Start XZ |
| `m_fHeight` | float | Cached | Elevation at segment (not drawn) |
| `_pRaceData[0]` | float | Overlay | Start/finish distance along track (`sfMeters`) |

The overlay map uses the tessellated polyline (`poly[]` in SHM), not raw segments.

If centerline is missing, the plugin records a local XZ **trail** as a fallback line.

---

## 6. Race event & entry list

### `RaceEvent` → `SPluginsRaceEvent_t` — Received (track name/length only)

| Field | Type | Status |
| --- | --- | --- |
| `m_iType` | int | Unused |
| `m_szName` | char[100] | Unused (event name) |
| `m_szTrackName` | char[100] | Overlay |
| `m_fTrackLength` | float | Overlay |

### `RaceAddEntry` → `SPluginsRaceAddEntry_t`

| Field | Type | Status | Notes |
| --- | --- | --- | --- |
| `m_iRaceNum` | int | Overlay | Bike number / id used everywhere |
| `m_szName` | char[100] | Overlay | Display name (truncated to 32 in SHM) |
| `m_szBikeName` | char[100] | Unused | Full bike name |
| `m_szBikeShortName` | char[100] | Overlay | Standings / Relative bike column; Delta / Sectors class key (250 / 450) |
| `m_szCategory` | char[100] | Unused | Class |
| `m_iUnactive` | int | Cached | Inactive / not racing |
| `m_iNumberOfGears` | int | Unused | That rider’s gearbox |
| `m_iMaxRPM` | int | Unused | That rider’s redline |

### `RaceRemoveEntry` → `{ m_iRaceNum }` — Cached (removes from maps)

---

## 7. Session (`RaceSession` / `RaceSessionState`)

### `SPluginsRaceSession_t` — Overlay (`setSession`)

| Field | Type | Status | Notes |
| --- | --- | --- | --- |
| `m_iSession` | int | Overlay | Session kind (`session_kind`). Logged: **warmup = 5**, **race 2 = 7**. Kind is **which moto**, not lap vs timed: race 2 was `7` for both **8:00 +1** and a **4-lap** moto. Race 1 not dumped yet (likely **6**). Kind does **not** change when you leave the gate. |
| `m_iSessionState` | int | Overlay | Session state (`session_state`). **16** = running (warmup and race 2 on track). **256** = race 2 on the start gate. |
| `m_iSessionLength` | int | Overlay | Time-limited session (minutes, seconds, or ms — plugin normalizes). Plugin cache is **`-1` until this session writes a length**; **`0` means the game sent 0** (lap moto). Kind change clears the cache so leftover warmup minutes are not locked. |
| `m_iSessionNumLaps` | int | Overlay | Lap moto length, or extras on a timed set |
| `m_iConditions` | int | Unused | Conditions |
| `m_fAirTemperature` | float | Overlay | Air temp |

### `SPluginsRaceSessionState_t` — Overlay (`setSessionState`)

| Field | Type | Status |
| --- | --- | --- |
| `m_iSession` | int | Overlay | Updates `session_kind` |
| `m_iSessionState` | int | Overlay | Updates `session_state` |
| `m_iSessionLength` | int | Overlay | Remaining or elapsed length |

Classification header also has `m_iSessionTime` (see below).

---

## 8. Race laps, splits, holeshot, flags

All **Unused** today.

### `RaceLap` → `SPluginsRaceLap_t`

| Field | Type | Notes |
| --- | --- | --- |
| `m_iSession` | int | |
| `m_iRaceNum` | int | Which rider |
| `m_iLapNum` | int | |
| `m_iInvalid` | int | |
| `m_iLapTime` | int | ms |
| `m_aiSplit[2]` | int | Two stored split times |
| `m_iBest` | int | Best lap ms |

### `RaceSplit` → `SPluginsRaceSplit_t` — **Overlay**

| Field | Type |
| --- | --- |
| `m_iSession` | int |
| `m_iRaceNum` | int |
| `m_iLapNum` | int |
| `m_iSplit` | int |
| `m_iSplitTime` | int |

### `RaceHoleshot` → `SPluginsRaceHoleshot_t`

| Field | Type | Widget ideas |
| --- | --- | --- |
| `m_iSession` | int | |
| `m_iRaceNum` | int | Who got holeshot |
| `m_iTime` | int | Holeshot time |

### `RaceCommunication` → `SPluginsRaceCommunication_t`

Race-control / penalty / message. Integers are **game-defined**; log them in-game to map enums.

| Field | Type | Widget ideas |
| --- | --- | --- |
| `m_iSession` | int | |
| `m_iRaceNum` | int | Target rider |
| `m_iCommunication` | int | Message kind |
| `m_iState` | int | |
| `m_iReason` | int | |
| `m_iOffence` | int | |
| `m_iLap` | int | |
| `m_iStart` | int | |
| `m_iType` | int | |
| `m_iTime` | int | Duration or timestamp |

---

## 9. Classification / standings (`RaceClassification`)

Header `SPluginsRaceClassification_t` + array of `SPluginsRaceClassificationEntry_t`.  
Array order **is race order**; we set `position = index + 1`.

### Header — mostly unused (only `m_iNumEntries` used)

| Field | Type | Status | Widget ideas |
| --- | --- | --- | --- |
| `m_iSession` | int | Unused | |
| `m_iSessionState` | int | Unused | |
| `m_iSessionTime` | int | Unused | Elapsed/remaining clock |
| `m_iNumEntries` | int | Cached | How many rows |

### Each entry

| Field | Type | Status | Notes |
| --- | --- | --- | --- |
| `m_iRaceNum` | int | Overlay | |
| `m_iState` | int | Overlay | `1` DNS, `3` OUT, `4` DSQ in our standings labels; other values = racing |
| `m_iBestLap` | int | Overlay | Best lap ms (`bestLapMs`) |
| `m_iBestLapNum` | int | Unused | Which lap was the best |
| `m_iNumLaps` | int | Overlay | |
| `m_iGap` | int | Overlay | Gap ms to leader (or ahead) |
| `m_iGapLaps` | int | Overlay | Lapped gap |
| `m_iPenalty` | int | Cached | Penalty ms — **not in SHM** |
| `m_iPit` | int | Overlay | Pit flag |

Name is joined from the entry list, not this struct.

---

## 10. Everyone’s position (`RaceTrackPosition`)

`SPluginsRaceTrackPosition_t[]` — Overlay (Y dropped).

| Field | Type | Status | Notes |
| --- | --- | --- | --- |
| `m_iRaceNum` | int | Overlay | |
| `m_fPosX/Y/Z` | float | Overlay XZ | Map dots. Y unused |
| `m_fYaw` | float | Overlay | |
| `m_fTrackPos` | float | Overlay | Distance along track; relative widget wraps this vs you |
| `m_iCrashed` | int | Overlay | |

---

## 11. Everyone’s slim live data (`RaceVehicleData`)

`SPluginsRaceVehicleData_t` — **Cached** per race number, **not in SHM**.

| Field | Type | Widget ideas |
| --- | --- | --- |
| `m_iRaceNum` | int | |
| `m_iActive` | int | Hide inactive |
| `m_iRPM` | int | Rival RPM |
| `m_iGear` | int | |
| `m_fSpeedometer` | float | Relative “+3 mph vs #12” |
| `m_fThrottle` | float | |
| `m_fFrontBrake` | float | |
| `m_fLean` | float | Lean of any rider |

This is **not** full telemetry. Other players do not get temps, fuel, clutch, suspension, etc.

---

## 12. Spectate

### `SpectateVehicles` → `SPluginsSpectateVehicle_t`

| Field | Type | Status |
| --- | --- | --- |
| `m_iRaceNum` | int | Cached as `focusRaceNum` |
| `m_szName` | char[100] | Unused (we already have names from entries) |

Return `0` = do not change the game’s selection. Return `1` and write the vehicle **index** into `_piSelect` to switch the camera (one-shot). While this callback is running we set `spectating=1` on `Local\MXBOHudCmdV1` and treat that race number as `focusRaceNum`. If `SpectateVehicles` stops (garage / riding), `spectating` drops after ~250 ms, focus falls back to `localRaceNum` (also cleared on `RunInit` / `RunDeinit`), and overlay clicks pass through to the game. Do not keep a replay camera target after the session ends — that highlights the wrong rider and feeds the dash their RPM.

### `SpectateCameras` — **Unused**

Camera name list only. No image, FOV, or matrix.

---

## 13. In-game draw (not overlay data)

`SPluginQuad_t` / `SPluginString_t` are what **we send back** to MX Bikes when `ingame_hud=1`. Colors are packed **ABGR**. Not a source of telemetry.

---

## Shared memory (`MxboShmSnapshot`) — overlay contract

Already published (version **1**):

- Clock: `tickQpc` (for interpolation)
- Local: race num, focus num, crashed, XZ, vel XZ, yaw, speed, track pos
- Track: name, length, S/F meters, polyline
- Riders: race num, XZ, yaw, track pos, crashed, name
- Standings: race num, position, state, best lap, laps, gap ms/laps, pit, name
- Session: length, laps, remaining clock, **kind** (`m_iSession`), **state** (`m_iSessionState`) — SHM version **9**
- Fuel: `fuel` / `maxFuel` — SHM version **10**
- Presence: `guid`, `serverName`, `serverType` — SHM version **11**
- Steam friends: `localSteamId`, `friendCount`, `friends[]` — SHM version **12**. Do not log the IDs.
- Layout: map / standings / relative rects + show flags + row counts

Command mapping `Local\MXBOHudCmdV1` (`MxboShmCmd`): overlay writes `spectateRaceNum`; plugin writes `spectating` while `SpectateVehicles` is live. Not part of the snapshot seqlock.

**Not published yet** (but available in the API or `PluginState`): penalty, bike names, laps/splits, holeshot, comms, RPM/gear/inputs/temps/suspension, per-rider `VehicleLive`, pitch/roll, spectate camera list.

Bump `MXBO_SHM_VERSION` when you add fields; keep C and Rust `#[repr(C)]` layouts identical.

---

## Not available from MX Bikes

The plugin API does **not** include:

- Tire temp, pressure, wear
- Heart rate / fitness
- Detailed weather (rain %, wind vector) beyond `m_iConditions` + air temp
- Other riders’ full telemetry (only `RaceVehicleData`)
- Live camera picture or FOV
- Inputs from other players beyond throttle / front brake / lean
- Setup XML contents (only setup **filename** on `RunInit`)
- Rider sit / stand from the plugin API. Game HUD: Simulation → **Show Rider Stand**. Confirmed 2026-08-26: no extra telemetry bytes; parked rear shock length does not jump with sit/stand. Overlay **Stance** widget is a pad-button mirror (not telemetry) — see [stance.md](widgets/stance.md).

---

## Widget cheat sheet

Existing overlay widgets: [widgets.md](widgets.md) (behavior + change logs). Fields below are plugin status, not the overlay UI.

| Widget | Primary fields | Status |
| --- | --- | --- |
| [Standings](widgets/standings.md) | classification + names | Overlay |
| [Relative](widgets/relative.md) | `m_fTrackPos` + names | Overlay |
| [Map](widgets/map.md) | centerline + positions | Overlay |
| Speed / gear / RPM | telemetry + event max/shift RPM | Overlay (dash) |
| [Systems](widgets/systems.md) | host meters | Overlay |
| [Sectors](widgets/sector.md) | `RunSplit` / `RaceSplit` | Overlay (labs flag) |
| Shift light | `m_iRPM` vs `m_iShiftRPM` | Need SHM |
| Throttle / brakes / clutch | telemetry inputs | Need SHM |
| Lean / pitch | `m_fRoll` / `m_fPitch` or `m_fLean` | Need SHM |
| Fuel | `m_fFuel` / `m_fMaxFuel` | Overlay (dash / standings / relative / ticker) |
| Temps | engine/water + alarm band | Need SHM |
| Suspension | length / velocity / max travel | Need SHM |
| G-meter | acceleration XYZ | Need SHM |
| Wheel slip | wheel speed vs chassis velocity | Need SHM |
| Current / last / best lap | `RunLap` / `RaceLap` | Overlay (dash / standings) |
| Sector delta | `RunSplit` / `RaceSplit` | Overlay (labs: Sectors) |
| Session timer / laps to go | `RaceSession` + classification header | Unused |
| Penalty banner | `m_iPenalty` + `RaceCommunication` | Cached / unused |
| Holeshot | `RaceHoleshot` | Unused |
| Rival compare | `RaceVehicleData` speed/rpm vs you | Cached |
| Air temp | session | Unused |

---

## Files to touch for a new field

1. `src/vendor/piboso/mxb_api.h` — already defined  
2. `src/plugin.cpp` — make sure the callback copies the struct  
3. `src/state.h` / `src/state.cpp` — keep latest value  
4. `src/shm/mxbo_shm.h` + `src/shm_writer.cpp` — publish  
5. `overlay/src/shm.rs` — same C layout  
6. `overlay/hud/src/render.rs` — draw the widget
7. `src/config.h` — optional `show_*` + ini rect
8. `wiki/widgets/<name>.md` — agent context + change log

Enums without comments in the header (`m_iSession`, `m_iConditions`, `m_iType`, communication codes) should be logged once in-game before you hard-code labels.
