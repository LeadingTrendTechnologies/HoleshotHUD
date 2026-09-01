#![windows_subsystem = "windows"]

mod changelog;
mod compat;
mod config;
mod feedback;
mod layout;
mod record;
mod render;
mod settings;
mod plugin;
mod shm;
mod stance;
mod startup;
mod sys;
mod tray;
mod uninstall;
mod update;
mod util;

use std::mem::size_of;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::time::{Duration, Instant};

use tiny_skia::Pixmap;
use windows::core::w;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    ClientToScreen, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject,
    AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS,
    HBITMAP, HDC, HGDIOBJ,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};
use windows::Win32::System::Threading::Sleep;
use windows::Win32::UI::HiDpi::{SetProcessDpiAwareness, PROCESS_PER_MONITOR_DPI_AWARE};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_F9, VK_LBUTTON};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateIconFromResourceEx, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    GetClassNameW, GetClientRect, GetSystemMetrics, GetWindowThreadProcessId, IsIconic, IsWindow,
    IsWindowVisible, LoadCursorW,
    LoadImageW, LookupIconIdFromDirectoryEx, PeekMessageW, PostQuitMessage, RegisterClassExW,
    SendMessageW, SetCursor, SetWindowPos, ShowWindow, TranslateMessage, UpdateLayeredWindow,
    HWND_TOPMOST, ICON_BIG, ICON_SMALL, IDC_ARROW, IDC_HAND, IMAGE_ICON, LR_DEFAULTCOLOR, MSG, PM_REMOVE,
    SM_CXSCREEN, SM_CYSCREEN, SWP_NOACTIVATE, SWP_SHOWWINDOW,
    SW_SHOWNOACTIVATE, ULW_ALPHA, WM_ACTIVATE, WM_CLOSE, WM_DESTROY, WM_QUIT, WM_SETCURSOR,
    WM_SETICON, WNDCLASSEXW, WS_CAPTION, WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_MINIMIZEBOX, WS_OVERLAPPED, WS_POPUP,
    WS_SYSMENU,
};

use crate::render::Fonts;
use crate::shm::{Cmd, Shm, Snapshot, VERSION};

static HOST: AtomicIsize = AtomicIsize::new(0);
static QUITTING: AtomicBool = AtomicBool::new(false);

fn host_hwnd() -> HWND {
    HWND(HOST.load(Ordering::SeqCst) as *mut _)
}

fn set_host(hwnd: HWND) {
    HOST.store(hwnd.0 as isize, Ordering::SeqCst);
}

/// Save is caller's job. Tears down tray/windows and ends the process so the exe unlocks for rebuilds.
pub(crate) fn quit_app() {
    if QUITTING.swap(true, Ordering::SeqCst) {
        return;
    }
    crate::compat::restore_taskbars();
    crate::startup::kill_other_hud_processes();
    crate::tray::remove();
    crate::compat::stop_background_threads();
    unsafe {
        let host = host_hwnd();
        if !host.0.is_null() && IsWindow(host).as_bool() {
            let _ = DestroyWindow(host);
        }
        set_host(HWND(null_mut()));
        PostQuitMessage(0);
    }
    std::process::exit(0);
}

fn write_shm_dump(text: &str) -> Option<std::path::PathBuf> {
    eprint!("\n*** F9 SHM dump ***\n{text}");
    let mut paths = Vec::new();
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        paths.push(
            std::path::PathBuf::from(local)
                .join("Holeshot HUD")
                .join("logs")
                .join("snapshot.txt"),
        );
    }
    paths.push(std::env::temp_dir().join("Holeshot HUD").join("logs").join("snapshot.txt"));
    for path in paths {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if std::fs::write(&path, text.as_bytes()).is_ok() {
            eprintln!("*** wrote {} ***\n", path.display());
            return Some(path);
        }
    }
    eprintln!("*** F9 dump failed to write snapshot.txt ***\n");
    None
}

fn f9_dump_text(shm: Option<&Shm>, snap: Option<&Snapshot>) -> String {
    if let Some(s) = snap {
        return s.dump_text();
    }
    let mut o = String::from("No live Snapshot (overlay SHM version mismatch or plugin not publishing).\n");
    o.push_str(&format!("overlay VERSION={VERSION} rust_size={}\n", std::mem::size_of::<Snapshot>()));
    match shm {
        None => o.push_str("OpenFileMapping Local\\MXBOHudV10 failed. Start MX Bikes with Holeshot-HUD.dlo loaded.\n"),
        Some(s) => match s.header() {
            Some((magic, version, seq, size)) => {
                o.push_str(&format!(
                    "SHM header magic={magic:#x} version={version} seq={seq} size={size}\n"
                ));
                if version != VERSION {
                    o.push_str(
                        "Restart MX Bikes after build.bat so the plugin matches this overlay.\n",
                    );
                }
            }
            None => o.push_str("SHM view is null.\n"),
        },
    }
    o
}

fn dump_whats_new_path() -> Option<std::path::PathBuf> {
    let mut args = std::env::args();
    while let Some(a) = args.next() {
        if a == "--dump-whats-new" {
            return Some(
                args.next()
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|| std::path::PathBuf::from("announce-shots/whats-new.png")),
            );
        }
    }
    None
}

fn dump_whats_new_and_exit(path: &std::path::Path) -> ! {
    match crate::settings::dump_whats_new(path) {
        Ok(notes) => {
            let text = crate::changelog::format_notes(&notes);
            let txt = path.with_extension("txt");
            let _ = std::fs::write(&txt, &text);
            eprintln!("{text}");
            eprintln!("Wrote {} and {}", path.display(), txt.display());
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}

fn main() {
    std::panic::set_hook(Box::new(|info| {
        let text = format!("{info}\n{}", std::backtrace::Backtrace::force_capture());
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            let dir = std::path::PathBuf::from(local).join("Holeshot HUD").join("logs");
            let _ = std::fs::create_dir_all(&dir);
                let _ = std::fs::write(dir.join("panic.txt"), text.as_bytes());
        }
    }));
    if let Some(path) = dump_whats_new_path() {
        dump_whats_new_and_exit(&path);
    }
    if std::env::args().any(|a| a == "--wait-for-game") {
        crate::startup::wait_for_mx_bikes();
    } else if !crate::startup::take_hud_instance() {
        return;
    }
    let loaded = crate::config::HudConfig::load_file();
    if loaded.auto_update_on_launch && crate::update::apply_on_launch() {
        return;
    }
    if !loaded.auto_update_on_launch {
        crate::update::check();
    }
    crate::record::init();
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        let dir = std::path::PathBuf::from(local).join("Holeshot HUD").join("track-pbs");
        mxbo_hud::track_pb::set_store_dir(dir);
    }
    let clock_log_path = crate::record::path();
    match clock_log_path {
        Some(p) => mxbo_hud::set_status_hint(format!("Clock log: {}", p.display())),
        None => mxbo_hud::set_status_hint("Clock log failed — see AppData\\Local\\Holeshot HUD\\logs\\boot.txt"),
    }
    crate::plugin::sync();
    crate::plugin::apply_updater_plugin_flag();
    let family = loaded.font_family;
    *crate::config::CONFIG.lock().unwrap_or_else(|e| e.into_inner()) = loaded;
    let fonts = Fonts::for_family(family)
        .or_else(Fonts::load)
        .expect("need a HUD font (bundled or Windows\\Fonts)");
    unsafe { run(fonts, family) }
}

unsafe fn run(mut fonts: Fonts, mut font_family: crate::config::FontFamily) {
    let _ = SetProcessDpiAwareness(PROCESS_PER_MONITOR_DPI_AWARE);
    let hinst = GetModuleHandleW(None).unwrap();
    let icon_bytes = include_bytes!("../icon.ico");
    let icon_big = load_app_icon(hinst, icon_bytes, 256);
    let icon_small = load_app_icon(hinst, icon_bytes, 32);
    let class = w!("MXBOOverlay");
    let wc = WNDCLASSEXW {
        cbSize: size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: Some(wndproc),
        hInstance: hinst.into(),
        lpszClassName: class,
        hIcon: icon_big,
        hIconSm: icon_small,
        hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
        ..Default::default()
    };
    RegisterClassExW(&wc);

    let host = CreateWindowExW(
        WS_EX_APPWINDOW,
        class,
        w!("Holeshot HUD — Settings"),
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
        80,
        80,
        1000,
        720,
        None,
        None,
        hinst,
        None,
    )
    .expect("host window");
    set_host(host);
    crate::settings::attach(host);
    crate::startup::sync_from_config();
    apply_window_icons(host, icon_big, icon_small);
    crate::tray::add(host, icon_small);
    let start_minimized = std::env::args().any(|a| a == "--minimized" || a == "--wait-for-game");
    let show_notes = crate::changelog::force_whats_new()
        || {
            let seen = crate::config::with_config(|c| c.whats_new_seen.clone());
            crate::changelog::should_auto_open(
                &seen,
                crate::update::current_version(),
                crate::changelog::just_updated(),
            )
        };
    // Registry / MX Bikes waiter pass --minimized. Stay in the tray (Hide), and do
    // not let WM_ACTIVATE pop Settings back up.
    if start_minimized && !show_notes {
        crate::settings::hide(host);
    } else {
        crate::settings::show(host);
    }
    if show_notes {
        crate::settings::open_whats_new();
    }

    let mut game = find_game_hwnd();
    let mut last_game_pid = crate::startup::mx_bikes_pid();
    let mut restart_hint = false;
    if let Some(path) = crate::plugin::game_exe() {
        let wrote = compat::ensure_disable_fullscreen_optimizations(&path);
        restart_hint = wrote && last_game_pid.is_some();
    }
    if let Some(path) = last_game_pid.and_then(compat::exe_path_for_pid) {
        restart_hint = compat::ensure_disable_fullscreen_optimizations(&path);
    }
    let (mut x, mut y, mut w, mut h) = game
        .and_then(client_screen_rect)
        .unwrap_or_else(primary_screen);
    let mut hwnd = create_overlay(hinst, class, x, y, w, h);

    let mut dib = Dib::new(w, h);
    let mut pixmap = Pixmap::new(w as u32, h as u32).unwrap();
    dib.present(hwnd, w, h, x, y);
    let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);

    let mut shm = Shm::open();
    let mut cmd = Cmd::open();
    let mut last_snap = None;
    let mut freq = 0i64;
    QueryPerformanceFrequency(&mut freq).ok();
    let mut zfix = compat::FullscreenFix::new();
    let mut editor = crate::layout::Editor::default();
    let mut sys = crate::sys::Sampler::default();
    let mut stance = crate::stance::Tracker::default();
    let mut f8_was = false;
    let mut f9_was = false;
    let mut spectate_down = false;
    let mut live_mark: Option<(f32, f32, f32, f32)> = None;
    let mut next_game_scan = Instant::now();
    let mut placed = false;
    let mut saw_game = crate::startup::mx_bikes_pid().is_some();
    let mut game_gone_at: Option<Instant> = None;
    let mut shm_miss_since: Option<Instant> = None;

    let mut msg = MSG::default();
    loop {
        while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
            if msg.message == WM_QUIT || crate::update::should_quit() || QUITTING.load(Ordering::SeqCst) {
                quit_app();
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        if QUITTING.load(Ordering::SeqCst) || crate::update::should_quit() {
            quit_app();
        }

        if !IsWindow(hwnd).as_bool() {
            game = find_game_hwnd();
            let r = game
                .and_then(client_screen_rect)
                .unwrap_or_else(primary_screen);
            x = r.0;
            y = r.1;
            w = r.2;
            h = r.3;
            hwnd = create_overlay(hinst, class, x, y, w, h);
            dib = Dib::new(w, h);
            pixmap = Pixmap::new(w as u32, h as u32).unwrap();
            placed = false;
        }

        if game.is_none() || Instant::now() >= next_game_scan {
            let next_game = find_game_hwnd();
            if next_game != game {
                game = next_game;
                placed = false;
            }
            next_game_scan = Instant::now() + Duration::from_millis(500);
            let pid = crate::startup::mx_bikes_pid();
            let game_on = pid.is_some();
            if pid != last_game_pid {
                let had_game = last_game_pid.is_some();
                last_game_pid = pid;
                if let Some(path) = pid.and_then(compat::exe_path_for_pid) {
                    let wrote = compat::ensure_disable_fullscreen_optimizations(&path);
                    restart_hint = wrote && had_game;
                } else {
                    restart_hint = false;
                    shm = None;
                    cmd = None;
                    last_snap = None;
                }
            }
            if !game_on {
                crate::plugin::retry_if_needed();
                crate::plugin::clear_game_restart();
            }
            if game_on {
                if !saw_game && crate::config::with_config(|c| c.minimize_on_close) {
                    crate::settings::hide(host);
                }
                saw_game = true;
                game_gone_at = None;
            } else if saw_game {
                let gone = *game_gone_at.get_or_insert_with(Instant::now);
                let (close, reopen) = crate::config::with_config(|c| (c.close_with_game, c.open_with_game));
                if close && gone.elapsed() >= Duration::from_secs(3) {
                    if reopen {
                        // Stay in the tray. A child waiter used to die with this process.
                        crate::settings::hide(host);
                        saw_game = false;
                        game_gone_at = None;
                    } else {
                        crate::config::update_config(|_| {});
                        quit_app();
                    }
                }
            }
        }
        let overlay_on = zfix.keep_overlay_above(hwnd, game, host);
        if overlay_on {
            crate::settings::keep_above_overlay(host);
        }
        if let Some(g) = game {
            if let Some((nx, ny, nw, nh)) = client_screen_rect(g) {
                if nw > 64 && nh > 64 && ((nx - x).abs() > 2 || (ny - y).abs() > 2 || (nw - w).abs() > 2 || (nh - h).abs() > 2)
                {
                    x = nx;
                    y = ny;
                    w = nw;
                    h = nh;
                    dib = Dib::new(w, h);
                    pixmap = Pixmap::new(w as u32, h as u32).unwrap();
                    placed = false;
                }
            }
        }

        if overlay_on && !placed {
            let _ = SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                x,
                y,
                w,
                h,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
            placed = true;
        }

        let layout_on = overlay_on && crate::layout::Editor::ctrl_down();
        if cmd.is_none() {
            cmd = Cmd::open();
        }
        let spectating = overlay_on && cmd.as_ref().is_some_and(|c| c.spectating());
        let hover_rider = if spectating && !layout_on {
            crate::layout::cursor_norm(x, y, w, h).and_then(|(nx, ny)| {
                mxbo_hud::click_rider_at(nx * w as f32, ny * h as f32)
            })
        } else {
            None
        };
        let hover_mark = overlay_on
            && live_mark.is_some_and(|(mx, my, mw, mh)| {
                crate::layout::cursor_norm(x, y, w, h).is_some_and(|(nx, ny)| {
                    let px = nx * w as f32;
                    let py = ny * h as f32;
                    px >= mx && px < mx + mw && py >= my && py < my + mh
                })
            });
        zfix.set_layout_mode(hwnd, layout_on || hover_rider.is_some() || hover_mark);
        if hover_mark {
            let cur = LoadCursorW(None, IDC_HAND).unwrap_or_default();
            let _ = SetCursor(cur);
        } else if layout_on {
            let cur = LoadCursorW(None, IDC_ARROW).unwrap_or_default();
            let _ = SetCursor(cur);
        } else if hover_rider.is_some() {
            let cur = LoadCursorW(None, IDC_HAND).unwrap_or_default();
            let _ = SetCursor(cur);
        }
        let lmb = unsafe { GetAsyncKeyState(VK_LBUTTON.0 as i32) < 0 };
        let spectate_click = lmb && !spectate_down;
        spectate_down = lmb;
        if spectate_click {
            if hover_mark {
                crate::settings::show(host);
            } else if !layout_on {
                if let (Some(num), Some(c)) = (hover_rider, cmd.as_ref()) {
                    c.request(num);
                }
            }
        }

        let settings_vk = crate::config::with_config(|c| c.settings_key.vk());
        let settings_down = unsafe { GetAsyncKeyState(settings_vk) < 0 };
        if settings_down && !f8_was {
            crate::settings::toggle(host);
        }
        f8_was = settings_down;
        let f9 = unsafe { GetAsyncKeyState(VK_F9.0 as i32) < 0 };
        let f9_hit = f9 && !f9_was;
        f9_was = f9;

        if shm.is_none() {
            shm = Shm::open();
        }
        if let Some(s) = shm.as_ref().and_then(|s| s.read()) {
            last_snap = Some(s);
        }
        if shm.is_some() && last_snap.is_some() {
            shm_miss_since = None;
        } else if overlay_on {
            shm_miss_since.get_or_insert_with(Instant::now);
        } else {
            shm_miss_since = None;
        }
        if f9_hit {
            crate::record::rotate();
            let text = f9_dump_text(shm.as_ref(), last_snap.as_ref());
            let dump_hint = write_shm_dump(&text);
            match (crate::record::path(), dump_hint) {
                (Some(p), Some(d)) => mxbo_hud::set_status_hint(format!(
                    "Clock log: {} | SHM dump: {}",
                    p.display(),
                    d.display()
                )),
                (Some(p), None) => mxbo_hud::set_status_hint(format!(
                    "Clock log: {} | dump write failed",
                    p.display()
                )),
                (None, Some(d)) => mxbo_hud::set_status_hint(format!("SHM dump: {}", d.display())),
                (None, None) => mxbo_hud::set_status_hint(
                    "Clock log failed — see AppData\\Local\\Holeshot HUD\\logs\\boot.txt",
                ),
            }
        }
        if crate::update::should_quit() || QUITTING.load(Ordering::SeqCst) {
            quit_app();
        }
        let commit_layout = crate::config::with_config(|cfg| {
            if cfg.font_family != font_family {
                if let Some(next) = Fonts::for_family(cfg.font_family) {
                    fonts = next;
                    font_family = cfg.font_family;
                }
            }
            if let Some(s) = last_snap.as_mut() {
                cfg.apply_to_snapshot(s);
            }
            editor.tick(hwnd, x, y, w, h, last_snap.as_ref(), cfg, hover_mark)
        });
        if commit_layout {
            editor.commit(last_snap.as_ref());
        }
        if let Some(s) = last_snap.as_mut() {
            editor.apply(s);
        }
        let preview_cfg = if editor.has_preview() {
            let mut cfg = crate::config::with_config(|c| c.clone());
            editor.apply_cfg(&mut cfg);
            Some(cfg)
        } else {
            None
        };
        let raw_age = last_snap
            .as_ref()
            .map(|s| qpc_age(s.tick_qpc, freq))
            .unwrap_or(999.0);
        let age = raw_age.clamp(0.0, 0.08);
        let live = raw_age < 2.5;
        let settings_open = crate::settings::is_open();
        crate::feedback::tick(settings_open);
        if live {
            if let Some(s) = last_snap.as_ref() {
                crate::record::tick(s);
            }
        }
        // Replay never sets plugin on_track. Telemetry / rider positions are the session.
        if last_snap.as_ref().is_some_and(|s| s.has_session_data()) {
            if let Some(s) = last_snap.as_mut() {
                s.on_track = 1;
            }
        }
        if let Some(s) = last_snap.as_mut() {
            let spectating = cmd.as_ref().is_some_and(|c| c.spectating());
            if spectating {
                // Replay still publishes leftover bike data. Drop it so the map follows
                // the camera subject, and so Delta / Sectors do not tape a replay.
                s.has_telemetry = 0;
            } else if s.has_telemetry != 0 || s.local_race_num > 0 {
                s.focus_race_num = s.local_race_num;
            }
        }
        let in_session = last_snap.as_ref().is_some_and(|s| s.has_session_data());
        // Plugin publish can pause for a few seconds during a hitch. Keep the last
        // session HUD instead of blanking at 2.5s; drop after 15s so garage/menus
        // still hide if SHM stops.
        let hitch_hold = in_session && raw_age < 15.0;
        if live || hitch_hold {
            if let Some(s) = last_snap.as_ref() {
                mxbo_hud::delta::tick(s);
                mxbo_hud::sector::tick(s);
            }
        }
        let hud = if overlay_on && (live || hitch_hold || layout_on || (settings_open && in_session))
        {
            last_snap.as_ref()
        } else {
            None
        };

        let frame_start = Instant::now();
        let (sys_show, stance_show, stance_bind, stance_mode) = match preview_cfg.as_ref() {
            Some(cfg) => (
                cfg[crate::config::WidgetId::Sys].show,
                cfg[crate::config::WidgetId::Stance].show,
                cfg.stance_bind,
                cfg.stance_mode,
            ),
            None => crate::config::with_config(|cfg| {
                (
                    cfg[crate::config::WidgetId::Sys].show,
                    cfg[crate::config::WidgetId::Stance].show,
                    cfg.stance_bind,
                    cfg.stance_mode,
                )
            }),
        };
        sys.tick(
            last_snap.as_ref().map(|s| s.seq),
            overlay_on && in_session && sys_show,
        );
        if let Some(bind) = stance.tick(
            stance_bind,
            stance_mode,
            crate::settings::listening_bind(),
            overlay_on && in_session && stance_show,
        ) {
            crate::settings::apply_stance_bind(bind);
        }
        let mut paint = |cfg: &crate::config::HudConfig| {
            render::draw(
                &mut pixmap,
                &fonts,
                hud,
                cfg,
                w as u32,
                h as u32,
                age,
                overlay_on && restart_hint,
                overlay_on
                    && (crate::plugin::needs_restart()
                        || shm_miss_since.is_some_and(|t| t.elapsed() >= Duration::from_secs(2))),
                layout_on,
            );
        };
        if let Some(cfg) = preview_cfg.as_ref() {
            paint(cfg);
        } else {
            let cfg = crate::config::with_config(|c| c.clone());
            paint(&cfg);
        }
        if overlay_on {
            live_mark = Some(render::draw_live_mark(&mut pixmap, w as u32, h as u32, hover_mark));
        } else {
            live_mark = None;
        }
        dib.blit_premul_bgra(pixmap.data());
        dib.present(hwnd, w, h, x, y);
        crate::settings::paint(&fonts);
        let target = Duration::from_millis(16);
        if let Some(remain) = target.checked_sub(frame_start.elapsed()) {
            Sleep(remain.as_millis() as u32);
        }
    }
}

fn qpc_age(tick: u64, freq: i64) -> f32 {
    if freq <= 0 || tick == 0 {
        return 0.0;
    }
    let mut now = 0i64;
    unsafe {
        QueryPerformanceCounter(&mut now).ok();
    }
    ((now as f64 - tick as f64) / freq as f64) as f32
}

unsafe fn load_app_icon(
    hinst: windows::Win32::Foundation::HMODULE,
    bytes: &[u8],
    size: i32,
) -> windows::Win32::UI::WindowsAndMessaging::HICON {
    if let Ok(handle) = LoadImageW(
        hinst,
        windows::core::PCWSTR(1usize as *const u16),
        IMAGE_ICON,
        size,
        size,
        LR_DEFAULTCOLOR,
    ) {
        if !handle.is_invalid() {
            return windows::Win32::UI::WindowsAndMessaging::HICON(handle.0);
        }
    }
    icon_from_ico(bytes, size)
}

unsafe fn apply_window_icons(
    hwnd: HWND,
    big: windows::Win32::UI::WindowsAndMessaging::HICON,
    small: windows::Win32::UI::WindowsAndMessaging::HICON,
) {
    let _ = SendMessageW(hwnd, WM_SETICON, WPARAM(ICON_BIG as usize), LPARAM(big.0 as isize));
    let _ = SendMessageW(
        hwnd,
        WM_SETICON,
        WPARAM(ICON_SMALL as usize),
        LPARAM(small.0 as isize),
    );
}

unsafe fn icon_from_ico(bytes: &[u8], size: i32) -> windows::Win32::UI::WindowsAndMessaging::HICON {
    let offset = LookupIconIdFromDirectoryEx(bytes.as_ptr(), true, size, size, LR_DEFAULTCOLOR);
    if offset <= 0 || offset as usize >= bytes.len() {
        return Default::default();
    }
    CreateIconFromResourceEx(
        &bytes[offset as usize..],
        true,
        0x0003_0000,
        size,
        size,
        LR_DEFAULTCOLOR,
    )
    .unwrap_or_default()
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if hwnd == host_hwnd() {
        if msg == crate::tray::callback_msg() {
            crate::tray::on_callback(lp);
            return LRESULT(0);
        }
        if msg != 0 && msg == crate::tray::taskbar_created_msg() {
            crate::tray::readd();
            return LRESULT(0);
        }
        if msg == WM_ACTIVATE && (wp.0 as u32 & 0xFFFF) != 0 {
            // Tray-hidden windows are not visible and not iconic. Don't pop Settings
            // just because Windows activated the host (Run key / MX Bikes waiter).
            if IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
                crate::settings::show(hwnd);
            }
        }
        if crate::settings::handle_message(msg, wp, lp) {
            return LRESULT(0);
        }
        if msg == WM_CLOSE {
            if crate::config::with_config(|c| c.minimize_on_close) {
                crate::settings::hide(hwnd);
                return LRESULT(0);
            }
            crate::config::update_config(|_| {});
            quit_app();
            return LRESULT(0);
        }
        if msg == WM_DESTROY {
            if !QUITTING.load(Ordering::SeqCst) {
                crate::config::update_config(|_| {});
                quit_app();
            }
            return LRESULT(0);
        }
    } else if msg == WM_SETCURSOR {
        let cur = LoadCursorW(None, IDC_ARROW).unwrap_or_default();
        let _ = SetCursor(cur);
        return LRESULT(1);
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}

unsafe fn create_overlay(
    hinst: windows::Win32::Foundation::HMODULE,
    class: windows::core::PCWSTR,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) -> HWND {
    let ex = WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW;
    if let Some(hwnd) = compat::try_create_window_in_band(
        ex.0,
        class,
        w!("Holeshot HUD"),
        WS_POPUP.0,
        x,
        y,
        w,
        h,
        hinst.into(),
    ) {
        return hwnd;
    }
    CreateWindowExW(
        ex,
        class,
        w!("Holeshot HUD"),
        WS_POPUP,
        x,
        y,
        w,
        h,
        HWND::default(),
        None,
        hinst,
        None,
    )
    .expect("overlay window")
}

fn find_game_hwnd() -> Option<HWND> {
    let pid = crate::startup::mx_bikes_pid()?;
    struct St {
        pid: u32,
        best: HWND,
        area: i32,
    }
    unsafe extern "system" fn cb(hwnd: HWND, lp: LPARAM) -> BOOL {
        use windows::Win32::UI::WindowsAndMessaging::{GetWindowRect, IsWindowVisible};
        let st = unsafe { &mut *(lp.0 as *mut St) };
        if !IsWindowVisible(hwnd).as_bool() {
            return true.into();
        }
        let mut wpid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut wpid));
        if wpid != st.pid {
            return true.into();
        }
        let mut name = [0u16; 64];
        let n = GetClassNameW(hwnd, &mut name);
        if n > 0 {
            let class = String::from_utf16_lossy(&name[..n as usize]);
            if class == "MXBOOverlay" {
                return true.into();
            }
        }
        let mut r = RECT::default();
        let _ = GetWindowRect(hwnd, &mut r);
        let area = (r.right - r.left).saturating_mul(r.bottom - r.top);
        if area > st.area && (r.right - r.left) > 64 && (r.bottom - r.top) > 64 {
            st.area = area;
            st.best = hwnd;
        }
        true.into()
    }
    let mut st = St {
        pid,
        best: HWND::default(),
        area: 0,
    };
    unsafe {
        let _ = windows::Win32::UI::WindowsAndMessaging::EnumWindows(
            Some(cb),
            LPARAM(&mut st as *mut St as isize),
        );
    }
    if st.best.0.is_null() {
        None
    } else {
        Some(st.best)
    }
}

fn client_screen_rect(hwnd: HWND) -> Option<(i32, i32, i32, i32)> {
    unsafe {
        let mut cr = RECT::default();
        GetClientRect(hwnd, &mut cr).ok()?;
        let mut pt = POINT { x: 0, y: 0 };
        let _ = ClientToScreen(hwnd, &mut pt);
        let w = cr.right - cr.left;
        let h = cr.bottom - cr.top;
        if w < 64 || h < 64 {
            return None;
        }
        Some((pt.x, pt.y, w, h))
    }
}

fn primary_screen() -> (i32, i32, i32, i32) {
    unsafe {
        (
            0,
            0,
            GetSystemMetrics(SM_CXSCREEN).max(800),
            GetSystemMetrics(SM_CYSCREEN).max(600),
        )
    }
}

struct Dib {
    hdc: HDC,
    bmp: HBITMAP,
    bits: *mut u8,
    w: i32,
    h: i32,
}

impl Dib {
    fn new(w: i32, h: i32) -> Self {
        unsafe {
            let hdc = CreateCompatibleDC(None);
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w,
                    biHeight: -h,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0 as u32,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits = null_mut();
            let bmp = CreateDIBSection(hdc, &info, DIB_RGB_COLORS, &mut bits, None, 0).unwrap();
            SelectObject(hdc, HGDIOBJ(bmp.0));
            Self {
                hdc,
                bmp,
                bits: bits as *mut u8,
                w,
                h,
            }
        }
    }

    fn blit_premul_bgra(&self, rgba: &[u8]) {
        let n = (self.w * self.h * 4) as usize;
        unsafe {
            let dst = std::slice::from_raw_parts_mut(self.bits, n);
            let mut i = 0;
            while i + 16 <= n {
                dst[i] = rgba[i + 2];
                dst[i + 1] = rgba[i + 1];
                dst[i + 2] = rgba[i];
                dst[i + 3] = rgba[i + 3];
                dst[i + 4] = rgba[i + 6];
                dst[i + 5] = rgba[i + 5];
                dst[i + 6] = rgba[i + 4];
                dst[i + 7] = rgba[i + 7];
                dst[i + 8] = rgba[i + 10];
                dst[i + 9] = rgba[i + 9];
                dst[i + 10] = rgba[i + 8];
                dst[i + 11] = rgba[i + 11];
                dst[i + 12] = rgba[i + 14];
                dst[i + 13] = rgba[i + 13];
                dst[i + 14] = rgba[i + 12];
                dst[i + 15] = rgba[i + 15];
                i += 16;
            }
            while i + 4 <= n {
                dst[i] = rgba[i + 2];
                dst[i + 1] = rgba[i + 1];
                dst[i + 2] = rgba[i];
                dst[i + 3] = rgba[i + 3];
                i += 4;
            }
        }
    }

    fn present(&self, hwnd: HWND, w: i32, h: i32, x: i32, y: i32) {
        unsafe {
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            let src = POINT { x: 0, y: 0 };
            let dst = POINT { x, y };
            let size = windows::Win32::Foundation::SIZE { cx: w, cy: h };
            let _ = UpdateLayeredWindow(
                hwnd,
                None,
                Some(&dst),
                Some(&size),
                self.hdc,
                Some(&src),
                windows::Win32::Foundation::COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );
        }
    }
}

impl Drop for Dib {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(HGDIOBJ(self.bmp.0));
            let _ = DeleteDC(self.hdc);
        }
    }
}
