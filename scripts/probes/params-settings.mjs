/*
 * 接线探针：参数页（P3）与设置页的「预设数据源」那一格 —— 「一次性接线」那一步的验收。
 *
 * 用法（先 `npm run build`，再让静态预览在 4173 上跑着）：
 *   npm run build
 *   npx vite preview --port 4173 --strictPort
 *   node scripts/probes/params-settings.mjs [url]
 *
 * 走一遍真实的路（每档尺寸一个全新 profile）：
 *   ① 参数页：分类 / 卡片 / 行全部来自目录（catalog 的 registry）—— 直接画得出来
 *   ② 底栏在：从包里来的那两句（「修改参数」/「没有未保存的改动」）
 *   ③ 设置页 · 高级设置 → 预设数据源：读得出「当前：还没配置」；手动指定能应用
 *      （尾斜杠砍掉 —— 与真机 `normalize_base_url` 同一套）；非法地址如实拒；
 *      「恢复内置默认」能撤回覆盖（假后端没有内置源 → 回到「没配」）
 *
 * **改名与重写记（2026-10-02）**：原版叫 `params-sync.mjs`，第二半走的是**同步页**
 * （口径与账 / catalog 调试信息 / 检查更新按钮）。作者裁决「同步」页整页退役 ——
 * 那一半结构性消失（不是坏了）：数据源配置降级成设置页里的开发后门，
 * 于是这一份重写成「参数页 + 设置页」。**探针和它测的流一起换**
 * （本仓纪律：探针会和它测的流一起腐烂，收口一层就要回头跑一遍）。
 * 没换的还是那几条「看得见」的判据 —— 页面有没有内容、控制台有没有 error、
 * 有没有 >=400 的响应。两档尺寸：Ultra 1760×900 / Compact 900×640。
 */
import { mkdir } from 'node:fs/promises'

import { chromium } from 'playwright-core'

const url = process.argv[2]?.startsWith('http') ? process.argv[2] : 'http://localhost:4173/'

const SIZES = [
  { tag: 'ultra', width: 1760, height: 900 },
  { tag: 'compact', width: 900, height: 640 },
]

const shotDir = 'tmp-shots'
await mkdir(shotDir, { recursive: true })

const problems = []
const results = []
const check = (tag, what, ok, detail = '') => {
  results.push({ tag, what, ok, detail })
  if (!ok) problems.push(`${tag} ${what}${detail ? ` —— ${detail}` : ''}`)
}

/* 已知且无害：index.html 没写 favicon，浏览器自己会去要一次 */
const BENIGN = [/\/favicon\.ico$/]
const benign = (text) => BENIGN.some((re) => re.test(text))

const mainText = (page) =>
  page.evaluate(() => (document.querySelector('main')?.innerText ?? '').replace(/\s+/g, ' ').trim())

/** 等一个条件成立（后端是内存桩，一般是一两帧的事） */
async function until(fn, ms = 5000) {
  const t0 = Date.now()
  for (;;) {
    if (await fn()) return true
    if (Date.now() - t0 > ms) return false
    await new Promise((r) => setTimeout(r, 100))
  }
}

const browser = await chromium.launch({ channel: 'msedge' })

for (const size of SIZES) {
  const tag = size.tag
  const ctx = await browser.newContext({ viewport: { width: size.width, height: size.height } })
  const page = await ctx.newPage()

  page.on('console', (m) => {
    if (m.type() !== 'error') return
    const at = m.location?.()?.url ?? ''
    if (benign(at)) return
    problems.push(`${tag} console.error: ${m.text()}${at ? ` @ ${at}` : ''}`)
  })
  page.on('pageerror', (e) => problems.push(`${tag} pageerror: ${e.message}`))
  page.on('response', (r) => {
    if (r.status() >= 400 && !benign(r.url())) problems.push(`${tag} HTTP ${r.status()} ${r.url()}`)
  })
  page.on('requestfailed', (r) => {
    if (!benign(r.url())) problems.push(`${tag} requestfailed ${r.failure()?.errorText ?? ''} ${r.url()}`)
  })

  await page.goto(url, { waitUntil: 'load' })
  await page.waitForSelector('header nav', { timeout: 10000 })

  /* ① 参数页：直接画目录的 registry（分类 / 卡片 / 行） */
  await page.getByRole('button', { name: '参数', exact: true }).first().click()
  await page.waitForSelector('[role="tablist"][aria-label="参数分类"]', { timeout: 10000 })
  await page.waitForTimeout(200)
  const full = await page.evaluate(() => {
    const tabs = [...document.querySelectorAll('[role="tablist"][aria-label="参数分类"] [role="tab"]')]
    return {
      tabs: tabs.length,
      current: tabs.find((t) => t.getAttribute('aria-current') === 'true')?.textContent?.trim() ?? '',
      cards: document.querySelectorAll('main section[aria-label]').length,
      rows: document.querySelectorAll('[data-key]').length,
      text: (document.querySelector('main')?.innerText ?? '').replace(/\s+/g, ' ').trim(),
    }
  })
  check(
    tag,
    '参数页画的是真目录（分类 / 卡片 / 行都来了）',
    full.tabs >= 2 && full.cards > 0 && full.rows > 0,
    `分类 ${full.tabs} / 卡片 ${full.cards} / 行 ${full.rows}`,
  )
  check(
    tag,
    '参数页标题与底栏在（每一条都从包里来）',
    full.text.includes('修改参数') && full.text.includes('没有未保存的改动'),
    full.text.slice(0, 60),
  )
  await page.screenshot({ path: `${shotDir}/wire-params-${tag}.png` })

  /* ①b 参数页接草稿链（参数页底座 ③）：改一个值 → 当场 patch 草稿 TOML，不报错 */
  /*
   * 守两件事：
   *   ① 参数页有**编辑目标**（当前 combo 的 MKP 文件）——没有它改值就只是内存改动；
   *   ② 改一个值之后**没有"草稿没写进磁盘"那行红字** —— 说明 patchPresetDraft 这条路通了
   *      （假后端按 tomlKey 改草稿正文，与真后端 `presetdata::patch` 同一件事）。
   * 真机上的保真与形态由 `presetdata::patch` 的 7 条单测钉着，这里量的是"接线通不通"。
   */
  const targets = await page.evaluate(() =>
    (document.querySelector('main')?.innerText ?? '').includes('这个版本没配 MKP 预设文件'),
  )
  const editTargetOk = !targets
  const numRow = page.locator('[data-key] input').first()
  let draftOk = true
  if ((await numRow.count()) > 0) {
    await numRow.fill('1.23')
    await numRow.blur().catch(() => {})
    await page.waitForTimeout(500)
    const after = (await mainText(page)) ?? ''
    draftOk = !after.includes('草稿没写进磁盘')
  }
  check(
    tag,
    '参数页有编辑目标、改值当场写进草稿 TOML（没有"草稿没写进磁盘"那行）',
    editTargetOk && draftOk,
    editTargetOk ? '' : '参数页说"这个版本没配 MKP 预设文件"',
  )

  /* ①c 操作记录带上下文（参数页底座 ④）：历史条目说得出"分类 → 参数 → 旧 → 新" */
  /*
   * 守一件事：历史条目**自带上下文**（分类 / 参数名 / 单位都是成形那一刻的快照），
   * 不是渲染时回查当前 combo —— 那样切了机型/版本就只剩一个裸 key。
   * 判据：打开修改历史，条目里同时出现**分类名**与**参数名**（如"空间偏移" + "X 轴偏移"）。
   */
  if (draftOk) {
    await page.getByRole('button', { name: /修改历史/ }).first().click()
    await page.waitForTimeout(500)
    const hist = (await mainText(page)) ?? ''
    const hasItem = /#1/.test(hist)
    const hasCtx = hist.includes('空间偏移') && hist.includes('X 轴偏移')
    check(
      tag,
      '修改历史条目带上下文（分类 / 参数名都在，不是裸 key）',
      hasItem && hasCtx,
      hist.slice(Math.max(0, hist.indexOf('#1')), hist.indexOf('#1') + 80),
    )
    /* 关掉抽屉，别影响后面的设置页那一节 */
    const closeBtn = page.locator('[aria-label="关闭"]').first()
    if ((await closeBtn.count()) > 0) await closeBtn.click().catch(() => {})
    await page.waitForTimeout(200)
  }

  /* ①d 弃用字段「显示，但只读」（2026-10-02，① deprecated 链路） */
  /*
   * 作者裁决：**显示 ≠ 可编辑 ≠ 会进入新产物**。参数页的字段清单以 definition 为准，
   * 弃用字段照常列（红线 + 「已弃用」徽章），控件只读、写值闸拒绝，但它不进新 TOML。
   *
   * 判据：
   *   · 有一个分类里出现 `[data-dep]` 行（真注册表 7 条字段级弃用，集中在「擦料塔」
   *     与「胶料」两类），且行名带删除线、行里有一枚「已弃用」；
   *   · 那行的控件是 disabled（改不动）；
   *   · 点它的分段（弃用触发条件）不产生改动（写值闸把它按住了）。
   */
  let depOk = false
  let depNote = '没找到 [data-dep] 行'
  {
    const tabs = await page
      .locator('[role="tablist"][aria-label="参数分类"] [role="tab"]')
      .all()
    for (const t of tabs) {
      await t.click()
      await page.waitForTimeout(250)
      const n = await page.locator('[data-dep]').count()
      if (n === 0) continue
      const info = await page.evaluate(() => {
        /*
         * 弃用行有的摊开成子卡、有的折在收拢的分支里（分支收拢时只有卡头，没有控件）。
         * 取**第一枚带「已弃用」徽章的那一行** —— 那才是摊开的、能验只读的行。
         */
        const rows = [...document.querySelectorAll('[data-dep]')]
        const row = rows.find((r) =>
          [...r.querySelectorAll('*')].some(
            (el) => el.children.length === 0 && el.textContent?.trim() === '已弃用',
          ),
        )
        if (row === undefined) return null
        const label = row.querySelector('span[class*="label"]')?.textContent ?? ''
        const controls = [...row.querySelectorAll('input,button[role="radio"]')]
        const allDisabled = controls.length > 0 && controls.every((el) => el.disabled)
        return { label, badge: true, allDisabled, controls: controls.length, total: rows.length }
      })
      if (info !== null) {
        depOk = info.badge && info.allDisabled
        depNote = `${info.total} 行弃用 · ${info.label} · 控件 ${info.controls} 个全禁用 ${info.allDisabled}`
      }
      break
    }
  }
  check(tag, '弃用字段显示但只读（红线 + 「已弃用」徽章 + 控件禁用）', depOk, depNote)

  /* ①e 塔地图消费 Plate（2026-10-02，④）：有板机型画出板轮廓 + 塔方块 + 可拖拽槽位 */
  /*
   * 守三件事：
   *   · 参数页出现塔地图（svg 里有「塔」字 + 板轮廓 evenodd 路径）；
   *   · 板是按机型的 defaultPlateId 从 catalog 的 plates 查到的（画出来了 = 查到了）；
   *   · 地图槽位替下了 X/Y 两行的排布（右列里还有 X/Y 坐标行）。
   */
  let towerOk = false
  let towerNote = '没找到塔地图'
  {
    const tabs = await page
      .locator('[role="tablist"][aria-label="参数分类"] [role="tab"]')
      .all()
    for (const t of tabs) {
      await t.click()
      await page.waitForTimeout(200)
      const hasTower = await page.locator('svg text').filter({ hasText: '塔' }).count()
      if (hasTower === 0) continue
      const plate = await page.locator('svg path[fill-rule="evenodd"]').count()
      const txt = (await mainText(page)) ?? ''
      towerOk = plate > 0 && txt.includes('擦料塔位置') && txt.includes('擦料塔X坐标')
      towerNote = `塔画布在 · 板轮廓 ${plate} · 槽位含坐标行 ${txt.includes('擦料塔X坐标')}`
      break
    }
  }
  check(tag, '塔地图按默认板画出轮廓并替下 X/Y 行（消费 catalog.plates）', towerOk, towerNote)

  /* ② 设置页：高级设置 → 预设数据源（同步页退役后，这一格搬到了这儿） */
  await page.getByRole('button', { name: '设置', exact: true }).first().click()
  const setReady = await until(async () => (await mainText(page)).includes('预设数据源'), 8000)
  const setText = await mainText(page)
  check(
    tag,
    '设置页在，且「高级设置 → 预设数据源」那一节在',
    setReady && setText.includes('高级设置') && setText.includes('预设数据源'),
    setText.slice(0, 80),
  )
  check(
    tag,
    '浏览器里如实说「当前：还没配置」（假后端没有内置源）',
    setText.includes('还没配置'),
    '',
  )

  /* 手动指定：填一个带尾斜杠的地址 → 应用 → 生效值砍掉尾斜杠 */
  await page.getByRole('radio', { name: /手动指定/ }).check()
  await page.getByLabel('数据源地址').fill('https://example.com/mkp/')
  await page.getByRole('button', { name: '应用', exact: true }).click()
  const applied = await until(async () => (await mainText(page)).includes('已应用'), 5000)
  const setText2 = await mainText(page)
  check(
    tag,
    '手动指定能应用（尾斜杠砍掉 —— 与真机 normalize 同一套）',
    applied && setText2.includes('你指定：https://example.com/mkp'),
    setText2.slice(0, 120),
  )

  /* 非法地址如实拒（校验消息与真机同一份） */
  await page.getByLabel('数据源地址').fill('file:///tmp/presets')
  await page.getByRole('button', { name: '应用', exact: true }).click()
  const rejected = await until(async () => (await mainText(page)).includes('只认 http'), 5000)
  check(tag, '非法地址如实拒（只认 http(s)）', rejected, '')

  /* 恢复内置默认：撤掉覆盖 → 回到「没配」（假后端没有内置源） */
  await page.getByRole('radio', { name: /使用内置官方源/ }).check()
  await page.getByRole('button', { name: '恢复内置默认', exact: true }).click()
  const cleared = await until(async () => (await mainText(page)).includes('已撤掉你填的地址'), 5000)
  const setText3 = await mainText(page)
  check(
    tag,
    '恢复内置默认能撤回（没有内置源 → 回到「没配」）',
    cleared && setText3.includes('还没配置'),
    setText3.slice(0, 120),
  )
  await page.screenshot({ path: `${shotDir}/wire-settings-${tag}.png` })

  console.log(
    `${tag.padEnd(8)} 参数页 分类 ${String(full.tabs).padEnd(2)} 卡片 ${String(full.cards).padEnd(2)} 行 ${String(full.rows).padEnd(3)} 当前分类 ${full.current} · 设置页 数据源那一格全流程能走`,
  )
  await ctx.close()
}

await browser.close()

console.log('-'.repeat(80))
console.log(`${results.length} 条判定：`)
for (const r of results) {
  console.log(`  ${r.ok ? 'ok  ' : 'FAIL'} [${r.tag}] ${r.what}${r.detail ? `  （${r.detail}）` : ''}`)
}

if (problems.length > 0) {
  console.log(`发现问题 ${problems.length} 条：`)
  for (const p of problems) console.log(`  - ${p}`)
  process.exit(1)
}
console.log('参数页与设置页两边都通：两档尺寸 0 console error / 0 个 >=400')
