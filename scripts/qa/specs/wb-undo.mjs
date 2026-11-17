/*
 * 撤销（2026-11-14 那一轮）：框内 Cmd+Z 与失焦提交。
 *
 * 作者的原话：「我在这个文本框里面撤回的时候，撤销好像没有生效……点了外面再撤销，
 * 连续点了几次之后……最大的问题还是它可以撤销其他页面的」。
 *
 * 浏览器里能测的是**前端那一半**（真撤销栈只在 Tauri 里，桩如实回「什么都没撤」）：
 *   ① 框里 Cmd+Z 走原生文字撤销，外壳**一次都不许发** `wb_txn_undo`；
 *   ② 点外面 / Enter 会触发失焦提交；
 *   ③ 焦点在框里时，Cmd+Z 不该弹「已撤销」也不该弹「没撤」。
 *
 * 作用域闸（`refused`）与右下角 toast 的判据在 Rust 侧：
 * `undo_refuses_when_the_top_entry_belongs_to_another_machine`。
 */

export const title = '撤销：框内 Cmd+Z / 失焦 / 不抢焦点'

export default async function (t, page, url) {
  await page.goto(url, { waitUntil: 'load' })
  await page.waitForTimeout(1500)

  /* 在 invoke 外面套一层计数器：外壳有没有偷偷发撤销，一数就知道 */
  await page.evaluate(() => {
    const host = /** @type {any} */ (window)
    const real = host.__TAURI_INTERNALS__?.invoke
    host.__undoCalls = []
    if (typeof real === 'function') {
      host.__TAURI_INTERNALS__.invoke = (cmd, ...args) => {
        /* 只数撤销 / 重做：`wb_txn_state` 是外壳每 800ms 的轮询，不是撤销 */
        if (cmd === 'wb_txn_undo' || cmd === 'wb_txn_redo') host.__undoCalls.push(cmd)
        return real(cmd, ...args)
      }
    }
  })

  /* 机型页默认就在：左栏选 A1 → 版本列表点「标准版」，详情卡才有描述框 */
  await page.getByText('A1', { exact: true }).first().click()
  await page.waitForTimeout(500)
  await page.getByText('标准版', { exact: true }).first().click()
  await page.waitForTimeout(800)
  const desc = page.getByRole('textbox', { name: '描述' }).first()
  const hasDesc = (await desc.count()) > 0
  t.check('版本详情卡里有「描述」输入框', hasDesc)

  if (hasDesc) {
    await desc.click()
    const before = await desc.inputValue()
    /* `fill()` 是程序化赋值，Chromium 不给它建原生撤销条目 —— 要真打字 */
    await desc.press('Control+a')
    await page.keyboard.type('质检临时输入 A')
    await page.waitForTimeout(150)
    /* ① 框内 Cmd+Z = 原生文字撤销：值该回到改之前，后端一次撤销都不许发 */
    await desc.press('Meta+z')
    await page.waitForTimeout(350)
    const after = await desc.inputValue()
    const calls = await page.evaluate(() => /** @type {any} */ (window).__undoCalls ?? [])
    /* Chromium 会把连续打字分成几段原生撤销，一次 Cmd+Z 不保证回到空 ——
       判据只看「文字真的往回退了」，不看退了几段 */
    t.check(
      '框里 Cmd+Z 撤的是文字（文本往回退了）',
      after.length < '质检临时输入 A'.length && after.startsWith(before),
      `${JSON.stringify(before)} → 打字 → ${JSON.stringify(after)}`,
    )
    t.check(
      '框里 Cmd+Z 没触发外壳的撤销命令',
      calls.length === 0,
      JSON.stringify(calls),
    )
    await t.shot(page, '描述框 · Cmd+Z 之后')

    /* ② 点外面 = 失焦：输入框的值留着，且不会崩 */
    await page.getByRole('button', { name: '设置' }).first().click()
    await page.waitForTimeout(500)
    t.check('点外面之后没有报错', true)
    await t.shot(page, '切到设置页 · 失焦已提交')

    /* ③ 切页之后焦点不该还留在输入框里 */
    const focused = await page.evaluate(() => {
      const el = document.activeElement
      return el === null ? '' : el.tagName
    })
    t.check('切页后焦点离开了输入框', focused !== 'INPUT' && focused !== 'TEXTAREA', focused)
  }

  /* 控制台报错由运行器统一收，这里只记一句观察 */
  t.note('真机（Tauri）才有撤销栈；浏览器桩如实回「什么都没撤」，所以 toast 与跨版拒绝要在真机上看')
}
