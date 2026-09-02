use super::pick_remote;

fn ip(a: u8, b: u8, c: u8, d: u8) -> u32 {
    u32::from_ne_bytes([a, b, c, d])
}

#[test]
fn public_beats_lan() {
    let lan = ip(192, 168, 1, 20);
    let pub_ip = ip(1, 1, 1, 1);
    assert_eq!(pick_remote(&[lan, pub_ip]), Some(pub_ip));
}

#[test]
fn lan_when_no_public() {
    let lan = ip(10, 0, 0, 5);
    assert_eq!(pick_remote(&[ip(127, 0, 0, 1), lan]), Some(lan));
}

#[test]
fn skips_loopback_and_empty() {
    assert_eq!(pick_remote(&[ip(127, 0, 0, 1), 0]), None);
    assert_eq!(pick_remote(&[]), None);
}
