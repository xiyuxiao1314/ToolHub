//! Bounded local Skill packages. Reading/copying never runs a script or grants authority.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

pub const MAX_FILES: usize = 256;
pub const MAX_FILE: usize = 4 * 1024 * 1024;
pub const MAX_PACKAGE: usize = 16 * 1024 * 1024;
pub type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub name: String,
    pub description: String,
    pub version: String,
    pub host_references: Vec<String>,
}
pub struct Package {
    pub metadata: Metadata,
    pub files: BTreeMap<String, Vec<u8>>,
    pub digest: String,
    pub bytes: usize,
}
pub fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && !name.starts_with('-')
        && !name.ends_with('-')
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !["con", "prn", "aux", "nul", "synced", "anthropic-skills"].contains(&name)
        && !["com", "lpt"].iter().any(|p| {
            name.strip_prefix(p)
                .is_some_and(|v| v.len() == 1 && v.bytes().all(|b| b.is_ascii_digit()))
        })
}
fn plain_scalar(value: &str) -> String {
    let v = value.trim();
    if v.starts_with('"') {
        serde_json::from_str::<String>(v).unwrap_or_default()
    } else {
        v.strip_prefix('\'')
            .and_then(|v| v.strip_suffix('\''))
            .unwrap_or(v)
            .replace("''", "'")
    }
}
// Read the small standard string fields, preserving the original complete frontmatter verbatim.
// Complex/nonstandard names are rejected rather than rewritten with a guessed interpretation.
fn field(front: &str, key: &str) -> String {
    let mut lines = front.lines().peekable();
    while let Some(line) = lines.next() {
        if let Some(v) = line.strip_prefix(&format!("{key}:")) {
            if ["|", ">", "|-", ">-", "|+", ">+"].contains(&v.trim()) {
                let mut block = Vec::new();
                while lines
                    .peek()
                    .is_some_and(|l| l.starts_with(' ') || l.trim().is_empty())
                {
                    block.push(lines.next().unwrap().trim());
                }
                return block.join(if v.trim().starts_with('>') { " " } else { "\n" });
            }
            return plain_scalar(v);
        }
    }
    String::new()
}
pub fn metadata(text: &str) -> Result<Metadata> {
    let text = text.trim_start_matches('\u{feff}');
    let normalized = text.replace("\r\n", "\n");
    let after = normalized
        .strip_prefix("---\n")
        .ok_or("SKILL.md 缺少 YAML 元信息（name、description）")?;
    let end = after
        .lines()
        .scan(0usize, |offset, line| {
            let start = *offset;
            *offset += line.len() + 1;
            Some((start, line))
        })
        .find(|(_, l)| *l == "---")
        .map(|(n, _)| n)
        .ok_or("SKILL.md YAML 元信息未正确结束")?;
    let front = &after[..end];
    if front.len() > 8192
        || ["name:", "description:"]
            .iter()
            .any(|key| front.lines().filter(|l| l.starts_with(key)).count() != 1)
    {
        return Err("Skill 元信息过长或 name/description 重复".into());
    }
    let name = field(front, "name");
    let description = field(front, "description");
    if !safe_name(&name) || description.trim().is_empty() || description.chars().count() > 1024 {
        return Err(
            "Skill 名称需为 1–64 位小写英文/数字/连字符；description 不能为空且限 1024 字".into(),
        );
    }
    let version = field(front, "version");
    let lower = text.to_lowercase();
    let mut host_references: Vec<String> = [
        "image_gen",
        "imagegen.imagegen",
        "mcp__node_repl",
        "mcp__cua_repl",
        "functions.exec",
        "codex_apps",
        "@oai/sky",
    ]
    .into_iter()
    .filter(|v| lower.contains(v))
    .map(str::to_owned)
    .collect();
    for key in ["allowed-tools", "hooks", "context", "agent"] {
        if front.lines().any(|l| l.starts_with(&format!("{key}:"))) {
            host_references.push(format!("宿主声明 {key}"));
        }
    }
    Ok(Metadata {
        name,
        description,
        version,
        host_references,
    })
}
pub fn read_bounded(path: &Path, max: usize) -> Result<Vec<u8>> {
    let meta =
        fs::symlink_metadata(path).map_err(|e| format!("无法读取 {}：{e}", path.display()))?;
    if linked(&meta) || !meta.is_file() || meta.len() > max as u64 {
        return Err("文件类型、链接或大小超出允许范围".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(max as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > max {
        return Err("文件读取期间超过大小上限".into());
    }
    Ok(bytes)
}
pub fn linked(m: &fs::Metadata) -> bool {
    if m.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if m.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    false
}
pub fn checked_dir(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute() {
        return Err("需要绝对目录路径".into());
    }
    let mut current = PathBuf::new();
    for part in path.components() {
        if matches!(
            part,
            std::path::Component::ParentDir | std::path::Component::CurDir
        ) {
            return Err("目录不能包含 . 或 ..".into());
        }
        current.push(part);
        if let Ok(m) = fs::symlink_metadata(&current) {
            if linked(&m) {
                return Err("Skill 路径不能经过符号链接或目录联接".into());
            }
        }
    }
    let m = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !m.is_dir() || linked(&m) {
        return Err("需要普通目录".into());
    }
    path.canonicalize().map_err(|e| e.to_string())
}
fn walk(
    root: &Path,
    dir: &Path,
    depth: usize,
    files: &mut BTreeMap<String, Vec<u8>>,
    total: &mut usize,
    entries: &mut usize,
) -> Result<()> {
    if depth > 12 {
        return Err("Skill 子目录层级超过 12".into());
    }
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        *entries += 1;
        if *entries > 1024 {
            return Err("Skill 超过 1024 个目录/文件条目".into());
        }
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        // Repository/cache material is not part of a distributable Skill.
        if [".git", "node_modules", "__pycache__", ".DS_Store"].contains(&name.as_str()) {
            continue;
        }
        if name.starts_with(".env")
            || ["credentials.json", "auth.json", "id_rsa", "id_ed25519"].contains(&name.as_str())
            || name.ends_with(".pem")
            || name.ends_with(".key")
        {
            return Err(format!("包内含可能的凭据文件：{name}，请先整理源目录"));
        }
        if name.ends_with([' ', '.'])
            || name.contains([':', '\\'])
            || name.chars().any(char::is_control)
        {
            return Err("不支持的跨平台文件名".into());
        }
        let m = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        if linked(&m) {
            return Err(format!("包内链接不会分发：{name}"));
        }
        if m.is_dir() {
            walk(root, &path, depth + 1, files, total, entries)?;
        } else if m.is_file() {
            if files.len() >= MAX_FILES {
                return Err("Skill 超过 256 个文件".into());
            }
            let bytes = read_bounded(&path, MAX_FILE)?;
            *total += bytes.len();
            if *total > MAX_PACKAGE {
                return Err("Skill 超过 16 MiB".into());
            }
            let relative = path
                .strip_prefix(root)
                .map_err(|e| e.to_string())?
                .to_str()
                .ok_or("路径不是 Unicode")?
                .replace('\\', "/");
            files.insert(relative, bytes);
        } else {
            return Err("包中只允许普通文件和目录".into());
        }
    }
    Ok(())
}
pub fn read_package(path: &Path) -> Result<Package> {
    let root = checked_dir(path)?;
    let mut files = BTreeMap::new();
    let mut bytes = 0;
    walk(&root, &root, 0, &mut files, &mut bytes, &mut 0)?;
    let text = std::str::from_utf8(files.get("SKILL.md").ok_or("目录缺少 SKILL.md")?)
        .map_err(|_| "SKILL.md 必须是 UTF-8")?;
    if text.len() > 65536 {
        return Err("SKILL.md 超过 64 KiB".into());
    }
    let mut metadata = match metadata(text) {
        Ok(m) => m,
        Err(error) => {
            if text.trim_start().starts_with("---") {
                return Err(error);
            }
            let declared: toolhub_core::SkillManifest = toolhub_core::SkillManifest::validate_json(
                &serde_json::from_slice(files.get("skill.json").ok_or(error)?)
                    .map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            if declared.instruction_file.as_deref() != Some("SKILL.md")
                || !declared
                    .library_compatibility(std::env::consts::OS, Some(text))
                    .reusable
            {
                return Err("旧版流程缺少可复用声明，不能生成分发包".into());
            }
            let name = declared.id.as_str().replace('.', "-");
            if !safe_name(&name) {
                return Err("旧版流程 ID 无法转换为标准 Skill 名称".into());
            }
            let header = format!(
                "---\nname: {name}\ndescription: {}\n---\n\n{}",
                serde_json::to_string(&declared.description).map_err(|e| e.to_string())?,
                text
            );
            files.insert("SKILL.md".into(), header.as_bytes().to_vec());
            metadata(&header)?
        }
    };
    for (path, data) in &files {
        if path == "SKILL.md" {
            continue;
        }
        if let Ok(text) = std::str::from_utf8(data) {
            let lower = text.to_lowercase();
            for token in [
                "image_gen",
                "mcp__node_repl",
                "mcp__cua_repl",
                "functions.exec",
                "codex_apps",
                "@oai/sky",
            ] {
                if lower.contains(token) && !metadata.host_references.iter().any(|s| s == token) {
                    metadata.host_references.push(token.into());
                }
            }
        }
        if path == "skill.json" {
            let manifest: toolhub_core::SkillManifest = toolhub_core::SkillManifest::validate_json(
                &serde_json::from_slice(data).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            if let Some(p) = manifest.portability {
                if p.scope == toolhub_core::skill_portability::SkillScope::HostSpecific
                    || !p.host_dependencies.is_empty()
                {
                    metadata.host_references.push("skill.json 宿主依赖".into());
                }
                if !p.platforms.iter().any(|p| {
                    serde_json::to_value(p)
                        .ok()
                        .as_ref()
                        .and_then(|v| v.as_str())
                        == Some(std::env::consts::OS)
                }) {
                    metadata
                        .host_references
                        .push("skill.json 平台不适用".into());
                }
            }
        }
    }
    let bytes = files.values().map(Vec::len).sum::<usize>();
    if bytes > MAX_PACKAGE {
        return Err("规范化后的 Skill 超过 16 MiB".into());
    }
    let digest = digest_files(&files);
    Ok(Package {
        metadata,
        files,
        digest,
        bytes,
    })
}
pub fn digest_files(files: &BTreeMap<String, Vec<u8>>) -> String {
    let mut h = Sha256::new();
    for (name, data) in files {
        h.update((name.len() as u64).to_le_bytes());
        h.update(name.as_bytes());
        h.update((data.len() as u64).to_le_bytes());
        h.update(data);
    }
    hex::encode(h.finalize())
}
pub fn write_new(root: &Path, package: &Package) -> Result<()> {
    fs::create_dir(root).map_err(|e| format!("新建目录失败（保留已有内容）：{e}"))?;
    for (relative, data) in &package.files {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| e.to_string())?;
        file.write_all(data).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub fn preview_directory(root: &Path) -> Result<Metadata> {
    let root = checked_dir(root)?;
    let bytes = read_bounded(&root.join("SKILL.md"), 65536)?;
    metadata(std::str::from_utf8(&bytes).map_err(|_| "SKILL.md 必须是 UTF-8")?)
}

#[cfg(test)]
mod tests {
    use super::*;
    // macOS exposes TMPDIR through /var -> /private/var. Fixtures must use
    // the physical parent; production continues rejecting every symlink ancestor.
    fn fixture_dir() -> tempfile::TempDir {
        let parent = std::env::temp_dir().canonicalize().unwrap();
        tempfile::tempdir_in(parent).unwrap()
    }
    fn fixture(root: &Path) {
        fs::create_dir_all(root.join("references")).unwrap();
        fs::write(root.join("SKILL.md"),"---\nname: test-flow\ndescription: >-\n  Test workflow\n  with a reference.\n---\nRead references/a.md").unwrap();
        fs::write(root.join("references/a.md"), "reference").unwrap();
    }
    #[test]
    fn complete_package_digest_and_bounded_parser() {
        let d = fixture_dir();
        fixture(d.path());
        let p = read_package(d.path()).unwrap();
        assert_eq!(p.metadata.description, "Test workflow with a reference.");
        assert_eq!(p.files.len(), 2);
        let other = d.path().join("copy");
        write_new(&other, &p).unwrap();
        assert_eq!(read_package(&other).unwrap().digest, p.digest);
        assert!(write_new(&other, &p).is_err());
        assert!(!safe_name("../bad"));
        assert!(!safe_name("CON"));
        assert!(!safe_name("com1"));
    }
    #[test]
    fn credentials_size_and_supporting_host_reference() {
        let d = fixture_dir();
        fixture(d.path());
        fs::write(d.path().join("references/a.md"), "call image_gen").unwrap();
        assert!(!read_package(d.path())
            .unwrap()
            .metadata
            .host_references
            .is_empty());
        fs::write(d.path().join(".env"), "SECRET=x").unwrap();
        assert!(read_package(d.path()).is_err());
        fs::remove_file(d.path().join(".env")).unwrap();
        fs::write(d.path().join("big"), vec![0; MAX_FILE + 1]).unwrap();
        assert!(read_package(d.path()).is_err());
    }
    #[test]
    fn empty_and_malformed_frontmatter_rejected() {
        for s in [
            "name: test",
            "---\nname: test\n---\nbody",
            "---\nname: ../escape\ndescription: test\n---\n",
        ] {
            assert!(metadata(s).is_err());
        }
    }
    #[cfg(unix)]
    #[test]
    fn physical_temp_parent_works_but_custom_symlink_ancestors_are_rejected() {
        let d = fixture_dir();
        let physical = d.path().join("physical");
        fixture(&physical);
        assert!(read_package(&physical).is_ok());
        let alias = d.path().join("alias");
        std::os::unix::fs::symlink(&physical, &alias).unwrap();
        assert!(checked_dir(&alias).is_err());
        assert!(checked_dir(&alias.join("references")).is_err());
        assert!(read_package(&alias).is_err());
    }
}

/// Metadata-only quota scan of our private cache, with link and entry-count bounds.
pub fn tree_bytes(root: &Path, budget: u64) -> Result<u64> {
    fn visit(path: &Path, total: &mut u64, entries: &mut usize, budget: u64) -> Result<()> {
        *entries += 1;
        if *entries > 65536 {
            return Err("同步缓存条目过多，请先整理历史快照/备份".into());
        }
        let meta = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
        if linked(&meta) {
            return Err("同步缓存含链接，停止写入".into());
        }
        if meta.is_dir() {
            for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
                visit(
                    &entry.map_err(|e| e.to_string())?.path(),
                    total,
                    entries,
                    budget,
                )?;
            }
        } else if meta.is_file() {
            *total += meta.len();
            if *total > budget {
                return Err("同步缓存配额已满；历史快照和备份已保留，请先人工整理".into());
            }
        } else {
            return Err("同步缓存含特殊文件".into());
        }
        Ok(())
    }
    let mut total = 0;
    visit(root, &mut total, &mut 0, budget)?;
    Ok(total)


}
