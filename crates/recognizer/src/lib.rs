//! Tool recognition from static metadata and reviewed known probes only.
//! Unknown executables are NEVER run (including --version/--help).

pub mod native_metadata;
pub mod resources;

use std::collections::BTreeMap;
use std::path::Path;

use toolhub_core::{
    validate_confidence, CapabilityId, CapabilityRegistry, DefinitionId, Evidence, EvidenceSource,
    InstanceId, Origin, Owner, ScanCandidate, ToolDefinition, ToolInstance, TrustRecord,
};

#[derive(Debug, Clone)]
pub struct RecognitionResult {
    pub recognized: bool,
    pub definition: Option<ToolDefinition>,
    pub instance: Option<ToolInstance>,
    pub evidence: Vec<Evidence>,
    pub confidence: f64,
    pub ambiguity: Option<String>,
}

#[derive(Debug, Clone)]
pub struct KnownToolRule {
    pub definition_id: &'static str,
    pub name: &'static str,
    pub file_names: &'static [&'static str],
    pub path_substrings: &'static [&'static str],
    pub capabilities: &'static [&'static str],
    pub vendor: &'static str,
}

/// Reviewed recognition rules (static metadata only).
pub const KNOWN_TOOLS: &[KnownToolRule] = &[
    KnownToolRule {
        definition_id: "org.python.python",
        name: "Python",
        file_names: &["python.exe", "python3", "python3.exe", "python"],
        path_substrings: &["python", "conda", "venv", "Programs/Python"],
        capabilities: &["language.python.execute"],
        vendor: "Python Software Foundation",
    },
    KnownToolRule {
        definition_id: "org.nodejs.node",
        name: "Node.js",
        file_names: &["node.exe", "node"],
        path_substrings: &["nodejs", "nvm", "node_modules/.bin"],
        capabilities: &["language.node.execute"],
        vendor: "OpenJS Foundation",
    },
    KnownToolRule {
        definition_id: "org.openjdk.java",
        name: "Java",
        file_names: &["java.exe", "java"],
        path_substrings: &["java", "jdk", "jbr", "openjdk"],
        capabilities: &["language.java.runtime"],
        vendor: "OpenJDK",
    },
    KnownToolRule {
        definition_id: "org.openjdk.javac",
        name: "Java 编译器",
        file_names: &["javac.exe", "javac"],
        path_substrings: &["java", "jdk", "openjdk"],
        capabilities: &["language.java.compile"],
        vendor: "OpenJDK",
    },
    KnownToolRule {
        definition_id: "org.rust.rustc",
        name: "Rustc",
        file_names: &["rustc.exe", "rustc"],
        path_substrings: &["rustup", "rust"],
        capabilities: &["language.rust.compile"],
        vendor: "Rust Project",
    },
    KnownToolRule {
        definition_id: "org.rust.cargo",
        name: "Cargo",
        file_names: &["cargo.exe", "cargo"],
        path_substrings: &["cargo", "rustup", "rust"],
        capabilities: &["language.rust.build"],
        vendor: "Rust Project",
    },
    KnownToolRule {
        definition_id: "org.go.go",
        name: "Go",
        file_names: &["go.exe", "go"],
        path_substrings: &["go/bin", "golang"],
        capabilities: &["language.go.compile"],
        vendor: "Go Project",
    },
    KnownToolRule {
        definition_id: "org.git.git",
        name: "Git",
        file_names: &["git.exe", "git"],
        path_substrings: &["Git/cmd", "Git/bin", "git-core"],
        capabilities: &["code.vcs.git"],
        vendor: "Git Project",
    },
    KnownToolRule {
        definition_id: "org.ffmpeg.ffmpeg",
        name: "FFmpeg",
        file_names: &["ffmpeg.exe", "ffmpeg"],
        path_substrings: &["ffmpeg"],
        capabilities: &[
            "media.video.transcode",
            "media.audio.transcode",
            "media.video.probe",
            "media.video.frame.extract",
        ],
        vendor: "FFmpeg",
    },
    KnownToolRule {
        definition_id: "org.docker.docker",
        name: "Docker",
        file_names: &["docker.exe", "docker"],
        path_substrings: &["docker", "Docker"],
        capabilities: &["container.run"],
        vendor: "Docker Inc.",
    },
    KnownToolRule {
        definition_id: "org.sqlite.sqlite",
        name: "SQLite",
        file_names: &["sqlite3.exe", "sqlite3"],
        path_substrings: &["sqlite"],
        capabilities: &["database.sqlite.query"],
        vendor: "SQLite",
    },
    KnownToolRule {
        definition_id: "org.android.adb",
        name: "Android Debug Bridge",
        file_names: &["adb.exe", "adb"],
        path_substrings: &["android", "platform-tools", "sdk"],
        capabilities: &["android.device.control"],
        vendor: "Google",
    },
    KnownToolRule {
        definition_id: "org.android.jadx",
        name: "JADX",
        file_names: &["jadx.exe", "jadx", "jadx-gui.exe"],
        path_substrings: &["jadx"],
        capabilities: &["android.apk.decompile", "binary.decompile.java"],
        vendor: "skylot",
    },
    KnownToolRule {
        definition_id: "org.llvm.clang",
        name: "Clang",
        file_names: &[
            "clang.exe",
            "clang",
            "clang-cl.exe",
            "gcc.exe",
            "gcc",
            "cl.exe",
        ],
        path_substrings: &["llvm", "mingw", "msvc", "Visual Studio", "gcc"],
        capabilities: &["language.c.compile", "language.cpp.compile"],
        vendor: "LLVM/GCC/MSVC",
    },
];

/// Windows App Execution Aliases under WindowsApps are tiny stubs, not full tools.
fn is_windows_store_alias(candidate: &ScanCandidate) -> bool {
    let path = candidate.path.replace('/', "\\").to_ascii_lowercase();
    // `...\Microsoft\WindowsApps\...` is reserved for Store app execution aliases.
    path.contains("\\windowsapps\\")
}

/// Install-location folders are not tool executables.
fn is_directory_install_location(candidate: &ScanCandidate) -> bool {
    let path = Path::new(&candidate.path);
    if candidate.path.ends_with('/') || candidate.path.ends_with('\\') {
        return true;
    }
    if path.is_dir() {
        return true;
    }
    // Uninstall InstallLocation without a file extension is typically a folder.
    let file_name = candidate
        .file_name
        .as_deref()
        .unwrap_or_else(|| path.file_name().and_then(|s| s.to_str()).unwrap_or(""));
    !file_name.contains('.') && path.extension().is_none() && path.is_dir()
}

/// Prefer PE/ELF versions, then scan/uninstall hints, then a normalized PE version string.
fn resolve_version(candidate: &ScanCandidate, native_version: Option<String>) -> Option<String> {
    if let Some(v) = native_version {
        return Some(v);
    }
    let hint = candidate
        .version_hint
        .clone()
        .or_else(|| {
            candidate
                .metadata
                .get("display_version")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        })
        .unwrap_or_default();
    if hint.is_empty() {
        return None;
    }
    Some(normalize_display_version(&hint))
}

/// Keep numeric x.y.z from strings like `2.47.1.windows.1`.
fn normalize_display_version(raw: &str) -> String {
    let trimmed = raw.trim().trim_start_matches('v');
    if toolhub_core::VersionConstraint::parse(&format!("={trimmed}")).is_ok() {
        return trimmed.to_string();
    }
    let mut buf = String::new();
    for c in trimmed.chars() {
        if c.is_ascii_digit() {
            buf.push(c);
        } else if c == '.' && !buf.is_empty() && !buf.ends_with('.') {
            buf.push(c);
        } else {
            break;
        }
    }
    let parts: Vec<&str> = buf
        .trim_end_matches('.')
        .split('.')
        .filter(|s| !s.is_empty())
        .take(3)
        .collect();
    if parts.is_empty() {
        trimmed.to_string()
    } else {
        parts.join(".")
    }
}

/// Recognize a candidate using static path/name metadata only.
pub fn recognize(candidate: &ScanCandidate) -> RecognitionResult {
    let mut evidence = vec![];
    if is_directory_install_location(candidate) {
        return RecognitionResult {
            recognized: false,
            definition: None,
            instance: None,
            evidence,
            confidence: 0.0,
            ambiguity: Some("install location folder is not a tool executable".into()),
        };
    }
    if is_windows_store_alias(candidate) {
        return RecognitionResult {
            recognized: false,
            definition: None,
            instance: None,
            evidence,
            confidence: 0.0,
            ambiguity: Some("windows store app execution alias is not a full tool install".into()),
        };
    }
    let file_name = candidate
        .file_name
        .clone()
        .unwrap_or_else(|| {
            Path::new(&candidate.path)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default()
        })
        .to_ascii_lowercase();

    let mut matches: Vec<&KnownToolRule> = vec![];
    for rule in KNOWN_TOOLS {
        let name_hit = rule
            .file_names
            .iter()
            .any(|n| n.eq_ignore_ascii_case(&file_name));
        let path_hit = rule.path_substrings.iter().any(|p| {
            candidate
                .path
                .to_ascii_lowercase()
                .contains(&p.to_ascii_lowercase())
        });
        if name_hit && path_hit {
            matches.push(rule);
            evidence.push(
                Evidence::native(
                    EvidenceSource::PathPattern,
                    0.75,
                    format!("path/name pattern matched {}", rule.name),
                )
                .unwrap(),
            );
        } else if name_hit {
            matches.push(rule);
            evidence.push(
                Evidence::native(
                    EvidenceSource::PathPattern,
                    0.55,
                    format!("file name matched {}", rule.name),
                )
                .unwrap(),
            );
        }
    }

    if matches.is_empty() {
        return RecognitionResult {
            recognized: false,
            definition: None,
            instance: None,
            evidence,
            confidence: 0.0,
            ambiguity: Some("unknown candidate preserved for inspection".into()),
        };
    }

    // Prefer the highest-priority unique rule; flag ambiguity when multiple names match.
    matches.sort_by_key(|r| r.definition_id);
    matches.dedup_by_key(|r| r.definition_id);
    let rule = matches[0];
    let ambiguity = if matches.len() > 1 {
        Some(format!(
            "multiple rules matched: {}",
            matches
                .iter()
                .map(|r| r.name)
                .collect::<Vec<_>>()
                .join(", ")
        ))
    } else {
        None
    };

    // F07/R2-B05: path/name alone is a weak hypothesis.
    // Known trust requires corroborating executable metadata (MZ/size), not a renamed text file.
    let native = native_metadata::read(Path::new(&candidate.path));
    if let Some(format) = &native.format {
        evidence.push(
            Evidence::native(
                EvidenceSource::ExecutableMetadata,
                0.85,
                format!("measured native format {format}; architecture {}; format does not prove product identity", native.arch.as_deref().unwrap_or("unknown")),
            )
            .unwrap(),
        );
    }
    let corroborated = native
        .product
        .as_deref()
        .map(|p| {
            p.eq_ignore_ascii_case(rule.name)
                || (rule.name == "Python" && p.to_ascii_lowercase().starts_with("python"))
        })
        .unwrap_or(false)
        && native
            .publisher
            .as_deref()
            .map(|p| p.eq_ignore_ascii_case(rule.vendor))
            .unwrap_or(false)
        && native
            .original_filename
            .as_deref()
            .map(|n| rule.file_names.iter().any(|f| f.eq_ignore_ascii_case(n)))
            .unwrap_or(false);
    if corroborated {
        evidence.push(
            Evidence::native(
                EvidenceSource::PublisherMetadata,
                0.85,
                format!(
                    "native product {}, publisher {}, original filename {}",
                    native.product.as_deref().unwrap(),
                    native.publisher.as_deref().unwrap(),
                    native.original_filename.as_deref().unwrap()
                ),
            )
            .unwrap(),
        );
    }
    let confidence = if !corroborated {
        0.4
    } else if ambiguity.is_some() {
        0.6
    } else {
        0.85
    };
    let confidence = validate_confidence(confidence).unwrap_or(0.35);
    let trust = if corroborated {
        TrustRecord::known(evidence.clone())
    } else {
        TrustRecord::unknown()
    };

    let caps: Vec<CapabilityId> = rule
        .capabilities
        .iter()
        .filter_map(|c| CapabilityId::new(*c).ok())
        .collect();

    let definition = ToolDefinition {
        id: DefinitionId::new(rule.definition_id).expect("static id"),
        name: rule.name.to_string(),
        categories: vec![],
        vendor: Some(rule.vendor.to_string()),
        capabilities: caps,
        description: None,
    };

    let platform = match native.format.as_deref() {
        Some("pe") => "windows",
        Some("elf") => "linux",
        Some("mach_o" | "mach_o_fat") => "macos",
        _ => "unknown",
    }
    .to_string();

    let instance = ToolInstance {
        id: InstanceId::new(stable_instance_id(
            candidate
                .canonical_path
                .as_deref()
                .unwrap_or(&candidate.path),
        ))
        .expect("stable id"),
        definition_id: definition.id.clone(),
        version: resolve_version(candidate, native.version.clone()),
        platform,
        arch: native.arch.unwrap_or_else(|| "unknown".into()),
        path: candidate.path.clone(),
        canonical_path: candidate.canonical_path.clone(),
        environment_id: None,
        origin: infer_origin(&candidate.path),
        owner: Owner::from_directory_hint(&candidate.path, rule.name),
        trust,
        interfaces: vec![],
        evidence: evidence.clone(),
        status: toolhub_core::InstanceStatus::Available,
        first_seen: chrono::Utc::now(),
        last_seen: chrono::Utc::now(),
    };

    RecognitionResult {
        recognized: true,
        definition: Some(definition),
        instance: Some(instance),
        evidence,
        confidence,
        ambiguity,
    }
}

/// Active data rules participate in the real recognition path; trust still requires native identity.
pub fn recognize_with_resources(
    candidate: &ScanCandidate,
    store: &resources::ResourceStore,
) -> RecognitionResult {
    if is_directory_install_location(candidate) || is_windows_store_alias(candidate) {
        return RecognitionResult {
            recognized: false,
            definition: None,
            instance: None,
            evidence: vec![],
            confidence: 0.0,
            ambiguity: Some("not a full tool executable".into()),
        };
    }
    let file = candidate.file_name.as_deref().unwrap_or("");
    let Some(rule) = store
        .rules()
        .into_iter()
        .find(|r| r.file_names.iter().any(|n| n.eq_ignore_ascii_case(file)))
    else {
        return recognize(candidate);
    };
    let native = native_metadata::read(Path::new(&candidate.path));
    let mut evidence = vec![Evidence::native(
        EvidenceSource::PathPattern,
        0.4,
        format!("active resource name hypothesis for {}", rule.name),
    )
    .unwrap()];
    let known = native.product.as_deref() == Some(rule.name.as_str())
        && native.publisher.as_deref() == Some(rule.vendor.as_str())
        && native
            .original_filename
            .as_deref()
            .is_some_and(|n| rule.file_names.iter().any(|f| f.eq_ignore_ascii_case(n)));
    if native.format.is_some() {
        evidence.push(
            Evidence::native(
                EvidenceSource::ExecutableMetadata,
                0.8,
                format!(
                    "measured {} {}",
                    native.format.as_deref().unwrap_or("unknown"),
                    native.arch.as_deref().unwrap_or("unknown")
                ),
            )
            .unwrap(),
        )
    }
    if known {
        evidence.push(
            Evidence::native(
                EvidenceSource::PublisherMetadata,
                0.85,
                "native product/publisher/original filename corroborate active rule",
            )
            .unwrap(),
        )
    }
    let definition = ToolDefinition {
        id: DefinitionId::new(&rule.definition_id).unwrap(),
        name: rule.name.clone(),
        vendor: Some(rule.vendor),
        categories: vec![],
        capabilities: rule
            .capabilities
            .iter()
            .map(|c| CapabilityId::new(c).unwrap())
            .collect(),
        description: Some("active recognition resource".into()),
    };
    let instance = ToolInstance {
        id: InstanceId::new(stable_instance_id(
            candidate
                .canonical_path
                .as_deref()
                .unwrap_or(&candidate.path),
        ))
        .unwrap(),
        definition_id: definition.id.clone(),
        version: resolve_version(candidate, native.version),
        platform: match native.format.as_deref() {
            Some("pe") => "windows",
            Some("elf") => "linux",
            Some("mach_o" | "mach_o_fat") => "macos",
            _ => "unknown",
        }
        .into(),
        arch: native.arch.unwrap_or_else(|| "unknown".into()),
        path: candidate.path.clone(),
        canonical_path: candidate.canonical_path.clone(),
        environment_id: None,
        origin: infer_origin(&candidate.path),
        owner: Owner::unknown(),
        trust: if known {
            TrustRecord::known(evidence.clone())
        } else {
            TrustRecord::unknown()
        },
        interfaces: vec![],
        evidence: evidence.clone(),
        status: toolhub_core::InstanceStatus::Available,
        first_seen: chrono::Utc::now(),
        last_seen: chrono::Utc::now(),
    };
    RecognitionResult {
        recognized: true,
        definition: Some(definition),
        instance: Some(instance),
        evidence,
        confidence: if known { 0.85 } else { 0.4 },
        ambiguity: if known {
            None
        } else {
            Some("resource rule is a hypothesis; independent product identity missing".into())
        },
    }
}

/// F06: stable instance identity from normalized path (not per-discovery UUID).
pub fn stable_instance_id(path: &str) -> String {
    let norm = toolhub_core::normalize_path(path);
    let fp = toolhub_core::path_fingerprint(&norm);
    format!("inst-{}", &fp[..16])
}

fn infer_origin(path: &str) -> Origin {
    let p = path.to_ascii_lowercase();
    if p.contains("homebrew") || p.contains("cellar") {
        Origin::PackageManager {
            manager: "homebrew".into(),
        }
    } else if p.contains("scoop") {
        Origin::PackageManager {
            manager: "scoop".into(),
        }
    } else if p.contains("chocolatey") {
        Origin::PackageManager {
            manager: "chocolatey".into(),
        }
    } else if p.contains("winget") {
        Origin::PackageManager {
            manager: "winget".into(),
        }
    } else if p.contains("conda") || p.contains("venv") {
        Origin::ProjectLocal
    } else if p.contains("cursor") || p.contains("opencode") || p.contains("codex") {
        Origin::AgentSandbox {
            agent: "unknown-agent".into(),
        }
    } else {
        Origin::Unknown
    }
}

/// Map recognition results into registry inputs (used by daemon).
pub fn to_instance_json(result: &RecognitionResult) -> Option<(String, String, String)> {
    let def = result.definition.as_ref()?;
    let inst = result.instance.as_ref()?;
    Some((
        inst.id.as_str().to_string(),
        def.id.as_str().to_string(),
        def.name.clone(),
    ))
}

/// Build capability registry core taxonomy for resolver use.
pub fn core_capabilities() -> CapabilityRegistry {
    CapabilityRegistry::with_core_taxonomy()
}

/// Known probe rules: only for tools with reviewed allowlists.
/// This function intentionally does NOT spawn processes.
pub fn known_probe_allowed(file_name: &str) -> Option<(&'static str, &'static [&'static str])> {
    match file_name.to_ascii_lowercase().as_str() {
        "python.exe" | "python3" | "python" => Some(("python", &["--version"])),
        "node.exe" | "node" => Some(("node", &["--version"])),
        "git.exe" | "git" => Some(("git", &["--version"])),
        "ffmpeg.exe" | "ffmpeg" => Some(("ffmpeg", &["-version"])),
        "java.exe" | "java" => Some(("java", &["-version"])),
        "go.exe" | "go" => Some(("go", &["version"])),
        "rustc.exe" | "rustc" => Some(("rustc", &["--version"])),
        "cargo.exe" | "cargo" => Some(("cargo", &["--version"])),
        "docker.exe" | "docker" => Some(("docker", &["--version"])),
        _ => None,
    }
}

/// Unknown executable fixture helper for tests: would leave a marker if executed.
pub struct UnknownProbeGuard;

impl UnknownProbeGuard {
    pub fn should_execute(_path: &str) -> bool {
        // Product invariant: never auto-execute unknown binaries.
        false
    }
}

#[allow(dead_code)]
fn _unused_map() -> BTreeMap<String, String> {
    BTreeMap::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_pe_format_is_not_product_identity() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("python.exe");
        let mut bytes = vec![0u8; 2048];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[0x3c..0x40].copy_from_slice(&128u32.to_le_bytes());
        bytes[128..132].copy_from_slice(b"PE\0\0");
        bytes[132..134].copy_from_slice(&0xaa64u16.to_le_bytes());
        std::fs::write(&path, bytes).unwrap();
        let instance = recognize(&cand(&path.to_string_lossy())).instance.unwrap();
        assert_eq!(instance.trust.level, toolhub_core::TrustLevel::Unknown);
        assert_eq!(instance.arch, "aarch64");
        assert_eq!(instance.version, None);
    }

    fn cand(path: &str) -> ScanCandidate {
        let mut c = ScanCandidate::from_path(path);
        c.file_name = Some(
            Path::new(path)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string(),
        );
        c
    }

    #[test]
    fn recognizes_python_from_path() {
        let r = recognize(&cand(
            "C:/Users/x/AppData/Local/Programs/Python/Python313/python.exe",
        ));
        assert!(r.recognized);
        assert_eq!(r.definition.unwrap().id.as_str(), "org.python.python");
    }

    #[test]
    fn renamed_text_python_is_not_known() {
        let dir = std::env::temp_dir().join("toolhub-r2b05");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("python.exe");
        std::fs::write(&path, b"this is not a real binary").unwrap();
        let r = recognize(&cand(&path.to_string_lossy()));
        // May match the rule by basename, but trust must remain unknown (no MZ).
        if r.recognized {
            let inst = r.instance.unwrap();
            let level = inst.trust.level;
            assert_ne!(level, toolhub_core::TrustLevel::Known);
            assert_ne!(level, toolhub_core::TrustLevel::Verified);
        }
    }

    #[test]
    fn unknown_stays_unrecognized() {
        let dir = std::env::temp_dir().join("toolhub-unknown-marker-test");
        let path = dir.join("totally-unknown-binary-xyz.exe");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(&path, b"marker-if-executed").unwrap();
        let r = recognize(&cand(&path.to_string_lossy()));
        assert!(!r.recognized);
        assert!(!UnknownProbeGuard::should_execute(&path.to_string_lossy()));
    }
}
