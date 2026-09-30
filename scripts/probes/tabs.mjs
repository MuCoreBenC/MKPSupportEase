/*
 * 页签骨架探针（P1 的验收）：浏览器的 8 个页签能不能点开、有没有把控制台点红。
 *
 * 用法（先起 dev 预览，再跑它）：
 *   npm run dev            # 固定 5321，strictPort
 *   node scripts/probes/tabs.mjs [url] [--shot]
 *
 * 为什么用 `playwright-core` 而不是 `playwright`：前者不下载浏览器，直接借本机的
 * Microsoft Edge（`channel: 'msedge'`）—— 试验场的探针也是这么跑的，
 * 而多下三百兆浏览器对一个只做验收的依赖太重。
 *
 * 探针只做"看得见"的判断：页签点开之后正文里有没有字、标题对不对、控制台有没有 error。
 * 具体到某一页的量（行高 / 溢出 / 位置）由各阶段的专用探针去量，别都堆在这一个里。
 */
import { chromium } from 'playwright-core'

const url = process.argv[2]?.startsWith('http') ? process.argv[2] : 'http://localhost:5321/'
const wantShot = process.argv.includes('--shot')

const TABS = ['首页', '预设', '校准', '参数', '同步', 'BBS 预设', '报告', '设置']

const browser = await chromium.launch({ channel: 'msedge' })
const page = await browser.newPage({ viewport: { width: 1760, height: 900 } })

const problems = []
/*
 * 已知且无害的一条：`/favicon.ico`。
 * `index.html` 没写 favicon，浏览器自己会去要一次，拿到 404 —— 产品仓从 v023 起就是如此，
 * 与这一轮的改动无关。记在这儿是为了让"0 条问题"这句话仍然算数（不是把它藏掉：
 * 想把图标补上就补 `public/favicon.ico` 并在 index.html 里声明，那时把这条删掉）。
 */
const BENIGN = [/\/favicon\.ico$/]
const benign = (text) => BENIGN.some((re) => re.test(text))

page.on('console', (m) => {
  if (m.type() !== 'error') return
  const at = m.location?.()?.url ?? ''
  if (benign(at)) return
  problems.push(`console.error: ${m.text()}${at ? ` @ ${at}` : ''}`)
})
page.on('pageerror', (e) => problems.push(`pageerror: ${e.message}`))
/* 404 这类只从控制台看不出来是谁 —— 把 URL 一起记下来，省得下次再猜 */
page.on('response', (r) => {
  if (r.status() >= 400 && !benign(r.url())) problems.push(`HTTP ${r.status()} ${r.url()}`)
})
page.on('requestfailed', (r) => {
  if (!benign(r.url())) problems.push(`requestfailed ${r.failure()?.errorText ?? ''} ${r.url()}`)
})

await page.goto(url, { waitUntil: 'load' })
await page.waitForSelector('header', { timeout: 10000 })

console.log(`url ${url}`)
console.log(`${'页签'.padEnd(10)} ${'正文首行'.padEnd(52)} 判定`)
console.log('-'.repeat(80))

for (const label of TABS) {
  const btn = page.getByRole('button', { name: label, exact: true }).first()
  await btn.click()
  await page.waitForTimeout(120)

  const info = await page.evaluate(() => {
    const main = document.querySelector('main')
    const active = document.querySelector('header nav [aria-current="page"]')
    return {
      text: (main?.innerText ?? '').replace(/\s+/g, ' ').trim(),
      activeLabel: active?.textContent?.trim() ?? active?.getAttribute('aria-label') ?? '',
    }
  })

  const ok = info.activeLabel === label && info.text.length > 0
  if (!ok) problems.push(`${label}: active=${info.activeLabel} 正文=${info.text.slice(0, 30)}`)
  console.log(`${label.padEnd(10)} ${info.text.slice(0, 50).padEnd(52)} ${ok ? 'ok' : 'FAIL'}`)
}

console.log('-'.repeat(80))
if (wantShot) {
  const tag = process.env.PROBE_TAG ?? 'tabs'
  const out = `tmp-port/shot-${tag}.png`
  await page.screenshot({ path: out })
  console.log(`截图 ${out}`)
}

await browser.close()

if (problems.length > 0) {
  console.log(`发现问题 ${problems.length} 条：`)
  for (const p of problems) console.log(`  - ${p}`)
  process.exit(1)
}
console.log('全部页签打开正常，控制台没有 error')
