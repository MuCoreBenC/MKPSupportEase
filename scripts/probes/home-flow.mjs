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

const browser = await chromium.launch({ channel: 'msedge' })
const page = await browser.newPage({ viewport: { width: 1760, height: 900 } })

const problems = []
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
