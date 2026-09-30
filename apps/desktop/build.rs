fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(&["rpc", "app_versions"])),
    )
    .expect("build desktop permissions");
}
