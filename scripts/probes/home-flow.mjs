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
