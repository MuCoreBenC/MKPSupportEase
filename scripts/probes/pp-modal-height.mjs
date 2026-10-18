#!/usr/bin/env node
/*
 * 判据：**后处理模态框换一屏不许变大**（2026-10-18）。
 *
 * # 它防的是哪一种坏
 *
 * 作者连着两轮说「模态框尺寸变化了」，两次都栽在同一件事上：那一屏的几块各自
 * 内联写死高度（一次是板子按宽度算出 160、一次是翻页槽比两栏事实高一截），
 * 于是"结果"和"挪塔那一问"两扇框一高一矮 —— 明明只是换个内容，窗口却跳。
 *
 * 这条探针把那两屏（外加"机型不一致"那一问）都开出来，**量 `.box` 的实际高度**，
 * 三屏必须一致（≤1px）。它比 eslint 强的地方就在这：eslint 只能挡"又手写一个数"，
 * 量不了渲染出来的高度。
 *
 * # 怎么跑
 *
 *   node scripts/probes/pp-modal-height.mjs        # 需要 dev server（默认 5321）
 *
 * 靠 `src/api/mock.ts` 的 `devPostProcessRun()`（只在 dev + URL 带 `?pp=` 时给假快照）：
 *   `?pp=result` / `?pp=relocate` / `?pp=mismatch`
 * 截图落 `tmp-shots/`（该目录已 gitignore）。
 */
import fs from 'node:fs'
import { chromium } from 'playwright-core'

const BASE = process.env.PROBE_URL ?? 'http://localhost:5321/'
const SHOTS = 'tmp-shots'
fs.mkdirSync(SHOTS, { recursive: true })

const SCREENS = [
  { name: 'result', query: '?pp=result', title: '后处理结果' },
  { name: 'relocate', query: '?pp=relocate', title: '后处理进度' },
  { name: 'mismatch', query: '?pp=mismatch', title: '后处理进度' },
]

const browser = await chromium.launch()
/*
 * ★ 两套环境各量一遍（2026-10-18）：作者真机是**深色 + 1080 宽**，浅色 1200 宽量着相等、
 *   深色下未必 —— 高度差过一次就是在这里漏掉的。
 */
const THEMES = [
  { name: 'light', width: 1200, height: 860, theme: 'light' },
  { name: 'dark', width: 1080, height: 800, theme: 'dark' },
]

const got = []
/* 另外两条：翻页槽与两栏事实的高度差（=作者说的"留空"） */
const extras = []
/* 标题栏命中测试（拖窗能不能收到 mousedown） */
const drags = []
for (const env of THEMES) {
  const page = await browser.newPage({ viewport: { width: env.width, height: env.height } })
  for (const s of SCREENS) {
    await page.goto(BASE + s.query, { waitUntil: 'domcontentloaded' })
    const boxEl = page.locator(`div[role="dialog"][aria-label="${s.title}"]`)
    await boxEl.waitFor({ state: 'visible', timeout: 8000 })
    /* 主题由 `<html data-theme>` 唯一决定（与真机同一条路） */
    await page.evaluate((t) => document.documentElement.setAttribute('data-theme', t), env.theme)
    /* 让 hero / 事实表 / 两幅图都排完（图是异步取的板规格） */
    await page.waitForTimeout(700)
    const box = await boxEl.boundingBox()
    got.push({
      env: env.name,
      name: s.name,
      w: Math.round(box.width),
      h: Math.round(box.height),
    })

    /*
     * **标题栏必须还是窗口把手**（2026-10-18 作者：「模态框会导致窗口拖不动」）：
     * 后处理那一屏的层铺满整扇窗，标题栏被它盖住时拖窗收不到 mousedown。
     * 这里做命中测试：标题栏那一带点下去，命中的元素必须 still 落在 `<header>` 里
     * （页签按钮也在 header 里，同样算过；`useTitlebarDrag` 挂在 header 上）。
     */
    const dragHit = await page.evaluate(() => {
      const x = Math.round(window.innerWidth / 2)
      const el = document.elementFromPoint(x, 10)
      /* 两种合法把手：标题栏本身（`<header>`，`useTitlebarDrag` 挂在它上面），
         或「跑着 / 问句」那一态接管标题栏的 `data-drag-handle` 拖窗带 */
      const inHeader = el?.closest('header') != null
      const inShield = el?.closest('[data-drag-handle="true"]') != null
      return { tag: el?.tagName ?? null, dragOk: inHeader || inShield }
    })
    drags.push({ env: env.name, name: s.name, ...dragHit })
    await page.screenshot({ path: `${SHOTS}/pp-${env.name}-${s.name}.png` })

    /*
     * 挪塔那一屏有**两页**（说明 / 原始信息，左下角那颗小钮翻）。预览档会把页脚换成
     * 「关闭预览」，翻不了页 —— 但两页是**叠在同一格里**的，信息页的高度可以直接量：
     * 槽高必须等于两栏事实的自然高度（多出来那截就是作者说的"留空太多"）。
     */
    if (s.name === 'relocate') {
      const slot = await page.evaluate(() => {
        const box = document.querySelector('[class*="relocateSlot"]')
        const groups = box?.querySelector('[class*="groups"]')
        const h = (el) => (el === null || el === undefined ? 0 : el.getBoundingClientRect().height)
        return { slot: Math.round(h(box)), groups: Math.round(h(groups)) }
      })
      extras.push({ env: env.name, name: 'slot-vs-facts', gap: slot.slot - slot.groups })
    }
  }
  await page.close()
}

await browser.close()

/* 基准 = 同一套环境里的"结果"那一屏（作者就是拿它比的） */
const diffs = []
for (const env of THEMES) {
  const base = got.find((g) => g.env === env.name && g.name === 'result')
  for (const g of got.filter((x) => x.env === env.name)) {
    diffs.push({ env: env.name, name: g.name, d: g.h - base.h })
  }
}
const heightOk = diffs.every((g) => Math.abs(g.d) <= 1)
/* 翻页槽不许比两栏事实高（高了就是信息页底下那片空） */
const slotOk = extras.every((g) => g.gap <= 1)
/* 模态框开着时，标题栏仍要收得到拖窗的 mousedown */
const dragOk = drags.length > 0 && drags.every((d) => d.dragOk)
const ok = heightOk && slotOk && dragOk
console.log(JSON.stringify({ ok, got, diffs, extras, drags }, null, 2))
process.exit(ok ? 0 : 1)
