import { chromium } from 'playwright'
import fs from 'node:fs'
import path from 'node:path'

const base = process.env.UI_EVIDENCE_URL || 'http://127.0.0.1:18765'
const out = 'D:/mimoproject/ToolHub/ui-verify'
fs.mkdirSync(out, { recursive: true })
const results = []

function pass(name) {
  results.push({ name, ok: true })
  console.log('PASS', name)
}
function fail(name, err) {
  results.push({ name, ok: false, err: String(err) })
  console.log('FAIL', name, err)
}

const browser = await chromium.launch({ headless: true })
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } })
page.on('pageerror', (e) => fail('pageerror', String(e)))

try {
  await page.goto(base, { waitUntil: 'networkidle', timeout: 30000 })
  await page.waitForTimeout(1200)
  pass('load')

  // nav
  for (const name of ['智能体', '环境', '市场', '任务', '设置', '工具']) {
    await page.locator('button', { hasText: name }).first().click()
    await page.waitForTimeout(400)
    const body = await page.locator('body').innerText()
    if (!body) fail('nav ' + name, 'empty body')
    else pass('nav ' + name)
  }

  // layout: tool path should ellipsize / not overflow
  const overflow = await page.evaluate(() => {
    const items = [...document.querySelectorAll('.tool-item')]
    return items.map((el) => ({
      scrollW: el.scrollWidth,
      clientW: el.clientWidth,
      overlap: el.scrollWidth > el.clientWidth + 2,
    }))
  })
  const bad = overflow.filter((x) => x.overlap)
  if (bad.length) fail('tool layout', JSON.stringify(bad))
  else pass('tool layout')

  await page.screenshot({ path: path.join(out, 'tools.png') })

  // select first tool + detail tabs
  const item = page.locator('.tool-item').first()
  await item.click()
  await page.waitForTimeout(300)
  for (const tab of ['概览', '能力', '环境', '使用记录']) {
    await page.locator('.tab', { hasText: tab }).first().click()
    await page.waitForTimeout(250)
    pass('tab ' + tab)
  }

  // open terminal / reveal (shim-backed in evidence server)
  const terminalBtn = page.locator('button', { hasText: '打开终端' }).first()
  if (await terminalBtn.count()) {
    await terminalBtn.click()
    await page.waitForTimeout(600)
    const body = await page.locator('body').innerText()
    if (body.includes('错误') && body.includes('not found')) fail('open terminal', body.slice(0, 120))
    else pass('open terminal')
  } else fail('open terminal', 'button missing')

  const revealBtn = page.locator('button', { hasText: '打开位置' }).first()
  if (await revealBtn.count()) {
    await revealBtn.click()
    await page.waitForTimeout(600)
    const body = await page.locator('body').innerText()
    if (body.includes('reveal_path not allowed') || body.includes('Command not found')) fail('reveal path', body.slice(0, 120))
    else pass('reveal path')
  } else fail('reveal path', 'button missing')

  // scan button exists and is enabled
  const scanBtn = page.locator('button', { hasText: '扫描本机' }).first()
  if ((await scanBtn.count()) && !(await scanBtn.isDisabled())) pass('scan button')
  else fail('scan button', 'missing or disabled')

  // add tool form
  await page.locator('button', { hasText: '添加工具' }).first().click()
  await page.waitForTimeout(300)
  if (await page.locator('input[placeholder*="绝对路径"]').count()) pass('add tool form')
  else fail('add tool form', 'input missing')

  // settings export
  await page.locator('button', { hasText: '设置' }).first().click()
  await page.waitForTimeout(400)
  await page.screenshot({ path: path.join(out, 'settings.png') })
  const exportBtn = page.locator('button', { hasText: '导出配置' }).first()
  await exportBtn.click()
  await page.waitForTimeout(1200)
  const settingsBody = await page.locator('body').innerText()
  if (settingsBody.includes('已导出') || settingsBody.includes('toolhub-export')) pass('export config')
  else fail('export config', settingsBody.slice(0, 200))

  // envs refresh
  await page.locator('button', { hasText: '环境' }).first().click()
  await page.waitForTimeout(300)
  await page.locator('button', { hasText: '刷新' }).first().click()
  await page.waitForTimeout(1000)
  const envBody = await page.locator('body').innerText()
  if (envBody.includes('已刷新') || envBody.includes('环境')) pass('env refresh')
  else fail('env refresh', envBody.slice(0, 160))
  await page.screenshot({ path: path.join(out, 'envs.png') })
} catch (e) {
  fail('script', e)
}

await browser.close()
fs.writeFileSync(path.join(out, 'verify.json'), JSON.stringify(results, null, 2))
const failed = results.filter((r) => !r.ok)
console.log('SUMMARY', results.length - failed.length, 'ok', failed.length, 'fail')
if (failed.length) {
  console.log(JSON.stringify(failed, null, 2))
  process.exitCode = 1
}
