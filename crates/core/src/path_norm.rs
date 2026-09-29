//! Platform path normalization used for identity and symlink resolution.

use sha2::{Digest, Sha256};

/// Normalize a filesystem path for comparison while preserving original display form.
/// - Windows: lowercase drive/path separators for comparison keys
/// - POSIX: collapse duplicate separators
pub fn normalize_path(path: &str) -> String {
    if cfg!(windows) {
        path.replace('/', "\\").to_lowercase()
    } else {
        let mut out = String::with_capacity(path.len());
        let mut prev_slash = false;
        for c in path.chars() {
            if c == '/' {
                if !prev_slash {
                    out.push(c);
                }
                prev_slash = true;
            } else {
                out.push(c);
                prev_slash = false;
            }
        }
        out
    }
}

/// Stable fingerprint of a normalized path (for evidence, not trust).
pub fn path_fingerprint(path: &str) -> String {
    let mut h = Sha256::new();
    h.update(normalize_path(path).as_bytes());
    hex::encode(h.finalize())
}

/// Resolve symlink/shim to a canonical path when possible; otherwise return original.
pub fn canonicalize_best_effort(path: &str) -> String {
    std::fs::canonicalize(path)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_is_stable() {
        let a = normalize_path("C:/Program Files/Python/python.exe");
        let b = normalize_path("c:\\program files\\python\\python.exe");
        if cfg!(windows) {
            assert_eq!(a, b);
        }
        assert_eq!(path_fingerprint("x"), path_fingerprint("x"));
    }
}
