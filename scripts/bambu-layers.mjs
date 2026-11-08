#!/usr/bin/env node
/**
 * Bambu Studio 会把这个 G-code 数成几层（2026-11-08）。
 *
 * 为什么需要它：预览里的层数**不是文件里任何一个声明**，是 Bambu 自己算出来的，
 * 而且有两套算法，开关是切片器写进配置块的 `has_scarf_joint_seam`：
 *
 *   平台段口径  每次挤出移动的 Z 与当前层 Z 差 > EPSILON(1e-4) 就开一层
 *               src/slic3r/GUI/GCodeRenderer/AdvancedRenderer.cpp:947-960
 *   tag 口径    每个 `; CHANGE_LAYER` 开一层，层的 Z 取该层内挤出移动 Z 的**最大值**
 *               src/libslic3r/GCode/GCodeProcessor.cpp:3522-3535（开层）
 *                                            :4409-4421（取 max）
 *   开关        has_scarf_joint_seam=1 ⇒ tag 口径（GCodeProcessor.cpp:2054-2056）
 *
 * 两套口径都只认 `EMoveType::Extrude`：`E>0` **且有 X 或 Y** 位移
 * （`GCodeProcessor.cpp:4046-4066` 的 move_type()）。`E>0` 但只有 Z 的算 Travel，
 * 所以纯抬笔不会被数成一层。
 *
 * 用法：
 *   node scripts/bambu-layers.mjs <file.gcode> [--json]
 *
 * 退出码：0 = 读通；2 = 参数/读文件出错。
 */

import { readFileSync } from 'node:fs'

const EPSILON = 1e-4 // libslic3r.h:52
const LAYER_CHANGE_TAG = '; CHANGE_LAYER'
const MODE_KEY = 'has_scarf_joint_seam'

function parseArgs(argv) {
  const out = { file: '', json: false }
  for (const a of argv) {
    if (a === '--json') out.json = true
    else if (!out.file) out.file = a
  }
  return out
}

/** `; has_scarf_joint_seam = 1` → true / false / null（没写就是 null）。 */
function readScarfSwitch(text) {
  const m = text.match(
    new RegExp(`^;\\s*${MODE_KEY}\\s*=\\s*(\\S+)`, 'm'),
  )
  if (!m) return null
  return m[1] === '1' || m[1].toLowerCase() === 'true'
}

/**
 * 逐行解析。返回每条 `EMoveType::Extrude` 的 (当前层序号, Z)。
 *
 * 位置：XYZ 绝对（`G90`），E 相对（`M83`）—— 与切片器产物一致；
 * 遇到 `G91`/`M82` 就地改状态，不假设整份文件同一档。
 */
function scanMoves(text) {
  const lines = text.split('\n')
  const extrudes = [] // { layer, z }
  let x = 0
  let y = 0
  let z = 0
  let wiping = false
  let relativeXyz = false
  let relativeE = true
  let layer = -1

  for (const raw of lines) {
    const line = raw.trim()
    if (!line) continue

    if (line === '; WIPE_START') {
      wiping = true
      continue
    }
    if (line === '; WIPE_END') {
      wiping = false
      continue
    }
    if (line === LAYER_CHANGE_TAG) {
      layer += 1
      continue
    }
    if (line[0] === ';') continue

    const code = line.split(';')[0].trim()
    const cmd = code.split(/\s+/, 1)[0]
    if (cmd === 'G90') {
      relativeXyz = false
      continue
    }
    if (cmd === 'G91') {
      relativeXyz = true
      continue
    }
    if (cmd === 'M82') {
      relativeE = false
      continue
    }
    if (cmd === 'M83') {
      relativeE = true
      continue
    }
    // G0 走的是同一个处理函数（`process_G0` → `process_G1`，GCodeProcessor.cpp:4019-4022）
    if (cmd !== 'G0' && cmd !== 'G1' && cmd !== 'G2' && cmd !== 'G3') continue

    let nx = null
    let ny = null
    let nz = null
    let de = 0
    for (const tok of code.split(/\s+/).slice(1)) {
      const head = tok[0]
      const v = Number(tok.slice(1))
      if (!Number.isFinite(v)) continue
      if (head === 'X') nx = relativeXyz ? x + v : v
      else if (head === 'Y') ny = relativeXyz ? y + v : v
      else if (head === 'Z') nz = relativeXyz ? z + v : v
      else if (head === 'E') de = relativeE ? v : v - de
    }

    const dx = nx === null ? 0 : nx - x
    const dy = ny === null ? 0 : ny - y
    const dz = nz === null ? 0 : nz - z
    if (nx !== null) x = nx
    if (ny !== null) y = ny
    if (nz !== null) z = nz

    // move_type()：只有「E>0 且有 X/Y 位移」才是 Extrude。
    if (wiping) continue
    if (de <= 0) continue
    if (dx === 0 && dy === 0) continue

    extrudes.push({ layer, z })
  }

  return extrudes
}

/** 平台段口径：挤出 Z 每变一次就开一层。 */
function countPlateauLayers(extrudes) {
  const zs = []
  for (const m of extrudes) {
    if (zs.length === 0 || Math.abs(m.z - zs[zs.length - 1]) > EPSILON) zs.push(m.z)
  }
  return zs
}

/** tag 口径：`; CHANGE_LAYER` 开层，层 Z 取层内挤出 Z 的最大值。 */
function tagLayers(extrudes, layerCount) {
  const zs = new Array(layerCount).fill(null)
  for (const m of extrudes) {
    if (m.layer < 0 || m.layer >= layerCount) continue
    zs[m.layer] = zs[m.layer] === null ? m.z : Math.max(zs[m.layer], m.z)
  }
  return zs
}

/** 层 Z 是否单调不减；返回第一处回退。 */
function firstBackstep(zs) {
  for (let i = 1; i < zs.length; i += 1) {
    if (zs[i] < zs[i - 1] - EPSILON) {
      return { at: i, prev: zs[i - 1], value: zs[i] }
    }
  }
  return null
}

function main() {
  const args = parseArgs(process.argv.slice(2))
  if (!args.file) {
    console.error('用法：node scripts/bambu-layers.mjs <file.gcode> [--json]')
    process.exit(2)
  }

  let text
  try {
    text = readFileSync(args.file, 'utf8')
  } catch (e) {
    console.error(`读不到文件：${args.file}（${e.message}）`)
    process.exit(2)
  }

  const layerChanges = text.split('\n').filter((l) => l.trim() === LAYER_CHANGE_TAG).length
  const scarf = readScarfSwitch(text)
  const extrudes = scanMoves(text)

  const plateau = countPlateauLayers(extrudes)
  const tagged = tagLayers(extrudes, layerChanges)

  const report = {
    file: args.file,
    // Bambu 读的是配置块里这一格：有斜接缝就走 tag 口径，没有就走平台段口径。
    scarfJointSeam: scarf,
    bambuMode: scarf ? 'tag' : 'plateau',
    changeLayerTags: layerChanges,
    extrudeMoves: extrudes.length,
    // 平台段口径的层数（没有斜接缝时就是界面上的那个数）。
    plateauLayers: plateau.length,
    // tag 口径的层数（有斜接缝时就是界面上的那个数）。
    tagLayers: tagged.filter((z) => z !== null).length,
    // 两套口径各自给出的层 Z 是否单调 —— 不单调时 Bambu 的层滑块会切错内容。
    plateauBackstep: firstBackstep(plateau),
    tagBackstep: firstBackstep(tagged.filter((z) => z !== null)),
    zRange: {
      first: plateau.length ? plateau[0] : null,
      last: plateau.length ? plateau[plateau.length - 1] : null,
    },
  }

  if (args.json) {
    console.log(JSON.stringify(report, null, 2))
    return
  }

  const mode = report.bambuMode === 'tag' ? 'tag（有斜接缝）' : '平台段（无斜接缝）'
  console.log(`文件：${args.file}`)
  console.log(`  ${MODE_KEY} = ${scarf === null ? '未声明' : scarf ? 1 : 0}`)
  console.log(`  Bambu 会走：${mode} ⇒ 界面层数 ≈ ${scarf ? report.tagLayers : report.plateauLayers}`)
  console.log(`  两种口径：平台段 ${report.plateauLayers} 层 / tag ${report.tagLayers} 层`)
  console.log(`  \`; CHANGE_LAYER\` ${report.changeLayerTags} 条 / 挤出移动 ${report.extrudeMoves} 条`)
  console.log(`  层 Z 范围 ${report.zRange.first} .. ${report.zRange.last}`)
  for (const [label, at] of [
    ['平台段', report.plateauBackstep],
    ['tag', report.tagBackstep],
  ]) {
    if (!at) {
      console.log(`  ${label} 层 Z：单调不减`)
      continue
    }
    console.log(
      `  ${label} 层 Z：第 ${at.at} 处回退 ${at.prev} → ${at.value}（非单调，Bambu 会切错内容）`,
    )
  }
}

main()
