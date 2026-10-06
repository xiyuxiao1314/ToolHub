import ToolsList from './ToolsList'
import ManagedTasks from './ManagedTasks'
import Skills from './Skills'
import Market from './Market'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import Programs from './Programs'
import ToolIcon from './ToolIcon'
import AgentHelp from './AgentHelp'
import StartupSetting from './StartupSetting'
import brandIcon from './assets/toolhub-a.png'

type Json = Record<string, unknown> | unknown[] | string | number | boolean | null

async function rpc(method: string, params: Json = {}): Promise<any> {
  return invoke('rpc', { method, params })
}

type PageId = 'tools' | 'programs' | 'agents' | 'envs' | 'skills' | 'market' | 'tasks' | 'settings'

const NAV: { id: PageId; label: string; icon: string }[] = [
  { id: 'tools', label: '工具', icon: '🧰' },
  { id: 'programs', label: '程序', icon: '🚀' },
  { id: 'agents', label: '智能体', icon: '🤖' },
  { id: 'envs', label: '环境', icon: '🧩' },
  { id: 'skills', label: 'Skill 库', icon: '📚' },
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
    return parsePrefs(JSON.parse(localStorage.getItem('th.prefs') || '{}'))
  } catch {
    return { ...DEFAULT_PREFS }
  }
}

function parsePrefs(raw: unknown): UiPrefs {
  const p = asRecord(raw)
  const next = { ...DEFAULT_PREFS }
  for (const key of ['autoScan', 'showHidden'] as const) {
    if (typeof p[key] === 'boolean') next[key] = p[key]
  }
  if (['ask', 'deny', 'allow'].includes(p.unknownPerm)) next.unknownPerm = p.unknownPerm
  if (['7', '30', '90'].includes(p.retainDays)) next.retainDays = p.retainDays
  if (['toolhub', 'sandbox'].includes(p.execPolicy)) next.execPolicy = p.execPolicy
  return next
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

function toolKindCount(items: any[]): number {
  return new Set(items.map((t) => String(t.definition_id || t.id || t.path))).size
}

function toolTags(name: string, path: string): string[] {
  const n = `${name} ${path}`.toLowerCase()
  const tags: string[] = []
  if (/python|node|java|rust|cargo|golang|go\.exe/.test(n)) tags.push('Runtime')
  if (/\.exe|\\cmd\\|\\bin\\|\.cmd|\.bat/.test(n)) tags.push('CLI')
  if (/ffmpeg|ffprobe|7-?zip|7za?\.exe|imagemagick|magick|pandoc|poppler|pdfto|tesseract|yt-dlp|curl/.test(`${name} ${path.replace(/\\/g, '/').split('/').pop()}`.toLowerCase())) tags.push('实用工具')
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
  const [searchRows, setSearchRows] = useState<any[]>([])
  const [grouped, setGrouped] = useState(() => localStorage.getItem('th.grouped') !== 'false')
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
  const [agentHelpOpen, setAgentHelpOpen] = useState(false)
  const [pinnedExecutionSupported, setPinnedExecutionSupported] = useState(false)
  useEffect(() => { void invoke<{platform:string}>('app_versions').then(info => setPinnedExecutionSupported(['windows','linux'].includes(info.platform))).catch(() => setPinnedExecutionSupported(false)) }, [])
  const loadSeq = useRef(0)
  const startupScan = useRef(false)
  const searchRef = useRef<HTMLInputElement>(null)
  const contentRef = useRef<HTMLElement>(null)

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
      const [toolRows, agentRows, envRows, dupRows, actRows, st, cfg, policy] = await Promise.all([
        rpc('registry.search', { query: '', include_missing: true }),
        rpc('agent.list', {}),
        rpc('environment.list', {}),
        rpc('environment.duplicates', {}),
        rpc('activity.list', {}),
        rpc('status', {}),
        rpc('settings.get', {}),
        rpc('policy.get', {}),
      ])
      if (seq !== loadSeq.current) return false
      const rows = listFrom(toolRows)
      setTools(rows)
      setAgents(listFrom(agentRows))
      setEnvs(listFrom(envRows))
      setDups(listFrom(dupRows))
      setActivity(listFrom(actRows))
      setStatus(st)
      const rule = listFrom(policy?.defaults).find((r: any) => r.scope === 'tool' && r.subject === '*')
      if (rule && ['ask', 'deny', 'allow'].includes(rule.action)) {
        setPrefs((prev) => {
          const next = { ...prev, unknownPerm: rule.action }
          savePrefs(next)
          return next
        })
      }
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
      return true
    } catch (e: any) {
      if (seq === loadSeq.current) fail(e)
      return false
    } finally {
      if (seq === loadSeq.current) setLoading(false)
    }
  }, [])

  useEffect(() => {
    localStorage.setItem('th.page', page)
    if (contentRef.current) contentRef.current.scrollTop = 0
  }, [page])

  useEffect(() => {
    if (!selected?.id) {
      setDetailData(null)
      return
    }
    let alive = true
    setDetailData(null)
    void rpc('registry.inspect_instance', { id: selected.id })
      .then((data) => alive && setDetailData(data))
      .catch(() => alive && setDetailData(null))
    return () => {
      alive = false
    }
  }, [selected])

  const onScan = async (mode: 'quick' | 'full' = scanMode) => {
    if (busy) return
    try {
      setBusy(true)
      setError(null)
      notify(mode === 'full' ? '正在深度扫描…' : '正在快速扫描…')
      const result = await rpc('scan.start', { mode })
      const refreshed = await load()
      if (refreshed) notify(result.status === 'partial' ? '扫描结束，部分路径无法读取；已更新可读取的工具' : '扫描完成')
    } catch (e: any) {
      fail(e)
    } finally {
      setBusy(false)
    }
  }

  useEffect(() => {
    void load()
  }, [load])

  useEffect(() => {
    if (loading || !status || startupScan.current) return
    startupScan.current = true
    if (loadPrefs().autoScan) {
      void rpc('settings.get', {})
        .then((cfg) => onScan(cfg.scan_mode === 'full' ? 'full' : 'quick'))
        .catch(fail)
    }
  }, [loading, status])

  useEffect(() => {
    const shortcut = (e: KeyboardEvent) => {
      if (page === 'programs') return
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault()
        setPage('tools')
        requestAnimationFrame(() => searchRef.current?.focus())
      }
    }
    window.addEventListener('keydown', shortcut)
    return () => window.removeEventListener('keydown', shortcut)
  }, [page])

  const updatePrefs = async (next: UiPrefs) => {
    setPrefs(next)
    savePrefs(next)
    notify('界面设置已保存')
  }

  const persistScanMode = async (mode: 'quick' | 'full') => {
    setBusy(true)
    try {
      const prev = { ...await rpc('settings.get', {}) }
      delete prev.versions
      const cfg = await rpc('settings.set', { settings: { ...prev, scan_mode: mode } })
      setServerSettings(asRecord(cfg))
      setScanMode(mode)
      notify(`默认扫描模式已设为${mode === 'quick' ? '快速扫描' : '深度扫描'}`)
    } catch (e: any) {
      fail(e)
    } finally {
      setBusy(false)
    }
  }

  const exportConfig = async () => {
    try {
      setBusy(true)
      const report = await rpc('export.report', {})
      const settings = await rpc('settings.get', {})
      const payload = JSON.stringify(
        {
          schema: 'toolhub.config/v1',
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
      const data = asRecord(JSON.parse(text))
      const isConfig = data.schema === 'toolhub.config/v1' ||
        (data.schema === 'toolhub.report/v1' && (data.ui_prefs || data.settings))
      if (isConfig) {
        if (!data.settings || typeof data.settings !== 'object' || Array.isArray(data.settings)) {
          throw new Error('配置文件缺少有效的 settings 对象')
        }
        const clean = { ...data.settings }
        delete clean.versions
        await rpc('settings.set', { settings: clean })
        if (data.ui_prefs) {
          const next = parsePrefs(data.ui_prefs)
          await rpc('policy.set', { scope: 'tool', subject: '*', action: next.unknownPerm })
          setPrefs(next)
          savePrefs(next)
        }
      } else if (data.schema === 'toolhub.report/v1' && Array.isArray(data.tools)) {
        await rpc('import.report', { report: data })
      } else {
        throw new Error('不支持的配置格式，请选择 ToolHub 导出的 JSON 文件')
      }
      if (!await load()) return
      setImportOpen(false)
      notify(isConfig ? '配置导入完成；本机工具以实际扫描结果为准' : '已导入工具清单；清单记录需本机扫描确认后才能使用')
    } catch (e: any) {
      fail(e)
    } finally {
      setBusy(false)
    }
  }

  const resetSettings = async () => {
    setBusy(true)
    try {
      await rpc('settings.set', {
        settings: {
          scan_mode: 'quick',
          scan_roots: [],
          resolver_preferences: {},
          privacy: { redact_exports: true },
        },
      })
      await rpc('policy.set', { scope: 'tool', subject: '*', action: 'ask' })
      const next = { ...DEFAULT_PREFS }
      setPrefs(next)
      savePrefs(next)
      setScanMode('quick')
      if (await load()) notify('已重置为默认设置')
    } catch (e: any) {
      fail(e)
    } finally {
      setBusy(false)
    }
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
      notify('已在文件管理器中定位工具')
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
      setError(null)
      const scanRoot = await invoke<string>('scan_root_for_path', { path: root })
      const result = await rpc('scan.start', { mode: 'custom', roots: [scanRoot] })
      if (!result.candidates && result.coverage?.roots_failed?.length) throw new Error('无法读取该路径，请检查路径是否存在及访问权限')
      if (!await load()) return
      setAddToolOpen(false)
      setCat('all')
      setQuery(root)
      notify(result.status === 'partial' ? '已更新工具列表；部分子目录或链接未扫描' : '已扫描该路径并更新工具列表')
    } catch (e: any) {
      fail(e)
    } finally {
      setBusy(false)
    }
  }

  const saveUnknownPerm = async (value: UiPrefs['unknownPerm']) => {
    setBusy(true)
    try {
      await rpc('policy.set', {
        scope: 'tool',
        subject: '*',
        action: value === 'allow' ? 'allow' : value === 'deny' ? 'deny' : 'ask',
      })
      await updatePrefs({ ...prefs, unknownPerm: value })
      notify('工具默认权限已更新；具体工具和命令规则优先')
    } catch (e: any) {
      fail(e)
    } finally {
      setBusy(false)
    }
  }

  const inventoryCounts = useMemo(() => {
    const available = tools.filter((t) => t.status !== 'missing')
    return { registered: tools.length, available: available.length, unavailable: tools.length - available.length, kinds: toolKindCount(available) }
  }, [tools])

  const filteredTools = useMemo(() => {
    const base = query.trim() ? searchRows : tools
    return prefs.showHidden ? base : base.filter(t => t.status !== 'missing')
  }, [tools, searchRows, query, prefs.showHidden])

  useEffect(() => {
    let active = true
    const timer = setTimeout(() => { if(query.trim()) void rpc('registry.search', {query:query.trim(),include_missing:true}).then(rows => { if(active) setSearchRows(listFrom(rows)) }).catch(fail) }, 180)
    return () => { active = false; clearTimeout(timer) }
  }, [query, tools])

  const cats = useMemo(() => {
    const count = (k: string) =>
      filteredTools.filter((t) => toolTags(String(t.name ?? ''), String(t.path ?? '')).some((x) => x.toLowerCase() === k.toLowerCase())).length
    return [
      { label: `全部 (${filteredTools.length})`, key: 'all' },
      { label: `CLI (${count('cli')})`, key: 'cli' },
      { label: `Runtime (${count('runtime')})`, key: 'runtime' },
      { label: `实用工具 (${count('实用工具')})`, key: '实用工具' },
      { label: `SDK (${count('sdk')})`, key: 'sdk' },
      { label: `DevOps (${count('devops')})`, key: 'devops' },
    ]
  }, [filteredTools])

  const [cat, setCat] = useState('all')
  const shown = useMemo(() => {
    if (cat === 'all') return filteredTools
    return filteredTools.filter((t) => toolTags(String(t.name ?? ''), String(t.path ?? '')).some((x) => x.toLowerCase() === cat))
  }, [filteredTools, cat])

  useEffect(() => {
    setSelected((prev: any) => shown.find((t) => t.id === prev?.id) ?? shown[0] ?? null)
  }, [shown])

  const statusRec = asRecord(status?.result ?? status)
  const refresh = async (message: string) => {
    if (await load()) notify(message)
  }

  return (
    <div className="app-shell">
      {toast && <Toast message={toast} onClose={() => setToast(null)} />}
      <aside className="sidebar">
        <div className="sidebar-scroll">
          <div className="brand">
            <img className="brand-badge" src={brandIcon} alt="" aria-hidden="true" />
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
            <button className="quick-btn" onClick={() => void onScan()} disabled={busy}>
              🔍 扫描本机
            </button>
            <button className="quick-btn" onClick={() => setPage('market')}>
              📦 模板与清单
            </button>
            <button className="quick-btn" onClick={() => { setPage('settings'); setImportOpen(true) }}>
              ⬇️ 导入配置
            </button>
          </div>
        </div>
        <div className="side-foot">
          <div className="muted">可用工具</div>
          <strong>{inventoryCounts.available}</strong>
          <div className="progress" title={`可用 ${inventoryCounts.available} / 已登记 ${inventoryCounts.registered}`}>
            <i style={{ width: `${inventoryCounts.registered ? inventoryCounts.available / inventoryCounts.registered * 100 : 0}%` }} />
          </div>
          <div className="muted" style={{ fontSize: 12 }}>
            已登记 {inventoryCounts.registered} 个安装实例<br />
            不可用 {inventoryCounts.unavailable} · 共 {inventoryCounts.kinds} 种可用工具
          </div>
        </div>
      </aside>

      <section className="content" ref={contentRef}>
        {error && (
          <div className="layout-1" style={{ paddingBottom: 0 }}>
            <div className="error-box">{error}</div>
          </div>
        )}

        {page === 'programs' && <Programs notify={notify} />}

        {page === 'tools' && (
          <>
            <div className="topbar">
              <div>
                <h1>工具</h1>
                <p>管理和使用本地及远程工具，让智能体拥有更强的能力。</p>
              </div>
              <div style={{ display: 'flex', gap: 10 }}>
                <button className="btn" onClick={() => void onScan()} disabled={busy}>
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
                    <p className="muted">输入本机已安装工具的绝对路径或目录，将扫描其所在目录。</p>
                    <div style={{ display: 'flex', gap: 8 }}>
                      <input
                        className="select"
                        style={{ flex: 1, minWidth: 0 }}
                        value={addToolPath}
                        onChange={(e) => setAddToolPath(e.target.value)}
                        placeholder="填写工具绝对路径或目录"
                        data-testid="add-tool-path"
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
                      ref={searchRef}
                      value={query}
                      onChange={(e) => setQuery(e.target.value)}
                      placeholder="搜索名称、路径，或压缩视频 / OCR 等任务..."
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
                <div className="tool-count-note" role="status" aria-live="polite">
                  <div>当前 {shown.length} 个安装实例 · {toolKindCount(shown)} 种工具 <button className="btn" onClick={() => {setGrouped(!grouped);localStorage.setItem('th.grouped',String(!grouped))}}>{grouped ? '显示安装实例' : '按工具合并'}</button></div>
                  <div className="muted">
                    分类标签可重叠；不同安装位置分别计数。
                    {!prefs.showHidden && inventoryCounts.unavailable > 0 && ` 已隐藏 ${inventoryCounts.unavailable} 个不可用实例。`}
                  </div>
                </div>
                <ToolsList key={JSON.stringify([query.trim(), cat, prefs.showHidden, grouped])} items={shown} grouped={grouped} selected={selected} onSelect={setSelected} loading={loading} tags={toolTags} tagClass={tagClass} trustLabel={trustLabel} badge={item => <StatusBadge text={item.status === 'available' ? '已安装' : item.status === 'missing' ? '不可用' : String(item.status ?? '未知')} tone={item.status === 'available' ? 'ok' : 'warn'} />} />
              </div>

              <aside className="card card-pad">
                {!selected ? (
                  <div className="empty">选择左侧工具查看详情</div>
                ) : (
                  <>
                    <div className="detail-head">
                      <ToolIcon name={String(selected.name ?? '')} path={String(selected.path ?? '')} version={String(selected.version ?? '')} size={56} />
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
                      <button className="btn btn-primary" style={{ flex: 1 }} onClick={() => void openTerminal()} disabled={selected.platform === 'foreign' || selected.status !== 'available'}>
                        &gt;_ 打开终端
                      </button>
                      <button className="btn" onClick={() => void revealPath()} disabled={selected.platform === 'foreign' || selected.status !== 'available'}>
                        打开位置
                      </button>
                    </div>
                    <div className="program-actions"><button className="btn" disabled={selected.status !== 'available'} onClick={() => void rpc('registry.prefer_instance',{id:selected.id,clear:selected.preferred===true}).then(()=>refresh('默认路径已保存，执行仍需权限')).catch(fail)}>{selected.preferred?'取消默认路径':'设为默认路径'}</button><button className="btn" onClick={() => void rpc('registry.health_instance',{id:selected.id}).then(result=>refresh(result.path_exists?'文件可读取，尚未执行验证':'路径已失效')).catch(fail)}>检查文件</button><button className="btn" disabled={selected.status !== 'available'} onClick={() => void invoke<string|null>('pick_program_path',{directory:true}).then(project=>project&&rpc('registry.prefer_instance',{id:selected.id,project})).then(()=>notify('项目默认路径已保存')).catch(fail)}>设为项目默认</button>{trustLevel(selected.trust)==='unknown'&&<button className="btn" onClick={()=>void rpc('registry.correct',{id:selected.id,trust:'user_trusted'}).then(()=>refresh('已信任此路径；执行仍需本次批准')).catch(fail)}>信任此路径</button>}{detailData?.version_probe_args&&<button className="btn" disabled={!pinnedExecutionSupported||trustLevel(selected.trust)==='unknown'||selected.status!=='available'} title={!pinnedExecutionSupported?'macOS 预览版暂不支持身份绑定执行':undefined} onClick={()=>void (async()=>{try{const params={instance_id:selected.id,args:detailData.version_probe_args,cwd:selected.path.slice(0,Math.max(selected.path.lastIndexOf('/'),selected.path.lastIndexOf('\\'))),timeout_ms:10000};const requested=await rpc('execute.approval_request',params);const approval=await rpc('execute.approve',{request_id:requested.request_id});const result=await rpc('execute.tool',{...params,approval_id:approval.approval_id,session_id:approval.session_id});if(result.status==='success')await rpc('registry.health_instance',{id:selected.id,execution_id:result.execution_id});await refresh(result.status==='success'?'版本检查运行成功':'版本检查：'+result.status)}catch(e){fail(e)}})()}>执行版本检查（本次授权）</button>}</div>
                    {selected.health && <p className="muted">文件检查：{selected.health.path_exists?'可读取':'失效'} · {new Date(selected.health.checked_at).toLocaleString()} · {selected.health.execution_tested?'已执行验证':'尚未执行验证'}</p>}
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
                        近期活动
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
                          <div className="empty">暂无活动记录</div>
                        ) : (
                          <ul className="list-clean" aria-label="全局近期活动">
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

        {page === 'skills' && <Skills notify={notify} />}

        {page === 'agents' && (
          <>
            <div className="topbar">
              <div>
                <h1>智能体</h1>
                <p>连接和管理可使用 ToolHub 的 AI 智能体，让它们共享本机工具能力。</p>
              </div>
              <button
                className="btn btn-primary"
                onClick={() => setAgentHelpOpen((open) => !open)}
              >
                接入说明
              </button>
            </div>
            <div className="layout-1">
              {agentHelpOpen && <AgentHelp notify={notify} fail={fail} refresh={() => void refresh('智能体列表已刷新')} />}
              <div className="card card-pad">
                {agents.length === 0 ? (
                  <div className="empty">
                    尚未检测到智能体或接入配置。
                    <div className="muted" style={{ marginTop: 8 }}>
                      点击「接入说明」查看配置方式；安装好的适配器可通过刷新重新检测。
                    </div>
                    <button className="btn" style={{ marginTop: 12 }} onClick={() => void refresh('智能体列表已刷新')}>
                      刷新列表
                    </button>
                  </div>
                ) : (
                  <><div className="program-actions"><button className="btn" onClick={() => void refresh('智能体列表已刷新')}>重新检测</button><button className="btn" onClick={()=>void invoke<any>('agent_mcp_check').then(r=>notify('ToolHub MCP 自检通过：'+r.tool_count+' 个元工具；宿主需实际加载配置')).catch(fail)}>运行 MCP 自检</button></div><ul className="list-clean agent-inventory">
                    {agents.map((a, i) => (
                      <li key={i}>
                        <div>
                          <strong>{String(a.name ?? a.id ?? '智能体')}</strong>
                          <div className="muted">{String(a.executable ?? '启动文件待定位')}</div>
                          <div className="tags"><span className="tag">{a.configuration?.configured ? (a.configuration.enabled === false ? 'MCP 配置已禁用' : 'MCP 已配置') : (a.configuration?.reader_supported ? '默认位置未发现配置' : '配置位置待确认')}</span></div>
                          {a.configuration?.config_path && <div className="muted">配置位置：{String(a.configuration.config_path)}</div>}
                          {a.configuration?.scope && <div className="muted">配置范围：{String(a.configuration.scope)}</div>}
                          {a.configuration?.note && <div className="muted">{String(a.configuration.note)}</div>}
                          {a.configuration?.error && <div className="help-warning">配置检查：{String(a.configuration.error)}</div>}
                          <div className="muted">最近握手：{a.last_handshake_at ? new Date(a.last_handshake_at).toLocaleString() : '尚无记录'} · 最近调用成功：{a.last_call_at ? new Date(a.last_call_at).toLocaleString() : '尚无记录'}</div>
                          <div className="muted">{a.last_detected_at ? '最近检测：' + new Date(a.last_detected_at).toLocaleString() : ''}</div>
                        </div>
                        <StatusBadge text={a.health === 'detected' ? '已检测到程序' : (a.last_handshake_at || a.last_call_at ? '有 MCP 历史记录' : '当前未检测到，保留记录')} tone="muted" />
                      </li>
                    ))}
                  </ul></>
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
              <button
                className="btn"
                onClick={() => {
                  notify('正在刷新环境列表…')
                  void refresh('环境已刷新')
                }}
              >
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
                        <div className="muted">
                          {TRUST_LABEL[String(asRecord(e.owner).kind)] ??
                            TRUST_LABEL[String(asRecord(e.owner).certainty)] ??
                            '未知'}
                        </div>
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
                              const definition = tools.find((t) => t.definition_id === name)
                              setCat('all')
                              setQuery(String(definition?.name ?? name))
                              setPage('tools')
                              notify(`已在工具页搜索「${name}」`)
                            }}
                          >
                            查看实例 ({count})
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

        {page === 'market' && <Market notify={notify} onLibrary={() => setPage('skills')} onTools={() => setPage('tools')} onAddTool={() => {setPage('tools');setAddToolOpen(true)}} />}

        {page === 'tasks' && (
          <>
            <div className="topbar">
              <div>
                <h1>任务</h1>
                <p>查看 ToolHub 的扫描、配置变更和工具执行活动。</p>
              </div>
              <button
                className="btn"
                onClick={() => {
                  notify('正在刷新任务列表…')
                  void refresh('任务已刷新')
                }}
              >
                刷新
              </button>
            </div>
            <ManagedTasks notify={notify} />
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
                <StartupSetting notify={notify} />
                <div className="setting-row">
                  <div>
                    <h3>默认扫描模式</h3>
                    <div className="muted">用于「扫描本机」按钮和启动自动扫描。</div>
                  </div>
                  <div className="seg">
                    <button className={scanMode === 'quick' ? 'active' : ''} onClick={() => void persistScanMode('quick')} disabled={busy}>
                      快速扫描
                    </button>
                    <button className={scanMode === 'full' ? 'active' : ''} onClick={() => void persistScanMode('full')} disabled={busy}>
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
                    aria-label="启动时自动扫描"
                    aria-pressed={prefs.autoScan}
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
                    aria-label="显示隐藏工具"
                    aria-pressed={prefs.showHidden}
                    className={`switch ${prefs.showHidden ? 'on' : ''}`}
                    onClick={() => void updatePrefs({ ...prefs, showHidden: !prefs.showHidden })}
                  >
                    <i />
                  </button>
                </div>
                <div className="setting-row">
                  <div>
                    <h3>工具默认权限</h3>
                    <div className="muted">适用于无更具体规则的工具；具体工具和命令规则优先。</div>
                  </div>
                  <select
                    className="select"
                    value={prefs.unknownPerm}
                    aria-label="工具默认权限"
                    disabled={busy}
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
                    <div className="muted">自动日志清理尚未实现，当前不会按该值删除日志。</div>
                  </div>
                  <select
                    className="select"
                    value={prefs.retainDays}
                    aria-label="日志保留时长（暂未实现）"
                    disabled
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
                    <div className="muted">ToolHub 提供本机调用；沙箱回退由外部智能体决定。</div>
                  </div>
                  <div className="seg">
                    <button
                      className={prefs.execPolicy === 'toolhub' ? 'active' : ''}
                      disabled
                      onClick={() => void updatePrefs({ ...prefs, execPolicy: 'toolhub' })}
                    >
                      优先 ToolHub
                    </button>
                    <button
                      className={prefs.execPolicy === 'sandbox' ? 'active' : ''}
                      disabled
                      onClick={() => void updatePrefs({ ...prefs, execPolicy: 'sandbox' })}
                    >
                      允许回退到沙箱
                    </button>
                  </div>
                </div>
                <div className="setting-row">
                  <div>
                    <h3>配置管理</h3>
                    <div className="muted">导出设置及工具报告快照；配置导入恢复设置，不复制本机安装记录。</div>
                  </div>
                  <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
                    <input
                      className="select"
                      aria-label="导出文件路径"
                      value={exportPath}
                      onChange={(e) => setExportPath(e.target.value)}
                      placeholder="导出路径（留空使用文档目录）"
                      disabled={busy}
                    />
                    <button className="btn" onClick={() => setImportOpen(true)}>
                      导入配置
                    </button>
                    <button className="btn btn-primary" onClick={() => void exportConfig()} disabled={busy}>
                      {busy ? '导出中…' : '导出配置'}
                    </button>
                    <button className="btn btn-danger" onClick={() => void resetSettings()} disabled={busy}>
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
                      <StatusBadge text={error ? '连接异常' : !status ? '连接中' : loading ? '刷新中' : '正常'} tone={error ? 'warn' : 'ok'} />
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
