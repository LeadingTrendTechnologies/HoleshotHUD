//! Shared UI file I/O — stock `.mnu` reads, text writes, english.str patches.

use std::fs;
use std::path::Path;

pub(crate) fn write_text(path: &Path, body: &str) -> Result<(), String> {
    fs::write(path, body).map_err(|e| e.to_string())
}

/// Stock dest textids (`Testing` / `Single_race`) keep main.fnt; remap their english.str values.
pub(crate) fn write_english_labels(ui: &Path, bak: Option<&Path>) -> Result<(), String> {
    let from_bak = bak
        .map(|b| b.join("english.str"))
        .filter(|p| p.is_file());
    let from_ui = Some(ui.join("english.str")).filter(|p| p.is_file());
    let raw = if let Some(src) = from_bak.as_ref().or(from_ui.as_ref()) {
        fs::read_to_string(src).map_err(|e| e.to_string())?
    } else if let Some(game) = ui.parent() {
        match read_ui_pkz_text(game, "english.str") {
            Some(s) => s,
            None => return Ok(()),
        }
    } else {
        return Ok(());
    };
    let patched = patch_str_pair(&raw, "Testing", "Practice");
    let patched = patch_str_pair(&patched, "Single_race", "Online");
    write_text(&ui.join("english.str"), &patched)
}

pub(crate) fn patch_str_pair(raw: &str, key: &str, value: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 16);
    let mut lines = raw.split_inclusive('\n');
    while let Some(line) = lines.next() {
        out.push_str(line);
        let id = line.trim_end_matches(['\r', '\n']);
        if id != key {
            continue;
        }
        match lines.next() {
            Some(old) => {
                let ending = if old.ends_with("\r\n") {
                    "\r\n"
                } else if old.ends_with('\n') {
                    "\n"
                } else {
                    ""
                };
                out.push_str(value);
                out.push_str(ending);
            }
            None => out.push_str(value),
        }
    }
    out
}

pub(crate) fn stock_multijoin(ui: &Path, bak: Option<&Path>) -> Option<String> {
    stock_mnu(ui, bak, "multijoin.mnu")
}

pub(crate) fn stock_options(ui: &Path, bak: Option<&Path>) -> Option<String> {
    stock_mnu(ui, bak, "options.mnu")
}

pub(crate) fn stock_mnu(ui: &Path, bak: Option<&Path>, name: &str) -> Option<String> {
    if let Some(bak) = bak {
        let p = bak.join(name);
        if p.is_file() {
            return fs::read_to_string(p).ok();
        }
    }
    let p = ui.join(name);
    if p.is_file() {
        return fs::read_to_string(p).ok();
    }
    let game = ui.parent()?;
    read_ui_pkz_text(game, name)
}

pub(crate) fn read_ui_pkz_text(game: &Path, name: &str) -> Option<String> {
    let pkz = game.join("ui.pkz");
    if !pkz.is_file() {
        return None;
    }
    let file = fs::File::open(pkz).ok()?;
    let mut zip = zip::ZipArchive::new(file).ok()?;
    let key = format!("ui/{name}");
    let mut entry = zip.by_name(&key).ok()?;
    let mut raw = String::new();
    std::io::Read::read_to_string(&mut entry, &mut raw).ok()?;
    Some(raw)
}
