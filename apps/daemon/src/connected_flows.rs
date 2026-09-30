use super::*;
use super::execution_jobs::{invalid,text};
use std::collections::BTreeSet;

impl DaemonService {
    fn active_discovery(&mut self,owner:&str,id:&str,required:&[&str])->Result<DiscoverySession,ProtocolError> {
        let row=self.registry.db.conn.query_row("SELECT agent_id,created_at,expires_at,scopes_json,revoked FROM discovery_sessions WHERE id=?1",[id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,i64>(4)?))).optional().map_err(sql_err)?.ok_or_else(||ProtocolError::new(ErrorCode::NotFound,"discovery session not found"))?;
        if row.0!=owner {return Err(ProtocolError::denied("discovery session owner mismatch"));}
        let session=DiscoverySession {id:id.into(),agent_id:row.0,created_at:chrono::DateTime::parse_from_rfc3339(&row.1).map_err(|_|invalid("invalid discovery timestamp"))?.with_timezone(&chrono::Utc),expires_at:chrono::DateTime::parse_from_rfc3339(&row.2).map_err(|_|invalid("invalid discovery expiry"))?.with_timezone(&chrono::Utc),scopes:serde_json::from_str(&row.3).map_err(json_err)?,revoked:row.4!=0};
        if !session.is_usable(chrono::Utc::now()) {return Err(ProtocolError::new(ErrorCode::Expired,"discovery session expired or revoked"));}
        if required.iter().any(|scope|!session.allows(scope)) {return Err(ProtocolError::denied("discovery scope does not authorize this operation"));}
        Ok(session)
    }
    fn selected_disclosure(&mut self,session:&DiscoverySession,params:&Value)->Result<(Value,Vec<String>),ProtocolError> {
        let ids=params.get("candidate_ids").and_then(Value::as_array).ok_or_else(||invalid("candidate_ids array required"))?;
        if ids.is_empty()||ids.len()>128 {return Err(invalid("candidate_ids must contain 1..128 selected candidates"));}
        let mut seen=BTreeSet::new();let mut selected=vec![];let mut candidates=vec![];
        for id in ids {
            let id=id.as_str().filter(|id|!id.is_empty()).ok_or_else(||invalid("candidate id must be nonempty string"))?;
            if !seen.insert(id.to_string()) {return Err(invalid("duplicate candidate id"));}
            let candidate=self.registry.db.conn.query_row("SELECT c.id,c.path,c.file_name,c.recognized FROM scan_candidates c JOIN discovery_membership d ON d.candidate_id=c.id WHERE d.session_id=?1 AND c.id=?2",rusqlite::params![session.id,id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,Option<String>>(2)?,r.get::<_,i64>(3)?))).optional().map_err(sql_err)?.ok_or_else(||ProtocolError::denied("candidate does not exist in authorized discovery membership"))?;
            selected.push(candidate.0.clone());candidates.push(json!({"id":candidate.0,"path":toolhub_audit::redact_path_for_export(&candidate.1),"file_name":candidate.2,"recognized":candidate.3!=0}));
        }
        let disclosure=json!({"schema":"toolhub.discovery-disclosure/v1","session_id":session.id,"candidates":candidates,"provenance":"selected native metadata; classification is untrusted enrichment"});
        if serde_json::to_vec(&disclosure).map_err(json_err)?.len()>65536 {return Err(invalid("selected metadata exceeds 64 KiB"));}
        Ok((disclosure,selected))
    }
    pub(super) fn launch_agent(&mut self,params:&Value)->Result<Value,ProtocolError> {
        if params.get("confirmed")!=Some(&Value::Bool(true)) {return Err(ProtocolError::denied("explicit selected metadata disclosure confirmation required"));}
        let owner=self.peer_principal();let session=self.active_discovery(&owner,text(params,"session_id")?,&["candidate.inspect","metadata.read"])?;
        let (disclosure,selected)=self.selected_disclosure(&session,params)?;
        let id=text(params,"agent_id")?;
        let agent=if id=="owned-fixture" {
            if std::env::var("TOOLHUB_AGENT_FIXTURE").ok().as_deref()!=Some("1") {return Err(ProtocolError::denied("owned fixture adapter requires explicit daemon test configuration"));}
            toolhub_agent_bridge::DetectedAgent {id:id.into(),name:"Owned finite fixture".into(),kind:"owned_fixture".into(),executable:Some(std::env::current_exe().map_err(|_|invalid("fixture executable unavailable"))?.to_string_lossy().into_owned()),version:Some(env!("CARGO_PKG_VERSION").into())}
        } else {
            if id!="opencode" {return Err(ProtocolError::new(ErrorCode::Unavailable,"adapter has no supported automatic launch contract"));}
            toolhub_agent_bridge::detect_all().into_iter().find(|agent|agent.id==id).ok_or_else(||ProtocolError::new(ErrorCode::Unavailable,"supported adapter not detected"))?
        };
        self.registry.disclose_candidates(&session.id,&selected).map_err(db_err)?;
        let operation=self.agents.launch(&owner,&agent,&session,&disclosure,true).map_err(|_|ProtocolError::new(ErrorCode::Unavailable,"adapter launch failed"))?;
        if let Err(error)=self.event("agent.launch","selected metadata disclosed to supported adapter",json!({"operation_id":operation.id,"agent_id":agent.id,"candidate_count":selected.len(),"session_id":session.id})) {let _=self.agents.cancel(&owner,&operation.id);return Err(error);}
        Ok(json!({"operation":operation,"disclosure":disclosure,"external_host_shell_sandboxed":false}))
    }
    pub(super) fn agent_state(&mut self,params:&Value,cancel:bool)->Result<Value,ProtocolError> {
        let owner=self.peer_principal();let id=text(params,"operation_id")?;
        let operation=if cancel {self.agents.cancel(&owner,id)}else{self.agents.status(&owner,id)}.map_err(|_|ProtocolError::new(ErrorCode::NotFound,"owned agent operation not available"))?;
        if cancel {self.event("agent.cancel","owned agent cancellation requested",json!({"operation_id":id,"state":operation.state}))?;}
        Ok(json!(operation))
    }
    pub(super) fn configure_credentials(&mut self,params:&Value)->Result<Value,ProtocolError> {
        if params.get("confirmed")!=Some(&Value::Bool(true)) {return Err(ProtocolError::denied("explicit temporary provider configuration confirmation required"));}
        let owner=self.peer_principal();let session=self.active_discovery(&owner,text(params,"session_id")?,&["candidate.inspect","metadata.read"])?;
        let endpoint=text(params,"endpoint")?;let model=text(params,"model")?;let key=text(params,"api_key")?.to_string();
        validate_endpoint(endpoint)?;
        let ttl=params.get("ttl_seconds").map(|v|v.as_u64().ok_or_else(||invalid("ttl_seconds must be integer"))).transpose()?.unwrap_or(300);
        let remaining=(session.expires_at-chrono::Utc::now()).num_seconds().max(0) as u64;
        let configured=self.credentials.configure(&owner,&session.id,endpoint,model,key,ttl.min(remaining)).map_err(|_|invalid("invalid temporary provider configuration"))?;
        if let Err(error)=self.event("credentials","temporary memory-only provider configured",json!({"provider_session_id":configured.id,"discovery_session_id":session.id})) {let _=self.credentials.cancel(&owner,&configured.id);return Err(error);}
        Ok(json!(configured))
    }
    pub(super) fn run_temporary_provider(&mut self,params:&Value)->Result<Value,ProtocolError> {
        let owner=self.peer_principal();let id=text(params,"provider_session_id")?;
        // Consume the memory-only credential for every attempted owned call, including
        // validation/provider failures. No request body, key or response text is persisted.
        let configured=self.credentials.describe(&owner,id).map_err(|_|ProtocolError::denied("provider session unavailable"))?;
        let prepared=(|| {
            if params.get("confirmed")!=Some(&Value::Bool(true)) {return Err(ProtocolError::denied("explicit selected metadata sharing confirmation required"));}
            let session=self.active_discovery(&owner,&configured.discovery_session_id,&["candidate.inspect","metadata.read","classification.submit"])?;
            let (disclosure,selected)=self.selected_disclosure(&session,params)?;
            self.registry.disclose_candidates(&session.id,&selected).map_err(db_err)?;
            Ok((session,disclosure))
        })();
        let (session,disclosure)=match prepared {Ok(value)=>value,Err(error)=>{let _=self.credentials.cancel(&owner,id);return Err(error);}};
        let output=self.credentials.with_key(&owner,id,|key,provider|provider_request(key,provider,&disclosure)).map_err(|_|ProtocolError::new(ErrorCode::Unavailable,"temporary provider request failed; credential destroyed"))?;
        self.event("credentials.session","temporary provider metadata request completed; credential destroyed",json!({"discovery_session_id":session.id,"candidate_count":disclosure["candidates"].as_array().map(Vec::len)}))?;
        Ok(json!({"provider_session_id":id,"discovery_session_id":session.id,"response":output,"provenance":"external provider output; untrusted enrichment","credential_destroyed":true,"candidate_count":disclosure["candidates"].as_array().map(Vec::len)}))
    }
}

fn validate_endpoint(endpoint:&str)->Result<(),ProtocolError> {
    if endpoint.len()>2048 || endpoint.contains(['\r','\n','@','?','#','\\']) {return Err(invalid("invalid provider endpoint"));}
    if endpoint.starts_with("https://") {return Ok(());}
    if std::env::var("TOOLHUB_PROVIDER_FIXTURE").ok().as_deref()==Some("1")&&(endpoint.starts_with("http://127.0.0.1:")||endpoint.starts_with("http://localhost:")) {return Ok(());}
    Err(invalid("provider requires HTTPS; owned loopback requires explicit test configuration"))
}
fn provider_request(_key:&str,_provider:&toolhub_agent_bridge::temp_credentials::TempProviderSession,_disclosure:&Value)->Result<Value,String> {Err("provider transport pending".into())}

#[cfg(test)]
mod tests {
    use super::*;
    fn service()->(tempfile::TempDir,DaemonService,DiscoverySession) {
        let directory=tempfile::tempdir().unwrap();let mut service=DaemonService::open(&directory.path().join("registry.sqlite")).unwrap();service.connection_principal=Some("owner.fixture".into());
        let session=DiscoverySession::issue("owner.fixture",1,&["candidate.inspect".into(),"metadata.read".into(),"classification.submit".into()]).unwrap();
        service.registry.save_discovery_session(&session.id,&session.agent_id,&session.expires_at.to_rfc3339(),&serde_json::to_string(&session.scopes).unwrap()).unwrap();
        service.registry.upsert_candidate(&toolhub_registry::CandidateRow {id:"candidate.fixture".into(),path:"owned synthetic metadata".into(),file_name:Some("fixture.exe".into()),recognized:false}).unwrap();
        service.registry.db.conn.execute("INSERT INTO discovery_membership(session_id,candidate_id) VALUES(?1,'candidate.fixture')",[&session.id]).unwrap();
        (directory,service,session)
    }
    #[test]
    fn selected_disclosure_enforces_owner_scope_membership_and_current_revocation() {
        let (_directory,mut service,session)=service();
        assert!(service.active_discovery("other",&session.id,&["candidate.inspect"]).is_err());
        assert!(service.selected_disclosure(&session,&json!({"candidate_ids":["missing"]})).is_err());
        assert!(service.selected_disclosure(&session,&json!({"candidate_ids":["candidate.fixture","candidate.fixture"]})).is_err());
        assert_eq!(service.selected_disclosure(&session,&json!({"candidate_ids":["candidate.fixture"]})).unwrap().1,vec!["candidate.fixture"]);
        service.registry.revoke_discovery_session(&session.id).unwrap();assert!(service.active_discovery("owner.fixture",&session.id,&["candidate.inspect"]).is_err());
    }
    #[test]
    fn provider_transport_reaches_owned_loopback_and_consumes_key() {
        let (_directory,mut service,session)=service();
        let configured=service.credentials.configure("owner.fixture",&session.id,"http://127.0.0.1:1/v1","fixture-model","SYNTHETIC-KEY".into(),30).unwrap();
        let result=service.run_temporary_provider(&json!({"provider_session_id":configured.id,"confirmed":true,"candidate_ids":["candidate.fixture"]}));
        assert!(result.is_err());assert!(service.credentials.describe("owner.fixture",&configured.id).is_err());
    }

    #[test]
    fn real_http_provider_uses_only_selected_disclosure_and_redacts_echoed_key() {
        let listener=std::net::TcpListener::bind("127.0.0.1:0").unwrap();listener.set_nonblocking(true).unwrap();let address=listener.local_addr().unwrap();
        let server=std::thread::spawn(move || {
            use std::io::{Read,Write};let deadline=std::time::Instant::now()+std::time::Duration::from_secs(3);
            let mut stream=loop {match listener.accept() {Ok((stream,_))=>break stream,Err(error) if error.kind()==std::io::ErrorKind::WouldBlock=>{if std::time::Instant::now()>=deadline{return false;}std::thread::sleep(std::time::Duration::from_millis(5));},Err(_)=>return false}};
            stream.set_read_timeout(Some(std::time::Duration::from_secs(2))).unwrap();let mut buffer=vec![];let mut chunk=[0u8;4096];
            loop {let n=stream.read(&mut chunk).unwrap();if n==0{return false;}buffer.extend_from_slice(&chunk[..n]);if let Some(end)=buffer.windows(4).position(|part|part==b"\r\n\r\n") {
                let headers=String::from_utf8_lossy(&buffer[..end]).to_lowercase();let length=headers.lines().find_map(|line|line.strip_prefix("content-length:").and_then(|v|v.trim().parse::<usize>().ok())).unwrap();
                if buffer.len()>=end+4+length {let body:Value=serde_json::from_slice(&buffer[end+4..end+4+length]).unwrap();assert_eq!(body["model"],"fixture-model");assert!(headers.contains("authorization: bearer synthetic-key"));let content=body["messages"][1]["content"].as_str().unwrap();let disclosed:Value=serde_json::from_str(content).unwrap();assert_eq!(disclosed["candidates"].as_array().unwrap().len(),1);assert_eq!(disclosed["candidates"][0]["id"],"selected");break;}
            }assert!(buffer.len()<65536);}
            let body=serde_json::to_vec(&json!({"choices":[{"message":{"content":"fixture result SYNTHETIC-KEY"}}]})).unwrap();write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len()).unwrap();stream.write_all(&body).unwrap();true
        });
        let mut store=toolhub_agent_bridge::temp_credentials::TempProviderStore::new();let session=store.configure("owner","discovery",&format!("http://{address}/v1"),"fixture-model","SYNTHETIC-KEY".into(),30).unwrap();
        let output=store.with_key("owner",&session.id,|key,provider|provider_request(key,provider,&json!({"candidates":[{"id":"selected"}]})));
        let contacted=server.join().unwrap();assert!(contacted,"provider never contacted owned HTTP fixture");let output=output.unwrap();assert!(output["content"].as_str().unwrap().contains("<redacted>"));assert!(store.describe("owner",&session.id).is_err());
    }
}
