import { useEffect, useMemo, useRef, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { NavItem } from './host'
import { rpc } from './host'
import { extensionState } from './extensionStorage'
export default function ExtensionFrame({item,navigate,notify}:{item:NavItem;navigate:(id:string)=>void;notify:(m:string)=>void}){
  const frame=useRef<HTMLIFrameElement>(null), [url,setUrl]=useState(''),[error,setError]=useState('')
  const token=useMemo(()=>crypto.randomUUID(),[item.id,item.extension?.digest])
  const view=item.view!, extension=item.extension!
  useEffect(()=>{
    let active=true;setUrl('');setError('')
    void invoke<string>('extension_view_url',{id:extension.package.id,expectedDigest:extension.digest,viewId:view.id,token}).then(value=>{if(active)setUrl(value)}).catch(e=>{if(active)setError(String(e))})
    return()=>{active=false}
  },[token,extension.digest,extension.package.id,view.id])
  useEffect(()=>{
    let active=true,inFlight=0
    const listener=async(e:MessageEvent)=>{
      const m=e.data
      if(e.source!==frame.current?.contentWindow||m?.channel!=='toolhub.sdk'||m.token!==token||!Number.isSafeInteger(m.id)||m.id<1)return
      const respond=(data:Record<string,unknown>)=>{if(active)frame.current?.contentWindow?.postMessage({channel:'toolhub.sdk',token,id:m.id,...data},'*')}
      if(inFlight>=4){respond({error:'Too many concurrent requests'});return}
      inFlight++
      try{
        if(m.type==='call'){if(typeof m.method!=='string'||JSON.stringify(m.params??{}).length>65536)throw new Error('Invalid bridge request');respond({result:await rpc('extension.invoke',{id:extension.package.id,expected_digest:extension.digest,method:m.method,params:m.params??{}})})}
        else if(m.type==='navigate'){if(typeof m.page!=='string')throw new Error('Invalid page');navigate(m.page);respond({result:true})}
        else if(m.type==='notify'){notify(String(m.message??'').slice(0,256));respond({result:true})}
        else if(m.type==='storage'){
          if(JSON.stringify(m.data??null).length>65536)throw Error('Extension storage payload too large')
          const current=await rpc('extension.list')
          if(!active||!current.some((row:any)=>row.enabled&&row.package.id===extension.package.id&&row.digest===extension.digest))throw Error('Extension disabled, removed or updated')
          respond({result:extensionState(localStorage,extension.package.id,m.operation,m.revision,m.data)})
        }
        else respond({error:'Unknown SDK action'})
      }catch(error){respond({error:String(error)})}finally{inFlight--}
    }
    window.addEventListener('message',listener);return()=>{active=false;window.removeEventListener('message',listener)}
  },[token,extension.digest,extension.package.id,navigate,notify])
  return <><div className="topbar extension-frame-heading"><div><h1>{view.label}</h1><p>{extension.package.name} · v{extension.package.version}</p></div><button className="btn" onClick={()=>navigate('extensions')}>管理此扩展</button></div>{error?<div className="banner-error" role="alert">{error}</div>:url?<iframe ref={frame} title={view.label} className="extension-frame" sandbox="allow-scripts" src={url} onLoad={()=>{
      const colors:Record<string,string>={};const style=getComputedStyle(document.documentElement)
      for(const key of ['bg','panel','panel-soft','ink','ink-2','muted','line','accent','accent-soft','radius'])colors[key]=style.getPropertyValue('--'+key).trim()
      frame.current?.contentWindow?.postMessage({channel:'toolhub.sdk',token,type:'theme',colors},'*')
    }}/>:<p className="muted">正在加载隔离扩展页面…</p>}</>
}
