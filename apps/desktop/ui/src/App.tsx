import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'

type Json = Record<string, unknown> | unknown[] | string | number | boolean | null

async function rpc(method: string, params: Json = {}): Promise<any> {
  return invoke('rpc', { method, params })
}

type PageId = 'tools' | 'agents' | 'envs' | 'market' | 'tasks' | 'settings'

const NAV: { id: PageId; label: string; icon: string }[] = [
  { id: 'tools', label: '工具', icon: '🧰' },
  { id: 'agents', label: '智能体', icon: '🤖' },
  { id: 'envs', label: '环境', icon: '🧩' },
  { id: 'market', label: '市场', icon: '🛍️' },
  { id: 'tasks', label: '任务', icon: '📋' },
  { id: 'settings', label: '设置', icon: '⚙️' },
]

const TRUST_LABEL: Record<string, string> = {
  unknown: '未知',
  known: '已知',
  probable: '较可能',
  user_trusted: '用户信任',
  blocked: '已阻止',
  user_owned: '用户所有',
}

const KIND_LABEL: Record<string, string> = {
  cargo: 'Cargo',
  system: '系统',
  user: '用户',
  homebrew: 'Homebrew',
  conda: 'Conda',
  nvm: 'nvm',
  pyenv: 'pyenv',
  venv: 'venv',
  unknown: '未分类',
}

type UiPrefs = {
  autoScan: boolean
  showHidden: boolean
  unknownPerm: 'ask' | 'deny' | 'allow'
  retainDays: string
  execPolicy: 'toolhub' | 'sandbox'
  theme: 'light' | 'dark'
}

const DEFAULT_PREFS: UiPrefs = {
  autoScan: true,
  showHidden: false,
  unknownPerm: 'ask',
  retainDays: '30',
  execPolicy: 'toolhub',
  theme: 'light',
}

function loadPrefs(): UiPrefs {
  try {
    return { ...DEFAULT_PREFS, ...JSON.parse(localStorage.getItem('th.prefs') || '{}') }
  } catch {
    return { ...DEFAULT_PREFS }
  }
}

function savePrefs(p: UiPrefs) {
  localStorage.setItem('th.prefs', JSON.stringify(p))
}

function listFrom(value: unknown): any[] {
  if (Array.isArray(value)) return value
  if (value && typeof value === 'object' && Array.isArray((value as any).result)) {
    return (value as any).result
  }
  return []
}

function asRecord(value: any): Record<string, any> {
  return value && typeof value === 'object' && !Array.isArray(value) ? value : {}
}

function parseMaybeJson(raw: unknown): any {
  if (typeof raw === 'string') {
    try {
      return JSON.parse(raw)
    } catch {
      return raw
    }
  }
  return raw
}

function trustLevel(raw: unknown): string {
  const parsed = parseMaybeJson(raw)
  if (parsed && typeof parsed === 'object') return String(parsed.level ?? parsed.kind ?? '')
  return String(parsed ?? '')
}

function trustLabel(raw: unknown): string {
  const level = trustLevel(raw)
  return TRUST_LABEL[level] ?? (level || '未知')
}

function formatTs(raw: unknown): string {
  const text = String(raw ?? '')
  if (!text) return '—'
  const d = new Date(text)
  return Number.isNaN(d.getTime()) ? text : d.toLocaleString('zh-CN', { hour12: false })
}

function toolIcon(name: string): { bg: string; text: string } {
  const n = (name || '?').toLowerCase()
  if (n.includes('python')) return { bg: '#3776ab', text: 'Py' }
  if (n.includes('node')) return { bg: '#3c873a', text: 'JS' }
  if (n.includes('git')) return { bg: '#f05033', text: 'Git' }
  if (n.includes('javac') || n.includes('编译')) return { bg: '#c2410c', text: 'Jc' }
  if (n.includes('java')) return { bg: '#e76f00', text: 'Jv' }
  if (n.includes('rust') || n.includes('cargo')) return { bg: '#b7410e', text: 'Rs' }
  return { bg: '#64748b', text: name.slice(0, 2).toUpperCase() }
}

function toolTags(name: string, path: string): string[] {
  const n = `${name} ${path}`.toLowerCase()
  const tags: string[] = []
  if (/python|node|java|rust|cargo|golang|go\.exe/.test(n)) tags.push('Runtime')
  if (/\.exe|\\cmd\\|\\bin\\|\.cmd|\.bat/.test(n)) tags.push('CLI')
  if (/sdk|jdk/.test(n)) tags.push('SDK')
  if (/git|docker|code/.test(n)) tags.push('DevOps')
  return tags.length ? tags : ['工具']
}

function tagClass(tag: string): string {
  if (tag === 'Runtime' || tag === 'CLI') return 'tag blue'
  if (tag === 'SDK') return 'tag purple'
  if (tag === 'DevOps') return 'tag orange'
  return 'tag'
}

function StatusBadge({ text, tone = 'ok' }: { text: string; tone?: 'ok' | 'warn' | 'muted' }) {
  return <span className={`badge badge-${tone}`}>{text}</span>
}

function ToolIcon({ name, size = 48 }: { name: string; size?: number }) {
  const { bg, text } = toolIcon(name)
  return (
    <div className="tool-icon" style={{ background: bg, width: size, height: size, fontSize: size < 40 ? 14 : 18 }}>
      {text}
    </div>
  )
}

function Toast({ message, onClose }: { message: string; onClose: () => void }) {
  useEffect(() => {
    const t = setTimeout(onClose, 3500)
    return () => clearTimeout(t)
  }, [onClose])
  return (
    <div
      style={{
        position: 'fixed',
        right: 20,
        bottom: 20,
        background: '#0f172a',
        color: '#fff',
        padding: '12px 16px',
        borderRadius: 12,
        zIndex: 50,
        maxWidth: 420,
        boxShadow: '0 10px 30px rgba(0,0,0,.2)',
      }}
    >
      {message}
    </div>
  )
}

export default function App() {
  const [page, setPage] = useState<PageId>(() => {
    const saved = localStorage.getItem('th.page') as PageId | null
    return saved && NAV.some((n) => n.id === saved) ? saved : 'tools'
  })
  const [tools, setTools] = useState<any[]>([])
  const [agents, setAgents] = useState<any[]>([])
  const [envs, setEnvs] = useState<any[]>([])
  const [dups, setDups] = useState<any[]>([])
  const [activity, setActivity] = useState<any[]>([])
  const [status, setStatus] = useState<any>(null)
  const [query, setQuery] = useState('')
  const [selected, setSelected] = useState<any | null>(null)
  const [detailTab, setDetailTab] = useState<'overview' | 'caps' | 'env' | 'usage'>('overview')
  const [detailData, setDetailData] = useState<any>(null)
  const [error, setError] = useState<string | null>(null)
  const [toast, setToast] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)
  const [busy, setBusy] = useState(false)
  const [prefs, setPrefs] = useState<UiPrefs>(() => loadPrefs())
  const [scanMode, setScanMode] = useState<'quick' | 'full'>('quick')
  const [serverSettings, setServerSettings] = useState<any>(null)
  const [importOpen, setImportOpen] = useState(false)
  const [importPath, setImportPath] = useState('')
  const [addToolOpen, setAddToolOpen] = useState(false)
  const [addToolPath, setAddToolPath] = useState('')
  const [exportPath, setExportPath] = useState('')
  const loadSeq = useRef(0)

  const notify = (msg: string) => setToast(msg)
  const fail = (e: any) => {
    const msg = String(e?.message ?? e)
    setError(msg)
    notify(msg)
  }

  const load = useCallback(async () => {
    const seq = ++loadSeq.current
    setLoading(true)
    setError(null)
    try {
      const [toolRows, agentRows, envRows, dupRows, actRows, st, cfg] = await Promise.all([
        rpc('registry.search', { query: '' }),
        rpc('agent.list', {}),
        rpc('environment.list', {}),
        rpc('environment.duplicates', {}),
        rpc('activity.list', {}),
        rpc('status', {}),
        rpc('settings.get', {}).catch(() => null),
      ])
      if (seq !== loadSeq.current) return
      const rows = listFrom(toolRows)
      setTools(rows)
      setAgents(listFrom(agentRows))
      setEnvs(listFrom(envRows))
      setDups(listFrom(dupRows))
      setActivity(listFrom(actRows))
      setStatus(st)
      if (cfg) {
        setServerSettings(asRecord(cfg))
        const mode = String(asRecord(cfg).scan_mode || '')
        if (mode === 'quick' || mode === 'full') setScanMode(mode)
      }
      setSelected((prev: any) => {
        if (prev) {
          const again = rows.find((r: any) => r.id === prev.id)
          if (again) return again
        }
        return rows[0] ?? null
      })
    } catch (e: any) {
      if (seq === loadSeq.current) fail(e)
    } finally {
      if (seq === loadSeq.current) setLoading(false)
    }
  }, [])

  useEffect(() => {
    localStorage.setItem('th.page', page)
  }, [page])

  useEffect(() => {
    void load()
  }, [load])

  useEffect(() => {
    if (!selected?.id) {
      setDetailData(null)
      return
    }
    let alive = true
    void rpc('registry.inspect_instance', { id: selected.id })
      .then((data) => alive && setDetailData(data))
      .catch(() => alive && setDetailData(null))
    return () => {
      alive = false
    }
  }, [selected?.id])

  const onScan = async (mode: 'quick' | 'full' = scanMode) => {
    try {
      setBusy(true)
      setError(null)
      notify(mode === 'full' ? '正在深度扫描…' : '正在快速扫描…')
      await rpc('scan.start', { mode })
      await load()
      notify('扫描完成')
    } catch (e: any) {
      fail(e)
    } finally {
      setBusy(false)
    }
  }

  const updatePrefs = async (next: UiPrefs) => {
    setPrefs(next)
    savePrefs(next)
    try {
      await rpc('settings.set', {
        settings: {
          scan_mode: next === prefs ? scanMode : scanMode,
        },
      })
    } catch {
      /* controller may be required for server-side settings */
    }
  }

  const persistScanMode = async (mode: 'quick' | 'full') => {
    setScanMode(mode)
    try {
      const prev = serverSettings ?? {}
      await rpc('settings.set', {
        settings: {
          scan_mode: mode,
          scan_roots: prev.scan_roots ?? [],
          resolver_preferences: prev.resolver_preferences ?? {},
          privacy: prev.privacy ?? { redact_exports: true },
        },
      })
      notify(`默认扫描模式已设为${mode === 'quick' ? '快速扫描' : '深度扫描'}`)
      const cfg = await rpc('settings.get', {})
      setServerSettings(asRecord(cfg))
    } catch (e: any) {
      // still keep UI selection
      notify(`扫描模式已切换（服务端：${String(e?.message ?? e)}）`)
    }
  }

  const exportConfig = async () => {
    try {
      setBusy(true)
      const report = await rpc('export.report', {})
      const settings = serverSettings ?? (await rpc('settings.get', {}).catch(() => ({})))
      const payload = JSON.stringify(
        {
          schema: 'toolhub.report/v1',
          exported_at: new Date().toISOString(),
          ui_prefs: prefs,
          settings,
          report,
        },
        null,
        2,
      )
      let path = exportPath
      if (!path) {
        path = await invoke<string>('default_export_path_string')
        setExportPath(path)
      }
      const saved = await invoke<string>('save_text_file', { path, contents: payload })
      notify(`配置已导出到 ${saved}`)
    } catch (e: any) {
      fail(e)
    } finally {
      setBusy(false)
    }
  }

  const importConfig = async () => {
    try {
      if (!importPath.trim()) {
        notify('请填写配置文件路径')
        return
      }
      setBusy(true)
      const text = await invoke<string>('read_text_file', { path: importPath.trim() })
      const data = JSON.parse(text)
      if (data.ui_prefs) {
        const next = { ...DEFAULT_PREFS, ...data.ui_prefs }
        setPrefs(next)
        savePrefs(next)
      }
      if (data.settings) {
        try {
          const clean: any = { ...data.settings }
          delete clean.versions
          await rpc('settings.set', { settings: clean })
        } catch (e: any) {
          notify(`服务端设置未导入：${String(e?.message ?? e)}`)
        }
      }
      if (data.report || data.tools) {
        const res = await rpc('export.report', { import: data.report ?? data })
        notify(`已导入报告：${JSON.stringify(res)}`)
      }
      await load()
      setImportOpen(false)
      notify('配置导入完成')
    } catch (e: any) {
      fail(e)
    } finally {
      setBusy(false)
    }
  }

  const resetSettings = async () => {
    const next = { ...DEFAULT_PREFS }
    setPrefs(next)
    savePrefs(next)
    setScanMode('quick')
    try {
      await rpc('settings.set', {
        settings: {
          scan_mode: 'quick',
          scan_roots: [],
          resolver_preferences: {},
          privacy: { redact_exports: true },
        },
      })
    } catch (e: any) {
      notify(`已恢复界面默认；服务端：${String(e?.message ?? e)}`)
      return
    }
    await load()
    notify('已重置为默认设置')
  }

  const openTerminal = async () => {
    const path = selected?.path
    if (!path) {
      notify('请先选择工具')
      return
    }
    try {
      const cwd = await invoke<string>('open_terminal', { path: String(path) })
      notify(`已在终端中打开 ${cwd}`)
    } catch (e: any) {
      fail(e)
    }
  }

  const revealPath = async () => {
    const path = selected?.path
    if (!path) return
    try {
      await invoke('reveal_path', { path: String(path) })
    } catch (e: any) {
      fail(e)
    }
  }

  const addToolByPath = async () => {
    const root = addToolPath.trim()
    if (!root) {
      notify('请填写绝对路径')
      return
    }
    try {
      setBusy(true)
      await rpc('scan.start', { mode: 'custom', roots: [root] })
      await load()
      setAddToolOpen(false)
      notify('已扫描该路径并更新工具列表')
    } catch (e: any) {
      fail(e)
    } finally {
      setBusy(false)
    }
  }

  const saveUnknownPerm = async (value: UiPrefs['unknownPerm']) => {
    await updatePrefs({ ...prefs, unknownPerm: value })
    try {
      await rpc('policy.set', {
        scope: 'tool',
        subject: '*',
        action: value === 'allow' ? 'allow' : value === 'deny' ? 'deny' : 'ask',
      })
      notify('未知工具默认权限已更新')
    } catch (e: any) {
      notify(`界面偏好已保存；策略：${String(e?.message ?? e)}`)
    }
  }

  const filteredTools = useMemo(() => {
    const q = query.trim().toLowerCase()
    const base = prefs.showHidden ? tools : tools.filter((t) => t.status !== 'missing')
    if (!q) return base
    return base.filter((t) => `${t.name ?? ''} ${t.path ?? ''} ${t.definition_id ?? ''}`.toLowerCase().includes(q))
  }, [tools, query, prefs.showHidden])

  const cats = useMemo(() => {
    const count = (k: string) =>
      filteredTools.filter((t) => toolTags(String(t.name ?? ''), String(t.path ?? '')).some((x) => x.toLowerCase() === k.toLowerCase())).length
    return [
      { label: `全部 (${filteredTools.length})`, key: 'all' },
      { label: `CLI (${count('cli')})`, key: 'cli' },
      { label: `Runtime (${count('runtime')})`, key: 'runtime' },
      { label: `SDK (${count('sdk')})`, key: 'sdk' },
      { label: `DevOps (${count('devops')})`, key: 'devops' },
    ]
  }, [filteredTools])

  const [cat, setCat] = useState('all')
  const shown = useMemo(() => {
    if (cat === 'all') return filteredTools
    return filteredTools.filter((t) => toolTags(String(t.name ?? ''), String(t.path ?? '')).some((x) => x.toLowerCase() === cat))
  }, [filteredTools, cat])

  const statusRec = asRecord(status?.result ?? status)

  return (
    <div className="app-shell">
      {toast && <Toast message={toast} onClose={() => setToast(null)} />}
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-badge">TH</div>
          <span>ToolHub</span>
        </div>
        <nav className="nav">
          {NAV.map((item) => (
            <button
              key={item.id}
              className={`nav-btn ${page === item.id ? 'active' : ''}`}
              onClick={() => setPage(item.id)}
            >
              <span>{item.icon}</span>
              <span>{item.label}</span>
            </button>
          ))}
        </nav>
        <div className="quick">
          <div className="quick-title">快速操作</div>
          <button className="quick-btn" onClick={() => { setPage('tools'); setAddToolOpen(true) }}>
            ＋ 添加工具
          </button>
          <button className="quick-btn" onClick={() => void onScan('quick')} disabled={busy}>
            🔍 扫描本机
          </button>
          <button className="quick-btn" onClick={() => setPage('market')}>
            📦 从模板安装
          </button>
          <button className="quick-btn" onClick={() => { setPage('settings'); setImportOpen(true) }}>
            ⬇️ 导入配置
          </button>
        </div>
        <div className="side-foot">
          <div className="muted">工具总数</div>
          <strong>{tools.length}</strong>
          <div className="progress">
            <i style={{ width: `${Math.min(100, tools.length * 4)}%` }} />
          </div>
          <div className="muted" style={{ fontSize: 12 }}>
            本地磁盘 · 已识别 {tools.length} 个工具
          </div>
        </div>
      </aside>

      <section className="content">
        {error && (
          <div className="layout-1" style={{ paddingBottom: 0 }}>
            <div className="error-box">{error}</div>
          </div>
        )}

        {page === 'tools' && (
          <>
            <div className="topbar">
              <div>
                <h1>工具</h1>
                <p>管理和使用本地及远程工具，让智能体拥有更强的能力。</p>
              </div>
              <div style={{ display: 'flex', gap: 10 }}>
                <button className="btn" onClick={() => void onScan('full')} disabled={busy}>
                  {busy ? '处理中…' : '扫描本机'}
                </button>
                <button className="btn btn-primary" onClick={() => setAddToolOpen(true)}>
                  ＋ 添加工具
                </button>
              </div>
            </div>
            <div className="layout-2">
              <div>
                {addToolOpen && (
                  <div className="card card-pad" style={{ marginBottom: 12 }}>
                    <h3 style={{ marginTop: 0 }}>添加工具</h3>
                    <p className="muted">输入本机已安装工具的绝对路径或目录，将进行定向扫描。</p>
                    <div style={{ display: 'flex', gap: 8 }}>
                      <input
                        className="select"
                        style={{ flex: 1, minWidth: 0 }}
                        value={addToolPath}
                        onChange={(e) => setAddToolPath(e.target.value)}
                        placeholder="例如 D:\code\Git\cmd"
                      />
                      <button className="btn btn-primary" onClick={() => void addToolByPath()} disabled={busy}>
                        扫描路径
                      </button>
                      <button className="btn" onClick={() => setAddToolOpen(false)}>
                        取消
                      </button>
                    </div>
                  </div>
                )}
                <div className="search-row">
                  <div className="search-box">
                    <span>🔍</span>
                    <input
                      value={query}
                      onChange={(e) => setQuery(e.target.value)}
                      placeholder="搜索工具名称、描述或标签..."
                    />
                    <span className="kbd">Ctrl K</span>
                  </div>
                </div>
                <div className="chips">
                  {cats.map((c) => (
                    <button key={c.key} className={`chip ${cat === c.key ? 'active' : ''}`} onClick={() => setCat(c.key)}>
                      {c.label}
                    </button>
                  ))}
                </div>
                <div className="tool-list">
                  {loading && <div className="empty">加载中…</div>}
                  {!loading && shown.length === 0 && <div className="empty">没有匹配的工具</div>}
                  {shown.map((t, i) => {
                    const active = selected && selected.id === t.id
                    return (
                      <button key={t.id ?? i} className={`tool-item ${active ? 'active' : ''}`} onClick={() => setSelected(t)}>
                        <ToolIcon name={String(t.name ?? '')} />
                        <div>
                          <div className="tool-name">
                            {String(t.name ?? '—')}
                            {t.version ? ` ${t.version}` : ''}
                          </div>
                          <div className="tool-desc">{String(t.path ?? '')}</div>
                          <div className="tags">
                            {toolTags(String(t.name ?? ''), String(t.path ?? '')).map((tag) => (
                              <span key={tag} className={tagClass(tag)}>
                                {tag}
                              </span>
                            ))}
                          </div>
                        </div>
                        <div className="tool-meta">
                          <div className="ver">{t.version ? String(t.version) : '—'}</div>
                          <StatusBadge text={t.status === 'available' ? '已安装' : String(t.status ?? '未知')} tone="ok" />
                          <div className="muted" style={{ marginTop: 6, fontSize: 12 }}>
                            {trustLabel(t.trust)}
                          </div>
                        </div>
                      </button>
                    )
                  })}
                </div>
              </div>

              <aside className="card card-pad">
                {!selected ? (
                  <div className="empty">选择左侧工具查看详情</div>
                ) : (
                  <>
                    <div className="detail-head">
                      <ToolIcon name={String(selected.name ?? '')} size={56} />
                      <div style={{ flex: 1 }}>
                        <h2>
                          {String(selected.name ?? '—')}
                          {selected.version ? ` ${String(selected.version)}` : ''}
                        </h2>
                        <div className="muted">{String(selected.definition_id ?? '')}</div>
                        <div style={{ marginTop: 8, display: 'flex', gap: 8, alignItems: 'center', flexWrap: 'wrap' }}>
                          <StatusBadge text={selected.status === 'available' ? '已安装' : String(selected.status ?? '')} />
                          <StatusBadge text={trustLabel(selected.trust)} tone={trustLevel(selected.trust) === 'known' ? 'ok' : 'muted'} />
                        </div>
                      </div>
                    </div>
                    <p className="muted" style={{ marginTop: 0 }}>
                      {String(selected.name ?? '')}
                      {selected.version ? ` 版本 ${String(selected.version)}。` : '。'}
                      路径：<code>{String(selected.path ?? '—')}</code>
                    </p>
                    <div className="tags" style={{ marginBottom: 14 }}>
                      {toolTags(String(selected.name ?? ''), String(selected.path ?? '')).map((tag) => (
                        <span key={tag} className={tagClass(tag)}>
                          {tag}
                        </span>
                      ))}
                    </div>
                    <div style={{ display: 'flex', gap: 8 }}>
                      <button className="btn btn-primary" style={{ flex: 1 }} onClick={() => void openTerminal()}>
                        &gt;_ 打开终端
                      </button>
                      <button className="btn" onClick={() => void revealPath()}>
                        打开位置
                      </button>
                    </div>
                    <div className="tabs">
                      <button className={`tab ${detailTab === 'overview' ? 'active' : ''}`} onClick={() => setDetailTab('overview')}>
                        概览
                      </button>
                      <button className={`tab ${detailTab === 'caps' ? 'active' : ''}`} onClick={() => setDetailTab('caps')}>
                        能力
                      </button>
                      <button className={`tab ${detailTab === 'env' ? 'active' : ''}`} onClick={() => setDetailTab('env')}>
                        环境
                      </button>
                      <button className={`tab ${detailTab === 'usage' ? 'active' : ''}`} onClick={() => setDetailTab('usage')}>
                        使用记录
                      </button>
                    </div>
                    {detailTab === 'overview' && (
                      <div className="kv">
                        <div className="kv-row">
                          <span>安装路径</span>
                          <code>{String(selected.path ?? '—')}</code>
                        </div>
                        <div className="kv-row">
                          <span>版本</span>
                          <span>{selected.version ? String(selected.version) : '未识别'}</span>
                        </div>
                        <div className="kv-row">
                          <span>所属环境</span>
                          <span>{String(selected.environment_id ?? '—')}</span>
                        </div>
                        <div className="kv-row">
                          <span>类型</span>
                          <div className="tags">
                            {toolTags(String(selected.name ?? ''), String(selected.path ?? '')).map((tag) => (
                              <span key={tag} className={tagClass(tag)}>
                                {tag}
                              </span>
                            ))}
                          </div>
                        </div>
                        <div className="kv-row">
                          <span>平台/架构</span>
                          <span>
                            {String(selected.platform ?? '—')} / {String(selected.arch ?? '—')}
                          </span>
                        </div>
                        <div className="kv-row">
                          <span>信任状态</span>
                          <span>{trustLabel(selected.trust)}</span>
                        </div>
                      </div>
                    )}
                    {detailTab === 'caps' && (
                      <div>
                        {listFrom(detailData?.capabilities).length === 0 ? (
                          <div className="empty">暂无登记能力</div>
                        ) : (
                          <ul className="list-clean">
                            {listFrom(detailData.capabilities).map((c: any, i: number) => (
                              <li key={i}>
                                <code>{String(c)}</code>
                              </li>
                            ))}
                          </ul>
                        )}
                        {detailData?.capabilities && !Array.isArray(detailData.capabilities) && (
                          <pre>{JSON.stringify(detailData.capabilities, null, 2)}</pre>
                        )}
                      </div>
                    )}
                    {detailTab === 'env' && (
                      <div className="kv">
                        <div className="kv-row">
                          <span>环境 ID</span>
                          <span>{String(selected.environment_id ?? '—')}</span>
                        </div>
                        <div className="kv-row">
                          <span>说明</span>
                          <span>
                            {KIND_LABEL[String(asRecord(envs.find((e: any) => e.id === selected.environment_id)).kind)] ??
                              '见「环境」页'}
                          </span>
                        </div>
                      </div>
                    )}
                    {detailTab === 'usage' && (
                      <div>
                        {activity.length === 0 ? (
                          <div className="empty">暂无使用记录</div>
                        ) : (
                          <ul className="list-clean">
                            {activity.slice(0, 8).map((a: any, i: number) => (
                              <li key={i}>
                                <div>
                                  <strong>{String(a.kind ?? '')}</strong>
                                  <div className="muted">{String(a.summary ?? '')}</div>
                                </div>
                                <span className="muted">{formatTs(a.ts)}</span>
                              </li>
                            ))}
                          </ul>
                        )}
                      </div>
                    )}
                  </>
                )}
              </aside>
            </div>
          </>
        )}

        {page === 'agents' && (
          <>
            <div className="topbar">
              <div>
                <h1>智能体</h1>
                <p>连接和管理可使用 ToolHub 的 AI 智能体，让它们共享本机工具能力。</p>
              </div>
              <button
                className="btn btn-primary"
                onClick={() => notify('可通过 MCP / CLI 接入；请在智能体侧配置 ToolHub 端点后刷新列表')}
              >
                ＋ 添加智能体
              </button>
            </div>
            <div className="layout-1">
              <div className="card card-pad">
                {agents.length === 0 ? (
                  <div className="empty">
                    暂无已接入的智能体。
                    <div className="muted" style={{ marginTop: 8 }}>
                      本机软件请到「工具」页查看。添加后点「添加智能体」旁的说明可查看接入方式。
                    </div>
                    <button className="btn" style={{ marginTop: 12 }} onClick={() => void load()}>
                      刷新列表
                    </button>
                  </div>
                ) : (
                  <ul className="list-clean">
                    {agents.map((a, i) => (
                      <li key={i}>
                        <div>
                          <strong>{String(a.name ?? a.id ?? '智能体')}</strong>
                          <div className="muted">{String(a.id ?? a.agent_id ?? '')}</div>
                        </div>
                        <StatusBadge text={String(a.status ?? a.state ?? '未知')} tone="ok" />
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            </div>
          </>
        )}

        {page === 'envs' && (
          <>
            <div className="topbar">
              <div>
                <h1>环境</h1>
                <p>查看本机工具所在环境，理解来源、重复安装与调用关系。</p>
              </div>
              <button className="btn" onClick={() => void load()}>
                刷新
              </button>
            </div>
            <div className="layout-1">
              <div className="card card-pad" style={{ marginBottom: 16 }}>
                <div className="tool-list">
                  {envs.length === 0 && <div className="empty">尚未发现环境</div>}
                  {envs.map((e, i) => (
                    <div key={i} className="tool-item" style={{ cursor: 'default' }}>
                      <ToolIcon name={String(e.name ?? '环境')} />
                      <div>
                        <div className="tool-name">{String(e.name ?? e.id ?? '—')}</div>
                        <div className="tool-desc">{String(e.root_path ?? '内置分类环境')}</div>
                        <div className="tags">
                          <span className="tag">{KIND_LABEL[String(e.kind ?? '')] ?? String(e.kind ?? '—')}</span>
                        </div>
                      </div>
                      <div className="tool-meta">
                        <div className="muted">{String(asRecord(e.owner).kind ?? '未知')}</div>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
              <div className="card card-pad">
                <h3 style={{ marginTop: 0 }}>疑似重复</h3>
                {dups.length === 0 ? (
                  <div className="empty">未发现同名多实例</div>
                ) : (
                  <ul className="list-clean">
                    {dups.map((d: any, i) => {
                      const name = Array.isArray(d) ? String(d[0]) : String(d.name ?? '')
                      const count = Array.isArray(d) ? Number(d[1] ?? 0) : Number(d.count ?? 0)
                      return (
                        <li key={i}>
                          <strong>{name}</strong>
                          <button
                            className="btn"
                            onClick={() => {
                              setQuery(name)
                              setPage('tools')
                              notify(`已在工具页搜索「${name}」`)
                            }}
                          >
                            查看实例
                          </button>
                        </li>
                      )
                    })}
                  </ul>
                )}
              </div>
            </div>
          </>
        )}

        {page === 'market' && (
          <>
            <div className="topbar">
              <div>
                <h1>市场</h1>
                <p>发现可接入 ToolHub 的工具模板、技能包与智能体扩展。</p>
              </div>
              <button
                className="btn btn-primary"
                onClick={() => {
                  setAddToolOpen(true)
                  setPage('tools')
                  notify('可在「工具」页添加本机工具；模板市场即将接入')
                }}
              >
                添加本机工具
              </button>
            </div>
            <div className="layout-1">
              <div className="card card-pad">
                <h3>本地模板入口</h3>
                <ul className="list-clean">
                  <li>
                    <div>
                      <strong>从本机路径安装/识别</strong>
                      <div className="muted">输入已安装软件路径，执行定向扫描并入库。</div>
                    </div>
                    <button
                      className="btn btn-primary"
                      onClick={() => {
                        setPage('tools')
                        setAddToolOpen(true)
                      }}
                    >
                      去添加
                    </button>
                  </li>
                  <li>
                    <div>
                      <strong>导入工具清单</strong>
                      <div className="muted">使用「设置 → 导入配置」导入 toolhub.report JSON。</div>
                    </div>
                    <button
                      className="btn"
                      onClick={() => {
                        setPage('settings')
                        setImportOpen(true)
                      }}
                    >
                      去导入
                    </button>
                  </li>
                </ul>
              </div>
            </div>
          </>
        )}

        {page === 'tasks' && (
          <>
            <div className="topbar">
              <div>
                <h1>任务</h1>
                <p>查看智能体通过 ToolHub 发起的任务、执行状态与结果记录。</p>
              </div>
              <button className="btn" onClick={() => void load()}>
                刷新
              </button>
            </div>
            <div className="layout-1">
              <div className="card card-pad">
                <ul className="list-clean">
                  {activity.length === 0 && <div className="empty">暂无任务记录</div>}
                  {activity.map((a, i) => (
                    <li key={i}>
                      <div>
                        <strong>{String(a.kind ?? '活动')}</strong>
                        <div className="muted">{String(a.summary ?? '')}</div>
                      </div>
                      <span className="muted">{formatTs(a.ts)}</span>
                    </li>
                  ))}
                </ul>
              </div>
            </div>
          </>
        )}

        {page === 'settings' && (
          <>
            <div className="topbar">
              <div>
                <h1>设置</h1>
                <p>配置 ToolHub 的扫描、权限、界面与智能体连接行为。</p>
              </div>
            </div>
            <div className="layout-2">
              <div className="card">
                <div className="setting-row">
                  <div>
                    <h3>默认扫描模式</h3>
                    <div className="muted">设置添加工具或启动时的默认扫描模式。</div>
                  </div>
                  <div className="seg">
                    <button className={scanMode === 'quick' ? 'active' : ''} onClick={() => void persistScanMode('quick')}>
                      快速扫描
                    </button>
                    <button className={scanMode === 'full' ? 'active' : ''} onClick={() => void persistScanMode('full')}>
                      深度扫描
                    </button>
                  </div>
                </div>
                <div className="setting-row">
                  <div>
                    <h3>启动时自动扫描</h3>
                    <div className="muted">应用启动时自动扫描本地已安装的工具。</div>
                  </div>
                  <button
                    className={`switch ${prefs.autoScan ? 'on' : ''}`}
                    onClick={() => void updatePrefs({ ...prefs, autoScan: !prefs.autoScan })}
                  >
                    <i />
                  </button>
                </div>
                <div className="setting-row">
                  <div>
                    <h3>显示隐藏工具</h3>
                    <div className="muted">在工具列表中显示已隐藏/失效的工具。</div>
                  </div>
                  <button
                    className={`switch ${prefs.showHidden ? 'on' : ''}`}
                    onClick={() => void updatePrefs({ ...prefs, showHidden: !prefs.showHidden })}
                  >
                    <i />
                  </button>
                </div>
                <div className="setting-row">
                  <div>
                    <h3>未知工具默认权限</h3>
                    <div className="muted">扫描到未识别的新工具时的默认权限设置。</div>
                  </div>
                  <select
                    className="select"
                    value={prefs.unknownPerm}
                    onChange={(e) => void saveUnknownPerm(e.target.value as UiPrefs['unknownPerm'])}
                  >
                    <option value="ask">询问我</option>
                    <option value="deny">拒绝</option>
                    <option value="allow">允许</option>
                  </select>
                </div>
                <div className="setting-row">
                  <div>
                    <h3>日志保留时长</h3>
                    <div className="muted">设置操作日志和运行日志的本地保留时间。</div>
                  </div>
                  <select
                    className="select"
                    value={prefs.retainDays}
                    onChange={(e) => void updatePrefs({ ...prefs, retainDays: e.target.value })}
                  >
                    <option value="7">7 天</option>
                    <option value="30">30 天</option>
                    <option value="90">90 天</option>
                  </select>
                </div>
                <div className="setting-row">
                  <div>
                    <h3>默认执行策略</h3>
                    <div className="muted">当智能体请求调用工具时的执行策略。</div>
                  </div>
                  <div className="seg">
                    <button
                      className={prefs.execPolicy === 'toolhub' ? 'active' : ''}
                      onClick={() => void updatePrefs({ ...prefs, execPolicy: 'toolhub' })}
                    >
                      优先 ToolHub
                    </button>
                    <button
                      className={prefs.execPolicy === 'sandbox' ? 'active' : ''}
                      onClick={() => void updatePrefs({ ...prefs, execPolicy: 'sandbox' })}
                    >
                      允许回退到沙箱
                    </button>
                  </div>
                </div>
                <div className="setting-row">
                  <div>
                    <h3>配置管理</h3>
                    <div className="muted">导出完整报告与设置，或从 JSON 文件导入。</div>
                  </div>
                  <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
                    <button className="btn" onClick={() => setImportOpen(true)}>
                      导入配置
                    </button>
                    <button className="btn btn-primary" onClick={() => void exportConfig()} disabled={busy}>
                      {busy ? '导出中…' : '导出配置'}
                    </button>
                    <button className="btn btn-danger" onClick={() => void resetSettings()}>
                      重置默认设置
                    </button>
                  </div>
                </div>

                {importOpen && (
                  <div className="setting-row">
                    <div>
                      <h3>导入配置文件</h3>
                      <div className="muted">支持导出生成的 toolhub-export.json</div>
                    </div>
                    <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap', minWidth: 280 }}>
                      <input
                        className="select"
                        style={{ minWidth: 220 }}
                        value={importPath}
                        onChange={(e) => setImportPath(e.target.value)}
                        placeholder="C:\Users\...\toolhub-export.json"
                      />
                      <button className="btn btn-primary" onClick={() => void importConfig()} disabled={busy}>
                        导入
                      </button>
                      <button className="btn" onClick={() => setImportOpen(false)}>
                        取消
                      </button>
                    </div>
                  </div>
                )}
              </div>

              <aside>
                <div className="card card-pad" style={{ marginBottom: 16 }}>
                  <h3 style={{ marginTop: 0 }}>当前配置摘要</h3>
                  <div className="kv">
                    <div className="kv-row">
                      <span>扫描模式</span>
                      <span>{scanMode === 'quick' ? '快速扫描' : '深度扫描'}</span>
                    </div>
                    <div className="kv-row">
                      <span>启动时自动扫描</span>
                      <StatusBadge text={prefs.autoScan ? '已开启' : '已关闭'} tone={prefs.autoScan ? 'ok' : 'muted'} />
                    </div>
                    <div className="kv-row">
                      <span>显示隐藏工具</span>
                      <StatusBadge text={prefs.showHidden ? '已开启' : '已关闭'} tone={prefs.showHidden ? 'ok' : 'muted'} />
                    </div>
                    <div className="kv-row">
                      <span>工具数量</span>
                      <span>{String(statusRec.tool_count ?? tools.length)}</span>
                    </div>
                    <div className="kv-row">
                      <span>候选数量</span>
                      <span>{String(statusRec.candidate_count ?? 0)}</span>
                    </div>
                    <div className="kv-row">
                      <span>协议版本</span>
                      <span>{String(statusRec.protocol_version ?? '—')}</span>
                    </div>
                    <div className="kv-row">
                      <span>服务状态</span>
                      <StatusBadge text="正常" />
                    </div>
                  </div>
                </div>
                <div className="card card-pad">
                  <h3 style={{ marginTop: 0 }}>安全</h3>
                  <p className="muted" style={{ marginTop: 0 }}>
                    策略修改需控制器权限；普通客户端不可削弱「拒绝」规则。未知可执行文件不会被自动运行。
                  </p>
                  <div style={{ display: 'flex', gap: 8, flexDirection: 'column' }}>
                    <button
                      className="btn"
                      onClick={() => {
                        setPage('agents')
                        notify('可在智能体页刷新连接状态')
                      }}
                    >
                      管理连接
                    </button>
                    <button
                      className="btn"
                      onClick={() => {
                        setPage('tasks')
                        void load()
                      }}
                    >
                      查看安全活动
                    </button>
                  </div>
                </div>
              </aside>
            </div>
          </>
        )}
      </section>
    </div>
  )
}
