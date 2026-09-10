#![allow(unused_imports)]
use super::*;

pub(crate) fn pane_app(
    px: &mut Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    open_drop: Option<Drop>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let mut y = heading(
        px,
        fonts,
        x,
        y,
        w,
        "Settings",
        "Font applies to every widget. Units are per measurement",
        None,
        hover,
        hits,
    );
    y = section(px, fonts, x, y, "Install");
    text(px, fonts, "Installed to", 10.0, x + 2.0, y + 2.0, dim(), false);
    y += 18.0;
    let install = fit_path(fonts, &crate::update::install_dir_display(), 12.0, w - 8.0);
    text(px, fonts, &install, 13.0, x + 4.0, y, text_col(), false);
    y += 22.0;
    text(px, fonts, "MX Bikes", 10.0, x + 2.0, y + 2.0, dim(), false);
    y += 18.0;
    let game = crate::plugin::game_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "Not found".into());
    let game = fit_path(fonts, &game, 12.0, w - 148.0);
    text(px, fonts, &game, 13.0, x + 4.0, y, text_col(), false);
    action_btn(px, fonts, x + w - 132.0, y - 6.0, 132.0, 28.0, "Change folder", Hit::GameFolder, hover, hits, false);
    y += 28.0;
    let plugin_line = if crate::plugin::needs_restart() {
        crate::plugin::RESTART_PLUGIN
    } else if crate::plugin::plugin_installed() {
        "Plugin is in the game plugins folder."
    } else {
        "Plugin is missing. Fully quit MX Bikes, then Change folder or restart the overlay."
    };
    let plugin_col = if crate::plugin::needs_restart() || !crate::plugin::plugin_installed() {
        accent()
    } else {
        muted()
    };
    text(px, fonts, plugin_line, 11.0, x + 4.0, y, plugin_col, false);
    y += 22.0;
    if crate::update::update_may_need_admin() {
        text(px, fonts, crate::update::ADMIN_UPDATE_HINT, 11.0, x + 4.0, y, accent(), false);
        y += 20.0;
    }
    y = section(px, fonts, x, y, "Look");
    y = dropdown_row(
        px,
        fonts,
        x,
        y,
        w,
        "Font",
        cfg.font_family.label(),
        open_drop == Some(Drop::FontFamily),
        Hit::FontOpen,
        &[
            (Hit::FontSegoe, "Segoe UI", cfg.font_family == FontFamily::Segoe),
            (Hit::FontArial, "Arial", cfg.font_family == FontFamily::Arial),
            (Hit::FontTahoma, "Tahoma", cfg.font_family == FontFamily::Tahoma),
            (Hit::FontRoboto, "Roboto", cfg.font_family == FontFamily::Roboto),
            (Hit::FontExo2, "Exo 2", cfg.font_family == FontFamily::Exo2),
            (Hit::FontTeko, "Teko", cfg.font_family == FontFamily::Teko),
            (Hit::FontGoldman, "Goldman", cfg.font_family == FontFamily::Goldman),
            (Hit::FontMontserrat, "Montserrat", cfg.font_family == FontFamily::Montserrat),
        ],
        hover,
        hits,
    );
    for kind in UnitKind::ALL {
        let selected = cfg.units.get(kind);
        y = dropdown_row(
            px,
            fonts,
            x,
            y,
            w,
            kind.label(),
            selected.label(),
            open_drop == Some(Drop::Units(kind)),
            Hit::UnitsOpen(kind),
            &[
                (Hit::UnitsPick(kind, Units::Metric), "Metric", selected == Units::Metric),
                (Hit::UnitsPick(kind, Units::Imperial), "Imperial", selected == Units::Imperial),
            ],
            hover,
            hits,
        );
    }
    y = dropdown_row(
        px,
        fonts,
        x,
        y,
        w,
        "Settings key",
        cfg.settings_key.label(),
        open_drop == Some(Drop::SettingsKey),
        Hit::SettingsKeyOpen,
        &SettingsKey::ALL.map(|key| (Hit::SettingsKeyPick(key), key.label(), cfg.settings_key == key)),
        hover,
        hits,
    );
    text(px, fonts, "Press again to close. Medal and other clip apps use F8. F9 still rotates the clock log.", 11.0, x + 4.0, y + 2.0, dim(), false);
    y += 22.0;
    y = section(px, fonts, x, y, "Startup");
    y = toggle_row(px, fonts, x, y, w, "Open when Windows starts", cfg.start_with_windows, Hit::StartWithWindows, hover, hits);
    if cfg.start_with_windows {
        text(
            px,
            fonts,
            "Starts in the tray at login. The tray icon or settings key opens settings.",
            11.0,
            x + 4.0,
            y + 2.0,
            dim(),
            false,
        );
        y += 22.0;
    }
    y = toggle_row(px, fonts, x, y, w, "Minimize on close", cfg.minimize_on_close, Hit::MinimizeOnClose, hover, hits);
    if cfg.minimize_on_close {
        text(
            px,
            fonts,
            &format!(
                "Close, Open when Windows starts, and Open when MX Bikes opens hide to the tray. {} or the tray icon brings settings back. Quit overlay exits.",
                cfg.settings_key.label()
            ),
            11.0,
            x + 4.0,
            y + 2.0,
            dim(),
            false,
        );
        y += 22.0;
    }
    y = toggle_row(px, fonts, x, y, w, "Close when MX Bikes closes", cfg.close_with_game, Hit::CloseWithGame, hover, hits);
    if cfg.close_with_game {
        text(
            px,
            fonts,
            if cfg.open_with_game {
                "Hides to the tray. Opening MX Bikes brings the HUD back."
            } else {
                "Quits the overlay a few seconds after the game exits."
            },
            11.0,
            x + 4.0,
            y + 2.0,
            dim(),
            false,
        );
        y += 22.0;
    }
    y = toggle_row(px, fonts, x, y, w, "Open when MX Bikes opens", cfg.open_with_game, Hit::OpenWithGame, hover, hits);
    if cfg.open_with_game {
        text(px, fonts, "Starts the overlay in the tray when MX Bikes launches, including after you close the game. F8 or the HUD mark opens settings.", 11.0, x + 4.0, y + 2.0, dim(), false);
        y += 22.0;
    }
    y = section(px, fonts, x, y, "Labs");
    y = toggle_row(px, fonts, x, y, w, "Experimental widgets", cfg.experimental, Hit::FeatureSector, hover, hits);
    text(px, fonts, "Adds Controller. Off until you turn this on.", 11.0, x + 4.0, y + 2.0, dim(), false);
    y += 22.0;
    y = section(px, fonts, x, y, "Updates");
    let need_admin = crate::update::update_may_need_admin();
    y = toggle_row(px, fonts, x, y, w, "Update automatically on launch", cfg.auto_update_on_launch, Hit::AutoUpdateOnLaunch, hover, hits);
    if cfg.auto_update_on_launch {
        text(px, fonts, "Checks GitHub before opening and installs if a newer version is out.", 11.0, x + 4.0, y + 2.0, dim(), false);
        y += 22.0;
    } else {
        text(px, fonts, "When a newer version is out, a banner at the top can install it.", 11.0, x + 4.0, y + 2.0, dim(), false);
        y += 22.0;
    }
    if need_admin {
        text(px, fonts, crate::update::ADMIN_UPDATE_HINT, 11.0, x + 4.0, y + 2.0, accent(), false);
        y += 22.0;
    }
    let update = crate::update::state();
    let (status, extra, show_check, show_install) = match &update {
        crate::update::UpdateState::Idle => ("Check GitHub for a newer build.", None, true, false),
        crate::update::UpdateState::Checking => ("Checking…", None, false, false),
        crate::update::UpdateState::Current => ("You already have the latest version.", None, true, false),
        crate::update::UpdateState::Available { version, .. } => (
            "A newer version is ready to install.",
            Some(format!("Version {version} is available.")),
            true,
            true,
        ),
        crate::update::UpdateState::Downloading => {
            ("Downloading and installing… the app will restart.", None, false, false)
        }
        crate::update::UpdateState::Failed(msg) => (msg.as_str(), None, true, false),
    };
    let mut card_h = if extra.is_some() { 168.0 } else { 148.0 };
    if need_admin && show_install {
        card_h += 18.0;
    }
    outlined(px, x, y, w, card_h, 10.0, panel());
    text(px, fonts, "Installed", 10.0, x + 16.0, y + 14.0, dim(), false);
    text(
        px,
        fonts,
        crate::update::current_version(),
        16.0,
        x + 16.0,
        y + 30.0,
        text_col(),
        false,
    );
    let mut iy = y + 58.0;
    if let Some(line) = extra.as_deref() {
        text(px, fonts, line, 13.0, x + 16.0, iy, accent(), false);
        iy += 22.0;
    }
    text(px, fonts, status, 12.0, x + 16.0, iy, muted(), false);
    iy += 26.0;
    if need_admin && show_install {
        text(px, fonts, crate::update::ADMIN_UPDATE_HINT, 11.0, x + 16.0, iy, accent(), false);
        iy += 20.0;
    }
    let btn_w = 156.0;
    let mut bx = x + 16.0;
    if show_check {
        action_btn(px, fonts, bx, iy, btn_w, 32.0, "Check for updates", Hit::UpdateCheck, hover, hits, false);
        bx += btn_w + 10.0;
    }
    if crate::changelog::modal_notes().is_some() {
        let label = if crate::changelog::previewing()
            && crate::changelog::next_notes().as_ref().map(|n| n.version.as_str())
                != crate::changelog::current_notes().as_ref().map(|n| n.version.as_str())
        {
            "Preview next"
        } else {
            "What's new"
        };
        action_btn(px, fonts, bx, iy, 132.0, 32.0, label, Hit::WhatsNewOpen, hover, hits, false);
        bx += 142.0;
    }
    if show_install {
        action_btn(
            px,
            fonts,
            bx,
            iy,
            168.0,
            32.0,
            "Download and install",
            Hit::UpdateInstall,
            hover,
            hits,
            true,
        );
    }
    y += card_h + 14.0;
    action_btn(px, fonts, x, y, 120.0, 32.0, "Quit overlay", Hit::QuitApp, hover, hits, false);
    action_btn(px, fonts, x + 132.0, y, 120.0, 32.0, "Uninstall", Hit::Uninstall, hover, hits, false);
    y += 46.0;
    text(
        px,
        fonts,
        "Uninstall removes the overlay, MX Bikes plugin, and shortcuts.",
        11.0,
        x,
        y,
        dim(),
        false,
    );
    y += 18.0;
    text(
        px,
        fonts,
        "The overlay installs the MX Bikes plugin when you start it. Restart the game only if it was already open.",
        11.0,
        x,
        y,
        dim(),
        false,
    );
    y + 28.0
}
