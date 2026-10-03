fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "rpc",
            "app_versions",
            "default_export_path_string",
            "save_text_file",
            "read_text_file",
            "open_terminal",
            "reveal_path",
            "scan_root_for_path",
            "pick_program_path",
            "open_program_folder",
        ]),
    ))
    .expect("build desktop permissions");
}
