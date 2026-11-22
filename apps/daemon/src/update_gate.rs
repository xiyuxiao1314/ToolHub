//! A short controller-only maintenance lease. Existing registered services remain alive.
use super::*;
use std::time::{Duration, Instant};
pub(super) struct Lease {
    token: String,
    owner: String,
    expires: Instant,
}
impl DaemonService {
    pub fn update_probation(&mut self, token: &str) -> Result<(), String> {
        uuid::Uuid::parse_str(token).map_err(|_| "invalid probation nonce")?;
        self.update_boot_nonce = Some(token.to_string());
        self.update_lease = Some(Lease {
            token: token.to_string(),
            owner: String::new(),
            expires: Instant::now() + Duration::from_secs(120),
        });
        Ok(())
    }
    pub fn update_busy(&self) -> bool {
        self.update_lease
            .as_ref()
            .is_some_and(|l| l.expires > Instant::now())
    }
    pub fn update_access(&mut self, method: &str) -> Result<(), ProtocolError> {
        if !self.update_busy() {
            self.update_lease = None;
            return Ok(());
        }
        if matches!(
            method,
            "ping" | "status" | "update.cancel" | "update.commit" | "update.finish"
        ) {
            return Ok(());
        }
        Err(ProtocolError::new(
            ErrorCode::Denied,
            "ToolHub 正在准备更新，请稍后重试",
        ))
    }
    pub(super) fn update_action(
        &mut self,
        method: &str,
        params: &Value,
    ) -> Result<Value, ProtocolError> {
        self.require_controller()?;
        if serde_json::to_vec(params).map_or(true, |b| b.len() > 1024) {
            return Err(ProtocolError::new(
                ErrorCode::InvalidParams,
                "update parameters too large",
            ));
        }
        if method == "update.finish" {
            if self.update_boot_nonce.as_deref() != params["token"].as_str()
                || self.update_boot_nonce.is_none()
            {
                return Err(ProtocolError::new(
                    ErrorCode::Denied,
                    "invalid update bootstrap",
                ));
            }
            self.update_lease = None;
            self.update_boot_nonce = None;
            return Ok(json!({"ready":true}));
        }
        if method == "update.prepare" {
            if !params.is_null() && !params.as_object().is_some_and(|m| m.is_empty()) {
                return Err(ProtocolError::new(
                    ErrorCode::InvalidParams,
                    "prepare takes no parameters",
                ));
            }
            if self.update_busy() {
                return Err(ProtocolError::new(ErrorCode::Denied, "已有更新正在准备"));
            }
            if !self.running_jobs.is_empty()
                || !self.running_scans.is_empty()
                || self.agents.has_running()
            {
                return Err(ProtocolError::new(
                    ErrorCode::Denied,
                    "仍有执行任务、扫描或智能体在运行，请完成后再更新",
                ));
            }
            self.shutdown_for_update = false;
            let token = uuid::Uuid::new_v4().to_string();
            self.update_lease = Some(Lease {
                token: token.clone(),
                owner: self.peer_principal(),
                expires: Instant::now() + Duration::from_secs(120),
            });
            return Ok(json!({"token":token,"daemon_pid":std::process::id(),"lease_seconds":120}));
        }
        if !matches!(method, "update.cancel" | "update.commit") {
            return Err(ProtocolError::new(
                ErrorCode::MethodNotFound,
                "unknown update method",
            ));
        }
        let lease = self
            .update_lease
            .as_ref()
            .filter(|l| {
                l.expires > Instant::now()
                    && l.owner == self.peer_principal()
                    && params["token"].as_str() == Some(l.token.as_str())
            })
            .ok_or_else(|| ProtocolError::new(ErrorCode::Denied, "更新会话已失效，请重新开始"))?;
        let _ = lease;
        if params.as_object().is_none_or(|p| p.len() != 1) {
            return Err(ProtocolError::new(ErrorCode::InvalidParams, "token only"));
        }
        if method == "update.cancel" {
            self.shutdown_for_update = false;
            self.update_lease = None;
            return Ok(json!({"cancelled":true}));
        }
        if !self.running_jobs.is_empty()
            || !self.running_scans.is_empty()
            || self.agents.has_running()
        {
            return Err(ProtocolError::new(ErrorCode::Denied, "仍有任务在运行"));
        }
        self.shutdown_for_update = true;
        Ok(json!({"ready":true}))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn request(method: &str, params: Value) -> JsonRpcRequest {
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: method.into(),
            params,
        }
    }
    #[test]
    fn probation_cannot_be_released_by_ordinary_peers_or_stale_tokens() {
        let d = tempfile::tempdir().unwrap();
        let mut service = DaemonService::open(&d.path().join("db.sqlite")).unwrap();
        let token = uuid::Uuid::new_v4().to_string();
        service.update_probation(&token).unwrap();
        assert!(service.update_access("execute.tool").is_err());
        assert!(service
            .handle_authenticated(
                &request("update.finish", json!({"token":token})),
                "ordinary",
                false
            )
            .error
            .is_some());
        assert!(service
            .handle_authenticated(
                &request("update.finish", json!({"token":"wrong"})),
                "desktop",
                true
            )
            .error
            .is_some());
        assert!(service
            .handle_authenticated(
                &request("update.finish", json!({"token":token})),
                "desktop",
                true
            )
            .error
            .is_none());
        assert!(service.update_access("execute.tool").is_ok());
        assert!(service
            .handle_authenticated(
                &request("update.finish", json!({"token":token})),
                "desktop",
                true
            )
            .error
            .is_some());
    }
    #[test]
    fn only_real_controller_can_quiesce_commit_or_abort_and_task_start_is_gated() {
        let d = tempfile::tempdir().unwrap();
        let mut service = DaemonService::open(&d.path().join("db.sqlite")).unwrap();
        let prepare = request("update.prepare", json!({}));
        assert!(service
            .handle_authenticated(&prepare, "ordinary", false)
            .error
            .is_some());
        service.running_jobs.insert(
            "owned".into(),
            ("ordinary".into(), Arc::new(AtomicBool::new(false))),
        );
        assert!(service
            .handle_authenticated(&prepare, "desktop", true)
            .error
            .is_some());
        service.running_jobs.clear();
        let out = service.handle_authenticated(&prepare, "desktop", true);
        let token = out.result.unwrap()["token"].clone();
        assert!(service.update_busy());
        assert!(service.update_access("execute.tool").is_err());
        assert!(service.update_access("program.launch").is_err());
        assert!(service.update_access("scan.start").is_err());
        assert!(service
            .handle_authenticated(
                &request("update.commit", json!({"token":token})),
                "ordinary",
                false
            )
            .error
            .is_some());
        assert!(service
            .handle_authenticated(
                &request("update.commit", json!({"token":"wrong"})),
                "desktop",
                true
            )
            .error
            .is_some());
        assert!(service
            .handle_authenticated(
                &request("update.cancel", json!({"token":token})),
                "desktop",
                true
            )
            .error
            .is_none());
        assert!(!service.update_busy());
        let out = service.handle_authenticated(&prepare, "desktop", true);
        let token = out.result.unwrap()["token"].clone();
        assert!(service
            .handle_authenticated(
                &request("update.commit", json!({"token":token})),
                "desktop",
                true
            )
            .error
            .is_none());
        assert!(service.shutdown_for_update);
    }
}
