/*
 * 工作台「资产库」预览探针 —— **凡是渲染出来的预览图，都必须真的画出来**。
 *
 * # 为什么单独一条（2026-10-03 第一刀补）
 *
 * 这一刀把工作台的资产预览从"`public/assets` 里的副本"改成"dev 直读源 / build 装配进
 * dist"（`tools/assets/plugin.mjs`）。旧那套的病是**静默破图**：界面说文件在、图却取不到
 * （`public/assets` 被上一条命令删掉时）。`workbench-build.mjs` 逛的是生成与发布页与
 * 参数台，**不碰资产库** —— 这条链原本没有网。
 *
 * # 判据（正式：有问题就非零退出）
 *
 *   1. 资产库页进得去，行是齐的（`button[id^="t-asset-"]`）
 *   2. **逐行走一遍**：详情卡里凡渲染出 `<img src="/assets/…">`（`image` / `icon` 这两类
 *      才有），就必须 `naturalWidth > 0`（真解码出来了，不是破图占位）
 *   3. 每一次 `/assets/*` 请求，**按"源文件在不在"分档**：
 *        源里真有（`presets/assets/<rel>`）→ 必须 2xx
 *        源里没有（vite 自己的哈希产物也叫 `/assets/…`；桩里还有几条假路径）→ 只记一行
 *      分档的理由：拿桩数据与构建产物去判，会把"桩不真 / 构建产物名字不真"误报成链坏了。
 *   4. 控制台没有 error，页面没有异常
 *
 * ★ **第 2 条才是这张网**（实测，别指望第 3 条）：`vite preview` 有 SPA 回退 ——
 * 取不到的 `/assets/printers/a1.webp` 返回的是 **`index.html` + HTTP 200**，所以
 * **光看状态码永远查不出破图**；能指出"哪一行、哪个文件"的只有 `naturalWidth`。
 * （反向测试：把预览产物里的 `assets/printers`、`assets/icons` 搬走再跑 ——
 * 第 2 条报 5 行破图，第 3 条照样全绿。真机 Tauri 那一侧没有这条回退，
 * 那条 2xx 断言留着是给 dev（中间件如实 404）与将来用的。）
 *
 * # 怎么跑（与 workbench-build.mjs 同三步，探针自己不起服务；用完关掉 4174）
 *
 *   npm run build:workbench
 *   BUILD_WORKBENCH=1 NODE_ENV=development npx vite build --target esnext \
 *     --outDir node_modules/.cache/wb-probe/dist --emptyOutDir
 *   npx vite preview --outDir node_modules/.cache/wb-probe/dist --port 4174 --strictPort
 *   node scripts/probes/asset-preview.mjs http://localhost:4174/workbench.html --shots
 *
 * 想亲手看它红：把 `presets/assets/printers/a1.webp` 临时改个名再跑 —— 第 2、3 条都会响。
 */
import { existsSync, statSync } from 'node:fs'
import { mkdir } from 'node:fs/promises'
import { join, normalize, resolve, sep } from 'node:path'

import { chromium } from 'playwright-core'

const rawUrl = process.argv[2]?.startsWith('http') ? process.argv[2] : 'http://localhost:4174/workbench.html'
const url = /[?&]mock=/.test(rawUrl) ? rawUrl : `${rawUrl}${rawUrl.includes('?') ? '&' : '?'}mock=1`
const wantShots = process.argv.includes('--shots')
const shotDir = 'tmp-shots'

/** 资产源根：判"这个文件该不该有"就靠它（探针一律从仓库根跑） */
const sourceRoot = resolve(process.cwd(), 'presets', 'assets')

const problems = []
const say = (ok, label, detail = '') =>
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${label}${detail ? ` —— ${detail}` : ''}`)

/** `/assets/<rel>` → 源目录里的绝对路径；不在这个前缀下返回 null */
function sourceFileOf(rawHref) {
  let pathname
  try {
    pathname = new URL(rawHref, url).pathname
  } catch {
    return null
  }
  if (!pathname.startsWith('/assets/')) return null
  let rel
  try {
    rel = decodeURIComponent(pathname.slice('/assets/'.length))
  } catch {
    return { rel: pathname, full: null }
  }
  const full = normalize(join(sourceRoot, rel))
  // 探针自己的收口：只认落在源根里的（越界一律当"源里没有"）
  if (full !== sourceRoot && !full.startsWith(sourceRoot + sep)) return { rel, full: null }
  return { rel, full }
}

/** 源里真存在吗（目录不算） */
const presentInSource = (hit) => hit !== null && hit.full !== null && existsSync(hit.full) && statSync(hit.full).isFile()

const BENIGN = [/\/favicon\.ico$/]
const benign = (text) => BENIGN.some((re) => re.test(text))

const browser = await chromium.launch({ channel: 'msedge' })
const page = await browser.newPage({ viewport: { width: 1760, height: 900 } })

/** 记 `/assets/` 请求的状态：rel → status（只记源里真有的那些，其余是产物/桩，不参与判） */
const realResponses = new Map()
page.on('response', (r) => {
  const hit = sourceFileOf(r.url())
  if (!presentInSource(hit)) return
  realResponses.set(hit.rel, r.status())
  if (r.status() < 200 || r.status() >= 300) problems.push(`源里有 ${hit.rel}，请求却拿到 HTTP ${r.status()}`)
})
page.on('console', (m) => {
  if (m.type() !== 'error') return
  const at = m.location?.()?.url ?? ''
  if (benign(at)) return
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

/** 详情卡上那张预览图（有就等它 settle，然后报 naturalWidth） */
const previewNow = () =>
  page.evaluate(() => {
    const el = document.querySelector('[class*="shellBody"] img[src*="/assets/"]')
    if (el === null) return null
    return { src: el.getAttribute('src') ?? '', loaded: el.naturalWidth > 0 && el.naturalHeight > 0 }
  })

if (wantShots) await mkdir(shotDir, { recursive: true })

await page.goto(url, { waitUntil: 'load' })
const shell = page.locator('[class*="shellBody"]')
await shell.first().waitFor({ timeout: 15000 })

/* —— 进资产库 —— */
await page.locator('button[title="资产库"]').first().click()
const arrived = await until(() => page.locator('button[id^="t-asset-"]').count().then((n) => n > 0))
say(arrived, '点「资产库」进得去，行是齐的')
if (!arrived) {
  problems.push('资产库页进不去 / 一行都没有')
  console.log('-'.repeat(96))
  await browser.close()
  for (const p of problems) console.log(`  - ${p}`)
  process.exit(1)
}

/* —— 逐行走一遍：谁渲染了预览图，谁就得画出来 —— */
const rowIds = await page.evaluate(() =>
  [...document.querySelectorAll('button[id^="t-asset-"]')].map((b) => b.id),
)
let previews = 0
const broken = []
for (const id of rowIds) {
  await page.locator(`[id="${id}"]`).first().click()
  await until(async () => (await previewNow()) !== null, 800)
  const shot = await until(async () => {
    const p = await previewNow()
    if (p === null) return true // 这一类没有预览图（MKP 预设 / 切片器配置 / 模型），正常
    return p.loaded || p.src === '' // 画出来了，或者 src 是空的（不归这条判）
  }, 3000)
  const p = await previewNow()
  if (p === null) continue
  previews += 1
  if (!shot || !p.loaded) broken.push(`${id} → ${p.src}`)
}
say(
  previews > 0 && broken.length === 0,
  `资产库里渲染出预览图的 ${previews} 行，全部画出来了（naturalWidth > 0）`,
  broken.length === 0 ? '' : broken.join(' · '),
)
if (previews === 0) problems.push('资产库一行预览图都没渲染（image / icon 这两类都不见了？）')
if (broken.length > 0) problems.push(`${broken.length} 行预览没画出来（破图）`)

/* —— 结算 `/assets/` 请求那一档 —— */
const bad = [...realResponses.entries()].filter(([, s]) => s < 200 || s >= 300)
say(
  bad.length === 0,
  `源里真有的资产：${realResponses.size} 个 URL 请求过，全部 2xx`,
  bad.length === 0 ? '' : bad.map(([rel, s]) => `${rel}→${s}`).join(' · '),
)

if (wantShots) await page.screenshot({ path: `${shotDir}/wb-asset-preview.png` })

console.log('-'.repeat(96))
await browser.close()

if (problems.length > 0) {
  console.log(`发现问题 ${problems.length} 条：`)
  for (const p of problems) console.log(`  - ${p}`)
  process.exit(1)
}
console.log(
  `资产库预览通：${previews} 行预览全部画出来，源里真有的 ${realResponses.size} 个 URL 全部 2xx，控制台没有 error`,
)
