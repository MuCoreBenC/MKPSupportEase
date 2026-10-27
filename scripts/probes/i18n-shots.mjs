/*
 * 探针：整站截图（2026-10-26「英文界面还有中文」那一轮的走查工具）。
 *
 * 用途不是断言，是**取证**：把每个页面在两种语言下各拍一张，落在
 * `tmp-shots/i18n-screens/<阶段>/<页>/<语言>.png`，改前改后两套直接对照。
 *
 * 用法（先 `npm run build`，再让静态预览在 4173 上跑着）：
 *   npm run build
 *   npx vite preview --port 4173 --strictPort
 *   node scripts/probes/i18n-shots.mjs before      # 改之前
 *   node scripts/probes/i18n-shots.mjs after       # 改之后
 *
 * 落点 `tmp-shots/` 在 .gitignore 里 —— 这些图只在本机看，不入库。
 *
 * # 覆盖的页
 *
 *   home / presets / presets-compare / presets-params / calib / bbs /
 *   report / report-detail / settings
 *
 * 每页两张：视口一张（`<语言>.png`）+ 整页一张（`<语言>-full.png`）。
 * 语言两种：`en` 与 `zh`，同一页并排放，一眼就能看出英文页里哪句还是中文。
 */
import { mkdir } from 'node:fs/promises'

import { chromium } from 'playwright-core'

const arg = (i, fallback) => {
  const v = process.argv[i]
  return v !== undefined && !v.startsWith('-') ? v : fallback
}

const phase = arg(2, 'before')
const url = process.argv.find((a) => a.startsWith('http')) ?? 'http://localhost:4173/'
const root = `tmp-shots/i18n-screens/${phase}`

await mkdir(root, { recursive: true })

const problems = []

const BENIGN_URL = [/\/favicon\.ico$/]
const benign = (u) => BENIGN_URL.some((re) => re.test(u))
const BENIGN_TEXT = [/未实现的接口: checkRemoteUpdate/]

/* 页签名两种语言都认（页签自 2026-10-25 起随语言变） */
const goTab = async (page, zh, en) => {
  await page.getByRole('button', { name: new RegExp(`^(${zh}|${en})$`) }).first().click()
  await page.waitForTimeout(600)
}

/*
 * 点**整行的 label**（不是原生 input）：`Check` 是自绘的，原生 input 视觉隐藏。
 * 0 = 跟随系统 / 1 = 简体中文 / 2 = English。
 */
const pickLocale = async (page, index) => {
  await page.locator('label:has(input[name="locale"])').nth(index).click()
  await page.waitForTimeout(500)
}

const goSettings = async (page) => {
  await goTab(page, '设置', 'Settings')
  await page.waitForSelector('input[name="locale"]', { timeout: 10000 })
}

/** 一页两张图（视口 + 整页） */
const shoot = async (page, name, locale) => {
  const dir = `${root}/${name}`
  await mkdir(dir, { recursive: true })
  await page.waitForTimeout(250)
  await page.screenshot({ path: `${dir}/${locale}.png` })
  await page.screenshot({ path: `${dir}/${locale}-full.png`, fullPage: true })
}

/* ---------- 预设页的三件套：列表 / 对比台 / 参数弹窗 ---------- */

/** 表里某一行的定位器（展开排那一格是 `td[colspan]`，不算行） */
const rowOf = (page, file) =>
  page
    .locator('main tbody tr[data-rowkey]')
    .filter({ has: page.locator('td:not([colspan])') })
    .filter({ hasText: file })
    .first()

const openMenu = async (page, rowName, itemRe) => {
  const row = rowOf(page, rowName)
  if ((await row.count()) === 0) {
    problems.push(`预设列表里没有「${rowName}」——假后端样本变了？`)
    return false
  }
  await row.click({ button: 'right' })
  await page.waitForTimeout(300)
  const item = page.getByRole('menuitem', { name: itemRe })
  if ((await item.count()) === 0) {
    problems.push(`右键「${rowName}」的菜单里没有 ${itemRe}`)
    return false
  }
  await item.first().click()
  await page.waitForTimeout(900)
  return true
}

const closeDialog = async (page) => {
  await page.keyboard.press('Escape')
  await page.waitForTimeout(400)
}

const shootPresets = async (page, locale) => {
  await goTab(page, '预设', 'Presets')
  await shoot(page, 'presets', locale)

  /* 本地那一档才有 MKP 行（假后端固定样本） */
  const scopeLocal = page.locator('main input[type="radio"][name="preset-scope"][value="local"]')
  if ((await scopeLocal.count()) > 0) {
    await scopeLocal.first().click({ force: true })
    await page.waitForTimeout(400)
  }

  /* 文件名是**数据**，两种语言下都一样（不译） */
  const first = 'A1 涂胶.toml'
  const second = 'A1-fast.toml'

  /* 对比台：右键两下（2026-10-10 定的入口） */
  if (await openMenu(page, first, /^(参数对比|Compare parameters)$/)) {
    if (await openMenu(page, second, /^(与「|Compare with)/)) {
      await shoot(page, 'presets-compare', locale)
    }
    await closeDialog(page)
  }

  /* 参数弹窗 */
  if (await openMenu(page, first, /^(修改参数|Edit parameters)$/)) {
    await shoot(page, 'presets-params', locale)
    await closeDialog(page)
  }
}

const shootReport = async (page, locale) => {
  await goTab(page, '报告', 'Report')
  await shoot(page, 'report', locale)
  /* 点第一条记录看详情（记的是管线各步的明细 —— 截图 4 里那一屏） */
  const target = page.locator('main button').filter({ hasText: '.gcode' }).first()
  if ((await target.count()) === 0) {
    problems.push('报告页没有一条记录 —— 管线明细那一屏截不到')
    return
  }
  await target.click()
  await page.waitForTimeout(700)
  await shoot(page, 'report-detail', locale)
}

const browser = await chromium.launch({ channel: 'msedge' })
const ctx = await browser.newContext({ viewport: { width: 1180, height: 820 }, locale: 'zh-CN' })
const page = await ctx.newPage()

page.on('console', (m) => {
  if (m.type() !== 'error') return
  const at = m.location?.()?.url ?? ''
  if (benign(at) || BENIGN_TEXT.some((re) => re.test(m.text()))) return
  problems.push(`console.error: ${m.text()}${at ? ` @ ${at}` : ''}`)
})
page.on('pageerror', (e) => problems.push(`pageerror: ${e.message}`))

await page.goto(url, { waitUntil: 'load' })
await page.waitForSelector('header nav', { timeout: 10000 })

for (const { locale, index } of [
  { locale: 'zh', index: 1 },
  { locale: 'en', index: 2 },
]) {
  await goSettings(page)
  await pickLocale(page, index)
  await shoot(page, 'settings', locale)

  await goTab(page, '首页', 'Home')
  await shoot(page, 'home', locale)

  await shootPresets(page, locale)

  await goTab(page, '校准', 'Calibration')
  await shoot(page, 'calib', locale)

  await goTab(page, '工艺', 'Process')
  await shoot(page, 'bbs', locale)

  await shootReport(page, locale)
}

await browser.close()

console.log(`截图落在 ${root}/（每页一张视口 + 一张整页，zh / en 各一套）`)
if (problems.length > 0) {
  console.log(`\n${problems.length} 处走查问题：`)
  for (const p of problems) console.log('  -', p)
  process.exitCode = 1
}
