export type ExtensionState = { revision: number; data: unknown }
export const extensionStateLimit = 1048576
const limit = extensionStateLimit
export function extensionState(storage: Storage, packageId: string, operation: unknown, revision?: unknown, data?: unknown): ExtensionState {
  const key = 'th.extension-state.v1.' + packageId
  const raw = storage.getItem(key)
  let current: ExtensionState = { revision: 0, data: null }
  if (raw) {
    const parsed = JSON.parse(raw)
    if (!Number.isSafeInteger(parsed.revision) || parsed.revision < 0 || !('data' in parsed)) throw Error('扩展预设存储损坏，请先备份并恢复')
    current = parsed
  }
  if (operation === 'get') return current
  if (operation !== 'set' || !Number.isSafeInteger(revision) || revision !== current.revision) throw Error('预设已在其他窗口更新，请重新打开页面后再保存')
  const serialized = JSON.stringify(data)
  if (!serialized || new TextEncoder().encode(serialized).length > limit) throw Error('扩展预设最多保存 1 MiB，请减少预设数量')
  if (current.revision === Number.MAX_SAFE_INTEGER) throw Error('扩展存储版本已达上限')
  const next = { revision: current.revision + 1, data: JSON.parse(serialized) }
  storage.setItem(key, JSON.stringify(next))
  return next
}
