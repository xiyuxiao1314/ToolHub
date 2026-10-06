use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};
pub use crate::id::CapabilityId;

/// Canonical capability identifier: dotted lowercase namespace.
///
/// Video frame extraction canonical spelling: `media.video.frame.extract`.
/// Legacy design examples such as `media.frame.extract` are aliases only.
pub fn is_canonical_capability(id: &str) -> bool {
    if id.is_empty() || id.len() > 128 {
        return false;
    }
    if !id.starts_with(char::is_lowercase) {
        return false;
    }
    id.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.')
        && !id.contains("..")
        && !id.ends_with('.')
        && id.contains('.')
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityRequirement {
    pub capability: CapabilityId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default)]
    pub optional: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapabilityRegistry {
    /// canonical id -> description
    capabilities: std::collections::BTreeMap<String, String>,
    /// alias -> canonical
    aliases: std::collections::BTreeMap<String, String>,
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed the core taxonomy used by B01 fixtures and resolution tests.
    pub fn with_core_taxonomy() -> Self {
        let mut r = Self::new();
        for (id, desc) in CORE_CAPABILITIES {
            r.register(id, desc).expect("core taxonomy valid");
        }
        for (alias, canonical) in CORE_ALIASES {
            r.add_alias(alias, canonical).expect("core aliases valid");
        }
        r
    }

    pub fn register(&mut self, id: &str, description: &str) -> CoreResult<CapabilityId> {
        if !is_canonical_capability(id) {
            return Err(CoreError::InvalidCapability(id.to_string()));
        }
        if let Some(existing) = self.aliases.get(id) {
            return Err(CoreError::AliasConflict(format!(
                "{id} is already an alias of {existing}"
            )));
        }
        self.capabilities
            .insert(id.to_string(), description.to_string());
        CapabilityId::new(id)
    }

    pub fn add_alias(&mut self, alias: &str, canonical: &str) -> CoreResult<()> {
        if !self.capabilities.contains_key(canonical) {
            return Err(CoreError::UnknownCapability(canonical.to_string()));
        }
        if self.capabilities.contains_key(alias) {
            return Err(CoreError::AliasConflict(format!(
                "{alias} is a canonical capability"
            )));
        }
        // Prevent alias cycles: alias cannot point to another alias chain that loops.
        let mut seen = std::collections::BTreeSet::new();
        let mut cur = canonical.to_string();
        while let Some(next) = self.aliases.get(&cur) {
            if !seen.insert(cur.clone()) {
                return Err(CoreError::AliasConflict(format!(
                    "cycle involving {alias} -> {canonical}"
                )));
            }
            cur = next.clone();
        }
        self.aliases
            .insert(alias.to_string(), canonical.to_string());
        Ok(())
    }

    /// Deterministic alias resolution to a canonical capability id.
    pub fn resolve_id(&self, raw: &str) -> CoreResult<CapabilityId> {
        if self.capabilities.contains_key(raw) {
            return CapabilityId::new(raw);
        }
        if let Some(canon) = self.aliases.get(raw) {
            return CapabilityId::new(canon.clone());
        }
        if is_canonical_capability(raw) {
            // Unknown but well-formed: report unknown rather than invent a provider.
            return Err(CoreError::UnknownCapability(raw.to_string()));
        }
        Err(CoreError::InvalidCapability(raw.to_string()))
    }

    pub fn canonical_of<'a>(&'a self, raw: &str) -> Option<&'a str> {
        if let Some((k, _)) = self.capabilities.get_key_value(raw) {
            Some(k.as_str())
        } else {
            self.aliases.get(raw).map(|s| s.as_str())
        }
    }

    pub fn contains(&self, id: &str) -> bool {
        self.capabilities.contains_key(id) || self.aliases.contains_key(id)
    }

    pub fn list_canonical(&self) -> Vec<&str> {
        self.capabilities.keys().map(|s| s.as_str()).collect()
    }

    pub fn aliases_for(&self, canonical: &str) -> Vec<&str> {
        self.aliases
            .iter()
            .filter(|(_, c)| c.as_str() == canonical)
            .map(|(a, _)| a.as_str())
            .collect()
    }
}

pub const CORE_CAPABILITIES: &[(&str, &str)] = &[
    ("language.python.execute", "Execute Python code"),
    ("language.node.execute", "Execute Node.js code"),
    ("language.java.runtime", "Run Java programs"),
    ("language.rust.compile", "Compile Rust code"),
    ("language.go.compile", "Compile Go code"),
    ("language.c.compile", "Compile C code"),
    ("language.cpp.compile", "Compile C++ code"),
    ("code.vcs.git", "Git version control"),
    ("media.video.transcode", "Transcode video"),
    ("media.audio.transcode", "Transcode audio"),
    ("media.video.probe", "Probe media metadata"),
    ("media.video.frame.extract", "Extract video frames"),
    ("media.image.convert", "Convert images"),
    ("document.convert", "Convert documents"),
    ("document.pdf.render", "Render PDF pages"),
    ("document.pdf.text.extract", "Extract text from PDF"),
    ("document.ocr", "Recognize text in images"),
    (
        "media.video.download",
        "Download media from supported sources",
    ),
    ("archive.extract", "Extract archives"),
    ("archive.create", "Create archives"),
    ("container.run", "Run container workloads"),
    ("database.sqlite.query", "Query SQLite databases"),
    ("android.device.control", "Control Android devices"),
    ("android.apk.decompile", "Decompile Android APK"),
    ("binary.decompile.java", "Decompile Java bytecode"),
    ("network.http.request", "Perform HTTP requests"),
    ("agent.mcp.serve", "Serve MCP tools"),
    ("agent.cli.drive", "Drive a CLI coding agent"),
];

pub const CORE_ALIASES: &[(&str, &str)] = &[
    // Legacy design example spellings retained only as aliases.
    ("media.frame.extract", "media.video.frame.extract"),
    ("media.video.extract.frame", "media.video.frame.extract"),
    ("video.transcode", "media.video.transcode"),
    ("python.execute", "language.python.execute"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_spelling_for_frame_extract() {
        let reg = CapabilityRegistry::with_core_taxonomy();
        let id = reg.resolve_id("media.video.frame.extract").unwrap();
        assert_eq!(id.as_str(), "media.video.frame.extract");
        let legacy = reg.resolve_id("media.frame.extract").unwrap();
        assert_eq!(legacy.as_str(), "media.video.frame.extract");
    }

    #[test]
    fn unknown_capability_is_not_invented() {
        let reg = CapabilityRegistry::with_core_taxonomy();
        assert!(matches!(
            reg.resolve_id("totally.unknown.capability"),
            Err(CoreError::UnknownCapability(_))
        ));
    }

    #[test]
    fn rejects_alias_cycle() {
        let mut reg = CapabilityRegistry::new();
        reg.register("a.b.c", "x").unwrap();
        reg.add_alias("x.y.z", "a.b.c").unwrap();
        assert!(reg.add_alias("a.b.c", "x.y.z").is_err());
    }
}
