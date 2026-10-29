#!/usr/bin/env node
/**
 * **产物边界闸** —— 别让开发时生成的东西混进正式交付。
 *
 * ```bash
 * npm run check:artifacts              # 与 origin/main 比
 * npm run check:artifacts -- --base X  # 与任意基准比
 * npm run check:artifacts -- --staged  # 只看暂存区（pre-commit 用）
 * ```
 *
 * # 它防的是哪一类事故（作者 2026-10-28）
 *
 * > 「我改个预设测一下，AI 忘了还原就提交了，最后用户撞上一个根本不算 bug 的 bug。」
 *
 * 这个仓库里有**三层会被提交进库的生成物**：
 *
 * ```text
 * presets/…（源：配方 / registry / machines / 资产表）
 *     │ gen-presets
 *     ▼
 * crates/preset/assets/presets/*.toml   ← include_str! 进内核，随包预设在客户端解析的就是它
 *     │ 工作台/CLI 生成
 *     ▼
 * presets/delivery/**（catalog / manifest / 9 份产物）  ← 客户端 Bootstrap 读的就是它
 *     │ gen-catalog
 *     ▼
 * src-tauri/src/runtime/catalog.generated.json          ← 随包目录
 * ```
 *
 * 三层都是"机器写的"，但它们**住在 git 里**（必须住：客户端要按 URL 直接取）。
 * 于是实测中只有两道防线：人的记性，和 `.gitignore` 注释里那句
 * 「`presets/delivery/` 只由发布命令动它」—— 而后者**从来没有机器执行过**。
 *
 * 这个脚本执行的就是那两句话。
 *
 * # 三条判据（都是零误报的，能用才敢天天跑）
 *
 * | 判据 | 红了说明 |
 * |---|---|
 * | R1 交付目录变了，账本没变 | 这批产物**不是发布产生的** —— 正式发布一定会写 `publish-serial.json` |
 * | R2 内核产物变了，源没变 | 产物无源 —— 几乎必然是测试时重生成留下的 |
 * | R3 本该被忽略的路径进来了 | `dist/`、沙箱、临时脚本…… 有人 `-f` 强加了 |
 *
 * # 它**抓不到**什么（说在前面，别指望它）
 *
 * - **源改了但忘了还原**：那是 diff 里一处正常的改动，机器分不出"有意"与"忘还原"。
 *   那件事只有人看一眼，或者靠 `presets diff` 把改动摆出来。
 * - **产物与源内容对不对**：那要真跑一遍生成器，由 `presets verify` / `presets diff` 管
 *   （发布事务里已经跑过；这里刻意不做第二套，免得两个实现慢慢漂开）。
 *
 * # 逃生口
 *
 * `ALLOW_PRESET_ARTIFACTS=1`：确实要"只提交产物"时用（会打印一行警告，写进 CI 日志）。
 * 逃生口一旦变成日常工具，这道闸就白加了 —— 与 `_lib.sh` 里那几个开关同一条纪律。
 */

import { execFileSync } from 'node:child_process'

/* ---------- 路径分类（唯一一批定义，改这里就够） ---------- */

const DELIVERY = 'presets/delivery'
const LEDGER = `${DELIVERY}/publish-serial.json`
const KERNEL_ARTIFACTS = ['crates/preset/assets/presets/', 'src-tauri/src/runtime/catalog.generated.json']

/** 本该被 `.gitignore` 挡住、绝不能出现在提交里的路径 */
const FORBIDDEN = [
  { prefix: 'dist/', why: 'Vite 构建产物（`.gitignore` 已挡，别 -f 强加）' },
  { prefix: 'dist-ssr/', why: '同上' },
  { prefix: 'client-assets/', why: '随包资产交付结果，构建时重建' },
  { prefix: 'workbench/.sandbox/', why: '测试模式那棵树（本机状态）' },
  { prefix: 'workbench/.draft/', why: '工作台草稿（本机状态）' },
  { prefix: 'tmp-shots/', why: '探针截图（一次走查的现场）' },
  { prefix: 'tmp-port/', why: '一次性脚本的临时落点' },
  { prefix: 'node_modules/', why: '依赖' },
  { prefix: 'target/', why: '构建产物' },
  { prefix: 'target-workbench/', why: '工作台构建产物' },
  { prefix: '.playwright-cli/', why: '浏览器验证会话快照' },
]

const isForbidden = (p) =>
  FORBIDDEN.find((f) => p === f.prefix.replace(/\/$/, '') || p.startsWith(f.prefix)) ??
  (/\.(orig|rej|bak|swp)$/.test(p) || /(^|\/)\.DS_Store$/.test(p)
    ? { prefix: p, why: '编辑器 / 系统的残留文件' }
    : null)

const isDelivery = (p) => p === DELIVERY || p.startsWith(`${DELIVERY}/`)
const isLedger = (p) => p === LEDGER
const isKernelArtifact = (p) => KERNEL_ARTIFACTS.some((k) => p === k || p.startsWith(k))
/** 权威源 = `presets/` 下除交付目录之外的一切（配方 / registry / machines / 资产表 / 预设施加源） */
const isSource = (p) => p.startsWith('presets/') && !isDelivery(p)

/* ---------- git ---------- */

const git = (args) => {
  try {
    return execFileSync('git', args, { encoding: 'utf8', stdio: 'pipe' })
  } catch (e) {
    return `__ERR__${String(e.stderr ?? e.message)}`
  }
}

/** 变更清单（相对仓库根的路径，已去重、已排序） */
function changedFiles({ staged, base }) {
  const args = staged
    ? ['diff', '--cached', '--name-only', '--diff-filter=ACDMRTUXB', '-z']
    : ['diff', '--name-only', '--diff-filter=ACDMRTUXB', '-z', `${base}...HEAD`]
  const out = git(args)
  if (typeof out === 'string' && out.startsWith('__ERR__')) {
    const msg = out.slice(7).trim()
    return { error: msg }
  }
  return {
    files: [...new Set(out.split('\0').filter((s) => s !== ''))].sort(),
  }
}

/* ---------- 主判定 ---------- */

const argv = process.argv.slice(2)
const staged = argv.includes('--staged')
const baseArg = argv.indexOf('--base')
const base = baseArg >= 0 ? (argv[baseArg + 1] ?? 'origin/main') : 'origin/main'
const allow = process.env.ALLOW_PRESET_ARTIFACTS === '1'

if (staged && baseArg >= 0) {
  console.error('✗ --staged 与 --base 不能同时用（暂存区没有"基准"这个概念）')
  process.exit(2)
}

const scope = staged ? '暂存区' : `${base}...HEAD`
const got = changedFiles({ staged, base })
if (got.error) {
  /* 基准取不到不是"通过"：宁可红一次让人看清，也别在关键时刻静默放行 */
  console.error(`\n  产物边界：读不出变更清单（${scope}）`)
  console.error(`  ${got.error.split('\n')[0]}`)
  console.error(`\n  基准 \`${base}\` 在本地可能不存在。先：git fetch origin main\n`)
  process.exit(1)
}

const files = got.files
const delivery = files.filter(isDelivery)
const ledger = files.filter(isLedger)
const kernel = files.filter(isKernelArtifact)
const source = files.filter(isSource)
const forbidden = files.map((p) => [p, isForbidden(p)]).filter(([, f]) => f !== null)

const problems = []

/* R1 —— 交付目录只由发布命令动（正式发布必然写账本，所以"账本没变"= 不是发布） */
if (delivery.length > 0 && ledger.length === 0) {
  problems.push({
    id: 'R1',
    title: '交付目录变了，但发布账本没变 —— 这批产物不是发布产生的',
    files: delivery,
    why:
      '`presets/delivery/` 的规则是「只由发布命令动它」（`.gitignore` 里写着，但一直没人执行）。\n' +
      '    正式发布一定会写 `presets/delivery/publish-serial.json`；它没变，说明这些字节\n' +
      '    是「生成」或测试留下的，而不是「发布」出来的。',
    fix:
      '两条路，选一条：\n' +
      '      · 这是测试残留 ⇒ 还原：`git restore --staged --worktree presets/delivery`\n' +
      '      · 这确实要发出去 ⇒ 走正式发布：`npm run presets -- publish`（分配序号并写账本，\n' +
      '        它自己会把目录与账本放进同一笔提交）',
  })
}

/* R2 —— 内核产物必须与源同批（include_str! 进客户端的那一层，改错了没人看得出来） */
if (kernel.length > 0 && source.length === 0) {
  problems.push({
    id: 'R2',
    title: '内核产物变了，但预设源没变 —— 产物无源',
    files: kernel,
    why:
      '这两个位置是「配方 → 产物」那一层的输出：`crates/preset/assets/presets/*.toml`\n' +
      '    被 `include_str!` 编进客户端（随包预设解析的就是它），`catalog.generated.json`\n' +
      '    是随包目录。只改产物不改源，等于悄悄换掉所有用户的随包预设。',
    fix:
      '两条路，选一条：\n' +
      '      · 测试时重生成留下的 ⇒ `git restore --staged --worktree` 那两个路径\n' +
      '      · 真的改了配方 ⇒ 把 `presets/` 下的源一起提交（`npm run preset-source:make` 之后）',
  })
}

/* R3 —— 该被忽略的路径不该出现在变更里 */
for (const [p, f] of forbidden) {
  problems.push({
    id: 'R3',
    title: '本该被忽略的路径进了变更',
    files: [p],
    why: `${f.why}。它被 ` + '`.gitignore` 挡着，出现在这里说明是 `git add -f` 强加的。',
    fix: `git restore --staged '${p}'（或 \`git rm --cached\`）`,
  })
}

/* ---------- 报告 ---------- */

const total = files.length
if (problems.length === 0) {
  console.log(
    `✓ 产物边界干净（${scope}：${total} 个变更文件；交付 ${delivery.length} / 内核产物 ${kernel.length} / 源 ${source.length}）`,
  )
  process.exit(0)
}

console.error(`\n  产物边界：${problems.length} 处不对（${scope}，${total} 个变更文件）\n`)
for (const p of problems) {
  console.error(`  ✗ [${p.id}] ${p.title}`)
  const shown = p.files.slice(0, 8)
  for (const f of shown) console.error(`      ${f}`)
  if (p.files.length > shown.length) console.error(`      …还有 ${p.files.length - shown.length} 个`)
  console.error(`    ${p.why}`)
  console.error(`    怎么办：${p.fix}\n`)
}

if (allow) {
  console.error('  ⚠ ALLOW_PRESET_ARTIFACTS=1 —— 这次放行（会写进 CI 日志）。\n')
  process.exit(0)
}

console.error('  确实要这么提交（会留痕）：ALLOW_PRESET_ARTIFACTS=1 git push … / npm run …\n')
process.exit(1)
