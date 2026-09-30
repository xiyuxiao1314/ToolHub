//! ToolHub Desktop — local shared-daemon UI shell.
//!
//! Serves a self-contained SPA on a user-scoped localhost port and proxies
//! JSON-RPC to the same daemon registry/policy used by CLI and MCP.
//! This is the B02 desktop product surface (nine pages) without a second registry.

use std::io::{BufRead, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

fn main() -> std::io::Result<()> {
    let port: u16 = std::env::var("TOOLHUB_DESKTOP_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(18765);
    // Bind loopback only — no public listener.
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    eprintln!("toolhub-desktop listening on http://127.0.0.1:{port}");
    // Open default browser (best-effort).
    #[cfg(windows)]
    {
        let _ = Command::new("cmd")
            .args(["/c", "start", &format!("http://127.0.0.1:{port}")])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }

    let daemon = Arc::new(Mutex::new(DaemonHandle::spawn()?));
    for stream in listener.incoming() {
        let stream = stream?;
        let daemon = Arc::clone(&daemon);
        std::thread::spawn(move || {
            let _ = handle_client(stream, daemon);
        });
    }
    Ok(())
}

struct DaemonHandle {
    #[allow(dead_code)]
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    reader: std::io::BufReader<std::process::ChildStdout>,
}

impl DaemonHandle {
    fn spawn() -> std::io::Result<Self> {
        let bin = std::env::var("TOOLHUBD_BIN").unwrap_or_else(|_| "toolhubd".into());
        let mut child = Command::new(bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        Ok(Self {
            child,
            stdin,
            reader: std::io::BufReader::new(stdout),
        })
    }

    fn call(&mut self, body: &str) -> std::io::Result<String> {
        writeln!(self.stdin, "{body}")?;
        self.stdin.flush()?;
        let mut line = String::new();
        self.reader.read_line(&mut line)?;
        Ok(line)
    }
}

fn handle_client(
    mut stream: TcpStream,
    daemon: Arc<Mutex<DaemonHandle>>,
) -> std::io::Result<()> {
    let mut buf = vec![0u8; 65536];
    let n = stream.read(&mut buf)?;
    let req = String::from_utf8_lossy(&buf[..n]).to_string();
    let first = req.lines().next().unwrap_or("");
    let parts: Vec<&str> = first.split_whitespace().collect();
    if parts.len() < 2 {
        return respond(&mut stream, 400, "text/plain", "bad request");
    }
    let path = parts[1];
    if path == "/" || path.starts_with("/index") {
        respond(&mut stream, 200, "text/html; charset=utf-8", INDEX_HTML)
    } else if path == "/app.js" {
        respond(&mut stream, 200, "application/javascript", APP_JS)
    } else if path == "/app.css" {
        respond(&mut stream, 200, "text/css", APP_CSS)
    } else if path == "/rpc" {
        // POST body after blank line
        let body = req
            .split("\r\n\r\n")
            .nth(1)
            .or_else(|| req.split("\n\n").nth(1))
            .unwrap_or("{}")
            .to_string();
        let mut d = daemon.lock().unwrap();
        let out = d.call(&body)?;
        respond(&mut stream, 200, "application/json", &out)
    } else {
        respond(&mut stream, 404, "text/plain", "not found")
    }
}

fn respond(stream: &mut TcpStream, code: u16, ctype: &str, body: &str) -> std::io::Result<()> {
    let status = if code == 200 {
        "200 OK"
    } else if code == 404 {
        "404 Not Found"
    } else {
        "400 Bad Request"
    };
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body.as_bytes())?;
    stream.flush()
}

const INDEX_HTML: &str = r#"<!doctype html>
<html lang="zh-CN"><head><meta charset="utf-8"><title>ToolHub</title>
<link rel="stylesheet" href="/app.css"></head>
<body>
<header><h1>ToolHub</h1><nav id="nav"></nav></header>
<main id="app"><p class="muted">加载中…</p></main>
<script src="/app.js"></script>
</body></html>"#;

const APP_CSS: &str = r#"
:root { --bg:#0f1419; --panel:#1a2332; --ink:#e7ecf3; --muted:#8b9bb4; --accent:#3d8bfd; }
* { box-sizing:border-box; }
body { margin:0; font:14px/1.5 system-ui,'Segoe UI',sans-serif; background:var(--bg); color:var(--ink); }
header { display:flex; gap:16px; align-items:center; padding:12px 20px; background:var(--panel); }
h1 { font-size:18px; margin:0; }
nav button { background:none; border:0; color:var(--muted); padding:8px 10px; cursor:pointer; }
nav button.active { color:var(--accent); border-bottom:2px solid var(--accent); }
main { padding:20px; }
.card { background:var(--panel); border-radius:10px; padding:16px; margin-bottom:12px; }
.muted { color:var(--muted); }
table { width:100%; border-collapse:collapse; }
td,th { text-align:left; padding:6px 8px; border-bottom:1px solid #2a3548; }
input,select { background:#0c1218; color:var(--ink); border:1px solid #2a3548; border-radius:6px; padding:8px; }
button.primary { background:var(--accent); color:#fff; border:0; border-radius:6px; padding:8px 12px; cursor:pointer; }
@media (prefers-reduced-motion: reduce) { * { transition:none!important; animation:none!important; } }
"#;

const APP_JS: &str = r#"
const pages = ['Overview','Tools','Environments','Capabilities','Skills','Agents','Activity','Security','Settings'];
let current = localStorage.getItem('th.page') || 'Overview';

async function rpc(method, params={}) {
  const r = await fetch('/rpc', {method:'POST', body: JSON.stringify({jsonrpc:'2.0',id:1,method,params})});
  return r.json();
}

function el(html){ const d=document.createElement('div'); d.innerHTML=html.trim(); return d.firstChild; }

async function render() {
  const nav = document.getElementById('nav');
  nav.innerHTML = pages.map(p => `<button class="${p===current?'active':''}" data-p="${p}">${p}</button>`).join('');
  nav.querySelectorAll('button').forEach(b => b.onclick = () => { current=b.dataset.p; localStorage.setItem('th.page', current); render(); });
  const app = document.getElementById('app');
  app.innerHTML = '<p class="muted">加载中…</p>';
  try {
    if (current==='Overview') {
      const st = await rpc('status');
      const act = await rpc('activity.list').catch(()=>[]);
      app.innerHTML = `
        <div class="card"><h2>Overview</h2>
        <p>工具数：${st.result?.tool_count ?? 0}　候选：${st.result?.candidate_count ?? 0}</p>
        <p class="muted">最近扫描：${st.result?.last_scan || '从未'}</p>
        <button class="primary" id="scan">快速扫描</button>
        <div id="scanOut"></div></div>
        <div class="card"><h3>Activity</h3><pre>${JSON.stringify(act.result||act,null,2)}</pre></div>`;
      document.getElementById('scan').onclick = async () => {
        document.getElementById('scanOut').textContent = '扫描中…';
        const r = await rpc('scan.start',{mode:'quick'});
        document.getElementById('scanOut').textContent = JSON.stringify(r.result||r,null,2);
      };
    } else if (current==='Tools') {
      const q = prompt('搜索工具','python') || '';
      const r = await rpc('registry.search',{query:q});
      const rows = (r.result||[]).map(i=>`<tr><td>${i.name}</td><td>${i.version||''}</td><td><code>${i.path}</code></td></tr>`).join('');
      app.innerHTML = `<div class="card"><h2>Tools</h2><table><tr><th>名称</th><th>版本</th><th>路径</th></tr>${rows||'<tr><td colspan=3 class=muted>无结果</td></tr>'}</table></div>`;
    } else if (current==='Environments') {
      const r = await rpc('environment.list');
      app.innerHTML = `<div class="card"><h2>Environments</h2><pre>${JSON.stringify(r.result||r,null,2)}</pre></div>`;
    } else if (current==='Capabilities') {
      const c = prompt('能力 ID','language.python.execute')||'';
      const r = await rpc('resolve.capability',{capability:c});
      app.innerHTML = `<div class="card"><h2>Capabilities</h2><pre>${JSON.stringify(r.result||r,null,2)}</pre></div>`;
    } else if (current==='Skills') {
      const r = await rpc('skill.list');
      app.innerHTML = `<div class="card"><h2>Skills</h2><pre>${JSON.stringify(r.result||r,null,2)}</pre></div>`;
    } else if (current==='Agents') {
      const r = await rpc('agent.list');
      app.innerHTML = `<div class="card"><h2>Agents</h2><pre>${JSON.stringify(r.result||r,null,2)}</pre></div>`;
    } else if (current==='Activity') {
      const r = await rpc('activity.list');
      app.innerHTML = `<div class="card"><h2>Activity</h2><pre>${JSON.stringify(r.result||r,null,2)}</pre></div>`;
    } else if (current==='Security') {
      const r = await rpc('policy.get');
      app.innerHTML = `<div class="card"><h2>Security</h2><pre>${JSON.stringify(r.result||r,null,2)}</pre></div>`;
    } else {
      const st = await rpc('status');
      app.innerHTML = `<div class="card"><h2>Settings</h2>
        <p>协议：${st.result?.protocol_version||''}</p>
        <p>注册表：<code>${st.result?.registry_path||''}</code></p>
        <p class="muted">版本独立：app 0.2.0 / core 0.2.0 / recognition resources local</p></div>`;
    }
  } catch(e) {
    app.innerHTML = `<div class="card"><p class="muted">错误：${e}</p></div>`;
  }
}
render();
"#;
