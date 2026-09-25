//! Motos list + Analyze paint (shared `mxbo-motos-ui`).

use super::{Drop, Hit, HitBox};
use mxbo_hud::config::Units;
use mxbo_hud::render::Fonts;
use mxbo_review::{self, ListFilter, SessionDetail};
use tiny_skia::Pixmap;

pub(crate) fn pane_review(
    px: &mut Pixmap,
    fonts: &Fonts,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
    analyze_id: Option<i64>,
    filter: ListFilter,
    compare: i32,
    you_lap: i32,
    warmup: bool,
    open_drop: Option<Drop>,
    scrub: f32,
    zoom: f32,
    pan_x: f32,
    pan_z: f32,
    follow: bool,
    recording: bool,
    units: Units,
) -> f32 {
    let mut mh = Vec::new();
    let y = mxbo_motos_ui::pane_review(
        px,
        fonts,
        map_hover(hover),
        &mut mh,
        x,
        y,
        w,
        analyze_id,
        filter,
        compare,
        you_lap,
        warmup,
        map_drop(open_drop),
        scrub,
        zoom,
        pan_x,
        pan_z,
        follow,
        recording,
        units,
    );
    hits.extend(mh.into_iter().map(map_hit_box));
    for menu in mxbo_motos_ui::take_pending_drops() {
        super::queue_drop_menu(
            menu.mx,
            menu.my,
            menu.bw,
            menu.content_h,
            map_hit_to_overlay(menu.open_hit),
            menu.options
                .into_iter()
                .map(|(hit, label, on)| (map_hit_to_overlay(hit), label, on))
                .collect(),
        );
    }
    y
}

pub(crate) fn default_analyze_scrub(detail: &SessionDetail, you_lap: i32, warmup: bool) -> f32 {
    mxbo_motos_ui::default_analyze_scrub(detail, you_lap, warmup)
}

pub(crate) fn follow_pan_for(
    id: i64,
    you_sel: i32,
    warmup: bool,
    scrub: f32,
) -> Option<(f32, f32)> {
    mxbo_motos_ui::follow_pan_for(id, you_sel, warmup, scrub)
}

fn map_hover(h: Option<Hit>) -> Option<mxbo_motos_ui::Hit> {
    h.map(map_hit_to_motos)
}

fn map_drop(d: Option<Drop>) -> Option<mxbo_motos_ui::Drop> {
    d.map(|drop| match drop {
        Drop::AnalyzeCompare => mxbo_motos_ui::Drop::AnalyzeCompare,
        Drop::AnalyzeLap => mxbo_motos_ui::Drop::AnalyzeLap,
        _ => mxbo_motos_ui::Drop::AnalyzeCompare,
    })
}

fn map_hit_box(b: mxbo_motos_ui::HitBox) -> HitBox {
    HitBox {
        id: map_hit_to_overlay(b.id),
        x: b.x,
        y: b.y,
        w: b.w,
        h: b.h,
    }
}

fn map_hit_to_motos(h: Hit) -> mxbo_motos_ui::Hit {
    match h {
        Hit::ReviewFilterAll => mxbo_motos_ui::Hit::ReviewFilterAll,
        Hit::ReviewFilterRanked => mxbo_motos_ui::Hit::ReviewFilterRanked,
        Hit::ReviewFilterSaved => mxbo_motos_ui::Hit::ReviewFilterSaved,
        Hit::ReviewOpen(id) => mxbo_motos_ui::Hit::ReviewOpen(id),
        Hit::ReviewKeep(id) => mxbo_motos_ui::Hit::ReviewKeep(id),
        Hit::ReviewDelete(id) => mxbo_motos_ui::Hit::ReviewDelete(id),
        Hit::ReviewBack => mxbo_motos_ui::Hit::ReviewBack,
        Hit::ReviewToggle => mxbo_motos_ui::Hit::ReviewToggle,
        Hit::ReviewClear => mxbo_motos_ui::Hit::ReviewClear,
        Hit::AnalyzeCompare(n) => mxbo_motos_ui::Hit::AnalyzeCompare(n),
        Hit::AnalyzeCompareOpen => mxbo_motos_ui::Hit::AnalyzeCompareOpen,
        Hit::AnalyzeYouLap(n) => mxbo_motos_ui::Hit::AnalyzeYouLap(n),
        Hit::AnalyzeLapOpen => mxbo_motos_ui::Hit::AnalyzeLapOpen,
        Hit::AnalyzeScrub => mxbo_motos_ui::Hit::AnalyzeScrub,
        Hit::AnalyzeMap => mxbo_motos_ui::Hit::AnalyzeMap,
        Hit::AnalyzeFollow => mxbo_motos_ui::Hit::AnalyzeFollow,
        _ => mxbo_motos_ui::Hit::ReviewBack,
    }
}

fn map_hit_to_overlay(h: mxbo_motos_ui::Hit) -> Hit {
    match h {
        mxbo_motos_ui::Hit::ReviewFilterAll => Hit::ReviewFilterAll,
        mxbo_motos_ui::Hit::ReviewFilterRanked => Hit::ReviewFilterRanked,
        mxbo_motos_ui::Hit::ReviewFilterSaved => Hit::ReviewFilterSaved,
        mxbo_motos_ui::Hit::ReviewOpen(id) => Hit::ReviewOpen(id),
        mxbo_motos_ui::Hit::ReviewKeep(id) => Hit::ReviewKeep(id),
        mxbo_motos_ui::Hit::ReviewDelete(id) => Hit::ReviewDelete(id),
        mxbo_motos_ui::Hit::ReviewBack => Hit::ReviewBack,
        mxbo_motos_ui::Hit::ReviewToggle => Hit::ReviewToggle,
        mxbo_motos_ui::Hit::ReviewClear => Hit::ReviewClear,
        mxbo_motos_ui::Hit::AnalyzeCompare(n) => Hit::AnalyzeCompare(n),
        mxbo_motos_ui::Hit::AnalyzeCompareOpen => Hit::AnalyzeCompareOpen,
        mxbo_motos_ui::Hit::AnalyzeYouLap(n) => Hit::AnalyzeYouLap(n),
        mxbo_motos_ui::Hit::AnalyzeLapOpen => Hit::AnalyzeLapOpen,
        mxbo_motos_ui::Hit::AnalyzeScrub => Hit::AnalyzeScrub,
        mxbo_motos_ui::Hit::AnalyzeMap => Hit::AnalyzeMap,
        mxbo_motos_ui::Hit::AnalyzeFollow => Hit::AnalyzeFollow,
        mxbo_motos_ui::Hit::TabWidgets | mxbo_motos_ui::Hit::TabMotos => Hit::ReviewBack,
    }
}
