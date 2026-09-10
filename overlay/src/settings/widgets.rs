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
        Some((cfg[spec.id].show, spec.show)),
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
    open_widget_pane(px, fonts, cfg, hover, hits, x, y, w, spec, |px, fonts, y, shown, hits| {
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
            cfg.st_hl,
            Hit::StHl,
            cfg.st_text,
            Drop::StText,
            Hit::StTextOpen,
            Hit::StTextWhite,
            Hit::StTextBlack,
            cfg.st_stripe,
            Hit::StStripe,
        );
        let mut g = PairGrid::new(x, y, w);
        g.place(|cx, cy, cw| {
            stepper_row(px, fonts, cx, cy, cw, "Rows", &cfg.standings_rows.to_string(), Hit::StDec, Hit::StInc, hover, hits)
        });
        y = g.end();
        y = board_slots_section(px, fonts, x, y, w, "Header", InfoBar::StHead, cfg.st_head, open_drop, hover, hits);
        y = board_slots_section(px, fonts, x, y, w, "Footer", InfoBar::StFoot, cfg.st_foot, open_drop, hover, hits);
        y = section(px, fonts, x, y, "Columns  ·  drag to reorder · slide width · toggle to show");
        for (i, field) in cfg.st_order.iter().enumerate() {
            y = field_row(
                px,
                fonts,
                x,
                y,
                w,
                field.label(),
                field.enabled(cfg),
                field.width(cfg),
                Hit::StDrag(i as u8),
                st_toggle(*field),
                Hit::StW(i as u8),
                field.width_max(),
                i,
                hover,
                drag.filter(|d| d.kind == DragKind::St),
                hits,
            );
        }
        y
    })
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
    open_widget_pane(px, fonts, cfg, hover, hits, x, y, w, spec, |px, fonts, y, shown, hits| {
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
            cfg.rel_hl,
            Hit::RelHl,
            cfg.rel_text,
            Drop::RelText,
            Hit::RelTextOpen,
            Hit::RelTextWhite,
            Hit::RelTextBlack,
            cfg.rel_stripe,
            Hit::RelStripe,
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
                &cfg.relative_count.to_string(),
                Hit::RelDec,
                Hit::RelInc,
                hover,
                hits,
            )
        });
        y = g.end();
        y = board_slots_section(px, fonts, x, y, w, "Header", InfoBar::RelHead, cfg.rel_head, open_drop, hover, hits);
        y = board_slots_section(px, fonts, x, y, w, "Footer", InfoBar::RelFoot, cfg.rel_foot, open_drop, hover, hits);
        y = section(px, fonts, x, y, "Columns  ·  drag to reorder · slide width · toggle to show");
        for (i, field) in cfg.rel_order.iter().enumerate() {
            y = field_row(
                px,
                fonts,
                x,
                y,
                w,
                field.label(),
                field.enabled(cfg),
                field.width(cfg),
                Hit::RelDrag(i as u8),
                rel_toggle(*field),
                Hit::RelW(i as u8),
                field.width_max(),
                i,
                hover,
                drag.filter(|d| d.kind == DragKind::Rel),
                hits,
            );
        }
        y
    })
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
    open_widget_pane(px, fonts, cfg, hover, hits, x, y, w, spec, |px, fonts, y, shown, hits| {
        if !shown {
            return y;
        }
        let mut y = pane_style(px, fonts, spec, cfg, hover, hits, x, y, w);
        y = section(px, fonts, x, y, "On the map");
        y = toggle_row(px, fonts, x, y, w, "Other riders", cfg.map_others, Hit::MapOthers, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Start / finish", cfg.map_sf, Hit::MapSf, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Sector lines", cfg.map_sectors, Hit::MapSectors, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Track arrows", cfg.map_arrows, Hit::MapArrows, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Leader crown", cfg.map_crown, Hit::MapCrown, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Nearest ahead / behind", cfg.map_place, Hit::MapPlace, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Numbers in dots", cfg.map_numbers, Hit::MapNumbers, hover, hits);
        dropdown_row(
            px,
            fonts,
            x,
            y,
            w,
            "Dot number",
            cfg.map_dot.label(),
            open_drop == Some(Drop::MapDot),
            Hit::MapDotOpen,
            &[
                (Hit::MapDotNum, "Number", cfg.map_dot == DotLabel::Number),
                (Hit::MapDotPos, "Position", cfg.map_dot == DotLabel::Position),
            ],
            hover,
            hits,
        )
    })
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
    open_widget_pane(px, fonts, cfg, hover, hits, x, y, w, spec, |px, fonts, y, shown, hits| {
        if !shown {
            return y;
        }
        let mut y = pane_style(px, fonts, spec, cfg, hover, hits, x, y, w);
        y = section(px, fonts, x, y, "On the minimap");
        y = toggle_row(px, fonts, x, y, w, "Other riders", cfg.mini_others, Hit::MiniOthers, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Start / finish", cfg.mini_sf, Hit::MiniSf, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Sector lines", cfg.mini_sectors, Hit::MiniSectors, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Track arrows", cfg.mini_arrows, Hit::MiniArrows, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Leader crown", cfg.mini_crown, Hit::MiniCrown, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Nearest ahead / behind", cfg.mini_place, Hit::MiniPlace, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Numbers in dots", cfg.mini_numbers, Hit::MiniNumbers, hover, hits);
        y = dropdown_row(
            px,
            fonts,
            x,
            y,
            w,
            "Dot number",
            cfg.mini_dot.label(),
            open_drop == Some(Drop::MiniDot),
            Hit::MiniDotOpen,
            &[
                (Hit::MiniDotNum, "Number", cfg.mini_dot == DotLabel::Number),
                (Hit::MiniDotPos, "Position", cfg.mini_dot == DotLabel::Position),
            ],
            hover,
            hits,
        );
        slider_row(px, fonts, x, y, w, "Zoom", cfg.mini_zoom, 0, 100, "%", Hit::MiniZoom, hover, hits)
    })
}

pub(crate) fn pane_radar(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let spec = widget_pane_spec(WidgetId::Radar);
    open_widget_pane(px, fonts, cfg, hover, hits, x, y, w, spec, |px, fonts, y, shown, hits| {
        if !shown {
            return y;
        }
        let mut y = pane_style(px, fonts, spec, cfg, hover, hits, x, y, w);
        y = section(px, fonts, x, y, "On the radar");
        y = slider_row(px, fonts, x, y, w, "Range", cfg.radar_range, RADAR_RANGE_MIN, RADAR_RANGE_MAX, "m", Hit::RadarRange, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Side proximity", cfg.radar_sides, Hit::RadarSides, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Rear proximity", cfg.radar_rear, Hit::RadarRear, hover, hits);
        toggle_row(px, fonts, x, y, w, "Range rings", cfg.radar_rings, Hit::RadarRings, hover, hits)
    })
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
    open_widget_pane(px, fonts, cfg, hover, hits, x, y, w, spec, |px, fonts, y, shown, hits| {
        if !shown {
            return y;
        }
        let mut y = pane_style(px, fonts, spec, cfg, hover, hits, x, y, w);
        y = toggle_row(px, fonts, x, y, w, "Simple dash", cfg.dash_simple, Hit::DashSimple, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Shift color", cfg.dash_shift_color, Hit::DashShiftColor, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Yellow flag", cfg.dash_yellow, Hit::DashYellow, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Blue flag", cfg.dash_blue, Hit::DashBlue, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Red flag", cfg.dash_red, Hit::DashRed, hover, hits);
        if !cfg.dash_simple {
            y = toggle_row(px, fonts, x, y, w, "Rev indicator", cfg.dash_rev, Hit::DashRev, hover, hits);
            y = slots_section(
                px,
                fonts,
                x,
                y,
                w,
                "Footer  ·  3 slots",
                [
                    dash_slot(0, cfg.dash_left, open_drop),
                    dash_slot(1, cfg.dash_mid, open_drop),
                    dash_slot(2, cfg.dash_right, open_drop),
                ],
                hover,
                hits,
            );
        }
        y
    })
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
    open_widget_pane(px, fonts, cfg, hover, hits, x, y, w, spec, |px, fonts, y, shown, hits| {
        if !shown {
            return y;
        }
        let mut y = pane_style(px, fonts, spec, cfg, hover, hits, x, y, w);
        y = toggle_row(px, fonts, x, y, w, "Track name", cfg.ticker_title, Hit::TickerTitle, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Autoscroll", cfg.ticker_autoscroll, Hit::TickerAutoscroll, hover, hits);
        y = section(px, fonts, x, y, "Side info");
        y = ticker_field_row(px, fonts, x, y, w, "Left", cfg.ticker_left, 0, open_drop, hover, hits);
        y = ticker_field_row(px, fonts, x, y, w, "Right", cfg.ticker_right, 1, open_drop, hover, hits);
        stepper_row(px, fonts, x, y, w, "Riders shown", &cfg.ticker_count.to_string(), Hit::TickerDec, Hit::TickerInc, hover, hits)
    })
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
    open_widget_pane(px, fonts, cfg, hover, hits, x, y, w, spec, |px, fonts, y, shown, hits| {
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
        for (i, app) in cfg.sys_apps.iter().enumerate() {
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
            .filter(|(_, p)| p.addable() && !cfg.sys_apps.iter().any(|a| a.key == p.key))
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
        action_btn(px, fonts, x, y, 140.0, 32.0, "Browse .exe", Hit::SysAppBrowse, hover, hits, false);
        y + 40.0
    })
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
    open_widget_pane(px, fonts, cfg, hover, hits, x, y, w, spec, |px, fonts, y, shown, hits| {
        let mut y = note_lines(
            px,
            fonts,
            x,
            y,
            w,
            "The wide cell is the sector you are in. Live sector ticks vs your best at this point; off waits until the split. LAP on the right is the whole lap — running time over full-lap delta while you are out, last lap when the clock is off. The night-ink pill in LAP is the ideal (best S1 + S2 + S3, maybe from different laps). Lap log puts IDEAL / LAST / -2 / … under the strip when the box is tall enough; gold is the fastest of those laps. Same tape as Delta Bar. Saved per track and class (250 vs 450). Session best is this visit only.",
        );
        y = toggle_row(px, fonts, x, y, w, "Live sector", cfg.sector_live, Hit::SectorLive, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Compare to session best", cfg.sector_session, Hit::SectorSession, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Lap log", cfg.sector_hist, Hit::SectorHist, hover, hits);
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
        action_btn(px, fonts, x, y, 168.0, 32.0, "Clear this track", Hit::TrackPbClear, hover, hits, false);
        y += 40.0;
        if !shown {
            return y;
        }
        pane_style(px, fonts, spec, cfg, hover, hits, x, y, w)
    })
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
    open_widget_pane(px, fonts, cfg, hover, hits, x, y, w, spec, |px, fonts, y, shown, hits| {
        let mut y = note_lines(
            px,
            fonts,
            x,
            y,
            w,
            "Compares this lap to a lap we recorded — not the in-game ghost. Saved per track and class (250 vs 450). REC while the first flying lap fills if none is saved. Session best is this visit only.",
        );
        y = toggle_row(px, fonts, x, y, w, "Compare to session best", cfg.delta_session, Hit::DeltaSession, hover, hits);
        action_btn(px, fonts, x, y, 168.0, 32.0, "Clear this track", Hit::TrackPbClear, hover, hits, false);
        y += 40.0;
        if !shown {
            return y;
        }
        pane_style(px, fonts, spec, cfg, hover, hits, x, y, w)
    })
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
    open_widget_pane(px, fonts, cfg, hover, hits, x, y, w, spec, |px, fonts, y, shown, hits| {
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
            cfg.stance_mode.label(),
            open_drop == Some(Drop::StanceMode),
            Hit::StanceModeOpen,
            &[
                (Hit::StanceModePick(StanceMode::Toggle), "Toggle", cfg.stance_mode == StanceMode::Toggle),
                (Hit::StanceModePick(StanceMode::Hold), "Hold to sit", cfg.stance_mode == StanceMode::Hold),
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
            cfg.stance_style.label(),
            open_drop == Some(Drop::StanceStyle),
            Hit::StanceStyleOpen,
            &[
                (Hit::StanceStylePick(StanceStyle::Text), "Text", cfg.stance_style == StanceStyle::Text),
                (Hit::StanceStylePick(StanceStyle::Icon), "Icon", cfg.stance_style == StanceStyle::Icon),
            ],
            hover,
            hits,
        );
        y = toggle_row(px, fonts, x, y, w, "Show sitting", cfg.stance_show_sit, Hit::StanceShowSit, hover, hits);
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
        action_btn(px, fonts, x, y, 168.0, 32.0, "Reset to standing", Hit::StanceReset, hover, hits, false);
        y += 40.0;
        pane_style(px, fonts, spec, cfg, hover, hits, x, y, w)
    })
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
    open_widget_pane(px, fonts, cfg, hover, hits, x, y, w, spec, |px, fonts, y, shown, hits| {
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
        y = toggle_row(px, fonts, x, y, w, "Traces", cfg.telemetry_traces, Hit::TelemetryTraces, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Bars", cfg.telemetry_bars, Hit::TelemetryBars, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Gear / speed", cfg.telemetry_dial, Hit::TelemetryDial, hover, hits);
        if cfg.telemetry_traces {
            y = section(px, fonts, x, y, "Traces");
            y = toggle_nested(px, fonts, x, y, w, "Throttle", cfg.telemetry_trace_throttle, Hit::TelemetryTraceThrottle, hover, hits);
            y = toggle_nested(px, fonts, x, y, w, "Brake", cfg.telemetry_trace_brake, Hit::TelemetryTraceBrake, hover, hits);
            y = toggle_nested(px, fonts, x, y, w, "Steer", cfg.telemetry_trace_steer, Hit::TelemetryTraceSteer, hover, hits);
        }
        if cfg.telemetry_bars {
            y = section(px, fonts, x, y, "Bars");
            y = toggle_nested(px, fonts, x, y, w, "Clutch", cfg.telemetry_bar_clutch, Hit::TelemetryBarClutch, hover, hits);
            y = toggle_nested(px, fonts, x, y, w, "Brake", cfg.telemetry_bar_brake, Hit::TelemetryBarBrake, hover, hits);
            y = toggle_nested(px, fonts, x, y, w, "Throttle", cfg.telemetry_bar_throttle, Hit::TelemetryBarThrottle, hover, hits);
            y = toggle_nested(px, fonts, x, y, w, "Steer", cfg.telemetry_bar_steer, Hit::TelemetryBarSteer, hover, hits);
        }
        pane_style(px, fonts, spec, cfg, hover, hits, x, y, w)
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
    open_widget_pane(px, fonts, cfg, hover, hits, x, y, w, spec, |px, fonts, y, shown, hits| {
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
            cfg.lean_style.label(),
            open_drop == Some(Drop::LeanStyle),
            Hit::LeanStyleOpen,
            &[
                (Hit::LeanStylePick(LeanStyle::Figure), "Figure", cfg.lean_style == LeanStyle::Figure),
                (Hit::LeanStylePick(LeanStyle::Minimal), "Minimal", cfg.lean_style == LeanStyle::Minimal),
            ],
            hover,
            hits,
        );
        pane_style(px, fonts, spec, cfg, hover, hits, x, y, w)
    })
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
    open_widget_pane(px, fonts, cfg, hover, hits, x, y, w, spec, |px, fonts, y, shown, hits| {
        let mut y = note_lines(
            px,
            fonts,
            x,
            y,
            w,
            "Your local pad, not plugin telemetry. Auto matches your controller. PlayStation and Xbox force that pad art. Triggers fill with squeeze. Bumpers light when held. No pad shows No controller. Other riders’ inputs are not available.",
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
            cfg.gamepad_style.label(),
            open_drop == Some(Drop::GamepadStyle),
            Hit::GamepadStyleOpen,
            &[
                (Hit::GamepadStylePick(GamepadStyle::Auto), "Auto", cfg.gamepad_style == GamepadStyle::Auto),
                (
                    Hit::GamepadStylePick(GamepadStyle::PlayStation),
                    "PlayStation",
                    cfg.gamepad_style == GamepadStyle::PlayStation,
                ),
                (Hit::GamepadStylePick(GamepadStyle::Xbox), "Xbox", cfg.gamepad_style == GamepadStyle::Xbox),
            ],
            hover,
            hits,
        );
        pane_style(px, fonts, spec, cfg, hover, hits, x, y, w)
    })
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
    open_widget_pane(px, fonts, cfg, hover, hits, x, y, w, spec, |px, fonts, y, shown, hits| {
        if !shown {
            return y;
        }
        let mut y = toggle_row(px, fonts, x, y, w, "Text", cfg.flag_text, Hit::FlagText, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Yellow flag", cfg.flag_yellow, Hit::FlagYellow, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Blue flag", cfg.flag_blue, Hit::FlagBlue, hover, hits);
        y = toggle_row(px, fonts, x, y, w, "Red flag", cfg.flag_red, Hit::FlagRed, hover, hits);
        pane_style(px, fonts, spec, cfg, hover, hits, x, y, w)
    })
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
        .map(|&field| (Hit::TickerFootPick(slot, field), field.label(), field == value))
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
            .map(|&field| (Hit::DashFootPick(slot, field), field.label(), field == value))
            .collect(),
    }
}

pub(crate) fn set_info_slot(c: &mut HudConfig, bar: InfoBar, slot: u8, field: BoardField) {
    let slots = match bar {
        InfoBar::StHead => &mut c.st_head,
        InfoBar::StFoot => &mut c.st_foot,
        InfoBar::RelHead => &mut c.rel_head,
        InfoBar::RelFoot => &mut c.rel_foot,
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
                    .map(|&field| (Hit::InfoPick(bar, slot as u8, field), field.label(), field == value))
                    .collect(),
            }
        }),
        hover,
        hits,
    )
}
