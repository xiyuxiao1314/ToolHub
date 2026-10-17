import {useEffect, useState} from 'react'
import {invoke} from '@tauri-apps/api/core'
const rpc=(method:string,params:unknown={})=>invoke<any>('rpc',{method,params})
const labels:Record<string,string>={available:'完整安装',missing_package:'npm 包未安装',missing_binary:'只有 npm 包，缺少二进制',invalid_package:'包信息无效',invalid_binary:'二进制无效'}
export default function ProjectDependencies({refresh,fail}:{refresh:(message:string)=>Promise<any>;fail:(error:unknown)=>void}) {
 const [items,setItems]=useState<any[]>([]),[root,setRoot]=useState(''),[dist,setDist]=useState(''),[busy,setBusy]=useState(false),[open,setOpen]=useState(false)
 const load=()=>rpc('registry.project_dependencies').then(setItems)
 useEffect(()=>{void load().catch(fail)},[])
 async function choose(distribution=false){try{const path=await invoke<string|null>('pick_program_path',{directory:true});if(path){if(distribution)setDist(path);else{setRoot(path);setDist('');setOpen(true)}}}catch(e){fail(e)}}
 async function scan(roots:string[],custom?:string){setBusy(true);try{const result=await rpc('scan.start',{mode:'project',roots,project_overrides:custom!==undefined?(custom?[{project_root:roots[0],electron_dist:custom}]:[]):items.filter(i=>roots.includes(i.project_root)&&i.distribution_override).map(i=>({project_root:i.project_root,electron_dist:i.distribution_override}))});await load();await refresh(result.status==='completed'?'项目依赖扫描完成；未执行或提升信任':'部分目录未完成，请查看扫描记录')}catch(e){fail(e)}finally{setBusy(false)}}
 return <div className="card card-pad" style={{marginBottom:16}} data-testid="project-dependencies">
  <div style={{display:'flex',alignItems:'center',gap:10,flexWrap:'wrap'}}><strong>项目工具依赖</strong><button className="btn" onClick={()=>setOpen(!open)}>{open?'收起':'查看'}（{items.length}）</button><button className="btn" disabled={busy} onClick={()=>void choose()}>选择项目目录</button>{items.length>0&&<button className="btn" disabled={busy} onClick={()=>void scan([...new Set(items.map(i=>String(i.project_root)))])}>重新扫描已选项目</button>}</div>
  <p className="muted">识别所选项目的 Electron 包和实际程序，不遍历全部 node_modules，也不运行程序或自动信任。</p>
  {root&&<div><p>所选项目：<code>{root}</code></p>{dist&&<p>自定义分发目录：<code>{dist}</code></p>}<div style={{display:'flex',gap:8,flexWrap:'wrap'}}><button className="btn" disabled={busy} onClick={()=>void choose(true)}>选择自定义 Electron 分发目录（可选）</button>{dist&&<button className="btn" onClick={()=>setDist('')}>使用默认位置</button>}<button className="btn btn-primary" disabled={busy} onClick={()=>void scan([root],dist)}>{busy?'扫描中…':'扫描项目依赖'}</button></div></div>}
  {open&&items.map(i=><div key={i.project_root+i.package_name} style={{borderTop:'1px solid var(--border, #e5eaf3)',paddingTop:12,marginTop:12,overflowWrap:'anywhere'}}><strong>{i.project_name} · Electron {i.binary_version||i.package_version||i.declared_version||''}</strong><span className={`badge badge-${i.status==='available'?'ok':'warn'}`} style={{marginLeft:10}}>{labels[i.status]||i.status}</span><p className="muted"><code>{i.project_root}</code></p>{i.executable&&<p><code>{i.executable}</code></p>}{i.package_version&&<p className="muted">npm 包：{i.package_version} · 二进制：{i.binary_version||'未测得版本'}</p>}{i.issue&&<p className="muted">{i.issue}</p>}</div>)}
 </div>
}
