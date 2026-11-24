//! Sign a release zip with the update key, or create that key.
//!
//! `sign-update generate <directory>` writes `holeshot-update.pub` and
//! `holeshot-update.key`. The key file has no password.
//! `sign-update sign <secret-key> <zip>` writes `<zip>.minisig`.

use std::env;
use std::fs;
use std::io::Read;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_default();
    let result = match command.as_str() {
        "generate" => generate(args.next()),
        "sign" => sign(args.next(), args.next()),
        _ => Err("usage: sign-update generate <directory> | sign-update sign <secret-key> <zip>".into()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn generate(directory: Option<String>) -> Result<(), String> {
    let directory = directory.ok_or("generate needs an output directory")?;
    let pair = minisign::KeyPair::generate_encrypted_keypair(Some(String::new()))
        .map_err(|error| error.to_string())?;
    let public_box = pair.pk.to_box().map_err(|error| error.to_string())?;
    let secret_box = pair
        .sk
        .to_box(Some("Holeshot HUD update key"))
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let public_path = std::path::Path::new(&directory).join("holeshot-update.pub");
    let secret_path = std::path::Path::new(&directory).join("holeshot-update.key");
    fs::write(&public_path, public_box.into_string()).map_err(|error| error.to_string())?;
    fs::write(&secret_path, secret_box.into_string()).map_err(|error| error.to_string())?;
    println!("{}", pair.pk.to_base64());
    Ok(())
}

fn sign(secret: Option<String>, zip: Option<String>) -> Result<(), String> {
    let secret = secret.ok_or("sign needs the secret key path")?;
    let zip = zip.ok_or("sign needs the zip path")?;
    let secret_key = minisign::SecretKey::from_file(&secret, Some(String::new()))
        .map_err(|error| error.to_string())?;
    let public_key =
        minisign::PublicKey::from_secret_key(&secret_key).map_err(|error| error.to_string())?;
    let mut file = fs::File::open(&zip).map_err(|error| error.to_string())?;
    let signature = minisign::sign(Some(&public_key), &secret_key, &mut file, None, None)
        .map_err(|error| error.to_string())?;
    let mut signed = fs::File::open(&zip).map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    signed.read_to_end(&mut bytes).map_err(|error| error.to_string())?;
    if !crate_signature_ok(&public_key, &bytes, &signature) {
        return Err("Signed zip did not verify.".into());
    }
    let out = format!("{zip}.minisig");
    fs::write(&out, signature.into_string()).map_err(|error| error.to_string())?;
    Ok(())
}

fn crate_signature_ok(
    public_key: &minisign::PublicKey,
    bytes: &[u8],
    signature: &minisign::SignatureBox,
) -> bool {
    minisign::verify(
        public_key,
        signature,
        std::io::Cursor::new(bytes),
        true,
        false,
        false,
    )
    .is_ok()
}
