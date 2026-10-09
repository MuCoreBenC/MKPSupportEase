/*
 * 现状地图的离线交互验证（用仓里已有的 playwright-core + 本机 Edge，不下载浏览器）。
 *
 *   node docs/audit-current-state/map/verify.mjs
 *
 * 检查项（对应任务里的交付物验证要求）：
 *   1. file:// 直接打开，无网络请求、无控制台错误、无页面异常
 *   2. 统计数字渲染出来了（来自内联数据，不是写死的）
 *   3. 导航、全局搜索、证据等级筛选、裁断筛选、清除筛选都真的可用
 *   4. 点证据索引里的记录按钮能跳到对应记录
 *   5. 页面里没有必须联网才能工作的依赖（外链/外图/外字体）
 */

import { chromium } from 'playwright-core'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const mapDir = path.dirname(fileURLToPath(import.meta.url))
const htmlPath = path.resolve(mapDir, '..', 'index.html')
const url = 'file:///' + htmlPath.replace(/\\/g, '/')

const fails = []
const notes = []
const ok = (cond, msg) => { if (!cond) fails.push(msg); else notes.push('OK  ' + msg) }

const browser = await chromium.launch({ channel: 'msedge' })
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } })

const consoleErrors = []
const pageErrors = []
const requests = []
page.on('console', (m) => { if (m.type() === 'error') consoleErrors.push(m.text()) })
page.on('pageerror', (e) => pageErrors.push(String(e)))
page.on('request', (r) => requests.push(r.url()))

const resp = await page.goto(url)
ok(resp !== null, 'index.html 能打开（file://）')

const total = await page.locator('.rec').count()
ok(total > 100, `记录渲染数 > 100（实际 ${total}）`)

const statCount = await page.locator('#stats .stat').count()
ok(statCount === 8, `顶部统计渲染 ${statCount} 项`)

const navCount = await page.locator('.navBtn').count()
ok(navCount >= 8, `导航分节 ${navCount} 个`)

// 概览可见
const visibleSection = await page.locator('.section:visible').count()
ok(visibleSection === 1, `同一时刻只显示一个分节（实际 ${visibleSection}）`)

// 全局搜索
await page.fill('#q', 'fetchOfficialPreset')
await page.waitForTimeout(120)
const searchCount = await page.locator('#count').innerText()
ok(/^\d+ \/ \d+ 条命中$/.test(searchCount.trim()), `搜到结果：${searchCount.trim()}`)
const hit = Number(searchCount.split('/')[0].trim())
ok(hit > 0 && hit < total, `搜索确实过滤了（命中 ${hit} / 共 ${total}）`)

// 清除
await page.click('#clear')
await page.waitForTimeout(80)
const afterClear = await page.locator('#count').innerText()
ok(afterClear.includes(String(total)), `清除筛选后回到全量：${afterClear.trim()}`)

const nav = (name) => page.locator('.navBtn').filter({ hasText: name }).first().click()

// 证据等级筛选：跳到「问题与裁断」分节后筛 conflict
await nav('问题与裁断')
await page.waitForTimeout(120)
await page.selectOption('#lv', 'conflict')
await page.waitForTimeout(150)
const levels = await page.$$eval('.rec:not([style*="display: none"])', (els) => els.map((e) => e.dataset.level))
ok(levels.length > 0 && levels.every((l) => l === 'conflict'), `按「已发现冲突」筛选后可见项等级一致（${levels.length} 条）`)

// 裁断筛选
await page.selectOption('#lv', '')
await page.selectOption('#vd', 'drop')
await page.waitForTimeout(150)
const verdicts = await page.$$eval('.rec:not([style*="display: none"])', (els) => els.map((e) => e.dataset.verdict))
ok(verdicts.length > 0 && verdicts.every((v) => v === 'drop'), `按裁断「砍掉」筛选后可见项一致（${verdicts.length} 条）`)
await page.click('#clear')

// 展开/折叠（只看当前可见的那一节里的第一个分组）
await nav('领域与状态')
await page.waitForTimeout(120)
const grp = page.locator('.section:visible details.grp').first()
const wasOpen = await grp.evaluate((d) => d.open)
await grp.locator('summary').click()
await page.waitForTimeout(80)
const nowOpen = await grp.evaluate((d) => d.open)
ok(wasOpen !== nowOpen, '分组可折叠/展开')

// 实体记录存在（点击一个实体看到字段/状态/命令）
const entity = page.locator('[data-id="E-06"]').first()
ok((await entity.count()) > 0, '实体 E-06 在页面里存在')

// 回哈希跳转 + 高亮（规范形态：分节/记录号）
await page.evaluate(() => (window.location.hash = '#contract/C-39'))
await page.waitForTimeout(250)
const cmdVisible = await page.locator('.section[data-section="contract"]:visible').count()
ok(cmdVisible === 1, '哈希 #contract/C-39 切到契约分节')
const flashed = await page.locator('[data-id="C-39"].flash').count()
ok(flashed > 0, '目标记录被定位并高亮')

// 容错形态：kind:记录号
await page.evaluate(() => (window.location.hash = '#entity:E-06'))
await page.waitForTimeout(250)
const domVisible = await page.locator('.section[data-section="domain"]:visible').count()
ok(domVisible === 1, '容错哈希 #entity:E-06 反查到领域分节')

// 证据索引按钮可点，并跳到「那一节的哪一条」
await page.evaluate(() => (window.location.hash = '#evidence'))
await page.waitForTimeout(200)
const evRows = await page.locator('.section[data-section="evidence"] table.kv tr').count()
ok(evRows > 100, `证据索引列出 ${evRows} 条引用`)
const firstBtnText = await page.locator('.section[data-section="evidence"] .linkBtn').first().innerText()
await page.locator('.section[data-section="evidence"] .linkBtn').first().click()
await page.waitForTimeout(400)
const sectionAfterJump = await page.locator('.section:visible').getAttribute('data-section')
ok(sectionAfterJump !== 'evidence', `点证据索引里的引用（${firstBtnText}）跳到 ${sectionAfterJump} 分节`)
const jumped = firstBtnText.split('/')[1]
const landed = await page.locator(`[data-id="${jumped}"].flash`).count()
ok(landed > 0, `跳转后目标记录 ${jumped} 被定位并高亮`)

// 无外部依赖
const externals = requests.filter((u) => !u.startsWith('file://') && !u.startsWith('data:'))
ok(externals.length === 0, `没有任何非 file:// 请求（实际 ${externals.length}：${externals.slice(0, 3).join(', ')}）`)
ok(consoleErrors.length === 0, `无控制台错误（实际 ${consoleErrors.length}：${consoleErrors.slice(0, 2).join(' | ')}）`)
ok(pageErrors.length === 0, `无页面异常（实际 ${pageErrors.length}：${pageErrors.slice(0, 2).join(' | ')}）`)

await browser.close()

for (const n of notes) console.log(n)
if (fails.length) {
  console.error('\n[verify] 失败 ' + fails.length + ' 条：')
  for (const f of fails) console.error('  - ' + f)
  process.exit(1)
}
console.log('\n[verify] 全部通过：' + notes.length + ' 项（file:// 离线打开 + 交互）')
