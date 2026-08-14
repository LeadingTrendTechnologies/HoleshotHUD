# MX Bikes HUD (mxbo)

Standings, relative, and a track map for MX Bikes.

There are two layers:

1. **`mxbo.dlo`** — in-game plugin. Reads telemetry and writes a shared-memory snapshot (`Local\MXBOHudV1`).
2. **`mxbo-overlay`** — a transparent Rust window (tiny-skia) that sits on top of the game and draws anti-aliased lines and real Windows fonts.

In-game drawing is **off** by default (`ingame_hud=0`) so the plugin stays cheap. The overlay is what you look at.

## Build

Visual Studio 2022 x64 tools, plus [Rust](https://rustup.rs/) for the overlay.

```bat
build.bat
```

That installs `mxbo.dlo` into `MX Bikes\plugins\`. Overlay exe:

`overlay\target\release\mxbo-overlay.exe`

## Use

1. Set MX Bikes to **borderless** or **windowed** (exclusive fullscreen will cover the overlay).
2. Start the game (loads the plugin).
3. Start `mxbo-overlay.exe`.
4. Hold **Ctrl** and drag still works only if you turn in-game HUD back on.

Layout still lives in `Documents\PiBoSo\MX Bikes\mxbo.ini`. Overlay widgets follow those normalized 0..1 rects.

```
ingame_hud=0
show_map=1
show_standings=1
show_relative=1
```

Quit the overlay by closing its process (it is click-through so it will not steal mouse input).

## Data wiki

Every plugin callback and field, plus what we already store vs ignore:

[wiki/Home.md](wiki/Home.md)
# mx-bikes-overlay
