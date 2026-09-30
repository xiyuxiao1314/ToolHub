import { useCallback, useEffect, useState } from 'react'
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

  const load = useCallback(async () => {
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
      setData(d)
    } catch (e: any) {
      setError(String(e))
    } finally {
      setLoading(false)
    }
  }, [page, query])

  useEffect(() => {
    localStorage.setItem('th.page', page)
    void load()
  }, [page, load])

  return (
    <div className="shell">
      <header>
        <h1>ToolHub</h1>
        <nav aria-label="primary">
          {PAGES.map((p) => (
            <button
              key={p}
              className={p === page ? 'active' : ''}
              onClick={() => setPage(p)}
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
              {Array.isArray(data?.activity?.result || data?.activity) ? (
                <ul>
                  {((data.activity.result ?? data.activity) as any[]).slice(0, 20).map((a, i) => (
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
                {((data?.result ?? data ?? []) as any[]).map((t, i) => (
                  <tr key={i}>
                    <td>{String(t.name)}</td>
                    <td>{String(t.version ?? '')}</td>
                    <td>{String(t.trust ?? '')}</td>
                    <td>
                      <code>{String(t.path ?? '')}</code>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
            {((data?.result ?? data ?? []) as any[]).length === 0 && <Empty text="无匹配工具" />}
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
            {Array.isArray(data?.result ?? data) && ((data.result ?? data) as any[]).length === 0 ? (
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
