use super::*;

#[test]
fn ranked_url_uses_ff_and_steamid64_hex() {
    let url = ranked_url_from_account(0x1284_9BE5).unwrap();
    assert_eq!(url, "https://mxb-ranked.com/Rider/FF0110000112849BE5");
}

#[test]
fn ranked_url_rejects_signed_out_account() {
    assert!(ranked_url_from_account(0).is_none());
}

#[test]
fn cbr_url_uses_ff_and_steamid64_hex() {
    let url = cbr_url_from_account(0x1284_9BE5).unwrap();
    assert_eq!(url, "https://www.cbrservers.com/player/FF0110000112849BE5");
}

#[test]
fn cbr_url_rejects_signed_out_account() {
    assert!(cbr_url_from_account(0).is_none());
}

#[test]
fn most_recent_login_user_is_the_steam_id() {
    let vdf = r#"
"users"
{
	"76561198000000000"
	{
		"MostRecent"		"0"
	}
	"76561198270946277"
	{
		"AccountName"		"rider"
		"MostRecent"		"1"
	}
}
"#;
    assert_eq!(most_recent_steam_id(vdf), Some(76561198270946277));
    assert_eq!(
        ranked_url_from_steam_id64(most_recent_steam_id(vdf).unwrap()),
        "https://mxb-ranked.com/Rider/FF0110000112849BE5"
    );
}

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
