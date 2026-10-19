/*
 * 探针：客户端「修改参数」—— **改了不保存、关掉之后草稿还在不在**（2026-10-19）。
 *
 * 守的是作者 2026-10-19 报的那三件事（都是客户端 `src/app/params/` 这条链）：
 *
 *   ① **外面那一行要亮橙色「未保存」圆点**（`PresetTable` 的 `unsavedDot`）：
 *      原来 `draftState` 那条广播**参数链一次都没发**（`git grep draftMutated` 只有
 *      预设页正文链那两处），所以圆点只有等整表重读才可能亮 —— 作者："那个橙色的点也不显示了"。
 *   ② **"只打开过、一个字没改"不该亮**：`begin_preset_edit` 一打开就写出草稿文件，
 *      摘要是 `Some` —— 判据必须看 `dirty`（"真改过才算"）。
 *   ③ **带着没保存的草稿重开**：应当**直接是编辑模式**，且**刚才那处改动还看得见**
 *      （作者："我没保存的时候关闭它应该是记住着……它一样还是保留着那个编辑模式的"）。
 *
 * 用法（探针跑的是**静态预览**，客户端在浏览器里自动走假后端 `src/api/mock.ts`）：
 *   npm run build
 *   npx vite preview --port 4173 --strictPort
 *   node scripts/probes/params-draft-dot.mjs [url]
 */
import { chromium } from 'playwright-core'

const url = process.argv[2]?.startsWith('http') ? process.argv[2] : 'http://localhost:4173/'

const problems = []
const check = (what, ok, detail = '') => {
  console.log(`${ok ? 'PASS' : 'FAIL'} ${what}${detail ? `（${detail}）` : ''}`)
  if (!ok) problems.push(`${what}${detail ? ` —— ${detail}` : ''}`)
}

const browser = await chromium.launch({ channel: 'msedge' })
const page = await browser.newPage({ viewport: { width: 1560, height: 900 } })
page.on('pageerror', (e) => problems.push(`pageerror: ${e.message}`))

await page.goto(url, { waitUntil: 'load' })
await page.waitForSelector('header nav', { timeout: 10000 })

/* 进「预设」页 → 切到「本地」那一档 */
await page.getByRole('button', { name: '预设', exact: true }).first().click()
await page.waitForSelector('main tbody tr[data-rowkey]', { timeout: 10000 })
await page.waitForTimeout(400)
await page.evaluate(() => {
  const l = [...document.querySelectorAll('main [role="radiogroup"] label')].find(
    (x) => (x.textContent ?? '').trim() === '本地',
  )
  l?.click()
})
await page.waitForTimeout(400)

const row = page.locator('main tbody tr[data-rowkey]').filter({ hasText: 'A1 涂胶.toml' }).first()
check('本地表里找得到 A1 涂胶.toml', (await row.count()) > 0)
check('起步：这一行没有「未保存」圆点', (await row.locator('[class*="unsavedDot"]').count()) === 0)

/** 右键这一行 → 点「修改参数」 → 等弹窗出来 */
const openParams = async () => {
  await row.click({ button: 'right' })
  await page.waitForTimeout(300)
  await page.getByRole('menuitem', { name: '修改参数', exact: true }).click()
  await page.waitForSelector('[role="dialog"] [role="tablist"][aria-label="参数分类"]', {
    timeout: 10000,
  })
  await page.waitForTimeout(300)
}

/** 弹窗底栏那几颗按钮的名字（判两态：只读「进入编辑模式」 vs 编辑态「放弃并退出」） */
const footNames = () =>
  page.evaluate(() => {
    const d = document.querySelector('[role="dialog"]')
    return [...(d?.querySelectorAll('footer button') ?? [])].map((b) => (b.textContent ?? '').trim())
  })

/** 点弹窗底栏一颗名字正好是这些字的按钮 */
const footClick = (text) =>
  page.evaluate((want) => {
    const d = document.querySelector('[role="dialog"]')
    const b = [...(d?.querySelectorAll('footer button') ?? [])].find(
      (x) => x.textContent.trim() === want,
    )
    b?.click()
    return b !== undefined
  }, text)

/* ---------- ① 只打开、一个字不改 ---------- */
await openParams()
check('打开先只读（底栏只有「进入编辑模式」）', (await footNames()).includes('进入编辑模式'))
await page.waitForTimeout(500)
check(
  '只打开不改 → 圆点不亮（判据看 dirty，不看"有没有草稿"）',
  (await row.locator('[class*="unsavedDot"]').count()) === 0,
)

/* ---------- ② 进编辑模式，改一个值 ---------- */
await footClick('进入编辑模式')
await page.waitForTimeout(400)

/*
 * 挑一个**假后端真的写得进去**的数字参数。
 *
 * 演示里那份 `presets-local/A1 涂胶.toml` 的正文只有 `[toolhead] offset_y / speed_limit`
 * 两个键（见 `src/api/mock.ts` 的夹具）—— 点别的（如 `toolhead.offset.x`）时假后端的
 * `patchPresetDraft` 会如实报「草稿的 [toolhead] 里没有 offset_x」，草稿压根没落盘，
 * 后面那些判据就全被它带偏了。优先挑夹具里有的那个，挑不到再退回第一个可改的数字格。
 */
const known = page.locator('[role="dialog"] [data-key="toolhead.offset.y"]')
const cell =
  (await known.count()) > 0
    ? known.first()
    : page
        .locator('[role="dialog"] [data-key]')
        .filter({ has: page.locator('button[aria-label$="增加"]') })
        .filter({ has: page.locator('input:not([disabled])') })
        .first()
await cell.waitFor({ timeout: 8000 })
const key = await cell.getAttribute('data-key')
const input = cell.locator('input').first()
const before = await input.inputValue()
await cell.locator('button[aria-label$="增加"]').first().click()
await page.waitForTimeout(600)
const after = await input.inputValue()
check(`改了 ${key} 一个值`, after !== before, `${before} → ${after}`)
/* 草稿没写进盘上时页面会挂一行"草稿自动落盘失败了" —— 把它摆出来，免得后面的判据被它带偏 */
const draftErrLine = await page.evaluate(() => {
  const d = document.querySelector('[role="dialog"]')
  const t = (d?.innerText ?? '').replace(/\s+/g, ' ')
  const m = t.match(/[^。]*?(草稿|失败|改不了|没得存)[^。]*/g)
  return m === null ? '' : m.join(' | ')
})
if (draftErrLine !== '') console.log(`  ⚠ 弹窗里那句：${draftErrLine}`)

/* ---------- ③ 关掉（不保存，Esc = 只关不丢） ---------- */
await page.keyboard.press('Escape')
await page.waitForTimeout(600)
check('弹窗关掉了', (await page.locator('[role="dialog"]').count()) === 0)

/* ★ 主判据：外面那一行该亮起橙色圆点（轮询 3s：广播是 150ms 尾随，别用死等） */
let dotAfterClose = 0
for (let i = 0; i < 30; i++) {
  dotAfterClose = await page.locator('main tbody tr[data-rowkey] [class*="unsavedDot"]').count()
  if (dotAfterClose > 0) break
  await page.waitForTimeout(100)
}
check('关掉（不保存）之后，表里有行亮出橙色「未保存」圆点', dotAfterClose > 0, `全表 ${dotAfterClose} 枚`)

/* ---------- ④ 重开：直接编辑模式 + 那处改动还在 ---------- */
await openParams()
const names2 = await footNames()
check('重开直接是编辑模式（底栏有「放弃并退出」）', names2.includes('放弃并退出'), names2.join('/'))
const input2 = page.locator(`[role="dialog"] [data-key="${key}"] input`).first()
const shown = await input2.inputValue()
check(`重开仍看得见那处改动（${key} = ${shown}）`, shown === after, `重开显示 ${shown}，关之前 ${after}`)

/* ---------- ⑤ 保存 → 圆点当场灭（保存要过一道「确认保存？」） ---------- */
await footClick('保存')
await page.waitForTimeout(400)
/* 确认那一层是**页内**的 sheet（`role="dialog"` + `aria-label="保存确认"`） */
await page
  .locator('[role="dialog"][aria-label="保存确认"] button')
  .filter({ hasText: '确认保存' })
  .first()
  .click()
await page.waitForTimeout(1200)
const dotAfterSave = await row.locator('[class*="unsavedDot"]').count()
check('保存之后圆点灭', dotAfterSave === 0)

await browser.close()
console.log(problems.length === 0 ? '\n全部通过' : `\n${problems.length} 条没过：\n${problems.join('\n')}`)
process.exit(problems.length === 0 ? 0 : 1)
