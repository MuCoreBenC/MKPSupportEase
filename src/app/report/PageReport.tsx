/*
 * 报告页 —— **后处理执行报告与历史**（2026-10-09 接入，占位退场）。
 *
 * # 账在哪
 *
 * 后处理由本程序自己跑（首页「复制后处理脚本」贴进切片器的后处理栏，导出时触发）。
 * 每跑一次落三件套：`<用户根>/gcode_history/<日期>/<名>_{_original.gcode, .gcode, _meta.json}`，
 * 本页读的就是那棵树（Rust 侧 `ipc/report.rs`，只读）。**页面自己不造一个字的数据**。
 *
 * # 2026-10-11 改版：拆成「列表 + 详情」两块（抽卡定稿的那版）
 *
 *   之前    列表行内联展开 —— 内容一多就坏：展开一条 400~600px，把列表顶开、
 *           行位置跳、要来回滚（作者在 HTML 抽卡里判的）。
 *   现在    左窄列表常驻 + 右详情独占（master–detail），点下一条直接换右栏；
 *           详情按**分组**收成页签（默认「全部」：一组一条横线，从上往下铺）。
 *
 * # 2026-10-11 再改版：左栏是一块能筛的悬浮面板（V13 定稿）
 *
 *   左栏从"贴窗口边的灰条"改成**白面板**（细边 + 柔影 + 大圆角），里面加了三件
 *   工具（都是本页自己的 state，不问后端）：
 *
 *     搜索   文件名子串（大小写不敏感）。输入框是**受控的同一颗节点**（React 重渲染
 *            不重挂），所以打字时焦点不丢、光标不跳。
 *     筛选   全部 / 成功 / 失败 三颗胶囊带计数（计数看**全量**，不随搜索变 ——
 *            它回答的是"这批记录里有多少条失败"，不是"我现在看得见几条"）。
 *     折叠   日期组点一下就收起来（不记进任何地方，切页回来就是全展开）。
 *
 *   行是**两层**：第一层文件名 + 成功/失败（红绿由 `data-ok` 给），第二层
 *   时间 · 预设 · 机型 + 耗时；长文件名走「尾段保真」（`.fHead` / `.fTail`），
 *   `.gcode` 与末尾几个字永远看得见。
 *
 *   ★ 三条 V13 口径，与旧版刻意不同：
 *     · **失败原因不进列表**（整句错误原文只在详情「结论 → 原因」那一行）——
 *       列表里一行一句红字会把整栏顶得七零八落，而"有没有失败"这件事由红色
 *       状态字 + 失败计数胶囊说得很清楚。
 *     · **顶部没有大标题**：一行小灰字说明 + 刷新靠右，窄档也不收这行。
 *     · **「导出」在页头**（2026-10-11 接上）：三件套一起导（原始 / 处理后 / 元数据），
 *       落点默认 `<用户根>/exports/<基名>/`，可以改成任何地方（记着上一次）。框住
 *       `ExportModal.tsx` —— **文件名与默认落点都是后端算的**（命名规则只许一处），
 *       界面照单显示；缺哪几份也照实说（见那一份的文件头）。
 *       **导出成了不在这一页顶上报一句**（作者 2026-10-11）：框留着，落在哪 / 打开位置
 *       都在框里说完，这一页再通知一次只是吵。
 *
 * 窗口挤的时候（<880px 布局宽度）列表**滑盖化**：整块常驻、只露出一截（行首那枚
 * 文件图标 + 抓手），点一下整块滑过来盖在详情上，点遮罩或往左拖滑回去 —— 手机
 * 分屏那种手感。CSS 用容器查询（同一套 DOM 两档），JS 只管拖拽与"点了先滑开"。
 *
 * # 三态分得开
 *
 *   还没有记录   空列表 → 「还没有执行记录」+ 怎么才有记录的一句话
 *   读失败       命令抛错 → 整块错误态（带原因），**不装成空列表**
 *   记录本身坏了 照进列表（红标）—— 藏起失败就是替钩子瞒了一次事故；原因在
 *               详情「结论 → 原因」那一行等着（V13：那句话不进列表）
 *
 * # 动作只有三件
 *
 *   刷新   重读一遍磁盘（钩子随时可能又跑了一次）—— **后处理一跑完会自己重读一次**
 *          （订阅 `postprocess-finished`，见下面那条 effect），这颗按钮是手动补一刀
 *   导出   把选中的那一条的三件套复制到用户点名的目录（`ExportModal.tsx`）
 *   选一条 看这一条的详情（结论 / 打印 / 管线 / 文件）—— 左栏点一下就是选中
 *
 * 「清理缓存」那些**没有真实实现的动作不摆**（纪律：没有实现不给入口）。
 *
 * # 字段面（2026-10-11 补齐）
 *
 * 涂胶（总长 / 换笔 / 速度 / 笔偏移）、擦料塔（层数 / 挤出比 / 安全 Z）、切片工艺
 * （切片器版本 / 预设名 / 熨平）读 `detail.glue / tower / slicer` 三个分包，
 * 键名与旧版 `_meta.json` 一致（写侧在 `src-tauri/src/archive.rs`）；
 * 管线每步的多行事实铺在步骤名下面（`pipeline[].detail`）。**没给就不摆**：
 * 旧记录 / 失败记录没有这些键，对应整组消失，不编占位数字。
 *
 * # 视觉对齐（2026-10-11 作者口径）
 *
 *   一列字段、一列参数：`.kv` 是一张**两列表**（行盒 `display: contents` 下场，
 *   见 `PageReport.module.css`），同组里的值从同一条竖线起排。
 *   单位**跟着值走**（`6.46 mm` / `124 次`），不挂在字段名上（早先的「塔高 mm」）。
 *   笔偏移拆成 X / Y / Z 三行 —— 逗号串在一格里，三个数谁是谁看不出来。
 *
 * # 这一屏的字都能拖选（2026-10-11 作者口径）
 *
 *   报告是给人抄的：详情正文（`.main`）整块 `user-select: text` —— 文件名 / 分组摘要 /
 *   每个字段的值 / 管线每一步 / 错误原文都能拖选，且**各格分开**（从哪个值上按下就选哪个）。
 *   全局那层 `user-select: none`（去除"网页感"）在 `PageReport.module.css` 里按块打开：
 *   页签不收（控件），左边的执行记录列表也不收 —— 那是点击 / 滑盖的落点。
 */

import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react'

import { api, errorText, listen } from '../../api'
import type {
  ReportDetail,
  ReportExportPlan,
  ReportGlueFacts,
  ReportSlicerFacts,
  ReportSummary,
  ReportTowerFacts,
} from '../../api'
import { POSTPROCESS_FINISHED } from '../postprocess/events'
import NoteBar from '../shared/NoteBar'
import type { Note } from '../shared/note'
import { Btn } from '../ui/Controls'
import ExportModal from './ExportModal'
import { useDevPreview } from '../dev/useDevPreview'
import s from './PageReport.module.css'

/**
 * 统计里挑给用户看的几格（键与 `_meta.json` 的 `detail.stats` 同名）。
 * 第三格是**值的单位** —— 单位跟着值印（`0 mm`），不挂在字段名上。
 */
const STAT_CELLS: [string, string, string][] = [
  ['towerHeight', '塔高', 'mm'],
  ['glueLayerCount', '涂胶层', ''],
  ['totalLayerNumber', '总层数', ''],
  ['maxZHeight', '最高 Z', 'mm'],
  ['originalPrintTime', '切片器预估', ''],
]

/** 列表滑盖的临界宽度：与 CSS 里 `@container (width < 880px)` 同一个数 */
const NARROW_W = 880
/** 收起时露出的那一截 */
const PEEK = 44
/** 拖动超过这个距离才吸边，否则回弹 */
const SNAP = 40

/** 「正在重读…」那条提示的原文；读完（成或败）按原文收掉，不误伤别的提示 */
const READING_NOTE = '正在重读执行记录…'

const TAB_ALL = 'all'

/** 左栏状态筛选的三档（`all` 是「全部」，与详情页签那个 TAB_ALL 无关） */
type StatusFilter = 'all' | 'ok' | 'bad'

/** 三颗筛选胶囊（顺序就是摆放顺序）；计数按 id 从 `counts` 取 */
const FILTERS: [StatusFilter, string][] = [
  ['all', '全部'],
  ['ok', '成功'],
  ['bad', '失败'],
]

/** 文件名短于这个长度就整条铺开，不拆头尾 */
const FILE_SHORT_MAX = 22
/** 拆的时候尾段留几个字（`.gcode` 与它前面的几个字）—— 名字再长这两样也得看得见 */
const FILE_TAIL_LEN = 12

/**
 * 管线 12 步的中文显示名。
 *
 * `pipeline[].id` 是**机器名**（`machine-detect` 这种），直接摆出来不好认。
 * 名字取自管线自己那 12 条 `timer.begin` 的文案（`crates/postprocess/src/pipeline/mod.rs`），
 * 去掉「正在…」与省略号 —— 内核那边写着「中文显示名仍只许住 UI 一处」，
 * 这张表就是那一处。认不出来的 id 原样摆（新步骤忘了登记也不会显示成空白）。
 */
const STEP_NAMES: Record<string, string> = {
  input: '读取文件',
  config: '加载配置',
  'machine-detect': '机型识别',
  parse: '解析 G-code',
  support: '支撑面检测',
  collision: '碰撞检测',
  'calib-check': '校准模型检测',
  pass1: '第一遍扫描',
  pass2: '第二遍扫描',
  calibration: '校准 G-code 插入',
  write: '写入文件',
  printtime: '打印时间估算',
}

/**
 * 组小图标（跟组名一排，扫一眼找得到组）—— 只留形状，壳由 `GrpIcon` 给。
 * 24×24 网格、`currentColor` 描边，尺寸由调用方定。
 */
const GROUP_SHAPES: Record<string, ReactNode> = {
  result: (
    <>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M8.5 12.5l2.5 2.5 4.5-5" />
    </>
  ),
  print: (
    <>
      <path d="M7 8.5V3.5h10v5" />
      <rect x="3.5" y="8.5" width="17" height="8" rx="2" />
      <path d="M7 13.5h10v7H7z" />
    </>
  ),
  glue: <path d="M12 3.5c3.5 4 6 7.2 6 10a6 6 0 0 1-12 0c0-2.8 2.5-6 6-10z" />,
  tower: (
    <>
      <rect x="6.5" y="4" width="11" height="16" rx="1.5" />
      <path d="M6.5 9.5h11M6.5 14.5h11" />
    </>
  ),
  slicer: (
    <>
      <path d="M4 7.5h4M13 7.5h7M4 16.5h7M16 16.5h4" />
      <circle cx="10.5" cy="7.5" r="2.2" />
      <circle cx="13.5" cy="16.5" r="2.2" />
    </>
  ),
  warnings: (
    <>
      <path d="M12 4l9 15.5H3z" />
      <path d="M12 10v4.5M12 17v.5" />
    </>
  ),
  pipeline: (
    <>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M12 7.5V12l3 2" />
    </>
  ),
  files: (
    <>
      <path d="M7 3h7l4.5 4.5V20a1.5 1.5 0 0 1-1.5 1.5H7A1.5 1.5 0 0 1 5.5 20V4.5A1.5 1.5 0 0 1 7 3z" />
      <path d="M13.5 3v5h5" />
    </>
  ),
}

/**
 * 日期分组的标题词：今天 / 昨天 / N 天前（`day` 就是 `<日期>` 目录名）。
 *
 * 目录名不一定是日期（有人手动往 `gcode_history/` 里丢过一个文件夹就会这样），
 * 那时 `diff` 是 `NaN` / 负数 —— 与其摆一句「NaN 天前」，不如原样把那串名字印出来。
 */
function dayLabel(day: string): string {
  const d = new Date(`${day}T00:00:00`)
  const now = new Date()
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate())
  const diff = Math.round((today.getTime() - d.getTime()) / 86400000)
  if (!Number.isFinite(diff) || diff < 0) return day
  return diff === 0 ? '今天' : diff === 1 ? '昨天' : `${diff} 天前`
}

/**
 * 列表行第二层的灰字：时间 · 预设 · 机型。
 * 拿不到的那几段**不占位**（不摆空分隔符，也不会剩一串 ` · `）。
 */
function rowMeta(row: ReportSummary): string {
  return [
    (row.startedAt ?? '').slice(11, 16),
    (row.presetName ?? '').replace(/\.toml$/i, ''),
    row.machineType ?? '',
  ]
    .filter((x) => x !== '')
    .join(' · ')
}

/** 管线一步的名字：认得出来的换中文，认不出来原样摆（没有 id 就按序号） */
function stepName(id: string | undefined, i: number): string {
  if (id === undefined || id === '') return `第 ${i + 1} 步`
  return STEP_NAMES[id] ?? id
}

/** 详情里的一组（页签的一项） */
interface Group {
  id: string
  title: string
  /** 组名后面那句摘要（横线上跟在标题后） */
  sum: string
  cells: [string, string][]
}

function elapsedText(ms: number | null): string {
  if (ms === null) return '—'
  if (ms < 1000) return `${ms} 毫秒`
  const sec = ms / 1000
  if (sec < 60) return `${sec.toFixed(1)} 秒`
  const m = Math.floor(sec / 60)
  return `${m} 分 ${Math.round(sec - m * 60)} 秒`
}

function printTimeText(sec: number | undefined): string {
  if (sec === undefined || !Number.isFinite(sec)) return '—'
  const h = Math.floor(sec / 3600)
  const m = Math.round((sec % 3600) / 60)
  return h > 0 ? `${h} 小时 ${m} 分` : `${m} 分`
}

/** 打印时间还没补上时的说法 —— 不许拿 0 凑数（延后估算的那几百毫秒里它是 null） */
function printTimeStatusText(status: string | null | undefined): string {
  if (status === 'computing') return '正在估算…'
  if (status === 'failed') return '估算失败'
  return '—'
}

function shaShort(sha: string | undefined): string {
  return sha === undefined ? '—' : sha.slice(0, 12)
}

function mb(bytes: number | undefined): string | null {
  return bytes === undefined ? null : `${(bytes / 1024 / 1024).toFixed(2)} MB`
}

/** 数字尽量不留尾零：整数原样，小数最多两位（`-0.05` / `1.4` / `23`）。 */
function fmtNum(v: number): string {
  return Number.isInteger(v) ? String(v) : String(Number(v.toFixed(2)))
}

/** 毫米：绝对值过 1000 换米（两位小数），例：`9.35 m` / `0.6 mm` / `5 mm`。 */
function mmText(v: number | undefined): string | null {
  if (v === undefined || !Number.isFinite(v)) return null
  return Math.abs(v) >= 1000 ? `${(v / 1000).toFixed(2)} m` : `${fmtNum(v)} mm`
}

/** 格子值：数字按 `fmtNum` 加后缀，缺了给「—」。 */
function numCell(v: number | undefined, suffix = ''): string {
  return v === undefined ? '—' : `${fmtNum(v)}${suffix}`
}

/**
 * 统计格的值：数字**带着单位**印 —— `mm` 那一档交给 `mmText`（越 1000 换 m），
 * 其余单位补一个空格（`124 次`）。`originalPrintTime` 是切片器原话（字符串），原样铺；
 * 键没有 / 不是数字就给「—」（不拿 0 凑数）。
 */
function statCell(v: unknown, unit: string): string {
  if (typeof v === 'number' && Number.isFinite(v)) {
    if (unit === '') return fmtNum(v)
    if (unit === 'mm') return mmText(v) ?? fmtNum(v)
    return `${fmtNum(v)} ${unit}`
  }
  return typeof v === 'string' && v !== '' ? v : '—'
}

/**
 * 从切片器的原话里抠「total estimated time」的秒数
 * （`"3h 16m 52s; total estimated time: 3h 22m 55s"` → 12175）—— 对不上就是 `null`。
 */
function slicerEstimateSeconds(original: string | undefined): number | null {
  if (original === undefined) return null
  const m = /total estimated time:\s*(?:(\d+)\s*h\s*)?(?:(\d+)\s*m\s*)?(?:(\d+)\s*s)?/.exec(
    original,
  )
  if (m === null || (m[1] === undefined && m[2] === undefined && m[3] === undefined)) return null
  return Number(m[1] ?? 0) * 3600 + Number(m[2] ?? 0) * 60 + Number(m[3] ?? 0)
}

/** 涂胶组（键与 `detail.glue` 同名；全格都空就整组不摆） */
function glueGroup(glue: ReportGlueFacts): Group | null {
  const cells: [string, string][] = [
    ['涂胶总长', mmText(glue.glueTotalLength) ?? '—'],
    ['换笔', numCell(glue.gluePenSwitches, ' 次')],
    ['涂胶遍数', numCell(glue.gluePassCount, ' 遍')],
    ['涂胶速度', numCell(glue.glueSpeed, ' mm/s')],
    /* 笔偏移拆 X / Y / Z 三行：逗号串在一格里，三个数谁是谁看不出来 */
    ['笔偏移 X', mmText(glue.penOffsetX) ?? '—'],
    ['笔偏移 Y', mmText(glue.penOffsetY) ?? '—'],
    ['笔偏移 Z', mmText(glue.penOffsetZ) ?? '—'],
    [
      '抬笔高度',
      glue.glueZLiftHeight === undefined
        ? '—'
        : `${fmtNum(glue.glueZLiftHeight)} mm${
            glue.glueZLiftHeightFirstLayers === undefined
              ? ''
              : `（首层 ${fmtNum(glue.glueZLiftHeightFirstLayers)}）`
          }`,
    ],
    ['模型间抬笔', mmText(glue.glueInterModelLiftHeight) ?? '—'],
    ['Z 偏移', glue.glueZOffset === undefined ? '—' : `${fmtNum(glue.glueZOffset)} mm`],
    [
      '润笔',
      glue.primeTriggerLayers === undefined && glue.primeTotalLength === undefined
        ? '—'
        : `${numCell(glue.primeTriggerLayers, ' 层')} · ${
            glue.primeTotalLength === undefined ? '—' : `${glue.primeTotalLength.toFixed(1)} mm`
          }`,
    ],
  ]
  if (!hasAnyValue(cells)) return null
  const sum = [
    glue.primeTriggerLayers === undefined ? null : `${fmtNum(glue.primeTriggerLayers)} 层`,
    mmText(glue.glueTotalLength),
    glue.gluePenSwitches === undefined ? null : `换笔 ${fmtNum(glue.gluePenSwitches)} 次`,
  ]
    .filter((x): x is string => x !== null)
    .join(' · ')
  return { id: 'glue', title: '涂胶', sum, cells }
}

/** 擦料塔组（键与 `detail.tower` 同名） */
function towerGroup(tower: ReportTowerFacts): Group | null {
  const cells: [string, string][] = [
    [
      '层数 / 高度',
      tower.towerLayers === undefined && tower.towerHeight === undefined
        ? '—'
        : `${numCell(tower.towerLayers, ' 层')} · ${
            tower.towerHeight === undefined ? '—' : `${fmtNum(tower.towerHeight)} mm`
          }`,
    ],
    ['挤出比', tower.towerExtrudeRatio === undefined ? '—' : fmtNum(tower.towerExtrudeRatio)],
    [
      '抬笔 / 安全 Z',
      tower.towerZLift === undefined && tower.towerSafeZOffset === undefined
        ? '—'
        : `${tower.towerZLift === undefined ? '—' : fmtNum(tower.towerZLift)} / ${
            tower.towerSafeZOffset === undefined ? '—' : fmtNum(tower.towerSafeZOffset)
          } mm`,
    ],
    [
      '移动速度 / 模式',
      tower.towerTravelSpeed === undefined && tower.fastTowerMode === undefined
        ? '—'
        : `${numCell(tower.towerTravelSpeed, ' mm/s')} · ${tower.fastTowerMode ?? '—'}`,
    ],
  ]
  if (!hasAnyValue(cells)) return null
  const sum = [
    tower.towerLayers === undefined ? null : `${fmtNum(tower.towerLayers)} 层`,
    tower.towerHeight === undefined ? null : `${fmtNum(tower.towerHeight)} mm`,
  ]
    .filter((x): x is string => x !== null)
    .join(' · ')
  return { id: 'tower', title: '擦料塔', sum, cells }
}

/** 切片工艺组（键与 `detail.slicer` 同名） */
function slicerGroup(slicer: ReportSlicerFacts): Group | null {
  const cells: [string, string][] = [
    ['切片软件', slicer.slicerVersion ?? '—'],
    ['机型预设', slicer.machinePresetName ?? '—'],
    ['工艺预设', slicer.processPresetName ?? '—'],
    [
      '耗材 / 喷嘴',
      slicer.filamentType === undefined && slicer.nozzleDiameter === undefined
        ? '—'
        : `${slicer.filamentType ?? '—'} · ${
            slicer.nozzleDiameter === undefined ? '—' : `${fmtNum(slicer.nozzleDiameter)} mm`
          }`,
    ],
    [
      '熨平',
      slicer.slicerIroningEnabled === undefined &&
      slicer.ironingMode === undefined &&
      slicer.ironingThreshold === undefined
        ? '—'
        : `${
            slicer.slicerIroningEnabled === undefined ? '—' : slicer.slicerIroningEnabled ? '开' : '关'
          } · ${slicer.ironingMode ?? '—'} · 阈值 ${
            slicer.ironingThreshold === undefined ? '—' : fmtNum(slicer.ironingThreshold)
          }`,
    ],
    [
      '支撑',
      slicer.treeSupport === undefined && slicer.useDiskWipe === undefined
        ? '—'
        : `树状 ${slicer.treeSupport === undefined ? '—' : slicer.treeSupport ? '开' : '关'} · 圆盘擦拭 ${
            slicer.useDiskWipe === undefined ? '—' : slicer.useDiskWipe ? '开' : '关'
          }`,
    ],
  ]
  if (!hasAnyValue(cells)) return null
  return { id: 'slicer', title: '切片工艺', sum: slicer.slicerVersion ?? '', cells }
}

/** 一组里所有格子的值都是「—」就别摆这一组（成功/失败两种情况字段不一样多） */
function hasAnyValue(cells: [string, string][]): boolean {
  return cells.some(([, v]) => v !== '—')
}

/** 把详情摊成几组（键与 `_meta.json` 同名，值一律按"有没有"说话） */
function buildGroups(detail: ReportDetail): Group[] {
  const st = detail.summary
  const stats = detail.stats ?? {}
  const groups: Group[] = []

  /* 结论：一眼的答案 */
  const resultCells: [string, string][] = [
    ['结果', st.ok ? '成功' : '失败'],
    ['耗时', elapsedText(st.elapsedMs)],
    ['打印时间', printTimeText(detail.printTime?.totalSeconds)],
    [
      '预设 · 机型',
      [st.presetName, st.machineType].filter((x): x is string => x !== null).join(' · ') || '—',
    ],
    /*
     * 文件行：3MF 模型名在前（拿得到的话），G-code 名跟在括号里 ——
     * 模型名是钩子在切片器临时目录里取的，老记录 / 搬走的 G-code 都没有它，
     * 那时退回只显示 G-code 名（`?? null` 是给"后端没这一格"兜底，不编名字）。
     */
    [
      '文件',
      st.modelName != null && st.gcodeName != null
        ? `${st.modelName}（${st.gcodeName}）`
        : (st.gcodeName ?? '—'),
    ],
  ]
  if (st.error !== null) resultCells.push(['原因', st.error])
  groups.push({
    id: 'result',
    title: '结论',
    sum: `${st.ok ? '成功' : '失败'} · ${elapsedText(st.elapsedMs)}`,
    cells: resultCells,
  })

  /* 打印：切片器给的事实 + 本程序算出来的打印时间 */
  const printCells: [string, string][] = [
    [
      '打印时间（估算）',
      detail.printTime?.totalSeconds === undefined
        ? printTimeStatusText(detail.printTimeStatus)
        : printTimeText(detail.printTime.totalSeconds),
    ],
    ...STAT_CELLS.map(([key, label, unit]): [string, string] => [
      label,
      statCell(stats[key], unit),
    ]),
  ]
  if (detail.printTime?.segments !== undefined) {
    printCells.push(['耗时段数', String(detail.printTime.segments)])
  }
  /* 本程序估算 vs 切片器原话：两个数都真有才摆（这一段最值钱的信息） */
  const slicerSec = slicerEstimateSeconds(
    typeof stats.originalPrintTime === 'string' ? stats.originalPrintTime : undefined,
  )
  const ours = detail.estimatedPrintSeconds ?? detail.printTime?.totalSeconds
  if (slicerSec !== null && slicerSec > 0 && ours !== undefined) {
    const pct = Math.round(((ours - slicerSec) / slicerSec) * 100)
    printCells.push(['比切片器预估', `${pct >= 0 ? '+' : ''}${pct}%`])
  }
  if (hasAnyValue(printCells)) {
    groups.push({
      id: 'print',
      title: '打印',
      sum: printTimeText(detail.printTime?.totalSeconds),
      cells: printCells,
    })
  }

  /* 涂胶 / 擦料塔 / 切片工艺：旧记录没有这些分包 ⇒ 整组不摆（不编占位值） */
  if (detail.glue !== undefined && detail.glue !== null) {
    const g = glueGroup(detail.glue)
    if (g !== null) groups.push(g)
  }
  if (detail.tower !== undefined && detail.tower !== null) {
    const g = towerGroup(detail.tower)
    if (g !== null) groups.push(g)
  }
  if (detail.slicer !== undefined && detail.slicer !== null) {
    const g = slicerGroup(detail.slicer)
    if (g !== null) groups.push(g)
  }

  /* 警告：没有就不摆这一组（空组还要占一条横线，等于喊狼来了） */
  if (detail.warnings.length > 0) {
    groups.push({
      id: 'warnings',
      title: '警告',
      sum: `${detail.warnings.length} 条`,
      cells: detail.warnings.map((w, i): [string, string] => [`第 ${i + 1} 条`, w]),
    })
  }

  /* 管线计时：一步一行，步里的多行细节跟着 */
  const pipeMs = detail.pipeline.reduce((a, p) => a + (p.elapsedMs ?? 0), 0)
  groups.push({
    id: 'pipeline',
    title: '管线计时',
    sum: `${detail.pipeline.length} 步 · ${pipeMs.toFixed(0)} 毫秒`,
    cells: [],
  })

  /* 文件与校验：输入/输出两边的事实摆在一起，才看得出"改了多少" */
  const inMb = mb(detail.inputFile?.sizeBytes)
  const outMb = mb(detail.outputFile?.sizeBytes)
  const diff =
    detail.inputFile?.sizeBytes !== undefined && detail.outputFile?.sizeBytes !== undefined
      ? `${((detail.outputFile.sizeBytes - detail.inputFile.sizeBytes) / 1024 / 1024).toFixed(2)} MB`
      : null
  const fileCells: [string, string][] = [
    [
      '输入',
      [
        detail.inputFile?.path ?? detail.gcodePath ?? '—',
        inMb,
        detail.inputFile?.lines === undefined ? null : `${detail.inputFile.lines} 行`,
        `sha ${shaShort(detail.inputFile?.sha256)}`,
      ]
        .filter((x): x is string => x !== null)
        .join(' · '),
    ],
    [
      '输出',
      [detail.outputFile?.path ?? '—', outMb, `sha ${shaShort(detail.outputFile?.sha256)}`]
        .filter((x): x is string => x !== null)
        .join(' · '),
    ],
    ['预设', detail.presetPath ?? st.presetName ?? '—'],
  ]
  if (detail.invocation?.exe !== undefined) {
    fileCells.push([
      '调用',
      `${detail.invocation.exe}${
        detail.invocation.args !== undefined ? ` ${detail.invocation.args.join(' ')}` : ''
      }`,
    ])
  }
  groups.push({
    id: 'files',
    title: '文件与校验',
    sum: diff === null ? '' : `改了 ${diff}`,
    cells: fileCells,
  })

  return groups
}

/**
 * 文件名：长名「尾段保真」—— 超过 22 字就拆两段，尾段（`.gcode` 与它前面那 11 个字）
 * 钉住不缩，前段自己收省略号；短名整条原样。
 *
 * 为什么不直接一句 `text-overflow: ellipsis`：那样截出来的是"前 22 个字 + …"，
 * 而用户认这份文件靠的恰恰是**尾巴**（`.gcode` 前面那截，`帕姆` / `deepfish` 那种），
 * 全被省略号吃掉了。全名挂在调用方的 `title` 上。
 */
function FileName({ name }: { name: string | null }) {
  const n = name ?? '（记录里没写文件名）'
  if (n.length <= FILE_SHORT_MAX) return <>{n}</>
  return (
    <>
      <span className={s.fHead}>{n.slice(0, n.length - FILE_TAIL_LEN)}</span>
      <span className={s.fTail}>{n.slice(-FILE_TAIL_LEN)}</span>
    </>
  )
}

export default function PageReport() {
  /* null = 还在读；读失败是另一格（err），不与空列表混 */
  const [rows, setRows] = useState<ReportSummary[] | null>(null)
  const [err, setErr] = useState<string | null>(null)
  const [note, setNote] = useState<Note | null>(null)
  /* 开发台：点「浮出的条」时摆一条提示条（in-flow、要手动关，不自己消失） */
  const devNote = useDevPreview('float-toast')
  useEffect(() => {
    if (devNote.on) {
      setNote({
        text: '已确认导出：3 份文件写到 /Users/wzy/Library/Application Support/SupportEase/exports/',
        bad: false,
      })
    }
  }, [devNote.on])
  const [selId, setSelId] = useState<string | null>(null)
  const [detail, setDetail] = useState<ReportDetail | null>(null)
  const [detailErr, setDetailErr] = useState<string | null>(null)
  const [tab, setTab] = useState<string>(TAB_ALL)
  const [sideOpen, setSideOpen] = useState(false)
  /*
   * 左栏工具区（V13）：搜索词 / 状态筛选 / 折叠了的日期组。
   * 三样都是**本页自己的**筛选态，不问后端、不落盘 —— 切走再回来就回到全展开。
   */
  const [query, setQuery] = useState('')
  const [filter, setFilter] = useState<StatusFilter>('all')
  const [collapsedDays, setCollapsedDays] = useState<ReadonlySet<string>>(new Set())
  /** 导出框开着没有（它的内务全在 `ExportModal` 里，页面只管这一格开关） */
  const [exportOpen, setExportOpen] = useState(false)
  /* 开发台：（只在开发构建里收得到事件）—— 有选中记录才开那一框（没记录时那框本来是灰的） */
  const devExport = useDevPreview('export')
  useEffect(() => {
    if (devExport.on && selId !== null) setExportOpen(true)
  }, [devExport.on, selId])
  /** 选中那条的导出计划（**预取**：开框就是终态高度，见 `ExportModal` 的 `preload`） */
  const [exportPlan, setExportPlan] = useState<{ id: string; plan: ReportExportPlan } | null>(null)
  /*
   * 只用来告诉 CSS「走浮层档」；交互一律看 `isFloating()` 的实际几何，不认这个布尔值。
   *
   * ★ 活口（2026-10-14 标记）：**这个 state 的值没人读**（解构里就空着一位），
   * 它只借 `setNarrow` 触发一次重渲染 —— 下面那两处 `setNarrow(...)` 是真在用的调用。
   * 别当"死 state"删了：删掉之后窄档切换时页面不重渲，浮层判据会晚一帧。
   */
  const [, setNarrow] = useState(false)

  const pageRef = useRef<HTMLDivElement>(null)
  const sideRef = useRef<HTMLElement>(null)
  /*
   * 浮层判据 = **真的浮着**（`position:absolute`），不是窄档这个布尔值。
   *
   * ★ 二者必须分开判：窄档下 DOM 里是同一份 `.side`，但 CSS 的档位是**容器查询**算的，
   *   与 JS 的窄档判断可能差一帧，也可能因为"页面被挤窄"这类布局反噬而不同步。
   *   若只认 `narrow`，一个布尔值的误判就会把整份列表变成"怎么点都不响应"
   *   （拖拽被当成滑盖、那次 click 又被压掉）。以实际生效的几何为准，才不会整页死。
   */
  const isFloating = useCallback(() => {
    const el = sideRef.current
    return el !== null && getComputedStyle(el).position === 'absolute'
  }, [])

  /** 拖拽中的现场；null = 没在拖 */
  const drag = useRef<{ x: number; y: number; dx: number; open: boolean; moved: boolean } | null>(
    null,
  )
  /**
   * 下一次落到行按钮上的 click 不当"选行"：① 收起态点开抽屉那一下（露出的
   * 44px 下面就是行按钮）；② 真拖拽收尾的那一下。新一次 pointerdown 会清残留，
   * 挂不住（见 `onDown`）。
   */
  const suppressClick = useRef(false)

  /**
   * `pickNewest` = 读完把选中切到最新那条。手动刷新不传（别抢用户正看着的那条）；
   * **后处理跑完那一趟才传**（作者 2026-10-17）—— 新记录就该是主角，右栏跟着换详情。
   */
  const read = useCallback((pickNewest = false) => {
    setErr(null)
    setRows(null)
    api.getReportList().then(
      (list) => {
        setRows(list)
        if (pickNewest) {
          /* 后处理刚跑完：选最新那条（失败记录也选 —— 失败更该立刻看见） */
          setSelId(list[0]?.id ?? null)
        } else {
          /* 默认选中最新那条：进页面就有内容看，不用先点一下 */
          setSelId((cur) => cur ?? list[0]?.id ?? null)
        }
        /*
         * 读到了就收掉那条「正在重读…」——不收回就是挂着的谎：
         * 页面早就不在读了，提示还赖在那儿，用户只会当它坏了（提示条本来
         * 就"手动关、不自动消失"，那**读完了**这件事就必须由代码自己说）。
         */
        setNote((cur) => (cur?.text === READING_NOTE ? null : cur))
      },
      (e: unknown) => {
        setRows([])
        setErr(errorText(e))
        setNote((cur) => (cur?.text === READING_NOTE ? null : cur))
      },
    )
  }, [])

  useEffect(() => {
    read()
  }, [read])

  /*
   * 后处理一跑完就**自己重读一遍**（作者 2026-10-11：「这个报告需要自己更新一下」）。
   *
   * 报告页读的就是那一趟落下的账（`gcode_history/…/_meta.json`），而钩子跑完时用户
   * 多半还站在切片器那边 —— 等他切回来，这一条应当已经在列表里了，不用先按刷新。
   * **成功与失败都重读**：失败也落一条记录，那一条更该立刻看得见。
   *
   * 事件名与那一屏模态框共用一处（`postprocess/events.ts`）—— 字面量只许有那一处。
   */
  useEffect(() => {
    let alive = true
    const offs: (() => void)[] = []
    void listen(POSTPROCESS_FINISHED, () => {
      if (alive) read(true)
    }).then((off) => {
      /* 订阅回执比卸载晚到时就地退订（否则监听器留在后端没人收） */
      if (alive) offs.push(off)
      else off()
    })
    return () => {
      alive = false
      for (const off of offs) off()
    }
  }, [read])

  /*
   * 导出计划**在选中那一条时就读**（不是等按了「导出」才读）。
   *
   * 那一条命令只是"读一份 meta + 看看三个文件在不在"，比列表本身还轻；换来的是点
   * 「导出」时框**打开就是终态高度** —— 现场读的话会先摆一行"正在读…"、再长高跳一下，
   * 看着就像整页闪了一下（作者 2026-10-11 点的）。
   * 预取失败**不在这儿报**：真按了「导出」时框会自己再读一次，并把后端那句话摆出来。
   */
  useEffect(() => {
    if (selId === null) {
      setExportPlan(null)
      return
    }
    let alive = true
    api.getReportExportPlan(selId).then(
      (plan) => {
        if (alive) setExportPlan({ id: selId, plan })
      },
      () => {
        /* 预取失败 = 什么都不留（框那边自己去读、自己报错） */
      },
    )
    return () => {
      alive = false
    }
  }, [selId])

  /* 展开一条：详情按需读（列表只带摘要，选中才读整份 meta） */
  useEffect(() => {
    if (selId === null) {
      setDetail(null)
      setDetailErr(null)
      return
    }
    setDetail(null)
    setDetailErr(null)
    let alive = true
    api.getReportDetail(selId).then(
      (got) => {
        if (alive) setDetail(got)
      },
      (e: unknown) => {
        if (alive) setDetailErr(errorText(e))
      },
    )
    return () => {
      alive = false
    }
  }, [selId])

  /*
   * 窄档判断：CSS 里的容器查询管布局，JS 只要一个布尔值来决定"点了是滑开还是选中"。
   *
   * ★ 量的是**窗口**内宽，不是页面自己的 `clientWidth`：库里这条左栏是 `position:absolute`
   *   浮层（窄档档位），页面一旦被挤到比窗口窄（Tauri 的窗口控制浮层展开时就会挤），
   *   `page.clientWidth` 仍然报"宽"，于是 CSS 已经切成浮层盖住了列表、JS 这边却以为还是
   *   常驻双栏 ⇒ 点列表只当"选行"，永远滑不开 —— 表现就是**列表点不动**。
   *   窗口内宽是这条判据里唯一不会被自己布局反噬的量。
   */
  useEffect(() => {
    const measure = () => {
      const el = pageRef.current
      if (el === null) {
        setNarrow(window.innerWidth < NARROW_W)
        return
      }
      const box = el.getBoundingClientRect()
      /*
       * 贴着窗口右缘（差值 ≤ 2px）时以页面宽为准（对得上 CSS 的容器查询）；
       * 否则说明页面被挤窄了 —— 取两者小的那个，宁可早一点当窄档。
       */
      const flush = Math.abs(box.right - window.innerWidth) <= 2
      setNarrow((flush ? Math.min(box.width, window.innerWidth) : box.width) < NARROW_W)
    }
    measure()
    const ro = new ResizeObserver(measure)
    const el = pageRef.current
    if (el !== null) ro.observe(el)
    window.addEventListener('resize', measure)
    return () => {
      ro.disconnect()
      window.removeEventListener('resize', measure)
    }
  }, [])

  /*
   * 选一条记录。
   *
   * ★ 常驻双栏档下**只有一条路**：选中它 —— 早先这里用 JS 的 `narrow` 当闸，
   *   一旦这个布尔值与 CSS 的档位不同步（页面被挤窄 / 收窄到刚好过界），
   *   点一下会被当成"该滑开"而吞掉 ⇒ **整份列表怎么点都不响应**。
   *   现在收起/展开只认**真的是浮层**时的状态，常驻档永远落到"选中"。
   */
  const pick = (row: ReportSummary) => {
    /*
     * 收起态那一下点的是**把手**那 44px（整块列表在屏幕左缘之外，命中测试是空的），
     * 所以这一下既要滑开、也要**顺手选上** —— 只滑开不选，用户得点两下才看见内容，
     * 手感就是「点了没反应」。展开态才是纯粹的"换一条"，选完收回。
     */
    if (suppressClick.current) {
      suppressClick.current = false
      return
    }
    setSelId(row.id)
    if (isFloating()) setSideOpen(false)
  }

  /* ---------- 滑盖拖拽（只在浮层档生效） ---------- */
  /*
   * ★★ 别在 pointerdown 当场 setPointerCapture。
   *
   * 已在 WebKit（Tauri macOS 的 WKWebView）与 Chromium 双内核实测：祖先（nav）
   * 在按下的那一刻把指针 capture 走，这次按压随后产生的 click 会被重定向到祖先，
   * 列表里每行 <button> 自己的 onClick 压根不触发 —— 表现就是窄窗浮层态下
   * **整块执行记录怎么点都不切换**（宽窗口双栏档只是因为 `isFloating()` 早退、
   * 没 capture 才一直是好的，所以平时试不出来）。
   *
   * 做法：按下只记账；等真的横过 8px 阈值（onMove）确认是拖拽，再 capture。
   * 普通点击不 capture，click 老老实实落在行按钮上。
   */
  const onDown = (e: React.PointerEvent<HTMLElement>) => {
    /*
     * 新一次按下绝不继承上一轮残留的 suppressClick：真拖拽后那次 click 会被
     * capture 导向 nav（到不了行按钮），标记没人消费就挂住了；不在这儿清，
     * 下一次点行会被白白吞掉。拖拽自己的那下 click 由**同一次**按压产生，
     * 前面不会再有 pointerdown，所以这里清不掉"该压的那一下"。
     */
    suppressClick.current = false
    if (!isFloating()) return
    /*
     * 抽屉**开着**的时候，在搜索框里按下不算"抓抽屉"：那里横向拖是**选字**，
     * 抓走指针就变成抽屉跟着手走、刚选中的一段也丢了。
     * 收起态不在此列 —— 那 44px 里也压着搜索框的左缘，按下去该把抽屉拉出来。
     */
    if (sideOpen && e.target instanceof HTMLInputElement) return
    drag.current = { x: e.clientX, y: e.clientY, dx: 0, open: sideOpen, moved: false }
    /*
     * ★ 收尾线挂在 **window** 上，不是只靠 nav 自己那两颗 onPointerUp。
     *
     * 按下之后如果还没到 8px 阈值（也就还没 capture），指针就在 nav 外面松开了 ——
     * nav 收不到那一下 pointerup，`drag.current` 与那一行内联 `transform` **一起留下**：
     * 手感就是"手早就松了、面板还跟着鼠标走"，而且面板会永远卡在拖到一半的位置
     * （作者 2026-10-11 实测：左栏被推得只剩一窄条，怎么点都回不来）。
     * 挂在 window 上就一定收得到；重复触发无害（`onUp` 自己认 `drag.current` 是否已清）。
     */
    window.addEventListener('pointerup', onUp)
    window.addEventListener('pointercancel', onUp)
  }
  const onMove = (e: React.PointerEvent<HTMLElement>) => {
    const d = drag.current
    if (d === null || sideRef.current === null || !isFloating()) return
    const dx = e.clientX - d.x
    const dy = e.clientY - d.y
    if (!d.moved && Math.abs(dx) < 8) return
    /* 纵向意图交给列表自己滚 */
    if (!d.moved && Math.abs(dx) <= Math.abs(dy)) {
      drag.current = null
      return
    }
    d.moved = true
    d.dx = dx
    /* 确认是拖拽的这一刻才抓指针 —— 抓早了会把行按钮的 click 吃掉（见 onDown） */
    e.currentTarget.setPointerCapture(e.pointerId)
    const el = sideRef.current
    el.style.transition = 'none'
    const min = -(el.getBoundingClientRect().width - PEEK)
    const base = d.open ? 0 : min
    el.style.transform = `translateX(${Math.min(0, Math.max(min, base + dx))}px)`
  }
  /**
   * 收尾（拖拽 / 点把手两件事都从这儿出）。
   *
   * ★ 身份必须**稳定**（`useCallback` 空依赖）：`onDown` 把它挂到 `window` 上，
   *   每次渲染换个新函数的话 `removeEventListener` 摘掉的是别人，收尾线会越挂越多。
   */
  const onUp = useCallback(() => {
    window.removeEventListener('pointerup', onUp)
    window.removeEventListener('pointercancel', onUp)
    const d = drag.current
    if (d === null) return
    drag.current = null
    const el = sideRef.current
    if (el !== null) {
      el.style.transition = ''
      el.style.transform = ''
    }
    if (!d.moved) {
      /*
       * 没拖：收起态下点把手就是"滑过来"，随后那次 click 要压掉。
       * ※ 现在 pointerdown 不再 capture（见 onDown），露出那 44px 下面压着的就是
       *   一行行按钮 —— 不压就会变成「刚展开、又被这一行选中并关回去」，看起来
       *   跟没反应一样。展开态不压 —— 那才是真正的"选一条"。
       */
      if (!d.open && isFloating()) {
        setSideOpen(true)
        suppressClick.current = true
      }
      return
    }
    suppressClick.current = true
    if (!d.open && d.dx > SNAP) setSideOpen(true)
    else if (d.open && d.dx < -SNAP) setSideOpen(false)
  }, [isFloating])

  /* ---------- 列表：搜索 + 筛选 + 按日分组 ---------- */

  /**
   * 当前搜索 + 筛选下看得见的记录（顺序照后端给的：最新在前）。
   *
   * 搜索只认**行上看得见的那两个字**（G-code 文件名）—— 拿 3MF 模型名去匹配
   * 就会出现"打「帕姆」跳出一行 `.63288.1.gcode`"这种对不上的事。
   * 大小写不敏感；筛选按成败两档。两样都空就是原样一张表。
   */
  const q = query.trim().toLowerCase()
  const visible = (rows ?? []).filter((r) => {
    if (filter === 'ok' && !r.ok) return false
    if (filter === 'bad' && r.ok) return false
    if (q === '') return true
    return (r.gcodeName ?? '').toLowerCase().includes(q)
  })

  /*
   * 三颗胶囊的计数看**全量**，不随搜索变：它回答的是"这批记录里有多少条失败"，
   * 不是"我现在看得见几条"（后者会让"失败 3"在打字时一路缩，读起来像在丢记录）。
   */
  const counts = {
    all: (rows ?? []).length,
    ok: (rows ?? []).filter((r) => r.ok).length,
    bad: (rows ?? []).filter((r) => !r.ok).length,
  }

  const days: string[] = []
  const byDay = new Map<string, ReportSummary[]>()
  visible.forEach((r) => {
    if (!days.includes(r.day)) days.push(r.day)
    const list = byDay.get(r.day)
    if (list === undefined) byDay.set(r.day, [r])
    else list.push(r)
  })

  /** 日期组：点一下收起来 / 再点展开 */
  const toggleDay = (day: string) => {
    setCollapsedDays((cur) => {
      const next = new Set(cur)
      if (next.has(day)) next.delete(day)
      else next.add(day)
      return next
    })
  }

  const groups = detail === null ? [] : buildGroups(detail)
  const activeTab = groups.some((g) => g.id === tab) ? tab : TAB_ALL
  const sel = (rows ?? []).find((r) => r.id === selId) ?? null

  const listHtml = (
    <nav
      className={s.side}
      ref={sideRef}
      data-open={sideOpen}
      onPointerDown={onDown}
      onPointerMove={onMove}
      onPointerUp={onUp}
      onPointerCancel={onUp}
    >
      <div className={s.sideHead}>
        <span className={s.sideLb}>执行记录</span>
        <span className={s.sideCnt}>{(rows ?? []).length} 条</span>
      </div>

      <div className={s.sideTools}>
        <div className={s.searchBox}>
          <svg
            className={s.searchIcon}
            width={14}
            height={14}
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth={2}
            aria-hidden="true"
          >
            <circle cx="11" cy="11" r="6.5" />
            <path d="M16 16l4.5 4.5" />
          </svg>
          <input
            className={s.searchInput}
            type="text"
            placeholder="搜索文件名…"
            aria-label="搜索文件名"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
        </div>
        <div className={s.filterBar}>
          {FILTERS.map(([id, label]) => (
            <button
              key={id}
              type="button"
              className={s.filterPill}
              data-on={filter === id}
              aria-pressed={filter === id}
              onClick={() => setFilter(id)}
            >
              {label} <b>{counts[id]}</b>
            </button>
          ))}
        </div>
      </div>

      <ul className={s.sideList}>
        {days.length === 0 && (
          <li>
            <p className={s.listEmpty}>没有匹配的记录</p>
          </li>
        )}
        {days.map((day) => {
          const collapsed = collapsedDays.has(day)
          return (
            <li key={day}>
              <button
                type="button"
                className={s.dayRow}
                data-collapsed={collapsed}
                aria-expanded={!collapsed}
                onClick={() => toggleDay(day)}
              >
                <svg
                  className={s.dayIcon}
                  width={13}
                  height={13}
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth={1.8}
                  aria-hidden="true"
                >
                  <rect x="3.5" y="5" width="17" height="15.5" rx="2.5" />
                  <path d="M8 3v4M16 3v4M3.5 10h17" />
                </svg>
                <span className={s.dayName}>
                  {dayLabel(day)} · {day}
                </span>
                <svg
                  className={s.dayChev}
                  width={12}
                  height={12}
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth={2}
                  aria-hidden="true"
                >
                  <path d="M6 15l6-6 6 6" />
                </svg>
              </button>
              {!collapsed && (
                <ul className={s.dayList}>
                  {(byDay.get(day) ?? []).map((row) => (
                    <li key={row.id}>
                      <button
                        type="button"
                        className={s.sideRow}
                        data-ok={row.ok}
                        data-on={row.id === selId}
                        title={`${row.gcodeName ?? '（记录里没写文件名）'} · ${row.startedAt ?? row.day} · ${elapsedText(row.elapsedMs)}`}
                        onClick={() => pick(row)}
                      >
                        <span className={s.rowIcon} aria-hidden="true">
                          <svg
                            width={15}
                            height={15}
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            strokeWidth={1.7}
                          >
                            <path d="M7 3h7l4.5 4.5V20a1.5 1.5 0 0 1-1.5 1.5H7A1.5 1.5 0 0 1 5.5 20V4.5A1.5 1.5 0 0 1 7 3z" />
                            <path d="M13.5 3v5h5" />
                          </svg>
                        </span>
                        <span className={s.rowMain}>
                          <span className={s.rowTop}>
                            <span className={s.sideFile}>
                              <FileName name={row.gcodeName} />
                            </span>
                            <span className={s.rowStatus} data-ok={row.ok}>
                              <span className={s.stDot} aria-hidden="true" />
                              {row.ok ? '成功' : '失败'}
                            </span>
                          </span>
                          <span className={s.rowSub}>
                            <span className={s.rowMeta}>{rowMeta(row)}</span>
                            <span className={s.sideDur}>{elapsedText(row.elapsedMs)}</span>
                          </span>
                        </span>
                      </button>
                    </li>
                  ))}
                </ul>
              )}
            </li>
          )
        })}
      </ul>
      <span className={s.grabber} aria-hidden="true" />
    </nav>
  )

  return (
    <section className={s.page} ref={pageRef} aria-label="执行报告">
      <header className={s.head}>
        <p className={s.sub}>
          后处理每跑一次（切片器导出时按「复制后处理脚本」那条命令跑），这里多一条记录。
        </p>
        <div className={s.headActions}>
          {/*
            两颗都是产品那套 `.btn`（`ui/Controls`）—— 槽上的 `[data-page] .btn`
            已经给了胶囊圆角。**不另开一套按钮皮**：全站按钮只许一套（同"模态框只许
            一套"那条），而且动作按钮一律文字、不带图标（见 `shell/icons.tsx` 那条）。
          */}
          <Btn disabled={selId === null} onClick={() => setExportOpen(true)}>
            导出
          </Btn>
          <Btn
            onClick={() => {
              read()
              setNote({ text: READING_NOTE, bad: false })
            }}
          >
            刷新
          </Btn>
        </div>
      </header>

      <div className={s.noteSlot}>
        <NoteBar note={note} onClose={() => setNote(null)} />
      </div>

      <div className={s.content}>
        {err !== null ? (
          <p className={s.err} role="alert">
            执行记录读不出来：{err}
          </p>
        ) : rows === null ? (
          <p className={s.empty}>正在读执行记录…</p>
        ) : rows.length === 0 ? (
          <p className={s.empty}>
            还没有执行记录 —— 把首页「复制后处理脚本」贴进切片器的后处理脚本栏，
            导出打印文件时会自动生成第一条。
          </p>
        ) : (
          <div className={s.pane}>
            {listHtml}
            <div className={s.scrim} onClick={() => setSideOpen(false)} role="presentation" />

            <section className={s.main}>
              {sel === null ? (
                <p className={s.empty}>左边选一条记录看详情。</p>
              ) : (
                <>
                  <div className={s.mainHead}>
                    <h3
                      className={s.mainFile}
                      title={sel.gcodeName ?? '（记录里没写文件名）'}
                    >
                      <FileName name={sel.gcodeName} />
                    </h3>
                    <span className={sel.ok ? s.chip : `${s.chip} ${s.chipBad}`}>
                      {sel.ok ? '成功' : '失败'}
                    </span>
                    <span className={s.mainWhen}>
                      {sel.startedAt ?? sel.day} · {elapsedText(sel.elapsedMs)}
                    </span>
                  </div>

                  {detail !== null && (
                    <>
                      <div className={s.tabs} role="tablist">
                        <button
                          type="button"
                          role="tab"
                          className={s.tab}
                          data-on={activeTab === TAB_ALL}
                          aria-selected={activeTab === TAB_ALL}
                          onClick={() => setTab(TAB_ALL)}
                        >
                          全部
                        </button>
                        {groups.map((g) => (
                          <button
                            key={g.id}
                            type="button"
                            role="tab"
                            className={s.tab}
                            data-on={activeTab === g.id}
                            aria-selected={activeTab === g.id}
                            onClick={() => setTab(g.id)}
                          >
                            {g.title}
                          </button>
                        ))}
                      </div>

                      <div className={s.panels}>
                        {groups.map((g) => (
                          <div
                            key={g.id}
                            className={s.panel}
                            data-on={activeTab === TAB_ALL || activeTab === g.id}
                          >
                            <p className={s.grpBar}>
                              <span className={s.grpIcon} aria-hidden="true">
                                <svg
                                  width={15}
                                  height={15}
                                  viewBox="0 0 24 24"
                                  fill="none"
                                  stroke="currentColor"
                                  strokeWidth={1.8}
                                  strokeLinecap="round"
                                  strokeLinejoin="round"
                                >
                                  {GROUP_SHAPES[g.id]}
                                </svg>
                              </span>
                              <b>{g.title}</b>
                              {g.sum !== '' && <span className={s.grpSum}>{g.sum}</span>}
                            </p>
                            {g.id === 'pipeline' ? (
                              <ol className={s.steps}>
                                {detail.pipeline.map((step, i) => (
                                  <li key={i} className={s.step}>
                                    <span className={s.stepHead}>
                                      <b>{stepName(step.id, i)}</b>
                                      <span className={s.stepMs}>
                                        {elapsedText(step.elapsedMs ?? null)}
                                      </span>
                                      {/* 失败 / 取消的记录：最后停在的那一步给个记号
                                          （status 由钩子写侧给；旧记录没有就不摆） */}
                                      {step.status === 'error' && (
                                        <span className={s.stepBad}>停在这一步</span>
                                      )}
                                      {step.status === 'cancelled' && (
                                        <span className={s.stepStopped}>在这一步取消</span>
                                      )}
                                    </span>
                                    {step.message !== undefined && step.message !== '' && (
                                      <span className={s.stepMsg}>{step.message}</span>
                                    )}
                                    {step.detail !== undefined && step.detail !== '' && (
                                      <span className={s.stepDetail}>{step.detail}</span>
                                    )}
                                  </li>
                                ))}
                              </ol>
                            ) : (
                              <dl className={s.kv}>
                                {g.cells.map(([k, v], i) => (
                                  <div className={s.kvRow} key={`${k}-${i}`}>
                                    <dt>{k}</dt>
                                    <dd className={k === '调用' ? s.mono : undefined}>{v}</dd>
                                  </div>
                                ))}
                              </dl>
                            )}
                          </div>
                        ))}
                      </div>
                    </>
                  )}

                  {detailErr !== null && (
                    <p className={s.err} role="alert">
                      详情读不出来：{detailErr}
                    </p>
                  )}
                  {detail === null && detailErr === null && (
                    <p className={s.empty}>正在读这一条的详情…</p>
                  )}
                </>
              )}
            </section>
          </div>
        )}
      </div>

      {/*
        导出框挂在 `.page` 上（不是整页）：遮罩盖住这一页的内容区，
        窗口标题栏那几颗窗口键照样点得着 —— 全站模态框的约定（见 `components/modal`）。
      */}
      {/*
        ★ 导出成功**不在这一页顶上报一句**了（作者 2026-10-11）：那一句与框里说的是同一件事，
        两条并排只是吵；框现在留着把"落在哪 + 打开位置"一起给完，这一页不必再通知一次。
      */}
      <ExportModal
        open={exportOpen}
        id={selId}
        preload={exportPlan}
        onClose={() => {
          if (devExport.on) devExport.close()
          setExportOpen(false)
        }}
      />
    </section>
  )
}
