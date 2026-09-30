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
| Local HTTP + OBS **Browser Source** | Yes | Paste `/` at the stream size |
| Companion / second HWND | Later | Second monitor + Window Capture |
| Capture the overlay window | No | Display Capture or Window Capture of Holeshot HUD |

Edit layout in the browser `/edit` page. OBS only paints `/`. Do not put edit chrome on the OBS URL.

## Intended streamer steps

1. F8 → Settings → **Stream** → **Browser Source** On (copy OBS `/` URL).
2. OBS → Browser Source → paste `/` → set width and height to the size shown on `/edit`, then scale that source to the canvas. Do not set the source to the game resolution. A 3440×1440 source makes the page composite at that size, which is why OBS lags while `/edit` stays smooth.
3. Open `/edit` in a normal browser → pick Practice / Warmup / Race / Spectate → **Show on stream**, tune settings in the subnav, drag boxes on the canvas.
4. Optional: F8 Stream pane **Copy game → stream** seeds that chip’s stream slot from the game layout.

Port conflict → next port; the Stream pane shows it. Localhost only.

## Layout: add, remove, move

OBS places **one** Browser Source at the paint size from `/edit`, then you scale that source to the canvas. Moving or cropping the source moves every widget. `/edit` on another monitor letterboxes that same aspect. One OBS URL; the page follows the live session.

**Four stream layouts**, not a fifth chip. Practice / Warmup / Race / Spectate each keep an in-game `HudLayout` **and** a stream `HudLayout`.

| Action | Where | What happens |
| --- | --- | --- |
| Add / remove | `/edit` left nav toggles | That preset’s browser page shows or hides the widget. Game HUD unchanged. |
| Move / resize | `/edit` canvas drag | Orange boxes over the live stream PNG. Writes the **stream** slot for that chip. |
| Widget prefs | `/edit` settings subnav | Font, bold, background, and the same widget controls as F8, written on the stream layout only. |
| Seed | F8 Stream → Copy game → stream | Copies the open chip’s game layout onto its stream slot. |
| OBS | Place / scale the one Browser Source | Nudge the whole HUD. Not per-widget. |

Fresh install: every stream **Show** starts off. Coords stay normalized 0–1. F8 Ctrl-drag only moves the **game** board.

## Build notes

- Tiny HTTP server in the overlay process (`overlay/src/stream/`), localhost only.
- PNG over `/ws` (OBS, live session stream board) and `/ws/edit` (editor preset).
- `GET`/`POST` `/api/stream-layout` for Show, rects, font, and each widget’s F8 controls on the stream slot.
- INI: `[PracticeStream]` / `[WarmupStream]` / `[RaceStream]` / `[SpectateStream]`.

A second-monitor companion window can come later.

## Do not

- Do not treat Window Capture of **Holeshot HUD** as stream-only.
- Do not draw a stream layout on the game HWND.
- Do not put edit chrome on the OBS `/` URL.
- Do not serve past localhost.
- Do not collapse the four stream layouts into one board. Do not add a fifth Stream chip.

## Change log

- 2026-09-30 — Standings and the other text widgets send a style change or hide to OBS on the next frame, even while a newer sample is waiting.
- 2026-09-30 — OBS paints a style change when the frame arrives, instead of waiting for an animation frame.
- 2026-09-30 — Turning a stream widget on copies that preset’s in-game settings once. Later in-game edits leave the stream widget as it was.
- 2026-09-30 — Stream widgets use the in-game pixels only while the look matches, so a stream background change paints with the stream layout.
- 2026-09-29 — The stream sends the widgets the game just drew, so the map and minimap move on that frame instead of being drawn again.
- 2026-09-29 — Moving stream widgets paint from the newest sample, so a standings draw no longer steps the map or the minimap.
- 2026-09-29 — The stream minimap is sent on every sample, so the rider dot is not held behind the other widgets.
- 2026-09-29 — The stream minimap is drawn at the same size as the in-game circle, so 70% zoom shows the same track.
- 2026-09-29 — Each widget is sent as soon as it is drawn, so the map does not wait for the widgets drawn after it.
- 2026-09-29 — Each widget is sent as soon as its pixels are ready, so the map does not wait for the rest of the paint.
- 2026-09-29 — The stream paints with the game frame, so the map and other widgets stay on the same beat as the in-game HUD.
- 2026-09-29 — Each stream widget is its own message, so the next paint starts when the current widget finishes instead of after every widget.
- 2026-09-29 — OBS draws once per display frame and drops the frames that piled up, so the picture does not run late and then snap forward.
- 2026-09-29 — OBS draws the latest frame as soon as it arrives, instead of waiting for the next animation frame.
- 2026-09-29 — Stream widgets are sent as pixels and drawn directly, so OBS does not wait on a PNG decode.
- 2026-09-29 — OBS clears a widget before drawing its new frame, so the previous picture does not stay underneath.
- 2026-09-29 — OBS shows each decoded frame, then catches up, so the stream stays with `/edit` instead of falling behind.
- 2026-09-29 — OBS updates only the widgets that changed, instead of clearing the whole source every frame.
- 2026-09-29 — Set the Browser Source to the size on `/edit`, then scale it to the canvas. A game-sized source makes OBS composite every frame at that resolution.
- 2026-09-29 — The stream keeps the OBS aspect inside a 1080p pixel budget, so a 3440×1440 or 4K source stays light. `/edit` on another monitor shows that same frame.
- 2026-09-29 — Column moves and dropdowns use the same plaques as the rest of `/edit`.
- 2026-09-29 — `/edit` lists widgets again because the layout JSON parses.
- 2026-09-29 — Stream widgets can be tuned from `/edit` without changing the in-game HUD.
- 2026-09-29 — Unchanged stream widgets are not encoded or decoded again.
- 2026-09-29 — Stream widgets stay with the bike because only the widget pixels are encoded and decoded.
- 2026-09-29 — Live frames stay current while you ride. Sending runs on its own thread, so the next picture does not wait on the PNG encode.
- 2026-09-29 — A slow websocket write no longer drops OBS or `/edit`. One frame finishes before the next starts, so the picture keeps up while you ride.
- 2026-09-29 — The first stream frame is written on the listed socket under the same lock as later frames, so OBS and `/edit` keep decoding. Dragging a box posts the rect each frame, so the painted widget follows the orange box.
- 2026-09-29 — Stream frames paint off the overlay thread at 16 ms, one PNG when OBS and `/edit` share a preset, and the browser keeps only the newest frame. Minimap centers on the centerline when the bike is outside the zoom. Pit Board is on the `/edit` Cockpit list.
- 2026-09-24 — Stream layout editor at `/edit` (Show, drag/resize, basic prefs); OBS stays paint-only `/`; F8 Game|Stream surface removed.
- 2026-09-24 — Stream surface edit chrome is orange Ctrl-drag boxes only; stream Shows never paint the game HWND (Browser Source / OBS only).
- 2026-09-24 — Browser Source keeps painting while you alt-tab; it does not follow game-overlay z-order, and holds the last frame across brief SHM gaps.
- 2026-09-24 — Browser Source stamps stream-live Map/Standings/Relative show+rects onto a snapshot copy before draw (those widgets gate on SHM fields, not cfg show).
- 2026-09-24 — F8 Game/Stream chips only when Browser Source is on; hairline divider before preset chips; turning Stream off forces Game surface.
- 2026-09-24 — Phase 2: four stream HudLayouts, F8 Game/Stream surface, Show on stream, Ctrl-drag → stream edit slot, Browser Source uses `for_stream()`.
- 2026-09-24 — Phase 1: Settings → Stream Browser Source, localhost HTTP + WS PNG feed, live game layout.
- 2026-09-10 — Four stream layouts (Practice / Warmup / Race / Spectate), switched with `session_preset()`. F8 Game / Stream surface. Not a fifth chip.
- 2026-09-10 — Page created. Browser Source is the intended path. Not shipped.
