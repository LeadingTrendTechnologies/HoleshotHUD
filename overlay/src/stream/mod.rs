//! Localhost Browser Source feed for OBS + `/edit` stream layout editor.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use tiny_skia::Pixmap;

use crate::config::{self, HudConfig, SessionPreset, WidgetId};

const DEFAULT_PORT: u16 = 8765;
const PORT_LAST: u16 = 8775;
const STREAM_W: u32 = 1920;
const STREAM_H: u32 = 1080;
const MIN_FRAME_GAP: Duration = Duration::from_millis(33);

static ENABLED: AtomicBool = AtomicBool::new(false);
static BOUND_PORT: AtomicU16 = AtomicU16::new(0);
static CLIENTS: AtomicUsize = AtomicUsize::new(0);
static EDIT_CLIENTS: AtomicUsize = AtomicUsize::new(0);
static EDIT_PRESET: AtomicU8 = AtomicU8::new(0);
static STOP: AtomicBool = AtomicBool::new(false);
static SERVER: OnceLock<Mutex<Option<ServerState>>> = OnceLock::new();

struct ServerState {
    join: Option<JoinHandle<()>>,
    clients: Arc<Mutex<Vec<ClientOut>>>,
    edit_clients: Arc<Mutex<Vec<ClientOut>>>,
    last_png: Arc<Mutex<Option<Vec<u8>>>>,
    last_edit_png: Arc<Mutex<Option<Vec<u8>>>>,
}

struct ClientOut {
    stream: TcpStream,
}

pub fn bound_port() -> u16 {
    BOUND_PORT.load(Ordering::SeqCst)
}

pub fn is_running() -> bool {
    ENABLED.load(Ordering::SeqCst) && bound_port() != 0
}

pub fn client_count() -> usize {
    CLIENTS.load(Ordering::SeqCst) + EDIT_CLIENTS.load(Ordering::SeqCst)
}

pub fn url() -> String {
    let port = bound_port();
    let p = if port == 0 { DEFAULT_PORT } else { port };
    format!("http://127.0.0.1:{p}/")
}

pub fn edit_url() -> String {
    let port = bound_port();
    let p = if port == 0 { DEFAULT_PORT } else { port };
    format!("http://127.0.0.1:{p}/edit")
}

pub fn status_line() -> String {
    let port = bound_port();
    if !ENABLED.load(Ordering::SeqCst) {
        return "Off".into();
    }
    if port == 0 {
        return "Starting…".into();
    }
    if port != DEFAULT_PORT {
        return format!("Listening on port {port} (default was busy)");
    }
    format!("Listening on port {port}")
}

pub fn set_enabled(on: bool) -> u16 {
    if on {
        start()
    } else {
        stop();
        0
    }
}

fn edit_preset() -> SessionPreset {
    SessionPreset::ALL[EDIT_PRESET.load(Ordering::SeqCst) as usize % SessionPreset::COUNT]
}

fn set_edit_preset(p: SessionPreset) {
    EDIT_PRESET.store(p.idx() as u8, Ordering::SeqCst);
}

fn server_slot() -> &'static Mutex<Option<ServerState>> {
    SERVER.get_or_init(|| Mutex::new(None))
}

fn start() -> u16 {
    ENABLED.store(true, Ordering::SeqCst);
    let mut slot = server_slot().lock().unwrap_or_else(|e| e.into_inner());
    if let Some(state) = slot.as_ref() {
        if state.join.as_ref().is_some_and(|j| !j.is_finished()) {
            return bound_port();
        }
    }
    STOP.store(false, Ordering::SeqCst);
    let clients: Arc<Mutex<Vec<ClientOut>>> = Arc::new(Mutex::new(Vec::new()));
    let edit_clients: Arc<Mutex<Vec<ClientOut>>> = Arc::new(Mutex::new(Vec::new()));
    let last_png: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));
    let last_edit_png: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));
    let clients_thread = Arc::clone(&clients);
    let edit_clients_thread = Arc::clone(&edit_clients);
    let last_png_thread = Arc::clone(&last_png);
    let last_edit_png_thread = Arc::clone(&last_edit_png);
    let join = thread::Builder::new()
        .name("mxbo-stream".into())
        .spawn(move || {
            run_server(
                clients_thread,
                edit_clients_thread,
                last_png_thread,
                last_edit_png_thread,
            )
        })
        .ok();
    *slot = Some(ServerState {
        join,
        clients,
        edit_clients,
        last_png,
        last_edit_png,
    });
    for _ in 0..50 {
        let port = bound_port();
        if port != 0 {
            return port;
        }
        thread::sleep(Duration::from_millis(10));
    }
    bound_port()
}

fn stop() {
    ENABLED.store(false, Ordering::SeqCst);
    STOP.store(true, Ordering::SeqCst);
    BOUND_PORT.store(0, Ordering::SeqCst);
    CLIENTS.store(0, Ordering::SeqCst);
    EDIT_CLIENTS.store(0, Ordering::SeqCst);
    let mut slot = server_slot().lock().unwrap_or_else(|e| e.into_inner());
    if let Some(state) = slot.take() {
        for port in DEFAULT_PORT..=PORT_LAST {
            if let Ok(mut s) = TcpStream::connect(("127.0.0.1", port)) {
                let _ = s.write_all(b"GET / HTTP/1.0\r\n\r\n");
                break;
            }
        }
        if let Some(join) = state.join {
            let _ = join.join();
        }
    }
}

/// Paint transparent HUD frames for OBS (`/ws`) and the layout editor (`/ws/edit`).
pub fn publish_frame(
    fonts: &crate::render::Fonts,
    snap: Option<&crate::shm::Snapshot>,
    live_cfg: &HudConfig,
    age: f32,
) {
    if !is_running() || client_count() == 0 {
        return;
    }
    static LAST: OnceLock<Mutex<Instant>> = OnceLock::new();
    let last = LAST.get_or_init(|| Mutex::new(Instant::now() - MIN_FRAME_GAP));
    {
        let mut guard = last.lock().unwrap_or_else(|e| e.into_inner());
        if guard.elapsed() < MIN_FRAME_GAP {
            return;
        }
        *guard = Instant::now();
    }

    let slot = server_slot().lock().unwrap_or_else(|e| e.into_inner());
    let Some(state) = slot.as_ref() else {
        return;
    };

    let obs_n = state.clients.lock().map(|c| c.len()).unwrap_or(0);
    let edit_n = state.edit_clients.lock().map(|c| c.len()).unwrap_or(0);

    if snap.is_none() {
        if obs_n > 0 {
            if let Ok(guard) = state.last_png.lock() {
                if let Some(png) = guard.as_ref() {
                    broadcast_png(&state.clients, png, &CLIENTS);
                }
            }
        }
        if edit_n > 0 {
            if let Ok(guard) = state.last_edit_png.lock() {
                if let Some(png) = guard.as_ref() {
                    broadcast_png(&state.edit_clients, png, &EDIT_CLIENTS);
                }
            }
        }
        return;
    }

    if obs_n > 0 {
        if let Some(png) = paint_png(fonts, snap, live_cfg, age) {
            if let Ok(mut last) = state.last_png.lock() {
                *last = Some(png.clone());
            }
            broadcast_png(&state.clients, &png, &CLIENTS);
        }
    }

    if edit_n > 0 {
        let preset = edit_preset();
        let edit_cfg = config::with_config(|c| c.for_stream_preset(preset));
        let mut edit_snap = snap.copied();
        if let Some(s) = edit_snap.as_mut() {
            edit_cfg.apply_stream_live_to_snapshot(s);
        }
        if let Some(png) = paint_png(fonts, edit_snap.as_ref(), &edit_cfg, age) {
            if let Ok(mut last) = state.last_edit_png.lock() {
                *last = Some(png.clone());
            }
            broadcast_png(&state.edit_clients, &png, &EDIT_CLIENTS);
        }
    }
}

fn paint_png(
    fonts: &crate::render::Fonts,
    snap: Option<&crate::shm::Snapshot>,
    cfg: &HudConfig,
    age: f32,
) -> Option<Vec<u8>> {
    let mut px = Pixmap::new(STREAM_W, STREAM_H)?;
    crate::render::draw(
        &mut px,
        fonts,
        snap,
        cfg,
        STREAM_W,
        STREAM_H,
        age,
        false,
        false,
        false,
    );
    px.encode_png().ok()
}

fn broadcast_png(
    clients: &Arc<Mutex<Vec<ClientOut>>>,
    png: &[u8],
    counter: &AtomicUsize,
) {
    let Ok(mut list) = clients.lock() else {
        return;
    };
    let frame = ws_binary_frame(png);
    list.retain_mut(|client| client.stream.write_all(&frame).is_ok());
    counter.store(list.len(), Ordering::SeqCst);
}

fn run_server(
    clients: Arc<Mutex<Vec<ClientOut>>>,
    edit_clients: Arc<Mutex<Vec<ClientOut>>>,
    last_png: Arc<Mutex<Option<Vec<u8>>>>,
    last_edit_png: Arc<Mutex<Option<Vec<u8>>>>,
) {
    let mut listener = None;
    for port in DEFAULT_PORT..=PORT_LAST {
        match TcpListener::bind(("127.0.0.1", port)) {
            Ok(l) => {
                let _ = l.set_nonblocking(true);
                BOUND_PORT.store(port, Ordering::SeqCst);
                listener = Some(l);
                break;
            }
            Err(_) => continue,
        }
    }
    let Some(listener) = listener else {
        ENABLED.store(false, Ordering::SeqCst);
        BOUND_PORT.store(0, Ordering::SeqCst);
        return;
    };
    while !STOP.load(Ordering::SeqCst) && ENABLED.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => {
                let clients = Arc::clone(&clients);
                let edit_clients = Arc::clone(&edit_clients);
                let last_png = Arc::clone(&last_png);
                let last_edit_png = Arc::clone(&last_edit_png);
                thread::spawn(move || {
                    handle_conn(stream, clients, edit_clients, last_png, last_edit_png)
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(15));
            }
            Err(_) => thread::sleep(Duration::from_millis(50)),
        }
    }
    if let Ok(mut list) = clients.lock() {
        list.clear();
    }
    if let Ok(mut list) = edit_clients.lock() {
        list.clear();
    }
    CLIENTS.store(0, Ordering::SeqCst);
    EDIT_CLIENTS.store(0, Ordering::SeqCst);
    BOUND_PORT.store(0, Ordering::SeqCst);
}

fn handle_conn(
    mut stream: TcpStream,
    clients: Arc<Mutex<Vec<ClientOut>>>,
    edit_clients: Arc<Mutex<Vec<ClientOut>>>,
    last_png: Arc<Mutex<Option<Vec<u8>>>>,
    last_edit_png: Arc<Mutex<Option<Vec<u8>>>>,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
    let mut buf = vec![0u8; 16384];
    let Ok(n) = stream.read(&mut buf) else {
        return;
    };
    if n == 0 {
        return;
    }
    let header_bytes = &buf[..n];
    let req_head = String::from_utf8_lossy(header_bytes);
    let line = req_head.lines().next().unwrap_or("");
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("/");
    let path_only = path.split('?').next().unwrap_or(path);

    if method == "POST" && path_only == "/api/stream-layout" {
        let body = read_http_body(&req_head, header_bytes, &mut stream);
        let json = apply_stream_layout_post(&body);
        write_json(&mut stream, &json);
        return;
    }

    if method != "GET" && method != "HEAD" {
        let _ = stream.write_all(b"HTTP/1.1 405 Method Not Allowed\r\nConnection: close\r\n\r\n");
        return;
    }

    if path_only == "/ws" || path_only == "/ws/edit" {
        let edit = path_only == "/ws/edit";
        if let Some(key) = sec_websocket_key(&req_head) {
            if upgrade_websocket(&mut stream, &key).is_ok() {
                let list = if edit { &edit_clients } else { &clients };
                let counter = if edit { &EDIT_CLIENTS } else { &CLIENTS };
                let last = if edit { &last_edit_png } else { &last_png };
                if let Ok(mut guard) = list.lock() {
                    if let Ok(clone) = stream.try_clone() {
                        guard.push(ClientOut { stream: clone });
                        counter.store(guard.len(), Ordering::SeqCst);
                    }
                }
                if let Ok(guard) = last.lock() {
                    if let Some(png) = guard.as_ref() {
                        let _ = stream.write_all(&ws_binary_frame(png));
                    }
                }
                let _ = stream.set_read_timeout(Some(Duration::from_secs(3600)));
                let mut sink = [0u8; 256];
                loop {
                    match stream.read(&mut sink) {
                        Ok(0) => break,
                        Ok(_) => continue,
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(200));
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::TimedOut => continue,
                        Err(_) => break,
                    }
                    if STOP.load(Ordering::SeqCst) {
                        break;
                    }
                }
                if let Ok(mut guard) = list.lock() {
                    list_retain_peer(&mut guard, &stream);
                    counter.store(guard.len(), Ordering::SeqCst);
                }
            }
        }
        return;
    }

    if path_only == "/api/stream-layout" {
        write_json(&mut stream, &stream_layout_json());
        return;
    }

    let (body, ctype) = static_asset(path_only);
    let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    if method != "HEAD" {
        let _ = stream.write_all(&body);
    }
}

fn list_retain_peer(list: &mut Vec<ClientOut>, stream: &TcpStream) {
    list.retain(|c| {
        c.stream
            .peer_addr()
            .ok()
            .zip(stream.peer_addr().ok())
            .map(|(a, b)| a != b)
            .unwrap_or(true)
    });
}

fn read_http_body(req_head: &str, first: &[u8], stream: &mut TcpStream) -> String {
    let content_len = req_head
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.eq_ignore_ascii_case("Content-Length") {
                value.trim().parse::<usize>().ok()
            } else {
                None
            }
        })
        .unwrap_or(0);
    let header_end = req_head
        .find("\r\n\r\n")
        .map(|i| i + 4)
        .or_else(|| req_head.find("\n\n").map(|i| i + 2))
        .unwrap_or(first.len());
    let mut body = Vec::new();
    if header_end < first.len() {
        body.extend_from_slice(&first[header_end..]);
    }
    while body.len() < content_len {
        let mut chunk = [0u8; 4096];
        let Ok(n) = stream.read(&mut chunk) else {
            break;
        };
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
    }
    if body.len() > content_len {
        body.truncate(content_len);
    }
    String::from_utf8_lossy(&body).into_owned()
}

fn write_json(stream: &mut TcpStream, body: &str) {
    let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body.as_bytes());
}

fn static_asset(path: &str) -> (Vec<u8>, &'static str) {
    match path {
        "/" | "/index.html" | "/stream.html" => {
            (STREAM_HTML.as_bytes().to_vec(), "text/html; charset=utf-8")
        }
        "/edit" | "/edit.html" => (
            include_str!("edit.html").as_bytes().to_vec(),
            "text/html; charset=utf-8",
        ),
        _ => (
            b"<!doctype html><title>Holeshot Stream</title><p>Not found</p>".to_vec(),
            "text/html; charset=utf-8",
        ),
    }
}

fn widget_key(id: WidgetId) -> &'static str {
    match id {
        WidgetId::Standings => "standings",
        WidgetId::Relative => "relative",
        WidgetId::Map => "map",
        WidgetId::Minimap => "minimap",
        WidgetId::Radar => "radar",
        WidgetId::Dash => "dash",
        WidgetId::Ticker => "ticker",
        WidgetId::Sys => "sys",
        WidgetId::Sector => "sector",
        WidgetId::Delta => "delta",
        WidgetId::Stance => "stance",
        WidgetId::Flag => "flag",
        WidgetId::Lean => "lean",
        WidgetId::Gamepad => "gamepad",
        WidgetId::Telemetry => "telemetry",
    }
}

fn widget_label(id: WidgetId) -> &'static str {
    match id {
        WidgetId::Standings => "Standings",
        WidgetId::Relative => "Relative",
        WidgetId::Map => "Map",
        WidgetId::Minimap => "Minimap",
        WidgetId::Radar => "Radar",
        WidgetId::Dash => "Dash",
        WidgetId::Ticker => "H-Standings",
        WidgetId::Sys => "Systems",
        WidgetId::Sector => "Sectors",
        WidgetId::Delta => "Delta Bar",
        WidgetId::Stance => "Stance",
        WidgetId::Flag => "Flags",
        WidgetId::Lean => "Lean",
        WidgetId::Gamepad => "Controller",
        WidgetId::Telemetry => "Telemetry",
    }
}

fn widget_from_key(s: &str) -> Option<WidgetId> {
    WidgetId::ALL
        .into_iter()
        .find(|id| widget_key(*id) == s)
}

fn preset_from_key(s: &str) -> Option<SessionPreset> {
    SessionPreset::ALL
        .into_iter()
        .find(|p| p.key() == s)
}

fn stream_layout_json() -> String {
    let edit = edit_preset();
    config::with_config(|cfg| {
        let live = cfg.active_preset;
        let lay = cfg.stream_slot(edit);
        let mut widgets = String::from("[");
        for (i, id) in WidgetId::ALL.iter().enumerate() {
            if i > 0 {
                widgets.push(',');
            }
            let p = &lay[*id];
            widgets.push_str(&format!(
                r#"{{"id":"{}","label":"{}","show":{},"font":{},"bold":{},"bg":{},"rect":{{"x":{:.6},"y":{:.6},"w":{:.6},"h":{:.6}}}}}"#,
                widget_key(*id),
                widget_label(*id),
                if p.show { "true" } else { "false" },
                p.font,
                if p.bold { "true" } else { "false" },
                p.bg,
                p.rect.x,
                p.rect.y,
                p.rect.w,
                p.rect.h,
            ));
        }
        widgets.push(']');
        format!(
            r#"{{"live":"{}","edit":"{}","obsUrl":"{}","editUrl":"{}","widgets":{}}}"#,
            live.key(),
            edit.key(),
            url(),
            edit_url(),
            widgets
        )
    })
}

fn json_str_field<'a>(body: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{key}\"");
    let i = body.find(&needle)?;
    let rest = &body[i + needle.len()..];
    let rest = rest.trim_start_matches([' ', '\t', '\n', '\r', ':']);
    if rest.starts_with('"') {
        let rest = &rest[1..];
        let end = rest.find('"')?;
        Some(&rest[..end])
    } else {
        None
    }
}

fn json_bool_field(body: &str, key: &str) -> Option<bool> {
    let needle = format!("\"{key}\"");
    let i = body.find(&needle)?;
    let rest = body[i + needle.len()..].trim_start_matches([' ', '\t', '\n', '\r', ':']);
    if rest.starts_with("true") {
        Some(true)
    } else if rest.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

fn json_i32_field(body: &str, key: &str) -> Option<i32> {
    let needle = format!("\"{key}\"");
    let i = body.find(&needle)?;
    let rest = body[i + needle.len()..].trim_start_matches([' ', '\t', '\n', '\r', ':']);
    let num: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '-' )
        .collect();
    num.parse().ok()
}

fn json_f32_in_rect(body: &str, key: &str) -> Option<f32> {
    let rect_i = body.find("\"rect\"")?;
    let slice = &body[rect_i..];
    let needle = format!("\"{key}\"");
    let i = slice.find(&needle)?;
    let rest = slice[i + needle.len()..].trim_start_matches([' ', '\t', '\n', '\r', ':']);
    let num: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '-' || *c == '.' || *c == 'e' || *c == 'E')
        .collect();
    num.parse().ok()
}

fn apply_stream_layout_post(body: &str) -> String {
    if let Some(p) = json_str_field(body, "preset").and_then(preset_from_key) {
        set_edit_preset(p);
    }
    let preset = edit_preset();
    let copy_game = json_bool_field(body, "copyGame").unwrap_or(false);
    config::update_config(|cfg| {
        if copy_game {
            cfg.settings_preset = preset;
            cfg.copy_game_edit_to_stream();
        }
        if let Some(id) = json_str_field(body, "widget").and_then(widget_from_key) {
            let slot = &mut cfg.stream_slot_mut(preset)[id];
            if let Some(show) = json_bool_field(body, "show") {
                slot.show = show;
            }
            if let Some(bold) = json_bool_field(body, "bold") {
                slot.bold = bold;
            }
            if let Some(font) = json_i32_field(body, "font") {
                slot.font = font.clamp(70, 160);
            }
            if let Some(bg) = json_i32_field(body, "bg") {
                slot.bg = bg.clamp(0, 100);
            }
            if body.contains("\"rect\"") {
                if let (Some(x), Some(y), Some(w), Some(h)) = (
                    json_f32_in_rect(body, "x"),
                    json_f32_in_rect(body, "y"),
                    json_f32_in_rect(body, "w"),
                    json_f32_in_rect(body, "h"),
                ) {
                    slot.rect.x = x.clamp(0.0, 1.0);
                    slot.rect.y = y.clamp(0.0, 1.0);
                    slot.rect.w = w.clamp(0.04, 1.0);
                    slot.rect.h = h.clamp(0.04, 1.0);
                }
            }
        }
    });
    stream_layout_json()
}

fn sec_websocket_key(req: &str) -> Option<String> {
    for line in req.lines() {
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("Sec-WebSocket-Key") {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

fn upgrade_websocket(stream: &mut TcpStream, key: &str) -> std::io::Result<()> {
    let accept = ws_accept_key(key);
    let resp = format!(
        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
    );
    stream.write_all(resp.as_bytes())
}

fn ws_accept_key(key: &str) -> String {
    use sha1::{Digest, Sha1};
    let mut hasher = Sha1::new();
    hasher.update(key.as_bytes());
    hasher.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    base64_encode(&hasher.finalize())
}

fn ws_binary_frame(payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len() + 10);
    out.push(0x82);
    let len = payload.len();
    if len < 126 {
        out.push(len as u8);
    } else if len <= 65535 {
        out.push(126);
        out.extend_from_slice(&(len as u16).to_be_bytes());
    } else {
        out.push(127);
        out.extend_from_slice(&(len as u64).to_be_bytes());
    }
    out.extend_from_slice(payload);
    out
}

fn base64_encode(bytes: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            T[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

const STREAM_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8"/>
<title>Holeshot HUD — Stream</title>
<style>
  html, body { margin: 0; width: 100%; height: 100%; background: transparent; overflow: hidden; }
  canvas { display: block; width: 100%; height: 100%; background: transparent; }
</style>
</head>
<body>
<canvas id="c" width="1920" height="1080"></canvas>
<script>
(() => {
  const canvas = document.getElementById("c");
  const ctx = canvas.getContext("2d");
  const fit = () => {
    const w = window.innerWidth || 1920;
    const h = window.innerHeight || 1080;
    canvas.width = w;
    canvas.height = h;
  };
  fit();
  window.addEventListener("resize", fit);
  let ws;
  const connect = () => {
    const proto = location.protocol === "https:" ? "wss" : "ws";
    ws = new WebSocket(`${proto}://${location.host}/ws`);
    ws.binaryType = "arraybuffer";
    ws.onmessage = async (ev) => {
      const blob = new Blob([ev.data], { type: "image/png" });
      const bmp = await createImageBitmap(blob);
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      ctx.drawImage(bmp, 0, 0, canvas.width, canvas.height);
      bmp.close();
    };
    ws.onclose = () => setTimeout(connect, 1000);
    ws.onerror = () => { try { ws.close(); } catch (_) {} };
  };
  connect();
})();
</script>
</body>
</html>
"#;
