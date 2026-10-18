/**
 * 判据 —— **校准素材的 3MF 身份**（2026-10-19）。
 *
 * 校准件身份只有一处真值：模型里 `3D/3dmodel.model` 的
 * `<metadata name="MKPSE:Calibration">`。这条判据把它变成能在 CI 上红的检查：
 *
 *   - 台账（`presets/assets.toml`）里声明了 `calibration = '…'` 的模型，
 *     文件里必须真的是那个身份；
 *   - 没声明预期的模型会**提示**出来（多半是普通拼盘，不是错）；
 *   - `--scan` 还会报出 `assets/models` 下没登记进台账的 `.3mf`（防漏登记）。
 *
 * # 为什么这个脚本自己不解析 zip
 *
 * 作者定的分工：**检查脚本负责自动验收，工作台负责查看和管理，规则只有一套**。
 * 读 3MF 的代码只住在 Rust 的 `calibration_asset`（工作台与这里共用），
 * 这里再写一份 zip 解析就迟早两边对不上（"脚本说合格、工作台说没标记"）。
 * 所以本脚本只做一件事：调 `mkpse-pp check-assets`，把输出整理给人看。
 *
 * # 只读
 *
 * 这个脚本**绝不改文件** —— 发现不合格就红。改文件是工作台的事。
 *
 * 用法：
 *   npm run check:calibration-assets           # 验收台账里声明的那些
 *   npm run check:calibration-assets -- --scan # 顺带查漏登记
 */

import { spawnSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { join } from 'node:path'

const ROOT = process.cwd()
const MANIFEST = join(ROOT, 'presets', 'assets.toml')
const cratesDir = join(ROOT, 'crates', 'postprocess')

/** 调内核：`cargo run -q -p mkpse-postprocess --bin mkpse-pp -- check-assets …`
 *
 * 用 `cargo run` 而不是找现成二进制：判据要跟着源码走（改了没重编也能测出真结果），
 * 代价是一次增量编译（几秒）。
 */
function runCheck(extraArgs) {
  if (!existsSync(MANIFEST)) {
    console.error(`✗ 找不到 ${MANIFEST} —— 请在仓库根运行这个脚本`)
    process.exit(2)
  }

  const args = [
    'run',
    '-q',
    '-p',
    'mkpse-postprocess',
    '--bin',
    'mkpse-pp',
    '--',
    'check-assets',
    '--manifest',
    MANIFEST,
    ...extraArgs,
  ]

  const res = spawnSync('cargo', args, {
    cwd: cratesDir,
    encoding: 'utf8',
    // 台账路径是绝对路径，工作目录在 crate 里也不影响
    maxBuffer: 8 * 1024 * 1024,
  })

  if (res.error) {
    console.error(`✗ 调不动 cargo：${res.error.message}`)
    process.exit(2)
  }

  return {
    code: res.status ?? 1,
    rows: parseRows(res.stdout ?? ''),
    log: (res.stderr ?? '').trimEnd(),
  }
}

/** stdout 每行：`path \t 预期 \t 实际 \t 结论 \t 说明` */
function parseRows(stdout) {
  return stdout
    .split('\n')
    .map((l) => l.trimEnd())
    .filter((l) => l.includes('\t'))
    .map((l) => {
      const [path, expected, actual, verdict, note] = l.split('\t')
      return { path, expected, actual, verdict, note }
    })
}

const VERDICT_TEXT = {
  ok: '合格',
  mismatch: '身份与预期不一致',
  invalid: '身份值不合法（只认 ZOffset / Precise / Rough / Repetition）',
  missing: '缺 MKPSE:Calibration 标记',
  unreadable: '文件读不出来或 3MF 结构异常',
}

function main() {
  const scan = process.argv.includes('--scan')
  const { code, rows, log } = runCheck(scan ? ['--scan'] : [])

  /* 内核的 stderr 里已经有摘要与逐条原因 —— 原样转出，别在这里重新拼一遍 */
  if (log) console.log(log)

  if (code === 0) {
    return 0
  }

  /* 失败时补一句"这些该去哪修"，把修复入口说清楚（自查脚本本身不改文件） */
  const bad = rows.filter((r) => r.verdict !== 'ok')
  if (bad.length > 0) {
    console.error('')
    console.error(`✗ ${bad.length} 份校准素材不合格：`)
    for (const r of bad) {
      console.error(`   ${r.path}`)
      console.error(`     ${VERDICT_TEXT[r.verdict] ?? r.verdict}`)
      console.error(`     预期 ${r.expected}，文件里是 ${r.actual}`)
      if (r.note && r.note !== '-') console.error(`     ${r.note}`)
    }
    console.error('')
    console.error('   修法二选一：')
    console.error('   · 工作台 → 资产 → 该模型 → 「校准身份」里补标 / 改写')
    console.error('   · 如果文件本来就不该带标记，把台账里那条的 calibration 去掉')
  }

  return code === 0 ? 0 : 1
}

process.exitCode = main()
