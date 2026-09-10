#![allow(unused_imports)]
use super::*;

pub(crate) fn draw_reply(
    px: &mut Pixmap,
    fonts: &Fonts,
    win_w: f32,
    win_h: f32,
    view: &crate::feedback::ReplyView,
    hover: Option<Hit>,
    scroll: f32,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let scrim = Color::from_rgba8(8, 8, 10, 204);
    if let Some(r) = Rect::from_xywh(0.0, 0.0, win_w, win_h) {
        fill_rect(px, r, scrim);
    }
    hits.push(HitBox {
        id: Hit::ReplyScrim,
        x: 0.0,
        y: 0.0,
        w: win_w,
        h: win_h,
    });

    let compose = crate::feedback::compose_snapshot();
    let pad = 22.0;
    let plaque_h = 40.0;
    let skew = 6.0;
    let btn_h = 40.0;
    let box_h = 72.0;
    let footer_h = 16.0 + box_h + 10.0 + 18.0 + 8.0 + btn_h + 16.0;
    let panel_w = 520.0_f32.min(win_w - 48.0).max(320.0);
    let inner_w = (panel_w - pad * 2.0).max(200.0);
    let head_size = 16.0;
    let body_size = 12.0;
    let line_h = 18.0;
    let last_dev = view.lines.last().is_some_and(|l| l.from_dev);
    let headline = if last_dev {
        "Can you tell us more?"
    } else {
        "We wrote back"
    };
    let heads = wrap_fb(fonts, headline, inner_w, head_size);
    let wrapped: Vec<(&str, bool, Vec<String>)> = view
        .lines
        .iter()
        .map(|l| {
            (
                if l.from_dev { "Holeshot" } else { "You" },
                l.from_dev,
                wrap_fb(fonts, &l.text, inner_w, body_size),
            )
        })
        .collect();
    let mut body_h = heads.len() as f32 * 22.0 + 10.0;
    for (_, _, lines) in &wrapped {
        body_h += 16.0 + lines.len() as f32 * line_h + 10.0;
    }

    let header_h = pad + plaque_h + 16.0;
    let want = header_h + body_h + footer_h;
    let panel_h = want.min(win_h - 48.0).max(header_h + footer_h + 24.0);
    let panel_x = ((win_w - panel_w) * 0.5).max(16.0);
    let panel_y = ((win_h - panel_h) * 0.5).max(16.0);
    let board = if high_contrast_on() {
        panel()
    } else {
        Color::from_rgba8(20, 20, 22, 255)
    };
    fill_round(px, panel_x, panel_y, panel_w, panel_h, 10.0, board);
    hits.push(HitBox {
        id: Hit::ReplyPanel,
        x: panel_x,
        y: panel_y,
        w: panel_w,
        h: panel_h,
    });

    let plaque_x = panel_x + pad;
    let plaque_y = panel_y + pad;
    let kind_sz = 18.0;
    let kind_w = measure(fonts, view.kind_label, kind_sz);
    let plaque_w = (kind_w + 36.0).min(inner_w).max(72.0);
    fill_skew(px, plaque_x, plaque_y, (plaque_w - skew).max(48.0), plaque_h, skew, accent());
    text(
        px,
        fonts,
        view.kind_label,
        kind_sz,
        plaque_x + 16.0,
        plaque_y + 10.0,
        ink(),
        false,
    );

    let body_top = plaque_y + plaque_h + 16.0;
    let body_bot = panel_y + panel_h - footer_h;
    let view_h = (body_bot - body_top).max(8.0);
    let scroll_max = (body_h - view_h).max(0.0);
    let scroll = scroll.clamp(0.0, scroll_max);

    if view_h > 8.0 && inner_w > 8.0 {
        if let Some(mut body) = Pixmap::new(inner_w.ceil() as u32, view_h.ceil() as u32) {
            body.fill(board);
            let mut y = -scroll;
            for line in &heads {
                if y + 22.0 > 0.0 && y < view_h {
                    text(&mut body, fonts, line, head_size, 0.0, y, text_col(), false);
                }
                y += 22.0;
            }
            y += 10.0;
            for (who, from_dev, lines) in &wrapped {
                if y + 16.0 > 0.0 && y < view_h {
                    text(&mut body, fonts, &who.to_ascii_uppercase(), 10.0, 0.0, y, dim(), false);
                }
                y += 16.0;
                let col = if *from_dev { text_col() } else { muted() };
                for line in lines {
                    if y + line_h > 0.0 && y < view_h {
                        text(&mut body, fonts, line, body_size, 0.0, y, col, false);
                    }
                    y += line_h;
                }
                y += 10.0;
            }
            px.draw_pixmap(
                panel_x as i32 + pad as i32,
                body_top as i32,
                body.as_ref(),
                &PixmapPaint::default(),
                Transform::identity(),
                None,
            );
        }
    }

    if scroll_max > 1.0 && view_h > 16.0 {
        let track_x = panel_x + panel_w - 10.0;
        let thumb_h = (view_h * view_h / body_h.max(view_h)).clamp(16.0, view_h);
        let thumb_y = body_top
            + if scroll_max > 0.0 {
                scroll / scroll_max * (view_h - thumb_h)
            } else {
                0.0
            };
        fill_round(
            px,
            track_x,
            body_top,
            3.0,
            view_h,
            1.5,
            Color::from_rgba8(255, 255, 255, 18),
        );
        fill_round(
            px,
            track_x,
            thumb_y,
            3.0,
            thumb_h,
            1.5,
            Color::from_rgba8(255, 255, 255, 48),
        );
    }

    let box_x = panel_x + pad;
    let box_y = panel_y + panel_h - footer_h + 16.0;
    crate::feedback::set_compose_rect(box_x, box_y, inner_w, box_h);
    hits.push(HitBox {
        id: Hit::ReplyText,
        x: box_x,
        y: box_y,
        w: inner_w,
        h: box_h,
    });
    let box_fill = if compose.focused {
        Color::from_rgba8(20, 20, 24, 255)
    } else {
        btn_bg()
    };
    outlined(px, box_x, box_y, inner_w, box_h, 8.0, box_fill);
    let tx = box_x + 12.0;
    let ty = box_y + 10.0;
    let tw = (inner_w - 24.0).max(40.0);
    if compose.message.is_empty() && !compose.focused {
        text(px, fonts, "Write a reply", 12.0, tx, ty + 2.0, dim(), false);
        crate::feedback::set_caret_layout(tx, ty, 16.0, vec![vec![(0, 0.0)]]);
    } else {
        draw_fb_text(px, fonts, &compose.message, compose.cursor, tx, ty, tw, box_h - 16.0);
    }

    let status_y = box_y + box_h + 8.0;
    let (status, status_c) = match &compose.status {
        crate::feedback::Status::Idle => ("", muted()),
        crate::feedback::Status::Sending => ("Sending…", muted()),
        crate::feedback::Status::Sent => ("Sent.", accent()),
        crate::feedback::Status::Error(msg) => (msg.as_str(), Color::from_rgba8(255, 120, 100, 255)),
    };
    if !status.is_empty() {
        text(px, fonts, status, 11.0, box_x, status_y, status_c, false);
    }

    let btn_y = panel_y + panel_h - 16.0 - btn_h;
    let got_w = 120.0;
    action_btn(px, fonts, box_x, btn_y, got_w, btn_h, "Got it", Hit::ReplyDismiss, hover, hits, false);
    let sending = matches!(compose.status, crate::feedback::Status::Sending);
    action_btn(
        px,
        fonts,
        box_x + got_w + 10.0,
        btn_y,
        (inner_w - got_w - 10.0).max(100.0),
        btn_h,
        if sending { "Sending…" } else { "Send" },
        Hit::ReplySend,
        hover,
        hits,
        true,
    );
    scroll_max
}
