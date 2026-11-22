import { createContext, useCallback, useContext, useEffect, useState, type ReactNode } from 'react'
import { invoke } from '@tauri-apps/api/core'
import './updates.css'

type UpdateView = {phase:string;current_version:string;available_version:string|null;downloaded:number;total:number;notes:string;error:string|null;auto_check:boolean;preview:boolean;last_check:number;supported:boolean;last_result:{status:string;version:string;message:string|null}|null}
type UpdateContext = {state:UpdateView|null;error:string;check:()=>Promise<void>;download:()=>Promise<void>;cancel:()=>Promise<void>;install:()=>Promise<void>;preferences:(auto:boolean,preview:boolean)=>Promise<void>}
const Context=createContext<UpdateContext|null>(null)
const busy=(phase?:string)=>['checking','downloading','cancelling','installing'].includes(phase||'')
const size=(bytes:number)=>`${(bytes/1024/1024).toFixed(1)} MB`
export function UpdateProvider({children}:{children:ReactNode}) {
  const [state,setState]=useState<UpdateView|null>(null),[error,setError]=useState('')
  const refresh=useCallback(async()=>{try{setState(await invoke<UpdateView>('updater_status'))}catch(e){setError(String(e))}},[])
  useEffect(()=>{void refresh();const timer=setTimeout(()=>{void invoke<UpdateView>('updater_check',{automatic:true}).then(setState).catch(e=>setError(String(e)))},10000);return()=>clearTimeout(timer)},[refresh])
  useEffect(()=>{if(!busy(state?.phase))return;const timer=setInterval(()=>void refresh(),700);return()=>clearInterval(timer)},[state?.phase,refresh])
  const action=async(name:string,params?:Record<string,unknown>)=>{setError('');try{const result=await invoke<UpdateView>(name,params);if(result)setState(result)}catch(e){setError(String(e));void refresh()}}
  const value:UpdateContext={state,error,check:()=>action('updater_check',{automatic:false}),download:()=>action('updater_download'),cancel:()=>action('updater_cancel'),install:async()=>{setError('');try{setState(s=>s?{...s,phase:'installing'}:s);await invoke('updater_install')}catch(e){setError(String(e));void refresh()}},preferences:(auto,preview)=>action('updater_preferences',{autoCheck:auto,preview})}
  return <Context.Provider value={value}>{children}</Context.Provider>
}
function useUpdates(){const context=useContext(Context);if(!context)throw Error('Update provider unavailable');return context}
export function UpdateNotice({open}:{open:()=>void}) {
  const {state}=useUpdates()
  if(!state?.available_version||state.phase==='current'||state.phase==='idle')return null
  return <div className="update-notice" role="status"><span><i/>{state.phase==='ready'?'更新已就绪':'发现新版本'} <strong>v{state.available_version}</strong></span><button className="btn" onClick={open}>{state.phase==='ready'?'前往重启更新':'查看更新'}</button></div>
}
export default function UpdateSetting() {
  const {state,error,check,download,cancel,install,preferences}=useUpdates()
  const active=busy(state?.phase),phase=state?.phase||'idle'
  const labels:Record<string,string>={idle:'随时检查新版本',checking:'正在检查…',current:'当前已是最新版本',available:'发现新版本',downloading:'正在下载',cancelling:'正在取消…',ready:'已验证，等待重启',cancelled:'下载已取消',installing:'正在准备重启…',error:'暂时无法完成更新'}
  const progress=state&&state.total?Math.min(100,Math.floor(state.downloaded/state.total*100)):0
  return <section className="card card-pad update-setting" aria-label="软件更新">
    <div className="update-heading"><h3>软件更新</h3><span className={'update-status '+(phase==='current'?'is-current':'')}>{labels[phase]}</span></div>
    <div className="update-version"><span>ToolHub</span><strong>v{state?.current_version||'—'}</strong></div>
    {state?.available_version&&<div className="update-next">可更新至 <strong>v{state.available_version}</strong><span>{size(state.total)}</span></div>}
    {['downloading','cancelling','ready'].includes(phase)&&state&&<div className="update-progress"><progress aria-label="更新下载进度" max={100} value={phase==='ready'?100:progress}/><span>{phase==='ready'?'完整性校验通过':`${size(state.downloaded)} / ${size(state.total)}`}</span></div>}
    <div className="update-buttons">
      <button className="btn" disabled={active||!state?.supported} onClick={()=>void check()}>{phase==='checking'?'检查中…':'检查更新'}</button>
      {state?.available_version&&['available','cancelled','error'].includes(phase)&&<button className="btn btn-primary" onClick={()=>void download()}>下载更新</button>}
      {['downloading','cancelling'].includes(phase)&&<button className="btn" disabled={phase==='cancelling'} onClick={()=>void cancel()}>取消下载</button>}
      {phase==='ready'&&<button className="btn btn-primary" onClick={()=>void install()}>安装并重启</button>}
      {phase==='installing'&&<button className="btn btn-primary" disabled>正在准备…</button>}
    </div>
    {(error||state?.error)&&<p className="update-error" role="alert">{error||state?.error}</p>}
    {state?.notes&&<details className="update-notes"><summary>本次更新内容</summary><div>{state.notes}</div></details>}
    <div className="update-option"><label htmlFor="auto-check-updates">启动时检查更新</label><button id="auto-check-updates" className={'switch '+(state?.auto_check?'on':'')} aria-label="启动时检查更新" aria-pressed={state?.auto_check||false} disabled={!state||active} onClick={()=>state&&void preferences(!state.auto_check,state.preview)}><i/></button></div>
    <div className="update-channel"><span>更新通道</span><div className="seg"><button disabled={!state||active} className={!state?.preview?'active':''} onClick={()=>state&&void preferences(state.auto_check,false)}>稳定版</button><button disabled={!state||active} className={state?.preview?'active':''} onClick={()=>state&&void preferences(state.auto_check,true)}>预览版</button></div></div>
    {state?.last_check? <div className="update-time">上次检查 {new Date(state.last_check*1000).toLocaleString()}</div>:null}
    {state?.last_result&&<p className={'update-last '+(state.last_result.status==='updated'?'':'update-error')}>{state.last_result.status==='updated'?`已更新至 v${state.last_result.version}`:state.last_result.status==='rolled_back'?`更新未完成，已恢复旧版本。${state.last_result.message||''}`:state.last_result.message}</p>}
  </section>
}
