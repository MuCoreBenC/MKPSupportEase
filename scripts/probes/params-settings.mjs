/*
 * 接线探针：参数页（P3）与设置页的「预设数据源」那一格 —— 「一次性接线」那一步的验收。
 *
 * 用法（先 `npm run build`，再让静态预览在 4173 上跑着）：
 *   npm run build
 *   npx vite preview --port 4173 --strictPort
 *   node scripts/probes/params-settings.mjs [url]
 *
 * 走一遍真实的路（每档尺寸一个全新 profile）：
 *   ① 「修改参数」弹窗（预设页本地面那一行右键进入 —— ★ 参数**页签** 2026-10-09 已退役，
 *      旧版入口那几步早跑不到底，2026-10-16 按弹窗入口重写）：分类 / 卡片 / 行全部来自
 *      目录（catalog 的 registry）；**两态**（打开先只读 → 「进入编辑模式」→
 *      撤销/重做/放弃并退出/保存 → 「放弃并退出」回只读）；改一个值当场进草稿 TOML；
 *      修改历史条目带上下文；弃用字段只读；塔地图消费 catalog.plates
 *   ② 设置页 · 高级设置 → 预设数据源：读得出「当前：还没配置」（假后端没有内置源）；
 *      「自定义地址（开发 / 排查）」能应用（尾斜杠砍掉 —— 与真机 `normalize_base_url`
 *      同一套）；非法地址如实拒（主句说"取不到、没有改"）
 *
 * **改名与重写记（2026-10-02）**：原版叫 `params-sync.mjs`，第二半走的是**同步页**
 * （口径与账 / catalog 调试信息 / 检查更新按钮）。作者裁决「同步」页整页退役 ——
 * 那一半结构性消失（不是坏了）：数据源配置降级成设置页里的开发后门，
 * 于是这一份重写成「参数页 + 设置页」。**探针和它测的流一起换**
 * （本仓纪律：探针会和它测的流一起腐烂，收口一层就要回头跑一遍）。
 * 没换的还是那几条「看得见」的判据 —— 页面有没有内容、控制台有没有 error、
 * 有没有 >=400 的响应。两档尺寸：Ultra 1760×900 / Compact 900×640。
 *
 * **再改记（2026-10-16）**：参数**页签**退役之后，参数那一半自己也在腐烂
 * （旧版第一步还去点一颗不存在的「参数」按钮，从那以后整段跑不到底）——
 * 跟着新入口（预设行右键「修改参数」）与编辑模式两态一起翻修。设置页那一半没动。
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

/*
 * 已知且无害：
 *   · index.html 没写 favicon，浏览器自己会去要一次；
 *   · 假后端必抛的那几条「未实现的接口」—— 其中 `checkRemoteUpdate` 是**设计成要发生的**
 *     （进入预设页会后台检查一次远端目录；浏览器里没有远端，页面自己静默吞掉）。
 */
const BENIGN = [/\/favicon\.ico$/, /未实现的接口/]
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

/** 弹窗（「修改参数」）里的小工具：底栏按钮名 / 行内控件数 —— 全在 dialog 作用域里数 */
const dlgState = (page) =>
  page.evaluate(() => {
    const d = document.querySelector('[role="dialog"]')
    const foot = d?.querySelector('footer')
    return {
      foot: foot?.innerText.replace(/\s+/g, ' ').trim() ?? '',
      /* 撤销 / 重做是图标按钮 —— 名字连 aria-label / title 一起取 */
      names: [...(foot?.querySelectorAll('button') ?? [])].map((b) =>
        (b.innerText || b.getAttribute('aria-label') || b.getAttribute('title') || '').trim(),
      ),
      rowControls: d?.querySelectorAll('[data-key] input, [data-key] textarea').length ?? -1,
      text: (d?.innerText ?? '').replace(/\s+/g, ' ').trim(),
    }
  })

/** 点弹窗底栏一颗「文字正好是这些字」的按钮 */
const dlgFootBtn = (page, text) =>
  page.evaluate((want) => {
    const d = document.querySelector('[role="dialog"]')
    const b = [...(d?.querySelectorAll('footer button') ?? [])].find(
      (x) => x.textContent.trim() === want,
    )
    b?.click()
    return b !== undefined
  }, text)

/** 切分段控件的那一档（`本地 / 云端`、`MKP 配置 / 切片器配置`） */
const pickSeg = async (page, want) => {
  await page.evaluate((w) => {
    const l = [...document.querySelectorAll('main [role="radiogroup"] label')].find(
      (x) => (x.textContent ?? '').trim() === w,
    )
    l?.click()
  }, want)
  await page.waitForTimeout(400)
}

const browser = await chromium.launch({ channel: 'msedge' })

for (const size of SIZES) {
  const tag = size.tag
  const ctx = await browser.newContext({ viewport: { width: size.width, height: size.height } })
  const page = await ctx.newPage()

  page.on('console', (m) => {
    if (m.type() !== 'error') return
    const at = m.location?.()?.url ?? ''
    /* 白名单要**连正文一起看**：`未实现的接口: checkRemoteUpdate` 认的是消息本身
       （2026-10-16：原来只查 URL，这条设计成要发生的消息每档恒进 problems） */
    if (benign(at) || benign(m.text())) return
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

  /* ① 进「预设」页 → 右键本地那一份 MKP 预设 → 「修改参数」弹窗 → 两态 */
  /*
   * ★ 2026-10-16 重写入口：参数**页签** 2026-10-09 整页退场（`constants/tabs.ts`），
   *   旧版还去点「参数」页签 —— 自那以后这一段一直跑不到底（点不到那颗按钮）。
   *   现在的真入口是**预设页那一行右键「修改参数」**（同一个 `PageParams`，嵌在
   *   `PresetParamsModal` 里）。顺带把 2026-10-16 的**两态**钉在这里：
   *     打开先只读（行里没有可改控件、底栏只有「进入编辑模式」）→
   *     点它进编辑模式（撤销 / 重做 / 放弃并退出 / 保存出场）→
   *     点「放弃并退出」= 作废（回只读，没保存的改退回已保存值 —— 2026-10-16 作者口径；
   *     这颗按钮当天从「退出编辑模式」改的词，见 `EDIT_TEXT.exitMode`）。
   */
  await page.getByRole('button', { name: '预设', exact: true }).first().click()
  await page.waitForSelector('main tbody tr[data-rowkey]', { timeout: 10000 })
  await page.waitForTimeout(500)
  await pickSeg(page, '本地')
  const paramRow = page
    .locator('main tbody tr[data-rowkey]')
    .filter({ has: page.locator('td:not([colspan])') })
    .filter({ hasText: 'A1 涂胶.toml' })
    .first()
  if ((await paramRow.count()) === 0) {
    problems.push(`${tag} 预设页本地面没有 A1 涂胶.toml —— 「修改参数」弹窗进不去`)
  } else {
    await paramRow.click({ button: 'right' })
    await page.waitForTimeout(300)
    await page.getByRole('menuitem', { name: '修改参数', exact: true }).click()
  }
  await page.waitForSelector('[role="tablist"][aria-label="参数分类"]', { timeout: 10000 })
  await page.waitForTimeout(300)
  const full = await page.evaluate(() => {
    const d = document.querySelector('[role="dialog"]')
    const tabs = [...(d?.querySelectorAll('[role="tablist"][aria-label="参数分类"] [role="tab"]') ?? [])]
    return {
      tabs: tabs.length,
      current: tabs.find((t) => t.getAttribute('aria-current') === 'true')?.textContent?.trim() ?? '',
      cards: d?.querySelectorAll('section[aria-label]').length ?? 0,
      rows: d?.querySelectorAll('[data-key]').length ?? 0,
      text: (d?.innerText ?? '').replace(/\s+/g, ' ').trim(),
    }
  })
  check(
    tag,
    '「修改参数」弹窗画的是真目录（分类 / 卡片 / 行都来了）',
    full.tabs >= 2 && full.cards > 0 && full.rows > 0,
    `分类 ${full.tabs} / 卡片 ${full.cards} / 行 ${full.rows}`,
  )
  check(
    tag,
    '弹窗标题与底栏在（照 a05：大标题「修改参数」+ 副标题=文件名；底栏状态从包里来）',
    full.text.includes('修改参数') &&
      full.text.includes('A1 涂胶.toml') &&
      full.text.includes('没有未保存的改动'),
    full.text.slice(0, 60),
  )
  /*
   * 版式（照 a05）：**「修改历史」在模态框标题栏那一簇里**（`×` 左边，见 PageParams 的
   * `headActions`）；**搜索框在分类条那一行**（够宽时）—— 不再占头部一行。
   */
  const layout = await page.evaluate(() => {
    const d = document.querySelector('[role="dialog"]')
    const head = d?.querySelector('header')
    const tablist = d?.querySelector('[role="tablist"][aria-label="参数分类"]')
    /* 分类条那一行 = `.bar`：`.group[role=tablist]` → `.wrap` → `.bar`（搜索框与 `.wrap` 同级） */
    const bar = tablist?.parentElement?.parentElement ?? null
    const inHead = (text) =>
      [...(head?.querySelectorAll('button') ?? [])].some((b) => (b.textContent ?? '').includes(text))
    return {
      historyInHead: inHead('修改历史'),
      searchInBar: bar !== null && bar.querySelector('[aria-label="搜索参数"]') !== null,
    }
  })
  check(
    tag,
    '版式照 a05：「修改历史」在标题栏、「搜索框」在分类条那一行',
    layout.historyInHead && layout.searchInBar,
    `标题栏 ${layout.historyInHead} / 分类条行 ${layout.searchInBar}`,
  )

  /* ①a 打开先只读（2026-10-16 两态）：行里没有可改控件，底栏只有「进入编辑模式」 */
  const roState = await dlgState(page)
  check(
    tag,
    '弹窗打开先只读（行内无可改控件、底栏只有「进入编辑模式」）',
    roState.rowControls === 0 &&
      roState.names.includes('进入编辑模式') &&
      /* 判据看**按钮名**（"没有未保存的改动"这句状态文字里也含"保存"二字，别用脚本文本判） */
      !roState.names.includes('保存') &&
      !roState.names.includes('放弃并退出'),
    `行内控件 ${roState.rowControls} · 底栏 ${JSON.stringify(roState.names)}`,
  )
  await page.screenshot({ path: `${shotDir}/wire-params-${tag}.png` })

  /* ①a2 点「进入编辑模式」→ 控件出场、底栏 = 撤销 / 重做 / 放弃并退出 / 保存 */
  await dlgFootBtn(page, '进入编辑模式')
  await page.waitForTimeout(400)
  const edState = await dlgState(page)
  check(
    tag,
    '进编辑模式：行内控件出场、底栏 = 撤销/重做/放弃并退出/保存',
    edState.rowControls > 0 &&
      ['撤销', '重做', '放弃并退出', '保存'].every((n) => edState.names.includes(n)),
    `行内控件 ${edState.rowControls} · 底栏 ${JSON.stringify(edState.names)}`,
  )

  /* ①b 草稿链（参数页底座 ③）：改一个值 → 当场 patch 草稿 TOML，不报错 */
  /*
   * 守两件事：
   *   ① 弹窗有**编辑目标**（这一份 MKP 文件）——没有它改值就只是内存改动；
   *   ② 改一个值之后**没有"草稿没写进磁盘"那行红字** —— 说明 patchPresetDraft 这条路通了
   *      （假后端按 tomlKey 改草稿正文，与真后端 `presetdata::patch` 同一件事）。
   * 真机上的保真与形态由 `presetdata::patch` 的 7 条单测钉着，这里量的是"接线通不通"。
   */
  const targets = (await dlgState(page)).text.includes('这个版本没配 MKP 预设文件')
  const editTargetOk = !targets
  /*
   * ★ 2026-10-16：fill 的目标改挑 `toolhead.offset_y` —— 演示那份 `A1 涂胶.toml` 的字节里
   *   只有 `[toolhead] offset_y / speed_limit` 两个键；原样取"第一个 input"会落到
   *   `offset_x` 上，patch 找不到那一行、**如实报"草稿的 [toolhead] 里没有 offset_x"**
   *   （产品行为对，是探针挑错了目标）。
   */
  const keyShapes = await page.evaluate(() =>
    [...document.querySelectorAll('[role="dialog"] [data-key]')]
      .map((r) => r.getAttribute('data-key') ?? '')
      .filter((k) => k.includes('offset')),
  )
  console.log(`[${tag}] offset 那两行的 data-key：${JSON.stringify(keyShapes)}`)
  const numRow = page
    .locator('[role="dialog"] [data-key$="offset.y"], [role="dialog"] [data-key$="offset_y"]')
    .locator('input')
    .first()
  let draftOk = true
  let draftWhy = ''
  /*
   * 「已保存值」：**下面 ①c2 真的保存过两次之后它就变成 2.34**（22.4 是文件里原本的值）。
   * ①f「放弃并退出 = 作废」比的是"退回已保存值"，所以这个期望值跟着保存走。
   */
  let expectSaved = '22.4'
  if ((await numRow.count()) > 0) {
    await numRow.fill('1.23')
    await numRow.blur().catch(() => {})
    await page.waitForTimeout(500)
    const after = (await dlgState(page)).text
    draftOk = !after.includes('草稿没写进磁盘')
    if (!draftOk) draftWhy = after.slice(Math.max(0, after.indexOf('草稿没写进磁盘')), 160)
  }
  check(
    tag,
    '弹窗有编辑目标、改值当场写进草稿 TOML（没有"草稿没写进磁盘"那行）',
    editTargetOk && draftOk,
    draftOk ? (editTargetOk ? '' : '弹窗说"这个版本没配 MKP 预设文件"') : draftWhy,
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
    const hist = (await dlgState(page)).text
    const hasItem = /#1/.test(hist)
    /* 刚改的那一条 = 空间偏移 → Y 轴偏移（上面 fill 的是 toolhead.offset_y） */
    const hasCtx = hist.includes('空间偏移') && hist.includes('Y 轴偏移')
    check(
      tag,
      '修改历史条目带上下文（分类 / 参数名都在，不是裸 key）',
      hasItem && hasCtx,
      hist.includes('还没有改动')
        ? '抽屉说：这个文件本次打开还没有改动'
        : `#1=${hasItem} 上下文=${hasCtx} · ${hist.slice(hist.indexOf('修改历史'), hist.indexOf('修改历史') + 220)}`,
    )
    /*
     * 两档视图（2026-10-17 作者口径）：「逐项」（一项一行）与「整批」（一次保存一行、
     * 一次退整段）—— **两个都按保存组织，「实时」（变一下记一条）那一档没了**。
     */
    const views = await page.evaluate(() => {
      const d = document.querySelector('[role="dialog"][aria-label="修改历史"]')
      const btns = [...(d?.querySelectorAll('button') ?? [])].map((b) => (b.textContent ?? '').trim())
      return { hasItem: btns.includes('逐项'), hasBatch: btns.includes('整批'), live: btns.includes('实时') }
    })
    check(
      tag,
      '修改历史两档：逐项 / 整批在，实时不在',
      views.hasItem && views.hasBatch && !views.live,
      `逐项=${views.hasItem} 整批=${views.hasBatch} 实时=${views.live}`,
    )
    /*
     * 整批那一屏：一段一行、行上有「还原」。
     * ★ 两档是 `SegmentedField` —— 它的段是 `role="radio"`（不是普通 button），
     *   按 button 找会一直等到超时。
     */
    await page.getByRole('radio', { name: '整批', exact: true }).click()
    await page.waitForTimeout(400)
    const whole = (await dlgState(page)).text
    check(
      tag,
      '整批那一屏：一段一行（未保存段在、行上有还原）',
      /未保存/.test(whole) && /还原/.test(whole),
      whole.slice(whole.indexOf('修改历史'), whole.indexOf('修改历史') + 200),
    )
    /*
     * 整批行**可以展开看明细**（作者 2026-10-17：「整批的要可以展开看详细的」）——
     * 点那颗展开箭头，这一段里的每一项摊出来（每项自己也带「还原」）。
     */
    const caret = page
      .getByRole('dialog', { name: '修改历史' })
      .getByRole('button', { name: /展开 .*的明细/ })
      .first()
    const before = (await dlgState(page)).text
    await caret.click()
    await page.waitForTimeout(400)
    const after = (await dlgState(page)).text
    check(
      tag,
      '整批行可展开：展开后这一段里每一项都摊出来',
      after.length > before.length && /Y 轴偏移/.test(after),
      `展开前 ${before.length} 字 → 展开后 ${after.length} 字 · ${after.slice(after.indexOf('修改历史'), after.indexOf('修改历史') + 200)}`,
    )
    /*
     * **清空历史**（2026-10-17 作者要求：抽屉右上那颗 + 二次确认）。
     * 点第一下只是武装（变「确认清空？」），再点才真清 —— 清完抽屉回到"还没有改动记录"。
     */
    const dlg = page.getByRole('dialog', { name: '修改历史' })
    await dlg.getByRole('button', { name: '清空历史' }).click()
    await page.waitForTimeout(300)
    const armed = await dlg.getByRole('button', { name: '确认清空？' }).count()
    await dlg.getByRole('button', { name: '确认清空？' }).click()
    await page.waitForTimeout(400)
    const cleared = (await dlgState(page)).text
    check(
      tag,
      '清空历史：二次确认（先武装）之后真的清掉',
      armed === 1 && /还没有改动/.test(cleared),
      `武装=${armed} · ${cleared.slice(cleared.indexOf('修改历史'), cleared.indexOf('修改历史') + 160)}`,
    )
    /* 切回「逐项」—— 后面几步在默认那一屏上做 */
    await page.getByRole('radio', { name: '逐项', exact: true }).click()
    await page.waitForTimeout(200)

    /*
     * 收起抽屉：点**抽屉自己那颗 ×**。两条弯路别走：
     *   · 「修改历史」那颗开关在浮层态被抽屉自己盖住（它俩都在右上角，点击被抽屉拦截）；
     *   · Esc 不行 —— Modal 与 Drawer 都挂了 document 级 Esc，会连弹窗一起关掉。
     */
    await page
      .getByRole('dialog', { name: '修改历史' })
      .getByRole('button', { name: '关闭' })
      .click()
    await page.waitForTimeout(300)

    /*
     * ①c2 **保存 → 接着改 → 再保存**（2026-10-17 真机两条红字的回归判据）。
     *
     * 后端契约是「存完丢草稿」（`commit_preset_draft` 成功后 `clear_draft`），
     * 前端必须**重开草稿**（`beginPresetEdit`）—— 否则保存后改一个值就报
     * 「草稿没写进磁盘：现在没有正在改的那一份，改不了参数」，再点保存报「…没得存」。
     * 判据：保存之后再改一个值、再保存一次，**两条红字都不许出现**。
     */
    if (draftOk) {
      await dlgFootBtn(page, '保存')
      await page.waitForTimeout(400)
      await page.getByRole('button', { name: '确认保存' }).click()
      await page.waitForTimeout(600)
      /*
       * 保存成功后会问一次「要不要回切片器重切」（`ResliceNotice`，有遮罩）——
       * 它盖着页面，后面几步点不动 ⇒ 这里点「关闭」把它收掉（"这次不问了"）。
       */
      const reslice = page.getByRole('dialog', { name: '参数已保存' })
      if ((await reslice.count()) > 0) {
        /* 这一屏有两颗「关闭」（标题栏 × 与底栏那颗）—— 点底栏那颗 */
        await reslice.getByRole('button', { name: '关闭' }).last().click()
      }
      await page.waitForTimeout(300)
      /* 保存完接着改同一个值 —— 草稿链该还开着 */
      expectSaved = '2.34'
      await numRow.fill(expectSaved)
      await numRow.blur().catch(() => {})
      await page.waitForTimeout(600)
      const after1 = (await dlgState(page)).text
      await dlgFootBtn(page, '保存')
      await page.waitForTimeout(400)
      await page.getByRole('button', { name: '确认保存' }).click()
      await page.waitForTimeout(600)
      const after2 = (await dlgState(page)).text
      /* 第二次保存同样会问一次，照样收掉 */
      const reslice2 = page.getByRole('dialog', { name: '参数已保存' })
      if ((await reslice2.count()) > 0) {
        await reslice2.getByRole('button', { name: '关闭' }).last().click()
      }
      await page.waitForTimeout(300)
      check(
        tag,
        '保存 → 接着改 → 再保存：全程没有"现在没有正在改的那一份"',
        !after1.includes('现在没有正在改的那一份') &&
          !after2.includes('现在没有正在改的那一份') &&
          !after2.includes('保存失败'),
        `改值后=${after1.includes('现在没有正在改的那一份')} 二次保存后=${after2.includes('保存失败')} · ${after2.slice(Math.max(0, after2.indexOf('已写回')), 120)}`,
      )
    }
  }

  /* ①d 弃用字段「显示，但只读」（2026-10-02，① deprecated 链路） */
  /*
   * 作者裁决：**显示 ≠ 可编辑 ≠ 会进入新产物**。参数页的字段清单以 definition 为准，
   * 弃用字段照常列（红线 + 「已弃用」徽章），控件只读、写值闸拒绝，但它不进新 TOML。
   *
   * 判据：
   *   · 有一个分类里出现 `[data-dep]` 行（真注册表 6 条字段级弃用，集中在「擦料塔」
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
         * 作用域在 dialog 里（参数弹窗；2026-10-16 起这里也是编辑态）。
         */
        const rows = [...document.querySelectorAll('[role="dialog"] [data-dep]')]
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
      const hasTower = await page
        .locator('[role="dialog"] svg text')
        .filter({ hasText: '塔' })
        .count()
      if (hasTower === 0) continue
      const plate = await page.locator('[role="dialog"] svg path[fill-rule="evenodd"]').count()
      const txt = (await dlgState(page)).text
      towerOk = plate > 0 && txt.includes('擦料塔位置') && txt.includes('擦料塔X坐标')
      towerNote = `塔画布在 · 板轮廓 ${plate} · 槽位含坐标行 ${txt.includes('擦料塔X坐标')}`
      break
    }
  }
  check(tag, '塔地图按默认板画出轮廓并替下 X/Y 行（消费 catalog.plates）', towerOk, towerNote)

  /* ①f 「放弃并退出」（2026-10-16）：**= 作废** —— 回只读，且没保存的改动全部回滚 */
  await dlgFootBtn(page, '放弃并退出')
  await page.waitForTimeout(400)
  const backRo = await dlgState(page)
  check(
    tag,
    '「放弃并退出」回到只读（行内控件收回、底栏回到「进入编辑模式」）',
    backRo.rowControls === 0 && backRo.foot.includes('进入编辑模式'),
    `行内控件 ${backRo.rowControls} · 底栏「${backRo.foot}」`,
  )
  /*
   * 作废的硬判据（作者 2026-10-16：「没有保存的话，退出编辑模式就是作废，那些改的都作废，
   * 保存了才有」）：①b 填过 offset_y=1.23，退出后该退回已保存的 22.4；底栏也该干净。
   *
   * ★ offset_y 住偏移那一类 —— ①d 找弃用行把分类切走了（停在擦料那一类），要读那一行
   *   先切回来。**认 tab 要按 `title` 里的简写**：界面上分类名是短的（实测
   *   `偏移 · 5 项 / 擦料 · 27 项 / 风扇 · 3 项 / 涂胶 · 19 项 / 切换 · 2 项 / 更多 · 18 项`），
   *   胶囊里只有图标和数量，分类名根本不在 textContent 上（探针 2026-10-16 实测踩到）。
   */
  const tabHit = await page.evaluate(() => {
    const t = [
      ...document.querySelectorAll('[role="tablist"][aria-label="参数分类"] [role="tab"]'),
    ].find((x) => (x.getAttribute('title') ?? '').startsWith('偏移'))
    t?.click()
    return t !== undefined
  })
  console.log(`[${tag}] 切回「偏移」那一类=${tabHit}`)
  await page.waitForTimeout(350)
  await dlgFootBtn(page, '进入编辑模式')
  await page.waitForTimeout(400)
  const restored = await page
    .locator('[role="dialog"] [data-key$="offset.y"] input')
    .first()
    .inputValue()
  const restoredState = await dlgState(page)
  check(
    tag,
    `「放弃并退出」= 作废（值退回已保存的 ${expectSaved}、底栏没有未保存的改动）`,
    restored === expectSaved && /没有未保存的改动/.test(restoredState.foot),
    `offset_y=${restored}（期望 ${expectSaved}） · 底栏「${restoredState.foot}」`,
  )
  /* 收尾回只读，别把编辑态带去设置页那一节 */
  await dlgFootBtn(page, '放弃并退出')
  await page.waitForTimeout(300)

  /* 关掉弹窗（未保存的改动留在草稿里 —— 与「放弃并退出」同一条口径），别影响设置页那一节 */
  await page.keyboard.press('Escape')
  await page.waitForTimeout(400)
  /* 保险：真有没关掉的弹窗就再补一记（它的遮罩会挡住设置页里的点击） */
  if ((await page.getByRole('dialog').count()) > 0) {
    await page.keyboard.press('Escape')
    await page.waitForTimeout(300)
  }

  /* ② 设置页：高级设置 → 预设数据源（同步页退役后，这一格搬到了这儿） */
  /*
   * ★ 2026-10-16 按现状重写：原来那两档（「手动指定」/「使用内置云端源」+「恢复内置默认」）
   *   已经改成「两个固定内置单选（构建期注入）+ 收起的自定义地址」——
   *   假后端没有内置源（`getPresetSource()` 给 null），所以这里只有：
   *     当前「还没配置」→ 选「自定义地址（开发 / 排查）」→ 填地址 → 应用 →
   *     当前变「你指定：…」（尾斜杠砍掉，与真机 `normalize_base_url` 同一套）→
   *     非法地址如实拒（主句说"取不到、没有改"，技术详情收在折叠里）。
   */
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
  /*
   * **备份与恢复**（2026-10-17）：设置页那一块在（两颗按钮都在）。
   * 真的打包 / 恢复要本机数据根 —— 浏览器预览里假后端照实说做不到，
   * 所以这里只量"这一块摆出来了、入口在哪"，不假装打过一份备份。
   */
  check(
    tag,
    '设置页「备份与恢复」那一块在（打一份 / 从备份恢复两颗按钮都在）',
    setText.includes('备份与恢复') &&
      (await page.getByRole('button', { name: '打一份备份…' }).count()) === 1 &&
      (await page.getByRole('button', { name: '从备份恢复…' }).count()) === 1,
    setText.includes('备份与恢复') ? '' : `没看到那一块：${setText.slice(0, 120)}`,
  )

  /* 自定义地址：选上才展开输入框（防错第一道：普通用户没有输错地址的机会）。
     原生 radio 被自绘墨点盖着（`.mark` 拦 pointer）—— 直接 dispatch 点击，别用 `.check()` */
  await page.evaluate(() => {
    const input = [...document.querySelectorAll('input[name="preset-source"]')].find((x) =>
      (x.closest('label')?.textContent ?? '').includes('自定义地址'),
    )
    input?.click()
  })
  await page.waitForTimeout(300)
  await page.getByLabel('数据源地址').fill('https://example.com/mkp/')
  await page.getByRole('button', { name: '应用', exact: true }).click()
  const applied = await until(async () => (await mainText(page)).includes('已应用'), 5000)
  const setText2 = await mainText(page)
  check(
    tag,
    '自定义地址能应用（尾斜杠砍掉 —— 与真机 normalize 同一套）',
    applied && setText2.includes('你指定：https://example.com/mkp'),
    setText2.slice(0, 120),
  )

  /* 非法地址如实拒：主句说清"取不到、没有改"（校验消息在折叠的技术详情里） */
  await page.getByLabel('数据源地址').fill('file:///tmp/presets')
  await page.getByRole('button', { name: '应用', exact: true }).click()
  const rejected = await until(async () => (await mainText(page)).includes('这个地址取不到预设数据'), 5000)
  check(tag, '非法地址如实拒（主句说"取不到、没有改"）', rejected, '')
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
