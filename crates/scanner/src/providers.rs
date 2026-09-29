use toolhub_core::ScanCandidate;

use crate::types::{candidate_from_file, is_executable};
use crate::ScannerProvider;

/// PATH directories — primary native discovery source.
#[derive(Default)]
pub struct PathProvider;

impl ScannerProvider for PathProvider {
    fn name(&self) -> &'static str {
        "path"
    }

    fn roots(&self) -> Vec<String> {
        std::env::var_os("PATH")
            .map(|p| {
                std::env::split_paths(&p)
                    .filter(|x| !x.as_os_str().is_empty())
                    .map(|x| x.to_string_lossy().to_string())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn scan_root(&self, root: &str) -> Result<Vec<ScanCandidate>, std::io::Error> {
        let dir = std::path::Path::new(root);
        if !dir.is_dir() {
            return Ok(vec![]);
        }
        let mut out = vec![];
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && is_executable(&path) {
                out.push(candidate_from_file(&path));
            }
        }
        Ok(out)
    }
}

/// Well-known tool directories for quick/full scan (no recursive full-disk scan).
#[derive(Default)]
pub struct KnownDirsProvider;

impl ScannerProvider for KnownDirsProvider {
    fn name(&self) -> &'static str {
        "known_dirs"
    }

    fn roots(&self) -> Vec<String> {
        let mut roots = vec![];
        if let Some(pf) = std::env::var_os("ProgramFiles") {
            roots.push(format!(
                "{}/Git/cmd",
                pf.to_string_lossy().replace('\\', "/")
            ));
            roots.push(format!(
                "{}/nodejs",
                pf.to_string_lossy().replace('\\', "/")
            ));
            roots.push(format!(
                "{}/Python*",
                pf.to_string_lossy().replace('\\', "/")
            ));
        }
        if let Some(pf86) = std::env::var_os("ProgramFiles(x86)") {
            roots.push(format!(
                "{}/Git/cmd",
                pf86.to_string_lossy().replace('\\', "/")
            ));
        }
        if let Some(home) = dirs::home_dir() {
            roots.push(home.join("scoop/shims").to_string_lossy().to_string());
            roots.push(home.join(".cargo/bin").to_string_lossy().to_string());
            roots.push(
                home.join("AppData/Local/Programs")
                    .to_string_lossy()
                    .to_string(),
            );
            roots.push("/opt/homebrew/bin".into());
            roots.push("/usr/local/bin".into());
        }
        roots.retain(|r| std::path::Path::new(r).exists());
        roots
    }

    fn full_only(&self) -> bool {
        false
    }

    fn scan_root(&self, root: &str) -> Result<Vec<ScanCandidate>, std::io::Error> {
        // Expand simple globs like Python*
        if root.contains('*') {
            return scan_glob(root);
        }
        PathProvider.scan_root(root)
    }
}

fn scan_glob(pattern: &str) -> Result<Vec<ScanCandidate>, std::io::Error> {
    let (parent, leaf) = pattern.rsplit_once(['/', '\\']).unwrap_or((".", pattern));
    let prefix = leaf.trim_end_matches('*');
    let mut out = vec![];
    if let Ok(rd) = std::fs::read_dir(parent) {
        for entry in rd.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(prefix) {
                let p = entry.path();
                if p.is_dir() {
                    let mut sub = PathProvider.scan_root(&p.to_string_lossy())?;
                    out.append(&mut sub);
                } else if is_executable(&p) {
                    out.push(candidate_from_file(&p));
                }
            }
        }
    }
    Ok(out)
}

/// Package manager metadata roots (winget/scoop/choco/homebrew) — read-only lists.
#[derive(Default)]
pub struct PackageManagerProvider;

impl ScannerProvider for PackageManagerProvider {
    fn name(&self) -> &'static str {
        "package_managers"
    }

    fn roots(&self) -> Vec<String> {
        let mut r = vec![];
        if let Some(home) = dirs::home_dir() {
            r.push(home.join("scoop").to_string_lossy().to_string());
        }
        r.push("C:/ProgramData/chocolatey/bin".into());
        r.push("/opt/homebrew/Cellar".into());
        r.push("/usr/local/Cellar".into());
        r.retain(|p| std::path::Path::new(p).exists());
        r
    }

    fn scan_root(&self, root: &str) -> Result<Vec<ScanCandidate>, std::io::Error> {
        let dir = std::path::Path::new(root);
        if !dir.is_dir() {
            return Ok(vec![]);
        }
        let mut out = vec![];
        // Depth-2 shims / bin files only (bounded).
        for entry in std::fs::read_dir(dir)?.flatten().take(200) {
            let path = entry.path();
            if path.is_file() && is_executable(&path) {
                out.push(candidate_from_file(&path));
            } else if path.is_dir() {
                if let Ok(inner) = std::fs::read_dir(&path) {
                    for e2 in inner.flatten().take(50) {
                        let p2 = e2.path();
                        if p2.is_file() && is_executable(&p2) {
                            out.push(candidate_from_file(&p2));
                        }
                    }
                }
            }
        }
        Ok(out)
    }
}

/// Windows Registry / App Paths — best-effort via `reg.exe` query is NOT used
/// (would execute a tool). We read well-known metadata dirs instead and mark
/// registry root coverage when the OS API path is unavailable in this build.
#[derive(Default)]
pub struct WindowsRegistryProvider;

impl ScannerProvider for WindowsRegistryProvider {
    fn name(&self) -> &'static str {
        "windows_registry"
    }

    fn roots(&self) -> Vec<String> {
        // App Paths mirrored locations and user program roots.
        let mut r = vec![];
        if let Some(pf) = std::env::var_os("ProgramFiles") {
            r.push(pf.to_string_lossy().to_string());
        }
        if let Some(pf86) = std::env::var_os("ProgramFiles(x86)") {
            r.push(pf86.to_string_lossy().to_string());
        }
        if let Some(la) = std::env::var_os("LOCALAPPDATA") {
            r.push(format!("{}/Programs", la.to_string_lossy()));
            r.push(format!("{}/Microsoft/WindowsApps", la.to_string_lossy()));
        }
        r.retain(|p| std::path::Path::new(p).exists());
        r
    }

    fn scan_root(&self, root: &str) -> Result<Vec<ScanCandidate>, std::io::Error> {
        // Shallow scan of install roots (depth 2) for launchers.
        let dir = std::path::Path::new(root);
        if !dir.is_dir() {
            return Ok(vec![]);
        }
        let mut out = vec![];
        for entry in walkdir::WalkDir::new(dir)
            .max_depth(2)
            .into_iter()
            .flatten()
            .take(500)
        {
            let path = entry.path();
            if path.is_file() && is_executable(path) {
                out.push(candidate_from_file(path));
            }
        }
        Ok(out)
    }
}

/// macOS Applications / Homebrew / Xcode roots.
#[derive(Default)]
pub struct MacApplicationsProvider;

impl ScannerProvider for MacApplicationsProvider {
    fn name(&self) -> &'static str {
        "macos_applications"
    }

    fn roots(&self) -> Vec<String> {
        vec![
            "/Applications".into(),
            "/usr/local/bin".into(),
            "/opt/homebrew/bin".into(),
            "/usr/bin".into(),
        ]
        .into_iter()
        .filter(|p| std::path::Path::new(p).exists())
        .collect()
    }

    fn scan_root(&self, root: &str) -> Result<Vec<ScanCandidate>, std::io::Error> {
        let dir = std::path::Path::new(root);
        if !dir.is_dir() {
            return Ok(vec![]);
        }
        let mut out = vec![];
        if root.ends_with("Applications") {
            for entry in std::fs::read_dir(dir)?.flatten().take(200) {
                let path = entry.path();
                if path.extension().map(|e| e == "app").unwrap_or(false) {
                    let mut c = ScanCandidate::from_path(path.to_string_lossy().to_string());
                    c.metadata = serde_json::json!({"bundle": true});
                    out.push(c);
                }
            }
            return Ok(out);
        }
        for entry in std::fs::read_dir(dir)?.flatten().take(500) {
            let path = entry.path();
            if path.is_file() && is_executable(&path) {
                out.push(candidate_from_file(&path));
            }
        }
        Ok(out)
    }
}
