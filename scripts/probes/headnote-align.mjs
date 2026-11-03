#!/usr/bin/env node
/*
 * 判据：**标题右边那一格（`HeadNote`）与标题同一条基线**（2026-11-04）。
 *
 * # 它防的是哪一种坏
 *
 * 作者连着两轮指着导出框说「这个导出失败这句话还是偏高 / 徽章还是偏高」。
 * 病根不在某一屏：标题那一行是 `align-items: center`，两端**中线对齐、基线不对齐**——
 * 小字（`--fs-label`）的基线比标题（`--fs-body + 1px`）高约 0.6px，眼睛读的是基线，
 * 于是怎么调都"高一点点"。所以修法只能落在**共用件** `components/modal/HeadNote.module.css`。
 *
 * 这条探针把它变成可量的一件事：
 *
 *   1. **静态**：扫源码，列出所有用 `HeadNote` 的屏（改共用件会一起改到谁）；
 *   2. **实测**：把能开出来的那几屏逐一开出来，量**标题基线与回执基线的差**，
 *      超过 1px 就红。带 `--shots` 会顺带截图，肉眼复核。
 *
 * # 怎么跑
 *
 *   node scripts/probes/headnote-align.mjs            # 需要 dev server（默认 5321）
 *   node scripts/probes/headnote-align.mjs --shots    # 顺带落图到 tmp-shots/
 *
 * 能覆盖到的屏：
 *   · 后处理那三屏（`?pp=result` / `?pp=relocate` / `?pp=mismatch`，见 `mock.ts` 的
 *     `devPostProcessRun()`）；
 *   · 报告页的导出框（先导一次 → 拨两次压缩开关 → 再导 = 撞名，回执是红的）。
 *
 * 开不出来的那几屏（校准框、预设编辑框、重切提示）**不假装量过**：它们与上面是
 * 同一个件，静态那一段会把它们列出来，但表里只写「未覆盖」。
 */
import fs from 'node:fs'
import path from 'node:path'
import { chromium } from 'playwright-core'

const BASE = process.env.PROBE_URL ?? 'http://localhost:5321/'
const SHOTS = 'tmp-shots'
const wantShots = process.argv.includes('--shots')

/* ---------- 1. 静态：谁在用这个件 ---------- */

/** 扫 `src/` 下所有 `.tsx`，返回「文件 → 用到 `HeadNote` 的处数」 */
function whoUsesHeadNote() {
  const hits = []
  const walk = (dir) => {
    for (const ent of fs.readdirSync(dir, { withFileTypes: true })) {
      const at = path.join(dir, ent.name)
      if (ent.isDirectory()) {
        walk(at)
        continue
      }
      if (!ent.name.endsWith('.tsx')) continue
      const text = fs.readFileSync(at, 'utf8')
      const n = (text.match(/<HeadNote\b/g) ?? []).length
      if (n > 0) hits.push({ file: at.replace(/\\/g, '/'), n })
    }
  }
  walk('src')
  return hits.sort((a, b) => a.file.localeCompare(b.file))
}

/* ---------- 2. 实测：标题基线 vs 回执基线 ---------- */

/**
 * 量两段文字的**基线**（不是盒子中线）。
 *
 * 单行内联元素的 `getBoundingClientRect()` 给的是**行盒**：基线 = 行盒顶 + 半行距 + 上伸。
 * 上伸 / 下伸用 canvas 按同一个 font 取（`fontBoundingBox*`），于是两段字的量法完全一致，
 * 差值就是"眼睛看到的那点错位"。
 */
const MEASURE = () => {
  const cv = document.createElement('canvas')
  const ctx = cv.getContext('2d')
  const baselineOf = (el) => {
    if (el === null) return null
    const cs = getComputedStyle(el)
    ctx.font = `${cs.fontStyle} ${cs.fontWeight} ${cs.fontSize} ${cs.fontFamily}`
    const m = ctx.measureText(el.textContent.trim() || 'x')
    const asc = m.fontBoundingBoxAscent
    const desc = m.fontBoundingBoxDescent
    const r = el.getBoundingClientRect()
    const lh = cs.lineHeight === 'normal' ? asc + desc : parseFloat(cs.lineHeight)
    /* 半行距 + 上伸 = 行盒顶到基线的那一段 */
    return +(r.top + (lh - (asc + desc)) / 2 + asc).toFixed(2)
  }

  const dlg = document.querySelector('[role="dialog"]')
  if (dlg === null) return { ok: false, why: '没有模态框' }
  const title = dlg.querySelector('strong')
  /* 回执槽 = `Modal.module.css` 的 `.headNote`（portal 进去的那一格） */
  const slot = dlg.querySelector('span[class*="headNote"]')
  /*
   * 槽里可能住着两样东西：一枚徽章（没有 `data-tip`）与一句回执（`HeadNote` 自己带
   * `data-tip` = 全文）。要量的是**那句字**，不是胶囊 —— 胶囊的高度是定死的 20px，
   * 它的"基线"跟字面本来就不是一回事。
   */
  const note = slot?.querySelector('[data-tip]') ?? null
  if (title === null) return { ok: false, why: '没有标题' }
  if (note === null) return { ok: true, note: null }
  return {
    ok: true,
    title: title.textContent.trim(),
    text: note.textContent.trim(),
    titleBase: baselineOf(title),
    noteBase: baselineOf(note),
    /* > 0 = 回执**低**于标题；< 0 = 偏高（作者说的那种） */
    delta: +(baselineOf(note) - baselineOf(title)).toFixed(2),
    /* CSS Modules 的类名带哈希（`_error_6fsob_40`），这里只留三档里那一档 */
    tone: (['error', 'ok', 'note'].find((t) => note.className.includes(`_${t}_`)) ?? '?'),
  }
}

const rows = []
const browser = await chromium.launch()

/** 开一屏，量一条 */
async function probe(name, url, open) {
  /*
   * `locale: 'zh-CN'`：界面文案由 `resolveLocale()` 跟着 `navigator.language` 走，
   * 不给这一格时无头浏览器是 en-US —— 下面那些按中文名字找的按钮就全找不到。
   * 作者真机是中文，量他那一种。
   */
  const page = await browser.newPage({ viewport: { width: 1080, height: 800 }, locale: 'zh-CN' })
  try {
    await page.goto(BASE + url, { waitUntil: 'domcontentloaded' })
    await page.waitForTimeout(500)
    await page.evaluate(() => document.documentElement.setAttribute('data-theme', 'dark'))
    if (open) await open(page)
    await page.waitForTimeout(400)
    const got = await page.evaluate(MEASURE)
    rows.push({ name, ...got })
    if (wantShots) {
      fs.mkdirSync(SHOTS, { recursive: true })
      await page.screenshot({ path: `${SHOTS}/headnote-${name}.png` })
    }
  } catch (e) {
    rows.push({ name, ok: false, why: String(e).split('\n')[0] })
  } finally {
    await page.close()
  }
}

/* 后处理那三屏：URL 一带 `?pp=` 就是终态，不用点任何东西 */
for (const q of ['result', 'relocate', 'mismatch']) {
  await probe(`pp-${q}`, `?pp=${q}`)
}

/* 报告页的导出框：先导一次，拨两次开关回到同一落点，再导 ⇒ 撞名（红回执） */
await probe('report-export', '', async (page) => {
  const step = async (what, fn) => {
    try {
      await fn()
    } catch (e) {
      throw new Error(`[${what}] ${String(e).split('\n')[0]}`)
    }
  }
  await step('切到报告页', () =>
    page.getByRole('navigation', { name: '主页签' }).getByRole('button', { name: '报告' }).click(),
  )
  await page.waitForTimeout(900)
  /* 挑一条**失败**的记录：它多一枚「处理失败」徽章，回执那一格最难对 */
  await step('选一条失败记录', async () => {
    const row = page.locator('button:has-text("失败")').filter({ hasText: /毫秒|秒/ }).first()
    await row.click({ timeout: 8000 })
  })
  await page.waitForTimeout(500)
  await step('打开导出框', () => page.getByRole('button', { name: '导出', exact: true }).click())
  const dlg = page.locator('[role="dialog"]')
  await step('第一次导出', () => dlg.getByRole('button', { name: '确认导出' }).click())
  await page.waitForTimeout(1200)
  await step('拨两次压缩开关', async () => {
    const sw = dlg.getByRole('switch').first()
    await sw.click()
    await sw.click()
  })
  await page.waitForTimeout(300)
  await step('第二次导出（撞名）', () => dlg.getByRole('button', { name: '确认导出' }).click())
  await page.waitForTimeout(700)
})

await browser.close()

/* ---------- 3. 报出来 ---------- */

const users = whoUsesHeadNote()
console.log('\n用到 HeadNote（标题右边那一格）的屏：')
for (const u of users) console.log(`  · ${u.file}  ×${u.n}`)

console.log('\n基线与标题的差（负 = 偏高，正 = 偏低；|差| > 1px 算没对上）：')
let bad = 0
for (const r of rows) {
  if (!r.ok) {
    console.log(`  ? ${r.name.padEnd(16)} 开不出来：${r.why}`)
    continue
  }
  if (r.note === null) {
    console.log(`  · ${r.name.padEnd(16)} 这一屏没有回执（跳过）`)
    continue
  }
  const off = Math.abs(r.delta) > 1
  if (off) bad += 1
  console.log(
    `  ${off ? '✗' : '✓'} ${r.name.padEnd(16)} Δ${String(r.delta).padStart(6)}px  ` +
      `[${r.tone}] ${r.text.slice(0, 34)}`,
  )
}

console.log(
  bad === 0
    ? '\n✓ 回执与标题同基线（|Δ| ≤ 1px）\n'
    : `\n✗ ${bad} 屏没对上 —— 改 components/modal/HeadNote.module.css（那是唯一的落点）\n`,
)
process.exit(bad === 0 ? 0 : 1)
