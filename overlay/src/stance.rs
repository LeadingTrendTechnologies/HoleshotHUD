//! Sit/stand from a local pad, key, or mouse. MX Bikes does not publish posture.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use hidapi::{HidApi, HidDevice};
use mxbo_hud::config::{StanceBind, StanceMode};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_LBUTTON, VK_MBUTTON, VK_RBUTTON, VK_XBUTTON1, VK_XBUTTON2,
};
use windows::Win32::UI::Input::XboxController::{
    XInputGetState, XINPUT_GAMEPAD_A, XINPUT_GAMEPAD_B, XINPUT_GAMEPAD_DPAD_DOWN,
    XINPUT_GAMEPAD_DPAD_LEFT, XINPUT_GAMEPAD_DPAD_RIGHT, XINPUT_GAMEPAD_DPAD_UP,
    XINPUT_GAMEPAD_LEFT_SHOULDER, XINPUT_GAMEPAD_RIGHT_SHOULDER, XINPUT_GAMEPAD_X,
    XINPUT_GAMEPAD_Y, XINPUT_STATE,
};

const SONY_VID: u16 = 0x054C;
const DS_PID: u16 = 0x0CE6;
const DS_EDGE_PID: u16 = 0x0DF2;
const DS4_V1_PID: u16 = 0x05C4;
const DS4_V2_PID: u16 = 0x09CC;
const DS4_DONGLE_PID: u16 = 0x0BA0;

const TRIGGER_DOWN: u8 = 30;

static RESET: AtomicBool = AtomicBool::new(false);

pub fn reset_standing() {
    RESET.store(true, Ordering::SeqCst);
    mxbo_hud::set_stance(false);
}

#[derive(Clone, Copy, Default)]
struct Snap {
    pad: u16,
    mouse: u8,
    keys: [u8; 32],
}

pub struct Tracker {
    prev_down: bool,
    sitting: bool,
    capturing: bool,
    ignore: Snap,
    hid: HidPad,
}

impl Default for Tracker {
    fn default() -> Self {
        Self {
            prev_down: false,
            sitting: false,
            capturing: false,
            ignore: Snap::default(),
            hid: HidPad::new(),
        }
    }
}

impl Tracker {
    /// Poll sit/stand, or capture the next pad / key / mouse press while settings is listening.
    pub fn tick(&mut self, bind: StanceBind, mode: StanceMode, listen: bool) -> Option<StanceBind> {
        if RESET.swap(false, Ordering::SeqCst) {
            self.sitting = false;
        }
        let now = self.snapshot();
        let down = now.held(bind);

        if listen {
            if !self.capturing {
                self.capturing = true;
                self.ignore = now;
                mxbo_hud::set_stance(self.sitting);
                return None;
            }
            let picked = rising_input(&self.ignore, &now);
            self.ignore = now;
            if picked.is_some() {
                self.capturing = false;
                self.prev_down = true;
            }
            mxbo_hud::set_stance(self.sitting);
            return picked;
        }

        if self.capturing {
            self.capturing = false;
            self.prev_down = down;
            mxbo_hud::set_stance(self.sitting);
            return None;
        }

        let (next, prev) = apply_edge(self.sitting, self.prev_down, down, mode);
        self.sitting = next;
        self.prev_down = prev;
        mxbo_hud::set_stance(self.sitting);
        None
    }

    fn snapshot(&mut self) -> Snap {
        let pad = if let Some((buttons, lt, rt)) = xinput_pad() {
            mask_from_buttons(|b| xinput_held(buttons, lt, rt, b))
        } else {
            self.hid.held_mask()
        };
        Snap {
            pad,
            mouse: mouse_mask(),
            keys: key_bits(),
        }
    }
}

impl Snap {
    fn held(self, bind: StanceBind) -> bool {
        match bind {
            StanceBind::Key(vk) => key_on(&self.keys, vk),
            StanceBind::MouseLeft => self.mouse & (1 << 0) != 0,
            StanceBind::MouseRight => self.mouse & (1 << 1) != 0,
            StanceBind::MouseMiddle => self.mouse & (1 << 2) != 0,
            StanceBind::MouseX1 => self.mouse & (1 << 3) != 0,
            StanceBind::MouseX2 => self.mouse & (1 << 4) != 0,
            _ => self.pad & bind_bit(bind) != 0,
        }
    }
}

pub(crate) fn apply_edge(sitting: bool, prev_down: bool, down: bool, mode: StanceMode) -> (bool, bool) {
    match mode {
        StanceMode::Hold => (down, down),
        StanceMode::Toggle => {
            let next = if down && !prev_down { !sitting } else { sitting };
            (next, down)
        }
    }
}

fn rising_input(prev: &Snap, now: &Snap) -> Option<StanceBind> {
    if let Some(bind) = rising_bind(prev.pad, now.pad) {
        return Some(bind);
    }
    for (i, bind) in StanceBind::MOUSE.iter().enumerate() {
        let bit = 1u8 << i;
        if now.mouse & bit != 0 && prev.mouse & bit == 0 {
            return Some(*bind);
        }
    }
    let skip = skip_vk();
    for vk in 8u16..256 {
        if vk == skip || !capture_vk(vk) {
            continue;
        }
        if key_on(&now.keys, vk) && !key_on(&prev.keys, vk) {
            return Some(StanceBind::Key(vk));
        }
    }
    None
}

fn skip_vk() -> u16 {
    mxbo_hud::config::with_config(|c| c.settings_key.vk() as u16)
}

fn capture_vk(vk: u16) -> bool {
    !matches!(
        vk,
        0x01..=0x06
            | 0x1B
            | 0x10
            | 0x11
            | 0x12
            | 0x5B
            | 0x5C
            | 0x5D
    )
}

fn vk_down(vk: i32) -> bool {
    unsafe { GetAsyncKeyState(vk) < 0 }
}

fn mouse_mask() -> u8 {
    let mut m = 0u8;
    if vk_down(VK_LBUTTON.0 as i32) {
        m |= 1 << 0;
    }
    if vk_down(VK_RBUTTON.0 as i32) {
        m |= 1 << 1;
    }
    if vk_down(VK_MBUTTON.0 as i32) {
        m |= 1 << 2;
    }
    if vk_down(VK_XBUTTON1.0 as i32) {
        m |= 1 << 3;
    }
    if vk_down(VK_XBUTTON2.0 as i32) {
        m |= 1 << 4;
    }
    m
}

fn key_bits() -> [u8; 32] {
    let mut bits = [0u8; 32];
    for vk in 8u16..256 {
        if !capture_vk(vk) {
            continue;
        }
        if vk_down(vk as i32) {
            set_key(&mut bits, vk);
        }
    }
    bits
}

fn key_on(bits: &[u8; 32], vk: u16) -> bool {
    let i = vk as usize;
    i < 256 && bits[i / 8] & (1 << (i % 8)) != 0
}

fn set_key(bits: &mut [u8; 32], vk: u16) {
    let i = vk as usize;
    if i < 256 {
        bits[i / 8] |= 1 << (i % 8);
    }
}

fn xinput_mask(bind: StanceBind) -> u16 {
    match bind {
        StanceBind::PadRb => XINPUT_GAMEPAD_RIGHT_SHOULDER.0,
        StanceBind::PadLb => XINPUT_GAMEPAD_LEFT_SHOULDER.0,
        StanceBind::PadA => XINPUT_GAMEPAD_A.0,
        StanceBind::PadB => XINPUT_GAMEPAD_B.0,
        StanceBind::PadX => XINPUT_GAMEPAD_X.0,
        StanceBind::PadY => XINPUT_GAMEPAD_Y.0,
        StanceBind::PadDpadUp => XINPUT_GAMEPAD_DPAD_UP.0,
        StanceBind::PadDpadDown => XINPUT_GAMEPAD_DPAD_DOWN.0,
        StanceBind::PadDpadLeft => XINPUT_GAMEPAD_DPAD_LEFT.0,
        StanceBind::PadDpadRight => XINPUT_GAMEPAD_DPAD_RIGHT.0,
        _ => 0,
    }
}

fn xinput_held(buttons: u16, lt: u8, rt: u8, bind: StanceBind) -> bool {
    match bind {
        StanceBind::PadLt => lt > TRIGGER_DOWN,
        StanceBind::PadRt => rt > TRIGGER_DOWN,
        StanceBind::Key(_)
        | StanceBind::MouseLeft
        | StanceBind::MouseRight
        | StanceBind::MouseMiddle
        | StanceBind::MouseX1
        | StanceBind::MouseX2 => false,
        _ => buttons & xinput_mask(bind) != 0,
    }
}

fn bind_bit(bind: StanceBind) -> u16 {
    StanceBind::ALL
        .iter()
        .position(|&b| b == bind)
        .map(|i| 1u16 << i)
        .unwrap_or(0)
}

fn bind_from_mask(mask: u16) -> Option<StanceBind> {
    StanceBind::ALL.into_iter().find(|&b| mask & bind_bit(b) != 0)
}

fn rising_bind(prev: u16, now: u16) -> Option<StanceBind> {
    bind_from_mask(now & !prev)
}

fn mask_from_buttons(check: impl Fn(StanceBind) -> bool) -> u16 {
    StanceBind::ALL.iter().enumerate().fold(0u16, |m, (i, b)| {
        if check(*b) {
            m | (1 << i)
        } else {
            m
        }
    })
}

fn xinput_pad() -> Option<(u16, u8, u8)> {
    let mut state = XINPUT_STATE::default();
    for i in 0u32..4 {
        if unsafe { XInputGetState(i, &mut state) } == 0 {
            let g = state.Gamepad;
            return Some((g.wButtons.0, g.bLeftTrigger, g.bRightTrigger));
        }
    }
    None
}

#[derive(Clone, Copy)]
enum HidKind {
    DualSense,
    Ds4,
}

#[derive(Clone, Copy)]
struct DsPad {
    b0: u8,
    b1: u8,
    lt: u8,
    rt: u8,
}

fn ds_held(pad: DsPad, bind: StanceBind) -> bool {
    let hat = pad.b0 & 0x0F;
    match bind {
        StanceBind::PadX => pad.b0 & (1 << 4) != 0,
        StanceBind::PadA => pad.b0 & (1 << 5) != 0,
        StanceBind::PadB => pad.b0 & (1 << 6) != 0,
        StanceBind::PadY => pad.b0 & (1 << 7) != 0,
        StanceBind::PadLb => pad.b1 & (1 << 0) != 0,
        StanceBind::PadRb => pad.b1 & (1 << 1) != 0,
        StanceBind::PadLt => pad.b1 & (1 << 2) != 0 || pad.lt > TRIGGER_DOWN,
        StanceBind::PadRt => pad.b1 & (1 << 3) != 0 || pad.rt > TRIGGER_DOWN,
        StanceBind::PadDpadUp => matches!(hat, 0 | 1 | 7),
        StanceBind::PadDpadRight => matches!(hat, 1 | 2 | 3),
        StanceBind::PadDpadDown => matches!(hat, 3 | 4 | 5),
        StanceBind::PadDpadLeft => matches!(hat, 5 | 6 | 7),
        _ => false,
    }
}

fn ds_buttons(buf: &[u8]) -> Option<DsPad> {
    if buf.is_empty() {
        return None;
    }
    match buf[0] {
        0x01 if buf.len() > 9 => Some(DsPad {
            b0: buf[8],
            b1: buf[9],
            lt: buf.get(5).copied().unwrap_or(0),
            rt: buf.get(6).copied().unwrap_or(0),
        }),
        0x31 if buf.len() > 10 => Some(DsPad {
            b0: buf[9],
            b1: buf[10],
            lt: buf.get(6).copied().unwrap_or(0),
            rt: buf.get(7).copied().unwrap_or(0),
        }),
        _ if buf.len() > 8 => Some(DsPad {
            b0: buf[7],
            b1: buf[8],
            lt: buf.get(4).copied().unwrap_or(0),
            rt: buf.get(5).copied().unwrap_or(0),
        }),
        _ => None,
    }
}

fn ds4_buttons(buf: &[u8]) -> Option<DsPad> {
    if buf.is_empty() {
        return None;
    }
    match buf[0] {
        0x01 if buf.len() > 9 => Some(DsPad {
            b0: buf[5],
            b1: buf[6],
            lt: buf.get(8).copied().unwrap_or(0),
            rt: buf.get(9).copied().unwrap_or(0),
        }),
        0x11 if buf.len() > 11 => Some(DsPad {
            b0: buf[7],
            b1: buf[8],
            lt: buf.get(10).copied().unwrap_or(0),
            rt: buf.get(11).copied().unwrap_or(0),
        }),
        _ if buf.len() > 8 => Some(DsPad {
            b0: buf[5],
            b1: buf[6],
            lt: buf.get(8).copied().unwrap_or(0),
            rt: buf.get(9).copied().unwrap_or(0),
        }),
        _ => None,
    }
}

fn hid_kind(pid: u16) -> Option<HidKind> {
    match pid {
        DS_PID | DS_EDGE_PID => Some(HidKind::DualSense),
        DS4_V1_PID | DS4_V2_PID | DS4_DONGLE_PID => Some(HidKind::Ds4),
        _ => None,
    }
}

struct HidPad {
    api: Option<HidApi>,
    device: Option<HidDevice>,
    kind: HidKind,
    last: u16,
    next_scan: Instant,
}

impl HidPad {
    fn new() -> Self {
        Self {
            api: HidApi::new().ok(),
            device: None,
            kind: HidKind::DualSense,
            last: 0,
            next_scan: Instant::now(),
        }
    }

    fn held_mask(&mut self) -> u16 {
        self.ensure();
        let Some(dev) = self.device.as_mut() else {
            return 0;
        };
        let mut buf = [0u8; 128];
        match dev.read(&mut buf) {
            Ok(0) => self.last,
            Ok(n) => {
                let pad = match self.kind {
                    HidKind::DualSense => ds_buttons(&buf[..n]),
                    HidKind::Ds4 => ds4_buttons(&buf[..n]),
                };
                if let Some(pad) = pad {
                    self.last = mask_from_buttons(|bind| ds_held(pad, bind));
                }
                self.last
            }
            Err(_) => {
                self.device = None;
                self.last = 0;
                self.next_scan = Instant::now() + Duration::from_millis(400);
                0
            }
        }
    }

    fn ensure(&mut self) {
        if self.device.is_some() || Instant::now() < self.next_scan {
            return;
        }
        self.next_scan = Instant::now() + Duration::from_millis(800);
        let Some(api) = self.api.as_mut() else {
            return;
        };
        let _ = api.refresh_devices();
        let found = api.device_list().find_map(|info| {
            if info.vendor_id() != SONY_VID {
                return None;
            }
            hid_kind(info.product_id()).map(|kind| (info.path().to_owned(), kind))
        });
        let Some((path, kind)) = found else {
            return;
        };
        if let Ok(dev) = api.open_path(&path) {
            let _ = dev.set_blocking_mode(false);
            self.kind = kind;
            self.device = Some(dev);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_flips_on_press() {
        let (sit, prev) = apply_edge(false, false, true, StanceMode::Toggle);
        assert!(sit);
        assert!(prev);
        let (sit, prev) = apply_edge(true, true, true, StanceMode::Toggle);
        assert!(sit);
        assert!(prev);
        let (sit, prev) = apply_edge(true, true, false, StanceMode::Toggle);
        assert!(sit);
        assert!(!prev);
        let (sit, _) = apply_edge(true, false, true, StanceMode::Toggle);
        assert!(!sit);
    }

    #[test]
    fn hold_follows_button() {
        assert_eq!(apply_edge(false, false, true, StanceMode::Hold), (true, true));
        assert_eq!(apply_edge(true, true, false, StanceMode::Hold), (false, false));
    }

    #[test]
    fn capture_takes_first_new_press() {
        let rb = bind_bit(StanceBind::PadRb);
        let a = bind_bit(StanceBind::PadA);
        assert_eq!(rising_bind(0, a), Some(StanceBind::PadA));
        assert_eq!(rising_bind(a, a), None);
        assert_eq!(rising_bind(rb, rb | a), Some(StanceBind::PadA));
        assert_eq!(rising_bind(0, rb | a), Some(StanceBind::PadRb));
    }

    #[test]
    fn capture_prefers_pad_then_mouse_then_key() {
        let mut prev = Snap::default();
        let mut now = Snap::default();
        now.mouse = 1;
        now.keys[0x20 / 8] |= 1 << (0x20 % 8);
        now.pad = bind_bit(StanceBind::PadA);
        assert_eq!(rising_input(&prev, &now), Some(StanceBind::PadA));
        now.pad = 0;
        assert_eq!(rising_input(&prev, &now), Some(StanceBind::MouseLeft));
        now.mouse = 0;
        assert_eq!(rising_input(&prev, &now), Some(StanceBind::Key(0x20)));
        prev.keys = now.keys;
        assert_eq!(rising_input(&prev, &now), None);
    }

    #[test]
    fn dualsense_usb_square_is_not_l1() {
        let mut usb = [0u8; 64];
        usb[0] = 0x01;
        usb[8] = 0x08 | (1 << 4);
        let pad = ds_buttons(&usb).unwrap();
        assert!(ds_held(pad, StanceBind::PadX));
        assert!(!ds_held(pad, StanceBind::PadLb));
        assert!(!ds_held(pad, StanceBind::PadDpadUp));
        usb[8] = 0x08;
        usb[9] = 1;
        let pad = ds_buttons(&usb).unwrap();
        assert!(ds_held(pad, StanceBind::PadLb));
        assert!(!ds_held(pad, StanceBind::PadX));
        assert!(!ds_held(pad, StanceBind::PadRb));
    }

    #[test]
    fn dualsense_hat_and_l2() {
        let mut usb = [0u8; 64];
        usb[0] = 0x01;
        usb[8] = 0;
        let pad = ds_buttons(&usb).unwrap();
        assert!(ds_held(pad, StanceBind::PadDpadUp));
        assert!(!ds_held(pad, StanceBind::PadDpadDown));
        usb[8] = 2;
        let pad = ds_buttons(&usb).unwrap();
        assert!(ds_held(pad, StanceBind::PadDpadRight));
        usb[8] = 0x08;
        usb[9] = 1 << 2;
        let pad = ds_buttons(&usb).unwrap();
        assert!(ds_held(pad, StanceBind::PadLt));
        assert!(!ds_held(pad, StanceBind::PadRt));
        usb[9] = 1 << 3;
        let pad = ds_buttons(&usb).unwrap();
        assert!(ds_held(pad, StanceBind::PadRt));
        usb[9] = 0;
        usb[5] = 40;
        let pad = ds_buttons(&usb).unwrap();
        assert!(ds_held(pad, StanceBind::PadLt));
    }

    #[test]
    fn ds4_usb_square_and_l2() {
        let mut usb = [0u8; 64];
        usb[0] = 0x01;
        usb[5] = 0x08 | (1 << 4);
        let pad = ds4_buttons(&usb).unwrap();
        assert!(ds_held(pad, StanceBind::PadX));
        assert!(!ds_held(pad, StanceBind::PadDpadUp));
        usb[5] = 0x08;
        usb[6] = 1 << 2;
        usb[8] = 40;
        let pad = ds4_buttons(&usb).unwrap();
        assert!(ds_held(pad, StanceBind::PadLt));
        assert!(!ds_held(pad, StanceBind::PadRt));
    }

    #[test]
    fn xinput_trigger_and_dpad() {
        assert!(xinput_held(XINPUT_GAMEPAD_DPAD_LEFT.0, 0, 0, StanceBind::PadDpadLeft));
        assert!(!xinput_held(0, 0, 0, StanceBind::PadLt));
        assert!(xinput_held(0, 40, 0, StanceBind::PadLt));
        assert!(xinput_held(0, 0, 40, StanceBind::PadRt));
        assert!(!xinput_held(0, TRIGGER_DOWN, 0, StanceBind::PadLt));
        assert!(!xinput_held(0, 0, 0, StanceBind::Key(0x20)));
        assert!(!xinput_held(0, 0, 0, StanceBind::MouseLeft));
    }
}
