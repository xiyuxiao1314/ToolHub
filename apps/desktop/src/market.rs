//! Native capability catalog: content-addressed snapshots, explicit import/add/export.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};
use toolhub_skills::bundle::{SkillBundle, MAX_BUNDLE_BYTES};
type CatalogEntries = Vec<(SkillBundle, String)>;
type Pending = BTreeMap<String, (Instant, SkillBundle)>;
static PENDING: OnceLock<Mutex<Pending>> = OnceLock::new();
fn digest(bundle: &SkillBundle) -> Result<String, String> {
    Ok(hex::encode(Sha256::digest(bundle.bytes()?)))
}
fn read_bundle(path: &Path) -> Result<SkillBundle, String> {
    if linked(path)? {
        return Err("能力包不能是链接或重解析点".into());
    }
    if !path.is_file() {
        return Err("能力包需要普通文件".into());
    }
    let file = std::fs::File::open(path).map_err(|_| "能力包文件不可读取")?;
    let mut data = vec![];
    file.take((MAX_BUNDLE_BYTES + 1) as u64)
        .read_to_end(&mut data)
        .map_err(|_| "能力包读取失败")?;
    SkillBundle::parse(&data)
}
fn linked(path: &Path) -> Result<bool, String> {
    let meta = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        Ok(meta.file_attributes() & 0x400 != 0)
    }
    #[cfg(not(windows))]
    {
        Ok(meta.file_type().is_symlink())
    }
}
fn bundled_root() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let packaged = crate::installation::asset_root(&exe)?.join("skills/market");
    if packaged.is_dir() {
        Ok(packaged)
    } else {
        Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../skills/market"))
    }
}
fn cache_root() -> Result<PathBuf, String> {
    let registry = std::env::var_os("TOOLHUB_REGISTRY")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".toolhub/registry.sqlite")
        });
    Ok(registry
        .parent()
        .ok_or("用户数据目录不可用")?
        .join("market/packages"))
}
fn inventory() -> Result<(CatalogEntries, Vec<String>), String> {
    let mut bundles = BTreeMap::new();
    let mut warnings = vec![];
    let root = bundled_root()?;
    let files = std::fs::read_dir(&root).map_err(|_| "安装目录缺少能力包目录")?;
    for file in files
        .flatten()
        .take(128)
        .filter(|f| f.path().extension().is_some_and(|e| e == "json"))
    {
        match read_bundle(&file.path()) {
            Ok(bundle) => {
                bundles.insert(digest(&bundle)?, (bundle, "builtin".to_string()));
            }
            Err(_) => warnings.push(format!(
                "内置包 {} 无法读取或格式有误",
                file.file_name().to_string_lossy()
            )),
        }
    }
    let cache = cache_root()?;
    if cache.exists() {
        if linked(&cache)? {
            return Err("能力包缓存不能是链接".into());
        }
        for dir in std::fs::read_dir(&cache)
            .map_err(|e| e.to_string())?
            .flatten()
            .take(128)
        {
            let name = dir.file_name().to_string_lossy().to_string();
            if name.len() != 64 || !name.bytes().all(|b| b.is_ascii_hexdigit()) {
                continue;
            }
            let candidate = if linked(&dir.path())? {
                Err("linked package".into())
            } else {
                read_bundle(&dir.path().join("package.bundle.json"))
            };
            match candidate {
                Ok(bundle) if digest(&bundle)? == name => {
                    bundles.entry(name).or_insert((bundle, "local".to_string()));
                }
                _ => warnings.push("一个本地能力包内容已改变或不可读取，请重新导入".into()),
            }
        }
    }
    if bundles.len() > 64 {
        warnings.push("目录超过 64 个能力包，本次显示前 64 个".into());
    }
    Ok((bundles.into_values().take(64).collect(), warnings))
}
fn selected(key: &str) -> Result<(SkillBundle, String), String> {
    if let Some(key) = key.strip_prefix("preview-") {
        let pending = PENDING
            .get_or_init(|| Mutex::new(BTreeMap::new()))
            .lock()
            .map_err(|_| "预览状态不可用")?;
        if let Some((at, bundle)) = pending
            .get(key)
            .filter(|(at, _)| at.elapsed() < Duration::from_secs(900))
        {
            let _ = at;
            return Ok((bundle.clone(), "preview".into()));
        }
        return Err("导入预览已过期，请重新选择文件".into());
    }
    inventory()?
        .0
        .into_iter()
        .find(|(bundle, _)| digest(bundle).as_deref() == Ok(key))
        .ok_or_else(|| "能力包已改变，请刷新目录并重新预览".into())
}
fn view(bundle: &SkillBundle, source: &str) -> Result<Value, String> {
    let hash = digest(bundle)?;
    Ok(
        json!({"key":if source=="preview"{format!("preview-{hash}")}else{hash.clone()},"digest":hash,"bundle":bundle,"source":source,"compatibility":bundle.manifest.library_compatibility(std::env::consts::OS,Some(&bundle.instructions))}),
    )
}
#[tauri::command]
pub async fn market_list() -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let (bundles, warnings) = inventory()?;
        let entries = bundles
            .iter()
            .map(|(b, s)| view(b, s))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(json!({"entries":entries,"warnings":warnings,"mode":"local_catalog"}))
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn market_pick(window: tauri::WebviewWindow) -> Result<Option<Value>, String> {
    let Some(path) = crate::program_picker::pick_bundle_file(window).await? else {
        return Ok(None);
    };
    let bundle = read_bundle(Path::new(&path))?;
    let hash = digest(&bundle)?;
    let result = view(&bundle, "preview")?;
    let mut pending = PENDING
        .get_or_init(|| Mutex::new(BTreeMap::new()))
        .lock()
        .map_err(|_| "预览状态不可用")?;
    pending.retain(|_, (at, _)| at.elapsed() < Duration::from_secs(900));
    if pending.len() >= 16 && !pending.contains_key(&hash) {
        return Err("预览数量已达上限，请完成现有导入或稍后重试".into());
    }
    pending.insert(hash, (Instant::now(), bundle));
    Ok(Some(result))
}
#[tauri::command]
pub async fn market_inspect(key: String) -> Result<Value, String> {
    let (bundle, source) = selected(&key)?;
    let mut result = view(&bundle, &source)?;
    result["dependencies"] = crate::rpc(
        "skill.preview".into(),
        json!({"manifest":bundle.manifest,"instructions":bundle.instructions}),
    )
    .await?;
    result["collection"] = collection(&bundle).await?;
    Ok(result)
}
async fn collection(bundle: &SkillBundle) -> Result<Value, String> {
    let rows = crate::rpc("skill.list".into(), json!({})).await?;
    if !rows.as_array().is_some_and(|rows| {
        rows.iter()
            .any(|row| row["id"] == bundle.manifest.id.as_str())
    }) {
        return Ok(json!({"status":"not_added"}));
    }
    let existing = crate::rpc("skill.inspect".into(), json!({"id":bundle.manifest.id})).await?;
    let manifest = toolhub_core::SkillManifest::validate_json(&existing["manifest"])
        .map_err(|e| e.to_string())?;
    let same = manifest == bundle.manifest
        && existing["instructions"].as_str() == Some(bundle.instructions.as_str());
    Ok(json!({"status":if same{"added"}else{"conflict"}}))
}
fn write_new(path: &Path, data: &[u8]) -> Result<(), String> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(data).map_err(|e| e.to_string())
}
fn snapshot(bundle: &SkillBundle, cache: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(cache).map_err(|e| e.to_string())?;
    if linked(cache)? {
        return Err("能力包缓存不能是链接".into());
    }
    let hash = digest(bundle)?;
    let dest = cache.join(&hash);
    if dest.exists() {
        if linked(&dest)? || digest(&read_bundle(&dest.join("package.bundle.json"))?)? != hash {
            return Err("已保存能力包内容有误，请核对本地缓存".into());
        }
        // Recheck the actual assets that daemon registration will read.
        let manifest = read_limited(&dest.join("skill.json"), MAX_BUNDLE_BYTES)?;
        let parsed = toolhub_core::SkillManifest::validate_json(
            &serde_json::from_slice(&manifest).map_err(|_| "能力包声明有误")?,
        )
        .map_err(|e| e.to_string())?;
        if parsed != bundle.manifest
            || read_limited(&dest.join("SKILL.md"), 65536)? != bundle.instructions.as_bytes()
        {
            return Err("能力包文件已改变，请核对后重新导入".into());
        }
        return Ok(dest.join("skill.json"));
    }
    let stage = cache.join(format!(".staging-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&stage).map_err(|e| e.to_string())?;
    write_new(&stage.join("package.bundle.json"), &bundle.bytes()?)?;
    write_new(
        &stage.join("skill.json"),
        &serde_json::to_vec_pretty(&bundle.manifest).map_err(|e| e.to_string())?,
    )?;
    write_new(&stage.join("SKILL.md"), bundle.instructions.as_bytes())?;
    std::fs::rename(&stage, &dest).map_err(|e| e.to_string())?;
    Ok(dest.join("skill.json"))
}
fn read_limited(path: &Path, max: usize) -> Result<Vec<u8>, String> {
    if linked(path)? {
        return Err("能力包不能引用链接".into());
    }
    if !path.is_file() {
        return Err("能力包需要普通文件".into());
    }
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut data = vec![];
    file.take((max + 1) as u64)
        .read_to_end(&mut data)
        .map_err(|e| e.to_string())?;
    if data.len() > max {
        return Err("能力包文件超过大小限制".into());
    }
    Ok(data)
}
#[tauri::command]
pub async fn market_add(key: String) -> Result<Value, String> {
    let (bundle, _) = selected(&key)?;
    if !bundle
        .manifest
        .library_compatibility(std::env::consts::OS, Some(&bundle.instructions))
        .reusable
    {
        return Err("此能力包不适用于当前平台".into());
    }
    let status = collection(&bundle).await?;
    if status["status"] == "conflict" {
        return Err("Skill 库已有相同 ID 的不同内容，市场不会覆盖；请使用新的 Skill ID".into());
    }
    if status["status"] == "added" {
        return Ok(json!({"status":"already_added","id":bundle.manifest.id}));
    }
    let path = snapshot(&bundle, &cache_root()?)?;
    crate::rpc(
        "skill.register".into(),
        json!({"path":path,"if_absent":true}),
    )
    .await?;
    Ok(json!({"status":"added","id":bundle.manifest.id}))
}
pub fn share_text(bundle: &SkillBundle) -> Result<String, String> {
    let bytes = bundle.bytes()?;
    let text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
    // Do not accidentally publish local absolute paths embedded in third-party instructions.
    let private = text
        .as_bytes()
        .windows(3)
        .any(|p| p[0].is_ascii_alphabetic() && p[1] == b':' && matches!(p[2], b'\\' | b'/'))
        || text.contains("/home/")
        || text.contains("/Users/");
    if private {
        return Err("分享正文含本机绝对路径，请在原能力包中改为用户输入或占位符后重新导入".into());
    }
    Ok(text)
}
#[tauri::command]
pub fn market_share(key: String) -> Result<String, String> {
    share_text(&selected(&key)?.0)
}
#[tauri::command]
pub async fn market_export(
    window: tauri::WebviewWindow,
    key: String,
) -> Result<Option<String>, String> {
    let (bundle, _) = selected(&key)?;
    let data = share_text(&bundle)?;
    let Some(path) = crate::program_picker::save_bundle_file(
        window,
        format!("toolhub_{}.toolhub-skill.json", bundle.manifest.id),
    )
    .await?
    else {
        return Ok(None);
    };
    let path = PathBuf::from(path);
    if path.exists() && linked(&path)? {
        return Err("不能覆盖链接文件".into());
    }
    std::fs::write(&path, data).map_err(|e| e.to_string())?;
    Ok(Some(path.to_string_lossy().into_owned()))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> SkillBundle {
        SkillBundle::parse(include_bytes!(
            "../../../skills/market/media-inspect.toolhub-skill.json"
        ))
        .unwrap()
    }
    #[test]
    fn snapshots_preserve_unicode_content_detect_asset_changes_and_share_without_registry_data() {
        let root = std::env::temp_dir().join(format!("toolhub-market-{}", uuid::Uuid::new_v4()));
        let bundle = sample();
        let path = snapshot(&bundle, &root).unwrap();
        assert_eq!(snapshot(&bundle, &root).unwrap(), path);
        assert!(share_text(&bundle).unwrap().contains("toolhub.bundle/v1"));
        std::fs::write(path.parent().unwrap().join("SKILL.md"), "changed").unwrap();
        assert!(snapshot(&bundle, &root).is_err());
        let mut private = bundle;
        private
            .instructions
            .push_str(r" C:\Users\fixture\private.txt");
        assert!(share_text(&private).is_err());
        // Finite, owned fixture files only; never recurse over a computed directory.
        for file in ["skill.json", "SKILL.md", "package.bundle.json"] {
            std::fs::remove_file(path.parent().unwrap().join(file)).unwrap();
        }
        std::fs::remove_dir(path.parent().unwrap()).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
}

#[cfg(test)]
mod import_roundtrip_tests {
    use super::*;
    #[test]
    fn selected_file_import_is_a_snapshot_and_survives_original_file_move() {
        let root = std::env::temp_dir().join(format!("toolhub-import-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let original = root.join("能力包 [1].toolhub-skill.json");
        let mut bundle = SkillBundle::parse(include_bytes!(
            "../../../skills/market/media-inspect.toolhub-skill.json"
        ))
        .unwrap();
        bundle.manifest.id = toolhub_core::SkillId::new("workflow.market.fixture").unwrap();
        std::fs::write(&original, bundle.bytes().unwrap()).unwrap();
        let imported = read_bundle(&original).unwrap();
        let cache = root.join("packages");
        let saved = snapshot(&imported, &cache).unwrap();
        std::fs::rename(&original, root.join("moved.json")).unwrap();
        let loaded = read_bundle(&saved.parent().unwrap().join("package.bundle.json")).unwrap();
        assert_eq!(loaded.manifest, imported.manifest);
        assert_eq!(
            read_limited(&saved.parent().unwrap().join("SKILL.md"), 65536).unwrap(),
            imported.instructions.as_bytes()
        );
        assert_eq!(
            SkillBundle::parse(share_text(&loaded).unwrap().as_bytes())
                .unwrap()
                .manifest,
            imported.manifest
        );
        std::fs::write(root.join("invalid.json"), b"{ invalid JSON }").unwrap();
        assert!(read_bundle(&root.join("invalid.json")).is_err());
        std::fs::write(
            root.join("oversized.json"),
            vec![b' '; MAX_BUNDLE_BYTES + 1],
        )
        .unwrap();
        assert!(read_bundle(&root.join("oversized.json")).is_err());
        for file in ["skill.json", "SKILL.md", "package.bundle.json"] {
            std::fs::remove_file(saved.parent().unwrap().join(file)).unwrap();
        }
        std::fs::remove_dir(saved.parent().unwrap()).unwrap();
        std::fs::remove_dir(cache).unwrap();
        for file in ["moved.json", "invalid.json", "oversized.json"] {
            std::fs::remove_file(root.join(file)).unwrap();
        }
        std::fs::remove_dir(root).unwrap();
    }
}
