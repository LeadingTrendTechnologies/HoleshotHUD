# MXBO Overlay

Standings, relative, horizontal standings, map, minimap, and radar for [MX Bikes](https://store.steampowered.com/app/655500/MX_Bikes/).

## Preview

A browser demo of the same HUD widgets lives in [`web/`](web/). Build it with `build-web.bat`, then serve the `web` folder.

**Vercel:** import this GitHub repo (root directory = repo root). `vercel.json` publishes `web/`. After you change the HUD, run `build-web.bat` and commit the updated `web/pkg` files before you deploy.

In-app feedback (F8 → Feedback) posts to `/api/feedback` and stores **private gists**, not GitHub issues. Add these Vercel env vars:

- `FEEDBACK_GITHUB_TOKEN` — classic PAT with the **gist** scope
- `FEEDBACK_INBOX_SECRET` — password for [the inbox](https://holeshot-hud.vercel.app/inbox.html)

Open the inbox to filter ratings, bugs, and feature asks. Each card shows **first** version (or that they were using the app before that was tracked). **Done** archives that gist (it stays so the overlay can still show your reply). Open Details and **Send** a follow-up — it pops up in their settings. They can type an answer and send it back. Waiting means they wrote back to you.

## Download and install

1. Download **[HoleshotHUD-Setup.exe](https://github.com/LeadingTrendTechnologies/HoleshotHUD/releases/latest/download/HoleshotHUD-Setup.exe)** (always the latest release).
2. Run the Setup file. If SmartScreen appears, choose **More info → Run anyway**. Default install is `%LOCALAPPDATA%\Holeshot HUD` (no admin); Setup lets you pick another folder.
3. Setup finds Steam MX Bikes (registry + Steam library folders) and copies `Holeshot-HUD.dlo` into the game `plugins` folder. If the game is not found, pick the folder that contains `mxbikes.exe`.
4. Set MX Bikes to **borderless** or **windowed**.
5. Start the game, then start **Holeshot HUD** from the desktop shortcut.
6. Press **F8** for settings. Hold **Ctrl** and drag widgets to move or resize.

The overlay copies the plugin again on launch if it is missing or outdated. The chosen game folder is saved next to the overlay so a manual pick still works later.

Pull requests also build `HoleshotHUD-Setup.exe` as a workflow artifact for testing. That does not change the website download until you publish a `v*` tag.

Uninstall from Windows Settings, or run `Uninstall.bat` in the install folder.

## Use

The plugin (`Holeshot-HUD.dlo`) reads the game and writes shared memory. The overlay is a transparent window on top of MX Bikes.

Layout is saved to `Documents\PiBoSo\MX Bikes\Holeshot-HUD.ini`. Uninstall deletes that file and AppData leftovers so a reinstall is a fresh install.

Restart MX Bikes after installing or updating the plugin.

## Build from source

Needs Visual Studio 2022 (C++ desktop) and [Rust](https://rustup.rs/).

From **cmd**:

```bat
build.bat
pack.bat
```

From **Git Bash**, prefix with `cmd.exe //c` so `/c` is not treated as a path:

```bash
cmd.exe //c build.bat
```

`build.bat` compiles `out\Release\Holeshot-HUD.dlo` and `overlay\target\release\Holeshot-HUD.exe`.  
`pack.bat` writes `dist\HoleshotHUD-Setup.exe` (needs [Inno Setup 6](https://jrsoftware.org/isinfo.php)).

If cargo fails with **Access is denied** on `Holeshot-HUD.exe`, quit the overlay (tray → **Quit overlay**) or, from Git Bash:

```bash
cmd.exe //c "taskkill /IM Holeshot-HUD.exe /F"
```

### Local debug

`cargo run` and `build.bat` keep every overlay widget off until **Show on overlay** is on. **Sectors**, **Delta Bar**, **Lean**, **Stance**, and **Flags** are regular Cockpit widgets.

```bash
cmd.exe //c "cargo run --manifest-path overlay\Cargo.toml --bin Holeshot-HUD"
```

Dev builds optimize crates like tiny-skia (`opt-level = 3`) so the HUD stays smooth. The first `cargo run` after a clean build takes longer; later ones are incremental. For a full release binary, use `build.bat`.

Push a tag to publish a downloadable release:

```bat
git tag v0.6.0
git push origin v0.6.0
```

## Data wiki

Plugin callbacks and fields: [wiki/Home.md](wiki/Home.md). Overlay widget behavior and change history: [wiki/widgets.md](wiki/widgets.md).
