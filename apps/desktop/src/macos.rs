//! Native macOS dialogs and Terminal integration. Paths are argv data, never script source.
use std::process::Command;

const DIALOG: &str = r#"on run argv
    try
        set operation to item 1 of argv
        activate
        if operation is "folder" then
            set chosen to choose folder with prompt (item 2 of argv)
        else if operation is "save" then
            set chosen to choose file name with prompt "导出 ToolHub 通用能力包" default name (item 2 of argv)
        else
            set chosen to choose file with prompt (item 2 of argv)
        end if
        return POSIX path of chosen
    on error number -128
        return "__TOOLHUB_CANCEL__"
    end try
end run"#;

const TERMINAL: &str = r#"on run argv
    tell application "Terminal"
        activate
        do script "cd -- " & quoted form of (item 1 of argv)
    end tell
end run"#;

pub fn choose(operation: &str, prompt: &str) -> Result<Option<String>, String> {
    let output = Command::new("/usr/bin/osascript")
        .args(["-e", DIALOG, operation, prompt])
        .output()
        .map_err(|error| format!("macOS 文件选择器不可用：{error}"))?;
    if !output.status.success() {
        return Err(format!(
            "macOS 文件选择失败：{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let path = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
    let path = path.strip_suffix('\n').unwrap_or(&path);
    Ok((path != "__TOOLHUB_CANCEL__").then(|| path.to_owned()))
}

pub fn open_terminal(path: &std::path::Path) -> Result<(), String> {
    let status = Command::new("/usr/bin/osascript")
        .args(["-e", TERMINAL])
        .arg(path)
        .status()
        .map_err(|error| error.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("macOS Terminal 打开失败".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_scripts_compile_without_running_dialogs() {
        for script in [DIALOG, TERMINAL] {
            let output =
                std::env::temp_dir().join(format!("toolhub-{}.scpt", uuid::Uuid::new_v4()));
            let result = Command::new("/usr/bin/osacompile")
                .arg("-o")
                .arg(&output)
                .args(["-e", script])
                .output()
                .unwrap();
            let _ = std::fs::remove_file(output);
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
    }
}
