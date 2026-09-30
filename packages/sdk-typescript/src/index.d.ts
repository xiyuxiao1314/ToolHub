export type Json = null | boolean | number | string | Json[] | { [key: string]: Json };
export type RequestId = number | string | null;
export interface RpcRequest { jsonrpc: '2.0'; id: RequestId; method: string; params?: Json[] | Record<string, Json>; }
export type RpcResponse<T=Json> = { jsonrpc: '2.0'; id: RequestId; result: T; error?: never } | { jsonrpc: '2.0'; id: RequestId; result?: never; error: { code: number; message: string; data?: Json } };
export interface CallOptions { timeoutMs?: number; signal?: AbortSignal; }
export interface Transport { (request: RpcRequest, options?: CallOptions): Promise<RpcResponse>; close?: () => void; }
export interface StatusResult { protocol_version: string; daemon: string; registry_path: string; tool_count: number; candidate_count: number; last_scan: string | null; }
export interface InstanceRow { id: string; definition_id: string; name: string; version: string | null; path: string; canonical_path: string | null; environment_id: string | null; trust: string; status: string; arch: string; platform: string; }
export interface CandidateInstance { instance_id:string; definition_id:string; name:string; version:string|null; path:string; environment:string|null; trust:string; arch:string; cwd_match:boolean; }
export interface ResolveResult { capability:string; canonical:string; selected:CandidateInstance|null; alternatives:CandidateInstance[]; explanation:string; error:string|null; eligibility_error:'unknown_capability'|'invalid_preference'|'invalid_version_constraint'|'no_eligible_provider'|null; rejected:{candidate:CandidateInstance;reasons:string[]}[]; fallback_allowed:boolean; }
export interface ResolveOptions { version?:string; cwd?:string; prefer_environment?:string; preferred_environment?:string; min_version?:string; require_trust?:'verified'|'known'|'user_trusted'; require_arch?:string; }
export interface ExecuteParams { instance_id?:string; capability?:string; args:string[]; cwd?:string; approval_id?:string; session_id?:string; execution_id?:string; timeout_ms?:number; max_output_bytes?:number; stdin?:string; }
export interface ExecutionResult { status:'success'|'failed'|'denied'|'expired'|'timed_out'|'cancelled'|'unavailable'|'invalid_request'; exit_code?:number; stdout:string; stderr:string; duration_ms:number; truncated:boolean; error_code?:string; fallback_allowed?:boolean; execution_id?:string; }
export interface ApprovalRequest { request_id:string; session_id:string; expires_at:string; }
export interface MethodMap {
 'status': { params: {}; result: StatusResult };
 'capability.list': { params: {}; result: Json[] };
 'registry.correct': { params: {id:string;trust?:'verified'|'known'|'user_trusted'|'unknown'|'blocked';name?:string}; result: Json };

 'ping': { params: {}; result: Json };
 'protocol.negotiate': { params: {versions:string[]}; result: Json };
 'registry.search': { params: {query:string}; result: InstanceRow[] };
 'registry.inspect_instance': { params: {id:string}; result: Json };
 'resolve.capability': { params: {capability:string}&ResolveOptions; result: ResolveResult };
 'scan.start': { params: {mode:'quick'|'full'|'custom'}; result: Json };
 'execute.tool': { params: ExecuteParams; result: ExecutionResult };
 'execute.approval_request': { params: ExecuteParams; result: ApprovalRequest };
 'execute.cancel': { params: {execution_id:string}; result: Json };
 'activity.list': { params: {}; result: Json[] };
 'settings.get': { params: {}; result: Json };
 'settings.set': { params: Record<string,Json>; result: Json };
 'skill.register': { params: {path:string}; result: Json };
 'skill.resolve': { params: {id:string}; result: Json };
 'skill.list': { params: {}; result: Json[] };
 'skill.inspect': { params: {id:string}; result: Json };
 'discovery.start': { params: {agent_id:string}; result: Json };
 'discovery.list': { params: {}; result: Json[] };
 'discovery.inspect': { params: {id:string}; result: Json };
 'discovery.classify': { params: Record<string,Json>; result: Json };
 'discovery.revoke': { params: {session_id:string}; result: Json };
 'import.report': { params: {report:Json}; result: Json };
 'export.report': { params: {format:string}; result: Json };
 'environment.list': { params: {}; result: Json[] };
 'environment.duplicates': { params: {}; result: Json[] };
 'agent.list': { params: {}; result: Json[] };
 'agent.launch': { params: {agent_id:string;args?:string[]}; result: Json };
 'agent.status': { params: {operation_id:string}; result: Json };
 'agent.cancel': { params: {operation_id:string}; result: Json };
 'scan.status': { params: {scan_session_id:string}; result: Json };
 'scan.cancel': { params: {scan_session_id:string}; result: Json };
 'registry.inspect_tool': { params: {id:string}; result: Json };
 'execute.approve': { params: {request_id:string}; result: Json };
 'execute.approvals': { params: {}; result: Json[] };
 'execute.revoke': { params: {approval_id:string}; result: Json };
 'policy.get': { params: {}; result: Json };
 'policy.set': { params: {scope:string;subject:string;action:'allow'|'ask'|'deny'}; result: Json };
 'resource.list': { params: {}; result: Json };
 'resource.activate': { params: {path:string}; result: Json };
 'resource.rollback': { params: {}; result: Json };
 'update.verify': { params: {manifest_path:string}; result: Json };
 'update.apply': { params: {update_id:string;destination:string;confirmed:true}; result: Json };
 'update.cancel': { params: {update_id:string}; result: Json };
 'credentials.set': { params: Record<string,Json>; result: Json };
 'credentials.clear': { params: Record<string,Json>; result: Json };
 'credentials.status': { params: {}; result: Json };
 'credentials.session': { params: Record<string,Json>; result: Json };
 'events.subscribe': { params: {after_seq?:number}; result: {subscription_id:string;after_seq:number;retained:number} };
 'events.poll': { params: {subscription_id:string;limit?:number}; result: {events:Json[];last_seq:number;gap:boolean;oldest_seq:number} };
 'events.unsubscribe': { params: {subscription_id:string}; result: {unsubscribed:boolean} };
}
export declare class ToolhubError extends Error { code:string; data?:Json; constructor(code:string,message:string,data?:Json); }
export declare class ToolhubClient {
 constructor(transport:Transport);
 call<K extends keyof MethodMap>(method:K, params:MethodMap[K]['params'], options?:CallOptions):Promise<MethodMap[K]['result']>;
 status(options?:CallOptions):Promise<StatusResult>;
 negotiate(options?:CallOptions):Promise<Json>;
 search(query:string,options?:CallOptions):Promise<InstanceRow[]>;
 resolveCapability(capability:string,opts?:ResolveOptions,options?:CallOptions):Promise<ResolveResult>;
 scan(mode?:'quick'|'full'|'custom',options?:CallOptions):Promise<Json>;
 execute(params:ExecuteParams,options?:CallOptions):Promise<ExecutionResult>;
 requestApproval(params:ExecuteParams,options?:CallOptions):Promise<ApprovalRequest>;
 close():void;
}
export declare function nodeStdioTransport(child:StdioChild,options?:{timeoutMs?:number;maxBytes?:number;maxPending?:number}):Transport;
export declare function nodeCliTransport(options?:{executable?:string;timeoutMs?:number;maxPending?:number;env?:Record<string,string|undefined>}):Transport;

export interface StdioChild { stdout:EventStream;stdin:EventStream&{write(data:string,callback:(error?:Error|null)=>void):boolean};exitCode?:number|null;on(name:string,listener:(...args:any[])=>void):unknown;off(name:string,listener:(...args:any[])=>void):unknown; }
export interface EventStream { setEncoding?(encoding:string):unknown;on(name:string,listener:(...args:any[])=>void):unknown;off(name:string,listener:(...args:any[])=>void):unknown; }




