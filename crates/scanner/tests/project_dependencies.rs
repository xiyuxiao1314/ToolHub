use std::{path::Path, sync::atomic::AtomicBool};
use toolhub_scanner::project_dependencies::scan;
fn project(root: &Path, version: Option<&str>, binary: bool) {
    std::fs::create_dir_all(root).unwrap();
    std::fs::write(
        root.join("package.json"),
        r#"{"name":"fixture","devDependencies":{"electron":"^37.0.0"}}"#,
    )
    .unwrap();
    if let Some(version) = version {
        let p = root.join("node_modules/electron");
        std::fs::create_dir_all(&p).unwrap();
        std::fs::write(
            p.join("package.json"),
            format!(r#"{{"name":"electron","version":"{version}"}}"#),
        )
        .unwrap();
        if binary {
            write_binary(&p.join("dist").join(binary_name()));
        }
    }
}
fn binary_name() -> &'static str {
    if cfg!(windows) {
        "electron.exe"
    } else if cfg!(target_os = "macos") {
        "Electron.app/Contents/MacOS/Electron"
    } else {
        "electron"
    }
}
fn write_binary(path: &Path) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    // Valid native header, intentionally not runnable; discovery must not launch it.
    let mut bytes = vec![0u8; 512];
    bytes[..2].copy_from_slice(b"MZ");
    bytes[0x3c..0x40].copy_from_slice(&128u32.to_le_bytes());
    bytes[128..132].copy_from_slice(b"PE\0\0");
    bytes[132..134].copy_from_slice(&0x8664u16.to_le_bytes());
    std::fs::write(path, bytes).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}
fn run(roots: &[&Path]) -> toolhub_scanner::ScanReport {
    scan(
        &roots
            .iter()
            .map(|p| p.to_string_lossy().into())
            .collect::<Vec<_>>(),
        &[],
        &AtomicBool::new(false),
    )
}
#[test]
fn full_partial_missing_multiple_versions_and_repeat() {
    let t = tempfile::tempdir().unwrap();
    let a = t.path().join("中文 [a]&'$");
    let b = t.path().join("b");
    let c = t.path().join("c");
    project(&a, Some("37.10.3"), true);
    project(&b, Some("33.4.11"), false);
    project(&c, None, false);
    let r = run(&[&a, &b, &c, &a]);
    assert_eq!(r.candidates.len(), 1);
    assert_eq!(r.project_dependencies.len(), 3);
    assert_eq!(
        r.project_dependencies[0].package_version.as_deref(),
        Some("37.10.3")
    );
    assert_eq!(r.project_dependencies[0].status, "available");
    assert_eq!(r.project_dependencies[1].status, "missing_binary");
    assert_eq!(r.project_dependencies[2].status, "missing_package");
    assert_eq!(
        r.candidates[0].canonical_path,
        run(&[&a]).candidates[0].canonical_path
    );
    project(&b, Some("33.4.11"), true);
    let r = run(&[&a, &b]);
    assert_eq!(r.candidates.len(), 2);
    assert_ne!(
        r.project_dependencies[0].instance_id,
        r.project_dependencies[1].instance_id
    );
    for c in r.candidates {
        assert_eq!(
            toolhub_recognizer::recognize(&c)
                .instance
                .unwrap()
                .trust
                .level,
            toolhub_core::TrustLevel::Unknown
        );
    }
}
#[test]
fn explicit_shared_dist_dedups_instances_preserving_both_projects() {
    let t = tempfile::tempdir().unwrap();
    let a = t.path().join("a");
    let b = t.path().join("b");
    let dist = t.path().join("custom");
    project(&a, Some("37.10.3"), false);
    project(&b, Some("38.0.0"), false);
    write_binary(&dist.join(binary_name()));
    let roots = vec![
        a.to_string_lossy().into_owned(),
        b.to_string_lossy().into_owned(),
    ];
    let overrides = roots
        .iter()
        .map(|p| toolhub_core::ProjectDistOverride {
            project_root: p.clone(),
            electron_dist: dist.to_string_lossy().into(),
        })
        .collect::<Vec<_>>();
    let r = scan(&roots, &overrides, &AtomicBool::new(false));
    assert_eq!(r.project_dependencies.len(), 2);
    assert_eq!(r.candidates.len(), 1);
    assert_eq!(
        r.project_dependencies[0].instance_id,
        r.project_dependencies[1].instance_id
    );
}
#[test]
fn rejects_traversal_absolute_and_non_native_binary() {
    let t = tempfile::tempdir().unwrap();
    project(t.path(), Some("37.10.3"), true);
    let p = t.path().join("node_modules/electron/path.txt");
    for bad in [
        "../electron.exe",
        "C:\\electron.exe",
        "\\\\server\\electron.exe",
        "a/../../electron.exe",
        "other.exe",
    ] {
        std::fs::write(&p, bad).unwrap();
        let r = run(&[t.path()]);
        assert!(r.candidates.is_empty(), "{bad}");
        assert_eq!(r.project_dependencies[0].status, "missing_binary");
    }
    std::fs::remove_file(&p).unwrap();
    std::fs::write(
        t.path()
            .join("node_modules/electron/dist")
            .join(binary_name()),
        b"never execute",
    )
    .unwrap();
    assert_eq!(
        run(&[t.path()]).project_dependencies[0].status,
        "invalid_binary"
    );
}
#[test]
fn bounded_bad_manifest_and_cancel_do_not_certify_coverage() {
    let t = tempfile::tempdir().unwrap();
    project(t.path(), Some("37.10.3"), true);
    std::fs::write(t.path().join("package.json"), vec![b' '; 1024 * 1024 + 1]).unwrap();
    let r = run(&[t.path()]);
    assert!(r.candidates.is_empty());
    assert!(!r.coverage.scopes[0].complete);
    let r = scan(
        &[t.path().to_string_lossy().into()],
        &[],
        &AtomicBool::new(true),
    );
    assert!(r.cancelled);
    assert!(r.candidates.is_empty());
}
#[cfg(unix)]
#[test]
fn links_cannot_escape_selected_project() {
    let t = tempfile::tempdir().unwrap();
    let a = t.path().join("a");
    let b = t.path().join("b");
    project(&a, None, false);
    project(&b, Some("37.10.3"), true);
    std::fs::create_dir_all(a.join("node_modules")).unwrap();
    std::os::unix::fs::symlink(
        b.join("node_modules/electron"),
        a.join("node_modules/electron"),
    )
    .unwrap();
    let r = run(&[&a]);
    assert_eq!(r.project_dependencies[0].status, "invalid_package");
    assert!(r.candidates.is_empty());
}
