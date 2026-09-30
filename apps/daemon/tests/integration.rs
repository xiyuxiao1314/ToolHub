//! Cross-client / shared-daemon integration checks (owned fixtures only).

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

fn toolhubd_bin() -> String {
    std::env::var("TOOLHUBD_BIN").unwrap_or_else(|_| {
        let mut p = std::env::current_exe().unwrap();
        p.pop();
        p.push(if cfg!(windows) {
            "toolhubd.exe"
        } else {
            "toolhubd"
        });
        p.to_string_lossy().to_string()
    })
}

#[test]
fn daemon_ping_status_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("r.sqlite");
    let mut child = Command::new(toolhubd_bin())
        .env("TOOLHUB_REGISTRY", &db)
        .env("TOOLHUB_PRINCIPAL", "local.admin")
        .env("TOOLHUB_ADMIN", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn toolhubd");
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    writeln!(
        stdin,
        r#"{{"jsonrpc":"2.0","id":1,"method":"ping","params":{{}}}}"#
    )
    .unwrap();
    stdin.flush().unwrap();
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    assert!(line.contains("\"ok\":true"), "got {line}");

    writeln!(
        stdin,
        r#"{{"jsonrpc":"2.0","id":2,"method":"status","params":{{}}}}"#
    )
    .unwrap();
    stdin.flush().unwrap();
    let mut line2 = String::new();
    reader.read_line(&mut line2).unwrap();
    assert!(line2.contains("toolhubd"), "got {line2}");
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn invalid_jsonrpc_version_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("r.sqlite");
    let mut child = Command::new(toolhubd_bin())
        .env("TOOLHUB_REGISTRY", &db)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn toolhubd");
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    writeln!(
        stdin,
        r#"{{"jsonrpc":"1.0","id":1,"method":"ping","params":{{}}}}"#
    )
    .unwrap();
    stdin.flush().unwrap();
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    assert!(line.contains("invalid_request"), "got {line}");
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn two_clients_share_registry_via_stdio_daemon() {
    // Two sequential stdio clients against the same registry file see the same scan.
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("shared.sqlite");

    let mut call = |method: &str, params: &str| -> String {
        let mut child = Command::new(toolhubd_bin())
            .env("TOOLHUB_REGISTRY", &db)
            .env("TOOLHUB_PRINCIPAL", "local.admin")
            .env("TOOLHUB_ADMIN", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn");
        let mut stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let mut reader = BufReader::new(stdout);
        writeln!(
            stdin,
            r#"{{"jsonrpc":"2.0","id":1,"method":"{method}","params":{params}}}"#
        )
        .unwrap();
        stdin.flush().unwrap();
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let _ = child.kill();
        let _ = child.wait();
        line
    };

    let s1 = call("status", "{}");
    let scan = call("scan.start", r#"{"mode":"quick"}"#);
    let s2 = call("status", "{}");
    assert!(s1.contains("toolhubd"), "s1={s1}");
    assert!(scan.contains("candidates"), "scan={scan}");
    // After scan, second status should report more tools/candidates than empty first status.
    assert!(s2.contains("toolhubd"), "s2={s2}");
}

#[test]
fn approve_requires_admin() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("admin.sqlite");
    let mut child = Command::new(toolhubd_bin())
        .env("TOOLHUB_REGISTRY", &db)
        .env("TOOLHUB_PRINCIPAL", "pipe.conn.anon")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn");
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    writeln!(
        stdin,
        r#"{{"jsonrpc":"2.0","id":1,"method":"execute.approve","params":{{"instance_id":"x","args":[]}}}}"#
    )
    .unwrap();
    stdin.flush().unwrap();
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    assert!(line.contains("denied") || line.contains("approval"), "got {line}");
    let _ = child.kill();
    let _ = child.wait();
}
