/** Read-only native WebView geometry/keyboard smoke; explicit disposable registry only. */
import {chromium} from 'playwright'
import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
const endpoint=process.env.TOOLHUB_INSTALLED_NATIVE_CDP,registry=process.env.TOOLHUB_INSTALLED_REGISTRY,output=process.env.TOOLHUB_INSTALLED_OUTPUT
if(!endpoint||!registry||!output||process.env.TOOLHUB_INSTALLED_FIXTURE!=='yes')throw Error('Explicit isolated native CDP, registry, output and fixture=yes required')
const browser=await chromium.connectOverCDP(endpoint),page=browser.contexts()[0].pages()[0];page.setDefaultTimeout(12000)
const errors=[];page.on('pageerror',e=>errors.push(String(e)))
try{
 await page.waitForFunction(()=>!!window.__TAURI_INTERNALS__ && document.querySelectorAll('.primary-link').length===4)
 const rpc=(method,params={})=>page.evaluate(({method,params})=>window.__TAURI_INTERNALS__.invoke('rpc',{method,params}),{method,params})
 const actual=(await rpc('status')).registry_path;assert.equal(path.resolve(actual).toLowerCase(),path.resolve(registry).toLowerCase());assert(!/[\\/]\.toolhub[\\/]registry\.sqlite$/i.test(actual))
 const before=(await rpc('extension.list')).map(i=>[i.package.id,i.digest,i.enabled])
 await page.getByRole('navigation',{name:'一级导航',exact:true}).getByRole('button',{name:'扩展中心',exact:true}).click()
 await page.getByRole('navigation',{name:'二级导航',exact:true}).getByRole('button',{name:'已安装',exact:true}).click()
 await page.getByRole('button',{name:'刷新扩展',exact:true}).click()
 await page.waitForFunction(()=>!document.querySelector('.extensions-manager>.topbar button').disabled && !!document.querySelector('.installed-detail-tabs'))
 const overview=page.getByRole('tab',{name:'概览',exact:true});await overview.focus();await page.keyboard.press('End');assert.equal(await page.getByRole('tab',{name:'Skill',exact:true}).getAttribute('aria-selected'),'true');await page.keyboard.press('Home');assert.equal(await overview.getAttribute('aria-selected'),'true')
 const next=page.getByRole('button',{name:'下一页扩展',exact:true});if(await next.isEnabled())await next.click()
 const geometry=await page.evaluate(()=>{
  const c=document.querySelector('.content'),m=document.querySelector('.extensions-manager'),list=document.querySelector('.installed-list-space').getBoundingClientRect(),pager=document.querySelector('.installed-pagination').getBoundingClientRect()
  return {height:innerHeight,width:innerWidth,outer:[c.clientHeight,c.scrollHeight],manager:[m.clientHeight,m.scrollHeight],listBottom:list.bottom,pagerBottom:pager.bottom,rowBottoms:[...document.querySelectorAll('.market-package')].map(e=>e.getBoundingClientRect().bottom)}
 })
 assert(geometry.outer[1]<=geometry.outer[0]+1);assert(geometry.manager[1]<=geometry.manager[0]+1);assert(geometry.pagerBottom<=geometry.height+1);assert(geometry.rowBottoms.every(y=>y<=geometry.listBottom+1));assert.equal(await page.locator('.extensions-manager select').count(),0)
 fs.mkdirSync(output,{recursive:true});await page.screenshot({path:path.join(output,'native-installed.png')})
 assert.deepEqual((await rpc('extension.list')).map(i=>[i.package.id,i.digest,i.enabled]),before);assert.deepEqual(errors,[])
 fs.writeFileSync(path.join(output,'native-results.json'),JSON.stringify({passed:true,geometry,keyboardTabs:true,statesUnchanged:true,errors},null,2))
 console.log('PASS native WebView paging, bounded layout, filter buttons and keyboard tabs; registry unchanged')
}finally{await browser.close()}
