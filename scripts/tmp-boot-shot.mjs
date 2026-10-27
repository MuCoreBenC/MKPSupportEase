/* 临时截图：窗口键特写（跑完即删） */
import { chromium } from 'playwright-core'

const browser = await chromium.launch({ channel: 'msedge' })
const page = await browser.newPage({
  viewport: { width: 1000, height: 750 },
  colorScheme: 'light',
  reducedMotion: 'reduce',
  deviceScaleFactor: 2,
})
await page.route('**/src/main.tsx', (r) => r.abort())
await page.goto('http://[::1]:5399/', { waitUntil: 'domcontentloaded' })
await page.waitForTimeout(400)
await page.screenshot({
  path: `${process.env.TEMP}\\boot-wins15.png`,
  clip: { x: 800, y: 0, width: 200, height: 40 },
})
console.log('shot')
await browser.close()
