#!/usr/bin/env node
/**
 * **发版脚本的自检**（2026-11-06，作者："我不希望每次都要发新版再去测"）。
 *
 * 为什么需要它：`scripts/release.mjs` 的绝大多数代码只有**真发版**那一刻才会跑，
 * 所以它的 bug 只能在发版当天暴露，一暴露就是二十分钟起步。2026-11-06 一天里踩到：
 *
 *   · 通道解析块放在 `suggested` 之后 → `Cannot access 'channel' before initialization`
 *   · `--dry-run` 从来没传给内核 → "演练"真的推了分支
 *   · 判据写死版本号 → 发版当天两条测试红在 tag 之前
 *
 * 这三条的共同点是：**在参数解析这一段就已经决定了**。所以这里只跑那一段
 * （`--args-only`：零写入、不联网、不碰 git），几秒钟，跟着每个 PR 跑。
 *
 * 用法：`node scripts/check-release-cli.mjs`（CI 的 web job 里跑）
 */

import { execFileSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

const HERE = fileURLToPath(new URL('.', import.meta.url))
const SCRIPT = `${HERE}release.mjs`

/** 跑一次 `release.mjs --args-only`，回 `{ code, out, err }`（不抛）。 */
function run(args) {
  try {
    const out = execFileSync('node', [SCRIPT, ...args, '--args-only'], {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    })
    return { code: 0, out: out.trim(), err: '' }
  } catch (e) {
    return { code: e.status ?? 1, out: String(e.stdout ?? '').trim(), err: String(e.stderr ?? '') }
  }
}

let failed = 0
function check(name, fn) {
  try {
    fn()
    console.log(`  ✓ ${name}`)
  } catch (e) {
    console.error(`  ✗ ${name}\n      ${e.message}`)
    failed += 1
  }
}

const eq = (got, want, what) => {
  if (got !== want) throw new Error(`${what}：期望 ${want}，实际 ${got}`)
}
const match = (text, re, what) => {
  if (!re.test(text)) throw new Error(`${what}：输出里没有 ${re}`)
}

/* ① 正常的一趟：预览号走预览通道 —— 解析出来的必须是它自己，不能被改写成 .1 */
check('预览号 + 预览通道 → 原样保留', () => {
  const r = run(['0.0.10-preview.2', '说明', '--channel=preview'])
  eq(r.code, 0, '退出码')
  const j = JSON.parse(r.out)
  eq(j.channel, 'preview', '通道')
  eq(j.version, '0.0.10-preview.2', '版本号')
})

/* ② `--preview` 短写法、`--channel preview` 空格写法都要认 */
check('三种通道写法等价', () => {
  for (const args of [
    ['0.0.10-preview.2', 'x', '--preview'],
    ['0.0.10-preview.2', 'x', '--channel', 'preview'],
    ['0.0.10-preview.2', 'x', '--channel=preview'],
  ]) {
    const r = run(args)
    eq(r.code, 0, `${args.join(' ')} 退出码`)
    eq(JSON.parse(r.out).channel, 'preview', `${args.join(' ')} 通道`)
  }
})

/* ③ 串台两种：都必须在**写盘之前**停（这是不可撤销的那一类） */
check('正式通道拒绝预览号', () => {
  const r = run(['0.0.10-preview.2', 'x'])
  if (r.code === 0) throw new Error('居然放行了')
  match(r.err + r.out, /预览号不能走正式通道/, '提示语')
})

check('预览通道拒绝正式号', () => {
  const r = run(['0.0.10', 'x', '--channel=preview'])
  if (r.code === 0) throw new Error('居然放行了')
  match(r.err + r.out, /预览通道要发预览号/, '提示语')
})

/* ④ 认不出的通道不许兜底成正式版 */
check('认不出的通道直接报错', () => {
  const r = run(['0.0.10', 'x', '--channel=beta'])
  if (r.code === 0) throw new Error('居然放行了')
  match(r.err + r.out, /认不出的更新通道/, '提示语')
})

/* ⑤ **没有 `-dev` 了**（2026-11-06 作者：「不要再出现 DEV」）：
   形状不对就当场停，而不是悄悄归一成什么东西 */
check('拒绝 -dev 形状的号', () => {
  const r = run(['0.0.10-dev', 'x'])
  if (r.code === 0) throw new Error('居然放行了')
  match(r.err + r.out, /没有 -dev 了/, '提示语')
})

/* ⑥ 建议值：预览通道给 `-preview.N`，正式通道给裸号 */
check('建议版本号按通道给', () => {
  const p = JSON.parse(run(['0.0.10-preview.2', 'x', '--preview']).out)
  if (!/-preview\.\d+$/.test(p.suggested)) throw new Error(`预览建议值不对：${p.suggested}`)
  const s = JSON.parse(run(['0.0.12', 'x']).out)
  if (/-/.test(s.suggested)) throw new Error(`正式建议值不该带后缀：${s.suggested}`)
})

/* ⑦ **大小写归一**（作者习惯写 `Preview.1`，历史 tag 是小写）：
   同一个号只能有**一个** tag，否则 git 里两个都"合法"、谁也说不清谁是权威 */
check('Preview 大写归一成小写', () => {
  const r = run(['0.0.12-Preview.3', 'x', '--channel=preview'])
  eq(r.code, 0, '退出码')
  eq(JSON.parse(r.out).version, '0.0.12-preview.3', '版本号')
})

/* ⑧ 建议值要跟着**当前工作区的预览号**走，不是永远给 `.1` */
check('预览建议值 = 当前预览号 + 1', () => {
  const r = run(['0.0.12-preview.9', 'x', '--channel=preview'])
  const j = JSON.parse(r.out)
  if (!/-preview\.(\d+)$/.test(j.suggested)) throw new Error(`建议值形状不对：${j.suggested}`)
  const n = Number(/-preview\.(\d+)$/.exec(j.suggested)[1])
  if (n <= 1) throw new Error(`建议值该往后再给一个号：${j.suggested}`)
})

if (failed > 0) {
  console.error(`\n发版脚本自检：${failed} 条没过`)
  process.exit(1)
}
console.log('\n✓ 发版脚本自检：参数解析 / 通道 / 版本身份都对')
