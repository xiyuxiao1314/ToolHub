import test from 'node:test';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { ToolhubClient, nodeStdioTransport } from '../src/index.js';
function child(script) { return spawn(process.execPath, ['-e', script], { stdio: ['pipe', 'pipe', 'pipe'] }); }
test('pending request rejects on child disconnect', async () => {
  const proc = child('process.stdin.once("data",()=>process.exit(0))');
  const transport = nodeStdioTransport(proc, { timeoutMs: 500 });
  try { await assert.rejects(transport({jsonrpc:'2.0', id:1, method:'status', params:{}}), /closed|exit|disconnect/); }
  finally { proc.kill(); transport.close?.(); }
});
test('deadline clears request and next request still resolves', async () => {
  const proc = child('let b="";process.stdin.on("data",c=>{b+=c;let i;while((i=b.indexOf("\\n"))>=0){let r=JSON.parse(b.slice(0,i));b=b.slice(i+1);if(r.id===2)console.log(JSON.stringify({jsonrpc:"2.0",id:r.id,result:{ok:true}}));}})');
  const transport = nodeStdioTransport(proc, { timeoutMs: 200 });
  try { await assert.rejects(transport({jsonrpc:'2.0',id:1,method:'status',params:{}}), /deadline|timeout/);assert.deepEqual((await transport({jsonrpc:'2.0',id:2,method:'status',params:{}})).result,{ok:true}); }
  finally { transport.close?.();proc.kill(); }
});
test('rejects unmatched or invalid response envelope', async () => {
  const client=new ToolhubClient(async req=>({jsonrpc:'2.0',id:req.id+1,result:{}}));
  await assert.rejects(client.status(),/invalid|mismatch/);
});
test('oversized stdout cannot grow transport buffer', async () => {
  const proc=child('process.stdin.once("data",()=>process.stdout.write("x".repeat(2048)))');
  const transport=nodeStdioTransport(proc,{maxBytes:1024,timeoutMs:500});
  try {await assert.rejects(transport({jsonrpc:'2.0',id:1,method:'status',params:{}}),/large|limit/);}finally{transport.close?.();proc.kill();}
});
