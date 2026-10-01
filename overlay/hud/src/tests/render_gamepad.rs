use super::*;
use crate::config::{
    BoardField, DashField, FontFamily, HudConfig, LeanStyle, RelField, SessionPreset,
    StField, StanceStyle, WidgetId,
};
use crate::race_store::{
    effective_extra_laps, effective_race_laps, is_practice_session, live_session, session_preset,
    ClockMode,
};
use crate::shm::{write_name, Point, Rider, Snapshot, Standing, MAGIC, TRACK_NAME, VERSION};
use tiny_skia::{Color, Pixmap, Rect};

use super::render_support::*;
use super::render_support::draw;

#[test]
fn gamepad_goldens() {
    let _g = session_lock();
    reset_session();
    let base = live_snap();
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Gamepad].show = true;
    crate::gamepad::set(crate::gamepad::demo_sony());
    let s = golden_snap(&base, &cfg);
    cfg.gamepad.gamepad_theme = crate::config::GamepadTheme::Dark;
    draw_widget_golden("gamepad", &s, &cfg, cfg[WidgetId::Gamepad].rect);
    cfg.gamepad.gamepad_theme = crate::config::GamepadTheme::Light;
    draw_widget_golden("gamepad-ds4-light", &s, &cfg, cfg[WidgetId::Gamepad].rect);
    crate::gamepad::set(crate::gamepad::demo_xbox());
    draw_widget_golden("gamepad-xbox", &s, &cfg, cfg[WidgetId::Gamepad].rect);
    cfg.gamepad.gamepad_theme = crate::config::GamepadTheme::Dark;
    draw_widget_golden("gamepad-xbox-dark", &s, &cfg, cfg[WidgetId::Gamepad].rect);
    cfg.gamepad.gamepad_theme = crate::config::GamepadTheme::Light;
    crate::gamepad::set(crate::gamepad::PadState::DISCONNECTED);
    draw_widget_golden("gamepad-none", &s, &cfg, cfg[WidgetId::Gamepad].rect);
}

#[test]
fn xbox_press_covers_the_whole_control() {
    let _g = session_lock();
    reset_session();
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Gamepad].show = true;
    let s = golden_snap(&live_snap(), &cfg);
    let idle = crate::gamepad::PadState {
        kind: crate::gamepad::PadKind::Xbox,
        ..crate::gamepad::PadState::DISCONNECTED
    };
    let shot = |pad| {
        crate::gamepad::set(pad);
        let mut px = Pixmap::new(1280, 720).expect("pixmap");
        draw(
            &mut px,
            &fonts(),
            Some(&s),
            &cfg,
            1280,
            720,
            0.0,
            false,
            false,
            false,
        );
        px
    };
    // [x0, y0, x1, y1] of everything matching `pick`, or None.
    let bounds = |px: &Pixmap, pick: &dyn Fn([u8; 4]) -> bool| {
        let mut b: Option<[u32; 4]> = None;
        for y in 0..px.height() {
            for x in 0..px.width() {
                let i = ((y * px.width() + x) * 4) as usize;
                let d = px.data();
                if pick([d[i], d[i + 1], d[i + 2], d[i + 3]]) {
                    b = Some(match b {
                        None => [x, y, x, y],
                        Some(p) => [p[0].min(x), p[1].min(y), p[2].max(x), p[3].max(y)],
                    });
                }
            }
        }
        b
    };
    let orange = |p: [u8; 4]| p[3] > 200 && p[0] > 200 && (120..190).contains(&p[1]) && p[2] < 90;
    let ink = |p: [u8; 4]| p[3] > 200 && p[0] < 45 && p[1] < 45 && p[2] < 45;
    let idle_px = shot(idle);
    let pressed_bounds = |px: &Pixmap, before: &Pixmap| {
        let (d, b) = (px.data(), before.data());
        let mut out: Option<[u32; 4]> = None;
        for y in 0..px.height() {
            for x in 0..px.width() {
                let i = ((y * px.width() + x) * 4) as usize;
                let now = [d[i], d[i + 1], d[i + 2], d[i + 3]];
                let was = [b[i], b[i + 1], b[i + 2], b[i + 3]];
                if orange(now) && now != was {
                    out = Some(match out {
                        None => [x, y, x, y],
                        Some(p) => [p[0].min(x), p[1].min(y), p[2].max(x), p[3].max(y)],
                    });
                }
            }
        }
        out
    };

    // A press must not leave the disc's rim unlit: a radius taken from the target
    // instead of the art used to leave a ring of ink around the orange.
    for (label, pad) in [
        (
            "A",
            crate::gamepad::PadState {
                buttons: crate::gamepad::SOUTH,
                ..idle
            },
        ),
        (
            "View",
            crate::gamepad::PadState {
                buttons: crate::gamepad::BACK,
                ..idle
            },
        ),
    ] {
        let px = shot(pad);
        // Only pixels the press turned orange: anti-aliased yellow (Y) edges also pass `orange`.
        let lit = pressed_bounds(&px, &idle_px).unwrap_or_else(|| panic!("{label} lit nothing"));
        let mut left = 0;
        for y in lit[1] - 2..=lit[3] + 2 {
            for x in lit[0] - 2..=lit[2] + 2 {
                if ink(sample_px(&px, x as f32, y as f32)) {
                    left += 1;
                }
            }
        }
        assert!(left <= 8, "{label}: {left} ink px left inside the press");
    }

    // A trigger held past the snap fills its tab to the very top.
    let top = bounds(&shot(idle), &ink).expect("idle pad draws ink")[1];
    let held = crate::gamepad::PadState { rt: 1.0, ..idle };
    let lit = pressed_bounds(&shot(held), &idle_px).expect("RT lit nothing");
    assert!(
        lit[1] <= top + 1,
        "RT: orange starts at {} but the tab starts at {top}",
        lit[1]
    );
}

#[test]
fn xbox_press_edges_are_antialiased() {
    let _g = session_lock();
    reset_session();
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Gamepad].show = true;
    let s = golden_snap(&live_snap(), &cfg);
    let idle = crate::gamepad::PadState {
        kind: crate::gamepad::PadKind::Xbox,
        ..crate::gamepad::PadState::DISCONNECTED
    };
    let shot = |pad| {
        crate::gamepad::set(pad);
        let mut px = Pixmap::new(1280, 720).expect("pixmap");
        draw(&mut px, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
        crate::gamepad::set(crate::gamepad::PadState::DISCONNECTED);
        px
    };
    let orange = |p: [u8; 4]| p[3] > 200 && p[0] > 200 && (120..190).contains(&p[1]) && p[2] < 90;
    let green = |p: [u8; 4]| {
        p[3] > 200 && p[1] > 150 && p[1] > p[0].saturating_add(40) && p[1] > p[2].saturating_add(40)
    };
    let before = shot(idle);
    // (pixels the press turned fully orange, partially lit edge pixels, green letter pixels)
    let scan = |px: &Pixmap| {
        let (d, b) = (px.data(), before.data());
        let (mut full, mut soft, mut letter) = (0, 0, 0);
        for i in (0..d.len()).step_by(4) {
            let now = [d[i], d[i + 1], d[i + 2], d[i + 3]];
            let was = [b[i], b[i + 1], b[i + 2], b[i + 3]];
            if now == was {
                continue;
            }
            if orange(now) {
                full += 1;
            } else if green(now) {
                letter += 1;
            } else {
                soft += 1;
            }
        }
        (full, soft, letter)
    };

    let (full, soft, _) = scan(&shot(crate::gamepad::PadState { rt: 1.0, ..idle }));
    assert!(full > 20, "RT: only {full} px lit");
    assert!(soft >= 6, "RT: only {soft} blended edge px — the fill edge is aliased");

    let (full, soft, _) = scan(&shot(crate::gamepad::PadState {
        buttons: crate::gamepad::SOUTH,
        ..idle
    }));
    assert!(full > 20, "A: only {full} px lit");
    assert!(soft >= 6, "A: only {soft} blended edge px — the disc edge is aliased");
    let mut letter = 0;
    let px = shot(crate::gamepad::PadState {
        buttons: crate::gamepad::SOUTH,
        ..idle
    });
    for p in px.data().chunks_exact(4) {
        letter += green([p[0], p[1], p[2], p[3]]) as u32;
    }
    assert!(letter >= 3, "A: {letter} green px — the letter should stay on the fill");
}

#[test]
fn ds4_press_edges_are_antialiased() {
    let _g = session_lock();
    reset_session();
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Gamepad].show = true;
    cfg.gamepad.gamepad_theme = crate::config::GamepadTheme::Dark;
    let s = golden_snap(&live_snap(), &cfg);
    let idle = crate::gamepad::PadState {
        kind: crate::gamepad::PadKind::Sony,
        ..crate::gamepad::PadState::DISCONNECTED
    };
    let shot = |pad| {
        crate::gamepad::set(pad);
        let mut px = Pixmap::new(1280, 720).expect("pixmap");
        draw(&mut px, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
        crate::gamepad::set(crate::gamepad::PadState::DISCONNECTED);
        px
    };
    let orange = |p: [u8; 4]| p[3] > 200 && p[0] > 200 && (120..190).contains(&p[1]) && p[2] < 90;
    let blended =
        |p: [u8; 4]| p[3] > 200 && (60..200).contains(&p[0]) && p[0] > p[1].saturating_add(20);
    let cream = |p: [u8; 4]| p[3] > 200 && p[0].min(p[1]).min(p[2]) > 170;
    let scan = |px: &Pixmap| {
        let d = px.data();
        let at = |x: u32, y: u32| {
            let i = ((y * px.width() + x) * 4) as usize;
            [d[i], d[i + 1], d[i + 2], d[i + 3]]
        };
        let mut lit: Option<[u32; 4]> = None;
        for y in 0..px.height() {
            for x in 0..px.width() {
                if orange(at(x, y)) {
                    lit = Some(match lit {
                        None => [x, y, x, y],
                        Some(b) => [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)],
                    });
                }
            }
        }
        let b = lit.expect("press lit nothing");
        let (mut soft, mut light) = (0, 0);
        for y in b[1]..=b[3] {
            for x in b[0]..=b[2] {
                let p = at(x, y);
                soft += blended(p) as u32;
                light += cream(p) as u32;
            }
        }
        (soft, light)
    };

    let (soft, light) = scan(&shot(crate::gamepad::PadState { rt: 1.0, ..idle }));
    assert!(soft >= 6, "R2: only {soft} blended edge px — the fill edge is aliased");
    assert_eq!(light, 0, "R2: {light} cream px inside the fill — the label should be gone");

    let (soft, light) = scan(&shot(crate::gamepad::PadState {
        buttons: crate::gamepad::SOUTH,
        ..idle
    }));
    assert!(soft >= 6, "Cross: only {soft} blended edge px — the disc edge is aliased");
    assert!(light >= 3, "Cross: {light} cream px — the × should stay on the fill");
}

#[test]
fn xbox_dark_press_fills_stay_inside_their_outlines() {
    let _g = session_lock();
    reset_session();
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Gamepad].show = true;
    cfg.gamepad.gamepad_theme = crate::config::GamepadTheme::Dark;
    let s = golden_snap(&live_snap(), &cfg);
    let idle = crate::gamepad::PadState {
        kind: crate::gamepad::PadKind::Xbox,
        ..crate::gamepad::PadState::DISCONNECTED
    };
    let shot = |pad| {
        crate::gamepad::set(pad);
        let mut px = Pixmap::new(1280, 720).expect("pixmap");
        draw(&mut px, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
        crate::gamepad::set(crate::gamepad::PadState::DISCONNECTED);
        px
    };
    let orange = |p: [u8; 4]| p[3] > 200 && p[0] > 200 && (120..190).contains(&p[1]) && p[2] < 90;
    // Outlined letters are under a pixel wide at widget size, so their cream only half-covers.
    let cream = |p: [u8; 4]| p[3] > 200 && p[0].min(p[1]).min(p[2]) > 120;
    let before = shot(idle);
    let pixel = |px: &Pixmap, i: usize| {
        let d = px.data();
        [d[i], d[i + 1], d[i + 2], d[i + 3]]
    };
    let mut pad_box: Option<[u32; 4]> = None;
    for y in 0..before.height() {
        for x in 0..before.width() {
            if pixel(&before, ((y * before.width() + x) * 4) as usize)[3] > 200 {
                pad_box = Some(match pad_box {
                    None => [x, y, x, y],
                    Some(b) => [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)],
                });
            }
        }
    }
    let pad_box = pad_box.expect("idle dark pad draws");
    let pad_w = (pad_box[2] - pad_box[0]) as f32;
    // (bounds of pixels the press turned orange, fully orange px, blended px, cream px in bounds)
    let scan = |px: &Pixmap| {
        let mut lit: Option<[u32; 4]> = None;
        let (mut full, mut soft) = (0, 0);
        for y in 0..px.height() {
            for x in 0..px.width() {
                let i = ((y * px.width() + x) * 4) as usize;
                let (now, was) = (pixel(px, i), pixel(&before, i));
                if now == was {
                    continue;
                }
                if orange(now) {
                    full += 1;
                    lit = Some(match lit {
                        None => [x, y, x, y],
                        Some(b) => [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)],
                    });
                } else {
                    soft += 1;
                }
            }
        }
        let lit = lit.expect("press lit nothing");
        let mut light = 0;
        for y in lit[1]..=lit[3] {
            for x in lit[0]..=lit[2] {
                light += cream(pixel(px, ((y * px.width() + x) * 4) as usize)) as u32;
            }
        }
        (lit, full, soft, light)
    };

    let (_, full, soft, _) = scan(&shot(crate::gamepad::PadState { rt: 1.0, ..idle }));
    assert!(full > 20, "RT: only {full} px lit");
    assert!(soft >= 6, "RT: only {soft} blended edge px — the fill edge is aliased");

    let (_, full, soft, light) = scan(&shot(crate::gamepad::PadState {
        buttons: crate::gamepad::SOUTH,
        ..idle
    }));
    assert!(full > 20, "A: only {full} px lit");
    assert!(soft >= 6, "A: only {soft} blended edge px — the disc edge is aliased");
    assert!(light >= 3, "A: {light} cream px — the letter should stay on the fill");

    // LB lights the same band as on the light skin.
    let lb = crate::gamepad::PadState {
        buttons: crate::gamepad::LB,
        ..idle
    };
    let (dark_lb, ..) = scan(&shot(lb));
    let mut light_cfg = cfg.clone();
    light_cfg.gamepad.gamepad_theme = crate::config::GamepadTheme::Light;
    let light_shot = |pad| {
        crate::gamepad::set(pad);
        let mut px = Pixmap::new(1280, 720).expect("pixmap");
        draw(&mut px, &fonts(), Some(&s), &light_cfg, 1280, 720, 0.0, false, false, false);
        crate::gamepad::set(crate::gamepad::PadState::DISCONNECTED);
        px
    };
    let light_idle = light_shot(idle);
    let light_lb = light_shot(lb);
    let mut light_lit: Option<[u32; 4]> = None;
    for y in 0..light_lb.height() {
        for x in 0..light_lb.width() {
            let i = ((y * light_lb.width() + x) * 4) as usize;
            let now = pixel(&light_lb, i);
            if orange(now) && now != pixel(&light_idle, i) {
                light_lit = Some(match light_lit {
                    None => [x, y, x, y],
                    Some(b) => [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)],
                });
            }
        }
    }
    let light_lb = light_lit.expect("light LB lit nothing");
    for side in 0..4 {
        assert!(
            dark_lb[side].abs_diff(light_lb[side]) <= 3,
            "LB: dark lit {dark_lb:?}, light lit {light_lb:?}"
        );
    }

    // The body is ink too: View lights inside its ring, not its whole layout rect.
    let view = shot(crate::gamepad::PadState {
        buttons: crate::gamepad::BACK,
        ..idle
    });
    let (lit, ..) = scan(&view);
    let view_w = (lit[2] - lit[0]) as f32 / pad_w;
    assert!(view_w < 0.052, "View: lit {view_w:.3} of the pad wide — spilled past its ring");
    for (x, y) in [(lit[0], lit[1]), (lit[2], lit[1]), (lit[0], lit[3]), (lit[2], lit[3])] {
        let corner = pixel(&view, ((y * view.width() + x) * 4) as usize);
        assert!(!orange(corner), "View: lit corner at ({x}, {y}) — a square fill, not the dot");
    }

    let (lit, ..) = scan(&shot(crate::gamepad::PadState {
        buttons: crate::gamepad::UP,
        ..idle
    }));
    let arm_w = (lit[2] - lit[0]) as f32 / pad_w;
    assert!(arm_w < 0.052, "Up: lit {arm_w:.3} of the pad wide — spilled past the cross arm");
}

#[test]
fn gamepad_theme_switches_playstation() {
    let _g = session_lock();
    reset_session();
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Gamepad].show = true;
    let s = golden_snap(&live_snap(), &cfg);
    crate::gamepad::set(crate::gamepad::demo_sony());
    let mut shot = |theme| {
        cfg.gamepad.gamepad_theme = theme;
        let mut px = Pixmap::new(1280, 720).expect("pixmap");
        draw(&mut px, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
        px
    };
    let light = shot(crate::config::GamepadTheme::Light);
    let dark = shot(crate::config::GamepadTheme::Dark);
    crate::gamepad::set(crate::gamepad::PadState::DISCONNECTED);
    assert!(light.data() != dark.data(), "Theme did not change the PlayStation pad");
}

#[test]
fn ds4_light_press_fills_stay_inside_their_outlines() {
    let _g = session_lock();
    reset_session();
    let mut cfg = HudConfig::new();
    hide_widgets(&mut cfg);
    cfg[WidgetId::Gamepad].show = true;
    cfg.gamepad.gamepad_theme = crate::config::GamepadTheme::Light;
    let s = golden_snap(&live_snap(), &cfg);
    let idle = crate::gamepad::PadState {
        kind: crate::gamepad::PadKind::Sony,
        ..crate::gamepad::PadState::DISCONNECTED
    };
    let shot = |pad| {
        crate::gamepad::set(pad);
        let mut px = Pixmap::new(1280, 720).expect("pixmap");
        draw(&mut px, &fonts(), Some(&s), &cfg, 1280, 720, 0.0, false, false, false);
        crate::gamepad::set(crate::gamepad::PadState::DISCONNECTED);
        px
    };
    let orange = |p: [u8; 4]| p[3] > 200 && p[0] > 200 && (120..190).contains(&p[1]) && p[2] < 90;
    let cream = |p: [u8; 4]| p[3] > 200 && p[0].min(p[1]).min(p[2]) > 120;
    let pixel = |px: &Pixmap, x: u32, y: u32| {
        let i = ((y * px.width() + x) * 4) as usize;
        let d = px.data();
        [d[i], d[i + 1], d[i + 2], d[i + 3]]
    };
    let before = shot(idle);
    let mut pad_box: Option<[u32; 4]> = None;
    for y in 0..before.height() {
        for x in 0..before.width() {
            if pixel(&before, x, y)[3] > 200 {
                pad_box = Some(match pad_box {
                    None => [x, y, x, y],
                    Some(b) => [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)],
                });
            }
        }
    }
    let pad_box = pad_box.expect("idle light pad draws");
    let pad_w = (pad_box[2] - pad_box[0]) as f32;
    // (bounds of pixels the press turned orange, fully orange px, blended px, cream px in bounds)
    let scan = |px: &Pixmap| {
        let mut lit: Option<[u32; 4]> = None;
        let (mut full, mut soft) = (0, 0);
        for y in 0..px.height() {
            for x in 0..px.width() {
                let (now, was) = (pixel(px, x, y), pixel(&before, x, y));
                if now == was {
                    continue;
                }
                if orange(now) {
                    full += 1;
                    lit = Some(match lit {
                        None => [x, y, x, y],
                        Some(b) => [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)],
                    });
                } else {
                    soft += 1;
                }
            }
        }
        let lit = lit.expect("press lit nothing");
        let mut light = 0;
        for y in lit[1]..=lit[3] {
            for x in lit[0]..=lit[2] {
                light += cream(pixel(px, x, y)) as u32;
            }
        }
        (lit, full, soft, light)
    };
    let r2 = crate::gamepad::PadState { rt: 1.0, ..idle };
    let (lit, full, soft, _) = scan(&shot(r2));
    assert!(full > 20, "R2: only {full} px lit");
    assert!(soft >= 6, "R2: only {soft} blended edge px — the fill edge is aliased");
    // Same drawing as the dark pad, so R2 lights the same trigger.
    let mut dark_cfg = cfg.clone();
    dark_cfg.gamepad.gamepad_theme = crate::config::GamepadTheme::Dark;
    let dark_shot = |pad| {
        crate::gamepad::set(pad);
        let mut px = Pixmap::new(1280, 720).expect("pixmap");
        draw(&mut px, &fonts(), Some(&s), &dark_cfg, 1280, 720, 0.0, false, false, false);
        crate::gamepad::set(crate::gamepad::PadState::DISCONNECTED);
        px
    };
    let (dark_idle, dark_r2) = (dark_shot(idle), dark_shot(r2));
    let mut dark_lit: Option<[u32; 4]> = None;
    for y in 0..dark_r2.height() {
        for x in 0..dark_r2.width() {
            let now = pixel(&dark_r2, x, y);
            if orange(now) && now != pixel(&dark_idle, x, y) {
                dark_lit = Some(match dark_lit {
                    None => [x, y, x, y],
                    Some(b) => [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)],
                });
            }
        }
    }
    let dark_lit = dark_lit.expect("dark R2 lit nothing");
    for side in 0..4 {
        assert!(
            lit[side].abs_diff(dark_lit[side]) <= 3,
            "R2: light lit {lit:?}, dark lit {dark_lit:?} — spilled past the trigger"
        );
    }

    let (_, full, _, light) = scan(&shot(crate::gamepad::PadState {
        buttons: crate::gamepad::SOUTH,
        ..idle
    }));
    assert!(full > 20, "Cross: only {full} px lit");
    assert!(light >= 3, "Cross: {light} cream px — the symbol should stay on the fill");

    let (lit, ..) = scan(&shot(crate::gamepad::PadState {
        buttons: crate::gamepad::UP,
        ..idle
    }));
    let arm_w = (lit[2] - lit[0]) as f32 / pad_w;
    assert!(arm_w < 0.06, "Up: lit {arm_w:.3} of the pad wide — spilled onto the d-pad panel");
}

