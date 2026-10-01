use super::*;
use crate::config::{
    BoardField, DashField, FontFamily, HudConfig, LeanStyle, RelField, SessionPreset,
    StField, StanceStyle, WidgetId,
};
use crate::race_store::{
    effective_extra_laps, effective_race_laps, is_practice_session, live_session, session_preset,
    ClockMode,
};
use crate::shm::{write_name, Point, Rider, Snapshot, Standing, MAGIC, TRACK_NAME, VERSION};
use tiny_skia::{Color, Pixmap, Rect};

use super::render_support::*;
use super::render_support::draw;

#[test]
fn lean_min_golden() {
    let _g = session_lock();
    reset_session();
    let base = live_snap();
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Lean].show = true;
    cfg[WidgetId::Lean].bg = 0;
    cfg.lean.lean_style = LeanStyle::Minimal;
    let mut lean = base;
    lean.local_roll = -32.0;
    lean.local_pitch = -18.0;
    lean.local_steer = -0.12;
    lean.steer_lock = 0.40;
    let s = golden_snap(&lean, &cfg);
    draw_widget_golden("lean-min", &s, &cfg, cfg[WidgetId::Lean].rect);
}

