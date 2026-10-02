#![allow(unused_imports)]
use super::*;

#[derive(Clone, Copy)]
pub(crate) struct WidgetPaneSpec {
    pub(crate) id: WidgetId,
    pub(crate) title: &'static str,
    pub(crate) subtitle: &'static str,
    pub(crate) show: Hit,
    pub(crate) bg: Hit,
    pub(crate) bg_label: &'static str,
}

pub(crate) fn widget_pane_spec(id: WidgetId) -> WidgetPaneSpec {
    match id {
        WidgetId::Standings => WidgetPaneSpec {
            id,
            title: "Standings",
            subtitle: "Who is ahead and by how much",
            show: Hit::StShow,
            bg: Hit::StBg,
            bg_label: "Background",
        },
        WidgetId::Relative => WidgetPaneSpec {
            id,
            title: "Relative",
            subtitle: "Riders just ahead and behind you",
            show: Hit::RelShow,
            bg: Hit::RelBg,
            bg_label: "Background",
        },
        WidgetId::Map => WidgetPaneSpec {
            id,
            title: "Map",
            subtitle: "Where you and others are on track",
            show: Hit::MapShow,
            bg: Hit::MapBg,
            bg_label: "Background",
        },
        WidgetId::Minimap => WidgetPaneSpec {
            id,
            title: "Minimap",
            subtitle: "Circular track with numbered riders",
            show: Hit::MiniShow,
            bg: Hit::MiniBg,
            bg_label: "Background",
        },
        WidgetId::Radar => WidgetPaneSpec {
            id,
            title: "Radar",
            subtitle: "Riders beside and behind you",
            show: Hit::RadarShow,
            bg: Hit::RadarBg,
            bg_label: "Panel opacity",
        },
        WidgetId::Dash => WidgetPaneSpec {
            id,
            title: "Dash",
            subtitle: "Gear, speed, and footer stats",
            show: Hit::DashShow,
            bg: Hit::DashBg,
            bg_label: "Panel opacity",
        },
        WidgetId::Ticker => WidgetPaneSpec {
            id,
            title: "Horizontal Standings",
            subtitle: "Your name is highlighted in the field",
            show: Hit::TickerShow,
            bg: Hit::TickerBg,
            bg_label: "Panel opacity",
        },
        WidgetId::Sys => WidgetPaneSpec {
            id,
            title: "Systems",
            subtitle: "CPU, memory, FPS, ping, GPU, and per-app load",
            show: Hit::SysShow,
            bg: Hit::SysBg,
            bg_label: "Panel opacity",
        },
        WidgetId::Sector => WidgetPaneSpec {
            id,
            title: "Sectors",
            subtitle: "Split times vs your best, plus lap time, delta, and ideal",
            show: Hit::SectorShow,
            bg: Hit::SectorBg,
            bg_label: "Panel opacity",
        },
        WidgetId::Delta => WidgetPaneSpec {
            id,
            title: "Delta Bar",
            subtitle: "Time vs your best at this point on the lap",
            show: Hit::DeltaShow,
            bg: Hit::DeltaBg,
            bg_label: "Panel opacity",
        },
        WidgetId::Stance => WidgetPaneSpec {
            id,
            title: "Stance",
            subtitle: "Sit / stand from a bind you set",
            show: Hit::StanceShow,
            bg: Hit::StanceBg,
            bg_label: "Panel opacity",
        },
        WidgetId::Flag => WidgetPaneSpec {
            id,
            title: "Flags",
            subtitle: "White and checkered — same timing as Dash",
            show: Hit::FlagShow,
            bg: Hit::FlagBg,
            bg_label: "Panel opacity",
        },
        WidgetId::Lean => WidgetPaneSpec {
            id,
            title: "Lean",
            subtitle: "Bike roll, pitch, and steering — follows the camera",
            show: Hit::LeanShow,
            bg: Hit::LeanBg,
            bg_label: "Panel opacity",
        },
        WidgetId::Gamepad => WidgetPaneSpec {
            id,
            title: "Controller",
            subtitle: "Live pad — sticks, triggers, bumpers, and buttons",
            show: Hit::GamepadShow,
            bg: Hit::GamepadBg,
            bg_label: "Panel opacity",
        },
        WidgetId::Telemetry => WidgetPaneSpec {
            id,
            title: "Telemetry",
            subtitle: "Throttle and brake traces, analog bars, gear and speed",
            show: Hit::TelemetryShow,
            bg: Hit::TelemetryBg,
            bg_label: "Panel opacity",
        },
        WidgetId::Pitboard => WidgetPaneSpec {
            id,
            title: "Pit Board",
            subtitle: "Slots from the plate, pick what each shows",
            show: Hit::PitShow,
            bg: Hit::PitBg,
            bg_label: "Panel opacity",
        },
    }
}

pub(crate) fn open_widget_pane(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
    spec: WidgetPaneSpec,
    body: impl FnOnce(&mut Pixmap, &Fonts, f32, bool, &mut Vec<HitBox>) -> f32,
) -> f32 {
    let mut y = heading(
        px,
        fonts,
        x,
        y,
        w,
        spec.title,
        spec.subtitle,
        Some((cfg[spec.id].show, spec.show, "Show on overlay")),
        hover,
        hits,
    );
    let shown = cfg[spec.id].show;
    y = body(px, fonts, y, shown, hits);
    if shown {
        y = look_section(px, fonts, x, y, w, spec.id, hover, hits);
    }
    y
}

pub(crate) fn pane_style(
    px: &mut Pixmap,
    fonts: &Fonts,
    spec: WidgetPaneSpec,
    cfg: &HudConfig,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    style_controls(
        px,
        fonts,
        x,
        y,
        w,
        spec.id,
        cfg,
        spec.bg_label,
        cfg[spec.id].bg,
        spec.bg,
        hover,
        hits,
    )
}

pub(crate) fn pane_standings(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    open_drop: Option<Drop>,
    drag: Option<ColDrag>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let spec = widget_pane_spec(WidgetId::Standings);
    open_widget_pane(
        px,
        fonts,
        cfg,
        hover,
        hits,
        x,
        y,
        w,
        spec,
        |px, fonts, y, shown, hits| {
            if !shown {
                return y;
            }
            let mut y = table_style_controls(
                px,
                fonts,
                spec,
                cfg,
                hover,
                open_drop,
                hits,
                x,
                y,
                w,
                cfg.standings.st_hl,
                Hit::StHl,
                cfg.standings.st_text,
                Drop::StText,
                Hit::StTextOpen,
                Hit::StTextWhite,
                Hit::StTextBlack,
                cfg.standings.st_stripe,
                Hit::StStripe,
                cfg.standings.st_plaque_text,
                Drop::StPlaqueText,
                Hit::StPlaqueTextOpen,
                Hit::StPlaqueTextWhite,
                Hit::StPlaqueTextBlack,
                cfg.standings.st_plaque,
                Hit::StPlaque,
            );
            let mut g = PairGrid::new(x, y, w);
            g.place(|cx, cy, cw| {
                stepper_row(
                    px,
                    fonts,
                    cx,
                    cy,
                    cw,
                    "Rows",
                    &cfg.standings.standings_rows.to_string(),
                    Hit::StDec,
                    Hit::StInc,
                    hover,
                    hits,
                )
            });
            y = g.end();
            y = board_slots_section(
                px,
                fonts,
                x,
                y,
                w,
                "Header",
                InfoBar::StHead,
                cfg.standings.st_head,
                open_drop,
                hover,
                hits,
            );
            y = board_slots_section(
                px,
                fonts,
                x,
                y,
                w,
                "Footer",
                InfoBar::StFoot,
                cfg.standings.st_foot,
                open_drop,
                hover,
                hits,
            );
            y = section(
                px,
                fonts,
                x,
                y,
                "Columns  ·  drag to reorder · slide width · toggle to show",
            );
            paint_col_field_rows(
                px,
                fonts,
                cfg,
                &cfg.standings.st_order,
                DragKind::St,
                drag,
                hover,
                x,
                y,
                w,
                hits,
                |f| f as i32,
                |f| f.label(),
                |f, c| f.enabled(c),
                |f, c| f.width(c),
                |f| f.width_max(),
                Hit::StDrag,
                st_toggle,
                Hit::StW,
            )
        },
    )
}

pub(crate) fn pane_relative(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    open_drop: Option<Drop>,
    drag: Option<ColDrag>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let spec = widget_pane_spec(WidgetId::Relative);
    open_widget_pane(
        px,
        fonts,
        cfg,
        hover,
        hits,
        x,
        y,
        w,
        spec,
        |px, fonts, y, shown, hits| {
            if !shown {
                return y;
            }
            let mut y = table_style_controls(
                px,
                fonts,
                spec,
                cfg,
                hover,
                open_drop,
                hits,
                x,
                y,
                w,
                cfg.relative.rel_hl,
                Hit::RelHl,
                cfg.relative.rel_text,
                Drop::RelText,
                Hit::RelTextOpen,
                Hit::RelTextWhite,
                Hit::RelTextBlack,
                cfg.relative.rel_stripe,
                Hit::RelStripe,
                cfg.relative.rel_plaque_text,
                Drop::RelPlaqueText,
                Hit::RelPlaqueTextOpen,
                Hit::RelPlaqueTextWhite,
                Hit::RelPlaqueTextBlack,
                cfg.relative.rel_plaque,
                Hit::RelPlaque,
            );
            let mut g = PairGrid::new(x, y, w);
            g.place(|cx, cy, cw| {
                stepper_row(
                    px,
                    fonts,
                    cx,
                    cy,
                    cw,
                    "Nearby riders",
                    &cfg.relative.relative_count.to_string(),
                    Hit::RelDec,
                    Hit::RelInc,
                    hover,
                    hits,
                )
            });
            y = g.end();
            y = board_slots_section(
                px,
                fonts,
                x,
                y,
                w,
                "Header",
                InfoBar::RelHead,
                cfg.relative.rel_head,
                open_drop,
                hover,
                hits,
            );
            y = board_slots_section(
                px,
                fonts,
                x,
                y,
                w,
                "Footer",
                InfoBar::RelFoot,
                cfg.relative.rel_foot,
                open_drop,
                hover,
                hits,
            );
            y = section(
                px,
                fonts,
                x,
                y,
                "Columns  ·  drag to reorder · slide width · toggle to show",
            );
            paint_col_field_rows(
                px,
                fonts,
                cfg,
                &cfg.relative.rel_order,
                DragKind::Rel,
                drag,
                hover,
                x,
                y,
                w,
                hits,
                |f| f as i32,
                |f| f.label(),
                |f, c| f.enabled(c),
                |f, c| f.width(c),
                |f| f.width_max(),
                Hit::RelDrag,
                rel_toggle,
                Hit::RelW,
            )
        },
    )
}

pub(crate) fn pane_map(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    open_drop: Option<Drop>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let spec = widget_pane_spec(WidgetId::Map);
    open_widget_pane(
        px,
        fonts,
        cfg,
        hover,
        hits,
        x,
        y,
        w,
        spec,
        |px, fonts, y, shown, hits| {
            if !shown {
                return y;
            }
            let mut y = pane_style(px, fonts, spec, cfg, hover, hits, x, y, w);
            y = section(px, fonts, x, y, "On the map");
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Follow me",
                cfg.map.map_follow,
                Hit::MapFollow,
                hover,
                hits,
            );
            if cfg.map.map_follow {
                y = slider_row(
                    px,
                    fonts,
                    x,
                    y,
                    w,
                    "Zoom",
                    cfg.map.map_zoom,
                    0,
                    100,
                    "%",
                    Hit::MapZoom,
                    hover,
                    hits,
                );
            }
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Other riders",
                cfg.map.map_others,
                Hit::MapOthers,
                hover,
                hits,
            );
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Start / finish",
                cfg.map.map_sf,
                Hit::MapSf,
                hover,
                hits,
            );
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Sector lines",
                cfg.map.map_sectors,
                Hit::MapSectors,
                hover,
                hits,
            );
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Track arrows",
                cfg.map.map_arrows,
                Hit::MapArrows,
                hover,
                hits,
            );
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Leader crown",
                cfg.map.map_crown,
                Hit::MapCrown,
                hover,
                hits,
            );
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Nearest ahead / behind",
                cfg.map.map_place,
                Hit::MapPlace,
                hover,
                hits,
            );
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Numbers in dots",
                cfg.map.map_numbers,
                Hit::MapNumbers,
                hover,
                hits,
            );
            if cfg.map.map_numbers {
                y = dropdown_row(
                    px,
                    fonts,
                    x,
                    y,
                    w,
                    "Dot number",
                    cfg.map.map_dot.label(),
                    open_drop == Some(Drop::MapDot),
                    Hit::MapDotOpen,
                    &[
                        (Hit::MapDotNum, "Number", cfg.map.map_dot == DotLabel::Number),
                        (
                            Hit::MapDotPos,
                            "Position",
                            cfg.map.map_dot == DotLabel::Position,
                        ),
                    ],
                    hover,
                    hits,
                );
                y = dropdown_row(
                    px,
                    fonts,
                    x,
                    y,
                    w,
                    "My number",
                    cfg.map.map_you_text.label(),
                    open_drop == Some(Drop::MapYouText),
                    Hit::MapYouTextOpen,
                    &[
                        (
                            Hit::MapYouTextWhite,
                            "White",
                            cfg.map.map_you_text == TableText::White,
                        ),
                        (
                            Hit::MapYouTextBlack,
                            "Black",
                            cfg.map.map_you_text == TableText::Black,
                        ),
                    ],
                    hover,
                    hits,
                );
            }
            y = slider_row(
                px,
                fonts,
                x,
                y,
                w,
                "Dot opacity",
                cfg.map.map_dot_opacity,
                0,
                100,
                "%",
                Hit::MapDotOpacity,
                hover,
                hits,
            );
            y
        },
    )
}

pub(crate) fn pane_minimap(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    open_drop: Option<Drop>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let spec = widget_pane_spec(WidgetId::Minimap);
    open_widget_pane(
        px,
        fonts,
        cfg,
        hover,
        hits,
        x,
        y,
        w,
        spec,
        |px, fonts, y, shown, hits| {
            if !shown {
                return y;
            }
            let mut y = pane_style(px, fonts, spec, cfg, hover, hits, x, y, w);
            y = section(px, fonts, x, y, "On the minimap");
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Other riders",
                cfg.mini.mini_others,
                Hit::MiniOthers,
                hover,
                hits,
            );
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Start / finish",
                cfg.mini.mini_sf,
                Hit::MiniSf,
                hover,
                hits,
            );
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Sector lines",
                cfg.mini.mini_sectors,
                Hit::MiniSectors,
                hover,
                hits,
            );
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Track arrows",
                cfg.mini.mini_arrows,
                Hit::MiniArrows,
                hover,
                hits,
            );
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Leader crown",
                cfg.mini.mini_crown,
                Hit::MiniCrown,
                hover,
                hits,
            );
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Nearest ahead / behind",
                cfg.mini.mini_place,
                Hit::MiniPlace,
                hover,
                hits,
            );
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Numbers in dots",
                cfg.mini.mini_numbers,
                Hit::MiniNumbers,
                hover,
                hits,
            );
            if cfg.mini.mini_numbers {
                y = dropdown_row(
                    px,
                    fonts,
                    x,
                    y,
                    w,
                    "Dot number",
                    cfg.mini.mini_dot.label(),
                    open_drop == Some(Drop::MiniDot),
                    Hit::MiniDotOpen,
                    &[
                        (Hit::MiniDotNum, "Number", cfg.mini.mini_dot == DotLabel::Number),
                        (
                            Hit::MiniDotPos,
                            "Position",
                            cfg.mini.mini_dot == DotLabel::Position,
                        ),
                    ],
                    hover,
                    hits,
                );
            }
            slider_row(
                px,
                fonts,
                x,
                y,
                w,
                "Zoom",
                cfg.mini.mini_zoom,
                0,
                100,
                "%",
                Hit::MiniZoom,
                hover,
                hits,
            )
        },
    )
}

pub(crate) fn pane_radar(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    open_drop: Option<Drop>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let spec = widget_pane_spec(WidgetId::Radar);
    open_widget_pane(
        px,
        fonts,
        cfg,
        hover,
        hits,
        x,
        y,
        w,
        spec,
        |px, fonts, y, shown, hits| {
            if !shown {
                return y;
            }
            let mut y = pane_style(px, fonts, spec, cfg, hover, hits, x, y, w);
            y = dropdown_row(
                px,
                fonts,
                x,
                y,
                w,
                "Look",
                cfg.radar.radar_style.label(),
                open_drop == Some(Drop::RadarStyle),
                Hit::RadarStyleOpen,
                &[
                    (
                        Hit::RadarStylePick(RadarStyle::Plaque),
                        "Plaque",
                        cfg.radar.radar_style == RadarStyle::Plaque,
                    ),
                    (
                        Hit::RadarStylePick(RadarStyle::Arrows),
                        "Arrows",
                        cfg.radar.radar_style == RadarStyle::Arrows,
                    ),
                ],
                hover,
                hits,
            );
            y = section(px, fonts, x, y, "On the radar");
            y = slider_row(
                px,
                fonts,
                x,
                y,
                w,
                "Range",
                cfg.radar.radar_range,
                RADAR_RANGE_MIN,
                RADAR_RANGE_MAX,
                "m",
                Hit::RadarRange,
                hover,
                hits,
            );
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Side proximity",
                cfg.radar.radar_sides,
                Hit::RadarSides,
                hover,
                hits,
            );
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Rear proximity",
                cfg.radar.radar_rear,
                Hit::RadarRear,
                hover,
                hits,
            );
            if cfg.radar.radar_style == RadarStyle::Plaque {
                y = toggle_row(
                    px,
                    fonts,
                    x,
                    y,
                    w,
                    "Range rings",
                    cfg.radar.radar_rings,
                    Hit::RadarRings,
                    hover,
                    hits,
                );
            }
            y
        },
    )
}

pub(crate) fn pane_dash(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    open_drop: Option<Drop>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let spec = widget_pane_spec(WidgetId::Dash);
    open_widget_pane(
        px,
        fonts,
        cfg,
        hover,
        hits,
        x,
        y,
        w,
        spec,
        |px, fonts, y, shown, hits| {
            if !shown {
                return y;
            }
            let mut y = pane_style(px, fonts, spec, cfg, hover, hits, x, y, w);
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Simple dash",
                cfg.dash.dash_simple,
                Hit::DashSimple,
                hover,
                hits,
            );
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Shift color",
                cfg.dash.dash_shift_color,
                Hit::DashShiftColor,
                hover,
                hits,
            );
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Yellow flag",
                cfg.dash.dash_yellow,
                Hit::DashYellow,
                hover,
                hits,
            );
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Blue flag",
                cfg.dash.dash_blue,
                Hit::DashBlue,
                hover,
                hits,
            );
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Red flag",
                cfg.dash.dash_red,
                Hit::DashRed,
                hover,
                hits,
            );
            if !cfg.dash.dash_simple {
                y = toggle_row(
                    px,
                    fonts,
                    x,
                    y,
                    w,
                    "Rev indicator",
                    cfg.dash.dash_rev,
                    Hit::DashRev,
                    hover,
                    hits,
                );
                y = slots_section(
                    px,
                    fonts,
                    x,
                    y,
                    w,
                    "Footer  ·  3 slots",
                    [
                        dash_slot(0, cfg.dash.dash_left, open_drop),
                        dash_slot(1, cfg.dash.dash_mid, open_drop),
                        dash_slot(2, cfg.dash.dash_right, open_drop),
                    ],
                    hover,
                    hits,
                );
            }
            y
        },
    )
}

pub(crate) fn pane_ticker(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    open_drop: Option<Drop>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let spec = widget_pane_spec(WidgetId::Ticker);
    open_widget_pane(
        px,
        fonts,
        cfg,
        hover,
        hits,
        x,
        y,
        w,
        spec,
        |px, fonts, y, shown, hits| {
            if !shown {
                return y;
            }
            let mut y = pane_style(px, fonts, spec, cfg, hover, hits, x, y, w);
            y = slider_row(
                px,
                fonts,
                x,
                y,
                w,
                "Row highlight",
                cfg.ticker.ticker_hl,
                0,
                100,
                "%",
                Hit::TickerHl,
                hover,
                hits,
            );
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Track name",
                cfg.ticker.ticker_title,
                Hit::TickerTitle,
                hover,
                hits,
            );
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Autoscroll",
                cfg.ticker.ticker_autoscroll,
                Hit::TickerAutoscroll,
                hover,
                hits,
            );
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Slide on pass",
                cfg.ticker.ticker_slide,
                Hit::TickerSlide,
                hover,
                hits,
            );
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Status",
                cfg.ticker.ticker_status,
                Hit::TickerStatus,
                hover,
                hits,
            );
            y = section(px, fonts, x, y, "Side info");
            y = ticker_field_row(
                px,
                fonts,
                x,
                y,
                w,
                "Left",
                cfg.ticker.ticker_left,
                0,
                open_drop,
                hover,
                hits,
            );
            y = ticker_field_row(
                px,
                fonts,
                x,
                y,
                w,
                "Right",
                cfg.ticker.ticker_right,
                1,
                open_drop,
                hover,
                hits,
            );
            stepper_row(
                px,
                fonts,
                x,
                y,
                w,
                "Riders shown",
                &cfg.ticker.ticker_count.to_string(),
                Hit::TickerDec,
                Hit::TickerInc,
                hover,
                hits,
            )
        },
    )
}

pub(crate) fn pane_sys(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    open_drop: Option<Drop>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let spec = widget_pane_spec(WidgetId::Sys);
    open_widget_pane(
        px,
        fonts,
        cfg,
        hover,
        hits,
        x,
        y,
        w,
        spec,
        |px, fonts, y, shown, hits| {
            if !shown {
                return y;
            }
            let mut y = pane_style(px, fonts, spec, cfg, hover, hits, x, y, w);
            y = note_lines(
            px,
            fonts,
            x,
            y,
            w,
            &format!("Up to {SYS_PROC_MAX} on the overlay. Matched by .exe name, not install folder. Hide a row with the switch; extras can be removed."),
        );
            for (i, app) in cfg.sys.sys_apps.iter().enumerate() {
                let i = i as u8;
                y = sys_app_row(
                    px,
                    fonts,
                    x,
                    y,
                    w,
                    &app.label,
                    app.show,
                    app.removable(),
                    Hit::SysAppShow(i),
                    Hit::SysAppRemove(i),
                    hover,
                    hits,
                );
            }
            let add: Vec<(Hit, &'static str, bool)> = SYS_PRESETS
                .iter()
                .enumerate()
                .filter(|(_, p)| p.addable() && !cfg.sys.sys_apps.iter().any(|a| a.key == p.key))
                .map(|(i, p)| (Hit::SysAddPick(i as u8), p.label, false))
                .collect();
            if !add.is_empty() {
                y = dropdown_row(
                    px,
                    fonts,
                    x,
                    y,
                    w,
                    "Add app",
                    "Choose",
                    open_drop == Some(Drop::SysAdd),
                    Hit::SysAddOpen,
                    &add,
                    hover,
                    hits,
                );
            }
            action_btn(
                px,
                fonts,
                x,
                y,
                140.0,
                32.0,
                "Browse .exe",
                Hit::SysAppBrowse,
                hover,
                hits,
                false,
            );
            y + 40.0
        },
    )
}

pub(crate) fn pane_sector(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let spec = widget_pane_spec(WidgetId::Sector);
    open_widget_pane(
        px,
        fonts,
        cfg,
        hover,
        hits,
        x,
        y,
        w,
        spec,
        |px, fonts, y, shown, hits| {
            let mut y = note_lines(
            px,
            fonts,
            x,
            y,
            w,
            "The wide cell is the sector you are in. Live sector ticks vs your best at this point; off waits until the split. LAP on the right is the whole lap — running time over full-lap delta while you are out, last lap when the clock is off. The night-ink pill in LAP is the ideal (best S1 + S2 + S3, maybe from different laps). Lap log puts IDEAL / LAST / -2 / … under the strip when the box is tall enough; gold is the fastest of those laps. Same tape as Delta Bar. Saved per track and class (250 vs 450). Session best is this visit only.",
        );
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Live sector",
                cfg.sector.sector_live,
                Hit::SectorLive,
                hover,
                hits,
            );
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Compare to session best",
                cfg.sector.sector_session,
                Hit::SectorSession,
                hover,
                hits,
            );
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Lap log",
                cfg.sector.sector_hist,
                Hit::SectorHist,
                hover,
                hits,
            );
            if cfg.sector.sector_hist {
                y = stepper_row(
                    px,
                    fonts,
                    x,
                    y,
                    w,
                    "Laps back",
                    &cfg.sector_hist_count().to_string(),
                    Hit::SectorHistDec,
                    Hit::SectorHistInc,
                    hover,
                    hits,
                );
            }
            action_btn(
                px,
                fonts,
                x,
                y,
                168.0,
                32.0,
                "Clear this track",
                Hit::TrackPbClear,
                hover,
                hits,
                false,
            );
            y += 40.0;
            if !shown {
                return y;
            }
            pane_style(px, fonts, spec, cfg, hover, hits, x, y, w)
        },
    )
}

pub(crate) fn pane_delta(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let spec = widget_pane_spec(WidgetId::Delta);
    open_widget_pane(
        px,
        fonts,
        cfg,
        hover,
        hits,
        x,
        y,
        w,
        spec,
        |px, fonts, y, shown, hits| {
            let mut y = note_lines(
            px,
            fonts,
            x,
            y,
            w,
            "Compares this lap to a lap we recorded — not the in-game ghost. Saved per track and class (250 vs 450). REC while the first flying lap fills if none is saved. Session best is this visit only.",
        );
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Compare to session best",
                cfg.delta.delta_session,
                Hit::DeltaSession,
                hover,
                hits,
            );
            action_btn(
                px,
                fonts,
                x,
                y,
                168.0,
                32.0,
                "Clear this track",
                Hit::TrackPbClear,
                hover,
                hits,
                false,
            );
            y += 40.0;
            if !shown {
                return y;
            }
            pane_style(px, fonts, spec, cfg, hover, hits, x, y, w)
        },
    )
}

pub(crate) fn pane_stance(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    open_drop: Option<Drop>,
    bind_listen: bool,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let spec = widget_pane_spec(WidgetId::Stance);
    open_widget_pane(
        px,
        fonts,
        cfg,
        hover,
        hits,
        x,
        y,
        w,
        spec,
        |px, fonts, y, shown, hits| {
            let mut y = note_lines(
            px,
            fonts,
            x,
            y,
            w,
            "Not connected to MX Bikes. Sit and stand only follow the pad, key, or mouse you bind here — not rider animation.",
        );
            if !shown {
                return y;
            }
            let now = if crate::render::stance_sitting() {
                "Now: SIT"
            } else {
                "Now: STAND"
            };
            text(px, fonts, now, 13.0, x + 4.0, y + 2.0, text_col(), false);
            y += 22.0;
            let bind_name = cfg.stance_bind.label();
            y = bind_row(
                px,
                fonts,
                x,
                y,
                w,
                "Sit button",
                bind_name.as_ref(),
                bind_listen,
                Hit::StanceBindOpen,
                hover,
                hits,
            );
            y = dropdown_row(
                px,
                fonts,
                x,
                y,
                w,
                "Sit mode",
                cfg.stance.stance_mode.label(),
                open_drop == Some(Drop::StanceMode),
                Hit::StanceModeOpen,
                &[
                    (
                        Hit::StanceModePick(StanceMode::Toggle),
                        "Toggle",
                        cfg.stance.stance_mode == StanceMode::Toggle,
                    ),
                    (
                        Hit::StanceModePick(StanceMode::Hold),
                        "Hold to sit",
                        cfg.stance.stance_mode == StanceMode::Hold,
                    ),
                ],
                hover,
                hits,
            );
            y = dropdown_row(
                px,
                fonts,
                x,
                y,
                w,
                "Look",
                cfg.stance.stance_style.label(),
                open_drop == Some(Drop::StanceStyle),
                Hit::StanceStyleOpen,
                &[
                    (
                        Hit::StanceStylePick(StanceStyle::Text),
                        "Text",
                        cfg.stance.stance_style == StanceStyle::Text,
                    ),
                    (
                        Hit::StanceStylePick(StanceStyle::Icon),
                        "Icon",
                        cfg.stance.stance_style == StanceStyle::Icon,
                    ),
                ],
                hover,
                hits,
            );
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Show sitting",
                cfg.stance.stance_show_sit,
                Hit::StanceShowSit,
                hover,
                hits,
            );
            text(
                px,
                fonts,
                "Toggle counts presses. Crash or reset can desync — use Reset to standing.",
                11.0,
                x + 4.0,
                y + 2.0,
                dim(),
                false,
            );
            y += 24.0;
            action_btn(
                px,
                fonts,
                x,
                y,
                168.0,
                32.0,
                "Reset to standing",
                Hit::StanceReset,
                hover,
                hits,
                false,
            );
            y += 40.0;
            pane_style(px, fonts, spec, cfg, hover, hits, x, y, w)
        },
    )
}

pub(crate) fn pane_telemetry(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let spec = widget_pane_spec(WidgetId::Telemetry);
    open_widget_pane(
        px,
        fonts,
        cfg,
        hover,
        hits,
        x,
        y,
        w,
        spec,
        |px, fonts, y, shown, hits| {
            let mut y = note_lines(
                px,
                fonts,
                x,
                y,
                w,
                "Spectate has throttle and front brake only. Clutch, rear, and steer stay off.",
            );
            if !shown {
                return y;
            }
            y = section(px, fonts, x, y, "Show");
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Traces",
                cfg.telemetry.telemetry_traces,
                Hit::TelemetryTraces,
                hover,
                hits,
            );
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Bars",
                cfg.telemetry.telemetry_bars,
                Hit::TelemetryBars,
                hover,
                hits,
            );
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Gear / speed",
                cfg.telemetry.telemetry_dial,
                Hit::TelemetryDial,
                hover,
                hits,
            );
            if cfg.telemetry.telemetry_traces {
                y = section(px, fonts, x, y, "Traces");
                y = toggle_nested(
                    px,
                    fonts,
                    x,
                    y,
                    w,
                    "Throttle",
                    cfg.telemetry.telemetry_trace_throttle,
                    Hit::TelemetryTraceThrottle,
                    hover,
                    hits,
                );
                y = toggle_nested(
                    px,
                    fonts,
                    x,
                    y,
                    w,
                    "Brake",
                    cfg.telemetry.telemetry_trace_brake,
                    Hit::TelemetryTraceBrake,
                    hover,
                    hits,
                );
                y = toggle_nested(
                    px,
                    fonts,
                    x,
                    y,
                    w,
                    "Steer",
                    cfg.telemetry.telemetry_trace_steer,
                    Hit::TelemetryTraceSteer,
                    hover,
                    hits,
                );
            }
            if cfg.telemetry.telemetry_bars {
                y = section(px, fonts, x, y, "Bars");
                y = toggle_nested(
                    px,
                    fonts,
                    x,
                    y,
                    w,
                    "Clutch",
                    cfg.telemetry.telemetry_bar_clutch,
                    Hit::TelemetryBarClutch,
                    hover,
                    hits,
                );
                y = toggle_nested(
                    px,
                    fonts,
                    x,
                    y,
                    w,
                    "Brake",
                    cfg.telemetry.telemetry_bar_brake,
                    Hit::TelemetryBarBrake,
                    hover,
                    hits,
                );
                y = toggle_nested(
                    px,
                    fonts,
                    x,
                    y,
                    w,
                    "Throttle",
                    cfg.telemetry.telemetry_bar_throttle,
                    Hit::TelemetryBarThrottle,
                    hover,
                    hits,
                );
                y = toggle_nested(
                    px,
                    fonts,
                    x,
                    y,
                    w,
                    "Steer",
                    cfg.telemetry.telemetry_bar_steer,
                    Hit::TelemetryBarSteer,
                    hover,
                    hits,
                );
            }
            pane_style(px, fonts, spec, cfg, hover, hits, x, y, w)
        },
    )
}

fn slot_menu_options(
    slot: u8,
    place: &mxbo_hud::pitboard::PitPlace,
    menu: &[(mxbo_hud::pitboard::PitVar, &'static str)],
) -> (String, Vec<(Hit, &'static str, bool)>) {
    let value = if !place.show {
        "None".to_string()
    } else {
        menu.iter()
            .find(|(var, _)| *var == place.var)
            .map(|(_, label)| (*label).to_string())
            .unwrap_or_else(|| place.var.label().to_string())
    };
    let mut options = vec![(Hit::PitSlotNone(slot), "None", !place.show)];
    options.extend(menu.iter().map(|&(var, label)| {
        (
            Hit::PitSlotPick(slot, var.idx()),
            label,
            place.show && place.var == var,
        )
    }));
    (value, options)
}

fn pit_type_bar(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    title: &str,
    value: &str,
    options: &[(Hit, &'static str, bool)],
    open: bool,
    open_hit: Hit,
    size: i32,
    bold: bool,
    rgb: [u8; 3],
    color_open: bool,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let h = ROW_H;
    let text_hit = ColorPickKind::PitSlot.open();
    let hot = open
        || color_open
        || hover == Some(open_hit)
        || hover == Some(Hit::PitSlotSize)
        || hover == Some(Hit::PitSlotBold)
        || hover == Some(text_hit);
    row_card(px, x, y, w, h, hot);

    let mut cursor = x + 14.0;
    text(px, fonts, title, 13.0, cursor, y + 16.0, text_col(), false);
    cursor += measure(fonts, title, 13.0) + 10.0;

    let drop_w = 128.0;
    let drop_h = 28.0;
    let drop_y = y + 10.0;
    hits.push(HitBox {
        id: open_hit,
        x: cursor,
        y: drop_y,
        w: drop_w,
        h: drop_h,
    });
    outlined(
        px,
        cursor,
        drop_y,
        drop_w,
        drop_h,
        7.0,
        if open || hover == Some(open_hit) {
            chip_hover()
        } else {
            bg()
        },
    );
    text(px, fonts, value, 12.0, cursor + 10.0, drop_y + 6.0, text_col(), false);
    chevron(
        px,
        cursor + drop_w - 14.0,
        drop_y + drop_h * 0.5,
        open,
        muted(),
    );
    if open {
        let options = sorted_drop_options(options);
        let item_h = 28.0;
        let pad = 5.0;
        let content_h = pad * 2.0 + item_h * options.len() as f32;
        DROP_MENUS.with(|menus| {
            menus.borrow_mut().push(PendingDrop {
                mx: cursor,
                my: drop_y + drop_h + 6.0,
                bw: drop_w,
                content_h,
                open_hit,
                options,
            });
        });
    }
    cursor += drop_w + 14.0;

    text(px, fonts, "Size", 13.0, cursor, y + 16.0, text_col(), false);
    cursor += measure(fonts, "Size", 13.0) + 8.0;
    let size_label = size.to_string();
    text(px, fonts, &size_label, 13.0, cursor, y + 16.0, text_col(), false);
    cursor += measure(fonts, &size_label, 13.0) + 10.0;

    let swatch_w = 52.0;
    let text_w = measure(fonts, "Text", 13.0);
    let bold_w = measure(fonts, "Bold", 13.0);
    let switch_w = 38.0;
    let swatch_x = x + w - 14.0 - swatch_w;
    let text_x = swatch_x - 8.0 - text_w;
    let bold_x = text_x - 12.0 - bold_w - 8.0 - switch_w;
    let slider_w = (bold_x - 12.0 - cursor).max(36.0);
    draw_slider(
        px,
        cursor,
        y + 16.0,
        slider_w,
        16.0,
        size,
        10,
        56,
        Hit::PitSlotSize,
        hover,
        hits,
    );
    text(px, fonts, "Bold", 13.0, bold_x, y + 16.0, text_col(), false);
    switch(
        px,
        bold_x + bold_w + 8.0,
        y + 14.0,
        bold,
        Hit::PitSlotBold,
        hover,
        hits,
    );

    text(px, fonts, "Text", 13.0, text_x, y + 16.0, text_col(), false);
    hits.push(HitBox {
        id: text_hit,
        x: swatch_x,
        y: drop_y,
        w: swatch_w,
        h: drop_h,
    });
    outlined(
        px,
        swatch_x,
        drop_y,
        swatch_w,
        drop_h,
        7.0,
        if color_open || hover == Some(text_hit) {
            chip_hover()
        } else {
            bg()
        },
    );
    fill_round(
        px,
        swatch_x + 8.0,
        drop_y + 5.0,
        18.0,
        18.0,
        5.0,
        Color::from_rgba8(rgb[0], rgb[1], rgb[2], 255),
    );
    if let Some(path) = round_path(swatch_x + 8.0, drop_y + 5.0, 18.0, 18.0, 5.0) {
        let mut paint = Paint::default();
        paint.set_color(Color::from_rgba8(255, 255, 255, 40));
        paint.anti_alias = true;
        px.stroke_path(
            &path,
            &paint,
            &Stroke {
                width: 1.0,
                ..Stroke::default()
            },
            Transform::identity(),
            None,
        );
    }
    chevron(
        px,
        swatch_x + swatch_w - 12.0,
        drop_y + drop_h * 0.5,
        color_open,
        muted(),
    );
    if color_open {
        COLOR_PICKER_ANCHOR.with(|anchor| anchor.set(Some((swatch_x, drop_y, swatch_w, drop_h))));
    }
    y + h + ROW_GAP
}

fn pit_action_row(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    buttons: &[(&str, f32, Hit)],
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    if buttons.is_empty() {
        return y;
    }
    let gap = 8.0;
    let h = 32.0;
    let mut cursor_x = x;
    let mut cursor_y = y;
    for &(label, width, hit) in buttons {
        if cursor_x > x && cursor_x + width > x + w {
            cursor_x = x;
            cursor_y += h + gap;
        }
        action_btn(px, fonts, cursor_x, cursor_y, width, h, label, hit, hover, hits, false);
        cursor_x += width + gap;
    }
    cursor_y + h + 8.0
}

pub(crate) fn pane_pitboard(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    open_drop: Option<Drop>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let spec = widget_pane_spec(WidgetId::Pitboard);
    open_widget_pane(px, fonts, cfg, hover, hits, x, y, w, spec, |px, fonts, y, shown, hits| {
        let mut y = y;
        if crate::stock_pitboard::needs_restart() {
            text(
                px,
                fonts,
                crate::stock_pitboard::RESTART_STOCK,
                11.0,
                x + 4.0,
                y + 2.0,
                accent(),
                false,
            );
            y += 20.0;
        }
        if !shown {
            return y;
        }
        y = pit_action_row(
            px,
            fonts,
            x,
            y,
            w,
            &[
                ("Download demo", 148.0, Hit::PitDemo),
                ("Make your own", 148.0, Hit::PitHelpOpen),
            ],
            hover,
            hits,
        );
        y = pit_action_row(
            px,
            fonts,
            x,
            y,
            w,
            &[
                ("Upload Pitboard image", 210.0, Hit::PitBrowse),
                ("Export", 76.0, Hit::PitExport),
                ("Import", 76.0, Hit::PitImport),
            ],
            hover,
            hits,
        );
        seed_pit_name(&cfg.pit.pit_board);
        let factory = cfg.pit.pit_board.is_empty() && (cfg.pit.pit_art.is_empty() || cfg.pit.pit_art == FACTORY_ART);
        if factory && pit_name_focused() {
            set_pit_name_focus(false);
        }
        let boards = mxbo_hud::pitboard::saved_boards();
        let current = if !cfg.pit.pit_board.is_empty() {
            cfg.pit.pit_board.clone()
        } else if cfg.pit.pit_art.is_empty() || cfg.pit.pit_art == FACTORY_ART {
            "Holeshot".to_string()
        } else {
            cfg.pit.pit_art.clone()
        };
        let mut board_options = vec![(
            Hit::PitBoardFactory,
            "Holeshot".to_string(),
            cfg.pit.pit_board.is_empty() && (cfg.pit.pit_art.is_empty() || cfg.pit.pit_art == FACTORY_ART),
        )];
        for (index, name) in boards.iter().take(255).enumerate() {
            board_options.push((
                Hit::PitBoardPick(index as u8),
                name.clone(),
                cfg.pit.pit_board == *name,
            ));
        }
        let saved_board = !cfg.pit.pit_board.is_empty();
        let row_gap = 8.0;
        let when_w = 228.0_f32.min(w * 0.4);
        let board_w = (w - when_w - row_gap).max(160.0);
        let board_y = y;
        let board_bottom = ordered_menu_row(
            px,
            fonts,
            x,
            y,
            board_w,
            "Board",
            &current,
            open_drop == Some(Drop::PitBoard),
            Hit::PitBoardOpen,
            board_options,
            hover,
            hits,
        );
        let when_options = [
            (Hit::PitWhenAlways, PitWhen::Always.label(), cfg.pit.pit_when == PitWhen::Always),
            (Hit::PitWhenSector, PitWhen::Sector.label(), cfg.pit.pit_when == PitWhen::Sector),
            (Hit::PitWhenLap, PitWhen::Lap.label(), cfg.pit.pit_when == PitWhen::Lap),
        ];
        let when_bottom = dropdown_row(
            px,
            fonts,
            x + board_w + row_gap,
            board_y,
            when_w,
            "When",
            cfg.pit.pit_when.label(),
            open_drop == Some(Drop::PitWhen),
            Hit::PitWhenOpen,
            &when_options,
            hover,
            hits,
        );
        y = board_bottom.max(when_bottom);
        if factory {
            y = pit_action_row(
                px,
                fonts,
                x,
                y,
                w,
                &[("Reset layout", 124.0, Hit::PitReset)],
                hover,
                hits,
            );
        }
        let name = if factory {
            "Holeshot".to_string()
        } else {
            pit_name_draft()
        };
        let delete_w = 76.0;
        let name_w = if saved_board {
            (w - delete_w - row_gap).max(160.0)
        } else {
            w
        };
        let name_y = y;
        y = name_row(
            px,
            fonts,
            x,
            y,
            name_w,
            &name,
            !factory && pit_name_focused(),
            Hit::PitName,
            if factory { None } else { hover },
            hits,
        );
        if saved_board {
            action_btn(
                px,
                fonts,
                x + name_w + row_gap,
                name_y + 8.0,
                delete_w,
                32.0,
                "Delete",
                Hit::PitDelete,
                hover,
                hits,
                false,
            );
        }
        let notice = pit_notice();
        if !notice.is_empty() {
            y = note_lines(px, fonts, x, y, w, &notice);
        }

        let slots = if cfg.pit.pit_vars.len() >= LEGACY_CATALOG {
            factory_places()
        } else {
            cfg.pit.pit_vars.clone()
        };
        let selected = if slots.is_empty() {
            0
        } else {
            pit_selected().min(slots.len() as u8 - 1)
        };
        if pit_selected() != selected {
            set_pit_selected(selected);
        }
        let names = pack_slot_names(&cfg.pit.pit_art);
        let menu = slot_menu();
        if let Some(place) = slots.get(selected as usize) {
            let title = names
                .get(selected as usize)
                .cloned()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| place.row_label());
            let (value, options) = slot_menu_options(selected, place, &menu);
            let rgb = place.color.unwrap_or([16, 16, 18]);
            y = pit_type_bar(
                px,
                fonts,
                x,
                y,
                w,
                &title,
                &value,
                &options,
                open_drop == Some(Drop::PitSlot(selected)),
                Hit::PitSlotOpen(selected),
                place.size.round() as i32,
                place.bold,
                rgb,
                open_drop == Some(Drop::PitSlotColor),
                hover,
                hits,
            );
            let cell = 44.0;
            let gap = 6.0;
            let bar_w = (cell * 3.0 + gap * 2.0 - gap) * 0.5;
            snap_axis(
                px,
                x,
                y,
                bar_w,
                cell,
                true,
                Hit::PitSlotCenterX,
                hover,
                hits,
            );
            snap_axis(
                px,
                x + bar_w + gap,
                y,
                bar_w,
                cell,
                false,
                Hit::PitSlotCenterY,
                hover,
                hits,
            );
            y += cell + 8.0;
        }
        y = pit_plate_preview(px, fonts, cfg, &slots, selected, hits, x, y, w);
        action_btn(px, fonts, x, y, 132.0, 32.0, "Add slot", Hit::PitSlotAdd, hover, hits, false);
        action_btn(px, fonts, x + 144.0, y, 148.0, 32.0, "Remove slot", Hit::PitSlotRemove, hover, hits, false);
        y += 40.0;
        if slots.len() >= MAX_DESIGN_SLOTS {
            y = note_lines(px, fonts, x, y, w, "12 slots is the limit.");
        }
        if cfg.pit.pit_art.is_empty() || cfg.pit.pit_art == FACTORY_ART {
            y = color_row(
                px,
                fonts,
                x,
                y,
                w,
                cfg.pit.pit_yellow,
                open_drop == Some(Drop::PitYellow),
                ColorPickKind::PitYellow,
                hover,
                hits,
            );
            y = color_row(
                px,
                fonts,
                x,
                y,
                w,
                cfg.pit.pit_blue,
                open_drop == Some(Drop::PitBlue),
                ColorPickKind::PitBlue,
                hover,
                hits,
            );
        }
        let col_gap = 8.0;
        let col_w = ((w - col_gap) * 0.5).max(160.0);
        let mut left_y = y;
        let mut right_y = y;
        for (slot, place) in slots.iter().enumerate() {
            let title = names
                .get(slot)
                .cloned()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| place.row_label());
            let stat = slot_menu_options(slot as u8, place, &menu).0;
            let on_right = slot % 2 == 1;
            let col_x = if on_right { x + col_w + col_gap } else { x };
            let col_y = if on_right { right_y } else { left_y };
            let next = slot_readout_row(
                px,
                fonts,
                col_x,
                col_y,
                col_w,
                &title,
                &stat,
                slot as u8,
                slot as u8 == selected,
                hover,
                hits,
            );
            if on_right {
                right_y = next;
            } else {
                left_y = next;
            }
        }
        y = left_y.max(right_y);
        y
    })
}

fn slot_readout_row(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    title: &str,
    value: &str,
    slot: u8,
    selected: bool,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let h = ROW_H;
    let hit = Hit::PitSlotSelect(slot);
    hits.push(HitBox {
        id: hit,
        x,
        y,
        w,
        h,
    });
    row_card(px, x, y, w, h, selected || hover == Some(hit));
    text(px, fonts, title, 13.0, x + 16.0, y + 16.0, text_col(), false);
    if !value.is_empty() {
        let value_w = measure(fonts, value, 13.0);
        text(
            px,
            fonts,
            value,
            13.0,
            x + w - 16.0 - value_w,
            y + 16.0,
            text_col(),
            false,
        );
    }
    y + h + ROW_GAP
}

fn pit_plate_preview(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    places: &[mxbo_hud::pitboard::PitPlace],
    selected: u8,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let max_h = 220.0;
    let size = plate_pixel_size(cfg);
    let (pw, ph, scale) = match size {
        Some((iw, ih)) if iw > 0 && ih > 0 => {
            let iw = iw as f32;
            let ih = ih as f32;
            let mut scale = w / iw;
            let mut ph = ih * scale;
            if ph > max_h {
                scale = max_h / ih;
                ph = max_h;
            }
            ((iw * scale).max(1.0), ph.max(1.0), scale)
        }
        _ => (w, 120.0, 1.0),
    };
    let left = x + ((w - pw) * 0.5).max(0.0);
    fill_round(px, left, y, pw, ph, 8.0, panel());
    paint_plate(px, cfg, scale, left, y);
    set_pit_preview(Some((left, y, pw, ph)));
    for (index, place) in places.iter().enumerate() {
        let i = index as u8;
        let sample = slot_sample(place);
        let fs = place.size.clamp(9.0, 56.0);
        let cx = left + place.x * pw;
        let cy = y + place.y * ph - fs * 0.55;
        let tw = measure(fonts, sample, fs).max(28.0);
        let th = fs + 10.0;
        let hx = cx - tw * 0.5 - 6.0;
        let hy = cy - 4.0;
        let hw = tw + 12.0;
        if i == selected {
            fill_round(px, hx, hy, hw, 2.0, 1.0, accent());
            fill_round(px, hx, hy + th - 2.0, hw, 2.0, 1.0, accent());
            fill_round(px, hx, hy, 2.0, th, 1.0, accent());
            fill_round(px, hx + hw - 2.0, hy, 2.0, th, 1.0, accent());
        }
        let rgb = mxbo_hud::pitboard::place_ink(place.color);
        let ink = Color::from_rgba8(rgb[0], rgb[1], rgb[2], 255);
        if !sample.is_empty() {
            if place.bold {
                text_bold(px, fonts, sample, fs, cx, cy, ink, true);
            } else {
                text(px, fonts, sample, fs, cx, cy, ink, true);
            }
        }
        hits.push(HitBox {
            id: Hit::PitSlotGrab(i),
            x: hx,
            y: hy,
            w: hw,
            h: th,
        });
    }
    y + ph + 12.0
}

fn plate_pixel_size(cfg: &HudConfig) -> Option<(u32, u32)> {
    with_plate(cfg, |img| img.map(|img| (img.width(), img.height())))
}

fn paint_plate(px: &mut Pixmap, cfg: &HudConfig, scale: f32, x: f32, y: f32) {
    with_plate(cfg, |img| {
        let Some(img) = img else {
            return;
        };
        let mut paint = tiny_skia::PixmapPaint::default();
        paint.quality = tiny_skia::FilterQuality::Bilinear;
        let _ = px.draw_pixmap(
            0,
            0,
            img.as_ref(),
            &paint,
            tiny_skia::Transform::from_scale(scale, scale).post_translate(x, y),
            None,
        );
    });
}

fn with_plate<T>(cfg: &HudConfig, f: impl FnOnce(Option<&tiny_skia::Pixmap>) -> T) -> T {
    use std::cell::RefCell;
    thread_local! {
        static CACHE: RefCell<(u32, String, [u8; 3], [u8; 3], Option<tiny_skia::Pixmap>)> =
            RefCell::new((u32::MAX, String::new(), [0, 0, 0], [0, 0, 0], None));
    }
    let art = cfg.pit.pit_art.as_str();
    let factory = art.is_empty() || art == FACTORY_ART;
    let main = if factory { cfg.pit.pit_yellow } else { [0, 0, 0] };
    let secondary = if factory { cfg.pit.pit_blue } else { [0, 0, 0] };
    let gen = plate_preview_gen();
    CACHE.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.0 != gen || slot.1 != art || slot.2 != main || slot.3 != secondary {
            let decoded = if factory {
                mxbo_hud::render::painted_factory_plate(art, cfg.pit.pit_yellow, cfg.pit.pit_blue)
            } else {
                mxbo_hud::pitboard::art_bytes(art)
                    .and_then(|bytes| tiny_skia::Pixmap::decode_png(&bytes).ok())
            };
            *slot = (gen, art.to_string(), main, secondary, decoded);
        }
        f(slot.4.as_ref())
    })
}

pub(crate) fn pane_lean(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    open_drop: Option<Drop>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let spec = widget_pane_spec(WidgetId::Lean);
    open_widget_pane(
        px,
        fonts,
        cfg,
        hover,
        hits,
        x,
        y,
        w,
        spec,
        |px, fonts, y, shown, hits| {
            let mut y = note_lines(
            px,
            fonts,
            x,
            y,
            w,
            "Bike roll, pitch, and bar angle for the rider the camera is on. Riding uses your telemetry. Spectate follows that rider’s lean; steer and pitch hide because other bikes do not send them. This is not sit / stand.",
        );
            if !shown {
                return y;
            }
            y = dropdown_row(
                px,
                fonts,
                x,
                y,
                w,
                "Look",
                cfg.lean.lean_style.label(),
                open_drop == Some(Drop::LeanStyle),
                Hit::LeanStyleOpen,
                &[
                    (
                        Hit::LeanStylePick(LeanStyle::Figure),
                        "Figure",
                        cfg.lean.lean_style == LeanStyle::Figure,
                    ),
                    (
                        Hit::LeanStylePick(LeanStyle::Minimal),
                        "Minimal",
                        cfg.lean.lean_style == LeanStyle::Minimal,
                    ),
                ],
                hover,
                hits,
            );
            pane_style(px, fonts, spec, cfg, hover, hits, x, y, w)
        },
    )
}

pub(crate) fn pane_gamepad(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
    open_drop: Option<Drop>,
) -> f32 {
    let spec = widget_pane_spec(WidgetId::Gamepad);
    open_widget_pane(
        px,
        fonts,
        cfg,
        hover,
        hits,
        x,
        y,
        w,
        spec,
        |px, fonts, y, shown, hits| {
            let mut y = note_lines(
            px,
            fonts,
            x,
            y,
            w,
            "Your local pad, not plugin telemetry. Auto matches your controller. PlayStation and Xbox force that pad art. Theme picks a Light or Dark pad. Triggers fill with squeeze. Bumpers light when held. No pad shows No controller. Other riders’ inputs are not available.",
        );
            if !shown {
                return y;
            }
            y = dropdown_row(
                px,
                fonts,
                x,
                y,
                w,
                "Pad",
                cfg.gamepad.gamepad_style.label(),
                open_drop == Some(Drop::GamepadStyle),
                Hit::GamepadStyleOpen,
                &[
                    (
                        Hit::GamepadStylePick(GamepadStyle::Auto),
                        "Auto",
                        cfg.gamepad.gamepad_style == GamepadStyle::Auto,
                    ),
                    (
                        Hit::GamepadStylePick(GamepadStyle::PlayStation),
                        "PlayStation",
                        cfg.gamepad.gamepad_style == GamepadStyle::PlayStation,
                    ),
                    (
                        Hit::GamepadStylePick(GamepadStyle::Xbox),
                        "Xbox",
                        cfg.gamepad.gamepad_style == GamepadStyle::Xbox,
                    ),
                ],
                hover,
                hits,
            );
            y = dropdown_row(
                px,
                fonts,
                x,
                y,
                w,
                "Theme",
                cfg.gamepad.gamepad_theme.label(),
                open_drop == Some(Drop::GamepadTheme),
                Hit::GamepadThemeOpen,
                &[
                    (
                        Hit::GamepadThemePick(GamepadTheme::Light),
                        "Light",
                        cfg.gamepad.gamepad_theme == GamepadTheme::Light,
                    ),
                    (
                        Hit::GamepadThemePick(GamepadTheme::Dark),
                        "Dark",
                        cfg.gamepad.gamepad_theme == GamepadTheme::Dark,
                    ),
                ],
                hover,
                hits,
            );
            pane_style(px, fonts, spec, cfg, hover, hits, x, y, w)
        },
    )
}

pub(crate) fn pane_flag(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let spec = widget_pane_spec(WidgetId::Flag);
    open_widget_pane(
        px,
        fonts,
        cfg,
        hover,
        hits,
        x,
        y,
        w,
        spec,
        |px, fonts, y, shown, hits| {
            if !shown {
                return y;
            }
            let mut y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Text",
                cfg.flag.flag_text,
                Hit::FlagText,
                hover,
                hits,
            );
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Yellow flag",
                cfg.flag.flag_yellow,
                Hit::FlagYellow,
                hover,
                hits,
            );
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Blue flag",
                cfg.flag.flag_blue,
                Hit::FlagBlue,
                hover,
                hits,
            );
            y = toggle_row(
                px,
                fonts,
                x,
                y,
                w,
                "Red flag",
                cfg.flag.flag_red,
                Hit::FlagRed,
                hover,
                hits,
            );
            pane_style(px, fonts, spec, cfg, hover, hits, x, y, w)
        },
    )
}

pub(crate) fn ticker_field_row(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    value: BoardField,
    slot: u8,
    open_drop: Option<Drop>,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let options: Vec<(Hit, &'static str, bool)> = BoardField::ALL
        .iter()
        .map(|&field| {
            (
                Hit::TickerFootPick(slot, field),
                field.label(),
                field == value,
            )
        })
        .collect();
    dropdown_row(
        px,
        fonts,
        x,
        y,
        w,
        label,
        value.label(),
        open_drop == Some(Drop::TickerFoot(slot)),
        Hit::TickerFootOpen(slot),
        &options,
        hover,
        hits,
    )
}

pub(crate) struct SlotDrop {
    pub(crate) open_hit: Hit,
    pub(crate) value: &'static str,
    pub(crate) open: bool,
    pub(crate) options: Vec<(Hit, &'static str, bool)>,
}

pub(crate) fn dash_slot(slot: u8, value: DashField, open_drop: Option<Drop>) -> SlotDrop {
    SlotDrop {
        open_hit: Hit::DashFootOpen(slot),
        value: value.label(),
        open: open_drop == Some(Drop::DashFoot(slot)),
        options: DashField::ALL
            .iter()
            .map(|&field| {
                (
                    Hit::DashFootPick(slot, field),
                    field.label(),
                    field == value,
                )
            })
            .collect(),
    }
}

pub(crate) fn set_info_slot(c: &mut HudConfig, bar: InfoBar, slot: u8, field: BoardField) {
    let slots = match bar {
        InfoBar::StHead => &mut c.standings.st_head,
        InfoBar::StFoot => &mut c.standings.st_foot,
        InfoBar::RelHead => &mut c.relative.rel_head,
        InfoBar::RelFoot => &mut c.relative.rel_foot,
    };
    if let Some(dst) = slots.get_mut(slot as usize) {
        *dst = field;
    }
}

pub(crate) fn board_slots_section(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    title: &str,
    bar: InfoBar,
    values: [BoardField; 3],
    open_drop: Option<Drop>,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    slots_section(
        px,
        fonts,
        x,
        y,
        w,
        &format!("{title}  ·  3 slots"),
        [0, 1, 2].map(|slot| {
            let value = values[slot];
            SlotDrop {
                open_hit: Hit::InfoOpen(bar, slot as u8),
                value: value.label(),
                open: open_drop == Some(Drop::Info(bar, slot as u8)),
                options: BoardField::ALL
                    .iter()
                    .map(|&field| {
                        (
                            Hit::InfoPick(bar, slot as u8, field),
                            field.label(),
                            field == value,
                        )
                    })
                    .collect(),
            }
        }),
        hover,
        hits,
    )
}
