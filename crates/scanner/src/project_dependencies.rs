//! Allowlisted project adapters. No directory traversal, package JS, npm or probes.
use crate::{candidate_from_file, record_scope, ScanReport, ScanScope};
use serde_json::{json, Value};
use std::{
    collections::BTreeSet,
    io::Read,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};
use toolhub_core::{ProjectDependency, ProjectDistOverride};

fn confined(path: &Path, root: &Path) -> Result<PathBuf, String> {
    let p = path.canonicalize().map_err(|e| e.to_string())?;
    if !p.starts_with(root) {
        return Err("linked path leaves selected project/distribution directory".into());
    }
    Ok(p)
}
fn read(path: &Path, root: &Path, max: u64) -> Result<String, String> {
    let p = confined(path, root)?;
    let f = std::fs::File::open(p).map_err(|e| e.to_string())?;
    if !f.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("expected a regular metadata file".into());
    }
    let mut bytes = Vec::new();
    f.take(max + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > max {
        return Err("metadata read limit exceeded".into());
    }
    String::from_utf8(bytes).map_err(|_| "metadata is not UTF-8".into())
}
fn manifest(path: &Path, root: &Path) -> Result<Value, String> {
    let value: Value =
        serde_json::from_str(&read(path, root, 1024 * 1024)?).map_err(|e| e.to_string())?;
    if !value.is_object() {
        return Err("package.json must be an object".into());
    }
    Ok(value)
}
fn default_binary() -> &'static str {
    if cfg!(windows) {
        "electron.exe"
    } else if cfg!(target_os = "macos") {
        "Electron.app/Contents/MacOS/Electron"
    } else {
        "electron"
    }
}
fn relative_binary(text: &str) -> Result<PathBuf, String> {
    let text = text.trim();
    // Handle foreign separators too; do not allow drive/UNC/device/ADS or traversal.
    if text.is_empty()
        || text.len() > 4096
        || text.contains([':', '\0'])
        || text.starts_with(['/', '\\'])
        || text
            .replace('\\', "/")
            .split('/')
            .any(|s| s == ".." || s.is_empty())
    {
        return Err("invalid Electron path.txt relative executable".into());
    }
    let p = PathBuf::from(text.replace('\\', "/"));
    if p.is_absolute() {
        return Err("absolute path.txt is not allowed".into());
    }
    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if !name.eq_ignore_ascii_case(if cfg!(windows) {
        "electron.exe"
    } else {
        "electron"
    }) {
        return Err("path.txt must name an Electron binary".into());
    }
    Ok(p)
}
fn observe(
    root: &Path,
    override_dist: Option<&str>,
) -> Result<Option<(ProjectDependency, Option<toolhub_core::ScanCandidate>)>, String> {
    let project = manifest(&root.join("package.json"), root)?;
    let declared = ["dependencies", "devDependencies", "optionalDependencies"]
        .iter()
        .find_map(|k| project.get(k)?.get("electron")?.as_str())
        .filter(|v| v.len() <= 4096)
        .map(str::to_owned);
    let package = root.join("node_modules/electron");
    if declared.is_none() && !package.exists() {
        return Ok(None);
    }
    let mut d = ProjectDependency {
        project_root: root.to_string_lossy().into(),
        project_name: project["name"]
            .as_str()
            .unwrap_or("Unnamed project")
            .chars()
            .take(256)
            .collect(),
        package_name: "electron".into(),
        declared_version: declared,
        package_version: None,
        package_root: package.to_string_lossy().into(),
        executable: None,
        distribution_override: override_dist.map(str::to_owned),
        binary_version: None,
        instance_id: None,
        status: "missing_package".into(),
        source: "package.json + node_modules/electron/package.json".into(),
        issue: None,
    };
    if !package.exists() {
        d.issue = Some("Electron is declared but its npm package is absent".into());
        return Ok(Some((d, None)));
    }
    let p = match confined(&package, root)
        .and_then(|p| manifest(&p.join("package.json"), root).map(|m| (p, m)))
    {
        Ok(p) => p,
        Err(e) => {
            d.status = "invalid_package".into();
            d.issue = Some(e);
            return Ok(Some((d, None)));
        }
    };
    d.package_root = p.0.to_string_lossy().into();
    let version = p.1["version"]
        .as_str()
        .filter(|v| v.len() <= 128 && toolhub_core::compare_versions(v, v).is_ok());
    if p.1["name"] != "electron" || version.is_none() {
        d.status = "invalid_package".into();
        d.issue = Some("Electron package name/version is invalid".into());
        return Ok(Some((d, None)));
    }
    d.package_version = version.map(str::to_owned);
    let result = (|| -> Result<PathBuf, String> {
        let relative = if p.0.join("path.txt").exists() {
            relative_binary(&read(&p.0.join("path.txt"), root, 4096)?)?
        } else {
            PathBuf::from(default_binary())
        };
        let dist = if let Some(custom) = override_dist {
            let custom = Path::new(custom);
            if !custom.is_absolute() {
                return Err("explicit distribution override must be absolute".into());
            }
            d.source.push_str(" + explicit distribution selection");
            custom.canonicalize().map_err(|e| e.to_string())?
        } else {
            // Ambient process settings are not a directory grant. An external override
            // must be supplied explicitly with the selected project.
            if let Some(custom) =
                std::env::var_os("ELECTRON_OVERRIDE_DIST_PATH").filter(|v| !v.is_empty())
            {
                let custom = PathBuf::from(custom);
                d.source
                    .push_str(" + ELECTRON_OVERRIDE_DIST_PATH (confined)");
                confined(&custom, root)?
            } else {
                confined(&p.0.join("dist"), root)?
            }
        };
        let binary = confined(&dist.join(relative), &dist)?;
        if !binary.is_file() || !crate::is_executable(&binary) {
            return Err("Electron binary is missing or not executable".into());
        }
        Ok(binary)
    })();
    let binary = match result {
        Ok(p) => p,
        Err(e) => {
            d.status = "missing_binary".into();
            d.issue = Some(e);
            return Ok(Some((d, None)));
        }
    };
    d.executable = Some(binary.to_string_lossy().into());
    let native = toolhub_recognizer::native_metadata::read(&binary);
    if native.format.is_none() {
        d.status = "invalid_binary".into();
        d.issue = Some("binary has no supported native executable header".into());
        return Ok(Some((d, None)));
    }
    if native
        .product
        .as_deref()
        .is_some_and(|s| !s.eq_ignore_ascii_case("Electron"))
    {
        d.status = "invalid_binary".into();
        d.issue = Some("native product metadata does not identify Electron".into());
        return Ok(Some((d, None)));
    }
    d.status = "available".into();
    d.executable = Some(binary.to_string_lossy().into());
    d.binary_version = native.version;
    if let (Some(package), Some(actual)) = (&d.package_version, &d.binary_version) {
        if toolhub_core::compare_versions(package, actual).ok() != Some(std::cmp::Ordering::Equal) {
            d.issue=Some(format!("package version {package} differs from binary version {actual}; instance uses binary version"));
        }
    }
    let mut candidate = candidate_from_file(&binary);
    d.instance_id = Some(toolhub_recognizer::stable_instance_id(
        candidate
            .canonical_path
            .as_deref()
            .unwrap_or(&candidate.path),
    ));
    candidate.version_hint = d
        .binary_version
        .clone()
        .or_else(|| d.package_version.clone());
    candidate.metadata = json!({"project_dependency":d,"scanner_scope":{"provider":"project_dependencies","root":d.project_root,"complete":true}});
    Ok(Some((d, Some(candidate))))
}

pub fn scan(
    roots: &[String],
    overrides: &[ProjectDistOverride],
    cancelled: &AtomicBool,
) -> ScanReport {
    let mut report = ScanReport::default();
    if roots.is_empty() || roots.len() > 64 || overrides.len() > 64 {
        report
            .coverage
            .limitations
            .push("select 1..64 project roots and at most 64 distribution overrides".into());
        return report;
    }
    let mut seen = BTreeSet::new();
    for selected in roots {
        if cancelled.load(Ordering::Acquire) {
            break;
        }
        let resolved = Path::new(selected).canonicalize();
        let (root, error) = match resolved {
            Ok(root) if Path::new(selected).is_absolute() && root.is_dir() => (root, None),
            _ => (
                PathBuf::from(selected),
                Some("selected project is not an accessible absolute directory".to_owned()),
            ),
        };
        if !seen.insert(toolhub_core::normalize_path(&root.to_string_lossy())) {
            continue;
        }
        let mut scope = ScanScope {
            provider: "project_dependencies".into(),
            root: root.to_string_lossy().into(),
            complete: false,
            recursive: false,
            error,
        };
        if scope.error.is_none() {
            let custom = overrides.iter().find(|o| {
                toolhub_core::normalize_path(&o.project_root)
                    == toolhub_core::normalize_path(selected)
            });
            match observe(&root, custom.map(|o| o.electron_dist.as_str())) {
                Ok(observation) => {
                    scope.complete = true;
                    if let Some((d, c)) = observation {
                        report.project_dependencies.push(d);
                        if let Some(c) = c {
                            report.candidates.push(c);
                        }
                    }
                }
                Err(e) => scope.error = Some(e),
            }
        }
        record_scope(&mut report, &scope);
    }
    report.coverage.limitations.push("Electron adapter only: exact selected project/package/path files; no recursive node_modules, ancestor/hoisted package search, npm/JS execution or automatic trust. External package links require selecting their owning project. External distribution directories require explicit overrides.".into());
    super::finish_cancel(report, cancelled)
}
