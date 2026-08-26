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
  pane: "#1C1F21"
  hairline: "#2A2A2E"
  text: "#E4E4E6"
  text-dim: "#84848A"
  ink-on-accent: "#0C0C0E"
  ink-on-light: "#101012"
  field-slate: "#303440"
  track-line: "#ECECF0"
  knob: "#FAFAFC"
  tab-on: "rgba(255, 148, 48, 0.11)"
  stage-warm: "#171616"
  stage-deep: "#0A0A0C"
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
    letterSpacing: "0.02em"
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
    letterSpacing: "0.08em"
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
  topbar: "56px"
  twin-pad: "18px 16px 28px"
  rail-gap: "4px"
  pill-x: "10px"
  pill-y: "4px"
components:
  button-primary:
    backgroundColor: "{colors.holeshot-orange}"
    textColor: "{colors.ink-on-accent}"
    rounded: "{rounded.lg}"
    padding: "10px 16px"
    typography: "{typography.headline}"
    height: "40px"
  button-primary-hover:
    backgroundColor: "{colors.holeshot-orange}"
    textColor: "{colors.ink-on-accent}"
  download-plaque:
    backgroundColor: "{colors.holeshot-orange}"
    textColor: "{colors.ink-on-accent}"
    rounded: "{rounded.sm}"
    padding: "9px 20px 9px 22px"
    typography: "{typography.body}"
  download-plaque-hover:
    backgroundColor: "{colors.holeshot-orange}"
    textColor: "{colors.ink-on-accent}"
  button-widget:
    backgroundColor: "transparent"
    textColor: "{colors.text}"
    rounded: "{rounded.lg}"
    padding: "10px 12px 10px 16px"
  button-widget-on:
    backgroundColor: "{colors.tab-on}"
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
    backgroundColor: "{colors.tab-on}"
    textColor: "{colors.holeshot-orange}"
    rounded: "{rounded.lg}"
    padding: "10px 12px"
  heading-plaque:
    backgroundColor: "{colors.holeshot-orange}"
    textColor: "{colors.ink-on-accent}"
    padding: "10px 16px"
    typography: "{typography.headline}"
  center-plaque:
    backgroundColor: "{colors.panel}"
    textColor: "{colors.text}"
    rounded: "{rounded.xl}"
    padding: "22px"
    width: "520px"
---

# Design System: Holeshot HUD

## Overview

**Creative North Star: "Broadcast Booth Glass"**

Holeshot HUD is a TV race overlay sitting on the game: dark plaques, skewed hardware bars, and a hot orange that always means *you*. It is broadcast-hot — stronger bars, louder flags, denser tables — not a quiet utility panel and not a consumer app shell. The rider glances; the graphic has to punch.

Type is Exo 2 ExtraBold Italic at table scale. Corners stay modest. Chrome is opt-in: a fresh install draws nothing. Settings is the same family in opaque charcoal, including a Center Plaque that can stack on the dimmed host. The website is not a second identity and not a marketing landing: it is an F8 Twin pit-box — settings pane, F8 widget rail, live HUD stage, Download as one orange clip-path plaque in a thin top bar.

Do not drift toward generic sim-HUD blues, glassmorphism blobs, or light “esports dashboard” kits. Manufacturer bike colors stay on pills only.

**Key Characteristics:**
- Dark plaques over the game; orange is you / CTA only
- Skewed TV bars and bike-brand pills as hardware, not cards
- Exo 2 ExtraBold Italic as the HUD, settings, and web face (Teko / Goldman optional in the overlay)
- Tonal charcoal stacking for depth — no card shadows
- Race meaning uses blue / red / green / violet, never a second orange
- Web preview is F8 Twin: settings pane, 204px widget rail, then the live stage

## Colors

One brand accent. Everything else is either charcoal, race meaning, or a manufacturer pill.

### Primary
- **Holeshot Orange**: You on the map, wordmark “HUD”, Download plaque, toggles on, selected rail type, focus rings. Settings native chrome paints a nearby 255,140,36; use this token, not a second orange.

### Neutral
- **Night Ink**: HUD panel fill (alpha ~200) and the web page ground. Map/minimap default background opacity is 0 — the game shows through.
- **Charcoal / Charcoal Side / Panel / Pane**: Opaque settings and web chrome stacking. Charcoal Side is the top bar and settings rail. Pane is the web settings twin. Panel is inner boards and the Center Plaque.
- **Hairline**: Borders, toggle-off tracks, dividers (white at ~12 alpha in settings).
- **Text / Text Dim**: Cell ink and column headers. Tables may invert to black ink; pills never follow that invert.
- **Ink on Accent / Ink on Light**: Dark type on orange plaques and light manufacturer pills.
- **Knob**: White toggle and slider thumbs.
- **Tab On**: Selected rail/tab wash (~11% orange).
- **Stage Warm / Stage Deep**: Full-bleed web stage gradient (warm vignette over deep ink). Not a card fill.
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
**Body Font:** Same face at 12px for table cells and settings labels
**Label/Mono Font:** Same face at 10px, often uppercase. Overlay icons: Font Awesome Free Solid.

**Character:** Condensed sport italic — broadcast lower-third energy, not UI sans. Web chrome (top bar, twin, Download) uses the same ExtraBold Italic cut as settings. There is no roman shell face.

### Hierarchy
- **Display** (ExtraBold Italic, 28px, tight, slight tracking): Dash gear, large P#, and the twin’s widget name (uppercase). The loudest type on a surface.
- **Headline** (ExtraBold Italic, ~18px): Event title on the orange header bar; Settings pane titles (~22px) and What’s new version plaques (18px) on content-sized orange skews. Web wordmark is 26px ExtraBold Italic uppercase; “HUD” is Holeshot Orange with wider tracking (0.12em).
- **Title** (ExtraBold Italic, 10px, 0.08em tracking): Twin section labels (SETTINGS), track-name plaques, board titles, and uppercase Center Plaque section labels.
- **Body** (ExtraBold Italic, 12px): Standings and relative cells; settings field labels; Download plaque type. Default scale 100%; rider can set 70–160% per widget.
- **Label** (700–800, 10px, uppercase, 0.06em): Column headers (P, #, NAME, GAP). Dim ink. Stage caption is 10px dim, not uppercase.

### Named Rules
**The One-Face Rule.** HUD, settings, and the web demo speak Exo 2 ExtraBold Italic (or the rider’s Teko / Goldman in the overlay). Segoe is fallback only.

**The Italic Default Rule.** The bundled Exo 2 cut is italic extra-bold. Do not “correct” it to a roman UI font on the website or in settings.

## Layout

Widgets are free-floating rectangles on the game, placed by the rider (Ctrl-drag, snap-to-monitor). There is no page grid in the overlay. Density is race-table tight: 12px cells, 10px headers, bike bar 5px skewed 3px after Position.

Settings is a left-rail tool inside F8: sidebar tabs (~48px rows, 8px gaps), opaque charcoal, 8–10px corners. A Center Plaque (What’s new) dims that host and centers a Panel board (~520px, 10px corners).

The website is an **F8 Twin pit-box**, not a boxed canvas. Thin top bar (56px, Charcoal Side): 40px mark (8px corners) + wordmark left, Download plaque right. Below: CSS grid `clamp(260px, 24vw, 340px) 204px 1fr` — settings pane, widget nav (F8 rail: Boards / Track / Cockpit), then live HUD stage. Stage is full-bleed (no inner card). Twin is Pane with a hairline right edge. Widget nav is Charcoal Side, 204px like F8 `SIDE_W`. Under 960px wide or 620px tall the pit stacks HUD, then a sticky chip rail, then settings (wordmark hides under 540px; Download shortens to “Download”). Short landscape (max 520px tall, min 700px wide) splits HUD left and rail + settings right. Canvas is 1280×720 contained. Safe-area insets pad the top bar and stacked chrome.

Map default fill is transparent. Radar keeps a solid square panel (opacity default 86). Fresh install: every widget off.

**The Opt-In Chrome Rule.** Nothing draws until **Show on overlay** is on. Empty race data shows “Waiting for race data”, not a blank plaque.

**The Twin Monitor Rule.** The web preview is a pit box: settings pane, F8 widget rail, and live HUD stage, left to right. Download lives in the top bar on every width.

## Elevation & Depth

Tonal charcoal stacking. Depth is fill vs fill (Night Ink over the game, Pane over Charcoal Side, hairline dividers), not drop shadows. HUD panels are translucent so the track stays visible; that translucency is a game-overlay constraint, not a glassmorphism style.

The web stage conveys depth with a warm radial vignette (ellipse at 50% 28%, #2A2218 fading out) over a Stage Warm → Stage Deep linear wash. The HUD widget floats on that wash; the stage itself is not a card.

Settings may use a faint dark disc under knobs and menus (black ~50–90 alpha). Web sliders use a small knob disc (`0 2px 6px rgba(0,0,0,0.55)`). Do not promote that into HUD tables, the twin, or stage cards. A Center Plaque sits on a Charcoal Side scrim (~80% / 204 alpha) — still fill-over-fill, not a drop shadow.

### Named Rules
**The Tonal Stack Rule.** Surfaces are flat at rest. No ambient card shadows. A knob may sit on a small dark disc; a standings board may not.

## Shapes

Modest rounds: 4px pills and Download under-clip, 6px HUD boards / steppers / selects, 7px dash, 8px rail tabs and the web mark, 8–10px settings tabs and Center Plaque. Signature geometry is the **skew plaque** — a parallelogram used for rider-count, track name, the 5px bike bar after Position, Settings heading plaques, and the web Download bar.

HUD bars skew ~4px. Settings heading plaques skew 6–8px and size to the label (measure + 36px, min 72px), left-aligned. Web Download uses a clip-path parallelogram (`polygon(10px 0, 100% 0, calc(100% - 10px) 100%, 0 100%)`; 6px cut when stacked), 12px Windows four-square mark, uppercase ExtraBold Italic. Bike pills are short stadium-rectangles, padded 10×4, vertically centered in the row.

Map is a thin Track Line polyline, not a filled region. Radar is a square panel, white bike silhouette, circular blips (closer = larger, more orange). Flags are full-width banners (white flag: diagonal stripes into a white plaque; checkered: wrap the dash).

**The TV Plaque Rule.** Skewed bars and manufacturer pills are broadcast hardware. Do not replace them with cards, chips-in-a-row, or Material buttons.

**The Content-Sized Plaque Rule.** Heading plaques size to the label. Do not stretch orange across the board. Download is the exception: it is a CTA plaque, not a heading.

## Components

### Buttons
TV-plaque energy even in chrome: filled orange for the one action, rail rows for the rest.

- **Shape:** 8px on settings fills; Download is a clip-path parallelogram (4px radius under the clip).
- **Primary fill:** Holeshot Orange, ink-on-accent, 10×16, 40px tall. What’s new **Got it** is this fill, full inner width of the Center Plaque.
- **Download plaque:** Holeshot Orange clip-path bar, 9×20×9×22 pad, 12px uppercase, Windows mark at 12px. Hover brightens 1.08. The only web CTA.
- **Hover / Focus:** Keep the fill. Focus is a 2px Holeshot Orange ring, 2px offset. Selected rail rows pick up Tab On wash plus a 3px orange pip — not an orange border.
- **Secondary:** Panel fill, hairline border, 6px. Stepper squares are 28×28.

### Chips
- **Bike pill:** Manufacturer color from bike name (Yamaha blue, Honda red, KTM orange, etc.). Ink flips at luminance 0.62. Table text color does not recode the pill.
- **Skew plaque:** Holeshot Orange parallelogram, dark icon + count or track name (HUD scale).
- **Heading plaque:** Content-sized orange parallelogram for Settings pane titles and the What’s new version. Ink-on-accent ExtraBold Italic, 16px left inset. The plaque is the heading — not an eyebrow above a title.

### Cards / Containers
- **HUD panel:** Night Ink ~200 alpha, 6–7px corners. Header strip slightly darker. Standings header can be a full-width orange event bar.
- **Web stage:** Full-bleed Stage Warm → Stage Deep with a warm radial vignette. No corners, no card shadow, no inner frame. One live widget.
- **Web twin:** Pane fill, hairline right (top on stacked). Not a card.
- **Settings:** Charcoal body, darker rail, 10px inner panels. Center Plaque board is Panel, 10px corners, 22px pad.

### Inputs / Fields
- **Toggle:** 36×20 pill; off = hairline, on = Holeshot Orange; Knob 14px, 0.12s ease.
- **Slider:** Hairline track, orange fill, Knob with a faint dark disc.
- **Select:** Panel fill, hairline, 6px, 7×8 padding.
- **Focus:** Orange, not a blue ring.

### Navigation
Settings left rail and the web twin rail share one grammar: 8px rounded row, 4px gap, selected = Tab On wash plus a 3px orange pip inset 6px. Hover is white at 4% alpha. Web top bar is brand left, Download right — not a site nav.

### Standings / Relative (signature)
Tight classification boards. Alternating near-black rows. Your row = You Row tint. Session-best time = Best Lap Violet. Position is followed by a manufacturer skew bar. Headers uppercase dim. Rows slide when live order changes.

### Dash (signature)
Horizontal plaque: gear box, RPM/speed stack, large italic orange P#, flag wrap on checkered/white. Shift lights are a green → yellow → red capsule row.

### Map / Minimap / Radar (signature)
You = larger Holeshot Orange dot. Others = Field Slate unless lapping/closing. Leader crown and ahead/behind rings are overlays, not recodes of the whole field. Radar: no blind-spot wedges — panel, bike, blips only.

### Center Plaque (signature)
Blocking note on Settings (What’s new after an in-app update). Charcoal Side scrim; centered Panel board. Content-sized heading plaque names the version. Headline in text ink, uppercase dim section labels, orange dots + 12px body. One primary fill: **Got it**. Re-open from App → Updates via secondary **What’s new**.

### Download Plaque (signature)
Web-only CTA in the top bar. Clip-path orange parallelogram, Windows four-square mark, “Download for Windows” (shortens to “Download” when the pit stacks). Ink-on-accent ExtraBold Italic. Not a rounded Material button.

## Do's and Don'ts

### Do:
- **Do** keep Holeshot Orange for you, CTA, and accent only.
- **Do** use Exo 2 ExtraBold Italic as the face for HUD, settings, and web chrome.
- **Do** ship bike pills in manufacturer colors with contrast-flipped ink.
- **Do** start every widget hidden on a fresh install.
- **Do** treat map fill as transparent unless the rider raises opacity.
- **Do** size Settings heading plaques to the label (text + 36px), left-aligned.
- **Do** treat the website as an F8 Twin pit-box: settings, widget rail (204px), live stage, Download in the top bar.

### Don't:
- **Don't** paint other riders orange.
- **Don't** invent a second lapping palette for minimap or radar.
- **Don't** bring back radar side/rear zone wedges.
- **Don't** add drop shadows under HUD tables, the twin, or the stage.
- **Don't** let a roman / Segoe shell face into overlay widgets or web chrome.
- **Don't** recode manufacturer pills to follow White/Black table text.
- **Don't** draw overlay work into the in-game C++ HUD (`ingame_hud`).
- **Don't** stretch heading plaques into full-width orange bars.
- **Don't** wrap the live HUD in a boxed canvas inside a left-rail dashboard.
