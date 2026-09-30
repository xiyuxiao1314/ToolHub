use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VersionReqOp {
    Eq,
    Gte,
    Gt,
    Lte,
    Lt,
    Tilde,
    Caret,
}

/// Simple version constraint: `>=17`, `=3.12`, `^1.2`, `~1.2.3`, `*`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionConstraint {
    pub op: VersionReqOp,
    pub version: String,
}

impl VersionConstraint {
    pub fn parse(raw: &str) -> CoreResult<Self> {
        let raw = raw.trim();
        if raw == "*" || raw.is_empty() {
            return Ok(Self {
                op: VersionReqOp::Gte,
                version: "0".into(),
            });
        }
        let (op, ver) = if let Some(v) = raw.strip_prefix(">=") {
            (VersionReqOp::Gte, v.trim())
        } else if let Some(v) = raw.strip_prefix("<=") {
            (VersionReqOp::Lte, v.trim())
        } else if let Some(v) = raw.strip_prefix('>') {
            (VersionReqOp::Gt, v.trim())
        } else if let Some(v) = raw.strip_prefix('<') {
            (VersionReqOp::Lt, v.trim())
        } else if let Some(v) = raw.strip_prefix('=') {
            (VersionReqOp::Eq, v.trim())
        } else if let Some(v) = raw.strip_prefix('^') {
            (VersionReqOp::Caret, v.trim())
        } else if let Some(v) = raw.strip_prefix('~') {
            (VersionReqOp::Tilde, v.trim())
        } else {
            (VersionReqOp::Eq, raw)
        };
        if ver.is_empty() {
            return Err(CoreError::InvalidVersion(raw.to_string()));
        }
        // F10: reject invalid grammar (||, spaces, empty) instead of silent discard
        if ver.is_empty()
            || ver.contains("||")
            || ver.contains(' ')
            || !ver
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '+')
        {
            return Err(CoreError::InvalidVersion(raw.to_string()));
        }
        Ok(Self {
            op,
            version: ver.to_string(),
        })
    }

    pub fn matches(&self, actual: &str) -> bool {
        // Prerelease does not satisfy a stable constraint unless both are prerelease.
        let a_prerelease = is_prerelease(actual);
        let b_prerelease = is_prerelease(&self.version);
        if a_prerelease && !b_prerelease && self.op != VersionReqOp::Eq {
            return false;
        }
        let a = normalize_ver(actual);
        let b = normalize_ver(&self.version);
        let cmp = compare_ver(&a, &b);
        match self.op {
            VersionReqOp::Eq => cmp == std::cmp::Ordering::Equal && a_prerelease == b_prerelease,
            VersionReqOp::Gte => cmp != std::cmp::Ordering::Less,
            VersionReqOp::Gt => cmp == std::cmp::Ordering::Greater,
            VersionReqOp::Lte => cmp != std::cmp::Ordering::Greater,
            VersionReqOp::Lt => cmp == std::cmp::Ordering::Less,
            VersionReqOp::Tilde => cmp != std::cmp::Ordering::Less && major_minor_eq(&a, &b),
            VersionReqOp::Caret => {
                if cmp == std::cmp::Ordering::Less {
                    return false;
                }
                // Zero-major caret: ^0.2.0 allows 0.2.x only
                let av = split_ver(&a);
                let bv = split_ver(&b);
                if bv.first() == Some(&0) {
                    av.first() == Some(&0) && av.get(1) == bv.get(1)
                } else {
                    major_eq(&a, &b)
                }
            }
        }
    }
}

fn is_prerelease(v: &str) -> bool {
    v.contains('-') || v.contains("rc") || v.contains("alpha") || v.contains("beta")
}

fn normalize_ver(v: &str) -> String {
    v.trim().trim_start_matches('v').to_string()
}

fn split_ver(v: &str) -> Vec<u64> {
    v.split(['.', '-'])
        .filter_map(|p| p.parse::<u64>().ok())
        .collect()
}

fn compare_ver(a: &str, b: &str) -> std::cmp::Ordering {
    let av = split_ver(a);
    let bv = split_ver(b);
    for i in 0..av.len().max(bv.len()) {
        let x = av.get(i).copied().unwrap_or(0);
        let y = bv.get(i).copied().unwrap_or(0);
        match x.cmp(&y) {
            std::cmp::Ordering::Equal => {}
            other => return other,
        }
    }
    std::cmp::Ordering::Equal
}

fn major_eq(a: &str, b: &str) -> bool {
    split_ver(a).first().copied().unwrap_or(0) == split_ver(b).first().copied().unwrap_or(0)
}

fn major_minor_eq(a: &str, b: &str) -> bool {
    let av = split_ver(a);
    let bv = split_ver(b);
    av.first().copied() == bv.first().copied() && av.get(1).copied() == bv.get(1).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_constraints() {
        let c = VersionConstraint::parse(">=17").unwrap();
        assert!(c.matches("17.0.1"));
        assert!(c.matches("21"));
        assert!(!c.matches("16.0.2"));
        let t = VersionConstraint::parse("~1.2.3").unwrap();
        assert!(t.matches("1.2.9"));
        assert!(!t.matches("1.3.0"));
    }
}
