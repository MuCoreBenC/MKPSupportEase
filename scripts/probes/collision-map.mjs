#!/usr/bin/env node
/*
 * 判据：**碰撞失败那条记录要摆出「建议」与「碰撞位置」两块，图要真画出来**（2026-10-18）。
 *
 * # 它防的是哪一种坏
 *
 * 报告页这两块是「数据说了算」的：`_meta.json` 的 `collision` 是 `null` ⇒ 整组不摆。
 * 于是最可能出现的坏法有两种，都是静悄悄的：
 *   ① 前端判空写错（`collision !== undefined`）⇒ `null` 过闸 ⇒ 读几何当场抛、整屏白掉；
 *   ② 数据有、图没画出来（坐标算错 / 塔图落点跑到床外）⇒ 用户看到一块空画布。
 *
 * 这条探针把两份"真形状"的碰撞事实塞进报告夹具，打开失败那一行，确认：
 *   1. 页签里多出「建议」「碰撞位置」两项；
 *   2. 「建议」那组有"往哪边挪 / 至少多少 mm"，并挂着建议码；
 *   3. 俯视图里真的有塔块、模型框、冲突点（数 DOM 节点，不是看颜色）；
 *   4. 成功那一行**没有**这两块（`collision` 是 `null` ⇒ 整组不摆）。
 *
 * # 怎么跑
 *
 *   node scripts/probes/collision-map.mjs        # 需要 dev server（默认 5321）
 *
 * 它**临时**改 `src/api/mockServer/reports.ts`，跑完**还原**（备份 + try/finally）。
 * 所以别在改那份夹具的同时跑它。截图落 `tmp-shots/`（该目录已 gitignore）。
 */

import fs from 'node:fs'
import { chromium } from 'playwright-core'

const FIXTURE = 'src/api/mockServer/reports.ts'
const SHOTS = 'tmp-shots'
const URL = process.env.PROBE_URL ?? 'http://localhost:5321/'

/* 一份"真形状"的碰撞事实（数值照内核那份的形状编，键一个不错） */
const COLLISION = `{
    kind: 'model',
    target: 'model',
    direction: '右上',
    requiredMoveX: 12.5,
    requiredMoveY: 8,
    conflictCount: 37,
    conflictPoints: [
      [46.2, 38.4], [47.1, 39.2], [45.8, 37.6], [48.4, 40.1], [46.9, 38.9],
      [50.2, 41.3], [45.1, 37.1], [49.6, 40.8], [47.7, 39.6], [46.4, 38.6],
    ],
    towerBBox: { xMin: 40, xMax: 57.04, yMin: 20, yMax: 37.04 },
    modelBBox: { xMin: 44, xMax: 96, yMin: 26, yMax: 88 },
    printable: { xMin: 0, xMax: 256, yMin: 0, yMax: 256 },
    tower: {
      outerStructure: 'brim',
      bodyMm: 19.58,
      ribWidth: 3,
      ribExtraLength: 4,
      ribFilletWall: false,
      sheathBaseExpand: 5,
    },
    forbiddenZones: [],
    adviceCode: 'E_GCODE_COLLISION_002',
    adviceText: '建议：把模型往右上挪 —— X 至少 12.5mm，Y 至少 8.0mm',
  }`

const backup = fs.readFileSync(FIXTURE, 'utf8')
/*
 * 只改**第二处**（通用详情 = 失败那几行走的那条路）：第一处是成功那条记录的详情，
 * 保持 `null` —— 探针最后要拿它反证"成功记录没有这两块"。
 */
const first = backup.indexOf('collision: null,')
const second = backup.indexOf('collision: null,', first + 1)
if (first < 0 || second < 0) {
  console.error('没找到要打补丁的 collision 那两处 —— 夹具结构改过了？')
  process.exit(2)
}
const patched = `${backup.slice(0, second)}collision: ${COLLISION},${backup.slice(
  second + 'collision: null,'.length,
)}`

let browser
try {
  fs.writeFileSync(FIXTURE, patched)
  fs.mkdirSync(SHOTS, { recursive: true })
  await new Promise((r) => setTimeout(r, 1200))

  browser = await chromium.launch({ channel: 'chrome' })
  const page = await browser.newPage({ viewport: { width: 1200, height: 860 } })
  const errs = []
  page.on('pageerror', (e) => errs.push(String(e)))

  await page.goto(URL, { waitUntil: 'networkidle' })
  /* 进报告页：主导航那一排（点不到就直接找页上的 section[aria-label="执行报告"]） */
  for (const name of ['报告', '执行报告']) {
    const t = page.getByRole('button', { name }).first()
    if ((await t.count()) > 0) {
      await t.click().catch(() => {})
      break
    }
  }
  await page.waitForSelector('section[aria-label="执行报告"]', { timeout: 8000 })
  await page.waitForTimeout(600)

  /* 点一条**失败**记录（行按钮上的 `data-ok="false"` 就是它 —— 行里不写"失败"两个字） */
  await page.locator('button[data-ok="false"]').first().click()
  await page.waitForTimeout(900)

  const got = await page.evaluate(() => {
    const tabs = [...document.querySelectorAll('[role="tab"]')].map((b) => b.textContent?.trim())
    const body = document.body.innerText
    /* 俯视图里的三类东西：塔块 svg、模型虚线框、冲突点圆 */
    const svgs = [...document.querySelectorAll('svg')]
    const towerSvg = svgs.find((v) => (v.textContent ?? '').includes('塔'))
    const dots = svgs.flatMap((v) => [...v.querySelectorAll('circle')])
    /* 虚线框是 CSS 给的（`stroke-dasharray` 在样式表里），只能按 class 名找
       —— CSS Modules 会把名字哈希掉，所以用"包含"匹配 */
    const dashed = svgs.flatMap((v) => [
      ...v.querySelectorAll('rect[class*="model"], rect[class*="towerBox"]'),
    ])
    return {
      tabs,
      hasAdviceTab: (tabs ?? []).includes('建议'),
      hasMapTab: (tabs ?? []).includes('碰撞位置'),
      adviceTextShown: /建议：把模型往右上挪/.test(body),
      adviceMmShown: /12\.5 mm/.test(body) && /8\.0 mm/.test(body),
      codeShown: /E_GCODE_COLLISION_002/.test(body),
      towerDrawn: towerSvg !== undefined,
      dots: dots.length,
      dashedRects: dashed.length,
    }
  })

  await page.screenshot({ path: `${SHOTS}/collision-map.png`, fullPage: false })

  /* 成功那一行：两块都必须消失 */
  await page.locator('button[data-ok="true"]').first().click()
  await page.waitForTimeout(700)
  const afterOk = await page.evaluate(() => ({
    tabs: [...document.querySelectorAll('[role="tab"]')].map((b) => b.textContent?.trim()),
  }))

  const ok =
    got.hasAdviceTab === true &&
    got.hasMapTab === true &&
    got.adviceTextShown === true &&
    got.adviceMmShown === true &&
    got.codeShown === true &&
    got.towerDrawn === true &&
    got.dots >= 5 &&
    got.dashedRects >= 2 &&
    !(afterOk.tabs ?? []).includes('碰撞位置') &&
    errs.length === 0

  console.log(JSON.stringify({ ok, ...got, afterOkTabs: afterOk.tabs, errs }, null, 2))
  process.exitCode = ok ? 0 : 1
} finally {
  fs.writeFileSync(FIXTURE, backup)
  if (browser) await browser.close()
}
