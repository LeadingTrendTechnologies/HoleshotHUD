# MX Bikes menu pack (`game_ui`) — cleanup backlog

**P1–P3 landed 2026-09-21.** Helpers live in [`overlay/src/game_ui/shell.rs`](../overlay/src/game_ui/shell.rs); pack apply is table-driven (`MENU_SPLICES`); sources split under [`overlay/src/game_ui/`](../overlay/src/game_ui/). Use this page for later refactors (P4+, page coverage).

Toggle: Settings → In game HUD → **MX Bikes menus** (`game_ui` in ini).

Related: [Home.md](Home.md) · [rust-patterns.md](rust-patterns.md) · product wishlist [Updates.md](Updates.md) (not this page).

---

## Current shape

| Module | Concern |
| --- | --- |
| `pack.rs` | apply / remove, bak, manifest, `MENU_SPLICES` |
| `io.rs` | stock `.mnu` reads, `write_text`, english.str patches |
| `sprites.rs` | TGA writers + `SPRITES` / `STOCK_CHROME` |
| `shell.rs` | shared restyle primitives + ink recolor |
| `mnu/` | `main_mnu` / `ui_ui` authors + `splice_*` / `restyle_*` |

What still hurts change velocity: **per-dialog deltas** in `mnu/` (large file) and fragile stock `.replace` matches. Prefer composing shell helpers for new page cleanups.

```
Landed: MenuSplice table → shared shell ops → thin per-dialog deltas (ongoing)
```

---

## Pain points

- **Fragile string matches** — stock multiline snippets break when PiBoSo tweaks whitespace or rects
- **Giant inline test fixtures** — CRLF stock blobs embedded in unit tests (`mod.rs` tests)
- **`mnu/mod.rs` still large** — further split into domain files is optional follow-up
- **Per-page sprite quirks** — Host Setup hover-only sprites vs usual `sprite2 back1` / `done1`

---

## Suggested changes

Each item: what / why / break risk / need before saying yes.  
Priority is **value vs risk**. Skip [Do not apply](#do-not-apply).

### P1 — Shared shell primitives — **done (2026-09-21)**

Helpers live in `shell.rs`. Host / Race / Profiles / Best Laps / View Replays / Bike Choose chrome use them.

**What.** Extract pure string transforms used by setup-style pages:

| Helper | Responsibility |
| --- | --- |
| `clear_title_wash(s)` | `backcolor 127 0 0 0` → clear |
| `glass_board(s, stock, glass, y1)` | board sprite swap + Y1 clamp to `SETUP_BOARD_Y1` |
| `clear_footer_bar(s)` | footer darkbox → chrome Y, clear color |
| `lift_chrome_button(s, name, x0, x1)` | stock footer rect → chrome slot |
| `force_idle_plaques(s, …)` | Host `back2`/`done2` quirks + usual `back1`/`done1` |
| `paint_chrome_row(s, backs, cta)` | `center_bike_chrome_button` + `paint_bike_chrome_button` batch |
| `frost_track_bar(s)` | beige `ID_DARKBOX` → `setupbar.tga` |

Then refactor onto them: `restyle_host_setup`, `restyle_race_setup`, `restyle_profiles_dialog`, `restyle_viewreplays_dialog`, `lift_bikechoose_chrome` (and prefer this for future page cleanups).

**Why.** New page cleanup becomes “compose helpers + page-specific deltas,” not another 80-line clone.

**Break logic?** **Low** if transforms are behavior-identical and existing `game_ui` tests stay green. Live risk: MX Bikes must fully restart to reload menus; locked `ui/` while the game runs still needs the existing retry path.

**Need to know.** Exact stock snippets for Host `back2`/`done2` must remain covered by helpers or page-specific overrides. Do **before** a module split so moves stay mechanical.

### P1 — Unify ink recolor — **done (2026-09-21)**

`recolor_menu_ink` is the shared pass; `recolor_profiles_dialog` wraps it; `recolor_testsetup_dialog` = ink + setup board remaps.

**What.** Merge the shared half of `recolor_profiles_dialog` and `recolor_testsetup_dialog` into `recolor_menu_ink(s, accent)`, then optional board remaps (`dialog940` → `setupboard_r`, etc.).

**Why.** One place for TEXT / accent / pull / section-header ink; fewer missed `color 255 0 0 0` cases on glass.

**Break logic?** **Medium** — ink swaps are easy to over-replace (e.g. `pullbackcolor` vs `backcolor`). Golden path: unit tests per splice that already assert colors/sprites.

**Need to know.** Which callers need board remaps vs ink-only (Host Setup form vs Profiles list).

### P2 — Table-driven pack apply — **done (2026-09-21)**

**What.** Replace the `if splice_*` chain in `apply_pack_inner` with a registry:

```rust
struct MenuSplice {
    file: &'static str,
    run: fn(&Path, Option<&Path>, [u8; 3]) -> Result<bool, String>,
}
```

Same idea inside each splice: dialog name → restyle fn map.

**Why.** Adding a menu = one table row + one function. Harder to forget manifest registration.

**Break logic?** **Low** if order of splices and “skip when stock missing” stay the same.

**Need to know.** `main.mnu` / `ui.ui` stay authored (not spliced); keep them outside the table or as a separate “always write” step.

### P3 — Module split — **done (2026-09-21)**

**What.** Keep public API on `game_ui` (`sync_from_config`, `apply`, `remove`, `MANIFEST`, restart flags). Internals:

- `game_ui/pack.rs` — backup, manifest, pkz, apply/remove, `MENU_SPLICES`
- `game_ui/io.rs` — stock reads, text write, english.str
- `game_ui/sprites.rs` — TGA writers + `SPRITES` / `STOCK_CHROME`
- `game_ui/screens.rs` — opening `splash.tga` + loading `bkgrnd.tga` (defaults or Browse PNG/JPG)
- `game_ui/shell.rs` — P1 primitives + ink recolor
- `game_ui/mnu/` — authors + splices (further domain files optional)
- tests remain in `game_ui/mod.rs` for now

**Why.** Navigation and review size; agents stop loading 7k lines for a one-page tweak.

**Break logic?** **Low** if pure moves + `pub(crate)`. Compile + `cargo test --bin Holeshot-HUD game_ui` is the gate.

**Need to know.** Do P1 first so `shell` is a real module, not an empty placeholder.

### P4 — Later (only if stock breaks often)

| Item | Why | Risk |
| --- | --- | --- |
| Light `.mnu` tokenizer (dialogs / items / key-values) | Ops by path instead of exact multiline strings | Higher — invest after shell ops stabilize |
| Stock fixture corpus (from `ui.holeshot-bak` or checked-in snippets) | Tests stop embedding 2kb CRLF blobs | Low–medium (path / license for shipping fixtures) |
| Manifest schema version | Migrate when `SPRITES` / chrome lists change | Low if versioned carefully |

---

## Page coverage backlog

Track “clean up this screen” work. Status is about **pack treatment**, not product polish.

| Menu / dialog | File | Status (2026-09-21) | Notes |
| --- | --- | --- | --- |
| Main dest card | `main.mnu` | Authored | Full rewrite |
| `ui.ui` | `ui.ui` | Authored | Pointer / tooltip colors |
| Server browser | `multijoin.mnu` | Spliced | Inset `serverboard` |
| Options | `options.mnu` | Spliced | Chrome + form remaps |
| Connection modal | `connection.mnu` | Spliced | Glass confirm-style |
| Practice Setup | `testsetup.mnu` | Spliced | Authored chrome + form recolor |
| Profiles / Best Laps | `profiles.mnu` | Spliced | Glass board + chrome |
| View Replays | `viewreplays.mnu` | Spliced | Same family as Profiles |
| Replay HUD | `replay.mnu` | Spliced | Frost panels + plaques |
| Multi pit / race info | `multiclient.mnu` | Spliced | `restyle_pit_shell` |
| Practice pit / test panel | `test.mnu` | Spliced | Pit shell + panels |
| Garage | `garage.mnu` | Spliced | Setup bars + panes |
| Host Setup | `hostsetup.mnu` → `idd_host_setup` | Spliced | Glass + chrome (2026-09) |
| Race Setup shell | `hostsetup.mnu` → `idd_race_setup` | Spliced | Title / track bar / chrome (2026-09) |
| Race Setup form | `hostsetup.mnu` → `idd_host_racesetup` | Spliced | `setupboard_r` + field pills |
| Export | `export.mnu` | Listed in `ui.ui` only | No splice yet |
| Test day | `testday.mnu` | Listed in `ui.ui` only | No splice yet |
| Straight rhythm | `straightrhythm.mnu` | Listed in `ui.ui` only | No splice yet |

When cleaning a page: prefer P1 helpers; add a focused unit test; fully quit MX Bikes to reload menus.

---

## Do not apply

- **Rewrite every menu as a full generated dialog** (Practice Setup style) — loses stock control IDs/layout and fights game updates
- **Big-bang `.mnu` parser + module split in one PR** — high regression risk on live installs
- **Abstract one-off modals early** (connection, gate choose) until a second sibling shares the recipe
- **Silent expand of manifest / `STOCK_CHROME` without remove tests** — toggle-off must restore stock (see pointer/arrow restore)

---

## Suggested next steps

P1–P3 done. Useful follow-ups: page coverage cleanups using shell helpers, optional `mnu/` domain split, or P4 if stock `.replace` breaks often.

---

## Related bugs / pack hygiene (already fixed or adjacent)

| Topic | Notes |
| --- | --- |
| Toggle-off incomplete restore | Pointer + spin arrows must be in manifest; bak gap-fill from `ui.pkz` even when stock seed exists |
| Locked `ui/` while game runs | Existing `NEED_RETRY` / restart banner — not fixed by refactor |
| Accent skip | `pack_accent_current` skips rewrite when accent matches — force path still available |
