/*
 * 工作台 C15 增量探针：**生成与发布页**（发布物 / 查看 JSON / 查看 TOML / 版本轴 / 云端那一格）
 * 与**参数台的两处版面增量**（模式开关整卡收起 · 抽屉说清类型）。
 *
 * # 为什么探针打的是 development + esnext 的那一份（而不是 `npm run build:workbench` 的产物）
 *
 * `workbench.html` 只在 `npm run build:workbench`（`vite build --mode workbench`）里进包，
 * 而**开发桩只在 `import.meta.env.DEV` 且 URL 带 `?mock=1` 时装**（2026-10-02 起；
 * 见 `src/workbench/main.tsx`）：production 构建的页面没有 Tauri IPC、也没有桩，
 * 整页读不到任何后端数据 —— 那时候探针量的不是页面，是白屏（实测：
 * `TypeError: Cannot read properties of undefined (reading 'invoke')`）。
 * 本探针**自己给 URL 补 `?mock=1`**（传进来的地址上已有就不重复加）。
 * 生产产物不该有桩，所以这不是要去修的 bug，而是**验收路径**的问题：`npm run dev`
 * （DEV=true、有桩）在这台机器上被 `target/` 两棵构建树（5.8 万文件）拖死，HTTP 全超时。
 *
 * 于是浏览器验收分两步走：**生产那一份**照 `npm run build:workbench` 报体积（闸门一），
 * **带了桩的那一份**用下面三条命令单独打（逐条理由见每条后面的括注）：
 *
 *   npm run build:workbench                                     # 生产产物（含 workbench.html）
 *   $env:NODE_ENV='development'; $env:BUILD_WORKBENCH='1'
 *   npx vite build --target esnext --outDir node_modules/.cache/wb-probe/dist --emptyOutDir
 *       （NODE_ENV=development 才让 import.meta.env.DEV 为真、桩才进包；
 *         --target esnext 放行 main.tsx 里装桩那句 top-level await，默认 target 下 esbuild 直接报错；
 *         outDir 落在 gitignore 的机器本地缓存里 —— 不碰仓库的 dist/）
 *   npx vite preview --outDir node_modules/.cache/wb-probe/dist --port 4174 --strictPort
 *   node scripts/probes/workbench-build.mjs http://localhost:4174/workbench.html --shots
 *
 * 4173 是另一个 agent 的预览，这里用 4174；探针自己不起服务，**用完把 4174 那台关掉**。
 *
 * 探针只判"看得见"的事（照试验场与 `tabs.mjs` 那一套）：点得动、有字、控制台不红、
 * HTTP 不 4xx/5xx。它**不当判据**（判据在 Rust 与 `tsc`），只把这一次的现场打出来。
 */
import { mkdir } from 'node:fs/promises'

import { chromium } from 'playwright-core'

/*
 * `?mock=1`：开发桩不再自动装（2026-10-02 起 —— 真机 `tauri:workbench:dev` 永远走真
 * Tauri IPC；桩是**探针的测试后端**，得自己显式要，见 src/workbench/main.tsx）。
 * 传进来的 URL 上若已经带 `mock` 就不重复加。
 */
const rawUrl = process.argv[2]?.startsWith('http') ? process.argv[2] : 'http://localhost:4174/workbench.html'
const url = /[?&]mock=/.test(rawUrl) ? rawUrl : `${rawUrl}${rawUrl.includes('?') ? '&' : '?'}mock=1`
const wantShots = process.argv.includes('--shots')
const shotDir = 'tmp-shots'

const problems = []
const BENIGN = [/\/favicon\.ico$/]
const benign = (text) => BENIGN.some((re) => re.test(text))

const browser = await chromium.launch({ channel: 'msedge' })
const page = await browser.newPage({ viewport: { width: 1760, height: 900 } })

page.on('console', (m) => {
  if (m.type() !== 'error') return
  const at = m.location?.()?.url ?? ''
  if (benign(at)) return
  problems.push(`console.error: ${m.text()}${at ? ` @ ${at}` : ''}`)
})
page.on('pageerror', (e) => problems.push(`pageerror: ${e.message}`))
page.on('response', (r) => {
  if (r.status() >= 400 && !benign(r.url())) problems.push(`HTTP ${r.status()} ${r.url()}`)
})
page.on('requestfailed', (r) => {
  if (!benign(r.url())) problems.push(`requestfailed ${r.failure()?.errorText ?? ''} ${r.url()}`)
})

if (wantShots) await mkdir(shotDir, { recursive: true })

const text = (t) => (t ?? '').replace(/\s+/g, ' ').trim()
/* 工作台外壳的正文在 `.shellBody` 里（这一页没有 `<main>`：那是客户端顶栏那一套） */
const mainText = () => page.evaluate(() => document.querySelector('[class*="shellBody"]')?.innerText ?? '')
const say = (ok, label, detail = '') =>
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${label}${detail ? ` —— ${detail}` : ''}`)

/** 等一个条件成立（后端是内存桩，一般是一两帧的事） */
async function until(fn, ms = 3000) {
  const t0 = Date.now()
  for (;;) {
    if (await fn()) return true
    if (Date.now() - t0 > ms) return false
    await page.waitForTimeout(80)
  }
}

/** 页面里有没有这段可见文字 */
const hasText = (needle) => mainText().then((t) => text(t).includes(needle))
/** 模态框里的字（框是 portal 到 shellBody 的，读 [role=dialog] 那一段） */
const dialogText = () =>
  page.evaluate(() => document.querySelector('[role="dialog"]')?.innerText ?? '')

/* —— 切页立刻有反馈（2026-10-02）：整本没回来时先画骨架屏，不是黑屏 —— */
/*
 * 守作者那条验收：「点了等一段时间它才显示是不对的，它必须立马显示，就是那个反馈」。
 * 判据：一条慢命令占着时，页面上有 `[data-skeleton]`（骨架），且**导航已经画出来**。
 * 用 CDP 的 CPU 节流造"慢"（真机后端慢时走的是同一条路：`renderPage` 在 `book/words`
 * 没到时返回骨架而不是 null）。
 */
{
  const client = await page.context().newCDPSession(page)
  await client.send('Emulation.setCPUThrottlingRate', { rate: 20 })
  await page.goto(url, { waitUntil: 'commit' })
  const earlySkel = await until(
    () => page.locator('[data-skeleton]').count().then((n) => n > 0),
    10000,
  )
  const earlyShell = await page.locator('nav[aria-label="一级导航"]').count()
  say(earlySkel && earlyShell > 0, `整本回来之前先画骨架屏（不是黑屏）—— 骨架 ${earlySkel} · 导航已在`)
  if (!earlySkel) problems.push('数据没到时不画骨架屏（点了没反馈）')
  await client.send('Emulation.setCPUThrottlingRate', { rate: 1 })
  /* 放开节流，等真内容顶上（骨架应当消失） */
  await page.waitForSelector('nav[aria-label="一级导航"]', { timeout: 15000 })
  await until(() => page.locator('[data-skeleton]').count().then((n) => n === 0), 10000)
}

/* 工作台外壳没有 `<header>`（那是客户端顶栏）—— 它是一级导航 + `.shellBody` 正文 */
await page.waitForSelector('nav[aria-label="一级导航"]', { timeout: 15000 })
await page.waitForSelector('[class*="shellBody"]', { timeout: 15000 })

console.log(`url ${url}`)
console.log(`视口 1760×900 · dev 桩（浏览器里没有 Tauri IPC）`)
console.log('-'.repeat(96))

/* ============================ 一、生成与发布 ============================ */

console.log('\n【一】生成与发布（C15：发布物 / 查看 JSON / 查看 TOML / 版本轴 / 云端）')
/* 用 title 认导航项：带徽标的那几项可访问名字会多出一个数字（「生成与发布 1」） */
await page.locator('button[title="生成与发布"]').first().click()
await until(() => hasText('② 客户端数据包'))
const buildText = text(await mainText())
console.log(`页面可见文字（前 600 字）：\n${buildText.slice(0, 600)}`)

for (const needle of ['① 生成', '② 客户端数据包', '三个版本，各管一件事', '客户端兼容性清单', '③ 发布', '本次发布的发布物', '云端 preset 文件夹（模拟）']) {
  const ok = await hasText(needle)
  say(ok, `这一页写着「${needle}」`)
  if (!ok) problems.push(`生成与发布页缺少「${needle}」`)
}

/* —— 包版本的三枚快捷：点一下那个数要跟着变（不是个死按钮） —— */
/* 那一格里的「建议值」是一个**整段就是 x.y.z** 的 span（三枚按钮的文本带 +，不会误认） */
const verText = () =>
  page.evaluate(() => {
    const box = document.querySelector('#t-pkg-ver')
    if (!box) return ''
    const hit = [...box.querySelectorAll('span')]
      .map((e) => (e.textContent ?? '').trim())
      .find((t) => /^\d+\.\d+\.\d+$/.test(t))
    return hit ?? ''
  })
const before = await verText()
await page.locator('#t-pkg-ver button', { hasText: '+0.1.0' }).first().click()
const after = await verText()
say(before !== '' && after !== '' && before !== after, `包版本快捷（+0.1.0）：${before || '（空）'} → ${after || '（空）'}`)
if (before === after) problems.push('包版本快捷点了没反应')

/* —— 最低客户端版本那一格：说明书装出来之后，「自动判断」要给出一个数 —— */
const minClient = text(await page.evaluate(() => document.querySelector('#t-min-client')?.innerText ?? ''))
console.log(`最低客户端版本那一格：${minClient.slice(0, 260)}`)
const autoVer = /自动判断：\d+\.\d+\.\d+/.exec(minClient)?.[0] ?? '（没找到）'
const minOk = minClient.includes('客户端兼容性清单') && autoVer.startsWith('自动判断：')
say(minOk, `自动判断给出了数：${autoVer}（依据是那一张兼容性清单）`)
if (!minOk) problems.push('最低客户端版本那一格没给出自动判断的结论')

/* —— 查看 JSON：弹出来的这一份就是**要传出去的那份说明书** —— */
await page.getByRole('button', { name: '查看 JSON', exact: true }).first().click()
await page.waitForSelector('[role="dialog"] textarea', { timeout: 5000 })
const json = await page.evaluate(() => document.querySelector('[role="dialog"] textarea')?.value ?? '')
const jsonOk =
  json.includes('"clientPackage"') && !json.includes('"clientPackage": null') &&
  json.includes('"inputsHash"') && json.includes('"release"')
say(jsonOk, `查看 JSON：${json.length} 字符，clientPackage 是真包（带 inputsHash）`, json.split('\n').slice(0, 2).join(' '))
if (!jsonOk) problems.push('查看 JSON 里没有真说明书')
if (wantShots) await page.screenshot({ path: `${shotDir}/wb-build-json.png` })
await page.getByRole('button', { name: '关闭', exact: true }).first().click()
await page.waitForSelector('[role="dialog"]', { state: 'detached', timeout: 5000 })

/* —— 发布物：产物名单与 TOML 正文都该出来（开发桩开局已经生成过 A1/STANDARD 一版） —— */
const seeded = /preset\.toml × \d+ 份/.exec(text(await mainText()))?.[0] ?? '（没找到）'
say(seeded === 'preset.toml × 1 份', `开局产物名单跟着后端 buildRows 走：${seeded}`)
if (seeded !== 'preset.toml × 1 份') problems.push('开局产物名单与 buildRows 对不上')
/* —— 选择：已生成也能勾 + 一个「全选」（2026-10-02，作者点名） —— */
{
  const boxes = await page.locator('#t-build input[type="checkbox"]').count()
  const off = await page.locator('#t-build input[type="checkbox"][disabled]').count()
  say(boxes > 0 && off === 0, `已生成的行也能勾（勾选框 ${boxes} 个、被禁的 ${off} 个）`)
  if (boxes === 0 || off > 0) problems.push('已生成的行还是勾不上')
  const hasAll = (await page.getByRole('button', { name: '全选', exact: true }).count()) > 0
  say(hasAll, '「全选」按钮在（与「全选待生成」并存）')
  if (!hasAll) problems.push('缺少「全选」按钮')
  /* 点「全选」= 勾上所有能勾的（含已生成） */
  await page.getByRole('button', { name: '全选', exact: true }).first().click()
  await page.waitForTimeout(150)
  const allChecked = await page.locator('#t-build input[type="checkbox"]:checked').count()
  say(allChecked === boxes, `「全选」勾上全部 ${allChecked}/${boxes} 行`)
  if (allChecked !== boxes) problems.push('「全选」没勾满')
}

await page.getByRole('button', { name: '全选待生成', exact: true }).first().click()
await page.getByRole('button', { name: /^生成 \d+ 项$/ }).first().click()

/* —— 生成前确认（2026-10-02）：点「生成」**不写盘**，先弹 diff 确认框 —— */
await page.waitForSelector('[role="dialog"]', { timeout: 5000 })
const dlgReady = await until(async () => (await dialogText()).includes('生成前确认'), 5000)
const dlgBody = await dialogText()
const dlgSeen = await dlgReady
const hasList = (await page.locator('[role="dialog"] nav[aria-label="要生成的文件"] button').count()) > 0
say(dlgSeen && hasList, `点生成先弹确认框（不是直接覆盖）：${text(dlgBody).slice(0, 60)}`)
if (!dlgSeen || !hasList) problems.push('生成前没有弹 diff 确认框')
/* 还没确认 —— 产物名单此时**不该**已经变了（改盘发生在确认之后） */
const beforeConfirm = /preset\.toml × \d+ 份/.exec(text(await mainText()))?.[0] ?? '（没找到）'
say(beforeConfirm === 'preset.toml × 1 份', `确认之前不写盘，产物名单还是开局那份：${beforeConfirm}`)
if (beforeConfirm !== 'preset.toml × 1 份') problems.push('还没确认就把盘写了（预演应是只读）')
if (wantShots) await page.screenshot({ path: `${shotDir}/wb-build-generate-diff.png` })
/* 点确认 → 框切成结果页 → 点「完成」关掉 */
await page.getByRole('button', { name: '确认生成', exact: true }).first().click()
const doneShown = await until(async () => (await dialogText()).includes('生成完成'), 5000)
say(doneShown, '确认之后框内换成结果页（写了几份 / 几份未变）')
if (!doneShown) problems.push('确认之后没看到结果页')
await page.getByRole('button', { name: '完成', exact: true }).first().click()
await page.waitForSelector('[role="dialog"]', { state: 'detached', timeout: 5000 })

const gotArtifacts = await until(() => hasText('preset.toml × 2 份'))
const artLine = /preset\.toml × \d+ 份/.exec(text(await mainText()))?.[0] ?? '（没找到）'
say(gotArtifacts, `确认生成之后：${artLine}`)
if (!gotArtifacts) problems.push('生成之后产物名单没跟上')

/* —— ② 现在报的是**真包**：那几个数从现装的那份说明书上数 —— */
const pkgChips = /(\d+) 台机型 (\d+) 个版本 (\d+) 个字段 带条件 (\d+) 可选文件 (\d+)/.exec(text(await mainText()))
say(pkgChips !== null, `「包里有什么」从这份包上数：${pkgChips?.slice(1).join(' / ') ?? '（没找到）'}`)
if (pkgChips === null) problems.push('「包里有什么」没报出真包的计数')
const pkgChip = /说明书 × 1 · \d+ 字段（带条件 \d+）/.exec(text(await mainText()))?.[0] ?? '（没找到）'
say(pkgChip.startsWith('说明书 × 1'), `③ 发布物里那一枚说明书报的是真包：${pkgChip}`)
if (!pkgChip.startsWith('说明书 × 1')) problems.push('③ 的说明书 chip 没报出真包')

await page.getByRole('button', { name: '查看 TOML', exact: true }).first().click()
await page.waitForSelector('[role="dialog"] textarea', { timeout: 5000 })
const toml = await page.evaluate(() => document.querySelector('[role="dialog"] textarea')?.value ?? '')
const tomlOk = toml.includes('# machine:') && toml.includes('[demo]')
say(tomlOk, `查看 TOML：${toml.length} 字符（正文来自后端那条 wb_preview_toml）`, toml.split('\n')[0])
if (!tomlOk) problems.push('查看 TOML 没拿到正文')
if (wantShots) await page.screenshot({ path: `${shotDir}/wb-build-toml.png` })
await page.getByRole('button', { name: '关闭', exact: true }).first().click()
await page.waitForSelector('[role="dialog"]', { state: 'detached', timeout: 5000 })

/* —— 云端那一格：静态种子读得到；「上传到云端」按得动并真写进那一格 —— */
const upload = page.getByRole('button', { name: '上传到云端', exact: true }).first()
const uploadDisabled = await upload.isDisabled()
const cloudBefore = await page.evaluate(() => localStorage.getItem('mkp.cloud.presets'))
say(
  !uploadDisabled && cloudBefore === null,
  `上传到云端：可以按（说明书已装出来）· 传之前 localStorage[mkp.cloud.presets]=${cloudBefore ?? 'null'}`,
)
if (uploadDisabled) problems.push('上传到云端按不动（说明书装出来了就该能按）')

/* 云端**读**的那一半要是真的：静态种子（public/cloud/presets.json）该在这一格里 */
const cloudSeeded = await until(() => hasText('云端已有'))
const cloudText = text(await mainText())
const seedSummary = /说明书 \d+ 个字段（带条件 \d+）· 预设文件 \d+ 份/.exec(cloudText)?.[0] ?? '（没找到）'
say(cloudSeeded, `云端读得到静态种子那一份：「云端已有」+ ${seedSummary}`)
if (!cloudSeeded) problems.push('云端读不到静态种子（public/cloud/presets.json）')
say(await hasText('这一格现在是空的') === false, '云端非空时不再摆「这一格现在是空的」')
if (wantShots) await page.screenshot({ path: `${shotDir}/wb-build-page.png` })

/* 真按一下：整份 release（说明书 + 预设文件）写进 STORAGE.cloud —— 客户端读的就是这一格 */
await upload.click()
const wrote = await until(async () =>
  (await page.evaluate(() => localStorage.getItem('mkp.cloud.presets')))?.includes('工作台发布') === true)
const uploaded = await page.evaluate(() => {
  const raw = localStorage.getItem('mkp.cloud.presets')
  if (raw === null) return null
  const list = JSON.parse(raw)
  const e = list[list.length - 1]
  return { n: list.length, name: e.name, version: e.version, presets: e.presets.length, fields: e.package.fields.length }
})
say(
  wrote && uploaded !== null && uploaded.presets > 0,
  `上传：${uploaded?.name} · 预设文件 ${uploaded?.presets} 份 · 说明书 ${uploaded?.fields} 字段 · 云端共 ${uploaded?.n} 条`,
)
if (!wrote) problems.push('上传到云端没写进 localStorage')
if ((uploaded?.presets ?? 0) === 0) problems.push('上传的 release 里一份预设文件都没有')
console.log(`[上传] 客户端要读的那一格：${JSON.stringify(uploaded)}`)

/* ============================ 二、参数台 ============================ */

console.log('\n【二】参数台（C15 A2 整卡收起 · B1 抽屉说清类型）')
await page.locator('button[title="参数台"]').first().click()
await page.waitForTimeout(200)

/* 左树默认收起：点把手钉住，再选 A1 / 标准版 */
await page.locator('button[title*="钉住"]').first().click()
await page.waitForSelector('nav[aria-label="机型与版本"] button', { timeout: 5000 })
await page
  .locator('nav[aria-label="机型与版本"] button')
  .filter({ hasText: 'STANDARD' })
  .first()
  .click()
const deskReady = await until(() => hasText('擦料方式'))
say(deskReady, '选中 A1 / 标准版之后参数表出来了')
if (!deskReady) problems.push('参数台没出参数表')

const openGroups = text(await mainText())
for (const label of ['塔位置与打印', '塔结构加强']) {
  say(openGroups.includes(label), `开着的这一档有「${label}」`)
}

/* —— A2：切到「圆盘擦拭」，属于另一支的整张卡收起来 —— */
/* 分段选择器里的选项是 `role="radio"`（见共用件 SegmentedField），不是 button */
await page
  .getByRole('radiogroup', { name: '擦料方式' })
  .first()
  .getByRole('radio', { name: '圆盘擦拭' })
  .click()
const collapsed = await until(() => hasText('仍然展开看'))
const afterSwitch = text(await mainText())
say(collapsed, '切到「圆盘擦拭」之后有整卡收起（出现「仍然展开看」）')
if (!collapsed) problems.push('切模式之后整卡没有收起')

/* 两张演示卡走的是两条判据：section 级（后端整句）与字段级（第一行那句 hint） */
const stubSection = /「[^」]+」选了，这一组 \d+ 项现在不生效/.exec(afterSwitch)?.[0] ?? '（没找到）'
const stubField = /要 [^。]*才可改/.exec(afterSwitch)?.[0] ?? '（没找到）'
say(stubSection.startsWith('「'), `section 级整组那句（DeskGroup.offNote）：${stubSection}`)
say(stubField.startsWith('要 '), `字段级那句（第一行 blockedHint）：${stubField}`)
say(!afterSwitch.includes('擦料塔首层流量'), '收起之后那几行不在页面上（整卡收起，不是灰行）')
if (afterSwitch.includes('擦料塔首层流量')) problems.push('整卡收起之后行还在')
if (wantShots) await page.screenshot({ path: `${shotDir}/wb-params-collapsed.png` })

/* —— 收起的那几行要能摊开看（两张卡各点一次：section 级那句与字段级那句） —— */
const collapsedCard = (label) =>
  page.locator('section[data-off="true"]').filter({ hasText: label }).first()
await collapsedCard('塔结构加强').getByRole('button', { name: '仍然展开看' }).click()
const reopenedField = await until(() => hasText('擦料塔首层流量'))
say(reopenedField, '点「仍然展开看」（字段级那张卡）之后那几行回来了')
if (!reopenedField) problems.push('展开看没把行放回来（字段级）')
await collapsedCard('塔位置与打印').getByRole('button', { name: '仍然展开看' }).click()
const reopenedSection = await until(() => hasText('擦料塔位置 X'))
say(reopenedSection, '点「仍然展开看」（section 级那张卡）之后那几行回来了')
if (!reopenedSection) problems.push('展开看没把行放回来（section 级）')

/* —— B1：抽屉里「基本信息」要有值类型 / 控件，技术信息只留注册键 —— */
await page.locator('[class*="shellBody"]').getByText('擦料方式', { exact: true }).first().click()
const drawerOpen = await until(() => hasText('基本信息'))
say(drawerOpen, '点一行之后右栏抽屉出来了')
/* 这一页有两个 `<aside>`（左树 rail 与右栏抽屉）—— 取写着「各版本取值」的那个 */
const aside = text(
  await page.evaluate(() => {
    const all = [...document.querySelectorAll('aside')]
    return (all.find((a) => a.innerText.includes('各版本取值')) ?? all[all.length - 1])?.innerText ?? ''
  }),
)
const atBasic = aside.indexOf('基本信息')
const atTech = aside.indexOf('技术信息')
const b1ok =
  atBasic >= 0 && atTech > atBasic && aside.slice(atBasic, atTech).includes('值类型') &&
  aside.slice(atBasic, atTech).includes('控件') && !aside.slice(atTech).includes('值类型')
console.log(`抽屉：${aside.slice(0, 260)}`)
say(b1ok, '「基本信息」里有值类型 / 控件，「技术信息」只剩注册键')
if (!b1ok) problems.push('抽屉里类型信息的位置不对')
if (wantShots) await page.screenshot({ path: `${shotDir}/wb-params-drawer.png` })

/* ============================ 三、参数台 · 窄档复查（900×640） ============================ */
/*
 * 验收口径是**两档尺寸走查**，所以这一页的两处判据要在窄档下再走一遍。
 * 先 `reload` 再量：参数台的选中项与「仍然展开看」都是组件内的 state，
 * 上一段（ultra）已经把两张卡摊开过了 —— 不重来一遍，量到的就不是窄档的默认态。
 */
console.log('\n【三】参数台 · 窄档（900×640）—— 同一套判据在两档尺寸下都成立')
await page.setViewportSize({ width: 900, height: 640 })
await page.reload({ waitUntil: 'load' })
await page.waitForSelector('nav[aria-label="一级导航"]', { timeout: 15000 })
await page.locator('button[title="参数台"]').first().click()
await page.waitForTimeout(300)
const pinC = page.locator('button[title*="钉住"]').first()
if ((await pinC.count()) > 0) await pinC.click()
await page.waitForSelector('nav[aria-label="机型与版本"] button', { timeout: 5000 })
await page
  .locator('nav[aria-label="机型与版本"] button')
  .filter({ hasText: 'STANDARD' })
  .first()
  .click()
const deskNarrow = await until(() => hasText('擦料方式'))
say(deskNarrow, '窄档：选中 A1 / 标准版之后参数表出来了')
if (!deskNarrow) problems.push('窄档下参数台没出参数表')

await page
  .getByRole('radiogroup', { name: '擦料方式' })
  .first()
  .getByRole('radio', { name: '圆盘擦拭' })
  .click()
const collapsedNarrow = await until(() => hasText('仍然展开看'))
const narrowText = text(await mainText())
say(collapsedNarrow, '窄档：切到「圆盘擦拭」之后整卡收起同样成立')
say(!narrowText.includes('擦料塔首层流量'), '窄档：收起之后那几行也不在页面上')
if (!collapsedNarrow) problems.push('窄档下切模式没有整卡收起')
if (narrowText.includes('擦料塔首层流量')) problems.push('窄档下整卡收起之后行还在')

await page.locator('[class*="shellBody"]').getByText('擦料方式', { exact: true }).first().click()
const drawerNarrow = await until(() => hasText('基本信息'))
const asideNarrow = text(
  await page.evaluate(() => {
    const all = [...document.querySelectorAll('aside')]
    return (all.find((a) => a.innerText.includes('各版本取值')) ?? all[all.length - 1])?.innerText ?? ''
  }),
)
const nB = asideNarrow.indexOf('基本信息')
const nT = asideNarrow.indexOf('技术信息')
const b1Narrow = nB >= 0 && nT > nB && asideNarrow.slice(nB, nT).includes('值类型') && asideNarrow.slice(nB, nT).includes('控件')
say(drawerNarrow && b1Narrow, '窄档：抽屉「基本信息」里同样有值类型 / 控件')
if (!b1Narrow) problems.push('窄档下抽屉类型信息的位置不对')
if (wantShots) await page.screenshot({ path: `${shotDir}/wb-params-compact.png` })

console.log('-'.repeat(96))
await browser.close()

if (problems.length > 0) {
  console.log(`发现问题 ${problems.length} 条：`)
  for (const p of problems) console.log(`  - ${p}`)
  process.exit(1)
}
console.log('两处增量都点得动：生成与发布页五项齐全，参数台整卡收起与抽屉类型就位，控制台没有 error')
