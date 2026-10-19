import { useCallback, useEffect, useState } from 'react'
import { useWorkbench, Toast } from './official/Workbench'
import { useExtensionHost } from './workbench/host'
import BuiltinPage from './workbench/BuiltinPage'
import ExtensionFrame from './workbench/ExtensionFrame'
import ExtensionCli from './workbench/ExtensionCli'
import Extensions from './Extensions'
import Navigation from './workbench/WorkbenchNavigation'
import ExtensionApplications from './workbench/ExtensionApplications'
import NavIcon from './workbench/NavIcon'
import { GROUPS, groupFor, groupItems, isApplication, readNavigationPrefs, writeNavigationPrefs, type GroupId } from './workbench/navigation'
import './workbench/workbench.css'

export default function App() {
  const host = useExtensionHost()
  const workbench = useWorkbench(host.enabled, host.nav)
  const { page, setPage, toast, setToast, notify, inventoryCounts, error, contentRef } = workbench
  const [preferences, setPreferences] = useState(readNavigationPrefs)
  const [group, setGroup] = useState<GroupId>('workspace')
  const item = host.nav.find(entry => entry.id === page)

  useEffect(() => { writeNavigationPrefs(preferences) }, [preferences])
  useEffect(() => {
    if (!host.ready) return
    const installedPages = new Set(host.installed.flatMap(entry => [entry.package.cli?.id, ...(entry.package.views ?? []).map(view => view.id)]).filter(Boolean))
    setPreferences(previous => previous.pins.every(id => installedPages.has(id)) ? previous : { ...previous, pins: previous.pins.filter(id => installedPages.has(id)) })
  }, [host.ready, host.installed])
  useEffect(() => {
    if (!host.ready) return
    if (!item) { setPage('extensions'); return }
    const inGroup = groupItems(host.nav, group, preferences.pins).some(entry => entry.id === page)
      || (group === 'extensions' && isApplication(item)) || (group === 'workspace' && page === 'workspace')
    const nextGroup = inGroup ? group : groupFor(item)
    if (nextGroup !== group) setGroup(nextGroup)
    setPreferences(previous => previous.last[nextGroup] === page ? previous : { ...previous, last: { ...previous.last, [nextGroup]: page } })
  }, [host.ready, host.nav, item, page, group, preferences.pins, setPage])

  const navigate = useCallback((id: string, destination?: GroupId) => {
    const target = host.nav.find(entry => entry.id === id)
    if (!target) { notify('该功能扩展尚未启用，可在扩展管理中恢复'); return }
    setGroup(destination ?? groupFor(target))
    setPage(id)
    if (window.matchMedia('(max-width:760px)').matches) setPreferences(previous => ({ ...previous, collapsed: true }))
  }, [host.nav, setPage, notify])
  const selectGroup = (destination: GroupId) => {
    const candidates = groupItems(host.nav, destination, preferences.pins)
    const last = preferences.last[destination]
    const previous = host.nav.find(entry => entry.id === last)
    const remembered = candidates.some(entry => entry.id === last) || (destination === 'extensions' && previous && isApplication(previous))
    navigate(remembered && last ? last : candidates[0]?.id ?? 'workspace', destination)
    if (window.matchMedia('(max-width:760px)').matches) setPreferences(previous => ({ ...previous, collapsed: false }))
  }
  const togglePin = (id: string) => {
    if (!host.nav.some(entry => entry.id === id && isApplication(entry))) return
    const pinned = preferences.pins.includes(id)
    if (!pinned && preferences.pins.length >= 8) { notify('最多固定 8 个常用扩展，请先取消一个'); return }
    setPreferences(previous => ({ ...previous, pins: pinned ? previous.pins.filter(pin => pin !== id) : [...previous.pins, id] }))
    notify(pinned ? '已从常用扩展中移除' : '已固定到工作空间')
  }
  const currentGroup = GROUPS.find(entry => entry.id === group)!
  const extensionSection = page === 'extension-catalog' ? 'catalog' : page === 'extension-appearance' ? 'appearance' : 'installed'

  return <div className={`app-shell hierarchical-shell workbench-layout-${host.layout} workbench-density-${host.density}${preferences.collapsed ? ' secondary-collapsed' : ''}`}>
    <a className="shell-skip" href="#workbench-main" onClick={() => contentRef.current?.focus()}>跳转到页面内容</a>
    {toast && <Toast message={toast} onClose={() => setToast(null)}/>}
    <Navigation nav={host.nav} page={page} group={group} prefs={preferences} navigate={navigate} selectGroup={selectGroup}
      toggleCollapsed={() => setPreferences(previous => ({ ...previous, collapsed: !previous.collapsed }))} togglePin={togglePin}
      counts={inventoryCounts} toolsEnabled={host.enabled.has('tools')} enabledCount={host.installed.filter(entry => entry.enabled).length}/>
    <main className="workbench-main" id="workbench-main">
      <header className="workspace-bar">
        <div className="workspace-location"><NavIcon id={group} size={16}/><span>{currentGroup.label}</span><span className="location-divider">/</span><strong>{item?.label || '正在加载'}</strong></div>
        <div className="workspace-bar-actions"><button className="secondary-toggle" aria-label={preferences.collapsed ? '展开二级导航' : '收起二级导航'} aria-controls="secondary-navigation" aria-expanded={!preferences.collapsed} onClick={() => setPreferences(previous => ({ ...previous, collapsed: !previous.collapsed }))}><NavIcon id="collapse" size={17}/><span>{preferences.collapsed ? '展开导航' : '收起导航'}</span></button><span className="workspace-local"><i/>本机工作台</span></div>
      </header>
      <section className="content" ref={contentRef} aria-label="页面内容" tabIndex={-1}>
        {(host.error || error) && <div className="banner-error" role="alert"><span>{host.error || error}</span><button className="btn" onClick={() => { void host.reload(); void workbench.load() }}>重试连接</button></div>}
        {!host.ready ? <div className="card card-pad">正在加载扩展工作台…</div>
          : page === 'workspace' ? <div className="applications-empty card card-pad"><NavIcon id="workspace" size={36}/><h2>工作空间由你定义</h2><p className="muted">启用官方功能，或把扩展应用固定到这里。</p><button className="btn btn-primary" onClick={() => navigate('extensions')}>管理扩展</button></div>
          : page === 'extension-apps' ? <ExtensionApplications nav={host.nav} pins={preferences.pins} navigate={navigate} togglePin={togglePin}/>
          : ['extensions', 'extension-catalog', 'extension-appearance'].includes(page) ? <Extensions key={extensionSection} section={extensionSection} notify={notify} onTools={() => navigate('tools')} onLibrary={() => navigate('skills')}/>
          : page === 'agent-guide' ? <BuiltinPage id="agents" context={workbench} guide/>
          : item?.cli ? <ExtensionCli key={item.id + item.extension?.digest} item={item} navigate={navigate}/>
          : item?.view ? <ExtensionFrame key={item.id + item.extension?.digest} item={item} navigate={navigate} notify={notify}/>
          : item ? <BuiltinPage id={item.id} context={workbench}/> : null}
      </section>
    </main>
  </div>
}
