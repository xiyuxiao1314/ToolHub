//! Signed, bounded updates for the whole ToolHub distribution. No Agent-facing authority.
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
pub type Result<T> = std::result::Result<T, String>;
pub const REPOSITORY: &str = "xiyuxiao1314/ToolHub";
pub const PUBLIC_KEY: &str = include_str!("../../../resources/updater-public-key.hex");
pub const MAX_ARCHIVE: u64 = 256 * 1024 * 1024;
const MAX_EXPANDED: u64 = 1024 * 1024 * 1024;
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: String,
    pub version: String,
    pub min_current: String,
    pub platform: String,
    pub url: String,
    pub size: u64,
    pub sha256: String,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct Offer {
    pub manifest: Manifest,
    pub signed_json: String,
    pub signature: String,
    pub notes: String,
    pub release_url: String,
}
pub fn platform() -> Result<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => Ok("windows-x64"),
        ("macos", "aarch64") => Ok("macos-arm64"),
        ("macos", "x86_64") => Ok("macos-x64"),
        _ => Err("该平台暂不支持应用内更新".into()),
    }
}
pub fn package_name(version: &str, platform: &str) -> String {
    format!("ToolHub-v{version}-{platform}.zip")
}
pub fn package_url(version: &str, platform: &str) -> String {
    format!(
        "https://github.com/{REPOSITORY}/releases/download/v{version}/{}",
        package_name(version, platform)
    )
}
pub fn metadata_name(platform: &str) -> String {
    format!("ToolHub-update-{platform}.json")
}
pub fn verify_with_key(
    raw: &[u8],
    signature: &str,
    current: &str,
    expected_platform: &str,
    key: &[u8; 32],
) -> Result<Manifest> {
    if raw.len() > 16384 || signature.len() > 256 {
        return Err("更新元数据超出大小限制".into());
    }
    let bytes = hex::decode(signature.trim()).map_err(|_| "更新签名格式不正确")?;
    let sig = Signature::from_slice(&bytes).map_err(|_| "更新签名长度不正确")?;
    VerifyingKey::from_bytes(key)
        .map_err(|_| "更新公钥无效")?
        .verify_strict(raw, &sig)
        .map_err(|_| "更新签名校验失败，未安装")?;
    let m: Manifest = serde_json::from_slice(raw).map_err(|_| "更新元数据格式不正确")?;
    let next = semver::Version::parse(&m.version).map_err(|_| "更新版本格式不正确")?;
    let old = semver::Version::parse(current).map_err(|_| "当前版本格式不正确")?;
    let minimum = semver::Version::parse(&m.min_current).map_err(|_| "更新兼容版本格式不正确")?;
    if m.schema != "toolhub.desktop-update/v1"
        || m.platform != expected_platform
        || !["windows-x64", "macos-arm64", "macos-x64"].contains(&m.platform.as_str())
        || next <= old
        || old < minimum
        || next < minimum
        || m.url != package_url(&m.version, &m.platform)
        || m.size == 0
        || m.size > MAX_ARCHIVE
        || m.sha256.len() != 64
        || !m.sha256.bytes().all(|c| c.is_ascii_hexdigit())
    {
        return Err("更新来源、版本或平台不匹配，未安装".into());
    }
    Ok(m)
}
pub fn verify(raw: &[u8], signature: &str, current: &str, target: &str) -> Result<Manifest> {
    let bytes = hex::decode(PUBLIC_KEY.trim()).map_err(|_| "应用更新公钥不可用")?;
    let key: [u8; 32] = bytes.try_into().map_err(|_| "应用更新公钥长度错误")?;
    verify_with_key(raw, signature, current, target, &key)
}
fn allowed_url(url: &reqwest::Url) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port_or_known_default() == Some(443)
        && matches!(
            url.host_str(),
            Some(
                "github.com"
                    | "api.github.com"
                    | "release-assets.githubusercontent.com"
                    | "objects.githubusercontent.com"
            )
        )
}
fn client(direct: bool, timeout: u64) -> Result<reqwest::blocking::Client> {
    let mut b = reqwest::blocking::Client::builder()
        .user_agent(concat!("ToolHub-Updater/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(timeout))
        .redirect(reqwest::redirect::Policy::custom(|a| {
            if a.previous().len() < 6 && allowed_url(a.url()) {
                a.follow()
            } else {
                a.error("更新下载重定向被拒绝")
            }
        }));
    if direct {
        b = b.no_proxy();
    }
    b.build().map_err(|_| "无法建立更新连接".into())
}
fn get(url: &str, cap: usize) -> Result<Vec<u8>> {
    for direct in [false, true] {
        let attempt = (|| {
            let response = client(direct, 15)?
                .get(url)
                .send()
                .map_err(|_| "无法连接 GitHub，请稍后重试")?;
            if !response.status().is_success() {
                return Err(format!(
                    "GitHub 返回 {}，请稍后重试",
                    response.status().as_u16()
                ));
            }
            let mut data = Vec::new();
            response
                .take(cap as u64 + 1)
                .read_to_end(&mut data)
                .map_err(|_| "更新信息下载中断")?;
            if data.len() > cap {
                return Err("更新信息超过大小限制".into());
            }
            Ok(data)
        })();
        if attempt.is_ok() || direct {
            return attempt;
        }
    }
    unreachable!()
}
#[derive(Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    body: Option<String>,
    assets: Vec<ReleaseAsset>,
}
#[derive(Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
}
pub fn check_latest(current: &str, preview: bool) -> Result<Option<Offer>> {
    let target = platform()?;
    let bytes = get(
        &format!("https://api.github.com/repos/{REPOSITORY}/releases?per_page=30"),
        2 * 1024 * 1024,
    )?;
    let releases: Vec<Release> =
        serde_json::from_slice(&bytes).map_err(|_| "GitHub 版本列表格式不正确")?;
    let old = semver::Version::parse(current).map_err(|_| "当前版本格式不正确")?;
    let mut candidates: Vec<_> = releases
        .into_iter()
        .filter_map(|r| {
            let version = semver::Version::parse(r.tag_name.strip_prefix('v')?).ok()?;
            (!r.draft && (preview || (!r.prerelease && version.pre.is_empty())) && version > old)
                .then_some((version, r))
        })
        .collect();
    candidates.sort_by(|a, b| b.0.cmp(&a.0));
    let Some((version, release)) = candidates.into_iter().next() else {
        return Ok(None);
    };
    let name = metadata_name(target);
    let signature_name = format!("{name}.sig");
    let expected = format!("https://github.com/{REPOSITORY}/releases/download/v{version}/{name}");
    let metadata = release
        .assets
        .iter()
        .find(|a| a.name == name)
        .ok_or("新版本尚未提供完整的应用内更新包，请稍后检查")?;
    let signature = release
        .assets
        .iter()
        .find(|a| a.name == signature_name)
        .ok_or("新版本尚未提供更新签名，请稍后检查")?;
    if metadata.browser_download_url != expected
        || signature.browser_download_url != format!("{expected}.sig")
    {
        return Err("更新地址不匹配".into());
    }
    let raw = get(&expected, 16384)?;
    let sig = String::from_utf8(get(&format!("{expected}.sig"), 256)?)
        .map_err(|_| "更新签名不是有效文本")?;
    let manifest = verify(&raw, &sig, current, target)?;
    if manifest.version != version.to_string() {
        return Err("更新标签与签名版本不一致".into());
    }
    let notes = release
        .body
        .unwrap_or_default()
        .chars()
        .take(12000)
        .collect();
    Ok(Some(Offer {
        manifest,
        signed_json: String::from_utf8(raw).map_err(|_| "更新元数据不是 UTF-8")?,
        signature: sig,
        notes,
        release_url: format!("https://github.com/{REPOSITORY}/releases/tag/v{version}"),
    }))
}
pub fn hash_file(path: &Path) -> Result<String> {
    let mut f = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut b = [0u8; 65536];
    loop {
        let n = f.read(&mut b).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&b[..n]);
    }
    Ok(hex::encode(hash.finalize()))
}
pub fn download(
    m: &Manifest,
    target: &Path,
    cancel: &AtomicBool,
    progress: impl Fn(u64),
) -> Result<()> {
    use std::{
        io::{Seek, SeekFrom},
        sync::{atomic::AtomicUsize, mpsc},
    };
    if m.url != package_url(&m.version, &m.platform) || m.size == 0 || m.size > MAX_ARCHIVE {
        return Err("更新下载描述不正确".into());
    }
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(target)
            .map_err(|e| e.to_string())?;
        let next = AtomicUsize::new(0);
        let failed = AtomicBool::new(false);
        let block = 256 * 1024u64;
        let count = m.size.div_ceil(block) as usize;
        let (tx, rx) = mpsc::sync_channel(4);
        let started = std::time::Instant::now();
        let transfer = std::thread::scope(|scope| {
            for _ in 0..4 {
                let tx = tx.clone();
                let next = &next;
                let failed = &failed;
                scope.spawn(move || {
                    let clients = (client(false, 20), client(true, 20));
                    loop {
                        if cancel.load(Ordering::SeqCst) || failed.load(Ordering::SeqCst) {
                            break;
                        }
                        let index = next.fetch_add(1, Ordering::SeqCst);
                        if index >= count {
                            break;
                        }
                        let first = index as u64 * block;
                        let last = (first + block - 1).min(m.size - 1);
                        let result = (|| {
                            let mut last_error = "无法下载更新，请检查网络后重试".to_string();
                            for candidate in [&clients.0, &clients.1] {
                                if cancel.load(Ordering::SeqCst) {
                                    return Err("已取消下载".to_string());
                                }
                                let client = match candidate {
                                    Ok(v) => v,
                                    Err(e) => {
                                        last_error = e.clone();
                                        continue;
                                    }
                                };
                                let response = match client
                                    .get(&m.url)
                                    .header(reqwest::header::RANGE, format!("bytes={first}-{last}"))
                                    .send()
                                {
                                    Ok(v) => v,
                                    Err(_) => continue,
                                };
                                if response.status().as_u16() != 206
                                    || response
                                        .headers()
                                        .get(reqwest::header::CONTENT_RANGE)
                                        .and_then(|v| v.to_str().ok())
                                        != Some(format!("bytes {first}-{last}/{}", m.size).as_str())
                                {
                                    last_error = format!(
                                        "更新服务器返回 {}，请稍后重试",
                                        response.status().as_u16()
                                    );
                                    continue;
                                }
                                let mut data = Vec::new();
                                if response
                                    .take(last - first + 2)
                                    .read_to_end(&mut data)
                                    .is_err()
                                {
                                    continue;
                                }
                                if data.len() as u64 != last - first + 1 {
                                    continue;
                                }
                                return Ok((first, data));
                            }
                            Err(last_error)
                        })();
                        if result.is_err() {
                            failed.store(true, Ordering::SeqCst);
                        }
                        if tx.send(result).is_err() {
                            break;
                        }
                    }
                });
            }
            drop(tx);
            let mut received = 0u64;
            let mut error = None;
            for result in rx {
                if cancel.load(Ordering::SeqCst) {
                    failed.store(true, Ordering::SeqCst);
                    error = Some("已取消下载".into());
                }
                if started.elapsed() > Duration::from_secs(900) {
                    failed.store(true, Ordering::SeqCst);
                    error = Some("更新下载超时，可重试".into());
                }
                match result {
                    Ok((offset, data)) if error.is_none() => {
                        if file
                            .seek(SeekFrom::Start(offset))
                            .and_then(|_| file.write_all(&data))
                            .is_err()
                        {
                            failed.store(true, Ordering::SeqCst);
                            error = Some("无法写入更新，请检查磁盘空间".into());
                        } else {
                            received += data.len() as u64;
                            progress(received);
                        }
                    }
                    Err(e) => {
                        failed.store(true, Ordering::SeqCst);
                        error.get_or_insert(e);
                    }
                    _ => {}
                }
            }
            if let Some(error) = error {
                return Err(error);
            }
            if received != m.size || cancel.load(Ordering::SeqCst) {
                return Err("更新下载中断或已取消".into());
            }
            Ok(())
        });
        transfer?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        if hash_file(target)? != m.sha256.to_ascii_lowercase() {
            return Err("更新完整性校验失败，请重新下载".into());
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(target);
    }
    result
}

fn safe_relative(raw: &str) -> Result<PathBuf> {
    if raw.len() > 2048 || raw.contains(['\\', ':', '\0']) {
        return Err("更新包含无效路径".into());
    }
    let path = Path::new(raw);
    if !path.components().all(|c| matches!(c, Component::Normal(_)))
        || raw.split('/').any(|p| {
            let stem = p.split('.').next().unwrap_or("").to_ascii_lowercase();
            p.ends_with([' ', '.'])
                || p.chars().any(char::is_control)
                || matches!(
                    stem.as_str(),
                    "con"
                        | "prn"
                        | "aux"
                        | "nul"
                        | "com1"
                        | "com2"
                        | "com3"
                        | "com4"
                        | "com5"
                        | "com6"
                        | "com7"
                        | "com8"
                        | "com9"
                        | "lpt1"
                        | "lpt2"
                        | "lpt3"
                        | "lpt4"
                        | "lpt5"
                        | "lpt6"
                        | "lpt7"
                        | "lpt8"
                        | "lpt9"
                )
        })
    {
        return Err("更新包路径越界或名称不受支持".into());
    }
    Ok(path.to_path_buf())
}
pub fn extract_verified(archive: &Path, destination: &Path, m: &Manifest) -> Result<PathBuf> {
    let mut bytes = Vec::new();
    fs::File::open(archive)
        .map_err(|e| e.to_string())?
        .take(MAX_ARCHIVE + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 != m.size
        || bytes.len() as u64 > MAX_ARCHIVE
        || hex::encode(Sha256::digest(&bytes)) != m.sha256.to_ascii_lowercase()
    {
        return Err("安装前完整性复核失败".into());
    }
    fs::create_dir(destination).map_err(|e| e.to_string())?;
    let mut zip =
        zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|_| "更新 ZIP 无法读取")?;
    if zip.len() > 6000 {
        return Err("更新包文件数量超限".into());
    }
    let root_name = format!("ToolHub-{}-{}", m.version, m.platform);
    let mut seen = BTreeSet::new();
    let mut expanded = 0u64;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(|_| "更新 ZIP 条目无效")?;
        let name = entry.name().trim_end_matches('/').to_string();
        let relative = safe_relative(&name)?;
        if relative
            .components()
            .next()
            .and_then(|c| c.as_os_str().to_str())
            != Some(&root_name)
            || !seen.insert(name.to_lowercase())
        {
            return Err("更新包根目录或重复路径不正确".into());
        }
        let mode = entry.unix_mode().unwrap_or(0);
        let kind = mode & 0o170000;
        if !matches!(kind, 0 | 0o100000 | 0o040000) {
            return Err("更新包不能包含链接或特殊文件".into());
        }
        expanded = expanded.checked_add(entry.size()).ok_or("更新包大小溢出")?;
        if expanded > MAX_EXPANDED || entry.size() > MAX_ARCHIVE {
            return Err("更新包解压大小超限".into());
        }
        let output = destination.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&output).map_err(|e| e.to_string())?;
            continue;
        }
        fs::create_dir_all(output.parent().ok_or("更新目录不可用")?).map_err(|e| e.to_string())?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&output)
            .map_err(|e| e.to_string())?;
        let count = std::io::copy(&mut entry.by_ref().take(MAX_ARCHIVE + 1), &mut file)
            .map_err(|_| "更新包解压失败")?;
        if count != entry.size() {
            return Err("更新包条目大小不一致".into());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(
                &output,
                fs::Permissions::from_mode(if mode & 0o111 != 0 { 0o755 } else { 0o644 }),
            )
            .map_err(|e| e.to_string())?;
        }
    }
    let root = destination.join(root_name);
    let mac = m.platform.starts_with("macos-");
    let assets = if mac {
        root.join("ToolHub.app/Contents/Resources")
    } else {
        root.clone()
    };
    let bin = if mac {
        root.join("ToolHub.app/Contents/MacOS")
    } else {
        root.clone()
    };
    let metadata: serde_json::Value = serde_json::from_slice(
        &fs::read(assets.join("package.json")).map_err(|_| "更新包缺少版本信息")?,
    )
    .map_err(|_| "更新包版本信息无效")?;
    if metadata["version"] != m.version && metadata["version"] != format!("toolhub {}", m.version) {
        return Err("包内版本与更新签名不一致".into());
    }
    for name in ["toolhub", "toolhubd", "toolhub-desktop"] {
        let path = bin.join(format!("{name}{}", if mac { "" } else { ".exe" }));
        verify_binary(&path, &m.platform)?;
    }
    for name in [
        "schemas/extension.schema.json",
        "skills/toolhub-integration/SKILL.md",
        "extensions/official/catalog.json",
    ] {
        if !assets.join(name).is_file() {
            return Err(format!("更新包缺少资源：{name}"));
        }
    }
    Ok(root)
}
fn verify_binary(path: &Path, platform: &str) -> Result<()> {
    let file = fs::File::open(path).map_err(|_| "更新包缺少配对程序")?;
    let mut data = Vec::new();
    file.take(1024 * 1024)
        .read_to_end(&mut data)
        .map_err(|e| e.to_string())?;
    if platform == "windows-x64" {
        if data.len() < 64 || &data[..2] != b"MZ" {
            return Err("更新程序不是 Windows 可执行文件".into());
        }
        let offset = u32::from_le_bytes(data[60..64].try_into().unwrap()) as usize;
        if data.get(offset..offset + 6) != Some(b"PE\0\0\x64\x86".as_slice()) {
            return Err("更新程序处理器架构不匹配".into());
        }
    } else {
        if data.len() < 8
            || data[..4] != [0xcf, 0xfa, 0xed, 0xfe]
            || u32::from_le_bytes(data[4..8].try_into().unwrap())
                != if platform == "macos-arm64" {
                    0x100000c
                } else {
                    0x1000007
                }
        {
            return Err("更新程序处理器架构不匹配".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    fn sample() -> Manifest {
        Manifest {
            schema: "toolhub.desktop-update/v1".into(),
            version: "0.3.0-beta.15".into(),
            min_current: "0.3.0-beta.14".into(),
            platform: "windows-x64".into(),
            url: package_url("0.3.0-beta.15", "windows-x64"),
            size: 123,
            sha256: "ab".repeat(32),
        }
    }
    fn signed(m: &Manifest) -> (Vec<u8>, String, [u8; 32]) {
        let key = SigningKey::from_bytes(&[42; 32]);
        let raw = serde_json::to_vec(m).unwrap();
        let sig = hex::encode(key.sign(&raw).to_bytes());
        (raw, sig, key.verifying_key().to_bytes())
    }
    #[test]
    fn signed_update_rejects_tampering_downgrade_wrong_platform_and_external_urls() {
        let (raw, sig, key) = signed(&sample());
        assert!(verify_with_key(&raw, &sig, "0.3.0-beta.14", "windows-x64", &key).is_ok());
        let mut changed = raw.clone();
        changed[1] ^= 1;
        assert!(verify_with_key(&changed, &sig, "0.3.0-beta.14", "windows-x64", &key).is_err());
        assert!(verify_with_key(&raw, &sig, "0.3.0-beta.15", "windows-x64", &key).is_err());
        assert!(verify_with_key(&raw, &sig, "0.3.0-beta.14", "macos-x64", &key).is_err());
        for url in [
            "http://github.com/x",
            "https://example.com/a.zip",
            "https://github.com.evil.test/a",
        ] {
            let mut m = sample();
            m.url = url.into();
            let (r, s, k) = signed(&m);
            assert!(verify_with_key(&r, &s, "0.3.0-beta.14", "windows-x64", &k).is_err());
        }
    }
    #[test]
    fn archive_paths_and_special_names_cannot_escape() {
        for path in [
            "../evil",
            "/evil",
            "C:/evil",
            "root/../evil",
            "root/CON.exe",
            "root/a.",
            "root\\evil",
        ] {
            assert!(safe_relative(path).is_err(), "{path}")
        }
        assert!(safe_relative("root/中文 [1]/file.md").is_ok());
    }
    fn archive(extra: Option<(&str, bool)>) -> (tempfile::TempDir, PathBuf, Manifest) {
        use zip::write::SimpleFileOptions;
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("fixture.zip");
        let mut zip = zip::ZipWriter::new(fs::File::create(&path).unwrap());
        let mut m = sample();
        let root = format!("ToolHub-{}-{}", m.version, m.platform);
        let options = SimpleFileOptions::default();
        let mut binary = vec![0u8; 512];
        binary[..2].copy_from_slice(b"MZ");
        binary[60..64].copy_from_slice(&128u32.to_le_bytes());
        binary[128..134].copy_from_slice(b"PE\0\0\x64\x86");
        for name in ["toolhub.exe", "toolhubd.exe", "toolhub-desktop.exe"] {
            zip.start_file(format!("{root}/{name}"), options).unwrap();
            zip.write_all(&binary).unwrap();
        }
        zip.start_file(format!("{root}/package.json"), options)
            .unwrap();
        zip.write_all(
            serde_json::json!({"version":m.version})
                .to_string()
                .as_bytes(),
        )
        .unwrap();
        for name in [
            "schemas/extension.schema.json",
            "skills/toolhub-integration/SKILL.md",
            "extensions/official/catalog.json",
        ] {
            zip.start_file(format!("{root}/{name}"), options).unwrap();
            zip.write_all(b"fixture").unwrap();
        }
        if let Some((name, link)) = extra {
            if link {
                zip.add_symlink(format!("{root}/{name}"), "../outside", options)
                    .unwrap();
            } else {
                zip.start_file(format!("{root}/{name}"), options).unwrap();
                zip.write_all(b"malicious").unwrap();
            }
        }
        zip.finish().unwrap();
        m.size = fs::metadata(&path).unwrap().len();
        m.sha256 = hash_file(&path).unwrap();
        (d, path, m)
    }
    #[test]
    fn paired_bundle_extracts_and_even_signed_traversal_links_and_duplicates_are_rejected() {
        let (d, path, m) = archive(None);
        let out = extract_verified(&path, &d.path().join("output"), &m).unwrap();
        assert!(out.join("toolhubd.exe").exists());
        for extra in [
            ("../../outside", false),
            ("TOOLHUB.exe", false),
            ("shortcut", true),
        ] {
            let (d, path, m) = archive(Some(extra));
            assert!(extract_verified(&path, &d.path().join("output"), &m).is_err());
            assert!(!d.path().join("outside").exists());
        }
    }
    #[test]
    fn cancellation_prevents_network_and_leaves_no_download_file() {
        let d = tempfile::tempdir().unwrap();
        let file = d.path().join("cancel.zip");
        let cancelled = AtomicBool::new(true);
        assert!(download(&sample(), &file, &cancelled, |_| {}).is_err());
        assert!(!file.exists());
    }
    #[test]
    fn extraction_rechecks_bytes_before_creating_installation() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("archive.zip");
        fs::write(&path, b"corrupt").unwrap();
        assert!(extract_verified(&path, &d.path().join("output"), &sample()).is_err());
        assert!(!d.path().join("output").exists());
    }
}
