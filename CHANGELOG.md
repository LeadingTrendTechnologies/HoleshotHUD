# Changelog

## 0.2.0

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
