#![allow(unused_imports)]
use super::*;

pub(crate) fn dash_pos_col() -> Color { Color::from_rgba8(232, 120, 23, 255) }

/// Amber for the lapped tag: reads as a warning without competing with the orange position.
pub(crate) fn dash_lapped_col() -> Color { Color::from_rgba8(226, 186, 74, 255) }

pub(crate) struct DashLay {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) w: f32,
    pub(crate) h: f32,
    pub(crate) flag: DashFlag,
    pub(crate) flag_h: f32,
    pub(crate) flag_grow: f32,
    pub(crate) pad: f32,
    pub(crate) foot_pad: f32,
    pub(crate) footer_h: f32,
    pub(crate) rev_x: f32,
    pub(crate) rev_y: f32,
    pub(crate) rev_w: f32,
    pub(crate) rev_h: f32,
    pub(crate) gear_x: f32,
    pub(crate) gear_w: f32,
    pub(crate) main_y: f32,
    pub(crate) main_h: f32,
    pub(crate) mid_x: f32,
    pub(crate) mid_w: f32,
    pub(crate) right_x: f32,
    pub(crate) label: f32,
    pub(crate) gear_n: f32,
    pub(crate) val: f32,
    pub(crate) pos_n: f32,
    pub(crate) lap_sz: f32,
    pub(crate) icon_s: f32,
    pub(crate) fsz: f32,
    pub(crate) cut: f32,
    pub(crate) simple: bool,
    pub(crate) gear: String,
    pub(crate) rpm: String,
    pub(crate) speed: String,
    pub(crate) speed_label: &'static str,
    pub(crate) ptxt: String,
    pub(crate) lap_txt: String,
    pub(crate) lapped: bool,
    pub(crate) lead: bool,
    pub(crate) tag_sz: f32,
    pub(crate) foot: Vec<(char, String)>,
}

/// Shown beside the lap/clock text when you are a lap or more down.
pub(crate) const LAPPED_TAG: &str = "~Lapped";

pub(crate) const LAPPED_GAP: f32 = 6.0;

pub(crate) fn dash_layout(fonts: &Fonts, s: &Snapshot, cfg: &HudConfig, sw: f32, sh: f32, flag: DashFlag, grow: f32) -> DashLay {
    let x0 = cfg[WidgetId::Dash].rect.x * sw;
    let y = cfg[WidgetId::Dash].rect.y * sh;
    if cfg.dash_simple {
        return dash_layout_simple(fonts, s, cfg, sw, sh, x0, y, flag, grow);
    }

    let gear = if s.local_gear <= 0 {
        "N".into()
    } else {
        format!("{}", s.local_gear)
    };
    let speed = cfg.units.format_speed(s.local_speed);
    let speed_label = cfg.units.speed_label();
    let rpm_n = s.local_rpm.max(0);
    let rpm = format!("{rpm_n}");
    let focus_num = if s.focus_race_num > 0 {
        s.focus_race_num
    } else {
        s.local_race_num
    };
    let pos = Some(standing_pos(s, focus_num)).filter(|p| *p > 0);
    let lead = pos == Some(1);
    let ptxt = pos.map(|p| format!("P{p}")).unwrap_or_else(|| "P--".into());
    let lap_txt = race_progress_text(s);
    let lapped = lapped(s);
    let foot: Vec<(char, String)> = [cfg.dash_left, cfg.dash_mid, cfg.dash_right]
        .into_iter()
        .filter_map(|field| dash_foot_item(s, cfg, field))
        .collect();

    const NAT_H: f32 = 147.0;
    const NAT_W: f32 = 280.0;
    let slot_w = (cfg[WidgetId::Dash].rect.w * sw).max(1.0);
    let slot_h = (cfg[WidgetId::Dash].rect.h * sh).max(1.0);
    let k = (slot_h / NAT_H).min(slot_w / NAT_W).clamp(0.35, 2.4);
    let pad = 15.0 * k;
    let foot_pad = 5.0 * k;
    let has_foot = !foot.is_empty();
    let footer_h = if has_foot { 23.0 * k } else { 0.0 };
    let mid_gap = if has_foot { 5.0 * k } else { 0.0 };
    let (rev_h, rev_lead, rev_to_main) = if cfg.dash_rev {
        (16.0 * k, 5.0 * k, 9.0 * k)
    } else {
        (0.0, pad, 0.0)
    };
    let h = slot_h;
    let w = slot_w;
    let x = x0;
    let chrome = rev_lead + rev_h + rev_to_main + mid_gap + footer_h + foot_pad;
    let main_h = (h - chrome).max(28.0 * k);
    let flag_full = if flag != DashFlag::None {
        (h * 0.22).clamp(20.0 * k, 28.0 * k)
    } else {
        0.0
    };
    let flag_h = flag_full * grow;
    let rev_y = y + rev_lead;
    let main_y = if cfg.dash_rev {
        rev_y + rev_h + rev_to_main
    } else {
        y + pad
    };
    let label = 14.0 * k;
    let gear_n = (main_h * 0.58).clamp(24.0 * k, 48.0 * k);
    let val = (main_h * 0.26).clamp(15.0 * k, 22.0 * k);
    let pos_n = (main_h * 0.44).clamp(22.0 * k, 38.0 * k);
    let lap_sz = (val * 0.88).max(13.0 * k);
    let icon_s = if has_foot { 15.0 * k } else { 0.0 };
    let fsz = if has_foot { 14.0 * k } else { 0.0 };
    let tag_sz = (lap_sz * 0.82).max(12.0 * k);
    let lap_row_w = measure(fonts, &lap_txt, lap_sz)
        + if lapped {
            LAPPED_GAP + measure(fonts, LAPPED_TAG, tag_sz)
        } else {
            0.0
        };

    let gear_w = (main_h * 0.82).clamp(52.0 * k, 70.0 * k);
    let digit = max_digit_w(fonts, val);
    let mid_w = (measure(fonts, "RPM", label) + 8.0 * k + digit * 5.0)
        .max(measure(fonts, speed_label, label) + 8.0 * k + digit * 3.0);
    let right_w = measure(fonts, &ptxt, pos_n).max(lap_row_w);
    let base_gap = 18.0 * k;
    let extra = (w - pad * 2.0 - gear_w - mid_w - right_w).max(0.0);
    let col_gap = (extra / 2.0).max(base_gap * 0.4);
    let col_origin = x + pad + ((w - pad * 2.0 - gear_w - mid_w - right_w - col_gap * 2.0) * 0.5).max(0.0);
    let gear_x = col_origin;
    let mid_x = gear_x + gear_w + col_gap;
    let right_x = mid_x + mid_w + col_gap;

    DashLay {
        x,
        y,
        w,
        h,
        flag,
        flag_h,
        flag_grow: grow,
        pad,
        foot_pad,
        footer_h,
        rev_x: x + pad,
        rev_y,
        rev_w: w - pad * 2.0,
        rev_h,
        gear_x,
        gear_w,
        main_y,
        main_h,
        mid_x,
        mid_w,
        right_x,
        label,
        gear_n,
        val,
        pos_n,
        lap_sz,
        icon_s,
        fsz,
        cut: (h * 0.14).clamp(8.0, 14.0),
        simple: false,
        gear,
        rpm,
        speed,
        speed_label,
        ptxt,
        lap_txt,
        lapped,
        lead,
        tag_sz,
        foot,
    }
}

pub(crate) fn dash_layout_simple(
    fonts: &Fonts,
    s: &Snapshot,
    cfg: &HudConfig,
    sw: f32,
    sh: f32,
    x0: f32,
    y: f32,
    flag: DashFlag,
    grow: f32,
) -> DashLay {
    const NAT_H: f32 = 92.0;
    const NAT_W: f32 = 220.0;
    let k = ((cfg[WidgetId::Dash].rect.h * sh) / NAT_H)
        .min((cfg[WidgetId::Dash].rect.w * sw) / NAT_W)
        .clamp(0.35, 2.4);
    let pad = 10.0 * k;
    let main_h = (cfg[WidgetId::Dash].rect.h * sh - pad * 2.0).max(28.0 * k);
    let h = cfg[WidgetId::Dash].rect.h * sh;
    let w = cfg[WidgetId::Dash].rect.w * sw;
    let x = x0;
    let flag_full = if flag != DashFlag::None {
        (h * 0.22).clamp(20.0 * k, 28.0 * k)
    } else {
        0.0
    };
    let flag_h = flag_full * grow;
    let cut = (h * 0.14).clamp(8.0 * k, 14.0 * k);
    let skew = (cut * 1.1).max(6.0 * k);
    let main_y = y + pad;
    let gear_n = (main_h * 0.78).clamp(32.0 * k, 56.0 * k);
    let val = (main_h * 0.62).clamp(28.0 * k, 46.0 * k);
    let unit_sz = (val * 0.22).clamp(8.0 * k, 13.0 * k);

    let gear = if s.local_gear <= 0 {
        "N".into()
    } else {
        format!("{}", s.local_gear)
    };
    let speed = cfg.units.format_speed(s.local_speed);
    let speed_label = cfg.units.speed_label();

    let gear_digit = measure(fonts, &gear, gear_n).max(max_digit_w(fonts, gear_n));
    let gear_w = (gear_digit + 18.0 * k + skew).clamp(56.0 * k, 110.0 * k);
    let speed_w = measure(fonts, &speed, val).max(max_digit_w(fonts, val) * 2.0);
    let gap = (h * 0.10).clamp(12.0 * k, 20.0 * k);
    let unit_gap = 8.0 * k;
    let gear_x = x + pad;
    let mid_x = gear_x + gear_w + gap;

    DashLay {
        x,
        y,
        w,
        h,
        flag,
        flag_h,
        flag_grow: grow,
        pad,
        foot_pad: 0.0,
        footer_h: 0.0,
        rev_x: x + pad,
        rev_y: y,
        rev_w: 0.0,
        rev_h: 0.0,
        gear_x,
        gear_w,
        main_y,
        main_h,
        mid_x,
        mid_w: speed_w,
        right_x: mid_x + speed_w + unit_gap,
        label: unit_sz,
        gear_n,
        val,
        pos_n: 0.0,
        lap_sz: 0.0,
        icon_s: 0.0,
        fsz: unit_sz,
        cut,
        simple: true,
        gear,
        rpm: String::new(),
        speed,
        speed_label,
        ptxt: String::new(),
        lap_txt: String::new(),
        lapped: false,
        lead: false,
        tag_sz: 0.0,
        foot: Vec::new(),
    }
}

/// Top banner: chamfered top corners (same language as the dash), square bottom flush on the body.
pub(crate) fn dash_flag_top_path(x: f32, y: f32, w: f32, h: f32, cut: f32) -> Option<Path> {
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    let cut = cut.min(w * 0.45).min(h * 0.9).max(0.0);
    let mut pb = PathBuilder::new();
    if cut <= 0.5 {
        push_rect(&mut pb, x, y, w, h);
    } else {
        pb.move_to(x + cut, y);
        pb.line_to(x + w - cut, y);
        pb.line_to(x + w, y + cut);
        pb.line_to(x + w, y + h);
        pb.line_to(x, y + h);
        pb.line_to(x, y + cut);
        pb.close();
    }
    pb.finish()
}

/// Flag wrap sits on the *outside* of the dash so the body cannot cover it.
pub(crate) fn dash_wrap_border(d: &DashLay) -> f32 {
    if d.flag == DashFlag::None || d.flag_grow <= 0.02 {
        0.0
    } else {
        (6.0 * d.flag_grow).clamp(0.0, 6.0)
    }
}

pub(crate) fn dash_wrap_outer(d: &DashLay, border: f32) -> (f32, f32, f32, f32) {
    let top_h = d.flag_h.max(1.0);
    (
        d.x - border,
        d.y - top_h,
        d.w + border * 2.0,
        top_h + d.h + border,
    )
}

pub(crate) fn dash_wrap_frame_path(d: &DashLay, border: f32) -> Option<Path> {
    if border <= 0.5 {
        return None;
    }
    let (ox, oy, ow, oh) = dash_wrap_outer(d, border);
    let top_h = d.flag_h.max(1.0);
    // Match flag top corners; bottom follows the dash body chamfer (not a round blob).
    let top_cut = d.cut.min(top_h * 0.9);
    let bot_cut = d.cut;
    let mut pb = PathBuilder::new();
    push_chamfer_tb(&mut pb, ox, oy, ow, oh, top_cut, bot_cut);
    // Hole matches the dash body exactly so top corners meet with no gap.
    push_chamfer(&mut pb, d.x, d.y, d.w, d.h, d.cut);
    pb.finish()
}

pub(crate) fn draw_rev_bar(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, rpm: i32, max_rpm: i32, shift_rpm: i32) {
    if w < 24.0 || h < 6.0 {
        return;
    }
    let n = 20i32;
    let ceiling = max_rpm.max(shift_rpm).max(rpm).max(8000) as f32;
    let t = (rpm.max(0) as f32 / ceiling).clamp(0.0, 1.0);
    let lit = if rpm <= 0 {
        0
    } else {
        ((t * n as f32).ceil() as i32).clamp(1, n)
    };
    fill_round(
        px,
        x - 3.0,
        y - 2.5,
        w + 6.0,
        h + 5.0,
        (h + 5.0) * 0.42,
        Color::from_rgba8(6, 6, 8, 170),
    );
    let gap = (w * 0.014).clamp(1.8, 3.0);
    let seg_w = ((w - gap * (n - 1) as f32) / n as f32).max(2.2);
    for i in 0..n {
        let sx = x + i as f32 * (seg_w + gap);
        let on = i < lit;
        let col = if !on {
            Color::from_rgba8(48, 50, 54, 230)
        } else if i >= 13 {
            Color::from_rgba8(236, 44, 36, 255)
        } else if i >= 11 {
            Color::from_rgba8(244, 214, 36, 255)
        } else {
            Color::from_rgba8(36, 230, 68, 255)
        };
        fill_round(px, sx, y, seg_w, h, h * 0.5, col);
        if on {
            fill_round(
                px,
                sx + 0.55,
                y + 0.7,
                (seg_w - 1.1).max(0.6),
                (h * 0.36).max(1.0),
                h * 0.18,
                Color::from_rgba8(255, 255, 255, 58),
            );
        }
    }
}

pub(crate) fn dash_best_ms(s: &Snapshot) -> i32 {
    RaceStore::with(|race| {
    let standing_best = race
        .field
        .focus
        .and_then(|i| race.field.rows.get(i))
        .map(|r| r.standing.best_lap_ms)
        .or_else(|| focus_standing(s).map(|st| st.best_lap_ms))
        .unwrap_or(0);
    [s.best_lap_ms, standing_best]
        .into_iter()
        .filter(|ms| *ms > 0)
        .min()
        .unwrap_or(0)
    })
}

pub(crate) fn dash_foot_item(s: &Snapshot, cfg: &HudConfig, field: DashField) -> Option<(char, String)> {
    if field == DashField::None {
        return None;
    }
    RaceStore::with(|race| {
    let st = race
        .field
        .focus
        .and_then(|i| race.field.rows.get(i))
        .map(|r| &r.standing)
        .or_else(|| focus_standing(s));
    let text = match field {
        DashField::None => return None,
        DashField::Speed => format!("{} {}", cfg.units.format_speed(s.local_speed), cfg.units.speed_label()),
        DashField::Rpm => format!("{}", s.local_rpm.max(0)),
        DashField::Gear => {
            if s.local_gear <= 0 {
                "N".into()
            } else {
                format!("{}", s.local_gear)
            }
        }
        DashField::Position => st
            .map(|r| format!("P{}", r.position.max(0)))
            .unwrap_or_else(|| "P--".into()),
        DashField::Number => {
            let n = if s.focus_race_num > 0 { s.focus_race_num } else { s.local_race_num };
            if n > 0 { format!("#{n}") } else { "--".into() }
        }
        DashField::LapCount => race_progress_text(s),
        DashField::LapsLeft => race_laps_left_text(s),
        DashField::Last => {
            let ms = st.map(|r| r.last_lap_ms).filter(|ms| *ms > 0).unwrap_or(s.last_lap_ms);
            format_clock(ms)
        }
        DashField::Best => format_clock(dash_best_ms(s)),
        DashField::Current => format_clock(s.current_lap_ms),
        DashField::Delta => {
            let best = dash_best_ms(s);
            let src = if s.current_lap_ms > 0 { s.current_lap_ms } else { s.last_lap_ms };
            if best <= 0 || src <= 0 {
                "--".into()
            } else {
                format_delta_ms(src - best)
            }
        }
        DashField::Air => cfg.units.format_temp(s.air_temp),
        DashField::Engine => cfg.units.format_temp(s.engine_temp),
        DashField::Gap => st
            .map(|r| gap_ahead_text(s, &race.field, r))
            .unwrap_or_else(|| "---".into()),
        DashField::Interval => st
            .map(|r| gap_ahead_text(s, &race.field, r))
            .unwrap_or_else(|| "---".into()),
        DashField::GapBehind => st
            .map(|r| gap_behind_text(s, &race.field, r))
            .unwrap_or_else(|| "--".into()),
        DashField::Penalty => format_penalty(st.map(|r| r.penalty_ms).unwrap_or(0)),
        DashField::Session => race_progress_text(s),
        DashField::LocalTime => local_clock(),
        DashField::Bike => st
            .map(|r| cstr(&r.bike))
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "--".into()),
        DashField::Class => st
            .map(|r| cstr(&r.category))
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "--".into()),
        DashField::Fuel => cfg.units.format_fuel(s.fuel, s.max_fuel),
        DashField::FuelPct => format_fuel_pct(s.fuel, s.max_fuel),
        DashField::Setup => {
            let name = s.setup_label();
            if name.is_empty() {
                "--".into()
            } else {
                name
            }
        }
    };
    Some((field.icon(), text))
    })
}

pub(crate) fn draw_dash_wrap(px: &mut Pixmap, fonts: &Fonts, d: &DashLay) {
    if d.flag == DashFlag::None || d.flag_grow <= 0.02 {
        return;
    }
    let grow = d.flag_grow;
    let border = dash_wrap_border(d);
    let top_h = d.flag_h.max(2.0);
    let (ox, _oy, ow, _) = dash_wrap_outer(d, border);
    let top_y = d.y - top_h;
    let top_cut = d.cut.min(top_h * 0.9);

    let Some(band) = dash_flag_top_path(ox, top_y, ow, top_h + 0.75, top_cut) else {
        return;
    };
    match d.flag {
        DashFlag::White => {
            draw_white_wrap(px, d, border, ox, ow);
            draw_white_banner(px, fonts, &band, ox, top_y, ow, top_h, grow);
        }
        DashFlag::Yellow => {
            let cloth = flag_yellow_cloth(255);
            let ink = Color::from_rgba8(24, 20, 8, 255);
            draw_solid_wrap(px, d, border, cloth);
            draw_solid_banner(px, fonts, &band, ox, top_y, ow, top_h, grow, cloth, ink, "YELLOW FLAG");
        }
        DashFlag::Blue => {
            let cloth = flag_blue_cloth(255);
            let ink = Color::from_rgba8(248, 248, 250, 255);
            draw_solid_wrap(px, d, border, cloth);
            draw_solid_banner(px, fonts, &band, ox, top_y, ow, top_h, grow, cloth, ink, "BLUE FLAG");
        }
        DashFlag::Red => {
            let cloth = flag_red_cloth(255);
            let ink = Color::from_rgba8(248, 248, 250, 255);
            draw_solid_wrap(px, d, border, cloth);
            draw_solid_banner(px, fonts, &band, ox, top_y, ow, top_h, grow, cloth, ink, "RED FLAG");
        }
        DashFlag::Checkered => {
            draw_checkered_wrap(px, d, border, ox, ow);
            draw_checkered_banner(px, fonts, &band, ox, top_y, ow, top_h, grow);
        }
        DashFlag::None => {}
    }
}

pub(crate) fn draw_unit_stack(
    px: &mut Pixmap,
    fonts: &Fonts,
    label: &str,
    size: f32,
    x: f32,
    y: f32,
    h: f32,
    color: Color,
) {
    let n = label.chars().count().max(1) as f32;
    let step = (h / n).min(size * 1.05);
    let stack_h = step * n;
    let mut cy = y + (h - stack_h) * 0.5;
    let mut buf = [0u8; 4];
    for ch in label.chars() {
        text(px, fonts, ch.encode_utf8(&mut buf), size, x, cy, color, true);
        cy += step;
    }
}

pub(crate) fn draw_simple_dash(px: &mut Pixmap, fonts: &Fonts, d: &DashLay, a: u8, shift_warn: bool) {
    if let Some(path) = chamfer_path(d.x, d.y, d.w, d.h, d.cut) {
        if a > 0 {
            fill_path(px, &path, Color::from_rgba8(18, 18, 20, a));
        }
        if (d.flag == DashFlag::None || d.flag_grow <= 0.02) && a > 40 {
            stroke_path(
                px,
                &path,
                Color::from_rgba8(220, 220, 224, ((a as u16 * 200) / 255).max(90) as u8),
                1.4,
            );
        }
    }

    let skew = (d.cut * 1.1).max(6.0);
    let tile_a = a.max(220);
    fill_skew(
        px,
        d.gear_x,
        d.main_y,
        (d.gear_w - skew).max(28.0),
        d.main_h,
        skew,
        if shift_warn {
            Color::from_rgba8(232, 132, 138, tile_a)
        } else {
            Color::from_rgba8(255, 148, 48, tile_a)
        },
    );
    let ink = ink_on(accent());
    text_bold(
        px,
        fonts,
        &d.gear,
        d.gear_n,
        d.gear_x + d.gear_w * 0.5,
        d.main_y + (d.main_h - d.gear_n) * 0.45,
        ink,
        true,
    );

    let white = Color::from_rgba8(248, 248, 250, 255);
    let unit = Color::from_rgba8(176, 176, 184, 255);
    let speed_y = d.main_y + (d.main_h - d.val) * 0.38;
    text_bold(px, fonts, &d.speed, d.val, d.mid_x, speed_y, white, false);
    draw_unit_stack(
        px,
        fonts,
        d.speed_label,
        d.fsz,
        d.right_x + d.fsz * 0.45,
        d.main_y,
        d.main_h,
        unit,
    );

    draw_dash_wrap(px, fonts, d);
}

pub(crate) fn draw_dash_lead_crown(px: &mut Pixmap, fonts: &Fonts, d: &DashLay, pos_y: f32) {
    let size = (d.pos_n * 0.38).clamp(11.0, 17.0);
    let cx = d.right_x + measure(fonts, &d.ptxt, d.pos_n) * 0.5;
    let cy = pos_y - size * 0.88;
    icon(
        px,
        fonts,
        '\u{f521}',
        size,
        cx + 0.7,
        cy + 0.7,
        Color::from_rgba8(8, 8, 10, 220),
        true,
    );
    icon(
        px,
        fonts,
        '\u{f521}',
        size,
        cx,
        cy,
        Color::from_rgba8(255, 196, 48, 255),
        true,
    );
}

pub(crate) fn draw_dash(px: &mut Pixmap, fonts: &Fonts, s: &Snapshot, cfg: &HudConfig, sw: f32, sh: f32, flag: DashFlag, grow: f32) {
    let d = dash_layout(fonts, s, cfg, sw, sh, flag, grow);
    if d.simple {
        draw_simple_dash(
            px,
            fonts,
            &d,
            bg_a(cfg[WidgetId::Dash].bg),
            cfg.dash_shift_color && crate::telemetry::shift_warn(s),
        );
        return;
    }
    let a = bg_a(cfg[WidgetId::Dash].bg);
    if let Some(path) = chamfer_path(d.x, d.y, d.w, d.h, d.cut) {
        if a > 0 {
            fill_path(px, &path, Color::from_rgba8(18, 18, 20, a));
        }
        if let Some(shader) = LinearGradient::new(
            SkPoint::from_xy(d.x + d.w * 0.42, d.y),
            SkPoint::from_xy(d.x + d.w, d.y + d.h * 0.85),
            vec![
                GradientStop::new(0.0, Color::from_rgba8(255, 255, 255, 0)),
                GradientStop::new(0.58, Color::from_rgba8(255, 255, 255, 0)),
                GradientStop::new(0.74, Color::from_rgba8(255, 255, 255, ((38.0 * a as f32) / 255.0) as u8)),
                GradientStop::new(1.0, Color::from_rgba8(255, 255, 255, 0)),
            ],
            SpreadMode::Pad,
            Transform::identity(),
        ) {
            let mut paint = Paint::default();
            paint.shader = shader;
            paint.anti_alias = true;
            px.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
        }
        if d.flag == DashFlag::None || d.flag_grow <= 0.02 {
            stroke_path(px, &path, Color::from_rgba8(220, 220, 224, ((a as u16 * 200) / 255).max(90) as u8), 1.4);
        }
    }

    if cfg.dash_rev {
        draw_rev_bar(px, d.rev_x, d.rev_y, d.rev_w, d.rev_h, s.local_rpm, s.max_rpm, s.shift_rpm);
    }

    let white = Color::from_rgba8(248, 248, 250, 255);
    let dim = Color::from_rgba8(168, 168, 176, 255);
    if let Some(path) = chamfer_path(d.gear_x, d.main_y, d.gear_w, d.main_h, d.cut * 0.45) {
        stroke_path(px, &path, Color::from_rgba8(236, 236, 240, 210), 1.3);
    }
    text_bold(
        px,
        fonts,
        &d.gear,
        d.gear_n,
        d.gear_x + d.gear_w * 0.5,
        d.main_y + (d.main_h - d.gear_n) * 0.42,
        shift_gear_col(s, cfg.dash_shift_color, white),
        true,
    );

    text(px, fonts, "RPM", d.label, d.mid_x, d.main_y + d.main_h * 0.12, dim, false);
    text(px, fonts, &d.rpm, d.val, d.mid_x + d.mid_w - measure(fonts, &d.rpm, d.val), d.main_y + d.main_h * 0.10, white, false);
    if let Some(line) = rr(d.mid_x, d.main_y + d.main_h * 0.48, d.mid_w, 1.0) {
        fill_rect(px, line, Color::from_rgba8(200, 200, 206, 70));
    }
    text(px, fonts, d.speed_label, d.label, d.mid_x, d.main_y + d.main_h * 0.62, dim, false);
    text(px, fonts, &d.speed, d.val, d.mid_x + d.mid_w - measure(fonts, &d.speed, d.val), d.main_y + d.main_h * 0.58, white, false);

    let pos_y = if d.lead {
        d.main_y + d.main_h * 0.22
    } else {
        d.main_y + d.main_h * 0.10
    };
    if d.lead {
        draw_dash_lead_crown(px, fonts, &d, pos_y);
    }
    text_bold(px, fonts, &d.ptxt, d.pos_n, d.right_x + 1.0, pos_y + 1.0, Color::from_rgba8(20, 12, 6, 160), false);
    text_bold(px, fonts, &d.ptxt, d.pos_n, d.right_x, pos_y, dash_pos_col(), false);
    let lap_y = d.main_y + d.main_h * 0.68;
    text(px, fonts, &d.lap_txt, d.lap_sz, d.right_x, lap_y, white, false);
    if d.lapped {
        // Baselines differ with the smaller size, so nudge down to sit on the lap text.
        let tag_x = d.right_x + measure(fonts, &d.lap_txt, d.lap_sz) + LAPPED_GAP;
        let tag_y = lap_y + (d.lap_sz - d.tag_sz) * 0.72;
        text(px, fonts, LAPPED_TAG, d.tag_sz, tag_x, tag_y, dash_lapped_col(), false);
    }

    if !d.foot.is_empty() {
        let fy = d.y + d.h - d.foot_pad - d.footer_h + 1.0;
        let foot_inner = (d.w - d.pad * 2.0).max(1.0);
        let foot_used: f32 = d
            .foot
            .iter()
            .map(|(ch, label)| fonts.icons.metrics(*ch, d.icon_s).advance_width + 5.0 + measure(fonts, label, d.fsz))
            .sum();
        let foot_gap = ((foot_inner - foot_used) / (d.foot.len() as f32 + 1.0)).max(8.0);
        let mut fx = d.x + d.pad + foot_gap;
        for (ch, label) in &d.foot {
            icon(px, fonts, *ch, d.icon_s, fx, fy, white, false);
            fx += fonts.icons.metrics(*ch, d.icon_s).advance_width + 5.0;
            text(px, fonts, label, d.fsz, fx, fy + 1.0, white, false);
            fx += measure(fonts, label, d.fsz) + foot_gap;
        }
    }

    // Last: wrap sits on the outer edge so the body cannot cover it.
    draw_dash_wrap(px, fonts, &d);
}
