#![allow(unused_imports)]
use super::*;

// THESIS: After an in-app update, a centered TV plaque names the version and the change — not a generic changelog sheet.
// OWN-WORLD: Charcoal board, orange skew version plaque, Exo 2 ExtraBold Italic, Got it as the only orange fill.
// STORY: Rider sees what this build changed, hits Got it, returns to Settings.
// FIRST VIEWPORT: Dimmed Settings; centered ~520px board; orange skew 0.1.x heading; headline; section bullets; full-width Got it.
// FORM: Center Plaque; approved .impeccable/mocks/whats-new-center.png
// FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
pub(crate) fn draw_whats_new(
    px: &mut Pixmap,
    fonts: &Fonts,
    win_w: f32,
    win_h: f32,
    notes: &crate::changelog::Notes,
    hover: Option<Hit>,
    scroll: f32,
    hits: &mut Vec<HitBox>,
) -> f32 {
    let scrim = Color::from_rgba8(8, 8, 10, 204);
    if let Some(r) = Rect::from_xywh(0.0, 0.0, win_w, win_h) {
        fill_rect(px, r, scrim);
    }
    hits.push(HitBox {
        id: Hit::WhatsNewScrim,
        x: 0.0,
        y: 0.0,
        w: win_w,
        h: win_h,
    });

    let pad = 22.0;
    let plaque_h = 40.0;
    let skew = 6.0;
    let btn_h = 40.0;
    let footer_h = 16.0 + btn_h + 16.0;
    let panel_w = 520.0_f32.min(win_w - 48.0).max(320.0);
    let inner_w = (panel_w - pad * 2.0).max(200.0);
    let head_size = 16.0;
    let body_size = 12.0;
    let line_h = 18.0;

    let warn = crate::plugin::needs_restart();
    let warn_lines = if warn {
        wrap_fb(fonts, crate::plugin::RESTART_PLUGIN_STILL_RUNNING, inner_w, body_size)
    } else {
        Vec::new()
    };
    let heads = wrap_fb(fonts, &notes.headline, inner_w, head_size);
    let mut body_h = 0.0;
    if !warn_lines.is_empty() {
        body_h += warn_lines.len() as f32 * line_h + 14.0;
    }
    if !notes.headline.is_empty() {
        body_h += heads.len() as f32 * 22.0 + 10.0;
    }
    let mut wrapped: Vec<Vec<Vec<String>>> = Vec::new();
    for sec in &notes.sections {
        if !sec.title.is_empty() {
            body_h += 26.0;
        }
        let mut bullets = Vec::new();
        for b in &sec.bullets {
            let lines = wrap_fb(fonts, b, inner_w - 18.0, body_size);
            body_h += lines.len() as f32 * line_h + 6.0;
            bullets.push(lines);
        }
        wrapped.push(bullets);
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
        id: Hit::WhatsNewPanel,
        x: panel_x,
        y: panel_y,
        w: panel_w,
        h: panel_h,
    });

    let plaque_x = panel_x + pad;
    let plaque_y = panel_y + pad;
    let ver_sz = 18.0;
    let ver_w = measure(fonts, &notes.version, ver_sz);
    let plaque_w = (ver_w + 36.0).min(inner_w).max(72.0);
    fill_skew(px, plaque_x, plaque_y, (plaque_w - skew).max(48.0), plaque_h, skew, accent());
    text(
        px,
        fonts,
        &notes.version,
        ver_sz,
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
            if !warn_lines.is_empty() {
                for line in &warn_lines {
                    if y + line_h > 0.0 && y < view_h {
                        text(&mut body, fonts, line, body_size, 0.0, y, accent(), false);
                    }
                    y += line_h;
                }
                y += 14.0;
            }
            if !notes.headline.is_empty() {
                for line in &heads {
                    if y + 22.0 > 0.0 && y < view_h {
                        text(&mut body, fonts, line, head_size, 0.0, y, text_col(), false);
                    }
                    y += 22.0;
                }
                y += 10.0;
            }
            for (sec, bullets) in notes.sections.iter().zip(wrapped.iter()) {
                if !sec.title.is_empty() {
                    if y + 22.0 > 0.0 && y < view_h {
                        text(
                            &mut body,
                            fonts,
                            &sec.title.to_ascii_uppercase(),
                            10.0,
                            0.0,
                            y + 4.0,
                            dim(),
                            false,
                        );
                    }
                    y += 26.0;
                }
                for lines in bullets {
                    let block_h = lines.len() as f32 * line_h;
                    if y + block_h > 0.0 && y < view_h {
                        fill_circle(&mut body, 4.0, y + 8.0, 2.2, accent());
                        let mut ly = y;
                        for line in lines {
                            text(&mut body, fonts, line, body_size, 14.0, ly, text_col(), false);
                            ly += line_h;
                        }
                    }
                    y += block_h + 6.0;
                }
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

    let btn_w = panel_w - pad * 2.0;
    let btn_x = panel_x + pad;
    let btn_y = panel_y + panel_h - 16.0 - btn_h;
    action_btn(
        px,
        fonts,
        btn_x,
        btn_y,
        btn_w,
        btn_h,
        "Got it",
        Hit::WhatsNewDismiss,
        hover,
        hits,
        true,
    );
    scroll_max
}
