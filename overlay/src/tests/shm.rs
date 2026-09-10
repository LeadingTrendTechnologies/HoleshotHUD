use super::*;
use std::fmt::Write;
use std::mem::{offset_of, size_of};

fn abi_text() -> String {
    let mut o = mxbo_hud::shm::abi_text();
    if let Some(i) = o.find("\nVERSION=") {
        let rest = &o[i + 1..];
        if let Some(nl) = rest.find('\n') {
            let insert_at = i + 1 + nl + 1;
            o.insert_str(insert_at, &format!("CMD_MAGIC={CMD_MAGIC:#X}\n"));
        }
    }
    let _ = writeln!(o, "MxboShmCmd.size {}", size_of::<CmdView>());
    let _ = writeln!(o, "MxboShmCmd.magic {} {}", offset_of!(CmdView, magic), 4);
    let _ = writeln!(
        o,
        "MxboShmCmd.spectating {} {}",
        offset_of!(CmdView, spectating),
        4
    );
    let _ = writeln!(
        o,
        "MxboShmCmd.spectateRaceNum {} {}",
        offset_of!(CmdView, spectate_race_num),
        4
    );
    o
}

fn abi_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("src/shm/abi.txt")
}

#[test]
fn rust_shm_layout_matches_checked_in_abi() {
    let got = abi_text();
    let path = abi_path();
    if matches!(
        std::env::var("UPDATE_SHM_ABI").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE")
    ) {
        std::fs::write(&path, &got).expect("write src/shm/abi.txt");
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_default();
    assert_eq!(
        expected.replace("\r\n", "\n"),
        got,
        "Rust SHM layout drifted from src/shm/abi.txt. If the C header and Snapshot changed together, set UPDATE_SHM_ABI=1 and rebuild tools/shm-abi.cpp output in CI."
    );
}

#[test]
fn cmd_view_is_12_bytes() {
    assert_eq!(std::mem::size_of::<CmdView>(), 12);
    assert_eq!(CMD_MAGIC, 0x4342_584D);
}
