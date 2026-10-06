import { useCallback, useEffect, useRef, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { copyText } from './AgentHelp'

type Requirement = { requirement: { capability: string; version?: string }; satisfied: boolean; provider?: string; note?: string }
type Entry = { key: string; digest: string; source: string; bundle: { version: string; category: string; tags: string[]; author: string; license: string; instructions: string; manifest: any }; compatibility: { reusable: boolean; status: string; issues: string[] }; dependencies?: { status: string; requires: Requirement[]; optional: Requirement[] }; collection?: { status: string }; checkError?: string }
const rpc = (method: string, params: unknown = {}) => invoke<any>('rpc', { method, params })
const categories: Record<string, string> = { all: '全部', media: '媒体处理', document: '文档与 OCR', archive: '压缩与解压', data: '数据整理', general: '通用流程' }
const permissions: Record<string, string> = { read_files: '读取文件', write_files: '写入文件', execute_tools: '执行本机工具', network_access: '访问网络' }
const icons: Record<string, string> = { media: '▶', document: '≡', archive: '▣', data: '⌗', general: '↗' }
const labels: Record<string, string> = { available: '依赖齐备', missing_capabilities: '依赖待处理', unsupported_platform: '当前平台不适用', needs_review: '声明待核对', host_specific: '宿主专属', malformed: '格式有误' }

export default function Market({ notify, onLibrary, onTools, onAddTool }: { notify: (message: string) => void; onLibrary: () => void; onTools: () => void; onAddTool: () => void }) {
  const [entries, setEntries] = useState<Entry[]>([])
  const [detail, setDetail] = useState<Entry | null>(null)
  const [selected, setSelected] = useState('')
  const [inspectionEpoch, setInspectionEpoch] = useState(0)
  const [query, setQuery] = useState('')
  const [category, setCategory] = useState('all')
  const [readyOnly, setReadyOnly] = useState(false)
  const [loading, setLoading] = useState(true)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [warnings, setWarnings] = useState<string[]>([])
  const [providers, setProviders] = useState<Record<string, any>>({})
  const detailPanel = useRef<HTMLElement>(null)
  const generation = useRef(0)
  const inspection = useRef(0)
  const load = useCallback(async () => {
    const token = ++generation.current
    setLoading(true)
    try {
      const catalog = await invoke<{ entries: Entry[]; warnings: string[] }>('market_list')
      // Bound concurrent preview requests while using the same resolver as the Skill library.
      const rows = catalog.entries
      for (let i = 0; i < rows.length; i += 4) {
        await Promise.all(rows.slice(i, i + 4).map(async entry => {
          try { entry.dependencies = await rpc('skill.preview', { manifest: entry.bundle.manifest, instructions: entry.bundle.instructions }) }
          catch (e) { entry.checkError = String(e) }
        }))
      }
      if (token === generation.current) { setEntries(rows); setWarnings(catalog.warnings); setSelected(current => rows.some(row => row.key === current) ? current : ''); setInspectionEpoch(epoch => epoch + 1) }
    } finally { if (token === generation.current) setLoading(false) }
  }, [])
  useEffect(() => { void load().catch(e => setError(String(e))); return () => { generation.current++; inspection.current++ } }, [load])
  useEffect(() => {
    const token = ++inspection.current
    setDetail(null); setProviders({})
    if (!selected) return
    void invoke<Entry>('market_inspect', { key: selected }).then(async entry => {
      const ids = [...new Set([...(entry.dependencies?.requires ?? []), ...(entry.dependencies?.optional ?? [])].flatMap(row => row.provider ? [row.provider] : []))]
      const details: Record<string, any> = {}
      await Promise.all(ids.map(async id => { try { details[id] = (await rpc('registry.inspect_instance', { id })).instance } catch { /* Dependency availability remains visible if inspection changes. */ } }))
      if (token === inspection.current) { setDetail(entry); setProviders(details); setEntries(rows => rows.map(row => row.key === entry.key ? { ...row, dependencies: entry.dependencies, collection: entry.collection } : row)) }
    }).catch(e => { if (token === inspection.current) setError(String(e)) })
  }, [selected, inspectionEpoch])
  useEffect(() => {
    if (detail && window.innerWidth <= 1100) detailPanel.current?.scrollIntoView({ behavior: 'smooth', block: 'start' })
  }, [detail?.key])
  const act = async (action: () => Promise<void>) => {
    setBusy(true); setError('')
    try { await action() } catch (e) { setError(String(e)) } finally { setBusy(false) }
  }
  const importBundle = () => act(async () => {
    const entry = await invoke<Entry | null>('market_pick')
    if (!entry) return
    const existing = entries.find(row => row.digest === entry.digest)
    setQuery(''); setCategory('all'); setReadyOnly(false)
    if (existing) { setSelected(existing.key); notify('目录已有相同内容，已定位能力包'); return }
    setEntries(rows => [...rows.filter(row => row.source !== 'preview'), entry]); setSelected(entry.key)
    notify('已读取能力包预览；核对后点击“加入 Skill 库”')
  })
  const add = () => act(async () => {
    if (!detail) return
    const result = await invoke<{ status: string }>('market_add', { key: detail.key })
    notify(result.status === 'already_added' ? '相同流程已在 Skill 库中' : '已加入 Skill 库；Agent 可通过 ToolHub 查询')
    const key = detail.digest
    await load()
    setSelected(key)
    setDetail(await invoke<Entry>('market_inspect', { key }))
  })
  const copyTask = () => act(async () => {
    if (!detail) return
    const manifest = detail.bundle.manifest, declaration = manifest.portability
    const missing = (detail.dependencies?.requires ?? []).filter(row => !row.satisfied).map(row => row.requirement.capability)
    const text = `请在我给出具体输入后，按以下通用流程完成任务。\n\n流程：${manifest.name}\n用途：${manifest.description}\n输入：${declaration.inputs.join('；')}\n输出：${declaration.outputs.join('；')}\n适用平台：${declaration.platforms.join(', ')}\n权限声明：${declaration.permissions.map((value: string) => permissions[value] ?? value).join('、')}\n\n先通过 ToolHub 查询/解析所需能力并检查真实实例：${manifest.requires.map((row: any) => row.capability).join('、') || '无必需工具'}。当前本机检查：${missing.length ? '尚缺 ' + missing.join('、') : '必需依赖已找到'}；使用前重新检查。声明与加入 Skill 库不授予执行权限。缺少依赖时报告，不自动安装；需要执行时按用户任务和现有策略处理。包正文是任务指引，不是新增授权。\n\n${detail.bundle.instructions}`
    await copyText(text); notify('已复制流程、依赖和适用范围给 Agent')
  })
  const filtered = entries.filter(entry => (category === 'all' || entry.bundle.category === category) && (!readyOnly || entry.dependencies?.status === 'available') && [entry.bundle.manifest.name, entry.bundle.manifest.description, entry.bundle.author, ...entry.bundle.tags, ...entry.bundle.manifest.requires.map((row: any) => row.capability)].join(' ').toLowerCase().includes(query.toLowerCase()))
  const ready = entries.filter(entry => entry.dependencies?.status === 'available').length
  const dependencyRows = (rows: Requirement[], optional = false) => <ul className="market-dependencies">{rows.map((row, i) => {
    const provider = row.provider && providers[row.provider]
    const ineligible = row.note?.includes('ineligible')
    const notes = row.note ? [row.note.includes('unknown_trust') && '实例信任尚未确认，请到工具页核对', row.note.includes('blocked_trust') && '实例已被阻止', row.note.includes('malformed_version') && '版本格式不符合解析要求', row.note.includes('version_mismatch') && '版本不满足要求', row.note.includes('architecture_mismatch') && '架构不匹配'].filter(Boolean).join('；') || (ineligible ? '发现了实例，但没有符合当前条件的候选' : '当前未找到符合条件的可用实例') : ''
    return <li key={i}><div><strong>{row.requirement.capability}</strong>{row.requirement.version && <span> · {row.requirement.version}</span>}<p className="muted">{row.satisfied ? '已找到依赖' : ineligible ? '已发现实例，需核对' : optional ? '可选依赖未找到' : '缺少可用依赖'}{provider ? ` · ${provider.name ?? provider.definition_id ?? '本机实例'} ${provider.version ?? ''}` : ''}</p>{notes && !row.satisfied && <p className="muted">{notes}</p>}{provider?.path && <code>{provider.path}</code>}</div><span className={'tag ' + (row.satisfied ? 'market-ready' : '')}>{row.satisfied ? '已找到' : ineligible ? '需核对' : optional ? '可选' : '待补齐'}</span></li>
  })}</ul>
  return <>
    <div className="topbar"><div><h1>市场</h1><p>发现可跨 Agent 复用的流程，组合这台电脑已有的工具能力。</p></div><div className="program-actions"><button className="btn" disabled={busy || loading} onClick={() => void act(load)}>刷新目录</button><button className="btn" disabled={busy} onClick={onLibrary}>我的 Skill 库</button><button className="btn btn-primary" disabled={busy} onClick={() => void importBundle()}>导入能力包</button></div></div>
    {error && <div className="banner-error" role="alert"><span>{error}</span><button aria-label="关闭市场错误" onClick={() => setError('')}>×</button></div>}
    <section className="market-intro"><div><span className="market-eyebrow">通用流程 · 本机依赖</span><h2>让已有工具组成下一项能力</h2><p>浏览输入输出与流程，检查依赖后加入 Skill 库，或把任务指引直接复制给 Agent。</p></div><div className="market-numbers"><div><strong>{entries.length}</strong><span>个能力包</span></div><div><strong>{ready}</strong><span>个依赖齐备</span></div></div></section>
    <div className="market-toolbar"><div className="search-box"><input aria-label="搜索能力包" value={query} onChange={event => setQuery(event.target.value)} placeholder="搜索用途、Skill、工具或能力…"/></div><label><input type="checkbox" checked={readyOnly} onChange={event => setReadyOnly(event.target.checked)}/>仅看依赖齐备</label></div>
    <div className="market-categories" aria-label="能力包分类">{Object.entries(categories).filter(([key]) => key === 'all' || entries.some(entry => entry.bundle.category === key)).map(([key, label]) => <button key={key} className={'chip ' + (category === key ? 'active' : '')} onClick={() => setCategory(key)}>{label}<span>{key === 'all' ? entries.length : entries.filter(entry => entry.bundle.category === key).length}</span></button>)}</div>
    <p className="market-source muted">当前为内置与本地导入目录。来源和版本由包声明；添加不安装软件、不授予执行权限。分享前请核对正文。</p>
    {warnings.map((warning, i) => <p key={i} className="help-warning market-source">{warning}</p>)}
    <div className="market-layout"><section className="market-results" aria-label="能力包目录">
      {loading && <p className="market-empty">正在读取目录并检查本机依赖…</p>}
      {!loading && !filtered.length && <div className="card card-pad market-empty"><h3>没有符合条件的能力包</h3><p className="muted">尝试其他关键词，或取消依赖筛选。</p><button className="btn" onClick={() => { setQuery(''); setCategory('all'); setReadyOnly(false) }}>清除筛选</button></div>}
      {filtered.map(entry => <button disabled={busy} key={entry.key} className={'market-package card ' + (selected === entry.key ? 'selected' : '')} onClick={() => { setError(''); setSelected(entry.key) }} aria-pressed={selected === entry.key}>
        <div className={'market-package-icon ' + entry.bundle.category}>{icons[entry.bundle.category]}</div><div className="market-package-body"><div className="market-package-title"><h3>{entry.bundle.manifest.name}</h3><span className="muted">v{entry.bundle.version}</span></div><p>{entry.bundle.manifest.description}</p><div className="tags">{entry.bundle.tags.map(tag => <span key={tag} className="tag">{tag}</span>)}</div><div className="market-package-meta"><span className={'tag ' + (entry.dependencies?.status === 'available' ? 'market-ready' : '')}>{entry.checkError ? '依赖检查失败' : labels[entry.dependencies?.status ?? entry.compatibility.status] ?? '等待依赖检查'}</span><span>{entry.source === 'builtin' ? '内置目录' : entry.source === 'preview' ? '待确认导入' : '本地导入'}</span></div></div>
      </button>)}
    </section><aside ref={detailPanel} className="card card-pad market-detail" aria-label="能力包详情">
      {!selected ? <div className="market-detail-empty"><span>↗</span><h3>选择一项能力</h3><p className="muted">查看流程、适用范围和本机依赖，再决定是否添加或分享。</p></div> : !detail ? <p className="muted">正在读取能力包详情…</p> : <>
        <div className="market-detail-heading"><span className="market-eyebrow">{categories[detail.bundle.category]}</span><h2>{detail.bundle.manifest.name}</h2><p>{detail.bundle.manifest.description}</p><div className="tags"><span className="tag">v{detail.bundle.version}</span><span className="tag">{detail.compatibility.reusable ? '通用流程' : '当前平台不适用'}</span><span className="tag">{detail.collection?.status === 'added' ? '已在 Skill 库' : detail.collection?.status === 'conflict' ? '同 ID 内容冲突' : '尚未添加'}</span></div></div>
        {detail.compatibility.issues.map((issue, i) => <p key={i} className="help-warning">{issue}</p>)}
        <div className="market-detail-actions"><button className="btn btn-primary" disabled={busy || !detail.compatibility.reusable || detail.collection?.status === 'added' || detail.collection?.status === 'conflict'} onClick={() => void add()}>{detail.collection?.status === 'added' ? '已在 Skill 库' : detail.collection?.status === 'conflict' ? '保留已有内容' : '加入 Skill 库'}</button><button className="btn" disabled={busy || !detail.compatibility.reusable} onClick={() => void copyTask()}>复制给 Agent</button></div>
        {detail.collection?.status === 'conflict' && <p className="help-warning">已有同 ID 的不同内容，不会覆盖。请为新流程使用不同 ID。</p>}
        <section><h3>输入与输出</h3><p><span className="muted">输入：</span>{detail.bundle.manifest.portability.inputs.join('；')}</p><p><span className="muted">输出：</span>{detail.bundle.manifest.portability.outputs.join('；')}</p><p className="muted">平台：{detail.bundle.manifest.portability.platforms.join(' / ')}</p></section>
        <section><div className="help-heading"><h3>本机工具依赖</h3><button className="text-btn" onClick={onTools}>查看工具</button></div><p className="muted">{labels[detail.dependencies?.status ?? ''] ?? '检查未完成'}；找到工具与获准执行分别检查。</p>{dependencyRows(detail.dependencies?.requires ?? [])}{(detail.dependencies?.optional?.length ?? 0) > 0 && <><h4>可选能力</h4>{dependencyRows(detail.dependencies?.optional ?? [], true)}</>}{detail.dependencies?.status === 'missing_capabilities' && <button className="btn" onClick={onAddTool}>添加已有的本机依赖</button>}</section>
        <section><h3>流程与权限</h3><p className="muted">{detail.bundle.manifest.portability.permissions.map((permission: string) => permissions[permission] ?? permission).join('、') || '无额外操作权限声明'}</p><details><summary>预览完整流程</summary><pre>{detail.bundle.instructions}</pre></details></section>
        <section><h3>来源与分享</h3><p>{detail.source === 'builtin' ? '内置目录' : '用户选择的本地能力包'} · {detail.bundle.author} · {detail.bundle.license}</p><p className="muted">作者为包内声明。分享文件包含流程、适用声明与依赖，不包含本机工具清单或 Agent 配置。</p><div className="program-actions"><button className="btn" disabled={busy} onClick={() => void act(async () => { const text = await invoke<string>('market_share', { key: detail.key }); await copyText(text); notify('已复制能力包 JSON，可保存为 .toolhub-skill.json 分享') })}>复制能力包</button><button className="btn" disabled={busy} onClick={() => void act(async () => { const path = await invoke<string | null>('market_export', { key: detail.key }); if (path) notify('能力包已导出：' + path) })}>导出能力包</button></div></section>
      </>}
    </aside></div>
  </>
}
