import { invoke } from '@tauri-apps/api/core'
import ToolsList from '../ToolsList'
import ToolIcon from '../ToolIcon'
import AgentHelp from '../AgentHelp'
import StartupSetting from '../StartupSetting'
import ManagedTasks from '../ManagedTasks'
import type { WorkbenchContext } from './Workbench'
import { asRecord, parseMaybeJson, trustLevel, trustLabel, formatTs, toolTags, tagClass, StatusBadge } from './Workbench'

const rpc = (method: string, params: unknown = {}) => invoke<any>('rpc', {method, params})
export default function TasksPage(props: WorkbenchContext) {
  const { activity, notify, refresh } = props
  return (
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
  )
}
