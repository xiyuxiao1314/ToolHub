use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! id_newtype {
    ($name:ident, $prefix:expr) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(raw: impl Into<String>) -> crate::CoreResult<Self> {
                let raw = raw.into();
                crate::validate_id(&raw, stringify!($name))?;
                Ok(Self(raw))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<$name> for String {
            fn from(v: $name) -> Self {
                v.0
            }
        }
    };
}

id_newtype!(DefinitionId, "definition");
id_newtype!(InstanceId, "instance");
id_newtype!(InterfaceId, "interface");
id_newtype!(EnvironmentId, "environment");
id_newtype!(CapabilityId, "capability");
id_newtype!(CandidateId, "candidate");
id_newtype!(AgentId, "agent");
id_newtype!(SkillId, "skill");

/// Generate a stable-looking instance UUID string.
pub fn new_instance_id() -> InstanceId {
    InstanceId::new(uuid::Uuid::new_v4().to_string()).expect("uuid is valid id")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_id_roundtrip() {
        let id = new_instance_id();
        let s = id.to_string();
        let back = InstanceId::new(s).unwrap();
        assert_eq!(id, back);
    }
}
