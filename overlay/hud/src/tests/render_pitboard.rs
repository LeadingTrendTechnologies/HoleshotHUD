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
fn pitboard_golden() {
    let _g = session_lock();
    reset_session();
    crate::delta::set_preview(None);
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Pitboard].show = true;
    cfg.pit.pit_sponsor = "HOLESHOT".into();
    let mut s = live_snap();
    write_name(&mut s.standings[1].name, "You");
    s.standings[1].num_laps = 4;
    s.standings[1].last_lap_ms = 0;
    s.last_lap_ms = 0;
    s.session_time_ms = 8 * 60 * 1000;
    let s = golden_snap(&s, &cfg);
    draw_widget_golden("pitboard", &s, &cfg, cfg[WidgetId::Pitboard].rect);
    crate::delta::set_preview(None);
}

