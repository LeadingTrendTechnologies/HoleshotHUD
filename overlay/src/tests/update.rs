use super::*;

fn available() -> UpdateState {
    UpdateState::Available {
        version: "9.9.9".into(),
        url: "https://example.test/app.zip".into(),
    }
}

#[test]
fn banner_when_auto_update_is_off_and_a_build_is_ready() {
    assert_eq!(
        manual_banner(false, false, &available()),
        Some(ManualBanner::Available {
            version: "9.9.9".into()
        })
    );
}

#[test]
fn no_banner_when_auto_update_is_on() {
    assert_eq!(manual_banner(true, false, &available()), None);
}

#[test]
fn no_banner_after_dismiss() {
    assert_eq!(manual_banner(false, true, &available()), None);
}

#[test]
fn no_banner_when_already_current() {
    assert_eq!(manual_banner(false, false, &UpdateState::Current), None);
    assert_eq!(manual_banner(false, false, &UpdateState::Idle), None);
    assert_eq!(manual_banner(false, false, &UpdateState::Checking), None);
    assert_eq!(
        manual_banner(false, false, &UpdateState::Failed("offline".into())),
        None
    );
}

#[test]
fn banner_stays_up_while_installing() {
    assert_eq!(
        manual_banner(false, false, &UpdateState::Downloading),
        Some(ManualBanner::Installing)
    );
}

#[test]
fn protected_paths_need_admin() {
    assert!(looks_protected(Path::new(r"C:\Program Files\Holeshot HUD")));
    assert!(looks_protected(Path::new(r"C:\Program Files (x86)\Holeshot HUD")));
    assert!(!looks_protected(Path::new(
        r"C:\Users\troye\AppData\Local\Holeshot HUD"
    )));
    assert!(looks_protected(Path::new(r"C:\Windows\System32\foo")));
    assert!(looks_protected(Path::new("/program files/Holeshot HUD")));
}

#[test]
fn no_banner_when_auto_update_is_on_during_download() {
    assert_eq!(manual_banner(true, false, &UpdateState::Downloading), None);
}

#[test]
fn version_newer_compares_semver_and_v_prefix() {
    assert!(version_newer("1.2.0", "1.1.9"));
    assert!(!version_newer("1.2.0", "1.2.0"));
    assert!(!version_newer("1.1.9", "1.2.0"));
    assert_eq!(parse_ver("v1.2.3-beta"), [1, 2, 3]);
    assert_eq!(parse_ver("0.1.19"), [0, 1, 19]);
}

#[test]
fn json_download_url_prefers_windows_x64_zip() {
    let body = r#"{
        "assets": [
            {"browser_download_url":"https://example.test/app.tar.gz"},
            {"browser_download_url":"https://example.test/Holeshot-windows-x64.zip"}
        ]
    }"#;
    assert_eq!(
        json_download_url(body).as_deref(),
        Some("https://example.test/Holeshot-windows-x64.zip")
    );
}
