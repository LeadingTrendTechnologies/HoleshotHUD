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
- Side slots (`ticker_left` / `ticker_right`) are `BoardField` (default Lap, Air). **Fuel**, **Fuel %**, **Setup**, **Gap ahead**, **Gap behind**, **Delta**, **Last** / **Last lap diff** / **Current**, **Gap to leader**, **Engine**, **Penalty**, and **Server** are options. **Last lap diff** is your last completed lap minus the one before it (not Delta versus best): green faster, red slower, `0.000` the same, `--` until two laps. Ahead/behind are live-order place neighbors. That live gap is not the number on the card.
- Cards show position, name, and the classification interval to the rider one place ahead (`+1.234` or `+1L`, same as the Standings interval). The leader (live P1, including you) shows a lap time: last lap, then your snapshot last lap when that card is you, then best. Session-best lap is purple. Position gets a trailing green/red `*` when live place ≠ on-track place due to penalties (see [live race order](../live-order.md)).
- Your card is a left-to-right fade of the accent with a 1px line on all four edges and 4px rounded corners. **Row highlight** opacity is adjustable in settings (`ticker_hl`, default 50 — same scale as Standings / Relative) and scales that fade and that frame. Standings and Relative keep a top-and-bottom line only.
- Optional **Status** (`ticker_status`, default off) appends a finish / crash / DNS / OUT / DSQ / pit icon at the **end** of each card. Finished riders show only the flag. When on, gap stays gap (status is not written over it). When off, DNS/OUT/DSQ/PIT still replace the gap text for out riders (legacy).
- In replay / spectate, clicking a card follows that rider (same camera path as standings names).
- **Riders shown** (`ticker_count`, 3–15) is a target; `hstand_layout` shrinks to what fits at a minimum card width.
- Default: keep you in view (scroll start from your index). A camera change does not scroll the strip while that card is already showing. **Autoscroll** loops the whole field when there are more cards than fit.
- A pass eases cards into the new slots (`HS_SLIDE`, 0.30s) when **Slide on pass** (`ticker_slide`, default on) is enabled. Off: cards jump to the new slots. New riders appear in place; they do not fly in from slot 0.
- Cards are drawn into a clipped layer so they do not paint over the side meta.

## Do not regress

- Keep the code id `Ticker` in ini (`ticker_x`, `show_ticker`, …). The UI name is Horizontal Standings / H-Standings.
- Do not add north/south resize; height is a fixed band.
- The line under the name is the classification interval to the live-order rider ahead (`interval_text_from_row`), not the gap versus you and not the live along-track Gap ahead side slot. The leader (live P1) shows a lap time.
- Side-slot **Last lap diff** is the last completed lap minus the previous one, not Delta versus best. A zeroed last-lap does not replace the stored time. A new session clears it.
- Side-slot **Gap ahead** / **Gap behind** are live-order P−1 / P+1. Same lap ticks along the track toward that rider; a live lap or more is `1L` / `-1L`. Not the card interval.
- Cards iterate `RaceField::board()` (live order), not `s.standings`. Scroll index, slide animation, and the focus card follow that order.
- Click-to-follow only while spectating / replay. Do not capture overlay clicks while riding.
- Changing the camera must not slide cards that are already in the window. Scroll only when the watched card is fully outside.
- Setup side-slot is the loaded bike setup filename stem. `--` when `RunInit` has not sent it.
- With **Status** on, keep gap/delta on the card and put the mark at the trailing edge — do not replace gap with status text.
- Finished riders show only the Status finish flag — not crash / pit / DNS / OUT / DSQ on top of it.
- **Slide on pass** off must snap cards to new slots (no `HS_SLIDE` ease). Autoscroll and keep-you-in-view scroll stay independent.

## Change log

- 2026-10-04 — A rider listed on the Groups tab gets that group's icon in the group color, immediately left of the name. The gap line stays put. The first group in the list wins.
- 2026-10-03 — The line under each name is the classification interval to the rider ahead. The leader's card shows a lap time. It was the gap versus you, and your card always showed your last lap.
- 2026-10-03 — Your card's 4px corners were cut off: the frame was four 1px strips clipped by the rounded mask, and a stroke on the outer edge is clipped by the card layer. The frame is a 1px stroke inset half a pixel so the arc stays on the card.
- 2026-10-03 — Your card's frame has 4px rounded corners. Clicking a card that is already on screen follows that rider and does not slide the strip.
- 2026-10-03 — Your card's accent line wraps all four edges. Standings and Relative stay top and bottom only. **Row highlight** still scales the wash and the frame.
- 2026-10-01 — Your card is a left-to-right accent fade with a 1px line on the top and bottom, same treatment as Standings. **Row highlight** still scales it.
- 2026-09-25 — Side-slot **Last lap diff**. Last completed lap minus the previous one. Green is faster, red is slower. Not Delta versus best.
- 2026-09-25 — Side-slot **BoardField** options: Delta, Last, Current, Gap to leader, Engine, Penalty, Server (shared with Standings / Relative chrome).
- 2026-09-25 — **Row highlight** opacity (`ticker_hl`, default 50) and **Slide on pass** toggle (`ticker_slide`, default on).
- 2026-09-25 — Trailing green/red `*` on card position when live place ≠ on-track place due to penalties.
- 2026-09-22 — Status finish flag wins over crash/pit for done riders (same `standing_mark` as Standings / Relative).
- 2026-09-22 — **Status** toggle (`ticker_status`) appends crash / finish / DNS / OUT / DSQ / pit icons at the end of each card; gap stays when the toggle is on.
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
