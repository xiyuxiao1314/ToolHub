import { useCallback, useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'

type StartupStatus = {
  supported: boolean; enabled: boolean; registered: boolean; command_matches: boolean
  backend: 'task_scheduler' | 'legacy_run' | 'none'
  last_run_local: string | null; task_result: string | null
  last_startup: { at_ms: number; pid: number; stage: string; error: string | null } | null
}

export default function StartupSetting({ notify }: { notify: (message: string) => void }) {
  const [status, setStatus] = useState<StartupStatus | null>(null)
  const [pending, setPending] = useState(false)
  const [error, setError] = useState('')
  const refresh = useCallback(async () => {
    setPending(true)
    try { setStatus(await invoke<StartupStatus>('autostart_status')); setError('') } catch (e) { setError(String(e)) }
    finally { setPending(false) }
  }, [])
  useEffect(() => { void refresh() }, [refresh])
  const change = async (enabled: boolean) => {
    setPending(true)
    try {
      const updated = await invoke<StartupStatus>('autostart_set', { enabled })
      setStatus(updated)
      if (updated.enabled !== enabled || (enabled && !updated.command_matches)) {
        setError('Windows 启动配置未通过校验，请重新修复。')
      } else {
        setError('')
        notify(updated.enabled ? '已开启开机自启动，Windows 登录 15 秒后打开' : '已关闭开机自启动')
      }
    } catch (e) { setError(String(e)) } finally { setPending(false) }
  }
  const repair = status?.registered && (status.backend === 'legacy_run' || !status.command_matches)
  return <div className="setting-row">
    <div>
      <h3>开机自启动</h3>
      <div className="muted">Windows 登录 15 秒后自动打开 ToolHub，仅对当前用户生效。启动失败时自动重试。</div>
      {status && !status.supported && <div className="muted">当前平台暂不支持。</div>}
      {repair && <div className="help-warning">
        {status.backend === 'legacy_run' ? '当前使用旧版启动登记，建议升级为可检查运行结果的登录任务。' : '登录任务的配置与当前应用不一致。'}
        <button className="text-btn" disabled={pending} onClick={() => void change(true)}>修复并启用</button>
      </div>}
      {status?.backend === 'task_scheduler' && <>
        <div className="muted">启动方式：Windows 登录任务{!status.enabled && '（已禁用）'}</div>
        <div className="muted">{status.last_run_local ? `${status.last_run_local} · ` : ''}{status.task_result}</div>
        {status.last_startup && <div className={status.last_startup.error ? 'help-warning' : 'muted'}>
          最近启动检查：{new Date(status.last_startup.at_ms).toLocaleString()} · {status.last_startup.stage === 'window_created' ? '应用窗口已创建' : status.last_startup.stage === 'startup_failed' ? '应用启动失败' : '进程已启动'}
          {status.last_startup.error && ` · ${status.last_startup.error}`}
        </div>}
        <button className="text-btn" disabled={pending} onClick={() => void refresh()}>刷新启动状态</button>
      </>}
      {error && <div className="help-warning" role="alert">{error} <button className="text-btn" disabled={pending} onClick={() => void refresh()}>重试读取</button></div>}
    </div>
    <button role="switch" aria-label="开机自启动" aria-checked={status?.enabled ?? false} disabled={pending || !status?.supported || !!error} className={`switch ${status?.enabled ? 'on' : ''}`} onClick={() => void change(!status?.enabled)}><i /></button>
  </div>
}
