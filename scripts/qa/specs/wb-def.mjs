/*
 * 编辑定义框：那几颗选择器都能开、能选、能关，取消不落盘。
 *
 * 守的是 2026-11-14 那一轮：浮层下拉全部换成模态框（`pick/PickerModal`）。
 * 判据不看长相，只看**点得开、选得中、关得掉、框里的字跟着变**。
 */

import { openParams, pickParam, openDef } from './_lib.mjs'

export const title = '编辑定义：选择器都是模态框'

/** 四颗单选选择器（它们都该点开一个框） */
const PICKERS = [
  { label: '所属分组 (sectionId)', pick: null },
  { label: '属于', pick: '不属于谁（顶层）' },
  { label: '前置条件', pick: '无条件' },
]

export default async function (t, page, url) {
  await openParams(page, url)
  await pickParam(page, '擦料塔位置 X')
  const def = await openDef(page)

  /* ① 触发器还在原位（换成模态框不该把它们弄丢） */
  for (const p of PICKERS) {
    const btn = def.getByRole('button', { name: p.label }).first()
    t.check(`「${p.label}」这一格在`, (await btn.count()) > 0)
  }
  await t.shot(page, '编辑定义 · 归属与条件')

  /* ② 每一颗都点得开、里有「确认」、Esc 关得掉，且只关里面那层 */
  for (const p of PICKERS) {
    const btn = def.getByRole('button', { name: p.label }).first()
    if ((await btn.count()) === 0) continue
    await btn.click()
    const inner = page.getByRole('dialog').last()
    await inner.waitFor({ timeout: 5000 })
    t.check(`「${p.label}」点开是一个模态框`, await inner.isVisible())
    t.check(
      `「${p.label}」框里有「确认」`,
      (await inner.getByRole('button', { name: '确认', exact: true }).count()) > 0,
    )
    /* 页脚那句说明不许把「取消 / 确认」挤成竖排两行 */
    const foot = await inner.locator('footer').first().evaluate((el) => ({
      h: el.getBoundingClientRect().height,
      buttons: [...el.querySelectorAll('button')].map((b) => ({
        text: (b.textContent ?? '').trim(),
        h: b.getBoundingClientRect().height,
        lines: Math.round(b.getBoundingClientRect().height / parseFloat(getComputedStyle(b).lineHeight || '17')),
      })),
    }))
    t.check(
      `「${p.label}」页脚是单行，按钮没被压换行`,
      foot.h < 60 && foot.buttons.every((b) => b.h < 34),
      `页脚 ${foot.h}px · ${JSON.stringify(foot.buttons)}`,
    )
    await page.keyboard.press('Escape')
    await page.waitForTimeout(200)
    t.check(
      `「${p.label}」Esc 只关那一层（定义框还在）`,
      await def.isVisible(),
    )
  }
  await t.shot(page, 'Esc 之后定义框还在')

  /* ③ 值类型换一个 → 触发器上的字跟着变（选择器真的把值写回草稿了） */
  const vtBtn = def.getByRole('button', { name: '值类型' }).first()
  const vtBefore = (await vtBtn.innerText()).trim()
  await vtBtn.click()
  const vtDlg = page.getByRole('dialog').last()
  await vtDlg.waitFor()
  await vtDlg.getByRole('option', { name: /int/ }).first().click()
  /* 点一行只高亮，不写回；要点「确认」才生效（作者的「这个都没有确认」） */
  await page.waitForTimeout(200)
  const vtPending = (await vtBtn.innerText()).trim()
  t.check('没点确认之前触发器不变', vtPending === vtBefore, `${vtBefore} → ${vtPending}`)
  await vtDlg.getByRole('button', { name: '确认', exact: true }).click()
  await page.waitForTimeout(300)
  const vtAfter = (await vtBtn.innerText()).trim()
  t.check('点确认之后触发器跟着变', vtBefore !== vtAfter, `${vtBefore} → ${vtAfter}`)
  await t.shot(page, '值类型改成 int')

  /* ④ 取消整框 → 定义没变（草稿语义还在） */
  await def.getByRole('button', { name: '取消', exact: true }).click()
  await page.waitForTimeout(400)
  await pickParam(page, '擦料塔位置 X')
  const def2 = await openDef(page)
  const vtAgain = (await def2.getByRole('button', { name: '值类型' }).first().innerText()).trim()
  t.check('取消定义框之后值类型没被写进去', vtAgain === vtBefore, `${vtAgain}`)
  await t.shot(page, '取消之后再打开 · 还是原来的值类型')
  await def2.getByRole('button', { name: '取消', exact: true }).click()
}
