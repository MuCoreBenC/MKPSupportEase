/*
 * 组内排序模态框：位次输入、撤销、重做、恢复、确认落盘。
 *
 * 守的是 2026-11-14 那一轮（`pick/SortModal` + `wb_set_section_order`）：
 * 排序能挪的不再只是"自己那一条"，而且确认之前一个字节都不写。
 *
 * 拖拽这一条**没有在这里点**：playwright 的 `dragTo` 走的是 HTML5 DnD，
 * 而这一件是 pointer 事件手写的 —— 用错的 API 只会测出"假红"。
 * 拖拽由 AI 巡检那一轮人工看（`npm run qa -- --legacy` 之后看报告里的观察）。
 */

import { openParams, pickParam, openDef, sortRows, sortDialog } from './_lib.mjs'

export const title = '组内排序：位次 / 撤销 / 重做 / 恢复 / 确认'

/** 排序框里第 n 行（1 起）的行 */
const rowAt = (page, n) => sortDialog(page).locator('[role="listitem"]').nth(n - 1)

const posOf = (page, n) => rowAt(page, n).locator('input').first()

export default async function (t, page, url) {
  await openParams(page, url)
  await pickParam(page, '擦料塔位置 X')
  const def = await openDef(page)

  const trigger = def.getByRole('button', { name: '组内顺序' }).first()
  t.check('「组内顺序」这一格在', (await trigger.count()) > 0)
  const before = (await trigger.innerText()).trim()

  await trigger.click()
  const dlg = sortDialog(page)
  await dlg.waitFor({ timeout: 5000 })
  const rows0 = await sortRows(page)
  t.check('排序框按整组列出', rows0.length >= 2, `${rows0.length} 行`)
  await t.shot(page, '排序框 · 打开时')

  /* ⓪a 原位：按下不动 / 挪不到半行，都不许改顺序（作者：「我希望能停在原本的位置」） */
  const row2pre = await rowAt(page, 2).boundingBox()
  if (row2pre !== null) {
    const atX = row2pre.x + 60
    const atY = row2pre.y + row2pre.height / 2
    /* ① 只按下、不移动：连拖拽态都不该进 */
    await page.mouse.move(atX, atY)
    await page.mouse.down()
    await page.waitForTimeout(120)
    t.check(
      '按下不动不进入拖拽态',
      (await dlg.locator('[data-drag]').count()) === 0,
    )
    /* ② 在自己那一行里晃几下（远不到半行）：落点仍应是原位，不占位、不让位 */
    await page.mouse.move(atX, atY + 8, { steps: 3 })
    await page.waitForTimeout(120)
    t.check(
      '原位小幅移动不出现落点占位',
      (await dlg.locator('[data-slot]').count()) === 0,
    )
    t.check(
      '原位小幅移动不让别的行挪位',
      !(await rowAt(page, 1).evaluate((el) => el.style.transform.includes('translateY'))),
    )
    await page.mouse.up()
    /* 原位也要滑回：等落地动画走完（160ms + 清理帧）再看终态 */
    await page.waitForTimeout(260)
    t.check(
      '原位松手顺序一个字没变',
      JSON.stringify(await sortRows(page)) === JSON.stringify(rows0),
    )
    t.check(
      '原位松手不算一次改动（确认仍禁用）',
      await dlg.getByRole('button', { name: '确认', exact: true }).isDisabled(),
    )
    await t.shot(page, '原位松手 · 什么都没改')
  }

  /* ⓪b 真拖拽：跟手、实底不透明、其余行让位、松手吸附（2026-11-14 那一轮） */
  const row2 = await rowAt(page, 2).boundingBox()
  const row1 = await rowAt(page, 1).boundingBox()
  if (row2 !== null && row1 !== null) {
    await page.mouse.move(row2.x + 60, row2.y + row2.height / 2)
    await page.mouse.down()
    /* 拖到第 1 行的上半 ⇒ 落点下标 0，第一行要让位 */
    await page.mouse.move(row1.x + 60, row1.y + 4, { steps: 4 })
    const mid = await rowAt(page, 2).evaluate((el) => ({
      transform: el.style.transform,
      opacity: getComputedStyle(el).opacity,
      bg: getComputedStyle(el).backgroundColor,
    }))
    t.check('拖动时当前项跟手（transform 跟着指针）', mid.transform.includes('translate3d'), mid.transform)
    t.check('拖动时只动 Y、不缩放（没有左右晃）', !mid.transform.includes('scale'), mid.transform)
    t.check('拖动时不会拖选文字', await page.evaluate(() => (window.getSelection()?.toString() ?? '') === ''))
    t.check('拖动时当前项不透明', mid.opacity === '1', mid.opacity)
    t.check('拖动时当前项是实底绿带（有背景色）', mid.bg !== 'rgba(0, 0, 0, 0)', mid.bg)
    const shifted = await rowAt(page, 1).evaluate((el) => el.style.transform)
    t.check('其余行平滑让位', shifted.includes('translateY'), shifted)
    /* 落点：一整块背景色占位，而不是某一行上沿的一条横线 */
    const slot = await dlg.locator('[data-slot]').first()
    t.check('落点是一块背景占位', (await slot.count()) > 0, '')
    if ((await slot.count()) > 0) {
      const look = await slot.evaluate((el) => ({
        bg: getComputedStyle(el).backgroundColor,
        h: el.getBoundingClientRect().height,
        shadow: getComputedStyle(el).boxShadow,
      }))
      t.check('占位块有实底背景色', look.bg !== 'rgba(0, 0, 0, 0)', look.bg)
      t.check('占位块正好一行高', Math.round(look.h) === 30, `${look.h}px`)
      t.check('占位块不是一条横线', !look.shadow.includes('inset 0px 2px'), look.shadow)
    }
    const anyDropLine = await dlg.locator('[data-row]').evaluateAll((els) =>
      els.some((el) => el.hasAttribute('data-drop') || getComputedStyle(el).boxShadow.includes('inset 0px 2px')),
    )
    t.check('行上不再有落点横线', !anyDropLine)
    await t.shot(page, '拖拽中 · 悬浮放大 + 让位')
    await page.mouse.up()
    /* 落地动画本身：行先滑到落点，动画结束之前 order 不许变 */
    await page.waitForTimeout(40)
    const landing = await rowAt(page, 2).evaluate((el) => ({
      mark: el.dataset.landing ?? '',
      transform: el.style.transform,
      transition: getComputedStyle(el).transitionDuration,
    }))
    t.check('松手后有一段落地动画', landing.mark === 'true', JSON.stringify(landing))
    t.check('落地动画有过渡而不是瞬移', parseFloat(landing.transition) > 0, landing.transition)
    t.check(
      '落地动画期间其它行仍保持让位',
      (await rowAt(page, 1).evaluate((el) => el.style.transform)).includes('translateY'),
    )
    await page.waitForTimeout(240)
    /* 终态：没有残留 transform、没有残留标记，视觉坐标与 DOM 顺序一致 */
    const settled = await dlg.locator('[data-row]').evaluateAll((els) =>
      els.map((el) => ({ t: el.style.transform, mark: el.dataset.landing ?? '' })),
    )
    t.check(
      '落地结束后没有残留 transform 或标记',
      settled.every((x) => x.t === '' && x.mark === ''),
      JSON.stringify(settled),
    )
    const rowsDropped = await sortRows(page)
    t.check('松手吸附到最新位置', rowsDropped[0] === rows0[1], rowsDropped[0])
    await t.shot(page, '松手 · 已落到新位置')
    /* 恢复，后面的位次 / 撤销重做判据从打开时那一序重新走 */
    await dlg.getByRole('button', { name: '恢复', exact: true }).click()
    await page.waitForTimeout(200)
  }

  /* ① 位次输入：把第 2 行写成 1 → 它变成第一行 */
  await posOf(page, 2).fill('1')
  await posOf(page, 2).press('Enter')
  await page.waitForTimeout(300)
  const rows1 = await sortRows(page)
  t.check('输位次能挪到最前', rows1[0] === rows0[1] && rows1[1] === rows0[0], `${rows1[0]}`)
  await t.shot(page, '位次改成第 1 位')

  /* ② 撤销 / 重做 */
  await dlg.getByRole('button', { name: '撤销排序' }).click()
  await page.waitForTimeout(200)
  const rowsUndo = await sortRows(page)
  t.check('撤销回到原序', JSON.stringify(rowsUndo) === JSON.stringify(rows0))

  await dlg.getByRole('button', { name: '重做排序' }).click()
  await page.waitForTimeout(200)
  const rowsRedo = await sortRows(page)
  t.check('重做又回到改过的序', JSON.stringify(rowsRedo) === JSON.stringify(rows1))

  /* ③ 恢复 = 放弃排序 */
  await dlg.getByRole('button', { name: '恢复', exact: true }).click()
  await page.waitForTimeout(200)
  const rowsReset = await sortRows(page)
  t.check('恢复回到打开时那一刻', JSON.stringify(rowsReset) === JSON.stringify(rows0))
  t.check(
    '没改动时「确认」是灰的',
    await dlg.getByRole('button', { name: '确认', exact: true }).isDisabled(),
  )
  await t.shot(page, '恢复之后 · 确认不可点')

  /* ④ 改一次 → 确认 → 真的落盘（重开框读回来还是新序） */
  await posOf(page, 2).fill('1')
  await posOf(page, 2).press('Enter')
  await page.waitForTimeout(200)
  const rowsCommit = await sortRows(page)
  await dlg.getByRole('button', { name: '确认', exact: true }).click()
  await page.waitForTimeout(900)
  t.check('确认之后框关掉', (await dlg.count()) === 0)

  await trigger.click()
  const dlg2 = sortDialog(page)
  await dlg2.waitFor({ timeout: 5000 })
  const rowsAfter = await sortRows(page)
  t.check(
    '重开还是新序（真的写回去了）',
    JSON.stringify(rowsAfter) === JSON.stringify(rowsCommit),
    rowsAfter[0],
  )
  t.check(
    '触发器上的位次跟着变',
    (await trigger.innerText()).trim() !== before,
    `${before} → ${(await trigger.innerText()).trim()}`,
  )
  await t.shot(page, '重开确认 · 次序还在')

  await page.keyboard.press('Escape')
  await page.waitForTimeout(200)
  t.check('Esc 关排序框后定义框还在', await def.isVisible())
  await def.getByRole('button', { name: '取消', exact: true }).click()
}
