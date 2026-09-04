#[cfg(windows)]
use std::env;
#[cfg(windows)]
use std::fs;
#[cfg(windows)]
use std::path::PathBuf;

fn main() {
    #[cfg(windows)]
    embed_icon_and_plugin();
}

#[cfg(windows)]
fn embed_icon_and_plugin() {
    embed_icon();
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let src = manifest.join("..").join("out").join("Release").join("Holeshot-HUD.dlo");
    let dest = PathBuf::from(env::var("OUT_DIR").unwrap()).join("Holeshot-HUD.dlo");
    if src.exists() {
        fs::copy(&src, &dest).expect("copy plugin into overlay build");
        println!("cargo:rerun-if-changed={}", src.display());
    } else {
        fs::write(&dest, []).expect("placeholder plugin");
    }
}

#[cfg(windows)]
fn embed_icon() {
    let icon = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("icon.ico");
    println!("cargo:rerun-if-changed={}", icon.display());
    if !icon.exists() {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon(icon.to_str().unwrap());
    res.set("ProductName", "Holeshot HUD");
    res.set("FileDescription", "Holeshot HUD");
    res.set("FileVersion", "0.6.0.0");
    res.set("ProductVersion", "0.6.0.0");
    let _ = res.compile();
}
