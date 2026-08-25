# Horizontal Standings

Top bar of rider cards, leader on the left, your name highlighted. Settings tab label is **H-Standings**. Code name is **Ticker** (`WidgetId::Ticker`, `show_ticker`, `cfg.ticker`).

Settings subtitle: “Your name is highlighted in the field”.

## Code

- Draw: `draw_ticker`, `draw_ticker_card`, `draw_ticker_meta` in `overlay/hud/src/render.rs`
- Scroll: `HS_SCROLL` (ease toward you) or autoscroll at `HS_AUTO_SPEED` (0.42)
- Settings: `pane_ticker` in `overlay/src/settings.rs`

## Behavior

- Height is clamped about 42–64 px. Settings layout handles are **east/west only** (`ew_only`).
- Optional title: `WARMUP` / `LAP RACE` / `TIMED` / `EXTRA` / `SESSION` plus track name. Warmup is 10:00 (or 12/15/20 / 30+ min practice) with no extras; not a leftover 8-minute race.
- Side slots (`ticker_left` / `ticker_right`) are `BoardField` (default Lap, Air).
- Cards show position, name, gap vs you (`ticker_delta` = signed gap difference), last/best. Session-best lap is purple.
- **Riders shown** (`ticker_count`, 3–15) is a target; `hstand_layout` shrinks to what fits at a minimum card width.
- Default: keep you in view (scroll start from your index). **Autoscroll** loops the whole field when there are more cards than fit.
- Cards are drawn into a clipped layer so they do not paint over the side meta.

## Do not regress

- Keep the code id `Ticker` in ini (`ticker_x`, `show_ticker`, …). The UI name is Horizontal Standings / H-Standings.
- Do not add north/south resize; height is a fixed band.
- Gap on a card is vs **you**, not vs the leader (except you / P1 edge cases via `format_signed_delta`).
- Cards iterate `RaceField::board()` (live order), not `s.standings`. Scroll index and the focus card follow that order.

## Change log

- 2026-08-25 — Cards follow the live race order, so a pass slides the field immediately instead of at the line. See [live race order](../live-order.md).
- 2026-08-24 — Card gap-vs-you and session best come from shared `RaceStore`. No visual change.
- 0.1.2 — Horizontal Standings bar added (leader left, you highlighted).
- 0.1.8 — Hidden until **Show on overlay**.
- 2026-08-20 — Dropped the carbon-fiber dot grid on the bar background; those per-pixel fills stalled the whole HUD.
