use super::*;
fn call(s: &mut DaemonService, name: &str, params: Value, controller: bool) -> Value {
    let r = s.handle_authenticated(
        &JsonRpcRequest::new(1, name, params),
        "project-fixture",
        controller,
    );
    assert!(r.error.is_none(), "{:?}", r.error);
    r.result.unwrap()
}
#[test]
fn project_scan_persists_associations_missing_state_and_manual_trust_without_execution() {
    let t = tempfile::tempdir().unwrap();
    let db = t.path().join("registry.sqlite");
    let a = t.path().join("project");
    let p = a.join("node_modules/electron");
    std::fs::create_dir_all(p.join("dist")).unwrap();
    std::fs::write(
        a.join("package.json"),
        r#"{"name":"test-project","devDependencies":{"electron":"37.10.3"}}"#,
    )
    .unwrap();
    std::fs::write(
        p.join("package.json"),
        r#"{"name":"electron","version":"37.10.3"}"#,
    )
    .unwrap();
    let filename = if cfg!(windows) {
        "electron.exe"
    } else if cfg!(target_os = "macos") {
        "Electron.app/Contents/MacOS/Electron"
    } else {
        "electron"
    };
    let binary = p.join("dist").join(filename);
    std::fs::create_dir_all(binary.parent().unwrap()).unwrap();
    let mut bytes = vec![0u8; 512];
    bytes[..2].copy_from_slice(b"MZ");
    bytes[60..64].copy_from_slice(&128u32.to_le_bytes());
    bytes[128..132].copy_from_slice(b"PE\0\0");
    bytes[132..134].copy_from_slice(&0x8664u16.to_le_bytes());
    std::fs::write(&binary, bytes).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let mut s = DaemonService::open(&db).unwrap();
    let params = json!({"mode":"project","roots":[a]});
    call(&mut s, "scan.start", params.clone(), false);
    let deps = call(&mut s, "registry.project_dependencies", json!({}), false);
    let id = deps[0]["instance_id"].as_str().unwrap().to_owned();
    let inspect = call(&mut s, "registry.inspect_instance", json!({"id":id}), false);
    assert_eq!(inspect["instance"]["version"], "37.10.3");
    assert_eq!(inspect["projects"][0]["project_name"], "test-project");
    assert!(inspect["instance"]["environment_id"]
        .as_str()
        .unwrap()
        .starts_with("env.npmproject."));
    assert!(inspect["instance"]["trust"]
        .as_str()
        .unwrap()
        .contains("unknown"));
    call(
        &mut s,
        "registry.correct",
        json!({"id":id,"trust":"blocked"}),
        true,
    );
    call(&mut s, "scan.start", params.clone(), false);
    assert_eq!(s.registry.count_instances().unwrap(), 1);
    assert!(s
        .registry
        .get_instance(&id)
        .unwrap()
        .unwrap()
        .trust
        .contains("blocked"));
    std::fs::write(&binary, b"invalid replacement; never execute").unwrap();
    call(&mut s, "scan.start", params.clone(), false);
    assert_eq!(
        call(&mut s, "registry.project_dependencies", json!({}), false)[0]["status"],
        "invalid_binary"
    );
    assert_eq!(
        s.registry.get_instance(&id).unwrap().unwrap().status,
        "missing"
    );
    std::fs::remove_file(&binary).unwrap();
    call(&mut s, "scan.start", params.clone(), false);
    let deps = call(&mut s, "registry.project_dependencies", json!({}), false);
    assert_eq!(deps[0]["status"], "missing_binary");
    assert!(deps[0]["instance_id"].is_null());
    assert_eq!(
        s.registry.get_instance(&id).unwrap().unwrap().status,
        "missing"
    );
    drop(s);
    let mut s = DaemonService::open(&db).unwrap();
    assert_eq!(
        call(&mut s, "registry.project_dependencies", json!({}), false)[0]["status"],
        "missing_binary"
    );
    let invalid=s.handle_authenticated(&JsonRpcRequest::new(2,"scan.start",json!({"mode":"project","roots":[a],"project_overrides":[{"project_root":t.path(),"electron_dist":t.path()}]})),"fixture",false);
    assert!(invalid.error.is_some());
    let count: i64 = s
        .registry
        .db
        .conn
        .query_row("SELECT COUNT(*) FROM managed_tasks", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn shared_distribution_keeps_all_project_references_and_incomplete_scans_keep_observations() {
    let t = tempfile::tempdir().unwrap();
    let a = t.path().join("a");
    let b = t.path().join("b");
    let dist = t.path().join("dist");
    for project in [&a, &b] {
        let p = project.join("node_modules/electron");
        std::fs::create_dir_all(&p).unwrap();
        std::fs::write(
            project.join("package.json"),
            r#"{"name":"shared-fixture","dependencies":{"electron":"37.10.3"}}"#,
        )
        .unwrap();
        std::fs::write(
            p.join("package.json"),
            r#"{"name":"electron","version":"37.10.3"}"#,
        )
        .unwrap();
    }
    let filename = if cfg!(windows) {
        "electron.exe"
    } else if cfg!(target_os = "macos") {
        "Electron.app/Contents/MacOS/Electron"
    } else {
        "electron"
    };
    let binary = dist.join(filename);
    std::fs::create_dir_all(binary.parent().unwrap()).unwrap();
    let mut bytes = vec![0u8; 512];
    bytes[..2].copy_from_slice(b"MZ");
    bytes[60..64].copy_from_slice(&128u32.to_le_bytes());
    bytes[128..132].copy_from_slice(b"PE\0\0");
    std::fs::write(&binary, bytes).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let mut s = DaemonService::open(&t.path().join("registry.sqlite")).unwrap();
    let params = json!({"mode":"project","roots":[a,b],"project_overrides":[{"project_root":a,"electron_dist":dist},{"project_root":b,"electron_dist":dist}]});
    call(&mut s, "scan.start", params.clone(), false);
    let deps = call(&mut s, "registry.project_dependencies", json!({}), false);
    assert_eq!(deps.as_array().unwrap().len(), 2);
    assert_eq!(deps[0]["instance_id"], deps[1]["instance_id"]);
    let id = deps[0]["instance_id"].as_str().unwrap();
    assert_eq!(s.registry.count_instances().unwrap(), 1);
    assert_eq!(
        call(&mut s, "registry.inspect_instance", json!({"id":id}), false)["projects"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    std::fs::write(a.join("package.json"), "broken manifest").unwrap();
    let result = call(&mut s, "scan.start", params.clone(), false);
    assert_eq!(result["status"], "partial");
    assert_eq!(
        call(&mut s, "registry.project_dependencies", json!({}), false)
            .as_array()
            .unwrap()
            .len(),
        2
    );
    std::fs::write(a.join("package.json"), "{}").unwrap();
    std::fs::remove_file(a.join("node_modules/electron/package.json")).unwrap();
    call(
        &mut s,
        "scan.start",
        json!({"mode":"project","roots":[a]}),
        false,
    );
    assert_eq!(
        s.registry.get_instance(id).unwrap().unwrap().status,
        "available"
    );
    assert_eq!(
        call(&mut s, "registry.inspect_instance", json!({"id":id}), false)["projects"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
