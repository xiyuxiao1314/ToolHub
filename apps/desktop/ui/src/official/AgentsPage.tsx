import { invoke } from '@tauri-apps/api/core'
import ToolsList from '../ToolsList'
import ToolIcon from '../ToolIcon'
import AgentHelp from '../AgentHelp'
import StartupSetting from '../StartupSetting'
import ManagedTasks from '../ManagedTasks'
import type { WorkbenchContext } from './Workbench'
import { asRecord, parseMaybeJson, trustLevel, trustLabel, formatTs, toolTags, tagClass, StatusBadge } from './Workbench'

const rpc = (method: string, params: unknown = {}) => invoke<any>('rpc', {method, params})
export default function AgentsPage(props: WorkbenchContext & { guide?: boolean }) {
  const { agents, notify, fail, refresh } = props
  return (
          <>
            <div className="topbar">
              <div>
                <h1>{props.guide ? '接入说明' : '智能体'}</h1>
                <p>连接和管理可使用 ToolHub 的 AI 智能体，让它们共享本机工具能力。</p>
              </div>
              <button className="btn" onClick={()=>props.setPage("skills")}>Skill 同步与目录</button>
              <button
                className="btn btn-primary"
                onClick={() => props.setPage(props.guide ? 'agents' : 'agent-guide')}
              >
                {props.guide ? '查看智能体' : '接入说明'}
              </button>
            </div>
            <div className="layout-1">
              {props.guide && <AgentHelp notify={notify} fail={fail} refresh={() => void refresh('智能体列表已刷新')} />}
              {!props.guide && <div className="card card-pad">
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
              </div>}
            </div>
          </>
  )
}
