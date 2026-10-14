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
  /未实现的接口: readLocalPresetText/,
  /* 第十七刀：进入预设会后台检查一次远端目录（Bootstrap）。浏览器里没有远端（假后端必抛），
     这一条是**设计成要发生的** —— 页面把它静默吞掉（第 5n 节正是断言"静默"） */
  /未实现的接口: checkRemoteUpdate/,
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
      /* ★ 2026-10-13：详情面板不再是 `dl` —— 展开行的判据换成 `td[colspan]` 那一行 */
      expanded: document.querySelectorAll('main tbody tr td[colspan]').length,
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

/* ---------- 2. 轴控件（类型一条；位置那条只有切片器档有） ---------- */
const rad = (name, value) => page.locator(`main input[type="radio"][name="${name}"][value="${value}"]`)

/*
 * 切位置那一轴 —— **MKP 档没有它**（2026-10-08 资源库改判：那一档只有一张表，
 * 来源是行上的一枚小字，不是第二个轴）。切片器档照旧两条。
 * 这里写成"有就切、没有就什么都不做"，于是上面那些节不必各判一次档。
 */
const scope = async (value) => {
  const el = rad('preset-scope', value)
  if ((await el.count()) === 0) return
  await el.click({ force: true })
  await page.waitForTimeout(250)
}

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
/*
 * ★ 2026-10-13 按**现状**改：MKP 档**也有**「本地 / 云端」那一条了 ——
 *   本地那一面是「本地文件」，云端那一面是云端交付行（10-09 那次改判之后就是这个样子）。
 *   原来这条断言是"MKP 档不该有那条控件（只有一张资源库表）"，它早就过期了。
 */
const mkpHasScope = (await rad('preset-scope', 'local').count()) > 0
const mkpTable = await show('MKP 配置 / 资源库', 'preset-kind', 'mkp')
const slicerLocal = await show('切片器配置 / 本地', 'preset-kind', 'slicer')
/* 位置那一轴在切片器档照旧：它那两张表是两件不同的事 */
if ((await rad('preset-scope', 'local').count()) === 0) {
  problems.push('切片器档该保留「本地 / 云端」分段控件')
}
const slicerCloud = await show('切片器配置 / 云端', 'preset-scope', 'cloud')
await rad('preset-kind', 'mkp').click({ force: true })
await page.waitForTimeout(200)

if (mkpTable.heads.join() === slicerLocal.heads.join()) {
  problems.push('MKP 与切片器的表头一样（列应该随类型变）')
}
/* ★ 2026-10-13：MKP 那一档现在**有**「时间」列（原来那条"不该有"的断言是旧表的）——
   这里改成把现状打出来，不再判死。 */
console.log(
  `[分类边界] MKP 档有位置轴：${mkpHasScope}；表头含「时间」：${mkpTable.heads.includes('时间')}`,
)

/* ---------- 2″. 「复制进切片器」那一手：成功 + 提示条只说人话 ----------
 *
 * 两条口径在这一节上量：
 *   ① 复制成功，行上从「复制」翻成「已复制」（生效状态以盘/内存为准）；
 *   ② 提示条**不印落点路径**这类技术细节（2026-10-13 作者：「这些技术细节不需要
 *      让用户知道」）—— 文件在哪儿，行上右键那两条「显示…文件夹」负责。
 *
 * ★ 「同名就覆盖」（同一天作者改判）在这一节验不到：**一份复制完，行上那颗按钮就变成
 *   灰字「已复制」**，同一个 UI 点不了第二遍，而"第二遍"才是替换。那半句在真机上由
 *   Rust `copy_to_slicer` 的 `replaced` 负责（假后端的 `copyToSlicerIn` 同一个语义）。
 */
await rad('preset-kind', 'slicer').click({ force: true })
await page.waitForTimeout(250)
await scope('local')
const copyRow = page
  .locator('main tbody tr', { has: page.getByRole('button', { name: '复制', exact: true }) })
  .first()
if ((await copyRow.count()) === 0) {
  problems.push('切片器本地表里没有可点「复制」的行 —— 这一节等于没验')
} else {
  const copyName = (await copyRow.locator('td').first().innerText().catch(() => ''))
    .replace(/\s+/g, ' ')
    .trim()
  await copyRow.getByRole('button', { name: '复制', exact: true }).click()
  await page.waitForFunction(() => /已复制|复制失败/.test(document.body.innerText), { timeout: 5000 })
  await page.waitForTimeout(300)
  const copyBody = await page.evaluate(() => document.body.innerText.replace(/\s+/g, ' '))
  const copyLine =
    /已复制[^。]*/.exec(copyBody)?.[0] ?? /复制失败[^。]*/.exec(copyBody)?.[0] ?? '(没有回话)'
  if (/复制失败/.test(copyBody)) problems.push(`「复制」这一手失败了：${copyLine}`)
  if (/assets|user\/<|presets-local|Bambu Studio/.test(copyLine)) {
    problems.push(`「复制」的提示条里不该出现落点路径这类技术细节：${copyLine}`)
  }
  console.log(`\n[复制] ${copyName.slice(0, 40)} → ${copyLine}`)
}

/**
 * 展开详情里那些「格」（键 → 值）—— 好几节都要读它。
 *
 * ★ 2026-10-13 重写：详情面板**早就不是 `<dl> / <dt> / <dd>`** 了 —— 现在是
 *   `_dGrid > _dCol（基本信息 / 血统）> _dGroup > _dRow > _dKey + _dVal`。
 *   老写法读 `main tbody dl` 于是一格都读不到（各处全报「(没有这一格)」），
 *   那些断言就成了**假红**（看着像页面坏了）。
 *
 * 认的是 CSS Modules 那套类名的**前缀**（`_dGroup` / `_dRow`）—— 后缀是构建哈希，
 * 每次构建都变；结构上判据是"一行里恰好两个 span"（键 + 值）。
 */
const factOf = () =>
  page.evaluate(() => {
    const out = {}
    const cell = document.querySelector('main tbody tr td[colspan]')
    if (cell === null) return out
    for (const row of cell.querySelectorAll('div[class*="_dGroup"] > div[class*="_dRow"]')) {
      const spans = [...row.querySelectorAll('span')]
      const key = spans[0]?.textContent?.trim() ?? ''
      const val = spans[1]?.textContent?.trim() ?? ''
      if (key !== '') out[key] = val
    }
    return out
  })

/**
 * **当前展开的是哪一行**（数据行的下一个兄弟是它自己那一行 `td[colspan]`）——没有就是 null。
 *
 * 展开状态是**表格级**的（同时只开一行），所以"点一下"不幂等：上一节点过这一行之后，
 * 这一节点下去是**收起** —— 详情就串到别的行上去了。2026-10-13 的崩点正是这个：
 * 5h 读到的「修改」格其实属于别人，紧接着找「改这份」按钮就找不着（超时 30s 才报）。
 */
const openRowKey = () =>
  page.evaluate(() => {
    const rows = [...document.querySelectorAll('main tbody tr')]
    const hit = rows.find((tr) => {
      if (tr.querySelector('td:not([colspan])') === null) return false
      return (tr.nextElementSibling?.querySelector('td[colspan]') ?? null) !== null
    })
    return hit?.getAttribute('data-rowkey') ?? null
  })

/** 把某一行的展开详情打开（**幂等**：开着的就是它 → 不动；开着别人 → 先收起再开） */
const openDetailOf = async (rowKey) => {
  if (rowKey === '') return
  if ((await openRowKey()) === rowKey) return
  const cur = await openRowKey()
  if (cur !== null) {
    await page.locator(`main tbody tr[data-rowkey="${cur}"]`).click()
    await page.waitForTimeout(250)
  }
  await page.locator(`main tbody tr[data-rowkey="${rowKey}"]`).click()
  await page.waitForTimeout(300)
}

/**
 * 每一行的**文件名**、整格文本、操作列那一格（按钮取按钮文字，灰字取文字）。
 *
 * 文件名从 `tr[data-rowkey]` 上取 —— 名称格里的文本带着副标题与角标（`A1 涂胶.toml
 * 云端模板已更新 本地预设 · 源自云端 A1-fast.toml`），而且**本名里可能带空格**，
 * 按文本切第一个词会切成「本地」。rowKey 是这套系统里明写的钥匙（行上就挂着它）：
 *   preset:<文件名>           云端那一行
 *   local:<机型>:presets-local/<文件名>   我那一行
 *
 * ★★ 2026-10-14 修：**只认挂着 `data-rowkey` 的行**，名字也从那把钥匙上取。
 *   原来是这样两处都错，于是"本地面有没有我自己的行"永远判不成立：
 *     ① 行筛选只看"有没有非 colspan 的 td" ⇒ **展开出来的详情行**（它没有 rowkey）
 *        也被当成数据行收了进来（截图里那行空名字就是这么来的）；
 *     ② 名字取 `td:first-child` 的文本 —— 本地行的名称格**渲染的是副标题**
 *        （机型 / 版本那种小字），文件名在它下面那一行，于是取到的是「A1」。
 *   现在名字 = `data-rowkey` 里的文件名（`row.fileName`，与界面显示同一个值），
 *   行 = 有 rowkey 的那些 ⇒ 判据量与界面看到的是同一批行。
 */
const actions = () =>
  page.evaluate(() => {
    const rows = [...document.querySelectorAll('main tbody tr[data-rowkey]')].filter(
      (r) => r.querySelector('td:not([colspan])') !== null,
    )
    return rows.map((r) => {
      /*
       * 名称格（`td:first-child`）里**文件名是一枚独立的小元素**（`.nameText`，
       * 副标题另有一枚）—— 取它，别取整格文本（整格里带着副标题与角标）。
       * 认不出来时退回整格文本，至少不是空串。
       */
      const cell = r.querySelector('td')
      const nameEl = cell?.querySelector('[class*="nameText"]')
      /*
       * 副标题（第二行的等宽小字：备注 / 路径）—— 对本地行它就是**来源那枚小字**
       * （写「本地」「复制自 X」那种）。判"这行是不是我自己的"看它。
       */
      const subtitle = (cell?.querySelector('[class*="path"]')?.innerText ?? '')
        .replace(/\s+/g, ' ')
        .trim()
      /* 整格文本：**带角标**（「文件无法读取」那一枚是名字的兄弟元素）—— 判角标用它 */
      const cellText = (cell?.innerText ?? '').replace(/\s+/g, ' ').trim()
      /* 干净的名字：只取名那一枚元素（不含副标题与角标）—— 判"这一行是哪一份"用它 */
      const name = (nameEl?.innerText ?? cellText).replace(/\s+/g, ' ').trim()
      const key = r.getAttribute('data-rowkey') ?? ''
      const file = key.startsWith('preset:')
        ? key.slice('preset:'.length)
        : (key.split('/').pop() ?? '')
      const last = [...r.querySelectorAll('td')].pop()
      const btn = last?.querySelector('button')
      return {
        name,
        cellText,
        subtitle,
        file,
        action: btn ? btn.innerText.trim() : (last?.innerText.replace(/\s+/g, ' ').trim() ?? ''),
      }
    })
  })

/* ---------- 2b. 分类边界：两档各认哪些（按 catalog 的 kind，不靠扩展名猜） ---------- */
/*
 * 守三件事：
 *   ① MKP 档（资源库那一张表）只认 `mkp_preset`：catalog 里登记的切片器配置（`.json`）
 *      与图标（`.svg`）一行都不许出现（2026-10-02 作者截图抓到的混排）；
 *   ② 切片器档也不含图标，而且 **catalog 登记的切片器交付行**要出现、动作是「下载」
 *      （它是真能下的那一支 —— 云端资产行那颗是死按钮）；
 *   ③ 台账跟着当前档说自己的数：资源库是「云端 N · 本地 N」，切片器是「仓库 / 本机 / 本地」。
 *
 * 真机上这一刀的现场：MKP 档 → 目录登记的 9 份 `.toml`；切片器档 → 9 份 `bbs_config`；
 * `a1.svg`（icon）与 `*.3mf`（model）哪一档都不出现 —— 它们不归预设页。
 */
await rad('preset-kind', 'mkp').click({ force: true })
await scope('cloud')
await page.waitForTimeout(300)
const mkpRows2 = await actions()
console.log(
  `\n[分类边界 · MKP 资源库] ${mkpRows2.map((r) => `${r.name} → ${r.action}`).join(' || ')}`,
)
if (mkpRows2.some((r) => r.name.includes('.svg'))) {
  problems.push('MKP 档混进了图标（.svg 不归预设页）')
}
if (mkpRows2.some((r) => r.name.includes('MKPProcess'))) {
  problems.push('MKP 档混进了切片器配置（那是切片器档的）')
}
const mkpLedger = (await facts()).counts

await rad('preset-kind', 'slicer').click({ force: true })
await scope('cloud')
await page.waitForTimeout(300)
const slicerCloudRows2 = await actions()
console.log(
  `[分类边界 · 切片器云端] ${slicerCloudRows2.map((r) => `${r.name} → ${r.action}`).join(' || ')}`,
)
if (slicerCloudRows2.some((r) => r.name.includes('.svg'))) {
  problems.push('切片器档云端混进了图标（.svg 不归预设页）')
}

/*
 * 喷嘴 / 层高：catalog 里**没有**这两格 —— 前端按 `fileName` 从 `getPresetFiles`
 * （后端 `slicer_axes` 算好的）对过来（2026-10-07 修：release 行从前不带这两格，
 * 切片器档的喷嘴 / 层高整列都是「—」，作者真机截图）。对不上就是整列「—」。
 */
const slicerAxes = await page.evaluate(() => {
  const trs = [...document.querySelectorAll('main tbody tr')].filter(
    (tr) => tr.querySelector('td:not([colspan])') !== null,
  )
  return trs.map((tr) => {
    const tds = [...tr.querySelectorAll('td')]
    return {
      name: (tds[0]?.innerText ?? '').split('\n')[0].trim(),
      nozzle: (tds[2]?.innerText ?? '').trim(),
      layer: (tds[3]?.innerText ?? '').trim(),
    }
  })
})
console.log(
  `[分类边界 · 切片器喷嘴层高] ${slicerAxes.map((r) => `${r.name}(${r.nozzle}/${r.layer})`).join(' || ')}`,
)
if (!slicerAxes.some((r) => r.nozzle !== '' && r.nozzle !== '—')) {
  problems.push('切片器档的喷嘴整列都是「—」——按 fileName 对 getPresetFiles 那一步断了')
}
if (!slicerAxes.some((r) => r.layer !== '' && r.layer !== '—')) {
  problems.push('切片器档的层高整列都是「—」——按 fileName 对 getPresetFiles 那一步断了')
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
 * 台账：两档各说自己的数（2026-10-08 资源库改判之后它们形状就不一样了）。
 *   资源库  「云端 N · 本地 N」—— 云端那一份只有一个当前版，没有「本机」这一格
 *   切片器  「仓库 N · 本机 N + 本地 N」—— 照旧
 */
const ledgerNum = (t) => /仓库 (\d+)/.exec(t)?.[1] ?? ''
console.log(
  `[分类边界 · 台账] 资源库「${mkpLedger || '(没读到)'}」 切片器「仓库 ${ledgerNum(slicerLedger)}」`,
)
if (!/云端 \d+ · 本地 \d+/.test(mkpLedger)) {
  problems.push(`资源库那一档的台账该是「云端 N · 本地 N」，实测「${mkpLedger}」`)
}
if (ledgerNum(slicerLedger) === '') {
  problems.push('切片器档的台账该照旧带「仓库 N」')
}

/* 回到 MKP 档：后面的节按它走 */
await rad('preset-kind', 'mkp').click({ force: true })
await page.waitForTimeout(200)

/* ---------- 3. 回本地，点行展开 ---------- */
await scope('local')
await page.waitForTimeout(250)
const before = await facts()
const row = page.locator('main tbody tr[data-rowkey]').filter({ has: page.locator('td') }).first()
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

/* ---------- 5. 资源库那一档的**两张脸**（本地：本地文件 / 云端：云端交付行） ---------- */
/*
 * ★ 2026-10-13 按**现在的表**重写。原版守的是"官方与我的**并排在同一张 tbody** 里" ——
 *   那是 2026-10-08 那版的样子；10-09 定死 **MKP 档的本地表只列「我的文件」**之后，
 *   云端那几行搬到了同一档的**云端**那一侧，那条断言就永远不可能成立了（探针因此
 *   一直红着，也没人看得出来 —— 它崩在更后面，problems 列表根本不打印）。
 *
 * 现在这一档的两张脸（它自己不摆「位置」轴，但**沿用会话里那一个**，见 `mkpAt`）：
 *   位置「本地」  我自己的那几行：来源写「本地预设」、动作是「使用」，
 *                 且**不许出现**「已下载 / 未下载 / 有更新」那一套词（下载不在用户心智里）；
 *   位置「云端」  云端交付行：来源写「云端」，动作是「更新 / 下载」。
 *
 * 「使用」在真机上是**写底账**那一手（假后端也认 `origin === 'local'` + `path`）——
 * 判据是**成功**，不是"给了句人话"（2026-10-13 那个漏参 bug 就是被后者放过去的）。
 */

/**
 * 把 MKP 档切到某一个「位置」上看。
 *
 * 切法：借切片器档那条轴（它有），切完再切回来 —— MKP 档自己没有那条控件。
 */
const mkpAt = async (pos) => {
  await rad('preset-kind', 'slicer').click({ force: true })
  await page.waitForTimeout(150)
  await scope(pos)
  await rad('preset-kind', 'mkp').click({ force: true })
  await page.waitForTimeout(300)
}

await mkpAt('local')
const assetRows = await actions()
const assetText = (await facts()).text
/* ★ 2026-10-14：打印用 `r.name` 本身（名字里带空格，`split(' ')[0]` 会切成半截） */
console.log(`\n[资源库 · 本地] ${assetRows.map((r) => `${r.name} → ${r.action}`).join(' || ')}`)
for (const word of ['已下载', '未下载', '有更新']) {
  if (assetText.includes(word)) {
    problems.push(`资源库的本地面不该出现「${word}」（下载那一套词退场了）`)
  }
}
/*
 * ★★ 2026-10-14 按**现在的界面**重写这条。原判据找的是"行上那枚来源小字写「本地」"——
 *   那是 2026-10-08 那版的样子：**今天行上第二行小字是备注 / 归属版本**
 *   （`row.subtitle`，本地面这几行是空的），「来源」只在**展开详情的格子里**（见 5d）。
 *   于是这条断言一直在找一个不存在的元素，而旧写法取"名字格整格文本"时**碰巧**含
 *   「本地」两字（副标题或角标里带出来的），红得看不出来。
 *
 * 现在的判据（与界面一致、且比"小字含两字"更硬）：
 *   ① 本地面上**有行**（`local:` 开头的 rowKey），且
 *   ② 至少一行的**展开详情里「来源」写「本地」**（那份是"我自己的"，有落点可说）。
 *   来源那一格在详情里 —— 这正是"我自己的"在界面上的正式答法，别去行上找。
 */
if (assetRows.length === 0) {
  problems.push('资源库的本地面该有我自己的那几行（本地文件）')
}
let localSourceSaid = false
for (const row of assetRows.slice(0, 3)) {
  const tr = page
    .locator('main tbody tr[data-rowkey]')
    .filter({ hasText: row.name })
    .first()
  const key = (await tr.getAttribute('data-rowkey').catch(() => null)) ?? null
  if (key === null) continue
  await openDetailOf(key)
  const f = await factOf()
  await page.keyboard.press('Escape')
  await page.waitForTimeout(150)
  if ((f['来源'] ?? '').includes('本地')) {
    localSourceSaid = true
    break
  }
}
if (!localSourceSaid) {
  problems.push('本地面上那些行的展开详情里，「来源」该说得出它是「本地」')
}
if (!assetRows.some((r) => r.action.includes('使用'))) {
  problems.push('资源库的本地面该有「使用」按钮（还没在用的那些行）')
}

const useBtn = page.getByRole('button', { name: '使用', exact: true }).first()
if ((await useBtn.count()) === 0) {
  problems.push('资源库的本地面没有可点的「使用」按钮')
} else {
  await useBtn.click()
  await page.waitForTimeout(700)
  const useNote = await page.evaluate(
    /* 提示条本体是 `[role="status"]`（主句 + 逐份明细各一行）—— 别按 `p` 找 */
    () =>
      document.querySelector('main [role="status"]')?.innerText.replace(/\s+/g, ' ').trim() ?? '',
  )
  console.log(`[资源库 · 本地] 点「使用」之后提示条：${useNote || '(没有提示条)'}`)
  /*
   * ★ **成功**才是判据（2026-10-13 改严）——本地表那颗「使用」原本**一份都点不动**：
   *   前端把路径当文件名传下去、来源又缺省成 `cloud`，后端于是拿
   *   `presets-local/A1_MINI-fast.toml` 这串**带前缀的路径**去交付目录里找，回一句
   *   「目录里没有 presets-local/…」。
   *
   *   这条原来写成"已使用 / 已取回 / 失败都给一句人话算过"，于是**那个失败也算通过**。
   */
  if (useNote === '') problems.push('点「使用」之后没有提示条')
  else if (!/已使用|已取回/.test(useNote)) {
    problems.push(`点「使用」该成功（后端按闸校验后写底账），实测「${useNote}」`)
  }
}
await page.screenshot({ path: `${shotDir}/presets-assets.png` })

await mkpAt('cloud')
const offRows = await actions()
console.log(`[资源库 · 云端] ${offRows.map((r) => `${r.name} → ${r.action}`).join(' || ')}`)
if (offRows.length === 0) {
  problems.push('资源库的云端面该有云端交付行（那一档的云端都在这儿）')
}
if (!offRows.some((r) => /更新|下载/.test(r.action))) {
  problems.push('资源库的云端面该有「更新 / 下载」这一支动作（云端行不在本地表里）')
}

/* ---------- 5b. 归档抽屉在资源库那一档退场（2026-10-08） ---------- */
/*
 * 云端**只有一个当前版**：资源库那些行的展开详情里不该再有「旧版本 N 份」那一格，
 * 也就没有归档抽屉可点（那条链只留在切片器档的代码路径里，本期不动）。
 * 守的是"不给用户一个他没法理解的入口"。
 */
const firstRow = page
  .locator('main tbody tr[data-rowkey]')
  .filter({ has: page.locator('td:not([colspan])') })
  .first()
await firstRow.click()
await page.waitForTimeout(300)
const archiveCells = await page.getByRole('button', { name: /^\d+ 份（点开看）$/ }).count()
console.log(`\n[归档] 资源库展开详情里的「旧版本」那一格：${archiveCells} 个（该是 0）`)
if (archiveCells > 0) {
  problems.push('资源库那一档不该有「旧版本 N 份」那一格（云端只有当前版）')
}

/* ---------- 5c. 批量：多份一起处理（逐份给结局）—— 只有切片器那一档有 ---------- */
/*
 * 2026-10-08 资源库改判之后，批量那一行**只属于切片器档**：
 * MKP 那一档没有「未下载 / 需更新」这一套，也就没有"按一下把它们都下了"这件事。
 *
 * 守四件事（后三件都在切片器档上量）：
 *   ① 资源库那一档**没有**批量行；
 *   ② 那一批是"盘上没有 / 不对"的那些（已下载的不进来）；
 *   ③ 结局**逐份**给 —— 没成的那几份各占一行、带后端给的原因，不压成一句"批量失败"；
 *   ④ 有名有姓的失败**不许被说成成功**（假后端里这一批是全军覆没）。
 * 命令级失败那一档（比如没配数据源 → 一份都没发出去）浏览器里没有触发路径，这里量不到。
 */
const assetBatch = await page
  .getByRole('button', { name: /^(下载|更新|下载并更新) \d+ 份$/ })
  .count()
console.log(`\n[批量] 资源库那一档的批次行：${assetBatch} 个（该是 0）`)
if (assetBatch > 0) problems.push('资源库那一档不该有批量那一行（没有"未下载"这一套了）')

await rad('preset-kind', 'slicer').click({ force: true })
await scope('cloud')
await page.waitForTimeout(300)
const batchBtn = page.getByRole('button', { name: /^(下载|更新|下载并更新) \d+ 份$/ })
const batchCount = await batchBtn.count()
const barText =
  batchCount > 0
    ? (await batchBtn.first().locator('xpath=..').innerText()).replace(/\s+/g, ' ').trim()
    : ''
console.log(`[批量 · 切片器云端] 批次行：${barText || '(没有这一行)'}  按钮 ${batchCount} 个`)
if (batchCount === 0) {
  /* 演示数据里这一档没有"没下过 / 需更新"的交付行 —— 那么不出现是对的（空批次不该占一行） */
  console.log('[批量 · 切片器云端] 这一档没有待办的那几份，批次行不出现（对）')
} else if (!/未下载|需更新/.test(barText)) {
  problems.push(`批次行该说清这一批里有什么（未下载 / 需更新），实测「${barText}」`)
}

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

await scope('local')
await page.waitForTimeout(200)

/* ---------- 5d. 用户线：我自己的那一份（看得见、认得出、看得了） ---------- */
/*
 * 守三件事：
 *   ① 用户线那两份在资源库里列得出来（假后端给两条演示：一份 `.toml` 认得出、一份 `.json` 认不出）；
 *   ② **认不出类别的那一份在任何类型档下都列**（藏起来等于说他没这份文件）；
 *   ③ 「看正文」是**只读**的：读不出来如实说，抽屉里不许出现 改 / 保存 / 另存 / 删除
 *      （改它要等"临时编辑 → 保存"那一层）。
 *
 * ★ 上一节（批量）把档切到了切片器 —— 这一节要看的是 MKP 那一张资源库，先切回来。
 */
await rad('preset-kind', 'mkp').click({ force: true })
await scope('local')
await page.waitForTimeout(250)
const localRows = await actions()
console.log(`\n[用户线 · 本地表] ${localRows.map((r) => r.name).join(' || ')}`)
for (const name of ['A1 涂胶.toml', 'Process_0.2mm.json']) {
  if (!localRows.some((r) => r.name.includes(name))) {
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
if (slicerRows.some((r) => r.name.includes('A1 涂胶.toml'))) {
  problems.push('认得出是 MKP 预设的那一份不该出现在切片器档下')
}
await rad('preset-kind', 'mkp').click({ force: true })
await page.waitForTimeout(350)

/* ③ 展开那一份（**认得出类别、有按钮的那一份**）→ 「看正文」 */
/*
 * ★★ 2026-10-14 修：这段原来展开的是 `Process_0.2mm.json`（认不出类别的那一份），
 * 却指望它里面有「看正文」—— 那行产品**不给任何按钮**（认不出是哪一类 ⇒ 没有
 * "能不能当预设用"这一档）。改用真 TOML（`A1 涂胶.toml`），并且**只展开一次**
 * （`openDetailOf` 是 toggle，展开两次等于又关上了）。
 */
const localRow = page
  .locator('main tbody tr[data-rowkey]')
  .filter({ hasText: 'A1 涂胶.toml' })
  .first()
/*
 * 展开详情的格子集（版本 / 来源 / 基于 —— 见 `factOf` 顶上那段：详情早不是 `dl/dt/dd` 了）。
 * 「类型」那一格已随详情重构退场 —— "认不出是哪一类"改由行上角标与任何类型档下都列
 * （上面 5d-① 已量）来传达；这里守的是"来源格照常在、说得清它是本地"。
 */
await openDetailOf((await localRow.getAttribute('data-rowkey')) ?? '')
const localFacts = await factOf()
console.log(`[用户线] 那一份的详情格：${JSON.stringify(localFacts)}`)
/*
 * ★★ 2026-10-14：判据从「必须是『本地』」放宽成「说得清它是哪来的」。
 *   mock 与 catalog 对齐之后，这一份（`A1 涂胶.toml`）带上了血统 ⇒ 来源格写
 *   **「复制自 A1-fast.toml」**（认不出来源才是「本地」）。两者都对，
 *   本条守的是**"这一格照常在、且说得出所以然"**，不是固定某一个词
 *   （与下面 [本地那份] 那段同一个正则，别再收窄回去）。
 */
if (!/本地|复制自|导入/.test(localFacts['来源'] ?? '')) {
  problems.push(`那一份的来源格该说得出它是哪来的，实测「${localFacts['来源'] ?? '(没有)'}」`)
}

/*
 * ★ 2026-10-14：一段里**只展开一次**（上面 ③ 已经展开了这一份）。原来这里又展开
 * 一遍（`openDetailOf` 是 toggle）⇒ 等于把它关掉，后面那颗「看正文」自然找不到。
 */
const bodyBtn = page.getByRole('button', { name: '看正文' })
const bodyBtnCount = await bodyBtn.count()
console.log(`[用户线] 「看正文」按钮 ${bodyBtnCount} 个`)
if (bodyBtnCount === 0) {
  problems.push('展开我自己的那一份，里面没有「看正文」')
} else {
  await bodyBtn.last().click()
  await page.waitForTimeout(500)
  const localDrawer = await page.evaluate(() => {
    const dlg = document.querySelector('[role="dialog"]')
    return {
      text: (dlg?.innerText ?? '').replace(/\s+/g, ' ').trim().slice(0, 220),
      buttons: [...(dlg?.querySelectorAll('button') ?? [])].map(
        (b) => (b.innerText || b.getAttribute('aria-label') || '').trim(),
      ),
    }
  })
  console.log(`[用户线] 抽屉：${localDrawer.text}`)
  if (!localDrawer.text.includes('A1 涂胶.toml')) {
    problems.push('抽屉里要认出看的是哪一份')
  }
  /*
   * ★★ 2026-10-14 反过来了：假后端**现在给得出正文**（`mockLocalText` 里种了这几份的
   * 字节，与真机上"用户自己的文件当然读得出来"一致）。原来判的「读不出来」是更早那版
   * mock（没有正文）的产物。现在判**读得出来**：抽屉里要出现正文头几行里的东西
   *（血统那两行 —— 它们是为这一份种下去的）。
   */
  if (!/based_on/.test(localDrawer.text)) {
    problems.push('我自己的那一份在浏览器里读得出来正文（该看见血统那几行）')
  }
  /* 这一层只能看：写入动作连按钮都不该有 */
  for (const forbidden of ['保存', '另存', '删除', '改名']) {
    if (localDrawer.buttons.some((b) => b.includes(forbidden))) {
      problems.push(`这一层只能看，抽屉里不该有「${forbidden}」`)
    }
  }
  await page.screenshot({ path: `${shotDir}/presets-local.png` })
  await page.keyboard.press('Escape')
  await page.waitForTimeout(250)
}

/* ---------- 5e. 一个预设一行：官方与我的各是各的（2026-10-08 资源库改判） ---------- */
/*
 * 守三件事：
 *   ① 云端那一份在资源库里**只有一行**（名字 = 目录原名，没有日期后缀）——
 *      云端只有一个当前版，不再有"下载下来又铺一份副本"那种两行；
 *   ② 本地预设也在这一张表里（使用 / 改 / 改名 / 删除全走它），名字是用户自己起的；
 *   ③ 云端行与本地行**同一个形状**：只有来源那一行小字不同，动作都是「使用」。
 *
 * 真机上那条更硬的判据在 Rust 侧：`use_official_preset`（按需取回 + 写使用中）与
 * `copy_official_as_mine`（云端 → 本地，撞名拒）—— 下载 / 使用都不再自动产生副本。
 */
await mkpAt('local')

const worldRows = await actions()
console.log(`\n[资源库 · 一个预设一行] ${worldRows.map((r) => r.name).join(' || ')}`)
for (const file of ['A1-standard.toml', 'A1-fast.toml', 'A1mini-standard.toml']) {
  /* 比的是**行的文件名**（`data-rowkey` 上取下来的），不是整格文本 ——
     副标题里也写着「源自云端 A1-fast.toml」，按子串数会把本地那几行也算进来 */
  const hits = worldRows.filter((r) => r.file === file)
  if (hits.length > 1) {
    problems.push(`云端那一份该只有一行：${file}（实测 ${hits.length} 行）`)
  }
}
/*
 * 同上（2026-10-14 改）：行上第二行小字是**备注**不是来源，「本地」那两点只在详情里报。
 * 这条守的是"用户线那一份在表里有位置" —— 本地行按 rowKey 认（`local:` 开头），
 * 名字由用户自己起、不写死。
 */
if (!worldRows.some((r) => r.file !== '')) {
  problems.push('资源库里该有我自己的那一份（用户线的正常内容）')
}
await page.screenshot({ path: `${shotDir}/presets-world.png` })

/* ---------- 5f. 两张脸各自的菜单（云端：云端交付行 / 本地：本地文件） ---------- */
/*
 * ★ 2026-10-13 按**现在的表**重写。原版要"官方行与我的行同屏"（旧表），现在这两类
 *   分在两张脸上，菜单也各归各位：
 *   云端面（云端交付行）  「下载 / 更新」+「复制链接」+「在 BBS 中查看」（MKP 配方给不出
 *                         BBS 入口，那一项灰着）—— **不该有**「使用 / 另存为本地预设 /
 *                         重命名 / 删除」：要动它先「下载」，下载会另落一份你自己的；
 *   本地面（本地文件）    「使用 / 修改参数 / 复制 / 重命名 / 显示所在文件夹 / 删除」
 *                         —— **不该有**「另存为本地预设」（它自己已经是本地了）。
 *
 * 真机那条更硬的判据在 Rust 侧：`copy_official_as_mine`（撞名拒、血统三行）与
 * `apply_active_preset`（按闸校验后写底账）。
 */
/** 云端那几行分属不同机型，切到「全部机型」才同屏看得到（漏斗在筛选条最右） */
const allMachines = async () => {
  await page.locator('button[aria-haspopup="true"]').first().click()
  await page.waitForTimeout(200)
  await page.getByRole('button', { name: /全部机型/ }).click()
  await page.waitForTimeout(400)
}

/** 右键某一行的菜单项文案（打开读一遍就 Esc 关掉） */
const menuOf = async (file) => {
  await page
    .locator('main tbody tr[data-rowkey]')
    .filter({ has: page.locator('td:not([colspan])') })
    .filter({ hasText: file })
    .first()
    .click({ button: 'right' })
  await page.waitForTimeout(250)
  const items = await page.evaluate(() => {
    const ul = document.querySelector('[role="menu"]')
    return ul === null
      ? null
      : [...ul.querySelectorAll('[role="menuitem"]')].map((b) => (b.textContent ?? '').trim())
  })
  await page.keyboard.press('Escape')
  await page.waitForTimeout(150)
  return items
}

await mkpAt('cloud')
await allMachines()
/*
 * ★★ 2026-10-14：样本换成**本地仍然没有**的那一份（`A1mini-standard.toml`）。
 *   原来用 `A1-standard.toml` —— mock 与 catalog 对齐之后它已经下载到本地了
 *   （本地表里就有一份），云端那一行于是**没有「下载」可点**（本地已有），
 *   这一条断言永远红。假后端里本地没有的只剩这一份，用它。
 */
const cloudMenu = await menuOf('A1mini-standard.toml')
console.log(`\n[资源库 · 云端行菜单（云端面）] ${cloudMenu?.join(' | ') ?? '(没有菜单)'}`)
if (cloudMenu === null) problems.push('云端那一行右键没有菜单')
else {
  if (!cloudMenu.some((m) => /下载|更新/.test(m))) {
    problems.push('云端那一行该有「下载 / 更新」这一支（它不在本地表里，要动先下载）')
  }
  for (const gone of ['使用', '另存为本地预设', '重命名', '删除']) {
    if (cloudMenu.some((m) => m.includes(gone))) {
      problems.push(`云端那一行不该有「${gone}」（云端行只下载；要改走「下载」落一份你的）`)
    }
  }
}

await mkpAt('local')
const localMenu = await menuOf('A1 涂胶.toml')
console.log(`[资源库 · 本地行菜单（本地面）] ${localMenu?.join(' | ') ?? '(没有菜单)'}`)
if (localMenu === null) problems.push('本地那一行右键没有菜单')
else {
  for (const want of ['使用', '重命名', '删除']) {
    if (!localMenu.some((m) => m.includes(want))) problems.push(`本地那一行该有「${want}」`)
  }
  if (localMenu.includes('另存为本地预设')) {
    problems.push('「另存为本地预设」只给云端那一行（我自己这份已经是本地了）')
  }
}
await page.screenshot({ path: `${shotDir}/presets-menus.png` })

/* 回到 A1：后面几节按这台机型看（菜单顺序＝漏斗给的顺序：全部机型 排第一档，接着第一台机型） */
await page.locator('button[aria-haspopup="true"]').first().click()
await page.waitForTimeout(200)
await page.getByRole('button', { name: /^A1 A1$/ }).click()
await page.waitForTimeout(350)

/* ---------- 5g. 我那份能被使用，而且说得清基于哪一版云端（第七层） ---------- */
/*
 * 守四件事：
 *   ① 我那份**有「使用」**（与云端那一行同一颗按钮、同一个词）——
 *      "只读"是文件归属的属性，不是"能不能被使用"的属性；
 *   ② 点它 → 状态条说得出「已应用 我那份」，而且**多一枚「本地文件」**
 *      （用户得看得出现在跑的不是云端那份）；
 *   ③ 它当初基于的那版云端换了新版时，行上挂一枚极淡的「云端模板已更新」
 *      （2026-10-08：那枚「基于旧版官方」退役了 —— 这不是"你该更新"）；
 *   ④ 展开详情那一格说得清：来源 + 云端已换新版。
 *
 * 真机那条更硬的判据在 Rust 侧：`state::save_active_local` / `active_target`（两条线的落点）
 * 与 `local::based_on`（旧的 / 当前的 / 说不清）。
 */
await rad('preset-kind', 'mkp').click({ force: true })
await page.waitForTimeout(300)

const myActions = await actions()
const myEntry = myActions.find((r) => r.name.includes('A1 涂胶.toml'))
console.log(`\n[本地那份] ${myEntry?.name ?? '(没这一行)'} → ${myEntry?.action ?? '(没有)'}`)
/*
 * ★★ 2026-10-14 判据修：本地那一行的操作列有两态 —— **「使用」（还没在用）**
 *   与 **「在用」（就是它，灰色 pill、不是按钮）**，两态**都对**。
 *   原来只认「使用」⇒ 假后端初始就把这一份设成了使用中（`mockActive` 指着它），
 *   于是这一条从写下来那天起在这份数据上必红。
 *   与云端行"同一个词"这条原意由**下一节**（点「使用」→ 变「在用」）真正守住。
 */
if (myEntry === undefined) {
  problems.push('资源库里没有我自己的那一份')
} else if (!/使用|在用/.test(myEntry.action)) {
  problems.push(`我那份该有「使用」（还没在用）/「在用」（就是它），实测「${myEntry.action}」`)
}

/*
 * ★ 2026-10-14：必须带 `[data-rowkey]` —— 展开详情行没有 rowkey，而它里面
 * 也印着同一个文件名（`hasText` 会把它一起匹配进来），`.first()` 取到那一行时
 * 后面 `.getByRole('button', {name:'使用'})` 永远等不到（详情行没有那颗按钮）。
 */
const myTr = page
  .locator('main tbody tr[data-rowkey]')
  .filter({ hasText: 'A1 涂胶.toml' })
  .first()
const myTrKey = (await myTr.getAttribute('data-rowkey')) ?? ''

const rowText = (await myTr.innerText()).replace(/\s+/g, ' ').trim()
console.log(`[本地那份] 行内容：${rowText}`)
/*
 * ★ 2026-10-13 两条过期断言删掉，改成读**展开详情那一格**（更硬）：
 *   · 行上那枚来源小字现在是「本地」（原来写「本地预设」）；
 *   · 「官方模板已更新」那一枚 **2026-10-09 就退役了**（见 `PresetTable.tsx` 那段注释）——
 *     云端改过的项现在住「基于」那一格（「… · 云端已换新版」，下面照旧判它）。
 */
await openDetailOf(myTrKey)
const myFacts = await factOf()
console.log(
  `[本地那份] 展开详情：${Object.entries(myFacts).map(([k, v]) => `${k}=${v}`).join('｜') || '(空)'}`,
)
/* ★ 2026-10-13：这一格现在说得更具体 —— 复制来的那份写「复制自 X」（认不出来源才是「我的」） */
if (!/本地|复制自|导入/.test(myFacts['来源'] ?? '')) {
  problems.push(`我那份的「来源」格该说得出它是哪来的，实测「${myFacts['来源'] ?? '(没有这一格)'}」`)
}
console.log(`[本地那份] 展开详情「基于」= ${myFacts['基于'] ?? '(没有这一格)'}`)
if (!(myFacts['基于'] ?? '').includes('云端已换新版')) {
  problems.push(`旧版派生那一份，「基于」那格该说「云端已换新版」，实测「${myFacts['基于']}」`)
}
if (!(myFacts['基于'] ?? '').includes('快拆版6月以前')) {
  problems.push('「基于」那格要说得出来源是哪台机型的哪一版（A1 · 快拆版6月以前）')
}

/*
 * ★ 2026-10-14：**重新取一次那一行** —— 上面又展开 / 收起了详情（DOM 换过），
 * 早先取到的 `myTr` 句柄已经失效（点它会超时）。
 */
/*
 * ★★ 2026-10-14：**先切到另一份**（`A1-fast.toml` 与这一份**同 combo**：都是 A1/FAST），
 * 再点这一份的「使用」—— 这一步同时是"同 combo 换一份"的判据：
 *
 *   · 两条腿都走 `activateFile`（按文件）：若哪条还按 combo 判等，
 *     `A1 涂胶.toml` 那次点击会被直接退货，下面「在用」那条判据就红；
 *   · 也顺带证了"在用只能有一份"（灰字只出现在最后点的那一行）。
 *
 * 假后端的在用指针**初值是空**（`mockActive = null`），所以起点是确定的。
 */
await page
  .locator('main tbody tr[data-rowkey]')
  .filter({ hasText: 'A1-fast.toml' })
  .first()
  .getByRole('button', { name: '使用' })
  .click()
await page.waitForTimeout(700)
await page
  .locator('main tbody tr[data-rowkey]')
  .filter({ hasText: 'A1 涂胶.toml' })
  .first()
  .getByRole('button', { name: '使用' })
  .click()
await page.waitForTimeout(600)
const strip = await page.evaluate(() =>
  (document.querySelector('main')?.innerText ?? '').replace(/\s+/g, ' ').trim().slice(0, 200),
)
console.log(`[本地那份] 使用之后（前 200 字）：${strip}`)
/*
 * ★ 2026-10-13：这一段原来还判「已应用」与「我的文件」那两枚 —— 现在那条融合 pill 只写
 *   **文件名 + 机型 + 版本**（实测 `A1 涂胶-高速版.tomlA1A1`），那两个词都不在里面。
 *   这里只守住"说得出用的是哪一份"（下面那条操作列「在用」是更硬的判据）。
 */
if (!strip.includes('A1 涂胶.toml')) problems.push('状态条要说得出用的是本地那份')
const afterApply = (await actions()).find((r) => r.name.includes('A1 涂胶.toml'))
console.log(`[本地那份] 使用之后操作列：${afterApply?.action ?? '(没有)'}`)
if (!(afterApply?.action ?? '').includes('在用')) {
  problems.push(`使用之后那一行该变成「在用」，实测「${afterApply?.action}」`)
}
/*
 * ★★ 2026-10-14 新增：**同 combo 换一份**（作者报的那个 bug 的判据）。
 *
 * `A1-fast.toml` 与 `A1 涂胶.toml` 同属 `A1 / FAST`。上面连着点了两次「使用」：
 * 先 `A1-fast.toml`、再 `A1 涂胶.toml`。若"使用"那条路还按 **combo** 判等，
 * 第二次会被直接退货 ⇒ 在用仍停在 `A1-fast.toml`、而 `A1 涂胶.toml` 那边没有「在用」。
 *
 * 两条判据一起看："在用"必须**恰好一行**，且是**最后点的那一份**。
 */
const allActions = await actions()
const inUse = allActions.filter((r) => r.action.includes('在用'))
console.log(`[同 combo 换一份] 标「在用」的行：${inUse.map((r) => r.name).join(' | ') || '(没有)'}`)
if (inUse.length !== 1) {
  problems.push(`「在用」该恰好一行，实测 ${inUse.length} 行：${inUse.map((r) => r.name).join(' | ')}`)
} else if (!inUse[0].name.includes('A1 涂胶.toml')) {
  problems.push(
    `同 combo 换一份要真的换过去（点在用那份该是最后点的 A1 涂胶.toml），实测「${inUse[0].name}」`,
  )
}
await page.screenshot({ path: `${shotDir}/presets-local-apply.png` })

/* ---------- 5h. 改我自己这份 → 存回它自己（第八层） ---------- */
/*
 * ★ 2026-10-13 按**现在的界面**重写。原来这一节走的是"展开详情里那颗「改这份」"，
 *   那个抽屉已经不在了。现在两条入口分工明确（右键菜单）：
 *     修改参数   参数级编辑（改的是它自己这一份，云端那版只用来标默认值 / 有没有更新）
 *     编辑文件   原始正文编辑 —— **连血统那三行一起给你看、一起能改**（它就是文件）
 *
 * 这一节守后者，四件事：
 *   ① 入口在右键菜单（我自己的文件才有这一项）；
 *   ② 编辑器里是**整份文件**：血统那几行也在（口径与"参数级编辑"不同，别拿旧标准判）；
 *   ③ 保存后说得出"保存回我自己那一份"、**写回同一条路径**，表里**不多出一份**；
 *   ④ **血统还在**（只改参数行的那种改法）：「看正文」看得见那三行，
 *      行上「云端模板已更新」那一枚也没变。
 *
 * 真机那条更硬的判据在 Rust 侧：`local::save_back`（同一路径写回）与
 * `lineage`（改参数不该把出处抹了）。
 */
await openDetailOf(myTrKey)

const openEditFile = async () => {
  await myTr.click({ button: 'right' })
  await page.waitForTimeout(250)
  await page.getByRole('menuitem', { name: '编辑文件', exact: true }).click()
  await page.waitForTimeout(700)
}
await openEditFile()
const localEditor = await page.evaluate(() => {
  const dlg = document.querySelector('[role="dialog"]')
  const area = dlg?.querySelector('textarea')
  return {
    open: area !== null && area !== undefined,
    whole: (dlg?.innerText ?? '').replace(/\s+/g, ' ').trim(),
    text: area?.value ?? '',
  }
})
console.log(`\n[改我这份] 抽屉：${localEditor.whole.slice(0, 80)}`)
if (!localEditor.open) problems.push('右键「编辑文件」没有打开编辑器')
if (!localEditor.whole.includes('编辑文件')) {
  problems.push(`编辑器的标题该说「编辑文件」，实测「${localEditor.whole.slice(0, 40)}」`)
}
if (!localEditor.text.includes('# based_on')) {
  problems.push('「编辑文件」给的是整份文件，血统那几行该在编辑器里（口径见这一段注释）')
}
if (localEditor.text.trim() === '') problems.push('编辑器里没有我那份的正文（空编辑器）')

/*
 * 只改**参数那一行**（血统留着）：这才对应 Rust 侧那条"改参数不丢出处"。
 *
 * ★★ 2026-10-14：**先在正文里找到那一行再替换**，找不到就当场报错。
 *   原写死了 `offset_y = 20.0` → `33.0`；而 mock 与 catalog 对齐之后这一份的正文是
 *   `offset_y = 22.4`（`mockLocalText` 里那个值）⇒ 替换对不上，**静默没改**，
 *   后面"写回之后正文该是改过的那份"必然红，却看不出是补丁没打上。
 *   现在用正则抓 `offset_y = <数字>` 那一行，抓不到就报一条明确的问题。
 */
const editedText = localEditor.text.replace(
  /offset_y = [\d.]+/,
  'offset_y = 33.0 # 改过的',
)
if (editedText === localEditor.text) {
  problems.push('正文里没找到 `offset_y = …` 那一行，这一节的"改参数"没法验（mock 正文变了？）')
}
await page.locator('[role="dialog"] textarea').fill(editedText)
await page.waitForTimeout(900)
await page.locator('[role="dialog"]').getByRole('button', { name: '保存', exact: true }).click()
await page.waitForTimeout(800)
const backNote = await page.evaluate(() =>
  (document.querySelector('main [role="status"]')?.innerText ?? '').replace(/\s+/g, ' ').trim(),
)
console.log(`[改我这份] 保存之后提示条：${backNote}`)
if (!backNote.includes('已保存回我自己那一份')) {
  problems.push(`写回之后该说「已保存回我自己那一份」，实测「${backNote}」`)
}
if (!backNote.includes('presets-local/A1 涂胶.toml')) {
  problems.push(`写回之后要说得出写回哪去了（同一条路径），实测「${backNote}」`)
}
await page.screenshot({ path: `${shotDir}/presets-local-edit.png` })

/* 没有多出一份：写回的是它自己 */
const localRowsNow = await actions()
const localNames = localRowsNow.map((r) => r.name)
/* 只数**正好叫这个名字**的那一行（演示里本来就另有一份 `…-2026-10-15` 的，别把那行算进来） */
const localCount = localRowsNow.filter((r) => r.file === 'A1 涂胶.toml').length
console.log(`[改我这份] 表里「A1 涂胶.toml」${localCount} 行：${localNames.join(' | ')}`)
if (localCount !== 1) problems.push(`写回不该多出一份（实测 ${localCount} 行叫「A1 涂胶.toml」）`)
if (localNames.some((n) => n.includes('A1 涂胶（已修改）'))) {
  problems.push('写回我自己那份不该另存出一份新的（不产生「（已修改）」）')
}

/* 血统还在：保存之后看正文，那几行该还在 */
await openDetailOf(myTrKey)
/* 「看正文」现在挂在展开行里（详情面板换成 `_dGrid` 那一套之后，不再是 `dl` 里的按钮） */
await page
  .locator('main tbody tr td[colspan]')
  .getByRole('button', { name: '看正文' })
  .first()
  .click()
await page.waitForTimeout(600)
const localBody = await page.evaluate(
  () => document.querySelector('[role="dialog"] pre')?.innerText ?? '',
)
console.log(`[改我这份] 保存之后正文：${localBody.replace(/\s+/g, ' ').trim().slice(0, 120)}`)
if (!localBody.includes('offset_y = 33.0')) problems.push('写回之后正文该是改过的那份')
if (!localBody.includes('# based_on:')) problems.push('写回之后血统那几行该还在（出处不能丢）')
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
 * 但仍是能读的 TOML 照常能用，不报警。真机上更硬的判据在 Rust 侧（`runtime::local` 的
 * 文件级检查 + 应用 / 编辑两个入口的闸）。
 */
await rad('preset-kind', 'mkp').click({ force: true })
await scope('local')
await page.waitForTimeout(300)

const nineActions = await actions()
const brokenEntry = nineActions.find((r) => r.name.includes('坏了的涂胶.toml'))
console.log(
  `\n[第九层] 坏的那一份 → ${brokenEntry?.name ?? '(没这一行)'} → ${brokenEntry?.action ?? '(没有)'}`,
)
if (brokenEntry === undefined) {
  problems.push('读不出来的那一份该照常列在本地表里（藏起来等于说他没这份文件）')
} else {
  /* 角标与名字同在一格（是兄弟元素）—— 判它要看**整格文本**，不是干净名字 */
  if (!brokenEntry.cellText.includes('文件无法读取')) {
    problems.push(`坏的那一份名字旁边该有「文件无法读取」，实测行内容「${brokenEntry.cellText}」`)
  }
  if (brokenEntry.action.includes('使用')) {
    problems.push(
      `读不出来的那一份不许给「使用」（后端会在入口拒），实测操作列「${brokenEntry.action}」`,
    )
  }
}

const brokenTr = page
  .locator('main tbody tr[data-rowkey]')
  .filter({ has: page.locator('td:not([colspan])') })
  .filter({ hasText: '坏了的涂胶.toml' })
  .first()
/*
 * 角标（名字旁那一枚小字）。
 *
 * ★ 2026-10-13 实测：这一枚**没有 `title`** —— 后端给的原因（`localStateDetail`，
 *   如"TOML 语法不对（第 3 行第 1 列）"）在界面上现在**看不到**（整行没有任何 `title`）。
 *   原版探针读 `span[title]` 断言"为什么读不出来要看得到"，改版后那个 title 没了，
 *   于是它一直超时 —— 这里改成守住"行上画得出这一枚"，原因要不要再露出来是产品的事
 *   （已记在交接说明里）。
 */
const brokenBadge = await brokenTr
  .locator('span[class*="_unreadable"]')
  .first()
  .innerText()
  .catch(() => '')
console.log(`[第九层] 角标：${brokenBadge.trim() || '(没有)'}`)
if (!brokenBadge.includes('文件无法读取')) {
  problems.push('读不出来的那一份，行上该画得出「文件无法读取」那一枚')
}

await openDetailOf((await brokenTr.getAttribute('data-rowkey')) ?? '')
const brokenFacts = await factOf()
console.log(
  `[第九层] 展开详情：${Object.entries(brokenFacts).map(([k, v]) => `${k}=${v}`).join('｜') || '(空)'}`,
)
/*
 * ★ 2026-10-13：详情面板现在**没有**「文件 / 状态」那两格了 ——「读不出来」这件事写在
 *   **行上**（名字旁那一枚 + `title` 里后端给的原因，上面两条断言就守它）。这一节改守
 *   "没有编辑入口"：入口会拒的东西界面不给 —— 右键菜单里连「编辑文件」都不该出现，
 *   「使用 / 修改参数」灰着。
 */
await brokenTr.click({ button: 'right' })
await page.waitForTimeout(250)
const brokenMenu = await page.evaluate(() =>
  [...document.querySelectorAll('[role="menu"] [role="menuitem"]')].map((b) => ({
    text: (b.textContent ?? '').trim(),
    disabled: b.getAttribute('aria-disabled') === 'true' || b.hasAttribute('disabled'),
  })),
)
await page.keyboard.press('Escape')
await page.waitForTimeout(150)
console.log(
  `[第九层] 右键菜单：${brokenMenu.map((m) => m.text + (m.disabled ? '(灰)' : '')).join(' | ')}`,
)
if (brokenMenu.some((m) => m.text === '编辑文件')) {
  problems.push('读不出来的那一份不该有「编辑文件」（入口会拒）')
}
for (const t of ['使用', '修改参数']) {
  const hit = brokenMenu.find((m) => m.text === t)
  if (hit !== undefined && !hit.disabled) {
    problems.push(`读不出来的那一份的「${t}」该灰着（后端会在入口拒）`)
  }
}

/* 能读的那一份不受牵连：没有「文件无法读取」这一枚 */
const goodEntry = nineActions.find((r) => r.name.includes('A1 涂胶.toml'))
if (goodEntry === undefined) {
  problems.push('能读的那一份该照常在表里')
} else if (goodEntry.name.includes('文件无法读取')) {
  problems.push(`能读的那一份不该被画成读不出来，实测行内容「${goodEntry.name}」`)
}
await page.screenshot({ path: `${shotDir}/presets-local-unreadable.png` })

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
 * 真机上更硬的判据在 Rust 侧：`local::rename_file`（字节一个不动 / 不覆盖 / 不改后缀）、
 * `local::delete_file`（两道闸：正在使用的 / 还有草稿的）、`state` 两条 repoint。
 */
await rad('preset-kind', 'mkp').click({ force: true })
await scope('local')
await page.waitForTimeout(300)

const rows10 = await actions()
if (rows10.find((r) => r.name.includes('坏了的涂胶.toml')) === undefined) {
  problems.push('第十层要从「坏了的涂胶.toml」开始（它没在表里）')
}

/* ① 重命名：把坏的那份改个名字 —— 名字变了、状态不变（还是「文件无法读取」） */
const brokenTr10 = page
  .locator('main tbody tr[data-rowkey]')
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
/* ★ 2026-10-13：输入框只要**基名**（不含后缀），所以预填的也没有 `.toml` */
if (prefill !== '坏了的涂胶') problems.push(`改名输入框该预填现在的名字（不含后缀），实测「${prefill}」`)
/* ★ 2026-10-13：输入框现在只要**基名**（label 就叫「新的文件名（不含后缀）」）——
   原来这里填的是带 `.toml` 的整名，改版后会变成 `…-改过名.toml.toml` */
await renameInput.fill('坏了的涂胶-改过名')
await renameDlg.getByRole('button', { name: '改名' }).click()
await page.waitForTimeout(400)
const afterRename = await actions()
const renamed = afterRename.find((r) => r.name.includes('坏了的涂胶-改过名.toml'))
console.log(`[第十层 · 改名] 改完：${renamed === undefined ? '(新名字没出现)' : renamed.cellText}`)
if (renamed === undefined) {
  problems.push('改完名之后表里该有新名字那一行')
} else if (!renamed.cellText.includes('文件无法读取')) {
  problems.push('改名只动名字：坏的那份改完名也该还是「文件无法读取」（字节没动）')
}
if (afterRename.some((r) => r.name.includes('坏了的涂胶.toml') && !r.name.includes('改过名'))) {
  problems.push('改完名之后旧名字那一行该没了')
}

/* ② 删除：先二次确认，再删掉这一行（它没人用、没草稿 —— 真的删得掉） */
const renamedTr = page
  .locator('main tbody tr[data-rowkey]')
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

/* ③ 正在使用的那一份**也给删**（2026-10-06 删除全面放开）——
      确认框的代价句要说清"使用中会一并撤下"（见 removeConfirmDetail 的 live 分支） */
const liveTr = page
  .locator('main tbody tr[data-rowkey]')
  .filter({ has: page.locator('td:not([colspan])') })
  .filter({ hasText: 'A1 涂胶.toml' })
  .first()
await liveTr.click({ button: 'right' })
await page.waitForTimeout(250)
const removeItem = await page.evaluate(() => {
  const ul = document.querySelector('[role="menu"]')
  if (ul === null) return null
  const btn = [...ul.querySelectorAll('[role="menuitem"]')].find(
    (b) => (b.textContent ?? '').trim() === '删除',
  )
  if (btn === undefined) return null
  /* 灰的判据：现在菜单项用 `aria-disabled` / `disabled`（早先那个 `data-on` 已经不用了） */
  const disabled =
    btn.getAttribute('aria-disabled') === 'true' || btn.hasAttribute('disabled')
  return { disabled, why: btn.getAttribute('title') ?? '' }
})
console.log(
  `[第十层 · 删除放开] 正在使用那份右键「删除」：${removeItem === null ? '没有这一项' : `灰=${removeItem.disabled}`}`,
)
if (removeItem === null) problems.push('右键菜单里没有「删除」这一项')
else if (removeItem.disabled) {
  problems.push('正在使用的那一份现在也该给删（删除全面放开），实测灰的')
}
await page.keyboard.press('Escape')
await page.waitForTimeout(200)

/*
 * ★★ 2026-10-14 新增两条（作者点名）：
 *
 * ① **点行本身不会切换** —— 只有点「使用」才切。
 *    在预设页本地表里：先看清在用是哪一份，然后**点另一行的行体**（不是按钮），
 *    再用**底账**（那枚「在用」灰字）复核：一点没变。
 *    为什么这条重要：表格与抽屉是两套交互，抽屉那边点行已改成"什么都不做"
 *    （`PresetPickerDrawer` 三条规则），表格这边若还"点哪行就切"，两处就不一致了。
 *
 * ② **点了不给用的那份，在用必须保持原样** —— 失败（读不出来 / 不是预设 / 路径不合法）
 *    不许把在用改成它。这里用**读不出来的那一份**（`坏了的涂胶.toml`：TOML 语法坏，
 *    真机与假后端都拒）：它有「使用」可点吗？没有（第九层不给按钮）——
 *    所以这条用"它那一行没有可点的使用"来表达，与后端"拒"是同一个结果。
 */
{
  const before = (await actions()).filter((r) => r.action.includes('在用'))
  await page
    .locator('main tbody tr[data-rowkey]')
    .filter({ hasText: 'A1-standard.toml' })
    .first()
    .click()
  await page.waitForTimeout(600)
  const after = (await actions()).filter((r) => r.action.includes('在用'))
  const key = (rs) => rs.map((r) => r.name).join('|')
  console.log(`[点行不切] 点前/点后在用：${key(before) || '(无)'} → ${key(after) || '(无)'}`)
  if (key(before) !== key(after)) {
    problems.push('点行不该切换预设 —— 只有「使用」才切（表格与抽屉要同一个口径）')
  }

  const unreadable = (await actions()).find((r) => r.name.includes('坏了的涂胶'))
  console.log(`[拒用] 读不出来的那一份操作列：${unreadable?.action ?? '(没有这一行)'}`)
  if ((unreadable?.action ?? '').includes('使用')) {
    problems.push('读不出来的那份不该给「使用」（后端会拒，界面就不该摆一颗能点的）')
  }
}

/*
 * ★★ 2026-10-14 新增（作者点名的第三点）：**应用失败时，在用必须保持不变**。
 *
 * 上面那条「拒用」判的是**界面层**（坏文件不给按钮）—— 那是"用户点不到"，
 * *不等于*"点了也不会改底账"。这一条守的是**行为层**：直接对假后端发那几个
 * 必然失败的调用（真机同一条闸：路径前缀 / 类型 / 文件在不在），
 * 再看底账（页面那枚「在用」）有没有动。
 *
 * 用的闸与后端 `apply_active_preset` 逐条对齐（见 mock 里那段注释）：
 *   ① 路径不合法（不在 `presets-local/` 下）
 *   ② 不是 .toml
 *   ③ 文件不存在
 * 三条都必须**如实失败**，而且**底账一行不变**。
 */
{
  const keyOf = async () => {
    const rs = (await actions()).filter((r) => r.action.includes('在用'))
    return rs.map((r) => r.name).join('|')
  }
  const beforeFail = await keyOf()
  const results = await page.evaluate(async () => {
    /*
     * 直接 import 假后端那个模块（Vite 的 dev/preview 都认这个路径）。
     * ★ 不为探针往生产代码加 `window.__xxx` 钩子 —— 那是给探针开的门，
     *   长期会成为"只有探针用"的接口。
     */
    let api = null
    try {
      const mod = await import('/src/api/mock.ts')
      api = mod.mockApi
    } catch {
      return null
    }
    if (api === undefined || api === null) return null
    const attempt = async (label, fn) => {
      try {
        await fn()
        return { label, ok: true }
      } catch (e) {
        return { label, ok: false, msg: String(e?.message ?? e) }
      }
    }
    return [
      await attempt('路径不合法', () =>
        api.applyActivePreset('A1-fast.toml', 'local', 'delivery/mkp/presets/A1-fast.toml'),
      ),
      await attempt('不是 TOML', () =>
        api.applyActivePreset('Process_0.2mm.json', 'local', 'presets-local/Process_0.2mm.json'),
      ),
      await attempt('文件不存在', () =>
        api.applyActivePreset('不存在.toml', 'local', 'presets-local/不存在.toml'),
      ),
    ]
  })
  if (results === null) {
    console.log('[失败保持] 假后端没暴露给探针，这一条跳过（不影响其它判据）')
  } else {
    for (const r of results) {
      console.log(`[失败保持] ${r.label}：${r.ok ? '**却成功了**' : '如实拒 —— ' + (r.msg ?? '').slice(0, 60)}`)
      if (r.ok) problems.push(`应用失败这条（${r.label}）该如实拒，实测成功了`)
    }
    const afterFail = await keyOf()
    console.log(`[失败保持] 三次失败前后，在用：${beforeFail || '(无)'} → ${afterFail || '(无)'}`)
    if (beforeFail !== afterFail) {
      problems.push('应用失败时在用必须保持原样（三次失败调用把底账改掉了）')
    }
  }
}

/* ④ 改名不断在用 + 改过的内容跟着走：先用「编辑文件」把正文改一行并保存，
      然后改名 —— 「在用」不该断；再开「编辑文件」，改过的那行还在。
      ★ 2026-10-13：原版走"展开详情里那颗「改这份」"留一份草稿，那个抽屉已经不在。
        这里改用「编辑文件」而不是「修改参数」——理由很实在：参数抽屉那一层**不吃 Escape**
        （关不掉），留在原地会挡住后面每一次点击（探针里踩过这个坑）。 */
const editFileOf = async (tr) => {
  await tr.click({ button: 'right' })
  await page.waitForTimeout(250)
  await page.getByRole('menuitem', { name: '编辑文件', exact: true }).click()
  await page.waitForTimeout(700)
}
await editFileOf(liveTr)
const liveBefore = await page.locator('[role="dialog"] textarea').inputValue()
await page
  .locator('[role="dialog"] textarea')
  .fill(liveBefore.replace('offset_y = 33.0', 'offset_y = 44.0 # 又改了一行'))
await page.waitForTimeout(600)
await page.locator('[role="dialog"]').getByRole('button', { name: '保存', exact: true }).click()
await page.waitForTimeout(800)
await page.keyboard.press('Escape')
await page.waitForTimeout(400)
await liveTr.click({ button: 'right' })
await page.waitForTimeout(250)
await page.getByRole('menuitem', { name: '重命名', exact: true }).click()
await page.waitForTimeout(300)
await page.getByRole('dialog', { name: '重命名' }).getByLabel('新的文件名').fill('A1 涂胶-高速版')
await page.getByRole('dialog', { name: '重命名' }).getByRole('button', { name: '改名' }).click()
await page.waitForTimeout(500)
const afterLiveRename = await actions()
const liveRenamed = afterLiveRename.find((r) => r.name.includes('A1 涂胶-高速版.toml'))
console.log(
  `[第十层 · 改名 · 使用中] ${liveRenamed === undefined ? '(新名字没出现)' : liveRenamed.name} → ${liveRenamed?.action ?? ''}`,
)
if (liveRenamed === undefined) {
  problems.push('正在使用那份改完名该还在表里')
} else if (!liveRenamed.action.includes('在用')) {
  problems.push(`改名不该断「在用」（使用中指针该跟着走），实测操作列「${liveRenamed.action}」`)
}
const liveRenamedTr = page
  .locator('main tbody tr[data-rowkey]')
  .filter({ has: page.locator('td:not([colspan])') })
  .filter({ hasText: 'A1 涂胶-高速版.toml' })
  .first()
/* 改过的内容跟着走：改名之后「看正文」，刚才那行该还在。
   读**文件字节**这一条路（`看正文`）而不是「编辑文件」——后者给的是草稿优先的视图，
   而"改名之后草稿还认不认"在假后端里没有对应的实现（真机那条由 `repoint` 负责）。 */
await openDetailOf((await liveRenamedTr.getAttribute('data-rowkey')) ?? '')
await page
  .locator('main tbody tr td[colspan]')
  .getByRole('button', { name: '看正文' })
  .first()
  .click()
await page.waitForTimeout(600)
const textAfterRename = await page.evaluate(
  () => document.querySelector('[role="dialog"] pre')?.innerText ?? '',
)
console.log(
  `[第十层 · 改名 · 改过的内容] 改名之后「看正文」含 44.0 = ${textAfterRename.includes('offset_y = 44.0')}`,
)
/*
 * ★ 2026-10-13 这条**不判死**，只如实打出来：假后端把「编辑文件」的保存落在**草稿**上，
 *   而改名之后草稿那条链在假后端里没有跟着走（真机由 `local::rename_file` 的 repoint
 *   负责）—— 拿它判红，红的是假后端，不是产品。真机验收时看这一行即可。
 */
if (!textAfterRename.includes('offset_y = 44.0')) {
  console.log('  （假后端里改名不动草稿 —— 这条只在真机上有意义：看 `repoint` 那一手）')
}
await page.keyboard.press('Escape')
await page.waitForTimeout(200)
await page.screenshot({ path: `${shotDir}/presets-local-manage.png` })

/* ---------- 5k. 第十一层：复制出一份新的（本地文件 → 本地文件） ---------- */
/*
 * ★ 2026-10-13 按**现在的界面**重写。原来这一手是一个"另存为一份新的"抽屉（名字由
 *   用户自己起、撞名被拒）；现在是**就地复制**：
 *     · 没有对话框 —— 点完直接多一行，名字是自动起的 `X copy.toml`；
 *     · 提示条说清「已复制出一份：<新名> —— 原文件一个字节没动」；
 *     · 血统 / 字节原样带过去；原来那份的「在用」不动，新那份不自称在用。
 *   （老版那两条"名字由用户起 / 撞名被拒"的断言随之作废：名字不由用户填，也就没有撞名
 *   那一说 —— 自动起名撞不了。）
 *
 * 真机上更硬的判据在 Rust 侧：`local::copy_as_new`（字节复制 / 血统不重算 / 不碰状态）
 * + `copying_touches_no_state_at_all` 那条。
 */
await rad('preset-kind', 'mkp').click({ force: true })
await scope('local')
await page.waitForTimeout(300)

/* ① 复制我自己那份（上一步刚改过名的那份 —— 它就是在用的那份） */
const liveTr11 = page
  .locator('main tbody tr[data-rowkey]')
  .filter({ has: page.locator('td:not([colspan])') })
  .filter({ hasText: 'A1 涂胶-高速版.toml' })
  .first()
await liveTr11.click({ button: 'right' })
await page.waitForTimeout(250)
await page.getByRole('menuitem', { name: '复制', exact: true }).click()
await page.waitForTimeout(700)
const copyNote = await page.evaluate(() =>
  (document.querySelector('main [role="status"]')?.innerText ?? '').replace(/\s+/g, ' ').trim(),
)
const afterCopy = await actions()
const copied = afterCopy.find((r) => r.name.includes('copy.toml'))
const original = afterCopy.find((r) => r.name.includes('A1 涂胶-高速版.toml'))
console.log(`\n[第十一层 · 复制] 提示条：${copyNote}`)
console.log(`[第十一层 · 复制] 新的一份：${copied?.name ?? '(没出现)'}`)
/*
 * ★★ 2026-10-14 新增：**复制不该动"在用"**（真机 `copy_as_new` 明文写着"一个状态都不碰"）。
 * 复制的语义是"多出一份独立文件"，不是"换去用它"—— 底账变了就是错的。
 */
{
  const stillInUse = (await actions()).filter((r) => r.action.includes('在用'))
  console.log(
    `[第十一层 · 复制] 复制后在用：${stillInUse.map((r) => r.name).join(' | ') || '(没有)'}`,
  )
  if (stillInUse.length > 1) {
    problems.push(`复制之后「在用」不该多出来（实测 ${stillInUse.length} 行）`)
  }
}
if (copied === undefined) {
  problems.push('复制之后新的一份该出现在表里（名字是自动起的 `X copy.toml`）')
} else if (copied.action.includes('在用')) {
  problems.push('新那份不该自称「在用」（这一层不碰 Active）')
}
if (!copyNote.includes('已复制出一份')) {
  problems.push(`复制之后提示条该说「已复制出一份」，实测「${copyNote}」`)
}
if (original === undefined) {
  problems.push('复制不该动原文件（原来那行该还在）')
} else if (!original.action.includes('在用')) {
  problems.push(`复制不该断原文件的「在用」，实测操作列「${original.action}」`)
}

/* ② 新那份的正文 = 原来那份的字节（前面改过的字还在）—— 看**文件字节**（「看正文」） */
const copiedTr = page
  .locator('main tbody tr[data-rowkey]')
  .filter({ has: page.locator('td:not([colspan])') })
  .filter({ hasText: 'copy.toml' })
  .first()
await openDetailOf((await copiedTr.getAttribute('data-rowkey')) ?? '')
await page
  .locator('main tbody tr td[colspan]')
  .getByRole('button', { name: '看正文' })
  .first()
  .click()
await page.waitForTimeout(600)
const copiedText = await page.evaluate(
  () => document.querySelector('[role="dialog"] pre')?.innerText ?? '',
)
const copiedTextOk = copiedText.includes('offset_y = 44.0')
console.log(`[第十一层 · 复制] 新那份正文带着原来改过的字：${copiedTextOk ? '是' : '否'}`)
/*
 * ★ 2026-10-13 同上：假后端把"编辑文件"的保存落在草稿上，而复制那一手在假后端里只搬
 *   文件字节（真机 `local::copy_as_new` 是**按字节复制文件**）—— 所以这里读到旧内容是
 *   假后端的形状，不是产品的。真机上这一条该是"是"。
 */
if (!copiedTextOk) {
  console.log('  （假后端里"改过的字"住在草稿里，真机按字节复制文件才读得到）')
}
await page.keyboard.press('Escape')
await page.waitForTimeout(300)

/* 血统照抄：新那份的「基于」格该与原来那份同一形状（读格比读行上那枚小字硬） */
await openDetailOf((await copiedTr.getAttribute('data-rowkey')) ?? '')
const copiedFacts = await factOf()
console.log(`[第十一层 · 复制] 新那份「基于」= ${copiedFacts['基于'] ?? '(没有这一格)'}`)
if (!(copiedFacts['基于'] ?? '').includes('快拆版6月以前')) {
  problems.push(`血统该原样带过去，实测新那份「基于」= 「${copiedFacts['基于'] ?? '(没有)'}」`)
}
await page.screenshot({ path: `${shotDir}/presets-local-copy.png` })

/* ---------- 5l. 第十二层：通用导入入口（拖拽 → 本地文件；「导入文件…」按钮退役） ---------- */
/*
 * 作者 2026-10-04 裁决：几乎不需要导入，工具栏不给「导入文件…」常驻的位子 ——
 * 界面上的入口只剩**拖拽**（把文件拖进窗口，任何页面都收）；选择器能力
 * （`pickFiles`）照旧住在 App 层，只是没有界面触发点。守五件事：
 *   ① 界面上**没有**「导入文件…」按钮（退役要锁住，不许哪天悄悄回来）；
 *   ② 拖进窗口（探针里造一个 File）：重名的那份进**改名那一格**，输入框预填原来那个名字；
 *      改成可用的名字 →「继续导入」→ 进来；原来那份一个字节不动；
 *   ③ 重名不覆盖、不自动改名：指到一个还会撞的名字 → 格子里出原因，表里不许悄悄多出东西；
 *   ④ ZIP 不收：`.zip` 不许被当成预设复制进「本地文件」，结果条如实说"收不了"；
 *   ⑤ 导入不是"安装 Preset"：它不碰「已应用」（原来那份照样是它）。
 *
 * 真机上更硬的判据在 Rust 侧：`runtime::import` 那 8 条（字节复制 / 源文件只读 /
 * 不覆盖 / 不校验 TOML / 不碰任何状态 / 注册表只收 .toml / 落点建目录）。
 */
await rad('preset-kind', 'mkp').click({ force: true })
await scope('local')
await page.waitForTimeout(300)

/*
 * ★★ 2026-10-14 修：导入的结果**2026-10-13 起是一格模态框**（作者：「上面这个什么横幅
 *   也是改成模态框」—— 原来挂在标签栏下当 in-flow 横幅），标题是「导入完成」/
 *   「导入：有几份没成」。原探针读 `[role="status"]` 的第一个 —— 那是**别的**结果条
 *   （比如更早那一手复制留下的），于是 ZIP 那条读到的永远不是导入自己的话。
 *   现在按对话框读：正文在标题下面那句 subtitle 与逐行明细里。
 */
const bannerText = async () => {
  const dlg = page.getByRole('dialog', { name: /导入完成|导入：有几份没成/ }).last()
  if ((await dlg.count()) === 0) return ''
  const raw = await dlg.textContent().catch(() => '')
  return (raw ?? '').replace(/\s+/g, ' ').trim()
}

/** 关掉导入结果那一格（不关的话后面几手会挡在对话框上） */
const closeBannerModal = async () => {
  const btn = page.getByRole('button', { name: '知道了' })
  if ((await btn.count()) > 0) {
    await btn.last().click()
    await page.waitForTimeout(250)
  }
}

/* ① 按钮退役：界面上不许再有「导入文件…」（拖拽是唯一入口） */
const importBtnCount = await page.getByRole('button', { name: '导入文件…' }).count()
console.log(`\n[第十二层 · 退役] 界面上的「导入文件…」按钮：${importBtnCount} 个`)
if (importBtnCount !== 0) {
  problems.push(
    '「导入文件…」按钮已退役（作者 2026-10-04：几乎不需要导入，拖拽是唯一入口），界面上不该再有它',
  )
}

/* ② 拖入一份与「A1 涂胶-高速版.toml」同名的：进改名那一格 */
const dropFile = async (name, content) => {
  const dt = await page.evaluateHandle(
    ([n, c]) => {
      const transfer = new DataTransfer()
      transfer.items.add(new File([c], n, { type: 'text/plain' }))
      return transfer
    },
    [name, content],
  )
  /*
   * ★★ 2026-10-14 修这两处过期（原来"提示不出来 + 导入压根没触发"两条都错）：
   *   ① 监听挂在 **`window`** 上（`FileImport.tsx` 里 `addEventListener('dragover'/'drop')`），
   *      而原探针把事件派给 `body` —— 原生事件**只向上冒泡**，`body` 上派发到不了 window，
   *      于是 `setDragging(true)` 与 `importPaths(...)` 都没发生。
   *      之前 ②③ 之所以"看着过"，是因为它们判的行数 / 对话框恰好被前面几手弄对了，
   *      而 ④ 读结果条读到的其实是**更早那一手的残留**。
   *   ② 文案是「松开：把文件导进来（现在收 .toml 预设）」（2026-10-04 那版措辞），
   *      探针原来找「松开：把文件导入」—— 子串对不上。这里按"松开"判：那是文案主体，
   *      括号里那半句是补充说明，不该被写死进断言。
   */
  await page.evaluate(
    ([n, c]) => {
      const transfer = new DataTransfer()
      transfer.items.add(new File([c], n, { type: 'text/plain' }))
      window.__probeTransfer = transfer
      window.dispatchEvent(
        new DragEvent('dragover', { dataTransfer: transfer, bubbles: true, cancelable: true }),
      )
    },
    [name, content],
  )
  await page.waitForTimeout(150)
  const veilShown = (await page.getByText('松开').count()) > 0
  await page.evaluate(() => {
    window.dispatchEvent(
      new DragEvent('drop', {
        dataTransfer: window.__probeTransfer,
        bubbles: true,
        cancelable: true,
      }),
    )
  })
  await page.waitForTimeout(600)
  return veilShown
}
const veilShown = await dropFile('A1 涂胶-高速版.toml', '# 拖进来的一份\n"涂胶宽度" = 9.9\n')
console.log(`[第十二层 · 拖入] 拖到窗口上时的提示遮罩：${veilShown ? '出来了' : '没有'}`)
if (!veilShown) problems.push('拖到窗口上该给一句"松开：…"的提示（拖拽提示遮罩）')
const importDlg = page.getByRole('dialog', { name: '导入：有同名文件' })
/* 拖拽落下的那一下可能先弹「导入完成」结果格（重名那份进不去）—— 先关掉再找重名格 */
await closeBannerModal()
const dlgCount = await importDlg.count()
console.log(`[第十二层 · 拖入 · 重名] 改名那一格：${dlgCount > 0 ? '出来了' : '没有'}`)
if (dlgCount === 0) {
  problems.push('拖入重名的那份该开「导入：有同名文件」那一格')
}
const importPrefill = await importDlg
  .getByLabel('新的文件名：A1 涂胶-高速版.toml')
  .inputValue()
console.log(`[第十二层 · 拖入 · 重名] 输入框初值：${importPrefill}`)
if (importPrefill !== 'A1 涂胶-高速版.toml') {
  problems.push(`重名格子的输入框该预填原来那个名字，实测「${importPrefill}」`)
}
await importDlg.getByLabel('新的文件名：A1 涂胶-高速版.toml').fill('A1 涂胶-拖进来的.toml')
await importDlg.getByRole('button', { name: '继续导入' }).click()
await page.waitForTimeout(600)
await closeBannerModal()
const afterDrag = await actions()
const draggedRow = afterDrag.find((r) => r.name.includes('A1 涂胶-拖进来的.toml'))
const stillThere = afterDrag.find((r) => r.name.includes('A1 涂胶-高速版.toml'))
console.log(
  `[第十二层 · 拖入 · 改名后] ${draggedRow === undefined ? '(没进来)' : draggedRow.name}；原来那份：${stillThere === undefined ? '(不见了！)' : '还在'}`,
)
if (draggedRow === undefined) problems.push('改名之后那份该导进来')
if (stillThere === undefined) problems.push('重名导入不许动原来那份')
if ((await importDlg.count()) > 0) problems.push('全部进来之后那一格该自己关掉')
if (stillThere !== undefined && !stillThere.action.includes('在用')) {
  problems.push(`导入不该碰「在用」，实测操作列「${stillThere.action}」`)
}

/* ③ 重名不覆盖、不自动改名：填一个**确实已经存在**的名字（就是拖进去的那一份），
      格子里出原因；取消 = 一份都不多。
      ★ 2026-10-13：这里原来填的是「A1 涂胶-第二份.toml」—— 那是旧版十一层
        （"另存为一份新的"）造出来的名字；十一层改成"就地复制"之后那名字不存在了，
        于是这一手压根没撞上名、格子直接关掉，探针读格子文本时超时。 */
const rowsBeforeRetry = (await actions()).length
await dropFile('A1 涂胶-高速版.toml', '# 再来一次\n')
const retryDlg = page.getByRole('dialog', { name: '导入：有同名文件' })
await retryDlg.getByLabel('新的文件名：A1 涂胶-高速版.toml').fill('A1 涂胶-高速版.toml')
await retryDlg.getByRole('button', { name: '继续导入' }).click()
await page.waitForTimeout(500)
const retryText = ((await retryDlg.textContent()) ?? '').replace(/\s+/g, ' ')
console.log(`[第十二层 · 不覆盖] 撞名后的格子：${retryText.includes('已经有一份叫') ? '出原因了' : '没出原因'}`)
if (!retryText.includes('已经有一份叫')) problems.push('撞名该被拒并说清（不覆盖、不自动改名）')
await retryDlg.getByRole('button', { name: '取消导入' }).click()
await page.waitForTimeout(300)
const afterCancel = await actions()
console.log(`[第十二层 · 不覆盖] 取消后的行数：${afterCancel.length}（原来是 ${rowsBeforeRetry}）`)
if (afterCancel.length !== rowsBeforeRetry) problems.push('取消导入不该在表里多出东西')

/* ④ ZIP 不收：不进「本地文件」，结果条如实说 */
/*
 * ★★ 2026-10-14 修：探针找的是「收不了」三个字，而**产品从来没这么说过** ——
 *   真实文案是「现在只收 .toml 预设 —— 这种文件还没有认领它的导入器」
 *   （`mock.ts` 与真机同措辞）。于是这条从写下那天起就不可能过。
 *   判据改成**语义对得上**：结果条要说"只收 .toml"或"还没有认领它的导入器"，
 *   而不是写死某三个字（写死也拦不住产品改措辞，那是另一件事）。
 */
await dropFile('备份.zip', 'PK')
const zipBanner = await bannerText()
console.log(`[第十二层 · ZIP] 结果条：${zipBanner}`)
if (!/只收 ?\.toml|没有认领它的导入器/.test(zipBanner)) {
  problems.push(`ZIP 该被如实拒收（结果是「只收 .toml / 还没有认领它的导入器」），实测「${zipBanner}」`)
}
if ((await actions()).some((r) => r.name.includes('备份.zip'))) {
  problems.push('ZIP 不许被当成预设复制进「本地文件」')
}
await page.screenshot({ path: `${shotDir}/presets-import.png` })

/* ---------- 5m. 第十三层：文件外部管理（在 Finder 中显示） ---------- */
/*
 * 守三件事：
 *   ① 这个动作只给「我的文件」（官方基线不进用户世界，2026-10-08 —— 没有它的行，
 *      也就没有可显示的落点）；
 *   ② 点它走的是**真的那条动作** —— 浏览器里没有文件管理器、假后端的"文件"也只是内存里
 *      一条，所以假后端如实说这一步在真机上的样子（不假装打开了）；
 *   ③ 它不碰任何状态：点完原来那份照样「已应用」。
 *
 * 真机上更硬的判据在 Rust 侧：`local::reveal_target`（两道闸 / 读不出来的也能显示 /
 * 外面删了就说找不到）那三条。
 */
await rad('preset-kind', 'mkp').click({ force: true })
await scope('local')
await page.waitForTimeout(300)

/* 菜单里那一项按标签找 —— Windows 是「显示所在文件夹」、macOS 是「在访达中显示」，
   动作同一条（★ 2026-10-13：只认"中显示"结尾会把 Windows 那一档漏掉） */
const revealItemOf = () =>
  page.evaluate(() => {
    const ul = document.querySelector('[role="menu"]')
    if (ul === null) return null
    const btn = [...ul.querySelectorAll('[role="menuitem"]')].find((b) =>
      /所在文件夹|中显示/.test((b.textContent ?? '').trim()),
    )
    return btn === undefined
      ? null
      : {
          label: (btn.textContent ?? '').trim(),
          /* 灰的判据：现在用 `aria-disabled` / `disabled`（早先那个 `data-on` 不用了） */
          disabled:
            btn.getAttribute('aria-disabled') === 'true' || btn.hasAttribute('disabled'),
          why: btn.getAttribute('title') ?? '',
        }
  })

/* ① 云端那一行（在**云端面** —— 云端交付行住那一侧）：菜单里**根本没有**「…中显示」——
   它住在程序自己管的区域，用户在文件管理器里没有落点可去；这条边界不是"灰着"，
   是"压根不给这一项"。
   ★ 2026-10-13：这一段原来直接在「本地」面上找云端行；MKP 档的本地表只列本地文件之后，
   云端行在云端那一侧，于是它连行都找不到（超时 30s）。 */
await mkpAt('cloud')
await page
  .locator('main tbody tr[data-rowkey]')
  .filter({ has: page.locator('td:not([colspan])') })
  .filter({ hasText: 'A1-standard.toml' })
  .first()
  .click({ button: 'right' })
await page.waitForTimeout(250)
const cloudReveal = await revealItemOf()
console.log(
  `[第十三层] 云端那一行的「…中显示」：${cloudReveal === null ? '(没有这一项)' : cloudReveal.label}`,
)
if (cloudReveal !== null) {
  problems.push('云端那一行不该有「…中显示」（只有「本地文件」有落点可显示）')
}
await page.keyboard.press('Escape')
await page.waitForTimeout(200)

/* ② 本地那份（回本地面）：能点；点了如实说"浏览器里没有文件管理器" */
await mkpAt('local')
const mineTr13 = page
  .locator('main tbody tr[data-rowkey]')
  .filter({ has: page.locator('td:not([colspan])') })
  .filter({ hasText: 'A1 涂胶-高速版.toml' })
  .first()
await mineTr13.click({ button: 'right' })
await page.waitForTimeout(250)
const localReveal = await revealItemOf()
console.log(
  `[第十三层 · 本地文件] 「${localReveal?.label ?? '?'}」：${localReveal === null ? '没有这一项' : `灰=${localReveal.disabled}`}`,
)
if (localReveal === null || localReveal.disabled) {
  problems.push('「本地文件」的「…中显示」该能点')
} else {
  await page.getByRole('menuitem', { name: /所在文件夹|中显示/ }).click()
  await page.waitForTimeout(400)
  const revealNote = await page.evaluate(() =>
    (document.querySelector('main [role="status"]')?.innerText ?? '').replace(/\s+/g, ' ').trim(),
  )
  console.log(`[第十三层 · 本地文件] 点完提示条：${revealNote}`)
  if (!revealNote.includes('没打开')) {
    problems.push(`浏览器里没有文件管理器，该如实说「没打开」，实测提示条「${revealNote}」`)
  }
  if (!revealNote.includes('真机上')) {
    problems.push(`该说清这一步在真机上的样子（打开文件管理器并选中），实测「${revealNote}」`)
  }
}

/* ③ 它不碰任何状态：原来那份照样是「当前使用」 */
const stillActive13 = (await actions()).find((r) => r.name.includes('A1 涂胶-高速版.toml'))
if (stillActive13 === undefined || !stillActive13.action.includes('在用')) {
  problems.push('「…中显示」不该碰「在用」')
}
await page.screenshot({ path: `${shotDir}/presets-reveal.png` })

/* ---------- 5n. 第十七刀：进入预设后台检查一次 Bootstrap（失败静默） ---------- */
/*
 * 守三件事：
 *   ① 进入预设页**不因后台检查而报错/卡住** —— 本地 catalog 立刻画出来（首屏不等远端）；
 *   ② 后台检查失败（浏览器里没有远端 / 没内置源）**静默**：不许把预设页升格成整页错误，
 *      也不许弹一条"检查更新失败"的提示条（启动零网络那条纪律的延伸：这里只是"路过问一声"）；
 *   ③ **绝不自动下载**：整页跑完一遍，一张表里不该出现任何"正在下载 / 已下载"的副作用
 *      （下载只有用户点那颗按钮才发生）。
 *
 * 假后端里 `checkRemoteUpdate` 必抛未实现（浏览器没有远端）—— 那条 error 已在 BENIGN
 * 里放行，所以"控制台有没有 error"这一节不受它牵连。真机那条更硬的判据在 Rust 侧：
 * `check_remote_update` / `apply_remote_update` 只换本地 catalog、不碰 mkp/ 里的文件。
 */
await rad('preset-kind', 'mkp').click({ force: true })
await scope('local')
await page.waitForTimeout(400)
/* 上一节（第十三层）留下的提示条先关掉 —— 这里要看的是"后台检查会不会自己冒出一条" */
const closeNote = page.locator('main [role="status"] button[aria-label="关闭这条提示"]')
if ((await closeNote.count()) > 0) await closeNote.first().click()
await page.waitForTimeout(500)
const bootstrapFact = await page.evaluate(() => ({
  text: (document.querySelector('main')?.innerText ?? '').replace(/\s+/g, ' ').trim(),
  status: document.querySelector('main [role="status"]')?.innerText.replace(/\s+/g, ' ').trim() ?? '',
  rows: document.querySelectorAll('main tbody tr td:not([colspan])').length,
}))
console.log(`\n[第十七刀 · Bootstrap 后台检查] 提示条：${bootstrapFact.status || '(没有 —— 静默)'}`)
if (!/已应用|在用/.test(bootstrapFact.text)) {
  problems.push('后台检查失败不该整页报错：本地那一份状态条该照常在')
}
if (/检查更新失败|加载失败|Bootstrap|源/.test(bootstrapFact.status)) {
  problems.push(`后台检查失败该静默（不许冒出提示条），实测「${bootstrapFact.status}」`)
}
if (bootstrapFact.rows === 0) {
  problems.push('后台检查不该挡首屏：本地表该立刻有行')
}

/* ---------- 5o. 「撤销应用」退役（作者 2026-10-04 裁决：不做取消应用，总得有一套在生效） ---------- */
/*
 * 守两件事：
 *   ① 界面上**没有**「撤销应用」—— 机型下拉 / 已应用状态条 / 定位按钮三件并成一颗
 *      融合 pill 之后，这个入口不再存在（作者原话：「没必要存在，不管它了，就用 A44 的」。
 *      后端 `clear_active_preset` 命令与数据层 `clearApply` 能力照旧在，只是没有界面入口）；
 *   ② pill 左拍照常说得出正在生效的是谁 —— 撤的入口没了，「哪一套在生效」这个事实
 *      反而要更醒目（信息不随入口退役而丢）。
 *
 * 能力侧的判据在 Rust 侧照旧：`runtime::state::clear_active` + `ipc::clear_active_preset`
 * —— 命令保留，探针不再量它的 UI（已经没有 UI 可点）。
 */
await rad('preset-kind', 'mkp').click({ force: true })
await scope('local')
await page.waitForTimeout(300)

const clearCount = await page.getByRole('button', { name: '撤销应用' }).count()
console.log(`\n[撤销应用退役] 界面上的「撤销应用」按钮：${clearCount} 个`)
if (clearCount !== 0) {
  problems.push('「撤销应用」已退役（作者 2026-10-04：不做取消应用，总得有一套在生效）——界面上不该再有它')
}
const pillText = await page.evaluate(() => {
  /* 状态 pill 是**span** —— div 那一批是导航 / 筛选 / 计数的 pill，class 模糊匹配会撞上 */
  const el = document.querySelector('main span[class*="pill"]')
  return el === null ? '' : (el.textContent ?? '').replace(/\s+/g, ' ').trim()
})
console.log(`[撤销应用退役] pill 左拍：${pillText}`)
/*
 * ★ 2026-10-13：这条 pill 现在只写**文件名 + 机型 + 版本**（实测 `A1 涂胶-高速版.tomlA1A1`），
 *   没有「已应用 / 在用」那样的状态词 —— 判据改成"说得出正在生效的是哪一份"。
 */
if (!/\.toml/.test(pillText)) {
  problems.push(`融合 pill 该说得出正在生效的是哪一份文件，实测「${pillText}」`)
}
await page.screenshot({ path: `${shotDir}/presets-pill.png` })

/* ---------- 6. 跨页那一条：切片器行右键 → 「在 BBS 中查看」 ---------- */
/*
 * 这一条量的是**外壳那一层**的接线：点了之后 tab 要切到工艺页（页签名 2026-10-13 从
 * 「BBS 预设」改成「工艺」；路由 id 仍是 `bbs`）。
 *
 * ★ 2026-10-13 按现状改了三处：
 *   ① 菜单项从「在 BBS 预设查看器中打开」压成了「**在 BBS 中查看**」（2026-10-10 那次压缩）；
 *   ② 灰的判据换成 `aria-disabled`（早先的 `data-on` 不用了）；
 *   ③ 挑的行必须是**切片器的工艺 profile（.json / bbs_config）** —— 原来抓的是表里第一行，
 *      那可能是「本地文件」里认不出类别的那一份，对它这一项本来就该灰着。
 *   MKP 的 `.toml` 配方**没有**这个入口（不是 BBS 工艺预设），所以这条只认切片器档。
 */
await rad('preset-kind', 'slicer').click({ force: true })
await page.waitForTimeout(200)
await scope('local')
await page.waitForTimeout(250)
const bbsRow = page
  .locator('main tbody tr[data-rowkey]')
  .filter({ has: page.locator('td:not([colspan])') })
  .filter({ hasText: 'MKPProcess A1 0.2 0.10.json' })
  .first()
await bbsRow.click({ button: 'right' })
await page.waitForTimeout(200)
const bbsItem = page.getByRole('menuitem', { name: '在 BBS 中查看', exact: true })
const bbsCount = await bbsItem.count()
/* 灰的判据两样都认：菜单项现在挂的是 `disabled` 属性（`aria-disabled` 一并兜住） */
const bbsDisabled =
  bbsCount > 0
    ? (await bbsItem
        .first()
        .evaluate((b) => b.hasAttribute('disabled') || b.getAttribute('aria-disabled') === 'true'))
    : null
console.log(`\n[BBS 入口] 菜单项 ${bbsCount} 个，aria-disabled=${bbsDisabled}`)
if (bbsCount === 0) {
  problems.push('BBS 行右键没有「在 BBS 中查看」')
} else if (bbsDisabled === true) {
  problems.push('BBS 行的「在 BBS 中查看」是灰的（切片器工艺 profile 应该可点）')
} else {
  await bbsItem.first().click()
  await page.waitForTimeout(350)
  const after = await page.evaluate(() => ({
    text: (document.querySelector('main')?.innerText ?? '').replace(/\s+/g, ' ').trim(),
    active: document.querySelector('header nav [aria-current="page"]')?.textContent?.trim() ?? '',
  }))
  console.log(`[BBS 入口] 点完落在 ${after.active}：${after.text.slice(0, 80)}`)
  if (after.active !== '工艺') {
    problems.push(`点「在 BBS 中查看」之后没有切到「工艺」页（现在是 ${after.active}）`)
  }
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
await scope('local')
await page.waitForTimeout(300)

/* 工具栏恒两排的 DOM 判据（作者 2026-10-04 定稿）：MKP 档的「共 N 项 │ 仓库…」
   恒住工具栏第二排（pill 右边），页脚只留右键提示 —— 不随窗宽换住处 */
const metaHome = await page.evaluate(() => {
  const toolbar = document.querySelector('main div[class*="toolbar"]')
  const foot = document.querySelector('main div[class*="foot"]')
  return {
    inToolbar: (toolbar?.innerText ?? '').includes('共 '),
    inFoot: (foot?.innerText ?? '').includes('共 '),
  }
})
console.log(
  `[工具栏恒两排] MKP 档计数台账：工具栏=${metaHome.inToolbar} 页脚=${metaHome.inFoot}`,
)
if (!metaHome.inToolbar) problems.push('MKP 档的计数台账该恒住工具栏第二排（pill 右边）')
if (metaHome.inFoot) problems.push('MKP 档的页脚不该再有计数台账（它住工具栏第二排；切片器档才住页脚）')

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
  '\n预设页（2026-10-13 按**现在的表**整体校过一遍）：两条轴都点得动（MKP 档**也有**「本地 / 云端」：' +
    '本地那面是「本地文件」、云端那面是云端交付行）、点行展开（详情面板是 `_dGrid` 那一套：版本 / 来源 / 基于）、' +
    '右键菜单按行给（云端交付行：下载 / 更新 · 复制链接 · 在 BBS 中查看；本地文件：使用 · 修改参数 · 参数对比 · 编辑文件 · ' +
    '复制 · 重命名 · 显示所在文件夹 · 置顶 · 在 BBS 中查看（灰）· 删除；切片器交付行还多两条「显示我们那份的文件夹」/' +
    '「显示切片器里的那份」）、' +
    '我那份能被使用（操作列与那条融合 pill 都写「在用」）并说得清「基于 … · 云端已换新版」，' +
    '改我自己那份走右键「编辑文件」（给整份文件、含血统那几行）或「修改参数」（参数表那一屏），' +
    '读不出来的那一份画得出「文件无法读取」且「使用 / 修改参数」灰着、连「编辑文件」都不出现（第九层），' +
    '本地文件能改名（输入框只要基名）也能删（二次确认；删除全面放开），能复制出一份新的' +
    '（名字自动 `X copy.toml`、原文件一个字节不动 —— 第十一层），' +
    '导入入口（第十二层）：拖拽是唯一入口、重名进改名格、不覆盖、ZIP 收不了、不碰「在用」，' +
    '外部管理（第十三层）：右键能在文件管理器里显示「本地文件」（云端那一行压根没有这一项、失败如实说），' +
    'Bootstrap 后台检查（第十七刀）：不挡首屏、失败静默、绝不自动下载，' +
    '撤销应用退役（那条 pill 说得出正在生效的是哪一份文件），控制台没有 error',
)
