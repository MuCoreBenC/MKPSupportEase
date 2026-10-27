/*
 * 探针：参数台连点步进器 —— **跟手 + 一次落库 + 显示不回跳**（2026-10-19）。
 *
 * 守的是作者 2026-10-19 实测到的那条病：改一个参数时"很慢很卡、数字不跟手"。
 * 两轮修，两次都在这条探针上加判据：
 *
 *   第一轮（增量之四十七）：一次点击引爆一串同步 IPC（`apply_draft` + `registry` +
 *     整页 `desk` + 重 preflight）⇒ 数字落后于点击。修法见 `src/workbench/editQueue.ts`。
 *   第二轮（增量之四十八，本条新增）★★ **显示在"在飞那一趟"里掉回旧值**：
 *     队列把批次**发出前**就删了 ⇒ IPC 往返期间乐观覆盖层没了 ⇒ 屏上数字先跳回旧值、
 *     响应回来才跳到正确值。作者原话：「最后跳到的是正确的值，只不过过程不好看、
 *     不跟手 —— 从 0.1 点一下该变 0.05，可它先像是 0.15 才变成 0.05」。
 *
 * ## 判据
 *
 *   ① **跟手**：连点 10 下，停手那一刻就该是「原值 + 10」，落库之后仍 +10；
 *   ② **不倒退**（本轮新增）：全程每 20ms 采一次框里的值，**必须单调不倒退** ——
 *      第二轮那条病的唯一形状就是"中间倒退一次"；
 *   ③ **一次手势 = 一次落库**：这 10 下之间 `wb_apply_draft` 只发 **1** 条；
 *   ④ **不再整页重取**：`wb_desk` / `wb_registry` / `wb_preflight` 一条都不发；
 *   ⑤ **橙色「改之前的值」**：行上出现 `↩ <改前那个数>`，点它回到那个数、chip 自己消失。
 *
 * ## 为什么要给 IPC 人为加延迟
 *
 * 假后端的 `wb_apply_draft` 几乎瞬时返回（`dev/mockBackend.ts` 同步 resolve），
 * **"在飞"那个窗口宽度≈0** —— 第一版探针就是这么漏掉第二轮的病的（它只在
 * "停手那一刻"与"400ms 之后"两处采样，两点正好都落在正确的一侧）。
 * 这里包一层 invoke，把 `wb_apply_draft` 人为拖 300ms（真机往返的量级），
 * 病才照得出来。
 *
 * 用法（先让工作台的 vite dev 在 5322 上跑着；它会自动走浏览器假后端）：
 *   npm run dev:workbench            # 另一个终端
 *   node scripts/probes/params-lag.mjs
 *
 * 与 `params-settings.mjs` 同一副骨架：`chromium.launch({ channel: 'msedge' })`。
 */
import { chromium } from 'playwright-core'

const URL = 'http://localhost:5322/workbench.html?mock=1'
/** 给写入口人为加的往返延迟（ms）—— 真机量级，用来把"在飞窗口"照出来 */
const APPLY_DELAY = 300

const browser = await chromium.launch({ channel: 'msedge' })
const page = await browser.newPage({ viewport: { width: 1560, height: 900 } })
const problems = []
page.on('pageerror', (e) => problems.push(`pageerror: ${e.message}`))

await page.goto(URL, { waitUntil: 'networkidle' })

/* 进参数台，选「机型基底」（第一行 radio） */
await page.getByRole('button', { name: '参数台' }).first().click()
await page.getByRole('radio').first().click()

/* 第一行「有可用数字步进器」的：**必须挑可改的** —— 改不动的那几行压根不画箭头 */
const row = page
  .locator('div[class*="pRow"]')
  .filter({ has: page.locator('button[aria-label$="增加"]') })
  .filter({ has: page.locator('input:not([disabled])') })
  .first()
await row.waitFor({ timeout: 8000 })

const arrow = row.locator('button[aria-label$="增加"]').first()
const input = row.locator('input').first()
const label = await row.locator('[class*="pRowText"]').first().innerText()

/* 数 IPC + 给写入口加延迟。装过一次就别再包（否则计数裹了三层） */
await page.evaluate((delay) => {
  const w = window
  if (w.__wrapped) return
  const orig = w.__TAURI_INTERNALS__.invoke
  w.__calls = {}
  w.__TAURI_INTERNALS__.invoke = (cmd, args) => {
    w.__calls[cmd] = (w.__calls[cmd] ?? 0) + 1
    if (cmd === 'wb_apply_draft') {
      return new Promise((res, rej) => {
        setTimeout(() => {
          orig(cmd, args).then(res, rej)
        }, delay)
      })
    }
    return orig(cmd, args)
  }
  w.__wrapped = true
}, APPLY_DELAY)

/* 给这个框打个记号，好让页内的采样器抓得住它 */
await input.evaluate((el) => {
  el.setAttribute('data-probe', '1')
})

const before = await input.inputValue()

/* 全程采样：每 20ms 读一眼框里的值 —— 病的样子就是"中间倒退一次" */
await page.evaluate(() => {
  window.__calls = {}
  window.__trace = [Number(document.querySelector('input[data-probe="1"]')?.value)]
  window.__probe = setInterval(() => {
    const el = document.querySelector('input[data-probe="1"]')
    if (el === null) return
    const n = Number(el.value)
    window.__trace.push(Number.isFinite(n) ? n : -1)
  }, 20)
})

/* 连点 10 下，每下 50ms；**等 10 下全部发完**才往下走 */
await arrow.evaluate(async (el) => {
  const fire = (t) =>
    el.dispatchEvent(new PointerEvent(t, { bubbles: true, cancelable: true, pointerId: 1 }))
  for (let i = 0; i < 10; i++) {
    fire('pointerdown')
    fire('pointerup')
    await new Promise((r) => setTimeout(r, 50))
  }
})

/* 手停下的这一刻（队列还没合批发出去）—— 跟手 = 这里就已经 +10 */
const during = await input.inputValue()

/* 等落库那一趟走完：120ms 尾随 + APPLY_DELAY 往返 + 余量 */
await page.waitForTimeout(APPLY_DELAY + 600)
const after = await input.inputValue()
const calls = await page.evaluate(() => window.__calls)
/* 再多看 1.5 秒：**落库之后显示值一次都不该变**（本轮那条"闪一帧"就死在这儿） */
await page.waitForTimeout(1500)
const trace = await page.evaluate(() => {
  clearInterval(window.__probe)
  return window.__trace
})

await page.waitForTimeout(300)
const chip = await row.locator('button[class*="pRevert"]').count()
const chipText = chip > 0 ? await row.locator('button[class*="pRevert"]').first().innerText() : '（没有）'

/* 点那枚橙色 chip：应当回到改之前的数，且 chip 自己消失 */
let afterRevert = null
let chipAfter = null
if (chip > 0) {
  await row.locator('button[class*="pRevert"]').first().click()
  await page.waitForTimeout(APPLY_DELAY + 400)
  afterRevert = await input.inputValue()
  chipAfter = await row.locator('button[class*="pRevert"]').count()
}

/* ---------- 判据 ---------- */
const okFollow = Number(during) === Number(before) + 10 && Number(after) === Number(before) + 10
/* 单调不倒退：后一个采样永远不小于前一个（本轮新增的那条病就在这儿现形） */
let backwardAt = -1
for (let i = 1; i < trace.length; i++) {
  if (trace[i] < trace[i - 1]) {
    backwardAt = i
    break
  }
}
const okTrace = backwardAt < 0 && trace[0] === Number(before) && trace[trace.length - 1] === Number(before) + 10
/* 到了最终值之后**不许再动**：闪一帧再回来（或落库后再跳）都算 FAIL */
const reachedAt = trace.indexOf(Number(before) + 10)
const okStable = reachedAt >= 0 && trace.slice(reachedAt).every((v) => v === Number(before) + 10)
const okApply = (calls.wb_apply_draft ?? 0) === 1
const okDesk = (calls.wb_desk ?? 0) === 0
const okReg = (calls.wb_registry ?? 0) === 0
const okPre = (calls.wb_preflight ?? 0) === 0
const okChip = chip > 0 && chipText.includes(String(before))
const okRevert = afterRevert === before && chipAfter === 0

console.log(
  JSON.stringify(
    {
      label,
      before,
      during,
      after,
      calls,
      trace: trace.length > 40 ? `${trace.slice(0, 40).join(',')} …` : trace.join(','),
      backwardAt,
      chip,
      chipText,
      afterRevert,
      chipAfter,
      problems,
    },
    null,
    2,
  ),
)
console.log(
  okFollow
    ? 'PASS 跟手（+10）'
    : `FAIL 跟手：${before} → 停手时 ${during} / 落库后 ${after}（应为 ${Number(before) + 10}）`,
  '\n' + (okTrace ? 'PASS 显示不回跳（全程单调）' : `FAIL 显示回跳：采样第 ${backwardAt} 步倒退（${trace[backwardAt - 1]} → ${trace[backwardAt]}）`),
  '\n' + (okStable ? 'PASS 到最终值后不再动（落库前后都不闪）' : `FAIL 到最终值后又变过（第 ${reachedAt} 步起到末尾）`),
  '\n' + (okApply ? 'PASS apply_draft=1' : `FAIL apply_draft=${calls.wb_apply_draft}`),
  '\n' + (okDesk ? 'PASS desk=0' : `FAIL desk=${calls.wb_desk}`),
  '\n' + (okReg ? 'PASS registry=0' : `FAIL registry=${calls.wb_registry}`),
  '\n' + (okPre ? 'PASS preflight=0' : `FAIL preflight=${calls.wb_preflight}`),
  '\n' + (okChip ? `PASS 橙色 chip（${chipText}）` : `FAIL 橙色 chip：${chipText}`),
  '\n' + (okRevert ? `PASS chip 回退（→ ${afterRevert}，chip 消失）` : `FAIL chip 回退：${afterRevert} / chip=${chipAfter}`),
  problems.length ? `\nFAIL pageerror: ${problems.join(' | ')}` : '',
)

await browser.close()
process.exit(
  okFollow && okTrace && okStable && okApply && okDesk && okReg && okPre && okChip && okRevert && problems.length === 0
    ? 0
    : 1,
)
