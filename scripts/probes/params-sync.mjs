/*
 * 接线探针：参数页（P3）与同步页（P4）两页 + 互跳 —— 「一次性接线」那一步的验收。
 *
 * 用法（先 `npm run build`，再让静态预览在 4173 上跑着）：
 *   npm run build
 *   npx vite preview --port 4173 --strictPort
 *   node scripts/probes/params-sync.mjs [url]
 *
 * 走一遍真实的路（每档尺寸一个全新 profile，localStorage 是空的 = 还没同步过）：
 *   ① 参数页：一份说明书都没有 → 诚实空态（不编假数据）
 *   ② 空态那颗「去「同步」页获取一份」→ 跳到同步页（互跳第一程）
 *   ③ 同步页自动同步完 → 「去看参数页」点回来（互跳第二程）
 *   ④ 参数页这回有东西画了：分类 / 卡片 / 行全部来自刚同步的那份说明书
 *
 * 判断只做「看得见」的那几条：跳没跳过去、页面上有没有内容、控制台有没有 error、
 * 有没有 >=400 的响应。两档尺寸：Ultra 1760×900 / Compact 900×640。
 */
import { mkdir } from 'node:fs/promises'

import { chromium } from 'playwright-core'

const url = process.argv[2]?.startsWith('http') ? process.argv[2] : 'http://localhost:4173/'

const SIZES = [
  { tag: 'ultra', width: 1760, height: 900 },
  { tag: 'compact', width: 900, height: 640 },
]

const shotDir = 'tmp-shots'
await mkdir(shotDir, { recursive: true })

const problems = []
const results = []
const check = (tag, what, ok, detail = '') => {
  results.push({ tag, what, ok, detail })
  if (!ok) problems.push(`${tag} ${what}${detail ? ` —— ${detail}` : ''}`)
}

/* 已知且无害：index.html 没写 favicon，浏览器自己会去要一次 */
const BENIGN = [/\/favicon\.ico$/]
const benign = (text) => BENIGN.some((re) => re.test(text))

const mainText = (page) =>
  page.evaluate(() => (document.querySelector('main')?.innerText ?? '').replace(/\s+/g, ' ').trim())

const activeTab = (page) =>
  page.evaluate(() => document.querySelector('header nav [aria-current="page"]')?.textContent?.trim() ?? '')

const browser = await chromium.launch({ channel: 'msedge' })

for (const size of SIZES) {
  const tag = size.tag
  const ctx = await browser.newContext({ viewport: { width: size.width, height: size.height } })
  const page = await ctx.newPage()

  page.on('console', (m) => {
    if (m.type() !== 'error') return
    const at = m.location?.()?.url ?? ''
    if (benign(at)) return
    problems.push(`${tag} console.error: ${m.text()}${at ? ` @ ${at}` : ''}`)
  })
  page.on('pageerror', (e) => problems.push(`${tag} pageerror: ${e.message}`))
  page.on('response', (r) => {
    if (r.status() >= 400 && !benign(r.url())) problems.push(`${tag} HTTP ${r.status()} ${r.url()}`)
  })
  page.on('requestfailed', (r) => {
    if (!benign(r.url())) problems.push(`${tag} requestfailed ${r.failure()?.errorText ?? ''} ${r.url()}`)
  })

  await page.goto(url, { waitUntil: 'load' })
  await page.waitForSelector('header nav', { timeout: 10000 })

  /* ① 参数页：没有说明书的时候只说去哪儿拿，不编一份假的 */
  await page.getByRole('button', { name: '参数', exact: true }).first().click()
  await page.waitForSelector('text=还没有数据包', { timeout: 10000 })
  const empty = await mainText(page)
  const jumpCount = await page.getByRole('button', { name: '去「同步」页获取一份' }).count()
  check(tag, '参数页没包时是诚实空态', !empty.includes('修改参数'), empty.slice(0, 60))
  check(tag, '空态里有去同步页的那条路', jumpCount === 1)
  await page.screenshot({ path: `${shotDir}/wire-params-empty-${tag}.png` })

  /* ② 互跳第一程：空态那颗按钮 → 同步页（「去看参数页」出现 = 自动同步完） */
  await page.getByRole('button', { name: '去「同步」页获取一份' }).click()
  await page.waitForSelector('text=去看参数页', { timeout: 15000 })
  const syncText = await mainText(page)
  check(tag, '互跳：参数页空态 → 同步页', (await activeTab(page)) === '同步')
  check(tag, '同步页自动同步出说明书', syncText.includes('已同步'), syncText.slice(0, 80))
  check(tag, '同步页摊出说明书里的布局与模式开关', syncText.includes('这份说明书里有什么') && syncText.includes('模式开关'), '')
  await page.screenshot({ path: `${shotDir}/wire-sync-${tag}.png` })

  /* ③ 互跳第二程：去看参数页 → 参数页这次有东西画了 */
  await page.getByRole('button', { name: '去看参数页' }).click()
  await page.waitForSelector('[role="tablist"][aria-label="参数分类"]', { timeout: 10000 })
  await page.waitForTimeout(200)
  const full = await page.evaluate(() => {
    const tabs = [...document.querySelectorAll('[role="tablist"][aria-label="参数分类"] [role="tab"]')]
    return {
      tabs: tabs.length,
      current: tabs.find((t) => t.getAttribute('aria-current') === 'true')?.textContent?.trim() ?? '',
      cards: document.querySelectorAll('main section[aria-label]').length,
      rows: document.querySelectorAll('[data-key]').length,
      text: (document.querySelector('main')?.innerText ?? '').replace(/\s+/g, ' ').trim(),
    }
  })
  check(tag, '互跳：同步页 → 参数页', (await activeTab(page)) === '参数')
  check(
    tag,
    '参数页画的是真说明书（分类 / 卡片 / 行都来了）',
    full.tabs >= 2 && full.cards > 0 && full.rows > 0,
    `分类 ${full.tabs} / 卡片 ${full.cards} / 行 ${full.rows}`,
  )
  check(
    tag,
    '参数页标题与底栏在（每一条都从包里来）',
    full.text.includes('修改参数') && full.text.includes('没有未保存的改动'),
    full.text.slice(0, 60),
  )
  await page.screenshot({ path: `${shotDir}/wire-params-full-${tag}.png` })

  console.log(
    `${tag.padEnd(8)} 分类 ${String(full.tabs).padEnd(2)} 卡片 ${String(full.cards).padEnd(2)} 行 ${String(full.rows).padEnd(3)} 当前分类 ${full.current}`,
  )
  await ctx.close()
}

await browser.close()

console.log('-'.repeat(80))
console.log(`${results.length} 条判定：`)
for (const r of results) {
  console.log(`  ${r.ok ? 'ok  ' : 'FAIL'} [${r.tag}] ${r.what}${r.detail ? `  （${r.detail}）` : ''}`)
}

if (problems.length > 0) {
  console.log(`发现问题 ${problems.length} 条：`)
  for (const p of problems) console.log(`  - ${p}`)
  process.exit(1)
}
console.log('接线两程都通：两档尺寸 0 console error / 0 个 >=400')
