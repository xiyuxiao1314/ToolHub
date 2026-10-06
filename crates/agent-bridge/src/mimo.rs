//! MiMo discovery reads only installation and MCP metadata; it grants no host authority.
use serde_json::{json, Value};
use std::io::Read;
use std::path::{Path, PathBuf};

pub fn configuration(home: &Path) -> Value {
    let root = home.join(".config/mimocode");
    let Some(path) = [root.join("mimocode.jsonc"), root.join("mimocode.json")]
        .into_iter()
        .find(|path| path.is_file())
    else {
        return json!({"configured":false});
    };
    let failure = |reason: &str| json!({"configured":false,"config_path":path,"error":reason});
    let mut data = Vec::new();
    let Ok(file) = std::fs::File::open(&path) else {
        return failure("cannot read MiMo configuration");
    };
    if file.take(1_048_577).read_to_end(&mut data).is_err() || data.len() > 1_048_576 {
        return failure("configuration unavailable or exceeds 1 MiB");
    }
    let Ok(text) = std::str::from_utf8(&data) else {
        return failure("MiMo configuration is not UTF-8");
    };
    let Some(config) = parse_jsonc(text) else {
        return failure("cannot parse MiMo JSONC configuration");
    };
    let Some(server) = config.get("mcp").and_then(|mcp| mcp.get("toolhub")) else {
        return json!({"configured":false,"config_path":path});
    };
    let Some(command) = server.get("command").and_then(Value::as_array) else {
        return failure("MiMo ToolHub needs a local command array");
    };
    if server.get("type").and_then(Value::as_str) != Some("local")
        || command.len() != 3
        || !command[0]
            .as_str()
            .is_some_and(|s| !s.is_empty() && s.len() <= 4096 && !s.contains('\0'))
        || command[1] != "mcp"
        || command[2] != "serve"
        || server.get("enabled").is_some_and(|v| !v.is_boolean())
    {
        return failure("MiMo ToolHub needs type=local and [CLI, mcp, serve]");
    }
    json!({"configured":true,"enabled":server.get("enabled").and_then(Value::as_bool).unwrap_or(true),"config_path":path,"transport":"stdio","command":command[0],"args":["mcp","serve"]})
}

/// Normalize JSONC comments/trailing commas without evaluating code or touching strings.
pub(crate) fn parse_jsonc(text: &str) -> Option<Value> {
    let source = text.trim_start_matches('\u{feff}').as_bytes();
    let mut clean = Vec::with_capacity(source.len());
    let (mut i, mut quoted, mut escaped) = (0, false, false);
    while i < source.len() {
        let byte = source[i];
        if quoted {
            clean.push(byte);
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
            i += 1;
        } else if byte == b'"' {
            quoted = true;
            clean.push(byte);
            i += 1;
        } else if byte == b'/' && source.get(i + 1) == Some(&b'/') {
            clean.push(b' ');
            i += 2;
            while i < source.len() && !matches!(source[i], b'\n' | b'\r') {
                i += 1;
            }
        } else if byte == b'/' && source.get(i + 1) == Some(&b'*') {
            clean.push(b' ');
            i += 2;
            while i + 1 < source.len() && !(source[i] == b'*' && source[i + 1] == b'/') {
                i += 1;
            }
            if i + 1 >= source.len() {
                return None;
            }
            i += 2;
        } else {
            clean.push(byte);
            i += 1;
        }
    }
    let (mut quoted, mut escaped) = (false, false);
    for i in 0..clean.len() {
        let byte = clean[i];
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
        } else if byte == b'"' {
            quoted = true;
        } else if byte == b','
            && clean[i + 1..]
                .iter()
                .find(|b| !b.is_ascii_whitespace())
                .is_some_and(|b| matches!(b, b'}' | b']'))
        {
            clean[i] = b' ';
        }
    }
    serde_json::from_slice(&clean).ok()
}

pub fn installation_candidates(location: &str, icon: &str) -> Vec<PathBuf> {
    let icon = icon.trim();
    let icon = icon
        .rsplit_once(',')
        .filter(|(_, index)| index.trim().parse::<i32>().is_ok())
        .map_or(icon, |(path, _)| path)
        .trim()
        .trim_matches('"');
    let mut paths = Vec::new();
    if !icon.is_empty() && icon.len() <= 4096 {
        let path = PathBuf::from(icon);
        if icon.rsplit(['\\', '/']).next().is_some_and(|name| {
            matches!(
                name.to_ascii_lowercase().as_str(),
                "xiaomi mimo.exe" | "mimocode.exe"
            )
        }) {
            paths.push(path);
        }
    }
    if !location.is_empty() && location.len() <= 4096 {
        paths.push(Path::new(location.trim_matches('"')).join("Xiaomi MiMo.exe"));
    }
    paths
}

#[cfg(windows)]
pub fn installed_from_registry() -> Option<PathBuf> {
    use winreg::{
        enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE},
        RegKey,
    };
    for (hive, subkey) in [
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
        let Ok(root) = RegKey::predef(hive).open_subkey(subkey) else {
            continue;
        };
        for name in root.enum_keys().take(1024).flatten() {
            let Ok(entry) = root.open_subkey(name) else {
                continue;
            };
            let display = entry
                .get_value::<String, _>("DisplayName")
                .unwrap_or_default();
            let label = display.to_ascii_lowercase();
            if label != "xiaomi mimo" && !label.starts_with("xiaomi mimo ") {
                continue;
            }
            let location = entry
                .get_value::<String, _>("InstallLocation")
                .unwrap_or_default();
            let icon = entry
                .get_value::<String, _>("DisplayIcon")
                .unwrap_or_default();
            if let Some(path) = installation_candidates(&location, &icon)
                .into_iter()
                .find(|path| path.is_absolute() && path.is_file())
            {
                return Some(path);
            }
        }
    }
    None
}

#[cfg(not(windows))]
pub fn installed_from_registry() -> Option<PathBuf> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn jsonc_preserves_strings_and_reads_only_toolhub_metadata() {
        let home = tempfile::tempdir().unwrap();
        let root = home.path().join(".config/mimocode");
        std::fs::create_dir_all(&root).unwrap();
        let text = r#"{
          // Actual MiMo local MCP shape with comments and trailing commas.
          "provider":{"token":"private","url":"https://example.test/a//b","quote":"a\"/*b"},
          "mcp":{"toolhub":{"type":"local","command":["D:\\程序 [1]\\Tool Hub\\toolhub.exe","mcp","serve",],"enabled":false,},},
        }"#;
        std::fs::write(root.join("mimocode.jsonc"), text).unwrap();
        let config = configuration(home.path());
        assert_eq!(config["configured"], true);
        assert_eq!(config["enabled"], false);
        assert_eq!(config["command"], r"D:\程序 [1]\Tool Hub\toolhub.exe");
        assert_eq!(config["args"], json!(["mcp", "serve"]));
        assert!(!config.to_string().contains("private"));
        let parsed = parse_jsonc(text).unwrap();
        assert_eq!(parsed["provider"]["url"], "https://example.test/a//b");
        assert_eq!(parsed["provider"]["quote"], "a\"/*b");
        assert!(parse_jsonc("{ /* unfinished").is_none());
        assert!(parse_jsonc("[1,,]").is_none());
    }
    #[test]
    fn malformed_mimo_config_is_explicit_not_a_connection_claim() {
        let home = tempfile::tempdir().unwrap();
        assert_eq!(configuration(home.path())["configured"], false);
        let root = home.path().join(".config/mimocode");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("mimocode.json");
        for text in [
            "{",
            r#"{"mcp":{"toolhub":{"type":"local","command":"toolhub mcp serve"}}}"#,
            r#"{"mcp":{"toolhub":{"type":"remote","url":"secret"}}}"#,
        ] {
            std::fs::write(&path, text).unwrap();
            let config = configuration(home.path());
            assert_eq!(config["configured"], false);
            assert!(config["error"].is_string());
            assert!(!config.to_string().contains("secret"));
        }
        std::fs::write(&path, vec![b' '; 1_048_577]).unwrap();
        assert!(configuration(home.path())["error"]
            .as_str()
            .unwrap()
            .contains("1 MiB"));
    }
    #[test]
    fn installation_icon_keeps_commas_unicode_and_index_separate() {
        let icon = r#""D:\软件,目录 [1]\Xiaomi MiMo\Xiaomi MiMo.exe",0"#;
        let candidates = installation_candidates("", icon);
        assert_eq!(
            candidates,
            vec![PathBuf::from(
                r"D:\软件,目录 [1]\Xiaomi MiMo\Xiaomi MiMo.exe"
            )]
        );
        assert!(installation_candidates("", r"D:\Tools\icon.dll,0").is_empty());
    }
}
