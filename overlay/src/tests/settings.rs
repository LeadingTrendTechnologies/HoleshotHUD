use super::*;
use std::fs;
use std::path::PathBuf;

fn dummy_ui(open: bool) -> SettingsUi {
    SettingsUi {
        host: HWND::default(),
        tab: Tab::App,
        last_widget: Tab::Standings,
        app_section: AppSection::Look,
        hover: None,
        focus: None,
        hits: Vec::new(),
        open_drop: None,
        drag: None,
        slide: None,
        sv_drag: None,
        scroll: 0.0,
        content_h: 0.0,
        scroll_max: 0.0,
        nav_scroll: 0.0,
        nav_content_h: 0.0,
        nav_top: 0.0,
        nav_bottom: 0.0,
        banner_dismissed: false,
        whats_new_open: open,
        whats_new_scroll: 0.0,
        whats_new_scroll_max: 0.0,
        reply_id: None,
        reply_scroll: 0.0,
        reply_scroll_max: 0.0,
        drop_scroll: 0.0,
        drop_menu: None,
        bind_listen: false,
        review_filter: crate::review::ListFilter::All,
        review_stay_on_list: false,
        analyze_id: None,
        analyze_compare: -1,
        analyze_you_lap: -1,
        analyze_warmup: false,
        analyze_scrub: 0.0,
        analyze_zoom: 1.0,
        analyze_pan_x: 0.0,
        analyze_pan_z: 0.0,
        analyze_scrubbing: false,
        analyze_follow: false,
        map_drag: None,
        profile_all_time: true,
        clear_confirm: None,
        st_col_slides: ColSlides::new(),
        rel_col_slides: ColSlides::new(),
        col_list_y: 0.0,
        col_list_n: 0,
        col_list_kind: None,
    }
}

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/goldens")
}

fn update_goldens() -> bool {
    matches!(
        std::env::var("UPDATE_GOLDENS").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE")
    )
}

fn assert_golden(name: &str, px: &Pixmap) {
    let dir = golden_dir();
    let path = dir.join(format!("{name}.png"));
    if update_goldens() {
        fs::create_dir_all(&dir).expect("goldens dir");
        fs::write(&path, px.encode_png().expect("png")).expect("write golden");
        return;
    }
    let bytes = fs::read(&path)
        .unwrap_or_else(|_| panic!("missing golden {name}.png — run with UPDATE_GOLDENS=1"));
    let expected = Pixmap::decode_png(&bytes).expect("decode golden");
    if expected.width() != px.width()
        || expected.height() != px.height()
        || expected.data() != px.data()
    {
        let actual_path = dir.join(format!("{name}.actual.png"));
        let _ = fs::write(&actual_path, px.encode_png().expect("png"));
        panic!("golden mismatch {name} (wrote {})", actual_path.display());
    }
}

fn hit_ids(hits: &[HitBox]) -> Vec<Hit> {
    hits.iter().map(|h| h.id).collect()
}

fn snap_analyze() -> (Option<i64>, bool, i32, i32, bool, f32) {
    let ui = UI.lock().unwrap();
    let u = ui.as_ref().unwrap();
    (
        u.analyze_id,
        u.review_stay_on_list,
        u.analyze_compare,
        u.analyze_you_lap,
        u.analyze_warmup,
        u.analyze_scrub,
    )
}

#[test]
fn whats_new_modal_paints_got_it() {
    if crate::changelog::modal_notes().is_none() {
        return;
    }
    refresh_palette();
    let fonts = Fonts::for_family(FontFamily::Exo2).expect("Exo 2");
    *UI.lock().unwrap() = Some(dummy_ui(true));
    let mut px = Pixmap::new(1000, 720).expect("pixmap");
    draw(&mut px, &fonts, 1000.0, 720.0);
    let hits = UI.lock().unwrap().as_ref().unwrap().hits.clone();
    assert!(hit_ids(&hits).contains(&Hit::WhatsNewDismiss));
    assert!(hit_ids(&hits).contains(&Hit::WhatsNewScrim));
    let dismiss = hits.iter().find(|h| h.id == Hit::WhatsNewDismiss).unwrap();
    assert!(dismiss.w > 80.0 && dismiss.h > 24.0);
    assert_golden("whats-new", &px);
    *UI.lock().unwrap() = None;
}

#[test]
fn reply_modal_paints_got_it() {
    refresh_palette();
    let fonts = Fonts::for_family(FontFamily::Exo2).expect("Exo 2");
    let view = crate::feedback::ReplyView {
        id: "t1".into(),
        kind_label: "Bug",
        lines: vec![
            crate::feedback::ChatLine {
                from_dev: false,
                text: "The map vanished".into(),
            },
            crate::feedback::ChatLine {
                from_dev: true,
                text: "Which track were you on?".into(),
            },
        ],
    };
    let mut px = Pixmap::new(1000, 720).expect("pixmap");
    let mut hits = Vec::new();
    let _ = draw_reply(&mut px, &fonts, 1000.0, 720.0, &view, None, 0.0, &mut hits);
    let ids = hit_ids(&hits);
    assert!(ids.contains(&Hit::ReplyDismiss));
    assert!(ids.contains(&Hit::ReplySend));
    assert!(ids.contains(&Hit::ReplyText));
    assert!(ids.contains(&Hit::ReplyScrim));
    assert_golden("reply", &px);
}

#[test]
fn dump_review_mock_images() {
    if std::env::var("DUMP_REVIEW_MOCKS").as_deref() != Ok("1") {
        return;
    }
    let _g = crate::review::serial();
    let tmp = std::env::temp_dir().join("mxbo-review-mocks");
    let _ = fs::remove_dir_all(&tmp);
    crate::review::init(tmp);
    let dest = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(".impeccable")
        .join("mocks");
    dump_review_pages(&dest).expect("dump Review mocks");
    crate::review::reset();
}

#[test]
fn review_empty_paints_enable_gate() {
    let _g = crate::review::serial();
    crate::review::reset();
    {
        let mut g = crate::config::CONFIG.lock().unwrap();
        g.review = false;
    }
    refresh_palette();
    let fonts = Fonts::for_family(FontFamily::Exo2).expect("Exo 2");
    let mut ui = dummy_ui(false);
    ui.tab = Tab::Review;
    *UI.lock().unwrap() = Some(ui);
    let mut px = Pixmap::new(1000, 720).expect("pixmap");
    draw(&mut px, &fonts, 1000.0, 720.0);
    let hits = UI.lock().unwrap().as_ref().unwrap().hits.clone();
    let ids = hit_ids(&hits);
    assert!(ids.contains(&Hit::TabReview));
    assert!(ids.contains(&Hit::ReviewToggle));
    assert!(!ids.contains(&Hit::ReviewFilterAll));
    assert!(!ids.contains(&Hit::ReviewFilterRace));
    assert!(!ids.contains(&Hit::ReviewFilterPractice));
    assert!(!ids.contains(&Hit::ReviewFilterSaved));
    assert_golden("review-empty", &px);
    *UI.lock().unwrap() = None;
}

#[test]
fn review_empty_recording_on_paints_filters() {
    let _g = crate::review::serial();
    crate::review::reset();
    {
        let mut g = crate::config::CONFIG.lock().unwrap();
        g.review = true;
    }
    refresh_palette();
    let fonts = Fonts::for_family(FontFamily::Exo2).expect("Exo 2");
    let mut ui = dummy_ui(false);
    ui.tab = Tab::Review;
    *UI.lock().unwrap() = Some(ui);
    let mut px = Pixmap::new(1000, 720).expect("pixmap");
    draw(&mut px, &fonts, 1000.0, 720.0);
    let hits = UI.lock().unwrap().as_ref().unwrap().hits.clone();
    let ids = hit_ids(&hits);
    assert!(ids.contains(&Hit::ReviewFilterAll));
    assert!(ids.contains(&Hit::ReviewFilterRace));
    assert!(ids.contains(&Hit::ReviewFilterPractice));
    assert!(ids.contains(&Hit::ReviewFilterSaved));
    assert!(ids.contains(&Hit::ReviewToggle));
    *UI.lock().unwrap() = None;
    {
        let mut g = crate::config::CONFIG.lock().unwrap();
        g.review = false;
    }
}

#[test]
fn review_list_sheet_has_icon_hits() {
    let _g = crate::review::serial();
    let tmp = std::env::temp_dir().join("mxbo-review-list-hits");
    let _ = fs::remove_dir_all(&tmp);
    crate::review::init(tmp);
    crate::review::seed_demo().expect("demo");
    refresh_palette();
    let fonts = Fonts::for_family(FontFamily::Exo2).expect("Exo 2");
    let mut ui = dummy_ui(false);
    ui.tab = Tab::Review;
    *UI.lock().unwrap() = Some(ui);
    let mut px = Pixmap::new(1000, 720).expect("pixmap");
    draw(&mut px, &fonts, 1000.0, 720.0);
    let hits = UI.lock().unwrap().as_ref().unwrap().hits.clone();
    let ids = hit_ids(&hits);
    assert!(ids.iter().any(|h| matches!(h, Hit::ReviewOpen(_))));
    assert!(ids.iter().any(|h| matches!(h, Hit::ReviewDelete(_))));
    assert!(ids.iter().any(|h| matches!(h, Hit::ReviewKeep(_))));
    assert!(ids.contains(&Hit::ReviewToggle));
    assert!(ids.contains(&Hit::ReviewClear));
    *UI.lock().unwrap() = None;
    crate::review::reset();
}

#[test]
fn review_analyze_has_lap_map_and_compare_drop() {
    let _g = crate::review::serial();
    let tmp = std::env::temp_dir().join("mxbo-review-analyze-hits");
    let _ = fs::remove_dir_all(&tmp);
    crate::review::init(tmp);
    let id = crate::review::seed_demo().expect("demo");
    refresh_palette();
    let fonts = Fonts::for_family(FontFamily::Exo2).expect("Exo 2");
    let mut ui = dummy_ui(false);
    ui.tab = Tab::Review;
    ui.analyze_id = Some(id);
    ui.analyze_zoom = 2.0;
    *UI.lock().unwrap() = Some(ui);
    let mut px = Pixmap::new(1000, 920).expect("pixmap");
    draw(&mut px, &fonts, 1000.0, 920.0);
    let hits = UI.lock().unwrap().as_ref().unwrap().hits.clone();
    let ids = hit_ids(&hits);
    assert!(ids.contains(&Hit::AnalyzeLapOpen));
    assert!(ids.contains(&Hit::AnalyzeMap));
    assert!(ids.contains(&Hit::AnalyzeCompareOpen));
    assert!(ids.contains(&Hit::AnalyzeScrub));
    assert!(ids.contains(&Hit::AnalyzeFollow));
    assert!(ids.contains(&Hit::ReviewBack));
    assert!(ids.contains(&Hit::ReviewDelete(id as u64)));
    *UI.lock().unwrap() = None;
    crate::review::reset();
}

#[test]
fn review_follow_paint_stores_pan() {
    let _g = crate::review::serial();
    let tmp = std::env::temp_dir().join("mxbo-review-follow-pan");
    let _ = fs::remove_dir_all(&tmp);
    crate::review::init(tmp);
    let id = crate::review::seed_demo().expect("demo");
    refresh_palette();
    let fonts = Fonts::for_family(FontFamily::Exo2).expect("Exo 2");
    let mut ui = dummy_ui(false);
    ui.tab = Tab::Review;
    ui.analyze_id = Some(id);
    ui.analyze_zoom = 2.0;
    ui.analyze_follow = true;
    ui.analyze_scrub = 0.0;
    *UI.lock().unwrap() = Some(ui);
    let mut px = Pixmap::new(1000, 920).expect("pixmap");
    draw(&mut px, &fonts, 1000.0, 920.0);
    let want = follow_pan_for(id, -1, false, 0.0).expect("follow pan");
    let ui = UI.lock().unwrap();
    let u = ui.as_ref().unwrap();
    assert!(
        (u.analyze_pan_x - want.0).abs() < 1.0e-4 && (u.analyze_pan_z - want.1).abs() < 1.0e-4,
        "follow paint must store pan ({}, {}) got ({}, {})",
        want.0,
        want.1,
        u.analyze_pan_x,
        u.analyze_pan_z
    );
    drop(ui);
    *UI.lock().unwrap() = None;
    crate::review::reset();
}

#[test]
fn review_analyze_open_lap_drop_has_you_lap_hits() {
    let _g = crate::review::serial();
    let tmp = std::env::temp_dir().join("mxbo-review-analyze-lap-drop");
    let _ = fs::remove_dir_all(&tmp);
    crate::review::init(tmp);
    let id = crate::review::seed_demo().expect("demo");
    refresh_palette();
    let fonts = Fonts::for_family(FontFamily::Exo2).expect("Exo 2");
    let mut ui = dummy_ui(false);
    ui.tab = Tab::Review;
    ui.analyze_id = Some(id);
    ui.open_drop = Some(Drop::AnalyzeLap);
    *UI.lock().unwrap() = Some(ui);
    let mut px = Pixmap::new(1000, 920).expect("pixmap");
    draw(&mut px, &fonts, 1000.0, 920.0);
    {
        let ui = UI.lock().unwrap();
        let ui = ui.as_ref().unwrap();
        assert!(
            ui.drop_menu.is_some(),
            "open lap menu must reach overlay drop painter"
        );
        assert!(
            ui.hits
                .iter()
                .any(|h| matches!(h.id, Hit::AnalyzeYouLap(_))),
            "open lap menu must register AnalyzeYouLap hits"
        );
    }
    *UI.lock().unwrap() = None;

    let mut ui = dummy_ui(false);
    ui.tab = Tab::Review;
    ui.analyze_id = Some(id);
    ui.open_drop = Some(Drop::AnalyzeCompare);
    *UI.lock().unwrap() = Some(ui);
    draw(&mut px, &fonts, 1000.0, 920.0);
    let hits = UI.lock().unwrap().as_ref().unwrap().hits.clone();
    let ids = hit_ids(&hits);
    assert!(
        ids.iter().any(|h| matches!(h, Hit::AnalyzeCompare(_))),
        "open compare menu must register AnalyzeCompare hits"
    );
    *UI.lock().unwrap() = None;
    crate::review::reset();
}

#[test]
fn live_analyze_target_force_from_list() {
    assert_eq!(live_analyze_target(true, false, None, Some(7)), Some(7));
    assert_eq!(live_analyze_target(true, true, None, Some(7)), Some(7));
}

#[test]
fn live_analyze_target_skip_when_stay() {
    assert_eq!(live_analyze_target(false, true, None, Some(7)), None);
}

#[test]
fn live_analyze_target_skip_when_already_live() {
    assert_eq!(live_analyze_target(true, false, Some(7), Some(7)), None);
    assert_eq!(live_analyze_target(false, false, Some(7), Some(7)), None);
}

#[test]
fn live_analyze_target_paint_open_when_analyze_none() {
    assert_eq!(live_analyze_target(false, false, None, Some(7)), Some(7));
    assert_eq!(live_analyze_target(false, false, Some(3), Some(7)), None);
    assert_eq!(live_analyze_target(true, false, None, None), None);
}

#[test]
fn open_live_analyze_cases() {
    let _g = crate::review::serial();
    crate::review::reset();
    crate::review::set_live(7, "Hangtown");

    let mut ui = dummy_ui(false);
    ui.tab = Tab::Review;
    *UI.lock().unwrap() = Some(ui);
    open_live_analyze(true);
    let (id, stay, compare, you_lap, warmup, _) = snap_analyze();
    assert_eq!(id, Some(7), "force from list");
    assert!(!stay);
    assert_eq!(compare, -1);
    assert_eq!(you_lap, -1);
    assert!(!warmup);

    let mut ui = dummy_ui(false);
    ui.tab = Tab::Review;
    ui.review_stay_on_list = true;
    *UI.lock().unwrap() = Some(ui);
    open_live_analyze(false);
    let (id, stay, _, _, _, _) = snap_analyze();
    assert_eq!(id, None, "skip when stay");
    assert!(stay);

    let mut ui = dummy_ui(false);
    ui.tab = Tab::Review;
    ui.analyze_id = Some(7);
    ui.analyze_compare = 3;
    ui.analyze_you_lap = 2;
    ui.analyze_scrub = 0.4;
    *UI.lock().unwrap() = Some(ui);
    open_live_analyze(true);
    let (id, stay, compare, you_lap, _, scrub) = snap_analyze();
    assert_eq!(id, Some(7), "skip when already live");
    assert_eq!(compare, 3);
    assert_eq!(you_lap, 2);
    assert!((scrub - 0.4).abs() < 1.0e-6);
    assert!(!stay);

    crate::review::set_live(7, "Hangtown");
    let mut ui = dummy_ui(false);
    ui.tab = Tab::Review;
    *UI.lock().unwrap() = Some(ui);
    open_live_analyze(false);
    let (id, stay, _, _, _, _) = snap_analyze();
    assert_eq!(id, Some(7), "paint-open when analyze_id is None");
    assert!(!stay);

    *UI.lock().unwrap() = None;
    crate::review::reset();
}

#[test]
fn wrap_fb_does_not_split_words() {
    let fonts = Fonts::for_family(FontFamily::Exo2).expect("Exo 2");
    assert_eq!(wrap_fb(&fonts, "", 100.0, 16.0), vec![""]);
    assert_eq!(wrap_fb(&fonts, "a\n\nb", 400.0, 16.0), vec!["a", "", "b"]);
    let s = "Follow a rider in replay, sit or stand from Stance";
    let widest = s
        .split_whitespace()
        .map(|w| measure(&fonts, w, 16.0))
        .fold(0.0_f32, f32::max);
    let max_w = widest + 8.0;
    let lines = wrap_fb(&fonts, s, max_w, 16.0);
    for word in s.split_whitespace() {
        assert!(
            lines
                .iter()
                .any(|l| l.split_whitespace().any(|w| w == word)),
            "word {word:?} split across {lines:?}"
        );
    }
    assert!(lines.len() > 1, "expected wrapping, got {lines:?}");
}

#[test]
fn on_track_preset_strip_names_the_edit_and_live_slots() {
    assert_eq!(
        preset_strip_status(true, SessionPreset::Spectate, SessionPreset::Spectate),
        "On track — editing Spectate"
    );
    assert_eq!(
        preset_strip_status(true, SessionPreset::Race, SessionPreset::Warmup),
        "Editing Race — Warmup is live"
    );
    assert_eq!(
        preset_strip_status(false, SessionPreset::Race, SessionPreset::Warmup),
        "Editing Race — garage"
    );

    crate::config::sync_session_preset(Some(SessionPreset::Warmup));
    {
        let mut g = crate::config::CONFIG.lock().unwrap();
        g.settings_preset = SessionPreset::Race;
    }
    assert_eq!(
        hit_label(Hit::Preset(SessionPreset::Warmup)),
        "Warmup — live HUD"
    );
    assert_eq!(hit_label(Hit::Preset(SessionPreset::Race)), "Race");
    assert_eq!(
        hit_label(Hit::PresetCopyOpen),
        "Copy Race to another preset"
    );
    crate::config::sync_session_preset(None);
    {
        let mut g = crate::config::CONFIG.lock().unwrap();
        g.settings_preset = g.active_preset;
    }
}

#[test]
fn scrolled_widget_pane_keeps_preset_strip_on_top() {
    refresh_palette();
    crate::config::sync_session_preset(None);
    let fonts = Fonts::for_family(FontFamily::Exo2).expect("Exo 2");
    let mut ui = dummy_ui(false);
    ui.tab = Tab::Standings;
    ui.banner_dismissed = true;
    ui.scroll = 240.0;
    *UI.lock().unwrap() = Some(ui);
    let mut px = Pixmap::new(1000, 720).expect("pixmap");
    draw(&mut px, &fonts, 1000.0, 720.0);
    let hits = UI.lock().unwrap().as_ref().unwrap().hits.clone();
    *UI.lock().unwrap() = None;

    let race = hits
        .iter()
        .find(|h| h.id == Hit::Preset(SessionPreset::Race))
        .expect("race chip");
    assert!(
        race.y > TOP_H && race.y < TOP_H + 10.0 + PRESET_STRIP_H,
        "preset chip should stay under the top bar, y={}",
        race.y
    );

    let hx = race.x + race.w * 0.5;
    let hy = race.y + race.h * 0.5;
    assert!(matches!(
        hit_at(&hits, hx, hy),
        Some(Hit::Preset(SessionPreset::Race))
    ));

    let sticky_bottom = TOP_H + 10.0 + PRESET_STRIP_H;
    for h in &hits {
        if matches!(
            h.id,
            Hit::Preset(_) | Hit::PresetCopyOpen | Hit::PresetCopyTo(_) | Hit::PresetCopyAll
        ) {
            continue;
        }
        assert!(
            h.x + h.w <= SIDE_W || h.y + h.h <= TOP_H || h.y >= sticky_bottom,
            "pane hit overlaps pinned preset strip at y={} h={}",
            h.y,
            h.h
        );
    }
}

#[test]
fn on_track_preset_strip_keeps_copy_to_and_other_chips() {
    refresh_palette();
    crate::config::sync_session_preset(Some(SessionPreset::Warmup));
    {
        let mut g = crate::config::CONFIG.lock().unwrap();
        g.settings_preset = SessionPreset::Race;
    }
    let fonts = Fonts::for_family(FontFamily::Exo2).expect("Exo 2");
    let mut ui = dummy_ui(false);
    ui.tab = Tab::Standings;
    ui.banner_dismissed = true;
    *UI.lock().unwrap() = Some(ui);
    let mut px = Pixmap::new(1000, 720).expect("pixmap");
    draw(&mut px, &fonts, 1000.0, 720.0);
    let hits = UI.lock().unwrap().as_ref().unwrap().hits.clone();
    *UI.lock().unwrap() = None;
    crate::config::sync_session_preset(None);
    {
        let mut g = crate::config::CONFIG.lock().unwrap();
        g.settings_preset = g.active_preset;
    }

    assert!(hits
        .iter()
        .any(|h| h.id == Hit::Preset(SessionPreset::Practice)));
    assert!(hits
        .iter()
        .any(|h| h.id == Hit::Preset(SessionPreset::Race)));
    assert!(hits.iter().any(|h| h.id == Hit::PresetCopyOpen));
    assert!(!hits.iter().any(|h| h.id == Hit::PresetCopyAll));
}

#[test]
fn clamp_keeps_second_monitor_and_negative_origin() {
    let primary = [0, 0, 1920, 1080];
    let right = [1920, 0, 3840, 1080];
    let left = [-1920, 0, 0, 1080];
    let dual = [primary, right];
    assert_eq!(
        clamp_settings_origin(2000, 80, SETTINGS_W, SETTINGS_H, &dual),
        (2000, 80)
    );
    assert_eq!(
        clamp_settings_origin(-1800, 80, SETTINGS_W, SETTINGS_H, &[left, primary]),
        (-1800, 80)
    );
    assert_eq!(
        clamp_settings_origin(80, 80, SETTINGS_W, SETTINGS_H, &dual),
        (80, 80)
    );
}

#[test]
fn clamp_snaps_to_nearest_when_monitor_is_gone() {
    let primary = [0, 0, 1920, 1080];
    assert_eq!(
        clamp_settings_origin(2000, 80, SETTINGS_W, SETTINGS_H, &[primary]),
        (920, 80)
    );
    assert_eq!(
        clamp_settings_origin(80, 80, SETTINGS_W, SETTINGS_H, &[]),
        (80, 80)
    );
}

#[test]
fn profile_tab_hidden_when_motos_off() {
    let _g = crate::review::serial();
    crate::review::reset();
    {
        let mut g = crate::config::CONFIG.lock().unwrap();
        g.review = false;
    }
    refresh_palette();
    let fonts = Fonts::for_family(FontFamily::Exo2).expect("Exo 2");
    let mut ui = dummy_ui(false);
    ui.tab = Tab::Profile;
    *UI.lock().unwrap() = Some(ui);
    let mut px = Pixmap::new(1000, 720).expect("pixmap");
    draw(&mut px, &fonts, 1000.0, 720.0);
    let hits = UI.lock().unwrap().as_ref().unwrap().hits.clone();
    let ids = hit_ids(&hits);
    assert!(!ids.contains(&Hit::TabProfile));
    assert!(ids.contains(&Hit::TabReview));
    *UI.lock().unwrap() = None;
}

#[test]
fn profile_empty_paints_teach_copy() {
    let _g = crate::review::serial();
    crate::review::reset();
    {
        let mut g = crate::config::CONFIG.lock().unwrap();
        g.review = true;
    }
    refresh_palette();
    let fonts = Fonts::for_family(FontFamily::Exo2).expect("Exo 2");
    let mut ui = dummy_ui(false);
    ui.tab = Tab::Profile;
    *UI.lock().unwrap() = Some(ui);
    let mut px = Pixmap::new(1000, 720).expect("pixmap");
    draw(&mut px, &fonts, 1000.0, 720.0);
    let hits = UI.lock().unwrap().as_ref().unwrap().hits.clone();
    let ids = hit_ids(&hits);
    assert!(ids.contains(&Hit::TabProfile));
    assert!(ids.contains(&Hit::ProfileAllTime));
    assert!(ids.contains(&Hit::ProfileTwoWeeks));
    assert!(!ids.contains(&Hit::ProfileClear));
    assert_golden("profile-empty", &px);
    *UI.lock().unwrap() = None;
    {
        let mut g = crate::config::CONFIG.lock().unwrap();
        g.review = false;
    }
}

#[test]
fn profile_demo_paints_spider() {
    let _g = crate::review::serial();
    let tmp = std::env::temp_dir().join("mxbo-profile-demo");
    let _ = fs::remove_dir_all(&tmp);
    crate::review::reset();
    crate::review::init(tmp);
    crate::review::seed_demo().expect("demo");
    {
        let mut g = crate::config::CONFIG.lock().unwrap();
        g.review = true;
    }
    refresh_palette();
    let fonts = Fonts::for_family(FontFamily::Exo2).expect("Exo 2");
    let mut ui = dummy_ui(false);
    ui.tab = Tab::Profile;
    *UI.lock().unwrap() = Some(ui);
    let mut px = Pixmap::new(1000, 720).expect("pixmap");
    draw(&mut px, &fonts, 1000.0, 720.0);
    let hits = UI.lock().unwrap().as_ref().unwrap().hits.clone();
    let ids = hit_ids(&hits);
    assert!(ids.contains(&Hit::ProfileAllTime));
    assert!(ids.contains(&Hit::ProfileClear));
    for i in 0..8 {
        assert!(ids.contains(&Hit::ProfileAxis(i)), "missing axis {i}");
    }
    assert_eq!(
        hit_label(Hit::ProfileAxis(0)),
        "Your best lap vs the fastest in that race."
    );
    assert_golden("profile", &px);
    *UI.lock().unwrap() = None;
    crate::review::reset();
    {
        let mut g = crate::config::CONFIG.lock().unwrap();
        g.review = false;
    }
}

#[test]
fn profile_clear_confirm_paints_buttons() {
    let _g = crate::review::serial();
    refresh_palette();
    let fonts = Fonts::for_family(FontFamily::Exo2).expect("Exo 2");
    let mut ui = dummy_ui(false);
    ui.tab = Tab::Profile;
    ui.clear_confirm = Some(ClearKind::Profile);
    *UI.lock().unwrap() = Some(ui);
    let mut px = Pixmap::new(1000, 720).expect("pixmap");
    draw(&mut px, &fonts, 1000.0, 720.0);
    let hits = UI.lock().unwrap().as_ref().unwrap().hits.clone();
    let ids = hit_ids(&hits);
    assert!(ids.contains(&Hit::ClearScrim));
    assert!(ids.contains(&Hit::ClearCancel));
    assert!(ids.contains(&Hit::ClearConfirm));
    assert_eq!(hit_label(Hit::ClearConfirm), "Clear");
    assert_golden("clear-profile", &px);
    *UI.lock().unwrap() = None;
}
