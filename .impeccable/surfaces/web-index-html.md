---
version: 1
slug: "web-index-html"
primary_target: "web/index.html"
related_targets: ["web/style.css","web/demo.js"]
---

# Web preview (F8 Twin)

## Scope
Experience · `web/index.html` (chrome in `web/style.css`, `web/demo.js`). Live widget editor plus Windows download. HUD canvas renderer unchanged.

## Audience / job
MX Bikes rider trying widgets in the browser, then downloading. First seconds: live HUD, widget rail, settings (desktop LTR: settings, rail, HUD), Download one click in the top bar.

## Direction
F8 Twin pit-box. Left to right: settings pane, F8 widget rail (Boards / Track / Cockpit), live HUD stage. Thin top bar: mark, wordmark, Download plaque. One widget at a time.

## Memorable moment
Switching a rail row swaps the live overlay and the field list instantly, like hitting a widget tab in F8.

## Approved comp
`.impeccable/mocks/decision/f8-twin.png`

## Unresolved
None.

## Inventory
| Region | Medium | Notes |
| Top bar | HTML/CSS | sampled bar ~#010101; use night-ink/charcoal-side |
| Logo | existing `web/logo.png` | app mark; do not regenerate |
| Wordmark | CSS, Exo 2 ExtraBold Italic | HOLESHOT + HUD |
| Download CTA | HTML/CSS skew plaque | #FF9430, ink #0C0C0E |
| Stage | CSS radial vignette | sampled ~#171616; #161412 → #0A0A0C |
| HUD widgets | existing WASM canvas | do not literalize multi-widget or fake P3 from the comp |
| Settings twin | HTML/CSS | sampled pane ~#1C1F21; charcoal #18191D |
| Widget rail | buttons, 8px round, 3px orange pip | settings nav_tab grammar |
| Settings fields | existing demo.js controls | same keys as the app |
| Demo caption | HTML on stage | “Demo data — not connected to MX Bikes” |
| Type | self-hosted Exo 2 ExtraBold Italic | `web/fonts/Exo2-ExtraBoldItalic.ttf` |
