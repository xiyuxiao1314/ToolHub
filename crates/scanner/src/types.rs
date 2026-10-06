use toolhub_core::ScanCandidate;

/// Shared candidate construction helpers (read-only metadata only).
pub fn candidate_from_file(path: &std::path::Path) -> ScanCandidate {
    let mut c = ScanCandidate::from_path(path.to_string_lossy().to_string());
    if let Ok(meta) = std::fs::metadata(path) {
        c.size_bytes = Some(meta.len());
    }
    c.canonical_path = Some(toolhub_core::path_norm::canonicalize_best_effort(
        &path.to_string_lossy(),
    ));
    c
}

pub fn is_executable_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".exe")
        || lower.ends_with(".cmd")
        || lower.ends_with(".bat")
        || lower.ends_with(".com")
        || (!lower.contains('.') && !lower.ends_with(".md") && !lower.ends_with(".txt"))
}

#[cfg(windows)]
pub fn is_executable(path: &std::path::Path) -> bool {
    path.extension()
        .map(|e| {
            let e = e.to_string_lossy().to_ascii_lowercase();
            matches!(e.as_str(), "exe" | "cmd" | "bat" | "com")
        })
        .unwrap_or(false)
}

#[cfg(not(windows))]
pub fn is_executable(path: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}
