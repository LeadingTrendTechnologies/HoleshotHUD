# Streaming

**Browser Source + `/edit`** — OBS paints `/`; stream Show / move / resize / basic prefs live at `/edit`. F8 configures the in-game HUD only. When you change this page, append a **Change log** bullet (why, not just what).

Widget wishlist: [widgets/future.md](widgets/future.md). Shipped widgets: [widgets.md](widgets.md).

## Today

OBS **Game Capture** of MX Bikes misses the layered Holeshot HWND. Streamers turn on **Settings → Stream → Browser Source**, copy `http://127.0.0.1:<port>/` into OBS, and open `http://127.0.0.1:<port>/edit` in Chrome to toggle and place stream widgets.

Four stream layouts (Practice / Warmup / Race / Spectate). The Browser Source follows `session_preset()` on the **stream** slots. The editor chips pick which stream slot you are editing.

The overlay binds localhost only (default port **8765**, through **8775** if busy), serves a transparent OBS page, pushes live PNG frames from `draw()` using `for_stream()`, and serves `/edit` with a second PNG feed plus HTML chrome.

`web/index.html` remains the marketing twin editor with **demo data**.

## Why Browser Source

| Approach | Stream-only? | Streamer work |
| --- | --- | --- |
| Local HTTP + OBS **Browser Source** | Yes | Paste `/`, set 1920×1080 |
| Companion / second HWND | Later | Second monitor + Window Capture |
| Capture the overlay window | No | Display Capture or Window Capture of Holeshot HUD |

Edit layout in the browser `/edit` page. OBS only paints `/`. Do not put edit chrome on the OBS URL.

## Intended streamer steps

1. F8 → Settings → **Stream** → **Browser Source** On (copy OBS `/` URL).
2. OBS → Browser Source → paste `/` → width/height = canvas (1920×1080 default).
3. Open `/edit` in a normal browser → pick Practice / Warmup / Race / Spectate → **Show on stream**, tune settings in the subnav, drag boxes on the canvas.
4. Optional: F8 Stream pane **Copy game → stream** seeds that chip’s stream slot from the game layout.

Port conflict → next port; the Stream pane shows it. Localhost only.

## Layout: add, remove, move

OBS places **one** Browser Source (the whole 16:9 canvas). Moving/cropping the source moves every widget. One OBS URL; the page follows the live session.

**Four stream layouts**, not a fifth chip. Practice / Warmup / Race / Spectate each keep an in-game `HudLayout` **and** a stream `HudLayout`.

| Action | Where | What happens |
| --- | --- | --- |
| Add / remove | `/edit` left nav toggles | That preset’s browser page shows or hides the widget. Game HUD unchanged. |
| Move / resize | `/edit` canvas drag | Orange boxes over the live stream PNG. Writes the **stream** slot for that chip. |
| Widget prefs | `/edit` settings subnav | Font / bold / background for the selected stream widget (v1). |
| Seed | F8 Stream → Copy game → stream | Copies the open chip’s game layout onto its stream slot. |
| OBS | Place / scale the one Browser Source | Nudge the whole HUD. Not per-widget. |

Fresh install: every stream **Show** starts off. Coords stay normalized 0–1. F8 Ctrl-drag only moves the **game** board.

## Build notes

- Tiny HTTP server in the overlay process (`overlay/src/stream/`), localhost only.
- PNG over `/ws` (OBS, live session stream board) and `/ws/edit` (editor preset).
- `GET`/`POST` `/api/stream-layout` for Show, rects, and basic prefs.
- INI: `[PracticeStream]` / `[WarmupStream]` / `[RaceStream]` / `[SpectateStream]`.

A second-monitor companion window can come later.

## Do not

- Do not treat Window Capture of **Holeshot HUD** as stream-only.
- Do not draw a stream layout on the game HWND.
- Do not put edit chrome on the OBS `/` URL.
- Do not serve past localhost.
- Do not collapse the four stream layouts into one board. Do not add a fifth Stream chip.

## Change log

- 2026-09-24 — Stream layout editor at `/edit` (Show, drag/resize, basic prefs); OBS stays paint-only `/`; F8 Game|Stream surface removed.
- 2026-09-24 — Stream surface edit chrome is orange Ctrl-drag boxes only; stream Shows never paint the game HWND (Browser Source / OBS only).
- 2026-09-24 — Browser Source keeps painting while you alt-tab; it does not follow game-overlay z-order, and holds the last frame across brief SHM gaps.
- 2026-09-24 — Browser Source stamps stream-live Map/Standings/Relative show+rects onto a snapshot copy before draw (those widgets gate on SHM fields, not cfg show).
- 2026-09-24 — F8 Game/Stream chips only when Browser Source is on; hairline divider before preset chips; turning Stream off forces Game surface.
- 2026-09-24 — Phase 2: four stream HudLayouts, F8 Game/Stream surface, Show on stream, Ctrl-drag → stream edit slot, Browser Source uses `for_stream()`.
- 2026-09-24 — Phase 1: Settings → Stream Browser Source, localhost HTTP + WS PNG feed, live game layout.
- 2026-09-10 — Four stream layouts (Practice / Warmup / Race / Spectate), switched with `session_preset()`. F8 Game / Stream surface. Not a fifth chip.
- 2026-09-10 — Page created. Browser Source is the intended path. Not shipped.
