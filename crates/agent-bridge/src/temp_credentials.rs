//! B02-12: temporary AI credentials — process memory only, never persisted.

use std::collections::BTreeMap;
use std::sync::Mutex;

use once_cell::sync::Lazy;

static SESSION_KEYS: Lazy<Mutex<BTreeMap<String, String>>> =
    Lazy::new(|| Mutex::new(BTreeMap::new()));

/// Insert a temporary key. Not written to disk/logs/config.
pub fn put_temporary_key(session_id: &str, key: &str) {
    let mut g = SESSION_KEYS.lock().unwrap();
    g.insert(session_id.to_string(), key.to_string());
}

pub fn take_temporary_key(session_id: &str) -> Option<String> {
    let mut g = SESSION_KEYS.lock().unwrap();
    g.remove(session_id)
}

/// Destroy all keys (session end / cancel).
pub fn destroy_all() {
    let mut g = SESSION_KEYS.lock().unwrap();
    g.clear();
}

/// Redacted view for error paths — never returns the secret.
pub fn describe_session(session_id: &str) -> String {
    let g = SESSION_KEYS.lock().unwrap();
    if g.contains_key(session_id) {
        format!("session {session_id}: <redacted>")
    } else {
        format!("session {session_id}: empty")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_not_in_debug_output() {
        put_temporary_key("s1", "sk-SUPER-SECRET");
        let d = describe_session("s1");
        assert!(!d.contains("SUPER-SECRET"));
        assert!(take_temporary_key("s1").is_some());
        assert!(take_temporary_key("s1").is_none());
    }

    #[test]
    fn destroy_clears() {
        put_temporary_key("s2", "sk-x");
        destroy_all();
        assert!(take_temporary_key("s2").is_none());
    }
}
