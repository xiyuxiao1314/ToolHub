import { chromium } from 'playwright'
import fs from 'node:fs'
import path from 'node:path'

const out = 'D:/mimoproject/ToolHub/ui-redesign-shots'
fs.mkdirSync(out, { recursive: true })
const browser = await chromium.launch({ headless: true })
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } })
await page.goto('http://127.0.0.1:18765', { waitUntil: 'networkidle' })
await page.waitForTimeout(1500)

const pages = ['工具', '智能体', '环境', '市场', '任务', '设置']
for (const name of pages) {
  await page.getByRole('button', { name, exact: true }).first().click()
  await page.waitForTimeout(800)
  const file = path.join(out, `${name}.png`)
  await page.screenshot({ path: file })
  console.log('shot', name)
}

// tools detail
await page.getByRole('button', { name: '工具', exact: true }).first().click()
await page.waitForTimeout(500)
const items = page.locator('.tool-item')
if (await items.count()) {
  await items.first().click()
  await page.waitForTimeout(400)
  await page.screenshot({ path: path.join(out, '工具-详情.png') })
  console.log('shot detail')
}
await browser.close()
console.log('done')
