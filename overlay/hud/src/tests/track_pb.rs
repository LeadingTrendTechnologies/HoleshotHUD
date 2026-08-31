use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

fn tmp_dir() -> PathBuf {
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("mxbo-pb-{n}"));
    let _ = fs::create_dir_all(&dir);
    dir
}

#[test]
fn slug_strips_windows_illegal() {
    assert_eq!(slug("Paleta Raceway v2"), "Paleta Raceway v2");
    assert_eq!(slug("a/b:c*"), "a_b_c_");
    assert_eq!(slug("..."), "_");
}

#[test]
fn bike_class_is_displacement() {
    assert_eq!(bike_class(""), "");
    assert_eq!(bike_class("YZ450F"), "450");
    assert_eq!(bike_class("CRF250R"), "250");
    assert_eq!(bike_class("450 SX-F"), "450");
    assert_eq!(bike_class("FC 250"), "250");
    assert_eq!(bike_class("OEM YZ450F"), "450");
    assert_eq!(bike_class("KX450"), "450");
    assert_eq!(bike_class("YZ125"), "125");
    assert_eq!(bike_class("FC 350"), "350");
    assert_eq!(bike_class("250"), "250");
    assert_eq!(bike_class("450"), "450");
}

#[test]
fn round_trip_json() {
    let mut pb = TrackPb::empty();
    pb.lap_ms = 71_234;
    pb.bins[0] = 10;
    pb.bins[255] = 71_200;
    pb.sectors = [24_093, 25_640, 22_910];
    pb.split_milli = [333, 671];
    pb.used = 1_700_000_000;
    let back = decode(&encode(&pb)).unwrap();
    assert_eq!(back, pb);
}

fn bins_json() -> String {
    let mut bins = String::from("[");
    for i in 0..BINS {
        if i > 0 {
            bins.push(',');
        }
        bins.push('0');
    }
    bins.push(']');
    bins
}

#[test]
fn old_json_without_split_pos_loads() {
    let text = format!(
        "{{\"v\":1,\"ms\":1,\"bins\":{},\"s\":[1,2,3]}}",
        bins_json()
    );
    let pb = decode(&text).unwrap();
    assert_eq!(pb.sectors, [1, 2, 3]);
    assert_eq!(pb.split_milli, [0, 0]);
    assert_eq!(pb.used, 0);
}

#[test]
fn old_json_without_used_loads() {
    let text = format!(
        "{{\"v\":1,\"ms\":1,\"bins\":{},\"s\":[1,2,3],\"p\":[333,671]}}",
        bins_json()
    );
    let pb = decode(&text).unwrap();
    assert_eq!(pb.split_milli, [333, 671]);
    assert_eq!(pb.used, 0);
}

#[test]
fn time_at_reads_bin_and_interpolates() {
    let mut bins = [0; BINS];
    bins[10] = 1_000;
    bins[20] = 2_000;
    assert_eq!(time_at(&bins, 10.0 / BINS as f32), Some(1_000));
    assert_eq!(time_at(&bins, 15.0 / BINS as f32), Some(1_500));
    bins[11] = 1_100;
    assert_eq!(time_at(&bins, 10.5 / BINS as f32), Some(1_050));
    bins[11] = 0;
    bins[12] = 1_200;
    assert_eq!(time_at(&bins, 10.5 / BINS as f32), Some(1_050));
    let a = time_at(&bins, 10.2 / BINS as f32).unwrap();
    let b = time_at(&bins, 10.8 / BINS as f32).unwrap();
    assert!(b > a, "fractional walk must move, {a} then {b}");
    assert_eq!(pos_at_time(&bins, 0), None);
    let p = pos_at_time(&bins, 1_500).unwrap();
    assert!((p * BINS as f32 - 15.0).abs() < 0.6, "pos {p}");
}

#[test]
fn time_at_rejects_large_gaps() {
    let mut bins = [0; BINS];
    bins[0] = 1_000;
    bins[BINS / 4 + 2] = 10_000;
    assert_eq!(time_at(&bins, 0.2), None);
}

#[test]
fn note_split_pos_rounds_and_skips_noise() {
    let _lock = exclusive_test();
    let dir = tmp_dir();
    set_store_dir(dir.clone());
    assert!(!note_split_pos("A", "", 0, 0.03));
    assert!(!note_split_pos("A", "", 0, 0.97));
    assert!(note_split_pos("A", "", 0, 0.33));
    assert_eq!(bind("A", "").split_milli[0], 330);
    assert!(!note_split_pos("A", "", 0, 0.333));
    assert!(note_split_pos("A", "", 1, 0.67));
    assert_eq!(bind("A", "").split_milli[1], 670);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn clear_current_drops_loaded_tape() {
    let _lock = exclusive_test();
    let dir = tmp_dir();
    set_store_dir(dir.clone());
    let mut bins = [0; BINS];
    bins[0] = 100;
    bins[200] = 80_000;
    assert!(commit_tape("A", "", 80_000, bins));
    assert!(bind("A", "").has_tape());
    clear_current();
    assert!(!bind("A", "").has_tape());
    assert_eq!(bind("A", "").lap_ms, 0);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn corrupt_file_is_ignored() {
    assert!(decode("{nope}").is_none());
    assert!(decode("{\"v\":1,\"ms\":1,\"bins\":[1],\"s\":[1,2,3]}").is_none());
}

#[test]
fn faster_tape_overwrites_slower_does_not() {
    let _lock = exclusive_test();
    let dir = tmp_dir();
    set_store_dir(dir.clone());
    let mut bins = [0; BINS];
    bins[0] = 100;
    bins[200] = 80_000;
    assert!(commit_tape("A", "", 80_000, bins));
    let mut slower = bins;
    slower[200] = 90_000;
    assert!(!commit_tape("A", "", 90_000, slower));
    assert_eq!(bind("A", "").lap_ms, 80_000);
    let mut faster = bins;
    faster[200] = 70_000;
    assert!(commit_tape("A", "", 70_000, faster));
    assert_eq!(bind("A", "").lap_ms, 70_000);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn tracks_stay_isolated() {
    let _lock = exclusive_test();
    let dir = tmp_dir();
    set_store_dir(dir.clone());
    let mut a = [0; BINS];
    a[1] = 1;
    commit_tape("A", "", 60_000, a);
    commit_sector("A", "", 0, 20_000);
    let mut b = [0; BINS];
    b[1] = 2;
    commit_tape("B", "", 90_000, b);
    commit_sector("B", "", 0, 30_000);
    assert_eq!(bind("A", "").lap_ms, 60_000);
    assert_eq!(bind("A", "").sectors[0], 20_000);
    assert_eq!(bind("B", "").lap_ms, 90_000);
    assert_eq!(bind("B", "").sectors[0], 30_000);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn faster_sector_updates_file() {
    let _lock = exclusive_test();
    let dir = tmp_dir();
    set_store_dir(dir.clone());
    assert!(commit_sector("A", "", 0, 24_093));
    assert!(!commit_sector("A", "", 0, 25_000));
    assert!(commit_sector("A", "", 0, 23_000));
    assert_eq!(bind("A", "").sectors[0], 23_000);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn commit_tape_stamps_used() {
    let _lock = exclusive_test();
    let dir = tmp_dir();
    set_store_dir(dir.clone());
    let mut bins = [0; BINS];
    bins[0] = 100;
    bins[200] = 80_000;
    let before = now_unix();
    assert!(commit_tape("A", "", 80_000, bins));
    let after = now_unix();
    let used = bind("A", "").used;
    assert!(used >= before && used <= after, "used {used} not in [{before}, {after}]");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn bind_does_not_create_file_for_unknown_track() {
    let _lock = exclusive_test();
    let dir = tmp_dir();
    set_store_dir(dir.clone());
    assert_eq!(bind("Ghost", "").used, 0);
    assert!(!dir.join("Ghost.json").exists());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn bind_stamps_existing_file_missing_used() {
    let _lock = exclusive_test();
    let dir = tmp_dir();
    let _ = fs::create_dir_all(&dir);
    let text = format!(
        "{{\"v\":1,\"ms\":80000,\"bins\":{},\"s\":[1,2,3],\"p\":[0,0]}}",
        bins_json()
    );
    fs::write(dir.join("A.json"), text).unwrap();
    set_store_dir(dir.clone());
    let before = now_unix();
    let pb = bind("A", "");
    let after = now_unix();
    assert_eq!(pb.lap_ms, 80_000);
    assert!(
        pb.used >= before && pb.used <= after,
        "used {} not in [{before}, {after}]",
        pb.used
    );
    let on_disk = fs::read_to_string(dir.join("A.json")).unwrap();
    assert!(on_disk.contains(&format!("\"used\":{}", pb.used)));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn bind_skips_rewrite_when_used_is_fresh() {
    let _lock = exclusive_test();
    let dir = tmp_dir();
    set_store_dir(dir.clone());
    let mut bins = [0; BINS];
    bins[0] = 100;
    bins[200] = 80_000;
    assert!(commit_tape("A", "", 80_000, bins));
    let stamped = bind("A", "").used;
    bind("B", "");
    assert_eq!(bind("A", "").used, stamped);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn bind_restamps_stale_used() {
    let _lock = exclusive_test();
    let dir = tmp_dir();
    let _ = fs::create_dir_all(&dir);
    let stale = now_unix() - TOUCH_SECS - 1;
    let text = format!(
        "{{\"v\":1,\"ms\":80000,\"bins\":{},\"s\":[0,0,0],\"p\":[0,0],\"used\":{stale}}}",
        bins_json()
    );
    fs::write(dir.join("A.json"), text).unwrap();
    set_store_dir(dir.clone());
    let before = now_unix();
    let used = bind("A", "").used;
    assert!(used >= before, "used {used} should refresh past stale {stale}");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn bikes_on_same_track_stay_isolated() {
    let _lock = exclusive_test();
    let dir = tmp_dir();
    set_store_dir(dir.clone());
    let mut a = [0; BINS];
    a[1] = 1;
    assert!(commit_tape("A", "YZ450F", 60_000, a));
    assert!(commit_sector("A", "YZ450F", 0, 20_000));
    let mut b = [0; BINS];
    b[1] = 2;
    assert!(commit_tape("A", "YZ250F", 90_000, b));
    assert!(commit_sector("A", "YZ250F", 0, 30_000));
    assert_eq!(bind("A", "YZ450F").lap_ms, 60_000);
    assert_eq!(bind("A", "YZ450F").sectors[0], 20_000);
    assert_eq!(bind("A", "YZ250F").lap_ms, 90_000);
    assert_eq!(bind("A", "YZ250F").sectors[0], 30_000);
    let on_disk = fs::read_to_string(dir.join("A.json")).unwrap();
    assert!(on_disk.contains("\"v\":2"));
    assert!(on_disk.contains("\"450\""));
    assert!(on_disk.contains("\"250\""));
    assert!(!on_disk.contains("YZ450F"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn same_class_bikes_share_a_tape() {
    let _lock = exclusive_test();
    let dir = tmp_dir();
    set_store_dir(dir.clone());
    let mut a = [0; BINS];
    a[1] = 1;
    assert!(commit_tape("A", "YZ250F", 90_000, a));
    assert!(commit_sector("A", "YZ250F", 0, 30_000));
    assert_eq!(bind("A", "CRF250R").lap_ms, 90_000);
    assert_eq!(bind("A", "CRF250R").sectors[0], 30_000);
    assert_eq!(bind("A", "FC 250").lap_ms, 90_000);
    assert_eq!(bind("A", "YZ450F").lap_ms, 0);
    let mut faster = [0; BINS];
    faster[1] = 2;
    assert!(commit_tape("A", "CRF250R", 85_000, faster));
    assert_eq!(bind("A", "YZ250F").lap_ms, 85_000);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn v1_file_adopts_into_first_named_bike() {
    let _lock = exclusive_test();
    let dir = tmp_dir();
    let _ = fs::create_dir_all(&dir);
    let text = format!(
        "{{\"v\":1,\"ms\":80000,\"bins\":{},\"s\":[1,2,3],\"p\":[333,671],\"used\":1}}",
        bins_json()
    );
    fs::write(dir.join("A.json"), text).unwrap();
    set_store_dir(dir.clone());
    let four = bind("A", "YZ450F");
    assert_eq!(four.lap_ms, 80_000);
    assert_eq!(four.sectors, [1, 2, 3]);
    assert_eq!(four.split_milli, [333, 671]);
    let two = bind("A", "YZ250F");
    assert_eq!(two.lap_ms, 0);
    assert_eq!(two.sectors, [0, 0, 0]);
    assert_eq!(two.split_milli, [333, 671]);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn split_pos_is_shared_across_bikes() {
    let _lock = exclusive_test();
    let dir = tmp_dir();
    set_store_dir(dir.clone());
    assert!(note_split_pos("A", "YZ450F", 0, 0.33));
    assert_eq!(bind("A", "YZ250F").split_milli[0], 330);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn v2_round_trip_keeps_both_bikes() {
    let _lock = exclusive_test();
    let dir = tmp_dir();
    set_store_dir(dir.clone());
    let mut a = [0; BINS];
    a[0] = 10;
    a[255] = 60_000;
    commit_tape("A", "YZ450F", 60_000, a);
    let mut b = [0; BINS];
    b[0] = 12;
    b[255] = 90_000;
    commit_tape("A", "YZ250F", 90_000, b);
    set_store_dir(dir.clone());
    assert_eq!(bind("A", "YZ450F").lap_ms, 60_000);
    assert_eq!(bind("A", "YZ250F").lap_ms, 90_000);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn model_name_keys_fold_into_class() {
    let _lock = exclusive_test();
    let dir = tmp_dir();
    set_store_dir(dir.clone());
    let mut a = [0; BINS];
    a[0] = 10;
    a[255] = 90_000;
    commit_tape("A", "YZ250F", 90_000, a);
    let mut raw = fs::read_to_string(dir.join("A.json")).unwrap();
    raw = raw.replace("\"250\":", "\"YZ250F\":");
    fs::write(dir.join("A.json"), raw).unwrap();
    set_store_dir(dir.clone());
    assert_eq!(bind("A", "CRF250R").lap_ms, 90_000);
    let on_disk = fs::read_to_string(dir.join("A.json")).unwrap();
    assert!(on_disk.contains("\"250\""));
    assert!(!on_disk.contains("YZ250F"));
    let _ = fs::remove_dir_all(dir);
}
