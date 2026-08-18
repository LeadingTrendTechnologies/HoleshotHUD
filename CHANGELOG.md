# Changelog

## 0.1.5

Silent in-app updates, one race log file, and feedback that attaches that log even mid-moto.

### Updates

- **Download and install** runs in the background with no PowerShell window
- The overlay closes, replaces files, then reopens when the new build is ready

### Feedback

- One log file: `%LOCALAPPDATA%\Holeshot HUD\logs\race.jsonl`
- A new session clears that file and starts writing again
- Send snapshots the log so a report can go out while you are still on track
- Bug reports attach the race samples, not an empty header file
- Old `clock-*.jsonl` files are removed on launch

## 0.1.4

In-app feedback, last-race logs, lapping colors, and session clock/flag fixes from race testing.

### Feedback

- **F8 → App → Feedback:** rate the app (1–5 stars) or report a bug
- Bug reports can attach the last race log (`%LOCALAPPDATA%\Holeshot HUD\logs`)
- Send posts to the Holeshot HUD server; you do not need a GitHub account

### Map and standings

- Other riders use a dark slate dot with a white number
- Blue only if they are a lap ahead and closing from behind
- Red only if you are a lap ahead and closing on them

### Dash and session

- Warmup no longer sticks at `10:00` after a race (plugin prefers the ticking clock)
- Checkered does not carry into the next warmup or a new moto
- White flag on the last-lap approach, then checkered on the finish approach
- Timed extras: no flags until extras start; +1 extra is white then checkered

## 0.1.3

Installer remembers the MX Bikes folder, and a version tag only runs the Release pipeline.

### Install

- Setup still finds Steam MX Bikes from the registry and library folders, then copies `mxbo.dlo` into `plugins`
- If you pick the game folder by hand, that path is saved to `%LOCALAPPDATA%\Holeshot HUD\gamedir.txt`
- The overlay uses the saved folder on launch so plugin updates still copy when Steam search would miss
- Uninstall removes the plugin from that saved folder

### Other

- Pushing a `Ship …` commit plus a `v*` tag runs only the Release workflow (Build and Pages skip)

## 0.1.2

Dash session clock, extra laps, and race flags from recent motos. Horizontal Standings bar.

### Dash and session

- Practice, gate, and race time share the dash clock slot
- Lap motos (4+ laps) show `1 / N` instead of leftover warmup minutes
- 3-lap motos show lap count when the session is not a standard timed length
- Timed extras stay `0 / N` until the **leader** crosses after time expires, then your crosses count
- Crossing the line as a backmarker right when time hits zero does not start extras

### Flags

- White in the last ~40 m before you start your last lap
- Checkered in the last ~40 m before you finish, then it holds until you leave the session
- Flag is a banner above the dash (no striped side panels over the widget)

### Other

- Horizontal Standings bar (leader on the left; your name is highlighted)
- Website widget demo uses imperial units (MPH, °F)

## 0.1.0

One-click Windows installer and a website download that always gets the latest release. HUD timing, flags, and dash polish from recent races.

### Install

- `HoleshotHUD-Setup.exe` installs the overlay, desktop shortcut, and MX Bikes plugin
- Website **Download for Windows** always fetches the latest tagged Setup.exe
- Pull requests build that same installer as a CI artifact
- Overlay copies `mxbo.dlo` into MX Bikes on launch if the game folder can be found

### Dash and session

- Practice and gate countdowns use the same dash slot as race time
- Timed +2L last lap waits for the second extra crossing (`1 / 2` then `2 / 2`)
- Clock stays at `00:00` until you cross or the leader puts a lap on you
- White flag is a top banner, not full-height side panels
- Flags only light on the real run-in to the line (about 8–70 m)

### Other

- Configurable dash footer and standings/relative header fields
- Minimap no longer flashes blank on sparse track segments
- No flags while stopped on the gate before the race
