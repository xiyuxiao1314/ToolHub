//! Declarative application metadata; sharing never grants execution authority.
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProgramMetadata {
    pub purpose: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub dependencies: Vec<String>,
    pub examples: Vec<String>,
    pub agent_visible: bool,
}
impl ProgramMetadata {
    pub fn validate(&self) -> Result<(), String> {
        if self.purpose.len() > 4096 || self.purpose.contains('\0') {
            return Err("purpose exceeds bounds".into());
        }
        for items in [
            &self.inputs,
            &self.outputs,
            &self.dependencies,
            &self.examples,
        ] {
            if items.len() > 32 || items.iter().any(|s| s.len() > 4096 || s.contains('\0')) {
                return Err("metadata list exceeds bounds".into());
            }
        }
        if self
            .dependencies
            .iter()
            .any(|s| !crate::capability::is_canonical_capability(s))
        {
            return Err("dependencies must be dotted capability identifiers".into());
        }
        Ok(())
    }
}
