# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

MX Bikes racers in a moto or practice session. The job is race awareness — gaps, nearby riders, place on track, flags — without taking eyes off the track.

## Product Purpose

Holeshot HUD is a Windows overlay for [MX Bikes](https://store.steampowered.com/app/655500/MX_Bikes/). A game plugin reads telemetry; a transparent window draws standings, relative, map, minimap, radar, dash, and related widgets on top of the game.

Success is glanceable, trustworthy race information during a session — not a second product to manage.

## Positioning

A broadcast-style race HUD MX Bikes does not ship: standings, relative, map/minimap, radar, dash, and live pass order, delivered through a PiBoSo plugin and a transparent overlay. Generic sim HUDs and in-game draw cannot truthfully claim that stack.

## Operating Context

- Windows only. MX Bikes must be **borderless** or **windowed** (not exclusive fullscreen).
- Install: `HoleshotHUD-Setup.exe` → `%LOCALAPPDATA%\Holeshot HUD` (or a chosen folder). Setup copies `Holeshot-HUD.dlo` into the game `plugins` folder.
- Start the game, then start **Holeshot HUD**. **F8** opens settings. Hold **Ctrl** and drag to move or resize widgets.
- Layout lives in `Documents\PiBoSo\MX Bikes\Holeshot-HUD.ini`. Restart MX Bikes after installing or updating the plugin.
- The website (`web/`, hosted at holeshot-hud.vercel.app) is a widget demo plus a Windows download — not the product.

## Capabilities and Constraints

- Widgets: Standings, Relative, Horizontal Standings, Map, Minimap, Radar, Dash, Systems, Stance. Sectors are labs-only until **Experimental widgets** is on.
- Fresh install: every **Show on overlay** toggle starts off. Nothing draws until the rider turns it on. Widgets only draw during a session (not in the garage, lobby, or menus). Stance follows a local bind, not rider animation.
- Optional simpler in-game HUD (`ingame_hud`) draws inside the game; overlay work does not go there.
- Architecture: `Holeshot-HUD.dlo` → shared memory `Local\MXBOHudV9` → Rust overlay. Field availability is gated by the PiBoSo plugin API (see `wiki/Home.md`). Bump `MXBO_SHM_VERSION` when the snapshot layout changes.
- Auto-update from GitHub releases; after an in-app update, Settings shows a What's new board for this version. In-app feedback (F8 → App) posts private gists.
- MX Bikes only. Not a general sim HUD.

## Brand Commitments

- Product name is **Holeshot HUD** (not MXBO Overlay). Executable and plugin: `Holeshot-HUD.exe` / `Holeshot-HUD.dlo`.
- Wordmark and mark: `web/logo.png`, `web/logo.svg`.
- Icons: Font Awesome Free.
- Publisher/repo: LeadingTrendTechnologies / HoleshotHUD.

## Evidence on Hand

- Widget announce shots: `announce-shots/*.png`.
- Browser demo with the same HUD renderer: `web/` (demo data, not live telemetry).
- Installer copy: `installer/README.txt`.
- Plugin and widget wikis: `wiki/Home.md`, `wiki/widgets.md`.

Do not fabricate testimonials, customers, league endorsements, benchmarks, or download counts.

## Product Principles

- Eyes stay on the track: every HUD element must be glanceable at race speed.
- Opt-in chrome: a blank install shows nothing until the rider enables it.
- Race truth over the game’s delayed scoreboard: passes, flags, and clocks must match what is happening on track.
- MX Bikes native: plugin, shared memory, and overlay constraints are the product, not an implementation detail.
- The website demonstrates the HUD; it is not a second product to design for.
