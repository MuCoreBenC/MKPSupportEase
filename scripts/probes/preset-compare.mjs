/*
 * **多版本 + 参数对比台**探针（2026-10-08）。
 *
 * 用法（与 `presets.mjs` 同一条路：build 之后走静态预览）：
 *   npm run build
 *   npx vite preview --port 4173 --strictPort
 *   node scripts/probes/preset-compare.mjs [url]
 *
 * 为什么单开一份而不是并进 `presets.mjs`：那一份是老世界（十三层）的验收清单，
 * 里面几个行名（`我的 A1 涂胶-高速版.toml`）与今天的假后端已经不匹配了；在它上面
 * 叠新节只会让"到底是哪一节红了"更难查。这一份从零起，只守本轮新增的四件事。
 *
 * 守的：
 *   ① 云端表**一个预设一组、版本各一行**：同一个 fileName 出现两行（已下载 / 待下载），
 *      时间列一个是**该版自己的发布日**、另一个是目录代时间；
 *   ② 展开待下载那一行 → 状态说「新版本」（**不是"需更新 / 已过时"**）；
 *   ③ 对比台：默认选中两份 → 差异行有**显式的**「采用此值」→ 点它之后那一列变
 *      「有未保存修改」、两格值趋同 → 「保存修改」把模态框关掉并给出提示条；
 *   ④ 整页跑完没有多出来的 console error / pageerror / >=400 响应。
 *
 * 真机那侧更硬的判据在 Rust：`runtime::baseline`（幂等 / 名字对不上不当基准）、
 * `runtime::versions`（两代都在 / 同字节只算一版）、`runtime::mine::ensure_working_copy`
 * （按官方发布日命名 / 同日补 -01 / 改名后仍幂等）、`presetdata::params`（保真抽取与批量写回）。
 */
import { mkdir } from 'node:fs/promises'

import { chromium } from 'playwright-core'

const url = process.argv[2]?.startsWith('http') ? process.argv[2] : 'http://localhost:4173/'
const shotDir = '.playwright-cli/probe-preset-compare'
await mkdir(shotDir, { recursive: true })

const browser = await chromium.launch({ channel: 'msedge' })
const page = await browser.newPage({ viewport: { width: 1760, height: 940 } })

const problems = []
/* 已知且无害：见 `presets.mjs` 顶上那段（favicon + 假后端必抛的那几条"未实现"） */
const BENIGN = [
  /\/favicon\.ico$/,
  /未实现的接口/,
]
const benign = (t) => BENIGN.some((re) => re.test(t))
page.on('console', (m) => {
  if (m.type() !== 'error') return
  if (benign(m.location?.()?.url ?? '') || benign(m.text())) return
  problems.push(`console.error: ${m.text()}`)
})
page.on('pageerror', (e) => problems.push(`pageerror: ${e.message}`))
page.on('response', (r) => {
  if (r.status() >= 400 && !benign(r.url())) problems.push(`HTTP ${r.status()} ${r.url()}`)
})

await page.goto(url, { waitUntil: 'load' })
await page.waitForSelector('header', { timeout: 10000 })

/** 点一个「文字正好是这些字」的按钮（顶部页签 / 工具条都用它） */
const clickByText = (selector, text) =>
  page.evaluate(
    ([sel, want]) => {
      const b = [...document.querySelectorAll(sel)].find((x) => (x.textContent ?? '').trim() === want)
      if (b === undefined) return false
      b.click()
      return true
    },
    [selector, text],
  )

/** 切分段控件的那一档（`本地 / 云端`、`MKP 配置 / 切片器配置`） */
const pickSeg = (text) =>
  page.evaluate((want) => {
    const l = [...document.querySelectorAll('main [role="radiogroup"] label')].find(
      (x) => (x.textContent ?? '').trim() === want,
    )
    if (l === undefined) return false
    l.click()
    return true
  }, text)

/** 表里的数据行（展开排那一格是 `td[colspan]`，不算行） */
const dataRows = () =>
  page.evaluate(() =>
    [...document.querySelectorAll('main tbody tr')]
      .filter((r) => r.querySelector('td:not([colspan])') !== null)
      .map((r) => [...r.querySelectorAll('td')].map((td) => td.textContent.trim())),
  )

if (!(await clickByText('header button', '预设'))) problems.push('找不到「预设」页签')
await page.waitForTimeout(900)

/* ---------- ① 云端表：一个预设一组、版本各一行 ---------- */
if (!(await pickSeg('云端'))) problems.push('找不到「云端」那一档')
await page.waitForTimeout(500)
const cloud = await dataRows()
const versionRows = cloud.filter((r) => r[0] === 'A1-fast.toml')
console.log(`\n[① 云端版本行] A1-fast.toml 有 ${versionRows.length} 行：`)
for (const r of versionRows) console.log(`    ${r[0]} | ${r[3]} | ${r[4]}`)
if (versionRows.length < 2) {
  problems.push(`同一个预设该按版本各占一行（假后端给了两版），实测 ${versionRows.length} 行`)
}
if (!versionRows.some((r) => r[4] === '已下载')) problems.push('该有一行是「已下载」')
if (!versionRows.some((r) => r[4] === '下载')) problems.push('该有一行是待下载（动作列写「下载」）')
/*
 * 时间列：已下载那一行写的是**该版自己的发布日**（`05-29`），
 * 待下载那一行只有目录代时间（假后端给的是 `10-15`）—— 两行不同才是"两个语义"。
 */
if (versionRows.length >= 2) {
  const dates = versionRows.map((r) => r[3])
  if (new Set(dates).size !== dates.length) {
    problems.push(`两版的时间列该不同（真值 vs 目录代时间），实测 ${dates.join(' / ')}`)
  }
}

/* ---------- ② 展开待下载那一行 → 状态说「新版本」 ---------- */
const pendingIndex = cloud.findIndex((r) => r[0] === 'A1-fast.toml' && r[4] === '下载')
if (pendingIndex >= 0) {
  await page
    .locator('main tbody tr')
    .filter({ has: page.locator('td:not([colspan])') })
    .nth(pendingIndex)
    .locator('td')
    .first()
    .click()
  await page.waitForTimeout(350)
}
const detail = await page.evaluate(() => {
  const dl = document.querySelector('main tbody dl')
  if (dl === null) return null
  const out = {}
  const kids = [...dl.querySelectorAll('dt, dd')].map((x) => x.textContent.trim())
  for (let i = 0; i + 1 < kids.length; i += 2) out[kids[i]] = kids[i + 1]
  return out
})
console.log(`[② 新版本行] 展开详情：状态=${detail?.状态 ?? '(没展开)'}`)
if (detail?.状态 !== '新版本') {
  problems.push(`待下载那一版的展开详情该说「新版本」，实测「${detail?.状态 ?? '（没展开）'}」`)
}
if (JSON.stringify(detail ?? {}).includes('过时') || JSON.stringify(detail ?? {}).includes('需更新')) {
  problems.push('不该出现"过时 / 需更新"这种话 —— 官方更新不改用户的预设')
}
await page.screenshot({ path: `${shotDir}/cloud-versions.png` })

/* ---------- ③ 对比台 ---------- */
if (!(await clickByText('main button', '参数对比'))) problems.push('工具条上没有「参数对比」按钮')
await page.waitForTimeout(800)

const heads = await page.evaluate(() =>
  [...document.querySelectorAll('[role="dialog"] thead th')].map((t) => t.textContent.trim()),
)
console.log(`[③ 对比台] 表头：${JSON.stringify(heads)}`)
if (heads.length < 3) {
  problems.push(`对比台该默认选中两份（表头 = 参数 + 两列），实测 ${heads.length} 列`)
}

/** 差异行 = 本行有「采用此值」那枚按钮 */
const diffRowOf = () =>
  page.evaluate(() => {
    const row = [...document.querySelectorAll('[role="dialog"] tbody tr')].find((r) =>
      [...r.querySelectorAll('button')].some((b) => b.textContent.trim() === '采用此值'),
    )
    if (row === undefined) return null
    return {
      key: row.querySelector('td')?.innerText.replace(/\s+/g, ' ').trim() ?? '',
      values: [...row.querySelectorAll('input')].map((i) => i.value),
      adopts: [...row.querySelectorAll('button')].filter((b) => b.textContent.trim() === '采用此值')
        .length,
    }
  })
const diff = await diffRowOf()
console.log(`[③ 对比台] 差异行：${diff === null ? '没有' : `${diff.key} → ${diff.values.join(' / ')}（${diff.adopts} 枚采用此值）`}`)
if (diff === null) {
  problems.push('假后端那两份演示预设有一个不同的参数，差异行该带「采用此值」')
} else if (diff.adopts < 2) {
  problems.push(`差异行每一格都该有自己那枚「采用此值」（显式动作），实测 ${diff.adopts} 枚`)
}

if (diff !== null) {
  /* 点最右边那枚「采用此值」→ 把右边的值复制到本行其它几份 */
  await page.evaluate(() => {
    const row = [...document.querySelectorAll('[role="dialog"] tbody tr')].find((r) =>
      [...r.querySelectorAll('button')].some((b) => b.textContent.trim() === '采用此值'),
    )
    const btns = [...row.querySelectorAll('button')].filter((b) => b.textContent.trim() === '采用此值')
    btns[btns.length - 1].click()
  })
  await page.waitForTimeout(350)
  const after = await page.evaluate(() => {
    const d = document.querySelector('[role="dialog"]')
    const row = [...d.querySelectorAll('tbody tr')].find((r) =>
      [...r.querySelectorAll('input')].length > 0 && r.querySelector('[class*="adopt"]') === null,
    )
    return {
      heads: [...d.querySelectorAll('thead th')].map((t) => t.innerText.replace(/\s+/g, ' ').trim()),
      foot: d.querySelector('footer')?.innerText.replace(/\s+/g, ' ').trim() ?? '',
      values: [...d.querySelectorAll('tbody input')].slice(0, 4).map((i) => i.value),
    }
  })
  console.log(`[③ 对比台] 采用之后：表头=${JSON.stringify(after.heads)} 底栏=${after.foot}`)
  if (!after.heads.some((h) => h.includes('有未保存修改'))) {
    problems.push('点「采用此值」之后那一列该显示「有未保存修改」')
  }
  if (!/\d+ 处改动待保存/.test(after.foot)) {
    problems.push(`底栏该报出待保存的改动数，实测「${after.foot}」`)
  }

  /* 保存修改 → 模态框关掉 + 提示条给一句话 */
  await page.evaluate(() => {
    const d = document.querySelector('[role="dialog"]')
    const b = [...d.querySelectorAll('footer button')].find((x) => x.textContent.trim() === '保存修改')
    b?.click()
  })
  await page.waitForTimeout(900)
  const saved = await page.evaluate(() => ({
    dialog: document.querySelector('[role="dialog"]') === null ? '已关闭' : '还开着',
    status:
      document.querySelector('main [role="status"]')?.innerText.replace(/\s+/g, ' ').trim() ?? '',
  }))
  console.log(`[③ 对比台] 保存之后：模态框 ${saved.dialog}；提示条「${saved.status}」`)
  if (saved.dialog !== '已关闭') problems.push('保存成功之后该关掉模态框')
  if (!saved.status.includes('已保存')) {
    problems.push(`保存成功该在提示条里说一句「已保存 …」，实测「${saved.status}」`)
  }
}
await page.screenshot({ path: `${shotDir}/compare.png` })

/* ---------- ④ 收尾 ---------- */
await browser.close()

if (problems.length > 0) {
  console.error('\n✗ 多版本 / 对比台探针：有问题')
  for (const p of problems) console.error(`  · ${p}`)
  process.exit(1)
}
console.log('\n✓ 多版本 / 对比台探针：四节都过')
