# Changelog

## Unreleased

## 0.3.3

Fully quit MX Bikes after a plugin update if the game was already open.

### Overlay

- If an update changes the MX Bikes plugin while the game is still running, What's new and the HUD ask you to fully quit MX Bikes so the new plugin can load. Overlay-only updates do not.

## 0.3.2

Widgets show after an update.

### Overlay

- After an update, a blank HUD with only the top-right mark now explains itself: widgets appear on track, or MX Bikes needs a full restart so the plugin can load.
- Running the app again replaces a leftover tray instance, so a rebuild or update actually starts.
- The plugin baked into the overlay wins over a leftover file next to the exe, so an update cannot keep an old MX Bikes plugin.

## 0.3.1

### Internals

- Minimize on close hides Settings to the tray. The tray icon or settings key brings it back.
- Open when Windows starts and starting with the game both open Settings in the tray.
- With close-with-game and open-with-game both on, leaving a session keeps the overlay in the tray so it comes back when you race again.
- A short freeze in MX Bikes no longer blanks overlay widgets.
- Ctrl+resize of standings and relative grows the Name column and row count. The orange box is the plaque, not leftover empty glass.

## 0.3.0

Range rings on radar so close vs far is a glance.

### Delta Bar

- Best lap tape is saved per class on that track. Every 250 shares one time; a 450 is separate. Yamaha 250 and Honda 250 use the same tape.

### Sectors

- Split times vs your best are per class on that track (all 250s together, all 450s together). **Clear this track** still wipes every class on that file.

### Systems

- Twin columns: CPU left, MEM right, process rows under each, FPS and NET in a footer. Gold heat tracks (red when hot). No green bars.
- Game FPS follows the game's draw rate instead of bouncing with the overlay loop.

### Radar

- Hairline 3 / 6 / 12 m circles centered on your bike, with dim **6** and **12** at the bottom. Rings lighten when the panel is solid so they still read at 100% background. **Range rings** can be turned off. Blips are larger, orange close / cream farther out. No wedges.

### Dash

- Default size is the compact in-game lockup (~11.1%×11.5%).
- Fuel is a footer option: liters/gallons, or percent.
- 8:00+1 that publishes extras late and resets standings no longer sticks on `1/1` or waves the checkered three laps early. After the clock: `0/1` until you start the extra, `1/1` on that lap.

### Standings

- Fuel is a header and footer option: liters/gallons, or percent.

### Map / Minimap

- Sector lines mark where each sector **starts**: **S1** at the start/finish line, **S2** / **S3** at the learned splits. They used to sit on the split that ended S1 / S2.

### Map / Minimap / Relative

- Getting lapped a second time no longer paints the leader red. Blue/red uses laps behind the leader, not a completed-lap count that can sit on the race lap.

### Relative

- Fuel is a header and footer option: liters/gallons, or percent.

### H-Standings

- Fuel is a side-slot option: liters/gallons, or percent.

### Website

- Radar in the demo has **Range rings** (on by default) and the new rings/blips.
- Systems in the demo is twin columns with gold heat.
- Fuel and Fuel % are in the board and dash field pickers.

## 0.2.0

White and checkered flags as their own overlay widget.

### Flags

- New **Flags** widget. A skew plaque — white stripes or checkers, same timing as the Dash wrap. When no flag is up, it draws nothing. Dash wrap stays on.
- Optional **Yellow flag** and **Blue flag** (each off by default). Yellow when a rider has crashed ahead of you and close; blue when someone a lap up is close behind. Dash wrap stays white and checkered only.
- Default size is a slim top-center strip (~11%×2%). Caption white matches the Dash wrap fade, with extra pad beside the text. Cloth shows above and below the white band.
- Place it anywhere (default top-center). Hold Ctrl and drag. Hidden until **Show on overlay**.

### Website

- Widget rail has an **Experimental** group for Sectors and Delta Bar.
- Sectors in the demo keep ticking through S2 and S3.
- Dash starts a bit wider so **~Lapped** stays on the plaque. Simple dash uses a compact size.
- Cockpit rail includes **Flags**. The demo cycles checkered, white, and hidden. Turn on **Yellow flag** or **Blue flag** to see those too.

## 0.1.20

Delta Bar vs your best, live sector splits, and a board when we reply to feedback.

### Delta Bar

- Labs unlocks Delta Bar. Time vs your best at this point on the lap. After one decent lap — or a saved tape from last time — it is live. It is not the in-game ghost.
- First lap shows **REC** while it fills if nothing is saved, with **complete two full laps** under the hairline. A faster lap updates the saved tape. A dab does not throw the lap away.
- **BEST** and **LAST** under the hairline are larger. A new personal best swaps LAST for orange **NEW BEST** and the time for a few seconds.
- Delta is a hairline: orange Δ, the signed time, a 2px center-zero line, BEST and LAST on the ends. No plaque. Turn up Panel opacity if you want a plate behind it.
- When Panel opacity is under 40% (the Hair default is 0), BEST and LAST sit on small night-ink pills so they stay readable on the game.
- The hairline lerps across empty tape bins and damps on a 0.4 s time constant, so it does not jump with track-pos noise.
- Coming back to a track with a saved tape, the first flying lap no longer opens at a fake +16s. That was the out-lap clock compared at the line.
- The out-lap is not recorded. The tape starts when you cross S/F (a new last-lap time), even if that line is not at track position 0.

### Sectors

- Sectors is a three-column strip: the sector you are in is the wide cell, with a live delta that freezes when you leave.
- Live delta is vs your best **at this point in the sector** (same tape as Delta Bar), not elapsed vs the full split.
- **Live sector** (on by default) ticks the current cell. Turn it off to only show a time after each split.
- After a lap, last-lap times stay until the next lap clock runs.
- Completing a lap fills S3 and keeps it the orange cell until you start the next lap.
- Delta is vs. your best on this track (saved), not the in-game ghost. The plaque says **vs. your best**.
- Faster splits were showing 0.000. The game fires two split events; the second one compared against the time just recorded.
- Best S1/S2/S3 and the delta-bar tape are saved per track name under AppData. A faster frozen split or lap updates that file. **Clear this track** in Labs wipes it.
- Split times at the bottom of each cell sit on night-ink pills, same as Delta Bar BEST / LAST.

### Standings and Relative

- Standings and Relative can turn off alternating row colors.
- Standings and Relative keep striped rows unless you turn Alternating rows off.
- Alternating rows still show when Background is 100%.
- A new install starts Standings and Relative narrower so they match the default columns.
- Standings and Relative plaques hug the columns — extra widget width is not empty glass.

### Map and Minimap

- Map and minimap treat the rider you are watching as you: orange dot on them, minimap follows. Leave spectate and they follow you again.
- Map and minimap can show thin violet dotted **S1** / **S2** sector lines. Turn them with **Sector lines**. They appear after the overlay has seen those splits on this track.

### Dash

- Default dash matches the compact in-game size. Hold Ctrl and drag the orange handles to resize — the plaque fills that box.

### Overlay

- While you ride, those clicks go to the game — except the Holeshot HUD mark in the top-right, which opens settings (including while you hold Ctrl to place widgets).
- The Holeshot HUD icon sits in the top-right while the overlay is on MX Bikes. If you can see it, the HUD is working. Click it to open settings. The settings key (F8 by default) opens settings, and presses again to close.
- **Open when MX Bikes opens** starts the overlay in the tray. It does not pop the settings window.
- If no widget is on, a plaque on the game says to press F8 and turn on Show on overlay (widgets still only draw on track).
- If the plugin could not update because MX Bikes was already open, a plaque says to fully quit the game and start it again. A clean start after the overlay copied the plugin does not show that plaque.

### Settings

- App → Settings shows the MX Bikes folder and whether the plugin is in `plugins`, with Change folder.
- When we reply to feedback you sent, a board pops up the next time you open settings. You can write back with Send.

### Website

- Standings and Relative settings include Alternating rows.

### Internals

- Track PB JSON stores `used` (unix seconds last ridden). Stamped on a faster lap/split and when you visit a track that already has a file (at most hourly). No empty files for tracks with no PB.
- Feedback includes first-install version. New settings files stamp this build; existing installs are `unknown` so they are not treated as new.

## 0.1.19

Follow a rider in replay, sit or stand on the overlay, and see what changed after an update.

### Settings

- After an update, a board lists what changed. Open it again from Settings → Updates.
- Labs unlocks Sectors. Turn it on with Show on overlay.
- Stance is sit/stand. It lives with Dash and Systems. Click Sit button and the row turns orange: press a pad, key, or mouse now. Sitting hides it unless Show sitting is on. Settings says it is not connected to the game — only the bind you set.
- Dash can be just gear and speed. Turn on Simple dash; flags still wrap it.

### Overlay

- In replay, click a name or standings card to follow that rider.
- While you ride, those clicks go to the game.
- The HUD stays up in replay and hides in menus. Systems and Stance hide there too, and when MX Bikes is not running.
- Leave replay and the HUD follows you again.
- Simple dash is an orange gear plaque and speed. Flags still wrap it.

### Website

- Demo is a pit box: settings on the left, widget rail, live HUD on the right, Download for Windows on the top bar.
- Dash settings include Simple dash. The live stage draws the compact gear-and-speed lockup.

### Internals

- Overlay writes spectate clicks on `Local\MXBOHudCmdV1`; the plugin consumes them in `SpectateVehicles`.
- Stance mirrors the sit pad button (not rider animation from the plugin API).

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
