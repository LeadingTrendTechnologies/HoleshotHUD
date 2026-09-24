use crate::config::ink_on_rgb;

pub(crate) const NIGHT: [u8; 3] = [10, 10, 10];

pub(crate) const CHARCOAL: [u8; 3] = [8, 8, 10];

pub(crate) const ROW: [u8; 3] = [20, 20, 22];

pub(crate) const FIELD: [u8; 3] = [96, 96, 102];

pub(crate) const TEXT: [u8; 3] = [228, 228, 230];

pub(crate) const INK: [u8; 3] = [16, 16, 18];

pub(crate) const HAIR: [u8; 3] = [42, 42, 46];

/// PiBoSo `.mnu` / `ui.ui` colors are packed **A B G R** (same as plugin ABGR).
/// Writing R G B made Look orange show up as light blue on hover.
pub(crate) fn argb(a: u8, rgb: [u8; 3]) -> String {
    format!("{a} {} {} {}", rgb[2], rgb[1], rgb[0])
}

pub(crate) fn bgr(rgb: [u8; 3]) -> String {
    format!("{} {} {}", rgb[2], rgb[1], rgb[0])
}

pub(crate) const DIM: [u8; 3] = [132, 132, 140];

/// Inset night-ink board. List Y stays stock so rows still bind.
/// Board starts at the list so Host / Filter / Hide sit on the studio.
/// Card ends above the chrome row so Back / Local / World / Join stay visible.

/// Practice / Testing Setup footer — lifted off the absolute bottom edge.
pub(crate) const SETUP_CHROME_Y0: f32 = 0.928;

pub(crate) const SETUP_CHROME_Y1: f32 = 0.968;
/// Content boards end above the chrome row (stock went to 0.944 and covered Back/Start).

/// Content boards end above the chrome row (stock went to 0.944 and covered Back/Start).
pub(crate) const SETUP_BOARD_Y1: f32 = 0.900;

/// Horizontally center a button label (`align right` + inset pos → `align center`).
pub(crate) fn center_button_label(dialog: &str, button_name: &str) -> String {
    let marker = format!("name {button_name}");
    let Some(btn_at) = dialog.find(&marker) else {
        return dialog.to_string();
    };
    let (head, rest) = dialog.split_at(btn_at);
    let item_end = rest.find("\n\titem_").unwrap_or(rest.len());
    let (btn, tail) = rest.split_at(item_end);
    let btn = btn
        .replace("align right", "align center")
        .replace("pos 0.006250 0.005556", "pos 0.000000 0.005556");
    format!("{head}{btn}{tail}")
}

/// Shared menu ink (TEXT / accent / pulls / section clears). Board remaps stay caller-side.
pub(crate) fn recolor_menu_ink(body: &str, accent: [u8; 3]) -> String {
    let text = bgr(TEXT);
    let hot = bgr(accent);
    let night = bgr(NIGHT);
    let row = bgr(ROW);
    let mut s = body.replace("\r\n", "\n").replace('\r', "\n");
    s = s.replace("backcolor 255 220 220 220", &format!("backcolor 255 {row}"));
    s = s.replace("backcolor 255 37 37 37", &format!("backcolor 255 {row}"));
    // Pull list panels BEFORE clearing bare black backcolors —
    // `backcolor 255 0 0 0` is a suffix of `pullbackcolor 255 0 0 0`.
    s = s.replace(
        "pullbackcolor 200 0 0 0",
        &format!("pullbackcolor 255 {night}"),
    );
    s = s.replace(
        "pullbackcolor 255 0 0 0",
        &format!("pullbackcolor 255 {night}"),
    );
    s = s.replace(
        &format!("pullbackcolor 160 {night}"),
        &format!("pullbackcolor 255 {night}"),
    );
    s = s.replace("\tbackcolor 255 0 0 0", "\tbackcolor 0 0 0 0");
    s = s.replace("\tcolor 255 0 0 0\n", &format!("\tcolor 255 {text}\n"));
    s = s.replace("\tcolor 255 60 60 60\n", &format!("\tcolor 255 {text}\n"));
    s = s.replace("\tcolor 255 80 80 80\n", &format!("\tcolor 255 {text}\n"));
    s = s.replace("\tcolor 255 40 40 40\n", &format!("\tcolor 255 {text}\n"));
    s = s.replace("\tcolor 255 240 240 240\n", &format!("\tcolor 255 {text}\n"));
    s = s.replace("\tcolor 255 255 255 255\n", &format!("\tcolor 255 {text}\n"));
    s = s.replace("textcolor 255 60 60 60", &format!("textcolor 255 {text}"));
    s = s.replace("color1 255 60 60 60", &format!("color1 255 {text}"));
    s = s.replace("color1 255 80 80 80", &format!("color1 255 {text}"));
    s = s.replace("color2 255 0 0 0", &format!("color2 255 {hot}"));
    s = s.replace("color3 255 0 0 0", &format!("color3 255 {hot}"));
    s = s.replace("color1 255 240 240 240", &format!("color1 255 {text}"));
    s = s.replace("color2 255 240 240 240", &format!("color2 255 {hot}"));
    s = s.replace("color3 255 240 240 240", &format!("color3 255 {hot}"));
    s = s.replace(
        "pulltextcolor1 255 255 255 255",
        &format!("pulltextcolor1 255 {text}"),
    );
    s = s.replace(
        "pulltextcolor2 255 40 60 240",
        &format!("pulltextcolor2 255 {hot}"),
    );
    s = s.replace("color 255 40 60 240", &format!("color 255 {hot}"));
    s
}

/// Shared ink swaps for profiles.mnu (no Practice Setup board remaps).

/// Shared ink swaps for profiles.mnu (no Practice Setup board remaps).
pub(crate) fn recolor_profiles_dialog(body: &str, accent: [u8; 3]) -> String {
    recolor_menu_ink(body, accent)
}

pub(crate) fn clear_title_wash(s: &str) -> String {
    s.replace("backcolor 127 0 0 0", "backcolor 0 0 0 0")
}

pub(crate) fn clear_row_field_fills(s: &str) -> String {
    let row = bgr(ROW);
    s.replace(&format!("backcolor 255 {row}"), "backcolor 0 0 0 0")
}

/// Wide setup board: stock full-bleed dialog → glass sprite, stop above chrome.

/// Wide setup board: stock full-bleed dialog → glass sprite, stop above chrome.
pub(crate) fn glass_wide_board(s: &str, stock_sprite: &str, glass_sprite: &str) -> String {
    let mut out = s.replace(
        &format!("sprite {stock_sprite}"),
        &format!("sprite {glass_sprite}"),
    );
    out = out.replace(
        "rect 0.006250 0.133333 0.993750 0.944444",
        &format!("rect 0.006250 0.133333 0.993750 {SETUP_BOARD_Y1:.6}"),
    );
    out
}

pub(crate) fn clear_footer_bar(s: &str) -> String {
    let clear = format!(
        "rect 0.000000 {SETUP_CHROME_Y0:.6} 1.000000 {SETUP_CHROME_Y1:.6}\n\t\tcolor 0 0 0 0"
    );
    let mut out = s.replace(
        "rect 0.000000 0.966667 1.000000 1.000000\n\t\tcolor 127 0 0 0",
        &clear,
    );
    out = out.replace(
        "rect 0.000000 0.966667 1.000000 1.000000\n\t\tcolor 160 0 0 0",
        &clear,
    );
    out = out.replace(
        &format!(
            "rect 0.000000 {SETUP_CHROME_Y0:.6} 1.000000 {SETUP_CHROME_Y1:.6}\n\t\tcolor 127 0 0 0"
        ),
        &clear,
    );
    out
}

pub(crate) fn lift_chrome_rect(s: &str, stock_rect: &str, x0: f32, x1: f32) -> String {
    s.replace(
        stock_rect,
        &format!("rect {x0:.6} {SETUP_CHROME_Y0:.6} {x1:.6} {SETUP_CHROME_Y1:.6}"),
    )
}

/// Force idle plaques for stock buttons that only set sprite2/3.

/// Force idle plaques for stock buttons that only set sprite2/3.
pub(crate) fn force_idle_plaques(s: &str) -> String {
    let mut out = s.replace(
        "sprite2 back1.tga\n\t\t\tsprite3 back2.tga",
        "sprite1 back1.tga\n\t\t\tsprite2 back1.tga\n\t\t\tsprite3 back2.tga",
    );
    out = out.replace(
        "sprite2 done1.tga\n\t\t\tsprite3 done2.tga",
        "sprite1 done1.tga\n\t\t\tsprite2 done1.tga\n\t\t\tsprite3 done2.tga",
    );
    // Host Setup quirks — hover-only back2/done2.
    out = out.replace(
        "sprite2 back2.tga\n\t\t\tsprite3 back2.tga",
        "sprite1 back1.tga\n\t\t\tsprite2 back1.tga\n\t\t\tsprite3 back2.tga",
    );
    out = out.replace(
        "sprite2 done2.tga\n\t\t\tsprite3 done2.tga",
        "sprite1 done1.tga\n\t\t\tsprite2 done1.tga\n\t\t\tsprite3 done2.tga",
    );
    out = out.replace(
        "sprite2 b_button2.tga\n\t\t\tsprite3 b_button3.tga",
        "sprite1 button1.tga\n\t\t\tsprite2 button2.tga\n\t\t\tsprite3 button3.tga",
    );
    out
}

pub(crate) fn paint_chrome_row(s: &str, accent: [u8; 3], backs: &[&str], ctas: &[&str]) -> String {
    let idle = argb(255, TEXT);
    let hot = argb(255, accent);
    let ink = argb(255, ink_on_rgb(accent));
    let tip_y = (SETUP_CHROME_Y1 - SETUP_CHROME_Y0 - 0.022222) * 0.5;
    let mut out = s.to_string();
    for name in backs.iter().chain(ctas.iter()) {
        out = center_bike_chrome_button(&out, name, tip_y);
    }
    for name in backs {
        out = paint_bike_chrome_button(&out, name, &idle, &hot, &hot);
    }
    for name in ctas {
        out = paint_bike_chrome_button(&out, name, &ink, &ink, &ink);
    }
    out
}

pub(crate) fn frost_track_bar(s: &str) -> String {
    s.replace(
        "\titem_darkbox\n\t{\n\t\tname ID_DARKBOX\n\t\trect 0.006250 0.133333 0.993750 0.188889\n\t\tcolor 240 170 180 190\n\t}\n",
        "\titem_bitmap\n\t{\n\t\tname ID_TRACK_BAR\n\t\trect 0.006250 0.133333 0.993750 0.188889\n\t\tsprite setupbar.tga\n\t}\n",
    )
    .replace("backcolor 240 170 180 190", "backcolor 0 0 0 0")
}

pub(crate) fn set_button_sprites(dialog: &str, name: &str, s1: &str, s2: &str, s3: &str) -> String {
    let marker = format!("name {name}\n");
    let Some(btn_at) = dialog.find(&marker) else {
        return dialog.to_string();
    };
    let (head, rest) = dialog.split_at(btn_at);
    let item_end = rest.find("\n\titem_").unwrap_or(rest.len());
    let (btn, tail) = rest.split_at(item_end);
    let btn = btn.to_string();
    // Drop any existing sprite lines inside the button {{ }} block, then insert ours.
    let mut kept = String::new();
    for line in btn.lines() {
        let t = line.trim_start();
        if t.starts_with("sprite1 ") || t.starts_with("sprite2 ") || t.starts_with("sprite3 ") {
            continue;
        }
        kept.push_str(line);
        kept.push('\n');
    }
    if let Some(at) = kept.find("\t\t{\n") {
        let insert = at + "\t\t{\n".len();
        kept.insert_str(
            insert,
            &format!("\t\t\tsprite1 {s1}\n\t\t\tsprite2 {s2}\n\t\t\tsprite3 {s3}\n"),
        );
    }
    format!("{head}{kept}{tail}")
}

/// Insert a padded `fieldbox.tga` behind every `item_editbox`.
pub(crate) fn inject_editbox_fieldboxes(dialog: &str) -> String {
    const PAD: f32 = 0.002;
    let mut out = String::new();
    let mut rest = dialog;
    while let Some(rel) = rest.find("\titem_editbox\n") {
        out.push_str(&rest[..rel]);
        let after = &rest[rel..];
        let end = after[1..]
            .find("\n\titem_")
            .map(|i| i + 1)
            .unwrap_or(after.len());
        let box_item = &after[..end];
        if let (Some(name), Some((x0, y0, x1, y1))) = (pull_name(box_item), pull_rect(box_item)) {
            out.push_str(&format!(
                "\titem_bitmap\n\t{{\n\t\tname ID_BG_{name}\n\t\trect {rx0:.6} {ry0:.6} {rx1:.6} {ry1:.6}\n\t\tsprite fieldbox.tga\n\t}}\n",
                rx0 = x0 - PAD,
                ry0 = y0 - PAD,
                rx1 = x1 + PAD,
                ry1 = y1 + PAD,
            ));
        }
        out.push_str(box_item);
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

/// Insert a padded `fieldbox.tga` bitmap behind every `item_pull`.
pub(crate) fn inject_pull_fieldboxes(dialog: &str) -> String {
    const PAD: f32 = 0.002;
    let mut out = String::new();
    let mut rest = dialog;
    while let Some(rel) = rest.find("\titem_pull\n") {
        out.push_str(&rest[..rel]);
        let after = &rest[rel..];
        let end = after[1..]
            .find("\n\titem_")
            .map(|i| i + 1)
            .unwrap_or(after.len());
        let pull = &after[..end];
        if let (Some(name), Some((x0, y0, x1, y1))) = (pull_name(pull), pull_rect(pull)) {
            out.push_str(&format!(
                "\titem_bitmap\n\t{{\n\t\tname ID_BG_{name}\n\t\trect {rx0:.6} {ry0:.6} {rx1:.6} {ry1:.6}\n\t\tsprite fieldbox.tga\n\t}}\n",
                rx0 = x0 - PAD,
                ry0 = y0 - PAD,
                rx1 = x1 + PAD,
                ry1 = y1 + PAD,
            ));
        }
        out.push_str(pull);
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

/// Closed pull value on a light fieldbox — pure black (A B G R). Garage only.
pub(crate) fn force_pull_textcolor_black(dialog: &str) -> String {
    let mut out = String::with_capacity(dialog.len());
    for line in dialog.lines() {
        let t = line.trim_start();
        if t.starts_with("textcolor ") {
            let indent = &line[..line.len() - t.len()];
            out.push_str(indent);
            out.push_str("textcolor 255 0 0 0\n");
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    // Preserve a trailing newline if the input had one; lines() drops it.
    if dialog.ends_with('\n') && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

pub(crate) fn pull_name(pull: &str) -> Option<&str> {
    let line = pull.lines().find(|l| l.trim_start().starts_with("name "))?;
    Some(line.trim().trim_start_matches("name ").trim())
}

pub(crate) fn pull_rect(pull: &str) -> Option<(f32, f32, f32, f32)> {
    let line = pull.lines().find(|l| l.trim_start().starts_with("rect "))?;
    let mut nums = line
        .trim()
        .trim_start_matches("rect ")
        .split_whitespace()
        .filter_map(|t| t.parse::<f32>().ok());
    Some((nums.next()?, nums.next()?, nums.next()?, nums.next()?))
}

pub(crate) fn center_bike_chrome_button(dialog: &str, button_name: &str, tip_y: f32) -> String {
    let marker = format!("name {button_name}\n");
    let Some(btn_at) = dialog.find(&marker) else {
        return dialog.to_string();
    };
    let (head, rest) = dialog.split_at(btn_at);
    let item_end = rest.find("\n\titem_").unwrap_or(rest.len());
    let (btn, tail) = rest.split_at(item_end);
    let btn = btn
        .replace("align right", "align center")
        .replace("align left", "align center")
        .replace("pos 0.006250 0.005556", &format!("pos 0.000000 {tip_y:.6}"))
        .replace("pos 0.000000 0.005556", &format!("pos 0.000000 {tip_y:.6}"));
    format!("{head}{btn}{tail}")
}

pub(crate) fn paint_bike_chrome_button(
    dialog: &str,
    button_name: &str,
    c1: &str,
    c2: &str,
    c3: &str,
) -> String {
    let marker = format!("name {button_name}\n");
    let Some(btn_at) = dialog.find(&marker) else {
        return dialog.to_string();
    };
    let (head, rest) = dialog.split_at(btn_at);
    let item_end = rest.find("\n\titem_").unwrap_or(rest.len());
    let (btn, tail) = rest.split_at(item_end);
    let mut btn = btn.to_string();
    if let Some(at) = btn.find("color1 255 ") {
        let end = btn[at..].find('\n').map(|i| at + i).unwrap_or(btn.len());
        btn.replace_range(at..end, &format!("color1 {c1}"));
    }
    if let Some(at) = btn.find("color2 255 ") {
        let end = btn[at..].find('\n').map(|i| at + i).unwrap_or(btn.len());
        btn.replace_range(at..end, &format!("color2 {c2}"));
    }
    if let Some(at) = btn.find("color3 255 ") {
        let end = btn[at..].find('\n').map(|i| at + i).unwrap_or(btn.len());
        btn.replace_range(at..end, &format!("color3 {c3}"));
    }
    format!("{head}{btn}{tail}")
}

pub(crate) fn split_mnu_dialogs(src: &str) -> Vec<(String, String)> {
    let src = src.replace("\r\n", "\n").replace('\r', "\n");
    let mut starts = Vec::new();
    let mut off = 0;
    for line in src.lines() {
        if line.trim() == "dialog" {
            starts.push(off);
        }
        off += line.len() + 1;
    }
    let mut out = Vec::new();
    for (i, start) in starts.iter().copied().enumerate() {
        let start = start.min(src.len());
        let Some(brace_rel) = src[start..].find('{') else {
            continue;
        };
        let brace = start + brace_rel;
        let mut depth = 0i32;
        let mut end = src.len();
        for (j, c) in src[brace..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = brace + j + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        if i + 1 < starts.len() {
            end = end.min(starts[i + 1]);
        }
        let body = src[start..end].trim().to_string();
        let name = body
            .lines()
            .find_map(|l| {
                l.trim()
                    .strip_prefix("name ")
                    .map(|s| s.trim().to_string())
            })
            .unwrap_or_default();
        if !name.is_empty() {
            out.push((name, body));
        }
    }
    out
}

