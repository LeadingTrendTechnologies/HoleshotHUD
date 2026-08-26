//! This version's CHANGELOG notes for the What's new modal.

const CHANGELOG: &str = include_str!("../../CHANGELOG.md");

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notes {
    pub version: String,
    pub headline: String,
    pub sections: Vec<Section>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    pub title: String,
    pub bullets: Vec<String>,
}

impl Notes {
    pub fn is_empty(&self) -> bool {
        self.headline.is_empty() && self.sections.iter().all(|s| s.bullets.is_empty())
    }
}

pub fn just_updated() -> bool {
    std::env::args().any(|a| a == "--whats-new") || running_from_build_tree()
}

fn running_from_build_tree() -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let s = exe.to_string_lossy().to_ascii_lowercase();
    s.contains(r"\target\release\")
        || s.contains(r"\target\debug\")
        || s.contains("/target/release/")
        || s.contains("/target/debug/")
}

pub fn current_notes() -> Option<Notes> {
    notes_for(CHANGELOG, crate::update::current_version())
}

pub fn should_auto_open(seen: &str, current: &str, just_updated: bool) -> bool {
    just_updated && seen != current && notes_for(CHANGELOG, current).is_some()
}

pub fn notes_for(changelog: &str, version: &str) -> Option<Notes> {
    let ver = version.trim().trim_start_matches('v');
    let named = parse_section(changelog, ver).and_then(rider_facing);
    if named.as_ref().is_some_and(|n| !n.is_empty()) {
        return named;
    }
    parse_section(changelog, "Unreleased")
        .and_then(rider_facing)
        .filter(|n| !n.is_empty() && named.is_none())
        .map(|mut n| {
            n.version = ver.to_string();
            n
        })
}

/// Keep the headline and rider-facing bullets. Skip developer sections and internals.
fn rider_facing(notes: Notes) -> Option<Notes> {
    let notes = Notes {
        version: notes.version,
        headline: plain(&notes.headline),
        sections: notes
            .sections
            .into_iter()
            .filter(|s| !internal_section(&s.title))
            .map(|s| Section {
                title: s.title,
                bullets: s
                    .bullets
                    .into_iter()
                    .filter(|b| !internal_bullet(b))
                    .map(|b| plain(&b))
                    .collect(),
            })
            .filter(|s| !s.bullets.is_empty())
            .collect(),
    };
    if notes.is_empty() {
        None
    } else {
        Some(notes)
    }
}

fn internal_section(title: &str) -> bool {
    matches!(
        title.trim().to_ascii_lowercase().as_str(),
        "other" | "website" | "internals"
    )
}

fn internal_bullet(s: &str) -> bool {
    let t = s.to_ascii_lowercase();
    const MARKERS: &[&str] = &[
        "shm",
        "shared memory",
        "on_track",
        "show_*=",
        ".dlo",
        ".jsonl",
        "cargo run",
        "f9 ",
        "m_isession",
        "mxbohud",
        "announce-shot",
        "plugin session",
        "plugin's value",
        "seqlock",
        "in the snapshot",
        "race trace",
        "from a log",
        "mxbo.ini",
    ];
    MARKERS.iter().any(|m| t.contains(m))
}

fn plain(s: &str) -> String {
    s.replace("**", "").replace('`', "")
}

fn parse_section(changelog: &str, heading: &str) -> Option<Notes> {
    let needle = format!("## {heading}");
    let start = find_heading(changelog, &needle)?;
    let rest = &changelog[start..];
    let body = match rest.find("\n## ") {
        Some(end) => &rest[..end],
        None => rest,
    };
    let mut headline = String::new();
    let mut sections: Vec<Section> = Vec::new();
    let mut cur: Option<Section> = None;
    for raw in body.lines().skip(1) {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(title) = line.strip_prefix("### ") {
            if let Some(sec) = cur.take() {
                if !sec.bullets.is_empty() {
                    sections.push(sec);
                }
            }
            cur = Some(Section {
                title: title.trim().to_string(),
                bullets: Vec::new(),
            });
            continue;
        }
        if let Some(bullet) = line.strip_prefix("- ") {
            let text = bullet.trim().to_string();
            if text.is_empty() {
                continue;
            }
            match cur.as_mut() {
                Some(sec) => sec.bullets.push(text),
                None => {
                    cur = Some(Section {
                        title: String::new(),
                        bullets: vec![text],
                    });
                }
            }
            continue;
        }
        if headline.is_empty() && !line.starts_with('#') {
            headline = line.to_string();
        }
    }
    if let Some(sec) = cur.take() {
        if !sec.bullets.is_empty() {
            sections.push(sec);
        }
    }
    let notes = Notes {
        version: heading.trim_start_matches('v').to_string(),
        headline,
        sections,
    };
    if notes.is_empty() {
        None
    } else {
        Some(notes)
    }
}

fn find_heading(changelog: &str, needle: &str) -> Option<usize> {
    let mut from = 0;
    while from < changelog.len() {
        let i = changelog[from..].find(needle)?;
        let abs = from + i;
        let at_line = abs == 0 || changelog.as_bytes().get(abs - 1) == Some(&b'\n');
        let after = abs + needle.len();
        let boundary = changelog
            .as_bytes()
            .get(after)
            .is_none_or(|c| c.is_ascii_whitespace());
        if at_line && boundary {
            return Some(abs);
        }
        from = after;
    }
    None
}

#[cfg(test)]
mod tests {
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
    fn auto_open_only_after_in_app_update() {
        assert!(should_auto_open("0.1.17", "0.1.18", true));
        assert!(!should_auto_open("0.1.18", "0.1.18", true));
        assert!(!should_auto_open("0.1.17", "0.1.18", false));
        assert!(!should_auto_open("", "0.1.18", false));
        assert!(should_auto_open("", "0.1.18", true));
    }

    #[test]
    fn shipped_changelog_has_current_or_unreleased_notes() {
        let n = current_notes().expect("notes");
        assert!(!n.headline.is_empty());
        assert!(n.sections.iter().all(|s| !internal_section(&s.title)));
        assert!(n
            .sections
            .iter()
            .flat_map(|s| s.bullets.iter())
            .all(|b| !internal_bullet(b)));
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
        assert_eq!(n.sections[0].title, "Overlay");
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
        assert_eq!(n.sections[0].bullets[0], "Top mode bar is Widgets / Settings");
        assert_eq!(n.sections[0].bullets[1], "Dash shows a ~Lapped tag");
    }
}
