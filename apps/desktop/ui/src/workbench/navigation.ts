import type { NavItem } from './host'

export type GroupId = 'workspace' | 'agents' | 'extensions' | 'settings'
export const GROUPS: { id: GroupId; label: string; description: string }[] = [
  { id: 'workspace', label: '工作空间', description: '本机资源与常用能力' },
  { id: 'agents', label: '智能体', description: '连接你的 AI 工作伙伴' },
  { id: 'extensions', label: '扩展中心', description: '发现、管理与定制能力' },
  { id: 'settings', label: '设置', description: '管理 ToolHub 的运行方式' },
]
const workspaceIds = new Set(['tools', 'programs', 'services', 'artifacts', 'envs', 'skills', 'tasks'])
export const isApplication = (item: NavItem) => !!(item.cli || item.view)
export function groupFor(item?: NavItem): GroupId {
  if (item?.id === 'agents' || item?.id === 'agent-guide') return 'agents'
  if (item?.id === 'settings') return 'settings'
  if (item?.id === 'workspace' || (item && workspaceIds.has(item.id) && !isApplication(item))) return 'workspace'
  return 'extensions'
}
export function groupItems(nav: NavItem[], group: GroupId, pins: string[]) {
  if (group === 'workspace') return nav.filter(item => (workspaceIds.has(item.id) && !isApplication(item)) || (isApplication(item) && pins.includes(item.id)))
  if (group === 'extensions') {
    const order = ['extension-apps', 'extensions', 'extension-catalog', 'extension-appearance', 'market']
    return nav.filter(item => groupFor(item) === group && !isApplication(item)).sort((a, b) => order.indexOf(a.id) - order.indexOf(b.id))
  }
  return nav.filter(item => groupFor(item) === group)
}

export type NavigationPrefs = { pins: string[]; last: Partial<Record<GroupId, string>>; collapsed: boolean }
export const NAV_PREFS_KEY = 'th.navigation.v1'
export function parseNavigationPrefs(raw: unknown): NavigationPrefs {
  const value = raw && typeof raw === 'object' ? raw as Record<string, unknown> : {}
  const pins = Array.isArray(value.pins) ? [...new Set(value.pins.filter((id): id is string => typeof id === 'string' && id.length <= 256))].slice(0, 8) : []
  const last: NavigationPrefs['last'] = {}
  if (value.last && typeof value.last === 'object') {
    for (const { id } of GROUPS) {
      const page = (value.last as Record<string, unknown>)[id]
      if (typeof page === 'string' && page.length <= 256) last[id] = page
    }
  }
  return { pins, last, collapsed: value.collapsed === true }
}
export function readNavigationPrefs(): NavigationPrefs {
  try { return parseNavigationPrefs(JSON.parse(localStorage.getItem(NAV_PREFS_KEY) || '{}')) }
  catch { return parseNavigationPrefs(null) }
}
export function writeNavigationPrefs(prefs: NavigationPrefs) {
  try { localStorage.setItem(NAV_PREFS_KEY, JSON.stringify(prefs)) } catch { /* Navigation still works when storage is unavailable. */ }
}
