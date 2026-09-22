use std::path::Path;

use crate::config::ink_on_rgb;

use super::io::{stock_mnu, stock_multijoin, stock_options, write_text};
use super::shell::*;

pub(crate) const BROWSER_DIALOGS: &[&str] = &["idd_multi_lan", "idd_multi_world", "idd_server_browser"];

pub(crate) const OPTIONS_CHROME: &str = "idd_options";

pub(crate) const OPTIONS_PANES: &[&str] = &[
    "idd_graphic_options",
    "idd_input_options",
    "idd_input2_options",
    "idd_input3_options",
    "idd_misc_options",
    "idd_others_options",
];

/// Inset night-ink board. List Y stays stock so rows still bind.
/// Board starts at the list so Host / Filter / Hide sit on the studio.
/// Card ends above the chrome row so Back / Local / World / Join stay visible.
pub(crate) const CARD_X0: f32 = 0.006;

pub(crate) const LIST_Y0: f32 = 0.188889;

pub(crate) const CARD_Y0: f32 = LIST_Y0;

pub(crate) const CARD_X1: f32 = 0.994;

pub(crate) const CARD_Y1: f32 = 0.900;

pub(crate) const CHROME_Y0: f32 = 0.912;

pub(crate) const CHROME_Y1: f32 = 0.948;

/// Options pit-bottom tabs: inset form board; tabs share the Back/Done row.

/// Options pit-bottom tabs: inset form board; tabs share the Back/Done row.
pub(crate) const OPT_FORM_X0: f32 = 0.030;

pub(crate) const OPT_FORM_X1: f32 = 0.970;

pub(crate) const OPT_FORM_Y0: f32 = 0.050;
/// Board stops above the chrome row — pane boards paint over chrome tabs.

/// Board stops above the chrome row — pane boards paint over chrome tabs.
pub(crate) const OPT_FORM_Y1: f32 = 0.912;
/// Left of in-server Chat (`CHAT_SLOT`) so Chat + Done stay clear.

/// Left of in-server Chat (`CHAT_SLOT`) so Chat + Done stay clear.
pub(crate) const OPT_TAB_GROUP_X0: f32 = 0.150;

pub(crate) const OPT_TAB_GROUP_X1: f32 = 0.550;
/// Same band as Back / Done.

/// Same band as Back / Done.
pub(crate) const OPT_TAB_Y0: f32 = 0.928;

pub(crate) const OPT_TAB_Y1: f32 = 0.968;

pub(crate) const OPT_CHROME_Y0: f32 = 0.928;

pub(crate) const OPT_CHROME_Y1: f32 = 0.968;
/// Content remap inset — keeps rows clear of the board edge.

/// Content remap inset — keeps rows clear of the board edge.
pub(crate) const OPT_CONTENT_X0: f32 = 0.050;

pub(crate) const OPT_CONTENT_X1: f32 = 0.950;

pub(crate) const OPT_CONTENT_Y0: f32 = 0.070;
/// Extra vertical room now that tabs sit on the chrome row.

/// Extra vertical room now that tabs sit on the chrome row.
pub(crate) const OPT_CONTENT_Y1: f32 = 0.888;
/// Stock content board bounds used when remapping pane rects into the form.

/// Stock content board bounds used when remapping pane rects into the form.
pub(crate) const OPT_STOCK_X0: f32 = 0.006250;

pub(crate) const OPT_STOCK_X1: f32 = 0.993750;

pub(crate) const OPT_STOCK_Y0: f32 = 0.133333;

pub(crate) const OPT_STOCK_Y1: f32 = 0.911111;

/// Practice / Testing Setup footer — lifted off the absolute bottom edge.

pub(crate) const SETUP_CHROME: &str = "idd_testing_setup";

pub(crate) const TRACK_INFO_DIALOG: &str = "idd_track_info";

pub(crate) const BIKE_CHOOSE: &str = "idd_bikechoose";

pub(crate) const BIKE_CHOOSE2: &str = "idd_bikechoose2";

pub(crate) const BIKE_INFO: &str = "idd_bikeinfo";

pub(crate) const PROFILES_DIALOG: &str = "idd_profiles";

pub(crate) const PROFILE_MODIFY: &str = "idd_profilemodify";

pub(crate) const PROFILE_DELETE: &str = "idd_profiledeleteconfirm";

pub(crate) const BESTLAPS_DIALOG: &str = "idd_bestlaps";

pub(crate) const BESTLAPS_OVERALL: &str = "idd_bestoverall";

pub(crate) const BESTLAPS_BIKE: &str = "idd_bestbike";

pub(crate) const VIEWREPLAYS_DIALOG: &str = "idd_viewreplays";

pub(crate) const VIEWREPLAYS_DELETE: &str = "idd_confirm_replay_delete";

pub(crate) const HOST_SETUP_DIALOG: &str = "idd_host_setup";

pub(crate) const RACE_SETUP_DIALOG: &str = "idd_race_setup";

pub(crate) const HOST_RACESETUP_DIALOG: &str = "idd_host_racesetup";

pub(crate) const REPLAY_DIALOG: &str = "idd_replay";

pub(crate) const REPLAY_STRAIGHTRHYTHM: &str = "idd_replay_straightrhythm";

pub(crate) const REPLAY_CLASSIFICATION: &str = "idd_replay_classification";

pub(crate) const REPLAY_TELEMETRY: &str = "idd_replay_telemetry";

pub(crate) const REPLAY_SAVE: &str = "idd_replay_save";

pub(crate) const REPLAY_OVERWRITE: &str = "idd_replay_overwrite";

pub(crate) const MULTI_PIT: &str = "idd_multi_pit";

pub(crate) const RACE_ENTRIES: &str = "idd_race_entries";

pub(crate) const RACE_INFO: &str = "idd_race_info";

pub(crate) const RACE_INFO2: &str = "idd_race_info2";

pub(crate) const RACE_RESULTS: &str = "idd_race_results";
/// Results tab child lists (board/export live on `idd_race_results`).

/// Results tab child lists (board/export live on `idd_race_results`).
pub(crate) const RACE_STANDINGS: &[&str] = &[
    "idd_race_practice",
    "idd_race_analysis",
    "idd_race_startinggrid",
    "idd_race_classification",
    "idd_race_penalties",
    "idd_race_ideallaptime",
];

pub(crate) const CHAT_SWITCH: &str = "idd_chatswitch";

pub(crate) const RACE_EXIT: &str = "idd_raceexit";

pub(crate) const GATE_CHOOSE: &str = "idd_gatechoose";

pub(crate) const TESTING_EXIT: &str = "idd_testingexit";

pub(crate) const TESTING_PIT: &str = "testing_pit";

pub(crate) const TEST_PANEL: &str = "test_panel";

pub(crate) const TEST_EVENTINFO: &str = "idd_eventinfo";

pub(crate) const TEST_LAPS: &str = "idd_laps";

pub(crate) const TEST_TRACKINFO_PIT: &str = "idd_track_info_pit";

pub(crate) const TEST_TRAINER: &str = "idd_trainer";

pub(crate) const GARAGE_DIALOG: &str = "idd_garage";

/// Practice Track Info map — stock/remapped rect + matching rounded-corner mask.

/// Practice Track Info map — stock/remapped rect + matching rounded-corner mask.
pub(crate) const TRACKMAP_PIT_RECT: &str = "0.137500 0.211111 0.543750 0.880000";

pub(crate) const TRACKMAP_SETUP_RECT: &str = "0.212500 0.155556 0.587500 0.822222";

/// Mid-row segment group on the in-server pit (Settings / Replay / Garage).
/// Chat sits in `CHAT_SLOT` immediately after (shared with garage Info|Chat|Test).

/// Mid-row segment group on the in-server pit (Settings / Replay / Garage).
/// Chat sits in `CHAT_SLOT` immediately after (shared with garage Info|Chat|Test).
pub(crate) const PIT_GROUP_X0: f32 = 0.200;

pub(crate) const PIT_GROUP_X1: f32 = 0.560;

pub(crate) const PIT_GROUP_N: usize = 3;
/// Shared Chat chrome slot — `idd_chatswitch` on pit + garage.

/// Shared Chat chrome slot — `idd_chatswitch` on pit + garage.
pub(crate) const CHAT_SLOT: (f32, f32) = (0.560, 0.680);
/// Garage chrome actions Info | Chat | Test (Chat uses `CHAT_SLOT`).

/// Garage chrome actions Info | Chat | Test (Chat uses `CHAT_SLOT`).
pub(crate) const GARAGE_ACT_X0: f32 = 0.460;

pub(crate) const GARAGE_ACT_X1: f32 = 0.790;
/// Category tabs sit above chrome and left of Info|Chat|Test (no overlap).

/// Category tabs sit above chrome and left of Info|Chat|Test (no overlap).
pub(crate) const GARAGE_TAB_X0: f32 = 0.006250;

pub(crate) const GARAGE_TAB_X1: f32 = 0.440;

pub(crate) const GARAGE_TAB_Y0: f32 = 0.884;

pub(crate) const GARAGE_TAB_Y1: f32 = 0.920;
/// Left setup boards end just above the category tabs.

/// Left setup boards end just above the category tabs.
pub(crate) const GARAGE_PANE_Y1: f32 = 0.878;

pub(crate) fn browser_card_fill() -> String {
    format!(
        "\titem_bitmap\n\t{{\n\t\tname ID_BOARD\n\t\trect {CARD_X0:.6} {CARD_Y0:.6} {CARD_X1:.6} {CARD_Y1:.6}\n\t\tsprite serverboard.tga\n\t}}\n"
    )
}

pub(crate) fn dest_sprites() -> &'static str {
    "sprite1 button1.tga\n\t\t\tsprite2 button2.tga\n\t\t\tsprite3 button3.tga"
}

pub(crate) fn splice_multijoin(ui: &Path, bak: Option<&Path>, accent: [u8; 3]) -> Result<bool, String> {
    let Some(stock) = stock_multijoin(ui, bak) else {
        return Ok(false);
    };
    let rest: Vec<String> = split_mnu_dialogs(&stock)
        .into_iter()
        .filter(|(n, _)| !BROWSER_DIALOGS.contains(&n.as_str()))
        .map(|(_, b)| b)
        .collect();
    let mut out = String::new();
    out.push_str(&browser_list("idd_multi_lan", "host_local", false, accent));
    out.push_str(&browser_list("idd_multi_world", "host_world", true, accent));
    out.push_str(&browser_chrome(accent));
    for d in rest {
        out.push('\n');
        out.push_str(&d);
        out.push('\n');
    }
    write_text(&ui.join("multijoin.mnu"), &out)?;
    Ok(true)
}

pub(crate) fn splice_options(ui: &Path, bak: Option<&Path>, accent: [u8; 3]) -> Result<bool, String> {
    let Some(stock) = stock_options(ui, bak) else {
        return Ok(false);
    };
    let dialogs = split_mnu_dialogs(&stock);
    let mut out = String::new();
    out.push_str(&options_chrome(accent));
    for (name, body) in dialogs {
        if name.eq_ignore_ascii_case(OPTIONS_CHROME) {
            continue;
        }
        out.push('\n');
        let pane = OPTIONS_PANES
            .iter()
            .any(|p| name.eq_ignore_ascii_case(p));
        out.push_str(&recolor_options_dialog(&body, accent, pane));
        out.push('\n');
    }
    write_text(&ui.join("options.mnu"), &out)?;
    Ok(true)
}

/// Loading / connection wait: center Cancel on the night-ink button sprite.

/// Loading / connection wait: center Cancel on the night-ink button sprite.
pub(crate) fn splice_connection(ui: &Path, bak: Option<&Path>, accent: [u8; 3]) -> Result<bool, String> {
    let Some(stock) = stock_mnu(ui, bak, "connection.mnu") else {
        return Ok(false);
    };
    let mut out = String::new();
    for (name, body) in split_mnu_dialogs(&stock) {
        out.push('\n');
        if name.eq_ignore_ascii_case("connection_dialog") {
            out.push_str(&restyle_connection_dialog(&body, accent));
        } else {
            out.push_str(&body);
        }
        out.push('\n');
    }
    write_text(&ui.join("connection.mnu"), &out)?;
    Ok(true)
}

pub(crate) fn restyle_connection_dialog(body: &str, accent: [u8; 3]) -> String {
    let idle = argb(255, TEXT);
    let hot = argb(255, accent);
    let text = bgr(TEXT);
    let tip_y = (0.600000 - 0.566667 - 0.022222) * 0.5;
    let mut s = body.replace("\r\n", "\n").replace('\r', "\n");
    // Frosted modal — same glass as Profiles delete (600×200).
    s = s.replace("sprite dialog600x200.tga", "sprite profiledeleteboard.tga");
    // Status + detail labels are stock black / dark grey.
    s = s.replace("\tcolor 255 0 0 0\n", &format!("\tcolor 255 {text}\n"));
    s = s.replace("\tcolor 255 40 40 40\n", &format!("\tcolor 255 {text}\n"));
    s = center_bike_chrome_button(&s, "id_cancel", tip_y);
    s = paint_bike_chrome_button(&s, "id_cancel", &idle, &hot, &hot);
    s
}

/// Horizontally center a button label (`align right` + inset pos → `align center`).

pub(crate) fn splice_testsetup(ui: &Path, bak: Option<&Path>, accent: [u8; 3]) -> Result<bool, String> {
    let Some(stock) = stock_mnu(ui, bak, "testsetup.mnu") else {
        return Ok(false);
    };
    let dialogs = split_mnu_dialogs(&stock);
    let mut out = String::new();
    out.push_str(&testing_setup_chrome(accent));
    for (name, body) in dialogs {
        if name.eq_ignore_ascii_case(SETUP_CHROME) {
            continue;
        }
        out.push('\n');
        if name.eq_ignore_ascii_case(TRACK_INFO_DIALOG) {
            out.push_str(&track_info_dialog(accent));
        } else if name.eq_ignore_ascii_case(BIKE_CHOOSE)
            || name.eq_ignore_ascii_case(BIKE_CHOOSE2)
        {
            out.push_str(&restyle_bikechoose_dialog(&body, accent));
        } else if name.eq_ignore_ascii_case(BIKE_INFO) {
            out.push_str(&restyle_bikeinfo_dialog(&body, accent));
        } else {
            out.push_str(&recolor_testsetup_dialog(&body, accent));
        }
        out.push('\n');
    }
    write_text(&ui.join("testsetup.mnu"), &out)?;
    Ok(true)
}

pub(crate) fn splice_profiles(ui: &Path, bak: Option<&Path>, accent: [u8; 3]) -> Result<bool, String> {
    let Some(stock) = stock_mnu(ui, bak, "profiles.mnu") else {
        return Ok(false);
    };
    let dialogs = split_mnu_dialogs(&stock);
    let mut out = String::new();
    for (name, body) in dialogs {
        out.push('\n');
        if name.eq_ignore_ascii_case(PROFILES_DIALOG) {
            out.push_str(&restyle_profiles_dialog(&body, accent));
        } else if name.eq_ignore_ascii_case(PROFILE_MODIFY) {
            out.push_str(&restyle_profilemodify_dialog(&body, accent));
        } else if name.eq_ignore_ascii_case(PROFILE_DELETE) {
            out.push_str(&restyle_profiledelete_dialog(&body, accent));
        } else if name.eq_ignore_ascii_case(BESTLAPS_DIALOG) {
            out.push_str(&restyle_bestlaps_dialog(&body, accent));
        } else if name.eq_ignore_ascii_case(BESTLAPS_OVERALL)
            || name.eq_ignore_ascii_case(BESTLAPS_BIKE)
        {
            out.push_str(&restyle_bestlaps_list(&body, accent));
        } else {
            out.push_str(&recolor_profiles_dialog(&body, accent));
        }
        out.push('\n');
    }
    write_text(&ui.join("profiles.mnu"), &out)?;
    Ok(true)
}

/// Shared menu ink (TEXT / accent / pulls / section clears). Board remaps stay caller-side.

pub(crate) fn restyle_profiles_dialog(body: &str, accent: [u8; 3]) -> String {
    let mut s = recolor_menu_ink(body, accent);
    s = glass_wide_board(&s, "dialog1580x730.tga", "profilesboard.tga");
    s = clear_title_wash(&s);
    s = clear_row_field_fills(&s);
    s = inject_pull_fieldboxes(&s);
    s = inject_editbox_fieldboxes(&s);
    s = clear_footer_bar(&s);
    s = force_idle_plaques(&s);
    s = lift_chrome_rect(&s, "rect 0.875000 0.966667 1.000000 1.000000", 0.800000, 0.970000);
    // New/Modify/Delete stay TEXT/accent; Done is CTA.
    let tip_y = (SETUP_CHROME_Y1 - SETUP_CHROME_Y0 - 0.022222) * 0.5;
    let idle = argb(255, TEXT);
    let hot = argb(255, accent);
    let ink = argb(255, ink_on_rgb(accent));
    for name in ["id_done", "id_new", "id_modify", "id_delete"] {
        s = center_bike_chrome_button(&s, name, tip_y);
    }
    s = paint_bike_chrome_button(&s, "id_done", &ink, &ink, &ink);
    for name in ["id_new", "id_modify", "id_delete"] {
        s = paint_bike_chrome_button(&s, name, &idle, &hot, &hot);
    }
    s
}

pub(crate) fn restyle_profilemodify_dialog(body: &str, accent: [u8; 3]) -> String {
    let tip_y = (0.655556 - 0.622222 - 0.022222) * 0.5;
    let mut s = recolor_menu_ink(body, accent);
    s = s.replace("sprite dialog600x300.tga", "sprite profilemodifyboard.tga");
    s = s.replace("color 127 0 0 0", "color 160 0 0 0");
    // Footer strip under Ok/Cancel — clear so plaques float.
    s = s.replace(
        "rect 0.318750 0.622222 0.681250 0.655556\n\t\tcolor 160 0 0 0",
        "rect 0.318750 0.622222 0.681250 0.655556\n\t\tcolor 0 0 0 0",
    );
    s = clear_row_field_fills(&s);
    s = inject_pull_fieldboxes(&s);
    s = inject_editbox_fieldboxes(&s);
    s = force_idle_plaques(&s);
    let idle = argb(255, TEXT);
    let hot = argb(255, accent);
    for name in ["id_ok", "id_cancel"] {
        s = center_bike_chrome_button(&s, name, tip_y);
        s = paint_bike_chrome_button(&s, name, &idle, &hot, &hot);
    }
    s
}

/// 600×200 YES/NO confirm chrome (scrim, plaques, TEXT title).
/// `solid_board`: keep opaque `dialog600x200.tga`; otherwise frosted `profiledeleteboard.tga`.

/// 600×200 YES/NO confirm chrome (scrim, plaques, TEXT title).
/// `solid_board`: keep opaque `dialog600x200.tga`; otherwise frosted `profiledeleteboard.tga`.
pub(crate) fn restyle_yes_no_confirm(
    body: &str,
    accent: [u8; 3],
    yes_name: &str,
    no_name: &str,
    solid_board: bool,
) -> String {
    let idle = argb(255, TEXT);
    let hot = argb(255, accent);
    let tip_y = (0.600000 - 0.566667 - 0.022222) * 0.5;
    let mut s = recolor_profiles_dialog(body, accent);
    if !solid_board {
        s = s.replace("sprite dialog600x200.tga", "sprite profiledeleteboard.tga");
    }
    s = s.replace("color 127 0 0 0", "color 160 0 0 0");
    s = s.replace(
        "rect 0.318750 0.566667 0.681250 0.600000\n\t\tcolor 160 0 0 0",
        "rect 0.318750 0.566667 0.681250 0.600000\n\t\tcolor 0 0 0 0",
    );
    s = s.replace(
        "sprite2 b_button2.tga\n\t\t\tsprite3 b_button3.tga",
        "sprite1 button1.tga\n\t\t\tsprite2 button2.tga\n\t\t\tsprite3 button3.tga",
    );
    for name in [yes_name, no_name] {
        s = center_bike_chrome_button(&s, name, tip_y);
        s = paint_bike_chrome_button(&s, name, &idle, &hot, &hot);
    }
    s
}

pub(crate) fn restyle_yes_no_glass_confirm(
    body: &str,
    accent: [u8; 3],
    yes_name: &str,
    no_name: &str,
) -> String {
    restyle_yes_no_confirm(body, accent, yes_name, no_name, false)
}

pub(crate) fn restyle_yes_no_solid_confirm(
    body: &str,
    accent: [u8; 3],
    yes_name: &str,
    no_name: &str,
) -> String {
    restyle_yes_no_confirm(body, accent, yes_name, no_name, true)
}

pub(crate) fn restyle_profiledelete_dialog(body: &str, accent: [u8; 3]) -> String {
    restyle_yes_no_glass_confirm(body, accent, "id_yes", "id_no")
}

pub(crate) fn restyle_bestlaps_dialog(body: &str, accent: [u8; 3]) -> String {
    let mut s = recolor_menu_ink(body, accent);
    s = glass_wide_board(&s, "dialog1580x730.tga", "profilesboard.tga");
    s = clear_title_wash(&s);
    s = clear_footer_bar(&s);
    s = force_idle_plaques(&s);
    s = lift_chrome_rect(&s, "rect 0.000000 0.966667 0.100000 1.000000", 0.030000, 0.140000);
    // Export beside Back (same slot as Bike Selection Info).
    s = lift_chrome_rect(&s, "rect 0.012500 0.900000 0.137500 0.933333", 0.160000, 0.300000);
    paint_chrome_row(&s, accent, &["id_back", "id_export"], &[])
}

pub(crate) fn restyle_bestlaps_list(body: &str, accent: [u8; 3]) -> String {
    let text = argb(255, TEXT);
    let hot = argb(255, accent);
    let row = bgr(ROW);
    let mut s = recolor_profiles_dialog(body, accent);
    // Frosted glass list fill (recolor turns beige into opaque ROW).
    s = s.replace(&format!("backcolor 255 {row}"), "backcolor 160 0 0 0");
    s = s.replace("backcolor 255 220 220 220", "backcolor 160 0 0 0");
    // Sort bar: same frosted black as the list body (no darker ROW band).
    s = s.replace("sortbackcolor 255 180 180 180", "sortbackcolor 160 0 0 0");
    s = s.replace(&format!("sortbackcolor {}", argb(160, ROW)), "sortbackcolor 160 0 0 0");
    s = s.replace("sortbartextcolor 255 0 0 0", &format!("sortbartextcolor {text}"));
    // Force remaining stock list blues → accent.
    s = s.replace("color2 255 0 0 255", &format!("color2 {hot}"));
    s = s.replace("color3 255 0 255 255", &format!("color3 {hot}"));
    s
}

pub(crate) fn splice_viewreplays(ui: &Path, bak: Option<&Path>, accent: [u8; 3]) -> Result<bool, String> {
    let Some(stock) = stock_mnu(ui, bak, "viewreplays.mnu") else {
        return Ok(false);
    };
    let dialogs = split_mnu_dialogs(&stock);
    let mut out = String::new();
    for (name, body) in dialogs {
        out.push('\n');
        if name.eq_ignore_ascii_case(VIEWREPLAYS_DIALOG) {
            out.push_str(&restyle_viewreplays_dialog(&body, accent));
        } else if name.eq_ignore_ascii_case(VIEWREPLAYS_DELETE) {
            out.push_str(&restyle_viewreplays_delete_dialog(&body, accent));
        } else {
            out.push_str(&recolor_profiles_dialog(&body, accent));
        }
        out.push('\n');
    }
    write_text(&ui.join("viewreplays.mnu"), &out)?;
    Ok(true)
}

pub(crate) fn restyle_viewreplays_dialog(body: &str, accent: [u8; 3]) -> String {
    let idle = argb(255, TEXT);
    let hot = argb(255, accent);
    let row = bgr(ROW);
    let mut s = recolor_menu_ink(body, accent);
    s = glass_wide_board(&s, "dialog1580x730.tga", "profilesboard.tga");
    s = clear_title_wash(&s);
    // Frosted list + sort bar (same glass as Best Laps).
    s = s.replace(&format!("backcolor 255 {row}"), "backcolor 160 0 0 0");
    s = s.replace("backcolor 255 220 220 220", "backcolor 160 0 0 0");
    s = s.replace("sortbackcolor 255 200 200 200", "sortbackcolor 160 0 0 0");
    s = s.replace("sortbackcolor 255 180 180 180", "sortbackcolor 160 0 0 0");
    s = s.replace("sortbartextcolor 255 0 0 0", &format!("sortbartextcolor {idle}"));
    s = s.replace(
        "sortbartextcolor2 255 80 80 80",
        &format!("sortbartextcolor2 {idle}"),
    );
    s = s.replace(
        "sortbartextcolor3 255 40 60 240",
        &format!("sortbartextcolor3 {hot}"),
    );
    s = s.replace("color2 255 40 60 240", &format!("color2 {hot}"));
    s = clear_footer_bar(&s);
    s = force_idle_plaques(&s);
    s = lift_chrome_rect(&s, "rect 0.000000 0.966667 0.100000 1.000000", 0.030000, 0.140000);
    s = lift_chrome_rect(&s, "rect 0.875000 0.966667 1.000000 1.000000", 0.800000, 0.970000);
    // Delete beside Back (same slot as Best Laps Export).
    s = lift_chrome_rect(&s, "rect 0.012500 0.900000 0.137500 0.933333", 0.160000, 0.300000);
    paint_chrome_row(&s, accent, &["id_back", "id_delete"], &["id_view"])
}

pub(crate) fn restyle_viewreplays_delete_dialog(body: &str, accent: [u8; 3]) -> String {
    // Same glass confirm as Profiles delete.
    restyle_profiledelete_dialog(body, accent)
}

pub(crate) fn splice_hostsetup(ui: &Path, bak: Option<&Path>, accent: [u8; 3]) -> Result<bool, String> {
    let Some(stock) = stock_mnu(ui, bak, "hostsetup.mnu") else {
        return Ok(false);
    };
    let dialogs = split_mnu_dialogs(&stock);
    let mut out = String::new();
    for (name, body) in dialogs {
        out.push('\n');
        if name.eq_ignore_ascii_case(HOST_SETUP_DIALOG) {
            out.push_str(&restyle_host_setup(&body, accent));
        } else if name.eq_ignore_ascii_case(RACE_SETUP_DIALOG) {
            out.push_str(&restyle_race_setup(&body, accent));
        } else if name.eq_ignore_ascii_case(HOST_RACESETUP_DIALOG) {
            out.push_str(&restyle_host_racesetup(&body, accent));
        } else {
            out.push_str(&recolor_profiles_dialog(&body, accent));
        }
        out.push('\n');
    }
    write_text(&ui.join("hostsetup.mnu"), &out)?;
    Ok(true)
}

/// Host Setup shell — clear title, frosted board, Back / Continue chrome plaques.

/// Host Setup shell — clear title, frosted board, Back / Continue chrome plaques.
pub(crate) fn restyle_host_setup(body: &str, accent: [u8; 3]) -> String {
    let mut s = recolor_menu_ink(body, accent);
    s = glass_wide_board(&s, "dialog1580x730.tga", "profilesboard.tga");
    s = clear_title_wash(&s);
    s = clear_row_field_fills(&s);
    s = inject_editbox_fieldboxes(&s);
    s = inject_pull_fieldboxes(&s);
    s = clear_footer_bar(&s);
    s = force_idle_plaques(&s);
    s = lift_chrome_rect(&s, "rect 0.000000 0.966667 0.100000 1.000000", 0.030000, 0.140000);
    s = lift_chrome_rect(&s, "rect 0.875000 0.966667 1.000000 1.000000", 0.800000, 0.970000);
    paint_chrome_row(&s, accent, &["id_back"], &["id_continue"])
}

/// Race Setup shell — clear title, frosted track bar, Back / Bike / Start chrome.

/// Race Setup shell — clear title, frosted track bar, Back / Bike / Start chrome.
pub(crate) fn restyle_race_setup(body: &str, accent: [u8; 3]) -> String {
    let mut s = recolor_menu_ink(body, accent);
    s = clear_title_wash(&s);
    s = frost_track_bar(&s);
    s = clear_row_field_fills(&s);
    s = inject_pull_fieldboxes(&s);
    s = clear_footer_bar(&s);
    s = force_idle_plaques(&s);
    s = lift_chrome_rect(&s, "rect 0.000000 0.966667 0.100000 1.000000", 0.030000, 0.140000);
    s = lift_chrome_rect(&s, "rect 0.718750 0.966667 0.843750 1.000000", 0.440000, 0.560000);
    s = lift_chrome_rect(&s, "rect 0.875000 0.966667 1.000000 1.000000", 0.800000, 0.970000);
    // Bike Selection: charcoal plaque (not the orange Start skew).
    s = set_button_sprites(
        &s,
        "ID_BIKECHOOSE",
        "button1.tga",
        "button2.tga",
        "button3.tga",
    );
    paint_chrome_row(&s, accent, &["id_back", "ID_BIKECHOOSE"], &["id_start"])
}

/// Race Setup form board — glass right panel + field pills (Practice Setup recipe).

/// Race Setup form board — glass right panel + field pills (Practice Setup recipe).
pub(crate) fn restyle_host_racesetup(body: &str, accent: [u8; 3]) -> String {
    let mut s = recolor_testsetup_dialog(body, accent);
    s = clear_row_field_fills(&s);
    s = inject_editbox_fieldboxes(&s);
    s = inject_pull_fieldboxes(&s);
    s
}

pub(crate) fn splice_replay(ui: &Path, bak: Option<&Path>, accent: [u8; 3]) -> Result<bool, String> {
    let Some(stock) = stock_mnu(ui, bak, "replay.mnu") else {
        return Ok(false);
    };
    let dialogs = split_mnu_dialogs(&stock);
    let mut out = String::new();
    for (name, body) in dialogs {
        out.push('\n');
        if name.eq_ignore_ascii_case(REPLAY_DIALOG)
            || name.eq_ignore_ascii_case(REPLAY_STRAIGHTRHYTHM)
        {
            out.push_str(&restyle_replay_chrome(&body, accent));
        } else if name.eq_ignore_ascii_case(REPLAY_CLASSIFICATION) {
            out.push_str(&restyle_replay_classification(&body, accent));
        } else if name.eq_ignore_ascii_case(REPLAY_TELEMETRY) {
            out.push_str(&restyle_replay_telemetry(&body, accent));
        } else if name.eq_ignore_ascii_case(REPLAY_SAVE) {
            out.push_str(&restyle_replay_save(&body, accent));
        } else if name.eq_ignore_ascii_case(REPLAY_OVERWRITE) {
            out.push_str(&restyle_yes_no_glass_confirm(&body, accent, "ID_YES", "ID_NO"));
        } else {
            out.push_str(&recolor_profiles_dialog(&body, accent));
        }
        out.push('\n');
    }
    write_text(&ui.join("replay.mnu"), &out)?;
    Ok(true)
}

/// In-replay HUD chrome — frost panels + solid Save/Settings/Done (transport icons stay stock).

/// In-replay HUD chrome — frost panels + solid Save/Settings/Done (transport icons stay stock).
pub(crate) fn restyle_replay_chrome(body: &str, accent: [u8; 3]) -> String {
    let idle = argb(255, TEXT);
    let hot = argb(255, accent);
    let ink = argb(255, ink_on_rgb(accent));
    // Transport / Save row height; Settings/Done sit on the row below.
    let tip_transport = (0.955556 - 0.922222 - 0.022222) * 0.5;
    let tip_footer = (1.000000 - 0.966667 - 0.022222) * 0.5;
    let mut s = recolor_profiles_dialog(body, accent);
    // Top-right rider/camera panel + bottom bar → frosted glass bitmaps.
    s = s.replace(
        "\titem_darkbox\n\t{\n\t\tname idc_darkbox1\n\t\trect 0.675000 0.000000 1.000000 0.144444\n\t\tcolor 127 0 0 0\n\t}\n",
        "\titem_bitmap\n\t{\n\t\tname ID_REPLAY_TOP\n\t\trect 0.675000 0.000000 1.000000 0.144444\n\t\tsprite replaypanel.tga\n\t}\n",
    );
    s = s.replace(
        "\titem_darkbox\n\t{\n\t\tname idc_darkbox2\n\t\trect 0.000000 0.866667 1.000000 1.000000\n\t\tcolor 127 0 0 0\n\t}\n",
        "\titem_bitmap\n\t{\n\t\tname ID_REPLAY_FOOTER\n\t\trect 0.000000 0.866667 1.000000 1.000000\n\t\tsprite replaypanel.tga\n\t}\n",
    );
    // Leftover beige timeline wash under the scrubber.
    s = s.replace(
        "rect 0.012500 0.877778 0.987500 0.897778\n\t\tcolor 255 230 230 230",
        "rect 0.012500 0.877778 0.987500 0.897778\n\t\tcolor 0 0 0 0",
    );
    s = s.replace(
        "rect 0.012500 0.877778 0.987500 0.897778\n\t\tcolor 160 0 0 0",
        "rect 0.012500 0.877778 0.987500 0.897778\n\t\tcolor 0 0 0 0",
    );
    // Save: cream r_button → solid charcoal plaque + light ink.
    s = s.replace(
        "sprite1 r_button1.tga\n\t\t\tsprite2 r_button2.tga\n\t\t\tsprite3 r_button3.tga",
        "sprite1 button1.tga\n\t\t\tsprite2 button2.tga\n\t\t\tsprite3 button3.tga",
    );
    s = center_bike_chrome_button(&s, "id_save", tip_transport);
    s = paint_bike_chrome_button(&s, "id_save", &idle, &hot, &hot);
    // Settings / Done — keep stock Y (below transport); solid plaques.
    s = s.replace(
        "sprite2 b_button2.tga\n\t\t\tsprite3 b_button3.tga",
        "sprite1 button1.tga\n\t\t\tsprite2 button2.tga\n\t\t\tsprite3 button3.tga",
    );
    s = s.replace(
        "sprite2 done1.tga\n\t\t\tsprite3 done2.tga",
        "sprite1 done1.tga\n\t\t\tsprite2 done1.tga\n\t\t\tsprite3 done2.tga",
    );
    s = s.replace(
        "rect 0.875000 0.966667 1.000000 1.000000",
        "rect 0.800000 0.966667 0.970000 1.000000",
    );
    s = center_bike_chrome_button(&s, "ID_SETTINGS", tip_footer);
    s = paint_bike_chrome_button(&s, "ID_SETTINGS", &idle, &hot, &hot);
    s = center_bike_chrome_button(&s, "id_done", tip_footer);
    s = paint_bike_chrome_button(&s, "id_done", &ink, &ink, &ink);
    s
}

pub(crate) fn restyle_replay_classification(body: &str, accent: [u8; 3]) -> String {
    let _ = accent;
    let mut s = recolor_profiles_dialog(body, accent);
    // Title / session / list washes → frosted glass.
    s = s.replace("backcolor 127 0 0 0", "backcolor 160 0 0 0");
    s
}

pub(crate) fn restyle_replay_telemetry(body: &str, accent: [u8; 3]) -> String {
    let mut s = recolor_profiles_dialog(body, accent);
    s = s.replace(
        "\titem_darkbox\n\t{\n\t\tname ID_DARKBOX\n\t\trect 0.900000 0.700000 0.993750 0.822222\n\t\tcolor 127 0 0 0\n\t}\n",
        "\titem_bitmap\n\t{\n\t\tname ID_TELEMETRY_PANEL\n\t\trect 0.900000 0.700000 0.993750 0.822222\n\t\tsprite replaypanel.tga\n\t}\n",
    );
    s = s.replace(
        "\titem_darkbox\n\t{\n\t\tname ID_DARKBOX\n\t\trect 0.900000 0.700000 0.993750 0.822222\n\t\tcolor 160 0 0 0\n\t}\n",
        "\titem_bitmap\n\t{\n\t\tname ID_TELEMETRY_PANEL\n\t\trect 0.900000 0.700000 0.993750 0.822222\n\t\tsprite replaypanel.tga\n\t}\n",
    );
    s
}

pub(crate) fn restyle_replay_save(body: &str, accent: [u8; 3]) -> String {
    let idle = argb(255, TEXT);
    let hot = argb(255, accent);
    let tip_y = (0.822222 - 0.788889 - 0.022222) * 0.5;
    let mut s = recolor_profiles_dialog(body, accent);
    s = s.replace("sprite dialog600x600.tga", "sprite replayboard.tga");
    s = s.replace("color 127 0 0 0", "color 0 0 0 0");
    let row = bgr(ROW);
    s = s.replace(&format!("backcolor 255 {row}"), "backcolor 0 0 0 0");
    s = s.replace("backcolor 255 220 220 220", "backcolor 160 0 0 0");
    s = inject_editbox_fieldboxes(&s);
    s = s.replace(
        "sprite2 b_button2.tga\n\t\t\tsprite3 b_button3.tga",
        "sprite1 button1.tga\n\t\t\tsprite2 button2.tga\n\t\t\tsprite3 button3.tga",
    );
    for name in ["id_ok", "id_cancel"] {
        s = center_bike_chrome_button(&s, name, tip_y);
        s = paint_bike_chrome_button(&s, name, &idle, &hot, &hot);
    }
    s
}

pub(crate) fn splice_multiclient(ui: &Path, bak: Option<&Path>, accent: [u8; 3]) -> Result<bool, String> {
    let Some(stock) = stock_mnu(ui, bak, "multiclient.mnu") else {
        return Ok(false);
    };
    let dialogs = split_mnu_dialogs(&stock);
    let mut out = String::new();
    for (name, body) in dialogs {
        out.push('\n');
        if name.eq_ignore_ascii_case(MULTI_PIT) {
            out.push_str(&restyle_multi_pit(&body, accent));
        } else if name.eq_ignore_ascii_case(RACE_ENTRIES) {
            out.push_str(&restyle_race_entries(&body, accent));
        } else if name.eq_ignore_ascii_case(RACE_INFO)
            || name.eq_ignore_ascii_case(RACE_INFO2)
            || name.eq_ignore_ascii_case(RACE_RESULTS)
        {
            out.push_str(&restyle_race_info_board(&body, accent));
        } else if RACE_STANDINGS
            .iter()
            .any(|n| name.eq_ignore_ascii_case(n))
        {
            out.push_str(&restyle_race_standings_list(&body, accent));
        } else if name.eq_ignore_ascii_case(CHAT_SWITCH) {
            out.push_str(&restyle_chatswitch(&body, accent));
        } else if name.eq_ignore_ascii_case(GATE_CHOOSE) {
            out.push_str(&restyle_gatechoose(&body, accent));
        } else if name.eq_ignore_ascii_case(RACE_EXIT) {
            out.push_str(&restyle_yes_no_solid_confirm(&body, accent, "ID_YES", "ID_NO"));
        } else {
            out.push_str(&recolor_profiles_dialog(&body, accent));
        }
        out.push('\n');
    }
    write_text(&ui.join("multiclient.mnu"), &out)?;
    Ok(true)
}

pub(crate) fn pit_group_slot(i: usize) -> (f32, f32) {
    let w = (PIT_GROUP_X1 - PIT_GROUP_X0) / PIT_GROUP_N as f32;
    let x0 = PIT_GROUP_X0 + w * i as f32;
    (x0, x0 + w)
}

pub(crate) fn restyle_multi_pit(body: &str, accent: [u8; 3]) -> String {
    restyle_pit_shell(body, accent, PitShellKind::Multi)
}

pub(crate) fn restyle_testing_pit(body: &str, accent: [u8; 3]) -> String {
    restyle_pit_shell(body, accent, PitShellKind::Practice)
}

pub(crate) enum PitShellKind {
    /// Multiplayer pit: frosted race + weather bars; To Track / Join plaques.
    Multi,
    /// Practice hub: clear weather strip only; To Track is `id_start`; photo on chrome.
    Practice,
}

pub(crate) fn restyle_pit_shell(body: &str, accent: [u8; 3], kind: PitShellKind) -> String {
    let idle = argb(255, TEXT);
    let hot = argb(255, accent);
    let ink = argb(255, ink_on_rgb(accent));
    let tip_y = (SETUP_CHROME_Y1 - SETUP_CHROME_Y0 - 0.022222) * 0.5;
    let mut s = body.replace("\r\n", "\n").replace('\r', "\n");
    // Title band clear + light ink.
    s = s.replace("backcolor 127 0 0 0", "backcolor 0 0 0 0");
    s = s.replace("\tcolor 255 240 240 240\n", &format!("\tcolor {idle}\n"));
    match kind {
        PitShellKind::Multi => {
            // Race / weather strips: frosted setupbar behind clear text.
            let bars = "\titem_bitmap\n\t{\n\t\tname ID_RACEINFO_BAR\n\t\trect 0.006250 0.133333 0.993750 0.155556\n\t\tsprite setupbar.tga\n\t}\n\titem_bitmap\n\t{\n\t\tname ID_WEATHER_BAR\n\t\trect 0.006250 0.166667 0.993750 0.188889\n\t\tsprite setupbar.tga\n\t}\n";
            s = s.replace(
                "\titem_text\n\t{\n\t\tname id_weather\n",
                &format!("{bars}\titem_text\n\t{{\n\t\tname id_weather\n"),
            );
        }
        PitShellKind::Practice => {
            // Practice: remove the beige weather wash — no frosted bar.
        }
    }
    s = s.replace("backcolor 240 170 180 190", "backcolor 0 0 0 0");
    s = s.replace("\tcolor 255 0 0 0\n", &format!("\tcolor {idle}\n"));
    // Footer bar clear + Setup chrome row.
    s = s.replace(
        "rect 0.000000 0.966667 1.000000 1.000000\n\t\tcolor 127 0 0 0",
        &format!(
            "rect 0.000000 {SETUP_CHROME_Y0:.6} 1.000000 {SETUP_CHROME_Y1:.6}\n\t\tcolor 0 0 0 0"
        ),
    );
    s = s.replace(
        "sprite2 back1.tga\n\t\t\tsprite3 back2.tga",
        "sprite1 back1.tga\n\t\t\tsprite2 back1.tga\n\t\t\tsprite3 back2.tga",
    );
    s = s.replace(
        "sprite2 done1.tga\n\t\t\tsprite3 done2.tga",
        "sprite1 done1.tga\n\t\t\tsprite2 done1.tga\n\t\t\tsprite3 done2.tga",
    );
    // Back + To Track / Join / Start.
    s = s.replace(
        "rect 0.000000 0.966667 0.100000 1.000000",
        &format!("rect 0.030000 {SETUP_CHROME_Y0:.6} 0.140000 {SETUP_CHROME_Y1:.6}"),
    );
    s = s.replace(
        "rect 0.875000 0.966667 1.000000 1.000000",
        &format!("rect 0.800000 {SETUP_CHROME_Y0:.6} 0.970000 {SETUP_CHROME_Y1:.6}"),
    );
    // Segment track behind Settings / Replay / Garage.
    let track = format!(
        "\titem_bitmap\n\t{{\n\t\tname ID_PITGROUP\n\t\trect {PIT_GROUP_X0:.6} {SETUP_CHROME_Y0:.6} {PIT_GROUP_X1:.6} {SETUP_CHROME_Y1:.6}\n\t\tsprite optionstabtrack.tga\n\t}}\n"
    );
    let track_anchor = match kind {
        PitShellKind::Multi => "id_totrack",
        PitShellKind::Practice => "id_start",
    };
    s = s.replace(
        &format!("\titem_button\n\t{{\n\t\tname {track_anchor}\n"),
        &format!("{track}\titem_button\n\t{{\n\t\tname {track_anchor}\n"),
    );
    let (sx0, sx1) = pit_group_slot(0);
    let (rx0, rx1) = pit_group_slot(1);
    let (gx0, gx1) = pit_group_slot(2);
    s = s.replace(
        "rect 0.406250 0.966667 0.531250 1.000000",
        &format!("rect {sx0:.6} {SETUP_CHROME_Y0:.6} {sx1:.6} {SETUP_CHROME_Y1:.6}"),
    );
    s = s.replace(
        "rect 0.562500 0.966667 0.687500 1.000000",
        &format!("rect {rx0:.6} {SETUP_CHROME_Y0:.6} {rx1:.6} {SETUP_CHROME_Y1:.6}"),
    );
    s = s.replace(
        "rect 0.718750 0.966667 0.843750 1.000000",
        &format!("rect {gx0:.6} {SETUP_CHROME_Y0:.6} {gx1:.6} {SETUP_CHROME_Y1:.6}"),
    );
    if matches!(kind, PitShellKind::Practice) {
        // Photo icon: stock sat on the old footer between Back and Settings.
        s = s.replace(
            "rect 0.243750 0.966667 0.262500 1.000000",
            &format!("rect 0.155000 {SETUP_CHROME_Y0:.6} 0.185000 {SETUP_CHROME_Y1:.6}"),
        );
    }
    s = set_button_sprites(&s, "ID_SETTINGS", "segl1.tga", "segl2.tga", "segl2.tga");
    s = set_button_sprites(&s, "id_replay", "segm1.tga", "segm2.tga", "segm2.tga");
    s = set_button_sprites(&s, "id_garage", "segr1.tga", "segr2.tga", "segr2.tga");
    for name in ["id_back", "ID_SETTINGS", "id_replay", "id_garage"] {
        s = center_bike_chrome_button(&s, name, tip_y);
        s = paint_bike_chrome_button(&s, name, &idle, &hot, &hot);
    }
    for name in ["id_totrack", "id_join", "id_start"] {
        s = center_bike_chrome_button(&s, name, tip_y);
        s = paint_bike_chrome_button(&s, name, &ink, &ink, &ink);
    }
    // Side tabs — keep geometry; light idle / accent hot.
    s = s.replace("color1 255 240 240 240", &format!("color1 {idle}"));
    s = s.replace("color2 255 0 0 0", &format!("color2 {hot}"));
    s
}

pub(crate) fn restyle_chatswitch(body: &str, accent: [u8; 3]) -> String {
    let idle = argb(255, TEXT);
    let hot = argb(255, accent);
    let tip_y = (SETUP_CHROME_Y1 - SETUP_CHROME_Y0 - 0.022222) * 0.5;
    let mut s = body.replace("\r\n", "\n").replace('\r', "\n");
    // Shared slot between pit Garage and garage Test (Info | Chat | Test).
    s = s.replace(
        "rect 0.190625 0.966667 0.315625 1.000000",
        &format!(
            "rect {:.6} {SETUP_CHROME_Y0:.6} {:.6} {SETUP_CHROME_Y1:.6}",
            CHAT_SLOT.0, CHAT_SLOT.1
        ),
    );
    // Solid charcoal plaque — mid-seg sprites are clear (for Settings|Replay|Garage only).
    s = set_button_sprites(&s, "ID_CHAT", "button1.tga", "button2.tga", "button3.tga");
    s = center_bike_chrome_button(&s, "ID_CHAT", tip_y);
    s = paint_bike_chrome_button(&s, "ID_CHAT", &idle, &hot, &hot);
    s
}

/// In-server Gate Selection — clear title wash, chrome Done, grey gate fieldbox.

/// In-server Gate Selection — clear title wash, chrome Done, grey gate fieldbox.
pub(crate) fn restyle_gatechoose(body: &str, accent: [u8; 3]) -> String {
    let ink = argb(255, ink_on_rgb(accent));
    let tip_y = (SETUP_CHROME_Y1 - SETUP_CHROME_Y0 - 0.022222) * 0.5;
    let row = bgr(ROW);
    let mut s = recolor_profiles_dialog(body, accent);
    // Title band behind logo / "Gate Selection".
    s = s.replace("backcolor 127 0 0 0", "backcolor 0 0 0 0");
    // Cream plaque → grey fieldbox; clear pull fill so the plaque shows.
    s = s.replace("sprite dialog220x40.tga", "sprite fieldbox.tga");
    s = s.replace(&format!("backcolor 255 {row}"), "backcolor 0 0 0 0");
    s = s.replace("backcolor 255 220 220 220", "backcolor 0 0 0 0");
    // Footer clear + Done plaque (Chat overlays via idd_chatswitch).
    s = s.replace(
        "rect 0.000000 0.966667 1.000000 1.000000\n\t\tcolor 127 0 0 0",
        &format!(
            "rect 0.000000 {SETUP_CHROME_Y0:.6} 1.000000 {SETUP_CHROME_Y1:.6}\n\t\tcolor 0 0 0 0"
        ),
    );
    s = s.replace(
        "sprite2 done1.tga\n\t\t\tsprite3 done2.tga",
        "sprite1 done1.tga\n\t\t\tsprite2 done1.tga\n\t\t\tsprite3 done2.tga",
    );
    s = s.replace(
        "rect 0.875000 0.966667 1.000000 1.000000",
        &format!("rect 0.800000 {SETUP_CHROME_Y0:.6} 0.970000 {SETUP_CHROME_Y1:.6}"),
    );
    s = center_bike_chrome_button(&s, "ID_DONE", tip_y);
    s = paint_bike_chrome_button(&s, "ID_DONE", &ink, &ink, &ink);
    s
}

pub(crate) fn restyle_race_entries(body: &str, accent: [u8; 3]) -> String {
    let text = argb(255, TEXT);
    let hot = argb(255, accent);
    let mut s = recolor_profiles_dialog(body, accent);
    let row = bgr(ROW);
    s = s.replace(&format!("backcolor 255 {row}"), "backcolor 160 0 0 0");
    s = s.replace("backcolor 255 220 220 220", "backcolor 160 0 0 0");
    s = s.replace("sortbackcolor 255 180 180 180", "sortbackcolor 160 0 0 0");
    s = s.replace(&format!("sortbackcolor {}", argb(160, ROW)), "sortbackcolor 160 0 0 0");
    s = s.replace("sortbartextcolor 255 0 0 0", &format!("sortbartextcolor {text}"));
    s = s.replace("color1 255 80 80 80", &format!("color1 {text}"));
    s = s.replace("color2 255 127 127 127", &format!("color2 {text}"));
    s = s.replace("color3 255 0 255 255", &format!("color3 {hot}"));
    s
}

pub(crate) fn restyle_race_info_board(body: &str, accent: [u8; 3]) -> String {
    let idle = argb(255, TEXT);
    let hot = argb(255, accent);
    let mut s = recolor_profiles_dialog(body, accent);
    s = s.replace("sprite dialog1380x670t.tga", "sprite raceinfoboard.tga");
    s = s.replace(
        "rect 0.131250 0.200000 0.993750 0.944444",
        &format!("rect 0.131250 0.200000 0.993750 {SETUP_BOARD_Y1:.6}"),
    );
    s = glass_race_list_colors(&s, accent);
    // Results session/class pulls were cream whites — grey fieldboxes.
    s = s.replace("backcolor 255 240 240 240", "backcolor 0 0 0 0");
    s = inject_pull_fieldboxes(&s);
    // Practice Track Info: stock map ran past the board into chrome.
    s = s.replace(
        "rect 0.137500 0.211111 0.543750 0.933333",
        &format!("rect {TRACKMAP_PIT_RECT}"),
    );
    // Soften square track-image corners with a frosted mask (drawn after the map).
    s = inject_trackmap_mask(&s, TRACKMAP_PIT_RECT);
    // Multi Reset Track: bottom-right inside the board (name-scoped — Export shares stock coords).
    if let Some(at) = s.find("name ID_RESETTRACK\n") {
        let item_end = s[at..].find("\n\titem_").map(|i| at + i).unwrap_or(s.len());
        let mut btn = s[at..item_end].to_string();
        btn = btn
            .replace(
                "rect 0.862500 0.900000 0.987500 0.933333",
                "rect 0.800000 0.850000 0.968750 0.885000",
            )
            .replace(
                "rect 0.137500 0.900000 0.262500 0.933333",
                "rect 0.137500 0.850000 0.262500 0.885000",
            )
            .replace("align right", "align center")
            .replace("pos 0.006250 0.005556", "pos 0.000000 0.005556");
        if let Some(c1) = btn.find("color1 255 ") {
            let end = btn[c1..].find('\n').map(|i| c1 + i).unwrap_or(btn.len());
            btn.replace_range(c1..end, &format!("color1 {idle}"));
        }
        if let Some(c2) = btn.find("color2 255 ") {
            let end = btn[c2..].find('\n').map(|i| c2 + i).unwrap_or(btn.len());
            btn.replace_range(c2..end, &format!("color2 {hot}"));
        }
        if let Some(c3) = btn.find("color3 255 ") {
            let end = btn[c3..].find('\n').map(|i| c3 + i).unwrap_or(btn.len());
            btn.replace_range(c3..end, &format!("color3 {hot}"));
        }
        s.replace_range(at..item_end, &btn);
    }
    // Multi Admin: stock sat bottom-left above Settings — park beside Reset Track.
    if let Some(at) = s.find("name ID_ADMIN\n") {
        let item_end = s[at..].find("\n\titem_").map(|i| at + i).unwrap_or(s.len());
        let mut btn = s[at..item_end].to_string();
        btn = btn
            .replace(
                "rect 0.137500 0.900000 0.262500 0.933333",
                "rect 0.650000 0.850000 0.785000 0.885000",
            )
            .replace(
                "rect 0.137500 0.850000 0.262500 0.885000",
                "rect 0.650000 0.850000 0.785000 0.885000",
            )
            .replace("align right", "align center")
            .replace("pos 0.006250 0.005556", "pos 0.000000 0.005556");
        if let Some(c1) = btn.find("color1 255 ") {
            let end = btn[c1..].find('\n').map(|i| c1 + i).unwrap_or(btn.len());
            btn.replace_range(c1..end, &format!("color1 {idle}"));
        }
        if let Some(c2) = btn.find("color2 255 ") {
            let end = btn[c2..].find('\n').map(|i| c2 + i).unwrap_or(btn.len());
            btn.replace_range(c2..end, &format!("color2 {hot}"));
        }
        if let Some(c3) = btn.find("color3 255 ") {
            let end = btn[c3..].find('\n').map(|i| c3 + i).unwrap_or(btn.len());
            btn.replace_range(c3..end, &format!("color3 {hot}"));
        }
        s.replace_range(at..item_end, &btn);
    }
    // Practice Laps (`id_export`) / Multi Results (`ID_EXPORT`): park bottom-right.
    for export_name in ["id_export", "ID_EXPORT"] {
        let marker = format!("name {export_name}\n");
        if let Some(at) = s.find(&marker) {
            let item_end = s[at..].find("\n\titem_").map(|i| at + i).unwrap_or(s.len());
            let mut btn = s[at..item_end].to_string();
            btn = btn
                .replace(
                    "rect 0.137500 0.900000 0.262500 0.933333",
                    "rect 0.800000 0.850000 0.968750 0.885000",
                )
                .replace(
                    "rect 0.137500 0.850000 0.262500 0.885000",
                    "rect 0.800000 0.850000 0.968750 0.885000",
                )
                .replace("align right", "align center")
                .replace("pos 0.006250 0.005556", "pos 0.000000 0.005556");
            if let Some(c1) = btn.find("color1 255 ") {
                let end = btn[c1..].find('\n').map(|i| c1 + i).unwrap_or(btn.len());
                btn.replace_range(c1..end, &format!("color1 {idle}"));
            }
            if let Some(c2) = btn.find("color2 255 ") {
                let end = btn[c2..].find('\n').map(|i| c2 + i).unwrap_or(btn.len());
                btn.replace_range(c2..end, &format!("color2 {hot}"));
            }
            if let Some(c3) = btn.find("color3 255 ") {
                let end = btn[c3..].find('\n').map(|i| c3 + i).unwrap_or(btn.len());
                btn.replace_range(c3..end, &format!("color3 {hot}"));
            }
            s.replace_range(at..item_end, &btn);
        }
    }
    s
}

/// Results / standings child lists — glass fill + readable sort headers.

/// Results / standings child lists — glass fill + readable sort headers.
pub(crate) fn restyle_race_standings_list(body: &str, accent: [u8; 3]) -> String {
    let mut s = recolor_profiles_dialog(body, accent);
    s = glass_race_list_colors(&s, accent);
    // Leave room for Results Export parked at Y 0.850–0.885.
    s = s.replace(
        "rect 0.137500 0.255556 0.987500 0.877778",
        "rect 0.137500 0.255556 0.987500 0.840000",
    );
    s = s.replace(
        "rect 0.137500 0.288889 0.987500 0.877778",
        "rect 0.137500 0.288889 0.987500 0.840000",
    );
    s
}

pub(crate) fn glass_race_list_colors(body: &str, accent: [u8; 3]) -> String {
    let text = argb(255, TEXT);
    let hot = argb(255, accent);
    let row = bgr(ROW);
    let mut s = body.to_string();
    // List fills → translucent glass.
    s = s.replace(&format!("backcolor 255 {row}"), "backcolor 160 0 0 0");
    s = s.replace("backcolor 255 220 220 220", "backcolor 160 0 0 0");
    // Header bar was light grey with dark ink — unreadable once body is glass.
    s = s.replace("sortbackcolor 255 200 200 200", "sortbackcolor 160 0 0 0");
    s = s.replace("sortbackcolor 255 180 180 180", "sortbackcolor 160 0 0 0");
    s = s.replace(&format!("sortbackcolor {}", argb(160, ROW)), "sortbackcolor 160 0 0 0");
    s = s.replace("sortbartextcolor 255 0 0 0", &format!("sortbartextcolor {text}"));
    s = s.replace("color1 255 80 80 80", &format!("color1 {text}"));
    s = s.replace("color2 255 127 127 127", &format!("color2 {text}"));
    s = s.replace("color2 255 0 0 255", &format!("color2 {hot}"));
    s = s.replace("color3 255 0 255 255", &format!("color3 {hot}"));
    s
}

pub(crate) fn splice_test(ui: &Path, bak: Option<&Path>, accent: [u8; 3]) -> Result<bool, String> {
    let Some(stock) = stock_mnu(ui, bak, "test.mnu") else {
        return Ok(false);
    };
    let dialogs = split_mnu_dialogs(&stock);
    let mut out = String::new();
    for (name, body) in dialogs {
        out.push('\n');
        if name.eq_ignore_ascii_case(TESTING_PIT) {
            out.push_str(&restyle_testing_pit(&body, accent));
        } else if name.eq_ignore_ascii_case(TEST_EVENTINFO)
            || name.eq_ignore_ascii_case(TEST_LAPS)
            || name.eq_ignore_ascii_case(TEST_TRACKINFO_PIT)
            || name.eq_ignore_ascii_case(TEST_TRAINER)
        {
            out.push_str(&restyle_race_info_board(&body, accent));
        } else if name.eq_ignore_ascii_case(TEST_PANEL) {
            out.push_str(&restyle_test_panel(&body, accent));
        } else if name.eq_ignore_ascii_case(TESTING_EXIT) {
            out.push_str(&restyle_yes_no_solid_confirm(&body, accent, "ID_YES", "ID_NO"));
        } else {
            out.push_str(&recolor_profiles_dialog(&body, accent));
        }
        out.push('\n');
    }
    write_text(&ui.join("test.mnu"), &out)?;
    Ok(true)
}

/// On-track Practice menu (Continue / Return To Pit / Replay / Settings).

/// On-track Practice menu (Continue / Return To Pit / Replay / Settings).
pub(crate) fn restyle_test_panel(body: &str, accent: [u8; 3]) -> String {
    let idle = argb(255, TEXT);
    let hot = argb(255, accent);
    // Stock button height 0.033334 → center 0.022222 label.
    let tip_y = (0.033334 - 0.022222) * 0.5;
    let mut s = recolor_profiles_dialog(body, accent);
    s = s.replace("sprite dialog400x200.tga", "sprite testpanelboard.tga");
    s = s.replace(
        "sprite1 w_button1.tga\n\t\t\tsprite2 w_button2.tga\n\t\t\tsprite3 w_button3.tga",
        "sprite1 button1.tga\n\t\t\tsprite2 button2.tga\n\t\t\tsprite3 button3.tga",
    );
    for name in ["ID_CONTINUE", "ID_TOPIT", "ID_REPLAY", "ID_SETTINGS"] {
        s = center_bike_chrome_button(&s, name, tip_y);
        s = paint_bike_chrome_button(&s, name, &idle, &hot, &hot);
    }
    s
}

pub(crate) fn inject_trackmap_mask(body: &str, rect: &str) -> String {
    let needle = format!("name id_trackmap\n\t\trect {rect}\n\t}}");
    if !body.contains(&needle) || body.contains("name ID_TRACKMAP_MASK\n") {
        return body.to_string();
    }
    body.replacen(
        &needle,
        &format!(
            "name id_trackmap\n\t\trect {rect}\n\t}}\n\titem_bitmap\n\t{{\n\t\tname ID_TRACKMAP_MASK\n\t\trect {rect}\n\t\tsprite trackmapmask.tga\n\t}}"
        ),
        1,
    )
}

pub(crate) fn splice_garage(ui: &Path, bak: Option<&Path>, accent: [u8; 3]) -> Result<bool, String> {
    let Some(stock) = stock_mnu(ui, bak, "garage.mnu") else {
        return Ok(false);
    };
    let dialogs = split_mnu_dialogs(&stock);
    let mut out = String::new();
    for (name, body) in dialogs {
        out.push('\n');
        if name.eq_ignore_ascii_case(GARAGE_DIALOG) {
            out.push_str(&restyle_garage(&body, accent));
        } else if is_garage_setup_pane(&name) {
            out.push_str(&restyle_garage_pane(&body, accent));
        } else {
            out.push_str(&recolor_profiles_dialog(&body, accent));
        }
        out.push('\n');
    }
    write_text(&ui.join("garage.mnu"), &out)?;
    Ok(true)
}

pub(crate) fn is_garage_setup_pane(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.starts_with("idd_garage_")
        && (n.contains("general")
            || n.contains("suspensions")
            || n.contains("drivetrain")
            || n.contains("others"))
}

pub(crate) fn restyle_garage(body: &str, accent: [u8; 3]) -> String {
    let idle = argb(255, TEXT);
    let hot = argb(255, accent);
    let ink = argb(255, ink_on_rgb(accent));
    let tip_y = (SETUP_CHROME_Y1 - SETUP_CHROME_Y0 - 0.022222) * 0.5;
    let mut s = recolor_profiles_dialog(body, accent);
    // Title band clear.
    s = s.replace("backcolor 127 0 0 0", "backcolor 0 0 0 0");
    // Dry/Wet + Current/Compare strips → same frosted glass as boards / setupbar.
    s = s.replace(
        "\titem_darkbox\n\t{\n\t\tname ID_DARKBOX\n\t\trect 0.006250 0.133333 0.993750 0.211111\n\t\tcolor 240 170 180 190\n\t}\n",
        "\titem_bitmap\n\t{\n\t\tname ID_SETUP_BAR\n\t\trect 0.006250 0.133333 0.993750 0.211111\n\t\tsprite setupbar.tga\n\t}\n",
    );
    s = s.replace(
        "\titem_darkbox\n\t{\n\t\tname ID_DARKBOX3\n\t\trect 0.006250 0.222222 0.712500 0.266667\n\t\tcolor 240 170 180 190\n\t}\n",
        "\titem_bitmap\n\t{\n\t\tname ID_COMPARE_BAR\n\t\trect 0.006250 0.222222 0.712500 0.266667\n\t\tsprite setupbar.tga\n\t}\n",
    );
    // Any leftover beige (re-apply from cleared pack) → translucent glass darkbox.
    s = s.replace("color 240 170 180 190", "color 160 0 0 0");
    s = s.replace(
        "rect 0.006250 0.133333 0.993750 0.211111\n\t\tcolor 0 0 0 0",
        "rect 0.006250 0.133333 0.993750 0.211111\n\t\tcolor 160 0 0 0",
    );
    s = s.replace(
        "rect 0.006250 0.222222 0.712500 0.266667\n\t\tcolor 0 0 0 0",
        "rect 0.006250 0.222222 0.712500 0.266667\n\t\tcolor 160 0 0 0",
    );
    // Setup-select pulls: stock cream fills → clear; fieldbox pills sit on the glass.
    s = s.replace("backcolor 255 240 240 240", "backcolor 0 0 0 0");
    // Recover pull list panels zeroed by older packs / substring clears.
    let night = bgr(NIGHT);
    s = s.replace(
        "pullbackcolor 0 0 0 0",
        &format!("pullbackcolor 255 {night}"),
    );
    s = s.replace(
        &format!("pullbackcolor 160 {night}"),
        &format!("pullbackcolor 255 {night}"),
    );
    s = inject_pull_fieldboxes(&s);
    // Right action / tyre panels → frosted glass, stop above chrome.
    s = s.replace("sprite dialog440x170.tga", "sprite garagepanel_t.tga");
    s = s.replace("sprite dialog440x470.tga", "sprite garagepanel_r.tga");
    s = s.replace(
        "rect 0.718750 0.422222 0.993750 0.944444",
        &format!("rect 0.718750 0.422222 0.993750 {SETUP_BOARD_Y1:.6}"),
    );
    // Rear tyre strip + blueprint were sitting on the chrome row.
    s = s.replace(
        "rect 0.725000 0.888889 0.818750 0.933333",
        "rect 0.725000 0.844444 0.818750 0.888889",
    );
    s = s.replace(
        "rect 0.868750 0.511111 0.987500 0.933333",
        &format!("rect 0.868750 0.511111 0.987500 {SETUP_BOARD_Y1:.6}"),
    );
    // Footer clear + Setup chrome.
    s = s.replace(
        "rect 0.000000 0.966667 1.000000 1.000000\n\t\tcolor 127 0 0 0",
        &format!(
            "rect 0.000000 {SETUP_CHROME_Y0:.6} 1.000000 {SETUP_CHROME_Y1:.6}\n\t\tcolor 0 0 0 0"
        ),
    );
    // After recolor, darkbox may already be 160.
    s = s.replace(
        &format!("rect 0.000000 0.966667 1.000000 1.000000\n\t\tcolor 160 0 0 0"),
        &format!(
            "rect 0.000000 {SETUP_CHROME_Y0:.6} 1.000000 {SETUP_CHROME_Y1:.6}\n\t\tcolor 0 0 0 0"
        ),
    );
    s = s.replace(
        "sprite2 done1.tga\n\t\t\tsprite3 done2.tga",
        "sprite1 done1.tga\n\t\t\tsprite2 done1.tga\n\t\t\tsprite3 done2.tga",
    );
    s = s.replace(
        "sprite2 b_button2.tga\n\t\t\tsprite3 b_button3.tga",
        "sprite1 button1.tga\n\t\t\tsprite2 button2.tga\n\t\t\tsprite3 button3.tga",
    );
    // Info | Chat | Test — contiguous group (Chat via chatswitch at CHAT_SLOT).
    let act_track = format!(
        "\titem_bitmap\n\t{{\n\t\tname ID_GARAGE_ACT\n\t\trect {GARAGE_ACT_X0:.6} {SETUP_CHROME_Y0:.6} {GARAGE_ACT_X1:.6} {SETUP_CHROME_Y1:.6}\n\t\tsprite optionstabtrack.tga\n\t}}\n"
    );
    if !s.contains("name ID_GARAGE_ACT\n") {
        s = s.replace(
            "\titem_button\n\t{\n\t\tname id_done\n",
            &format!("{act_track}\titem_button\n\t{{\n\t\tname id_done\n"),
        );
    }
    let info = (GARAGE_ACT_X0, CHAT_SLOT.0);
    let test = (CHAT_SLOT.1, GARAGE_ACT_X1);
    s = s.replace(
        "rect 0.562500 0.966667 0.687500 1.000000",
        &format!(
            "rect {:.6} {SETUP_CHROME_Y0:.6} {:.6} {SETUP_CHROME_Y1:.6}",
            info.0, info.1
        ),
    );
    s = s.replace(
        "rect 0.718750 0.966667 0.843750 1.000000",
        &format!(
            "rect {:.6} {SETUP_CHROME_Y0:.6} {:.6} {SETUP_CHROME_Y1:.6}",
            test.0, test.1
        ),
    );
    s = s.replace(
        "rect 0.875000 0.966667 1.000000 1.000000",
        &format!("rect 0.820000 {SETUP_CHROME_Y0:.6} 0.970000 {SETUP_CHROME_Y1:.6}"),
    );
    s = set_button_sprites(&s, "ID_INFO", "segl1.tga", "segl2.tga", "segl2.tga");
    s = set_button_sprites(&s, "ID_TEST", "segr1.tga", "segr2.tga", "segr2.tga");
    for name in ["ID_INFO", "ID_TEST"] {
        s = center_bike_chrome_button(&s, name, tip_y);
        s = paint_bike_chrome_button(&s, name, &idle, &hot, &hot);
    }
    s = center_bike_chrome_button(&s, "id_done", tip_y);
    s = paint_bike_chrome_button(&s, "id_done", &ink, &ink, &ink);
    // General / Suspensions / Drivetrain / Others — Options-style segment group.
    s = restyle_garage_setup_tabs(&s, accent);
    s
}

/// Category tabs under the left board → contiguous segl/segm/segr + track.

/// Category tabs under the left board → contiguous segl/segm/segr + track.
pub(crate) fn restyle_garage_setup_tabs(body: &str, accent: [u8; 3]) -> String {
    let tabs = [
        "id_general_tab",
        "id_suspensions_tab",
        "id_drivetrain_tab",
        "ID_OTHERS_TAB",
    ];
    let idle = argb(255, TEXT);
    let hot = argb(255, accent);
    let tpos = (GARAGE_TAB_Y1 - GARAGE_TAB_Y0 - 0.022222) * 0.5;
    let mut s = body.to_string();
    let track = format!(
        "\titem_bitmap\n\t{{\n\t\tname ID_SETUPTABS\n\t\trect {GARAGE_TAB_X0:.6} {GARAGE_TAB_Y0:.6} {GARAGE_TAB_X1:.6} {GARAGE_TAB_Y1:.6}\n\t\tsprite optionstabtrack.tga\n\t}}\n"
    );
    // Insert track once, ahead of the first tab.
    if !s.contains("name ID_SETUPTABS\n") {
        s = s.replace(
            "\titem_tab\n\t{\n\t\tname id_general_tab\n",
            &format!("{track}\titem_tab\n\t{{\n\t\tname id_general_tab\n"),
        );
    }
    let w = (GARAGE_TAB_X1 - GARAGE_TAB_X0) / tabs.len() as f32;
    for (i, name) in tabs.iter().enumerate() {
        let x0 = GARAGE_TAB_X0 + w * i as f32;
        let x1 = x0 + w;
        let (s1, s2) = match i {
            0 => ("segl1.tga", "segl2.tga"),
            n if n + 1 == tabs.len() => ("segr1.tga", "segr2.tga"),
            _ => ("segm1.tga", "segm2.tga"),
        };
        let marker = format!("name {name}\n");
        let Some(at) = s.find(&marker) else {
            continue;
        };
        let end = s[at..].find("\n\titem_").map(|i| at + i).unwrap_or(s.len());
        let tab = s[at..end].to_string();
        // Drop old sprites / rect, rewrite geometry + segment sprites.
        let mut kept = String::new();
        for line in tab.lines() {
            let t = line.trim_start();
            if t.starts_with("sprite1 ")
                || t.starts_with("sprite2 ")
                || t.starts_with("rect ")
                || t.starts_with("pos ")
                || t.starts_with("align ")
                || t.starts_with("color1 ")
                || t.starts_with("color2 ")
            {
                continue;
            }
            kept.push_str(line);
            kept.push('\n');
        }
        // Re-insert after `group 0` (or name line).
        let insert_at = kept
            .find("group 0\n")
            .map(|i| i + "group 0\n".len())
            .unwrap_or(kept.find('\n').map(|i| i + 1).unwrap_or(0));
        kept.insert_str(
            insert_at,
            &format!(
                "\t\trect {x0:.6} {GARAGE_TAB_Y0:.6} {x1:.6} {GARAGE_TAB_Y1:.6}\n\t\tsprite1 {s1}\n\t\tsprite2 {s2}\n"
            ),
        );
        // Center label inside text {{ }}.
        if let Some(text_at) = kept.find("\t\ttext\n\t\t{\n") {
            let text_end = kept[text_at + 1..]
                .find("\n\t\t}")
                .map(|i| text_at + 1 + i)
                .unwrap_or(kept.len());
            let mut block = kept[text_at..=text_end].to_string();
            if !block.contains("align center") {
                if let Some(p) = block.find("\t\t\tfontsize ") {
                    block.insert_str(p, "\t\t\talign center\n");
                }
            }
            if !block.contains("pos 0.000000 ") {
                if let Some(p) = block.find("\t\t\tfontsize ") {
                    block.insert_str(p, &format!("\t\t\tpos 0.000000 {tpos:.6}\n"));
                }
            }
            kept.replace_range(text_at..=text_end, &block);
        }
        // Colors must stay inside the item — strip the closing brace, then re-add.
        let kept = kept.trim_end().trim_end_matches('}').trim_end().to_string();
        let kept = format!("{kept}\n\t\tcolor1 {idle}\n\t\tcolor2 {hot}\n\t}}\n");
        s.replace_range(at..end, &kept);
    }
    s
}

pub(crate) fn restyle_garage_pane(body: &str, accent: [u8; 3]) -> String {
    let idle = argb(255, TEXT);
    let hot = argb(255, accent);
    let row = bgr(ROW);
    let mut s = recolor_profiles_dialog(body, accent);
    s = s.replace("sprite dialog820x570t.tga", "sprite garageboard.tga");
    // Stock board started under the setup strips (0.277778) and painted behind the
    // section title (Fuel / Geometry / …). Drop Y0 to the title's bottom; stop above tabs.
    s = s.replace(
        "rect 0.006250 0.277778 0.518750 0.911111",
        &format!("rect 0.006250 0.311111 0.518750 {GARAGE_PANE_Y1:.6}"),
    );
    // Idempotent if a prior pack already lifted Y0 but still ends at stock Y1.
    s = s.replace(
        "rect 0.006250 0.311111 0.518750 0.911111",
        &format!("rect 0.006250 0.311111 0.518750 {GARAGE_PANE_Y1:.6}"),
    );
    // Cream (tyres/brakes) + ROW (Mapping etc.) fills → clear; fieldbox pills underneath.
    s = s.replace("backcolor 255 240 240 240", "backcolor 0 0 0 0");
    s = s.replace(&format!("backcolor 255 {row}"), "backcolor 0 0 0 0");
    // Stock Mapping used black textcolor — invisible once the fill goes dark.
    s = s.replace("textcolor 255 0 0 0", &format!("textcolor {idle}"));
    s = s.replace("textcolor 255 80 80 80", &format!("textcolor {idle}"));
    s = inject_pull_fieldboxes(&s);
    s = s.replace("color1 255 240 240 240", &format!("color1 {idle}"));
    s = s.replace("color2 255 0 0 0", &format!("color2 {hot}"));
    s
}

/// Insert a padded `fieldbox.tga` behind every `item_editbox`.

pub(crate) fn testing_setup_chrome(accent: [u8; 3]) -> String {
    let idle = argb(255, TEXT);
    let hot = argb(255, accent);
    let ink = argb(255, ink_on_rgb(accent));
    let sprites = dest_sprites();
    let cpos = (SETUP_CHROME_Y1 - SETUP_CHROME_Y0 - 0.022222) * 0.5;
    let night_rgb = bgr(NIGHT);
    // Track strip: center controls in the bar.
    const BAR_Y0: f32 = 0.133333;
    const BAR_Y1: f32 = 0.188889;
    const CTRL_H: f32 = 0.034000;
    const FONT: f32 = 0.022222;
    let ctrl_y0 = BAR_Y0 + (BAR_Y1 - BAR_Y0 - CTRL_H) * 0.5;
    let ctrl_y1 = ctrl_y0 + CTRL_H;
    let bar_pos = (CTRL_H - FONT) * 0.5;
    let bg_y0 = ctrl_y0 - 0.002;
    let bg_y1 = ctrl_y1 + 0.002;
    format!(
        r#"
dialog
{{
	name idd_testing_setup
	item_bitmap
	{{
		name ID_BITMAP
		rect 0.006250 0.000000 0.206250 0.088889
		sprite logo_ui.tga
	}}
	item_text
	{{
		name ID_TEXT
		rect 0.000000 0.000000 1.000000 0.088889
		text
		{{
			font main.fnt
			pos 0.006250 0.022222
			fontsize 0.066667
			align right
		}}
		textid testingsetup_title
		color {idle}
		backcolor 0 0 0 0
	}}
	item_bitmap
	{{
		name ID_TRACK_BAR
		rect 0.006250 {bar_y0:.6} 0.993750 {bar_y1:.6}
		sprite setupbar.tga
	}}
	item_text
	{{
		name ID_TRACKTEXT
		rect 0.012500 {ctrl_y0:.6} 0.137500 {ctrl_y1:.6}
		text
		{{
			font main.fnt
			pos 0.001250 {bar_pos:.6}
			fontsize {font:.6}
			align left
		}}
		textid track
		color {idle}
		backcolor 0 0 0 0
	}}
	item_bitmap
	{{
		name ID_TRACK_PULL_BG
		rect 0.135000 {bg_y0:.6} 0.515000 {bg_y1:.6}
		sprite fieldbox.tga
	}}
	item_pull
	{{
		name id_changetrack
		tooltip TT_TESTING
		rect 0.137500 {ctrl_y0:.6} 0.512500 {ctrl_y1:.6}
		noselection 0
		search 1
		editbox
		{{
			password 0
			text
			{{
				font main.fnt
				pos 0.003125 {bar_pos:.6}
				fontsize {font:.6}
				align left
			}}
			color1 {idle}
			color2 {hot}
			backcolor 0 0 0 0
			readonly 0
			multiline 0
			spacing 0.000000
		}}
		spacing 0.033333
		visiblerows 20
		pullbackcolor 255 {night_rgb}
		pulltextcolor1 {idle}
		pulltextcolor2 {hot}
		switchsize 0.018750
		sprite1 switch1.tga
		sprite2 switch2.tga
		sample ui_pullswitch.wav
		slider
		{{
			rect 0.481250 {ctrl_y1:.6} 0.493750 0.844445
			size 0.022222
			sprite1 scroll_box.tga
			sprite2 scroll.tga
		}}
		sliderwidth 0.012500
		spin
		{{
			buttonssize 0.018750
			type 1
			button1
			{{
				rect 0.137500 {ctrl_y0:.6} 0.156250 {ctrl_y1:.6}
				sprite1 arrow_lf1.tga
				sprite2 arrow_lf2.tga
				sprite3 arrow_lf3.tga
			}}
			button2
			{{
				rect 0.493750 {ctrl_y0:.6} 0.512500 {ctrl_y1:.6}
				sprite1 arrow_rg1.tga
				sprite2 arrow_rg2.tga
				sprite3 arrow_rg3.tga
			}}
		}}
	}}
	item_bitmap
	{{
		name ID_CATEGORY_BG
		rect 0.860000 {bg_y0:.6} 0.990000 {bg_y1:.6}
		sprite fieldbox.tga
	}}
	item_pull
	{{
		name ID_CATEGORY
		rect 0.862500 {ctrl_y0:.6} 0.987500 {ctrl_y1:.6}
		text
		{{
			font main.fnt
			pos 0.003125 {bar_pos:.6}
			fontsize 0.020000
			align left
		}}
		noselection 0
		search 0
		textcolor {idle}
		backcolor 0 0 0 0
		spacing 0.022222
		visiblerows 10
		pullbackcolor 255 {night_rgb}
		pulltextcolor1 {idle}
		pulltextcolor2 {hot}
		switchsize 0.012500
		sprite1 switch1.tga
		sprite2 switch2.tga
		sample ui_pullswitch.wav
	}}
	item_button
	{{
		name id_back
		button
		{{
			rect 0.030000 {cy0:.6} 0.140000 {cy1:.6}
			sprite1 back1.tga
			sprite2 back1.tga
			sprite3 back2.tga
			sample1 ui_button_over.wav
			sample2 ui_button_click.wav
		}}
		textid Back
		text
		{{
			font main.fnt
			pos 0.000000 {cpos:.6}
			fontsize 0.022222
			align center
		}}
		color1 {idle}
		color2 {hot}
		color3 {hot}
	}}
	item_button
	{{
		name ID_BIKECHOOSE
		guide GG_bikeselect
		button
		{{
			rect 0.650000 {cy0:.6} 0.770000 {cy1:.6}
			{sprites}
			sample1 ui_button_over.wav
			sample2 ui_button_click.wav
		}}
		textid bike_selection
		text
		{{
			font main.fnt
			pos 0.000000 {cpos:.6}
			fontsize 0.022222
			align center
		}}
		color1 {idle}
		color2 {hot}
		color3 {hot}
	}}
	item_button
	{{
		name id_start
		button
		{{
			rect 0.800000 {cy0:.6} 0.970000 {cy1:.6}
			sprite1 done1.tga
			sprite2 done1.tga
			sprite3 done2.tga
			sample1 ui_button_over.wav
			sample2 ui_button_click.wav
		}}
		textid Start
		text
		{{
			font main.fnt
			pos 0.000000 {cpos:.6}
			fontsize 0.022222
			align center
		}}
		color1 {ink}
		color2 {ink}
		color3 {ink}
	}}
}}
"#,
        cy0 = SETUP_CHROME_Y0,
        cy1 = SETUP_CHROME_Y1,
        bar_y0 = BAR_Y0,
        bar_y1 = BAR_Y1,
        font = FONT,
    )
}

/// Practice Track Info modal — glass board, dim labels, centered Close.

/// Practice Track Info modal — glass board, dim labels, centered Close.
pub(crate) fn track_info_dialog(accent: [u8; 3]) -> String {
    let idle = argb(255, TEXT);
    let dim = argb(255, DIM);
    let hot = argb(255, accent);
    // Close pill: vertically center label in stock button rect.
    const CLOSE_Y0: f32 = 0.844444;
    const CLOSE_Y1: f32 = 0.877778;
    const FONT: f32 = 0.022222;
    let close_pos = (CLOSE_Y1 - CLOSE_Y0 - FONT) * 0.5;
    // Meta column: stock x0, widen to board inner edge.
    const META_X0: f32 = 0.593750;
    const META_X1: f32 = 0.787500;
    format!(
        r#"dialog
{{
	name idd_track_info
	item_darkbox
	{{
		name ID_DARKBOX
		rect 0.000000 0.000000 1.000000 1.000000
		color 160 0 0 0
	}}
	item_bitmap
	{{
		name ID_BITMAP
		rect 0.206250 0.111111 0.793750 0.888889
		sprite trackinfoboard.tga
	}}
	item_bitmap
	{{
		name id_trackmap
		rect {TRACKMAP_SETUP_RECT}
	}}
	item_bitmap
	{{
		name ID_TRACKMAP_MASK
		rect {TRACKMAP_SETUP_RECT}
		sprite trackmapmask.tga
	}}
	item_text
	{{
		name ID_TEXT2
		rect 0.212500 0.122222 0.787500 0.144444
		text
		{{
			font main.fnt
			pos 0.000000 0.000000
			fontsize {font:.6}
			align center
		}}
		textid track_info
		color {idle}
		backcolor 0 0 0 0
	}}
	item_text
	{{
		name ID_TEXT
		rect {mx0:.6} 0.155556 {mx1:.6} 0.177778
		text
		{{
			font main.fnt
			pos 0.001250 0.000000
			fontsize {font:.6}
			align left
		}}
		textid author
		color {dim}
		backcolor 0 0 0 0
	}}
	item_text
	{{
		name ID_TRACK_AUTHOR
		rect {mx0:.6} 0.188889 {mx1:.6} 0.211111
		text
		{{
			font main.fnt
			pos 0.001250 0.000000
			fontsize {font:.6}
			align left
		}}
		color {idle}
		backcolor 0 0 0 0
	}}
	item_text
	{{
		name ID_LOCATION_TEXT
		rect {mx0:.6} 0.233333 {mx1:.6} 0.255556
		text
		{{
			font main.fnt
			pos 0.001250 0.000000
			fontsize {font:.6}
			align left
		}}
		textid location
		color {dim}
		backcolor 0 0 0 0
	}}
	item_text
	{{
		name ID_TRACK_LOCATION
		rect {mx0:.6} 0.266667 {mx1:.6} 0.288889
		text
		{{
			font main.fnt
			pos 0.001250 0.000000
			fontsize {font:.6}
			align left
		}}
		color {idle}
		backcolor 0 0 0 0
	}}
	item_text
	{{
		name ID_TEXT
		rect {mx0:.6} 0.311111 {mx1:.6} 0.333333
		text
		{{
			font main.fnt
			pos 0.001250 0.000000
			fontsize {font:.6}
			align left
		}}
		textid track_length
		color {dim}
		backcolor 0 0 0 0
	}}
	item_text
	{{
		name id_tracklength
		rect {mx0:.6} 0.344444 {mx1:.6} 0.366667
		text
		{{
			font main.fnt
			pos 0.001250 0.000000
			fontsize {font:.6}
			align left
		}}
		color {idle}
		backcolor 0 0 0 0
	}}
	item_text
	{{
		name ID_TEXT1
		rect {mx0:.6} 0.388889 {mx1:.6} 0.411111
		text
		{{
			font main.fnt
			pos 0.001250 0.000000
			fontsize {font:.6}
			align left
		}}
		textid track_altitude
		color {dim}
		backcolor 0 0 0 0
	}}
	item_text
	{{
		name id_trackaltitude
		rect {mx0:.6} 0.422222 {mx1:.6} 0.444444
		text
		{{
			font main.fnt
			pos 0.001250 0.000000
			fontsize {font:.6}
			align left
		}}
		color {idle}
		backcolor 0 0 0 0
	}}
	item_text
	{{
		name ID_PITS_TEXT
		rect {mx0:.6} 0.466667 {mx1:.6} 0.488889
		text
		{{
			font main.fnt
			pos 0.001250 0.000000
			fontsize {font:.6}
			align left
		}}
		textid pits
		color {dim}
		backcolor 0 0 0 0
	}}
	item_text
	{{
		name ID_TRACK_PITS
		rect {mx0:.6} 0.500000 {mx1:.6} 0.522222
		text
		{{
			font main.fnt
			pos 0.001250 0.000000
			fontsize {font:.6}
			align left
		}}
		color {idle}
		backcolor 0 0 0 0
	}}
	item_button
	{{
		name ID_CLOSE
		button
		{{
			rect 0.437500 {cy0:.6} 0.562500 {cy1:.6}
			sprite1 button1.tga
			sprite2 button2.tga
			sprite3 button3.tga
			sample1 ui_button_over.wav
			sample2 ui_button_click.wav
		}}
		textid close
		text
		{{
			font main.fnt
			pos 0.000000 {cpos:.6}
			fontsize {font:.6}
			align center
		}}
		color1 {idle}
		color2 {hot}
		color3 {hot}
	}}
}}
"#,
        font = FONT,
        mx0 = META_X0,
        mx1 = META_X1,
        cy0 = CLOSE_Y0,
        cy1 = CLOSE_Y1,
        cpos = close_pos,
    )
}

pub(crate) fn restyle_bikeinfo_dialog(body: &str, accent: [u8; 3]) -> String {
    let mut s = recolor_testsetup_dialog(body, accent);
    s = s.replace("sprite dialog420x200.tga", "sprite bikeinfoboard.tga");
    // Title band: stock paints a full-width A=127 behind "Bike Info".
    s = s.replace("backcolor 127 0 0 0", "backcolor 0 0 0 0");
    // Same chrome Back plaque as Practice / Bike Selection.
    s = lift_bikechoose_chrome(&s, accent);
    s
}

pub(crate) fn restyle_bikechoose_dialog(body: &str, accent: [u8; 3]) -> String {
    let mut s = recolor_testsetup_dialog(body, accent);
    s = s.replace("sprite dialog600x460.tga", "sprite bikeboard.tga");
    s = s.replace("sprite dialog320x70.tga", "sprite biketrackboard.tga");
    // Title band: stock paints a full-width A=127 behind "Bike Selection".
    s = s.replace("backcolor 127 0 0 0", "backcolor 0 0 0 0");
    // Opaque ROW field fills → clear so fieldbox pills show through.
    let row = bgr(ROW);
    s = s.replace(&format!("backcolor 255 {row}"), "backcolor 0 0 0 0");
    s = inject_pull_fieldboxes(&s);
    s = lift_bikechoose_chrome(&s, accent);
    s
}

/// Insert a padded `fieldbox.tga` bitmap behind every `item_pull`.

pub(crate) fn lift_bikechoose_chrome(dialog: &str, accent: [u8; 3]) -> String {
    let mut s = dialog.to_string();
    s = clear_footer_bar(&s);
    s = force_idle_plaques(&s);
    // Same chrome slots as Practice Setup (also lift already-chrome Y variants).
    s = lift_chrome_rect(&s, "rect 0.000000 0.966667 0.100000 1.000000", 0.030000, 0.140000);
    s = lift_chrome_rect(
        &s,
        &format!("rect 0.000000 {SETUP_CHROME_Y0:.6} 0.100000 {SETUP_CHROME_Y1:.6}"),
        0.030000,
        0.140000,
    );
    s = lift_chrome_rect(&s, "rect 0.875000 0.966667 1.000000 1.000000", 0.800000, 0.970000);
    s = lift_chrome_rect(
        &s,
        &format!("rect 0.875000 {SETUP_CHROME_Y0:.6} 1.000000 {SETUP_CHROME_Y1:.6}"),
        0.800000,
        0.970000,
    );
    s = lift_chrome_rect(&s, "rect 0.562500 0.966667 0.687500 1.000000", 0.440000, 0.560000);
    s = lift_chrome_rect(
        &s,
        &format!("rect 0.562500 {SETUP_CHROME_Y0:.6} 0.687500 {SETUP_CHROME_Y1:.6}"),
        0.440000,
        0.560000,
    );
    paint_chrome_row(&s, accent, &["id_back", "ID_INFO"], &["id_start", "id_done"])
}

pub(crate) fn recolor_testsetup_dialog(body: &str, accent: [u8; 3]) -> String {
    let mut s = recolor_menu_ink(body, accent);
    // Same frosted glass as the server board; stop above the chrome row.
    s = s.replace("sprite dialog620x670.tga", "sprite setupboard_l.tga");
    s = s.replace("sprite dialog940x670.tga", "sprite setupboard_r.tga");
    s = s.replace(
        "rect 0.006250 0.200000 0.393750 0.944444",
        &format!("rect 0.006250 0.200000 0.393750 {SETUP_BOARD_Y1:.6}"),
    );
    s = s.replace(
        "rect 0.406250 0.200000 0.993750 0.944444",
        &format!("rect 0.406250 0.200000 0.993750 {SETUP_BOARD_Y1:.6}"),
    );
    // Track Info on the chrome row, just right of Back (was covering the footer).
    s = s.replace(
        "rect 0.012500 0.900000 0.137500 0.933333",
        &format!("rect 0.160000 {SETUP_CHROME_Y0:.6} 0.300000 {SETUP_CHROME_Y1:.6}"),
    );
    s = s.replace(
        "rect 0.012500 0.855000 0.137500 0.888000",
        &format!("rect 0.160000 {SETUP_CHROME_Y0:.6} 0.300000 {SETUP_CHROME_Y1:.6}"),
    );
    let tip_y = (SETUP_CHROME_Y1 - SETUP_CHROME_Y0 - 0.022222) * 0.5;
    let mut s = center_button_label(&s, "ID_TRACK_INFO");
    // Vertically center on the chrome row (center_button_label only fixes X / align).
    if let Some(at) = s.find("name ID_TRACK_INFO\n") {
        let end = s[at..]
            .find("\n\titem_")
            .map(|i| at + i)
            .unwrap_or(s.len());
        let btn = s[at..end].replace(
            "pos 0.000000 0.005556",
            &format!("pos 0.000000 {tip_y:.6}"),
        );
        s = format!("{}{}{}", &s[..at], btn, &s[end..]);
    }
    s
}

pub(crate) fn options_chrome(accent: [u8; 3]) -> String {
    let idle = argb(255, TEXT);
    let hot = argb(255, accent);
    let dim = argb(255, DIM);
    let ink = argb(255, ink_on_rgb(accent));
    let tabs = [
        ("id_input_tab", "input_tab"),
        ("ID_INPUT2_TAB", "input2_tab"),
        ("ID_INPUT3_TAB", "input3_tab"),
        ("id_gfx_tab", "gfx_tab"),
        ("id_misc_tab", "misc_tab"),
        ("id_others_tab", "simulation"),
    ];
    // Tabs sit left of Chat/Done — Chat overlays at CHAT_SLOT while in a server.
    let group_x0 = OPT_TAB_GROUP_X0;
    let group_x1 = OPT_TAB_GROUP_X1;
    let tab_w = (group_x1 - group_x0) / tabs.len() as f32;
    let mut tab_block = String::new();
    for (i, (name, textid)) in tabs.iter().enumerate() {
        let x0 = group_x0 + tab_w * i as f32;
        let x1 = x0 + tab_w;
        let (s1, s2) = match i {
            0 => ("segl1.tga", "segl2.tga"),
            n if n + 1 == tabs.len() => ("segr1.tga", "segr2.tga"),
            _ => ("segm1.tga", "segm2.tga"),
        };
        tab_block.push_str(&format!(
            r#"	item_tab
	{{
		name {name}
		group 0
		rect {x0:.6} {ty0:.6} {x1:.6} {ty1:.6}
		sprite1 {s1}
		sprite2 {s2}
		sample2 ui_tab.wav
		text
		{{
			font main.fnt
			pos 0.000000 {tpos:.6}
			fontsize 0.018000
			align center
		}}
		textid {textid}
		color1 {idle}
		color2 {hot}
	}}
"#,
            ty0 = OPT_TAB_Y0,
            ty1 = OPT_TAB_Y1,
            // (tab_h - fontsize) / 2
            tpos = (OPT_TAB_Y1 - OPT_TAB_Y0 - 0.018) * 0.5,
        ));
    }
    // (chrome_h - fontsize) / 2 — same recipe as main menu row labels.
    let chrome_pos = (OPT_CHROME_Y1 - OPT_CHROME_Y0 - 0.022222) * 0.5;
    format!(
        r#"
dialog
{{
	name idd_options
	item_text
	{{
		name ID_TEXT
		rect 0.000000 0.000000 0.001000 0.001000
		text
		{{
			font main.fnt
			pos 0.000000 0.000000
			fontsize 0.000000
			align left
		}}
		textid settings
		color {dim}
		backcolor 0 0 0 0
	}}
	item_bitmap
	{{
		name ID_BITMAP
		rect 0.000000 0.000000 0.001000 0.001000
		sprite logo_ui.tga
	}}
	item_bitmap
	{{
		name ID_TABGROUP
		rect {group_x0:.6} {ty0:.6} {group_x1:.6} {ty1:.6}
		sprite optionstabtrack.tga
	}}
{tabs}	item_button
	{{
		name id_back
		button
		{{
			rect 0.030000 {cy0:.6} 0.140000 {cy1:.6}
			sprite1 back1.tga
			sprite2 back1.tga
			sprite3 back2.tga
			sample1 ui_button_over.wav
			sample2 ui_button_click.wav
		}}
		textid Back
		text
		{{
			font main.fnt
			pos 0.000000 {cpos:.6}
			fontsize 0.022222
			align center
		}}
		color1 {idle}
		color2 {hot}
		color3 {hot}
	}}
	item_button
	{{
		name id_done
		button
		{{
			rect 0.700000 {cy0:.6} 0.970000 {cy1:.6}
			sprite1 done1.tga
			sprite2 done1.tga
			sprite3 done2.tga
			sample1 ui_button_over.wav
			sample2 ui_button_click.wav
		}}
		textid Done
		text
		{{
			font main.fnt
			pos 0.000000 {cpos:.6}
			fontsize 0.022222
			align center
		}}
		color1 {ink}
		color2 {ink}
		color3 {ink}
	}}
}}
"#,
        tabs = tab_block,
        group_x0 = group_x0,
        group_x1 = group_x1,
        ty0 = OPT_TAB_Y0,
        ty1 = OPT_TAB_Y1,
        cy0 = OPT_CHROME_Y0,
        cy1 = OPT_CHROME_Y1,
        cpos = chrome_pos,
    )
}

pub(crate) fn recolor_options_dialog(body: &str, accent: [u8; 3], remap_to_form: bool) -> String {
    let text = bgr(TEXT);
    let hot = bgr(accent);
    let night = bgr(NIGHT);
    let row = bgr(ROW);
    let mut s = body.replace("\r\n", "\n").replace('\r', "\n");
    s = s.replace("dialog1580x700t.tga", "optionsboard.tga");
    s = s.replace("dialog1580x730.tga", "optionsboard.tga");
    if remap_to_form {
        s = remap_options_rects(&s);
    }
    // Pull/field fills first — must beat the section-header `color 255 37…` replace,
    // which otherwise matches inside `backcolor`.
    s = s.replace("backcolor 255 37 37 37", &format!("backcolor 255 {row}"));
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
    // Section headers: dark label on grey bar → light type, clear wash.
    s = s.replace("\tcolor 255 37 37 37", &format!("\tcolor 255 {text}"));
    s = s.replace("backcolor 255 163 163 163", "backcolor 0 0 0 0");
    // Opaque black section bars → clear on glass (tabbed so pullbackcolor is safe).
    s = s.replace("\tbackcolor 255 0 0 0", "\tbackcolor 0 0 0 0");
    s = s.replace("color2 255 0 0 0", &format!("color2 255 {hot}"));
    s = s.replace("color3 255 0 0 0", &format!("color3 255 {hot}"));
    s = s.replace("color3 255 255 255 255", &format!("color3 255 {hot}"));
    // Stock form labels are black / dark grey — invisible on glass.
    s = s.replace("\tcolor 255 0 0 0\n", &format!("\tcolor 255 {text}\n"));
    s = s.replace("\tcolor 255 40 40 40\n", &format!("\tcolor 255 {text}\n"));
    s = s.replace("\tcolor 255 60 60 60\n", &format!("\tcolor 255 {text}\n"));
    s = s.replace("\tcolor 255 45 45 180\n", &format!("\tcolor 255 {hot}\n"));
    s = s.replace("textcolor 255 0 0 0", &format!("textcolor 255 {text}"));
    s = s.replace("textcolor 255 60 60 60", &format!("textcolor 255 {text}"));
    s = s.replace("color1 255 60 60 60", &format!("color1 255 {text}"));
    s = s.replace("color1 255 80 80 80", &format!("color1 255 {text}"));
    s = s.replace(
        "pulltextcolor1 255 255 255 255",
        &format!("pulltextcolor1 255 {text}"),
    );
    s = s.replace("163 163 163", &text);
    s = s.replace("208 208 208", &text);
    s = s.replace("240 240 240", &text);
    s = s.replace("37 37 37", &night);
    // Stock PiBoSo blue highlight (not covered by 208→text above).
    s = s.replace("40 60 240", &hot);
    // Dropdown open-list hover stays accent (208→text above would flatten it).
    s = s.replace(
        &format!("pulltextcolor2 255 {text}"),
        &format!("pulltextcolor2 255 {hot}"),
    );
    s
}

pub(crate) fn remap_options_rects(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    for line in body.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("rect ") {
            let nums: Vec<f32> = rest
                .split_whitespace()
                .filter_map(|p| p.parse().ok())
                .collect();
            if nums.len() == 4 {
                let (x0, y0, x1, y1) = remap_options_point(nums[0], nums[1], nums[2], nums[3]);
                let indent = &line[..line.len() - trimmed.len()];
                out.push_str(indent);
                out.push_str(&format!("rect {x0:.6} {y0:.6} {x1:.6} {y1:.6}\n"));
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

pub(crate) fn remap_options_point(x0: f32, y0: f32, x1: f32, y1: f32) -> (f32, f32, f32, f32) {
    let map_x = |x: f32| {
        let t = ((x - OPT_STOCK_X0) / (OPT_STOCK_X1 - OPT_STOCK_X0)).clamp(0.0, 1.0);
        OPT_CONTENT_X0 + t * (OPT_CONTENT_X1 - OPT_CONTENT_X0)
    };
    let map_y = |y: f32| {
        let t = ((y - OPT_STOCK_Y0) / (OPT_STOCK_Y1 - OPT_STOCK_Y0)).clamp(0.0, 1.0);
        OPT_CONTENT_Y0 + t * (OPT_CONTENT_Y1 - OPT_CONTENT_Y0)
    };
    // Tiny collapsed chrome (logo etc.) stays put.
    if (x1 - x0) < 0.002 && (y1 - y0) < 0.002 {
        return (x0, y0, x1, y1);
    }
    // Full-bleed stock board bitmap → full form plaque (not the padded content box).
    let near = |a: f32, b: f32| (a - b).abs() < 0.0005;
    if near(x0, OPT_STOCK_X0)
        && near(y0, OPT_STOCK_Y0)
        && near(x1, OPT_STOCK_X1)
        && near(y1, OPT_STOCK_Y1)
    {
        return (OPT_FORM_X0, OPT_FORM_Y0, OPT_FORM_X1, OPT_FORM_Y1);
    }
    (map_x(x0), map_y(y0), map_x(x1), map_y(y1))
}

pub(crate) fn browser_list(name: &str, host_id: &str, world_filter: bool, accent: [u8; 3]) -> String {
    // ExtraBold italic needs more Name + Track than stock; pull from Status / Cat / Length.
    // `sort` is only an on/off flag in PiBoSo lists (stock uses 0/1) — not a column index.
    let (c0, c4, c5, c6, c9) = if world_filter {
        (0.300000, 0.200000, 0.090000, 0.050000, 0.080000)
    } else {
        (0.300000, 0.210000, 0.090000, 0.050000, 0.080000)
    };
    let filter = if world_filter {
        format!(
            r#"	item_bitmap
	{{
		name ID_FILTER_BG
		rect 0.412000 0.152000 0.575000 0.182000
		sprite fieldbox.tga
	}}
	item_editbox
	{{
		name ID_FILTER
		rect 0.412000 0.152000 0.575000 0.182000
		editbox
		{{
			password 0
			text
			{{
				font main.fnt
				pos 0.003125 0.005556
				fontsize 0.020000
				align left
			}}
			color1 {idle}
			color2 {dim}
			backcolor 0 0 0 0
			readonly 0
			multiline 0
			spacing 0.000000
		}}
	}}
	item_text
	{{
		name ID_TEXT
		rect 0.300000 0.152000 0.410000 0.182000
		text
		{{
			font main.fnt
			pos 0.001250 0.005556
			fontsize 0.020000
			align right
		}}
		textid filter_name
		color {ink}
		backcolor 0 0 0 0
	}}
"#,
            idle = argb(255, TEXT),
            dim = argb(255, DIM),
            ink = argb(255, INK),
        )
    } else {
        String::new()
    };
    format!(
        r#"
dialog
{{
	name {name}
	item_bitmap
	{{
		name ID_BITMAP1
		rect 0.000000 0.000000 0.001000 0.001000
	}}
{card}	item_list
	{{
		name id_sessionlist
		tooltip TT_browser
		list
		{{
			rect 0.012500 0.188889 0.987500 0.855556
			spacing 0.022222
			numcolumns 12
			columnsize0 {c0:.6}
			columnsize1 0.020000
			columnsize2 0.020000
			columnsize3 0.050000
			columnsize4 {c4:.6}
			columnsize5 {c5:.6}
			columnsize6 {c6:.6}
			columnsize7 0.040000
			columnsize8 0.020000
			columnsize9 {c9:.6}
			columnsize10 0.050000
			columnsize11 0.020000
			selectable 1
			text
			{{
				font main.fnt
				pos 0.001250 0.001111
				fontsize 0.020000
				align left
			}}
			color1 {idle}
			color2 {hot}
			color3 {hot}
			backcolor 0 0 0 0
			sortbar 1
			columntextid0 name_server
			columntextid1 d_server
			columntextid2 p_Server
			columntextid3 competitors
			columntextid4 track_server
			columntextid5 category_server
			columntextid6 length_server
			columntextid7 h_server
			columntextid8 w_server
			columntextid9 status_server
			columntextid10 ping_server
			columntextid11 rating_server
			sortbarheight 0.055556
			sortbartext
			{{
				font main.fnt
				pos 0.001250 0.005556
				fontsize 0.020000
				align left
			}}
			sortbartextcolor {idle}
			sortbartextcolor2 {idle}
			sortbartextcolor3 {hot}
			sortbackcolor 0 0 0 0
			sort 1
			sortsprite1 sort1.tga
			sortsprite2 sort2.tga
		}}
		slider
		{{
			rect 0.975000 0.244445 0.987500 0.855556
			size 0.022222
			sprite1 scroll_box.tga
			sprite2 scroll.tga
		}}
		sliderwidth 0.012500
	}}
	item_button
	{{
		name id_refreshlist
		button
		{{
			rect 0.360000 0.862000 0.480000 0.894000
			{sprites}
			sample1 ui_button_over.wav
			sample2 ui_button_click.wav
		}}
		textid Refresh_sessions
		text
		{{
			font main.fnt
			pos 0.000000 0.005556
			fontsize 0.020000
			align center
		}}
		color1 {idle}
		color2 {hot}
		color3 {hot}
	}}
	item_button
	{{
		name id_host
		button
		{{
			rect 0.032000 0.155000 0.157000 0.188000
			{sprites}
			sample1 ui_button_over.wav
			sample2 ui_button_click.wav
		}}
		textid {host_id}
		text
		{{
			font main.fnt
			pos 0.000000 0.005556
			fontsize 0.020000
			align center
		}}
		color1 {idle}
		color2 {hot}
		color3 {hot}
	}}
	item_bitmap
	{{
		name ID_PASSWORD_BG
		rect 0.790000 0.860000 0.970000 0.896000
		sprite fieldbox.tga
	}}
	item_editbox
	{{
		name id_password
		rect 0.790000 0.860000 0.970000 0.896000
		editbox
		{{
			password 0
			text
			{{
				font main.fnt
				pos 0.003125 0.005556
				fontsize 0.020000
				align left
			}}
			color1 {idle}
			color2 {dim}
			backcolor 0 0 0 0
			readonly 0
			multiline 0
			spacing 0.000000
		}}
	}}
	item_text
	{{
		name id_passwordtext
		rect 0.680000 0.860000 0.790000 0.896000
		text
		{{
			font main.fnt
			pos 0.001250 0.005556
			fontsize 0.020000
			align left
		}}
		textid passwordtext
		color {idle}
		backcolor 0 0 0 0
	}}
	item_button
	{{
		name ID_SERVERINFO
		button
		{{
			rect 0.490000 0.862000 0.610000 0.894000
			{sprites}
			sample1 ui_button_over.wav
			sample2 ui_button_click.wav
		}}
		textid info_server
		text
		{{
			font main.fnt
			pos 0.000000 0.005556
			fontsize 0.020000
			align center
		}}
		color1 {idle}
		color2 {hot}
		color3 {hot}
	}}
	item_bitmap
	{{
		name ID_HIDEMISSING_BG
		rect 0.730000 0.150000 0.975000 0.184000
		sprite button1.tga
	}}
	item_checkbox
	{{
		name ID_FILTERMISSINGCONTENTS
		rect 0.735000 0.152000 0.970000 0.182000
		spriteposx 0.000000
		spriteposy 0.004444
		spritesizex 0.012500
		spritesizey 0.022222
		sprite1 check1.tga
		sprite2 check2.tga
		sample2 ui_checkbox.wav
		text
		{{
			font main.fnt
			pos 0.015625 0.005556
			fontsize 0.020000
			align left
		}}
		textid filter_servers_missingcontent
		color {idle}
	}}
	item_bitmap
	{{
		name ID_HIDEEMPTY_BG
		rect 0.590000 0.150000 0.722000 0.184000
		sprite button1.tga
	}}
	item_checkbox
	{{
		name ID_FILTEREMTPY
		rect 0.595000 0.152000 0.718000 0.182000
		spriteposx 0.000000
		spriteposy 0.004444
		spritesizex 0.012500
		spritesizey 0.022222
		sprite1 check1.tga
		sprite2 check2.tga
		sample2 ui_checkbox.wav
		text
		{{
			font main.fnt
			pos 0.015625 0.005556
			fontsize 0.020000
			align left
		}}
		textid filter_empty_servers
		color {idle}
	}}
{filter}}}
"#,
        card = browser_card_fill(),
        sprites = dest_sprites(),
        idle = argb(255, TEXT),
        hot = argb(255, accent),
        dim = argb(255, DIM),
    )
}

pub(crate) fn browser_chrome(accent: [u8; 3]) -> String {
    let sprites = dest_sprites();
    format!(
        r#"
dialog
{{
	name idd_server_browser
	item_text
	{{
		name ID_TEXT
		rect 0.000000 0.000000 0.001000 0.001000
		text
		{{
			font main.fnt
			pos 0.000000 0.000000
			fontsize 0.000000
			align left
		}}
		textid serverbrowser_title
		color {dim}
		backcolor 0 0 0 0
	}}
	item_darkbox
	{{
		name ID_DARKBOX1
		rect 0.000000 0.000000 0.001000 0.001000
		color {bar}
	}}
	item_button
	{{
		name id_back
		button
		{{
			rect 0.030000 {cy0:.6} 0.140000 {cy1:.6}
			sprite1 back1.tga
			sprite2 back1.tga
			sprite3 back2.tga
			sample1 ui_button_over.wav
			sample2 ui_button_click.wav
		}}
		textid Back
		text
		{{
			font main.fnt
			pos 0.000000 0.005556
			fontsize 0.022222
			align center
		}}
		color1 {idle}
		color2 {hot}
		color3 {hot}
	}}
	item_bitmap
	{{
		name ID_SEGGROUP
		rect 0.160000 {cy0:.6} 0.340000 {cy1:.6}
		sprite segtrack.tga
	}}
	item_tab
	{{
		name ID_LOCAL
		group 0
		rect 0.160000 {cy0:.6} 0.250000 {cy1:.6}
		sprite1 segl1.tga
		sprite2 segl2.tga
		sample2 ui_tab.wav
		text
		{{
			font main.fnt
			pos 0.000000 0.005556
			fontsize 0.020000
			align center
		}}
		textid local
		color1 {idle}
		color2 {hot}
	}}
	item_tab
	{{
		name ID_WORLD
		guide GG_world
		group 0
		rect 0.250000 {cy0:.6} 0.340000 {cy1:.6}
		sprite1 segr1.tga
		sprite2 segr2.tga
		sample2 ui_tab.wav
		text
		{{
			font main.fnt
			pos 0.000000 0.005556
			fontsize 0.020000
			align center
		}}
		textid world
		color1 {idle}
		color2 {hot}
	}}
	item_button
	{{
		name ID_SPECTATE
		button
		{{
			rect 0.520000 {cy0:.6} 0.640000 {cy1:.6}
			{sprites}
			sample1 ui_button_over.wav
			sample2 ui_button_click.wav
		}}
		textid spectate
		text
		{{
			font main.fnt
			pos 0.000000 0.005556
			fontsize 0.022222
			align center
		}}
		color1 {idle}
		color2 {hot}
		color3 {hot}
	}}
	item_button
	{{
		name ID_BIKECHOOSE
		button
		{{
			rect 0.650000 {cy0:.6} 0.770000 {cy1:.6}
			{sprites}
			sample1 ui_button_over.wav
			sample2 ui_button_click.wav
		}}
		textid bike_selection
		text
		{{
			font main.fnt
			pos 0.000000 0.005556
			fontsize 0.022222
			align center
		}}
		color1 {idle}
		color2 {hot}
		color3 {hot}
	}}
	item_button
	{{
		name id_join
		button
		{{
			rect 0.800000 {cy0:.6} 0.970000 {cy1:.6}
			sprite1 done1.tga
			sprite2 done1.tga
			sprite3 done2.tga
			sample1 ui_button_over.wav
			sample2 ui_button_click.wav
		}}
		textid Join
		text
		{{
			font main.fnt
			pos 0.000000 0.005556
			fontsize 0.022222
			align center
		}}
		color1 {ink}
		color2 {ink}
		color3 {ink}
	}}
	item_bitmap
	{{
		name ID_BITMAP
		rect 0.000000 0.000000 0.001000 0.001000
	}}
}}
"#,
        cy0 = CHROME_Y0,
        cy1 = CHROME_Y1,
        dim = argb(255, DIM),
        bar = argb(230, NIGHT),
        idle = argb(255, TEXT),
        hot = argb(255, accent),
        ink = argb(255, ink_on_rgb(accent)),
        sprites = sprites,
    )
}

pub(crate) fn ui_ui(accent: [u8; 3]) -> String {
    format!(
        r#"
ratio 16:9

uidef
{{
	addmenu main.mnu
	addmenu testsetup.mnu
	addmenu test.mnu
	addmenu options.mnu
	addmenu replay.mnu
	addmenu viewreplays.mnu
	addmenu connection.mnu
	addmenu garage.mnu
	addmenu profiles.mnu
	addmenu multiclient.mnu
	addmenu multijoin.mnu
	addmenu hostsetup.mnu
	addmenu export.mnu
	addmenu testday.mnu
	addmenu straightrhythm.mnu
}}

pointer
{{
	normal
	{{
		sprite pointer.tga
		shadow pointer_shadow.tga
		sizex 0.022500
		sizey 0.040000
	}}
}}

tooltip
{{
	font data.fnt
	fontsize 0.020000
	fontcolor {text}
	backcolor {night}
	bordercolor {hair}
}}

guide
{{
	font data.fnt
	fontsize 0.020000
	fontcolor {text}
	backcolor {night}
	bordercolor {accent}
	arrow_tl guide_arrowtl.tga
	arrow_tr guide_arrowtr.tga
	arrow_bl guide_arrowbl.tga
	arrow_br guide_arrowbr.tga
}}

volume_attenuation 0.650000
"#,
        text = argb(255, TEXT),
        night = argb(255, NIGHT),
        hair = argb(255, HAIR),
        accent = argb(255, accent),
    )
}

pub(crate) fn dest_button(
    name: &str,
    extra: &str,
    textid: Option<&str>,
    sprites: (&str, &str, &str),
    rect: [f32; 4],
    accent: [u8; 3],
) -> String {
    let [x0, y0, x1, y1] = rect;
    let (s1, s2, s3) = sprites;
    let label_line = match textid {
        Some(id) => format!("\t\ttextid {id}\n"),
        None => String::new(),
    };
    let fontsize = if textid.is_some() {
        "0.022222"
    } else {
        "0.000000"
    };
    format!(
        r#"	item_button
	{{
		name {name}
{extra}		button
		{{
			rect {x0:.6} {y0:.6} {x1:.6} {y1:.6}
			sprite1 {s1}
			sprite2 {s2}
			sprite3 {s3}
			sample1 ui_button_over.wav
			sample2 ui_button_click.wav
		}}
{label_line}		text
		{{
			font main.fnt
			pos 0.016000 0.010889
			fontsize {fontsize}
			align left
		}}
		color1 {idle}
		color2 {hot}
		color3 {hot}
	}}
"#,
        idle = argb(255, TEXT),
        hot = argb(255, accent),
    )
}

pub(crate) fn main_mnu(accent: [u8; 3]) -> String {
    let card_x0 = 0.022000;
    let card_x1 = 0.202000;
    let pad = 0.010000;
    let row_h = 0.044000;
    let gap = 0.008000;
    let n = 8.0;
    let block = n * row_h + (n - 1.0) * gap;
    let card_y0 = 0.180000;
    let x0 = card_x0 + pad;
    let x1 = card_x1 - pad;
    // Stock logo_ui.tga is 512×128. PiBoSo rects are screen-normalized, so on
    // 16:9 (stock main.mnu) height = width × (16/9) / 4 — not width/4.
    let logo_x0 = x0;
    let logo_x1 = x1;
    let logo_w = logo_x1 - logo_x0;
    let logo_h = logo_w * (16.0 / 9.0) / 4.0;
    let logo_top = 0.004000;
    let logo_gap = 0.020000;
    let menu_h = logo_top + logo_h + logo_gap;
    // Opening splash bakes PiBoSo logo into splash.tga (connection wait blits the TGA).
    let logo_y0 = card_y0 + pad + logo_top;
    let logo_y1 = logo_y0 + logo_h;
    let card_y1 = card_y0 + pad + menu_h + block + pad;
    let dest = ("main1.tga", "main2.tga", "main3.tga");
    let mut y = card_y0 + pad + menu_h;
    let mut row = |name: &str, extra: &str, textid: Option<&str>| {
        let r = [x0, y, x1, y + row_h];
        y += row_h + gap;
        dest_button(name, extra, textid, dest, r, accent)
    };
    let rows = format!(
        "{}{}{}{}{}{}{}{}",
        row("id_testing", "", Some("Testing")),
        row("ID_MULTIPLAYER", "", Some("Single_race")),
        row("ID_BIKE", "", Some("bike_selection")),
        row("id_profiles", "		guide GG_Profiles\n", Some("profiles")),
        row("id_viewreplays", "", Some("Viewreplays")),
        row("ID_BESTLAPS", "", Some("Bestlaps")),
        row("id_settings", "		guide GG_Settings\n", Some("settings")),
        row("id_exit", "", Some("Exit")),
    );
    format!(
        r#"
dialog
{{
	name main_menu
	item_darkbox
	{{
		name ID_DARKBOX
		rect {card_x0:.6} {card_y0:.6} {card_x1:.6} {card_y1:.6}
		color 0 0 0 0
	}}
	item_bitmap
	{{
		name ID_BITMAP
		rect {card_x0:.6} {card_y0:.6} {card_x1:.6} {card_y1:.6}
		sprite mainbox.tga
	}}
	item_bitmap
	{{
		name ID_LOGO
		rect {logo_x0:.6} {logo_y0:.6} {logo_x1:.6} {logo_y1:.6}
		sprite logo_ui.tga
	}}
{rows}	item_button
	{{
		name ID_HOME
		button
		{{
			rect 0.000000 0.000000 0.001000 0.001000
			sprite1 b_home1.tga
			sprite2 b_home2.tga
			sprite3 b_home3.tga
			sample1 ui_button_over.wav
			sample2 ui_button_click.wav
		}}
		text
		{{
			pos 0.000000 0.000000
			fontsize 0.000000
			align left
		}}
		color1 255 255 255 255
		color2 255 0 255 255
		color3 0 0 0 0
	}}
	item_button
	{{
		name ID_ERROR
		button
		{{
			rect 0.000000 0.000000 0.001000 0.001000
			sample1 ui_button_over.wav
			sample2 ui_button_click.wav
		}}
		textid error_sign
		text
		{{
			font main.fnt
			pos 0.000000 0.000000
			fontsize 0.000000
			align center
		}}
		color1 {idle}
		color2 {hot}
		color3 {hot}
	}}
}}

dialog
{{
	name exit_confirm
	item_darkbox
	{{
		name idc_darkbox1
		rect 0.000000 0.000000 1.000000 1.000000
		color 160 0 0 0
	}}
	item_bitmap
	{{
		name ID_BITMAP
		rect 0.312500 0.388889 0.687500 0.611111
		sprite dialog600x200.tga
	}}
	item_darkbox
	{{
		name ID_DARKBOX1
		rect 0.318750 0.566667 0.681250 0.600000
		color 0 0 0 0
	}}
	item_button
	{{
		name id_no
		button
		{{
			rect 0.581250 0.566667 0.681250 0.600000
			sprite1 button1.tga
			sprite2 button2.tga
			sprite3 button3.tga
			sample1 ui_button_over.wav
			sample2 ui_button_click.wav
		}}
		textid no
		text
		{{
			font main.fnt
			pos 0.000000 0.005556
			fontsize 0.022222
			align center
		}}
		color1 {idle}
		color2 {hot}
		color3 {hot}
	}}
	item_button
	{{
		name id_yes
		button
		{{
			rect 0.318750 0.566667 0.418750 0.600000
			sprite1 button1.tga
			sprite2 button2.tga
			sprite3 button3.tga
			sample1 ui_button_over.wav
			sample2 ui_button_click.wav
		}}
		textid yes
		text
		{{
			font main.fnt
			pos 0.000000 0.005556
			fontsize 0.022222
			align center
		}}
		color1 {idle}
		color2 {hot}
		color3 {hot}
	}}
	item_text
	{{
		name ID_TEXT
		rect 0.318750 0.400000 0.681250 0.422222
		text
		{{
			font main.fnt
			pos 0.000000 0.000000
			fontsize 0.022222
			align center
		}}
		textid exit_confirm
		color {idle}
		backcolor 0 0 0 0
	}}
}}

dialog
{{
	name IDD_SPLASH
	item_bitmap
	{{
		name ID_BITMAP
		rect 0.000000 0.000000 1.000000 1.000000
		sprite splash.tga
	}}
}}

dialog
{{
	name idd_home
	item_bitmap
	{{
		name ID_PLACEHOLDER
		rect 0.468750 0.444444 0.531250 0.555556
	}}
}}

dialog
{{
	name idd_viewer
	item_bitmap
	{{
		name ID_BACKGROUND
		rect 0.000000 0.000000 1.000000 1.000000
	}}
	item_text
	{{
		name ID_TEXT
		rect 0.000000 0.000000 1.000000 0.088889
		text
		{{
			font main.fnt
			pos 0.006250 0.022222
			fontsize 0.066667
			align right
		}}
		textid viewer
		color {idle}
		backcolor {card}
	}}
	item_bitmap
	{{
		name ID_BITMAP
		rect 0.000000 0.000000 0.001000 0.001000
	}}
	item_darkbox
	{{
		name ID_DARKBOX
		rect 0.000000 0.966667 1.000000 1.000000
		color {bar}
	}}
	item_button
	{{
		name id_back
		button
		{{
			rect 0.000000 0.966667 0.100000 1.000000
			sprite2 back1.tga
			sprite3 back2.tga
			sample1 ui_button_over.wav
			sample2 ui_button_click.wav
		}}
		textid Back
		text
		{{
			font main.fnt
			pos 0.006250 0.005556
			fontsize 0.022222
			align left
		}}
		color1 {idle}
		color2 {hot}
		color3 {hot}
	}}
	item_bitmap
	{{
		name ID_GUIDE
		guide GG_viewer
		rect 0.468750 0.444444 0.531250 0.555556
	}}
}}

dialog
{{
	name idd_background
	item_bitmap
	{{
		name ID_BITMAP
		rect 0.000000 0.000000 1.000000 1.000000
		sprite bkgrnd.tga
	}}
}}

dialog
{{
	name idd_corruptedpaints
	item_darkbox
	{{
		name ID_DARKBOX
		rect 0.000000 0.000000 1.000000 1.000000
		color 180 0 0 0
	}}
	item_bitmap
	{{
		name ID_BITMAP2
		rect 0.250000 0.222222 0.750000 0.777778
		sprite dialog800x500.tga
	}}
	item_text
	{{
		name ID_TEXT
		rect 0.256250 0.233333 0.743750 0.255556
		text
		{{
			font main.fnt
			pos 0.000000 0.000000
			fontsize 0.022222
			align center
		}}
		textid corrupted_paints
		color {idle}
		backcolor 0 0 0 0
	}}
	item_button
	{{
		name ID_CLOSE
		button
		{{
			rect 0.437500 0.733333 0.562500 0.766667
			sprite1 button1.tga
			sprite2 button2.tga
			sprite3 button3.tga
			sample1 ui_button_over.wav
			sample2 ui_button_click.wav
		}}
		textid close
		text
		{{
			font main.fnt
			pos 0.006250 0.005556
			fontsize 0.022222
			align right
		}}
		color1 {idle}
		color2 {hot}
		color3 {hot}
	}}
	item_list
	{{
		name ID_LIST
		list
		{{
			rect 0.256250 0.266667 0.743750 0.711111
			spacing 0.022222
			numcolumns 3
			columnsize0 0.125000
			columnsize1 0.156250
			columnsize2 0.206250
			selectable 0
			text
			{{
				font main.fnt
				pos 0.001250 0.001111
				fontsize 0.020000
				align left
			}}
			color1 {idle}
			color2 {hot}
			color3 {hot}
			backcolor {list}
			sortbar 0
			sortbarheight 0.000000
			sortbartext
			{{
				pos 0.000000 0.000000
				fontsize 0.000000
				align left
			}}
			sortbartextcolor 0 0 0 0
			sortbartextcolor2 0 0 0 0
			sortbartextcolor3 0 0 0 0
			sortbackcolor 0 0 0 0
			sort 0
		}}
		slider
		{{
			rect 0.731250 0.266667 0.743750 0.711111
			size 0.022222
		}}
		sliderwidth 0.012500
	}}
}}
"#,
        card_x0 = card_x0,
        card_y0 = card_y0,
        card_x1 = card_x1,
        card_y1 = card_y1,
        card = argb(230, NIGHT),
        bar = argb(230, CHARCOAL),
        list = argb(255, [20, 20, 22]),
        idle = argb(255, TEXT),
        hot = argb(255, accent),
    )
}

