//! Authenticated, managed user-service transport regression (owned daemon and registry).
use serde_json::json;
use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use toolhub_ipc::{read_response, write_frame, LocalStream, RpcClient};
fn binary(name: &str) -> std::path::PathBuf {
    let exe = std::env::current_exe().unwrap();
    let dir = exe.parent().unwrap().parent().unwrap();
    dir.join(if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.into()
    })
}
struct OwnedDaemon(Child);
impl Drop for OwnedDaemon {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
#[test]
fn authenticated_controller_and_shared_reconnect_and_frame_recovery() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("owned.sqlite");
    std::env::set_var("TOOLHUB_REGISTRY", &db);
    let daemon = binary("toolhubd");
    let controller = std::env::current_exe().unwrap();
    let mut child = OwnedDaemon(
        Command::new(&daemon)
            .args(["--listen", "--controller-image"])
            .arg(&controller)
            .env("TOOLHUB_REGISTRY", &db)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut stream = loop {
        if let Ok(stream) = LocalStream::connect(&daemon) {
            break stream;
        }
        assert!(Instant::now() < deadline, "service readiness");
        std::thread::sleep(Duration::from_millis(20));
    };
    stream.set_timeout(Duration::from_secs(2)).unwrap();
    stream.write_all(&1u32.to_be_bytes()).unwrap();
    stream.write_all(b"{").unwrap();
    stream.flush().unwrap();
    assert_eq!(
        read_response(&mut stream).unwrap().error.unwrap().code,
        -32700
    );
    write_frame(&mut stream, &json!({"jsonrpc":"2.0","method":"ping"})).unwrap();
    write_frame(
        &mut stream,
        &json!({"jsonrpc":"2.0","id":null,"method":"ping"}),
    )
    .unwrap();
    let null = read_response(&mut stream).unwrap();
    assert_eq!(null.id, Some(serde_json::Value::Null));
    assert!(null.result.is_some());
    let mut first = RpcClient::connect_or_start(&daemon).unwrap();
    assert!(first
        .call(
            "policy.set",
            json!({"scope":"tool","subject":"fixture","action":"deny"})
        )
        .is_ok());
    let session = first
        .call("discovery.start", json!({"agent_id":"owned.fixture"}))
        .unwrap();
    let id = session["session_id"].as_str().unwrap().to_string();
    drop(first);
    assert!(
        child.0.try_wait().unwrap().is_none(),
        "client closure must preserve service"
    );
    let mut second = RpcClient::connect_or_start(&daemon).unwrap();
    let sessions = second.call("discovery.list", json!({})).unwrap();
    assert!(
        sessions.to_string().contains(&id),
        "same authenticated image principal must survive reconnect: {sessions}"
    );
    let ordinary = Command::new(binary("toolhub"))
        .args(["--json", "policy", "set", "tool", "fixture", "allow"])
        .env("TOOLHUB_REGISTRY", &db)
        .env("TOOLHUB_PRINCIPAL", "local.admin")
        .env("TOOLHUB_ADMIN", "1")
        .env("TOOLHUBD_BIN", &daemon)
        .output()
        .unwrap();
    assert!(!ordinary.status.success());
    assert!(
        String::from_utf8_lossy(&ordinary.stdout).contains("denied"),
        "ordinary CLI must not become controller: {}",
        String::from_utf8_lossy(&ordinary.stdout)
    );
    let mut duplicate = Command::new(&daemon)
        .arg("--listen")
        .env("TOOLHUB_REGISTRY", &db)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let duplicate_deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(status) = duplicate.try_wait().unwrap() {
            assert!(!status.success());
            break;
        }
        if Instant::now() > duplicate_deadline {
            let _ = duplicate.kill();
            panic!("duplicate daemon did not reject service lock");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(second.call("ping", json!({})).is_ok());
    child.0.kill().unwrap();
    child.0.wait().unwrap();
}
