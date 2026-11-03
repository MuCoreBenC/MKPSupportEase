#!/usr/bin/env node
/**
 * 涂胶动作体检（2026-11-03）。
 *
 * 为什么是脚本而不是文档：`docs/GLUE-PATH-CONTINUITY-2026-11-03.md` 里那些结论，
 * 每一条都是「肉眼在预览里看出来的」。人眼会累、会漏，规则不会 —— 这个脚本就是
 * 那几条结论的**机械实现**：喂一份导出的 .gcode，逐条判，报行号。
 *
 * 判据全部只看 G-code 自身（`Z` 的升降、坐标是否移动），不看注释、不看 E、不用拍脑袋的距离：
 *
 *   R1 白落  落笔后没有任何有效位移，紧接着又抬起来         —— 多一套抬/落
 *   R2 贴面飞 落笔状态下一段 XY 位移 > 5mm                   —— 笔尖压着表面走
 *   R3 白抬  抬起来之后整段空走的净位移 = 0                  —— 抬了却没走
 *   R4 丢圆弧 涂胶块内一条 G2/G3 都没有                      —— 路径被拉直（需 --original 才判）
 *
 * 用法：
 *   node scripts/glue-check.mjs <导出.gcode> [--original <原始.gcode>] [--json]
 *
 * 退出码：0 = 全部通过；1 = 有违规（CI 可挂）；2 = 参数/读文件出错。
 */

import { readFileSync } from 'node:fs'

const LANDING_TRAVEL_MM = 5 // R2 的「长」——只是体检阈值，不是生成算法
const EPS = 1e-6

function parseArgs(argv) {
  const out = { file: '', original: '', json: false }
  for (let i = 0; i < argv.length; i += 1) {
    const a = argv[i]
    if (a === '--original') {
      out.original = argv[i + 1] ?? ''
      i += 1
    } else if (a === '--json') {
      out.json = true
    } else if (!out.file) {
      out.file = a
    }
  }
  return out
}

function num(tok, lead) {
  if (!tok || tok[0] !== lead) return null
  const v = Number(tok.slice(1))
  return Number.isFinite(v) ? v : null
}

/** 逐行走一遍，产出「涂胶块内」的移动与 Z 事件。 */
function scan(text) {
  const lines = text.split('\n')
  const events = []
  let inPaint = false
  let block = -1
  let x = 0
  let y = 0
  let z = NaN
  let layer = NaN

  for (let i = 0; i < lines.length; i += 1) {
    const s = lines[i].trim()
    if (s.startsWith(';MKP_BEGIN paint')) {
      inPaint = true
      block += 1
      const m = /z=([\d.]+)/.exec(s)
      layer = m ? Number(m[1]) : NaN
      continue
    }
    if (s.startsWith(';MKP_END paint') || s.startsWith(';MKP_BEGIN swap')) {
      inPaint = false
      continue
    }
    if (!inPaint) continue

    const code = s.split(';')[0].trim()
    const cmd = code.split(/\s+/)[0]
    if (cmd !== 'G0' && cmd !== 'G1' && cmd !== 'G2' && cmd !== 'G3') continue

    let nx = null
    let ny = null
    let nz = null
    for (const tok of code.split(/\s+/).slice(1)) {
      const vx = num(tok, 'X')
      if (vx !== null) nx = vx
      const vy = num(tok, 'Y')
      if (vy !== null) ny = vy
      const vz = num(tok, 'Z')
      if (vz !== null) nz = vz
    }

    if (nz !== null) {
      const dir = nz > z + EPS ? 'up' : nz < z - EPS ? 'down' : 'same'
      z = nz
      events.push({ kind: 'z', line: i + 1, z: nz, dir, layer, block })
      continue
    }
    if (nx === null && ny === null) continue

    const px = nx ?? x
    const py = ny ?? y
    events.push({
      kind: 'xy',
      line: i + 1,
      x: px,
      y: py,
      fromX: x,
      fromY: y,
      dist: Math.hypot(px - x, py - y),
      arc: cmd === 'G2' || cmd === 'G3',
      layer,
      block,
    })
    x = px
    y = py
  }
  return events
}

function check(events) {
  const findings = []
  // 每个涂胶块的「落笔高度」= 块内最低 Z。逐块先扫一遍，避免把
  // 「63.71 → 60.71」这种段间抬升当成落笔（60.71 其实是抬升高度）。
  const landByLayer = new Map()
  const lastXyOfBlock = new Map()
  for (const e of events) {
    if (e.kind === 'xy') lastXyOfBlock.set(e.block, e.line)
  }

  for (const e of events) {
    if (e.kind !== 'z') continue
    const cur = landByLayer.get(e.layer)
    landByLayer.set(e.layer, cur === undefined ? e.z : Math.min(cur, e.z))
  }
  const isLandedZ = (e) => {
    const land = landByLayer.get(e.layer)
    return land !== undefined && e.z <= land + 1e-6
  }
  let landed = false
  let land = null // 当前这次落笔：{ line, moved }
  let lift = null // 当前这次抬笔：{ line, moved }
  let arcCount = 0

  // 只有「落笔之后又抬起来」才算白落。涂胶块结尾那次落笔是必须的
  // （`;Glueing Finished` 之前要落），不算多余，所以 `end` 不报。
  const closeLand = (why) => {
    if (why !== 'end' && land && land.moved < EPS) {
      findings.push({
        rule: 'R1',
        line: land.line,
        msg: `落笔后没有任何有效移动就${why === 'up' ? '又抬起来' : '结束'} —— 多一套抬/落`,
      })
    }
    land = null
  }

  for (const e of events) {
    if (e.kind === 'z') {
      if (e.dir === 'down' && isLandedZ(e)) {
        closeLand('down')
        if (lift) {
          if (lift.moved < EPS) {
            findings.push({
              rule: 'R3',
              line: lift.line,
              msg: `抬升后净位移 0（在第 ${e.line} 行落回）—— 白抬一次`,
            })
          }
          lift = null
        }
        landed = true
        land = { line: e.line, moved: 0 }
      } else if (e.dir === 'up') {
        // 块内最后一次抬升是收笔（`;Glueing Finished` 之前必须抬），不算白落
        const tail = lastXyOfBlock.get(e.block)
        closeLand(tail !== undefined && tail < e.line ? 'end' : 'up')
        landed = false
        lift = { line: e.line, moved: 0 }
      }
      continue
    }

    if (e.arc) arcCount += 1
    if (lift) lift.moved += e.dist
    if (land) land.moved += e.dist

    // 圆弧的「弦长」不是位移：笔是沿弧走的，弧再长也不算贴面飞。
    if (landed && !e.arc && e.dist > LANDING_TRAVEL_MM) {
      findings.push({
        rule: 'R2',
        line: e.line,
        msg: `落笔高度横移 ${e.dist.toFixed(1)}mm（层顶 ${e.layer}）—— 笔尖压着表面走`,
      })
    }
  }
  closeLand('end')
  return { findings, arcCount }
}

function main() {
  const args = parseArgs(process.argv.slice(2))
  if (!args.file) {
    console.error('用法：node scripts/glue-check.mjs <导出.gcode> [--original <原始.gcode>] [--json]')
    process.exit(2)
  }

  let text
  try {
    text = readFileSync(args.file, 'utf8')
  } catch (err) {
    console.error(`读不到文件：${args.file}（${err.message}）`)
    process.exit(2)
  }

  const events = scan(text)
  const { findings, arcCount } = check(events)

  // R4：涂胶块内一条圆弧都没有 —— 支撑面里本来就有 G2/G3，全丢说明路径被拉直了
  if (arcCount === 0 && events.some((e) => e.kind === 'xy')) {
    findings.push({
      rule: 'R4',
      line: 0,
      msg: '涂胶块内一条 G2/G3 都没有 —— 圆弧被丢掉、路径被拉直（对照原始件确认）',
    })
  }

  const blocks = text.split('\n').filter((l) => l.trim().startsWith(';MKP_BEGIN paint')).length
  const order = { R1: 0, R2: 1, R3: 2, R4: 3 }
  findings.sort((a, b) => a.line - b.line || order[a.rule] - order[b.rule])

  if (args.json) {
    console.log(JSON.stringify({ file: args.file, blocks, arcCount, findings }, null, 2))
  } else {
    console.log(`文件：${args.file}`)
    console.log(`涂胶块 ${blocks} 个，块内圆弧 ${arcCount} 条`)
    if (findings.length === 0) {
      console.log('通过：没有发现多余抬落 / 贴面横移 / 丢圆弧')
    } else {
      const byRule = {}
      for (const f of findings) byRule[f.rule] = (byRule[f.rule] ?? 0) + 1
      console.log(
        `问题 ${findings.length} 条：` +
          Object.entries(byRule)
            .sort()
            .map(([k, v]) => `${k}=${v}`)
            .join('  ')
      )
      for (const f of findings.slice(0, 60)) {
        console.log(`  [${f.rule}] 第 ${f.line} 行：${f.msg}`)
      }
      if (findings.length > 60) console.log(`  …… 其余 ${findings.length - 60} 条略（--json 看全量）`)
    }
  }

  process.exit(findings.length === 0 ? 0 : 1)
}

main()
