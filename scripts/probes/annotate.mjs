/*
 * 标注台（`src/workbench/views/AnnotatePage.tsx`）—— 在图上拖框、调样子、把**源稿写进仓库**。
 *
 * 守这几条（作者 2026-10-16 点名的）：
 *  ① 页面能开、图能取到（754×904 真尺寸）、两个框按百分比落在该在的地方
 *     —— 截图 `tmp-shots/annotate-100-light.png` / `-dark.png`（1:1 + 网格 50，直接读像素）
 *  ② 拖一下框，数字真的跟着变（并验 ⌘Z 能倒回去）；拉右下角手柄，宽高跟着变
 *  ③ 换到深色那张：**浅深共用一份**（框原样在）
 *  ④ 「框的样子」圆角 / 编号改完，图上**立刻**跟着变（与客户端同一套 CSS 变量）
 *  ⑤ 「写入仓库」→ 状态条说已写进仓库；换图回来框与样式**从仓库读回来**（不是靠本机暂存）
 *     ★ 内容一样时**不许**弹"本机那份比仓库新"那种假冲突
 *  ⑥ 导出的 JSON 带着 `style`
 *  ⑦ 视图切换器（2026-10-16 第二版）：`编辑 | 看效果 | 客户视角` **三档互斥**，切档只换显示 ——
 *     缩放 / 位置 / 尺寸一律不动（作者：「把在看效果和客户视角加一个切换器，我加切换器这个
 *     它的其他的都不变了」）；「适应」有选中态
 *  ⑧ 成品图**按参考显示比例烧**（`314 / 图宽`，与标注台当次缩放无关）—— 同一份标注在哪个
 *     视角保存都一样（作者：「客户视角明显和在看效果差很远」那条）
 *
 * 跑法（4174 那台 preview 先起着，见 workbench-build.mjs 头注）：
 *   node scripts/probes/annotate.mjs
 */
import { mkdir } from 'node:fs/promises'

import { chromium } from 'playwright-core'

const url = 'http://localhost:4174/workbench.html?mock=1'
const shotDir = 'tmp-shots'
await mkdir(shotDir, { recursive: true })

/* 现有的那份 marks（`openWithGuide.ts`）—— 结构与客户端同一份形状（含 note） */
const SEED = {
  /* 键是**那一组**（2026-10-16 起 = 台账 `[[groups]]` 的组 id；从前是抹掉浅深后缀）
     —— 与页面里的 `groupKeyOf()` 同值 */
  'open-with': {
    marks: [
      { id: 'a', at: { left: 18.8, top: 58.2, width: 62.3, height: 4.4 }, note: '打开方式', off: false },
      { id: 'b', at: { left: 18.8, top: 68.4, width: 19.4, height: 4.4 }, note: '全部更改', off: false },
    ],
    zoom: 1,
    grid: true,
    snap: false,
    savedAt: 1_700_000_000_000,
  },
}

const problems = []
const say = (ok, text) => {
  console.log(`${ok ? '✓' : '✗'} ${text}`)
  if (!ok) problems.push(text)
}

const browser = await chromium.launch({ channel: 'msedge' })
const page = await browser.newPage({ viewport: { width: 1680, height: 1180 }, deviceScaleFactor: 2 })
page.on('console', (m) => {
  if (m.type() !== 'error') return
  const at = m.location?.()?.url ?? ''
  if (/favicon\.ico$/.test(at)) return
  problems.push(`console.error: ${m.text()}${at ? ` @${at}` : ''}`)
})
page.on('pageerror', (e) => problems.push(`pageerror: ${e.message}`))
page.on('response', (r) => {
  if (r.status() >= 400 && !/favicon/.test(r.url())) problems.push(`HTTP ${r.status()} ${r.url()}`)
})

await page.addInitScript((seed) => {
  localStorage.setItem('mkp.wb.annotate', JSON.stringify(seed))
}, SEED)

await page.goto(url)
await page.getByRole('button', { name: '标注台' }).click()

const shot = (name) => page.locator('img[src*="open-with-"]').locator('..')
const imgOf = (name) => page.locator(`img[src*="${name}"]`)

await imgOf('open-with-light').waitFor({ timeout: 10_000 })
const natural = await imgOf('open-with-light').evaluate((i) => [i.naturalWidth, i.naturalHeight])
say(natural[0] === 754 && natural[1] === 904, `图是 754×904（读到 ${natural.join('×')}）`)

/* ★★★ 标注台显示的是**原图**（2026-10-16 作者：「标注台显示的不是原本的图片，是已经被改过的
   图片……右边把眼睛关了它还显示」）。判据：src 是 `source/` 下的原图，不是 `.rendered.webp`。 */
const normalSrc = await imgOf('open-with-light').getAttribute('src')
say(
  /\/source\//.test(normalSrc) && !/\.rendered\.webp$/.test(normalSrc),
  `普通模式显示的是**原图**（${normalSrc.split('/').slice(-2).join('/')}）`,
)

/* ★★★ 「客户视角」= 显示**成品图**（框烧在像素里）且**不再叠画** frames */
await page.getByRole('button', { name: '客户视角' }).click()
await page.waitForTimeout(500)
const cvSrc = await page
  .locator('img[src*="open-with-light"]')
  .first()
  .getAttribute('src')
say(
  /\.rendered\.webp(\?|$)/.test(cvSrc ?? ''),
  `客户视角显示**成品图**（${(cvSrc ?? '').split('/').pop()}）`,
)
await page.waitForTimeout(300)
/* ★ 只数**板上**的框（右栏图层列表也用 `data-on`，别把它算进来 —— 实测踩过这个坑） */
const boardMarks = () => page.locator('div[class*="board"] div[data-on]').count()
const cvMarks = await boardMarks()
say(cvMarks === 0, `客户视角**不叠画** frames（板上读到 ${cvMarks} 个）`)
await page.screenshot({ path: `${shotDir}/annotate-client-view.png` })
/* 切回可编辑的原图（★ 切换器是三档互斥：回编辑要**点「编辑」那一档**，再点「客户视角」不会关） */
await page.getByRole('button', { name: '编辑', exact: true }).click()
await page.waitForTimeout(500)
const backSrc = await page
  .locator('[class*="board"] img')
  .first()
  .getAttribute('src')
say(
  /\/source\//.test(backSrc ?? ''),
  `退出客户视角回到**原图**（${(backSrc ?? '').split('/').slice(-2).join('/')}）`,
)
say((await boardMarks()) === 2, '退出后 frames 又画回来了（板上 2 个）')

/* ---- ⑦ 视图切换器：三档互斥、切档**只换显示**（作者 2026-10-16 第二版） ---- */
const bgOf = (name, opts = {}) =>
  page.getByRole('button', { name, ...opts }).evaluate((b) => getComputedStyle(b).backgroundColor)
const zoomText = () => page.locator('[class*="_zoom_"]').innerText()
const boardBox = () => page.locator('div[class*="board"]').boundingBox()

await page.getByRole('button', { name: '适应' }).click()
await page.waitForTimeout(400)
say(
  (await bgOf('适应')) !== (await bgOf('100%')),
  `「适应」有选中态（适应底 ${await bgOf('适应')} ≠ 100% 底 ${await bgOf('100%')}）—— 一眼看出点没点`,
)
const z0 = await zoomText()
const bx0 = await boardBox()
await page.getByRole('button', { name: '看效果' }).click()
await page.waitForTimeout(300)
const z1 = await zoomText()
const bx1 = await boardBox()
await page.getByRole('button', { name: '客户视角' }).click()
await page.waitForTimeout(700)
const z2 = await zoomText()
const bx2 = await boardBox()
say(z0 === z1 && z1 === z2, `切档不动缩放（${z0} → ${z1} → ${z2}）`)
say(
  bx0 !== null &&
    bx1 !== null &&
    bx2 !== null &&
    Math.abs(bx2.width - bx0.width) < 0.6 &&
    Math.abs(bx2.x - bx0.x) < 0.6 &&
    Math.abs(bx2.y - bx0.y) < 0.6,
  `切档不动位置与尺寸（宽 ${bx0.width} → ${bx2.width}；x ${bx0.x} → ${bx2.x}）—— 这样才比得出差别`,
)
await page.getByRole('button', { name: '编辑', exact: true }).click()
await page.waitForTimeout(300)

/* ---- ⑦b 反选：再点一次当前那一档 = **什么都不叠**（只看原图）—— 作者 2026-10-17 ---- */
await page.getByRole('button', { name: '编辑', exact: true }).click()
await page.waitForTimeout(400)
const noMarks = await boardMarks()
const noHandle = await page.locator('span[data-edge="se"]').count()
say(noMarks === 0, `再点一次「编辑」⇒ 一个框都不叠（读到 ${noMarks} 个）`)
say(noHandle === 0, `那一档连手柄也不露（读到 ${noHandle} 个）`)
say(
  (await page.locator('[class*="_info_"]').innerText()).includes('什么都没叠'),
  '面板顶上那条说明跟着改成「什么都没叠：这张就是那张原图」',
)
await page.getByRole('button', { name: '编辑', exact: true }).click()
await page.waitForTimeout(300)
say((await boardMarks()) === 2, `再点一下回到「编辑」：框回来了（读到 ${await boardMarks()} 个）`)

await page.getByRole('button', { name: '100%' }).click()
await page.waitForTimeout(300)

/* 「一组」的呈现（作者 2026-10-16：「需要是套装的感觉……用同样的框的就是一组」）——
   选图那里成组的收进 `<optgroup>`，页脚点名同组那一张 */
const groupLabels = await page.locator('select optgroup').evaluateAll((gs) => gs.map((g) => g.label))
say(
  groupLabels.some((l) => /一组 2 张/.test(l) && /共用一份标注/.test(l)),
  `选图按组摆：${groupLabels.join(' ｜ ') || '（一个组都没有）'}`,
)
/* ★ 那条说明从画布底下搬到了**面板顶上**（2026-10-16 第二版，作者：「放到别的地方」） */
const info0 = await page.locator('[class*="_info_"]').innerText()
say(/同组还有/.test(info0), `面板顶上那条说明点名同组那张：${info0.replace(/\s+/g, ' ').trim()}`)

const board = shot()
const marks = board.locator('div[data-on]')
say((await marks.count()) === 2, `板上两个框（读到 ${await marks.count()} 个）`)

const rectOf = async (i) => {
  const el = marks.nth(i)
  await el.scrollIntoViewIfNeeded()
  return el.evaluate((n) => {
    const r = n.getBoundingClientRect()
    const b = n.parentElement.getBoundingClientRect()
    return [Math.round(r.left - b.left), Math.round(r.top - b.top), Math.round(r.width), Math.round(r.height)]
  })
}
console.log('  框 1（板上 px）:', (await rectOf(0)).join(' / '))
console.log('  框 2（板上 px）:', (await rectOf(1)).join(' / '))
console.log('  期望：142/526/470/40 与 142/618/146/40（= 百分比 × 754×904；canvas 扫出来的）')

await board.screenshot({ path: `${shotDir}/annotate-100-light.png` })
await page.screenshot({ path: `${shotDir}/annotate-page.png` })

/* ---- ② 拖一下 ---- */
const nums = page.locator('input[type="number"]')
const readX = async () => Number(await nums.nth(0).inputValue())

await marks.nth(0).click()
const x0 = await readX()
const b0 = await marks.nth(0).boundingBox()
await page.mouse.move(b0.x + b0.width / 2, b0.y + b0.height / 2)
await page.mouse.down()
await page.mouse.move(b0.x + b0.width / 2 + 20, b0.y + b0.height / 2 + 20, { steps: 6 })
await page.mouse.up()
const x1 = await readX()
say(x1 === x0 + 20, `拖动：X ${x0} → ${x1}（期望 +20）`)

await page.keyboard.press('Meta+z')
await page.waitForTimeout(250)
await marks.nth(0).click()
const x2 = await readX()
say(x2 === x0, `⌘Z：X 回到 ${x2}（期望 ${x0}）`)

/* ---- ③ 拉手柄 ---- */
const w0 = Number(await nums.nth(2).inputValue())
const handle = marks.nth(0).locator('span[data-edge="se"]')
const hb = await handle.boundingBox()
await page.mouse.move(hb.x + hb.width / 2, hb.y + hb.height / 2)
await page.mouse.down()
await page.mouse.move(hb.x + hb.width / 2 + 30, hb.y + hb.height / 2, { steps: 6 })
await page.mouse.up()
const w1 = Number(await nums.nth(2).inputValue())
/* ±1：手柄有 9px 宽、抓到的是它的中心，取整会差一个像素 —— 不是功能问题 */
say(Math.abs(w1 - w0 - 30) <= 1, `拉右下角：宽 ${w0} → ${w1}（期望 +30 ±1）`)

/* ===========================================================================
 * 编辑体验（2026-10-16 作者点名的四条）—— 这是本轮的主戏
 * ========================================================================= */

/* ---- ⑦ 数字格：退格 / 删除**只改数字**，绝不删掉整个框（最严重那条） ---- */
await marks.nth(0).click()
const xInp = page.locator('label:has-text("X") input[type="number"]').first()
await xInp.click()
const px0 = Number(await xInp.inputValue())
const cntBefore = await shot().locator('div[data-on]').count()
const align = await xInp.evaluate((el) => getComputedStyle(el).textAlign)
say(align === 'left', `数字格左对齐（读到 ${align}）`)
await page.keyboard.press('Backspace')
await page.keyboard.press('Backspace')
await page.keyboard.press('Delete')
const cntAfterKeys = await shot().locator('div[data-on]').count()
say(
  cntAfterKeys === cntBefore,
  `数字格里连按退格/删除：框还在（${cntBefore} → ${cntAfterKeys}，期望不变 —— 只改那个数字）`,
)
/* 焦点在数字格时按 ⌘Z：走的是浏览器文本撤销，**不许**把框撤销掉 */
await page.keyboard.press('Meta+z')
const cntAfterUndo = await shot().locator('div[data-on]').count()
say(cntAfterUndo === cntBefore, `数字格里 ⌘Z：也不删框（读到 ${cntAfterUndo} 个）`)
/* 把数字填回原值再离开（否则 onBlur 会把刚才删空的值提交掉，污染后面几条） */
await xInp.fill(String(Number(await xInp.inputValue()) || px0))
await xInp.blur()
await page.waitForTimeout(120)

/* ---- ⑧ 删框之后 ⌘Z **一定**能把框找回来 ---- */
/* 点一下框：`startDrag` 会把焦点收到页根上（否则 Delete / ⌘Z 全落在 body） */
await marks.nth(0).click()
await page.keyboard.press('Delete')
await page.waitForTimeout(150)
const cntDeleted = await shot().locator('div[data-on]').count()
say(cntDeleted === 1, `删掉一个框（读到 ${cntDeleted} 个）`)
await page.keyboard.press('Meta+z')
await page.waitForTimeout(200)
const cntRestored = await shot().locator('div[data-on]').count()
say(cntRestored === 2, `⌘Z 把删掉的框找回来了（读到 ${cntRestored} 个，期望 2）`)

/* ---- ⑨ 撤销 / 重做**不摆按钮了**（作者 2026-10-16：「撤销重做也不用显示出来，
 *         用快捷键就可以了」）—— 能力还在，走 ⌘Z / ⇧⌘Z ---- */
say(
  (await page.getByRole('button', { name: '↶ 撤销' }).count()) === 0 &&
    (await page.getByRole('button', { name: '↷ 重做' }).count()) === 0,
  '标注台面板上**不再有**撤销 / 重做按钮（改走快捷键）',
)

/* 先做一次可撤销的改动（挪一格），再验快捷键真能把状态倒回去、再正回来 */
await marks.nth(0).click()
const rx0 = await readX()
const rb = await marks.nth(0).boundingBox()
await page.mouse.move(rb.x + rb.width / 2, rb.y + rb.height / 2)
await page.mouse.down()
await page.mouse.move(rb.x + rb.width / 2 + 20, rb.y + rb.height / 2, { steps: 5 })
await page.mouse.up()
const rx1 = await readX()
await page.keyboard.press('Meta+z')
await page.waitForTimeout(200)
await marks.nth(0).click()
const rx2 = await readX()
say(rx1 === rx0 + 20 && rx2 === rx0, `⌘Z 撤销：X ${rx0} → ${rx1} → ${rx2}（期望 +20 后回 ${rx0}）`)
await page.keyboard.press('Meta+Shift+z')
await page.waitForTimeout(200)
await marks.nth(0).click()
const rx3 = await readX()
say(rx3 === rx1, `⇧⌘Z 重做：X 回到 ${rx3}（期望 ${rx1}）`)
/* 收拾干净：把这一格撤销回去，后面那几条才好算 */
await page.keyboard.press('Meta+z')
await page.waitForTimeout(200)

/* ---- ⑨b 页头那一槽：有文档的页才有（作者：「每一页都固定好，就用上面那一个」） ---- */
const head = page.locator('[class*="headOps"]')
const saveBtn = head.getByRole('button', { name: '保存' })
const discardBtn = head.getByRole('button', { name: '放弃' })
say((await head.count()) === 1, '页头右上角有那**唯一**一槽（状态字 + 放弃 + 保存）')
say((await saveBtn.count()) === 1, '标注台上页头有「保存」（它接管了这一槽）')
/*
 * ★ 保存那颗**默认灰、能保存才上主色**（2026-10-17 作者：「保存应该默认灰色，可以保存才
 *   显示，用有颜色的」）—— 判据是 `btnPrimary` 那一个类在不在（不在 = 普通灰按钮）。
 *   ★ 这一刻仓库里还没有这一份（`repo.doc === null` ⇒ 判脏为真）⇒ 它**是**主色；
 *     "灰的那一条"放在下面保存之后验（那时才真和仓库一致）。
 */
const saveCls = (await saveBtn.getAttribute('class')) ?? ''
say(
  /btnPrimary/.test(saveCls) && (await saveBtn.isEnabled()),
  '仓库还没有这一份时「保存」就是主色、点得动',
)
/* 切到设置页（只读、没有自己那份）⇒ 这一槽**不显示** */
await page.getByRole('button', { name: '设置' }).click()
await page.waitForTimeout(400)
say((await page.locator('[class*="headOps"]').count()) === 0, '设置页（只读）**不显示**那一槽')
await page.getByRole('button', { name: '标注台' }).click()
await imgOf('open-with-light').waitFor({ timeout: 10_000 })
await page.waitForTimeout(300)

/* ---- ⑩ 「放弃」是真动作：退回基线，且**这一步本身可撤销** ---- */
/* 先造一处改动（挪 X），再点页头那颗放弃 */
await marks.nth(0).click()
const dx0 = await readX()
const db = await marks.nth(0).boundingBox()
await page.mouse.move(db.x + db.width / 2, db.y + db.height / 2)
await page.mouse.down()
await page.mouse.move(db.x + db.width / 2 + 30, db.y + db.height / 2, { steps: 5 })
await page.mouse.up()
const dx1 = await readX()
say(await discardBtn.isEnabled(), '有改动时页头「放弃」可点')
/* 同上那条的反面：有改动 ⇒ 「保存」上主色、点得动（与"没改动是灰的"配成一对） */
const saveCls2 = (await saveBtn.getAttribute('class')) ?? ''
say(
  /btnPrimary/.test(saveCls2) && (await saveBtn.isEnabled()),
  '有改动时「保存」上主色、点得动',
)
await discardBtn.click()
await page.waitForTimeout(250)
await marks.nth(0).click().catch(() => {})
/* 放弃 = 退回打开时那份（此时本机暂存那份被清、仓库还没有 ⇒ 空）—— 再 ⌘Z 撤回这一步 */
await page.keyboard.press('Meta+z')
await page.waitForTimeout(250)
await marks.nth(0).click({ timeout: 2000 }).catch(() => {})
const dx2 = await readX()
say(
  dx1 === dx0 + 30 && dx2 === dx1,
  `放弃 → ⌘Z 能撤回这一步：X ${dx0} → ${dx1} →（放弃）→ ⌘Z → ${dx2}（期望回到 ${dx1}）`,
)
await page.keyboard.press('Meta+z')
await page.waitForTimeout(250)

/* ---- ⑪ 保存之后**仍然**能撤销（栈不清） ---- */
await marks.nth(0).click()
const sx0 = await readX()
const sb = await marks.nth(0).boundingBox()
await page.mouse.move(sb.x + sb.width / 2, sb.y + sb.height / 2)
await page.mouse.down()
await page.mouse.move(sb.x + sb.width / 2 + 15, sb.y + sb.height / 2, { steps: 5 })
await page.mouse.up()
const sx1 = await readX()
await saveBtn.click()
await page.waitForTimeout(500)
/* 保存后焦点被收回本页 ⇒ ⌘Z 还能按（作者要的"保存了也能撤销"） */
await page.keyboard.press('Meta+z')
await page.waitForTimeout(250)
await marks.nth(0).click()
const sx2 = await readX()
say(sx1 === sx0 + 15 && sx2 === sx0, `保存后 ⌘Z：X ${sx0} → ${sx1} →（保存）→ 撤销 → ${sx2}（期望回 ${sx0}）`)
await page.keyboard.press('Meta+Shift+z')
await page.waitForTimeout(250)

/* ---- 换深色那张：同一份框（浅深共用一份标注） ---- */
const optionOf = (needle) =>
  page
    .locator('select option')
    .evaluateAll((os, n) => os.find((o) => (o.textContent ?? '').includes(n))?.value ?? '', needle)
const darkVal = await optionOf('深色')
const lightVal = await optionOf('浅色')
await page.locator('select').selectOption(darkVal)
await imgOf('open-with-dark').waitFor({ timeout: 10_000 })
await page.getByRole('button', { name: '100%' }).click()
await page.waitForTimeout(400)
const darkMarks = shot().locator('div[data-on]')
say((await darkMarks.count()) === 2, `深色那张上还是那两个框（读到 ${await darkMarks.count()} 个 —— 浅深共用一份）`)
await shot().screenshot({ path: `${shotDir}/annotate-100-dark.png` })

/* ---------------------------------------------------------------------------
 * ④⑤⑥ 框的样子 + 写进仓库（作者：「改这个框框的圆角这一些」「我希望它是随着这个项目仓库的」）
 * ------------------------------------------------------------------------- */

/* ---- ④ 改圆角 / 编号：图上立刻跟着变（与客户端同一套 CSS 变量） ---- */
const back = await optionOf('浅色')
await page.locator('select').selectOption(back)
await imgOf('open-with-light').waitFor({ timeout: 10_000 })
await page.waitForTimeout(400)
await shot().locator('div[data-on]').nth(0).click()
const radiusInput = page.locator('label:has-text("圆角") input')
const badgeInput = page.locator('label:has-text("编号") input')
await radiusInput.fill('14')
await radiusInput.blur()
await page.waitForTimeout(120)
await badgeInput.fill('26')
await badgeInput.blur()
await page.waitForTimeout(120)

const styled = await shot()
  .locator('div[data-on]')
  .nth(0)
  .evaluate((el) => {
    const badge = el.querySelector('span')
    return {
      radius: Number.parseFloat(getComputedStyle(el).borderTopLeftRadius),
      badge: Number.parseFloat(getComputedStyle(badge).width),
    }
  })
/*
 * ★★★ 画布上量到的**屏幕**尺寸 = 设定值 ÷ `--zoom-k`（2026-10-16 第二版）：画布画的是
 * "成品图像素 × 当前缩放"，所以屏幕上会比设定值大一档。100% 下 `--zoom-k` = 314/754 ≈ 0.416
 * ⇒ 设定 14 量到 ≈ 33.6。那三格填的仍是**客户端那套屏幕尺寸**（与 `.badge` / `.mark` 同一口径）。
 */
const zoomKOf = () =>
  page
    .locator('div[class*="board"]')
    .evaluate((b) => Number.parseFloat(getComputedStyle(b).getPropertyValue('--zoom-k')) || 1)
const k14 = await zoomKOf()
say(
  Math.abs(styled.radius - 14 / k14) < 0.6,
  `改圆角 14：图上量到 ${styled.radius}px（= 14 ÷ --zoom-k ${k14.toFixed(3)} —— 画布画的是成品图像素）`,
)
say(
  Math.abs(styled.badge - 26 / k14) < 0.6,
  `改编号 26：圆点量到 ${styled.badge}px（= 26 ÷ --zoom-k ${k14.toFixed(3)}）`,
)

/* ---- ⑥ 复制 JSON：带着 style（★ 现在藏在「导出／导入（给开发用）」那一行后面，先点开） ---- */
await page.context().grantPermissions(['clipboard-read', 'clipboard-write'])
const ioHead = page.getByRole('button', { name: /导出／导入/ })
say((await ioHead.count()) === 1, '「导出／导入（给开发用）」收成一行（默认看不见那三颗）')
say(
  (await page.getByRole('button', { name: '复制 JSON' }).count()) === 0,
  '收起时「复制 JSON」不占地方',
)
await ioHead.click()
await page.waitForTimeout(200)
say((await page.getByRole('button', { name: '复制 JSON' }).count()) === 1, '点开后那三颗才出来')
await page.getByRole('button', { name: '复制 JSON' }).click()
await page.waitForTimeout(200)
const clip = await page.evaluate(() => navigator.clipboard.readText())
say(/"style"\s*:\s*\{/.test(clip) && /"radius"\s*:\s*14/.test(clip), '复制 JSON：style 跟着出去了')

/* ---- ⑤ 保存（页头那颗）→ 状态字说已写进仓库；换图回来框与样式从仓库读回来 ---- */
await saveBtn.click()
await page.waitForTimeout(450)
const bar = await head.innerText()
say(/已写进仓库/.test(bar), `页头状态字说「${bar.replace(/\s+/g, ' ').trim()}」`)
/* 保存之后与仓库一致 ⇒ 「保存」退成**普通灰按钮**（点不动、也没有主色） */
const saveCls3 = (await saveBtn.getAttribute('class')) ?? ''
say(
  !/btnPrimary/.test(saveCls3) && (await saveBtn.isDisabled()),
  '保存之后（和仓库一致）「保存」退成灰色、点不动',
)

await page.locator('select').selectOption(darkVal)
await imgOf('open-with-dark').waitFor({ timeout: 10_000 })
await page.waitForTimeout(300)
await page.locator('select').selectOption(back)
await imgOf('open-with-light').waitFor({ timeout: 10_000 })
await page.waitForTimeout(500)
say(
  (await shot().locator('div[data-on]').count()) === 2,
  `换回来：框还在（从仓库读的，本机暂存保存后就清了）`,
)
const kept = await shot()
  .locator('div[data-on]')
  .nth(0)
  .evaluate((el) => Number.parseFloat(getComputedStyle(el).borderTopLeftRadius))
const kBack = await zoomKOf()
say(Math.abs(kept - 14 / kBack) < 0.6, `换回来：圆角还是 14（量到 ${kept}px = 14 ÷ ${kBack.toFixed(3)}）`)
say(
  (await page.getByRole('button', { name: '用本机那份' }).count()) === 0,
  '换回来：没有"本机那份比仓库新"的假冲突（内容一样就不问）',
)
await page.screenshot({ path: `${shotDir}/annotate-repo.png` })

/* ---------------------------------------------------------------------------
 * ⑬ 点空白收起编辑区 / 空白处"点一下"不留小框（作者 2026-10-16）
 * ------------------------------------------------------------------------- */
/* 先选中一个框 ⇒ 下面那块编辑区出来（能看到 X 那个数字格） */
await shot().locator('div[data-on]').nth(0).click()
say((await page.locator('label:has-text("X") input[type="number"]').count()) === 1, '选中一个框：编辑区出来')
const bb = await shot().boundingBox()
const cntBeforeBlank = await shot().locator('div[data-on]').count()
/* 点画布**空白**（左下角，离两个框都远）——只点一下、不拖 */
await page.mouse.click(bb.x + 12, bb.y + bb.height - 12)
await page.waitForTimeout(250)
say(
  (await page.locator('label:has-text("X") input[type="number"]').count()) === 0,
  '点画布空白：编辑区收起（不选中任何框）',
)
say(
  (await shot().locator('div[data-on]').count()) === cntBeforeBlank,
  `点一下空白**不留下** 1×1 小框（仍是 ${cntBeforeBlank} 个）`,
)

/* 点列表空白：同样收起（先选中，再点列表的空白处）——
   ★ 限定在右侧面板里找 `list`：导航那棵树也有个 `navList`，`[class*="list"]` 会撞上 */
await shot().locator('div[data-on]').nth(0).click()
const listBox = await page.locator('[class*="panel"] [class*="list"]').first().boundingBox()
await page.mouse.click(listBox.x + listBox.width / 2, listBox.y + listBox.height - 4)
await page.waitForTimeout(250)
say(
  (await page.locator('label:has-text("X") input[type="number"]').count()) === 0,
  '点列表空白：也收起编辑区',
)

/* ---------------------------------------------------------------------------
 * ⑫ 冲突：能**先看再选**（作者：「我能不能预览一下这两个呢……看看到底是新不新」）
 * ------------------------------------------------------------------------- */
/* 造一份**比仓库新、内容又不一样**的本机暂存（`savedAt` 拉到很晚 = 它更新） */
await page.evaluate(() => {
  const raw = JSON.parse(localStorage.getItem('mkp.wb.annotate') ?? '{}')
  const key = 'open-with'
  raw[key] = {
    marks: [
      { id: 'x', at: { left: 25, top: 30, width: 40, height: 6 }, note: '本机那份', off: false },
    ],
    zoom: 'fit',
    grid: true,
    snap: false,
    size: { w: 754, h: 904 },
    savedAt: Date.now() + 86_400_000,
  }
  localStorage.setItem('mkp.wb.annotate', JSON.stringify(raw))
})
/*
 * 切到深色那张：`pick` 会读这一对的本机暂存 —— 我造的那份**比仓库新、又不一样** ⇒ 冲突挂出。
 * ★ 就在这一步判：编辑时的 `writeStore` 效果随后会把暂存按**当前**框重写一遍（`savedAt`=现在、
 *   内容=刚从仓库读到的那份）⇒ 再切回来"假冲突"就没了 —— 所以只能在这一拍抓。
 */
await page.locator('select').selectOption(darkVal)
await imgOf('open-with-dark').waitFor({ timeout: 10_000 })
await page.waitForTimeout(500)
const lookBtn = page.getByRole('button', { name: /看看两份差在哪|收起对比/ })
say((await lookBtn.count()) === 1, '冲突来了：有「看看两份差在哪」那颗（能先看再选）')
if ((await lookBtn.count()) === 1) {
  await lookBtn.click()
  await page.waitForTimeout(200)
  const repoDiff = await shot().locator('div[data-which="repo"]').count()
  const localDiff = await shot().locator('div[data-which="local"]').count()
  say(
    repoDiff >= 1 && localDiff >= 1,
    `对比层：图上同时画出「仓库那份」与「本机那份」（读到 repo=${repoDiff} local=${localDiff}）`,
  )
  await page.screenshot({ path: `${shotDir}/annotate-conflict.png` })
  await page.getByRole('button', { name: '用本机那份' }).click()
  await page.waitForTimeout(300)
  const afterUse = await shot().locator('div[data-on]').count()
  say(afterUse === 1, `选「用本机那份」：铺上本机那份（读到 ${afterUse} 个框，期望 1）`)
}

/* ———————————————— 「生成预览」= 拍一张照不存相册（2026-10-16 作者） ————————————————
   放在最后：它会把面板切进客户视角、img 换成 `blob:`，会打乱上面那些按原图写的判据。 */
/* 从头来一遍（上一段收尾停在"用本机那份"之后，状态不干净） */
await page.reload({ waitUntil: 'load' })
await page.locator('[class*="shellBody"]').first().waitFor({ timeout: 15000 })
await page.locator('button[title="标注台"]').first().click()
await imgOf('open-with-light').waitFor({ timeout: 10_000 })
await page.waitForTimeout(800)
const genBtn = page.getByRole('button', { name: /生成预览|生成中/ })
say((await genBtn.count()) === 1, '有「生成预览」那颗按钮')
if ((await genBtn.count()) === 1) {
  await genBtn.click()
  await page.waitForTimeout(1500)
  const prevSrc = await page.locator('[class*="board"] img').first().getAttribute('src')
  say(/^blob:/.test(prevSrc ?? ''), `生成预览 ⇒ 客户视角显示**临时那张**（${(prevSrc ?? '').slice(0, 10)}…）`)
  say((await page.getByText('刚生成的预览').count()) > 0, '页脚说明"这是刚生成的预览（临时）"')
  /*
   * ★★ **预览没存也算"要保存"**（2026-10-17 作者：「可是我生成了预览，你不让我保存怎么办？」）：
   *   框和仓库一模一样，但成品图（框烧进去那张）还是临时的 —— 比如换了编号字体之后，
   *   框没动也得重烧一张。⇒ 这时「保存」必须是**主色、可点**（不能因为它"不脏"就灰掉）。
   */
  const saveClsPrev = (await saveBtn.getAttribute('class')) ?? ''
  say(
    /btnPrimary/.test(saveClsPrev) && (await saveBtn.isEnabled()),
    '生成了预览（框没动）⇒「保存」依然是主色、可点',
  )
  await page.screenshot({ path: `${shotDir}/annotate-preview.png` })
}

console.log(problems.length === 0 ? '\n全绿' : `\n问题 ${problems.length} 条：\n${problems.join('\n')}`)
await browser.close()
