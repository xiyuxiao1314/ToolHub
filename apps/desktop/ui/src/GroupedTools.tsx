import { useMemo } from 'react'
import ToolIcon from './ToolIcon'
export default function GroupedTools({items,selected,onSelect}:{items:any[],selected:any,onSelect:(item:any)=>void}) {
 const groups=useMemo(()=>groupTools(items),[items])
 return <div className="tool-groups">{groups.map(([id,rows])=>{const main=rows.find(r=>r.preferred)||rows.find(r=>r.status==='available')||rows[0];return <details className="card tool-group" key={id} open={rows.some(r=>r.id===selected?.id)}><summary><ToolIcon name={main.name} path={main.path}/><span><strong>{main.name}</strong><small className="muted">{rows.length} 个安装实例 · {main.preferred?'已设默认路径':'按版本与环境选择'}</small></span></summary><div className="tool-list">{rows.map(row=><button key={row.id} className={`tool-item ${row.id===selected?.id?'active':''}`} onClick={()=>onSelect(row)}><div><strong>{row.version||'版本未知'} {row.preferred?' · 默认':''}</strong><div className="tool-desc" title={row.path}>{row.path}</div><div className="muted">{row.environment_id} · {row.status==='available'?'已登记路径':'路径失效'}{row.health?.checked_at?' · 文件检查 '+new Date(row.health.checked_at).toLocaleDateString():''}</div></div></button>)}</div></details>})}</div>
}

export function groupTools(items: any[]): [string, any[]][] {
 const map = new Map<string, any[]>();
 for (const item of items) { const key = item.definition_id || item.id; const rows = map.get(key); if (rows) rows.push(item); else map.set(key, [item]); }
 return [...map];
}
