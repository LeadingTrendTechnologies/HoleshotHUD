use super::*;

const SAMPLE: &str = "\
# Changelog

## Unreleased

Click a rider on standings to follow them in replay.

### Overlay

- Clicking a name on Standings moves the camera.

## 0.1.18

Closing MX Bikes brings the Windows taskbar back.

### Overlay

- Closing the game restores the taskbar

## 0.1.17

Settings is a working board with an orange Show plaque.

### Settings

- Top mode bar is Widgets / Settings / Feedback
- App → Updates shows the install folder

### Overlay

- The HUD stays visible while Settings is in front
";

#[test]
fn reads_named_version_headline_and_bullets() {
    let n = notes_for(SAMPLE, "0.1.17").unwrap();
    assert_eq!(n.version, "0.1.17");
    assert_eq!(
        n.headline,
        "Settings is a working board with an orange Show plaque."
    );
    assert_eq!(n.sections.len(), 2);
    assert_eq!(n.sections[0].title, "Settings");
    assert_eq!(n.sections[0].bullets.len(), 2);
    assert_eq!(n.sections[1].title, "Overlay");
    assert_eq!(
        n.sections[1].bullets[0],
        "The HUD stays visible while Settings is in front"
    );
}

#[test]
fn named_version_wins_over_unreleased() {
    let n = notes_for(SAMPLE, "0.1.18").unwrap();
    assert_eq!(
        n.headline,
        "Closing MX Bikes brings the Windows taskbar back."
    );
    assert!(!n.headline.contains("Click a rider"));
}

#[test]
fn missing_version_falls_back_to_unreleased() {
    let n = notes_for(SAMPLE, "0.1.19").unwrap();
    assert_eq!(n.version, "0.1.19");
    assert!(n.headline.contains("Click a rider"));
}

#[test]
fn heading_does_not_match_a_longer_version() {
    const VER: &str = "\
## 0.1.1

One.

## 0.1.10

Ten.
";
    assert_eq!(notes_for(VER, "0.1.1").unwrap().headline, "One.");
    assert_eq!(notes_for(VER, "0.1.10").unwrap().headline, "Ten.");
    assert_eq!(notes_for(VER, "v0.1.1").unwrap().headline, "One.");
}

#[test]
fn contains_needle_needs_word_boundaries() {
    assert!(contains_needle("standings board", "standings"));
    assert!(!contains_needle("understandings", "standings"));
    assert!(overlay_wide("Closing MX Bikes brings the taskbar back"));
    assert!(!overlay_wide("Standings row colors"));
}

#[test]
fn auto_open_only_after_in_app_update() {
    assert!(should_auto_open("0.1.17", "0.1.18", true));
    assert!(!should_auto_open("0.1.18", "0.1.18", true));
    assert!(!should_auto_open("0.1.17", "0.1.18", false));
    assert!(!should_auto_open("", "0.1.18", false));
    assert!(should_auto_open("", "0.1.18", true));
}

#[test]
fn bump_patch_ticks_the_last_number() {
    assert_eq!(bump_patch("0.1.19"), "0.1.20");
    assert_eq!(bump_patch("v1.2.9"), "1.2.10");
}

#[test]
fn next_notes_stamp_unreleased_as_the_next_patch() {
    let current = crate::update::current_version();
    let unreleased = parse_section(CHANGELOG, "Unreleased")
        .and_then(rider_facing)
        .filter(|n| !n.is_empty());
    if let Some(n) = unreleased {
        let next = next_notes().expect("unreleased notes");
        assert_eq!(next.version, bump_patch(current));
        assert!(!n.headline.is_empty() || !n.sections.is_empty());
        assert!(next.sections.iter().all(|s| !internal_section(&s.title)));
    } else if let Some(n) = next_notes() {
        assert_eq!(n.version, current);
        assert!(n.sections.iter().all(|s| !internal_section(&s.title)));
    } else {
        assert!(
            parse_section(CHANGELOG, current).is_some(),
            "shipped version should still be in CHANGELOG"
        );
    }
}

#[test]
fn cargo_run_is_a_preview_build() {
    assert!(previewing());
    assert_eq!(
        modal_notes().as_ref().map(|n| n.version.as_str()),
        next_notes().as_ref().map(|n| n.version.as_str())
    );
}

#[test]
fn shipped_changelog_has_current_or_unreleased_notes() {
    let current = crate::update::current_version();
    assert!(
        parse_section(CHANGELOG, current).is_some()
            || parse_section(CHANGELOG, "Unreleased").is_some(),
        "CHANGELOG must mention this version or Unreleased"
    );
    if let Some(n) = current_notes() {
        assert!(!n.headline.is_empty() || !n.sections.is_empty());
        assert!(n.sections.iter().all(|s| !internal_section(&s.title)));
        assert!(n
            .sections
            .iter()
            .flat_map(|s| s.bullets.iter())
            .all(|b| !internal_bullet(b)));
    }
}

#[test]
fn internals_only_version_has_no_whats_new_modal() {
    assert!(
        notes_for(CHANGELOG, "0.3.1").is_none(),
        "0.3.1 is internals-only; What's new should not open"
    );
}

#[test]
fn modal_keeps_rider_notes_and_drops_internals() {
    const MIXED: &str = "\
## 0.2.0

Click a rider on standings to follow them in replay.

### Overlay

- Clicking a name on Standings moves the camera.
- The HUD shows whenever standings are in the snapshot (replays never set `on_track`).
- Shared memory mapping is `Local\\MXBOHudV9`.
- F9 SHM dump includes session_kind so a flag complaint can be diagnosed from a log.

### Website

- Browser demo rebuilt with the current HUD renderer.

### Other

- Pushing a tag runs the Release workflow.
";
    let n = notes_for(MIXED, "0.2.0").unwrap();
    assert_eq!(n.headline, "Click a rider on standings to follow them in replay.");
    assert_eq!(n.sections.len(), 1);
    assert_eq!(n.sections[0].title, "Standings");
    assert_eq!(
        n.sections[0].bullets,
        vec!["Clicking a name on Standings moves the camera."]
    );
}

#[test]
fn modal_strips_markdown() {
    const MARK: &str = "\
## 0.2.1

Settings is a working board.

### Settings

- Top mode bar is **Widgets** / **Settings**
- Dash shows a `~Lapped` tag
";
    let n = notes_for(MARK, "0.2.1").unwrap();
    assert_eq!(n.sections[0].title, "Dash");
    assert_eq!(n.sections[0].bullets[0], "Dash shows a ~Lapped tag");
    assert_eq!(n.sections[1].title, "Settings");
    assert_eq!(n.sections[1].bullets[0], "Top mode bar is Widgets / Settings");
}

#[test]
fn modal_splits_overlay_and_settings_by_widget() {
    const MIXED: &str = "\
## 0.2.2

Follow a rider, sit or stand, and see what changed.

### Settings

- App shows the MX Bikes folder.
- Stance is sit/stand. It lives with Dash and Systems.
- Dash can be just gear and speed.
- Standings and Relative can turn off alternating row colors.
- Labs unlocks Sectors.

### Overlay

- In replay, click a name on Standings to follow that rider.
- Map and minimap treat the rider you are watching as you.
- The HUD stays up in replay. Systems and Stance hide there too.
- Simple dash is an orange gear plaque.
";
    let n = notes_for(MIXED, "0.2.2").unwrap();
    let titles: Vec<&str> = n.sections.iter().map(|s| s.title.as_str()).collect();
    assert_eq!(
        titles,
        [
            "Standings",
            "Relative",
            "Map",
            "Minimap",
            "Dash",
            "Stance",
            "Sectors",
            "Settings",
            "Overlay"
        ]
    );
    assert!(n.sections[0].bullets.iter().any(|b| b.contains("click a name")));
    assert!(n.sections[0].bullets.iter().any(|b| b.contains("alternating row colors")));
    assert!(n.sections[1].bullets.iter().any(|b| b.contains("alternating row colors")));
    assert_eq!(n.sections[2].bullets.len(), 1);
    assert_eq!(n.sections[3].bullets.len(), 1);
    assert!(n.sections[4].bullets.iter().any(|b| b.contains("gear and speed")));
    assert_eq!(n.sections[5].title, "Stance");
    assert!(n.sections[5].bullets[0].starts_with("Stance is sit/stand"));
    assert_eq!(n.sections[6].bullets, vec!["Labs unlocks Sectors."]);
    assert_eq!(n.sections[7].bullets, vec!["App shows the MX Bikes folder."]);
    assert_eq!(
        n.sections[8].bullets,
        vec!["The HUD stays up in replay. Systems and Stance hide there too."]
    );
}

#[test]
fn shipped_notes_are_grouped_by_widget() {
    let Some(n) = next_notes() else {
        return;
    };
    let titles: Vec<&str> = n.sections.iter().map(|s| s.title.as_str()).collect();
    assert!(
        !titles.contains(&"Website"),
        "website notes stay out of What's new: {titles:?}"
    );
    assert!(
        !titles.iter().any(|t| t.eq_ignore_ascii_case("overlay")),
        "0.2.0 overlay notes are internals: {titles:?}"
    );
}
