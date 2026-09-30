#!/usr/bin/env node
/**
 * Capture nine-page UI operation evidence against scripts/ui-evidence/server.mjs.
 * Writes PNG screenshots + a JSON operation log under docs/reviews/evidence/nine-page/.
 */
import { chromium } from 'playwright';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(__dirname, '../..');
const outDir = path.join(repo, 'docs/reviews/evidence/nine-page');
const base = process.env.UI_EVIDENCE_URL || 'http://127.0.0.1:18765';

const PAGES = [
  'Overview',
  'Tools',
  'Environments',
  'Capabilities',
  'Skills',
  'Agents',
  'Activity',
  'Security',
  'Settings',
];

fs.mkdirSync(outDir, { recursive: true });

const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ viewport: { width: 1280, height: 800 } });
const log = [];

async function openPage() {
  const page = await context.newPage();
  page.setDefaultTimeout(20000);
  page.on('pageerror', (err) => log.push({ kind: 'pageerror', message: String(err) }));
  page.on('console', (msg) => {
    if (msg.type() === 'error') log.push({ kind: 'console', message: msg.text() });
  });
  await page.goto(base, { waitUntil: 'domcontentloaded', timeout: 30000 });
  await page.locator('header').waitFor({ state: 'visible' });
  return page;
}

async function clickNav(page, name) {
  await page.getByRole('button', { name, exact: true }).click({ timeout: 15000 });
  await page.locator('header').waitFor({ state: 'visible' });
  await page.waitForTimeout(400);
  await page
    .getByText('加载中…')
    .waitFor({ state: 'hidden', timeout: 30000 })
    .catch(() => {});
}

// Overview + quick scan against live daemon.
{
  const page = await openPage();
  await clickNav(page, 'Overview');
  const overviewBtn = page.getByRole('button', { name: '快速扫描' });
  if (await overviewBtn.count()) {
    await overviewBtn.click();
    log.push({ kind: 'action', page: 'Overview', action: 'quick-scan-click' });
    const deadline = Date.now() + 120000;
    let settled = false;
    while (Date.now() < deadline) {
      try {
        const res = await fetch(`${base}/api/invoke`, {
          method: 'POST',
          headers: { 'content-type': 'application/json' },
          body: JSON.stringify({ method: 'status', params: {} }),
        });
        const status = await res.json();
        if ((status.tool_count ?? 0) > 0 || (status.candidate_count ?? 0) > 0) {
          settled = true;
          log.push({ kind: 'action', page: 'Overview', action: 'quick-scan-settled', status });
          break;
        }
      } catch {
        /* retry */
      }
      await new Promise((r) => setTimeout(r, 2000));
    }
    if (!settled) log.push({ kind: 'action', page: 'Overview', action: 'quick-scan-timeout' });
  }
  await page.reload({ waitUntil: 'domcontentloaded' });
  await clickNav(page, 'Overview');
  await page.getByText('工具：').waitFor({ timeout: 20000 }).catch(() => {});
  await page.waitForTimeout(500);
  const file = path.join(outDir, '01-Overview.png');
  await page.screenshot({ path: file });
  const bodyText = await page.locator('main').innerText();
  log.push({
    kind: 'page',
    page: 'Overview',
    screenshot: path.relative(repo, file).replaceAll('\\', '/'),
    mainChars: bodyText.length,
    hasError: bodyText.includes('错误'),
    snippet: bodyText.slice(0, 240),
  });
  console.log('captured Overview');
  await page.close();
}

for (let i = 1; i < PAGES.length; i++) {
  const name = PAGES[i];
  const page = await openPage();
  // Keep Tools query narrow so the table stays readable in evidence shots.
  if (name === 'Tools') {
    const input = page.locator('input');
    if (await input.count()) {
      await input.fill('python');
    }
  }
  await clickNav(page, name);
  // Extra settle for data-heavy pages.
  await page.waitForTimeout(name === 'Tools' ? 1500 : 600);
  const file = path.join(outDir, `${String(i + 1).padStart(2, '0')}-${name}.png`);
  await page.screenshot({ path: file });
  const bodyText = await page.locator('main').innerText().catch(() => '');
  log.push({
    kind: 'page',
    page: name,
    screenshot: path.relative(repo, file).replaceAll('\\', '/'),
    mainChars: bodyText.length,
    hasError: bodyText.includes('错误') || /"error"/i.test(bodyText),
    snippet: bodyText.slice(0, 240),
  });
  console.log('captured', name, '->', file);
  await page.close();
}

fs.writeFileSync(
  path.join(outDir, 'nine-page-log.json'),
  JSON.stringify({ capturedAt: new Date().toISOString(), base, log }, null, 2),
);

await browser.close();
console.log('done');
