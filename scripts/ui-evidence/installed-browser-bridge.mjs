/** Serve built UI with a native RPC relay, only against our disposable desktop. */
import {chromium} from 'playwright'
import fs from 'node:fs'
import {fileURLToPath} from 'node:url'
import path from 'node:path'
import http from 'node:http'
import assert from 'node:assert/strict'
import {spawn} from 'node:child_process'
const here=path.dirname(fileURLToPath(import.meta.url)),root=path.resolve(here,'../..')
const endpoint=process.env.TOOLHUB_INSTALLED_NATIVE_CDP,registry=process.env.TOOLHUB_INSTALLED_REGISTRY,output=process.env.TOOLHUB_INSTALLED_OUTPUT
if(!endpoint||!registry||!output||process.env.TOOLHUB_INSTALLED_FIXTURE!=='yes')throw Error('Explicit isolated native CDP, registry, output and fixture=yes required')
const native=await chromium.connectOverCDP(endpoint),nativePage=native.contexts()[0].pages()[0]
await nativePage.waitForFunction(()=>!!window.__TAURI_INTERNALS__ && document.querySelectorAll('.primary-link').length===4)
const rpc=(method,params={})=>nativePage.evaluate(({method,params})=>window.__TAURI_INTERNALS__.invoke('rpc',{method,params}),{method,params})
const actual=(await rpc('status')).registry_path
assert.equal(path.resolve(actual).toLowerCase(),path.resolve(registry).toLowerCase())
assert(!/[\\/]\.toolhub[\\/]registry\.sqlite$/i.test(actual),'Refusing default production registry')
const dist=path.join(root,'apps/desktop/ui/dist')
const server=http.createServer(async(req,res)=>{
 try{
  if(req.url==='/invoke'){
   if(req.headers.origin!=='http://127.0.0.1:19336'){res.writeHead(403).end();return}
   let body='';for await(const c of req)body+=c
   const {cmd,args}=JSON.parse(body)
   if(['extension_pick','extension_export','extension_repository','clipboard_write'].includes(cmd))throw Error('Native dialog/external side effect requires explicit test interception')
   const value=await nativePage.evaluate(({cmd,args})=>window.__TAURI_INTERNALS__.invoke(cmd,args),{cmd,args})
   res.setHeader('Content-Type','application/json');res.end(JSON.stringify(value??null));return
  }
  const target=path.resolve(dist,'.'+(new URL(req.url,'http://localhost')).pathname.replace(/^\/$/,'/index.html'))
  if(!target.startsWith(path.resolve(dist)+path.sep)||!fs.existsSync(target)){res.writeHead(404).end();return}
  res.setHeader('Content-Type',({'.html':'text/html','.js':'text/javascript','.css':'text/css','.svg':'image/svg+xml','.png':'image/png'})[path.extname(target)]||'application/octet-stream')
  res.end(fs.readFileSync(target))
 }catch(e){res.writeHead(400,{'Content-Type':'application/json'});res.end(JSON.stringify({error:String(e)}))}
})
await new Promise(r=>server.listen(19336,'127.0.0.1',r))
let browser
try{
 browser=await chromium.launch({headless:true,channel:'msedge',args:['--remote-debugging-port=19335']})
 const context=await browser.newContext({viewport:{width:1180,height:780}})
 await context.addInitScript(()=>{
  window.__TAURI_INTERNALS__={invoke:async(cmd,args={})=>{const response=await fetch('/invoke',{method:'POST',body:JSON.stringify({cmd,args})});const value=await response.json();if(!response.ok)throw Error(value.error);return value}}
  localStorage.setItem('th.prefs',JSON.stringify({autoScan:false}))
 })
 const page=await context.newPage();await page.goto('http://127.0.0.1:19336/');await page.waitForFunction(()=>document.querySelectorAll('.primary-link').length===4)
 const child=spawn(process.execPath,[path.join(here,'verify-installed-manager.mjs')],{stdio:'inherit',env:{...process.env,TOOLHUB_INSTALLED_CDP:'http://127.0.0.1:19335',TOOLHUB_INSTALLED_REGISTRY:registry,TOOLHUB_INSTALLED_FIXTURE:'yes',TOOLHUB_INSTALLED_OUTPUT:output}})
 const code=await new Promise(r=>child.on('exit',r));if(code!==0)process.exitCode=code??1
}finally{await browser?.close();server.close();await native.close()}
