use super::*;
use crate::shm::{write_name, Rider, Snapshot};

fn snap() -> Snapshot {
    let mut s = Snapshot::default();
    s.has_telemetry = 1;
    s.local_race_num = 12;
    s.focus_race_num = 12;
    s.rider_count = 2;
    s.riders[0] = Rider {
        race_num: 1,
        lean: 18.0,
        ..Rider::default()
    };
    write_name(&mut s.riders[0].name, "Other");
    s.riders[1] = Rider {
        race_num: 12,
        lean: 10.0,
        ..Rider::default()
    };
    write_name(&mut s.riders[1].name, "You");
    s
}

#[test]
fn degrees_stay_degrees() {
    assert!((angle_deg(32.0) - 32.0).abs() < 0.01);
    assert!((angle_deg(1.0) - 1.0).abs() < 0.01, "upright 1° is not 1 rad");
    assert!((angle_deg(5.0) - 5.0).abs() < 0.01);
    assert_eq!(angle_deg(f32::NAN), 0.0);
    assert!((angle_deg(75.0) - 75.0).abs() < 0.01, "70°+ chassis lean is real");
    assert!((angle_deg(170.0) - 90.0).abs() < 0.01, "euler wrap must not snap to 0");
}

#[test]
fn airborne_uses_rider_lean_when_roll_pegs() {
    let mut s = snap();
    s.local_roll = 55.0;
    s.local_steer = 0.12;
    s.steer_lock = 0.40;
    s.riders[1].lean = 28.0;
    let v = view(&s);
    assert!((v.deg + 28.0).abs() < 0.2);
}

#[test]
fn ground_lean_past_sixty_uses_chassis() {
    let mut s = snap();
    s.local_roll = 75.0;
    s.local_steer = 0.12;
    s.steer_lock = 0.40;
    s.riders[1].lean = 40.0;
    let v = view(&s);
    assert!(
        (v.deg + 75.0).abs() < 0.2,
        "70°+ on the ground is chassis, not body"
    );
}

#[test]
fn steer_scales_by_lock() {
    let f = steer_frac(0.21, 0.42).expect("frac");
    assert!((f - 0.5).abs() < 0.01);
    let unit = steer_frac(-0.4, 0.0).expect("unit");
    assert!((unit + 0.4).abs() < 0.01);
    assert!(steer_frac(12.0, 0.0).is_none());
    let already = steer_frac(0.3, 25.0).expect("unit when lock is degrees");
    assert!((already - 0.3).abs() < 0.01);
}

#[test]
fn riding_uses_local_roll_and_steer() {
    let mut s = snap();
    s.local_roll = 32.0;
    s.local_steer = 0.12;
    s.steer_lock = 0.40;
    let v = view(&s);
    assert!((v.deg + 32.0).abs() < 0.2, "plugin positive is left; HUD is from behind");
    let steer = v.steer.expect("steer while riding");
    assert!((steer + 0.3).abs() < 0.02);
}

#[test]
fn spectate_follows_camera_lean_without_steer() {
    let mut s = snap();
    s.has_telemetry = 0;
    s.focus_race_num = 1;
    s.local_roll = 40.0;
    s.local_steer = 0.3;
    s.steer_lock = 0.4;
    let v = view(&s);
    assert!((v.deg + 18.0).abs() < 0.2);
    assert!(v.steer.is_none(), "other riders have no steer");
    assert!(v.pitch.is_none(), "other riders have no pitch");
}

#[test]
fn pitch_scales_to_clamp() {
    assert!((pitch_frac(30.0) - 0.5).abs() < 0.01);
    assert!((pitch_frac(-60.0) + 1.0).abs() < 0.01);
    assert!((pitch_frac(75.0) - 1.0).abs() < 0.01, "air pitch must not snap to 0");
}

#[test]
fn riding_flips_pitch_so_nose_up_is_positive() {
    let mut s = snap();
    s.local_roll = 32.0;
    s.local_pitch = 18.0;
    s.local_steer = 0.12;
    s.steer_lock = 0.40;
    let v = view(&s);
    let pitch = v.pitch.expect("pitch while riding");
    assert!(
        (pitch + 0.3).abs() < 0.02,
        "plugin +pitch is nose down; HUD nose-up is positive"
    );
    s.local_pitch = -18.0;
    let v = view(&s);
    let pitch = v.pitch.expect("pitch while riding");
    assert!((pitch - 0.3).abs() < 0.02, "nose up is +");
}
