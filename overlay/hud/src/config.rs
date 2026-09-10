use std::borrow::Cow;
use std::fs;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};
use std::time::SystemTime;

use crate::shm::{Rect, Snapshot};

pub static CONFIG: LazyLock<Mutex<HudConfig>> = LazyLock::new(|| Mutex::new(HudConfig::new()));

pub const COL_W_MIN: i32 = 18;
pub const COL_W_MAX: i32 = 160;
pub const NAME_W_MAX: i32 = 400;
pub const RADAR_RANGE_MIN: i32 = 6;
pub const RADAR_RANGE_MAX: i32 = 30;
pub const RADAR_RANGE_DEFAULT: i32 = 12;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FontFamily {
    Segoe,
    Arial,
    Tahoma,
    Roboto,
    Exo2,
    Teko,
    Goldman,
    Montserrat,
}

impl FontFamily {
    pub const ALL: [Self; 8] = [
        Self::Segoe,
        Self::Arial,
        Self::Tahoma,
        Self::Roboto,
        Self::Exo2,
        Self::Teko,
        Self::Goldman,
        Self::Montserrat,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Segoe => "Segoe UI",
            Self::Arial => "Arial",
            Self::Tahoma => "Tahoma",
            Self::Roboto => "Roboto",
            Self::Exo2 => "Exo 2",
            Self::Teko => "Teko",
            Self::Goldman => "Goldman",
            Self::Montserrat => "Montserrat",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Segoe => "segoe",
            Self::Arial => "arial",
            Self::Tahoma => "tahoma",
            Self::Roboto => "roboto",
            Self::Exo2 => "exo2",
            Self::Teko => "teko",
            Self::Goldman => "goldman",
            Self::Montserrat => "montserrat",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "arial" => Self::Arial,
            "tahoma" => Self::Tahoma,
            "roboto" => Self::Roboto,
            "exo2" | "exo" | "agency" | "agencyfb" | "ethnocentric" | "racesport" | "race" => {
                Self::Exo2
            }
            "teko" | "industry" | "oswald" => Self::Teko,
            "goldman" | "aero" | "aeromatics" | "bebas" | "bebasneue" | "faster" | "fasterone" => {
                Self::Goldman
            }
            "montserrat" | "impact" => Self::Montserrat,
            _ => Self::Segoe,
        }
    }

    pub fn windows_files(self) -> Option<(&'static str, &'static str)> {
        match self {
            Self::Segoe => Some((r"C:\Windows\Fonts\segoeui.ttf", r"C:\Windows\Fonts\segoeuib.ttf")),
            Self::Arial => Some((r"C:\Windows\Fonts\arial.ttf", r"C:\Windows\Fonts\arialbd.ttf")),
            Self::Tahoma => Some((r"C:\Windows\Fonts\tahoma.ttf", r"C:\Windows\Fonts\tahomabd.ttf")),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Units {
    Metric,
    Imperial,
}

impl Units {
    pub fn label(self) -> &'static str {
        match self {
            Self::Metric => "Metric",
            Self::Imperial => "Imperial",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Metric => "metric",
            Self::Imperial => "imperial",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "imperial" | "us" | "mph" => Self::Imperial,
            _ => Self::Metric,
        }
    }

    pub fn speed_label(self) -> &'static str {
        match self {
            Self::Metric => "KPH",
            Self::Imperial => "MPH",
        }
    }

    pub fn format_speed(self, mps: f32) -> String {
        let n = match self {
            Self::Metric => mps * 3.6,
            Self::Imperial => mps * 2.236936,
        };
        format!("{}", n.round().max(0.0) as i32)
    }

    pub fn format_temp(self, celsius: f32) -> String {
        if celsius <= 0.5 {
            return match self {
                Self::Metric => "--°C".into(),
                Self::Imperial => "--°F".into(),
            };
        }
        match self {
            Self::Metric => format!("{:.0}°C", celsius),
            Self::Imperial => format!("{:.0}°F", celsius * 9.0 / 5.0 + 32.0),
        }
    }

    /// Game fuel is liters. Imperial uses US gallons (same as MPH / °F).
    pub fn format_fuel(self, liters: f32, max_liters: f32) -> String {
        if liters <= 0.0 && max_liters <= 0.01 {
            return match self {
                Self::Metric => "-- L".into(),
                Self::Imperial => "-- gal".into(),
            };
        }
        match self {
            Self::Metric => format!("{:.1} L", liters.max(0.0)),
            Self::Imperial => format!("{:.1} gal", liters.max(0.0) * 0.264172),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UnitKind {
    Speed,
    Liquids,
    Temperature,
}

impl UnitKind {
    pub const COUNT: usize = 3;
    pub const ALL: [Self; Self::COUNT] = [Self::Speed, Self::Liquids, Self::Temperature];

    pub fn label(self) -> &'static str {
        match self {
            Self::Speed => "Speed",
            Self::Liquids => "Liquids",
            Self::Temperature => "Temperature",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Speed => "units_speed",
            Self::Liquids => "units_liquids",
            Self::Temperature => "units_temperature",
        }
    }

    pub fn idx(self) -> usize {
        match self {
            Self::Speed => 0,
            Self::Liquids => 1,
            Self::Temperature => 2,
        }
    }

    pub fn parse_key(key: &str) -> Option<Self> {
        match key {
            "units_speed" => Some(Self::Speed),
            "units_liquids" | "units_liquid" | "units_fuel" => Some(Self::Liquids),
            "units_temperature" | "units_temp" => Some(Self::Temperature),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct UnitPrefs {
    pub speed: Units,
    pub liquids: Units,
    pub temperature: Units,
}

impl UnitPrefs {
    pub fn all(units: Units) -> Self {
        Self {
            speed: units,
            liquids: units,
            temperature: units,
        }
    }

    pub fn get(self, kind: UnitKind) -> Units {
        match kind {
            UnitKind::Speed => self.speed,
            UnitKind::Liquids => self.liquids,
            UnitKind::Temperature => self.temperature,
        }
    }

    pub fn set(&mut self, kind: UnitKind, units: Units) {
        match kind {
            UnitKind::Speed => self.speed = units,
            UnitKind::Liquids => self.liquids = units,
            UnitKind::Temperature => self.temperature = units,
        }
    }

    pub fn speed_label(self) -> &'static str {
        self.speed.speed_label()
    }

    pub fn format_speed(self, mps: f32) -> String {
        self.speed.format_speed(mps)
    }

    pub fn format_temp(self, celsius: f32) -> String {
        self.temperature.format_temp(celsius)
    }

    pub fn format_fuel(self, liters: f32, max_liters: f32) -> String {
        self.liquids.format_fuel(liters, max_liters)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TableText {
    White,
    Black,
}

impl TableText {
    pub fn label(self) -> &'static str {
        match self {
            Self::White => "White",
            Self::Black => "Black",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::White => "white",
            Self::Black => "black",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "black" | "dark" => Self::Black,
            _ => Self::White,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SettingsKey {
    F6,
    F7,
    F8,
    F10,
    F11,
    Insert,
    Home,
    End,
}

impl SettingsKey {
    pub const ALL: [Self; 8] = [
        Self::F6,
        Self::F7,
        Self::F8,
        Self::F10,
        Self::F11,
        Self::Insert,
        Self::Home,
        Self::End,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::F6 => "F6",
            Self::F7 => "F7",
            Self::F8 => "F8",
            Self::F10 => "F10",
            Self::F11 => "F11",
            Self::Insert => "Insert",
            Self::Home => "Home",
            Self::End => "End",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::F6 => "f6",
            Self::F7 => "f7",
            Self::F8 => "f8",
            Self::F10 => "f10",
            Self::F11 => "f11",
            Self::Insert => "insert",
            Self::Home => "home",
            Self::End => "end",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "f6" => Self::F6,
            "f7" => Self::F7,
            "f10" => Self::F10,
            "f11" => Self::F11,
            "insert" | "ins" => Self::Insert,
            "home" => Self::Home,
            "end" => Self::End,
            _ => Self::F8,
        }
    }

    pub fn vk(self) -> i32 {
        match self {
            Self::F6 => 0x75,
            Self::F7 => 0x76,
            Self::F8 => 0x77,
            Self::F10 => 0x79,
            Self::F11 => 0x7A,
            Self::Insert => 0x2D,
            Self::Home => 0x24,
            Self::End => 0x23,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StanceBind {
    PadRb,
    PadLb,
    PadRt,
    PadLt,
    PadA,
    PadB,
    PadX,
    PadY,
    PadDpadUp,
    PadDpadDown,
    PadDpadLeft,
    PadDpadRight,
    MouseLeft,
    MouseRight,
    MouseMiddle,
    MouseX1,
    MouseX2,
    Key(u16),
}

impl StanceBind {
    pub const ALL: [Self; 12] = [
        Self::PadRb,
        Self::PadLb,
        Self::PadRt,
        Self::PadLt,
        Self::PadA,
        Self::PadB,
        Self::PadX,
        Self::PadY,
        Self::PadDpadUp,
        Self::PadDpadDown,
        Self::PadDpadLeft,
        Self::PadDpadRight,
    ];

    pub const MOUSE: [Self; 5] = [
        Self::MouseLeft,
        Self::MouseRight,
        Self::MouseMiddle,
        Self::MouseX1,
        Self::MouseX2,
    ];

    pub fn label(self) -> Cow<'static, str> {
        match self {
            Self::PadRb => Cow::Borrowed("Right bumper"),
            Self::PadLb => Cow::Borrowed("Left bumper"),
            Self::PadRt => Cow::Borrowed("R2 / RT"),
            Self::PadLt => Cow::Borrowed("L2 / LT"),
            Self::PadA => Cow::Borrowed("A / Cross"),
            Self::PadB => Cow::Borrowed("B / Circle"),
            Self::PadX => Cow::Borrowed("X / Square"),
            Self::PadY => Cow::Borrowed("Y / Triangle"),
            Self::PadDpadUp => Cow::Borrowed("D-pad up"),
            Self::PadDpadDown => Cow::Borrowed("D-pad down"),
            Self::PadDpadLeft => Cow::Borrowed("D-pad left"),
            Self::PadDpadRight => Cow::Borrowed("D-pad right"),
            Self::MouseLeft => Cow::Borrowed("Mouse left"),
            Self::MouseRight => Cow::Borrowed("Mouse right"),
            Self::MouseMiddle => Cow::Borrowed("Mouse middle"),
            Self::MouseX1 => Cow::Borrowed("Mouse 4"),
            Self::MouseX2 => Cow::Borrowed("Mouse 5"),
            Self::Key(vk) => Cow::Owned(vk_label(vk)),
        }
    }

    pub fn key(self) -> String {
        match self {
            Self::PadRb => "rb".into(),
            Self::PadLb => "lb".into(),
            Self::PadRt => "rt".into(),
            Self::PadLt => "lt".into(),
            Self::PadA => "a".into(),
            Self::PadB => "b".into(),
            Self::PadX => "x".into(),
            Self::PadY => "y".into(),
            Self::PadDpadUp => "dup".into(),
            Self::PadDpadDown => "ddown".into(),
            Self::PadDpadLeft => "dleft".into(),
            Self::PadDpadRight => "dright".into(),
            Self::MouseLeft => "m1".into(),
            Self::MouseRight => "m2".into(),
            Self::MouseMiddle => "m3".into(),
            Self::MouseX1 => "m4".into(),
            Self::MouseX2 => "m5".into(),
            Self::Key(vk) => format!("k{vk}"),
        }
    }

    pub fn parse(s: &str) -> Self {
        let s = s.trim().to_ascii_lowercase();
        match s.as_str() {
            "lb" | "l1" => Self::PadLb,
            "rt" | "r2" => Self::PadRt,
            "lt" | "l2" => Self::PadLt,
            "a" | "cross" => Self::PadA,
            "b" | "circle" => Self::PadB,
            "x" | "square" => Self::PadX,
            "y" | "triangle" => Self::PadY,
            "dup" | "dpad_up" | "up" => Self::PadDpadUp,
            "ddown" | "dpad_down" | "down" => Self::PadDpadDown,
            "dleft" | "dpad_left" => Self::PadDpadLeft,
            "dright" | "dpad_right" => Self::PadDpadRight,
            "m1" | "mouse_left" | "lmb" => Self::MouseLeft,
            "m2" | "mouse_right" | "rmb" => Self::MouseRight,
            "m3" | "mouse_middle" | "mmb" => Self::MouseMiddle,
            "m4" | "mouse_x1" => Self::MouseX1,
            "m5" | "mouse_x2" => Self::MouseX2,
            "space" => Self::Key(0x20),
            "enter" | "return" => Self::Key(0x0D),
            "tab" => Self::Key(0x09),
            "lshift" => Self::Key(0xA0),
            "rshift" => Self::Key(0xA1),
            "lctrl" => Self::Key(0xA2),
            "rctrl" => Self::Key(0xA3),
            other => parse_key_bind(other).unwrap_or(Self::PadRb),
        }
    }
}

fn parse_key_bind(s: &str) -> Option<StanceBind> {
    let n = s.strip_prefix("key").or_else(|| s.strip_prefix('k'))?;
    let vk: u16 = n.parse().ok()?;
    (vk >= 8 && vk != 0x1B).then_some(StanceBind::Key(vk))
}

fn vk_label(vk: u16) -> String {
    match vk {
        0x08 => "Backspace".into(),
        0x09 => "Tab".into(),
        0x0D => "Enter".into(),
        0x13 => "Pause".into(),
        0x14 => "Caps Lock".into(),
        0x20 => "Space".into(),
        0x21 => "Page Up".into(),
        0x22 => "Page Down".into(),
        0x23 => "End".into(),
        0x24 => "Home".into(),
        0x25 => "Left Arrow".into(),
        0x26 => "Up Arrow".into(),
        0x27 => "Right Arrow".into(),
        0x28 => "Down Arrow".into(),
        0x2D => "Insert".into(),
        0x2E => "Delete".into(),
        0x30..=0x39 => ((b'0' + (vk - 0x30) as u8) as char).to_string(),
        0x41..=0x5A => ((b'A' + (vk - 0x41) as u8) as char).to_string(),
        0x60..=0x69 => format!("Numpad {}", vk - 0x60),
        0x6A => "Numpad *".into(),
        0x6B => "Numpad +".into(),
        0x6D => "Numpad -".into(),
        0x6E => "Numpad .".into(),
        0x6F => "Numpad /".into(),
        0x70..=0x87 => format!("F{}", vk - 0x6F),
        0x90 => "Num Lock".into(),
        0x91 => "Scroll Lock".into(),
        0xA0 => "Left Shift".into(),
        0xA1 => "Right Shift".into(),
        0xA2 => "Left Ctrl".into(),
        0xA3 => "Right Ctrl".into(),
        0xA4 => "Left Alt".into(),
        0xA5 => "Right Alt".into(),
        0xBA => ";".into(),
        0xBB => "=".into(),
        0xBC => ",".into(),
        0xBD => "-".into(),
        0xBE => ".".into(),
        0xBF => "/".into(),
        0xC0 => "`".into(),
        0xDB => "[".into(),
        0xDC => "\\".into(),
        0xDD => "]".into(),
        0xDE => "'".into(),
        _ => format!("Key {vk}"),
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StanceMode {
    Toggle,
    Hold,
}

impl StanceMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Toggle => "Toggle",
            Self::Hold => "Hold to sit",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Toggle => "toggle",
            Self::Hold => "hold",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "hold" | "hold_sit" => Self::Hold,
            _ => Self::Toggle,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StanceStyle {
    Text,
    Icon,
}

impl StanceStyle {
    pub fn label(self) -> &'static str {
        match self {
            Self::Text => "Text",
            Self::Icon => "Icon",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Icon => "icon",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "icon" => Self::Icon,
            _ => Self::Text,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GamepadStyle {
    Auto,
    PlayStation,
    Xbox,
}

impl GamepadStyle {
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::PlayStation => "PlayStation",
            Self::Xbox => "Xbox",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::PlayStation => "playstation",
            Self::Xbox => "xbox",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "playstation" | "sony" | "ps" | "dualshock" | "dualsense" => Self::PlayStation,
            "xbox" | "xinput" => Self::Xbox,
            _ => Self::Auto,
        }
    }

    pub fn sony_art(self, pad: crate::gamepad::PadKind) -> bool {
        match self {
            Self::Auto => pad == crate::gamepad::PadKind::Sony,
            Self::PlayStation => true,
            Self::Xbox => false,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LeanStyle {
    Figure,
    Minimal,
}

impl LeanStyle {
    pub fn label(self) -> &'static str {
        match self {
            Self::Figure => "Figure",
            Self::Minimal => "Minimal",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Figure => "figure",
            Self::Minimal => "minimal",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "minimal" | "attitude" => Self::Minimal,
            _ => Self::Figure,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WidgetId {
    Standings,
    Relative,
    Map,
    Minimap,
    Radar,
    Dash,
    Ticker,
    Sys,
    Sector,
    Delta,
    Stance,
    Flag,
    Lean,
    Gamepad,
    Telemetry,
}

impl WidgetId {
    pub const ALL: [Self; 15] = [
        Self::Standings,
        Self::Relative,
        Self::Map,
        Self::Minimap,
        Self::Radar,
        Self::Dash,
        Self::Ticker,
        Self::Sys,
        Self::Sector,
        Self::Delta,
        Self::Stance,
        Self::Flag,
        Self::Lean,
        Self::Gamepad,
        Self::Telemetry,
    ];
    pub const COUNT: usize = 15;

    pub fn idx(self) -> usize {
        self as usize
    }

    fn default_prefs(self) -> WidgetPrefs {
        WidgetPrefs {
            rect: self.default_rect(),
            show: false,
            font: 100,
            bold: false,
            bg: self.default_bg(),
        }
    }

    fn default_rect(self) -> Rect {
        match self {
            Self::Standings => Rect { x: 0.012, y: 0.03, w: 0.20, h: 0.46 },
            Self::Relative => Rect { x: 0.012, y: 0.62, w: 0.20, h: 0.36 },
            Self::Map => Rect { x: 0.775, y: 0.62, w: 0.21, h: 0.34 },
            Self::Minimap => Rect { x: 0.815, y: 0.035, w: 0.165, h: 0.295 },
            Self::Radar => Rect { x: 0.438, y: 0.755, w: 0.124, h: 0.22 },
            Self::Dash => Rect { x: 0.445, y: 0.865, w: 0.111, h: 0.115 },
            Self::Ticker => Rect { x: 0.06, y: 0.012, w: 0.88, h: 0.055 },
            Self::Sys => Rect { x: 0.012, y: 0.36, w: 0.086, h: 0.25 },
            Self::Sector => Rect { x: 0.62, y: 0.68, w: 0.36, h: 0.26 },
            Self::Delta => Rect { x: 0.36, y: 0.76, w: 0.28, h: 0.09 },
            Self::Stance => Rect { x: 0.445, y: 0.705, w: 0.11, h: 0.065 },
            Self::Flag => Rect { x: 0.447, y: 0.026, w: 0.107, h: 0.019 },
            Self::Lean => Rect { x: 0.318, y: 0.755, w: 0.11, h: 0.20 },
            Self::Gamepad => Rect { x: 0.38, y: 0.76, w: 0.24, h: 0.20 },
            Self::Telemetry => Rect { x: 0.22, y: 0.80, w: 0.56, h: 0.145 },
        }
    }

    fn default_bg(self) -> i32 {
        match self {
            Self::Standings | Self::Relative => 78,
            Self::Radar | Self::Ticker | Self::Stance | Self::Lean => 86,
            Self::Gamepad | Self::Map | Self::Minimap | Self::Delta => 0,
            Self::Dash | Self::Sys | Self::Sector | Self::Telemetry => 82,
            Self::Flag => 100,
        }
    }

    /// Disk key names. Changing these breaks existing `Holeshot-HUD.ini` files.
    fn ini(self) -> WidgetIni {
        match self {
            Self::Standings => WidgetIni { rect: "standings", show: "show_standings", font: "st_font", bold: "st_bold", bg: "st_bg" },
            Self::Relative => WidgetIni { rect: "relative", show: "show_relative", font: "rel_font", bold: "rel_bold", bg: "rel_bg" },
            Self::Map => WidgetIni { rect: "map", show: "show_map", font: "map_font", bold: "map_bold", bg: "map_bg" },
            Self::Minimap => WidgetIni { rect: "minimap", show: "show_minimap", font: "mini_font", bold: "mini_bold", bg: "mini_bg" },
            Self::Radar => WidgetIni { rect: "radar", show: "show_radar", font: "radar_font", bold: "radar_bold", bg: "radar_bg" },
            Self::Dash => WidgetIni { rect: "dash", show: "show_dash", font: "dash_font", bold: "dash_bold", bg: "dash_bg" },
            Self::Ticker => WidgetIni { rect: "ticker", show: "show_ticker", font: "ticker_font", bold: "ticker_bold", bg: "ticker_bg" },
            Self::Sys => WidgetIni { rect: "sys", show: "show_sys", font: "sys_font", bold: "sys_bold", bg: "sys_bg" },
            Self::Sector => WidgetIni { rect: "sector", show: "show_sector", font: "sector_font", bold: "sector_bold", bg: "sector_bg" },
            Self::Delta => WidgetIni { rect: "delta", show: "show_delta", font: "delta_font", bold: "delta_bold", bg: "delta_bg" },
            Self::Stance => WidgetIni { rect: "stance", show: "show_stance", font: "stance_font", bold: "stance_bold", bg: "stance_bg" },
            Self::Flag => WidgetIni { rect: "flag", show: "show_flag", font: "flag_font", bold: "flag_bold", bg: "flag_bg" },
            Self::Lean => WidgetIni { rect: "lean", show: "show_lean", font: "lean_font", bold: "lean_bold", bg: "lean_bg" },
            Self::Gamepad => WidgetIni { rect: "gamepad", show: "show_gamepad", font: "gamepad_font", bold: "gamepad_bold", bg: "gamepad_bg" },
            Self::Telemetry => WidgetIni { rect: "telemetry", show: "show_telemetry", font: "telemetry_font", bold: "telemetry_bold", bg: "telemetry_bg" },
        }
    }
}

#[derive(Clone, Copy)]
struct WidgetIni {
    rect: &'static str,
    show: &'static str,
    font: &'static str,
    bold: &'static str,
    bg: &'static str,
}

/// Shared per-widget style: rect, visibility, font size, bold, background.
/// Standings columns, dash layout, flag yellow/blue, etc. stay on `HudLayout`.
#[derive(Clone, Copy, Debug)]
pub struct WidgetPrefs {
    pub rect: Rect,
    pub show: bool,
    pub font: i32,
    pub bold: bool,
    pub bg: i32,
}

const _: () = assert!(WidgetId::ALL.len() == WidgetId::COUNT);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SessionPreset {
    Practice,
    Warmup,
    Race,
    Spectate,
}

impl SessionPreset {
    pub const ALL: [Self; 4] = [Self::Practice, Self::Warmup, Self::Race, Self::Spectate];
    pub const COUNT: usize = 4;

    pub fn idx(self) -> usize {
        self as usize
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Practice => "Practice",
            Self::Warmup => "Warmup",
            Self::Race => "Race",
            Self::Spectate => "Spectate",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Practice => "practice",
            Self::Warmup => "warmup",
            Self::Race => "race",
            Self::Spectate => "spectate",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "practice" => Self::Practice,
            "warmup" => Self::Warmup,
            "spectate" | "spectating" => Self::Spectate,
            _ => Self::Race,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SnapAlign {
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
    HCenter,
    VCenter,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StField {
    Pos,
    Num,
    Name,
    Gap,
    Interval,
    Laps,
    Current,
    Best,
    Last,
    Status,
    Bike,
    Penalty,
    Crashed,
}

impl StField {
    pub const ALL: [Self; 13] = [
        Self::Pos,
        Self::Num,
        Self::Name,
        Self::Gap,
        Self::Interval,
        Self::Laps,
        Self::Current,
        Self::Best,
        Self::Last,
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
            Self::Current => "current",
            Self::Best => "best",
            Self::Last => "last",
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
            Self::Laps => "Completed Laps",
            Self::Current => "Current lap",
            Self::Best => "Fastest",
            Self::Last => "Last lap",
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
            "cur" | "current" => Self::Current,
            "best" => Self::Best,
            "last" => Self::Last,
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
            Self::Current => c.st_current,
            Self::Best => c.st_best,
            Self::Last => c.st_last,
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
            Self::Current => c.st_w_current,
            Self::Best => c.st_w_best,
            Self::Last => c.st_w_last,
            Self::Status => c.st_w_status,
            Self::Bike => c.st_w_bike,
            Self::Penalty => c.st_w_penalty,
            Self::Crashed => c.st_w_crashed,
        }
    }

    pub fn set_width(self, c: &mut HudConfig, w: i32) {
        self.add_width(c, w - self.width(c));
    }

    pub fn width_max(self) -> i32 {
        match self {
            Self::Name => NAME_W_MAX,
            _ => COL_W_MAX,
        }
    }

    pub fn add_width(self, c: &mut HudConfig, d: i32) {
        let next = (self.width(c) + d).clamp(COL_W_MIN, self.width_max());
        match self {
            Self::Pos => c.st_w_pos = next,
            Self::Num => c.st_w_num = next,
            Self::Name => c.st_w_name = next,
            Self::Gap => c.st_w_gap = next,
            Self::Interval => c.st_w_interval = next,
            Self::Laps => c.st_w_laps = next,
            Self::Current => c.st_w_current = next,
            Self::Best => c.st_w_best = next,
            Self::Last => c.st_w_last = next,
            Self::Status => c.st_w_status = next,
            Self::Bike => c.st_w_bike = next,
            Self::Penalty => c.st_w_penalty = next,
            Self::Crashed => c.st_w_crashed = next,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RelField {
    Num,
    Name,
    Gap,
    Laps,
    Current,
    Pos,
    Bike,
    Penalty,
    Interval,
    Crashed,
    Best,
    Last,
}

impl RelField {
    pub const ALL: [Self; 12] = [
        Self::Num,
        Self::Name,
        Self::Gap,
        Self::Laps,
        Self::Current,
        Self::Pos,
        Self::Bike,
        Self::Penalty,
        Self::Interval,
        Self::Crashed,
        Self::Best,
        Self::Last,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Self::Num => "num",
            Self::Name => "name",
            Self::Gap => "gap",
            Self::Laps => "laps",
            Self::Current => "current",
            Self::Pos => "pos",
            Self::Bike => "bike",
            Self::Penalty => "pen",
            Self::Interval => "int",
            Self::Crashed => "crash",
            Self::Best => "best",
            Self::Last => "last",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Num => "Number",
            Self::Name => "Name",
            Self::Gap => "Gap",
            Self::Laps => "Completed Laps",
            Self::Current => "Current lap",
            Self::Pos => "Position",
            Self::Bike => "Bike",
            Self::Penalty => "Penalty",
            Self::Interval => "Interval",
            Self::Crashed => "Crashed",
            Self::Best => "Fastest",
            Self::Last => "Last lap",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "num" => Self::Num,
            "name" => Self::Name,
            "gap" => Self::Gap,
            "laps" => Self::Laps,
            "cur" | "current" => Self::Current,
            "pos" => Self::Pos,
            "bike" => Self::Bike,
            "pen" | "penalty" => Self::Penalty,
            "int" | "interval" => Self::Interval,
            "crash" | "crashed" => Self::Crashed,
            "best" => Self::Best,
            "last" => Self::Last,
            _ => return None,
        })
    }

    pub fn enabled(self, c: &HudConfig) -> bool {
        match self {
            Self::Num => c.rel_num,
            Self::Name => c.rel_name,
            Self::Gap => c.rel_gap,
            Self::Laps => c.rel_laps,
            Self::Current => c.rel_current,
            Self::Pos => c.rel_pos,
            Self::Bike => c.rel_bike,
            Self::Penalty => c.rel_penalty,
            Self::Interval => c.rel_interval,
            Self::Crashed => c.rel_crashed,
            Self::Best => c.rel_best,
            Self::Last => c.rel_last,
        }
    }

    pub fn width(self, c: &HudConfig) -> i32 {
        match self {
            Self::Num => c.rel_w_num,
            Self::Name => c.rel_w_name,
            Self::Gap => c.rel_w_gap,
            Self::Laps => c.rel_w_laps,
            Self::Current => c.rel_w_current,
            Self::Pos => c.rel_w_pos,
            Self::Bike => c.rel_w_bike,
            Self::Penalty => c.rel_w_penalty,
            Self::Interval => c.rel_w_interval,
            Self::Crashed => c.rel_w_crashed,
            Self::Best => c.rel_w_best,
            Self::Last => c.rel_w_last,
        }
    }

    pub fn set_width(self, c: &mut HudConfig, w: i32) {
        self.add_width(c, w - self.width(c));
    }

    pub fn width_max(self) -> i32 {
        match self {
            Self::Name => NAME_W_MAX,
            _ => COL_W_MAX,
        }
    }

    pub fn add_width(self, c: &mut HudConfig, d: i32) {
        let next = (self.width(c) + d).clamp(COL_W_MIN, self.width_max());
        match self {
            Self::Num => c.rel_w_num = next,
            Self::Name => c.rel_w_name = next,
            Self::Gap => c.rel_w_gap = next,
            Self::Laps => c.rel_w_laps = next,
            Self::Current => c.rel_w_current = next,
            Self::Pos => c.rel_w_pos = next,
            Self::Bike => c.rel_w_bike = next,
            Self::Penalty => c.rel_w_penalty = next,
            Self::Interval => c.rel_w_interval = next,
            Self::Crashed => c.rel_w_crashed = next,
            Self::Best => c.rel_w_best = next,
            Self::Last => c.rel_w_last = next,
        }
    }
}

/// Overlay Systems widget: how many process rows to paint.
pub const SYS_PROC_MAX: usize = 8;
/// Settings list cap (shown or hidden).
pub const SYS_APP_MAX: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SysAppKind {
    Hud,
    Mxbikes,
    MxbApp,
    Reshade,
    Exe,
}

#[derive(Clone, Copy, Debug)]
pub struct SysPreset {
    pub key: &'static str,
    pub label: &'static str,
    pub names: &'static [&'static str],
    pub kind: SysAppKind,
}

impl SysPreset {
    pub fn builtin(self) -> bool {
        matches!(self.key, "hud" | "mxbikes" | "mxbapp" | "reshade" | "obs")
    }

    pub fn addable(self) -> bool {
        matches!(self.kind, SysAppKind::Exe) && !self.builtin()
    }
}

pub const SYS_PRESETS: &[SysPreset] = &[
    SysPreset { key: "hud", label: "HUD", names: &[], kind: SysAppKind::Hud },
    SysPreset { key: "mxbikes", label: "MX Bikes", names: &["mxbikes.exe"], kind: SysAppKind::Mxbikes },
    SysPreset {
        key: "mxbapp",
        label: "MXB App",
        names: &["frost.exe", "mxb app.exe", "mxb-app.exe", "mxbapp.exe", "frostmod.exe"],
        kind: SysAppKind::MxbApp,
    },
    SysPreset { key: "reshade", label: "ReShade", names: &[], kind: SysAppKind::Reshade },
    SysPreset { key: "obs", label: "OBS", names: &["obs64.exe", "obs32.exe"], kind: SysAppKind::Exe },
    SysPreset { key: "discord", label: "Discord", names: &["discord.exe"], kind: SysAppKind::Exe },
    SysPreset { key: "steam", label: "Steam", names: &["steam.exe"], kind: SysAppKind::Exe },
    SysPreset {
        key: "nvidia",
        label: "NVIDIA",
        names: &["nvidia overlay.exe", "nvidia app.exe", "nvidia share.exe"],
        kind: SysAppKind::Exe,
    },
    SysPreset { key: "afterburner", label: "Afterburner", names: &["msiafterburner.exe"], kind: SysAppKind::Exe },
    SysPreset { key: "rtss", label: "RTSS", names: &["rtss.exe"], kind: SysAppKind::Exe },
    SysPreset { key: "medal", label: "Medal", names: &["medal.exe"], kind: SysAppKind::Exe },
    SysPreset { key: "spotify", label: "Spotify", names: &["spotify.exe"], kind: SysAppKind::Exe },
    SysPreset { key: "gamebar", label: "Game Bar", names: &["gamebar.exe"], kind: SysAppKind::Exe },
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SysApp {
    pub key: String,
    pub label: String,
    pub names: Vec<String>,
    pub kind: SysAppKind,
    pub show: bool,
}

impl SysApp {
    pub fn from_preset(p: &SysPreset, show: bool) -> Self {
        Self {
            key: p.key.into(),
            label: p.label.into(),
            names: p.names.iter().map(|n| (*n).to_string()).collect(),
            kind: p.kind,
            show,
        }
    }

    pub fn removable(&self) -> bool {
        !SYS_PRESETS.iter().any(|p| p.key == self.key && p.builtin())
    }
}

pub fn default_sys_apps() -> Vec<SysApp> {
    SYS_PRESETS
        .iter()
        .filter(|p| p.builtin())
        .map(|p| SysApp::from_preset(p, true))
        .collect()
}

fn sanitize_sys_token(s: &str) -> String {
    s.chars()
        .filter(|c| *c != ',' && *c != '|' && *c != '\n' && *c != '\r')
        .take(24)
        .collect::<String>()
        .trim()
        .to_string()
}

pub fn encode_sys_apps(apps: &[SysApp]) -> String {
    apps.iter()
        .map(|a| {
            if a.key.starts_with("exe:") {
                let name = a.names.first().map(|s| s.as_str()).unwrap_or("app.exe");
                format!(
                    "exe|{}|{}|{}",
                    sanitize_sys_token(&a.label),
                    sanitize_sys_token(name),
                    if a.show { 1 } else { 0 }
                )
            } else {
                format!("{}:{}", a.key, if a.show { 1 } else { 0 })
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

pub fn parse_sys_apps(val: &str) -> Vec<SysApp> {
    let mut apps = Vec::new();
    for token in val.split(',') {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        if let Some(app) = parse_sys_app_token(token) {
            if apps.iter().any(|a: &SysApp| a.key == app.key) {
                continue;
            }
            apps.push(app);
        }
    }
    ensure_sys_builtins(&mut apps);
    apps
}

fn parse_sys_app_token(token: &str) -> Option<SysApp> {
    if let Some(rest) = token.strip_prefix("exe|") {
        let mut parts = rest.split('|');
        let label = sanitize_sys_token(parts.next().unwrap_or(""));
        let name = parts.next().unwrap_or("").trim().to_ascii_lowercase();
        let show = parts.next().is_some_and(|s| s == "1");
        if label.is_empty() || !name.ends_with(".exe") || name.contains('\\') || name.contains('/') {
            return None;
        }
        if let Some(p) = SYS_PRESETS.iter().find(|p| p.names.iter().any(|n| *n == name)) {
            return Some(SysApp::from_preset(p, show));
        }
        return Some(SysApp {
            key: format!("exe:{name}"),
            label,
            names: vec![name],
            kind: SysAppKind::Exe,
            show,
        });
    }
    let (key, show) = token.split_once(':')?;
    let key = key.trim();
    let show = show.trim() == "1";
    SYS_PRESETS
        .iter()
        .find(|p| p.key == key)
        .map(|p| SysApp::from_preset(p, show))
}

fn ensure_sys_builtins(apps: &mut Vec<SysApp>) {
    for (i, p) in SYS_PRESETS.iter().filter(|p| p.builtin()).enumerate() {
        if !apps.iter().any(|a| a.key == p.key) {
            apps.insert(i.min(apps.len()), SysApp::from_preset(p, true));
        }
    }
    apps.truncate(SYS_APP_MAX);
}

#[derive(Clone)]
pub struct HudLayout {
    widgets: [WidgetPrefs; WidgetId::COUNT],
    /// Tick the current sector live. Off: times only after the split.
    pub sector_live: bool,
    /// Compare Sectors to this session's best splits instead of the saved tape.
    pub sector_session: bool,
    /// Draw LAST / -2 / … under the live strip.
    pub sector_hist: bool,
    /// How many completed laps the underboard shows (1–5).
    pub sector_hist_laps: i32,
    /// Compare Delta Bar to this session's fastest decent lap instead of the saved tape.
    pub delta_session: bool,
    /// Nearby crash. Flags widget only. Off by default. Dash has `dash_yellow`.
    pub flag_yellow: bool,
    /// Someone a lap up closing from behind. Flags widget only. Off by default. Dash has `dash_blue`.
    pub flag_blue: bool,
    /// Someone you are coming to lap. Flags widget only. Off by default. Dash has `dash_red`.
    pub flag_red: bool,
    /// Caption on the Flags cloth (WHITE FLAG / …). On by default.
    pub flag_text: bool,
    pub standings_rows: i32,
    pub relative_count: i32,
    pub ticker_count: i32,
    pub ticker_title: bool,
    pub ticker_autoscroll: bool,
    pub st_pos: bool,
    pub st_num: bool,
    pub st_name: bool,
    pub st_gap: bool,
    pub st_interval: bool,
    pub st_laps: bool,
    pub st_current: bool,
    pub st_best: bool,
    pub st_last: bool,
    pub st_status: bool,
    pub st_bike: bool,
    pub st_penalty: bool,
    pub st_crashed: bool,
    pub rel_num: bool,
    pub rel_name: bool,
    pub rel_gap: bool,
    pub rel_laps: bool,
    pub rel_current: bool,
    pub rel_pos: bool,
    pub rel_bike: bool,
    pub rel_penalty: bool,
    pub rel_interval: bool,
    pub rel_crashed: bool,
    pub rel_best: bool,
    pub rel_last: bool,
    pub map_others: bool,
    pub map_sf: bool,
    pub map_sectors: bool,
    pub map_name: bool,
    pub map_numbers: bool,
    pub map_arrows: bool,
    pub map_crown: bool,
    pub map_place: bool,
    pub map_dot: DotLabel,
    pub mini_others: bool,
    pub mini_sf: bool,
    pub mini_sectors: bool,
    pub mini_numbers: bool,
    pub mini_arrows: bool,
    pub mini_crown: bool,
    pub mini_place: bool,
    pub mini_dot: DotLabel,
    pub radar_sides: bool,
    pub radar_rear: bool,
    pub radar_rings: bool,
    /// How far beside and behind, in meters, the radar shows other riders.
    /// Rings stay at 3 / 6 / 12 m; a longer range places dots outside the 12 m ring.
    pub radar_range: i32,
    pub st_hl: i32,
    pub st_text: TableText,
    pub st_stripe: bool,
    pub rel_hl: i32,
    pub rel_text: TableText,
    pub rel_stripe: bool,
    pub mini_zoom: i32,
    pub dash_rev: bool,
    /// Nearby crash wrap on Dash. Off by default.
    pub dash_yellow: bool,
    /// Someone a lap up closing from behind, wrap on Dash. Off by default.
    pub dash_blue: bool,
    /// Someone you are coming to lap, wrap on Dash. Off by default.
    pub dash_red: bool,
    /// Gear + speed lockup; hides RPM, place, footer, and the rev bar.
    pub dash_simple: bool,
    /// Flush Dash gear faint red at the shift light or limiter. Off by default.
    pub dash_shift_color: bool,
    pub dash_left: DashField,
    pub dash_mid: DashField,
    pub dash_right: DashField,
    pub ticker_left: BoardField,
    pub ticker_right: BoardField,
    pub st_head: [BoardField; 3],
    pub st_foot: [BoardField; 3],
    pub rel_head: [BoardField; 3],
    pub rel_foot: [BoardField; 3],
    pub stance_mode: StanceMode,
    pub stance_style: StanceStyle,
    pub stance_show_sit: bool,
    pub lean_style: LeanStyle,
    pub gamepad_style: GamepadStyle,
    /// Throttle / brake history well.
    pub telemetry_traces: bool,
    pub telemetry_trace_throttle: bool,
    pub telemetry_trace_brake: bool,
    pub telemetry_trace_steer: bool,
    /// Clutch / brake / throttle analog bars.
    pub telemetry_bars: bool,
    pub telemetry_bar_clutch: bool,
    pub telemetry_bar_brake: bool,
    pub telemetry_bar_throttle: bool,
    pub telemetry_bar_steer: bool,
    /// Gear, speed, and RPM dial.
    pub telemetry_dial: bool,
    /// Process rows on Systems. Built-ins stay; extras can be added and removed.
    pub sys_apps: Vec<SysApp>,
    pub st_order: Vec<StField>,
    pub rel_order: Vec<RelField>,
    pub st_w_pos: i32,
    pub st_w_num: i32,
    pub st_w_name: i32,
    pub st_w_gap: i32,
    pub st_w_interval: i32,
    pub st_w_laps: i32,
    pub st_w_current: i32,
    pub st_w_best: i32,
    pub st_w_last: i32,
    pub st_w_status: i32,
    pub st_w_bike: i32,
    pub st_w_penalty: i32,
    pub st_w_crashed: i32,
    pub rel_w_num: i32,
    pub rel_w_name: i32,
    pub rel_w_gap: i32,
    pub rel_w_laps: i32,
    pub rel_w_current: i32,
    pub rel_w_pos: i32,
    pub rel_w_bike: i32,
    pub rel_w_penalty: i32,
    pub rel_w_interval: i32,
    pub rel_w_crashed: i32,
    pub rel_w_best: i32,
    pub rel_w_last: i32,
}

impl HudLayout {
    pub fn new() -> Self {
        Self {
            widgets: std::array::from_fn(|i| WidgetId::ALL[i].default_prefs()),
            sector_live: true,
            sector_session: false,
            sector_hist: true,
            sector_hist_laps: 3,
            delta_session: false,
            flag_yellow: false,
            flag_blue: false,
            flag_red: false,
            flag_text: true,
            standings_rows: 12,
            relative_count: 3,
            ticker_count: 7,
            ticker_title: true,
            ticker_autoscroll: false,
            st_pos: true,
            st_num: true,
            st_name: true,
            st_gap: true,
            st_interval: false,
            st_laps: false,
            st_current: false,
            st_best: true,
            st_last: true,
            st_status: false,
            st_bike: false,
            st_penalty: false,
            st_crashed: false,
            rel_num: true,
            rel_name: true,
            rel_gap: true,
            rel_laps: false,
            rel_current: false,
            rel_pos: false,
            rel_bike: false,
            rel_penalty: false,
            rel_interval: false,
            rel_crashed: false,
            rel_best: true,
            rel_last: true,
            map_others: true,
            map_sf: true,
            map_sectors: true,
            map_name: true,
            map_numbers: true,
            map_arrows: true,
            map_crown: true,
            map_place: true,
            map_dot: DotLabel::Position,
            mini_others: true,
            mini_sf: true,
            mini_sectors: true,
            mini_numbers: true,
            mini_arrows: true,
            mini_crown: true,
            mini_place: true,
            mini_dot: DotLabel::Number,
            radar_sides: true,
            radar_rear: true,
            radar_rings: true,
            radar_range: RADAR_RANGE_DEFAULT,
            st_hl: 50,
            st_text: TableText::White,
            st_stripe: true,
            rel_hl: 50,
            rel_text: TableText::White,
            rel_stripe: true,
            mini_zoom: 70,
            dash_rev: true,
            dash_yellow: false,
            dash_blue: false,
            dash_red: false,
            dash_simple: false,
            dash_shift_color: false,
            dash_left: DashField::Engine,
            dash_mid: DashField::Air,
            dash_right: DashField::Best,
            ticker_left: BoardField::Lap,
            ticker_right: BoardField::Air,
            st_head: BoardField::DEFAULT_HEAD,
            st_foot: BoardField::DEFAULT_FOOT,
            rel_head: BoardField::DEFAULT_HEAD,
            rel_foot: BoardField::DEFAULT_FOOT,
            stance_mode: StanceMode::Toggle,
            stance_style: StanceStyle::Text,
            stance_show_sit: false,
            lean_style: LeanStyle::Figure,
            gamepad_style: GamepadStyle::Auto,
            telemetry_traces: true,
            telemetry_trace_throttle: true,
            telemetry_trace_brake: true,
            telemetry_trace_steer: false,
            telemetry_bars: true,
            telemetry_bar_clutch: true,
            telemetry_bar_brake: true,
            telemetry_bar_throttle: true,
            telemetry_bar_steer: false,
            telemetry_dial: true,
            sys_apps: default_sys_apps(),
            st_order: StField::ALL.to_vec(),
            rel_order: RelField::ALL.to_vec(),
            st_w_pos: 26,
            st_w_num: 30,
            st_w_name: 80,
            st_w_gap: 58,
            st_w_interval: 58,
            st_w_laps: 90,
            st_w_current: 72,
            st_w_best: 58,
            st_w_last: 54,
            st_w_status: 40,
            st_w_bike: 56,
            st_w_penalty: 48,
            st_w_crashed: 44,
            rel_w_num: 32,
            rel_w_name: 80,
            rel_w_gap: 58,
            rel_w_laps: 90,
            rel_w_current: 72,
            rel_w_pos: 28,
            rel_w_bike: 56,
            rel_w_penalty: 48,
            rel_w_interval: 58,
            rel_w_crashed: 44,
            rel_w_best: 54,
            rel_w_last: 54,
        }
    }

    pub fn prefs(&self, id: WidgetId) -> &WidgetPrefs {
        &self.widgets[id.idx()]
    }

    pub fn prefs_mut(&mut self, id: WidgetId) -> &mut WidgetPrefs {
        &mut self.widgets[id.idx()]
    }

    fn migrate_rects(&mut self) {
        migrate_default_dash(&mut self[WidgetId::Dash].rect);
        migrate_default_sector(&mut self[WidgetId::Sector].rect);
        migrate_default_flag(&mut self[WidgetId::Flag].rect);
    }
}

impl Index<WidgetId> for HudLayout {
    type Output = WidgetPrefs;

    fn index(&self, id: WidgetId) -> &WidgetPrefs {
        self.prefs(id)
    }
}

impl IndexMut<WidgetId> for HudLayout {
    fn index_mut(&mut self, id: WidgetId) -> &mut WidgetPrefs {
        self.prefs_mut(id)
    }
}

#[derive(Clone)]
pub struct HudConfig {
    layouts: [HudLayout; SessionPreset::COUNT],
    pub active_preset: SessionPreset,
    pub settings_preset: SessionPreset,
    /// True while a session is live. The HUD uses `active_preset`; F8 can edit another slot.
    pub session_live: bool,
    /// Kept in the ini for older builds. No widget is gated on this anymore.
    pub experimental: bool,
    /// Plugin-only: when true the in-game HUD draws. Overlay still saves this key.
    pub ingame_hud: bool,
    pub font_family: FontFamily,
    pub units: UnitPrefs,
    pub start_with_windows: bool,
    pub minimize_on_close: bool,
    pub close_with_game: bool,
    pub open_with_game: bool,
    pub auto_update_on_launch: bool,
    /// Last version whose What's new modal was dismissed with Got it.
    pub whats_new_seen: String,
    /// Overlay version on the first launch that wrote this settings file.
    /// `"unknown"` if they already had settings before this field existed.
    pub first_install_version: String,
    pub settings_key: SettingsKey,
    pub stance_bind: StanceBind,
    loaded_mtime: Option<SystemTime>,
}

impl HudConfig {
    pub fn new() -> Self {
        let layout = HudLayout::new();
        Self {
            layouts: [
                layout.clone(),
                layout.clone(),
                layout.clone(),
                layout,
            ],
            active_preset: SessionPreset::Race,
            settings_preset: SessionPreset::Race,
            session_live: false,
            experimental: false,
            ingame_hud: false,
            font_family: FontFamily::Exo2,
            units: UnitPrefs::all(Units::Metric),
            start_with_windows: false,
            minimize_on_close: false,
            close_with_game: false,
            open_with_game: false,
            auto_update_on_launch: false,
            whats_new_seen: String::new(),
            first_install_version: String::new(),
            settings_key: SettingsKey::F8,
            stance_bind: StanceBind::PadRb,
            loaded_mtime: None,
        }
    }

    pub fn live(&self) -> &HudLayout {
        &self.layouts[self.active_preset.idx()]
    }

    pub fn live_mut(&mut self) -> &mut HudLayout {
        &mut self.layouts[self.active_preset.idx()]
    }

    pub fn edit(&self) -> &HudLayout {
        &self.layouts[self.settings_preset.idx()]
    }

    pub fn edit_mut(&mut self) -> &mut HudLayout {
        &mut self.layouts[self.settings_preset.idx()]
    }

    pub fn for_overlay(&self) -> Self {
        let mut c = self.clone();
        c.settings_preset = c.active_preset;
        c
    }

    pub fn copy_settings_to(&mut self, dst: SessionPreset) {
        if dst == self.settings_preset {
            return;
        }
        let src = self.edit().clone();
        self.layouts[dst.idx()] = src;
    }

    pub fn copy_settings_to_all(&mut self) {
        let src = self.edit().clone();
        self.layouts = [src.clone(), src.clone(), src.clone(), src];
    }

    /// Follow the live session. When `hold_settings` is set and F8 is on another
    /// slot, keep editing that slot; otherwise Settings tracks the HUD.
    pub fn sync_live(&mut self, preset: Option<SessionPreset>, hold_settings: bool) {
        match preset {
            Some(p) => {
                if !hold_settings || self.settings_preset == self.active_preset {
                    self.settings_preset = p;
                }
                self.active_preset = p;
                self.session_live = true;
            }
            None => self.session_live = false,
        }
    }

    pub fn load_file() -> Self {
        let path = ini_path();
        let legacy = legacy_ini_path();
        let mut cfg = Self::new();
        let text = fs::read_to_string(&path)
            .or_else(|_| fs::read_to_string(&legacy));
        let Ok(text) = text else {
            cfg.first_install_version = env!("CARGO_PKG_VERSION").to_string();
            cfg.save();
            return cfg;
        };
        let meta_path = if path.is_file() { &path } else { &legacy };
        cfg.loaded_mtime = fs::metadata(meta_path).and_then(|m| m.modified()).ok();
        let mut saw_last_cols = [false; SessionPreset::COUNT];
        let mut saw_first_install = false;
        let mut saw_unit = [false; UnitKind::COUNT];
        let mut saw_preset = [false; SessionPreset::COUNT];
        let mut section = IniSection::Legacy;
        let mut legacy_layout = HudLayout::new();
        let mut saw_legacy_layout = false;
        let mut legacy_last_cols = false;
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }
            if line.starts_with('[') {
                section = parse_ini_section(line);
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let key = k.trim();
            let val = v.trim();
            let f = val.parse::<f32>().unwrap_or(0.0);
            let b = val == "1" || val.eq_ignore_ascii_case("true") || val.eq_ignore_ascii_case("yes");
            match section {
                IniSection::App => {
                    apply_app_key(&mut cfg, key, val, b, &mut saw_first_install, &mut saw_unit);
                }
                IniSection::Preset(p) => {
                    saw_preset[p.idx()] = true;
                    let layout = &mut cfg.layouts[p.idx()];
                    if apply_widget_prefs(layout, key, val, f, b) {
                        continue;
                    }
                    apply_layout_key(layout, key, val, b, &mut saw_last_cols[p.idx()]);
                }
                IniSection::Legacy => {
                    if apply_app_key(&mut cfg, key, val, b, &mut saw_first_install, &mut saw_unit) {
                        continue;
                    }
                    saw_legacy_layout = true;
                    if apply_widget_prefs(&mut legacy_layout, key, val, f, b) {
                        continue;
                    }
                    apply_layout_key(&mut legacy_layout, key, val, b, &mut legacy_last_cols);
                }
            }
        }
        if saw_preset.iter().any(|&on| on) {
            let donor = SessionPreset::ALL
                .iter()
                .find(|p| saw_preset[p.idx()])
                .map(|p| cfg.layouts[p.idx()].clone())
                .unwrap_or_else(HudLayout::new);
            for p in SessionPreset::ALL {
                if !saw_preset[p.idx()] {
                    cfg.layouts[p.idx()] = donor.clone();
                }
                if !saw_last_cols[p.idx()] && saw_preset[p.idx()] {
                    cfg.layouts[p.idx()].st_best = true;
                    cfg.layouts[p.idx()].st_last = true;
                    cfg.layouts[p.idx()].rel_best = true;
                    cfg.layouts[p.idx()].rel_last = true;
                }
            }
        } else if saw_legacy_layout {
            if !legacy_last_cols {
                legacy_layout.st_best = true;
                legacy_layout.st_last = true;
                legacy_layout.rel_best = true;
                legacy_layout.rel_last = true;
            }
            cfg.layouts = [
                legacy_layout.clone(),
                legacy_layout.clone(),
                legacy_layout.clone(),
                legacy_layout,
            ];
        }
        for layout in &mut cfg.layouts {
            layout.migrate_rects();
        }
        if !saw_first_install || cfg.first_install_version.is_empty() {
            cfg.first_install_version = "unknown".into();
            cfg.save();
        }
        cfg
    }

    pub fn add_sys_preset(&mut self, key: &str) {
        if self.sys_apps.len() >= SYS_APP_MAX {
            return;
        }
        if self.sys_apps.iter().any(|a| a.key == key) {
            return;
        }
        if let Some(p) = SYS_PRESETS.iter().find(|p| p.key == key && p.addable()) {
            self.sys_apps.push(SysApp::from_preset(p, true));
        }
    }

    pub fn add_sys_exe(&mut self, exe_name: &str) {
        let name = exe_name.trim().to_ascii_lowercase();
        if !name.ends_with(".exe") || name.contains('\\') || name.contains('/') || name.contains('|') {
            return;
        }
        if let Some(p) = SYS_PRESETS.iter().find(|p| p.names.iter().any(|n| *n == name)) {
            if let Some(a) = self.sys_apps.iter_mut().find(|a| a.key == p.key) {
                a.show = true;
                return;
            }
            self.add_sys_preset(p.key);
            return;
        }
        if self.sys_apps.iter().any(|a| a.names.iter().any(|n| n == &name)) {
            if let Some(a) = self.sys_apps.iter_mut().find(|a| a.names.iter().any(|n| n == &name)) {
                a.show = true;
            }
            return;
        }
        if self.sys_apps.len() >= SYS_APP_MAX {
            return;
        }
        let stem = exe_name
            .trim()
            .rsplit_once('.')
            .map(|(s, _)| s)
            .unwrap_or(exe_name.trim());
        let label = sanitize_sys_token(stem);
        let label = if label.is_empty() {
            "App".into()
        } else {
            label
        };
        self.sys_apps.push(SysApp {
            key: format!("exe:{name}"),
            label,
            names: vec![name],
            kind: SysAppKind::Exe,
            show: true,
        });
    }

    pub fn toggle_sys_app(&mut self, i: usize) {
        if let Some(a) = self.sys_apps.get_mut(i) {
            a.show = !a.show;
        }
    }

    pub fn remove_sys_app(&mut self, i: usize) {
        if self.sys_apps.get(i).is_some_and(|a| a.removable()) {
            self.sys_apps.remove(i);
        }
    }

    pub fn save(&mut self) {
        let path = ini_path();
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let body = format!(
            "# Holeshot HUD layout (normalized 0..1, origin top-left)\n\
             [App]\n\
             font_family={}\nunits={}\nunits_speed={}\nunits_liquids={}\nunits_temperature={}\n\
             settings_key={}\nstart_with_windows={}\nminimize_on_close={}\n\
             close_with_game={}\nopen_with_game={}\nauto_update_on_launch={}\nwhats_new_seen={}\n\
             first_install_version={}\nexperimental={}\ningame_hud={}\nstance_bind={}\nactive_preset={}\n\
             \n[Practice]\n{}\n\n[Warmup]\n{}\n\n[Race]\n{}\n\n[Spectate]\n{}\n",
            self.font_family.key(),
            self.units.speed.key(),
            self.units.speed.key(),
            self.units.liquids.key(),
            self.units.temperature.key(),
            self.settings_key.key(),
            b(self.start_with_windows),
            b(self.minimize_on_close),
            b(self.close_with_game),
            b(self.open_with_game),
            b(self.auto_update_on_launch),
            self.whats_new_seen,
            self.first_install_version,
            b(self.experimental),
            b(self.ingame_hud),
            self.stance_bind.key(),
            self.active_preset.key(),
            layout_ini(&self.layouts[SessionPreset::Practice.idx()]),
            layout_ini(&self.layouts[SessionPreset::Warmup.idx()]),
            layout_ini(&self.layouts[SessionPreset::Race.idx()]),
            layout_ini(&self.layouts[SessionPreset::Spectate.idx()]),
        );
        let _ = fs::write(&path, body);
        self.loaded_mtime = fs::metadata(&path).and_then(|m| m.modified()).ok();
        let legacy = legacy_ini_path();
        if legacy != path {
            let _ = fs::remove_file(legacy);
        }
    }

    pub fn apply_to_snapshot(&self, s: &mut Snapshot) {
        let lay = self.live();
        s.standings_rect = lay[WidgetId::Standings].rect;
        s.relative = lay[WidgetId::Relative].rect;
        s.map = lay[WidgetId::Map].rect;
        s.show_standings = i32::from(lay[WidgetId::Standings].show);
        s.show_relative = i32::from(lay[WidgetId::Relative].show);
        s.show_map = i32::from(lay[WidgetId::Map].show);
        s.standings_rows = lay.standings_rows;
        s.relative_count = lay.relative_count;
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

    pub fn prefs(&self, id: WidgetId) -> &WidgetPrefs {
        &self.edit()[id]
    }

    pub fn prefs_mut(&mut self, id: WidgetId) -> &mut WidgetPrefs {
        &mut self.edit_mut()[id]
    }

    pub fn font_pct(&self, id: WidgetId) -> i32 {
        self[id].font
    }

    pub fn set_font_pct(&mut self, id: WidgetId, v: i32) {
        self[id].font = v.clamp(70, 160);
    }

    pub fn bold(&self, id: WidgetId) -> bool {
        self[id].bold
    }

    pub fn set_bold(&mut self, id: WidgetId, on: bool) {
        self[id].bold = on;
    }

    pub fn widget_rect(&self, id: WidgetId) -> Rect {
        self[id].rect
    }

    pub fn snapped_rect(&self, id: WidgetId, align: SnapAlign) -> Rect {
        let mut r = self.widget_rect(id);
        snap_rect(&mut r, align);
        r
    }

    pub fn snap(&mut self, id: WidgetId, align: SnapAlign) {
        snap_rect(&mut self[id].rect, align);
    }

    pub fn relative_cols(&self) -> Vec<RelField> {
        let mut cols: Vec<_> = self.rel_order.iter().copied().filter(|c| c.enabled(self)).collect();
        if cols.is_empty() {
            cols.push(RelField::Name);
        }
        cols
    }

    /// Settings → Labs → Experimental widgets. Gates Controller.
    pub fn experimental_unlocked(&self) -> bool {
        self.experimental
    }

    pub fn gamepad_visible(&self) -> bool {
        self.experimental_unlocked() && self[WidgetId::Gamepad].show
    }

    pub fn sector_visible(&self) -> bool {
        self[WidgetId::Sector].show
    }

    pub fn sector_hist_count(&self) -> usize {
        self.sector_hist_laps.clamp(1, 5) as usize
    }

    pub fn telemetry_draw_traces(&self) -> bool {
        self.telemetry_traces
            && (self.telemetry_trace_throttle || self.telemetry_trace_brake || self.telemetry_trace_steer)
    }

    pub fn telemetry_draw_bars(&self) -> bool {
        self.telemetry_bars
            && (self.telemetry_bar_clutch
                || self.telemetry_bar_brake
                || self.telemetry_bar_throttle
                || self.telemetry_bar_steer)
    }

    pub fn delta_visible(&self) -> bool {
        self[WidgetId::Delta].show
    }

    pub fn stance_visible(&self) -> bool {
        self[WidgetId::Stance].show
    }

    pub fn any_overlay_widget(&self) -> bool {
        WidgetId::ALL.iter().any(|&id| match id {
            WidgetId::Sector => self.sector_visible(),
            WidgetId::Delta => self.delta_visible(),
            WidgetId::Stance => self.stance_visible(),
            WidgetId::Gamepad => self.gamepad_visible(),
            _ => self[id].show,
        })
    }
}

impl Index<WidgetId> for HudConfig {
    type Output = WidgetPrefs;

    fn index(&self, id: WidgetId) -> &WidgetPrefs {
        self.prefs(id)
    }
}

impl IndexMut<WidgetId> for HudConfig {
    fn index_mut(&mut self, id: WidgetId) -> &mut WidgetPrefs {
        self.prefs_mut(id)
    }
}

impl Deref for HudConfig {
    type Target = HudLayout;

    fn deref(&self) -> &HudLayout {
        self.edit()
    }
}

impl DerefMut for HudConfig {
    fn deref_mut(&mut self) -> &mut HudLayout {
        self.edit_mut()
    }
}

#[derive(Clone, Copy)]
enum IniSection {
    App,
    Preset(SessionPreset),
    Legacy,
}

fn parse_ini_section(line: &str) -> IniSection {
    let name = line
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim()
        .to_ascii_lowercase();
    match name.as_str() {
        "app" => IniSection::App,
        "practice" => IniSection::Preset(SessionPreset::Practice),
        "warmup" => IniSection::Preset(SessionPreset::Warmup),
        "race" => IniSection::Preset(SessionPreset::Race),
        "spectate" | "spectating" => IniSection::Preset(SessionPreset::Spectate),
        _ => IniSection::Legacy,
    }
}

fn apply_app_key(
    cfg: &mut HudConfig,
    key: &str,
    val: &str,
    b: bool,
    saw_first_install: &mut bool,
    saw_unit: &mut [bool; UnitKind::COUNT],
) -> bool {
    match key {
        "experimental" | "feature_experimental" | "feature_sector" => cfg.experimental = b,
        "ingame_hud" => cfg.ingame_hud = b,
        "font_family" => cfg.font_family = FontFamily::parse(val),
        "units" => {
            let u = Units::parse(val);
            for kind in UnitKind::ALL {
                if !saw_unit[kind.idx()] {
                    cfg.units.set(kind, u);
                }
            }
        }
        "start_with_windows" => cfg.start_with_windows = b,
        "minimize_on_close" => cfg.minimize_on_close = b,
        "close_with_game" => cfg.close_with_game = b,
        "open_with_game" => cfg.open_with_game = b,
        "auto_update_on_launch" => cfg.auto_update_on_launch = b,
        "whats_new_seen" => cfg.whats_new_seen = val.trim().to_string(),
        "first_install_version" => {
            cfg.first_install_version = val.trim().to_string();
            *saw_first_install = true;
        }
        "settings_key" => cfg.settings_key = SettingsKey::parse(val),
        "stance_bind" => cfg.stance_bind = StanceBind::parse(val),
        "active_preset" => {
            let p = SessionPreset::parse(val);
            cfg.active_preset = p;
            cfg.settings_preset = p;
        }
        other => {
            let Some(kind) = UnitKind::parse_key(other) else {
                return false;
            };
            cfg.units.set(kind, Units::parse(val));
            saw_unit[kind.idx()] = true;
        }
    }
    true
}

fn apply_layout_key(cfg: &mut HudLayout, key: &str, val: &str, b: bool, saw_last_cols: &mut bool) {
    match key {
        "sector_live" => cfg.sector_live = b,
        "sector_session" => cfg.sector_session = b,
        "sector_hist" => cfg.sector_hist = b,
        "sector_hist_laps" => cfg.sector_hist_laps = val.parse().unwrap_or(3).clamp(1, 5),
        "delta_session" => cfg.delta_session = b,
        "flag_caution" => {
            cfg.flag_yellow = b;
            cfg.flag_blue = b;
        }
        "flag_yellow" => cfg.flag_yellow = b,
        "flag_blue" => cfg.flag_blue = b,
        "flag_red" => cfg.flag_red = b,
        "flag_text" => cfg.flag_text = b,
        "standings_rows" => cfg.standings_rows = val.parse().unwrap_or(12).max(3),
        "relative_count" => cfg.relative_count = val.parse().unwrap_or(3).max(1),
        "ticker_count" => cfg.ticker_count = val.parse().unwrap_or(7).clamp(3, 15),
        "ticker_title" => cfg.ticker_title = b,
        "ticker_autoscroll" => cfg.ticker_autoscroll = b,
        "st_pos" => cfg.st_pos = b,
        "st_num" => cfg.st_num = b,
        "st_name" => cfg.st_name = b,
        "st_gap" => cfg.st_gap = b,
        "st_interval" => cfg.st_interval = b,
        "st_laps" => cfg.st_laps = b,
        "st_current" => cfg.st_current = b,
        "st_best" => cfg.st_best = b,
        "st_last" => {
            cfg.st_last = b;
            *saw_last_cols = true;
        }
        "st_status" => cfg.st_status = b,
        "st_bike" => cfg.st_bike = b,
        "st_penalty" => cfg.st_penalty = b,
        "st_crashed" => cfg.st_crashed = b,
        "rel_num" => cfg.rel_num = b,
        "rel_name" => cfg.rel_name = b,
        "rel_gap" => cfg.rel_gap = b,
        "rel_laps" => cfg.rel_laps = b,
        "rel_current" => cfg.rel_current = b,
        "rel_pos" => cfg.rel_pos = b,
        "rel_bike" => cfg.rel_bike = b,
        "rel_penalty" => cfg.rel_penalty = b,
        "rel_interval" => cfg.rel_interval = b,
        "rel_crashed" => cfg.rel_crashed = b,
        "rel_best" => cfg.rel_best = b,
        "rel_last" => cfg.rel_last = b,
        "map_others" => cfg.map_others = b,
        "map_sf" => cfg.map_sf = b,
        "map_sectors" => cfg.map_sectors = b,
        "map_name" => cfg.map_name = b,
        "map_numbers" => cfg.map_numbers = b,
        "map_arrows" => cfg.map_arrows = b,
        "map_crown" => cfg.map_crown = b,
        "map_place" => cfg.map_place = b,
        "map_dot" => cfg.map_dot = DotLabel::parse(val),
        "mini_others" => cfg.mini_others = b,
        "mini_sf" => cfg.mini_sf = b,
        "mini_sectors" => cfg.mini_sectors = b,
        "mini_numbers" => cfg.mini_numbers = b,
        "mini_arrows" => cfg.mini_arrows = b,
        "mini_crown" => cfg.mini_crown = b,
        "mini_place" => cfg.mini_place = b,
        "mini_dot" => cfg.mini_dot = DotLabel::parse(val),
        "radar_sides" => cfg.radar_sides = b,
        "radar_rear" => cfg.radar_rear = b,
        "radar_rings" => cfg.radar_rings = b,
        "radar_range" => cfg.radar_range = clamp_radar_range(val),
        "st_hl" => cfg.st_hl = clamp_pct(val),
        "st_text" => cfg.st_text = TableText::parse(val),
        "st_stripe" => cfg.st_stripe = b,
        "rel_hl" => cfg.rel_hl = clamp_pct(val),
        "rel_text" => cfg.rel_text = TableText::parse(val),
        "rel_stripe" => cfg.rel_stripe = b,
        "mini_zoom" => cfg.mini_zoom = clamp_pct(val),
        "dash_rev" => cfg.dash_rev = b,
        "dash_yellow" => cfg.dash_yellow = b,
        "dash_blue" => cfg.dash_blue = b,
        "dash_red" => cfg.dash_red = b,
        "dash_simple" => cfg.dash_simple = b,
        "dash_shift_color" => cfg.dash_shift_color = b,
        "dash_left" => cfg.dash_left = DashField::parse(val),
        "dash_mid" => cfg.dash_mid = DashField::parse(val),
        "dash_right" => cfg.dash_right = DashField::parse(val),
        "ticker_left" => cfg.ticker_left = BoardField::parse(val),
        "ticker_right" => cfg.ticker_right = BoardField::parse(val),
        "st_head" => cfg.st_head = parse_board(val, BoardField::DEFAULT_HEAD),
        "st_foot" => cfg.st_foot = parse_board(val, BoardField::DEFAULT_FOOT),
        "rel_head" => cfg.rel_head = parse_board(val, BoardField::DEFAULT_HEAD),
        "rel_foot" => cfg.rel_foot = parse_board(val, BoardField::DEFAULT_FOOT),
        "stance_mode" => cfg.stance_mode = StanceMode::parse(val),
        "stance_style" => cfg.stance_style = StanceStyle::parse(val),
        "lean_style" => cfg.lean_style = LeanStyle::parse(val),
        "gamepad_style" => cfg.gamepad_style = GamepadStyle::parse(val),
        "sys_apps" => cfg.sys_apps = parse_sys_apps(val),
        "stance_show_sit" => cfg.stance_show_sit = b,
        "stance_icon" => {
            if b {
                cfg.stance_style = StanceStyle::Icon;
            }
        }
        "st_order" => cfg.st_order = parse_st_order(val),
        "rel_order" => cfg.rel_order = parse_rel_order(val),
        "st_w_pos" => cfg.st_w_pos = clamp_w(val),
        "st_w_num" => cfg.st_w_num = clamp_w(val),
        "st_w_name" => cfg.st_w_name = clamp_name_w(val),
        "st_w_gap" => cfg.st_w_gap = clamp_w(val),
        "st_w_interval" => cfg.st_w_interval = clamp_w(val),
        "st_w_laps" => cfg.st_w_laps = clamp_w(val),
        "st_w_current" => cfg.st_w_current = clamp_w(val),
        "st_w_best" => cfg.st_w_best = clamp_w(val),
        "st_w_last" => cfg.st_w_last = clamp_w(val),
        "st_w_status" => cfg.st_w_status = clamp_w(val),
        "st_w_bike" => cfg.st_w_bike = clamp_w(val),
        "st_w_penalty" => cfg.st_w_penalty = clamp_w(val),
        "st_w_crashed" => cfg.st_w_crashed = clamp_w(val),
        "rel_w_num" => cfg.rel_w_num = clamp_w(val),
        "rel_w_name" => cfg.rel_w_name = clamp_name_w(val),
        "rel_w_gap" => cfg.rel_w_gap = clamp_w(val),
        "rel_w_laps" => cfg.rel_w_laps = clamp_w(val),
        "rel_w_current" => cfg.rel_w_current = clamp_w(val),
        "rel_w_pos" => cfg.rel_w_pos = clamp_w(val),
        "rel_w_bike" => cfg.rel_w_bike = clamp_w(val),
        "rel_w_penalty" => cfg.rel_w_penalty = clamp_w(val),
        "rel_w_interval" => cfg.rel_w_interval = clamp_w(val),
        "rel_w_crashed" => cfg.rel_w_crashed = clamp_w(val),
        "rel_w_best" => cfg.rel_w_best = clamp_w(val),
        "rel_w_last" => cfg.rel_w_last = clamp_w(val),
        "telemetry_traces" => cfg.telemetry_traces = b,
        "telemetry_trace_throttle" => cfg.telemetry_trace_throttle = b,
        "telemetry_trace_brake" => cfg.telemetry_trace_brake = b,
        "telemetry_trace_steer" => cfg.telemetry_trace_steer = b,
        "telemetry_bars" => cfg.telemetry_bars = b,
        "telemetry_bar_clutch" => cfg.telemetry_bar_clutch = b,
        "telemetry_bar_brake" => cfg.telemetry_bar_brake = b,
        "telemetry_bar_throttle" => cfg.telemetry_bar_throttle = b,
        "telemetry_bar_steer" => cfg.telemetry_bar_steer = b,
        "telemetry_dial" => cfg.telemetry_dial = b,
        _ => {}
    }
}

fn apply_widget_prefs(cfg: &mut HudLayout, key: &str, val: &str, f: f32, b: bool) -> bool {
    for id in WidgetId::ALL {
        let k = id.ini();
        if key == k.show {
            cfg[id].show = b;
            return true;
        }
        if key == k.font {
            cfg[id].font = clamp_font(val);
            return true;
        }
        if key == k.bold {
            cfg[id].bold = b;
            return true;
        }
        if key == k.bg {
            cfg[id].bg = clamp_pct(val);
            return true;
        }
        if let Some(axis) = key.strip_prefix(k.rect).and_then(|s| s.strip_prefix('_')) {
            match axis {
                "x" => cfg[id].rect.x = f,
                "y" => cfg[id].rect.y = f,
                "w" => cfg[id].rect.w = f,
                "h" => {
                    cfg[id].rect.h = if id == WidgetId::Ticker {
                        f.clamp(0.035, 0.07)
                    } else {
                        f
                    };
                }
                _ => continue,
            }
            return true;
        }
    }
    false
}

fn layout_ini(l: &HudLayout) -> String {
    let st = l[WidgetId::Standings];
    let rel = l[WidgetId::Relative];
    let map = l[WidgetId::Map];
    let mini = l[WidgetId::Minimap];
    let radar = l[WidgetId::Radar];
    let dash = l[WidgetId::Dash];
    let ticker = l[WidgetId::Ticker];
    let sys = l[WidgetId::Sys];
    let sector = l[WidgetId::Sector];
    let delta = l[WidgetId::Delta];
    let stance = l[WidgetId::Stance];
    let flag = l[WidgetId::Flag];
    let lean = l[WidgetId::Lean];
    let gamepad = l[WidgetId::Gamepad];
    let telemetry = l[WidgetId::Telemetry];
    format!(
        "standings_x={}\nstandings_y={}\nstandings_w={}\nstandings_h={}\n\
         relative_x={}\nrelative_y={}\nrelative_w={}\nrelative_h={}\n\
         map_x={}\nmap_y={}\nmap_w={}\nmap_h={}\n\
         minimap_x={}\nminimap_y={}\nminimap_w={}\nminimap_h={}\n\
         radar_x={}\nradar_y={}\nradar_w={}\nradar_h={}\n\
         dash_x={}\ndash_y={}\ndash_w={}\ndash_h={}\n\
         ticker_x={}\nticker_y={}\nticker_w={}\nticker_h={}\n\
         sys_x={}\nsys_y={}\nsys_w={}\nsys_h={}\n\
         sector_x={}\nsector_y={}\nsector_w={}\nsector_h={}\n\
         delta_x={}\ndelta_y={}\ndelta_w={}\ndelta_h={}\n\
         stance_x={}\nstance_y={}\nstance_w={}\nstance_h={}\n\
         flag_x={}\nflag_y={}\nflag_w={}\nflag_h={}\n\
         lean_x={}\nlean_y={}\nlean_w={}\nlean_h={}\n\
         gamepad_x={}\ngamepad_y={}\ngamepad_w={}\ngamepad_h={}\n\
         telemetry_x={}\ntelemetry_y={}\ntelemetry_w={}\ntelemetry_h={}\n\
         show_standings={}\nshow_relative={}\nshow_map={}\nshow_minimap={}\nshow_radar={}\n\
         show_dash={}\nshow_ticker={}\nshow_sys={}\nshow_sector={}\nshow_delta={}\n\
         show_stance={}\nshow_flag={}\nshow_lean={}\nshow_gamepad={}\nshow_telemetry={}\n\
         standings_rows={}\nrelative_count={}\nticker_count={}\n\
         st_pos={}\nst_num={}\nst_name={}\nst_gap={}\nst_interval={}\nst_laps={}\nst_current={}\n\
         st_best={}\nst_last={}\nst_status={}\nst_bike={}\nst_penalty={}\nst_crashed={}\n\
         st_order={}\n\
         st_w_pos={}\nst_w_num={}\nst_w_name={}\nst_w_gap={}\nst_w_interval={}\nst_w_laps={}\n\
         st_w_current={}\nst_w_best={}\nst_w_last={}\nst_w_status={}\nst_w_bike={}\nst_w_penalty={}\nst_w_crashed={}\n\
         st_bg={}\nst_hl={}\nst_text={}\nst_stripe={}\nst_font={}\nst_bold={}\n\
         st_head={}\nst_foot={}\n\
         rel_num={}\nrel_name={}\nrel_gap={}\nrel_laps={}\nrel_current={}\nrel_pos={}\nrel_bike={}\n\
         rel_penalty={}\nrel_interval={}\nrel_crashed={}\nrel_best={}\nrel_last={}\n\
         rel_order={}\n\
         rel_w_num={}\nrel_w_name={}\nrel_w_gap={}\nrel_w_laps={}\nrel_w_current={}\nrel_w_pos={}\n\
         rel_w_bike={}\nrel_w_penalty={}\nrel_w_interval={}\nrel_w_crashed={}\nrel_w_best={}\nrel_w_last={}\n\
         rel_bg={}\nrel_hl={}\nrel_text={}\nrel_stripe={}\nrel_font={}\nrel_bold={}\n\
         rel_head={}\nrel_foot={}\n\
         map_others={}\nmap_sf={}\nmap_sectors={}\nmap_name={}\nmap_numbers={}\nmap_arrows={}\n\
         map_crown={}\nmap_place={}\nmap_dot={}\nmap_bg={}\nmap_font={}\nmap_bold={}\n\
         mini_others={}\nmini_sf={}\nmini_sectors={}\nmini_numbers={}\nmini_arrows={}\nmini_crown={}\n\
         mini_place={}\nmini_dot={}\nmini_bg={}\nmini_zoom={}\nmini_font={}\nmini_bold={}\n\
         radar_sides={}\nradar_rear={}\nradar_rings={}\nradar_range={}\nradar_bg={}\nradar_font={}\nradar_bold={}\n\
         dash_rev={}\ndash_yellow={}\ndash_blue={}\ndash_red={}\ndash_simple={}\ndash_shift_color={}\ndash_left={}\ndash_mid={}\ndash_right={}\n\
         dash_bg={}\ndash_font={}\ndash_bold={}\n\
         ticker_left={}\nticker_right={}\nticker_title={}\nticker_autoscroll={}\n\
         ticker_bg={}\nticker_font={}\nticker_bold={}\n\
         sys_bg={}\nsys_font={}\nsys_bold={}\nsys_apps={}\n\
         sector_live={}\nsector_session={}\nsector_hist={}\nsector_hist_laps={}\n\
         sector_bg={}\nsector_font={}\nsector_bold={}\n\
         delta_session={}\ndelta_bg={}\ndelta_font={}\ndelta_bold={}\n\
         stance_mode={}\nstance_style={}\nstance_show_sit={}\n\
         stance_bg={}\nstance_font={}\nstance_bold={}\n\
         flag_bg={}\nflag_yellow={}\nflag_blue={}\nflag_red={}\nflag_text={}\nflag_font={}\nflag_bold={}\n\
         lean_style={}\nlean_bg={}\nlean_font={}\nlean_bold={}\n\
         gamepad_style={}\ngamepad_bg={}\ngamepad_font={}\ngamepad_bold={}\n\
         telemetry_traces={}\ntelemetry_trace_throttle={}\ntelemetry_trace_brake={}\ntelemetry_trace_steer={}\n\
         telemetry_bars={}\ntelemetry_bar_clutch={}\ntelemetry_bar_brake={}\ntelemetry_bar_throttle={}\ntelemetry_bar_steer={}\n\
         telemetry_dial={}\n\
         telemetry_bg={}\ntelemetry_font={}\ntelemetry_bold={}",
        st.rect.x, st.rect.y, st.rect.w, st.rect.h,
        rel.rect.x, rel.rect.y, rel.rect.w, rel.rect.h,
        map.rect.x, map.rect.y, map.rect.w, map.rect.h,
        mini.rect.x, mini.rect.y, mini.rect.w, mini.rect.h,
        radar.rect.x, radar.rect.y, radar.rect.w, radar.rect.h,
        dash.rect.x, dash.rect.y, dash.rect.w, dash.rect.h,
        ticker.rect.x, ticker.rect.y, ticker.rect.w, ticker.rect.h,
        sys.rect.x, sys.rect.y, sys.rect.w, sys.rect.h,
        sector.rect.x, sector.rect.y, sector.rect.w, sector.rect.h,
        delta.rect.x, delta.rect.y, delta.rect.w, delta.rect.h,
        stance.rect.x, stance.rect.y, stance.rect.w, stance.rect.h,
        flag.rect.x, flag.rect.y, flag.rect.w, flag.rect.h,
        lean.rect.x, lean.rect.y, lean.rect.w, lean.rect.h,
        gamepad.rect.x, gamepad.rect.y, gamepad.rect.w, gamepad.rect.h,
        telemetry.rect.x, telemetry.rect.y, telemetry.rect.w, telemetry.rect.h,
        b(st.show), b(rel.show), b(map.show), b(mini.show), b(radar.show),
        b(dash.show), b(ticker.show), b(sys.show), b(sector.show), b(delta.show),
        b(stance.show), b(flag.show), b(lean.show), b(gamepad.show), b(telemetry.show),
        l.standings_rows, l.relative_count, l.ticker_count,
        b(l.st_pos), b(l.st_num), b(l.st_name), b(l.st_gap), b(l.st_interval), b(l.st_laps), b(l.st_current),
        b(l.st_best), b(l.st_last), b(l.st_status), b(l.st_bike), b(l.st_penalty), b(l.st_crashed),
        join_st(&l.st_order),
        l.st_w_pos, l.st_w_num, l.st_w_name, l.st_w_gap, l.st_w_interval, l.st_w_laps,
        l.st_w_current, l.st_w_best, l.st_w_last, l.st_w_status, l.st_w_bike, l.st_w_penalty, l.st_w_crashed,
        st.bg, l.st_hl, l.st_text.key(), b(l.st_stripe), st.font, b(st.bold),
        join_board(&l.st_head), join_board(&l.st_foot),
        b(l.rel_num), b(l.rel_name), b(l.rel_gap), b(l.rel_laps), b(l.rel_current), b(l.rel_pos), b(l.rel_bike),
        b(l.rel_penalty), b(l.rel_interval), b(l.rel_crashed), b(l.rel_best), b(l.rel_last),
        join_rel(&l.rel_order),
        l.rel_w_num, l.rel_w_name, l.rel_w_gap, l.rel_w_laps, l.rel_w_current, l.rel_w_pos,
        l.rel_w_bike, l.rel_w_penalty, l.rel_w_interval, l.rel_w_crashed, l.rel_w_best, l.rel_w_last,
        rel.bg, l.rel_hl, l.rel_text.key(), b(l.rel_stripe), rel.font, b(rel.bold),
        join_board(&l.rel_head), join_board(&l.rel_foot),
        b(l.map_others), b(l.map_sf), b(l.map_sectors), b(l.map_name), b(l.map_numbers), b(l.map_arrows),
        b(l.map_crown), b(l.map_place), l.map_dot.key(), map.bg, map.font, b(map.bold),
        b(l.mini_others), b(l.mini_sf), b(l.mini_sectors), b(l.mini_numbers), b(l.mini_arrows), b(l.mini_crown),
        b(l.mini_place), l.mini_dot.key(), mini.bg, l.mini_zoom, mini.font, b(mini.bold),
        b(l.radar_sides), b(l.radar_rear), b(l.radar_rings), l.radar_range, radar.bg, radar.font, b(radar.bold),
        b(l.dash_rev), b(l.dash_yellow), b(l.dash_blue), b(l.dash_red), b(l.dash_simple), b(l.dash_shift_color), l.dash_left.key(), l.dash_mid.key(), l.dash_right.key(),
        dash.bg, dash.font, b(dash.bold),
        l.ticker_left.key(), l.ticker_right.key(), b(l.ticker_title), b(l.ticker_autoscroll),
        ticker.bg, ticker.font, b(ticker.bold),
        sys.bg, sys.font, b(sys.bold), encode_sys_apps(&l.sys_apps),
        b(l.sector_live), b(l.sector_session), b(l.sector_hist), l.sector_hist_laps.clamp(1, 5),
        sector.bg, sector.font, b(sector.bold),
        b(l.delta_session), delta.bg, delta.font, b(delta.bold),
        l.stance_mode.key(), l.stance_style.key(), b(l.stance_show_sit),
        stance.bg, stance.font, b(stance.bold),
        flag.bg, b(l.flag_yellow), b(l.flag_blue), b(l.flag_red), b(l.flag_text), flag.font, b(flag.bold),
        l.lean_style.key(), lean.bg, lean.font, b(lean.bold),
        l.gamepad_style.key(), gamepad.bg, gamepad.font, b(gamepad.bold),
        b(l.telemetry_traces), b(l.telemetry_trace_throttle), b(l.telemetry_trace_brake), b(l.telemetry_trace_steer),
        b(l.telemetry_bars), b(l.telemetry_bar_clutch), b(l.telemetry_bar_brake), b(l.telemetry_bar_throttle), b(l.telemetry_bar_steer),
        b(l.telemetry_dial),
        telemetry.bg, telemetry.font, b(telemetry.bold),
    )
}

fn b(v: bool) -> i32 {
    i32::from(v)
}

fn clamp_w(val: &str) -> i32 {
    val.parse().unwrap_or(40).clamp(COL_W_MIN, COL_W_MAX)
}

fn clamp_name_w(val: &str) -> i32 {
    val.parse().unwrap_or(80).clamp(COL_W_MIN, NAME_W_MAX)
}

fn clamp_pct(val: &str) -> i32 {
    val.parse().unwrap_or(80).clamp(0, 100)
}

fn clamp_radar_range(val: &str) -> i32 {
    val.parse().unwrap_or(RADAR_RANGE_DEFAULT).clamp(RADAR_RANGE_MIN, RADAR_RANGE_MAX)
}

fn clamp_font(val: &str) -> i32 {
    val.parse().unwrap_or(100).clamp(70, 160)
}

fn snap_rect(r: &mut Rect, align: SnapAlign) {
    const PAD: f32 = 0.012;
    let max_x = (1.0 - r.w - PAD).max(PAD);
    let max_y = (1.0 - r.h - PAD).max(PAD);
    let cx = ((1.0 - r.w) * 0.5).clamp(PAD, max_x);
    let cy = ((1.0 - r.h) * 0.5).clamp(PAD, max_y);
    match align {
        SnapAlign::TopLeft => {
            r.x = PAD;
            r.y = PAD;
        }
        SnapAlign::Top => {
            r.x = cx;
            r.y = PAD;
        }
        SnapAlign::TopRight => {
            r.x = max_x;
            r.y = PAD;
        }
        SnapAlign::Left => {
            r.x = PAD;
            r.y = cy;
        }
        SnapAlign::Center => {
            r.x = cx;
            r.y = cy;
        }
        SnapAlign::Right => {
            r.x = max_x;
            r.y = cy;
        }
        SnapAlign::BottomLeft => {
            r.x = PAD;
            r.y = max_y;
        }
        SnapAlign::Bottom => {
            r.x = cx;
            r.y = max_y;
        }
        SnapAlign::BottomRight => {
            r.x = max_x;
            r.y = max_y;
        }
        SnapAlign::HCenter => r.x = cx,
        SnapAlign::VCenter => r.y = cy,
    }
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

fn join_board(fields: &[BoardField; 3]) -> String {
    fields.iter().map(|f| f.key()).collect::<Vec<_>>().join(",")
}

fn parse_board(s: &str, fallback: [BoardField; 3]) -> [BoardField; 3] {
    let mut out = fallback;
    for (i, part) in s.split(',').take(3).enumerate() {
        out[i] = BoardField::parse(part);
    }
    out
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BoardField {
    None,
    Position,
    ClassPos,
    Session,
    RaceTime,
    Lap,
    LapsLeft,
    Track,
    Air,
    Best,
    SessionBest,
    LocalTime,
    Riders,
    SessionType,
    Fuel,
    FuelPct,
    Setup,
    GapAhead,
    GapBehind,
}

impl BoardField {
    pub const ALL: [Self; 19] = [
        Self::None,
        Self::Position,
        Self::ClassPos,
        Self::Session,
        Self::RaceTime,
        Self::Lap,
        Self::LapsLeft,
        Self::Track,
        Self::Air,
        Self::Best,
        Self::SessionBest,
        Self::LocalTime,
        Self::Riders,
        Self::SessionType,
        Self::Fuel,
        Self::FuelPct,
        Self::Setup,
        Self::GapAhead,
        Self::GapBehind,
    ];

    pub const DEFAULT_HEAD: [Self; 3] = [Self::Session, Self::None, Self::Riders];
    pub const DEFAULT_FOOT: [Self; 3] = [Self::None, Self::None, Self::None];

    pub fn key(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Position => "pos",
            Self::ClassPos => "classpos",
            Self::Session => "sess",
            Self::RaceTime => "race",
            Self::Lap => "lap",
            Self::LapsLeft => "left",
            Self::Track => "track",
            Self::Air => "air",
            Self::Best => "best",
            Self::SessionBest => "sbest",
            Self::LocalTime => "local",
            Self::Riders => "riders",
            Self::SessionType => "stype",
            Self::Fuel => "fuel",
            Self::FuelPct => "fuelpct",
            Self::Setup => "setup",
            Self::GapAhead => "gapahead",
            Self::GapBehind => "gapbehind",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Position => "Position",
            Self::ClassPos => "Class position",
            Self::Session => "Session time",
            Self::RaceTime => "Race time",
            Self::Lap => "Lap",
            Self::LapsLeft => "Laps remaining",
            Self::Track => "Track name",
            Self::Air => "Air temp",
            Self::Best => "Best lap",
            Self::SessionBest => "Session best",
            Self::LocalTime => "Local time",
            Self::Riders => "Riders",
            Self::SessionType => "Session type",
            Self::Fuel => "Fuel",
            Self::FuelPct => "Fuel %",
            Self::Setup => "Setup",
            Self::GapAhead => "Gap ahead",
            Self::GapBehind => "Gap behind",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim() {
            "none" => Self::None,
            "pos" | "position" => Self::Position,
            "classpos" | "class_pos" => Self::ClassPos,
            "sess" | "session" => Self::Session,
            "race" | "racetime" => Self::RaceTime,
            "lap" => Self::Lap,
            "left" | "lapsleft" => Self::LapsLeft,
            "track" => Self::Track,
            "air" => Self::Air,
            "best" => Self::Best,
            "sbest" | "sessionbest" => Self::SessionBest,
            "local" | "localtime" => Self::LocalTime,
            "riders" | "count" => Self::Riders,
            "stype" | "sessiontype" => Self::SessionType,
            "fuel" => Self::Fuel,
            "fuelpct" | "fuel%" | "fuelpercent" => Self::FuelPct,
            "setup" => Self::Setup,
            "gapahead" | "gap_ahead" | "ahead" => Self::GapAhead,
            "gapbehind" | "gap_behind" | "behind" => Self::GapBehind,
            _ => Self::None,
        }
    }

    pub fn icon(self) -> char {
        match self {
            Self::None => '\0',
            Self::Position | Self::ClassPos => '\u{f091}',
            Self::Session | Self::RaceTime | Self::LocalTime => '\u{f2f2}',
            Self::Lap | Self::LapsLeft => '\u{f1da}',
            Self::Track | Self::Riders => '\u{f553}',
            Self::Air => '\u{f72e}',
            Self::Best | Self::SessionBest => '\u{f2f2}',
            Self::GapAhead => '\u{f062}',
            Self::GapBehind => '\u{f063}',
            Self::SessionType => '\u{f11e}',
            Self::Fuel | Self::FuelPct => '\u{f52f}',
            Self::Setup => '\u{f0ad}',
        }
    }

    pub fn any(fields: &[Self; 3]) -> bool {
        fields.iter().any(|f| *f != Self::None)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DashField {
    None,
    Speed,
    Rpm,
    Gear,
    Position,
    Number,
    LapCount,
    LapsLeft,
    Last,
    Best,
    Current,
    Delta,
    Air,
    Engine,
    Gap,
    Interval,
    GapBehind,
    Penalty,
    Session,
    LocalTime,
    Bike,
    Class,
    Fuel,
    FuelPct,
    Setup,
}

impl DashField {
    pub const ALL: [Self; 25] = [
        Self::None,
        Self::Speed,
        Self::Rpm,
        Self::Gear,
        Self::Position,
        Self::Number,
        Self::LapCount,
        Self::LapsLeft,
        Self::Last,
        Self::Best,
        Self::Current,
        Self::Delta,
        Self::Air,
        Self::Engine,
        Self::Gap,
        Self::Interval,
        Self::GapBehind,
        Self::Penalty,
        Self::Session,
        Self::LocalTime,
        Self::Bike,
        Self::Class,
        Self::Fuel,
        Self::FuelPct,
        Self::Setup,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Speed => "speed",
            Self::Rpm => "rpm",
            Self::Gear => "gear",
            Self::Position => "pos",
            Self::Number => "num",
            Self::LapCount => "laps",
            Self::LapsLeft => "left",
            Self::Last => "last",
            Self::Best => "best",
            Self::Current => "cur",
            Self::Delta => "delta",
            Self::Air => "air",
            Self::Engine => "eng",
            Self::Gap => "gap",
            Self::Interval => "int",
            Self::GapBehind => "gapbehind",
            Self::Penalty => "pen",
            Self::Session => "sess",
            Self::LocalTime => "local",
            Self::Bike => "bike",
            Self::Class => "class",
            Self::Fuel => "fuel",
            Self::FuelPct => "fuelpct",
            Self::Setup => "setup",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Speed => "Speed",
            Self::Rpm => "RPM",
            Self::Gear => "Gear",
            Self::Position => "Position",
            Self::Number => "Bike number",
            Self::LapCount => "Lap count",
            Self::LapsLeft => "Laps left",
            Self::Last => "Last lap",
            Self::Best => "Best lap",
            Self::Current => "Current lap",
            Self::Delta => "Delta",
            Self::Air => "Air temp",
            Self::Engine => "Engine temp",
            Self::Gap => "Gap",
            Self::Interval => "Interval",
            Self::GapBehind => "Gap behind",
            Self::Penalty => "Penalty",
            Self::Session => "Session time",
            Self::LocalTime => "Local time",
            Self::Bike => "Bike",
            Self::Class => "Class",
            Self::Fuel => "Fuel",
            Self::FuelPct => "Fuel %",
            Self::Setup => "Setup",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim() {
            "none" => Self::None,
            "speed" => Self::Speed,
            "rpm" => Self::Rpm,
            "gear" => Self::Gear,
            "pos" | "position" => Self::Position,
            "num" | "number" => Self::Number,
            "laps" | "lapcount" => Self::LapCount,
            "left" | "lapsleft" => Self::LapsLeft,
            "last" => Self::Last,
            "best" => Self::Best,
            "cur" | "current" => Self::Current,
            "delta" => Self::Delta,
            "air" => Self::Air,
            "eng" | "engine" => Self::Engine,
            "gap" => Self::Gap,
            "int" | "interval" => Self::Interval,
            "gapbehind" | "gap_behind" | "behind" => Self::GapBehind,
            "pen" | "penalty" => Self::Penalty,
            "sess" | "session" => Self::Session,
            "local" | "localtime" => Self::LocalTime,
            "bike" => Self::Bike,
            "class" => Self::Class,
            "fuel" => Self::Fuel,
            "fuelpct" | "fuel%" | "fuelpercent" => Self::FuelPct,
            "setup" => Self::Setup,
            _ => Self::None,
        }
    }

    pub fn icon(self) -> char {
        match self {
            Self::None => '\0',
            Self::Speed => '\u{f3fd}',
            Self::Rpm => '\u{f3fd}',
            Self::Gear => '\u{f013}',
            Self::Position => '\u{f091}',
            Self::Number => '\u{f292}',
            Self::LapCount | Self::LapsLeft => '\u{f1da}',
            Self::Last | Self::Best | Self::Current | Self::Session | Self::LocalTime => '\u{f2f2}',
            Self::Delta => '\u{f362}',
            Self::Air => '\u{f72e}',
            Self::Engine => '\u{f2c9}',
            Self::Gap | Self::Interval => '\u{f062}',
            Self::GapBehind => '\u{f063}',
            Self::Penalty => '\u{f06a}',
            Self::Bike => '\u{f21c}',
            Self::Class => '\u{f0c0}',
            Self::Fuel | Self::FuelPct => '\u{f52f}',
            Self::Setup => '\u{f0ad}',
        }
    }
}

fn move_to<T>(items: &mut Vec<T>, from: usize, to: usize) {
    if from >= items.len() || to >= items.len() || from == to {
        return;
    }
    let item = items.remove(from);
    items.insert(to, item);
}

/// Untouched older factory placements pick up the compact in-game size (11.1%×11.5%).
fn migrate_default_dash(r: &mut Rect) {
    let untouched = |x: f32, y: f32, w: f32, h: f32| {
        (r.x - x).abs() < 0.001
            && (r.y - y).abs() < 0.001
            && (r.w - w).abs() < 0.001
            && (r.h - h).abs() < 0.001
    };
    let factory = Rect {
        x: 0.445,
        y: 0.865,
        w: 0.111,
        h: 0.115,
    };
    if untouched(0.41, 0.82, 0.18, 0.16)
        || untouched(0.43, 0.86, 0.14, 0.12)
        || untouched(0.43, 0.90, 0.14, 0.08)
        || untouched(0.43, 0.87, 0.14, 0.11)
        || untouched(0.43, 0.84, 0.14, 0.14)
        || untouched(0.41, 0.84, 0.18, 0.14)
        || untouched(0.442, 0.872, 0.115, 0.108)
    {
        *r = factory;
        return;
    }
    // Slot was narrower than the plaque, so orange handles sat on the visual
    // edge and missed the saved rect. Keep placement; use the visual width.
    if r.w < 0.09 && (r.h - 0.108).abs() < 0.01 {
        r.w = 0.111;
    }
}

fn migrate_default_flag(r: &mut Rect) {
    let factory = Rect {
        x: 0.447,
        y: 0.026,
        w: 0.107,
        h: 0.019,
    };
    let untouched = |x: f32, y: f32, w: f32, h: f32| {
        (r.x - x).abs() < 0.001
            && (r.y - y).abs() < 0.001
            && (r.w - w).abs() < 0.001
            && (r.h - h).abs() < 0.001
    };
    if untouched(0.442, 0.032, 0.116, 0.155)
        || untouched(0.34, 0.032, 0.32, 0.072)
        || untouched(0.414, 0.032, 0.172, 0.030)
    {
        *r = factory;
    }
}

fn migrate_default_sector(r: &mut Rect) {
    let factory = Rect {
        x: 0.62,
        y: 0.68,
        w: 0.36,
        h: 0.26,
    };
    let untouched = |x: f32, y: f32, w: f32, h: f32| {
        (r.x - x).abs() < 0.001
            && (r.y - y).abs() < 0.001
            && (r.w - w).abs() < 0.001
            && (r.h - h).abs() < 0.001
    };
    if untouched(0.66, 0.84, 0.32, 0.085)
        || untouched(0.66, 0.78, 0.32, 0.14)
        || untouched(0.66, 0.70, 0.32, 0.22)
        || untouched(0.62, 0.70, 0.36, 0.22)
    {
        *r = factory;
        return;
    }
    if r.w >= 0.28 && ((r.h - 0.085).abs() < 0.002 || (r.h - 0.14).abs() < 0.002) {
        let cy = r.y + r.h * 0.5;
        r.h = 0.22;
        r.y = (cy - r.h * 0.5).clamp(0.0, 1.0 - r.h);
    }
}

pub fn ini_path() -> PathBuf {
    #[cfg(test)]
    {
        if let Ok(p) = std::env::var("MXBO_TEST_INI") {
            if !p.is_empty() {
                return PathBuf::from(p);
            }
        }
    }
    let home = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\Public".into());
    PathBuf::from(home)
        .join("Documents")
        .join("PiBoSo")
        .join("MX Bikes")
        .join("Holeshot-HUD.ini")
}

fn legacy_ini_path() -> PathBuf {
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

/// Follow live session mode without writing the ini every frame.
pub fn sync_session_preset(preset: Option<SessionPreset>) {
    sync_session_preset_held(preset, false);
}

/// Same as [`sync_session_preset`], but keep the F8 edit slot when it is not the live one.
pub fn sync_session_preset_held(preset: Option<SessionPreset>, hold_settings: bool) {
    let mut g = CONFIG.lock().unwrap_or_else(|e| e.into_inner());
    g.sync_live(preset, hold_settings);
}

#[cfg(test)]
#[path = "tests/config.rs"]
mod tests;
