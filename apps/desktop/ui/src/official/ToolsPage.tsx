import { invoke } from '@tauri-apps/api/core'
import ToolsList from '../ToolsList'
import ToolIcon from '../ToolIcon'
import AgentHelp from '../AgentHelp'
import StartupSetting from '../StartupSetting'
import ManagedTasks from '../ManagedTasks'
import type { WorkbenchContext, UiPrefs } from './Workbench'
import { KIND_LABEL, TRUST_LABEL, listFrom, toolKindCount, asRecord, parseMaybeJson, trustLevel, trustLabel, formatTs, toolTags, tagClass, StatusBadge } from './Workbench'

const rpc = (method: string, params: unknown = {}) => invoke<any>('rpc', {method, params})
export default function ToolsPage(props: WorkbenchContext) {
  const { grouped, setGrouped, envs, activity, status, query, setQuery, selected, setSelected, detailTab, setDetailTab, detailData, loading, busy, prefs, addToolOpen, setAddToolOpen, addToolPath, setAddToolPath, pinnedExecutionSupported, searchRef, notify, fail, onScan, openTerminal, revealPath, addToolByPath, inventoryCounts, cats, cat, setCat, shown, refresh } = props
  return (
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
                <ToolsList key={JSON.stringify([query.trim(), cat, prefs.showHidden, grouped])} items={shown} grouped={grouped} selected={selected} onSelect={setSelected} loading={loading} tags={toolTags} tagClass={tagClass} trustLabel={trustLabel} badge={item => <StatusBadge text={item.status === 'available' ? '已安装' : item.status === 'extension_disabled' ? '扩展已停用' : item.status === 'missing' ? '不可用' : String(item.status ?? '未知')} tone={item.status === 'available' ? 'ok' : 'warn'} />} />
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
                          <StatusBadge text={selected.status === 'available' ? '已安装' : selected.status === 'extension_disabled' ? '扩展已停用' : String(selected.status ?? '')} />
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
                    <div className="program-actions"><button className="btn" disabled={selected.status !== 'available'} onClick={() => void rpc('registry.prefer_instance',{id:selected.id,clear:selected.preferred===true}).then(()=>refresh('默认路径已保存，执行仍需权限')).catch(fail)}>{selected.preferred?'取消默认路径':'设为默认路径'}</button><button className="btn" onClick={() => void rpc('registry.health_instance',{id:selected.id}).then(result=>refresh(result.path_exists?'文件可读取，尚未执行验证':'路径已失效')).catch(fail)}>检查文件</button><button className="btn" disabled={selected.status !== 'available'} onClick={() => void invoke<string|null>('pick_program_path',{directory:true}).then(project=>project&&rpc('registry.prefer_instance',{id:selected.id,project})).then(()=>notify('项目默认路径已保存')).catch(fail)}>设为项目默认</button>{trustLevel(selected.trust)==='unknown'&&<button className="btn" onClick={()=>void rpc('registry.correct',{id:selected.id,trust:'user_trusted'}).then(()=>refresh('已信任此路径；后续执行遵循自动批准设置')).catch(fail)}>信任此路径</button>}{detailData?.version_probe_args&&<button className="btn" disabled={!pinnedExecutionSupported||trustLevel(selected.trust)==='unknown'||selected.status!=='available'} title={!pinnedExecutionSupported?'macOS 预览版暂不支持身份绑定执行':undefined} onClick={()=>void (async()=>{try{const params={instance_id:selected.id,args:detailData.version_probe_args,cwd:selected.path.slice(0,Math.max(selected.path.lastIndexOf('/'),selected.path.lastIndexOf('\\'))),timeout_ms:10000};const requested=await rpc('execute.approval_request',params);const approval=await rpc('execute.approve',{request_id:requested.request_id});const result=await rpc('execute.tool',{...params,approval_id:approval.approval_id,session_id:approval.session_id});if(result.status==='success')await rpc('registry.health_instance',{id:selected.id,execution_id:result.execution_id});await refresh(result.status==='success'?'版本检查运行成功':'版本检查：'+result.status)}catch(e){fail(e)}})()}>执行版本检查（本次授权）</button>}</div>
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
  )
}
