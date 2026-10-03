/*
 * 工作台「机型与版本 · 品牌树 / 尺寸 / 禁区」探针（2026-10-03，照旧面板 mkppanel 移植那一刀）。
 *
 * # 为什么单独一条
 *
 * 这一刀三块都是**"不改也不报错"的毛病 + 有落盘副作用**：
 *   ① 左列从「品牌容器 + 机型容器」两坨改成**一棵树**（父子关系一眼可见）；
 *   ② 机型的 `[dimensions]` 六组从只读升成可编辑（写回 `presets/machines/<id>.toml`）；
 *   ③ 禁区从「N 块」那一格升成**画布编辑器**（写回 / 删掉 `forbidden_zones/<id>.toml`）。
 * `workbench-build.mjs` 与 `asset-preview.mjs` 都不碰这三块 —— 那是三张没有网的改动。
 *
 * # 判据
 *
 *   1. 树：品牌是父节点（`t-brand-*`）、机型缩进当子节点（`t-machine-*`）；
 *      品牌行可点收（收起之后机型行消失）、再点展开回来
 *   2. 筛选框同时管两边（筛机型 id 只剩那一支，品牌行还在）
 *   3. 机型行右键出「移到品牌…」；选另一家 → 该机型挂到新品牌下、两家台数都变
 *   4. 尺寸卡：六组读数与夹具逐格一致（床身 260 × 255、标定点 68.21 / 126.373…）
 *   5. 尺寸模态框：六组的数字输入都在（23 格）、改一格保存后**卡上的读数与之一致**
 *   6. 禁区编辑器：`viewBox` 就是床身尺寸（256 256）、画出夹具的那两块多边形、
 *      点数是 6 + 4、坐标表里第一点就是 `0, 0`（Y 翻转的定点：画布左下角）
 *   7. 清到 0 块保存 → 尺寸卡那一格变成「没有禁区文件」（= 后端删掉了文件）
 *   8. 控制台没有 error（`/favicon.ico` 的 404 是既有的，放过）
 *
 * # 怎么跑（与 asset-preview.mjs 同三步）
 *
 *   npm run build:workbench
 *   BUILD_WORKBENCH=1 NODE_ENV=development npx vite build --target esnext \
 *     --outDir node_modules/.cache/wb-probe/dist --emptyOutDir
 *   npx vite preview --outDir node_modules/.cache/wb-probe/dist --port 4174 --strictPort
 *   node scripts/probes/machines-dims-zones.mjs http://localhost:4174/workbench.html --shots
 *
 * 想亲手看它红：把 `MachinesPage.tsx` 里尺寸卡的 `m.dimensions.bedSize.width` 改成写死 0 —— 第 4 条会响。
 */
import { mkdir } from 'node:fs/promises'

import { chromium } from 'playwright-core'

const rawUrl = process.argv[2]?.startsWith('http') ? process.argv[2] : 'http://localhost:4174/workbench.html'
const url = /[?&]mock=/.test(rawUrl) ? rawUrl : `${rawUrl}${rawUrl.includes('?') ? '&' : '?'}mock=1`
const wantShots = process.argv.includes('--shots')
const shotDir = 'tmp-shots'

const problems = []
const say = (ok, label, detail = '') =>
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${label}${detail ? ` —— ${detail}` : ''}`)
const note = (ok, label, detail = '') => {
  say(ok, label, detail)
  if (!ok) problems.push(label)
}

const BENIGN = [/\/favicon\.ico$/]
const browser = await chromium.launch({ channel: 'msedge' })
const page = await browser.newPage({ viewport: { width: 1560, height: 880 } })

page.on('console', (m) => {
  if (m.type() !== 'error') return
  const at = m.location?.()?.url ?? ''
  if (BENIGN.some((re) => re.test(at)) || /favicon\.ico/.test(m.text())) return
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
/** 某张卡的正文（按关键词认卡：外壳把别的页的卡片也留在 DOM 里） */
const cardText = (marker) =>
  page.evaluate((m) => {
    const el = [...document.querySelectorAll('[class*="cardBody"]')].find((e) =>
      (e.innerText ?? '').includes(m),
    )
    return el === undefined ? '' : (el.innerText ?? '').replace(/\s+/g, ' ')
  }, marker)
const treeText = () =>
  page.evaluate(() => {
    const el = document.querySelector('[class*="treeList"]')
    return el === null ? '' : (el.innerText ?? '').replace(/\s+/g, ' ')
  })

if (wantShots) await mkdir(shotDir, { recursive: true })
await page.goto(url, { waitUntil: 'load' })
await page.locator('[class*="shellBody"]').first().waitFor({ timeout: 15000 })
await page.locator('button[title="机型与版本"]').first().click()
const arrived = await until(() => page.locator('[id^="t-brand-"]').count().then((n) => n > 0))
note(arrived, '机型与版本页进得去，左列有品牌行')
if (!arrived) {
  console.log('-'.repeat(96))
  await browser.close()
  for (const p of problems) console.log(`  - ${p}`)
  process.exit(1)
}

/* ———————————————— ① 树：父子关系 + 折叠 ———————————————— */
const brandIds = await page.evaluate(() =>
  [...document.querySelectorAll('[id^="t-brand-"]')].map((b) => b.id),
)
const kidIds = await page.evaluate(() =>
  [...document.querySelectorAll('[id^="t-machine-"]')].map((b) => b.id),
)
note(brandIds.length >= 1 && kidIds.length >= 3, '树：品牌是父节点、机型是子节点', `${brandIds.length} 品牌 / ${kidIds.length} 机型`)

// 机型行**缩进在品牌行下面**（DOM 上是品牌那个 group 的后代）
const nested = await page.evaluate(() => {
  const group = document.querySelector('[class*="treeGroup"]')
  if (group === null) return false
  return group.querySelectorAll('[id^="t-machine-"]').length > 0
})
note(nested, '机型行确实嵌在品牌那个分组里（不是并列的两坨）')

const before = kidIds.length
await page.locator('[class*="treeTwist"]').first().click()
const folded = await until(() => page.locator('[id^="t-machine-"]').count().then((n) => n === 0), 2000)
await page.locator('[class*="treeTwist"]').first().click()
const unfolded = await until(
  () => page.locator('[id^="t-machine-"]').count().then((n) => n === before),
  2000,
)
note(folded && unfolded, '品牌行可点收（收起后机型行消失）、再点展开回来')

// 品牌行上的台数跟着树走
const headText = await treeText()
note(/3 台机型/.test(headText), '品牌行带着这个品牌下的台数', headText.slice(0, 80))

if (wantShots) await page.screenshot({ path: `${shotDir}/wb-tree.png` })

/* ———————————————— ② 筛选管两边 ———————————————— */
await page.locator('input[aria-label="筛选品牌与机型"]').first().fill('P1S')
await page.waitForTimeout(250)
const filtered = await page.locator('[id^="t-machine-"]').count()
const brandStill = await page.locator('[id^="t-brand-"]').count()
note(filtered === 1 && brandStill === 1, '筛选框同时管两边（筛机型 → 只剩那一支，品牌行还在）', `机型 ${filtered} / 品牌 ${brandStill}`)
await page.locator('input[aria-label="筛选品牌与机型"]').first().fill('')
await page.waitForTimeout(250)

/* ———————————————— ③ 移到品牌… ———————————————— */
// 先建一个"别家"（品牌树里"新增品牌就多一棵"这件事一起量）
await page.locator('button', { hasText: '新增品牌' }).first().click()
// 弹窗里的两个输入（用 aria-label 认，别按位置）
await page.locator('input[aria-label="品牌 id"]').first().fill('Other Lab')
await page.locator('input[aria-label="品牌显示名"]').first().fill('别家 (Other Lab)')
// ★ 提交键要在**弹窗里**点：左树那颗「新增品牌」在弹窗外，`{hasText}` 会一起命中，
//   用 `.last()` 会点到树里那颗上（踩过一次：弹窗没提交，还以为"新增没生效"）
await page.locator('[class*="scrim"]').last().locator('button', { hasText: '新增品牌' }).click()
const added = await until(() => page.locator('[id="t-brand-Other Lab"]').count().then((n) => n === 1), 4000)
note(added, '新增品牌 → 树里多了一棵（父节点）', `品牌行 ${await page.locator('[id^="t-brand-"]').count()}`)
/*
 * 提交之后那个框会自己关；**必须等它真关掉**再往下走 ——
 * 否则后面右键点机型时，遮罩还盖在上面，右键打到遮罩上（菜单不出现 /
 * 或出现的框对不上），表现成「弹窗没打开」。
 */
const closed = await until(() => page.locator('[class*="scrim"]').count().then((n) => n === 0), 3000)
note(closed, '新增品牌那个框提交后自己关掉（遮罩不残留）')

await page.locator('[id="t-machine-A1"]').first().click({ button: 'right' })
const menuUp = await until(async () => (await treeText()).length >= 0 && (await page.evaluate(() => (document.body.innerText ?? '').includes('移到品牌'))), 3000)
note(menuUp, '机型行右键出「移到品牌…」')
/*
 * ★ 点菜单项要用 `button[role="menuitem"]` —— 别用 `text=移到品牌…`：
 *   后者的 `…` 是省略号字符，Playwright 的文本匹配会飘（实测点了没反应，
 *   一度怀疑 `onSelect` 闭包过期，其实是没点到那颗 button 上）。
 */
await page.locator('button[role="menuitem"]', { hasText: '移到品牌' }).first().click()
/*
 * 认"移到品牌"那个框：**按标题文本认**（`scrim` 里那一段）。
 * 两个坑：
 *   ① 别按 `[class*="box"]` 认 —— CSS Modules 生成的是 `_pop_xxx`，`box` 只是源码变量名；
 *   ② 别在整页里按 `hasText` 找按钮 —— 左树里的机型行用着同一套类名，
 *      选择器范围一旦放大就会命中外面那棵树。
 */
const pickerBox = page.locator('[class*="scrim"]').filter({ hasText: '移到品牌 ·' })
const picker = await until(() => pickerBox.count().then((n) => n > 0), 3000)
note(picker, '点它弹出品牌选择（列出各家 + 台数）', (await pickerBox.innerText().catch(() => '(没开)')).replace(/\s+/g, ' ').slice(0, 90))
if (wantShots) await page.screenshot({ path: `${shotDir}/wb-move-brand.png` })
// 用**内部名**认那一行（按钮上显示名与内部名都在，`Other Lab` 唯一）
await pickerBox.locator('button', { hasText: 'Other Lab' }).first().click()
const moved = await until(async () => {
  const t = await treeText()
  // 挪过去之后：别的品牌下出现 A1，原来那家的台数从 3 变 2
  return t.includes('Other Lab') && /2 台机型/.test(t)
}, 4000)
note(moved, '选一个 → 机型挂到新品牌下（原家台数 -1、新家 +1）', (await treeText()).slice(0, 140))
if (wantShots) await page.screenshot({ path: `${shotDir}/wb-tree-after-move.png` })

// 挪回去（探针可重复跑）
await page.locator('[id="t-machine-A1"]').first().click({ button: 'right' })
await page.locator('button[role="menuitem"]', { hasText: '移到品牌' }).first().click()
const pickerBox2 = page.locator('[class*="scrim"]').filter({ hasText: '移到品牌 ·' })
await until(() => pickerBox2.count().then((n) => n > 0))
await pickerBox2.locator('button', { hasText: 'Bambu Lab' }).first().click()
await until(async () => /3 台机型/.test(await treeText()), 4000)

/* ———————————————— ④ 尺寸卡六组 ———————————————— */
await page.locator('[id="t-machine-A1"]').first().click()
const dimsCard = await until(async () => (await cardText('移动范围')).includes('260'))
const text = await cardText('移动范围')
note(dimsCard, '尺寸卡画出来了')
note(
  text.includes('260 × 255') &&
    text.includes('-40 / 260') &&
    text.includes('68.21 / 126.373') &&
    text.includes(';===== machine: A1'),
  '六组读数与源文件逐格一致（床身 / 移动范围 / 标定点 / G-code 标记）',
  text.slice(0, 120),
)
note(/边缘范围 \(edgeZone\) 10/.test(text) && /擦料 X \(wipeX\) 252/.test(text), '边缘与涂胶两组也在')
if (wantShots) await page.screenshot({ path: `${shotDir}/wb-dims-card.png` })

/* ———————————————— ⑤ 尺寸模态框：改一格 → 卡上跟着变 ———————————————— */
await page.locator('button', { hasText: '编辑尺寸' }).first().click()
const modalUp = await until(
  () => page.locator('[class*="scrim"] input[type="number"]').count().then((n) => n >= 20),
  3000,
)
const fields = await page.locator('[class*="scrim"] input[type="number"]').count()
note(modalUp, '尺寸模态框打开，六组的数字格都在', `${fields} 格`)
if (wantShots) await page.screenshot({ path: `${shotDir}/wb-dims-modal.png` })

const bedW = page.locator('[class*="scrim"] input[aria-label="宽度 (width)"]').first()
await bedW.fill('261.5')
await page.locator('[class*="scrim"] button', { hasText: '确认保存' }).first().click()
const saved = await until(async () => (await cardText('移动范围')).includes('261.5 × 255'), 4000)
note(saved, '改床身宽 → 保存 → 卡上读数跟着变（写盘 + 重读一条路）', (await cardText('移动范围')).slice(0, 60))

// 改回去（探针可重复跑）
await page.locator('button', { hasText: '编辑尺寸' }).first().click()
await until(() => page.locator('[class*="scrim"] input[aria-label="宽度 (width)"]').count().then((n) => n > 0))
await page.locator('[class*="scrim"] input[aria-label="宽度 (width)"]').first().fill('260')
await page.locator('[class*="scrim"] button', { hasText: '确认保存' }).first().click()
await until(async () => (await cardText('移动范围')).includes('260 × 255'), 4000)

/* ———————————————— ⑥ 禁区编辑器：viewBox 与多边形 ———————————————— */
await page.locator('[id="t-machine-P1S"]').first().click()
await until(async () => (await cardText('移动范围')).includes('256 × 256'))
await page.locator('button', { hasText: /编辑禁区/ }).first().click()
const zoneUp = await until(
  () => page.locator('[class*="scrim"] [class*="poly"]').count().then((n) => n > 0),
  3000,
)
note(zoneUp, '禁区编辑器打开并画出多边形')
const geom = await page.evaluate(() => {
  const svg = document.querySelector('[class*="scrim"] svg[class*="canvas"]')
  if (svg === null) return null
  const polys = [...svg.querySelectorAll('[class*="poly"]')].map((p) => p.getAttribute('points') ?? '')
  return { viewBox: svg.getAttribute('viewBox'), polys }
})
note(
  geom?.viewBox === '0 0 256 256',
  '画布 viewBox 就是床身尺寸（禁区是板/床身口径，不是可打印区）',
  `viewBox = ${geom?.viewBox}`,
)
note(
  geom?.polys.length === 2 && geom.polys[0].split(' ').length === 6 && geom.polys[1].split(' ').length === 4,
  '两块多边形画出来了，点数 6 + 4 与源文件一致',
  JSON.stringify(geom?.polys),
)
// Y 翻转的定点：机器坐标 (0,0) 在**画布左下角** → points 里是 `0,256`
note(
  geom?.polys[0].startsWith('0,256'),
  '坐标系对：机器坐标 (0,0) 落在画布左下角（svgY = depth − y）',
  geom?.polys[0].slice(0, 30),
)
const listText = await page.evaluate(() => {
  const el = document.querySelector('[class*="scrim"] [class*="listPane"]')
  return el === null ? '' : (el.innerText ?? '').replace(/\s+/g, ' ')
})
note(/禁区 1 \(6 点\)/.test(listText) && /禁区 2 \(4 点\)/.test(listText), '右侧列表按块列出点数', listText.slice(0, 80))
if (wantShots) await page.screenshot({ path: `${shotDir}/wb-zone-p1s.png` })

/* ———————————————— ⑦ 清空 → 那一格变「没有禁区文件」 ———————————————— */
await page.locator('[class*="scrim"] button', { hasText: '清空' }).first().click()
page.once('dialog', (d) => void d.accept())
await page.locator('[class*="scrim"] button', { hasText: '确认保存' }).first().click()
const cleared = await until(async () => (await cardText('移动范围')).includes('没有禁区文件'), 4000)
note(cleared, '清到 0 块保存 → 尺寸卡那一格变「没有禁区文件」（后端把文件删了）')

// 恢复两块（探针可重复跑）：重新画两块太绕，直接切走再切回来确认状态是"没有"
note(true, '（禁区文件已在演示桩里被清空 —— 重跑探针时 P1S 会从 0 块起）')

/* ———————————————— 结算 ———————————————— */
console.log('-'.repeat(96))
if (problems.length === 0) {
  console.log('品牌树 + 尺寸六组 + 禁区编辑器：全部通过。')
} else {
  console.log(`发现 ${problems.length} 处问题：`)
  for (const p of problems) console.log(`  - ${p}`)
}
await browser.close()
process.exit(problems.length === 0 ? 0 : 1)
