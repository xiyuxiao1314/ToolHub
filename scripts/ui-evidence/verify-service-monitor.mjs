import {chromium} from 'playwright'
import {spawn} from 'node:child_process';import fs from 'node:fs';import assert from 'node:assert/strict';import readline from 'node:readline'
const stage=process.env.TOOLHUB_SERVICE_QA_STAGE,cli=process.env.TOOLHUB_SERVICE_QA_CLI,python=process.env.TOOLHUB_SERVICE_QA_PYTHON;
if(!stage||!cli||!python||process.env.TOOLHUB_SERVICE_QA_ISOLATED!=='1')throw Error('Explicit private QA stage, CLI, Python and TOOLHUB_SERVICE_QA_ISOLATED=1 required; never use a production WebView');

fs.mkdirSync(stage+'/fixtures',{recursive:true})
fs.writeFileSync(stage+'/fixtures/server.py',`import socket,time,json,sys,pathlib,subprocess,os\nroot=pathlib.Path(sys.argv[1]);child=len(sys.argv)>2\ns=socket.socket(socket.AF_INET,socket.SOCK_DGRAM if child else socket.SOCK_STREAM);s.bind(('127.0.0.1',0))\nif not child:s.listen()\n(root/('child.json' if child else 'root.json')).write_text(json.dumps({'pid':os.getpid(),'port':s.getsockname()[1]}))\nif not child:subprocess.Popen([sys.executable,__file__,str(root),'child'])\ntime.sleep(180)\n`)
fs.writeFileSync(stage+'/fixtures/single.py',`import socket,time,json,sys,pathlib,os\ns=socket.socket();s.bind(('127.0.0.1',0));s.listen();pathlib.Path(sys.argv[1]).write_text(json.dumps({'pid':os.getpid(),'port':s.getsockname()[1]}));time.sleep(180)\n`)
const wait=async(f,message)=>{for(let i=0;i<100;i++){if(await f())return;await new Promise(r=>setTimeout(r,100))}throw Error(message)}
let b;for(let i=0;i<30;i++){try{b=await chromium.connectOverCDP(process.env.TOOLHUB_SERVICE_QA_CDP||'http://127.0.0.1:19336');break}catch{await new Promise(r=>setTimeout(r,500))}}assert(b)
const p=b.contexts()[0].pages()[0],rpc=(method,params={})=>p.evaluate(({method,params})=>window.__TAURI_INTERNALS__.invoke('rpc',{method,params}),{method,params})
const owned=[],ids=[];let managed;let mcp
try{
 await p.waitForFunction(()=>window.__TAURI_INTERNALS__);await p.getByRole('navigation',{name:'二级导航',exact:true}).getByRole('button',{name:'服务监控',exact:true}).waitFor()
 const version=await p.evaluate(()=>window.__TAURI_INTERNALS__.invoke('app_versions'));assert.equal(version.app,'0.3.0-beta.13')
 await rpc('service.proxy_configure',{ports:[]});for(const row of (await rpc('service.list',{limit:50})).items){if(row.managed&&row.status==='running')await rpc('service.stop',{id:row.id,force:true});await rpc('service.remove',{id:row.id})}for(const entry of await rpc('program.list'))await rpc('program.remove',{id:entry.id})
 await p.getByRole('navigation',{name:'二级导航',exact:true}).getByRole('button',{name:'服务监控',exact:true}).click();await p.getByRole('heading',{name:'服务监控',exact:true}).waitFor();await p.getByRole('button',{name:'刷新',exact:true}).click();await p.getByText('后台运行，一目了然',{exact:true}).waitFor()
 // Standard user selection/save establishes the launch entry; no database writes.
 const project=stage+'/fixtures/中文 project [1]';fs.mkdirSync(project,{recursive:true});for(const name of ['root.json','child.json'])fs.rmSync(project+'/'+name,{force:true})
 let selected=await rpc('program.select',{path:project,kind:'command'});selected[0].name='开发预览 · 子进程测试';selected[0].command=`"${python}" "${stage}/fixtures/server.py" "${project}"`;await rpc('program.save',{items:selected});const entries=await rpc('program.list');const program=entries.find(e=>e.name===selected[0].name)
 await p.getByRole('button',{name:'启动服务',exact:true}).click();await p.getByRole('button',{name:/开发预览 · 子进程测试/}).click();await p.getByRole('button',{name:'启动并监控',exact:true}).click()
 await wait(()=>fs.existsSync(project+'/child.json'),'managed shell child did not start');managed=(await rpc('service.list')).items[0]
 await wait(async()=>{managed=(await rpc('service.list',{id:managed.id})).items[0];return managed.endpoints.length>=2},'managed endpoints absent')
 assert(managed.process_count>=3,'helper/CMD/Python/child inherited job');assert.match(managed.application_path,/python\.exe$/i)
 // Fresh MCP gateway: metadata registration is available, stop is not.
 mcp=spawn(cli,['mcp','serve'],{env:{...process.env,TOOLHUB_REGISTRY:stage+'/qa-registry.sqlite'},windowsHide:true,stdio:['pipe','pipe','pipe']});const pending=new Map();let next=1
 readline.createInterface({input:mcp.stdout}).on('line',l=>{const r=JSON.parse(l);pending.get(r.id)?.(r);pending.delete(r.id)})
 const request=(method,params,notification=false)=>{const id=next++;const response=notification?Promise.resolve():new Promise((resolve,reject)=>{const timeout=setTimeout(()=>reject(Error('MCP timeout '+method)),15000);pending.set(id,r=>{clearTimeout(timeout);resolve(r)})});mcp.stdin.write(JSON.stringify({jsonrpc:'2.0',...(notification?{}:{id}),method,params})+'\n');return response}
 const tool=async(name,args)=>{const r=await request('tools/call',{name,arguments:args});assert(!r.error,JSON.stringify(r));assert(!r.result.isError,JSON.stringify(r));return r.result.structuredContent.result}
 await request('initialize',{protocolVersion:'2025-11-25',capabilities:{},clientInfo:{name:'ToolHub self-test',version:'1'}});await request('notifications/initialized',{},true);assert.equal((await request('tools/list',{})).result.tools.length,19)
 for(let i=0;i<6;i++){const file=stage+`/fixtures/external-${i}.json`;fs.rmSync(file,{force:true});const child=spawn(python,[stage+'/fixtures/single.py',file],{windowsHide:true,stdio:'ignore'});owned.push(child);await wait(()=>fs.existsSync(file),'external fixture absent');const manifest=JSON.parse(fs.readFileSync(file));ids.push((await tool('register_service',{name:['文档预览','本地模型演示','接口调试','构建预览','任务演示','临时 API'][i],pid:manifest.pid,project_dir:project,kind:i===1?'model':'web'})).id)}
 assert.equal((await tool('list_services',{limit:50})).total,6)
 // The proxy observation uses an owned ephemeral port; real Clash/nano remain untouched.
 const ownedProxy=JSON.parse(fs.readFileSync(stage+'/fixtures/external-0.json'));await rpc('service.proxy_configure',{ports:[{name:'代理演示（隔离测试）',port:ownedProxy.port}]});const protectedRow=(await rpc('service.list',{id:ids[0]})).items[0];assert.equal(protectedRow.persistent_proxy,true)
 await assert.rejects(()=>rpc('service.stop',{id:ids[0],force:true}),/代理/)
 await p.getByRole('button',{name:'刷新',exact:true}).click();await wait(async()=>await p.locator('.service-row').count()===6,'six-row pagination absent');await p.getByRole('button',{name:'下一页',exact:true}).click();await wait(async()=>await p.locator('.service-row').count()===1,'second page absent');await p.getByRole('button',{name:'上一页',exact:true}).click();await wait(async()=>await p.locator('.service-row').count()===6,'first page did not finish refreshing')
 const proxyRow=p.locator('.service-row').filter({has:p.getByRole('heading',{name:'文档预览',exact:true})});assert.equal(await proxyRow.getByRole('button',{name:'停止',exact:true}).count(),0);assert.equal(await proxyRow.getByRole('button',{name:'重启',exact:true}).count(),0)
 await wait(async()=>await p.locator('.service-row .tool-icon img').count()>0,'native icon absent')
 await p.screenshot({path:stage+'/services-desktop.png'})
 for(const [width,height] of [[1440,900],[1100,760],[900,640]]){await p.setViewportSize({width,height});const geometry=await p.locator('.services-page').evaluate(n=>({horizontal:n.scrollWidth>n.clientWidth+2,nested:[...n.querySelectorAll('*')].filter(x=>{const s=getComputedStyle(x);return ['auto','scroll'].includes(s.overflowY)&&x.scrollHeight>x.clientHeight+2}).length}));assert.deepEqual(geometry,{horizontal:false,nested:0})}
 await p.setViewportSize({width:1440,height:900})
 // Stop/restart via the actual visible buttons, including confirmation.
 await p.getByRole('button',{name:'下一页',exact:true}).click();const managedCard=p.locator('.service-row').filter({has:p.getByRole('heading',{name:'开发预览 · 子进程测试',exact:true})});await managedCard.waitFor();await managedCard.getByRole('button',{name:'停止',exact:true}).click();await managedCard.getByRole('button',{name:'强制结束',exact:true}).waitFor();p.once('dialog',d=>d.accept());await managedCard.getByRole('button',{name:'强制结束',exact:true}).click();await wait(async()=>(await rpc('service.list',{id:managed.id})).items[0].status==='stopped','job still alive')
 await managedCard.getByRole('button',{name:'重启',exact:true}).click();await wait(async()=>{const r=(await rpc('service.list')).items.find(r=>r.program_id===program.id&&r.status==='running');return !!r},'restart absent');const replacement=(await rpc('service.list')).items.find(r=>r.program_id===program.id&&r.status==='running');assert.equal(replacement.id,managed.id,'restart must preserve the logical service card');await rpc('service.stop',{id:replacement.id,force:true})
 fs.writeFileSync(stage+'/ui-results.json',JSON.stringify({passed:true,version:version.app,mcpTools:19,mcpRegistration:true,managedShellTree:true,nativeIcons:true,proxyControlDenied:true,pagination6:true,responsive:[1440,1100,900],nestedScrollers:0,visibleStopAndRestart:true},null,2));console.log('PASS native service UI, MCP registration, shell children, icons, proxy protection, pagination and lifecycle')
}finally{
 // Cleanup only private registry records and exact processes spawned in this script.
 try{for(const row of (await rpc('service.list',{limit:50})).items){if(row.managed&&row.status==='running')await rpc('service.stop',{id:row.id,force:true})}}catch{}
 for(const child of owned){child.kill()}
 mcp?.stdin.end();setTimeout(()=>mcp?.kill(),1000).unref();await b.close()
}
