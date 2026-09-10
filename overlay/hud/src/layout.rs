use crate::config::{HudConfig, WidgetId};
use crate::shm::Rect;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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
    pub fn changes_w(self) -> bool {
        matches!(self, Self::E | Self::W | Self::NE | Self::NW | Self::SE | Self::SW)
    }

    pub fn changes_h(self) -> bool {
        matches!(self, Self::N | Self::S | Self::NE | Self::NW | Self::SE | Self::SW)
    }

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

pub fn parse_target(name: &str) -> Option<WidgetId> {
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
        "stance" => WidgetId::Stance,
        "flag" => WidgetId::Flag,
        "lean" => WidgetId::Lean,
        "gamepad" => WidgetId::Gamepad,
        "telemetry" => WidgetId::Telemetry,
        _ => return None,
    })
}

pub fn rect_of(cfg: &HudConfig, id: WidgetId) -> Rect {
    cfg[id].rect
}

pub fn set_rect(cfg: &mut HudConfig, id: WidgetId, r: Rect) {
    cfg[id].rect = r;
}

pub fn min_px(id: WidgetId) -> (f32, f32) {
    match id {
        WidgetId::Standings => (140.0, 90.0),
        WidgetId::Relative => (140.0, 70.0),
        WidgetId::Map => (90.0, 90.0),
        WidgetId::Minimap => (72.0, 72.0),
        WidgetId::Radar => (72.0, 72.0),
        WidgetId::Dash => (96.0, 48.0),
        WidgetId::Ticker => (360.0, 44.0),
        WidgetId::Sys => (160.0, 90.0),
        WidgetId::Sector => (260.0, 72.0),
        WidgetId::Delta => (220.0, 48.0),
        WidgetId::Stance => (72.0, 36.0),
        WidgetId::Flag => (72.0, 12.0),
        WidgetId::Lean => (100.0, 72.0),
        WidgetId::Gamepad => (160.0, 96.0),
        WidgetId::Telemetry => (220.0, 64.0),
    }
}

pub fn contains(r: Rect, x: f32, y: f32) -> bool {
    x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h
}

pub fn contains_circle(r: Rect, x: f32, y: f32, ow: f32, oh: f32) -> bool {
    let px = x * ow;
    let py = y * oh;
    let rw = r.w * ow;
    let rh = r.h * oh;
    let d = rw.min(rh) * 0.5;
    let cx = r.x * ow + rw * 0.5;
    let cy = r.y * oh + rh * 0.5;
    let dx = px - cx;
    let dy = py - cy;
    dx * dx + dy * dy <= d * d
}

/// Inscribed square for the minimap; other widgets keep the stored rect.
pub fn visual_rect(r: Rect, id: WidgetId, ow: f32, oh: f32) -> Rect {
    if id != WidgetId::Minimap {
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

pub fn edit_rect(cfg: &HudConfig, id: WidgetId, ow: f32, oh: f32) -> Rect {
    visual_rect(rect_of(cfg, id), id, ow, oh)
}

pub fn handle_at(r: Rect, nx: f32, ny: f32, ow: f32, oh: f32, id: WidgetId) -> Option<Handle> {
    let px = nx * ow;
    let py = ny * oh;
    let x0 = r.x * ow;
    let y0 = r.y * oh;
    let x1 = (r.x + r.w) * ow;
    let y1 = (r.y + r.h) * oh;
    let s = 12.0;
    if id == WidgetId::Ticker {
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

pub fn hit(cfg: &HudConfig, id: WidgetId, x: f32, y: f32, ow: f32, oh: f32) -> Option<Handle> {
    let stored = rect_of(cfg, id);
    let r = visual_rect(stored, id, ow, oh);
    if let Some(h) = handle_at(r, x, y, ow, oh, id) {
        return Some(h);
    }
    let inside = if id == WidgetId::Minimap {
        contains_circle(stored, x, y, ow, oh)
    } else {
        contains(r, x, y)
    };
    inside.then_some(Handle::Move)
}

pub fn resize(
    orig: Rect,
    handle: Handle,
    nx: f32,
    ny: f32,
    grab_x: f32,
    grab_y: f32,
    ow: f32,
    oh: f32,
    id: WidgetId,
) -> Rect {
    let (min_w_px, min_h_px) = min_px(id);
    let min_w = min_w_px / ow;
    let min_h = min_h_px / oh;
    if handle == Handle::Move {
        let mut r = orig;
        r.x = (orig.x + nx - grab_x).clamp(0.01 - r.w, 0.99);
        r.y = (orig.y + ny - grab_y).clamp(0.01 - r.h, 0.99);
        return r;
    }
    if id == WidgetId::Minimap {
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
    if id == WidgetId::Ticker {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_includes_stance() {
        assert_eq!(parse_target("stance"), Some(WidgetId::Stance));
        assert_eq!(parse_target("standings"), Some(WidgetId::Standings));
        assert!(parse_target("nope").is_none());
    }

    #[test]
    fn ticker_only_resizes_east_west() {
        let r = Rect {
            x: 0.1,
            y: 0.1,
            w: 0.4,
            h: 0.05,
        };
        assert_eq!(handle_at(r, 0.1, 0.125, 1000.0, 720.0, WidgetId::Ticker), Some(Handle::W));
        assert_eq!(handle_at(r, 0.5, 0.125, 1000.0, 720.0, WidgetId::Ticker), Some(Handle::E));
        assert_eq!(handle_at(r, 0.3, 0.1, 1000.0, 720.0, WidgetId::Ticker), None);
        let grown = resize(r, Handle::S, 0.3, 0.4, 0.3, 0.125, 1000.0, 720.0, WidgetId::Ticker);
        assert_eq!(grown.y, r.y);
        assert_eq!(grown.h, r.h);
    }

    #[test]
    fn stance_has_a_min_box() {
        let (w, h) = min_px(WidgetId::Stance);
        assert!(w >= 72.0 && h >= 36.0);
        let orig = Rect {
            x: 0.4,
            y: 0.4,
            w: 0.11,
            h: 0.065,
        };
        let mut cfg = HudConfig::new();
        set_rect(&mut cfg, WidgetId::Stance, orig);
        assert_eq!(rect_of(&cfg, WidgetId::Stance).w, orig.w);
        assert!(hit(&cfg, WidgetId::Stance, 0.455, 0.432, 1280.0, 720.0).is_some());
    }
}
