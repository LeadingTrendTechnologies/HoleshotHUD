#![allow(unused_imports)]
use super::*;

pub(crate) fn draw_relative(px: &mut Pixmap, fonts: &Fonts, s: &Snapshot, cfg: &HudConfig, sw: f32, sh: f32) {
    let n = s.rider_count.max(0) as usize;
    let focus = if s.focus_race_num > 0 { s.focus_race_num } else { s.local_race_num };
    let mut focus_pos = s.local_track_pos;
    let mut have = s.has_telemetry != 0;
    let mut uniq: Vec<usize> = Vec::new();
    for i in 0..n {
        let rider = &s.riders[i];
        let empty = rider.race_num <= 0 && cstr(&rider.name).is_empty();
        if empty {
            continue;
        }
        if rider.race_num > 0 && uniq.iter().any(|&j| s.riders[j].race_num == rider.race_num) {
            continue;
        }
        if rider.race_num == focus {
            focus_pos = rider.track_pos;
            have = true;
        }
        uniq.push(i);
    }
    let side = s.relative_count.max(1) as usize;
    let vis = if have { uniq.len().min(side * 2 + 1).max(1) } else { 1 };
    let cols = cfg.relative_cols();
    let empty = if !have || uniq.is_empty() {
        Some("Waiting for positions")
    } else {
        None
    };
    draw_table_board(
        px,
        fonts,
        s,
        cfg,
        s.relative,
        sw,
        sh,
        &cols,
        TableLook {
            bg: cfg[WidgetId::Relative].bg,
            hl: cfg.rel_hl,
            text: cfg.rel_text,
            stripe: cfg.rel_stripe,
            head: &cfg.rel_head,
            foot: &cfg.rel_foot,
        },
        vis,
        n,
        empty,
        |tbl| {
            let mut order: Vec<(usize, f32)> = uniq
                .iter()
                .map(|&i| (i, wrap_frac(s.riders[i].track_pos, focus_pos)))
                .collect();
            order.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            let self_idx = order
                .iter()
                .position(|(i, _)| s.riders[*i].race_num == focus)
                .unwrap_or(0);
            let behind = self_idx.min(side);
            let ahead = (order.len() - self_idx - 1).min(side);
            let mut show = Vec::new();
            for k in (1..=ahead).rev() {
                show.push(self_idx + k);
            }
            show.push(self_idx);
            for k in 1..=behind {
                show.push(self_idx - k);
            }

            let purple = Color::from_rgba8(196, 112, 255, 255);
            RaceStore::with(|race| {
            let best_ms = if race.field.session_best_ms > 0 {
                race.field.session_best_ms
            } else {
                s.standings
                    .iter()
                    .take(s.standing_count.max(0) as usize)
                    .map(|row| row.best_lap_ms)
                    .filter(|ms| *ms > 0)
                    .min()
                    .unwrap_or(0)
            };
            let track_len = if s.track_length > 1.0 { s.track_length } else { 1.0 };
            let speed = s.local_speed.max(4.0);
            let ids = row_ids(show.iter().map(|oi| s.riders[order[*oi].0].race_num));
            let hl = cfg.rel_hl;
            REL_SLIDE.with(|a| {
                tbl.rows(&ids, &mut a.borrow_mut(), |px, fonts, row| {
                    let oi = show[row.vis_i];
                    let (ri, wrapped) = order[oi];
                    let rider = &s.riders[ri];
                    let is_self = rider.race_num == focus;
                    let race_row = race.field.row_by_num(rider.race_num);
                    let st = race_row.map(|r| &r.standing).or_else(|| standing_of(s, rider.race_num));
                    let cat = st.map(|r| cstr(&r.category)).unwrap_or_default();
                    let bike_name = st.map(|r| cstr(&r.bike)).unwrap_or_default();
                    let accent_c = bike_color(&bike_name, &cat);
                    let out = rider.crashed != 0
                        || st.is_some_and(|r| r.crashed != 0 || matches!(r.state, 1 | 3 | 4));
                    if is_self {
                        row.fill_you(px);
                    } else if let Some(bg) = lap_row_bg(lap_rel(s, rider.race_num), hl) {
                        row.fill(px, bg);
                    }
                    let name_c = if out { row.out_c } else { row.ink };
                    let dim = if out { row.out_c } else { row.ink_dim };
                    let pos = st.map(|r| r.position).unwrap_or(0);
                    let best = st.map(|r| r.best_lap_ms).unwrap_or(0);
                    let last_ms = if st.map(|r| r.last_lap_ms).unwrap_or(0) > 0 {
                        st.map(|r| r.last_lap_ms).unwrap_or(0)
                    } else if is_self {
                        s.last_lap_ms
                    } else {
                        0
                    };
                    row.paint(px, fonts, accent_c, None, |kind| match kind {
                        RelField::Pos => (
                            if pos > 0 { format!("{pos}") } else { String::new() },
                            name_c,
                            true,
                        ),
                        RelField::Num => (format!("{}", rider.race_num), dim, true),
                        RelField::Name => (cstr(&rider.name).to_string(), name_c, false),
                        RelField::Bike => (bike_name.to_string(), name_c, false),
                        RelField::Gap => (
                            if is_self {
                                "0.0".into()
                            } else {
                                format!("{:.1}", ((wrapped * track_len) / speed).abs())
                            },
                            dim,
                            true,
                        ),
                        RelField::Laps => (
                            st.map(|r| {
                                format!("{}", standing_num_laps(s, rider.race_num, r.num_laps))
                            })
                            .unwrap_or_default(),
                            dim,
                            true,
                        ),
                        RelField::Current => (
                            format!(
                                "{}",
                                race_row
                                    .map(|r| r.current_lap)
                                    .unwrap_or_else(|| {
                                        rider_current_lap(s, rider.race_num, st.map(|r| r.num_laps).unwrap_or(0))
                                    })
                            ),
                            dim,
                            true,
                        ),
                        RelField::Penalty => (st.map(|r| format_penalty(r.penalty_ms)).unwrap_or_default(), dim, true),
                        RelField::Interval => (
                            race_row
                                .map(interval_text_from_row)
                                .or_else(|| st.map(|r| interval_text(s, r)))
                                .unwrap_or_default(),
                            dim,
                            true,
                        ),
                        RelField::Crashed => {
                            if rider.crashed != 0 || st.is_some_and(|r| r.crashed != 0) {
                                ("CRASH".into(), behind_col(), true)
                            } else {
                                (String::new(), dim, true)
                            }
                        }
                        RelField::Best => (
                            format_lap(best),
                            if best_ms > 0 && best == best_ms && !out { purple } else { dim },
                            true,
                        ),
                        RelField::Last => (format_lap(last_ms), dim, true),
                    });
                });
            });
            });
        },
    );
}
