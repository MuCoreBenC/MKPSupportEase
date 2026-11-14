/*
 * 工作台外壳：能不能进去、参数台能不能选机型、有没有控制台报错。
 *
 * 这是所有别的 spec 的地基 —— 它红了，先修它，别去看下一条为什么乱。
 */

import { openParams } from './_lib.mjs'

export const title = '外壳：进得去、参数台能选机型'

export default async function (t, page, url) {
  await page.goto(url, { waitUntil: 'load' })
  await page.waitForSelector('nav', { timeout: 15000 })

  const navs = ['机型与版本', '参数台', '套餐', '资产库', '标注台', '生成与发布', '设置']
  const missing = []
  for (const n of navs) {
    if ((await page.getByRole('button', { name: n, exact: true }).count()) === 0) missing.push(n)
  }
  t.check('七个导航项都在', missing.length === 0, missing.join('、'))
  await t.shot(page, '工作台首屏')

  await openParams(page, url)
  const cards = await page.$$eval('[class*="pCard"], [class*="card"]', (els) =>
    els.map((e) => (e.textContent ?? '').trim().slice(0, 20)).filter(Boolean),
  )
  t.check(
    '选上机型版本之后参数卡片出来了',
    cards.length > 0,
    `卡片 ${cards.length} 张`,
  )
  t.check(
    '页头写的是 A1 / 标准版',
    (await page.getByText(/A1\s*\/\s*标准版/).count()) > 0,
  )
  await t.shot(page, '参数台 · A1 标准版')
}
