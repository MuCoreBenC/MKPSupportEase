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

/* ---------- 5. 交付预设的三态（catalog 登记 + 下载区 mkp/） ---------- */
/*
 * 守的是**状态可见性**：目录里那一份在本机是什么样，页面上要说得对、给的动作要对。
 *
 * 浏览器模式（假后端）给的是固定演示集合（`src/api/mock.ts` 的三个读合起来）：
 * 一份对得上目录、一份对不上、一份还没下过 —— 三态都得画出来。
 * 最容易犯的错是**把「需更新」画成「未下载」**：只看"文件在不在"就会把一份坏档
 * 说成没下过，用户点"下载"以为是第一次下。所以这条单独断言。
 */
await rad('preset-kind', 'mkp').click({ force: true })
await page.waitForTimeout(200)
await rad('preset-scope', 'local').click({ force: true })
await page.waitForTimeout(300)

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

const localActions = await actions()
console.log(`\n[交付三态 · 本地表] ${localActions.map((r) => `${r.name} → ${r.action}`).join(' || ')}`)
const localFast = localActions.find((r) => r.name.includes('A1-fast.toml'))
if (localFast === undefined) {
  problems.push('本地表里没有「需更新」的那一份（盘上确实有它，藏起来就等于说本机没有）')
} else if (!localFast.action.includes('更新')) {
  problems.push(`盘上与目录不符的那一份，本地表的动作该是「更新」，实测「${localFast.action}」`)
}

await rad('preset-scope', 'cloud').click({ force: true })
await page.waitForTimeout(300)
const cloudRows = await actions()
console.log(`[交付三态 · 云端表] ${cloudRows.map((r) => `${r.name} → ${r.action}`).join(' || ')}`)

/*
 * 三态里只断言两态：假后端给的是**两份**固定演示数据（见 `src/api/mock.ts` 那段注释）——
 * 交付构造上每个 (机型, 版本) 只有一份产物，再塞一份同版本的条目就是编形状了。
 * 「未下载」那一档由官方行的「下载」按钮覆盖（同一套动作列），这里不重复量。
 */
const wantState = [
  ['A1-standard.toml', '已下载', '对得上目录的那一份 → 灰字「已下载」，没有可点的动作'],
  ['A1-fast.toml', '更新', '盘上与目录不符的那一份 → 按钮是「更新」，不是「下载」'],
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
console.log(`[交付三态] 点「更新」之后提示条：${note || '(没有提示条)'}`)
if (note === '') problems.push('点「更新」之后没有提示条')
if (note.includes('已更新')) problems.push(`浏览器里没有下载区，不许说「已更新」（实测「${note}」）`)
if (!/失败|没成/.test(note)) problems.push(`点「更新」应当如实报失败，实测提示条是「${note}」`)

/* 展开详情里那句「状态」：三态各自的原话（不是"已应用 / 未应用"那一句） */
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
console.log(`[交付三态] 展开详情「状态」= ${statusFact || '(没有这一格)'}`)
if (!statusFact.includes('需更新')) problems.push(`展开详情的状态该说「需更新」，实测「${statusFact}」`)
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
  '\n预设页：两轴可点、四张表可读、点行展开、右键菜单出得来、交付预设的两态（已下载 / 需更新）画得对且点得动，控制台没有 error',
)
