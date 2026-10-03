/*
 * 工作台「机型与版本 · 品牌」+「套餐」可读性 探针（2026-10-03，作者五连问那条）。
 *
 * # 为什么单独一条
 *
 * 这一刀把两页的界面改了，两处都**没有网**：
 *   ① 品牌从"机型下拉里的一个字符串"升成一等条目 —— 左列有品牌行、右侧有品牌卡，
 *      显示名与品牌图能改（「不管客户端消不消费都提供」）。没有后端命令的界面改动
 *      最怕"看着像能用、其实没落盘"。
 *   ② 套餐页的四件事（左列两行读法、右标题显示名、选择版本改树状去背景、
 *      待确认 chip 不再粘在一起）都是"不改也不报错"的毛病 —— 只有探针能盯住。
 *
 * # 判据
 *
 *   1. 机型与版本页：左列有品牌段（`[id^="t-brand-"]`），行里**显示名 + id + 台数**都在
 *   2. 点品牌 → 品牌卡：显示名 / 品牌图 / 这个品牌下的机型 三样都在，
 *      且品牌图那枚 `<img src="/assets/…">` 真解码（`naturalWidth > 0`）
 *   3. 改显示名（demo 数据）→ 左列行与卡头跟着变（证明是"改了、回了新清单"，不是摆设），
 *      改完**改回原名**（探针可重复跑）
 *   4. 新增品牌 → 左列多一行、右侧切到新品牌卡；**撞名（只差大小写）当场被拒**（toast 说人话）
 *   5. 套餐页：左列行 = 显示名一行、id + "装了 N 个文件"一行；卡头大标题 = 显示名、附注 = id
 *   6. 「选择版本…」= **树状**（机型组头 + 缩进的版本行），行**没有垫背景色**；
 *      勾两个 → 完成 → 待确认 chip 里 uid 与"它现在指着谁"之间有间距（不粘连）
 *   7. 状态栏：`upNote` 只摆 `仓库名/目录名`（不是一整条绝对路径）；
 *      导航缩到 150px 时这一块**不越出导航栏**（实测过溢出压内容区那种病）
 *   8. 控制台没有 error（`/favicon.ico` 的 404 是既有的，放过）
 *
 * # 怎么跑（与 asset-preview.mjs 同三步，探针自己不起服务；用完关掉 4174）
 *
 *   npm run build:workbench
 *   BUILD_WORKBENCH=1 NODE_ENV=development npx vite build --target esnext \
 *     --outDir node_modules/.cache/wb-probe/dist --emptyOutDir
 *   npx vite preview --outDir node_modules/.cache/wb-probe/dist --port 4174 --strictPort
 *   node scripts/probes/brands-bundles.mjs http://localhost:4174/workbench.html --shots
 *
 * 想亲手看它红：把品牌卡的「显示名」那把 input 换成只读（`readOnly`）再跑 —— 第 3 条会响。
 */
import { mkdir } from 'node:fs/promises'

import { chromium } from 'playwright-core'

const rawUrl = process.argv[2]?.startsWith('http') ? process.argv[2] : 'http://localhost:4174/workbench.html'
const url = /[?&]mock=/.test(rawUrl) ? rawUrl : `${rawUrl}${rawUrl.includes('?') ? '&' : '?'}mock=1`
const wantShots = process.argv.includes('--shots')
const shotDir = 'tmp-shots'

const problems = []
const say = (ok, label, detail = '') =>
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${label}${detail ? ` —— ${detail}` : ''}`)
const note = (ok, label, detail = '') => {
  say(ok, label, detail)
  if (!ok) problems.push(label)
}

const BENIGN = [/\/favicon\.ico$/]
const browser = await chromium.launch({ channel: 'msedge' })
const page = await browser.newPage({ viewport: { width: 1560, height: 820 } })

page.on('console', (m) => {
  if (m.type() !== 'error') return
  const at = m.location?.()?.url ?? ''
  if (BENIGN.some((re) => re.test(at)) || /favicon\.ico/.test(m.text())) return
  problems.push(`console.error: ${m.text()}${at ? ` @ ${at}` : ''}`)
})
page.on('pageerror', (e) => problems.push(`pageerror: ${e.message}`))

const until = async (fn, ms = 5000) => {
  const t0 = Date.now()
  for (;;) {
    if (await fn()) return true
    if (Date.now() - t0 > ms) return false
    await page.waitForTimeout(60)
  }
}
const text = (sel) =>
  page
    .locator(sel)
    .first()
    .innerText()
    .then((t) => (t ?? '').replace(/\s+/g, ' ').trim())
    .catch(() => '')
const bodyHas = (s) => page.evaluate((x) => (document.body.innerText ?? '').includes(x), s)

if (wantShots) await mkdir(shotDir, { recursive: true })
await page.goto(url, { waitUntil: 'load' })
await page.locator('[class*="shellBody"]').first().waitFor({ timeout: 15000 })

/* ———————————————— ① 品牌段 ———————————————— */
await page.locator('button[title="机型与版本"]').first().click()
const brandRows = await until(() => page.locator('[id^="t-brand-"]').count().then((n) => n > 0))
note(brandRows, '机型与版本页左列有品牌段（`t-brand-*` 行出来了）')
if (!brandRows) {
  console.log('-'.repeat(96))
  await browser.close()
  for (const p of problems) console.log(`  - ${p}`)
  process.exit(1)
}

const firstBrand = await page.locator('[id^="t-brand-"]').first().getAttribute('id')
const brandRowText = await text(`[id="${firstBrand}"]`)
note(
  brandRowText.includes('拓竹 (Bambu Lab)') && brandRowText.includes('Bambu Lab') && /台机型/.test(brandRowText),
  '品牌行把三件事摆全：显示名 / id / 台数',
  brandRowText,
)

/* ———————————————— ② 品牌卡 ———————————————— */
await page.locator(`[id="${firstBrand}"]`).first().click()
const cardTextOf = () =>
  page.evaluate(() => {
    const el = [...document.querySelectorAll('[class*="cardBody"]')].find((e) =>
      (e.innerText ?? '').includes('这个品牌下的机型'),
    )
    return el === undefined ? '' : (el.innerText ?? '').replace(/\s+/g, ' ')
  })
const brandCard = await until(async () => (await cardTextOf()).includes('品牌图'))
/** 品牌图那枚缩略图：**要等它解码**（img 刚插进 DOM 时 naturalWidth 还是 0 —— 第一版就栽在这） */
const brandImgNow = () =>
  page.evaluate(() => {
    const el = [...document.querySelectorAll('[class*="cardBody"] img[src*="/assets/"]')].find((e) =>
      (e.getAttribute('src') ?? '').includes('brands/'),
    )
    if (el === undefined) return null
    return { src: el.getAttribute('src') ?? '', loaded: el.naturalWidth > 0 && el.naturalHeight > 0 }
  })
let brandShot = await brandImgNow()
if (brandShot !== null && !brandShot.loaded) {
  await until(async () => (await brandImgNow())?.loaded === true, 3000)
  brandShot = await brandImgNow()
}
note(brandCard, '点品牌，右侧开的是品牌卡', (await cardTextOf()).slice(0, 120))
note(
  brandShot !== null && brandShot.loaded,
  '品牌卡上的品牌图**真的解码了**（不是破图占位）',
  brandShot === null ? '卡上没有 /assets/brands/ 的 img' : `${brandShot.src} loaded=${brandShot.loaded}`,
)
note(
  (await cardTextOf()).includes('这个品牌下的机型') && (await cardTextOf()).includes('A1'),
  '品牌卡列出「这个品牌下的机型」（反查后端算）',
)

/* ———————————————— ③ 改显示名（改完改回去） ———————————————— */
const nameInput = page.locator('input[aria-label="品牌显示名"]').first()
const originalName = await nameInput.inputValue()
await nameInput.fill('拓竹（探针临时）')
await nameInput.blur()
const renamed = await until(async () => {
  const row = await text(`[id="${firstBrand}"]`)
  const head = await text('[class*="cardHead"] h2')
  return row.includes('探针临时') && head.includes('探针临时')
}, 4000)
note(renamed, '改显示名 → 左列行与卡头都跟着变（不是只改了内存里的字）')
await nameInput.fill(originalName)
await nameInput.blur()
await until(async () => (await text(`[id="${firstBrand}"]`)).includes('拓竹'))

/* ———————————————— ④ 新增品牌 + 撞名 ———————————————— */
await page.locator('button', { hasText: '新增品牌' }).first().click()
await page.locator('[class*="scrim"] input[aria-label="品牌 id"]').first().fill('Probe Lab')
await page
  .locator('[class*="scrim"] input[aria-label="品牌显示名"]')
  .first()
  .fill('探针 (Probe Lab)')
await page.locator('[class*="scrim"] button', { hasText: '新增品牌' }).last().click()
const added = await until(() => page.locator('[id="t-brand-Probe Lab"]').count().then((n) => n === 1), 4000)
note(added, '新增品牌 → 左列多出一行')
note(
  added && (await text('[class*="cardHead"] h2')).includes('探针 (Probe Lab)'),
  '新增完右侧切到新品牌卡（大标题 = 显示名）',
)

/* 撞名：只差大小写 */
await page.locator('button', { hasText: '新增品牌' }).first().click()
await page.locator('[class*="scrim"] input[aria-label="品牌 id"]').first().fill('probe LAB')
await page.locator('[class*="scrim"] input[aria-label="品牌显示名"]').first().fill('撞名的')
await page.locator('[class*="scrim"] button', { hasText: '新增品牌' }).last().click()
const collided = await until(() => bodyHas('已经有一个叫'), 3000)
note(collided, '撞名（只差大小写）被后端当场拒，toast 说人话')
if (collided) await page.locator('[class*="scrim"] button', { hasText: '取消' }).first().click()

if (wantShots) await page.screenshot({ path: `${shotDir}/wb-brand-probe.png` })

/* ———————————————— ④.5 机型那一侧的「品牌」格：显示名 + 看品牌 ———————————————— */
await page.locator('button[title="机型与版本"]').first().click()
await page.locator('[id^="t-machine-"]').first().click()
const machineField = await page
  .evaluate(() => {
    const el = [...document.querySelectorAll('[class*="kv"]')].find((e) =>
      (e.textContent ?? '').includes('品牌'),
    )
    return el === null ? '' : (el.textContent ?? '').replace(/\s+/g, ' ')
  })
  .catch(() => '')
note(
  machineField.includes('拓竹 (Bambu Lab)'),
  '机型卡「品牌」那格显示的是**显示名**（值仍是品牌 id —— 之前这里选一次会把显示名写进机型文件）',
  machineField.slice(0, 80),
)
const jump = await page.locator('button', { hasText: '看品牌' }).first()
if ((await jump.count()) > 0) {
  await jump.click()
  const back = await until(async () => (await text('[class*="cardHead"] h2')).includes('拓竹'))
  note(back, '机型卡上的「看品牌」能跳到那张品牌卡')
} else {
  note(false, '机型卡上应有「看品牌」按钮（品牌能对上时）')
}

/* ———————————————— ⑤ 套餐页：两行读法 + 卡头 ———————————————— */
await page.locator('button[title="套餐"]').first().click()
await until(() => page.locator('[id^="t-bundle-"]').count().then((n) => n > 0))
const rowName = await text('[id^="t-bundle-"] [class*="rowName"]')
const rowAll = await text('[id^="t-bundle-"]')
note(
  rowName !== '' && !rowName.includes('…') && rowAll.includes('装了'),
  '套餐首行：显示名完整（不再是「官…」），下面一行是 id + 装了 N 个文件',
  rowAll,
)
const headH2 = await text('[class*="cardHead"] h2')
const headNote = await text('[class*="cardHead"] [class*="cardNote"]')
note(
  headNote.includes('_') || /^[A-Z]/.test(headNote),
  '套餐卡头：大标题是显示名、附注才是 id',
  `${headH2} / ${headNote}`,
)

/* ———————————————— ⑥ 选择版本：树状、无背景、chip 不粘 ———————————————— */
await page.locator('button', { hasText: '选择版本…' }).first().click()
const tree = await until(() => page.locator('[class*="pickTree"] [class*="pMgName"]').count().then((n) => n > 0))
note(tree, '「选择版本…」是树状（有 `pMgName` 机型组头）')
const groupText = await text('[class*="pickTree"] [class*="pMgName"]')
note(/\d/.test(groupText), '组头带台数（机型名 + 数量）', groupText)

const rowBg = await page.evaluate(() => {
  const el = document.querySelector('[class*="pickTree"] [role="checkbox"]')
  return el === null ? null : getComputedStyle(el).backgroundColor
})
note(
  rowBg === 'rgba(0, 0, 0, 0)' || rowBg === 'transparent',
  '树行没有垫背景色（作者：「这个没必要加背景吧」）',
  `background-color = ${rowBg}`,
)

const pickRows = page.locator('[class*="pickTree"] [role="checkbox"]')
const nPick = Math.min(2, await pickRows.count())
for (let i = 0; i < nPick; i += 1) await pickRows.nth(i).click()
await page.locator('[class*="scrim"] button', { hasText: '完成（' }).first().click()
await until(async () => (await page.locator('[class*="chips"] [class*="chip"]').count()) >= nPick)
const chipGap = await page.evaluate(() => {
  // 认「待确认」那一排：chip 里带着「→ 现在：…」或「已经指着它」的才是它
  // （别的页也常驻着 chips —— 品牌卡的机型 chip 只有一个孩子，会把它误认成"没间距"）
  const chip = [...document.querySelectorAll('[class*="chips"] [class*="chip"]')].find(
    (c) => /→|已经指着它/.test(c.textContent ?? '') && c.children.length >= 2,
  )
  if (chip === undefined) return null
  const a = chip.children[0].getBoundingClientRect()
  const b = chip.children[1].getBoundingClientRect()
  return Math.round(b.left - a.right)
})
note(
  chipGap !== null && chipGap >= 4,
  '待确认 chip 里 uid 与「它现在指着谁」有间距（不再粘成 `X1C/LITEA1_…`）',
  `gap = ${chipGap}px`,
)
if (wantShots) await page.screenshot({ path: `${shotDir}/wb-chips-probe.png` })

/* ———————————————— ⑦ 状态栏：短名字 + 不越界 ———————————————— */
const upNote = await text('[class*="upNote"]')
note(
  upNote === 'MKPSupportEase/presets',
  '状态栏只摆「仓库名/目录名」，不是一整条绝对路径',
  upNote,
)
await page.evaluate(() => {
  const el = [...document.querySelectorAll('*')].find(
    (e) => e instanceof HTMLElement && e.style.getPropertyValue('--nav-w') !== '',
  )
  if (el instanceof HTMLElement) el.style.setProperty('--nav-w', '150px')
})
await page.waitForTimeout(200)
const spill = await page.evaluate(() => {
  const nav = document.querySelector('nav')
  const up = document.querySelector('[class*="upstream"]')
  if (nav === null || up === null) return null
  return Math.round(up.getBoundingClientRect().right - nav.getBoundingClientRect().right)
})
note(spill !== null && spill <= 0, '导航缩到 150px 时状态块不越出导航栏', `右边界差 ${spill}px`)
if (wantShots) await page.screenshot({ path: `${shotDir}/wb-narrow-probe.png` })

/* ———————————————— 结算 ———————————————— */
console.log('-'.repeat(96))
if (problems.length === 0) {
  console.log('品牌 + 套餐 + 状态栏：全部通过。')
} else {
  console.log(`发现 ${problems.length} 处问题：`)
  for (const p of problems) console.log(`  - ${p}`)
}
await browser.close()
process.exit(problems.length === 0 ? 0 : 1)
