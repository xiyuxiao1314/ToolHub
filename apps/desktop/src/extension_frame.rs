//! An on-demand loopback document origin. It serves no disk paths or RPC endpoints.
//! Tauri treats this origin as remote, so the local application's native ACL is absent.
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::time::Duration;
use toolhub_skills::extension::{ExtensionPackage, ExtensionView};

#[derive(Clone)]
struct Session {
    id: String,
    digest: String,
    view: String,
}
#[derive(Default)]
pub struct FrameServer {
    sessions: Arc<Mutex<BTreeMap<String, Session>>>,
    origin: Mutex<Option<String>>,
}
impl FrameServer {
    fn start(&self) -> Result<String, String> {
        let mut origin = self.origin.lock().map_err(|_| "frame origin lock")?;
        if let Some(value) = &*origin {
            return Ok(value.clone());
        }
        let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| e.to_string())?;
        let address = format!(
            "http://127.0.0.1:{}",
            listener.local_addr().map_err(|e| e.to_string())?.port()
        );
        let sessions = Arc::clone(&self.sessions);
        std::thread::spawn(move || {
            let running = Arc::new(AtomicUsize::new(0));
            for stream in listener.incoming().flatten() {
                if running.load(Ordering::SeqCst) >= 8 {
                    continue;
                }
                running.fetch_add(1, Ordering::SeqCst);
                let sessions = Arc::clone(&sessions);
                let running = Arc::clone(&running);
                std::thread::spawn(move || {
                    let _ = serve(stream, sessions);
                    running.fetch_sub(1, Ordering::SeqCst);
                });
            }
        });
        *origin = Some(address.clone());
        Ok(address)
    }
}
fn package(id: &str, digest: &str) -> Result<ExtensionPackage, String> {
    let mut client = toolhub_ipc::RpcClient::connect_or_start(&crate::daemon_bin()?)
        .map_err(|e| e.to_string())?;
    let rows = client
        .call("extension.list", serde_json::json!({}))
        .map_err(|e| e.to_string())?;
    let row = rows
        .as_array()
        .and_then(|rows| {
            rows.iter().find(|r| {
                r["enabled"] == true
                    && r["package"]["id"].as_str() == Some(id)
                    && r["digest"].as_str() == Some(digest)
            })
        })
        .ok_or("extension disabled, updated or removed")?;
    ExtensionPackage::parse(row["package"].to_string().as_bytes())
}
#[tauri::command]
pub fn extension_view_url(
    state: tauri::State<'_, FrameServer>,
    id: String,
    expected_digest: String,
    view_id: String,
    token: String,
) -> Result<String, String> {
    if !valid_token(&token) {
        return Err("invalid frame session".into());
    }
    let p = package(&id, &expected_digest)?;
    if !p.views.iter().any(|v| v.id == view_id) {
        return Err("extension view unavailable".into());
    }
    let origin = state.start()?;
    let mut sessions = state.sessions.lock().map_err(|_| "frame session lock")?;
    if sessions.len() >= 32 {
        if let Some(first) = sessions.keys().next().cloned() {
            sessions.remove(&first);
        }
    }
    sessions.insert(
        token.clone(),
        Session {
            id,
            digest: expected_digest,
            view: view_id,
        },
    );
    Ok(format!("{origin}/view/{token}/index.html"))
}
fn valid_token(value: &str) -> bool {
    value.len() == 36
        && value.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
        && [8, 13, 18, 23].iter().all(|i| value.as_bytes()[*i] == b'-')
}
fn route(path: &str) -> Option<(&str, &str)> {
    let parts = path.split('/').collect::<Vec<_>>();
    if parts.len() != 4
        || parts[0] != ""
        || parts[1] != "view"
        || !valid_token(parts[2])
        || !["index.html", "view.css", "view.js"].contains(&parts[3])
    {
        return None;
    }
    Some((parts[2], parts[3]))
}
fn serve(
    mut stream: TcpStream,
    sessions: Arc<Mutex<BTreeMap<String, Session>>>,
) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let mut line = vec![];
    BufReader::new(stream.try_clone()?)
        .take(2048)
        .read_until(b'\n', &mut line)?;
    let request = String::from_utf8_lossy(&line);
    let fields = request.split_whitespace().collect::<Vec<_>>();
    let result = (|| -> Result<(String, &'static str), String> {
        if fields.len() != 3 || fields[0] != "GET" || fields[2] != "HTTP/1.1" {
            return Err("invalid request".into());
        }
        let (token, asset) = route(fields[1]).ok_or("invalid route")?;
        let session = sessions
            .lock()
            .map_err(|_| "frame session lock")?
            .get(token)
            .cloned()
            .ok_or("expired frame session")?;
        // Each resource load revalidates current installation and content digest.
        let p = package(&session.id, &session.digest)?;
        let view = p
            .views
            .iter()
            .find(|v| v.id == session.view)
            .ok_or("view removed")?;
        Ok(render(view, asset, token))
    })();
    let (status, body, mime) = match result {
        Ok((body, mime)) => ("200 OK", body, mime),
        Err(_) => (
            "403 Forbidden",
            "Extension page unavailable".into(),
            "text/plain; charset=utf-8",
        ),
    };
    let document_origin = format!("http://127.0.0.1:{}", stream.local_addr()?.port());
    let policy = content_policy(&document_origin, fields.get(1).copied());
    let headers=format!("HTTP/1.1 {status}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: {policy}\r\n\r\n",body.len());
    stream.write_all(headers.as_bytes())?;
    stream.write_all(body.as_bytes())?;
    stream.flush()
}
fn content_policy(origin: &str, path: Option<&str>) -> String {
    let (scripts, styles) = path
        .and_then(route)
        .map(|(token, _)| {
            (
                format!("{origin}/view/{token}/view.js"),
                format!("{origin}/view/{token}/view.css"),
            )
        })
        .unwrap_or(("'none'".into(), "'none'".into()));
    format!("default-src 'none'; script-src {scripts}; style-src {styles}; img-src data:; connect-src 'none'; frame-src 'none'; form-action 'none'; base-uri 'none'; sandbox allow-scripts; frame-ancestors http://tauri.localhost tauri: http://localhost:5173")
}
fn render(view: &ExtensionView, asset: &str, token: &str) -> (String, &'static str) {
    match asset {
        "index.html"=>(format!("<!doctype html><html><head><meta charset=\"utf-8\"><link rel=\"stylesheet\" href=\"view.css\"><script defer src=\"view.js\"></script></head><body>{}</body></html>",view.html),"text/html; charset=utf-8"),
        "view.css"=>(format!(":root{{--bg:#f4f6fb;--panel:#ffffff;--panel-soft:#f8fafc;--ink:#0f172a;--ink-2:#334155;--muted:#64748b;--line:#e2e8f0;--accent:#2563eb;--accent-soft:#eff6ff;--radius:14px}}\n{}",view.css),"text/css; charset=utf-8"),
        _=>(format!("{}\n{}",include_str!("../../../extensions/sdk/runtime.js").replace("__TOKEN__",&serde_json::to_string(token).unwrap()),view.script),"text/javascript; charset=utf-8")
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn document_routes_never_resolve_disk_paths() {
        let token = "d3f8d2e1-7992-4a3a-bbe0-2098caa9a553";
        assert!(route(&format!("/view/{token}/view.js")).is_some());
        for path in [
            "/C:/Users/AA/secrets",
            "/view/../view.js",
            "/view/no/view.js",
            "/view/d3f8d2e1-7992-4a3a-bbe0-2098caa9a553/view.js?path=x",
        ] {
            assert!(route(path).is_none());
        }
    }
    #[test]
    fn documents_can_load_only_their_own_session_code_and_style() {
        let token = "d3f8d2e1-7992-4a3a-bbe0-2098caa9a553";
        let csp = content_policy(
            "http://127.0.0.1:19399",
            Some(&format!("/view/{token}/index.html")),
        );
        assert!(csp.contains(&format!(
            "script-src http://127.0.0.1:19399/view/{token}/view.js;"
        )));
        assert!(!csp.contains("127.0.0.1:*"));
        assert!(csp.contains("connect-src 'none'"));
        assert!(
            content_policy("http://127.0.0.1:19399", Some("/bad")).contains("script-src 'none'")
        );
    }
    #[test]
    fn code_is_served_only_as_a_separate_document_resource() {
        let v = ExtensionView {
            id: "community.demo.page".into(),
            label: "Demo".into(),
            icon: "x".into(),
            order: 1,
            html: "<h1>Demo</h1>".into(),
            css: "body{margin:0}".into(),
            script: "document.title='Demo';".into(),
        };
        let (html, _) = render(&v, "index.html", "test");
        assert!(!html.contains("document.title"));
        assert!(html.contains("view.js"));
        let (js, _) = render(&v, "view.js", "test");
        assert!(js.contains("Object.freeze"));
        assert!(js.contains("document.title='Demo'"));
        assert!(!js.contains("__TOKEN__"));
    }
}
