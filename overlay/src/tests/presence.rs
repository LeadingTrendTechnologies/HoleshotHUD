use super::*;
use std::time::{Duration, Instant};

fn board(rows: &[(i32, &str)]) -> Vec<BoardRider> {
    rows.iter()
        .map(|(n, name)| BoardRider {
            race_num: *n,
            name: (*name).into(),
        })
        .collect()
}

#[test]
fn empty_guid_and_server_skips_publish() {
    assert_eq!(session_key("", "", "MD", &[1, 2]), None);
}

#[test]
fn guid_wins_over_fallback() {
    assert_eq!(
        session_key(" abc-guid ", "Server", "MD", &[7]).as_deref(),
        Some("abc-guid")
    );
}

#[test]
fn fallback_fingerprint_is_stable() {
    let a = session_key("", "Night Cup", "MD", &[12, 3, 3, 7]).unwrap();
    let b = session_key("", "Night Cup", "MD", &[7, 12, 3]).unwrap();
    let c = session_key("", "Other", "MD", &[12, 3, 7]).unwrap();
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert!(a.starts_with("fb:"));
}

#[test]
fn match_prefers_race_num_and_name() {
    let field = board(&[(7, "Troy"), (12, "Alex")]);
    let marks = match_room(
        &field,
        &[
            RemoteRider {
                race_num: 12,
                name: "alex".into(),
                steam_id: 0,
            },
            RemoteRider {
                race_num: 99,
                name: "Ghost".into(),
                steam_id: 0,
            },
        ],
    );
    assert_eq!(marks, vec![12]);
}

#[test]
fn match_unique_name_if_number_drifted() {
    let field = board(&[(7, "Troy"), (12, "Alex")]);
    let marks = match_room(
        &field,
        &[RemoteRider {
            race_num: 99,
            name: "Troy".into(),
            steam_id: 0,
        }],
    );
    assert_eq!(marks, vec![7]);
}

#[test]
fn match_ignores_duplicate_names_without_number() {
    let field = board(&[(7, "Sam"), (12, "Sam")]);
    let marks = match_room(
        &field,
        &[RemoteRider {
            race_num: 0,
            name: "Sam".into(),
            steam_id: 0,
        }],
    );
    assert!(marks.is_empty());
}

#[test]
fn warmup_publishes_immediately_then_waits() {
    let mut st = PulseState::default();
    let t0 = Instant::now();
    assert_eq!(
        next_pulse(&mut st, true, Some("g"), 5, 16, i32::MAX, t0),
        Pulse::Publish
    );
    assert_eq!(
        next_pulse(
            &mut st,
            true,
            Some("g"),
            5,
            16,
            i32::MAX,
            t0 + Duration::from_secs(30)
        ),
        Pulse::Silent
    );
    assert_eq!(
        next_pulse(
            &mut st,
            true,
            Some("g"),
            5,
            16,
            i32::MAX,
            t0 + Duration::from_secs(180)
        ),
        Pulse::Publish
    );
}

#[test]
fn gate_fires_once_at_two_seconds() {
    let mut st = PulseState::default();
    let t0 = Instant::now();
    assert_eq!(
        next_pulse(&mut st, true, Some("g"), 5, 16, i32::MAX, t0),
        Pulse::Publish
    );
    assert_eq!(
        next_pulse(&mut st, true, Some("g"), 7, 256, 50_000, t0),
        Pulse::Silent
    );
    assert_eq!(
        next_pulse(&mut st, true, Some("g"), 7, 256, 2_000, t0),
        Pulse::Publish
    );
    assert_eq!(
        next_pulse(&mut st, true, Some("g"), 7, 256, 1_000, t0),
        Pulse::Silent
    );
    assert_eq!(
        next_pulse(&mut st, true, Some("g"), 7, 16, 8 * 60_000, t0),
        Pulse::Silent
    );
}

#[test]
fn gate_fallback_when_two_seconds_skipped() {
    let mut st = PulseState::default();
    let t0 = Instant::now();
    next_pulse(&mut st, true, Some("g"), 5, 16, i32::MAX, t0);
    next_pulse(&mut st, true, Some("g"), 7, 256, 50_000, t0);
    assert_eq!(
        next_pulse(&mut st, true, Some("g"), 7, 16, 8 * 60_000, t0),
        Pulse::Publish
    );
}

#[test]
fn warmup_countdown_two_seconds_is_not_a_gate_ping() {
    let mut st = PulseState::default();
    let t0 = Instant::now();
    next_pulse(&mut st, true, Some("g"), 5, 16, i32::MAX, t0);
    assert_eq!(
        next_pulse(&mut st, true, Some("g"), 5, 16, 2_000, t0 + Duration::from_secs(1)),
        Pulse::Silent
    );
}

#[test]
fn disabled_or_empty_session_leaves() {
    let mut st = PulseState::default();
    let t0 = Instant::now();
    next_pulse(&mut st, true, Some("g"), 5, 16, i32::MAX, t0);
    assert_eq!(next_pulse(&mut st, false, Some("g"), 5, 16, i32::MAX, t0), Pulse::Leave);
    assert_eq!(next_pulse(&mut st, false, Some("g"), 5, 16, i32::MAX, t0), Pulse::Silent);
}

#[test]
fn rolling_join_stays_closed() {
    let mut st = PulseState::default();
    let t0 = Instant::now();
    assert_eq!(
        next_pulse(&mut st, true, Some("g"), 7, 16, 8 * 60_000, t0),
        Pulse::Silent
    );
}

#[test]
fn session_change_to_rolling_does_not_join() {
    let mut st = PulseState::default();
    let t0 = Instant::now();
    assert_eq!(
        next_pulse(&mut st, true, Some("a"), 5, 16, i32::MAX, t0),
        Pulse::Publish
    );
    assert_eq!(
        next_pulse(&mut st, true, Some("b"), 7, 16, 8 * 60_000, t0),
        Pulse::Silent
    );
}

#[test]
fn friend_mark_is_steam_id_only() {
    let field = board(&[(7, "Troy"), (12, "Alex")]);
    let remote = [
        RemoteRider {
            race_num: 12,
            name: "Alex".into(),
            steam_id: 76561198000000012,
        },
        RemoteRider {
            race_num: 7,
            name: "Troy".into(),
            steam_id: 0,
        },
    ];
    let marks = match_friends(&field, &remote, &[76561198000000012], 76561198000000001, 3);
    assert_eq!(marks, vec![12]);
}

#[test]
fn same_name_without_steam_id_is_not_a_friend() {
    let field = board(&[(7, "Troy"), (12, "Alex")]);
    let remote = [RemoteRider {
        race_num: 12,
        name: "Alex".into(),
        steam_id: 0,
    }];
    let marks = match_friends(&field, &remote, &[76561198000000012], 76561198000000001, 3);
    assert!(marks.is_empty());
}

#[test]
fn own_steam_id_is_not_a_friend() {
    let field = board(&[(3, "Me"), (12, "Alex")]);
    let remote = [RemoteRider {
        race_num: 12,
        name: "Alex".into(),
        steam_id: 76561198000000001,
    }];
    let marks = match_friends(&field, &remote, &[76561198000000001], 76561198000000001, 3);
    assert!(marks.is_empty());
}

#[test]
fn new_session_can_still_fire_gate() {
    let mut st = PulseState::default();
    let t0 = Instant::now();
    next_pulse(&mut st, true, Some("a"), 5, 16, i32::MAX, t0);
    next_pulse(&mut st, true, Some("b"), 7, 16, 8 * 60_000, t0);
    assert_eq!(
        next_pulse(&mut st, true, Some("b"), 7, 256, 2_000, t0),
        Pulse::Publish
    );
}
