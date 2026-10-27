/** Delivery outcomes are independent of whether a third-party host has loaded a Skill. */
export const syncStatusLabels: Record<string, string> = {
  recorded: '已有同步记录', not_distributed: '待同步', written: '已同步', present: '已有相同内容',
  update_available: '可更新', conflict: '本地有差异', missing: '文件已移除', incompatible: '不兼容',
  target_unavailable: '目录不可用', failed: '同步失败', saved: '已收录', registered: '已登记',
  files_preserved: '保留文件', portable: '通用', host_specific: '宿主专属', needs_review: '待核对',
  unsupported_platform: '平台不适用',
}

export const missingSkillHelp = '先在目标智能体中查找此 Skill。已经能找到时无需操作；尚未出现时，再刷新技能列表或新开会话。ToolHub 不检测目标智能体的加载状态。'

export function summarizeSync(results: ReadonlyArray<{status?: string}>) {
  const synced = results.filter(r => r.status === 'written').length
  const unchanged = results.filter(r => r.status === 'present').length
  const needsAttention = results.length - synced - unchanged
  const outcomes: string[] = []
  if (synced) outcomes.push(`已同步到 ${synced} 个智能体`)
  if (unchanged) outcomes.push(`${unchanged} 个目标已有相同内容，无需重复同步`)
  if (needsAttention) outcomes.push(`${needsAttention} 个目标未同步，请查看对应原因`)
  return {
    synced, unchanged, needsAttention,
    message: outcomes.join('；') || '未收到同步结果，请刷新状态后核对',
  }
}
