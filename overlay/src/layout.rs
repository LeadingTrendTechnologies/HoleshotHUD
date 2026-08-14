use crate::shm::{Rect, Snapshot};
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_LBUTTON};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Target {
    Map,
    Standings,
    Relative,
}

#[derive(Clone, Copy)]
struct Drag {
    target: Target,
    grab_x: f32,
    grab_y: f32,
}

#[derive(Default)]
pub struct Editor {
    map: Option<Rect>,
    standings: Option<Rect>,
    relative: Option<Rect>,
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

    pub fn tick(&mut self, overlay: HWND, ox: i32, oy: i32, ow: i32, oh: i32, snap: Option<&Snapshot>) {
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
            return;
        }

        if pressed {
            if let Some(s) = snap {
                if let Some(t) = hit(s, self, nx, ny) {
                    let r = rect_of(s, self, t);
                    self.drag = Some(Drag {
                        target: t,
                        grab_x: nx - r.x,
                        grab_y: ny - r.y,
                    });
                }
            }
        }

        if let Some(d) = self.drag {
            let mut r = snap.map(|s| rect_of(s, self, d.target)).unwrap_or_default();
            r.x = (nx - d.grab_x).clamp(0.02 - r.w, 0.98);
            r.y = (ny - d.grab_y).clamp(0.02 - r.h, 0.98);
            match d.target {
                Target::Map => self.map = Some(r),
                Target::Standings => self.standings = Some(r),
                Target::Relative => self.relative = Some(r),
            }
            let _ = overlay;
        }

        if released && self.drag.take().is_some() {
            self.save(snap);
        }
    }

    fn save(&self, snap: Option<&Snapshot>) {
        let Some(s) = snap else {
            return;
        };
        let map = self.map.unwrap_or(s.map);
        let standings = self.standings.unwrap_or(s.standings_rect);
        let relative = self.relative.unwrap_or(s.relative);
        let path = ini_path();
        let body = format!(
            "# mxbo HUD layout (normalized 0..1, origin top-left)\n\
             [Layout]\n\
             standings_x={}\nstandings_y={}\nstandings_w={}\nstandings_h={}\n\
             relative_x={}\nrelative_y={}\nrelative_w={}\nrelative_h={}\n\
             map_x={}\nmap_y={}\nmap_w={}\nmap_h={}\n\
             \n[Widgets]\n\
             show_standings={}\nshow_relative={}\nshow_map={}\n\
             ingame_hud=0\nstandings_rows={}\nrelative_count={}\n",
            standings.x,
            standings.y,
            standings.w,
            standings.h,
            relative.x,
            relative.y,
            relative.w,
            relative.h,
            map.x,
            map.y,
            map.w,
            map.h,
            s.show_standings,
            s.show_relative,
            s.show_map,
            s.standings_rows,
            s.relative_count,
        );
        let _ = std::fs::write(path, body);
    }
}

fn ini_path() -> std::path::PathBuf {
    let home = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\Public".into());
    std::path::PathBuf::from(home)
        .join("Documents")
        .join("PiBoSo")
        .join("MX Bikes")
        .join("mxbo.ini")
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

fn rect_of(s: &Snapshot, ed: &Editor, t: Target) -> Rect {
    match t {
        Target::Map => ed.map.unwrap_or(s.map),
        Target::Standings => ed.standings.unwrap_or(s.standings_rect),
        Target::Relative => ed.relative.unwrap_or(s.relative),
    }
}

fn hit(s: &Snapshot, ed: &Editor, x: f32, y: f32) -> Option<Target> {
    if s.show_map != 0 && contains(rect_of(s, ed, Target::Map), x, y) {
        return Some(Target::Map);
    }
    if s.show_relative != 0 && contains(rect_of(s, ed, Target::Relative), x, y) {
        return Some(Target::Relative);
    }
    if s.show_standings != 0 && contains(rect_of(s, ed, Target::Standings), x, y) {
        return Some(Target::Standings);
    }
    None
}
