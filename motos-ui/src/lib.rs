pub mod chrome;
mod paint;

pub use chrome::{paint_mode_bar, take_pending_drops, Drop, Hit, HitBox, PendingDrop};
pub use mxbo_hud::render::Fonts;
pub use mxbo_review::SessionDetail;
pub use paint::{
    default_analyze_scrub, draw_pending_drop_menus, follow_pan_for, paint_analyze_open,
    paint_track_bank_map, pane_review,
};

#[derive(Clone, Copy, Debug)]
pub struct AnalyzePaintState {
    pub compare: i32,
    pub you_lap: i32,
    pub warmup: bool,
    pub scrub: f32,
    pub zoom: f32,
    pub pan_x: f32,
    pub pan_z: f32,
    pub follow: bool,
    pub open_drop: Option<Drop>,
    pub live: bool,
}
