/*
 * 接线探针：参数页（P3）与同步页（P4）两页 —— 「一次性接线」那一步的验收。
 *
 * 用法（先 `npm run build`，再让静态预览在 4173 上跑着）：
 *   npm run build
 *   npx vite preview --port 4173 --strictPort
 *   node scripts/probes/params-sync.mjs [url]
 *
 * 走一遍真实的路（每档尺寸一个全新 profile）：
 *   ① 参数页：分类 / 卡片 / 行全部来自那份说明书（catalog 的 registry）—— 直接画得出来
 *   ② 底栏在：从包里来的那两句（「修改参数」/「没有未保存的改动」）
 *   ③ 同步页：如实说说明书随安装包走 + 数据源地址那格（浏览器里"还没配置"）+
 *      catalog 的账（schema / 指纹 / 机型与版本数）+「检查更新」在（显式动作，不进首屏）
 *
 * **2026-10-02 重写**：原版走的是 C4 之前的老流程 ——「参数页空态（"还没有数据包"）→
 * 点「去「同步」页获取一份」→ 同步页自动同步出说明书 → 点「去看参数页」跳回来」。
 * 那条流已经不在了：参数页现在**直接读 catalog**（随安装包走，"没有包"不再是正常态），
 * 同步页也不再有"自动同步 / 去看参数页"那对互跳。原版那几条断言读的正是退役结构，
 * 恒 FAIL —— 与 `chain.mjs` **同一场病**（C4 收口时一起烂的，登记见 HANDOFF §4 清扫批次）。
 *
 * 换的是什么：互跳那两程换成了两页各自的现状（参数页照 catalog 画；同步页如实说口径与账）。
 * 没换的是什么：还是只做「看得见」的那几条 —— 页面有没有内容、控制台有没有 error、
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

/** 等一个条件成立（后端是内存桩，一般是一两帧的事） */
async function until(fn, ms = 5000) {
  const t0 = Date.now()
  for (;;) {
    if (await fn()) return true
    if (Date.now() - t0 > ms) return false
    await new Promise((r) => setTimeout(r, 100))
  }
}

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

  /* ① 参数页：直接画 catalog 的 registry（分类 / 卡片 / 行） */
  await page.getByRole('button', { name: '参数', exact: true }).first().click()
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
  await page.screenshot({ path: `${shotDir}/wire-params-${tag}.png` })

  /* ② 同步页：口径（catalog 随包走）+ 数据源那格 + catalog 的账 + 检查更新 */
  await page.getByRole('button', { name: '同步', exact: true }).first().click()
  await until(async () => (await mainText(page)).includes('随安装包走'), 8000)
  const syncText = await mainText(page)
  check(
    tag,
    '同步页如实说「说明书随安装包走」（不再有"自动同步一份"那道老流程）',
    syncText.includes('随安装包走') && syncText.includes('catalog'),
    syncText.slice(0, 80),
  )
  check(
    tag,
    '数据源那格在，且浏览器里如实说"还没配置"',
    syncText.includes('数据源地址') && syncText.includes('还没配置'),
    '',
  )
  check(
    tag,
    'catalog 的账在（schema / 指纹 / 机型与版本数）',
    /schema \d/.test(syncText) && syncText.includes('指纹') && /\d+ 台/.test(syncText),
    syncText.slice(0, 60),
  )
  const checkBtn = await page.getByRole('button', { name: '检查更新', exact: true }).count()
  check(tag, '「检查更新」在（显式动作，不进首屏）', checkBtn === 1, `找到 ${checkBtn} 个`)
  await page.screenshot({ path: `${shotDir}/wire-sync-${tag}.png` })

  console.log(
    `${tag.padEnd(8)} 参数页 分类 ${String(full.tabs).padEnd(2)} 卡片 ${String(full.cards).padEnd(2)} 行 ${String(full.rows).padEnd(3)} 当前分类 ${full.current} · 同步页 口径与账都在`,
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
console.log('参数页与同步页两边都通：两档尺寸 0 console error / 0 个 >=400')
