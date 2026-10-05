//! Profile → CBR. A WebView2 child for the local Steam rider page.

use std::cell::RefCell;
use std::num::NonZeroIsize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use tiny_skia::{Color, PathBuilder, Pixmap, Rect};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::WindowsAndMessaging::IsWindow;
use wry::raw_window_handle::{
    HandleError, HasWindowHandle, RawWindowHandle, Win32WindowHandle, WindowHandle,
};
use wry::{
    NewWindowResponse, PageLoadEvent, Rect as ViewRect, WebContext, WebView, WebViewBuilder,
};

use crate::render::{fill_rect, measure, text, Fonts};

use super::{bg, muted, PROFILE_SUBNAV_H};

const NO_STEAM: &str = "Steam is not signed in on this PC.";
const OPEN_FAILED: &str = "Couldn't open the CBR page.";
const LOADING: &str = "Loading";
const LOAD_WAIT: Duration = Duration::from_secs(6);

static PAGE_READY: AtomicBool = AtomicBool::new(false);
static MANUAL_REFRESH: AtomicBool = AtomicBool::new(false);
static LINK_BLOCKED: AtomicBool = AtomicBool::new(false);
static RETURN_RIDER: AtomicBool = AtomicBool::new(false);
static POPUP_URL: Mutex<Option<String>> = Mutex::new(None);
static BLOCKED_URL: Mutex<Option<String>> = Mutex::new(None);

pub(crate) fn request_refresh() {
    MANUAL_REFRESH.store(true, Ordering::Relaxed);
}

/// Tell the overlay the player page has something to show.
const READY_SCRIPT: &str = r#"
(() => {
  const markReady = () => {
    if (window.__cbrReady || !document.body) return;
    const text = document.body.innerText.replace(/\s+/g, " ").trim();
    if (text.length < 24) return;
    window.__cbrReady = true;
    if (window.ipc) window.ipc.postMessage("ready");
  };
  const arm = () => {
    markReady();
    if (window.__cbrWatch) return;
    window.__cbrWatch = new MutationObserver(markReady);
    window.__cbrWatch.observe(document.documentElement, {
      childList: true,
      subtree: true,
    });
  };
  if (document.body) arm();
  else document.addEventListener("DOMContentLoaded", arm);
})();
"#;

/// Notice shown over the page when a link is outside CBR and Steam.
const BLOCKED_NOTICE_SCRIPT: &str = r##"
(() => {
  if (window.top !== window) return;
  let blockedUrl = "";
  window.__cbrShowBlocked = (url) => {
    if (typeof url === "string" && url) blockedUrl = url;
    let panel = document.getElementById("cbr-blocked");
    if (!panel) {
      panel = document.createElement("div");
      panel.id = "cbr-blocked";
      panel.innerHTML = '<div id="cbr-blocked-card"><p>That link can\'t be opened in this app.</p><button type="button" data-open>Open in browser</button><button type="button" data-close>Close</button></div>';
      const style = document.createElement("style");
      style.textContent = "#cbr-blocked{position:fixed;inset:0;z-index:2147483647;display:none;align-items:center;justify-content:center;background:rgba(0,0,0,.55)}#cbr-blocked-card{width:280px;padding:20px;background:#18191d;color:#f4f4f7;font:14px sans-serif;text-align:center}#cbr-blocked-card p{margin:0 0 16px}#cbr-blocked-card button{display:block;width:100%;margin-top:8px;padding:8px 12px;border:0;background:#2a2c31;color:#f4f4f7;cursor:pointer}#cbr-blocked-card button[data-open]{background:#f4f4f7;color:#18191d}";
      document.documentElement.appendChild(style);
      document.documentElement.appendChild(panel);
      panel.querySelector("[data-open]").addEventListener("click", () => {
        if (window.ipc && blockedUrl) window.ipc.postMessage("open-browser " + blockedUrl);
        panel.style.display = "none";
      });
      panel.querySelector("[data-close]").addEventListener("click", () => {
        panel.style.display = "none";
      });
    }
    panel.style.display = "flex";
  };
  const roots = ["cbrservers.com", "steamcommunity.com", "steampowered.com"];
  const hostOk = (host) => {
    const name = String(host || "").replace(/\.$/, "").toLowerCase();
    return roots.some((root) => name === root || name.endsWith("." + root));
  };
  const httpUrl = (raw) => {
    try {
      const url = new URL(raw, location.href);
      if (url.protocol !== "http:" && url.protocol !== "https:") return null;
      return url;
    } catch (err) {
      return null;
    }
  };
  document.addEventListener("click", (event) => {
    const link = event.target && event.target.closest && event.target.closest("a[href]");
    if (!link) return;
    const url = httpUrl(link.href);
    if (!url || hostOk(url.hostname)) return;
    event.preventDefault();
    event.stopPropagation();
    window.__cbrShowBlocked(url.href);
  }, true);
  document.addEventListener("submit", (event) => {
    const form = event.target;
    if (!form || !form.action) return;
    const url = httpUrl(form.action);
    if (!url || hostOk(url.hostname)) return;
    event.preventDefault();
    event.stopPropagation();
    window.__cbrShowBlocked(url.href);
  }, true);
})();
"##;

struct SettingsHost(HWND);

impl HasWindowHandle for SettingsHost {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let raw = self.0 .0 as isize;
        let hwnd = NonZeroIsize::new(raw).ok_or(HandleError::Unavailable)?;
        let handle = Win32WindowHandle::new(hwnd);
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::Win32(handle)) })
    }
}

struct Page {
    view: WebView,
    /// WebView2 environment the view uses. Dropped after `view`.
    #[allow(dead_code)]
    context: WebContext,
    url: String,
    top: i32,
    width: u32,
    height: u32,
    visible: bool,
    started: Instant,
}

struct UrlCache {
    at: Instant,
    url: Option<String>,
}

thread_local! {
    static PAGE: RefCell<Option<Page>> = const { RefCell::new(None) };
    static FAILED: RefCell<bool> = const { RefCell::new(false) };
    static URLS: RefCell<Option<UrlCache>> = const { RefCell::new(None) };
}

pub(crate) fn rider_url() -> Option<String> {
    URLS.with(|cell| {
        let mut slot = cell.borrow_mut();
        if let Some(cache) = slot.as_ref() {
            if cache.at.elapsed() < Duration::from_secs(2) {
                return cache.url.clone();
            }
        }
        let url = crate::plugin::local_cbr_url();
        *slot = Some(UrlCache {
            at: Instant::now(),
            url: url.clone(),
        });
        url
    })
}

pub(crate) fn paint_pane(px: &mut Pixmap, fonts: &Fonts, w: f32, h: f32, clip_top: f32) -> f32 {
    let top = clip_top + PROFILE_SUBNAV_H;
    if let Some(rect) = Rect::from_xywh(0.0, top, w, (h - top).max(0.0)) {
        fill_rect(px, rect, bg());
    }
    let line = if FAILED.with(|failed| *failed.borrow()) {
        Some(OPEN_FAILED)
    } else if rider_url().is_none() {
        Some(NO_STEAM)
    } else if !page_is_ready() {
        Some(LOADING)
    } else {
        None
    };
    if let Some(line) = line {
        let size = 13.0;
        let lw = measure(fonts, line, size);
        let x = ((w - lw) * 0.5).max(0.0);
        if line == LOADING {
            let spinner = 16.0;
            let gap = 8.0;
            let stack = spinner + gap + size;
            let stack_top = top + (((h - top) - stack) * 0.5).max(0.0);
            stroke_spinner(px, w * 0.5, stack_top + spinner * 0.5, super::accent());
            text(px, fonts, line, size, x, stack_top + spinner + gap, muted(), false);
        } else {
            let y = top + (((h - top) - size) * 0.5).max(0.0);
            text(px, fonts, line, size, x, y, muted(), false);
        }
    }
    top
}

/// Drop the CBR profile that was nested inside the MXB-Ranked WebView2 folder.
pub(crate) fn retire_nested_profile() {
    static DONE: OnceLock<()> = OnceLock::new();
    DONE.get_or_init(|| {
        let nested = app_data_dir().join("WebView2").join("cbr");
        let _ = std::fs::remove_dir_all(nested);
    });
}

/// Show the rider page only while Profile → CBR is the visible settings pane.
pub(crate) fn sync(host: HWND, show: bool, top: i32, width: u32, height: u32) {
    let show = show && super::is_open();
    if !show || width == 0 || height == 0 {
        hide_page();
        return;
    }
    let Some(url) = rider_url() else {
        hide_page();
        return;
    };
    if unsafe { !IsWindow(host).as_bool() } {
        return;
    }
    if FAILED.with(|failed| *failed.borrow()) {
        return;
    }
    show_page(host, &url, top, width, height);
}

fn hide_page() {
    PAGE.with(|cell| {
        let mut slot = cell.borrow_mut();
        let Some(page) = slot.as_mut() else {
            return;
        };
        if page.visible || page.width != 0 || page.height != 0 {
            let _ = page.view.set_visible(false);
            let _ = page.view.set_bounds(view_bounds(0, 0, 0));
            page.visible = false;
            page.width = 0;
            page.height = 0;
        }
    });
}

fn show_page(host: HWND, url: &str, top: i32, width: u32, height: u32) {
    PAGE.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.is_none() {
            match create_page(host, url, top, width, height) {
                Ok(page) => *slot = Some(page),
                Err(_) => {
                    FAILED.with(|failed| *failed.borrow_mut() = true);
                    return;
                }
            }
        }
        let Some(page) = slot.as_mut() else {
            return;
        };
        if RETURN_RIDER.swap(false, Ordering::Relaxed) {
            if page.view.load_url(url).is_ok() {
                page.url = url.to_string();
                restart_load(page);
            } else {
                RETURN_RIDER.store(true, Ordering::Relaxed);
            }
        } else if let Some(next) = take_popup() {
            if next != page.url && page.view.load_url(&next).is_ok() {
                page.url = next;
                restart_load(page);
            }
        } else if page.url != url {
            if page.view.load_url(url).is_ok() {
                page.url = url.to_string();
                restart_load(page);
            }
        } else if MANUAL_REFRESH.swap(false, Ordering::Relaxed) && page.view.reload().is_ok() {
            restart_load(page);
        }
        if page.top != top || page.width != width || page.height != height {
            if page
                .view
                .set_bounds(view_bounds(top, width, height))
                .is_ok()
            {
                page.top = top;
                page.width = width;
                page.height = height;
            }
        }
        let ready = PAGE_READY.load(Ordering::Relaxed) || page.started.elapsed() >= LOAD_WAIT;
        if ready && !page.visible {
            if page.view.set_visible(true).is_ok() {
                page.visible = true;
            }
        } else if !ready && page.visible {
            if page.view.set_visible(false).is_ok() {
                page.visible = false;
            }
        }
        if page.visible && LINK_BLOCKED.swap(false, Ordering::Relaxed) {
            let script = take_blocked_url().map_or_else(
                || "window.__cbrShowBlocked&&window.__cbrShowBlocked()".to_string(),
                |url| show_blocked_script(&url),
            );
            let _ = page.view.evaluate_script(&script);
        }
    });
}

fn create_page(host: HWND, url: &str, top: i32, width: u32, height: u32) -> Result<Page, ()> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }
    let mut context = WebContext::new(Some(data_dir()));
    let window = SettingsHost(host);
    let color = page_color();
    let view = WebViewBuilder::new_with_web_context(&mut context)
        .with_url(url)
        .with_bounds(view_bounds(top, width, height))
        .with_background_color(color)
        .with_focused(false)
        .with_visible(false)
        .with_initialization_script(format!("{READY_SCRIPT}{BLOCKED_NOTICE_SCRIPT}"))
        .with_on_page_load_handler(|event, url| {
            if matches!(event, PageLoadEvent::Finished)
                && is_http(&url)
                && !url_is_allowed(&url)
            {
                RETURN_RIDER.store(true, Ordering::Relaxed);
                remember_blocked(&url);
            }
        })
        .with_new_window_req_handler(|url, _features| {
            if is_http(&url) && url_is_allowed(&url) {
                if let Ok(mut slot) = POPUP_URL.lock() {
                    *slot = Some(url);
                }
            } else if is_http(&url) {
                remember_blocked(&url);
            }
            NewWindowResponse::Deny
        })
        .with_ipc_handler(|request| {
            let body = request.body();
            if body == "ready" {
                PAGE_READY.store(true, Ordering::Relaxed);
            } else if let Some(url) = body.strip_prefix("open-browser ") {
                if is_http(url) {
                    super::dispatch::open_default_browser(url);
                }
            }
        })
        .build_as_child(&window)
        .map_err(|_| ())?;
    Ok(Page {
        view,
        context,
        url: url.to_string(),
        top,
        width,
        height,
        visible: false,
        started: Instant::now(),
    })
}

fn remember_blocked(url: &str) {
    if let Ok(mut slot) = BLOCKED_URL.lock() {
        *slot = Some(url.to_string());
    }
    LINK_BLOCKED.store(true, Ordering::Relaxed);
}

fn take_blocked_url() -> Option<String> {
    BLOCKED_URL.lock().ok().and_then(|mut slot| slot.take())
}

fn show_blocked_script(url: &str) -> String {
    let escaped = url
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace(['\n', '\r'], "");
    format!("window.__cbrShowBlocked&&window.__cbrShowBlocked('{escaped}')")
}

fn take_popup() -> Option<String> {
    POPUP_URL.lock().ok().and_then(|mut slot| slot.take())
}

fn is_http(raw: &str) -> bool {
    let raw = raw.trim();
    let Some((scheme, _)) = raw.split_once("://") else {
        return false;
    };
    scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https")
}

fn url_is_allowed(raw: &str) -> bool {
    let raw = raw.trim();
    if raw.eq_ignore_ascii_case("about:blank") {
        return true;
    }
    let Some((scheme, rest)) = raw.split_once("://") else {
        return false;
    };
    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return false;
    }
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = host.rsplit('@').next().unwrap_or(host);
    let host = host.split(':').next().unwrap_or(host);
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    host_is(&host, "cbrservers.com")
        || host_is(&host, "steamcommunity.com")
        || host_is(&host, "steampowered.com")
}

fn host_is(host: &str, root: &str) -> bool {
    host == root || host.ends_with(&format!(".{root}"))
}

fn stroke_spinner(px: &mut Pixmap, cx: f32, cy: f32, color: Color) {
    let radius = 8.0;
    let sweep = std::f32::consts::PI * 1.5;
    let start = spinner_angle();
    let steps = 24;
    let mut path = PathBuilder::new();
    for step in 0..=steps {
        let angle = start + sweep * (step as f32 / steps as f32);
        let x = cx + radius * angle.cos();
        let y = cy + radius * angle.sin();
        if step == 0 {
            path.move_to(x, y);
        } else {
            path.line_to(x, y);
        }
    }
    if let Some(path) = path.finish() {
        super::paint::icon_stroke(px, &path, color, 2.0);
    }
}

fn spinner_angle() -> f32 {
    static START: OnceLock<Instant> = OnceLock::new();
    let start = START.get_or_init(Instant::now);
    start.elapsed().as_secs_f32() * std::f32::consts::TAU / 0.9
}

fn restart_load(page: &mut Page) {
    page.started = Instant::now();
    PAGE_READY.store(false, Ordering::Relaxed);
    if page.visible {
        let _ = page.view.set_visible(false);
        page.visible = false;
    }
}

fn view_bounds(top: i32, width: u32, height: u32) -> ViewRect {
    ViewRect {
        position: wry::dpi::Position::Physical(wry::dpi::PhysicalPosition::new(0, top)),
        size: wry::dpi::Size::Physical(wry::dpi::PhysicalSize::new(width, height)),
    }
}

fn page_is_ready() -> bool {
    if PAGE_READY.load(Ordering::Relaxed) {
        return true;
    }
    PAGE.with(|cell| {
        cell.borrow()
            .as_ref()
            .is_some_and(|page| page.started.elapsed() >= LOAD_WAIT)
    })
}

fn page_color() -> wry::RGBA {
    let color = bg();
    (
        (color.red() * 255.0).round() as u8,
        (color.green() * 255.0).round() as u8,
        (color.blue() * 255.0).round() as u8,
        255,
    )
}

fn app_data_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Holeshot HUD")
}

fn data_dir() -> PathBuf {
    let dir = app_data_dir().join("WebView2Cbr");
    let _ = std::fs::create_dir_all(&dir);
    dir
}
