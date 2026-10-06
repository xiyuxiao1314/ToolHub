use serde::{Deserialize, Serialize};

pub use crate::id::EnvironmentId;
use crate::owner::{Origin, Owner};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentKind {
    System,
    User,
    Homebrew,
    Winget,
    Scoop,
    Chocolatey,
    Conda,
    Venv,
    Uv,
    Pyenv,
    Nvm,
    Pnpm,
    Npm,
    Cargo,
    Docker,
    Wsl,
    AgentSandbox,
    ApplicationBundle,
    ProjectLocal,
    Unknown,
}

impl EnvironmentKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            EnvironmentKind::System => "system",
            EnvironmentKind::User => "user",
            EnvironmentKind::Homebrew => "homebrew",
            EnvironmentKind::Winget => "winget",
            EnvironmentKind::Scoop => "scoop",
            EnvironmentKind::Chocolatey => "chocolatey",
            EnvironmentKind::Conda => "conda",
            EnvironmentKind::Venv => "venv",
            EnvironmentKind::Uv => "uv",
            EnvironmentKind::Pyenv => "pyenv",
            EnvironmentKind::Nvm => "nvm",
            EnvironmentKind::Pnpm => "pnpm",
            EnvironmentKind::Npm => "npm",
            EnvironmentKind::Cargo => "cargo",
            EnvironmentKind::Docker => "docker",
            EnvironmentKind::Wsl => "wsl",
            EnvironmentKind::AgentSandbox => "agent_sandbox",
            EnvironmentKind::ApplicationBundle => "application_bundle",
            EnvironmentKind::ProjectLocal => "project_local",
            EnvironmentKind::Unknown => "unknown",
        }
    }
}

/// Installation/execution context. Graph edges are parent/child environments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Environment {
    pub id: EnvironmentId,
    pub name: String,
    pub kind: EnvironmentKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<EnvironmentId>,
    #[serde(default)]
    pub origin: Origin,
    #[serde(default)]
    pub owner: Owner,
    #[serde(default)]
    pub labels: Vec<String>,
}

impl Environment {
    pub fn system() -> Self {
        Self {
            id: EnvironmentId::new("env.system").unwrap(),
            name: "System".into(),
            kind: EnvironmentKind::System,
            root_path: None,
            parent_id: None,
            origin: Origin::System,
            owner: Owner::user(),
            labels: vec!["builtin".into()],
        }
    }

    pub fn user() -> Self {
        Self {
            id: EnvironmentId::new("env.user").unwrap(),
            name: "User".into(),
            kind: EnvironmentKind::User,
            root_path: None,
            parent_id: None,
            origin: Origin::UserInstall,
            owner: Owner::user(),
            labels: vec!["builtin".into()],
        }
    }
}
