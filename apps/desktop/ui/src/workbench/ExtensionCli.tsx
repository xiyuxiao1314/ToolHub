import {useCallback,useEffect,useRef,useState} from 'react'
import {invoke} from '@tauri-apps/api/core'
import {copyText} from '../AgentHelp'
import {rpc} from './host'
import type {NavItem} from './host'
import './extension-cli.css'

type Tool = {id:string;name:string;path:string;version:string|null;trust:string;status:string;definition_id:string;preferred?:boolean}
const trust=(s:string)=>{try{return JSON.parse(s).level as string}catch{return s}}
const labels:Record<string,string>={running:'运行中',success:'已完成',failed:'失败',cancelled:'已取消',timed_out:'超时',interrupted:'重启中断'}
export default function ExtensionCli({item,navigate}:{item:NavItem;navigate:(id:string)=>void}){
  const extension=item.extension!, cli=item.cli!
  const [actionId,setActionId]=useState(cli.actions[0].id), [values,setValues]=useState<Record<string,string>>({})
  const action=cli.actions.find(a=>a.id===actionId)!
  const [tools,setTools]=useState<Tool[]>([]),[instance,setInstance]=useState(''),[error,setError]=useState(''),[loading,setLoading]=useState(true),[busy,setBusy]=useState(false)
  const [preview,setPreview]=useState<any>(null),[tasks,setTasks]=useState<any[]>([]),[detail,setDetail]=useState<any>(null),[supported,setSupported]=useState(true)
  const alive=useRef(true), epoch=useRef(0), panel=useRef<HTMLDivElement>(null)
  const bridge=useCallback((method:string,params:unknown={})=>rpc('extension.invoke',{id:extension.package.id,expected_digest:extension.digest,method,params}),[extension.package.id,extension.digest])
  const refreshTasks=useCallback(async()=>{const rows=await bridge('execute.list');if(alive.current)setTasks(rows)},[bridge])
  const load=useCallback(async()=>{
    setLoading(true)
    try{
      const [rows,version]=await Promise.all([bridge('registry.search',{query:cli.tool_definition,limit:100}),invoke<{platform:string}>('app_versions')])
      const matching:Tool[]=rows.filter((t:Tool)=>t.definition_id===cli.tool_definition&&t.status==='available')
      if(alive.current){setTools(matching);setInstance(old=>matching.some(t=>t.id===old)?old:(matching.find(t=>t.preferred)?.id??matching[0]?.id??''));setSupported(version.platform==='windows');setError('')}
      await refreshTasks()
    }catch(e){if(alive.current)setError(String(e))}finally{if(alive.current)setLoading(false)}
  },[bridge,cli.tool_definition,refreshTasks])
  useEffect(()=>{alive.current=true;void load();return()=>{alive.current=false;epoch.current++}},[load])
  useEffect(()=>{setValues(Object.fromEntries(action.parameters.map(p=>[p.id,p.default??''])));setPreview(null);setError('');epoch.current++},[action])
  useEffect(()=>{
    if(!tasks.some(t=>t.status==='running'))return
    const timer=window.setInterval(()=>void refreshTasks().catch(e=>{if(alive.current)setError(String(e))}),1500)
    return()=>window.clearInterval(timer)
  },[tasks,refreshTasks])
  useEffect(()=>{if(preview)panel.current?.focus()},[preview])
  const setValue=(id:string,value:string)=>{epoch.current++;setValues(v=>({...v,[id]:value}));setPreview(null)}
  const tool=tools.find(t=>t.id===instance), trusted=!!tool&&['known','verified','user_trusted'].includes(trust(tool.trust))
  const act=async(job:()=>Promise<void>)=>{setBusy(true);setError('');try{await job()}catch(e){if(alive.current)setError(String(e))}finally{if(alive.current)setBusy(false)}}
  const pick=(id:string,save:boolean)=>act(async()=>{
    const parameter=action.parameters.find(p=>p.id===id)!
    const name=values.input?.split(/[\\/]/).pop()?.replace(/\.[^.]+$/,'')??'media'
    const selected=await invoke<string|null>('pick_cli_path',{save,name:`${name}-${action.id}.${parameter.extensions?.[0]??'mp4'}`})
    if(selected&&alive.current)setValue(id,selected)
  })
  const params={id:extension.package.id,expected_digest:extension.digest,action_id:action.id,instance_id:instance,values}
  const getPreview=()=>act(async()=>{const token=++epoch.current;const result=await rpc('extension.cli_preview',params);if(alive.current&&epoch.current===token)setPreview(result)})
  const run=()=>act(async()=>{if(!preview)return;await rpc('extension.cli_start',{...params,confirmation:preview});setPreview(null);await refreshTasks()})
  const selectAction=(id:string)=>{epoch.current++;setActionId(id)}
  return <div className="cli-workshop">
    <header className="topbar"><div><h1>{cli.label}</h1><p>{extension.package.name} · v{extension.package.version}</p></div><button className="btn" onClick={()=>navigate('extensions')}>管理扩展</button></header>
    {error&&<div className="banner-error" role="alert"><span>{error}</span><button aria-label="关闭媒体错误" onClick={()=>setError('')}>×</button></div>}
    <section className="cli-hero"><div className="cli-emblem" aria-hidden="true">▶</div><div><span className="cli-eyebrow">LOCAL CLI / TOOLHUB</span><h2>把本机命令，变成日常操作。</h2><p>{extension.package.description}</p></div><span className="cli-local-tag">本机工具 · 后台运行</span></section>
    {!supported&&<p className="help-warning">当前平台暂不支持 ToolHub 身份绑定执行。可查看操作和复制流程；请勿将此状态视为已完成转码。</p>}
    <div className="cli-columns"><section className="card cli-form" aria-label="媒体操作">
      <div className="cli-step"><span>01</span><h3>选择处理方式</h3></div>
      <div className="cli-tabs" role="tablist" aria-label="处理方式">{cli.actions.map(a=><button role="tab" aria-selected={a.id===action.id} key={a.id} disabled={busy} onClick={()=>selectAction(a.id)}>{a.label}</button>)}</div>
      <p className="cli-description">{action.description}</p>
      <div className="cli-step"><span>02</span><h3>选择输入与输出</h3></div>
      {action.parameters.map(p=><div className="cli-field" key={p.id}><label htmlFor={'cli-'+p.id}>{p.label}</label>{p.kind==='choice'?<select id={'cli-'+p.id} value={values[p.id]??p.default??''} disabled={busy} onChange={e=>setValue(p.id,e.target.value)}>{p.choices?.map(c=><option value={c.value} key={c.value}>{c.label}</option>)}</select>:<div className="cli-file"><input id={'cli-'+p.id} value={values[p.id]??''} readOnly placeholder={p.kind==='input_file'?'选择本地媒体文件':'选择新文件的保存位置'} title={values[p.id]}/><button className="btn" disabled={busy||loading} onClick={()=>void pick(p.id,p.kind==='output_file')}>{p.kind==='input_file'?'选择文件':'保存到…'}</button></div>}<small>{p.kind==='output_file'?`输出 .${p.extensions?.join(' / .')}，已有文件不会被覆盖`:p.kind==='input_file'?`支持 ${p.extensions?.map(e=>'.'+e).join(' / ')}`:null}</small></div>)}
      <div className="cli-form-footer"><span>单次最长 {Math.round(action.timeout_ms/60000)} 分钟 · 随时取消</span><button className="btn btn-primary" disabled={busy||loading||!supported||!trusted||action.parameters.some(p=>!values[p.id])} onClick={()=>void getPreview()}>预览并运行 →</button></div>
    </section><aside className="cli-aside"><section className="card cli-engine"><div className="cli-step"><span>03</span><h3>本机执行引擎</h3></div><label htmlFor="cli-engine">本机程序实例</label><select id="cli-engine" value={instance} disabled={busy||loading} onChange={e=>{setInstance(e.target.value);setPreview(null);epoch.current++}}>{!tools.length&&<option value="">{loading?'正在查询…':'尚未发现匹配的可用程序'}</option>}{tools.map(t=><option key={t.id} value={t.id}>{t.version??'版本未探测'} · {t.path}</option>)}</select>{tool&&<><code className="cli-tool-path">{tool.path}</code><span className={'tag '+(trusted?'market-ready':'')}>{trusted?'已信任 · 执行仍按策略检查':'信任待核对'}</span></>}{!trusted&&!loading&&<p className="cli-description">{tools.length?'请先在工具页核对来源并信任这个安装实例。扩展不会自动信任程序。':'到工具页扫描或添加已有程序，再回来刷新。'}</p>}<div className="program-actions"><button className="btn" disabled={busy} onClick={()=>void load()}>刷新引擎</button><button className="text-btn" onClick={()=>navigate('tools')}>核对本机工具</button></div></section>
    <section className="cli-agent"><span className="cli-eyebrow">FOR YOUR AGENT</span><h3>同一套工具，也给 Agent 用。</h3><p>安装后附带通用流程，Agent 按需查找。沿用统一 MCP，无需再连接一个服务。</p><button className="btn" onClick={()=>void act(()=>copyText(extension.package.skills[0]?.instructions??'通过 ToolHub 查询并检查已有 FFmpeg 后，按用户授权使用。'))}>复制 Agent 流程</button></section></aside></div>
    {preview&&<section ref={panel} tabIndex={-1} className="card cli-confirm" aria-label="本次执行预览"><div className="help-heading"><h3>核对本次执行</h3><button className="text-btn" disabled={busy} onClick={()=>setPreview(null)}>收起</button></div><p>{preview.label} · 一次授权；当前工具与目录策略仍生效。</p><code>{preview.executable}</code><pre>{JSON.stringify(preview.args,null,2)}</pre><p className="muted">输出：{preview.output}<br/>工作目录：{preview.cwd}</p><div className="program-actions"><button className="btn btn-primary" disabled={busy||!supported} onClick={()=>void run()}>确认并运行（本次授权）</button><button className="btn" disabled={busy} onClick={()=>setPreview(null)}>返回修改</button></div></section>}
    <section className="card cli-jobs" aria-label="媒体任务"><div className="help-heading"><h3>处理记录 <span className="muted">{tasks.length}</span></h3><button className="text-btn" disabled={busy} onClick={()=>void act(refreshTasks)}>刷新任务</button></div>{!tasks.length?<p className="muted">准备好文件后，先预览命令，再开始处理。</p>:<ul className="list-clean">{tasks.map(t=><li key={t.execution_id}><div><strong>{labels[t.status]??t.status}</strong><p className="muted">{t.artifacts?.[0]?.name??t.name??extension.package.name} · {t.elapsed_ms!==undefined?`已运行 ${Math.floor(t.elapsed_ms/1000)} 秒`:t.duration_ms!==undefined?`${(t.duration_ms/1000).toFixed(1)} 秒`:''}</p>{t.status==='running'&&<div className="cli-working" role="progressbar" aria-label="正在处理媒体"/>}{(t.artifacts??[]).map((a:any)=><button className="text-btn" key={a.path} onClick={()=>void act(async()=>{await invoke('reveal_path',{path:a.path})})}>定位输出：{a.name}</button>)}</div><div className="program-actions"><button className="btn" disabled={busy} onClick={()=>void act(async()=>setDetail(await bridge('execute.status',{execution_id:t.execution_id})))}>查看结果</button>{t.status==='running'&&<button className="btn" disabled={busy} onClick={()=>void act(async()=>{await rpc('execute.cancel',{execution_id:t.execution_id});await refreshTasks()})}>取消处理</button>}</div></li>)}</ul>}<p className="cli-note">运行中显示状态与耗时；完整日志在结束后可读。取消或失败可能留下不完整输出，ToolHub 会保留文件。重启后历史状态仍保留，输出日志仅在当前服务会话内保留。</p>{detail&&<div className="cli-log"><div className="help-heading"><strong>{labels[detail.status]??detail.status} · 退出码 {detail.exit_code??'—'}</strong><button className="text-btn" onClick={()=>setDetail(null)}>收起结果</button></div><pre>{detail.result?`${detail.result.stdout??''}\n${detail.result.stderr??''}`:'任务仍在运行，或当前服务会话未保留日志。'}</pre>{detail.result?.truncated&&<p>日志过长，已截断。</p>}</div>}</section>
  </div>
}
