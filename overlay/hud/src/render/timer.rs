use super::*;

/// One line: place and the session clock or lap count, same size.
pub(crate) fn draw_timer(
    px: &mut Pixmap,
    fonts: &Fonts,
    s: &Snapshot,
    cfg: &HudConfig,
    sw: f32,
    sh: f32,
) {
    let slot = cfg[WidgetId::Timer].rect;
    let x = slot.x * sw;
    let y = slot.y * sh;
    let w = (slot.w * sw).max(1.0);
    let h = (slot.h * sh).max(1.0);
    let cut = (h * 0.28).clamp(6.0, 16.0);
    let alpha = background_alpha(cfg[WidgetId::Timer].bg);
    let glass = cfg[WidgetId::Timer].bg < 40;
    if alpha > 0 {
        if let Some(path) = chamfer_path(x, y, w, h, cut) {
            fill_path(px, &path, Color::from_rgba8(18, 18, 20, alpha));
            let stroke = ((alpha as u16 * 200) / 255).max(90) as u8;
            stroke_path(
                px,
                &path,
                Color::from_rgba8(220, 220, 224, stroke),
                1.2,
            );
        }
    }

    let focus_num = if s.focus_race_num > 0 {
        s.focus_race_num
    } else {
        s.local_race_num
    };
    let place = Some(standing_pos(s, focus_num)).filter(|place| *place > 0);
    let lead = place == Some(1);
    let mut place_text = place
        .map(|place| format_place_digits(place, true))
        .unwrap_or_else(|| "-".into());
    if cfg.timer.timer_of {
        let field = if s.standing_count > 0 {
            s.standing_count
        } else {
            s.rider_count
        };
        if field > 0 {
            place_text.push('/');
            place_text.push_str(&field.to_string());
        }
    }
    let star = place_star_col(penalty_place_delta(focus_num));
    let clock = race_progress_text(s);
    let lapped = lapped(s) && !clock.is_empty();

    let pad_x = (h * 0.28).max(8.0);
    let avail = (w - pad_x * 2.0).max(1.0);
    let mut size = if lead { h * 0.46 } else { h * 0.56 };
    let mut gap = size * 0.52;
    let fitted = timer_row_width(fonts, &place_text, &clock, size, star, lapped, gap);
    if fitted > avail {
        let scale = avail / fitted;
        size *= scale;
        gap *= scale;
    }
    let tag_size = size * 0.72;
    let text_y = if lead {
        y + h * 0.36
    } else {
        y + (h - size) * 0.42
    };
    let row_width = timer_row_width(fonts, &place_text, &clock, size, star, lapped, gap);
    let origin = x + ((w - row_width) * 0.5).max(pad_x * 0.35);

    if lead {
        let crown = (size * 0.38).clamp(10.0, 18.0);
        let place_span = place_width(fonts, &place_text, size, star);
        icon(
            px,
            fonts,
            '\u{f521}',
            crown,
            origin + place_span * 0.5,
            text_y - crown * 0.72,
            Color::from_rgba8(255, 196, 48, 255),
            true,
        );
    }
    text_halo(
        px,
        fonts,
        &place_text,
        size,
        origin,
        text_y,
        dash_pos_col(),
        false,
        glass,
    );
    if let Some(star_col) = star {
        text_halo(
            px,
            fonts,
            "*",
            size,
            origin + measure(fonts, &place_text, size),
            text_y,
            star_col,
            false,
            glass,
        );
    }
    let mut cursor = origin + place_width(fonts, &place_text, size, star);
    if !clock.is_empty() {
        cursor += gap;
        text_halo(
            px,
            fonts,
            &clock,
            size,
            cursor,
            text_y,
            Color::from_rgba8(248, 248, 250, 255),
            false,
            glass,
        );
        cursor += measure(fonts, &clock, size);
    }
    if lapped {
        let tag_y = text_y + (size - tag_size) * 0.72;
        text_halo(
            px,
            fonts,
            LAPPED_TAG,
            tag_size,
            cursor + LAPPED_GAP,
            tag_y,
            dash_lapped_col(),
            false,
            glass,
        );
    }
}

fn timer_row_width(
    fonts: &Fonts,
    place_text: &str,
    clock: &str,
    size: f32,
    star: Option<Color>,
    lapped: bool,
    gap: f32,
) -> f32 {
    let mut width = place_width(fonts, place_text, size, star);
    if !clock.is_empty() {
        width += gap + measure(fonts, clock, size);
    }
    if lapped {
        width += LAPPED_GAP + measure(fonts, LAPPED_TAG, size * 0.72);
    }
    width
}
