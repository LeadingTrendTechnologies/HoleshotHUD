#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
include!("win_app.rs");

fn main() {
    #[cfg(windows)]
    win_run();
    #[cfg(not(windows))]
    {
        eprintln!("Holeshot HUD requires Windows (MX Bikes plugin + overlay).");
        eprintln!("On macOS/Linux, run tests with:");
        eprintln!("  cargo test --manifest-path overlay/Cargo.toml --workspace");
        std::process::exit(1);
    }
}
