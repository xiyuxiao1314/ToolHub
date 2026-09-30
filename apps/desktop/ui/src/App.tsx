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
  if (n.includes('java')) return { bg: '#e76f00', text: 'Jv' }
  if (n.includes('rust') || n.includes('cargo')) return { bg: '#b7410e', text: 'Rs' }
  if (n.includes('docker')) return { bg: '#2496ed', text: 'Dk' }
  return { bg: '#64748b', text: name.slice(0, 2).toUpperCase() }
}

function toolTags(name: string, path: string): string[] {
  const n = `${name} ${path}`.toLowerCase()
  const tags: string[] = []
  if (n.includes('python') || n.includes('node') || n.includes('java') || n.includes('rust') || n.includes('cargo') || n.includes('go'))
    tags.push('Runtime')
  if (n.includes('.exe') || n.includes('\\cmd\\') || n.includes('\\bin\\')) tags.push('CLI')
  if (n.includes('sdk') || n.includes('jdk')) tags.push('SDK')
  if (n.includes('code') || n.includes('git')) tags.push('DevOps')
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

function ToolsPage({
  tools,
  query,
  setQuery,
  selected,
  setSelected,
  onScan,
}: {
  tools: any[]
  query: string
  setQuery: (v: string) => void
  selected: any | null
  setSelected: (t: any) => void
  onScan: () => void
}) {
  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase()
    if (!q) return tools
    return tools.filter((t) => `${t.name ?? ''} ${t.path ?? ''} ${t.definition_id ?? ''}`.toLowerCase().includes(q))
  }, [tools, query])

  const cats = useMemo(() => {
    const all = tools.length
    const count = (k: string) =>
      tools.filter((t) => toolTags(String(t.name ?? ''), String(t.path ?? '')).some((x) => x.toLowerCase() === k.toLowerCase())).length
    return [
      { label: `全部 (${all})`, key: 'all' },
      { label: `CLI (${count('CLI')})`, key: 'cli' },
      { label: `Runtime (${count('Runtime')})`, key: 'runtime' },
      { label: `SDK (${count('SDK')})`, key: 'sdk' },
      { label: `DevOps (${count('DevOps')})`, key: 'devops' },
    ]
  }, [tools])

  const [cat, setCat] = useState('all')
  const shown = useMemo(() => {
    if (cat === 'all') return filtered
    return filtered.filter((t) => toolTags(String(t.name ?? ''), String(t.path ?? '')).some((x) => x.toLowerCase() === cat))
  }, [filtered, cat])

  return (
    <>
      <div className="topbar">
        <div>
          <h1>工具</h1>
          <p>管理和使用本地及远程工具，让智能体拥有更强的能力。</p>
        </div>
        <div style={{ display: 'flex', gap: 10 }}>
          <button className="btn" onClick={onScan}>
            扫描本机
          </button>
          <button className="btn btn-primary">＋ 添加工具</button>
        </div>
      </div>
      <div className="layout-2">
        <div>
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
            {shown.length === 0 && <div className="empty">没有匹配的工具</div>}
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
                  <div style={{ marginTop: 8, display: 'flex', gap: 8, alignItems: 'center' }}>
                    <StatusBadge text={selected.status === 'available' ? '已安装' : String(selected.status ?? '')} />
                    <StatusBadge text={trustLabel(selected.trust)} tone={trustLevel(selected.trust) === 'known' ? 'ok' : 'muted'} />
                  </div>
                </div>
              </div>
              <p className="muted" style={{ marginTop: 0 }}>
                {String(selected.name ?? '')}
                {selected.version ? ` 版本 ${String(selected.version)}。` : '。'}
                路径：
                <code>{String(selected.path ?? '—')}</code>
              </p>
              <div className="tags" style={{ marginBottom: 14 }}>
                {toolTags(String(selected.name ?? ''), String(selected.path ?? '')).map((tag) => (
                  <span key={tag} className={tagClass(tag)}>
                    {tag}
                  </span>
                ))}
              </div>
              <button className="btn btn-primary" style={{ width: '100%' }}>
                &gt;_ 打开终端
              </button>
              <div className="tabs">
                <button className="tab active">概览</button>
                <button className="tab">能力</button>
                <button className="tab">环境</button>
                <button className="tab">使用记录</button>
              </div>
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
            </>
          )}
        </aside>
      </div>
    </>
  )
}

function AgentsPage({ agents }: { agents: any[] }) {
  return (
    <>
      <div className="topbar">
        <div>
          <h1>智能体</h1>
          <p>连接和管理可使用 ToolHub 的 AI 智能体，让它们共享本机工具能力。</p>
        </div>
        <button className="btn btn-primary">＋ 添加智能体</button>
      </div>
      <div className="layout-1">
        <div className="card card-pad">
          {agents.length === 0 ? (
            <div className="empty">
              暂无已接入的智能体。
              <div className="muted" style={{ marginTop: 8 }}>
                本机软件请到「工具」页查看。
              </div>
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
  )
}

function EnvsPage({ envs, dups }: { envs: any[]; dups: any[] }) {
  return (
    <>
      <div className="topbar">
        <div>
          <h1>环境</h1>
          <p>查看本机工具所在环境，理解来源、重复安装与调用关系。</p>
        </div>
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
                    <span className="muted">{count} 个安装副本</span>
                  </li>
                )
              })}
            </ul>
          )}
        </div>
      </div>
    </>
  )
}

function MarketPage() {
  return (
    <>
      <div className="topbar">
        <div>
          <h1>市场</h1>
          <p>发现可接入 ToolHub 的工具模板、技能包与智能体扩展。</p>
        </div>
      </div>
      <div className="layout-1">
        <div className="card card-pad">
          <div className="empty">市场内容接入中。当前可先使用本机扫描到的工具。</div>
        </div>
      </div>
    </>
  )
}

function TasksPage({ activity }: { activity: any[] }) {
  return (
    <>
      <div className="topbar">
        <div>
          <h1>任务</h1>
          <p>查看智能体通过 ToolHub 发起的任务、执行状态与结果记录。</p>
        </div>
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
  )
}

function SettingsPage({ status }: { status: any }) {
  const s = asRecord(status?.result ?? status)
  const [autoScan, setAutoScan] = useState(true)
  const [hidden, setHidden] = useState(false)
  const [scanMode, setScanMode] = useState<'quick' | 'full'>('quick')
  const [perm, setPerm] = useState('ask')
  const [retain, setRetain] = useState('30')
  const [exec, setExec] = useState('toolhub')

  return (
    <>
      <div className="topbar">
        <div>
          <h1>设置</h1>
          <p>配置 ToolHub 的扫描、权限、界面与智能体连接行为。</p>
        </div>
      </div>
      <div className="layout-2">
        <div className="card">
          <div className="tabs" style={{ padding: '12px 16px 0', margin: 0 }}>
            <button className="tab active">通用</button>
            <button className="tab">扫描规则</button>
            <button className="tab">权限</button>
            <button className="tab">智能体连接</button>
            <button className="tab">外观</button>
          </div>
          <div className="setting-row">
            <div>
              <h3>默认扫描模式</h3>
              <div className="muted">设置添加工具或启动时的默认扫描模式。</div>
            </div>
            <div className="seg">
              <button className={scanMode === 'quick' ? 'active' : ''} onClick={() => setScanMode('quick')}>
                快速扫描
              </button>
              <button className={scanMode === 'full' ? 'active' : ''} onClick={() => setScanMode('full')}>
                深度扫描
              </button>
            </div>
          </div>
          <div className="setting-row">
            <div>
              <h3>启动时自动扫描</h3>
              <div className="muted">应用启动时自动扫描本地已安装的工具。</div>
            </div>
            <button className={`switch ${autoScan ? 'on' : ''}`} onClick={() => setAutoScan(!autoScan)} aria-label="启动时自动扫描">
              <i />
            </button>
          </div>
          <div className="setting-row">
            <div>
              <h3>显示隐藏工具</h3>
              <div className="muted">在工具列表中显示已隐藏的工具。</div>
            </div>
            <button className={`switch ${hidden ? 'on' : ''}`} onClick={() => setHidden(!hidden)} aria-label="显示隐藏工具">
              <i />
            </button>
          </div>
          <div className="setting-row">
            <div>
              <h3>未知工具默认权限</h3>
              <div className="muted">扫描到未识别的新工具时的默认权限设置。</div>
            </div>
            <select className="select" value={perm} onChange={(e) => setPerm(e.target.value)}>
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
            <select className="select" value={retain} onChange={(e) => setRetain(e.target.value)}>
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
              <button className={exec === 'toolhub' ? 'active' : ''} onClick={() => setExec('toolhub')}>
                优先 ToolHub
              </button>
              <button className={exec === 'sandbox' ? 'active' : ''} onClick={() => setExec('sandbox')}>
                允许回退到沙箱
              </button>
            </div>
          </div>
          <div className="setting-row">
            <div>
              <h3>配置管理</h3>
              <div className="muted">导入或导出您的设置配置，或恢复到默认设置。</div>
            </div>
            <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
              <button className="btn">导入配置</button>
              <button className="btn">导出配置</button>
              <button className="btn btn-danger">重置默认设置</button>
            </div>
          </div>
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
                <StatusBadge text={autoScan ? '已开启' : '已关闭'} tone={autoScan ? 'ok' : 'muted'} />
              </div>
              <div className="kv-row">
                <span>显示隐藏工具</span>
                <StatusBadge text={hidden ? '已开启' : '已关闭'} tone={hidden ? 'ok' : 'muted'} />
              </div>
              <div className="kv-row">
                <span>工具数量</span>
                <span>{String(s.tool_count ?? 0)}</span>
              </div>
              <div className="kv-row">
                <span>候选数量</span>
                <span>{String(s.candidate_count ?? 0)}</span>
              </div>
              <div className="kv-row">
                <span>协议版本</span>
                <span>{String(s.protocol_version ?? '—')}</span>
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
            <button className="btn" style={{ width: '100%' }}>
              管理连接
            </button>
          </div>
        </aside>
      </div>
    </>
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
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)
  const loadSeq = useRef(0)

  const load = useCallback(async () => {
    const seq = ++loadSeq.current
    setLoading(true)
    setError(null)
    try {
      const [toolRows, agentRows, envRows, dupRows, actRows, st] = await Promise.all([
        rpc('registry.search', { query: '' }),
        rpc('agent.list', {}),
        rpc('environment.list', {}),
        rpc('environment.duplicates', {}),
        rpc('activity.list', {}),
        rpc('status', {}),
      ])
      if (seq !== loadSeq.current) return
      const rows = listFrom(toolRows)
      setTools(rows)
      setAgents(listFrom(agentRows))
      setEnvs(listFrom(envRows))
      setDups(listFrom(dupRows))
      setActivity(listFrom(actRows))
      setStatus(st)
      setSelected((prev: any) => {
        if (prev) {
          const again = rows.find((r: any) => r.id === prev.id)
          if (again) return again
        }
        return rows[0] ?? null
      })
    } catch (e: any) {
      if (seq === loadSeq.current) setError(String(e?.message ?? e))
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

  const onScan = async () => {
    try {
      setError(null)
      setLoading(true)
      await rpc('scan.start', { mode: 'quick' })
      await load()
    } catch (e: any) {
      setError(String(e?.message ?? e))
      setLoading(false)
    }
  }

  return (
    <div className="app-shell">
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
          <button className="quick-btn">＋ 添加工具</button>
          <button className="quick-btn" onClick={() => void onScan()}>
            🔍 扫描本机
          </button>
          <button className="quick-btn">📦 从模板安装</button>
          <button className="quick-btn">⬇️ 导入配置</button>
        </div>
        <div className="side-foot">
          <div className="muted">工具总数</div>
          <strong>{tools.length}</strong>
          <div className="progress">
            <i />
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
        {loading && !error && (
          <div className="layout-1">
            <div className="empty">加载中…</div>
          </div>
        )}

        {!loading &&
          (page === 'tools' ? (
            <ToolsPage
              tools={tools}
              query={query}
              setQuery={setQuery}
              selected={selected}
              setSelected={setSelected}
              onScan={() => void onScan()}
            />
          ) : page === 'agents' ? (
            <AgentsPage agents={agents} />
          ) : page === 'envs' ? (
            <EnvsPage envs={envs} dups={dups} />
          ) : page === 'market' ? (
            <MarketPage />
          ) : page === 'tasks' ? (
            <TasksPage activity={activity} />
          ) : (
            <SettingsPage status={status} />
          ))}
      </section>
    </div>
  )
}
