//! Allow / Ask / Deny policy with effective-rule precedence.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyAction {
    Allow,
    Ask,
    Deny,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyScope {
    Command,
    Tool,
    Capability,
    Agent,
    Directory,
    Environment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub scope: PolicyScope,
    pub subject: String,
    pub action: PolicyAction,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PolicyEngine {
    pub rules: Vec<PolicyRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub action: PolicyAction,
    pub matched: Vec<String>,
    pub explanation: String,
}

impl PolicyEngine {
    pub fn with_defaults() -> Self {
        Self {
            rules: vec![
                // Known safe probes / version commands are allowed by default for known tools.
                PolicyRule {
                    scope: PolicyScope::Command,
                    subject: "version-probe".into(),
                    action: PolicyAction::Allow,
                },
                // Unknown subjects default to Ask rather than silent Allow.
                PolicyRule {
                    scope: PolicyScope::Tool,
                    subject: "*".into(),
                    action: PolicyAction::Ask,
                },
                PolicyRule {
                    scope: PolicyScope::Command,
                    subject: "rm".into(),
                    action: PolicyAction::Deny,
                },
                PolicyRule {
                    scope: PolicyScope::Command,
                    subject: "del".into(),
                    action: PolicyAction::Deny,
                },
            ],
        }
    }

    pub fn set(&mut self, rule: PolicyRule) {
        self.rules
            .retain(|r| !(r.scope == rule.scope && r.subject == rule.subject));
        self.rules.push(rule);
    }

    /// Precedence: Command > Tool > Capability > Agent > Directory > Environment.
    /// Within the same scope, exact subject beats wildcard. Deny beats Allow.
    pub fn decide(&self, ctx: &PolicyContext) -> PolicyDecision {
        let mut matched = vec![];
        let mut best: Option<(usize, PolicyAction)> = None;
        for rule in &self.rules {
            let specificity = match rule.scope {
                PolicyScope::Command if ctx.command.as_deref() == Some(rule.subject.as_str()) => {
                    Some(100)
                }
                PolicyScope::Tool if ctx.tool.as_deref() == Some(rule.subject.as_str()) => Some(90),
                PolicyScope::Capability
                    if ctx.capability.as_deref() == Some(rule.subject.as_str()) =>
                {
                    Some(80)
                }
                PolicyScope::Agent if ctx.agent.as_deref() == Some(rule.subject.as_str()) => {
                    Some(70)
                }
                PolicyScope::Directory
                    if ctx.directory.as_deref() == Some(rule.subject.as_str()) =>
                {
                    Some(60)
                }
                PolicyScope::Environment
                    if ctx.environment.as_deref() == Some(rule.subject.as_str()) =>
                {
                    Some(50)
                }
                PolicyScope::Tool if rule.subject == "*" && ctx.tool.is_some() => Some(10),
                _ => None,
            };
            if let Some(spec) = specificity {
                matched.push(format!("{:?}:{}", rule.scope, rule.subject));
                let better = match best {
                    None => true,
                    Some((s, a)) => {
                        spec > s
                            || (spec == s
                                && matches!(
                                    (rule.action, a),
                                    (PolicyAction::Deny, _)
                                        | (PolicyAction::Ask, PolicyAction::Allow)
                                ))
                    }
                };
                if better {
                    best = Some((spec, rule.action));
                }
            }
        }
        let action = best.map(|(_, a)| a).unwrap_or(PolicyAction::Ask);
        PolicyDecision {
            action,
            matched,
            explanation: match action {
                PolicyAction::Allow => "explicit allow".into(),
                PolicyAction::Ask => "requires interactive approval".into(),
                PolicyAction::Deny => "denied by policy".into(),
            },
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PolicyContext {
    pub tool: Option<String>,
    pub capability: Option<String>,
    pub agent: Option<String>,
    pub directory: Option<String>,
    pub environment: Option<String>,
    pub command: Option<String>,
    pub trust: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deny_beats_allow_on_same_specificity() {
        let mut e = PolicyEngine::default();
        e.set(PolicyRule {
            scope: PolicyScope::Tool,
            subject: "python".into(),
            action: PolicyAction::Allow,
        });
        e.set(PolicyRule {
            scope: PolicyScope::Command,
            subject: "rm".into(),
            action: PolicyAction::Deny,
        });
        let d = e.decide(&PolicyContext {
            tool: Some("python".into()),
            command: Some("rm".into()),
            ..Default::default()
        });
        assert_eq!(d.action, PolicyAction::Deny);
    }

    #[test]
    fn unknown_defaults_to_ask() {
        let e = PolicyEngine::with_defaults();
        let d = e.decide(&PolicyContext {
            tool: Some("mystery".into()),
            ..Default::default()
        });
        assert_eq!(d.action, PolicyAction::Ask);
    }
}
