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

const SECRET_COMMENT: &str = "untrusted comment: Holeshot HUD update key";

fn sign(secret: Option<String>, zip: Option<String>) -> Result<(), String> {
    let secret = secret.ok_or("sign needs the secret key path")?;
    let zip = zip.ok_or("sign needs the zip path")?;
    let secret_key = load_secret_key(&secret)?;
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

/// Windows keeps a GitHub secret on one line, so the comment and the key can arrive joined.
fn normalize_secret_key(raw: &str) -> Result<String, String> {
    let lines: Vec<&str> = raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    if lines.len() >= 2 {
        return Ok(format!("{}\n{}\n", lines[0], lines[1]));
    }
    let line = lines.first().copied().unwrap_or("").trim();
    let key = line
        .strip_prefix(SECRET_COMMENT)
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .unwrap_or(line);
    if key.is_empty() || key.contains(' ') {
        return Err("Update secret key is not a minisign secret key.".into());
    }
    Ok(format!("{SECRET_COMMENT}\n{key}\n"))
}

fn load_secret_key(path: &str) -> Result<minisign::SecretKey, String> {
    let raw = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let boxed = minisign::SecretKeyBox::from_string(&normalize_secret_key(&raw)?)
        .map_err(|error| error.to_string())?;
    minisign::SecretKey::from_box(boxed, Some(String::new())).map_err(|error| error.to_string())
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

#[cfg(test)]
mod tests {
    use super::normalize_secret_key;

    #[test]
    fn collapsed_secret_key_becomes_two_lines() {
        let collapsed = "untrusted comment: Holeshot HUD update keyQUJDRA==";
        let got = normalize_secret_key(collapsed).unwrap();
        assert_eq!(
            got,
            "untrusted comment: Holeshot HUD update key\nQUJDRA==\n"
        );
    }
}
