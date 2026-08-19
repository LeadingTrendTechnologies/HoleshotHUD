# Changelog

## 0.1.11

Dash numbers stay put as RPM changes, bug reports send during a moto, and the overlay can follow MX Bikes.

### Settings

- **Close when MX Bikes closes** quits the overlay a few seconds after the game exits
- **Open when MX Bikes opens** starts the overlay when MX Bikes launches, including after a reboot

### Overlay

- Dash RPM/speed column width is reserved from the widest digits, so gear and position do not shift as RPM ticks
- Bug reports attach the latest 700 KB of the race log so a send works mid-session

## 0.1.10

Settings dropdowns stay over the rows below them, options are A–Z, and plus/minus buttons use Font Awesome so they show in Exo 2.

### Settings

- Open dropdowns overlay the content under them instead of pushing the next row down
- Dropdown options sort A–Z, with **None** first when the list has it
- Stepper plus and minus use Font Awesome icons so they render in Exo 2

## 0.1.9

Systems widget, race fonts with Exo 2 as the default, crash marks on radar, and warmup that no longer looks like a race.

### Settings

- **Settings key** on the Settings tab picks which key opens settings (F8 still the default; Medal uses F8)
- **Systems** tab for CPU, memory, FPS, and per-app load
- Font picker: **Exo 2**, **Teko**, **Goldman**, and **Montserrat** replace Agency FB, Industry, and Faster One. **Exo 2** is the default (overlay and website demo)

### Overlay

- **Systems** shows host load (HUD, MX Bikes, ReShade) even in the menu
- Radar uses the same crash icon as the map when a rider is down
- Warmup keeps slate map/minimap dots and un-tinted relative rows. Practice laps are not lapping
- Horizontal Standings title is **WARMUP** during practice, not TIMED or LAP RACE

### Dash and session

- **Rev indicator** can be turned off
- 4-lap motos ignore leftover `08:00` and show laps
- A later start board after `00:10` stays a countdown instead of flashing frozen `08:00`

## 0.1.8

Tab icons, widgets off until you turn them on, and uninstall from Settings.

### Settings

- Sidebar tabs have icons, including distinct ones for each widget
- Rate, Bug, and Feature on the Feedback tab have icons
- **Uninstall** next to Quit overlay removes the app, the MX Bikes plugin, and shortcuts

### Overlay

- A first install starts with every widget hidden. Turn each one on with **Show on overlay**

## 0.1.7

Startup options, opt-in auto-update on launch, and settings that come to the front.

### Settings

- Opening the app from the exe, taskbar, or tray brings Settings on top so you can see it
- A tray icon is added while the overlay is running; click it to open Settings
- **Open when Windows starts** and **Minimize on close** are on the Settings tab
- Minimize on close hides Settings instead of quitting; F8 or the tray brings it back, **Quit overlay** exits
- Feedback is its own tab under General

### Updates

- **Update automatically on launch** is off until you turn it on
- When it is on, a newer GitHub release is installed before the window opens

## 0.1.6

Private feedback inbox, a feature-ask tab, and race logs that survive until after the moto.

### Feedback

- **F8 → App → Feedback** now has Rate, Bug, and Feature
- Sends go to a private inbox, not public GitHub issues
- Bug reports snapshot the race log in memory so a send works while the overlay still has the file open
- The current moto is kept in full (2 MB cap). When it ends, that log is saved as `last-race.jsonl`, so a bug from the start of the race can still be reported after the checkered flag
- The text caret sits on the letters you typed

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
