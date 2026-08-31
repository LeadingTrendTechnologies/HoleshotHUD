use super::*;

#[test]
fn local_bike_reads_standings_row() {
    let mut s = Snapshot::default();
    assert_eq!(s.local_bike(), "");
    s.local_race_num = 7;
    s.standing_count = 2;
    s.standings[0].race_num = 3;
    write_name(&mut s.standings[0].bike, "YZ250F");
    s.standings[1].race_num = 7;
    write_name(&mut s.standings[1].bike, "YZ450F");
    assert_eq!(s.local_bike(), "YZ450F");
}

#[test]
fn write_and_read_cstr() {
    let mut buf = [0u8; NAME];
    write_name(&mut buf, "Troy");
    assert_eq!(cstr(&buf), "Troy");
    write_name(&mut buf, "");
    assert_eq!(cstr(&buf), "");
}

#[test]
fn cstr_strips_windows1252_trademark() {
    // "OEM YZ450F" + Windows-1252 ™ (0x99) — common on bike short names.
    let mut buf = [0u8; NAME];
    let raw = b"OEM YZ450F\x99";
    buf[..raw.len()].copy_from_slice(raw);
    assert_eq!(cstr(&buf), "OEM YZ450F");
}

#[test]
fn cstr_strips_registered_and_copyright() {
    let mut buf = [0u8; NAME];
    write_name(&mut buf, "YZ450F®");
    assert_eq!(cstr(&buf), "YZ450F");
    write_name(&mut buf, "Honda©");
    assert_eq!(cstr(&buf), "Honda");
}

#[test]
fn snapshot_default_layout_is_sane() {
    let mut s = Snapshot::default();
    assert_eq!(s.magic, 0);
    assert_eq!(s.map.x, 0.775);
    assert_eq!(s.map.y, 0.62);
    assert_eq!(s.map.w, 0.21);
    assert_eq!(s.map.h, 0.34);
    assert_eq!(s.standings_rect.x, 0.012);
    assert_eq!(s.standings_rect.y, 0.03);
    assert_eq!(s.standings_rect.w, 0.20);
    assert_eq!(s.standings_rect.h, 0.42);
    assert_eq!(s.relative.x, 0.012);
    assert_eq!(s.relative.y, 0.62);
    assert_eq!(s.relative.w, 0.20);
    assert_eq!(s.relative.h, 0.33);
    assert_eq!(s.show_standings, 1);
    assert_eq!(s.on_track, 0);
    assert!(!s.has_session_data());
    s.standing_count = 2;
    assert!(s.has_session_data());
}

#[test]
fn snapshot_dump_skips_empty_arrays() {
    let mut s = Snapshot::default();
    s.poly_count = 8;
    s.poly[0] = Point { x: 1.0, z: 2.0 };
    s.poly[7] = Point { x: 9.0, z: 8.0 };
    s.rider_count = 1;
    s.riders[0].race_num = 12;
    write_name(&mut s.riders[0].name, "Troy");
    s.standing_count = 1;
    s.standings[0].position = 1;
    s.standings[0].race_num = 12;
    let dump = s.dump_text();
    assert!(dump.contains("session_laps=0"));
    assert!(dump.contains("session_kind=-1"));
    assert!(dump.contains("session_state=-1"));
    assert!(dump.contains("poly_count=8"));
    assert!(dump.contains("poly[0]="));
    assert!(dump.contains("poly[7]="));
    assert!(!dump.contains("poly[4]="));
    assert!(dump.contains("rider_count=1"));
    assert!(dump.contains("#12"));
    assert!(dump.contains("Troy"));
    assert!(!dump.contains("rider[1]"));
}
