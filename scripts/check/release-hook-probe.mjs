#!/usr/bin/env node
/**
 * 发布物形态的钩子探针 —— 专治「开发机绿、用户机器红」。
 *
 * 为什么需要它：CI 的 rust job 都在仓库根跑，测试可以从仓库装机型表；而发布物
 * 只有随包数据，`presets/` 不在盘上。**库内已经没有「去仓库找」的备用源**
 * （见 `machine_dims.rs` 模块头），本探针验的是最后一步：真交付物在隔离根里
 * 能不能只靠随包数据跑完整钩子。
 *
 * 本探针把 exe 拿出来，在**不含 presets/ 的隔离数据根**里跑完整钩子：
 *   - 预设 / 3MF / gcode 都从仓库夹具复制到临时目录（不写仓库、不碰用户数据根）
 *   - 断言退出码、身份探测字段、校准问句
 *
 * ★ 校准件那一档需要**真界面**接问句（钩子会自己拉界面；命令行里没人答会等 110 秒）。
 *   先起界面再跑，问句就在真窗上弹、立刻能点：
 *
 *     # 终端 A：起真界面，数据根指向同一个隔离根
 *     MKP_SUPPORT_EASE_DATA=<isolated>/app-data npm run tauri dev
 *     # 终端 B：跑探针（--exe 指定上面同一次构建的 exe）
 *     node scripts/check/release-hook-probe.mjs <exe>
 *
 *   探针自己不拉界面、不起假 TCP 服务：连不上界面就**明确报错退出 2**，
 *   提示先起界面（免得弹「[::1] 拒绝连接」那种空窗，也免得等满 110 秒超时）。
 *
 * 用法：node scripts/check/release-hook-probe.mjs [exe 路径]
 * 默认 exe：target/debug/mkp-support-ease.exe（先 cargo build -p mkp-support-ease --bin mkp-support-ease）
 */

import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { dirname, resolve, join, basename } from 'node:path'
import { existsSync, mkdtempSync, mkdirSync, writeFileSync, copyFileSync, readFileSync, rmSync, readdirSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { createServer, connect as netConnect } from 'node:net'

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..')
const exe = resolve(process.argv[2] || join(ROOT, 'target/debug/mkp-support-ease.exe'))
const FIX = join(ROOT, 'crates/postprocess/tests/fixtures/dev')
const PRESET = join(ROOT, 'presets/delivery/mkp/presets/A1-fast261111.toml')
const MODEL_Z = join(ROOT, 'presets/assets/models/ZOffset_Calibration_ext.3mf')
const MODEL_COMBINED = join(ROOT, 'presets/assets/models/Calibration_Combined.3mf')

const fail = []
const ok = []
const check = (cond, msg) => (cond ? ok : fail).push(msg)

if (!existsSync(exe)) {
  console.error(`exe 不存在：${exe}\n先跑 cargo build -p mkp-support-ease --bin mkp-support-ease`)
  process.exit(2)
}
for (const [name, p] of [['预设', PRESET], ['Z 模型', MODEL_Z], ['二合一模型', MODEL_COMBINED], ['夹具目录', FIX]]) {
  if (!existsSync(p)) {
    console.error(`缺 ${name}：${p}`)
    process.exit(2)
  }
}

/** 等端点文件出现并可连（界面已起）；没有就返回 false。 */
function waitForEndpoint(dataRoot, ms = 20_000) {
  const portFile = join(dataRoot, 'run/hook.port')
  const deadline = Date.now() + ms
  while (Date.now() < deadline) {
    if (existsSync(portFile)) {
      const port = Number(readFileSync(portFile, 'utf8').trim())
      if (port > 0) {
        const probe = netConnect({ host: '127.0.0.1', port })
        let connected = false
        probe.on('connect', () => { connected = true; probe.end() })
        probe.on('error', () => {})
        // 同步等一小会儿让 connect 事件跑完
        const until = Date.now() + 200
        while (Date.now() < until && !connected) {
          // busy wait：只 200ms，端点探测用
        }
        if (connected) return true
      }
    }
    const until = Date.now() + 100
    while (Date.now() < until) { /* 等下一轮 */ }
  }
  return false
}

/** 跑一次钩子；返回 { code, stderr, meta, hookLog }。 */
function runHook({ label, gcode, origin, waitMs }) {
  // 所有档共用同一个数据根（界面 bind 的也是它）；slicer 临时目录每档独立。
  const root = mkdtempSync(join(tmpdir(), `mkp-probe-${label}-`))
  const dataRoot = DATA
  // 共享根里只清这一档会写的产物；run/hook.port 是界面 bind 的，留着。
  rmSync(join(dataRoot, 'user'), { recursive: true, force: true })
  rmSync(join(dataRoot, 'preset.toml'), { force: true })
  const slicerRoot = join(root, 'slicer')
  const metaDir = join(slicerRoot, 'Metadata')
  mkdirSync(metaDir, { recursive: true })
  mkdirSync(dataRoot, { recursive: true })

  const gcodeName = basename(gcode)
  const input = join(metaDir, gcodeName)
  copyFileSync(gcode, input)
  if (origin) writeFileSync(join(slicerRoot, 'origin.txt'), origin, 'utf8')

  const preset = join(dataRoot, 'preset.toml')
  copyFileSync(PRESET, preset)

  const env = { ...process.env }
  env.MKP_SUPPORT_EASE_DATA = dataRoot
  env.RUST_BACKTRACE = '1'

  const args = ['--Toml', preset, '--Gcode', 'on', 'file', input]
  const r = spawnSync(exe, args, { env, cwd: root, encoding: 'utf8', timeout: waitMs ?? 60_000 })
  const code = r.status ?? -1
  const stderr = `${r.stderr || ''}${r.stdout || ''}`

  const history = join(dataRoot, 'user/gcode_history')
  const days = existsSync(history) ? readdirSync(history).sort() : []
  const day = days.at(-1)
  const metas = day && existsSync(join(history, day))
    ? readdirSync(join(history, day)).filter((n) => n.endsWith('_meta.json'))
    : []
  const meta = metas.length ? JSON.parse(readFileSync(join(history, day, metas[0]), 'utf8')) : null

  const logDir = join(dataRoot, 'logs')
  const logFile = existsSync(logDir)
    ? readdirSync(logDir).find((n) => n.startsWith('hook.log.'))
    : undefined
  const hookLog = logFile ? readFileSync(join(logDir, logFile), 'utf8') : ''

  return { code, stderr, root, dataRoot, meta, input, hookLog }
}

function cleanup(r) {
  if (process.env.PROBE_KEEP === '1') {
    console.log(`  保留隔离根：${r.root}`)
    return
  }
  rmSync(r.root, { recursive: true, force: true })
}

function report(label, r) {
  console.log(`\n=== ${label} ===`)
  console.log(`  退出码=${r.code}  meta=${r.meta ? '有' : '无'}`)
  if (r.meta) {
    const ident = r.meta.identity ?? {}
    console.log(`  verdict=${ident.verdict ?? r.meta.verdict ?? '-'}  identity3mf=${ident.identity3mf ?? r.meta.identity3mf ?? '-'}`)
    console.log(`  error=${r.meta.error ?? 'null'}`)
  }
  if (r.code !== 0) console.log(`  stderr: ${r.stderr.trim().slice(0, 300)}`)
}

// 探针用独立隔离根；界面得先起并 bind 到同一个根才能接问句。
// 用固定的隔离根（不是每次随机），这样使用者可以先起界面、再多次跑探针。
const SHARED = join(tmpdir(), 'mkp-probe-shared')
const DATA = join(SHARED, 'app-data')
mkdirSync(DATA, { recursive: true })

console.log(`隔离数据根：${DATA}`)

// 严格先起界面再跑：钩子在任一档都会尝试连端点，连不上就自己拉一个 debug 界面，
// 那会因为没有 dev server 而显示「[::1] 拒绝连接」——既不真实也干扰使用。
// 所以这里先验端点，没起就**什么都不跑**。
if (!waitForEndpoint(DATA)) {
  console.error('\n界面还没起，探针退出（不会自己拉窗、不会弹空错误页）。')
  console.error('先在一个终端起真界面，数据根要与这里一致：')
  console.error(`  MKP_SUPPORT_EASE_DATA="${DATA}" npm run tauri dev`)
  console.error('等界面出现后再重跑本脚本。')
  process.exit(2)
}
console.log('✓ 界面端点已就绪\n')

// ---- ① 普通模型：带真界面跑通 ----
{
  const r = runHook({
    label: 'plain',
    gcode: join(FIX, 'small-ok.gcode'),
    origin: `${ROOT}/presets/assets/models/普通模型.3mf`, // 指向不存在的 3MF ⇒ Absent
  })
  report('普通模型', r)
  check(r.code !== 101, '普通模型：退出码不是 101（机型表已装）')
  check(r.meta !== null, '普通模型：写出 _meta.json')
  check(r.meta?.error == null, '普通模型：error=null（当普通模型跑通）')
  cleanup(r)
}

{
  // ---- ② ZOffset 校准：真界面接问句，点哪颗由使用者在窗上决定 ----
  {
    const r = runHook({
      label: 'zoffset',
      gcode: join(FIX, 'calib-zoffset-ext.gcode'),
      origin: MODEL_Z,
    })
    report('ZOffset 校准件', r)
    const raw = JSON.stringify(r.meta ?? {})
    check(r.code !== 101, 'ZOffset：退出码不是 101')
    check(raw.includes('from_3mf'), 'ZOffset：证据来自 3MF')
    check(
      !(r.meta?.error ?? '').includes('不是校准件') && !r.stderr.includes('E_CALIB_UNRECOGNIZED_001'),
      'ZOffset：识别为校准件（不再报「不是校准件」）'
    )
    cleanup(r)
  }

  // ---- ③ 二合一（回归现场）：同样走真界面问句 ----
  {
    const r = runHook({
      label: 'combined',
      gcode: join(FIX, 'calib-combined.gcode'),
      origin: MODEL_COMBINED,
    })
    report('二合一校准件', r)
    const raw = JSON.stringify(r.meta ?? {})
    check(r.code !== 101, '二合一：退出码不是 101')
    check(raw.includes('from_3mf') && raw.includes('Combined'), '二合一：身份 Combined（来自 3MF）')
    check(
      !(r.meta?.error ?? '').includes('不是校准件') && !r.stderr.includes('E_CALIB_UNRECOGNIZED_001'),
      '二合一：不再报「不是校准件」（档表已随包装入）'
    )
    cleanup(r)
  }
}

console.log(`\n通过 ${ok.length} 条：\n  - ${ok.join('\n  - ')}`)
if (fail.length) {
  console.error(`\n失败 ${fail.length} 条：\n  - ${fail.join('\n  - ')}`)
  process.exit(1)
}
console.log('\n✓ 发布物钩子探针通过')
