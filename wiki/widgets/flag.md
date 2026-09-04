# Flags

A standalone flag as a skew plaque. White and checkered use the same timing as Dash — when Dash would wrap, this widget shows the flag. Optional **Yellow flag** and **Blue flag** (each off by default) are inferred: yellow when a rider has crashed **ahead of you and close** (~50 m); blue when someone a lap up is **close behind** (~40 m). When no flag is up, it draws **nothing**. Dash wrap stays white/checkered only; this is a second graphic you can place anywhere.

Settings subtitle: “White and checkered — same timing as Dash”. Toggles: **Text** (`flag_text=1`, default on), **Yellow flag** (`flag_yellow=0`), **Blue flag** (`flag_blue=0`). Old `flag_caution=1` turns both on. Off **Text** is cloth only — no caption white band.

## Code

- Draw: `draw_flag` in `overlay/hud/src/render.rs`
- Timing: `dash_race_flag` + `flag_anim_step`, ticked once per frame in `draw()` (`tick_display_flag`) when Dash or Flags is on, so they cannot double-step the anim
- Caution: `caution_flag` — yellow is a crash **ahead** within `FLAG_YELLOW_SPAN_M` (50 m); blue is `LapRel::LappingMe` **and** behind within `FLAG_BLUE_SPAN_M` (40 m). Merged in `wanted_flag` only when Flags is on **and** the matching toggle. Priority: checkered > white > yellow > blue. Preview codes: 0 none, 1 white, 2 checkered, 3 yellow, 4 blue
- Preview: `set_flag_preview` (website demo cycles checkered / white / hidden; with each toggle on, that color too)
- Settings: `pane_flag` in `overlay/src/settings.rs`

Fresh install: `show_flag = false`, `flag_yellow = false`, `flag_blue = false`, `flag_text = true`. Default ~10.7%×1.9% (the size we settled on in-game), top-center. Hold Ctrl and drag to place. Panel opacity tints the cloth (default 100). Caption white uses the same 5% edge fade as the Dash wrap banners, with extra side pad so it is not tight on the glyphs. The white band is shorter than the plaque so cloth shows above and below. An old ini without `flag_text` keeps the caption on.

## Do not regress

- Do not draw a plaque when the flag is down. Empty slot, except the Ctrl layout box.
- Do not invent a second flag machine. White wave, checkered latch, run-in hold, and `finish_earned` live in `dash_race_flag`.
- Do not tick `flag_anim_step` from both Dash and Flags. One step per frame.
- Do not hide Dash wrap when Flags is on. They are independent.
- Do not paint yellow or blue on the Dash wrap. `dash_wrap_flag` strips them before `draw_dash`.
- Do not treat `lapped()` (whole-race gap) as a blue flag. Blue is situational `LapRel::LappingMe` only.
- Do not paint Holeshot orange on the cloth. Checkers and white stripes match the Dash banners. Yellow is `#F4D624`, blue is `#3B82F6`.
- Do not wave or bob the plaque. Grow is opacity only.
- Do not paint the caption white band when **Text** is off. Cloth only.

## Change log

- 2026-09-02 — **Text** toggle (`flag_text`, default on). Off hides the caption and its white band.
- 2026-08-30 — Caption white is a shorter band so cloth shows above and below the text.
- 2026-08-30 — Caption white uses the Dash banner fade (5% at the cloth edges) and extra side pad so it is not tight on the text.
- 2026-08-29 — Default ~10.7%×1.9% (saved in-game size). Caption white fades at the ends. Yellow only for a crash ahead and close (~50 m). Blue only when the lapper is within ~40 m behind.
- 2026-08-29 — **Yellow flag** and **Blue flag** are separate toggles. `flag_caution=1` still enables both.
- 2026-08-28 — Optional yellow (nearby crash) and blue (being lapped), behind **Yellow and blue**. Dash wrap stays white/checkered.
- 2026-08-28 — Skew plaque (Dash checkers / white stripes + caption). No waving cloth. Default is a wide top-center strip.
- 2026-08-28 — Added. Hidden when `DashFlag::None`.
