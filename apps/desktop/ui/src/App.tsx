import { useCallback, useEffect } from 'react'
import brandIcon from './assets/toolhub-a.png'
import { useWorkbench, Toast } from './official/Workbench'
import { useExtensionHost } from './workbench/host'
import BuiltinPage from './workbench/BuiltinPage'
import ExtensionFrame from './workbench/ExtensionFrame'
import ExtensionCli from './workbench/ExtensionCli'
import Extensions from './Extensions'
import './workbench/workbench.css'
export default function App(){
  const host=useExtensionHost()
  const workbench=useWorkbench(host.enabled,host.nav)
  const {page,setPage,toast,setToast,notify,busy,onScan,inventoryCounts,error,contentRef,setAddToolOpen,setImportOpen}=workbench
  useEffect(()=>{if(host.ready&&!host.nav.some(n=>n.id===page))setPage('extensions')},[host.ready,host.nav,page,setPage])
  const navigate=useCallback((id:string)=>{if(host.nav.some(n=>n.id===id))setPage(id);else notify('该功能扩展尚未启用，可在扩展管理中恢复')},[host.nav,setPage,notify])
  const item=host.nav.find(n=>n.id===page)
  return <div className={'app-shell workbench-layout-'+host.layout+' workbench-density-'+host.density}>
    {toast&&<Toast message={toast} onClose={()=>setToast(null)}/>}
    <aside className="sidebar"><div className="sidebar-scroll"><div className="brand"><img className="brand-badge" src={brandIcon} alt="" aria-hidden="true"/><span>ToolHub</span></div>
      <nav className="nav" aria-label="功能导航">{host.nav.map(n=><button key={n.id} className={'nav-btn '+(page===n.id?'active':'')} aria-current={page===n.id?'page':undefined} onClick={()=>navigate(n.id)}><span>{n.icon}</span><span>{n.label}</span></button>)}</nav>
      <div className="quick"><div className="quick-title">快速操作</div>
        {host.enabled.has('tools')&&<><button className="quick-btn" onClick={()=>{navigate('tools');setAddToolOpen(true)}}>＋ 添加工具</button><button className="quick-btn" disabled={busy} onClick={()=>void onScan()}>🔍 扫描本机</button></>}
        {host.enabled.has('market')&&<button className="quick-btn" onClick={()=>navigate('market')}>📦 模板与清单</button>}
        <button className="quick-btn" onClick={()=>{navigate('settings');setImportOpen(true)}}>⬇️ 导入配置</button>
        <button className="quick-btn" onClick={()=>navigate('extensions')}>◈ 管理扩展</button>
      </div></div>
      <div className="side-foot">{host.enabled.has('tools')?<><div className="muted">可用工具</div><strong>{inventoryCounts.available}</strong><div className="progress" title={`可用 ${inventoryCounts.available} / 已登记 ${inventoryCounts.registered}`}><i style={{width:`${inventoryCounts.registered?inventoryCounts.available/inventoryCounts.registered*100:0}%`}}/></div><div className="muted" style={{fontSize:12}}>已登记 {inventoryCounts.registered} 个安装实例<br/>不可用 {inventoryCounts.unavailable} · 共 {inventoryCounts.kinds} 种可用工具</div></>:<><div className="muted">已启用扩展</div><strong>{host.installed.filter(e=>e.enabled).length}</strong><p className="muted">工具管理扩展已停用，原有数据保留。</p></>}</div>
    </aside>
    <section className="content" ref={contentRef}>
      {(host.error||error)&&<div className="banner-error" role="alert"><span>{host.error||error}</span><button className="btn" onClick={()=>{void host.reload();void workbench.load()}}>重试连接</button></div>}
      {page==='extensions'?<Extensions notify={notify} onTools={()=>navigate('tools')} onLibrary={()=>navigate('skills')}/>:!host.ready?<div className="card card-pad">正在加载扩展工作台…</div>:item?.cli?<ExtensionCli key={item.id+item.extension?.digest} item={item} navigate={navigate}/>:item?.view?<ExtensionFrame key={item.id+item.extension?.digest} item={item} navigate={navigate} notify={notify}/>:item?<BuiltinPage id={item.id} context={workbench}/>:null}
    </section>
  </div>
}
