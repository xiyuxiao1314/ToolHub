use crate::error::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
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
/// Numeric one-to-three-component versions with SemVer prerelease/build identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionConstraint {
    pub op: VersionReqOp,
    pub version: String,
}
#[derive(Debug)]
struct Version {
    numbers: [u64; 3],
    precision: usize,
    pre: Vec<String>,
}
fn parse_version(raw: &str) -> CoreResult<Version> {
    let invalid = || CoreError::InvalidVersion(raw.into());
    let raw = raw.strip_prefix('v').unwrap_or(raw);
    let (base, build) = raw
        .split_once('+')
        .map_or((raw, None), |(a, b)| (a, Some(b)));
    let valid_ids = |s: &str| {
        !s.is_empty()
            && s.split('.')
                .all(|p| !p.is_empty() && p.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-'))
    };
    if build.is_some_and(|b| !valid_ids(b)) {
        return Err(invalid());
    }
    let (numbers, pre) = base
        .split_once('-')
        .map_or((base, None), |(a, b)| (a, Some(b)));
    let parts: Vec<_> = numbers.split('.').collect();
    if parts.is_empty() || parts.len() > 3 {
        return Err(invalid());
    }
    let mut n = [0; 3];
    for (i, p) in parts.iter().enumerate() {
        if p.is_empty()
            || !p.bytes().all(|c| c.is_ascii_digit())
            || (p.len() > 1 && p.starts_with('0'))
        {
            return Err(invalid());
        }
        n[i] = p.parse().map_err(|_| invalid())?;
    }
    let pre = match pre {
        Some(p) => {
            if !valid_ids(p)
                || p.split('.').any(|s| {
                    s.bytes().all(|c| c.is_ascii_digit()) && s.len() > 1 && s.starts_with('0')
                })
            {
                return Err(invalid());
            }
            p.split('.').map(str::to_string).collect()
        }
        None => vec![],
    };
    Ok(Version {
        numbers: n,
        precision: parts.len(),
        pre,
    })
}
fn compare(a: &Version, b: &Version) -> Ordering {
    let numbers = a.numbers.cmp(&b.numbers);
    if numbers != Ordering::Equal {
        return numbers;
    }
    match (a.pre.is_empty(), b.pre.is_empty()) {
        (true, false) => return Ordering::Greater,
        (false, true) => return Ordering::Less,
        _ => {}
    }
    for (x, y) in a.pre.iter().zip(&b.pre) {
        let numeric = |s: &str| s.bytes().all(|c| c.is_ascii_digit());
        let c = match (numeric(x), numeric(y)) {
            (true, true) => x.len().cmp(&y.len()).then_with(|| x.cmp(y)),
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (false, false) => x.cmp(y),
        };
        if c != Ordering::Equal {
            return c;
        }
    }
    a.pre.len().cmp(&b.pre.len())
}
pub fn compare_versions(a: &str, b: &str) -> CoreResult<Ordering> {
    Ok(compare(&parse_version(a)?, &parse_version(b)?))
}
impl VersionConstraint {
    pub fn parse(raw: &str) -> CoreResult<Self> {
        let raw = raw.trim();
        if raw == "*" {
            return Ok(Self {
                op: VersionReqOp::Gte,
                version: "0".into(),
            });
        }
        let (op, ver) = [
            (">=", VersionReqOp::Gte),
            ("<=", VersionReqOp::Lte),
            (">", VersionReqOp::Gt),
            ("<", VersionReqOp::Lt),
            ("=", VersionReqOp::Eq),
            ("^", VersionReqOp::Caret),
            ("~", VersionReqOp::Tilde),
        ]
        .into_iter()
        .find_map(|(prefix, op)| raw.strip_prefix(prefix).map(|v| (op, v.trim())))
        .unwrap_or((VersionReqOp::Eq, raw));
        parse_version(ver)?;
        Ok(Self {
            op,
            version: ver.into(),
        })
    }
    pub fn matches(&self, actual: &str) -> bool {
        let (Ok(a), Ok(b)) = (parse_version(actual), parse_version(&self.version)) else {
            return false;
        };
        if !a.pre.is_empty() && (b.pre.is_empty() || a.numbers != b.numbers) {
            return false;
        }
        let cmp = compare(&a, &b);
        match self.op {
            VersionReqOp::Eq => cmp == Ordering::Equal,
            VersionReqOp::Gte => cmp != Ordering::Less,
            VersionReqOp::Gt => cmp == Ordering::Greater,
            VersionReqOp::Lte => cmp != Ordering::Greater,
            VersionReqOp::Lt => cmp == Ordering::Less,
            VersionReqOp::Tilde => {
                cmp != Ordering::Less
                    && a.numbers[0] == b.numbers[0]
                    && (b.precision == 1 || a.numbers[1] == b.numbers[1])
            }
            VersionReqOp::Caret => {
                let i = if b.numbers[0] > 0 || b.precision == 1 {
                    0
                } else if b.numbers[1] > 0 || b.precision == 2 {
                    1
                } else {
                    2
                };
                cmp != Ordering::Less && a.numbers[..=i] == b.numbers[..=i]
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_non_numeric_versions_and_orders_prereleases() {
        for bad in [">=banana", "1..2", "01.2", "1.2.3-01", "1.2.3+", "1.2.3.4"] {
            assert!(VersionConstraint::parse(bad).is_err(), "accepted {bad}");
        }
        assert!(!VersionConstraint::parse("=1.2.3-alpha")
            .unwrap()
            .matches("1.2.3-beta"));
        assert!(VersionConstraint::parse(">1.2.3-alpha.2")
            .unwrap()
            .matches("1.2.3-alpha.10"));
        assert!(!VersionConstraint::parse("^0.0.2").unwrap().matches("0.0.3"));
    }
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
