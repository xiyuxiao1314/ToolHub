import { useEffect, useState } from 'react'
import brandIcon from '../assets/toolhub-a.png'
import type { NavItem } from './host'
import { GROUPS, groupItems, isApplication, type GroupId, type NavigationPrefs } from './navigation'
import NavIcon from './NavIcon'

type Props = {
  nav: NavItem[]; page: string; group: GroupId; prefs: NavigationPrefs;
  navigate: (id: string, group?: GroupId) => void;
  selectGroup: (id: GroupId) => void;
  toggleCollapsed: () => void; togglePin: (id: string) => void;
  counts: { available: number; registered: number; unavailable: number; kinds: number };
  toolsEnabled: boolean; enabledCount: number;
}
export default function Navigation(props: Props) {
  const { nav, page, group, prefs, navigate, selectGroup, counts } = props
  const [query, setQuery] = useState('')
  useEffect(() => { setQuery('') }, [group])
  const info = GROUPS.find(item => item.id === group)!
  const matches = (item: NavItem) => `${item.label} ${item.extension?.package.name ?? ''}`.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase())
  const entries = groupItems(nav, group, prefs.pins).filter(matches)
  const apps = group === 'extensions' ? nav.filter(isApplication).filter(matches) : []
  const packages = new Map<string, NavItem[]>()
  apps.forEach(item => {
    const key = item.extension!.package.id
    packages.set(key, [...(packages.get(key) || []), item])
  })
  const row = (item: NavItem) => <div className={'secondary-row ' + (page === item.id ? 'is-current' : '')} key={item.id}>
    <button className="secondary-link" aria-current={page === item.id ? 'page' : undefined} title={item.label} onClick={() => navigate(item.id, group)}>
      <NavIcon id={item.id} fallback={item.icon}/><span>{item.id === 'market' && item.label === '市场' ? '能力包目录' : item.label}</span>
    </button>
    {isApplication(item) && <button className={'navigation-pin ' + (prefs.pins.includes(item.id) ? 'is-pinned' : '')} aria-label={`${prefs.pins.includes(item.id) ? '取消固定' : '固定到工作空间'}：${item.label}`} aria-pressed={prefs.pins.includes(item.id)} onClick={() => props.togglePin(item.id)}><NavIcon id="pin" size={14}/></button>}
  </div>
  return <>
    <aside className="primary-rail" aria-label="主导航栏">
      <button className="rail-brand" title="ToolHub · 返回工作空间" aria-label="ToolHub 工作空间" onClick={() => selectGroup('workspace')}><img src={brandIcon} alt=""/></button>
      <nav aria-label="一级导航" className="primary-links">{GROUPS.map(item => <button key={item.id} className={'primary-link ' + (group === item.id ? 'is-active' : '')} aria-current={group === item.id ? 'true' : undefined} onClick={() => selectGroup(item.id)}><NavIcon id={item.id} size={22}/><span>{item.label}</span></button>)}</nav>
      <button className="rail-collapse" aria-controls="secondary-navigation" aria-expanded={!prefs.collapsed} aria-label={prefs.collapsed ? '展开二级导航' : '收起二级导航'} title={prefs.collapsed ? '展开二级导航' : '收起二级导航'} onClick={props.toggleCollapsed}><NavIcon id="collapse"/></button>
    </aside>
    <aside id="secondary-navigation" className="secondary-panel" hidden={prefs.collapsed} aria-label={`${info.label}二级导航`}>
      <header className="secondary-heading"><span className="shell-wordmark">ToolHub<span className="shell-wordmark-dot"/></span><h2>{info.label}</h2><p>{info.description}</p></header>
      <label className="navigation-search"><NavIcon id="search" size={16}/><input aria-label="筛选二级导航" placeholder="查找功能…" value={query} onChange={event => setQuery(event.target.value)}/>{query && <button aria-label="清除导航搜索" onClick={() => setQuery('')}>×</button>}</label>
      <div className="secondary-scroll"><nav aria-label="二级导航">
        <div className="navigation-section-label">{group === 'workspace' ? '我的工作空间' : group === 'extensions' ? '管理与发现' : '功能'}</div>
        {entries.filter(item => !isApplication(item)).map(row)}
        {group === 'workspace' && <>
          <div className="navigation-section-label navigation-section-spaced">常用扩展 <span>{entries.filter(isApplication).length}/8</span></div>
          {entries.filter(isApplication).map(row)}
          {!entries.some(isApplication) && <p className="navigation-hint">{query ? '没有匹配的常用扩展' : '将常用应用固定在这里，随时打开。'}</p>}
          <button className="navigation-add" onClick={() => navigate('extension-apps', 'extensions')}>＋ 浏览扩展应用</button>
        </>}
        {group === 'extensions' && <>
          <div className="navigation-section-label navigation-section-spaced">应用页面 <span>{apps.length}</span></div>
          {[...packages].map(([id, items]) => <details className="navigation-package" key={id} open={query.trim() || items.some(item => item.id === page) ? true : undefined}>
            <summary title={items[0].extension!.package.name}><span>{items[0].extension!.package.name}</span><small>{items.length}</small></summary>{items.map(row)}
          </details>)}
          {!apps.length && <p className="navigation-hint">{query ? '没有匹配的应用页面' : '启用页面或 CLI 扩展后显示在这里。'}</p>}
        </>}
        {!entries.length && group !== 'extensions' && group !== 'workspace' && <p className="navigation-hint">没有匹配的功能</p>}
      </nav></div>
      <footer className="navigation-status">{props.toolsEnabled ? <>
        <div><span><i/>可用工具</span><strong>{counts.available}<small> / {counts.registered}</small></strong></div>
        <p>{counts.kinds} 种工具 · {counts.unavailable} 个不可用实例</p>
      </> : <><div><span><i/>已启用扩展</span><strong>{props.enabledCount}</strong></div><p>工具模块已停用，数据仍然保留。</p></>}</footer>
    </aside>
  </>
}
