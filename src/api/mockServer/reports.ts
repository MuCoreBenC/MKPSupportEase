/*
 * 报告页（后处理执行报告与历史）的**演示夹具**。
 *
 * 产品里这棵账是钩子落在 `<用户根>/gcode_history/<日期>/` 下的 `_meta.json` 树，
 * 纯前端探索仓里没有文件系统 —— 按作者 2026-10-10 裁决，**照真机 2026-10-10 的
 * 真实记录原样做成静态夹具**：报告页很少有人看，先原原本本搬进预览里，以后再改。
 *
 * 三条规矩：
 * 1. 记录顺序、文件名、时间、耗时、成功/失败文案逐字照截图，不自己编一套；
 * 2. 首条（`.42952.1.gcode`）的详情逐字段照真机（统计 / 打印时间 / 管线计时 / 文件）；
 * 3. 其余记录给同形状的详情（按本行字段合成），展开任何一行都不是空的。
 */
import type { ReportDetail, ReportPipelineStep, ReportSummary } from '../contract'

/** 真机上的用户数据根（路径也照真机写，排查时才像真的） */
const USER_ROOT = '/Users/wzy/Library/Application Support/SupportEase/user'
const TEMP_ROOT =
  '/var/folders/xq/p1zkn4ms2klgwpvrh2nwhsh40000gq/T/bamboo_model'
const DAY = '2026-10-10'
/** DAY 去掉分隔符的形式（记录 id 与输出文件名里用它） */
const DAY_COMPACT = DAY.replace(/-/g, '')

const CANCEL_ERROR = '已取消 [E_SYS_CANCELLED_001]：原文件没有被改动'

/** 一行记录的输入（顺序就是列表顺序：最新在前） */
interface FixtureRow {
  startedAt: string
  gcode: string
  preset: string
  machine: string
  elapsedMs: number
  ok: boolean
  /** 顺序号（真机 temp 目录里的 #456 那种），用来拼假路径 */
  seq: number
}

const rows: FixtureRow[] = [
  { startedAt: '11:17:11', gcode: '.42952.1.gcode', preset: 'A1_MINI-fastv3.3.toml', machine: 'A1_MINI', elapsedMs: 19000, ok: true, seq: 456 },
  { startedAt: '11:04:48', gcode: '.42952.0.gcode', preset: 'A1_MINI-fastv3.3.toml', machine: 'A1_MINI', elapsedMs: 238, ok: true, seq: 448 },
  { startedAt: '11:02:33', gcode: '.26212.1.gcode', preset: 'A1_MINI-fastv3.3.toml', machine: 'A1_MINI', elapsedMs: 524, ok: true, seq: 441 },
  { startedAt: '11:02:12', gcode: '.26212.0.gcode', preset: 'A1_MINI-fastv3.3.toml', machine: 'A1_MINI', elapsedMs: 244, ok: true, seq: 439 },
  { startedAt: '10:51:19', gcode: 'ref.gcode', preset: 'A1_MINI-fast.toml', machine: 'A1_MINI', elapsedMs: 891, ok: true, seq: 423 },
  { startedAt: '10:51:19', gcode: 'a.gcode', preset: 'A1-fast.toml', machine: 'A1', elapsedMs: 63, ok: false, seq: 421 },
  { startedAt: '10:51:19', gcode: 'a1mini.gcode', preset: 'A1-fast.toml', machine: 'A1', elapsedMs: 92, ok: true, seq: 420 },
  { startedAt: '10:50:40', gcode: 'ref.gcode', preset: 'A1_MINI-fast.toml', machine: 'A1_MINI', elapsedMs: 836, ok: true, seq: 418 },
  { startedAt: '10:50:40', gcode: 'a.gcode', preset: 'A1-fast.toml', machine: 'A1', elapsedMs: 75, ok: false, seq: 417 },
  { startedAt: '10:50:40', gcode: 'a1mini.gcode', preset: 'A1-fast.toml', machine: 'A1', elapsedMs: 109, ok: true, seq: 416 },
]

/** `.42952.1.gcode` → `42952.1`（真机记录 id 的基名部分） */
function baseName(gcode: string): string {
  return gcode.replace(/^\./, '').replace(/\.gcode$/i, '')
}

function compactStamp(hm: string): string {
  return hm.replace(/:/g, '')
}

function idOf(r: FixtureRow): string {
  return `${baseName(r.gcode)}_${DAY.replace(/-/g, '')}_${compactStamp(r.startedAt)}`
}

/** 确定性假 SHA：同一行永远同值，看起来像真 sha（页面只截前 12 位） */
function fakeSha(seed: number, salt: string): string {
  let h = seed * 2654435761 + salt.length * 40503
  const out: string[] = []
  for (let i = 0; i < 64; i++) {
    h = (Math.imul(h ^ (h >>> 13), 1274126177)) >>> 0
    h = (h + i * 97 + salt.charCodeAt(i % salt.length)) >>> 0
    out.push('0123456789abcdef'[h % 16])
  }
  return out.join('')
}

function toSummary(r: FixtureRow): ReportSummary {
  return {
    id: idOf(r),
    day: DAY,
    startedAt: `${DAY} ${r.startedAt}`,
    finishedAt: `${DAY} ${r.startedAt}`,
    elapsedMs: r.elapsedMs,
    presetName: r.preset,
    machineType: r.machine,
    gcodeName: r.gcode,
    ok: r.ok,
    error: r.ok ? null : CANCEL_ERROR,
    warningCount: 0,
  }
}

export function reportSummaries(): ReportSummary[] {
  return rows.map(toSummary)
}

/* ---------- 详情 ---------- */

const PIPELINE_META: { id: string; msg: string; frac: number }[] = [
  { id: 'input', msg: '正在读取文件…', frac: 0.095 },
  { id: 'config', msg: '配置已就绪（IR 由调用方给出）', frac: 0 },
  { id: 'machine-detect', msg: '识别机型…', frac: 0.014 },
  { id: 'parse', msg: '解析 G-code…', frac: 0.068 },
  { id: 'support', msg: '支撑面检测…', frac: 0 },
  { id: 'collision', msg: '碰撞检测…', frac: 0.068 },
  { id: 'pass1', msg: '第一遍扫描: 99%', frac: 0.274 },
  { id: 'pass2', msg: '第二遍扫描: 99%', frac: 0.41 },
  { id: 'write', msg: '正在写入文件…', frac: 0.049 },
]

/** 按本行耗时按比例合成管线计时（config / support 真机上就是 0） */
function pipelineFor(r: FixtureRow): ReportPipelineStep[] {
  return PIPELINE_META.map((p) => ({
    id: p.id,
    message: p.msg,
    status: 'ok',
    elapsedMs: Math.round(r.elapsedMs * p.frac),
  }))
}

/** 首条记录的详情 —— 逐字段照真机截图（不合成） */
function firstDetail(summary: ReportSummary, r: FixtureRow): ReportDetail {
  const inputPath =
    `${TEMP_ROOT}/Sat_Oct_10/11_05_12#42952#456/Metadata/${r.gcode}`
  const outputPath =
    `${USER_ROOT}/gcode_history/${DAY}/42952.1_${DAY_COMPACT}_111711.gcode`
  const presetPath = `${USER_ROOT}/presets-mine/${r.preset}`

  return {
    summary,
    inputFile: {
      path: inputPath,
      sizeBytes: Math.round(27.3 * 1024 * 1024),
      lines: 1241690,
      sha256: `6a2398dba1db${fakeSha(r.seq, r.gcode).slice(12)}`,
    },
    outputFile: {
      path: outputPath,
      sizeBytes: Math.round(27.9 * 1024 * 1024),
      sha256: `d0c556cace79${fakeSha(r.seq + 7, r.gcode).slice(12)}`,
    },
    header: {
      machine: r.machine,
      preset: r.preset,
    },
    stats: {
      towerHeight: 0,
      glueLayerCount: 109,
      totalLayerNumber: 633,
      maxZHeight: 67,
      originalPrintTime: '3h 16m 52s; total estimated time: 3h 22m 55s',
    },
    /* 页面显示「3 小时 39 分」= 13140 秒 */
    printTime: {
      totalSeconds: 13140,
      segments: 893809,
    },
    printTimeStatus: 'ready',
    pipeline: [
      { id: 'input', message: '正在读取文件…', status: 'ok', elapsedMs: 1800 },
      { id: 'config', message: '配置已就绪（IR 由调用方给出）', status: 'ok', elapsedMs: 0 },
      { id: 'machine-detect', message: '识别机型…', status: 'ok', elapsedMs: 255 },
      { id: 'parse', message: '解析 G-code…', status: 'ok', elapsedMs: 1300 },
      { id: 'support', message: '支撑面检测…', status: 'ok', elapsedMs: 0 },
      { id: 'collision', message: '碰撞检测…', status: 'ok', elapsedMs: 1300 },
      { id: 'pass1', message: '第一遍扫描: 99%', status: 'ok', elapsedMs: 5200 },
      { id: 'pass2', message: '第二遍扫描: 99%', status: 'ok', elapsedMs: 7800 },
      { id: 'write', message: '正在写入文件…', status: 'ok', elapsedMs: 933 },
    ],
    warnings: [],
    invocation: {
      exe: '/Users/wzy/projects/MKPSupportEase/target/debug/mkp-support-ease',
      args: ['--Toml', presetPath, '--Gcode', inputPath],
    },
    presetPath,
    gcodePath: inputPath,
  }
}

/** 其余记录：同形状，按本行字段合成 */
function genericDetail(summary: ReportSummary, r: FixtureRow): ReportDetail {
  const stamp = compactStamp(r.startedAt)
  const inputPath =
    `${TEMP_ROOT}/Sat_Oct_10/${r.startedAt.replace(/:/g, '_')}#${r.seq}/Metadata/${r.gcode}`
  const outputPath =
    `${USER_ROOT}/gcode_history/${DAY}/${baseName(r.gcode)}_${DAY_COMPACT}_${stamp}.gcode`
  const presetPath = `${USER_ROOT}/presets-mine/${r.preset}`

  /* 文件大小/行数随序号变化，纯粹为了让每行不一样 */
  const inputMb = 0.5 + ((r.seq % 9) * 3.4)
  const lines = 1200 + ((r.seq * 137) % 26000)

  return {
    summary,
    inputFile: {
      path: inputPath,
      sizeBytes: Math.round(inputMb * 1024 * 1024),
      lines,
      sha256: fakeSha(r.seq, r.gcode),
    },
    outputFile: {
      path: outputPath,
      sizeBytes: Math.round(inputMb * 1.02 * 1024 * 1024),
      sha256: fakeSha(r.seq + 3, r.gcode),
    },
    header: {
      machine: r.machine,
      preset: r.preset,
    },
    stats: r.ok
      ? {
          towerHeight: r.seq % 3,
          glueLayerCount: 2 + (r.seq % 40),
          totalLayerNumber: 30 + ((r.seq * 13) % 600),
          maxZHeight: 8 + ((r.seq * 7) % 60),
        }
      : {},
    /* 成功记录给一个估算；取消的记录没有（页面显示「—」） */
    printTime: r.ok
      ? { totalSeconds: 120 + ((r.seq * 211) % 12000), segments: 1000 + ((r.seq * 977) % 80000) }
      : null,
    printTimeStatus: r.ok ? 'ready' : null,
    pipeline: pipelineFor(r),
    warnings: [],
    invocation: {
      exe: '/Users/wzy/projects/MKPSupportEase/target/debug/mkp-support-ease',
      args: ['--Toml', presetPath, '--Gcode', inputPath],
    },
    presetPath,
    gcodePath: inputPath,
  }
}

export function reportDetail(id: string): ReportDetail {
  const idx = rows.findIndex((r) => idOf(r) === id)
  const r = rows[Math.max(idx, 0)]
  const summary = toSummary(r)
  return idx === 0
    ? firstDetail(summary, r)
    : genericDetail(summary, r)
}
