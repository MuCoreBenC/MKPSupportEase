/*
 * 探针：界面语言（2026-10-25「全站 i18n 第一刀」的端到端验收）。
 *
 * 用法（先 `npm run build`，再让静态预览在 4173 上跑着）：
 *   npm run build
 *   npx vite preview --port 4173 --strictPort
 *   node scripts/probes/i18n.mjs [url]
 *
 * # 这一刀验什么
 *
 * 语言档真的能走通：设置页选中 → 落 AppState（mock 内存）→ 通知 → 组件换字。
 * 这一刀只迁了**工艺页那两枚窗口胶囊**和**设置页「外观」里新增的语言行**，
 * 所以探针只断言这两处 —— 其余页面还在阶段 1 的清单里，别在这里替它们打包票。
 *
 * 判定：
 *   ① 浏览器语言钉成 zh-CN + 默认档「跟随系统」⇒ 胶囊读作「关闭 / 小窗」
 *   ② 设置页「外观」下有语言单选，三档齐全，当前选中「跟随系统」
 *   ③ 选 English ⇒ 设置页那一块自己先变英文（Appearance / Interface language）
 *   ④ 回工艺页 ⇒ 胶囊读作「Off / Compact」
 *   ⑤ 切回「简体中文」⇒ 胶囊复原成「关闭 / 小窗」
 *   ⑥ 英文下胶囊**盒宽由最长档撑着**：切档仍然一个像素不动（`This page` 比 `Off` 宽）
 *   ⑦ 0 console error / 0 个 >= 400
 */
import { mkdir } from 'node:fs/promises'

import { chromium } from 'playwright-core'

const url = process.argv[2]?.startsWith('http') ? process.argv[2] : 'http://localhost:4173/'

const shotDir = 'tmp-shots'
await mkdir(shotDir, { recursive: true })

const problems = []
const results = []
const check = (what, ok, detail = '') => {
  results.push({ what, ok, detail })
  if (!ok) problems.push(`${what}${detail ? ` —— ${detail}` : ''}`)
}

const BENIGN_URL = [/\/favicon\.ico$/]
const benign = (u) => BENIGN_URL.some((re) => re.test(u))
const BENIGN_TEXT = [/未实现的接口: checkRemoteUpdate/]

/** 两枚胶囊的可见文字与宽度（只读可见那份，隐形副本见 WindowGadgets.module.css） */
const gadgets = (page) =>
  page.evaluate(() => {
    const flat = (s) => (s ?? '').replace(/\s+/g, ' ').trim()
    const live = (el) => flat(el?.querySelector('span:not([aria-hidden])')?.textContent)
    const pin = document.querySelector('button[aria-label^="Pin window:"]')
    const compact = document.querySelector(
      'button[aria-label="Shrink the window to compact size"], button[aria-label="Restore the window’s previous position and size"]',
    )
    const zhPin = document.querySelector('button[aria-label^="窗口置顶："]')
    const zhCompact = document.querySelector(
      'button[aria-label="把窗口收成小窗"], button[aria-label="还原窗口原本的位置与尺寸"]',
    )
    const w = (el) => (el ? Math.round(el.getBoundingClientRect().width * 10) / 10 : -1)
    /* 两种语言各一套 aria-label，谁在就用谁 */
    const pinEl = pin ?? zhPin
    const compactEl = compact ?? zhCompact
    return {
      pinText: live(pinEl),
      compactText: live(compactEl),
      pinWidth: w(pinEl),
    }
  })

/* 页签文案自 2026-10-25 起随语言变 ⇒ 两种名字都认 */
const goBbs = async (page) => {
  await page.getByRole('button', { name: /^(工艺|Process)$/ }).first().click()
  await page.waitForSelector('[data-role="params"]', { timeout: 10000 })
  await page.waitForTimeout(300)
}

const goSettings = async (page) => {
  await page.getByRole('button', { name: /^(设置|Settings)$/ }).first().click()
  await page.waitForSelector('input[name="locale"]', { timeout: 10000 })
  await page.waitForTimeout(300)
}

/*
 * 点**整行的 label**（不是原生 input）：`Check` 是自绘的，原生 input 视觉隐藏、
 * 上面盖着画墨点的 `<span>` —— 直接点 input 会被那一层拦下来（实测 TimeoutError）。
 * 用户点的也是这一整行。
 */
const pickLocale = async (page, index) => {
  await page.locator('label:has(input[name="locale"])').nth(index).click()
  await page.waitForTimeout(250)
}

const browser = await chromium.launch({ channel: 'msedge' })
const ctx = await browser.newContext({ viewport: { width: 1000, height: 720 }, locale: 'zh-CN' })
const page = await ctx.newPage()

page.on('console', (m) => {
  if (m.type() !== 'error') return
  const at = m.location?.()?.url ?? ''
  if (benign(at) || BENIGN_TEXT.some((re) => re.test(m.text()))) return
  problems.push(`console.error: ${m.text()}${at ? ` @ ${at}` : ''}`)
})
page.on('pageerror', (e) => problems.push(`pageerror: ${e.message}`))
page.on('response', (r) => {
  if (r.status() >= 400 && !benign(r.url())) problems.push(`HTTP ${r.status()} ${r.url()}`)
})

await page.goto(url, { waitUntil: 'load' })
await page.waitForSelector('header nav', { timeout: 10000 })

/* ---------- ① 默认（跟随系统 + zh-CN） ---------- */
await goBbs(page)
let g = await gadgets(page)
check('① 默认档胶囊是中文（关闭 / 小窗）', g.pinText === '关闭' && g.compactText === '小窗', `读作「${g.pinText}」「${g.compactText}」`)
const zhWidth = g.pinWidth

/* ---------- ② 设置页语言单选 ---------- */
await goSettings(page)
const radios = await page.locator('input[name="locale"]').count()
check('② 语言单选三档齐全', radios === 3, `数到 ${radios} 个`)
const checkedSystem = await page.locator('input[name="locale"]').nth(0).isChecked()
check('② 默认选中「跟随系统」', checkedSystem, `system checked=${checkedSystem}`)
const appearanceZh = await page.locator('section[aria-label="外观"]').count()
check('② 「外观」块标题是中文', appearanceZh === 1, `数到 ${appearanceZh} 个`)

/* ---------- ③ 选 English：这一块自己先变英文 ---------- */
await pickLocale(page, 2)
const appearanceEn = await page.locator('section[aria-label="Appearance"]').count()
check('③ 切英文后「外观」块变 Appearance', appearanceEn === 1, `数到 ${appearanceEn} 个`)
const enLabelCount = await page.getByText('Interface language', { exact: true }).count()
check('③ 语言行标题变英文', enLabelCount === 1, `数到 ${enLabelCount} 处「Interface language」`)

/* 第二批迁移（2026-10-25）：页签 + 设置页那几个独立子块也一起换 */
const tabsEn = await page.getByRole('button', { name: 'Settings', exact: true }).count()
check('③ 顶部页签变英文（Settings）', tabsEn === 1, `数到 ${tabsEn} 个 Settings 页签`)
await page.waitForTimeout(600)
const postprocEn = await page.locator('section[aria-label="Post-processing"]').count()
check('③ 「后处理模式」块变英文', postprocEn === 1, `数到 ${postprocEn} 个 Post-processing`)
const glueEn = await page.locator('section[aria-label="Bed pre-glue"]').count()
check('③ 「热床预涂胶」块变英文', glueEn === 1, `数到 ${glueEn} 个 Bed pre-glue`)
await page.screenshot({ path: `${shotDir}/i18n-settings-en.png` })

/* ---------- ④ 回工艺页：胶囊变英文 ---------- */
await goBbs(page)
g = await gadgets(page)
check('④ 英文下胶囊读作 Off / Compact', g.pinText === 'Off' && g.compactText === 'Compact', `读作「${g.pinText}」「${g.compactText}」`)
const enWidth = g.pinWidth
check('④ 英文比中文宽（宽度跟着语言走，没写死）', enWidth > zhWidth, `中文 ${zhWidth}px → 英文 ${enWidth}px`)
await page.screenshot({ path: `${shotDir}/i18n-bbs-en.png` })

/* ---------- ⑥ 英文下切档宽度不动 ---------- */
const pinBtn = page.locator('button[aria-label^="Pin window:"]')
await pinBtn.click()
await page.waitForTimeout(160)
g = await gadgets(page)
check('⑥ 英文下切到 This page 盒宽不变', Math.abs(g.pinWidth - enWidth) < 0.6, `${enWidth}px → ${g.pinWidth}px`)

/* ---------- ⑤ 切回简体中文 ---------- */
await goSettings(page)
await pickLocale(page, 1)
await goBbs(page)
g = await gadgets(page)
check('⑤ 切回中文后胶囊复原', g.pinText === '本页' && g.compactText === '小窗', `读作「${g.pinText}」「${g.compactText}」`)

await ctx.close()
await browser.close()

for (const r of results) console.log(`${r.ok ? '✓' : '✗'} ${r.what}${r.detail ? ` —— ${r.detail}` : ''}`)
console.log(`\n${results.length - problems.length}/${results.length} 条通过`)
if (problems.length) {
  console.log(`\n问题 ${problems.length} 条：`)
  for (const p of problems) console.log(`  · ${p}`)
  process.exitCode = 1
}
