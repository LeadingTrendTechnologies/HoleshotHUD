# MXBO Overlay

Standings, relative, map, minimap, and radar for [MX Bikes](https://store.steampowered.com/app/655500/MX_Bikes/).

## Download and install

1. Get the latest **MXBO-Overlay-*-windows-x64.zip** from [Releases](https://github.com/troyerl/mx-bikes-overlay/releases).
2. Unzip it and run **Install.bat**.
3. If SmartScreen appears, choose **More info → Run anyway**.
4. Set MX Bikes to **borderless** or **windowed**.
5. Start the game, then start **MXBO Overlay** from the desktop shortcut.
6. Press **F8** for settings. Hold **Ctrl** and drag widgets to move or resize.

You can also grab a zip from the latest [Actions](https://github.com/troyerl/mx-bikes-overlay/actions) run if no release is published yet.

Uninstall with `Uninstall.bat`, or from `%LOCALAPPDATA%\MXBO Overlay`.

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

`build.bat` compiles `out\Release\mxbo.dlo` and `overlay\target\release\mxbo-overlay.exe`.  
`pack.bat` writes `dist\MXBO-Overlay-<version>-windows-x64.zip` for other PCs.

Push a tag to publish a downloadable release:

```bat
git tag v0.1.0
git push origin v0.1.0
```

## Data wiki

Plugin callbacks and fields: [wiki/Home.md](wiki/Home.md)
