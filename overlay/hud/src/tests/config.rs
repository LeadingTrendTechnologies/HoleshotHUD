use super::*;

static INI_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn field_keys_round_trip() {
    for field in DashField::ALL {
        assert_eq!(DashField::parse(field.key()), field, "{field:?}");
    }
    for field in BoardField::ALL {
        assert_eq!(BoardField::parse(field.key()), field, "{field:?}");
    }
    for field in StField::ALL {
        assert_eq!(StField::parse(field.key()), Some(field), "{field:?}");
    }
    for field in RelField::ALL {
        assert_eq!(RelField::parse(field.key()), Some(field), "{field:?}");
    }
    assert_eq!(DotLabel::parse(DotLabel::Number.key()), DotLabel::Number);
    assert_eq!(DotLabel::parse(DotLabel::Position.key()), DotLabel::Position);
    for family in FontFamily::ALL {
        assert_eq!(FontFamily::parse(family.key()), family, "{}", family.label());
    }
    assert_eq!(FontFamily::parse("agency"), FontFamily::Exo2);
    assert_eq!(FontFamily::parse("industry"), FontFamily::Teko);
    assert_eq!(FontFamily::parse("faster"), FontFamily::Goldman);
    assert_eq!(FontFamily::parse("bebas"), FontFamily::Goldman);
    assert_eq!(FontFamily::parse("impact"), FontFamily::Montserrat);
    for key in SettingsKey::ALL {
        assert_eq!(SettingsKey::parse(key.key()), key, "{}", key.label());
    }
    assert_eq!(SettingsKey::parse("ins"), SettingsKey::Insert);
    assert_eq!(SettingsKey::parse("nope"), SettingsKey::F8);
    for bind in StanceBind::ALL {
        assert_eq!(StanceBind::parse(&bind.key()), bind, "{}", bind.label());
    }
    for bind in StanceBind::MOUSE {
        assert_eq!(StanceBind::parse(&bind.key()), bind, "{}", bind.label());
    }
    assert_eq!(StanceBind::parse("k32"), StanceBind::Key(0x20));
    assert_eq!(StanceBind::parse("space"), StanceBind::Key(0x20));
    assert_eq!(StanceBind::parse("lmb"), StanceBind::MouseLeft);
    assert_eq!(StanceBind::parse("rb"), StanceBind::PadRb);
    assert_eq!(StanceBind::parse("l1"), StanceBind::PadLb);
    assert_eq!(StanceBind::parse("l2"), StanceBind::PadLt);
    assert_eq!(StanceBind::parse("r2"), StanceBind::PadRt);
    assert_eq!(StanceBind::parse("dpad_up"), StanceBind::PadDpadUp);
    assert_eq!(StanceMode::parse("hold"), StanceMode::Hold);
    assert_eq!(StanceMode::parse("toggle"), StanceMode::Toggle);
    assert_eq!(StanceStyle::parse("icon"), StanceStyle::Icon);
    assert_eq!(StanceStyle::parse("text"), StanceStyle::Text);
}

#[test]
fn default_hud_hides_every_widget() {
    let cfg = HudConfig::new();
    assert!(!cfg[WidgetId::Standings].show);
    assert!(!cfg[WidgetId::Relative].show);
    assert!(!cfg[WidgetId::Map].show);
    assert!(!cfg[WidgetId::Minimap].show);
    assert!(!cfg[WidgetId::Radar].show);
    assert!(cfg.radar_rings);
    assert!(!cfg[WidgetId::Dash].show);
    assert!(!cfg[WidgetId::Ticker].show);
    assert!(!cfg[WidgetId::Sys].show);
    assert!(!cfg[WidgetId::Sector].show);
    assert!(cfg.sector_live);
    assert!(cfg.sector_hist);
    assert_eq!(cfg.sector_hist_laps, 3);
    assert!(!cfg.sector_session);
    assert!(!cfg.delta_session);
    assert!(!cfg[WidgetId::Delta].show);
    assert!(!cfg[WidgetId::Stance].show);
    assert!(!cfg[WidgetId::Flag].show);
    assert!(!cfg.flag_yellow);
    assert!(!cfg.flag_blue);
    assert!(!cfg.any_overlay_widget());
    assert_eq!(cfg.stance_style, StanceStyle::Text);
    assert!(!cfg.stance_show_sit);
    assert!(!cfg.experimental);
    assert!(cfg.whats_new_seen.is_empty());
    assert!(cfg.first_install_version.is_empty());
    assert!(cfg.ticker_title);
    assert_eq!(cfg.font_family, FontFamily::Exo2);
    assert!(cfg.st_stripe);
    assert!(cfg.rel_stripe);
    assert_eq!(cfg[WidgetId::Standings].rect, crate::shm::Rect {
        x: 0.012,
        y: 0.03,
        w: 0.20,
        h: 0.46,
    });
    assert_eq!(cfg[WidgetId::Relative].rect.w, 0.20);
    assert_eq!(cfg[WidgetId::Dash].rect.w, 0.111);
    assert_eq!(cfg[WidgetId::Dash].rect.h, 0.115);
    assert!(!cfg.dash_simple);
    assert_eq!(cfg.dash_left, DashField::Engine);
    assert_eq!(cfg.dash_mid, DashField::Air);
    assert_eq!(cfg.dash_right, DashField::Best);
    assert!(BoardField::any(&BoardField::DEFAULT_HEAD));
    assert!(!BoardField::any(&BoardField::DEFAULT_FOOT));
    assert!(!cfg.standings_cols().is_empty());
    assert!(!cfg.relative_cols().is_empty());
    for id in WidgetId::ALL {
        assert_eq!(cfg.font_pct(id), 100);
        assert!(!cfg[id].show);
        assert!(!cfg[id].bold);
        assert_eq!(cfg[id].font, 100);
    }
}

#[test]
fn widget_prefs_index_mutates_shared_slot() {
    let mut cfg = HudConfig::new();
    cfg[WidgetId::Dash].show = true;
    cfg[WidgetId::Dash].font = 120;
    cfg[WidgetId::Dash].bold = true;
    cfg[WidgetId::Dash].bg = 40;
    cfg[WidgetId::Dash].rect.x = 0.33;
    assert!(cfg[WidgetId::Dash].show);
    assert_eq!(cfg.font_pct(WidgetId::Dash), 120);
    assert!(cfg.bold(WidgetId::Dash));
    assert_eq!(cfg.prefs(WidgetId::Dash).bg, 40);
    assert!((cfg.widget_rect(WidgetId::Dash).x - 0.33).abs() < 0.0001);
    assert!(!cfg[WidgetId::Standings].show);
}

#[test]
fn old_default_dash_rect_migrates() {
    let mut r = crate::shm::Rect {
        x: 0.41,
        y: 0.82,
        w: 0.18,
        h: 0.16,
    };
    super::migrate_default_dash(&mut r);
    assert!((r.w - 0.111).abs() < 0.001);
    assert!((r.h - 0.115).abs() < 0.001);
    let mut mid = crate::shm::Rect {
        x: 0.43,
        y: 0.86,
        w: 0.14,
        h: 0.12,
    };
    super::migrate_default_dash(&mut mid);
    assert!((mid.h - 0.115).abs() < 0.001);
    let mut tiny = crate::shm::Rect {
        x: 0.43,
        y: 0.90,
        w: 0.14,
        h: 0.08,
    };
    super::migrate_default_dash(&mut tiny);
    assert!((tiny.h - 0.115).abs() < 0.001);
    let mut compact = crate::shm::Rect {
        x: 0.442,
        y: 0.872,
        w: 0.115,
        h: 0.108,
    };
    super::migrate_default_dash(&mut compact);
    assert!((compact.w - 0.111).abs() < 0.001);
    assert!((compact.h - 0.115).abs() < 0.001);
    let mut slot = crate::shm::Rect {
        x: 0.4536885,
        y: 0.6840987,
        w: 0.073346466,
        h: 0.10811812,
    };
    super::migrate_default_dash(&mut slot);
    assert!((slot.w - 0.111).abs() < 0.001);
    assert!((slot.y - 0.6840987).abs() < 0.0001);
    let mut custom = crate::shm::Rect {
        x: 0.50,
        y: 0.82,
        w: 0.18,
        h: 0.16,
    };
    super::migrate_default_dash(&mut custom);
    assert!((custom.x - 0.50).abs() < 0.001);
    assert!((custom.w - 0.18).abs() < 0.001);
}

#[test]
fn migrate_default_sector_restores_tall_strip() {
    let mut factory = crate::shm::Rect {
        x: 0.66,
        y: 0.84,
        w: 0.32,
        h: 0.085,
    };
    super::migrate_default_sector(&mut factory);
    assert!((factory.h - 0.22).abs() < 0.001);
    assert!((factory.y - 0.70).abs() < 0.001);
    let mut mid = crate::shm::Rect {
        x: 0.66,
        y: 0.78,
        w: 0.32,
        h: 0.14,
    };
    super::migrate_default_sector(&mut mid);
    assert!((mid.h - 0.22).abs() < 0.001);
    assert!((mid.y - 0.70).abs() < 0.001);
    let mut wide = crate::shm::Rect {
        x: 0.60,
        y: 0.8275,
        w: 0.32,
        h: 0.085,
    };
    super::migrate_default_sector(&mut wide);
    assert!((wide.h - 0.22).abs() < 0.001);
    assert!((wide.w - 0.32).abs() < 0.001);
    let mut custom = crate::shm::Rect {
        x: 0.50,
        y: 0.70,
        w: 0.20,
        h: 0.16,
    };
    super::migrate_default_sector(&mut custom);
    assert!((custom.h - 0.16).abs() < 0.001);
}

#[test]
fn migrate_default_flag_widens_portrait_cloth() {
    let mut old = crate::shm::Rect {
        x: 0.442,
        y: 0.032,
        w: 0.116,
        h: 0.155,
    };
    super::migrate_default_flag(&mut old);
    assert!((old.w - 0.107).abs() < 0.001);
    assert!((old.h - 0.019).abs() < 0.001);
    let mut wide = crate::shm::Rect {
        x: 0.34,
        y: 0.032,
        w: 0.32,
        h: 0.072,
    };
    super::migrate_default_flag(&mut wide);
    assert!((wide.w - 0.107).abs() < 0.001);
    assert!((wide.h - 0.019).abs() < 0.001);
    let mut mock = crate::shm::Rect {
        x: 0.414,
        y: 0.032,
        w: 0.172,
        h: 0.030,
    };
    super::migrate_default_flag(&mut mock);
    assert!((mock.w - 0.107).abs() < 0.001);
    let mut custom = crate::shm::Rect {
        x: 0.10,
        y: 0.20,
        w: 0.20,
        h: 0.12,
    };
    super::migrate_default_flag(&mut custom);
    assert!((custom.w - 0.20).abs() < 0.001);
}

#[test]
fn units_format_speed_and_temp() {
    assert_eq!(Units::parse("imperial").format_speed(10.0), "22");
    assert_eq!(Units::Metric.format_speed(10.0), "36");
    assert_eq!(Units::Metric.format_temp(21.0), "21°C");
    assert_eq!(Units::Imperial.format_temp(0.0), "--°F");
    assert_eq!(Units::Imperial.format_temp(21.0), "70°F");
    assert_eq!(Units::Metric.speed_label(), "KPH");
    assert_eq!(Units::Imperial.speed_label(), "MPH");
    assert_eq!(Units::Metric.format_fuel(5.6, 7.0), "5.6 L");
    assert_eq!(Units::Imperial.format_fuel(5.6, 7.0), "1.5 gal");
    assert_eq!(Units::Metric.format_fuel(0.0, 0.0), "-- L");
    assert_eq!(Units::Imperial.format_fuel(0.0, 0.0), "-- gal");
    assert_eq!(Units::Metric.format_fuel(0.0, 7.0), "0.0 L");
}

#[test]
fn disabled_columns_drop_from_widget_layout() {
    let mut cfg = HudConfig::new();
    cfg.st_name = false;
    cfg.st_pos = false;
    cfg.st_num = false;
    cfg.st_gap = false;
    cfg.st_best = false;
    cfg.st_last = false;
    assert_eq!(cfg.standings_cols(), vec![StField::Name]);
    cfg.rel_name = false;
    cfg.rel_num = false;
    cfg.rel_gap = false;
    cfg.rel_best = false;
    cfg.rel_last = false;
    assert_eq!(cfg.relative_cols(), vec![RelField::Name]);
}

#[test]
fn experimental_no_longer_gates_sectors() {
    let mut cfg = HudConfig::new();
    assert!(!cfg.experimental);
    assert!(!cfg.experimental_unlocked());
    cfg[WidgetId::Sector].show = true;
    cfg[WidgetId::Delta].show = true;
    cfg[WidgetId::Stance].show = true;
    assert!(cfg.sector_visible());
    assert!(cfg.delta_visible());
    assert!(cfg.stance_visible());
    cfg.experimental = true;
    assert!(cfg.sector_visible());
    assert!(cfg.delta_visible());
    assert!(cfg.stance_visible());
    cfg[WidgetId::Sector].show = false;
    cfg[WidgetId::Delta].show = false;
    cfg[WidgetId::Stance].show = false;
    assert!(!cfg.sector_visible());
    assert!(!cfg.delta_visible());
    assert!(!cfg.stance_visible());
}

#[test]
fn ini_round_trip_enables_delta_and_sector() {
    let _g = INI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!("mxbo-ini-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("Holeshot-HUD.ini");
    std::fs::write(
        &path,
        "show_delta=1\nshow_sector=1\nexperimental=1\nfirst_install_version=0.1.0\nst_last=1\nrel_last=1\n",
    )
    .unwrap();
    std::env::set_var("MXBO_TEST_INI", &path);
    let cfg = HudConfig::load_file();
    std::env::remove_var("MXBO_TEST_INI");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(cfg[WidgetId::Delta].show);
    assert!(cfg[WidgetId::Sector].show);
    assert!(cfg.experimental);
    assert!(cfg.delta_visible());
    assert!(cfg.sector_visible());
}

#[test]
fn delta_show_does_not_need_experimental() {
    let _g = INI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!("mxbo-ini-delta-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("Holeshot-HUD.ini");
    std::fs::write(
        &path,
        "show_delta=1\nfirst_install_version=0.1.0\nst_last=1\nrel_last=1\n",
    )
    .unwrap();
    std::env::set_var("MXBO_TEST_INI", &path);
    let cfg = HudConfig::load_file();
    std::env::remove_var("MXBO_TEST_INI");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(cfg[WidgetId::Delta].show);
    assert!(!cfg.experimental);
    assert!(cfg.delta_visible());
    assert!(!cfg.sector_visible());
}

#[test]
fn sector_show_does_not_need_experimental() {
    let _g = INI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!("mxbo-ini-sector-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("Holeshot-HUD.ini");
    std::fs::write(
        &path,
        "show_sector=1\nfirst_install_version=0.1.0\nst_last=1\nrel_last=1\n",
    )
    .unwrap();
    std::env::set_var("MXBO_TEST_INI", &path);
    let cfg = HudConfig::load_file();
    std::env::remove_var("MXBO_TEST_INI");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(cfg[WidgetId::Sector].show);
    assert!(!cfg.experimental);
    assert!(cfg.sector_visible());
}

#[test]
fn widget_id_indexes_match_all() {
    assert_eq!(WidgetId::ALL.len(), WidgetId::COUNT);
    for (i, id) in WidgetId::ALL.iter().copied().enumerate() {
        assert_eq!(id.idx(), i, "{id:?}");
    }
}

#[test]
fn widget_ini_keys_are_unique() {
    let mut seen = std::collections::HashSet::new();
    for id in WidgetId::ALL {
        let k = id.ini();
        for key in [
            k.show,
            k.font,
            k.bold,
            k.bg,
            &format!("{}_x", k.rect),
            &format!("{}_y", k.rect),
            &format!("{}_w", k.rect),
            &format!("{}_h", k.rect),
        ] {
            assert!(seen.insert(key.to_string()), "duplicate INI key {key}");
        }
    }
}

#[test]
fn widget_prefs_round_trip_keeps_legacy_keys() {
    let _g = INI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!("mxbo-prefs-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("Holeshot-HUD.ini");
    std::env::set_var("MXBO_TEST_INI", &path);

    let mut cfg = HudConfig::new();
    cfg.first_install_version = "0.1.0".into();
    cfg[WidgetId::Dash].show = true;
    cfg[WidgetId::Dash].font = 120;
    cfg[WidgetId::Dash].bold = true;
    cfg[WidgetId::Dash].bg = 40;
    cfg[WidgetId::Dash].rect.x = 0.33;
    cfg[WidgetId::Map].show = true;
    cfg[WidgetId::Map].bg = 55;
    cfg.save();
    let text = std::fs::read_to_string(&path).unwrap();
    for key in [
        "dash_x=",
        "show_dash=1",
        "dash_font=120",
        "dash_bold=1",
        "dash_bg=40",
        "show_map=1",
        "map_bg=55",
        "st_font=",
        "mini_font=",
        "show_standings=0",
    ] {
        assert!(text.contains(key), "missing {key} in\n{text}");
    }

    let loaded = HudConfig::load_file();
    std::env::remove_var("MXBO_TEST_INI");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(loaded[WidgetId::Dash].show);
    assert_eq!(loaded[WidgetId::Dash].font, 120);
    assert!(loaded[WidgetId::Dash].bold);
    assert_eq!(loaded[WidgetId::Dash].bg, 40);
    assert!((loaded[WidgetId::Dash].rect.x - 0.33).abs() < 0.0001);
    assert!(loaded[WidgetId::Map].show);
    assert_eq!(loaded[WidgetId::Map].bg, 55);
    assert!(!loaded[WidgetId::Standings].show);
}
