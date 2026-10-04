/*
 * P6 探针：**工作台发布这一端**（生成 → 报出真实产物）＋ 客户端那一端的边界。
 *
 * # 这条链现在的真实形态（2026-10-04 第二次重写）
 *
 * 原版走的是「工作台生成 → 上传到云端 → 客户端同步 → 下载 → 应用」五步，两个入口同源、
 * 共用一格 `localStorage`。**这一整条在 2026-10-04 随 `ClientDataPackage` 退役消失**：
 *
 *   · 「上传到云端」那个按钮与 `mkp.cloud.presets` 那一格**已经不存在** —— 模拟云端是一份
 *     **假的发布契约**（真契约是 `catalog.path` + Git，见 `docs/PUBLISH-ARCHITECTURE.md`）；
 *   · 客户端的底账在 C4 收口时整体搬进了 Internal 根（catalog / `mkp/` / `run/`），
 *     真机的同步 / 下载走**真数据源**（HTTP + 真盘）；浏览器里的假后端没有盘、也没有源，
 *     下载会如实抛「浏览器里没有下载区」—— 那是对的，不是 bug。
 *
 * 所以这一份现在守两头：
 *
 *   工作台端   ① 勾「待生成」→ 生成 → ② 卡的产物名单报出**真产物份数**
 *              ② 设置页「官方源（Bootstrap）」收得下仓库地址 + 「重新读取」能看清磁盘真相
 *   客户端端   ③ 设置页的数据源那一格如实说「还没配置」（浏览器里没有源，也不假装联动）
 *
 * 客户端那半条链（下载 → 应用）**在浏览器里不再覆盖**，覆盖搬到了：
 *   · `params-settings.mjs`  参数页照目录渲染、设置页「预设数据源」那一格全流程能走
 *   · `presets.mjs`          下载如实拒、应用用户文件、「已应用」状态
 *   · 真机那条链（工作台发布 `presets/dist` → 数据源地址 → 客户端下载 → 应用）
 *     只在真机上跑得通 —— 归 `docs/PROJECT-AUDIT.md` ⑧「云端交付最终验收」的清单。
 *
 * **上次的教训也写在这儿**（同一处栽过两次）：原版断言读 `mkp.a40.package` / `active`、
 * 后来读 `mkp.cloud.presets` —— 都是**已经退役的结构**，于是那些条目恒 FAIL，
 * 谁也不再点开看它。**读退役结构的探针比没有探针更糟：它红着，于是没人再看它。**
 * 所以退役一个结构时，**必须同时裁掉读它的那一段探针**（本次就是这么做的）。
 *
 * # 为什么打的是 development + esnext 那一份（而不是 `npm run build:workbench` 的产物）
 *
 * 与 `scripts/probes/workbench-build.mjs` 同一条理由（那文件头里写全了）：开发桩只在
 * `import.meta.env.DEV` **且 URL 带 `?mock=1`** 时装（2026-10-02 起 —— 真机不再自动装桩），
 * 生产产物里既没有 Tauri IPC 也没有桩，整页读不到后端数据 —— 那时量的不是页面，是白屏。
 * 本探针打开工作台时**自己带上 `?mock=1`**。所以浏览器验收分两步：生产那一份报体积（闸门），
 * **带了桩的那一份**单独打给探针用：
 *
 *   npm run build:workbench                                                    # 生产产物（闸门）
 *   NODE_ENV=development BUILD_WORKBENCH=1 npx vite build --target esnext \
 *     --outDir node_modules/.cache/wb-probe/dist --emptyOutDir
 *   npx vite preview --outDir node_modules/.cache/wb-probe/dist --port 4174 --strictPort
 *   node scripts/probes/chain.mjs [http://localhost:4174]
 *
 * 两个入口**同源**（`index.html` 与 `workbench.html` 都挂在 4174 上）。
 * 探针自己不起服务，**用完把 4174 那台关掉**。
 *
 * 每档尺寸一个全新 context（localStorage 空的 = 还没传过、还没同步过）。
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

/** 等一个条件成立（后端是内存桩，一般是一两帧的事） */
async function until(fn, ms = 5000) {
  const t0 = Date.now()
  for (;;) {
    if (await fn()) return true
    if (Date.now() - t0 > ms) return false
    await new Promise((r) => setTimeout(r, 100))
  }
}

/** 把两页的控制台 / 网络都盯上 —— 这条链跨两个页面，哪一边红都算失败 */
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

/* ---------- 〇. 不带 `?mock=1` → **不许假装能跑**（2026-10-02 定的口径） ---------- */
/*
 * 这条守的是"桩不是产品运行时能力"：正常人（或 `tauri:workbench:dev` 的浏览器降级）
 * 打开 workbench.html 时没有 Tauri IPC，此时**必须如实失败**，不许装上一层夹具
 * 让人以为"能跑、数据是真的"。以前是"dev 且没 Tauri 就自动装"，于是浏览器里打开
 * 看着能用、实际读的是手写数据 —— "改了配置界面没变"就是这么来的。
 *
 * 判据很硬：不带 mock 时，工作台**不该**出现带夹具数据的一级导航（那是桩装上了的证据）。
 */
{
  const bare = await browser.newContext({ viewport: { width: 1280, height: 800 } })
  const p = await bare.newPage()
  await p.goto(`${base}/workbench.html`, { waitUntil: 'load' })
  await p.waitForTimeout(1500)
  const t = await p.evaluate(() => document.body.innerText.replace(/\s+/g, ' '))
  /*
   * 判据不能是"导航在不在"——外壳是静态的，没有后端也照渲染（实测如此）。
   * 真正的证据是**数据**：桩特有的那份夹具值（假路径 `C:\dev\presets`）**不该出现**，
   * 而页面里该出现「读取中」那种如实空态。
   */
  const looksLikeMock = t.includes('C:\\dev\\presets')
  const honest = t.includes('读取中') || t.includes('正在读')
  check(
    'nomock',
    '〇 不带 ?mock=1 时工作台**不装桩**（没有夹具数据，只剩如实空态）',
    !looksLikeMock && honest,
    looksLikeMock ? '出现了桩的夹具值（C:\\dev\\presets）—— 桩仍在自动装' : t.slice(0, 60),
  )
  await bare.close()
}

for (const size of SIZES) {
  const tag = size.tag
  const ctx = await browser.newContext({ viewport: { width: size.width, height: size.height } })

  /* ---------- ① 工作台：生成 ---------- */
  const wbPage = await ctx.newPage()
  wire(tag, wbPage)
  /* `?mock=1`：开发桩不再自动装（2026-10-02 起 —— 真机永远走真 Tauri IPC；
     桩是**探针的测试后端**，得自己显式要。见 src/workbench/main.tsx） */
  await wbPage.goto(`${base}/workbench.html?mock=1`, { waitUntil: 'load' })
  await wbPage.waitForSelector('nav[aria-label="一级导航"]', { timeout: 15000 })
  await wbPage.locator('button[title="生成与发布"]').first().click()
  const wbReady = await until(async () => (await wbText(wbPage)).includes('② 发布'))
  if (!wbReady) problems.push(`${tag} 工作台没进「生成与发布」`)

  /* 生成前先记下产物名单 —— 生成之后它必须跟着长（② 卡报的就是真产物份数） */
  const before = Number(/preset\.toml × (\d+) 份/.exec(flat(await wbText(wbPage)))?.[1] ?? '0')
  await wbPage.getByRole('button', { name: '全选待生成', exact: true }).first().click()
  await wbPage.getByRole('button', { name: /^生成 \d+ 项$/ }).first().click()
  /* 生成不写盘到确认之后：过一道「生成前确认」框（点确认 → 结果页 → 完成） */
  await wbPage.waitForSelector('[role="dialog"]', { timeout: 5000 })
  await wbPage.getByRole('button', { name: '确认生成', exact: true }).first().click()
  await until(async () =>
    (
      await wbPage.evaluate(() => document.querySelector('[role="dialog"]')?.innerText ?? '')
    ).includes('生成完成'),
  )
  await wbPage.getByRole('button', { name: '完成', exact: true }).first().click()
  await wbPage.waitForSelector('[role="dialog"]', { state: 'detached', timeout: 5000 })

  const built = await until(async () => {
    const n = Number(/preset\.toml × (\d+) 份/.exec(flat(await wbText(wbPage)))?.[1] ?? '0')
    return n > before
  })
  const builtCount = Number(/preset\.toml × (\d+) 份/.exec(flat(await wbText(wbPage)))?.[1] ?? '0')
  check(
    tag,
    '① 工作台：生成之后产物名单跟着长（② 卡报的是真产物份数）',
    built && builtCount > before,
    `生成前 ${before} 份 → 生成后 ${builtCount} 份`,
  )
  await wbPage.screenshot({ path: `${shotDir}/chain-${tag}-workbench-build.png` })

  /* ---------- ② 工作台「设置」页：官方源（Bootstrap）那一格 ---------- */
  /*
   * 守第十七/十八刀定的**输入契约**：用户只表达"我有一个仓库"，其余是系统的事。
   *   ① 那格在、提示语说清"填仓库地址就够"（不是只认 blob 页）；
   *   ② 仓库地址**能被接受并保存**（浏览器桩不做 GitHub → raw 规范化 —— 那是真后端
   *      `dist::normalize_bootstrap_url` 的活，它有 3 组单元测试钉着；这里只验 UI 收得下）；
   *   ③ 空地址**存不进去**（按钮压住 / 如实拒）。
   */
  await wbPage.getByRole('button', { name: '设置', exact: true }).first().click()
  const setReady = await until(async () => (await wbText(wbPage)).includes('官方源（Bootstrap）'), 8000)
  const wbSetText = flat(await wbText(wbPage))
  check(
    tag,
    '② 设置页有「官方源（Bootstrap）」，且说明"填仓库地址就够"',
    setReady && wbSetText.includes('填仓库地址就够'),
    wbSetText.slice(0, 80),
  )
  const urlInput = wbPage.getByLabel('官方源（Bootstrap）地址')
  await urlInput.fill('https://github.com/MuCoreBenC/MKPSupportEase.git')
  /* 「保存」有两个：外壳那颗（草稿保存，此刻禁用）+ 这一格自己那颗。
     用输入框的**后续兄弟**定位，别用 .first() —— 那会点到外壳那颗上。 */
  await wbPage.locator('input[aria-label="官方源（Bootstrap）地址"] ~ button').first().click()
  const saved = await until(async () => (await wbText(wbPage)).includes('已保存：'), 5000)
  check(
    tag,
    '② 仓库 .git 克隆地址能被接受并保存（UI 收得下；规范化是真后端的活）',
    saved,
    '',
  )

  /*
   * ② 「重新读取」= 从磁盘重读（`wb.reload()`）。
   * 先在输入框里敲一段**没保存**的垃圾，再点重新读取 —— 它该被磁盘那份**覆盖掉**，
   * 这一条量的正是"看到的是磁盘真相，不是我敲进去的东西"。
   */
  await urlInput.fill('https://example.com/not-saved')
  await wbPage.getByRole('button', { name: '重新读取', exact: true }).first().click()
  const reread = await until(async () => (await wbText(wbPage)).includes('已从磁盘重读'), 5000)
  const afterReread = await urlInput.inputValue()
  check(
    tag,
    '② 「重新读取」把输入框里没保存的内容清掉、换回磁盘那份（看真相）',
    reread && afterReread !== 'https://example.com/not-saved',
    `重读后输入框=${afterReread || '(空)'}`,
  )
  await wbPage.screenshot({ path: `${shotDir}/chain-${tag}-workbench-settings.png` })

  /* ---------- ③ 客户端：那一端的边界（浏览器里不再读工作台那一格） ---------- */
  const appPage = await ctx.newPage()
  wire(tag, appPage)
  await appPage.goto(`${base}/index.html`, { waitUntil: 'load' })
  await appPage.waitForSelector('header nav', { timeout: 15000 })
  await appPage.getByRole('button', { name: '设置', exact: true }).first().click()
  await until(async () => (await appText(appPage)).includes('预设数据源'), 8000)
  const setText = flat(await appText(appPage))
  /*
   * 客户端的底账现在是 catalog（随安装包走、程序管版本）；数据源那一格只答"下载去哪拿"
   * —— 真实来源由构建注入（Bootstrap 那一刀），今天浏览器里如实说"没配"。
   * **不再断言**「同步下来的是工作台刚发的那一份」：那是 C4 之前的结构
   * （客户端读 `mkp.cloud.presets`），现在客户端与那一格没有关系。
   * （2026-10-02 起客户端那一头的落点是**设置页** —— 「同步」页整页退役。）
   */
  check(
    tag,
    '③ 设置页的数据源那一格如实（浏览器里「还没配置」；不假装与工作台那一格联动）',
    setText.includes('预设数据源') && setText.includes('还没配置'),
    setText.slice(0, 80),
  )
  await appPage.screenshot({ path: `${shotDir}/chain-${tag}-settings.png` })

  console.log(
    `${tag.padEnd(8)} 工作台产物 ${before} → ${builtCount} 份（② 卡真产物份数）· `
    + `设置页官方源收得下 / 重新读取看真相 · 客户端边界如实`,
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
console.log(
  '工作台发布这一端走通：生成 → ② 卡报出真产物份数；客户端那一端如实（数据源那一格说真话）；'
  + '桩只在 ?mock=1 时装（不带就如实失败）、设置页「官方源」收得下 / 「重新读取」能从磁盘看真相'
  + ' —— 两档尺寸 0 console error / 0 个 >=400',
)
