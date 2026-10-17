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
 *    涂胶 / 擦料塔 / 切片工艺三组与 `estimatedPrintSeconds` 是一套自洽样本
 *    （真机那一刻的记录里还没有这几组，形状照 `docs/REPORT-DETAIL-FIELDS.md`）；
 * 3. 其余记录给同形状的详情（按本行字段合成），展开任何一行都不是空的。
 */
import type {
  ReportDetail,
  ReportExportKind,
  ReportExportPlan,
  ReportPipelineStep,
  ReportSummary,
} from '../contract'

/** 真机上的用户数据根（路径也照真机写，排查时才像真的） */
const USER_ROOT = '/Users/wzy/Library/Application Support/SupportEase/user'
const TEMP_ROOT =
  '/var/folders/xq/p1zkn4ms2klgwpvrh2nwhsh40000gq/T/bamboo_model'
const DAY = '2026-10-10'
/** DAY 去掉分隔符的形式（记录 id 与输出文件名里用它） */
const DAY_COMPACT = DAY.replace(/-/g, '')

/**
 * 失败 / 取消那一行的话（`_meta.json` 的 `error` 字段）。
 *
 * ★ 2026-10-11 起：这句话由 Rust 侧的文案表（`crates/postprocess/messages.toml`）渲染后
 * 写进记录，前端**不再自己拼**。形态是「结论：原因」——
 * 「已取消」这一档**不带错误码**（用户自己按的停止不是错误，看门狗那种系统叫停才带）。
 */
const CANCEL_ERROR = '已取消：这一盘没有做后处理，原文件没有被改动'

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
  /**
   * 3MF 模型名（`_meta.json` 顶层 `modelName`；钩子在切片器临时目录里从
   * `origin.txt` / 同名 zip / `_temp_1.config` 三级兜底取的）。
   *
   * 缺省 = 这一行**取不到**（老记录，或 G-code 已被搬出临时目录）——
   * 页面那时只显示 G-code 名，不编名字。夹具里两态都留着，预览里能一眼看出区别。
   */
  modelName?: string
}

const rows: FixtureRow[] = [
  { startedAt: '11:17:11', gcode: '.42952.1.gcode', preset: 'A1_MINI-fastv3.3.toml', machine: 'A1_MINI', elapsedMs: 19000, ok: true, seq: 456, modelName: '帕姆' },
  { startedAt: '11:04:48', gcode: '.42952.0.gcode', preset: 'A1_MINI-fastv3.3.toml', machine: 'A1_MINI', elapsedMs: 238, ok: true, seq: 448, modelName: '帕姆' },
  { startedAt: '11:02:33', gcode: '.26212.1.gcode', preset: 'A1_MINI-fastv3.3.toml', machine: 'A1_MINI', elapsedMs: 524, ok: true, seq: 441, modelName: 'deepfish' },
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
    modelName: r.modelName ?? null,
    ok: r.ok,
    error: r.ok ? null : CANCEL_ERROR,
    warningCount: 0,
  }
}

export function reportSummaries(): ReportSummary[] {
  return rows.map(toSummary)
}

/* ---------- 导出（真机上是 Rust 侧的活，见 `src-tauri/src/ipc/report.rs`） ---------- */

/**
 * 三件套的后缀 + 机器名。**与 Rust 侧 `EXPORT_PARTS` 同一份口径**（那边是权威，
 * 这里只是让预览里的文件名长得一样）。
 */
const EXPORT_SUFFIX: Record<ReportExportKind, string> = {
  original: '_original.gcode',
  processed: '.gcode',
  meta: '_meta.json',
}
const EXPORT_KINDS: ReportExportKind[] = ['original', 'processed', 'meta']

/** `<原名>_<YYYYMMDD>_<HHMMSS>` 尾巴上那 14 位时刻 */
function stampOf(r: FixtureRow): string {
  return idOf(r)
    .slice(-16)
    .replace(/_/g, '')
}

/**
 * 「导出执行报告」的计划。口径照 Rust 侧那条：
 * 有模型名 ⇒ `<模型名>_<时刻>`，没有 ⇒ 归档自己那串；**失败的那一次没有「处理后的文件」**
 * （真机上 `<id>.gcode` 根本没落盘），照实报缺，不假装齐。
 */
export function reportExportPlan(id: string): ReportExportPlan {
  const idx = rows.findIndex((r) => idOf(r) === id)
  const r = rows[Math.max(idx, 0)]
  const base = r.modelName === undefined ? idOf(r) : `${r.modelName}_${stampOf(r)}`
  const kinds = EXPORT_KINDS.filter((k) => r.ok || k !== 'processed')
  return {
    base,
    /* 默认落点：用户根下的 `exports/`（真机上由 `fsx::paths::EXPORTS_DIR` 给） */
    defaultDir: `${USER_ROOT}/exports`,
    files: kinds.map((kind) => ({ kind, name: `${base}${EXPORT_SUFFIX[kind]}` })),
    missing: r.ok ? [] : ['processed'],
  }
}

/** 演示导出：假后端没有文件系统 —— 只把"会落到哪几个路径"算出来（真机上那步是真复制） */
export function reportExportPaths(id: string, dir: string): string[] {
  const plan = reportExportPlan(id)
  return plan.files.map((f) => `${dir}/${plan.base}/${f.name}`)
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

/** 各步的多行事实（与首条同形状；数字随本行变，展开任何一条都不空） */
function stepDetailOf(
  id: string,
  r: FixtureRow,
  lines: number,
  inputMb: number,
): string | undefined {
  const towerLayers = (r.seq % 3) * 20
  switch (id) {
    case 'input':
      return `读取 ${lines} 行\n文件大小: ${inputMb.toFixed(2)} MB\nSHA256: ${fakeSha(r.seq, r.gcode).slice(0, 8)}`
    case 'config':
      return `预设: ${r.preset}`
    case 'machine-detect':
      return `G-code 机型: ${r.machine}\n预设机型: ${r.machine}\n机型维度: 180×180mm\n禁区数量: 0`
    case 'parse':
      return `机型: ${r.machine}\n耗材类型: PETG\n切片软件: BambuStudio`
    case 'support':
      return '支撑类型: 普通支撑\n熨烫模式: 自动\n熨烫覆盖率阈值: 20.0%'
    case 'collision':
      return '无碰撞，擦料塔位置安全\n模型坐标均在打印范围内\n擦料塔类型: 圆盘擦拭\n擦料塔位置: (20.0, 20.0)'
    case 'pass1':
      return `收集 ${30 + ((r.seq * 13) % 600)} 层高索引\n涂胶遍数: 1\n润笔: 否`
    case 'pass2':
      return `擦料塔层数: ${towerLayers}\n擦料塔高度: ${(r.seq % 3).toFixed(2)}mm\n擦料塔类型: 圆盘擦拭\n从擦料塔到模型的抬笔高度: 0.6mm\n斜肋外墙: 否\n圆盘擦拭: 是\n写入完成`
    case 'write':
      return `原地覆盖完成\n输出 ${(inputMb * 1.02).toFixed(2)} MB`
    default:
      return undefined
  }
}

/** 按本行耗时按比例合成管线计时（config / support 真机上就是 0） */
function pipelineFor(r: FixtureRow): ReportPipelineStep[] {
  const inputMb = 0.5 + ((r.seq % 9) * 3.4)
  const lines = 1200 + ((r.seq * 137) % 26000)
  return PIPELINE_META.map((p) => ({
    id: p.id,
    message: p.msg,
    status: 'ok',
    elapsedMs: Math.round(r.elapsedMs * p.frac),
    detail: stepDetailOf(p.id, r, lines, inputMb),
  }))
}

/** 首条记录的详情 —— 逐字段照真机截图（不合成） */
function firstDetail(summary: ReportSummary, r: FixtureRow): ReportDetail {
  const inputPath =
    `${TEMP_ROOT}/Sat_Oct_10/11_05_12#42952#456/Metadata/${r.gcode}`
  const outputPath =
    `${USER_ROOT}/gcode_history/${DAY}/42952.1_${DAY_COMPACT}_111711.gcode`
  const presetPath = `${USER_ROOT}/presets-local/${r.preset}`

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
      towerHeight: 6.46,
      glueLayerCount: 109,
      totalLayerNumber: 633,
      maxZHeight: 67,
      originalPrintTime: '3h 16m 52s; total estimated time: 3h 22m 55s',
    },
    /* 页面显示「3 小时 39 分」= 13140 秒；比切片器预估（3h 22m 55s）多 8% */
    printTime: {
      totalSeconds: 13140,
      segments: 893809,
    },
    printTimeStatus: 'ready',
    estimatedPrintSeconds: 13140,
    pipeline: [
      {
        id: 'input',
        message: '正在读取文件…',
        status: 'ok',
        elapsedMs: 1800,
        detail: '读取 1241690 行\n文件大小: 27.30 MB\nSHA256: 6a2398db',
      },
      {
        id: 'config',
        message: '配置已就绪（IR 由调用方给出）',
        status: 'ok',
        elapsedMs: 0,
        detail: '预设: A1_MINI-fastv3.3.toml',
      },
      {
        id: 'machine-detect',
        message: '识别机型…',
        status: 'ok',
        elapsedMs: 255,
        detail: 'G-code 机型: A1_MINI\n预设机型: A1_MINI\n机型维度: 180×180mm\n禁区数量: 0',
      },
      {
        id: 'parse',
        message: '解析 G-code…',
        status: 'ok',
        elapsedMs: 1300,
        detail: '机型: A1_MINI\n耗材类型: PETG\n切片软件: BambuStudio',
      },
      {
        id: 'support',
        message: '支撑面检测…',
        status: 'ok',
        elapsedMs: 0,
        detail: '支撑类型: 普通支撑\n熨烫模式: 自动\n熨烫覆盖率阈值: 20.0%',
      },
      {
        id: 'collision',
        message: '碰撞检测…',
        status: 'ok',
        elapsedMs: 1300,
        detail: '无碰撞，擦料塔位置安全\n模型坐标均在打印范围内\n擦料塔类型: 圆盘擦拭\n擦料塔位置: (20.0, 20.0)',
      },
      {
        id: 'pass1',
        message: '第一遍扫描: 99%',
        status: 'ok',
        elapsedMs: 5200,
        detail: '收集 633 层高索引\n涂胶遍数: 1\n润笔: 否',
      },
      {
        id: 'pass2',
        message: '第二遍扫描: 99%',
        status: 'ok',
        elapsedMs: 7800,
        detail:
          '擦料塔层数: 38\n擦料塔高度: 6.46mm\n擦料塔类型: 圆盘擦拭\n从擦料塔到模型的抬笔高度: 0.7mm\n斜肋外墙: 否\n圆盘擦拭: 是\n写入完成',
      },
      {
        id: 'write',
        message: '正在写入文件…',
        status: 'ok',
        elapsedMs: 933,
        detail: '原地覆盖完成\n输出 27.90 MB',
      },
    ],
    glue: {
      glueTotalLength: 9350.845588078173,
      gluePenSwitches: 30,
      gluePassCount: 1,
      glueSpeed: 70,
      penOffsetX: -0.9,
      penOffsetY: 23,
      penOffsetZ: 4,
      glueZLiftHeight: 5,
      glueZLiftHeightFirstLayers: 6,
      glueInterModelLiftHeight: 2,
      glueZOffset: -0.05,
      primeTriggerLayers: 30,
      primeTotalLength: 102.4131416821871,
    },
    tower: {
      towerLayers: 38,
      towerHeight: 6.46,
      towerExtrudeRatio: 1.4,
      towerZLift: 0.7,
      towerSafeZOffset: 0.7,
      towerTravelSpeed: 200,
      fastTowerMode: 'fast',
    },
    slicer: {
      slicerVersion: 'BambuStudio 02.08.02.60',
      machinePresetName: 'Bambu Lab A1 mini 0.4 nozzle',
      processPresetName: '0.20mm Standard @BBL A1M',
      filamentType: 'PETG',
      nozzleDiameter: 0.4,
      slicerIroningEnabled: true,
      ironingMode: 'auto',
      ironingThreshold: 7.9,
      treeSupport: false,
      useDiskWipe: false,
    },
    /* 真机夹具里没有碰撞那一趟（那条记录是别的失败）⇒ 恒 null */
    collision: null,
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
  const presetPath = `${USER_ROOT}/presets-local/${r.preset}`

  /* 文件大小/行数随序号变化，纯粹为了让每行不一样 */
  const inputMb = 0.5 + ((r.seq % 9) * 3.4)
  const lines = 1200 + ((r.seq * 137) % 26000)
  /* 估算秒数（与 printTime.totalSeconds 同值 —— 现实里就是同一个数） */
  const estSeconds = 120 + ((r.seq * 211) % 12000)

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
      ? { totalSeconds: estSeconds, segments: 1000 + ((r.seq * 977) % 80000) }
      : null,
    printTimeStatus: r.ok ? 'ready' : null,
    estimatedPrintSeconds: r.ok ? estSeconds : null,
    pipeline: pipelineFor(r),
    glue: r.ok
      ? {
          glueTotalLength: 300 + ((r.seq * 137) % 9000),
          gluePenSwitches: r.seq % 12,
          gluePassCount: 1 + (r.seq % 3),
          glueSpeed: 70 + (r.seq % 30),
          penOffsetX: -0.9,
          penOffsetY: 23,
          penOffsetZ: 4,
          glueZLiftHeight: 5,
          glueZLiftHeightFirstLayers: 6,
          glueInterModelLiftHeight: 2,
          glueZOffset: -0.05,
          primeTriggerLayers: 30,
          primeTotalLength: 40 + ((r.seq * 31) % 300),
        }
      : null,
    tower: r.ok
      ? {
          towerLayers: (r.seq % 3) * 20,
          towerHeight: r.seq % 3,
          towerExtrudeRatio: 1.4,
          towerZLift: 0.6,
          towerSafeZOffset: 0.6,
          towerTravelSpeed: 200,
          fastTowerMode: 'fast',
        }
      : null,
    slicer: r.ok
      ? {
          slicerVersion: 'BambuStudio 02.08.02.60',
          machinePresetName:
            r.machine === 'A1' ? 'Bambu Lab A1 0.4 nozzle' : 'Bambu Lab A1 mini 0.4 nozzle',
          processPresetName: '0.20mm Standard @BBL A1M',
          filamentType: 'PETG',
          nozzleDiameter: 0.4,
          slicerIroningEnabled: false,
          ironingMode: 'auto',
          ironingThreshold: 7.9,
          treeSupport: false,
          useDiskWipe: false,
        }
      : null,
    /* 同上：夹具照真机，碰撞那一格没有就是 null（预览里那两块整组不摆） */
    collision: null,
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
