//! Localhost Browser Source feed for OBS + `/edit` stream layout editor.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use tiny_skia::Pixmap;

use crate::config::{self, HudConfig, SessionPreset, WidgetId};

mod widget_settings;

const DEFAULT_PORT: u16 = 8765;
const PORT_LAST: u16 = 8775;
const DEFAULT_PAINT_W: u32 = 1920;
const DEFAULT_PAINT_H: u32 = 1080;
const MAX_PAINT_PIXELS: u64 = 1920 * 1080;

static VIEW_W: AtomicU32 = AtomicU32::new(0);
static VIEW_H: AtomicU32 = AtomicU32::new(0);

/// OBS source aspect, scaled so the bitmap stays within a 1080p pixel budget.
fn paint_size(view_w: u32, view_h: u32) -> (u32, u32) {
    if view_w < 64 || view_h < 64 {
        return (DEFAULT_PAINT_W, DEFAULT_PAINT_H);
    }
    let area = u64::from(view_w) * u64::from(view_h);
    let (mut width, mut height) = if area <= MAX_PAINT_PIXELS {
        (view_w, view_h)
    } else {
        let scale = (MAX_PAINT_PIXELS as f64 / area as f64).sqrt();
        (
            (f64::from(view_w) * scale).round().max(64.0) as u32,
            (f64::from(view_h) * scale).round().max(64.0) as u32,
        )
    };
    width = width.clamp(64, 16384);
    height = height.clamp(64, 16384);
    (width, height)
}

static ENABLED: AtomicBool = AtomicBool::new(false);
static BOUND_PORT: AtomicU16 = AtomicU16::new(0);
static CLIENTS: AtomicUsize = AtomicUsize::new(0);
static EDIT_CLIENTS: AtomicUsize = AtomicUsize::new(0);
static EDIT_PRESET: AtomicU8 = AtomicU8::new(0);
/// Bumped on each `/edit` layout write. The painter runs one text pass when this moves.
static STREAM_REV: AtomicU64 = AtomicU64::new(0);
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
    /// Age already used for the in-game frame, so the minimap bike stays with that draw.
    age: f32,
    window_w: u32,
    window_h: u32,
    /// Pixels copied out of the game draw. Empty when that widget was not drawn.
    copies: GameCopies,
    /// Layout revision captured with `live_cfg`, so a text pass is not marked done on an older frame.
    stream_rev: u64,
}

struct GameCopy {
    width: u32,
    height: u32,
    /// Premultiplied RGBA, row-major.
    pixels: Vec<u8>,
    /// Look of the layout that drew these pixels. A different stream look is painted instead.
    look: u64,
}

type GameCopies = [Option<GameCopy>; WidgetId::COUNT];

fn empty_copies() -> GameCopies {
    std::array::from_fn(|_| None)
}

static GAME_COPIES: Mutex<Option<GameCopies>> = Mutex::new(None);

static LATEST: Mutex<Option<PaintJob>> = Mutex::new(None);
static LATEST_CV: Condvar = Condvar::new();
static WRITER_CV: Condvar = Condvar::new();
static WRITER_PING: Mutex<()> = Mutex::new(());
static WRITER_SEQ: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone)]
struct OutMessage {
    id: u8,
    /// Assigned when the message is queued. Stays valid after older messages are dropped.
    seq: u64,
    /// Websocket frame, header included. Shared so the writer does not copy it.
    frame: Arc<Vec<u8>>,
}

#[derive(Clone)]
struct SharedFrame {
    /// Grows as each crop is encoded. Payloads are already websocket frames.
    messages: Vec<OutMessage>,
    /// Seq the next queued message will get.
    next_seq: u64,
    /// Seq of the first message of `generation`.
    generation_seq: u64,
    /// One partial message per widget for a client that just connected.
    keyframe: Arc<Vec<OutMessage>>,
    generation: u64,
}

struct ClientOut {
    stream: TcpStream,
    /// Websocket frame still being written.
    pending: Option<Arc<Vec<u8>>>,
    pending_id: u8,
    sent: usize,
    /// Widget messages not yet started. A newer paint replaces ids it includes.
    rest: Vec<OutMessage>,
    /// Generation of the messages already copied out of `SharedFrame`.
    installed: u64,
    /// Next seq this client has not copied.
    taken: u64,
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
    *GAME_COPIES.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

/// Copy one widget out of the overlay pixmap. Called after that widget is drawn and before the next one starts.
pub fn copy_game_widget(
    id: WidgetId,
    px: &Pixmap,
    snap: &crate::shm::Snapshot,
    cfg: &HudConfig,
) {
    if !is_running() || client_count() == 0 {
        return;
    }
    let Some(rect) = crop_rect_for(id, snap, cfg, px.width(), px.height()) else {
        return;
    };
    let Some((x, y, width, height)) = padded_pixel_rect(rect, px.width(), px.height()) else {
        return;
    };
    let Some(pixels) = copy_premul(px, x, y, width, height) else {
        return;
    };
    let mut slots = GAME_COPIES.lock().unwrap_or_else(|e| e.into_inner());
    let copies = slots.get_or_insert_with(empty_copies);
    copies[id.idx()] = Some(GameCopy {
        width,
        height,
        pixels,
        look: cfg.widget_look(id),
    });
}

fn take_game_copies() -> GameCopies {
    GAME_COPIES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
        .unwrap_or_else(empty_copies)
}

fn copy_premul(px: &Pixmap, x: u32, y: u32, width: u32, height: u32) -> Option<Vec<u8>> {
    if width == 0 || height == 0 {
        return None;
    }
    let stride = px.width() as usize * 4;
    let data = px.data();
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    for row in 0..height as usize {
        let source = (y as usize + row) * stride + x as usize * 4;
        let dest = row * width as usize * 4;
        let end = source + width as usize * 4;
        if end > data.len() {
            return None;
        }
        pixels[dest..dest + width as usize * 4].copy_from_slice(&data[source..end]);
    }
    Some(pixels)
}

/// Store the newest HUD sample. The paint thread encodes it off the overlay loop.
pub fn layout_rev() -> u64 {
    STREAM_REV.load(Ordering::SeqCst)
}

pub fn publish_frame(
    snap: Option<crate::shm::Snapshot>,
    live_cfg: HudConfig,
    age: f32,
    window_w: u32,
    window_h: u32,
    stream_rev: u64,
) {
    let copies = take_game_copies();
    if !is_running() || client_count() == 0 {
        return;
    }
    let mut latest = LATEST.lock().unwrap_or_else(|e| e.into_inner());
    *latest = Some(PaintJob {
        snap,
        live_cfg,
        age,
        window_w,
        window_h,
        copies,
        stream_rev,
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
    let Some(mut frame_px) = Pixmap::new(DEFAULT_PAINT_W, DEFAULT_PAINT_H) else {
        return;
    };
    let Some(mut text_px) = Pixmap::new(DEFAULT_PAINT_W, DEFAULT_PAINT_H) else {
        return;
    };
    let mut obs_cache = empty_widget_cache();
    let mut edit_cache = empty_widget_cache();
    let mut fonts: Option<crate::render::Fonts> = None;
    let mut font_family = None;
    let mut painted_text_rev = 0u64;
    let mut painted_edit: Option<SessionPreset> = None;
    let mut edit_cleared_for: Option<SessionPreset> = None;
    let mut stream_cleared = false;
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
            latest.take()
        };
        if STOP.load(Ordering::SeqCst) || !ENABLED.load(Ordering::SeqCst) {
            break;
        }
        let Some(job) = job else {
            continue;
        };
        match_paint_size(&mut frame_px, &mut text_px, &mut obs_cache, &mut edit_cache);
        paint_job(
            &job,
            &clients,
            &edit_clients,
            &last_png,
            &last_edit_png,
            &mut frame_px,
            &mut text_px,
            &mut obs_cache,
            &mut edit_cache,
            &mut fonts,
            &mut font_family,
            &mut painted_text_rev,
            &mut painted_edit,
            &mut edit_cleared_for,
            &mut stream_cleared,
        );
    }
}

fn match_paint_size(
    px: &mut Pixmap,
    text_px: &mut Pixmap,
    obs_cache: &mut WidgetCache,
    edit_cache: &mut WidgetCache,
) {
    let (width, height) = paint_size(
        VIEW_W.load(Ordering::SeqCst),
        VIEW_H.load(Ordering::SeqCst),
    );
    if px.width() == width && px.height() == height && text_px.width() == width && text_px.height() == height
    {
        return;
    }
    let Some(next) = Pixmap::new(width, height) else {
        return;
    };
    let Some(next_text) = Pixmap::new(width, height) else {
        return;
    };
    *px = next;
    *text_px = next_text;
    *obs_cache = empty_widget_cache();
    *edit_cache = empty_widget_cache();
}

fn paint_job(
    job: &PaintJob,
    clients: &Arc<Mutex<Vec<ClientOut>>>,
    edit_clients: &Arc<Mutex<Vec<ClientOut>>>,
    last_png: &Arc<Mutex<Option<SharedFrame>>>,
    last_edit_png: &Arc<Mutex<Option<SharedFrame>>>,
    frame_px: &mut Pixmap,
    text_px: &mut Pixmap,
    obs_cache: &mut WidgetCache,
    edit_cache: &mut WidgetCache,
    fonts: &mut Option<crate::render::Fonts>,
    font_family: &mut Option<crate::config::FontFamily>,
    painted_text_rev: &mut u64,
    painted_edit: &mut Option<SessionPreset>,
    edit_cleared_for: &mut Option<SessionPreset>,
    stream_cleared: &mut bool,
) {
    let obs_n = clients.lock().map(|c| c.len()).unwrap_or(0);
    let edit_n = edit_clients.lock().map(|c| c.len()).unwrap_or(0);
    if obs_n == 0 && edit_n == 0 {
        return;
    }
    let age = job.age;
    let window_w = job.window_w;
    let window_h = job.window_h;

    // Garage and menus have no session. The game overlay returns without drawing.
    // Widget messages are partial, so the last on-track frame stays up until each id is removed.
    let in_session = job
        .snap
        .as_ref()
        .is_some_and(|snap| snap.has_session_data());
    if !in_session {
        if !*stream_cleared {
            let paint_w = frame_px.width();
            let paint_h = frame_px.height();
            push_edit_removes(last_png, paint_w, paint_h);
            push_edit_removes(last_edit_png, paint_w, paint_h);
            finish_keyframe(last_png, Vec::new());
            finish_keyframe(last_edit_png, Vec::new());
            *obs_cache = empty_widget_cache();
            *edit_cache = empty_widget_cache();
            *painted_edit = None;
            *edit_cleared_for = None;
            *stream_cleared = true;
        }
        return;
    }
    *stream_cleared = false;

    let family = job.live_cfg.font_family;
    if font_family.is_none_or(|current| current != family) {
        *fonts = crate::render::Fonts::for_family(family);
        *font_family = fonts.as_ref().map(|_| family);
    }
    let Some(fonts) = fonts.as_ref() else {
        return;
    };

    let motion = motion_widgets();
    let text = text_widgets();
    let edit = edit_preset();
    let force_text = job.stream_rev != *painted_text_rev;
    // Widget messages are partial, so a preset switch has to name every id or the
    // previous preset stays on /edit. The shared live path paints from the OBS cache
    // and would not send those removes.
    let preset_changed = edit_n > 0 && *painted_edit != Some(edit);
    if preset_changed && *edit_cleared_for != Some(edit) {
        push_edit_removes(last_edit_png, frame_px.width(), frame_px.height());
        *edit_cache = empty_widget_cache();
        *edit_cleared_for = Some(edit);
    }
    if obs_n > 0 && edit_n > 0 && edit == job.live_cfg.active_preset && !preset_changed {
        if let Some(snap) = job.snap.as_ref() {
            let yielded = paint_pass(
                frame_px,
                fonts,
                snap,
                &job.live_cfg,
                age,
                window_w,
                window_h,
                obs_cache,
                &[last_png, last_edit_png],
                &motion,
                &job.copies,
                true,
                false,
            );
            if force_text || (!yielded && !sample_waiting()) {
                paint_pass(
                    text_px,
                    fonts,
                    snap,
                    &job.live_cfg,
                    age,
                    window_w,
                    window_h,
                    obs_cache,
                    &[last_png, last_edit_png],
                    &text,
                    &job.copies,
                    false,
                    force_text,
                );
                *painted_text_rev = job.stream_rev;
            }
            *edit_cache = obs_cache.clone();
        }
        return;
    }

    if obs_n > 0 {
        if let Some(snap) = job.snap.as_ref() {
            let yielded = paint_pass(
                frame_px,
                fonts,
                snap,
                &job.live_cfg,
                age,
                window_w,
                window_h,
                obs_cache,
                &[last_png],
                &motion,
                &job.copies,
                true,
                false,
            );
            if !force_text && (yielded || sample_waiting()) {
                return;
            }
        }
    }

    let edit_board = if edit_n > 0 {
        let edit_cfg = config::with_config(|c| c.for_stream_preset(edit));
        let mut edit_snap = job.snap.clone();
        if let Some(s) = edit_snap.as_mut() {
            edit_cfg.apply_stream_live_to_snapshot(s);
        }
        Some((edit_cfg, edit_snap))
    } else {
        None
    };
    if let Some((edit_cfg, edit_snap)) = edit_board.as_ref() {
        if let Some(snap) = edit_snap.as_ref() {
            let yielded = paint_pass(
                frame_px,
                fonts,
                snap,
                edit_cfg,
                age,
                window_w,
                window_h,
                edit_cache,
                &[last_edit_png],
                &motion,
                &job.copies,
                true,
                false,
            );
            if !force_text && (yielded || sample_waiting()) {
                return;
            }
        }
    }

    if obs_n > 0 {
        if let Some(snap) = job.snap.as_ref() {
            let yielded = paint_pass(
                text_px,
                fonts,
                snap,
                &job.live_cfg,
                age,
                window_w,
                window_h,
                obs_cache,
                &[last_png],
                &text,
                &job.copies,
                false,
                force_text,
            );
            if !force_text && (yielded || sample_waiting()) {
                return;
            }
        }
    }

    if let Some((edit_cfg, edit_snap)) = edit_board.as_ref() {
        if let Some(snap) = edit_snap.as_ref() {
            paint_pass(
                text_px,
                fonts,
                snap,
                edit_cfg,
                age,
                window_w,
                window_h,
                edit_cache,
                &[last_edit_png],
                &text,
                &job.copies,
                false,
                force_text,
            );
            *painted_edit = Some(edit);
        }
    }
    if force_text {
        *painted_text_rev = job.stream_rev;
    }
}

/// Tell /edit to drop every widget. A later draw of the same id replaces its remove.
fn push_edit_removes(sink: &Mutex<Option<SharedFrame>>, paint_w: u32, paint_h: u32) {
    begin_paint(sink);
    for id in WidgetId::ALL {
        let piece = WidgetPiece {
            id: id.idx() as u8,
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            png: Arc::new(Vec::new()),
            remove: true,
        };
        push_message(sink, write_widget_message(&piece, false, paint_w, paint_h));
    }
}

type WidgetCache = [Option<CachedCrop>; WidgetId::COUNT];

#[derive(Clone)]
struct CachedCrop {
    hash: u64,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    sent: bool,
    /// Last websocket frame. Reused while the crop is unchanged.
    frame: Option<OutMessage>,
}

struct WidgetPiece {
    id: u8,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    png: Arc<Vec<u8>>,
    remove: bool,
}

fn empty_widget_cache() -> WidgetCache {
    std::array::from_fn(|_| None)
}

fn sample_waiting() -> bool {
    LATEST
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .is_some()
}

fn motion_widgets() -> [bool; WidgetId::COUNT] {
    let mut mask = [false; WidgetId::COUNT];
    for id in [
        WidgetId::Map,
        WidgetId::Minimap,
        WidgetId::Radar,
        WidgetId::Dash,
        WidgetId::Lean,
        WidgetId::Stance,
        WidgetId::Flag,
    ] {
        mask[id.idx()] = true;
    }
    mask
}

fn text_widgets() -> [bool; WidgetId::COUNT] {
    let motion = motion_widgets();
    std::array::from_fn(|index| !motion[index])
}

/// Draw one set of widgets. Motion passes start a generation. The text pass only pushes,
/// and either pass returns when a newer sample is already waiting.
/// `force` finishes this pass even if a newer sample is queued.
fn paint_pass(
    px: &mut Pixmap,
    fonts: &crate::render::Fonts,
    snap: &crate::shm::Snapshot,
    cfg: &HudConfig,
    age: f32,
    window_w: u32,
    window_h: u32,
    cache: &mut WidgetCache,
    sinks: &[&Mutex<Option<SharedFrame>>],
    include: &[bool; WidgetId::COUNT],
    copies: &GameCopies,
    start_generation: bool,
    force: bool,
) -> bool {
    let paint_w = px.width();
    let paint_h = px.height();
    if start_generation {
        for sink in sinks {
            begin_paint(sink);
        }
    }
    let crops = widget_crops(snap, cfg, paint_w, paint_h);
    remove_hidden(cache, &crops, include, sinks, paint_w, paint_h);
    let mut draw_include = *include;
    let mut yielded = false;
    place_game_copies(
        px,
        cfg,
        &crops,
        copies,
        include,
        &mut draw_include,
        cache,
        sinks,
        paint_w,
        paint_h,
    );
    if !force && sample_waiting() {
        yielded = true;
    }
    let still_drawing = crops.iter().any(|crop| {
        let index = crop.id as usize;
        index < WidgetId::COUNT && draw_include[index]
    });
    if snap.has_session_data() && !yielded && still_drawing {
        crate::render::draw_each(
            px,
            fonts,
            Some(snap),
            cfg,
            paint_w,
            paint_h,
            age,
            window_w,
            window_h,
            Some(&draw_include),
            |widget_id, pixmap| {
                let index = widget_id.idx();
                if index < WidgetId::COUNT {
                    if let Some(crop) = crops.iter().find(|crop| crop.id == index as u8) {
                        queue_crop(pixmap, crop, cache, sinks, paint_w, paint_h);
                    }
                }
                if !force && sample_waiting() {
                    yielded = true;
                    return false;
                }
                true
            },
        );
    }
    if start_generation {
        let keyframe = keyframe_from_cache(cache);
        for sink in sinks {
            finish_keyframe(sink, keyframe.clone());
        }
    }
    yielded
}

const CROP_PAD: i32 = 8;

struct WidgetCrop {
    id: u8,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

fn widget_crops(
    snap: &crate::shm::Snapshot,
    cfg: &HudConfig,
    paint_w: u32,
    paint_h: u32,
) -> Vec<WidgetCrop> {
    let mut crops = Vec::new();
    for id in WidgetId::ALL {
        let Some(rect) = crop_rect_for(id, snap, cfg, paint_w, paint_h) else {
            continue;
        };
        let Some(crop) = padded_pixel_rect(rect, paint_w, paint_h) else {
            continue;
        };
        crops.push(WidgetCrop {
            id: id.idx() as u8,
            x: crop.0,
            y: crop.1,
            width: crop.2,
            height: crop.3,
        });
    }
    crops
}

fn crop_rect_for(
    id: WidgetId,
    snap: &crate::shm::Snapshot,
    cfg: &HudConfig,
    frame_w: u32,
    frame_h: u32,
) -> Option<crate::shm::Rect> {
    let width = frame_w as f32;
    let height = frame_h as f32;
    match id {
        WidgetId::Standings if snap.show_standings != 0 => Some(crate::render::table_layout_rect(
            snap,
            cfg,
            id,
            snap.standings_rect,
            width,
            height,
        )),
        WidgetId::Relative if snap.show_relative != 0 => Some(crate::render::table_layout_rect(
            snap,
            cfg,
            id,
            snap.relative,
            width,
            height,
        )),
        WidgetId::Map if snap.show_map != 0 => Some(snap.map),
        WidgetId::Minimap if cfg[id].show => Some(mxbo_hud::layout::visual_rect(
            cfg[id].rect,
            id,
            width,
            height,
        )),
        WidgetId::Radar if cfg[id].show => Some(cfg[id].rect),
        WidgetId::Dash if cfg[id].show => Some(cfg[id].rect),
        WidgetId::Ticker if cfg[id].show => Some(cfg[id].rect),
        WidgetId::Sys if cfg[id].show => Some(cfg[id].rect),
        WidgetId::Sector if cfg.sector_visible() => Some(cfg[id].rect),
        WidgetId::Delta if cfg.delta_visible() => Some(cfg[id].rect),
        WidgetId::Flag if cfg[id].show => Some(cfg[id].rect),
        WidgetId::Stance if cfg.stance_visible() => Some(cfg[id].rect),
        WidgetId::Lean if cfg[id].show => Some(cfg[id].rect),
        WidgetId::Gamepad if cfg.gamepad_visible() => Some(cfg[id].rect),
        WidgetId::Telemetry if cfg[id].show => Some(cfg[id].rect),
        WidgetId::Pitboard
            if mxbo_hud::pitboard::drawing(cfg[id].show, cfg.pit.pit_when) =>
        {
            Some(cfg[id].rect)
        }
        _ => None,
    }
}

fn place_game_copies(
    px: &mut Pixmap,
    cfg: &HudConfig,
    crops: &[WidgetCrop],
    copies: &GameCopies,
    include: &[bool; WidgetId::COUNT],
    draw_include: &mut [bool; WidgetId::COUNT],
    cache: &mut WidgetCache,
    sinks: &[&Mutex<Option<SharedFrame>>],
    paint_w: u32,
    paint_h: u32,
) {
    for crop in crops {
        let index = crop.id as usize;
        if index >= WidgetId::COUNT || !include[index] {
            continue;
        }
        let Some(copy) = copies[index].as_ref() else {
            continue;
        };
        if copy.look != cfg.widget_look(WidgetId::ALL[index]) {
            continue;
        }
        blit_copy_fit(px, copy, crop);
        queue_crop(px, crop, cache, sinks, paint_w, paint_h);
        draw_include[index] = false;
    }
}

fn blit_copy_fit(px: &mut Pixmap, copy: &GameCopy, crop: &WidgetCrop) {
    clear_pixmap_rect(px, crop.x, crop.y, crop.width, crop.height);
    if copy.width == 0 || copy.height == 0 || crop.width == 0 || crop.height == 0 {
        return;
    }
    let scale = (crop.width as f32 / copy.width as f32).min(crop.height as f32 / copy.height as f32);
    if scale <= 0.0 {
        return;
    }
    let dest_w = ((copy.width as f32 * scale).round() as u32).clamp(1, crop.width);
    let dest_h = ((copy.height as f32 * scale).round() as u32).clamp(1, crop.height);
    let origin_x = crop.x + (crop.width - dest_w) / 2;
    let origin_y = crop.y + (crop.height - dest_h) / 2;
    let dest_stride = px.width() as usize * 4;
    let dest = px.data_mut();
    for row in 0..dest_h as usize {
        for column in 0..dest_w as usize {
            let source_x = (column as f32 + 0.5) * copy.width as f32 / dest_w as f32 - 0.5;
            let source_y = (row as f32 + 0.5) * copy.height as f32 / dest_h as f32 - 0.5;
            let pixel = sample_copy(copy, source_x, source_y);
            let offset = (origin_y as usize + row) * dest_stride + (origin_x as usize + column) * 4;
            if offset + 4 > dest.len() {
                continue;
            }
            dest[offset..offset + 4].copy_from_slice(&pixel);
        }
    }
}

fn sample_copy(copy: &GameCopy, x: f32, y: f32) -> [u8; 4] {
    let max_x = copy.width.saturating_sub(1) as f32;
    let max_y = copy.height.saturating_sub(1) as f32;
    let x = x.clamp(0.0, max_x);
    let y = y.clamp(0.0, max_y);
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(copy.width.saturating_sub(1));
    let y1 = (y0 + 1).min(copy.height.saturating_sub(1));
    let tx = x - x0 as f32;
    let ty = y - y0 as f32;
    let top = lerp_pixel(copy_pixel(copy, x0, y0), copy_pixel(copy, x1, y0), tx);
    let bottom = lerp_pixel(copy_pixel(copy, x0, y1), copy_pixel(copy, x1, y1), tx);
    lerp_pixel(top, bottom, ty)
}

fn copy_pixel(copy: &GameCopy, x: u32, y: u32) -> [u8; 4] {
    let index = ((y * copy.width + x) * 4) as usize;
    if index + 4 > copy.pixels.len() {
        return [0, 0, 0, 0];
    }
    [
        copy.pixels[index],
        copy.pixels[index + 1],
        copy.pixels[index + 2],
        copy.pixels[index + 3],
    ]
}

fn lerp_pixel(left: [u8; 4], right: [u8; 4], t: f32) -> [u8; 4] {
    let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round().clamp(0.0, 255.0) as u8;
    [mix(left[0], right[0]), mix(left[1], right[1]), mix(left[2], right[2]), mix(left[3], right[3])]
}

fn clear_pixmap_rect(px: &mut Pixmap, x: u32, y: u32, width: u32, height: u32) {
    let stride = px.width() as usize;
    let frame_h = px.height() as usize;
    let x0 = x as usize;
    let y0 = y as usize;
    if x0 >= stride || y0 >= frame_h || width == 0 || height == 0 {
        return;
    }
    let x1 = (x0 + width as usize).min(stride);
    let y1 = (y0 + height as usize).min(frame_h);
    let pixels = px.pixels_mut();
    let blank = tiny_skia::PremultipliedColorU8::TRANSPARENT;
    for row in y0..y1 {
        pixels[row * stride + x0..row * stride + x1].fill(blank);
    }
}

fn padded_pixel_rect(rect: crate::shm::Rect, paint_w: u32, paint_h: u32) -> Option<(u32, u32, u32, u32)> {
    let left = (rect.x * paint_w as f32).floor() as i32 - CROP_PAD;
    let top = (rect.y * paint_h as f32).floor() as i32 - CROP_PAD;
    let right = ((rect.x + rect.w) * paint_w as f32).ceil() as i32 + CROP_PAD;
    let bottom = ((rect.y + rect.h) * paint_h as f32).ceil() as i32 + CROP_PAD;
    let x0 = left.clamp(0, paint_w as i32);
    let y0 = top.clamp(0, paint_h as i32);
    let x1 = right.clamp(0, paint_w as i32);
    let y1 = bottom.clamp(0, paint_h as i32);
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

fn queue_crop(
    px: &Pixmap,
    crop: &WidgetCrop,
    cache: &mut WidgetCache,
    sinks: &[&Mutex<Option<SharedFrame>>],
    paint_w: u32,
    paint_h: u32,
) -> bool {
    let index = crop.id as usize;
    if index >= WidgetId::COUNT {
        return false;
    }
    let hash = hash_rect(px, crop.x, crop.y, crop.width, crop.height);
    let x = crop.x as u16;
    let y = crop.y as u16;
    let width = crop.width as u16;
    let height = crop.height as u16;
    let unchanged = cache[index].as_ref().is_some_and(|cached| {
        cached.sent
            && cached.frame.is_some()
            && cached.hash == hash
            && cached.x == x
            && cached.y == y
            && cached.width == width
            && cached.height == height
    });
    if unchanged {
        return true;
    }
    let Some(encoded) = encode_crop(px, crop.x, crop.y, crop.width, crop.height) else {
        return false;
    };
    let piece = WidgetPiece {
        id: crop.id,
        x,
        y,
        width,
        height,
        png: Arc::new(encoded),
        remove: false,
    };
    let message = write_widget_message(&piece, true, paint_w, paint_h);
    cache[index] = Some(CachedCrop {
        hash,
        x,
        y,
        width,
        height,
        sent: true,
        frame: Some(message.clone()),
    });
    for sink in sinks {
        push_message(sink, message.clone());
    }
    true
}

fn remove_hidden(
    cache: &mut WidgetCache,
    crops: &[WidgetCrop],
    include: &[bool; WidgetId::COUNT],
    sinks: &[&Mutex<Option<SharedFrame>>],
    paint_w: u32,
    paint_h: u32,
) {
    for (index, slot) in cache.iter_mut().enumerate() {
        if !include[index] || crops.iter().any(|crop| crop.id == index as u8) {
            continue;
        }
        let Some(cached) = slot.as_mut() else {
            continue;
        };
        if cached.sent {
            let piece = WidgetPiece {
                id: index as u8,
                x: 0,
                y: 0,
                width: 0,
                height: 0,
                png: Arc::new(Vec::new()),
                remove: true,
            };
            let message = write_widget_message(&piece, false, paint_w, paint_h);
            for sink in sinks {
                push_message(sink, message.clone());
            }
        }
        cached.sent = false;
        cached.frame = None;
    }
}

fn keyframe_from_cache(cache: &WidgetCache) -> Vec<OutMessage> {
    let mut messages = Vec::new();
    for slot in cache.iter() {
        if let Some(cached) = slot {
            if cached.sent {
                if let Some(frame) = &cached.frame {
                    messages.push(frame.clone());
                }
            }
        }
    }
    messages
}

const PARTIAL_FLAG: u8 = 1;
const PIXEL_FLAG: u8 = 1;
const REMOVE_FLAG: u8 = 2;

fn write_widget_message(piece: &WidgetPiece, include_pixels: bool, paint_w: u32, paint_h: u32) -> OutMessage {
    let flags = if piece.remove {
        REMOVE_FLAG
    } else if include_pixels {
        PIXEL_FLAG
    } else {
        0
    };
    let pixel_bytes = if (flags & PIXEL_FLAG) != 0 {
        4 + piece.png.len()
    } else {
        0
    };
    let body_len = 21 + pixel_bytes;
    let mut frame = Vec::with_capacity(body_len + 10);
    write_ws_header(&mut frame, body_len);
    frame.extend_from_slice(b"HSF5");
    frame.push(PARTIAL_FLAG);
    frame.extend_from_slice(&(paint_w as u16).to_le_bytes());
    frame.extend_from_slice(&(paint_h as u16).to_le_bytes());
    frame.extend_from_slice(&1u16.to_le_bytes());
    frame.push(piece.id);
    frame.push(flags);
    frame.extend_from_slice(&piece.x.to_le_bytes());
    frame.extend_from_slice(&piece.y.to_le_bytes());
    frame.extend_from_slice(&piece.width.to_le_bytes());
    frame.extend_from_slice(&piece.height.to_le_bytes());
    if (flags & PIXEL_FLAG) != 0 {
        frame.extend_from_slice(&(piece.png.len() as u32).to_le_bytes());
        frame.extend_from_slice(piece.png.as_slice());
    }
    OutMessage {
        id: piece.id,
        seq: 0,
        frame: Arc::new(frame),
    }
}

/// A newer paint replaces the same widget. Other unsent widgets stay queued.
fn replace_unsent(unsent: Vec<OutMessage>, incoming: &[OutMessage]) -> Vec<OutMessage> {
    let fresh: std::collections::HashSet<u8> = incoming.iter().map(|message| message.id).collect();
    let mut next = Vec::with_capacity(unsent.len() + incoming.len());
    next.extend(incoming.iter().cloned());
    next.extend(unsent.into_iter().filter(|message| !fresh.contains(&message.id)));
    next
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
    Some(rgba)
}

fn begin_paint(slot: &Mutex<Option<SharedFrame>>) {
    let mut guard = slot.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(frame) = guard.as_mut() {
        let mut generation = frame.generation.wrapping_add(1);
        if generation == 0 {
            generation = 1;
        }
        frame.generation = generation;
        frame.generation_seq = frame.next_seq;
    } else {
        *guard = Some(SharedFrame {
            messages: Vec::new(),
            next_seq: 0,
            generation_seq: 0,
            keyframe: Arc::new(Vec::new()),
            generation: 1,
        });
    }
    drop(guard);
    wake_writer();
}

fn push_message(slot: &Mutex<Option<SharedFrame>>, mut message: OutMessage) {
    let mut guard = slot.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(frame) = guard.as_mut() {
        message.seq = frame.next_seq;
        frame.next_seq = frame.next_seq.wrapping_add(1);
        frame.messages.push(message);
    }
    drop(guard);
    wake_writer();
}

fn finish_keyframe(slot: &Mutex<Option<SharedFrame>>, keyframe: Vec<OutMessage>) {
    let mut guard = slot.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(frame) = guard.as_mut() {
        frame.keyframe = Arc::new(keyframe);
    }
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

/// Copies newly encoded widgets into each client, then writes the in-flight frame.
fn flush_clients(
    clients: &Mutex<Vec<ClientOut>>,
    slot: &Mutex<Option<SharedFrame>>,
    counter: &AtomicUsize,
) -> FlushPace {
    {
        let mut frame_guard = slot.lock().unwrap_or_else(|e| e.into_inner());
        let Ok(mut list) = clients.lock() else {
            return FlushPace::Idle;
        };
        if let Some(frame) = frame_guard.as_mut() {
            for client in list.iter_mut() {
                if client.sent == 0 {
                    if let Some(pending_frame) = client.pending.take() {
                        client.rest.insert(
                            0,
                            OutMessage {
                                id: client.pending_id,
                                seq: 0,
                                frame: pending_frame,
                            },
                        );
                    }
                }
                take_new_messages(client, frame);
                if client.pending.is_none() && !client.rest.is_empty() {
                    let next = client.rest.remove(0);
                    client.pending_id = next.id;
                    client.pending = Some(next.frame);
                    client.sent = 0;
                }
            }
            compact_messages(frame, &mut list);
        }
    }
    let Ok(mut list) = clients.lock() else {
        return FlushPace::Idle;
    };
    let mut blocked = false;
    let mut ready = false;
    list.retain_mut(|client| {
        if client.pending.is_none() {
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

fn take_new_messages(client: &mut ClientOut, frame: &SharedFrame) {
    if frame.generation != client.installed {
        let mut unsent = std::mem::take(&mut client.rest);
        let skipped: Vec<OutMessage> = frame
            .messages
            .iter()
            .filter(|message| message.seq >= client.taken && message.seq < frame.generation_seq)
            .cloned()
            .collect();
        if !skipped.is_empty() {
            let skipped = latest_per_widget(&skipped);
            unsent = replace_unsent(unsent, &skipped);
        }
        let incoming: Vec<OutMessage> = frame
            .messages
            .iter()
            .filter(|message| message.seq >= frame.generation_seq)
            .cloned()
            .collect();
        client.rest = replace_unsent(unsent, &incoming);
        client.installed = frame.generation;
        client.taken = frame.next_seq;
    } else if client.taken < frame.next_seq {
        let incoming: Vec<OutMessage> = frame
            .messages
            .iter()
            .filter(|message| message.seq >= client.taken)
            .cloned()
            .collect();
        client.rest = replace_unsent(std::mem::take(&mut client.rest), &incoming);
        client.taken = frame.next_seq;
    }
}

fn latest_per_widget(messages: &[OutMessage]) -> Vec<OutMessage> {
    let mut last_index = [usize::MAX; 256];
    for (index, message) in messages.iter().enumerate() {
        last_index[message.id as usize] = index;
    }
    messages
        .iter()
        .enumerate()
        .filter(|(index, message)| last_index[message.id as usize] == *index)
        .map(|(_, message)| message.clone())
        .collect()
}

fn compact_messages(frame: &mut SharedFrame, clients: &mut [ClientOut]) {
    let Some(min_taken) = clients.iter().map(|client| client.taken).min() else {
        return;
    };
    frame.messages.retain(|message| message.seq >= min_taken);
}

/// Writes the in-flight frame without blocking. A full buffer keeps the client.
fn drive_client_write(client: &mut ClientOut) -> SocketWrite {
    loop {
        let len = match client.pending.as_ref() {
            Some(frame) => frame.len(),
            None => {
                client.sent = 0;
                return SocketWrite::Open;
            }
        };
        if client.sent >= len {
            client.pending = None;
            client.sent = 0;
            return SocketWrite::Open;
        }
        let wrote = {
            let frame = client.pending.as_ref().unwrap();
            client.stream.write(&frame[client.sent..])
        };
        match wrote {
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

    if method == "POST" && path_only == "/api/stream-viewport" {
        let body = read_http_body(&req_head, header_bytes, &mut stream);
        if let (Some(width), Some(height)) = (json_i32_field(&body, "w"), json_i32_field(&body, "h"))
        {
            if width >= 64 && height >= 64 {
                VIEW_W.store(width as u32, Ordering::SeqCst);
                VIEW_H.store(height as u32, Ordering::SeqCst);
            }
        }
        write_json(&mut stream, r#"{"ok":true}"#);
        return;
    }

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
                            pending: None,
                            pending_id: 0,
                            sent: 0,
                            rest: first_frame
                                .as_ref()
                                .map(|frame| frame.keyframe.as_ref().clone())
                                .unwrap_or_default(),
                            installed: first_frame
                                .as_ref()
                                .map(|frame| frame.generation)
                                .unwrap_or(0),
                            taken: first_frame
                                .as_ref()
                                .map(|frame| frame.next_seq)
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
    let (paint_w, paint_h) = paint_size(
        VIEW_W.load(Ordering::SeqCst),
        VIEW_H.load(Ordering::SeqCst),
    );
    config::with_config(|cfg| {
        let live = cfg.active_preset;
        let lay = cfg.stream_slot(edit);
        let mut widgets = String::from("[");
        for (i, id) in WidgetId::ALL.iter().enumerate() {
            if i > 0 {
                widgets.push(',');
            }
            let p = &lay[*id];
            let controls = widget_settings::controls_json(*id, lay);
            widgets.push_str(&format!(
                r#"{{"id":"{}","label":"{}","show":{},"font":{},"bold":{},"bg":{},"rect":{{"x":{:.6},"y":{:.6},"w":{:.6},"h":{:.6}}},"controls":{controls}}}"#,
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
            r#"{{"live":"{}","edit":"{}","obsUrl":"{}","editUrl":"{}","paintW":{},"paintH":{},"widgets":{}}}"#,
            live.key(),
            edit.key(),
            url(),
            edit_url(),
            paint_w,
            paint_h,
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

fn json_object_field<'a>(body: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{key}\"");
    let start = body.find(&needle)?;
    let rest = body[start + needle.len()..].trim_start_matches([' ', '\t', '\n', '\r', ':']);
    if !rest.starts_with('{') {
        return None;
    }
    let bytes = rest.as_bytes();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    for (index, byte) in bytes.iter().enumerate() {
        if in_string {
            if escaped {
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match *byte {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&rest[..=index]);
                }
            }
            _ => {}
        }
    }
    None
}

fn apply_settings_object(layout: &mut crate::config::HudLayout, object: &str) {
    let inner = object.trim().trim_start_matches('{').trim_end_matches('}');
    let bytes = inner.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        while index < bytes.len() && matches!(bytes[index], b' ' | b'\t' | b'\n' | b'\r' | b',') {
            index += 1;
        }
        if index >= bytes.len() || bytes[index] != b'"' {
            break;
        }
        index += 1;
        let key_start = index;
        while index < bytes.len() && bytes[index] != b'"' {
            index += 1;
        }
        if index >= bytes.len() {
            break;
        }
        let key = &inner[key_start..index];
        index += 1;
        while index < bytes.len() && bytes[index] != b':' {
            index += 1;
        }
        if index >= bytes.len() {
            break;
        }
        index += 1;
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index >= bytes.len() {
            break;
        }
        let value = if bytes[index] == b'"' {
            index += 1;
            let value_start = index;
            while index < bytes.len() && bytes[index] != b'"' {
                index += 1;
            }
            let text = inner[value_start..index].to_string();
            if index < bytes.len() {
                index += 1;
            }
            text
        } else if bytes[index] == b'[' {
            let value_start = index;
            let mut depth = 0i32;
            while index < bytes.len() {
                if bytes[index] == b'[' {
                    depth += 1;
                } else if bytes[index] == b']' {
                    depth -= 1;
                    if depth == 0 {
                        index += 1;
                        break;
                    }
                }
                index += 1;
            }
            join_json_strings(&inner[value_start..index])
        } else if inner[index..].starts_with("true") {
            index += 4;
            "1".to_string()
        } else if inner[index..].starts_with("false") {
            index += 5;
            "0".to_string()
        } else {
            let value_start = index;
            while index < bytes.len()
                && (bytes[index].is_ascii_digit() || matches!(bytes[index], b'-' | b'.'))
            {
                index += 1;
            }
            inner[value_start..index].to_string()
        };
        apply_stream_setting(layout, key, &value);
    }
}

fn join_json_strings(slice: &str) -> String {
    let bytes = slice.as_bytes();
    let mut joined = String::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'"' {
            index += 1;
            let start = index;
            while index < bytes.len() && bytes[index] != b'"' {
                index += 1;
            }
            if !joined.is_empty() {
                joined.push(',');
            }
            joined.push_str(&slice[start..index]);
            if index < bytes.len() {
                index += 1;
            }
        } else {
            index += 1;
        }
    }
    joined
}

fn apply_stream_setting(layout: &mut crate::config::HudLayout, key: &str, value: &str) {
    if key == "pit_vars" && !value.contains(':') {
        let keys: Vec<&str> = value
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect();
        if keys.len() == layout.pit.pit_vars.len() {
            for (place, var_key) in layout.pit.pit_vars.iter_mut().zip(keys) {
                if var_key == "none" {
                    place.show = false;
                } else if let Some(var) = mxbo_hud::pitboard::PitVar::parse(var_key) {
                    place.var = var;
                    place.show = true;
                }
            }
        }
        return;
    }
    layout.apply_setting(key, value);
}

fn apply_stream_layout_post(body: &str) -> String {
    if let Some(p) = json_str_field(body, "preset").and_then(preset_from_key) {
        set_edit_preset(p);
    }
    let preset = edit_preset();
    let copy_game = json_bool_field(body, "copyGame").unwrap_or(false);
    let copy_to = json_str_field(body, "copyTo").and_then(preset_from_key);
    config::update_config(|cfg| {
        if copy_game {
            cfg.settings_preset = preset;
            cfg.copy_game_edit_to_stream();
        }
        if let Some(dst) = copy_to {
            cfg.copy_stream_layout_to(preset, dst);
        }
        if let Some(id) = json_str_field(body, "widget").and_then(widget_from_key) {
            {
                let turning_on =
                    json_bool_field(body, "show") == Some(true) && !cfg.stream_slot(preset)[id].show;
                if turning_on {
                    cfg.seed_stream_widget_from_game(preset, id);
                }
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
            if let Some(settings) = json_object_field(body, "settings") {
                apply_settings_object(cfg.stream_slot_mut(preset), settings);
            }
        }
    });
    STREAM_REV.fetch_add(1, Ordering::SeqCst);
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

fn write_ws_header(out: &mut Vec<u8>, len: usize) {
    out.push(0x82);
    if len < 126 {
        out.push(len as u8);
    } else if len <= 65535 {
        out.push(126);
        out.extend_from_slice(&(len as u16).to_be_bytes());
    } else {
        out.push(127);
        out.extend_from_slice(&(len as u64).to_be_bytes());
    }
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
  #stage { position: absolute; left: 0; top: 0; transform-origin: top left; }
  #stage canvas { position: absolute; display: block; }
</style>
</head>
<body>
<div id="stage"></div>
<script>
(() => {
  const stage = document.getElementById("stage");
  let viewTimer = 0;
  const reportView = () => {
    clearTimeout(viewTimer);
    viewTimer = setTimeout(() => {
      const w = window.innerWidth || 1920;
      const h = window.innerHeight || 1080;
      fetch("/api/stream-viewport", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ w: w, h: h }),
      }).catch(() => {});
    }, 200);
  };
  reportView();
  window.addEventListener("resize", () => { reportView(); fitStage(); });
  let ws;
  const widgets = new Map();
  let paintW = 1920;
  let paintH = 1080;
  const fitStage = () => {
    stage.style.width = paintW + "px";
    stage.style.height = paintH + "px";
    const viewW = window.innerWidth || paintW;
    const viewH = window.innerHeight || paintH;
    if (viewW === paintW && viewH === paintH) {
      stage.style.transform = "none";
      return;
    }
    const scale = Math.min(viewW / paintW, viewH / paintH);
    stage.style.transform = "scale(" + scale + ")";
  };
  const ensure = (id) => {
    let slot = widgets.get(id);
    if (slot) return slot;
    const canvas = document.createElement("canvas");
    stage.appendChild(canvas);
    slot = { canvas: canvas, ctx: canvas.getContext("2d"), x: -1, y: -1, w: -1, h: -1, pixels: null };
    widgets.set(id, slot);
    return slot;
  };
  const move = (slot, part) => {
    if (slot.x === part.x && slot.y === part.y && slot.w === part.w && slot.h === part.h) return;
    slot.x = part.x;
    slot.y = part.y;
    slot.w = part.w;
    slot.h = part.h;
    slot.canvas.style.left = part.x + "px";
    slot.canvas.style.top = part.y + "px";
    slot.canvas.style.width = part.w + "px";
    slot.canvas.style.height = part.h + "px";
  };
  const paintWidget = (slot) => {
    const pixels = slot.pixels;
    if (!pixels || slot.w < 1 || slot.h < 1 || pixels.length !== slot.w * slot.h * 4) return;
    if (slot.canvas.width !== slot.w || slot.canvas.height !== slot.h) {
      slot.canvas.width = slot.w;
      slot.canvas.height = slot.h;
    }
    slot.ctx.putImageData(new ImageData(pixels, slot.w, slot.h), 0, 0);
  };
  const readPieces = (buffer) => {
    const bytes = new Uint8Array(buffer);
    if (bytes.length < 11 || bytes[0] !== 72 || bytes[1] !== 83 || bytes[2] !== 70 || bytes[3] !== 53) return null;
    const view = new DataView(buffer);
    const partial = (bytes[4] & 1) !== 0;
    const frameW = view.getUint16(5, true);
    const frameH = view.getUint16(7, true);
    const count = view.getUint16(9, true);
    const parts = [];
    let offset = 11;
    for (let index = 0; index < count; index++) {
      if (offset + 10 > buffer.byteLength) break;
      const id = view.getUint8(offset);
      const flags = view.getUint8(offset + 1);
      const x = view.getUint16(offset + 2, true);
      const y = view.getUint16(offset + 4, true);
      const w = view.getUint16(offset + 6, true);
      const h = view.getUint16(offset + 8, true);
      offset += 10;
      let pixels = null;
      const remove = (flags & 2) !== 0;
      if (!remove && (flags & 1)) {
        if (offset + 4 > buffer.byteLength) break;
        const length = view.getUint32(offset, true);
        offset += 4;
        if (offset + length > buffer.byteLength) break;
        pixels = new Uint8ClampedArray(buffer, offset, length);
        offset += length;
      }
      parts.push({ id: id, x: x, y: y, w: w, h: h, pixels: pixels, remove: remove });
    }
    return { paintW: frameW, paintH: frameH, partial: partial, parts: parts };
  };
  const applyMessage = (buffer) => {
    const frame = readPieces(buffer);
    if (!frame) return;
    const nextW = frame.paintW || paintW;
    const nextH = frame.paintH || paintH;
    if (nextW !== paintW || nextH !== paintH) {
      paintW = nextW;
      paintH = nextH;
      fitStage();
    }
    const seen = new Set();
    for (const part of frame.parts) {
      if (part.remove) {
        const slot = widgets.get(part.id);
        if (slot) {
          slot.canvas.remove();
          widgets.delete(part.id);
        }
        continue;
      }
      seen.add(part.id);
      const slot = ensure(part.id);
      move(slot, part);
      if (part.pixels && part.w > 0 && part.h > 0 && part.pixels.length === part.w * part.h * 4) {
        slot.pixels = new Uint8ClampedArray(part.pixels);
        paintWidget(slot);
        slot.pixels = null;
      }
    }
    if (!frame.partial) {
      for (const id of Array.from(widgets.keys())) {
        if (!seen.has(id)) {
          widgets.get(id).canvas.remove();
          widgets.delete(id);
        }
      }
    }
  };
  const connect = () => {
    const proto = location.protocol === "https:" ? "wss" : "ws";
    ws = new WebSocket(`${proto}://${location.host}/ws`);
    ws.binaryType = "arraybuffer";
    ws.onmessage = (ev) => {
      applyMessage(ev.data);
    };
    ws.onclose = () => setTimeout(connect, 1000);
    ws.onerror = () => { try { ws.close(); } catch (_) {} };
  };
  fitStage();
  connect();
})();
</script>
</body>
</html>
"#;
