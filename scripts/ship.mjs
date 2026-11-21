#!/usr/bin/env node
/**
 * 发货入口：**先问清楚这次发什么，再去发，发完核对到底生效没有**。
 *
 * # 为什么要有它
 *
 * 2026-11-12 之前，「发」至少有四条路，形状各不相同，而且**都不像发版**：
 *
 * | 你做的 | 看着像发版吗 | 实际影响 |
 * |---|---|---|
 * | `preview:build` | 像（出包了、有版本号） | 只有带 `--local-release` 的本地服务看得到 |
 * | `local-release` | 像（有清单、有下载地址） | 同上，`127.0.0.1` |
 * | `presets publish` | 像（“发布”两个字） | 只换预设目录，**软件版本一动没动** |
 * | `release` | 像 | 打 tag、传包、写清单 —— 但**清单还要合一次 PR 才生效** |
 *
 * 四种「像」，只有一种真的所有人都能看到。所以把选择摊开成人话，选完复述一遍，
 * 发完再用**可核对的事实**（tag / Release / main 上的清单）回答“到底生效没有”。
 *
 * # 用法
 *
 * ```bash
 * npm run ship                       # 交互：菜单 → 复述 → 敲 yes
 * npm run ship -- --what local       # 跳过菜单（AI 用；发版仍需 --confirm）
 * npm run ship -- --what preview --confirm --note "这一版改了什么"
 * npm run ship -- --check            # 只看“现在发到哪一步了”，什么都不做
 * ```
 *
 * # 它不做的事
 *
 * 不替内核发版。真正的 tag / Release / 上传 / 写清单仍在 `release_tx`，
 * 这里只负责**分流、复述、事后核对**。
 */

import { execFileSync } from 'node:child_process'
import { argv, stdin, stdout, exit } from 'node:process'
import { readFileSync } from 'node:fs'
import { createInterface } from 'node:readline'
import { fileURLToPath } from 'node:url'
import { join, resolve } from 'node:path'

const REPO = resolve(fileURLToPath(new URL('.', import.meta.url)), '..')

/* ---------- 四种「发」 ---------- */

export const SHIPS = [
  {
    key: 'local',
    label: '只在本机看',
    tag: false,
    audience: '只有你自己（还要本地服务带 --local-release）',
    desc: '编译一份包 + 本地清单；不打 tag、不写仓库里的清单',
  },
  {
    key: 'preview',
    label: '发预览版',
    tag: true,
    audience: '主动点「检查更新」的人才看得到；正式版用户不受影响',
    desc: '打 tag、传包、Release 标 prerelease、写 release-preview.json',
  },
  {
    key: 'stable',
    label: '发正式版',
    tag: true,
    audience: '所有客户端都会收到「有新版」',
    desc: '打 tag、传包、Release、写 release.json',
  },
  {
    key: 'presets',
    label: '只发预设数据',
    tag: false,
    audience: '所有人（下次检查预设更新时）',
    desc: '只换 presets/delivery 的目录与账本；软件版本一动没动',
  },
]

/** 通道与版本号对不对得上（`-preview.N` 只能走预览通道）。 */
export function channelFor(version) {
  const v = String(version || '').trim()
  if (!v) return null
  return /-preview\.\d+$/i.test(v) ? 'preview' : 'stable'
}

/** 给一句人话：这次发的是哪一档。 */
export function sayShip(choice, version) {
  if (!choice) return '（不知道要发什么）'
  if (choice.key === 'presets') return '这次只发预设数据 —— 软件版本不变，客户端不会收到「有新版」'
  if (choice.key === 'local') return '这次只编本地包 —— 别人看不到，仓库清单不动'
  return `这次发 ${choice.key === 'stable' ? '正式版' : '预览版'} ${version}，会打 tag`
}

/* ---------- 命令 ---------- */

function run(cmd, args, cwd = REPO) {
  // 静默 stderr：探测类命令（gh release view 之类）失败是正常分支，不该刷屏
  return execFileSync(cmd, args, {
    cwd,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'ignore'],
  }).trim()
}
function tryRun(cmd, args, cwd = REPO) {
  try {
    return run(cmd, args, cwd)
  } catch {
    return null
  }
}
/**
 * Windows 上 `npm` / `npx` 是 `.cmd`，`execFileSync` 认不出裸名（ENOENT）。
 * 经 cmd 走一遍；含空格的参数补引号（`ship` 的说明文字就带空格）。
 */
function nativeCommand(cmd, args) {
  if (process.platform !== 'win32' || (cmd !== 'npm' && cmd !== 'npx')) {
    return { cmd, args }
  }
  const q = (a) => (/\s/.test(a) ? `"${a}"` : a)
  return { cmd: process.env.ComSpec || 'cmd.exe', args: ['/d', '/s', '/c', [cmd, ...args.map(q)].join(' ')] }
}

function sh(cmd, args) {
  const c = nativeCommand(cmd, args)
  execFileSync(c.cmd, c.args, { cwd: REPO, stdio: 'inherit' })
}
function die(msg, hints = []) {
  console.error(`\n\x1b[31m✗ ${msg}\x1b[0m\n`)
  for (const h of hints) console.error(`    ${h}`)
  if (hints.length) console.error('')
  exit(1)
}
const ok = (m) => console.log(`\x1b[32m✓\x1b[0m ${m}`)
const bad = (m) => console.log(`\x1b[31m✗\x1b[0m ${m}`)
const info = (m) => console.log(`  ${m}`)

function versionNow() {
  const text = readFileSync(join(REPO, 'version.toml'), 'utf8')
  const m = text.match(/^version\s*=\s*"([^"]+)"/m)
  return m ? m[1] : null
}

/* ---------- 事后核对：到底生效没有 ---------- */

/**
 * 三件可核对的事实：远端有没有 tag、有没有 Release、**main 上的清单是不是这一版**。
 *
 * 最后那条才是「别人看不看得到」的开关 —— 2026-11-12 之前一直漏在这。
 * 给了 `assets` 就再加一格：清单里每个资产的**每个下载源**都得真的下得到。
 */
export function verdict({ tag, release, manifestVersion, wantVersion, channel, assets }) {
  const lines = []
  let effective = true

  lines.push(tag ? ['✓', `tag v${wantVersion} 在远端`] : ['✗', `tag v${wantVersion} 不在远端`])
  if (!tag) effective = false

  lines.push(release ? ['✓', `Release v${wantVersion} 已建`] : ['✗', `Release v${wantVersion} 没有`])
  if (!release) effective = false

  const same = manifestVersion === wantVersion
  lines.push(
    same
      ? ['✓', `main 上的清单已经是 ${wantVersion}`]
      : ['✗', `main 上的清单还是 ${manifestVersion ?? '（读不到）'}`],
  )
  if (!same) effective = false

  /* 资产那一格：清单写着「这个地址能下载」，就逐份去确认一遍（HEAD）。
     没核的数字不出现 —— `shaChecked` 只在真比过 GitHub 摘要时才计数。 */
  if (assets) {
    if (assets.problems.length > 0) {
      lines.push(['✗', `资产核对：${assets.problems.length} 处对不上`])
      for (const p of assets.problems) lines.push(['✗', p])
      effective = false
    } else {
      lines.push(['✓', `资产两源核对通过（${assets.checked} 份地址可达、大小与清单一致）`])
      if (assets.shaChecked > 0) {
        lines.push(['✓', `GitHub SHA 与清单一致（${assets.shaChecked} 份）`])
      }
    }
  }

  return { effective, lines, channel }
}

/**
 * HEAD 一下远端文件，返回字节数；下不到 / 没给长度 = `null`（调用方如实报）。
 *
 * 用 HEAD 不用 GET：一个包十几 MB，核对不该先把它下完。Gitee 会 302 到文件存储，
 * `fetch` 默认跟随重定向，最终那一段的 `content-length` 就是文件本身的大小。
 */
async function headSize(url) {
  try {
    const res = await fetch(url, { method: 'HEAD', signal: AbortSignal.timeout(15000) })
    if (!res.ok) return null
    const len = Number(res.headers.get('content-length'))
    return Number.isFinite(len) && len > 0 ? len : null
  } catch {
    return null
  }
}

/** GitHub 上这一版每个附件（`{name, size, digest}`）；没有 gh / 没登录 = `null`（那就是没核）。 */
function githubAssets(tag) {
  const out = tryRun('gh', ['release', 'view', tag, '--json', 'assets'])
  if (!out) return null
  try {
    const map = new Map()
    for (const a of JSON.parse(out).assets ?? []) map.set(a.name, a)
    return map
  } catch {
    return null
  }
}

/**
 * 清单里的资产 × 它写的每个下载源，逐份核：
 *
 *   · 两源都要**真的下得到**（HEAD 200），大小与清单一致；
 *   · GitHub 额外核 SHA（`gh` 给 `digest: sha256:…`）；
 *   · Gitee 的 API 不给摘要 —— **只核大小，不假装核过 SHA**。
 *
 * 清单没写 `sources.github` 时就只有 Gitee 一份，不替它补一个「应该有 GitHub」。
 */
export async function checkAssets(info, version) {
  const list =
    Array.isArray(info.assets) && info.assets.length > 0
      ? info.assets
      : info.asset
        ? [info.asset]
        : []
  const gh = githubAssets(`v${version}`)
  const problems = []
  let checked = 0
  let shaChecked = 0
  for (const a of list) {
    const sources = a.sources && Object.keys(a.sources).length > 0 ? a.sources : { gitee: a.url }
    for (const [src, url] of Object.entries(sources)) {
      if (typeof url !== 'string' || !/^https?:\/\//.test(url)) {
        problems.push(`${a.name} 的 ${src} 地址不是一个 http(s) 链接`)
        continue
      }
      /* GitHub 这一侧**问 gh 的资产表**，不去 HEAD 下载地址：
         ① 本机走代理时 Node 的 fetch 不认系统代理（直连 GitHub 会超时），`gh` 认；
         ② 一次就把「在不在、多大、SHA」拿齐，不用下载。 */
      if (src === 'github' && gh) {
        const got = gh.get(a.name)
        if (!got) {
          problems.push(`${a.name} 不在 GitHub Release 的资产列表里`)
          continue
        }
        if (typeof a.size === 'number' && a.size > 0 && got.size !== a.size) {
          problems.push(`${a.name} 在 github 上是 ${got.size} 字节，清单写 ${a.size}`)
          continue
        }
        if (typeof a.sha256 === 'string' && a.sha256) {
          const want = `sha256:${a.sha256.toLowerCase()}`
          if (typeof got.digest !== 'string' || !got.digest) {
            problems.push(`${a.name} 在 GitHub 上没有摘要可核`)
            continue
          }
          if (got.digest.toLowerCase() !== want) {
            problems.push(`${a.name} 在 GitHub 上的 SHA 是 ${got.digest}，清单写 ${want}`)
            continue
          }
          shaChecked++
        }
        checked++
        continue
      }
      const size = await headSize(url)
      if (size === null) {
        problems.push(`${a.name} 在 ${src} 上下不到 —— ${url}`)
        continue
      }
      if (typeof a.size === 'number' && a.size > 0 && size !== a.size) {
        problems.push(`${a.name} 在 ${src} 上是 ${size} 字节，清单写 ${a.size}`)
        continue
      }
      checked++
    }
  }
  return { checked, shaChecked, problems }
}

/**
 * Release 到底在不在：**按清单里的发布仓库问**。
 *
 * 软件版本的 Release 按 M6 只发 Gitee；以前这里写死查 GitHub，于是每次真发完
 * 都会误报“Release 没有”，逼人去补一个没人用的 GitHub 镜像。清单的 `url`
 * 就是权威落点，从它取 owner/repo 再问 Gitee API。问不到才退回 GitHub（老账）。
 */
async function releaseExists(version, manifestText) {
  try {
    const url = new URL(JSON.parse(manifestText).url)
    if (url.hostname !== 'gitee.com') throw new Error('not gitee')
    const [owner, repo] = url.pathname.split('/').filter(Boolean)
    const res = await fetch(
      `https://gitee.com/api/v5/repos/${owner}/${repo}/releases/tags/v${version}`,
    )
    return res.ok
  } catch {
    return Boolean(
      tryRun('gh', ['release', 'view', `v${version}`, '--json', 'tagName', '--jq', '.tagName']),
    )
  }
}

async function checkEffective(choice, version) {
  if (!choice.tag) return null
  info('核对：远端 tag / Release / main 上的清单 / 两源资产')

  const tag = tryRun('git', ['ls-remote', '--tags', 'origin', `refs/tags/v${version}`])

  const rel = choice.key === 'stable' ? 'presets/delivery/release.json' : 'presets/delivery/release-preview.json'
  tryRun('git', ['fetch', 'origin', 'main', '--quiet'])
  const manifestText = tryRun('git', ['show', `origin/main:${rel}`])
  let manifest = null
  if (manifestText) {
    try {
      manifest = JSON.parse(manifestText)
    } catch {
      manifest = null
    }
  }
  const release = manifestText ? await releaseExists(version, manifestText) : false
  /* 清单读得懂才核资产：读不懂时上面那一格已经报红，这里不添一条假结论。 */
  const assets = manifest ? await checkAssets(manifest, version) : undefined

  return verdict({
    tag: Boolean(tag),
    release: Boolean(release),
    manifestVersion: manifest?.version ?? null,
    wantVersion: version,
    channel: choice.key,
    assets,
  })
}

function printVerdict(result, choice) {
  if (!result) return
  console.log('')
  for (const [mark, line] of result.lines) {
    if (mark === '✓') ok(line)
    else bad(line)
  }
  console.log('')
  if (result.effective) {
    ok('这一版真的生效了：客户端点「重新检查」就能看到')
    return
  }
  bad('还没生效 —— 别以为发了。本命令以失败退出，不许当成成功')
  info(`清单文件：${choice.key === 'stable' ? 'presets/delivery/release.json' : 'presets/delivery/release-preview.json'}`)
  info('去看有没有开着的清单 PR：npm run pr -- --list')
  info('合并它之后重跑：npm run ship -- --check')
  exit(1)
}

/* ---------- 主流程 ---------- */

async function main() {
  const args = argv.slice(2)
  const argOf = (n) => {
    const i = args.indexOf(n)
    return i >= 0 ? (args[i + 1] ?? '') : null
  }

  const version = versionNow()
  if (!version) die('读不到 version.toml 的 version')
  const natural = channelFor(version)

  if (args.includes('--check')) {
    const ch = choiceFromKey(natural) ?? SHIPS[1]
    const r = await checkEffective(ch, version)
    if (!r) return ok(`${version} 是本机开发号，不是发过的版本 —— 没有可核对的 tag / 清单`)
    printVerdict(r, ch)
    return
  }

  let choice = choiceFromKey(argOf('--what'))
  let confirmed = args.includes('--confirm')
  let note = argOf('--note')

  /* 交互：菜单 → 复述 → 敲 yes */
  if (!choice) {
    /* 管道输入（AI / `printf` 喂）时 EOF 会让 rl.question 永远不回调，
     * 所以自己收行：先缓冲，问题来了先取缓冲；没有就排队等，close 时用空串兜底。 */
    const rl = createInterface({ input: stdin, output: stdout, terminal: false })
    const buffered = []
    const waiters = []
    rl.on('line', (l) => (waiters.length ? waiters.shift()(l) : buffered.push(l)))
    rl.on('close', () => {
      while (waiters.length) waiters.shift()('')
    })
    const ask = (q) => {
      stdout.write(q)
      if (buffered.length) return Promise.resolve(buffered.shift())
      return new Promise((res) => waiters.push(res))
    }
    console.log(`\n  当前 version.toml：\x1b[1m${version}\x1b[0m\n`)
    console.log('  这次要发什么？\n')
    SHIPS.forEach((s, i) => {
      console.log(`  ${i + 1}) ${s.label}`)
      console.log(`     ${s.desc}`)
      console.log(`     影响范围：${s.audience}\n`)
    })
    console.log('  0) 什么都不做\n')

    const pick = (await ask('  选一个（0-4）：')).trim()
    choice = choiceFromKey(SHIPS[Number(pick) - 1]?.key) ?? null
    if (!choice) {
      rl.close()
      return ok('什么都没做')
    }

    console.log('')
    console.log(`  \x1b[1m${sayShip(choice, version)}\x1b[0m`)
    if (choice.tag) {
      if (!natural) die('版本号形状认不出来')
      if (choice.key !== natural) {
        die(
          `${version} 是${natural === 'preview' ? '预览' : '正式'}号的形状，不能按${choice.key === 'stable' ? '正式' : '预览'}版发`,
          ['换号：npm run version:ensure 或改 version.toml 后重来'],
        )
      }
      console.log(`  通道：${choice.key}　版本：${version}`)
      console.log(`  会打 tag：\x1b[33mv${version}\x1b[0m\n`)
    }
    const yes = (await ask('  确认就敲 yes：')).trim().toLowerCase()
    rl.close()
    if (yes !== 'yes') return ok('没有确认，什么都没做')
    confirmed = true
  }

  if (!choice) die(`--what 要 one of: ${SHIPS.map((s) => s.key).join(' / ')}`)
  if (choice.tag && !confirmed) {
    die(
      '发版要人类当次确认：交互跑敲 yes，非交互跑带 --confirm',
      ['AI 必须先拿到人的明确许可，再带 --confirm 跑这一条'],
    )
  }
  if (choice.tag && choice.key !== natural) {
    die(`${version} 与通道 ${choice.key} 不匹配`)
  }

  /* 发 */
  if (choice.key === 'local') {
    const dirty = tryRun('git', ['status', '--porcelain'])
    if (dirty) die('本地包要求工作区干净（tauri build 的闸）', [dirty.split('\n').slice(0, 6).join('\n')])
    info('编译（npm run tauri -- build）')
    sh('npm', ['run', 'tauri', '--', 'build'])
    info('生成本地清单（node scripts/local-release.mjs）')
    sh('node', ['scripts/local-release.mjs'])
    console.log('')
    ok('本地包好了。别人看不到它 —— 要看就带 --local-release 起服务：')
    info('npm run preset-source:dev -- --local-release')
    return
  }

  if (choice.key === 'presets') {
    info('预演（--dry-run）')
    sh('npm', ['run', 'presets', '--', 'publish', '--dry-run'])
    console.log('')
    info('真发（会自己 commit + push）')
    sh('npm', ['run', 'presets', '--', 'publish', '--confirm'])
    console.log('')
    ok('预设目录发了。软件版本没动 —— 客户端只会在下次检查预设更新时看到')
    return
  }

  /* preview / stable */
  /* 深度预检：把凭据失效、tag 闸标记缺失、空 PR 这些以前要等 CI 全绿之后
     才暴露的问题挡在开 PR 之前（静态部分也已挂在 lint 上，跟着每个 PR 跑）。 */
  info('发版深度预检（版本 / tag 闸 / 分支 / 凭据 / 空 PR）')
  sh('node', ['scripts/check-release-readiness.mjs', `--channel=${choice.key}`])
  const summary = (note ?? '').trim() || `发 ${choice.label} ${version}`
  info(`调用：node scripts/release.mjs ${version} "${summary}" --channel ${choice.key} --confirm`)
  /*
   * `MKP_SHIP=1` 是「这一趟是 ship 发起的」的凭据 —— `release.mjs` 见到它才肯真发版。
   * 反过来，直接 `npm run release` 会被它当场拒掉：2026-11-18 就是这么出的错，
   * 跑完整趟、tag 与 Release 都发了，清单没写、`ship --check` 第三格一直红。
   */
  process.env.MKP_SHIP = '1'
  try {
    /* 直接调 node，不经 npm/cmd：说明带空格，Windows 的 cmd /c 会把它拆成多个位置参数，
       Release 正文只剩第一个词（0.0.13 真机踩过）。 */
    sh(process.execPath, [
      'scripts/release.mjs',
      version,
      summary,
      `--channel=${choice.key}`,
      '--confirm',
    ])
  } finally {
    delete process.env.MKP_SHIP
  }

  console.log('')
  const r = await checkEffective(choice, version)
  printVerdict(r, choice)
}

function choiceFromKey(key) {
  if (!key) return null
  return SHIPS.find((s) => s.key === String(key).trim()) ?? null
}

/* 直接 `node scripts/ship.mjs` 才跑主流程：被 import（判据 / 自检）时只拿函数，不执行。 */
const invokedDirectly = process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)
if (invokedDirectly) {
  main().catch((e) => {
    console.error(`\n\x1b[31m✗ ${e?.message ?? e}\x1b[0m\n`)
    exit(1)
  })
}
