use super::*;

fn available() -> UpdateState {
    UpdateState::Available {
        version: "9.9.9".into(),
        url: "https://github.com/LeadingTrendTechnologies/HoleshotHUD/releases/download/v9.9.9/Holeshot-HUD-9.9.9-windows-x64.zip".into(),
        digest: "a".repeat(64),
        signature: "https://github.com/LeadingTrendTechnologies/HoleshotHUD/releases/download/v9.9.9/Holeshot-HUD-9.9.9-windows-x64.zip.minisig".into(),
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
    assert!(looks_protected(Path::new(
        r"C:\Program Files (x86)\Holeshot HUD"
    )));
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
        "tag_name": "v9.9.9",
        "assets": [
            {"name":"app.tar.gz","browser_download_url":"https://github.com/LeadingTrendTechnologies/HoleshotHUD/releases/download/v9.9.9/app.tar.gz","digest":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},
            {"name":"Holeshot-HUD-9.9.9-windows-x64.zip","browser_download_url":"https://github.com/LeadingTrendTechnologies/HoleshotHUD/releases/download/v9.9.9/Holeshot-HUD-9.9.9-windows-x64.zip","digest":"sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"},
            {"name":"Holeshot-HUD-9.9.9-windows-x64.zip.minisig","browser_download_url":"https://github.com/LeadingTrendTechnologies/HoleshotHUD/releases/download/v9.9.9/Holeshot-HUD-9.9.9-windows-x64.zip.minisig"}
        ]
    }"#;
    let rel = parse_latest_release(body).expect("zip");
    assert_eq!(
        rel.url,
        "https://github.com/LeadingTrendTechnologies/HoleshotHUD/releases/download/v9.9.9/Holeshot-HUD-9.9.9-windows-x64.zip"
    );
    assert_eq!(
        rel.digest,
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    );
    assert_eq!(
        rel.signature,
        "https://github.com/LeadingTrendTechnologies/HoleshotHUD/releases/download/v9.9.9/Holeshot-HUD-9.9.9-windows-x64.zip.minisig"
    );
}

#[test]
fn rejects_non_github_zip_url() {
    let body = r#"{
        "tag_name": "v1.0.0",
        "assets": [
            {"browser_download_url":"https://example.test/Holeshot-windows-x64.zip","digest":"sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"}
        ]
    }"#;
    assert!(parse_latest_release(body).is_err());
}

#[test]
fn rejects_a_zip_with_no_signature() {
    let body = r#"{
        "tag_name": "v1.0.0",
        "assets": [
            {"name":"Holeshot-HUD-1.0.0-windows-x64.zip","browser_download_url":"https://github.com/LeadingTrendTechnologies/HoleshotHUD/releases/download/v1.0.0/Holeshot-HUD-1.0.0-windows-x64.zip","digest":"sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"}
        ]
    }"#;
    assert!(parse_latest_release(body).is_err());
}

#[test]
fn signature_accepts_its_key_and_rejects_another() {
    let message = b"holeshot-update";
    let pair = minisign::KeyPair::generate_encrypted_keypair(Some(String::new())).unwrap();
    let other = minisign::KeyPair::generate_encrypted_keypair(Some(String::new())).unwrap();
    let signature = minisign::sign(Some(&pair.pk), &pair.sk, std::io::Cursor::new(message), None, None)
        .unwrap()
        .into_string();
    assert!(signature_is_valid(
        &pair.pk.to_base64(),
        message,
        &signature
    ));
    assert!(!signature_is_valid(
        &other.pk.to_base64(),
        message,
        &signature
    ));
    assert!(!signature_is_valid(&pair.pk.to_base64(), b"other", &signature));
}

#[test]
fn rejects_setup_exe_named_like_a_zip() {
    assert!(!release_zip_url_ok(
        "https://github.com/LeadingTrendTechnologies/HoleshotHUD/releases/download/v1/HoleshotHUD-Setup.exe"
    ));
    assert!(release_zip_url_ok(
        "https://github.com/LeadingTrendTechnologies/HoleshotHUD/releases/download/v1.0.0/Holeshot-HUD-1.0.0-windows-x64.zip"
    ));
}

#[test]
fn overlay_exe_name_is_exact() {
    assert!(is_overlay_exe("Holeshot-HUD.exe"));
    assert!(is_overlay_exe("holeshot-hud.exe"));
    assert!(!is_overlay_exe("HoleshotHUD-Setup.exe"));
    assert!(!is_overlay_exe("Install.exe"));
    assert!(is_plugin_dlo("Holeshot-HUD.dlo"));
    assert!(!is_plugin_dlo("mxbo.dlo"));
}

#[test]
fn zip_paths_cannot_escape_extract_dir() {
    let dest = Path::new(r"C:\temp\update");
    assert!(safe_extract_path(dest, Path::new("Holeshot-HUD.exe")).is_some());
    assert!(safe_extract_path(dest, Path::new("..\\evil.exe")).is_none());
    assert!(safe_extract_path(dest, Path::new("/evil.exe")).is_none());
}

#[test]
fn digest_must_be_sha256_hex() {
    assert!(parse_sha256_digest(
        "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    )
    .is_some());
    assert!(parse_sha256_digest(
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    )
    .is_none());
    assert!(parse_sha256_digest("sha256:short").is_none());
}

#[test]
fn overlay_only_relaunch_skips_plugin_changed() {
    assert_eq!(relaunch_args(false), vec!["--skip-update", "--whats-new"]);
    assert!(!dlo_differs(b"plugin", Some(b"plugin")));
}

#[test]
fn plugin_byte_diff_relaunch_passes_plugin_changed() {
    assert_eq!(
        relaunch_args(true),
        vec!["--skip-update", "--whats-new", "--plugin-changed"]
    );
    assert!(dlo_differs(b"new", Some(b"old")));
    assert!(dlo_differs(b"new", None));
}
