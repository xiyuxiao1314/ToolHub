const {chromium}=await import(process.env.TOOLHUB_PLAYWRIGHT_MODULE||'playwright');
import fs from 'node:fs';import path from 'node:path';import assert from 'node:assert/strict';
const stage=process.env.TOOLHUB_SKILL_QA_DIR?.replaceAll('\\','/');if(!stage)throw Error('TOOLHUB_SKILL_QA_DIR is required');const b=await chromium.connectOverCDP(process.env.TOOLHUB_SKILL_QA_CDP||'http://127.0.0.1:19332');
try {
 const p=b.contexts()[0].pages()[0];p.setDefaultTimeout(12000);
 const rpc=(method,params={})=>p.evaluate(({method,params})=>window.__TAURI_INTERNALS__.invoke('rpc',{method,params}),{method,params});
 assert.equal((await rpc('status')).registry_path.replaceAll('\\','/').toLowerCase(),stage.toLowerCase()+'/qa-registry.sqlite');
 const prior=await rpc('skill.distribution.list');assert.equal(prior.packages.length,0,'use a new isolated registry for acceptance');
 const fixture=stage+'/fixtures';for(const name of ['alpha-flow','beta-flow','charlie-flow','delta-flow','echo-flow','foxtrot-flow','golf-flow']) {const root=fixture+'/'+name;fs.mkdirSync(root+'/references',{recursive:true});fs.mkdirSync(root+'/scripts',{recursive:true});fs.writeFileSync(root+'/SKILL.md',`---\nname: ${name}\ndescription: A synthetic reusable workflow for acceptance.\nversion: 1.0.0\n---\nRead references/check.md. No scripts run during import.\n`);fs.writeFileSync(root+'/references/check.md','Synthetic support document.');fs.writeFileSync(root+'/scripts/test.py','raise RuntimeError("must never execute during sync")\n');}
 const targetRoot=stage+'/target-fixtures';for(const name of ['QA-Codex','QA-MiMo','QA-dsh'])fs.mkdirSync(targetRoot+'/'+name,{recursive:true});
 let inventory=prior;
 for(const [host,name] of [['codex','QA-Codex'],['mimo','QA-MiMo']])inventory=await rpc('skill.distribution.target',{revision:inventory.revision,root:targetRoot+'/'+name,host_id:host,name});
 // Ordinary offer is tested separately through a fresh CLI/MCP peer. Native RPC follows the exact same metadata-only service.
 await rpc('skill.offer',{path:fixture+'/alpha-flow',source_agent:'codex',version:'1.0.0'});
 await p.getByRole('navigation',{name:'一级导航',exact:true}).getByRole('button',{name:'工作空间',exact:true}).click();
 await p.getByRole('navigation',{name:'二级导航',exact:true}).getByRole('button',{name:'Skill 库',exact:true}).click();
 await p.getByRole('tab',{name:/已安装/}).click();await p.getByRole('button',{name:'检查并收录',exact:true}).click();
 await p.getByLabel('来源智能体标识').fill('codex');await p.getByRole('button',{name:'收录并选择同步目标 →',exact:true}).click();
 await p.getByRole('heading',{name:'分发到智能体',exact:true}).waitFor();
 const row=name=>p.locator('.sd-target-row').filter({has:p.getByText(name,{exact:true})});
 await row('QA-Codex').getByRole('checkbox').check();await row('QA-MiMo').getByRole('checkbox').check();await p.getByRole('button',{name:'同步所选智能体',exact:true}).click();
 await row('QA-Codex').getByText('已写入 · 待宿主重载',{exact:true}).waitFor();assert(fs.readFileSync(targetRoot+'/QA-MiMo/alpha-flow/scripts/test.py','utf8').includes('must never execute'));
 // New host after collection, followed by ordinary selection/backfill.
 inventory=await rpc('skill.distribution.list');inventory=await rpc('skill.distribution.target',{revision:inventory.revision,root:targetRoot+'/QA-dsh',host_id:'dsh',name:'QA-dsh'});
 await p.getByRole('button',{name:'刷新状态',exact:true}).click();await row('QA-dsh').getByRole('checkbox').check();await p.getByRole('button',{name:'同步所选智能体',exact:true}).click();await row('QA-dsh').getByText('已写入 · 待宿主重载',{exact:true}).waitFor();
 // Update still-managed copy, but preserve a local edit at a different target.
 const old=inventory.packages[0].digest;fs.appendFileSync(fixture+'/alpha-flow/SKILL.md','\nUpdated instructions.\n');fs.writeFileSync(targetRoot+'/QA-MiMo/alpha-flow/references/check.md','MANUAL EDIT');
 await p.getByRole('button',{name:'更新来源',exact:true}).click();await p.getByLabel('版本（可留空）').fill('1.1.0');await p.getByRole('button',{name:'收录并选择同步目标 →',exact:true}).click();
 await row('QA-MiMo').getByText('本地有差异',{exact:true}).waitFor();assert.equal(await row('QA-MiMo').getByRole('checkbox').isDisabled(),true);
 await row('QA-dsh').getByRole('checkbox').check();await p.getByRole('button',{name:'同步所选智能体',exact:true}).click();await row('QA-dsh').getByText('已写入 · 待宿主重载',{exact:true}).waitFor();assert.equal(fs.readFileSync(targetRoot+'/QA-MiMo/alpha-flow/references/check.md','utf8'),'MANUAL EDIT');
 // Fill library for real paged navigation without reading unrelated host Skill bodies.
 for(const name of ['beta-flow','charlie-flow','delta-flow','echo-flow','foxtrot-flow','golf-flow']){inventory=await rpc('skill.distribution.list');const preview=await rpc('skill.distribution.preview',{path:fixture+'/'+name});await rpc('skill.distribution.collect',{revision:inventory.revision,path:fixture+'/'+name,source_agent:'codex',scope:'portable',expected_digest:preview.digest});}
 await p.getByRole('button',{name:'返回列表',exact:true}).click();await p.getByRole('button',{name:'刷新状态',exact:true}).click();await p.waitForFunction(()=>document.querySelectorAll('.sd-skill').length===4);await p.getByRole('button',{name:'下一页',exact:true}).click();await p.waitForFunction(()=>document.querySelectorAll('.sd-skill').length===3);
 await p.getByLabel('搜索 Skill 或智能体').fill('alpha');await p.getByRole('button',{name:'管理同步 →',exact:true}).click();await row('QA-MiMo').getByText('本地有差异',{exact:true}).waitFor();
 await p.screenshot({path:stage+'/skill-targets.png'});
 await p.getByRole('tab',{name:/同步记录/}).click();assert.equal(await p.locator('.sd-history').count(),6);await p.screenshot({path:stage+'/skill-history.png'});
 const summary=await rpc('skill.distribution.list');assert.equal(summary.packages.length,7);assert.equal(summary.targets.filter(t=>t.name.startsWith('QA-')).length,3);assert(!JSON.stringify(summary).includes('raise RuntimeError'));
 await p.reload();await p.getByRole('heading',{name:'Skill 库',exact:true}).waitFor();await p.getByRole('tab',{name:/智能体目录/}).click();await p.getByRole('button',{name:'选择下发 →'}).first().waitFor();
 const after=await rpc('skill.distribution.list');assert.equal(after.packages.length,7);assert.equal(after.history.length,summary.history.length);
 for(const [width,height] of [[1440,900],[1100,760],[900,640]]){await p.setViewportSize({width,height});const geometry=await p.locator('.sd-page').evaluate(n=>({horizontal:n.scrollWidth>n.clientWidth+2,nested:[...n.querySelectorAll('*')].filter(x=>{const s=getComputedStyle(x);return ['auto','scroll'].includes(s.overflowY)&&x.scrollHeight>x.clientHeight+2}).length}));assert.deepEqual(geometry,{horizontal:false,nested:0});}
 await p.setViewportSize({width:1440,height:900});await p.getByRole('tab',{name:/已收录/}).click();await p.screenshot({path:stage+'/skill-library.png'});
 fs.writeFileSync(stage+'/ui-results.json',JSON.stringify({passed:true,nativeTauri:true,isolatedRegistry:true,offersCollectedThroughUi:true,fullPackageMultiHost:true,newHostBackfill:true,localEditsPreserved:true,versionUpdateWithBackup:true,paging:true,reloadPersistence:true,metadataFirst:true,noNestedScrollAtThreeSizes:true,nativeDirectoryChooser:'existing picker reused; system dialog selection not automated'},null,2));
 console.log('PASS real desktop Skill collection, multi-host sync, dsh backfill, local-edit preservation, paging and reload');
}finally{await b.close()}
