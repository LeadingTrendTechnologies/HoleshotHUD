mod demo_track;
mod edit;

use mxbo_hud::config::{
    BoardField, DashField, DotLabel, FontFamily, HudConfig, SnapAlign, TableText, Units, WidgetId,
};
use mxbo_hud::render::{draw, Fonts};
use mxbo_hud::snapshot::{
    write_name, Point, Rider, Snapshot, Standing, MAGIC, MAX_POLY, VERSION,
};
use tiny_skia::{Color, Paint, PathBuilder, Pixmap, Stroke, Transform};
use wasm_bindgen::prelude::*;

const RIDERS: &[(&str, &str, &str)] = &[
    ("Eli Tomac", "YZ450F", "MX1"),
    ("Jett Lawrence", "CRF450R", "MX1"),
    ("Chase Sexton", "FC 450", "MX1"),
    ("Cooper Webb", "450 SX-F", "MX1"),
    ("Aaron Plessinger", "KX450", "MX1"),
    ("Ken Roczen", "RM-Z450", "MX1"),
    ("Justin Barcia", "MC 450", "MX1"),
    ("Dylan Ferrandis", "YZ450F", "MX1"),
    ("Hunter Lawrence", "CRF450R", "MX1"),
    ("Jason Anderson", "KX450", "MX1"),
    ("You", "FC 450", "MX1"),
    ("Haiden Deegan", "YZ250F", "MX2"),
];

const FOCUS: i32 = 11;

const DEMO_S1_MS: i32 = 24_093;
const DEMO_S2_MS: i32 = 25_760;
const DEMO_S3_MS: i32 = 23_090;
const DEMO_S1_END: f32 = 0.31;
const DEMO_S2_END: f32 = 0.64;

#[wasm_bindgen]
pub struct Preview {
    fonts: Fonts,
    cfg: HudConfig,
    snap: Snapshot,
    t: f32,
    active: String,
    drag: Option<edit::Drag>,
    placed: Vec<String>,
    layout_edit: bool,
}

#[wasm_bindgen]
impl Preview {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<Preview, JsValue> {
        let fonts = Fonts::for_family(FontFamily::Exo2)
            .or_else(Fonts::load)
            .ok_or_else(|| JsValue::from_str("failed to load fonts"))?;
        let mut cfg = HudConfig::new();
        cfg.font_family = FontFamily::Exo2;
        cfg.units = Units::Imperial;
        show_only(&mut cfg, "standings");
        center_widget(&mut cfg, "standings");
        let mut snap = demo_snapshot();
        cfg.apply_to_snapshot(&mut snap);
        Ok(Self {
            fonts,
            cfg,
            snap,
            t: 0.0,
            active: "standings".into(),
            drag: None,
            placed: vec!["standings".into()],
            layout_edit: false,
        })
    }

    pub fn select_widget(&mut self, name: &str) {
        if edit::parse_target(name).is_none() {
            return;
        }
        self.active = name.to_string();
        show_only(&mut self.cfg, name);
        if !self.placed.iter().any(|w| w == name) {
            center_widget(&mut self.cfg, name);
            self.placed.push(name.to_string());
        }
        self.cfg.apply_to_snapshot(&mut self.snap);
        sync_delta_preview(&self.active, self.t);
        self.drag = None;
    }

    pub fn active_widget(&self) -> String {
        self.active.clone()
    }

    pub fn set_widget(&mut self, name: &str, on: bool) {
        if on {
            self.select_widget(name);
        }
    }

    pub fn widget_on(&self, name: &str) -> bool {
        self.active == name
    }

    pub fn get_bool(&self, key: &str) -> bool {
        if key == "layout_edit" {
            return self.layout_edit;
        }
        flag(&self.cfg, key).unwrap_or(false)
    }

    pub fn set_bool(&mut self, key: &str, on: bool) {
        if key == "layout_edit" {
            self.layout_edit = on;
            if !on {
                self.drag = None;
            }
            return;
        }
        set_flag(&mut self.cfg, key, on);
        if key == "dash_simple" {
            size_demo_dash(&mut self.cfg);
        }
        self.cfg.apply_to_snapshot(&mut self.snap);
    }

    pub fn get_int(&self, key: &str) -> i32 {
        int_val(&self.cfg, key).unwrap_or(0)
    }

    pub fn set_int(&mut self, key: &str, value: i32) {
        set_int(&mut self.cfg, key, value);
        self.cfg.apply_to_snapshot(&mut self.snap);
    }

    pub fn get_field(&self, key: &str) -> String {
        field_val(&self.cfg, key).unwrap_or_default()
    }

    pub fn set_field(&mut self, key: &str, value: &str) {
        set_field(&mut self.cfg, key, value);
        self.cfg.apply_to_snapshot(&mut self.snap);
    }

    pub fn hover_cursor(&self, nx: f32, ny: f32, width: u32, height: u32) -> String {
        if !self.layout_edit {
            return String::new();
        }
        let Some(t) = edit::parse_target(&self.active) else {
            return String::new();
        };
        edit::hit(&self.cfg, t, nx, ny, width as f32, height as f32)
            .map(|h| h.cursor().to_string())
            .unwrap_or_default()
    }

    pub fn pointer_down(&mut self, nx: f32, ny: f32, width: u32, height: u32) {
        if !self.layout_edit {
            return;
        }
        let Some(t) = edit::parse_target(&self.active) else {
            return;
        };
        let Some(handle) = edit::hit(&self.cfg, t, nx, ny, width as f32, height as f32) else {
            return;
        };
        self.drag = Some(edit::Drag {
            target: t,
            handle,
            grab_x: nx,
            grab_y: ny,
            orig: edit::rect_of(&self.cfg, t),
        });
    }

    pub fn pointer_move(&mut self, nx: f32, ny: f32, width: u32, height: u32) {
        let Some(d) = self.drag else {
            return;
        };
        let r = edit::resize(
            d.orig,
            d.handle,
            nx,
            ny,
            d.grab_x,
            d.grab_y,
            width as f32,
            height as f32,
            d.target,
        );
        edit::set_rect(&mut self.cfg, d.target, r);
        self.cfg.apply_to_snapshot(&mut self.snap);
    }

    pub fn pointer_up(&mut self) {
        self.drag = None;
    }

    pub fn snap_widget(&mut self, align: &str) {
        let Some(id) = widget_id(&self.active) else {
            return;
        };
        let align = match align {
            "tl" => SnapAlign::TopLeft,
            "t" => SnapAlign::Top,
            "tr" => SnapAlign::TopRight,
            "l" => SnapAlign::Left,
            "c" => SnapAlign::Center,
            "r" => SnapAlign::Right,
            "bl" => SnapAlign::BottomLeft,
            "b" => SnapAlign::Bottom,
            "br" => SnapAlign::BottomRight,
            _ => return,
        };
        self.cfg.snap(id, align);
        self.cfg.apply_to_snapshot(&mut self.snap);
    }

    pub fn tick(&mut self, dt: f32) {
        self.t += dt.max(0.0).min(0.08);
        animate(&mut self.snap, self.t, dt.max(0.0).min(0.08));
        self.cfg.apply_to_snapshot(&mut self.snap);
        sync_delta_preview(&self.active, self.t);
        sync_flag_preview(&self.active, self.t, self.cfg.flag_yellow, self.cfg.flag_blue);
    }

    pub fn frame(&mut self, width: u32, height: u32) -> Vec<u8> {
        let w = width.clamp(320, 1920);
        let h = height.clamp(180, 1080);
        let mut px = Pixmap::new(w, h).expect("pixmap");
        if self.active == "sys" {
            let t = self.t;
            mxbo_hud::set_sys_stats(
                42.0 + (t * 0.7).sin() * 8.0,
                61.0 + (t * 0.25).sin() * 3.0,
                89.0 + (t * 1.4).sin() * 6.0,
                14.0 + (t * 0.9).sin() * 7.0,
            );
            mxbo_hud::set_sys_procs([
                mxbo_hud::SysProc {
                    cpu: 3.0 + (t * 0.9).sin() * 1.2,
                    mem_mb: 92.0 + (t * 0.4).sin() * 6.0,
                    mem_pct: 0.6,
                    on: true,
                },
                mxbo_hud::SysProc {
                    cpu: 18.0 + (t * 0.55).sin() * 4.0,
                    mem_mb: 2100.0 + (t * 0.2).sin() * 80.0,
                    mem_pct: 13.0,
                    on: true,
                },
                mxbo_hud::SysProc {
                    cpu: 6.0 + (t * 0.8).sin() * 2.0,
                    mem_mb: 190.0 + (t * 0.35).sin() * 12.0,
                    mem_pct: 1.2,
                    on: true,
                },
                mxbo_hud::SysProc {
                    cpu: -1.0,
                    mem_mb: 48.0 + (t * 0.3).sin() * 4.0,
                    mem_pct: 0.3,
                    on: true,
                },
            ]);
        }
        draw(
            &mut px,
            &self.fonts,
            Some(&self.snap),
            &self.cfg,
            w,
            h,
            0.0,
            false,
            false,
            false,
        );
        if self.layout_edit {
            if let Some(t) = edit::parse_target(&self.active) {
                draw_edit_frame(
                    &mut px,
                    edit::edit_rect(&self.cfg, t, w as f32, h as f32),
                    w as f32,
                    h as f32,
                    t == edit::Target::Ticker,
                );
            }
        }
        px.data().to_vec()
    }
}

fn center_widget(cfg: &mut HudConfig, name: &str) {
    if name == "dash" {
        size_demo_dash(cfg);
    } else if let Some(id) = widget_id(name) {
        cfg.snap(id, SnapAlign::Center);
    }
}

/// Full dash needs room for ~Lapped; simple dash is gear + speed only.
fn size_demo_dash(cfg: &mut HudConfig) {
    if cfg.dash_simple {
        cfg[WidgetId::Dash].rect.w = 0.09;
        cfg[WidgetId::Dash].rect.h = 0.08;
    } else {
        cfg[WidgetId::Dash].rect.w = 0.16;
        cfg[WidgetId::Dash].rect.h = 0.115;
    }
    cfg.snap(WidgetId::Dash, SnapAlign::Center);
}

fn show_only(cfg: &mut HudConfig, name: &str) {
    cfg[WidgetId::Standings].show = name == "standings";
    cfg[WidgetId::Relative].show = name == "relative";
    cfg[WidgetId::Dash].show = name == "dash";
    cfg[WidgetId::Map].show = name == "map";
    cfg[WidgetId::Minimap].show = name == "minimap";
    cfg[WidgetId::Radar].show = name == "radar";
    cfg[WidgetId::Ticker].show = name == "ticker";
    cfg[WidgetId::Sys].show = name == "sys";
    cfg[WidgetId::Sector].show = name == "sector";
    cfg[WidgetId::Delta].show = name == "delta";
    cfg[WidgetId::Flag].show = name == "flag";
}

fn widget_id(name: &str) -> Option<WidgetId> {
    Some(match name {
        "standings" => WidgetId::Standings,
        "relative" => WidgetId::Relative,
        "map" => WidgetId::Map,
        "minimap" => WidgetId::Minimap,
        "radar" => WidgetId::Radar,
        "dash" => WidgetId::Dash,
        "ticker" => WidgetId::Ticker,
        "sys" => WidgetId::Sys,
        "sector" => WidgetId::Sector,
        "delta" => WidgetId::Delta,
        "flag" => WidgetId::Flag,
        _ => return None,
    })
}

fn sync_delta_preview(active: &str, t: f32) {
    if active != "delta" {
        mxbo_hud::delta::set_preview(None);
        return;
    }
    let wobble = (t * 1.4).sin() * 220.0;
    mxbo_hud::delta::set_preview(Some(mxbo_hud::delta::DeltaView {
        ready: true,
        recording: false,
        has_delta: true,
        delta_ms: -347 + wobble as i32,
        ref_lap_ms: 72_140,
        last_lap_ms: 72_480,
        cover: 100,
        new_best: false,
    }));
}

fn sync_flag_preview(active: &str, t: f32, yellow: bool, blue: bool) {
    if active != "flag" {
        mxbo_hud::set_flag_preview(-1);
        return;
    }
    let mut codes = [2i32, 1, 0, 0, 0];
    let mut n = 2;
    if yellow {
        codes[n] = 3;
        n += 1;
    }
    if blue {
        codes[n] = 4;
        n += 1;
    }
    codes[n] = 0;
    n += 1;
    let slot = 2.5;
    let i = ((t % (slot * n as f32)) / slot) as usize;
    mxbo_hud::set_flag_preview(codes[i.min(n - 1)]);
}

fn flag(cfg: &HudConfig, key: &str) -> Option<bool> {
    Some(match key {
        "st_pos" => cfg.st_pos,
        "st_num" => cfg.st_num,
        "st_name" => cfg.st_name,
        "st_gap" => cfg.st_gap,
        "st_interval" => cfg.st_interval,
        "st_laps" => cfg.st_laps,
        "st_current" => cfg.st_current,
        "st_best" => cfg.st_best,
        "st_last" => cfg.st_last,
        "st_status" => cfg.st_status,
        "st_bike" => cfg.st_bike,
        "st_penalty" => cfg.st_penalty,
        "st_crashed" => cfg.st_crashed,
        "rel_num" => cfg.rel_num,
        "rel_name" => cfg.rel_name,
        "rel_gap" => cfg.rel_gap,
        "rel_laps" => cfg.rel_laps,
        "rel_current" => cfg.rel_current,
        "rel_pos" => cfg.rel_pos,
        "rel_bike" => cfg.rel_bike,
        "rel_penalty" => cfg.rel_penalty,
        "rel_interval" => cfg.rel_interval,
        "rel_crashed" => cfg.rel_crashed,
        "rel_best" => cfg.rel_best,
        "rel_last" => cfg.rel_last,
        "map_others" => cfg.map_others,
        "map_sf" => cfg.map_sf,
        "map_sectors" => cfg.map_sectors,
        "map_arrows" => cfg.map_arrows,
        "map_crown" => cfg.map_crown,
        "map_place" => cfg.map_place,
        "map_numbers" => cfg.map_numbers,
        "mini_others" => cfg.mini_others,
        "mini_sf" => cfg.mini_sf,
        "mini_sectors" => cfg.mini_sectors,
        "mini_arrows" => cfg.mini_arrows,
        "mini_crown" => cfg.mini_crown,
        "mini_place" => cfg.mini_place,
        "mini_numbers" => cfg.mini_numbers,
        "radar_sides" => cfg.radar_sides,
        "radar_rear" => cfg.radar_rear,
        "radar_rings" => cfg.radar_rings,
        "st_bold" => cfg[WidgetId::Standings].bold,
        "st_stripe" => cfg.st_stripe,
        "rel_bold" => cfg[WidgetId::Relative].bold,
        "rel_stripe" => cfg.rel_stripe,
        "map_bold" => cfg[WidgetId::Map].bold,
        "mini_bold" => cfg[WidgetId::Minimap].bold,
        "radar_bold" => cfg[WidgetId::Radar].bold,
        "dash_bold" => cfg[WidgetId::Dash].bold,
        "dash_rev" => cfg.dash_rev,
        "dash_simple" => cfg.dash_simple,
        "ticker_bold" => cfg[WidgetId::Ticker].bold,
        "sys_bold" => cfg[WidgetId::Sys].bold,
        "sector_live" => cfg.sector_live,
        "sector_session" => cfg.sector_session,
        "sector_hist" => cfg.sector_hist,
        "delta_session" => cfg.delta_session,
        "sector_bold" => cfg[WidgetId::Sector].bold,
        "delta_bold" => cfg[WidgetId::Delta].bold,
        "flag_bold" => cfg[WidgetId::Flag].bold,
        "flag_yellow" => cfg.flag_yellow,
        "flag_blue" => cfg.flag_blue,
        "ticker_title" => cfg.ticker_title,
        "ticker_autoscroll" => cfg.ticker_autoscroll,
        _ => return None,
    })
}

fn set_flag(cfg: &mut HudConfig, key: &str, on: bool) {
    match key {
        "st_pos" => cfg.st_pos = on,
        "st_num" => cfg.st_num = on,
        "st_name" => cfg.st_name = on,
        "st_gap" => cfg.st_gap = on,
        "st_interval" => cfg.st_interval = on,
        "st_laps" => cfg.st_laps = on,
        "st_current" => cfg.st_current = on,
        "st_best" => cfg.st_best = on,
        "st_last" => cfg.st_last = on,
        "st_status" => cfg.st_status = on,
        "st_bike" => cfg.st_bike = on,
        "st_penalty" => cfg.st_penalty = on,
        "st_crashed" => cfg.st_crashed = on,
        "rel_num" => cfg.rel_num = on,
        "rel_name" => cfg.rel_name = on,
        "rel_gap" => cfg.rel_gap = on,
        "rel_laps" => cfg.rel_laps = on,
        "rel_current" => cfg.rel_current = on,
        "rel_pos" => cfg.rel_pos = on,
        "rel_bike" => cfg.rel_bike = on,
        "rel_penalty" => cfg.rel_penalty = on,
        "rel_interval" => cfg.rel_interval = on,
        "rel_crashed" => cfg.rel_crashed = on,
        "rel_best" => cfg.rel_best = on,
        "rel_last" => cfg.rel_last = on,
        "map_others" => cfg.map_others = on,
        "map_sf" => cfg.map_sf = on,
        "map_sectors" => cfg.map_sectors = on,
        "map_arrows" => cfg.map_arrows = on,
        "map_crown" => cfg.map_crown = on,
        "map_place" => cfg.map_place = on,
        "map_numbers" => cfg.map_numbers = on,
        "mini_others" => cfg.mini_others = on,
        "mini_sf" => cfg.mini_sf = on,
        "mini_sectors" => cfg.mini_sectors = on,
        "mini_arrows" => cfg.mini_arrows = on,
        "mini_crown" => cfg.mini_crown = on,
        "mini_place" => cfg.mini_place = on,
        "mini_numbers" => cfg.mini_numbers = on,
        "radar_sides" => cfg.radar_sides = on,
        "radar_rear" => cfg.radar_rear = on,
        "radar_rings" => cfg.radar_rings = on,
        "st_bold" => cfg[WidgetId::Standings].bold = on,
        "st_stripe" => cfg.st_stripe = on,
        "rel_bold" => cfg[WidgetId::Relative].bold = on,
        "rel_stripe" => cfg.rel_stripe = on,
        "map_bold" => cfg[WidgetId::Map].bold = on,
        "mini_bold" => cfg[WidgetId::Minimap].bold = on,
        "radar_bold" => cfg[WidgetId::Radar].bold = on,
        "dash_bold" => cfg[WidgetId::Dash].bold = on,
        "dash_rev" => cfg.dash_rev = on,
        "dash_simple" => cfg.dash_simple = on,
        "ticker_bold" => cfg[WidgetId::Ticker].bold = on,
        "sys_bold" => cfg[WidgetId::Sys].bold = on,
        "sector_live" => cfg.sector_live = on,
        "sector_session" => cfg.sector_session = on,
        "sector_hist" => cfg.sector_hist = on,
        "delta_session" => cfg.delta_session = on,
        "sector_bold" => cfg[WidgetId::Sector].bold = on,
        "delta_bold" => cfg[WidgetId::Delta].bold = on,
        "flag_bold" => cfg[WidgetId::Flag].bold = on,
        "flag_yellow" => cfg.flag_yellow = on,
        "flag_blue" => cfg.flag_blue = on,
        "ticker_title" => cfg.ticker_title = on,
        "ticker_autoscroll" => cfg.ticker_autoscroll = on,
        _ => {}
    }
}

fn int_val(cfg: &HudConfig, key: &str) -> Option<i32> {
    Some(match key {
        "standings_rows" => cfg.standings_rows,
        "relative_count" => cfg.relative_count,
        "st_bg" => cfg[WidgetId::Standings].bg,
        "st_hl" => cfg.st_hl,
        "rel_bg" => cfg[WidgetId::Relative].bg,
        "rel_hl" => cfg.rel_hl,
        "map_bg" => cfg[WidgetId::Map].bg,
        "mini_bg" => cfg[WidgetId::Minimap].bg,
        "mini_zoom" => cfg.mini_zoom,
        "radar_bg" => cfg[WidgetId::Radar].bg,
        "dash_bg" => cfg[WidgetId::Dash].bg,
        "ticker_bg" => cfg[WidgetId::Ticker].bg,
        "sys_bg" => cfg[WidgetId::Sys].bg,
        "sector_bg" => cfg[WidgetId::Sector].bg,
        "ticker_count" => cfg.ticker_count,
        "sector_hist_laps" => cfg.sector_hist_count() as i32,
        "st_font" => cfg[WidgetId::Standings].font,
        "rel_font" => cfg[WidgetId::Relative].font,
        "map_font" => cfg[WidgetId::Map].font,
        "mini_font" => cfg[WidgetId::Minimap].font,
        "radar_font" => cfg[WidgetId::Radar].font,
        "dash_font" => cfg[WidgetId::Dash].font,
        "ticker_font" => cfg[WidgetId::Ticker].font,
        "sys_font" => cfg[WidgetId::Sys].font,
        "sector_font" => cfg[WidgetId::Sector].font,
        "delta_font" => cfg[WidgetId::Delta].font,
        "delta_bg" => cfg[WidgetId::Delta].bg,
        "flag_font" => cfg[WidgetId::Flag].font,
        "flag_bg" => cfg[WidgetId::Flag].bg,
        _ => return None,
    })
}

fn set_int(cfg: &mut HudConfig, key: &str, value: i32) {
    match key {
        "standings_rows" => cfg.standings_rows = value.clamp(3, 40),
        "relative_count" => cfg.relative_count = value.clamp(1, 8),
        "st_bg" => cfg[WidgetId::Standings].bg = value.clamp(0, 100),
        "st_hl" => cfg.st_hl = value.clamp(0, 100),
        "rel_bg" => cfg[WidgetId::Relative].bg = value.clamp(0, 100),
        "rel_hl" => cfg.rel_hl = value.clamp(0, 100),
        "map_bg" => cfg[WidgetId::Map].bg = value.clamp(0, 100),
        "mini_bg" => cfg[WidgetId::Minimap].bg = value.clamp(0, 100),
        "mini_zoom" => cfg.mini_zoom = value.clamp(0, 100),
        "radar_bg" => cfg[WidgetId::Radar].bg = value.clamp(0, 100),
        "dash_bg" => cfg[WidgetId::Dash].bg = value.clamp(0, 100),
        "ticker_bg" => cfg[WidgetId::Ticker].bg = value.clamp(0, 100),
        "sys_bg" => cfg[WidgetId::Sys].bg = value.clamp(0, 100),
        "sector_bg" => cfg[WidgetId::Sector].bg = value.clamp(0, 100),
        "delta_bg" => cfg[WidgetId::Delta].bg = value.clamp(0, 100),
        "flag_bg" => cfg[WidgetId::Flag].bg = value.clamp(0, 100),
        "ticker_count" => cfg.ticker_count = value.clamp(3, 15),
        "sector_hist_laps" => cfg.sector_hist_laps = value.clamp(1, 5),
        "st_font" => cfg.set_font_pct(WidgetId::Standings, value),
        "rel_font" => cfg.set_font_pct(WidgetId::Relative, value),
        "map_font" => cfg.set_font_pct(WidgetId::Map, value),
        "mini_font" => cfg.set_font_pct(WidgetId::Minimap, value),
        "radar_font" => cfg.set_font_pct(WidgetId::Radar, value),
        "dash_font" => cfg.set_font_pct(WidgetId::Dash, value),
        "ticker_font" => cfg.set_font_pct(WidgetId::Ticker, value),
        "sys_font" => cfg.set_font_pct(WidgetId::Sys, value),
        "sector_font" => cfg.set_font_pct(WidgetId::Sector, value),
        "delta_font" => cfg.set_font_pct(WidgetId::Delta, value),
        "flag_font" => cfg.set_font_pct(WidgetId::Flag, value),
        _ => {}
    }
}

fn field_val(cfg: &HudConfig, key: &str) -> Option<String> {
    Some(match key {
        "dash_left" => cfg.dash_left.key().into(),
        "dash_mid" => cfg.dash_mid.key().into(),
        "dash_right" => cfg.dash_right.key().into(),
        "ticker_left" => cfg.ticker_left.key().into(),
        "ticker_right" => cfg.ticker_right.key().into(),
        "st_head0" => cfg.st_head[0].key().into(),
        "st_head1" => cfg.st_head[1].key().into(),
        "st_head2" => cfg.st_head[2].key().into(),
        "st_foot0" => cfg.st_foot[0].key().into(),
        "st_foot1" => cfg.st_foot[1].key().into(),
        "st_foot2" => cfg.st_foot[2].key().into(),
        "rel_head0" => cfg.rel_head[0].key().into(),
        "rel_head1" => cfg.rel_head[1].key().into(),
        "rel_head2" => cfg.rel_head[2].key().into(),
        "rel_foot0" => cfg.rel_foot[0].key().into(),
        "rel_foot1" => cfg.rel_foot[1].key().into(),
        "rel_foot2" => cfg.rel_foot[2].key().into(),
        "map_dot" => cfg.map_dot.key().into(),
        "mini_dot" => cfg.mini_dot.key().into(),
        "st_text" => cfg.st_text.key().into(),
        "rel_text" => cfg.rel_text.key().into(),
        _ => return None,
    })
}

fn set_field(cfg: &mut HudConfig, key: &str, value: &str) {
    match key {
        "dash_left" => cfg.dash_left = DashField::parse(value),
        "dash_mid" => cfg.dash_mid = DashField::parse(value),
        "dash_right" => cfg.dash_right = DashField::parse(value),
        "ticker_left" => cfg.ticker_left = BoardField::parse(value),
        "ticker_right" => cfg.ticker_right = BoardField::parse(value),
        "st_head0" => cfg.st_head[0] = BoardField::parse(value),
        "st_head1" => cfg.st_head[1] = BoardField::parse(value),
        "st_head2" => cfg.st_head[2] = BoardField::parse(value),
        "st_foot0" => cfg.st_foot[0] = BoardField::parse(value),
        "st_foot1" => cfg.st_foot[1] = BoardField::parse(value),
        "st_foot2" => cfg.st_foot[2] = BoardField::parse(value),
        "rel_head0" => cfg.rel_head[0] = BoardField::parse(value),
        "rel_head1" => cfg.rel_head[1] = BoardField::parse(value),
        "rel_head2" => cfg.rel_head[2] = BoardField::parse(value),
        "rel_foot0" => cfg.rel_foot[0] = BoardField::parse(value),
        "rel_foot1" => cfg.rel_foot[1] = BoardField::parse(value),
        "rel_foot2" => cfg.rel_foot[2] = BoardField::parse(value),
        "map_dot" => cfg.map_dot = DotLabel::parse(value),
        "mini_dot" => cfg.mini_dot = DotLabel::parse(value),
        "st_text" => cfg.st_text = TableText::parse(value),
        "rel_text" => cfg.rel_text = TableText::parse(value),
        _ => {}
    }
}

fn draw_edit_frame(px: &mut Pixmap, r: mxbo_hud::snapshot::Rect, sw: f32, sh: f32, ew_only: bool) {
    let x = r.x * sw;
    let y = r.y * sh;
    let w = r.w * sw;
    let h = r.h * sh;
    let mut pb = PathBuilder::new();
    pb.move_to(x, y);
    pb.line_to(x + w, y);
    pb.line_to(x + w, y + h);
    pb.line_to(x, y + h);
    pb.close();
    if let Some(path) = pb.finish() {
        let mut paint = Paint::default();
        paint.set_color(Color::from_rgba8(255, 148, 48, 220));
        paint.anti_alias = true;
        px.stroke_path(&path, &paint, &Stroke { width: 2.0, ..Stroke::default() }, Transform::identity(), None);
    }
    let mut paint = Paint::default();
    paint.set_color(Color::from_rgba8(255, 148, 48, 255));
    let handles: &[(f32, f32)] = if ew_only {
        &[(x, y + h * 0.5), (x + w, y + h * 0.5)]
    } else {
        &[
            (x, y),
            (x + w, y),
            (x, y + h),
            (x + w, y + h),
            (x + w * 0.5, y),
            (x + w * 0.5, y + h),
            (x, y + h * 0.5),
            (x + w, y + h * 0.5),
        ]
    };
    for &(hx, hy) in handles {
        if let Some(rect) = tiny_skia::Rect::from_xywh(hx - 4.0, hy - 4.0, 8.0, 8.0) {
            px.fill_rect(rect, &paint, Transform::identity(), None);
        }
    }
}

fn demo_snapshot() -> Snapshot {
    let mut s = Snapshot::default();
    s.magic = MAGIC;
    s.version = VERSION;
    s.on_track = 1;
    s.has_telemetry = 1;
    s.local_race_num = FOCUS;
    s.focus_race_num = FOCUS;
    s.max_rpm = 13500;
    s.shift_rpm = 11800;
    s.engine_temp = 89.0;
    s.air_temp = 24.0;
    s.fuel = 5.6;
    s.max_fuel = 7.0;
    s.session_laps = 12;
    s.session_length = 45 * 60;
    s.session_time_ms = 20 * 60 * 1000;
    s.best_lap_ms = 72_140;
    s.last_lap_ms = 73_220;
    s.current_lap = 8;
    s.local_gear = 3;
    s.sector_count = 3;
    s.sector_last = 0;
    s.sector_cur = [24_093, 0, 0];
    s.sector_last_lap = [24_310, 25_820, 23_090];
    s.sector_best = [24_180, 25_640, 22_910];
    s.sector_delta = [-87, 0, 0];
    s.sector_delta_valid = 0b001;
    s.local_speed = 18.0;
    let (poly, length, sf, name) = captured_track();
    write_name(&mut s.track_name, &name);
    s.poly_count = poly.len() as i32;
    for (i, p) in poly.iter().enumerate() {
        s.poly[i] = *p;
    }
    s.track_length = length;
    s.sf_meters = sf;
    mxbo_hud::sector::set_split_fracs([DEMO_S1_END, DEMO_S2_END]);
    mxbo_hud::sector::set_history([
        [24_180, 25_640, 20_147],
        [24_410, 25_890, 20_400],
        [24_250, 25_710, 20_220],
        [24_500, 26_010, 20_550],
        [24_330, 25_800, 20_310],
    ]);

    s.rider_count = RIDERS.len() as i32;
    for (i, (name, _, _)) in RIDERS.iter().enumerate() {
        let pos = (i as f32 / RIDERS.len() as f32 + 0.08).fract();
        let (x, z, yaw) = sample_track(&s, pos);
        s.riders[i] = Rider {
            race_num: i as i32 + 1,
            x,
            z,
            yaw,
            track_pos: pos,
            crashed: 0,
            name: [0; 32],
        };
        write_name(&mut s.riders[i].name, name);
    }
    apply_radar_pack(&mut s, 0.0);
    refresh_standings(&mut s);
    s
}

/// Keep a few riders in the focus rider's sides and rear so the radar has blips.
const RADAR_PACK: &[(usize, f32, f32, f32, f32, f32)] = &[
    (6, -1.4, -2.3, 0.5, 0.35, 0.4),
    (7, -2.2, 2.0, 0.7, 0.40, 1.1),
    (8, 2.4, -1.6, 1.1, 0.55, 2.0),
    (9, 5.2, 0.8, 1.4, 0.70, 2.8),
];

fn offset_from_track(s: &Snapshot, pos: f32, along_m: f32, lat_m: f32) -> (f32, f32, f32, f32) {
    let dt = along_m / s.track_length.max(1.0);
    let t = (pos + dt).rem_euclid(1.0);
    let (x, z, yaw) = sample_track(s, t);
    let rx = yaw.cos();
    let rz = -yaw.sin();
    (x + rx * lat_m, z + rz * lat_m, yaw, t)
}

fn apply_radar_pack(s: &mut Snapshot, t: f32) {
    let n = s.rider_count.max(0) as usize;
    let Some(fi) = (0..n).find(|&i| s.riders[i].race_num == FOCUS) else {
        return;
    };
    let focus_pos = s.riders[fi].track_pos;
    for &(i, along0, lat0, aa, la, phase) in RADAR_PACK {
        if i >= n || i == fi {
            continue;
        }
        let along = along0 + aa * (t * 1.15 + phase).sin();
        let lat = lat0 + la * (t * 0.85 + phase * 1.4).sin();
        let (x, z, yaw, pos) = offset_from_track(s, focus_pos, along, lat);
        s.riders[i].x = x;
        s.riders[i].z = z;
        s.riders[i].yaw = yaw;
        s.riders[i].track_pos = pos;
    }
}

fn captured_track() -> (Vec<Point>, f32, f32, String) {
    if demo_track::POLY.len() >= 8 {
        let poly = demo_track::POLY
            .iter()
            .map(|(x, z)| Point { x: *x, z: *z })
            .collect();
        let name = if demo_track::TRACK_NAME.is_empty() {
            "RedBud".into()
        } else {
            demo_track::TRACK_NAME.into()
        };
        return (poly, demo_track::TRACK_LENGTH, demo_track::SF_METERS, name);
    }
    let (poly, length) = redbud_poly();
    (poly, length, 0.0, "RedBud".into())
}

fn redbud_poly() -> (Vec<Point>, f32) {
    let keys = [
        (2.0, -78.0),
        (10.0, -42.0),
        (16.0, 2.0),
        (18.0, 48.0),
        (8.0, 82.0),
        (-22.0, 92.0),
        (-54.0, 78.0),
        (-78.0, 46.0),
        (-86.0, 8.0),
        (-80.0, -28.0),
        (-58.0, -56.0),
        (-24.0, -70.0),
        (12.0, -52.0),
        (36.0, -22.0),
        (48.0, 16.0),
        (40.0, 52.0),
        (14.0, 64.0),
        (-12.0, 44.0),
        (-28.0, 14.0),
        (-18.0, -16.0),
        (4.0, -38.0),
        (2.0, -78.0),
    ];
    let n = keys.len();
    let samples = 220.min(MAX_POLY);
    let mut poly = Vec::with_capacity(samples);
    for i in 0..samples {
        let t = i as f32 / samples as f32 * n as f32;
        let i1 = t.floor() as usize % n;
        let i0 = (i1 + n - 1) % n;
        let i2 = (i1 + 1) % n;
        let i3 = (i1 + 2) % n;
        let f = t.fract();
        poly.push(Point {
            x: catmull(keys[i0].0, keys[i1].0, keys[i2].0, keys[i3].0, f),
            z: catmull(keys[i0].1, keys[i1].1, keys[i2].1, keys[i3].1, f),
        });
    }
    let mut length = 0.0f32;
    for i in 0..poly.len() {
        let a = &poly[i];
        let b = &poly[(i + 1) % poly.len()];
        let dx = b.x - a.x;
        let dz = b.z - a.z;
        length += (dx * dx + dz * dz).sqrt();
    }
    (poly, length)
}

fn catmull(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * (2.0 * p1
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

fn sample_track(s: &Snapshot, t: f32) -> (f32, f32, f32) {
    let n = s.poly_count.max(2) as usize;
    let u = t.fract().rem_euclid(1.0) * (n as f32);
    let i = u.floor() as usize % n;
    let j = (i + 1) % n;
    let f = u.fract();
    let x = s.poly[i].x + (s.poly[j].x - s.poly[i].x) * f;
    let z = s.poly[i].z + (s.poly[j].z - s.poly[i].z) * f;
    let yaw = (s.poly[j].x - s.poly[i].x).atan2(s.poly[j].z - s.poly[i].z);
    (x, z, yaw)
}

fn animate(s: &mut Snapshot, t: f32, dt: f32) {
    s.session_time_ms += (dt * 1000.0) as i32;
    s.local_rpm = (8200.0 + 3800.0 * (t * 2.4).sin()) as i32;
    s.local_gear = 2 + ((t * 0.35).sin() * 1.6 + 1.6) as i32;
    s.local_speed = 14.0 + 7.0 * (0.5 + 0.5 * (t * 1.1).sin());

    for i in 0..s.rider_count.max(0) as usize {
        let speed = 0.006 + (i as f32) * 0.00022 + 0.0012 * ((t * 0.35) + i as f32).sin();
        let next = (s.riders[i].track_pos + speed * dt).fract();
        s.riders[i].track_pos = if next < 0.0 { next + 1.0 } else { next };
        let (x, z, yaw) = sample_track(s, s.riders[i].track_pos);
        s.riders[i].x = x;
        s.riders[i].z = z;
        s.riders[i].yaw = yaw;
        if s.riders[i].race_num == FOCUS {
            s.local_x = x;
            s.local_z = z;
            s.local_yaw = yaw;
            s.local_track_pos = s.riders[i].track_pos;
            s.local_vel_x = yaw.sin() * s.local_speed;
            s.local_vel_z = yaw.cos() * s.local_speed;
        }
    }
    apply_radar_pack(s, t);
    animate_sectors(s);
    refresh_standings(s);
}

/// Lap clock follows track pos so S2/S3 live time is clock minus completed splits.
fn animate_sectors(s: &mut Snapshot) {
    mxbo_hud::sector::set_split_fracs([DEMO_S1_END, DEMO_S2_END]);
    mxbo_hud::sector::set_history([
        [24_180, 25_640, 20_147],
        [24_410, 25_890, 20_400],
        [24_250, 25_710, 20_220],
        [24_500, 26_010, 20_550],
        [24_330, 25_800, 20_310],
    ]);
    s.sector_count = 3;
    s.sector_best = [24_180, 25_640, 22_910];
    s.sector_last_lap = [DEMO_S1_MS, DEMO_S2_MS, DEMO_S3_MS];
    s.sector_delta = [-87, 120, -40];
    s.last_lap_ms = DEMO_S1_MS + DEMO_S2_MS + DEMO_S3_MS;
    let frac = s.local_track_pos.rem_euclid(1.0);
    if frac < DEMO_S1_END {
        let p = (frac / DEMO_S1_END).clamp(0.0, 1.0);
        s.current_lap_ms = (DEMO_S1_MS as f32 * p) as i32;
        s.sector_cur = [0, 0, 0];
        s.sector_delta_valid = 0;
        s.sector_last = 2;
    } else if frac < DEMO_S2_END {
        let p = ((frac - DEMO_S1_END) / (DEMO_S2_END - DEMO_S1_END)).clamp(0.0, 1.0);
        s.current_lap_ms = DEMO_S1_MS + (DEMO_S2_MS as f32 * p).max(1.0) as i32;
        s.sector_cur = [DEMO_S1_MS, 0, 0];
        s.sector_delta_valid = 0b001;
        s.sector_last = 0;
    } else {
        let p = ((frac - DEMO_S2_END) / (1.0 - DEMO_S2_END)).clamp(0.0, 1.0);
        s.current_lap_ms = DEMO_S1_MS + DEMO_S2_MS + (DEMO_S3_MS as f32 * p).max(1.0) as i32;
        s.sector_cur = [DEMO_S1_MS, DEMO_S2_MS, 0];
        s.sector_delta_valid = 0b011;
        s.sector_last = 1;
    }
}

fn refresh_standings(s: &mut Snapshot) {
    let n = s.rider_count.max(0) as usize;
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|a, b| {
        s.riders[*b]
            .track_pos
            .partial_cmp(&s.riders[*a].track_pos)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    s.standing_count = n as i32;
    let leader_pos = s.riders[order[0]].track_pos;
    for (i, &ri) in order.iter().enumerate() {
        let r = &s.riders[ri];
        let (name, bike, cat) = RIDERS[ri];
        let gap = if i == 0 {
            0
        } else {
            let d = (leader_pos - r.track_pos).rem_euclid(1.0);
            (d * s.track_length / s.local_speed.max(8.0) * 1000.0) as i32
        };
        let num_laps = match ri {
            6 | 7 => 8,
            8 | 9 => 6,
            _ => 7,
        };
        s.standings[i] = Standing {
            race_num: r.race_num,
            position: i as i32 + 1,
            state: 0,
            best_lap_ms: 71_800 + ri as i32 * 210,
            num_laps,
            gap_ms: gap,
            gap_laps: (8 - num_laps).max(0),
            pit: 0,
            penalty_ms: 0,
            crashed: 0,
            name: [0; 32],
            bike: [0; 32],
            last_lap_ms: 72_400 + ri as i32 * 180,
            category: [0; 32],
        };
        write_name(&mut s.standings[i].name, name);
        write_name(&mut s.standings[i].bike, bike);
        write_name(&mut s.standings[i].category, cat);
    }
}
