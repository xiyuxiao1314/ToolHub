import { useEffect, useMemo, useRef, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'

type Entry = {
  id?: string; selection_id?: string; cwd_selection_id?: string
  kind: 'file' | 'command'; name: string; path: string; cwd: string; command: string
  args: string[]; favorite: boolean; evidence: string[]; available?: boolean
  last_launched?: string; launch_count?: number
}
type Candidate = Entry & { id: string }
type Scan = { id?: string; status: string; roots?: string[]; visited?: number; skipped?: number; unreadable?: number; current?: string; limitations?: string[]; total: number; candidates: Candidate[] }
type Draft = Entry & { argText: string }
const rpc = <T,>(method: string, params: unknown = {}) => invoke<T>('rpc', { method, params })
const key = (entry: Entry) => entry.kind === 'file' ? `file:${entry.path.toLowerCase().replace(/\\/g, '/')}` : `command:${entry.cwd.toLowerCase().replace(/\\/g, '/')}:${entry.command}`
const draft = (entry: Entry): Draft => ({ ...entry, argText: entry.args?.join('\n') || '' })
const patch = (entry: Entry) => ({ id: entry.id, selection_id: entry.selection_id, cwd_selection_id: entry.cwd_selection_id, name: entry.name, command: entry.command, args: entry.args, favorite: entry.favorite })

export default function Programs({ notify }: { notify: (message: string) => void }) {
  const [entries, setEntries] = useState<Entry[]>([])
  const [scan, setScan] = useState<Scan>({ status: 'idle', total: 0, candidates: [] })
  const [candidates, setCandidates] = useState<Candidate[]>([])
  const [chosen, setChosen] = useState<string[]>([])
  const [drafts, setDrafts] = useState<Draft[]>([])
  const [addOpen, setAddOpen] = useState(false)
  const [scanOpen, setScanOpen] = useState(false)
  const [query, setQuery] = useState('')
  const [candidateQuery, setCandidateQuery] = useState('')
  const [favoritesOnly, setFavoritesOnly] = useState(false)
  const [busy, setBusy] = useState(false)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')
  const [launching, setLaunching] = useState<string[]>([])
  const scanId = useRef<string>()
  const pollSeq = useRef(0)
  const searchRef = useRef<HTMLInputElement>(null)
  const editorRef = useRef<HTMLElement>(null)
  const mounted = useRef(true)
  const fail = (e: unknown) => { if (mounted.current) { const message = String(e); setError(message); notify(message) } }
  const load = async () => {
    const rows = await rpc<Entry[]>('program.list')
    if (mounted.current) setEntries(rows)
  }
  const poll = async () => {
    const seq = ++pollSeq.current
    const report = await rpc<Scan>('program.scan_status')
    if (!mounted.current || seq !== pollSeq.current) return
    if (report.id !== scanId.current) { scanId.current = report.id; setCandidates(report.candidates); setChosen([]) }
    else setCandidates((previous) => { const existing = new Map(previous.map((c) => [c.id, c])); report.candidates.forEach((c) => existing.set(c.id, c)); return [...existing.values()] })
    setScan(report)
    if (report.status !== 'idle') setScanOpen(true)
  }
  useEffect(() => {
    mounted.current = true
    void Promise.all([load(), poll()]).catch(fail).finally(() => { if (mounted.current) setLoading(false) })
    const focus = (event: KeyboardEvent) => { if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') { event.preventDefault(); searchRef.current?.focus() } }
    window.addEventListener('keydown', focus)
    return () => { mounted.current = false; window.removeEventListener('keydown', focus) }
  }, [])
  useEffect(() => {
    if (scan.status !== 'running') return
    let pending = false
    const timer = setInterval(() => { if (pending) return; pending = true; void poll().catch(fail).finally(() => { pending = false }) }, 900)
    return () => clearInterval(timer)
  }, [scan.status])

  useEffect(() => {
    if (drafts.length === 0) return
    const frame = requestAnimationFrame(() => {
      editorRef.current?.scrollIntoView({ block: 'start', behavior: window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth' })
      editorRef.current?.querySelector<HTMLInputElement>('input')?.focus({ preventScroll: true })
    })
    return () => cancelAnimationFrame(frame)
  }, [drafts.length])

  const act = async (fn: () => Promise<void>) => {
    if (busy) return
    setBusy(true); setError('')
    try { await fn() } catch (e) { fail(e) } finally { if (mounted.current) setBusy(false) }
  }
  const pick = async (kind: 'file' | 'command'): Promise<Entry | undefined> => {
    const path = await invoke<string | null>('pick_program_path', { directory: kind === 'command' })
    if (!path) return
    const selected = await rpc<Entry[]>('program.select', { path, kind })
    return selected[0]
  }
  const add = (kind: 'file' | 'command') => act(async () => {
    const selected = await pick(kind)
    if (selected) { setDrafts([draft(selected)]); setAddOpen(false) }
  })
  const startScan = (directory = false) => act(async () => {
    let roots: string[] | undefined
    if (directory) { const path = await invoke<string | null>('pick_program_path', { directory: true }); if (!path) return; roots = [path] }
    await rpc('program.scan_start', roots ? { roots } : {})
    setCandidates([]); setChosen([]); setCandidateQuery(''); setScanOpen(true)
    await poll()
  })
  const selectCandidates = () => act(async () => {
    const selected = await rpc<Entry[]>('program.select', { candidate_ids: chosen })
    setDrafts(selected.map(draft))
    setChosen([])
  })
  const save = () => act(async () => {
    await rpc('program.save', { items: drafts.map((d) => patch({ ...d, args: d.kind === 'file' ? d.argText.split('\n').filter((s) => s.length > 0) : [] })) })
    setDrafts([])
    await load()
    notify('已保存选中的程序')
  })
  const updateDraft = (index: number, values: Partial<Draft>) => setDrafts((rows) => rows.map((row, i) => i === index ? { ...row, ...values } : row))
  const changePath = (index: number, workingDirectory: boolean) => act(async () => {
    const selected = await pick(workingDirectory ? 'command' : drafts[index].kind)
    if (!selected) return
    if (workingDirectory) updateDraft(index, { cwd: selected.cwd, cwd_selection_id: selected.selection_id })
    else updateDraft(index, { selection_id: selected.selection_id, cwd_selection_id: undefined, path: selected.path, cwd: selected.cwd, evidence: selected.evidence })
  })
  const launch = async (entry: Entry) => {
    if (!entry.id || launching.includes(entry.id)) return
    const id = entry.id
    setLaunching((ids) => [...ids, id]); setError('')
    try { const result = await rpc<{ pid: number }>('program.launch', { id }); notify(`已提交启动请求：${entry.name}（PID ${result.pid}）`); await load() }
    catch (e) { fail(e) } finally { if (mounted.current) setLaunching((ids) => ids.filter((value) => value !== id)) }
  }
  const saved = useMemo(() => new Set(entries.map(key)), [entries])
  const shown = useMemo(() => entries.filter((entry) => (!favoritesOnly || entry.favorite) && `${entry.name} ${entry.path} ${entry.cwd} ${entry.command}`.toLowerCase().replace(/\\/g, '/').includes(query.trim().toLowerCase().replace(/\\/g, '/'))).sort((a, b) => Number(b.favorite) - Number(a.favorite) || a.name.localeCompare(b.name, 'zh-CN')), [entries, favoritesOnly, query])
  const availableCandidates = candidates.filter((c) => !saved.has(key(c)))
  const shownCandidates = availableCandidates.filter((c) => `${c.name} ${c.path} ${c.cwd} ${c.command} ${c.evidence.join(' ')}`.toLowerCase().includes(candidateQuery.trim().toLowerCase()))
  const statusText: Record<string, string> = { idle: '等待扫描', running: '正在扫描', completed: '扫描完成', partial: '扫描结束，部分范围未覆盖', cancelled: '已取消，保留已发现候选', failed: '扫描失败' }

  return <>
    <div className="topbar program-topbar">
      <div><h1>程序</h1><p>把散落的项目启动入口，收进你的程序架。</p></div>
      <div className="program-actions">
        <button className="btn" disabled={busy || scan.status === 'running'} onClick={() => void startScan()}>扫描所有磁盘</button>
        <button className="btn btn-primary" disabled={busy || drafts.length > 0} onClick={() => setAddOpen(!addOpen)}>＋ 添加程序</button>
      </div>
    </div>
    <div className="layout-1 program-page">
      {error && <div className="error-box" role="alert">{error}<button className="program-dismiss" aria-label="关闭程序错误" onClick={() => setError('')}>×</button></div>}
      <div className="program-summary">
        <div><span className="program-eyebrow">我的启动入口</span><strong>{entries.length}<small> 个程序</small></strong></div>
        <div><span className="muted">收藏</span><strong>{entries.filter((e) => e.favorite).length}</strong></div>
        <div><span className="muted">路径失效</span><strong>{entries.filter((e) => !e.available).length}</strong></div>
        <p>支持 EXE、BAT、CMD、LNK、PS1 与自定义命令。扫描提供候选，由你选择收录。</p>
      </div>

      {addOpen && <section className="card card-pad program-add">
        <div><h3>先选入口，再填写信息</h3><p className="muted">文件入口选择启动文件；命令入口先选择工作目录，再填写命令。</p></div>
        <div className="program-actions"><button className="btn" disabled={busy} onClick={() => void add('file')}>选择启动文件</button><button className="btn" disabled={busy} onClick={() => void add('command')}>选择命令工作目录</button><button className="btn" disabled={busy} onClick={() => setAddOpen(false)}>取消添加</button></div>
      </section>}

      {drafts.length > 0 && <section ref={editorRef} className="card card-pad program-editor" aria-label="程序信息编辑">
        <div className="program-section-head"><div><h3>{drafts[0].id ? '编辑程序' : `填写已选中的 ${drafts.length} 个入口`}</h3><p className="muted">路径来自你的选择。保存前可以检查名称、参数和工作目录。</p></div><span className="tag blue">用户已选择</span></div>
        {drafts.map((d, i) => <div className="program-draft" key={d.id || d.selection_id}>
          <label>程序名称<input className="select" aria-label={`程序名称 ${i + 1}`} value={d.name} maxLength={128} disabled={busy} onChange={(e) => updateDraft(i, { name: e.target.value })}/></label>
          {d.kind === 'file' && <label>启动文件<div className="program-field-row"><input className="select" aria-label={`启动文件 ${i + 1}`} value={d.path} readOnly/><button className="btn" disabled={busy} onClick={() => void changePath(i, false)}>重新选择文件</button></div></label>}
          <label>工作目录<div className="program-field-row"><input className="select" aria-label={`工作目录 ${i + 1}`} value={d.cwd} readOnly/><button className="btn" disabled={busy} onClick={() => void changePath(i, true)}>选择工作目录</button></div></label>
          {d.kind === 'command' ? <label>启动命令<textarea className="select program-command" aria-label={`启动命令 ${i + 1}`} rows={2} value={d.command} disabled={busy} placeholder="例如 npm run dev 或 python app.py" onChange={(e) => updateDraft(i, { command: e.target.value })}/><span className="muted">使用 CMD 执行，启动窗口会保留供你查看输出。</span></label> : !d.path.toLowerCase().endsWith('.lnk') ? <label>启动参数（每行一个参数，不必加引号）<textarea className="select program-command" aria-label={`启动参数 ${i + 1}`} rows={2} value={d.argText} disabled={busy} placeholder={'--port\n8080'} onChange={(e) => updateDraft(i, { argText: e.target.value })}/></label> : <p className="muted">快捷方式使用其自身配置的目标、参数和工作目录。</p>}
          <label className="program-check"><input type="checkbox" checked={d.favorite} disabled={busy} onChange={(e) => updateDraft(i, { favorite: e.target.checked })}/>加入收藏</label>
          <div className="muted program-evidence">{d.evidence.join(' · ')}</div>
        </div>)}
        <div className="program-actions"><button className="btn btn-primary" disabled={busy || drafts.some((d) => !d.name.trim() || (d.kind === 'command' && !d.command.trim()))} onClick={() => void save()}>{busy ? '处理中…' : '保存程序'}</button><button className="btn" disabled={busy} onClick={() => setDrafts([])}>取消填写</button></div>
      </section>}

      <div className="program-toolbar">
        <div className="search-box"><span>⌕</span><input ref={searchRef} aria-label="搜索程序" placeholder="搜索程序名称、命令或路径…" value={query} onChange={(e) => setQuery(e.target.value)}/><span className="kbd">Ctrl K</span></div>
        <button className={`chip ${favoritesOnly ? 'active' : ''}`} onClick={() => setFavoritesOnly(!favoritesOnly)}>★ 只看收藏</button>
        <button className="btn" disabled={busy} onClick={() => void act(async () => { await load(); notify('程序列表已刷新') })}>刷新程序</button>
      </div>
      <div className="program-grid">
        {loading && <div className="empty">加载程序列表…</div>}
        {!loading && shown.length === 0 && <div className="card program-empty"><span>↗</span><h3>{entries.length ? '没有匹配的程序' : '把第一个程序放上来'}</h3><p className="muted">选择一个启动文件或工作目录，也可以扫描磁盘后勾选候选。</p></div>}
        {shown.map((entry) => <article key={entry.id} className="card program-card" data-program-id={entry.id}>
          <div className="program-card-head"><span className={`program-symbol ${entry.kind === 'command' ? 'command' : ''}`}>{entry.kind === 'command' ? '>_' : '↗'}</span><div><h3>{entry.name}</h3><span className={`badge badge-${entry.available ? 'ok' : 'warn'}`}>{entry.available ? (entry.kind === 'command' ? '命令入口' : entry.path.split('.').pop()?.toUpperCase()) : '路径失效'}</span></div><button className={`program-star ${entry.favorite ? 'active' : ''}`} aria-label={`${entry.favorite ? '取消收藏' : '收藏'} ${entry.name}`} disabled={busy} onClick={() => void act(async () => { await rpc('program.save', { items: [patch({ ...entry, favorite: !entry.favorite })] }); await load() })}>{entry.favorite ? '★' : '☆'}</button></div>
          <code className="program-entry-path">{entry.kind === 'command' ? entry.command : entry.path}</code>
          <div className="program-working-dir"><span className="muted">工作目录</span><code>{entry.cwd}</code></div>
          {entry.args.length > 0 && <div className="program-evidence muted">参数：{entry.args.map((a) => JSON.stringify(a)).join(' ')}</div>}
          <div className="program-last muted">{entry.last_launched ? `上次启动 ${new Date(entry.last_launched).toLocaleString('zh-CN', { hour12: false })}` : '尚未启动'}</div>
          <div className="program-actions"><button className="btn btn-primary" disabled={!entry.available || launching.includes(entry.id!)} onClick={() => void launch(entry)}>{launching.includes(entry.id!) ? '启动中…' : '启动'}</button><button className="btn" onClick={() => void act(async () => { await invoke('open_program_folder', { path: entry.kind === 'file' ? entry.path.slice(0, Math.max(entry.path.lastIndexOf('/'), entry.path.lastIndexOf('\\')) + 1) : entry.cwd }); notify('已打开程序所在目录') })} disabled={busy}>打开目录</button><button className="btn" disabled={busy || drafts.length > 0} onClick={() => { setDrafts([draft(entry)]); setError('') }}>编辑</button><button className="program-remove" disabled={busy} onClick={() => void act(async () => { await rpc('program.remove', { id: entry.id }); await load(); notify('已移除程序条目，磁盘文件保留') })}>移除</button></div>
        </article>)}
      </div>

      <section className="card card-pad program-scan">
        <div className="program-section-head"><div><h3>扫描候选 <span className="muted">{scan.total || 0}</span></h3><p className="muted">按项目标记、打包目录和启动文件筛选，无法仅凭文件证明 AI 来源。</p></div><button className="btn" onClick={() => setScanOpen(!scanOpen)}>{scanOpen ? '收起候选' : '展开候选'}</button></div>
        <div className="program-actions"><button className="btn" disabled={busy || scan.status === 'running'} onClick={() => void startScan()}>重新扫描所有磁盘</button><button className="btn" disabled={busy || scan.status === 'running'} onClick={() => void startScan(true)}>选择目录扫描</button>{scan.status === 'running' && <button className="btn" disabled={busy} onClick={() => void act(async () => { await rpc('program.scan_cancel'); await poll() })}>取消扫描</button>}</div>
        {scan.status !== 'idle' && <div className="program-scan-progress" aria-live="polite"><strong>{statusText[scan.status] || scan.status}</strong><span className="muted">已检查 {(scan.visited || 0).toLocaleString()} 项 · 跳过 {scan.skipped || 0} 个目录或链接</span><code>{scan.current || scan.roots?.join(' · ')}</code>{scan.limitations?.map((limit) => <span className="program-limit" key={limit}>{limit}</span>)}</div>}
        {scanOpen && <>
          <p className="muted program-filter-note">过滤 Windows、Program Files、已登记的安装目录、AppData、node_modules、虚拟环境、缓存和编译中间文件。保留 dist、build 与 target 下的主程序；扫描不会运行候选。</p>
          <div className="program-toolbar"><div className="search-box"><input aria-label="搜索扫描候选" value={candidateQuery} placeholder="筛选已加载的候选…" onChange={(e) => setCandidateQuery(e.target.value)}/></div><button className="btn btn-primary" disabled={busy || chosen.length === 0 || drafts.length > 0} onClick={() => void selectCandidates()}>填写选中的入口 ({chosen.length})</button></div>
          <div className="program-candidates">
            {shownCandidates.map((c) => <label className={`program-candidate ${chosen.includes(c.id) ? 'selected' : ''}`} key={c.id}><input type="checkbox" aria-label={`选择 ${c.name}`} disabled={busy || (chosen.length >= 100 && !chosen.includes(c.id))} checked={chosen.includes(c.id)} onChange={(e) => setChosen((ids) => e.target.checked ? [...ids, c.id] : ids.filter((id) => id !== c.id))}/><div><strong>{c.name}</strong><span className="tag">{c.kind === 'command' ? '命令' : c.path.split('.').pop()?.toUpperCase()}</span><code>{c.kind === 'command' ? `${c.command} · ${c.cwd}` : c.path}</code><span className="muted program-evidence">{c.evidence.join(' · ')}</span></div></label>)}
            {shownCandidates.length === 0 && <div className="empty">{scan.status === 'running' ? '正在查找项目启动入口…' : candidates.length ? '当前候选已添加或没有匹配项' : '尚无候选。可以扫描磁盘，或手动选择启动文件。'}</div>}
          </div>
          {candidates.length < scan.total && <button className="btn" disabled={busy} onClick={() => void act(async () => { const more = await rpc<Scan>('program.scan_status', { offset: candidates.length, limit: 100 }); setCandidates((rows) => [...rows, ...more.candidates]); setScan(more) })}>加载更多候选（已加载 {candidates.length} / {scan.total}）</button>}
        </>}
      </section>
    </div>
  </>
}
