/** Native updater smoke. Explicit disposable registry/CDP/output required. */
import { chromium } from 'playwright'
import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
const endpoint=process.env.TOOLHUB_UPDATE_QA_CDP,registry=process.env.TOOLHUB_UPDATE_QA_REGISTRY,output=process.env.TOOLHUB_UPDATE_QA_OUTPUT
if(!endpoint||!registry||!output||process.env.TOOLHUB_UPDATE_QA_FIXTURE!=='yes')throw Error('Explicit isolated fixture required')
const browser=await chromium.connectOverCDP(endpoint),page=browser.contexts()[0].pages()[0]
page.setDefaultTimeout(12000)
const errors=[];page.on('pageerror',e=>errors.push(String(e)))
const native=(command,params={})=>page.evaluate(({command,params})=>window.__TAURI_INTERNALS__.invoke(command,params),{command,params})
const results=[]
try{
 await page.waitForFunction(()=>!!window.__TAURI_INTERNALS__ && document.querySelectorAll('.primary-link').length===4)
 const actual=(await native('rpc',{method:'status',params:{}})).registry_path
 assert.equal(path.resolve(actual).toLowerCase(),path.resolve(registry).toLowerCase())
 assert(!/[\\/]\.toolhub[\\/]registry\.sqlite$/i.test(actual),'Production registry is forbidden')
 const initial=await native('updater_status');assert.equal(initial.current_version,'0.3.0-beta.14');assert.equal(initial.auto_check,true)
 await page.getByRole('navigation',{name:'一级导航',exact:true}).getByRole('button',{name:'设置',exact:true}).click()
 const card=page.getByRole('region',{name:'软件更新'})
 await card.getByRole('button',{name:'检查更新',exact:true}).waitFor()
 await card.getByRole('button',{name:'启动时检查更新',exact:true}).click()
 await page.waitForFunction(async()=>!(await window.__TAURI_INTERNALS__.invoke('updater_status')).auto_check)
 await card.getByRole('button',{name:'稳定版',exact:true}).click()
 await page.waitForFunction(async()=>!(await window.__TAURI_INTERNALS__.invoke('updater_status')).preview)
 await card.getByRole('button',{name:'预览版',exact:true}).click()
 await page.waitForFunction(async()=>(await window.__TAURI_INTERNALS__.invoke('updater_status')).preview)
 await card.getByRole('button',{name:'检查更新',exact:true}).click()
 let checked;const deadline=Date.now()+65000
 do{checked=await native('updater_status');if(['current','available','error'].includes(checked.phase))break;await new Promise(resolve=>setTimeout(resolve,150))}while(Date.now()<deadline)
 results.push({test:'real-public-release-check',phase:checked.phase,error:checked.error})
 assert(['current','available','error'].includes(checked.phase))
 assert(!checked.available_version||checked.available_version!==initial.current_version)
 await assert.rejects(native('updater_install'),/隔离实例/)
 if(!checked.available_version)await assert.rejects(native('updater_download'),/先检查更新/)
 await card.getByRole('button',{name:'启动时检查更新',exact:true}).click()
 await page.waitForFunction(async()=>(await window.__TAURI_INTERNALS__.invoke('updater_status')).auto_check)
 await page.reload()
 await page.getByRole('region',{name:'软件更新'}).waitFor()
 const saved=await native('updater_status');assert.equal(saved.auto_check,true);assert.equal(saved.preview,true)
 const geometry=await page.evaluate(()=>{const e=document.querySelector('.update-setting'),r=e.getBoundingClientRect();return {width:innerWidth,bodyWidth:document.body.scrollWidth,cardWidth:r.width,cardScroll:e.scrollWidth,cardClient:e.clientWidth}})
 assert(geometry.bodyWidth<=geometry.width+1);assert(geometry.cardScroll<=geometry.cardClient+1)
 fs.mkdirSync(output,{recursive:true});await page.getByRole('region',{name:'软件更新'}).scrollIntoViewIfNeeded()
 await page.screenshot({path:path.join(output,'software-updates.png')})
 assert.deepEqual(errors,[])
 fs.writeFileSync(path.join(output,'native-updates.json'),JSON.stringify({passed:true,results,preferencesPersist:true,isolatedInstallDenied:true,geometry,errors},null,2))
 console.log(JSON.stringify({passed:true,releaseCheck:checked.phase,preferencesPersist:true,isolatedInstallDenied:true}))
}finally{await browser.close()}
