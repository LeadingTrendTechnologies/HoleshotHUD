use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let src = manifest.join("..").join("out").join("Release").join("mxbo.dlo");
    let dest = PathBuf::from(env::var("OUT_DIR").unwrap()).join("mxbo.dlo");
    if src.exists() {
        fs::copy(&src, &dest).expect("copy plugin into overlay build");
        println!("cargo:rerun-if-changed={}", src.display());
    } else {
        fs::write(&dest, []).expect("placeholder plugin");
    }
}
