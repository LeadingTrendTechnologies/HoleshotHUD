#![allow(unused_imports)]
use super::*;

pub(crate) fn dispatch(id: Hit, p: (f32, f32)) {
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
        Hit::AppLabs => {
            set_app_section(AppSection::Labs);
            return;
        }
        Hit::AppUpdates => {
            set_app_section(AppSection::Updates);
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
        Hit::PresetCopyOpen => {
            toggle_drop(Drop::PresetCopy);
            return;
        }
        Hit::MapDotOpen => {
            toggle_drop(Drop::MapDot);
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
        Hit::PrimaryPanel | Hit::PrimarySv | Hit::PrimaryHue => return,
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
        Hit::GameUiPrimaryPanel | Hit::GameUiPrimarySv | Hit::GameUiPrimaryHue => return,
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
        Hit::DashRev => c.dash_rev = !c.dash_rev,
        Hit::DashYellow => c.dash_yellow = !c.dash_yellow,
        Hit::DashBlue => c.dash_blue = !c.dash_blue,
        Hit::DashRed => c.dash_red = !c.dash_red,
        Hit::DashSimple => c.dash_simple = !c.dash_simple,
        Hit::DashShiftColor => c.dash_shift_color = !c.dash_shift_color,
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
        Hit::SectorLive => c.sector_live = !c.sector_live,
        Hit::SectorSession => c.sector_session = !c.sector_session,
        Hit::SectorHist => c.sector_hist = !c.sector_hist,
        Hit::DeltaShow => c[WidgetId::Delta].show ^= true,
        Hit::DeltaSession => c.delta_session = !c.delta_session,
        Hit::StanceShow => c[WidgetId::Stance].show ^= true,
        Hit::FlagShow => c[WidgetId::Flag].show ^= true,
        Hit::LeanShow => c[WidgetId::Lean].show ^= true,
        Hit::TelemetryShow => c[WidgetId::Telemetry].show ^= true,
        Hit::TelemetryTraces => c.telemetry_traces = !c.telemetry_traces,
        Hit::TelemetryTraceThrottle => c.telemetry_trace_throttle = !c.telemetry_trace_throttle,
        Hit::TelemetryTraceBrake => c.telemetry_trace_brake = !c.telemetry_trace_brake,
        Hit::TelemetryTraceSteer => c.telemetry_trace_steer = !c.telemetry_trace_steer,
        Hit::TelemetryBars => c.telemetry_bars = !c.telemetry_bars,
        Hit::TelemetryBarClutch => c.telemetry_bar_clutch = !c.telemetry_bar_clutch,
        Hit::TelemetryBarBrake => c.telemetry_bar_brake = !c.telemetry_bar_brake,
        Hit::TelemetryBarThrottle => c.telemetry_bar_throttle = !c.telemetry_bar_throttle,
        Hit::TelemetryBarSteer => c.telemetry_bar_steer = !c.telemetry_bar_steer,
        Hit::TelemetryDial => c.telemetry_dial = !c.telemetry_dial,
        Hit::GamepadShow => c[WidgetId::Gamepad].show ^= true,
        Hit::FeatureSector => {
            c.experimental = !c.experimental;
        }
        Hit::FlagYellow => c.flag_yellow = !c.flag_yellow,
        Hit::FlagBlue => c.flag_blue = !c.flag_blue,
        Hit::FlagRed => c.flag_red = !c.flag_red,
        Hit::FlagText => c.flag_text = !c.flag_text,
        Hit::StanceShowSit => c.stance_show_sit = !c.stance_show_sit,
        Hit::TickerTitle => c.ticker_title = !c.ticker_title,
        Hit::TickerAutoscroll => c.ticker_autoscroll = !c.ticker_autoscroll,
        Hit::TickerStatus => c.ticker_status = !c.ticker_status,
        Hit::TickerSlide => c.ticker_slide = !c.ticker_slide,
        Hit::StStripe => c.st_stripe = !c.st_stripe,
        Hit::RelStripe => c.rel_stripe = !c.rel_stripe,
        Hit::StPlaque => c.st_plaque = !c.st_plaque,
        Hit::RelPlaque => c.rel_plaque = !c.rel_plaque,
        Hit::StPos => c.st_pos = !c.st_pos,
        Hit::StNum => c.st_num = !c.st_num,
        Hit::StName => c.st_name = !c.st_name,
        Hit::StGap => c.st_gap = !c.st_gap,
        Hit::StLaps => c.st_laps = !c.st_laps,
        Hit::StCurrent => c.st_current = !c.st_current,
        Hit::StBest => c.st_best = !c.st_best,
        Hit::StLast => c.st_last = !c.st_last,
        Hit::StStatus => c.st_status = !c.st_status,
        Hit::StBike => c.st_bike = !c.st_bike,
        Hit::StPenalty => c.st_penalty = !c.st_penalty,
        Hit::StCrashed => c.st_crashed = !c.st_crashed,
        Hit::StInterval => c.st_interval = !c.st_interval,
        Hit::StCategory => c.st_category = !c.st_category,
        Hit::RelNum => c.rel_num = !c.rel_num,
        Hit::RelName => c.rel_name = !c.rel_name,
        Hit::RelGap => c.rel_gap = !c.rel_gap,
        Hit::RelLaps => c.rel_laps = !c.rel_laps,
        Hit::RelCurrent => c.rel_current = !c.rel_current,
        Hit::RelPos => c.rel_pos = !c.rel_pos,
        Hit::RelBike => c.rel_bike = !c.rel_bike,
        Hit::RelPenalty => c.rel_penalty = !c.rel_penalty,
        Hit::RelInterval => c.rel_interval = !c.rel_interval,
        Hit::RelStatus => c.rel_status = !c.rel_status,
        Hit::RelBest => c.rel_best = !c.rel_best,
        Hit::RelLast => c.rel_last = !c.rel_last,
        Hit::RelCategory => c.rel_category = !c.rel_category,
        Hit::RelSpeed => c.rel_speed = !c.rel_speed,
        Hit::MapOthers => c.map_others = !c.map_others,
        Hit::MapSf => c.map_sf = !c.map_sf,
        Hit::MapSectors => c.map_sectors = !c.map_sectors,
        Hit::MapArrows => c.map_arrows = !c.map_arrows,
        Hit::MapCrown => c.map_crown = !c.map_crown,
        Hit::MapPlace => c.map_place = !c.map_place,
        Hit::MapNumbers => c.map_numbers = !c.map_numbers,
        Hit::MapDotNum => c.map_dot = DotLabel::Number,
        Hit::MapDotPos => c.map_dot = DotLabel::Position,
        Hit::MiniOthers => c.mini_others = !c.mini_others,
        Hit::MiniSf => c.mini_sf = !c.mini_sf,
        Hit::MiniSectors => c.mini_sectors = !c.mini_sectors,
        Hit::MiniArrows => c.mini_arrows = !c.mini_arrows,
        Hit::MiniCrown => c.mini_crown = !c.mini_crown,
        Hit::MiniPlace => c.mini_place = !c.mini_place,
        Hit::MiniNumbers => c.mini_numbers = !c.mini_numbers,
        Hit::MiniDotNum => c.mini_dot = DotLabel::Number,
        Hit::MiniDotPos => c.mini_dot = DotLabel::Position,
        Hit::RadarSides => c.radar_sides = !c.radar_sides,
        Hit::RadarRear => c.radar_rear = !c.radar_rear,
        Hit::RadarRings => c.radar_rings = !c.radar_rings,
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
        Hit::StTextWhite => c.st_text = TableText::White,
        Hit::StTextBlack => c.st_text = TableText::Black,
        Hit::RelTextWhite => c.rel_text = TableText::White,
        Hit::RelTextBlack => c.rel_text = TableText::Black,
        Hit::StPlaqueTextWhite => c.st_plaque_text = TableText::White,
        Hit::StPlaqueTextBlack => c.st_plaque_text = TableText::Black,
        Hit::RelPlaqueTextWhite => c.rel_plaque_text = TableText::White,
        Hit::RelPlaqueTextBlack => c.rel_plaque_text = TableText::Black,
        Hit::SettingsKeyPick(key) => c.settings_key = key,
        Hit::ThemePick(theme) => c.settings_theme = theme,
        Hit::StanceModePick(mode) => c.stance_mode = mode,
        Hit::StanceStylePick(style) => c.stance_style = style,
        Hit::LeanStylePick(style) => c.lean_style = style,
        Hit::GamepadStylePick(style) => c.gamepad_style = style,
        Hit::GamepadThemePick(theme) => c.gamepad_theme = theme,
        Hit::DashFootPick(slot, field) => match slot {
            0 => c.dash_left = field,
            1 => c.dash_mid = field,
            2 => c.dash_right = field,
            _ => {}
        },
        Hit::TickerFootPick(slot, field) => match slot {
            0 => c.ticker_left = field,
            1 => c.ticker_right = field,
            _ => {}
        },
        Hit::InfoPick(bar, slot, field) => set_info_slot(c, bar, slot, field),
        Hit::Preset(p) => c.settings_preset = p,
        Hit::PresetCopyTo(p) => {
            c.copy_settings_to(p);
            c.settings_preset = p;
        }
        Hit::PresetCopyAll => c.copy_settings_to_all(),
        Hit::StDec => c.standings_rows = (c.standings_rows - 1).max(3),
        Hit::StInc => c.standings_rows = (c.standings_rows + 1).min(40),
        Hit::RelDec => c.relative_count = (c.relative_count - 1).max(1),
        Hit::RelInc => c.relative_count = (c.relative_count + 1).min(8),
        Hit::TickerDec => c.ticker_count = (c.ticker_count - 1).max(3),
        Hit::TickerInc => c.ticker_count = (c.ticker_count + 1).min(15),
        Hit::SectorHistDec => c.sector_hist_laps = (c.sector_hist_laps - 1).max(1),
        Hit::SectorHistInc => c.sector_hist_laps = (c.sector_hist_laps + 1).min(5),
        Hit::TabWidgets
        | Hit::TabApp
        | Hit::AppLook
        | Hit::AppMenus
        | Hit::AppInstall
        | Hit::AppStartup
        | Hit::AppLabs
        | Hit::AppUpdates
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
        | Hit::MapDotOpen
        | Hit::MiniDotOpen
        | Hit::FontOpen
        | Hit::PrimaryOpen
        | Hit::PrimaryPanel
        | Hit::PrimarySv
        | Hit::PrimaryHue
        | Hit::PrimarySwatch(_)
        | Hit::PrimaryReset
        | Hit::GameUiMatchPrimary
        | Hit::GameUiPrimaryOpen
        | Hit::GameUiPrimaryPanel
        | Hit::GameUiPrimarySv
        | Hit::GameUiPrimaryHue
        | Hit::GameUiPrimarySwatch(_)
        | Hit::GameUiPrimaryReset
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
        | Hit::GamepadStyleOpen
        | Hit::GamepadThemeOpen
        | Hit::SysAddOpen
        | Hit::DashFootOpen(_)
        | Hit::TickerFootOpen(_)
        | Hit::InfoOpen(_, _)
        | Hit::PresetCopyOpen
        | Hit::UpdateCheck
        | Hit::UpdateInstall
        | Hit::UpdateBanner
        | Hit::UpdateBannerDismiss
        | Hit::WhatsNewOpen
        | Hit::WhatsNewDismiss
        | Hit::WhatsNewScrim
        | Hit::WhatsNewPanel
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
