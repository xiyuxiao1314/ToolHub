import { useCallback, useEffect, useRef, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { copyText } from './AgentHelp'
import { changed } from './workbench/host'
import type { Package, Installed, Preferences } from './workbench/host'
type Rule = { definition_id: string; name: string; file_names: string[]; path_substrings: string[]; capabilities: string[] }
type Preview = { package: Package; digest: string; compatible: boolean; conflict?: string; installed_digest?: string; dependencies: any[] }
const rpc = (method: string, params: unknown = {}) => invoke<any>('rpc', { method, params })
export default function Extensions({ notify, onTools, onLibrary, section = 'installed' }: { notify: (m: string) => void; onTools: () => void; onLibrary: () => void; section?: 'installed' | 'catalog' | 'appearance' }) {
  const [preferences, setPreferences] = useState<Preferences>({})
  const [editor, setEditor] = useState<string | null>(null)
  const [installed, setInstalled] = useState<Installed[]>([])
  const [catalog, setCatalog] = useState<Package[]>([])
  const [imported, setImported] = useState<Package | null>(null)
  const [selected, setSelected] = useState('')
  const [preview, setPreview] = useState<Preview | null>(null)
  const [busy, setBusy] = useState(false)
  const [loading, setLoading] = useState(true)
  const [query, setQuery] = useState('')
  const [kind,setKind]=useState('all'), [platform,setPlatform]=useState('all')
  const category=(p:Package)=>p.cli?'cli':p.host_module?'module':p.theme?'theme':p.views?.length?'page':p.recognition?'recognition':'skill'
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
    const [rows, examples, preferences] = await Promise.all([rpc('extension.list'), invoke<Package[]>('extension_catalog'), rpc('extension.preferences')])
    setInstalled(rows); setCatalog(examples); setPreferences(preferences); changed()
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
    setImported(null); await load(); setEditor(null); notify('扩展已保存，导航和页面已更新；识别规则需重新扫描')
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
    await copyText('请创建 ToolHub 扩展包（toolhub.extension/v2），参考我导出的内置页面或主题示例。包可贡献 views（id/label/icon/order/html/css/script）、theme（受限配色、layout/density/sidebar_width/radius）、recognition 和通用 skills。view.id 必须以包 id + 点号开头，依赖声明 dependencies:[{id,min_version}]。页面在不具同源和桌面权限的隔离 iframe 中运行，通过全局 toolhub.call(method,params)、toolhub.navigate(page)、toolhub.notify(message) 与宿主协作；需要声明 permissions: tools.read / programs.read / programs.propose / skills.read / environments.read / tasks.read。页面脚本仅开放有限查询和程序待确认提案，不直接启动或清理文件。beta.4 可声明 cli:{id,label,icon,order,tool_definition,actions:[{id,label,description,parameters,args,timeout_ms}]}，由核心生成独立操作页；parameters 仅 input_file/output_file/choice，args 使用完整参数占位符 {input}/{output}。需要 beta.4，文件选择、预览和单次授权由核心完成；脚本不能调用 extension.cli_start。禁止原生库、安装钩子和宿主专属 API，单包最多256 KiB。通过 extension.preview 验证后，交由用户在扩展管理导入、核对权限再安装；Agent 无桌面管理权限。更新需提高 version，recognition.version 同步提高。修改官方模块时只可定制导航等声明；host_module 是保留模块接口，不承载第三方代码。可编辑本地包、导出分享和回滚；卸载保留用户数据。Agent 使用统一查询入口，按需读取选中流程，不加载全部扩展正文。')
    notify('已复制扩展开发指引，配合导出的示例 JSON 使用')
  })
  const savePreferences = (value: Preferences) => act(async () => { await rpc('extension.preferences', {settings:value}); await load(); notify('工作台界面已更新') })
  const editPackage = () => {
    if (!preview) return
    const draft = JSON.parse(JSON.stringify(preview.package))
    draft.version = draft.version.replace(/^(\d+)\.(\d+)\.(\d+).*$/, (_:string,a:string,b:string,c:string)=>`${a}.${b}.${Number(c)+1}`)
    if (draft.recognition) draft.recognition.version = draft.version
    setEditor(JSON.stringify(draft, null, 2))
  }
  const previewEdit = () => act(async () => {
    if (editor===null) return
    const draft = JSON.parse(editor) as Package
    await rpc('extension.preview',{package:draft});setImported(draft);setSelected(draft.id);notify('修改已通过格式校验，请核对预览后安装更新')
  })
  const same = current?.digest === preview?.digest
  const rows = [...choices.values()].filter(({ package: p }) => (imported?.id===p.id || (section==='installed' ? installed.some(item=>item.package.id===p.id) : section==='appearance' ? !!p.theme : true)) && [p.name, p.description, p.id, p.author].join(' ').toLowerCase().includes(query.toLowerCase())&&(kind==='all'||category(p)===kind)&&(platform==='all'||p.platforms.includes(platform)))
  const heading = section==='appearance' ? '主题与布局' : section==='catalog' ? '扩展市场' : '已安装扩展'
  return <>
    <div className="topbar"><div><h1>{heading}</h1><p>{section==='appearance' ? '调整主题、导航位置与界面密度。' : section==='catalog' ? '浏览内置与本地导入目录，从社区仓库获取更多扩展。' : '管理已安装的功能、页面与后台能力。'}</p></div><div className="program-actions"><button className="btn" disabled={busy || loading} onClick={() => void act(load)}>刷新扩展</button><button className="btn" disabled={busy} onClick={()=>void act(async()=>{await invoke('extension_repository')})}>浏览社区扩展 ↗</button><button className="btn" disabled={busy} onClick={() => void copyGuide()}>复制开发说明</button><button className="btn btn-primary" disabled={busy} onClick={() => void importFile()}>导入扩展包</button></div></div>
    {error && <div className="banner-error" role="alert"><span>{error}</span><button aria-label="关闭扩展错误" onClick={() => setError('')}>×</button></div>}
    {section!=='appearance'&&<section className="market-intro"><div><span className="market-eyebrow">EXTENSION WORKBENCH</span><h2>让工作台随你而变</h2><p>功能按需装卸，应用集中管理，常用扩展固定到工作空间。</p></div><div className="market-numbers"><div><strong>{installed.length}</strong><span>个已安装扩展</span></div><div><strong>{installed.filter(p => p.enabled).length}</strong><span>个已启用</span></div></div></section>}
    {section==='appearance'&&<section className="extension-kernel-settings" aria-label="工作台外观">
      <label>主题<select aria-label="工作台主题" disabled={busy} value={preferences.theme_id??''} onChange={e=>void savePreferences({...preferences,theme_id:e.target.value||null})}><option value="">ToolHub 默认</option>{installed.filter(e=>e.enabled&&e.package.theme).map(e=><option key={e.package.id} value={e.package.id}>{e.package.name}</option>)}</select></label>
      <label>导航布局<select aria-label="导航布局" disabled={busy} value={preferences.layout??''} onChange={e=>void savePreferences({...preferences,layout:(e.target.value||null) as Preferences['layout']})}><option value="">跟随主题</option><option value="left">左侧导航</option><option value="right">右侧导航</option><option value="top">顶部导航</option></select></label>
      <label>密度<select aria-label="界面密度" disabled={busy} value={preferences.density??''} onChange={e=>void savePreferences({...preferences,density:(e.target.value||null) as Preferences['density']})}><option value="">跟随主题</option><option value="comfortable">宽松</option><option value="compact">紧凑</option></select></label>
      <button className="btn" disabled={busy} onClick={()=>void savePreferences({})}>恢复默认界面</button>
    </section>}
    {editor!==null&&<section className="extension-edit-area"><h3>编辑本地扩展</h3><p className="muted">修改不会立即生效。先校验预览，再安装更新；原版本可回滚。更新时请提高版本号。</p><textarea className="extension-editor" aria-label="扩展 JSON 编辑器" value={editor} onChange={e=>setEditor(e.target.value)} spellCheck={false}/><div className="program-actions"><button className="btn btn-primary" disabled={busy} onClick={()=>void previewEdit()}>校验修改并预览</button><button className="btn" disabled={busy} onClick={()=>setEditor(null)}>关闭编辑器</button></div></section>}
    <div className="market-toolbar"><div className="search-box"><input aria-label="搜索扩展" value={query} onChange={e => setQuery(e.target.value)} placeholder="搜索扩展名称、用途或作者…"/></div><select aria-label="扩展类型" value={kind} onChange={e=>setKind(e.target.value)}><option value="all">所有类型</option><option value="cli">CLI 工具面板</option><option value="module">官方模块</option><option value="page">交互页面</option><option value="theme">主题</option><option value="recognition">识别规则</option><option value="skill">通用 Skill</option></select><select aria-label="扩展平台" value={platform} onChange={e=>setPlatform(e.target.value)}><option value="all">所有平台</option><option value="windows">Windows</option><option value="macos">macOS</option><option value="linux">Linux</option></select></div>
    <p className="market-source muted">官方功能默认启用；卸载保留数据，可从目录重新安装。第三方页面运行在隔离环境，按声明权限查询核心服务。社区目录独立维护；打开仓库选择并下载 JSON，再导入预览和确认安装。</p>
    <div className="market-layout"><section className="market-results" aria-label="扩展目录">
      {loading ? <p className="market-empty">正在读取扩展…</p> : !rows.length ? <div className="card card-pad market-empty">没有匹配的扩展，可导入新的 JSON 包。</div> : rows.map(({ package: p, source }) => {
        const own = installed.find(i => i.package.id === p.id)
        return <button key={p.id} disabled={busy} className={'market-package card ' + (selected === p.id ? 'selected' : '')} aria-pressed={selected === p.id} onClick={() => { setSelected(p.id); setError('') }}><div className="market-package-icon general">◈</div><div className="market-package-body"><div className="market-package-title"><h3>{p.name}</h3><span className="muted">v{p.version}</span></div><p>{p.description}</p><div className="market-package-meta"><span className={'tag ' + (own?.enabled ? 'market-ready' : '')}>{own ? own.enabled ? '已启用' : '已停用' : '待安装'}</span><span>{p.host_module ? '官方功能模块' : `${(p.views?.length??0)+(p.cli?1:0)} 个页面 · ${p.cli?'CLI 工具面板':p.theme?'主题 / 布局':`${p.skills.length} 个 Skill`}`}</span><span>{p.host_module ? 'ToolHub 内置' : source === 'builtin' ? '内置示例' : '本地导入'}</span></div></div></button>
      })}
    </section><aside className="card card-pad market-detail" aria-label="扩展详情">
      {!selected ? <div className="market-detail-empty"><span>◈</span><h3>选择一个扩展</h3><p className="muted">选择官方功能，或从工具速览、石墨主题和 Blender 示例开始扩展。</p></div> : !preview ? <p className="muted">正在校验扩展和本机依赖…</p> : <>
        <div className="market-detail-heading"><span className="market-eyebrow">{preview.package.schema}</span><h2>{preview.package.name}</h2><p>{preview.package.description}</p><div className="tags"><span className="tag">v{preview.package.version}</span><span className={'tag ' + (preview.compatible ? 'market-ready' : '')}>{preview.compatible ? '当前平台兼容' : '平台或版本不兼容'}</span></div></div>
        {preview.conflict && <p className="help-warning">冲突：{preview.conflict}。已有内容将保留。</p>}
        {current && !same && <p className="help-warning">更新预览：v{current.package.version} → v{preview.package.version}。更新会保存上一版供回滚；识别规则改变后需要重新扫描。</p>}
        <div className="market-detail-actions"><button className="btn btn-primary" disabled={busy || !preview.compatible || !!preview.conflict || same} onClick={() => void install()}>{same ? '当前版本已安装' : current ? '确认更新扩展' : '确认安装扩展'}</button><button className="btn" disabled={busy} onClick={() => void exportFile()}>导出 JSON</button></div>
        {current && <div className="extension-manage"><button className="btn" disabled={busy} onClick={() => void state()}>{current.enabled ? '停用扩展' : '启用扩展'}</button><button className="btn" disabled={busy || !current.can_rollback} onClick={() => void rollback()}>回滚上一版</button><button className="btn extension-remove" disabled={busy} onClick={() => setRemove(current)}>卸载扩展</button></div>}
        <section><h3>工作台贡献</h3>{preview.package.cli&&<div className="extension-runtime-info"><strong>CLI 工具面板：{preview.package.cli.label}</strong><p>本机工具：{preview.package.cli.tool_definition}</p><p>读取所选输入，写入所选输出；由核心预览和逐次授权，后台运行最长 5 分钟。</p>{preview.package.cli.actions.map(a=><details key={a.id}><summary>{a.label}</summary><p>{a.description}</p><pre className="extension-instructions">{JSON.stringify(a.args,null,2)}</pre></details>)}</div>}<div className="extension-contributions">{preview.package.host_module&&<span className="tag">官方模块：{preview.package.navigation?.label}</span>}{(preview.package.views??[]).map(v=><span key={v.id} className="tag">导航页面：{v.label}</span>)}{preview.package.theme&&<span className="tag">主题与布局</span>}</div><p className="extension-runtime-info">{preview.package.host_module?'官方模块使用宿主维护的页面和原生服务，扩展负责启停及导航配置。':preview.package.views?.length?'页面包含 HTML / CSS / JavaScript，在隔离 iframe 中运行，不能取得桌面控制器权限。':'此包提供声明式资源或主题。'}</p><p className="muted">页面权限：{preview.package.permissions?.join(' · ')||'不申请页面服务权限'}</p>{(preview.package.dependencies??[]).map(d=><p key={d.id}>依赖 {d.id} ≥ {d.min_version}</p>)}<button className="btn" disabled={busy} onClick={editPackage}>编辑扩展</button>{current?.enabled&&preview.package.theme&&<button className="btn" disabled={busy} onClick={()=>void savePreferences({...preferences,theme_id:current.package.id,layout:null,density:null})}>应用主题</button>}{(preview.package.views??[]).map(v=><details key={v.id} className="extension-skill"><summary>页面源码：{v.label}</summary><pre className="extension-instructions">{v.html+'\n\n'+v.css+'\n\n'+v.script}</pre></details>)}</section>
        <section><h3>来源与兼容性</h3><p>{preview.package.author} · {preview.package.license}</p><p className="muted">平台：{preview.package.platforms.join(' / ')}<br/>最低 ToolHub 版本：{preview.package.min_toolhub_version}</p><code className="extension-digest">SHA-256: {preview.digest}</code></section>
        <section><h3>工具识别规则</h3>{(preview.package.recognition?.tools ?? []).map(rule => <div className="extension-rule" key={rule.definition_id}><strong>{rule.name}</strong><p><code>{rule.file_names.join(' / ')}</code></p><p className="muted">{rule.path_substrings.length ? '路径条件：' + rule.path_substrings.join(' / ') : '从已有 PATH 或用户选择的扫描目录识别'}</p><div className="tags">{rule.capabilities.map((c: string) => <span className="tag" key={c}>{c}</span>)}</div></div>)}{!preview.package.recognition && <p className="muted">此包未声明额外识别规则。</p>}<p className="muted">扫描只读，安装扩展不安装对应软件。识别结果需核对信任，包声明不能授予执行权限。</p><button className="text-btn" onClick={onTools}>到工具页扫描与核对</button></section>
        <section><div className="help-heading"><h3>通用 Skill 与本机依赖</h3><button className="text-btn" onClick={onLibrary}>查看 Skill 库</button></div>{preview.package.skills.map((skill, i) => <details className="extension-skill" key={skill.manifest.id}><summary>{skill.manifest.name}<span className="muted"> · {preview.dependencies[i]?.status === 'available' ? '依赖齐备' : '依赖或信任待处理'}</span></summary><p>{skill.manifest.description}</p><p className="muted">输入：{skill.manifest.portability.inputs.join('；')}<br/>输出：{skill.manifest.portability.outputs.join('；')}</p>{(preview.dependencies[i]?.requires ?? []).map((r: any) => <p key={r.requirement.capability}><code>{r.requirement.capability}</code> · {r.satisfied ? '已找到可用实例' : '待补齐或核对信任'}</p>)}<pre className="extension-instructions">{skill.instructions}</pre></details>)}{!preview.package.skills.length && <p className="muted">此包未声明通用 Skill。</p>}</section>
      </>}
    </aside></div>
    {remove && <div className="extension-modal-overlay"><section ref={dialog} className="card card-pad extension-confirm" role="dialog" aria-modal="true" aria-labelledby="extension-remove-title"><h2 id="extension-remove-title">卸载 {remove.package.name}？</h2><p>将移除扩展贡献的页面、功能和流程。本机文件、已登记程序、工具与产物数据会保留；产物功能停用时暂停清理。主题被移除后自动恢复默认外观。</p><div className="program-actions"><button className="btn" disabled={busy} autoFocus onClick={() => setRemove(null)}>取消</button><button className="btn btn-primary" disabled={busy} onClick={() => void uninstall()}>确认卸载扩展</button></div></section></div>}
  </>
}
