/*
 * 预设页探针：这一页到底长得对不对、点得动点不动。
 *
 * 用法（先 `npm run build`，再让静态预览在 4173 上跑着）：
 *   npm run build
 *   npx vite preview --port 4173 --strictPort
 *   node scripts/probes/presets.mjs [url]
 *
 * 为什么不用 dev（5321）：那台 watcher 会去扫 `target/` 下 5.8 万个文件，
 * 自己把自己拖死 —— 验收一律走「build 之后 preview」这条静态路。
 *
 * 判断只做「看得见」的那几条：点一下有没有反应、表里有没有行、右键菜单出不出来、
 * 控制台有没有 error、有没有 >=400 的响应。**不去量行高与像素**（那是专用探针的事）。
 */
import { mkdir } from 'node:fs/promises'

import { chromium } from 'playwright-core'

const url = process.argv[2]?.startsWith('http') ? process.argv[2] : 'http://localhost:4173/'

const browser = await chromium.launch({ channel: 'msedge' })
const page = await browser.newPage({ viewport: { width: 1760, height: 900 } })

const problems = []
/*
 * 已知且无害：
 *   ① index.html 没写 favicon，浏览器自己会去要一次；
 *   ② 浏览器里点「下载 / 更新」**必然**抛「未实现的接口」（没有下载区、没有数据源）——
 *      那是设计成要报错的：探针自己在第 5 节点了它一次，页面把这条错误如实显示出来，
 *      正是要的结果（"点了说成功但盘上什么都没有"才是要抓的）；
 *   ③ 同一档还有「看正文」：归档里那份 / 用户自己那份的字节都要真的盘，浏览器里没有 ——
 *      页面照实说"读不出来"。
 */
const BENIGN = [
  /\/favicon\.ico$/,
  /未实现的接口: downloadCatalogFile/,
  /未实现的接口: readArchivedText/,
  /未实现的接口: readUserPresetText/,
]
const benign = (text) => BENIGN.some((re) => re.test(text))

page.on('console', (m) => {
  if (m.type() !== 'error') return
  const at = m.location?.()?.url ?? ''
  /* 白名单要**连正文一起看**：`② ` 那一条认的是消息本身，不是它从哪个文件抛出来的 */
  if (benign(at) || benign(m.text())) return
  problems.push(`console.error: ${m.text()}${at ? ` @ ${at}` : ''}`)
})
page.on('pageerror', (e) => problems.push(`pageerror: ${e.message}`))
page.on('response', (r) => {
  if (r.status() >= 400 && !benign(r.url())) problems.push(`HTTP ${r.status()} ${r.url()}`)
})
page.on('requestfailed', (r) => {
  if (!benign(r.url())) problems.push(`requestfailed ${r.failure()?.errorText ?? ''} ${r.url()}`)
})

await page.goto(url, { waitUntil: 'load' })
await page.waitForSelector('header', { timeout: 10000 })

/** 表格那一块的可读事实 */
const facts = () =>
  page.evaluate(() => {
    const table = document.querySelector('main table')
    const rows = [...document.querySelectorAll('main tbody tr')]
    /*
     * 展开排紧跟在数据行后面，它那一格是 `td[colspan]`、内容是一张 `<dl>`。
     * 「数据行」的判据因此是「带一个 td.name 的普通格」—— 这样展开排不会被算成一行
     * （第一版探针就是把它算进去了，compact 档量到 2 行，查了半天）。
     */
    const dataRows = rows.filter((r) => r.querySelector('td:not([colspan])') !== null)
    const heads = [...document.querySelectorAll('main thead th')].map((th) => th.textContent?.trim() ?? '')
    const main = document.querySelector('main')
    const axes = [...document.querySelectorAll('main [role="radiogroup"]')].map((g) =>
      [...g.querySelectorAll('label')]
        .filter((l) => l.getAttribute('data-on') === 'true')
        .map((l) => l.textContent?.trim() ?? '')
        .join(''),
    )
    return {
      text: (main?.innerText ?? '').replace(/\s+/g, ' ').trim(),
      heads,
      dataRows: dataRows.length,
      firstRow: dataRows[0]?.innerText.replace(/\s+/g, ' ').trim().slice(0, 110) ?? '',
      /* 每一行的第一格（文件名）+ 操作列那一格 —— 报告「这张表里到底是什么」用 */
      rows: dataRows.slice(0, 8).map((r) => {
        const tds = [...r.querySelectorAll('td')]
        const name = tds[0]?.innerText.replace(/\s+/g, ' ').trim().slice(0, 60) ?? ''
        const act = tds[tds.length - 1]?.innerText.replace(/\s+/g, ' ').trim() ?? ''
        return `${name} → ${act}`
      }),
      /* 工具条右端那两格：当前这张表的真计数 + 仓库台账 */
      counts: [...document.querySelectorAll('main span')]
        .map((s) => s.innerText.replace(/\s+/g, ' ').trim())
        .filter((t) => /^共 \d+ 项/.test(t) || /^仓库 \d+/.test(t))
        .join(' | '),
      axes,
      empty: document.querySelector('main table')?.parentElement?.parentElement?.innerText.includes('还没有')
        ? [...document.querySelectorAll('main p')].map((p) => p.innerText.trim()).find((t) => t.includes('还没有')) ?? ''
        : '',
      expanded: document.querySelectorAll('main tbody dl').length,
    }
  })

console.log(`url ${url}`)
const shotDir = 'tmp-shots'
await mkdir(shotDir, { recursive: true })

/* ---------- 1. 打开「预设」 ---------- */
await page.getByRole('button', { name: '预设', exact: true }).first().click()
await page.waitForTimeout(400)

let f = await facts()
console.log(`\n[预设页] 轴选中 ${JSON.stringify(f.axes)}`)
console.log(`表头 ${f.heads.join(' / ') || '(无)'}`)
console.log(`数据行 ${f.dataRows}  计数 ${f.counts}`)
console.log(`行内容 ${f.rows.join(' || ') || '(无)'}`)
console.log(`可见正文 ${f.text.slice(0, 260)}`)
if (f.dataRows === 0) problems.push('预设页打开后表格 0 行')
if (f.heads.length === 0) problems.push('预设页没有表头')
await page.screenshot({ path: `${shotDir}/presets-ultra.png` })

/* ---------- 2. 两条轴控件 ---------- */
const rad = (name, value) => page.locator(`main input[type="radio"][name="${name}"][value="${value}"]`)

/** 切一档、把这一档的可读事实打出来 */
const show = async (title, name, value) => {
  await rad(name, value).click({ force: true })
  await page.waitForTimeout(250)
  const v = await facts()
  console.log(`\n[${title}] 轴 ${JSON.stringify(v.axes)} 行 ${v.dataRows}  计数 ${v.counts}`)
  console.log(`表头 ${v.heads.join(' / ')}`)
  console.log(`行内容 ${v.rows.join(' || ') || '(无)'}`)
  if (v.empty !== '') console.log(`空态文案 ${v.empty}`)
  return v
}

await rad('preset-kind', 'mkp').click({ force: true })
await page.waitForTimeout(200)
const mkpLocal = await show('MKP 配置 / 本地', 'preset-scope', 'local')
const slicerLocal = await show('切片器配置 / 本地', 'preset-kind', 'slicer')
const slicerCloud = await show('切片器配置 / 云端', 'preset-scope', 'cloud')
await rad('preset-kind', 'mkp').click({ force: true })
await page.waitForTimeout(200)
const mkpCloud = await show('MKP 配置 / 云端', 'preset-kind', 'mkp')

if (mkpLocal.heads.join() === slicerLocal.heads.join()) {
  problems.push('MKP 与切片器的表头一样（列应该随类型变）')
}

/** 每一行的文件名 + 操作列那一格（按钮取按钮文字，灰字取文字） */
const actions = () =>
  page.evaluate(() => {
    const rows = [...document.querySelectorAll('main tbody tr')].filter(
      (r) => r.querySelector('td:not([colspan])') !== null,
    )
    return rows.map((r) => {
      const name = r.querySelector('td')?.innerText.replace(/\s+/g, ' ').trim() ?? ''
      const last = [...r.querySelectorAll('td')].pop()
      const btn = last?.querySelector('button')
      return {
        name,
        action: btn ? btn.innerText.trim() : (last?.innerText.replace(/\s+/g, ' ').trim() ?? ''),
      }
    })
  })

/* ---------- 2b. 分类边界：两档各认哪些（按 catalog 的 kind，不靠扩展名猜） ---------- */
/*
 * 守三件事：
 *   ① MKP 档只认 `mkp_preset`：catalog 里登记的切片器配置（`.json`）与图标（`.svg`）
 *      一行都不许出现在 MKP 那两张表里（2026-10-02 作者截图抓到的混排）；
 *   ② 切片器档也不含图标，而且 **catalog 登记的切片器交付行**要出现、动作是「下载」
 *      （它是真能下的那一支 —— 官方资产行那颗是死按钮）；
 *   ③ 台账「仓库 N」跟着当前档数：两档各数各的类型，不混成一个全 catalog 的数。
 *
 * 真机上这一刀的现场：MKP 档 → 目录登记的 9 份 `.toml`；切片器档 → 9 份 `bbs_config`；
 * `a1.svg`（icon）与 `*.3mf`（model）哪一档都不出现 —— 它们不归预设页。
 */
await rad('preset-kind', 'mkp').click({ force: true })
await rad('preset-scope', 'cloud').click({ force: true })
await page.waitForTimeout(300)
const mkpCloudRows2 = await actions()
console.log(
  `\n[分类边界 · MKP 云端] ${mkpCloudRows2.map((r) => `${r.name.split(' ')[0]} → ${r.action}`).join(' || ')}`,
)
if (mkpCloudRows2.some((r) => r.name.includes('.svg'))) {
  problems.push('MKP 档云端混进了图标（.svg 不归预设页）')
}
if (mkpCloudRows2.some((r) => r.name.includes('MKPProcess'))) {
  problems.push('MKP 档云端混进了切片器配置（那是切片器档的）')
}
const mkpLedger = (await facts()).counts

await rad('preset-kind', 'slicer').click({ force: true })
await page.waitForTimeout(300)
const slicerCloudRows2 = await actions()
console.log(
  `[分类边界 · 切片器云端] ${slicerCloudRows2.map((r) => `${r.name.split(' ')[0]} → ${r.action}`).join(' || ')}`,
)
if (slicerCloudRows2.some((r) => r.name.includes('.svg'))) {
  problems.push('切片器档云端混进了图标（.svg 不归预设页）')
}
const bbsReleaseRow = slicerCloudRows2.find((r) => r.name.includes('MKPProcess A1 0.4 0.20.json'))
console.log(`[分类边界] catalog 登记的切片器交付行：${bbsReleaseRow?.action ?? '(没这一行)'}`)
if (bbsReleaseRow === undefined) {
  problems.push('切片器档云端没有 catalog 登记的切片器交付行（它该按 kind 分流到这里）')
} else if (!/下载|更新|重新下载/.test(bbsReleaseRow.action)) {
  problems.push(`切片器交付行的动作该是「下载」那一支（真能下），实测「${bbsReleaseRow.action}」`)
}
const slicerLedger = (await facts()).counts

/*
 * 台账：mock 里两档的「仓库 N」是两个不同的数（各数各的类型）。
 * 相同 = 要么在数全量、要么这个断言要跟着演示数据改 —— 两种情况都该有人来看一眼。
 */
const ledgerNum = (t) => /仓库 (\d+)/.exec(t)?.[1] ?? ''
console.log(
  `[分类边界 · 台账] MKP 档「仓库 ${ledgerNum(mkpLedger)}」 切片器档「仓库 ${ledgerNum(slicerLedger)}」`,
)
if (ledgerNum(mkpLedger) === '' || ledgerNum(mkpLedger) === ledgerNum(slicerLedger)) {
  problems.push('台账「仓库 N」该跟着当前档数（两档是各自类型的数，不该相同）')
}

/* 回到 MKP 档：后面的节按它走 */
await rad('preset-kind', 'mkp').click({ force: true })
await page.waitForTimeout(200)

/* ---------- 3. 回本地，点行展开 ---------- */
await rad('preset-scope', 'local').click({ force: true })
await page.waitForTimeout(250)
const before = await facts()
const row = page.locator('main tbody tr').filter({ has: page.locator('td') }).first()
await row.click()
await page.waitForTimeout(250)
const afterClick = await facts()
console.log(`\n[点行展开] 展开前 dl=${before.expanded} 展开后 dl=${afterClick.expanded}`)
if (afterClick.expanded <= before.expanded) problems.push('点一行没有展开（dl 数没变）')

/* ---------- 4. 右键菜单 ---------- */
await row.click({ button: 'right' })
await page.waitForTimeout(250)
const menu = await page.evaluate(() => {
  const ul = document.querySelector('[role="menu"]')
  if (ul === null) return null
  return [...ul.querySelectorAll('[role="menuitem"]')].map((b) => ({
    label: b.textContent?.trim() ?? '',
    disabled: b.getAttribute('data-on') !== '1',
  }))
})
console.log(`\n[右键菜单] ${menu === null ? '没有出现' : menu.map((m) => `${m.label}${m.disabled ? '(灰)' : ''}`).join(' | ')}`)
if (menu === null) problems.push('右键一行没有菜单')
await page.screenshot({ path: `${shotDir}/presets-menu.png` })
await page.keyboard.press('Escape')
await page.waitForTimeout(150)

/* ---------- 5. 交付预设的四态（catalog 登记 + 下载区 mkp/ + 认得出是哪一版吗） ---------- */
/*
 * 守的是**状态可见性**：目录里那一份在本机是什么样，页面上要说得对、给的动作要对。
 *
 * 浏览器模式（假后端）给的是固定演示集合（`src/api/mock.ts` 那三个读合起来）：
 * 一份对得上目录、一份是归档里那版旧版、一份哪儿都查不出是哪一版 —— 四档都得画出来
 *（「未下载」那一档由官方行的「下载」按钮覆盖，见那一节）。
 * 最容易犯的错是**把盘上不对劲的那份画成「未下载」**：只看"文件在不在"就会把一份坏档
 * 说成没下过，用户点"下载"以为是第一次下。所以这条单独断言。
 */
await rad('preset-kind', 'mkp').click({ force: true })
await page.waitForTimeout(200)
await rad('preset-scope', 'local').click({ force: true })
await page.waitForTimeout(300)

const localActions = await actions()
console.log(`\n[交付四态 · 本地表] ${localActions.map((r) => `${r.name} → ${r.action}`).join(' || ')}`)
const localFast = localActions.find((r) => r.name.includes('A1-fast.toml'))
if (localFast === undefined) {
  problems.push('本地表里没有盘上不对劲的那一份（盘上确实有它，藏起来就等于说本机没有）')
} else if (!localFast.action.includes('更新')) {
  problems.push(`旧版本那一份，本地表的动作该是「更新」，实测「${localFast.action}」`)
}

await rad('preset-scope', 'cloud').click({ force: true })
await page.waitForTimeout(300)
const cloudRows = await actions()
console.log(`[交付四态 · 云端表] ${cloudRows.map((r) => `${r.name} → ${r.action}`).join(' || ')}`)

/*
 * 四态里只断言当前机型这两档：假后端给的是**按 (机型, 版本) 各一份**的固定演示数据
 *（见 `src/api/mock.ts` 那段注释），A1 下这两份正好是「已下载」与「旧版本」。
 * 另两档（内容异常 / 未下载）在 5f 与官方行那颗「下载」按钮上量，这里不重复。
 */
const wantState = [
  ['A1-standard.toml', '已下载', '对得上目录的那一份 → 灰字「已下载」，没有可点的动作'],
  ['A1-fast.toml', '更新', '盘上那份就是归档里那一版 → 按钮是「更新」，不是「下载」'],
]
for (const [file, want, why] of wantState) {
  const hit = cloudRows.find((r) => r.name.includes(file))
  console.log(`  ${file} → ${hit?.action ?? '(没这一行)'}（期望含「${want}」）—— ${why}`)
  if (hit === undefined) problems.push(`云端表里没有 ${file}`)
  else if (!hit.action.includes(want)) {
    problems.push(`${file} 该显示「${want}」，实测「${hit.action}」`)
  }
}

/* 点「更新」：浏览器里没有下载区，**必须如实说失败** —— 不许说"已更新" */
const updRow = page
  .locator('main tbody tr')
  .filter({ has: page.locator('td:not([colspan])') })
  .filter({ hasText: 'A1-fast.toml' })
  .first()
await updRow.getByRole('button', { name: '更新' }).click()
await page.waitForTimeout(700)
const note = await page.evaluate(
  /* 提示条本体是 `[role="status"]`（里面主句 + 逐份明细各一行）—— 别按 `p` 找 */
  () => document.querySelector('main [role="status"]')?.innerText.replace(/\s+/g, ' ').trim() ?? '',
)
console.log(`[交付四态] 点「更新」之后提示条：${note || '(没有提示条)'}`)
if (note === '') problems.push('点「更新」之后没有提示条')
if (note.includes('已更新')) problems.push(`浏览器里没有下载区，不许说「已更新」（实测「${note}」）`)
if (!/失败|没成/.test(note)) problems.push(`点「更新」应当如实报失败，实测提示条是「${note}」`)

/* 展开详情里那句「状态」：四态各自的原话（不是"已应用 / 未应用"那一句） */
await updRow.click()
await page.waitForTimeout(300)
const statusFact = await page.evaluate(() => {
  const dl = document.querySelector('main tbody dl')
  if (dl === null) return ''
  const dts = [...dl.querySelectorAll('dt')]
  const dds = [...dl.querySelectorAll('dd')]
  const i = dts.findIndex((d) => (d.textContent ?? '').trim() === '状态')
  return i < 0 ? '' : (dds[i]?.textContent ?? '').trim()
})
console.log(`[交付四态] 展开详情「状态」= ${statusFact || '(没有这一格)'}`)
if (!statusFact.includes('旧版本')) problems.push(`展开详情的状态该说「旧版本」，实测「${statusFact}」`)
await page.screenshot({ path: `${shotDir}/presets-release-states.png` })

/* ---------- 5b. 归档：官方旧版本看得见、认得出、看得了 ---------- */
/*
 * 守两件事：
 *   ① 归档的那一格画得出来、点得开，里面按**路径**列出旧版本（假后端给了一条演示）；
 *   ② 这一层**不提供归档管理** —— 抽屉里不许出现「删除 / 恢复 / 清空」这类动作
 *      （归档管理不在这一层，见 HANDOFF §3.5 的七步顺序）。
 * 读正文在浏览器里必然失败（没有盘），所以要断言它**如实说读不出来**，不是显示空正文。
 */
const archiveCell = page.getByRole('button', { name: /^\d+ 份（点开看）$/ })
const archiveCells = await archiveCell.count()
console.log(`\n[归档] 展开详情里的「旧版本」那一格：${archiveCells} 个`)
if (archiveCells === 0) {
  problems.push('展开详情里没有「旧版本」那一格（假后端给了一份归档演示）')
} else {
  await archiveCell.first().click()
  await page.waitForTimeout(400)
  const drawer = await page.evaluate(() => {
    const dlg = document.querySelector('[role="dialog"]')
    return {
      open: dlg !== null,
      text: (dlg?.innerText ?? '').replace(/\s+/g, ' ').trim().slice(0, 240),
      buttons: [...(dlg?.querySelectorAll('button') ?? [])].map(
        (b) => (b.innerText || b.getAttribute('aria-label') || '').trim(),
      ),
    }
  })
  console.log(`[归档] 抽屉${drawer.open ? '开着' : '没开'}：${drawer.text}`)
  if (!drawer.open) problems.push('点「旧版本」没有打开抽屉')
  /* 列表里要认出它：文件名 + 这是哪台机型的哪一版（那条归档是不是你要找的，就靠这两个） */
  if (!drawer.text.includes('A1-fast.toml')) problems.push('抽屉里没列出那份旧版本')
  if (!/A1 · FAST/.test(drawer.text)) problems.push('抽屉里要认出它是哪台机型的哪一版')
  for (const forbidden of ['删除', '恢复', '清空']) {
    if (drawer.buttons.some((b) => b.includes(forbidden))) {
      problems.push(`这一层不提供归档管理，抽屉里不该有「${forbidden}」`)
    }
  }

  await page.getByRole('button', { name: '看正文' }).first().click()
  await page.waitForTimeout(500)
  const bodyText = await page.evaluate(() =>
    (document.querySelector('[role="dialog"]')?.innerText ?? '').replace(/\s+/g, ' ').trim(),
  )
  console.log(`[归档] 点「看正文」之后：${bodyText.slice(0, 160)}`)
  if (!/读不出来/.test(bodyText)) {
    problems.push(`浏览器里没有盘，读正文该如实说读不出来，实测「${bodyText.slice(0, 120)}」`)
  }
  /* 正文头要带**归档里那条路径** —— 读它用的就是这条路径（界面上的"哪一份"由此无歧义） */
  if (!bodyText.includes('archive/mkp/presets/A1-fast.toml')) {
    problems.push('正文头要带那份旧版本在归档里的路径')
  }
  await page.screenshot({ path: `${shotDir}/presets-archive.png` })
  await page.keyboard.press('Escape')
  await page.waitForTimeout(300)
}

/* ---------- 5c. 批量：多份一起处理（逐份给结局） ---------- */
/*
 * 守三件事：
 *   ① 批次的范围 —— **已下载的不进来**（这一批只有那一份「需更新」的）；
 *   ② 结局**逐份**给 —— 没成的那几份各占一行、带后端给的原因，不压成一句"批量失败"；
 *   ③ 有名有姓的失败**不许被说成成功**（假后端里这一批是全军覆没）。
 * 命令级失败那一档（比如没配数据源 → 一份都没发出去）浏览器里没有触发路径，这里量不到。
 */
const batchBtn = page.getByRole('button', { name: /^(下载|更新|下载并更新) \d+ 份$/ })
const batchCount = await batchBtn.count()
const barText =
  batchCount > 0
    ? (await batchBtn.first().locator('xpath=..').innerText()).replace(/\s+/g, ' ').trim()
    : ''
console.log(`\n[批量] 批次行：${barText || '(没有这一行)'}  按钮 ${batchCount} 个`)
if (batchCount === 0) problems.push('云端表有 1 份需更新，却没出现批量那一行')
if (/未下载/.test(barText)) {
  problems.push(`已下载的那一份不该进这一批（它已经对了），实测批次行「${barText}」`)
}
if (!/需更新 1 份/.test(barText)) problems.push(`批次行该说「需更新 1 份」，实测「${barText}」`)

if (batchCount > 0) {
  await batchBtn.first().click()
  await page.waitForTimeout(900)
  const batchNote = await page.evaluate(() => ({
    text: (document.querySelector('main [role="status"]')?.innerText ?? '').replace(/\s+/g, ' ').trim(),
    /* 明细是**各占一行**的（主句 + 每份一段），拼成一句就没法读了 */
    lines: document.querySelectorAll('main [role="status"] p').length,
  }))
  console.log(`[批量] 点完之后提示条（${batchNote.lines} 行）：${batchNote.text}`)
  if (batchNote.lines < 2) {
    problems.push(`逐份结局该各占一行，实测提示条里只有 ${batchNote.lines} 个段落`)
  }
  if (!/没成/.test(batchNote.text)) {
    problems.push(`这一批全军覆没，提示条该如实说「没成」，实测「${batchNote.text}」`)
  }
  if (/都成了/.test(batchNote.text)) {
    problems.push(`一份都没成，不许说成功（实测「${batchNote.text}」）`)
  }
  if (!/没有下载区/.test(batchNote.text)) {
    problems.push(`逐份结局里要带后端给的原因（"浏览器里没有下载区…"），实测「${batchNote.text}」`)
  }
  await page.screenshot({ path: `${shotDir}/presets-batch.png` })
}

await rad('preset-scope', 'local').click({ force: true })
await page.waitForTimeout(200)

/* ---------- 5d. 用户线：我自己的那一份（看得见、认得出、看得了） ---------- */
/*
 * 守三件事：
 *   ① 用户线那两份在本地表里列得出来（假后端给两条演示：一份 `.toml` 认得出、一份 `.json` 认不出）；
 *   ② **认不出类别的那一份在任何类型档下都列**（藏起来等于说他没这份文件）；
 *   ③ 「看正文」是**只读**的：读不出来如实说，抽屉里不许出现 改 / 保存 / 另存 / 删除
 *      （改它要等"临时编辑 → 保存"那一层）。
 */
await page.waitForTimeout(200)
const mineRows = await actions()
console.log(`\n[用户线 · 本地表] ${mineRows.map((r) => r.name).join(' || ')}`)
for (const name of ['我的 A1 涂胶.toml', 'Process_0.2mm.json']) {
  if (!mineRows.some((r) => r.name.includes(name))) {
    problems.push(`本地表里没有我自己的那一份：${name}`)
  }
}

/* ② 认不出的那一份在切片器档下也还在；认得出是 MKP 预设的那一份不该跑过去 */
await rad('preset-kind', 'slicer').click({ force: true })
await page.waitForTimeout(350)
const slicerRows = await actions()
console.log(`[用户线 · 切片器档] ${slicerRows.map((r) => r.name).join(' || ')}`)
if (!slicerRows.some((r) => r.name.includes('Process_0.2mm.json'))) {
  problems.push('认不出类别的那一份在切片器档下也该列出来（认不出就不藏）')
}
if (slicerRows.some((r) => r.name.includes('我的 A1 涂胶.toml'))) {
  problems.push('认得出是 MKP 预设的那一份不该出现在切片器档下')
}
await rad('preset-kind', 'mkp').click({ force: true })
await page.waitForTimeout(350)

/* ③ 展开那一份 → 「正文 / 看正文」 */
const mineRow = page
  .locator('main tbody tr')
  .filter({ has: page.locator('td:not([colspan])') })
  .filter({ hasText: 'Process_0.2mm.json' })
  .first()
await mineRow.click()
await page.waitForTimeout(300)
const mineKind = await page.evaluate(() => {
  const dl = document.querySelector('main tbody dl')
  if (dl === null) return ''
  const dts = [...dl.querySelectorAll('dt')]
  const dds = [...dl.querySelectorAll('dd')]
  const i = dts.findIndex((d) => (d.textContent ?? '').trim() === '类型')
  return i < 0 ? '' : (dds[i]?.textContent ?? '').trim()
})
console.log(`[用户线] 那一份的类型：${mineKind || '(没有这一格)'}`)
if (!mineKind.includes('认不出')) {
  problems.push(`认不出类别的那一份，类型该写「认不出是哪一类」，实测「${mineKind}」`)
}

const bodyBtn = page.getByRole('button', { name: '看正文' })
const bodyBtnCount = await bodyBtn.count()
console.log(`[用户线] 「看正文」按钮 ${bodyBtnCount} 个`)
if (bodyBtnCount === 0) {
  problems.push('展开我自己的那一份，里面没有「看正文」')
} else {
  await bodyBtn.last().click()
  await page.waitForTimeout(500)
  const mineDrawer = await page.evaluate(() => {
    const dlg = document.querySelector('[role="dialog"]')
    return {
      text: (dlg?.innerText ?? '').replace(/\s+/g, ' ').trim().slice(0, 220),
      buttons: [...(dlg?.querySelectorAll('button') ?? [])].map(
        (b) => (b.innerText || b.getAttribute('aria-label') || '').trim(),
      ),
    }
  })
  console.log(`[用户线] 抽屉：${mineDrawer.text}`)
  if (!mineDrawer.text.includes('Process_0.2mm.json')) {
    problems.push('抽屉里要认出看的是哪一份')
  }
  if (!/读不出来/.test(mineDrawer.text)) {
    problems.push('浏览器里没有用户目录，读正文该如实说读不出来')
  }
  /* 这一层只能看：写入动作连按钮都不该有 */
  for (const forbidden of ['保存', '另存', '删除', '改名']) {
    if (mineDrawer.buttons.some((b) => b.includes(forbidden))) {
      problems.push(`这一层只能看，抽屉里不该有「${forbidden}」`)
    }
  }
  await page.screenshot({ path: `${shotDir}/presets-mine.png` })
  await page.keyboard.press('Escape')
  await page.waitForTimeout(250)
}

/* ---------- 5e. 临时编辑：改 → 另存成用户那一份 ---------- */
/*
 * 守三件事：
 *   ① 只有"与目录一致"的交付行有「改这份」（没下载就没正文可改；需更新的那份内容存疑）；
 *   ② 编辑器里是**临时文件里那份**正文，不是就地改官方；
 *   ③ 保存之后本地表多出 `（已修改）` 那一份 —— 而**原来那一份还在**（官方原件没被改掉）。
 *
 * 真机那条更硬的判据在 Rust 侧（`runtime::mine`）：另存只写用户根、
 * 官方原件字节不变、下载区里不会多出文件（临时文件不住 `mkp/`）。
 */
const stdRow = page
  .locator('main tbody tr')
  .filter({ has: page.locator('td:not([colspan])') })
  .filter({ hasText: 'A1-standard.toml' })
  .first()
await stdRow.click()
await page.waitForTimeout(300)

const editBtn = page.getByRole('button', { name: '改这份' })
const editBtnCount = await editBtn.count()
console.log(`\n[编辑] 「改这份」按钮 ${editBtnCount} 个`)
if (editBtnCount === 0) {
  problems.push('与目录一致的交付行展开后没有「改这份」')
} else {
  await editBtn.first().click()
  await page.waitForTimeout(500)
  const editor = await page.evaluate(() => {
    const dlg = document.querySelector('[role="dialog"]')
    const area = dlg?.querySelector('textarea')
    return {
      open: area !== null && area !== undefined,
      text: area?.value ?? '',
      buttons: [...(dlg?.querySelectorAll('button') ?? [])].map((b) =>
        (b.innerText || b.getAttribute('aria-label') || '').trim(),
      ),
    }
  })
  console.log(
    `[编辑] 抽屉${editor.open ? '开着' : '没开'}，正文 ${editor.text.length} 字节：${editor.text.replace(/\n/g, ' / ').slice(0, 70)}`,
  )
  if (!editor.open) problems.push('点「改这份」没有打开编辑器（抽屉里该有一个 textarea）')
  if (editor.text.length === 0) problems.push('编辑器里没有正文（该是临时文件里那份）')
  if (!editor.buttons.includes('保存为用户文件')) problems.push('编辑器里没有「保存为用户文件」')
  if (!editor.buttons.includes('放弃这次编辑')) problems.push('编辑器里没有「放弃这次编辑」')

  /* 改一行：边改边存（debounce 700ms），抽屉里不该出现"草稿没存上" */
  await page.locator('[role="dialog"] textarea').fill('# 改过的正文\n涂胶宽度 = 1.4\n')
  await page.waitForTimeout(1300)
  const draftErr = await page.evaluate(
    () => document.querySelector('[role="dialog"]')?.innerText ?? '',
  )
  if (draftErr.includes('草稿没存上')) problems.push('草稿没存上（浏览器里草稿也在内存里，不该失败）')

  await page.getByRole('button', { name: '保存为用户文件' }).click()
  await page.waitForTimeout(700)
  const savedNote = await page.evaluate(() =>
    (document.querySelector('main [role="status"]')?.innerText ?? '').replace(/\s+/g, ' ').trim(),
  )
  console.log(`[编辑] 保存之后提示条：${savedNote}`)
  if (!savedNote.includes('已保存')) {
    problems.push(`保存之后提示条该说「已保存…」，实测「${savedNote}」`)
  }
  if (!savedNote.includes('presets-mine/')) {
    problems.push('保存之后要说清落在哪（presets-mine/…）')
  }

  const afterSave = await actions()
  console.log(`[编辑] 保存后本地表：${afterSave.map((r) => r.name).join(' || ')}`)
  if (!afterSave.some((r) => r.name.includes('A1-standard（已修改）.toml'))) {
    problems.push('保存之后本地表里该多出「（已修改）」那一份')
  }
  if (!afterSave.some((r) => r.name.includes('A1-standard.toml'))) {
    problems.push('保存不该动官方原件：本地表里原来那一份还得在')
  }
  await page.screenshot({ path: `${shotDir}/presets-edit.png` })
}

/* ---------- 5f. 认得出 / 认不出（第三圈第 6 层：官方文件的 SHA 报警） ---------- */
/*
 * 守三件事：
 *   ① 盘上与目录不符的两档**分得开**：`旧版本`（认得出是官方某一版旧版）与
 *      `内容异常`（这台机器上查不出它属于哪一版）—— 假后端各给一份演示；
 *   ② 内容存疑的那两份**没有「应用」也没有「改这份」**：修复动作只有重新下载
 *      （不给点了必报错的按钮：`applyActivePreset` 的第一道闸就是 SHA）；
 *   ③ 那两份的「复制」在右键菜单里是**灰的、而且带原因**（不许复制那条边界）。
 *
 * 真机上更硬的判据在 Rust 侧（`runtime::delivery`）：旧版 / 查不出是哪一版的判定，
 * 以及「改这份」在入口就把漂了的字节拒掉（`official_text`）。
 */
await rad('preset-kind', 'mkp').click({ force: true })
await rad('preset-scope', 'local').click({ force: true })
await page.waitForTimeout(250)
/* 那两份演示分属两台机型：切到「全部机型」才同屏看得到（本机的东西不该被机型选择藏起来） */
await page.getByRole('button', { name: '机型' }).click()
await page.waitForTimeout(200)
await page.getByRole('option', { name: /全部机型/ }).click()
await page.waitForTimeout(400)

const trustRows = await actions()
console.log(`\n[认得出 · 全部机型] ${trustRows.map((r) => `${r.name} → ${r.action}`).join(' || ')}`)

const trustWant = [
  ['A1-fast.toml', '更新', '盘上那份**就是归档里那一版** → 旧版本，换成当前版'],
  ['A1mini-standard.toml', '重新下载', '盘上那份哪儿都查不出是哪一版 → 内容异常，只能重下一份'],
]
for (const [file, want, why] of trustWant) {
  const hit = trustRows.find((r) => r.name.includes(file))
  console.log(`  ${file} → ${hit?.action ?? '(没这一行)'}（期望含「${want}」）—— ${why}`)
  if (hit === undefined) problems.push(`本地表里没有 ${file}`)
  else if (!hit.action.includes(want)) problems.push(`${file} 的动作该含「${want}」，实测「${hit.action}」`)
  else if (hit.action.includes('应用')) problems.push(`${file} 内容存疑，不该给「应用」（实测「${hit.action}」）`)
}

/** 展开详情里某一格的值（dt 文案 → dd 文案） */
const factOf = () =>
  page.evaluate(() => {
    const dl = document.querySelector('main tbody dl')
    if (dl === null) return {}
    const dts = [...dl.querySelectorAll('dt')]
    const dds = [...dl.querySelectorAll('dd')]
    const out = {}
    dts.forEach((dt, i) => {
      out[(dt.textContent ?? '').trim()] = (dds[i]?.textContent ?? '').trim()
    })
    return out
  })

const suspectRow = (file) =>
  page
    .locator('main tbody tr')
    .filter({ has: page.locator('td:not([colspan])') })
    .filter({ hasText: file })
    .first()

await suspectRow('A1mini-standard.toml').click()
await page.waitForTimeout(300)
const tamperedFacts = await factOf()
console.log(`[认不出] A1mini-standard.toml 展开详情：状态=${tamperedFacts['状态'] ?? '(没有)'}`)
if (!(tamperedFacts['状态'] ?? '').includes('内容异常')) {
  problems.push(`认不出的那一份，状态该说「内容异常」，实测「${tamperedFacts['状态']}」`)
}
const editBtns = await page.getByRole('button', { name: '改这份' }).count()
console.log(`[认不出] 展开详情里「改这份」按钮：${editBtns} 个（展开的这一份内容存疑，该是 0）`)
if (editBtns > 0) problems.push('内容存疑的那一份不该有「改这份」（改的来源必须是官方当前版）')

/* 右键：不许复制那条边界要说得出原因（灰一项不说为什么等于坏了） */
await suspectRow('A1mini-standard.toml').click({ button: 'right' })
await page.waitForTimeout(250)
const copyItem = await page.evaluate(() => {
  const ul = document.querySelector('[role="menu"]')
  if (ul === null) return null
  const btn = [...ul.querySelectorAll('[role="menuitem"]')].find(
    (b) => (b.textContent ?? '').trim() === '另存为一份新的',
  )
  return btn === undefined ? null : { disabled: btn.getAttribute('data-on') !== '1', why: btn.getAttribute('title') ?? '' }
})
console.log(`[认不出] 右键「另存为一份新的」：${copyItem === null ? '没有这一项' : `灰=${copyItem.disabled} 原因「${copyItem.why}」`}`)
if (copyItem === null) problems.push('右键菜单里没有「另存为一份新的」这一项')
else if (!copyItem.disabled) problems.push('内容存疑的那一份不许复制，菜单里该是灰的')
else if (!copyItem.why.includes('不许')) problems.push(`灰掉的「另存为一份新的」要带原因（不许复制），实测「${copyItem.why}」`)
await page.screenshot({ path: `${shotDir}/presets-trust.png` })
await page.keyboard.press('Escape')
await page.waitForTimeout(200)

/* 回到 A1：后面几节按这台机型看（选项顺序＝下拉给的顺序：全部机型 排第一档，接着第一台机型） */
await page.getByRole('button', { name: '机型' }).click()
await page.waitForTimeout(200)
await page.getByRole('option').nth(1).click()
await page.waitForTimeout(350)

/* ---------- 5g. 我那份能被使用，而且说得清基于哪一版官方（第七层） ---------- */
/*
 * 守四件事：
 *   ① 我那份**有「应用」**（不再是没有动作的「—」）：两条线都能成为使用中的那一份
 *      —— "只读"是文件归属的属性，不是"能不能被使用"的属性；
 *   ② 点它 → 状态条说得出「已应用 我那份」，而且**多一枚「我的文件」**
 *      （用户得看得出现在跑的不是官方那份）；
 *   ③ 「基于旧版官方」那一枚画得出来（假后端给的那份正好是从旧版改的）；
 *   ④ 展开详情那一格说得清：来源 + 官方已换新版。
 *
 * 真机那条更硬的判据在 Rust 侧：`state::save_active_mine` / `active_target`（两条线的落点）
 * 与 `mine::based_on`（旧的 / 当前的 / 说不清）。
 */
await rad('preset-kind', 'mkp').click({ force: true })
await rad('preset-scope', 'local').click({ force: true })
await page.waitForTimeout(300)

const myActions = await actions()
const myEntry = myActions.find((r) => r.name.includes('我的 A1 涂胶.toml'))
console.log(`\n[我的那份] ${myEntry?.name ?? '(没这一行)'} → ${myEntry?.action ?? '(没有)'}`)
if (myEntry === undefined) {
  problems.push('本地表里没有我自己的那一份')
} else if (!myEntry.action.includes('应用')) {
  problems.push(`我那份该有「应用」（两条线都能成为使用中的那一份），实测「${myEntry.action}」`)
}

const myTr = page
  .locator('main tbody tr')
  .filter({ has: page.locator('td:not([colspan])') })
  .filter({ hasText: '我的 A1 涂胶.toml' })
  .first()
const rowText = (await myTr.innerText()).replace(/\s+/g, ' ').trim()
console.log(`[我的那份] 行内容：${rowText}`)
if (!rowText.includes('基于旧版官方')) {
  problems.push('从旧版改出来的那一份，名字旁边该有「基于旧版官方」那一枚')
}

await myTr.click()
await page.waitForTimeout(300)
const myFacts = await factOf()
console.log(`[我的那份] 展开详情「基于」= ${myFacts['基于'] ?? '(没有这一格)'}`)
if (!(myFacts['基于'] ?? '').includes('官方已换新版')) {
  problems.push(`旧版派生那一份，「基于」那格该说「官方已换新版」，实测「${myFacts['基于']}」`)
}
if (!(myFacts['基于'] ?? '').includes('快拆版6月以前')) {
  problems.push('「基于」那格要说得出来源是哪台机型的哪一版（A1 · 快拆版6月以前）')
}

await myTr.getByRole('button', { name: '应用' }).click()
await page.waitForTimeout(600)
const strip = await page.evaluate(() =>
  (document.querySelector('main')?.innerText ?? '').replace(/\s+/g, ' ').trim().slice(0, 200),
)
console.log(`[我的那份] 应用之后（前 200 字）：${strip}`)
if (!strip.includes('已应用')) problems.push('应用我那份之后，状态条该说「已应用」')
if (!strip.includes('我的 A1 涂胶.toml')) problems.push('状态条要说得出用的是我那一份')
if (!strip.includes('我的文件')) problems.push('用的是我那份时，状态条上该有「我的文件」那一枚')
const afterApply = (await actions()).find((r) => r.name.includes('我的 A1 涂胶.toml'))
console.log(`[我的那份] 应用之后操作列：${afterApply?.action ?? '(没有)'}`)
if (!(afterApply?.action ?? '').includes('已应用')) {
  problems.push(`应用之后那一行该变成灰字「已应用」，实测「${afterApply?.action}」`)
}
await page.screenshot({ path: `${shotDir}/presets-mine-apply.png` })

/* ---------- 5h. 改我自己这份 → 保存回它自己（第八层） ---------- */
/*
 * 守四件事：
 *   ① 我那份也有「改这份」—— 它已经是我自己的文件，"能改"是自然的下一步；
 *   ② 编辑器里给的是**正文**：三行血统是程序的元数据，不该出现在编辑器里；
 *   ③ 保存说「保存回我这份」，存完**没有多出一份**（不产生 `（已修改）2.toml`）；
 *   ④ **血统还在**：保存之后「看正文」看得见那三行，「基于旧版官方」那一枚也没变
 *      —— 改的是参数，不是它从哪一版官方派生。
 *
 * 真机那条更硬的判据在 Rust 侧：`mine::save_back`（同一路径写回、血统照抄）与
 * `lineage::rewrite_keeping_lineage`（摘要不许变成"我自己改过的字节"）。
 */
/* 展开详情这一格：上一节点过这一行，它可能已经展开着 —— 再点一次会收起来 */
const ensureExpanded = async () => {
  if ((await page.locator('main tbody dl').count()) === 0) {
    await myTr.click()
    await page.waitForTimeout(300)
  }
}
await ensureExpanded()
const mineEditFact = (await factOf())['修改'] ?? '(没有这一格)'
console.log(`\n[改我这份] 展开详情「修改」= ${mineEditFact}`)
if (mineEditFact !== '改这份') {
  problems.push(`我那份展开详情里该有「修改 → 改这份」，实测「${mineEditFact}」`)
}

/* 展开详情那一格在**另一行**（colspan 那一行），所以从 `dl` 里点 */
await page.locator('main tbody dl').getByRole('button', { name: '改这份' }).click()
await page.waitForTimeout(600)
const mineEditor = await page.evaluate(() => {
  const dlg = document.querySelector('[role="dialog"]')
  const area = dlg?.querySelector('textarea')
  return {
    open: area !== null && area !== undefined,
    whole: (dlg?.innerText ?? '').replace(/\s+/g, ' ').trim(),
    text: area?.value ?? '',
  }
})
console.log(`[改我这份] 抽屉：${mineEditor.whole.slice(0, 80)}`)
if (!mineEditor.open) problems.push('点我那份的「改这份」没有打开编辑器')
if (!mineEditor.whole.includes('改我自己这份')) {
  problems.push(`编辑我自己那份时标题该说「改我自己这份」，实测「${mineEditor.whole.slice(0, 40)}」`)
}
if (!mineEditor.whole.includes('保存回我这份')) {
  problems.push('编辑我自己那份时，保存按钮该说「保存回我这份」（不是「保存为用户文件」）')
}
if (mineEditor.text.includes('# based_on')) {
  problems.push('编辑器里不该出现血统那三行（那是程序的元数据，不是用户改的正文）')
}
if (!mineEditor.text.includes('涂胶宽度')) problems.push('编辑器里没有我那份的正文')

/* 改一行：边改边存（debounce 700ms） */
await page.locator('[role="dialog"] textarea').fill('涂胶宽度 = 1.8\n起始延时 = 0.4\n')
await page.waitForTimeout(1300)
await page.getByRole('button', { name: '保存回我这份' }).click()
await page.waitForTimeout(700)
const backNote = await page.evaluate(() =>
  (document.querySelector('main [role="status"]')?.innerText ?? '').replace(/\s+/g, ' ').trim(),
)
console.log(`[改我这份] 保存之后提示条：${backNote}`)
if (!backNote.includes('已保存回我自己那一份')) {
  problems.push(`写回之后该说「已保存回我自己那一份」，实测「${backNote}」`)
}
if (!backNote.includes('presets-mine/我的 A1 涂胶.toml')) {
  problems.push(`写回之后要说得出写回哪去了（同一条路径），实测「${backNote}」`)
}
await page.screenshot({ path: `${shotDir}/presets-mine-edit.png` })

/* 没有多出一份：写回的是它自己 */
const mineNames = (await actions()).map((r) => r.name)
const mineCount = mineNames.filter((n) => n.includes('我的 A1 涂胶')).length
console.log(`[改我这份] 表里「我的 A1 涂胶」${mineCount} 行：${mineNames.join(' | ')}`)
if (mineCount !== 1) problems.push(`写回不该多出一份（实测 ${mineCount} 行都叫「我的 A1 涂胶」）`)
if (mineNames.some((n) => n.includes('我的 A1 涂胶（已修改）'))) {
  problems.push('写回我自己那份不该另存出一份新的（不产生「（已修改）」）')
}

/* 血统还在：保存之后看正文，那三行与"旧版派生"那一枚都还在 */
await ensureExpanded()
const mineRowAfter = (await myTr.innerText()).replace(/\s+/g, ' ').trim()
if (!mineRowAfter.includes('基于旧版官方')) {
  problems.push('写回之后「基于旧版官方」那一枚该还在（出处没变）')
}
await page.locator('main tbody dl').getByRole('button', { name: '看正文' }).click()
await page.waitForTimeout(600)
const mineBody = await page.evaluate(
  () => document.querySelector('[role="dialog"] pre')?.innerText ?? '',
)
console.log(`[改我这份] 保存之后正文：${mineBody.replace(/\s+/g, ' ').trim().slice(0, 120)}`)
if (!mineBody.includes('涂胶宽度 = 1.8')) problems.push('写回之后正文该是改过的那份')
if (!mineBody.includes('# based_on:')) problems.push('写回之后血统那三行该还在（出处不能丢）')
await page.keyboard.press('Escape')
await page.waitForTimeout(250)

/* ---------- 5i. 第九层：用户文件的文件级状态（读不出来 → 不许应用 / 编辑） ---------- */
/*
 * 守三件事：
 *   ① TOML 语法坏的那一份**照常列在表里**、行上画得出「文件无法读取」（藏起来 = 说他没这份文件）；
 *   ② 它**没有「应用」**（操作列是灰字不是按钮），展开详情里**没有「改这份」** ——
 *      后端会在入口拒的按钮，界面不给（"不给必报错的按钮"那条口径）；
 *   ③ 能读的那一份不受牵连：不因此多出任何角标。
 *
 * 这一层看的是"还能不能读"（能读 + UTF-8 + TOML 语法），**不是 SHA** —— 外部修改过
 * 但仍是能读的 TOML 照常能用，不报警。真机上更硬的判据在 Rust 侧（`runtime::mine` 的
 * 文件级检查 + 应用 / 编辑两个入口的闸）。
 */
await rad('preset-kind', 'mkp').click({ force: true })
await rad('preset-scope', 'local').click({ force: true })
await page.waitForTimeout(300)

const nineActions = await actions()
const brokenEntry = nineActions.find((r) => r.name.includes('坏了的涂胶.toml'))
console.log(
  `\n[第九层] 坏的那一份 → ${brokenEntry?.name ?? '(没这一行)'} → ${brokenEntry?.action ?? '(没有)'}`,
)
if (brokenEntry === undefined) {
  problems.push('读不出来的那一份该照常列在本地表里（藏起来等于说他没这份文件）')
} else {
  if (!brokenEntry.name.includes('文件无法读取')) {
    problems.push(`坏的那一份名字旁边该有「文件无法读取」，实测行内容「${brokenEntry.name}」`)
  }
  if (brokenEntry.action.includes('应用')) {
    problems.push(`坏的那一份不许给「应用」（后端会在入口拒），实测操作列「${brokenEntry.action}」`)
  }
}

const brokenTr = page
  .locator('main tbody tr')
  .filter({ has: page.locator('td:not([colspan])') })
  .filter({ hasText: '坏了的涂胶.toml' })
  .first()
/* 角标 title 里带后端给的原因（"TOML 语法不对（第 3 行第 1 列）"）—— 为什么读不出来要看得到。
   认 `span[title]`：上面那层 `.nameLine` 是普通 span（没 title），不认它就选到父级去了 */
const brokenBadgeWhy = await brokenTr
  .locator('span[title]')
  .filter({ hasText: '文件无法读取' })
  .first()
  .getAttribute('title')
console.log(`[第九层] 角标 title：${brokenBadgeWhy ?? '(没有)'}`)
if (!(brokenBadgeWhy ?? '').includes('TOML')) {
  problems.push(`角标 title 该带后端给的原因（TOML 语法错在哪），实测「${brokenBadgeWhy ?? ''}」`)
}

await brokenTr.click()
await page.waitForTimeout(300)
const brokenFacts = await factOf()
console.log(
  `[第九层] 展开详情「文件」= ${brokenFacts['文件'] ?? '(没有这一格)'}｜状态=${brokenFacts['状态'] ?? '(没有)'}`,
)
if (!(brokenFacts['文件'] ?? '').includes('无法读取')) {
  problems.push(`展开详情该有一格说「文件无法读取」，实测「${brokenFacts['文件'] ?? '(没有)'}」`)
}
const brokenEditCount = await page.getByRole('button', { name: '改这份' }).count()
console.log(`[第九层] 展开详情里「改这份」按钮：${brokenEditCount} 个（这一份读不出来，该是 0）`)
if (brokenEditCount > 0) problems.push('读不出来的那一份不该有「改这份」（入口会拒）')

/* 能读的那一份不受牵连：没有「文件无法读取」这一枚 */
const goodEntry = nineActions.find((r) => r.name.includes('我的 A1 涂胶.toml'))
if (goodEntry === undefined) {
  problems.push('能读的那一份该照常在表里')
} else if (goodEntry.name.includes('文件无法读取')) {
  problems.push(`能读的那一份不该被画成读不出来，实测行内容「${goodEntry.name}」`)
}
await page.screenshot({ path: `${shotDir}/presets-mine-unreadable.png` })

/* ---------- 5j. 第十层：用户文件管理（重命名 / 删除） ---------- */
/*
 * 守四件事：
 *   ① 重命名 = 只换名字：行上的名字跟着变、**状态不变**（坏的那份改名后照样画「文件无法读取」——
 *      字节没动）；旧名字那一行没了；
 *   ② 删除要**二次确认**（菜单里那个问句），确认完那一行从表里消失 —— 真删除，没有留档；
 *   ③ 正在使用的那一份**不给删**（右键里「删除」灰掉、带原因）；
 *   ④ 改名**不断使用中**：正在使用那份改完名，行上照样「已应用」（指针跟着走）；
 *      改到一半关掉的那份草稿也跟着走 —— 再点「改这份」还是「上次改到一半的那一份」。
 *
 * 真机上更硬的判据在 Rust 侧：`mine::rename_file`（字节一个不动 / 不覆盖 / 不改后缀）、
 * `mine::delete_file`（两道闸：正在使用的 / 还有草稿的）、`state` 两条 repoint。
 */
await rad('preset-kind', 'mkp').click({ force: true })
await rad('preset-scope', 'local').click({ force: true })
await page.waitForTimeout(300)

const rows10 = await actions()
if (rows10.find((r) => r.name.includes('坏了的涂胶.toml')) === undefined) {
  problems.push('第十层要从「坏了的涂胶.toml」开始（它没在表里）')
}

/* ① 重命名：把坏的那份改个名字 —— 名字变了、状态不变（还是「文件无法读取」） */
const brokenTr10 = page
  .locator('main tbody tr')
  .filter({ has: page.locator('td:not([colspan])') })
  .filter({ hasText: '坏了的涂胶.toml' })
  .first()
await brokenTr10.click({ button: 'right' })
await page.waitForTimeout(250)
await page.getByRole('menuitem', { name: '重命名' }).click()
await page.waitForTimeout(300)
const renameDlg = page.getByRole('dialog', { name: '重命名' })
const renameInput = renameDlg.getByLabel('新的文件名')
const prefill = await renameInput.inputValue()
console.log(`\n[第十层 · 改名] 输入框初值：${prefill}`)
if (prefill !== '坏了的涂胶.toml') problems.push(`改名输入框该预填现在的名字，实测「${prefill}」`)
await renameInput.fill('坏了的涂胶-改过名.toml')
await renameDlg.getByRole('button', { name: '改名' }).click()
await page.waitForTimeout(400)
const afterRename = await actions()
const renamed = afterRename.find((r) => r.name.includes('坏了的涂胶-改过名.toml'))
console.log(`[第十层 · 改名] 改完：${renamed === undefined ? '(新名字没出现)' : renamed.name}`)
if (renamed === undefined) {
  problems.push('改完名之后表里该有新名字那一行')
} else if (!renamed.name.includes('文件无法读取')) {
  problems.push('改名只动名字：坏的那份改完名也该还是「文件无法读取」（字节没动）')
}
if (afterRename.some((r) => r.name.includes('坏了的涂胶.toml') && !r.name.includes('改过名'))) {
  problems.push('改完名之后旧名字那一行该没了')
}

/* ② 删除：先二次确认，再删掉这一行（它没人用、没草稿 —— 真的删得掉） */
const renamedTr = page
  .locator('main tbody tr')
  .filter({ has: page.locator('td:not([colspan])') })
  .filter({ hasText: '坏了的涂胶-改过名.toml' })
  .first()
await renamedTr.click({ button: 'right' })
await page.waitForTimeout(250)
await page.getByRole('menuitem', { name: '删除' }).click()
await page.waitForTimeout(250)
const askDlg = page.getByRole('dialog', { name: '删除 坏了的涂胶-改过名.toml？' })
const askCount = await askDlg.count()
console.log(`[第十层 · 删除] 二次确认：${askCount > 0 ? '出来了' : '没有'}`)
if (askCount === 0) problems.push('删除要先弹二次确认（不可逆的事必须显式选一个）')
await askDlg.getByRole('button', { name: '删除' }).click()
await page.waitForTimeout(400)
const afterDelete = await actions()
console.log(`[第十层 · 删除] 删完还在吗：${afterDelete.some((r) => r.name.includes('坏了的涂胶')) ? '还在' : '没了'}`)
if (afterDelete.some((r) => r.name.includes('坏了的涂胶'))) {
  problems.push('删完那一行该从表里消失（列表以磁盘为准）')
}

/* ③ 正在使用的那一份不给删（菜单项灰掉、带原因） */
const liveTr = page
  .locator('main tbody tr')
  .filter({ has: page.locator('td:not([colspan])') })
  .filter({ hasText: '我的 A1 涂胶.toml' })
  .first()
await liveTr.click({ button: 'right' })
await page.waitForTimeout(250)
const removeItem = await page.evaluate(() => {
  const ul = document.querySelector('[role="menu"]')
  if (ul === null) return null
  const btn = [...ul.querySelectorAll('[role="menuitem"]')].find(
    (b) => (b.textContent ?? '').trim() === '删除',
  )
  return btn === undefined
    ? null
    : { disabled: btn.getAttribute('data-on') !== '1', why: btn.getAttribute('title') ?? '' }
})
console.log(
  `[第十层 · 删除闸] 正在使用那份右键「删除」：${removeItem === null ? '没有这一项' : `灰=${removeItem.disabled} 原因「${removeItem.why}」`}`,
)
if (removeItem === null) problems.push('右键菜单里没有「删除」这一项')
else if (!removeItem.disabled || !removeItem.why.includes('正在使用')) {
  problems.push('正在使用的那一份「删除」该灰掉并说清原因')
}
await page.keyboard.press('Escape')
await page.waitForTimeout(200)

/* ④ 改名不断使用中 + 草稿跟着走：先在这份上起一份草稿（打开编辑器再关掉，草稿留在盘上），
      然后改名 —— 「已应用」不该断；再点「改这份」还是「上次改到一半的那一份」 */
await liveTr.click()
await page.waitForTimeout(300)
await page.getByRole('button', { name: '改这份' }).click()
await page.waitForTimeout(400)
await page.keyboard.press('Escape')
await page.waitForTimeout(300)
await liveTr.click({ button: 'right' })
await page.waitForTimeout(250)
await page.getByRole('menuitem', { name: '重命名' }).click()
await page.waitForTimeout(300)
await page.getByRole('dialog', { name: '重命名' }).getByLabel('新的文件名').fill('我的 A1 涂胶-高速版.toml')
await page.getByRole('dialog', { name: '重命名' }).getByRole('button', { name: '改名' }).click()
await page.waitForTimeout(500)
const afterLiveRename = await actions()
const liveRenamed = afterLiveRename.find((r) => r.name.includes('我的 A1 涂胶-高速版.toml'))
console.log(
  `[第十层 · 改名 · 使用中] ${liveRenamed === undefined ? '(新名字没出现)' : liveRenamed.name} → ${liveRenamed?.action ?? ''}`,
)
if (liveRenamed === undefined) {
  problems.push('正在使用那份改完名该还在表里')
} else if (!liveRenamed.action.includes('已应用')) {
  problems.push(`改名不该断「已应用」（使用中指针该跟着走），实测操作列「${liveRenamed.action}」`)
}
const liveRenamedTr = page
  .locator('main tbody tr')
  .filter({ has: page.locator('td:not([colspan])') })
  .filter({ hasText: '我的 A1 涂胶-高速版.toml' })
  .first()
await liveRenamedTr.click()
await page.waitForTimeout(300)
await page.getByRole('button', { name: '改这份' }).click()
await page.waitForTimeout(400)
const reusedText = await page.getByRole('dialog', { name: '改我自己这份' }).textContent()
const reused = (reusedText ?? '').includes('上次改到一半')
console.log(`[第十层 · 改名 · 草稿] 再点「改这份」：${reused ? '接着上次改（草稿跟着走了）' : '没接上'}`)
if (!reused) problems.push('改名之后草稿该跟着走（再点「改这份」要说「上次改到一半的那一份」）')
await page.keyboard.press('Escape')
await page.waitForTimeout(200)
await page.screenshot({ path: `${shotDir}/presets-mine-manage.png` })

/* ---------- 5k. 第十一层：另存为一份新的（我的文件 → 我的文件，字节复制） ---------- */
/*
 * 守五件事：
 *   ① 只有「我的文件」这一项能点（官方那份灰掉带原因：副本走「改这份」→ 保存）；
 *   ② 名字由**用户自己起**：抽屉**不预填**；
 *   ③ 复制出来的是**独立的新文件**：新的一行在、旧的一行不动；血统原样带过去
 *      （新那份行上照样「基于旧版官方」）；内容按字节复制（新那份正文里还是改过的字）；
 *   ④ **不碰任何状态**：原来那份照样「已应用」；新那份不会自称使用中；也没把草稿顺走
 *      （再点「改这份」不是「上次改到一半」）；
 *   ⑤ **不覆盖、不自动改名**：起一个已存在的名字会被拒（抽屉里出原因），表里不许悄悄多出东西。
 *
 * 真机上更硬的判据在 Rust 侧：`mine::copy_as_new`（字节复制 / 血统不重算 / 不覆盖 /
 * 不碰状态）+ `copying_touches_no_state_at_all` 那几条。
 */
await rad('preset-kind', 'mkp').click({ force: true })
await rad('preset-scope', 'local').click({ force: true })
await page.waitForTimeout(300)

/* ① 官方那一份：这一项灰掉、带原因 */
const officialTr11 = page
  .locator('main tbody tr')
  .filter({ has: page.locator('td:not([colspan])') })
  .filter({ hasText: 'A1-standard.toml' })
  .first()
await officialTr11.click({ button: 'right' })
await page.waitForTimeout(250)
const copyBoundary = await page.evaluate(() => {
  const ul = document.querySelector('[role="menu"]')
  if (ul === null) return null
  const btn = [...ul.querySelectorAll('[role="menuitem"]')].find(
    (b) => (b.textContent ?? '').trim() === '另存为一份新的',
  )
  return btn === undefined
    ? null
    : { disabled: btn.getAttribute('data-on') !== '1', why: btn.getAttribute('title') ?? '' }
})
console.log(
  `\n[第十一层 · 边界] 官方那份右键「另存为一份新的」：${copyBoundary === null ? '没有这一项' : `灰=${copyBoundary.disabled} 原因「${copyBoundary.why}」`}`,
)
if (copyBoundary === null) problems.push('右键菜单里没有「另存为一份新的」这一项')
else if (!copyBoundary.disabled || !copyBoundary.why.includes('改这份')) {
  problems.push('官方那份的「另存为一份新的」该灰掉并说清走「改这份」→ 保存')
}
await page.keyboard.press('Escape')
await page.waitForTimeout(200)

/* ② 我自己那份：名字不预填；③ 复制出一份新的独立文件 */
const liveTr11 = page
  .locator('main tbody tr')
  .filter({ has: page.locator('td:not([colspan])') })
  .filter({ hasText: '我的 A1 涂胶-高速版.toml' })
  .first()
await liveTr11.click({ button: 'right' })
await page.waitForTimeout(250)
await page.getByRole('menuitem', { name: '另存为一份新的' }).click()
await page.waitForTimeout(300)
const copyDlg = page.getByRole('dialog', { name: '另存为一份新的' })
const copyInput = copyDlg.getByLabel('新的文件名')
const copyPrefill = await copyInput.inputValue()
console.log(
  `[第十一层 · 另存] 输入框初值：${copyPrefill === '' ? '(空 —— 名字由用户自己起)' : `「${copyPrefill}」(不该预填)`}`,
)
if (copyPrefill !== '') problems.push('另存为的名字该由用户自己起（输入框不该预填）')
await copyInput.fill('我的 A1 涂胶-第二份.toml')
await copyDlg.getByRole('button', { name: '另存为' }).click()
await page.waitForTimeout(400)
const afterCopy = await actions()
const copied = afterCopy.find((r) => r.name.includes('我的 A1 涂胶-第二份.toml'))
const original = afterCopy.find((r) => r.name.includes('我的 A1 涂胶-高速版.toml'))
console.log(`[第十一层 · 另存] 新的一份：${copied === undefined ? '(没出现)' : copied.name}`)
console.log(
  `[第十一层 · 另存] 原来那份：${original === undefined ? '(不见了)' : `${original.name} → ${original.action}`}`,
)
if (copied === undefined) {
  problems.push('另存之后新的一份该出现在表里')
} else {
  if (!copied.name.includes('基于旧版官方')) {
    problems.push('血统该原样带过去（新那份行上照样「基于旧版官方」）')
  }
  if (copied.action.includes('已应用')) {
    problems.push('新那份不该自称「使用中」（这一层不碰 Active）')
  }
}
if (original === undefined) {
  problems.push('另存不该动原文件（原来那行该还在）')
} else if (!original.action.includes('已应用')) {
  problems.push(`另存不该断原文件的「已应用」，实测操作列「${original.action}」`)
}

/* ④ 新那份的正文 = 原来那份的字节（5h 改过的字还在）；且不是"接着上次改"（草稿没被顺走） */
const copiedTr = page
  .locator('main tbody tr')
  .filter({ has: page.locator('td:not([colspan])') })
  .filter({ hasText: '我的 A1 涂胶-第二份.toml' })
  .first()
await copiedTr.click()
await page.waitForTimeout(300)
await page.locator('main tbody dl').getByRole('button', { name: '改这份' }).click()
await page.waitForTimeout(400)
const copiedEditor = await page.evaluate(() => {
  const dlg = document.querySelector('[role="dialog"]')
  const area = dlg?.querySelector('textarea')
  return {
    whole: (dlg?.innerText ?? '').replace(/\s+/g, ' ').trim(),
    text: area?.value ?? '',
  }
})
const copiedTextOk = copiedEditor.text.includes('涂胶宽度 = 1.8')
const copiedReused = copiedEditor.whole.includes('上次改到一半')
console.log(
  `[第十一层 · 另存] 新那份正文带着原来改过的字：${copiedTextOk ? '是' : '否'}；接着上次改：${copiedReused ? '是（不该）' : '否'}`,
)
if (!copiedTextOk) problems.push('按字节复制：新那份的正文该还是原来那份的字节（含改过的字）')
if (copiedReused) problems.push('另存不该把原来那份的草稿顺走（新那份是全新的一份）')
await page.keyboard.press('Escape')
await page.waitForTimeout(200)

/* ⑤ 不覆盖、不自动改名：起一个已存在的名字会被拒（抽屉里出原因），表里不许悄悄多出东西 */
await copiedTr.click({ button: 'right' })
await page.waitForTimeout(250)
await page.getByRole('menuitem', { name: '另存为一份新的' }).click()
await page.waitForTimeout(300)
const collisionDlg = page.getByRole('dialog', { name: '另存为一份新的' })
await collisionDlg.getByLabel('新的文件名').fill('我的 A1 涂胶-高速版.toml')
await collisionDlg.getByRole('button', { name: '另存为' }).click()
await page.waitForTimeout(400)
const collisionErr = await collisionDlg.textContent()
const collisionShown = (collisionErr ?? '').includes('已经有一份叫')
console.log(`[第十一层 · 不覆盖] 撞名后的抽屉：${collisionShown ? '出原因了' : '没出原因'}`)
if (!collisionShown) problems.push('撞名该被拒并说清（不覆盖、不自动改名）')
await page.keyboard.press('Escape')
await page.waitForTimeout(200)
const finalRows = await actions()
const collisionCount = finalRows.filter((r) => r.name.includes('我的 A1 涂胶-高速版.toml')).length
console.log(`[第十一层 · 不覆盖] 表里叫这个名字的行：${collisionCount}（该是 1）`)
if (collisionCount !== 1) problems.push('撞名被拒之后表里不许悄悄多出东西')
await page.screenshot({ path: `${shotDir}/presets-mine-copy.png` })

/* ---------- 5l. 第十二层：通用导入入口（选择器 / 拖拽 → 我的文件） ---------- */
/*
 * 守五件事：
 *   ① 工具栏「导入文件…」：选择器（假后端给一条演示路径）→ 结果条说"导入了 1 份" →
 *      新的一份**立刻**出现在「我的文件」里（列表以磁盘为准，不用切页刷新）；
 *   ② 拖进窗口（探针里造一个 File）：重名的那份进**改名那一格**，输入框预填原来那个名字；
 *      改成可用的名字 →「继续导入」→ 进来；原来那份一个字节不动；
 *   ③ 重名不覆盖、不自动改名：指到一个还会撞的名字 → 格子里出原因，表里不许悄悄多出东西；
 *   ④ ZIP 不收：`.zip` 不许被当成预设复制进「我的文件」，结果条如实说"收不了"；
 *   ⑤ 导入不是"安装 Preset"：它不碰「已应用」（原来那份照样是它）。
 *
 * 真机上更硬的判据在 Rust 侧：`runtime::import` 那 8 条（字节复制 / 源文件只读 /
 * 不覆盖 / 不校验 TOML / 不碰任何状态 / 注册表只收 .toml / 落点建目录）。
 */
await rad('preset-kind', 'mkp').click({ force: true })
await rad('preset-scope', 'local').click({ force: true })
await page.waitForTimeout(300)

const bannerText = async () => {
  const raw = await page.locator('[role="status"]').first().textContent()
  return (raw ?? '').replace(/\s+/g, ' ').trim()
}

/* ① 选择器：点「导入文件…」→ 假后端给一条演示路径 → 直接导进来 */
await page.getByRole('button', { name: '导入文件…' }).click()
await page.waitForTimeout(500)
const pickedBanner = await bannerText()
console.log(`\n[第十二层 · 选择器] 结果条：${pickedBanner}`)
if (!pickedBanner.includes('导入了 1 份')) {
  problems.push('选择器导入之后结果条该说「导入了 1 份」')
}
const afterPick = await actions()
const pickedRow = afterPick.find((r) => r.name.includes('从选择器导进来.toml'))
console.log(
  `[第十二层 · 选择器] 新行：${pickedRow === undefined ? '(没出现)' : `${pickedRow.name} → ${pickedRow.action}`}`,
)
if (pickedRow === undefined) {
  problems.push('导入进来的那份该立刻出现在「我的文件」里（列表以磁盘为准）')
} else if (!pickedRow.action.includes('应用')) {
  problems.push('导入进来的那份该是一份正常用户文件（操作列该有「应用」）')
}

/* ② 拖入一份与「我的 A1 涂胶-高速版.toml」同名的：进改名那一格 */
const dropFile = async (name, content) => {
  const dt = await page.evaluateHandle(
    ([n, c]) => {
      const transfer = new DataTransfer()
      transfer.items.add(new File([c], n, { type: 'text/plain' }))
      return transfer
    },
    [name, content],
  )
  await page.dispatchEvent('body', 'dragover', { dataTransfer: dt })
  await page.waitForTimeout(150)
  const veilShown = (await page.getByText('松开：把文件导入').count()) > 0
  await page.dispatchEvent('body', 'drop', { dataTransfer: dt })
  await page.waitForTimeout(500)
  return veilShown
}
const veilShown = await dropFile('我的 A1 涂胶-高速版.toml', '# 拖进来的一份\n"涂胶宽度" = 9.9\n')
console.log(`[第十二层 · 拖入] 拖到窗口上时的提示遮罩：${veilShown ? '出来了' : '没有'}`)
if (!veilShown) problems.push('拖到窗口上该给一句"松开：把文件导入"的提示')
const importDlg = page.getByRole('dialog', { name: '导入：有同名文件' })
const dlgCount = await importDlg.count()
console.log(`[第十二层 · 拖入 · 重名] 改名那一格：${dlgCount > 0 ? '出来了' : '没有'}`)
if (dlgCount === 0) {
  problems.push('拖入重名的那份该开「导入：有同名文件」那一格')
}
const importPrefill = await importDlg
  .getByLabel('新的文件名：我的 A1 涂胶-高速版.toml')
  .inputValue()
console.log(`[第十二层 · 拖入 · 重名] 输入框初值：${importPrefill}`)
if (importPrefill !== '我的 A1 涂胶-高速版.toml') {
  problems.push(`重名格子的输入框该预填原来那个名字，实测「${importPrefill}」`)
}
await importDlg.getByLabel('新的文件名：我的 A1 涂胶-高速版.toml').fill('我的 A1 涂胶-拖进来的.toml')
await importDlg.getByRole('button', { name: '继续导入' }).click()
await page.waitForTimeout(500)
const afterDrag = await actions()
const draggedRow = afterDrag.find((r) => r.name.includes('我的 A1 涂胶-拖进来的.toml'))
const stillThere = afterDrag.find((r) => r.name.includes('我的 A1 涂胶-高速版.toml'))
console.log(
  `[第十二层 · 拖入 · 改名后] ${draggedRow === undefined ? '(没进来)' : draggedRow.name}；原来那份：${stillThere === undefined ? '(不见了！)' : '还在'}`,
)
if (draggedRow === undefined) problems.push('改名之后那份该导进来')
if (stillThere === undefined) problems.push('重名导入不许动原来那份')
if ((await importDlg.count()) > 0) problems.push('全部进来之后那一格该自己关掉')
if (stillThere !== undefined && !stillThere.action.includes('已应用')) {
  problems.push(`导入不该碰「已应用」，实测操作列「${stillThere.action}」`)
}

/* ③ 重名不覆盖、不自动改名：指到一个还会撞的名字，格子里出原因；取消 = 一份都不多 */
await dropFile('我的 A1 涂胶-高速版.toml', '# 再来一次\n')
const retryDlg = page.getByRole('dialog', { name: '导入：有同名文件' })
await retryDlg.getByLabel('新的文件名：我的 A1 涂胶-高速版.toml').fill('我的 A1 涂胶-第二份.toml')
await retryDlg.getByRole('button', { name: '继续导入' }).click()
await page.waitForTimeout(500)
const retryText = ((await retryDlg.textContent()) ?? '').replace(/\s+/g, ' ')
console.log(`[第十二层 · 不覆盖] 撞名后的格子：${retryText.includes('已经有一份叫') ? '出原因了' : '没出原因'}`)
if (!retryText.includes('已经有一份叫')) problems.push('撞名该被拒并说清（不覆盖、不自动改名）')
await retryDlg.getByRole('button', { name: '取消导入' }).click()
await page.waitForTimeout(300)
const afterCancel = await actions()
const secondCopies = afterCancel.filter((r) => r.name.includes('我的 A1 涂胶-第二份.toml'))
console.log(`[第十二层 · 不覆盖] 取消后「第二份」的行数：${secondCopies.length}（该是 1）`)
if (secondCopies.length !== 1) problems.push('取消导入不该在表里多出东西')

/* ④ ZIP 不收：不进「我的文件」，结果条如实说 */
await dropFile('备份.zip', 'PK')
const zipBanner = await bannerText()
console.log(`[第十二层 · ZIP] 结果条：${zipBanner}`)
if (!zipBanner.includes('收不了')) problems.push('ZIP 该被如实拒收（还没有认领它的导入器）')
if ((await actions()).some((r) => r.name.includes('备份.zip'))) {
  problems.push('ZIP 不许被当成预设复制进「我的文件」')
}
await page.screenshot({ path: `${shotDir}/presets-import.png` })

/* ---------- 5m. 第十三层：文件外部管理（在 Finder 中显示） ---------- */
/*
 * 守三件事：
 *   ① 只有「我的文件」这一项能点（官方那份灰掉带原因：它住程序自己管的下载区）；
 *   ② 点它走的是**真的那条动作** —— 浏览器里没有文件管理器、假后端的"文件"也只是内存里
 *      一条，所以假后端如实说这一步在真机上的样子（不假装打开了）；
 *   ③ 它不碰任何状态：点完原来那份照样「已应用」。
 *
 * 真机上更硬的判据在 Rust 侧：`mine::reveal_target`（两道闸 / 读不出来的也能显示 /
 * 外面删了就说找不到）那三条。
 */
await rad('preset-kind', 'mkp').click({ force: true })
await rad('preset-scope', 'local').click({ force: true })
await page.waitForTimeout(300)

/* 菜单里那一项按"…中显示"结尾找 —— 标签按平台换（Finder / 文件资源管理器），动作同一条 */
const revealItemOf = () =>
  page.evaluate(() => {
    const ul = document.querySelector('[role="menu"]')
    if (ul === null) return null
    const btn = [...ul.querySelectorAll('[role="menuitem"]')].find((b) =>
      (b.textContent ?? '').trim().endsWith('中显示'),
    )
    return btn === undefined
      ? null
      : {
          label: (btn.textContent ?? '').trim(),
          disabled: btn.getAttribute('data-on') !== '1',
          why: btn.getAttribute('title') ?? '',
        }
  })

/* ① 官方那份：灰掉带原因 */
const officialTr13 = page
  .locator('main tbody tr')
  .filter({ has: page.locator('td:not([colspan])') })
  .filter({ hasText: 'A1-standard.toml' })
  .first()
await officialTr13.click({ button: 'right' })
await page.waitForTimeout(250)
const officialReveal = await revealItemOf()
console.log(
  `\n[第十三层 · 边界] 官方那份右键「${officialReveal?.label ?? '?'}」：${officialReveal === null ? '没有这一项' : `灰=${officialReveal.disabled} 原因「${officialReveal.why}」`}`,
)
if (officialReveal === null) problems.push('右键菜单里没有「…中显示」这一项')
else if (!officialReveal.disabled || officialReveal.why === '') {
  problems.push('官方那份的「…中显示」该灰掉并说清原因')
}
await page.keyboard.press('Escape')
await page.waitForTimeout(200)

/* ② 我的那份：能点；点了如实说"浏览器里没有文件管理器" */
const mineTr13 = page
  .locator('main tbody tr')
  .filter({ has: page.locator('td:not([colspan])') })
  .filter({ hasText: '我的 A1 涂胶-高速版.toml' })
  .first()
await mineTr13.click({ button: 'right' })
await page.waitForTimeout(250)
const mineReveal = await revealItemOf()
console.log(
  `[第十三层 · 我的文件] 「${mineReveal?.label ?? '?'}」：${mineReveal === null ? '没有这一项' : `灰=${mineReveal.disabled}`}`,
)
if (mineReveal === null || mineReveal.disabled) {
  problems.push('「我的文件」的「…中显示」该能点')
} else {
  await page.getByRole('menuitem', { name: /中显示$/ }).click()
  await page.waitForTimeout(400)
  const revealNote = await page.evaluate(() =>
    (document.querySelector('main [role="status"]')?.innerText ?? '').replace(/\s+/g, ' ').trim(),
  )
  console.log(`[第十三层 · 我的文件] 点完提示条：${revealNote}`)
  if (!revealNote.includes('没打开')) {
    problems.push(`浏览器里没有文件管理器，该如实说「没打开」，实测提示条「${revealNote}」`)
  }
  if (!revealNote.includes('真机上')) {
    problems.push(`该说清这一步在真机上的样子（打开文件管理器并选中），实测「${revealNote}」`)
  }
}

/* ③ 它不碰任何状态：原来那份照样「已应用」 */
const stillActive13 = (await actions()).find((r) => r.name.includes('我的 A1 涂胶-高速版.toml'))
if (stillActive13 === undefined || !stillActive13.action.includes('已应用')) {
  problems.push('「…中显示」不该碰「已应用」')
}
await page.screenshot({ path: `${shotDir}/presets-reveal.png` })

/* ---------- 6. 跨页那一条：BBS 行右键 → 「在 BBS 预设查看器中打开」 ---------- */
/*
 * 这一条量的是**外壳那一层**的接线：点了之后 tab 要切到 BBS。
 * BBS 页本轮还是空态（`PagePlaceholder`），所以落地之后看到的应该是那句空态文案 ——
 * 那也算通过：说明「目标文件名 + 切页」这条意图真的传出去了。
 */
await rad('preset-kind', 'slicer').click({ force: true })
await page.waitForTimeout(200)
await rad('preset-scope', 'local').click({ force: true })
await page.waitForTimeout(250)
const bbsRow = page.locator('main tbody tr').filter({ has: page.locator('td:not([colspan])') }).first()
await bbsRow.click({ button: 'right' })
await page.waitForTimeout(200)
const bbsItem = page.getByRole('menuitem', { name: '在 BBS 预设查看器中打开' })
const bbsCount = await bbsItem.count()
const bbsDisabled = bbsCount > 0 ? await bbsItem.first().getAttribute('data-on') : null
console.log(`\n[BBS 入口] 菜单项 ${bbsCount} 个，data-on=${bbsDisabled}（'1' = 可点）`)
if (bbsCount === 0) {
  problems.push('BBS 行右键没有「在 BBS 预设查看器中打开」')
} else if (bbsDisabled === '1') {
  await bbsItem.first().click()
  await page.waitForTimeout(350)
  const after = await page.evaluate(() => ({
    text: (document.querySelector('main')?.innerText ?? '').replace(/\s+/g, ' ').trim(),
    active: document.querySelector('header nav [aria-current="page"]')?.textContent?.trim() ?? '',
  }))
  console.log(`[BBS 入口] 点完落在 ${after.active}：${after.text.slice(0, 80)}`)
  if (after.active !== 'BBS 预设') problems.push(`点「在 BBS 预设查看器中打开」之后没有切到 BBS（现在是 ${after.active}）`)
} else {
  problems.push('BBS 行的「在 BBS 预设查看器中打开」是灰的（BBS 工艺 profile 应该可点）')
}
await page.getByRole('button', { name: '预设', exact: true }).first().click()
await page.waitForTimeout(300)

/* ---------- 7. 两档尺寸截图 ---------- */
await page.setViewportSize({ width: 1760, height: 900 })
await page.waitForTimeout(300)
await page.screenshot({ path: `${shotDir}/presets-ultra.png` })
console.log(`\n截图 ${shotDir}/presets-ultra.png (1760x900)`)

await page.setViewportSize({ width: 900, height: 640 })
await page.waitForTimeout(400)
await rad('preset-kind', 'mkp').click({ force: true })
await page.waitForTimeout(200)
await rad('preset-scope', 'local').click({ force: true })
await page.waitForTimeout(300)
const compact = await facts()
await page.screenshot({ path: `${shotDir}/presets-compact.png` })
console.log(`截图 ${shotDir}/presets-compact.png (900x640) 行 ${compact.dataRows} 计数 ${compact.counts}`)
console.log(`表头 ${compact.heads.join(' / ')}`)
console.log(`行内容 ${compact.rows.join(' || ') || '(无)'}`)

await browser.close()

if (problems.length > 0) {
  console.log(`\n发现问题 ${problems.length} 条：`)
  for (const p of problems) console.log(`  - ${p}`)
  process.exit(1)
}
console.log(
  '\n预设页：两轴可点、四张表可读、点行展开、右键菜单出得来、交付预设的四态（已下载 / 旧版本 / 内容异常 / 未下载）画得对且动作对、' +
    '我那份能被应用并说得出「基于旧版官方」，改我那份能保存回它自己（不产生第二份、血统还在），' +
    '读不出来的那一份画得出「文件无法读取」且不给应用 / 改这份（第九层），' +
    '我的文件能改名（只动名字、使用中与草稿跟着走）也能删（二次确认；正在使用的不给删）（第十层），' +
    '我的文件能另存为一份新的（字节复制、血统原样、不覆盖、不自动改名、不碰使用中与草稿）（第十一层），' +
    '导入入口（第十二层）：选择器能进、拖入重名进改名格、不覆盖、ZIP 收不了、不碰「已应用」，' +
    '外部管理（第十三层）：右键能在文件管理器里显示「我的文件」（官方那份灰掉带原因、失败如实说、不碰「已应用」），' +
    '控制台没有 error',
)
