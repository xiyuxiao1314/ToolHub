import { useCallback, useEffect, useMemo, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
export type View = {id:string; label:string; icon:string; order:number; html:string; css:string; script:string}
export type Theme = {colors:Record<string,string>; layout:'left'|'right'|'top'; density:'comfortable'|'compact'; sidebar_width:number; radius:number}
export type Package = {schema:string; id:string; name:string; description:string; version:string; author:string; license:string; platforms:string[]; min_toolhub_version:string; host_module?:string; navigation?:{label:string;icon:string;order:number}; views?:View[]; theme?:Theme; permissions?:string[]; dependencies?:{id:string;min_version:string}[]; recognition?:{tools:any[];version:string}; skills:any[]}
export type Installed = {package:Package;digest:string;enabled:boolean;can_rollback:boolean;source:string}
export type Preferences = {theme_id?:string|null;layout?:'left'|'right'|'top'|null;density?:'comfortable'|'compact'|null}
export type NavItem = {id:string;label:string;icon:string;order:number;extension?:Installed;view?:View}
export const rpc = (method:string,params:unknown={})=>invoke<any>('rpc',{method,params})
export const changed = ()=>window.dispatchEvent(new Event('toolhub:extensions-changed'))
export function useExtensionHost(){
  const [installed,setInstalled]=useState<Installed[]>([]), [prefs,setPrefs]=useState<Preferences>({}), [ready,setReady]=useState(false),[error,setError]=useState('')
  const reload=useCallback(async()=>{try{const [rows,p]=await Promise.all([rpc('extension.list'),rpc('extension.preferences')]);setInstalled(rows);setPrefs(p);setError('');setReady(true)}catch(e){setError(String(e))}},[])
  useEffect(()=>{void reload(); const refresh=()=>void reload();window.addEventListener('toolhub:extensions-changed',refresh);window.addEventListener('focus',refresh);return()=>{window.removeEventListener('toolhub:extensions-changed',refresh);window.removeEventListener('focus',refresh)}},[reload])
  const nav=useMemo(()=>{
    const items:NavItem[]=[{id:'agents',label:'智能体',icon:'🤖',order:40},{id:'extensions',label:'扩展',icon:'◈',order:85},{id:'settings',label:'设置',icon:'⚙️',order:1000}]
    for(const item of installed.filter(e=>e.enabled)){
      const p=item.package
      if(p.host_module && p.navigation)items.push({id:p.host_module,...p.navigation,extension:item})
      for(const view of p.views??[])items.push({id:view.id,label:view.label,icon:view.icon,order:view.order,extension:item,view})
    }
    return items.sort((a,b)=>a.order-b.order||a.id.localeCompare(b.id))
  },[installed])
  const enabled=useMemo(()=>new Set(installed.filter(e=>e.enabled&&e.package.host_module).map(e=>e.package.host_module!)),[installed])
  const theme=installed.find(e=>e.enabled&&e.package.id===prefs.theme_id)?.package.theme
  useEffect(()=>{
    const style=document.documentElement.style
    const keys=['bg','panel','panel-soft','ink','ink-2','muted','line','accent','accent-soft','ok','ok-soft','warn','warn-soft','danger','danger-soft','radius','sidebar']
    keys.forEach(k=>style.removeProperty('--'+k))
    if(theme){Object.entries(theme.colors).forEach(([k,v])=>style.setProperty('--'+k,v));style.setProperty('--radius',theme.radius+'px');style.setProperty('--sidebar',theme.sidebar_width+'px')}
    return()=>keys.forEach(k=>style.removeProperty('--'+k))
  },[theme])
  return {installed,prefs,ready,error,reload,nav,enabled,theme,layout:prefs.layout??theme?.layout??'left',density:prefs.density??theme?.density??'comfortable'}
}
