#![allow(unused_imports)]
use super::*;

pub(crate) fn draw_standings(px: &mut Pixmap, fonts: &Fonts, s: &Snapshot, cfg: &HudConfig, sw: f32, sh: f32) {
    RaceStore::with(|race| {
    // Live order and live places, so a pass moves a row now instead of at the line.
    let mut board = race.field.board();
    if board.is_empty() {
        let count = (s.standing_count.max(0) as usize).min(MAX_SAFE);
        board.extend_from_slice(&s.standings[..count]);
    }
    let rows = board.len();
    let max_rows = s.standings_rows.max(3) as usize;
    let n = rows.min(MAX_SAFE);
    let focus = s.focus_race_num;
    let mut start = 0;
    if n > max_rows {
        let fi = (0..n).find(|&i| board[i].race_num == focus).unwrap_or(0);
        start = fi.saturating_sub(max_rows / 2).min(n.saturating_sub(max_rows));
    }
    let end = (start + max_rows).min(n);
    let slice = &board[start..end];
    let cols = cfg.standings_cols();
    let empty = if rows == 0 { Some("Waiting for race data") } else { None };
    draw_table_board(
        px,
        fonts,
        s,
        cfg,
        s.standings_rect,
        sw,
        sh,
        &cols,
        TableLook {
            bg: cfg[WidgetId::Standings].bg,
            hl: cfg.st_hl,
            text: cfg.st_text,
            stripe: cfg.st_stripe,
            head: &cfg.st_head,
            foot: &cfg.st_foot,
        },
        slice.len().max(1),
        n,
        empty,
        |tbl| {
            let purple = Color::from_rgba8(196, 112, 255, 255);
            let best_ms = if race.field.session_best_ms > 0 {
                race.field.session_best_ms
            } else {
                board
                    .iter()
                    .map(|row| row.best_lap_ms)
                    .filter(|ms| *ms > 0)
                    .min()
                    .unwrap_or(0)
            };
            let ids = row_ids(slice.iter().map(|row| row.race_num));
            ST_SLIDE.with(|a| {
                tbl.rows(&ids, &mut a.borrow_mut(), |px, fonts, row| {
                    let standing = &slice[row.vis_i];
                    let cat = cstr(&standing.category);
                    let accent_c = bike_color(&cstr(&standing.bike), &cat);
                    let is_focus = standing.race_num == focus;
                    let out = standing_status(standing).is_some() && standing_status(standing) != Some("PIT");
                    if is_focus {
                        row.fill_you(px);
                    }
                    let name_c = if out { row.out_c } else { row.ink };
                    let dim = if out { row.out_c } else { row.ink_dim };
                    let status = standing_status(standing);
                    row.paint(px, fonts, accent_c, Some(standing.race_num), |kind| match kind {
                        StField::Pos => (format!("{}", standing.position.max(0)), name_c, true),
                        StField::Num => (format!("{}", standing.race_num), dim, true),
                        StField::Name => (cstr(&standing.name).to_string(), name_c, false),
                        StField::Bike => (cstr(&standing.bike).to_string(), name_c, false),
                        StField::Gap => (format_board_gap(standing.gap_ms, standing.gap_laps, standing.position <= 1), dim, true),
                        StField::Interval => {
                            let txt = race
                                .field
                                .row_by_num(standing.race_num)
                                .map(interval_text_from_row)
                                .unwrap_or_else(|| interval_text(s, standing));
                            (txt, dim, true)
                        }
                        StField::Laps => (
                            format!("{}", standing_num_laps(s, standing.race_num, standing.num_laps)),
                            dim,
                            true,
                        ),
                        StField::Current => {
                            let lap = race
                                .field
                                .row_by_num(standing.race_num)
                                .map(|r| r.current_lap)
                                .unwrap_or_else(|| rider_current_lap(s, standing.race_num, standing.num_laps));
                            (format!("{lap}"), dim, true)
                        }
                        StField::Best => (
                            format_lap(standing.best_lap_ms),
                            if best_ms > 0 && standing.best_lap_ms == best_ms && !out { purple } else { dim },
                            true,
                        ),
                        StField::Last => {
                            let ms = if standing.last_lap_ms > 0 {
                                standing.last_lap_ms
                            } else if standing.race_num == focus {
                                s.last_lap_ms
                            } else {
                                0
                            };
                            (format_lap(ms), dim, true)
                        }
                        StField::Status => (status.unwrap_or("").to_string(), dim, true),
                        StField::Penalty => (format_penalty(standing.penalty_ms), dim, true),
                        StField::Crashed => {
                            if standing.crashed != 0 || rider_crashed(s, standing.race_num) {
                                ("CRASH".into(), behind_col(), true)
                            } else {
                                (String::new(), dim, true)
                            }
                        }
                    });
                });
            });
        },
    );
    });
}
