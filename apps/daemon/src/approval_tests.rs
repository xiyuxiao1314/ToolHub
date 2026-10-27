use super::*;

fn call(svc: &mut DaemonService, method: &str, params: Value, controller: bool) -> JsonRpcResponse {
    svc.handle_authenticated(
        &JsonRpcRequest::new(1, method, params),
        "fixture.agent",
        controller,
    )
}

fn set_auto(svc: &mut DaemonService, enabled: bool) {
    let result = call(
        svc,
        "settings.set",
        json!({"settings":{"auto_approve_execution":enabled}}),
        true,
    );
    assert!(result.error.is_none(), "{:?}", result.error);
    assert_eq!(result.result.unwrap()["auto_approve_execution"], enabled);
}

#[test]
fn default_legacy_opt_out_and_controller_setting_are_persistent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("private.sqlite");
    let mut svc = DaemonService::open(&path).unwrap();
    assert_eq!(
        call(&mut svc, "settings.get", json!({}), false)
            .result
            .unwrap()["auto_approve_execution"],
        true
    );
    svc.registry
        .set_setting("user_preferences", &json!({"scan_mode":"quick"}))
        .unwrap();
    assert_eq!(
        call(&mut svc, "settings.get", json!({}), false)
            .result
            .unwrap()["auto_approve_execution"],
        true
    );
    assert!(call(
        &mut svc,
        "settings.set",
        json!({"auto_approve_execution":false}),
        false
    )
    .error
    .is_some());
    set_auto(&mut svc, false);
    assert!(call(
        &mut svc,
        "settings.set",
        json!({"auto_approve_execution":"true"}),
        true
    )
    .error
    .is_some());
    assert!(
        call(&mut svc, "settings.set", json!({"scan_mode":"full"}), true)
            .error
            .is_none()
    );
    drop(svc);
    let mut svc = DaemonService::open(&path).unwrap();
    assert_eq!(
        call(&mut svc, "settings.get", json!({}), false)
            .result
            .unwrap()["auto_approve_execution"],
        false
    );
}

#[test]
fn failed_setting_write_preserves_approval_mode_and_revision() {
    let dir = tempfile::tempdir().unwrap();
    let mut svc = DaemonService::open(&dir.path().join("private.sqlite")).unwrap();
    set_auto(&mut svc, true);
    let revision = svc.registry.revision().unwrap();
    svc.registry.db.conn.execute_batch("CREATE TRIGGER reject_setting BEFORE INSERT ON settings BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap();
    assert!(call(
        &mut svc,
        "settings.set",
        json!({"auto_approve_execution":false}),
        true
    )
    .error
    .is_some());
    assert_eq!(svc.registry.revision().unwrap(), revision);
    assert_eq!(
        call(&mut svc, "settings.get", json!({}), false)
            .result
            .unwrap()["auto_approve_execution"],
        true
    );
}

#[cfg(any(windows, target_os = "linux"))]
mod supported_platform {
    use super::*;

    fn fixture() -> (tempfile::TempDir, DaemonService, Value) {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join(if cfg!(windows) {
            "approval-fixture.exe"
        } else {
            "approval-fixture"
        });
        // A copy of this owned test binary; --list is finite and runs no tests/user tools.
        std::fs::copy(std::env::current_exe().unwrap(), &exe).unwrap();
        let mut svc = DaemonService::open(&dir.path().join("private.sqlite")).unwrap();
        svc.registry
            .upsert_instance(&UpsertInstanceInput {
                id: "fixture.tool".into(),
                definition_id: "fixture.definition".into(),
                definition_name: "Owned test fixture".into(),
                version: None,
                platform: std::env::consts::OS.into(),
                arch: std::env::consts::ARCH.into(),
                path: exe.to_string_lossy().into_owned(),
                canonical_path: None,
                environment_id: None,
                origin_json: serde_json::to_string(&toolhub_core::Origin::Unknown).unwrap(),
                owner_json: serde_json::to_string(&toolhub_core::Owner::unknown()).unwrap(),
                trust_json: json!({"level":"user_trusted"}).to_string(),
                status: "available".into(),
                capabilities: vec![],
            })
            .unwrap();
        let params = json!({"instance_id":"fixture.tool","args":["--list"],"cwd":dir.path(),"timeout_ms":10000});
        (dir, svc, params)
    }

    fn prepare(
        svc: &mut DaemonService,
        params: Value,
    ) -> Result<execution_jobs::PreparedExecution, Box<JsonRpcResponse>> {
        svc.prepare_execution(
            &JsonRpcRequest::new(1, "execute.tool", params),
            "fixture.agent",
            false,
        )
        .map_err(Box::new)
    }

    #[test]
    fn direct_execution_auto_approves_without_desktop_and_records_success() {
        let (_dir, mut svc, params) = fixture();
        let job = prepare(&mut svc, params).unwrap_or_else(|e| panic!("{:?}", e.error));
        let result = job.run();
        let finished = svc.finish_execution(job, result);
        assert!(finished.error.is_none(), "{:?}", finished.error);
        assert_eq!(finished.result.unwrap()["status"], "success");
        let count:i64=svc.registry.db.conn.query_row("SELECT count(*) FROM execution_requests WHERE status='approved' AND json_extract(params_json,'$.auto_approved')=1",[],|r|r.get(0)).unwrap();
        assert_eq!(count, 1);
        let consumed: i64 = svc
            .registry
            .db
            .conn
            .query_row("SELECT consumed FROM execution_approvals", [], |r| r.get(0))
            .unwrap();
        assert_eq!(consumed, 1);
    }

    #[test]
    fn immediate_approval_is_bound_owned_one_use_and_legacy_desktop_compatible() {
        let (_dir, mut svc, mut params) = fixture();
        let issued = call(&mut svc, "execute.approval_request", params.clone(), false)
            .result
            .unwrap();
        assert_eq!(issued["status"], "approved");
        assert_eq!(issued["auto_approved"], true);
        let request_id = issued["request_id"].clone();
        let polled = call(
            &mut svc,
            "execute.approval_status",
            json!({"request_id":request_id}),
            false,
        )
        .result
        .unwrap();
        assert_eq!(polled["approval_id"], issued["approval_id"]);
        assert!(svc
            .handle_authenticated(
                &JsonRpcRequest::new(
                    2,
                    "execute.approval_status",
                    json!({"request_id":request_id})
                ),
                "fixture.other",
                false
            )
            .error
            .is_some());
        assert!(call(
            &mut svc,
            "execute.approve",
            json!({"request_id":request_id}),
            false
        )
        .error
        .is_some());
        let desktop = call(
            &mut svc,
            "execute.approve",
            json!({"request_id":request_id}),
            true,
        )
        .result
        .unwrap();
        assert_eq!(desktop["approval_id"], issued["approval_id"]);
        params["approval_id"] = issued["approval_id"].clone();
        params["session_id"] = issued["session_id"].clone();
        let mut changed = params.clone();
        changed["args"] = json!(["--help"]);
        assert!(prepare(&mut svc, changed).is_err());
        let mut wrong_session = params.clone();
        wrong_session["session_id"] = json!("wrong");
        assert!(prepare(&mut svc, wrong_session).is_err());
        let job = prepare(&mut svc, params.clone()).unwrap_or_else(|e| panic!("{:?}", e.error));
        let result = job.run();
        assert_eq!(
            svc.finish_execution(job, result).result.unwrap()["status"],
            "success"
        );
        assert!(prepare(&mut svc, params).is_err());
    }

    #[test]
    fn disabled_mode_waits_for_controller_and_manual_approval_still_executes() {
        let (_dir, mut svc, mut params) = fixture();
        set_auto(&mut svc, false);
        assert!(prepare(&mut svc, params.clone()).is_err());
        let requested = call(&mut svc, "execute.approval_request", params.clone(), false)
            .result
            .unwrap();
        assert_eq!(requested["status"], "pending");
        assert!(requested["approval_id"].is_null());
        assert_eq!(
            call(
                &mut svc,
                "execute.approval_status",
                json!({"request_id":requested["request_id"]}),
                false
            )
            .result
            .unwrap()["status"],
            "pending"
        );
        let approved = call(
            &mut svc,
            "execute.approve",
            json!({"request_id":requested["request_id"]}),
            true,
        )
        .result
        .unwrap();
        assert_eq!(approved["auto_approved"], false);
        params["approval_id"] = approved["approval_id"].clone();
        params["session_id"] = approved["session_id"].clone();
        let job = prepare(&mut svc, params).unwrap_or_else(|e| panic!("{:?}", e.error));
        let result = job.run();
        assert_eq!(
            svc.finish_execution(job, result).result.unwrap()["status"],
            "success"
        );
    }

    #[test]
    fn switching_mode_invalidates_issued_grants_even_after_reenable() {
        let (_dir, mut svc, mut params) = fixture();
        let issued = call(&mut svc, "execute.approval_request", params.clone(), false)
            .result
            .unwrap();
        params["approval_id"] = issued["approval_id"].clone();
        params["session_id"] = issued["session_id"].clone();
        set_auto(&mut svc, false);
        assert!(prepare(&mut svc, params.clone()).is_err());
        set_auto(&mut svc, true);
        assert!(prepare(&mut svc, params).is_err());
    }

    #[test]
    fn trust_denials_and_explicit_policy_denials_remain_effective() {
        let (_dir, mut svc, params) = fixture();
        for trust in ["unknown", "blocked"] {
            call(
                &mut svc,
                "registry.correct",
                json!({"id":"fixture.tool","trust":trust}),
                true,
            );
            assert!(
                call(&mut svc, "execute.approval_request", params.clone(), false)
                    .error
                    .is_some()
            );
            assert!(prepare(&mut svc, params.clone()).is_err());
        }
        call(
            &mut svc,
            "registry.correct",
            json!({"id":"fixture.tool","trust":"user_trusted"}),
            true,
        );
        assert!(call(
            &mut svc,
            "policy.set",
            json!({"scope":"tool","subject":"approval-fixture","action":"deny"}),
            true
        )
        .error
        .is_none());
        assert!(
            call(&mut svc, "execute.approval_request", params.clone(), false)
                .error
                .is_some()
        );
        assert!(prepare(&mut svc, params).is_err());
        assert_eq!(
            svc.registry
                .db
                .conn
                .query_row("SELECT count(*) FROM execution_approvals", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }

    #[test]
    fn automatic_approval_survives_restart_until_consumed() {
        let (dir, mut svc, mut params) = fixture();
        let issued = call(&mut svc, "execute.approval_request", params.clone(), false)
            .result
            .unwrap();
        drop(svc);
        let mut svc = DaemonService::open(&dir.path().join("private.sqlite")).unwrap();
        let status = call(
            &mut svc,
            "execute.approval_status",
            json!({"request_id":issued["request_id"]}),
            false,
        )
        .result
        .unwrap();
        assert_eq!(status["status"], "approved");
        assert_eq!(status["auto_approved"], true);
        params["approval_id"] = status["approval_id"].clone();
        params["session_id"] = status["session_id"].clone();
        let job = prepare(&mut svc, params).unwrap_or_else(|e| panic!("{:?}", e.error));
        let result = job.run();
        assert_eq!(
            svc.finish_execution(job, result).result.unwrap()["status"],
            "success"
        );
    }

    #[test]
    fn enabling_auto_mode_expires_pending_request_and_resubmission_is_immediate() {
        let (_dir, mut svc, params) = fixture();
        set_auto(&mut svc, false);
        let pending = call(&mut svc, "execute.approval_request", params.clone(), false)
            .result
            .unwrap();
        set_auto(&mut svc, true);
        let expired = call(
            &mut svc,
            "execute.approval_status",
            json!({"request_id":pending["request_id"]}),
            false,
        )
        .result
        .unwrap();
        assert_eq!(expired["status"], "expired");
        assert!(expired["approval_id"].is_null());
        assert_eq!(
            call(&mut svc, "execute.approval_request", params, false)
                .result
                .unwrap()["status"],
            "approved"
        );
    }

    #[test]
    fn supplied_approval_is_validated_even_under_an_allow_rule() {
        let (_dir, mut svc, mut params) = fixture();
        assert!(call(
            &mut svc,
            "policy.set",
            json!({"scope":"tool","subject":"approval-fixture","action":"allow"}),
            true
        )
        .error
        .is_none());
        params["approval_id"] = json!("fake");
        params["session_id"] = json!("fake");
        assert!(prepare(&mut svc, params.clone()).is_err());
        params.as_object_mut().unwrap().remove("approval_id");
        params.as_object_mut().unwrap().remove("session_id");
        let issued = call(&mut svc, "execute.approval_request", params.clone(), false)
            .result
            .unwrap();
        params["approval_id"] = issued["approval_id"].clone();
        params["session_id"] = issued["session_id"].clone();
        assert!(svc
            .prepare_execution(
                &JsonRpcRequest::new(1, "execute.tool", params.clone()),
                "fixture.other",
                false
            )
            .is_err());
        let job = prepare(&mut svc, params.clone()).unwrap_or_else(|e| panic!("{:?}", e.error));
        let result = job.run();
        assert_eq!(
            svc.finish_execution(job, result).result.unwrap()["status"],
            "success"
        );
        assert!(prepare(&mut svc, params).is_err());
    }
    #[test]
    fn signature_bound_record_rejects_changed_hash_before_automatic_approval() {
        let (_dir,mut svc,params)=fixture();
        let row=svc.registry.get_instance("fixture.tool").unwrap().unwrap();
        let canonical=std::fs::canonicalize(&row.path).unwrap();
        let mut evidence=toolhub_core::Evidence::native(toolhub_core::EvidenceSource::CodeSignature,1.0,"synthetic stale signature record").unwrap();
        evidence.detail=Some(json!({"canonical_path":canonical,"sha256":"stale-hash"}).to_string());
        let mut record=toolhub_core::TrustRecord::known(vec![evidence]);record.level=toolhub_core::TrustLevel::Verified;record.reason=Some(toolhub_recognizer::native_signature::REASON.into());
        svc.registry.db.conn.execute("UPDATE tool_instances SET trust_json=?1 WHERE id='fixture.tool'",[serde_json::to_string(&record).unwrap()]).unwrap();
        let denied=call(&mut svc,"execute.approval_request",params,false);assert!(denied.error.unwrap().message.contains("signature-verified executable changed"));
        let count:i64=svc.registry.db.conn.query_row("SELECT count(*) FROM execution_approvals",[],|r|r.get(0)).unwrap();assert_eq!(count,0);
    }
}
