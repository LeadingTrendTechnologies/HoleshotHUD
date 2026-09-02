use super::pid_for;
use super::shm_publishes;
use mxbo_hud::config::{SysApp, SysAppKind};

#[test]
fn seqlock_delta_is_half_the_seq_step() {
    assert_eq!(shm_publishes(0, 140), 70);
    assert_eq!(shm_publishes(10, 10), 0);
    assert_eq!(shm_publishes(u32::MAX - 1, 2), 2);
}

fn exe(key: &str, names: &[&str]) -> SysApp {
    SysApp {
        key: key.into(),
        label: key.into(),
        names: names.iter().map(|n| (*n).to_string()).collect(),
        kind: SysAppKind::Exe,
        show: true,
    }
}

#[test]
fn pid_for_matches_kind_and_exe_names() {
    let procs = vec![
        (10, "holeshot-hud.exe".into()),
        (20, "mxbikes.exe".into()),
        (30, "frostmod.exe".into()),
        (40, "frost.exe".into()),
        (50, "obs64.exe".into()),
        (60, "reshade_setup.exe".into()),
    ];
    let hud = SysApp::from_preset(
        mxbo_hud::config::SYS_PRESETS.iter().find(|p| p.key == "hud").unwrap(),
        true,
    );
    assert_eq!(pid_for(&hud, 10, &procs), Some(10));
    let mx = SysApp::from_preset(
        mxbo_hud::config::SYS_PRESETS.iter().find(|p| p.key == "mxbikes").unwrap(),
        true,
    );
    assert_eq!(pid_for(&mx, 10, &procs), Some(20));
    let app = SysApp::from_preset(
        mxbo_hud::config::SYS_PRESETS.iter().find(|p| p.key == "mxbapp").unwrap(),
        true,
    );
    assert_eq!(pid_for(&app, 10, &procs), Some(40));
    let shade = SysApp::from_preset(
        mxbo_hud::config::SYS_PRESETS.iter().find(|p| p.key == "reshade").unwrap(),
        true,
    );
    assert_eq!(pid_for(&shade, 10, &procs), Some(60));
    assert_eq!(pid_for(&exe("obs", &["obs64.exe", "obs32.exe"]), 10, &procs), Some(50));
    assert_eq!(
        pid_for(
            &exe("obs", &["obs64.exe", "obs32.exe"]),
            10,
            &[(70, "obs32.exe".into())]
        ),
        Some(70)
    );
    assert_eq!(pid_for(&exe("discord", &["discord.exe"]), 10, &procs), None);
}
