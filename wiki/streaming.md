# Streaming

**Not shipped.** Agent context for putting Holeshot widgets on a stream without drawing them on the game. When this lands, keep this page and append a **Change log** bullet (why, not just what).

Widget wishlist: [widgets/future.md](widgets/future.md). Shipped widgets: [widgets.md](widgets.md).

## Today

The HUD is one layered window on MX Bikes. OBS **Game Capture** of the game never sees it. Discord **Share this window** (MX Bikes) is the same. Streamers must **Display Capture** (desktop junk) or add a **Window Capture** of **Holeshot HUD** and stack it over the game. That capture is the *in-game* layout — there is no stream-only set.

`web/` is the same HUD renderer in the browser with **demo data**, not live telemetry. It is not an OBS source.

## Why Browser Source

| Approach | Stream-only? | Streamer work |
| --- | --- | --- |
| Local HTTP + OBS **Browser Source** | Yes | Paste `http://127.0.0.1:<port>`, set 1920×1080 |
| Companion / second HWND | Yes, if that window is what you capture | Second monitor + Window Capture. Layered HWNDs are flaky in OBS — we already miss Game Capture |
| Capture the overlay window | No | Display Capture or Window Capture of Holeshot HUD |

**Ship Browser Source first.** Streamers already know it (alerts, chat). Transparent page above Game Capture. No second monitor. Do not start with a companion HWND.

Edit the stream layout in F8. OBS only paints the URL. Do not ship a second settings surface inside the browser page.

## Intended streamer steps

1. F8 → Stream → **On** (copy `http://127.0.0.1:<port>`).
2. OBS → Browser Source → paste → width/height = canvas (1920×1080 default) → shutdown source when not visible.
3. Pick Practice / Warmup / Race / Spectate and turn on widgets for **that** stream layout. Those never draw on the game.

Port conflict → pick another and show it. Localhost only.

## Layout: add, remove, move

OBS places **one** Browser Source (the whole 16:9 canvas). You cannot add Standings as its own OBS source or drag it in the OBS preview. Moving/cropping the source moves every widget. One URL; the page follows the live session.

**Four stream layouts**, not a fifth chip. Practice / Warmup / Race / Spectate each keep an in-game `HudLayout` **and** a stream `HudLayout`. The browser uses the same `session_preset()` as the game HUD ([widgets.md](widgets.md)): warmup on track → Warmup stream, race → Race stream, spectate/replay → Spectate stream. Garage/menus hold the last stream slot, same as the game.

F8 still has those four chips. A **Game / Stream** surface (next to the chips) chooses which one you are editing. Game is **Show on overlay** + Ctrl-drag as today. Stream is **Show on stream** + Ctrl-drag that writes the stream slot for the open chip.

| Action | Where | What happens |
| --- | --- | --- |
| Add / remove | Chip (e.g. Race) + Stream surface → **Show on stream** | That preset’s browser page shows or hides the widget. Game HUD unchanged. |
| Move / resize | Stream surface + chip selected → **Ctrl-drag on the game** | Orange boxes are that preset’s stream layout. Live in-game widgets hide while Stream is the edit surface. Writes the **stream** slot for that chip. |
| Copy | **Copy to** | Can copy a stream slot onto another stream slot (Race stream → all stream slots), or copy a game layout onto that chip’s stream. Does not overwrite the other surface unless they pick that. |
| OBS | Place / scale the one Browser Source | Nudge the whole HUD. Not per-widget. Session still swaps which of the four stream layouts draw. |

Today Ctrl-drag writes the **live** preset, not the F8 edit slot ([widgets.md](widgets.md)). Stream edit must write the **stream** slot for the open chip, even when the live session is a different preset (same as editing Race in F8 while you warmup — the game HUD stays on Warmup; stream chrome is the Race stream board).

Coords stay normalized 0–1 against a 16:9 stream canvas. Snap on a stream slot is snap-to-canvas. Close F8 or switch back to Game → the game returns to the live in-game preset; the browser keeps following `session_preset()`.

Garage with Stream + Spectate (or any chip) open still needs chrome to drag — same as editing Spectate in the garage today.

Typical split: Race stream = standings + nameplate; Practice / Warmup stream = delta + sectors; Spectate stream = nameplate + standings. All optional. Fresh install: every stream **Show** starts off, same as the game.

Do **not** drag inside OBS Interact or save layout in `localStorage`. That splits state from `Holeshot-HUD.ini`. A later “open in browser to preview” still reads the four stream slots from the overlay.

## Build notes (when we start)

- Tiny HTTP server in the overlay process, localhost only.
- Page reuses the WASM HUD (`web/` / `web-preview`) fed live over WebSocket or SSE from the snapshot — not demo data.
- Copy-URL control in F8. Show the port. If bind fails, try the next port and say so.
- Each of the four presets stores stream show + rects separately from the in-game slot. Stream **Show** never paints the game window.
- The browser page switches with `session_preset()`. Do not make a fifth Stream chip win that picker.

A second-monitor companion window can come later for a pit tablet without OBS. Not v1.

## Do not

- Do not treat Window Capture of **Holeshot HUD** as stream-only.
- Do not draw a stream layout on the game except as F8 edit chrome while the Stream surface is selected.
- Do not give OBS a second settings surface (gear + localStorage) for positions.
- Do not serve past localhost.
- Do not collapse the four stream layouts into one board. Do not add a fifth Stream chip that ignores Practice / Warmup / Race / Spectate.

## Change log

- 2026-09-10 — Four stream layouts (Practice / Warmup / Race / Spectate), switched with `session_preset()`. F8 Game / Stream surface. Not a fifth chip.
- 2026-09-10 — Page created. Browser Source is the intended path. Not shipped.
