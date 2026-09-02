use mxbo_hud::config::{HudConfig, WidgetId};
use mxbo_hud::snapshot::Rect;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Map,
    Standings,
    Relative,
    Minimap,
    Radar,
    Dash,
    Ticker,
    Sys,
    Sector,
    Delta,
    Flag,
    Lean,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Handle {
    Move,
    N,
    S,
    E,
    W,
    NE,
    NW,
    SE,
    SW,
}

impl Handle {
    pub fn cursor(self) -> &'static str {
        match self {
            Self::Move => "move",
            Self::N | Self::S => "ns-resize",
            Self::E | Self::W => "ew-resize",
            Self::NE | Self::SW => "nesw-resize",
            Self::NW | Self::SE => "nwse-resize",
        }
    }
}

#[derive(Clone, Copy)]
pub struct Drag {
    pub target: Target,
    pub handle: Handle,
    pub grab_x: f32,
    pub grab_y: f32,
    pub orig: Rect,
}

pub fn parse_target(name: &str) -> Option<Target> {
    Some(match name {
        "standings" => Target::Standings,
        "relative" => Target::Relative,
        "dash" => Target::Dash,
        "map" => Target::Map,
        "minimap" => Target::Minimap,
        "radar" => Target::Radar,
        "ticker" => Target::Ticker,
        "sys" => Target::Sys,
        "sector" => Target::Sector,
        "delta" => Target::Delta,
        "flag" => Target::Flag,
        "lean" => Target::Lean,
        _ => return None,
    })
}

pub fn rect_of(cfg: &HudConfig, t: Target) -> Rect {
    match t {
        Target::Map => cfg[WidgetId::Map].rect,
        Target::Standings => cfg[WidgetId::Standings].rect,
        Target::Relative => cfg[WidgetId::Relative].rect,
        Target::Minimap => cfg[WidgetId::Minimap].rect,
        Target::Radar => cfg[WidgetId::Radar].rect,
        Target::Dash => cfg[WidgetId::Dash].rect,
        Target::Ticker => cfg[WidgetId::Ticker].rect,
        Target::Sys => cfg[WidgetId::Sys].rect,
        Target::Sector => cfg[WidgetId::Sector].rect,
        Target::Delta => cfg[WidgetId::Delta].rect,
        Target::Flag => cfg[WidgetId::Flag].rect,
        Target::Lean => cfg[WidgetId::Lean].rect,
    }
}

pub fn edit_rect(cfg: &HudConfig, t: Target, ow: f32, oh: f32) -> Rect {
    visual_rect(rect_of(cfg, t), t, ow, oh)
}

pub fn set_rect(cfg: &mut HudConfig, t: Target, r: Rect) {
    match t {
        Target::Map => cfg[WidgetId::Map].rect = r,
        Target::Standings => cfg[WidgetId::Standings].rect = r,
        Target::Relative => cfg[WidgetId::Relative].rect = r,
        Target::Minimap => cfg[WidgetId::Minimap].rect = r,
        Target::Radar => cfg[WidgetId::Radar].rect = r,
        Target::Dash => cfg[WidgetId::Dash].rect = r,
        Target::Ticker => cfg[WidgetId::Ticker].rect = r,
        Target::Sys => cfg[WidgetId::Sys].rect = r,
        Target::Sector => cfg[WidgetId::Sector].rect = r,
        Target::Delta => cfg[WidgetId::Delta].rect = r,
        Target::Flag => cfg[WidgetId::Flag].rect = r,
        Target::Lean => cfg[WidgetId::Lean].rect = r,
    }
}

pub fn hit(cfg: &HudConfig, t: Target, x: f32, y: f32, ow: f32, oh: f32) -> Option<Handle> {
    let r = visual_rect(rect_of(cfg, t), t, ow, oh);
    if let Some(h) = handle_at(r, x, y, ow, oh, t) {
        return Some(h);
    }
    let inside = if t == Target::Minimap {
        contains_circle(rect_of(cfg, t), x, y, ow, oh)
    } else {
        contains(rect_of(cfg, t), x, y)
    };
    inside.then_some(Handle::Move)
}

fn contains(r: Rect, x: f32, y: f32) -> bool {
    x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h
}

fn contains_circle(r: Rect, x: f32, y: f32, ow: f32, oh: f32) -> bool {
    let vis = visual_rect(r, Target::Minimap, ow, oh);
    let d = vis.w.min(vis.h) * 0.5;
    let cx = vis.x + vis.w * 0.5;
    let cy = vis.y + vis.h * 0.5;
    let dx = x - cx;
    let dy = y - cy;
    dx * dx + dy * dy <= d * d
}

fn visual_rect(r: Rect, t: Target, ow: f32, oh: f32) -> Rect {
    if t != Target::Minimap {
        return r;
    }
    let rw = r.w * ow;
    let rh = r.h * oh;
    let d = rw.min(rh);
    let cx = r.x * ow + rw * 0.5;
    let cy = r.y * oh + rh * 0.5;
    Rect {
        x: (cx - d * 0.5) / ow,
        y: (cy - d * 0.5) / oh,
        w: d / ow,
        h: d / oh,
    }
}

fn handle_at(r: Rect, nx: f32, ny: f32, ow: f32, oh: f32, t: Target) -> Option<Handle> {
    let px = nx * ow;
    let py = ny * oh;
    let x0 = r.x * ow;
    let y0 = r.y * oh;
    let x1 = (r.x + r.w) * ow;
    let y1 = (r.y + r.h) * oh;
    let s = 12.0;
    if t == Target::Ticker {
        if px >= x0 - s && px <= x1 + s && py >= y0 - s && py <= y1 + s {
            if (px - x0).abs() <= s {
                return Some(Handle::W);
            }
            if (px - x1).abs() <= s {
                return Some(Handle::E);
            }
        }
        return None;
    }
    let near = |hx: f32, hy: f32| (px - hx).abs() <= s && (py - hy).abs() <= s;
    if near(x0, y0) {
        return Some(Handle::NW);
    }
    if near(x1, y0) {
        return Some(Handle::NE);
    }
    if near(x0, y1) {
        return Some(Handle::SW);
    }
    if near(x1, y1) {
        return Some(Handle::SE);
    }
    if (py - y0).abs() <= s && px > x0 + s && px < x1 - s {
        return Some(Handle::N);
    }
    if (py - y1).abs() <= s && px > x0 + s && px < x1 - s {
        return Some(Handle::S);
    }
    if (px - x0).abs() <= s && py > y0 + s && py < y1 - s {
        return Some(Handle::W);
    }
    if (px - x1).abs() <= s && py > y0 + s && py < y1 - s {
        return Some(Handle::E);
    }
    None
}

pub fn resize(orig: Rect, handle: Handle, nx: f32, ny: f32, grab_x: f32, grab_y: f32, ow: f32, oh: f32, target: Target) -> Rect {
    let (min_w_px, min_h_px) = match target {
        Target::Standings => (140.0, 90.0),
        Target::Relative => (140.0, 70.0),
        Target::Map => (90.0, 90.0),
        Target::Minimap => (72.0, 72.0),
        Target::Radar => (72.0, 72.0),
        Target::Dash => (96.0, 48.0),
        Target::Ticker => (360.0, 44.0),
        Target::Sys => (160.0, 90.0),
        Target::Sector => (88.0, 44.0),
        Target::Delta => (96.0, 48.0),
        Target::Flag => (72.0, 12.0),
        Target::Lean => (88.0, 72.0),
    };
    let min_w = min_w_px / ow;
    let min_h = min_h_px / oh;
    if handle == Handle::Move {
        let mut r = orig;
        r.x = (orig.x + nx - grab_x).clamp(0.01 - r.w, 0.99);
        r.y = (orig.y + ny - grab_y).clamp(0.01 - r.h, 0.99);
        return r;
    }
    if target == Target::Minimap {
        return resize_square(orig, handle, nx, ny, ow, oh, min_w_px);
    }
    let right = orig.x + orig.w;
    let bottom = orig.y + orig.h;
    let mut x = orig.x;
    let mut y = orig.y;
    let mut w = orig.w;
    let mut h = orig.h;
    match handle {
        Handle::E | Handle::NE | Handle::SE => w = (nx - orig.x).max(min_w),
        Handle::W | Handle::NW | Handle::SW => {
            x = nx.min(right - min_w);
            w = right - x;
        }
        _ => {}
    }
    match handle {
        Handle::S | Handle::SE | Handle::SW => h = (ny - orig.y).max(min_h),
        Handle::N | Handle::NE | Handle::NW => {
            y = ny.min(bottom - min_h);
            h = bottom - y;
        }
        _ => {}
    }
    if target == Target::Ticker {
        y = orig.y;
        h = orig.h;
    }
    Rect { x, y, w, h }
}

fn resize_square(orig: Rect, handle: Handle, nx: f32, ny: f32, ow: f32, oh: f32, min_px: f32) -> Rect {
    let x0 = orig.x * ow;
    let y0 = orig.y * oh;
    let x1 = (orig.x + orig.w) * ow;
    let y1 = (orig.y + orig.h) * oh;
    let px = nx * ow;
    let py = ny * oh;
    let size_from = |dx: f32, dy: f32| dx.abs().max(dy.abs()).max(min_px);
    let (x, y, size) = match handle {
        Handle::SE | Handle::S | Handle::E => (x0, y0, size_from(px - x0, py - y0)),
        Handle::SW | Handle::W => {
            let size = size_from(x1 - px, py - y0);
            (x1 - size, y0, size)
        }
        Handle::NE | Handle::N => {
            let size = size_from(px - x0, y1 - py);
            (x0, y1 - size, size)
        }
        Handle::NW => {
            let size = size_from(x1 - px, y1 - py);
            (x1 - size, y1 - size, size)
        }
        Handle::Move => (x0, y0, (x1 - x0).max(min_px)),
    };
    Rect {
        x: x / ow,
        y: y / oh,
        w: size / ow,
        h: size / oh,
    }
}
