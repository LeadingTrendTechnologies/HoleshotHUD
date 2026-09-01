use super::*;

#[test]
fn embedded_plugin_wins_over_sidecar() {
    let embedded = vec![1u8; 2000];
    let sidecar = vec![2u8; 2000];
    assert_eq!(pick_plugin_bytes(&embedded, Some(&sidecar)).unwrap()[0], 1);
}

#[test]
fn sidecar_used_when_embed_is_placeholder() {
    let sidecar = vec![3u8; 2000];
    assert_eq!(pick_plugin_bytes(&[], Some(&sidecar)).unwrap()[0], 3);
    assert!(pick_plugin_bytes(&[], Some(&[1, 2, 3])).is_none());
}

#[test]
fn overlay_only_install_does_not_ask_for_game_restart() {
    let outcome = InstallOutcome::AlreadyCurrent;
    assert!(!should_retry(outcome));
    assert!(!should_mark_game_restart(outcome, true));
    assert!(!should_mark_from_updater_flag(false, true));
}

#[test]
fn plugin_write_while_game_running_asks_for_restart() {
    assert!(should_mark_game_restart(InstallOutcome::Wrote, true));
    assert!(!should_retry(InstallOutcome::Wrote));
    assert!(should_mark_game_restart(InstallOutcome::Locked, true));
    assert!(should_retry(InstallOutcome::Locked));
}

#[test]
fn plugin_write_while_game_closed_does_not_ask_for_restart() {
    assert!(!should_mark_game_restart(InstallOutcome::Wrote, false));
    assert!(!should_mark_game_restart(InstallOutcome::Locked, false));
    assert!(!should_mark_from_updater_flag(true, false));
}

#[test]
fn updater_flag_asks_for_restart_only_when_game_is_open() {
    assert!(should_mark_from_updater_flag(true, true));
    assert!(!should_mark_from_updater_flag(true, false));
    assert!(!should_mark_from_updater_flag(false, true));
}
