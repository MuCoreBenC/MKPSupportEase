#!/usr/bin/env node
/**
 * **本地发布**：一个版本一跑，不用发版就能测更新。
 *
 * # 它解决的那件事
 *
 * 想验证「下载 → 校验 → 解包 → 替换 → 重启」这一整条更新链，以前只有一条路：
 * 真的发一个版本（打不可变 tag、传安装包、写清单、等镜像）。那要二十分钟，
 * 而 tag 推上去删不掉。
 *
 * 这个脚本把**已经在盘上的构建产物**变成一份"发布形状"的货：
 *
 * ```text
 * target/release/bundle/macos/*.app.zip        ← `npm run tauri build` 的产物
 *        ↓ 拷进
 * target/local-release/SupportEase_<版本>_<arch>.app.zip
 *        ↓ 再写一份清单
 * target/local-release/release-preview.json    ← 指向 http://127.0.0.1:8787/delivery/<那支 zip>
 * ```
 *
 * 然后 `npm run preset-source:dev -- --local-release` 把 `presets/delivery` 端出来，
 * **并对 `release-preview.json` 做覆盖**（只改响应，盘上一个字节不动）。
 * 客户端把预设源切到 `http://127.0.0.1:8787/delivery`、点「重新检查」——
 * 它走的是 catalog / manifest / 寻址 / SHA / 下载 / 替换那一条**真链**，
 * 只是"云端"在本机。
 *
 * # 版本号为什么要专门算
 *
 * 客户端只摆**比当前构建新**的候选（`release_info::is_newer_version`）。而
 * `APP_VERSION` 是编译进二进制的，`npm run tauri build` 编出来的号就是当前这个 ——
 * 直接拿它写清单，界面上一张卡都不会出现，看着像"本地源没生效"。
 *
 * 所以默认取**包里那个版本的下一个预览号**（`0.0.10-preview.2` → `preview.3`），
 * 并明确告诉你"包里真正的版本还是 X"。想换成别的号用 `--version`。
 *
 * # 用法
 *
 * ```bash
 * npm run local-release                    # 用最新的 app.zip，预览通道
 * npm run local-release -- --version 0.0.11-preview.1
 * npm run local-release -- --channel stable
 * npm run local-release -- --asset <某个 .zip / .app / .dmg>
 * ```
 */

import { execFileSync } from 'node:child_process'
import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { createHash } from 'node:crypto'
import { basename, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { tmpdir } from 'node:os'

const HERE = fileURLToPath(new URL('.', import.meta.url))
const REPO = resolve(HERE, '..')
const BUNDLE = join(REPO, 'target', 'release', 'bundle')
const OUT = join(REPO, 'target', 'local-release')
/** 清单与 zip 都在这一层；服务端对 `/delivery/<名字>` 先在这里找（找到就覆盖） */

const argv = process.argv.slice(2)
const argOf = (name) => {
  const at = argv.indexOf(name)
  return at >= 0 ? argv[at + 1] : undefined
}

/* 端口必须与 `preset-source:dev` 那一档一致：清单里的地址是**绝对地址**，
   写错了客户端会去一个没人听的端口上取包（症状是"下载失败"，看不出是写错了） */
const PORT = Number(argOf('--port') ?? process.env.PRESET_TEST_PORT ?? 8787)
const ORIGIN = `http://127.0.0.1:${PORT}`

const die = (msg) => {
  console.error(`\n\u001b[31m停在这里：${msg}\u001b[0m\n`)
  process.exit(1)
}

const run = (cmd, args, opts = {}) =>
  execFileSync(cmd, args, { encoding: 'utf8', ...opts }).trim()

/* ---------- 1. 找那份构建产物 ---------- */

/** 目录里最新的一个匹配项（按 mtime）；没有就 null */
function newest(dir, re) {
  if (!existsSync(dir)) return null
  const files = readdirSync(dir)
    .filter((n) => re.test(n))
    .map((n) => join(dir, n))
    .filter((p) => statSync(p).isFile())
    .map((p) => ({ p, t: statSync(p).mtimeMs }))
    .sort((a, b) => b.t - a.t)
  return files[0]?.p ?? null
}

const ARCH = process.arch === 'arm64' ? 'aarch64' : 'x64'

/** 把一个 `.app` 打成 Tauri 同款的 app.zip（`ditto` 保留符号链接与权限） */
function zipApp(app, out) {
  rmSync(out, { force: true })
  execFileSync('ditto', ['-c', '-k', '--sequesterRsrc', '--keepParent', app, out])
  return out
}

/**
 * 从 dmg 里拿出 `.app`（挂到临时挂载点，**先拷出来**，用完卸载）。
 *
 * ★ **必须先拷出来**：`finally` 一 detach，挂载点里的路径就没了
 *   那个路径就没了 —— 后面的 `ditto` 报 `Cannot get the real path`。
 */
function appFromDmg(dmg) {
  const mnt = join(tmpdir(), `mkpse-dmg-${process.pid}`)
  const dest = join(tmpdir(), `mkpse-dmg-app-${process.pid}`)
  mkdirSync(mnt, { recursive: true })
  try {
    execFileSync('hdiutil', ['attach', '-nobrowse', '-readonly', '-mountpoint', mnt, dmg], {
      stdio: 'ignore',
    })
    const app = readdirSync(mnt).find((n) => n.endsWith('.app'))
    if (!app) die(`dmg 里没有 .app：${dmg}`)
    rmSync(dest, { recursive: true, force: true })
    run('ditto', [join(mnt, app), dest])
    return dest
  } finally {
    try {
      execFileSync('hdiutil', ['detach', '-quiet', mnt], { stdio: 'ignore' })
    } catch {
      /* 卸不掉就留着 —— 下次 `hdiutil detach <挂载点>` 收拾，不在这里编错误 */
    }
  }
}

/**
 * 产出 `{ zip, appVersion }`：优先用盘上现成的 `.app.zip`（Tauri 自己打的），
 * 没有就从 `.app` 压，再没有就从最新的 dmg 里拿。
 */
function pickAsset() {
  const explicit = argOf('--asset')
  if (explicit !== undefined) {
    const p = resolve(explicit)
    if (!existsSync(p)) die(`--asset 指的路径不存在：${p}`)
    if (p.endsWith('.app.zip')) return { zip: p, appVersion: null }
    if (p.endsWith('.app')) return { zip: zipApp(p, join(OUT, `tmp-${basename(p)}.zip`)), appVersion: plistVersion(p) }
    if (p.endsWith('.dmg')) {
      const app = appFromDmg(p)
      return { zip: zipApp(app, join(OUT, `tmp-${basename(app)}.zip`)), appVersion: plistVersion(app) }
    }
    die(`认不出这是什么：${p}（要 .app.zip / .app / .dmg）`)
  }

  /*
   * ★ **按 mtime 取最新那一份**：`.app.zip` 可能是 `scripts/release.mjs` 上次留下的旧成品，
   *   而刚编的 `.app` 就在旁边 —— 挑错了会"装上去还是老代码，日志一切正常"。
   *   改成**按 mtime 取最新的一份**，并把挑中的是谁打出来。
   */
  const cands = []
  const zip = newest(join(BUNDLE, 'macos'), /\.app\.zip$/)
  if (zip !== null) cands.push({ kind: 'app.zip', path: zip, at: statSync(zip).mtimeMs })
  const appDir = join(BUNDLE, 'macos', 'SupportEase.app')
  if (existsSync(appDir)) {
    /* 二进制那一刻才是"这一份是什么时候编的"；目录 mtime 会被别的写入带着跳 */
    const macos = join(appDir, 'Contents', 'MacOS')
    cands.push({
      kind: 'app',
      path: appDir,
      at: statSync(existsSync(macos) ? macos : appDir).mtimeMs,
    })
  }
  /*
   * **dmg 不进这张候选表**：它是同一次构建的派生物，mtime 总比 `.app` 新，
   * 拿它当候选只会把最可靠的那份挤掉。
   * 只有 `.app` / `.app.zip` 都没有时，才拿 dmg 兜底。
   */
  if (cands.length === 0) {
    const dmg = newest(join(BUNDLE, 'dmg'), /\.dmg$/)
    if (dmg === null) {
      die(
        '没找到构建产物（target/release/bundle 下没有 .app.zip / .app / .dmg）。\n' +
          '    先编一份：npm run tauri -- build（客户端；不要用工作台那一档）',
      )
    }
    console.log(`[local-release] 只有 dmg，从它里面取 .app：${dmg}`)
    mkdirSync(OUT, { recursive: true })
    const app = appFromDmg(dmg)
    const version = plistVersion(app)
    const zip = zipApp(app, join(OUT, `tmp-${basename(app)}.zip`))
    rmSync(app, { recursive: true, force: true })
    return { zip, appVersion: version }
  }

  cands.sort((a, b) => b.at - a.at)
  const best = cands[0]
  console.log(`[local-release] 用最新的一份产物：${best.kind} · ${best.path}`)

  if (best.kind === 'app.zip') return { zip: best.path, appVersion: null }

  mkdirSync(OUT, { recursive: true })
  return {
    zip: zipApp(best.path, join(OUT, `tmp-${basename(best.path)}.zip`)),
    appVersion: plistVersion(best.path),
  }
}

/** 那个 .app 里写的版本（Tauri 从 Cargo.toml 取，等于编译进二进制的 APP_VERSION） */
function plistVersion(app) {
  const plist = join(app, 'Contents', 'Info.plist')
  if (!existsSync(plist)) return null
  try {
    return run('plutil', ['-extract', 'CFBundleShortVersionString', 'raw', '-o', '-', plist])
  } catch {
    return null
  }
}

/* ---------- 2. 版本号 ---------- */

const RANK = { dev: 0, alpha: 1, a: 1, beta: 2, b: 2, preview: 3, pre: 3, rc: 4 }

function parseVersion(raw) {
  const m = /^(\d+)\.(\d+)\.(\d+)(?:-([a-zA-Z]+)\.?(\d*))?$/.exec(String(raw ?? '').trim())
  if (m === null) return null
  return {
    major: Number(m[1]),
    minor: Number(m[2]),
    patch: Number(m[3]),
    kind: m[4]?.toLowerCase() ?? null,
    seq: m[5] === '' || m[5] === undefined ? 0 : Number(m[5]),
  }
}

/**
 * 下一个**严格更新**的预览号 —— 客户端只摆"比当前新"的候选，
 * 所以本地清单的号必须真的更大，否则界面一张卡都不出现。
 */
export function nextPreview(raw) {
  const v = parseVersion(raw)
  if (v === null) return null
  if (v.kind === 'preview' || v.kind === 'pre') {
    return `${v.major}.${v.minor}.${v.patch}-preview.${v.seq + 1}`
  }
  /* 正式版、rc：同基础再发预览不算"更新"，要进补丁号 */
  if (v.kind === null || (RANK[v.kind] ?? 0) >= RANK.preview) {
    return `${v.major}.${v.minor}.${v.patch + 1}-preview.1`
  }
  /* dev / alpha / beta：同基础的预览版就比它们新 */
  return `${v.major}.${v.minor}.${v.patch}-preview.1`
}

/** 与 `release_info::is_newer_version` 同一套口径（只判大小，不含通知守卫） */
export function isNewer(candidate, current) {
  const a = parseVersion(candidate)
  const b = parseVersion(current)
  if (a === null || b === null) return candidate !== current
  const base = (a.major - b.major) || (a.minor - b.minor) || (a.patch - b.patch)
  if (base !== 0) return base > 0
  if (a.kind === null && b.kind === null) return false
  if (a.kind === null) return true // 正式版比同基础预发布新
  if (b.kind === null) return false
  const ra = RANK[a.kind] ?? 0
  const rb = RANK[b.kind] ?? 0
  return ra === rb ? a.seq > b.seq : ra > rb
}

/* ---------- 3. 写货 ---------- */

const sha256 = (path) => createHash('sha256').update(readFileSync(path)).digest('hex')

function main() {
  const pkgVersion = JSON.parse(readFileSync(join(REPO, 'package.json'), 'utf8')).version
  const channel = (argOf('--channel') ?? 'preview').trim()
  if (channel !== 'preview' && channel !== 'stable') {
    die(`认不出的通道：${channel}（只认 preview / stable）`)
  }

  mkdirSync(OUT, { recursive: true })
  const picked = pickAsset()

  /* 1. **产物自报的号必须等于仓库这个号** —— 包里说 0.0.3、清单说 0.0.4 就是坏数据。
        `.app.zip` 那一档读不到 Info.plist（它是别人打好的成品），只能退回 package.json ——
        但**只要读得到就必须一致**，不一致说明"改了号没重编"。
        `APP_VERSION` 是编译期常量：改了 `Cargo.toml` 不重编，包里还是旧号。 */
  if (picked.appVersion !== null && picked.appVersion !== pkgVersion) {
    die(
      `产物自报 ${picked.appVersion}，而仓库版本是 ${pkgVersion} —— 两个号不是同一个。\n` +
        '    多半是改了版本号却没重新编译（APP_VERSION 是编译期常量）。\n' +
        '    重编一份再回来：npm run tauri -- build',
    )
  }

  /* 2. **清单号 = 产物自己报的那个号**，不再另算。
        清单号与包里不一致时，客户端装完看到的还是旧号。要号真的变，
        就得**先抬号再重编**（`npm run preview:build` 按顺序做完）。
        真要手写一个号才用 `--version`。 */
  const appVersion = picked.appVersion ?? pkgVersion
  const version = (argOf('--version') ?? appVersion).trim()
  if (parseVersion(version) === null) die(`版本号格式不对：${version}（要 x.y.z 或 x.y.z-preview.N）`)

  const name = `SupportEase_${version}_${ARCH}.app.zip`
  const zipPath = join(OUT, name)
  if (resolve(picked.zip) !== resolve(zipPath)) copyFileSync(picked.zip, zipPath)
  /* 临时 zip 只用来搬这一趟 */
  if (basename(picked.zip).startsWith('tmp-')) rmSync(picked.zip, { force: true })

  /* ★ **历史安装包一律保留**（
     而且"删掉安装包也要能认出它冻结过"）。以前这里会把别的版本删掉 —— 那等于把回滚路径
     也删了。客户端只认清单里那一支，目录里多留几支不会让它点错。 */

  const size = statSync(zipPath).size
  const digest = sha256(zipPath)
  const manifestName = channel === 'stable' ? 'release.json' : 'release-preview.json'
  const assetUrl = `${ORIGIN}/delivery/${name}`
  const manifest = {
    releaseSchema: 1,
    version,
    notes: `本地测试版（未发布）—— 由 scripts/local-release.mjs 生成，包里的版本是 ${appVersion}`,
    /* 「查看更新」那一颗点开就是这份清单本身：本地没有 Release 页，给一个能看的东西 */
    url: `${ORIGIN}/delivery/${manifestName}`,
    asset: {
      name,
      url: assetUrl,
      size,
      sha256: digest,
      platform: 'macos',
      kind: 'appZip',
    },
  }
  writeFileSync(join(OUT, manifestName), `${JSON.stringify(manifest, null, 2)}\n`)

  /* ★ **历史交付要留得住**：
     · 安装包 zip 名字里带版本号，天然不会被下一轮盖掉；
     · 但清单每次都被**同名覆盖**（客户端只认 `release-preview.json` 这一份），
       所以这里再存一份带版本号的副本 —— 想回滚、想查"第 3 轮那版的清单",都还找得到。 */
  const history = join(OUT, 'history')
  mkdirSync(history, { recursive: true })
  copyFileSync(join(OUT, manifestName), join(history, `${version}.json`))

  /* 账本不在这里写：**只有 Tag 打成功才算交付**，那件事由发布事务负责。
     这个脚本只负责"把包和清单摆好"，它成功不代表这一版冻结了。 */

  /* ★ 三处身份摆在同一行：仓库 / 包内 / 清单。
     作者要的是"每个产物都报自己的真实身份" —— 那就让它们并排出现，
     一眼看得出有没有谁在冒充别人。 */
  const same = version === appVersion
  console.log('')
  console.log(`\u001b[32m✓ 本地发布就绪\u001b[0m  ${name}`)
  console.log(`  清单      ${join(OUT, manifestName)}`)
  console.log(`  版本      ${version}`)
  console.log(
    `  一致性    仓库 ${pkgVersion} · 包内 ${appVersion} · 清单 ${version}` +
      (same ? '  \u001b[32m✓\u001b[0m' : `  \u001b[31m← 清单号与包内不符\u001b[0m`),
  )
  console.log(`  大小/SHA  ${size} B / ${digest.slice(0, 16)}…`)
  console.log(`  下载地址  ${assetUrl}`)
  console.log('')
  console.log('  下一步：')
  console.log('    npm run preset-source:dev -- --local-release     # 起服务（覆盖这一份清单）')
  console.log(`    客户端 → 设置 → 高级设置 → 预设数据源 → 自定义地址：${ORIGIN}/delivery`)
  console.log('    然后点「重新检查」')
  console.log('')
  if (channel === 'stable') {
    console.log(
      '  \u001b[33m注意\u001b[0m：这份是**正式通道**的清单 —— 客户端会在下次拉目录时自动通知。',
    )
    console.log('         只测安装链的话用缺省的 preview。')
    console.log('')
  }
}



/* 被当做模块 import 时（自检 / 别的脚本）不执行 —— 只有直接跑才动盘 */
if (process.argv[1] !== undefined && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main()
}
