#![allow(unused_imports)]
use super::*;

pub(crate) fn draw_clear_confirm(
    px: &mut Pixmap,
    fonts: &Fonts,
    win_w: f32,
    win_h: f32,
    kind: ClearKind,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) {
    let scrim = scrim();
    if let Some(r) = Rect::from_xywh(0.0, 0.0, win_w, win_h) {
        fill_rect(px, r, scrim);
    }
    hits.push(HitBox {
        id: Hit::ClearScrim,
        x: 0.0,
        y: 0.0,
        w: win_w,
        h: win_h,
    });

    let pad = 22.0;
    let plaque_h = 40.0;
    let skew = 6.0;
    let btn_h = 40.0;
    let panel_w = 520.0_f32.min(win_w - 48.0).max(320.0);
    let inner_w = (panel_w - pad * 2.0).max(200.0);
    let body_size = 13.0;
    let (title, body) = match kind {
        ClearKind::Profile => (
            "Clear Profile",
            "Clears your Profile graph. Motos stays. This cannot be undone.".to_string(),
        ),
        ClearKind::Motos => {
            let mut s = "Deletes stored motos. Profile stays. This cannot be undone.".to_string();
            if crate::review::live_id().is_some() {
                s.push_str(" The moto you are in now is kept.");
            }
            ("Clear Motos", s)
        }
    };
    let heads = wrap_fb(fonts, &body, inner_w, body_size);
    let body_h = heads.len() as f32 * 20.0 + 8.0;
    let header_h = pad + plaque_h + 16.0;
    let footer_h = 16.0 + btn_h + 16.0;
    let panel_h = (header_h + body_h + footer_h).min(win_h - 48.0).max(160.0);
    let panel_x = ((win_w - panel_w) * 0.5).max(16.0);
    let panel_y = ((win_h - panel_h) * 0.5).max(16.0);
    let board = menu_fill();
    fill_round(px, panel_x, panel_y, panel_w, panel_h, 10.0, board);
    hits.push(HitBox {
        id: Hit::ClearPanel,
        x: panel_x,
        y: panel_y,
        w: panel_w,
        h: panel_h,
    });

    let plaque_x = panel_x + pad;
    let plaque_y = panel_y + pad;
    let ver_sz = 16.0;
    let ver_w = measure(fonts, title, ver_sz);
    let plaque_w = (ver_w + 36.0).min(inner_w).max(72.0);
    fill_skew(
        px,
        plaque_x,
        plaque_y,
        (plaque_w - skew).max(48.0),
        plaque_h,
        skew,
        accent(),
    );
    text(
        px,
        fonts,
        title,
        ver_sz,
        plaque_x + 16.0,
        plaque_y + 10.0,
        ink(),
        false,
    );

    let mut ty = plaque_y + plaque_h + 16.0;
    for line in &heads {
        text(px, fonts, line, body_size, plaque_x, ty, text_col(), false);
        ty += 20.0;
    }

    let btn_y = panel_y + panel_h - 16.0 - btn_h;
    let cancel_w = 120.0;
    action_btn(
        px,
        fonts,
        plaque_x,
        btn_y,
        cancel_w,
        btn_h,
        "Cancel",
        Hit::ClearCancel,
        hover,
        hits,
        false,
    );
    action_btn(
        px,
        fonts,
        plaque_x + cancel_w + 10.0,
        btn_y,
        (inner_w - cancel_w - 10.0).max(100.0),
        btn_h,
        "Clear",
        Hit::ClearConfirm,
        hover,
        hits,
        true,
    );
}
