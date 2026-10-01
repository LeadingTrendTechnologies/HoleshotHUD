
use tiny_skia::{
    Color, FillRule, GradientStop, LineCap, LineJoin, LinearGradient, Paint, Path, PathBuilder,
    Pixmap, PixmapPaint, Point as SkPoint, Rect, SpreadMode, Stroke, Transform,
};
use windows::Win32::Foundation::{BOOL, HWND};
use windows::Win32::Graphics::Gdi::{
    GetDC, ReleaseDC, SetDIBitsToDevice, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
};

use crate::config::{
    format_primary_color, hsv_to_rgb, rgb_to_hsv, with_config, HudConfig, RelField, SessionPreset, SnapAlign, StField, TableText, WidgetId, COL_W_MIN,
    DEFAULT_PRIMARY, PRIMARY_SWATCHES,
};
use crate::render::{fill_rect, measure, text, Fonts};

use super::*;

pub(crate) fn draw(px: &mut Pixmap, fonts: &Fonts, w: f32, h: f32) {
    refresh_palette();
    px.fill(bg());
    with_config(|cfg| draw_with_cfg(px, fonts, w, h, cfg));
}

pub(crate) fn draw_with_cfg(px: &mut Pixmap, fonts: &Fonts, w: f32, h: f32, cfg: &HudConfig) {
    let pending = crate::feedback::pending_reply();
    let (
        tab,
        hover,
        focus,
        open_drop,
        drag,
        scroll,
        nav_scroll,
        banner_dismissed,
        whats_new_open,
        whats_new_scroll,
        pit_help_open,
        bind_listen,
        reply_scroll,
        reply_id,
        ui_all_time,
        clear_confirm,
        app_section,
        mut profile_section,
        tracks_selected,
        tracks_zoom,
        tracks_pan_x,
        tracks_pan_z,
    ) = {
        let mut ui = UI.lock().unwrap();
        if let Some(u) = ui.as_mut() {
            if u.tab == Tab::Review {
                u.tab = Tab::Profile;
                u.profile_section = ProfileSection::Motos;
            }
            if !u.whats_new_open
                && !u.pit_help_open
                && u.reply_id.is_none()
                && u.clear_confirm.is_none()
            {
                if let Some(view) = pending.as_ref() {
                    u.reply_id = Some(view.id.clone());
                    u.reply_scroll = 0.0;
                    u.focus = Some(Hit::ReplyText);
                }
            }
        }
        let ui = ui.as_ref();
        (
            ui.map(|u| u.tab).unwrap_or(Tab::Standings),
            ui.and_then(|u| u.hover),
            ui.and_then(|u| u.focus),
            ui.and_then(|u| u.open_drop),
            ui.and_then(|u| u.drag),
            ui.map(|u| u.scroll).unwrap_or(0.0),
            ui.map(|u| u.nav_scroll).unwrap_or(0.0),
            ui.map(|u| u.banner_dismissed).unwrap_or(false),
            ui.map(|u| u.whats_new_open).unwrap_or(false),
            ui.map(|u| u.whats_new_scroll).unwrap_or(0.0),
            ui.map(|u| u.pit_help_open).unwrap_or(false),
            ui.map(|u| u.bind_listen).unwrap_or(false),
            ui.map(|u| u.reply_scroll).unwrap_or(0.0),
            ui.and_then(|u| u.reply_id.clone()),
            ui.map(|u| u.profile_all_time).unwrap_or(true),
            ui.and_then(|u| u.clear_confirm),
            ui.map(|u| u.app_section).unwrap_or(AppSection::Look),
            ui.map(|u| u.profile_section)
                .unwrap_or(ProfileSection::Overview),
            ui.and_then(|u| u.tracks_selected.clone()),
            ui.map(|u| u.tracks_zoom).unwrap_or(1.0),
            ui.map(|u| u.tracks_pan_x).unwrap_or(0.0),
            ui.map(|u| u.tracks_pan_z).unwrap_or(0.0),
        )
    };
    if profile_section == ProfileSection::Tracks && !cfg.experimental_unlocked() {
        profile_section = ProfileSection::Overview;
        if let Some(ui) = UI.lock().unwrap().as_mut() {
            ui.profile_section = ProfileSection::Overview;
            ui.tracks_selected = None;
        }
    }
    let banner = crate::update::manual_banner(
        cfg.auto_update_on_launch,
        banner_dismissed,
        &crate::update::state(),
    );
    let banner_h = if banner.is_some() {
        if crate::update::update_may_need_admin() {
            UPDATE_BANNER_H_ADMIN
        } else {
            UPDATE_BANNER_H
        }
    } else {
        0.0
    };
    let mut hits = Vec::new();
    DROP_MENUS.with(|menus| menus.borrow_mut().clear());
    COLOR_PICKER_ANCHOR.with(|a| a.set(None));
    if let Some(ui) = UI.lock().unwrap().as_mut() {
        ui.col_list_n = 0;
        ui.col_list_kind = None;
    }

    let top_y = banner_h;
    let clip_top = top_y + TOP_H;
    let clip_bottom = h;
    let widgets = tab.is_widget();
    let show_side = widgets || tab == Tab::App;
    let side_w = if show_side { SIDE_W } else { 0.0 };

    let mut nav_content_h = 0.0;
    let mut nav_scroll = nav_scroll;
    if show_side {
        if let Some(r) = Rect::from_xywh(0.0, clip_top, SIDE_W, (h - clip_top).max(0.0)) {
            fill_rect(px, r, side());
        }
        if let Some(r) = Rect::from_xywh(SIDE_W, clip_top, 1.0, (h - clip_top).max(0.0)) {
            fill_rect(px, r, row_line());
        }
        if widgets {
            nav_content_h = widget_rail_height(&cfg);
            let view_h = (clip_bottom - clip_top).max(0.0);
            let nav_max = (nav_content_h - view_h).max(0.0);
            nav_scroll = nav_scroll.clamp(0.0, nav_max);
            draw_widget_rail(
                px,
                fonts,
                &cfg,
                tab,
                hover,
                &mut hits,
                clip_top,
                clip_bottom,
                nav_scroll,
            );
            if nav_max > 1.0 && view_h > 8.0 {
                let track_x = SIDE_W - 7.0;
                let thumb_h = (view_h * view_h / nav_content_h).clamp(16.0, view_h);
                let thumb_y = clip_top + nav_scroll / nav_max * (view_h - thumb_h);
                fill_round(
                    px,
                    track_x,
                    clip_top + 4.0,
                    3.0,
                    (view_h - 8.0).max(4.0),
                    1.5,
                    menu_edge(),
                );
                fill_round(px, track_x, thumb_y, 3.0, thumb_h, 1.5, menu_edge_strong());
            }
        } else {
            draw_app_rail(px, fonts, app_section, hover, &mut hits, clip_top);
            nav_scroll = 0.0;
            nav_content_h = 0.0;
        }
    }

    let x = side_w + 28.0;
    let cw = (w - x - 28.0).max(200.0);
    let preset_h = if widgets { PRESET_STRIP_H } else { 0.0 };
    let profile_h = if tab == Tab::Profile {
        PROFILE_SUBNAV_H
    } else {
        0.0
    };
    let py = clip_top + 20.0 + preset_h + profile_h - scroll;
    let on_motos_pane =
        tab == Tab::Profile && profile_section == ProfileSection::Motos || tab == Tab::Review;
    if on_motos_pane {
        open_live_analyze(false);
    }
    let (
        analyze_id,
        review_filter,
        analyze_compare,
        analyze_you_lap,
        analyze_warmup,
        analyze_scrub,
        analyze_zoom,
        analyze_pan_x,
        analyze_pan_z,
        analyze_follow,
    ) = {
        let ui = UI.lock().unwrap();
        let u = ui.as_ref();
        (
            u.and_then(|u| u.analyze_id),
            u.map(|u| u.review_filter)
                .unwrap_or(crate::review::ListFilter::All),
            u.map(|u| u.analyze_compare).unwrap_or(-1),
            u.map(|u| u.analyze_you_lap).unwrap_or(-1),
            u.map(|u| u.analyze_warmup).unwrap_or(false),
            u.map(|u| u.analyze_scrub).unwrap_or(0.0),
            u.map(|u| u.analyze_zoom).unwrap_or(1.0),
            u.map(|u| u.analyze_pan_x).unwrap_or(0.0),
            u.map(|u| u.analyze_pan_z).unwrap_or(0.0),
            u.map(|u| u.analyze_follow).unwrap_or(false),
        )
    };
    let bottom = match tab {
        Tab::App => pane_app(
            px,
            fonts,
            &cfg,
            hover,
            open_drop,
            app_section,
            &mut hits,
            x,
            py,
            cw,
        ),
        Tab::Review | Tab::Profile => {
            if profile_section == ProfileSection::Motos || tab == Tab::Review {
                let b = pane_review(
                    px,
                    fonts,
                    hover,
                    &mut hits,
                    x,
                    py,
                    cw,
                    analyze_id,
                    review_filter,
                    analyze_compare,
                    analyze_you_lap,
                    analyze_warmup,
                    open_drop,
                    analyze_scrub,
                    analyze_zoom,
                    analyze_pan_x,
                    analyze_pan_z,
                    analyze_follow,
                    cfg.review,
                    cfg.units.speed,
                );
                persist_analyze_follow_pan(
                    analyze_id,
                    analyze_you_lap,
                    analyze_warmup,
                    analyze_scrub,
                    analyze_zoom,
                    analyze_follow,
                );
                b
            } else if profile_section == ProfileSection::Tracks {
                tracks::pane_tracks(
                    px,
                    fonts,
                    hover,
                    &mut hits,
                    x,
                    py,
                    cw,
                    &tracks_selected,
                    tracks_zoom,
                    tracks_pan_x,
                    tracks_pan_z,
                )
            } else {
                pane_profile(
                    px,
                    fonts,
                    hover,
                    &mut hits,
                    x,
                    py,
                    cw,
                    ui_all_time,
                    cfg.review,
                )
            }
        }
        Tab::Feedback => pane_feedback_tab(px, fonts, hover, &mut hits, x, py, cw),
        Tab::Standings => pane_standings(
            px, fonts, &cfg, hover, open_drop, drag, &mut hits, x, py, cw,
        ),
        Tab::Relative => pane_relative(
            px, fonts, &cfg, hover, open_drop, drag, &mut hits, x, py, cw,
        ),
        Tab::Map => pane_map(px, fonts, &cfg, hover, open_drop, &mut hits, x, py, cw),
        Tab::Minimap => pane_minimap(px, fonts, &cfg, hover, open_drop, &mut hits, x, py, cw),
        Tab::Radar => pane_radar(px, fonts, &cfg, hover, open_drop, &mut hits, x, py, cw),
        Tab::Dash => pane_dash(px, fonts, &cfg, hover, open_drop, &mut hits, x, py, cw),
        Tab::Ticker => pane_ticker(px, fonts, &cfg, hover, open_drop, &mut hits, x, py, cw),
        Tab::Sys => pane_sys(px, fonts, &cfg, hover, open_drop, &mut hits, x, py, cw),
        Tab::Sector => pane_sector(px, fonts, &cfg, hover, &mut hits, x, py, cw),
        Tab::Delta => pane_delta(px, fonts, &cfg, hover, &mut hits, x, py, cw),
        Tab::Stance => pane_stance(
            px,
            fonts,
            &cfg,
            hover,
            open_drop,
            bind_listen,
            &mut hits,
            x,
            py,
            cw,
        ),
        Tab::Flag => pane_flag(px, fonts, &cfg, hover, &mut hits, x, py, cw),
        Tab::Lean => pane_lean(px, fonts, &cfg, hover, open_drop, &mut hits, x, py, cw),
        Tab::Gamepad => pane_gamepad(px, fonts, &cfg, hover, &mut hits, x, py, cw, open_drop),
        Tab::Telemetry => pane_telemetry(px, fonts, &cfg, hover, &mut hits, x, py, cw),
        Tab::Pitboard => pane_pitboard(px, fonts, &cfg, hover, open_drop, &mut hits, x, py, cw),
    };
    if widgets {
        let sticky_bottom = clip_top + 10.0 + PRESET_STRIP_H;
        clip_content_hits(&mut hits, side_w, sticky_bottom);
        if let Some(r) = Rect::from_xywh(
            side_w + 1.0,
            clip_top,
            (w - side_w - 1.0).max(0.0),
            (sticky_bottom - clip_top).max(0.0),
        ) {
            fill_rect(px, r, bg());
        }
        draw_preset_strip(
            px,
            fonts,
            &cfg,
            hover,
            open_drop,
            &mut hits,
            x,
            clip_top + 10.0,
            cw,
        );
    }
    if tab == Tab::Profile {
        let sticky_bottom = clip_top + PROFILE_SUBNAV_H;
        clip_content_hits(&mut hits, 0.0, sticky_bottom);
        if let Some(r) = Rect::from_xywh(0.0, clip_top, w, PROFILE_SUBNAV_H) {
            fill_rect(px, r, side());
        }
        if let Some(r) = Rect::from_xywh(0.0, sticky_bottom, w, 1.0) {
            fill_rect(px, r, row_line());
        }
        draw_profile_subnav(
            px,
            fonts,
            profile_section,
            cfg.experimental_unlocked(),
            hover,
            &mut hits,
            28.0,
            clip_top + 8.0,
        );
    }
    draw_top_bar(
        px,
        fonts,
        w,
        top_y,
        tab,
        cfg.settings_key.label(),
        cfg.review,
        cfg.stream_enabled,
        hover,
        &mut hits,
    );

    if let Some(kind) = banner {
        draw_update_banner(px, fonts, w, kind, hover, &mut hits);
    }
    let mut whats_new_scroll_max = 0.0;
    let mut reply_scroll_max = 0.0;
    let reply_view = reply_id
        .as_deref()
        .and_then(crate::feedback::ticket_view)
        .or_else(|| pending.clone());
    if let Some(view) = reply_view.as_ref() {
        crate::feedback::prepare_compose(&view.id);
    }
    if whats_new_open {
        if let Some(notes) = crate::changelog::modal_notes() {
            hits.clear();
            whats_new_scroll_max =
                draw_whats_new(px, fonts, w, h, &notes, hover, whats_new_scroll, &mut hits);
            paint_focus(px, &hits, focus);
        }
    } else if let Some(view) = reply_view.as_ref() {
        hits.clear();
        reply_scroll_max = draw_reply(px, fonts, w, h, view, hover, reply_scroll, &mut hits);
        paint_focus(px, &hits, focus);
    } else if let Some(kind) = clear_confirm {
        hits.clear();
        draw_clear_confirm(px, fonts, w, h, kind, hover, &mut hits);
        paint_focus(px, &hits, focus);
    } else if pit_help_open {
        hits.clear();
        draw_pit_help(px, fonts, w, h, hover, &mut hits);
        paint_focus(px, &hits, focus);
    } else {
        paint_focus(px, &hits, focus);
        paint_drop_menus(px, fonts, hover, &mut hits);
        match open_drop {
            Some(Drop::PrimaryColor) => {
                paint_color_picker(
                    px,
                    fonts,
                    hover,
                    &mut hits,
                    ColorPickKind::App,
                    cfg.primary,
                    DEFAULT_PRIMARY,
                );
            }
            Some(Drop::GameUiPrimaryColor) => {
                paint_color_picker(
                    px,
                    fonts,
                    hover,
                    &mut hits,
                    ColorPickKind::Menu,
                    cfg.game_ui_primary,
                    DEFAULT_PRIMARY,
                );
            }
            Some(Drop::PitYellow) => {
                paint_color_picker(
                    px,
                    fonts,
                    hover,
                    &mut hits,
                    ColorPickKind::PitYellow,
                    cfg.pit.pit_yellow,
                    cfg.primary,
                );
            }
            Some(Drop::PitBlue) => {
                paint_color_picker(
                    px,
                    fonts,
                    hover,
                    &mut hits,
                    ColorPickKind::PitBlue,
                    cfg.pit.pit_blue,
                    [0, 0, 0],
                );
            }
            Some(Drop::PitSlotColor) => {
                let rgb = cfg
                    .pit.pit_vars
                    .get(pit_selected() as usize)
                    .and_then(|place| place.color)
                    .unwrap_or([16, 16, 18]);
                paint_color_picker(
                    px,
                    fonts,
                    hover,
                    &mut hits,
                    ColorPickKind::PitSlot,
                    rgb,
                    [16, 16, 18],
                );
            }
            _ => {}
        }
        paint_snap_tooltip(px, fonts, &cfg, hover, focus, &hits, w, h, clip_top);
        paint_slot_center_tooltip(px, fonts, hover, focus, &hits, w, h, clip_top);
        paint_edit_stream_tooltip(px, fonts, hover, focus, &hits, w, h);
    }

    if let Some(ui) = UI.lock().unwrap().as_mut() {
        ui.hits = hits;
        ui.content_h = bottom + scroll;
        let max = (ui.content_h - h + 24.0).max(0.0);
        ui.scroll_max = max;
        ui.scroll = ui.scroll.clamp(0.0, max);
        ui.whats_new_scroll_max = whats_new_scroll_max;
        ui.whats_new_scroll = ui.whats_new_scroll.clamp(0.0, whats_new_scroll_max);
        ui.reply_scroll_max = reply_scroll_max;
        ui.reply_scroll = ui.reply_scroll.clamp(0.0, reply_scroll_max);
        if widgets {
            ui.nav_top = clip_top;
            ui.nav_bottom = clip_bottom;
            ui.nav_content_h = nav_content_h;
            ui.nav_scroll = nav_scroll;
        } else {
            ui.nav_top = 0.0;
            ui.nav_bottom = 0.0;
            ui.nav_content_h = 0.0;
            ui.nav_scroll = 0.0;
        }
    }
}

pub(crate) fn snap_align_label(align: SnapAlign) -> &'static str {
    match align {
        SnapAlign::TopLeft => "Top left",
        SnapAlign::Top => "Top center",
        SnapAlign::TopRight => "Top right",
        SnapAlign::Left => "Middle left",
        SnapAlign::Center => "Center",
        SnapAlign::Right => "Middle right",
        SnapAlign::BottomLeft => "Bottom left",
        SnapAlign::Bottom => "Bottom center",
        SnapAlign::BottomRight => "Bottom right",
        SnapAlign::HCenter => "Center horizontally",
        SnapAlign::VCenter => "Center vertically",
    }
}

pub(crate) fn widget_short_name(id: WidgetId) -> &'static str {
    match id {
        WidgetId::Standings => "Standings",
        WidgetId::Relative => "Relative",
        WidgetId::Map => "Map",
        WidgetId::Minimap => "Minimap",
        WidgetId::Radar => "Radar",
        WidgetId::Dash => "Dash",
        WidgetId::Ticker => "H-Standings",
        WidgetId::Sys => "Systems",
        WidgetId::Sector => "Sectors",
        WidgetId::Delta => "Delta Bar",
        WidgetId::Stance => "Stance",
        WidgetId::Flag => "Flags",
        WidgetId::Lean => "Lean",
        WidgetId::Gamepad => "Controller",
        WidgetId::Telemetry => "Telemetry",
        WidgetId::Pitboard => "Pit Board",
    }
}

pub(crate) fn paint_slot_center_tooltip(
    px: &mut Pixmap,
    fonts: &Fonts,
    hover: Option<Hit>,
    focus: Option<Hit>,
    hits: &[HitBox],
    win_w: f32,
    win_h: f32,
    clip_top: f32,
) {
    let Some(hit) = hover.or(focus) else {
        return;
    };
    let (title, hint) = match hit {
        Hit::PitSlotCenterX => ("Center horizontally", "Won't move up or down"),
        Hit::PitSlotCenterY => ("Center vertically", "Won't move left or right"),
        _ => return,
    };
    let Some(button) = hits.iter().rev().find(|box_hit| box_hit.id == hit) else {
        return;
    };
    let pad = 12.0;
    let tw = measure(fonts, title, 14.0)
        .max(measure(fonts, hint, 11.0))
        + pad * 2.0;
    let th = pad + 16.0 + 4.0 + 14.0 + pad;
    let mut tx = button.x + button.w + 16.0;
    let mut ty = button.y;
    if tx + tw > win_w - 16.0 {
        tx = (button.x - 16.0 - tw).max(16.0);
    }
    if ty < clip_top + 8.0 {
        ty = clip_top + 8.0;
    }
    if ty + th > win_h - 10.0 {
        ty = (win_h - th - 10.0).max(clip_top + 8.0);
    }
    fill_round(px, tx - 1.0, ty - 1.0, tw + 2.0, th + 2.0, 12.0, shadow());
    outlined(px, tx, ty, tw, th, 11.0, menu_fill());
    text(px, fonts, title, 14.0, tx + pad, ty + pad, text_col(), false);
    text(px, fonts, hint, 11.0, tx + pad, ty + pad + 20.0, dim(), false);
}

pub(crate) fn paint_edit_stream_tooltip(
    px: &mut Pixmap,
    fonts: &Fonts,
    hover: Option<Hit>,
    focus: Option<Hit>,
    hits: &[HitBox],
    win_w: f32,
    win_h: f32,
) {
    let hit = Hit::StreamOpenEditUrl;
    if hover.or(focus) != Some(hit) {
        return;
    }
    let Some(button) = hits.iter().rev().find(|box_hit| box_hit.id == hit) else {
        return;
    };
    let title = "Edit stream";
    let hint = "Updates in your browser";
    let pad = 12.0;
    let tw = measure(fonts, title, 14.0)
        .max(measure(fonts, hint, 11.0))
        + pad * 2.0;
    let th = pad + 16.0 + 4.0 + 14.0 + pad;
    let mut tx = button.x;
    let mut ty = button.y + button.h + 8.0;
    if tx + tw > win_w - 16.0 {
        tx = (win_w - 16.0 - tw).max(16.0);
    }
    if ty + th > win_h - 10.0 {
        ty = (button.y - 8.0 - th).max(8.0);
    }
    fill_round(px, tx - 1.0, ty - 1.0, tw + 2.0, th + 2.0, 12.0, shadow());
    outlined(px, tx, ty, tw, th, 11.0, menu_fill());
    text(px, fonts, title, 14.0, tx + pad, ty + pad, text_col(), false);
    text(px, fonts, hint, 11.0, tx + pad, ty + pad + 20.0, dim(), false);
}

pub(crate) fn paint_snap_tooltip(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    focus: Option<Hit>,
    hits: &[HitBox],
    win_w: f32,
    win_h: f32,
    clip_top: f32,
) {
    let Some(Hit::Snap(id, align)) = hover.or(focus) else {
        return;
    };
    let mut cluster_l = f32::MAX;
    let mut cluster_r = 0.0f32;
    let mut cluster_t = f32::MAX;
    let mut found = false;
    for hb in hits {
        if matches!(hb.id, Hit::Snap(wid, _) if wid == id) {
            found = true;
            cluster_l = cluster_l.min(hb.x);
            cluster_r = cluster_r.max(hb.x + hb.w);
            cluster_t = cluster_t.min(hb.y);
        }
    }
    if !found {
        return;
    }

    let hint = match align {
        SnapAlign::HCenter => Some("Won't move up or down"),
        SnapAlign::VCenter => Some("Won't move left or right"),
        _ => None,
    };
    let title = snap_align_label(align);
    let name = widget_short_name(id);
    let pad = 12.0;
    let screen_w = 176.0;
    let screen_h = 99.0;
    let tw = screen_w + pad * 2.0;
    let th = pad + 16.0 + 16.0 + if hint.is_some() { 15.0 } else { 0.0 } + 8.0 + screen_h + pad;
    let mut tx = cluster_r + 16.0;
    let mut ty = cluster_t;
    if tx + tw > win_w - 16.0 {
        tx = (cluster_l - 16.0 - tw).max(16.0);
    }
    if ty < clip_top + 8.0 {
        ty = clip_top + 8.0;
    }
    if ty + th > win_h - 10.0 {
        ty = (win_h - th - 10.0).max(clip_top + 8.0);
    }

    fill_round(px, tx - 1.0, ty - 1.0, tw + 2.0, th + 2.0, 12.0, shadow());
    outlined(px, tx, ty, tw, th, 11.0, menu_fill());
    text(px, fonts, name, 11.0, tx + pad, ty + pad, muted(), false);
    text(
        px,
        fonts,
        title,
        14.0,
        tx + pad,
        ty + pad + 15.0,
        text_col(),
        false,
    );
    let mut sy = ty + pad + 34.0;
    if let Some(hint) = hint {
        text(px, fonts, hint, 11.0, tx + pad, sy, dim(), false);
        sy += 15.0;
    }
    let sx = tx + pad;
    outlined(
        px,
        sx,
        sy,
        screen_w,
        screen_h,
        6.0,
        Color::from_rgba8(12, 12, 16, 255),
    );
    let inset = 5.0;
    let iw = screen_w - inset * 2.0;
    let ih = screen_h - inset * 2.0;
    let ix = sx + inset;
    let iy = sy + inset;
    let before = cfg.widget_rect(id);
    let after = cfg.snapped_rect(id, align);
    paint_snap_preview_blob(
        px,
        ix,
        iy,
        iw,
        ih,
        before,
        Color::from_rgba8(255, 255, 255, 36),
    );
    paint_snap_preview_blob(px, ix, iy, iw, ih, after, accent());
}

pub(crate) fn paint_snap_preview_blob(
    px: &mut Pixmap,
    ix: f32,
    iy: f32,
    iw: f32,
    ih: f32,
    r: crate::shm::Rect,
    c: Color,
) {
    let x = ix + r.x.clamp(0.0, 1.0) * iw;
    let y = iy + r.y.clamp(0.0, 1.0) * ih;
    let w = (r.w.clamp(0.02, 1.0) * iw)
        .max(10.0)
        .min((ix + iw - x).max(4.0));
    let h = (r.h.clamp(0.02, 1.0) * ih)
        .max(8.0)
        .min((iy + ih - y).max(4.0));
    fill_round(px, x, y, w, h, 3.0, c);
}

pub(crate) fn paint_focus(px: &mut Pixmap, hits: &[HitBox], focus: Option<Hit>) {
    let Some(id) = focus else {
        return;
    };
    let Some(hb) = hits.iter().filter(|h| h.id == id).max_by(|a, b| {
        (a.w * a.h)
            .partial_cmp(&(b.w * b.h))
            .unwrap_or(std::cmp::Ordering::Equal)
    }) else {
        return;
    };
    let pad = 3.0;
    let mut pb = PathBuilder::new();
    let x = hb.x - pad;
    let y = hb.y - pad;
    let w = hb.w + pad * 2.0;
    let h = hb.h + pad * 2.0;
    let r = 10.0;
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.quad_to(x + w, y, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.quad_to(x + w, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.quad_to(x, y + h, x, y + h - r);
    pb.line_to(x, y + r);
    pb.quad_to(x, y, x + r, y);
    pb.close();
    if let Some(path) = pb.finish() {
        let mut p = Paint::default();
        p.set_color(accent());
        p.anti_alias = true;
        px.stroke_path(
            &path,
            &p,
            &Stroke {
                width: 2.0,
                line_cap: LineCap::Round,
                line_join: LineJoin::Round,
                ..Stroke::default()
            },
            Transform::identity(),
            None,
        );
    }
}

pub(crate) fn widget_groups(cfg: &HudConfig) -> Vec<(&'static str, Vec<(Tab, Hit, &'static str, bool)>)> {
    let cockpit = vec![
        (Tab::Dash, Hit::TabDash, "Dash", cfg[WidgetId::Dash].show),
        (
            Tab::Telemetry,
            Hit::TabTelemetry,
            "Telemetry",
            cfg[WidgetId::Telemetry].show,
        ),
        (Tab::Lean, Hit::TabLean, "Lean", cfg[WidgetId::Lean].show),
        (
            Tab::Delta,
            Hit::TabDelta,
            "Delta Bar",
            cfg[WidgetId::Delta].show,
        ),
        (
            Tab::Sector,
            Hit::TabSector,
            "Sectors",
            cfg[WidgetId::Sector].show,
        ),
        (Tab::Flag, Hit::TabFlag, "Flags", cfg[WidgetId::Flag].show),
        (Tab::Sys, Hit::TabSys, "Systems", cfg[WidgetId::Sys].show),
        (
            Tab::Stance,
            Hit::TabStance,
            "Stance",
            cfg[WidgetId::Stance].show,
        ),
        (
            Tab::Gamepad,
            Hit::TabGamepad,
            "Controller",
            cfg[WidgetId::Gamepad].show,
        ),
        (
            Tab::Pitboard,
            Hit::TabPitboard,
            "Pit Board",
            cfg[WidgetId::Pitboard].show,
        ),
    ];
    let mut groups = vec![
        (
            "Boards",
            vec![
                (
                    Tab::Standings,
                    Hit::TabSt,
                    "Standings",
                    cfg[WidgetId::Standings].show,
                ),
                (
                    Tab::Relative,
                    Hit::TabRel,
                    "Relative",
                    cfg[WidgetId::Relative].show,
                ),
                (
                    Tab::Ticker,
                    Hit::TabTicker,
                    "H-Standings",
                    cfg[WidgetId::Ticker].show,
                ),
            ],
        ),
        (
            "Track",
            vec![
                (Tab::Map, Hit::TabMap, "Map", cfg[WidgetId::Map].show),
                (
                    Tab::Minimap,
                    Hit::TabMini,
                    "Minimap",
                    cfg[WidgetId::Minimap].show,
                ),
                (
                    Tab::Radar,
                    Hit::TabRadar,
                    "Radar",
                    cfg[WidgetId::Radar].show,
                ),
            ],
        ),
        ("Cockpit", cockpit),
    ];
    groups.sort_by(|left, right| left.0.cmp(right.0));
    for (_, items) in &mut groups {
        items.sort_by(|left, right| left.2.cmp(right.2));
    }
    groups
}

pub(crate) fn widget_rail_height(cfg: &HudConfig) -> f32 {
    let mut h = 10.0;
    for (_, items) in widget_groups(cfg) {
        h += 24.0 + items.len() as f32 * 40.0 + 6.0;
    }
    h
}

pub(crate) fn draw_widget_rail(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    tab: Tab,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    clip_top: f32,
    clip_bottom: f32,
    nav_scroll: f32,
) {
    let clip = Some((clip_top, clip_bottom));
    let mut y = clip_top + 10.0 - nav_scroll;
    for (title, items) in widget_groups(cfg) {
        y = nav_group(px, fonts, 12.0, y, title);
        for (t, hit, name, on) in items {
            nav_tab(
                px,
                fonts,
                12.0,
                y,
                SIDE_W - 24.0,
                36.0,
                t == tab,
                on,
                name,
                hit,
                hover,
                hits,
                clip,
            );
            y += 40.0;
        }
        y += 6.0;
    }
}

pub(crate) fn draw_app_rail(
    px: &mut Pixmap,
    fonts: &Fonts,
    section: AppSection,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    clip_top: f32,
) {
    let mut y = clip_top + 10.0;
    for (title, items) in app_section_groups() {
        y = nav_group(px, fonts, 12.0, y, title);
        for &item in &items {
            section_nav_tab(
                px,
                fonts,
                12.0,
                y,
                SIDE_W - 24.0,
                36.0,
                item == section,
                item.label(),
                item.hit(),
                hover,
                hits,
            );
            y += 40.0;
        }
        y += 6.0;
    }
}

pub(crate) fn draw_profile_subnav(
    px: &mut Pixmap,
    fonts: &Fonts,
    section: ProfileSection,
    show_tracks: bool,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
) {
    let mut cx = x;
    let items: &[ProfileSection] = if show_tracks {
        &[
            ProfileSection::Overview,
            ProfileSection::Motos,
            ProfileSection::Tracks,
        ]
    } else {
        &[ProfileSection::Overview, ProfileSection::Motos]
    };
    for item in items {
        cx += profile::profile_chip(
            px,
            fonts,
            cx,
            y,
            item.label(),
            *item == section,
            item.hit(),
            hover,
            hits,
        ) + 8.0;
    }
}

pub(crate) fn draw_top_bar(
    px: &mut Pixmap,
    fonts: &Fonts,
    w: f32,
    y: f32,
    tab: Tab,
    key_label: &str,
    _review_on: bool,
    stream_enabled: bool,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) {
    if let Some(r) = Rect::from_xywh(0.0, y, w, TOP_H) {
        fill_rect(px, r, side());
    }
    if let Some(r) = Rect::from_xywh(0.0, y + TOP_H, w, 1.0) {
        fill_rect(px, r, menu_edge_strong());
    }
    let logo = brand_logo();
    let lx = 12.0;
    let ly = y + ((TOP_H - logo.height() as f32) * 0.5).max(0.0);
    let _ = px.draw_pixmap(
        lx as i32,
        ly as i32,
        logo.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
    let mut mx = lx + logo.width() as f32 + 14.0;
    let ty = y + (TOP_H - 32.0) * 0.5;
    mx += mode_tab(
        px,
        fonts,
        mx,
        ty,
        "Widgets",
        tab.is_widget(),
        Hit::TabWidgets,
        hover,
        hits,
    );
    mx += mode_tab(
        px,
        fonts,
        mx,
        ty,
        "Profile",
        tab == Tab::Profile || tab == Tab::Review,
        Hit::TabProfile,
        hover,
        hits,
    );
    mx += mode_tab(
        px,
        fonts,
        mx,
        ty,
        "Settings",
        tab == Tab::App,
        Hit::TabApp,
        hover,
        hits,
    );
    mx += mode_tab(
        px,
        fonts,
        mx,
        ty,
        "Feedback",
        tab == Tab::Feedback,
        Hit::TabFeedback,
        hover,
        hits,
    );
    if stream_enabled {
        let _ = mode_tab(
            px,
            fonts,
            mx,
            ty,
            "Edit stream",
            false,
            Hit::StreamOpenEditUrl,
            hover,
            hits,
        );
    }

    let quit_w = 148.0;
    let quit_h = 32.0;
    let qx = w - 14.0 - quit_w;
    let qy = y + (TOP_H - quit_h) * 0.5;
    let hint = format!("{key_label}  settings");
    let hw = measure(fonts, &hint, 10.0);
    text(
        px,
        fonts,
        &hint,
        10.0,
        qx - 16.0 - hw,
        y + 20.0,
        dim(),
        false,
    );
    sidebar_quit(px, fonts, qx, qy, quit_w, quit_h, hover, hits);
}

pub(crate) fn clip_content_hits(hits: &mut Vec<HitBox>, left: f32, top: f32) {
    hits.retain(|h| h.x + h.w <= left || h.y + h.h > top);
    for h in hits.iter_mut() {
        if h.x + h.w > left && h.y < top {
            let bottom = h.y + h.h;
            h.h = (bottom - top).max(0.0);
            h.y = top;
        }
    }
}

pub(crate) fn draw_preset_strip(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    open_drop: Option<Drop>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) {
    let live = cfg.session_live;
    let selected = cfg.settings_preset;
    let status = preset_strip_status(live, selected, cfg.active_preset);
    text(px, fonts, &status, 11.0, x, y + 2.0, muted(), false);
    let mut mx = x;
    let cy = y + 18.0;
    for p in SessionPreset::ALL {
        let hit = Hit::Preset(p);
        let on = p == selected;
        let live_mark = live && p == cfg.active_preset && !on;
        mx += preset_chip(
            px,
            fonts,
            mx,
            cy,
            p.label(),
            on,
            live_mark,
            hit,
            hover,
            hits,
        );
    }
    let _ = preset_copy_btn(px, fonts, mx + 8.0, cy, selected, open_drop, hover, hits);
    let _ = w;
}

pub(crate) fn preset_copy_btn(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    src: SessionPreset,
    open_drop: Option<Drop>,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let copy = "Copy to";
    let tw = measure(fonts, copy, 12.0);
    let bw = tw + 32.0;
    let h = 28.0;
    let hit = Hit::PresetCopyOpen;
    let open = open_drop == Some(Drop::PresetCopy);
    hits.push(HitBox {
        id: hit,
        x,
        y,
        w: bw,
        h,
    });
    if open || hover == Some(hit) {
        fill_round(px, x, y, bw, h, 8.0, hover_wash());
    }
    text(px, fonts, copy, 12.0, x + 10.0, y + 6.0, nav_idle(), false);
    chevron(px, x + bw - 12.0, y + h * 0.5, open, muted());
    if open {
        let mut options: Vec<(Hit, String, bool)> = SessionPreset::ALL
            .into_iter()
            .filter(|p| *p != src)
            .map(|p| (Hit::PresetCopyTo(p), p.label().to_string(), false))
            .collect();
        options.push((Hit::PresetCopyAll, "All".into(), false));
        let item_h = 28.0;
        let pad = 5.0;
        let content_h = pad * 2.0 + item_h * options.len() as f32;
        DROP_MENUS.with(|menus| {
            menus.borrow_mut().push(PendingDrop {
                mx: x,
                my: y + h + 6.0,
                bw: bw.max(168.0),
                content_h,
                open_hit: hit,
                options,
            });
        });
    }
    bw
}

pub(crate) fn preset_strip_status(live: bool, selected: SessionPreset, live_preset: SessionPreset) -> String {
    let board = selected.label();
    if live {
        if selected == live_preset {
            format!("On track — editing {board}")
        } else {
            format!("Editing {board} — {} is live", live_preset.label())
        }
    } else {
        format!("Editing {board} — garage")
    }
}

pub(crate) fn preset_chip(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    label: &str,
    selected: bool,
    live_mark: bool,
    hit: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let size = 13.0;
    let tw = measure(fonts, label, size);
    let h = 28.0;
    let skew = 6.0;
    let pip = if live_mark { 10.0 } else { 0.0 };
    let bw = (tw + 22.0 + pip).max(72.0);
    hits.push(HitBox {
        id: hit,
        x,
        y,
        w: bw + skew,
        h,
    });
    if selected {
        fill_skew(px, x, y, (bw - skew).max(48.0), h, skew, accent());
        text(px, fonts, label, size, x + 12.0, y + 6.0, ink(), false);
    } else {
        if hover == Some(hit) {
            fill_round(px, x, y, bw, h, 8.0, hover_wash());
        }
        if live_mark {
            fill_circle(px, x + 12.0, y + h * 0.5, 3.0, accent());
        }
        text(
            px,
            fonts,
            label,
            size,
            x + 12.0 + pip,
            y + 6.0,
            nav_idle(),
            false,
        );
    }
    bw + 8.0
}

pub(crate) fn mode_tab(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    label: &str,
    selected: bool,
    hit: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let size = 14.0;
    let tw = measure(fonts, label, size);
    let h = 32.0;
    let skew = 6.0;
    let bw = (tw + 28.0).max(80.0);
    hits.push(HitBox {
        id: hit,
        x,
        y,
        w: bw + skew,
        h,
    });
    if selected {
        fill_skew(px, x, y, (bw - skew).max(48.0), h, skew, accent());
        text(px, fonts, label, size, x + 14.0, y + 8.0, ink(), false);
    } else {
        if hover == Some(hit) {
            fill_round(px, x, y, bw, h, 8.0, hover_wash());
        }
        text(px, fonts, label, size, x + 14.0, y + 8.0, nav_idle(), false);
    }
    bw + 8.0
}

pub(crate) fn brand_logo() -> &'static Pixmap {
    static LOGO: std::sync::OnceLock<Pixmap> = std::sync::OnceLock::new();
    LOGO.get_or_init(|| {
        Pixmap::decode_png(include_bytes!("../../icon-48.png")).expect("icon-48.png")
    })
}

pub(crate) fn draw_update_banner(
    px: &mut Pixmap,
    fonts: &Fonts,
    w: f32,
    kind: crate::update::ManualBanner,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) {
    let need_admin = crate::update::update_may_need_admin();
    let h = if need_admin {
        UPDATE_BANNER_H_ADMIN
    } else {
        UPDATE_BANNER_H
    };
    if let Some(r) = Rect::from_xywh(0.0, 0.0, w, h) {
        fill_rect(px, r, Color::from_rgba8(48, 36, 22, 255));
    }
    if let Some(r) = Rect::from_xywh(0.0, 0.0, 3.0, h) {
        fill_rect(px, r, accent());
    }
    if let Some(r) = Rect::from_xywh(0.0, h, w, 1.0) {
        fill_rect(px, r, accent_a(50));
    }
    hits.push(HitBox {
        id: Hit::UpdateBanner,
        x: 0.0,
        y: 0.0,
        w,
        h,
    });
    let (line, show_update) = match &kind {
        crate::update::ManualBanner::Available { version } => {
            (format!("Version {version} is available."), true)
        }
        crate::update::ManualBanner::Installing => (
            "Downloading and installing… the app will restart.".to_string(),
            false,
        ),
    };
    let later_w = 108.0;
    let later_x = w - 16.0 - later_w;
    let btn_h = 28.0;
    let by = (h - btn_h) * 0.5;
    text(
        px,
        fonts,
        &line,
        13.0,
        16.0,
        if need_admin { 10.0 } else { 16.0 },
        text_col(),
        false,
    );
    if need_admin {
        text(
            px,
            fonts,
            crate::update::ADMIN_UPDATE_HINT,
            11.0,
            16.0,
            30.0,
            muted(),
            false,
        );
    }
    if show_update {
        action_btn(
            px,
            fonts,
            later_x - 8.0 - 92.0,
            by,
            92.0,
            btn_h,
            "Update",
            Hit::UpdateInstall,
            hover,
            hits,
            true,
        );
    }
    action_btn(
        px,
        fonts,
        later_x,
        by,
        later_w,
        btn_h,
        "Not now",
        Hit::UpdateBannerDismiss,
        hover,
        hits,
        false,
    );
}

pub(crate) fn nav_group(px: &mut Pixmap, fonts: &Fonts, x: f32, y: f32, label: &str) -> f32 {
    text(
        px,
        fonts,
        &label.to_ascii_uppercase(),
        10.0,
        x + 14.0,
        y + 6.0,
        dim(),
        false,
    );
    y + 24.0
}

pub(crate) fn nav_icon(px: &mut Pixmap, hit: Hit, cx: f32, cy: f32, c: Color) {
    match hit {
        Hit::TabApp => {
            icon_stroke_line(px, cx - 6.0, cy - 4.5, cx + 6.0, cy - 4.5, c, 1.6);
            icon_stroke_line(px, cx - 6.0, cy, cx + 6.0, cy, c, 1.6);
            icon_stroke_line(px, cx - 6.0, cy + 4.5, cx + 6.0, cy + 4.5, c, 1.6);
            fill_circle(px, cx - 2.0, cy - 4.5, 2.1, c);
            fill_circle(px, cx + 2.5, cy, 2.1, c);
            fill_circle(px, cx - 1.0, cy + 4.5, 2.1, c);
        }
        Hit::AppLook => {
            icon_stroke_circle(px, cx, cy, 5.8, c);
            fill_circle(px, cx + 1.6, cy - 1.4, 2.4, c);
        }
        Hit::AppMenus => {
            fill_round(px, cx - 6.0, cy - 5.0, 12.0, 2.0, 1.0, c);
            fill_round(px, cx - 6.0, cy - 1.0, 12.0, 2.0, 1.0, c);
            fill_round(px, cx - 6.0, cy + 3.0, 8.0, 2.0, 1.0, c);
        }
        Hit::AppInstall => {
            if let Some(path) = round_path(cx - 6.2, cy - 2.4, 12.4, 8.0, 1.6) {
                icon_stroke(px, &path, c, 1.5);
            }
            fill_round(px, cx - 4.0, cy - 5.2, 5.6, 3.0, 1.0, c);
        }
        Hit::AppStartup => {
            let mut pb = PathBuilder::new();
            pb.move_to(cx - 3.2, cy - 5.4);
            pb.line_to(cx + 5.2, cy);
            pb.line_to(cx - 3.2, cy + 5.4);
            pb.close();
            if let Some(path) = pb.finish() {
                icon_stroke(px, &path, c, 1.5);
            }
        }
        Hit::AppStream => {
            icon_stroke_circle(px, cx - 2.8, cy, 2.4, c);
            icon_stroke_circle(px, cx + 2.8, cy, 2.4, c);
            icon_stroke_line(px, cx - 0.4, cy, cx + 0.4, cy, c, 1.4);
            let mut wave = PathBuilder::new();
            wave.move_to(cx - 6.4, cy - 3.2);
            wave.quad_to(cx - 4.8, cy, cx - 6.4, cy + 3.2);
            wave.move_to(cx + 6.4, cy - 3.2);
            wave.quad_to(cx + 4.8, cy, cx + 6.4, cy + 3.2);
            if let Some(path) = wave.finish() {
                icon_stroke(px, &path, c, 1.4);
            }
        }
        Hit::AppLabs => {
            let mut pb = PathBuilder::new();
            pb.move_to(cx - 2.4, cy - 6.0);
            pb.line_to(cx + 2.4, cy - 6.0);
            pb.line_to(cx + 5.6, cy + 5.6);
            pb.line_to(cx - 5.6, cy + 5.6);
            pb.close();
            if let Some(path) = pb.finish() {
                icon_stroke(px, &path, c, 1.5);
            }
            icon_stroke_line(px, cx - 2.0, cy - 1.0, cx + 2.0, cy - 1.0, c, 1.4);
        }
        Hit::AppDiagnostics => {
            icon_stroke_line(px, cx - 6.0, cy - 4.2, cx + 6.0, cy - 4.2, c, 1.5);
            icon_stroke_line(px, cx - 6.0, cy, cx + 2.2, cy, c, 1.5);
            icon_stroke_line(px, cx - 6.0, cy + 4.2, cx + 6.0, cy + 4.2, c, 1.5);
        }
        Hit::AppUpdates => {
            icon_stroke_circle(px, cx, cy, 5.6, c);
            let mut up = PathBuilder::new();
            up.move_to(cx, cy - 4.4);
            up.line_to(cx - 3.0, cy - 0.6);
            up.move_to(cx, cy - 4.4);
            up.line_to(cx + 3.0, cy - 0.6);
            up.move_to(cx, cy - 4.4);
            up.line_to(cx, cy + 3.6);
            if let Some(path) = up.finish() {
                icon_stroke(px, &path, c, 1.5);
            }
        }
        Hit::TabFeedback => {
            if let Some(path) = round_path(cx - 6.5, cy - 5.5, 13.0, 9.5, 2.5) {
                icon_stroke(px, &path, c, 1.5);
            }
            let mut pb = PathBuilder::new();
            pb.move_to(cx - 3.0, cy + 4.0);
            pb.line_to(cx - 5.5, cy + 7.5);
            pb.line_to(cx + 0.5, cy + 4.0);
            if let Some(path) = pb.finish() {
                icon_stroke(px, &path, c, 1.5);
            }
        }
        Hit::TabSt => {
            fill_round(px, cx - 6.8, cy + 0.5, 4.0, 6.0, 1.0, c);
            fill_round(px, cx - 2.0, cy - 4.8, 4.0, 11.3, 1.0, c);
            fill_round(px, cx + 2.8, cy + 2.2, 4.0, 4.3, 1.0, c);
        }
        Hit::TabRel => {
            icon_stroke_circle(px, cx, cy - 5.4, 2.2, c);
            fill_circle(px, cx, cy, 2.7, c);
            icon_stroke_circle(px, cx, cy + 5.4, 2.2, c);
        }
        Hit::TabMap => {
            if let Some(path) = round_path(cx - 7.0, cy - 5.2, 14.0, 10.4, 5.0) {
                icon_stroke(px, &path, c, 1.7);
            }
            if let Some(path) = round_path(cx - 3.4, cy - 2.0, 6.8, 4.0, 2.0) {
                icon_stroke(px, &path, c, 1.3);
            }
        }
        Hit::TabMini => {
            icon_stroke_circle(px, cx, cy, 6.8, c);
            let mut pb = PathBuilder::new();
            pb.move_to(cx - 3.8, cy - 1.5);
            pb.quad_to(cx + 0.5, cy - 5.2, cx + 4.2, cy - 0.5);
            if let Some(path) = pb.finish() {
                icon_stroke(px, &path, c, 1.4);
            }
            fill_circle(px, cx + 1.2, cy + 2.2, 1.8, c);
        }
        Hit::TabRadar => {
            fill_round(px, cx - 2.1, cy - 4.2, 4.2, 8.4, 1.4, c);
            let mut nose = PathBuilder::new();
            nose.move_to(cx, cy - 6.6);
            nose.line_to(cx - 2.4, cy - 3.6);
            nose.line_to(cx + 2.4, cy - 3.6);
            nose.close();
            if let Some(path) = nose.finish() {
                let mut p = Paint::default();
                p.set_color(c);
                p.anti_alias = true;
                px.fill_path(&path, &p, FillRule::Winding, Transform::identity(), None);
            }
            let mut left = PathBuilder::new();
            left.move_to(cx - 1.6, cy + 1.5);
            left.line_to(cx - 7.2, cy + 6.6);
            left.line_to(cx - 2.2, cy + 6.6);
            if let Some(path) = left.finish() {
                icon_stroke(px, &path, c, 1.4);
            }
            let mut right = PathBuilder::new();
            right.move_to(cx + 1.6, cy + 1.5);
            right.line_to(cx + 7.2, cy + 6.6);
            right.line_to(cx + 2.2, cy + 6.6);
            if let Some(path) = right.finish() {
                icon_stroke(px, &path, c, 1.4);
            }
        }
        Hit::TabDash => {
            if let Some(path) = round_path(cx - 7.2, cy - 3.6, 4.2, 7.2, 1.2) {
                icon_stroke(px, &path, c, 1.4);
            }
            if let Some(path) = round_path(cx - 2.1, cy - 3.6, 4.2, 7.2, 1.2) {
                icon_stroke(px, &path, c, 1.4);
            }
            if let Some(path) = round_path(cx + 3.0, cy - 3.6, 4.2, 7.2, 1.2) {
                icon_stroke(px, &path, c, 1.4);
            }
        }
        Hit::TabTicker => {
            if let Some(path) = round_path(cx - 7.0, cy - 4.2, 6.2, 8.4, 1.5) {
                icon_stroke(px, &path, c, 1.4);
            }
            fill_round(px, cx - 2.2, cy - 4.2, 6.2, 8.4, 1.5, c);
            if let Some(path) = round_path(cx + 2.6, cy - 4.2, 6.2, 8.4, 1.5) {
                icon_stroke(px, &path, c, 1.4);
            }
        }
        Hit::TabSys => {
            fill_round(px, cx - 7.0, cy - 5.6, 14.0, 2.2, 1.1, c);
            fill_round(px, cx - 7.0, cy - 1.1, 10.0, 2.2, 1.1, c);
            fill_round(px, cx - 7.0, cy + 3.4, 7.0, 2.2, 1.1, c);
        }
        Hit::TabSector => {
            fill_round(px, cx - 6.8, cy - 5.4, 13.6, 3.2, 1.0, c);
            fill_round(px, cx - 6.8, cy - 1.1, 13.6, 3.2, 1.0, c);
            fill_round(px, cx - 6.8, cy + 3.2, 13.6, 3.2, 1.0, c);
        }
        Hit::TabDelta => {
            fill_round(px, cx - 7.0, cy - 1.6, 14.0, 3.2, 1.4, c);
            fill_round(px, cx - 0.7, cy - 5.4, 1.4, 10.8, 0.6, c);
        }
        Hit::TabStance => {
            fill_round(px, cx - 5.4, cy + 1.4, 4.4, 4.2, 1.0, c);
            fill_round(px, cx + 1.0, cy - 5.2, 4.4, 10.8, 1.0, c);
        }
        Hit::TabFlag => {
            icon_stroke_line(px, cx - 5.6, cy - 6.2, cx - 5.6, cy + 6.2, c, 1.6);
            let mut pb = PathBuilder::new();
            pb.move_to(cx - 5.0, cy - 5.8);
            pb.line_to(cx + 6.2, cy - 3.4);
            pb.line_to(cx + 4.4, cy + 0.6);
            pb.line_to(cx - 5.0, cy - 1.2);
            pb.close();
            if let Some(path) = pb.finish() {
                icon_stroke(px, &path, c, 1.4);
            }
        }
        Hit::TabLean => {
            let mut pb = PathBuilder::new();
            pb.move_to(cx - 6.4, cy + 4.2);
            pb.quad_to(cx - 6.0, cy - 6.0, cx, cy - 6.2);
            pb.quad_to(cx + 6.0, cy - 6.0, cx + 6.4, cy + 4.2);
            if let Some(path) = pb.finish() {
                icon_stroke(px, &path, c, 1.5);
            }
            fill_round(px, cx - 1.0, cy - 3.4, 2.0, 8.0, 0.8, c);
        }
        Hit::TabGamepad => {
            fill_round(px, cx - 6.6, cy - 3.2, 13.2, 8.4, 2.4, c);
            fill_circle(
                px,
                cx - 3.2,
                cy + 1.6,
                1.6,
                Color::from_rgba8(12, 12, 16, 255),
            );
            fill_circle(
                px,
                cx + 3.2,
                cy + 1.6,
                1.6,
                Color::from_rgba8(12, 12, 16, 255),
            );
        }
        Hit::TabTelemetry => {
            icon_stroke_line(px, cx - 6.0, cy + 2.0, cx - 2.0, cy - 3.2, c, 1.5);
            icon_stroke_line(px, cx - 2.0, cy - 3.2, cx + 1.4, cy + 3.4, c, 1.5);
            icon_stroke_line(px, cx + 1.4, cy + 3.4, cx + 6.2, cy - 2.4, c, 1.5);
        }
        Hit::TabPitboard => {
            fill_round(px, cx - 7.0, cy - 4.6, 14.0, 9.2, 2.2, c);
            fill_round(px, cx - 3.6, cy - 1.8, 7.2, 3.6, 1.0, Color::from_rgba8(12, 12, 16, 255));
        }
        Hit::QuitApp => {
            icon_stroke_circle(px, cx, cy, 6.2, c);
            icon_stroke_line(px, cx, cy - 7.2, cx, cy - 1.2, c, 1.7);
        }
        Hit::FbRate => fill_star(px, cx, cy, 6.2, c),
        Hit::FbBug => {
            let mut pb = PathBuilder::new();
            pb.move_to(cx, cy - 6.4);
            pb.line_to(cx + 6.6, cy + 5.4);
            pb.line_to(cx - 6.6, cy + 5.4);
            pb.close();
            if let Some(path) = pb.finish() {
                icon_stroke(px, &path, c, 1.5);
            }
            icon_stroke_line(px, cx, cy - 2.2, cx, cy + 1.4, c, 1.6);
            fill_circle(px, cx, cy + 3.4, 1.05, c);
        }
        Hit::FbFeature => {
            icon_stroke_circle(px, cx, cy - 1.4, 4.0, c);
            icon_stroke_line(px, cx, cy + 2.6, cx, cy + 6.2, c, 1.6);
            icon_stroke_line(px, cx - 2.4, cy + 6.2, cx + 2.4, cy + 6.2, c, 1.6);
            icon_stroke_line(px, cx - 1.6, cy + 4.4, cx + 1.6, cy + 4.4, c, 1.5);
        }
        _ => {}
    }
}

pub(crate) fn icon_stroke(px: &mut Pixmap, path: &Path, c: Color, width: f32) {
    let mut p = Paint::default();
    p.set_color(c);
    p.anti_alias = true;
    px.stroke_path(
        path,
        &p,
        &Stroke {
            width,
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            ..Stroke::default()
        },
        Transform::identity(),
        None,
    );
}

pub(crate) fn icon_stroke_line(px: &mut Pixmap, x0: f32, y0: f32, x1: f32, y1: f32, c: Color, width: f32) {
    let mut pb = PathBuilder::new();
    pb.move_to(x0, y0);
    pb.line_to(x1, y1);
    if let Some(path) = pb.finish() {
        icon_stroke(px, &path, c, width);
    }
}

pub(crate) fn icon_stroke_circle(px: &mut Pixmap, cx: f32, cy: f32, r: f32, c: Color) {
    let mut pb = PathBuilder::new();
    pb.push_circle(cx, cy, r);
    if let Some(path) = pb.finish() {
        icon_stroke(px, &path, c, 1.5);
    }
}

pub(crate) fn nav_tab(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    selected: bool,
    visible: bool,
    name: &str,
    hit: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    clip: Option<(f32, f32)>,
) {
    if let Some((top, bot)) = clip {
        if y + h <= top || y >= bot {
            return;
        }
        let hy = y.max(top);
        let hh = (y + h).min(bot) - hy;
        if hh > 1.0 {
            hits.push(HitBox {
                id: hit,
                x,
                y: hy,
                w,
                h: hh,
            });
        }
    } else {
        hits.push(HitBox {
            id: hit,
            x,
            y,
            w,
            h,
        });
    }
    if selected {
        fill_round(px, x, y, w, h, 8.0, tab_on());
        fill_round(px, x, y + 8.0, 3.0, h - 16.0, 1.5, accent());
    } else if hover == Some(hit) {
        fill_round(px, x, y, w, h, 8.0, hover_wash());
    }
    let name_c = if selected { accent() } else { nav_idle() };
    nav_icon(px, hit, x + 18.0, y + h * 0.5, name_c);
    text(px, fonts, name, 13.0, x + 32.0, y + 10.0, name_c, false);
    let dx = x + w - 16.0;
    let dy = y + h * 0.5;
    if visible {
        fill_circle(px, dx, dy, 3.5, accent());
    } else {
        icon_stroke_circle(px, dx, dy, 3.5, muted());
    }
}

/// Settings section rail tab — same chrome as `nav_tab`, without the widget on/off dot.
pub(crate) fn section_nav_tab(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    selected: bool,
    name: &str,
    hit: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) {
    hits.push(HitBox {
        id: hit,
        x,
        y,
        w,
        h,
    });
    if selected {
        fill_round(px, x, y, w, h, 8.0, tab_on());
        fill_round(px, x, y + 8.0, 3.0, h - 16.0, 1.5, accent());
    } else if hover == Some(hit) {
        fill_round(px, x, y, w, h, 8.0, hover_wash());
    }
    let name_c = if selected { accent() } else { nav_idle() };
    nav_icon(px, hit, x + 18.0, y + h * 0.5, name_c);
    text(px, fonts, name, 13.0, x + 32.0, y + 10.0, name_c, false);
}

pub(crate) fn kind_chip(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    hit: Hit,
    on: bool,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) {
    hits.push(HitBox {
        id: hit,
        x,
        y,
        w,
        h: 32.0,
    });
    let col = if on { accent() } else { text_col() };
    if on {
        fill_round(px, x, y, w, 32.0, 8.0, accent_dim());
    } else {
        let fill = if hover == Some(hit) {
            chip_hover()
        } else {
            btn_bg()
        };
        outlined(px, x, y, w, 32.0, 8.0, fill);
    }
    let tw = measure(fonts, label, 13.0);
    let total = 18.0 + tw;
    let start = x + ((w - total) * 0.5).max(8.0);
    nav_icon(px, hit, start + 6.0, y + 16.0, col);
    text(px, fonts, label, 13.0, start + 16.0, y + 8.0, col, false);
}

pub(crate) fn draw_fb_text(
    px: &mut Pixmap,
    fonts: &Fonts,
    s: &str,
    cursor: usize,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) {
    const SIZE: f32 = 12.0;
    const LINE: f32 = 16.0;
    let lines = wrap_fb(fonts, s, w, SIZE);
    let max_rows = (h / LINE).max(1.0) as usize;
    let caret_line = {
        let mut i = 0usize;
        let mut line = 0usize;
        for (li, line_s) in lines.iter().enumerate() {
            let end = i + line_s.len();
            if cursor <= end {
                line = li;
                break;
            }
            i = end;
            if s.get(i..).is_some_and(|r| r.starts_with('\n')) {
                i += 1;
            }
            line = li + 1;
        }
        line
    };
    let start = caret_line.saturating_sub(max_rows.saturating_sub(1));
    let mut drawn = 0usize;
    let mut idx = 0usize;
    let mut rows = Vec::new();
    for (li, line) in lines.iter().enumerate() {
        if li >= start && drawn < max_rows {
            text(
                px,
                fonts,
                line,
                SIZE,
                x,
                y + drawn as f32 * LINE,
                text_col(),
                false,
            );
            let mut stops = vec![(idx, 0.0)];
            let mut prefix = String::new();
            for (off, ch) in line.char_indices() {
                prefix.push(ch);
                stops.push((idx + off + ch.len_utf8(), measure(fonts, &prefix, SIZE)));
            }
            if crate::feedback::caret_on() && li == caret_line {
                let cx = x + stops
                    .iter()
                    .rev()
                    .find(|(b, _)| *b <= cursor)
                    .map(|s| s.1)
                    .unwrap_or(0.0);
                if let Some(r) = Rect::from_xywh(cx, y + drawn as f32 * LINE, 1.5, 14.0) {
                    fill_rect(px, r, accent());
                }
            }
            rows.push(stops);
            drawn += 1;
        }
        idx += line.len();
        if s.get(idx..).is_some_and(|r| r.starts_with('\n')) {
            idx += 1;
        }
    }
    if s.is_empty() {
        rows.push(vec![(0, 0.0)]);
        if crate::feedback::caret_on() {
            if let Some(r) = Rect::from_xywh(x, y, 1.5, 14.0) {
                fill_rect(px, r, accent());
            }
        }
    }
    crate::feedback::set_caret_layout(x, y, LINE, rows);
}

pub(crate) fn wrap_fb(fonts: &Fonts, s: &str, max_w: f32, size: f32) -> Vec<String> {
    let mut lines = Vec::new();
    if s.is_empty() {
        lines.push(String::new());
        return lines;
    }
    for para in s.split('\n') {
        if para.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut line = String::new();
        let mut line_w = 0.0;
        for token in para.split_inclusive(' ') {
            push_wrap_token(
                fonts,
                &mut lines,
                &mut line,
                &mut line_w,
                token,
                max_w,
                size,
            );
        }
        lines.push(line);
    }
    lines
}

pub(crate) fn push_wrap_token(
    fonts: &Fonts,
    lines: &mut Vec<String>,
    line: &mut String,
    line_w: &mut f32,
    token: &str,
    max_w: f32,
    size: f32,
) {
    let token_w = measure(fonts, token, size);
    if !line.is_empty() && *line_w + token_w > max_w {
        lines.push(std::mem::take(line));
        *line_w = 0.0;
    }
    if token_w <= max_w {
        line.push_str(token);
        *line_w += token_w;
        return;
    }
    for ch in token.chars() {
        let ch_w = measure(fonts, ch.encode_utf8(&mut [0; 4]), size);
        if !line.is_empty() && *line_w + ch_w > max_w {
            lines.push(std::mem::take(line));
            *line_w = 0.0;
        }
        line.push(ch);
        *line_w += ch_w;
    }
}

pub(crate) fn fill_star(px: &mut Pixmap, cx: f32, cy: f32, r: f32, c: Color) {
    let mut pb = PathBuilder::new();
    for i in 0..10 {
        let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
        let rad = if i % 2 == 0 { r } else { r * 0.42 };
        let px_ = cx + a.cos() * rad;
        let py = cy + a.sin() * rad;
        if i == 0 {
            pb.move_to(px_, py);
        } else {
            pb.line_to(px_, py);
        }
    }
    pb.close();
    let Some(path) = pb.finish() else {
        return;
    };
    let mut p = Paint::default();
    p.set_color(c);
    p.anti_alias = true;
    px.fill_path(&path, &p, FillRule::Winding, Transform::identity(), None);
}

pub(crate) fn sidebar_quit(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) {
    hits.push(HitBox {
        id: Hit::QuitApp,
        x,
        y,
        w,
        h,
    });
    let fill = if hover == Some(Hit::QuitApp) {
        chip_hover()
    } else {
        btn_bg()
    };
    outlined(px, x, y, w, h, 8.0, fill);
    let c = if hover == Some(Hit::QuitApp) {
        accent()
    } else {
        text_col()
    };
    nav_icon(px, Hit::QuitApp, x + 18.0, y + h * 0.5, c);
    text(
        px,
        fonts,
        "Quit overlay",
        13.0,
        x + 32.0,
        y + 10.0,
        c,
        false,
    );
}

pub(crate) fn action_btn(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    label: &str,
    hit: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    primary: bool,
) {
    hits.push(HitBox {
        id: hit,
        x,
        y,
        w,
        h,
    });
    if primary {
        let fill = if hover == Some(hit) {
            accent_hot()
        } else {
            accent()
        };
        fill_round(px, x, y, w, h, 8.0, fill);
        text(px, fonts, label, 13.0, x + w * 0.5, y + 8.0, ink(), true);
    } else {
        let fill = if hover == Some(hit) {
            chip_hover()
        } else {
            btn_bg()
        };
        outlined(px, x, y, w, h, 8.0, fill);
        text(
            px,
            fonts,
            label,
            13.0,
            x + w * 0.5,
            y + 8.0,
            text_col(),
            true,
        );
    }
}

pub(crate) fn table_style_controls(
    px: &mut Pixmap,
    fonts: &Fonts,
    spec: WidgetPaneSpec,
    cfg: &HudConfig,
    hover: Option<Hit>,
    open_drop: Option<Drop>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
    hl: i32,
    hl_hit: Hit,
    text: TableText,
    text_drop: Drop,
    text_open: Hit,
    text_white: Hit,
    text_black: Hit,
    stripe: bool,
    stripe_hit: Hit,
    plaque_text: TableText,
    plaque_text_drop: Drop,
    plaque_text_open: Hit,
    plaque_text_white: Hit,
    plaque_text_black: Hit,
    show_plaque: bool,
    plaque_hit: Hit,
) -> f32 {
    let mut g = PairGrid::new(x, y, w);
    g.place(|cx, cy, cw| {
        slider_row(
            px,
            fonts,
            cx,
            cy,
            cw,
            "Font size",
            cfg.font_pct(spec.id),
            70,
            160,
            "%",
            Hit::Font(spec.id),
            hover,
            hits,
        )
    });
    g.place(|cx, cy, cw| {
        slider_row(
            px,
            fonts,
            cx,
            cy,
            cw,
            spec.bg_label,
            cfg[spec.id].bg,
            0,
            100,
            "%",
            spec.bg,
            hover,
            hits,
        )
    });
    g.place(|cx, cy, cw| {
        toggle_row(
            px,
            fonts,
            cx,
            cy,
            cw,
            "Bold text",
            cfg.bold(spec.id),
            Hit::Bold(spec.id),
            hover,
            hits,
        )
    });
    g.place(|cx, cy, cw| {
        slider_row(
            px,
            fonts,
            cx,
            cy,
            cw,
            "Row highlight",
            hl,
            0,
            100,
            "%",
            hl_hit,
            hover,
            hits,
        )
    });
    g.place(|cx, cy, cw| {
        dropdown_row(
            px,
            fonts,
            cx,
            cy,
            cw,
            "Text color",
            text.label(),
            open_drop == Some(text_drop),
            text_open,
            &[
                (text_white, "White", text == TableText::White),
                (text_black, "Black", text == TableText::Black),
            ],
            hover,
            hits,
        )
    });
    g.place(|cx, cy, cw| {
        toggle_row(
            px,
            fonts,
            cx,
            cy,
            cw,
            "Alternating rows",
            stripe,
            stripe_hit,
            hover,
            hits,
        )
    });
    g.place(|cx, cy, cw| {
        toggle_row(
            px,
            fonts,
            cx,
            cy,
            cw,
            "Show plaques",
            show_plaque,
            plaque_hit,
            hover,
            hits,
        )
    });
    if show_plaque {
        g.place(|cx, cy, cw| {
            dropdown_row(
                px,
                fonts,
                cx,
                cy,
                cw,
                "Plaque text",
                plaque_text.label(),
                open_drop == Some(plaque_text_drop),
                plaque_text_open,
                &[
                    (plaque_text_white, "White", plaque_text == TableText::White),
                    (plaque_text_black, "Black", plaque_text == TableText::Black),
                ],
                hover,
                hits,
            )
        });
    }
    g.end()
}

pub(crate) fn slots_section(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    title: &str,
    slots: [SlotDrop; 3],
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let y = section(px, fonts, x, y, title);
    let h = 70.0;
    row_card(px, x, y, w, h, false);
    let col = (w - 16.0) / 3.0;
    for (slot, (label, drop)) in ["Left", "Middle", "Right"].iter().zip(slots).enumerate() {
        let cx = x + 8.0 + slot as f32 * col;
        text(px, fonts, label, 11.0, cx + 4.0, y + 8.0, dim(), false);
        let bw = (col - 12.0).max(80.0);
        let bh = 28.0;
        let bx = cx + 4.0;
        let by = y + 30.0;
        hits.push(HitBox {
            id: drop.open_hit,
            x: bx,
            y: by,
            w: bw,
            h: bh,
        });
        let hot = drop.open || hover == Some(drop.open_hit);
        outlined(
            px,
            bx,
            by,
            bw,
            bh,
            7.0,
            if hot { chip_hover() } else { bg() },
        );
        text(
            px,
            fonts,
            drop.value,
            12.0,
            bx + 8.0,
            by + 6.0,
            text_col(),
            false,
        );
        chevron(px, bx + bw - 14.0, by + bh * 0.5, drop.open, muted());
        if drop.open {
            let options = sorted_drop_options(&drop.options);
            let item_h = 28.0;
            let pad = 5.0;
            let content_h = pad * 2.0 + item_h * options.len() as f32;
            DROP_MENUS.with(|menus| {
                menus.borrow_mut().push(PendingDrop {
                    mx: bx,
                    my: by + bh + 6.0,
                    bw,
                    content_h,
                    open_hit: drop.open_hit,
                    options,
                });
            });
        }
    }
    y + h + ROW_GAP
}

pub(crate) fn heading(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    title: &str,
    sub: &str,
    show: Option<(bool, Hit, &'static str)>,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let title_sz = 22.0;
    let h = 48.0;
    let skew = 8.0;
    let ink = ink();
    let mut right_w = 0.0;
    if show.is_some() {
        let label = show.map(|(_, _, label)| label).unwrap_or("");
        let label_w = measure(fonts, label, 13.0);
        right_w = label_w + 14.0 + 52.0 + 4.0;
    }
    let title_w = measure(fonts, title, title_sz);
    let max_plaque = (w - right_w - 12.0).max(72.0);
    let plaque_w = (title_w + 36.0).min(max_plaque);
    fill_skew(px, x, y, (plaque_w - skew).max(48.0), h, skew, accent());
    let label = ellipsize_heading(fonts, title, title_sz, plaque_w - 28.0);
    text(px, fonts, &label, title_sz, x + 16.0, y + 12.0, ink, false);
    if let Some((on, hit, switch_label)) = show {
        let lx = x + w - right_w;
        text(
            px,
            fonts,
            switch_label,
            13.0,
            lx,
            y + 16.0,
            text_col(),
            false,
        );
        hits.push(HitBox {
            id: hit,
            x: lx,
            y,
            w: right_w,
            h,
        });
        switch_lg(
            px,
            lx + measure(fonts, switch_label, 13.0) + 12.0,
            y + (h - 28.0) * 0.5,
            on,
            hit,
            hover,
            hits,
        );
    }
    text(px, fonts, sub, 13.0, x, y + h + 8.0, muted(), false);
    y + h + 8.0 + 46.0
}

pub(crate) fn note_lines(px: &mut Pixmap, fonts: &Fonts, x: f32, y: f32, w: f32, msg: &str) -> f32 {
    let lines = wrap_fb(fonts, msg, (w - 8.0).max(40.0), 12.0);
    let mut y = y;
    for line in &lines {
        text(px, fonts, line, 12.0, x + 4.0, y + 2.0, caution(), false);
        y += 18.0;
    }
    y + 10.0
}

pub(crate) fn ellipsize_heading(fonts: &Fonts, s: &str, size: f32, max_w: f32) -> String {
    if measure(fonts, s, size) <= max_w {
        return s.to_string();
    }
    let dots = if fonts.ui.has_glyph('…') {
        "…"
    } else {
        "..."
    };
    let mut out = String::new();
    for ch in s.chars() {
        let next = format!("{out}{ch}{dots}");
        if measure(fonts, &next, size) > max_w {
            if out.is_empty() {
                return dots.into();
            }
            out.push_str(dots);
            return out;
        }
        out.push(ch);
    }
    out
}

pub(crate) fn fit_path(fonts: &Fonts, path: &str, size: f32, max_w: f32) -> String {
    if measure(fonts, path, size) <= max_w {
        return path.to_string();
    }
    let chars: Vec<char> = path.chars().collect();
    let mut take = chars.len().saturating_sub(3);
    while take >= 8 {
        let left = take / 2;
        let right = take - left;
        let candidate = format!(
            "{}...{}",
            chars[..left].iter().collect::<String>(),
            chars[chars.len() - right..].iter().collect::<String>()
        );
        if measure(fonts, &candidate, size) <= max_w {
            return candidate;
        }
        take -= 1;
    }
    "...".into()
}

pub(crate) fn section(px: &mut Pixmap, fonts: &Fonts, x: f32, y: f32, label: &str) -> f32 {
    text(
        px,
        fonts,
        &label.to_ascii_uppercase(),
        10.0,
        x + 2.0,
        y + 10.0,
        dim(),
        false,
    );
    y + 28.0
}

pub(crate) fn style_controls(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    id: WidgetId,
    cfg: &HudConfig,
    bg_label: &str,
    bg: i32,
    bg_hit: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    if id == WidgetId::Pitboard {
        return y;
    }
    let mut g = PairGrid::new(x, y, w);
    g.place(|cx, cy, cw| {
        slider_row(
            px,
            fonts,
            cx,
            cy,
            cw,
            "Font size",
            cfg.font_pct(id),
            70,
            160,
            "%",
            Hit::Font(id),
            hover,
            hits,
        )
    });
    g.place(|cx, cy, cw| {
        slider_row(
            px, fonts, cx, cy, cw, bg_label, bg, 0, 100, "%", bg_hit, hover, hits,
        )
    });
    g.place(|cx, cy, cw| {
        toggle_row(
            px,
            fonts,
            cx,
            cy,
            cw,
            "Bold text",
            cfg.bold(id),
            Hit::Bold(id),
            hover,
            hits,
        )
    });
    g.end()
}

pub(crate) fn look_section(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    id: WidgetId,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let mut y = section(px, fonts, x, y, "Position on screen");
    let snap_h = 268.0;
    row_card(px, x, y, w, snap_h, false);
    text(
        px,
        fonts,
        "Snap to the monitor this widget is on. Size stays the same.",
        12.0,
        x + 16.0,
        y + 14.0,
        muted(),
        false,
    );
    y += 40.0;
    let cell = 44.0;
    let gap = 6.0;
    let grid = cell * 3.0 + gap * 2.0;
    let gx = x + 16.0;
    let aligns = [
        [SnapAlign::TopLeft, SnapAlign::Top, SnapAlign::TopRight],
        [SnapAlign::Left, SnapAlign::Center, SnapAlign::Right],
        [
            SnapAlign::BottomLeft,
            SnapAlign::Bottom,
            SnapAlign::BottomRight,
        ],
    ];
    for (row, line) in aligns.iter().enumerate() {
        for (col, align) in line.iter().enumerate() {
            let bx = gx + col as f32 * (cell + gap);
            let by = y + row as f32 * (cell + gap);
            snap_cell(px, bx, by, cell, *align, Hit::Snap(id, *align), hover, hits);
        }
    }
    let bar_y = y + grid + 8.0;
    let bar_w = (grid - gap) * 0.5;
    snap_axis(
        px,
        gx,
        bar_y,
        bar_w,
        cell,
        true,
        Hit::Snap(id, SnapAlign::HCenter),
        hover,
        hits,
    );
    snap_axis(
        px,
        gx + bar_w + gap,
        bar_y,
        bar_w,
        cell,
        false,
        Hit::Snap(id, SnapAlign::VCenter),
        hover,
        hits,
    );
    y + grid + cell + 28.0
}

pub(crate) fn snap_cell(
    px: &mut Pixmap,
    x: f32,
    y: f32,
    s: f32,
    align: SnapAlign,
    hit: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) {
    hits.push(HitBox {
        id: hit,
        x,
        y,
        w: s,
        h: s,
    });
    let fill = if hover == Some(hit) {
        chip_hover()
    } else {
        panel()
    };
    outlined(px, x, y, s, s, 8.0, fill);
    let pad = 7.0;
    let (dx, dy) = match align {
        SnapAlign::TopLeft => (pad, pad),
        SnapAlign::Top => (s * 0.5, pad),
        SnapAlign::TopRight => (s - pad, pad),
        SnapAlign::Left => (pad, s * 0.5),
        SnapAlign::Center => (s * 0.5, s * 0.5),
        SnapAlign::Right => (s - pad, s * 0.5),
        SnapAlign::BottomLeft => (pad, s - pad),
        SnapAlign::Bottom => (s * 0.5, s - pad),
        SnapAlign::BottomRight => (s - pad, s - pad),
        _ => (s * 0.5, s * 0.5),
    };
    fill_circle(px, x + dx, y + dy, 3.2, text_col());
}

pub(crate) fn snap_axis(
    px: &mut Pixmap,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    horizontal: bool,
    hit: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) {
    hits.push(HitBox {
        id: hit,
        x,
        y,
        w,
        h,
    });
    let fill = if hover == Some(hit) {
        chip_hover()
    } else {
        panel()
    };
    outlined(px, x, y, w, h, 8.0, fill);
    if horizontal {
        fill_round(
            px,
            x + w * 0.22,
            y + h * 0.5 - 3.0,
            w * 0.56,
            6.0,
            3.0,
            text_col(),
        );
    } else {
        fill_round(
            px,
            x + w * 0.5 - 3.0,
            y + h * 0.18,
            6.0,
            h * 0.64,
            3.0,
            text_col(),
        );
    }
}

pub(crate) fn st_toggle(f: StField) -> Hit {
    match f {
        StField::Pos => Hit::StPos,
        StField::Num => Hit::StNum,
        StField::Name => Hit::StName,
        StField::Gap => Hit::StGap,
        StField::Laps => Hit::StLaps,
        StField::Current => Hit::StCurrent,
        StField::Best => Hit::StBest,
        StField::Last => Hit::StLast,
        StField::Status => Hit::StStatus,
        StField::Bike => Hit::StBike,
        StField::Penalty => Hit::StPenalty,
        StField::Crashed => Hit::StCrashed,
        StField::Interval => Hit::StInterval,
        StField::Category => Hit::StCategory,
        StField::LapDiff => Hit::StLapDiff,
    }
}

pub(crate) fn rel_toggle(f: RelField) -> Hit {
    match f {
        RelField::Num => Hit::RelNum,
        RelField::Name => Hit::RelName,
        RelField::Gap => Hit::RelGap,
        RelField::Laps => Hit::RelLaps,
        RelField::Current => Hit::RelCurrent,
        RelField::Pos => Hit::RelPos,
        RelField::Bike => Hit::RelBike,
        RelField::Penalty => Hit::RelPenalty,
        RelField::Interval => Hit::RelInterval,
        RelField::Status => Hit::RelStatus,
        RelField::Best => Hit::RelBest,
        RelField::Last => Hit::RelLast,
        RelField::Category => Hit::RelCategory,
        RelField::Speed => Hit::RelSpeed,
        RelField::LapDiff => Hit::RelLapDiff,
    }
}

pub(crate) fn field_row(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    on: bool,
    width: i32,
    drag: Hit,
    toggle: Hit,
    wslide: Hit,
    wmax: i32,
    i: usize,
    hover: Option<Hit>,
    col_drag: Option<ColDrag>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let h = ROW_H;
    let cluster = 188.0;
    let grabbed = col_drag.is_some_and(|d| d.from as usize == i);
    let hot = hover == Some(drag) || hover == Some(toggle) || hover == Some(wslide);
    if grabbed {
        fill_round(px, x, y, w, h, 10.0, tab_on());
        fill_round(px, x + 4.0, y + 12.0, 3.0, h - 24.0, 1.5, accent());
    } else {
        row_card(px, x, y, w, h, hot);
    }
    hits.push(HitBox {
        id: drag,
        x,
        y,
        w: (w - cluster).max(48.0),
        h,
    });
    draw_grip(px, x + 12.0, y + h * 0.5);
    let label_c = if on { text_col() } else { muted() };
    text(px, fonts, label, 13.0, x + 38.0, y + 16.0, label_c, false);
    let switch_x = x + w - 54.0;
    let slider_w = 88.0;
    let slider_x = switch_x - 10.0 - slider_w;
    text(
        px,
        fonts,
        &width.to_string(),
        12.0,
        slider_x - 18.0,
        y + 16.0,
        muted(),
        true,
    );
    draw_slider(
        px,
        slider_x,
        y + 16.0,
        slider_w,
        16.0,
        width,
        COL_W_MIN,
        wmax,
        wslide,
        hover,
        hits,
    );
    switch(px, switch_x, y + 14.0, on, toggle, hover, hits);
    y + h + ROW_GAP
}

/// Draw standings/relative column rows with slide animation on reorder.
pub(crate) fn paint_col_field_rows<F>(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    order: &[F],
    kind: DragKind,
    drag: Option<ColDrag>,
    hover: Option<Hit>,
    x: f32,
    base_y: f32,
    w: f32,
    hits: &mut Vec<HitBox>,
    field_id: impl Fn(F) -> i32,
    label: impl Fn(F) -> &'static str,
    enabled: impl Fn(F, &HudConfig) -> bool,
    width: impl Fn(F, &HudConfig) -> i32,
    width_max: impl Fn(F) -> i32,
    drag_hit: impl Fn(u8) -> Hit,
    toggle_hit: impl Fn(F) -> Hit,
    width_hit: impl Fn(u8) -> Hit,
) -> f32
where
    F: Copy + PartialEq,
{
    let n = order.len();
    let stride = col_stride();
    let col_drag = drag.filter(|d| d.kind == kind);
    let preview = match col_drag {
        Some(d) if d.from != d.over => preview_move_to(order, d.from as usize, d.over as usize),
        _ => order.to_vec(),
    };
    let ids: Vec<i32> = preview.iter().copied().map(&field_id).collect();
    let now = col_anim_now();
    let anim = {
        let mut ui = UI.lock().unwrap();
        let Some(ui) = ui.as_mut() else {
            return base_y + n as f32 * stride;
        };
        ui.col_list_y = base_y;
        ui.col_list_n = n;
        ui.col_list_kind = Some(kind);
        let slides = match kind {
            DragKind::St => &mut ui.st_col_slides,
            DragKind::Rel => &mut ui.rel_col_slides,
        };
        slides.indices(&ids, now)
    };
    for (j, field) in preview.iter().copied().enumerate() {
        let i = order.iter().position(|f| *f == field).unwrap_or(j);
        let draw_y = base_y + anim.get(j).copied().unwrap_or(j as f32) * stride;
        let _ = field_row(
            px,
            fonts,
            x,
            draw_y,
            w,
            label(field),
            enabled(field, cfg),
            width(field, cfg),
            drag_hit(i as u8),
            toggle_hit(field),
            width_hit(i as u8),
            width_max(field),
            i,
            hover,
            col_drag,
            hits,
        );
    }
    base_y + n as f32 * stride
}

pub(crate) fn draw_grip(px: &mut Pixmap, x: f32, cy: f32) {
    let c = Color::from_rgba8(108, 108, 116, 255);
    for row in 0..3 {
        for col in 0..2 {
            fill_circle(
                px,
                x + col as f32 * 6.0,
                cy - 6.0 + row as f32 * 6.0,
                1.6,
                c,
            );
        }
    }
}

pub(crate) fn fill_skew(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, skew: f32, c: Color) {
    let mut pb = PathBuilder::new();
    pb.move_to(x + skew, y);
    pb.line_to(x + w + skew, y);
    pb.line_to(x + w, y + h);
    pb.line_to(x, y + h);
    pb.close();
    let Some(path) = pb.finish() else {
        return;
    };
    let mut p = Paint::default();
    p.set_color(c);
    p.anti_alias = true;
    px.fill_path(&path, &p, FillRule::Winding, Transform::identity(), None);
}

pub(crate) fn sys_app_row(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    on: bool,
    removable: bool,
    show_hit: Hit,
    remove_hit: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let h = ROW_H;
    let remove_w = if removable { 72.0 } else { 0.0 };
    let row_w = (w - remove_w).max(80.0);
    hits.push(HitBox {
        id: show_hit,
        x,
        y,
        w: row_w,
        h,
    });
    let hot = hover == Some(show_hit) || hover == Some(remove_hit);
    row_card(px, x, y, w, h, hot);
    text(
        px,
        fonts,
        label,
        13.0,
        x + 16.0,
        y + 16.0,
        text_col(),
        false,
    );
    if removable {
        let bx = x + w - 52.0 - 8.0 - 64.0;
        hits.push(HitBox {
            id: remove_hit,
            x: bx,
            y: y + 10.0,
            w: 64.0,
            h: 28.0,
        });
        let fill = if hover == Some(remove_hit) {
            chip_hover()
        } else {
            btn_bg()
        };
        outlined(px, bx, y + 10.0, 64.0, 28.0, 7.0, fill);
        text(
            px,
            fonts,
            "Remove",
            11.0,
            bx + 32.0,
            y + 16.0,
            text_col(),
            true,
        );
    }
    switch(px, x + w - 52.0, y + 14.0, on, show_hit, hover, hits);
    y + h + ROW_GAP
}

/// Opening / loading image picker: label + status, Browse + Default chips.
pub(crate) fn game_ui_image_row(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    path: &str,
    browse: Hit,
    reset: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let h = ROW_H + 18.0;
    let hot = hover == Some(browse) || hover == Some(reset);
    row_card(px, x, y, w, h, hot);
    text(
        px,
        fonts,
        label,
        13.0,
        x + 16.0,
        y + 12.0,
        text_col(),
        false,
    );
    let status = if path.trim().is_empty() {
        "Default".to_string()
    } else {
        std::path::Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string())
    };
    text(px, fonts, &status, 11.0, x + 16.0, y + 34.0, dim(), false);
    let btn_w = 72.0;
    let gap = 8.0;
    let def_x = x + w - 16.0 - btn_w;
    let br_x = def_x - gap - btn_w;
    for (hit, bx, caption) in [(browse, br_x, "Browse"), (reset, def_x, "Default")] {
        hits.push(HitBox {
            id: hit,
            x: bx,
            y: y + 14.0,
            w: btn_w,
            h: 28.0,
        });
        let fill = if hover == Some(hit) {
            chip_hover()
        } else {
            btn_bg()
        };
        outlined(px, bx, y + 14.0, btn_w, 28.0, 7.0, fill);
        text(
            px,
            fonts,
            caption,
            11.0,
            bx + btn_w * 0.5,
            y + 20.0,
            text_col(),
            true,
        );
    }
    y + h + ROW_GAP
}

pub(crate) fn browse_exe(host: HWND) -> Option<String> {
    use windows::core::w;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::{
        FileOpenDialog, IFileOpenDialog, FOS_FILEMUSTEXIST, FOS_FORCEFILESYSTEM, FOS_PATHMUSTEXIST,
        SIGDN_FILESYSPATH,
    };
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let dlg: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_ALL).ok()?;
        dlg.SetOptions(FOS_FILEMUSTEXIST | FOS_PATHMUSTEXIST | FOS_FORCEFILESYSTEM)
            .ok()?;
        dlg.SetTitle(w!("Select an app to watch")).ok()?;
        dlg.Show(host).ok()?;
        let item = dlg.GetResult().ok()?;
        let name = item.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
        let path = name.to_string().ok().filter(|s| !s.is_empty());
        CoTaskMemFree(Some(name.0 as _));
        let path = path?;
        let file = std::path::Path::new(&path)
            .file_name()?
            .to_string_lossy()
            .into_owned();
        if !file.to_ascii_lowercase().ends_with(".exe") {
            let _ = windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
                host,
                w!("Pick an .exe file."),
                w!("Holeshot HUD"),
                windows::Win32::UI::WindowsAndMessaging::MB_OK
                    | windows::Win32::UI::WindowsAndMessaging::MB_ICONWARNING,
            );
            return None;
        }
        Some(file)
    }
}

pub(crate) fn browse_image(host: HWND) -> Option<String> {
    use windows::core::w;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
    use windows::Win32::UI::Shell::{
        FileOpenDialog, IFileOpenDialog, FOS_FILEMUSTEXIST, FOS_FORCEFILESYSTEM, FOS_PATHMUSTEXIST,
        SIGDN_FILESYSPATH,
    };
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let dlg: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_ALL).ok()?;
        dlg.SetOptions(FOS_FILEMUSTEXIST | FOS_PATHMUSTEXIST | FOS_FORCEFILESYSTEM)
            .ok()?;
        dlg.SetTitle(w!("Select an image")).ok()?;
        let filters = [COMDLG_FILTERSPEC {
            pszName: w!("Images (PNG, JPG)"),
            pszSpec: w!("*.png;*.jpg;*.jpeg"),
        }];
        let _ = dlg.SetFileTypes(&filters);
        let _ = dlg.SetFileTypeIndex(1);
        dlg.Show(host).ok()?;
        let item = dlg.GetResult().ok()?;
        let name = item.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
        let path = name.to_string().ok().filter(|s| !s.is_empty());
        CoTaskMemFree(Some(name.0 as _));
        let path = path?;
        let lower = path.to_ascii_lowercase();
        if !(lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg")) {
            let _ = windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
                host,
                w!("Pick a .png, .jpg, or .jpeg file."),
                w!("Holeshot HUD"),
                windows::Win32::UI::WindowsAndMessaging::MB_OK
                    | windows::Win32::UI::WindowsAndMessaging::MB_ICONWARNING,
            );
            return None;
        }
        Some(path)
    }
}

pub(crate) fn browse_json(host: HWND) -> Option<String> {
    use windows::core::w;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
    use windows::Win32::UI::Shell::{
        FileOpenDialog, IFileOpenDialog, FOS_FILEMUSTEXIST, FOS_FORCEFILESYSTEM, FOS_PATHMUSTEXIST,
        SIGDN_FILESYSPATH,
    };
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let dlg: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_ALL).ok()?;
        dlg.SetOptions(FOS_FILEMUSTEXIST | FOS_PATHMUSTEXIST | FOS_FORCEFILESYSTEM)
            .ok()?;
        dlg.SetTitle(w!("Import pit board")).ok()?;
        let filters = [COMDLG_FILTERSPEC {
            pszName: w!("Pit board (board.json)"),
            pszSpec: w!("*.json"),
        }];
        let _ = dlg.SetFileTypes(&filters);
        let _ = dlg.SetFileTypeIndex(1);
        dlg.Show(host).ok()?;
        let item = dlg.GetResult().ok()?;
        let name = item.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
        let path = name.to_string().ok().filter(|s| !s.is_empty());
        CoTaskMemFree(Some(name.0 as _));
        let path = path?;
        if !path.to_ascii_lowercase().ends_with(".json") {
            let _ = windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
                host,
                w!("Pick a board.json file."),
                w!("Holeshot HUD"),
                windows::Win32::UI::WindowsAndMessaging::MB_OK
                    | windows::Win32::UI::WindowsAndMessaging::MB_ICONWARNING,
            );
            return None;
        }
        Some(path)
    }
}

const DEMO_PIT_PNG: &[u8] =
    include_bytes!("../../../pit-text-designs/chevron-frame/chevron-frame.png");

pub(crate) fn pick_demo_png(host: HWND) -> Option<String> {
    use windows::core::w;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
    use windows::Win32::UI::Shell::{
        FileSaveDialog, IFileSaveDialog, FOS_FORCEFILESYSTEM, FOS_OVERWRITEPROMPT, FOS_PATHMUSTEXIST,
        SIGDN_FILESYSPATH,
    };
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let dlg: IFileSaveDialog = CoCreateInstance(&FileSaveDialog, None, CLSCTX_ALL).ok()?;
        dlg.SetOptions(FOS_OVERWRITEPROMPT | FOS_PATHMUSTEXIST | FOS_FORCEFILESYSTEM)
            .ok()?;
        dlg.SetTitle(w!("Save demo pit board")).ok()?;
        let filters = [COMDLG_FILTERSPEC {
            pszName: w!("PNG"),
            pszSpec: w!("*.png"),
        }];
        let _ = dlg.SetFileTypes(&filters);
        let _ = dlg.SetFileTypeIndex(1);
        let _ = dlg.SetDefaultExtension(w!("png"));
        let _ = dlg.SetFileName(w!("holeshot-demo.png"));
        dlg.Show(host).ok()?;
        let item = dlg.GetResult().ok()?;
        let name = item.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
        let path = name.to_string().ok().filter(|s| !s.is_empty());
        CoTaskMemFree(Some(name.0 as _));
        path
    }
}

pub(crate) fn write_demo_png(path: &str) -> Result<String, String> {
    let mut path = std::path::PathBuf::from(path);
    let ext = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
    if !ext.eq_ignore_ascii_case("png") {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("holeshot-demo");
        path.set_file_name(format!("{name}.png"));
    }
    std::fs::write(&path, DEMO_PIT_PNG).map_err(|_| "Could not save the demo pit board.".to_string())?;
    Ok(path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("holeshot-demo.png")
        .to_string())
}

pub(crate) fn pick_export_folder(host: HWND) -> Option<String> {
    use windows::core::w;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::{
        FileOpenDialog, IFileOpenDialog, FOS_FORCEFILESYSTEM, FOS_PICKFOLDERS, SIGDN_FILESYSPATH,
    };
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let dlg: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_ALL).ok()?;
        dlg.SetOptions(FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM).ok()?;
        dlg.SetTitle(w!("Export pit board")).ok()?;
        dlg.Show(host).ok()?;
        let item = dlg.GetResult().ok()?;
        let name = item.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
        let path = name.to_string().ok().filter(|s| !s.is_empty());
        CoTaskMemFree(Some(name.0 as _));
        path
    }
}

pub(crate) fn confirm_delete_board(host: HWND, board: &str) -> bool {
    use windows::core::PCWSTR;
    use windows::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, IDYES, MB_DEFBUTTON2, MB_ICONWARNING, MB_YESNO,
    };
    let prompt: Vec<u16> = format!("Delete the pit board \"{board}\"? This removes its folder.")
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let title: Vec<u16> = "Holeshot HUD"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        MessageBoxW(
            host,
            PCWSTR(prompt.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_YESNO | MB_ICONWARNING | MB_DEFBUTTON2,
        ) == IDYES
    }
}

pub(crate) fn bind_row(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    value: &str,
    listen: bool,
    hit: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) -> f32 {
    if listen {
        let h = 64.0;
        hits.push(HitBox {
            id: hit,
            x,
            y,
            w,
            h,
        });
        fill_round(px, x, y, w, h, 10.0, accent());
        let max_w = (w - 32.0).max(40.0);
        let title = ellipsize_heading(fonts, "Press a button now", 16.0, max_w);
        let sub = ellipsize_heading(fonts, "Pad, key, or mouse  ·  Esc cancels", 12.0, max_w);
        text(px, fonts, &title, 16.0, x + 16.0, y + 12.0, ink(), false);
        text(px, fonts, &sub, 12.0, x + 16.0, y + 36.0, ink(), false);
        return y + h + ROW_GAP;
    }
    let h = ROW_H;
    hits.push(HitBox {
        id: hit,
        x,
        y,
        w,
        h,
    });
    row_card(px, x, y, w, h, hover == Some(hit));
    text(
        px,
        fonts,
        label,
        13.0,
        x + 16.0,
        y + 16.0,
        text_col(),
        false,
    );
    let label_w = measure(fonts, label, 13.0);
    let bw = (w - 30.0 - label_w - 16.0).clamp(108.0, 176.0);
    let bh = 28.0;
    let bx = x + w - bw - 14.0;
    let by = y + 10.0;
    hits.push(HitBox {
        id: hit,
        x: bx,
        y: by,
        w: bw,
        h: bh,
    });
    let hot = hover == Some(hit);
    outlined(
        px,
        bx,
        by,
        bw,
        bh,
        7.0,
        if hot { chip_hover() } else { bg() },
    );
    text(
        px,
        fonts,
        value,
        12.0,
        bx + 10.0,
        by + 6.0,
        text_col(),
        false,
    );
    y + h + ROW_GAP
}

pub(crate) fn paint_drop_menus(px: &mut Pixmap, fonts: &Fonts, hover: Option<Hit>, hits: &mut Vec<HitBox>) {
    let menus = DROP_MENUS.with(|menus| std::mem::take(&mut *menus.borrow_mut()));
    let (mut drop_scroll, mut drop_menu) = {
        let ui = UI.lock().unwrap();
        let ui = ui.as_ref();
        (
            ui.map(|u| u.drop_scroll).unwrap_or(0.0),
            None::<(f32, f32, f32, f32, f32)>,
        )
    };
    let item_h = 28.0;
    let pad = 5.0;
    let max_visible = 9.0;
    let win_h = px.height() as f32;

    for menu in menus {
        let mx = menu.mx;
        let my = menu.my;
        let bw = menu.bw;
        let content_h = menu.content_h;
        let space_below = (win_h - my - 10.0).max(item_h + pad * 2.0);
        let view_h = content_h
            .min(pad * 2.0 + item_h * max_visible)
            .min(space_below);
        let max_scroll = (content_h - view_h).max(0.0);
        drop_scroll = drop_scroll.clamp(0.0, max_scroll);
        drop_menu = Some((mx, my, bw, view_h, content_h));

        hits.push(HitBox {
            id: menu.open_hit,
            x: mx,
            y: my,
            w: bw,
            h: view_h,
        });
        fill_round(
            px,
            mx - 1.0,
            my - 1.0,
            bw + 2.0,
            view_h + 2.0,
            10.0,
            shadow(),
        );
        outlined(px, mx, my, bw, view_h, 9.0, menu_fill());

        let view_top = my + pad;
        let view_bot = my + view_h - pad;
        for (i, (hit, name, selected)) in menu.options.iter().enumerate() {
            let iy = my + pad + i as f32 * item_h - drop_scroll;
            if iy + item_h <= view_top || iy >= view_bot {
                continue;
            }
            let hit_y = iy.max(my);
            let hit_b = (iy + item_h).min(my + view_h);
            if hit_b > hit_y {
                hits.push(HitBox {
                    id: *hit,
                    x: mx,
                    y: hit_y,
                    w: bw,
                    h: hit_b - hit_y,
                });
            }
            if hover == Some(*hit) {
                fill_round(px, mx + 5.0, iy, bw - 10.0, item_h, 6.0, chip_hover());
            } else if *selected {
                fill_round(px, mx + 5.0, iy, bw - 10.0, item_h, 6.0, accent_dim());
            }
            let col = if *selected { accent() } else { text_col() };
            text(px, fonts, name, 12.0, mx + 12.0, iy + 6.0, col, false);
        }

        // Cover padding so partially scrolled rows don't spill into the chrome.
        let fill = menu_fill();
        if let Some(r) = Rect::from_xywh(mx + 2.0, my + 2.0, bw - 4.0, (pad - 1.0).max(1.0)) {
            fill_rect(px, r, fill);
        }
        if let Some(r) =
            Rect::from_xywh(mx + 2.0, my + view_h - pad, bw - 4.0, (pad - 1.0).max(1.0))
        {
            fill_rect(px, r, fill);
        }

        if max_scroll > 0.5 {
            let track_h = (view_h - 12.0).max(8.0);
            let thumb_h = (view_h / content_h * track_h).clamp(12.0, track_h);
            let thumb_y = my + 6.0 + (drop_scroll / max_scroll) * (track_h - thumb_h);
            fill_round(px, mx + bw - 7.0, my + 6.0, 3.0, track_h, 1.5, menu_edge());
            fill_round(
                px,
                mx + bw - 7.0,
                thumb_y,
                3.0,
                thumb_h,
                1.5,
                menu_edge_strong(),
            );
        }
    }

    if let Some(ui) = UI.lock().unwrap().as_mut() {
        ui.drop_scroll = drop_scroll;
        ui.drop_menu = drop_menu;
    }
}

pub(crate) fn paint_color_picker(
    px: &mut Pixmap,
    fonts: &Fonts,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    kind: ColorPickKind,
    rgb: [u8; 3],
    reset_to: [u8; 3],
) {
    let Some((bx, by, bw, bh)) = COLOR_PICKER_ANCHOR.with(|a| a.get()) else {
        return;
    };
    let (hue, sat, val) = rgb_to_hsv(rgb);
    let pad = 12.0;
    let picker_w = 240.0;
    let sw = 22.0;
    let sv_h = 132.0;
    let hue_h = 16.0;
    let foot_h = 22.0;
    let show_reset = rgb != reset_to;
    let content_h = pad + sw + 10.0 + sv_h + 10.0 + hue_h + 10.0 + foot_h + pad;
    let win_w = px.width() as f32;
    let win_h = px.height() as f32;
    let mut mx = bx + bw - picker_w;
    if mx < 16.0 {
        mx = 16.0;
    }
    if mx + picker_w > win_w - 10.0 {
        mx = (win_w - 10.0 - picker_w).max(10.0);
    }
    let mut my = by + bh + 6.0;
    if my + content_h > win_h - 10.0 {
        my = (by - 6.0 - content_h).max(10.0);
    }

    hits.push(HitBox {
        id: kind.panel(),
        x: mx,
        y: my,
        w: picker_w,
        h: content_h,
    });
    fill_round(
        px,
        mx - 1.0,
        my - 1.0,
        picker_w + 2.0,
        content_h + 2.0,
        10.0,
        shadow(),
    );
    outlined(px, mx, my, picker_w, content_h, 9.0, menu_fill());

    let inner_x = mx + pad;
    let inner_w = picker_w - pad * 2.0;
    let mut cy = my + pad;
    let n = PRIMARY_SWATCHES.len();
    let gap = ((inner_w - sw * n as f32) / (n as f32 - 1.0)).max(4.0);
    for (i, chip) in PRIMARY_SWATCHES.iter().copied().enumerate() {
        let sx = inner_x + i as f32 * (sw + gap);
        let hit = kind.swatch(i as u8);
        hits.push(HitBox {
            id: hit,
            x: sx,
            y: cy,
            w: sw,
            h: sw,
        });
        fill_round(
            px,
            sx,
            cy,
            sw,
            sw,
            6.0,
            Color::from_rgba8(chip[0], chip[1], chip[2], 255),
        );
        if chip == rgb || hover == Some(hit) {
            if let Some(path) = round_path(sx + 1.0, cy + 1.0, sw - 2.0, sw - 2.0, 5.0) {
                let mut p = Paint::default();
                p.set_color(Color::from_rgba8(248, 248, 250, 220));
                p.anti_alias = true;
                px.stroke_path(
                    &path,
                    &p,
                    &Stroke {
                        width: 2.0,
                        ..Stroke::default()
                    },
                    Transform::identity(),
                    None,
                );
            }
        }
    }

    cy += sw + 10.0;
    let sv_x = inner_x;
    let sv_y = cy;
    let sv_hit = kind.saturation_value();
    hits.push(HitBox {
        id: sv_hit,
        x: sv_x,
        y: sv_y,
        w: inner_w,
        h: sv_h,
    });
    paint_sv_square(px, sv_x, sv_y, inner_w, sv_h, hue);
    let cx = sv_x + sat * inner_w;
    let cy_sv = sv_y + (1.0 - val) * sv_h;
    fill_circle(px, cx, cy_sv, 6.0, shadow());
    fill_circle(px, cx, cy_sv, 5.0, Color::from_rgba8(248, 248, 250, 255));
    fill_circle(
        px,
        cx,
        cy_sv,
        3.2,
        Color::from_rgba8(rgb[0], rgb[1], rgb[2], 255),
    );

    cy += sv_h + 10.0;
    let hue_hit = kind.hue();
    hits.push(HitBox {
        id: hue_hit,
        x: inner_x,
        y: cy,
        w: inner_w,
        h: hue_h,
    });
    paint_hue_bar(px, inner_x, cy, inner_w, hue_h);
    let t = (hue / 360.0).clamp(0.0, 1.0);
    let kx = inner_x + inner_w * t;
    let kr = if hover == Some(hue_hit) { 7.0 } else { 6.0 };
    fill_circle(
        px,
        kx,
        cy + hue_h * 0.5 + 1.0,
        kr + 1.0,
        Color::from_rgba8(0, 0, 0, 70),
    );
    fill_circle(px, kx, cy + hue_h * 0.5, kr, knob());

    cy += hue_h + 10.0;
    text(
        px,
        fonts,
        &format_primary_color(rgb),
        12.0,
        inner_x,
        cy + 2.0,
        muted(),
        false,
    );
    if show_reset {
        let label = "Reset";
        let rw = measure(fonts, label, 12.0) + 16.0;
        let rx = mx + picker_w - pad - rw;
        let reset_hit = kind.reset();
        hits.push(HitBox {
            id: reset_hit,
            x: rx,
            y: cy,
            w: rw,
            h: foot_h,
        });
        let hot = hover == Some(reset_hit);
        fill_round(
            px,
            rx,
            cy,
            rw,
            foot_h,
            6.0,
            if hot { chip_hover() } else { hover_wash() },
        );
        text(
            px,
            fonts,
            label,
            12.0,
            rx + 8.0,
            cy + 4.0,
            text_col(),
            false,
        );
    }

    if let Some(ui) = UI.lock().unwrap().as_mut() {
        ui.drop_menu = Some((mx, my, picker_w, content_h, content_h));
    }
}

pub(crate) fn paint_sv_square(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, hue: f32) {
    let hue_c = {
        let [r, g, b] = hsv_to_rgb(hue, 1.0, 1.0);
        Color::from_rgba8(r, g, b, 255)
    };
    let Some(path) = round_path(x, y, w, h, 7.0) else {
        return;
    };
    if let Some(shader) = LinearGradient::new(
        SkPoint::from_xy(x, y),
        SkPoint::from_xy(x + w, y),
        vec![
            GradientStop::new(0.0, Color::from_rgba8(255, 255, 255, 255)),
            GradientStop::new(1.0, hue_c),
        ],
        SpreadMode::Pad,
        Transform::identity(),
    ) {
        let mut paint = Paint::default();
        paint.shader = shader;
        paint.anti_alias = true;
        px.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
    if let Some(shader) = LinearGradient::new(
        SkPoint::from_xy(x, y),
        SkPoint::from_xy(x, y + h),
        vec![
            GradientStop::new(0.0, Color::from_rgba8(0, 0, 0, 0)),
            GradientStop::new(1.0, Color::from_rgba8(0, 0, 0, 255)),
        ],
        SpreadMode::Pad,
        Transform::identity(),
    ) {
        let mut paint = Paint::default();
        paint.shader = shader;
        paint.anti_alias = true;
        px.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
}

pub(crate) fn paint_hue_bar(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32) {
    let stops: Vec<GradientStop> = [0.0, 60.0, 120.0, 180.0, 240.0, 300.0, 360.0]
        .into_iter()
        .map(|deg| {
            let [r, g, b] = hsv_to_rgb(deg, 1.0, 1.0);
            GradientStop::new(deg / 360.0, Color::from_rgba8(r, g, b, 255))
        })
        .collect();
    let Some(path) = round_path(x, y, w, h, h * 0.5) else {
        return;
    };
    if let Some(shader) = LinearGradient::new(
        SkPoint::from_xy(x, y),
        SkPoint::from_xy(x + w, y),
        stops,
        SpreadMode::Pad,
        Transform::identity(),
    ) {
        let mut paint = Paint::default();
        paint.shader = shader;
        paint.anti_alias = true;
        px.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
}

pub(crate) unsafe fn present(hwnd: HWND, px: &Pixmap) {
    let hdc = GetDC(hwnd);
    if hdc.is_invalid() {
        return;
    }
    let w = px.width() as i32;
    let h = px.height() as i32;
    let rgba = px.data();
    let mut bgra = vec![0u8; rgba.len()];
    for i in (0..rgba.len()).step_by(4) {
        bgra[i] = rgba[i + 2];
        bgra[i + 1] = rgba[i + 1];
        bgra[i + 2] = rgba[i];
        bgra[i + 3] = rgba[i + 3];
    }
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0 as u32,
            ..Default::default()
        },
        ..Default::default()
    };
    let _ = SetDIBitsToDevice(
        hdc,
        0,
        0,
        w as u32,
        h as u32,
        0,
        0,
        0,
        h as u32,
        bgra.as_ptr() as *const core::ffi::c_void,
        &info,
        DIB_RGB_COLORS,
    );
    let _ = ReleaseDC(hwnd, hdc);
}

pub(crate) unsafe fn set_titlebar_dark(hwnd: HWND, dark: bool) {
    let on = BOOL(if dark { 1 } else { 0 });
    let _ = windows::Win32::Graphics::Dwm::DwmSetWindowAttribute(
        hwnd,
        windows::Win32::Graphics::Dwm::DWMWINDOWATTRIBUTE(20),
        &on as *const BOOL as *const core::ffi::c_void,
        std::mem::size_of::<BOOL>() as u32,
    );
}

#[cfg(test)]
#[path = "../tests/settings.rs"]
mod tests;
