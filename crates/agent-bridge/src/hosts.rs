//! Host metadata and read-only default configuration discovery. Never grants authority.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};

pub struct HostProfile {
    pub id: &'static str,
    pub name: &'static str,
    pub format: &'static str,
    pub location: &'static str,
    pub note: &'static str,
    pub docs: &'static str,
}
pub const HOSTS: &[HostProfile] = &[
    HostProfile { id: "codex", name: "Codex", format: "toml", location: "~/.codex/config.toml", note: "只合并 mcp_servers.toolhub；宿主可能需要重新打开对话。", docs: "https://developers.openai.com/codex/mcp/" },
    HostProfile { id: "mimo", name: "MiMo", format: "local-array", location: "~/.config/mimocode/mimocode.jsonc", note: "保留 provider 等已有配置，合并 mcp.toolhub。", docs: "" },
    HostProfile { id: "claude-code", name: "Claude Code", format: "mcpServers", location: "~/.claude.json（用户级）；项目 .mcp.json", note: "本地范围可位于 .claude.json 的 projects 中；项目服务仍需宿主信任。", docs: "https://code.claude.com/docs/en/mcp" },
    HostProfile { id: "claude-desktop", name: "Claude Desktop", format: "mcpServers", location: "%APPDATA%/Claude/claude_desktop_config.json（Windows）", note: "桌面端与 Claude Code 分别配置；重启桌面客户端后验证。", docs: "https://modelcontextprotocol.io/docs/develop/connect-local-servers" },
    HostProfile { id: "cursor", name: "Cursor", format: "mcpServers", location: "~/.cursor/mcp.json；项目 .cursor/mcp.json", note: "默认只读检测用户配置；项目覆盖和启用状态以 Cursor 为准。", docs: "https://prod.cursor.com/docs/mcp" },
    HostProfile { id: "opencode", name: "OpenCode", format: "local-array", location: "~/.config/opencode/opencode.jsonc 或 opencode.json", note: "合并 mcp.toolhub；自定义配置路径及项目配置可能覆盖用户配置。", docs: "https://opencode.ai/docs/mcp-servers/" },
    HostProfile { id: "gemini-cli", name: "Gemini CLI", format: "mcpServers", location: "~/.gemini/settings.json", note: "mcp.allowed / excluded 及独立的启用文件会影响实际加载；配置存在不等于已启用。", docs: "https://geminicli.com/docs/tools/mcp-server/" },
    HostProfile { id: "vscode", name: "VS Code / Copilot", format: "servers", location: "MCP: Open User Configuration；Windows 默认 %APPDATA%/Code/User/mcp.json", note: "VS Code 格式使用 servers；也支持项目 .mcp.json 和 ~/.copilot/mcp-config.json 的 mcpServers 格式。多配置档与工作区设置需在宿主核对。", docs: "https://code.visualstudio.com/docs/agent-customization/mcp-servers" },
    HostProfile { id: "copilot-cli", name: "GitHub Copilot CLI", format: "copilot", location: "~/.copilot/mcp-config.json", note: "COPILOT_HOME 可覆盖目录；项目配置及信任状态以宿主为准。", docs: "https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-config-dir-reference" },
    HostProfile { id: "devin-desktop", name: "Devin Desktop / Windsurf", format: "mcpServers", location: "%APPDATA%/devin/mcp_config.json（当前 Windows 文档）", note: "Windsurf 文档现指向 Devin Desktop；旧版请从宿主 MCP 设置确认实际配置位置。", docs: "https://docs.devin.ai/desktop/cascade/mcp" },
];
pub fn profile(id: &str) -> Option<&'static HostProfile> {
    HOSTS.iter().find(|host| host.id == id)
}
pub fn template(host: &HostProfile, cli: &str) -> String {
    if host.format == "toml" {
        return format!(
            "[mcp_servers.toolhub]\ncommand = {}\nargs = [\"mcp\", \"serve\"]\n",
            json!(cli)
        );
    }
    let server = match host.format {
        "local-array" => json!({"type":"local","command":[cli,"mcp","serve"],"enabled":true}),
        "copilot" => json!({"type":"stdio","command":cli,"args":["mcp","serve"],"tools":["*"]}),
        _ => json!({"command":cli,"args":["mcp","serve"]}),
    };
    let root = match host.format {
        "local-array" => "mcp",
        "servers" => "servers",
        _ => "mcpServers",
    };
    serde_json::to_string_pretty(&json!({root:{"toolhub":server}})).expect("serializable template")
}
pub fn templates(cli: &str) -> Value {
    let mut rows: Vec<Value> = HOSTS.iter().map(|host| json!({"id":host.id,"name":host.name,"location":host.location,"note":host.note,"docs":host.docs,"format":host.format,"config":template(host,cli)})).collect();
    rows.push(json!({"id":"generic","name":"其他 MCP 客户端","location":"在宿主的 MCP 设置中添加本地 stdio 服务","note":"支持 MCP stdio 即可接入；具体配置格式、重载和 Skill 目录按该宿主要求。","format":"mcpServers","docs":"https://modelcontextprotocol.io/docs/develop/connect-local-servers","config":template(&HOSTS[2],cli)}));
    json!(rows)
}

/// Names are self-reported display metadata, never an authentication identity.
pub fn observed_identity(raw: &str) -> Option<(String, String)> {
    let display = raw
        .chars()
        .filter(|c| !c.is_control())
        .take(128)
        .collect::<String>();
    let display = display.split_whitespace().collect::<Vec<_>>().join(" ");
    let normalized = display.to_lowercase();
    if normalized.is_empty() || normalized == "toolhub self-test" {
        return None;
    }
    let words: Vec<_> = normalized
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();
    let has = |word: &str| words.contains(&word);
    let id = if has("codex") {
        Some("codex")
    } else if has("mimo") || has("mimocode") {
        Some("mimo")
    } else if has("claude") && has("code") {
        Some("claude-code")
    } else if has("claude") {
        Some("claude-desktop")
    } else if has("cursor") {
        Some("cursor")
    } else if has("opencode") {
        Some("opencode")
    } else if has("gemini") {
        Some("gemini-cli")
    } else if has("vscode") || (has("visual") && has("studio") && has("code")) {
        Some("vscode")
    } else if has("copilot") {
        Some("copilot-cli")
    } else if has("windsurf") || has("devin") {
        Some("devin-desktop")
    } else {
        None
    };
    if let Some(host) = id.and_then(profile) {
        return Some((host.id.into(), host.name.into()));
    }
    let digest = hex::encode(Sha256::digest(normalized.as_bytes()));
    Some((format!("mcp-client-{}", &digest[..24]), display))
}

fn locations(id: &str, home: &Path, roaming: Option<&Path>) -> Vec<PathBuf> {
    match id {
        "claude-code" => vec![home.join(".claude.json")],
        "claude-desktop" => roaming
            .map(|p| vec![p.join("Claude/claude_desktop_config.json")])
            .unwrap_or_default(),
        "cursor" => vec![home.join(".cursor/mcp.json")],
        "opencode" => vec![
            home.join(".config/opencode/opencode.jsonc"),
            home.join(".config/opencode/opencode.json"),
        ],
        "gemini-cli" => vec![home.join(".gemini/settings.json")],
        "vscode" => {
            let mut paths = roaming
                .map(|p| vec![p.join("Code/User/mcp.json")])
                .unwrap_or_default();
            paths.push(home.join(".copilot/mcp-config.json"));
            paths
        }
        "copilot-cli" => vec![home.join(".copilot/mcp-config.json")],
        "devin-desktop" => roaming
            .map(|p| vec![p.join("devin/mcp_config.json")])
            .unwrap_or_default(),
        _ => vec![],
    }
}
fn read_config(path: &Path) -> Result<Value, &'static str> {
    let file = std::fs::File::open(path).map_err(|_| "cannot read configuration")?;
    let mut data = Vec::new();
    file.take(1_048_577)
        .read_to_end(&mut data)
        .map_err(|_| "cannot read configuration")?;
    if data.len() > 1_048_576 {
        return Err("configuration exceeds 1 MiB");
    }
    let text = std::str::from_utf8(&data).map_err(|_| "configuration is not UTF-8")?;
    crate::mimo::parse_jsonc(text).ok_or("cannot parse JSON/JSONC configuration")
}
fn server_metadata(server: &Value, path: &Path, scope: &str, array: bool) -> Value {
    let failure = |message| json!({"configured":false,"config_path":path,"error":message});
    let (command, args) = if array {
        let Some(command) = server["command"].as_array() else {
            return failure("local MCP command must be an array");
        };
        if server["type"] != "local" || command.len() != 3 {
            return failure("expected type=local and [CLI, mcp, serve]");
        }
        (command[0].clone(), json!([command[1], command[2]]))
    } else {
        if server
            .get("type")
            .is_some_and(|v| v != "stdio" && v != "local")
        {
            return failure("ToolHub uses local stdio MCP");
        }
        (server["command"].clone(), server["args"].clone())
    };
    if !command
        .as_str()
        .is_some_and(|s| !s.is_empty() && s.len() <= 4096 && !s.contains('\0'))
        || args != json!(["mcp", "serve"])
    {
        return failure("expected CLI command and args [mcp, serve]");
    }
    if ["enabled", "disabled"]
        .iter()
        .any(|key| server.get(key).is_some_and(|v| !v.is_boolean()))
    {
        return failure("invalid MCP enablement flag");
    }
    json!({"configured":true,"enabled":server["enabled"]!=false && server["disabled"]!=true,"transport":"stdio","config_path":path,"scope":scope,"command":command,"args":args})
}
pub fn configuration(id: &str, home: &Path, roaming: Option<&Path>) -> Value {
    let mut result = match id {
        "codex" => crate::discovery::codex_config(home),
        "mimo" => crate::mimo::configuration(home),
        _ => {
            let mut first = json!({"configured":false});
            for path in locations(id, home, roaming)
                .into_iter()
                .filter(|p| p.is_file())
            {
                let config = match read_config(&path) {
                    Ok(config) => config,
                    Err(error) => {
                        if first.get("config_path").is_none() {
                            first = json!({"configured":false,"config_path":path,"error":error});
                        }
                        continue;
                    }
                };
                let server = match id {
                    "opencode" => config
                        .pointer("/mcp/toolhub")
                        .or_else(|| config.pointer("/mcp/servers/toolhub")),
                    "vscode" if path.file_name().is_some_and(|name| name == "mcp.json") => {
                        config.pointer("/servers/toolhub")
                    }
                    _ => config.pointer("/mcpServers/toolhub"),
                };
                let mut candidates = Vec::new();
                if let Some(server) = server {
                    candidates.push((server, "user".to_owned()));
                }
                if id == "claude-code" {
                    if let Some(projects) = config["projects"].as_object() {
                        for (project, value) in projects.iter().take(128) {
                            if let Some(server) = value.pointer("/mcpServers/toolhub") {
                                candidates.push((server, format!("project: {project}")));
                            }
                        }
                    }
                }
                if first.get("config_path").is_none() {
                    first = json!({"configured":false,"config_path":path});
                }
                for (server, scope) in candidates {
                    let mut metadata =
                        server_metadata(server, &path, &scope, server["command"].is_array());
                    if metadata["configured"] == true {
                        if config["disabledMcpServers"]
                            .as_array()
                            .is_some_and(|items| items.iter().any(|v| v == "toolhub"))
                        {
                            metadata["enabled"] = json!(false);
                        }
                        first = metadata;
                        break;
                    } else if first.get("error").is_none() {
                        first = metadata;
                    }
                }
                if first["configured"] == true {
                    break;
                }
            }
            first
        }
    };
    result["reader_supported"] = json!(profile(id).is_some());
    result["note"] =
        json!("只检查默认用户配置；项目覆盖、自定义路径及宿主实际启用状态需在客户端核对。");
    result
}

/// Allowlisted Windows installation metadata only; no execution or filesystem scan.
pub fn installation_candidates(id: &str, location: &str, icon: &str) -> Vec<PathBuf> {
    let filenames: &[&str] = match id {
        "mimo" => &["Xiaomi MiMo.exe", "mimocode.exe"],
        "opencode" => &["OpenCode.exe"],
        "cursor" => &["Cursor.exe"],
        "claude-desktop" => &["Claude.exe"],
        "vscode" => &["Code.exe"],
        "devin-desktop" => &["Devin.exe", "Windsurf.exe"],
        _ => &[],
    };
    let mut paths = Vec::new();
    let icon = icon
        .trim()
        .rsplit_once(',')
        .filter(|(_, index)| index.trim().parse::<i32>().is_ok())
        .map_or(icon, |(p, _)| p)
        .trim()
        .trim_matches('"');
    if icon.len() <= 4096
        && filenames.iter().any(|name| {
            icon.rsplit(['\\', '/'])
                .next()
                .is_some_and(|file| file.eq_ignore_ascii_case(name))
        })
    {
        paths.push(PathBuf::from(icon));
    }
    if !location.is_empty() && location.len() <= 4096 {
        paths.extend(
            filenames
                .iter()
                .map(|name| Path::new(location.trim().trim_matches('"')).join(name)),
        );
    }
    paths
}
#[cfg(windows)]
pub fn registered_installations() -> std::collections::BTreeMap<String, PathBuf> {
    use winreg::{
        enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE},
        RegKey,
    };
    let mut found = std::collections::BTreeMap::new();
    for (hive, key) in [
        (
            HKEY_CURRENT_USER,
            r"Software\Microsoft\Windows\CurrentVersion\Uninstall",
        ),
        (
            HKEY_LOCAL_MACHINE,
            r"Software\Microsoft\Windows\CurrentVersion\Uninstall",
        ),
        (
            HKEY_LOCAL_MACHINE,
            r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
        ),
    ] {
        let Ok(root) = RegKey::predef(hive).open_subkey(key) else {
            continue;
        };
        for name in root.enum_keys().take(1024).flatten() {
            let Ok(entry) = root.open_subkey(name) else {
                continue;
            };
            let display = entry
                .get_value::<String, _>("DisplayName")
                .unwrap_or_default()
                .to_ascii_lowercase();
            let id = if display == "opencode" || display.starts_with("opencode ") {
                "opencode"
            } else if display == "cursor" || display.starts_with("cursor ") {
                "cursor"
            } else if display == "claude" || display.starts_with("claude ") {
                "claude-desktop"
            } else if display.starts_with("microsoft visual studio code") {
                "vscode"
            } else if display == "xiaomi mimo" || display.starts_with("xiaomi mimo ") {
                "mimo"
            } else if display == "windsurf"
                || display.starts_with("windsurf ")
                || display == "devin"
                || display.starts_with("devin desktop")
            {
                "devin-desktop"
            } else {
                continue;
            };
            let location = entry
                .get_value::<String, _>("InstallLocation")
                .unwrap_or_default();
            let icon = entry
                .get_value::<String, _>("DisplayIcon")
                .unwrap_or_default();
            if let Some(path) = installation_candidates(id, &location, &icon)
                .into_iter()
                .find(|p| p.is_absolute() && p.is_file())
            {
                found.entry(id.to_owned()).or_insert(path);
            }
        }
    }
    found
}
#[cfg(not(windows))]
pub fn registered_installations() -> std::collections::BTreeMap<String, PathBuf> {
    std::collections::BTreeMap::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_documented_template_round_trips_without_exposing_secrets() {
        let home = tempfile::tempdir().unwrap();
        let roaming = home.path().join("roaming");
        let cli = "D:\\Agent's 项目 [1]\\Tool Hub\\toolhub.exe";
        for host in HOSTS {
            if host.id == "codex" || host.id == "mimo" {
                continue;
            }
            let paths = locations(host.id, home.path(), Some(&roaming));
            let path = &paths[0];
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            let mut config: Value = serde_json::from_str(&template(host, cli)).unwrap();
            config["provider"] = json!({"apiKey":"secret-never-disclose","url":"https://a//b"});
            std::fs::write(path, format!("// fixture\n{config}")).unwrap();
            let result = configuration(host.id, home.path(), Some(&roaming));
            assert_eq!(result["configured"], true, "{}: {result}", host.id);
            assert_eq!(result["command"], cli);
            assert_eq!(result["args"], json!(["mcp", "serve"]));
            assert!(!result.to_string().contains("secret-never-disclose"));
        }
        let toml: toml::Value = toml::from_str(&template(&HOSTS[0], cli)).unwrap();
        assert_eq!(
            toml["mcp_servers"]["toolhub"]["command"].as_str(),
            Some(cli)
        );
    }
    #[test]
    fn local_scopes_disable_flags_malformed_and_bounded_files() {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join(".claude.json");
        std::fs::write(&path,json!({"projects":{"D:/项目":{"mcpServers":{"toolhub":{"command":"toolhub","args":["mcp","serve"]}}}},"disabledMcpServers":["toolhub"]}).to_string()).unwrap();
        let config = configuration("claude-code", home.path(), None);
        assert_eq!(config["scope"], "project: D:/项目");
        assert_eq!(config["enabled"], false);
        std::fs::write(&path, "{ /* incomplete").unwrap();
        assert!(configuration("claude-code", home.path(), None)["error"].is_string());
        std::fs::write(&path, vec![b' '; 1_048_577]).unwrap();
        assert!(configuration("claude-code", home.path(), None)["error"]
            .as_str()
            .unwrap()
            .contains("1 MiB"));
        assert_eq!(
            configuration("unlisted", home.path(), None)["reader_supported"],
            false
        );
        assert_eq!(
            server_metadata(
                &json!({"command":"toolhub","args":["mcp","serve"],"disabled":true}),
                &path,
                "user",
                false
            )["enabled"],
            false
        );
        assert!(server_metadata(
            &json!({"command":"toolhub","args":["mcp","serve","unsafe"],"env":{"token":"secret"}}),
            &path,
            "user",
            false
        )["error"]
            .is_string());
    }
    #[test]
    fn unknown_clients_are_stable_and_known_names_do_not_match_substrings() {
        assert_eq!(
            observed_identity("  Custom\n Host  ").unwrap().0,
            observed_identity("custom host").unwrap().0
        );
        for (name, id) in [
            ("Claude Code", "claude-code"),
            ("Claude Desktop", "claude-desktop"),
            ("gemini-cli", "gemini-cli"),
            ("Visual Studio Code", "vscode"),
            ("GitHub Copilot CLI", "copilot-cli"),
            ("mimocode-verify", "mimo"),
            ("Codex fixture", "codex"),
        ] {
            assert_eq!(observed_identity(name).unwrap().0, id);
        }
        assert!(observed_identity("notcodex")
            .unwrap()
            .0
            .starts_with("mcp-client-"));
        assert!(observed_identity("ToolHub self-test").is_none());
        assert!(observed_identity("\0 ").is_none());
        assert_eq!(observed_identity(&"x".repeat(1000)).unwrap().1.len(), 128);
    }
    #[test]
    fn registered_icons_preserve_quoted_unicode_paths_and_reject_other_files() {
        assert_eq!(
            installation_candidates("vscode", "", r#""D:\软件,目录 [1]\Code.exe",0"#),
            vec![PathBuf::from(r"D:\软件,目录 [1]\Code.exe")]
        );
        assert!(installation_candidates("vscode", "", r"D:\Tools\malware.exe,0").is_empty());
        assert_eq!(templates("toolhub").as_array().unwrap().len(), 11);
    }
}
