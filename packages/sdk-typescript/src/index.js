const MAX_BYTES = 1024 * 1024;
export class ToolhubError extends Error {
  constructor(code, message, data) { super(`${code}: ${message}`); this.name='ToolhubError'; this.code=code; this.data=data; }
}
function validateResponse(res, id) {
  if (!res || res.jsonrpc !== '2.0' || res.id !== id || (Object.hasOwn(res,'result') === Object.hasOwn(res,'error'))) throw new ToolhubError('invalid_response','mismatched or invalid JSON-RPC response');
  if (res.error && (!Number.isInteger(res.error.code) || typeof res.error.message !== 'string')) throw new ToolhubError('invalid_response','invalid error object');
}
export class ToolhubClient {
  constructor(transport) { this.transport=transport; this.nextId=1; }
  async call(method, params={}, options) {
    if (typeof method!=='string' || !method.length || method.length>256 || (!Array.isArray(params) && (params===null || typeof params!=='object'))) throw new ToolhubError('invalid_request','method or params invalid');
    const id=this.nextId++;
    const res=await this.transport({jsonrpc:'2.0',id,method,params},options);
    validateResponse(res,id);
    if (res.error) throw new ToolhubError(res.error.data?.error_code || 'daemon_error',res.error.message,res.error.data);
    return res.result;
  }
  status(options) { return this.call('status',{},options); }
  negotiate(options) { return this.call('protocol.negotiate',{versions:['1.0']},options); }
  search(query,options) { return this.call('registry.search',{query},options); }
  resolveCapability(capability,opts={},options) { return this.call('resolve.capability',{capability,...opts},options); }
  scan(mode='quick',options) { return this.call('scan.start',{mode},options); }
  execute(params,options) { return this.call('execute.tool',params,options); }
  requestApproval(params,options) { return this.call('execute.approval_request',params,options); }
  close() { this.transport.close?.(); }
}

/** Bounded newline JSON-RPC compatibility transport. The shared managed daemon is reached through the CLI for SDK examples. */
export function nodeStdioTransport(child, { timeoutMs=30_000, maxBytes=MAX_BYTES, maxPending=128 }={}) {
  if (!Number.isInteger(maxBytes) || maxBytes<1 || maxBytes>MAX_BYTES || !Number.isInteger(maxPending) || maxPending<1 || maxPending>1024 || !Number.isFinite(timeoutMs) || timeoutMs<1 || timeoutMs>600_000) throw new ToolhubError('invalid_params','invalid transport bounds');
  const pending=new Map();let buffer='';let closed=false;
  const settle=(id,error,value)=>{const item=pending.get(id);if(!item)return;pending.delete(id);clearTimeout(item.timer);item.signal?.removeEventListener('abort',item.abort);if(error)item.reject(error);else item.resolve(value);};
  const shutdown=(error)=>{if(closed)return;closed=true;buffer='';for(const id of [...pending.keys()])settle(id,error);child.stdout.off('data',onData);child.stdout.off('error',onError);child.stdout.off('end',onEnd);child.stdin.off('error',onError);child.off('error',onError);child.off('exit',onExit);child.off('close',onEnd);};
  const onError=error=>shutdown(error);const onEnd=()=>shutdown(new ToolhubError('closed','daemon disconnected'));const onExit=()=>shutdown(new ToolhubError('closed','daemon exited'));
  const onData=chunk=>{
    buffer+=chunk;
    let end;
    while((end=buffer.indexOf('\n'))>=0){
      const line=buffer.slice(0,end);buffer=buffer.slice(end+1);
      if(Buffer.byteLength(line)>maxBytes){shutdown(new ToolhubError('payload_too_large','response exceeds byte limit'));return;}
      if(!line.trim())continue;
      let msg;try{msg=JSON.parse(line);}catch{shutdown(new ToolhubError('parse_error','malformed daemon response'));return;}
      // Unknown IDs may be late replies to expired requests; never correlate them to a newer request.
      if (!pending.has(msg?.id)) continue;
      try {validateResponse(msg,msg.id);settle(msg.id,null,msg);}catch(error){settle(msg.id,error);}
    }
    if(Buffer.byteLength(buffer)>maxBytes){shutdown(new ToolhubError('payload_too_large','response exceeds byte limit'));}
  };
  child.stdout.setEncoding('utf8');child.stdout.on('data',onData);child.stdout.on('error',onError);child.stdout.on('end',onEnd);child.stdin.on('error',onError);child.on('error',onError);child.on('exit',onExit);child.on('close',onEnd);
  const transport=(req,options={})=>new Promise((resolve,reject)=>{
    if(closed || child.exitCode!==null && child.exitCode!==undefined){reject(new ToolhubError('closed','daemon disconnected'));return;}
    if(pending.size>=maxPending){reject(new ToolhubError('overloaded','pending request limit'));return;}
    const timeout=options.timeoutMs??timeoutMs;
    if (!Number.isFinite(timeout) || timeout<1 || timeout>600_000){reject(new ToolhubError('invalid_params','invalid deadline'));return;}
    if(!req || req.jsonrpc!=='2.0' || !(typeof req.id==='number' && Number.isFinite(req.id) || typeof req.id==='string') || pending.has(req.id)){reject(new ToolhubError('invalid_request','request ID invalid or duplicate'));return;}
    let line;try{line=JSON.stringify(req)+'\n';}catch(error){reject(error);return;}
    if(Buffer.byteLength(line)>maxBytes){reject(new ToolhubError('payload_too_large','request exceeds byte limit'));return;}
    const item={resolve,reject,signal:options.signal};
    item.abort=()=>settle(req.id,new ToolhubError('cancelled','request aborted'));
    item.timer=setTimeout(()=>settle(req.id,new ToolhubError('timeout','request deadline expired')),timeout);
    pending.set(req.id,item);
    if(options.signal?.aborted){item.abort();return;}
    options.signal?.addEventListener('abort',item.abort,{once:true});
    try{child.stdin.write(line,error=>{if(error)settle(req.id,error);});}catch(error){settle(req.id,error);}
  });
  transport.close=()=>shutdown(new ToolhubError('closed','transport closed'));
  return transport;
}

/** Uses toolhub CLI's authenticated shared-service bridge, never starts a private stdio daemon. */
export function nodeCliTransport({ executable='toolhub', timeoutMs=30_000, maxPending=128, env }={}) {
  let closed=false;let active=0;const children=new Set();
  const transport=async (req,options={})=>{
    if(closed)throw new ToolhubError('closed','transport closed');
    if(active>=maxPending)throw new ToolhubError('overloaded','pending request limit');
    const timeout=options.timeoutMs??timeoutMs;
    if(!Number.isFinite(timeout)||timeout<1||timeout>600_000)throw new ToolhubError('invalid_params','invalid deadline');
    const input=JSON.stringify(req.params??{})+'\n';
    if(Buffer.byteLength(input)>MAX_BYTES)throw new ToolhubError('payload_too_large','request exceeds byte limit');
    active++;
    try {
    const {spawn}=await import('node:child_process');
    return await new Promise((resolve,reject)=>{
      if(closed){reject(new ToolhubError('closed','transport closed'));return;}
      const child=spawn(executable,['--json','rpc',req.method],{stdio:['pipe','pipe','pipe'],windowsHide:true,env});children.add(child);
      let stdout='';let stderr='';let settled=false;
      const finish=(error,value)=>{if(settled)return;settled=true;clearTimeout(timer);options.signal?.removeEventListener('abort',abort);children.delete(child);if(error)reject(error);else resolve(value);};
      const abort=()=>{child.kill();finish(new ToolhubError('cancelled','request aborted'));};
      const timer=setTimeout(()=>{child.kill();finish(new ToolhubError('timeout','request deadline expired'));},timeout);
      child._toolhubClose=()=>{child.kill();finish(new ToolhubError('closed','transport closed'));};
      child.stdout.setEncoding('utf8');child.stderr.setEncoding('utf8');
      child.stdout.on('data',chunk=>{if(settled)return;stdout+=chunk;if(Buffer.byteLength(stdout)>MAX_BYTES){child.kill();finish(new ToolhubError('payload_too_large','response exceeds byte limit'));}});
      child.stderr.on('data',chunk=>{if(Buffer.byteLength(stderr)<16384)stderr+=(chunk.slice(0,16384));});
      child.on('error',error=>finish(error));child.stdin.on('error',error=>finish(error));
      child.on('close',code=>{if(settled)return;try{const value=JSON.parse(stdout);if(value?.ok===false && typeof value.error==='string'){finish(null,{jsonrpc:'2.0',id:req.id,error:{code:-32000,message:value.error}});}else{finish(null,{jsonrpc:'2.0',id:req.id,result:value});}}catch{finish(new ToolhubError('daemon_error',`CLI exited ${code}: ${stderr.slice(0,256)}`));}});
      if(options.signal?.aborted){abort();return;}options.signal?.addEventListener('abort',abort,{once:true});child.stdin.end(input);
    });
    } finally { active--; }
  };
  transport.close=()=>{closed=true;for(const child of [...children])child._toolhubClose();};return transport;
}


