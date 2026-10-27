import { chromium } from 'playwright-core'
const b = await chromium.launch({ channel: 'msedge' })
const p = await b.newPage({ viewport: { width: 1080, height: 900 } })
await p.goto('http://localhost:4173/?mock=1', { waitUntil: 'load' })
await p.getByRole('button', { name: '设置' }).click()
await p.getByRole('button', { name: '去系统里改…' }).click()
await p.getByText('改 .3mf 的默认打开方式').waitFor({ timeout: 8000 })
await p.locator('[role="dialog"] img').first().waitFor({ timeout: 8000 })
await p.waitForTimeout(500)
const m = await p.evaluate(() => {
  const img = document.querySelector('[role="dialog"] img')
  const ir = img.getBoundingClientRect()
  return { imgW: Math.round(ir.width), src: img.getAttribute('src'), marks: document.querySelectorAll('[role="dialog"] span[style*="left"]').length }
})
console.log('客户端:', JSON.stringify(m))
await b.close()
