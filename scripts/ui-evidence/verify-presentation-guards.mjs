import fs from 'node:fs'
import path from 'node:path'
import assert from 'node:assert/strict'
import {chromium} from 'playwright'
if(!process.env.STUDIO_CDP||!process.env.STUDIO_REGISTRY)throw Error('Explicit isolated CDP/registry required')
const browser=await chromium.connectOverCDP(process.env.STUDIO_CDP),page=browser.contexts()[0].pages()[0];page.setDefaultTimeout(10000)
const rpc=(method,params={})=>page.evaluate(({method,params})=>window.__TAURI_INTERNALS__.invoke('rpc',{method,params}),{method,params})
const status=await rpc('status');assert.equal(path.resolve(status.registry_path).toLowerCase(),path.resolve(process.env.STUDIO_REGISTRY).toLowerCase());assert(!/[\\/]\.toolhub[\\/]registry\.sqlite$/i.test(status.registry_path))
const frame=page.frameLocator('iframe[title="设计台"]'),expanded=page.locator('.extension-frame-surface.is-presenting'),close=async()=>{if(await expanded.count())await page.locator('.extension-presentation-bar button').click()}
try{
 await close();await frame.locator('button[data-step="2"]').click()
 await page.evaluate(()=>{window.__qaCapture=e=>{if(e.source===document.querySelector('iframe').contentWindow&&e.data?.channel==='toolhub.sdk')window.__qaToken=e.data.token};window.addEventListener('message',window.__qaCapture)})
 await frame.locator('body').evaluate(()=>toolhub.storage.get())
 await page.evaluate(()=>window.postMessage({channel:'toolhub.sdk',token:window.__qaToken,id:999999,type:'presentation',expanded:true},'*'))
 await frame.locator('body').evaluate(()=>parent.postMessage({channel:'toolhub.sdk',token:'wrong-token',id:999998,type:'presentation',expanded:true},'*'))
 await page.waitForTimeout(200);assert.equal(await expanded.count(),0)
 await assert.rejects(()=>frame.locator('body').evaluate(()=>toolhub.presentation.set('true')),/Invalid presentation state/);assert.equal(await expanded.count(),0)
 console.log('PASS wrong source/token and non-boolean display requests cannot open presentation')
 // Deterministically force the old callback-churn race while refreshing host state.
 await page.evaluate(()=>{window.__qaInvoke=window.__TAURI_INTERNALS__.invoke;window.__TAURI_INTERNALS__.invoke=async function(command,params,options){if(command==='rpc'&&params?.method==='extension.list')await new Promise(resolve=>setTimeout(resolve,350));return window.__qaInvoke.call(this,command,params,options)}})
 const pending=frame.locator('body').evaluate(()=>toolhub.presentation.set(true));await page.waitForTimeout(40);await page.evaluate(()=>window.dispatchEvent(new Event('focus')));await pending;await expanded.waitFor();await close()
 await page.evaluate(()=>{window.__TAURI_INTERNALS__.invoke=window.__qaInvoke;delete window.__qaInvoke})
 for(let i=0;i<6;i++){await frame.locator('body').evaluate(()=>toolhub.presentation.set(true));await expanded.waitFor();await close()}
 console.log('PASS in-flight presentation survives host callback refresh; repeated enter/exit remains responsive')
 const row=(await rpc('extension.list')).find(e=>e.package.id==='community.design-studio')
 await rpc('extension.set_enabled',{id:row.package.id,expected_digest:row.digest,enabled:false})
 await assert.rejects(()=>frame.locator('body').evaluate(()=>toolhub.presentation.set(true)),/disabled, removed or updated/);assert.equal(await expanded.count(),0)
 await rpc('extension.set_enabled',{id:row.package.id,expected_digest:row.digest,enabled:true})
 await frame.locator('body').evaluate(()=>toolhub.presentation.set(true));await expanded.waitFor();await rpc('extension.set_enabled',{id:row.package.id,expected_digest:row.digest,enabled:false});await page.evaluate(()=>window.dispatchEvent(new Event('toolhub:extensions-changed')));await page.locator('iframe[title="设计台"]').waitFor({state:'detached'});assert.equal(await page.locator('html').evaluate(e=>e.classList.contains('extension-presenting')),false);assert.equal(await page.locator('.primary-rail').evaluate(e=>e.inert),false)
 await rpc('extension.set_enabled',{id:row.package.id,expected_digest:row.digest,enabled:true});await page.evaluate(()=>window.dispatchEvent(new Event('toolhub:extensions-changed')))
 console.log('PASS disabled extension cannot enter; revocation/unmount restores navigation and display state')
}finally{await close().catch(()=>{});await page.evaluate(()=>{if(window.__qaInvoke)window.__TAURI_INTERNALS__.invoke=window.__qaInvoke;if(window.__qaCapture)window.removeEventListener('message',window.__qaCapture);delete window.__qaInvoke;delete window.__qaCapture;delete window.__qaToken}).catch(()=>{});await browser.close()}
