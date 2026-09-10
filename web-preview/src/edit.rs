use mxbo_hud::config::WidgetId;
use mxbo_hud::snapshot::Rect;

pub use mxbo_hud::layout::{
    edit_rect, hit, parse_target, rect_of, resize, set_rect, Handle,
};

#[derive(Clone, Copy)]
pub struct Drag {
    pub target: WidgetId,
    pub handle: Handle,
    pub grab_x: f32,
    pub grab_y: f32,
    pub orig: Rect,
}
