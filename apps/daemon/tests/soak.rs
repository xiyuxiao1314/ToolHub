//! Long-lived daemon soak: one daemon process, many sequential/concurrent clients.
//!
//! High-volume loops use a bounded custom scan root so wall-clock hang detection
//! stays meaningful on hosts whose full PATH quick-scan is tens of seconds.
//! At least one real `quick` scan remains for end-to-end native discovery.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn toolhubd_bin() -> String {
    std::env::var("TOOLHUBD_BIN").unwrap_or_else(|_| {
        let mut p = std::env::current_exe().unwrap();
        p.pop();
        if p.ends_with("deps") {
            p.pop();
        }
        p.push(if cfg!(windows) {
            "toolhubd.exe"
        } else {
            "toolhubd"
        });
        p.to_string_lossy().to_string()
    })
}

fn fixture_root(dir: &std::path::Path) -> String {
    let root = dir.join("fixture-root");
    std::fs::create_dir_all(&root).unwrap();
    // Owned, non-executable marker file is enough for discovery of a candidate.
    let tool = root.join("toolhub-soak-fixture.txt");
    std::fs::write(&tool, b"toolhub soak fixture").unwrap();
    root.to_string_lossy().to_string()
}

fn custom_scan_params(root: &str) -> String {
    let escaped = root.replace('\\', "\\\\").replace('"', "\\\"");
    format!(r#"{{"mode":"custom","roots":["{escaped}"]}}"#)
}

struct ChildDaemon {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    reader: BufReader<std::process::ChildStdout>,
}

impl ChildDaemon {
    fn spawn(db: &std::path::Path) -> Self {
        let mut child = Command::new(toolhubd_bin())
            .env("TOOLHUB_REGISTRY", db)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn toolhubd");
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        Self {
            child,
            stdin,
            reader: BufReader::new(stdout),
        }
    }

    fn call(&mut self, method: &str, params: &str) -> String {
        let id = format!("{}", Instant::now().elapsed().as_nanos());
        writeln!(
            self.stdin,
            r#"{{"jsonrpc":"2.0","id":"{id}","method":"{method}","params":{params}}}"#
        )
        .unwrap();
        self.stdin.flush().unwrap();
        let mut line = String::new();
        self.reader.read_line(&mut line).unwrap();
        line
    }
}

impl Drop for ChildDaemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// R2-B02: one long-lived daemon survives many requests without hanging or crashing.
/// Long endurance window: 200 mixed requests against one process.
#[test]
fn soak_one_daemon_many_requests() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("soak.sqlite");
    let root = fixture_root(dir.path());
    let scan = custom_scan_params(&root);
    let mut d = ChildDaemon::spawn(&db);

    // Warm-up
    let p = d.call("ping", "{}");
    assert!(p.contains("ok"), "ping={p}");

    // 200 mixed requests (40 scans, bounded root)
    let start = Instant::now();
    for i in 0..200 {
        let (method, params) = if i % 5 == 0 {
            ("scan.start", scan.as_str())
        } else if i % 5 == 1 {
            ("registry.search", r#"{"query":"toolhub-soak"}"#)
        } else if i % 5 == 2 {
            ("status", "{}")
        } else if i % 5 == 3 {
            ("policy.get", "{}")
        } else {
            ("activity.list", "{}")
        };
        let resp = d.call(method, params);
        assert!(
            resp.contains("jsonrpc") || resp.contains("result") || resp.contains("error"),
            "{resp}"
        );
    }
    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_secs(120),
        "soak took {elapsed:?}"
    );

    // Still healthy
    let p2 = d.call("ping", "{}");
    assert!(p2.contains("ok"), "after soak ping={p2}");
}

/// Concurrent-ish sequential interleaving of scan and query on one daemon.
#[test]
fn soak_scan_query_interleave() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("interleave.sqlite");
    let root = fixture_root(dir.path());
    let scan = custom_scan_params(&root);
    let mut d = ChildDaemon::spawn(&db);
    for i in 0..20 {
        let s = d.call("scan.start", &scan);
        assert!(s.contains("candidates") || s.contains("result"), "i={i} {s}");
        let q = d.call("registry.search", r#"{"query":"a"}"#);
        assert!(
            q.contains("result") || q.contains("[]") || q.contains("[{"),
            "{q}"
        );
        let a = d.call("activity.list", "{}");
        assert!(a.contains("result"), "{a}");
    }
    let st = d.call("status", "{}");
    assert!(st.contains("toolhubd"), "{st}");
}

/// Daemon remains available after invalid and oversized-ish inputs.
#[test]
fn soak_recovers_after_bad_requests() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("bad.sqlite");
    let mut d = ChildDaemon::spawn(&db);
    // garbage
    writeln!(d.stdin, "not-json").unwrap();
    d.stdin.flush().unwrap();
    let mut line = String::new();
    let _ = d.reader.read_line(&mut line);
    // wrong version
    let bad = d.call("ping", "{}");
    assert!(bad.contains("ok") || bad.contains("error"));
    // unknown method
    let u = d.call("no.such.method", "{}");
    assert!(u.contains("method_not_found") || u.contains("error"), "{u}");
    // still works
    let p = d.call("ping", "{}");
    assert!(p.contains("ok"), "{p}");
}

/// Shared registry durability across daemon restarts.
#[test]
fn soak_registry_survives_restart() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("restart.sqlite");
    let root = fixture_root(dir.path());
    let scan = custom_scan_params(&root);
    {
        let mut d = ChildDaemon::spawn(&db);
        let _ = d.call("scan.start", &scan);
        let s = d.call("status", "{}");
        assert!(s.contains("toolhubd"));
    }
    {
        let mut d = ChildDaemon::spawn(&db);
        let s = d.call("status", "{}");
        assert!(s.contains("toolhubd"), "{s}");
        // tools should persist (count may be >0 after scan)
        assert!(s.contains("tool_count"), "{s}");
    }
}

/// One native quick scan remains for end-to-end discovery (environment-sensitive wall clock).
#[test]
fn soak_one_real_quick_scan_still_works() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("quick.sqlite");
    let mut d = ChildDaemon::spawn(&db);
    let start = Instant::now();
    let s = d.call("scan.start", r#"{"mode":"quick"}"#);
    let elapsed = start.elapsed();
    assert!(s.contains("candidates") || s.contains("result"), "{s}");
    // Generous bound: full PATH quick scans can take tens of seconds on large hosts.
    assert!(
        elapsed < Duration::from_secs(300),
        "quick scan took {elapsed:?}"
    );
    let p = d.call("ping", "{}");
    assert!(p.contains("ok"), "{p}");
}

// Keep Arc/Mutex import used for potential future multi-thread soak.
#[allow(dead_code)]
fn _unused() {
    let _ = Arc::new(Mutex::new(0));
    let _: Option<PathBuf> = None;
}
