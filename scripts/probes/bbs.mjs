/*
 * P5 探针：工艺页（BBS 预设查看器）—— 两档尺寸。
 *
 * 用法（先 `npm run build`，再让静态预览在 4173 上跑着）：
 *   npm run build
 *   npx vite preview --port 4173 --strictPort
 *   node scripts/probes/bbs.mjs [url]
 *
 * # 这一版验的是哪条线（2026-10-13 改）
 *
 * 数据源换了：那一档不再是 serve 期的 dev 端点（`/api/bbs/presets`，已退役），
 * 而是走 **IPC** —— 真机上是 `src-tauri/src/ipc/bbs.rs` 现读本机 BBS 目录，
 * **浏览器预览里是 mock 给的演示集合**（`src/api/mockServer/bbsLocal.ts`，6 份）。
 * 所以这一版断言的"份数"是**演示集合的份数**，不是你这台机器上的份数 ——
 * 「本机目录读出来的对不对」那一半由 Rust 侧的判据管（`ipc::bbs` 的五个测试：
 * 目录口径、抽象层、坏文件、三个读闸、scope/uid 校验）。
 * 由此，"没装 BBS"那一档（诚实空态）在浏览器里**造不出来**了（演示集合恒 available），
 * 它现在只在真机（没装 BBS 的机器）上看得到 —— 这一条从探针里撤了。
 *
 * 走一遍真实的路（两档尺寸：Ultra 1760×900 / Compact 900×640）：
 *   ① 顶栏「工艺」→ 页面装配起来（不是白屏、不是空屏）
 *   ② 左抽屉最上面是两枚来源胶囊，默认停在 «MKP 工艺»
 *   ③ MKP 档（默认档）：列出我们发的那几份（预览里读得出一份的正文）
 *      + 参数区真画出行 + 详情栏「改动」是算出来的数
 *   ④ 详情栏那颗叉与页签行右端的图标**同一个位置换面孔**：开着没有图标
 *      → 收起后图标出现 → 点它还能开回来
 *   ⑤ 切到 «本机工艺»：列出演示集合（用户配置 / 系统配置），机型下拉第一项是「全部（N）」
 *   ⑥ 本机工艺档里点一份用户预设：参数区有行、详情栏「改动」是 `N / M 项`、继承链 ≥ 2 节
 *   ⑦ 点一份系统预设：它自己就是基准 ⇒ 「改动」是「—（系统预设就是基准）」
 *   ⑧ 按「重扫本机」：清单还在、没有报错
 *
 * 判定只做「看得见」的那几条 + 0 console error / 0 个 >=400。
 */
import { mkdir } from 'node:fs/promises'

import { chromium } from 'playwright-core'

const url = process.argv[2]?.startsWith('http') ? process.argv[2] : 'http://localhost:4173/'

const SIZES = [
  { tag: 'ultra', width: 1760, height: 900 },
  { tag: 'compact', width: 900, height: 640 },
]

/** 演示集合（`mockServer/bbsLocal.ts`）里的份数：可选的 3 份系统 + 1 份用户 */
const DEMO_SYSTEM = 3
const DEMO_USER = 1

const shotDir = 'tmp-shots'
await mkdir(shotDir, { recursive: true })

const problems = []
const results = []
const check = (tag, what, ok, detail = '') => {
  results.push({ tag, what, ok, detail })
  if (!ok) problems.push(`${tag} ${what}${detail ? ` —— ${detail}` : ''}`)
}

/* 已知且无害：index.html 没写 favicon，浏览器自己会去要一次 */
const BENIGN_URL = [/\/favicon\.ico$/]
const benign = (url) => BENIGN_URL.some((re) => re.test(url))

/*
 * 已知且无害的一句话：`checkRemoteUpdate` 在浏览器里**如实答"未实现"**
 * （预览没有远端目录，那件事是桌面版启动时自动做的）—— 它以 console.error 冒出来，
 * 与这一页无关。**只放行这一句**：别的 console.error 一律算问题。
 */
const BENIGN_TEXT = [/未实现的接口: checkRemoteUpdate/]

/** 那一页那一刻的可读事实。选择器只认 data-* / role / aria —— 不随 class 哈希变 */
const facts = (page) =>
  page.evaluate(() => {
    const flat = (s) => (s ?? '').replace(/\s+/g, ' ').trim()
    const list = document.querySelector('[data-role="preset-list"]')
    const items = list ? [...list.querySelectorAll('button')] : []
    const tabs = document.querySelector('[role="tablist"][aria-label="预设来源"]')
    const listTabs = document.querySelector('nav[aria-label="参数分页"]')
    const rail = document.querySelector('aside[aria-label="详情"]')
    return {
      main: flat(document.querySelector('main')?.innerText),
      hasDrawer: document.querySelector('aside[aria-label="预设列表"]') !== null,
      drawer: flat(document.querySelector('aside[aria-label="预设列表"]')?.innerText),
      listText: flat(list?.innerText),
      items: items.length,
      onFiles: items
        .filter((b) => b.getAttribute('data-on') === 'true')
        .map((b) => b.getAttribute('title') ?? ''),
      groups: list
        ? [...list.querySelectorAll('div')].map((d) => flat(d.textContent)).filter((t) => /（\d+）$/.test(t))
        : [],
      empty: flat(list?.querySelector('p')?.textContent),
      sourceTab: flat(tabs?.querySelector('[role="tab"][aria-selected="true"]')?.textContent),
      sourceTabs: tabs ? tabs.querySelectorAll('[role="tab"]').length : 0,
      hasRail: rail !== null,
      railText: flat(rail?.innerText),
      hasRailIcon: document.querySelector('button[aria-label="详情"]') !== null,
      rows: document.querySelectorAll('[data-key]').length,
      params: flat(document.querySelector('[data-role="params"]')?.innerText),
      paramTabs: listTabs ? listTabs.querySelectorAll('button').length : 0,
    }
  })

/** 机型下拉的全部选项（它是个自绘控件：要打开才看得到选项） */
async function targetOptions(page) {
  await page.locator('button[aria-label="按机型与喷嘴筛选"]').click()
  const texts = await page
    .locator('ul[role="listbox"][aria-label="按机型与喷嘴筛选"] [role="option"]')
    .allInnerTexts()
  await page.keyboard.press('Escape')
  return texts.map((t) => t.replace(/\s+/g, ' ').trim())
}

/** 选机型下拉里的某一项 */
async function chooseTargetOption(page, text) {
  await page.locator('button[aria-label="按机型与喷嘴筛选"]').click()
  await page
    .locator('ul[role="listbox"][aria-label="按机型与喷嘴筛选"] [role="option"]', { hasText: text })
    .first()
    .click()
  await page.waitForTimeout(400)
}

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
  page.on('requestfailed', (r) => {
    if (!benign(r.url())) problems.push(`${tag} requestfailed ${r.failure()?.errorText ?? ''} ${r.url()}`)
  })

  await page.goto(url, { waitUntil: 'load' })
  await page.waitForSelector('header nav', { timeout: 10000 })

  /* ---------- ① 顶栏进工艺页 ---------- */
  await page.getByRole('button', { name: '工艺', exact: true }).first().click()
  await page.waitForSelector('[data-role="params"]', { timeout: 10000 })
  await page.waitForSelector('[data-role="preset-list"]', { timeout: 10000 })
  await page.waitForFunction(
    () => !document.querySelector('[data-role="preset-list"]')?.innerText?.includes('正在读清单'),
    { timeout: 15000 },
  )
  await page.waitForTimeout(400)
  let f = await facts(page)

  check(
    tag,
    '① 工艺页装配起来了（左抽屉 + 参数区 + 详情栏都在）',
    f.hasDrawer && f.hasRail && f.main.length > 0,
    `抽屉 ${f.hasDrawer} / 详情栏 ${f.hasRail}`,
  )
  check(
    tag,
    '① 没有走成"读不到本机目录"那一档（演示集合是有的）',
    !f.params.includes('没读到本机 BBS 预设目录'),
    f.params.slice(0, 50),
  )

  /* ---------- ② 两枚来源胶囊，默认停在 MKP ---------- */
  check(tag, '② 抽屉最上面是两枚来源胶囊', f.sourceTabs === 2, `数到 ${f.sourceTabs} 枚`)
  check(tag, '② 默认停在 «MKP 工艺»', f.sourceTab === 'MKP 工艺', `当前「${f.sourceTab}」`)

  /* ---------- ③ MKP 档：列出我们发的那几份 ---------- */
  check(
    tag,
    '③ MKP 档列出了我们发的那几份',
    f.groups.some((g) => g === `MKP 工艺（${f.items}）`) && f.items >= 1,
    `列表 ${f.groups.join(' | ')}（${f.items} 项）`,
  )
  check(
    tag,
    '③ 参数区真画出了行（不是空壳）',
    f.rows > 0,
    `行 ${f.rows} / ${f.params.slice(0, 40)}`,
  )
  check(
    tag,
    '③ 详情栏「改动」是算出来的数（不是"还没选预设"）',
    /改动\s*\d+\s*\/\s*\d+\s*项/.test(f.railText),
    f.railText.replace(/\s+/g, ' ').slice(0, 90),
  )
  await page.screenshot({ path: `${shotDir}/bbs-${tag}.png` })

  /* ---------- ④ 详情栏那颗叉 ↔ 页签行右端那颗图标 ---------- */
  check(tag, '④ 详情栏开着时，工具栏那颗「详情」图标不画', f.hasRailIcon === false)
  await page.locator('button[aria-label="收起详情"]').click()
  await page.waitForSelector('aside[aria-label="详情"]', { state: 'detached', timeout: 5000 })
  const closed = await facts(page)
  check(tag, '④ 收起之后图标回到页签行右端', closed.hasRailIcon === true && !closed.hasRail)
  await page.locator('button[aria-label="详情"]').click()
  await page.waitForSelector('aside[aria-label="详情"]', { timeout: 5000 })
  check(tag, '④ 点它还能再开回来', (await facts(page)).hasRail)

  /* ---------- ⑤ 切到本机工艺：演示集合 ---------- */
  await page.locator('[role="tablist"][aria-label="预设来源"] [role="tab"]', { hasText: '本机工艺' }).click()
  await page.waitForTimeout(500)
  f = await facts(page)
  check(
    tag,
    '⑤ 本机工艺档列出了演示集合里的用户那一份',
    f.groups.some((g) => g === `用户配置（${DEMO_USER}）`),
    f.groups.join(' | '),
  )
  const all = (await targetOptions(page))[0]
  check(
    tag,
    '⑤ 机型下拉第一项是「全部（可选份数）」',
    all === `全部（${DEMO_USER + DEMO_SYSTEM}）`,
    `读到「${all}」`,
  )
  /*
   * 机型那一档默认跟着**当前选中的那一份**走（MKP 档默认选中 A1 那台）——
   * 所以刚切过来时系统那组只列 A1 的。换成「全部」之后三台机的都该在。
   */
  await chooseTargetOption(page, '全部（')
  f = await facts(page)
  check(
    tag,
    '⑤ 切到「全部」之后，系统那组三台机的档都在',
    f.groups.some((g) => g === `系统配置（${DEMO_SYSTEM}）`),
    f.groups.join(' | '),
  )
  await page.screenshot({ path: `${shotDir}/bbs-${tag}-local.png` })

  /* ---------- ⑥ 用户预设：改动是算出来的、继承链走得通 ---------- */
  const userGroup = page.locator('[data-role="preset-list"] > div', { hasText: '用户配置' })
  await userGroup.locator('button').first().click()
  await page.waitForTimeout(600)
  f = await facts(page)
  check(tag, '⑥ 用户预设的改动算出来了（N / M 项）', /改动\s*\d+\s*\/\s*\d+\s*项/.test(f.railText), f.railText.replace(/\s+/g, ' ').slice(0, 90))
  check(tag, '⑥ 继承链 ≥ 2 节（父档查得到）', (f.railText.match(/→/g) ?? []).length >= 1, f.railText.replace(/\s+/g, ' ').slice(0, 120))
  check(tag, '⑥ 参数区有行', f.rows > 0, `行 ${f.rows}`)

  /* ---------- ⑦ 系统预设自己就是基准 ---------- */
  const sysGroup = page.locator('[data-role="preset-list"] > div', { hasText: '系统配置' })
  const sysCount = await sysGroup.locator('button').count()
  if (sysCount > 0) {
    /* 浮层态下点一份就自动收起抽屉，所以再点一次来源胶囊把它开回来 */
    if (!(await page.locator('[data-role="preset-list"]').isVisible())) {
      await page.locator('button[aria-label="预设列表"]').click()
      await page.waitForTimeout(300)
    }
    await sysGroup.locator('button').first().click()
    await page.waitForTimeout(600)
    const g = await facts(page)
    check(
      tag,
      '⑦ 系统预设的「改动」是「—（系统预设就是基准）」',
      g.railText.includes('系统预设就是基准'),
      g.railText.replace(/\s+/g, ' ').slice(0, 90),
    )
    check(tag, '⑦ 参数区照样有行', g.rows > 0, `行 ${g.rows}`)
  } else {
    problems.push(`${tag} 系统配置组里一份都没有（演示集合该有 ${DEMO_SYSTEM} 份）`)
  }

  /* ---------- ⑧ 重扫本机 ---------- */
  if (!(await page.locator('[data-role="preset-list"]').isVisible())) {
    await page.locator('button[aria-label="预设列表"]').click()
    await page.waitForTimeout(300)
  }
  const before = (await facts(page)).items
  await page.locator('button[aria-label="重扫本机"]').click()
  await page.waitForTimeout(1200)
  const after = await facts(page)
  check(
    tag,
    '⑧ 重扫之后清单还是那几份（读的是同一份本机目录）',
    after.items === before && after.items > 0,
    `重扫前 ${before} / 重扫后 ${after.items}`,
  )

  console.log(
    `${tag.padEnd(8)} 来源「${f.sourceTab}」 抽屉项 ${String(f.items).padEnd(3)} 参数行 ${String(f.rows).padEnd(4)}`
    + ` 分组 ${f.groups.join(' / ')}`,
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
console.log('工艺页两档尺寸全绿：0 console error / 0 个 >=400')
