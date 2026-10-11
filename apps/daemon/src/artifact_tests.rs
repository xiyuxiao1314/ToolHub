use super::*;
fn fixture() -> (tempfile::TempDir, DaemonService) {
    let dir = tempfile::Builder::new()
        .prefix("artifact-fixture-")
        .tempdir_in(std::env::current_dir().unwrap())
        .unwrap();
    let mut svc = DaemonService::open(&dir.path().join("registry.sqlite")).unwrap();
    svc.connection_principal = Some("agent-a".into());
    (dir, svc)
}
fn root(svc: &mut DaemonService, dir: &Path) -> String {
    svc.controller_verified = true;
    let roots = svc
        .dispatch_artifact("artifact.root_add", &json!({"path":dir}))
        .unwrap();
    svc.controller_verified = false;
    roots[0]["id"].as_str().unwrap().into()
}
fn package(svc: &mut DaemonService, dir: &Path, priority: &str) -> String {
    std::fs::write(dir.join("test_owned.txt"), "owned fixture").unwrap();
    svc.dispatch_artifact("artifact.register", &json!({"root":dir,"name":"测试包","task":"test task","files":["test_owned.txt"],"priority":priority})).unwrap()["id"].as_str().unwrap().into()
}
fn complete(svc: &mut DaemonService, id: &str) -> Value {
    svc.dispatch_artifact(
        "artifact.complete",
        &json!({"id":id,"no_longer_needed":true,"reason":"fixture no longer needed"}),
    )
    .unwrap()
}
fn due(svc: &mut DaemonService, id: &str) {
    let mut item = svc.artifact_package(id).unwrap();
    item.cleanup_after = Some(0);
    svc.save_artifact(&mut item, "test_clock").unwrap();
}
#[test]
fn discovery_and_registration_never_delete_or_imply_scope() {
    let (dir, mut svc) = fixture();
    let id = package(&mut svc, dir.path(), "low");
    assert_eq!(complete(&mut svc, &id)["status"], "awaiting_scope");
    svc.artifact_cleanup_tick().unwrap();
    assert!(dir.path().join("test_owned.txt").exists());
    let root_id = root(&mut svc, dir.path());
    svc.controller_verified = true;
    let scan = svc
        .dispatch_artifact("artifact.scan", &json!({"root_id":root_id}))
        .unwrap();
    assert_eq!(scan["files"].as_array().unwrap().len(), 0); // already registered
    std::fs::write(dir.path().join("backup.old"), "leftover").unwrap();
    let scan = svc
        .dispatch_artifact("artifact.scan", &json!({"root_id":root_id}))
        .unwrap();
    assert_eq!(scan["files"].as_array().unwrap().len(), 1);
    assert!(dir.path().join("backup.old").exists());
}
#[test]
fn low_waits_for_completion_and_grace_and_keep_cancels() {
    let (dir, mut svc) = fixture();
    root(&mut svc, dir.path());
    let id = package(&mut svc, dir.path(), "low");
    svc.artifact_cleanup_tick().unwrap();
    assert!(dir.path().join("test_owned.txt").exists());
    let item = complete(&mut svc, &id);
    assert_eq!(item["status"], "scheduled");
    assert!(item["cleanup_after"].as_i64().unwrap() >= chrono::Utc::now().timestamp() + 298);
    svc.artifact_cleanup_tick().unwrap();
    assert!(dir.path().join("test_owned.txt").exists());
    svc.controller_verified = true;
    svc.dispatch_artifact("artifact.update", &json!({"id":id,"retained":true}))
        .unwrap();
    due(&mut svc, &id);
    svc.artifact_cleanup_tick().unwrap();
    assert!(dir.path().join("test_owned.txt").exists());
}
#[test]
fn medium_requires_review_and_assertions_high_is_human_only() {
    let (dir, mut svc) = fixture();
    root(&mut svc, dir.path());
    let id = package(&mut svc, dir.path(), "medium");
    assert_eq!(complete(&mut svc, &id)["status"], "needs_review");
    svc.artifact_cleanup_tick().unwrap();
    assert!(dir.path().join("test_owned.txt").exists());
    assert!(svc
        .dispatch_artifact(
            "artifact.review",
            &json!({"id":id,"decision":"recycle","reason":"test"})
        )
        .is_err());
    assert_eq!(svc.dispatch_artifact("artifact.review",&json!({"id":id,"decision":"recycle","unused":true,"dependencies_checked":true,"reason":"dependencies checked"})).unwrap()["status"],"scheduled");
    svc.controller_verified = true;
    svc.dispatch_artifact("artifact.update", &json!({"id":id,"priority":"high"}))
        .unwrap();
    svc.controller_verified = false;
    assert!(svc.dispatch_artifact("artifact.review",&json!({"id":id,"decision":"recycle","unused":true,"dependencies_checked":true,"reason":"test"})).is_err());
    assert!(svc
        .dispatch_artifact("artifact.update", &json!({"id":id,"priority":"low"}))
        .is_err());
    assert!(svc
        .dispatch_artifact("artifact.recycle", &json!({"id":id,"confirmed":true}))
        .is_err());
    due(&mut svc, &id);
    svc.artifact_cleanup_tick().unwrap();
    assert!(dir.path().join("test_owned.txt").exists());
}
#[test]
fn changed_files_and_paused_roots_block_due_cleanup() {
    let (dir, mut svc) = fixture();
    let rid = root(&mut svc, dir.path());
    let id = package(&mut svc, dir.path(), "low");
    complete(&mut svc, &id);
    due(&mut svc, &id);
    std::fs::write(dir.path().join("test_owned.txt"), "new important content").unwrap();
    svc.artifact_cleanup_tick().unwrap();
    assert_eq!(svc.artifact_package(&id).unwrap().status, "blocked");
    assert!(dir.path().join("test_owned.txt").exists());
    svc.controller_verified = true;
    svc.dispatch_artifact("artifact.root_disable", &json!({"id":rid}))
        .unwrap();
    assert!(!svc.artifact_scope(dir.path()).unwrap());
    assert!(dir.path().join("test_owned.txt").exists());
}
#[test]
fn auth_traversal_protected_paths_and_reference_checks() {
    let (dir, mut svc) = fixture();
    assert!(svc
        .dispatch_artifact("artifact.root_add", &json!({"path":dir.path()}))
        .is_err());
    let id = package(&mut svc, dir.path(), "high");
    svc.connection_principal = Some("agent-b".into());
    assert!(svc.artifact_package(&id).is_err());
    assert!(svc
        .dispatch_artifact(
            "artifact.complete",
            &json!({"id":id,"no_longer_needed":true,"reason":"test"})
        )
        .is_err());
    for p in [
        "../other.txt",
        "C:\\Windows\\file.txt",
        "node_modules/file.txt",
        ".git/config",
    ] {
        assert!(relative(p).is_err(), "{p}");
    }
    svc.registry.db.conn.execute("INSERT INTO program_entries(id,identity,entry_json,updated_at) VALUES('artifact-ref','artifact-ref',?1,'test')",[json!({"path":plain(&dir.path().join("test_owned.txt"))}).to_string()]).unwrap();
    assert!(svc
        .check_artifact_reference(&dir.path().join("test_owned.txt"))
        .is_err());
}
#[test]
fn restart_preserves_packages_and_blocks_interrupted_recycling() {
    let (dir, mut svc) = fixture();
    let id = package(&mut svc, dir.path(), "high");
    let mut item = svc.artifact_package(&id).unwrap();
    item.status = "recycling".into();
    svc.save_artifact(&mut item, "test_interruption").unwrap();
    drop(svc);
    let mut reopened = DaemonService::open(&dir.path().join("registry.sqlite")).unwrap();
    reopened.controller_verified = true;
    reopened.artifact_recover_interrupted().unwrap();
    assert_eq!(reopened.artifact_package(&id).unwrap().status, "blocked");
    assert!(dir.path().join("test_owned.txt").exists());
}
#[cfg(windows)]
#[test]
fn native_recycle_only_moves_owned_unicode_fixture() {
    let (dir, mut svc) = fixture();
    root(&mut svc, dir.path());
    let name = "test_中文 [1] & '$.txt";
    std::fs::write(dir.path().join(name), "disposable fixture only").unwrap();
    let id=svc.dispatch_artifact("artifact.register",&json!({"root":dir.path(),"name":"回收站测试","task":"owned fixture","files":[name],"priority":"low"})).unwrap()["id"].as_str().unwrap().to_owned();
    complete(&mut svc, &id);
    due(&mut svc, &id);
    svc.artifact_cleanup_tick().unwrap();
    let item = svc.artifact_package(&id).unwrap();
    assert_eq!(item.status, "trashed", "{:?}", item.files);
    assert!(!dir.path().join(name).exists());
}
#[cfg(windows)]
#[test]
fn file_in_use_is_preserved_without_delete_fallback() {
    use std::os::windows::fs::OpenOptionsExt;
    let (dir, mut svc) = fixture();
    root(&mut svc, dir.path());
    let id = package(&mut svc, dir.path(), "low");
    complete(&mut svc, &id);
    due(&mut svc, &id);
    let held = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(dir.path().join("test_owned.txt"))
        .unwrap();
    svc.artifact_cleanup_tick().unwrap();
    assert_eq!(svc.artifact_package(&id).unwrap().status, "blocked");
    assert!(dir.path().join("test_owned.txt").exists());
    drop(held);
}

#[test]
fn selected_parent_covers_specific_task_children_not_adjacent_folders() {
    let (dir, mut svc) = fixture();
    root(&mut svc, dir.path());
    let child = dir.path().join("task");
    std::fs::create_dir(&child).unwrap();
    assert!(svc.artifact_scope(&child).unwrap());
    assert!(!svc
        .artifact_scope(&dir.path().with_extension("neighbor"))
        .unwrap());
    std::fs::write(child.join("test_default.log"), "fixture").unwrap();
    let result = svc
        .dispatch_artifact(
            "artifact.register",
            &json!({"root":child,"name":"default","task":"test","files":["test_default.log"]}),
        )
        .unwrap();
    assert_eq!(result["package"]["priority"], "high");
    assert_eq!(result["auto_scope"], true);
}

#[test]
fn human_can_release_changed_registration_without_deleting_files() {
    let (dir, mut svc) = fixture();
    root(&mut svc, dir.path());
    let id = package(&mut svc, dir.path(), "low");
    complete(&mut svc, &id);
    due(&mut svc, &id);
    std::fs::write(
        dir.path().join("test_owned.txt"),
        "changed important output",
    )
    .unwrap();
    svc.artifact_cleanup_tick().unwrap();
    assert!(svc
        .dispatch_artifact("artifact.forget", &json!({"id":id}))
        .is_err());
    svc.controller_verified = true;
    assert_eq!(
        svc.dispatch_artifact("artifact.forget", &json!({"id":id}))
            .unwrap()["status"],
        "released"
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("test_owned.txt")).unwrap(),
        "changed important output"
    );
    let next=svc.dispatch_artifact("artifact.register",&json!({"root":dir.path(),"name":"new","task":"new version","files":["test_owned.txt"]})).unwrap();
    assert_ne!(next["id"], id);
}

#[cfg(windows)]
#[test]
fn windows_alternate_data_streams_are_not_regular_artifact_paths() {
    let (dir, mut svc) = fixture();
    std::fs::write(dir.path().join("test_base.txt"), "base must remain").unwrap();
    assert!(relative("test_base.txt:hidden").is_err());
    assert!(svc
        .dispatch_artifact(
            "artifact.register",
            &json!({"root":dir.path(),"name":"ADS","task":"test","files":["test_base.txt:hidden"]})
        )
        .is_err());
    assert_eq!(
        std::fs::read_to_string(dir.path().join("test_base.txt")).unwrap(),
        "base must remain"
    );
}

#[test]
fn summary_pagination_does_not_embed_all_file_manifests() {
    let (dir, mut svc) = fixture();
    let id = package(&mut svc, dir.path(), "high");
    let original = svc.artifact_package(&id).unwrap();
    for index in 0..39 {
        let mut next = original.clone();
        next.id = format!("summary-fixture-{index}");
        next.task = "a".repeat(2048);
        svc.save_artifact(&mut next, "fixture").unwrap();
    }
    let first = svc.dispatch_artifact("artifact.list", &json!({})).unwrap();
    let second = svc
        .dispatch_artifact("artifact.list", &json!({"offset":20}))
        .unwrap();
    assert_eq!(first.as_array().unwrap().len(), 20);
    assert_eq!(second.as_array().unwrap().len(), 20);
    assert_eq!(first[0]["file_count"], 1);
    assert_eq!(first[0]["files"], json!([]));
    assert!(serde_json::to_vec(&first).unwrap().len() < 1024 * 1024);
    let details = svc
        .dispatch_artifact("artifact.inspect", &json!({"id":first[0]["id"]}))
        .unwrap();
    assert_eq!(details["package"]["files"].as_array().unwrap().len(), 1);
}
