#![allow(unused_imports)]
use super::*;

const STEPS: [&str; 6] = [
    "1. Download demo saves holeshot-demo.png, or use your own PNG or JPEG.",
    "2. Upload Pitboard image. An empty name uses the file name. A picture alone starts as one blank slot.",
    "3. Drag slots on the plate. Size, Bold, and Text are on the bar. Add or remove slots, up to 12.",
    "4. Each menu picks the stat. None hides that spot.",
    "5. When is always, the end of a sector, or the end of a lap.",
    "6. Export writes plate.png and board.json. Import opens that pair.",
];

pub(crate) fn draw_pit_help(
    px: &mut Pixmap,
    fonts: &Fonts,
    win_w: f32,
    win_h: f32,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) {
    let scrim = scrim();
    if let Some(r) = Rect::from_xywh(0.0, 0.0, win_w, win_h) {
        fill_rect(px, r, scrim);
    }
    hits.push(HitBox {
        id: Hit::PitHelpScrim,
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
    let line_h = 20.0;
    let mut wrapped = Vec::new();
    let mut body_h = 0.0;
    for step in STEPS {
        let lines = wrap_fb(fonts, step, inner_w, body_size);
        body_h += lines.len() as f32 * line_h + 6.0;
        wrapped.push(lines);
    }
    let header_h = pad + plaque_h + 16.0;
    let footer_h = 16.0 + btn_h + 16.0;
    let panel_h = (header_h + body_h + footer_h)
        .min(win_h - 48.0)
        .max(160.0);
    let panel_x = ((win_w - panel_w) * 0.5).max(16.0);
    let panel_y = ((win_h - panel_h) * 0.5).max(16.0);
    fill_round(px, panel_x, panel_y, panel_w, panel_h, 10.0, menu_fill());
    hits.push(HitBox {
        id: Hit::PitHelpPanel,
        x: panel_x,
        y: panel_y,
        w: panel_w,
        h: panel_h,
    });

    let plaque_x = panel_x + pad;
    let plaque_y = panel_y + pad;
    let title = "Pit board";
    let title_size = 16.0;
    let title_w = measure(fonts, title, title_size);
    let plaque_w = (title_w + 36.0).min(inner_w).max(72.0);
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
        title_size,
        plaque_x + 16.0,
        plaque_y + 10.0,
        ink(),
        false,
    );

    let mut text_y = plaque_y + plaque_h + 16.0;
    for lines in &wrapped {
        for line in lines {
            text(px, fonts, line, body_size, plaque_x, text_y, text_col(), false);
            text_y += line_h;
        }
        text_y += 6.0;
    }

    let btn_y = panel_y + panel_h - 16.0 - btn_h;
    action_btn(
        px,
        fonts,
        plaque_x,
        btn_y,
        inner_w,
        btn_h,
        "Got it",
        Hit::PitHelpDismiss,
        hover,
        hits,
        true,
    );
}
