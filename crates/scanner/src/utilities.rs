//! Bounded discovery of portable utility binaries outside PATH, without executing them.
use crate::{
    types::{candidate_from_file, is_executable},
    ScannerProvider,
};
use toolhub_core::ScanCandidate;

#[derive(Default)]
pub struct PortableUtilitiesProvider;
impl ScannerProvider for PortableUtilitiesProvider {
    fn name(&self) -> &'static str {
        "portable_utilities"
    }
    fn full_only(&self) -> bool {
        true
    }
    fn roots(&self) -> Vec<String> {
        #[cfg(windows)]
        {
            crate::programs::local_disk_roots()
        }
        #[cfg(not(windows))]
        {
            dirs::home_dir()
                .map(|home| vec![home.to_string_lossy().into_owned()])
                .unwrap_or_default()
        }
    }
    fn scan_root(&self, root: &str) -> Result<Vec<ScanCandidate>, std::io::Error> {
        self.scan_root_with_cancel(root, &std::sync::atomic::AtomicBool::new(false))
    }
    fn scan_root_with_cancel(
        &self,
        root: &str,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<Vec<ScanCandidate>, std::io::Error> {
        if !std::path::Path::new(root).is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "utility scan root is unavailable",
            ));
        }
        let mut result = Vec::new();
        let walk = walkdir::WalkDir::new(root)
            .follow_links(false)
            .max_depth(12)
            .into_iter()
            .filter_entry(|entry| {
                if entry.path_is_symlink() {
                    return false;
                }
                if entry.depth() == 0 || !entry.file_type().is_dir() {
                    return true;
                }
                let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
                !matches!(
                    name.as_str(),
                    "windows"
                        | "program files"
                        | "program files (x86)"
                        | "programdata"
                        | "appdata"
                        | "$recycle.bin"
                        | "system volume information"
                        | ".git"
                        | ".hg"
                        | "node_modules"
                        | ".venv"
                        | "venv"
                        | "__pycache__"
                        | ".cache"
                        | "uv-cache"
                        | "bash-completion"
                        | "bash_completion.d"
                        | "site-functions"
                        | ".codex"
                        | ".rustup"
                        | ".cargo"
                        | ".gradle"
                        | ".m2"
                        | ".npm"
                )
            });
        // The orchestration reports this provider's scopes as bounded, never full-drive coverage.
        for entry in walk.take(300_000).flatten() {
            if cancelled.load(std::sync::atomic::Ordering::Acquire) {
                break;
            }
            if !entry.file_type().is_file()
                || entry.path_is_symlink()
                || !is_executable(entry.path())
            {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            if matches!(
                name.as_str(),
                "ffmpeg.exe"
                    | "ffprobe.exe"
                    | "7z.exe"
                    | "7za.exe"
                    | "magick.exe"
                    | "pandoc.exe"
                    | "pdftoppm.exe"
                    | "pdftotext.exe"
                    | "tesseract.exe"
                    | "yt-dlp.exe"
                    | "curl.exe"
                    | "ffmpeg"
                    | "ffprobe"
                    | "7z"
                    | "7za"
                    | "magick"
                    | "pandoc"
                    | "pdftoppm"
                    | "pdftotext"
                    | "tesseract"
                    | "yt-dlp"
                    | "curl"
            ) {
                let mut candidate = candidate_from_file(entry.path());
                candidate.metadata["source"] = serde_json::json!("portable_utility");
                result.push(candidate);
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_shell_completion_data_and_uv_cache_but_keeps_project_binaries() {
        let tmp = tempfile::tempdir().unwrap();
        for dir in [
            "project/bin",
            "share/bash-completion/completions",
            "app/uv-cache/package/bin",
        ] {
            std::fs::create_dir_all(tmp.path().join(dir)).unwrap();
        }
        for file in [
            "project/bin/ffmpeg.exe",
            "share/bash-completion/completions/yt-dlp",
            "app/uv-cache/package/bin/ffmpeg.exe",
        ] {
            std::fs::write(tmp.path().join(file), b"read only fixture").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(
                    tmp.path().join(file),
                    std::fs::Permissions::from_mode(0o755),
                )
                .unwrap();
            }
        }
        let found = PortableUtilitiesProvider
            .scan_root(tmp.path().to_str().unwrap())
            .unwrap();
        assert_eq!(found.len(), 1);
        assert!(found[0].path.ends_with("ffmpeg.exe"));
    }

    #[test]
    fn discovers_portable_media_without_crawling_dependencies_or_executing_files() {
        let tmp = tempfile::tempdir().unwrap();
        for dir in [
            "projects/media/bin",
            "projects/app/node_modules/ffmpeg",
            "Windows/bin",
        ] {
            std::fs::create_dir_all(tmp.path().join(dir)).unwrap();
        }
        for path in [
            "projects/media/bin/ffmpeg.exe",
            "projects/media/bin/ffprobe.exe",
            "projects/media/bin/unrelated.exe",
            "projects/app/node_modules/ffmpeg/ffmpeg.exe",
            "Windows/bin/ffmpeg.exe",
        ] {
            std::fs::write(
                tmp.path().join(path),
                b"not executable; discovery must only inspect",
            )
            .unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(
                    tmp.path().join(path),
                    std::fs::Permissions::from_mode(0o755),
                )
                .unwrap();
            }
        }
        let found = PortableUtilitiesProvider
            .scan_root(tmp.path().to_str().unwrap())
            .unwrap();
        assert_eq!(found.len(), 2);
        assert!(found
            .iter()
            .all(|c| c.metadata["source"] == "portable_utility"));
        let cancelled = std::sync::atomic::AtomicBool::new(true);
        assert!(PortableUtilitiesProvider
            .scan_root_with_cancel(tmp.path().to_str().unwrap(), &cancelled)
            .unwrap()
            .is_empty());
    }
}
