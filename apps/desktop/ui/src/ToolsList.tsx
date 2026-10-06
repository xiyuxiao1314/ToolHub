import { useEffect, useMemo, useRef, useState, type ReactNode } from 'react'
import GroupedTools, { groupTools } from './GroupedTools'
import ToolIcon from './ToolIcon'

const SIZES = [6, 12, 24]

export default function ToolsList({ items, grouped, selected, onSelect, loading, tags, tagClass, badge, trustLabel }: {
  items: any[]; grouped: boolean; selected: any; onSelect: (item: any) => void; loading: boolean
  tags: (name: string, path: string) => string[]; tagClass: (tag: string) => string; badge: (item: any) => ReactNode; trustLabel: (value: unknown) => string
}) {
  const [pageSize, setPageSize] = useState(() => {
    const saved = Number(localStorage.getItem('th.tools.pageSize'))
    return SIZES.includes(saved) ? saved : 6
  })
  const [currentPage, setCurrentPage] = useState(1)
  const browser = useRef<HTMLDivElement>(null)
  const groups = useMemo(() => groupTools(items), [items])
  const total = grouped ? groups.length : items.length
  const pageCount = Math.max(1, Math.ceil(total / pageSize))
  const page = Math.min(pageCount, currentPage)
  useEffect(() => { setCurrentPage(previous => Math.min(previous, pageCount)) }, [pageCount])
  const start = (page - 1) * pageSize
  const visible = grouped ? groups.slice(start, start + pageSize).flatMap(([, rows]) => rows) : items.slice(start, start + pageSize)
  const unit = grouped ? '种工具' : '个安装实例'
  const move = (next: number) => {
    setCurrentPage(Math.max(1, Math.min(pageCount, next)))
    browser.current?.scrollIntoView({ block: 'start' })
  }
  const resize = (size: number) => {
    if (!SIZES.includes(size)) return
    localStorage.setItem('th.tools.pageSize', String(size))
    setPageSize(size)
    setCurrentPage(1)
  }
  const pagination = (bottom = false) => <nav className="tool-pagination" aria-label={bottom ? '工具列表底部分页' : '工具列表分页'}>
    <span className="tool-page-range" role="status">第 {start + 1}–{Math.min(start + pageSize, total)} 项，共 {total} {unit}</span>
    <div className="tool-page-controls">
      {!bottom && <label>每页<select aria-label="工具每页数量" value={pageSize} onChange={event => resize(Number(event.target.value))}>{SIZES.map(size => <option key={size} value={size}>{size} 项</option>)}</select></label>}
      <button className="btn" aria-label="上一页" title="上一页" disabled={page === 1} onClick={() => move(page - 1)}>‹</button>
      <select aria-label="工具列表页码" value={page} onChange={event => move(Number(event.target.value))}>{Array.from({ length: pageCount }, (_, i) => <option key={i + 1} value={i + 1}>{i + 1} / {pageCount} 页</option>)}</select>
      <button className="btn" aria-label="下一页" title="下一页" disabled={page === pageCount} onClick={() => move(page + 1)}>›</button>
    </div>
  </nav>
  return <div className="tool-browser" ref={browser}>
    {!loading && total > 0 && pagination()}
    <div className="tool-list">
      {loading ? <div className="empty">加载中…</div> : total === 0 ? <div className="empty">没有匹配的工具</div> : grouped ? <GroupedTools items={visible} selected={selected} onSelect={onSelect} /> : visible.map((item, i) => <button key={item.id ?? i} className={`tool-item ${selected?.id === item.id ? 'active' : ''}`} onClick={() => onSelect(item)}>
        <ToolIcon name={String(item.name ?? '')} path={String(item.path ?? '')} version={String(item.version ?? '')} />
        <div><div className="tool-name">{String(item.name ?? '—')}{item.version ? ` ${item.version}` : ''}</div><div className="tool-desc">{String(item.path ?? '')}</div><div className="tags">{tags(String(item.name ?? ''), String(item.path ?? '')).map(tag => <span key={tag} className={tagClass(tag)}>{tag}</span>)}</div></div>
        <div className="tool-meta"><div className="ver">{item.version ? String(item.version) : '—'}</div>{badge(item)}<div className="muted" style={{ marginTop: 6, fontSize: 12 }}>{trustLabel(item.trust)}</div></div>
      </button>)}
    </div>
    {!loading && total > 0 && pagination(true)}
  </div>
}
