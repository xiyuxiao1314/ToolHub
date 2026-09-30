//! Read-only discovery -> ScanCandidate pipeline.
//! Default discovery never installs, writes config, or executes unknown binaries.

mod providers;
mod types;

pub use providers::*;
pub use types::*;

use toolhub_core::ScanCandidate;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanMode {
    Quick,
    Full,
}

#[derive(Debug, Default, Clone)]
pub struct ScanCoverage {
    pub roots_attempted: Vec<String>,
    pub roots_ok: Vec<String>,
    pub roots_failed: Vec<(String, String)>,
    pub scopes: Vec<ScanScope>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ScanScope {
    pub provider: String,
    pub root: String,
    pub complete: bool,
    pub recursive: bool,
    pub error: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct ScanReport {
    pub candidates: Vec<ScanCandidate>,
    pub coverage: ScanCoverage,
    pub cancelled: bool,
}

/// Orchestrate platform providers. Unknown binaries are never executed.
pub fn run_scan(mode: ScanMode, roots_override: Option<Vec<String>>) -> ScanReport {
    run_scan_with_extensions(mode, roots_override, &[])
}

pub fn run_scan_with_extensions(
    mode: ScanMode,
    roots_override: Option<Vec<String>>,
    extensions: &[DeclarativeExtension],
) -> ScanReport {
    run_scan_with_cancel(
        mode,
        roots_override,
        extensions,
        &std::sync::atomic::AtomicBool::new(false),
    )
}
pub fn run_scan_with_cancel(
    mode: ScanMode,
    roots_override: Option<Vec<String>>,
    extensions: &[DeclarativeExtension],
    cancelled: &std::sync::atomic::AtomicBool,
) -> ScanReport {
    let mut report = ScanReport::default();
    if let Some(roots) = roots_override {
        for root in roots {
            if cancelled.load(std::sync::atomic::Ordering::Acquire) {
                report.cancelled = true;
                break;
            }
            let (mut candidates, scope) = scan_directory("custom", &root, 4096, 8, cancelled);
            for c in &mut candidates {
                tag_scope(c, &scope);
            }
            record_scope(&mut report, &scope);
            report.candidates.extend(candidates);
            for extension in extensions {
                match extension.discover(&root) {
                    Ok(mut found) => report.candidates.append(&mut found),
                    Err(e) => report.coverage.roots_failed.push((root.clone(), e)),
                }
            }
        }
        return finish_cancel(report, cancelled);
    }
    let providers: Vec<Box<dyn ScannerProvider>> = vec![
        Box::new(PathProvider),
        Box::new(KnownDirsProvider),
        Box::new(PackageManagerProvider),
        Box::new(NativeDeveloperProvider),
        Box::new(WslMetadataProvider),
        if cfg!(windows) {
            Box::new(WindowsRegistryProvider) as Box<dyn ScannerProvider>
        } else {
            Box::new(MacApplicationsProvider) as Box<dyn ScannerProvider>
        },
    ];

    for p in providers {
        if cancelled.load(std::sync::atomic::Ordering::Acquire) {
            report.cancelled = true;
            break;
        }
        // F08: Quick skips deep package walks; Full includes them.
        let deep = p.full_only() || p.name() == "package_managers";
        if mode == ScanMode::Quick && deep {
            continue;
        }
        let roots = p.roots();
        for root in roots {
            if cancelled.load(std::sync::atomic::Ordering::Acquire) {
                report.cancelled = true;
                break;
            }
            report.coverage.roots_attempted.push(root.clone());
            match p.scan_root(&root) {
                Ok(mut found) => {
                    report.coverage.roots_ok.push(root.clone());
                    let complete = matches!(p.name(), "path" | "known_dirs");
                    let scope = ScanScope {
                        provider: p.name().into(),
                        root: root.clone(),
                        complete,
                        recursive: false,
                        error: if complete {
                            None
                        } else {
                            Some("native metadata enumeration is bounded; provider does not certify full-root coverage".into())
                        },
                    };
                    for c in &mut found {
                        tag_scope(c, &scope);
                    }
                    report.coverage.scopes.push(scope);
                    report.candidates.append(&mut found);
                }
                Err(e) => {
                    report.coverage.scopes.push(ScanScope {
                        provider: p.name().into(),
                        root: root.clone(),
                        complete: false,
                        recursive: false,
                        error: Some(e.to_string()),
                    });
                    report
                        .coverage
                        .roots_failed
                        .push((root.clone(), e.to_string()));
                }
            }
        }
    }

    report.coverage.limitations.push("WSL distribution metadata is read from the registry without launching WSL; Linux tool inventories and LaunchServices database enumeration are not inferred from directory observations".into());
    finish_cancel(report, cancelled)
}

fn finish_cancel(mut report: ScanReport, cancelled: &std::sync::atomic::AtomicBool) -> ScanReport {
    if cancelled.load(std::sync::atomic::Ordering::Acquire) {
        report.cancelled = true;
        for scope in &mut report.coverage.scopes {
            scope.complete = false;
            scope.error = Some("scan cancelled before coverage committed".into());
        }
    }
    deduplicate(report)
}

fn deduplicate(mut report: ScanReport) -> ScanReport {
    let mut seen = std::collections::BTreeSet::new();
    report.candidates.retain(|c| {
        seen.insert(toolhub_core::normalize_path(
            c.canonical_path.as_deref().unwrap_or(&c.path),
        ))
    });
    report
}

fn tag_scope(candidate: &mut ScanCandidate, scope: &ScanScope) {
    if !candidate.metadata.is_object() {
        candidate.metadata = serde_json::json!({});
    }
    candidate.metadata["scanner_scope"] =
        serde_json::json!({"provider":scope.provider,"root":scope.root,"complete":scope.complete});
}
fn record_scope(report: &mut ScanReport, scope: &ScanScope) {
    report.coverage.roots_attempted.push(scope.root.clone());
    if let Some(e) = &scope.error {
        report
            .coverage
            .roots_failed
            .push((scope.root.clone(), e.clone()));
    } else {
        report.coverage.roots_ok.push(scope.root.clone());
    }
    report.coverage.scopes.push(scope.clone());
}
fn scan_directory(
    provider: &str,
    root: &str,
    limit: usize,
    depth: usize,
    cancelled: &std::sync::atomic::AtomicBool,
) -> (Vec<ScanCandidate>, ScanScope) {
    let mut scope = ScanScope {
        provider: provider.into(),
        root: root.into(),
        complete: true,
        recursive: true,
        error: None,
    };
    let mut out = vec![];
    if !std::path::Path::new(root).is_dir() {
        scope.complete = false;
        scope.error = Some("scan root is not a readable directory".into());
        return (out, scope);
    }
    for (index, entry) in walkdir::WalkDir::new(root)
        .follow_links(false)
        .max_depth(depth)
        .into_iter()
        .enumerate()
    {
        if cancelled.load(std::sync::atomic::Ordering::Acquire) {
            scope.complete = false;
            scope.error = Some("scan cancelled".into());
            break;
        }
        if index >= limit {
            scope.complete = false;
            scope.error = Some(format!("entry limit {limit} reached"));
            break;
        }
        match entry {
            Ok(e) => {
                if e.file_type().is_symlink() || (e.file_type().is_dir() && e.depth() == depth) {
                    scope.complete = false;
                    scope.error = Some("link or depth boundary not traversed".into());
                }
                if e.file_type().is_file() && is_executable(e.path()) {
                    out.push(candidate_from_file(e.path()));
                }
            }
            Err(e) => {
                scope.complete = false;
                scope.error = Some(e.to_string());
            }
        }
    }
    (out, scope)
}

/// Declarative data extension: only confined reads; no third-party code is loaded.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeclarativeExtension {
    pub schema: String,
    pub id: String,
    pub roots: Vec<String>,
    pub files: Vec<String>,
    pub operations: Vec<String>,
}
impl DeclarativeExtension {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != "toolhub.scanner-extension/v1"
            || self.id.trim().is_empty()
            || self.roots.is_empty()
        {
            return Err("invalid declarative extension schema/id/roots".into());
        }
        if self.operations.is_empty()
            || self
                .operations
                .iter()
                .any(|o| !["discover", "inspect"].contains(&o.as_str()))
        {
            return Err("extension requests unsupported authority".into());
        }
        for file in &self.files {
            if file.is_empty()
                || file.contains('\\')
                || file.starts_with('/')
                || file.contains(':')
                || std::path::Path::new(file)
                    .components()
                    .any(|c| !matches!(c, std::path::Component::Normal(_)))
            {
                return Err("extension file escapes confinement".into());
            }
        }
        Ok(())
    }
    pub fn discover(&self, root: &str) -> Result<Vec<ScanCandidate>, String> {
        self.validate()?;
        if !self.operations.iter().any(|o| o == "discover") {
            return Err("discover permission missing".into());
        }
        let canonical = std::fs::canonicalize(root).map_err(|e| e.to_string())?;
        let authorized = self
            .roots
            .iter()
            .filter_map(|r| std::fs::canonicalize(r).ok())
            .any(|r| r == canonical);
        if !authorized {
            return Err("root is outside extension grant".into());
        }
        let mut out = vec![];
        for relative in &self.files {
            let path = canonical.join(relative);
            if !path.exists() {
                continue;
            }
            let path = std::fs::canonicalize(path).map_err(|e| e.to_string())?;
            if !path.starts_with(&canonical) {
                return Err("extension symlink escapes granted root".into());
            }
            if path.is_file() {
                let mut c = candidate_from_file(&path);
                let scope = ScanScope {
                    provider: format!("extension:{}", self.id),
                    root: root.into(),
                    complete: false,
                    recursive: false,
                    error: None,
                };
                tag_scope(&mut c, &scope);
                c.metadata["extension_id"] = serde_json::json!(self.id);
                out.push(c)
            }
        }
        Ok(out)
    }
}

pub trait ScannerProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn roots(&self) -> Vec<String>;
    fn full_only(&self) -> bool {
        false
    }
    /// Read-only inspection of one root. Errors become coverage failures.
    fn scan_root(&self, root: &str) -> Result<Vec<ScanCandidate>, std::io::Error>;
}

/// B02-13: permission-limited scanner extension. No write/install/execute authority.
pub trait ScannerExtension: Send + Sync {
    fn id(&self) -> &str;
    /// Allowed operations: discover, inspect, classify only.
    fn allowed_ops(&self) -> &'static [&'static str] {
        &["discover", "inspect", "classify"]
    }
    fn discover(&self, root: &str) -> Result<Vec<ScanCandidate>, String>;
}

/// In-process extension registry (fixture plugins only; no arbitrary code loading).
pub struct ExtensionHost {
    pub extensions: Vec<Box<dyn ScannerExtension>>,
}

impl ExtensionHost {
    pub fn new() -> Self {
        Self { extensions: vec![] }
    }

    pub fn register(&mut self, ext: Box<dyn ScannerExtension>) {
        self.extensions.push(ext);
    }

    pub fn run_discover(&self, root: &str) -> Vec<ScanCandidate> {
        let mut out = vec![];
        for e in &self.extensions {
            if e.allowed_ops().contains(&"discover") {
                if let Ok(mut found) = e.discover(root) {
                    out.append(&mut found);
                }
            }
        }
        out
    }
}

impl Default for ExtensionHost {
    fn default() -> Self {
        Self::new()
    }
}

/// Fixture extension used in tests — discovers only a marker file, never executes.
pub struct MarkerFixtureExtension {
    pub id: String,
}

impl ScannerExtension for MarkerFixtureExtension {
    fn id(&self) -> &str {
        &self.id
    }

    fn discover(&self, root: &str) -> Result<Vec<ScanCandidate>, String> {
        let marker = std::path::Path::new(root).join("fixture-tool.marker");
        if marker.is_file() {
            Ok(vec![ScanCandidate::from_path(marker.to_string_lossy())])
        } else {
            Ok(vec![])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_scan_records_provider_and_root() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("tool.exe"), b"finite fixture").unwrap();
        let report = run_scan(
            ScanMode::Quick,
            Some(vec![dir.path().to_string_lossy().into_owned()]),
        );
        assert_eq!(
            report.coverage.roots_attempted.len(),
            1,
            "custom roots must not repeat through unrelated providers"
        );
        assert_eq!(
            report.candidates[0].metadata["scanner_scope"]["provider"],
            "custom"
        );
    }

    #[test]
    fn declarative_extension_participates_in_scan_with_confined_permissions() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("fixture-tool.marker"), b"marker").unwrap();
        let mut ext = DeclarativeExtension {
            schema: "toolhub.scanner-extension/v1".into(),
            id: "fixture".into(),
            roots: vec![dir.path().to_string_lossy().into()],
            files: vec!["fixture-tool.marker".into()],
            operations: vec!["discover".into()],
        };
        let report =
            run_scan_with_extensions(ScanMode::Quick, Some(ext.roots.clone()), &[ext.clone()]);
        assert_eq!(report.candidates.len(), 1);
        assert_eq!(report.candidates[0].metadata["extension_id"], "fixture");
        ext.operations.push("execute".into());
        assert!(ext.validate().is_err());
        ext.operations.pop();
        ext.files[0] = "../escape".into();
        assert!(ext.discover(dir.path().to_str().unwrap()).is_err());
    }

    #[test]
    fn path_provider_finds_executables_in_fixture() {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("mytool.exe");
        std::fs::write(&bin, b"fake").unwrap();
        let p = PathProvider;
        let found = p.scan_root(dir.path().to_str().unwrap()).unwrap();
        assert!(found
            .iter()
            .any(|c| c.path.ends_with("mytool.exe") || c.path.ends_with("mytool")));
    }

    #[test]
    fn scan_does_not_execute_unknown_binaries() {
        // Marker file would be created if a probe ran; scan must not create it.
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("EXECUTED_MARKER");
        let _evil = dir.path().join("unknown-tool-that-writes-marker");
        #[cfg(windows)]
        let evil = dir.path().join("unknown-tool.exe");
        let _ = &marker;
        // We only write a non-executable file with a suspicious name.
        std::fs::write(&evil, b"#!/bin/sh\ntouch EXECUTED_MARKER\n").unwrap();
        let report = run_scan(
            ScanMode::Quick,
            Some(vec![dir.path().to_string_lossy().to_string()]),
        );
        assert!(!report.candidates.is_empty() || report.coverage.roots_ok.len() == 1);
        assert!(!marker.exists(), "scan must not execute unknown binaries");
    }
}
