/*
 * 分组成员编辑框：两层入口、两列、搜索、加入 / 取消 / 移出、改名、一次确认。
 *
 * 定位只用 role / aria-label / 看得见的文字，不写 class 名。
 * 抽屉那支笔与页签栏那支笔都是带 SVG 的按钮，playwright 的 click 落在图标中心 ——
 * 上一轮「↑↓ 点图标被 pointer capture 抢走」就是这条路径没测到。
 */

import { openParams, openSectionMembers, openTabMembers, pickParam } from './_lib.mjs'

export const title = '分组成员：分组 / 页签两个入口与两列编辑框'

const addButtons = (dlg) => dlg.getByRole('button', { name: /归入这一组$/ })
const cancelButtons = (dlg) => dlg.getByRole('button', { name: /取消归入$/ })
const outButtons = (dlg) => dlg.getByRole('button', { name: /移出本组$/ })
const backButtons = (dlg) => dlg.getByRole('button', { name: /放回这一组$/ })

export default async function (t, page, url) {
  await openParams(page, url)
  await pickParam(page, '擦料塔位置 X')

  /* ① 分组层：抽屉标题那支笔开框 */
  const dlg1 = await openSectionMembers(page)
  t.check('分组编辑框开着', await dlg1.isVisible())
  const adds0 = await addButtons(dlg1).count()
  const outs0 = await outButtons(dlg1).count()
  t.check('左列有候选参数', adds0 > 0, `${adds0} 条`)
  t.check('右列有本组成员', outs0 > 0, `${outs0} 条`)
  await t.shot(page, '分组成员框 · 两列')

  /* ② 搜索：候选收窄 */
  const search = dlg1.getByRole('searchbox', { name: '搜索参数' })
  t.check('参数多时有搜索框', (await search.count()) > 0)
  if ((await search.count()) > 0) {
    await search.fill('不存在的参数名')
    await page.waitForTimeout(150)
    t.check('搜索能收窄候选', (await addButtons(dlg1).count()) < adds0)
    await search.fill('')
    await page.waitForTimeout(150)
  }

  /* ③ 勾选加入 → 取消 → 移出 → 放回 */
  await addButtons(dlg1).first().click()
  await page.waitForTimeout(120)
  t.check('勾选后出现待归入', (await cancelButtons(dlg1).count()) === 1)
  await cancelButtons(dlg1).first().click()
  await page.waitForTimeout(120)
  t.check('取消待归入能回退', (await cancelButtons(dlg1).count()) === 0)

  if (outs0 > 0) {
    await outButtons(dlg1).first().click()
    await page.waitForTimeout(120)
    t.check('移出后出现放回与去处', (await backButtons(dlg1).count()) === 1)
    t.check('移出去处写在框里', (await dlg1.getByText(/已移出/).count()) > 0)
    await backButtons(dlg1).first().click()
    await page.waitForTimeout(120)
    t.check('放回能撤销移出', (await backButtons(dlg1).count()) === 0)
  }

  /* ④ 名字与成员一起改，确认一次 */
  const nameBox = dlg1.getByRole('textbox', { name: '名字' })
  await nameBox.fill('擦料方式（改）')
  await addButtons(dlg1).first().click()
  const confirm = dlg1.getByRole('button', { name: '确认', exact: true })
  t.check('有改动时确认可点', !(await confirm.isDisabled()))
  await confirm.click()
  await page.waitForTimeout(500)
  t.check('确认后框关掉', (await dlg1.count()) === 0)
  await t.shot(page, '分组成员 · 确认后')

  /* ⑤ 重开还是改过的 */
  const dlg2 = await openSectionMembers(page)
  t.check('重开名字还在', (await dlg2.getByRole('textbox', { name: '名字' }).inputValue()) === '擦料方式（改）')
  t.check('重开成员多了一条', (await outButtons(dlg2).count()) > outs0)
  await dlg2.getByRole('button', { name: '取消', exact: true }).click()
  await page.waitForTimeout(150)

  /* ⑥ 页签层：先选一个页签，再点栏末尾那支笔 */
  await page.getByRole('tab', { name: /擦料/ }).first().click()
  await page.waitForTimeout(150)
  const pen = page.getByRole('button', { name: '编辑当前页签' })
  t.check('选中页签后有编辑入口', (await pen.count()) > 0)
  const dlg3 = await openTabMembers(page)
  t.check('页签编辑框开着', await dlg3.isVisible())
  t.check('页签层有归到这一组', (await dlg3.getByRole('radiogroup', { name: '归到这一组' }).count()) > 0)
  t.check('目标分组至少一组', (await dlg3.getByRole('radio').count()) > 0)
  t.check('没改动时确认是灰的', await dlg3.getByRole('button', { name: '确认', exact: true }).isDisabled())
  await t.shot(page, '页签成员框 · 两列')
  await dlg3.getByRole('button', { name: '取消', exact: true }).click()
  await page.waitForTimeout(150)
  t.check('取消后页签框关掉', (await dlg3.count()) === 0)
}
