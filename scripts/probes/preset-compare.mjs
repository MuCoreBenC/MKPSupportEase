/*
 * **参数对比台**探针（2026-10-08 起；2026-10-16 按新入口与新两态重写）。
 *
 * 用法（与 `presets.mjs` 同一条路：build 之后走静态预览）：
 *   npm run build
 *   npx vite preview --port 4173 --strictPort
 *   node scripts/probes/preset-compare.mjs [url]
 *
 * # 这一版测什么（2026-10-16 重写）
 *
 * 老版四节里有两节测的是**已经退役**的东西，按纪律裁掉（退役结构要同时裁掉读它的探针）：
 *   · 「云端表一个预设一组、版本各一行」—— 2026-10-08 资源库改判后云端**只有一个当前版**
 *     （一个预设一行；这条由 `presets.mjs` 的 5e 节守着），版本行那两节没有对应物了；
 *   · 「工具条上的『参数对比』按钮」—— 2026-10-10 退役（作者：「参数对比几个字，
 *     我们去掉了……答案是右键选择一个，然后再选择另外一个，就去对比」）。
 *
 * 现在守的是对比台本身（入口 · 两态 · 采用 · 保存）：
 *   ① 右键两下开台：第一下「参数对比」记下那份、第二下「与「X」对比」开台；
 *   ② 打开先只读（2026-10-16）：没有 input、「采用此值」不出现，底栏只有「进入编辑模式」；
 *   ③ 点「进入编辑模式」→ 控件与「采用此值」出场，底栏 = 撤销/重做/放弃并退出/保存；
 *   ④ 点「采用此值」→ 底栏报「未保存的改动」、两格值趋同 → 「保存」把模态框关掉并提示；
 *   ⑤ 整页跑完没有多出来的 console error / pageerror / >=400 响应。
 *
 * 真机那侧更硬的判据在 Rust：`presetdata::params`（保真抽取与批量写回）。
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

/** 点一个「文字正好是这些字」的按钮 */
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

/** 分段控件本质是原生 radio：按 `name` + `value` 直接点（label 文字是双层的，别按文本找） */
const rad = (name, value) => page.locator(`main input[type="radio"][name="${name}"][value="${value}"]`)

/** 表里某一行的定位器（展开排那一格是 `td[colspan]`，不算行） */
const rowOf = (file) =>
  page
    .locator('main tbody tr[data-rowkey]')
    .filter({ has: page.locator('td:not([colspan])') })
    .filter({ hasText: file })
    .first()

/** 弹窗底栏：按钮名集合（图标按钮的字在 aria-label / title 里）+ 计数 */
const footState = () =>
  page.evaluate(() => {
    const d = document.querySelector('[role="dialog"]')
    const foot = d?.querySelector('footer')
    return {
      names: [...(foot?.querySelectorAll('button') ?? [])].map((b) =>
        (b.innerText || b.getAttribute('aria-label') || b.getAttribute('title') || '').trim(),
      ),
      foot: foot?.innerText.replace(/\s+/g, ' ').trim() ?? '',
      /* 行内的"可改控件"：**限定在值格（`.cell`）里** —— 搜索框不算（只读态它也在） */
      controls:
        d?.querySelectorAll('[class*="cell"] input, [class*="cell"] button[role="radio"]').length ??
        -1,
    }
  })

if (!(await clickByText('header button', '预设'))) problems.push('找不到「预设」页签')
await page.waitForTimeout(900)

/* ---------- ① 右键两下开台（2026-10-10 定的入口） ---------- */
/*
 * 第一下：右键 A1 涂胶.toml → 「参数对比」（记下它，页脚说一句）；
 * 第二下：右键 A1-fast.toml → 「与「A1 涂胶.toml」对比」→ 开台，两份直接坐进列里。
 */
await rad('preset-scope', 'local').click({ force: true })
await page.waitForTimeout(400)

const firstRow = rowOf('A1 涂胶.toml')
if ((await firstRow.count()) === 0) {
  problems.push('本地面没有 A1 涂胶.toml —— 对比台开不起来（假后端样本变了？）')
} else {
  await firstRow.click({ button: 'right' })
  await page.waitForTimeout(300)
  const pickItem = page.getByRole('menuitem', { name: '参数对比', exact: true })
  if ((await pickItem.count()) === 0) {
    problems.push('本地 MKP 行的右键菜单里没有「参数对比」')
  }
  await pickItem.click()
  await page.waitForTimeout(300)

  const secondRow = rowOf('A1-fast.toml')
  await secondRow.click({ button: 'right' })
  await page.waitForTimeout(300)
  const goItem = page.getByRole('menuitem', { name: '与「A1 涂胶.toml」对比', exact: true })
  if ((await goItem.count()) === 0) {
    problems.push('第二下右键没有「与「A1 涂胶.toml」对比」—— 右键两下的第二步断了')
  }
  await goItem.click()
  await page.waitForTimeout(900)
}

/* 列头是 div 网格（`.thead`/`.hcol`），不是 `<table>` —— 按 class 片段数 */
const heads = await page.evaluate(() => {
  const d = document.querySelector('[role="dialog"]')
  return {
    key: d?.querySelector('[class*="hkey"]') !== null,
    cols: d?.querySelectorAll('[class*="hcol"]').length ?? 0,
  }
})
console.log(`[① 对比台] 列头：参数列=${heads.key} 对比列=${heads.cols}`)
if (!heads.key || heads.cols < 2) {
  problems.push(`对比台该坐着两份（列头 = 参数 + 两列），实测 ${heads.cols} 列`)
}

/* ---------- ② 打开先只读（2026-10-16 两态） ---------- */
const roState = await footState()
const roAdopts = await page.evaluate(
  () =>
    [...document.querySelectorAll('[role="dialog"] button')].filter(
      (b) => b.textContent.trim() === '采用此值',
    ).length,
)
console.log(
  `[② 对比台] 只读态：控件=${roState.controls} 采用此值=${roAdopts} 底栏=「${roState.foot}」`,
)
if (roState.controls !== 0) problems.push('对比台打开该是只读的：不该有 input / 开关控件')
if (roAdopts !== 0) problems.push('对比台打开该是只读的：不该有「采用此值」')
if (!roState.names.some((n) => n === '进入编辑模式')) {
  problems.push(`只读态底栏该有「进入编辑模式」，实测 ${JSON.stringify(roState.names)}`)
}
await page.screenshot({ path: `${shotDir}/compare-readonly.png` })

/* ---------- ③ 进编辑模式：控件 / 「采用此值」/ 底栏四件套出场 ---------- */
await page.evaluate(() => {
  const d = document.querySelector('[role="dialog"]')
  const b = [...d.querySelectorAll('footer button')].find((x) => x.textContent.trim() === '进入编辑模式')
  b?.click()
})
await page.waitForTimeout(500)
const edState = await footState()
console.log(`[③ 对比台] 编辑态：控件=${edState.controls} 底栏=${JSON.stringify(edState.names)}`)
if (edState.controls < 1) problems.push('进了编辑模式该有可改的控件')
/* 撤销 / 重做是图标按钮 —— 认名字要连 aria-label 一起看（footState 已经这么取） */
for (const need of ['撤销', '重做', '放弃并退出', '保存']) {
  if (!edState.names.includes(need)) problems.push(`编辑态底栏少「${need}」`)
}

/* ---------- ③b 「放弃并退出」= 作废（2026-10-16）；再进来接着走 ---------- */
/* 先造一处脏（点第一枚「采用此值」），把"作废"能验到实处 */
await page.evaluate(() => {
  const btn = [...document.querySelectorAll('[role="dialog"] button')].find(
    (b) => b.textContent.trim() === '采用此值',
  )
  btn?.click()
})
await page.waitForTimeout(350)
const dirtyFoot = (await footState()).foot
console.log(`[③b 对比台] 采用一处后：${dirtyFoot}`)
if (!/未保存的改动/.test(dirtyFoot)) {
  problems.push(`点「采用此值」该记一处未保存的改动，实测「${dirtyFoot}」`)
}

/* 「放弃并退出」（= 作废：未保存的改动全丢）→ 回只读 */
await page.evaluate(() => {
  const d = document.querySelector('[role="dialog"]')
  const b = [...d.querySelectorAll('footer button')].find((x) => x.textContent.trim() === '放弃并退出')
  b?.click()
})
await page.waitForTimeout(400)
const backRo = await footState()
const backAdopts = await page.evaluate(
  () =>
    [...document.querySelectorAll('[role="dialog"] button')].filter(
      (b) => b.textContent.trim() === '采用此值',
    ).length,
)
console.log(`[③b 对比台] 「放弃并退出」后：控件=${backRo.controls} 采用此值=${backAdopts}`)
if (backRo.controls !== 0 || backAdopts !== 0) {
  problems.push('「放弃并退出」该回只读（控件与「采用此值」都收回）')
}
if (!backRo.names.some((n) => n === '进入编辑模式')) {
  problems.push('「放弃并退出」后底栏该回到「进入编辑模式」')
}
/* 再进来：底栏该是干净的（作废生效）—— ④ 接着用它 */
await page.evaluate(() => {
  const d = document.querySelector('[role="dialog"]')
  const b = [...d.querySelectorAll('footer button')].find((x) => x.textContent.trim() === '进入编辑模式')
  b?.click()
})
await page.waitForTimeout(400)
const afterExit = await footState()
console.log(`[③b 对比台] 作废后再进来：${afterExit.foot}`)
if (!afterExit.foot.includes('没有未保存的改动')) {
  problems.push('「放弃并退出」= 作废：再进来该是「没有未保存的改动」')
}

/* ---------- ④ 差异行 →「采用此值」→ 保存 ---------- */

/** 差异行 = 本行有「采用此值」那枚按钮（行是 div 网格：`[class*="grow"]`，不是 `<tr>`） */
const diffRowOf = () =>
  page.evaluate(() => {
    const row = [
      ...document.querySelectorAll('[role="dialog"] div[class*="grow"]:not([class*="groupRow"])'),
    ].find((r) => [...r.querySelectorAll('button')].some((b) => b.textContent.trim() === '采用此值'))
    if (row === undefined) return null
    return {
      key: row.querySelector('[class*="keyCell"]')?.innerText.replace(/\s+/g, ' ').trim() ?? '',
      values: [...row.querySelectorAll('input')].map((i) => i.value),
      adopts: [...row.querySelectorAll('button')].filter((b) => b.textContent.trim() === '采用此值')
        .length,
    }
  })
const diff = await diffRowOf()
console.log(
  `[④ 对比台] 差异行：${diff === null ? '没有' : `${diff.key} → ${diff.values.join(' / ')}（${diff.adopts} 枚采用此值）`}`,
)
if (diff === null) {
  problems.push('假后端那两份演示预设有一个不同的参数，差异行该带「采用此值」')
} else if (diff.adopts < 2) {
  problems.push(`差异行每一格都该有自己那枚「采用此值」（显式动作），实测 ${diff.adopts} 枚`)
}

if (diff !== null) {
  /* 点最右边那枚「采用此值」→ 把右边的值复制到本行其它几份 */
  await page.evaluate(() => {
    const row = [
      ...document.querySelectorAll('[role="dialog"] div[class*="grow"]:not([class*="groupRow"])'),
    ].find((r) => [...r.querySelectorAll('button')].some((b) => b.textContent.trim() === '采用此值'))
    const btns = [...row.querySelectorAll('button')].filter((b) => b.textContent.trim() === '采用此值')
    btns[btns.length - 1].click()
  })
  await page.waitForTimeout(350)
  const after = await page.evaluate(() => {
    const d = document.querySelector('[role="dialog"]')
    return {
      foot: d?.querySelector('footer')?.innerText.replace(/\s+/g, ' ').trim() ?? '',
    }
  })
  console.log(`[④ 对比台] 采用之后：底栏=${after.foot}`)
  /* 底栏计数（2026-10-16 文案：「未保存的改动（N）」） */
  if (!/未保存的改动/.test(after.foot)) {
    problems.push(`底栏该报出未保存的改动，实测「${after.foot}」`)
  }

  /* 保存 → 模态框关掉 + 提示条给一句话 */
  await page.evaluate(() => {
    const d = document.querySelector('[role="dialog"]')
    const b = [...d.querySelectorAll('footer button')].find((x) => x.textContent.trim() === '保存')
    b?.click()
  })
  await page.waitForTimeout(900)
  const saved = await page.evaluate(() => ({
    dialog: document.querySelector('[role="dialog"]') === null ? '已关闭' : '还开着',
    status:
      document.querySelector('main [role="status"]')?.innerText.replace(/\s+/g, ' ').trim() ?? '',
  }))
  console.log(`[④ 对比台] 保存之后：模态框 ${saved.dialog}；提示条「${saved.status}」`)
  if (saved.dialog !== '已关闭') problems.push('保存成功之后该关掉模态框')
  if (!saved.status.includes('已保存')) {
    problems.push(`保存成功该在提示条里说一句「已保存 …」，实测「${saved.status}」`)
  }
}
await page.screenshot({ path: `${shotDir}/compare.png` })

/* ---------- ⑤ 收尾 ---------- */
await browser.close()

if (problems.length > 0) {
  console.error('\n✗ 参数对比台探针：有问题')
  for (const p of problems) console.error(`  · ${p}`)
  process.exit(1)
}
console.log('\n✓ 参数对比台探针：入口 / 两态 / 采用 / 保存 都过')
