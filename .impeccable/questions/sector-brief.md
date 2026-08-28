# Sectors widget

## Scope
Operate · overlay HUD widget `draw_sector` in `overlay/hud/src/render.rs`. Labs-only until Experimental widgets is on.

## Audience / job
MX Bikes racer mid-session. Glance: am I up or down vs my best on this split. Eyes stay on the track.

## Direction
Split board + skew plaques. Night-ink 6px panel, three stacked rows like standings. Last completed sector is you-row gold. Delta lives in a content-sized 4px-skew parallelogram (violet PB / red slower / green ahead / charcoal pending). Split time is secondary on the right. Not an F1 timing tape.

## Memorable moment
The split you just took flashes you-row gold with a colored skew plaque — same "you" grammar as standings.

## Approved comp
`.impeccable/mocks/sector-comp-approved.png`

## Unresolved
None.

## Inventory
| Region | Medium | Notes |
| --- | --- | --- |
| Panel | tiny-skia fill_round | night-ink #0A0A0A, 6px, opacity from sector_bg |
| Fresh row | fill_rect | you-row #C48424 @ ~72% |
| Stripe | fill_rect | standings stripe_row_bg on odd non-fresh rows |
| S# | Exo 2 ExtraBold Italic | dim; white on fresh |
| Delta plaque | fill_skew 4px | content-sized; violet/red/green/charcoal |
| Delta ink | text | ink_on(fill); dim on charcoal |
| Split time | col_text right | dim; white on fresh |
| Type | existing HUD face | ExtraBold Italic via push_style |
