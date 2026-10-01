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
fn minimap_keeps_sparse_track_segments_near_the_rider() {
    let mut s = live_snap();
    s.poly_count = 4;
    s.poly[0] = Point { x: -200.0, z: 0.0 };
    s.poly[1] = Point { x: 200.0, z: 0.0 };
    s.poly[2] = Point { x: 200.0, z: 40.0 };
    s.poly[3] = Point { x: -200.0, z: 40.0 };
    let mut px = Pixmap::new(256, 256).expect("pixmap");
    let center = 128.5;
    stroke_smooth_minimap_track(
        &mut px,
        &s,
        4,
        0.0,
        0.0,
        Some(90.0),
        4.0,
        &|x, z| (center + x, center + z),
    );
    let on_track = px.pixel(128, 128).expect("center");
    assert!(
        on_track.alpha() > 20,
        "segment through the rider should stay visible"
    );
    let outside = px.pixel(128, 240).expect("outside radius");
    assert_eq!(
        outside.alpha(),
        0,
        "pixels well outside the 90 m radius stay clear"
    );
}

#[test]
fn minimap_zoom_stays_on_a_local_section() {
    assert_eq!(mini_view_radius(100), 22.0);
    assert_eq!(mini_view_radius(0), 85.0);
    assert!((mini_view_radius(70) - 40.9).abs() < 0.001);
}

#[test]
fn map_zoom_stops_wider_than_the_minimap() {
    let fit = 0.2;
    let w = 400.0;
    let h = 300.0;
    let usable = h * 0.8;
    let close = usable / MAP_ZOOM_WINDOW_M;
    assert!((map_follow_scale(fit, 0, w, h) - fit).abs() < 1e-4);
    assert!((map_follow_scale(fit, 100, w, h) - close).abs() < 1e-4);
    let window_m = usable / map_follow_scale(fit, 100, w, h);
    assert!((window_m - MAP_ZOOM_WINDOW_M).abs() < 0.01);
    assert!(window_m > mini_view_radius(100) * 2.0);
    let already_tight = close * 2.0;
    assert!((map_follow_scale(already_tight, 100, w, h) - already_tight).abs() < 1e-4);
}

#[test]
fn poly_at_frac_walks_centerline_distance() {
    let mut s = Snapshot::default();
    s.poly_count = 3;
    s.track_length = 200.0;
    s.poly[0] = Point { x: 0.0, z: 0.0 };
    s.poly[1] = Point { x: 100.0, z: 0.0 };
    s.poly[2] = Point { x: 100.0, z: 100.0 };
    let a = poly_at_frac(&s, 3, 0.25).expect("s1");
    assert!((a.wx - 50.0).abs() < 0.5, "wx {}", a.wx);
    assert!(a.wz.abs() < 0.5, "wz {}", a.wz);
    let b = poly_at_frac(&s, 3, 0.75).expect("s2");
    assert!((b.wx - 100.0).abs() < 0.5, "wx {}", b.wx);
    assert!((b.wz - 50.0).abs() < 0.5, "wz {}", b.wz);
    s.sf_meters = 50.0;
    let line = poly_at_frac(&s, 3, 0.0).expect("s1 starts at s/f");
    assert!((line.wx - 50.0).abs() < 0.5, "sf wx {}", line.wx);
    let s2 = poly_at_frac(&s, 3, 0.25).expect("s2 starts at first split");
    assert!((s2.wx - 100.0).abs() < 0.5, "s2 wx {}", s2.wx);
    assert!(s2.wz.abs() < 0.5, "s2 wz {}", s2.wz);
}

/// Map / minimap dot labels, the crown and the nearest ahead / behind rings all read
/// `standing_pos` and `leader_num`, so they move with a pass too.
#[test]
fn map_marks_follow_the_live_order() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.standing_count = 3;
    s.rider_count = 3;
    s.standings[2] = standing(7, 3, 5);
    s.riders[2] = rider(7, 30.0, 12.0, 0.0);
    // #7 was behind you; now they are 6 m up the track on you. The leader is clear.
    s.riders[0].track_pos = 0.600;
    s.riders[1].track_pos = 0.500;
    s.local_track_pos = 0.500;
    s.riders[2].track_pos = 0.506;
    s.session_kind = 7;
    s.session_laps = 2;
    s.session_length = 8;
    let _ = RaceStore::tick(&s);
    assert_eq!(leader_num(&s), 1);
    assert_eq!(standing_pos(&s, 7), 2);
    assert_eq!(standing_pos(&s, 12), 3);
    // The ring on #7 is now the green "ahead" mark, not the red one behind you.
    assert_eq!(standing_pos(&s, 7), standing_pos(&s, 12) - 1);
    assert!(
        place_rings_for_session(&s),
        "live race moto still shows ahead/behind rings"
    );
}

#[test]
fn place_rings_skip_warmup_and_practice() {
    let _g = session_lock();
    reset_session();
    let mut s = live_snap();
    s.session_kind = 5;
    s.session_laps = 0;
    s.session_length = 10;
    assert!(is_warmup(&s));
    assert!(!place_rings_for_session(&s), "warmup hides place rings");

    reset_session();
    s = live_snap();
    s.session_kind = -1;
    s.session_laps = 0;
    s.session_length = 0;
    assert!(is_practice_session(&s));
    assert!(!place_rings_for_session(&s), "practice hides place rings");

    reset_session();
    s = live_snap();
    s.session_kind = -1;
    s.session_laps = 0;
    s.session_length = 40;
    assert!(is_practice_session(&s));
    assert!(!place_rings_for_session(&s), "open practice hides place rings");

    reset_session();
    s = live_snap();
    s.session_kind = 7;
    s.session_laps = 2;
    s.session_length = 8;
    assert!(!is_warmup(&s));
    assert!(!is_practice_session(&s));
    assert!(place_rings_for_session(&s), "race moto keeps place rings");
}

#[test]
fn rider_map_pose_follows_a_cut_off_the_racing_line() {
    clear_map_xz_watch();
    let mut s = live_snap();
    s.riders[0].track_pos = 0.25;
    s.riders[0].yaw = 1.25;
    s.riders[0].x = 0.0;
    s.riders[0].z = 0.0;
    let _ = rider_map_pose(&s, &s.riders[0]);
    s.riders[0].x = 40.0;
    s.riders[0].z = -30.0;
    let cut = rider_map_pose(&s, &s.riders[0]);
    assert_eq!(cut.x, 40.0);
    assert_eq!(cut.z, -30.0);
    assert_eq!(cut.yaw, 1.25);
}

#[test]
fn rider_map_pose_follows_track_pos_when_world_xz_is_stale() {
    let _lock = session_lock();
    reset_session();
    clear_map_xz_watch();
    let mut s = live_snap();
    // One sample is not enough to call a far coordinate frozen.
    s.riders[0].x = 999.0;
    s.riders[0].z = 999.0;
    s.riders[0].track_pos = 0.10;
    let first = rider_map_pose(&s, &s.riders[0]);
    assert_eq!(first.x, 999.0);
    assert_eq!(first.z, 999.0);
    // Same XZ while progress runs: the dot walks the centerline instead of sticking.
    s.riders[0].track_pos = 0.60;
    let frozen = rider_map_pose(&s, &s.riders[0]);
    assert!(
        frozen.x.abs() < 200.0 && frozen.z.abs() < 200.0,
        "frozen xz falls back to the centerline"
    );
    assert!((frozen.x - 999.0).hypot(frozen.z - 999.0) > 1.0);
    s.riders[0].track_pos = 0.85;
    let walked = rider_map_pose(&s, &s.riders[0]);
    let moved = (frozen.x - walked.x).hypot(frozen.z - walked.z);
    assert!(
        moved > 20.0,
        "dot should keep walking the poly (moved {moved})"
    );
    assert!(walked.x.abs() < 200.0 && walked.z.abs() < 200.0, "walked on track");
}

#[test]
fn rider_map_pose_falls_back_to_world_xz_without_poly() {
    clear_map_xz_watch();
    let mut s = live_snap();
    s.poly_count = 0;
    s.riders[0].x = 33.0;
    s.riders[0].z = -12.0;
    s.riders[0].track_pos = 0.4;
    let pose = rider_map_pose(&s, &s.riders[0]);
    assert_eq!(pose.x, 33.0);
    assert_eq!(pose.z, -12.0);
}

#[test]
fn rider_map_pose_keeps_gate_stalls_on_world_xz() {
    let _lock = session_lock();
    reset_session();
    clear_map_xz_watch();
    let mut s = live_snap();
    s.local_speed = 0.0;
    // Same lap fraction, distinct stalls — poly sampling would pile them together.
    s.riders[0].track_pos = 0.0;
    s.riders[1].track_pos = 0.0;
    s.riders[0].x = -12.0;
    s.riders[0].z = 4.0;
    s.riders[1].x = 12.0;
    s.riders[1].z = 4.0;

    IN_GATE.store(1, Ordering::Relaxed);
    let a = rider_map_pose(&s, &s.riders[0]);
    let b = rider_map_pose(&s, &s.riders[1]);
    assert_eq!(a.x, -12.0);
    assert_eq!(a.z, 4.0);
    assert_eq!(b.x, 12.0);
    assert_eq!(b.z, 4.0);
    assert!((a.x - b.x).abs() > 1.0, "stalls must stay apart");

    IN_GATE.store(0, Ordering::Relaxed);
    s.local_speed = 18.0;
    // Stall XZ held while progress runs: do not leave the dot parked on the gate.
    s.riders[0].track_pos = 0.40;
    let on_poly = rider_map_pose(&s, &s.riders[0]);
    assert!(
        on_poly.x.abs() < 200.0 && on_poly.z.abs() < 200.0,
        "after green, a stuck stall coord walks the poly"
    );
    assert!((on_poly.x - a.x).hypot(on_poly.z - a.z) > 1.0);
}

#[test]
fn map_subject_pose_returns_to_me_when_riding_even_if_focus_is_stale() {
    let mut s = live_snap();
    s.has_telemetry = 1;
    s.focus_race_num = 1;
    let pose = subject_pose(&s, 0.0).expect("ride pose");
    assert!(pose.from_local);
    assert_eq!(pose.x, 10.0);
    assert_eq!(pose.z, 4.0);
}

