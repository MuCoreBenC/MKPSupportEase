/*
 * 工作台 C15 增量探针：**生成与发布页**（交付文件 / 查看交付文件 / 生成前确认 / 产物名单）
 * （2026-10-04：「说明书 JSON / 版本轴 / 云端那一格」随 `ClientDataPackage` 退役一并去掉）
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

console.log('\n【一】生成与发布（交付文件 / 查看交付文件 / 生成前确认 / 产物名单）')
/* 用 title 认导航项：带徽标的那几项可访问名字会多出一个数字（「生成与发布 1」） */
await page.locator('button[title="生成与发布"]').first().click()
await until(() => hasText('② 发布预设'))
const buildText = text(await mainText())
console.log(`页面可见文字（前 600 字）：\n${buildText.slice(0, 600)}`)

/* 「② 发布」在 2026-10-04 明确成「② 发布预设」（区分"发预设"与"发软件版本"）——
   探针跟着改名（退役旧文案必须同步裁读它的那一段） */
for (const needle of ['① 生成', '② 发布预设', '本次发布的发布物', '③ 对照基线']) {
  const ok = await hasText(needle)
  say(ok, `这一页写着「${needle}」`)
  if (!ok) problems.push(`生成与发布页缺少「${needle}」`)
}

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
/* —— 清单里也该有附属文件（2026-10-07）：生成不只写 toml，目录与清单也列出来 —— */
const auxRow = await page.locator('[role="dialog"] nav button', { hasText: 'catalog.json' }).count()
say(auxRow > 0, `清单里列着附属文件（catalog.json ×${auxRow}）—— 生成同时重算目录与清单`)
if (auxRow === 0) problems.push('清单里没有附属文件（catalog.json）')
/* —— 头部「完整 / 对比」切换（2026-10-07：**默认对比**，想看整份原文再切「完整」） —— */
const viewGroup = page.locator('[role="dialog"] [aria-label="详情显示方式"]')
const hasToggle = (await viewGroup.count()) > 0
const pressedOn = hasToggle ? await viewGroup.locator('[aria-pressed="true"]').first().textContent() : ''
say(hasToggle && pressedOn === '对比', `详情默认「对比」视图（标题行右侧切换）：${pressedOn || '（没有切换）'}`)
if (!hasToggle || pressedOn !== '对比') problems.push('详情切换缺失或默认不是「对比」')
const diffSeen = await until(
  async () =>
    (await page.locator('[role="dialog"] [class*="lineAdd"], [role="dialog"] [class*="lineDel"]').count()) > 0,
  3000,
)
say(diffSeen, '默认「对比」就摆着行级 diff（整份摊开、不省略）')
if (!diffSeen) problems.push('对比视图没有行级 diff')
await viewGroup.getByRole('button', { name: '完整', exact: true }).click()
const fullBack = await until(
  async () => (await page.locator('[role="dialog"] [class*="lineCtx"]').count()) > 0,
  3000,
)
say(fullBack, '切到「完整」看到新文件全文（无红绿底）')
if (!fullBack) problems.push('完整视图没有全文')
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

await page.getByRole('button', { name: '查看', exact: true }).first().click()
await page.waitForSelector('[role="dialog"] pre', { timeout: 5000 })
const json = await page.evaluate(() => document.querySelector('[role="dialog"] pre')?.textContent ?? '')
const jsonOk = json.length > 0 && json.includes('{')
say(jsonOk, `查看交付文件：${json.length} 字符（正文由 wb_delivery_file 直读盘上原文）`, json.split('\n')[0])
if (!jsonOk) problems.push('查看交付文件没拿到正文')
if (wantShots) await page.screenshot({ path: `${shotDir}/wb-build-delivery-file.png` })
await page.getByRole('button', { name: '关闭', exact: true }).first().click()
await page.waitForSelector('[role="dialog"]', { state: 'detached', timeout: 5000 })

if (wantShots) await page.screenshot({ path: `${shotDir}/wb-build-page.png` })

/* —— 发布闸（第二刀）：点「发布」先开闸，十五项逐项打勾，全绿才给往下走 —— */
{
  const dialog = page.locator('[role="dialog"]')
  await page.getByRole('button', { name: '发布', exact: true }).first().click()
  await page.waitForSelector('[role="dialog"]', { timeout: 5000 })
  const shown = await until(async () => (await dialogText()).includes('发布闸'), 5000)
  say(shown, '点「发布」先开发布闸（不是直接发）')
  if (!shown) problems.push('点发布没有先开发布闸')

  const rows = await dialog.locator('[role="listitem"]').count()
  say(rows === 15, `发布闸摆出十五项逐项打勾：${rows} 项`)
  if (rows !== 15) problems.push(`发布闸不是十五项（${rows}）`)

  const gate = text(await dialogText())
  /* 第三刀之后 ⑫ 真跑了：它要报出结构代次与最低客户端版本（不再是「未实现」） */
  const structureRow = gate.includes('结构代次') && gate.includes('最低正式客户端版本')
  say(structureRow, '⑫ 报出结构代次与最低客户端版本（第三刀：这一项真跑了）')
  if (!structureRow) problems.push('发布闸 ⑫ 没报出结构代次 / 最低客户端版本')

  /* 桩里 ④ 有 1 个残留 ⇒ `dist/no-strays` 是红的 ⇒ 这颗按钮不该亮 */
  const blockedRun = await dialog
    .getByRole('button', { name: '确认发布', exact: true })
    .isDisabled()
  say(blockedRun, '有 Blocker 红时「确认发布」不亮（全绿才给往下走）')
  if (!blockedRun) problems.push('有 Blocker 红时「确认发布」还是亮的')
  if (wantShots) await page.screenshot({ path: `${shotDir}/wb-publish-gate.png` })

  await dialog.getByRole('button', { name: '取消', exact: true }).click()
  await page.waitForSelector('[role="dialog"]', { state: 'detached', timeout: 5000 })

  /* 清掉残留再开一次 —— 「全绿才亮」这条规矩得看得见 */
  await page.getByRole('button', { name: '清理残留', exact: true }).first().click()
  const cleaned = await until(() => hasText('没有残留'))
  say(cleaned, '清理残留之后 ④ 说没有残留了')
  if (!cleaned) problems.push('清理残留之后 ④ 没跟着变')

  await page.getByRole('button', { name: '发布', exact: true }).first().click()
  await page.waitForSelector('[role="dialog"]', { timeout: 5000 })
  await until(async () => (await dialogText()).includes('发布闸'), 5000)
  const canRun = await page
    .locator('[role="dialog"]')
    .getByRole('button', { name: '确认发布', exact: true })
    .isEnabled()
  say(canRun, '残留清掉之后闸全绿，「确认发布」亮了')
  if (!canRun) problems.push('闸全绿了「确认发布」还是不亮')
  if (wantShots) await page.screenshot({ path: `${shotDir}/wb-publish-gate-green.png` })

  /*
   * 确认发布：**不再直接关框** —— 就地切成「发布回执」（第四刀：治的就是
   * "关掉再打开又是新的"）。② 卡同时记上这一笔。
   *
   * 第三刀下半起，「发布」是一次**事务**（审计 → 生成 → 定稿 → git → PR），
   * 桩走向成功那一路：commit `08ec040`、PR #128、CI 还在跑。
   */
  await page
    .locator('[role="dialog"]')
    .getByRole('button', { name: '确认发布', exact: true })
    .click()

  const receiptShown = await until(async () => (await dialogText()).includes('发布回执'), 6000)
  say(receiptShown, '点「确认发布」→ 就地切到「发布回执」（不再把框关掉）')
  if (!receiptShown) problems.push('确认发布之后没有进发布回执')

  const receiptText = text(await dialogText())
  const chain =
    receiptText.includes('发布检查') &&
    receiptText.includes('生成') &&
    receiptText.includes('提交') &&
    receiptText.includes('推送') &&
    receiptText.includes('PR #128')
  say(chain, '回执摆出阶段链：发布检查 / 生成 / 提交 / 推送 / PR #128')
  if (!chain) problems.push('发布回执的阶段链不全')

  const commitSha = receiptText.includes('08ec040')
  say(commitSha, '回执给出**提交号**（08ec040）')
  if (!commitSha) problems.push('发布回执没给提交号')

  const prAddr = receiptText.includes('github.com/MuCoreBenC/MKPSupportEase/pull/128')
  say(prAddr, '回执给出 **PR 地址**')
  if (!prAddr) problems.push('发布回执没给 PR 地址')

  const recorded = await until(() => hasText('本次发布落下'))
  say(recorded, '② 卡同时记上这一笔（本次发布落下 … · 08ec040 → PR #128）')
  if (!recorded) problems.push('发布之后 ② 卡没记上')
  if (wantShots) await page.screenshot({ path: `${shotDir}/wb-publish-receipt.png` })

  /* —— 合并（作者 2026-10-04 拍：squash、不强制等 CI；CI 没跑完要二次确认）—— */
  const mergeBtn = page
    .locator('[role="dialog"]')
    .getByRole('button', { name: '合并 #128', exact: true })
  const hasMerge = (await mergeBtn.count()) > 0
  say(hasMerge, '回执上有「合并 #128」入口（软件内合并）')
  if (!hasMerge) problems.push('回执上没有合并按钮')
  if (hasMerge) {
    await mergeBtn.click()
    const confirmShown = await until(async () => (await dialogText()).includes('CI 尚未完成'), 3000)
    say(confirmShown, 'CI 还在跑时点合并 → 二次确认如实说「CI 尚未完成」')
    if (!confirmShown) problems.push('CI 未完成时点合并没有二次确认（或文案不对）')

    await page
      .locator('[role="dialog"]')
      .getByRole('button', { name: '确定合并', exact: true })
      .click()
    const merged = await until(async () => (await dialogText()).includes('已合并'), 5000)
    say(merged, '合并走通 → 回执显示「已合并」（桩回读 state=merged）')
    if (!merged) problems.push('合并之后回执没显示已合并')
    if (wantShots) await page.screenshot({ path: `${shotDir}/wb-publish-merged.png` })
  }

  /*
   * 关掉再开：那份回执还在（「查看发布结果」），且**不重跑十五项**。
   * ★ 「关闭」有两颗（框头那颗 × 与页脚那颗）—— 取第一颗，别撞 Playwright 的严格模式。
   */
  await page
    .locator('[role="dialog"]')
    .getByRole('button', { name: '关闭', exact: true })
    .first()
    .click()
  await page.waitForSelector('[role="dialog"]', { state: 'detached', timeout: 5000 })
  await page.getByRole('button', { name: '查看发布结果', exact: true }).first().click()
  const reopened = await until(async () => {
    const t = text(await dialogText())
    return t.includes('发布回执') && !t.includes('十五项')
  }, 5000)
  say(reopened, '关掉之后「查看发布结果」还能打开那份回执（不是空的新面板）')
  if (!reopened) problems.push('关掉之后打不回那份回执')
  await page
    .locator('[role="dialog"]')
    .getByRole('button', { name: '关闭', exact: true })
    .first()
    .click()
  await page.waitForSelector('[role="dialog"]', { state: 'detached', timeout: 5000 })

  /* —— 发布历史（本地回执日志：打开读一次；每条一个手动「刷新」，不轮询）—— */
  await page.getByRole('button', { name: '发布历史', exact: true }).first().click()
  await page.waitForSelector('[role="dialog"]', { timeout: 5000 })
  const histShown = await until(async () => (await dialogText()).includes('条回执'), 5000)
  say(histShown, '「发布历史」列出本地回执（打开时读一次）')
  if (!histShown) problems.push('发布历史没列出记录')

  const histRows = await page.locator('[role="dialog"] [role="listitem"]').count()
  say(histRows >= 3, `历史里摆出 ${histRows} 条回执（桩给 3 条）`)
  if (histRows < 3) problems.push(`发布历史条数不对（${histRows}）`)

  const histText = text(await dialogText())
  const mergedRow = histText.includes('已合并')
  say(mergedRow, '历史里看得到「✓ 已合并」')
  if (!mergedRow) problems.push('历史里没有已合并那条')

  const row127 = page.locator('[role="dialog"] [role="listitem"]', { hasText: 'PR #127' })
  if ((await row127.count()) > 0) {
    await row127.getByRole('button', { name: '刷新', exact: true }).click()
    const refreshed = await until(async () => text(await row127.innerText()).includes('CI 通过'), 4000)
    say(refreshed, '每条「刷新」手动回读一次状态（#127：CI 运行中 → 通过）')
    if (!refreshed) problems.push('历史里点「刷新」没有回读到新状态')
  } else {
    problems.push('历史里找不到 #127 那条（刷新那条判不了）')
  }
  if (wantShots) await page.screenshot({ path: `${shotDir}/wb-publish-history.png` })
  await page
    .locator('[role="dialog"]')
    .getByRole('button', { name: '关闭', exact: true })
    .first()
    .click()
  await page.waitForSelector('[role="dialog"]', { state: 'detached', timeout: 5000 })
}

/* ============================ 二、参数台 ============================ */

console.log('\n【二】参数台（C15 A2 整卡收起 · B1 抽屉说清类型）')
await page.locator('button[title="参数台"]').first().click()
await page.waitForTimeout(200)

/*
 * 左树**默认就是钉住的**（作者 2026-10-04 起：「一进来就看得见机型与版本这棵树」）——
 * 老探针那句"左树默认收起、点把手钉住"随那个默认一起退役：现在没有可点的「钉住」把手，
 * 把手只在**收起态**才叫「钉住机型与版本」（点击会超时挂住整段探针）。
 */
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
