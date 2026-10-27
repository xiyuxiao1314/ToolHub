//! Owned loopback fixtures only. No production registry, settings, ports or applications.
use super::binding_native;
use super::*;
use std::{
    path::PathBuf,
    process::{Child, Command},
    time::{Duration, Instant},
};
#[test]
#[ignore = "only used by an owned child process to hold a fixture job handle"]
fn job_keeper_fixture() {
    let name = std::env::var("TOOLHUB_TEST_KEEP_JOB").unwrap();
    native::keep_job_alive(&name).unwrap();
}
struct Fixture(Child);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn fixture_binary(dir: &Path) -> PathBuf {
    let path = dir.join("owned-fixture.exe");
    std::fs::copy(std::env::current_exe().unwrap(), &path).unwrap();
    path
}
#[test]
#[ignore = "owned native process fixture, invoked explicitly by a parent test"]
fn single_process_fixture() {
    std::thread::sleep(Duration::from_secs(120));
}
#[test]
#[ignore = "owned loopback fixture, invoked explicitly by a parent test"]
fn native_server_fixture() {
    let arguments = std::env::args().collect::<Vec<_>>();
    let Some(i) = arguments.iter().position(|a| a == "--logfile") else {
        return;
    };
    let log = PathBuf::from(&arguments[i + 1]);
    let root = log.parent().unwrap();
    let child = log.file_name().unwrap() == "child.log";
    let tcp = if !child {
        Some(std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap())
    } else {
        None
    };
    let udp = if child {
        Some(std::net::UdpSocket::bind(("127.0.0.1", 0)).unwrap())
    } else {
        None
    };
    let tcp6 = std::net::TcpListener::bind(("::1", 0)).ok();
    let udp6 = std::net::UdpSocket::bind(("::1", 0)).ok();
    let port = tcp
        .as_ref()
        .map(|s| s.local_addr().unwrap().port())
        .or_else(|| udp.as_ref().map(|s| s.local_addr().unwrap().port()))
        .unwrap();
    std::fs::write(root.join(if child{"child.json"}else{"root.json"}),json!({"pid":std::process::id(),"port":port,"tcp6":tcp6.as_ref().map(|s|s.local_addr().unwrap().port()),"udp6":udp6.as_ref().map(|s|s.local_addr().unwrap().port())}).to_string()).unwrap();
    let mut spawned = if !child {
        Some(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "service::services::tests::native_server_fixture",
                    "--ignored",
                    "--logfile",
                ])
                .arg(root.join("child.log"))
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .unwrap(),
        )
    } else {
        None
    };
    std::thread::sleep(Duration::from_secs(120));
    if let Some(child) = &mut spawned {
        let _ = child.kill();
        let _ = child.wait();
    }
}
fn call(
    s: &mut DaemonService,
    method: &str,
    params: Value,
    admin: bool,
) -> Result<Value, ProtocolError> {
    let r = s.handle_authenticated(
        &JsonRpcRequest::new(1, method, params),
        "fixture.owner",
        admin,
    );
    r.result.ok_or_else(|| {
        let e = r.error.unwrap();
        ProtocolError::new(ErrorCode::Denied, e.message)
    })
}
fn wait(mut condition: impl FnMut() -> bool) {
    let end = Instant::now() + Duration::from_secs(8);
    while Instant::now() < end {
        if condition() {
            return;
        }
        std::thread::sleep(Duration::from_millis(40));
    }
    panic!("fixture did not reach expected state");
}
#[test]
fn registration_observes_only_owned_identity_and_grants_no_control() {
    let dir = tempfile::tempdir().unwrap();
    let file = fixture_binary(dir.path());
    let child = Command::new(&file)
        .args([
            "--exact",
            "service::services::tests::single_process_fixture",
            "--ignored",
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let owned = Fixture(child);
    let mut s = DaemonService::open(&dir.path().join("r.sqlite")).unwrap();
    let registration =
        json!({"name":"owned test","pid":owned.0.id(),"project_dir":dir.path(),"kind":"web"});
    let id = call(&mut s, "service.register", registration.clone(), false).unwrap()["id"].clone();
    assert_eq!(
        call(&mut s, "service.register", registration, false).unwrap()["id"],
        id
    );
    for method in [
        "service.stop",
        "service.restart",
        "service.remove",
        "service.configure",
        "service.start",
        "service.proxy_status",
        "service.proxy_configure",
    ] {
        assert!(
            call(&mut s, method, json!({"id":id}), false).is_err(),
            "{method}"
        );
    }
    assert_eq!(
        call(&mut s, "service.list", json!({}), false).unwrap()["total"],
        1
    );
    let summary = call(&mut s, "service.list", json!({}), false).unwrap();
    assert!(summary["items"][0].get("history").is_none());
    assert_eq!(summary["items"][0]["execution_authority"], false);
    let other = s.handle_authenticated(
        &JsonRpcRequest::new(1, "service.list", json!({})),
        "fixture.other",
        false,
    );
    assert_eq!(other.result.unwrap()["total"], 0);
    assert!(call(
        &mut s,
        "service.configure",
        json!({"id":id,"auto_stop_minutes":1,"confirm_force":true}),
        true
    )
    .is_err());
    assert!(call(
        &mut s,
        "service.configure",
        json!({"id":id,"web_url":"file:///C:/Windows/cmd.exe"}),
        true
    )
    .is_err());
    call(
        &mut s,
        "service.configure",
        json!({"id":id,"web_url":"https://example.local/"}),
        true,
    )
    .unwrap();
    assert!(call(&mut s, "service.page_url", json!({"id":id}), false).is_err());
    assert_eq!(
        call(&mut s, "service.page_url", json!({"id":id}), true).unwrap()["url"],
        "https://example.local/"
    );
    let mut records = s.service_records().unwrap();
    records[0].created = "1".into();
    s.save_services(&records).unwrap();
    assert!(call(&mut s, "service.stop", json!({"id":id,"force":true}), true).is_err());
    assert!(native::process(owned.0.id(), false).is_ok());
    assert!(call(
        &mut s,
        "service.register",
        json!({"name":"daemon","pid":std::process::id(),"project_dir":dir.path()}),
        false
    )
    .is_err());
    assert!(call(
        &mut s,
        "service.register",
        json!({"name":"bad","pid":owned.0.id(),"project_dir":dir.path(),"controller":true}),
        false
    )
    .is_err());
    assert!(call(&mut s, "service.list", json!({"limit":51}), true).is_err());
}
fn launch(s: &mut DaemonService, dir: &Path) -> String {
    let executable = fixture_binary(dir);
    let selected = call(
        s,
        "program.select",
        json!({"path":executable,"kind":"file"}),
        true,
    )
    .unwrap();
    let mut item = selected[0].clone();
    item["name"] = json!("Owned child tree");
    item["args"] = json!([
        "--exact",
        "service::services::tests::native_server_fixture",
        "--ignored",
        "--logfile",
        dir.join("root.log")
    ]);
    call(s, "program.save", json!({"items":[item]}), true).unwrap();
    let programs = call(s, "program.list", json!({}), true).unwrap();
    let id = call(
        s,
        "service.start",
        json!({"program_id":programs[0]["id"]}),
        true,
    )
    .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    wait(|| dir.join("child.json").exists());
    id
}
fn json_file(p: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
}
#[test]
fn managed_job_tracks_child_ports_controls_restarts_and_survives_daemon_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("r.sqlite");
    let mut s = DaemonService::open(&db).unwrap();
    let id = launch(&mut s, dir.path());
    let root = json_file(&dir.path().join("root.json"));
    let child = json_file(&dir.path().join("child.json"));
    let snapshot =
        call(&mut s, "service.list", json!({"id":id}), true).unwrap()["items"][0].clone();
    assert_eq!(snapshot["status"], "running");
    assert!(snapshot["process_count"].as_u64().unwrap() >= 2);
    assert!(snapshot["application_path"]
        .as_str()
        .unwrap()
        .to_lowercase()
        .ends_with("owned-fixture.exe"));
    for (manifest, protocol) in [(&root, "TCP"), (&child, "UDP")] {
        assert!(
            snapshot["endpoints"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["pid"] == manifest["pid"]
                    && e["port"] == manifest["port"]
                    && e["protocol"] == protocol),
            "{snapshot}"
        );
    }
    for manifest in [&root, &child] {
        for (key, protocol) in [("tcp6", "TCP"), ("udp6", "UDP")] {
            if let Some(port) = manifest[key].as_u64() {
                assert!(snapshot["endpoints"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|e| e["port"] == port
                        && e["protocol"] == protocol
                        && e["address"] == "::1"));
            }
        }
    }
    assert!(call(&mut s, "service.restart", json!({"id":id}), true).is_err());
    assert!(call(
        &mut s,
        "service.configure",
        json!({"id":id,"auto_stop_minutes":1}),
        true
    )
    .is_err());
    call(
        &mut s,
        "service.configure",
        json!({"id":id,"auto_stop_minutes":60,"confirm_force":true}),
        true,
    )
    .unwrap();
    drop(s);
    let mut s = DaemonService::open(&db).unwrap();
    let snapshot =
        call(&mut s, "service.list", json!({"id":id}), true).unwrap()["items"][0].clone();
    assert_eq!(snapshot["status"], "running");
    assert!(snapshot["auto_stop_at"].is_null());
    call(&mut s, "service.stop", json!({"id":id,"force":false}), true).unwrap();
    assert!(native::process(root["pid"].as_u64().unwrap() as u32, false).is_ok());
    call(&mut s, "service.stop", json!({"id":id,"force":true}), true).unwrap();
    wait(|| {
        let record = s.service_records().unwrap()[0].clone();
        s.service_snapshot(&record)["status"] == "stopped"
    });
    assert!(native::process(child["pid"].as_u64().unwrap() as u32, false).is_err());
    // The ports are actually released, not merely hidden in UI.
    assert!(
        std::net::TcpListener::bind(("127.0.0.1", root["port"].as_u64().unwrap() as u16)).is_ok()
    );
    assert!(
        std::net::UdpSocket::bind(("127.0.0.1", child["port"].as_u64().unwrap() as u16)).is_ok()
    );
    call(
        &mut s,
        "service.configure",
        json!({"id":id,"web_url":"https://example.local/"}),
        true,
    )
    .unwrap();
    let next = call(&mut s, "service.restart", json!({"id":id}), true).unwrap()["id"].clone();
    assert_eq!(next, id);
    let current = call(&mut s, "service.list", json!({"id":next}), true).unwrap();
    assert_eq!(current["items"][0]["web_url"], "https://example.local/");
    assert_eq!(
        call(&mut s, "service.list", json!({}), true).unwrap()["total"],
        1
    );

    call(
        &mut s,
        "service.stop",
        json!({"id":next,"force":true}),
        true,
    )
    .unwrap();
}
#[test]
fn proxy_protection_and_explicit_timer_disable_uninstall_and_no_reseed() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("r.sqlite");
    let mut s = DaemonService::open(&db).unwrap();
    let id = launch(&mut s, dir.path());
    let port = json_file(&dir.path().join("root.json"))["port"].clone();
    call(
        &mut s,
        "service.proxy_configure",
        json!({"ports":[{"name":"owned proxy fixture","port":port}]}),
        true,
    )
    .unwrap();
    assert_eq!(
        call(&mut s, "service.proxy_status", json!({}), true).unwrap()["items"][0]["listening"],
        true
    );
    assert!(call(&mut s, "service.stop", json!({"id":id,"force":true}), true).is_err());
    assert!(call(
        &mut s,
        "service.configure",
        json!({"id":id,"auto_stop_minutes":1,"confirm_force":true}),
        true
    )
    .is_err());
    call(&mut s, "service.proxy_configure", json!({"ports":[]}), true).unwrap();
    call(
        &mut s,
        "service.configure",
        json!({"id":id,"auto_stop_minutes":1,"confirm_force":true}),
        true,
    )
    .unwrap();
    let extension = call(&mut s, "extension.list", json!({}), true)
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["package"]["id"] == "toolhub.builtin.services")
        .unwrap()
        .clone();
    call(&mut s,"extension.set_enabled",json!({"id":"toolhub.builtin.services","expected_digest":extension["digest"],"enabled":false}),true).unwrap();
    assert!(s.service_records().unwrap()[0].auto_stop_at.is_none());
    s.service_monitor_tick().unwrap();
    let record = s.service_records().unwrap()[0].clone();
    assert_eq!(s.service_snapshot(&record)["status"], "running");
    call(&mut s,"extension.set_enabled",json!({"id":"toolhub.builtin.services","expected_digest":extension["digest"],"enabled":true}),true).unwrap();
    call(
        &mut s,
        "service.configure",
        json!({"id":id,"auto_stop_minutes":1,"confirm_force":true}),
        true,
    )
    .unwrap();
    let mut records = s.service_records().unwrap();
    records[0].auto_stop_at =
        Some((chrono::Utc::now() - chrono::Duration::seconds(1)).to_rfc3339());
    s.save_services(&records).unwrap();
    s.service_monitor_tick().unwrap();
    wait(|| {
        let record = s.service_records().unwrap()[0].clone();
        s.service_snapshot(&record)["status"] == "stopped"
    });
    let ext = call(&mut s, "extension.list", json!({}), true)
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["package"]["id"] == "toolhub.builtin.services")
        .unwrap()
        .clone();
    call(
        &mut s,
        "extension.remove",
        json!({"id":"toolhub.builtin.services","expected_digest":ext["digest"]}),
        true,
    )
    .unwrap();
    drop(s);
    let mut s = DaemonService::open(&db).unwrap();
    assert!(call(&mut s, "service.list", json!({}), true).is_err());
    assert_eq!(s.service_records().unwrap().len(), 1);
}

#[test]
fn detached_binding_follows_only_unique_exact_process_and_rejects_stale_stop() {
    let dir = tempfile::tempdir().unwrap();
    let executable = fixture_binary(dir.path());
    let argv = [
        "--exact",
        "service::services::tests::single_process_fixture",
        "--ignored",
    ];
    let spawn = || {
        Command::new(&executable)
            .args(argv)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap()
    };
    let mut first = Fixture(spawn());
    let binding = binding_native::Binding {
        executable_paths: vec![executable.to_string_lossy().into()],
        arguments: argv.iter().map(|s| s.to_string()).collect(),
    };
    assert_eq!(binding.find().unwrap().unwrap().identity.pid, first.0.id());
    let mut s = DaemonService::open(&dir.path().join("r.sqlite")).unwrap();
    let id = call(
        &mut s,
        "service.register",
        json!({"name":"detached fixture","pid":first.0.id(),"project_dir":dir.path()}),
        false,
    )
    .unwrap()["id"]
        .clone();
    assert!(call(
        &mut s,
        "service.configure",
        json!({"id":id,"binding":binding}),
        false
    )
    .is_err());
    call(
        &mut s,
        "service.configure",
        json!({"id":id,"binding":binding}),
        true,
    )
    .unwrap();
    let old = call(&mut s, "service.list", json!({"id":id}), true).unwrap()["items"][0].clone();
    let mut duplicate = Fixture(spawn());
    assert!(binding.find().is_err());
    let ambiguous =
        call(&mut s, "service.list", json!({"id":id}), true).unwrap()["items"][0].clone();
    assert_eq!(ambiguous["status"], "waiting_for_process");
    assert!(native::process(first.0.id(), false).is_ok());
    duplicate.0.kill().unwrap();
    duplicate.0.wait().unwrap();
    first.0.kill().unwrap();
    first.0.wait().unwrap();
    let next = Fixture(spawn());
    let updated = call(&mut s, "service.list", json!({"id":id}), true).unwrap()["items"][0].clone();
    assert_eq!(updated["pid"], next.0.id());
    assert_eq!(updated["status"], "running");
    assert_eq!(updated["managed"], false);
    assert!(call(
        &mut s,
        "service.stop",
        json!({"id":id,"force":true,"expected_pid":old["pid"],"expected_created":old["created"]}),
        true
    )
    .is_err());
    assert!(call(
        &mut s,
        "service.configure",
        json!({"id":id,"auto_stop_minutes":1,"confirm_force":true}),
        true
    )
    .is_err());
    assert!(native::process(next.0.id(), false).is_ok());
    call(&mut s,"service.stop",json!({"id":id,"force":true,"expected_pid":updated["pid"],"expected_created":updated["created"]}),true).unwrap();
}
#[test]
fn consolidate_keeps_history_and_configuration_without_stopping_or_relaunching() {
    let dir = tempfile::tempdir().unwrap();
    let executable = fixture_binary(dir.path());
    let child = Fixture(
        Command::new(&executable)
            .args([
                "--exact",
                "service::services::tests::single_process_fixture",
                "--ignored",
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap(),
    );
    let mut s = DaemonService::open(&dir.path().join("r.sqlite")).unwrap();
    let selected = call(
        &mut s,
        "program.select",
        json!({"path":executable,"kind":"file"}),
        true,
    )
    .unwrap();
    let mut entry = selected[0].clone();
    entry["args"] = json!([]);
    call(&mut s, "program.save", json!({"items":[entry]}), true).unwrap();
    let program = call(&mut s, "program.list", json!({}), true).unwrap()[0]["id"].clone();
    let id = call(
        &mut s,
        "service.register",
        json!({"name":"stable app","pid":child.0.id(),"project_dir":dir.path()}),
        false,
    )
    .unwrap()["id"]
        .clone();
    call(
        &mut s,
        "service.configure",
        json!({"id":id,"program_id":program}),
        true,
    )
    .unwrap();
    let mut records = s.service_records().unwrap();
    let mut legacy = records[0].clone();
    legacy.id = uuid::Uuid::new_v4().to_string();
    legacy.created = "0".into();
    legacy.web_url = Some("https://example.local/".into());
    records.push(legacy);
    s.save_services(&records).unwrap();
    assert!(call(
        &mut s,
        "service.consolidate",
        json!({"program_id":program,"keep_id":id}),
        false
    )
    .is_err());
    call(
        &mut s,
        "service.consolidate",
        json!({"program_id":program,"keep_id":id}),
        true,
    )
    .unwrap();
    let listed = call(&mut s, "service.list", json!({}), true).unwrap();
    assert_eq!(listed["total"], 1);
    assert_eq!(listed["items"][0]["web_url"], "https://example.local/");
    assert_eq!(s.service_records().unwrap().len(), 2);
    assert_eq!(
        s.service_records()
            .unwrap()
            .iter()
            .filter(|r| r.archived)
            .count(),
        1
    );
    assert!(native::process(child.0.id(), false).is_ok());
    for _ in 0..3 {
        let launched = call(&mut s, "program.launch", json!({"id":program}), true).unwrap();
        assert_eq!(launched["pid"], child.0.id());
        assert_eq!(launched["service_id"], id);
    }
    assert_eq!(
        call(&mut s, "service.list", json!({}), true).unwrap()["total"],
        1
    );
}
