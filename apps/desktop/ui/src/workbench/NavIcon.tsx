const paths: Record<string, string> = {
  workspace: 'M3 3h7v7H3z M14 3h7v7h-7z M3 14h7v7H3z M14 14h7v7h-7z',
  agents: 'M8 4h8 M12 2v2 M5 8h14v12H5z M2 12v4 M22 12v4 M9 12v1 M15 12v1 M9 17h6',
  extensions: 'M4 4h6v4a2 2 0 1 0 4 0V4h6v6h-4a2 2 0 1 0 0 4h4v6h-6v-4a2 2 0 1 0-4 0v4H4v-6h4a2 2 0 1 0 0-4H4z',
  settings: 'M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8 M9 3h6l1 3 3 1 2 5-2 5-3 1-1 3H9l-1-3-3-1-2-5 2-5 3-1z',
  tools: 'M4 8h16v12H4z M8 8V4h8v4 M4 12h16 M10 12v3h4v-3',
  programs: 'M4 4h16v13H4z M8 21h8 M12 17v4 M10 8l5 3-5 3z',
  artifacts: 'M3 7l9-4 9 4v11l-9 4-9-4z M3 7l9 4 9-4 M12 11v11 M8 5l9 4',
  envs: 'M4 7h16 M4 17h16 M8 4v6 M16 14v6',
  skills: 'M4 4h6a3 3 0 0 1 2 2 3 3 0 0 1 2-2h6v15h-6a3 3 0 0 0-2 2 3 3 0 0 0-2-2H4z M12 6v15',
  tasks: 'M8 4h8v4H8z M8 6H5v15h14V6h-3 M8 12h8 M8 16h5',
  market: 'M4 9h16v12H4z M3 9l2-6h14l2 6 M9 21v-7h6v7 M3 9c0 4 5 4 5 0 0 4 8 4 8 0 0 4 5 4 5 0',
  'agent-guide': 'M8 7l-5 5 5 5 M16 7l5 5-5 5 M14 4l-4 16',
  'extension-apps': 'M4 4h6v6H4z M14 4h6v6h-6z M4 14h6v6H4z M14 14h6v6h-6z',
  'extension-catalog': 'M12 3v12 M7 10l5 5 5-5 M4 16v5h16v-5',
  'extension-appearance': 'M12 3a9 9 0 1 0 0 18h2a2 2 0 0 0 0-4h-1a2 2 0 0 1 0-4h4a4 4 0 0 0 4-4c0-3-4-6-9-6 M7 9h.01 M10 6h.01 M16 7h.01 M6 14h.01',
  search: 'M10.5 3a7.5 7.5 0 1 0 0 15 7.5 7.5 0 0 0 0-15 M16 16l5 5',
  pin: 'M9 3h6l-1 6 4 4v2H6v-2l4-4z M12 15v7',
  collapse: 'M4 4h16v16H4z M9 4v16 M15 9l-3 3 3 3',
  arrow: 'M5 12h14 M14 7l5 5-5 5',
}
export default function NavIcon({ id, fallback, size = 20 }: { id: string; fallback?: string; size?: number }) {
  const path = paths[id]
  if (!path) return <span className="navigation-glyph" aria-hidden="true">{fallback || '◈'}</span>
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.65" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d={path}/></svg>
}
