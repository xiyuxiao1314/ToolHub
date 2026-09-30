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

#[derive(Debug, Clone, serde::Serialize)]
pub struct TempProviderSession {
    pub id: String,
    pub owner: String,
    pub discovery_session_id: String,
    pub endpoint: String,
    pub model: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub state: String,
}
struct Secret(Vec<u8>);
impl Drop for Secret {
    fn drop(&mut self) {
        for byte in &mut self.0 {
            unsafe { std::ptr::write_volatile(byte, 0) }
        }
    }
}
#[derive(Default)]
pub struct TempProviderStore {
    sessions: BTreeMap<String, (TempProviderSession, Secret)>,
}
impl TempProviderStore {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn configure(
        &mut self,
        owner: &str,
        session_id: &str,
        endpoint: &str,
        model: &str,
        key: String,
        ttl_seconds: u64,
    ) -> Result<TempProviderSession, String> {
        if owner.is_empty()
            || session_id.is_empty()
            || model.trim().is_empty()
            || key.is_empty()
            || key.len() > 4096
            || ttl_seconds == 0
            || ttl_seconds > 3600
        {
            return Err("invalid temporary provider session".into());
        }
        if endpoint.contains(['\r', '\n', '@', '?', '#'])
            || !(endpoint.starts_with("https://")
                || endpoint.starts_with("http://127.0.0.1:")
                || endpoint.starts_with("http://localhost:"))
        {
            return Err("provider requires HTTPS or an explicit owned loopback fixture".into());
        }
        self.purge_expired();
        let session = TempProviderSession {
            id: uuid::Uuid::new_v4().to_string(),
            owner: owner.into(),
            discovery_session_id: session_id.into(),
            endpoint: endpoint.into(),
            model: model.into(),
            expires_at: chrono::Utc::now() + chrono::Duration::seconds(ttl_seconds as i64),
            state: "ready".into(),
        };
        self.sessions.insert(
            session.id.clone(),
            (session.clone(), Secret(key.into_bytes())),
        );
        Ok(session)
    }
    fn purge_expired(&mut self) {
        let now = chrono::Utc::now();
        self.sessions.retain(|_, (s, _)| s.expires_at > now);
    }
    pub fn describe(&mut self, owner: &str, id: &str) -> Result<TempProviderSession, String> {
        self.purge_expired();
        let (s, _) = self
            .sessions
            .get(id)
            .ok_or("temporary provider unavailable or expired")?;
        if s.owner != owner {
            return Err("temporary provider owner mismatch".into());
        }
        Ok(s.clone())
    }
    pub fn finish(&mut self, owner: &str, id: &str) -> Result<(), String> {
        self.describe(owner, id)?;
        self.sessions.remove(id);
        Ok(())
    }
    pub fn cancel(&mut self, owner: &str, id: &str) -> Result<(), String> {
        self.finish(owner, id)
    }
    pub fn revoke_discovery(&mut self, owner: &str, discovery_session_id: &str) -> usize {
        let before = self.sessions.len();
        self.sessions
            .retain(|_, (s, _)| s.owner != owner || s.discovery_session_id != discovery_session_id);
        before - self.sessions.len()
    }
    pub fn with_key<T>(
        &mut self,
        owner: &str,
        id: &str,
        operation: impl FnOnce(&str, &TempProviderSession) -> Result<T, String>,
    ) -> Result<T, String> {
        self.describe(owner, id)?;
        let (session, secret) = self
            .sessions
            .remove(id)
            .ok_or("temporary provider unavailable")?;
        let key = std::str::from_utf8(&secret.0).map_err(|_| "invalid credential encoding")?;
        operation(key, &session).map_err(|error| error.replace(key, "<redacted>"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    static LEGACY_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn key_not_in_debug_output() {
        let _guard = LEGACY_TEST_LOCK.lock().unwrap();
        put_temporary_key("s1", "sk-SUPER-SECRET");
        let d = describe_session("s1");
        assert!(!d.contains("SUPER-SECRET"));
        assert!(take_temporary_key("s1").is_some());
        assert!(take_temporary_key("s1").is_none());
    }

    #[test]
    fn destroy_clears() {
        let _guard = LEGACY_TEST_LOCK.lock().unwrap();
        put_temporary_key("s2", "sk-x");
        destroy_all();
        assert!(take_temporary_key("s2").is_none());
    }

    #[test]
    fn temporary_provider_destroys_key_on_success_error_and_cancel() {
        let mut store = TempProviderStore::new();
        for outcome in ["success", "error", "cancel"] {
            let s = store
                .configure(
                    "owner",
                    "discovery",
                    "http://127.0.0.1:1/v1",
                    "fixture",
                    "SYNTHETIC-CREDENTIAL".into(),
                    30,
                )
                .unwrap();
            assert!(!serde_json::to_string(&s).unwrap().contains("SYNTHETIC"));
            assert!(store.describe("other", &s.id).is_err());
            match outcome {
                "success" => assert_eq!(
                    store
                        .with_key("owner", &s.id, |key, _| Ok(key.len()))
                        .unwrap(),
                    20
                ),
                "error" => assert_eq!(
                    store
                        .with_key::<()>("owner", &s.id, |key, _| Err(format!(
                            "provider rejected {key}"
                        )))
                        .unwrap_err(),
                    "provider rejected <redacted>"
                ),
                _ => store.cancel("owner", &s.id).unwrap(),
            }
            assert!(store.describe("owner", &s.id).is_err());
        }
    }
}
