/*
 * 预设页探针：这一页到底长得对不对、点得动点不动。
 *
 * 用法（先 `npm run build`，再让静态预览在 4173 上跑着）：
 *   npm run build
 *   npx vite preview --port 4173 --strictPort
 *   node scripts/probes/presets.mjs [url]
 *
 * 为什么不用 dev（5321）：那台 watcher 会去扫 `target/` 下 5.8 万个文件，
 * 自己把自己拖死 —— 验收一律走「build 之后 preview」这条静态路。
 *
 * 判断只做「看得见」的那几条：点一下有没有反应、表里有没有行、右键菜单出不出来、
 * 控制台有没有 error、有没有 >=400 的响应。**不去量行高与像素**（那是专用探针的事）。
 */
import { mkdir } from 'node:fs/promises'

import { chromium } from 'playwright-core'

const url = process.argv[2]?.startsWith('http') ? process.argv[2] : 'http://localhost:4173/'

const browser = await chromium.launch({ channel: 'msedge' })
const page = await browser.newPage({ viewport: { width: 1760, height: 900 } })

const problems = []
/* 已知且无害：index.html 没写 favicon，浏览器自己会去要一次 */
const BENIGN = [/\/favicon\.ico$/]
const benign = (text) => BENIGN.some((re) => re.test(text))

page.on('console', (m) => {
  if (m.type() !== 'error') return
  const at = m.location?.()?.url ?? ''
  if (benign(at)) return
  problems.push(`console.error: ${m.text()}${at ? ` @ ${at}` : ''}`)
})
page.on('pageerror', (e) => problems.push(`pageerror: ${e.message}`))
page.on('response', (r) => {
  if (r.status() >= 400 && !benign(r.url())) problems.push(`HTTP ${r.status()} ${r.url()}`)
})
page.on('requestfailed', (r) => {
  if (!benign(r.url())) problems.push(`requestfailed ${r.failure()?.errorText ?? ''} ${r.url()}`)
})

await page.goto(url, { waitUntil: 'load' })
await page.waitForSelector('header', { timeout: 10000 })

/** 表格那一块的可读事实 */
const facts = () =>
  page.evaluate(() => {
    const table = document.querySelector('main table')
    const rows = [...document.querySelectorAll('main tbody tr')]
    /*
     * 展开排紧跟在数据行后面，它那一格是 `td[colspan]`、内容是一张 `<dl>`。
     * 「数据行」的判据因此是「带一个 td.name 的普通格」—— 这样展开排不会被算成一行
     * （第一版探针就是把它算进去了，compact 档量到 2 行，查了半天）。
     */
    const dataRows = rows.filter((r) => r.querySelector('td:not([colspan])') !== null)
    const heads = [...document.querySelectorAll('main thead th')].map((th) => th.textContent?.trim() ?? '')
    const main = document.querySelector('main')
    const axes = [...document.querySelectorAll('main [role="radiogroup"]')].map((g) =>
      [...g.querySelectorAll('label')]
        .filter((l) => l.getAttribute('data-on') === 'true')
        .map((l) => l.textContent?.trim() ?? '')
        .join(''),
    )
    return {
      text: (main?.innerText ?? '').replace(/\s+/g, ' ').trim(),
      heads,
      dataRows: dataRows.length,
      firstRow: dataRows[0]?.innerText.replace(/\s+/g, ' ').trim().slice(0, 110) ?? '',
      /* 每一行的第一格（文件名）+ 操作列那一格 —— 报告「这张表里到底是什么」用 */
      rows: dataRows.slice(0, 8).map((r) => {
        const tds = [...r.querySelectorAll('td')]
        const name = tds[0]?.innerText.replace(/\s+/g, ' ').trim().slice(0, 60) ?? ''
        const act = tds[tds.length - 1]?.innerText.replace(/\s+/g, ' ').trim() ?? ''
        return `${name} → ${act}`
      }),
      /* 工具条右端那两格：当前这张表的真计数 + 仓库台账 */
      counts: [...document.querySelectorAll('main span')]
        .map((s) => s.innerText.replace(/\s+/g, ' ').trim())
        .filter((t) => /^共 \d+ 项/.test(t) || /^仓库 \d+/.test(t))
        .join(' | '),
      axes,
      empty: document.querySelector('main table')?.parentElement?.parentElement?.innerText.includes('还没有')
        ? [...document.querySelectorAll('main p')].map((p) => p.innerText.trim()).find((t) => t.includes('还没有')) ?? ''
        : '',
      expanded: document.querySelectorAll('main tbody dl').length,
    }
  })

console.log(`url ${url}`)
const shotDir = 'tmp-shots'
await mkdir(shotDir, { recursive: true })

/* ---------- 1. 打开「预设」 ---------- */
await page.getByRole('button', { name: '预设', exact: true }).first().click()
await page.waitForTimeout(400)

let f = await facts()
console.log(`\n[预设页] 轴选中 ${JSON.stringify(f.axes)}`)
console.log(`表头 ${f.heads.join(' / ') || '(无)'}`)
console.log(`数据行 ${f.dataRows}  计数 ${f.counts}`)
console.log(`行内容 ${f.rows.join(' || ') || '(无)'}`)
console.log(`可见正文 ${f.text.slice(0, 260)}`)
if (f.dataRows === 0) problems.push('预设页打开后表格 0 行')
if (f.heads.length === 0) problems.push('预设页没有表头')
await page.screenshot({ path: `${shotDir}/presets-ultra.png` })

/* ---------- 2. 两条轴控件 ---------- */
const rad = (name, value) => page.locator(`main input[type="radio"][name="${name}"][value="${value}"]`)

/** 切一档、把这一档的可读事实打出来 */
const show = async (title, name, value) => {
  await rad(name, value).click({ force: true })
  await page.waitForTimeout(250)
  const v = await facts()
  console.log(`\n[${title}] 轴 ${JSON.stringify(v.axes)} 行 ${v.dataRows}  计数 ${v.counts}`)
  console.log(`表头 ${v.heads.join(' / ')}`)
  console.log(`行内容 ${v.rows.join(' || ') || '(无)'}`)
  if (v.empty !== '') console.log(`空态文案 ${v.empty}`)
  return v
}

await rad('preset-kind', 'mkp').click({ force: true })
await page.waitForTimeout(200)
const mkpLocal = await show('MKP 配置 / 本地', 'preset-scope', 'local')
const slicerLocal = await show('切片器配置 / 本地', 'preset-kind', 'slicer')
const slicerCloud = await show('切片器配置 / 云端', 'preset-scope', 'cloud')
await rad('preset-kind', 'mkp').click({ force: true })
await page.waitForTimeout(200)
const mkpCloud = await show('MKP 配置 / 云端', 'preset-kind', 'mkp')

if (mkpLocal.heads.join() === slicerLocal.heads.join()) {
  problems.push('MKP 与切片器的表头一样（列应该随类型变）')
}

/* ---------- 3. 回本地，点行展开 ---------- */
await rad('preset-scope', 'local').click({ force: true })
await page.waitForTimeout(250)
const before = await facts()
const row = page.locator('main tbody tr').filter({ has: page.locator('td') }).first()
await row.click()
await page.waitForTimeout(250)
const afterClick = await facts()
console.log(`\n[点行展开] 展开前 dl=${before.expanded} 展开后 dl=${afterClick.expanded}`)
if (afterClick.expanded <= before.expanded) problems.push('点一行没有展开（dl 数没变）')

/* ---------- 4. 右键菜单 ---------- */
await row.click({ button: 'right' })
await page.waitForTimeout(250)
const menu = await page.evaluate(() => {
  const ul = document.querySelector('[role="menu"]')
  if (ul === null) return null
  return [...ul.querySelectorAll('[role="menuitem"]')].map((b) => ({
    label: b.textContent?.trim() ?? '',
    disabled: b.getAttribute('data-on') !== '1',
  }))
})
console.log(`\n[右键菜单] ${menu === null ? '没有出现' : menu.map((m) => `${m.label}${m.disabled ? '(灰)' : ''}`).join(' | ')}`)
if (menu === null) problems.push('右键一行没有菜单')
await page.screenshot({ path: `${shotDir}/presets-menu.png` })
await page.keyboard.press('Escape')
await page.waitForTimeout(150)

/* ---------- 5. 跨页那一条：BBS 行右键 → 「在 BBS 预设查看器中打开」 ---------- */
/*
 * 这一条量的是**外壳那一层**的接线：点了之后 tab 要切到 BBS。
 * BBS 页本轮还是空态（`PagePlaceholder`），所以落地之后看到的应该是那句空态文案 ——
 * 那也算通过：说明「目标文件名 + 切页」这条意图真的传出去了。
 */
await rad('preset-kind', 'slicer').click({ force: true })
await page.waitForTimeout(200)
await rad('preset-scope', 'local').click({ force: true })
await page.waitForTimeout(250)
const bbsRow = page.locator('main tbody tr').filter({ has: page.locator('td:not([colspan])') }).first()
await bbsRow.click({ button: 'right' })
await page.waitForTimeout(200)
const bbsItem = page.getByRole('menuitem', { name: '在 BBS 预设查看器中打开' })
const bbsCount = await bbsItem.count()
const bbsDisabled = bbsCount > 0 ? await bbsItem.first().getAttribute('data-on') : null
console.log(`\n[BBS 入口] 菜单项 ${bbsCount} 个，data-on=${bbsDisabled}（'1' = 可点）`)
if (bbsCount === 0) {
  problems.push('BBS 行右键没有「在 BBS 预设查看器中打开」')
} else if (bbsDisabled === '1') {
  await bbsItem.first().click()
  await page.waitForTimeout(350)
  const after = await page.evaluate(() => ({
    text: (document.querySelector('main')?.innerText ?? '').replace(/\s+/g, ' ').trim(),
    active: document.querySelector('header nav [aria-current="page"]')?.textContent?.trim() ?? '',
  }))
  console.log(`[BBS 入口] 点完落在 ${after.active}：${after.text.slice(0, 80)}`)
  if (after.active !== 'BBS 预设') problems.push(`点「在 BBS 预设查看器中打开」之后没有切到 BBS（现在是 ${after.active}）`)
} else {
  problems.push('BBS 行的「在 BBS 预设查看器中打开」是灰的（BBS 工艺 profile 应该可点）')
}
await page.getByRole('button', { name: '预设', exact: true }).first().click()
await page.waitForTimeout(300)

/* ---------- 6. 两档尺寸截图 ---------- */
await page.setViewportSize({ width: 1760, height: 900 })
await page.waitForTimeout(300)
await page.screenshot({ path: `${shotDir}/presets-ultra.png` })
console.log(`\n截图 ${shotDir}/presets-ultra.png (1760x900)`)

await page.setViewportSize({ width: 900, height: 640 })
await page.waitForTimeout(400)
await rad('preset-kind', 'mkp').click({ force: true })
await page.waitForTimeout(200)
await rad('preset-scope', 'local').click({ force: true })
await page.waitForTimeout(300)
const compact = await facts()
await page.screenshot({ path: `${shotDir}/presets-compact.png` })
console.log(`截图 ${shotDir}/presets-compact.png (900x640) 行 ${compact.dataRows} 计数 ${compact.counts}`)
console.log(`表头 ${compact.heads.join(' / ')}`)
console.log(`行内容 ${compact.rows.join(' || ') || '(无)'}`)

await browser.close()

if (problems.length > 0) {
  console.log(`\n发现问题 ${problems.length} 条：`)
  for (const p of problems) console.log(`  - ${p}`)
  process.exit(1)
}
console.log('\n预设页：两轴可点、四张表可读、点行展开、右键菜单出得来，控制台没有 error')
