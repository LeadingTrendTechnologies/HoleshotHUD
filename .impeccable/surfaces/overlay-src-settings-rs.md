# Settings window

## Scope
Operate · F8 host window (`overlay/src/settings.rs`). Same tabs, same controls, same copy. What's new is a Center Plaque modal after in-app update (`overlay/src/changelog.rs` supplies this version's notes).

## Audience / job
Rider first-run (turn widgets on), mid-session tweak, then depth (columns, snap). After an in-app update: read this version's notes, Got it, get back to Settings. Re-open from App → Updates.

## Direction
Show Plaque chrome is the orange **name** plaque (mid-session heading). **Show on overlay** sits on the right of that strip. Columns: Header Strip list — drag, width slider, **toggle to show/hide**. Header and Footer: exactly three slots (Left, Middle, Right). Top bar: Widgets / Settings / Feedback (selected = orange skew plaque). Widget rail grouped Boards / Track / Cockpit; rail hides on Settings and Feedback. Quit lives on the top bar. What's new: dimmed Settings (Charcoal Side scrim ~80%), centered Panel board (~520px, 10px), **content-sized** orange skew version plaque as the heading (left, like Settings pane titles), this version's headline + section bullets, full-width **Got it**. Re-open from App → Updates via secondary **What's new**.

## Memorable moment
Orange Show plaque is the first thing on a widget pane; when the widget is off, it is the only control. After update: the orange version plaque is the heading of the notes board.

## Approved comp
`.impeccable/mocks/settings-approved.png`

## What's new approved comp
`.impeccable/mocks/whats-new-center.png`

## Unresolved
None.

## Inventory
| Region | Medium |
| Show on overlay plaque | tiny-skia skew fill + large switch |
| Top mode bar | Widgets / Settings / Feedback skew plaques + Quit |
| Rail / tabs | grouped nav_tab + orange pip; hidden on Settings/Feedback |
| Look controls | existing sliders/toggles/dropdowns, two-column on Standings/Relative |
| Simple dash | existing toggle_row on Dash pane; hides rev + footer slots while on |
| Header / Footer | 3 dropdown slots (Left, Middle, Right) |
| Columns | field_row: grip, name, width slider, show/hide toggle |
| Snap grid | existing look_section |
| What's new scrim | tiny-skia fill Charcoal Side `#08080A` at 204 alpha (~80%) |
| What's new board | tiny-skia round rect Panel `#141416`, 10px, 520px max, 22px pad |
| Version plaque | content-sized fill_skew (label + 36px, min 72px, 40px × 6px skew, 18px Exo); version is the heading, not an eyebrow |
| Headline / bullets | Exo 2 ExtraBold Italic via text(); headline 16px; bullets 12px; section labels 10px uppercase dim; 2.2px orange dots |
| Got it | existing action_btn primary, 40px, full inner width of board |
| What's new (Updates) | existing action_btn secondary, 120×32 |
