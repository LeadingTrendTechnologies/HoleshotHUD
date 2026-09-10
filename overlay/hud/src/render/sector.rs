#![allow(unused_imports)]
use super::*;

pub(crate) fn sector_hero_index(s: &Snapshot) -> i32 {
    crate::sector::hero_index(s)
}

pub(crate) fn sector_live_type(live_h: f32, hero: bool) -> (f32, f32, f32) {
    let label_fs = if hero {
        (live_h * 0.18).clamp(10.0, 14.0)
    } else {
        (live_h * 0.16).clamp(9.0, 12.0)
    };
    // Settings font (`style_k`) already scales glyphs. Do not let a tall box
    // grow the live delta without a cap — that is what ran into the next column.
    let delta_fs = if hero {
        (live_h * 0.42).clamp(16.0, 28.0)
    } else {
        (live_h * 0.28).clamp(12.0, 20.0)
    };
    let time_fs = if hero {
        (live_h * 0.16).clamp(10.0, 15.0)
    } else {
        (live_h * 0.14).clamp(9.0, 12.0)
    };
    (label_fs, delta_fs, time_fs)
}

pub(crate) fn sector_probe_max(fonts: &Fonts, fs: f32, probes: &[&str]) -> f32 {
    let mut w = 0.0_f32;
    for p in probes {
        w = w.max(measure(fonts, p, fs));
    }
    w
}

pub(crate) fn sector_fit_probe(fonts: &Fonts, probes: &[&str], fs: f32, max_w: f32) -> f32 {
    if max_w <= 1.0 {
        return fs;
    }
    let w = sector_probe_max(fonts, fs, probes);
    if w <= max_w {
        fs
    } else {
        // Floor is 8px on screen. `measure` / `text` already multiply `style_k`,
        // so a raw 8.0 floor stays too big at 160% and the next column eats it.
        (fs * (max_w / w)).max(8.0 / style_k())
    }
}

/// Center the live string in the column. Probe `slot_w` still sizes pills and columns.
pub(crate) fn sector_num_x(fonts: &Fonts, s: &str, fs: f32, col_x: f32, col_w: f32, _slot_w: f32) -> f32 {
    let tw = measure(fonts, s, fs);
    col_x + (col_w - tw) * 0.5
}

pub(crate) fn draw_sector_num(
    px: &mut Pixmap,
    fonts: &Fonts,
    s: &str,
    fs: f32,
    col_x: f32,
    col_w: f32,
    slot_w: f32,
    y: f32,
    color: Color,
    glass: bool,
) {
    text_halo(
        px,
        fonts,
        s,
        fs,
        sector_num_x(fonts, s, fs, col_x, col_w, slot_w),
        y,
        color,
        false,
        glass,
    );
}

pub(crate) const SECTOR_PILL_PAD_X: f32 = 14.0;

pub(crate) const SECTOR_PILL_PAD_Y: f32 = 3.0;

/// Matches the sector plaque corner. Pills sit above this or the bottom is shaved off.
pub(crate) const SECTOR_CORNER: f32 = 6.0;

pub(crate) const SECTOR_COL_PAD: f32 = 12.0;

/// Night-ink chip around a split. Height follows Settings font (`style_k`).
pub(crate) fn sector_pill_metrics(fs: f32) -> (f32, f32) {
    let text_h = fs * style_k();
    (SECTOR_PILL_PAD_Y, text_h + SECTOR_PILL_PAD_Y * 2.0)
}

/// Fat probes so columns do not jump as live digits grow (`9.123` → `1:10.000`).
pub(crate) const SECTOR_PROBE_SPLIT: &str = "88.888";

pub(crate) const SECTOR_PROBE_SPLIT_LONG: &str = "8:88.888";

pub(crate) const SECTOR_PROBE_DELTA: &str = "+88.888";

pub(crate) const SECTOR_PROBE_DELTA_LONG: &str = "+8:88.8";

pub(crate) const SECTOR_PROBE_LAP: &str = "88:88.888";

pub(crate) const SECTOR_SPLIT_PROBES: &[&str] = &[SECTOR_PROBE_SPLIT, SECTOR_PROBE_SPLIT_LONG];

pub(crate) const SECTOR_DELTA_PROBES: &[&str] = &[SECTOR_PROBE_DELTA, SECTOR_PROBE_DELTA_LONG];

pub(crate) const SECTOR_LAP_PROBES: &[&str] = &[SECTOR_PROBE_LAP];

pub(crate) fn sector_col_need(
    fonts: &Fonts,
    s: &Snapshot,
    cfg: &HudConfig,
    live_h: f32,
    hist_fs: f32,
    show_hist: bool,
) -> [f32; 4] {
    let mut need = [0.0; 4];
    for i in 0..3 {
        let row = sector_row(s, i, cfg.sector_live, cfg.sector_session);
        let (label_fs, delta_fs, time_fs) = sector_live_type(live_h, row.fresh);
        let plaque = if row.fresh { 22.0 } else { 0.0 };
        let mut w = measure(fonts, row.label, label_fs) + plaque;
        w = w.max(measure(fonts, SECTOR_PROBE_DELTA, delta_fs));
        w = w.max(measure(fonts, SECTOR_PROBE_DELTA_LONG, delta_fs));
        w = w.max(measure(fonts, SECTOR_PROBE_SPLIT, time_fs) + SECTOR_PILL_PAD_X);
        w = w.max(measure(fonts, SECTOR_PROBE_SPLIT_LONG, time_fs) + SECTOR_PILL_PAD_X);
        if show_hist {
            w = w.max(measure(fonts, SECTOR_PROBE_SPLIT, hist_fs));
            w = w.max(measure(fonts, SECTOR_PROBE_SPLIT_LONG, hist_fs));
        }
        need[i] = w + SECTOR_COL_PAD;
    }
    let lap_label_fs = (live_h * 0.16).clamp(9.0, 12.0);
    let lap_time_fs = (live_h * 0.28).clamp(12.0, 22.0);
    let lap_delta_fs = (live_h * 0.18).clamp(10.0, 14.0);
    let pill_fs = (live_h * 0.14).clamp(9.0, 12.0);
    let mut lw = measure(fonts, "LAP", lap_label_fs);
    lw = lw.max(measure(fonts, SECTOR_PROBE_LAP, lap_time_fs));
    lw = lw.max(measure(fonts, SECTOR_PROBE_DELTA, lap_delta_fs));
    lw = lw.max(measure(fonts, SECTOR_PROBE_DELTA_LONG, lap_delta_fs));
    lw = lw.max(measure(fonts, SECTOR_PROBE_LAP, pill_fs) + SECTOR_PILL_PAD_X);
    if show_hist {
        lw = lw.max(measure(fonts, SECTOR_PROBE_LAP, hist_fs));
        lw = lw.max(measure(fonts, SECTOR_PROBE_DELTA, hist_fs));
    }
    need[3] = lw + SECTOR_COL_PAD;
    need
}

/// Give each column its measured text width. Extra space goes to the current sector, never LAP.
pub(crate) fn sector_col_widths(need: [f32; 4], cols_w: f32, hero: i32) -> [f32; 4] {
    let sum: f32 = need.iter().sum();
    if sum <= 0.0 || cols_w <= 0.0 {
        return [cols_w * 0.25; 4];
    }
    if sum >= cols_w {
        return need.map(|n| n * cols_w / sum);
    }
    let mut w = need;
    let extra = cols_w - sum;
    if (0..=2).contains(&hero) {
        w[hero as usize] += extra;
    } else {
        let share = extra / 3.0;
        for slot in w.iter_mut().take(3) {
            *slot += share;
        }
    }
    w
}

pub(crate) fn draw_sector(px: &mut Pixmap, fonts: &Fonts, s: &Snapshot, cfg: &HudConfig, sw: f32, sh: f32) {
    // THESIS: live sector delta stays the glance; LAP hangs the whole lap beside it; last laps share those columns.
    // OWN-WORLD: night-ink 6px plaque, orange skew S# on the current cell, you-row gold on the fastest log lap, green/red times vs best. No purple, no drop-shadow. Glass (bg < 40) rims floating type in night-ink.
    // STORY: rider reads up/down vs best on top, drops their eyes for LAST / -2 / -3 times and the lap total.
    // FIRST VIEWPORT: live strip on top (hero ~46%, LAP ~22% stacked time/delta); hairline; LAST / -2 / -3 aligned under the same four columns. Short boxes stay live-only.
    // FORM: Underboard · approved .impeccable/mocks/sector-lap-column.png
    // FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
    let r = cfg[WidgetId::Sector].rect;
    let x = r.x * sw;
    let y = r.y * sh;
    let w = r.w * sw;
    let h = r.h * sh;
    if w < 88.0 || h < 44.0 {
        return;
    }
    let a = bg_a(cfg[WidgetId::Sector].bg);
    let glass = cfg[WidgetId::Sector].bg < 40;
    let n = 3usize;
    let pad_x = (w * 0.03).clamp(6.0, 12.0);
    let pad_y = (h * 0.06).clamp(5.0, 10.0);
    if a > 0 {
        fill_round(px, x, y, w, h, 6.0, Color::from_rgba8(10, 10, 10, a));
        let rad = 6.0f32.min(w * 0.5).min(h * 0.5);
        let mut pb = PathBuilder::new();
        pb.move_to(x + rad, y);
        pb.line_to(x + w - rad, y);
        pb.quad_to(x + w, y, x + w, y + rad);
        pb.line_to(x + w, y + h - rad);
        pb.quad_to(x + w, y + h, x + w - rad, y + h);
        pb.line_to(x + rad, y + h);
        pb.quad_to(x, y + h, x, y + h - rad);
        pb.line_to(x, y + rad);
        pb.quad_to(x, y, x + rad, y);
        pb.close();
        if let Some(path) = pb.finish() {
            let edge = ((a as u16 * 70) / 255).max(36) as u8;
            stroke_path(px, &path, Color::from_rgba8(220, 220, 224, edge), 1.0);
        }
    }
    let inner_w = (w - pad_x * 2.0).max(60.0);
    let cap = if cfg.sector_session {
        "vs. session best"
    } else {
        "vs. your best"
    };
    let cap_fs = sector_fit_probe(
        fonts,
        &[cap],
        (h * 0.11).clamp(11.0, 16.0),
        (inner_w * 0.92).max(40.0),
    );
    let cap_y = y + (pad_y * 0.25).max(2.0);
    text_halo(px, fonts, cap, cap_fs, x + w * 0.5, cap_y, text_dim(), true, glass);
    // Caption glyphs are `cap_fs * style_k`. Spacing from the raw size left
    // "vs. your best" sitting on S2/S3 when Settings font went up.
    let body_y = cap_y + cap_fs * style_k() + 4.0;
    let want = if cfg.sector_hist {
        cfg.sector_hist_count()
    } else {
        0
    };
    let hist = if want > 0 {
        crate::sector::history_board(s, cfg.sector_session, want)
    } else {
        Vec::new()
    };
    let has_hist = hist.first().is_some_and(|r| r.cells.iter().any(|c| c.time_ms > 0));
    let ideal = crate::sector::ideal(s, cfg.sector_session);
    let k = style_k();
    // LAST stacks lap time over delta. Row height must follow the scaled type
    // or 1:24.171 / +5.316 land on the next sector time.
    let hist_row_h = ((h * 0.13).clamp(22.0, 32.0) * k).min(h * 0.20).max(22.0);
    let live_min = 52.0 * k.max(1.0);
    let avail = (y + h - pad_y - body_y - live_min).max(0.0);
    let fit = (avail / hist_row_h).floor() as usize;
    let (n_hist, show_ideal) = if has_hist {
        if ideal.ready() && fit >= 2 {
            let n = want.min(fit - 1).min(hist.len());
            (n, n >= 1)
        } else {
            (want.min(fit).min(hist.len()), false)
        }
    } else {
        (0, false)
    };
    let show_hist = n_hist >= 1;
    let n_rows = n_hist + usize::from(show_ideal);
    let hist_band = if show_hist {
        hist_row_h * n_rows as f32 + 5.0
    } else {
        0.0
    };
    let live_bottom = if show_hist {
        y + h - pad_y - hist_band
    } else {
        y + h - pad_y.max(SECTOR_CORNER + 3.0)
    };
    let live_h = (live_bottom - body_y).max(8.0);
    let hist_fs = (hist_row_h * 0.48).clamp(10.0, 14.0);
    let hist_delta_fs = (hist_row_h * 0.38).clamp(8.0, 11.0);
    let hist_label_fs = (hist_row_h * 0.42).clamp(8.0, 11.0);
    let gutter = 36.0_f32.min(inner_w * 0.12);
    let cols_w = (inner_w - gutter).max(40.0);
    let need = sector_col_need(fonts, s, cfg, live_h, hist_fs, show_hist);
    let widths = sector_col_widths(need, cols_w, sector_hero_index(s));
    let mut col_x = [0.0; 4];
    let mut col_w = [0.0; 4];
    let mut acc = x + pad_x + gutter;
    for i in 0..4 {
        col_x[i] = acc;
        col_w[i] = widths[i];
        acc += col_w[i];
    }
    for i in 1..4 {
        if let Some(line) = rr(col_x[i], body_y, 1.0, live_h) {
            fill_rect(px, line, Color::from_rgba8(42, 42, 46, a.max(90)));
        }
    }
    let ink = Color::from_rgba8(12, 12, 14, 255);
    for i in 0..n {
        let row = sector_row(s, i, cfg.sector_live, cfg.sector_session);
        let cx = col_x[i];
        let cw = col_w[i];
        let hero = row.fresh;
        if hero {
            if let Some(wash) = rr(cx + 1.0, body_y, (cw - 2.0).max(4.0), live_h) {
                fill_rect(px, wash, Color::from_rgba8(255, 148, 48, 28));
            }
        }
        let delta_c = if row.pending || !row.has_delta {
            text_dim()
        } else if row.slower {
            behind_col()
        } else {
            ahead_col()
        };
        let time_c = if hero { text_col() } else { text_dim() };
        let (label_fs, mut delta_fs, mut time_fs) = sector_live_type(live_h, hero);
        let text_max = (cw - 8.0).max(8.0);
        delta_fs = sector_fit_probe(fonts, SECTOR_DELTA_PROBES, delta_fs, text_max);
        time_fs = sector_fit_probe(
            fonts,
            SECTOR_SPLIT_PROBES,
            time_fs,
            (text_max - SECTOR_PILL_PAD_X).max(8.0),
        );
        let mid = cx + cw * 0.5;
        let label_y = body_y;
        if hero {
            let plaque_h = (label_fs + 8.0).min(live_h * 0.32);
            let plaque_w = (measure(fonts, row.label, label_fs) + 16.0).min(cw - 12.0);
            let skew = (plaque_h * 0.18).clamp(3.0, 6.0);
            let px0 = cx + 8.0;
            fill_skew(px, px0, label_y, plaque_w, plaque_h, skew, accent());
            text(
                px,
                fonts,
                row.label,
                label_fs,
                px0 + plaque_w * 0.5 + skew * 0.5,
                label_y + (plaque_h - label_fs) * 0.42,
                ink,
                true,
            );
        } else {
            text_halo(px, fonts, row.label, label_fs, mid, label_y, text_dim(), true, glass);
        }
        let delta_y = body_y + (live_h - delta_fs) * 0.38;
        let delta_slot = sector_probe_max(fonts, delta_fs, SECTOR_DELTA_PROBES);
        draw_sector_num(
            px,
            fonts,
            &row.delta,
            delta_fs,
            cx,
            cw,
            delta_slot,
            delta_y,
            delta_c,
            glass,
        );
        let (pill_pad, pill_h) = sector_pill_metrics(time_fs);
        let time_y = live_bottom - pill_h + pill_pad;
        let time_slot = sector_probe_max(fonts, time_fs, SECTOR_SPLIT_PROBES);
        let chip_x = 7.0;
        let pill_w = time_slot + chip_x * 2.0;
        fill_night_pill(
            px,
            mid - pill_w * 0.5,
            time_y - pill_pad,
            pill_w,
            pill_h,
        );
        draw_sector_num(
            px,
            fonts,
            &row.time,
            time_fs,
            cx,
            cw,
            time_slot,
            time_y,
            time_c,
            false,
        );
    }
    draw_sector_lap_live(
        px,
        fonts,
        s,
        cfg,
        col_x[3],
        col_w[3],
        body_y,
        live_h,
        live_bottom,
        ideal,
        glass,
    );
    if !show_hist {
        return;
    }
    if let Some(rule) = rr(x + pad_x, live_bottom, inner_w, 1.0) {
        fill_rect(px, rule, Color::from_rgba8(42, 42, 46, a.max(90)));
    }
    let mut row_i = 0usize;
    if show_ideal {
        let ry = live_bottom + 4.0;
        draw_sector_ideal_row(
            px,
            fonts,
            &ideal,
            x + pad_x,
            ry,
            hist_row_h,
            hist_fs,
            hist_label_fs,
            &col_x,
            &col_w,
            glass,
        );
        row_i = 1;
    }
    for (ri, row) in hist.iter().take(n_hist).enumerate() {
        let ry = live_bottom + 4.0 + (row_i + ri) as f32 * hist_row_h;
        if row.fastest {
            if let Some(wash) = rr(x + pad_x, ry, inner_w, hist_row_h - 1.0) {
                fill_rect(px, wash, you_row_bg(72));
            }
        }
        text_halo(
            px,
            fonts,
            row.label,
            hist_label_fs,
            x + pad_x + 2.0,
            ry + (hist_row_h - hist_label_fs) * 0.35,
            if row.fastest { text_col() } else { text_dim() },
            false,
            glass,
        );
        for i in 0..n {
            let cell = row.cells[i];
            let label = if cell.time_ms > 0 {
                format_lap(cell.time_ms)
            } else {
                "--".into()
            };
            let col = if cell.faster {
                ahead_col()
            } else if cell.slower {
                behind_col()
            } else if row.fastest {
                text_col()
            } else {
                text_dim()
            };
            let cell_fs = sector_fit_probe(fonts, SECTOR_SPLIT_PROBES, hist_fs, (col_w[i] - 8.0).max(8.0));
            let slot = sector_probe_max(fonts, cell_fs, SECTOR_SPLIT_PROBES);
            draw_sector_num(
                px,
                fonts,
                &label,
                cell_fs,
                col_x[i],
                col_w[i],
                slot,
                ry + (hist_row_h - cell_fs) * 0.18,
                col,
                glass,
            );
        }
        let lap_time = if row.lap_ms > 0 {
            format_lap(row.lap_ms)
        } else {
            "--".into()
        };
        let lap_delta = if row.has_lap_delta {
            format_delta_ms(row.lap_delta_ms)
        } else {
            "--".into()
        };
        let lap_time_c = if row.fastest { text_col() } else { text_dim() };
        let lap_delta_c = if !row.has_lap_delta {
            text_dim()
        } else if row.lap_slower {
            behind_col()
        } else if row.lap_faster {
            ahead_col()
        } else {
            lap_time_c
        };
        let lap_max = (col_w[3] - 8.0).max(8.0);
        let lap_fs = sector_fit_probe(fonts, SECTOR_LAP_PROBES, hist_fs, lap_max);
        let lap_dfs = sector_fit_probe(fonts, SECTOR_DELTA_PROBES, hist_delta_fs, lap_max);
        let lap_slot = sector_probe_max(fonts, lap_fs, SECTOR_LAP_PROBES);
        let d_slot = sector_probe_max(fonts, lap_dfs, SECTOR_DELTA_PROBES);
        draw_sector_num(
            px,
            fonts,
            &lap_time,
            lap_fs,
            col_x[3],
            col_w[3],
            lap_slot,
            ry + 1.0,
            lap_time_c,
            glass,
        );
        draw_sector_num(
            px,
            fonts,
            &lap_delta,
            lap_dfs,
            col_x[3],
            col_w[3],
            d_slot,
            ry + lap_fs + 2.0,
            lap_delta_c,
            glass,
        );
    }
}

pub(crate) fn draw_sector_lap_live(
    px: &mut Pixmap,
    fonts: &Fonts,
    s: &Snapshot,
    cfg: &HudConfig,
    cx: f32,
    cw: f32,
    body_y: f32,
    live_h: f32,
    live_bottom: f32,
    ideal: crate::sector::IdealLap,
    glass: bool,
) {
    let lap = crate::sector::live_lap(s, cfg.sector_session);
    let mid = cx + cw * 0.5;
    let label_fs = (live_h * 0.16).clamp(9.0, 12.0);
    text_halo(px, fonts, "LAP", label_fs, mid, body_y, text_dim(), true, glass);
    let time = if lap.time_ms > 0 {
        format_lap(lap.time_ms)
    } else {
        "--".into()
    };
    let delta = if lap.pending || !lap.has_delta {
        "--".into()
    } else {
        format_delta_ms(lap.delta_ms)
    };
    let pill = if ideal.ready() {
        format_lap(ideal.lap_ms)
    } else {
        "--".into()
    };
    let text_max = (cw - 8.0).max(8.0);
    let time_fs = sector_fit_probe(fonts, SECTOR_LAP_PROBES, (live_h * 0.28).clamp(12.0, 22.0), text_max);
    let delta_fs = sector_fit_probe(
        fonts,
        SECTOR_DELTA_PROBES,
        (live_h * 0.18).clamp(10.0, 14.0),
        text_max,
    );
    let pill_fs = sector_fit_probe(
        fonts,
        SECTOR_LAP_PROBES,
        (live_h * 0.14).clamp(9.0, 12.0),
        (text_max - SECTOR_PILL_PAD_X).max(8.0),
    );
    let time_c = text_col();
    let delta_c = if lap.pending || !lap.has_delta {
        text_dim()
    } else if lap.slower {
        behind_col()
    } else {
        ahead_col()
    };
    let pill_band = sector_pill_metrics(pill_fs).1 + 2.0;
    let stack_h = time_fs + delta_fs + 2.0;
    let time_y = body_y + ((live_h - pill_band - stack_h).max(0.0)) * 0.42;
    let time_slot = sector_probe_max(fonts, time_fs, SECTOR_LAP_PROBES);
    let delta_slot = sector_probe_max(fonts, delta_fs, SECTOR_DELTA_PROBES);
    draw_sector_num(px, fonts, &time, time_fs, cx, cw, time_slot, time_y, time_c, glass);
    draw_sector_num(
        px,
        fonts,
        &delta,
        delta_fs,
        cx,
        cw,
        delta_slot,
        time_y + time_fs + 1.0,
        delta_c,
        glass,
    );
    let (pill_pad, pill_h) = sector_pill_metrics(pill_fs);
    let time_y_pill = live_bottom - pill_h + pill_pad;
    let pill_slot = sector_probe_max(fonts, pill_fs, SECTOR_LAP_PROBES);
    let pill_w = pill_slot + SECTOR_PILL_PAD_X;
    fill_night_pill(
        px,
        mid - pill_w * 0.5,
        time_y_pill - pill_pad,
        pill_w,
        pill_h,
    );
    draw_sector_num(
        px,
        fonts,
        &pill,
        pill_fs,
        cx,
        cw,
        pill_slot,
        time_y_pill,
        text_dim(),
        false,
    );
}

pub(crate) fn draw_sector_ideal_row(
    px: &mut Pixmap,
    fonts: &Fonts,
    ideal: &crate::sector::IdealLap,
    label_x: f32,
    ry: f32,
    row_h: f32,
    hist_fs: f32,
    hist_label_fs: f32,
    col_x: &[f32; 4],
    col_w: &[f32; 4],
    glass: bool,
) {
    text_halo(
        px,
        fonts,
        "IDEAL",
        hist_label_fs,
        label_x + 2.0,
        ry + (row_h - hist_label_fs) * 0.35,
        text_dim(),
        false,
        glass,
    );
    for i in 0..3 {
        let t = ideal.sectors[i];
        let label = if t > 0 {
            format_lap(t)
        } else {
            "--".into()
        };
        let cell_fs = sector_fit_probe(fonts, SECTOR_SPLIT_PROBES, hist_fs, (col_w[i] - 8.0).max(8.0));
        let slot = sector_probe_max(fonts, cell_fs, SECTOR_SPLIT_PROBES);
        draw_sector_num(
            px,
            fonts,
            &label,
            cell_fs,
            col_x[i],
            col_w[i],
            slot,
            ry + (row_h - hist_fs) * 0.32,
            text_col(),
            glass,
        );
    }
    let lap = if ideal.ready() {
        format_lap(ideal.lap_ms)
    } else {
        "--".into()
    };
    let lap_fs = sector_fit_probe(fonts, SECTOR_LAP_PROBES, hist_fs, (col_w[3] - 8.0).max(8.0));
    let lap_slot = sector_probe_max(fonts, lap_fs, SECTOR_LAP_PROBES);
    draw_sector_num(
        px,
        fonts,
        &lap,
        lap_fs,
        col_x[3],
        col_w[3],
        lap_slot,
        ry + (row_h - hist_fs) * 0.32,
        text_col(),
        glass,
    );
}

pub(crate) struct SectorRowView {
    pub(crate) label: &'static str,
    pub(crate) time: String,
    pub(crate) delta: String,
    pub(crate) pending: bool,
    pub(crate) has_delta: bool,
    pub(crate) slower: bool,
    pub(crate) fresh: bool,
}

pub(crate) fn sector_row(s: &Snapshot, i: usize, live: bool, session: bool) -> SectorRowView {
    let src = crate::sector::row_vs(s, i, live, session);
    let showing = src.time_ms > 0;
    SectorRowView {
        label: src.label,
        time: if showing {
            format_lap(src.time_ms)
        } else {
            "--".into()
        },
        delta: if src.pending || !src.has_delta {
            "--".into()
        } else {
            format_delta_ms(src.delta_ms)
        },
        pending: src.pending,
        has_delta: src.has_delta,
        slower: src.slower,
        fresh: src.fresh,
    }
}
