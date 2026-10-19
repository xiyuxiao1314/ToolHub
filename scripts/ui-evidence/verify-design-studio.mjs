/** Native acceptance; only runs against the explicitly selected disposable registry. */
import {chromium} from 'playwright'
import fs from 'node:fs'
import path from 'node:path'
import assert from 'node:assert/strict'
import {fileURLToPath} from 'node:url'
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../..')
const out=process.env.STUDIO_OUTPUT||path.join(root,'output/playwright/design-studio')
fs.mkdirSync(out,{recursive:true})
if(!process.env.STUDIO_CDP||!process.env.STUDIO_REGISTRY)throw Error('Explicit isolated CDP/registry required')
const browser=await chromium.connectOverCDP(process.env.STUDIO_CDP)
const page=browser.contexts()[0].pages()[0];page.setDefaultTimeout(15000)
const results=[],errors=[],csp=[]
page.on('pageerror',error=>errors.push(String(error)))
page.on('console',message=>{if(/violates|refused to/i.test(message.text()))csp.push(message.text())})
const rpc=(method,params={})=>page.evaluate(({method,params})=>window.__TAURI_INTERNALS__.invoke('rpc',{method,params}),{method,params})
const pass=name=>{results.push({name,ok:true});console.log('PASS',name)}
let frame
async function studio(){
 const primary=page.getByRole('navigation',{name:'一级导航',exact:true})
 await primary.getByRole('button',{name:'扩展中心',exact:true}).click()
 await page.getByRole('navigation',{name:'二级导航',exact:true}).getByRole('button',{name:'扩展应用',exact:true}).click()
 await page.locator('.application-card').filter({has:page.getByRole('heading',{name:'设计台',exact:true})}).getByRole('button',{name:'打开应用'}).click()
 frame=page.frameLocator('iframe[title="设计台"]')
 await frame.locator('#design-studio').waitFor()
 await frame.getByText('本机预设已就绪',{exact:true}).waitFor()
}
const config=async()=>JSON.parse(await frame.locator('#output-text').inputValue())
async function preset(id){await frame.locator('#preset-select').selectOption('builtin:'+id);await frame.getByRole('button',{name:'应用预设',exact:true}).click()}
try{
 await page.waitForFunction(()=>window.__TAURI_INTERNALS__&&document.querySelectorAll('.primary-link').length===4)
 const status=await rpc('status');assert.equal(path.resolve(status.registry_path).toLowerCase(),path.resolve(process.env.STUDIO_REGISTRY).toLowerCase());assert(!/[\\/]\.toolhub[\\/]registry\.sqlite$/i.test(status.registry_path))
 await page.evaluate(()=>localStorage.setItem('th.prefs',JSON.stringify({autoScan:false})))
 const packageData=JSON.parse(fs.readFileSync(path.join(root,'extensions/examples/design-studio.toolhub-extension.json'),'utf8'))
 const existing=(await rpc('extension.list')).find(item=>item.package.id===packageData.id)
 if(!existing){const preview=await rpc('extension.preview',{package:packageData});assert(preview.compatible);await rpc('extension.install',{package:packageData,digest:preview.digest,source:'local'})}
 await page.evaluate(()=>window.dispatchEvent(new Event('toolhub:extensions-changed')))
 await page.setViewportSize({width:1700,height:1080});await studio()
 assert.equal(await page.getByRole('navigation',{name:'一级导航',exact:true}).getByRole('button').count(),4)
 await frame.locator('#storage-status').getByText('本机预设已就绪').waitFor()
 assert.equal(await frame.locator('.library-choice').count(),22)
 await frame.locator('#target-platform').selectOption('android');assert.equal(await frame.locator('#viewport').inputValue(),'mobile');await frame.locator('.p-status').waitFor();
 await frame.locator('#target-platform').selectOption('desktop');await frame.locator('.p-window').waitFor();await frame.locator('#target-platform').selectOption('web');await frame.locator('.p-browser').waitFor();
 assert.equal(await frame.locator('.direction-card').count(),12);await frame.locator('#collection-filter').selectOption('spatial');assert.equal(await frame.locator('.direction-card').count(),1);await frame.locator('#collection-filter').selectOption('all');await frame.locator('.direction-card').first().click();assert((await frame.locator('#direction-detail').textContent()).includes('Folia'));assert.equal(await frame.locator('#direction-detail').isVisible(),false);pass('three platform shells and curated direction filter/application')
 pass('real unprivileged extension loads under native CSP and stays in secondary navigation')
 for(const id of ['creator','metro','gallery','library','canvas','kanban','portfolio','mobile','cinema','swiss','photo','product']){await preset(id);const current=await config();assert(current.schema==='toolhub.design/v2');await page.waitForTimeout(700);await frame.locator('#studio-export').screenshot({path:path.join(out,'preset-'+id+'.png')})}
 pass('twelve builtin presets render through one configuration model')
 await preset('creator');await frame.getByRole('checkbox',{name:'封面悬停',exact:true}).uncheck();await frame.getByRole('button',{name:'顶部导航',exact:true}).click();await frame.locator('#radius').fill('20');await frame.locator('#radius').dispatchEvent('input');await frame.locator('#dark-preview').check()
 let selected=await config();assert.equal(selected.navigation,'top');assert.equal(selected.radius,20);assert.equal(selected.dark,true);assert(!selected.effects.includes('hover'))
 assert(await frame.locator('#studio-export').evaluate(el=>el.classList.contains('p-dark')))
 await frame.locator('.p-item-main').first().click();await frame.getByRole('dialog',{name:/作品详情/}).waitFor();await frame.getByRole('button',{name:'关闭作品详情'}).click();await frame.getByRole('dialog',{name:/作品详情/}).waitFor({state:'detached'})
 await frame.getByRole('button',{name:'上移页脚状态'}).click();assert.deepEqual((await config()).modules.slice(-2),['footer','content'])
 pass('composition, appearance, tokens, module order and card expansion match exported JSON')
 await preset('library');const virtual=frame.locator('.p-virtual');await frame.locator('.p-virtual .p-item-main').first().click();assert(await frame.locator('.p-virtual .p-item').count()<14)
 await virtual.evaluate(el=>el.scrollTop=160000);await page.waitForTimeout(120);assert(await frame.locator('.p-virtual .p-item').count()<14)
 assert.notEqual(await frame.locator('.p-virtual .p-item').first().getAttribute('data-index'),'0')
 await virtual.evaluate(el=>el.scrollTop=0);await page.waitForTimeout(120);assert.equal(await frame.locator('.p-virtual .p-item-main').first().getAttribute('aria-expanded'),'true')
 await frame.getByRole('button',{name:'Metro 磁贴',exact:true}).click();assert.equal((await config()).performance,'standard');assert.equal((await config()).dataSize,100)
 assert((await frame.locator('#studio-message').innerText()).includes('已改用'))
 pass('real 10,000-item virtualization preserves expansion and incompatible changes are explained')
 await preset('canvas');const before=await frame.locator('.p-world').evaluate(el=>el.style.transform);await frame.getByRole('button',{name:'放大画布',exact:true}).click();assert.notEqual(await frame.locator('.p-world').evaluate(el=>el.style.transform),before);assert(await frame.locator('.p-world .p-item').count()<100)
 const canvas=await frame.getByLabel('项目画布',{exact:true}).boundingBox();await page.mouse.move(canvas.x+canvas.width-40,canvas.y+35);await page.mouse.down();await page.mouse.move(canvas.x+canvas.width-120,canvas.y+70);await page.mouse.up()
 pass('canvas pan, zoom and actual viewport culling')
 await preset('kanban');const first=frame.locator('.p-lane').nth(0).locator('.p-item').first(),target=frame.locator('.p-lane').nth(1);const firstId=await first.getAttribute('data-id');await first.dragTo(target.locator('h3').first());assert(await target.locator(`[data-id="${firstId}"]`).count())
 pass('kanban preview supports cross-column dragging')
 await preset('creator');await frame.locator('.p-search').fill('品牌');assert.equal(await frame.locator('.p-item:visible').count(),1)
 await frame.getByRole('button',{name:'收藏',exact:true}).click();assert((await frame.locator('.p-result').innerText()).includes('收藏'))
 await frame.getByRole('tab',{name:'Agent 设计说明',exact:true}).click();assert((await frame.locator('#output-text').inputValue()).includes('页面结构：卡片网格'))
 await frame.getByRole('tab',{name:'配置 JSON',exact:true}).click()
 pass('preview search/navigation and matching Agent instructions')
 await frame.getByRole('button',{name:'粘贴导入配置'}).click();await frame.locator('#import-text').fill('{"schema":"toolhub.design/v2","script":"alert(1)"}');await frame.getByRole('button',{name:'校验并应用'}).click();assert((await frame.locator('#import-error').innerText()).includes('配置字段'));await frame.getByRole('button',{name:'取消',exact:true}).click()
 const raw=await config();const malicious={...raw,title:'<img src=x onerror=alert(1)>',purpose:'<script>window.pwned=true</script>'};await frame.getByRole('button',{name:'粘贴导入配置'}).click();await frame.locator('#import-text').fill(JSON.stringify(malicious));await frame.getByRole('button',{name:'校验并应用'}).click();assert.equal(await frame.locator('#studio-export img').count(),0);assert.equal(await frame.locator('#studio-export script').count(),0)
 await frame.getByRole('tab',{name:'可运行 HTML 示例',exact:true}).click();const html=await frame.locator('#output-text').inputValue();fs.writeFileSync(path.join(out,'escaped-example.html'),html)
 const standalone=await chromium.launch({headless:true});const standalonePage=await standalone.newPage();const standaloneErrors=[];standalonePage.on('pageerror',error=>standaloneErrors.push(String(error)));await standalonePage.goto('file:///'+path.join(out,'escaped-example.html').replaceAll('\\','/'));assert.equal(await standalonePage.locator('img').count(),0);assert.equal(await standalonePage.evaluate(()=>window.pwned),undefined);await standalonePage.locator('.p-item-main').first().click();await standalonePage.getByRole('dialog',{name:/作品详情/}).waitFor();assert.deepEqual(standaloneErrors,[]);await standalone.close()
 pass('strict pasted config validation, escaped content and runnable exported HTML')
 await frame.getByRole('tab',{name:'配置 JSON',exact:true}).click();await preset('creator');await frame.locator('#project-title').fill('我的真实项目');await frame.locator('#preset-name').fill('验收组合');await frame.getByRole('button',{name:'保存为预设',exact:true}).click();await frame.getByText('预设「验收组合」已保存；重新打开设计台后仍可使用。',{exact:true}).waitFor()
 await page.reload();await studio();assert.equal(await frame.locator('#project-title').inputValue(),'我的真实项目');await frame.locator('#preset-select').selectOption({label:'验收组合'});await frame.getByRole('button',{name:'应用预设',exact:true}).click();assert.equal((await config()).title,'我的真实项目')
 await frame.getByRole('button',{name:'删除自定义预设',exact:true}).click();await frame.getByRole('button',{name:'删除自定义预设',exact:true}).click();await frame.getByText('已删除预设，当前设计仍保留。',{exact:true}).waitFor();assert.equal((await config()).title,'我的真实项目')
 pass('named preset persistence across reload, restore and explicit deletion')
 for(const width of [1700,1180,720]){await page.setViewportSize({width,height:1080});assert(await frame.locator('#design-studio').evaluate(el=>el.scrollWidth<=document.documentElement.clientWidth+1))}
 await page.setViewportSize({width:1700,height:1080});await preset('creator');await page.screenshot({path:path.join(out,'native-studio.png')})
 assert.deepEqual(errors,[]);assert.deepEqual(csp,[])
 pass('responsive native layout without runtime or CSP violations')
}catch(error){results.push({name:'acceptance',ok:false,error:String(error),stack:error.stack});await page.screenshot({path:path.join(out,'failure.png')}).catch(()=>{});throw error}
finally{fs.writeFileSync(path.join(out,'results.json'),JSON.stringify({results,errors,csp},null,2));await browser.close()}
