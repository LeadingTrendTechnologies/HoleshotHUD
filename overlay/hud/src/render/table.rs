use std::cell::RefCell;

use tiny_skia::{Color, Pixmap};

use super::*;

thread_local! {
    pub(crate) static ST_SLIDE: RefCell<TableSlides> = RefCell::new(TableSlides { rows: Vec::new() });
    pub(crate) static REL_SLIDE: RefCell<TableSlides> = RefCell::new(TableSlides { rows: Vec::new() });
    pub(crate) static HS_SCROLL: RefCell<IndexSlide> = RefCell::new(IndexSlide {
        from: 0.0,
        to: 0.0,
        start: 0.0,
        init: false,
    });
    pub(crate) static HS_SLIDE: RefCell<TableSlides> = RefCell::new(TableSlides { rows: Vec::new() });
    static CLICK_RIDERS: RefCell<Vec<ClickRider>> = RefCell::new(Vec::new());
}

pub(crate) struct IndexSlide {
    pub(crate) from: f32,
    pub(crate) to: f32,
    pub(crate) start: f32,
    pub(crate) init: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct ClickRider {
    pub race_num: i32,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

pub(crate) fn push_click_rider(race_num: i32, x: f32, y: f32, w: f32, h: f32) {
    if race_num <= 0 || w <= 1.0 || h <= 1.0 {
        return;
    }
    CLICK_RIDERS.with(|v| {
        v.borrow_mut().push(ClickRider {
            race_num,
            x,
            y,
            w,
            h,
        });
    });
}

pub(crate) fn clear_click_riders() {
    CLICK_RIDERS.with(|v| v.borrow_mut().clear());
}

pub fn click_rider_hits() -> Vec<ClickRider> {
    CLICK_RIDERS.with(|v| v.borrow().clone())
}

pub fn click_rider_at(px: f32, py: f32) -> Option<i32> {
    CLICK_RIDERS.with(|v| {
        v.borrow()
            .iter()
            .rev()
            .find(|h| px >= h.x && px < h.x + h.w && py >= h.y && py < h.y + h.h)
            .map(|h| h.race_num)
    })
}

impl IndexSlide {
    pub(crate) fn step(&mut self, target: f32, now: f32) -> f32 {
        const DUR: f32 = 0.38;
        if !self.init {
            self.from = target;
            self.to = target;
            self.start = now;
            self.init = true;
            return target;
        }
        if (self.to - target).abs() > 0.02 {
            let t = ((now - self.start) / DUR).clamp(0.0, 1.0);
            self.from += (self.to - self.from) * ease_out_cubic(t);
            self.to = target;
            self.start = now;
        }
        let t = ((now - self.start) / DUR).clamp(0.0, 1.0);
        self.from + (self.to - self.from) * ease_out_cubic(t)
    }
}

pub(crate) struct RowSlide {
    id: i32,
    from: f32,
    to: f32,
    start: f32,
}

pub(crate) struct TableSlides {
    pub(crate) rows: Vec<RowSlide>,
}

pub(crate) fn anim_now() -> f32 {
    #[cfg(target_arch = "wasm32")]
    {
        thread_local! {
            static T: std::cell::Cell<f32> = const { std::cell::Cell::new(0.0) };
        }
        T.with(|c| {
            let v = c.get() + 1.0 / 60.0;
            c.set(v);
            v
        })
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        thread_local! {
            static ORIGIN: std::time::Instant = std::time::Instant::now();
        }
        ORIGIN.with(|o| o.elapsed().as_secs_f32())
    }
}

pub(crate) fn ease_out_cubic(t: f32) -> f32 {
    let u = 1.0 - t.clamp(0.0, 1.0);
    1.0 - u * u * u
}

impl TableSlides {
    pub(crate) fn indices(&mut self, ids: &[i32], now: f32) -> Vec<f32> {
        const DUR: f32 = 0.30;
        let displayed = |e: &RowSlide| {
            let t = ((now - e.start) / DUR).clamp(0.0, 1.0);
            e.from + (e.to - e.from) * ease_out_cubic(t)
        };
        let mut out = Vec::with_capacity(ids.len());
        for (i, &id) in ids.iter().enumerate() {
            let target = i as f32;
            if let Some(idx) = self.rows.iter().position(|e| e.id == id) {
                let cur = displayed(&self.rows[idx]);
                let e = &mut self.rows[idx];
                if (e.to - target).abs() > 0.05 {
                    e.from = cur;
                    e.to = target;
                    e.start = now;
                }
                out.push(displayed(e));
            } else {
                self.rows.push(RowSlide {
                    id,
                    from: target,
                    to: target,
                    start: now,
                });
                out.push(target);
            }
        }
        self.rows.retain(|e| ids.contains(&e.id));
        out
    }

    pub(crate) fn step(&mut self, ids: &[i32], body_y: f32, row_h: f32, now: f32) -> Vec<f32> {
        self.indices(ids, now)
            .into_iter()
            .map(|i| body_y + i * row_h)
            .collect()
    }
}

pub(crate) fn row_ids(ids: impl IntoIterator<Item = i32>) -> Vec<i32> {
    ids.into_iter()
        .enumerate()
        .map(|(i, id)| if id > 0 { id } else { -(i as i32 + 1) })
        .collect()
}
pub(crate) fn col_slots<T: Copy>(
    origin: f32,
    pad: f32,
    avail: f32,
    cols: &[T],
    mut width: impl FnMut(T) -> f32,
    is_flex: impl Fn(T) -> bool,
) -> Vec<(T, f32, f32)> {
    if cols.is_empty() {
        return Vec::new();
    }
    const GAP: f32 = 4.0;
    const MIN_W: f32 = 18.0;
    let mut widths: Vec<f32> = cols.iter().copied().map(&mut width).collect();
    let gaps = GAP * (cols.len() - 1) as f32;
    let inner = (avail - pad * 2.0).max(0.0);
    let used: f32 = widths.iter().sum::<f32>() + gaps;
    let leftover = inner - used;
    // Honor configured widths when they fit. Only shrink on overflow so width
    // sliders (especially Name) actually change how wide each column draws.
    if leftover < -0.5 {
        let flex = cols
            .iter()
            .position(|&c| is_flex(c))
            .unwrap_or(cols.len() - 1);
        let mut remain = -leftover;
        let shrink = (widths[flex] - MIN_W).max(0.0).min(remain);
        widths[flex] -= shrink;
        remain -= shrink;
        if remain > 0.5 {
            let room: Vec<f32> = widths.iter().map(|w| (*w - MIN_W).max(0.0)).collect();
            let total_room: f32 = room.iter().sum();
            if total_room > 0.0 {
                for (w, r) in widths.iter_mut().zip(room) {
                    if r <= 0.0 {
                        continue;
                    }
                    *w = (*w - remain * (r / total_room)).max(MIN_W);
                }
            }
        }
    }
    let mut x = origin + pad;
    let mut out = Vec::with_capacity(cols.len());
    for (i, &col) in cols.iter().enumerate() {
        out.push((col, x, widths[i]));
        x += widths[i] + GAP;
    }
    out
}

pub(crate) fn hug_board_w<T>(origin: f32, pad: f32, max_w: f32, slots: &[(T, f32, f32)]) -> f32 {
    let Some((_, x, cw)) = slots.last() else {
        return max_w;
    };
    (x + cw - origin + pad).clamp(pad * 2.0 + 40.0, max_w)
}

pub(crate) fn table_font_k(pct: i32) -> f32 {
    (pct.clamp(70, 160) as f32) / 100.0
}

pub(crate) fn table_stack_h(k: f32, vis_rows: usize, has_foot: bool, show_plaque: bool) -> f32 {
    let head_h = 26.0 * k;
    let col_h = 16.0 * k;
    let track_h = if show_plaque { 20.0 * k } else { 0.0 };
    let row_h = 22.0 * k;
    let foot_h = if has_foot { 20.0 * k } else { 0.0 };
    // Follow the row stack, not a stale widget box. Ctrl+move can save a
    // hugged 1-row height; Rows can grow without rewriting standings_h.
    head_h + col_h + track_h + vis_rows as f32 * row_h + foot_h + 8.0
}

pub(crate) fn standings_vis_rows(s: &Snapshot) -> usize {
    let n = (s.standing_count.max(0) as usize).min(MAX_STANDINGS);
    let max_rows = s.standings_rows.max(3) as usize;
    n.min(max_rows).max(1)
}

pub(crate) fn relative_vis_rows(s: &Snapshot) -> usize {
    let n = s.rider_count.max(0) as usize;
    let focus = if s.focus_race_num > 0 {
        s.focus_race_num
    } else {
        s.local_race_num
    };
    let mut have = s.has_telemetry != 0;
    let mut uniq: Vec<usize> = Vec::new();
    for i in 0..n {
        let rider = &s.riders[i];
        let empty = rider.race_num <= 0 && bytes_as_text(&rider.name).is_empty();
        if empty {
            continue;
        }
        if rider.race_num > 0 && uniq.iter().any(|&j| s.riders[j].race_num == rider.race_num) {
            continue;
        }
        if rider.race_num == focus {
            have = true;
        }
        uniq.push(i);
    }
    let side = s.relative_count.max(1) as usize;
    if !have || uniq.is_empty() {
        1
    } else {
        uniq.len().min(side * 2 + 1).max(1)
    }
}

/// Painted standings / relative plaque in normalized overlay space (hugged columns, row stack).
pub fn table_layout_rect(
    s: &Snapshot,
    cfg: &HudConfig,
    id: WidgetId,
    rect: crate::shm::Rect,
    sw: f32,
    sh: f32,
) -> crate::shm::Rect {
    if sw <= 0.0 || sh <= 0.0 {
        return rect;
    }
    let (widths, flex, vis, has_foot, show_plaque) = match id {
        WidgetId::Standings => {
            let cols = cfg.standings_cols();
            let flex = cols
                .iter()
                .position(|c| c.is_name())
                .unwrap_or(cols.len().saturating_sub(1));
            (
                cols.iter().map(|c| c.width(cfg) as f32).collect::<Vec<_>>(),
                flex,
                standings_vis_rows(s),
                BoardField::any(&cfg.standings.st_foot),
                cfg.standings.st_plaque,
            )
        }
        WidgetId::Relative => {
            let cols = cfg.relative_cols();
            let flex = cols
                .iter()
                .position(|c| c.is_name())
                .unwrap_or(cols.len().saturating_sub(1));
            (
                cols.iter().map(|c| c.width(cfg) as f32).collect::<Vec<_>>(),
                flex,
                relative_vis_rows(s),
                BoardField::any(&cfg.relative.rel_foot),
                cfg.relative.rel_plaque,
            )
        }
        _ => return rect,
    };
    let k = table_font_k(cfg[id].font);
    let pad = 8.0;
    let max_w = rect.w * sw;
    let idxs: Vec<usize> = (0..widths.len()).collect();
    let slots = col_slots(0.0, pad, max_w, &idxs, |i| widths[i], |i| i == flex);
    let w = hug_board_w(0.0, pad, max_w, &slots);
    let h = table_stack_h(k, vis, has_foot, show_plaque);
    crate::shm::Rect {
        x: rect.x,
        y: rect.y,
        w: w / sw,
        h: h / sh,
    }
}
pub(crate) trait BoardCol: Copy + PartialEq {
    fn header(self) -> &'static str;
    fn width(self, cfg: &HudConfig) -> i32;
    fn is_name(self) -> bool;
    fn is_bike(self) -> bool;
    fn is_pos(self) -> bool;
    fn is_status(self) -> bool;
}

impl BoardCol for StField {
    fn header(self) -> &'static str {
        match self {
            Self::Pos => "P",
            Self::Num => "#",
            Self::Name => "NAME",
            Self::Laps => "Completed Laps",
            Self::Current => "Current Lap",
            Self::Best => "Fastest",
            Self::Last => "Last",
            Self::LapDiff => "DIFF",
            Self::Status => "",
            Self::Gap => "GAP",
            Self::Interval => "INT",
            Self::Bike => "BIKE",
            Self::Penalty => "PEN",
            Self::Crashed => "CR",
            Self::Category => "CLASS",
        }
    }

    fn width(self, cfg: &HudConfig) -> i32 {
        StField::width(self, cfg)
    }

    fn is_name(self) -> bool {
        matches!(self, Self::Name)
    }

    fn is_bike(self) -> bool {
        matches!(self, Self::Bike)
    }

    fn is_pos(self) -> bool {
        matches!(self, Self::Pos)
    }

    fn is_status(self) -> bool {
        matches!(self, Self::Status)
    }
}

impl BoardCol for RelField {
    fn header(self) -> &'static str {
        match self {
            Self::Pos => "P",
            Self::Num => "#",
            Self::Name => "NAME",
            Self::Gap => "Gap",
            Self::Laps => "Completed Laps",
            Self::Current => "Current Lap",
            Self::Bike => "BIKE",
            Self::Penalty => "Pen",
            Self::Interval => "Int",
            Self::Status => "",
            Self::Best => "Fastest",
            Self::Last => "Last",
            Self::LapDiff => "DIFF",
            Self::Category => "CLASS",
            Self::Speed => "SPD",
        }
    }

    fn width(self, cfg: &HudConfig) -> i32 {
        RelField::width(self, cfg)
    }

    fn is_name(self) -> bool {
        matches!(self, Self::Name)
    }

    fn is_bike(self) -> bool {
        matches!(self, Self::Bike)
    }

    fn is_pos(self) -> bool {
        matches!(self, Self::Pos)
    }

    fn is_status(self) -> bool {
        matches!(self, Self::Status)
    }
}
pub(crate) fn table_ink(mode: TableText) -> (Color, Color, Color, Color) {
    match mode {
        TableText::White => (
            text_col(),
            Color::from_rgba8(210, 210, 216, 255),
            Color::from_rgba8(110, 110, 116, 255),
            Color::from_rgba8(160, 160, 168, 220),
        ),
        TableText::Black => (
            Color::from_rgba8(16, 16, 18, 255),
            Color::from_rgba8(48, 48, 54, 255),
            Color::from_rgba8(90, 90, 98, 255),
            Color::from_rgba8(64, 64, 72, 220),
        ),
    }
}

pub(crate) fn plaque_ink(mode: TableText) -> Color {
    match mode {
        TableText::White => text_col(),
        TableText::Black => Color::from_rgba8(12, 12, 14, 255),
    }
}

pub(crate) const BIKE_BAR_W: f32 = 5.0;

pub(crate) const BIKE_BAR_SKEW: f32 = 3.0;

pub(crate) const BIKE_BAR_PAD: f32 = 5.0;

pub(crate) const BIKE_PILL_PAD_X: f32 = 10.0;

pub(crate) const BIKE_PILL_PAD_Y: f32 = 4.0;

pub(crate) fn bike_bar_end(pos_cx: f32, pos_cw: f32) -> f32 {
    pos_cx + pos_cw + 1.0 + BIKE_BAR_W + BIKE_BAR_SKEW
}

pub(crate) fn name_left_pad(cx: f32, bar_end: Option<f32>) -> f32 {
    bar_end
        .map(|end| (end + BIKE_BAR_PAD - cx).max(0.0))
        .unwrap_or(0.0)
}

pub(crate) fn draw_bike_pill(
    px: &mut Pixmap,
    fonts: &Fonts,
    label: &str,
    cx: f32,
    cy: f32,
    cw: f32,
    row_h: f32,
    accent_c: Color,
) {
    let font_sz = 9.0;
    let max_inner = (cw - BIKE_PILL_PAD_X * 2.0).max(8.0);
    let badge = ellipsize(fonts, label, font_sz, max_inner);
    let tw = measure(fonts, &badge, font_sz);
    let bw = (tw + BIKE_PILL_PAD_X * 2.0).min(cw);
    let bh = font_sz + BIKE_PILL_PAD_Y * 2.0;
    let bx = cx + ((cw - bw) * 0.5).max(0.0);
    let by = cy + ((row_h - bh) * 0.5).max(0.0);
    // Same alphas as a default rider-row wash and its stronger edge.
    let wash = 52u8;
    let border_a = (wash as u32 * 3).min(255) as u8;
    fill_round(px, bx, by, bw, bh, 4.0, color_alpha(accent_c, border_a));
    let inset = 1.0;
    if bw > inset * 2.0 && bh > inset * 2.0 {
        fill_round(
            px,
            bx + inset,
            by + inset,
            bw - inset * 2.0,
            bh - inset * 2.0,
            3.0,
            color_alpha(accent_c, wash),
        );
    }
    text(
        px,
        fonts,
        &badge,
        font_sz,
        bx + BIKE_PILL_PAD_X,
        by + BIKE_PILL_PAD_Y - 1.0,
        ink_on(accent_c),
        false,
    );
}
pub(crate) struct TableLook<'a> {
    pub(crate) bg: i32,
    pub(crate) hl: i32,
    pub(crate) text: TableText,
    pub(crate) stripe: bool,
    pub(crate) plaque_text: TableText,
    pub(crate) show_plaque: bool,
    pub(crate) head: &'a [BoardField; 3],
    pub(crate) foot: &'a [BoardField; 3],
}

pub(crate) struct TableBoard<'a, C: BoardCol> {
    px: &'a mut Pixmap,
    fonts: &'a Fonts,
    s: &'a Snapshot,
    cfg: &'a HudConfig,
    slots: Vec<(C, f32, f32)>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    row_h: f32,
    body_y: f32,
    foot_h: f32,
    a: u8,
    you_bg: Color,
    stripe_c: Color,
    stripe: bool,
    bar_end: Option<f32>,
    ink: Color,
    ink_dim: Color,
    out_c: Color,
    foot: &'a [BoardField; 3],
}

pub(crate) struct TableRow<'a, C: BoardCol> {
    pub(crate) vis_i: usize,
    cy: f32,
    x: f32,
    w: f32,
    row_h: f32,
    slots: &'a [(C, f32, f32)],
    bar_end: Option<f32>,
    you_bg: Color,
    pub(crate) ink: Color,
    pub(crate) ink_dim: Color,
    pub(crate) out_c: Color,
}

pub(crate) fn plaque_cap_a(a: u8) -> u8 {
    ((a as u16 * 240) / 200).min(255) as u8
}

pub(crate) fn draw_table_board<C: BoardCol>(
    px: &mut Pixmap,
    fonts: &Fonts,
    s: &Snapshot,
    cfg: &HudConfig,
    rect: crate::shm::Rect,
    sw: f32,
    sh: f32,
    cols: &[C],
    look: TableLook<'_>,
    vis_rows: usize,
    count_n: usize,
    empty: Option<&'static str>,
    body: impl FnOnce(&mut TableBoard<'_, C>),
) {
    let x = rect.x * sw;
    let y = rect.y * sh;
    let max_w = rect.w * sw;
    let k = style_k();
    let head_h = 26.0 * k;
    let col_h = 16.0 * k;
    let track_h = if look.show_plaque { 20.0 * k } else { 0.0 };
    let row_h = 22.0 * k;
    let foot_h = if BoardField::any(look.foot) {
        20.0 * k
    } else {
        0.0
    };
    let h = table_stack_h(k, vis_rows, BoardField::any(look.foot), look.show_plaque);
    let pad = 8.0;
    let slots = col_slots(
        x,
        pad,
        max_w,
        cols,
        |c| c.width(cfg) as f32,
        |c| c.is_name(),
    );
    let w = hug_board_w(x, pad, max_w, &slots);
    let a = background_alpha(look.bg);
    if a > 0 {
        fill_round(px, x, y, w, h, 6.0, Color::from_rgba8(8, 8, 10, a));
        fill_round(
            px,
            x,
            y,
            w,
            head_h,
            6.0,
            Color::from_rgba8(4, 4, 6, plaque_cap_a(a)),
        );
        if let Some(rrt) = try_rect(x, y + head_h - 6.0, w, 6.0) {
            fill_rect(px, rrt, Color::from_rgba8(4, 4, 6, plaque_cap_a(a)));
        }
    }
    draw_board_bar(px, fonts, s, cfg, look.head, x, y, w, head_h);
    if let Some(msg) = empty {
        text(
            px,
            fonts,
            msg,
            12.0,
            x + 12.0,
            y + head_h + 10.0,
            text_dim(),
            false,
        );
        paint_table_footer(px, fonts, s, cfg, look.foot, x, y, w, h, foot_h, a);
        return;
    }

    let (ink, ink_dim, out_c, hdr_c) = table_ink(look.text);
    let bar_end = slots
        .iter()
        .find(|(c, _, _)| c.is_pos())
        .map(|(_, cx, cw)| bike_bar_end(*cx, *cw));
    let mut cy = y + head_h;
    if look.show_plaque {
        let track = {
            let t = bytes_as_text(&s.track_name);
            if t.is_empty() {
                "TRACK".into()
            } else {
                t.to_uppercase()
            }
        };
        draw_count_track(
            px,
            fonts,
            x,
            cy,
            count_n,
            &track,
            plaque_ink(look.plaque_text),
        );
        if let Some(line) = try_rect(x + 8.0, cy + 18.0, w - 16.0, 1.2) {
            fill_rect(px, line, accent());
        }
        cy += track_h;
    }
    let hdr_y = cy + 2.0;
    for (col, cx, cw) in &slots {
        let right = !col.is_name() && !col.is_bike();
        let pad = if col.is_name() {
            name_left_pad(*cx, bar_end)
        } else {
            0.0
        };
        col_text(
            px,
            fonts,
            col.header(),
            10.0,
            *cx + pad,
            (*cw - pad).max(8.0),
            hdr_y,
            hdr_c,
            right,
        );
    }
    cy += col_h;

    let mut board = TableBoard {
        px,
        fonts,
        s,
        cfg,
        slots,
        x,
        y,
        w,
        h,
        row_h,
        body_y: cy,
        foot_h,
        a,
        you_bg: you_row_bg(look.hl),
        stripe_c: stripe_row_bg(a),
        stripe: look.stripe,
        bar_end,
        ink,
        ink_dim,
        out_c,
        foot: look.foot,
    };
    body(&mut board);
    paint_table_footer(
        board.px,
        board.fonts,
        board.s,
        board.cfg,
        board.foot,
        board.x,
        board.y,
        board.w,
        board.h,
        board.foot_h,
        board.a,
    );
}

pub(crate) fn paint_table_footer(
    px: &mut Pixmap,
    fonts: &Fonts,
    s: &Snapshot,
    cfg: &HudConfig,
    foot: &[BoardField; 3],
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    foot_h: f32,
    a: u8,
) {
    if foot_h <= 0.0 {
        return;
    }
    if a > 0 {
        if let Some(rrt) = try_rect(x, y + h - foot_h, w, foot_h) {
            fill_rect(px, rrt, Color::from_rgba8(4, 4, 6, plaque_cap_a(a)));
        }
    }
    draw_board_bar(px, fonts, s, cfg, foot, x, y + h - foot_h, w, foot_h);
}

impl<C: BoardCol> TableBoard<'_, C> {
    pub(crate) fn rows(
        &mut self,
        ids: &[i32],
        slides: &mut TableSlides,
        mut each: impl FnMut(&mut Pixmap, &Fonts, TableRow<'_, C>),
    ) {
        let row_ys = slides.step(ids, self.body_y, self.row_h, anim_now());
        let x = self.x;
        let w = self.w;
        let row_h = self.row_h;
        let body_y = self.body_y;
        let a = self.a;
        let stripe_c = self.stripe_c;
        let bar_end = self.bar_end;
        let you_bg = self.you_bg;
        let ink = self.ink;
        let ink_dim = self.ink_dim;
        let out_c = self.out_c;
        if self.stripe && a > 0 {
            for vis_i in 0..ids.len() {
                if vis_i % 2 == 1 {
                    fill_focus_row(
                        self.px,
                        x,
                        body_y + vis_i as f32 * row_h,
                        w,
                        row_h,
                        stripe_c,
                    );
                }
            }
        }
        let px = &mut *self.px;
        let fonts = self.fonts;
        let slots = self.slots.as_slice();
        for vis_i in 0..ids.len() {
            each(
                px,
                fonts,
                TableRow {
                    vis_i,
                    cy: row_ys[vis_i],
                    x,
                    w,
                    row_h,
                    slots,
                    bar_end,
                    you_bg,
                    ink,
                    ink_dim,
                    out_c,
                },
            );
        }
    }
}

impl<C: BoardCol> TableRow<'_, C> {
    pub(crate) fn fill_you(&self, px: &mut Pixmap) {
        fill_fade_row(px, self.x, self.cy, self.w, self.row_h, self.you_bg);
    }

    pub(crate) fn fill(&self, px: &mut Pixmap, c: Color) {
        fill_fade_row(px, self.x, self.cy, self.w, self.row_h, c);
    }

    pub(crate) fn paint(
        &self,
        px: &mut Pixmap,
        fonts: &Fonts,
        accent: Color,
        click: Option<i32>,
        mut cell: impl FnMut(C) -> (String, Color, bool),
    ) {
        paint_table_row(px, fonts, self, accent, click, &mut cell);
    }
}

pub(crate) fn paint_table_row<C: BoardCol>(
    px: &mut Pixmap,
    fonts: &Fonts,
    row: &TableRow<'_, C>,
    accent: Color,
    click: Option<i32>,
    cell: &mut impl FnMut(C) -> (String, Color, bool),
) {
    let mut named = false;
    for (kind, cx, cw) in row.slots {
        if kind.is_pos() {
            fill_skew(
                px,
                *cx + *cw + 1.0,
                row.cy + 4.0,
                BIKE_BAR_W,
                row.row_h - 8.0,
                BIKE_BAR_SKEW,
                accent,
            );
        }
        let (val, color, right) = cell(*kind);
        let pad = if kind.is_name() {
            name_left_pad(*cx, row.bar_end)
        } else {
            0.0
        };
        if kind.is_name() {
            named = true;
            if let Some(num) = click {
                push_click_rider(num, *cx + pad, row.cy, (*cw - pad).max(8.0), row.row_h);
            }
        }
        if kind.is_bike() && !val.is_empty() {
            draw_bike_pill(px, fonts, &val, *cx, row.cy, *cw, row.row_h, accent);
        } else if kind.is_status() {
            let mark = mark_from_key(&val);
            if !matches!(mark, RiderMark::None) {
                let r = (row.row_h * 0.32).clamp(7.0, 11.0);
                draw_state_mark(
                    px,
                    fonts,
                    *cx + (*cw - r * 2.0) * 0.5,
                    row.cy + (row.row_h - r * 2.0) * 0.5,
                    r,
                    mark,
                );
            }
        } else if kind.is_pos() {
            let star = click.and_then(|num| place_star_col(penalty_place_delta(num)));
            col_place_text(
                px,
                fonts,
                &val,
                12.0,
                *cx + pad,
                (*cw - pad).max(8.0),
                row.cy + 4.0,
                color,
                star,
                right,
            );
        } else {
            col_text(
                px,
                fonts,
                &val,
                12.0,
                *cx + pad,
                (*cw - pad).max(8.0),
                row.cy + 4.0,
                color,
                right,
            );
        }
    }
    if !named {
        if let Some(num) = click {
            push_click_rider(num, row.x, row.cy, row.w, row.row_h);
        }
    }
}
