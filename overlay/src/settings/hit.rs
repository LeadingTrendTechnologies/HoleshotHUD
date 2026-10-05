

use crate::config::{
    with_config, BoardField,
    DashField, GamepadStyle, GamepadTheme, HudConfig, LeanStyle,
    RadarStyle, SessionPreset, SettingsKey, SettingsTheme, SnapAlign,
    StanceMode, StanceStyle, UnitKind, Units, WidgetId,
};

use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tab {
    App,
    Review,
    Profile,
    Feedback,
    Groups,
    Standings,
    Relative,
    Map,
    Minimap,
    Radar,
    Dash,
    Ticker,
    Sys,
    Sector,
    Delta,
    Stance,
    Flag,
    Lean,
    Gamepad,
    Telemetry,
    Pitboard,
    Timer,
}

impl Tab {
    pub(crate) fn is_widget(self) -> bool {
        !matches!(
            self,
            Tab::App | Tab::Review | Tab::Profile | Tab::Feedback | Tab::Groups
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum AppSection {
    Look,
    Menus,
    Install,
    Startup,
    Stream,
    Labs,
    Updates,
    Diagnostics,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProfileSection {
    Overview,
    Motos,
    Ranked,
    Tracks,
}

impl ProfileSection {
    pub(crate) fn label(self) -> &'static str {
        mxbo_hud::i18n::t(match self {
            Self::Overview => "Overview",
            Self::Motos => "Motos",
            Self::Ranked => "MXB-Ranked",
            Self::Tracks => "Tracks",
        })
    }

    pub(crate) fn hit(self) -> Hit {
        match self {
            Self::Overview => Hit::ProfileNavOverview,
            Self::Motos => Hit::ProfileNavMotos,
            Self::Ranked => Hit::ProfileNavRanked,
            Self::Tracks => Hit::ProfileNavTracks,
        }
    }
}

impl AppSection {
    pub(crate) fn label(self) -> &'static str {
        mxbo_hud::i18n::t(match self {
            Self::Look => "Look",
            Self::Menus => "In game HUD",
            Self::Install => "Install",
            Self::Startup => "Startup",
            Self::Stream => "Stream",
            Self::Labs => "Labs",
            Self::Updates => "Updates",
            Self::Diagnostics => "Diagnostics",
        })
    }

    pub(crate) fn hit(self) -> Hit {
        match self {
            Self::Look => Hit::AppLook,
            Self::Menus => Hit::AppMenus,
            Self::Install => Hit::AppInstall,
            Self::Startup => Hit::AppStartup,
            Self::Stream => Hit::AppStream,
            Self::Labs => Hit::AppLabs,
            Self::Updates => Hit::AppUpdates,
            Self::Diagnostics => Hit::AppDiagnostics,
        }
    }
}

/// Settings left rail: HUD chrome separate from app install/startup/menus.
pub(crate) fn app_section_groups() -> Vec<(&'static str, Vec<AppSection>)> {
    let mut groups = vec![
        (mxbo_hud::i18n::t("HUD"), vec![AppSection::Look]),
        (
            mxbo_hud::i18n::t("App"),
            vec![
                AppSection::Menus,
                AppSection::Install,
                AppSection::Startup,
                AppSection::Stream,
                AppSection::Labs,
                AppSection::Updates,
                AppSection::Diagnostics,
            ],
        ),
    ];
    groups.sort_by(|left, right| left.0.cmp(right.0));
    for (_, items) in &mut groups {
        items.sort_by_key(|item| item.label());
    }
    groups
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Hit {
    TabWidgets,
    TabApp,
    TabProfile,
    TabFeedback,
    ProfileNavOverview,
    ProfileNavMotos,
    ProfileNavRanked,
    ProfileNavTracks,
    RankedRefresh,
    TrackOpen(u16),
    TrackBack,
    TrackMap,
    AppLook,
    AppMenus,
    AppInstall,
    AppStartup,
    AppStream,
    AppLabs,
    AppUpdates,
    AppDiagnostics,
    TabGroups,
    GroupSelect(u8),
    GroupUp(u8),
    GroupDown(u8),
    GroupNew,
    GroupDelete,
    GroupsOnMaps,
    GroupIcon(u8),
    GroupColor(u8),
    GroupMemberRemove(u8),
    GroupCheck(u8),
    GroupAddSelected,
    GroupAddName,
    GroupName,
    GroupMember,
    DiagCopy,
    ReviewFilterAll,
    ReviewFilterRanked,
    ReviewFilterSaved,
    ReviewOpen(u64),
    ReviewKeep(u64),
    ReviewDelete(u64),
    ReviewBack,
    ReviewToggle,
    ProfileAllTime,
    ProfileTwoWeeks,
    ProfileRanked,
    ProfileClear,
    ProfileAxis(u8),
    ReviewClear,
    AnalyzeCompare(i32),
    AnalyzeCompareOpen,
    AnalyzeYouLap(i32),
    AnalyzeLapOpen,
    AnalyzeScrub,
    AnalyzeMap,
    AnalyzeFollow,
    TabSt,
    TabRel,
    TabMap,
    TabMini,
    TabRadar,
    TabDash,
    TabTicker,
    TabSys,
    TabSector,
    TabDelta,
    TabStance,
    TabFlag,
    TabLean,
    TabGamepad,
    TabTelemetry,
    TabPitboard,
    TabTimer,
    Preset(SessionPreset),
    PresetCopyOpen,
    PresetCopyTo(SessionPreset),
    PresetCopyAll,
    StShow,
    RelShow,
    MapShow,
    MiniShow,
    RadarShow,
    DashShow,
    DashRev,
    DashYellow,
    DashBlue,
    DashRed,
    DashSimple,
    DashShiftColor,
    TickerShow,
    SysShow,
    SysAppShow(u8),
    SysAppRemove(u8),
    SysAddOpen,
    SysAddPick(u8),
    SysAppBrowse,
    SectorShow,
    SectorLive,
    SectorSession,
    SectorHist,
    SectorHistDec,
    SectorHistInc,
    DeltaShow,
    DeltaSession,
    TrackPbClear,
    StanceShow,
    StanceShowSit,
    FlagShow,
    LeanShow,
    GamepadShow,
    TelemetryShow,
    PitShow,
    TimerShow,
    TimerOf,
    TimerBg,
    PitBrowse,
    PitName,
    PitBoardOpen,
    PitBoardFactory,
    PitBoardPick(u8),
    PitExport,
    PitImport,
    PitDemo,
    PitHelpOpen,
    PitHelpDismiss,
    PitHelpScrim,
    PitHelpPanel,
    PitDelete,
    PitReset,
    #[allow(dead_code)]
    PitOpenFolder,
    PitWhenAlways,
    PitWhenSector,
    PitWhenLap,
    PitWhenOpen,
    PitSlotOpen(u8),
    PitSlotSelect(u8),
    PitSlotPick(u8, u8),
    PitSlotNone(u8),
    PitSlotGrab(u8),
    PitSlotAdd,
    PitSlotRemove,
    PitSlotSize,
    PitSlotBold,
    PitSlotCenterX,
    PitSlotCenterY,
    PitSlotColorOpen,
    PitSlotColorPanel,
    PitSlotColorSaturation,
    PitSlotColorHue,
    PitSlotColorSwatch(u8),
    PitSlotColorReset,
    TelemetryTraces,
    TelemetryTraceThrottle,
    TelemetryTraceBrake,
    TelemetryTraceSteer,
    TelemetryBars,
    TelemetryBarClutch,
    TelemetryBarBrake,
    TelemetryBarThrottle,
    TelemetryBarSteer,
    TelemetryDial,
    FlagYellow,
    FlagBlue,
    FlagRed,
    FlagText,
    TickerTitle,
    TickerAutoscroll,
    TickerStatus,
    TickerSlide,
    StPos,
    StNum,
    StName,
    StGap,
    StLaps,
    StCurrent,
    StBest,
    StLast,
    StStatus,
    StBike,
    StPenalty,
    StCrashed,
    StInterval,
    StCategory,
    StLapDiff,
    RelNum,
    RelName,
    RelGap,
    RelLaps,
    RelCurrent,
    RelPos,
    RelBike,
    RelPenalty,
    RelInterval,
    RelStatus,
    RelBest,
    RelLast,
    RelCategory,
    RelSpeed,
    RelLapDiff,
    MapOthers,
    MapSf,
    MapSectors,
    MapArrows,
    MapFollow,
    MapCrown,
    MapPlace,
    MapNumbers,
    MapDotOpen,
    MapDotNum,
    MapDotPos,
    MapYouTextOpen,
    MapYouTextWhite,
    MapYouTextBlack,
    MiniOthers,
    MiniSf,
    MiniSectors,
    MiniArrows,
    MiniCrown,
    MiniPlace,
    MiniNumbers,
    MiniDotOpen,
    MiniDotNum,
    MiniDotPos,
    RadarSides,
    RadarRear,
    RadarRings,
    RadarRange,
    StBg,
    StHl,
    StStripe,
    StTextOpen,
    StTextWhite,
    StTextBlack,
    StPlaqueTextOpen,
    StPlaqueTextWhite,
    StPlaqueTextBlack,
    StPlaque,
    RelBg,
    RelHl,
    RelStripe,
    RelTextOpen,
    RelTextWhite,
    RelTextBlack,
    RelPlaqueTextOpen,
    RelPlaqueTextWhite,
    RelPlaqueTextBlack,
    RelPlaque,
    MapBg,
    MapZoom,
    MapDotOpacity,
    MiniBg,
    MiniZoom,
    RadarBg,
    DashBg,
    TickerBg,
    TickerHl,
    SysBg,
    SectorBg,
    DeltaBg,
    StanceBg,
    FlagBg,
    LeanBg,
    GamepadBg,
    TelemetryBg,
    PitBg,
    StDec,
    StInc,
    RelDec,
    RelInc,
    TickerDec,
    TickerInc,
    StDrag(u8),
    RelDrag(u8),
    StW(u8),
    RelW(u8),
    Font(WidgetId),
    Bold(WidgetId),
    Snap(WidgetId, SnapAlign),
    FontOpen,
    FontSegoe,
    FontArial,
    FontTahoma,
    FontRoboto,
    FontExo2,
    FontTeko,
    FontGoldman,
    FontMontserrat,
    UnitsOpen(UnitKind),
    UnitsPick(UnitKind, Units),
    SettingsKeyOpen,
    SettingsKeyPick(SettingsKey),
    LanguageOpen,
    LanguagePick(crate::config::Language),
    ThemeOpen,
    ThemePick(SettingsTheme),
    StanceBindOpen,
    StanceModeOpen,
    StanceModePick(StanceMode),
    StanceStyleOpen,
    StanceStylePick(StanceStyle),
    LeanStyleOpen,
    LeanStylePick(LeanStyle),
    RadarStyleOpen,
    RadarStylePick(RadarStyle),
    GamepadStyleOpen,
    GamepadStylePick(GamepadStyle),
    GamepadThemeOpen,
    GamepadThemePick(GamepadTheme),
    StanceReset,
    DashFootOpen(u8),
    DashFootPick(u8, DashField),
    TickerFootOpen(u8),
    TickerFootPick(u8, BoardField),
    InfoOpen(InfoBar, u8),
    InfoPick(InfoBar, u8, BoardField),
    UpdateCheck,
    UpdateInstall,
    UpdateBanner,
    UpdateBannerDismiss,
    WhatsNewOpen,
    WhatsNewDismiss,
    WhatsNewScrim,
    WhatsNewPanel,
    WhatsNewLink,
    ClearScrim,
    ClearPanel,
    ClearCancel,
    ClearConfirm,
    ReplyDismiss,
    ReplySend,
    ReplyText,
    ReplyScrim,
    ReplyPanel,
    StartWithWindows,
    MinimizeOnClose,
    CloseWithGame,
    OpenWithGame,
    StreamEnabled,
    StreamCopyUrl,
    StreamCopyEditUrl,
    StreamOpenUrl,
    StreamOpenEditUrl,
    StreamCopyGameToStream,
    FeatureSector,
    AutoUpdateOnLaunch,
    QuitApp,
    Uninstall,
    GameFolder,
    GameUi,
    GameUiMatchPrimary,
    PrimaryOpen,
    PrimaryPanel,
    PrimarySaturation,
    PrimaryHue,
    PrimarySwatch(u8),
    PrimaryReset,
    GameUiPrimaryOpen,
    GameUiPrimaryPanel,
    GameUiPrimarySaturation,
    GameUiPrimaryHue,
    GameUiPrimarySwatch(u8),
    GameUiPrimaryReset,
    PitYellowOpen,
    PitYellowPanel,
    PitYellowSaturation,
    PitYellowHue,
    PitYellowSwatch(u8),
    PitYellowReset,
    PitBlueOpen,
    PitBluePanel,
    PitBlueSaturation,
    PitBlueHue,
    PitBlueSwatch(u8),
    PitBlueReset,
    GameUiSplashBrowse,
    GameUiSplashDefault,
    GameUiLoadingBrowse,
    GameUiLoadingDefault,
    FbRate,
    FbBug,
    FbFeature,
    FbStar(u8),
    FbText,
    FbAttach,
    FbSend,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum InfoBar {
    StHead,
    StFoot,
    RelHead,
    RelFoot,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct HitBox {
    pub(crate) id: Hit,
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) w: f32,
    pub(crate) h: f32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Drop {
    MapDot,
    MapYouText,
    MiniDot,
    FontFamily,
    Units(UnitKind),
    SettingsKey,
    Language,
    Theme,
    StanceMode,
    StanceStyle,
    LeanStyle,
    RadarStyle,
    GamepadStyle,
    GamepadTheme,
    SysAdd,
    StText,
    RelText,
    StPlaqueText,
    RelPlaqueText,
    DashFoot(u8),
    TickerFoot(u8),
    Info(InfoBar, u8),
    PitSlot(u8),
    PitWhen,
    PitBoard,
    PresetCopy,
    AnalyzeCompare,
    AnalyzeLap,
    PrimaryColor,
    GameUiPrimaryColor,
    PitYellow,
    PitBlue,
    PitSlotColor,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ColorPickKind {
    App,
    Menu,
    PitYellow,
    PitBlue,
    PitSlot,
}

impl ColorPickKind {
    pub(crate) fn open(self) -> Hit {
        match self {
            Self::App => Hit::PrimaryOpen,
            Self::Menu => Hit::GameUiPrimaryOpen,
            Self::PitYellow => Hit::PitYellowOpen,
            Self::PitBlue => Hit::PitBlueOpen,
            Self::PitSlot => Hit::PitSlotColorOpen,
        }
    }

    pub(crate) fn panel(self) -> Hit {
        match self {
            Self::App => Hit::PrimaryPanel,
            Self::Menu => Hit::GameUiPrimaryPanel,
            Self::PitYellow => Hit::PitYellowPanel,
            Self::PitBlue => Hit::PitBluePanel,
            Self::PitSlot => Hit::PitSlotColorPanel,
        }
    }

    pub(crate) fn saturation_value(self) -> Hit {
        match self {
            Self::App => Hit::PrimarySaturation,
            Self::Menu => Hit::GameUiPrimarySaturation,
            Self::PitYellow => Hit::PitYellowSaturation,
            Self::PitBlue => Hit::PitBlueSaturation,
            Self::PitSlot => Hit::PitSlotColorSaturation,
        }
    }

    pub(crate) fn hue(self) -> Hit {
        match self {
            Self::App => Hit::PrimaryHue,
            Self::Menu => Hit::GameUiPrimaryHue,
            Self::PitYellow => Hit::PitYellowHue,
            Self::PitBlue => Hit::PitBlueHue,
            Self::PitSlot => Hit::PitSlotColorHue,
        }
    }

    pub(crate) fn swatch(self, i: u8) -> Hit {
        match self {
            Self::App => Hit::PrimarySwatch(i),
            Self::Menu => Hit::GameUiPrimarySwatch(i),
            Self::PitYellow => Hit::PitYellowSwatch(i),
            Self::PitBlue => Hit::PitBlueSwatch(i),
            Self::PitSlot => Hit::PitSlotColorSwatch(i),
        }
    }

    pub(crate) fn reset(self) -> Hit {
        match self {
            Self::App => Hit::PrimaryReset,
            Self::Menu => Hit::GameUiPrimaryReset,
            Self::PitYellow => Hit::PitYellowReset,
            Self::PitBlue => Hit::PitBlueReset,
            Self::PitSlot => Hit::PitSlotColorReset,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        mxbo_hud::i18n::t(match self {
            Self::App => "Primary color",
            Self::Menu => "Menu color",
            Self::PitYellow => "Main",
            Self::PitBlue => "Secondary",
            Self::PitSlot => "Text",
        })
    }
}

pub(crate) fn sync_menus_after_app_primary_change() {
    let (on, matched) = with_config(|c| (c.game_ui, c.game_ui_match_primary));
    if on && matched {
        crate::game_ui::sync_from_config();
    }
}

pub(crate) fn sync_menus_after_menu_accent_change() {
    if with_config(|c| c.game_ui) {
        crate::game_ui::sync_from_config();
    }
}

pub(crate) fn color_kind_from_drop(drop: Option<Drop>) -> Option<ColorPickKind> {
    match drop {
        Some(Drop::PrimaryColor) => Some(ColorPickKind::App),
        Some(Drop::GameUiPrimaryColor) => Some(ColorPickKind::Menu),
        Some(Drop::PitYellow) => Some(ColorPickKind::PitYellow),
        Some(Drop::PitBlue) => Some(ColorPickKind::PitBlue),
        Some(Drop::PitSlotColor) => Some(ColorPickKind::PitSlot),
        _ => None,
    }
}

pub(crate) fn color_kind_from_hue(hit: Hit) -> Option<ColorPickKind> {
    match hit {
        Hit::PrimaryHue => Some(ColorPickKind::App),
        Hit::GameUiPrimaryHue => Some(ColorPickKind::Menu),
        Hit::PitYellowHue => Some(ColorPickKind::PitYellow),
        Hit::PitBlueHue => Some(ColorPickKind::PitBlue),
        Hit::PitSlotColorHue => Some(ColorPickKind::PitSlot),
        _ => None,
    }
}

pub(crate) fn read_kind_color(cfg: &HudConfig, kind: ColorPickKind) -> [u8; 3] {
    match kind {
        ColorPickKind::App => cfg.primary,
        ColorPickKind::Menu => cfg.game_ui_primary,
        ColorPickKind::PitYellow => cfg.pit.pit_yellow,
        ColorPickKind::PitBlue => cfg.pit.pit_blue,
        ColorPickKind::PitSlot => cfg
            .pit.pit_vars
            .get(pit_selected() as usize)
            .and_then(|place| place.color)
            .unwrap_or([16, 16, 18]),
    }
}

pub(crate) fn write_kind_color(cfg: &mut HudConfig, kind: ColorPickKind, rgb: [u8; 3]) {
    match kind {
        ColorPickKind::App => cfg.primary = rgb,
        ColorPickKind::Menu => cfg.game_ui_primary = rgb,
        ColorPickKind::PitYellow => cfg.pit.pit_yellow = rgb,
        ColorPickKind::PitBlue => cfg.pit.pit_blue = rgb,
        ColorPickKind::PitSlot => {
            if let Some(place) = cfg.pit.pit_vars.get_mut(pit_selected() as usize) {
                place.color = Some(rgb);
            }
            let _ = mxbo_hud::pitboard::write_pack(&cfg.pit.pit_art, &cfg.pit.pit_vars, cfg.pit.pit_text);
        }
    }
}

pub(crate) fn sync_after_kind(kind: ColorPickKind) {
    match kind {
        ColorPickKind::App => sync_menus_after_app_primary_change(),
        ColorPickKind::Menu => sync_menus_after_menu_accent_change(),
        ColorPickKind::PitYellow | ColorPickKind::PitBlue | ColorPickKind::PitSlot => {}
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum DragKind {
    St,
    Rel,
}

#[derive(Clone, Copy)]
pub(crate) struct ColDrag {
    pub(crate) kind: DragKind,
    pub(crate) from: u8,
    pub(crate) over: u8,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct SlideDrag {
    pub(crate) hit: Hit,
    pub(crate) x: f32,
    pub(crate) w: f32,
    pub(crate) min: i32,
    pub(crate) max: i32,
}

pub(crate) struct ColRowSlide {
    id: i32,
    from: f32,
    to: f32,
    start: f32,
}

pub(crate) struct ColSlides {
    rows: Vec<ColRowSlide>,
}

impl ColSlides {
    pub(crate) fn new() -> Self {
        Self { rows: Vec::new() }
    }

    pub(crate) fn indices(&mut self, ids: &[i32], now: f32) -> Vec<f32> {
        const DUR: f32 = 0.30;
        let displayed = |e: &ColRowSlide| {
            let t = ((now - e.start) / DUR).clamp(0.0, 1.0);
            e.from + (e.to - e.from) * col_ease_out_cubic(t)
        };
        let mut out = Vec::with_capacity(ids.len());
        for (i, &id) in ids.iter().enumerate() {
            let target = i as f32;
            if let Some(idx) = self.rows.iter().position(|e| e.id == id) {
                let cur = displayed(&self.rows[idx]);
                let e = &mut self.rows[idx];
                if (e.to - target).abs() > 0.05 {
                    e.from = cur;
                    e.to = target;
                    e.start = now;
                }
                out.push(displayed(e));
            } else {
                self.rows.push(ColRowSlide {
                    id,
                    from: target,
                    to: target,
                    start: now,
                });
                out.push(target);
            }
        }
        self.rows.retain(|e| ids.contains(&e.id));
        out
    }
}

pub(crate) fn col_ease_out_cubic(t: f32) -> f32 {
    let u = 1.0 - t.clamp(0.0, 1.0);
    1.0 - u * u * u
}

pub(crate) fn col_anim_now() -> f32 {
    thread_local! {
        static ORIGIN: std::time::Instant = std::time::Instant::now();
    }
    ORIGIN.with(|o| o.elapsed().as_secs_f32())
}

pub(crate) fn col_stride() -> f32 {
    ROW_H + ROW_GAP
}

pub(crate) fn preview_move_to<T: Copy>(items: &[T], from: usize, to: usize) -> Vec<T> {
    let mut out = items.to_vec();
    if from >= out.len() || to >= out.len() || from == to {
        return out;
    }
    let item = out.remove(from);
    out.insert(to, item);
    out
}

pub(crate) fn drag_over_slots(list_y: f32, n: usize, y: f32) -> Option<u8> {
    if n == 0 {
        return None;
    }
    let stride = col_stride();
    if y < list_y {
        return Some(0);
    }
    let last = n - 1;
    let last_bottom = list_y + last as f32 * stride + ROW_H;
    if y >= last_bottom {
        return Some(last as u8);
    }
    let i = ((y - list_y) / stride).floor() as usize;
    Some(i.min(last) as u8)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClearKind {
    Profile,
    Motos,
}
