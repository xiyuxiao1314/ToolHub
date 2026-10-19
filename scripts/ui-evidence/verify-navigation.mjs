/** Real Tauri/WebView2 navigation acceptance. Only use an isolated TOOLHUB_REGISTRY.
 * This script installs disposable packages and changes fixture appearance settings.
 * Requires TOOLHUB_NAV_CDP, TOOLHUB_NAV_REGISTRY and TOOLHUB_NAV_FIXTURE=yes.
 */
import { chromium } from 'playwright'
import fs from 'node:fs'
import path from 'node:path'
import assert from 'node:assert/strict'
import { fileURLToPath } from 'node:url'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..')
const output = process.env.TOOLHUB_NAV_OUTPUT || path.join(root, 'output/playwright/navigation')
fs.mkdirSync(output, { recursive: true })
if (!process.env.TOOLHUB_NAV_CDP || !process.env.TOOLHUB_NAV_REGISTRY || process.env.TOOLHUB_NAV_FIXTURE !== 'yes') throw Error('Explicit CDP endpoint, isolated registry path and TOOLHUB_NAV_FIXTURE=yes are required; never run against production')
const browser = await chromium.connectOverCDP(process.env.TOOLHUB_NAV_CDP)
const page = browser.contexts()[0].pages()[0]
page.setDefaultTimeout(15000)
const results = [], errors = []
page.on('pageerror', error => errors.push(String(error)))
const rpc = (method, params = {}) => page.evaluate(({ method, params }) => window.__TAURI_INTERNALS__.invoke('rpc', { method, params }), { method, params })
const secondary = () => page.getByRole('navigation', { name: '二级导航', exact: true })
const primary = () => page.getByRole('navigation', { name: '一级导航', exact: true })
const primaryClick = async name => { await primary().getByRole('button', { name, exact: true }).click() }
const go = async name => { await secondary().getByRole('button', { name, exact: true }).click() }
const heading = async text => { await page.getByRole('heading', { name: text, exact: true, level: 1 }).waitFor() }
const pass = name => { results.push({ name, ok: true }); console.log('PASS', name) }
const refresh = () => page.evaluate(() => window.dispatchEvent(new Event('toolhub:extensions-changed')))
async function install(pkg) {
  const preview = await rpc('extension.preview', { package: pkg })
  await rpc('extension.install', { package: pkg, digest: preview.digest, source: 'local' })
}
const example = name => JSON.parse(fs.readFileSync(path.join(root, `extensions/examples/${name}.toolhub-extension.json`), 'utf8'))
async function layoutCheck() {
  const geometry = await page.evaluate(() => {
    const main = document.querySelector('.workbench-main').getBoundingClientRect()
    const rail = document.querySelector('.primary-rail').getBoundingClientRect()
    const panel = document.querySelector('.secondary-panel').getBoundingClientRect()
    return { width: innerWidth, height: innerHeight, bodyWidth: document.body.scrollWidth, main: { x: main.x, y: main.y, width: main.width, height: main.height }, rail: { x: rail.x, y: rail.y, right: rail.right, bottom: rail.bottom }, panel: { x: panel.x, y: panel.y, right: panel.right, bottom: panel.bottom }, hidden: document.querySelector('.secondary-panel').hidden }
  })
  assert(geometry.bodyWidth <= geometry.width + 1, JSON.stringify(geometry))
  assert(geometry.main.width >= Math.min(400, geometry.width - 76), JSON.stringify(geometry))
  assert(geometry.main.y + geometry.main.height <= geometry.height + 1, JSON.stringify(geometry))
  return geometry
}
try {
  await page.waitForFunction(() => !!window.__TAURI_INTERNALS__ && document.querySelectorAll('.secondary-link').length >= 6)
  const status = await rpc('status')
  assert.equal(path.resolve(status.registry_path).toLowerCase(), path.resolve(process.env.TOOLHUB_NAV_REGISTRY).toLowerCase(), 'Connected desktop uses a different registry')
  assert(!/[\\/]\.toolhub[\\/]registry\.sqlite$/i.test(status.registry_path), 'Refusing the default production registry')
  // Reject accidentally selecting a real user's inventory before any mutation.
  const initial = await rpc('extension.list')
  assert(initial.every(item => item.package.id.startsWith('toolhub.builtin.')), 'Fixture is not fresh: contains nonofficial extensions')
  assert.equal((await rpc('program.list')).length, 0, 'Fixture contains programs')
  await page.evaluate(() => {
    localStorage.setItem('th.prefs', JSON.stringify({ autoScan: false }))
    localStorage.setItem('th.page', 'tools')
    localStorage.removeItem('th.navigation.v1')
  })
  await page.reload()
  await page.setViewportSize({ width: 1440, height: 940 })
  await heading('工具')
  assert.equal(await primary().getByRole('button').count(), 4)
  assert.equal(await secondary().locator('.secondary-link').count(), 6)
  pass('four primary groups and six optional workspace modules')
  for (const label of ['程序', '产物', '环境', 'Skill 库', '任务', '工具']) { await go(label); await heading(label) }
  await primaryClick('智能体'); await heading('智能体')
  await go('接入说明'); await heading('接入说明')
  await page.getByRole('heading', { name: '让 Agent 先从 ToolHub 找工具' }).waitFor()
  await go('已接入智能体'); await heading('智能体')
  await primaryClick('设置'); await heading('设置')
  await primaryClick('工作空间'); await heading('工具')
  pass('all official and core destinations remain reachable')
  await primaryClick('扩展中心'); await heading('扩展应用')
  await go('已安装'); await heading('已安装扩展')
  await page.locator('.market-package').first().waitFor()
  assert.equal(await page.locator('.market-package').count(), 7)
  await go('扩展市场'); await heading('扩展市场')
  await page.locator('.market-package').filter({ hasText: '石墨' }).waitFor()
  assert(await page.locator('.market-package').count() > 7)
  await go('主题与布局'); await heading('主题与布局')
  await page.getByLabel('导航布局', { exact: true }).waitFor()
  pass('installed, local catalog and appearance have distinct views')
  for (const name of ['ffmpeg', 'workspace', 'graphite']) await install(example(name))
  await refresh(); await go('扩展应用'); await heading('扩展应用')
  await page.locator('.application-card').filter({ hasText: '媒体工坊' }).waitFor()
  assert.equal(await page.locator('.application-card').count(), 2)
  assert.equal(await primary().getByRole('button').count(), 4)
  assert.equal(await secondary().getByRole('button', { name: '石墨', exact: true }).count(), 0)
  await page.screenshot({ path: path.join(output, 'applications.png') })
  pass('CLI and page application directory; themes do not create destinations')
  const media = page.locator('.application-card').filter({ has: page.getByRole('heading', { name: '媒体工坊', exact: true }) })
  await media.getByRole('button', { name: '固定到工作空间：媒体工坊', exact: true }).click()
  await primaryClick('工作空间'); await go('媒体工坊'); await heading('媒体工坊')
  await page.reload(); await heading('媒体工坊')
  assert.equal(await primary().getByRole('button', { name: '工作空间', exact: true }).getAttribute('aria-current'), 'true')
  assert.equal(await secondary().getByRole('button', { name: '取消固定：媒体工坊' }).getAttribute('aria-pressed'), 'true')
  await go('程序'); await primaryClick('智能体'); await primaryClick('工作空间'); await heading('程序')
  pass('pins, current destination and per-group history persist across reload')

  // 22 packages × 8 pages, within the real 32-package native limit.
  for (let pack = 0; pack < 22; pack++) {
    const id = `test.navigation${String(pack).padStart(2, '0')}`
    await install({ schema: 'toolhub.extension/v2', id, name: `验收扩展 ${String(pack).padStart(2, '0')}`, version: '1.0.0', description: '二级导航压力验收包，仅用于隔离数据。', author: 'ToolHub QA', license: 'MIT', platforms: ['windows', 'macos'], min_toolhub_version: '0.3.0-beta.3', skills: [], views: Array.from({ length: 8 }, (_, view) => ({ id: `${id}.view${view}`, label: `验收页面 ${String(pack).padStart(2, '0')}-${view}`, icon: '◈', order: 100 + pack * 8 + view, html: '<h1>独立扩展页面</h1><button id="tools">进入工具</button>', css: 'body{font:16px sans-serif;padding:32px}', script: 'document.querySelector("#tools").onclick=()=>toolhub.navigate("tools")' })) })
  }
  await refresh(); await primaryClick('扩展中心'); await go('扩展应用')
  await page.locator('.application-card').nth(177).waitFor()
  assert.equal(await page.locator('.application-card').count(), 178)
  assert.equal(await primary().getByRole('button').count(), 4)
  assert.equal(await page.locator('.navigation-package').count(), 24)
  await page.getByRole('textbox', { name: '筛选二级导航' }).fill('21-7')
  assert.equal(await secondary().locator('.secondary-link').count(), 1)
  await go('验收页面 21-7'); await heading('验收页面 21-7')
  await page.frameLocator('iframe').getByRole('button', { name: '进入工具' }).click()
  await heading('工具')
  pass('178 application pages remain secondary; filtering and real sandbox SDK navigation work')
  await primaryClick('扩展中心'); await go('扩展应用')
  await page.getByRole('textbox', { name: '搜索扩展应用' }).fill('验收页面 00-')
  assert.equal(await page.locator('.application-card').count(), 8)
  for (let index = 0; index < 7; index++) await page.getByRole('button', { name: `固定到工作空间：验收页面 00-${index}`, exact: true }).click()
  await page.getByRole('button', { name: '固定到工作空间：验收页面 00-7', exact: true }).click()
  assert.equal(await page.getByRole('button', { name: '固定到工作空间：验收页面 00-7', exact: true }).getAttribute('aria-pressed'), 'false')
  assert.equal(await page.evaluate(() => JSON.parse(localStorage.getItem('th.navigation.v1')).pins.length), 8)
  pass('eight workspace pins cap; extension search matches individual pages')
  await primaryClick('工作空间'); await go('媒体工坊')
  const ffmpeg = (await rpc('extension.list')).find(item => item.package.id === example('ffmpeg').id)
  await rpc('extension.set_enabled', { id: ffmpeg.package.id, enabled: false, expected_digest: ffmpeg.digest })
  await refresh(); await heading('已安装扩展')
  assert.equal(await secondary().getByRole('button', { name: '媒体工坊', exact: true }).count(), 0)
  await rpc('extension.set_enabled', { id: ffmpeg.package.id, enabled: true, expected_digest: ffmpeg.digest })
  await refresh(); await primaryClick('工作空间')
  await secondary().getByRole('button', { name: '媒体工坊', exact: true }).waitFor()
  pass('disabled active extension recovers safely; re-enabling restores the pin')

  await primaryClick('扩展中心'); await go('主题与布局')
  const geometry = []
  for (const layout of ['left', 'right', 'top']) {
    for (const density of ['comfortable', 'compact']) {
      await page.getByLabel('导航布局', { exact: true }).selectOption(layout)
      await page.waitForFunction(value => document.querySelector('.app-shell').classList.contains('workbench-layout-' + value), layout)
      await page.getByLabel('界面密度', { exact: true }).selectOption(density)
      await page.waitForFunction(value => document.querySelector('.app-shell').classList.contains('workbench-density-' + value), density)
      geometry.push({ layout, density, ...await layoutCheck() })
    }
    await page.screenshot({ path: path.join(output, `appearance-${layout}.png`) })
  }
  await page.getByLabel('工作台主题', { exact: true }).selectOption(example('graphite').id)
  await page.waitForFunction(() => document.documentElement.style.getPropertyValue('--bg') !== '')
  await page.screenshot({ path: path.join(output, 'theme-top.png') })
  pass('all three layouts, two densities and existing theme variables remain functional')
  await page.getByRole('button', { name: '恢复默认界面' }).click()
  await page.waitForFunction(() => document.querySelector('.app-shell').classList.contains('workbench-layout-left'))
  await page.setViewportSize({ width: 1180, height: 780 }); geometry.push({ defaultWindow: true, ...await layoutCheck() })
  await page.getByRole('button', { name: '收起二级导航', exact: true }).first().click()
  await page.locator('.secondary-panel').waitFor({ state: 'hidden' })
  assert((await layoutCheck()).main.width > 1000)
  await page.reload(); await page.locator('.secondary-panel').waitFor({ state: 'hidden' })
  await page.getByRole('button', { name: '展开二级导航', exact: true }).first().click()
  pass('collapse expands content area and persists across reload')

  for (const layout of ['left', 'right', 'top']) {
    await rpc('extension.preferences', { settings: { layout } }); await refresh()
    await page.waitForFunction(value => document.querySelector('.app-shell').classList.contains('workbench-layout-' + value), layout)
    await page.setViewportSize({ width: 640, height: 740 })
    await primaryClick('工作空间'); await go('程序'); await heading('程序')
    await page.locator('.secondary-panel').waitFor({ state: 'hidden' })
    geometry.push({ narrow: layout, ...await layoutCheck() })
    await primaryClick('智能体'); await go('接入说明'); await heading('接入说明')
    await page.screenshot({ path: path.join(output, `narrow-${layout}.png`) })
    await page.setViewportSize({ width: 1440, height: 940 })
    await page.getByRole('button', { name: '展开二级导航', exact: true }).first().click()
  }
  pass('narrow windows use contextual flyout without shrinking content')

  await rpc('extension.preferences', { settings: {} }); await refresh()
  await primaryClick('扩展中心'); await go('扩展应用')
  const pagePackage = (await rpc('extension.list')).find(item => item.package.id === 'test.navigation00')
  await rpc('extension.remove', { id: pagePackage.package.id, expected_digest: pagePackage.digest })
  await refresh()
  await page.waitForFunction(() => JSON.parse(localStorage.getItem('th.navigation.v1')).pins.length === 1)
  pass('uninstalled extension pages do not leave stale workspace pins')
  for (const item of (await rpc('extension.list')).filter(item => item.enabled)) await rpc('extension.set_enabled', { id: item.package.id, enabled: false, expected_digest: item.digest })
  await refresh()
  await page.waitForFunction(() => document.querySelector('.navigation-status')?.textContent.includes('已启用扩展0'))
  await primaryClick('工作空间'); await page.getByRole('heading', { name: '工作空间由你定义', exact: true }).waitFor()
  await primaryClick('智能体'); await heading('接入说明')
  await primaryClick('设置'); await heading('设置')
  await primaryClick('扩展中心'); await go('已安装'); await heading('已安装扩展')
  assert.equal(await primary().getByRole('button').count(), 4)
  pass('all optional modules disabled: workspace, agents, settings and recovery remain available')
  // Poisoned presentation data cannot create new groups or arbitrary routes.
  await page.evaluate(() => localStorage.setItem('th.navigation.v1', '{broken'))
  await page.reload(); await heading('已安装扩展')
  assert.equal(await primary().getByRole('button').count(), 4)
  pass('malformed stored navigation preferences recover to defaults')
  assert.deepEqual(errors, [])
  fs.writeFileSync(path.join(output, 'geometry.json'), JSON.stringify(geometry, null, 2))
  pass('no JavaScript runtime exceptions')
} catch (error) {
  results.push({ name: 'acceptance', ok: false, error: String(error), stack: error.stack })
  await page.screenshot({ path: path.join(output, 'failure.png') }).catch(() => {})
  throw error
} finally {
  fs.writeFileSync(path.join(output, 'results.json'), JSON.stringify({ platform: 'Windows native Tauri WebView2; isolated registry', results, errors }, null, 2))
  await browser.close()
}
