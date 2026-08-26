---
version: 1
slug: "overlay-src-settings-rs"
primary_target: "overlay/src/settings.rs"
related_targets: []
---

# Settings window

## Scope
Operate · F8 host window (`overlay/src/settings.rs`). Same tabs, same controls, same copy.

## Audience / job
Rider first-run (turn widgets on), mid-session tweak, then depth (columns, snap).

## Direction
Show Plaque chrome is the orange **name** plaque (mid-session heading). **Show on overlay** sits on the right of that strip. Columns: Header Strip list — drag, width slider, **toggle to show/hide**. Header and Footer: exactly three slots (Left, Middle, Right). Top bar: Widgets / Settings / Feedback (selected = orange skew plaque). Widget rail grouped Boards / Track / Cockpit; rail hides on Settings and Feedback. Quit lives on the top bar.

## Memorable moment
Orange Show plaque is the first thing on a widget pane; when the widget is off, it is the only control.

## Approved comp
`.impeccable/mocks/settings-approved.png`

## Unresolved
None.

## Inventory
| Region | Medium |
| Show on overlay plaque | tiny-skia skew fill + large switch |
| Top mode bar | Widgets / Settings / Feedback skew plaques + Quit |
| Rail / tabs | grouped nav_tab + orange pip; hidden on Settings/Feedback |
| Look controls | existing sliders/toggles/dropdowns, two-column on Standings/Relative |
| Header / Footer | 3 dropdown slots (Left, Middle, Right) |
| Columns | field_row: grip, name, width slider, show/hide toggle |
| Snap grid | existing look_section |
