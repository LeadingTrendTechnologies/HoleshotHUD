//! F8 widget controls for the `/edit` settings list. Values live on the stream layout.

use crate::config::{
    BoardField, DashField, HudLayout, RelField, StField, WidgetId, COL_W_MIN, RADAR_RANGE_MAX,
    RADAR_RANGE_MIN,
};

pub fn controls_json(id: WidgetId, lay: &HudLayout) -> String {
    let mut out = ControlList::new();
    match id {
        WidgetId::Standings => standings(&mut out, lay),
        WidgetId::Relative => relative(&mut out, lay),
        WidgetId::Map => map(&mut out, lay),
        WidgetId::Minimap => minimap(&mut out, lay),
        WidgetId::Radar => radar(&mut out, lay),
        WidgetId::Dash => dash(&mut out, lay),
        WidgetId::Ticker => ticker(&mut out, lay),
        WidgetId::Sys => {}
        WidgetId::Sector => sector(&mut out, lay),
        WidgetId::Delta => delta(&mut out, lay),
        WidgetId::Stance => stance(&mut out, lay),
        WidgetId::Flag => flag(&mut out, lay),
        WidgetId::Lean => lean(&mut out, lay),
        WidgetId::Gamepad => gamepad(&mut out, lay),
        WidgetId::Telemetry => telemetry(&mut out, lay),
        WidgetId::Pitboard => pitboard(&mut out, lay),
        WidgetId::Timer => timer(&mut out, lay),
    }
    out.finish()
}

struct ControlList {
    body: String,
    first: bool,
}

impl ControlList {
    fn new() -> Self {
        Self {
            body: String::from("["),
            first: true,
        }
    }

    fn push(&mut self, item: String) {
        if !self.first {
            self.body.push(',');
        }
        self.first = false;
        self.body.push_str(&item);
    }

    fn finish(mut self) -> String {
        self.body.push(']');
        self.body
    }
}

fn section(list: &mut ControlList, label: &str) {
    list.push(format!(r#"{{"kind":"section","label":"{label}"}}"#));
}

fn toggle(list: &mut ControlList, key: &str, label: &str, on: bool) {
    list.push(format!(
        r#"{{"kind":"toggle","key":"{key}","label":"{label}","value":{}}}"#,
        if on { "true" } else { "false" }
    ));
}

fn range(list: &mut ControlList, key: &str, label: &str, value: i32, min: i32, max: i32) {
    list.push(format!(
        r#"{{"kind":"range","key":"{key}","label":"{label}","value":{value},"min":{min},"max":{max}}}"#
    ));
}

fn choice(list: &mut ControlList, key: &str, label: &str, value: &str, options: &str) {
    list.push(format!(
        r#"{{"kind":"choice","key":"{key}","label":"{label}","value":"{value}","options":{options}}}"#
    ));
}

fn option_list(pairs: &[(&str, &str)]) -> String {
    let mut out = String::from("[");
    for (index, (id, label)) in pairs.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&format!(r#"{{"id":"{id}","label":"{label}"}}"#));
    }
    out.push(']');
    out
}

fn pairs_from<T: Copy>(
    all: &[T],
    key: impl Fn(T) -> String,
    label: impl Fn(T) -> String,
) -> String {
    let mut out = String::from("[");
    for (index, item) in all.iter().copied().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            r#"{{"id":"{}","label":"{}"}}"#,
            key(item),
            label(item)
        ));
    }
    out.push(']');
    out
}

fn text_options() -> String {
    option_list(&[("white", "White"), ("black", "Black")])
}

fn dot_options() -> String {
    option_list(&[("num", "Number"), ("pos", "Position")])
}

fn standings(list: &mut ControlList, lay: &HudLayout) {
    range(list, "st_hl", "Row highlight", lay.standings.st_hl, 0, 100);
    choice(
        list,
        "st_text",
        "Text",
        lay.standings.st_text.key(),
        &text_options(),
    );
    toggle(list, "st_stripe", "Stripes", lay.standings.st_stripe);
    toggle(list, "st_plaque", "Plaques", lay.standings.st_plaque);
    choice(
        list,
        "st_plaque_text",
        "Plaque text",
        lay.standings.st_plaque_text.key(),
        &text_options(),
    );
    range(
        list,
        "standings_rows",
        "Rows",
        lay.standings.standings_rows,
        3,
        24,
    );
    slots(
        list,
        "st_head",
        "Header",
        &lay.standings.st_head.map(|field| field.key()),
    );
    slots(
        list,
        "st_foot",
        "Footer",
        &lay.standings.st_foot.map(|field| field.key()),
    );
    columns(list, "st_order", "Columns", &st_items(lay));
}

fn relative(list: &mut ControlList, lay: &HudLayout) {
    range(list, "rel_hl", "Row highlight", lay.relative.rel_hl, 0, 100);
    choice(
        list,
        "rel_text",
        "Text",
        lay.relative.rel_text.key(),
        &text_options(),
    );
    toggle(list, "rel_stripe", "Stripes", lay.relative.rel_stripe);
    toggle(list, "rel_plaque", "Plaques", lay.relative.rel_plaque);
    choice(
        list,
        "rel_plaque_text",
        "Plaque text",
        lay.relative.rel_plaque_text.key(),
        &text_options(),
    );
    range(
        list,
        "relative_count",
        "Riders",
        lay.relative.relative_count,
        1,
        12,
    );
    slots(
        list,
        "rel_head",
        "Header",
        &lay.relative.rel_head.map(|field| field.key()),
    );
    slots(
        list,
        "rel_foot",
        "Footer",
        &lay.relative.rel_foot.map(|field| field.key()),
    );
    columns(list, "rel_order", "Columns", &rel_items(lay));
}

fn map(list: &mut ControlList, lay: &HudLayout) {
    section(list, "On the map");
    toggle(list, "map_follow", "Follow me", lay.map.map_follow);
    if lay.map.map_follow {
        range(list, "map_zoom", "Zoom", lay.map.map_zoom, 0, 100);
    }
    toggle(list, "map_others", "Other riders", lay.map.map_others);
    toggle(list, "map_sf", "Start / finish", lay.map.map_sf);
    toggle(list, "map_sectors", "Sector lines", lay.map.map_sectors);
    toggle(list, "map_arrows", "Track arrows", lay.map.map_arrows);
    toggle(list, "map_crown", "Leader crown", lay.map.map_crown);
    toggle(
        list,
        "map_place",
        "Nearest ahead / behind",
        lay.map.map_place,
    );
    toggle(list, "map_numbers", "Numbers in dots", lay.map.map_numbers);
    if lay.map.map_numbers {
        choice(
            list,
            "map_dot",
            "Dot number",
            lay.map.map_dot.key(),
            &dot_options(),
        );
        choice(
            list,
            "map_you_text",
            "My number",
            lay.map.map_you_text.key(),
            &text_options(),
        );
    }
    range(
        list,
        "map_dot_opacity",
        "Dot opacity",
        lay.map.map_dot_opacity,
        0,
        100,
    );
}

fn minimap(list: &mut ControlList, lay: &HudLayout) {
    section(list, "On the minimap");
    toggle(list, "mini_others", "Other riders", lay.mini.mini_others);
    toggle(list, "mini_sf", "Start / finish", lay.mini.mini_sf);
    toggle(list, "mini_sectors", "Sector lines", lay.mini.mini_sectors);
    toggle(list, "mini_arrows", "Track arrows", lay.mini.mini_arrows);
    toggle(list, "mini_crown", "Leader crown", lay.mini.mini_crown);
    toggle(
        list,
        "mini_place",
        "Nearest ahead / behind",
        lay.mini.mini_place,
    );
    toggle(
        list,
        "mini_numbers",
        "Numbers in dots",
        lay.mini.mini_numbers,
    );
    choice(
        list,
        "mini_dot",
        "Dot number",
        lay.mini.mini_dot.key(),
        &dot_options(),
    );
    range(list, "mini_zoom", "Zoom", lay.mini.mini_zoom, 0, 100);
}

fn radar(list: &mut ControlList, lay: &HudLayout) {
    choice(
        list,
        "radar_style",
        "Look",
        lay.radar.radar_style.key(),
        &option_list(&[("plaque", "Plaque"), ("arrows", "Arrows")]),
    );
    section(list, "On the radar");
    range(
        list,
        "radar_range",
        "Range",
        lay.radar.radar_range,
        RADAR_RANGE_MIN,
        RADAR_RANGE_MAX,
    );
    toggle(list, "radar_sides", "Side proximity", lay.radar.radar_sides);
    toggle(list, "radar_rear", "Rear proximity", lay.radar.radar_rear);
    if lay.radar.radar_style.key() == "plaque" {
        toggle(list, "radar_rings", "Range rings", lay.radar.radar_rings);
    }
}

fn dash(list: &mut ControlList, lay: &HudLayout) {
    toggle(list, "dash_simple", "Simple dash", lay.dash.dash_simple);
    toggle(
        list,
        "dash_shift_color",
        "Shift color",
        lay.dash.dash_shift_color,
    );
    toggle(list, "dash_yellow", "Yellow flag", lay.dash.dash_yellow);
    toggle(list, "dash_blue", "Blue flag", lay.dash.dash_blue);
    toggle(list, "dash_red", "Red flag", lay.dash.dash_red);
    if lay.dash.dash_simple {
        return;
    }
    toggle(list, "dash_rev", "Rev indicator", lay.dash.dash_rev);
    let options = pairs_from(
        &DashField::ALL,
        |field| field.key().into(),
        |field| field.label().into(),
    );
    choice(
        list,
        "dash_left",
        "Left",
        lay.dash.dash_left.key(),
        &options,
    );
    choice(
        list,
        "dash_mid",
        "Middle",
        lay.dash.dash_mid.key(),
        &options,
    );
    choice(
        list,
        "dash_right",
        "Right",
        lay.dash.dash_right.key(),
        &options,
    );
}

fn ticker(list: &mut ControlList, lay: &HudLayout) {
    range(
        list,
        "ticker_hl",
        "Row highlight",
        lay.ticker.ticker_hl,
        0,
        100,
    );
    toggle(list, "ticker_title", "Track name", lay.ticker.ticker_title);
    toggle(
        list,
        "ticker_autoscroll",
        "Autoscroll",
        lay.ticker.ticker_autoscroll,
    );
    toggle(
        list,
        "ticker_slide",
        "Slide on pass",
        lay.ticker.ticker_slide,
    );
    toggle(list, "ticker_status", "Status", lay.ticker.ticker_status);
    section(list, "Side info");
    let options = pairs_from(
        &BoardField::ALL,
        |field| field.key().into(),
        |field| field.label().into(),
    );
    choice(
        list,
        "ticker_left",
        "Left",
        lay.ticker.ticker_left.key(),
        &options,
    );
    choice(
        list,
        "ticker_right",
        "Right",
        lay.ticker.ticker_right.key(),
        &options,
    );
    range(
        list,
        "ticker_count",
        "Riders shown",
        lay.ticker.ticker_count,
        3,
        15,
    );
}

fn sector(list: &mut ControlList, lay: &HudLayout) {
    toggle(list, "sector_live", "Live sector", lay.sector.sector_live);
    toggle(
        list,
        "sector_session",
        "Compare to session best",
        lay.sector.sector_session,
    );
    toggle(list, "sector_hist", "Lap log", lay.sector.sector_hist);
    range(
        list,
        "sector_hist_laps",
        "Laps back",
        lay.sector.sector_hist_laps,
        1,
        5,
    );
}

fn delta(list: &mut ControlList, lay: &HudLayout) {
    toggle(
        list,
        "delta_session",
        "Compare to session best",
        lay.delta.delta_session,
    );
}

fn stance(list: &mut ControlList, lay: &HudLayout) {
    choice(
        list,
        "stance_mode",
        "Sit mode",
        lay.stance.stance_mode.key(),
        &option_list(&[("toggle", "Toggle"), ("hold", "Hold to sit")]),
    );
    choice(
        list,
        "stance_style",
        "Look",
        lay.stance.stance_style.key(),
        &option_list(&[("text", "Text"), ("icon", "Icon")]),
    );
    toggle(
        list,
        "stance_show_sit",
        "Show sitting",
        lay.stance.stance_show_sit,
    );
}

fn flag(list: &mut ControlList, lay: &HudLayout) {
    toggle(list, "flag_text", "Text", lay.flag.flag_text);
    toggle(list, "flag_yellow", "Yellow flag", lay.flag.flag_yellow);
    toggle(list, "flag_blue", "Blue flag", lay.flag.flag_blue);
    toggle(list, "flag_red", "Red flag", lay.flag.flag_red);
}

fn lean(list: &mut ControlList, lay: &HudLayout) {
    choice(
        list,
        "lean_style",
        "Look",
        lay.lean.lean_style.key(),
        &option_list(&[("figure", "Figure"), ("minimal", "Minimal")]),
    );
}

fn gamepad(list: &mut ControlList, lay: &HudLayout) {
    choice(
        list,
        "gamepad_style",
        "Pad",
        lay.gamepad.gamepad_style.key(),
        &option_list(&[
            ("auto", "Auto"),
            ("playstation", "PlayStation"),
            ("xbox", "Xbox"),
        ]),
    );
    choice(
        list,
        "gamepad_theme",
        "Theme",
        lay.gamepad.gamepad_theme.key(),
        &option_list(&[("dark", "Dark"), ("light", "Light")]),
    );
}

fn telemetry(list: &mut ControlList, lay: &HudLayout) {
    section(list, "Show");
    toggle(
        list,
        "telemetry_traces",
        "Traces",
        lay.telemetry.telemetry_traces,
    );
    toggle(list, "telemetry_bars", "Bars", lay.telemetry.telemetry_bars);
    toggle(
        list,
        "telemetry_dial",
        "Gear / speed",
        lay.telemetry.telemetry_dial,
    );
    section(list, "Traces");
    toggle(
        list,
        "telemetry_trace_throttle",
        "Throttle",
        lay.telemetry.telemetry_trace_throttle,
    );
    toggle(
        list,
        "telemetry_trace_brake",
        "Brake",
        lay.telemetry.telemetry_trace_brake,
    );
    toggle(
        list,
        "telemetry_trace_steer",
        "Steer",
        lay.telemetry.telemetry_trace_steer,
    );
    section(list, "Bars");
    toggle(
        list,
        "telemetry_bar_clutch",
        "Clutch",
        lay.telemetry.telemetry_bar_clutch,
    );
    toggle(
        list,
        "telemetry_bar_brake",
        "Brake",
        lay.telemetry.telemetry_bar_brake,
    );
    toggle(
        list,
        "telemetry_bar_throttle",
        "Throttle",
        lay.telemetry.telemetry_bar_throttle,
    );
    toggle(
        list,
        "telemetry_bar_steer",
        "Steer",
        lay.telemetry.telemetry_bar_steer,
    );
}

fn pitboard(list: &mut ControlList, lay: &HudLayout) {
    choice(
        list,
        "pit_when",
        "Show",
        lay.pit.pit_when.key(),
        &option_list(&[
            ("always", "Always"),
            ("sector", "End of each sector"),
            ("lap", "End of each lap"),
        ]),
    );
    choice(
        list,
        "pit_text",
        "Text",
        lay.pit.pit_text.key(),
        &text_options(),
    );
    if lay.pit.pit_vars.is_empty() {
        return;
    }
    let variable_options = pairs_from(
        &mxbo_hud::pitboard::PitVar::ALL,
        |var| var.key().into(),
        |var| var.label().into(),
    );
    let options = format!(
        r#"[{{"id":"none","label":"None"}},{}]"#,
        &variable_options[1..variable_options.len() - 1]
    );
    let values: Vec<&str> = lay
        .pit
        .pit_vars
        .iter()
        .map(|place| if place.show { place.var.key() } else { "none" })
        .collect();
    list.push(format!(
        r#"{{"kind":"slots","key":"pit_vars","label":"Slots","values":{},"options":{}}}"#,
        json_strings(&values),
        options
    ));
}

fn timer(list: &mut ControlList, lay: &HudLayout) {
    toggle(list, "timer_of", "Out of riders", lay.timer.timer_of);
}

fn slots(list: &mut ControlList, key: &str, label: &str, values: &[&str; 3]) {
    let options = pairs_from(
        &BoardField::ALL,
        |field| field.key().into(),
        |field| field.label().into(),
    );
    list.push(format!(
        r#"{{"kind":"slots","key":"{key}","label":"{label}","values":{},"options":{options}}}"#,
        json_strings(values)
    ));
}

fn columns(list: &mut ControlList, key: &str, label: &str, items: &str) {
    list.push(format!(
        r#"{{"kind":"order","key":"{key}","label":"{label}","items":{items}}}"#
    ));
}

fn json_strings(values: &[&str]) -> String {
    let mut out = String::from("[");
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('"');
        out.push_str(value);
        out.push('"');
    }
    out.push(']');
    out
}

fn st_items(lay: &HudLayout) -> String {
    let mut out = String::from("[");
    for (index, field) in lay.standings.st_order.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        let (show_key, width_key, on, width) = st_bind(lay, *field);
        out.push_str(&format!(
            r#"{{"id":"{}","label":"{}","on":{},"showKey":"{show_key}","widthKey":"{width_key}","width":{width},"min":{COL_W_MIN},"max":{}}}"#,
            field.key(),
            field.label(),
            if on { "true" } else { "false" },
            field.width_max()
        ));
    }
    out.push(']');
    out
}

fn rel_items(lay: &HudLayout) -> String {
    let mut out = String::from("[");
    for (index, field) in lay.relative.rel_order.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        let (show_key, width_key, on, width) = rel_bind(lay, *field);
        out.push_str(&format!(
            r#"{{"id":"{}","label":"{}","on":{},"showKey":"{show_key}","widthKey":"{width_key}","width":{width},"min":{COL_W_MIN},"max":{}}}"#,
            field.key(),
            field.label(),
            if on { "true" } else { "false" },
            field.width_max()
        ));
    }
    out.push(']');
    out
}

fn st_bind(lay: &HudLayout, field: StField) -> (&'static str, &'static str, bool, i32) {
    match field {
        StField::Pos => (
            "st_pos",
            "st_w_pos",
            lay.standings.st_pos,
            lay.standings.st_w_pos,
        ),
        StField::Num => (
            "st_num",
            "st_w_num",
            lay.standings.st_num,
            lay.standings.st_w_num,
        ),
        StField::Name => (
            "st_name",
            "st_w_name",
            lay.standings.st_name,
            lay.standings.st_w_name,
        ),
        StField::Gap => (
            "st_gap",
            "st_w_gap",
            lay.standings.st_gap,
            lay.standings.st_w_gap,
        ),
        StField::Interval => (
            "st_interval",
            "st_w_interval",
            lay.standings.st_interval,
            lay.standings.st_w_interval,
        ),
        StField::Laps => (
            "st_laps",
            "st_w_laps",
            lay.standings.st_laps,
            lay.standings.st_w_laps,
        ),
        StField::Current => (
            "st_current",
            "st_w_current",
            lay.standings.st_current,
            lay.standings.st_w_current,
        ),
        StField::Best => (
            "st_best",
            "st_w_best",
            lay.standings.st_best,
            lay.standings.st_w_best,
        ),
        StField::Last => (
            "st_last",
            "st_w_last",
            lay.standings.st_last,
            lay.standings.st_w_last,
        ),
        StField::LapDiff => (
            "st_lapdiff",
            "st_w_lapdiff",
            lay.standings.st_lapdiff,
            lay.standings.st_w_lapdiff,
        ),
        StField::Status => (
            "st_status",
            "st_w_status",
            lay.standings.st_status,
            lay.standings.st_w_status,
        ),
        StField::Bike => (
            "st_bike",
            "st_w_bike",
            lay.standings.st_bike,
            lay.standings.st_w_bike,
        ),
        StField::Penalty => (
            "st_penalty",
            "st_w_penalty",
            lay.standings.st_penalty,
            lay.standings.st_w_penalty,
        ),
        StField::Crashed => (
            "st_crashed",
            "st_w_crashed",
            lay.standings.st_crashed,
            lay.standings.st_w_crashed,
        ),
        StField::Category => (
            "st_category",
            "st_w_category",
            lay.standings.st_category,
            lay.standings.st_w_category,
        ),
    }
}

fn rel_bind(lay: &HudLayout, field: RelField) -> (&'static str, &'static str, bool, i32) {
    match field {
        RelField::Num => (
            "rel_num",
            "rel_w_num",
            lay.relative.rel_num,
            lay.relative.rel_w_num,
        ),
        RelField::Name => (
            "rel_name",
            "rel_w_name",
            lay.relative.rel_name,
            lay.relative.rel_w_name,
        ),
        RelField::Gap => (
            "rel_gap",
            "rel_w_gap",
            lay.relative.rel_gap,
            lay.relative.rel_w_gap,
        ),
        RelField::Laps => (
            "rel_laps",
            "rel_w_laps",
            lay.relative.rel_laps,
            lay.relative.rel_w_laps,
        ),
        RelField::Current => (
            "rel_current",
            "rel_w_current",
            lay.relative.rel_current,
            lay.relative.rel_w_current,
        ),
        RelField::Pos => (
            "rel_pos",
            "rel_w_pos",
            lay.relative.rel_pos,
            lay.relative.rel_w_pos,
        ),
        RelField::Bike => (
            "rel_bike",
            "rel_w_bike",
            lay.relative.rel_bike,
            lay.relative.rel_w_bike,
        ),
        RelField::Penalty => (
            "rel_penalty",
            "rel_w_penalty",
            lay.relative.rel_penalty,
            lay.relative.rel_w_penalty,
        ),
        RelField::Interval => (
            "rel_interval",
            "rel_w_interval",
            lay.relative.rel_interval,
            lay.relative.rel_w_interval,
        ),
        RelField::Status => (
            "rel_status",
            "rel_w_status",
            lay.relative.rel_status,
            lay.relative.rel_w_status,
        ),
        RelField::Best => (
            "rel_best",
            "rel_w_best",
            lay.relative.rel_best,
            lay.relative.rel_w_best,
        ),
        RelField::Last => (
            "rel_last",
            "rel_w_last",
            lay.relative.rel_last,
            lay.relative.rel_w_last,
        ),
        RelField::LapDiff => (
            "rel_lapdiff",
            "rel_w_lapdiff",
            lay.relative.rel_lapdiff,
            lay.relative.rel_w_lapdiff,
        ),
        RelField::Category => (
            "rel_category",
            "rel_w_category",
            lay.relative.rel_category,
            lay.relative.rel_w_category,
        ),
        RelField::Speed => (
            "rel_speed",
            "rel_w_speed",
            lay.relative.rel_speed,
            lay.relative.rel_w_speed,
        ),
    }
}
