//! Read-only host discovery. Config presence is not connection or execution authority.
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub fn installed_executable(id: &str) -> Option<PathBuf> {
    let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    let roaming = std::env::var_os("APPDATA").map(PathBuf::from);
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from);
    installed_in(id, local.as_deref(), roaming.as_deref(), home.as_deref()).or_else(|| {
        if id == "mimo" {
            crate::mimo::installed_from_registry()
        } else {
            None
        }
    })
}
pub fn installed_in(
    id: &str,
    local: Option<&Path>,
    roaming: Option<&Path>,
    home: Option<&Path>,
) -> Option<PathBuf> {
    let mut candidates = vec![];
    if let Some(local) = local {
        match id {
            "codex" => {
                let root = local.join("OpenAI/Codex/bin");
                if let Ok(entries) = std::fs::read_dir(root) {
                    let mut dirs: Vec<_> = entries
                        .flatten()
                        .take(128)
                        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
                        .collect();
                    dirs.sort_by_key(|e| {
                        std::cmp::Reverse(e.metadata().and_then(|m| m.modified()).ok())
                    });
                    candidates.extend(dirs.into_iter().map(|e| e.path().join("codex.exe")));
                }
            }
            "cursor" => candidates.push(local.join("Programs/cursor/Cursor.exe")),
            "opencode" => candidates.push(local.join("Programs/@opencode-aidesktop/OpenCode.exe")),
            "vscode" => candidates.push(local.join("Programs/Microsoft VS Code/Code.exe")),
            "claude-desktop" => candidates.push(local.join("AnthropicClaude/claude.exe")),
            "devin-desktop" => {
                candidates.push(local.join("Programs/Windsurf/Windsurf.exe"));
                candidates.push(local.join("Programs/Devin/Devin.exe"));
            }
            "mimo" => {
                for path in [
                    "Programs/Xiaomi MiMo/Xiaomi MiMo.exe",
                    "Programs/xiaomi-mimo/Xiaomi MiMo.exe",
                    "Xiaomi MiMo/Xiaomi MiMo.exe",
                ] {
                    candidates.push(local.join(path));
                }
            }
            _ => {}
        }
    }
    if let Some(roaming) = roaming {
        let name = match id {
            "claude-code" => "claude",
            "gemini-cli" => "gemini",
            "copilot-cli" => "copilot",
            _ => id,
        };
        candidates.push(roaming.join(format!("npm/{name}.cmd")));
    }
    if let Some(home) = home {
        let name = match id {
            "claude-code" => "claude",
            "gemini-cli" => "gemini",
            "copilot-cli" => "copilot",
            _ => id,
        };
        candidates.push(home.join(format!(".local/bin/{name}.exe")));
        candidates.push(home.join(format!(".opencode/bin/{name}")));
    }
    candidates.into_iter().find(|p| p.is_file())
}
pub fn codex_config(home: &Path) -> Value {
    let path = home.join(".codex/config.toml");
    let Ok(metadata) = path.metadata() else {
        return json!({"configured":false});
    };
    if metadata.len() > 1_048_576 {
        return json!({"configured":false,"error":"configuration exceeds 1 MiB"});
    }
    let config = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| toml::from_str::<toml::Value>(&s).ok());
    let Some(config) = config else {
        return json!({"configured":false,"error":"cannot parse configuration"});
    };
    let Some(server) = config.get("mcp_servers").and_then(|v| v.get("toolhub")) else {
        return json!({"configured":false});
    };
    json!({"configured":true,"enabled":server.get("enabled").and_then(toml::Value::as_bool).unwrap_or(true),"config_path":path,"command":server.get("command").and_then(toml::Value::as_str),"args":server.get("args").and_then(toml::Value::as_array).map(|a|a.iter().filter_map(toml::Value::as_str).collect::<Vec<_>>())})
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovers_versioned_codex_without_path_and_parses_only_server_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("OpenAI/Codex/bin/hash/codex.exe");
        std::fs::create_dir_all(bin.parent().unwrap()).unwrap();
        std::fs::write(&bin, "fixture").unwrap();
        assert_eq!(
            installed_in("codex", Some(dir.path()), None, None),
            Some(bin)
        );
        std::fs::create_dir(dir.path().join(".codex")).unwrap();
        std::fs::write(dir.path().join(".codex/config.toml"), "secret='private'\n[mcp_servers.toolhub]\ncommand='D:\\Tool Hub\\toolhub.exe'\nargs=['mcp','serve']\nenabled=false").unwrap();
        let raw = std::fs::read_to_string(dir.path().join(".codex/config.toml")).unwrap();
        toml::from_str::<toml::Value>(&raw).unwrap();
        let config = codex_config(dir.path());
        assert_eq!(config["configured"], true);
        assert_eq!(config["enabled"], false);
        assert!(!config.to_string().contains("private"));
    }
}
