# MXBO Overlay

Standings, relative, horizontal standings, map, minimap, and radar for [MX Bikes](https://store.steampowered.com/app/655500/MX_Bikes/).

## Preview

A browser demo of the same HUD widgets lives in [`web/`](web/). Build it with `build-web.bat`, then serve the `web` folder.

**Vercel:** import this GitHub repo (root directory = repo root). `vercel.json` publishes `web/`. After you change the HUD, run `build-web.bat` and commit the updated `web/pkg` files before you deploy.

## Download and install

1. Download **[HoleshotHUD-Setup.exe](https://github.com/LeadingTrendTechnologies/HoleshotHUD/releases/latest/download/HoleshotHUD-Setup.exe)** (always the latest release).
2. Run the Setup file. If SmartScreen appears, choose **More info → Run anyway**. Setup installs to `%LOCALAPPDATA%\Holeshot HUD` (no admin).
3. Setup finds Steam MX Bikes (registry + Steam library folders) and copies `mxbo.dlo` into the game `plugins` folder. If the game is not found, pick the folder that contains `mxbikes.exe`.
4. Set MX Bikes to **borderless** or **windowed**.
5. Start the game, then start **Holeshot HUD** from the desktop shortcut.
6. Press **F8** for settings. Hold **Ctrl** and drag widgets to move or resize.

The overlay copies the plugin again on launch if it is missing or outdated. The chosen game folder is saved next to the overlay so a manual pick still works later.

Pull requests also build `HoleshotHUD-Setup.exe` as a workflow artifact for testing. That does not change the website download until you publish a `v*` tag.

Uninstall from Windows Settings, or run `Uninstall.bat` in `%LOCALAPPDATA%\Holeshot HUD`.

## Use

The plugin (`mxbo.dlo`) reads the game and writes shared memory. The overlay is a transparent window on top of MX Bikes.

Layout is saved to `Documents\PiBoSo\MX Bikes\mxbo.ini`.

Restart MX Bikes after installing or updating the plugin.

## Build from source

Needs Visual Studio 2022 (C++ desktop) and [Rust](https://rustup.rs/).

```bat
build.bat
pack.bat
```

`build.bat` compiles `out\Release\mxbo.dlo` and `overlay\target\release\Holeshot-HUD.exe`.  
`pack.bat` writes `dist\HoleshotHUD-Setup.exe` (needs [Inno Setup 6](https://jrsoftware.org/isinfo.php)).

Push a tag to publish a downloadable release:

```bat
git tag v0.1.5
git push origin v0.1.5
```

## Data wiki

Plugin callbacks and fields: [wiki/Home.md](wiki/Home.md)
