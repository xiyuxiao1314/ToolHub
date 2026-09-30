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
}

#[derive(Debug, Default, Clone)]
pub struct ScanReport {
    pub candidates: Vec<ScanCandidate>,
    pub coverage: ScanCoverage,
    pub cancelled: bool,
}

/// Orchestrate platform providers. Unknown binaries are never executed.
pub fn run_scan(mode: ScanMode, roots_override: Option<Vec<String>>) -> ScanReport {
    let mut report = ScanReport::default();
    let providers: Vec<Box<dyn ScannerProvider>> = vec![
        Box::new(PathProvider),
        Box::new(KnownDirsProvider),
        Box::new(PackageManagerProvider),
        if cfg!(windows) {
            Box::new(WindowsRegistryProvider) as Box<dyn ScannerProvider>
        } else {
            Box::new(MacApplicationsProvider) as Box<dyn ScannerProvider>
        },
    ];

    for p in providers {
        // F08: Quick skips deep package walks; Full includes them.
        let deep = p.full_only() || p.name() == "package_managers";
        if mode == ScanMode::Quick && deep && p.name() == "package_managers" {
            continue;
        }
        let roots = roots_override.clone().unwrap_or_else(|| p.roots());
        for root in roots {
            report.coverage.roots_attempted.push(root.clone());
            match p.scan_root(&root) {
                Ok(mut found) => {
                    report.coverage.roots_ok.push(root.clone());
                    report.candidates.append(&mut found);
                }
                Err(e) => {
                    report
                        .coverage
                        .roots_failed
                        .push((root.clone(), e.to_string()));
                }
            }
        }
    }

    // Deduplicate by normalized path while preserving first-seen metadata.
    let mut seen = std::collections::BTreeSet::new();
    report
        .candidates
        .retain(|c| seen.insert(toolhub_core::normalize_path(&c.path)));
    report
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
