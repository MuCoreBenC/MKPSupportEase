/*
 * P5 探针：BBS 预设页（改读本机 BBS 目录那一版）—— 两档尺寸。
 *
 * 用法（先 `npm run build`，再让静态预览在 4173 上跑着）：
 *   npm run build
 *   npx vite preview --port 4173 --strictPort
 *   node scripts/probes/bbs.mjs [url]
 *
 * **端点挂在 preview 上也要在**：`tools/dev-server/bbsFs.mjs` 同时实现了
 * `configureServer` 与 `configurePreviewServer` —— 因为这台机器上 `npm run dev` 会被
 * `target/` 那两棵构建树拖死文件监听，验收一律走 preview（阶段账债 #4）。
 *
 * 走一遍真实的路：
 *   ① 顶栏「BBS 预设」→ 页面装配起来（不是空态、不是白屏）
 *   ② 清单来自**本机 BBS 目录**：状态条报「本机 BBS · 用户 N / 系统 M」，系统那一档是 278 份那一级
 *   ③ 默认选中一份，参数面板**真画出行**（不是「这一档下没有可显示的参数」）
 *   ④ 五页分类页签在（只列有内容的那几页）
 *   ⑤ 跨页那一条：预设页（切片器 · 本机）右键「在 BBS 预设查看器中打开」→ 切到 BBS 且那一份真被选中
 *   ⑥ 把端点桩成「没有本机目录」（route 拦截）→ 点「重扫本机」→ **诚实空态**
 *      （本机装着 BBS，所以「没有目录」这一档只能这样可控地验，不能靠把 BBS 卸了）
 *
 * 判定只做「看得见」的那几条 + 0 console error / 0 个 >=400。两档尺寸：Ultra 1760×900 / Compact 900×640。
 */
import { mkdir } from 'node:fs/promises'

import { chromium } from 'playwright-core'

const url = process.argv[2]?.startsWith('http') ? process.argv[2] : 'http://localhost:4173/'

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

/** BBS 页那一刻的可读事实。选择器只认 data-* / role / aria —— 不随 class 哈希变 */
const facts = (page) =>
  page.evaluate(() => {
    const flat = (s) => (s ?? '').replace(/\s+/g, ' ').trim()
    const list = document.querySelector('[data-role="preset-list"]')
    const items = list ? [...list.querySelectorAll('button')] : []
    const on = items.filter((b) => b.getAttribute('data-on') === 'true')
    const tabs = document.querySelector('nav[aria-label="参数分页"]')
    return {
      main: flat(document.querySelector('main')?.innerText),
      footer: flat(document.querySelector('footer')?.innerText),
      listText: flat(list?.innerText),
      items: items.length,
      onFiles: on.map((b) => b.getAttribute('title') ?? ''),
      groups: list
        ? [...list.querySelectorAll('div')].map((d) => flat(d.textContent))
          .filter((t) => /（\d+）$/.test(t))
        : [],
      allOption: flat(document.querySelector('select[aria-label="按机型与喷嘴筛选"] option')?.textContent),
      rows: document.querySelectorAll('[data-key]').length,
      tabs: tabs ? tabs.querySelectorAll('button').length : 0,
      tabOn: flat(tabs?.querySelector('button[data-on="true"]')?.textContent),
      title: flat(document.querySelector('footer span')?.textContent),
      content: flat(document.querySelector('[data-role="params"]')?.innerText),
    }
  })

const num = (text, label) => {
  const m = text.match(new RegExp(`${label} (\\d+)`))
  return m ? Number(m[1]) : -1
}

const browser = await chromium.launch({ channel: 'msedge' })

for (const size of SIZES) {
  const tag = size.tag
  const ctx = await browser.newContext({ viewport: { width: size.width, height: size.height } })
  const page = await ctx.newPage()

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

  await page.goto(url, { waitUntil: 'load' })
  await page.waitForSelector('header nav', { timeout: 10000 })

  /* ---------- ① 顶栏进 BBS 页 ---------- */
  await page.getByRole('button', { name: 'BBS 预设', exact: true }).first().click()
  await page.waitForSelector('[data-role="params"]', { timeout: 10000 })
  await page.waitForFunction(
    () => !document.querySelector('[data-role="params"]')?.innerText?.includes('正在读预设清单'),
    { timeout: 15000 },
  )
  await page.waitForTimeout(400)
  const f = await facts(page)

  check(
    tag,
    '① BBS 页装配起来了（不是白屏 / 不是 fatal 空屏）',
    f.main.includes('预设列表') && f.main.includes('BBS '),
    f.main.slice(0, 70),
  )
  check(tag, '② 清单来自本机 BBS 目录', f.footer.includes('本机 BBS ·'), f.footer.slice(0, 90))

  const users = num(f.footer, '用户')
  const systems = num(f.footer, '系统')
  check(tag, '② 系统预设落在 278 份那一档', systems >= 200, `系统 ${systems}`)
  check(tag, '② 本机用户预设也在', users >= 1, `用户 ${users}`)

  check(
    tag,
    '③ 默认选中了一份（不是「—」）',
    f.title.replace(/^预设\s*/, '').length > 1 && !f.title.includes('—'),
    f.title,
  )
  check(
    tag,
    '③ 参数面板真画出了行',
    f.rows > 0 && !f.content.includes('没有可显示的参数') && !f.content.includes('没读到本机 BBS 预设目录'),
    `行 ${f.rows} / ${f.content.slice(0, 40)}`,
  )
  check(tag, '④ 分类页签在且当前页亮着', f.tabs >= 1 && f.tabOn.length > 0, `页签 ${f.tabs} 当前「${f.tabOn}」`)
  check(
    tag,
    '④ 抽屉里三档分组齐（用户 / 系统两组在）',
    f.listText.includes('用户配置（') && f.listText.includes('系统配置（'),
    f.groups.join(' | ').slice(0, 90),
  )

  await page.screenshot({ path: `${shotDir}/bbs-${tag}.png` })

  /* ---------- ⑤ 跨页：预设页右键 → 在 BBS 预设查看器中打开 ---------- */
  await page.getByRole('button', { name: '预设', exact: true }).first().click()
  await page.waitForSelector('main tbody tr', { timeout: 10000 })
  await page.locator('main input[type="radio"][name="preset-kind"][value="slicer"]').click({ force: true })
  await page.waitForTimeout(250)
  await page.locator('main input[type="radio"][name="preset-scope"][value="local"]').click({ force: true })
  await page.waitForTimeout(300)

  const row = page.locator('main tbody tr').filter({ has: page.locator('td:not([colspan])') }).first()
  const rowText = (await row.innerText()).replace(/\s+/g, ' ').trim()
  const wantFile = rowText.match(/[\w\u4e00-\u9fa5.\-@ ]+\.json/)?.[0]?.trim() ?? ''

  await row.click({ button: 'right' })
  await page.waitForTimeout(200)
  const item = page.getByRole('menuitem', { name: '在 BBS 预设查看器中打开' })
  const entries = await item.count()
  if (entries === 0) {
    problems.push(`${tag} 切片器本机行右键没有「在 BBS 预设查看器中打开」`)
  } else {
    await item.first().click()
    await page.waitForSelector('[data-role="params"]', { timeout: 10000 })
    await page.waitForTimeout(600)
    const after = await facts(page)
    const active = await page.evaluate(
      () => document.querySelector('header nav [aria-current="page"]')?.textContent?.trim() ?? '',
    )
    check(tag, '⑤ 跨页：切到了 BBS 预设', active === 'BBS 预设', `落在「${active}」`)
    /*
     * 两页的清单不是同一份：预设页那张表列的是 MKP 自己的 bbs_profile 资产，
     * BBS 页列的是你本机装在 Bambu Studio 里的那些。名字对得上就选中它，
     * 对不上要**如实说一句**再停在默认那一份 —— 两条都算这条线通了。
     */
    const selected = after.onFiles.join(' ')
    const carried = wantFile !== '' && selected.includes(wantFile)
    const explained = wantFile !== '' && after.main.includes(`清单里没有「${wantFile}」`)
    check(
      tag,
      '⑤ 带过去的目标被处理了（选中它，或如实说清没有它）',
      after.onFiles.length === 1 && (carried || explained),
      `想开「${wantFile}」/ 选中「${selected.slice(0, 60)}」/ ${carried ? '命中' : explained ? '已如实说明' : '既没命中也没说明'}`,
    )
  }

  /* ---------- ⑥ 端点桩成「没有本机目录」→ 诚实空态 ---------- */
  await page.route('**/api/bbs/presets', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json; charset=utf-8',
      body: JSON.stringify({ available: false, reason: '探针桩：模拟这台机器没装 BBS' }),
    }),
  )
  await page.getByRole('button', { name: '重扫本机' }).click()
  await page.waitForSelector('text=没读到本机 BBS 预设目录', { timeout: 10000 })
  await page.waitForTimeout(300)
  const none = await facts(page)
  check(
    tag,
    '⑥ 没有本机目录时是诚实空态（说清理由，不编数据）',
    none.content.includes('探针桩：模拟这台机器没装 BBS')
      && none.content.includes('本版不打包预设快照')
      && none.rows === 0,
    `行 ${none.rows} / ${none.content.slice(0, 60)}`,
  )
  check(tag, '⑥ 空态下不再假装是本机目录', none.footer.includes('本机 BBS 目录没读到'), none.footer.slice(0, 60))
  await page.screenshot({ path: `${shotDir}/bbs-${tag}-no-dir.png` })

  console.log(
    `${tag.padEnd(8)} 用户 ${String(users).padEnd(3)} 系统 ${String(systems).padEnd(4)} 抽屉项 ${String(f.items).padEnd(4)}`
    + ` 参数行 ${String(f.rows).padEnd(4)} 页签 ${f.tabs} 当前「${f.tabOn}」`,
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
console.log('BBS 页两档尺寸全绿：0 console error / 0 个 >=400')