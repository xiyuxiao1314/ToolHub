import { useCallback, useEffect, useMemo, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
export type View = {id:string; label:string; icon:string; order:number; html:string; css:string; script:string}
export type CliParameter = {id:string;label:string;kind:'input_file'|'output_file'|'choice';extensions?:string[];choices?:{label:string;value:string}[];default?:string}
export type CliAction = {id:string;label:string;description:string;parameters:CliParameter[];args:string[];timeout_ms:number}
export type Cli = {id:string;label:string;icon:string;order:number;tool_definition:string;actions:CliAction[]}
export type Theme = {colors:Record<string,string>; layout:'left'|'right'|'top'; density:'comfortable'|'compact'; sidebar_width:number; radius:number}
export type Package = {schema:string; id:string; name:string; description:string; version:string; author:string; license:string; platforms:string[]; min_toolhub_version:string; host_module?:string; navigation?:{label:string;icon:string;order:number}; views?:View[]; cli?:Cli; theme?:Theme; permissions?:string[]; dependencies?:{id:string;min_version:string}[]; recognition?:{tools:any[];version:string}; skills:any[]}
export type Installed = {package:Package;digest:string;enabled:boolean;can_rollback:boolean;source:string}
export type Preferences = {theme_id?:string|null;layout?:'left'|'right'|'top'|null;density?:'comfortable'|'compact'|null}
export type NavItem = {id:string;label:string;icon:string;order:number;extension?:Installed;view?:View;cli?:Cli}
export const rpc = (method:string,params:unknown={})=>invoke<any>('rpc',{method,params})
export const changed = ()=>window.dispatchEvent(new Event('toolhub:extensions-changed'))
export function useExtensionHost(){
  const [installed,setInstalled]=useState<Installed[]>([]), [prefs,setPrefs]=useState<Preferences>({}), [ready,setReady]=useState(false),[error,setError]=useState('')
  const reload=useCallback(async()=>{try{const [rows,p]=await Promise.all([rpc('extension.list'),rpc('extension.preferences')]);setInstalled(rows);setPrefs(p);setError('');setReady(true)}catch(e){setError(String(e))}},[])
  useEffect(()=>{void reload(); const refresh=()=>void reload();window.addEventListener('toolhub:extensions-changed',refresh);window.addEventListener('focus',refresh);return()=>{window.removeEventListener('toolhub:extensions-changed',refresh);window.removeEventListener('focus',refresh)}},[reload])
  const nav=useMemo(()=>{
    // Primary groups are owned by the shell. Packages only contribute secondary destinations.
    const items:NavItem[]=[
      {id:'workspace',label:'工作空间',icon:'',order:0},
      {id:'agents',label:'已接入智能体',icon:'',order:40},
      {id:'agent-guide',label:'接入说明',icon:'',order:41},
      {id:'extension-apps',label:'扩展应用',icon:'',order:80},
      {id:'extensions',label:'已安装',icon:'',order:85},
      {id:'extension-catalog',label:'扩展市场',icon:'',order:86},
      {id:'extension-appearance',label:'主题与布局',icon:'',order:87},
      {id:'settings',label:'通用设置',icon:'',order:1000},
    ]
    for(const item of installed.filter(e=>e.enabled)){
      const p=item.package
      if(p.host_module && p.navigation)items.push({id:p.host_module,...p.navigation,extension:item})
      if(p.cli)items.push({id:p.cli.id,label:p.cli.label,icon:p.cli.icon,order:p.cli.order,extension:item,cli:p.cli})
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
