/*
 * 体验巡检（2026-11-14 第一轮）：只截图 + 记观察，不判红。
 *
 * 判红的那一半在别的 spec 里。这一条回答的是另一个问题：**点得到不等于好找**。
 * 观察按 `docs/QA-WORKBENCH.md` 的处境来，每条都要能指着一张图说。
 */

import { openParams, pickParam, openDef, sortDialog } from './_lib.mjs'

export const title = '体验巡检（观察，不判红）'

export default async function (t, page, url) {
  await openParams(page, url)
  await t.shot(page, '参数台 · 一屏')

  t.note('左栏机型树要展开到「标准版」才看得到参数 —— 首屏写着「还没选机型」，新用户会先愣一下（可发现性）')

  await pickParam(page, '擦料塔位置 X')
  await t.shot(page, '右栏 · 选中一条参数')

  const def = await openDef(page)
  await t.shot(page, '编辑定义 · 从上到下')

  t.note('「归属与条件」里四格并排：所属分组 / 组内顺序 / 属于 / 前置条件 —— 前两格是"摆哪儿"，后两格是"什么时候生效"，现在没有小标题把这两类分开')
  t.note('「组内顺序」的触发器只写「第 1 / 2 位」，不写"点开能排" —— 想挪位置的人得先猜到这位次是可点的')

  await def.getByRole('button', { name: '组内顺序' }).click()
  const dlg = sortDialog(page)
  await dlg.waitFor({ timeout: 5000 })
  await t.shot(page, '排序框 · 整组')

  t.note('排序框里页脚那行小字「顺序单独生效，不用等这一条的定义确认」是关键信息，但它和「取消 / 确认」挤在同一行右侧，眼睛容易只看见按钮')
  t.note('排序框只有这一组的行（2 行）—— 「组内顺序」这个名字说得对，但用户想"把三条参数排好"时可能以为能一次排全表')

  await page.keyboard.press('Escape')
  await page.waitForTimeout(150)

  await def.getByRole('button', { name: '所属分组 (sectionId)' }).click()
  await page.getByRole('dialog', { name: /选择所属分组/ }).waitFor({ timeout: 5000 })
  await t.shot(page, '所属分组 · 框里')
  t.note('所属分组框里每行右边还有改名与上下移两枚小按钮，位置没变、也没解释 —— 从浮层搬进模态框之后它们更容易被当成装饰')

  await page.keyboard.press('Escape')
  await def.getByRole('button', { name: '取消', exact: true }).click()
}
