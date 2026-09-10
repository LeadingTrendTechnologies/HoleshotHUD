# Horizontal Standings

Top bar of rider cards, leader on the left, your name highlighted. Settings tab label is **H-Standings**. Code name is **Ticker** (`WidgetId::Ticker`, `show_ticker`, `cfg.ticker`).

Settings subtitle: “Your name is highlighted in the field”.

## Code

- Draw: `draw_ticker`, `draw_ticker_card`, `draw_ticker_meta` in `overlay/hud/src/render.rs`
- Scroll: `HS_SCROLL` (ease toward you) or autoscroll at `HS_AUTO_SPEED` (0.42)
- Card order: `HS_SLIDE` (same `TableSlides` as Standings) so a pass eases cards into the new slots
- Settings: `pane_ticker` in `overlay/src/settings.rs`

## Behavior

- Height is clamped about 42–64 px. Settings layout handles are **east/west only** (`ew_only`).
- Optional title: `WARMUP` / `LAP RACE` / `TIMED` / `EXTRA` / `SESSION` plus track name. Warmup is 10:00 (or 12/15/20 / 30+ min practice) with no extras; not a leftover 8-minute race.
- Side slots (`ticker_left` / `ticker_right`) are `BoardField` (default Lap, Air). **Fuel**, **Fuel %**, **Setup**, **Gap ahead**, and **Gap behind** are options. Ahead/behind are live-order place neighbors, not the card delta vs you.
- Cards show position, name, gap vs you (`ticker_delta` = signed gap difference), last/best. Session-best lap is purple.
- In replay / spectate, clicking a card follows that rider (same camera path as standings names).
- **Riders shown** (`ticker_count`, 3–15) is a target; `hstand_layout` shrinks to what fits at a minimum card width.
- Default: keep you in view (scroll start from your index). **Autoscroll** loops the whole field when there are more cards than fit.
- A pass eases cards into the new slots (`HS_SLIDE`, 0.30s). New riders appear in place; they do not fly in from slot 0.
- Cards are drawn into a clipped layer so they do not paint over the side meta.

## Do not regress

- Keep the code id `Ticker` in ini (`ticker_x`, `show_ticker`, …). The UI name is Horizontal Standings / H-Standings.
- Do not add north/south resize; height is a fixed band.
- Gap on a card is vs **you**, not vs the leader (except you / P1 edge cases via `format_signed_delta`).
- Side-slot **Gap ahead** / **Gap behind** are live-order P−1 / P+1. Same lap ticks along the track toward that rider; a live lap or more is `1L` / `-1L`. Not the card delta vs you.
- Cards iterate `RaceField::board()` (live order), not `s.standings`. Scroll index, slide animation, and the focus card follow that order.
- Click-to-follow only while spectating / replay. Do not capture overlay clicks while riding.
- Setup side-slot is the loaded bike setup filename stem. `--` when `RunInit` has not sent it.

## Change log

- 2026-09-09 — Side-slot **Gap ahead** / **Gap behind** use along-track time on the same lap and `1L` / `-1L` when live laps differ. Times have no leading `+`; ahead is an up arrow, behind a down arrow.
- 2026-09-08 — Side-slot **Gap ahead** / **Gap behind** tick a live running gap to place neighbors. Card gaps stay the signed classification delta vs you.
- 2026-09-08 — **Gap ahead** and **Gap behind** are side-slot options (shared `BoardField`). Place neighbors, not the signed card gap vs you.
- 2026-09-07 — A pass slides the cards into the new order (`HS_SLIDE`), same ease as Standings rows. Instant slot jump was unreadable at race speed.

- 2026-09-03 — **Setup** is a side-slot option (`BoardField::Setup`). Restart MX Bikes after this plugin so SHM `Local\MXBOHudV13` loads.
- 2026-08-29 — **Fuel %** is a separate side-slot option from volume.

- 2026-09-10 — Speed, Liquids, and Temperature units are independent in Settings. Legacy `units=` still seeds all three on upgrade.
- 2026-08-29 — Fuel reads as liters or US gallons from Liquids units, not percent.

- 2026-08-29 — Fuel level is a side-slot option (`BoardField::Fuel`). Tank percent; `--` if max fuel is missing.

- 2026-08-26 — Clicking a card in replay / spectate moves the camera to that rider. Same command SHM as standings (`Local\MXBOHudCmdV1`). Hit box is the visible card.

- 2026-08-25 — Cards follow the live race order, so a pass slides the field immediately instead of at the line. See [live race order](../live-order.md).
- 2026-08-24 — Card gap-vs-you and session best come from shared `RaceStore`. No visual change.
- 0.1.2 — Horizontal Standings bar added (leader left, you highlighted).
- 0.1.8 — Hidden until **Show on overlay**.
- 2026-08-20 — Dropped the carbon-fiber dot grid on the bar background; those per-pixel fills stalled the whole HUD.
