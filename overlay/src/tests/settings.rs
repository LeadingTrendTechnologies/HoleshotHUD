use super::*;
use std::fs;
use std::path::PathBuf;

fn dummy_ui(open: bool) -> SettingsUi {
    SettingsUi {
        host: HWND::default(),
        tab: Tab::App,
        last_widget: Tab::Standings,
        hover: None,
        focus: None,
        hits: Vec::new(),
        open_drop: None,
        drag: None,
        slide: None,
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
    let bytes = fs::read(&path).unwrap_or_else(|_| {
        panic!("missing golden {name}.png — run with UPDATE_GOLDENS=1")
    });
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
            lines.iter().any(|l| l.split_whitespace().any(|w| w == word)),
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
    assert_eq!(hit_label(Hit::Preset(SessionPreset::Warmup)), "Warmup — live HUD");
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

    assert!(hits.iter().any(|h| h.id == Hit::Preset(SessionPreset::Practice)));
    assert!(hits.iter().any(|h| h.id == Hit::Preset(SessionPreset::Race)));
    assert!(hits.iter().any(|h| h.id == Hit::PresetCopyOpen));
    assert!(!hits.iter().any(|h| h.id == Hit::PresetCopyAll));
}
