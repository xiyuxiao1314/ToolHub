import { Component, lazy, Suspense, type ReactNode } from 'react'
import type { WorkbenchContext } from '../official/Workbench'
const pages:Record<string,React.LazyExoticComponent<React.ComponentType<any>>>={
  services:lazy(()=>import('../Services')),
  tools:lazy(()=>import('../official/ToolsPage')),programs:lazy(()=>import('../Programs')),artifacts:lazy(()=>import('../Artifacts')),
  envs:lazy(()=>import('../official/EnvsPage')),skills:lazy(()=>import('../Skills')),market:lazy(()=>import('../Market')),tasks:lazy(()=>import('../official/TasksPage')),
  agents:lazy(()=>import('../official/AgentsPage')),settings:lazy(()=>import('../official/SettingsPage')),
}
class Boundary extends Component<{children:ReactNode;recover:()=>void},{failed:boolean}>{
  state={failed:false}
  static getDerivedStateFromError(){return {failed:true}}
  render(){return this.state.failed?<div className="card card-pad"><h2>页面暂时无法加载</h2><p>可返回扩展管理，禁用出错的扩展或恢复界面。</p><button className="btn" onClick={this.props.recover}>打开扩展管理</button></div>:this.props.children}
}
export default function BuiltinPage({id,context,guide=false}:{id:string;context:WorkbenchContext;guide?:boolean}){
  const Page=pages[id];if(!Page)return null
  const props={...context,guide,onLibrary:()=>context.setPage('skills'),onTools:()=>context.setPage('tools'),onAddTool:()=>{context.setPage('tools');context.setAddToolOpen(true)}}
  return <Boundary key={id} recover={()=>context.setPage('extensions')}><Suspense fallback={<p className="muted">正在加载功能扩展…</p>}><Page {...props}/></Suspense></Boundary>
}
