import { chromium } from 'playwright-core'
const b = await chromium.launch({ channel: 'msedge' })
const p = await b.newPage({ viewport: { width: 1400, height: 900 } })
await p.goto('http://localhost:4174/workbench.html?mock=1', { waitUntil: 'load' })
await p.locator('[class*="shellBody"]').first().waitFor({ timeout: 15000 })
await p.locator('button[title="标注台"]').first().click()
await p.waitForTimeout(1500)
// 量：图显示宽 + badge 直径（普通模式）
const m1 = await p.evaluate(() => {
  const img = document.querySelector('img[src*="open-with-light"]')
  const badge = document.querySelector('[class*="badge"]')
  const mark = document.querySelector('div[class*="mark"][data-on]')
  const cs = mark ? getComputedStyle(mark) : null
  const bs = badge ? badge.getBoundingClientRect() : null
  const ir = img.getBoundingClientRect()
  return { imgW: Math.round(ir.width), badge: bs ? Math.round(bs.width) : 0, shadow: cs?.boxShadow ?? '' }
})
console.log('普通模式:', JSON.stringify(m1))
// 客户视角
await p.getByRole('button', { name: '客户视角' }).click()
await p.waitForTimeout(700)
const m2 = await p.evaluate(() => {
  const img = document.querySelector('img[src*="open-with-light"]')
  return { imgW: Math.round(img.getBoundingClientRect().width), src: img.getAttribute('src').split('/').pop() }
})
console.log('客户视角:', JSON.stringify(m2))
await b.close()
