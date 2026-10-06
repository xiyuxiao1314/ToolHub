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
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "PATH root is not a readable directory",
            ));
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
            for folder in [
                "7-Zip",
                "ImageMagick*",
                "Pandoc",
                "Tesseract-OCR",
                "ffmpeg/bin",
            ] {
                roots.push(format!(
                    "{}/{}",
                    pf.to_string_lossy().replace('\\', "/"),
                    folder
                ));
            }
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
            for folder in [
                "Pandoc",
                "Tesseract-OCR",
                "ffmpeg/bin",
                "poppler/Library/bin",
            ] {
                roots.push(
                    home.join("AppData/Local/Programs")
                        .join(folder)
                        .to_string_lossy()
                        .to_string(),
                );
            }
            roots.push("/opt/homebrew/bin".into());
            roots.push("/usr/local/bin".into());
        }
        roots.retain(|r| {
            if r.contains('*') {
                r.rsplit_once(['/', '\\'])
                    .is_some_and(|(parent, _)| std::path::Path::new(parent).is_dir())
            } else {
                std::path::Path::new(r).exists()
            }
        });
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
    {
        let rd = std::fs::read_dir(parent)?;
        for entry in rd {
            let entry = entry?;
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

/// F08: real Windows Registry App Paths / uninstall metadata via winreg (read-only).
#[derive(Default)]
pub struct WindowsRegistryProvider;

#[cfg(windows)]
impl WindowsRegistryProvider {
    fn read_app_paths(&self) -> Vec<ScanCandidate> {
        use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
        use winreg::RegKey;
        let mut out = vec![];
        let roots = [
            (
                HKEY_LOCAL_MACHINE,
                r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths",
            ),
            (
                HKEY_CURRENT_USER,
                r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths",
            ),
            (
                HKEY_LOCAL_MACHINE,
                r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\App Paths",
            ),
        ];
        for (hive, sub) in roots {
            if let Ok(key) = RegKey::predef(hive).open_subkey(sub) {
                for name in key.enum_keys().flatten().take(200) {
                    if let Ok(app) = key.open_subkey(&name) {
                        if let Ok(path) = app.get_value::<String, _>("") {
                            let p = std::path::PathBuf::from(path.trim_matches('"'));
                            if p.is_file() {
                                let mut c = candidate_from_file(&p);
                                c.metadata = serde_json::json!({"source": "registry_app_paths", "name": name});
                                out.push(c);
                            }
                        }
                    }
                }
            }
        }
        out
    }

    fn read_installed_apps(&self) -> Vec<ScanCandidate> {
        use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
        use winreg::RegKey;
        let mut out = vec![];
        let subs = [
            (
                HKEY_LOCAL_MACHINE,
                r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
            ),
            (
                HKEY_CURRENT_USER,
                r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
            ),
            (
                HKEY_LOCAL_MACHINE,
                r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
            ),
        ];
        for (hive, sub) in subs {
            if let Ok(key) = RegKey::predef(hive).open_subkey(sub) {
                for name in key.enum_keys().flatten().take(300) {
                    if let Ok(app) = key.open_subkey(&name) {
                        let display: String = app.get_value("DisplayName").unwrap_or_default();
                        let loc: String = app.get_value("InstallLocation").unwrap_or_default();
                        let version: String = app.get_value("DisplayVersion").unwrap_or_default();
                        let publisher: String = app.get_value("Publisher").unwrap_or_default();
                        if !display.is_empty() {
                            let mut c = ScanCandidate::from_path(if loc.is_empty() {
                                format!("registry://uninstall/{name}")
                            } else {
                                loc.clone()
                            });
                            if !version.is_empty() {
                                c.version_hint = Some(version.clone());
                            }
                            c.metadata = serde_json::json!({
                                "source": "registry_uninstall",
                                "display_name": display,
                                "install_location": loc,
                                "display_version": version,
                                "publisher": publisher
                            });
                            out.push(c);
                        }
                    }
                }
            }
        }
        out
    }
}

impl ScannerProvider for WindowsRegistryProvider {
    fn name(&self) -> &'static str {
        "windows_registry"
    }

    fn roots(&self) -> Vec<String> {
        // F08: include real registry root plus filesystem mirrors.
        let mut r = vec!["registry:app_paths_uninstall".to_string()];
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
        r
    }

    fn scan_root(&self, root: &str) -> Result<Vec<ScanCandidate>, std::io::Error> {
        #[cfg(windows)]
        {
            if root.starts_with("registry:") {
                let mut out = self.read_app_paths();
                out.extend(self.read_installed_apps());
                return Ok(out);
            }
        }
        // Directory fallback (Program Files etc.)
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
            "/Applications/Xcode.app/Contents/Developer/usr/bin".into(),
            "/Library/Developer/CommandLineTools/usr/bin".into(),
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
                    let binaries = path.join("Contents/MacOS");
                    if binaries.is_dir() {
                        for executable in std::fs::read_dir(&binaries)?.take(256) {
                            let executable = executable?.path();
                            if executable.is_file() && is_executable(&executable) {
                                let mut candidate = candidate_from_file(&executable);
                                candidate.metadata = serde_json::json!({"source":"application_bundle","bundle_path":path,"info_plist_present":path.join("Contents/Info.plist").is_file()});
                                out.push(candidate);
                            }
                        }
                    }
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

/// Metadata directory providers are bounded filesystem observations, not package-manager commands.
#[derive(Default)]
pub struct NativeDeveloperProvider;
impl ScannerProvider for NativeDeveloperProvider {
    fn name(&self) -> &'static str {
        "developer_metadata_dirs"
    }
    fn full_only(&self) -> bool {
        true
    }
    fn roots(&self) -> Vec<String> {
        let mut roots = vec![];
        if let Some(home) = dirs::home_dir() {
            for relative in [
                ".cargo/bin",
                ".rustup/toolchains",
                ".nvm/versions",
                ".pyenv/versions",
                ".local/share/uv/python",
                "AppData/Local/Microsoft/WinGet/Packages",
                "scoop/apps",
                "miniconda3/envs",
                "anaconda3/envs",
            ] {
                let path = home.join(relative);
                if path.is_dir() {
                    roots.push(path.to_string_lossy().into_owned());
                }
            }
        }
        for key in ["ProgramFiles", "ProgramFiles(x86)"] {
            if let Some(base) = std::env::var_os(key) {
                for relative in ["Microsoft Visual Studio", "Windows Kits"] {
                    let path = std::path::Path::new(&base).join(relative);
                    if path.is_dir() {
                        roots.push(path.to_string_lossy().into_owned());
                    }
                }
            }
        }
        roots
    }
    fn scan_root(&self, root: &str) -> Result<Vec<ScanCandidate>, std::io::Error> {
        let mut out = vec![];
        for entry in walkdir::WalkDir::new(root)
            .follow_links(false)
            .max_depth(6)
            .into_iter()
            .take(4096)
        {
            let entry = entry.map_err(std::io::Error::other)?;
            if entry.file_type().is_file() && is_executable(entry.path()) {
                let mut candidate = candidate_from_file(entry.path());
                candidate.metadata["source"] = serde_json::json!("developer_metadata_directory");
                out.push(candidate);
            }
        }
        Ok(out)
    }
}

#[derive(Default)]
pub struct WslMetadataProvider;
impl ScannerProvider for WslMetadataProvider {
    fn name(&self) -> &'static str {
        "wsl_distribution_metadata"
    }
    fn full_only(&self) -> bool {
        true
    }
    fn roots(&self) -> Vec<String> {
        if cfg!(windows) {
            vec!["registry:wsl_distributions".into()]
        } else {
            vec![]
        }
    }
    fn scan_root(&self, _root: &str) -> Result<Vec<ScanCandidate>, std::io::Error> {
        #[cfg(windows)]
        {
            use winreg::{enums::HKEY_CURRENT_USER, RegKey};
            let key = match RegKey::predef(HKEY_CURRENT_USER)
                .open_subkey(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Lxss")
            {
                Ok(key) => key,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
                Err(e) => return Err(e),
            };
            let mut out = vec![];
            for name in key.enum_keys().take(256) {
                let name = name?;
                let distribution = key.open_subkey(&name)?;
                let display: String = distribution.get_value("DistributionName")?;
                let base: String = distribution.get_value("BasePath")?;
                let version: u32 = distribution.get_value("Version").unwrap_or(0);
                let mut candidate = ScanCandidate::from_path(base);
                candidate.metadata = serde_json::json!({"source":"wsl_distribution_registry","distribution":display,"wsl_version":version,"execution_interface":"not inferred from distribution metadata"});
                out.push(candidate);
            }
            Ok(out)
        }
        #[cfg(not(windows))]
        {
            Ok(vec![])
        }
    }
}
