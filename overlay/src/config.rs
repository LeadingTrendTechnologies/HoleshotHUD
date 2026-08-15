use std::fs;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};
use std::time::SystemTime;

use crate::shm::{Rect, Snapshot};

pub static CONFIG: LazyLock<Mutex<HudConfig>> = LazyLock::new(|| Mutex::new(HudConfig::new()));

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StField {
    Pos,
    Num,
    Name,
    Gap,
    Interval,
    Laps,
    Best,
    Status,
    Bike,
    Penalty,
    Crashed,
}

impl StField {
    pub const ALL: [Self; 11] = [
        Self::Pos,
        Self::Num,
        Self::Name,
        Self::Gap,
        Self::Interval,
        Self::Laps,
        Self::Best,
        Self::Status,
        Self::Bike,
        Self::Penalty,
        Self::Crashed,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Self::Pos => "pos",
            Self::Num => "num",
            Self::Name => "name",
            Self::Gap => "gap",
            Self::Interval => "int",
            Self::Laps => "laps",
            Self::Best => "best",
            Self::Status => "status",
            Self::Bike => "bike",
            Self::Penalty => "pen",
            Self::Crashed => "crash",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Pos => "Position",
            Self::Num => "Number",
            Self::Name => "Name",
            Self::Gap => "Gap",
            Self::Interval => "Interval",
            Self::Laps => "Laps",
            Self::Best => "Best lap",
            Self::Status => "Status",
            Self::Bike => "Bike",
            Self::Penalty => "Penalty",
            Self::Crashed => "Crashed",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "pos" => Self::Pos,
            "num" => Self::Num,
            "name" => Self::Name,
            "gap" => Self::Gap,
            "int" | "interval" => Self::Interval,
            "laps" => Self::Laps,
            "best" => Self::Best,
            "status" => Self::Status,
            "bike" => Self::Bike,
            "pen" | "penalty" => Self::Penalty,
            "crash" | "crashed" => Self::Crashed,
            _ => return None,
        })
    }

    pub fn enabled(self, c: &HudConfig) -> bool {
        match self {
            Self::Pos => c.st_pos,
            Self::Num => c.st_num,
            Self::Name => c.st_name,
            Self::Gap => c.st_gap,
            Self::Interval => c.st_interval,
            Self::Laps => c.st_laps,
            Self::Best => c.st_best,
            Self::Status => c.st_status,
            Self::Bike => c.st_bike,
            Self::Penalty => c.st_penalty,
            Self::Crashed => c.st_crashed,
        }
    }

    pub fn width(self, c: &HudConfig) -> i32 {
        match self {
            Self::Pos => c.st_w_pos,
            Self::Num => c.st_w_num,
            Self::Name => c.st_w_name,
            Self::Gap => c.st_w_gap,
            Self::Interval => c.st_w_interval,
            Self::Laps => c.st_w_laps,
            Self::Best => c.st_w_best,
            Self::Status => c.st_w_status,
            Self::Bike => c.st_w_bike,
            Self::Penalty => c.st_w_penalty,
            Self::Crashed => c.st_w_crashed,
        }
    }

    pub fn add_width(self, c: &mut HudConfig, d: i32) {
        let next = (self.width(c) + d).clamp(18, 160);
        match self {
            Self::Pos => c.st_w_pos = next,
            Self::Num => c.st_w_num = next,
            Self::Name => c.st_w_name = next,
            Self::Gap => c.st_w_gap = next,
            Self::Interval => c.st_w_interval = next,
            Self::Laps => c.st_w_laps = next,
            Self::Best => c.st_w_best = next,
            Self::Status => c.st_w_status = next,
            Self::Bike => c.st_w_bike = next,
            Self::Penalty => c.st_w_penalty = next,
            Self::Crashed => c.st_w_crashed = next,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DotLabel {
    Number,
    Position,
}

impl DotLabel {
    pub fn key(self) -> &'static str {
        match self {
            Self::Number => "num",
            Self::Position => "pos",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Number => "Number",
            Self::Position => "Position",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim() {
            "pos" | "position" => Self::Position,
            _ => Self::Number,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RelField {
    Num,
    Name,
    Gap,
    Pos,
    Bike,
    Penalty,
    Interval,
    Crashed,
}

impl RelField {
    pub const ALL: [Self; 8] = [
        Self::Num,
        Self::Name,
        Self::Gap,
        Self::Pos,
        Self::Bike,
        Self::Penalty,
        Self::Interval,
        Self::Crashed,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Self::Num => "num",
            Self::Name => "name",
            Self::Gap => "gap",
            Self::Pos => "pos",
            Self::Bike => "bike",
            Self::Penalty => "pen",
            Self::Interval => "int",
            Self::Crashed => "crash",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Num => "Number",
            Self::Name => "Name",
            Self::Gap => "Gap",
            Self::Pos => "Position",
            Self::Bike => "Bike",
            Self::Penalty => "Penalty",
            Self::Interval => "Interval",
            Self::Crashed => "Crashed",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "num" => Self::Num,
            "name" => Self::Name,
            "gap" => Self::Gap,
            "pos" => Self::Pos,
            "bike" => Self::Bike,
            "pen" | "penalty" => Self::Penalty,
            "int" | "interval" => Self::Interval,
            "crash" | "crashed" => Self::Crashed,
            _ => return None,
        })
    }

    pub fn enabled(self, c: &HudConfig) -> bool {
        match self {
            Self::Num => c.rel_num,
            Self::Name => c.rel_name,
            Self::Gap => c.rel_gap,
            Self::Pos => c.rel_pos,
            Self::Bike => c.rel_bike,
            Self::Penalty => c.rel_penalty,
            Self::Interval => c.rel_interval,
            Self::Crashed => c.rel_crashed,
        }
    }

    pub fn width(self, c: &HudConfig) -> i32 {
        match self {
            Self::Num => c.rel_w_num,
            Self::Name => c.rel_w_name,
            Self::Gap => c.rel_w_gap,
            Self::Pos => c.rel_w_pos,
            Self::Bike => c.rel_w_bike,
            Self::Penalty => c.rel_w_penalty,
            Self::Interval => c.rel_w_interval,
            Self::Crashed => c.rel_w_crashed,
        }
    }

    pub fn add_width(self, c: &mut HudConfig, d: i32) {
        let next = (self.width(c) + d).clamp(18, 160);
        match self {
            Self::Num => c.rel_w_num = next,
            Self::Name => c.rel_w_name = next,
            Self::Gap => c.rel_w_gap = next,
            Self::Pos => c.rel_w_pos = next,
            Self::Bike => c.rel_w_bike = next,
            Self::Penalty => c.rel_w_penalty = next,
            Self::Interval => c.rel_w_interval = next,
            Self::Crashed => c.rel_w_crashed = next,
        }
    }
}

#[derive(Clone)]
pub struct HudConfig {
    pub standings: Rect,
    pub relative: Rect,
    pub map: Rect,
    pub minimap: Rect,
    pub radar: Rect,
    pub dash: Rect,
    pub show_standings: bool,
    pub show_relative: bool,
    pub show_map: bool,
    pub show_minimap: bool,
    pub show_radar: bool,
    pub show_dash: bool,
    pub ingame_hud: bool,
    pub standings_rows: i32,
    pub relative_count: i32,
    pub st_pos: bool,
    pub st_num: bool,
    pub st_name: bool,
    pub st_gap: bool,
    pub st_interval: bool,
    pub st_laps: bool,
    pub st_best: bool,
    pub st_status: bool,
    pub st_bike: bool,
    pub st_penalty: bool,
    pub st_crashed: bool,
    pub rel_num: bool,
    pub rel_name: bool,
    pub rel_gap: bool,
    pub rel_pos: bool,
    pub rel_bike: bool,
    pub rel_penalty: bool,
    pub rel_interval: bool,
    pub rel_crashed: bool,
    pub map_others: bool,
    pub map_sf: bool,
    pub map_name: bool,
    pub map_numbers: bool,
    pub map_arrows: bool,
    pub map_crown: bool,
    pub map_place: bool,
    pub map_dot: DotLabel,
    pub mini_others: bool,
    pub mini_sf: bool,
    pub mini_numbers: bool,
    pub mini_arrows: bool,
    pub mini_crown: bool,
    pub mini_place: bool,
    pub mini_dot: DotLabel,
    pub radar_sides: bool,
    pub radar_rear: bool,
    pub st_bg: i32,
    pub rel_bg: i32,
    pub map_bg: i32,
    pub mini_bg: i32,
    pub radar_bg: i32,
    pub dash_bg: i32,
    pub st_order: Vec<StField>,
    pub rel_order: Vec<RelField>,
    pub st_w_pos: i32,
    pub st_w_num: i32,
    pub st_w_name: i32,
    pub st_w_gap: i32,
    pub st_w_interval: i32,
    pub st_w_laps: i32,
    pub st_w_best: i32,
    pub st_w_status: i32,
    pub st_w_bike: i32,
    pub st_w_penalty: i32,
    pub st_w_crashed: i32,
    pub rel_w_num: i32,
    pub rel_w_name: i32,
    pub rel_w_gap: i32,
    pub rel_w_pos: i32,
    pub rel_w_bike: i32,
    pub rel_w_penalty: i32,
    pub rel_w_interval: i32,
    pub rel_w_crashed: i32,
    loaded_mtime: Option<SystemTime>,
}

impl HudConfig {
    pub fn new() -> Self {
        Self {
            standings: Rect {
                x: 0.012,
                y: 0.03,
                w: 0.30,
                h: 0.46,
            },
            relative: Rect {
                x: 0.012,
                y: 0.62,
                w: 0.30,
                h: 0.36,
            },
            map: Rect {
                x: 0.775,
                y: 0.62,
                w: 0.21,
                h: 0.34,
            },
            minimap: Rect {
                x: 0.815,
                y: 0.035,
                w: 0.165,
                h: 0.295,
            },
            radar: Rect {
                x: 0.438,
                y: 0.755,
                w: 0.124,
                h: 0.22,
            },
            dash: Rect {
                x: 0.41,
                y: 0.82,
                w: 0.18,
                h: 0.16,
            },
            show_standings: true,
            show_relative: true,
            show_map: true,
            show_minimap: true,
            show_radar: true,
            show_dash: true,
            ingame_hud: false,
            standings_rows: 12,
            relative_count: 3,
            st_pos: true,
            st_num: true,
            st_name: true,
            st_gap: true,
            st_interval: false,
            st_laps: false,
            st_best: false,
            st_status: false,
            st_bike: false,
            st_penalty: false,
            st_crashed: false,
            rel_num: true,
            rel_name: true,
            rel_gap: true,
            rel_pos: false,
            rel_bike: false,
            rel_penalty: false,
            rel_interval: false,
            rel_crashed: false,
            map_others: true,
            map_sf: true,
            map_name: true,
            map_numbers: true,
            map_arrows: true,
            map_crown: true,
            map_place: true,
            map_dot: DotLabel::Position,
            mini_others: true,
            mini_sf: true,
            mini_numbers: true,
            mini_arrows: true,
            mini_crown: true,
            mini_place: true,
            mini_dot: DotLabel::Number,
            radar_sides: true,
            radar_rear: true,
            st_bg: 78,
            rel_bg: 78,
            map_bg: 0,
            mini_bg: 0,
            radar_bg: 86,
            dash_bg: 82,
            st_order: StField::ALL.to_vec(),
            rel_order: RelField::ALL.to_vec(),
            st_w_pos: 26,
            st_w_num: 30,
            st_w_name: 80,
            st_w_gap: 58,
            st_w_interval: 58,
            st_w_laps: 32,
            st_w_best: 58,
            st_w_status: 40,
            st_w_bike: 56,
            st_w_penalty: 48,
            st_w_crashed: 44,
            rel_w_num: 32,
            rel_w_name: 80,
            rel_w_gap: 58,
            rel_w_pos: 28,
            rel_w_bike: 56,
            rel_w_penalty: 48,
            rel_w_interval: 58,
            rel_w_crashed: 44,
            loaded_mtime: None,
        }
    }

    pub fn load_file() -> Self {
        let path = ini_path();
        let mut cfg = Self::new();
        let Ok(text) = fs::read_to_string(&path) else {
            cfg.save();
            return cfg;
        };
        cfg.loaded_mtime = fs::metadata(&path).and_then(|m| m.modified()).ok();
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') || line.starts_with('[') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let key = k.trim();
            let val = v.trim();
            let f = val.parse::<f32>().unwrap_or(0.0);
            let b = val == "1" || val.eq_ignore_ascii_case("true") || val.eq_ignore_ascii_case("yes");
            match key {
                "standings_x" => cfg.standings.x = f,
                "standings_y" => cfg.standings.y = f,
                "standings_w" => cfg.standings.w = f,
                "standings_h" => cfg.standings.h = f,
                "relative_x" => cfg.relative.x = f,
                "relative_y" => cfg.relative.y = f,
                "relative_w" => cfg.relative.w = f,
                "relative_h" => cfg.relative.h = f,
                "map_x" => cfg.map.x = f,
                "map_y" => cfg.map.y = f,
                "map_w" => cfg.map.w = f,
                "map_h" => cfg.map.h = f,
                "minimap_x" => cfg.minimap.x = f,
                "minimap_y" => cfg.minimap.y = f,
                "minimap_w" => cfg.minimap.w = f,
                "minimap_h" => cfg.minimap.h = f,
                "radar_x" => cfg.radar.x = f,
                "radar_y" => cfg.radar.y = f,
                "radar_w" => cfg.radar.w = f,
                "radar_h" => cfg.radar.h = f,
                "dash_x" => cfg.dash.x = f,
                "dash_y" => cfg.dash.y = f,
                "dash_w" => cfg.dash.w = f,
                "dash_h" => cfg.dash.h = f,
                "show_standings" => cfg.show_standings = b,
                "show_relative" => cfg.show_relative = b,
                "show_map" => cfg.show_map = b,
                "show_minimap" => cfg.show_minimap = b,
                "show_radar" => cfg.show_radar = b,
                "show_dash" => cfg.show_dash = b,
                "ingame_hud" => cfg.ingame_hud = b,
                "standings_rows" => cfg.standings_rows = val.parse().unwrap_or(12).max(3),
                "relative_count" => cfg.relative_count = val.parse().unwrap_or(3).max(1),
                "st_pos" => cfg.st_pos = b,
                "st_num" => cfg.st_num = b,
                "st_name" => cfg.st_name = b,
                "st_gap" => cfg.st_gap = b,
                "st_interval" => cfg.st_interval = b,
                "st_laps" => cfg.st_laps = b,
                "st_best" => cfg.st_best = b,
                "st_status" => cfg.st_status = b,
                "st_bike" => cfg.st_bike = b,
                "st_penalty" => cfg.st_penalty = b,
                "st_crashed" => cfg.st_crashed = b,
                "rel_num" => cfg.rel_num = b,
                "rel_name" => cfg.rel_name = b,
                "rel_gap" => cfg.rel_gap = b,
                "rel_pos" => cfg.rel_pos = b,
                "rel_bike" => cfg.rel_bike = b,
                "rel_penalty" => cfg.rel_penalty = b,
                "rel_interval" => cfg.rel_interval = b,
                "rel_crashed" => cfg.rel_crashed = b,
                "map_others" => cfg.map_others = b,
                "map_sf" => cfg.map_sf = b,
                "map_name" => cfg.map_name = b,
                "map_numbers" => cfg.map_numbers = b,
                "map_arrows" => cfg.map_arrows = b,
                "map_crown" => cfg.map_crown = b,
                "map_place" => cfg.map_place = b,
                "map_dot" => cfg.map_dot = DotLabel::parse(val),
                "mini_others" => cfg.mini_others = b,
                "mini_sf" => cfg.mini_sf = b,
                "mini_numbers" => cfg.mini_numbers = b,
                "mini_arrows" => cfg.mini_arrows = b,
                "mini_crown" => cfg.mini_crown = b,
                "mini_place" => cfg.mini_place = b,
                "mini_dot" => cfg.mini_dot = DotLabel::parse(val),
                "radar_sides" => cfg.radar_sides = b,
                "radar_rear" => cfg.radar_rear = b,
                "st_bg" => cfg.st_bg = clamp_pct(val),
                "rel_bg" => cfg.rel_bg = clamp_pct(val),
                "map_bg" => cfg.map_bg = clamp_pct(val),
                "mini_bg" => cfg.mini_bg = clamp_pct(val),
                "radar_bg" => cfg.radar_bg = clamp_pct(val),
                "dash_bg" => cfg.dash_bg = clamp_pct(val),
                "st_order" => cfg.st_order = parse_st_order(val),
                "rel_order" => cfg.rel_order = parse_rel_order(val),
                "st_w_pos" => cfg.st_w_pos = clamp_w(val),
                "st_w_num" => cfg.st_w_num = clamp_w(val),
                "st_w_name" => cfg.st_w_name = clamp_w(val),
                "st_w_gap" => cfg.st_w_gap = clamp_w(val),
                "st_w_interval" => cfg.st_w_interval = clamp_w(val),
                "st_w_laps" => cfg.st_w_laps = clamp_w(val),
                "st_w_best" => cfg.st_w_best = clamp_w(val),
                "st_w_status" => cfg.st_w_status = clamp_w(val),
                "st_w_bike" => cfg.st_w_bike = clamp_w(val),
                "st_w_penalty" => cfg.st_w_penalty = clamp_w(val),
                "st_w_crashed" => cfg.st_w_crashed = clamp_w(val),
                "rel_w_num" => cfg.rel_w_num = clamp_w(val),
                "rel_w_name" => cfg.rel_w_name = clamp_w(val),
                "rel_w_gap" => cfg.rel_w_gap = clamp_w(val),
                "rel_w_pos" => cfg.rel_w_pos = clamp_w(val),
                "rel_w_bike" => cfg.rel_w_bike = clamp_w(val),
                "rel_w_penalty" => cfg.rel_w_penalty = clamp_w(val),
                "rel_w_interval" => cfg.rel_w_interval = clamp_w(val),
                "rel_w_crashed" => cfg.rel_w_crashed = clamp_w(val),
                _ => {}
            }
        }
        cfg
    }

    pub fn save(&mut self) {
        let path = ini_path();
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let body = format!(
            "# mxbo HUD layout (normalized 0..1, origin top-left)\n\
             [Layout]\n\
             standings_x={}\nstandings_y={}\nstandings_w={}\nstandings_h={}\n\
             relative_x={}\nrelative_y={}\nrelative_w={}\nrelative_h={}\n\
             map_x={}\nmap_y={}\nmap_w={}\nmap_h={}\n\
             minimap_x={}\nminimap_y={}\nminimap_w={}\nminimap_h={}\n\
             radar_x={}\nradar_y={}\nradar_w={}\nradar_h={}\n\
             dash_x={}\ndash_y={}\ndash_w={}\ndash_h={}\n\
             \n[Widgets]\n\
             show_standings={}\nshow_relative={}\nshow_map={}\nshow_minimap={}\nshow_radar={}\nshow_dash={}\n\
             ingame_hud={}\nstandings_rows={}\nrelative_count={}\n\
             \n[Standings]\n\
             st_pos={}\nst_num={}\nst_name={}\nst_gap={}\nst_interval={}\nst_laps={}\nst_best={}\nst_status={}\n\
             st_bike={}\nst_penalty={}\nst_crashed={}\n\
             st_order={}\n\
             st_w_pos={}\nst_w_num={}\nst_w_name={}\nst_w_gap={}\nst_w_interval={}\nst_w_laps={}\nst_w_best={}\nst_w_status={}\n\
             st_w_bike={}\nst_w_penalty={}\nst_w_crashed={}\n\
             st_bg={}\n\
             \n[Relative]\n\
             rel_num={}\nrel_name={}\nrel_gap={}\nrel_pos={}\nrel_bike={}\nrel_penalty={}\nrel_interval={}\nrel_crashed={}\n\
             rel_order={}\n\
             rel_w_num={}\nrel_w_name={}\nrel_w_gap={}\nrel_w_pos={}\nrel_w_bike={}\nrel_w_penalty={}\nrel_w_interval={}\nrel_w_crashed={}\n\
             rel_bg={}\n\
             \n[Map]\n\
             map_others={}\nmap_sf={}\nmap_name={}\nmap_numbers={}\nmap_arrows={}\nmap_crown={}\nmap_place={}\nmap_dot={}\n\
             map_bg={}\n\
             \n[Minimap]\n\
             mini_others={}\nmini_sf={}\nmini_numbers={}\nmini_arrows={}\nmini_crown={}\nmini_place={}\nmini_dot={}\n\
             mini_bg={}\n\
             \n[Radar]\n\
             radar_sides={}\nradar_rear={}\n\
             radar_bg={}\n\
             \n[Dash]\n\
             dash_bg={}\n",
            self.standings.x,
            self.standings.y,
            self.standings.w,
            self.standings.h,
            self.relative.x,
            self.relative.y,
            self.relative.w,
            self.relative.h,
            self.map.x,
            self.map.y,
            self.map.w,
            self.map.h,
            self.minimap.x,
            self.minimap.y,
            self.minimap.w,
            self.minimap.h,
            self.radar.x,
            self.radar.y,
            self.radar.w,
            self.radar.h,
            self.dash.x,
            self.dash.y,
            self.dash.w,
            self.dash.h,
            b(self.show_standings),
            b(self.show_relative),
            b(self.show_map),
            b(self.show_minimap),
            b(self.show_radar),
            b(self.show_dash),
            b(self.ingame_hud),
            self.standings_rows,
            self.relative_count,
            b(self.st_pos),
            b(self.st_num),
            b(self.st_name),
            b(self.st_gap),
            b(self.st_interval),
            b(self.st_laps),
            b(self.st_best),
            b(self.st_status),
            b(self.st_bike),
            b(self.st_penalty),
            b(self.st_crashed),
            join_st(&self.st_order),
            self.st_w_pos,
            self.st_w_num,
            self.st_w_name,
            self.st_w_gap,
            self.st_w_interval,
            self.st_w_laps,
            self.st_w_best,
            self.st_w_status,
            self.st_w_bike,
            self.st_w_penalty,
            self.st_w_crashed,
            self.st_bg,
            b(self.rel_num),
            b(self.rel_name),
            b(self.rel_gap),
            b(self.rel_pos),
            b(self.rel_bike),
            b(self.rel_penalty),
            b(self.rel_interval),
            b(self.rel_crashed),
            join_rel(&self.rel_order),
            self.rel_w_num,
            self.rel_w_name,
            self.rel_w_gap,
            self.rel_w_pos,
            self.rel_w_bike,
            self.rel_w_penalty,
            self.rel_w_interval,
            self.rel_w_crashed,
            self.rel_bg,
            b(self.map_others),
            b(self.map_sf),
            b(self.map_name),
            b(self.map_numbers),
            b(self.map_arrows),
            b(self.map_crown),
            b(self.map_place),
            self.map_dot.key(),
            self.map_bg,
            b(self.mini_others),
            b(self.mini_sf),
            b(self.mini_numbers),
            b(self.mini_arrows),
            b(self.mini_crown),
            b(self.mini_place),
            self.mini_dot.key(),
            self.mini_bg,
            b(self.radar_sides),
            b(self.radar_rear),
            self.radar_bg,
            self.dash_bg,
        );
        let _ = fs::write(&path, body);
        self.loaded_mtime = fs::metadata(&path).and_then(|m| m.modified()).ok();
    }

    pub fn apply_to_snapshot(&self, s: &mut Snapshot) {
        s.standings_rect = self.standings;
        s.relative = self.relative;
        s.map = self.map;
        s.show_standings = i32::from(self.show_standings);
        s.show_relative = i32::from(self.show_relative);
        s.show_map = i32::from(self.show_map);
        s.standings_rows = self.standings_rows;
        s.relative_count = self.relative_count;
    }

    pub fn move_st_to(&mut self, from: usize, to: usize) {
        move_to(&mut self.st_order, from, to);
    }

    pub fn move_rel_to(&mut self, from: usize, to: usize) {
        move_to(&mut self.rel_order, from, to);
    }

    pub fn standings_cols(&self) -> Vec<StField> {
        let mut cols: Vec<_> = self.st_order.iter().copied().filter(|c| c.enabled(self)).collect();
        if cols.is_empty() {
            cols.push(StField::Name);
        }
        cols
    }

    pub fn relative_cols(&self) -> Vec<RelField> {
        let mut cols: Vec<_> = self.rel_order.iter().copied().filter(|c| c.enabled(self)).collect();
        if cols.is_empty() {
            cols.push(RelField::Name);
        }
        cols
    }
}

fn b(v: bool) -> i32 {
    i32::from(v)
}

fn clamp_w(val: &str) -> i32 {
    val.parse().unwrap_or(40).clamp(18, 160)
}

fn clamp_pct(val: &str) -> i32 {
    val.parse().unwrap_or(80).clamp(0, 100)
}

fn parse_st_order(s: &str) -> Vec<StField> {
    normalize(s.split(',').filter_map(|p| StField::parse(p.trim())), &StField::ALL)
}

fn parse_rel_order(s: &str) -> Vec<RelField> {
    normalize(s.split(',').filter_map(|p| RelField::parse(p.trim())), &RelField::ALL)
}

fn normalize<T: Copy + PartialEq>(found: impl Iterator<Item = T>, all: &[T]) -> Vec<T> {
    let mut out = Vec::new();
    for item in found {
        if !out.contains(&item) {
            out.push(item);
        }
    }
    for item in all {
        if !out.contains(item) {
            out.push(*item);
        }
    }
    out
}

fn join_st(order: &[StField]) -> String {
    order.iter().map(|c| c.key()).collect::<Vec<_>>().join(",")
}

fn join_rel(order: &[RelField]) -> String {
    order.iter().map(|c| c.key()).collect::<Vec<_>>().join(",")
}

fn move_to<T>(items: &mut Vec<T>, from: usize, to: usize) {
    if from >= items.len() || to >= items.len() || from == to {
        return;
    }
    let item = items.remove(from);
    items.insert(to, item);
}

pub fn ini_path() -> PathBuf {
    let home = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\Public".into());
    PathBuf::from(home)
        .join("Documents")
        .join("PiBoSo")
        .join("MX Bikes")
        .join("mxbo.ini")
}

pub fn with_config<T>(f: impl FnOnce(&HudConfig) -> T) -> T {
    let g = CONFIG.lock().unwrap_or_else(|e| e.into_inner());
    f(&g)
}

pub fn update_config(f: impl FnOnce(&mut HudConfig)) {
    let mut g = CONFIG.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut g);
    g.save();
}
