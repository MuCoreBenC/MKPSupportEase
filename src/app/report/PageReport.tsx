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
 * 窗口挤的时候（<880px 布局宽度）列表**滑盖化**：整块常驻、只露出一截（状态点
 * 那一列 + 抓手），点一下整块滑过来盖在详情上，点遮罩或往左拖滑回去 —— 手机
 * 分屏那种手感。CSS 用容器查询（同一套 DOM 两档），JS 只管拖拽与"点了先滑开"。
 *
 * # 三态分得开
 *
 *   还没有记录   空列表 → 「还没有执行记录」+ 怎么才有记录的一句话
 *   读失败       命令抛错 → 整块错误态（带原因），**不装成空列表**
 *   记录本身坏了 照进列表（红标 + 原因）—— 藏起它就等于替钩子瞒了一次事故
 *
 * # 动作只有两件
 *
 *   刷新   重读一遍磁盘（钩子随时可能又跑了一次）
 *   展开   看这一条的详情（结论 / 打印 / 管线 / 文件）
 *
 * 「确认导出 / 清理缓存」那些**没有真实实现的动作不摆**（纪律：没有实现不给入口）。
 *
 * # 字段面（2026-10-11 补齐）
 *
 * 涂胶（总长 / 换笔 / 速度 / 笔偏移）、擦料塔（层数 / 挤出比 / 安全 Z）、切片工艺
 * （切片器版本 / 预设名 / 熨平）读 `detail.glue / tower / slicer` 三个分包，
 * 键名与旧版 `_meta.json` 一致（写侧在 `src-tauri/src/archive.rs`）；
 * 管线每步的多行事实铺在步骤名下面（`pipeline[].detail`）。**没给就不摆**：
 * 旧记录 / 失败记录没有这些键，对应整组消失，不编占位数字。
 */

import { useCallback, useEffect, useRef, useState } from 'react'

import { api, errorText } from '../../api'
import type {
  ReportDetail,
  ReportGlueFacts,
  ReportSlicerFacts,
  ReportSummary,
  ReportTowerFacts,
} from '../../api'
import NoteBar from '../shared/NoteBar'
import type { Note } from '../shared/note'
import { Btn } from '../ui/Controls'
import s from './PageReport.module.css'

/** 统计里挑给用户看的几格（键与 `_meta.json` 的 `detail.stats` 同名） */
const STAT_CELLS: [string, string][] = [
  ['towerHeight', '塔高 mm'],
  ['glueLayerCount', '涂胶层'],
  ['totalLayerNumber', '总层数'],
  ['maxZHeight', '最高 Z mm'],
  ['originalPrintTime', '切片器预估'],
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
    [
      '笔偏移 XYZ',
      glue.penOffsetX === undefined && glue.penOffsetY === undefined && glue.penOffsetZ === undefined
        ? '—'
        : [glue.penOffsetX, glue.penOffsetY, glue.penOffsetZ]
            .map((v) => (v === undefined ? '—' : fmtNum(v)))
            .join(', '),
    ],
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
    ['文件', st.gcodeName ?? '—'],
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
    ...STAT_CELLS.map(([key, label]): [string, string] => [
      label,
      stats[key] === undefined ? '—' : String(stats[key]),
    ]),
  ]
  if (detail.printTime?.segments !== undefined) {
    printCells.push(['耗时段数', String(detail.printTime.segments)])
  }
  /* 本程序估算 vs 切片器原话：两个数都真有才摆（这一段最值钱的信息） */
  const slicerSec = slicerEstimateSeconds(
    typeof stats.originalPrintTime === 'string' ? stats.originalPrintTime : undefined,
  )
  const mine = detail.estimatedPrintSeconds ?? detail.printTime?.totalSeconds
  if (slicerSec !== null && slicerSec > 0 && mine !== undefined) {
    const pct = Math.round(((mine - slicerSec) / slicerSec) * 100)
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

export default function PageReport() {
  /* null = 还在读；读失败是另一格（err），不与空列表混 */
  const [rows, setRows] = useState<ReportSummary[] | null>(null)
  const [err, setErr] = useState<string | null>(null)
  const [note, setNote] = useState<Note | null>(null)
  const [selId, setSelId] = useState<string | null>(null)
  const [detail, setDetail] = useState<ReportDetail | null>(null)
  const [detailErr, setDetailErr] = useState<string | null>(null)
  const [tab, setTab] = useState<string>(TAB_ALL)
  const [sideOpen, setSideOpen] = useState(false)
  /** 只用来告诉 CSS「走浮层档」；交互一律看 `isFloating()` 的实际几何，不认这个布尔值 */
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
  const isFloating = () => {
    const el = sideRef.current
    return el !== null && getComputedStyle(el).position === 'absolute'
  }

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

  const read = useCallback(() => {
    setErr(null)
    setRows(null)
    api.getReportList().then(
      (list) => {
        setRows(list)
        /* 默认选中最新那条：进页面就有内容看，不用先点一下 */
        setSelId((cur) => cur ?? list[0]?.id ?? null)
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
    drag.current = { x: e.clientX, y: e.clientY, dx: 0, open: sideOpen, moved: false }
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
  const onUp = () => {
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
  }

  /* ---------- 列表：按日分组 ---------- */
  const days: string[] = []
  const byDay = new Map<string, ReportSummary[]>()
  ;(rows ?? []).forEach((r) => {
    if (!days.includes(r.day)) days.push(r.day)
    const list = byDay.get(r.day)
    if (list === undefined) byDay.set(r.day, [r])
    else list.push(r)
  })

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
      <ul className={s.sideList}>
        {days.map((day) => (
          <li key={day}>
            <p className={s.dayRow}>
              {day}
              <span className={s.dayCnt}>· {byDay.get(day)?.length ?? 0} 条</span>
            </p>
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
                    <span className={s.dot} aria-hidden="true" />
                    <span className={s.sideFile}>{row.gcodeName ?? '（记录里没写文件名）'}</span>
                    <span className={s.sideDur}>{elapsedText(row.elapsedMs)}</span>
                  </button>
                  {!row.ok && row.error !== null && <p className={s.rowErr}>{row.error}</p>}
                </li>
              ))}
            </ul>
          </li>
        ))}
      </ul>
      <span className={s.grabber} aria-hidden="true" />
    </nav>
  )

  return (
    <section className={s.page} ref={pageRef}>
      <header className={s.head}>
        <h2 className={s.title}>执行报告</h2>
        <p className={s.sub}>
          后处理每跑一次（切片器导出时按「复制后处理脚本」那条命令跑），这里多一条记录。
        </p>
        <div className={s.headActions}>
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
          <div
            className={s.scrim}
            onClick={() => setSideOpen(false)}
            role="presentation"
          />

          <section className={s.main}>
            {sel === null ? (
              <p className={s.empty}>左边选一条记录看详情。</p>
            ) : (
              <>
                <div className={s.mainHead}>
                  <h3 className={s.mainFile}>{sel.gcodeName ?? '（记录里没写文件名）'}</h3>
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
                          <div className={s.grp}>
                            <p className={s.grpBar}>
                              <b>{g.title}</b>
                              {g.sum !== '' && <span className={s.grpSum}>{g.sum}</span>}
                            </p>
                            {g.id === 'pipeline' ? (
                              <ol className={s.steps}>
                                {detail.pipeline.map((step, i) => (
                                  <li key={i} className={s.step}>
                                    <span className={s.stepHead}>
                                      <b>{step.id ?? `第 ${i + 1} 步`}</b>
                                      <span className={s.stepMs}>
                                        {elapsedText(step.elapsedMs ?? null)}
                                      </span>
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
    </section>
  )
}
