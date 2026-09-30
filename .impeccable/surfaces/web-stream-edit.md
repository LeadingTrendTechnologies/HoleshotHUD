# Stream layout editor (`/edit`)

## Scope
Operate · Localhost page served by `overlay/src/stream/` at `/edit`. Separate from OBS paint-only `/`.

## Audience / job
Streamer opens Chrome on the edit URL while riding or staging. Pick stream preset, enable widgets, tune the selected widget’s settings, drag orange boxes on the live canvas. OBS keeps `/` with no chrome.

## Direction
Inherit Holeshot DESIGN.md + web F8 Twin chrome (no top bar on this page). Desktop LTR: **main left nav** (Stream layout title, Practice/Warmup/Race/Spectate chips, Show on stream widget list with enable toggles + selection pip) → **settings subnav** (selected widget title + settings rows; content swaps on nav click) → **stage** live PNG with orange drag/resize boxes.

## Memorable moment
Click a widget in the left nav → its settings fill the subnav; the stage shows every enabled widget with orange drag/resize boxes where OBS will see them.

## Approved comp
`.impeccable/mocks/stream-edit.png` — approved 2026-09-24.

## Panel contents
- Stream layout + preset chips (left nav top)
- Widget enable + select (left nav)
- Per-widget settings (subnav)
- Live canvas drag/resize (stage)
- OBS `/` vs Edit `/edit` URLs (footer)

## Unresolved
None.
