import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'

type Json = Record<string, unknown> | unknown[] | string | number | boolean | null

async function rpc(method: string, params: Json = {}): Promise<any> {
  return invoke('rpc', { method, params })
}

type PageId = string


export const TRUST_LABEL: Record<string, string> = {
  unknown: '未知',
  known: '已知',
  probable: '较可能',
  user_trusted: '用户信任',
  blocked: '已阻止',
  user_owned: '用户所有',
}

export const KIND_LABEL: Record<string, string> = {
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

export type UiPrefs = {
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

export function listFrom(value: unknown): any[] {
  if (Array.isArray(value)) return value
  if (value && typeof value === 'object' && Array.isArray((value as any).result)) {
    return (value as any).result
  }
  return []
}

export function asRecord(value: any): Record<string, any> {
  return value && typeof value === 'object' && !Array.isArray(value) ? value : {}
}

export function parseMaybeJson(raw: unknown): any {
  if (typeof raw === 'string') {
    try {
      return JSON.parse(raw)
    } catch {
      return raw
    }
  }
  return raw
}

export function trustLevel(raw: unknown): string {
  const parsed = parseMaybeJson(raw)
  if (parsed && typeof parsed === 'object') return String(parsed.level ?? parsed.kind ?? '')
  return String(parsed ?? '')
}

export function trustLabel(raw: unknown): string {
  const level = trustLevel(raw)
  return TRUST_LABEL[level] ?? (level || '未知')
}

export function formatTs(raw: unknown): string {
  const text = String(raw ?? '')
  if (!text) return '—'
  const d = new Date(text)
  return Number.isNaN(d.getTime()) ? text : d.toLocaleString('zh-CN', { hour12: false })
}

export function toolKindCount(items: any[]): number {
  return new Set(items.map((t) => String(t.definition_id || t.id || t.path))).size
}

export function toolTags(name: string, path: string): string[] {
  const n = `${name} ${path}`.toLowerCase()
  const tags: string[] = []
  if (/python|node|electron|java|rust|cargo|golang|go\.exe/.test(n)) tags.push('Runtime')
  if (/\.exe|\\cmd\\|\\bin\\|\.cmd|\.bat/.test(n)) tags.push('CLI')
  if (/ffmpeg|ffprobe|7-?zip|7za?\.exe|imagemagick|magick|pandoc|poppler|pdfto|tesseract|yt-dlp|curl/.test(`${name} ${path.replace(/\\/g, '/').split('/').pop()}`.toLowerCase())) tags.push('实用工具')
  if (/sdk|jdk/.test(n)) tags.push('SDK')
  if (/git|docker|code/.test(n)) tags.push('DevOps')
  return tags.length ? tags : ['工具']
}

export function tagClass(tag: string): string {
  if (tag === 'Runtime' || tag === 'CLI') return 'tag blue'
  if (tag === 'SDK') return 'tag purple'
  if (tag === 'DevOps') return 'tag orange'
  return 'tag'
}

export function StatusBadge({ text, tone = 'ok' }: { text: string; tone?: 'ok' | 'warn' | 'muted' }) {
  return <span className={`badge badge-${tone}`}>{text}</span>
}

export function Toast({ message, onClose }: { message: string; onClose: () => void }) {
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


import type { NavItem } from '../workbench/host'
export function useWorkbench(enabled: ReadonlySet<string>, nav: NavItem[]) {
  const [page, setPage] = useState<PageId>(() => {
    const saved = localStorage.getItem('th.page') as PageId | null
    return saved || 'tools'
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

  const notify = useCallback((msg: string) => setToast(msg), [])
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
        enabled.has('tools') ? rpc('registry.search', { query: '', include_missing: true }) : Promise.resolve([]),
        rpc('agent.list', {}),
        enabled.has('envs') ? rpc('environment.list', {}) : Promise.resolve([]),
        enabled.has('envs') ? rpc('environment.duplicates', {}) : Promise.resolve([]),
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
  }, [Array.from(enabled).sort().join(',')])

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
    if (loading || !status || startupScan.current || !enabled.has('tools')) return
    startupScan.current = true
    if (loadPrefs().autoScan) {
      void rpc('settings.get', {})
        .then((cfg) => onScan(cfg.scan_mode === 'full' ? 'full' : 'quick'))
        .catch(fail)
    }
  }, [loading, status])

  useEffect(() => {
    const shortcut = (e: KeyboardEvent) => {
      if (page === 'programs' || !enabled.has('tools')) return
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault()
        setPage('tools')
        requestAnimationFrame(() => searchRef.current?.focus())
      }
    }
    window.addEventListener('keydown', shortcut)
    return () => window.removeEventListener('keydown', shortcut)
  }, [page, enabled.has('tools')])

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

  const toggleAutoApproval = async () => {
    if (!serverSettings || busy) return
    setBusy(true)
    try {
      const latest = { ...await rpc('settings.get', {}) }
      delete latest.versions
      const enabled = latest.auto_approve_execution !== true
      const saved = await rpc('settings.set', { settings: { ...latest, auto_approve_execution: enabled } })
      setServerSettings(asRecord(saved))
      notify(saved.auto_approve_execution ? '已开启自动批准，Agent 可直接继续执行' : '已关闭自动批准，后续请求需在任务页确认')
    } catch (e) { fail(e) } finally { setBusy(false) }
  }

  const resetSettings = async () => {
    setBusy(true)
    try {
      await rpc('settings.set', {
        settings: {
          scan_mode: 'quick',
          auto_approve_execution: true,
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
    const available = tools.filter((t) => t.status === 'available')
    return { registered: tools.length, available: available.length, unavailable: tools.length - available.length, kinds: toolKindCount(available) }
  }, [tools])

  const filteredTools = useMemo(() => {
    const base = query.trim() ? searchRows : tools
    return prefs.showHidden ? base : base.filter(t => t.status === 'available')
  }, [tools, searchRows, query, prefs.showHidden])

  useEffect(() => {
    let active = true
    const timer = setTimeout(() => { if(enabled.has('tools') && query.trim()) void rpc('registry.search', {query:query.trim(),include_missing:true}).then(rows => { if(active) setSearchRows(listFrom(rows)) }).catch(fail) }, 180)
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

  return { page, setPage, tools, setTools, searchRows, setSearchRows, grouped, setGrouped, agents, setAgents, envs, setEnvs, dups, setDups, activity, setActivity, status, setStatus, query, setQuery, selected, setSelected, detailTab, setDetailTab, detailData, setDetailData, error, setError, toast, setToast, loading, setLoading, busy, setBusy, prefs, setPrefs, scanMode, setScanMode, serverSettings, setServerSettings, importOpen, setImportOpen, importPath, setImportPath, addToolOpen, setAddToolOpen, addToolPath, setAddToolPath, exportPath, setExportPath, agentHelpOpen, setAgentHelpOpen, pinnedExecutionSupported, setPinnedExecutionSupported, loadSeq, startupScan, searchRef, contentRef, notify, fail, load, onScan, updatePrefs, persistScanMode, exportConfig, importConfig, toggleAutoApproval, resetSettings, openTerminal, revealPath, addToolByPath, saveUnknownPerm, inventoryCounts, filteredTools, cats, cat, setCat, shown, statusRec, refresh }
}
export type WorkbenchContext = ReturnType<typeof useWorkbench>
