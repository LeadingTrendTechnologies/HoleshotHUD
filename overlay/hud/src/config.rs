use std::borrow::Cow;
use std::cell::Cell;
use std::fs;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::SystemTime;

use crate::pitboard::{encode_places, factory_pack, factory_places, parse_places, PitPlace, PitWhen, FACTORY_ART};
use crate::shm::{Rect, Snapshot};

pub static CONFIG: LazyLock<Mutex<HudConfig>> = LazyLock::new(|| Mutex::new(HudConfig::new()));

pub const COL_W_MIN: i32 = 18;
pub const COL_W_MAX: i32 = 160;
pub const NAME_W_MAX: i32 = 400;
pub const RADAR_RANGE_MIN: i32 = 6;
pub const RADAR_RANGE_MAX: i32 = 30;
pub const RADAR_RANGE_DEFAULT: i32 = 12;

/// Holeshot orange. Default primary for HUD, settings plaques, and Motos.
pub const DEFAULT_PRIMARY: [u8; 3] = [255, 148, 48];

/// Preset chips in Settings → Look → Primary color. First entry is the default.
pub const PRIMARY_SWATCHES: [[u8; 3]; 7] = [
    DEFAULT_PRIMARY,
    [239, 68, 68],
    [234, 179, 8],
    [74, 222, 128],
    [34, 211, 238],
    [167, 139, 250],
    [232, 121, 249],
];

thread_local! {
    static ACCENT_RGB: Cell<[u8; 3]> = const { Cell::new(DEFAULT_PRIMARY) };
    static SETTINGS_THEME_LIGHT: Cell<bool> = const { Cell::new(false) };
}

pub fn set_accent_rgb(rgb: [u8; 3]) {
    ACCENT_RGB.with(|c| c.set(rgb));
}

pub fn accent_rgb() -> [u8; 3] {
    ACCENT_RGB.with(|c| c.get())
}

/// Cached for paint paths that already hold the config lock (settings / Motos chrome).
pub fn set_settings_theme_light(light: bool) {
    SETTINGS_THEME_LIGHT.with(|c| c.set(light));
}

pub fn settings_theme_light() -> bool {
    SETTINGS_THEME_LIGHT.with(|c| c.get())
}

pub fn format_primary_color(rgb: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2])
}

pub fn parse_primary_color(s: &str) -> Option<[u8; 3]> {
    let hex = s.trim().strip_prefix('#').unwrap_or(s.trim());
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    Some([
        u8::from_str_radix(&hex[0..2], 16).ok()?,
        u8::from_str_radix(&hex[2..4], 16).ok()?,
        u8::from_str_radix(&hex[4..6], 16).ok()?,
    ])
}

/// Keep a custom game-ui image path only when the file still exists.
pub fn game_ui_image_path(s: &str) -> String {
    let p = s.trim();
    if p.is_empty() {
        return String::new();
    }
    if std::path::Path::new(p).is_file() {
        p.to_string()
    } else {
        String::new()
    }
}

/// Hue 0..360, saturation 0..1, value 0..1.
pub fn rgb_to_hsv(rgb: [u8; 3]) -> (f32, f32, f32) {
    let r = rgb[0] as f32 / 255.0;
    let g = rgb[1] as f32 / 255.0;
    let b = rgb[2] as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d < 1e-6 {
        0.0
    } else if (max - r).abs() < 1e-6 {
        60.0 * ((g - b) / d)
    } else if (max - g).abs() < 1e-6 {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    let h = if h < 0.0 { h + 360.0 } else { h };
    let s = if max < 1e-6 { 0.0 } else { d / max };
    (h, s, max)
}

pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [u8; 3] {
    let h = h.rem_euclid(360.0);
    let s = s.clamp(0.0, 1.0);
    let v = v.clamp(0.0, 1.0);
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());
    let m = v - c;
    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    [
        ((r + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((g + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((b + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

pub fn ink_on_rgb(rgb: [u8; 3]) -> [u8; 3] {
    let lum = 0.2126 * (rgb[0] as f32 / 255.0)
        + 0.7152 * (rgb[1] as f32 / 255.0)
        + 0.0722 * (rgb[2] as f32 / 255.0);
    if lum > 0.62 {
        [16, 16, 18]
    } else {
        [248, 248, 250]
    }
}

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
            Self::Segoe => Some((
                r"C:\Windows\Fonts\segoeui.ttf",
                r"C:\Windows\Fonts\segoeuib.ttf",
            )),
            Self::Arial => Some((
                r"C:\Windows\Fonts\arial.ttf",
                r"C:\Windows\Fonts\arialbd.ttf",
            )),
            Self::Tahoma => Some((
                r"C:\Windows\Fonts\tahoma.ttf",
                r"C:\Windows\Fonts\tahomabd.ttf",
            )),
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

/// F8 Settings window chrome. Does not retheme in-game HUD plaques.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SettingsTheme {
    Dark,
    Light,
}

impl SettingsTheme {
    pub const ALL: [Self; 2] = [Self::Dark, Self::Light];

    pub fn label(self) -> &'static str {
        match self {
            Self::Dark => "Dark",
            Self::Light => "Light",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "light" => Self::Light,
            _ => Self::Dark,
        }
    }

    pub fn is_dark(self) -> bool {
        matches!(self, Self::Dark)
    }
}

/// F8 Look language. `System` follows the Windows UI locale.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Language {
    System,
    En,
    Fr,
    It,
    Es,
    De,
    PtBr,
    Nl,
}

impl Language {
    pub const CHOICES: [Self; 8] = [
        Self::System,
        Self::En,
        Self::Fr,
        Self::It,
        Self::Es,
        Self::De,
        Self::PtBr,
        Self::Nl,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::En => "en",
            Self::Fr => "fr",
            Self::It => "it",
            Self::Es => "es",
            Self::De => "de",
            Self::PtBr => "pt-BR",
            Self::Nl => "nl",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "en" | "en-us" | "en-gb" => Self::En,
            "fr" | "fr-fr" | "fr-ca" => Self::Fr,
            "it" | "it-it" => Self::It,
            "es" | "es-es" | "es-mx" | "es-ar" => Self::Es,
            "de" | "de-de" | "de-at" | "de-ch" => Self::De,
            "pt-br" | "pt_br" | "pt" | "pt-pt" => Self::PtBr,
            "nl" | "nl-nl" | "nl-be" => Self::Nl,
            _ => Self::System,
        }
    }

    /// Map a Windows locale tag. Unknown tags stay English. `pt` and `pt-PT` use Brazilian Portuguese.
    pub fn from_locale(tag: &str) -> Self {
        let tag = tag.trim().trim_matches('\0').replace('_', "-");
        let primary = tag
            .split('-')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        match primary.as_str() {
            "en" => Self::En,
            "fr" => Self::Fr,
            "it" => Self::It,
            "es" => Self::Es,
            "de" => Self::De,
            "pt" => Self::PtBr,
            "nl" => Self::Nl,
            _ => Self::En,
        }
    }

    /// Stored choice, with `System` replaced by the resolved Windows language.
    pub fn resolved(self, system: Self) -> Self {
        match self {
            Self::System => match system {
                Self::System => Self::En,
                other => other,
            },
            other => other,
        }
    }

    /// Name of the language in that language. `System` is the English word; paint it with `t`.
    pub fn endonym(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::En => "English",
            Self::Fr => "Français",
            Self::It => "Italiano",
            Self::Es => "Español",
            Self::De => "Deutsch",
            Self::PtBr => "Português (Brasil)",
            Self::Nl => "Nederlands",
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

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum GamepadTheme {
    #[default]
    Light,
    Dark,
}

impl GamepadTheme {
    pub fn label(self) -> &'static str {
        match self {
            Self::Light => "Light",
            Self::Dark => "Dark",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "dark" => Self::Dark,
            _ => Self::Light,
        }
    }

    pub fn filled(self) -> bool {
        matches!(self, Self::Light)
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

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum RadarStyle {
    #[default]
    Plaque,
    Arrows,
}

impl RadarStyle {
    pub fn label(self) -> &'static str {
        match self {
            Self::Plaque => "Plaque",
            Self::Arrows => "Arrows",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Plaque => "plaque",
            Self::Arrows => "arrows",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "arrows" | "arrow" | "edge" => Self::Arrows,
            _ => Self::Plaque,
        }
    }
}

/// Near-fullscreen inset used when Arrows mode expands the default plaque rect.
pub const RADAR_ARROWS_RECT: Rect = Rect {
    x: 0.02,
    y: 0.02,
    w: 0.96,
    h: 0.96,
};

pub fn radar_default_plaque_rect() -> Rect {
    WidgetId::Radar.default_rect()
}

pub fn radar_rect_is_default_plaque(rect: Rect) -> bool {
    let d = radar_default_plaque_rect();
    (rect.x - d.x).abs() < 0.001
        && (rect.y - d.y).abs() < 0.001
        && (rect.w - d.w).abs() < 0.001
        && (rect.h - d.h).abs() < 0.001
}

/// Expand the Radar rect once when switching to Arrows if it is still the default plaque.
pub fn maybe_expand_radar_for_arrows(cfg: &mut HudConfig) {
    if cfg.radar.radar_style != RadarStyle::Arrows {
        return;
    }
    if radar_rect_is_default_plaque(cfg[WidgetId::Radar].rect) {
        cfg[WidgetId::Radar].rect = RADAR_ARROWS_RECT;
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
    Pitboard,
    Timer,
}

impl WidgetId {
    pub const ALL: [Self; 17] = [
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
        Self::Pitboard,
        Self::Timer,
    ];
    pub const COUNT: usize = 17;

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
            Self::Standings => Rect {
                x: 0.012,
                y: 0.03,
                w: 0.20,
                h: 0.46,
            },
            Self::Relative => Rect {
                x: 0.012,
                y: 0.62,
                w: 0.20,
                h: 0.36,
            },
            Self::Map => Rect {
                x: 0.79375,
                y: 0.62,
                w: 0.19125,
                h: 0.34,
            },
            Self::Minimap => Rect {
                x: 0.815,
                y: 0.035,
                w: 0.165,
                h: 0.295,
            },
            Self::Radar => Rect {
                x: 0.438,
                y: 0.755,
                w: 0.124,
                h: 0.22,
            },
            Self::Dash => Rect {
                x: 0.445,
                y: 0.865,
                w: 0.111,
                h: 0.115,
            },
            Self::Ticker => Rect {
                x: 0.06,
                y: 0.012,
                w: 0.88,
                h: 0.055,
            },
            Self::Sys => Rect {
                x: 0.012,
                y: 0.36,
                w: 0.086,
                h: 0.25,
            },
            Self::Sector => Rect {
                x: 0.62,
                y: 0.68,
                w: 0.36,
                h: 0.26,
            },
            Self::Delta => Rect {
                x: 0.36,
                y: 0.76,
                w: 0.28,
                h: 0.09,
            },
            Self::Stance => Rect {
                x: 0.445,
                y: 0.705,
                w: 0.11,
                h: 0.065,
            },
            Self::Flag => Rect {
                x: 0.447,
                y: 0.026,
                w: 0.107,
                h: 0.019,
            },
            Self::Lean => Rect {
                x: 0.318,
                y: 0.755,
                w: 0.11,
                h: 0.20,
            },
            Self::Gamepad => Rect {
                x: 0.415625,
                y: 0.76,
                w: 0.16875,
                h: 0.20,
            },
            Self::Telemetry => Rect {
                x: 0.22,
                y: 0.80,
                w: 0.56,
                h: 0.145,
            },
            Self::Pitboard => Rect {
                x: 0.41,
                y: 0.05,
                w: 0.18,
                h: 0.22,
            },
            Self::Timer => Rect {
                x: 0.40,
                y: 0.058,
                w: 0.0645,
                h: 0.0337,
            },
        }
    }

    fn default_bg(self) -> i32 {
        match self {
            Self::Standings | Self::Relative => 78,
            Self::Radar | Self::Ticker | Self::Stance | Self::Lean => 86,
            Self::Gamepad | Self::Map | Self::Minimap | Self::Delta => 0,
            Self::Dash | Self::Sys | Self::Sector | Self::Telemetry | Self::Timer => 82,
            Self::Pitboard => 100,
            Self::Flag => 100,
        }
    }

    /// Disk key names. Changing these breaks existing `Holeshot-HUD.ini` files.
    fn ini(self) -> WidgetIni {
        match self {
            Self::Standings => WidgetIni {
                rect: "standings",
                show: "show_standings",
                font: "st_font",
                bold: "st_bold",
                bg: "st_bg",
            },
            Self::Relative => WidgetIni {
                rect: "relative",
                show: "show_relative",
                font: "rel_font",
                bold: "rel_bold",
                bg: "rel_bg",
            },
            Self::Map => WidgetIni {
                rect: "map",
                show: "show_map",
                font: "map_font",
                bold: "map_bold",
                bg: "map_bg",
            },
            Self::Minimap => WidgetIni {
                rect: "minimap",
                show: "show_minimap",
                font: "mini_font",
                bold: "mini_bold",
                bg: "mini_bg",
            },
            Self::Radar => WidgetIni {
                rect: "radar",
                show: "show_radar",
                font: "radar_font",
                bold: "radar_bold",
                bg: "radar_bg",
            },
            Self::Dash => WidgetIni {
                rect: "dash",
                show: "show_dash",
                font: "dash_font",
                bold: "dash_bold",
                bg: "dash_bg",
            },
            Self::Ticker => WidgetIni {
                rect: "ticker",
                show: "show_ticker",
                font: "ticker_font",
                bold: "ticker_bold",
                bg: "ticker_bg",
            },
            Self::Sys => WidgetIni {
                rect: "sys",
                show: "show_sys",
                font: "sys_font",
                bold: "sys_bold",
                bg: "sys_bg",
            },
            Self::Sector => WidgetIni {
                rect: "sector",
                show: "show_sector",
                font: "sector_font",
                bold: "sector_bold",
                bg: "sector_bg",
            },
            Self::Delta => WidgetIni {
                rect: "delta",
                show: "show_delta",
                font: "delta_font",
                bold: "delta_bold",
                bg: "delta_bg",
            },
            Self::Stance => WidgetIni {
                rect: "stance",
                show: "show_stance",
                font: "stance_font",
                bold: "stance_bold",
                bg: "stance_bg",
            },
            Self::Flag => WidgetIni {
                rect: "flag",
                show: "show_flag",
                font: "flag_font",
                bold: "flag_bold",
                bg: "flag_bg",
            },
            Self::Lean => WidgetIni {
                rect: "lean",
                show: "show_lean",
                font: "lean_font",
                bold: "lean_bold",
                bg: "lean_bg",
            },
            Self::Gamepad => WidgetIni {
                rect: "gamepad",
                show: "show_gamepad",
                font: "gamepad_font",
                bold: "gamepad_bold",
                bg: "gamepad_bg",
            },
            Self::Telemetry => WidgetIni {
                rect: "telemetry",
                show: "show_telemetry",
                font: "telemetry_font",
                bold: "telemetry_bold",
                bg: "telemetry_bg",
            },
            Self::Pitboard => WidgetIni {
                rect: "pitboard",
                show: "show_pitboard",
                font: "pit_font",
                bold: "pit_bold",
                bg: "pit_bg",
            },
            Self::Timer => WidgetIni {
                rect: "timer",
                show: "show_timer",
                font: "timer_font",
                bold: "timer_bold",
                bg: "timer_bg",
            },
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

/// F8 Widgets: edit the in-game board or the OBS stream board. Runtime only — not saved.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum EditSurface {
    #[default]
    Game,
    Stream,
}

impl EditSurface {
    pub fn label(self) -> &'static str {
        match self {
            Self::Game => "Game",
            Self::Stream => "Stream",
        }
    }
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
    LapDiff,
    Status,
    Bike,
    Penalty,
    Crashed,
    Category,
}

impl StField {
    pub const ALL: [Self; 15] = [
        Self::Pos,
        Self::Num,
        Self::Name,
        Self::Gap,
        Self::Interval,
        Self::Laps,
        Self::Current,
        Self::Best,
        Self::Last,
        Self::LapDiff,
        Self::Status,
        Self::Bike,
        Self::Penalty,
        Self::Crashed,
        Self::Category,
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
            Self::LapDiff => "lapdiff",
            Self::Status => "status",
            Self::Bike => "bike",
            Self::Penalty => "pen",
            Self::Crashed => "crash",
            Self::Category => "cat",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Pos => "Position",
            Self::Num => "Number",
            Self::Name => "Name",
            Self::Gap => "Gap to leader",
            Self::Interval => "Gap to rider ahead",
            Self::Laps => "Completed Laps",
            Self::Current => "Current lap",
            Self::Best => "Fastest",
            Self::Last => "Last lap",
            Self::LapDiff => "Last lap diff",
            Self::Status => "Status",
            Self::Bike => "Bike",
            Self::Penalty => "Penalty",
            Self::Crashed => "Crashed",
            Self::Category => "Category",
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
            "lapdiff" | "lap_diff" => Self::LapDiff,
            "status" => Self::Status,
            "bike" => Self::Bike,
            "pen" | "penalty" => Self::Penalty,
            "crash" | "crashed" => Self::Crashed,
            "cat" | "category" | "class" => Self::Category,
            _ => return None,
        })
    }

    pub fn enabled(self, c: &HudConfig) -> bool {
        match self {
            Self::Pos => c.standings.st_pos,
            Self::Num => c.standings.st_num,
            Self::Name => c.standings.st_name,
            Self::Gap => c.standings.st_gap,
            Self::Interval => c.standings.st_interval,
            Self::Laps => c.standings.st_laps,
            Self::Current => c.standings.st_current,
            Self::Best => c.standings.st_best,
            Self::Last => c.standings.st_last,
            Self::LapDiff => c.standings.st_lapdiff,
            Self::Status => c.standings.st_status,
            Self::Bike => c.standings.st_bike,
            Self::Penalty => c.standings.st_penalty,
            Self::Crashed => c.standings.st_crashed,
            Self::Category => c.standings.st_category,
        }
    }

    pub fn width(self, c: &HudConfig) -> i32 {
        match self {
            Self::Pos => c.standings.st_w_pos,
            Self::Num => c.standings.st_w_num,
            Self::Name => c.standings.st_w_name,
            Self::Gap => c.standings.st_w_gap,
            Self::Interval => c.standings.st_w_interval,
            Self::Laps => c.standings.st_w_laps,
            Self::Current => c.standings.st_w_current,
            Self::Best => c.standings.st_w_best,
            Self::Last => c.standings.st_w_last,
            Self::LapDiff => c.standings.st_w_lapdiff,
            Self::Status => c.standings.st_w_status,
            Self::Bike => c.standings.st_w_bike,
            Self::Penalty => c.standings.st_w_penalty,
            Self::Crashed => c.standings.st_w_crashed,
            Self::Category => c.standings.st_w_category,
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
            Self::Pos => c.standings.st_w_pos = next,
            Self::Num => c.standings.st_w_num = next,
            Self::Name => c.standings.st_w_name = next,
            Self::Gap => c.standings.st_w_gap = next,
            Self::Interval => c.standings.st_w_interval = next,
            Self::Laps => c.standings.st_w_laps = next,
            Self::Current => c.standings.st_w_current = next,
            Self::Best => c.standings.st_w_best = next,
            Self::Last => c.standings.st_w_last = next,
            Self::LapDiff => c.standings.st_w_lapdiff = next,
            Self::Status => c.standings.st_w_status = next,
            Self::Bike => c.standings.st_w_bike = next,
            Self::Penalty => c.standings.st_w_penalty = next,
            Self::Crashed => c.standings.st_w_crashed = next,
            Self::Category => c.standings.st_w_category = next,
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
    Status,
    Best,
    Last,
    LapDiff,
    Category,
    Speed,
}

impl RelField {
    pub const ALL: [Self; 15] = [
        Self::Num,
        Self::Name,
        Self::Gap,
        Self::Laps,
        Self::Current,
        Self::Pos,
        Self::Bike,
        Self::Penalty,
        Self::Interval,
        Self::Status,
        Self::Best,
        Self::Last,
        Self::LapDiff,
        Self::Category,
        Self::Speed,
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
            Self::Status => "status",
            Self::Best => "best",
            Self::Last => "last",
            Self::LapDiff => "lapdiff",
            Self::Category => "cat",
            Self::Speed => "speed",
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
            Self::Status => "Status",
            Self::Best => "Fastest",
            Self::Last => "Last lap",
            Self::LapDiff => "Last lap diff",
            Self::Category => "Category",
            Self::Speed => "Speed",
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
            "status" => Self::Status,
            "best" => Self::Best,
            "last" => Self::Last,
            "lapdiff" | "lap_diff" => Self::LapDiff,
            "cat" | "category" | "class" => Self::Category,
            "speed" => Self::Speed,
            _ => return None,
        })
    }

    pub fn enabled(self, c: &HudConfig) -> bool {
        match self {
            Self::Num => c.relative.rel_num,
            Self::Name => c.relative.rel_name,
            Self::Gap => c.relative.rel_gap,
            Self::Laps => c.relative.rel_laps,
            Self::Current => c.relative.rel_current,
            Self::Pos => c.relative.rel_pos,
            Self::Bike => c.relative.rel_bike,
            Self::Penalty => c.relative.rel_penalty,
            Self::Interval => c.relative.rel_interval,
            Self::Status => c.relative.rel_status,
            Self::Best => c.relative.rel_best,
            Self::Last => c.relative.rel_last,
            Self::LapDiff => c.relative.rel_lapdiff,
            Self::Category => c.relative.rel_category,
            Self::Speed => c.relative.rel_speed,
        }
    }

    pub fn width(self, c: &HudConfig) -> i32 {
        match self {
            Self::Num => c.relative.rel_w_num,
            Self::Name => c.relative.rel_w_name,
            Self::Gap => c.relative.rel_w_gap,
            Self::Laps => c.relative.rel_w_laps,
            Self::Current => c.relative.rel_w_current,
            Self::Pos => c.relative.rel_w_pos,
            Self::Bike => c.relative.rel_w_bike,
            Self::Penalty => c.relative.rel_w_penalty,
            Self::Interval => c.relative.rel_w_interval,
            Self::Status => c.relative.rel_w_status,
            Self::Best => c.relative.rel_w_best,
            Self::Last => c.relative.rel_w_last,
            Self::LapDiff => c.relative.rel_w_lapdiff,
            Self::Category => c.relative.rel_w_category,
            Self::Speed => c.relative.rel_w_speed,
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
            Self::Num => c.relative.rel_w_num = next,
            Self::Name => c.relative.rel_w_name = next,
            Self::Gap => c.relative.rel_w_gap = next,
            Self::Laps => c.relative.rel_w_laps = next,
            Self::Current => c.relative.rel_w_current = next,
            Self::Pos => c.relative.rel_w_pos = next,
            Self::Bike => c.relative.rel_w_bike = next,
            Self::Penalty => c.relative.rel_w_penalty = next,
            Self::Interval => c.relative.rel_w_interval = next,
            Self::Status => c.relative.rel_w_status = next,
            Self::Best => c.relative.rel_w_best = next,
            Self::Last => c.relative.rel_w_last = next,
            Self::LapDiff => c.relative.rel_w_lapdiff = next,
            Self::Category => c.relative.rel_w_category = next,
            Self::Speed => c.relative.rel_w_speed = next,
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
    SysPreset {
        key: "hud",
        label: "HUD",
        names: &[],
        kind: SysAppKind::Hud,
    },
    SysPreset {
        key: "mxbikes",
        label: "MX Bikes",
        names: &["mxbikes.exe"],
        kind: SysAppKind::Mxbikes,
    },
    SysPreset {
        key: "mxbapp",
        label: "MXB App",
        names: &[
            "frost.exe",
            "mxb app.exe",
            "mxb-app.exe",
            "mxbapp.exe",
            "frostmod.exe",
        ],
        kind: SysAppKind::MxbApp,
    },
    SysPreset {
        key: "reshade",
        label: "ReShade",
        names: &[],
        kind: SysAppKind::Reshade,
    },
    SysPreset {
        key: "obs",
        label: "OBS",
        names: &["obs64.exe", "obs32.exe"],
        kind: SysAppKind::Exe,
    },
    SysPreset {
        key: "discord",
        label: "Discord",
        names: &["discord.exe"],
        kind: SysAppKind::Exe,
    },
    SysPreset {
        key: "steam",
        label: "Steam",
        names: &["steam.exe"],
        kind: SysAppKind::Exe,
    },
    SysPreset {
        key: "nvidia",
        label: "NVIDIA",
        names: &["nvidia overlay.exe", "nvidia app.exe", "nvidia share.exe"],
        kind: SysAppKind::Exe,
    },
    SysPreset {
        key: "afterburner",
        label: "Afterburner",
        names: &["msiafterburner.exe"],
        kind: SysAppKind::Exe,
    },
    SysPreset {
        key: "rtss",
        label: "RTSS",
        names: &["rtss.exe"],
        kind: SysAppKind::Exe,
    },
    SysPreset {
        key: "medal",
        label: "Medal",
        names: &["medal.exe"],
        kind: SysAppKind::Exe,
    },
    SysPreset {
        key: "spotify",
        label: "Spotify",
        names: &["spotify.exe"],
        kind: SysAppKind::Exe,
    },
    SysPreset {
        key: "gamebar",
        label: "Game Bar",
        names: &["gamebar.exe"],
        kind: SysAppKind::Exe,
    },
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
        if label.is_empty() || !name.ends_with(".exe") || name.contains('\\') || name.contains('/')
        {
            return None;
        }
        if let Some(p) = SYS_PRESETS
            .iter()
            .find(|p| p.names.iter().any(|n| *n == name))
        {
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
pub struct StandingsLayout {
    pub standings_rows: i32,
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
    pub st_category: bool,
    pub st_lapdiff: bool,
    pub st_hl: i32,
    pub st_text: TableText,
    pub st_stripe: bool,
    /// Ink on the orange rider-count / track-name plaques. Default black.
    pub st_plaque_text: TableText,
    /// Draw the orange rider-count / track-name plaques. Default on.
    pub st_plaque: bool,
    pub st_head: [BoardField; 3],
    pub st_foot: [BoardField; 3],
    pub st_order: Vec<StField>,
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
    pub st_w_category: i32,
    pub st_w_lapdiff: i32,
}

#[derive(Clone)]
pub struct RelativeLayout {
    pub relative_count: i32,
    pub rel_num: bool,
    pub rel_name: bool,
    pub rel_gap: bool,
    pub rel_laps: bool,
    pub rel_current: bool,
    pub rel_pos: bool,
    pub rel_bike: bool,
    pub rel_penalty: bool,
    pub rel_interval: bool,
    pub rel_status: bool,
    pub rel_best: bool,
    pub rel_last: bool,
    pub rel_category: bool,
    pub rel_speed: bool,
    pub rel_lapdiff: bool,
    pub rel_hl: i32,
    pub rel_text: TableText,
    pub rel_stripe: bool,
    /// Ink on the orange rider-count / track-name plaques. Default black.
    pub rel_plaque_text: TableText,
    /// Draw the orange rider-count / track-name plaques. Default on.
    pub rel_plaque: bool,
    pub rel_head: [BoardField; 3],
    pub rel_foot: [BoardField; 3],
    pub rel_order: Vec<RelField>,
    pub rel_w_num: i32,
    pub rel_w_name: i32,
    pub rel_w_gap: i32,
    pub rel_w_laps: i32,
    pub rel_w_current: i32,
    pub rel_w_pos: i32,
    pub rel_w_bike: i32,
    pub rel_w_penalty: i32,
    pub rel_w_interval: i32,
    pub rel_w_status: i32,
    pub rel_w_best: i32,
    pub rel_w_last: i32,
    pub rel_w_category: i32,
    pub rel_w_speed: i32,
    pub rel_w_lapdiff: i32,
}

#[derive(Clone)]
pub struct TickerLayout {
    pub ticker_count: i32,
    pub ticker_title: bool,
    pub ticker_autoscroll: bool,
    pub ticker_status: bool,
    pub ticker_slide: bool,
    pub ticker_hl: i32,
    pub ticker_left: BoardField,
    pub ticker_right: BoardField,
}

#[derive(Clone)]
pub struct MapLayout {
    pub map_others: bool,
    pub map_sf: bool,
    pub map_sectors: bool,
    pub map_name: bool,
    pub map_numbers: bool,
    pub map_arrows: bool,
    /// Heading-up map: you stay centered, nose up. Off by default.
    pub map_follow: bool,
    /// Follow-me zoom percent. 0 fits the whole track. 100 shows 80 m across. Ignored unless Follow me is on.
    pub map_zoom: i32,
    pub map_crown: bool,
    pub map_place: bool,
    pub map_dot: DotLabel,
    /// Number color on your map dot only.
    pub map_you_text: TableText,
    /// 0–100. 78 matches the built-in dot wash. 100 is solid.
    pub map_dot_opacity: i32,
}

/// Default map dot opacity. `base * pct / 78` reproduces today's wash, halo, and ring.
pub const MAP_DOT_OPACITY_DEFAULT: i32 = 78;

#[derive(Clone)]
pub struct MiniLayout {
    pub mini_others: bool,
    pub mini_sf: bool,
    pub mini_sectors: bool,
    pub mini_numbers: bool,
    pub mini_arrows: bool,
    pub mini_crown: bool,
    pub mini_place: bool,
    pub mini_dot: DotLabel,
    pub mini_zoom: i32,
}

#[derive(Clone)]
pub struct RadarLayout {
    pub radar_sides: bool,
    pub radar_rear: bool,
    pub radar_rings: bool,
    /// Plaque (range arcs + blips) or Arrows (edge indicators on the widget frame).
    pub radar_style: RadarStyle,
    /// How far beside and behind, in meters, the radar shows other riders.
    /// Rings stay at 3 / 6 / 12 m; a longer range places dots outside the 12 m ring.
    pub radar_range: i32,
}

#[derive(Clone)]
pub struct TimerLayout {
    /// Place reads `P3/12`. Off by default. Missing ini key stays off.
    pub timer_of: bool,
}

#[derive(Clone)]
pub struct DashLayout {
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
}

#[derive(Clone)]
pub struct FlagLayout {
    /// Nearby crash. Flags widget only. Off by default. Dash has `dash_yellow`.
    pub flag_yellow: bool,
    /// Someone a lap up closing from behind. Flags widget only. Off by default. Dash has `dash_blue`.
    pub flag_blue: bool,
    /// Someone you are coming to lap. Flags widget only. Off by default. Dash has `dash_red`.
    pub flag_red: bool,
    /// Caption on the Flags cloth (WHITE FLAG / …). On by default.
    pub flag_text: bool,
}

#[derive(Clone)]
pub struct SectorLayout {
    /// Tick the current sector live. Off: times only after the split.
    pub sector_live: bool,
    /// Compare Sectors to this session's best splits instead of the saved tape.
    pub sector_session: bool,
    /// Draw LAST / -2 / … under the live strip.
    pub sector_hist: bool,
    /// How many completed laps the underboard shows (1–5).
    pub sector_hist_laps: i32,
}

#[derive(Clone)]
pub struct DeltaLayout {
    /// Compare Delta Bar to this session's fastest decent lap instead of the saved tape.
    pub delta_session: bool,
}

#[derive(Clone)]
pub struct StanceLayout {
    pub stance_mode: StanceMode,
    pub stance_style: StanceStyle,
    pub stance_show_sit: bool,
}

#[derive(Clone)]
pub struct LeanLayout {
    pub lean_style: LeanStyle,
}

#[derive(Clone)]
pub struct GamepadLayout {
    pub gamepad_style: GamepadStyle,
    pub gamepad_theme: GamepadTheme,
}

#[derive(Clone)]
pub struct TelemetryLayout {
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
}

#[derive(Clone)]
pub struct PitLayout {
    pub pit_sponsor: String,
    pub pit_art: String,
    /// Saved board folder under `pitboards`. Empty is the Holeshot plate.
    pub pit_board: String,
    pub pit_text: TableText,
    pub pit_vars: Vec<PitPlace>,
    /// Always, or flash 5s at each sector end / lap start.
    pub pit_when: PitWhen,
    /// Factory plate Main. Starts as the accent. Ignored for a browsed PNG.
    pub pit_yellow: [u8; 3],
    /// Factory plate Secondary. The word follows this. Starts black.
    pub pit_blue: [u8; 3],
}

#[derive(Clone)]
pub struct SysLayout {
    /// Process rows on Systems. Built-ins stay; extras can be added and removed.
    pub sys_apps: Vec<SysApp>,
}

#[derive(Clone)]
pub struct HudLayout {
    widgets: [WidgetPrefs; WidgetId::COUNT],
    pub standings: StandingsLayout,
    pub relative: RelativeLayout,
    pub ticker: TickerLayout,
    pub map: MapLayout,
    pub mini: MiniLayout,
    pub radar: RadarLayout,
    pub dash: DashLayout,
    pub flag: FlagLayout,
    pub sector: SectorLayout,
    pub delta: DeltaLayout,
    pub stance: StanceLayout,
    pub lean: LeanLayout,
    pub gamepad: GamepadLayout,
    pub telemetry: TelemetryLayout,
    pub pit: PitLayout,
    pub sys: SysLayout,
    pub timer: TimerLayout,
}

impl HudLayout {
    pub fn new() -> Self {
        Self {
            widgets: std::array::from_fn(|i| WidgetId::ALL[i].default_prefs()),
            standings: StandingsLayout {
                standings_rows: 12,
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
                st_category: false,
                st_lapdiff: false,
                st_hl: 50,
                st_text: TableText::White,
                st_stripe: true,
                st_plaque_text: TableText::Black,
                st_plaque: true,
                st_head: BoardField::DEFAULT_HEAD,
                st_foot: BoardField::DEFAULT_FOOT,
                st_order: StField::ALL.to_vec(),
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
                st_w_category: 48,
                st_w_lapdiff: 58,
            },
            relative: RelativeLayout {
                relative_count: 3,
                rel_num: true,
                rel_name: true,
                rel_gap: true,
                rel_laps: false,
                rel_current: false,
                rel_pos: false,
                rel_bike: false,
                rel_penalty: false,
                rel_interval: false,
                rel_status: false,
                rel_best: true,
                rel_last: true,
                rel_category: false,
                rel_speed: false,
                rel_lapdiff: false,
                rel_hl: 50,
                rel_text: TableText::White,
                rel_stripe: true,
                rel_plaque_text: TableText::Black,
                rel_plaque: true,
                rel_head: BoardField::DEFAULT_HEAD,
                rel_foot: BoardField::DEFAULT_FOOT,
                rel_order: RelField::ALL.to_vec(),
                rel_w_num: 32,
                rel_w_name: 80,
                rel_w_gap: 58,
                rel_w_laps: 90,
                rel_w_current: 72,
                rel_w_pos: 28,
                rel_w_bike: 56,
                rel_w_penalty: 48,
                rel_w_interval: 58,
                rel_w_status: 40,
                rel_w_best: 54,
                rel_w_last: 54,
                rel_w_category: 48,
                rel_w_speed: 56,
                rel_w_lapdiff: 58,
            },
            ticker: TickerLayout {
                ticker_count: 7,
                ticker_title: true,
                ticker_autoscroll: false,
                ticker_status: false,
                ticker_slide: true,
                ticker_hl: 50,
                ticker_left: BoardField::Lap,
                ticker_right: BoardField::Air,
            },
            map: MapLayout {
                map_others: true,
                map_sf: true,
                map_sectors: true,
                map_name: true,
                map_numbers: true,
                map_arrows: true,
                map_follow: false,
                map_zoom: 0,
                map_crown: true,
                map_place: true,
                map_dot: DotLabel::Position,
                map_you_text: TableText::Black,
                map_dot_opacity: MAP_DOT_OPACITY_DEFAULT,
            },
            mini: MiniLayout {
                mini_others: true,
                mini_sf: true,
                mini_sectors: true,
                mini_numbers: true,
                mini_arrows: true,
                mini_crown: true,
                mini_place: true,
                mini_dot: DotLabel::Number,
                mini_zoom: 70,
            },
            radar: RadarLayout {
                radar_sides: true,
                radar_rear: true,
                radar_rings: true,
                radar_style: RadarStyle::Plaque,
                radar_range: RADAR_RANGE_DEFAULT,
            },
            dash: DashLayout {
                dash_rev: true,
                dash_yellow: false,
                dash_blue: false,
                dash_red: false,
                dash_simple: false,
                dash_shift_color: false,
                dash_left: DashField::Engine,
                dash_mid: DashField::Air,
                dash_right: DashField::Best,
            },
            flag: FlagLayout {
                flag_yellow: false,
                flag_blue: false,
                flag_red: false,
                flag_text: true,
            },
            sector: SectorLayout {
                sector_live: true,
                sector_session: false,
                sector_hist: true,
                sector_hist_laps: 3,
            },
            delta: DeltaLayout {
                delta_session: false,
            },
            stance: StanceLayout {
                stance_mode: StanceMode::Toggle,
                stance_style: StanceStyle::Text,
                stance_show_sit: false,
            },
            lean: LeanLayout {
                lean_style: LeanStyle::Figure,
            },
            gamepad: GamepadLayout {
                gamepad_style: GamepadStyle::Auto,
                gamepad_theme: GamepadTheme::Light,
            },
            telemetry: TelemetryLayout {
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
            },
            pit: PitLayout {
                pit_sponsor: factory_pack().1,
                pit_art: FACTORY_ART.into(),
                pit_board: String::new(),
                pit_text: factory_pack().0,
                pit_vars: factory_places(),
                pit_when: PitWhen::Always,
                pit_yellow: DEFAULT_PRIMARY,
                pit_blue: [0, 0, 0],
            },
            sys: SysLayout {
                sys_apps: default_sys_apps(),
            },
            timer: TimerLayout { timer_of: false },
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
        migrate_default_map(&mut self[WidgetId::Map].rect);
        migrate_default_gamepad(&mut self[WidgetId::Gamepad].rect);
        migrate_default_pitboard(&mut self[WidgetId::Pitboard].rect);
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

pub const GROUP_MAX: usize = 24;
pub const GROUP_MEMBER_MAX: usize = 64;
pub const GROUP_NAME_MAX: usize = 31;

/// Font Awesome solid glyphs for a group mark. Order is the settings icon row.
pub const GROUP_ICONS: [char; 10] = [
    '\u{f024}', // flag
    '\u{f005}', // star
    '\u{f132}', // shield
    '\u{f807}', // helmet
    '\u{f0e7}', // bolt
    '\u{f521}', // crown
    '\u{f111}', // circle
    '\u{f3a5}', // gem
    '\u{f0c0}', // friends
    '\u{f091}', // ranked
];

/// Swatches on the Groups page. Orange is the Holeshot accent.
pub const GROUP_COLORS: [[u8; 3]; 6] = [
    [0xFF, 0x94, 0x30],
    [0x3B, 0x82, 0xF6],
    [0x30, 0xDC, 0x58],
    [0xFF, 0x40, 0x48],
    [0xC4, 0x70, 0xFF],
    [0xE4, 0xE4, 0xE6],
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RiderGroup {
    pub name: String,
    pub icon: char,
    pub color: [u8; 3],
    pub members: Vec<String>,
}

impl RiderGroup {
    pub fn new(name: &str) -> Self {
        Self {
            name: clamp_group_name(name),
            icon: GROUP_ICONS[0],
            color: GROUP_COLORS[0],
            members: Vec::new(),
        }
    }
}

pub fn rider_key(name: &str) -> String {
    clamp_group_name(name).to_lowercase()
}

pub fn clamp_group_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|ch| if ch == '\t' || ch == '\n' || ch == '\r' { ' ' } else { ch })
        .collect();
    cleaned.trim().chars().take(GROUP_NAME_MAX).collect()
}

/// First group in list order that contains this display name.
pub fn group_badge(groups: &[RiderGroup], name: &str) -> Option<(char, [u8; 3])> {
    let key = rider_key(name);
    if key.is_empty() {
        return None;
    }
    groups.iter().find_map(|group| {
        group
            .members
            .iter()
            .any(|member| rider_key(member) == key)
            .then_some((group.icon, group.color))
    })
}

fn groups_section(groups: &[RiderGroup], on_maps: bool) -> String {
    let mut body = format!("\n[Groups]\non_maps={}\n", ini_flag(on_maps));
    for (index, group) in groups.iter().enumerate().take(GROUP_MAX) {
        body.push_str(&format!("{index}={}\n", encode_group(group)));
    }
    body
}

fn encode_group(group: &RiderGroup) -> String {
    let mut parts = vec![
        clamp_group_name(&group.name),
        format!("{:04X}", group.icon as u32),
        format_primary_color(group.color).trim_start_matches('#').to_string(),
    ];
    for member in group.members.iter().take(GROUP_MEMBER_MAX) {
        let member = clamp_group_name(member);
        if !member.is_empty() {
            parts.push(member);
        }
    }
    parts.join("\t")
}

fn parse_group_record(val: &str) -> Option<RiderGroup> {
    let mut parts = val.split('\t');
    let name = clamp_group_name(parts.next().unwrap_or(""));
    if name.is_empty() {
        return None;
    }
    let icon = parts
        .next()
        .and_then(|hex| u32::from_str_radix(hex.trim(), 16).ok())
        .and_then(char::from_u32)
        .filter(|ch| GROUP_ICONS.contains(ch))
        .unwrap_or(GROUP_ICONS[0]);
    let color = parts
        .next()
        .and_then(parse_primary_color)
        .unwrap_or(GROUP_COLORS[0]);
    let mut members: Vec<String> = Vec::new();
    for part in parts {
        let member = clamp_group_name(part);
        if member.is_empty() || members.len() >= GROUP_MEMBER_MAX {
            continue;
        }
        let key = rider_key(&member);
        if members.iter().any(|have| rider_key(have) == key) {
            continue;
        }
        members.push(member);
    }
    Some(RiderGroup {
        name,
        icon,
        color,
        members,
    })
}

#[derive(Clone)]
pub struct HudConfig {
    layouts: [HudLayout; SessionPreset::COUNT],
    /// Parallel stream boards (OBS Browser Source). Same preset index as `layouts`.
    stream_layouts: [HudLayout; SessionPreset::COUNT],
    pub active_preset: SessionPreset,
    pub settings_preset: SessionPreset,
    /// True while a session is live. The HUD uses `active_preset`; F8 can edit another slot.
    pub session_live: bool,
    /// F8 Game / Stream surface. Not written to the ini.
    pub edit_surface: EditSurface,
    /// Settings → Labs → Experimental features. Gates Profile → Tracks.
    pub experimental: bool,
    /// F8 Motos records motos only when this is on. Default off; missing ini key stays off.
    pub review: bool,
    /// Plugin-only: when true the frozen in-game HUD draws standings / relative / map.
    /// Overlay still saves this key. The plugin never writes the ini.
    pub ingame_hud: bool,
    /// F8 Look: install the Floating F8 MX Bikes menu pack into the game `ui/` folder.
    pub game_ui: bool,
    /// When true, MX Bikes menus use Look `primary`. When false, use `game_ui_primary`.
    pub game_ui_match_primary: bool,
    /// Menu pack accent when `game_ui_match_primary` is false.
    pub game_ui_primary: [u8; 3],
    /// Custom opening splash (`splash.tga`). Empty = bundled night-ink default.
    pub game_ui_splash_path: String,
    /// Custom loading background (`bkgrnd.tga`). Empty = bundled night-ink default.
    pub game_ui_loading_path: String,
    pub font_family: FontFamily,
    /// HUD / settings / Motos accent. Default Holeshot orange.
    pub primary: [u8; 3],
    pub units: UnitPrefs,
    pub start_with_windows: bool,
    pub minimize_on_close: bool,
    pub close_with_game: bool,
    pub open_with_game: bool,
    /// F8 Settings → Stream: localhost Browser Source server.
    pub stream_enabled: bool,
    /// Last bound localhost port for the stream page (0 = use default).
    pub stream_port: u16,
    pub auto_update_on_launch: bool,
    /// Last version whose What's new modal was dismissed with Got it.
    pub whats_new_seen: String,
    /// Overlay version on the first launch that wrote this settings file.
    /// `"unknown"` if they already had settings before this field existed.
    pub first_install_version: String,
    pub settings_key: SettingsKey,
    /// F8 Settings chrome theme. Dark is the default charcoal look.
    pub settings_theme: SettingsTheme,
    /// F8 Look. `System` follows the Windows UI locale.
    pub language: Language,
    /// Settings host origin in virtual-screen pixels. A second monitor can be `x >= primary` or negative.
    pub settings_x: i32,
    pub settings_y: i32,
    pub stance_bind: StanceBind,
    /// Rider groups. Shared across presets. List order picks the icon when a rider is in more than one.
    pub groups: Vec<RiderGroup>,
    /// When on, other riders on Map and Minimap use their group color. The orange you-dot stays orange.
    pub groups_on_maps: bool,
    loaded_mtime: Option<SystemTime>,
}

impl HudConfig {
    pub fn new() -> Self {
        let layout = HudLayout::new();
        let stream = HudLayout::new();
        Self {
            layouts: [layout.clone(), layout.clone(), layout.clone(), layout],
            stream_layouts: [stream.clone(), stream.clone(), stream.clone(), stream],
            active_preset: SessionPreset::Race,
            settings_preset: SessionPreset::Race,
            session_live: false,
            edit_surface: EditSurface::Game,
            experimental: false,
            review: false,
            ingame_hud: false,
            game_ui: false,
            game_ui_match_primary: true,
            game_ui_primary: DEFAULT_PRIMARY,
            game_ui_splash_path: String::new(),
            game_ui_loading_path: String::new(),
            font_family: FontFamily::Exo2,
            primary: DEFAULT_PRIMARY,
            units: UnitPrefs::all(Units::Metric),
            start_with_windows: false,
            minimize_on_close: false,
            close_with_game: false,
            open_with_game: false,
            stream_enabled: false,
            stream_port: 0,
            auto_update_on_launch: false,
            whats_new_seen: String::new(),
            first_install_version: String::new(),
            settings_key: SettingsKey::F8,
            settings_theme: SettingsTheme::Dark,
            language: Language::System,
            settings_x: 80,
            settings_y: 80,
            stance_bind: StanceBind::PadRb,
            groups: Vec::new(),
            groups_on_maps: false,
            loaded_mtime: None,
        }
    }

    pub fn live(&self) -> &HudLayout {
        &self.layouts[self.active_preset.idx()]
    }

    pub fn live_mut(&mut self) -> &mut HudLayout {
        &mut self.layouts[self.active_preset.idx()]
    }

    pub fn stream_live(&self) -> &HudLayout {
        &self.stream_layouts[self.active_preset.idx()]
    }

    pub fn stream_live_mut(&mut self) -> &mut HudLayout {
        &mut self.stream_layouts[self.active_preset.idx()]
    }


    pub fn edit(&self) -> &HudLayout {
        match self.edit_surface {
            EditSurface::Game => &self.layouts[self.settings_preset.idx()],
            EditSurface::Stream => &self.stream_layouts[self.settings_preset.idx()],
        }
    }

    pub fn edit_mut(&mut self) -> &mut HudLayout {
        match self.edit_surface {
            EditSurface::Game => &mut self.layouts[self.settings_preset.idx()],
            EditSurface::Stream => &mut self.stream_layouts[self.settings_preset.idx()],
        }
    }

    pub fn stream_edit(&self) -> &HudLayout {
        &self.stream_layouts[self.settings_preset.idx()]
    }

    pub fn stream_edit_mut(&mut self) -> &mut HudLayout {
        &mut self.stream_layouts[self.settings_preset.idx()]
    }

    /// Clones all eight layouts. Release timing: about 18µs per call (7.4KB).
    /// Skia owns the frame, so this stays a clone.
    pub fn for_overlay(&self) -> Self {
        let mut c = self.clone();
        c.settings_preset = c.active_preset;
        c.edit_surface = EditSurface::Game;
        c
    }

    /// Browser Source paint: live stream board for `active_preset`.
    pub fn for_stream(&self) -> Self {
        let mut c = self.clone();
        c.layouts = c.stream_layouts.clone();
        c.settings_preset = c.active_preset;
        c.edit_surface = EditSurface::Game;
        c
    }

    /// Editor preview: stream board for a chosen session chip.
    pub fn for_stream_preset(&self, preset: SessionPreset) -> Self {
        let mut c = self.clone();
        c.active_preset = preset;
        c.settings_preset = preset;
        c.layouts = c.stream_layouts.clone();
        c.edit_surface = EditSurface::Game;
        c
    }

    pub fn stream_slot(&self, preset: SessionPreset) -> &HudLayout {
        &self.stream_layouts[preset.idx()]
    }

    pub fn stream_slot_mut(&mut self, preset: SessionPreset) -> &mut HudLayout {
        &mut self.stream_layouts[preset.idx()]
    }

    /// Game HWND while F8 is editing Stream: hide live widgets; chrome uses stream edit.
    pub fn for_stream_edit_chrome(&self) -> Self {
        let mut c = self.clone();
        c.edit_surface = EditSurface::Stream;
        for layout in &mut c.layouts {
            for id in WidgetId::ALL {
                layout[id].show = false;
            }
        }
        // Orange boxes / draw path for edit chrome read stream edit via layouts swap:
        c.layouts[c.settings_preset.idx()] = c.stream_layouts[c.settings_preset.idx()].clone();
        for id in WidgetId::ALL {
            // Keep show true for widgets that are on the stream edit board so chrome can hit them.
            let on = c.stream_layouts[c.settings_preset.idx()][id].show;
            c.layouts[c.settings_preset.idx()][id].show = on;
        }
        c.active_preset = c.settings_preset;
        c
    }

    /// True when any session preset has the Pit Board widget on.
    pub fn pitboard_shown_anywhere(&self) -> bool {
        self.layouts
            .iter()
            .any(|layout| layout[WidgetId::Pitboard].show)
    }

    /// Accent written into the MX Bikes menu pack.
    pub fn game_ui_accent(&self) -> [u8; 3] {
        if self.game_ui_match_primary {
            self.primary
        } else {
            self.game_ui_primary
        }
    }

    pub fn copy_settings_to(&mut self, dst: SessionPreset) {
        if dst == self.settings_preset {
            return;
        }
        match self.edit_surface {
            EditSurface::Game => {
                let src = self.layouts[self.settings_preset.idx()].clone();
                self.layouts[dst.idx()] = src;
            }
            EditSurface::Stream => {
                let src = self.stream_layouts[self.settings_preset.idx()].clone();
                self.stream_layouts[dst.idx()] = src;
            }
        }
    }

    pub fn copy_settings_to_all(&mut self) {
        match self.edit_surface {
            EditSurface::Game => {
                let src = self.layouts[self.settings_preset.idx()].clone();
                self.layouts = [src.clone(), src.clone(), src.clone(), src];
            }
            EditSurface::Stream => {
                let src = self.stream_layouts[self.settings_preset.idx()].clone();
                self.stream_layouts = [src.clone(), src.clone(), src.clone(), src];
            }
        }
    }

    /// Copy one stream preset onto another. Game layouts and the F8 slot stay put.
    pub fn copy_stream_layout_to(&mut self, src: SessionPreset, dst: SessionPreset) {
        if src == dst {
            return;
        }
        self.stream_layouts[dst.idx()] = self.stream_layouts[src.idx()].clone();
    }

    /// Copy the open chip's game layout onto that chip's stream slot.
    pub fn copy_game_edit_to_stream(&mut self) {
        let src = self.layouts[self.settings_preset.idx()].clone();
        self.stream_layouts[self.settings_preset.idx()] = src;
    }

    /// Copy one widget's game settings onto the stream slot while that stream widget is still factory.
    /// Rect and show stay put. A later edit to the game layout is not copied.
    pub fn seed_stream_widget_from_game(&mut self, preset: SessionPreset, id: WidgetId) {
        let factory = HudLayout::new().widget_look(id);
        if self.stream_layouts[preset.idx()].widget_look(id) != factory {
            return;
        }
        let source = self.layouts[preset.idx()].clone();
        self.stream_layouts[preset.idx()].copy_widget_settings_from(&source, id);
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
        let primary = fs::read_to_string(&path);
        let (mut scan, mut meta_path): (IniScan, Option<&Path>) = match &primary {
            Ok(text) => (apply_ini_text(&mut cfg, text), Some(path.as_path())),
            Err(_) => match fs::read_to_string(&legacy) {
                Ok(text) => (apply_ini_text(&mut cfg, &text), Some(legacy.as_path())),
                Err(_) => {
                    cfg.first_install_version = env!("CARGO_PKG_VERSION").to_string();
                    cfg.save();
                    return cfg;
                }
            },
        };
        if !scan.has_layout() && primary.is_ok() {
            if let Ok(legacy_text) = fs::read_to_string(&legacy) {
                let extra = apply_ini_text(&mut cfg, &legacy_text);
                if extra.has_layout() {
                    scan.take_layout(extra);
                    meta_path = Some(legacy.as_path());
                }
            }
        }
        if !scan.saw_first_install || cfg.first_install_version.is_empty() {
            cfg.first_install_version = "unknown".into();
        }
        apply_scanned_layouts(&mut cfg, scan);
        for layout in &mut cfg.layouts {
            layout.migrate_rects();
            crate::pitboard::normalize_places(&mut layout.pit.pit_vars);
            if crate::pitboard::previous_factory_slots(
                &layout.pit.pit_art,
                layout.pit.pit_text,
                &layout.pit.pit_vars,
            ) {
                let (text, _, places) = crate::pitboard::factory_pack();
                layout.pit.pit_text = text;
                layout.pit.pit_vars = places;
                if layout[WidgetId::Pitboard].bg == 86 {
                    layout[WidgetId::Pitboard].bg = 100;
                }
            }
            if crate::pitboard::stale_light_factory(&layout.pit.pit_vars) {
                let (text, _, places) = crate::pitboard::factory_pack();
                layout.pit.pit_text = text;
                layout.pit.pit_vars = places;
            }
            crate::pitboard::lift_low_factory(&mut layout.pit.pit_vars);
            crate::pitboard::apply_pack_colors(&layout.pit.pit_art, &mut layout.pit.pit_vars);
            migrate_saved_plate(layout, cfg.primary);
        }
        for layout in &mut cfg.stream_layouts {
            layout.migrate_rects();
        }
        if let Some(p) = meta_path {
            cfg.loaded_mtime = fs::metadata(p).and_then(|m| m.modified()).ok();
        }
        cfg
    }
}

fn migrate_saved_plate(layout: &mut HudLayout, accent: [u8; 3]) {
    if layout.pit.pit_yellow == crate::pitboard::PLATE_YELLOW
        && layout.pit.pit_blue == crate::pitboard::PLATE_NAVY
    {
        layout.pit.pit_yellow = accent;
        layout.pit.pit_blue = [0, 0, 0];
    }
}

impl HudConfig {
    /// Apply an INI body (App + layout sections) without touching disk.
    pub fn apply_ini_str(&mut self, text: &str) {
        let scan = apply_ini_text(self, text);
        apply_scanned_layouts(self, scan);
        let accent = self.primary;
        for layout in &mut self.layouts {
            layout.migrate_rects();
            if crate::pitboard::stale_light_factory(&layout.pit.pit_vars) {
                let (text, _, places) = crate::pitboard::factory_pack();
                layout.pit.pit_text = text;
                layout.pit.pit_vars = places;
            }
            crate::pitboard::lift_low_factory(&mut layout.pit.pit_vars);
            crate::pitboard::apply_pack_colors(&layout.pit.pit_art, &mut layout.pit.pit_vars);
            migrate_saved_plate(layout, accent);
        }
        for layout in &mut self.stream_layouts {
            layout.migrate_rects();
        }
    }

    /// Full ini body for the live stream layout (WASM feed).
    pub fn stream_ini(&self) -> String {
        let live = self.for_stream();
        format!(
            "[App]\n\
             font_family={}\nprimary_color={}\nunits={}\nunits_speed={}\nunits_liquids={}\nunits_temperature={}\n\
             active_preset={}\n\
             \n[{}]\n{}",
            live.font_family.key(),
            format_primary_color(live.primary),
            live.units.speed.key(),
            live.units.speed.key(),
            live.units.liquids.key(),
            live.units.temperature.key(),
            live.active_preset.key(),
            live.active_preset.label(),
            layout_ini(live.live()),
        )
    }

    pub fn add_sys_preset(&mut self, key: &str) {
        if self.sys.sys_apps.len() >= SYS_APP_MAX {
            return;
        }
        if self.sys.sys_apps.iter().any(|a| a.key == key) {
            return;
        }
        if let Some(p) = SYS_PRESETS.iter().find(|p| p.key == key && p.addable()) {
            self.sys.sys_apps.push(SysApp::from_preset(p, true));
        }
    }

    pub fn add_sys_exe(&mut self, exe_name: &str) {
        let name = exe_name.trim().to_ascii_lowercase();
        if !name.ends_with(".exe")
            || name.contains('\\')
            || name.contains('/')
            || name.contains('|')
        {
            return;
        }
        if let Some(p) = SYS_PRESETS
            .iter()
            .find(|p| p.names.iter().any(|n| *n == name))
        {
            if let Some(a) = self.sys.sys_apps.iter_mut().find(|a| a.key == p.key) {
                a.show = true;
                return;
            }
            self.add_sys_preset(p.key);
            return;
        }
        if self
            .sys.sys_apps
            .iter()
            .any(|a| a.names.iter().any(|n| n == &name))
        {
            if let Some(a) = self
                .sys.sys_apps
                .iter_mut()
                .find(|a| a.names.iter().any(|n| n == &name))
            {
                a.show = true;
            }
            return;
        }
        if self.sys.sys_apps.len() >= SYS_APP_MAX {
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
        self.sys.sys_apps.push(SysApp {
            key: format!("exe:{name}"),
            label,
            names: vec![name],
            kind: SysAppKind::Exe,
            show: true,
        });
    }

    pub fn toggle_sys_app(&mut self, i: usize) {
        if let Some(a) = self.sys.sys_apps.get_mut(i) {
            a.show = !a.show;
        }
    }

    pub fn remove_sys_app(&mut self, i: usize) {
        if self.sys.sys_apps.get(i).is_some_and(|a| a.removable()) {
            self.sys.sys_apps.remove(i);
        }
    }

    pub fn group_badge(&self, name: &str) -> Option<(char, [u8; 3])> {
        group_badge(&self.groups, name)
    }

    pub fn add_group(&mut self, name: &str) -> Option<usize> {
        if self.groups.len() >= GROUP_MAX {
            return None;
        }
        let mut group = RiderGroup::new(name);
        if group.name.is_empty() {
            group.name = "Group".into();
        }
        self.groups.push(group);
        Some(self.groups.len() - 1)
    }

    pub fn remove_group(&mut self, index: usize) {
        if index < self.groups.len() {
            self.groups.remove(index);
        }
    }

    pub fn move_group(&mut self, index: usize, delta: isize) -> usize {
        if index >= self.groups.len() {
            return index;
        }
        let next = index as isize + delta;
        if next < 0 || next >= self.groups.len() as isize {
            return index;
        }
        self.groups.swap(index, next as usize);
        next as usize
    }

    pub fn rename_group(&mut self, index: usize, name: &str) {
        let name = clamp_group_name(name);
        if name.is_empty() {
            return;
        }
        if let Some(group) = self.groups.get_mut(index) {
            group.name = name;
        }
    }

    pub fn set_group_icon(&mut self, index: usize, icon: char) {
        if !GROUP_ICONS.contains(&icon) {
            return;
        }
        if let Some(group) = self.groups.get_mut(index) {
            group.icon = icon;
        }
    }

    pub fn set_group_color(&mut self, index: usize, color: [u8; 3]) {
        if !GROUP_COLORS.contains(&color) {
            return;
        }
        if let Some(group) = self.groups.get_mut(index) {
            group.color = color;
        }
    }

    pub fn add_group_members(&mut self, index: usize, names: &[String]) {
        let Some(group) = self.groups.get_mut(index) else {
            return;
        };
        for name in names {
            let name = clamp_group_name(name);
            if name.is_empty() || group.members.len() >= GROUP_MEMBER_MAX {
                continue;
            }
            if group
                .members
                .iter()
                .any(|member| rider_key(member) == rider_key(&name))
            {
                continue;
            }
            group.members.push(name);
        }
    }

    pub fn remove_group_member(&mut self, index: usize, member: usize) {
        if let Some(group) = self.groups.get_mut(index) {
            if member < group.members.len() {
                group.members.remove(member);
            }
        }
    }

    pub fn save(&mut self) {
        let path = ini_path();
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let mut body = format!(
            "# Holeshot HUD layout (normalized 0..1, origin top-left)\n\
             [App]\n\
             font_family={}\nprimary_color={}\nunits={}\nunits_speed={}\nunits_liquids={}\nunits_temperature={}\n\
             settings_key={}\nsettings_theme={}\nlanguage={}\nsettings_x={}\nsettings_y={}\nstart_with_windows={}\nminimize_on_close={}\n\
             close_with_game={}\nopen_with_game={}\nstream_enabled={}\nstream_port={}\nauto_update_on_launch={}\nwhats_new_seen={}\n\
             first_install_version={}\nexperimental={}\nreview={}\ningame_hud={}\ngame_ui={}\n\
             game_ui_match_primary={}\ngame_ui_primary_color={}\n\
             game_ui_splash_path={}\ngame_ui_loading_path={}\nstance_bind={}\nactive_preset={}\n\
             \n[Practice]\n{}\n\n[Warmup]\n{}\n\n[Race]\n{}\n\n[Spectate]\n{}\n\
             \n[PracticeStream]\n{}\n\n[WarmupStream]\n{}\n\n[RaceStream]\n{}\n\n[SpectateStream]\n{}\n",
            self.font_family.key(),
            format_primary_color(self.primary),
            self.units.speed.key(),
            self.units.speed.key(),
            self.units.liquids.key(),
            self.units.temperature.key(),
            self.settings_key.key(),
            self.settings_theme.key(),
            self.language.key(),
            self.settings_x,
            self.settings_y,
            ini_flag(self.start_with_windows),
            ini_flag(self.minimize_on_close),
            ini_flag(self.close_with_game),
            ini_flag(self.open_with_game),
            ini_flag(self.stream_enabled),
            self.stream_port,
            ini_flag(self.auto_update_on_launch),
            self.whats_new_seen,
            self.first_install_version,
            ini_flag(self.experimental),
            ini_flag(self.review),
            ini_flag(self.ingame_hud),
            ini_flag(self.game_ui),
            ini_flag(self.game_ui_match_primary),
            format_primary_color(self.game_ui_primary),
            self.game_ui_splash_path,
            self.game_ui_loading_path,
            self.stance_bind.key(),
            self.active_preset.key(),
            layout_ini(&self.layouts[SessionPreset::Practice.idx()]),
            layout_ini(&self.layouts[SessionPreset::Warmup.idx()]),
            layout_ini(&self.layouts[SessionPreset::Race.idx()]),
            layout_ini(&self.layouts[SessionPreset::Spectate.idx()]),
            layout_ini(&self.stream_layouts[SessionPreset::Practice.idx()]),
            layout_ini(&self.stream_layouts[SessionPreset::Warmup.idx()]),
            layout_ini(&self.stream_layouts[SessionPreset::Race.idx()]),
            layout_ini(&self.stream_layouts[SessionPreset::Spectate.idx()]),
        );
        body.push_str(&groups_section(&self.groups, self.groups_on_maps));
        if write_atomic(&path, &body).is_ok() {
            self.loaded_mtime = fs::metadata(&path).and_then(|m| m.modified()).ok();
        }
        let legacy = legacy_ini_path();
        if legacy != path {
            let _ = fs::remove_file(legacy);
        }
    }

    pub fn apply_to_snapshot(&self, s: &mut Snapshot) {
        // Game HWND always follows the in-game board. Stream edits live in /edit.
        Self::write_layout_to_snapshot(self.live(), s);
    }

    /// Map / Standings / Relative draw from snapshot show+rects. Browser Source must
    /// stamp the live stream board, not the game (or F8 stream-edit) board.
    pub fn apply_stream_live_to_snapshot(&self, s: &mut Snapshot) {
        Self::write_layout_to_snapshot(self.stream_live(), s);
    }


    fn write_layout_to_snapshot(lay: &HudLayout, s: &mut Snapshot) {
        s.standings_rect = lay[WidgetId::Standings].rect;
        s.relative = lay[WidgetId::Relative].rect;
        s.map = lay[WidgetId::Map].rect;
        s.show_standings = i32::from(lay[WidgetId::Standings].show);
        s.show_relative = i32::from(lay[WidgetId::Relative].show);
        s.show_map = i32::from(lay[WidgetId::Map].show);
        s.standings_rows = lay.standings.standings_rows;
        s.relative_count = lay.relative.relative_count;
    }

    pub fn move_st_to(&mut self, from: usize, to: usize) {
        move_to(&mut self.standings.st_order, from, to);
    }

    pub fn move_rel_to(&mut self, from: usize, to: usize) {
        move_to(&mut self.relative.rel_order, from, to);
    }

    pub fn standings_cols(&self) -> Vec<StField> {
        let mut cols: Vec<_> = self
            .standings.st_order
            .iter()
            .copied()
            .filter(|c| c.enabled(self))
            .collect();
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
        let mut cols: Vec<_> = self
            .relative.rel_order
            .iter()
            .copied()
            .filter(|c| c.enabled(self))
            .collect();
        if cols.is_empty() {
            cols.push(RelField::Name);
        }
        cols
    }

    /// Settings → Labs → Experimental features. Gates Profile → Tracks.
    pub fn experimental_unlocked(&self) -> bool {
        self.experimental
    }

    pub fn gamepad_visible(&self) -> bool {
        self[WidgetId::Gamepad].show
    }

    pub fn sector_visible(&self) -> bool {
        self[WidgetId::Sector].show
    }

    pub fn sector_hist_count(&self) -> usize {
        self.sector.sector_hist_laps.clamp(1, 5) as usize
    }

    pub fn telemetry_draw_traces(&self) -> bool {
        self.telemetry.telemetry_traces
            && (self.telemetry.telemetry_trace_throttle
                || self.telemetry.telemetry_trace_brake
                || self.telemetry.telemetry_trace_steer)
    }

    pub fn telemetry_draw_bars(&self) -> bool {
        self.telemetry.telemetry_bars
            && (self.telemetry.telemetry_bar_clutch
                || self.telemetry.telemetry_bar_brake
                || self.telemetry.telemetry_bar_throttle
                || self.telemetry.telemetry_bar_steer)
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
    StreamPreset(SessionPreset),
    Groups,
    Legacy,
}

struct IniScan {
    saw_last_cols: [bool; SessionPreset::COUNT],
    saw_stream_last_cols: [bool; SessionPreset::COUNT],
    saw_first_install: bool,
    saw_unit: [bool; UnitKind::COUNT],
    saw_preset: [bool; SessionPreset::COUNT],
    saw_stream_preset: [bool; SessionPreset::COUNT],
    saw_legacy_layout: bool,
    legacy_last_cols: bool,
    legacy_layout: HudLayout,
}

impl Default for IniScan {
    fn default() -> Self {
        Self {
            saw_last_cols: [false; SessionPreset::COUNT],
            saw_stream_last_cols: [false; SessionPreset::COUNT],
            saw_first_install: false,
            saw_unit: [false; UnitKind::COUNT],
            saw_preset: [false; SessionPreset::COUNT],
            saw_stream_preset: [false; SessionPreset::COUNT],
            saw_legacy_layout: false,
            legacy_last_cols: false,
            legacy_layout: HudLayout::new(),
        }
    }
}

impl IniScan {
    fn has_layout(&self) -> bool {
        self.saw_preset.iter().any(|&on| on) || self.saw_legacy_layout
    }

    fn take_layout(&mut self, extra: Self) {
        self.saw_last_cols = extra.saw_last_cols;
        self.saw_stream_last_cols = extra.saw_stream_last_cols;
        self.saw_preset = extra.saw_preset;
        self.saw_stream_preset = extra.saw_stream_preset;
        self.saw_legacy_layout = extra.saw_legacy_layout;
        self.legacy_last_cols = extra.legacy_last_cols;
        self.legacy_layout = extra.legacy_layout;
        self.saw_first_install = self.saw_first_install || extra.saw_first_install;
    }
}

fn apply_ini_text(cfg: &mut HudConfig, text: &str) -> IniScan {
    let mut scan = IniScan::default();
    let mut section = IniSection::Legacy;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') {
            section = parse_ini_section(line);
            if matches!(section, IniSection::Groups) {
                cfg.groups.clear();
                cfg.groups_on_maps = false;
            }
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
                apply_app_key(
                    cfg,
                    key,
                    val,
                    b,
                    &mut scan.saw_first_install,
                    &mut scan.saw_unit,
                );
            }
            IniSection::Preset(p) => {
                scan.saw_preset[p.idx()] = true;
                let layout = &mut cfg.layouts[p.idx()];
                if apply_widget_prefs(layout, key, val, f, b) {
                    continue;
                }
                apply_layout_key(layout, key, val, b, &mut scan.saw_last_cols[p.idx()]);
            }
            IniSection::StreamPreset(p) => {
                scan.saw_stream_preset[p.idx()] = true;
                let layout = &mut cfg.stream_layouts[p.idx()];
                if apply_widget_prefs(layout, key, val, f, b) {
                    continue;
                }
                apply_layout_key(layout, key, val, b, &mut scan.saw_stream_last_cols[p.idx()]);
            }
            IniSection::Groups => {
                if key == "on_maps" {
                    cfg.groups_on_maps = b;
                    continue;
                }
                if let Some(group) = parse_group_record(val) {
                    if cfg.groups.len() < GROUP_MAX {
                        cfg.groups.push(group);
                    }
                }
            }
            IniSection::Legacy => {
                if apply_app_key(
                    cfg,
                    key,
                    val,
                    b,
                    &mut scan.saw_first_install,
                    &mut scan.saw_unit,
                ) {
                    continue;
                }
                scan.saw_legacy_layout = true;
                if apply_widget_prefs(&mut scan.legacy_layout, key, val, f, b) {
                    continue;
                }
                apply_layout_key(
                    &mut scan.legacy_layout,
                    key,
                    val,
                    b,
                    &mut scan.legacy_last_cols,
                );
            }
        }
    }
    scan
}

fn apply_scanned_layouts(cfg: &mut HudConfig, scan: IniScan) {
    if scan.saw_preset.iter().any(|&on| on) {
        let donor = SessionPreset::ALL
            .iter()
            .find(|p| scan.saw_preset[p.idx()])
            .map(|p| cfg.layouts[p.idx()].clone())
            .unwrap_or_else(HudLayout::new);
        for p in SessionPreset::ALL {
            if !scan.saw_preset[p.idx()] {
                cfg.layouts[p.idx()] = donor.clone();
            }
            if !scan.saw_last_cols[p.idx()] && scan.saw_preset[p.idx()] {
                cfg.layouts[p.idx()].standings.st_best = true;
                cfg.layouts[p.idx()].standings.st_last = true;
                cfg.layouts[p.idx()].relative.rel_best = true;
                cfg.layouts[p.idx()].relative.rel_last = true;
            }
        }
    } else if scan.saw_legacy_layout {
        let mut legacy_layout = scan.legacy_layout;
        if !scan.legacy_last_cols {
            legacy_layout.standings.st_best = true;
            legacy_layout.standings.st_last = true;
            legacy_layout.relative.rel_best = true;
            legacy_layout.relative.rel_last = true;
        }
        cfg.layouts = [
            legacy_layout.clone(),
            legacy_layout.clone(),
            legacy_layout.clone(),
            legacy_layout,
        ];
    }
    // Missing stream sections stay HudLayout::new() (all Show off). Only fill last-cols defaults.
    for p in SessionPreset::ALL {
        if scan.saw_stream_preset[p.idx()] && !scan.saw_stream_last_cols[p.idx()] {
            cfg.stream_layouts[p.idx()].standings.st_best = true;
            cfg.stream_layouts[p.idx()].standings.st_last = true;
            cfg.stream_layouts[p.idx()].relative.rel_best = true;
            cfg.stream_layouts[p.idx()].relative.rel_last = true;
        }
    }
}

fn ini_tmp_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".tmp");
    PathBuf::from(name)
}

fn write_atomic(path: &Path, body: &str) -> std::io::Result<()> {
    let tmp = ini_tmp_path(path);
    fs::write(&tmp, body)?;
    match fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(_) if path.exists() => {
            fs::remove_file(path)?;
            fs::rename(&tmp, path)
        }
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            Err(e)
        }
    }
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
        "practicestream" => IniSection::StreamPreset(SessionPreset::Practice),
        "warmupstream" => IniSection::StreamPreset(SessionPreset::Warmup),
        "racestream" => IniSection::StreamPreset(SessionPreset::Race),
        "spectatestream" | "spectatingstream" => IniSection::StreamPreset(SessionPreset::Spectate),
        "groups" => IniSection::Groups,
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
        "review" => cfg.review = b,
        "ingame_hud" => cfg.ingame_hud = b,
        "game_ui" => cfg.game_ui = b,
        "game_ui_match_primary" => cfg.game_ui_match_primary = b,
        "game_ui_primary_color" => {
            if let Some(rgb) = parse_primary_color(val) {
                cfg.game_ui_primary = rgb;
            }
        }
        "game_ui_splash_path" => {
            cfg.game_ui_splash_path = game_ui_image_path(val);
        }
        "game_ui_loading_path" => {
            cfg.game_ui_loading_path = game_ui_image_path(val);
        }
        "font_family" => cfg.font_family = FontFamily::parse(val),
        "primary_color" => {
            if let Some(rgb) = parse_primary_color(val) {
                cfg.primary = rgb;
            }
        }
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
        "stream_enabled" => cfg.stream_enabled = b,
        "stream_port" => {
            if let Ok(n) = val.parse::<u16>() {
                cfg.stream_port = n;
            }
        }
        "auto_update_on_launch" => cfg.auto_update_on_launch = b,
        "whats_new_seen" => cfg.whats_new_seen = val.trim().to_string(),
        "first_install_version" => {
            cfg.first_install_version = val.trim().to_string();
            *saw_first_install = true;
        }
        "settings_key" => cfg.settings_key = SettingsKey::parse(val),
        "settings_theme" => cfg.settings_theme = SettingsTheme::parse(val),
        "language" => cfg.language = Language::parse(val),
        "settings_x" => {
            if let Ok(n) = val.parse::<i32>() {
                cfg.settings_x = n;
            }
        }
        "settings_y" => {
            if let Ok(n) = val.parse::<i32>() {
                cfg.settings_y = n;
            }
        }
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

/// Fields that change a widget's pixels. Rect and show are left out.
struct LookMix(u64);

impl LookMix {
    fn new() -> Self {
        Self(0xcbf29ce484222325)
    }

    fn unit(&mut self, value: u64) {
        self.0 ^= value;
        self.0 = self.0.wrapping_mul(0x100000001b3);
    }

    fn i32(&mut self, value: i32) {
        self.unit(value as u32 as u64);
    }

    fn flag(&mut self, value: bool) {
        self.unit(u64::from(value));
    }

    fn text(&mut self, value: &str) {
        self.unit(value.len() as u64);
        for byte in value.bytes() {
            self.unit(u64::from(byte));
        }
    }

    fn scalar(&mut self, value: f32) {
        self.unit(u64::from(value.to_bits()));
    }

    fn finish(self) -> u64 {
        self.0
    }
}

fn mix_style(mix: &mut LookMix, lay: &HudLayout, id: WidgetId) {
    let prefs = &lay[id];
    mix.i32(prefs.font);
    mix.flag(prefs.bold);
    mix.i32(prefs.bg);
}

fn mix_board(mix: &mut LookMix, slots: &[BoardField]) {
    for field in slots {
        mix.text(field.key());
    }
}

fn rgb_key(rgb: [u8; 3]) -> u64 {
    u64::from(rgb[0]) << 16 | u64::from(rgb[1]) << 8 | u64::from(rgb[2])
}

fn sys_kind_key(kind: SysAppKind) -> &'static str {
    match kind {
        SysAppKind::Hud => "hud",
        SysAppKind::Mxbikes => "mxbikes",
        SysAppKind::MxbApp => "mxbapp",
        SysAppKind::Reshade => "reshade",
        SysAppKind::Exe => "exe",
    }
}

impl HudLayout {
    /// Fingerprint of the fields that change this widget's pixels.
    /// Rect and show stay out: the stream crop places the widget, and show decides whether it is drawn.
    /// Pit Board font, bold, and background are omitted because the plate draw does not read them.
    pub fn widget_look(&self, id: WidgetId) -> u64 {
        let mut mix = LookMix::new();
        mix.unit(id.idx() as u64);
        match id {
            WidgetId::Standings => self.mix_standings(&mut mix),
            WidgetId::Relative => self.mix_relative(&mut mix),
            WidgetId::Map => self.mix_map(&mut mix),
            WidgetId::Minimap => self.mix_minimap(&mut mix),
            WidgetId::Radar => self.mix_radar(&mut mix),
            WidgetId::Dash => self.mix_dash(&mut mix),
            WidgetId::Ticker => self.mix_ticker(&mut mix),
            WidgetId::Sys => self.mix_sys(&mut mix),
            WidgetId::Sector => self.mix_sector(&mut mix),
            WidgetId::Delta => self.mix_delta(&mut mix),
            WidgetId::Stance => self.mix_stance(&mut mix),
            WidgetId::Flag => self.mix_flag(&mut mix),
            WidgetId::Lean => self.mix_lean(&mut mix),
            WidgetId::Gamepad => self.mix_gamepad(&mut mix),
            WidgetId::Telemetry => self.mix_telemetry(&mut mix),
            WidgetId::Pitboard => self.mix_pit(&mut mix),
            WidgetId::Timer => {
                mix_style(&mut mix, self, WidgetId::Timer);
                mix.flag(self.timer.timer_of);
            }
        }
        mix.finish()
    }

    /// Copy the fields `widget_look` reads. Rect and show stay on `self`.
    pub fn copy_widget_settings_from(&mut self, source: &HudLayout, id: WidgetId) {
        if id != WidgetId::Pitboard {
            let rect = self[id].rect;
            let show = self[id].show;
            self[id] = source[id];
            self[id].rect = rect;
            self[id].show = show;
        }
        match id {
            WidgetId::Standings => self.standings = source.standings.clone(),
            WidgetId::Relative => self.relative = source.relative.clone(),
            WidgetId::Map => self.map = source.map.clone(),
            WidgetId::Minimap => self.mini = source.mini.clone(),
            WidgetId::Radar => self.radar = source.radar.clone(),
            WidgetId::Dash => self.dash = source.dash.clone(),
            WidgetId::Ticker => self.ticker = source.ticker.clone(),
            WidgetId::Sys => self.sys = source.sys.clone(),
            WidgetId::Sector => self.sector = source.sector.clone(),
            WidgetId::Delta => self.delta = source.delta.clone(),
            WidgetId::Stance => self.stance = source.stance.clone(),
            WidgetId::Flag => self.flag = source.flag.clone(),
            WidgetId::Lean => self.lean = source.lean.clone(),
            WidgetId::Gamepad => self.gamepad = source.gamepad.clone(),
            WidgetId::Telemetry => self.telemetry = source.telemetry.clone(),
            WidgetId::Pitboard => self.pit = source.pit.clone(),
            WidgetId::Timer => self.timer = source.timer.clone(),
        }
    }

    fn mix_standings(&self, mix: &mut LookMix) {
        mix_style(mix, self, WidgetId::Standings);
        mix.i32(self.standings.standings_rows);
        mix.i32(self.standings.st_hl);
        mix.text(self.standings.st_text.key());
        mix.flag(self.standings.st_stripe);
        mix.text(self.standings.st_plaque_text.key());
        mix.flag(self.standings.st_plaque);
        mix.flag(self.standings.st_pos);
        mix.flag(self.standings.st_num);
        mix.flag(self.standings.st_name);
        mix.flag(self.standings.st_gap);
        mix.flag(self.standings.st_interval);
        mix.flag(self.standings.st_laps);
        mix.flag(self.standings.st_current);
        mix.flag(self.standings.st_best);
        mix.flag(self.standings.st_last);
        mix.flag(self.standings.st_status);
        mix.flag(self.standings.st_bike);
        mix.flag(self.standings.st_penalty);
        mix.flag(self.standings.st_crashed);
        mix.flag(self.standings.st_category);
        mix.flag(self.standings.st_lapdiff);
        mix.i32(self.standings.st_w_pos);
        mix.i32(self.standings.st_w_num);
        mix.i32(self.standings.st_w_name);
        mix.i32(self.standings.st_w_gap);
        mix.i32(self.standings.st_w_interval);
        mix.i32(self.standings.st_w_laps);
        mix.i32(self.standings.st_w_current);
        mix.i32(self.standings.st_w_best);
        mix.i32(self.standings.st_w_last);
        mix.i32(self.standings.st_w_status);
        mix.i32(self.standings.st_w_bike);
        mix.i32(self.standings.st_w_penalty);
        mix.i32(self.standings.st_w_crashed);
        mix.i32(self.standings.st_w_category);
        mix.i32(self.standings.st_w_lapdiff);
        mix_board(mix, &self.standings.st_head);
        mix_board(mix, &self.standings.st_foot);
        mix.unit(self.standings.st_order.len() as u64);
        for field in &self.standings.st_order {
            mix.text(field.key());
        }
    }

    fn mix_relative(&self, mix: &mut LookMix) {
        mix_style(mix, self, WidgetId::Relative);
        mix.i32(self.relative.relative_count);
        mix.i32(self.relative.rel_hl);
        mix.text(self.relative.rel_text.key());
        mix.flag(self.relative.rel_stripe);
        mix.text(self.relative.rel_plaque_text.key());
        mix.flag(self.relative.rel_plaque);
        mix.flag(self.relative.rel_num);
        mix.flag(self.relative.rel_name);
        mix.flag(self.relative.rel_gap);
        mix.flag(self.relative.rel_laps);
        mix.flag(self.relative.rel_current);
        mix.flag(self.relative.rel_pos);
        mix.flag(self.relative.rel_bike);
        mix.flag(self.relative.rel_penalty);
        mix.flag(self.relative.rel_interval);
        mix.flag(self.relative.rel_status);
        mix.flag(self.relative.rel_best);
        mix.flag(self.relative.rel_last);
        mix.flag(self.relative.rel_category);
        mix.flag(self.relative.rel_speed);
        mix.flag(self.relative.rel_lapdiff);
        mix.i32(self.relative.rel_w_num);
        mix.i32(self.relative.rel_w_name);
        mix.i32(self.relative.rel_w_gap);
        mix.i32(self.relative.rel_w_laps);
        mix.i32(self.relative.rel_w_current);
        mix.i32(self.relative.rel_w_pos);
        mix.i32(self.relative.rel_w_bike);
        mix.i32(self.relative.rel_w_penalty);
        mix.i32(self.relative.rel_w_interval);
        mix.i32(self.relative.rel_w_status);
        mix.i32(self.relative.rel_w_best);
        mix.i32(self.relative.rel_w_last);
        mix.i32(self.relative.rel_w_category);
        mix.i32(self.relative.rel_w_speed);
        mix.i32(self.relative.rel_w_lapdiff);
        mix_board(mix, &self.relative.rel_head);
        mix_board(mix, &self.relative.rel_foot);
        mix.unit(self.relative.rel_order.len() as u64);
        for field in &self.relative.rel_order {
            mix.text(field.key());
        }
    }

    fn mix_map(&self, mix: &mut LookMix) {
        mix_style(mix, self, WidgetId::Map);
        mix.flag(self.map.map_others);
        mix.flag(self.map.map_sf);
        mix.flag(self.map.map_sectors);
        mix.flag(self.map.map_name);
        mix.flag(self.map.map_numbers);
        mix.flag(self.map.map_arrows);
        mix.flag(self.map.map_follow);
        mix.i32(self.map.map_zoom);
        mix.flag(self.map.map_crown);
        mix.flag(self.map.map_place);
        mix.text(self.map.map_dot.key());
        mix.text(self.map.map_you_text.key());
        mix.i32(self.map.map_dot_opacity);
    }

    fn mix_minimap(&self, mix: &mut LookMix) {
        mix_style(mix, self, WidgetId::Minimap);
        mix.flag(self.mini.mini_others);
        mix.flag(self.mini.mini_sf);
        mix.flag(self.mini.mini_sectors);
        mix.flag(self.mini.mini_numbers);
        mix.flag(self.mini.mini_arrows);
        mix.flag(self.mini.mini_crown);
        mix.flag(self.mini.mini_place);
        mix.text(self.mini.mini_dot.key());
        mix.i32(self.mini.mini_zoom);
    }

    fn mix_radar(&self, mix: &mut LookMix) {
        mix_style(mix, self, WidgetId::Radar);
        mix.flag(self.radar.radar_sides);
        mix.flag(self.radar.radar_rear);
        mix.flag(self.radar.radar_rings);
        mix.text(self.radar.radar_style.key());
        mix.i32(self.radar.radar_range);
    }

    fn mix_dash(&self, mix: &mut LookMix) {
        mix_style(mix, self, WidgetId::Dash);
        mix.flag(self.dash.dash_rev);
        mix.flag(self.dash.dash_yellow);
        mix.flag(self.dash.dash_blue);
        mix.flag(self.dash.dash_red);
        mix.flag(self.dash.dash_simple);
        mix.flag(self.dash.dash_shift_color);
        mix.text(self.dash.dash_left.key());
        mix.text(self.dash.dash_mid.key());
        mix.text(self.dash.dash_right.key());
    }

    fn mix_ticker(&self, mix: &mut LookMix) {
        mix_style(mix, self, WidgetId::Ticker);
        mix.i32(self.ticker.ticker_count);
        mix.flag(self.ticker.ticker_title);
        mix.flag(self.ticker.ticker_autoscroll);
        mix.flag(self.ticker.ticker_status);
        mix.flag(self.ticker.ticker_slide);
        mix.i32(self.ticker.ticker_hl);
        mix.text(self.ticker.ticker_left.key());
        mix.text(self.ticker.ticker_right.key());
    }

    fn mix_sys(&self, mix: &mut LookMix) {
        mix_style(mix, self, WidgetId::Sys);
        mix.unit(self.sys.sys_apps.len() as u64);
        for app in &self.sys.sys_apps {
            mix.text(&app.key);
            mix.text(&app.label);
            mix.text(sys_kind_key(app.kind));
            mix.flag(app.show);
            mix.unit(app.names.len() as u64);
            for name in &app.names {
                mix.text(name);
            }
        }
    }

    fn mix_sector(&self, mix: &mut LookMix) {
        mix_style(mix, self, WidgetId::Sector);
        mix.flag(self.sector.sector_live);
        mix.flag(self.sector.sector_session);
        mix.flag(self.sector.sector_hist);
        mix.i32(self.sector.sector_hist_laps);
    }

    fn mix_delta(&self, mix: &mut LookMix) {
        mix_style(mix, self, WidgetId::Delta);
        mix.flag(self.delta.delta_session);
    }

    fn mix_stance(&self, mix: &mut LookMix) {
        mix_style(mix, self, WidgetId::Stance);
        mix.text(self.stance.stance_style.key());
        mix.flag(self.stance.stance_show_sit);
    }

    fn mix_flag(&self, mix: &mut LookMix) {
        mix_style(mix, self, WidgetId::Flag);
        mix.flag(self.flag.flag_text);
        mix.flag(self.flag.flag_yellow);
        mix.flag(self.flag.flag_blue);
        mix.flag(self.flag.flag_red);
    }

    fn mix_lean(&self, mix: &mut LookMix) {
        mix_style(mix, self, WidgetId::Lean);
        mix.text(self.lean.lean_style.key());
    }

    fn mix_gamepad(&self, mix: &mut LookMix) {
        mix_style(mix, self, WidgetId::Gamepad);
        mix.text(self.gamepad.gamepad_style.key());
        mix.text(self.gamepad.gamepad_theme.key());
    }

    fn mix_telemetry(&self, mix: &mut LookMix) {
        mix_style(mix, self, WidgetId::Telemetry);
        mix.flag(self.telemetry.telemetry_traces);
        mix.flag(self.telemetry.telemetry_trace_throttle);
        mix.flag(self.telemetry.telemetry_trace_brake);
        mix.flag(self.telemetry.telemetry_trace_steer);
        mix.flag(self.telemetry.telemetry_bars);
        mix.flag(self.telemetry.telemetry_bar_clutch);
        mix.flag(self.telemetry.telemetry_bar_brake);
        mix.flag(self.telemetry.telemetry_bar_throttle);
        mix.flag(self.telemetry.telemetry_bar_steer);
        mix.flag(self.telemetry.telemetry_dial);
    }

    fn mix_pit(&self, mix: &mut LookMix) {
        mix.text(&self.pit.pit_sponsor);
        mix.text(&self.pit.pit_art);
        mix.text(&self.pit.pit_board);
        mix.text(self.pit.pit_text.key());
        mix.text(self.pit.pit_when.key());
        mix.unit(rgb_key(self.pit.pit_yellow));
        mix.unit(rgb_key(self.pit.pit_blue));
        mix.unit(self.pit.pit_vars.len() as u64);
        for place in &self.pit.pit_vars {
            mix.text(place.var.key());
            mix.scalar(place.x);
            mix.scalar(place.y);
            mix.scalar(place.size);
            mix.flag(place.show);
            mix.flag(place.bold);
            mix.text(&place.label);
            match place.color {
                Some(rgb) => {
                    mix.flag(true);
                    mix.unit(rgb_key(rgb));
                }
                None => mix.flag(false),
            }
        }
    }

    /// Apply one INI layout key. `true`/`1` sets a toggle on.
    pub fn apply_setting(&mut self, key: &str, value: &str) {
        let on = value == "1" || value.eq_ignore_ascii_case("true");
        let mut saw_last_cols = false;
        apply_layout_key(self, key, value, on, &mut saw_last_cols);
    }
}

impl SectorLayout {
    fn apply_key(&mut self, key: &str, val: &str, b: bool) -> bool {
        match key {
        "sector_live" => self.sector_live = b,
        "sector_session" => self.sector_session = b,
        "sector_hist" => self.sector_hist = b,
        "sector_hist_laps" => self.sector_hist_laps = val.parse().unwrap_or(3).clamp(1, 5),
            _ => return false,
        }
        true
    }
}

impl DeltaLayout {
    fn apply_key(&mut self, key: &str, _val: &str, b: bool) -> bool {
        match key {
        "delta_session" => self.delta_session = b,
            _ => return false,
        }
        true
    }
}

impl FlagLayout {
    fn apply_key(&mut self, key: &str, _val: &str, b: bool) -> bool {
        match key {
        "flag_caution" => {
            self.flag_yellow = b;
            self.flag_blue = b;
        }
        "flag_yellow" => self.flag_yellow = b,
        "flag_blue" => self.flag_blue = b,
        "flag_red" => self.flag_red = b,
        "flag_text" => self.flag_text = b,
            _ => return false,
        }
        true
    }
}

impl StandingsLayout {
    fn apply_key(&mut self, key: &str, val: &str, b: bool, saw_last_cols: &mut bool) -> bool {
        match key {
        "standings_rows" => self.standings_rows = val.parse().unwrap_or(12).max(3),
        "st_pos" => self.st_pos = b,
        "st_num" => self.st_num = b,
        "st_name" => self.st_name = b,
        "st_gap" => self.st_gap = b,
        "st_interval" => self.st_interval = b,
        "st_laps" => self.st_laps = b,
        "st_current" => self.st_current = b,
        "st_best" => self.st_best = b,
        "st_last" => {
            self.st_last = b;
            *saw_last_cols = true;
        }
        "st_status" => self.st_status = b,
        "st_bike" => self.st_bike = b,
        "st_penalty" => self.st_penalty = b,
        "st_crashed" => self.st_crashed = b,
        "st_category" => self.st_category = b,
        "st_lapdiff" => self.st_lapdiff = b,
        "st_hl" => self.st_hl = clamp_pct(val),
        "st_text" => self.st_text = TableText::parse(val),
        "st_stripe" => self.st_stripe = b,
        "st_plaque_text" => self.st_plaque_text = TableText::parse(val),
        "st_plaque" => self.st_plaque = b,
        "st_head" => self.st_head = parse_board(val, BoardField::DEFAULT_HEAD),
        "st_foot" => self.st_foot = parse_board(val, BoardField::DEFAULT_FOOT),
        "st_order" => self.st_order = parse_st_order(val),
        "st_w_pos" => self.st_w_pos = clamp_w(val),
        "st_w_num" => self.st_w_num = clamp_w(val),
        "st_w_name" => self.st_w_name = clamp_name_w(val),
        "st_w_gap" => self.st_w_gap = clamp_w(val),
        "st_w_interval" => self.st_w_interval = clamp_w(val),
        "st_w_laps" => self.st_w_laps = clamp_w(val),
        "st_w_current" => self.st_w_current = clamp_w(val),
        "st_w_best" => self.st_w_best = clamp_w(val),
        "st_w_last" => self.st_w_last = clamp_w(val),
        "st_w_status" => self.st_w_status = clamp_w(val),
        "st_w_bike" => self.st_w_bike = clamp_w(val),
        "st_w_penalty" => self.st_w_penalty = clamp_w(val),
        "st_w_crashed" => self.st_w_crashed = clamp_w(val),
        "st_w_category" => self.st_w_category = clamp_w(val),
        "st_w_lapdiff" => self.st_w_lapdiff = clamp_w(val),
            _ => return false,
        }
        true
    }
}

impl RelativeLayout {
    fn apply_key(&mut self, key: &str, val: &str, b: bool) -> bool {
        match key {
        "relative_count" => self.relative_count = val.parse().unwrap_or(3).max(1),
        "rel_num" => self.rel_num = b,
        "rel_name" => self.rel_name = b,
        "rel_gap" => self.rel_gap = b,
        "rel_laps" => self.rel_laps = b,
        "rel_current" => self.rel_current = b,
        "rel_pos" => self.rel_pos = b,
        "rel_bike" => self.rel_bike = b,
        "rel_penalty" => self.rel_penalty = b,
        "rel_interval" => self.rel_interval = b,
        "rel_status" => self.rel_status = b,
        "rel_best" => self.rel_best = b,
        "rel_last" => self.rel_last = b,
        "rel_category" => self.rel_category = b,
        "rel_speed" => self.rel_speed = b,
        "rel_lapdiff" => self.rel_lapdiff = b,
        "rel_hl" => self.rel_hl = clamp_pct(val),
        "rel_text" => self.rel_text = TableText::parse(val),
        "rel_stripe" => self.rel_stripe = b,
        "rel_plaque_text" => self.rel_plaque_text = TableText::parse(val),
        "rel_plaque" => self.rel_plaque = b,
        "rel_head" => self.rel_head = parse_board(val, BoardField::DEFAULT_HEAD),
        "rel_foot" => self.rel_foot = parse_board(val, BoardField::DEFAULT_FOOT),
        "rel_order" => self.rel_order = parse_rel_order(val),
        "rel_w_num" => self.rel_w_num = clamp_w(val),
        "rel_w_name" => self.rel_w_name = clamp_name_w(val),
        "rel_w_gap" => self.rel_w_gap = clamp_w(val),
        "rel_w_laps" => self.rel_w_laps = clamp_w(val),
        "rel_w_current" => self.rel_w_current = clamp_w(val),
        "rel_w_pos" => self.rel_w_pos = clamp_w(val),
        "rel_w_bike" => self.rel_w_bike = clamp_w(val),
        "rel_w_penalty" => self.rel_w_penalty = clamp_w(val),
        "rel_w_interval" => self.rel_w_interval = clamp_w(val),
        "rel_w_status" => self.rel_w_status = clamp_w(val),
        "rel_w_best" => self.rel_w_best = clamp_w(val),
        "rel_w_last" => self.rel_w_last = clamp_w(val),
        "rel_w_category" => self.rel_w_category = clamp_w(val),
        "rel_w_speed" => self.rel_w_speed = clamp_w(val),
        "rel_w_lapdiff" => self.rel_w_lapdiff = clamp_w(val),
            _ => return false,
        }
        true
    }
}

impl TickerLayout {
    fn apply_key(&mut self, key: &str, val: &str, b: bool) -> bool {
        match key {
        "ticker_count" => self.ticker_count = val.parse().unwrap_or(7).clamp(3, 15),
        "ticker_title" => self.ticker_title = b,
        "ticker_autoscroll" => self.ticker_autoscroll = b,
        "ticker_status" => self.ticker_status = b,
        "ticker_slide" => self.ticker_slide = b,
        "ticker_hl" => self.ticker_hl = clamp_pct(val),
        "ticker_left" => self.ticker_left = BoardField::parse(val),
        "ticker_right" => self.ticker_right = BoardField::parse(val),
            _ => return false,
        }
        true
    }
}

impl MapLayout {
    fn apply_key(&mut self, key: &str, val: &str, b: bool) -> bool {
        match key {
        "map_others" => self.map_others = b,
        "map_sf" => self.map_sf = b,
        "map_sectors" => self.map_sectors = b,
        "map_name" => self.map_name = b,
        "map_numbers" => self.map_numbers = b,
        "map_arrows" => self.map_arrows = b,
        "map_follow" => self.map_follow = b,
        "map_zoom" => self.map_zoom = clamp_pct(val),
        "map_crown" => self.map_crown = b,
        "map_place" => self.map_place = b,
        "map_dot" => self.map_dot = DotLabel::parse(val),
        "map_you_text" => self.map_you_text = TableText::parse(val),
        "map_dot_opacity" => {
            self.map_dot_opacity = val
                .parse()
                .unwrap_or(MAP_DOT_OPACITY_DEFAULT)
                .clamp(0, 100)
        }
            _ => return false,
        }
        true
    }
}

impl MiniLayout {
    fn apply_key(&mut self, key: &str, val: &str, b: bool) -> bool {
        match key {
        "mini_others" => self.mini_others = b,
        "mini_sf" => self.mini_sf = b,
        "mini_sectors" => self.mini_sectors = b,
        "mini_numbers" => self.mini_numbers = b,
        "mini_arrows" => self.mini_arrows = b,
        "mini_crown" => self.mini_crown = b,
        "mini_place" => self.mini_place = b,
        "mini_dot" => self.mini_dot = DotLabel::parse(val),
        "mini_zoom" => self.mini_zoom = clamp_pct(val),
            _ => return false,
        }
        true
    }
}

impl RadarLayout {
    fn apply_key(&mut self, key: &str, val: &str, b: bool) -> bool {
        match key {
        "radar_sides" => self.radar_sides = b,
        "radar_rear" => self.radar_rear = b,
        "radar_rings" => self.radar_rings = b,
        "radar_style" => self.radar_style = RadarStyle::parse(val),
        "radar_range" => self.radar_range = clamp_radar_range(val),
            _ => return false,
        }
        true
    }
}

impl TimerLayout {
    fn apply_key(&mut self, key: &str, on: bool) -> bool {
        match key {
            "timer_of" => self.timer_of = on,
            _ => return false,
        }
        true
    }
}

impl DashLayout {
    fn apply_key(&mut self, key: &str, val: &str, b: bool) -> bool {
        match key {
        "dash_rev" => self.dash_rev = b,
        "dash_yellow" => self.dash_yellow = b,
        "dash_blue" => self.dash_blue = b,
        "dash_red" => self.dash_red = b,
        "dash_simple" => self.dash_simple = b,
        "dash_shift_color" => self.dash_shift_color = b,
        "dash_left" => self.dash_left = DashField::parse(val),
        "dash_mid" => self.dash_mid = DashField::parse(val),
        "dash_right" => self.dash_right = DashField::parse(val),
            _ => return false,
        }
        true
    }
}

impl StanceLayout {
    fn apply_key(&mut self, key: &str, val: &str, b: bool) -> bool {
        match key {
        "stance_mode" => self.stance_mode = StanceMode::parse(val),
        "stance_style" => self.stance_style = StanceStyle::parse(val),
        "stance_show_sit" => self.stance_show_sit = b,
        "stance_icon" => {
            if b {
                self.stance_style = StanceStyle::Icon;
            }
        }
            _ => return false,
        }
        true
    }
}

impl LeanLayout {
    fn apply_key(&mut self, key: &str, val: &str, _b: bool) -> bool {
        match key {
        "lean_style" => self.lean_style = LeanStyle::parse(val),
            _ => return false,
        }
        true
    }
}

impl GamepadLayout {
    fn apply_key(&mut self, key: &str, val: &str, _b: bool) -> bool {
        match key {
        "gamepad_style" => self.gamepad_style = GamepadStyle::parse(val),
        "gamepad_theme" => self.gamepad_theme = GamepadTheme::parse(val),
            _ => return false,
        }
        true
    }
}

impl SysLayout {
    fn apply_key(&mut self, key: &str, val: &str, _b: bool) -> bool {
        match key {
        "sys_apps" => self.sys_apps = parse_sys_apps(val),
            _ => return false,
        }
        true
    }
}

impl TelemetryLayout {
    fn apply_key(&mut self, key: &str, _val: &str, b: bool) -> bool {
        match key {
        "telemetry_traces" => self.telemetry_traces = b,
        "telemetry_trace_throttle" => self.telemetry_trace_throttle = b,
        "telemetry_trace_brake" => self.telemetry_trace_brake = b,
        "telemetry_trace_steer" => self.telemetry_trace_steer = b,
        "telemetry_bars" => self.telemetry_bars = b,
        "telemetry_bar_clutch" => self.telemetry_bar_clutch = b,
        "telemetry_bar_brake" => self.telemetry_bar_brake = b,
        "telemetry_bar_throttle" => self.telemetry_bar_throttle = b,
        "telemetry_bar_steer" => self.telemetry_bar_steer = b,
        "telemetry_dial" => self.telemetry_dial = b,
            _ => return false,
        }
        true
    }
}

impl PitLayout {
    fn apply_key(&mut self, key: &str, val: &str, _b: bool) -> bool {
        match key {
        "pit_sponsor" => self.pit_sponsor = val.trim().to_string(),
        "pit_art" => self.pit_art = val.trim().to_string(),
        "pit_board" => self.pit_board = val.trim().to_string(),
        "pit_text" => self.pit_text = TableText::parse(val),
        "pit_vars" => self.pit_vars = parse_places(val),
        "pit_when" => self.pit_when = PitWhen::parse(val),
        "pit_yellow" => {
            if let Some(rgb) = parse_primary_color(val) {
                self.pit_yellow = rgb;
            }
        }
        "pit_blue" => {
            if let Some(rgb) = parse_primary_color(val) {
                self.pit_blue = rgb;
            }
        }            _ => return false,
        }
        true
    }
}

fn apply_layout_key(cfg: &mut HudLayout, key: &str, val: &str, b: bool, saw_last_cols: &mut bool) {
    if cfg.sector.apply_key(key, val, b) {
        return;
    }
    if cfg.delta.apply_key(key, val, b) {
        return;
    }
    if cfg.flag.apply_key(key, val, b) {
        return;
    }
    if cfg.standings.apply_key(key, val, b, saw_last_cols) {
        return;
    }
    if cfg.relative.apply_key(key, val, b) {
        return;
    }
    if cfg.ticker.apply_key(key, val, b) {
        return;
    }
    if cfg.map.apply_key(key, val, b) {
        return;
    }
    if cfg.mini.apply_key(key, val, b) {
        return;
    }
    if cfg.radar.apply_key(key, val, b) {
        return;
    }
    if cfg.dash.apply_key(key, val, b) {
        return;
    }
    if cfg.timer.apply_key(key, b) {
        return;
    }
    if cfg.stance.apply_key(key, val, b) {
        return;
    }
    if cfg.lean.apply_key(key, val, b) {
        return;
    }
    if cfg.gamepad.apply_key(key, val, b) {
        return;
    }
    if cfg.sys.apply_key(key, val, b) {
        return;
    }
    if cfg.telemetry.apply_key(key, val, b) {
        return;
    }
    if cfg.pit.apply_key(key, val, b) {
        return;
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
    let pit = l[WidgetId::Pitboard];
    let timer = l[WidgetId::Timer];
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
         pitboard_x={}\npitboard_y={}\npitboard_w={}\npitboard_h={}\n\
         timer_x={}\ntimer_y={}\ntimer_w={}\ntimer_h={}\n\
         show_standings={}\nshow_relative={}\nshow_map={}\nshow_minimap={}\nshow_radar={}\n\
         show_dash={}\nshow_ticker={}\nshow_sys={}\nshow_sector={}\nshow_delta={}\n\
         show_stance={}\nshow_flag={}\nshow_lean={}\nshow_gamepad={}\nshow_telemetry={}\nshow_pitboard={}\nshow_timer={}\n\
         standings_rows={}\nrelative_count={}\nticker_count={}\n\
         st_pos={}\nst_num={}\nst_name={}\nst_gap={}\nst_interval={}\nst_laps={}\nst_current={}\n\
         st_best={}\nst_last={}\nst_status={}\nst_bike={}\nst_penalty={}\nst_crashed={}\nst_category={}\nst_lapdiff={}\n\
         st_order={}\n\
         st_w_pos={}\nst_w_num={}\nst_w_name={}\nst_w_gap={}\nst_w_interval={}\nst_w_laps={}\n\
         st_w_current={}\nst_w_best={}\nst_w_last={}\nst_w_status={}\nst_w_bike={}\nst_w_penalty={}\nst_w_crashed={}\nst_w_category={}\nst_w_lapdiff={}\n\
         st_bg={}\nst_hl={}\nst_text={}\nst_stripe={}\nst_plaque_text={}\nst_plaque={}\nst_font={}\nst_bold={}\n\
         st_head={}\nst_foot={}\n\
         rel_num={}\nrel_name={}\nrel_gap={}\nrel_laps={}\nrel_current={}\nrel_pos={}\nrel_bike={}\n\
         rel_penalty={}\nrel_interval={}\nrel_status={}\nrel_best={}\nrel_last={}\nrel_category={}\nrel_speed={}\nrel_lapdiff={}\n\
         rel_order={}\n\
         rel_w_num={}\nrel_w_name={}\nrel_w_gap={}\nrel_w_laps={}\nrel_w_current={}\nrel_w_pos={}\n\
         rel_w_bike={}\nrel_w_penalty={}\nrel_w_interval={}\nrel_w_status={}\nrel_w_best={}\nrel_w_last={}\nrel_w_category={}\nrel_w_speed={}\nrel_w_lapdiff={}\n\
         rel_bg={}\nrel_hl={}\nrel_text={}\nrel_stripe={}\nrel_plaque_text={}\nrel_plaque={}\nrel_font={}\nrel_bold={}\n\
         rel_head={}\nrel_foot={}\n\
         map_others={}\nmap_sf={}\nmap_sectors={}\nmap_name={}\nmap_numbers={}\nmap_arrows={}\nmap_follow={}\nmap_zoom={}\n\
         map_crown={}\nmap_place={}\nmap_dot={}\nmap_you_text={}\nmap_dot_opacity={}\nmap_bg={}\nmap_font={}\nmap_bold={}\n\
         mini_others={}\nmini_sf={}\nmini_sectors={}\nmini_numbers={}\nmini_arrows={}\nmini_crown={}\n\
         mini_place={}\nmini_dot={}\nmini_bg={}\nmini_zoom={}\nmini_font={}\nmini_bold={}\n\
         radar_sides={}\nradar_rear={}\nradar_rings={}\nradar_style={}\nradar_range={}\nradar_bg={}\nradar_font={}\nradar_bold={}\n\
         dash_rev={}\ndash_yellow={}\ndash_blue={}\ndash_red={}\ndash_simple={}\ndash_shift_color={}\ndash_left={}\ndash_mid={}\ndash_right={}\n\
         dash_bg={}\ndash_font={}\ndash_bold={}\n\
         ticker_left={}\nticker_right={}\nticker_title={}\nticker_autoscroll={}\nticker_status={}\nticker_slide={}\n\
         ticker_bg={}\nticker_hl={}\nticker_font={}\nticker_bold={}\n\
         sys_bg={}\nsys_font={}\nsys_bold={}\nsys_apps={}\n\
         sector_live={}\nsector_session={}\nsector_hist={}\nsector_hist_laps={}\n\
         sector_bg={}\nsector_font={}\nsector_bold={}\n\
         delta_session={}\ndelta_bg={}\ndelta_font={}\ndelta_bold={}\n\
         stance_mode={}\nstance_style={}\nstance_show_sit={}\n\
         stance_bg={}\nstance_font={}\nstance_bold={}\n\
         flag_bg={}\nflag_yellow={}\nflag_blue={}\nflag_red={}\nflag_text={}\nflag_font={}\nflag_bold={}\n\
         lean_style={}\nlean_bg={}\nlean_font={}\nlean_bold={}\n\
         gamepad_style={}\ngamepad_theme={}\ngamepad_bg={}\ngamepad_font={}\ngamepad_bold={}\n\
         telemetry_traces={}\ntelemetry_trace_throttle={}\ntelemetry_trace_brake={}\ntelemetry_trace_steer={}\n\
         telemetry_bars={}\ntelemetry_bar_clutch={}\ntelemetry_bar_brake={}\ntelemetry_bar_throttle={}\ntelemetry_bar_steer={}\n\
         telemetry_dial={}\n\
         telemetry_bg={}\ntelemetry_font={}\ntelemetry_bold={}\n\
         pit_sponsor={}\npit_art={}\npit_board={}\npit_text={}\npit_vars={}\npit_when={}\n\
         pit_yellow={}\npit_blue={}\n\
         pit_bg={}\npit_font={}\npit_bold={}\n\
         timer_bg={}\ntimer_font={}\ntimer_bold={}\ntimer_of={}",
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
        pit.rect.x, pit.rect.y, pit.rect.w, pit.rect.h,
        timer.rect.x, timer.rect.y, timer.rect.w, timer.rect.h,
        ini_flag(st.show), ini_flag(rel.show), ini_flag(map.show), ini_flag(mini.show), ini_flag(radar.show),
        ini_flag(dash.show), ini_flag(ticker.show), ini_flag(sys.show), ini_flag(sector.show), ini_flag(delta.show),
        ini_flag(stance.show), ini_flag(flag.show), ini_flag(lean.show), ini_flag(gamepad.show), ini_flag(telemetry.show), ini_flag(pit.show), ini_flag(timer.show),
        l.standings.standings_rows, l.relative.relative_count, l.ticker.ticker_count,
        ini_flag(l.standings.st_pos), ini_flag(l.standings.st_num), ini_flag(l.standings.st_name), ini_flag(l.standings.st_gap), ini_flag(l.standings.st_interval), ini_flag(l.standings.st_laps), ini_flag(l.standings.st_current),
        ini_flag(l.standings.st_best), ini_flag(l.standings.st_last), ini_flag(l.standings.st_status), ini_flag(l.standings.st_bike), ini_flag(l.standings.st_penalty), ini_flag(l.standings.st_crashed), ini_flag(l.standings.st_category), ini_flag(l.standings.st_lapdiff),
        join_st(&l.standings.st_order),
        l.standings.st_w_pos, l.standings.st_w_num, l.standings.st_w_name, l.standings.st_w_gap, l.standings.st_w_interval, l.standings.st_w_laps,
        l.standings.st_w_current, l.standings.st_w_best, l.standings.st_w_last, l.standings.st_w_status, l.standings.st_w_bike, l.standings.st_w_penalty, l.standings.st_w_crashed, l.standings.st_w_category, l.standings.st_w_lapdiff,
        st.bg, l.standings.st_hl, l.standings.st_text.key(), ini_flag(l.standings.st_stripe), l.standings.st_plaque_text.key(), ini_flag(l.standings.st_plaque), st.font, ini_flag(st.bold),
        join_board(&l.standings.st_head), join_board(&l.standings.st_foot),
        ini_flag(l.relative.rel_num), ini_flag(l.relative.rel_name), ini_flag(l.relative.rel_gap), ini_flag(l.relative.rel_laps), ini_flag(l.relative.rel_current), ini_flag(l.relative.rel_pos), ini_flag(l.relative.rel_bike),
        ini_flag(l.relative.rel_penalty), ini_flag(l.relative.rel_interval), ini_flag(l.relative.rel_status), ini_flag(l.relative.rel_best), ini_flag(l.relative.rel_last), ini_flag(l.relative.rel_category), ini_flag(l.relative.rel_speed), ini_flag(l.relative.rel_lapdiff),
        join_rel(&l.relative.rel_order),
        l.relative.rel_w_num, l.relative.rel_w_name, l.relative.rel_w_gap, l.relative.rel_w_laps, l.relative.rel_w_current, l.relative.rel_w_pos,
        l.relative.rel_w_bike, l.relative.rel_w_penalty, l.relative.rel_w_interval, l.relative.rel_w_status, l.relative.rel_w_best, l.relative.rel_w_last,
        l.relative.rel_w_category, l.relative.rel_w_speed, l.relative.rel_w_lapdiff,
        rel.bg, l.relative.rel_hl, l.relative.rel_text.key(), ini_flag(l.relative.rel_stripe), l.relative.rel_plaque_text.key(), ini_flag(l.relative.rel_plaque), rel.font, ini_flag(rel.bold),
        join_board(&l.relative.rel_head), join_board(&l.relative.rel_foot),
        ini_flag(l.map.map_others), ini_flag(l.map.map_sf), ini_flag(l.map.map_sectors), ini_flag(l.map.map_name), ini_flag(l.map.map_numbers), ini_flag(l.map.map_arrows),
        ini_flag(l.map.map_follow), l.map.map_zoom,
        ini_flag(l.map.map_crown), ini_flag(l.map.map_place), l.map.map_dot.key(), l.map.map_you_text.key(), l.map.map_dot_opacity, map.bg, map.font, ini_flag(map.bold),
        ini_flag(l.mini.mini_others), ini_flag(l.mini.mini_sf), ini_flag(l.mini.mini_sectors), ini_flag(l.mini.mini_numbers), ini_flag(l.mini.mini_arrows), ini_flag(l.mini.mini_crown),
        ini_flag(l.mini.mini_place), l.mini.mini_dot.key(), mini.bg, l.mini.mini_zoom, mini.font, ini_flag(mini.bold),
        ini_flag(l.radar.radar_sides), ini_flag(l.radar.radar_rear), ini_flag(l.radar.radar_rings), l.radar.radar_style.key(), l.radar.radar_range, radar.bg, radar.font, ini_flag(radar.bold),
        ini_flag(l.dash.dash_rev), ini_flag(l.dash.dash_yellow), ini_flag(l.dash.dash_blue), ini_flag(l.dash.dash_red), ini_flag(l.dash.dash_simple), ini_flag(l.dash.dash_shift_color), l.dash.dash_left.key(), l.dash.dash_mid.key(), l.dash.dash_right.key(),
        dash.bg, dash.font, ini_flag(dash.bold),
        l.ticker.ticker_left.key(), l.ticker.ticker_right.key(), ini_flag(l.ticker.ticker_title), ini_flag(l.ticker.ticker_autoscroll), ini_flag(l.ticker.ticker_status), ini_flag(l.ticker.ticker_slide),
        ticker.bg, l.ticker.ticker_hl, ticker.font, ini_flag(ticker.bold),
        sys.bg, sys.font, ini_flag(sys.bold), encode_sys_apps(&l.sys.sys_apps),
        ini_flag(l.sector.sector_live), ini_flag(l.sector.sector_session), ini_flag(l.sector.sector_hist), l.sector.sector_hist_laps.clamp(1, 5),
        sector.bg, sector.font, ini_flag(sector.bold),
        ini_flag(l.delta.delta_session), delta.bg, delta.font, ini_flag(delta.bold),
        l.stance.stance_mode.key(), l.stance.stance_style.key(), ini_flag(l.stance.stance_show_sit),
        stance.bg, stance.font, ini_flag(stance.bold),
        flag.bg, ini_flag(l.flag.flag_yellow), ini_flag(l.flag.flag_blue), ini_flag(l.flag.flag_red), ini_flag(l.flag.flag_text), flag.font, ini_flag(flag.bold),
        l.lean.lean_style.key(), lean.bg, lean.font, ini_flag(lean.bold),
        l.gamepad.gamepad_style.key(),
        l.gamepad.gamepad_theme.key(),
        gamepad.bg,
        gamepad.font,
        ini_flag(gamepad.bold),
        ini_flag(l.telemetry.telemetry_traces), ini_flag(l.telemetry.telemetry_trace_throttle), ini_flag(l.telemetry.telemetry_trace_brake), ini_flag(l.telemetry.telemetry_trace_steer),
        ini_flag(l.telemetry.telemetry_bars), ini_flag(l.telemetry.telemetry_bar_clutch), ini_flag(l.telemetry.telemetry_bar_brake), ini_flag(l.telemetry.telemetry_bar_throttle), ini_flag(l.telemetry.telemetry_bar_steer),
        ini_flag(l.telemetry.telemetry_dial),
        telemetry.bg, telemetry.font, ini_flag(telemetry.bold),
        ini_line(&l.pit.pit_sponsor),
        ini_line(&l.pit.pit_art),
        ini_line(&l.pit.pit_board),
        l.pit.pit_text.key(),
        encode_places(&l.pit.pit_vars),
        l.pit.pit_when.key(),
        format_primary_color(l.pit.pit_yellow),
        format_primary_color(l.pit.pit_blue),
        pit.bg, pit.font, ini_flag(pit.bold),
        timer.bg, timer.font, ini_flag(timer.bold), ini_flag(l.timer.timer_of),
    )
}

fn ini_flag(v: bool) -> i32 {
    i32::from(v)
}

fn ini_line(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\n' | '\r' | '=' => ' ',
            c => c,
        })
        .collect()
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
    val.parse()
        .unwrap_or(RADAR_RANGE_DEFAULT)
        .clamp(RADAR_RANGE_MIN, RADAR_RANGE_MAX)
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
    normalize(
        s.split(',').filter_map(|p| StField::parse(p.trim())),
        &StField::ALL,
    )
}

fn parse_rel_order(s: &str) -> Vec<RelField> {
    normalize(
        s.split(',').filter_map(|p| RelField::parse(p.trim())),
        &RelField::ALL,
    )
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
    Delta,
    Last,
    LapDiff,
    Current,
    Gap,
    Engine,
    Penalty,
    Server,
}

impl BoardField {
    pub const ALL: [Self; 27] = [
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
        Self::Delta,
        Self::Last,
        Self::LapDiff,
        Self::Current,
        Self::Gap,
        Self::Engine,
        Self::Penalty,
        Self::Server,
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
            Self::Delta => "delta",
            Self::Last => "last",
            Self::LapDiff => "lapdiff",
            Self::Current => "cur",
            Self::Gap => "gap",
            Self::Engine => "eng",
            Self::Penalty => "pen",
            Self::Server => "server",
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
            Self::Delta => "Delta",
            Self::Last => "Last lap",
            Self::LapDiff => "Last lap diff",
            Self::Current => "Current lap",
            Self::Gap => "Gap to leader",
            Self::Engine => "Engine temp",
            Self::Penalty => "Penalty",
            Self::Server => "Server",
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
            "delta" => Self::Delta,
            "last" => Self::Last,
            "lapdiff" | "lap_diff" => Self::LapDiff,
            "cur" | "current" => Self::Current,
            "gap" => Self::Gap,
            "eng" | "engine" => Self::Engine,
            "pen" | "penalty" => Self::Penalty,
            "server" => Self::Server,
            _ => Self::None,
        }
    }

    pub fn icon(self) -> char {
        match self {
            Self::None => '\0',
            Self::Position | Self::ClassPos => '\u{f091}',
            Self::Session
            | Self::RaceTime
            | Self::LocalTime
            | Self::Last
            | Self::Current
            | Self::Best
            | Self::SessionBest
            | Self::LapDiff => '\u{f2f2}',
            Self::Lap | Self::LapsLeft => '\u{f1da}',
            Self::Track | Self::Riders => '\u{f553}',
            Self::Air => '\u{f72e}',
            Self::Delta => '\u{f362}',
            Self::Gap | Self::GapAhead => '\u{f062}',
            Self::GapBehind => '\u{f063}',
            Self::SessionType => '\u{f11e}',
            Self::Fuel | Self::FuelPct => '\u{f52f}',
            Self::Setup => '\u{f0ad}',
            Self::Engine => '\u{f2c9}',
            Self::Penalty => '\u{f06a}',
            Self::Server => '\u{f233}',
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
    LapDiff,
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
    pub const ALL: [Self; 26] = [
        Self::None,
        Self::Speed,
        Self::Rpm,
        Self::Gear,
        Self::Position,
        Self::Number,
        Self::LapCount,
        Self::LapsLeft,
        Self::Last,
        Self::LapDiff,
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
            Self::LapDiff => "lapdiff",
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
            Self::LapDiff => "Last lap diff",
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
            "lapdiff" | "lap_diff" => Self::LapDiff,
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
            Self::Last
            | Self::LapDiff
            | Self::Best
            | Self::Current
            | Self::Session
            | Self::LocalTime => '\u{f2f2}',
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

fn rect_is(r: Rect, x: f32, y: f32, w: f32, h: f32) -> bool {
    (r.x - x).abs() < 0.001
        && (r.y - y).abs() < 0.001
        && (r.w - w).abs() < 0.001
        && (r.h - h).abs() < 0.001
}

/// Untouched 21%×34% factory map becomes a 16:9 square. A dragged box stays.
fn migrate_default_map(r: &mut Rect) {
    if rect_is(*r, 0.775, 0.62, 0.21, 0.34) {
        *r = WidgetId::Map.default_rect();
    }
}

/// Untouched 24%×20% factory pad becomes the DualShock aspect. A dragged box stays.
fn migrate_default_gamepad(r: &mut Rect) {
    if rect_is(*r, 0.38, 0.76, 0.24, 0.20) {
        *r = WidgetId::Gamepad.default_rect();
    }
}

/// Untouched 32%×22% factory plate matches holeshot.png (1600×1100) on 16:9.
fn migrate_default_pitboard(r: &mut Rect) {
    if rect_is(*r, 0.34, 0.05, 0.32, 0.22) {
        *r = WidgetId::Pitboard.default_rect();
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
    let mut p = ini_path();
    p.set_file_name("mxbo.ini");
    p
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
