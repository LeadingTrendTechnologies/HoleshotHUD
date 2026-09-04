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
    force_whats_new() || running_from_build_tree()
}

pub fn force_whats_new() -> bool {
    std::env::args().any(|a| a == "--whats-new" || a == "--dump-whats-new")
}

/// Dev builds and `--whats-new` show Unreleased notes as the next update board.
pub fn previewing() -> bool {
    force_whats_new() || running_from_build_tree()
}

fn running_from_build_tree() -> bool {
    if std::env::var_os("CARGO").is_some() {
        return true;
    }
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let s = exe.to_string_lossy().to_ascii_lowercase();
    s.contains(r"\target\release\")
        || s.contains(r"\target\debug\")
        || s.contains(r"\cargo-target\release\")
        || s.contains(r"\cargo-target\debug\")
        || s.contains("/target/release/")
        || s.contains("/target/debug/")
        || s.contains("/cargo-target/release/")
        || s.contains("/cargo-target/debug/")
}

pub fn current_notes() -> Option<Notes> {
    notes_for(CHANGELOG, crate::update::current_version())
}

/// Notes the What's new board will show: Unreleased (next ship) while previewing, else this version.
pub fn modal_notes() -> Option<Notes> {
    if previewing() {
        next_notes()
    } else {
        current_notes()
    }
}

/// Rider-facing Unreleased copy, stamped with the next patch number. Falls back to this version.
pub fn next_notes() -> Option<Notes> {
    let current = crate::update::current_version();
    let next = bump_patch(current);
    if let Some(mut n) = parse_section(CHANGELOG, "Unreleased")
        .and_then(rider_facing)
        .filter(|n| !n.is_empty())
    {
        n.version = next;
        return Some(n);
    }
    notes_for(CHANGELOG, current)
}

pub fn format_notes(notes: &Notes) -> String {
    let mut o = format!("{}\n{}\n", notes.version, notes.headline);
    for sec in &notes.sections {
        o.push('\n');
        o.push_str(&sec.title);
        o.push('\n');
        for b in &sec.bullets {
            o.push_str("- ");
            o.push_str(b);
            o.push('\n');
        }
    }
    o
}

fn bump_patch(ver: &str) -> String {
    let v = ver.trim().trim_start_matches('v');
    let mut parts: Vec<u32> = v
        .split('.')
        .take(3)
        .map(|p| {
            p.chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse()
                .unwrap_or(0)
        })
        .collect();
    while parts.len() < 3 {
        parts.push(0);
    }
    parts[2] = parts[2].saturating_add(1);
    format!("{}.{}.{}", parts[0], parts[1], parts[2])
}

pub fn should_auto_open(seen: &str, current: &str, just_updated: bool) -> bool {
    if !just_updated {
        return false;
    }
    if force_whats_new() && modal_notes().is_some() {
        return true;
    }
    seen != current && notes_for(CHANGELOG, current).is_some()
}

pub fn notes_for(changelog: &str, version: &str) -> Option<Notes> {
    let ver = version.trim().trim_start_matches('v');
    if let Some(section) = parse_section(changelog, ver) {
        return rider_facing(section).filter(|n| !n.is_empty());
    }
    parse_section(changelog, "Unreleased")
        .and_then(rider_facing)
        .filter(|n| !n.is_empty())
        .map(|mut n| {
            n.version = ver.to_string();
            n
        })
}

/// Keep the headline and rider-facing bullets. Skip developer sections and internals.
/// Overlay / Settings / widget-named buckets are split so the modal is per widget.
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
    let notes = split_by_widget(notes);
    if notes.is_empty() {
        None
    } else {
        Some(notes)
    }
}

/// Widget rail names. Overlay / Settings bullets that name a widget move under that heading.
const WIDGETS: &[(&str, &[&str])] = &[
    ("Standings", &["standings"]),
    ("Relative", &["relative"]),
    ("H-Standings", &["horizontal standings", "h-standings", "h standings"]),
    ("Map", &["map"]),
    ("Minimap", &["minimap"]),
    ("Radar", &["radar"]),
    ("Dash", &["dash"]),
    ("Lean", &["lean"]),
    ("Gamepad", &["gamepad", "controller"]),
    ("Systems", &["systems"]),
    ("Stance", &["stance"]),
    ("Sectors", &["sectors", "sector"]),
    ("Delta Bar", &["delta bar"]),
    ("Flags", &["flags widget"]),
];

fn split_by_widget(notes: Notes) -> Notes {
    let mut by_widget: Vec<Vec<String>> = vec![Vec::new(); WIDGETS.len()];
    let mut kept: Vec<Section> = Vec::new();
    let mut leftovers: Vec<(String, Vec<String>)> = Vec::new();

    for sec in notes.sections {
        if overlay_or_settings(&sec.title) {
            let mut unassigned = Vec::new();
            for b in sec.bullets {
                if overlay_wide(&b) {
                    unassigned.push(b);
                    continue;
                }
                let idx = widget_indices(&b);
                if idx.is_empty() {
                    unassigned.push(b);
                } else {
                    for i in idx {
                        if !by_widget[i].iter().any(|x| x == &b) {
                            by_widget[i].push(b.clone());
                        }
                    }
                }
            }
            if !unassigned.is_empty() {
                leftovers.push((sec.title, unassigned));
            }
            continue;
        }
        kept.push(sec);
    }

    let mut sections = Vec::new();
    for (i, bullets) in by_widget.into_iter().enumerate() {
        if !bullets.is_empty() {
            sections.push(Section {
                title: WIDGETS[i].0.to_string(),
                bullets,
            });
        }
    }
    for sec in kept {
        if let Some(existing) = sections.iter_mut().find(|s| s.title.eq_ignore_ascii_case(&sec.title)) {
            for b in sec.bullets {
                if !existing.bullets.iter().any(|x| x == &b) {
                    existing.bullets.push(b);
                }
            }
        } else {
            sections.push(sec);
        }
    }
    for (title, bullets) in leftovers {
        if let Some(existing) = sections.iter_mut().find(|s| s.title.eq_ignore_ascii_case(&title)) {
            for b in bullets {
                if !existing.bullets.iter().any(|x| x == &b) {
                    existing.bullets.push(b);
                }
            }
        } else {
            sections.push(Section { title, bullets });
        }
    }

    Notes {
        version: notes.version,
        headline: notes.headline,
        sections,
    }
}

fn overlay_or_settings(title: &str) -> bool {
    matches!(
        title.trim().to_ascii_lowercase().as_str(),
        "overlay" | "settings" | "app"
    )
}

/// HUD-wide notes stay Overlay even if a widget is named as an example.
fn overlay_wide(s: &str) -> bool {
    let t = s.to_ascii_lowercase();
    const MARKERS: &[&str] = &[
        "the hud",
        "holeshot hud",
        "holeshot hud icon",
        "holeshot hud mark",
        "settings key",
        "open when mx bikes",
        "close when mx bikes",
        "plaque on the game",
        "mx bikes is not sending",
        "clicks go to the game",
        "leave replay and the hud",
        "taskbar",
    ];
    MARKERS.iter().any(|m| t.contains(m))
}

fn widget_indices(text: &str) -> Vec<usize> {
    let first = text.split('.').next().unwrap_or(text);
    let in_first = widget_indices_all(first);
    if !in_first.is_empty() {
        in_first
    } else {
        widget_indices_all(text)
    }
}

fn widget_indices_all(text: &str) -> Vec<usize> {
    let l = text.to_ascii_lowercase();
    let mut out = Vec::new();
    for (i, (_title, needles)) in WIDGETS.iter().enumerate() {
        if needles.iter().any(|n| contains_needle(&l, n)) {
            out.push(i);
        }
    }
    let h_idx = WIDGETS.iter().position(|(t, _)| *t == "H-Standings");
    let st_idx = WIDGETS.iter().position(|(t, _)| *t == "Standings");
    if let (Some(h), Some(st)) = (h_idx, st_idx) {
        if out.contains(&h) && !standings_besides_horizontal(&l) {
            out.retain(|&i| i != st);
        }
    }
    out
}

fn standings_besides_horizontal(l: &str) -> bool {
    let stripped = l
        .replace("horizontal standings", " ")
        .replace("h-standings", " ")
        .replace("h standings", " ");
    contains_needle(&stripped, "standings")
}

fn contains_needle(hay: &str, needle: &str) -> bool {
    let mut from = 0;
    while from + needle.len() <= hay.len() {
        let rest = &hay[from..];
        let Some(rel) = rest.find(needle) else {
            return false;
        };
        let abs = from + rel;
        let before_ok = abs == 0
            || !hay.as_bytes()[abs - 1].is_ascii_alphanumeric();
        let after = abs + needle.len();
        let after_ok = after >= hay.len()
            || !hay.as_bytes()[after].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return true;
        }
        from = abs + 1;
    }
    false
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
        "mongodb",
        "gist token",
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
#[path = "tests/changelog.rs"]
mod tests;
