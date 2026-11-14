/*
 * 用户故事 4：「我给这组新建一个分组，把这条参数搬进去」。
 *
 * 照 `docs/QA-WORKBENCH.md` 的处境走一遍，把能断言的断言掉、拿不准的留成观察。
 * 这一条盯的是「所属分组」从浮层搬进模态框之后，新建 / 改名 / 组上下移有没有掉。
 */

import { openParams, pickParam, openDef } from './_lib.mjs'

export const title = '故事 4：新建分组并搬过去'

export default async function (t, page, url) {
  await openParams(page, url)
  await pickParam(page, '擦料塔位置 X')
  const def = await openDef(page)

  await def.getByRole('button', { name: '所属分组 (sectionId)' }).click()
  const dlg = page.getByRole('dialog', { name: /选择所属分组/ })
  await dlg.waitFor({ timeout: 5000 })
  await t.shot(page, '所属分组 · 框里')

  const add = dlg.getByRole('button', { name: /新增分组/ })
  t.check('框里有「＋ 新增分组」', (await add.count()) > 0)

  /* 打开新建的输入（组件自己的展开态，点一下就该出来） */
  if ((await add.count()) > 0) {
    await add.first().click()
    await page.waitForTimeout(300)
    const input = dlg.locator('input[type="text"], input:not([type])').first()
    const hasInput = (await input.count()) > 0
    t.check('点「＋ 新增分组」之后出现名字输入框', hasInput)
    if (hasInput) {
      await input.fill('质检试建')
      await input.press('Enter')
      await page.waitForTimeout(400)
      t.check(
        '新建的那一组出现在列表里',
        (await dlg.getByText('质检试建').count()) > 0,
      )
      await t.shot(page, '新建的这一组（待保存）')
    }
  }

  /*
   * 新建**顺手选中**它（`onAddSection` 把草稿的 sectionId 指过去）——
   * 建了还得再点一次才生效的话，那一步是白点的。
   */
  await page.keyboard.press('Escape')
  await page.waitForTimeout(200)
  const picked = (await def.getByRole('button', { name: '所属分组 (sectionId)' }).innerText()).trim()
  t.check('新建之后就选着它，触发器上带「待保存」', picked.includes('质检试建') && picked.includes('待保存'), picked)
  t.note('新建分组只进草稿 —— 取消保存后数据里不留空组，靠后端 ensure_section 的幂等兜着')

  /* 取消整框 → 草稿作废：重开时选的还是原来那一组（不留残渣） */
  await def.getByRole('button', { name: '取消', exact: true }).click()
  await page.waitForTimeout(400)
  await pickParam(page, '擦料塔位置 X')
  const def2 = await openDef(page)
  const back = (await def2.getByRole('button', { name: '所属分组 (sectionId)' }).innerText()).trim()
  t.check('取消之后重开：还选着原来那一组', !back.includes('质检试建'), back)
  await t.shot(page, '取消之后重开 · 没有留下那个新分组')
  await def2.getByRole('button', { name: '取消', exact: true }).click()
}
