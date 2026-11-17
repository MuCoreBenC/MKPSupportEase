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
 * | 标题 | **必须**等于最新提交的主题 —— squash 之后 main 上那一条就是它 |
 * | 已有 PR | 标题对得上才复用；对不上就停，给 `--replace` 让你显式换 |
 *
 * # 用法
 *
 * ```bash
 * npm run pr                  # 建 / 校验已有 PR（标题取最新提交）
 * npm run pr -- --replace     # 标题对不上时：关掉旧的开新的
 * npm run pr -- --merge       # 建 + 等 CI 全绿 + squash 合并
 * npm run pr -- --list        # 列出 open PR（含标题与提交数），收旧 PR 用
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
 * PR 标题该是什么：**最新提交的主题**。
 *
 * 理由：仓库只允许 squash 合并，main 上那一条提交的标题就是 PR 标题。
 * 标题与最新提交不一致 ⇒ 合并后的历史指向别的东西。
 */
export function titleFor(subject) {
  return String(subject || '')
    .replace(/\s+/g, ' ')
    .trim()
}

/**
 * 已有 PR 的标题与最新提交对不上时，返回一句为什么；对得上返回 `null`。
 */
export function titleMismatch(prTitle, subject) {
  const want = titleFor(subject)
  const got = titleFor(prTitle)
  if (!want) return '最新提交没有主题，取不出标题'
  if (got === want) return null
  return `PR 标题「${got}」≠ 最新提交「${want}」—— 合并后 main 上那一条会是旧标题`
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

  if (has('--list')) return listPrs()

  const branch = tryRun('git', ['branch', '--show-current'])
  if (!branch) die('取不到当前分支（在 detached HEAD 上？）')

  const refusal = refusalFor(branch)
  if (refusal) {
    die(refusal, [
      '这类分支只推远端，不进 main。',
      '真要合并进 main，换个 feat/ 或 fix/ 分支再提交。',
    ])
  }

  const subject = tryRun('git', ['log', '-1', '--format=%s'])
  if (!subject) die('取不到最新提交')
  const title = titleFor(subject)

  const dirty = tryRun('git', ['status', '--porcelain'])
  if (dirty) die('工作区不干净 —— 先提交，PR 里不该有未入库的东西', [dirty.split('\n').slice(0, 8).join('\n')])

  const ahead = tryRun('git', ['rev-list', '--count', 'origin/main..HEAD'])
  if (ahead === '0') die('这个分支相对 origin/main 没有提交，没什么可开的 PR')

  /* 已有 PR：标题对得上才复用 */
  const existing = openPrFor(branch)
  if (existing) {
    const bad = titleMismatch(existing.title, title)
    if (bad) {
      if (!has('--replace')) {
        die(bad, [
          `旧 PR：#${existing.number}（${existing.url}）`,
          `关掉重开：gh pr close ${existing.number} --comment "标题与当前内容不符，重开" && npm run pr`,
          `或者一条命令：npm run pr -- --replace`,
        ])
      }
      info(`关掉旧 PR #${existing.number}（标题对不上）`)
      run('gh', ['pr', 'close', String(existing.number), '--comment', '标题与当前内容不符，按新提交重开'])
      ok(`已关闭 #${existing.number}`)
    } else {
      ok(`复用已有 PR #${existing.number}（标题与最新提交一致）`)
      info(existing.url)
      if (has('--merge')) return await mergePr(existing.number, title)
      return printNext()
    }
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
  info(`标题（= 合并后 main 上的那一条）：${title}`)

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

function openPrFor(branch) {
  const out = tryRun('gh', [
    'pr', 'list',
    '--head', branch,
    '--state', 'open',
    '--json', 'number,title,url',
  ])
  if (!out) return null
  const list = JSON.parse(out || '[]')
  return list[0] ?? null
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
    '- 标题 = 最新一条提交的主题（squash 之后 main 上那一条就是它）',
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

  // 合并前再核一次：推送中途改过提交就会在这里被抓住
  const fresh = openPrFor(tryRun('git', ['branch', '--show-current']))
  if (!fresh) die('合并前查不到 PR 了')
  const bad = titleMismatch(fresh.title, tryRun('git', ['log', '-1', '--format=%s']))
  if (bad) die(bad, ['CI 期间又推了提交，标题要跟着改：npm run pr -- --replace'])

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
  console.log('  标题与分支当前内容对不上的，关掉重开：gh pr close <n> && npm run pr\n')
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
