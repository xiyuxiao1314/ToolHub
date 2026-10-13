import { invoke } from '@tauri-apps/api/core'
import ToolsList from '../ToolsList'
import ToolIcon from '../ToolIcon'
import AgentHelp from '../AgentHelp'
import StartupSetting from '../StartupSetting'
import ManagedTasks from '../ManagedTasks'
import type { WorkbenchContext, UiPrefs } from './Workbench'
import { KIND_LABEL, TRUST_LABEL, listFrom, toolKindCount, asRecord, parseMaybeJson, trustLevel, trustLabel, formatTs, toolTags, tagClass, StatusBadge } from './Workbench'

const rpc = (method: string, params: unknown = {}) => invoke<any>('rpc', {method, params})
export default function EnvsPage(props: WorkbenchContext) {
  const { setPage, tools, envs, dups, setQuery, notify, setCat, refresh } = props
  return (
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
  )
}
