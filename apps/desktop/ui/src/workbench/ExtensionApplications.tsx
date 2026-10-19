import { useState } from 'react'
import type { NavItem } from './host'
import { isApplication } from './navigation'
import NavIcon from './NavIcon'

export default function ExtensionApplications({ nav, pins, navigate, togglePin }: { nav: NavItem[]; pins: string[]; navigate: (id: string) => void; togglePin: (id: string) => void }) {
  const [query, setQuery] = useState('')
  const [kind, setKind] = useState('all')
  const all = nav.filter(isApplication)
  const packages = new Map<string, NavItem[]>()
  all.filter(item => (kind === 'all' || (kind === 'cli' ? !!item.cli : !!item.view)) && `${item.label} ${item.extension!.package.name} ${item.extension!.package.description}`.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase())).forEach(item => {
    const key = item.extension!.package.id
    packages.set(key, [...(packages.get(key) || []), item])
  })
  return <>
    <div className="topbar"><div><h1>扩展应用</h1><p>按需打开应用，把常用能力放进自己的工作空间。</p></div><button className="btn" onClick={() => navigate('extensions')}>管理扩展</button></div>
    <section className="applications-intro"><div><span className="application-eyebrow">YOUR CAPABILITIES, TOGETHER</span><h2>一个工作空间，无限种可能。</h2><p>CLI 工具和独立页面集中在这里。主题与识别规则在后台生效，保持导航简洁。</p></div><div className="applications-total"><strong>{all.length}</strong><span>个可用应用页面</span></div></section>
    <div className="applications-toolbar"><label className="search-box"><NavIcon id="search"/><input aria-label="搜索扩展应用" placeholder="搜索应用名称、扩展或用途…" value={query} onChange={event => setQuery(event.target.value)}/></label><select aria-label="应用类型" value={kind} onChange={event => setKind(event.target.value)}><option value="all">全部应用</option><option value="cli">CLI 工具面板</option><option value="page">交互页面</option></select></div>
    <div className="applications-library">{[...packages].map(([id, items]) => <section className={'application-package' + (items.length > 1 ? ' application-package-multiple' : '')} key={id} aria-label={items[0].extension!.package.name}>
      <header><h3>{items[0].extension!.package.name}</h3><span>v{items[0].extension!.package.version}</span></header>
      <div className="application-grid">{items.map(item => <article className="application-card" key={item.id}>
        <div className="application-card-top"><span className="application-icon"><NavIcon id={item.id} fallback={item.icon} size={24}/></span><span className="application-type">{item.cli ? 'CLI 工具面板' : '交互页面'}</span><button className={'application-pin ' + (pins.includes(item.id) ? 'is-pinned' : '')} aria-label={`${pins.includes(item.id) ? '取消固定' : '固定到工作空间'}：${item.label}`} aria-pressed={pins.includes(item.id)} onClick={() => togglePin(item.id)}><NavIcon id="pin" size={17}/></button></div>
        <h4>{item.label}</h4><p>{item.extension!.package.description}</p>
        <button className="application-open" onClick={() => navigate(item.id)}>打开应用 <NavIcon id="arrow" size={17}/></button>
      </article>)}</div>
    </section>)}</div>
    {!packages.size && <div className="applications-empty card card-pad"><NavIcon id="extension-apps" size={36}/><h2>{all.length ? '没有找到匹配的应用' : '为工作空间添加第一项扩展能力'}</h2><p className="muted">{all.length ? '尝试其他关键词或应用类型。' : '安装并启用页面或 CLI 扩展后，就能在这里找到它们。'}</p><button className="btn" onClick={() => { if (all.length) { setQuery(''); setKind('all') } else navigate('extension-catalog') }}>{all.length ? '清除筛选' : '浏览扩展市场'}</button></div>}
  </>
}
