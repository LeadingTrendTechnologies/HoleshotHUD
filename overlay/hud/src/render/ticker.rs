#![allow(unused_imports)]
use super::*;

pub(crate) fn ticker_delta(focus: &crate::shm::Standing, row: &crate::shm::Standing) -> String {
    format_signed_delta(row.gap_ms - focus.gap_ms, row.gap_laps - focus.gap_laps)
}

pub(crate) fn ticker_meta_label(field: BoardField, val: &str) -> &'static str {
    match field {
        BoardField::Lap | BoardField::LapsLeft | BoardField::Session | BoardField::RaceTime => {
            if val.contains('/') || val.starts_with('+') {
                "LAPS"
            } else {
                "TIME"
            }
        }
        BoardField::Air => "TEMP",
        BoardField::Best | BoardField::SessionBest => "BEST",
        BoardField::Position | BoardField::ClassPos => "POS",
        BoardField::Track => "TRACK",
        BoardField::Riders => "RIDERS",
        BoardField::LocalTime => "CLOCK",
        BoardField::SessionType => "SESSION",
        BoardField::Fuel | BoardField::FuelPct => "FUEL",
        BoardField::Setup => "SETUP",
        BoardField::GapAhead => "AHEAD",
        BoardField::GapBehind => "BEHIND",
        BoardField::None => "",
    }
}

pub(crate) fn ticker_title(s: &Snapshot) -> String {
    let track = cstr(&s.track_name);
    let kind = if is_warmup(s) {
        "WARMUP"
    } else if is_lap_race(s) {
        "LAP RACE"
    } else if overtime_active(s) {
        "EXTRA"
    } else if s.session_length > 0 {
        "TIMED"
    } else {
        "SESSION"
    };
    if track.is_empty() {
        kind.into()
    } else {
        format!("{kind} - {}", track.to_uppercase())
    }
}

pub(crate) fn draw_ticker(px: &mut Pixmap, fonts: &Fonts, s: &Snapshot, cfg: &HudConfig, sw: f32, sh: f32) {
    let r = cfg[WidgetId::Ticker].rect;
    let x = r.x * sw;
    let y = r.y * sh;
    let w = (r.w * sw).max(280.0);
    let h = (r.h * sh).clamp(42.0, 64.0);
    let k = style_k();
    let a = bg_a(cfg[WidgetId::Ticker].bg);
    let cut = (h * 0.16).clamp(8.0, 14.0);
    if a > 0 {
        fill_skew(px, x, y, w, h, cut, Color::from_rgba8(16, 16, 20, a));
        fill_skew(px, x + 1.0, y, w - 2.0, 2.2, cut * 0.2, accent());
    }

    let pad_l = (h * 0.10).clamp(6.0, 12.0);
    let pad_r = (h * 0.05).clamp(3.0, 6.0);
    let show_title = cfg.ticker_title;
    let title_h = if show_title {
        (h * 0.22).clamp(11.0, 15.0)
    } else {
        0.0
    };
    let inner_x = x + pad_l + cut * 0.35;
    let inner_w = (w - pad_l - pad_r).max(120.0);
    let card_y = y + if show_title { title_h + 1.0 } else { 4.0 };
    let card_h = (y + h - 4.0 - card_y).max(22.0);
    let left_on = cfg.ticker_left != BoardField::None;
    let right_on = cfg.ticker_right != BoardField::None;
    let left_w = if left_on {
        ticker_meta_width(fonts, s, cfg, cfg.ticker_left, card_h, k)
    } else {
        0.0
    };
    let right_w = if right_on {
        ticker_meta_width(fonts, s, cfg, cfg.ticker_right, card_h, k)
    } else {
        0.0
    };
    let side_gap = (h * 0.12).clamp(8.0, 12.0);
    let cards_x = inner_x + left_w + if left_on { side_gap } else { 0.0 };
    let cards_w = (inner_x + inner_w - right_w - if right_on { side_gap } else { 0.0 } - cards_x).max(80.0);

    if left_on {
        draw_ticker_meta(
            px,
            fonts,
            s,
            cfg,
            cfg.ticker_left,
            inner_x,
            card_y,
            left_w,
            card_h,
            k,
            false,
        );
    }
    if right_on {
        draw_ticker_meta(
            px,
            fonts,
            s,
            cfg,
            cfg.ticker_right,
            inner_x + inner_w - right_w,
            card_y,
            right_w,
            card_h,
            k,
            true,
        );
    }

    if show_title {
        let title = ticker_title(s);
        let title_sz = (h * 0.16).clamp(8.5, 12.0);
        let title_max = (cards_w - 8.0).max(40.0);
        let title = ellipsize(fonts, &title, title_sz, title_max);
        text(
            px,
            fonts,
            &title,
            title_sz,
            cards_x + cards_w * 0.5,
            y + 4.0,
            Color::from_rgba8(236, 236, 240, 255),
            true,
        );
    }

    RaceStore::with(|race| {
    // Cards follow the live order, so a pass slides the field now, not at the line.
    let mut board = race.field.board();
    if board.is_empty() {
        let count = (s.standing_count.max(0) as usize).min(MAX_SAFE);
        board.extend_from_slice(&s.standings[..count]);
    }
    let n = board.len().min(MAX_SAFE);
    if n == 0 {
        text(
            px,
            fonts,
            "Waiting for race data",
            11.0,
            cards_x,
            card_y + card_h * 0.35,
            text_dim(),
            false,
        );
        return;
    }
    let want = cfg.ticker_count.clamp(3, 15) as usize;
    let (vis, card_w) = hstand_layout(cards_w, k, want, n);
    let focus = s.focus_race_num;
    let fi = race
        .field
        .focus
        .filter(|i| *i < n)
        .or_else(|| (0..n).find(|&i| board[i].race_num == focus))
        .unwrap_or(0);
    let Some(focus_row) = board.get(fi) else {
        return;
    };
    let best_ms = if race.field.session_best_ms > 0 {
        race.field.session_best_ms
    } else {
        session_best_ms(s)
    };
    let gap = HS_CARD_GAP;
    let stride = card_w + gap;
    let now = anim_now();
    let ids = row_ids(board.iter().take(n).map(|card| card.race_num));
    let slots = HS_SLIDE.with(|a| a.borrow_mut().indices(&ids, now));
    let scroll = if cfg.ticker_autoscroll && n > vis {
        (now * HS_AUTO_SPEED).rem_euclid(n as f32)
    } else {
        let target = hstand_scroll_start(fi, vis, n);
        HS_SCROLL.with(|a| a.borrow_mut().step(target, now))
    };
    let lw = cards_w.ceil().max(1.0) as u32;
    let lh = card_h.ceil().max(1.0) as u32;
    if let Some(mut layer) = Pixmap::new(lw, lh) {
        for (i, card) in board.iter().enumerate().take(n) {
            let slot = slots.get(i).copied().unwrap_or(i as f32);
            let x = if cfg.ticker_autoscroll && n > vis {
                match hstand_loop_x(slot, scroll, n as f32, stride, cards_w, card_w) {
                    Some(x) => x,
                    None => continue,
                }
            } else {
                let x = hstand_card_x(slot, scroll, 0.0, stride);
                if x + card_w < -2.0 || x > cards_w + 2.0 {
                    continue;
                }
                x
            };
            draw_ticker_card(
                &mut layer,
                fonts,
                s,
                card,
                focus_row,
                best_ms,
                x,
                0.0,
                card_w,
                card_h,
                k,
            );
            push_click_rider(card.race_num, cards_x + x, card_y, card_w, card_h);
        }
        px.draw_pixmap(
            0,
            0,
            layer.as_ref(),
            &PixmapPaint::default(),
            Transform::from_translate(cards_x, card_y),
            None,
        );
    }
    });
}

pub(crate) fn hstand_card_x(index: f32, scroll: f32, origin: f32, stride: f32) -> f32 {
    origin + (index - scroll) * stride
}

pub(crate) const HS_AUTO_SPEED: f32 = 0.42;

pub(crate) const HS_CARD_GAP: f32 = 3.0;

pub(crate) fn hstand_card_range(k: f32) -> (f32, f32) {
    let min_w = (86.0 * k).clamp(78.0, 96.0);
    let max_w = (124.0 * k).clamp(112.0, 140.0);
    (min_w, max_w.max(min_w + 8.0))
}

pub(crate) fn hstand_layout(cards_w: f32, k: f32, want: usize, n: usize) -> (usize, f32) {
    let gap = HS_CARD_GAP;
    let (min_w, max_w) = hstand_card_range(k);
    let n = n.max(1);
    let max_fit = if cards_w < min_w {
        1
    } else {
        (((cards_w + gap) / (min_w + gap)).floor() as usize).clamp(1, n)
    };
    let mut vis = want.clamp(1, max_fit);
    loop {
        let cw = (cards_w - gap * vis.saturating_sub(1) as f32) / vis as f32;
        if cw <= max_w || vis >= max_fit {
            return (vis, cw.clamp(min_w, max_w));
        }
        vis += 1;
    }
}

pub(crate) fn hstand_loop_x(
    index: f32,
    scroll: f32,
    n: f32,
    stride: f32,
    cards_w: f32,
    card_w: f32,
) -> Option<f32> {
    if n <= 0.0 || stride <= 0.0 {
        return None;
    }
    let period = n * stride;
    let mut x = (index - scroll) * stride;
    x = x.rem_euclid(period);
    if x > cards_w {
        x -= period;
    }
    if x + card_w < -2.0 || x > cards_w + 2.0 {
        None
    } else {
        Some(x)
    }
}

pub(crate) fn hstand_scroll_start(focus_idx: usize, vis: usize, n: usize) -> f32 {
    let vis = vis.max(1);
    if n <= vis {
        return 0.0;
    }
    if focus_idx + 1 <= vis {
        0.0
    } else {
        (focus_idx + 1 - vis).min(n - vis) as f32
    }
}

pub(crate) fn ticker_meta_copy(
    _fonts: &Fonts,
    s: &Snapshot,
    cfg: &HudConfig,
    field: BoardField,
    h: f32,
    k: f32,
) -> Option<(String, String, f32, f32)> {
    let (_, val) = board_item(s, cfg, field)?;
    let label = ticker_meta_label(field, &val).to_string();
    let label_sz = (8.5 * k).clamp(7.5, 10.0);
    let val_sz = (h * 0.28).clamp(13.0, 20.0);
    Some((label, val, label_sz, val_sz))
}

pub(crate) fn ticker_meta_width(
    fonts: &Fonts,
    s: &Snapshot,
    cfg: &HudConfig,
    field: BoardField,
    h: f32,
    k: f32,
) -> f32 {
    let Some((label, val, label_sz, val_sz)) = ticker_meta_copy(fonts, s, cfg, field, h, k) else {
        return 36.0;
    };
    measure(fonts, &label, label_sz)
        .max(measure_bold(fonts, &val, val_sz))
        + 2.0
}

pub(crate) fn draw_ticker_meta(
    px: &mut Pixmap,
    fonts: &Fonts,
    s: &Snapshot,
    cfg: &HudConfig,
    field: BoardField,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    k: f32,
    align_right: bool,
) {
    let Some((label, val, label_sz, val_sz)) = ticker_meta_copy(fonts, s, cfg, field, h, k) else {
        return;
    };
    let lx = if align_right {
        x + w - measure(fonts, &label, label_sz)
    } else {
        x
    };
    let vx = if align_right {
        x + w - measure_bold(fonts, &val, val_sz)
    } else {
        x
    };
    text(
        px,
        fonts,
        &label,
        label_sz,
        lx,
        y + h * 0.22,
        Color::from_rgba8(150, 150, 156, 255),
        false,
    );
    text_bold(
        px,
        fonts,
        &val,
        val_sz,
        vx,
        y + h * 0.42,
        Color::from_rgba8(244, 244, 247, 255),
        false,
    );
}

pub(crate) fn draw_ticker_card(
    px: &mut Pixmap,
    fonts: &Fonts,
    s: &Snapshot,
    row: &crate::shm::Standing,
    focus: &crate::shm::Standing,
    best_ms: i32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    k: f32,
) {
    let is_focus = row.race_num == focus.race_num;
    let out = standing_status(row).is_some() && standing_status(row) != Some("PIT");
    if is_focus {
        fill_round(px, x, y, w, h, 3.0, you_row_bg(100));
    }
    let pos_s = (h * 0.38).clamp(14.0, 20.0);
    let pos_y = y + (h - pos_s) * 0.5;
    if let Some(rrt) = rr(x + 4.0, pos_y, pos_s, pos_s) {
        fill_rect(px, rrt, Color::from_rgba8(244, 244, 247, 255));
    }
    let pos = format!("{}", row.position.max(0));
    text_bold(
        px,
        fonts,
        &pos,
        (pos_s * 0.62).clamp(9.0, 13.0),
        x + 4.0 + pos_s * 0.5,
        pos_y + pos_s * 0.18,
        Color::from_rgba8(12, 12, 14, 255),
        true,
    );
    let accent_c = bike_color(&cstr(&row.bike), &cstr(&row.category));
    let bar_x = x + 4.0 + pos_s + 3.0;
    if let Some(rrt) = rr(bar_x, y + 4.0, 2.2, h - 8.0) {
        fill_rect(px, rrt, accent_c);
    }
    let text_x = bar_x + 7.0;
    let name_sz = (h * 0.28).clamp(10.5, 13.5);
    let gap_sz = (h * 0.22).clamp(8.5, 11.0);
    let name = ellipsize(fonts, &cstr(&row.name), name_sz, (w - (text_x - x) - 8.0).max(24.0));
    let name_c = if out {
        Color::from_rgba8(110, 110, 116, 255)
    } else {
        Color::from_rgba8(244, 244, 247, 255)
    };
    text_bold(px, fonts, &name, name_sz, text_x, y + h * 0.16, name_c, false);
    let gap_c = if out {
        Color::from_rgba8(110, 110, 116, 255)
    } else if is_focus {
        Color::from_rgba8(200, 200, 206, 255)
    } else {
        Color::from_rgba8(168, 168, 176, 255)
    };
    let gap = if is_focus {
        let ms = if row.last_lap_ms > 0 {
            row.last_lap_ms
        } else if s.last_lap_ms > 0 {
            s.last_lap_ms
        } else {
            row.best_lap_ms
        };
        format_lap(ms)
    } else if let Some(st) = standing_status(row) {
        st.to_string()
    } else {
        RaceStore::with(|store| {
            store
                .field
                .row_by_num(row.race_num)
                .map(ticker_delta_from_row)
                .unwrap_or_else(|| ticker_delta(focus, row))
        })
    };
    text(px, fonts, &gap, gap_sz, text_x, y + h * 0.52, gap_c, false);
    if best_ms > 0 && row.best_lap_ms == best_ms && !out {
        let tag = "FASTEST LAP";
        let tag_sz = (7.5 * k).clamp(6.5, 8.5);
        let tw = measure(fonts, tag, tag_sz);
        text(
            px,
            fonts,
            tag,
            tag_sz,
            x + w - tw - 6.0,
            y + h - tag_sz - 5.0,
            Color::from_rgba8(232, 232, 236, 255),
            false,
        );
    }
}
