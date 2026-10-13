import { invoke } from '@tauri-apps/api/core'
import ToolsList from '../ToolsList'
import ToolIcon from '../ToolIcon'
import AgentHelp from '../AgentHelp'
import StartupSetting from '../StartupSetting'
import ManagedTasks from '../ManagedTasks'
import type { WorkbenchContext, UiPrefs } from './Workbench'
import { KIND_LABEL, TRUST_LABEL, listFrom, toolKindCount, asRecord, parseMaybeJson, trustLevel, trustLabel, formatTs, toolTags, tagClass, StatusBadge } from './Workbench'

const rpc = (method: string, params: unknown = {}) => invoke<any>('rpc', {method, params})
export default function SettingsPage(props: WorkbenchContext) {
  const { setPage, tools, agents, status, error, loading, busy, prefs, scanMode, serverSettings, importOpen, setImportOpen, importPath, setImportPath, exportPath, setExportPath, pinnedExecutionSupported, notify, load, updatePrefs, persistScanMode, exportConfig, importConfig, toggleAutoApproval, resetSettings, saveUnknownPerm, statusRec } = props
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
                <StartupSetting notify={notify} />
                <div className="setting-row">
                  <div>
                    <h3>自动批准工具执行</h3>
                    <div className="muted">默认开启，Agent 调用已信任工具时自动批准，无需等待桌面确认。拒绝规则仍生效；程序收录仍需勾选。</div>
                    {!pinnedExecutionSupported && <div className="muted">当前平台预览版暂不支持受控工具执行。</div>}
                  </div>
                  <button aria-label="自动批准工具执行" aria-pressed={serverSettings?.auto_approve_execution === true}
                    className={`switch ${serverSettings?.auto_approve_execution === true ? 'on' : ''}`}
                    disabled={busy || !serverSettings} onClick={() => void toggleAutoApproval()}><i /></button>
                </div>
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
                    <div className="muted">适用于无更具体规则的工具；具体规则优先。“询问我”在自动批准开启时自动确认，关闭后由你确认。</div>
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
  )
}
