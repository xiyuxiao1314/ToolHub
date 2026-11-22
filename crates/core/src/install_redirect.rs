//! Stable default-profile CLI and forwarding from retained versioned installations.
use std::{
    fs,
    path::{Path, PathBuf},
};
fn windows_root() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|p| PathBuf::from(p).join("ToolHub"))
}
pub fn current_target(
    executable: &Path,
    root: &Path,
    current: &serde_json::Value,
) -> Option<PathBuf> {
    let root = root.canonicalize().ok()?;
    let exe = executable.canonicalize().ok()?;
    let name = exe.file_name()?.to_str()?;
    if !matches!(name, "toolhub.exe" | "toolhub-desktop.exe") {
        return None;
    }
    let versions = root.join("versions");
    let in_versions = exe.starts_with(&versions);
    let in_bin = name == "toolhub.exe" && exe.parent() == Some(root.join("bin").as_path());
    if !in_versions && !in_bin {
        return None;
    }
    let target = PathBuf::from(
        current[if name == "toolhub.exe" {
            "cli"
        } else {
            "desktop"
        }]
        .as_str()?,
    )
    .canonicalize()
    .ok()?;
    if !target.starts_with(versions.canonicalize().ok()?)
        || target.file_name() != exe.file_name()
        || target == exe
        || !target.is_file()
    {
        return None;
    }
    Some(target)
}
pub fn stable_cli() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        if std::env::var_os("TOOLHUB_REGISTRY").is_some() {
            return None;
        }
        let bin = windows_root()?.join("bin");
        let metadata = fs::metadata(bin.join("launcher.json")).ok()?;
        if metadata.len() > 1024 {
            return None;
        }
        let marker: serde_json::Value =
            serde_json::from_slice(&fs::read(bin.join("launcher.json")).ok()?).ok()?;
        let path = bin.join("toolhub.exe");
        let content = fs::read(&path).ok()?;
        if content.len() > 16 * 1024 * 1024 {
            return None;
        }
        use sha2::{Digest, Sha256};
        if marker["schema"] != "toolhub.forwarder/v1"
            || marker["sha256"] != hex::encode(Sha256::digest(content))
        {
            return None;
        }
        Some(path)
    }
    #[cfg(not(windows))]
    {
        None
    }
}
pub fn forward_if_needed() -> Result<Option<i32>, String> {
    #[cfg(windows)]
    {
        if std::env::var_os("TOOLHUB_REGISTRY").is_some()
            || std::env::var_os("TOOLHUBD_BIN").is_some()
        {
            return Ok(None);
        }
        let Some(root) = windows_root() else {
            return Ok(None);
        };
        let metadata = match fs::metadata(root.join("current.json")) {
            Ok(v) => v,
            Err(_) => return Ok(None),
        };
        if metadata.len() > 8192 {
            return Err("ToolHub current.json exceeds size bound".into());
        }
        let current: serde_json::Value = serde_json::from_slice(
            &fs::read(root.join("current.json")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        // Version probes must identify the actual candidate binary before activation.
        // The stable bin entry still reports the active installed version.
        let arguments: Vec<_> = std::env::args_os().skip(1).collect();
        if exe.parent() != Some(root.join("bin").as_path())
            && arguments.len() == 1
            && arguments[0] == "--version"
        {
            return Ok(None);
        }
        let Some(target) = current_target(&exe, &root, &current) else {
            return Ok(None);
        };
        let desktop = target
            .file_name()
            .is_some_and(|n| n == "toolhub-desktop.exe");
        let mut command = std::process::Command::new(&target);
        command.args(std::env::args_os().skip(1));
        if desktop {
            command.current_dir(
                target
                    .parent()
                    .ok_or("ToolHub install directory unavailable")?,
            );
            use std::os::windows::process::CommandExt;
            command
                .creation_flags(0x08000000)
                .spawn()
                .map_err(|e| e.to_string())?;
            return Ok(Some(0));
        }
        let status = command.status().map_err(|e| e.to_string())?;
        Ok(Some(status.code().unwrap_or(1)))
    }
    #[cfg(not(windows))]
    {
        Ok(None)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_own_retained_versions_and_stable_cli_can_forward_inside_versions() {
        let root = std::env::temp_dir().join(format!("toolhub-redirect-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(root.join("versions/old")).unwrap();
        fs::create_dir_all(root.join("versions/new")).unwrap();
        fs::create_dir_all(root.join("bin")).unwrap();
        let old = root.join("versions/old/toolhub.exe");
        let new = root.join("versions/new/toolhub.exe");
        let stable = root.join("bin/toolhub.exe");
        for p in [&old, &new, &stable] {
            fs::write(p, b"fixture").unwrap()
        }
        let current = serde_json::json!({"cli":new});
        assert_eq!(
            current_target(&old, &root, &current),
            Some(new.canonicalize().unwrap())
        );
        assert_eq!(
            current_target(&stable, &root, &current),
            Some(new.canonicalize().unwrap())
        );
        assert!(current_target(&new, &root, &current).is_none());
        let outside = root.join("outside/toolhub.exe");
        fs::create_dir_all(outside.parent().unwrap()).unwrap();
        fs::write(&outside, b"fixture").unwrap();
        assert!(current_target(&old, &root, &serde_json::json!({"cli":outside})).is_none());
        let canonical = root.canonicalize().unwrap();
        assert!(canonical
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("toolhub-redirect-"));
        fs::remove_dir_all(canonical).unwrap();
    }
}
