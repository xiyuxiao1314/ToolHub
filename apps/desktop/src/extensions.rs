use serde_json::Value;
use std::io::{Read, Write};
use toolhub_skills::extension::{ExtensionPackage, MAX_EXTENSION_BYTES};
#[tauri::command]
pub fn extension_catalog() -> Result<Value, String> {
    // Embedded, audited example also ships as a developer-editable JSON asset.
    let p = ExtensionPackage::parse(include_bytes!(
        "../../../extensions/examples/blender.toolhub-extension.json"
    ))?;
    serde_json::to_value(vec![p]).map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn extension_pick(window: tauri::WebviewWindow) -> Result<Option<Value>, String> {
    let Some(path) = crate::program_picker::pick_bundle_file(window).await? else {
        return Ok(None);
    };
    let meta = std::fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
    #[cfg(windows)]
    let linked = {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & 0x400 != 0
    };
    #[cfg(not(windows))]
    let linked = meta.file_type().is_symlink();
    if linked || !meta.is_file() || meta.len() > MAX_EXTENSION_BYTES as u64 {
        return Err("扩展包须为不超过 256 KiB 的普通 JSON 文件".into());
    }
    let mut bytes = vec![];
    std::fs::File::open(&path)
        .map_err(|e| e.to_string())?
        .take((MAX_EXTENSION_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    let p = ExtensionPackage::parse(&bytes)?;
    Ok(Some(serde_json::to_value(p).map_err(|e| e.to_string())?))
}
#[tauri::command]
pub async fn extension_export(
    window: tauri::WebviewWindow,
    package: Value,
) -> Result<Option<String>, String> {
    let p = ExtensionPackage::parse(package.to_string().as_bytes())?;
    let Some(path) = crate::program_picker::save_bundle_file(
        window,
        format!("{}-{}.toolhub-extension.json", p.id, p.version),
    )
    .await?
    else {
        return Ok(None);
    };
    if let Ok(meta) = std::fs::symlink_metadata(&path) {
        #[cfg(windows)]
        let linked = {
            use std::os::windows::fs::MetadataExt;
            meta.file_attributes() & 0x400 != 0
        };
        #[cfg(not(windows))]
        let linked = meta.file_type().is_symlink();
        if linked || !meta.is_file() {
            return Err("不能覆盖链接或非普通文件".into());
        }
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    file.write_all(
        serde_json::to_string_pretty(&p)
            .map_err(|e| e.to_string())?
            .as_bytes(),
    )
    .map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    Ok(Some(path))
}
