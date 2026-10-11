import { useCallback, useEffect, useRef, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { copyText } from './AgentHelp'
type Rule = { definition_id: string; name: string; file_names: string[]; path_substrings: string[]; capabilities: string[] }
type Package = { schema: string; id: string; name: string; description: string; version: string; author: string; license: string; platforms: string[]; min_toolhub_version: string; recognition?: { tools: Rule[]; capabilities: string[] }; skills: { manifest: any; instructions: string }[] }
type Installed = { package: Package; digest: string; enabled: boolean; can_rollback: boolean; source: string }
type Preview = { package: Package; digest: string; compatible: boolean; conflict?: string; installed_digest?: string; dependencies: any[] }
const rpc = (method: string, params: unknown = {}) => invoke<any>('rpc', { method, params })
export default function Extensions({ notify, onTools, onLibrary }: { notify: (m: string) => void; onTools: () => void; onLibrary: () => void }) {
  const [installed, setInstalled] = useState<Installed[]>([])
  const [catalog, setCatalog] = useState<Package[]>([])
  const [imported, setImported] = useState<Package | null>(null)
  const [selected, setSelected] = useState('')
  const [preview, setPreview] = useState<Preview | null>(null)
  const [busy, setBusy] = useState(false)
  const [loading, setLoading] = useState(true)
  const [query, setQuery] = useState('')
  const [error, setError] = useState('')
  const [remove, setRemove] = useState<Installed | null>(null)
  const epoch = useRef(0)
  const dialog = useRef<HTMLElement>(null)
  useEffect(() => {
    if (!remove) return
    const previous = document.activeElement as HTMLElement | null
    const key = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && !busy) { e.preventDefault(); setRemove(null) }
      if (e.key === 'Tab') {
        const buttons = dialog.current?.querySelectorAll<HTMLButtonElement>('button:not(:disabled)')
        if (!buttons?.length) { e.preventDefault(); return }
        const first = buttons[0], last = buttons[buttons.length - 1]
        if (e.shiftKey && document.activeElement === first) { e.preventDefault(); last.focus() }
        else if (!e.shiftKey && document.activeElement === last) { e.preventDefault(); first.focus() }
      }
    }
    document.addEventListener('keydown', key)
    return () => { document.removeEventListener('keydown', key); previous?.focus() }
  }, [remove, busy])
  const load = useCallback(async () => {
    const [rows, examples] = await Promise.all([rpc('extension.list'), invoke<Package[]>('extension_catalog')])
    setInstalled(rows); setCatalog(examples)
  }, [])
  useEffect(() => { let live = true; void load().catch(e => { if (live) setError(String(e)) }).finally(() => { if (live) setLoading(false) }); return () => { live = false; epoch.current++ } }, [load])
  const choices = new Map<string, { package: Package; source: string }>()
  catalog.forEach(p => choices.set(p.id, { package: p, source: 'builtin' }))
  installed.forEach(p => choices.set(p.package.id, { package: p.package, source: p.source }))
  if (imported) choices.set(imported.id, { package: imported, source: 'local' })
  const choice = choices.get(selected)
  const current = installed.find(p => p.package.id === selected)
  const selectedPackage = choice?.package
  useEffect(() => {
    const token = ++epoch.current; setPreview(null)
    if (!selectedPackage) return
    void rpc('extension.preview', { package: selectedPackage }).then(v => { if (token === epoch.current) setPreview(v) }).catch(e => { if (token === epoch.current) setError(String(e)) })
  }, [selectedPackage])
  const act = async (job: () => Promise<void>) => { setBusy(true); setError(''); try { await job() } catch (e) { setError(String(e)) } finally { setBusy(false) } }
  const importFile = () => act(async () => {
    const p = await invoke<Package | null>('extension_pick'); if (!p) return
    // Strict native and daemon validation precede any persistent mutation.
    await rpc('extension.preview', { package: p }); setImported(p); setSelected(p.id); setQuery('')
    notify('已读取扩展预览，请核对内容后安装或更新')
  })
  const install = () => act(async () => {
    if (!preview || !choice) return
    await rpc('extension.install', { package: preview.package, digest: preview.digest, ...(preview.installed_digest ? { expected_digest: preview.installed_digest } : {}), source: choice.source })
    setImported(null); await load(); notify('扩展已保存。识别规则将在下次扫描生效，通用 Skill 已可查询')
  })
  const state = () => act(async () => {
    if (!current) return
    await rpc('extension.set_enabled', { id: current.package.id, enabled: !current.enabled, expected_digest: current.digest }); await load()
    notify(current.enabled ? '已停用扩展；它提供的能力停止参与查询和执行' : '已启用扩展；可扫描已有程序')
  })
  const rollback = () => act(async () => {
    if (!current) return
    await rpc('extension.rollback', { id: current.package.id, expected_digest: current.digest }); setImported(null); await load(); notify('已回滚扩展内容，请重新扫描核对工具')
  })
  const uninstall = () => act(async () => {
    if (!remove) return
    await rpc('extension.remove', { id: remove.package.id, expected_digest: remove.digest }); setRemove(null); setImported(null); await load()
    notify('已卸载扩展；本机文件和原有工具、程序、产物记录均保留')
  })
  const exportFile = () => act(async () => { if (!preview) return; const path = await invoke<string | null>('extension_export', { package: preview.package }); if (path) notify('已导出扩展 JSON，可分享或作为开发模板') })
  const copyGuide = () => act(async () => {
    await copyText('请创建一个 ToolHub 声明式扩展包，使用 toolhub.extension/v1。参考我导出的 Blender 示例 JSON：修改包 id/name/version/description/author/license/platforms/min_toolhub_version，识别规则使用 toolhub.resource/v1，通用 Skill 使用 toolhub.bundle/v1。工具 definition_id 和 Skill id 必须以包 id + 点号开头。只声明 discover/inspect 数据规则和可跨 Agent 复用的指令，禁止脚本载荷、安装钩子、宿主专属 API 或内置工具文件名冲突。总文件不超过 256 KiB。通过 extension.preview 校验后，把 .toolhub-extension.json 文件交给用户在市场 → 扩展管理中选择、预览并安装；Agent 不能自行授予桌面管理权限。安装不安装依赖、不修改 PATH、不授予执行信任。工具执行仍遵循用户任务和原有策略。新能力通过现有 ToolHub 查询/解析/Skill 接口使用，无需增加 MCP 元工具。')
    notify('已复制扩展开发指引，配合导出的示例 JSON 使用')
  })
  const same = current?.digest === preview?.digest
  const rows = [...choices.values()].filter(({ package: p }) => [p.name, p.description, p.id, p.author].join(' ').toLowerCase().includes(query.toLowerCase()))
  return <>
    <div className="topbar"><div><h1>扩展管理</h1><p>用识别规则和通用流程，为已有程序增加新的能力。</p></div><div className="program-actions"><button className="btn" disabled={busy || loading} onClick={() => void act(load)}>刷新扩展</button><button className="btn" disabled={busy} onClick={() => void copyGuide()}>复制开发说明</button><button className="btn btn-primary" disabled={busy} onClick={() => void importFile()}>导入扩展包</button></div></div>
    {error && <div className="banner-error" role="alert"><span>{error}</span><button aria-label="关闭扩展错误" onClick={() => setError('')}>×</button></div>}
    <section className="market-intro"><div><span className="market-eyebrow">声明式扩展 · 无需修改源码</span><h2>从一个 JSON 包开始扩展</h2><p>安装前查看规则、流程与依赖。启用后，所有接入 ToolHub 的 Agent 共享相同能力。</p></div><div className="market-numbers"><div><strong>{installed.length}</strong><span>个已安装扩展</span></div><div><strong>{installed.filter(p => p.enabled).length}</strong><span>个已启用</span></div></div></section>
    <div className="market-toolbar"><div className="search-box"><input aria-label="搜索扩展" value={query} onChange={e => setQuery(e.target.value)} placeholder="搜索扩展名称、用途或作者…"/></div></div>
    <p className="market-source muted">当前支持内置示例与本地 JSON 导入。作者和许可证由包声明，SHA-256 校验内容一致性；执行型插件与在线目录将在后续版本评估。</p>
    <div className="market-layout"><section className="market-results" aria-label="扩展目录">
      {loading ? <p className="market-empty">正在读取扩展…</p> : !rows.length ? <div className="card card-pad market-empty">没有匹配的扩展，可导入新的 JSON 包。</div> : rows.map(({ package: p, source }) => {
        const own = installed.find(i => i.package.id === p.id)
        return <button key={p.id} disabled={busy} className={'market-package card ' + (selected === p.id ? 'selected' : '')} aria-pressed={selected === p.id} onClick={() => { setSelected(p.id); setError('') }}><div className="market-package-icon general">◈</div><div className="market-package-body"><div className="market-package-title"><h3>{p.name}</h3><span className="muted">v{p.version}</span></div><p>{p.description}</p><div className="market-package-meta"><span className={'tag ' + (own?.enabled ? 'market-ready' : '')}>{own ? own.enabled ? '已启用' : '已停用' : '待安装'}</span><span>{p.recognition?.tools.length ?? 0} 条识别规则 · {p.skills.length} 个 Skill</span><span>{source === 'builtin' ? '内置示例' : '本地导入'}</span></div></div></button>
      })}
    </section><aside className="card card-pad market-detail" aria-label="扩展详情">
      {!selected ? <div className="market-detail-empty"><span>◈</span><h3>选择一个扩展</h3><p className="muted">Blender 示例展示从识别程序到通用 Skill 的完整接入方式。</p></div> : !preview ? <p className="muted">正在校验扩展和本机依赖…</p> : <>
        <div className="market-detail-heading"><span className="market-eyebrow">toolhub.extension/v1</span><h2>{preview.package.name}</h2><p>{preview.package.description}</p><div className="tags"><span className="tag">v{preview.package.version}</span><span className={'tag ' + (preview.compatible ? 'market-ready' : '')}>{preview.compatible ? '当前平台兼容' : '平台或版本不兼容'}</span></div></div>
        {preview.conflict && <p className="help-warning">冲突：{preview.conflict}。已有内容将保留。</p>}
        {current && !same && <p className="help-warning">更新预览：v{current.package.version} → v{preview.package.version}。更新会保存上一版供回滚；识别规则改变后需要重新扫描。</p>}
        <div className="market-detail-actions"><button className="btn btn-primary" disabled={busy || !preview.compatible || !!preview.conflict || same} onClick={() => void install()}>{same ? '当前版本已安装' : current ? '确认更新扩展' : '确认安装扩展'}</button><button className="btn" disabled={busy} onClick={() => void exportFile()}>导出 JSON</button></div>
        {current && <div className="extension-manage"><button className="btn" disabled={busy} onClick={() => void state()}>{current.enabled ? '停用扩展' : '启用扩展'}</button><button className="btn" disabled={busy || !current.can_rollback} onClick={() => void rollback()}>回滚上一版</button><button className="btn extension-remove" disabled={busy} onClick={() => setRemove(current)}>卸载扩展</button></div>}
        <section><h3>来源与兼容性</h3><p>{preview.package.author} · {preview.package.license}</p><p className="muted">平台：{preview.package.platforms.join(' / ')}<br/>最低 ToolHub 版本：{preview.package.min_toolhub_version}</p><code className="extension-digest">SHA-256: {preview.digest}</code></section>
        <section><h3>工具识别规则</h3>{(preview.package.recognition?.tools ?? []).map(rule => <div className="extension-rule" key={rule.definition_id}><strong>{rule.name}</strong><p><code>{rule.file_names.join(' / ')}</code></p><p className="muted">{rule.path_substrings.length ? '路径条件：' + rule.path_substrings.join(' / ') : '从已有 PATH 或用户选择的扫描目录识别'}</p><div className="tags">{rule.capabilities.map(c => <span className="tag" key={c}>{c}</span>)}</div></div>)}{!preview.package.recognition && <p className="muted">此包只提供通用流程。</p>}<p className="muted">扫描只读，安装扩展不安装对应软件。识别结果需核对信任，包声明不能授予执行权限。</p><button className="text-btn" onClick={onTools}>到工具页扫描与核对</button></section>
        <section><div className="help-heading"><h3>通用 Skill 与本机依赖</h3><button className="text-btn" onClick={onLibrary}>查看 Skill 库</button></div>{preview.package.skills.map((skill, i) => <details className="extension-skill" key={skill.manifest.id}><summary>{skill.manifest.name}<span className="muted"> · {preview.dependencies[i]?.status === 'available' ? '依赖齐备' : '依赖或信任待处理'}</span></summary><p>{skill.manifest.description}</p><p className="muted">输入：{skill.manifest.portability.inputs.join('；')}<br/>输出：{skill.manifest.portability.outputs.join('；')}</p>{(preview.dependencies[i]?.requires ?? []).map((r: any) => <p key={r.requirement.capability}><code>{r.requirement.capability}</code> · {r.satisfied ? '已找到可用实例' : '待补齐或核对信任'}</p>)}<pre className="extension-instructions">{skill.instructions}</pre></details>)}{!preview.package.skills.length && <p className="muted">此包只提供识别规则。</p>}</section>
      </>}
    </aside></div>
    {remove && <div className="extension-modal-overlay"><section ref={dialog} className="card card-pad extension-confirm" role="dialog" aria-modal="true" aria-labelledby="extension-remove-title"><h2 id="extension-remove-title">卸载 {remove.package.name}？</h2><p>将移除扩展内容及它提供的通用 Skill，停止使用其识别能力。本机程序文件、原有工具和用户登记的程序、产物都会保留。</p><div className="program-actions"><button className="btn" disabled={busy} autoFocus onClick={() => setRemove(null)}>取消</button><button className="btn btn-primary" disabled={busy} onClick={() => void uninstall()}>确认卸载扩展</button></div></section></div>}
  </>
}
