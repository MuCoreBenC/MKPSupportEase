/*
 * 一次性截图脚本（搜索框高度 36px 验收用）。
 * 跑法：preview 在 4173，`node scripts/probes/shots-round3.mjs`。
 */
import { mkdir } from 'node:fs/promises'

import { chromium } from 'playwright-core'

const url = process.argv[2]?.startsWith('http') ? process.argv[2] : 'http://localhost:4173/'

const browser = await chromium.launch({ channel: 'msedge' })
const page = await browser.newPage({ viewport: { width: 900, height: 640 } })

await page.goto(url, { waitUntil: 'load' })
await page.waitForSelector('header', { timeout: 10000 })
await page.getByRole('button', { name: '预设', exact: true }).first().click()
await page.waitForTimeout(500)

const rad = (name, value) => page.locator(`main input[type="radio"][name="${name}"][value="${value}"]`)
await rad('preset-kind', 'mkp').click({ force: true })
await rad('preset-scope', 'local').click({ force: true })
await page.waitForTimeout(400)

const shotDir = 'tmp-shots'
await mkdir(shotDir, { recursive: true })

/* 工具栏特写：量分段控件与搜索框的实际盒子，顺带截图 */
const boxes = await page.evaluate(() => {
  const pick = (sel) => {
    const el = document.querySelector(sel)
    if (el === null) return null
    const r = el.getBoundingClientRect()
    return { w: Math.round(r.width), h: Math.round(r.height), top: Math.round(r.top) }
  }
  return {
    kindSeg: pick('main [role="radiogroup"][aria-label="文件类型"]'),
    scopeSeg: pick('main [role="radiogroup"][aria-label="位置"]'),
    searchBox: (() => {
      const input = document.querySelector('main input[aria-label="搜索预设文件"]')
      if (input === null) return null
      const box = input.parentElement.getBoundingClientRect()
      return { w: Math.round(box.width), h: Math.round(box.height), top: Math.round(box.top) }
    })(),
  }
})
console.log(JSON.stringify(boxes))

await page.screenshot({ path: `${shotDir}/r3-900.png` })

/* 600 最小窗：mini 130 宽 + 36 高 */
await page.setViewportSize({ width: 600, height: 500 })
await page.waitForTimeout(400)
await page.screenshot({ path: `${shotDir}/r3-600.png` })

/* 切片器 900：高度变化对筛选排无牵连 */
await page.setViewportSize({ width: 900, height: 640 })
await page.waitForTimeout(400)
await rad('preset-kind', 'slicer').click({ force: true })
await page.waitForTimeout(400)
await page.screenshot({ path: `${shotDir}/r3-slicer-900.png` })

await browser.close()
console.log('done: r3-900 / r3-600 / r3-slicer-900')
