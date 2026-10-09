/*
 * 首页 / 校准页探针 —— 客户端骨架（A40 的 home + calib）落地后的走查工具。
 *
 * 用法（先 build 再 preview，dev server 在本仓会被文件监听拖死）：
 *   npm run build && npm run preview        # 静态服务，4173
 *   node scripts/probes/home-flow.mjs http://localhost:4173/ --shots
 *
 * 与 tabs.mjs 的分工：那个只回答「8 个页签点得开吗、控制台干净吗」；
 * 这个回答「首页那台五步向导真的画出来了吗、校准页的板子与预设下拉在不在」。
 *
 * **刻意不做硬断言**：这两页的文案与 DOM（CSS Modules 的哈希类名）会随后续阶段变，
 * 写死选择器只会变成"每次改版都要修探针"。这里只把**看得见的事实**打出来
 * （正文、控件个数、SVG 个数、有没有数字），由人对着 A40 的 README 逐条判。
 * 唯一会 FAIL 的是：点不动、页面空、控制台有 error —— 这三种一定是坏了。
 */
import { mkdir } from 'node:fs/promises'

import { chromium } from 'playwright-core'

const url = process.argv[2]?.startsWith('http') ? process.argv[2] : 'http://localhost:4173/'
const wantShots = process.argv.includes('--shots')
const shotDir = 'tmp-shots'
if (wantShots) await mkdir(shotDir, { recursive: true })

const BENIGN = [/\/favicon\.ico$/]
const benign = (t) => BENIGN.some((re) => re.test(t))

/*
 * 控制台上**预期内**的那几条 —— 不是"把 error 都吞掉"：名字逐个点出来。
 *
 *   checkRemoteUpdate  浏览器预览里没有远端目录，mock 如实拒（桌面版才做这件事），
 *                      **每次加载都会报**；探针基线里一直有它
 *
 * 另有一条 `applyActivePreset` 只在套餐那一段放行（见 `bundleProbe` 那行注释）：
 * 从前官方线的「应用」在浏览器里没有那份字节、mock 如实抛；2026-10-08 起首页走
 * 「下载即得工作副本」（先 `ensureUserCopy` 再应用「我那一份」），假后端里这条路
 * 真的能走通 —— 放行那一行保留（旧行为不冲突），不再依赖它触发。
 */
const EXPECTED = [/未实现的接口: checkRemoteUpdate/]
let bundleProbe = false

const browser = await chromium.launch({ channel: 'msedge' })
const page = await browser.newPage({ viewport: { width: 1760, height: 900 } })

const problems = []
page.on('console', (m) => {
  if (m.type() !== 'error') return
  const at = m.location?.()?.url ?? ''
  const text = m.text()
  if (benign(at)) return
  if (EXPECTED.some((re) => re.test(text))) return
  if (bundleProbe && /未实现的接口: applyActivePreset/.test(text)) return
  problems.push(`console.error: ${text}${at ? ` @ ${at}` : ''}`)
})
page.on('pageerror', (e) => problems.push(`pageerror: ${e.message}`))
page.on('response', (r) => {
  if (r.status() >= 400 && !benign(r.url())) problems.push(`HTTP ${r.status()} ${r.url()}`)
})

/** 一屏的"看得见的事实" */
async function snap(tag) {
  const info = await page.evaluate(() => {
    const main = document.querySelector('main')
    const text = (main?.innerText ?? '').replace(/\s+/g, ' ').trim()
    const q = (sel) => main?.querySelectorAll(sel).length ?? 0
    return {
      text,
      svg: q('svg'),
      input: q('input'),
      button: q('button'),
      select: q('select'),
      /* 正文里出现的数字（机型/版本/参数值那一类），用来判断"是不是真有数据" */
      numbers: (text.match(/\d+(\.\d+)?/g) ?? []).slice(0, 12).join(' '),
    }
  })
  console.log(`\n[${tag}]`)
  console.log(`  正文   ${info.text.slice(0, 220)}`)
  console.log(`  控件   svg ${info.svg} · input ${info.input} · button ${info.button} · select ${info.select}`)
  console.log(`  数字   ${info.numbers}`)
  if (info.text.length === 0) problems.push(`${tag}: 正文是空的`)
  if (wantShots) await page.screenshot({ path: `${shotDir}/${tag}.png` })
  return info
}

/*
 * 读正文时的那个坑（第一批走查踩过）：首页那台向导是 SlideDeck，**五张卡都挂在 DOM 里**
 * （没显示的那几张只是被挪到视野外），所以 `main.innerText` 会把邻卡的文案一起读进来 ——
 * 只看这个串会以为"点一下跳到第三步"。判断当前在哪一页要看"刚出现的那句"
 * （比如点「现在开始」之后多出来的「品牌 / 机型 / 打印件版本」＝ 选择那一页），
 * 或者直接看截图。`--shots` 就是为这件事留的。
 */

/** 按文字点一颗按钮；找不到就返回 false（不抛） */
async function clickText(label) {
  const btn = page.getByRole('button', { name: label }).first()
  if ((await btn.count()) === 0) return false
  await btn.click()
  /* 那台向导是「抽卡式」滑片（SlideDeck）：换页有动画，量早了会拍到过渡中间那一帧 */
  await page.waitForTimeout(900)
  return true
}

await page.goto(url, { waitUntil: 'load' })
await page.waitForSelector('header', { timeout: 10000 })

console.log(`url ${url}`)
await snap('home-0')

/* 首页那台向导：把「开始 / 下一步」这类推进按钮点到点不动为止（最多 4 步） */
let advanced = 0
for (const label of ['现在开始', '开始', '下一步', '继续']) {
  if (await clickText(label)) {
    advanced += 1
    await snap(`home-step${advanced}`)
    break
  }
}
if (advanced === 0) {
  console.log('\n（没找到推进按钮 —— 首页可能停在摘要态，或按钮文案变了；不判失败）')
}

/*
 * --pick：在选择那一页上逐级点下去（机型 → 版本），然后查"大图到底出来没有"。
 *
 * 这一段是**移植时留下的教训**：`home/heroArt.ts` 里那五条图片路径原来写的是试验场
 * `public/` 根下的文件，产品仓一条都没有 —— 而首页落地那一态不挂 `<img>`，
 * 所以「页签点得开、控制台也干净」这两条**看不出这个问题**，要真选到某一台才会露出来。
 * 这里量的是 `naturalWidth`：路径错（SPA 回落成 index.html）时它是 0。
 */
if (process.argv.includes('--pick')) {
  console.log('\n[--pick] 逐级点下去')
  /*
   * 这三组是**分级揭示**的：先点品牌，机型那一组才出现；点了机型，版本那一组才出现。
   * 所以品牌名必须在列表里 —— 第一批探针少写了它，结果一个都没点到（0 张图，
   * 看起来像"图片坏了"，其实是"根本没选到机型"）。
   */
  for (const label of [
    '拓竹 (Bambu Lab)',
    'A1 mini',
    '标准版',
    '快拆版260628',
    /* 其余机型的版本名：目录里就这几个（A1 / A1 mini 三档、P1S 与 X1C 是 lite版） */
    'lite版',
    'A1',
    'P1S',
  ]) {
    const btn = page.getByRole('button', { name: label, exact: false }).first()
    if ((await btn.count()) === 0) continue
    await btn.click()
    await page.waitForTimeout(700)
    console.log(`  点了「${label}」`)
  }
  /* 大图是**淡入**的（useArtLayers 先预载图片、加载成功才上层）：量早了会看到 0 张 */
  await page.waitForTimeout(2000)
  /*
   * 再点「回主页」：大图住在**第 0 张卡**（选好机型之后它从欢迎版面换成机型摘要卡）。
   * 停在选择那一页时那张卡在牌堆后面，量不到 —— 第一批探针就停在那儿，看到 0 张图，
   * 一度以为图片路径全坏了（路径那处确实是坏的，但这条量法本身也不对）。
   */
  const back = page.getByRole('button', { name: '回主页' }).first()
  if ((await back.count()) > 0) {
    await back.click()
    await page.waitForTimeout(1600)
  }
  await snap('home-picked')
  const imgs = await page.evaluate(() =>
    [...document.querySelectorAll('img')].map((i) => ({
      src: i.getAttribute('src') ?? '',
      natural: i.naturalWidth,
      /* 在不在 main 里：大图那套是"卡片位 + 退出层"两层，可能不在当前可见的那张卡上 */
      inMain: Boolean(i.closest('main')),
    })),
  )
  const artSlots = await page.evaluate(
    () => document.querySelectorAll('[class*="art" i], [class*="hero" i]').length,
  )
  console.log(`  大图槽（class 含 art/hero 的元素）  ${artSlots}`)
  console.log(`  图片   ${imgs.length} 张：${imgs.map((i) => `${i.src}(${i.natural}px)`).join(' · ') || '（一张都没有）'}`)
  const broken = imgs.filter((i) => i.natural === 0)
  if (broken.length > 0) problems.push(`图片没加载成功（路径或文件不对）：${JSON.stringify(broken)}`)

  /*
   * 再走一遍**两个选择层级**，把图位分层钉住（2026-10-03 第三刀）：
   *
   *   选到机型（还没选版本）→ `printers/a1mini.webp`   ← 机型图
   *   再选到版本            → `printers/a1mini.webp`   ← **版本没配图 ⇒ 回落机型图**
   *
   * 两条期待值相同是**故意的**：图位分层是「品牌图 / 机型图 / 版本图，版本缺则回落机型」，
   * 而 A1 mini 的 `STANDARD` 今天**没有**版本图 —— 所以第三级理应与第二级同图。
   * 哪天给那一版配了版本图，这条断言会红，那正是它该红的时候（改数据要改判据）。
   *
   * 这两个 URL 都是**台账 path**（`catalog.assets[]` 的 id → `path` → `/assets/<path>`），
   * 不是构建期写死的模块路径 —— 在客户端又回去认识具体文件名时，这条断言会红。
   *
   * 量法同前：**停在选择那一页时 DOM 里没有 `<img>`**（那张卡还没挂），必须回主页才量得到。
   */
  const artAt = async (steps, expect) => {
    if (!(await clickText('更换机型'))) {
      problems.push('量分层大图时找不到「更换机型」')
      return
    }
    for (const step of steps) await clickText(step)
    if (!(await clickText('回主页'))) {
      problems.push('量分层大图时找不到「回主页」')
      return
    }
    await page.waitForTimeout(1600)
    const srcs = await page.evaluate(() =>
      [...document.querySelectorAll('img')].map((i) => i.getAttribute('src') ?? ''),
    )
    const hit = srcs.find((s) => s.includes('/assets/')) ?? null
    console.log(`  分层大图 ${hit ?? '（一张都没有）'}（期待 ${expect}）`)
    if (hit !== expect) {
      problems.push(`分层大图不对：期待 ${expect}，实际 ${hit ?? '（一张都没有）'}`)
    }
  }
  await artAt(['A1 mini'], '/assets/printers/a1mini.webp')
  await artAt(['标准版'], '/assets/printers/a1mini.webp')
  /*
   * 第三级：**品牌图**（图位分层的最下一层）。A2L 没有机型图也没有版本图 ⇒ 回落品牌图，
   * 而品牌图**也在台账里**（`brands.toml` 的 `logo` 引用资产 id `bambu-lab-logo`），
   * 所以它同样是 `/assets/<台账 path>`，**不是**那个内置 data URI 字标。
   * 内置字标只在"台账没配品牌图 / 认不出那张图"时兜底（老版本 catalog.json 那条路）。
   */
  await artAt(['A2L'], '/assets/brands/bambu-lab-logo.svg')

  /*
   * 套餐四态（2026-10-06，`docs/HOME-BUNDLE-DOWNLOAD.md`）。
   *
   * 首页那颗按钮消费的是**整个套餐**（MKP + 配套 BBS），四态 = 已应用 / 下载并应用 /
   * 更新并应用 / 应用 —— 「缺」优先于「漂」，任一文件缺就是「下载并应用」。
   * 浏览器里的编排（`mock.ts` 的 MOCK_DOWNLOADED，四态各占一档）：
   *
   *   A1 / 标准版        MKP 已下载 + 配套 BBS 已下载 → 应用
   *   A1 / 快拆版6月以前  MKP 漂了   + 配套 BBS 已下载 → 更新并应用
   *   A1 mini / 标准版   MKP 漂了、没下载过          → 下载并应用
   *
   * 「全齐且全新时零下载」怎么判：浏览器里下载一定失败（没有下载区），所以
   *   - 「应用」那一态点下去**不该**出现我们自己那句「套餐没下全」—— 出现了就说明
   *     它走了下载分支（该下的算错了）；
   *   - 「更新并应用」「下载并应用」点下去**必须**出现那句 —— 那正是
   *     「只下缺的 ∪ 漂的，下不齐就停下、不应用」。
   */
  /*
   * 定位那颗按钮时踩到的坑（2026-10-06）：它是 `CardFrame` 的动作胶囊，而**别处也有同名的**
   *   - 预设列表浮层里每个文件一颗「应用」（标题写着"把这一份设成正在使用的配置"）
   *   - 参数页 / 校准页那些卡片的动作
   * 而首页那台向导是 SlideDeck —— **所有卡都挂在 DOM 里**（只有当前那张在视口内），
   * 于是 `getByRole(...).first()` 会点到浮层里那颗（Playwright 报"element is not visible"）。
   * 所以这里一律**只认 Playwright 说可见、可点的那一颗**；量状态也一样。
   */
  /* 这一段的「应用」点击会触发一条预期内的 mock 拒绝（见文件头的 `EXPECTED` 说明） */
  bundleProbe = true

  const LABELS = ['应用', '已应用', '下载并应用', '更新并应用', '套餐未配置', '读取中…']

  const readAction = async () => {
    for (const label of LABELS) {
      const all = page.getByRole('button', { name: label, exact: true })
      const n = await all.count()
      for (let i = 0; i < n; i += 1) {
        const b = all.nth(i)
        if (await b.isVisible()) return { label, button: b }
      }
    }
    return null
  }

  /**
   * 点一颗**可见**的按钮（同名的那几个里挑第一个能点的）。
   *
   * 先按全等找（「A1」不该点到「A1 mini」），找不到再放宽 —— 品牌那颗的
   * 可访问名里带了别的东西，全等匹配不上（上一版就是这么空手而归的）。
   */
  const clickVisible = async (label, tries = 18) => {
    /* 那一组是**分级揭示**的，还带换页动画：点完上一级要等它长出来，所以这里轮询着找 */
    for (let t = 0; t < tries; t += 1) {
      for (const exact of [true, false]) {
        const all = page.getByRole('button', { name: label, exact })
        const n = await all.count()
        for (let i = 0; i < n; i += 1) {
          const b = all.nth(i)
          if ((await b.isVisible()) && (await b.isEnabled())) {
            await b.click()
            await page.waitForTimeout(1100)
            return true
          }
        }
      }
      await page.waitForTimeout(200)
    }
    return false
  }

  /*
   * 量某一档：**每次从新加载的页面走一遍**（刷新 → 现在开始 → 品牌/机型/版本）。
   *
   * 两条踩过的坑：
   *   ① 不走「更换机型」那条路 —— 它挂在牌堆的另一张卡上，量完一次之后点不到；
   *   ② **品牌必须点**：那三组是分级揭示的（先点品牌，机型那一组才出现；
   *      点了机型，版本才出现）—— 刷新之后一台都没选，直接点「A1」是点不到的。
   */
  const bundleStateAt = async (steps) => {
    await page.goto(url, { waitUntil: 'load' })
    await page.waitForSelector('header', { timeout: 10000 })
    if (!(await clickVisible('现在开始'))) {
      problems.push('量套餐四态时点不到「现在开始」')
      return null
    }
    for (const step of steps) {
      if (!(await clickVisible(step))) {
        problems.push(`量套餐四态时点不到「${step}」`)
        return null
      }
    }
    let got = null
    for (let i = 0; i < 15; i += 1) {
      got = await readAction()
      if (got !== null && got.label !== '读取中…') break
      await page.waitForTimeout(200)
    }
    return got
  }

  const want = async (steps, expect) => {
    const got = await bundleStateAt(steps)
    const seen = got === null ? '（找不到那颗按钮）' : got.label
    console.log(`  套餐态 ${steps.join(' / ')}  ${seen}（期待 ${expect}）`)
    if (got === null || got.label !== expect) {
      problems.push(`套餐态不对：${steps.join(' / ')} 期待「${expect}」，实际「${seen}」`)
    }
    return got
  }

  /** 点那颗按钮，读回页面上那条错误（我们自己的「套餐没下全…」或 mock 的「…要用桌面版」） */
  const clickAction = async (got) => {
    await got.button.click()
    await page.waitForTimeout(1000)
    return page.evaluate(() => {
      const el = [...document.querySelectorAll('main p')].find((p) =>
        /套餐没下全|桌面版/.test(p.textContent ?? ''),
      )
      return el ? (el.textContent ?? '').trim() : ''
    })
  }

  /* ① 全齐且全新 → 「应用」，且**一个字节都不该下** */
  const readyState = await want(['拓竹 (Bambu Lab)', 'A1', '标准版'], '应用')
  if (readyState !== null) {
    const err = await clickAction(readyState)
    console.log(`  ① 点「应用」之后   ${err || '（没有错误行）'}`)
    if (err.includes('套餐没下全')) {
      problems.push('「应用」那一态却走了下载分支：全齐且全新时不该下任何字节')
    }
  }

  /* ② 有漂、无缺 → 「更新并应用」：只下漂的那份，下不齐就停（不许应用） */
  const driftState = await want(['拓竹 (Bambu Lab)', 'A1', '快拆版6月以前'], '更新并应用')
  if (driftState !== null) {
    const err = await clickAction(driftState)
    console.log(`  ② 点「更新并应用」之后   ${err || '（没有错误行）'}`)
    if (!err.includes('套餐没下全')) {
      problems.push('「更新并应用」下不齐时没有停下：没看到「套餐没下全」那句，可能已经应用了')
    }
  }

  /* ③ 有缺 → 「下载并应用」（缺优先于漂，即使另一份是漂的） */
  const missingState = await want(['拓竹 (Bambu Lab)', 'A1 mini', '标准版'], '下载并应用')
  if (missingState !== null) {
    const err = await clickAction(missingState)
    console.log(`  ③ 点「下载并应用」之后   ${err || '（没有错误行）'}`)
    if (!err.includes('套餐没下全')) {
      problems.push('「下载并应用」下不齐时没有停下：没看到「套餐没下全」那句，可能已经应用了')
    }
  }
  bundleProbe = false
}

/* 校准页 */
const calib = page.getByRole('button', { name: '校准', exact: true }).first()
if ((await calib.count()) > 0) {
  await calib.click()
  await page.waitForTimeout(400)
  await snap('calib')
  /* 板子上的热点：A40 的校准板是内联 SVG + 命中层 */
  const hit = await page.evaluate(() => {
    const main = document.querySelector('main')
    const withRole = main?.querySelectorAll('[role="button"], [tabindex]').length ?? 0
    return { withRole }
  })
  console.log(`  可点元素（role=button / tabindex）  ${hit.withRole}`)
} else {
  problems.push('校准: 找不到页签按钮')
}

await browser.close()

console.log('\n' + '-'.repeat(60))
if (problems.length > 0) {
  console.log(`发现问题 ${problems.length} 条：`)
  for (const p of problems) console.log(`  - ${p}`)
  process.exit(1)
}
console.log('首页与校准页都画出来了，控制台没有 error')
