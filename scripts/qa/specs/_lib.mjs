/*
 * 几条 spec 共用的开场步骤（下划线开头 = 不是一条 spec，运行器会跳过）。
 *
 * 一律走**界面上能看见的东西**：导航按钮的文字、左栏那几行 `role=radio`、
 * 卡片里的参数名、右栏按钮的字。不碰 class 名 —— 那是实现细节，改一次样式就全红，
 * 而红的原因跟"用户能不能用"没关系。
 */

/** 打开工作台 → 参数台 → 选 A1 的标准版 */
export async function openParams(page, url) {
  await page.goto(url, { waitUntil: 'load' })
  await page.getByRole('button', { name: '参数台' }).click()
  await page.getByRole('radio', { name: /标准版/ }).first().click()
  /* 卡片要等 mock 那几发 IPC 回来 */
  await page.waitForSelector('text=擦料 · 擦料方式', { timeout: 15000 })
  await page.waitForTimeout(400)
}

/** 点某条参数（左键选中），右栏会跟着换 */
export async function pickParam(page, label) {
  await page.getByText(label, { exact: true }).first().click()
  await page.waitForTimeout(500)
}

/** 打开「编辑定义」，等框出来 */
export async function openDef(page) {
  await page.getByRole('button', { name: '编辑定义' }).first().click()
  const dialog = page.getByRole('dialog').filter({ hasText: '编辑定义' })
  await dialog.waitFor({ timeout: 10000 })
  return dialog
}

/**
 * 排序框本身。
 *
 * ★ 用**无障碍名**认，不用 `filter({ hasText })`：定义框里也写着「组内顺序」四个字，
 *   按文字筛会把两层框都匹配上（strict mode 直接报"resolved to 2 elements"）。
 *   排序框的 aria-label 是 `组内顺序 · <组名>`。
 */
export const sortDialog = (page) => page.getByRole('dialog', { name: /^组内顺序/ })

/** 排序框里那一列行的文字（按次序） */
export async function sortRows(page) {
  return sortDialog(page).locator('[role="listitem"]').allInnerTexts()
}

/** 打开第一张分组卡的抽屉，再点标题那支笔 = 分组成员编辑框 */
export async function openSectionMembers(page) {
  await page.locator('[title^="打开分组抽屉"]').first().click()
  const drawer = page.getByRole('dialog', { name: /^分组 / })
  await drawer.waitFor({ timeout: 5000 })
  await drawer.getByRole('button', { name: '编辑分组成员' }).click()
  const dlg = page.getByRole('dialog', { name: /^编辑分组/ })
  await dlg.waitFor({ timeout: 5000 })
  return dlg
}

/** 点当前页签栏那支笔 = 页签成员编辑框 */
export async function openTabMembers(page) {
  await page.getByRole('button', { name: '编辑当前页签' }).click()
  const dlg = page.getByRole('dialog', { name: /^编辑页签/ })
  await dlg.waitFor({ timeout: 5000 })
  return dlg
}
