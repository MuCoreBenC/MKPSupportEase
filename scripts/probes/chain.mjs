/*
 * P6 探针：**联动端到端** —— 工作台生成 → 上传云端 → 客户端同步 → 下载 → 应用。
 *
 * # 为什么打的是 development + esnext 那一份（而不是 `npm run build:workbench` 的产物）
 *
 * 与 `scripts/probes/workbench-build.mjs` 同一条理由（那文件头里写全了）：开发桩只在
 * `import.meta.env.DEV` 时装，生产产物里既没有 Tauri IPC 也没有桩，整页读不到后端数据 ——
 * 那时量的不是页面，是白屏。所以浏览器验收分两步：生产那一份报体积（闸门），
 * **带了桩的那一份**单独打给探针用：
 *
 *   npm run build:workbench                                                    # 生产产物（闸门）
 *   $env:NODE_ENV='development'; $env:BUILD_WORKBENCH='1'
 *   npx vite build --target esnext --outDir node_modules/.cache/wb-probe/dist --emptyOutDir
 *   npx vite preview --outDir node_modules/.cache/wb-probe/dist --port 4174 --strictPort
 *   node scripts/probes/chain.mjs [http://localhost:4174]
 *
 * 两个入口**同源**（`index.html` 与 `workbench.html` 都挂在 4174 上），所以共用一格
 * `localStorage` —— 那正是这条链的物理形态（真实世界是同一台机器上的两个窗口）。
 * 探针自己不起服务，**用完把 4174 那台关掉**。
 *
 * 每档尺寸一个全新 context（localStorage 空的 = 还没传过、还没同步过）：
 *   ① 工作台：勾「待生成」→ 生成 → ② 卡里报的是真说明书（机型 / 版本 / 字段 / 带条件）
 *   ② 工作台：点「上传到云端」→ 那一格真写进去了，且是**整份 release**（说明书 + N 份 TOML）
 *   ③ 客户端「同步」页：自动同步 → 状态「刚刚同步到最新」，**报的数与工作台同一份**
 *   ④ 客户端「预设」页云端表：多出「工作台发布 · x.y.z」几行 → 点「下载」→ 变「已下载」
 *   ⑤ 客户端「预设」页本地表：那一行在 → 点「应用」→ 变「已应用」
 *
 * 两档尺寸：Ultra 1760×900 / Compact 900×640。
 */
import { mkdir } from 'node:fs/promises'

import { chromium } from 'playwright-core'

const base = (process.argv[2]?.startsWith('http') ? process.argv[2] : 'http://localhost:4174').replace(/\/$/, '')

const SIZES = [
  { tag: 'ultra', width: 1760, height: 900 },
  { tag: 'compact', width: 900, height: 640 },
]

const shotDir = 'tmp-shots'
await mkdir(shotDir, { recursive: true })

const problems = []
const results = []
const check = (tag, what, ok, detail = '') => {
  results.push({ tag, what, ok, detail })
  if (!ok) problems.push(`${tag} ${what}${detail ? ` —— ${detail}` : ''}`)
}

/* 已知且无害：index.html 没写 favicon，浏览器自己会去要一次 */
const BENIGN = [/\/favicon\.ico$/]
const benign = (text) => BENIGN.some((re) => re.test(text))

const flat = (s) => (s ?? '').replace(/\s+/g, ' ').trim()

/** 工作台的正文在 `.shellBody` 里（这一页没有 `<main>`：那是客户端顶栏那一套） */
const wbText = (page) => page.evaluate(() => document.querySelector('[class*="shellBody"]')?.innerText ?? '')
const appText = (page) => page.evaluate(() => (document.querySelector('main')?.innerText ?? ''))

/** 客户端本机那一份说明书的指纹（`STORAGE.clientPackage` 那一格） */
const hashOf = (page) =>
  page.evaluate(() => {
    const raw = localStorage.getItem('mkp.a40.package')
    if (raw === null) return null
    try {
      return JSON.parse(raw)?.package?.inputsHash ?? null
    } catch {
      return null
    }
  })

/** 等一个条件成立（后端是内存桩，一般是一两帧的事） */
async function until(fn, ms = 5000) {
  const t0 = Date.now()
  for (;;) {
    if (await fn()) return true
    if (Date.now() - t0 > ms) return false
    await new Promise((r) => setTimeout(r, 100))
  }
}

/** 把两页的控制台 / 网络都盯上 —— 这一条链跨两个页面，哪一边红都算失败 */
function wire(tag, page) {
  page.on('console', (m) => {
    if (m.type() !== 'error') return
    const at = m.location?.()?.url ?? ''
    if (benign(at)) return
    problems.push(`${tag} console.error: ${m.text()}${at ? ` @ ${at}` : ''}`)
  })
  page.on('pageerror', (e) => problems.push(`${tag} pageerror: ${e.message}`))
  page.on('response', (r) => {
    if (r.status() >= 400 && !benign(r.url())) problems.push(`${tag} HTTP ${r.status()} ${r.url()}`)
  })
  page.on('requestfailed', (r) => {
    if (!benign(r.url())) problems.push(`${tag} requestfailed ${r.failure()?.errorText ?? ''} ${r.url()}`)
  })
}

const browser = await chromium.launch({ channel: 'msedge' })

for (const size of SIZES) {
  const tag = size.tag
  const ctx = await browser.newContext({ viewport: { width: size.width, height: size.height } })

  /* ---------- ①② 工作台：生成 → 上传 ---------- */
  const wbPage = await ctx.newPage()
  wire(tag, wbPage)
  await wbPage.goto(`${base}/workbench.html`, { waitUntil: 'load' })
  await wbPage.waitForSelector('nav[aria-label="一级导航"]', { timeout: 15000 })
  await wbPage.locator('button[title="生成与发布"]').first().click()
  const wbReady = await until(async () => (await wbText(wbPage)).includes('② 客户端数据包'))
  if (!wbReady) problems.push(`${tag} 工作台没进「生成与发布」`)

  await wbPage.getByRole('button', { name: '全选待生成', exact: true }).first().click()
  await wbPage.getByRole('button', { name: /^生成 \d+ 项$/ }).first().click()
  await until(async () => /preset\.toml × \d+ 份/.test(await wbText(wbPage)))

  const wbNumbers = /(\d+) 台机型 (\d+) 个版本 (\d+) 个字段 带条件 (\d+) 可选文件 (\d+)/.exec(
    flat(await wbText(wbPage)),
  )
  check(
    tag,
    '① 工作台 ② 报的是真说明书（机型 / 版本 / 字段 / 带条件）',
    wbNumbers !== null,
    wbNumbers?.slice(1).join(' / ') ?? '没找到那几个数',
  )
  const [nMachines, nVersions, nFields, nConds] = wbNumbers?.slice(1, 5) ?? []

  await wbPage.getByRole('button', { name: '上传到云端', exact: true }).first().click()
  const wrote = await until(async () =>
    (await wbPage.evaluate(() => localStorage.getItem('mkp.cloud.presets')))?.includes('工作台发布') === true)
  const entry = await wbPage.evaluate(() => {
    const raw = localStorage.getItem('mkp.cloud.presets')
    if (raw === null) return null
    const list = JSON.parse(raw)
    const e = list[list.length - 1]
    return {
      name: e.name,
      version: e.version,
      at: e.at,
      presets: e.presets.length,
      files: e.presets.map((p) => `${p.machineId}/${p.versionId}=${p.fileName}`),
      fields: e.package.fields.length,
      machines: e.package.machines.length,
      hash: e.package.inputsHash,
    }
  })
  check(
    tag,
    '② 上传写进那一格，且是整份 release（说明书 + N 份 TOML）',
    wrote && entry !== null && entry.presets > 0 && entry.fields > 0,
    JSON.stringify(entry),
  )
  if (entry === null) {
    /* 后面几步全都要读这一份，没有它就没法继续 —— 报出来直接换档 */
    check(tag, '③ 客户端同步（前一步没数据，跳过）', false, '上传没成功')
    await ctx.close()
    continue
  }

  /* ---------- ③ 客户端：自动同步 ---------- */
  const appPage = await ctx.newPage()
  wire(tag, appPage)
  await appPage.goto(`${base}/index.html`, { waitUntil: 'load' })
  await appPage.waitForSelector('header nav', { timeout: 15000 })
  await appPage.getByRole('button', { name: '同步', exact: true }).first().click()
  const synced = await until(async () => {
    const t = await appText(appPage)
    return t.includes('刚刚同步到最新') || t.includes('已是最新')
  }, 8000)
  const syncText = flat(await appText(appPage))
  /*
   * 那一行状态在两种写法之间：**两次 effect** 的产物。
   * `main.tsx` 挂着 `<StrictMode>`，挂载期 effect 跑两遍 —— 第一遍真同步（写进本机那一格），
   * 第二遍再看已经是同一份指纹，于是报「已是最新」。两句都说明「本机与云端是同一份」，
   * 所以这里两个都收；真正硬的判据是下面那条**指纹相等**。
   */
  check(tag, '③ 客户端自动同步了（本机与云端是同一份）', synced, syncText.slice(0, 70))
  check(
    tag,
    '③ 同步下来那份说明书与工作台那份**同指纹**',
    (await hashOf(appPage)) === entry.hash,
    `客户端 ${await hashOf(appPage)} / 工作台 ${entry.hash}`,
  )
  check(
    tag,
    '③ 同步下来的是工作台刚发的那一份',
    syncText.includes(entry.name) && syncText.includes(`包版本 ${entry.version}`),
    `找「${entry.name}」`,
  )
  const pkgSummary = `${nMachines} 台机型 · ${nVersions} 个版本 · 说明书 ${nFields} 个字段（带条件 ${nConds}）`
  check(
    tag,
    '③ 客户端报的数与工作台是同一份',
    syncText.includes(pkgSummary),
    `要「${pkgSummary}」，页面上是「${/台机型[^·]*·[^·]*·[^·]*/.exec(syncText)?.[0] ?? '（没找到）'}」`,
  )
  await appPage.screenshot({ path: `${shotDir}/chain-${tag}-sync.png` })

  /* ---------- ④ 预设页 · 云端表：下载 ---------- */
  await appPage.getByRole('button', { name: '预设', exact: true }).first().click()
  await appPage.locator('main input[type="radio"][name="preset-kind"][value="mkp"]').click({ force: true })
  await appPage.waitForTimeout(200)
  await appPage.locator('main input[type="radio"][name="preset-scope"][value="cloud"]').click({ force: true })
  await appPage.waitForTimeout(300)

  const cloudRel = appPage.locator('main tbody tr').filter({ hasText: '工作台发布' })
  const relCount = await cloudRel.count()
  check(tag, '④ 云端表多出「工作台发布」几行', relCount >= 1, `行数 ${relCount}（上传了 ${entry.presets} 份）`)
  if (relCount >= 1) {
    const row0 = cloudRel.first()
    const rowText = flat(await row0.innerText())
    await row0.getByRole('button', { name: '下载', exact: true }).click()
    const downloaded = await until(async () => flat(await row0.innerText()).includes('已下载'), 8000)
    check(tag, '④ 点「下载」之后那一行变「已下载」', downloaded, `原样：「${rowText}」`)
    await appPage.screenshot({ path: `${shotDir}/chain-${tag}-cloud.png` })
  } else {
    check(tag, '④ 点「下载」之后那一行变「已下载」', false, '没有可下载的行')
  }

  /* ---------- ⑤ 预设页 · 本地表：应用 ---------- */
  await appPage.locator('main input[type="radio"][name="preset-scope"][value="local"]').click({ force: true })
  await appPage.waitForTimeout(300)
  const localRel = appPage.locator('main tbody tr').filter({ hasText: '工作台发布' })
  const localCount = await localRel.count()
  check(tag, '⑤ 下载之后本地表也有它了（下载 ≠ 使用，两格分开）', localCount >= 1, `行数 ${localCount}`)
  if (localCount >= 1) {
    const row0 = localRel.first()
    await row0.getByRole('button', { name: '应用', exact: true }).click()
    const applied = await until(async () => flat(await row0.innerText()).includes('已应用'), 8000)
    check(tag, '⑤ 点「应用」之后那一行变「已应用」', applied, flat(await row0.innerText()))
    const active = await appPage.evaluate(() => localStorage.getItem('mkp.a40.active'))
    check(
      tag,
      '⑤ 唯一底账（STORAGE.clientActive）里记的是这一份',
      active !== null && active.includes('release'),
      (active ?? 'null').slice(0, 90),
    )
    await appPage.screenshot({ path: `${shotDir}/chain-${tag}-applied.png` })
  } else {
    check(tag, '⑤ 点「应用」之后那一行变「已应用」', false, '本地表里没有那一行')
  }

  console.log(
    `${tag.padEnd(8)} 工作台 ${nMachines} 机型 / ${nVersions} 版本 / ${nFields} 字段 · `
    + `上传 ${entry.presets} 份 TOML（${entry.name}）· 客户端同步 → 下载 → 应用`,
  )
  await ctx.close()
}

await browser.close()

console.log('-'.repeat(80))
console.log(`${results.length} 条判定：`)
for (const r of results) {
  console.log(`  ${r.ok ? 'ok  ' : 'FAIL'} [${r.tag}] ${r.what}${r.detail ? `  （${r.detail}）` : ''}`)
}

if (problems.length > 0) {
  console.log(`发现问题 ${problems.length} 条：`)
  for (const p of problems) console.log(`  - ${p}`)
  process.exit(1)
}
console.log('联动一条链走通：工作台生成 → 上传 → 客户端同步 → 下载 → 应用（两档尺寸 0 console error / 0 个 >=400）')