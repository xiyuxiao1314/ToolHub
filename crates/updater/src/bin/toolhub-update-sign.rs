//! Release-only signing helper. Seed is environment-only; verification needs no secret.
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let verifying = arguments.first().is_some_and(|v| v == "verify");
    if verifying {
        arguments.remove(0);
    }
    if arguments.len() != 2 {
        return Err("Usage: toolhub-update-sign [verify] MANIFEST SIGNATURE".into());
    }
    let raw = std::fs::read(&arguments[0])?;
    if raw.len() > 16384 {
        return Err("Manifest exceeds size bound".into());
    }
    let m: toolhub_updater::Manifest = serde_json::from_slice(&raw)?;
    if m.schema != "toolhub.desktop-update/v1"
        || semver::Version::parse(&m.version)? < semver::Version::parse(&m.min_current)?
        || m.url != toolhub_updater::package_url(&m.version, &m.platform)
        || m.size == 0
        || m.size > toolhub_updater::MAX_ARCHIVE
        || m.sha256.len() != 64
        || !m.sha256.bytes().all(|c| c.is_ascii_hexdigit())
        || !["windows-x64", "macos-arm64", "macos-x64"].contains(&m.platform.as_str())
    {
        return Err("Invalid update descriptor".into());
    }
    let public: [u8; 32] = hex::decode(toolhub_updater::PUBLIC_KEY.trim())?
        .try_into()
        .map_err(|_| "Invalid updater public key")?;
    if verifying {
        let signature = hex::decode(std::fs::read_to_string(&arguments[1])?.trim())?;
        VerifyingKey::from_bytes(&public)?
            .verify_strict(&raw, &Signature::from_slice(&signature)?)?;
        println!("Verified update metadata");
        return Ok(());
    }
    let key = std::env::var("TOOLHUB_UPDATE_SIGNING_KEY")
        .map_err(|_| "Release signing key is not configured")?;
    let bytes: [u8; 32] = hex::decode(key.trim())?
        .try_into()
        .map_err(|_| "Signing seed must contain 32 bytes")?;
    let signing = SigningKey::from_bytes(&bytes);
    if signing.verifying_key().to_bytes() != public {
        return Err("Signing key does not match embedded public key".into());
    }
    std::fs::write(
        &arguments[1],
        hex::encode(signing.sign(&raw).to_bytes()) + "\n",
    )?;
    println!("Signed update metadata");
    Ok(())
}
