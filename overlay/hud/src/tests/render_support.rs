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


/// Full-HUD draws in these tests do not capture per-widget pixmaps.
pub(crate) fn draw(
    px: &mut Pixmap,
    fonts: &Fonts,
    snap: Option<&Snapshot>,
    cfg: &HudConfig,
    w: u32,
    h: u32,
    age: f32,
    restart_hint: bool,
    plugin_hint: bool,
    settings_hint: bool,
) {
    super::draw(
        px,
        fonts,
        snap,
        cfg,
        w,
        h,
        age,
        restart_hint,
        plugin_hint,
        settings_hint,
        &mut |_, _| {},
    );
}

pub(crate) fn session_lock() -> std::sync::MutexGuard<'static, ()> {
    crate::race_store::session_test_lock()
}

pub(crate) fn reset_session() {
    reset_session_clock_track();
    reset_flag_display();
    LAST_SESSION_SIG.store(0, Ordering::Relaxed);
    LAST_CUR_LAP.store(0, Ordering::Relaxed);
    HS_SCROLL.with(|a| {
        *a.borrow_mut() = IndexSlide {
            from: 0.0,
            to: 0.0,
            start: 0.0,
            init: false,
        };
    });
    HS_SLIDE.with(|a| *a.borrow_mut() = TableSlides { rows: Vec::new() });
    ST_SLIDE.with(|a| *a.borrow_mut() = TableSlides { rows: Vec::new() });
    REL_SLIDE.with(|a| *a.borrow_mut() = TableSlides { rows: Vec::new() });
}

pub(crate) fn standing(race_num: i32, position: i32, laps: i32) -> Standing {
    let mut row = Standing::default();
    row.race_num = race_num;
    row.position = position;
    row.num_laps = laps;
    write_name(&mut row.name, &format!("R{race_num}"));
    write_name(&mut row.bike, "YZ450");
    write_name(&mut row.category, "MX1");
    row
}

pub(crate) fn rider(race_num: i32, x: f32, z: f32, pos: f32) -> Rider {
    let mut r = Rider::default();
    r.race_num = race_num;
    r.x = x;
    r.z = z;
    r.track_pos = pos;
    write_name(&mut r.name, &format!("R{race_num}"));
    r
}

pub(crate) fn live_snap() -> Snapshot {
    let mut s = Snapshot {
        magic: MAGIC,
        version: VERSION,
        on_track: 1,
        has_telemetry: 1,
        local_race_num: 12,
        focus_race_num: 12,
        local_speed: 18.0,
        current_lap: 6,
        track_length: 1000.0,
        sf_meters: 0.0,
        local_track_pos: 0.92,
        local_x: 10.0,
        local_z: 4.0,
        standing_count: 2,
        rider_count: 2,
        show_standings: 1,
        show_relative: 1,
        show_map: 1,
        standings_rows: 12,
        relative_count: 3,
        local_gear: 3,
        local_rpm: 7200,
        engine_temp: 82.0,
        air_temp: 21.0,
        fuel: 5.6,
        max_fuel: 7.0,
        last_lap_ms: 95_000,
        current_lap_ms: 40_000,
        best_lap_ms: 93_500,
        ..Snapshot::default()
    };
    s.standings[0] = standing(1, 1, 5);
    s.standings[0].best_lap_ms = 92_000;
    s.standings[1] = standing(12, 2, 5);
    s.standings[1].gap_ms = 2_400;
    s.standings[1].best_lap_ms = 93_500;
    s.standings[1].last_lap_ms = 95_000;
    s.riders[0] = rider(1, 20.0, 8.0, 0.10);
    s.riders[1] = rider(12, 10.0, 4.0, 0.92);
    write_name(&mut s.track_name, "Test Track");
    write_name(&mut s.setup_name, r"C:\Setups\Washougal Soft.xml");
    let n = 24;
    s.poly_count = n;
    for i in 0..n as usize {
        let a = i as f32 / n as f32 * std::f32::consts::TAU;
        s.poly[i] = Point {
            x: a.cos() * 80.0,
            z: a.sin() * 50.0,
        };
    }
    s
}

/// Put the focus rider at `pos` round the lap and let the flag logic see the step.
pub(crate) fn ride_to(s: &mut Snapshot, pos: f32) -> DashFlag {
    s.local_track_pos = pos;
    s.riders[1].track_pos = pos;
    dash_race_flag(s)
}

/// Walk from just after the line round into the run-in window. The geometry guards need a
/// mid-lap sighting and a closing step before a run-in flag can arm.
pub(crate) fn ride_lap_to_line(s: &mut Snapshot) -> DashFlag {
    for pos in [0.10, 0.30, 0.50, 0.70, 0.85, 0.92] {
        ride_to(s, pos);
    }
    ride_to(s, 0.96)
}

/// Cross the line: laps tick over just past S/F, which is also where the overlay learns
/// where the line is.
pub(crate) fn cross_line(s: &mut Snapshot, laps: i32, lap_num: i32) -> DashFlag {
    s.standings[1].num_laps = laps;
    s.current_lap = lap_num;
    ride_to(s, 0.01)
}

/// Run the white-flag wave out without waiting on the clock.
pub(crate) fn age_white_wave() {
    WHITE_WAVE_AT.fetch_sub(WHITE_WAVE_MS + 1_000, Ordering::Relaxed);
}

pub(crate) fn expire_timed(s: &mut Snapshot) {
    expire_timed_extras(s, 2);
}

pub(crate) fn expire_timed_extras(s: &mut Snapshot, extras: i32) {
    reset_session();
    s.session_length = 8;
    s.session_laps = extras;
    s.session_time_ms = 30_000;
    s.current_lap = 1;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    let _ = session_remain_ms(s);
    s.session_time_ms = 8 * 60 * 1000;
    s.current_lap = 6;
    s.local_speed = 18.0;
    s.standings[0].num_laps = 5;
    s.standings[1].num_laps = 5;
    let remain = session_remain_ms(s).expect("countdown while time remains");
    assert!(remain > 60_000, "expected a real countdown, got {remain}");
    s.session_time_ms = 8 * 60 * 1000 - 1_000;
    let remain = session_remain_ms(s).expect("ticking race clock");
    assert!(remain < 8 * 60 * 1000);
    s.session_time_ms = 400;
    let remain = session_remain_ms(s).expect("expired timed session");
    assert_eq!(remain, 0);
    assert!(overtime_active(s));
}

pub(crate) fn fonts() -> Fonts {
    Fonts::for_family(FontFamily::Roboto).expect("bundled Roboto")
}

pub(crate) fn golden_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/goldens")
}

pub(crate) fn update_goldens() -> bool {
    matches!(
        std::env::var("UPDATE_GOLDENS").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE")
    )
}

pub(crate) fn crop_px(src: &Pixmap, x: f32, y: f32, w: f32, h: f32, pad: f32) -> Pixmap {
    let x0 = (x - pad).floor().max(0.0) as i32;
    let y0 = (y - pad).floor().max(0.0) as i32;
    let x1 = ((x + w + pad).ceil() as i32).min(src.width() as i32);
    let y1 = ((y + h + pad).ceil() as i32).min(src.height() as i32);
    let cw = (x1 - x0).max(1) as u32;
    let ch = (y1 - y0).max(1) as u32;
    let mut out = Pixmap::new(cw, ch).expect("crop");
    out.fill(Color::TRANSPARENT);
    out.draw_pixmap(
        -x0,
        -y0,
        src.as_ref(),
        &tiny_skia::PixmapPaint::default(),
        tiny_skia::Transform::identity(),
        None,
    );
    out
}

pub(crate) fn assert_golden(name: &str, px: &Pixmap) {
    let dir = golden_dir();
    let path = dir.join(format!("{name}.png"));
    if update_goldens() {
        std::fs::create_dir_all(&dir).expect("goldens dir");
        std::fs::write(&path, px.encode_png().expect("png")).expect("write golden");
        return;
    }
    let bytes = std::fs::read(&path).unwrap_or_else(|_| {
        panic!("missing golden {name}.png — run with UPDATE_GOLDENS=1")
    });
    let actual = px.encode_png().expect("png");
    if bytes != actual {
        let actual_path = dir.join(format!("{name}.actual.png"));
        let _ = std::fs::write(&actual_path, &actual);
        panic!(
            "golden mismatch {name} (wrote {})",
            actual_path.display()
        );
    }
}

pub(crate) fn hide_widgets(cfg: &mut HudConfig) {
    for id in WidgetId::ALL {
        cfg[id].show = false;
    }
}

pub(crate) fn golden_snap(s: &Snapshot, cfg: &HudConfig) -> Snapshot {
    let mut s = *s;
    cfg.apply_to_snapshot(&mut s);
    s
}

pub(crate) fn draw_widget_golden(name: &str, s: &Snapshot, cfg: &HudConfig, rect: crate::shm::Rect) {
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    draw(
        &mut px,
        &fonts(),
        Some(s),
        cfg,
        1280,
        720,
        0.0,
        false,
        false,
        false,
    );
    let crop = crop_px(
        &px,
        rect.x * 1280.0,
        rect.y * 720.0,
        rect.w * 1280.0,
        rect.h * 720.0,
        8.0,
    );
    assert_golden(name, &crop);
}

pub(crate) fn rgba8(c: Color) -> (u8, u8, u8, u8) {
    (
        (c.red() * 255.0).round() as u8,
        (c.green() * 255.0).round() as u8,
        (c.blue() * 255.0).round() as u8,
        (c.alpha() * 255.0).round() as u8,
    )
}

pub(crate) fn sample_px(px: &Pixmap, x: f32, y: f32) -> [u8; 4] {
    let xi = (x.floor() as i32).clamp(0, px.width() as i32 - 1) as u32;
    let yi = (y.floor() as i32).clamp(0, px.height() as i32 - 1) as u32;
    let i = ((yi * px.width() + xi) * 4) as usize;
    let d = px.data();
    [d[i], d[i + 1], d[i + 2], d[i + 3]]
}

pub(crate) fn hit_nums_by_pos() -> Vec<i32> {
    let mut hits = click_rider_hits();
    hits.sort_by(|a, b| {
        a.y.partial_cmp(&b.y)
            .unwrap()
            .then(a.x.partial_cmp(&b.x).unwrap())
    });
    hits.iter().map(|h| h.race_num).collect()
}

pub(crate) fn draw_ok(s: &Snapshot, cfg: &HudConfig) {
    let mut px = Pixmap::new(1280, 720).expect("pixmap");
    draw(&mut px, &fonts(), Some(s), cfg, 1280, 720, 0.0, false, false, false);
}

pub(crate) fn set_completed_lap(s: &mut Snapshot, slot: usize, ms: i32, laps: i32) {
    s.standings[slot].last_lap_ms = ms;
    s.standings[slot].num_laps = laps;
    if s.standings[slot].race_num == s.focus_race_num {
        s.last_lap_ms = ms;
        s.current_lap = laps + 1;
    }
}

pub(crate) fn col_right(slots: &[(StField, f32, f32)]) -> f32 {
    slots.last().map(|(_, x, w)| x + w).unwrap_or(0.0)
}

/// 10–30 min +1 looks like warmup/practice. Publishing extras after expiry must not
/// reset the session clock, or the dash sits on `0/1` with checkered while you
/// still have the uncounted lap and the extra to run.
pub(crate) fn long_timed_plus_one_late_extras(minutes: i32) {
    reset_session();
    let mut s = live_snap();
    s.session_kind = 7;
    s.session_length = session_length_for_minutes(minutes);
    s.session_laps = 0;
    s.session_time_ms = 30_000;
    s.current_lap = 1;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    let _ = session_remain_ms(&s);
    let len_ms = minutes * 60 * 1000;
    s.session_time_ms = len_ms;
    s.current_lap = 8;
    s.local_speed = 18.0;
    s.standings[0].num_laps = 8;
    s.standings[1].num_laps = 7;
    let remain = session_remain_ms(&s).expect("countdown while time remains");
    assert!(remain > 60_000, "{minutes}:00 expected a real countdown, got {remain}");
    s.session_time_ms = len_ms - 1_000;
    let _ = session_remain_ms(&s);
    s.session_time_ms = 400;
    assert_eq!(session_remain_ms(&s), Some(0));
    s.session_time_ms = len_ms;
    let _ = session_remain_ms(&s);
    assert_eq!(session_banner(&s).1, "00:00", "extras not published yet");
    assert_eq!(dash_race_flag(&s), DashFlag::None);

    s.current_lap = 9;
    s.standings[1].num_laps = 8;
    s.standings[0].num_laps = 7;
    let _ = session_remain_ms(&s);
    assert_eq!(dash_race_flag(&s), DashFlag::None, "uncounted lap, extras still unpublished");

    s.session_laps = 1;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    s.session_time_ms = 40_000;
    assert_eq!(session_banner(&s).1, "0/1");
    assert_eq!(dash_race_flag(&s), DashFlag::None, "empty standings after reset");

    s.standings[0].num_laps = 1;
    s.standings[1].num_laps = 1;
    s.current_lap = 2;
    assert_eq!(dash_race_flag(&s), DashFlag::None);
    s.standings[0].num_laps = 2;
    s.standings[1].num_laps = 2;
    s.current_lap = 3;
    assert_eq!(
        dash_race_flag(&s),
        DashFlag::None,
        "rebuilt lap 2 is not the extra"
    );
    s.standings[0].num_laps = 9;
    s.standings[1].num_laps = 8;
    s.current_lap = 9;
    assert_ne!(
        dash_race_flag(&s),
        DashFlag::Checkered,
        "must not latch checkered while the extra is still ahead"
    );
    assert_eq!(session_banner(&s).1, "0/1", "still the uncounted lap after recovery");

    s.standings[0].num_laps = 9;
    s.standings[1].num_laps = 8;
    s.current_lap = 9;
    assert_eq!(ride_to(&mut s, 0.40), DashFlag::None);
    s.standings[1].num_laps = 9;
    s.current_lap = 10;
    assert_eq!(dash_race_flag(&s), DashFlag::White, "last extra after the timed-lap base");
    age_white_wave();
    assert_eq!(ride_to(&mut s, 0.50), DashFlag::None);
    assert_eq!(cross_line(&mut s, 10, 11), DashFlag::Checkered);
}

pub(crate) fn leader_goes_past(s: &mut Snapshot) {
    s.riders[0].track_pos = 0.35;
    let _ = lapped(s);
    s.riders[0].track_pos = 0.42;
    let _ = lapped(s);
    s.riders[0].track_pos = 0.54;
    assert!(lapped(s), "~Lapped the moment the leader goes past");
}

pub(crate) fn lap_moto_after_gate_glitch_shows_laps(laps: i32, length: i32) {
    reset_session();
    let mut s = live_snap();
    s.session_length = length;
    s.session_laps = laps;
    s.session_time_ms = 45_350;
    s.current_lap = 1;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    assert_eq!(
        session_banner(&s).1,
        "00:45",
        "{laps}-lap length={length} gate"
    );
    s.session_time_ms = 1_970;
    assert_eq!(session_banner(&s).1, "00:01");
    s.session_time_ms = 980_000;
    s.local_speed = 4.0;
    let want = format!("1 / {laps}");
    assert_eq!(
        session_banner(&s).1,
        want,
        "{laps}-lap length={length} glitched 16:20 is not a timed clock"
    );
    assert!(is_lap_race(&s), "{laps}-lap length={length} must stay a lap race");
    s.session_time_ms = 453_000;
    s.standings[1].num_laps = 1;
    s.current_lap = 2;
    assert_eq!(
        session_banner(&s).1,
        format!("2 / {laps}"),
        "{laps}-lap elapsed clock must not replace laps"
    );
}

pub(crate) fn session_length_for_minutes(minutes: i32) -> i32 {
    // 60+ minutes cannot use the integer-minutes encoding: 60 is a 00:60 gate.
    if minutes >= 60 {
        minutes * 60 * 1000
    } else {
        minutes
    }
}

pub(crate) fn timed_plus_n_keeps_countdown(minutes: i32, extras: i32) {
    reset_session();
    let mut s = live_snap();
    s.session_kind = 7;
    s.session_length = session_length_for_minutes(minutes);
    s.session_laps = extras;
    s.session_time_ms = minutes * 60 * 1000;
    s.current_lap = 1;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    let start = session_banner(&s).1;
    assert!(
        !is_lap_race(&s),
        "{minutes}:00 +{extras} must be timed, banner {start}"
    );
    assert!(
        start.contains(':') && !start.contains('/'),
        "{minutes}:00 +{extras} prestart must be a clock, got {start}"
    );
    s.session_time_ms = 10_000;
    let gate = session_banner(&s).1;
    assert!(
        gate.starts_with("00:"),
        "{minutes}:00 +{extras} gate board, got {gate}"
    );
    s.session_time_ms = minutes * 60 * 1000 - 30_000;
    s.local_speed = 18.0;
    s.current_lap = 2;
    s.standings[0].num_laps = 1;
    s.standings[1].num_laps = 1;
    let text = session_banner(&s).1;
    assert!(
        !is_lap_race(&s),
        "{minutes}:00 +{extras} after gate must stay timed, got {text}"
    );
    assert!(
        text.contains(':') && !text.contains('/'),
        "{minutes}:00 +{extras} must keep countdown, got {text}"
    );
    assert!(
        !text.contains('+'),
        "{minutes}:00 +{extras} live clock must not show +extras, got {text}"
    );
}

pub(crate) fn radar_arrow_snap_behind() -> (Snapshot, HudConfig) {
    let mut s = live_snap();
    s.local_x = 0.0;
    s.local_z = 0.0;
    s.local_vel_x = 0.0;
    s.local_vel_z = 0.0;
    s.local_yaw = 0.0;
    s.local_track_pos = 0.40;
    s.track_length = 1000.0;
    s.focus_race_num = 12;
    s.local_race_num = 12;
    s.riders[0].race_num = 1;
    s.riders[0].x = 0.0;
    s.riders[0].z = -5.0;
    s.riders[0].track_pos = 0.395;
    s.riders[0].crashed = 0;
    s.riders[1].race_num = 12;
    s.riders[1].x = 0.0;
    s.riders[1].z = 0.0;
    s.riders[1].track_pos = 0.40;
    let mut cfg = HudConfig::new();
    cfg.radar.radar_sides = true;
    cfg.radar.radar_rear = true;
    cfg.radar.radar_range = 12;
    (s, cfg)
}

/// Both on the same lap, focus (#12) scored second but `along` metres up the track on the
/// leader. Track length is 1000 m in the fixture, so a metre is 0.001 of a lap.
pub(crate) fn pass_the_leader(s: &mut Snapshot, along: f32) {
    s.riders[0].track_pos = 0.500;
    s.riders[1].track_pos = 0.500 + along / 1000.0;
    s.local_track_pos = s.riders[1].track_pos;
}

/// Ride every listed rider from where they are to `target` (a lap fraction, may run past
/// 1.0) in small steps, one tick per step, so the distance tracker sees real riding.
pub(crate) fn ride_field_to(s: &mut Snapshot, targets: &[(i32, f32)]) {
    let count = s.rider_count as usize;
    let starts: Vec<f32> = targets
        .iter()
        .map(|(num, _)| {
            s.riders[..count]
                .iter()
                .find(|r| r.race_num == *num)
                .map(|r| r.track_pos)
                .unwrap_or(0.0)
        })
        .collect();
    let steps = 100;
    for step in 1..=steps {
        let along = step as f32 / steps as f32;
        for ((num, target), start) in targets.iter().zip(&starts) {
            let pos = (start + (target - start) * along).rem_euclid(1.0);
            if let Some(r) = s.riders[..count].iter_mut().find(|r| r.race_num == *num) {
                r.track_pos = pos;
            }
            if *num == s.focus_race_num {
                s.local_track_pos = pos;
            }
        }
        let _ = RaceStore::tick(s);
    }
}

/// A field of `nums`, scored in that order on lap 0, all sitting on the gate at 0.10.
pub(crate) fn gate_field(nums: &[i32]) -> Snapshot {
    let mut s = live_snap();
    s.standing_count = nums.len() as i32;
    s.rider_count = nums.len() as i32;
    for (i, &num) in nums.iter().enumerate() {
        s.standings[i] = standing(num, i as i32 + 1, 0);
        s.riders[i] = rider(num, i as f32, 0.0, 0.10);
    }
    s.local_track_pos = 0.10;
    IN_GATE.store(1, Ordering::Relaxed);
    let _ = RaceStore::tick(&s);
    IN_GATE.store(0, Ordering::Relaxed);
    s
}

/// Walk a session and record what the dash would show each frame. `extra` replays what the
/// live-order code used to do: ask the session heuristics again after the clock had already
/// decided the frame.
pub(crate) fn clock_trace(laps: i32, length: i32, extra: bool) -> Vec<(ClockMode, String, RaceFlag)> {
    reset_session();
    let mut s = live_snap();
    s.session_length = length;
    s.session_laps = laps;
    s.session_time_ms = 50_000;
    s.current_lap = 1;
    s.local_speed = 0.0;
    s.standings[0].num_laps = 0;
    s.standings[1].num_laps = 0;
    let mut out = Vec::new();
    let step = |s: &Snapshot, out: &mut Vec<_>| {
        if extra {
            // What `build_field` used to call, right after `build_clock` ran.
            let _ = is_warmup(s);
            let _ = leader_finished(s);
            let _ = effective_race_laps(s);
        }
        let store = RaceStore::tick(s);
        out.push((store.clock.mode, store.clock.banner.1.clone(), store.clock.flag));
    };
    step(&s, &mut out);
    // Green, then a lap at a time to the finish.
    s.local_speed = 18.0;
    for lap in 1..=laps.max(1) + 1 {
        s.session_time_ms = 60_000 * lap;
        s.current_lap = lap + 1;
        s.standings[0].num_laps = lap;
        s.standings[1].num_laps = lap;
        step(&s, &mut out);
        s.local_track_pos = 0.5;
        step(&s, &mut out);
    }
    out
}

pub(crate) fn sys_procs_fixture() -> Vec<SysProc> {
    vec![
        SysProc {
            label: "HUD".into(),
            cpu: 4.0,
            gpu: 3.0,
            mem_mb: 88.0,
            mem_pct: 0.5,
            on: true,
        },
        SysProc {
            label: "MX Bikes".into(),
            cpu: 22.0,
            gpu: 38.0,
            mem_mb: 1800.0,
            mem_pct: 11.0,
            on: true,
        },
        SysProc {
            label: "MXB App".into(),
            cpu: 8.0,
            gpu: 1.0,
            mem_mb: 180.0,
            mem_pct: 1.1,
            on: true,
        },
        SysProc {
            label: "ReShade".into(),
            cpu: -1.0,
            gpu: -1.0,
            mem_mb: 44.0,
            mem_pct: 0.3,
            on: true,
        },
    ]
}

pub(crate) fn sector_snap(base: Snapshot) -> Snapshot {
    let mut s = base;
    s.sector_count = 3;
    s.current_lap_ms = 70_000;
    s.sector_last = 1;
    s.sector_cur = [24_093, 25_760, 0];
    s.sector_last_lap = [24_310, 25_820, 23_090];
    s.sector_best = [24_180, 25_640, 20_147];
    s.sector_delta = [-87, 120, 0];
    s.sector_delta_valid = 0b011;
    s
}

pub(crate) fn mid_race_snap() -> Snapshot {
    let mut s = live_snap();
    s.session_laps = 10;
    s.session_length = 0;
    s.current_lap = 3;
    s.standings[0].num_laps = 2;
    s.standings[1].num_laps = 2;
    s.local_track_pos = 0.40;
    s.riders[1].track_pos = 0.40;
    s.riders[0].track_pos = 0.33;
    s.local_speed = 18.0;
    s
}

pub(crate) fn crash_ahead(s: &mut Snapshot) {
    s.riders[0].crashed = 1;
    s.riders[0].track_pos = 0.43;
    s.local_track_pos = 0.40;
    s.riders[1].track_pos = 0.40;
}

pub(crate) fn crash_behind(s: &mut Snapshot) {
    s.riders[0].crashed = 1;
    s.riders[0].track_pos = 0.33;
    s.local_track_pos = 0.40;
    s.riders[1].track_pos = 0.40;
}

pub(crate) fn lapping_from_behind(s: &mut Snapshot) {
    s.standings[0].num_laps = 3;
    s.riders[0].track_pos = 0.365;
    s.local_track_pos = 0.40;
    s.riders[1].track_pos = 0.40;
}

pub(crate) fn lapping_ahead(s: &mut Snapshot) {
    s.standings[1].num_laps = 3;
    s.standings[0].num_laps = 2;
    s.riders[0].track_pos = 0.435;
    s.local_track_pos = 0.40;
    s.riders[1].track_pos = 0.40;
}

pub(crate) fn rect_has(px: &Pixmap, x0: f32, y0: f32, x1: f32, y1: f32, pred: impl Fn([u8; 4]) -> bool) -> bool {
    let mut x = x0;
    while x < x1 {
        let mut y = y0;
        while y < y1 {
            if pred(sample_px(px, x, y)) {
                return true;
            }
            y += 6.0;
        }
        x += 6.0;
    }
    false
}

pub(crate) fn is_yellow(p: [u8; 4]) -> bool {
    p[3] > 40 && p[0] > 170 && p[1] > 140 && p[2] < 110 && (p[0] as i16 - p[2] as i16) > 70
}

pub(crate) fn is_blue(p: [u8; 4]) -> bool {
    p[3] > 40 && p[2] > 140 && p[0] < 130 && p[1] > 80 && p[2] > p[0]
}

pub(crate) fn is_red(p: [u8; 4]) -> bool {
    p[3] > 40 && p[0] > 150 && p[1] < 120 && p[2] < 120 && (p[0] as i16 - p[1] as i16) > 60
}

