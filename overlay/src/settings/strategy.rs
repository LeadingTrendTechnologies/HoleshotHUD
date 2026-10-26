//! F8 Profile → Strategy. Practice burn and pace, then a tank fill for the race.

use super::*;
use crate::config::UnitPrefs;
use mxbo_review::strategy::{FillAdvice, RaceLength, StrategyPage};

pub(crate) fn pane_strategy(
    px: &mut Pixmap,
    fonts: &Fonts,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
    units: UnitPrefs,
) -> f32 {
    let page = mxbo_review::strategy::page();
    let mut y = y + 8.0;
    text(
        px,
        fonts,
        mxbo_hud::i18n::t("Strategy"),
        22.0,
        x,
        y,
        text_col(),
        false,
    );
    y += 32.0;

    if page.track.is_empty() {
        text(
            px,
            fonts,
            mxbo_hud::i18n::t("Practice a track, then open this page before you fill the tank."),
            14.0,
            x,
            y,
            text_col(),
            false,
        );
        return y + 28.0;
    }

    text(px, fonts, &place_line(&page), 14.0, x, y, text_col(), false);
    y += 28.0;

    if page.ready {
        if let Some(advice) = &page.advice {
            let fill = units.format_fuel(
                advice.recommended_liters,
                advice.max_fuel.max(advice.recommended_liters),
            );
            let headline = mxbo_hud::i18n::t_fmt("Put {fuel} in the tank.", &[("fuel", &fill)]);
            text(px, fonts, &headline, 28.0, x, y, accent(), false);
            y += 40.0;
            text(px, fonts, garage_line(advice), 13.0, x, y, dim(), false);
            y += 22.0;
        }
    } else if page.laps.is_empty() {
        text(
            px,
            fonts,
            mxbo_hud::i18n::t("No full laps yet. Finish a practice or warmup lap without pitting."),
            14.0,
            x,
            y,
            dim(),
            false,
        );
        y += 28.0;
    } else {
        text(px, fonts, &progress_line(page.laps.len()), 14.0, x, y, text_col(), false);
        y += 28.0;
    }

    y += 8.0;
    let mut cx = x;
    cx += profile::profile_chip(
        px,
        fonts,
        cx,
        y,
        mxbo_hud::i18n::t("Laps"),
        page.laps_mode,
        Hit::StrategyLaps,
        hover,
        hits,
    ) + 8.0;
    profile::profile_chip(
        px,
        fonts,
        cx,
        y,
        mxbo_hud::i18n::t("Minutes"),
        !page.laps_mode,
        Hit::StrategyMinutes,
        hover,
        hits,
    );
    y += 40.0;
    let value = match page.length {
        Some(RaceLength::Laps(n) | RaceLength::Minutes(n)) => n.to_string(),
        None => "—".into(),
    };
    let label = if page.laps_mode { "Laps" } else { "Minutes" };
    y = stepper_row(
        px,
        fonts,
        x,
        y,
        w,
        label,
        &value,
        Hit::StrategyDec,
        Hit::StrategyInc,
        hover,
        hits,
    );
    if page.from_last_race {
        text(
            px,
            fonts,
            mxbo_hud::i18n::t("From the last race here."),
            12.0,
            x,
            y,
            dim(),
            false,
        );
        y += 22.0;
    }

    if !page.ready {
        return y;
    }
    let pace = page.advice.clone().or_else(|| {
        mxbo_review::strategy::advise(&page.laps, RaceLength::Laps(1), page.max_fuel)
    });
    let Some(advice) = pace else {
        return y;
    };
    y = pace_card(px, fonts, x, y, w, units, &advice);
    quiet_notes(px, fonts, x, y, &advice, &page, page.advice.is_some())
}

fn garage_line(advice: &FillAdvice) -> &'static str {
    if advice.timed {
        mxbo_hud::i18n::t("Type that in the garage. Includes about 15% extra.")
    } else {
        mxbo_hud::i18n::t("Type that in the garage. Includes an extra lap and a half.")
    }
}

fn pace_card(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    units: UnitPrefs,
    advice: &FillAdvice,
) -> f32 {
    let per_lap = format!(
        "{}/lap",
        units.format_fuel(advice.liters_per_lap, advice.max_fuel.max(advice.liters_per_lap))
    );
    let per_min = format!(
        "{}/min",
        units.format_fuel(
            advice.liters_per_minute,
            advice.max_fuel.max(advice.liters_per_minute)
        )
    );
    let mut cells = vec![
        ("Median", format_clock(advice.median_lap_ms)),
        ("Best", format_clock(advice.best_lap_ms)),
        ("Spread", format_clock(advice.spread_ms)),
        ("Per lap", per_lap),
        ("Per minute", per_min),
    ];
    if let Some(clean) = advice.clean_spread_ms {
        cells.push(("Without the slow lap", format_clock(clean)));
    }
    let rows = cells.len().div_ceil(2);
    let row_h = 28.0;
    let h = 16.0 + rows as f32 * row_h;
    row_card(px, x, y, w, h, false);
    let col_w = (w - 32.0) * 0.5;
    for (index, (label, value)) in cells.iter().enumerate() {
        let col = (index % 2) as f32;
        let row = (index / 2) as f32;
        let left = x + 16.0 + col * col_w;
        let baseline = y + 18.0 + row * row_h;
        text(px, fonts, mxbo_hud::i18n::t(label), 12.0, left, baseline, dim(), false);
        let vw = measure(fonts, value, 13.0);
        text(
            px,
            fonts,
            value,
            13.0,
            left + col_w - vw - 12.0,
            baseline,
            text_col(),
            false,
        );
    }
    y + h + ROW_GAP
}

fn quiet_notes(
    px: &mut Pixmap,
    fonts: &Fonts,
    x: f32,
    mut y: f32,
    advice: &FillAdvice,
    page: &StrategyPage,
    show_fill_notes: bool,
) -> f32 {
    y = note_line(px, fonts, x, y, &tank_line(advice));
    y = note_line(px, fonts, x, y, &recent_line(advice));
    y = note_line(px, fonts, x, y, &last_race_line(advice, page.last_race_best_ms));
    if show_fill_notes {
        if advice.used_push {
            y = note_line(
                px,
                fonts,
                x,
                y,
                mxbo_hud::i18n::t("Fast laps burn more, so the fill uses those."),
            );
        }
        if let Some(lap) = advice.dry_at_lap {
            let line = mxbo_hud::i18n::t_fmt(
                "A full tank runs dry on lap {n}.",
                &[("n", &lap.to_string())],
            );
            y = note_line(px, fonts, x, y, &line);
        } else if let Some(minute) = advice.dry_at_minute {
            let line = mxbo_hud::i18n::t_fmt(
                "A full tank runs dry around minute {n}.",
                &[("n", &minute.to_string())],
            );
            y = note_line(px, fonts, x, y, &line);
        } else if advice.short_margin {
            let line = if advice.timed {
                mxbo_hud::i18n::t("The race fits. The extra 15% does not.")
            } else {
                mxbo_hud::i18n::t("The race fits. The extra lap and a half does not.")
            };
            y = note_line(px, fonts, x, y, line);
        }
        if let Some(ms) = advice.projected_race_ms {
            let line = mxbo_hud::i18n::t_fmt(
                "About {time} at this pace.",
                &[("time", &format_clock(ms))],
            );
            y = note_line(px, fonts, x, y, &line);
        }
        if let Some(laps) = advice.timed_laps {
            let line = mxbo_hud::i18n::t_fmt(
                "About {n} laps at this pace.",
                &[("n", &laps.to_string())],
            );
            y = note_line(px, fonts, x, y, &line);
        }
    }
    if let Some(pending) = page.warmup_pending {
        y = note_line(px, fonts, x, y, &warmup_pending_line(pending));
    }
    y
}

fn note_line(px: &mut Pixmap, fonts: &Fonts, x: f32, y: f32, line: &str) -> f32 {
    if line.is_empty() {
        return y;
    }
    text(px, fonts, line, 13.0, x, y, text_col(), false);
    y + 22.0
}

fn tank_line(advice: &FillAdvice) -> String {
    let (Some(laps), Some(minutes)) = (advice.tank_laps, advice.tank_minutes) else {
        return String::new();
    };
    let laps_n = laps.floor() as i32;
    let mins = minutes.round().max(0.0) as i32;
    let laps_n = laps_n.max(0);
    if advice.used_push {
        mxbo_hud::i18n::t_fmt(
            "A full tank covers {laps} laps, about {min} min, at your fast laps.",
            &[("laps", &laps_n.to_string()), ("min", &mins.to_string())],
        )
    } else {
        mxbo_hud::i18n::t_fmt(
            "A full tank covers {laps} laps, about {min} min.",
            &[("laps", &laps_n.to_string()), ("min", &mins.to_string())],
        )
    }
}

fn recent_line(advice: &FillAdvice) -> String {
    let Some(delta) = advice.recent_delta_ms else {
        return String::new();
    };
    let time = format_gap(delta.abs());
    if delta < 0 {
        mxbo_hud::i18n::t_fmt("Last three are {time} quicker.", &[("time", &time)])
    } else {
        mxbo_hud::i18n::t_fmt("Last three are {time} slower.", &[("time", &time)])
    }
}

fn last_race_line(advice: &FillAdvice, race_best_ms: Option<i32>) -> String {
    let Some(race_best) = race_best_ms else {
        return String::new();
    };
    let Some(delta) = mxbo_review::strategy::race_gap_ms(advice.median_lap_ms, race_best) else {
        return String::new();
    };
    let time = format_gap(delta.abs());
    if delta < 0 {
        mxbo_hud::i18n::t_fmt(
            "Practice median is {time} inside your last race here.",
            &[("time", &time)],
        )
    } else {
        mxbo_hud::i18n::t_fmt(
            "Practice median is {time} off your last race here.",
            &[("time", &time)],
        )
    }
}

fn format_gap(ms: i32) -> String {
    if ms < 10_000 {
        format!("{:.1}s", ms as f32 / 1000.0)
    } else {
        format_clock(ms)
    }
}

fn progress_line(laps: usize) -> String {
    let n = laps.to_string();
    if laps + 1 >= 3 {
        mxbo_hud::i18n::t_fmt("{n} of 3 laps. One more for the fill.", &[("n", &n)])
    } else {
        let left = (3 - laps).to_string();
        mxbo_hud::i18n::t_fmt(
            "{n} of 3 laps. {left} more for the fill.",
            &[("n", &n), ("left", &left)],
        )
    }
}

fn warmup_pending_line(laps: usize) -> String {
    let n = laps.to_string();
    if laps + 1 >= 3 {
        mxbo_hud::i18n::t_fmt(
            "Warmup has {n} laps. One more and the fill uses warmup.",
            &[("n", &n)],
        )
    } else {
        let left = (3 - laps).to_string();
        mxbo_hud::i18n::t_fmt(
            "Warmup has {n} laps. {left} more and the fill uses warmup.",
            &[("n", &n), ("left", &left)],
        )
    }
}

fn place_line(page: &StrategyPage) -> String {
    let mut parts = vec![page.track.clone()];
    if !page.class_name.is_empty() {
        parts.push(page.class_name.clone());
    }
    if !page.setup.is_empty() {
        parts.push(page.setup.clone());
    }
    if page.from_warmup {
        parts.push(mxbo_hud::i18n::t("Warmup").to_string());
    }
    parts.join("  ·  ")
}

fn format_clock(ms: i32) -> String {
    if ms <= 0 {
        return "0.000".into();
    }
    let sec = ms as f32 / 1000.0;
    if sec >= 60.0 {
        let m = (sec / 60.0) as i32;
        format!("{m}:{:06.3}", sec - m as f32 * 60.0)
    } else {
        format!("{sec:.3}")
    }
}
