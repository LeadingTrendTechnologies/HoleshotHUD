#![allow(unused_imports)]
use super::*;

pub(crate) static SYS_CPU: AtomicI32 = AtomicI32::new(0);

pub(crate) static SYS_MEM: AtomicI32 = AtomicI32::new(0);

pub(crate) static SYS_FPS: AtomicI32 = AtomicI32::new(0);

pub(crate) static SYS_GPU: AtomicI32 = AtomicI32::new(0);

pub(crate) static SYS_PING: AtomicI32 = AtomicI32::new(-1);

#[derive(Clone, Default)]
pub struct SysProc {
    pub label: String,
    pub cpu: f32,
    pub gpu: f32,
    pub mem_mb: f32,
    pub mem_pct: f32,
    pub on: bool,
}

pub fn set_sys_stats(cpu: f32, mem: f32, fps: f32, gpu: f32, ping_ms: i32) {
    SYS_CPU.store((cpu.clamp(0.0, 100.0) * 10.0).round() as i32, Ordering::Relaxed);
    SYS_MEM.store((mem.clamp(0.0, 100.0) * 10.0).round() as i32, Ordering::Relaxed);
    SYS_FPS.store((fps.clamp(0.0, 999.0) * 10.0).round() as i32, Ordering::Relaxed);
    SYS_GPU.store((gpu.clamp(0.0, 100.0) * 10.0).round() as i32, Ordering::Relaxed);
    SYS_PING.store(if ping_ms < 0 { -1 } else { ping_ms.clamp(0, 9999) }, Ordering::Relaxed);
}

pub fn set_sys_procs(procs: Vec<SysProc>) {
    let mut procs = procs;
    procs.truncate(SYS_PROC_MAX);
    if let Ok(mut g) = sys_procs_lock().lock() {
        *g = procs;
    }
}

pub(crate) fn sys_stats() -> (f32, f32, f32, f32, i32) {
    (
        SYS_CPU.load(Ordering::Relaxed) as f32 / 10.0,
        SYS_MEM.load(Ordering::Relaxed) as f32 / 10.0,
        SYS_FPS.load(Ordering::Relaxed) as f32 / 10.0,
        SYS_GPU.load(Ordering::Relaxed) as f32 / 10.0,
        SYS_PING.load(Ordering::Relaxed),
    )
}

pub(crate) fn sys_procs_lock() -> &'static Mutex<Vec<SysProc>> {
    static SLOT: OnceLock<Mutex<Vec<SysProc>>> = OnceLock::new();
    SLOT.get_or_init(|| {
        Mutex::new(
            ["HUD", "MX Bikes", "MXB App", "ReShade"]
                .into_iter()
                .map(|label| SysProc {
                    label: label.into(),
                    ..Default::default()
                })
                .collect(),
        )
    })
}

pub(crate) fn sys_procs() -> Vec<SysProc> {
    sys_procs_lock()
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default()
}

pub(crate) fn fmt_sys_mem(mb: f32) -> String {
    if mb < 0.05 {
        "0 MB".into()
    } else if mb < 9.95 {
        format!("{mb:.1} MB")
    } else if mb < 1024.0 {
        format!("{:.0} MB", mb.round())
    } else {
        let gb = mb / 1024.0;
        if gb < 9.95 {
            format!("{gb:.1} GB")
        } else {
            format!("{:.0} GB", gb.round())
        }
    }
}

pub(crate) fn sys_heat(hot: f32) -> Color {
    if hot >= 90.0 {
        Color::from_rgba8(239, 68, 68, 230)
    } else {
        Color::from_rgba8(250, 180, 48, 230)
    }
}

pub(crate) fn draw_sys_track(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, fill: f32, col: Color) {
    let w = w.max(8.0);
    let h = h.clamp(2.0, 6.0);
    fill_round(px, x, y, w, h, 1.5, Color::from_rgba8(42, 42, 46, 160));
    let fw = (w * (fill / 100.0).clamp(0.0, 1.0)).max(if fill > 0.5 { h } else { 0.0 });
    if fw > 0.5 {
        fill_round(px, x, y, fw, h, 1.5, col);
    }
}

pub(crate) fn draw_sys_meter(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    value: &str,
    label_fs: f32,
    value_fs: f32,
    fill: f32,
    col: Color,
    track_h: f32,
) -> f32 {
    text(px, fonts, label, label_fs, x, y, text_dim(), false);
    let vy = y + label_fs * 1.12;
    text(px, fonts, value, value_fs, x, vy, Color::from_rgba8(248, 248, 252, 255), false);
    let ty = vy + value_fs * 1.08;
    if track_h > 0.5 {
        draw_sys_track(px, x, ty, w, track_h, fill, col);
        ty + track_h
    } else {
        vy + value_fs
    }
}

#[derive(Clone, Copy)]
pub(crate) enum SysProcKind {
    Cpu,
    Mem,
    Gpu,
}

pub(crate) fn draw_sys_procs(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    row_h: f32,
    fs: f32,
    label_w: f32,
    procs: &[SysProc],
    kind: SysProcKind,
    mem_scale: f32,
) {
    let mute = Color::from_rgba8(132, 132, 138, 255);
    let bar = Color::from_rgba8(150, 150, 158, 110);
    let bar_dim = Color::from_rgba8(150, 150, 158, 40);
    for (i, p) in procs.iter().enumerate() {
        let ry = y + row_h * i as f32;
        let (value, fill, dim) = match kind {
            SysProcKind::Mem => {
                if p.on {
                    (fmt_sys_mem(p.mem_mb), (p.mem_mb / mem_scale * 100.0).clamp(0.0, 100.0), false)
                } else {
                    ("—".into(), 0.0, true)
                }
            }
            SysProcKind::Cpu | SysProcKind::Gpu => {
                let load = match kind {
                    SysProcKind::Gpu => p.gpu,
                    _ => p.cpu,
                };
                let known = p.on && load >= 0.0;
                if known {
                    (format!("{:.0}%", load.round()), load.clamp(0.0, 100.0), false)
                } else {
                    ("—".into(), 0.0, true)
                }
            }
        };
        let ink = if dim { Color::from_rgba8(108, 108, 114, 220) } else { mute };
        text(px, fonts, &p.label, fs, x, ry + (row_h - fs) * 0.22, ink, false);
        let val_w = measure(fonts, &value, fs);
        let bx = x + label_w + 4.0;
        let bw = (x + w - val_w - 5.0 - bx).max(12.0);
        let bh = (row_h * 0.28).clamp(2.0, 4.0);
        let by = ry + (row_h - bh) * 0.42;
        draw_sys_track(px, bx, by, bw, bh, fill, if dim { bar_dim } else { bar });
        text(px, fonts, &value, fs, x + w - val_w, ry + (row_h - fs) * 0.22, ink, false);
    }
}

pub(crate) fn draw_sys(px: &mut Pixmap, fonts: &Fonts, cfg: &HudConfig, sw: f32, sh: f32) {
    // THESIS: CPU and MEM are two boards so process load is a glance, not a 12-row scroll. GPU repeats that list in the footer.
    // OWN-WORLD: night-ink 6px plaque, hairline split, huge ExtraBold Italic numbers, gold heat tracks (red when hot). No green, no orange.
    // STORY: rider glances — is the machine the problem, and is it the HUD or the game.
    // FIRST VIEWPORT: tall plaque, CPU left / MEM right, watched apps under each, FPS + ping and GPU in a footer, GPU with the same apps.
    // FORM: Twin Columns, user-locked sys-twin.png.
    // FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
    let r = cfg[WidgetId::Sys].rect;
    let x = r.x * sw;
    let y = r.y * sh;
    let w = r.w * sw;
    let h = r.h * sh;
    if w < 88.0 || h < 72.0 {
        return;
    }
    let a = bg_a(cfg[WidgetId::Sys].bg);
    fill_round(px, x, y, w, h, 6.0, Color::from_rgba8(10, 10, 10, a));
    if a > 0 {
        if let Some(frame) = round_rect_path(x + 0.5, y + 0.5, w - 1.0, h - 1.0, 5.5) {
            let edge = ((a as u16 * 170) / 255).max(70) as u8;
            stroke_path(px, &frame, Color::from_rgba8(42, 42, 46, edge), 1.0);
        }
    }

    let (cpu, mem, fps, gpu, ping) = sys_stats();
    let procs = sys_procs();
    let mem_scale = procs
        .iter()
        .filter(|p| p.on)
        .map(|p| p.mem_mb)
        .fold(0.0f32, f32::max)
        .max(1.0);

    let pad = (w * 0.045).clamp(8.0, 12.0);
    let gap = 1.0;
    let split_pad = (w * 0.055).clamp(10.0, 16.0);
    let inner_w = (w - pad * 2.0).max(40.0);
    let inner_h = (h - pad * 2.0).max(56.0);
    let col_w = ((inner_w - gap) * 0.5).max(36.0);
    let foot_h = (inner_h * 0.50).clamp(64.0, 120.0).min(inner_h - 52.0);
    let body_h = (inner_h - foot_h - 1.0).max(40.0);
    let mid = x + pad + col_w + gap * 0.5;
    let hair = Color::from_rgba8(42, 42, 46, a.max(90));
    if let Some(line) = rr(mid.floor(), y + pad, 1.0, body_h + foot_h + 1.0) {
        fill_rect(px, line, hair);
    }
    let foot_y = y + pad + body_h;
    if let Some(line) = rr(x + pad, foot_y.floor(), inner_w, 1.0) {
        fill_rect(px, line, hair);
    }

    let label_fs = (body_h * 0.09).clamp(8.0, 11.0);
    let value_fs = (body_h * 0.22).clamp(18.0, 34.0);
    let proc_fs = (body_h * 0.075).clamp(8.0, 11.0);
    let track_h = (body_h * 0.028).clamp(3.0, 5.0);
    let cpu_s = format!("{:.0}%", cpu.round());
    let mem_s = format!("{:.0}%", mem.round());
    let fps_s = format!("{:.0}", fps.round());
    let gpu_s = format!("{:.0}%", gpu.round());
    let ping_s = if ping < 0 {
        "—".into()
    } else {
        format!("{ping}")
    };

    let left_x = x + pad;
    let left_w = (col_w - split_pad).max(28.0);
    let right_x = mid + 1.0 + split_pad;
    let right_w = (x + pad + inner_w - right_x).max(28.0);
    let head_y = y + pad * 0.35 + 4.0;
    let after_l = draw_sys_meter(
        px,
        fonts,
        left_x,
        head_y,
        left_w,
        "CPU",
        &cpu_s,
        label_fs,
        value_fs,
        cpu.clamp(0.0, 100.0),
        sys_heat(cpu),
        track_h,
    );
    let after_r = draw_sys_meter(
        px,
        fonts,
        right_x,
        head_y,
        right_w,
        "MEM",
        &mem_s,
        label_fs,
        value_fs,
        mem.clamp(0.0, 100.0),
        sys_heat(mem),
        track_h,
    );
    let proc_top = after_l.max(after_r) + (body_h * 0.04).clamp(4.0, 8.0);
    let proc_bot = y + pad + body_h - 4.0;
    let proc_h = (proc_bot - proc_top).max(28.0);
    let n = procs.len().max(1) as f32;
    let row_h = proc_h / n;
    let label_w = procs
        .iter()
        .map(|p| measure(fonts, &p.label, proc_fs))
        .fold(0.0f32, f32::max)
        + 4.0;
    if !procs.is_empty() {
        draw_sys_procs(
            px, fonts, left_x, proc_top, left_w, row_h, proc_fs, label_w, &procs, SysProcKind::Cpu, 1.0,
        );
        draw_sys_procs(
            px, fonts, right_x, proc_top, right_w, row_h, proc_fs, label_w, &procs, SysProcKind::Mem, mem_scale,
        );
    }

    let foot_label = (foot_h * 0.10).clamp(8.0, 10.0);
    let foot_value = (foot_h * 0.16).clamp(13.0, 18.0);
    let foot_track = 3.0;
    let fy = foot_y + 3.0;
    let foot_bot = y + pad + inner_h - 2.0;
    let fp_slot = ((foot_bot - fy) * 0.5).max(20.0);
    let fp_label = (fp_slot * 0.16).clamp(8.0, 14.0);
    let fp_cap = (fp_slot * 0.72).clamp(16.0, 48.0);
    let fit_fp = |s: &str| {
        let w = measure(fonts, s, fp_cap);
        if w > left_w && w > 1.0 {
            (fp_cap * left_w / w).max(13.0)
        } else {
            fp_cap
        }
    };
    let fps_value = fit_fp(&fps_s);
    let ping_value = fit_fp(&ping_s);
    draw_sys_meter(
        px,
        fonts,
        left_x,
        fy,
        left_w,
        "FPS",
        &fps_s,
        fp_label,
        fps_value,
        0.0,
        Color::TRANSPARENT,
        0.0,
    );
    let ping_y = fy + fp_slot;
    draw_sys_meter(
        px,
        fonts,
        left_x,
        ping_y,
        left_w,
        "PING",
        &ping_s,
        fp_label,
        ping_value,
        0.0,
        Color::TRANSPARENT,
        0.0,
    );
    if ping >= 0 {
        let ms_fs = (ping_value * 0.36).clamp(7.0, 12.0);
        let num_w = measure(fonts, &ping_s, ping_value);
        let ms_w = measure(fonts, "ms", ms_fs);
        let ms_x = left_x + num_w + ms_fs * 0.22;
        if ms_x + ms_w <= left_x + left_w + 2.0 {
            text(
                px,
                fonts,
                "ms",
                ms_fs,
                ms_x,
                ping_y + fp_label * 1.12 + ping_value - ms_fs,
                text_dim(),
                false,
            );
        }
    }
    let after_g = draw_sys_meter(
        px,
        fonts,
        right_x,
        fy,
        right_w,
        "GPU",
        &gpu_s,
        foot_label,
        foot_value,
        gpu.clamp(0.0, 100.0),
        sys_heat(gpu),
        foot_track,
    );
    let gpu_proc_top = after_g + 4.0;
    let gpu_proc_bot = y + pad + inner_h - 2.0;
    let gpu_proc_h = gpu_proc_bot - gpu_proc_top;
    if gpu_proc_h >= 32.0 && !procs.is_empty() {
        let gpu_row = gpu_proc_h / procs.len() as f32;
        let gpu_fs = (gpu_row * 0.55).clamp(8.0, proc_fs);
        let gpu_label_w = procs
            .iter()
            .map(|p| measure(fonts, &p.label, gpu_fs))
            .fold(0.0f32, f32::max)
            + 4.0;
        draw_sys_procs(
            px,
            fonts,
            right_x,
            gpu_proc_top,
            right_w,
            gpu_row,
            gpu_fs,
            gpu_label_w,
            &procs,
            SysProcKind::Gpu,
            1.0,
        );
    }
}
