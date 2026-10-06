import { useEffect, useRef, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import cargo from './assets/tool-brands/cargo.png'
import rust from './assets/tool-brands/rust.svg'
import git from './assets/tool-brands/git.svg'
import python from './assets/tool-brands/python.svg'
import nodejs from './assets/tool-brands/nodejs.svg'

type Pixels = { width: number; height: number; rgba: number[] }
const cache = new Map<string, Promise<string | null>>()

function brandIcon(name: string, path: string): string | null {
  const label = name.trim().toLowerCase()
  const executable = path.replace(/\\/g, '/').split('/').filter(Boolean).pop()?.replace(/\.(exe|cmd|bat)$/i, '').toLowerCase()
  if (/^cargo\b/.test(label) || executable === 'cargo') return cargo
  if (/^rust(?:c|up)?\b/.test(label) || executable === 'rustc' || executable === 'rustup') return rust
  if (/^git\b/.test(label) || executable === 'git') return git
  if (/^python\b/.test(label) || /^python(?:\d+(?:\.\d+)*)?$/.test(executable ?? '')) return python
  if (/^node(?:\.js)?\b/.test(label) || executable === 'node') return nodejs
  return null
}

function embeddedIcon(path: string, version: string): Promise<string | null> {
  const key = `${path}\n${version}`
  let request = cache.get(key)
  if (!request) {
    request = invoke<Pixels | null>('tool_file_icon', { path }).then(pixels => {
      if (!pixels || pixels.width !== 64 || pixels.height !== 64 || pixels.rgba.length !== 64 * 64 * 4) return null
      const canvas = document.createElement('canvas')
      canvas.width = pixels.width; canvas.height = pixels.height
      const context = canvas.getContext('2d')
      if (!context) return null
      context.putImageData(new ImageData(new Uint8ClampedArray(pixels.rgba), pixels.width, pixels.height), 0, 0)
      return canvas.toDataURL('image/png')
    }).catch(() => null)
    if (cache.size >= 256) cache.delete(cache.keys().next().value!)
    cache.set(key, request)
  }
  return request
}

export default function ToolIcon({ name, path = '', version = '', size = 48 }: { name: string; path?: string; version?: string; size?: number }) {
  const wrapper = useRef<HTMLDivElement>(null)
  const [native, setNative] = useState<{ key: string; source: string } | null>(null)
  const [failedImages, setFailedImages] = useState<Set<string>>(() => new Set())
  const key = `${path}\n${version}`
  const fallback = brandIcon(name, path)
  let source = native?.key === key ? native.source : fallback
  if (source && failedImages.has(source)) source = fallback && !failedImages.has(fallback) ? fallback : null
  useEffect(() => {
    if (!path || !wrapper.current) return
    let alive = true
    let observer: IntersectionObserver | undefined
    const read = () => {
      observer?.disconnect()
      void embeddedIcon(path, version).then(source => { if (alive && source) setNative({ key, source }) })
    }
    if (typeof IntersectionObserver === 'undefined') read()
    else {
      observer = new IntersectionObserver(entries => { if (entries.some(e => e.isIntersecting)) read() }, { rootMargin: '120px' })
      observer.observe(wrapper.current)
    }
    return () => { alive = false; observer?.disconnect() }
  }, [path, version, key])
  return <div ref={wrapper} className="tool-icon" data-icon-source={source ? (native?.key === key && source === native.source ? 'file' : 'brand') : 'generic'} style={{ width: size, height: size }} title={source ? `${name} 图标` : `${name}：无可用图标`}>
    {source ? <img src={source} alt="" aria-hidden="true" onError={() => setFailedImages(previous => new Set([...previous, source]))} /> : <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" aria-hidden="true"><rect x="3" y="4" width="18" height="16" rx="3" /><path d="m7 9 3 3-3 3m6 0h4" /></svg>}
  </div>
}
