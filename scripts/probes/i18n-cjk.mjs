/*
 * 探针：英文界面里还剩哪些中文（DOM 取证，截图看漏的那层兜底）。
 *
 * 用法（先 `npm run build`，静态预览在 4173 上跑着）：
 *   node scripts/probes/i18n-cjk.mjs [url]
 *
 * 干的事：把语言切成 English，逐页扫 `document.body.innerText` 里**含中文的行**，
 * 连同上两三层祖先的 class 打出来。它不是断言（数据名允许有中文），是**清单**：
 * 眼睛盯一遍，剩下的要么是数据，要么是真漏。
 *
 * 判定：0 console error / 0 个 >= 400。
 */
import { chromium } from 'playwright-core'

const url = process.argv.find((a) => a.startsWith('http')) ?? 'http://localhost:4173/'

const problems = []
const BENIGN_URL = [/\/favicon\.ico$/]
const benign = (u) => BENIGN_URL.some((re) => re.test(u))

const browser = await chromium.launch({ channel: 'msedge' })
const ctx = await browser.newContext({ viewport: { width: 1180, height: 820 }, locale: 'zh-CN' })
const page = await ctx.newPage()

page.on('console', (m) => {
  if (m.type() !== 'error') return
  const at = m.location?.()?.url ?? ''
  if (benign(at)) return
  problems.push(`console.error: ${m.text()}`)
})
page.on('pageerror', (e) => problems.push(`pageerror: ${e.message}`))
page.on('response', (r) => {
  if (r.status() >= 400 && !benign(r.url())) problems.push(`HTTP ${r.status()} ${r.url()}`)
})

await page.goto(url, { waitUntil: 'load' })
await page.waitForSelector('header nav', { timeout: 10000 })

/* 语言：设置页第三档 = English（0 跟随系统 / 1 简体中文 / 2 English） */
await page.getByRole('button', { name: /^(设置|Settings)$/ }).first().click()
await page.waitForSelector('input[name="locale"]', { timeout: 10000 })
await page.locator('label:has(input[name="locale"])').nth(2).click()
await page.waitForTimeout(500)

/** 一页：扫出所有含中文的行 + 它们所在容器的 class */
const scan = async () =>
  page.evaluate(() => {
    const CJK = /[\u3400-\u9fff\uff00-\uffef]/
    const out = []
    const walk = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT)
    for (let n = walk.nextNode(); n !== null; n = walk.nextNode()) {
      const text = (n.textContent ?? '').trim()
      if (text === '' || !CJK.test(text)) continue
      const el = n.parentElement
      if (el === null) continue
      const style = getComputedStyle(el)
      if (style.display === 'none' || style.visibility === 'hidden') continue
      const path = []
      let cur = el
      for (let i = 0; i < 3 && cur !== null; i++) {
        const cls = String(cur.className ?? '').split(' ').filter(Boolean)[0] ?? ''
        if (cls !== '') path.push(cls)
        cur = cur.parentElement
      }
      out.push({ text, at: path.join(' < ') })
    }
    return out
  })

const pages = [
  ['home', /^(首页|Home)$/],
  ['presets', /^(预设|Presets)$/],
  ['calib', /^(校准|Calibration)$/],
  ['bbs', /^(工艺|Process)$/],
  ['report', /^(报告|Report)$/],
  ['settings', /^(设置|Settings)$/],
]

let total = 0
for (const [name, re] of pages) {
  await page.getByRole('button', { name: re }).first().click()
  await page.waitForTimeout(700)
  const hits = await scan()
  total += hits.length
  console.log(`\n## ${name}  (${hits.length})`)
  for (const h of hits.slice(0, 40)) console.log(`  · ${h.text}   [${h.at}]`)
  if (hits.length > 40) console.log(`  … 还有 ${hits.length - 40} 行`)
}

await browser.close()

console.log(`\n合计 ${total} 行含中文（数据名允许；逐条看清单）`)
if (problems.length > 0) {
  console.log(`\n${problems.length} 个运行时问题：`)
  for (const p of problems) console.log('  -', p)
  process.exitCode = 1
}
