---
name: Holeshot HUD
description: Broadcast-style race overlay for MX Bikes — TV plaques on dark glass.
colors:
  holeshot-orange: "#FF9430"
  you-row: "#C48424"
  night-ink: "#0A0A0A"
  charcoal: "#18191D"
  charcoal-side: "#08080A"
  panel: "#141416"
  hairline: "#2A2A2E"
  text: "#E4E4E6"
  text-dim: "#84848A"
  ink-on-accent: "#0C0C0E"
  ink-on-light: "#101012"
  field-slate: "#303440"
  track-line: "#ECECF0"
  lapping-blue: "#3B82F6"
  lapped-red: "#EF4444"
  ahead-green: "#30DC58"
  behind-red: "#FF4048"
  best-lap-violet: "#C470FF"
  dash-place: "#E87817"
  flag-yellow: "#F4D624"
typography:
  display:
    fontFamily: "Exo 2, Segoe UI, sans-serif"
    fontSize: "28px"
    fontWeight: 800
    lineHeight: 1
    letterSpacing: "normal"
  headline:
    fontFamily: "Exo 2, Segoe UI, sans-serif"
    fontSize: "18px"
    fontWeight: 800
    lineHeight: 1.1
    letterSpacing: "0.02em"
  title:
    fontFamily: "Exo 2, Segoe UI, sans-serif"
    fontSize: "10px"
    fontWeight: 800
    lineHeight: 1.2
    letterSpacing: "0.04em"
  body:
    fontFamily: "Exo 2, Segoe UI, sans-serif"
    fontSize: "12px"
    fontWeight: 800
    lineHeight: 1.15
    letterSpacing: "normal"
  label:
    fontFamily: "Exo 2, Segoe UI, sans-serif"
    fontSize: "10px"
    fontWeight: 700
    lineHeight: 1.2
    letterSpacing: "0.06em"
rounded:
  sm: "4px"
  md: "6px"
  lg: "8px"
  xl: "10px"
  pill: "10px"
spacing:
  row: "8px"
  settings-row: "48px"
  web-gap: "20px"
  web-pad: "32px"
  pill-x: "10px"
  pill-y: "4px"
components:
  button-primary:
    backgroundColor: "{colors.holeshot-orange}"
    textColor: "{colors.ink-on-accent}"
    rounded: "{rounded.md}"
    padding: "10px 16px"
    typography: "{typography.headline}"
  button-primary-hover:
    backgroundColor: "{colors.holeshot-orange}"
    textColor: "{colors.ink-on-accent}"
  button-widget:
    backgroundColor: "{colors.panel}"
    textColor: "{colors.text}"
    rounded: "{rounded.md}"
    padding: "10px 12px"
  button-widget-on:
    backgroundColor: "{colors.panel}"
    textColor: "{colors.holeshot-orange}"
  toggle-on:
    backgroundColor: "{colors.holeshot-orange}"
    rounded: "{rounded.pill}"
    width: "36px"
    height: "20px"
  toggle-off:
    backgroundColor: "{colors.hairline}"
    rounded: "{rounded.pill}"
    width: "36px"
    height: "20px"
  bike-pill:
    backgroundColor: "{colors.holeshot-orange}"
    textColor: "{colors.ink-on-accent}"
    rounded: "{rounded.sm}"
    padding: "4px 10px"
    typography: "{typography.label}"
  table-row-you:
    backgroundColor: "{colors.you-row}"
    textColor: "{colors.text}"
  skew-plaque:
    backgroundColor: "{colors.holeshot-orange}"
    textColor: "{colors.ink-on-accent}"
    padding: "3px 8px"
    typography: "{typography.label}"
  hud-panel:
    backgroundColor: "{colors.night-ink}"
    textColor: "{colors.text}"
    rounded: "{rounded.md}"
  settings-tab-on:
    backgroundColor: "{colors.holeshot-orange}"
    textColor: "{colors.holeshot-orange}"
    rounded: "{rounded.lg}"
    padding: "10px 12px"
---

# Design System: Holeshot HUD

## Overview

**Creative North Star: "Broadcast Booth Glass"**

Holeshot HUD is a TV race overlay sitting on the game: dark plaques, skewed hardware bars, and a hot orange that always means *you*. It is broadcast-hot — stronger bars, louder flags, denser tables — not a quiet utility panel and not a consumer app shell. The rider glances; the graphic has to punch.

Type is Exo 2 ExtraBold Italic at table scale. Corners stay modest. Chrome is opt-in: a fresh install draws nothing. The website is the same HUD on a charcoal stage, not a second identity. Settings is the same family in opaque charcoal.

Do not drift toward generic sim-HUD blues, glassmorphism blobs, or light “esports dashboard” kits. Manufacturer bike colors stay on pills only.

**Key Characteristics:**
- Dark plaques over the game; orange is you / CTA only
- Skewed TV bars and bike-brand pills as hardware, not cards
- Exo 2 ExtraBold Italic as the HUD face (Teko / Goldman optional)
- Tonal charcoal stacking for depth — almost no drop shadows
- Race meaning uses blue / red / green / violet, never a second orange

## Colors

One brand accent. Everything else is either charcoal, race meaning, or a manufacturer pill.

### Primary
- **Holeshot Orange**: You on the map, your row tint’s sibling, wordmark “HUD”, download, toggles on, focus rings. Settings paints a nearby 255,140,36 for native chrome; use this token, not a second orange.

### Neutral
- **Night Ink**: HUD panel fill (alpha ~200). Map/minimap default background opacity is 0 — the game shows through.
- **Charcoal / Charcoal Side / Panel**: Opaque settings and web chrome stacking.
- **Hairline**: Borders, toggle-off tracks, dividers (white at ~12 alpha in settings).
- **Text / Text Dim**: Cell ink and column headers. Tables may invert to black ink; pills never follow that invert.
- **Ink on Accent / Ink on Light**: Dark type on orange plaques and light manufacturer pills.
- **Field Slate**: Every other rider until lapping rules fire.
- **Track Line**: Centerline and dash outlines.
- **You Row**: Gold-brown highlight behind the local rider in standings/relative (opacity is a user slider, default 50%).

### Race meaning (not a second brand)
- **Lapping Blue / Lapped Red**: Other rows and dots only when a lap apart *and* closing. Off in warmup.
- **Ahead Green / Behind Red**: Nearest-ahead / nearest-behind rings on map — not general rider color.
- **Best Lap Violet**: Session-best lap time in tables.
- **Dash Place**: Large italic P# on the dash.
- **Flag Yellow**: Caution / yellow-flag segments on the shift/flag strip.

**The You-Are-Orange Rule.** Holeshot Orange is you, CTA, and accent. Other riders are Field Slate unless lapping/closing paints them blue or red.

**The Meaning-Not-Brand Rule.** Blue, red, green, and violet carry race state. They are never decorative theme colors.

## Typography

**Display Font:** Exo 2 ExtraBold Italic (bundled `Exo2-ExtraBoldItalic` / `Exo2-BlackItalic`; fallback Segoe UI, Roboto)
**Body Font:** Same face at 12px for table cells
**Label/Mono Font:** Same face at 10px, often uppercase. Icons: Font Awesome Free Solid.

**Character:** Condensed sport italic — broadcast lower-third energy, not UI sans. Web chrome around the canvas uses Segoe UI; do not let that leak into HUD widgets.

### Hierarchy
- **Display** (ExtraBold Italic, ~28px, tight): Dash gear and large P#. The loudest number on screen.
- **Headline** (ExtraBold Italic, ~18px): Event title on the orange header bar; web wordmark “HOLESHOT” is 26px/800 uppercase Segoe on the demo only.
- **Title** (ExtraBold Italic, 10px, slight tracking): Track-name plaques and board titles.
- **Body** (ExtraBold Italic, 12px): Standings and relative cells. Default scale 100%; rider can set 70–160% per widget.
- **Label** (700–800, 10px, uppercase, 0.06em): Column headers (P, #, NAME, GAP). Dim ink.

### Named Rules
**The One-Face Rule.** HUD and settings speak Exo 2 (or the rider’s Teko / Goldman). Segoe is web-shell only.

**The Italic Default Rule.** The bundled Exo 2 cut is italic extra-bold. Do not “correct” it to a roman UI font.

## Layout

Widgets are free-floating rectangles on the game, placed by the rider (Ctrl-drag, snap-to-monitor). There is no page grid in the overlay. Density is race-table tight: 12px cells, 10px headers, bike bar 5px skewed 3px after Position.

Settings is a left-rail tool: sidebar tabs (~48px rows, 8px gaps), opaque charcoal, 8–10px corners. Web demo is a 1320px max shell — 280px widget rail + stage — collapsing to one column under 800px. Stage is 16:9.

Map default fill is transparent. Radar keeps a solid square panel (opacity default 86). Fresh install: every widget off.

**The Opt-In Chrome Rule.** Nothing draws until **Show on overlay** is on. Empty race data shows “Waiting for race data”, not a blank plaque.

## Elevation & Depth

Tonal charcoal stacking. Depth is fill vs fill (Night Ink over the game, Charcoal over Charcoal Side, hairline dividers), not drop shadows. HUD panels are translucent so the track stays visible; that translucency is a game-overlay constraint, not a glassmorphism style.

Settings may use a faint dark disc under knobs and menus (black ~50–90 alpha). Do not promote that into HUD tables or web cards.

### Named Rules
**The Tonal Stack Rule.** Surfaces are flat at rest. No ambient card shadows. A knob may sit on a small dark disc; a standings board may not.

## Shapes

Modest rounds: 4px pills, 6px HUD boards and web buttons, 7px dash, 8–10px settings tabs. Signature geometry is the **skew plaque** — a parallelogram (skew ~4px) used for rider-count, track name, and the 5px bike bar after Position. Bike pills are short stadium-rectangles, padded 10×4, vertically centered in the row.

Map is a thin Track Line polyline, not a filled region. Radar is a square panel, white bike silhouette, circular blips (closer = larger, more orange). Flags are full-width banners (white flag: diagonal stripes into a white plaque; checkered: wrap the dash).

**The TV Plaque Rule.** Skewed bars and manufacturer pills are broadcast hardware. Do not replace them with cards, chips-in-a-row, or Material buttons.

## Components

### Buttons
TV-plaque energy even in chrome: filled orange for the one action, outlined charcoal for the rest.

- **Shape:** 6px on web; 8px on settings fills.
- **Primary:** Holeshot Orange fill, ink-on-accent, 10×16, weight 700. Download is the web primary.
- **Hover / Focus:** Keep the fill; widget buttons pick up an orange border and orange label when selected (`.on`).
- **Secondary:** Panel fill, hairline border, 6px. Stepper squares are 28×28.

### Chips
- **Bike pill:** Manufacturer color from bike name (Yamaha blue, Honda red, KTM orange, etc.). Ink flips at luminance 0.62. Table text color does not recode the pill.
- **Skew plaque:** Holeshot Orange parallelogram, dark icon + count or track name.

### Cards / Containers
- **HUD panel:** Night Ink ~200 alpha, 6–7px corners. Header strip slightly darker. Standings header can be a full-width orange event bar.
- **Web stage:** Warm radial vignette on #161412 → #0A0A0C, 10px corners. No card shadow.
- **Settings:** Charcoal body, darker rail, 10px inner panels.

### Inputs / Fields
- **Toggle:** 36×20 pill; off = hairline, on = Holeshot Orange; white 14px knob, 0.12s ease.
- **Slider:** Hairline track, orange fill, white knob with a faint dark disc.
- **Select:** Panel fill, hairline, 6px, 7×8 padding.
- **Focus:** Orange, not a blue ring.

### Navigation
Settings left rail: 8px rounded row, selected row gets a wash of orange at ~11% alpha plus a 3px orange pip. Web widget list is the same idea as stacked secondary buttons; selected = orange border + orange type.

### Standings / Relative (signature)
Tight classification boards. Alternating near-black rows. Your row = You Row tint. Session-best time = Best Lap Violet. Position is followed by a manufacturer skew bar. Headers uppercase dim. Rows slide when live order changes.

### Dash (signature)
Horizontal plaque: gear box, RPM/speed stack, large italic orange P#, flag wrap on checkered/white. Shift lights are a green → yellow → red capsule row.

### Map / Minimap / Radar (signature)
You = larger Holeshot Orange dot. Others = Field Slate unless lapping/closing. Leader crown and ahead/behind rings are overlays, not recodes of the whole field. Radar: no blind-spot wedges — panel, bike, blips only.

## Do's and Don'ts

### Do:
- **Do** keep Holeshot Orange for you, CTA, and accent only.
- **Do** use Exo 2 ExtraBold Italic as the default HUD face.
- **Do** ship bike pills in manufacturer colors with contrast-flipped ink.
- **Do** start every widget hidden on a fresh install.
- **Do** treat map fill as transparent unless the rider raises opacity.

### Don't:
- **Don't** paint other riders orange.
- **Don't** invent a second lapping palette for minimap or radar.
- **Don't** bring back radar side/rear zone wedges.
- **Don't** add drop shadows under HUD tables.
- **Don't** let Segoe / web-shell type into overlay widgets.
- **Don't** recode manufacturer pills to follow White/Black table text.
- **Don't** draw overlay work into the in-game C++ HUD (`ingame_hud`).
