# Changelog

## 0.1.18

Closing MX Bikes brings the Windows taskbar back.

### Overlay

- Closing the game restores the taskbar (it stayed hidden after a fullscreen session)

## 0.1.17

Settings is a working board with an orange Show plaque, you can install anywhere, and the HUD stays up while you tweak.

### Settings

- Top mode bar is **Widgets** / **Settings** / **Feedback**; the widget rail groups **Boards**, **Track**, and **Cockpit**
- Each widget pane opens on an orange name plaque with **Show on overlay** on the right — that is the only control until the widget is on
- Header and Footer are three slots (Left, Middle, Right)
- Columns keep drag, width, and show/hide; name columns can go wider
- Snap shows a live preview of where the widget will land
- Tab, arrows, Space, Enter, and Esc move through controls; Windows High Contrast is followed
- App → Updates shows the install folder
- If that folder looks protected (Program Files, not writable), auto-update and the update banner note that admin approval may be needed

### Overlay

- The HUD stays visible while Settings is in front so widget tweaks show live; alt-tab away from both still hides it
- Per-monitor DPI so Settings is not blurry on scaled displays

### Installer

- Setup shows a destination folder page so you can install somewhere other than the default `%LOCALAPPDATA%\Holeshot HUD`
- Uninstall follows that folder (and still clears AppData logs / the remembered game path)

## 0.1.16

Fresh installs start with every widget off, radar drops the blind-spot wedges, and the white flag matches the checkered look.

### Overlay

- The plugin no longer writes `show_*=1` into a new settings file. A blank install matches the overlay: all **Show on overlay** toggles start off until you turn them on
- Radar no longer draws the cream side/rear zone wedges behind your bike mark — only the panel, your bike, and proximity blips
- White flag banner uses diagonal stripes across the band (and the dash wrap), fading to a white plaque behind the icon and **WHITE FLAG** text — same layout idea as the checkered flag
- Each widget settings tab puts **Font size**, background / panel opacity, and **Bold text** right under **Show on overlay**
- With a widget off, its settings tab only shows **Show on overlay** until you turn it back on
- Standings / Relative column width sliders now change how wide each column draws (Name no longer silently fills leftover space)
- Standings and Relative have a **Row highlight** opacity slider (0–100%; 100% is a solid tint; default 50%)
- Standings and Relative **Text color** can be White or Black (bike pills keep their own brand ink)
- Bike pills use more padding and sit centered in the row
- Announce-shot PNGs regenerated from the live HUD (`cargo run -p mxbo-hud --example dump_announce_shots --release`)

## 0.1.15

Uninstall clears saved settings so a reinstall starts blank, and the plugin is branded Holeshot-HUD.

### Settings

- Uninstall removes `Documents\PiBoSo\MX Bikes\Holeshot-HUD.ini` (and leftover `mxbo.ini`), plus AppData logs and the remembered MX Bikes folder path. Reinstalling no longer keeps the old layout

### Overlay

- The MX Bikes plugin is `Holeshot-HUD.dlo` (not `mxbo.dlo`). Install and update remove any leftover `mxbo.dlo` so the game does not load both
- Layout file is `Documents\PiBoSo\MX Bikes\Holeshot-HUD.ini` (reads old `mxbo.ini` once, then saves under the new name)

## 0.1.14

No console window when you start the overlay.

### Overlay

- Launching from the desktop or Start menu no longer opens a black terminal. That was left on for F9 dump logging in 0.1.12 and rode through 0.1.13

## 0.1.13

Live race order, an update banner when auto-update is off, flag run-in polish, and a refreshed web demo.

### Settings

- If **Update automatically on launch** is off, a banner at the top of settings appears when a newer GitHub release is out, with **Update** or **Not now** (hides it until the next launch)

### Overlay

- Positions update the moment a pass happens instead of at the next start/finish line. MX Bikes only rescores the field when someone crosses the line, so standings, relative, dash `P#`, horizontal standings and the map/minimap all held the old order for up to a lap. The overlay now applies passes it can see on track: same lap, close together, and clearly in front (with a small margin so riders running side by side do not swap every frame). Practice and warmup are untouched, since that order is by lap time, and nothing moves on the gate
- Map and minimap marks follow those live positions: the leader crown moves when the lead changes hands, the green "nearest ahead" and red "nearest behind" rings move to whoever is actually ahead or behind you now, and dot position labels match
- Fixed a republished start board ending a timed race early. Mid-moto the game can drop a board value into the session clock for a moment (`04:43` → `00:05` → `04:42`), and the climb back out was read as the clock having run out: the dash switched to `0 / 2` extras with more than four minutes left, taking the lap counter and the flags with it. A clock that comes back where it left off is now the countdown resuming, while a real expiry still comes back at the session length. The board itself can still blink on the dash for a moment, because in a single frame it cannot be told from a clock that genuinely ran down that far
- Flags now appear as you come to the finish line, like a flagger waving them: white as you start your final lap, checkered as you come to the finish. Only actually crossing the line counts as finished, so slowing or crashing on the run-in takes the checkered back off
- White flag no longer sits on the dash for the whole final lap. It is waved as you cross onto the lap and comes down about five seconds later
- Flags no longer blink off and back on as you cross the line. The last few metres before the line were a hole in the approach window, which collapsed the banner and grew it again on the other side
- Checkered flag wraps the dash on the sides and bottom; the top is a white plaque with faded checkers and a crossed-flags icon

### Website

- Browser demo rebuilt with the current HUD renderer; Sectors stays hidden (labs flag off, same as the release overlay)

## 0.1.12

Sectors widget (experimental), flag and lap logic that matches real motos, and safer session data over shared memory.

### Settings

- **Sector times (experimental)** unlocks the Sectors widget and tab in release builds

### Overlay

- **Sectors** widget (S1â€“S3 times and delta vs best)
- Fixed the white/checkered flags flickering on timed races with extra laps (`8:00 + 2`). If the clock ran out mid-lap, the lap you were on did not count and the flags treated it as though it did, so white appeared a lap early; and a momentary glitch in the session fields could flash the checkered mid-race
- On a timed race with extras, the lap you are running when the leader starts the extras correctly counts for nothing, so `8:00 + 2` from a mid-lap expiry is one lap plus two extras
- Checkered flag waits until you cross the line instead of appearing on the run-in, on both lap motos and timed extras
- White flag now covers the run-in that starts your final lap and stays up for the whole lap, so it no longer disappears in the middle
- Getting lapped is handled: when the leader takes the finish the race is over, so you see white and then the checkered on your next crossing
- Dash shows a `~Lapped` tag next to the lap/clock text while you are a lap or more down, so it is obvious why your lap total shrank
- Getting lapped now shortens the lap total to the race you actually run, so a 5-lap moto you finish a lap down reads `4 / 4` instead of `4 / 5`. Timed races shrink their extra laps the same way
- The start/finish position for the flag window is learned from your own lap crossings rather than trusting the plugin's value, which is missing on tracks that send no centerline and put the window at the wrong point on track
- **Laps left** counts the lap you are on, so the final lap reads `1` instead of `0`, and it can no longer disagree with the flags
- 2-lap motos no longer show as timed `+2` extras when a leftover start board (`00:50`) is sitting in session length
- Race trace (`race.jsonl`) records the flag on the dash and the laps remaining, so a flag complaint can be diagnosed from a log
- F9 SHM dump includes `session_kind` / `session_state` (`m_iSession`, `m_iSessionState`) so warmup vs race 1 vs race 2 can be labeled from a log
- Plugin session length is `-1` until the current session writes it, so leftover warmup `8:00` is not kept when a lap moto publishes `0`
- Shared memory mapping is `Local\MXBOHudV9` with a size check on open, so a leftover smaller section cannot be overrun inside the game process

## 0.1.11

Dash numbers stay put as RPM changes, bug reports send during a moto, and the overlay can follow MX Bikes.

### Settings

- **Close when MX Bikes closes** quits the overlay a few seconds after the game exits
- **Open when MX Bikes opens** starts the overlay when MX Bikes launches, including after a reboot

### Overlay

- Dash RPM/speed column width is reserved from the widest digits, so gear and position do not shift as RPM ticks
- Bug reports attach the latest 700 KB of the race log so a send works mid-session

## 0.1.10

Settings dropdowns stay over the rows below them, options are Aâ€“Z, and plus/minus buttons use Font Awesome so they show in Exo 2.

### Settings

- Open dropdowns overlay the content under them instead of pushing the next row down
- Dropdown options sort Aâ€“Z, with **None** first when the list has it
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

- **F8 â†’ App â†’ Feedback** now has Rate, Bug, and Feature
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

- **F8 â†’ App â†’ Feedback:** rate the app (1â€“5 stars) or report a bug
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

- Setup still finds Steam MX Bikes from the registry and library folders, then copies `Holeshot-HUD.dlo` into `plugins`
- If you pick the game folder by hand, that path is saved to `%LOCALAPPDATA%\Holeshot HUD\gamedir.txt`
- The overlay uses the saved folder on launch so plugin updates still copy when Steam search would miss
- Uninstall removes the plugin from that saved folder

### Other

- Pushing a `Ship â€¦` commit plus a `v*` tag runs only the Release workflow (Build and Pages skip)

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
- Website widget demo uses imperial units (MPH, Â°F)

## 0.1.0

One-click Windows installer and a website download that always gets the latest release. HUD timing, flags, and dash polish from recent races.

### Install

- `HoleshotHUD-Setup.exe` installs the overlay, desktop shortcut, and MX Bikes plugin
- Website **Download for Windows** always fetches the latest tagged Setup.exe
- Pull requests build that same installer as a CI artifact
- Overlay copies `Holeshot-HUD.dlo` into MX Bikes on launch if the game folder can be found

### Dash and session

- Practice and gate countdowns use the same dash slot as race time
- Timed +2L last lap waits for the second extra crossing (`1 / 2` then `2 / 2`)
- Clock stays at `00:00` until you cross or the leader puts a lap on you
- White flag is a top banner, not full-height side panels
- Flags only light on the real run-in to the line (about 8â€“70 m)

### Other

- Configurable dash footer and standings/relative header fields
- Minimap no longer flashes blank on sparse track segments
- No flags while stopped on the gate before the race
