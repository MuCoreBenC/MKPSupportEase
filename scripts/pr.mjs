#!/usr/bin/env node
/**
 * 建 PR / 合并 PR 的唯一入口。
 *
 * # 为什么要有它，而不是直接 `gh pr create`
 *
 * 2026-11-12 踩到的三件事，靠"记得"是挡不住的：
 *
 * 1. **分支上早就开着一个 PR**（#90 开了 56 个提交），`gh pr create` 只会说
 *    "already exists"，于是沿用了一个标题是上一轮内容的 PR。squash 之后
 *    main 上那一条提交的标题就是 PR 标题 —— `git log` 从此骗人。
 * 2. **进下一个预览号不该建 PR**（作者口径：就在分支上），但那一步照做了，
 *    白等一轮 CI，还多出一个要收的 PR。
 * 3. 建完 PR 之后又推了两次修 CI 的东西，标题再也没回头对过。
 *
 * 所以把判据钉进脚本：
 *
 * | 判据 | 规则 |
 * |---|---|
 * | 分支 | `main` 不许；`chore/next-preview-*`、`chore/version-*`、`chore/next-dev-version` 不许（进号不 PR） |
 * | 标题 | **PR 自己的属性**：`--title` 显式给 > 分支最早提交 > 分支名；后续提交不改它 |
 * | 已有 PR | 同一个分支只复用 / 更新同一个 PR，**绝不因标题变化关旧开新** |
 *
 * # 用法
 *
 * ```bash
 * npm run pr                  # 建 / 更新同一个 PR
 * npm run pr -- --title "…"   # 显式改标题（走 gh pr edit，不重开）
 * npm run pr -- --merge       # 建 + 等 CI 全绿 + squash 合并
 * npm run pr -- --list        # 列出 open PR（含标题与提交数）
 * ```
 *
 * # 它不做的事
 *
 * 不替你写 PR 正文 —— 正文由 `origin/main..HEAD` 的提交列表自动生成，
 * 手写的那一层正是"标题与内容对不上"的来源。
 */

import { execFileSync } from 'node:child_process'
import { argv, exit } from 'node:process'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { resolve } from 'node:path'

const REPO = resolve(fileURLToPath(new URL('.', import.meta.url)), '..')

/* ---------- 判据（纯函数，`scripts/check-pr.mjs` 直接测这几个） ---------- */

/** 这些分支**不建 PR**：进号是本地动作，合并它等于把 main 的号往回拨。 */
export const NO_PR_PATTERNS = [
  { re: /^chore\/next-preview-\d+$/, why: '进下一个预览号不建 PR（作者口径：就在分支上）' },
  { re: /^chore\/version-/, why: '版本号滚动不建 PR' },
  { re: /^chore\/next-dev-version$/, why: '进下一个开发号不建 PR' },
  { re: /^main$/, why: 'main 只能由 PR 推进，不能在它上面开 PR' },
]

/** 与 `scripts/hooks/pre-commit` 闸⑥ 同口径。 */
export const ALLOWED_PREFIX = ['feat/', 'fix/', 'refactor/', 'chore/', 'docs/', 'style/']

/**
 * 这个分支为什么不建 PR；能建就返回 `null`。
 */
export function refusalFor(branch) {
  const bare = String(branch || '').trim()
  if (!bare) return '取不到当前分支名'
  for (const { re, why } of NO_PR_PATTERNS) {
    if (re.test(bare)) return `${bare}：${why}`
  }
  if (!ALLOWED_PREFIX.some((p) => bare.startsWith(p))) {
    return `${bare} 没有 ${ALLOWED_PREFIX.join(' / ')} 前缀（与 pre-commit 闸⑥ 同口径）`
  }
  return null
}

/**
 * PR 标题该是什么：**PR 自己的属性**，不跟最新提交走。
 *
 * 取值顺序：`--title` 显式给的 > 分支上**最早**那条提交的主题 > 分支名。
 * 用最早那条而不是最新，是因为后续修复提交不该把已经开着的 PR 标题顶掉。
 */
export function prTitleFrom({ explicit, firstCommit, branch }) {
  return titleFor(explicit) || titleFor(firstCommit) || titleFor(branch)
}

/** 空白归一；只做这一件事，不做业务判断。 */
export function titleFor(subject) {
  return String(subject || '')
    .replace(/\s+/g, ' ')
    .trim()
}

/* ---------- 命令 ---------- */

function run(cmd, args, cwd = REPO) {
  return execFileSync(cmd, args, { cwd, encoding: 'utf8' }).trim()
}
function tryRun(cmd, args, cwd = REPO) {
  try {
    return run(cmd, args, cwd)
  } catch {
    return null
  }
}
function die(msg, hints = []) {
  console.error(`\n\x1b[31m✗ ${msg}\x1b[0m\n`)
  for (const h of hints) console.error(`    ${h}`)
  if (hints.length) console.error('')
  exit(1)
}
const ok = (m) => console.log(`\x1b[32m✓\x1b[0m ${m}`)
const info = (m) => console.log(`  ${m}`)

/* ---------- 主流程 ---------- */

async function main() {
  const args = argv.slice(2)
  const has = (f) => args.includes(f)
  const argOf = (name) => {
    const i = args.indexOf(name)
    if (i >= 0) return args[i + 1] ?? ''
    const pref = `${name}=`
    const eq = args.find((a) => a.startsWith(pref))
    return eq ? eq.slice(pref.length) : null
  }

  if (has('--list')) return listPrs()
  if (has('--replace')) {
    info('--replace 已退役：标题不再跟提交绑定，同一个 PR 只更新不关闭。')
  }

  const branch = tryRun('git', ['branch', '--show-current'])
  if (!branch) die('取不到当前分支（在 detached HEAD 上？）')

  const refusal = refusalFor(branch)
  if (refusal) {
    die(refusal, [
      '这类分支只推远端，不进 main。',
      '真要合并进 main，换个 feat/ 或 fix/ 分支再提交。',
    ])
  }

  const dirty = tryRun('git', ['status', '--porcelain'])
  if (dirty) die('工作区不干净 —— 先提交，PR 里不该有未入库的东西', [dirty.split('\n').slice(0, 8).join('\n')])

  const ahead = tryRun('git', ['rev-list', '--count', 'origin/main..HEAD'])
  if (ahead === '0') die('这个分支相对 origin/main 没有提交，没什么可开的 PR')

  /* 标题是 PR 自己的属性：显式给了就用；否则用**最早**那条提交（后续修复不改它）。 */
  const firstCommit = (tryRun('git', ['log', '--reverse', '--format=%s', 'origin/main..HEAD']) || '')
    .split('\n')
    .filter(Boolean)[0]
  const title = prTitleFrom({ explicit: argOf('--title'), firstCommit, branch })
  if (!title) die('取不出 PR 标题：加 --title "..."')

  /* 已有 PR（含已合并的）：只复用 / 更新，绝不因为标题变化关闭重开。 */
  const existing = prForBranch(branch)
  if (existing && existing.state === 'MERGED') {
    ok(`#${existing.number} 已经合并过，不重复建 PR`)
    info(existing.url)
    return
  }
  if (existing) {
    ok(`复用已有 PR #${existing.number}`)
    info(existing.url)
    if (argOf('--title') !== null && existing.title !== title) {
      run('gh', ['pr', 'edit', String(existing.number), '--title', title])
      ok(`已更新 PR 标题：${title}`)
    } else if (existing.title !== title) {
      info(`保留已有标题：${existing.title}（改标题请显式传 --title）`)
    }
    if (has('--merge')) return await mergePr(existing.number, existing.title)
    return printNext()
  }

  const body = prBody()
  const created = tryRun('gh', [
    'pr', 'create',
    '--base', 'main',
    '--head', branch,
    '--title', title,
    '--body', body,
  ])
  if (!created) die('gh pr create 失败（看上面的报错）')
  const url = created.split('\n').pop().trim()
  ok(`建好 PR：${url}`)
  info(`标题：${title}（后续提交不会改它；要改请 gh pr edit 或 --title）`)

  const number = tryRun('gh', ['pr', 'view', '--json', 'number', '--jq', '.number'])
  if (has('--merge')) {
    if (!number) die('建好了但取不到 PR 号，手动：gh pr merge --squash')
    return await mergePr(Number(number), title)
  }
  printNext()
}

function printNext() {
  console.log(`
  下一步：
    npm run pr -- --merge     # 等 CI 全绿后 squash 合并（约 20 分钟）
    npm run pr -- --list      # 看还有哪些 PR 开着（收旧 PR 用）
`)
}

function prForBranch(branch) {
  const out = tryRun('gh', [
    'pr', 'list',
    '--head', branch,
    '--state', 'all',
    '--json', 'number,title,url,state',
  ])
  if (!out) return null
  const list = JSON.parse(out || '[]')
  /* 同分支可能既有已合并的旧 PR、又有新开的；优先 open。 */
  return list.find((p) => p.state === 'OPEN') ?? list[0] ?? null
}

function prBody() {
  const commits = tryRun('git', ['log', '--oneline', '--no-decorate', 'origin/main..HEAD']) || ''
  const lines = commits
    .split('\n')
    .filter(Boolean)
    .map((l) => `- ${l}`)
    .join('\n')
  return [
    '## 提交',
    '',
    lines || '（无）',
    '',
    '## 闸',
    '',
    '- PR 标题是 PR 自己的属性；后续提交只更新同一个 PR，不关旧开新',
    '- 合并方式只有 squash；merge commit 被仓库规则拒绝',
  ].join('\n')
}

async function mergePr(number, title) {
  const deadline = Date.now() + 35 * 60 * 1000
  info('等 CI（最多 35 分钟）')
  while (Date.now() < deadline) {
    const out = tryRun('gh', ['pr', 'checks', String(number), '--json', 'name,state'])
    if (out) {
      const checks = JSON.parse(out)
      const bad = checks.filter((c) => ['FAILURE', 'CANCELLED', 'TIMED_OUT', 'ERROR'].includes(c.state))
      const pending = checks.filter((c) => ['PENDING', 'QUEUED', 'IN_PROGRESS'].includes(c.state))
      if (bad.length) {
        die(
          `CI 红了：${bad.map((c) => `${c.name}=${c.state}`).join('、')}`,
          [`gh pr checks ${number} --web`, '修完 push，再跑 npm run pr -- --merge'],
        )
      }
      if (checks.length && !pending.length) {
        ok(`全绿：${checks.map((c) => c.name).join('、')}`)
        break
      }
      info(`等 ${pending.length} 项：${pending.map((c) => c.name).join('、')}`)
    }
    await sleep(15000)
  }

  // 合并前再核一次状态（标题不参与判断）
  const fresh = prForBranch(tryRun('git', ['branch', '--show-current']))
  if (!fresh || fresh.number !== number || fresh.state !== 'OPEN') {
    die('合并前查不到这个仍开着的 PR 了')
  }

  run('gh', ['pr', 'merge', String(number), '--squash', '--delete-branch'])
  ok(`已 squash 合并 #${number}`)
  info(`main 上那一条的标题：${title}`)
}

function listPrs() {
  const out = tryRun('gh', [
    'pr', 'list',
    '--state', 'open',
    '--limit', '30',
    '--json', 'number,title,headRefName,createdAt',
  ])
  if (!out) die('gh pr list 失败')
  const list = JSON.parse(out || '[]')
  if (!list.length) return ok('没有开着的 PR')

  console.log('\n开着的 PR：\n')
  for (const p of list) {
    const days = Math.floor((Date.now() - Date.parse(p.createdAt)) / 86400000)
    console.log(`  #${p.number}  ${p.headRefName}`)
    console.log(`         ${p.title}`)
    console.log(`         开了 ${days} 天\n`)
  }
  console.log('  同一个分支只会有一个 PR；要改标题：gh pr edit <n> --title "..."（不关旧开新）\n')
}

function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms))
}

/* 被 `scripts/check-pr.mjs` import 时不该跑主流程 */
const invoked = argv[1] && import.meta.url === pathToFileURL(argv[1]).href
if (invoked) {
  main().catch((e) => {
    console.error(`\n\x1b[31m✗ ${e?.message ?? e}\x1b[0m\n`)
    exit(1)
  })
}
