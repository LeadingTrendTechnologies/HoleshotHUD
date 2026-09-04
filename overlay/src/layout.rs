use crate::config::{HudConfig, WidgetId, COL_W_MIN, NAME_W_MAX};
use crate::render::table_layout_rect;
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
    Ticker,
    Sys,
    Sector,
    Delta,
    Stance,
    Flag,
    Lean,
    Gamepad,
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

impl Handle {
    fn changes_w(self) -> bool {
        matches!(self, Self::E | Self::W | Self::NE | Self::NW | Self::SE | Self::SW)
    }

    fn changes_h(self) -> bool {
        matches!(self, Self::N | Self::S | Self::NE | Self::NW | Self::SE | Self::SW)
    }
}

#[derive(Clone, Copy)]
struct Drag {
    target: Target,
    handle: Handle,
    grab_x: f32,
    grab_y: f32,
    orig: Rect,
    name_w: i32,
    rows: i32,
}

#[derive(Default)]
pub struct Editor {
    map: Option<Rect>,
    standings: Option<Rect>,
    relative: Option<Rect>,
    minimap: Option<Rect>,
    radar: Option<Rect>,
    dash: Option<Rect>,
    ticker: Option<Rect>,
    sys: Option<Rect>,
    sector: Option<Rect>,
    delta: Option<Rect>,
    stance: Option<Rect>,
    flag: Option<Rect>,
    lean: Option<Rect>,
    gamepad: Option<Rect>,
    st_w_name: Option<i32>,
    rel_w_name: Option<i32>,
    standings_rows: Option<i32>,
    relative_count: Option<i32>,
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
        if let Some(n) = self.standings_rows {
            s.standings_rows = n;
        }
        if let Some(n) = self.relative_count {
            s.relative_count = n;
        }
    }

    /// True when a drag preview must be written into a HudConfig copy for this frame.
    pub fn has_preview(&self) -> bool {
        self.minimap.is_some()
            || self.radar.is_some()
            || self.dash.is_some()
            || self.ticker.is_some()
            || self.sys.is_some()
            || self.sector.is_some()
            || self.delta.is_some()
            || self.stance.is_some()
            || self.flag.is_some()
            || self.lean.is_some()
            || self.gamepad.is_some()
            || self.st_w_name.is_some()
            || self.rel_w_name.is_some()
            || self.standings_rows.is_some()
            || self.relative_count.is_some()
    }

    pub fn apply_cfg(&self, cfg: &mut HudConfig) {
        if let Some(r) = self.minimap {
            cfg[WidgetId::Minimap].rect = r;
        }
        if let Some(r) = self.radar {
            cfg[WidgetId::Radar].rect = r;
        }
        if let Some(r) = self.dash {
            cfg[WidgetId::Dash].rect = r;
        }
        if let Some(t) = self.ticker {
            cfg[WidgetId::Ticker].rect = t;
        }
        if let Some(s) = self.sys {
            cfg[WidgetId::Sys].rect = s;
        }
        if let Some(s) = self.sector {
            cfg[WidgetId::Sector].rect = s;
        }
        if let Some(s) = self.delta {
            cfg[WidgetId::Delta].rect = s;
        }
        if let Some(s) = self.stance {
            cfg[WidgetId::Stance].rect = s;
        }
        if let Some(s) = self.flag {
            cfg[WidgetId::Flag].rect = s;
        }
        if let Some(s) = self.lean {
            cfg[WidgetId::Lean].rect = s;
        }
        if let Some(s) = self.gamepad {
            cfg[WidgetId::Gamepad].rect = s;
        }
        if let Some(r) = self.standings {
            cfg[WidgetId::Standings].rect = r;
        }
        if let Some(r) = self.relative {
            cfg[WidgetId::Relative].rect = r;
        }
        if let Some(w) = self.st_w_name {
            cfg.st_w_name = w;
        }
        if let Some(w) = self.rel_w_name {
            cfg.rel_w_name = w;
        }
        if let Some(n) = self.standings_rows {
            cfg.standings_rows = n;
        }
        if let Some(n) = self.relative_count {
            cfg.relative_count = n;
        }
    }

    /// Returns true when the drag should be written to config. Caller must
    /// [`commit`] after dropping any `with_config` guard — `save` takes that lock.
    pub fn tick(
        &mut self,
        overlay: HWND,
        ox: i32,
        oy: i32,
        ow: i32,
        oh: i32,
        snap: Option<&Snapshot>,
        cfg: &HudConfig,
        block_press: bool,
    ) -> bool {
        let ctrl = Self::ctrl_down();
        let down = unsafe { GetAsyncKeyState(VK_LBUTTON.0 as i32) < 0 };
        let pressed = down && !self.mouse_was_down && !block_press;
        let released = !down && self.mouse_was_down;
        self.mouse_was_down = down;

        let Some((nx, ny)) = cursor_norm(ox, oy, ow, oh) else {
            return false;
        };

        if !ctrl {
            let commit = self.drag.take().is_some();
            if !commit {
                self.clear_preview();
            }
            return commit;
        }

        if pressed {
            if let Some(s) = snap {
                if let Some((t, h)) = hit(s, self, cfg, nx, ny, ow, oh) {
                    let (name_w, rows) = match t {
                        Target::Standings => (cfg.st_w_name, cfg.standings_rows),
                        Target::Relative => (cfg.rel_w_name, cfg.relative_count),
                        _ => (0, 0),
                    };
                    self.drag = Some(Drag {
                        target: t,
                        handle: h,
                        grab_x: nx,
                        grab_y: ny,
                        orig: visual_rect(s, cfg, rect_of(s, self, cfg, t), t, ow, oh),
                        name_w,
                        rows,
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
                Target::Ticker => self.ticker = Some(r),
                Target::Sys => self.sys = Some(r),
                Target::Sector => self.sector = Some(r),
                Target::Delta => self.delta = Some(r),
                Target::Stance => self.stance = Some(r),
                Target::Flag => self.flag = Some(r),
                Target::Lean => self.lean = Some(r),
                Target::Gamepad => self.gamepad = Some(r),
            }
            apply_table_resize(self, cfg, d, r, ow, oh);
            let _ = overlay;
        }

        released && self.drag.take().is_some()
    }

    pub fn commit(&mut self, snap: Option<&Snapshot>) {
        self.save(snap);
        self.clear_preview();
    }

    fn clear_preview(&mut self) {
        self.map = None;
        self.standings = None;
        self.relative = None;
        self.minimap = None;
        self.radar = None;
        self.dash = None;
        self.ticker = None;
        self.sys = None;
        self.sector = None;
        self.delta = None;
        self.stance = None;
        self.flag = None;
        self.lean = None;
        self.gamepad = None;
        self.st_w_name = None;
        self.rel_w_name = None;
        self.standings_rows = None;
        self.relative_count = None;
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
        let ticker = self.ticker;
        let sys = self.sys;
        let sector = self.sector;
        let delta = self.delta;
        let stance = self.stance;
        let flag = self.flag;
        let lean = self.lean;
        let gamepad = self.gamepad;
        crate::config::update_config(|cfg| {
            cfg[WidgetId::Map].rect = map;
            cfg[WidgetId::Standings].rect = standings;
            cfg[WidgetId::Relative].rect = relative;
            if let Some(m) = minimap {
                cfg[WidgetId::Minimap].rect = m;
            }
            if let Some(r) = radar {
                cfg[WidgetId::Radar].rect = r;
            }
            if let Some(d) = dash {
                cfg[WidgetId::Dash].rect = d;
            }
            if let Some(t) = ticker {
                cfg[WidgetId::Ticker].rect = t;
            }
            if let Some(s) = sys {
                cfg[WidgetId::Sys].rect = s;
            }
            if let Some(s) = sector {
                cfg[WidgetId::Sector].rect = s;
            }
            if let Some(s) = delta {
                cfg[WidgetId::Delta].rect = s;
            }
            if let Some(s) = stance {
                cfg[WidgetId::Stance].rect = s;
            }
            if let Some(s) = flag {
                cfg[WidgetId::Flag].rect = s;
            }
            if let Some(s) = lean {
                cfg[WidgetId::Lean].rect = s;
            }
            if let Some(s) = gamepad {
                cfg[WidgetId::Gamepad].rect = s;
            }
            if let Some(w) = self.st_w_name {
                cfg.st_w_name = w;
            }
            if let Some(w) = self.rel_w_name {
                cfg.rel_w_name = w;
            }
            if let Some(n) = self.standings_rows {
                cfg.standings_rows = n;
            }
            if let Some(n) = self.relative_count {
                cfg.relative_count = n;
            }
        });
    }
}

pub(crate) fn cursor_norm(ox: i32, oy: i32, ow: i32, oh: i32) -> Option<(f32, f32)> {
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
    let rw = r.w * ow as f32;
    let rh = r.h * oh as f32;
    let d = rw.min(rh) * 0.5;
    let cx = r.x * ow as f32 + rw * 0.5;
    let cy = r.y * oh as f32 + rh * 0.5;
    let dx = px - cx;
    let dy = py - cy;
    dx * dx + dy * dy <= d * d
}

fn visual_rect(s: &Snapshot, cfg: &HudConfig, r: Rect, t: Target, ow: i32, oh: i32) -> Rect {
    match t {
        Target::Minimap => {
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
        Target::Standings => table_layout_rect(s, cfg, WidgetId::Standings, r, ow as f32, oh as f32),
        Target::Relative => table_layout_rect(s, cfg, WidgetId::Relative, r, ow as f32, oh as f32),
        _ => r,
    }
}

fn rect_of(s: &Snapshot, ed: &Editor, cfg: &HudConfig, t: Target) -> Rect {
    match t {
        Target::Map => ed.map.unwrap_or(s.map),
        Target::Standings => ed.standings.unwrap_or(s.standings_rect),
        Target::Relative => ed.relative.unwrap_or(s.relative),
        Target::Minimap => ed.minimap.unwrap_or(cfg[WidgetId::Minimap].rect),
        Target::Radar => ed.radar.unwrap_or(cfg[WidgetId::Radar].rect),
        Target::Dash => ed.dash.unwrap_or(cfg[WidgetId::Dash].rect),
        Target::Ticker => ed.ticker.unwrap_or(cfg[WidgetId::Ticker].rect),
        Target::Sys => ed.sys.unwrap_or(cfg[WidgetId::Sys].rect),
        Target::Sector => ed.sector.unwrap_or(cfg[WidgetId::Sector].rect),
        Target::Delta => ed.delta.unwrap_or(cfg[WidgetId::Delta].rect),
        Target::Stance => ed.stance.unwrap_or(cfg[WidgetId::Stance].rect),
        Target::Flag => ed.flag.unwrap_or(cfg[WidgetId::Flag].rect),
        Target::Lean => ed.lean.unwrap_or(cfg[WidgetId::Lean].rect),
        Target::Gamepad => ed.gamepad.unwrap_or(cfg[WidgetId::Gamepad].rect),
    }
}

fn shown(s: &Snapshot, cfg: &HudConfig, t: Target) -> bool {
    match t {
        Target::Map => s.show_map != 0,
        Target::Standings => s.show_standings != 0,
        Target::Relative => s.show_relative != 0,
        Target::Minimap => cfg[WidgetId::Minimap].show,
        Target::Radar => cfg[WidgetId::Radar].show,
        Target::Dash => cfg[WidgetId::Dash].show,
        Target::Ticker => cfg[WidgetId::Ticker].show,
        Target::Sys => cfg[WidgetId::Sys].show,
        Target::Sector => cfg.sector_visible(),
        Target::Delta => cfg.delta_visible(),
        Target::Stance => cfg.stance_visible(),
        Target::Flag => cfg[WidgetId::Flag].show,
        Target::Lean => cfg[WidgetId::Lean].show,
        Target::Gamepad => cfg.gamepad_visible(),
    }
}

fn hit(s: &Snapshot, ed: &Editor, cfg: &HudConfig, x: f32, y: f32, ow: i32, oh: i32) -> Option<(Target, Handle)> {
    const ORDER: [Target; 14] = [
        Target::Dash,
        Target::Ticker,
        Target::Sys,
        Target::Sector,
        Target::Delta,
        Target::Stance,
        Target::Flag,
        Target::Lean,
        Target::Gamepad,
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
        let stored = rect_of(s, ed, cfg, t);
        let r = visual_rect(s, cfg, stored, t, ow, oh);
        if let Some(h) = handle_at(r, x, y, ow, oh, t) {
            return Some((t, h));
        }
    }
    for t in ORDER {
        if !shown(s, cfg, t) {
            continue;
        }
        let stored = rect_of(s, ed, cfg, t);
        let r = visual_rect(s, cfg, stored, t, ow, oh);
        let inside = if t == Target::Minimap {
            contains_circle(stored, x, y, ow, oh)
        } else {
            contains(r, x, y)
        };
        if inside {
            return Some((t, Handle::Move));
        }
    }
    None
}

fn handle_at(r: Rect, nx: f32, ny: f32, ow: i32, oh: i32, t: Target) -> Option<Handle> {
    let px = nx * ow as f32;
    let py = ny * oh as f32;
    let x0 = r.x * ow as f32;
    let y0 = r.y * oh as f32;
    let x1 = (r.x + r.w) * ow as f32;
    let y1 = (r.y + r.h) * oh as f32;
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

fn apply_table_resize(ed: &mut Editor, cfg: &HudConfig, d: Drag, r: Rect, ow: i32, oh: i32) {
    if d.handle == Handle::Move {
        return;
    }
    let id = match d.target {
        Target::Standings => WidgetId::Standings,
        Target::Relative => WidgetId::Relative,
        _ => return,
    };
    let k = (cfg[id].font.clamp(70, 160) as f32) / 100.0;
    let row_h = 22.0 * k;
    if d.handle.changes_w() {
        let w = (d.name_w as f32 + (r.w - d.orig.w) * ow as f32).round() as i32;
        let w = w.clamp(COL_W_MIN, NAME_W_MAX);
        match d.target {
            Target::Standings => ed.st_w_name = Some(w),
            Target::Relative => ed.rel_w_name = Some(w),
            _ => {}
        }
    }
    if d.handle.changes_h() && row_h > 0.5 {
        let d_rows = ((r.h - d.orig.h) * oh as f32 / row_h).round() as i32;
        match d.target {
            Target::Standings => ed.standings_rows = Some((d.rows + d_rows).clamp(3, 40)),
            Target::Relative => {
                let orig_vis = 2 * d.rows + 1;
                let new_vis = (orig_vis + d_rows).max(1);
                ed.relative_count = Some(((new_vis - 1) / 2).clamp(1, 8));
            }
            _ => {}
        }
    }
}

fn min_px(t: Target) -> (f32, f32) {
    match t {
        Target::Standings => (140.0, 90.0),
        Target::Relative => (140.0, 70.0),
        Target::Map => (90.0, 90.0),
        Target::Minimap => (72.0, 72.0),
        Target::Radar => (72.0, 72.0),
        Target::Dash => (96.0, 48.0),
        Target::Ticker => (360.0, 44.0),
        Target::Sys => (160.0, 90.0),
        Target::Sector => (260.0, 72.0),
        Target::Delta => (220.0, 48.0),
        Target::Stance => (72.0, 36.0),
        Target::Flag => (72.0, 12.0),
        Target::Lean => (100.0, 72.0),
        Target::Gamepad => (160.0, 96.0),
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
    if target == Target::Ticker {
        y = orig.y;
        h = orig.h;
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
