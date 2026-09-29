//! Hide MX Bikes' stock 2D pitboard while any preset shows our Pit Board widget.
//! An empty `misc/hud/pitboard.cfg` is the game's own override. `holeshot-pitboard`
//! beside it marks a file we created, so a foreign `pitboard.cfg` is left alone.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

const CFG_FILE: &str = "pitboard.cfg";
const MARKER_FILE: &str = "holeshot-pitboard";

pub const RESTART_STOCK: &str =
    "Fully quit MX Bikes and start it again so the stock pitboard updates.";

static NEED_RETRY: AtomicBool = AtomicBool::new(false);
static NEED_GAME_RESTART: AtomicBool = AtomicBool::new(false);

pub fn needs_restart() -> bool {
    NEED_GAME_RESTART.load(Ordering::Relaxed)
}

pub fn clear_game_restart() {
    NEED_GAME_RESTART.store(false, Ordering::Relaxed);
}

pub fn retry_if_needed() {
    if NEED_RETRY.load(Ordering::Relaxed) {
        sync_from_config();
    }
}

/// Apply or remove the stock-board override from the live config.
pub fn sync_from_config() {
    let Some(game) = crate::plugin::game_dir() else {
        return;
    };
    let want_hidden = crate::config::with_config(|cfg| cfg.pitboard_shown_anywhere());
    sync_at(&game, want_hidden, true);
}

/// Delete the override only when we created it.
pub fn remove(game: &Path) {
    let _ = release(game);
}

fn hud_dir(game: &Path) -> PathBuf {
    game.join("misc").join("hud")
}

fn sync_at(game: &Path, want_hidden: bool, mark_restart_if_game_on: bool) {
    let game_on = crate::startup::mx_bikes_pid().is_some();
    let result = if want_hidden { hide(game) } else { release(game) };
    match result {
        Ok(changed) => {
            NEED_RETRY.store(false, Ordering::Relaxed);
            if mark_restart_if_game_on && game_on && changed {
                NEED_GAME_RESTART.store(true, Ordering::Relaxed);
            }
        }
        Err(_) => {
            NEED_RETRY.store(true, Ordering::Relaxed);
        }
    }
}

/// Create the empty override and our marker. A `pitboard.cfg` we did not create stays.
fn hide(game: &Path) -> io::Result<bool> {
    let dir = hud_dir(game);
    let cfg_path = dir.join(CFG_FILE);
    if cfg_path.is_file() {
        return Ok(false);
    }
    fs::create_dir_all(&dir)?;
    fs::write(&cfg_path, b"")?;
    if let Err(err) = fs::write(dir.join(MARKER_FILE), b"") {
        let _ = fs::remove_file(&cfg_path);
        return Err(err);
    }
    Ok(true)
}

/// Remove the override only when the marker says we created it.
fn release(game: &Path) -> io::Result<bool> {
    let dir = hud_dir(game);
    let marker_path = dir.join(MARKER_FILE);
    if !marker_path.is_file() {
        return Ok(false);
    }
    let cfg_path = dir.join(CFG_FILE);
    if cfg_path.is_file() {
        fs::remove_file(&cfg_path)?;
    }
    fs::remove_file(&marker_path)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_game() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "mxbo-stock-pit-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn hide_writes_empty_cfg_and_marker() {
        let game = temp_game();
        assert_eq!(hide(&game).unwrap(), true);
        let cfg_path = hud_dir(&game).join(CFG_FILE);
        assert_eq!(fs::read(&cfg_path).unwrap(), b"");
        assert!(hud_dir(&game).join(MARKER_FILE).is_file());
        assert_eq!(hide(&game).unwrap(), false);
        let _ = fs::remove_dir_all(&game);
    }

    #[test]
    fn foreign_cfg_is_left_in_place() {
        let game = temp_game();
        let dir = hud_dir(&game);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(CFG_FILE), b"kept").unwrap();
        assert_eq!(hide(&game).unwrap(), false);
        assert_eq!(fs::read(dir.join(CFG_FILE)).unwrap(), b"kept");
        assert!(!dir.join(MARKER_FILE).is_file());
        assert_eq!(release(&game).unwrap(), false);
        assert_eq!(fs::read(dir.join(CFG_FILE)).unwrap(), b"kept");
        let _ = fs::remove_dir_all(&game);
    }

    #[test]
    fn release_deletes_only_our_pair() {
        let game = temp_game();
        assert_eq!(hide(&game).unwrap(), true);
        assert_eq!(release(&game).unwrap(), true);
        assert!(!hud_dir(&game).join(CFG_FILE).is_file());
        assert!(!hud_dir(&game).join(MARKER_FILE).is_file());
        assert_eq!(release(&game).unwrap(), false);
        let _ = fs::remove_dir_all(&game);
    }
}
