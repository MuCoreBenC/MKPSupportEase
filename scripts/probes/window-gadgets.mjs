/*
 * 探针：工艺页工具条右端那两枚窗口开关（2026-10-17）。
 *
 * 用法（先 `npm run build`，再让静态预览在 4173 上跑着）：
 *   npm run build
 *   npx vite preview --port 4173 --strictPort
 *   node scripts/probes/window-gadgets.mjs [url]
 *
 * # 浏览器里能验什么、不能验什么
 *
 * 后端在预览里是 mock（`src/api/mock.ts`）：**窗口动作空转、两格账照记**。
 * 所以这里验的是"看得见"的那一半 —— 文字换档、盒子不变宽、窄窗下不挤爆；
 * 真窗口的置顶层级与几何只能真机看（`screencapture -l<winid>` + 系统窗口列表）。
 *
 * 判定：
 *   ① 两枚胶囊都在工具条上，默认是「不置顶」+「窄窗」
 *   ② 置顶点三下：不置顶 → 仅本页置顶 → 始终置顶 → 不置顶（循环闭合）
 *   ③ 每一档**盒子宽度一模一样**（全站铁律：状态不许改变盒子宽度）
 *   ④ 窄窗点两下：窄窗 → 恢复尺寸 → 窄窗
 *   ⑤ 900 与 600 两个宽度下，两枚胶囊都完整可见、工具条不横向溢出
 *   ⑥ 0 console error / 0 个 >= 400
 */
import { mkdir } from 'node:fs/promises'

import { chromium } from 'playwright-core'

const url = process.argv[2]?.startsWith('http') ? process.argv[2] : 'http://localhost:4173/'

const SIZES = [
  { tag: 'compact', width: 900, height: 640 },
  /* 600 = 窗口最小宽（窄窗也是这个宽）—— 这一档最容易把工具条挤爆 */
  { tag: 'narrow', width: 600, height: 640 },
]

const shotDir = 'tmp-shots'
await mkdir(shotDir, { recursive: true })

const problems = []
const results = []
const check = (tag, what, ok, detail = '') => {
  results.push({ tag, what, ok, detail })
  if (!ok) problems.push(`${tag} ${what}${detail ? ` —— ${detail}` : ''}`)
}

const BENIGN_URL = [/\/favicon\.ico$/]
const benign = (u) => BENIGN_URL.some((re) => re.test(u))
const BENIGN_TEXT = [/未实现的接口: checkRemoteUpdate/]

/** 两枚胶囊的可见事实（选择器只认 aria-label，不随 class 哈希变） */
const facts = (page) =>
  page.evaluate(() => {
    const flat = (s) => (s ?? '').replace(/\s+/g, ' ').trim()
    const pin = document.querySelector('button[aria-label^="窗口置顶："]')
    const compact = document.querySelector(
      'button[aria-label="把窗口收成小窗"], button[aria-label="还原窗口原本的位置与尺寸"]',
    )
    const bar = pin?.parentElement?.parentElement ?? null
    const rect = (el) => {
      if (!el) return null
      const r = el.getBoundingClientRect()
      return { x: r.x, y: r.y, w: r.width, h: r.height, right: r.right }
    }
    return {
      pinText: flat(pin?.textContent),
      pinWidth: rect(pin)?.w ?? -1,
      compactText: flat(compact?.textContent),
      compactWidth: rect(compact)?.w ?? -1,
      compactPressed: compact?.getAttribute('aria-pressed') ?? '',
      pinRect: rect(pin),
      compactRect: rect(compact),
      barRect: rect(bar),
      docOverflow: document.documentElement.scrollWidth - window.innerWidth,
      viewport: window.innerWidth,
    }
  })

const browser = await chromium.launch({ channel: 'msedge' })

for (const size of SIZES) {
  const tag = size.tag
  const ctx = await browser.newContext({ viewport: { width: size.width, height: size.height } })
  const page = await ctx.newPage()

  page.on('console', (m) => {
    if (m.type() !== 'error') return
    const at = m.location?.()?.url ?? ''
    if (benign(at) || BENIGN_TEXT.some((re) => re.test(m.text()))) return
    problems.push(`${tag} console.error: ${m.text()}${at ? ` @ ${at}` : ''}`)
  })
  page.on('pageerror', (e) => problems.push(`${tag} pageerror: ${e.message}`))
  page.on('response', (r) => {
    if (r.status() >= 400 && !benign(r.url())) problems.push(`${tag} HTTP ${r.status()} ${r.url()}`)
  })

  await page.goto(url, { waitUntil: 'load' })
  await page.waitForSelector('header nav', { timeout: 10000 })
  await page.getByRole('button', { name: '工艺', exact: true }).first().click()
  await page.waitForSelector('[data-role="params"]', { timeout: 10000 })
  await page.waitForTimeout(500)

  /* ---------- ① 两枚都在、默认档 ---------- */
  let f = await facts(page)
  check(tag, '① 置顶胶囊在，默认「不置顶」', f.pinText === '不置顶', `读作「${f.pinText}」`)
  check(tag, '① 小窗胶囊在，默认「小窗」', f.compactText === '小窗', `读作「${f.compactText}」`)

  const pinW = f.pinWidth
  const compactW = f.compactWidth

  /* ---------- ②③ 置顶三档循环，宽度不许跳 ---------- */
  const pinBtn = page.locator('button[aria-label^="窗口置顶："]')
  const chain = ['仅本页置顶', '始终置顶', '不置顶']
  for (const [i, want] of chain.entries()) {
    await pinBtn.click()
    await page.waitForTimeout(160)
    f = await facts(page)
    check(tag, `② 置顶点第 ${i + 1} 下 →「${want}」`, f.pinText === want, `读作「${f.pinText}」`)
    check(
      tag,
      `③ 第 ${i + 1} 档盒子宽度不变（${pinW}px）`,
      Math.abs(f.pinWidth - pinW) < 0.6,
      `现在 ${f.pinWidth}px`,
    )
  }

  /* ---------- ④ 窄窗开 / 关 ---------- */
  const compactBtn = page.locator(
    'button[aria-label="把窗口收成小窗"], button[aria-label="还原窗口原本的位置与尺寸"]',
  )
  await compactBtn.click()
  await page.waitForTimeout(160)
  f = await facts(page)
  check(tag, '④ 点小窗 →「还原」', f.compactText === '还原', `读作「${f.compactText}」`)
  /* 小窗一开就顺手置顶（作者 2026-10-17 第二轮） */
  check(tag, '④ 小窗自动置顶 →「始终置顶」', f.pinText === '始终置顶', `读作「${f.pinText}」`)
  check(tag, '④ 开启态对读屏是可读的（aria-pressed）', f.compactPressed === 'true', f.compactPressed)
  check(
    tag,
    `③ 小窗开 / 关盒子宽度不变（${compactW}px）`,
    Math.abs(f.compactWidth - compactW) < 0.6,
    `现在 ${f.compactWidth}px`,
  )
  await page.screenshot({ path: `${shotDir}/window-gadgets-${tag}-compact-on.png` })

  await compactBtn.click()
  await page.waitForTimeout(160)
  f = await facts(page)
  check(tag, '④ 再点一下 →「小窗」', f.compactText === '小窗', `读作「${f.compactText}」`)
  /* 还原的同一刻置顶也回到原来那一档（点小窗之前是「不置顶」） */
  check(tag, '④ 还原后置顶也回原档 →「不置顶」', f.pinText === '不置顶', `读作「${f.pinText}」`)

  /* ---------- ③′ 按下时盒子不许位移（作者：「点击的时候好像有像素位移」） ---------- */
  {
    const box0 = await pinBtn.boundingBox()
    await pinBtn.hover()
    await page.mouse.down()
    await page.waitForTimeout(80)
    const box1 = await pinBtn.boundingBox()
    await page.mouse.up()
    await page.waitForTimeout(120)
    const same =
      box0 !== null &&
      box1 !== null &&
      Math.abs(box0.x - box1.x) < 0.5 &&
      Math.abs(box0.y - box1.y) < 0.5 &&
      Math.abs(box0.width - box1.width) < 0.5 &&
      Math.abs(box0.height - box1.height) < 0.5
    check(tag, '③′ 按下不动一个像素', same, `${JSON.stringify(box0)} → ${JSON.stringify(box1)}`)
    /* 这一下按完档位也变了一次，循环回「不置顶」收尾（下面⑤只量几何，不看档位） */
    await page.waitForTimeout(60)
  }

  /* ---------- ⑤ 完整可见、不溢出 ---------- */
  check(
    tag,
    '⑤ 两枚胶囊完整落在视口内',
    f.pinRect !== null &&
      f.compactRect !== null &&
      f.pinRect.x >= 0 &&
      f.compactRect.right <= f.viewport + 0.5,
    `置顶 right ${f.pinRect?.right} / 窄窗 right ${f.compactRect?.right} / 视口 ${f.viewport}`,
  )
  check(
    tag,
    '⑤ 工具条不横向溢出',
    f.docOverflow <= 1,
    `scrollWidth 比视口多 ${f.docOverflow}px`,
  )
  if (f.barRect && f.compactRect) {
    check(
      tag,
      '⑤ 两枚胶囊都在工具条那一行里（没被挤到页外）',
      f.compactRect.right <= f.barRect.right + 0.5 && f.pinRect?.y >= f.barRect.y - 0.5,
      `胶囊 right ${f.compactRect.right} / 工具条 right ${f.barRect.right}`,
    )
  }

  await page.screenshot({ path: `${shotDir}/window-gadgets-${tag}.png` })
  await ctx.close()
}

await browser.close()

for (const r of results) {
  console.log(`${r.ok ? '✓' : '✗'} [${r.tag}] ${r.what}${r.detail ? ` —— ${r.detail}` : ''}`)
}
console.log(`\n${results.length - problems.length}/${results.length} 条通过`)
if (problems.length) {
  console.log(`\n问题 ${problems.length} 条：`)
  for (const p of problems) console.log(`  · ${p}`)
  process.exitCode = 1
}
