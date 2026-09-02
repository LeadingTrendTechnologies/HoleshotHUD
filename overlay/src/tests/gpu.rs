use super::{gpu_engine_pct, gpu_pid_pcts};

#[test]
fn pidless_3d_is_the_engine_total() {
    let samples = [
        (
            "luid_0x00000000_0x00017A4B_phys_0_eng_0_engtype_3D",
            41.0,
        ),
        (
            "luid_0x00000000_0x00017A4B_phys_0_eng_1_engtype_Copy",
            90.0,
        ),
    ];
    assert_eq!(gpu_engine_pct(samples), 41.0);
}

#[test]
fn per_process_3d_sums_on_the_same_engine() {
    let samples = [
        (
            "pid_100_luid_0x00000000_0x00017A4B_phys_0_eng_0_engtype_3D",
            30.0,
        ),
        (
            "pid_200_luid_0x00000000_0x00017A4B_phys_0_eng_0_engtype_3D",
            25.0,
        ),
        (
            "pid_200_luid_0x00000000_0x00017A4B_phys_0_eng_1_engtype_Copy",
            80.0,
        ),
    ];
    assert_eq!(gpu_engine_pct(samples), 55.0);
}

#[test]
fn pidless_wins_over_pid_sum_on_the_same_engine() {
    let samples = [
        (
            "luid_0x00000000_0x00017A4B_phys_0_eng_0_engtype_3D",
            40.0,
        ),
        (
            "pid_100_luid_0x00000000_0x00017A4B_phys_0_eng_0_engtype_3D",
            30.0,
        ),
    ];
    assert_eq!(gpu_engine_pct(samples), 40.0);
}

#[test]
fn hottest_card_wins() {
    let samples = [
        (
            "pid_1_luid_0x00000000_0x00000001_phys_0_eng_0_engtype_3D",
            12.0,
        ),
        (
            "pid_2_luid_0x00000000_0x00000002_phys_0_eng_0_engtype_Compute",
            77.0,
        ),
    ];
    assert_eq!(gpu_engine_pct(samples), 77.0);
}

#[test]
fn per_process_sum_caps_at_100() {
    let samples = [
        (
            "pid_1_luid_0x00000000_0x00017A4B_phys_0_eng_0_engtype_3D",
            70.0,
        ),
        (
            "pid_2_luid_0x00000000_0x00017A4B_phys_0_eng_0_engtype_3D",
            50.0,
        ),
    ];
    assert_eq!(gpu_engine_pct(samples), 100.0);
}

#[test]
fn compute_does_not_beat_3d_on_the_same_card() {
    let samples = [
        (
            "luid_0x00000000_0x00017A4B_phys_0_eng_0_engtype_3D",
            76.0,
        ),
        (
            "luid_0x00000000_0x00017A4B_phys_0_eng_1_engtype_Compute",
            100.0,
        ),
    ];
    assert_eq!(gpu_engine_pct(samples), 76.0);
}

#[test]
fn compute_used_when_the_card_has_no_3d() {
    let samples = [(
        "luid_0x00000000_0x00017A4B_phys_0_eng_1_engtype_Compute",
        77.0,
    )];
    assert_eq!(gpu_engine_pct(samples), 77.0);
}

#[test]
fn pid_pcts_take_the_hottest_work_engine() {
    let samples = [
        (
            "pid_100_luid_0x00000000_0x00017A4B_phys_0_eng_0_engtype_3D",
            30.0,
        ),
        (
            "pid_100_luid_0x00000000_0x00017A4B_phys_0_eng_1_engtype_Compute",
            12.0,
        ),
        (
            "pid_200_luid_0x00000000_0x00017A4B_phys_0_eng_0_engtype_3D",
            25.0,
        ),
        (
            "pid_200_luid_0x00000000_0x00017A4B_phys_0_eng_2_engtype_Copy",
            80.0,
        ),
        (
            "luid_0x00000000_0x00017A4B_phys_0_eng_0_engtype_3D",
            55.0,
        ),
    ];
    let by_pid = gpu_pid_pcts(samples);
    assert_eq!(by_pid.get(&100).copied(), Some(30.0));
    assert_eq!(by_pid.get(&200).copied(), Some(25.0));
    assert!(!by_pid.contains_key(&0));
}

#[test]
fn pid_pcts_prefer_3d_over_stuck_compute() {
    let samples = [
        (
            "pid_100_luid_0x00000000_0x00017A4B_phys_0_eng_0_engtype_3D",
            40.0,
        ),
        (
            "pid_100_luid_0x00000000_0x00017A4B_phys_0_eng_1_engtype_Compute",
            100.0,
        ),
    ];
    let by_pid = gpu_pid_pcts(samples);
    assert_eq!(by_pid.get(&100).copied(), Some(40.0));
}
