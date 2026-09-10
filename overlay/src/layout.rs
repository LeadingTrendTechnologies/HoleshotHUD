use crate::config::{HudConfig, WidgetId, COL_W_MIN, NAME_W_MAX};
use crate::render::table_layout_rect;
use crate::shm::{Rect, Snapshot};
use mxbo_hud::layout::Handle;
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_LBUTTON};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

#[derive(Clone, Copy)]
struct Drag {
    target: WidgetId,
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
    telemetry: Option<Rect>,
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
            || self.telemetry.is_some()
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
        if let Some(s) = self.telemetry {
            cfg[WidgetId::Telemetry].rect = s;
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
                        WidgetId::Standings => (cfg.st_w_name, cfg.standings_rows),
                        WidgetId::Relative => (cfg.rel_w_name, cfg.relative_count),
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
            let r = mxbo_hud::layout::resize(
                d.orig,
                d.handle,
                nx,
                ny,
                d.grab_x,
                d.grab_y,
                ow as f32,
                oh as f32,
                d.target,
            );
            match d.target {
                WidgetId::Map => self.map = Some(r),
                WidgetId::Standings => self.standings = Some(r),
                WidgetId::Relative => self.relative = Some(r),
                WidgetId::Minimap => self.minimap = Some(r),
                WidgetId::Radar => self.radar = Some(r),
                WidgetId::Dash => self.dash = Some(r),
                WidgetId::Ticker => self.ticker = Some(r),
                WidgetId::Sys => self.sys = Some(r),
                WidgetId::Sector => self.sector = Some(r),
                WidgetId::Delta => self.delta = Some(r),
                WidgetId::Stance => self.stance = Some(r),
                WidgetId::Flag => self.flag = Some(r),
                WidgetId::Lean => self.lean = Some(r),
                WidgetId::Gamepad => self.gamepad = Some(r),
                WidgetId::Telemetry => self.telemetry = Some(r),
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
        self.telemetry = None;
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
        let telemetry = self.telemetry;
        crate::config::update_config(|cfg| {
            let live = cfg.live_mut();
            live[WidgetId::Map].rect = map;
            live[WidgetId::Standings].rect = standings;
            live[WidgetId::Relative].rect = relative;
            if let Some(m) = minimap {
                live[WidgetId::Minimap].rect = m;
            }
            if let Some(r) = radar {
                live[WidgetId::Radar].rect = r;
            }
            if let Some(d) = dash {
                live[WidgetId::Dash].rect = d;
            }
            if let Some(t) = ticker {
                live[WidgetId::Ticker].rect = t;
            }
            if let Some(s) = sys {
                live[WidgetId::Sys].rect = s;
            }
            if let Some(s) = sector {
                live[WidgetId::Sector].rect = s;
            }
            if let Some(s) = delta {
                live[WidgetId::Delta].rect = s;
            }
            if let Some(s) = stance {
                live[WidgetId::Stance].rect = s;
            }
            if let Some(s) = flag {
                live[WidgetId::Flag].rect = s;
            }
            if let Some(s) = lean {
                live[WidgetId::Lean].rect = s;
            }
            if let Some(s) = gamepad {
                live[WidgetId::Gamepad].rect = s;
            }
            if let Some(s) = telemetry {
                live[WidgetId::Telemetry].rect = s;
            }
            if let Some(w) = self.st_w_name {
                live.st_w_name = w;
            }
            if let Some(w) = self.rel_w_name {
                live.rel_w_name = w;
            }
            if let Some(n) = self.standings_rows {
                live.standings_rows = n;
            }
            if let Some(n) = self.relative_count {
                live.relative_count = n;
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

fn visual_rect(s: &Snapshot, cfg: &HudConfig, r: Rect, t: WidgetId, ow: i32, oh: i32) -> Rect {
    match t {
        WidgetId::Minimap => mxbo_hud::layout::visual_rect(r, t, ow as f32, oh as f32),
        WidgetId::Standings => table_layout_rect(s, cfg, WidgetId::Standings, r, ow as f32, oh as f32),
        WidgetId::Relative => table_layout_rect(s, cfg, WidgetId::Relative, r, ow as f32, oh as f32),
        _ => r,
    }
}

fn rect_of(s: &Snapshot, ed: &Editor, cfg: &HudConfig, t: WidgetId) -> Rect {
    match t {
        WidgetId::Map => ed.map.unwrap_or(s.map),
        WidgetId::Standings => ed.standings.unwrap_or(s.standings_rect),
        WidgetId::Relative => ed.relative.unwrap_or(s.relative),
        WidgetId::Minimap => ed.minimap.unwrap_or(cfg[WidgetId::Minimap].rect),
        WidgetId::Radar => ed.radar.unwrap_or(cfg[WidgetId::Radar].rect),
        WidgetId::Dash => ed.dash.unwrap_or(cfg[WidgetId::Dash].rect),
        WidgetId::Ticker => ed.ticker.unwrap_or(cfg[WidgetId::Ticker].rect),
        WidgetId::Sys => ed.sys.unwrap_or(cfg[WidgetId::Sys].rect),
        WidgetId::Sector => ed.sector.unwrap_or(cfg[WidgetId::Sector].rect),
        WidgetId::Delta => ed.delta.unwrap_or(cfg[WidgetId::Delta].rect),
        WidgetId::Stance => ed.stance.unwrap_or(cfg[WidgetId::Stance].rect),
        WidgetId::Flag => ed.flag.unwrap_or(cfg[WidgetId::Flag].rect),
        WidgetId::Lean => ed.lean.unwrap_or(cfg[WidgetId::Lean].rect),
        WidgetId::Gamepad => ed.gamepad.unwrap_or(cfg[WidgetId::Gamepad].rect),
        WidgetId::Telemetry => ed.telemetry.unwrap_or(cfg[WidgetId::Telemetry].rect),
    }
}

fn shown(s: &Snapshot, cfg: &HudConfig, t: WidgetId) -> bool {
    match t {
        WidgetId::Map => s.show_map != 0,
        WidgetId::Standings => s.show_standings != 0,
        WidgetId::Relative => s.show_relative != 0,
        WidgetId::Minimap => cfg[WidgetId::Minimap].show,
        WidgetId::Radar => cfg[WidgetId::Radar].show,
        WidgetId::Dash => cfg[WidgetId::Dash].show,
        WidgetId::Ticker => cfg[WidgetId::Ticker].show,
        WidgetId::Sys => cfg[WidgetId::Sys].show,
        WidgetId::Sector => cfg.sector_visible(),
        WidgetId::Delta => cfg.delta_visible(),
        WidgetId::Stance => cfg.stance_visible(),
        WidgetId::Flag => cfg[WidgetId::Flag].show,
        WidgetId::Lean => cfg[WidgetId::Lean].show,
        WidgetId::Gamepad => cfg.gamepad_visible(),
        WidgetId::Telemetry => cfg[WidgetId::Telemetry].show,
    }
}

fn hit(s: &Snapshot, ed: &Editor, cfg: &HudConfig, x: f32, y: f32, ow: i32, oh: i32) -> Option<(WidgetId, Handle)> {
    const ORDER: [WidgetId; 15] = [
        WidgetId::Dash,
        WidgetId::Ticker,
        WidgetId::Sys,
        WidgetId::Sector,
        WidgetId::Delta,
        WidgetId::Stance,
        WidgetId::Flag,
        WidgetId::Lean,
        WidgetId::Gamepad,
        WidgetId::Telemetry,
        WidgetId::Minimap,
        WidgetId::Radar,
        WidgetId::Map,
        WidgetId::Relative,
        WidgetId::Standings,
    ];
    for t in ORDER {
        if !shown(s, cfg, t) {
            continue;
        }
        let stored = rect_of(s, ed, cfg, t);
        let r = visual_rect(s, cfg, stored, t, ow, oh);
        if let Some(h) = mxbo_hud::layout::handle_at(r, x, y, ow as f32, oh as f32, t) {
            return Some((t, h));
        }
    }
    for t in ORDER {
        if !shown(s, cfg, t) {
            continue;
        }
        let stored = rect_of(s, ed, cfg, t);
        let r = visual_rect(s, cfg, stored, t, ow, oh);
        let inside = if t == WidgetId::Minimap {
            mxbo_hud::layout::contains_circle(stored, x, y, ow as f32, oh as f32)
        } else {
            mxbo_hud::layout::contains(r, x, y)
        };
        if inside {
            return Some((t, Handle::Move));
        }
    }
    None
}

fn apply_table_resize(ed: &mut Editor, cfg: &HudConfig, d: Drag, r: Rect, ow: i32, oh: i32) {
    if d.handle == Handle::Move {
        return;
    }
    let id = match d.target {
        WidgetId::Standings => WidgetId::Standings,
        WidgetId::Relative => WidgetId::Relative,
        _ => return,
    };
    let k = (cfg[id].font.clamp(70, 160) as f32) / 100.0;
    let row_h = 22.0 * k;
    if d.handle.changes_w() {
        let w = (d.name_w as f32 + (r.w - d.orig.w) * ow as f32).round() as i32;
        let w = w.clamp(COL_W_MIN, NAME_W_MAX);
        match d.target {
            WidgetId::Standings => ed.st_w_name = Some(w),
            WidgetId::Relative => ed.rel_w_name = Some(w),
            _ => {}
        }
    }
    if d.handle.changes_h() && row_h > 0.5 {
        let d_rows = ((r.h - d.orig.h) * oh as f32 / row_h).round() as i32;
        match d.target {
            WidgetId::Standings => ed.standings_rows = Some((d.rows + d_rows).clamp(3, 40)),
            WidgetId::Relative => {
                let orig_vis = 2 * d.rows + 1;
                let new_vis = (orig_vis + d_rows).max(1);
                ed.relative_count = Some(((new_vis - 1) / 2).clamp(1, 8));
            }
            _ => {}
        }
    }
}
