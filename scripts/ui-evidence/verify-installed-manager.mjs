/** Installed-manager browser acceptance with real native RPC. Requires a fresh disposable registry.
 * Run through installed-browser-bridge.mjs; the native Tauri invoke is immutable.
 * Optional file picker/export are intercepted at the native dialog boundary only;
 * preview/install/state/rollback/remove all use the real daemon and controller.
 */
import {chromium} from 'playwright'
import fs from 'node:fs'
import {fileURLToPath} from 'node:url'
import path from 'node:path'
import assert from 'node:assert/strict'
const cdp=process.env.TOOLHUB_INSTALLED_CDP,registry=process.env.TOOLHUB_INSTALLED_REGISTRY,output=process.env.TOOLHUB_INSTALLED_OUTPUT
if(!cdp||!registry||!output||process.env.TOOLHUB_INSTALLED_FIXTURE!=='yes')throw Error('Explicit disposable CDP, registry, output and fixture=yes required')
fs.mkdirSync(output,{recursive:true})
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../..')
const browser=await chromium.connectOverCDP(cdp),page=browser.contexts()[0].pages()[0];page.setDefaultTimeout(12000)
const errors=[],results=[];page.on('pageerror',error=>errors.push(String(error)))
const pass=name=>{results.push(name);console.log('PASS',name)}
const rpc=(method,params={})=>page.evaluate(({method,params})=>window.__TAURI_INTERNALS__.invoke('rpc',{method,params}),{method,params})
const primary=()=>page.getByRole('navigation',{name:'一级导航',exact:true})
const go=async name=>page.getByRole('navigation',{name:'二级导航',exact:true}).getByRole('button',{name,exact:true}).click()
const search=()=>page.getByRole('textbox',{name:'搜索扩展',exact:true})
const type=async name=>page.getByRole('group',{name:'扩展类型',exact:true}).getByRole('button',{name:new RegExp('^'+name+'\\s*\\d+$')}).click()
const detail=()=>page.getByRole('complementary',{name:'扩展详情',exact:true})
const firstName=()=>page.locator('.market-package h3').first().innerText()
const ready=async()=>{await page.getByRole('tab',{name:'概览',exact:true}).waitFor();await page.waitForFunction(()=>document.querySelectorAll('.market-package[aria-pressed="true"]').length===1 && !document.querySelector('.extensions-manager>.topbar button')?.disabled && !!document.querySelector('.installed-detail-tabs'))}
const install=async pkg=>{const v=await rpc('extension.preview',{package:pkg});await rpc('extension.install',{package:pkg,digest:v.digest,source:'local'})}
const example=name=>JSON.parse(fs.readFileSync(path.join(root,`extensions/examples/${name}.toolhub-extension.json`),'utf8'))
async function geometry(label){
 await page.waitForTimeout(180)
 const data=await page.evaluate(()=>{
  const content=document.querySelector('.content'),manager=document.querySelector('.extensions-manager'),list=document.querySelector('.installed-list-space'),pager=document.querySelector('.installed-pagination'),detail=document.querySelector('.market-detail'),rows=[...document.querySelectorAll('.market-package')]
  const rect=e=>e.getBoundingClientRect().toJSON()
  return {viewport:[innerWidth,innerHeight],body:[document.body.scrollWidth,document.body.scrollHeight],content:[content.clientHeight,content.scrollHeight],manager:[manager.clientHeight,manager.scrollHeight],list:rect(list),pager:rect(pager),detail:rect(detail),rows:rows.map(rect)}
 })
 assert(data.content[1]<=data.content[0]+1,`${label}: outer overflow ${JSON.stringify(data)}`)
 assert(data.manager[1]<=data.manager[0]+1,`${label}: manager overflow ${JSON.stringify(data)}`)
 assert(data.body[0]<=data.viewport[0]+1,`${label}: horizontal page overflow`)
 if(data.list.height){assert(data.pager.bottom<=data.viewport[1]+1,`${label}: pager clipped`);for(const row of data.rows)assert(row.bottom<=data.list.bottom+1,`${label}: row clipped ${JSON.stringify(data)}`)}
 return data
}
try{
 await page.waitForFunction(()=>!!window.__TAURI_INTERNALS__ && document.querySelectorAll('.primary-link').length===4)
 assert(await page.evaluate(()=>Object.getOwnPropertyDescriptor(window.__TAURI_INTERNALS__,'invoke')?.writable), 'Use installed-browser-bridge.mjs; never intercept an immutable native invoke')
 const status=await rpc('status');assert.equal(path.resolve(status.registry_path).toLowerCase(),path.resolve(registry).toLowerCase());assert(!/[\\/]\.toolhub[\\/]registry\.sqlite$/i.test(status.registry_path))
 const initial=await rpc('extension.list');assert.equal(initial.length,7);assert(initial.every(i=>i.package.id.startsWith('toolhub.builtin.')))
 await page.evaluate(()=>localStorage.setItem('th.prefs',JSON.stringify({autoScan:false})))
 for(const name of ['ffmpeg','graphite','workspace','blender'])await install(example(name))
 const base=example('workspace')
 for(let i=0;i<12;i++){
  const id=`test.installed${String(i).padStart(2,'0')}`
  await install({...base,id,name:`分页验收 ${String(i).padStart(2,'0')}`,description:'临时隔离验收包，用于验证分页、筛选和管理操作。',platforms:i%2?['windows']:['windows','macos'],views:[{...base.views[0],id:id+'.page',label:'验收页 '+i}]})
 }
 await primary().getByRole('button',{name:'扩展中心',exact:true}).click();await go('已安装');await page.getByRole('button',{name:'刷新扩展',exact:true}).click();await ready()
 assert.equal(await page.locator('.extensions-manager select').count(),0)
 assert.equal(await page.locator('.market-intro').count(),0)
 assert.equal(await page.getByRole('group',{name:'扩展类型',exact:true}).getByRole('button').count(),7)
 const seen=new Set()
 do{
  for(const name of await page.locator('.market-package h3').allTextContents())seen.add(name)
  await geometry('page '+seen.size)
  if(await page.getByRole('button',{name:'下一页扩展',exact:true}).isDisabled())break
  await page.getByRole('button',{name:'下一页扩展',exact:true}).click();await ready()
 }while(true)
 assert.equal(seen.size,23);assert(await page.getByRole('button',{name:'下一页扩展',exact:true}).isDisabled())
 pass('23 installed packages are paginated without outer scrolling or clipped rows')
 await search().fill('分页验收 11');await ready();assert.equal(await page.locator('.market-package').count(),1);assert.equal(await firstName(),'分页验收 11')
 assert.match(await page.getByRole('navigation',{name:'扩展分页',exact:true}).innerText(),/1 \/ 1 页/)
 await search().fill('nothing matches');await page.getByRole('button',{name:'清除筛选',exact:true}).waitFor();assert.equal(await page.locator('.market-package').count(),0)
 await page.getByRole('button',{name:'清除筛选',exact:true}).click();await ready()
 await type('CLI 面板');await ready();assert.equal(await page.locator('.market-package').count(),1);assert.match(await firstName(),/FFmpeg/)
 await type('主题');await ready();assert.match(await firstName(),/石墨/)
 await type('识别规则');await ready();assert.match(await firstName(),/Blender/)
 await page.getByRole('tab',{name:'识别规则',exact:true}).click();await detail().getByText('Blender',{exact:true}).waitFor()
 await page.getByRole('tab',{name:'Skill',exact:true}).click();await detail().locator('summary').filter({hasText:'Blender 场景渲染'}).click();await detail().locator('pre').waitFor();await geometry('expanded skill')
 await type('通用 Skill');await page.getByRole('button',{name:'清除筛选',exact:true}).waitFor()
 await type('交互页面');await ready();await page.getByRole('group',{name:'扩展平台',exact:true}).getByRole('button',{name:'Windows',exact:true}).click();await ready()
 assert.equal(await page.locator('.installed-list-heading').innerText(),'扩展列表\n13 个结果')
 await page.getByRole('group',{name:'扩展平台',exact:true}).getByRole('button',{name:'macOS',exact:true}).click();await ready()
 assert.equal(await page.locator('.installed-list-heading').innerText(),'扩展列表\n7 个结果')
 pass('type/platform buttons, counts, search reset, empty results, rule/Skill detail tabs')
 await type('全部');await page.getByRole('group',{name:'扩展平台',exact:true}).getByRole('button',{name:'全部平台',exact:true}).click();await search().fill('分页验收 11');await ready()
 await detail().getByRole('button',{name:'停用扩展',exact:true}).click();await detail().getByRole('button',{name:'启用扩展',exact:true}).waitFor();assert.equal((await rpc('extension.list')).find(i=>i.package.id==='test.installed11').enabled,false)
 await detail().getByRole('button',{name:'启用扩展',exact:true}).click();await ready()
 await detail().getByRole('button',{name:'编辑扩展 JSON',exact:true}).click();const editor=page.getByRole('dialog',{name:'编辑本地扩展',exact:true});await editor.waitFor()
 const draft=JSON.parse(await editor.getByRole('textbox').inputValue());assert.equal(draft.version,'1.0.1')
 await editor.getByRole('textbox').fill('{ broken');await editor.getByRole('button',{name:'校验修改并预览',exact:true}).click();await editor.getByRole('alert').waitFor();assert.equal((await rpc('extension.list')).find(i=>i.package.id===draft.id).package.version,'1.0.0')
 await editor.getByRole('textbox').fill(JSON.stringify(draft));await editor.getByRole('button',{name:'校验修改并预览',exact:true}).click();await editor.waitFor({state:'hidden'});await detail().getByRole('button',{name:'确认更新扩展',exact:true}).click();await ready()
 assert.equal((await rpc('extension.list')).find(i=>i.package.id===draft.id).package.version,'1.0.1')
 await detail().getByRole('button',{name:'回滚上一版',exact:true}).click();await ready();assert.equal((await rpc('extension.list')).find(i=>i.package.id===draft.id).package.version,'1.0.0')
 await detail().getByRole('button',{name:'编辑扩展 JSON',exact:true}).click();await editor.waitFor();await page.keyboard.press('Escape');await editor.waitFor({state:'hidden'});assert.equal(await page.evaluate(()=>document.activeElement?.textContent),'编辑扩展 JSON')
 pass('native enable/disable, bounded JSON editor, validation, explicit update, rollback and Escape focus restore')
 // Supply a fixture through the file-dialog boundary, retaining all native validation/mutations.
 const imported=example('graphite');imported.id='test.imported-theme';imported.name='导入测试主题'
 await page.evaluate(pkg=>{window.__qaOriginalInvoke=window.__TAURI_INTERNALS__.invoke;window.__TAURI_INTERNALS__.invoke=(cmd,args,opts)=>{if(cmd==='extension_pick')return Promise.resolve(pkg);if(cmd==='extension_export'){window.__qaExport=args.package;return Promise.resolve('fixture-export.json')}return window.__qaOriginalInvoke(cmd,args,opts)}},imported)
 await type('CLI 面板');await ready();await page.getByRole('button',{name:'导入扩展包',exact:true}).click();await detail().getByRole('heading',{name:'导入测试主题',exact:true}).waitFor()
 assert.equal(await page.getByRole('group',{name:'扩展类型',exact:true}).getByRole('button',{name:/^全部\s*\d+$/}).getAttribute('aria-pressed'),'true')
 await detail().getByRole('button',{name:'确认安装扩展',exact:true}).click();await ready();await detail().getByRole('button',{name:'导出 JSON',exact:true}).click();assert.equal(await page.evaluate(()=>window.__qaExport.id),imported.id)
 await detail().getByRole('button',{name:'卸载扩展',exact:true}).click();const confirm=page.getByRole('dialog',{name:/^卸载/});await confirm.waitFor();await page.keyboard.press('Escape');await confirm.waitFor({state:'hidden'});assert((await rpc('extension.list')).some(i=>i.package.id===imported.id))
 await detail().getByRole('button',{name:'卸载扩展',exact:true}).click();await confirm.getByRole('button',{name:'确认卸载扩展',exact:true}).click();await confirm.waitFor({state:'hidden'});await ready();assert(!(await rpc('extension.list')).some(i=>i.package.id===imported.id));assert.equal(await page.locator('.market-package[aria-pressed=true]').count(),1)
 await page.evaluate(()=>{window.__TAURI_INTERNALS__.invoke=window.__qaOriginalInvoke;delete window.__qaOriginalInvoke;delete window.__qaExport})
 pass('import clears incompatible filters; export contract; uninstall cancel/confirm and last-page selection clamp')
 const geometryResults=[]
 for(const [width,height] of [[1180,780],[1440,900],[960,640],[800,600],[600,720]]){
  await page.setViewportSize({width,height});if(width<760 && await page.locator('.secondary-panel').isVisible())await page.locator('.workspace-bar').getByRole('button',{name:'收起二级导航',exact:true}).click();const back=detail().getByRole('button',{name:'返回列表',exact:false});if(await back.isVisible())await back.click();geometryResults.push(await geometry(`${width}x${height}`));await page.screenshot({path:path.join(output,`installed-${width}x${height}.png`)})
 }
 await page.setViewportSize({width:600,height:720});await page.locator('.market-package').first().click();await detail().getByRole('button',{name:'返回列表',exact:false}).waitFor();assert.equal(await page.locator('.market-results').isVisible(),false)
 await geometry('narrow detail');await detail().getByRole('button',{name:'返回列表',exact:false}).click();assert.equal(await detail().isVisible(),false);assert(await page.locator('.market-results').isVisible())
 await page.setViewportSize({width:1180,height:780});await page.getByRole('button',{name:'上一页扩展',exact:true}).click();await ready();await page.screenshot({path:path.join(output,'installed-final.png')})
 pass('responsive page capacity and narrow list/detail switch; no stacked full-page detail')
 if(await page.locator('.secondary-panel').getAttribute('hidden')!==null)await page.locator('.workspace-bar').getByRole('button',{name:'展开二级导航',exact:true}).click()
 await go('扩展市场');await page.locator('.market-intro').waitFor();await page.waitForFunction(()=>document.querySelectorAll('.market-package').length>23);assert(await page.locator('.market-package').count()>23);await page.getByRole('combobox',{name:'扩展类型',exact:true}).selectOption('cli');assert.equal(await page.locator('.market-package').count(),1)
 await go('主题与布局');await page.getByRole('combobox',{name:'导航布局',exact:true}).waitFor();await go('已安装');await ready()
 const final=await rpc('extension.list');for(const original of initial){const now=final.find(i=>i.package.id===original.package.id);assert.equal(now.digest,original.digest);assert.equal(now.enabled,original.enabled)}
 assert.deepEqual(errors,[]);pass('catalog/appearance remain usable; official module digests and enabled flags unchanged; no page errors')
 fs.writeFileSync(path.join(output,'results.json'),JSON.stringify({passed:true,results,geometry:geometryResults,errors},null,2))
}finally{
 await page.evaluate(()=>{if(window.__qaOriginalInvoke)window.__TAURI_INTERNALS__.invoke=window.__qaOriginalInvoke}).catch(()=>{})
 await browser.close()
}





