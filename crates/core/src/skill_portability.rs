//! Declared portability is compatibility metadata, never execution authority.
use crate::{CoreError, CoreResult, SkillManifest};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillPortability {
    pub scope: SkillScope,
    pub platforms: Vec<SkillPlatform>,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub permissions: Vec<SkillPermission>,
    pub host_dependencies: Vec<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillScope {
    Portable,
    HostSpecific,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillPlatform {
    Windows,
    Macos,
    Linux,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillPermission {
    ReadFiles,
    WriteFiles,
    ExecuteTools,
    NetworkAccess,
}
#[derive(Debug, Clone, Serialize)]
pub struct SkillCompatibility {
    pub status: String,
    pub reusable: bool,
    pub issues: Vec<String>,
    pub assessment: &'static str,
}
impl SkillPortability {
    pub fn validate(&self) -> CoreResult<()> {
        if self.platforms.is_empty() || self.platforms.len() > 3 || self.permissions.len() > 4 {
            return Err(CoreError::Validation(
                "通用 Skill 需要明确声明适用平台和权限范围".into(),
            ));
        }
        for (name, items, required) in [
            ("inputs", &self.inputs, true),
            ("outputs", &self.outputs, true),
            ("host_dependencies", &self.host_dependencies, false),
        ] {
            if items.len() > 32
                || (required && items.is_empty())
                || items
                    .iter()
                    .any(|s| s.trim().is_empty() || s.len() > 4096 || s.contains('\0'))
            {
                return Err(CoreError::Validation(format!(
                    "Skill {name} 必须是有界的非空文字数组"
                )));
            }
        }
        Ok(())
    }
}
impl SkillManifest {
    pub fn library_compatibility(
        &self,
        platform: &str,
        instructions: Option<&str>,
    ) -> SkillCompatibility {
        let result = |status: &str, issues: Vec<String>| SkillCompatibility {
            status: status.into(),
            reusable: status == "portable",
            issues,
            assessment: "声明与已知宿主工具引用检查；不保证任意正文已证明可移植，也不授予执行权限",
        };
        let Some(p) = &self.portability else {
            return result(
                "needs_review",
                vec!["未声明通用性、平台、输入输出和权限；请补齐 skill.json".into()],
            );
        };
        if p.scope == SkillScope::HostSpecific || !p.host_dependencies.is_empty() {
            return result(
                "host_specific",
                vec!["声明依赖宿主专属能力，请留在对应 Agent 的 Skill 目录".into()],
            );
        }
        if self.description.trim().is_empty() || p.validate().is_err() {
            return result("needs_review", vec!["用途或通用技能声明不完整".into()]);
        }
        if self.kind != crate::skill::SkillKind::Instruction {
            return result(
                "needs_review",
                vec!["通用库收录流程说明；独立脚本/服务通过 requires 声明工具能力".into()],
            );
        }
        let Some(text) = instructions.filter(|s| !s.trim().is_empty()) else {
            return result(
                "needs_review",
                vec!["缺少可读取的流程说明文件（限 64 KiB）".into()],
            );
        };
        let lower = text.to_lowercase();
        let references = [
            "image_gen",
            "imagegen.imagegen",
            "mcp__node_repl",
            "mcp__cua_repl",
            "functions.exec",
            "codex_apps",
            "@oai/sky",
        ]
        .into_iter()
        .filter(|token| lower.contains(token))
        .collect::<Vec<_>>();
        if !references.is_empty() {
            return result("needs_review",vec![format!("发现已知宿主工具引用：{}。请核对并替换为独立工具能力；文字引用本身不证明实际依赖",references.join(", "))]);
        }
        let supported = p.platforms.iter().any(|p| {
            matches!(
                (p, platform),
                (SkillPlatform::Windows, "windows")
                    | (SkillPlatform::Macos, "macos")
                    | (SkillPlatform::Linux, "linux")
            )
        });
        if !supported {
            return result(
                "unsupported_platform",
                vec![format!("当前平台 {platform} 不在声明范围内")],
            );
        }
        result("portable", vec![])
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn manifest() -> SkillManifest {
        SkillManifest::validate_json(&json!({"schema":"toolhub.skill/v1","id":"generic.video","name":"Video","description":"Video compression","instruction_file":"SKILL.md","portability":{"scope":"portable","platforms":["windows"],"inputs":["video"],"outputs":["compressed video"],"permissions":["read_files","write_files","execute_tools"],"host_dependencies":[]}})).unwrap()
    }
    #[test]
    fn checks_declarations_platform_and_actual_host_references() {
        let mut m = manifest();
        assert!(
            m.library_compatibility(
                "windows",
                Some("Created by ChatGPT or MiMo; use FFmpeg via ToolHub")
            )
            .reusable
        );
        assert_eq!(
            m.library_compatibility("macos", Some("Use FFmpeg")).status,
            "unsupported_platform"
        );
        assert_eq!(
            m.library_compatibility("windows", Some("Call image_gen to render"))
                .status,
            "needs_review"
        );
        m.portability
            .as_mut()
            .unwrap()
            .host_dependencies
            .push("chatgpt.image_gen".into());
        assert_eq!(
            m.library_compatibility("windows", Some("Use tool")).status,
            "host_specific"
        );
        m.portability = None;
        assert_eq!(
            m.library_compatibility("windows", Some("Use FFmpeg"))
                .status,
            "needs_review"
        );
    }
    #[test]
    fn incomplete_or_invalid_permission_declarations_fail() {
        let mut value = serde_json::to_value(manifest()).unwrap();
        value["portability"]["permissions"] = json!(["all_access"]);
        assert!(SkillManifest::validate_json(&value).is_err());
        value["portability"]["permissions"] = json!([]);
        value["portability"]["outputs"] = json!([]);
        assert!(SkillManifest::validate_json(&value).is_err());
    }
}
