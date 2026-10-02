#![allow(unused_imports)]
use super::*;

fn open_default_browser(url: &str) {
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let wide: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let _ = ShellExecuteW(
            HWND::default(),
            w!("open"),
            PCWSTR(wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );
    }
}

fn use_named_board(
    board: String,
    art: String,
    text: TableText,
    places: Vec<mxbo_hud::pitboard::PitPlace>,
) {
    set_pit_name(&board);
    set_pit_notice("");
    update_config(|c| {
        c.pit.pit_art = art;
        c.pit.pit_board = board;
        c.pit.pit_text = text;
        c.pit.pit_sponsor.clear();
        c.pit.pit_vars = places;
        let art = c.pit.pit_art.clone();
        apply_json_names(&art, &mut c.pit.pit_vars);
        apply_pack_colors(&art, &mut c.pit.pit_vars);
        let _ = write_pack(&c.pit.pit_art, &c.pit.pit_vars, c.pit.pit_text);
    });
    set_pit_selected(0);
    invalidate_plate_preview();
}

fn show_factory_board() {
    set_pit_name("");
    set_pit_notice("");
    let (text, _, places) = factory_pack();
    update_config(|c| {
        c.pit.pit_art = FACTORY_ART.into();
        c.pit.pit_board.clear();
        c.pit.pit_text = text;
        c.pit.pit_sponsor.clear();
        c.pit.pit_yellow = c.primary;
        c.pit.pit_blue = [0, 0, 0];
        c.pit.pit_vars = places;
        let _ = write_pack(&c.pit.pit_art, &c.pit.pit_vars, c.pit.pit_text);
    });
    set_pit_selected(0);
    invalidate_plate_preview();
}

pub(crate) fn dispatch(id: Hit, p: (f32, f32)) {
    if !matches!(id, Hit::PitName) {
        if pit_name_focused() && !commit_pit_name() {
            return;
        }
        set_pit_name_focus(false);
    }
    match id {
        Hit::TabWidgets => {
            let last = UI
                .lock()
                .unwrap()
                .as_ref()
                .map(|u| u.last_widget)
                .unwrap_or(Tab::Standings);
            set_tab(last);
            return;
        }
        Hit::TabApp => {
            set_tab(Tab::App);
            return;
        }
        Hit::AppLook => {
            set_app_section(AppSection::Look);
            return;
        }
        Hit::AppMenus => {
            set_app_section(AppSection::Menus);
            return;
        }
        Hit::AppInstall => {
            set_app_section(AppSection::Install);
            return;
        }
        Hit::AppStartup => {
            set_app_section(AppSection::Startup);
            return;
        }
        Hit::AppStream => {
            set_app_section(AppSection::Stream);
            return;
        }
        Hit::AppLabs => {
            set_app_section(AppSection::Labs);
            return;
        }
        Hit::AppUpdates => {
            set_app_section(AppSection::Updates);
            return;
        }
        Hit::AppDiagnostics => {
            set_app_section(AppSection::Diagnostics);
            return;
        }
        Hit::DiagCopy => {
            let _ = crate::crash_dump::copy_latest();
            return;
        }
        Hit::TabProfile => {
            set_tab(Tab::Profile);
            return;
        }
        Hit::ProfileNavOverview => {
            set_profile_section(ProfileSection::Overview);
            return;
        }
        Hit::ProfileNavMotos => {
            set_profile_section(ProfileSection::Motos);
            open_live_analyze(true);
            return;
        }
        Hit::ProfileNavTracks => {
            if with_config(|c| c.experimental_unlocked()) {
                set_profile_section(ProfileSection::Tracks);
            }
            return;
        }
        Hit::TrackOpen(idx) => {
            let rows = crate::review::track_bank_rows();
            if let Some(row) = rows.get(idx as usize) {
                if let Some(ui) = UI.lock().unwrap().as_mut() {
                    ui.tracks_selected = Some(row.track.clone());
                    ui.scroll = 0.0;
                    ui.tracks_zoom = 1.0;
                    ui.tracks_pan_x = 0.0;
                    ui.tracks_pan_z = 0.0;
                    ui.tracks_map_drag = None;
                }
            }
            return;
        }
        Hit::TrackBack => {
            if let Some(ui) = UI.lock().unwrap().as_mut() {
                ui.tracks_selected = None;
                ui.scroll = 0.0;
                ui.tracks_zoom = 1.0;
                ui.tracks_pan_x = 0.0;
                ui.tracks_pan_z = 0.0;
                ui.tracks_map_drag = None;
            }
            return;
        }
        Hit::TabFeedback => {
            set_tab(Tab::Feedback);
            return;
        }
        Hit::ReviewFilterAll => {
            if let Some(ui) = UI.lock().unwrap().as_mut() {
                ui.review_filter = crate::review::ListFilter::All;
            }
            return;
        }
        Hit::ReviewFilterRanked => {
            if let Some(ui) = UI.lock().unwrap().as_mut() {
                ui.review_filter = crate::review::ListFilter::Ranked;
            }
            return;
        }
        Hit::ReviewFilterSaved => {
            if let Some(ui) = UI.lock().unwrap().as_mut() {
                ui.review_filter = crate::review::ListFilter::Saved;
            }
            return;
        }
        Hit::ReviewOpen(id) => {
            if let Some(ui) = UI.lock().unwrap().as_mut() {
                ui.motos_list_scroll = ui.scroll;
                ui.analyze_id = Some(id as i64);
                ui.analyze_compare = -1;
                ui.analyze_you_lap = -1;
                ui.analyze_warmup = false;
                ui.analyze_zoom = 1.0;
                ui.analyze_pan_x = 0.0;
                ui.analyze_pan_z = 0.0;
                ui.analyze_follow = false;
                ui.scroll = 0.0;
            }
            reset_analyze_scrub();
            return;
        }
        Hit::ReviewKeep(id) => {
            let cur = crate::review::list(crate::review::ListFilter::All)
                .into_iter()
                .find(|r| r.id == id as i64)
                .map(|r| r.kept)
                .unwrap_or(false);
            crate::review::set_kept(id as i64, !cur);
            return;
        }
        Hit::ReviewDelete(id) => {
            let was_live = crate::review::live_id() == Some(id as i64);
            crate::review::delete(id as i64);
            if let Some(ui) = UI.lock().unwrap().as_mut() {
                if ui.analyze_id == Some(id as i64) {
                    ui.analyze_id = None;
                    ui.scroll = ui.motos_list_scroll;
                }
                if was_live {
                    ui.review_stay_on_list = true;
                }
            }
            return;
        }
        Hit::ReviewBack => {
            if let Some(ui) = UI.lock().unwrap().as_mut() {
                ui.analyze_id = None;
                ui.scroll = ui.motos_list_scroll;
                ui.review_stay_on_list = true;
            }
            return;
        }
        Hit::ReviewToggle => {
            let mut now_on = false;
            update_config(|c| {
                c.review = !c.review;
                now_on = c.review;
            });
            if !now_on {
                crate::review::tick(&mxbo_hud::shm::Snapshot::default(), false);
            }
            return;
        }
        Hit::ProfileAllTime => {
            if let Some(ui) = UI.lock().unwrap().as_mut() {
                ui.profile_all_time = true;
            }
            return;
        }
        Hit::ProfileTwoWeeks => {
            if let Some(ui) = UI.lock().unwrap().as_mut() {
                ui.profile_all_time = false;
            }
            return;
        }
        Hit::ProfileClear => {
            if let Some(ui) = UI.lock().unwrap().as_mut() {
                ui.clear_confirm = Some(ClearKind::Profile);
            }
            return;
        }
        Hit::ReviewClear => {
            if let Some(ui) = UI.lock().unwrap().as_mut() {
                ui.clear_confirm = Some(ClearKind::Motos);
            }
            return;
        }
        Hit::ClearCancel | Hit::ClearScrim => {
            dismiss_clear_confirm();
            return;
        }
        Hit::ClearPanel => return,
        Hit::ClearConfirm => {
            let kind = UI.lock().unwrap().as_ref().and_then(|u| u.clear_confirm);
            match kind {
                Some(ClearKind::Profile) => crate::review::clear_profile(),
                Some(ClearKind::Motos) => {
                    crate::review::clear_motos();
                    if let Some(ui) = UI.lock().unwrap().as_mut() {
                        if let Some(id) = ui.analyze_id {
                            if crate::review::load(id).is_none() {
                                ui.analyze_id = None;
                                ui.review_stay_on_list = true;
                            }
                        }
                    }
                }
                None => {}
            }
            dismiss_clear_confirm();
            return;
        }
        Hit::ProfileAxis(_) => return,
        Hit::AnalyzeFollow => {
            if let Some(ui) = UI.lock().unwrap().as_mut() {
                ui.analyze_follow = !ui.analyze_follow;
            }
            return;
        }
        Hit::AnalyzeCompare(n) => {
            if let Some(ui) = UI.lock().unwrap().as_mut() {
                ui.analyze_compare = n;
                ui.open_drop = None;
            }
            reset_analyze_scrub();
            return;
        }
        Hit::AnalyzeCompareOpen => {
            toggle_drop(Drop::AnalyzeCompare);
            return;
        }
        Hit::AnalyzeYouLap(n) => {
            if let Some(ui) = UI.lock().unwrap().as_mut() {
                ui.analyze_you_lap = n;
                if let Some(id) = ui.analyze_id {
                    if let Some(d) = crate::review::load(id) {
                        ui.analyze_warmup = d
                            .laps
                            .iter()
                            .any(|l| l.lap_num == n && l.race_num == d.your_race_num && l.warmup);
                    }
                }
                ui.open_drop = None;
            }
            reset_analyze_scrub();
            return;
        }
        Hit::AnalyzeLapOpen => {
            toggle_drop(Drop::AnalyzeLap);
            return;
        }
        Hit::TabSt => {
            set_tab(Tab::Standings);
            return;
        }
        Hit::TabRel => {
            set_tab(Tab::Relative);
            return;
        }
        Hit::TabMap => {
            set_tab(Tab::Map);
            return;
        }
        Hit::TabMini => {
            set_tab(Tab::Minimap);
            return;
        }
        Hit::TabRadar => {
            set_tab(Tab::Radar);
            return;
        }
        Hit::TabDash => {
            set_tab(Tab::Dash);
            return;
        }
        Hit::TabTicker => {
            set_tab(Tab::Ticker);
            return;
        }
        Hit::TabSys => {
            set_tab(Tab::Sys);
            return;
        }
        Hit::TabSector => {
            set_tab(Tab::Sector);
            return;
        }
        Hit::TabDelta => {
            set_tab(Tab::Delta);
            return;
        }
        Hit::TabStance => {
            set_tab(Tab::Stance);
            return;
        }
        Hit::TabFlag => {
            set_tab(Tab::Flag);
            return;
        }
        Hit::TabLean => {
            set_tab(Tab::Lean);
            return;
        }
        Hit::TabGamepad => {
            set_tab(Tab::Gamepad);
            return;
        }
        Hit::TabTelemetry => {
            set_tab(Tab::Telemetry);
            return;
        }
        Hit::TabPitboard => {
            set_tab(Tab::Pitboard);
            update_config(|c| {
                normalize_places(&mut c.pit.pit_vars);
                let art = c.pit.pit_art.clone();
                apply_json_names(&art, &mut c.pit.pit_vars);
            });
            return;
        }
        Hit::PresetCopyOpen => {
            toggle_drop(Drop::PresetCopy);
            return;
        }
        Hit::MapDotOpen => {
            toggle_drop(Drop::MapDot);
            return;
        }
        Hit::MapYouTextOpen => {
            toggle_drop(Drop::MapYouText);
            return;
        }
        Hit::MiniDotOpen => {
            toggle_drop(Drop::MiniDot);
            return;
        }
        Hit::FontOpen => {
            toggle_drop(Drop::FontFamily);
            return;
        }
        Hit::PrimaryOpen => {
            toggle_drop(Drop::PrimaryColor);
            return;
        }
        Hit::PrimaryPanel | Hit::PrimarySaturation | Hit::PrimaryHue => return,
        Hit::PrimaryReset => {
            update_config(|c| c.primary = DEFAULT_PRIMARY);
            sync_menus_after_app_primary_change();
            return;
        }
        Hit::PrimarySwatch(i) => {
            if let Some(&rgb) = PRIMARY_SWATCHES.get(i as usize) {
                update_config(|c| c.primary = rgb);
                sync_menus_after_app_primary_change();
            }
            return;
        }
        Hit::GameUi => {
            close_drop();
            crate::config::update_config(|c| c.game_ui = !c.game_ui);
            crate::game_ui::sync_from_config();
            return;
        }
        Hit::GameUiMatchPrimary => {
            close_drop();
            crate::config::update_config(|c| {
                c.game_ui_match_primary = !c.game_ui_match_primary;
                if !c.game_ui_match_primary {
                    c.game_ui_primary = c.primary;
                }
            });
            crate::game_ui::sync_from_config();
            return;
        }
        Hit::GameUiPrimaryOpen => {
            toggle_drop(Drop::GameUiPrimaryColor);
            return;
        }
        Hit::GameUiPrimaryPanel | Hit::GameUiPrimarySaturation | Hit::GameUiPrimaryHue => return,
        Hit::GameUiPrimaryReset => {
            update_config(|c| c.game_ui_primary = DEFAULT_PRIMARY);
            sync_menus_after_menu_accent_change();
            return;
        }
        Hit::GameUiPrimarySwatch(i) => {
            if let Some(&rgb) = PRIMARY_SWATCHES.get(i as usize) {
                update_config(|c| c.game_ui_primary = rgb);
                sync_menus_after_menu_accent_change();
            }
            return;
        }
        Hit::PitYellowOpen => {
            toggle_drop(Drop::PitYellow);
            return;
        }
        Hit::PitYellowPanel | Hit::PitYellowSaturation | Hit::PitYellowHue => return,
        Hit::PitYellowReset => {
            update_config(|c| c.pit.pit_yellow = c.primary);
            return;
        }
        Hit::PitYellowSwatch(i) => {
            if let Some(&rgb) = PRIMARY_SWATCHES.get(i as usize) {
                update_config(|c| c.pit.pit_yellow = rgb);
            }
            return;
        }
        Hit::PitBlueOpen => {
            toggle_drop(Drop::PitBlue);
            return;
        }
        Hit::PitBluePanel | Hit::PitBlueSaturation | Hit::PitBlueHue => return,
        Hit::PitBlueReset => {
            update_config(|c| c.pit.pit_blue = [0, 0, 0]);
            return;
        }
        Hit::PitBlueSwatch(i) => {
            if let Some(&rgb) = PRIMARY_SWATCHES.get(i as usize) {
                update_config(|c| c.pit.pit_blue = rgb);
            }
            return;
        }
        Hit::GameUiSplashBrowse => {
            close_drop();
            let host = UI.lock().unwrap().as_ref().map(|u| u.host);
            let Some(host) = host else {
                return;
            };
            if let Some(path) = browse_image(host) {
                update_config(|c| c.game_ui_splash_path = path);
                crate::game_ui::sync_forced();
            }
            return;
        }
        Hit::GameUiSplashDefault => {
            close_drop();
            update_config(|c| c.game_ui_splash_path.clear());
            crate::game_ui::sync_forced();
            return;
        }
        Hit::GameUiLoadingBrowse => {
            close_drop();
            let host = UI.lock().unwrap().as_ref().map(|u| u.host);
            let Some(host) = host else {
                return;
            };
            if let Some(path) = browse_image(host) {
                update_config(|c| c.game_ui_loading_path = path);
                crate::game_ui::sync_forced();
            }
            return;
        }
        Hit::GameUiLoadingDefault => {
            close_drop();
            update_config(|c| c.game_ui_loading_path.clear());
            crate::game_ui::sync_forced();
            return;
        }
        Hit::UnitsOpen(kind) => {
            toggle_drop(Drop::Units(kind));
            return;
        }
        Hit::StTextOpen => {
            toggle_drop(Drop::StText);
            return;
        }
        Hit::RelTextOpen => {
            toggle_drop(Drop::RelText);
            return;
        }
        Hit::StPlaqueTextOpen => {
            toggle_drop(Drop::StPlaqueText);
            return;
        }
        Hit::RelPlaqueTextOpen => {
            toggle_drop(Drop::RelPlaqueText);
            return;
        }
        Hit::SettingsKeyOpen => {
            toggle_drop(Drop::SettingsKey);
            return;
        }
        Hit::ThemeOpen => {
            toggle_drop(Drop::Theme);
            return;
        }
        Hit::StanceBindOpen => {
            let was = UI.lock().unwrap().as_ref().is_some_and(|u| u.bind_listen);
            close_drop();
            if !was {
                let host = {
                    let mut ui = UI.lock().unwrap();
                    ui.as_mut().map(|u| {
                        u.bind_listen = true;
                        u.host
                    })
                };
                if let Some(host) = host {
                    announce(
                        host,
                        "Press a pad button, key, or mouse button now. Escape cancels.",
                    );
                }
            }
            return;
        }
        Hit::StanceModeOpen => {
            toggle_drop(Drop::StanceMode);
            return;
        }
        Hit::StanceStyleOpen => {
            toggle_drop(Drop::StanceStyle);
            return;
        }
        Hit::LeanStyleOpen => {
            toggle_drop(Drop::LeanStyle);
            return;
        }
        Hit::RadarStyleOpen => {
            toggle_drop(Drop::RadarStyle);
            return;
        }
        Hit::GamepadStyleOpen => {
            toggle_drop(Drop::GamepadStyle);
            return;
        }
        Hit::GamepadThemeOpen => {
            toggle_drop(Drop::GamepadTheme);
            return;
        }
        Hit::SysAddOpen => {
            toggle_drop(Drop::SysAdd);
            return;
        }
        Hit::StanceReset => {
            close_drop();
            crate::stance::reset_standing();
            return;
        }
        Hit::TrackPbClear => {
            close_drop();
            mxbo_hud::track_pb::clear_current();
            mxbo_hud::delta::reload_saved();
            mxbo_hud::sector::reload();
            return;
        }
        Hit::DashFootOpen(slot) => {
            toggle_drop(Drop::DashFoot(slot));
            return;
        }
        Hit::TickerFootOpen(slot) => {
            toggle_drop(Drop::TickerFoot(slot));
            return;
        }
        Hit::PitSlotOpen(slot) => {
            set_pit_selected(slot);
            toggle_drop(Drop::PitSlot(slot));
            return;
        }
        Hit::PitSlotSelect(slot) => {
            close_drop();
            set_pit_selected(slot);
            return;
        }
        Hit::PitWhenOpen => {
            toggle_drop(Drop::PitWhen);
            return;
        }
        Hit::PitSlotAdd => {
            close_drop();
            update_config(|c| {
                if add_design_slot(&mut c.pit.pit_vars) {
                    let _ = write_pack(&c.pit.pit_art, &c.pit.pit_vars, c.pit.pit_text);
                }
            });
            let n = with_config(|c| c.pit.pit_vars.len());
            set_pit_selected(n.saturating_sub(1) as u8);
            return;
        }
        Hit::PitSlotRemove => {
            close_drop();
            let index = pit_selected() as usize;
            update_config(|c| {
                if remove_design_slot(&mut c.pit.pit_vars, index) {
                    let _ = write_pack(&c.pit.pit_art, &c.pit.pit_vars, c.pit.pit_text);
                }
            });
            let n = with_config(|c| c.pit.pit_vars.len());
            set_pit_selected(pit_selected().min(n.saturating_sub(1) as u8));
            return;
        }
        Hit::PitSlotBold => {
            close_drop();
            update_config(|c| {
                if let Some(place) = c.pit.pit_vars.get_mut(pit_selected() as usize) {
                    place.bold = !place.bold;
                }
                let _ = write_pack(&c.pit.pit_art, &c.pit.pit_vars, c.pit.pit_text);
            });
            return;
        }
        Hit::PitSlotCenterX | Hit::PitSlotCenterY => {
            close_drop();
            let horizontal = matches!(id, Hit::PitSlotCenterX);
            update_config(|c| {
                if let Some(place) = c.pit.pit_vars.get_mut(pit_selected() as usize) {
                    if horizontal {
                        place.x = 0.5;
                    } else {
                        place.y = 0.5;
                    }
                    let _ = write_pack(&c.pit.pit_art, &c.pit.pit_vars, c.pit.pit_text);
                }
            });
            return;
        }
        Hit::PitSlotColorOpen => {
            toggle_drop(Drop::PitSlotColor);
            return;
        }
        Hit::PitSlotColorPanel | Hit::PitSlotColorSaturation | Hit::PitSlotColorHue => return,
        Hit::PitSlotColorReset => {
            update_config(|c| {
                if let Some(place) = c.pit.pit_vars.get_mut(pit_selected() as usize) {
                    place.color = None;
                }
                let _ = write_pack(&c.pit.pit_art, &c.pit.pit_vars, c.pit.pit_text);
            });
            return;
        }
        Hit::PitSlotColorSwatch(i) => {
            if let Some(&rgb) = PRIMARY_SWATCHES.get(i as usize) {
                update_config(|c| {
                    if let Some(place) = c.pit.pit_vars.get_mut(pit_selected() as usize) {
                        place.color = Some(rgb);
                    }
                    let _ = write_pack(&c.pit.pit_art, &c.pit.pit_vars, c.pit.pit_text);
                });
            }
            return;
        }
        Hit::InfoOpen(bar, slot) => {
            toggle_drop(Drop::Info(bar, slot));
            return;
        }
        Hit::UpdateCheck => {
            close_drop();
            crate::update::check();
            return;
        }
        Hit::UpdateInstall => {
            close_drop();
            crate::update::install();
            return;
        }
        Hit::UpdateBanner => {
            close_drop();
            return;
        }
        Hit::UpdateBannerDismiss => {
            close_drop();
            if let Some(ui) = UI.lock().unwrap().as_mut() {
                ui.banner_dismissed = true;
            }
            return;
        }
        Hit::WhatsNewOpen => {
            close_drop();
            open_whats_new();
            return;
        }
        Hit::WhatsNewDismiss => {
            close_drop();
            dismiss_whats_new();
            return;
        }
        Hit::WhatsNewScrim | Hit::WhatsNewPanel => {
            close_drop();
            return;
        }
        Hit::WhatsNewLink => {
            close_drop();
            if let Some(url) = whats_new_link_url() {
                open_default_browser(&url);
            }
            return;
        }
        Hit::ReplyDismiss => {
            close_drop();
            dismiss_reply();
            return;
        }
        Hit::ReplySend => {
            close_drop();
            crate::feedback::send_compose();
            return;
        }
        Hit::ReplyText => {
            close_drop();
            crate::feedback::click_compose(p.0, p.1);
            return;
        }
        Hit::ReplyScrim | Hit::ReplyPanel => {
            close_drop();
            return;
        }
        Hit::StartWithWindows => {
            close_drop();
            let on = crate::config::with_config(|c| !c.start_with_windows);
            crate::config::update_config(|c| c.start_with_windows = on);
            crate::startup::sync_from_config();
            return;
        }
        Hit::MinimizeOnClose => {
            close_drop();
            crate::config::update_config(|c| c.minimize_on_close = !c.minimize_on_close);
            return;
        }
        Hit::CloseWithGame => {
            close_drop();
            crate::config::update_config(|c| c.close_with_game = !c.close_with_game);
            return;
        }
        Hit::OpenWithGame => {
            close_drop();
            let on = crate::config::with_config(|c| !c.open_with_game);
            crate::config::update_config(|c| c.open_with_game = on);
            crate::startup::sync_from_config();
            if on {
                crate::startup::spawn_game_waiter();
            } else {
                // Turning off: stop leftover HUD waiters from an older build.
                crate::startup::kill_other_hud_processes();
            }
            return;
        }
        Hit::StreamEnabled => {
            close_drop();
            let on = crate::config::with_config(|c| !c.stream_enabled);
            let port = crate::stream::set_enabled(on);
            crate::config::update_config(|c| {
                c.stream_enabled = on;
                c.stream_port = port;
                if !on {
                    c.edit_surface = EditSurface::Game;
                }
            });
            return;
        }
        Hit::StreamCopyUrl => {
            close_drop();
            let url = crate::stream::url();
            let _ = crate::feedback::copy_text(&url);
            return;
        }
        Hit::StreamCopyEditUrl => {
            close_drop();
            let url = crate::stream::edit_url();
            let _ = crate::feedback::copy_text(&url);
            return;
        }
        Hit::StreamOpenUrl => {
            close_drop();
            open_default_browser(&crate::stream::url());
            return;
        }
        Hit::StreamOpenEditUrl => {
            close_drop();
            open_default_browser(&crate::stream::edit_url());
            return;
        }
        Hit::StreamCopyGameToStream => {
            close_drop();
            crate::config::update_config(|c| c.copy_game_edit_to_stream());
            return;
        }
        Hit::AutoUpdateOnLaunch => {
            close_drop();
            let on = crate::config::with_config(|c| !c.auto_update_on_launch);
            crate::config::update_config(|c| c.auto_update_on_launch = on);
            if !on {
                crate::update::check();
            }
            return;
        }
        Hit::QuitApp => {
            close_drop();
            if let Some(host) = UI.lock().unwrap().as_ref().map(|u| u.host) {
                crate::settings::persist_host_pos(host);
            }
            crate::config::update_config(|_| {});
            crate::quit_app();
            return;
        }
        Hit::Uninstall => {
            close_drop();
            let host = UI.lock().unwrap().as_ref().map(|u| u.host);
            let Some(host) = host else {
                return;
            };
            if crate::uninstall::confirm(host) && crate::uninstall::start(host) {
                crate::settings::persist_host_pos(host);
                crate::config::update_config(|_| {});
                crate::quit_app();
            }
            return;
        }
        Hit::GameFolder => {
            close_drop();
            let host = UI.lock().unwrap().as_ref().map(|u| u.host);
            let Some(host) = host else {
                return;
            };
            crate::plugin::pick_game_folder(host);
            return;
        }
        Hit::SysAppBrowse => {
            close_drop();
            let host = UI.lock().unwrap().as_ref().map(|u| u.host);
            let Some(host) = host else {
                return;
            };
            if let Some(exe) = browse_exe(host) {
                update_config(|c| c.add_sys_exe(&exe));
            }
            return;
        }
        Hit::PitBrowse => {
            close_drop();
            set_pit_name_focus(false);
            let host = UI.lock().unwrap().as_ref().map(|u| u.host);
            let Some(host) = host else {
                return;
            };
            if let Some(path) = browse_image(host) {
                let path = std::path::PathBuf::from(path);
                match board_name_from_picture(&path) {
                    Ok(board) => match mxbo_hud::pitboard::save_named_board(&board, &path) {
                        Ok((art, text, places)) => use_named_board(board, art, text, places),
                        Err(msg) => set_pit_notice(&msg),
                    },
                    Err(msg) => set_pit_notice(&msg),
                }
            }
            return;
        }
        Hit::PitExport => {
            close_drop();
            set_pit_name_focus(false);
            let (board, art, places, text) = with_config(|c| {
                (
                    c.pit.pit_board.clone(),
                    c.pit.pit_art.clone(),
                    c.pit.pit_vars.clone(),
                    c.pit.pit_text,
                )
            });
            if board.is_empty() {
                set_pit_notice("Save a named pit board before exporting.");
                return;
            }
            let _ = write_pack(&art, &places, text);
            let host = UI.lock().unwrap().as_ref().map(|u| u.host);
            let Some(host) = host else {
                return;
            };
            let Some(path) = pick_export_folder(host) else {
                return;
            };
            match mxbo_hud::pitboard::export_board(
                &board,
                &art,
                &places,
                text,
                std::path::Path::new(&path),
            ) {
                Ok(()) => set_pit_notice(""),
                Err(msg) => set_pit_notice(&msg),
            }
            return;
        }
        Hit::PitDemo => {
            close_drop();
            set_pit_name_focus(false);
            let host = UI.lock().unwrap().as_ref().map(|u| u.host);
            let Some(host) = host else {
                return;
            };
            let Some(path) = pick_demo_png(host) else {
                return;
            };
            match write_demo_png(&path) {
                Ok(name) => set_pit_notice(&format!("Saved {name}.")),
                Err(msg) => set_pit_notice(&msg),
            }
            return;
        }
        Hit::PitHelpOpen => {
            close_drop();
            open_pit_help();
            return;
        }
        Hit::PitHelpDismiss => {
            close_drop();
            dismiss_pit_help();
            return;
        }
        Hit::PitHelpScrim | Hit::PitHelpPanel => {
            close_drop();
            return;
        }
        Hit::PitImport => {
            close_drop();
            set_pit_name_focus(false);
            let host = UI.lock().unwrap().as_ref().map(|u| u.host);
            let Some(host) = host else {
                return;
            };
            if let Some(path) = browse_json(host) {
                let path = std::path::PathBuf::from(path);
                match board_name_from_pack(&path) {
                    Ok(board) => match mxbo_hud::pitboard::import_pack_json(&board, &path) {
                        Ok((art, text, places)) => use_named_board(board, art, text, places),
                        Err(msg) => set_pit_notice(&msg),
                    },
                    Err(msg) => set_pit_notice(&msg),
                }
            }
            return;
        }
        Hit::PitDelete => {
            close_drop();
            set_pit_name_focus(false);
            let board = with_config(|c| c.pit.pit_board.clone());
            if board.is_empty() {
                set_pit_notice("Holeshot cannot be deleted.");
                return;
            }
            let host = UI.lock().unwrap().as_ref().map(|u| u.host);
            let Some(host) = host else {
                return;
            };
            if !confirm_delete_board(host, &board) {
                return;
            }
            match mxbo_hud::pitboard::delete_saved_board(&board) {
                Ok(()) => show_factory_board(),
                Err(msg) => set_pit_notice(&msg),
            }
            return;
        }
        Hit::PitName => {
            close_drop();
            if pit_default_board() {
                set_pit_name_focus(false);
                return;
            }
            set_pit_name_focus(true);
            crate::feedback::set_focus(false);
            return;
        }
        Hit::PitBoardOpen => {
            set_pit_name_focus(false);
            toggle_drop(Drop::PitBoard);
            return;
        }
        Hit::PitBoardFactory => {
            close_drop();
            set_pit_name_focus(false);
            show_factory_board();
            return;
        }
        Hit::PitBoardPick(index) => {
            close_drop();
            set_pit_name_focus(false);
            let boards = mxbo_hud::pitboard::saved_boards();
            let Some(name) = boards.get(index as usize).cloned() else {
                return;
            };
            let Some((art, text, places)) = mxbo_hud::pitboard::open_saved_board(&name) else {
                set_pit_notice("That pit board could not be opened.");
                return;
            };
            set_pit_name(&name);
            set_pit_notice("");
            update_config(|c| {
                c.pit.pit_art = art;
                c.pit.pit_board = name;
                c.pit.pit_text = text;
                c.pit.pit_sponsor.clear();
                c.pit.pit_vars = places;
                let art = c.pit.pit_art.clone();
                apply_json_names(&art, &mut c.pit.pit_vars);
                apply_pack_colors(&art, &mut c.pit.pit_vars);
                let _ = write_pack(&c.pit.pit_art, &c.pit.pit_vars, c.pit.pit_text);
            });
            set_pit_selected(0);
            invalidate_plate_preview();
            return;
        }
        Hit::PitOpenFolder => {
            close_drop();
            let (art, places, text) = with_config(|c| {
                (c.pit.pit_art.clone(), c.pit.pit_vars.clone(), c.pit.pit_text)
            });
            let _ = write_pack(&art, &places, text);
            let dir = pitboards_dir();
            let _ = std::process::Command::new("explorer").arg(dir).spawn();
            return;
        }
        Hit::FbRate => {
            close_drop();
            crate::feedback::set_kind(crate::feedback::Kind::Rate);
            return;
        }
        Hit::FbBug => {
            close_drop();
            crate::feedback::set_kind(crate::feedback::Kind::Bug);
            return;
        }
        Hit::FbFeature => {
            close_drop();
            crate::feedback::set_kind(crate::feedback::Kind::Feature);
            return;
        }
        Hit::FbStar(n) => {
            close_drop();
            crate::feedback::set_rating(n);
            return;
        }
        Hit::FbText => {
            close_drop();
            crate::feedback::click_text(p.0, p.1);
            return;
        }
        Hit::FbAttach => {
            close_drop();
            crate::feedback::toggle_attach();
            return;
        }
        Hit::FbSend => {
            close_drop();
            crate::feedback::send();
            return;
        }
        _ => close_drop(),
    }
    update_config(|c| match id {
        Hit::StShow => c[WidgetId::Standings].show ^= true,
        Hit::RelShow => c[WidgetId::Relative].show ^= true,
        Hit::MapShow => c[WidgetId::Map].show ^= true,
        Hit::MiniShow => c[WidgetId::Minimap].show ^= true,
        Hit::RadarShow => c[WidgetId::Radar].show ^= true,
        Hit::DashShow => c[WidgetId::Dash].show ^= true,
        Hit::DashRev => c.dash.dash_rev = !c.dash.dash_rev,
        Hit::DashYellow => c.dash.dash_yellow = !c.dash.dash_yellow,
        Hit::DashBlue => c.dash.dash_blue = !c.dash.dash_blue,
        Hit::DashRed => c.dash.dash_red = !c.dash.dash_red,
        Hit::DashSimple => c.dash.dash_simple = !c.dash.dash_simple,
        Hit::DashShiftColor => c.dash.dash_shift_color = !c.dash.dash_shift_color,
        Hit::TickerShow => c[WidgetId::Ticker].show ^= true,
        Hit::SysShow => c[WidgetId::Sys].show ^= true,
        Hit::SysAppShow(i) => c.toggle_sys_app(i as usize),
        Hit::SysAppRemove(i) => c.remove_sys_app(i as usize),
        Hit::SysAddPick(i) => {
            if let Some(p) = SYS_PRESETS.get(i as usize) {
                c.add_sys_preset(p.key);
            }
        }
        Hit::SectorShow => c[WidgetId::Sector].show ^= true,
        Hit::SectorLive => c.sector.sector_live = !c.sector.sector_live,
        Hit::SectorSession => c.sector.sector_session = !c.sector.sector_session,
        Hit::SectorHist => c.sector.sector_hist = !c.sector.sector_hist,
        Hit::DeltaShow => c[WidgetId::Delta].show ^= true,
        Hit::DeltaSession => c.delta.delta_session = !c.delta.delta_session,
        Hit::StanceShow => c[WidgetId::Stance].show ^= true,
        Hit::FlagShow => c[WidgetId::Flag].show ^= true,
        Hit::LeanShow => c[WidgetId::Lean].show ^= true,
        Hit::TelemetryShow => c[WidgetId::Telemetry].show ^= true,
        Hit::PitShow => c[WidgetId::Pitboard].show ^= true,
        Hit::PitReset => {
            let (text, _, places) = factory_pack();
            c.pit.pit_art = FACTORY_ART.into();
            c.pit.pit_board.clear();
            c.pit.pit_text = text;
            c.pit.pit_sponsor.clear();
            c.pit.pit_vars = places;
            c.pit.pit_yellow = c.primary;
            c.pit.pit_blue = [0, 0, 0];
            let _ = write_pack(&c.pit.pit_art, &c.pit.pit_vars, c.pit.pit_text);
            set_pit_selected(0);
            set_pit_name("");
            set_pit_notice("");
            invalidate_plate_preview();
        }
        Hit::PitWhenAlways => c.pit.pit_when = PitWhen::Always,
        Hit::PitWhenSector => c.pit.pit_when = PitWhen::Sector,
        Hit::PitWhenLap => c.pit.pit_when = PitWhen::Lap,
        Hit::PitSlotPick(slot, i) => {
            if let Some(var) = PitVar::from_idx(i) {
                mxbo_hud::pitboard::set_slot_var(&mut c.pit.pit_vars, slot as usize, Some(var));
            }
            let _ = write_pack(&c.pit.pit_art, &c.pit.pit_vars, c.pit.pit_text);
        }
        Hit::PitSlotNone(slot) => {
            mxbo_hud::pitboard::set_slot_var(&mut c.pit.pit_vars, slot as usize, None);
            let _ = write_pack(&c.pit.pit_art, &c.pit.pit_vars, c.pit.pit_text);
        }
        Hit::TelemetryTraces => c.telemetry.telemetry_traces = !c.telemetry.telemetry_traces,
        Hit::TelemetryTraceThrottle => c.telemetry.telemetry_trace_throttle = !c.telemetry.telemetry_trace_throttle,
        Hit::TelemetryTraceBrake => c.telemetry.telemetry_trace_brake = !c.telemetry.telemetry_trace_brake,
        Hit::TelemetryTraceSteer => c.telemetry.telemetry_trace_steer = !c.telemetry.telemetry_trace_steer,
        Hit::TelemetryBars => c.telemetry.telemetry_bars = !c.telemetry.telemetry_bars,
        Hit::TelemetryBarClutch => c.telemetry.telemetry_bar_clutch = !c.telemetry.telemetry_bar_clutch,
        Hit::TelemetryBarBrake => c.telemetry.telemetry_bar_brake = !c.telemetry.telemetry_bar_brake,
        Hit::TelemetryBarThrottle => c.telemetry.telemetry_bar_throttle = !c.telemetry.telemetry_bar_throttle,
        Hit::TelemetryBarSteer => c.telemetry.telemetry_bar_steer = !c.telemetry.telemetry_bar_steer,
        Hit::TelemetryDial => c.telemetry.telemetry_dial = !c.telemetry.telemetry_dial,
        Hit::GamepadShow => c[WidgetId::Gamepad].show ^= true,
        Hit::FeatureSector => {
            c.experimental = !c.experimental;
        }
        Hit::FlagYellow => c.flag.flag_yellow = !c.flag.flag_yellow,
        Hit::FlagBlue => c.flag.flag_blue = !c.flag.flag_blue,
        Hit::FlagRed => c.flag.flag_red = !c.flag.flag_red,
        Hit::FlagText => c.flag.flag_text = !c.flag.flag_text,
        Hit::StanceShowSit => c.stance.stance_show_sit = !c.stance.stance_show_sit,
        Hit::TickerTitle => c.ticker.ticker_title = !c.ticker.ticker_title,
        Hit::TickerAutoscroll => c.ticker.ticker_autoscroll = !c.ticker.ticker_autoscroll,
        Hit::TickerStatus => c.ticker.ticker_status = !c.ticker.ticker_status,
        Hit::TickerSlide => c.ticker.ticker_slide = !c.ticker.ticker_slide,
        Hit::StStripe => c.standings.st_stripe = !c.standings.st_stripe,
        Hit::RelStripe => c.relative.rel_stripe = !c.relative.rel_stripe,
        Hit::StPlaque => c.standings.st_plaque = !c.standings.st_plaque,
        Hit::RelPlaque => c.relative.rel_plaque = !c.relative.rel_plaque,
        Hit::StPos => c.standings.st_pos = !c.standings.st_pos,
        Hit::StNum => c.standings.st_num = !c.standings.st_num,
        Hit::StName => c.standings.st_name = !c.standings.st_name,
        Hit::StGap => c.standings.st_gap = !c.standings.st_gap,
        Hit::StLaps => c.standings.st_laps = !c.standings.st_laps,
        Hit::StCurrent => c.standings.st_current = !c.standings.st_current,
        Hit::StBest => c.standings.st_best = !c.standings.st_best,
        Hit::StLast => c.standings.st_last = !c.standings.st_last,
        Hit::StStatus => c.standings.st_status = !c.standings.st_status,
        Hit::StBike => c.standings.st_bike = !c.standings.st_bike,
        Hit::StPenalty => c.standings.st_penalty = !c.standings.st_penalty,
        Hit::StCrashed => c.standings.st_crashed = !c.standings.st_crashed,
        Hit::StInterval => c.standings.st_interval = !c.standings.st_interval,
        Hit::StCategory => c.standings.st_category = !c.standings.st_category,
        Hit::StLapDiff => c.standings.st_lapdiff = !c.standings.st_lapdiff,
        Hit::RelNum => c.relative.rel_num = !c.relative.rel_num,
        Hit::RelName => c.relative.rel_name = !c.relative.rel_name,
        Hit::RelGap => c.relative.rel_gap = !c.relative.rel_gap,
        Hit::RelLaps => c.relative.rel_laps = !c.relative.rel_laps,
        Hit::RelCurrent => c.relative.rel_current = !c.relative.rel_current,
        Hit::RelPos => c.relative.rel_pos = !c.relative.rel_pos,
        Hit::RelBike => c.relative.rel_bike = !c.relative.rel_bike,
        Hit::RelPenalty => c.relative.rel_penalty = !c.relative.rel_penalty,
        Hit::RelInterval => c.relative.rel_interval = !c.relative.rel_interval,
        Hit::RelStatus => c.relative.rel_status = !c.relative.rel_status,
        Hit::RelBest => c.relative.rel_best = !c.relative.rel_best,
        Hit::RelLast => c.relative.rel_last = !c.relative.rel_last,
        Hit::RelCategory => c.relative.rel_category = !c.relative.rel_category,
        Hit::RelSpeed => c.relative.rel_speed = !c.relative.rel_speed,
        Hit::RelLapDiff => c.relative.rel_lapdiff = !c.relative.rel_lapdiff,
        Hit::MapOthers => c.map.map_others = !c.map.map_others,
        Hit::MapSf => c.map.map_sf = !c.map.map_sf,
        Hit::MapSectors => c.map.map_sectors = !c.map.map_sectors,
        Hit::MapArrows => c.map.map_arrows = !c.map.map_arrows,
        Hit::MapFollow => c.map.map_follow = !c.map.map_follow,
        Hit::MapCrown => c.map.map_crown = !c.map.map_crown,
        Hit::MapPlace => c.map.map_place = !c.map.map_place,
        Hit::MapNumbers => c.map.map_numbers = !c.map.map_numbers,
        Hit::MapDotNum => c.map.map_dot = DotLabel::Number,
        Hit::MapDotPos => c.map.map_dot = DotLabel::Position,
        Hit::MapYouTextWhite => c.map.map_you_text = TableText::White,
        Hit::MapYouTextBlack => c.map.map_you_text = TableText::Black,
        Hit::MiniOthers => c.mini.mini_others = !c.mini.mini_others,
        Hit::MiniSf => c.mini.mini_sf = !c.mini.mini_sf,
        Hit::MiniSectors => c.mini.mini_sectors = !c.mini.mini_sectors,
        Hit::MiniArrows => c.mini.mini_arrows = !c.mini.mini_arrows,
        Hit::MiniCrown => c.mini.mini_crown = !c.mini.mini_crown,
        Hit::MiniPlace => c.mini.mini_place = !c.mini.mini_place,
        Hit::MiniNumbers => c.mini.mini_numbers = !c.mini.mini_numbers,
        Hit::MiniDotNum => c.mini.mini_dot = DotLabel::Number,
        Hit::MiniDotPos => c.mini.mini_dot = DotLabel::Position,
        Hit::RadarSides => c.radar.radar_sides = !c.radar.radar_sides,
        Hit::RadarRear => c.radar.radar_rear = !c.radar.radar_rear,
        Hit::RadarRings => c.radar.radar_rings = !c.radar.radar_rings,
        Hit::Bold(id) => {
            let on = !c.bold(id);
            c.set_bold(id, on);
        }
        Hit::Snap(id, align) => c.snap(id, align),
        Hit::FontSegoe => c.font_family = FontFamily::Segoe,
        Hit::FontArial => c.font_family = FontFamily::Arial,
        Hit::FontTahoma => c.font_family = FontFamily::Tahoma,
        Hit::FontRoboto => c.font_family = FontFamily::Roboto,
        Hit::FontExo2 => c.font_family = FontFamily::Exo2,
        Hit::FontTeko => c.font_family = FontFamily::Teko,
        Hit::FontGoldman => c.font_family = FontFamily::Goldman,
        Hit::FontMontserrat => c.font_family = FontFamily::Montserrat,
        Hit::UnitsPick(kind, units) => c.units.set(kind, units),
        Hit::StTextWhite => c.standings.st_text = TableText::White,
        Hit::StTextBlack => c.standings.st_text = TableText::Black,
        Hit::RelTextWhite => c.relative.rel_text = TableText::White,
        Hit::RelTextBlack => c.relative.rel_text = TableText::Black,
        Hit::StPlaqueTextWhite => c.standings.st_plaque_text = TableText::White,
        Hit::StPlaqueTextBlack => c.standings.st_plaque_text = TableText::Black,
        Hit::RelPlaqueTextWhite => c.relative.rel_plaque_text = TableText::White,
        Hit::RelPlaqueTextBlack => c.relative.rel_plaque_text = TableText::Black,
        Hit::SettingsKeyPick(key) => c.settings_key = key,
        Hit::ThemePick(theme) => c.settings_theme = theme,
        Hit::StanceModePick(mode) => c.stance.stance_mode = mode,
        Hit::StanceStylePick(style) => c.stance.stance_style = style,
        Hit::LeanStylePick(style) => c.lean.lean_style = style,
        Hit::RadarStylePick(style) => {
            c.radar.radar_style = style;
            mxbo_hud::config::maybe_expand_radar_for_arrows(c);
        }
        Hit::GamepadStylePick(style) => c.gamepad.gamepad_style = style,
        Hit::GamepadThemePick(theme) => c.gamepad.gamepad_theme = theme,
        Hit::DashFootPick(slot, field) => match slot {
            0 => c.dash.dash_left = field,
            1 => c.dash.dash_mid = field,
            2 => c.dash.dash_right = field,
            _ => {}
        },
        Hit::TickerFootPick(slot, field) => match slot {
            0 => c.ticker.ticker_left = field,
            1 => c.ticker.ticker_right = field,
            _ => {}
        },
        Hit::InfoPick(bar, slot, field) => set_info_slot(c, bar, slot, field),
        Hit::Preset(p) => c.settings_preset = p,
        Hit::PresetCopyTo(p) => {
            c.copy_settings_to(p);
            c.settings_preset = p;
        }
        Hit::PresetCopyAll => c.copy_settings_to_all(),
        Hit::StDec => c.standings.standings_rows = (c.standings.standings_rows - 1).max(3),
        Hit::StInc => c.standings.standings_rows = (c.standings.standings_rows + 1).min(40),
        Hit::RelDec => c.relative.relative_count = (c.relative.relative_count - 1).max(1),
        Hit::RelInc => c.relative.relative_count = (c.relative.relative_count + 1).min(8),
        Hit::TickerDec => c.ticker.ticker_count = (c.ticker.ticker_count - 1).max(3),
        Hit::TickerInc => c.ticker.ticker_count = (c.ticker.ticker_count + 1).min(15),
        Hit::SectorHistDec => c.sector.sector_hist_laps = (c.sector.sector_hist_laps - 1).max(1),
        Hit::SectorHistInc => c.sector.sector_hist_laps = (c.sector.sector_hist_laps + 1).min(5),
        Hit::TabWidgets
        | Hit::TabApp
        | Hit::AppLook
        | Hit::AppMenus
        | Hit::AppInstall
        | Hit::AppStartup
        | Hit::AppStream
        | Hit::AppLabs
        | Hit::AppUpdates
        | Hit::AppDiagnostics
        | Hit::DiagCopy
        | Hit::TabProfile
        | Hit::ProfileNavOverview
        | Hit::ProfileNavMotos
        | Hit::ProfileNavTracks
        | Hit::TrackOpen(_)
        | Hit::TrackBack
        | Hit::TrackMap
        | Hit::TabFeedback
        | Hit::ProfileAllTime
        | Hit::ProfileTwoWeeks
        | Hit::ProfileClear
        | Hit::ProfileAxis(_)
        | Hit::ReviewClear
        | Hit::ClearScrim
        | Hit::ClearPanel
        | Hit::ClearCancel
        | Hit::ClearConfirm
        | Hit::TabSt
        | Hit::TabRel
        | Hit::TabMap
        | Hit::TabMini
        | Hit::TabRadar
        | Hit::TabDash
        | Hit::TabTicker
        | Hit::TabSys
        | Hit::TabSector
        | Hit::TabDelta
        | Hit::TabStance
        | Hit::TabFlag
        | Hit::TabLean
        | Hit::TabGamepad
        | Hit::TabTelemetry
        | Hit::TabPitboard
        | Hit::MapDotOpen
        | Hit::MapYouTextOpen
        | Hit::MiniDotOpen
        | Hit::FontOpen
        | Hit::PrimaryOpen
        | Hit::PrimaryPanel
        | Hit::PrimarySaturation
        | Hit::PrimaryHue
        | Hit::PrimarySwatch(_)
        | Hit::PrimaryReset
        | Hit::GameUiMatchPrimary
        | Hit::GameUiPrimaryOpen
        | Hit::GameUiPrimaryPanel
        | Hit::GameUiPrimarySaturation
        | Hit::GameUiPrimaryHue
        | Hit::GameUiPrimarySwatch(_)
        | Hit::GameUiPrimaryReset
        | Hit::PitYellowOpen
        | Hit::PitYellowPanel
        | Hit::PitYellowSaturation
        | Hit::PitYellowHue
        | Hit::PitYellowSwatch(_)
        | Hit::PitYellowReset
        | Hit::PitBlueOpen
        | Hit::PitBluePanel
        | Hit::PitBlueSaturation
        | Hit::PitBlueHue
        | Hit::PitBlueSwatch(_)
        | Hit::PitBlueReset
        | Hit::GameUiSplashBrowse
        | Hit::GameUiSplashDefault
        | Hit::GameUiLoadingBrowse
        | Hit::GameUiLoadingDefault
        | Hit::UnitsOpen(_)
        | Hit::StTextOpen
        | Hit::RelTextOpen
        | Hit::StPlaqueTextOpen
        | Hit::RelPlaqueTextOpen
        | Hit::SettingsKeyOpen
        | Hit::ThemeOpen
        | Hit::StanceBindOpen
        | Hit::StanceModeOpen
        | Hit::StanceStyleOpen
        | Hit::LeanStyleOpen
        | Hit::RadarStyleOpen
        | Hit::GamepadStyleOpen
        | Hit::GamepadThemeOpen
        | Hit::SysAddOpen
        | Hit::DashFootOpen(_)
        | Hit::TickerFootOpen(_)
        | Hit::InfoOpen(_, _)
        | Hit::PitSlotOpen(_)
        | Hit::PitSlotSelect(_)
        | Hit::PitSlotGrab(_)
        | Hit::PitSlotAdd
        | Hit::PitSlotRemove
        | Hit::PitSlotSize
        | Hit::PitSlotBold
        | Hit::PitSlotCenterX
        | Hit::PitSlotCenterY
        | Hit::PitSlotColorOpen
        | Hit::PitSlotColorPanel
        | Hit::PitSlotColorSaturation
        | Hit::PitSlotColorHue
        | Hit::PitSlotColorSwatch(_)
        | Hit::PitSlotColorReset
        | Hit::PitWhenOpen
        | Hit::PresetCopyOpen
        | Hit::UpdateCheck
        | Hit::UpdateInstall
        | Hit::UpdateBanner
        | Hit::UpdateBannerDismiss
        | Hit::WhatsNewOpen
        | Hit::WhatsNewDismiss
        | Hit::WhatsNewScrim
        | Hit::WhatsNewPanel
        | Hit::WhatsNewLink
        | Hit::ReplyDismiss
        | Hit::ReplySend
        | Hit::ReplyText
        | Hit::ReplyScrim
        | Hit::ReplyPanel
        | Hit::StartWithWindows
        | Hit::GameUi
        | Hit::MinimizeOnClose
        | Hit::CloseWithGame
        | Hit::OpenWithGame
        | Hit::StreamEnabled
        | Hit::StreamCopyUrl
        | Hit::StreamCopyEditUrl
        | Hit::StreamOpenUrl
        | Hit::StreamOpenEditUrl
        | Hit::StreamCopyGameToStream
        | Hit::AutoUpdateOnLaunch
        | Hit::QuitApp
        | Hit::Uninstall
        | Hit::GameFolder
        | Hit::SysAppBrowse
        | Hit::FbRate
        | Hit::FbBug
        | Hit::FbFeature
        | Hit::FbStar(_)
        | Hit::FbText
        | Hit::FbAttach
        | Hit::FbSend
        | Hit::StDrag(_)
        | Hit::RelDrag(_)
        | Hit::StBg
        | Hit::StHl
        | Hit::RelBg
        | Hit::RelHl
        | Hit::MapBg
        | Hit::MapZoom
        | Hit::MapDotOpacity
        | Hit::MiniBg
        | Hit::MiniZoom
        | Hit::RadarRange
        | Hit::RadarBg
        | Hit::DashBg
        | Hit::TickerBg
        | Hit::TickerHl
        | Hit::SysBg
        | Hit::SectorBg
        | Hit::DeltaBg
        | Hit::StanceBg
        | Hit::FlagBg
        | Hit::LeanBg
        | Hit::GamepadBg
        | Hit::TelemetryBg
        | Hit::PitBg
        | Hit::PitBrowse
        | Hit::PitName
        | Hit::PitBoardOpen
        | Hit::PitBoardFactory
        | Hit::PitBoardPick(_)
        | Hit::PitExport
        | Hit::PitImport
        | Hit::PitDemo
        | Hit::PitHelpOpen
        | Hit::PitHelpDismiss
        | Hit::PitHelpScrim
        | Hit::PitHelpPanel
        | Hit::PitDelete
        | Hit::PitOpenFolder
        | Hit::StW(_)
        | Hit::RelW(_)
        | Hit::Font(_)
        | Hit::StanceReset
        | Hit::TrackPbClear
        | Hit::ReviewFilterAll
        | Hit::ReviewFilterRanked
        | Hit::ReviewFilterSaved
        | Hit::ReviewOpen(_)
        | Hit::ReviewKeep(_)
        | Hit::ReviewDelete(_)
        | Hit::ReviewBack
        | Hit::ReviewToggle
        | Hit::AnalyzeCompare(_)
        | Hit::AnalyzeCompareOpen
        | Hit::AnalyzeYouLap(_)
        | Hit::AnalyzeLapOpen
        | Hit::AnalyzeScrub
        | Hit::AnalyzeMap
        | Hit::AnalyzeFollow => {}
    });
    if matches!(
        id,
        Hit::PitShow | Hit::PresetCopyTo(_) | Hit::PresetCopyAll
    ) {
        crate::stock_pitboard::sync_from_config();
    }
    if matches!(id, Hit::ThemePick(_)) {
        if let Some(host) = UI.lock().unwrap().as_ref().map(|u| u.host) {
            refresh_palette();
            sync_titlebar(host);
        }
    }
    if id == Hit::FeatureSector && !with_config(|c| c.experimental_unlocked()) {
        if let Some(ui) = UI.lock().unwrap().as_mut() {
            if ui.profile_section == ProfileSection::Tracks {
                ui.profile_section = ProfileSection::Overview;
                ui.tracks_selected = None;
            }
        }
    }
}

fn reset_analyze_scrub() {
    let (id, you_lap, warmup) = {
        let ui = UI.lock().unwrap();
        let Some(ui) = ui.as_ref() else {
            return;
        };
        (ui.analyze_id, ui.analyze_you_lap, ui.analyze_warmup)
    };
    let scrub = id
        .and_then(crate::review::load)
        .map(|d| default_analyze_scrub(&d, you_lap, warmup))
        .unwrap_or(0.0);
    if let Some(ui) = UI.lock().unwrap().as_mut() {
        ui.analyze_scrub = scrub;
    }
}

/// Jump Analyze to the live moto. `force` is F8 / Review tab; paint is not.
pub(crate) fn open_live_analyze(force: bool) {
    let live = crate::review::live_id();
    {
        let mut ui = UI.lock().unwrap();
        let Some(ui) = ui.as_mut() else {
            return;
        };
        if force {
            ui.review_stay_on_list = false;
        }
        let Some(id) = live_analyze_target(force, ui.review_stay_on_list, ui.analyze_id, live)
        else {
            return;
        };
        ui.analyze_id = Some(id);
        ui.analyze_compare = -1;
        ui.analyze_you_lap = -1;
        ui.analyze_warmup = false;
        ui.analyze_zoom = 1.0;
        ui.analyze_pan_x = 0.0;
        ui.analyze_pan_z = 0.0;
        ui.analyze_follow = false;
        ui.motos_list_scroll = ui.scroll;
        ui.scroll = 0.0;
    }
    reset_analyze_scrub();
}

pub(crate) fn live_analyze_target(
    force: bool,
    stay: bool,
    analyze_id: Option<i64>,
    live_id: Option<i64>,
) -> Option<i64> {
    let live = live_id?;
    if analyze_id == Some(live) {
        return None;
    }
    if force {
        return Some(live);
    }
    if stay || analyze_id.is_some() {
        return None;
    }
    Some(live)
}
