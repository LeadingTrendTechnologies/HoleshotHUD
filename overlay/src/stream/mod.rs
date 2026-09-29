//! Localhost Browser Source feed for OBS + `/edit` stream layout editor.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use tiny_skia::Pixmap;

use crate::config::{self, HudConfig, SessionPreset, WidgetId};

const DEFAULT_PORT: u16 = 8765;
const PORT_LAST: u16 = 8775;
const STREAM_W: u32 = 1920;
const STREAM_H: u32 = 1080;
const MIN_FRAME_GAP: Duration = Duration::from_millis(16);

static ENABLED: AtomicBool = AtomicBool::new(false);
static BOUND_PORT: AtomicU16 = AtomicU16::new(0);
static CLIENTS: AtomicUsize = AtomicUsize::new(0);
static EDIT_CLIENTS: AtomicUsize = AtomicUsize::new(0);
static EDIT_PRESET: AtomicU8 = AtomicU8::new(0);
static STOP: AtomicBool = AtomicBool::new(false);
static SERVER: OnceLock<Mutex<Option<ServerState>>> = OnceLock::new();

struct ServerState {
    join: Option<JoinHandle<()>>,
    painter: Option<JoinHandle<()>>,
    writer: Option<JoinHandle<()>>,
}

struct PaintJob {
    snap: Option<crate::shm::Snapshot>,
    live_cfg: crate::config::HudConfig,
    freq: i64,
}

static LATEST: Mutex<Option<PaintJob>> = Mutex::new(None);
static LATEST_CV: Condvar = Condvar::new();
static WRITER_CV: Condvar = Condvar::new();
static WRITER_PING: Mutex<()> = Mutex::new(());
static WRITER_SEQ: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone)]
struct SharedFrame {
    /// Live update. PNG bytes are omitted for widgets that did not change.
    bytes: Arc<Vec<u8>>,
    /// Full picture for a client that just connected.
    keyframe: Arc<Vec<u8>>,
    generation: u64,
}

struct ClientOut {
    stream: TcpStream,
    /// Websocket frame still being written. Empty once the last byte is sent.
    pending: Vec<u8>,
    sent: usize,
    /// Generation of the frame in `pending`, or the last frame fully sent.
    installed: u64,
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
    let last_png: Arc<Mutex<Option<SharedFrame>>> = Arc::new(Mutex::new(None));
    let last_edit_png: Arc<Mutex<Option<SharedFrame>>> = Arc::new(Mutex::new(None));
    let clients_thread = Arc::clone(&clients);
    let edit_clients_thread = Arc::clone(&edit_clients);
    let last_png_thread = Arc::clone(&last_png);
    let last_edit_png_thread = Arc::clone(&last_edit_png);
    let clients_paint = Arc::clone(&clients);
    let edit_clients_paint = Arc::clone(&edit_clients);
    let last_png_paint = Arc::clone(&last_png);
    let last_edit_png_paint = Arc::clone(&last_edit_png);
    let clients_write = Arc::clone(&clients);
    let edit_clients_write = Arc::clone(&edit_clients);
    let last_png_write = Arc::clone(&last_png);
    let last_edit_png_write = Arc::clone(&last_edit_png);
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
    let painter = thread::Builder::new()
        .name("mxbo-stream-paint".into())
        .spawn(move || {
            run_painter(
                clients_paint,
                edit_clients_paint,
                last_png_paint,
                last_edit_png_paint,
            )
        })
        .ok();
    let writer = thread::Builder::new()
        .name("mxbo-stream-write".into())
        .spawn(move || {
            run_writer(
                clients_write,
                edit_clients_write,
                last_png_write,
                last_edit_png_write,
            )
        })
        .ok();
    *slot = Some(ServerState {
        join,
        painter,
        writer,
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
    LATEST_CV.notify_all();
    wake_writer();
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
        if let Some(painter) = state.painter {
            let _ = painter.join();
        }
        if let Some(writer) = state.writer {
            let _ = writer.join();
        }
    }
    *LATEST.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

/// Store the newest HUD sample. The paint thread encodes it off the overlay loop.
pub fn publish_frame(snap: Option<crate::shm::Snapshot>, live_cfg: HudConfig, freq: i64) {
    if !is_running() || client_count() == 0 {
        return;
    }
    let mut latest = LATEST.lock().unwrap_or_else(|e| e.into_inner());
    *latest = Some(PaintJob {
        snap,
        live_cfg,
        freq,
    });
    drop(latest);
    LATEST_CV.notify_one();
}

fn run_painter(
    clients: Arc<Mutex<Vec<ClientOut>>>,
    edit_clients: Arc<Mutex<Vec<ClientOut>>>,
    last_png: Arc<Mutex<Option<SharedFrame>>>,
    last_edit_png: Arc<Mutex<Option<SharedFrame>>>,
) {
    let Some(mut frame_px) = Pixmap::new(STREAM_W, STREAM_H) else {
        return;
    };
    let mut obs_cache = empty_widget_cache();
    let mut edit_cache = empty_widget_cache();
    let mut fonts: Option<crate::render::Fonts> = None;
    let mut font_family = None;
    let mut next_at = Instant::now();
    while !STOP.load(Ordering::SeqCst) && ENABLED.load(Ordering::SeqCst) {
        let job = {
            let mut latest = LATEST.lock().unwrap_or_else(|e| e.into_inner());
            while latest.is_none() && !STOP.load(Ordering::SeqCst) && ENABLED.load(Ordering::SeqCst)
            {
                let (guard, _) = LATEST_CV
                    .wait_timeout(latest, Duration::from_millis(50))
                    .unwrap_or_else(|e| e.into_inner());
                latest = guard;
            }
            let now = Instant::now();
            if now < next_at && !STOP.load(Ordering::SeqCst) {
                let (guard, _) = LATEST_CV
                    .wait_timeout(latest, next_at.saturating_duration_since(now))
                    .unwrap_or_else(|e| e.into_inner());
                latest = guard;
            }
            latest.take()
        };
        if STOP.load(Ordering::SeqCst) || !ENABLED.load(Ordering::SeqCst) {
            break;
        }
        let Some(job) = job else {
            continue;
        };
        next_at = Instant::now() + MIN_FRAME_GAP;
        paint_job(
            &job,
            &clients,
            &edit_clients,
            &last_png,
            &last_edit_png,
            &mut frame_px,
            &mut obs_cache,
            &mut edit_cache,
            &mut fonts,
            &mut font_family,
        );
    }
}

fn paint_job(
    job: &PaintJob,
    clients: &Arc<Mutex<Vec<ClientOut>>>,
    edit_clients: &Arc<Mutex<Vec<ClientOut>>>,
    last_png: &Arc<Mutex<Option<SharedFrame>>>,
    last_edit_png: &Arc<Mutex<Option<SharedFrame>>>,
    frame_px: &mut Pixmap,
    obs_cache: &mut WidgetCache,
    edit_cache: &mut WidgetCache,
    fonts: &mut Option<crate::render::Fonts>,
    font_family: &mut Option<crate::config::FontFamily>,
) {
    let obs_n = clients.lock().map(|c| c.len()).unwrap_or(0);
    let edit_n = edit_clients.lock().map(|c| c.len()).unwrap_or(0);
    if obs_n == 0 && edit_n == 0 {
        return;
    }
    let age = snapshot_age(job.snap.as_ref().map(|s| s.tick_qpc).unwrap_or(0), job.freq);

    if job.snap.is_none() {
        return;
    }

    let family = job.live_cfg.font_family;
    if font_family.is_none_or(|current| current != family) {
        *fonts = crate::render::Fonts::for_family(family);
        *font_family = fonts.as_ref().map(|_| family);
    }
    let Some(fonts) = fonts.as_ref() else {
        return;
    };

    let edit = edit_preset();
    if obs_n > 0 && edit_n > 0 && edit == job.live_cfg.active_preset {
        if let Some(snap) = job.snap.as_ref() {
            let (delta, keyframe) = paint_widgets(frame_px, fonts, snap, &job.live_cfg, age, obs_cache);
            *edit_cache = obs_cache.clone();
            let delta = Arc::new(delta);
            let keyframe = Arc::new(keyframe);
            store_frame(last_png, Arc::clone(&delta), Arc::clone(&keyframe));
            store_frame(last_edit_png, delta, keyframe);
        }
        return;
    }

    if obs_n > 0 {
        if let Some(snap) = job.snap.as_ref() {
            let (delta, keyframe) = paint_widgets(frame_px, fonts, snap, &job.live_cfg, age, obs_cache);
            store_frame(last_png, Arc::new(delta), Arc::new(keyframe));
        }
    }

    if edit_n > 0 {
        let edit_cfg = config::with_config(|c| c.for_stream_preset(edit));
        let mut edit_snap = job.snap.clone();
        if let Some(s) = edit_snap.as_mut() {
            edit_cfg.apply_stream_live_to_snapshot(s);
        }
        if let Some(snap) = edit_snap.as_ref() {
            let (delta, keyframe) =
                paint_widgets(frame_px, fonts, snap, &edit_cfg, age, edit_cache);
            store_frame(last_edit_png, Arc::new(delta), Arc::new(keyframe));
        }
    }
}

fn snapshot_age(tick: u64, freq: i64) -> f32 {
    if freq <= 0 || tick == 0 {
        return 0.0;
    }
    let mut now = 0i64;
    unsafe {
        windows::Win32::System::Performance::QueryPerformanceCounter(&mut now).ok();
    }
    let raw = ((now as f64 - tick as f64) / freq as f64) as f32;
    raw.clamp(0.0, 0.08)
}

type WidgetCache = [Option<CachedCrop>; WidgetId::COUNT];

#[derive(Clone)]
struct CachedCrop {
    hash: u64,
    width: u32,
    height: u32,
    png: Arc<Vec<u8>>,
    sent: bool,
}

struct WidgetPiece {
    id: u8,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    png: Arc<Vec<u8>>,
    changed: bool,
}

fn empty_widget_cache() -> WidgetCache {
    std::array::from_fn(|_| None)
}

fn paint_widgets(
    px: &mut Pixmap,
    fonts: &crate::render::Fonts,
    snap: &crate::shm::Snapshot,
    cfg: &HudConfig,
    age: f32,
    cache: &mut WidgetCache,
) -> (Vec<u8>, Vec<u8>) {
    px.fill(tiny_skia::Color::TRANSPARENT);
    crate::render::draw(
        px,
        fonts,
        Some(snap),
        cfg,
        STREAM_W,
        STREAM_H,
        age,
        false,
        false,
        false,
    );
    let pieces = widget_pieces(px, snap, cfg, cache);
    (
        write_widget_message(&pieces, false),
        write_widget_message(&pieces, true),
    )
}

const CROP_PAD: i32 = 8;

struct WidgetCrop {
    id: u8,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

fn widget_crops(snap: &crate::shm::Snapshot, cfg: &HudConfig) -> Vec<WidgetCrop> {
    let mut crops = Vec::new();
    let push = |crops: &mut Vec<WidgetCrop>, id: WidgetId, rect: crate::shm::Rect| {
        let Some(crop) = padded_pixel_rect(rect) else {
            return;
        };
        crops.push(WidgetCrop {
            id: id.idx() as u8,
            x: crop.0,
            y: crop.1,
            width: crop.2,
            height: crop.3,
        });
    };
    if snap.show_standings != 0 {
        let laid = crate::render::table_layout_rect(
            snap,
            cfg,
            WidgetId::Standings,
            snap.standings_rect,
            STREAM_W as f32,
            STREAM_H as f32,
        );
        push(&mut crops, WidgetId::Standings, laid);
    }
    if snap.show_relative != 0 {
        let laid = crate::render::table_layout_rect(
            snap,
            cfg,
            WidgetId::Relative,
            snap.relative,
            STREAM_W as f32,
            STREAM_H as f32,
        );
        push(&mut crops, WidgetId::Relative, laid);
    }
    if snap.show_map != 0 {
        push(&mut crops, WidgetId::Map, snap.map);
    }
    if cfg[WidgetId::Minimap].show {
        push(&mut crops, WidgetId::Minimap, cfg[WidgetId::Minimap].rect);
    }
    if cfg[WidgetId::Radar].show {
        push(&mut crops, WidgetId::Radar, cfg[WidgetId::Radar].rect);
    }
    if cfg[WidgetId::Dash].show {
        push(&mut crops, WidgetId::Dash, cfg[WidgetId::Dash].rect);
    }
    if cfg[WidgetId::Ticker].show {
        push(&mut crops, WidgetId::Ticker, cfg[WidgetId::Ticker].rect);
    }
    if cfg[WidgetId::Sys].show {
        push(&mut crops, WidgetId::Sys, cfg[WidgetId::Sys].rect);
    }
    if cfg.sector_visible() {
        push(&mut crops, WidgetId::Sector, cfg[WidgetId::Sector].rect);
    }
    if cfg.delta_visible() {
        push(&mut crops, WidgetId::Delta, cfg[WidgetId::Delta].rect);
    }
    if cfg[WidgetId::Flag].show {
        push(&mut crops, WidgetId::Flag, cfg[WidgetId::Flag].rect);
    }
    if cfg.stance_visible() {
        push(&mut crops, WidgetId::Stance, cfg[WidgetId::Stance].rect);
    }
    if cfg[WidgetId::Lean].show {
        push(&mut crops, WidgetId::Lean, cfg[WidgetId::Lean].rect);
    }
    if cfg.gamepad_visible() {
        push(&mut crops, WidgetId::Gamepad, cfg[WidgetId::Gamepad].rect);
    }
    if cfg[WidgetId::Telemetry].show {
        push(&mut crops, WidgetId::Telemetry, cfg[WidgetId::Telemetry].rect);
    }
    if mxbo_hud::pitboard::drawing(cfg[WidgetId::Pitboard].show, cfg.pit_when) {
        push(&mut crops, WidgetId::Pitboard, cfg[WidgetId::Pitboard].rect);
    }
    crops
}

fn padded_pixel_rect(rect: crate::shm::Rect) -> Option<(u32, u32, u32, u32)> {
    let left = (rect.x * STREAM_W as f32).floor() as i32 - CROP_PAD;
    let top = (rect.y * STREAM_H as f32).floor() as i32 - CROP_PAD;
    let right = ((rect.x + rect.w) * STREAM_W as f32).ceil() as i32 + CROP_PAD;
    let bottom = ((rect.y + rect.h) * STREAM_H as f32).ceil() as i32 + CROP_PAD;
    let x0 = left.clamp(0, STREAM_W as i32);
    let y0 = top.clamp(0, STREAM_H as i32);
    let x1 = right.clamp(0, STREAM_W as i32);
    let y1 = bottom.clamp(0, STREAM_H as i32);
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    Some((x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32))
}

fn hash_rect(px: &Pixmap, x: u32, y: u32, width: u32, height: u32) -> u64 {
    let stride = px.width() as usize * 4;
    let data = px.data();
    let mut hash = 0xcbf29ce484222325u64;
    for row in 0..height as usize {
        let start = (y as usize + row) * stride + x as usize * 4;
        let end = start + width as usize * 4;
        for byte in &data[start..end] {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    hash
}

fn widget_pieces(
    px: &Pixmap,
    snap: &crate::shm::Snapshot,
    cfg: &HudConfig,
    cache: &mut WidgetCache,
) -> Vec<WidgetPiece> {
    let mut pieces = Vec::new();
    let mut seen = [false; WidgetId::COUNT];
    for crop in widget_crops(snap, cfg) {
        let index = crop.id as usize;
        if index >= WidgetId::COUNT {
            continue;
        }
        let hash = hash_rect(px, crop.x, crop.y, crop.width, crop.height);
        let unchanged = cache[index].as_ref().is_some_and(|cached| {
            cached.sent && cached.hash == hash && cached.width == crop.width && cached.height == crop.height
        });
        let png = if unchanged {
            cache[index].as_ref().unwrap().png.clone()
        } else {
            let Some(encoded) = encode_crop(px, crop.x, crop.y, crop.width, crop.height) else {
                continue;
            };
            Arc::new(encoded)
        };
        cache[index] = Some(CachedCrop {
            hash,
            width: crop.width,
            height: crop.height,
            png: Arc::clone(&png),
            sent: true,
        });
        seen[index] = true;
        pieces.push(WidgetPiece {
            id: crop.id,
            x: crop.x as u16,
            y: crop.y as u16,
            width: crop.width as u16,
            height: crop.height as u16,
            png,
            changed: !unchanged,
        });
    }
    for (index, slot) in cache.iter_mut().enumerate() {
        if !seen[index] {
            if let Some(cached) = slot.as_mut() {
                cached.sent = false;
            }
        }
    }
    pieces
}

fn write_widget_message(pieces: &[WidgetPiece], keyframe: bool) -> Vec<u8> {
    let mut payload = Vec::from(&b"HSF2"[..]);
    payload.extend_from_slice(&(pieces.len() as u16).to_le_bytes());
    for piece in pieces {
        let include_png = keyframe || piece.changed;
        payload.push(piece.id);
        payload.push(u8::from(include_png));
        payload.extend_from_slice(&piece.x.to_le_bytes());
        payload.extend_from_slice(&piece.y.to_le_bytes());
        payload.extend_from_slice(&piece.width.to_le_bytes());
        payload.extend_from_slice(&piece.height.to_le_bytes());
        if include_png {
            payload.extend_from_slice(&(piece.png.len() as u32).to_le_bytes());
            payload.extend_from_slice(piece.png.as_slice());
        }
    }
    payload
}

fn encode_crop(px: &Pixmap, x: u32, y: u32, width: u32, height: u32) -> Option<Vec<u8>> {
    if width == 0 || height == 0 {
        return None;
    }
    let frame_width = px.width() as usize;
    let pixels = px.pixels();
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    for row in 0..height as usize {
        let source_index = (y as usize + row) * frame_width + x as usize;
        let dest_row = &mut rgba[row * width as usize * 4..];
        for (column, pixel) in pixels[source_index..source_index + width as usize]
            .iter()
            .enumerate()
        {
            let color = pixel.demultiply();
            let channel = column * 4;
            dest_row[channel] = color.red();
            dest_row[channel + 1] = color.green();
            dest_row[channel + 2] = color.blue();
            dest_row[channel + 3] = color.alpha();
        }
    }
    let mut data = Vec::new();
    let mut encoder = png::Encoder::new(&mut data, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::Fast);
    let mut writer = encoder.write_header().ok()?;
    writer.write_image_data(&rgba).ok()?;
    drop(writer);
    Some(data)
}

fn store_frame(slot: &Mutex<Option<SharedFrame>>, bytes: Arc<Vec<u8>>, keyframe: Arc<Vec<u8>>) {
    let mut guard = slot.lock().unwrap_or_else(|e| e.into_inner());
    let mut generation = guard
        .as_ref()
        .map(|frame| frame.generation)
        .unwrap_or(0)
        .wrapping_add(1);
    if generation == 0 {
        generation = 1;
    }
    *guard = Some(SharedFrame {
        bytes,
        keyframe,
        generation,
    });
    drop(guard);
    wake_writer();
}

fn wake_writer() {
    WRITER_SEQ.fetch_add(1, Ordering::SeqCst);
    let _guard = WRITER_PING.lock().unwrap_or_else(|e| e.into_inner());
    WRITER_CV.notify_one();
}

enum FlushPace {
    Idle,
    Blocked,
    Ready,
}

enum SocketWrite {
    Open,
    Blocked,
    Closed,
}

fn run_writer(
    clients: Arc<Mutex<Vec<ClientOut>>>,
    edit_clients: Arc<Mutex<Vec<ClientOut>>>,
    last_png: Arc<Mutex<Option<SharedFrame>>>,
    last_edit_png: Arc<Mutex<Option<SharedFrame>>>,
) {
    while !STOP.load(Ordering::SeqCst) && ENABLED.load(Ordering::SeqCst) {
        let seq = WRITER_SEQ.load(Ordering::SeqCst);
        let obs = flush_clients(&clients, &last_png, &CLIENTS);
        let edit = flush_clients(&edit_clients, &last_edit_png, &EDIT_CLIENTS);
        if STOP.load(Ordering::SeqCst) || !ENABLED.load(Ordering::SeqCst) {
            break;
        }
        let blocked = matches!(obs, FlushPace::Blocked) || matches!(edit, FlushPace::Blocked);
        let ready = matches!(obs, FlushPace::Ready) || matches!(edit, FlushPace::Ready);
        if ready && !blocked {
            continue;
        }
        let timeout = if blocked {
            Duration::from_millis(2)
        } else {
            Duration::from_millis(50)
        };
        let guard = WRITER_PING.lock().unwrap_or_else(|e| e.into_inner());
        if STOP.load(Ordering::SeqCst)
            || !ENABLED.load(Ordering::SeqCst)
            || WRITER_SEQ.load(Ordering::SeqCst) != seq
        {
            continue;
        }
        let (_guard, _) = WRITER_CV
            .wait_timeout(guard, timeout)
            .unwrap_or_else(|e| e.into_inner());
    }
}

/// Finishes the in-flight frame, then installs the newest PNG. One frame at a time.
fn flush_clients(
    clients: &Mutex<Vec<ClientOut>>,
    slot: &Mutex<Option<SharedFrame>>,
    counter: &AtomicUsize,
) -> FlushPace {
    let latest = slot
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let Ok(mut list) = clients.lock() else {
        return FlushPace::Idle;
    };
    let mut blocked = false;
    let mut ready = false;
    list.retain_mut(|client| {
        if client.sent >= client.pending.len() {
            client.pending.clear();
            client.sent = 0;
            if let Some(frame) = latest.as_ref() {
                if frame.generation != client.installed {
                    client.pending = ws_binary_frame(&frame.bytes);
                    client.sent = 0;
                    client.installed = frame.generation;
                }
            }
        }
        if client.sent >= client.pending.len() {
            return true;
        }
        match drive_client_write(client) {
            SocketWrite::Open => {
                ready = true;
                true
            }
            SocketWrite::Blocked => {
                blocked = true;
                true
            }
            SocketWrite::Closed => false,
        }
    });
    counter.store(list.len(), Ordering::SeqCst);
    if blocked {
        FlushPace::Blocked
    } else if ready {
        FlushPace::Ready
    } else {
        FlushPace::Idle
    }
}

/// Writes the in-flight frame without blocking. A full buffer keeps the client.
fn drive_client_write(client: &mut ClientOut) -> SocketWrite {
    while client.sent < client.pending.len() {
        match client.stream.write(&client.pending[client.sent..]) {
            Ok(0) => return SocketWrite::Closed,
            Ok(n) => client.sent += n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                return SocketWrite::Blocked;
            }
            Err(_) => return SocketWrite::Closed,
        }
    }
    client.pending.clear();
    client.sent = 0;
    SocketWrite::Open
}

fn run_server(
    clients: Arc<Mutex<Vec<ClientOut>>>,
    edit_clients: Arc<Mutex<Vec<ClientOut>>>,
    last_png: Arc<Mutex<Option<SharedFrame>>>,
    last_edit_png: Arc<Mutex<Option<SharedFrame>>>,
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
    last_png: Arc<Mutex<Option<SharedFrame>>>,
    last_edit_png: Arc<Mutex<Option<SharedFrame>>>,
) {
    let _ = stream.set_nodelay(true);
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
                // Copy before the client-list lock. The writer locks the PNG slot, then that list.
                let first_frame = last.lock().ok().and_then(|guard| guard.clone());
                if let Ok(mut guard) = list.lock() {
                    if let Ok(clone) = stream.try_clone() {
                        let _ = clone.set_nonblocking(true);
                        guard.push(ClientOut {
                            stream: clone,
                            pending: first_frame
                                .as_ref()
                                .map(|frame| ws_binary_frame(&frame.keyframe))
                                .unwrap_or_default(),
                            sent: 0,
                            installed: first_frame
                                .as_ref()
                                .map(|frame| frame.generation)
                                .unwrap_or(0),
                        });
                        counter.store(guard.len(), Ordering::SeqCst);
                    }
                }
                wake_writer();
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
        WidgetId::Pitboard => "pitboard",
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
        WidgetId::Pitboard => "Pit Board",
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
  let pending = null;
  let busy = false;
  let latestEpoch = 0;
  const widgets = new Map();
  const stageWidth = 1920;
  const stageHeight = 1080;
  const readPieces = (buffer) => {
    const bytes = new Uint8Array(buffer);
    if (bytes.length < 6 || bytes[0] !== 72 || bytes[1] !== 83 || bytes[2] !== 70 || bytes[3] !== 50) return null;
    const view = new DataView(buffer);
    const count = view.getUint16(4, true);
    const parts = [];
    let offset = 6;
    for (let index = 0; index < count; index++) {
      if (offset + 10 > buffer.byteLength) break;
      const id = view.getUint8(offset);
      const flags = view.getUint8(offset + 1);
      const x = view.getUint16(offset + 2, true);
      const y = view.getUint16(offset + 4, true);
      const w = view.getUint16(offset + 6, true);
      const h = view.getUint16(offset + 8, true);
      offset += 10;
      let png = null;
      if (flags & 1) {
        if (offset + 4 > buffer.byteLength) break;
        const length = view.getUint32(offset, true);
        offset += 4;
        if (offset + length > buffer.byteLength) break;
        png = buffer.slice(offset, offset + length);
        offset += length;
      }
      parts.push({ id: id, x: x, y: y, w: w, h: h, png: png });
    }
    return parts;
  };
  const redraw = () => {
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    const scaleX = canvas.width / stageWidth;
    const scaleY = canvas.height / stageHeight;
    for (const part of widgets.values()) {
      ctx.drawImage(part.bmp, part.x * scaleX, part.y * scaleY, part.w * scaleX, part.h * scaleY);
    }
  };
  const pump = () => {
    if (busy || !pending) return;
    const data = pending;
    pending = null;
    const epoch = latestEpoch;
    busy = true;
    const parts = readPieces(data);
    if (!parts) {
      busy = false;
      pump();
      return;
    }
    const seen = new Set();
    const positioned = [];
    const decodes = [];
    for (const part of parts) {
      seen.add(part.id);
      if (part.png) {
        decodes.push(createImageBitmap(new Blob([part.png], { type: "image/png" }))
          .then((bmp) => ({ bmp: bmp, id: part.id, x: part.x, y: part.y, w: part.w, h: part.h }))
          .catch(() => null));
      } else {
        positioned.push(part);
      }
    }
    Promise.all(decodes).then((items) => {
      const ready = items.filter(Boolean);
      if (epoch !== latestEpoch) {
        ready.forEach((part) => part.bmp.close());
        busy = false;
        pump();
        return;
      }
      for (const part of positioned) {
        const existing = widgets.get(part.id);
        if (existing) {
          existing.x = part.x;
          existing.y = part.y;
          existing.w = part.w;
          existing.h = part.h;
        }
      }
      for (const part of ready) {
        const existing = widgets.get(part.id);
        if (existing) existing.bmp.close();
        widgets.set(part.id, part);
      }
      for (const id of Array.from(widgets.keys())) {
        if (!seen.has(id)) {
          widgets.get(id).bmp.close();
          widgets.delete(id);
        }
      }
      redraw();
      busy = false;
      pump();
    }).catch(() => { busy = false; pump(); });
  };
  const connect = () => {
    const proto = location.protocol === "https:" ? "wss" : "ws";
    ws = new WebSocket(`${proto}://${location.host}/ws`);
    ws.binaryType = "arraybuffer";
    ws.onmessage = (ev) => {
      pending = ev.data;
      latestEpoch++;
      pump();
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
