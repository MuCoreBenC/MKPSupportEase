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
 *          （订阅 `postprocess-finished`，见下面那条 effect），这颗按钮是手动补一刀；
 *          **右栏详情一起重读**（列表重读了、详情还停在旧值，是最容易看出来的不一致）
 *   导出   把选中的那一条的三件套复制到用户点名的目录（`ExportModal.tsx`）
 *   选一条 看这一条的详情（结论 / 打印 / 管线 / 文件）—— 左栏点一下就是选中
 *
 * # 打印时间是**延后**算的（2026-10-20）
 *
 * 钩子不等估算：先落 `printTimeStatus = computing` 就退（切片器在等退出码），常驻
 * 进程算完再回填 `detail.printTime` 并补成 `ready`。于是「选中不变」的时候右栏也
 * 必须能再读一遍（`detailNonce`）：补算完成的事件一到就拨、按「刷新」也拨，
 * `computing` 期间再兜底轮询几秒。**只按 `selId` 读一次的老写法**会让那一格
 * 永远停在「—」，非得换一条记录再切回来才亮（作者点的那一版）。
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
import { useLocale, useT, type StringKey, type Tr } from '../../i18n'
import type {
  CatalogParamDef,
  PostProcessPrintTime,
  ReportBox4,
  ReportCollision,
  ReportDetail,
  PostProcessRun,
  ReportPipelineStep,
  ReportExportPlan,
  ReportGlueFacts,
  ReportCalibrationFacts,
  ReportSlicerFacts,
  ReportSnapshot,
  ReportSummary,
  ReportTowerFacts,
} from '../../api'
import { POSTPROCESS_FINISHED, POSTPROCESS_PRINT_TIME } from '../postprocess/events'
import NoteBar from '../shared/NoteBar'
import type { Note } from '../shared/note'
import { Btn } from '../ui/Controls'
import Icon from '../shell/icons'
import ui from '../ui/ui.module.css'
import ExportModal from './ExportModal'
import PostProcessModal from '../postprocess/PostProcessModal'
import CollisionMap from './CollisionMap'
import RelocationBoards from './RelocationBoards'
import { useDevPreview } from '../dev/useDevPreview'
import s from './PageReport.module.css'

/**
 * 统计里挑给用户看的几格（键与 `_meta.json` 的 `detail.stats` 同名）。
 * 第三格是**值的单位** —— 单位跟着值印（`0 mm`），不挂在字段名上。
 */
const STAT_CELLS: [string, StringKey, string][] = [
  ['towerHeight', 'report.stat.towerHeight', 'mm'],
  ['glueLayerCount', 'report.stat.glueLayers', ''],
  ['totalLayerNumber', 'report.stat.totalLayers', ''],
  ['maxZHeight', 'report.stat.maxZ', 'mm'],
  ['originalPrintTime', 'report.stat.slicerEstimate', ''],
]

/** 列表滑盖的临界宽度：与 CSS 里 `@container (width < 880px)` 同一个数 */
const NARROW_W = 880
/** 收起时露出的那一截 */
const PEEK = 44
/** 拖动超过这个距离才吸边，否则回弹 */
const SNAP = 40
/**
 * 打印时间还在补算（`printTimeStatus === 'computing'`）时再看一眼的间隔 / 上限。
 *
 * 补算完成的那个事件（`postprocess-print-time`）是主路；轮询只是兜底 ——
 * 事件正好落在"页面还没订阅上"或"这一屏刚被打开"的那一瞬，就不该永远横着。
 * 上限之后就停手：真算不出来由后端标 `failed`，页面不替它转圈。
 */
const PRINT_TIME_POLL_MS = 800
const PRINT_TIME_POLL_MAX_MS = 30_000

/** 「正在重读…」那条提示的键；读完（成或败）按原文收掉，不误伤别的提示 */
const READING_NOTE: StringKey = 'report.note.reading'

const TAB_ALL = 'all'

/** 左栏状态筛选的三档（`all` 是「全部」，与详情页签那个 TAB_ALL 无关） */
type StatusFilter = 'all' | 'ok' | 'bad'

/** 三颗筛选胶囊（顺序就是摆放顺序）；计数按 id 从 `counts` 取 */
const FILTERS: [StatusFilter, StringKey][] = [
  ['all', 'report.filter.all'],
  ['ok', 'report.filter.ok'],
  ['bad', 'report.filter.bad'],
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
const STEP_NAMES: Record<string, StringKey> = {
  input: 'postproc.steps.input',
  config: 'postproc.steps.config',
  'machine-detect': 'postproc.steps.machineDetect',
  parse: 'postproc.steps.parse',
  support: 'postproc.steps.support',
  collision: 'postproc.steps.collision',
  'calib-check': 'postproc.steps.calibCheck',
  pass1: 'postproc.steps.pass1',
  pass2: 'postproc.steps.pass2',
  calibration: 'postproc.steps.calibration',
  write: 'postproc.steps.write',
  printtime: 'postproc.steps.printtime',
}


/**
 * 把一份归档记录摊成「当时那一趟」（2026-10-27 作者：报告页要一颗回放按钮）。
 *
 * **不真跑**：作者裁定只回放当时那份 `_meta.json` 里的进度 / 耗时 / 明细 / 收尾，
 * 一个文件都不碰、也不产生新记录。所以这里造出来的是 `replay: true` 的一份
 * `PostProcessRun` —— 模态框见到它就把真业务按钮全禁掉，文案也说「回放」。
 *
 * 只摊记录里**真有**的那几格：没有的给 `null`，模态框本来就有「没给就不摆」的口径。
 */
function replayRunOf(d: ReportDetail): PostProcessRun {
  const g = d.glue
  return {
    presetName: d.summary.presetName ?? '',
    gcodeName: d.summary.gcodeName ?? d.summary.modelName ?? '',
    /* 开场事实那一栏是钩子当场报的，归档里没有同形状的一份 —— 不给，卡片自己会省掉 */
    facts: null,
    question: null,
    progress: null,
    finished: {
      ok: d.summary.ok,
      cancelled: false,
      message: d.summary.error ?? '',
      code: null,
      stage: null,
      exitCode: d.summary.ok ? 0 : 1,
      elapsedMs: d.summary.elapsedMs ?? 0,
      output: d.outputFile?.path ?? null,
      warnings: d.warnings ?? [],
      result:
        g == null
          ? undefined
          : {
            penX: g.penOffsetX,
            penY: g.penOffsetY,
            penZ: g.penOffsetZ,
            glueSpeed: g.glueSpeed,
            glueLengthMm: g.glueTotalLength,
            printTimeSeconds: d.printTime?.totalSeconds,
            slicer: d.slicer?.slicerVersion,
          },
    },
    preview: false,
    replay: true,
    questionEmitted: true,
  }
}

/**
 * 明细里那些**原始事实**（`disk` / `true` / `inPlace`）翻成人话。
 *
 * 记录里刻意不存"是/否"这种词（那是某一刻的语言），所以这一步在界面上做。
 * 认不出的值原样返回 —— 不猜、不吞。
 */
function argText(name: string, value: string, tr: Tr): string {
  if (value === 'true') return tr('report.value.yes')
  if (value === 'false') return tr('report.value.no')
  if (name === 'type' && (value === 'disk' || value === 'tower')) {
    return tr(value === 'disk' ? 'report.value.disk' : 'report.value.tower')
  }
  if (name === 'how' && (value === 'inPlace' || value === 'newFile')) {
    return tr(value === 'inPlace' ? 'report.value.inPlace' : 'report.value.newFile')
  }
  return value
}

/** 一步的小字：新记录按字典渲染，老记录原样摆当年那句话 */
function stepDetailText(step: ReportPipelineStep, tr: Tr): string | null {
  if (step.detailLines !== undefined && step.detailLines.length > 0) {
    return step.detailLines
      .map((l) => {
        const args: Record<string, string> = {}
        for (const [k, v] of Object.entries(l.args ?? {})) args[k] = argText(k, v, tr)
        return tr(l.key as StringKey, args)
      })
      .join('\n')
  }
  return step.detail === undefined || step.detail === '' ? null : step.detail
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
  /*
   * 「挪塔」（2026-10-18 作者：「可以加上图标」）：一把擦料塔 + 一支指向右边的箭头
   * ——"这座塔被挪到别处去了"，与下面「塔位变化」那两幅板子同一个语汇。
   */
  relocation: (
    <>
      <rect x="3.5" y="6.5" width="6.5" height="11" rx="1.2" />
      <path d="M13.5 12h5.8M17 9.7l2.4 2.3-2.4 2.3" />
    </>
  ),
  /* 「塔位变化」：两幅板子 + 中间那一小段箭头（就是这一组自己画的东西） */
  'relocation-map': (
    <>
      <rect x="2.5" y="7" width="7" height="10" rx="1.4" />
      <rect x="14.5" y="7" width="7" height="10" rx="1.4" />
      <path d="M10.3 12h3.2M11.9 10.6l1.5 1.4-1.5 1.4" />
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
  /* 建议（2026-10-18）：一个箭头 —— "往这边挪" */
  advice: (
    <>
      <path d="M4 12h11" />
      <path d="M11.5 8.5l4 3.5-4 3.5" />
    </>
  ),
  /* 参数快照（2026-10-21）：一摞横线（一份清单）—— "当时那一份的全部参数" */
  snapshot: (
    <>
      <path d="M5 5h14M5 10h14M5 15h10" />
      <circle cx="18" cy="18" r="3" />
    </>
  ),
  /* 碰撞位置：热床俯视 —— 外框 + 里面两块相撞 */
  collision: (
    <>
      <rect x="3.5" y="4.5" width="17" height="15" rx="1.5" />
      <rect x="6.5" y="7.5" width="7" height="6" rx="1" />
      <rect x="11.5" y="11.5" width="6" height="5" rx="1" />
    </>
  ),
}

/**
 * 日期分组的标题词：今天 / 昨天 / N 天前（`day` 就是 `<日期>` 目录名）。
 *
 * 目录名不一定是日期（有人手动往 `gcode_history/` 里丢过一个文件夹就会这样），
 * 那时 `diff` 是 `NaN` / 负数 —— 与其摆一句「NaN 天前」，不如原样把那串名字印出来。
 */
function dayLabel(tr: Tr, day: string): string {
  const d = new Date(`${day}T00:00:00`)
  const now = new Date()
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate())
  const diff = Math.round((today.getTime() - d.getTime()) / 86400000)
  if (!Number.isFinite(diff) || diff < 0) return day
  return diff === 0
    ? tr('report.day.today')
    : diff === 1
      ? tr('report.day.yesterday')
      : tr('report.day.daysAgo', { n: diff })
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
function stepName(tr: Tr, id: string | undefined, i: number): string {
  if (id === undefined || id === '') return tr('report.step.nth', { n: i + 1 })
  const key = STEP_NAMES[id]
  return key === undefined ? id : tr(key)
}

/** 详情里的一组（页签的一项） */
interface Group {
  id: string
  title: string
  /** 组名后面那句摘要（横线上跟在标题后） */
  sum: string
  cells: [string, string][]
}

function elapsedText(tr: Tr, ms: number | null): string {
  if (ms === null) return '—'
  if (ms < 1000) return tr('report.dur.ms', { n: ms })
  const sec = ms / 1000
  if (sec < 60) return tr('report.dur.s', { n: sec.toFixed(1) })
  const m = Math.floor(sec / 60)
  return tr('report.dur.mSec', { m, s: Math.round(sec - m * 60) })
}

function printTimeText(tr: Tr, sec: number | undefined): string {
  if (sec === undefined || !Number.isFinite(sec)) return '—'
  const h = Math.floor(sec / 3600)
  const m = Math.round((sec % 3600) / 60)
  return h > 0 ? tr('report.dur.hMin', { h, m }) : tr('report.dur.m', { n: m })
}

/** 打印时间还没补上时的说法 —— 不许拿 0 凑数（延后估算的那几百毫秒里它是 null） */
function printTimeStatusText(tr: Tr, status: string | null | undefined): string {
  if (status === 'computing') return tr('report.pt.computing')
  if (status === 'failed') return tr('report.pt.failed')
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
function glueGroup(tr: Tr, glue: ReportGlueFacts): Group | null {
  const cells: [string, string][] = [
    [tr('report.glue.total'), mmText(glue.glueTotalLength) ?? '—'],
    [
      tr('report.glue.switches'),
      glue.gluePenSwitches === undefined
        ? '—'
        : tr('report.glue.unit.times', { n: fmtNum(glue.gluePenSwitches) }),
    ],
    [
      tr('report.glue.passes'),
      glue.gluePassCount === undefined
        ? '—'
        : tr('report.glue.unit.passes', { n: fmtNum(glue.gluePassCount) }),
    ],
    [tr('report.glue.speed'), numCell(glue.glueSpeed, ' mm/s')],
    /* 笔偏移拆 X / Y / Z 三行：逗号串在一格里，三个数谁是谁看不出来 */
    [tr('report.glue.offsetX'), mmText(glue.penOffsetX) ?? '—'],
    [tr('report.glue.offsetY'), mmText(glue.penOffsetY) ?? '—'],
    [tr('report.glue.offsetZ'), mmText(glue.penOffsetZ) ?? '—'],
    [
      tr('report.glue.liftHeight'),
      glue.glueZLiftHeight === undefined
        ? '—'
        : `${fmtNum(glue.glueZLiftHeight)} mm${
            glue.glueZLiftHeightFirstLayers === undefined
              ? ''
              : tr('report.glue.firstLayer', { n: fmtNum(glue.glueZLiftHeightFirstLayers) })
          }`,
    ],
    [tr('report.glue.interLift'), mmText(glue.glueInterModelLiftHeight) ?? '—'],
    [tr('report.glue.zOffset'), glue.glueZOffset === undefined ? '—' : `${fmtNum(glue.glueZOffset)} mm`],
    [
      tr('report.glue.prime'),
      glue.primeTriggerLayers === undefined && glue.primeTotalLength === undefined
        ? '—'
        : `${
            glue.primeTriggerLayers === undefined
              ? '—'
              : tr('report.glue.unit.layers', { n: fmtNum(glue.primeTriggerLayers) })
          } · ${
            glue.primeTotalLength === undefined ? '—' : `${glue.primeTotalLength.toFixed(1)} mm`
          }`,
    ],
  ]
  if (!hasAnyValue(cells)) return null
  const sum = [
    glue.primeTriggerLayers === undefined
      ? null
      : tr('report.glue.primeSum', { n: fmtNum(glue.primeTriggerLayers) }),
    mmText(glue.glueTotalLength),
    glue.gluePenSwitches === undefined
      ? null
      : tr('report.glue.swSum', { n: fmtNum(glue.gluePenSwitches) }),
  ]
    .filter((x): x is string => x !== null)
    .join(' · ')
  return { id: 'glue', title: tr('report.glue.title'), sum, cells }
}

/** 擦料塔组（键与 `detail.tower` 同名） */
function towerGroup(tr: Tr, tower: ReportTowerFacts): Group | null {
  const cells: [string, string][] = [
    [
      tr('report.tower.layersHeight'),
      tower.towerLayers === undefined && tower.towerHeight === undefined
        ? '—'
        : `${
            tower.towerLayers === undefined
              ? '—'
              : tr('report.glue.unit.layers', { n: fmtNum(tower.towerLayers) })
          } · ${tower.towerHeight === undefined ? '—' : `${fmtNum(tower.towerHeight)} mm`}`,
    ],
    [tr('report.tower.ratio'), tower.towerExtrudeRatio === undefined ? '—' : fmtNum(tower.towerExtrudeRatio)],
    [
      tr('report.tower.liftSafe'),
      tower.towerZLift === undefined && tower.towerSafeZOffset === undefined
        ? '—'
        : `${tower.towerZLift === undefined ? '—' : fmtNum(tower.towerZLift)} / ${
            tower.towerSafeZOffset === undefined ? '—' : fmtNum(tower.towerSafeZOffset)
          } mm`,
    ],
    [
      tr('report.tower.travel'),
      tower.towerTravelSpeed === undefined && tower.fastTowerMode === undefined
        ? '—'
        : `${numCell(tower.towerTravelSpeed, ' mm/s')} · ${tower.fastTowerMode ?? '—'}`,
    ],
  ]
  if (!hasAnyValue(cells)) return null
  const sum = [
    tower.towerLayers === undefined
      ? null
      : tr('report.glue.unit.layers', { n: fmtNum(tower.towerLayers) }),
    tower.towerHeight === undefined ? null : `${fmtNum(tower.towerHeight)} mm`,
  ]
    .filter((x): x is string => x !== null)
    .join(' · ')
  return { id: 'tower', title: tr('report.tower.title'), sum, cells }
}

/** 切片工艺组（键与 `detail.slicer` 同名） */
function slicerGroup(tr: Tr, slicer: ReportSlicerFacts): Group | null {
  const onOff = (v: boolean | undefined): string =>
    v === undefined ? '—' : tr(v ? 'report.slicer.on' : 'report.slicer.off')
  const cells: [string, string][] = [
    [tr('report.slicer.software'), slicer.slicerVersion ?? '—'],
    [tr('report.slicer.machinePreset'), slicer.machinePresetName ?? '—'],
    [tr('report.slicer.processPreset'), slicer.processPresetName ?? '—'],
    [
      tr('report.slicer.filament'),
      slicer.filamentType === undefined && slicer.nozzleDiameter === undefined
        ? '—'
        : `${slicer.filamentType ?? '—'} · ${
            slicer.nozzleDiameter === undefined ? '—' : `${fmtNum(slicer.nozzleDiameter)} mm`
          }`,
    ],
    [
      tr('report.slicer.ironing'),
      slicer.slicerIroningEnabled === undefined &&
      slicer.ironingMode === undefined &&
      slicer.ironingThreshold === undefined
        ? '—'
        : `${onOff(slicer.slicerIroningEnabled)} · ${slicer.ironingMode ?? '—'} · ${tr(
            'report.slicer.threshold',
            {
              v: slicer.ironingThreshold === undefined ? '—' : fmtNum(slicer.ironingThreshold),
            },
          )}`,
    ],
    [
      tr('report.slicer.support'),
      slicer.treeSupport === undefined && slicer.useDiskWipe === undefined
        ? '—'
        : `${tr('report.slicer.tree', { v: onOff(slicer.treeSupport) })} · ${tr(
            'report.slicer.diskWipe',
            { v: onOff(slicer.useDiskWipe) },
          )}`,
    ],
  ]
  if (!hasAnyValue(cells)) return null
  return { id: 'slicer', title: tr('report.slicer.title'), sum: slicer.slicerVersion ?? '', cells }
}

/**
 * 校准件身份证据链（键与 `detail.calibration` 同名）。
 *
 * 这一组回答的是**当时怎么判的**，不是结论好不好：
 * 原始模型是哪个、两边各写了什么、实测多大、最终用了哪一档。排查"识别错了 /
 * 档位选错了"全看这几行。
 *
 * 值一律**照实说话**：读不到就摆 `—`，判定结果翻成人话，不把 `only_3mf` 这种机器串
 * 直接甩给用户。
 */
function calibrationGroup(tr: Tr, c: ReportCalibrationFacts): Group | null {
  const verdictText: Record<string, StringKey> = {
    from_3mf: 'report.calib.verdict3mf',
    absent: 'report.calib.verdictAbsent',
  }
  const verdict = verdictText[c.verdict] !== undefined ? tr(verdictText[c.verdict]) : c.verdict
  const modelFile =
    c.model3mf != null
      ? `${c.model3mf}${c.exists ? '' : tr('report.calib.gone')}`
      : c.originPath != null
        ? `${c.originPath}${tr('report.calib.not3mf')}`
        : '—'

  const cells: [string, string][] = [
    [tr('report.calib.model'), modelFile],
    [tr('report.calib.mark'), c.identity3mf ?? '—'],
    [tr('report.calib.verdict'), verdict],
  ]
  if (c.note != null && c.note !== '') cells.push([tr('report.calib.note'), c.note])

  if (c.measured != null) {
    const [x0, x1, y0, y1] = c.measured
    cells.push([tr('report.calib.measured'), `${fmtNum(x1 - x0)} × ${fmtNum(y1 - y0)} mm`])
  }
  if (c.profileId != null) {
    const nominal =
      c.footprint != null
        ? tr('report.calib.nominal', { w: fmtNum(c.footprint[0]), h: fmtNum(c.footprint[1]) })
        : null
    const tol = c.tolerance != null ? tr('report.calib.tolerance', { n: fmtNum(c.tolerance) }) : null
    const extra = [nominal, tol].filter((x): x is string => x !== null).join('，')
    cells.push([tr('report.calib.profile'), extra === '' ? c.profileId : `${c.profileId}（${extra}）`])
  }
  if (c.mode != null && c.mode !== '') cells.push([tr('report.calib.modeLabel'), c.mode])

  /* 只有"没有标记 + 也没认到档"才是真的一无所有（普通模型本不该进这里） */
  if (!hasAnyValue(cells)) return null
  return {
    id: 'calibration',
    title: tr('report.calib.title'),
    sum: c.mode ?? verdict,
    cells,
  }
}

/** 一组里所有格子的值都是「—」就别摆这一组（成功/失败两种情况字段不一样多） */
function hasAnyValue(cells: [string, string][]): boolean {
  return cells.some(([, v]) => v !== '—')
}

/** 把详情摊成几组（键与 `_meta.json` 同名，值一律按"有没有"说话） */
function buildGroups(tr: Tr, detail: ReportDetail): Group[] {
  const st = detail.summary
  const stats = detail.stats ?? {}
  const groups: Group[] = []
  const okText = () => tr(st.ok ? 'report.row.ok' : 'report.row.bad')

  /* 结论：一眼的答案 */
  const resultCells: [string, string][] = [
    [tr('report.cell.result'), okText()],
    [tr('report.cell.elapsed'), elapsedText(tr, st.elapsedMs)],
    [tr('report.cell.printTime'), printTimeText(tr, detail.printTime?.totalSeconds)],
    [
      tr('report.cell.presetMachine'),
      [st.presetName, st.machineType].filter((x): x is string => x !== null).join(' · ') || '—',
    ],
    /*
     * 文件行：3MF 模型名在前（拿得到的话），G-code 名跟在括号里 ——
     * 模型名是钩子在切片器临时目录里取的，老记录 / 搬走的 G-code 都没有它，
     * 那时退回只显示 G-code 名（`?? null` 是给"后端没这一格"兜底，不编名字）。
     */
    [
      tr('report.cell.file'),
      st.modelName != null && st.gcodeName != null
        ? tr('report.cell.filePair', { model: st.modelName, gcode: st.gcodeName })
        : (st.gcodeName ?? '—'),
    ],
  ]
  if (st.error !== null) resultCells.push([tr('report.cell.reason'), st.error])
  groups.push({
    id: 'result',
    title: tr('report.g.result'),
    sum: `${okText()} · ${elapsedText(tr, st.elapsedMs)}`,
    cells: resultCells,
  })

  /* 打印：切片器给的事实 + 本程序算出来的打印时间 */
  const printCells: [string, string][] = [
    [
      tr('report.cell.printTimeEst'),
      detail.printTime?.totalSeconds === undefined
        ? printTimeStatusText(tr, detail.printTimeStatus)
        : printTimeText(tr, detail.printTime.totalSeconds),
    ],
    ...STAT_CELLS.map(([key, label, unit]): [string, string] => [
      tr(label),
      statCell(stats[key], unit),
    ]),
  ]
  if (detail.printTime?.segments !== undefined) {
    printCells.push([tr('report.cell.segments'), String(detail.printTime.segments)])
  }
  /* 本程序估算 vs 切片器原话：两个数都真有才摆（这一段最值钱的信息） */
  const slicerSec = slicerEstimateSeconds(
    typeof stats.originalPrintTime === 'string' ? stats.originalPrintTime : undefined,
  )
  const ours = detail.estimatedPrintSeconds ?? detail.printTime?.totalSeconds
  if (slicerSec !== null && slicerSec > 0 && ours !== undefined) {
    const pct = Math.round(((ours - slicerSec) / slicerSec) * 100)
    printCells.push([tr('report.cell.vsSlicer'), `${pct >= 0 ? '+' : ''}${pct}%`])
  }
  if (hasAnyValue(printCells)) {
    groups.push({
      id: 'print',
      title: tr('report.g.print'),
      sum: printTimeText(tr, detail.printTime?.totalSeconds),
      cells: printCells,
    })
  }

  /* 涂胶 / 擦料塔 / 切片工艺：旧记录没有这些分包 ⇒ 整组不摆（不编占位值） */
  if (detail.glue !== undefined && detail.glue !== null) {
    const g = glueGroup(tr, detail.glue)
    if (g !== null) groups.push(g)
  }
  if (detail.tower !== undefined && detail.tower !== null) {
    const g = towerGroup(tr, detail.tower)
    if (g !== null) groups.push(g)
  }
  if (detail.slicer !== undefined && detail.slicer !== null) {
    const g = slicerGroup(tr, detail.slicer)
    if (g !== null) groups.push(g)
  }

  /* 校准件身份证据链：普通模型那一趟后端不写这一格 ⇒ 整组不摆（旧记录同理） */
  if (detail.calibration != null) {
    const g = calibrationGroup(tr, detail.calibration)
    if (g !== null) groups.push(g)
  }

  /*
   * **碰撞那两块**（2026-10-18）：只有碰撞失败的那一趟有 `collision` ⇒ 别的记录
   * 一组都不多（旧记录读到 `null`，整组不摆）。
   *
   * ① 建议：内核已经把方向 / 至少挪多少算好了，连那句话（含码）都渲染好了 ——
   *   这里只管摆，不自己拼（拼了就等于句子有两处真值）。
   * ② 碰撞位置：一张俯视图，撞在哪一眼看见（组件自己画）。
   */
  if (detail.collision != null) {
    const c = detail.collision
    const adviceCells: [string, string][] = []
    if (c.adviceText !== undefined && c.adviceText !== '') {
      adviceCells.push([tr('report.cell.advice'), c.adviceText])
    }
    /* 方向 / 至少挪多少：**只有"挪模型"那档才有数**（挪塔那档是 0，摆出来是噪音） */
    if (c.target === 'model') {
      if (c.direction !== undefined && c.direction !== '') {
        adviceCells.push([tr('report.cell.direction'), tr('report.cell.moveDir', { dir: c.direction })])
      }
      const mm = (v: number | undefined): string | null =>
        v === undefined || v <= 0 ? null : `${v.toFixed(1)} mm`
      const noMove = tr('report.cell.noMove')
      const xs = [mm(c.requiredMoveX), mm(c.requiredMoveY)]
      if (xs[0] !== null || xs[1] !== null) {
        adviceCells.push([
          tr('report.cell.minMove'),
          [
            tr('report.cell.axisX', { v: xs[0] ?? noMove }),
            tr('report.cell.axisY', { v: xs[1] ?? noMove }),
          ].join(' · '),
        ])
      }
    }
    if (c.conflictCount !== undefined && c.conflictCount > 0) {
      adviceCells.push([tr('report.cell.conflicts'), tr('report.cell.conflictCount', { n: c.conflictCount })])
    }
    if (c.adviceCode !== undefined && c.adviceCode !== '') {
      adviceCells.push([tr('report.cell.code'), c.adviceCode])
    }
    if (adviceCells.length > 0) {
      groups.push({
        id: 'advice',
        title: tr('report.g.advice'),
        sum: tr(c.target === 'wipe-tower' ? 'report.advice.tower' : 'report.advice.model'),
        cells: adviceCells,
      })
    }
    groups.push({
      id: 'collision',
      title: tr('report.g.collision'),
      sum: tr('report.collision.sum'),
      cells: [],
    })
  }

  /*
   * **挪塔那一块**（2026-10-18）：作者要「怎么移动，多少距离，用户选择自动移动吗？
   * 还是选了自己去移动模型停止了」。数据说了算 —— `relocation` 是 `null` 就整块不摆。
   *
   * 四行：你的选择 / 塔位置（从 → 到）/ 复检 / 塔高。数字**不取整**（与挪塔那一屏同一口径）。
   */
  if (detail.relocation != null) {
    const r = detail.relocation
    const choiceText =
      r.choice === 'relocate'
        ? tr('report.reloc.choiceAuto')
        : r.choice === 'model'
          ? tr('report.reloc.choiceModel')
          : r.choice === 'timeout'
            ? tr('report.reloc.choiceTimeout')
            : tr('report.reloc.choiceNoRoom')
    const cells: [string, string][] = [[tr('report.cell.yourChoice'), choiceText]]
    const mm = (v: number | null | undefined): string =>
      v === null || v === undefined ? '—' : v.toFixed(1)
    if (r.to != null) {
      cells.push([
        tr('report.cell.towerPos'),
        tr('report.reloc.posFromTo', {
          fx: mm(r.from[0]),
          fy: mm(r.from[1]),
          tx: mm(r.to[0]),
          ty: mm(r.to[1]),
        }),
      ])
      cells.push([tr('report.cell.moved'), `${mm(r.movedMm)} mm`])
    } else {
      cells.push([
        tr('report.cell.towerPos'),
        tr('report.reloc.posNoMove', { fx: mm(r.from[0]), fy: mm(r.from[1]) }),
      ])
    }
    if (r.verified === true) {
      cells.push([
        tr('report.cell.recheck'),
        tr('report.reloc.recheckOk', { n: r.beforeConflictCount }),
      ])
    } else if (r.verified === false) {
      cells.push([
        tr('report.cell.recheck'),
        tr('report.reloc.recheckBad', { n: r.afterConflictCount ?? 0, m: r.beforeConflictCount }),
      ])
    }
    cells.push([tr('report.cell.towerHeight'), `${mm(r.towerHeightMm)} mm`])
    groups.push({
      id: 'relocation',
      title: tr('report.g.reloc'),
      sum: tr(r.to != null ? 'report.reloc.sumAuto' : 'report.reloc.sumNone'),
      cells,
    })
    /*
     * 两幅板子（2026-10-18 作者：「成功挪塔的也可以显示，画出来碰撞位置，就是把后处理的
     * 那两个板子也做进报告页面」）—— 事实跟着 `relocation.before` 走，所以成功那趟也画得出。
     *
     * ★ 只在**真的挪过**时才摆（作者 2026-10-18：「这个失败的，就是没有移动的，就不要
     *   显示塔位变化了」）：答「自己挪模型」/ 超时 / 板上没有空位时 `to` 是 `null`，
     *   图里只剩一幅「现在」加一支没有终点的箭头 —— 等于凭空说"塔要过去"。
     *   那种记录看上面「挪塔」那一组（你的选择 / 塔位置（没有挪）/ 塔高）就够了。
     */
    const moved = r.to != null && (r.to[0] !== r.from[0] || r.to[1] !== r.from[1])
    if (r.before != null && moved) {
      groups.push({
        id: 'relocation-map',
        title: tr('report.g.relocMap'),
        sum: tr('report.reloc.mapSum'),
        cells: [],
      })
    }
  }

  /* 警告：没有就不摆这一组（空组还要占一条横线，等于喊狼来了） */
  if (detail.warnings.length > 0) {
    groups.push({
      id: 'warnings',
      title: tr('report.g.warnings'),
      sum: tr('report.warnings.count', { n: detail.warnings.length }),
      cells: detail.warnings.map((w, i): [string, string] => [
        tr('report.warnings.item', { n: i + 1 }),
        w,
      ]),
    })
  }

  /* 管线计时：一步一行，步里的多行细节跟着 */
  const pipeMs = detail.pipeline.reduce((a, p) => a + (p.elapsedMs ?? 0), 0)
  groups.push({
    id: 'pipeline',
    title: tr('report.g.pipeline'),
    sum: tr('report.pipeline.sum', { n: detail.pipeline.length, ms: pipeMs.toFixed(0) }),
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
      tr('report.cell.input'),
      [
        detail.inputFile?.path ?? detail.gcodePath ?? '—',
        inMb,
        detail.inputFile?.lines === undefined
          ? null
          : tr('report.files.lines', { n: detail.inputFile.lines }),
        `sha ${shaShort(detail.inputFile?.sha256)}`,
      ]
        .filter((x): x is string => x !== null)
        .join(' · '),
    ],
    [
      tr('report.cell.output'),
      [detail.outputFile?.path ?? '—', outMb, `sha ${shaShort(detail.outputFile?.sha256)}`]
        .filter((x): x is string => x !== null)
        .join(' · '),
    ],
    [tr('report.cell.preset'), detail.presetPath ?? st.presetName ?? '—'],
  ]
  if (detail.invocation?.exe !== undefined) {
    fileCells.push([
      tr('report.cell.invoke'),
      `${detail.invocation.exe}${
        detail.invocation.args !== undefined ? ` ${detail.invocation.args.join(' ')}` : ''
      }`,
    ])
  }
  groups.push({
    id: 'files',
    title: tr('report.g.files'),
    sum: diff === null ? '' : tr('report.files.sumChanged', { diff }),
    cells: fileCells,
  })

  /*
   * **参数快照**（2026-10-21）：切片那一刻的两份原文 —— 预设 TOML 读到的全量参数
   * 与 IR 终态。放在最后、默认收起（面板里一级分区点开才铺）：它回答的是
   * "当时那一盘到底用的是哪些值、程序认成了什么"，不是一眼要看的结论。
   */
  if (detail.snapshot != null) {
    const n = Object.keys(detail.snapshot.params).length
    groups.push({
      id: 'snapshot',
      title: tr('report.g.snapshot'),
      sum: n === 0 ? tr('report.snapshot.sumEmpty') : tr('report.snapshot.sum', { n }),
      cells: [],
    })
  }

  return groups
}

/* ---------- 参数快照（2026-10-21） ---------- */

/** IR 的 JSON 里取一个对象（不是对象就给空表，绝不抛） */
function objOf(v: unknown): Record<string, unknown> {
  return typeof v === 'object' && v !== null ? (v as Record<string, unknown>) : {}
}

/** IR 的 JSON 里取一个数（不是有限数就是 `null`） */
function numOf(v: unknown): number | null {
  return typeof v === 'number' && Number.isFinite(v) ? v : null
}

/** 参数快照里的一行 */
interface SnapRow {
  key: string
  label: string
  value: string
}

/**
 * **参数快照**（2026-10-21，作者：「按修改参数里的那些参数尽量都能显示出来，
 * 只是不展开；我要看的时候可以展开看」）。
 *
 * 两级折叠：
 *   · 按 TOML 节（`toolhead` / `wiping` …）收成折叠条，**默认全部收起** ——
 *     70 多项一次铺开会把这一屏顶到几屏高，而平时只要"当时是这一份、这些值在"；
 *   · 顶部一个搜索框（中文名 / key / 值都能命中），有搜索时**自动展开命中分区**。
 *
 * 中文名从 catalog 的字段定义来（`getParamMeta`），**不在这里手抄**：
 * 注册表改名这里跟着变。摘不到（浏览器没 mock / 老版本）就退回显示 key。
 */
function ParamSnapshotPanel({ snap }: { snap: ReportSnapshot }) {
  /* 字段定义（中文名 + 数据域分区）从 catalog 来 —— 与参数页同一份，不手抄 */
  const tr = useT()
  const [defList, setDefList] = useState<CatalogParamDef[] | null>(null)
  const [query, setQuery] = useState('')
  const [open, setOpen] = useState<Record<string, boolean>>({})

  useEffect(() => {
    void api.getRuntimeCatalog().then(
      (cat) => setDefList(cat.registry.params),
      () => setDefList([]),
    )
  }, [])

  const defs = new Map((defList ?? []).map((m) => [m.key, m]))
  const all: SnapRow[] = Object.entries(snap.params).map(([key, value]) => {
    const def = defs.get(key)
    return { key, label: def?.label ?? key, value }
  })

  const q = query.trim().toLowerCase()
  const rows =
    q === ''
      ? all
      : all.filter(
          (r) =>
            r.label.toLowerCase().includes(q) ||
            r.key.toLowerCase().includes(q) ||
            r.value.toLowerCase().includes(q),
        )

  /* 分区：按 TOML 节（字段定义里有就用它的 `section`，没有就取 key 的第一段） */
  const sections: { id: string; rows: SnapRow[] }[] = []
  for (const row of rows) {
    const id = defs.get(row.key)?.section ?? row.key.split('.')[0]
    const hit = sections.find((x) => x.id === id)
    if (hit === undefined) sections.push({ id, rows: [row] })
    else hit.rows.push(row)
  }

  const searching = q !== ''
  const allOpen = sections.length > 0 && sections.every((sec) => open[sec.id] === true)

  return (
    <div className={s.snap}>
      {/*
       * **这一盘有设置没读到**（2026-10-21 作者：「报告页面能看到这些对应的错误」）：
       * 缺项不报错、切片照跑，但那一项会退回默认值 —— 结果不是用户想的那样，
       * 所以摆在最上面、用红字，而不是埋在几十行参数里让他自己发现。
       */}
      {snap.missing !== undefined && snap.missing.length > 0 && (
        <p className={s.snapBad} role="alert">
          {tr('report.snapshot.missing', { n: snap.missing.length })}
          {snap.missing.join('、')}
        </p>
      )}
      <div className={s.snapTools}>
        <input
          className={s.snapSearch}
          type="text"
          value={query}
          placeholder={tr('report.snapshot.searchPh')}
          aria-label={tr('report.snapshot.searchAria')}
          onChange={(e) => setQuery(e.target.value)}
        />
        <span className={s.snapCount}>
          {rows.length === all.length
            ? tr('report.snapshot.countAll', { n: all.length })
            : tr('report.snapshot.countSome', { shown: rows.length, all: all.length })}
        </span>
        <button
          type="button"
          className={s.snapToggle}
          onClick={() => {
            /* 搜索时"展开"没有意义（命中的本来就铺开），这颗只管清空后的全展开 */
            if (allOpen) setOpen({})
            else setOpen(Object.fromEntries(sections.map((sec) => [sec.id, true])))
          }}
        >
          {tr(allOpen ? 'report.snapshot.collapseAll' : 'report.snapshot.expandAll')}
        </button>
      </div>

      {sections.length === 0 && (
        <p className={s.snapEmpty}>{tr('report.snapshot.noHit', { q: query })}</p>
      )}

      {sections.map((sec) => {
        const shown = searching || open[sec.id] === true
        return (
          <div className={s.snapSec} key={sec.id}>
            <button
              type="button"
              className={s.snapSecHead}
              aria-expanded={shown}
              onClick={() => setOpen((cur) => ({ ...cur, [sec.id]: !shown }))}
            >
              {/* 展开箭头用**形状**不用字形（2026-10-27 作者：「点了之后会变形，不是原地转的」
                  —— `▸` 是字体字形，旋转时墨迹跟着字体度量走，看着像被拉了一下）。
                  SVG 绕自己中心转 90°，闭着朝右、开着朝下，原地转。 */}
              <Icon
                name="chevronDown"
                size={12}
                className={ui.disclosureCaret}
                data-on={shown}
              />
              <b>{sec.id}</b>
              <span className={s.snapSecCount}>{sec.rows.length}</span>
            </button>
            {shown && (
              <dl className={s.snapRows}>
                {sec.rows.map((row) => (
                  <div className={s.snapRow} key={row.key}>
                    <dt title={row.key}>{row.label}</dt>
                    <dd>{row.value === '' ? tr('report.snapshot.empty') : row.value}</dd>
                  </div>
                ))}
              </dl>
            )}
          </div>
        )
      })}

      {/* IR 终态原文：排查"程序到底认成了什么"的最后一份材料（折叠，不倒一堆 JSON） */}
      <details className={s.snapIr}>
        <summary>{tr('report.snapshot.irTitle')}</summary>
        <pre className={s.snapJson}>{JSON.stringify(snap.ir, null, 2)}</pre>
      </details>
    </div>
  )
}

/**
 * **擦料塔俯视图**（2026-10-21；同日改：复用「塔位变化」那份板子图）。
 *
 * 作者：「擦料塔的完全可以复用那一个…… 他要把那个板子形状画出来…… 也同样可以点击
 * 放大查看」「那个小虚线要保留下来」。
 *
 * 于是不再自己画一张 100×100 的示意图，而是**直接摆一份 `CollisionMap`** —— 与
 * 「塔位变化」两幅同一块板（真板轮廓 + 网格）、同一个塔长相（`TowerCoreSvg`）、
 * 同一个放大层（点图看大、Esc 收起）：
 *
 *   · **判据框**（红虚线；塔框）= 这一盘**实际跑的**那个位置 —— IR 的
 *     `Wiping.WiperX/WiperY` 优先，只有预设坐标时退回预设；
 *   · **目标虚框**（灰虚线）= 预设 TOML 里写的那个位置（`params` 的
 *     `wiping.wiper_x/wiper_y`）—— 两处一致时只有一个；
 *   · 不一致时补一支箭头（预设 → 实际）；位移小到看不见就不画（`CollisionMap` 自己判）。
 *
 * 塔身 19.58mm 与内核 `TOWER_OUTER_BOUND - TOWER_INNER_BOUND`（29.79 - 10.21）同一个数；
 * 判据框每边再让 2.21mm 给外围结构（brim 那几圈 / 斜肋伸出去的那截）—— **快照里没有
 * 内核算的那份框**（成功那趟不产 `collision`），这里按同一套常量估。外围结构种类 /
 * 参数照 IR 的 `Wiping` / `Rib` / `Sheath` 原值给，形状与参数页是同一套算法。
 *
 * ★ **可打印区不在这里给**（2026-10-21 修）：`CollisionMap` 拿不到事实里的 `printable`
 *   时会用板规格的可打印区（原点 = 机器坐标原点）。机型的 `Machine.MinX..MaxX` 是
 *   **运动范围**（A1 mini 的 -10..180），比板宽一圈，拿来映射会把塔推出板外。
 */
function TowerPlate({
  snap,
  machineType,
}: {
  snap: ReportSnapshot
  /** 报告行的机型 —— `CollisionMap` 按它查板规格（`A1_MINI` 这种） */
  machineType: string | null
}) {
  const tr = useT()
  const ir = snap.ir
  const wiping = objOf(ir.Wiping)
  const rib = objOf(ir.Rib)
  const sheath = objOf(ir.Sheath)
  const presetX = Number(snap.params['wiping.wiper_x'])
  const presetY = Number(snap.params['wiping.wiper_y'])
  const irX = numOf(wiping.WiperX)
  const irY = numOf(wiping.WiperY)

  const hasPreset = Number.isFinite(presetX) && Number.isFinite(presetY)
  const hasIr = irX !== null && irY !== null
  if (!hasPreset && !hasIr) return null

  /* 塔身 19.58mm（内核的 TOWER_OUTER_BOUND - TOWER_INNER_BOUND）。
     判据框 = 塔身 + **外围结构那一圈**：brim 是 5 圈 × 0.8mm，斜肋按伸出的那截，
     护套按 IR 的 `Sheath.BaseExpand`。★ 框给足（塔位变化那张就是这么个大框）——
     给少了，`TowerCoreSvg` 画在框沿上的外围结构会被 viewBox 裁掉（2026-10-21 真机：
     塔的绿边缺一截、像被削了角）。 */
  const BODY_MM = 19.58
  const outer =
    typeof wiping.OuterStructure === 'string' && wiping.OuterStructure !== ''
      ? wiping.OuterStructure
      : 'brim'
  const padMm =
    outer === 'sheath'
      ? Math.max(4, numOf(sheath.BaseExpand) ?? 0)
      : outer === 'rib'
        ? 6
        : 4
  const BOX_MM = BODY_MM + padMm * 2

  /** 以塔左下角为角的判据框（机器坐标；内核同一个约定） */
  const boxAt = (x: number, y: number): ReportBox4 => ({
    xMin: x,
    xMax: x + BOX_MM,
    yMin: y,
    yMax: y + BOX_MM,
  })

  const same =
    hasPreset && hasIr && Math.abs(presetX - irX) < 0.01 && Math.abs(presetY - irY) < 0.01
  /* 实际用的那个位置：程序认到的优先（报告页那句"按程序认到的那个跑"就是这张图） */
  const at = hasIr
    ? { x: irX as number, y: irY as number }
    : { x: presetX, y: presetY }

  /*
   * 一份"只有塔、没有碰撞"的事实：`CollisionMap` 拿到它就照常画板轮廓 / 网格 /
   * 塔形状 / 判据框 / 放大层 —— 擦料塔这一栏不另写一套画法。
   * `printable` 故意不给：让画布按板规格的可打印区定（理由见上面那条）。
   */
  const collision: ReportCollision = {
    towerBbox: boxAt(at.x, at.y),
    tower: {
      outerStructure: outer,
      bodyMm: BODY_MM,
      ribWidth: numOf(rib.Width) ?? 0,
      ribExtraLength: numOf(rib.ExtraLength) ?? 0,
      ribFilletWall: rib.FilletWall === true,
      sheathBaseExpand: numOf(sheath.BaseExpand) ?? 0,
    },
  }
  /* 预设写的位置单独画一个虚框（两处一致时只有一个） */
  const ghost = hasPreset && !same ? boxAt(presetX, presetY) : null
  /* 不一致时：从预设那个位置指到实际这个位置（塔框中心之间，与「塔位变化」同一支箭头） */
  const arrow =
    ghost !== null
      ? {
          from: [presetX + BOX_MM / 2, presetY + BOX_MM / 2] as [number, number],
          vec: [at.x - presetX, at.y - presetY] as [number, number],
        }
      : null

  return (
    <div className={s.plate}>
      <CollisionMap
        collision={collision}
        machineType={machineType}
        arrow={arrow}
        ghost={ghost}
        hideLegend
        height={132}
        label={tr('report.towerPlate.label')}
      />
      <p className={same || !hasIr ? s.plateNote : `${s.plateNote} ${s.plateBad}`}>
        {hasPreset && tr('report.towerPlate.preset', { x: fmtNum(presetX), y: fmtNum(presetY) })}
        {hasPreset && hasIr && ' · '}
        {hasIr && tr('report.towerPlate.ir', { x: fmtNum(irX as number), y: fmtNum(irY as number) })}
        {hasPreset &&
          hasIr &&
          tr(same ? 'report.towerPlate.same' : 'report.towerPlate.diff')}
      </p>
    </div>
  )
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
  const tr = useT()
  const n = name ?? tr('report.row.noName')
  if (n.length <= FILE_SHORT_MAX) return <>{n}</>
  return (
    <>
      <span className={s.fHead}>{n.slice(0, n.length - FILE_TAIL_LEN)}</span>
      <span className={s.fTail}>{n.slice(-FILE_TAIL_LEN)}</span>
    </>
  )
}

export default function PageReport() {
  const tr = useT()
  /* null = 还在读；读失败是另一格（err），不与空列表混 */
  const [rows, setRows] = useState<ReportSummary[] | null>(null)
  const [err, setErr] = useState<string | null>(null)
  const [note, setNote] = useState<Note | null>(null)
  /* 开发台：点「浮出的条」时摆一条提示条（in-flow、要手动关，不自己消失） */
  const devNote = useDevPreview('float-toast')
  useEffect(() => {
    if (devNote.on) {
      setNote({
        text: tr('report.note.devExport', {
          path: '/Users/wzy/Library/Application Support/SupportEase/exports/',
        }),
        bad: false,
      })
    }
  }, [devNote.on, tr])
  const [selId, setSelId] = useState<string | null>(null)
  const [detail, setDetail] = useState<ReportDetail | null>(null)
  const [detailErr, setDetailErr] = useState<string | null>(null)
  /**
   * 详情**再读一遍**的触发计数（`selId` 没变时也要读）。
   *
   * 打印时间是**延后**算的：钩子先落 `printTimeStatus = computing` 就退，常驻进程
   * 算完才回填 `detail.printTime`。若只看 `selId`，这次回填永远进不了这一屏 ——
   * 表现就是「打印时间」一格一直是「—」，非得换一条记录再切回来才亮（作者 2026-10-20 点的）。
   * 所以补算完成的事件一到、按了「刷新」，都拨一下这个数。
   */
  const [detailNonce, setDetailNonce] = useState(0)
  const reloadDetail = useCallback(() => setDetailNonce((n) => n + 1), [])
  /** 右栏现在这份 `detail` 是给哪一条读的（换记录才清空，同一条补读不闪） */
  const detailFor = useRef<string | null>(null)
  /* 语言一换就重读记录：那句「为什么没成」是后端按当前语言给的（见 `localized_error`） */
  const locale = useLocale()
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
  /* 回放：拿当前这条记录摊成「当时那一趟」，塞进原本那扇后处理模态框 */
  const [replay, setReplay] = useState<PostProcessRun | null>(null)
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
        setNote((cur) => (cur?.text === tr(READING_NOTE) ? null : cur))
      },
      (e: unknown) => {
        setRows([])
        setErr(errorText(e))
        setNote((cur) => (cur?.text === tr(READING_NOTE) ? null : cur))
      },
    )
  }, [tr])

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
      if (!alive) return
      read(true)
      /*
       * 列表挑到最新那条之后详情也得跟着换。`read(true)` 里那句 `setSelId` 在
       * 「最新那条本来就是选中的」时是同值赋值 ⇒ 详情 effect 不会重跑，所以这里
       * 显式再拨一次（那一趟刚落下时打印时间多半还是 `computing`）。
       */
      reloadDetail()
    }).then((off) => {
      /* 订阅回执比卸载晚到时就地退订（否则监听器留在后端没人收） */
      if (alive) offs.push(off)
      else off()
    })
    /*
     * 延后的打印时间**补算完了** —— 右栏正看着的那条把它补上。
     *
     * 这一条事件只带秒数、不带记录 id（`hook_ui::PrintTimeDto`），所以只是**再读一次
     * 当前选中的那条**：读错了也只多一次读盘，读对了那一格就从「—」跳到真实时间。
     */
    void listen<PostProcessPrintTime>(POSTPROCESS_PRINT_TIME, () => {
      if (alive) reloadDetail()
    }).then((off) => {
      if (alive) offs.push(off)
      else off()
    })
    return () => {
      alive = false
      for (const off of offs) off()
    }
  }, [read, reloadDetail, locale])

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
      detailFor.current = null
      setDetail(null)
      setDetailErr(null)
      return
    }
    /*
     * 换一条才清空右栏（先摆"正在读这一条的详情…"）；同一条的补读（打印时间刚补上 /
     * 按了刷新）**静默换值** —— 清一下再长回来，看着就是整页闪了一下。
     */
    const first = detailFor.current !== selId
    if (first) {
      detailFor.current = selId
      setDetail(null)
      setDetailErr(null)
    }
    let alive = true
    api.getReportDetail(selId).then(
      (got) => {
        if (alive) setDetail(got)
      },
      (e: unknown) => {
        if (!alive) return
        /* 补读失败时已经摆着的那份不动（旧值比一句红字有用）；换记录后读不到才报 */
        if (first) setDetailErr(errorText(e))
      },
    )
    return () => {
      alive = false
    }
  }, [selId, detailNonce])

  /*
   * 打印时间还在补算（`printTimeStatus === 'computing'`）时的兜底：隔一会儿再看一眼。
   *
   * 主路是 `postprocess-print-time` 那条事件；这一条只补"事件落在缝里"的那种情形
   * （订阅刚挂上 / 这一屏刚被打开那一刻正好算完）。状态一变 `ready` / `failed`
   * 这里自然停下；上限到了也停 —— 真算不出来后端会自己标 `failed`，页面不替它转圈。
   */
  const printTimePollFrom = useRef<number | null>(null)
  useEffect(() => {
    if (detail === null || detail.printTimeStatus !== 'computing') {
      printTimePollFrom.current = null
      return
    }
    const now = Date.now()
    if (printTimePollFrom.current === null) printTimePollFrom.current = now
    const left = PRINT_TIME_POLL_MAX_MS - (now - printTimePollFrom.current)
    if (left <= 0) return
    const id = setTimeout(reloadDetail, Math.min(PRINT_TIME_POLL_MS, left))
    return () => clearTimeout(id)
  }, [detail, reloadDetail])

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

  const groups = detail === null ? [] : buildGroups(tr, detail)
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
        <span className={s.sideLb}>{tr('report.side.title')}</span>
        <span className={s.sideCnt}>{tr('report.side.count', { n: (rows ?? []).length })}</span>
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
            placeholder={tr('report.side.searchPh')}
            aria-label={tr('report.side.searchAria')}
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
              {tr(label)} <b>{counts[id]}</b>
            </button>
          ))}
        </div>
      </div>

      <ul className={s.sideList}>
        {days.length === 0 && (
          <li>
            <p className={s.listEmpty}>{tr('report.side.noMatch')}</p>
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
                  {dayLabel(tr, day)} · {day}
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
                        title={`${row.gcodeName ?? tr('report.row.noName')} · ${row.startedAt ?? row.day} · ${elapsedText(tr, row.elapsedMs)}`}
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
                              {row.ok ? tr('report.row.ok') : tr('report.row.bad')}
                            </span>
                          </span>
                          <span className={s.rowSub}>
                            <span className={s.rowMeta}>{rowMeta(row)}</span>
                            <span className={s.sideDur}>{elapsedText(tr, row.elapsedMs)}</span>
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
    <section className={s.page} ref={pageRef} aria-label={tr('report.aria.page')}>
      <header className={s.head}>
        <p className={s.sub}>{tr('report.head.sub')}</p>
        <div className={s.headActions}>
          {/*
            两颗都是产品那套 `.btn`（`ui/Controls`）—— 槽上的 `[data-page] .btn`
            已经给了胶囊圆角。**不另开一套按钮皮**：全站按钮只许一套（同"模态框只许
            一套"那条），而且动作按钮一律文字、不带图标（见 `shell/icons.tsx` 那条）。

            ★ 2026-10-26 起用 `size="sm"`：这一行要与工艺页工具条同高（41px，见
              `--head-h`），而那两枚胶囊是 24px 高 —— 它们一大一小的话，两页顶条
              怎么调都对不齐（作者：「改小报告的胶囊按钮和工艺的一样」）。
          */}
          <Btn size="sm" disabled={selId === null} onClick={() => setExportOpen(true)}>
            {tr('report.action.export')}
          </Btn>
          <Btn
            size="sm"
            disabled={detail === null}
            title={detail === null ? tr('report.replay.needDetail') : undefined}
            onClick={() => {
              if (detail !== null) setReplay(replayRunOf(detail))
            }}
          >
            {tr('report.action.replay')}
          </Btn>
          <Btn
            size="sm"
            onClick={() => {
              read()
              /* 列表重读之外，右栏那条也重读 —— 打印时间可能刚从 `computing` 变成 `ready` */
              reloadDetail()
              setNote({ text: tr(READING_NOTE), bad: false })
            }}
          >
            {tr('report.action.refresh')}
          </Btn>
        </div>
      </header>

      <div className={s.noteSlot}>
        <NoteBar note={note} onClose={() => setNote(null)} />
      </div>

      <div className={s.content}>
        {err !== null ? (
          <p className={s.err} role="alert">
            {tr('report.err.list', { err })}
          </p>
        ) : rows === null ? (
          <p className={s.empty}>{tr('report.empty.loading')}</p>
        ) : rows.length === 0 ? (
          <p className={s.empty}>{tr('report.empty.none')}</p>
        ) : (
          <div className={s.pane}>
            {listHtml}
            <div className={s.scrim} onClick={() => setSideOpen(false)} role="presentation" />

            <section className={s.main}>
              {sel === null ? (
                <p className={s.empty}>{tr('report.detail.pick')}</p>
              ) : (
                <>
                  <div className={s.mainHead}>
                    <h3 className={s.mainFile} title={sel.gcodeName ?? tr('report.row.noName')}>
                      <FileName name={sel.gcodeName} />
                    </h3>
                    <span className={sel.ok ? s.chip : `${s.chip} ${s.chipBad}`}>
                      {sel.ok ? tr('report.row.ok') : tr('report.row.bad')}
                    </span>
                    <span className={s.mainWhen}>
                      {sel.startedAt ?? sel.day} · {elapsedText(tr, sel.elapsedMs)}
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
                          {tr('report.tab.all')}
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
                                      <b>{stepName(tr, step.id, i)}</b>
                                      <span className={s.stepMs}>
                                        {elapsedText(tr, step.elapsedMs ?? null)}
                                      </span>
                                      {/* 失败 / 取消的记录：最后停在的那一步给个记号
                                          （status 由钩子写侧给；旧记录没有就不摆） */}
                                      {step.status === 'error' && (
                                        <span className={s.stepBad}>{tr('report.step.stopped')}</span>
                                      )}
                                      {step.status === 'cancelled' && (
                                        <span className={s.stepStopped}>
                                          {tr('report.step.cancelled')}
                                        </span>
                                      )}
                                    </span>
                                    {stepDetailText(step, tr) !== null && (
                                      <span className={s.stepDetail}>
                                        {stepDetailText(step, tr)}
                                      </span>
                                    )}
                                  </li>
                                ))}
                              </ol>
                            ) : g.id === 'relocation-map' ? (
                              detail.relocation?.before != null ? (
                                <RelocationBoards
                                  before={detail.relocation.before}
                                  from={detail.relocation.from}
                                  to={detail.relocation.to ?? null}
                                  machineType={detail.summary.machineType}
                                />
                              ) : null
                            ) : g.id === 'snapshot' ? (
                              /* 参数快照（2026-10-21）：分区默认收起，点开才铺；下面附 IR 原文 */
                              detail.snapshot != null ? (
                                <ParamSnapshotPanel snap={detail.snapshot} />
                              ) : null
                            ) : g.id === 'collision' ? (
                              /* 碰撞位置：一张俯视图（几何缺了就整块不画，组件自己判） */
                              detail.collision !== null ? (
                                <CollisionMap
                                  collision={detail.collision}
                                  machineType={detail.summary.machineType}
                                />
                              ) : null
                            ) : (
                              <>
                                <dl className={s.kv}>
                                  {g.cells.map(([k, v], i) => (
                                    <div className={s.kvRow} key={`${k}-${i}`}>
                                      <dt>{k}</dt>
                                      <dd
                                        className={
                                          k === tr('report.cell.invoke') || k === tr('report.cell.code')
                                            ? s.mono
                                            : undefined
                                        }
                                      >
                                        {v}
                                      </dd>
                                    </div>
                                  ))}
                                </dl>
                                {/* 擦料塔那一组多一张俯视图：预设写的坐标 vs 程序认到的坐标 */}
                                {g.id === 'tower' && detail.snapshot != null && (
                                  <TowerPlate
                                    snap={detail.snapshot}
                                    machineType={detail.summary.machineType}
                                  />
                                )}
                              </>
                            )}
                          </div>
                        ))}
                      </div>
                    </>
                  )}

                  {detailErr !== null && (
                    <p className={s.err} role="alert">
                      {tr('report.detail.err', { err: detailErr })}
                    </p>
                  )}
                  {detail === null && detailErr === null && (
                    <p className={s.empty}>{tr('report.detail.loading')}</p>
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

      {/* 回放：还是那扇后处理模态框，只是数据来自这一条记录（不真跑、不落新账） */}
      {replay !== null && (
        <PostProcessModal
          view={{
            run: replay,
            dismissed: false,
            sinceMs: null,
            dismiss: () => setReplay(null),
            reopen: () => {},
            silent: false,
            quietUntil: null,
          }}
          onOpenReport={() => setReplay(null)}
        />
      )}
    </section>
  )
}
