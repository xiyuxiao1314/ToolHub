import { useCallback, useEffect, useRef, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'

type Json = Record<string, unknown> | unknown[] | string | number | boolean | null

async function rpc(method: string, params: Json = {}): Promise<any> {
  return invoke('rpc', { method, params })
}

const PAGES = [
  'Overview',
  'Tools',
  'Environments',
  'Capabilities',
  'Skills',
  'Agents',
  'Activity',
  'Security',
  'Settings',
] as const
type Page = (typeof PAGES)[number]

function Card({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="card">
      <h2>{title}</h2>
      {children}
    </section>
  )
}

function Empty({ text }: { text: string }) {
  return <p className="muted">{text}</p>
}

export default function App() {
  const [page, setPage] = useState<Page>(
    () => (localStorage.getItem('th.page') as Page) || 'Overview',
  )
  const [data, setData] = useState<any>(null)
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  const [query, setQuery] = useState('python')
  const loadSeq = useRef(0)

  const load = useCallback(async () => {
    const seq = ++loadSeq.current
    setLoading(true)
    setError(null)
    try {
      let d: any = null
      switch (page) {
        case 'Overview':
          d = { status: await rpc('status'), activity: await rpc('activity.list', {}) }
          break
        case 'Tools':
          d = await rpc('registry.search', { query })
          break
        case 'Environments':
          d = { envs: await rpc('environment.list', {}), dups: await rpc('environment.duplicates', {}) }
          break
        case 'Capabilities':
          d = await rpc('resolve.capability', {
            capability: 'language.python.execute',
          })
          break
        case 'Skills':
          d = await rpc('skill.list', {})
          break
        case 'Agents':
          d = await rpc('agent.list', {})
          break
        case 'Activity':
          d = await rpc('activity.list', {})
          break
        case 'Security':
          d = await rpc('policy.get', {})
          break
        case 'Settings':
          d = {
            versions: await invoke('app_versions'),
            status: await rpc('status', {}),
          }
          break
      }
      if (seq !== loadSeq.current) return
      setData(d)
    } catch (e: any) {
      if (seq !== loadSeq.current) return
      setError(String(e))
    } finally {
      if (seq === loadSeq.current) setLoading(false)
    }
  }, [page, query])

  useEffect(() => {
    localStorage.setItem('th.page', page)
    void load()
  }, [page, load])

  const switchPage = (p: Page) => {
    // Drop the previous page payload before the next load so render never
    // maps a stale non-array onto a new page's list UI.
    setData(null)
    setError(null)
    setLoading(true)
    if (p === page) {
      void load()
    } else {
      setPage(p)
    }
  }

  const listFrom = (value: unknown): any[] => {
    if (Array.isArray(value)) return value
    if (value && typeof value === 'object' && Array.isArray((value as any).result)) {
      return (value as any).result
    }
    return []
  }

  const trustLabel = (raw: unknown): string => {
    if (raw == null) return ''
    if (typeof raw === 'string') {
      try {
        const parsed = JSON.parse(raw)
        return String(parsed?.level ?? parsed?.kind ?? raw)
      } catch {
        return raw
      }
    }
    if (typeof raw === 'object') {
      const obj = raw as any
      return String(obj.level ?? obj.kind ?? '')
    }
    return String(raw)
  }

  return (
    <div className="shell">
      <header>
        <h1>ToolHub</h1>
        <nav aria-label="primary">
          {PAGES.map((p) => (
            <button
              key={p}
              className={p === page ? 'active' : ''}
              onClick={() => switchPage(p)}
              aria-current={p === page ? 'page' : undefined}
            >
              {p}
            </button>
          ))}
        </nav>
      </header>
      <main>
        {loading && <Empty text="加载中…" />}
        {error && (
          <Card title="错误">
            <p className="error" role="alert">
              {error}
            </p>
            <button className="primary" onClick={() => void load()}>
              重试
            </button>
          </Card>
        )}
        {!loading && !error && page === 'Overview' && (
          <>
            <Card title="Overview">
              <p>
                工具：<b>{String(data?.status?.result?.tool_count ?? data?.status?.tool_count ?? 0)}</b>
                　候选：<b>{String(data?.status?.result?.candidate_count ?? data?.status?.candidate_count ?? 0)}</b>
              </p>
              <button className="primary" onClick={() => void rpc('scan.start', { mode: 'quick' }).then(() => load())}>
                快速扫描
              </button>
            </Card>
            <Card title="Activity">
              {listFrom(data?.activity).length > 0 ? (
                <ul>
                  {listFrom(data?.activity).slice(0, 20).map((a, i) => (
                    <li key={i}>
                      {String(a.ts ?? '')} — {String(a.kind ?? '')} — {String(a.summary ?? '')}
                    </li>
                  ))}
                </ul>
              ) : (
                <Empty text="暂无活动" />
              )}
            </Card>
          </>
        )}
        {!loading && !error && page === 'Tools' && (
          <Card title="Tools">
            <label>
              搜索{' '}
              <input value={query} onChange={(e) => setQuery(e.target.value)} />
            </label>
            <table>
              <thead>
                <tr>
                  <th>名称</th>
                  <th>版本</th>
                  <th>信任</th>
                  <th>路径</th>
                </tr>
              </thead>
              <tbody>
                {listFrom(data).slice(0, 50).map((t, i) => (
                  <tr key={i}>
                    <td>{String(t.name)}</td>
                    <td>{String(t.version ?? '')}</td>
                    <td>{trustLabel(t.trust)}</td>
                    <td>
                      <code>{String(t.path ?? t.canonical_path ?? '')}</code>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
            {listFrom(data).length === 0 && <Empty text="无匹配工具" />}
            {listFrom(data).length > 50 && <Empty text={`仅显示前 50 条（共 ${listFrom(data).length} 条）`} />}
          </Card>
        )}
        {!loading && !error && page === 'Environments' && (
          <Card title="Environments">
            <pre>{JSON.stringify(data, null, 2)}</pre>
          </Card>
        )}
        {!loading && !error && page === 'Capabilities' && (
          <Card title="Capabilities / Resolve">
            <pre>{JSON.stringify(data, null, 2)}</pre>
          </Card>
        )}
        {!loading && !error && page === 'Skills' && (
          <Card title="Skills">
            {listFrom(data).length === 0 ? (
              <Empty text="尚未注册技能" />
            ) : (
              <pre>{JSON.stringify(data, null, 2)}</pre>
            )}
          </Card>
        )}
        {!loading && !error && page === 'Agents' && (
          <Card title="Agents">
            <pre>{JSON.stringify(data, null, 2)}</pre>
          </Card>
        )}
        {!loading && !error && page === 'Activity' && (
          <Card title="Activity">
            <pre>{JSON.stringify(data, null, 2)}</pre>
          </Card>
        )}
        {!loading && !error && page === 'Security' && (
          <Card title="Security">
            <pre>{JSON.stringify(data, null, 2)}</pre>
            <p className="muted">策略修改需控制器权限；普通客户端不可削弱 Deny。</p>
          </Card>
        )}
        {!loading && !error && page === 'Settings' && (
          <Card title="Settings">
            <pre>{JSON.stringify(data, null, 2)}</pre>
          </Card>
        )}
      </main>
    </div>
  )
}
