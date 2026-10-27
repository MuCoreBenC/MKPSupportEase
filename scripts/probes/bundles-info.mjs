/*
 * 工作台「套餐页信息层级 + 文案瘦身」探针（2026-10-16，作者三图点名那条）。
 *
 * # 为什么单独一条
 *
 * 这一刀全是"不改也不报错"的毛病 —— 名单全叫「推荐」、编辑框没标签、长段说明糊满屏、
 * 改套餐内容显示中文名不显示文件名。任何一条回退，界面照样渲染，只有探针能钉住。
 *
 * # 判据（对应作者原话）
 *
 *   1. 名单行**不再同名**：至少两条 `t-bundle-*` 行的主名互不相同（「名字怎么都一样」）
 *   2. 名单行主名 = 显示名（A1 标准版 这种），且别的行 `id` 在（「加一个附加 ID」）
 *   3. 卡头：h2 = 显示名、附注 = id（等宽 mono）
 *   4. 分组标题旁有 `helpDot`（「?」），且它带非空 `title`（说明收进悬停）
 *   5. 正文里**不再有**那几句长说明（「真源关系是一版一套」「MKP 与 BBS 必须成套配发」…）
 *   6. 「编辑」弹窗：两个输入框**上方都有常驻标签**（套餐 id / 显示名）
 *   7. 「套餐内容」弹窗：候选行显示的是**文件名那一串**（等宽 id），不是只有中文名
 *
 * # 怎么跑（探针自己不起服务；用完关掉 4174）
 *
 *   BUILD_WORKBENCH=1 NODE_ENV=development npx vite build --target esnext \
 *     --outDir node_modules/.cache/wb-probe/dist --emptyOutDir
 *   npx vite preview --outDir node_modules/.cache/wb-probe/dist --port 4174 --strictPort
 *   node scripts/probes/bundles-info.mjs http://localhost:4174/workbench.html --shots
 */
import { mkdir } from 'node:fs/promises'

import { chromium } from 'playwright-core'

const rawUrl = process.argv[2]?.startsWith('http') ? process.argv[2] : 'http://localhost:4174/workbench.html'
const url = /[?&]mock=/.test(rawUrl) ? rawUrl : `${rawUrl}${rawUrl.includes('?') ? '&' : '?'}mock=1`
const wantShots = process.argv.includes('--shots')
const shotDir = 'tmp-shots'

const problems = []
const note = (ok, label, detail = '') => {
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${label}${detail ? ` —— ${detail}` : ''}`)
  if (!ok) problems.push(label)
}

const BENIGN = [/\/favicon\.ico$/]
const browser = await chromium.launch({ channel: 'msedge' })
const page = await browser.newPage({ viewport: { width: 1560, height: 900 } })

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
const text = (sel) =>
  page
    .locator(sel)
    .first()
    .innerText()
    .then((t) => (t ?? '').replace(/\s+/g, ' ').trim())
    .catch(() => '')
const bodyHas = (s) => page.evaluate((x) => (document.body.innerText ?? '').includes(x), s)

if (wantShots) await mkdir(shotDir, { recursive: true })
await page.goto(url, { waitUntil: 'load' })
await page.locator('[class*="shellBody"]').first().waitFor({ timeout: 15000 })

/* ———————————————— 进套餐页 ———————————————— */
await page.locator('button[title="套餐"]').first().click()
const rowsReady = await until(() =>
  page
    .locator('[class*="pageSlot"]:not([hidden]) [id^="t-bundle-"]')
    .count()
    .then((n) => n >= 2),
)
note(rowsReady, '套餐页左列读到 ≥2 条（`t-bundle-*`）')
if (!rowsReady) {
  console.log('-'.repeat(96))
  await browser.close()
  for (const p of problems) console.log(`  - ${p}`)
  process.exit(1)
}

/* ———————————————— ① 名字不再全一样 ———————————————— */
const names = await page.evaluate(() =>
  [
    ...document.querySelectorAll(
      '[class*="pageSlot"]:not([hidden]) [id^="t-bundle-"] [class*="rowName"]',
    ),
  ].map((e) => (e.textContent ?? '').trim()),
)
const uniq = new Set(names)
note(
  names.length >= 2 && uniq.size === names.length,
  '名单主名互不相同（不再是九条全叫「推荐」）',
  names.slice(0, 6).join(' / '),
)
note(
  names.every((n) => n !== '推荐'),
  '没有任何一行主名还是「推荐」',
  names.filter((n) => n === '推荐').length + ' 条',
)

/* ———————————————— ② 主名 + 附加 id ———————————————— */
const firstRow = await text('[class*="pageSlot"]:not([hidden]) [id^="t-bundle-"]')
note(
  /[A-Z]/.test(firstRow) && firstRow.includes('装了'),
  '名单行：主名 = 显示名、附注里有 id（等宽）与「装了 N 个文件」',
  firstRow,
)

/* ———————————————— ③ 卡头：h2 = 显示名、附注 = id+mono ———————————————— */
const headH2 = await text('[class*="pageSlot"]:not([hidden]) [class*="cardHead"] h2')
const headNoteCls = await page.evaluate(() => {
  const el = document.querySelector(
    '[class*="pageSlot"]:not([hidden]) [class*="cardHead"] [class*="cardNote"]',
  )
  return el === null ? '' : el.className
})
note(
  headH2 !== '' && /mono/.test(headNoteCls),
  '卡头：大标题 = 显示名、附注（id）走等宽 `mono`',
  `${headH2} / ${headNoteCls}`,
)

/* ———————————————— ④ 分组标题旁有「?」且带 title ———————————————— */
const dots = await page.evaluate(() => {
  const slot = document.querySelector('[class*="pageSlot"]:not([hidden])')
  if (slot === null) return []
  return [...slot.querySelectorAll('[class*="helpDot"]')].map((e) => ({
    text: (e.textContent ?? '').trim(),
    title: e.getAttribute('title') ?? '',
  }))
})
note(dots.length > 0, '页面上有 `helpDot`（「?」小图标）', `共 ${dots.length} 枚`)
note(
  dots.every((d) => d.text === '?' && d.title.trim().length > 10),
  '每一枚「?」都带非空 title（说明真的收进悬停了，不是装样子）',
  dots[0] === undefined ? '无' : dots[0].title.slice(0, 40),
)

/* ———————————————— ⑤ 长说明不在正文里了 ———————————————— */
/* 只看**当前可见那一页**的正文 —— `document.body.innerText` 会把隐藏页的字也算进来 */
const visibleText = await page.evaluate(
  () => document.querySelector('[class*="pageSlot"]:not([hidden])')?.innerText ?? '',
)
const gone = ['真源关系是', 'MKP 与 BBS 必须成套配发', '右键 chip 也是这两项', '点了先看'].filter(
  (s) => visibleText.includes(s),
)
note(gone.length === 0, '那几句长说明已从正文移除（改挂 title）', gone.join('、') || '都清了')

if (wantShots) await page.screenshot({ path: `${shotDir}/wb-bundles-info.png` })

/* ———————————————— ⑥ 编辑弹窗：输入框上方有常驻标签 ———————————————— */
/* ★ 页面常驻挂着，别页的按钮也还在 DOM 里（「编辑尺寸」这种不可见的会先命中）——
   用 `:visible` 只在**真的看得见**的那颗「编辑」上点 */
await page.locator('[class*="pageSlot"]:not([hidden]) [class*="cardHead"] button:visible', { hasText: '编辑' }).first().click()
const editLabels = await page.evaluate(() => {
  const scrim = document.querySelector('[class*="scrim"]')
  if (scrim === null) return null
  const cols = [...scrim.querySelectorAll('[class*="fieldCol"]')]
  return cols.map((c) => {
    const lab = c.querySelector('[class*="fieldLabel"]')
    const inp = c.querySelector('input')
    return {
      label: (lab?.textContent ?? '').replace(/\s+/g, ' ').trim(),
      hasInput: inp !== null,
      // 标签是否真的在输入框上方
      above: lab !== null && inp !== null && lab.getBoundingClientRect().bottom <= inp.getBoundingClientRect().top + 2,
    }
  })
})
note(
  editLabels !== null && editLabels.length >= 2,
  '「编辑套餐」框里每个输入框都包在 `fieldCol` 里',
  JSON.stringify(editLabels),
)
note(
  editLabels !== null && editLabels.every((l) => l.hasInput && l.above && l.label.length > 0),
  '标签**常驻在输入框上方**（不再是只靠 placeholder）',
  editLabels === null ? '' : editLabels.map((l) => l.label).join(' / '),
)
if (wantShots) await page.screenshot({ path: `${shotDir}/wb-bundles-edit.png` })
await page.locator('[class*="scrim"] button', { hasText: '取消' }).first().click()
await until(async () => (await page.locator('[class*="scrim"]').count()) === 0)

/* ———————————————— ⑦ 套餐内容：MKP 行显示完整产物名 ———————————————— */
await page.locator('[class*="pageSlot"]:not([hidden]) button:visible', { hasText: '改套餐内容…' }).first().click()
const resOk = await until(() => page.locator('[class*="bunList"] [class*="fileRow"]').count().then((n) => n > 0))
note(resOk, '「套餐内容」框列出候选行')
const leadOf = () =>
  page.evaluate(() =>
    [...document.querySelectorAll('[class*="bunList"] [class*="fileRow"]')]
      .slice(0, 5)
      .map((r) => (r.querySelector('[class*="fileName"]')?.textContent ?? '').trim()),
  )
const mkpLeads = await leadOf()
note(
  mkpLeads.length > 0 && mkpLeads.every((t) => /\.toml$/i.test(t)),
  'MKP 行主名 = **完整产物名（带 .toml）**（不再是空白）',
  mkpLeads.join(' / '),
)
if (wantShots) await page.screenshot({ path: `${shotDir}/wb-bundles-resources.png` })

/* ———————————————— ⑦b 切片器页签：分类筛选 + 真实文件名 ———————————————— */
await page.locator('[class*="pickTab"]:visible', { hasText: '切片器' }).first().click()
await page.waitForTimeout(150)
/* 分类筛选已从 chip 改成下拉（作者 2026-10-16：「改成下拉菜单吧」）。
   `SelectField` 是自绘下拉：触发器带 `aria-haspopup="listbox"`。 */
const trigger = page.locator('[aria-haspopup="listbox"]:visible').first()
note(await trigger.count() > 0, '切片器页签摆出分类**下拉**（不再是 chip 排）')
/* 点开看选项里有没有机型类目 */
await trigger.click()
await page.waitForTimeout(200)
const opts = await page.evaluate(() =>
  [...document.querySelectorAll('[role="listbox"] li')].map((e) =>
    (e.textContent ?? '').replace(/\s+/g, ' ').trim(),
  ),
)
note(opts.length >= 2, '下拉里有「全部 + 各机型」类目', opts.join(' / '))
note(
  opts.some((o) => /A1/.test(o)),
  '下拉类目里有机型名（A1 这类）',
  opts.filter((o) => /A1/.test(o)).join(' / '),
)
if (wantShots) await page.screenshot({ path: `${shotDir}/wb-bundles-filter-open.png` })
/* 选一个非「全部」的类目，列表应收窄 */
const slicerBefore = await page.locator('[class*="bunList"] [class*="fileRow"]').count()
await page.locator('[role="listbox"] li').first().click()
await page.waitForTimeout(200)
const slicerAfter = await page.locator('[class*="bunList"] [class*="fileRow"]').count()
note(slicerAfter > 0 && slicerAfter <= slicerBefore, '选类目后列表按机型收窄', `${slicerBefore} → ${slicerAfter}`)
const slicerLeads = await leadOf()
note(
  slicerLeads.length > 0 && slicerLeads.every((t) => /\.json$/i.test(t)),
  '切片器行主名 = 真实文件名（含 `.json` 扩展名）',
  slicerLeads.join(' / '),
)
if (wantShots) await page.screenshot({ path: `${shotDir}/wb-bundles-filter.png` })

/* ———————————————— 结算 ———————————————— */
console.log('-'.repeat(96))
if (problems.length === 0) {
  console.log('套餐页信息层级 + 文案瘦身：全部通过。')
} else {
  console.log(`发现 ${problems.length} 处问题：`)
  for (const p of problems) console.log(`  - ${p}`)
}
await browser.close()
process.exit(problems.length === 0 ? 0 : 1)
