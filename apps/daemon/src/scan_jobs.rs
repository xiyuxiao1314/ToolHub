use super::*;
use super::execution_jobs::{invalid,text};
use std::sync::atomic::Ordering;

pub struct PreparedScan {
    id:String, mode_name:String, mode:toolhub_scanner::ScanMode,
    roots:Option<Vec<String>>, extensions:Vec<toolhub_scanner::DeclarativeExtension>,
    cancelled:Arc<AtomicBool>, rpc_id:Option<Value>, principal:String,
}
impl PreparedScan {
    pub fn run(&self)->toolhub_scanner::ScanReport {
        toolhub_scanner::run_scan_with_cancel(self.mode,self.roots.clone(),&self.extensions,&self.cancelled)
    }
}
impl DaemonService {
    pub fn prepare_scan(&mut self,req:&JsonRpcRequest,principal:&str,controller:bool)->Result<PreparedScan,JsonRpcResponse> {
        let previous=self.connection_principal.replace(principal.into());
        let old=std::mem::replace(&mut self.controller_verified,controller);
        let result=toolhub_protocol::validate_method_params(req).and_then(|()|self.prepare_scan_job(&req.params,req.id.clone()));
        self.connection_principal=previous;self.controller_verified=old;
        result.map_err(|e|error_response(req.id.clone(),&e))
    }
    fn prepare_scan_job(&mut self,params:&Value,rpc_id:Option<Value>)->Result<PreparedScan,ProtocolError> {
        let settings=self.registry.get_setting("user_preferences").map_err(db_err)?.unwrap_or_else(||json!({}));
        let mode_name=params.get("mode").or_else(||settings.get("scan_mode")).and_then(Value::as_str).unwrap_or("quick").to_string();
        if !matches!(mode_name.as_str(),"quick"|"full"|"custom"){return Err(invalid("invalid scan mode"));}
        let roots=params.get("roots").or_else(||if mode_name=="custom"{settings.get("scan_roots")}else{None}).map(|roots| {
            let roots=roots.as_array().ok_or_else(||invalid("roots must be array"))?;
            if roots.is_empty()||roots.len()>64{return Err(invalid("1..64 roots required"));}
            roots.iter().map(|v|{let path=v.as_str().ok_or_else(||invalid("root must be string"))?;if !Path::new(path).is_absolute(){return Err(invalid("absolute root required"));}Ok(path.to_string())}).collect::<Result<Vec<_>,_>>()
        }).transpose()?;
        if mode_name=="custom"&&roots.is_none(){return Err(invalid("custom scan requires roots"));}
        let extensions=settings.get("extensions").map(|v|serde_json::from_value::<Vec<toolhub_scanner::DeclarativeExtension>>(v.clone()).map_err(json_err)).transpose()?.unwrap_or_default();
        for extension in &extensions{extension.validate().map_err(|_|invalid("invalid scanner extension"))?;}
        let id=params.get("scan_session_id").map(|v|v.as_str().filter(|v|!v.is_empty()&&v.len()<=128).map(str::to_string).ok_or_else(||invalid("invalid scan_session_id"))).transpose()?;
        // Only one scan reconciles a registry at a time; ordinary reads and executions remain live.
        if !self.running_scans.is_empty(){return Err(ProtocolError::new(ErrorCode::Unavailable,"scan already running"));}
        let id=if let Some(id)=id {self.registry.db.conn.execute("INSERT INTO scan_sessions(id,mode,status,started_at) VALUES(?1,?2,'running',?3)",rusqlite::params![id,mode_name,chrono::Utc::now().to_rfc3339()]).map_err(sql_err)?;id}else{self.registry.begin_scan(&mode_name).map_err(db_err)?};
        let cancelled=Arc::new(AtomicBool::new(false));let principal=self.peer_principal();
        self.running_scans.insert(id.clone(),(principal.clone(),Arc::clone(&cancelled)));
        if let Err(e)=self.event("scan.started","read-only scan started",json!({"scan_session_id":id,"mode":mode_name})){self.running_scans.remove(&id);return Err(e);}
        Ok(PreparedScan{id,mode:if mode_name=="full"{toolhub_scanner::ScanMode::Full}else{toolhub_scanner::ScanMode::Quick},mode_name,roots,extensions,cancelled,rpc_id,principal})
    }
    pub fn finish_scan(&mut self,job:PreparedScan,report:toolhub_scanner::ScanReport)->JsonRpcResponse {
        let id=job.rpc_id.clone();let previous=self.connection_principal.replace(job.principal.clone());
        let result=self.finish_scan_job(&job,&report);self.connection_principal=previous;
        match result{Ok(value)=>JsonRpcResponse{jsonrpc:"2.0".into(),id,result:Some(value),error:None},Err(e)=>error_response(id,&e)}
    }
    fn finish_scan_job(&mut self,job:&PreparedScan,report:&toolhub_scanner::ScanReport)->Result<Value,ProtocolError> {
        self.running_scans.remove(&job.id);
        let coverage=json!({"roots_ok":report.coverage.roots_ok,"roots_failed":report.coverage.roots_failed,"scopes":report.coverage.scopes,"limitations":report.coverage.limitations,"candidates":report.candidates.len()});
        let result=self.ingest_scan(report,&job.mode_name);
        let state=if result.is_err(){"failed"}else if report.cancelled{"cancelled"}else if report.coverage.scopes.iter().any(|s|!s.complete)||!report.coverage.roots_failed.is_empty(){"partial"}else{"completed"};
        self.registry.finish_scan(&job.id,state,&coverage.to_string(),&json!(report.coverage.roots_failed).to_string()).map_err(db_err)?;
        result?;
        self.event("scan.finished","read-only scan finished",json!({"scan_session_id":job.id,"status":state,"candidates":report.candidates.len()}))?;
        Ok(json!({"scan_session_id":job.id,"mode":job.mode_name,"status":state,"cancelled":report.cancelled,"candidates":report.candidates.len(),"recognized":self.registry.count_instances().map_err(db_err)?,"unknown_candidates":self.registry.count_candidates().map_err(db_err)?,"evidence":self.registry.count_evidence().map_err(db_err)?,"coverage":coverage}))
    }
    pub(super) fn synchronous_scan(&mut self,params:&Value)->Result<Value,ProtocolError>{let job=self.prepare_scan_job(params,None)?;let report=job.run();self.finish_scan_job(&job,&report)}
    pub(super) fn cancel_scan(&mut self,params:&Value)->Result<Value,ProtocolError>{let id=text(params,"scan_session_id")?;let (owner,token)=self.running_scans.get(id).ok_or_else(||ProtocolError::new(ErrorCode::NotFound,"scan not running"))?;if owner!=&self.peer_principal()&&!self.is_admin(){return Err(ProtocolError::denied("scan owned by another caller"));}token.store(true,Ordering::SeqCst);Ok(json!({"scan_session_id":id,"cancel_requested":true,"cancelled":false}))}
}
