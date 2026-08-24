#[path = "../shm.rs"]
mod shm;

use mxbo_hud::snapshot::cstr;
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    let shm = shm::Shm::open().unwrap_or_else(|| {
        eprintln!("MX Bikes is not publishing HUD data. Start the game on a track with mxbo.dlo loaded.");
        std::process::exit(1);
    });
    let snap = shm.read().unwrap_or_else(|| {
        eprintln!("Shared memory is open but the snapshot is stale or the wrong version. Restart MX Bikes after updating the plugin.");
        std::process::exit(1);
    });
    let n = snap.poly_count.clamp(0, shm::MAX_POLY as i32) as usize;
    if n < 8 {
        eprint!("{}", snap.dump_text());
        eprintln!("No track path yet ({n} points). Load a session on the track, then run this again.");
        std::process::exit(1);
    }
    let name = cstr(&snap.track_name);
    let name = if name.is_empty() { "Track".into() } else { name };
    let out = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("web-preview/src/demo_track.rs"));
    if let Some(dir) = out.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let mut body = String::new();
    body.push_str(&format!("pub const TRACK_NAME: &str = {:?};\n", name));
    body.push_str(&format!("pub const TRACK_LENGTH: f32 = {:?};\n", snap.track_length));
    body.push_str(&format!("pub const SF_METERS: f32 = {:?};\n", snap.sf_meters));
    body.push_str("pub const POLY: &[(f32, f32)] = &[\n");
    for p in snap.poly.iter().take(n) {
        body.push_str(&format!("    ({:?}, {:?}),\n", p.x, p.z));
    }
    body.push_str("];\n");
    let mut f = fs::File::create(&out).expect("write demo_track.rs");
    f.write_all(body.as_bytes()).expect("write demo_track.rs");
    println!("Wrote {n} points ({name}, {:.0} m) to {}", snap.track_length, out.display());
}
