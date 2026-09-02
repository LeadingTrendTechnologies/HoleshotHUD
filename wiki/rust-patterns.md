# Rust design patterns (overlay)

Suggestions only. **No code was changed** when this page was written (2026-08-29). Use it when deciding whether a refactor is worth the risk.

Plugin field inventory: [Home.md](Home.md). Widget behavior: [widgets.md](widgets.md). Live order: [live-order.md](live-order.md).

---

## How Rust patterns actually work

Gang-of-Four catalogs (Factory, Visitor, Abstract Factory, Singleton) are a poor default in Rust. The patterns that show up in real crates replace inheritance with **enums**, **traits**, **RAII**, and **making invalid states unrepresentable**.

| Pattern | What it is in Rust | Typical use |
| --- | --- | --- |
| **Algebraic data types (enums)** | A value is exactly one of N variants, each with its own data | Session mode, flags, settings hits, widget ids |
| **Newtype** | `struct RaceNum(i32)` wrapping a primitive | Stop mixing race numbers, ms, and magic `-1` |
| **RAII / Drop** | Resource or style is released when a guard goes out of scope | SHM unmap, font style, DIB, clock log |
| **Typestate** | Type (or enum variant) encodes which operations are legal | Gate → green → white → checkered |
| **Strategy (traits)** | Shared interface, swap the implementation | Widget draw, settings pane, table columns |
| **Facade** | One type hides a pile of helpers | `RaceStore` already aims at this |
| **Builder** | Chain setters, then `build()` | Config construction; Skia paths already do this |
| **Adapter** | Wire `#[repr(C)]` type vs a Rust-friendly view | `Snapshot` vs a per-frame `HudFrame` |
| **Interior mutability** | `Mutex` / `Cell` / atomics instead of `&mut` everywhere | Config, race view, caches |
| **Parse, don't validate** | Construct a typed value at the boundary; later code never sees raw ints | INI, SHM flags, session latches |
| **Iterator adapters** | `filter` / `map` instead of index loops | Rider lists, column packs |
| **From / TryFrom** | Explicit conversions between layers | `WidgetId` ↔ ini key, `i32` ↔ `RaceFlag` |
| **Cow / Arc** | Cheap share, clone only on write | Per-frame config and race view |
| **Command** | Input becomes a small enum, applied later | Settings `Hit` is already this |
| **Actor / channels** | Own a thread, talk via messages | Rarely needed here (one UI loop) |

This overlay **already** uses RAII (`Shm`, `Cmd`, `Dib`, `StyleGuard`, `ClockLog`, `FullscreenFix`), ADTs (`WidgetId`, `StField`, `ClockMode`, `RaceFlag`, `Hit`), seqlock (`Shm::read`), and a facade (`RaceStore::refresh` / `with`). It almost never uses custom traits, `From`/`TryFrom`, or newtypes.

---

## Suggested changes

Each item: what the pattern is, why it would help **this** repo, whether it can break live HUD / plugin logic, and what you need before saying yes.

Priority is **value vs risk**, not “how textbook.” Skip anything in [Do not apply](#do-not-apply).

### 1. `WidgetPrefs` map (composition / EnumMap)

**Pattern.** One `struct WidgetPrefs { rect, show, font, bold, bg, … }` stored per `WidgetId` (array or `enum_map`), instead of twelve parallel field groups on `HudConfig`.

**Why.** `HudConfig` is ~173 fields. `font_pct` / `set_font_pct` / `bold` / `widget_rect` / `snap` each re-match all 12 widgets (`overlay/hud/src/config.rs`). INI load (~211 key arms) and save are mirrors. Adding a widget means touching defaults, load, save, and five accessors.

**Break logic?** **No**, if INI **key names stay the same** (`st_font`, `show_standings`, `map_x`, …). Internals can change; the file on disk cannot without a migrator. SHM `apply_to_snapshot` must keep writing the same rects/flags. Per-widget extras (standings columns, dash layout, flag yellow/blue) stay **outside** the shared prefs.

**Need to know.** Fresh-install defaults and legacy `mxbo.ini` still have to round-trip. Golden render tests do not cover INI. Do this **before** a widget trait, so draw/settings can take `&WidgetPrefs`.

**Verdict.** Highest-clarity, lowest-risk cleanup. Do first if you do any of this.

---

### 2. Shared table painter (extract, not a new type hierarchy)

**Pattern.** One `draw_table_board(...)` (and maybe `paint_row`) used by standings and relative. Column enums (`StField` / `RelField`) stay as the strategy for cell text.

**Why.** `draw_standings` and `draw_relative` in `overlay/hud/src/render.rs` share plaque, header/footer bars, `col_slots`, hug width, stripe, focus highlight, bike pill, and `TableSlides`. They diverge on **data** (live board vs track-pos neighbors), not on chrome.

**Break logic?** **Medium visual risk**, no SHM risk. Relative row *selection* (who is on the board) must stay in relative. Standings windowing (center on you when the field is taller than **Rows**) must stay in standings. Goldens: `overlay/hud/tests/goldens/standings.png`, `relative.png`.

**Need to know.** Read [standings.md](widgets/standings.md) and [relative.md](widgets/relative.md) **Do not regress** before touching. Click-to-follow is name-column hit boxes only.

**Verdict.** Do after `WidgetPrefs` or in the same pass as board chrome tweaks.

---

### 3. Data-driven settings panes (Command already exists)

**Pattern.** `Hit` is already a Command enum (~174 variants). Drive widget panes from a small spec (`id`, shared style rows, extra toggles) instead of twelve near-copies (`pane_standings` … `pane_flag` in `overlay/src/settings.rs`). Nest hits as `Hit::Widget(WidgetId, WidgetHit)` where that is honest.

**Why.** Every widget pane repeats heading → font/bg/bold → unique controls → `look_section`. `dispatch` is ~449 lines of the same match. New widgets currently require new `Hit` variants, a pane, and dispatch arms.

**Break logic?** **Settings UX only**, if config writes stay equivalent. Live overlay is untouched unless a toggle is wired to the wrong field. Standings/relative still hand-roll some style rows instead of only `style_controls` — unify those first so a spec does not drop **Row highlight** / **Text color** / **Alternating rows**.

**Need to know.** App / Feedback / What’s new panes stay custom. High-contrast palette in settings must stay OS-aware (not the HUD palette).

**Verdict.** **Done (2026-08-31).** `WidgetPaneSpec` + `open_widget_pane` + `table_style_controls` for standings/relative. Hits stay flat (no `Hit::Widget` nest). App / Feedback / What’s new still custom.

---

### 4. Session FSM as one enum (typestate / parse-don't-validate)

**Pattern.** Replace ~30 `AtomicI32` latches in `overlay/hud/src/race_store.rs` (`IN_GATE`, `CHECKERED_LATCH`, `RUN_IN_FLAG` 0/1/2, `WHITE_WAVE_*`, S/F learn, overtime bases, …) with a `Mutex<SessionFsm>` (or a handful of typed fields). `ClockMode` and `RaceFlag` already exist; most of the machine is still magic integers. Move S/F run-in and flag approach **out of** `render.rs` so render only **draws** `DashFlag`.

**Why.** Flag/clock bugs are latch-order bugs. `RaceStore` comments say tick once from `draw`, but `render.rs` still reads and writes the same atomics (`note_line_progress`, `hold_across_line`, dash flag). Two `anim_now()` clocks (`render.rs` vs `race_store.rs`) make that worse.

**Break logic?** **Yes, easily** — white/checkered, extras, gate, warmup vs race, S/F calibration without a centerline. This is the highest-value cleanup and the highest **behavior** risk. SHM layout is unchanged.

**Need to know.** Invariants live in [dash.md](widgets/dash.md), [flag.md](widgets/flag.md), and comments on the atomics (run-in held across the line because `laps_left` lags; two S/F sightings must agree; timed-extras hint vs lap moto). Needs on-track tests (lap moto, timed + extras, gate, replay), not only goldens. Do **not** mix this with a visual refactor.

**Verdict.** Separate project. Worth it when flag/clock work is already on the table. Not a drive-by.

---

### 5. Widget strategy trait (optional, later)

**Pattern.** `trait HudWidget { fn id() -> WidgetId; fn visible(...); fn draw(...); }` plus a static list. `draw()` in `render.rs` is an if-chain (`show_standings` / `show_minimap` / `sector_visible` / …).

**Why.** A new widget today touches `render.rs`, `config.rs` INI, settings `Hit`/pane, layout `Target`, web-preview, changelog, and a wiki page. A registry would make the **host** loop stable.

**Break logic?** **Draw order / z-order** if the list is reordered (map under dash, flags vs dash wrap). Some widgets still read **snapshot** flags (`s.show_standings`) and others **config** (`cfg.show_minimap`) — that split is load-bearing until you know why. No SHM risk.

**Need to know.** Do **not** start here. Prefs map + table extract give most of the win. A trait over 12 one-off drawers is ceremony unless you are adding widgets often.

**Verdict.** Defer. Use a `WidgetId::ALL` loop only after prefs exist.

---

### 6. Stop cloning the world every frame (interior mutability, used better)

**Pattern.** `HudConfig` is cloned every overlay frame (`overlay/src/main.rs`). `RaceStore::tick` clones into `VIEW` **and** returns a clone; every `draw_*` calls `RaceStore::get()` and clones again.

**Why.** The types are large. The mutex already exists. `tick` can fill `VIEW`; drawers can `RaceStore::with(|s| ...)`. Config can stay behind `with_config` / a read guard except when the layout editor mutates a copy.

**Break logic?** **Low** if lifetimes stay inside `draw()`. **Medium** if you change when `apply_to_snapshot` / `editor.apply_cfg` run relative to draw. Snapshot seqlock copy stays — that is the plugin contract, not this clone.

**Need to know.** WASM preview also ticks `RaceStore`; keep that path. Measure before claiming a perf win (blit/Skia likely dominate).

**Verdict.** **Done (2026-08-31).** `refresh` + re-entrant `with` on the draw path; `HudConfig` cloned only while the layout editor has a cfg-backed preview. Not a hitch fix — Skia/blit still dominate.

---

### 7. Layout editor in `mxbo-hud` (DRY / facade)

**Pattern.** Move pure geometry (`Target`, `Handle`, `rect_of`, `set_rect`) into the HUD crate. Host adds Win32 cursor; web-preview adds CSS cursors.

**Why.** `overlay/src/layout.rs` and `web-preview/src/edit.rs` are the same editor. Web `Target` **omits Stance**, so the demo cannot drag that widget the same way.

**Break logic?** **Low** for the overlay if hit tests stay identical. Unify Stance on the web target list or document that the demo skips it on purpose.

**Need to know.** Host still owns `GetCursorPos` / `VK_CONTROL`. Do not pull `windows` into `mxbo-hud`.

**Verdict.** Do when touching layout or the website demo.

---

### 8. `HudFrame` adapter (keep `Snapshot` as wire)

**Pattern.** Keep `#[repr(C)] Snapshot` as the seqlock memcpy type. Build a cheap view once per frame (`&[Standing]`, `bool` show flags, slices instead of `i32` counts).

**Why.** Widgets scatter `s.show_map != 0` and `standing_count.max(0) as usize`. An adapter is the typed boundary; it does not replace SHM.

**Break logic?** **Critical** if you change `Snapshot` field order, `MAGIC`, `VERSION`, or `Local\MXBOHudV12`. The adapter must not invent fields the plugin did not publish.

**Need to know.** Version &lt; 9 backfill in `Shm::read` must remain. Spectate still clears `has_telemetry` on the **copy** in `main.rs` before draw.

**Verdict.** Nice after prefs; never a reason to bump SHM.

---

### 9. Newtypes for IDs and times (optional)

**Pattern.** `struct RaceNum(i32)`, `struct Millis(i32)` with `Option` instead of `-1` sentinels where the value is overlay-only.

**Why.** Race numbers, lap counts, and “not yet” (`-1`) share `i32` across race_store and render.

**Break logic?** **Yes** if applied to SHM structs (`Standing.race_num` must stay `i32` for C layout). Overlay-only state can use newtypes; convert at the Snapshot edge.

**Need to know.** Only worth it if you do the session FSM anyway. Do not newtype the wire struct.

**Verdict.** Bundle with item 4 or skip.

---

### 10. `static mut HOST` → atomic / OnceLock

**Pattern.** The only `static mut` in overlay Rust is `HOST: HWND` in `main.rs`. Everything else already uses atomics/mutexes.

**Why.** Tiny soundness cleanup on the UI thread.

**Break logic?** **No**, if set/clear still happens on the same thread as today (`quit_app` / create).

**Verdict.** **Done (2026-08-31).** `static HOST: AtomicIsize` with `host_hwnd` / `set_host`, same pattern as the tray.

---

## Do not apply

| Pattern | Why not here |
| --- | --- |
| **Visitor** | No AST. Enums + `match` are the visitor. |
| **Abstract Factory / Factory Method** | Widgets are a closed list of 12. `WidgetId` is enough. |
| **Singleton as a goal** | Too many globals already (`CONFIG`, `VIEW`, session atomics, TLS caches). Prefer fewer. |
| **Actor / mpsc for the HUD** | One render loop; channels would add latency and races vs seqlock. |
| **serde on the INI** | Users’ `Holeshot-HUD.ini` is a hand-written key list with legacy names. A migrator is mandatory; serde is not the win. |
| **Trait objects in the Skia hot path** | `dyn HudWidget` per widget per frame is noise vs a static list of fns. |
| **Changing SHM because of a pattern** | Plugin C header and Rust `Snapshot` must stay in lockstep. Patterns stop at the copy. |

---

## Suggested order (if you later do this)

1. `WidgetPrefs` + keep INI keys (1)
2. Pass `&RaceStore` / stop extra clones (6), `HOST` atomic (10)
3. Shared table painter (2)
4. Settings pane spec (3)
5. Layout editor into hud (7)
6. `HudFrame` adapter (8)
7. Session FSM only with a dedicated test plan (4, 9)
8. Widget trait only if you are about to add several widgets (5)

---

## Files these suggestions touch

| Area | Files |
| --- | --- |
| Config / INI | `overlay/hud/src/config.rs` |
| Draw | `overlay/hud/src/render.rs` (~6.2k), goldens under `overlay/hud/tests/goldens/` |
| Race / flags | `overlay/hud/src/race_store.rs`, flag/S/F code in `render.rs` |
| Settings | `overlay/src/settings.rs` (~5.7k) |
| Frame loop | `overlay/src/main.rs` |
| SHM (do not restyle) | `overlay/src/shm.rs`, `overlay/hud/src/snapshot.rs`, `src/shm/mxbo_shm.h` |
| Layout twin | `overlay/src/layout.rs`, `web-preview/src/edit.rs` |
