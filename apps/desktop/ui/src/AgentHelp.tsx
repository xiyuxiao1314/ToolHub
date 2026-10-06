import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'

type HostProfile = { id: string; name: string; location: string; note: string; docs: string; format: string; config: string }
type Integration = { cli_path: string; cli_available: boolean; host_profiles: HostProfile[]; mcp_config: string; mimo_config: string; codex_config: string; cli_check: string; skill_markdown: string; bootstrap_prompt: string }

export async function copyText(text: string) {
  if (navigator.clipboard?.writeText) {
    try { await navigator.clipboard.writeText(text); return } catch { /* Some WebView versions only allow the selection API. */ }
  }
  const selection = document.createElement('textarea')
  selection.value = text
  selection.readOnly = true
  selection.style.cssText = 'position:fixed;left:-9999px;top:0'
  document.body.appendChild(selection)
  try {
    selection.select()
    if (!document.execCommand('copy')) throw new Error('复制失败，请展开内容后手动复制')
  } finally { selection.remove() }
}

export default function AgentHelp({ notify, fail, refresh }: { notify: (message: string) => void; fail: (error: unknown) => void; refresh: () => void }) {
  const [info, setInfo] = useState<Integration | null>(null)
  const [error, setError] = useState('')
  const [hostId, setHostId] = useState('generic')
  useEffect(() => {
    let alive = true
    void invoke<Integration>('agent_integration').then((value) => { if (alive) setInfo(value) }).catch((e) => { if (alive) setError(String(e)) })
    return () => { alive = false }
  }, [])
  const selected = info?.host_profiles.find(host => host.id === hostId)
  const task = info && selected ? `${info.bootstrap_prompt}\n\n本次目标客户端：${selected.name}\n配置位置：${selected.location}\n${selected.note}\n本次应采用以下配置格式，按实际范围合并：\n\n\`\`\`${selected.format === 'toml' ? 'toml' : 'json'}\n${selected.config}\n\`\`\`\n${selected.docs ? `官方说明：${selected.docs}` : ''}` : info?.bootstrap_prompt ?? ''
  const copy = async (text: string, label: string) => {
    try { await copyText(text); notify(`${label}已复制`) } catch (e) { fail(e) }
  }
  return <div className="card card-pad agent-help">
    <h3>让 Agent 先从 ToolHub 找工具</h3>
    <p>支持本地 stdio MCP 的客户端均可接入。选择客户端后复制本机接入任务，让它配置 MCP、安装通用指引并验证连接。</p>
    {error ? <p role="alert">接入信息读取失败：{error}</p> : !info ? <p className="muted">正在读取本机接入信息…</p> : <>
      {!info.cli_available && <p className="help-warning">应用同目录缺少 toolhub.exe，请先补齐 CLI 和 toolhubd.exe，再连接 Agent。</p>}
      <div className="help-section">
        <label htmlFor="agent-host">目标客户端</label>
        <select id="agent-host" value={hostId} onChange={event => setHostId(event.target.value)}>
          {info.host_profiles.map(host => <option key={host.id} value={host.id}>{host.name}</option>)}
        </select>
        {selected && <><p className="muted">配置位置：{selected.location}</p><p className="muted">{selected.note}</p></>}
      </div>
      <div className="help-section">
        <div className="help-heading"><h3>本机接入任务</h3><button className="btn btn-primary" onClick={() => void copy(task, '本机接入任务')}>复制给 Agent 的接入任务</button></div>
        <p className="muted">包含当前安装路径、所选客户端配置和检查步骤。支持 Skills 的宿主保存到自己的 Skill 目录；其他宿主可用同一正文作为任务指引。</p>
        <details><summary>预览接入任务</summary><pre className="skill-preview">{task}</pre></details>
      </div>
      {selected && <div className="help-section">
        <div className="help-heading"><h3>{selected.name} MCP 配置</h3><button className="btn" onClick={() => void copy(selected.config, 'MCP 配置')}>复制 MCP 配置</button></div>
        <p className="muted">只合并 toolhub 服务，保留原有配置。模板支持不代表该客户端已经在本机通过实际连接验证。</p>
        <pre>{selected.config}</pre>
        <details><summary>配置来源与验证范围</summary><p>模板按客户端配置格式提供，已做本地格式检查；实际权限、启用、项目覆盖和重载由宿主管理。请完成一次工具查询后确认接入成功。</p>{selected.docs && <p className="muted">官方文档：{selected.docs}</p>}</details>
        <details><summary>查看 CLI 检查命令</summary><pre>{info.cli_check}</pre></details>
      </div>}
      <div className="help-section">
        <div className="help-heading"><h3>通用接入 Skill</h3><button className="btn" onClick={() => void copy(info.skill_markdown, 'Agent Skill')}>复制接入 Skill</button></div>
        <p className="muted">查询本机工具和提交待确认入口的通用流程。ToolHub 不自动汇总各客户端的专属 Skills。</p>
        <details><summary>预览完整 SKILL.md</summary><pre className="skill-preview">{info.skill_markdown}</pre></details>
      </div>
    </>}
    <p className="muted">列表分别记录软件检测、默认配置和历史握手/调用。检测到软件或收到自报名称都不代表当前在线，也不授予执行权限。</p>
    <button className="btn" onClick={refresh}>刷新列表</button>
  </div>
}
