use crate::config::HudConfig;
use crate::shm::{Rect, Snapshot};
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_LBUTTON};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Target {
    Map,
    Standings,
    Relative,
    Minimap,
    Radar,
    Dash,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Handle {
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

#[derive(Clone, Copy)]
struct Drag {
    target: Target,
    handle: Handle,
    grab_x: f32,
    grab_y: f32,
    orig: Rect,
}

#[derive(Default)]
pub struct Editor {
    map: Option<Rect>,
    standings: Option<Rect>,
    relative: Option<Rect>,
    minimap: Option<Rect>,
    radar: Option<Rect>,
    dash: Option<Rect>,
    drag: Option<Drag>,
    mouse_was_down: bool,
}

impl Editor {
    pub fn ctrl_down() -> bool {
        unsafe { GetAsyncKeyState(VK_CONTROL.0 as i32) < 0 }
    }

    pub fn apply(&self, s: &mut Snapshot) {
        if let Some(r) = self.map {
            s.map = r;
        }
        if let Some(r) = self.standings {
            s.standings_rect = r;
        }
        if let Some(r) = self.relative {
            s.relative = r;
        }
    }

    pub fn apply_cfg(&self, cfg: &mut HudConfig) {
        if let Some(r) = self.minimap {
            cfg.minimap = r;
        }
        if let Some(r) = self.radar {
            cfg.radar = r;
        }
        if let Some(r) = self.dash {
            cfg.dash = r;
        }
    }

    pub fn tick(
        &mut self,
        overlay: HWND,
        ox: i32,
        oy: i32,
        ow: i32,
        oh: i32,
        snap: Option<&Snapshot>,
        cfg: &HudConfig,
    ) {
        let ctrl = Self::ctrl_down();
        let down = unsafe { GetAsyncKeyState(VK_LBUTTON.0 as i32) < 0 };
        let pressed = down && !self.mouse_was_down;
        let released = !down && self.mouse_was_down;
        self.mouse_was_down = down;

        let Some((nx, ny)) = cursor_norm(ox, oy, ow, oh) else {
            return;
        };

        if !ctrl {
            if self.drag.take().is_some() {
                self.save(snap);
            }
            self.clear_preview();
            return;
        }

        if pressed {
            if let Some(s) = snap {
                if let Some((t, h)) = hit(s, self, cfg, nx, ny, ow, oh) {
                    self.drag = Some(Drag {
                        target: t,
                        handle: h,
                        grab_x: nx,
                        grab_y: ny,
                        orig: rect_of(s, self, cfg, t),
                    });
                }
            }
        }

        if let Some(d) = self.drag {
            let r = resize(d.orig, d.handle, nx, ny, d.grab_x, d.grab_y, ow, oh, d.target);
            match d.target {
                Target::Map => self.map = Some(r),
                Target::Standings => self.standings = Some(r),
                Target::Relative => self.relative = Some(r),
                Target::Minimap => self.minimap = Some(r),
                Target::Radar => self.radar = Some(r),
                Target::Dash => self.dash = Some(r),
            }
            let _ = overlay;
        }

        if released && self.drag.take().is_some() {
            self.save(snap);
            self.clear_preview();
        }
    }

    fn clear_preview(&mut self) {
        self.map = None;
        self.standings = None;
        self.relative = None;
        self.minimap = None;
        self.radar = None;
        self.dash = None;
    }

    fn save(&self, snap: Option<&Snapshot>) {
        let Some(s) = snap else {
            return;
        };
        let map = self.map.unwrap_or(s.map);
        let standings = self.standings.unwrap_or(s.standings_rect);
        let relative = self.relative.unwrap_or(s.relative);
        let minimap = self.minimap;
        let radar = self.radar;
        let dash = self.dash;
        crate::config::update_config(|cfg| {
            cfg.map = map;
            cfg.standings = standings;
            cfg.relative = relative;
            if let Some(m) = minimap {
                cfg.minimap = m;
            }
            if let Some(r) = radar {
                cfg.radar = r;
            }
            if let Some(d) = dash {
                cfg.dash = d;
            }
        });
    }
}

fn cursor_norm(ox: i32, oy: i32, ow: i32, oh: i32) -> Option<(f32, f32)> {
    if ow <= 0 || oh <= 0 {
        return None;
    }
    let mut p = POINT::default();
    unsafe {
        GetCursorPos(&mut p).ok()?;
    }
    Some((
        (p.x - ox) as f32 / ow as f32,
        (p.y - oy) as f32 / oh as f32,
    ))
}

fn contains(r: Rect, x: f32, y: f32) -> bool {
    x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h
}

fn contains_circle(r: Rect, x: f32, y: f32, ow: i32, oh: i32) -> bool {
    let px = x * ow as f32;
    let py = y * oh as f32;
    let vis = visual_rect(r, Target::Minimap, ow, oh);
    let rx = vis.x * ow as f32;
    let ry = vis.y * oh as f32;
    let rw = vis.w * ow as f32;
    let rh = vis.h * oh as f32;
    let d = rw.min(rh) * 0.5;
    let cx = rx + rw * 0.5;
    let cy = ry + rh * 0.5;
    let dx = px - cx;
    let dy = py - cy;
    dx * dx + dy * dy <= d * d
}

fn visual_rect(r: Rect, t: Target, ow: i32, oh: i32) -> Rect {
    if t != Target::Minimap {
        return r;
    }
    let rw = r.w * ow as f32;
    let rh = r.h * oh as f32;
    let d = rw.min(rh);
    let cx = r.x * ow as f32 + rw * 0.5;
    let cy = r.y * oh as f32 + rh * 0.5;
    Rect {
        x: (cx - d * 0.5) / ow as f32,
        y: (cy - d * 0.5) / oh as f32,
        w: d / ow as f32,
        h: d / oh as f32,
    }
}

fn rect_of(s: &Snapshot, ed: &Editor, cfg: &HudConfig, t: Target) -> Rect {
    match t {
        Target::Map => ed.map.unwrap_or(s.map),
        Target::Standings => ed.standings.unwrap_or(s.standings_rect),
        Target::Relative => ed.relative.unwrap_or(s.relative),
        Target::Minimap => ed.minimap.unwrap_or(cfg.minimap),
        Target::Radar => ed.radar.unwrap_or(cfg.radar),
        Target::Dash => {
            let mut r = ed.dash.unwrap_or_else(crate::render::dash_visual);
            let vis = crate::render::dash_visual();
            r.w = vis.w;
            r
        }
    }
}

fn shown(s: &Snapshot, cfg: &HudConfig, t: Target) -> bool {
    match t {
        Target::Map => s.show_map != 0,
        Target::Standings => s.show_standings != 0,
        Target::Relative => s.show_relative != 0,
        Target::Minimap => cfg.show_minimap,
        Target::Radar => cfg.show_radar,
        Target::Dash => cfg.show_dash,
    }
}

fn hit(s: &Snapshot, ed: &Editor, cfg: &HudConfig, x: f32, y: f32, ow: i32, oh: i32) -> Option<(Target, Handle)> {
    const ORDER: [Target; 6] = [
        Target::Dash,
        Target::Minimap,
        Target::Radar,
        Target::Map,
        Target::Relative,
        Target::Standings,
    ];
    for t in ORDER {
        if !shown(s, cfg, t) {
            continue;
        }
        let r = visual_rect(rect_of(s, ed, cfg, t), t, ow, oh);
        if let Some(h) = handle_at(r, x, y, ow, oh) {
            return Some((t, h));
        }
    }
    for t in ORDER {
        if !shown(s, cfg, t) {
            continue;
        }
        let r = rect_of(s, ed, cfg, t);
        let inside = if t == Target::Minimap {
            contains_circle(r, x, y, ow, oh)
        } else {
            contains(r, x, y)
        };
        if inside {
            return Some((t, Handle::Move));
        }
    }
    None
}

fn handle_at(r: Rect, nx: f32, ny: f32, ow: i32, oh: i32) -> Option<Handle> {
    let px = nx * ow as f32;
    let py = ny * oh as f32;
    let x0 = r.x * ow as f32;
    let y0 = r.y * oh as f32;
    let x1 = (r.x + r.w) * ow as f32;
    let y1 = (r.y + r.h) * oh as f32;
    let s = 12.0;
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

fn min_px(t: Target) -> (f32, f32) {
    match t {
        Target::Standings => (140.0, 90.0),
        Target::Relative => (140.0, 70.0),
        Target::Map => (90.0, 90.0),
        Target::Minimap => (72.0, 72.0),
        Target::Radar => (72.0, 72.0),
        Target::Dash => (220.0, 100.0),
    }
}

fn resize(
    orig: Rect,
    handle: Handle,
    nx: f32,
    ny: f32,
    grab_x: f32,
    grab_y: f32,
    ow: i32,
    oh: i32,
    target: Target,
) -> Rect {
    let (min_w_px, min_h_px) = min_px(target);
    let min_w = min_w_px / ow as f32;
    let min_h = min_h_px / oh as f32;
    let square = target == Target::Minimap;
    if handle == Handle::Move {
        let mut r = orig;
        r.x = (orig.x + nx - grab_x).clamp(0.01 - r.w, 0.99);
        r.y = (orig.y + ny - grab_y).clamp(0.01 - r.h, 0.99);
        return r;
    }
    if square {
        return resize_square(orig, handle, nx, ny, ow, oh, min_w_px);
    }
    let right = orig.x + orig.w;
    let bottom = orig.y + orig.h;
    let mut x = orig.x;
    let mut y = orig.y;
    let mut w = orig.w;
    let mut h = orig.h;
    match handle {
        Handle::Move => {}
        Handle::E | Handle::NE | Handle::SE => w = (nx - orig.x).max(min_w),
        Handle::W | Handle::NW | Handle::SW => {
            x = nx.min(right - min_w);
            w = right - x;
        }
        Handle::N | Handle::S => {}
    }
    match handle {
        Handle::Move => {}
        Handle::S | Handle::SE | Handle::SW => h = (ny - orig.y).max(min_h),
        Handle::N | Handle::NE | Handle::NW => {
            y = ny.min(bottom - min_h);
            h = bottom - y;
        }
        Handle::E | Handle::W => {}
    }
    Rect { x, y, w, h }
}

fn resize_square(orig: Rect, handle: Handle, nx: f32, ny: f32, ow: i32, oh: i32, min_px: f32) -> Rect {
    let ow = ow as f32;
    let oh = oh as f32;
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
