use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::config::format_primary_color;

use super::io::{write_english_labels, write_text};
use super::mnu::{
    main_mnu, splice_connection, splice_garage, splice_hostsetup, splice_multiclient,
    splice_multijoin, splice_options, splice_profiles, splice_replay, splice_test,
    splice_testsetup, splice_viewreplays, ui_ui,
};
use super::screens::{write_screen_images, LOADING_TGA, SPLASH_TGA};
use super::sprites::{
    write_accent_pointer, write_spin_arrows, write_sprites, SPRITES, STOCK_CHROME,
};
use super::{BAK_DIR, MANIFEST};

/// Bump when MNU splice / sprite pack logic changes so accent-matched sync still rewrites.
pub(crate) const PACK_REV: u32 = 5;

pub(crate) static NEED_GAME_RESTART: AtomicBool = AtomicBool::new(false);

pub(crate) static NEED_RETRY: AtomicBool = AtomicBool::new(false);

pub fn needs_restart() -> bool {
    NEED_GAME_RESTART.load(Ordering::Relaxed)
}

pub fn needs_retry() -> bool {
    NEED_RETRY.load(Ordering::Relaxed)
}

/// Re-run sync after a prior apply/remove failed (e.g. MX Bikes had `ui/` locked).
pub fn retry_if_needed() {
    if needs_retry() {
        sync_from_config();
    }
}

/// Force a full pack rewrite (image path changes, accent already current).
pub fn sync_forced() {
    NEED_RETRY.store(true, Ordering::Relaxed);
    sync_from_config();
}

/// Apply or remove from the resolved MX Bikes folder using the live config.

/// Apply or remove from the resolved MX Bikes folder using the live config.
pub fn sync_from_config() {
    sync_impl(true);
}

/// Sync pack to Look primary without requesting a menu restart (game just launched).

/// Sync pack to Look primary without requesting a menu restart (game just launched).
pub fn sync_quiet() {
    sync_impl(false);
}

pub(crate) fn sync_impl(mark_restart_if_game_on: bool) {
    let Some(game) = crate::plugin::game_dir() else {
        return;
    };
    let (on, accent, splash, loading) = crate::config::with_config(|c| {
        (
            c.game_ui,
            c.game_ui_accent(),
            c.game_ui_splash_path.clone(),
            c.game_ui_loading_path.clone(),
        )
    });
    sync_at(
        &game,
        on,
        accent,
        &splash,
        &loading,
        mark_restart_if_game_on,
    );
}

pub(crate) fn sync_at(
    game: &Path,
    on: bool,
    accent: [u8; 3],
    splash: &str,
    loading: &str,
    mark_restart_if_game_on: bool,
) {
    let game_on = crate::startup::mx_bikes_pid().is_some();
    let force = NEED_RETRY.load(Ordering::Relaxed);
    let ui = game.join("ui");
    let will_write = if on {
        force || !pack_accent_current(&ui, accent)
    } else {
        ui.join(MANIFEST).is_file()
    };
    // Image path changes must rewrite TGAs even when accent is current.
    let images_dirty = on
        && (!ui.join(SPLASH_TGA).is_file()
            || !ui.join(LOADING_TGA).is_file()
            || force
            || will_write);
    let result = if on {
        if force {
            apply_paths(game, accent, splash, loading)
        } else if will_write {
            apply_with_opts(game, accent, splash, loading, false)
        } else {
            // Accent unchanged — still refresh splash / loading from config.
            write_screen_images(&ui, Some(&game.join(BAK_DIR)), splash, loading)
                .map(|_| {
                    // Ensure manifest lists the two screens so remove restores bak.
                    ensure_screen_manifest_entries(&ui, accent);
                })
        }
    } else {
        remove(game)
    };
    match result {
        Ok(()) => {
            NEED_RETRY.store(false, Ordering::Relaxed);
            if mark_restart_if_game_on && game_on && (will_write || images_dirty) {
                NEED_GAME_RESTART.store(true, Ordering::Relaxed);
            }
        }
        Err(_) => {
            NEED_RETRY.store(true, Ordering::Relaxed);
        }
    }
}

/// Direct pack apply (unit tests / `MXBO_GAME_UI_APPLY` live install helper).
#[cfg(test)]
pub fn apply(game: &Path, accent: [u8; 3]) -> Result<(), String> {
    let (splash, loading) = crate::config::with_config(|c| {
        (c.game_ui_splash_path.clone(), c.game_ui_loading_path.clone())
    });
    apply_paths(game, accent, &splash, &loading)
}

fn apply_paths(
    game: &Path,
    accent: [u8; 3],
    splash: &str,
    loading: &str,
) -> Result<(), String> {
    apply_pack_inner(
        &game.join("ui"),
        accent,
        Some(&game.join(BAK_DIR)),
        splash,
        loading,
        true,
    )
}

pub(crate) fn apply_with_opts(
    game: &Path,
    accent: [u8; 3],
    splash: &str,
    loading: &str,
    force: bool,
) -> Result<(), String> {
    apply_pack_inner(
        &game.join("ui"),
        accent,
        Some(&game.join(BAK_DIR)),
        splash,
        loading,
        force,
    )
}

pub fn remove(game: &Path) -> Result<(), String> {
    remove_pack(&game.join("ui"), Some(&game.join(BAK_DIR)))
}

pub(crate) fn apply_pack_inner(
    ui: &Path,
    accent: [u8; 3],
    bak: Option<&Path>,
    splash: &str,
    loading: &str,
    force: bool,
) -> Result<(), String> {
    if !force && pack_accent_current(ui, accent) {
        write_screen_images(ui, bak, splash, loading)?;
        ensure_screen_manifest_entries(ui, accent);
        return Ok(());
    }
    fs::create_dir_all(ui).map_err(|e| e.to_string())?;
    if let Some(bak) = bak {
        backup_if_needed(ui, bak)?;
    }
    let mut files = vec![
        MANIFEST.to_string(),
        "main.mnu".into(),
        "ui.ui".into(),
        "english.str".into(),
        SPLASH_TGA.into(),
        LOADING_TGA.into(),
    ];
    files.extend(SPRITES.iter().map(|s| (*s).to_string()));

    write_text(&ui.join("main.mnu"), &main_mnu(accent))?;
    write_text(&ui.join("ui.ui"), &ui_ui(accent))?;
    write_english_labels(ui, bak)?;
    ensure_stock_from_bak(ui, bak)?;
    write_screen_images(ui, bak, splash, loading)?;
    write_accent_pointer(ui, bak, accent)?;
    write_sprites(ui, accent)?;
    write_spin_arrows(ui, bak, accent)?;
    // Mutated stock chrome — track so remove deletes/restores them.
    for name in STOCK_CHROME {
        if ui.join(name).is_file() {
            files.push((*name).into());
        }
    }
    for splice in MENU_SPLICES {
        if (splice.run)(ui, bak, accent)? {
            files.push(splice.file.into());
        }
    }
    write_text(&ui.join(MANIFEST), &manifest_body(accent, &files))?;
    Ok(())
}

/// If an older pack is already current, append splash/loading to the manifest file list.
fn ensure_screen_manifest_entries(ui: &Path, accent: [u8; 3]) {
    let path = ui.join(MANIFEST);
    let Ok(text) = fs::read_to_string(&path) else {
        return;
    };
    let mut files: Vec<String> = text
        .lines()
        .filter(|l| {
            !l.starts_with('#')
                && !l.starts_with("accent=")
                && !l.starts_with("pack=")
                && !l.trim().is_empty()
        })
        .map(|l| l.trim().to_string())
        .collect();
    let mut dirty = false;
    for name in [SPLASH_TGA, LOADING_TGA] {
        if !files.iter().any(|f| f.eq_ignore_ascii_case(name)) {
            files.push(name.into());
            dirty = true;
        }
    }
    if dirty {
        let _ = write_text(&path, &manifest_body(accent, &files));
    }
}

pub(crate) struct MenuSplice {
    file: &'static str,
    run: fn(&Path, Option<&Path>, [u8; 3]) -> Result<bool, String>,
}

/// Pack `.mnu` splices — order matches prior apply_pack_inner if-chain.

/// Pack `.mnu` splices — order matches prior apply_pack_inner if-chain.
pub(crate) const MENU_SPLICES: &[MenuSplice] = &[
    MenuSplice {
        file: "multijoin.mnu",
        run: splice_multijoin,
    },
    MenuSplice {
        file: "options.mnu",
        run: splice_options,
    },
    MenuSplice {
        file: "connection.mnu",
        run: splice_connection,
    },
    MenuSplice {
        file: "testsetup.mnu",
        run: splice_testsetup,
    },
    MenuSplice {
        file: "profiles.mnu",
        run: splice_profiles,
    },
    MenuSplice {
        file: "viewreplays.mnu",
        run: splice_viewreplays,
    },
    MenuSplice {
        file: "replay.mnu",
        run: splice_replay,
    },
    MenuSplice {
        file: "multiclient.mnu",
        run: splice_multiclient,
    },
    MenuSplice {
        file: "test.mnu",
        run: splice_test,
    },
    MenuSplice {
        file: "garage.mnu",
        run: splice_garage,
    },
    MenuSplice {
        file: "hostsetup.mnu",
        run: splice_hostsetup,
    },
];

pub fn remove_pack(ui: &Path, bak: Option<&Path>) -> Result<(), String> {
    if !ui.join(MANIFEST).is_file() {
        return Ok(());
    }
    let listed = read_manifest_files(ui);
    for name in listed {
        let _ = fs::remove_file(ui.join(name));
    }
    if let Some(bak) = bak {
        // Gap-fill even when seed exists — pointer/arrows may be missing.
        if let Some(game) = ui.parent() {
            let _ = merge_pkz_into_bak(game, bak);
        }
        if bak.is_dir() {
            copy_dir(bak, ui)?;
        }
    }
    let _ = fs::remove_file(ui.join(MANIFEST));
    Ok(())
}

pub(crate) fn has_stock_seed(dir: &Path) -> bool {
    dir.join("english.str").is_file() && dir.join("main.fnt").is_file()
}

/// Copy any stock files missing from `ui/` (fonts, pointer, etc.) so the game
/// does not fall back to `ui.pkz` for chrome while using our `.mnu` colors.

/// Copy any stock files missing from `ui/` (fonts, pointer, etc.) so the game
/// does not fall back to `ui.pkz` for chrome while using our `.mnu` colors.
pub(crate) fn ensure_stock_from_bak(ui: &Path, bak: Option<&Path>) -> Result<(), String> {
    let Some(bak) = bak else {
        return Ok(());
    };
    if !bak.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(bak).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name();
        let Some(name_str) = name.to_str() else {
            continue;
        };
        if name_str.eq_ignore_ascii_case(MANIFEST) || name_str.eq_ignore_ascii_case(BAK_DIR) {
            continue;
        }
        let dest = ui.join(&name);
        if dest.exists() {
            continue;
        }
        let src = entry.path();
        if src.is_dir() {
            copy_dir(&src, &dest)?;
        } else if src.is_file() {
            fs::copy(&src, &dest).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Stock pointer is hard-blue. Retint to Look primary so chrome matches the menus.

pub(crate) fn backup_if_needed(ui: &Path, bak: &Path) -> Result<(), String> {
    // Keep a complete stock bak. If loose ui/ was wiped, fill bak from ui.pkz.
    // Seed alone is not enough — pointer/arrows may still be missing from bak.
    if has_stock_seed(bak) {
        if let Some(game) = ui.parent() {
            let _ = merge_pkz_into_bak(game, bak);
        }
        return Ok(());
    }
    if ui.join(MANIFEST).is_file() {
        if let Some(game) = ui.parent() {
            merge_pkz_into_bak(game, bak)?;
        }
        return Ok(());
    }
    if ui.is_dir() {
        let any = fs::read_dir(ui)
            .map(|rd| {
                rd.filter_map(|e| e.ok()).any(|e| {
                    e.file_name() != *std::ffi::OsStr::new(BAK_DIR)
                        && e.file_name() != *std::ffi::OsStr::new(MANIFEST)
                })
            })
            .unwrap_or(false);
        if any {
            if bak.exists() {
                let _ = fs::remove_dir_all(bak);
            }
            copy_dir(ui, bak)?;
        }
    }
    if !has_stock_seed(bak) {
        if let Some(game) = ui.parent() {
            merge_pkz_into_bak(game, bak)?;
        }
    }
    Ok(())
}

pub(crate) fn merge_pkz_into_bak(game: &Path, bak: &Path) -> Result<(), String> {
    let pkz = game.join("ui.pkz");
    if !pkz.is_file() {
        return Ok(());
    }
    fs::create_dir_all(bak).map_err(|e| e.to_string())?;
    let file = fs::File::open(&pkz).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().replace('\\', "/");
        let Some(rel) = name.strip_prefix("ui/") else {
            continue;
        };
        if rel.is_empty() || rel.ends_with('/') {
            continue;
        }
        let out = bak.join(rel);
        if out.is_file() {
            continue;
        }
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut dest = fs::File::create(&out).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut dest).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub(crate) fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(from).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name();
        if name == BAK_DIR || name == MANIFEST {
            continue;
        }
        let src = entry.path();
        let dst = to.join(name);
        if src.is_dir() {
            copy_dir(&src, &dst)?;
        } else {
            fs::copy(&src, &dst).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub(crate) fn read_manifest_files(ui: &Path) -> Vec<String> {
    let Ok(text) = fs::read_to_string(ui.join(MANIFEST)) else {
        return SPRITES
            .iter()
            .map(|s| (*s).to_string())
            .chain(["main.mnu".into(), "ui.ui".into(), MANIFEST.into()])
            .collect();
    };
    text.lines()
        .filter(|l| {
            !l.starts_with('#')
                && !l.starts_with("accent=")
                && !l.starts_with("pack=")
                && !l.trim().is_empty()
        })
        .map(|l| l.trim().to_string())
        .collect()
}

pub(crate) fn read_manifest_accent(ui: &Path) -> Option<[u8; 3]> {
    let text = fs::read_to_string(ui.join(MANIFEST)).ok()?;
    for line in text.lines() {
        if let Some(hex) = line.strip_prefix("accent=") {
            return crate::config::parse_primary_color(hex.trim());
        }
    }
    None
}

pub(crate) fn read_manifest_pack(ui: &Path) -> Option<u32> {
    let text = fs::read_to_string(ui.join(MANIFEST)).ok()?;
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("pack=") {
            return v.trim().parse().ok();
        }
    }
    None
}

pub(crate) fn pack_accent_current(ui: &Path, accent: [u8; 3]) -> bool {
    read_manifest_accent(ui) == Some(accent)
        && read_manifest_pack(ui) == Some(PACK_REV)
        && ui.join("main.mnu").is_file()
        && ui.join("main.fnt").is_file()
}

pub(crate) fn manifest_body(accent: [u8; 3], files: &[String]) -> String {
    let mut s = format!(
        "# holeshot-ui\naccent={}\npack={PACK_REV}\n",
        format_primary_color(accent)
    );
    for f in files {
        s.push_str(f);
        s.push('\n');
    }
    s
}
