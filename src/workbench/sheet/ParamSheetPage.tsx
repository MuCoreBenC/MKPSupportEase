/*
 * 参数台 —— C14 版式（feat/b05-14b-c14-port，方案见 C14-PORT-PLAN.md）。
 *
 * 版式骨架照 C14 的参数台（B02 一脉）：
 *
 *   ┌──────────────────────────────────────────────────────────────┐
 *   │ A1 / 标准版 · 73 参数 · 52 可编辑              单版本 | 版本对照 │
 *   ├──────────┬─────────────────────────────────┬─────────────────┤
 *   │ (左树rail)│ 全部 73  偏移 5  擦料 27  …     │ 参数编辑区 / 批量 │
 *   │          ├────────────────┬────────────────┤ （撑满右侧的正式  │
 *   │          │ 空间偏移        │ 擦料方式        │  编辑区）         │
 *   │          │ 参数行          │ 参数行         │                  │
 *   └──────────┴────────────────┴────────────────┴──────────────────┘
 *
 * 两种模式（C14）共用左边那棵树：
 *
 *   单版本  我要改东西 → 一行只给「当前值 / 为什么」；左树是 radio
 *   对照    我要理解差别 → `wb_matrix` 按**基准机型的基底**判差异（绿底/状态列），
 *           左树是 checkbox（可跨机型、可刷选）；右栏两页签「参数详情 / 批量修改」，
 *           批量走 `wb_preview_bulk` 先看后写
 *
 * # 与原型的分工（判据全部在后端，前端只读派生）
 *
 *  - 状态/文案/能不能改：`wb_desk` / `wb_matrix` 的 Row / Cell（text / blocked /
 *    blockedHint / editable / deprecated / differs / diffTip）；
 *  - 选项级弃用 → 后端 `ChoiceView.deprecated`；「改了影响谁」→ 后端 `Row.impact`；
 *  - 差异（仅显示差异、绿格、一致/差异/本机无此项）→ 后端 `Matrix.diffKeys` /
 *    `Cell.differs` / `Matrix.notOwnKeys`，状态词来自 `words.matrixRow`；
 *  - 写值：翻成 `Patch::setValue` 走 `wb_apply_draft`（弃用闸在后端 patch 校验 +
 *    前端手势前的 toast）；批量先 `wb_preview_bulk` 预览、确认才写；
 *  - 撤销 / 重做 / 保存 / 未保存改动在**外壳**（P1 已就位）。
 *
 * # C15 A2：模式开关一关，整张卡收起来（只留一句 + 「仍然展开看」）
 *
 * 作者的原话是「切换到圆盘擦拭的时候那些擦料塔的都隐藏」—— 切一下模式开关，
 * 属于另一支的那几张卡整张收起来，而不是留一屏灰行。**判据仍然只有后端那一份**：
 *
 *   · 整组被** section 级条件**关掉的，后端把那一组的话都写好了（`DeskGroup.offNote`，
 *     措辞在 `wording.rs` 的 `relate::group_off`；`derive.rs` 的 `group_off_note`
 *     逐项查 `blocked[].scope === 'section'` 且是同一个 key）—— 这一句优先；
 *   · 全是**字段级条件**关的（作者的实测数据里 9 条塔参数就是这种），后端不给整组的话，
 *     就摆第一行那句 `blockedHint`（同样是后端拼的整句）—— 前端一句中文都不拼。
 *
 * 前端在这里只做**版面决定**：这一卡现在一行都改不动，就把行收起来、把这句摆在卡头下面。
 * 「哪一行为什么改不动」仍旧是 `Cell.blocked` / `blockedHint` 说了算（`visibility.rs`），
 * 这一层不重新判一遍可见性 —— 那就是把业务搬回前端了。
 *
 * 两条边界，都是照 A40 客户端那一版（`ParamCardA40` 第 43 轮）定的：
 *   · **搜索态永远不收起** —— 那份列表是人自己搜出来的，「命中就该看见」（第 37 轮）；
 *   · 单行被关着仍是「看得见、改不动」：藏起来的后果是用户以为这个参数不存在。
 *     收起来的那几行留一个 「仍然展开看」（词从后端来：`words.relate.showAnyway`）。
 */
import { memo, useCallback, useEffect, useMemo, useRef, useState } from 'react'
import type {
  CSSProperties,
  KeyboardEvent as ReactKeyboardEvent,
  MouseEvent as ReactMouseEvent,
  ReactNode,
} from 'react'

import {
  isAppError,
  wb,
  type BookView,
  type Col,
  type ColRef,
  type Desk,
  type DeskGroup,
  type GroupMembersEdit,
  type MachineList,
  type Matrix,
  type MetaApplied,
  type ParamMetaEdit,
  type Patch,
  type Cell,
  type Refresh,
  type RegistryView,
  type Row,
  type SectionMetaEdit,
  type Words,
} from '../api'
import type { GotoFocus } from '../c14/types'
import { locateAnchor } from '../c14/locate'
import { toasts } from '../c14/toast'
import { useScope } from '../scopeState'
import GcodeModal from '../c14/GcodeModal'
import SplitterC14 from '../c14/SplitterC14'
import { useSplitWidth } from '../c14/useSplitWidth'
import { usePaintSelectC14 as usePaintSelect } from '../c14/usePaintSelect'
import { refreshKeyOf, type EditEntry } from '../editQueue'
import {
  applyDraftToDesk,
  applyDraftToMatrix,
  cellKeyOf,
  draftAfterWrite,
  rawOf,
  settleDraft,
  type DraftEntry,
} from './draftOverlay'
import { sheetWrites, WriteQueue } from './writeQueue'
import type { ParamView } from '../api'
/* 行右键菜单：与资产页 / 机型页同一套（见 `components/menu`） */
import { ContextMenu } from '../components/menu'
import type { ContextMenuEntry } from '../components/menu/types'
import { useContextMenu } from '../components/menu/useContextMenu'
import BatchEdit from '../views/BatchEdit'
import type { BatchTarget } from '../views/BatchEdit'
import CellEditor from '../views/CellEditor'
import CompareMatrix from '../views/CompareMatrix'
import ParamDefModal from '../views/ParamDefModal'
import { nextSectionId, nextTabId } from '../views/sectionId'
import ParamDetail, { StatusDot, type DetailLayer } from '../views/ParamDetail'
/* 分组抽屉：改名 / 换位 / 这一组的参数清单都在里头（卡头只负责打开它） */
/* 组件卡里的画布占位（作者 2026-11-01：「它就占个位置就可以了」） */
import ComponentSlot from '../views/ComponentSlot'
import { slotSideOf } from '../views/componentSide'
import SectionDrawer from '../views/SectionDrawer'
import SectionMembersModal from '../views/SectionMembersModal'
import NewTabModal from '../views/NewTabModal'
import s from '../c14.module.css'

/**
 * 目标记号：`''` = 机型基底，其余是版本 uid，`null` = 还没选（C14 第十七轮：
 * 默认谁都不选，正文是空态 —— 「还没选」这个状态必须存在）。
 */
const BASE = ''
type Target = string | null

/** 对照列的记号：`机型|版本`（版本空串 = 那台的基底）。机型的 id 里没有 `|` */
const colId = (m: string, uid: string): string => `${m}|${uid}`
/** 按第一个 `|` 切开是安全的（同上） */
const colSplit = (id: string): { m: string; uid: string } => {
  const i = id.indexOf('|')
  return { m: id.slice(0, i), uid: id.slice(i + 1) }
}

/** 两条竖线的可拖范围（C14 第三/四轮的量）。默认 176 / 320，min 是「读得出名字」那条线 */
/** 左栏「钉住 / 收起」记在哪（`'0'` = 收起；没有这个键 = 钉住） */
const RAIL_PIN_KEY = 'mkp.wb.pRail.pinned'

const RAIL = { dft: 176, min: 152, max: 320 }
/* max 620 → 760（2026-11-03）：详情栏那几行要摆两颗回退按钮 + 控件，620 常常不够 */
const ASIDE = { dft: 320, min: 240, max: 760 }

/**
 * 「页面上的行」有多少条。与表头那个 `total` **不是**一个数 ——
 * 表头的 `total` 是这一页的参数条数（`Desk.total`），这里数的是眼下真被画出来的行：
 * 收起来的卡一行都不画，条件关着的行算的也是"没画"。
 *
 * 用它判"某一组现在页面上有没有东西"，抽屉里那句「页面上看不到它」就是按它算的。
 */
function drawnRowCount(desk: Desk, cur: number, searching: boolean, opened: string[]): number {
  let n = 0
  for (const g of desk.groups) {
    /* 与渲染那一支同一条判据（`collapseNoteOf`）：搜的时候卡不收起 */
    if (collapseNoteOf(g, cur, searching) !== null && !opened.includes(g.sectionId)) continue
    n += g.items.reduce((m, it) => m + 1 + it.children.length, 0)
  }
  return n
}

interface Props {
  book: BookView
  words: Words
  /** 检查页 / 机型页「去处理」带过来的定位。挂载时读一次 */
  initialFocus?: GotoFocus | null
  /** 后端状态变过了就 +1（外壳每次写完会拨） */
  tick: number
  /** 草稿脏不脏（放弃 / 保存按钮） */
  dirty: boolean
  /**
   * 唯一写入口（外壳的 run）。`refresh` 让后端顺带把那一页带回来 —— 不必事后再问一次。
   *
   * `opts.quiet` = **值写入那一路**（见 `editQueue` 头注）：外壳不拨 `tick`、
   * 不置 `busy`，这一页要的那份派生改吃返回值里的 `desk` / `matrix`。
   * `ok` 说这笔到底写进去没有（`false` 时队列要把乐观值回落掉）。
   */
  onApply: (
    label: string,
    patches: Patch[],
    refresh?: Refresh,
    opts?: { quiet?: boolean },
  ) => Promise<{ ok: boolean; desk: Desk | null; matrix: Matrix | null }>
  /** 「编辑定义」保存成功后交一条给外壳 —— 压进撤销栈（改前/改后整包） */
  onMetaApplied: (m: MetaApplied) => void
  /**
   * 别的结构编辑（分组改名 / 换位）落进草稿之后，让外壳把事务状态刷一下 ——
   * 状态栏那条「未保存改动（N）」是服务器侧那条栈的数字，前端不自己记
   */
  onEditApplied: () => void
  /** 外壳的保存 / 撤销（「放弃」也归外壳那对按钮 —— 这一页不再画自己的一对） */
  onSave: () => void
  onUndo: () => void
  /** 「回到版本页」—— 正在编辑某版时跳回机型与版本页 */
  onGoto: (view: string, focus?: GotoFocus) => void
}

export default memo(ParamSheetPage)

function ParamSheetPage({ book, words, initialFocus, tick, dirty, onApply, onMetaApplied, onEditApplied, onSave, onUndo, onGoto }: Props) {
  /* 定位只看挂载时的那一次 —— 之后就是普通的本页状态 */
  const init = initialFocus ?? null
  /*
   * 「正在编辑 A1/FASTV3.3」那枚提示。它是一个状态，不是常量 ——
   * 换了机型或换了正在改的那一版，它还挂着上一版就是在说谎。
   */
  const [focusUid, setFocusUid] = useState<string | null>(init?.uid ?? null)

  /* 单版本模式的「正在改谁」。两个都默认 null：正文显示空态（C14 第十七轮） */
  const [machineId, setMachineId] = useState<string | null>(init?.machineId ?? null)
  const [target, setTarget] = useState<Target>(init?.uid ?? null)
  /** 单版本 | 版本对照（C14 第八轮起的双模式） */
  const [mode, setMode] = useState<'single' | 'compare'>('single')
  /** 顶上 tab = 一级参数领域。`null` = 全部 */
  const [tabSel, setTabSel] = useState<string | null>(null)
  const [q, setQ] = useState('')
  /** 右栏详情读的那一项 */
  const [sel, setSel] = useState<string | null>(init?.key ?? null)
  /** 对照模式的列（左树勾选，可跨机型）。默认空 —— 「可以都不选，空着」 */
  const [compareCols, setCompareCols] = useState<string[]>([])
  /** 仅显示差异（对照模式） */
  const [diffOnly, setDiffOnly] = useState(false)

  /*
   * 撤销作用域（2026-11-14）：选着某一版时**只认那一版**。
   * 不能连机型一起认 —— 那会让别的版本的改动也算成"这一屏的"。
   */
  useScope([target === null ? (machineId ?? '') : target])
  /**
   * 「仍然展开看」摊开的那几张卡（C15 A2）。**只装 session 里点开过的** ——
   * 收不收起来每次渲染按后端给的行态现判，所以条件一变回来，卡自己就正常了。
   * 2026-10-03 作者：「展开了之后，还有按钮可以把它折回来」—— 摊开的卡上
   * 长一颗「收起来」，点了就把这一张从这里请出去（收起判据照旧在后端字段里）。
   */
  const [openedCards, setOpenedCards] = useState<string[]>([])
  /** 分组抽屉开着的是哪一组（`null` = 关着）。改名 / 换位 / 参数清单都在抽屉里 */
  const [drawerSection, setDrawerSection] = useState<string | null>(null)
  /** 页签栏那支笔开的是哪一个页签（null = 关着） */
  const [membersTab, setMembersTab] = useState<string | null>(null)
  /** 「＋ 新建页签」那个小框开着没有 */
  const [tabAdding, setTabAdding] = useState(false)
  /*
   * 抽屉里那颗「当前」标记住的是哪一组 —— **不一定是 `drawerSection`**。
   *
   * 作者 2026-11-01 指着截图问：「我的里面没有这个了但是侧边抽屉还有这个，是谁错了」。
   * 图里抽屉顶上是 `glue_advanced_motion`、页面上亮着的是「高级运动调控」那一行，
   * 而右边那棵分组清单里却把「高级运动调控」也标成了当前 —— 同一屏两个"当前"。
   *
   * 根源是**两件事被塞进了一个变量**：抽屉是"你点开的哪一张卡"，"当前"是"页面此刻
   * 把哪一组当主体"（高亮、页头的「第 N / 共 M 组」）。`onShowSection` 每次都会把
   * `sectionId` 一并改掉，于是"看别的组"这个动作顺手把"当前"也挪走了；未落盘的
   * 新组、组件卡（注册表里查不到）更明显 —— 抽屉退回"第一个组"当当前。
   *
   * 现在分开：`drawerAnchor` 才是当前（打开抽屉时同步成你点的那一组），抽屉标题与
   * 那个「当前」标记都跟着它；换看另一组只换抽屉内容，不换当前。
   */
  const [drawerAnchor, setDrawerAnchor] = useState<string | null>(null)
  /** 正在编辑定义的参数（2026-10-03。「编辑定义」模态框，null = 关着） */
  const [defEditKey, setDefEditKey] = useState<string | null>(null)
  /**
   * 参数行的右键菜单（2026-11-02 作者：「帮我加上右键菜单，可以直接右键编辑」）。
   *
   * 与资产页 / 机型页同一套 `useContextMenu`：熟手的第二条路，**不引入新动作** ——
   * 菜单里每一项在界面上都已经有对应入口（右栏「编辑定义」、右栏各版本取值…），
   * 文案照抄那些按钮，不另起说法。
   */
  const rowMenu = useContextMenu<string>()
  /**
   * 定义落盘的计数。定义一变，行的分组/文案/可改性都可能变 —— 挂进三处
   * 派生数据的 effect 里当一次「手动 tick」（`tick` 是外壳的，只管值）
   */
  const [metaTick, setMetaTick] = useState(0)
  /** 对照模式右栏的两页签：false = 参数详情，true = 批量修改 */
  const [batchTab, setBatchTab] = useState(false)
  /** 多行 G-code 的模态框：存「哪一层」—— 按钮只在框右上角那枚（C14 第八轮） */
  const [gcodeOpen, setGcodeOpen] = useState<{ key: string; mid: string; uid: string | null } | null>(null)

  const [registry, setRegistry] = useState<RegistryView | null>(null)
  const [desk, setDesk] = useState<Desk | null>(null)
  const [matrix, setMatrix] = useState<Matrix | null>(null)
  /** 对照模式右栏「参数详情」用的那一屏（基准机型的全部层） */
  const [drawerDesk, setDrawerDesk] = useState<Desk | null>(null)
  const [error, setError] = useState<string | null>(null)

  /**
   * **本地草稿层**（这一页的内核，2026-11-02）。
   *
   * 键是 `cellKey`（机型|版本|参数），值是 TOML 原文 + 一个 `clean` 标记
   * （用户已经改回保存值、只等后端跟上）。它替换了旧页的 `editQueue` 乐观层：
   *
   *   · 显示与徽章**只看它**，不等后端往返（点下去立刻就变）；
   *   · 后端把值写回来之后（`cell.raw` 与它相等）整条删掉 —— 见 `settleDraft`；
   *   · 保存 / 放弃 / 撤销 / 换目标时整层清掉。
   */
  const [draft, setDraft] = useState<Map<string, DraftEntry>>(() => new Map())


  const scrollRef = useRef<HTMLDivElement>(null)
  const bodyRef = useRef<HTMLDivElement>(null)
  const [railW, setRailW] = useSplitWidth('pRail', RAIL.dft, RAIL.min, RAIL.max)
  const [asideW, setAsideW] = useSplitWidth('pAside', ASIDE.dft, ASIDE.min, ASIDE.max)

  /*
   * 左 rail 的三种形态：collapsed 收起 / float 悬停浮出 / pinned 钉住（**默认**）。
   * 默认钉住（作者：「我希望这个默认可以展开的」）：一进来就看得见机型与版本这棵树，
   * 不用先去点页头那枚把手。点把手仍可收起 —— 收起后鼠标扫过把手临时浮出，
   * 切版本照样零点击。
   */
  /*
   * ★ 折叠状态**记在本地**（2026-11-03 作者：「不管窗口宽度大小，我都能主动折叠，
   * 而且记住」）。以前是裸 `useState(true)`：切页 / 重开软件就自己弹回来，
   * 每次都要重新收一遍。与栏宽同一个命名空间（`mkp.wb.…`）。
   * **没有任何宽度条件** —— 窗口多小都折得动。
   */
  const [railPinned, setRailPinned] = useState(() => {
    try {
      return localStorage.getItem(RAIL_PIN_KEY) !== '0'
    } catch {
      return true
    }
  })
  const [railFloat, setRailFloat] = useState(false)
  const railOpen = railPinned || railFloat
  useEffect(() => {
    try {
      localStorage.setItem(RAIL_PIN_KEY, railPinned ? '1' : '0')
    } catch {
      /* 隐私模式 / 配额满：记不住不算错误 */
    }
  }, [railPinned])
  /* 把手在页头：离开后延时 220ms 收起（够走过去、又不至于赖着不走） */
  const railTimer = useRef<number | null>(null)
  const openFloat = () => {
    if (railTimer.current !== null) {
      window.clearTimeout(railTimer.current)
      railTimer.current = null
    }
    if (!railPinned) setRailFloat(true)
  }
  const closeFloatSoon = () => {
    if (railTimer.current !== null) window.clearTimeout(railTimer.current)
    if (railPinned) return
    railTimer.current = window.setTimeout(() => setRailFloat(false), 220)
  }
  useEffect(
    () => () => {
      if (railTimer.current !== null) window.clearTimeout(railTimer.current)
    },
    [],
  )

  /*
   * 注册表（参数的元数据）。开场取一次；此后外壳每写一笔（tick 拨一格）就
   * 重取 —— 定义编辑的撤销/重做走外壳那条栈，回来时只有 tick 会动，注册表
   * 得自己跟上（值的改动顺带重取一次很便宜，注册表就这么几十条）。
   */
  useEffect(() => {
    void wb
      .registry()
      .then(setRegistry)
      .catch((e: unknown) => setError(isAppError(e) ? e.message : String(e)))
  }, [tick])

  const paramOf = useCallback(
    (key: string) => registry?.params.find((p) => p.key === key) ?? null,
    [registry],
  )

  const machine =
    machineId !== null ? (book.machines.find((m) => m.id === machineId) ?? null) : null

  /*
   * 差异的基准机型（C14 第六轮）：单版本模式选了谁就是谁；对照模式**不预设
   * 当前机型** —— 基准取「树序里第一条被勾中列」所属的那台。两处都没有时空串
   * （没有基准，矩阵不出现差异）。
   */
  const baseMachineId =
    machine?.id ?? (compareCols.length > 0 ? colSplit(compareCols[0]).m : '')

  const refresh: Refresh = useMemo(
    () => ({
      page: 'desk',
      machineId: machineId ?? '',
      uid: target === BASE ? null : target,
      tab: tabSel,
      query: q,
    }),
    [machineId, target, tabSel, q],
  )

  /**
   * 当前这一页 refresh 的记号 —— 判断"回来的派生还是不是这一页的"（切机型 / 改搜索就作废）。
   *
   * ★ **按模式算**（2026-10-29 修）：对照模式写回时递的是 `page: 'matrix'` 的
   * `refresh`（见 `onWrite` 里 `refreshKind === 'matrix'` 那一支），而这里曾经恒按
   * `page: 'desk'` 算 —— 两个串前缀都不同，`sendEdits` 于是把矩阵模式**每一次**
   * 写回的响应都当成"过期派生"丢掉。表现就是作者看到的：手动输入后格子只变红、
   * 数字还是旧的（编辑器开着时读 `raw` 才看得见，所以只有步进箭头"像是对的"）。
   */
  const refreshKeyRef = useRef('')

  /*
   * 草稿层（新内核）：改过的格子在**后端跟上之前**一直显示我的值。
   *
   * 与旧页那套乐观层（`editQueue` 里的条目落地也不撤）的区别只有一条但很关键：
   * 这里会在后端把值写回来之后**把这一条删掉**（`settleDraft`），所以"草稿"
   * 与"后端已经一致"是两件事，不会出现"徽章要等保存才灭"。
   *
   * `shown*` 是**只画不改**的副本，base 仍由响应与 effect 写。
   */
  const shownDesk = useMemo(() => applyDraftToDesk(desk, draft), [desk, draft])
  const shownDrawerDesk = useMemo(() => applyDraftToDesk(drawerDesk, draft), [drawerDesk, draft])
  const matrixOv = useMemo(() => applyDraftToMatrix(matrix, draft), [matrix, draft])

  /* 后端跟上来了的那几条草稿可以收掉（判据见 `settleDraft`） */
  useEffect(() => {
    setDraft((prev) => {
      const next = settleDraft(desk, prev)
      return next === prev ? prev : next
    })
  }, [desk])

  /*
   * 抽屉里那句「页面上看不到它」用的是**页面自己那两个数**（参数条数 / 画出来的行数），
   * 不从注册表另算一遍 —— 表头写着 74 条、底下只有 18 行，差的那些就是收起或关掉的。
   */
  const pageTotal = shownDesk?.total ?? 0
  const pageDrawn = useMemo(
    () =>
      shownDesk === null
        ? 0
        : drawnRowCount(shownDesk, shownDesk.cur, q.trim() !== '', openedCards),
    [shownDesk, q, openedCards],
  )

  /** 抽屉里「这一组有哪些参数」：行序与卡片里一致（父项在前、子项紧跟） */
  const drawerRows = useMemo(() => {
    const g = shownDesk?.groups.find((x) => x.sectionId === drawerSection) ?? null
    if (g === null) return []
    return g.items.flatMap((it) => [it.row, ...it.children]).map((r) => ({ key: r.key, label: r.label }))
  }, [shownDesk, drawerSection])


  /**
   * 队列把一批排好的改动交到这里 —— **一次 `apply_draft` 落完**，回来顺手把那一页
   * 派生收下（后端按 `refresh` 现算，省掉事后再问一次）。
   */
  const sendEdits = useCallback(
    async (batch: EditEntry[]): Promise<boolean> => {
      const first = batch[0]
      const label = batch.length === 1 ? first.label : `${first.label} 等 ${batch.length} 处`
      const out = await onApply(
        label,
        batch.map((e) => e.patch),
        first.refresh,
        { quiet: true },
      )
      if (!out.ok) return false
      /* 回来的得是**当前这一页**的派生：切了机型 / 改了搜索，旧的那份不能顶上来 */
      if (refreshKeyOf(first.refresh) !== refreshKeyRef.current) return true
      if (first.refresh.page === 'desk' && out.desk !== null) setDesk(out.desk)
      else if (first.refresh.page === 'matrix' && out.matrix !== null) setMatrix(out.matrix)
      return true
    },
    [onApply],
  )

  const compareColRefs: ColRef[] = useMemo(
    () =>
      compareCols.map((id) => {
        const { m, uid } = colSplit(id)
        return { machineId: m, versionUid: uid === '' ? null : uid }
      }),
    [compareCols],
  )

  /* 与 `onWrite` 递出去的 `pageRefresh` 用同一套构造（page / 列序 / 查询都要一字不差） */
  const refreshOfPage: Refresh = useMemo(
    () =>
      mode === 'compare'
        ? { page: 'matrix', cols: compareColRefs, tab: null, query: q }
        : refresh,
    [mode, compareColRefs, refresh, q],
  )
  refreshKeyRef.current = refreshKeyOf(refreshOfPage)

  /*
   * **写入队列（新内核）**：单飞 —— 前一发没回来，后一发根本不会发。
   *
   * 旧页那条队列是"120ms 尾随合批 + 一次发一批"：连点同一个步进器时会攒成一批，
   * 但跨格并发那一路（80 发出、90 发出、后者先回来）靠 `seq` 兜不住。这里一次只放
   * 一条出去，顺序就是发起顺序，落盘结果不可能被旧值覆盖。
   */
  const sendRef = useRef(sendEdits)
  sendRef.current = sendEdits
  const refreshRef = useRef(refreshOfPage)
  refreshRef.current = refreshOfPage
  const draftRef = useRef(draft)
  draftRef.current = draft

  const [queue] = useState(
    () =>
      new WriteQueue({
        send: async (e) => {
          const entry: EditEntry = {
            cellKey: e.cellKey,
            label: e.label,
            patch: e.patch,
            raw: draftRef.current.get(e.cellKey)?.raw ?? '',
            refresh: refreshRef.current,
            refreshKey: refreshKeyOf(refreshRef.current),
            sent: false,
          }
          const ok = await sendRef.current([entry])
          if (!ok) throw new Error(entry.label)
        },
        onFail: (cellKey, err) => {
          /* 这一格的草稿退回后端那份（不是"上一次成功的值"—— 那个可能已经过期） */
          setDraft((prev) => {
            if (!prev.has(cellKey)) return prev
            const next = new Map(prev)
            next.delete(cellKey)
            return next
          })
          toasts.push(isAppError(err) ? err.message : String(err))
        },
      }),
  )

  /**
   * 把这一页的队列挂到外壳上（见 `sheetWrites`）：**保存前必须排空**，
   * 后端把改动撤掉了（放弃 / 撤销 / 重做）就丢草稿。页头一个字不动。
   */
  useEffect(() => {
    sheetWrites.register(
      async () => {
        await queue.drain(5000)
      },
      () => {
        queue.cancelAll()
        setDraft(new Map())
      },
    )
    return () => sheetWrites.unregister()
  }, [queue])

  /* 一屏配方台。tab / 搜索的过滤在后端（搜索跨分类，B02 的行为） */
  useEffect(() => {
    if (mode !== 'single' || machineId === null || target === null) {
      return
    }
    void wb
      .desk(machineId, target === BASE ? null : target, tabSel, q)
      .then(setDesk)
      .catch((e: unknown) => setError(isAppError(e) ? e.message : String(e)))
  }, [mode, machineId, target, tabSel, q, tick, metaTick])

  /*
   * 「去处理」带过来的定位（C14 第二十四轮 / C15 同款）：等这一屏配方台读完，
   * 滚到那一项 + 闪一下。锚点 id 挂在 ParamLine 上（`t-param-<key>`）。
   * **只做一次** —— 之后用户在本页自己滚来滚去不该被拽回去。
   */
  const locatedRef = useRef(false)
  useEffect(() => {
    if (locatedRef.current) return
    const key = init?.key
    if (!key || desk === null || mode !== 'single') return
    locatedRef.current = true
    /* effect 在 DOM 提交之后跑，行已经在树里 —— 直接量、直接滚 */
    locateAnchor(`t-param-${key}`, { row: true })
  }, [init, desk, mode])

  /*
   * 「刚改完，带我去看它落在哪」（作者 2026-11-01：「改到这个位置，不用我关掉那个
   * 编辑模态框它就帮我切换过来」）。
   *
   * 锚点 id 在写成功那一刻就算好（行 `t-param-<key>` / 卡 `t-sec-<id>`），
   * **等这一屏重取回来**再滚 —— 换组之后那一行在新卡里，旧 DOM 已经不在了。
   * 只做一次：之后用户自己滚，不该被拽回去。
   *
   * ★ 2026-11-01 第二轮：**锚点找不到就先留着**。抽屉里「点另一组」时，那一组可能
   *   在**别的领域（tab）**下 —— 这里先把 `tabSel` 切过去，可这一趟 `desk` 还是旧
   *   领域的那份（后端刚从搜索态回来也一样）：此刻 `getElementById` 是空的，旧写法
   *   把 `pendingLocateRef` 一清，锚点就永远没人滚了。现在留着等下一份 `desk`。
   */
  const pendingLocateRef = useRef<string | null>(null)

  /*
   * 打开抽屉（任何入口）⇒ 把「当前」同步成这一组。
   *
   * 同步只发生在**抽屉换了主人**这一刻：`onShowSection` 是"在抽屉里换看另一组"，
   * 它不该改当前（那只管抽屉里看什么）—— 所以它把 `drawerAnchor` 再写回旧的。
   * 挂在 effect 上是为了覆盖所有入口（卡头、收起卡、抽屉里的换组），不必每处都记得。
   */
  useEffect(() => {
    if (drawerSection === null) return
    setDrawerAnchor((prev) => (prev === drawerSection ? prev : drawerSection))
  }, [drawerSection])
  useEffect(() => {
    const anchor = pendingLocateRef.current
    if (anchor === null || desk === null) return
    if (document.getElementById(anchor) === null) return
    pendingLocateRef.current = null
    locateAnchor(anchor, { row: anchor.startsWith('t-param-') })
  }, [desk])

  /*
   * 对照一屏：差异/行序/not_own 全部由后端按基准机型判（C14 第四轮）。
   * **tab 不递给后端**（2026-10-03）：矩阵一律取全量行，领域过滤挪到下面
   * `shownMatrix` 与「仅显示差异」同一处做 —— 之前 tab 一递，页签计数只能
   * 从过滤后的行里数，点了哪个领域其余页签就被数成 0 滤没了（作者：
   * 「必须返回到全部才能看到其他」）。附带的好处：切领域不再过一遍后端。
   */
  useEffect(() => {
    if (mode !== 'compare' || compareColRefs.length === 0) {
      setMatrix(null)
      return
    }
    void wb
      .matrix(compareColRefs, null, q, baseMachineId)
      .then(setMatrix)
      .catch((e: unknown) => setError(isAppError(e) ? e.message : String(e)))
  }, [mode, compareColRefs, q, baseMachineId, tick, metaTick])

  /* 对照模式右栏「参数详情」的数据：基准机型的全部层（选了参数才取） */
  useEffect(() => {
    if (mode !== 'compare' || sel === null || baseMachineId === '') {
      setDrawerDesk(null)
      return
    }
    void wb
      .desk(baseMachineId, target === BASE ? null : target, null, '')
      .then(setDrawerDesk)
      .catch(() => setDrawerDesk(null))
  }, [mode, sel, baseMachineId, target, tick, metaTick])

  /** 机器的中文名（对照模式的层标签用） */

  /** 层的中文名（撤销按钮的 label、G-code 模态框副标题共用） */
  const layerLabelOf = (mid: string, layerUid: string | null): string => {
    const m = book.machines.find((x) => x.id === mid)
    if (layerUid === null) return `${m?.display ?? mid} · ${words.level.machine.label}`
    const v = m?.versions.find((x) => x.uid === layerUid)
    return `${m?.display ?? mid} / ${v?.name ?? layerUid}`
  }

  /**
   * 「编辑定义」的确认保存（2026-10-03 第二轮）：落盘、把改前/改后两份整包
   * 交给外壳压撤销栈（状态栏那颗「撤销」管得到定义了）、关框。抛错原样抛回
   * —— 模态框留着框，把话摆出来。
   */
  const commitDef = async (key: string, before0: ParamView, edit: ParamMetaEdit) => {
    const next = await wb.setParamMeta(key, edit)
    onMetaApplied({ label: `定义 · ${edit.label}`, key, before: metaOf(before0), after: edit })
    setRegistry(next)
    setDefEditKey(null)
    setMetaTick((t) => t + 1)
    /*
     * 定义框里那颗「确认保存」**也只是应用**（2026-11-01 作者定的口径：
     * 「只有右上角的保存才是真的保存，其他的都只是那种应用……我点了右上角的放弃，
     * 那就真的是放弃，点了保存才真的是保存」）。
     *
     * 所以这里有两件事不能漏：
     *   · 别叫"写回文件" —— 它此刻只进了草稿（要看真落盘没有，看右上角那颗）；
     *   · 把状态栏那颗红色的「未保存改动（N）」与它的撤销栈刷出来 —— 不刷的话
     *     界面上写着"已保存 / 未保存改动（0）"，而草稿里已经躺着两笔。
     */
    onEditApplied()
    /*
     * 存完就带人去看它落在哪（作者 2026-11-01：「改到这个位置，它好像没帮我自动
     * 切换到对应的位置」）：
     *
     *   · 切到**目标分组所在的页签**（新建的分组就切它挂的那个）；
     *   · 搜索框里有字就先清掉 —— 搜不到等于没跳；
     *   · 锚点记下来，等这一屏重取回来再滚 + 闪（换组之后那一行在新卡里）。
     */
    const tabId =
      edit.newSection?.tabId ?? next.sections.find((x) => x.id === edit.sectionId)?.tabId ?? null
    if (tabId !== null) setTabSel(tabId)
    if (q.trim() !== '') setQ('')
    pendingLocateRef.current = `t-param-${key}`
    toasts.push(`定义已应用（保存才落盘）· ${edit.label}`, { label: '撤销', run: onUndo })
  }

  /**
   * 译文台账被清了几格（定义框里那颗「已经改好过」，2026-11-02）。
   *
   * 和 `commitDef` 同一副骨架，但**不压撤销栈**：这一笔改的是"台账"这件记账，
   * 不是参数定义；撤销一条"我把标记清了"的条目没有意义（真要回退，工作台「放弃」兜着）。
   * 唯一不许省的还是那一句：刷「未保存改动（N）」—— 它此刻只进了草稿。
   */
  const applyStaleCleared = (next: RegistryView, key: string) => {
    setRegistry(next)
    setMetaTick((t) => t + 1)
    onEditApplied()
    toasts.push(`译文标记已清（保存才落盘）· ${key}`)
  }

  /**
   * 成员编辑框确认：一次应用进草稿，右上角「保存」才落盘。
   * 与 `saveSection` 同一跳 —— 换注册表视图、重取桌面、刷「未保存改动」。
   */
  const applyGroupMembers = async (edit: GroupMembersEdit) => {
    const next = await wb.editGroup(edit)
    setRegistry(next)
    setMetaTick((t) => t + 1)
    onEditApplied()
    const name =
      edit.scope === 'section'
        ? (next.sections.find((x) => x.id === edit.id)?.label ?? edit.id)
        : (next.tabs.find((x) => x.id === edit.id)?.label ?? edit.id)
    toasts.push(`成员已应用（保存才落盘）· ${name}`, { label: '撤销', run: onUndo })
  }

  /** 页签栏末尾那颗笔：编辑当前页签的名字与成员。两处页签栏共用这一段 */
  const editTabId = tabSel
  const tabEditButton =
    editTabId !== null &&
    registry !== null &&
    registry.tabs.some((t) => t.id === editTabId) ? (
      <button
        type="button"
        className={s.pTabEdit}
        aria-label="编辑当前页签"
        title="编辑这个页签的名字与成员"
        onClick={() => setMembersTab(editTabId)}
      >
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
          <path d="M12 20h9" />
          <path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z" />
        </svg>
      </button>
    ) : null

  /**
   * 页签栏末尾那颗「＋」：新建一个页签。
   *
   * 与分组那颗「＋ 新增分组」同一个位置语言 —— 都长在它管的那一排的末尾。
   * 名字就地输，id 自动取 `custom_tab_N`（id 会进两份 TOML，中文当不了 id）。
   */
  const tabAddButton = (
    <button
      type="button"
      className={s.pTabEdit}
      aria-label="新建页签"
      title="新建一个页签"
      onClick={() => setTabAdding(true)}
    >
      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
        <path d="M12 5v14M5 12h14" />
      </svg>
    </button>
  )

  /**
   * 组内顺序排完了（排序模态框自己落盘）。与上一条同一跳：
   * 换掉注册表视图、让桌面与右栏按新次序重取。
   */
  const applySectionOrder = (next: RegistryView) => {
    setRegistry(next)
    setMetaTick((t) => t + 1)
    onEditApplied()
  }

  /**
   * 卡片头交上来的：改分组名 / 换这一组在页签里的位置。
   *
   * 位置只给方向 —— 后端一次写完两处（注册表数组次序给客户端、`order` 给卡片），
   * 回来的是整份注册表视图；桌面重取一次，然后滚到那张卡上闪一下，好让人看见
   * 它挪到哪儿去了（作者原话：「没办法去换它那个上下位置」）。
   */
  /**
   * 删掉一个分组。组里的参数不会跟着没：后端先把它们落到这一页末尾的「未归类」。
   *
   * 与改名、换位走**同一条路**：这一笔只是应用进草稿，右上角「保存」才真落盘，
   * 那之前「放弃」就退回来。
   *
   * 两道门在后端（是组件卡 / 带 showWhen）—— 这里不预判，拒了就把那句话原样报出来。
   *
   * # 抽屉不关（2026-11-02 作者：「删除之后的刷新，不要关闭我的抽屉」）
   *
   * 早先是"先关抽屉再重取"，理由是抽屉正对着的对象已经没了。但那是把**界面反应**
   * 当成**删除本身**：用户正在这一栏里做一串整理（删一个、改一个名、再挪一位），
   * 每删一个就把抽屉关掉，等于每次都把他踢出工作区。
   *
   * 现在：抽屉留着，由**这里**把它跟到同一页签的下一个还在的组（见下面那段），
   * 页面重取一次，其余什么都不动。`SectionDrawer` 里那段 `deletedFallback` 只是
   * 兜底（撤销/重做等别的路径把 id 弄丢时用），主路径在这里。
   */
  const removeSection = async (sectionId: string) => {
    try {
      /* 名字、页签、以及"下一站在哪一组"**都要先取**：`next` 里这一组已经没了 */
      const before = registry?.sections ?? []
      const gone = before.find((x) => x.id === sectionId) ?? null
      const name = gone?.label ?? sectionId
      /* 组里那几条参数会被后端搬到同一页签末尾的「未归类」—— 条数先数下来 */
      const moved = (registry?.params ?? []).filter((p) => p.sectionId === sectionId).length
      const next = await wb.removeSection(sectionId)

      /*
       * ★ 删完把抽屉**挪到还在的那一组**（2026-11-02 作者：「删除了之后出现这个」——
       * 抽屉顶上写着 `glue_advanced_motion`、副标题写「注册表里没有这一组」）。
       *
       * 原因是抽屉正对着的 id 已经不存在，而标题取 `here?.label ?? fallbackLabel ??
       * mark ?? sectionId` —— 查不到就退回**id 原文**。`SectionDrawer` 里那个
       * `deletedFallbackRef` 只在"上一帧还认得"时才有值，首帧就是删除后那次渲染，
       * 它还没来得及记，所以兜不住。
       *
       * 只靠组件内部兜底是猜的；**宿主知道得更多**：删除只少一条，同一页签里紧邻的
       * 那一组就是"同一位置上的下一组"（删的是最后一个就退到前一个）。
       * 这里直接把两个 id 都拨过去，「当前」与抽屉内容一起跟过去，界面不出现空洞。
       */
      if (drawerSection === sectionId) {
        const sibs = before.filter((x) => x.tabId === (gone?.tabId ?? ''))
        const at = sibs.findIndex((x) => x.id === sectionId)
        const nextOne = sibs[at + 1] ?? sibs[at - 1] ?? null
        setDrawerSection(nextOne?.id ?? null)
        setDrawerAnchor(nextOne?.id ?? null)
      }

      setRegistry(next)
      setMetaTick((t) => t + 1)
      onEditApplied()
      if (sel === sectionId) setSel(null)
      toasts.push(
        moved === 0
          ? `分组已删除（保存才落盘）· ${name}`
          : `分组已删除，${moved} 条参数已移到「未归类」（保存才落盘）`,
        { label: '撤销', run: onUndo },
      )
    } catch (e) {
      toasts.push(isAppError(e) ? e.message : String(e))
    }
  }

  /**
   * 新建一个空分组（抽屉里每个页签那一行右边的「＋ 新增分组」）。
   *
   * 与删除对称：**只进草稿**，右上角「保存」才落盘。`id` 前端给（`custom_N`），
   * 与参数定义框那条同一条口径 —— 撤销/重做把同一笔载荷原样重放时，后端按
   * 「确保这个分组在」处理（`ensure_section` 幂等）。
   *
   * 建完把抽屉切到新组上：那正是"我刚建的那个"，列在它自己名下（0 条参数）、
   * 也就立刻能删（误建的情形）。不关抽屉。
   */
  const addSection = async (tabId: string, label: string) => {
    try {
      /*
       * id 的挑法与参数定义框**同一份实现**（`nextSectionId` 从 `ParamDefModal`
       * 导出，这里 import）—— 作者 2026-11-02：「复用我说的是新增的方法复用」。
       * 两个入口建的是同一种东西（`custom_N` + `ensure_section` 幂等），规则只能有一处。
       */
      const id = nextSectionId(registry?.sections ?? [])
      const next = await wb.setSectionMeta(id, { label, move: null, newSection: { id, tabId, label } })
      setRegistry(next)
      setMetaTick((t) => t + 1)
      onEditApplied()
      setDrawerSection(id)
      setDrawerAnchor(id)
      /*
       * 新组里一条参数都没有 —— 它那张卡只有**限定了页签**时才画得出来（见
       * `derive::desk` 注入空组那一段）。所以这里把领域切过去、清掉搜索词，
       * 否则"建完页面上没有它"，也就滚不过去。
       */
      setTabSel(tabId)
      if (q.trim() !== '') setQ('')
      pendingLocateRef.current = `t-sec-${id}`
      toasts.push(`已新建分组「${label}」（保存才落盘）`, { label: '撤销', run: onUndo })
    } catch (e) {
      toasts.push(isAppError(e) ? e.message : String(e))
    }
  }

  /**
   * **新建一个页签**（页签栏末尾那颗「＋」）。两份 TOML 各加一段 `[[tabs]]`，
   * 后端顺手在它下面建一个空分组 —— 否则点进去是一屏空白，参数没处放。
   *
   * 建完直接把领域切过去、清掉搜索词：新页签里 0 条参数，只有限定领域时才画得出卡。
   */
  const addTab = async (label: string) => {
    try {
      const id = nextTabId(registry?.tabs ?? [])
      const next = await wb.addTab(id, label)
      setRegistry(next)
      setMetaTick((t) => t + 1)
      onEditApplied()
      setTabSel(id)
      if (q.trim() !== '') setQ('')
      setTabAdding(false)
      toasts.push(`已新建页签「${label}」（保存才落盘）`, { label: '撤销', run: onUndo })
    } catch (e) {
      toasts.push(isAppError(e) ? e.message : String(e))
    }
  }

  /** 页签上移 / 下移一位（「编辑页签」框里那两颗箭头） */
  const moveTab = async (tabId: string, dir: -1 | 1) => {
    try {
      const next = await wb.setTabMeta(tabId, {
        label: null,
        move: dir < 0 ? 'up' : 'down',
      })
      setRegistry(next)
      setMetaTick((t) => t + 1)
      onEditApplied()
      const name = next.tabs.find((t) => t.id === tabId)?.label ?? tabId
      toasts.push(`页签换位已应用（保存才落盘）· ${name}`, { label: '撤销', run: onUndo })
    } catch (e) {
      toasts.push(isAppError(e) ? e.message : String(e))
    }
  }

  /** 删掉一个空页签。带参数 / 带功能卡的由后端拒（它给出理由） */
  const removeTab = async (tabId: string) => {
    try {
      const next = await wb.removeTab(tabId)
      setRegistry(next)
      setMetaTick((t) => t + 1)
      onEditApplied()
      setMembersTab(null)
      if (tabSel === tabId) setTabSel(null)
      if (drawerSection !== null && !next.sections.some((x) => x.id === drawerSection)) {
        setDrawerSection(null)
        setDrawerAnchor(null)
      }
      toasts.push('页签已删除（保存才落盘）', { label: '撤销', run: onUndo })
    } catch (e) {
      toasts.push(isAppError(e) ? e.message : String(e))
    }
  }

  const saveSection = async (sectionId: string, edit: SectionMetaEdit) => {
    try {
      const next = await wb.setSectionMeta(sectionId, edit)
      setRegistry(next)
      setMetaTick((t) => t + 1)
      onEditApplied()
      pendingLocateRef.current = `t-sec-${sectionId}`
      const name = next.sections.find((x) => x.id === sectionId)?.label ?? sectionId
      const what = edit.label !== null ? '分组改名' : '分组换位'
      /*
       * 说清"只是应用了"—— 落盘仍归右上角那颗「保存」（作者 2026-11-01 的口径）。
       * 一句话说完：toast 是浮在小卡片上的，末尾那截会被挤到屏幕外。
       */
      toasts.push(`${what}已应用（保存才落盘）· ${name}`, { label: '撤销', run: onUndo })
    } catch (e) {
      toasts.push(isAppError(e) ? e.message : String(e))
    }
  }

  /**
   * 写值的统一入口：手势前拦弃用、按 `valueType` 归位，然后**排进 `editQueue`**。
   *
   * 这里**不再 `await onApply`** —— 那是"点一下就同步等一次 IPC"，连点步进器时
   * 会串成一队、落后于点击（作者 2026-10-19 的原话：「停了之后它才慢慢追上我点击的
   * 次数」）。改成：翻成 `Patch` 排进队列，乐观值**立刻显示**；真正落库由队列
   * 120ms 尾随合批、单飞发出去（连点 20 下 = 1 条 `apply_draft`、1 条撤销）。
   */
  const writeValue = (
    row: Row,
    param: ParamView,
    ownerMachineId: string,
    layerUid: string | null,
    next: string | null,
    /**
     * 旧页用它决定这一笔回哪一页的派生；新内核统一由 `refreshOfPage` 现算。
     * 留着这个位置只为**调用点签名不变**（三个模式各传一个，位置不能错）。
     */
    _refreshKind: 'desk' | 'matrix' = 'desk',
    beforeRaw?: string,
    cell?: Cell | null,
  ) => {
    void _refreshKind
    const before = beforeRaw ?? String(row.cells[0]?.raw ?? '')
    /*
     * 已弃用的不许写（C14 §五）。判据**按机型**：优先读这一格的后端派生态
     * （`cell.deprecated` / `cell.deadChoices`，后者含推定），没有格子时退到
     * 定义上的显式清单（全局 + 点名这台机型）。
     *
     * 闸本身在后端 patch 校验 —— 这里拦在前面的意义只是「一句人话的 toast」，
     * 不是第二道判定。
     */
    if (next !== null) {
      const deadHere =
        cell !== undefined && cell !== null
          ? cell.deprecated
          : param.deprecated || param.deprecatedMachines.includes(ownerMachineId)
      if (deadHere) {
        toasts.push(`${param.label} ${words.disabled.deprecatedWriteBlocked}`)
        return
      }
      const deadValue =
        cell !== undefined && cell !== null
          ? cell.deadChoices.includes(next)
          : param.choices.some(
              (c) =>
                String(c.value) === next &&
                (c.deprecated || c.deprecatedMachines.includes(ownerMachineId)),
            )
      if (deadValue) {
        const hit = param.choices.find((c) => String(c.value) === next)
        const name = hit !== undefined ? `「${hit.label}」` : ''
        toasts.push(`${param.label} 的${name}${words.disabled.deprecatedWriteBlocked}`)
        return
      }
    }

    /* 原文进原文出，但落库按注册表的值类型归位 —— 否则 true 和 'true' 是两回事 */
    const value: unknown = next === null ? null : valueOfRaw(next, param)

    const owner = layerUid ?? ownerMachineId
    const cellKey = cellKeyOf(ownerMachineId, layerUid, row.key)
    /*
     * ① 本地草稿立刻生效 —— 显示与徽章不等这一发。
     *
     * 脏不脏**由值决定**（`draftOverlay.dirtyOf`：显示值 ≠ 保存值），所以
     * "改回原值"这一刻徽章就灭了，不必等后端把值写回去。
     *
     * ★ `next === null` = **「跟随基底」**（不是"值变成空"）：这一笔之后的值是
     * **基底的当前值**，只有后端知道 —— 所以本地**一个数都不钉**（见
     * `draftAfterWrite`）。以前钉的是 `cell.rest`（保存值），结果徽章写着跟随、
     * 数字永远停在旧值上，改基底一动不动（2026-11-03 作者报的）。
     */
    const entry = draftAfterWrite(next)
    setDraft((prev) => {
      const copy = new Map(prev)
      if (entry === null) copy.delete(cellKey)
      else copy.set(cellKey, entry)
      return copy
    })

    /* ② 单飞队列：这一条落地之前，下一条不会发 */
    queue.enqueue(cellKey, `${layerLabelOf(ownerMachineId, layerUid)} · ${param.label}`, {
      kind: 'setValue',
      level: layerUid !== null ? 'version' : 'machine',
      owner,
      key: row.key,
      value,
    })

    /*
     * 一次点击 = 一次改动的提示（开关与枚举手滑改错是重灾区）；给一条能撤回的。
     * 枚举那一档同样按 `valueType === 'string'` 认（与 `CellEditor` 的控件分派
     * 同一道门）—— 数值参数身上的 choices 是「预设档」，打字改值不发这条。
     */
    if (
      next !== null &&
      before !== next &&
      (param.uiComponent === 'switch' ||
        (param.valueType === 'string' && param.choices.length > 0))
    ) {
      const label =
        param.choices.find((o) => String(o.value) === next)?.label ??
        (next === 'true' ? '开' : next === 'false' ? '关' : next)
      toasts.push(`${param.label} 改成 ${label}`, { label: '撤销', run: onUndo })
    }
  }

  /**
   * **「跟随默认基底」**（2026-11-02 定稿）：把这一层自己写的那一行删掉。
   *
   * # 为什么只有这一个方向
   *
   * 两档口径是「改过就不跟随」—— 只要是**写值**，这一层就永远算「不跟随」，
   * 哪怕写的就是基底那个数。所以要"回去"，只能显式删掉那一行，没有别的入口
   * （作者：「除非右键改成跟随默认基底，否则改动过的就不管了」）。
   *
   * 发 `setValue { value: null }`，不是写一个"等于基底的值"：写值只会让它继续不跟随。
   */
  const followBase = (key: string) => {
    if (mode !== 'single') return
    const col = shownDesk?.cols[shownDesk.cur]
    if (col === undefined) return
    const layerUid = col.versionUid ?? null
    const cellKey = cellKeyOf(col.machineId, layerUid, key)
    /* 本地草稿直接丢掉：那一层没有自己的值了，显示该回落到继承来的那个 */
    setDraft((prev) => {
      const copy = new Map(prev)
      copy.delete(cellKey)
      return copy
    })
    queue.enqueue(cellKey, `跟随基线 · ${paramOf(key)?.label ?? key}`, {
      kind: 'setValue',
      level: layerUid !== null ? 'version' : 'machine',
      owner: layerUid ?? col.machineId,
      key,
      value: null,
    })
  }

  /*
   * 左树版本行的形态吃宽度（C14 第五轮）：窄（默认 176）名字 / id 竖排，
   * 宽（≥ 200）回横排。ResizeObserver 写 DOM 属性、不进 React。
   * 依赖 railOpen：收起态不渲染 .pRailBody，展开后重新观察。
   */
  const railBodyRef = useRef<HTMLDivElement>(null)
  useEffect(() => {
    const el = railBodyRef.current
    if (el === null) return
    const HI = 200
    const LO = 186
    const apply = () => {
      const w = el.clientWidth
      const wide = el.dataset.form !== 'narrow'
      const next = wide ? (w < LO ? 'narrow' : 'wide') : w >= HI ? 'wide' : 'narrow'
      if (el.dataset.form !== next) el.dataset.form = next
    }
    apply()
    const ro = new ResizeObserver(apply)
    ro.observe(el)
    return () => ro.disconnect()
  }, [railOpen])

  /** 正文这一模式有没有内容可看（C14 第十七轮：没选就没有那一套） */
  const bodyReady =
    mode === 'single'
      ? machine !== null && target !== null && desk !== null
      : compareCols.length > 0

  /*
   * 正文摆不开两列就退一列（每张卡要放下 名称 + 控件 + 状态 ≈ 420px）。
   * 判据是正文的实际宽度。阈值带滞后：进两列要 ≥920，退一列要 <880。
   * 依赖 bodyReady：空态时 .pScroll 不渲染，不然观察器永远装不上。
   */
  useEffect(() => {
    const el = scrollRef.current
    if (!el) return
    const NARROW = 880
    const WIDE = 920
    const apply = () => {
      const w = el.clientWidth
      const two = el.dataset.cols !== '1'
      const next = two ? (w < NARROW ? '1' : '2') : w >= WIDE ? '2' : '1'
      if (el.dataset.cols !== next) el.dataset.cols = next
    }
    apply()
    const ro = new ResizeObserver(apply)
    ro.observe(el)
    return () => ro.disconnect()
  }, [bodyReady])

  /* 切 tab / 换模式 / 换目标回到顶部 */
  useEffect(() => {
    scrollRef.current?.scrollTo({ top: 0 })
  }, [tabSel, mode, machineId, target, bodyReady])

  /*
   * 换机型**不清右栏**：同一条参数在新机型上一样有各版本取值，面板跟着换过去即可；
   * 只有新这一屏里查不到它（`selRow === null`）时，才由 `asideShown` 自然收起。
   */
  const switchMachine = (next: string) => {
    const m = book.machines.find((x) => x.id === next)
    setMachineId(next)
    setTarget(m?.versions[0]?.uid ?? BASE)
    setCompareCols([])
    setFocusUid(null)
    setBatchTab(false)
    setTabSel(null)
  }

  /**
   * 换正在改的那一版（`null` = 取消选择）。那枚「正在编辑」提示跟着作废。
   *
   * 换到另一版时**留着右栏**（面板跟着换到那一版的取值）；取消选择则连它一起收掉 ——
   * 那一屏已经没有版本，右栏也没有格子可显示。
   */
  const switchTarget = (next: Target) => {
    setTarget(next)
    if (next === null) setSel(null)
    setFocusUid(null)
  }

  /**
   * 勾 / 取消一列。允许勾到空（C14 第十七轮）—— 空着是合法状态。
   * **函数式更新**：连续勾几列（或刷选手势）落到同一次渲染里时，
   * 读 state 的写法会把前几笔吞掉。
   */
  const toggleCol = (id: string) => {
    setCompareCols((prev) =>
      prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id],
    )
  }

  const totalCount = desk?.total ?? 0
  const cur = desk?.cur ?? 0
  const editableCount =
    desk?.groups.reduce(
      (n, g) =>
        n +
        g.items.reduce(
          (m, it) =>
            m +
            (it.row.cells[cur]?.editable ? 1 : 0) +
            it.children.filter((c) => c.cells[cur]?.editable).length,
          0,
        ),
      0,
    ) ?? 0

  /* 右栏按需出现：选了参数就有。没选参数时整栏收起，不摆一块空面板占位置 */
  const selParam = sel !== null ? (paramOf(sel) ?? null) : null

  /**
   * 「组内顺序」那一格的数据（2026-11-02）：这一条在**它所属分组**里的同级清单。
   *
   * 与编辑定义框里的 `siblings` 同一条口径：按 `[params.layout].order` 排，
   * 拿注册表那份（右栏读的是已落盘的定义，草稿只存在于定义框里）。
   * 取不到（注册表还没回来 / 这条不在里面）就返回 `null`，那一格不画 ——
   * 空着一张表比不给更让人困惑。
   */
  const siblingsOf = (key: string): { key: string; label: string }[] | null => {
    if (registry === null) return null
    const self = registry.params.find((p) => p.key === key)
    if (self === undefined) return null
    return registry.params
      .filter((p) => p.sectionId === self.sectionId)
      .map((p) => ({ key: p.key, label: p.label, order: p.order }))
      .sort((a, b) => a.order - b.order || a.key.localeCompare(b.key))
      .map(({ key: k, label }) => ({ key: k, label }))
  }

  /*
   * 「挪一位」那条独立命令（`moveOrderOf`）**已经退役**（2026-11-14）：
   * 右栏与定义框都换成同一个排序模态框，整组一次提交（`wb_set_section_order`）。
   * 逐条发 `wb_set_param_meta` 的写法留着就等于：连挪三次 = 三次写盘 + 三条撤销，
   * 而且那条载荷是整份定义（label/desc/六册译文全都要重抄一遍）。
   */
  /** 「编辑定义」正在改的那条（null = 关着） */
  const defParam = defEditKey !== null ? paramOf(defEditKey) : null
  /*
   * 「适用机型」选择器的候选（品牌 + 机型清单）。定义框开着才取一次 ——
   * 挂了会重取的钩子就多一次 IPC；这里一次够用（品牌/机型清单不跟草稿走）。
   */
  const [machineList, setMachineList] = useState<MachineList | null>(null)
  useEffect(() => {
    if (defEditKey === null) {
      setMachineList(null)
      return
    }
    let dead = false
    void wb
      .machines()
      .then((m) => {
        if (!dead) setMachineList(m)
      })
      .catch(() => undefined)
    return () => {
      dead = true
    }
  }, [defEditKey])
  const selRow =
    sel !== null
      ? mode === 'single'
        ? rowOf(shownDesk, sel)
        : rowOf(shownDrawerDesk, sel)
      : null

  /*
   * 行右键菜单的条目（2026-11-02 作者：「帮我加上右键菜单，可以直接右键编辑，
   * 就和这个一样的打开模态框」）。
   *
   * 三条自我约束：
   *
   *   · **不引入新动作**：每一项在界面上都已经有入口（右栏那颗「编辑定义」、
   *     行本身的写值），文案照抄那个按钮，不另起说法；
   *   · **禁用要给原因**：用 `disabled` 那一句把"为什么这行不能编辑定义"说出来
   *     （弃用 / 是 G-code / 注册表里查不到），不灰着不解释；
   *   · **图标交替**：通用件的硬约定（见 `components/menu/types.ts` 头注），
   *     这里首项挂、次项不挂，隔开就有节奏。
   */
  const rowEntries: ContextMenuEntry[] = (() => {
    const key = rowMenu.target
    if (key === null) return []
    const p = paramOf(key)
    /* 右键那一行的**当前格**：单列模式 = 正对着的那一列。对照模式一次改多列，
       「跟随基底」在那里没有单一目标 —— 所以那一档不给，并说清为什么 */
    const cell =
      mode === 'single' ? (rowOf(shownDesk, key)?.cells[shownDesk?.cur ?? 0] ?? null) : null
    const followWhy =
      mode !== 'single'
        ? '对照模式一次看多列，先切回单列再操作'
        : cell === null
          ? '这一行还没有格子（矩阵还在拉）'
          : !cell.own
            ? '这一层本来就没有自己的值 —— 它已经在跟随基线'
            : !cell.editable
              ? cell.deprecated
                ? '这一项在这台机型上已弃用'
                : '这一格现在改不了'
              : undefined
    return [
      {
        id: 'def',
        label: '编辑定义…',
        icon: 'params',
        /* 与右栏那颗「编辑定义」同一条判据：拿不到 `ParamView` 就没得改 */
        disabled: p === null ? '注册表里没有这一条（派生数据还没回来）' : undefined,
        onSelect: () => {
          setSel(key)
          setDefEditKey(key)
        },
      },
      {
        id: 'follow',
        /* 只有"这一层写过"时才可用：删掉那一行，让它重新跟着基底 */
        label: '跟随默认基线',
        disabled: followWhy,
        onSelect: () => {
          setSel(key)
          followBase(key)
        },
      },
      {
        id: 'focus',
        label: '选中并跳到这一行',
        onSelect: () => {
          setSel(key)
          locateAnchor(`t-param-${key}`, { row: true })
        },
      },
    ]
  })()
  /**
   * 「定位」：把正文滚到这一条参数那一行。
   *
   * 单版本 = 卡片里的那一行（`t-param-*`）；对照 = 矩阵里那一行（`t-mrow-*`）。
   * 右栏把正文遮住时，这是确认「我改的是哪一行」的唯一办法。
   */
  const locateParam = (key: string) => {
    locateAnchor(mode === 'single' ? `t-param-${key}` : `t-mrow-${key}`, { row: true })
  }
  const batchable = selParam !== null && selParam.uiComponent !== 'gcode' && !selParam.deprecated
  const asideShown =
    sel !== null &&
    selParam !== null &&
    bodyReady &&
    (mode === 'single'
      ? selRow !== null
      : /* 对照：详情要基准机型的行；批量只要参数能批量 */
        (batchTab && batchable) || (!batchTab && selRow !== null))

  /**
   * 批量目标：勾选里的**每一列**（机型基线也在其中），可跨机型。
   *
   * 基线必须能进目标 —— 作者 2026-11-09「我明显都把基线都选中了」：
   * 勾了它却只改版本，等于把人的选择悄悄丢掉。基线写进去之后，跟随它的版本
   * 自动同值（`BulkEffect.followsBase`），不必每个版本各写一份。
   */
  const batchTargets: BatchTarget[] = useMemo(() => {
    /* 抽屉要那一行在这一项上的格子：预填「设置新值」、判「有没有变化」，
       以及「恢复修改前的」该写回什么。行还没到（矩阵在拉）就是空态 */
    const row = sel !== null ? (matrixOv?.rows.find((r) => r.key === sel) ?? null) : null
    return compareCols.map((id) => {
      const { m, uid } = colSplit(id)
      const machine = book.machines.find((x) => x.id === m)
      const v = uid === '' ? null : machine?.versions.find((x) => x.uid === uid)
      /* 后端重排后的列记号：机型基线 = 机型 id，版本 = 版本 uid */
      const key = uid === '' ? m : uid
      const ci = matrixOv?.cols.findIndex((c) => c.key === key) ?? -1
      const cell = row !== null && ci >= 0 ? (row.cells[ci] ?? null) : null
      return {
        /* id = 后端 BatchEffect.col 的记号（机型 id 或版本 uid = patch 的 owner），
           不是左树的 `机器|版本` 记号 —— 两套记号混用会让确认写入对不上列 */
        id: key,
        name:
          uid === ''
            ? `${machine?.display ?? m} · 机型基线`
            : `${machine?.display ?? m} · ${v?.name ?? uid}`,
        machineId: m,
        uid: uid === '' ? null : uid,
        raw: cell?.raw ?? null,
        rest: cell?.rest ?? null,
        dirty: cell?.dirty ?? false,
        /* 「这一层跟不跟随」只认后端徽章那一档（`[params.follows]` 的唯一实现）。
           `Cell.own` 不能拿来判：同机型各版本写了同一个值时 `digest` 会把它上提成
           基线，`own` 变 false 而状态仍是"不跟随"。基线是根，恒 false */
        follows: uid === '' ? false : cell?.badge === 'follow',
      }
    })
  }, [compareCols, book.machines, matrixOv, sel])

  /**
   * 对照模式「参数详情」要列的层：**勾选的每一列一层**（可跨机型）。
   *
   * 以前只取基准机型的 `drawerDesk`，于是勾了 A1 mini / P1S / P2S / X1C 之后，
   * 右栏仍只显示基准机那几版 —— 作者 2026-11-08：「其他我选中的都没有显示出来」。
   * 数据取自矩阵那一行（它本来就有每一列的格子）。
   */
  const detailLayers = useMemo<DetailLayer[] | null>(() => {
    if (mode !== 'compare' || sel === null || matrixOv === null) return null
    const row = matrixOv.rows.find((r) => r.key === sel) ?? null
    if (row === null) return null
    const out: DetailLayer[] = []
    matrixOv.cols.forEach((c, i) => {
      const cell = row.cells[i]
      if (cell === undefined) return
      const base = c.level === 'machine'
      out.push({
        owner: base ? '' : (c.versionUid ?? ''),
        machineId: c.machineId,
        name: base ? `${c.machine} 基底` : `${c.machine} · ${c.label}`,
        title: base ? `${c.machine} 基底` : `${c.machine} / ${c.label}`,
        versionId: base ? null : (c.versionUid?.split('/')[1] ?? null),
        base,
        cell,
      })
    })
    return out
  }, [mode, sel, matrixOv])

  /* 模态框要的那几样，全部现推（值改一次它就跟着变，不存会过期的副本） */
  const gcodeParam = gcodeOpen !== null ? (paramOf(gcodeOpen.key) ?? null) : null
  const gcodeCell = useMemo(() => {
    if (gcodeOpen === null) return null
    if (mode === 'single') {
      const row = rowOf(shownDesk, gcodeOpen.key)
      return row?.cells[cellIndexOf(shownDesk, gcodeOpen.uid)] ?? null
    }
    const m = matrixOv
    if (m === null) return null
    const row = m.rows.find((r) => r.key === gcodeOpen.key)
    if (!row) return null
    const ci = m.cols.findIndex(
      (c) => c.machineId === gcodeOpen.mid && (c.versionUid ?? null) === gcodeOpen.uid,
    )
    return ci >= 0 ? (row.cells[ci] ?? null) : null
  }, [gcodeOpen, mode, shownDesk, matrixOv])

  /*
   * 左树刷选（C14 第七轮）：按住拖过去、经过哪行改哪行。对照模式里所有机型的
   * 所有版本都是可选项；A 全选、Ctrl+I 反选还在。enabled 里带 railOpen：
   * 收起态那棵树没渲染，hook 只在依赖变了时才装监听。
   */
  const railTreeRef = useRef<HTMLElement>(null)
  const railIds = useMemo(
    () =>
      book.machines.flatMap((m) => [
        colId(m.id, BASE),
        ...m.versions.map((v) => colId(m.id, v.uid)),
      ]),
    [book.machines],
  )
  const compareSet = useMemo(() => new Set(compareCols), [compareCols])
  const railAll = railIds.length > 0 && railIds.every((c) => compareSet.has(c))
  usePaintSelect({
    ref: railTreeRef,
    ids: railIds,
    selected: compareSet,
    onChange: (next) => setCompareCols([...next]),
    /*
     * 手势期间的实时反馈：直接翻这一行的 DOM（勾、蓝底、aria）——
     * 手一过就亮，React 一次都不渲染；松手那一下才提交换态。
     */
    preview: (el, on) => {
      el.classList.toggle(String(s.pVrowOn), on)
      el.setAttribute('aria-checked', String(on))
      el.firstElementChild?.setAttribute('data-on', String(on))
    },
    enabled: mode === 'compare' && railOpen,
  })

  /** 左树一行。单版本 = radio（选谁改谁）；对照 = checkbox（勾选比较列）。
      选中判断必须带机型 —— A2L 和 A1 都有叫 STANDARD 的版本（作者实测） */
  const railRow = (m: { id: string; display: string }, uid: string, name: string, id: string, note?: string) => {
    const isCurrent = m.id === machineId
    const cid = colId(m.id, uid)
    const on = mode === 'single' ? isCurrent && uid === target : compareSet.has(cid)
    const kind = uid === BASE ? 'base' : 'version'
    const title =
      uid === BASE ? `${m.display} 的基底 —— 改它这一台所有版本都跟着动` : note
    return (
      <button
        key={cid}
        type="button"
        className={`${s.pVrow} ${on ? s.pVrowOn : ''}`}
        data-kind={kind}
        /* 对照模式下每一行都能被刷选（usePaintSelect 认这个标记） */
        data-sel={mode === 'compare' ? cid : undefined}
        role={mode === 'single' ? 'radio' : 'checkbox'}
        aria-checked={on}
        onClick={() => {
          if (mode === 'compare') {
            toggleCol(cid)
            return
          }
          /*
           * 再点一次已选中的那一行 = **取消选择**（作者：「那些点击之后可以
           * 去掉选择状态，不是非要选择一个」）。没选是合法状态：正文回空态，
           * 不会兜一台假机型。机型 id 留着不撤 —— 从头再来时省一次点击。
           */
          if (isCurrent && uid === target) {
            switchTarget(null)
            return
          }
          /* 切机型后仍落到点的那一行 —— 不是机型的第一个版本（C14） */
          if (m.id !== machineId) switchMachine(m.id)
          switchTarget(uid)
        }}
        title={title}
      >
        {/*
          选中 = 蓝色打勾（作者：「还不如用打勾的」）。勾常驻渲染、由 CSS 按
          data-on 显隐（C14 第十一轮）。
        */}
        <span className={s.pVmark} data-on={on} data-radio={mode === 'single'} aria-hidden>
          <svg
            viewBox="0 0 12 12"
            fill="none"
            stroke="currentColor"
            strokeWidth={2}
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="m2.6 6.3 2.4 2.4L9.5 3.9" />
          </svg>
        </span>
        {/* 名字与 id 两行：一条 176px 的栏里横排两边都看不全，竖排各自完整 */}
        <span className={s.pVtext}>
          <span className={s.pVname}>{name}</span>
          {id && <span className={s.pVid}>{id}</span>}
        </span>
        {mode === 'compare' && isCurrent && uid === target && (
          <span className={s.pTagOff}>当前</span>
        )}
      </button>
    )
  }

  if (error) {
    return (
      <p className="wb-todo" data-tone="danger">
        {error}
      </p>
    )
  }

  /** 对照矩阵喂给视图的行（「仅显示差异」在这里滤） */
  const searching = q.trim() !== ''
  /*
   * 领域（tab）过滤也在这里滤 —— 行身上带着后端给的 `tabId`，与「仅显示差异」
   * 是同一处版面决定。搜索态不按领域收窄（后端的规矩就是搜索跨全部分类，
   * doc §8.1），跟后端那份过滤对齐。
   */
  const shownMatrix = matrixOv
    ? {
        ...matrixOv,
        rows: matrixOv.rows
          .filter((r) => searching || tabSel === null || r.tabId === tabSel)
          .filter((r) => !diffOnly || matrixOv.diffKeys.includes(r.key)),
      }
    : null
  /*
   * 页签条计数数的行：跟着「仅显示差异」走（与旧账一致），唯独**不吃当前
   * 领域那一刀** —— 领域过滤只管正文。不然点了哪个领域，其余页签就从过滤
   * 后的行里被数成 0，整条页签只剩「全部 + 当前」（作者：「必须返回到全部
   * 才能看到其他」）。
   */
  const tabCountRows = matrixOv
    ? diffOnly
      ? matrixOv.rows.filter((r) => matrixOv.diffKeys.includes(r.key))
      : matrixOv.rows
    : []

  return (
    <div className={s.pPage}>
      {/* 页头一行：左 = 当前上下文，右 = 模式。版本选择在左树 —— 不抢视觉中心 */}
      <div className={s.pHead}>
        <button
          type="button"
          className={`${s.pRailHandle} ${railPinned ? s.pRailHandleOn : ''}`}
          data-open={railOpen}
          aria-expanded={railOpen}
          data-tip={
            railPinned
              ? '收起机型与版本（收起后鼠标扫过就能临时打开）'
              : '钉住机型与版本（不点也行：鼠标扫过就浮出来）'
          }
          /* 收起必须把「悬停浮出」一起关掉 —— 不然收起会当场换成浮出（C14 第十七轮） */
          onClick={() => {
            if (railPinned) {
              setRailPinned(false)
              setRailFloat(false)
            } else {
              setRailPinned(true)
            }
          }}
          onMouseEnter={openFloat}
          onMouseLeave={closeFloatSoon}
        >
          <svg
            viewBox="0 0 24 24"
            width="13"
            height="13"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden
          >
            <path d="m9 18 6-6-6-6" />
          </svg>
        </button>
        <span className={s.pCtx}>
          <span className={s.pCtxName}>
            {/* 没选就是没选 —— 不摆一台假的当前机型（C14 第十七轮） */}
            {machine === null || target === null
              ? '还没选机型'
              : `${machine.display} / ${target === BASE ? '机型基线' : (machine.versions.find((v) => v.uid === target)?.name ?? target)}`}
          </span>
          <span className={s.cardNote}>
            {mode === 'single' && machine !== null && target !== null && desk !== null
              ? `${totalCount} 参数 · ${editableCount} 可编辑` +
                (target !== BASE
                  ? ` · 本版钉着 ${desk.cols[desk.cur]?.items ?? 0} 项`
                  : '')
              : '从左树选一个机型和版本 —— 参数都在它们身上'}
          </span>
        </span>
        {focusUid !== null && (
          <span className={s.pFocus}>
            正在编辑 <code className={s.mono}>{focusUid}</code>
            <button
              type="button"
              className={`${s.btn} ${s.btnSm}`}
              title="回到那个版本的工作台，接着做它的下一步"
              onClick={() => onGoto('machines', { machineId: machineId ?? '', uid: focusUid, key: null })}
            >
              回到版本页
            </button>
          </span>
        )}
        <span className={s.grow} />
        <span className={s.pMode}>
          <button
            type="button"
            className={`${s.pModeBtn} ${mode === 'single' ? s.pModeOn : ''}`}
            onClick={() => setMode('single')}
          >
            单版本
          </button>
          <button
            type="button"
            className={`${s.pModeBtn} ${mode === 'compare' ? s.pModeOn : ''}`}
            onClick={() => setMode('compare')}
          >
            版本对照
          </button>
        </span>
      </div>

      <div
        className={s.pBody}
        ref={bodyRef}
        /*
         * 两条竖线的宽度挂在这个变量上（state → 变量）。拖动中分隔条会直接
         * 覆写它们，React 只在值变了的时候才写 style —— 手写的中间值不会被打回去。
         */
        style={
          {
            '--p-rail-w': `${railW}px`,
            '--p-aside-w': `${asideW}px`,
          } as CSSProperties
        }
      >
        {/* 左 rail：机型 / 版本。默认收起，悬停浮出、点击钉住 */}
        <aside
          className={s.pRail}
          data-mode={railPinned ? 'pinned' : railFloat ? 'float' : 'collapsed'}
          onMouseEnter={openFloat}
          onMouseLeave={closeFloatSoon}
        >
          {railOpen && (
            <div className={s.pRailBody} ref={railBodyRef}>
              <div className={s.pRailHint}>
                {mode === 'single' ? (
                  '选一个机型和版本来改'
                ) : (
                  <>
                    {/*
                      只留一枚全选框（作者：「全选反选清空没必要，只留一个框框
                      在文案左边就好」）。三态：全选中 = 勾满；部分 = 横杠；
                      没勾 = 空。清空 = 再点一下（全空，不再兜一枚基准列）；
                      反选还在键盘上（Ctrl+I）。
                    */}
                    <label className={s.pAll}>
                      {/* 输入藏起来（仍可聚焦、可点），画的是旁边那枚自绘框 */}
                      <input
                        ref={(el) => {
                          if (el) el.indeterminate = !railAll && compareCols.length > 0
                        }}
                        className={s.pAllInput}
                        type="checkbox"
                        checked={railAll}
                        onChange={() =>
                          setCompareCols(railAll ? [] : [...railIds])
                        }
                      />
                      {/*
                        自绘的三态框，**与左树每一行那枚勾选框同一副长相**
                        （作者：「这个的多选框怎么样式不一样」）—— 原生
                        `accent-color` 画出来的半选横杠和行上那枚勾对不上。
                      */}
                      <span
                        className={s.pVmark}
                        data-on={railAll}
                        data-mixed={!railAll && compareCols.length > 0}
                        aria-hidden
                      >
                        <svg
                          viewBox="0 0 12 12"
                          fill="none"
                          stroke="currentColor"
                          strokeWidth={2}
                          strokeLinecap="round"
                          strokeLinejoin="round"
                        >
                          <path d="m2.6 6.3 2.4 2.4L9.5 3.9" />
                        </svg>
                      </span>
                      勾选要对照的列（可跨机型）
                    </label>
                  </>
                )}
              </div>
              <nav className={s.pRailTree} aria-label="机型与版本" ref={railTreeRef}>
                {book.machines.map((m) => (
                  <div key={m.id} className={s.pMg}>
                    <div className={s.pMgName}>
                      {m.display}
                      <em>{m.versions.length}</em>
                    </div>
                    {/* 基底永远排这一组的第一行（C14 第十七轮：「基底放最上面」） */}
                    {railRow(m, BASE, '机型基线', '', '改一次动一片')}
                    {m.versions.map((v) =>
                      railRow(m, v.uid, v.name || v.uid, v.versionId, v.tag ?? undefined),
                    )}
                  </div>
                ))}
              </nav>
            </div>
          )}
        </aside>

        <div className={s.pMain}>
          {/*
            正文按「有没有内容」分两态（C14 第十七轮）：这一页的参数表是所选
            对象的派生 —— 没选就编不出那一套。摆一套假的进来，点也点不动，
            只会让人以为「参数就在这，只是暂时改不了」；留白 + 一句从哪里开始
            才是实话。
          */}
          {bodyReady ? (
            mode === 'single' && shownDesk !== null ? (
              <>
                {/* 工具行：搜索收短（180px）放最左 —— 搜索不是这个页面的主操作 */}
                <div className={s.pOps}>
                  <input
                    className={s.pSearch}
                    type="search"
                    value={q}
                    placeholder="搜索参数"
                    onChange={(e) => setQ(e.target.value)}
                  />
                  <span className={s.cardNote}>
                    {countRows(shownDesk)} 条
                    {q.trim() ? ` · 搜索「${q.trim()}」跨全部分类` : ''}
                  </span>
                  <span className={s.grow} />
                  {/*
                    ★ **这里不再画「放弃 / 保存」**（2026-10-08 作者实测）：
                    外壳页头右上角同一位置已经有一对（`未保存 · 放弃 · 保存`），
                    这一页再画一对就是**同一屏两组一模一样的按钮**，作者原话
                    「这个工作台的保存有两个多余的。你把下面这个去掉，只留上面那个」。
                    留着页头那一对：它跟着草稿走（`book.dirtyCount`），而草稿是**全稿**的 ——
                    保存本来就跨页（改了参数去别的页面点保存也该算数），
                    把按钮钉在某一页上反而在暗示"只保存这一页"。
                  */}
                </div>

                {/* 顶上 tab = 一级参数领域。分组不再是控件 —— 它在下面做卡头 */}
                <div className={s.pTabRow} role="tablist" aria-label="参数领域">
                  <button
                    type="button"
                    role="tab"
                    aria-selected={tabSel === null}
                    className={`${s.pTab} ${tabSel === null ? s.pTabOn : ''}`}
                    onClick={() => setTabSel(null)}
                  >
                    全部 <em>{shownDesk.total}</em>
                  </button>
                  {shownDesk.nav.map((t) => (
                    <button
                      key={t.id}
                      type="button"
                      role="tab"
                      aria-selected={tabSel === t.id}
                      className={`${s.pTab} ${tabSel === t.id ? s.pTabOn : ''}`}
                      onClick={() => setTabSel(t.id)}
                    >
                      {t.label} <em>{t.count}</em>
                    </button>
                  ))}
                  {tabEditButton}
                  {tabAddButton}
                </div>

                <div className={s.pScroll} ref={scrollRef}>
                  {shownDesk.groups.length > 0 ? (
                    /* 分组卡最多两列：这是参数工作区，不是仪表盘（作者对五列的判语） */
                    <div className={s.pGrid2}>
                      {shownDesk.groups.map((g) => {
                        /*
                         * 模式开关整卡收起（C15 A2）：整卡现在一行都改不动就收起行，
                         * 只留后端那句 + 「仍然展开看」。判据见文件头那一节 ——
                         * 一句话说：**判据全在后端给的字段里**（`offNote` / `blockedHint`）。
                         * 摊开之后卡头长一颗「收起来」（2026-10-03）—— 出口与入口是一对。
                         */
                        const note = collapseNoteOf(g, shownDesk.cur, q.trim() !== '')
                        if (note !== null && !openedCards.includes(g.sectionId)) {
                          return (
                            <CollapsedCard
                              key={g.sectionId}
                              group={g}
                              note={note}
                              words={words}
                              onOpen={() => setOpenedCards((prev) => [...prev, g.sectionId])}
                              onOpenDrawer={() => setDrawerSection(g.sectionId)}
                            />
                          )
                        }
                        return (
                          <GroupCard
                            key={g.sectionId}
                            component={
                              (registry?.sections ?? []).find((x) => x.id === g.sectionId)
                                ?.component ?? null
                            }
                            group={g}
                            cur={shownDesk.cur}
                            uid={target === BASE ? null : target}
                            ownerMachineId={machineId ?? ''}
                            words={words}
                            paramOf={paramOf}
                            sel={sel}
                            onPick={(key) => setSel(key)}
                            /* 右键某一行 = 打开它自己的菜单（编辑定义等），见文件末尾 `ParamLine` */
                            rowMenu={(key) => rowMenu.triggerProps(key)}
                            /* 卡头点开抽屉时顺手把「当前很亮的那一行」换成这一组 ——
                               抽屉里那颗圆点与页面上的亮行必须是同一件事（见抽屉那节注释） */
                            onOpenDrawer={() => {
                              setDrawerSection(g.sectionId)
                              setDrawerAnchor(g.sectionId)
                            }}
                            onWrite={writeValue}
                            onOpenGcode={(key) =>
                              setGcodeOpen({
                                key,
                                mid: machineId ?? '',
                                uid: target === BASE ? null : target,
                              })
                            }
                            onCollapse={
                              note !== null
                                ? () =>
                                    setOpenedCards((prev) => prev.filter((id) => id !== g.sectionId))
                                : undefined
                            }
                          />
                        )
                      })}
                    </div>
                  ) : (
                    <div className={s.pAsideEmpty}>
                      {q.trim() ? `没有匹配「${q.trim()}」的参数` : '这一组下没有参数'}
                    </div>
                  )}
                </div>
              </>
            ) : shownMatrix !== null ? (
              <>
                {/* 工具行（对照）：搜索 + 仅显示差异 + 计数 + 放弃/保存 */}
                <div className={s.pOps}>
                  <input
                    className={s.pSearch}
                    type="search"
                    value={q}
                    placeholder="搜索参数"
                    onChange={(e) => setQ(e.target.value)}
                  />
                  <button
                    type="button"
                    className={`${s.pChip} ${diffOnly ? s.pChipOn : ''}`}
                    onClick={() => setDiffOnly(!diffOnly)}
                    title="只留下这几列里值不一样的参数 —— 对照模式就是为了看这个"
                  >
                    仅显示差异 <em>{shownMatrix.diffKeys.length}</em>
                  </button>
                  <span className={s.cardNote}>
                    {shownMatrix.rows.length} 条
                    {q.trim() ? ` · 搜索「${q.trim()}」跨全部分类` : ''}
                  </span>
                  <span className={s.grow} />
                  {/* 同「单版本」那一支：这一页不再画「放弃 / 保存」—— 外壳页头已经有
                      一对，同一屏两组一模一样的按钮（见上面那段注） */}
                </div>

                <div className={s.pTabRow} role="tablist" aria-label="参数领域">
                  <button
                    type="button"
                    role="tab"
                    aria-selected={tabSel === null}
                    className={`${s.pTab} ${tabSel === null ? s.pTabOn : ''}`}
                    onClick={() => setTabSel(null)}
                  >
                    全部 <em>{shownMatrix.totalRows}</em>
                  </button>
                  {tabCountsOf(tabCountRows, registry).map((t) => (
                    <button
                      key={t.id}
                      type="button"
                      role="tab"
                      aria-selected={tabSel === t.id}
                      className={`${s.pTab} ${tabSel === t.id ? s.pTabOn : ''}`}
                      onClick={() => setTabSel(t.id)}
                    >
                      {t.label} <em>{t.count}</em>
                    </button>
                  ))}
                  {tabEditButton}
                  {tabAddButton}
                </div>

                <div className={s.pScroll} ref={scrollRef}>
                  <CompareMatrix
                    matrix={shownMatrix}
                    baseMachineId={baseMachineId}
                    words={words}
                    paramOf={paramOf}
                    sel={sel}
                    onPick={(key) => {
                      setSel(key)
                      setBatchTab(false)
                    }}
                    onPickBatch={(key) => {
                      setSel(key)
                      setBatchTab(true)
                    }}
                    /* 格子里那枚「打开编辑器」—— 与单版本行上那枚同一个模态框。
                       模态框自己按 `mid + uid` 去 matrix 里取那一格（见 `gcodeCell`） */
                    onOpenGcode={(col, row) =>
                      setGcodeOpen({
                        key: row.key,
                        mid: col.machineId,
                        uid: col.versionUid ?? null,
                      })
                    }
                    onWriteCell={(col, row, param, next) => {
                      const cell = cellAtCol(shownMatrix, col, row.key)
                      writeValue(
                        row,
                        param,
                        col.machineId,
                        col.level === 'version' ? (col.versionUid ?? null) : null,
                        next,
                        'matrix',
                        String(cell?.raw ?? ''),
                        cell,
                      )
                    }}
                  />
                </div>
              </>
            ) : null
          ) : (
            <div className={s.pBlank}>
              {mode === 'single' ? (
                <>
                  <p className={s.pBlankTitle}>先选一个机型和版本</p>
                  <p className={s.pBlankNote}>
                    左上角的把手就是「机型与版本」：鼠标扫过临时展开，点一下钉住。选好之后，
                    参数会列在这里，可以直接改值 —— 想改哪一版就选哪一版，基底也在里面。
                  </p>
                </>
              ) : (
                <>
                  <p className={s.pBlankTitle}>勾选要对照的版本列</p>
                  <p className={s.pBlankNote}>
                    在左树里勾一个或多个版本（可以跨机型），勾上之后这里按列排开 ——
                    不一样的格子会亮出来。左树收起时，鼠标扫过左上角的把手就出来。
                  </p>
                </>
              )}
            </div>
          )}
        </div>

        {/* 右栏按需出现：选了参数就有。对照模式 = 两页签「参数详情 / 批量修改」 */}
        {asideShown && selParam !== null && (
          <aside className={s.pAside}>
            {mode === 'compare' && (
              <div className={s.pAsideHead}>
                <span className={s.pAsideTabs}>
                  <button
                    type="button"
                    className={`${s.pAsideTab} ${batchTab ? '' : s.pAsideTabOn}`}
                    onClick={() => setBatchTab(false)}
                  >
                    参数详情
                  </button>
                  {batchable && (
                    <button
                      type="button"
                      className={`${s.pAsideTab} ${batchTab ? s.pAsideTabOn : ''}`}
                      onClick={() => setBatchTab(true)}
                    >
                      批量修改
                    </button>
                  )}
                </span>
                <button
                  type="button"
                  className={s.pCardX}
                  onClick={() => {
                    setSel(null)
                    setBatchTab(false)
                  }}
                  aria-label="收起"
                >
                  <svg width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden="true"><path d="M3 3l6 6M9 3l-6 6" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"/></svg>
                </button>
              </div>
            )}
            {sel !== null && (mode === 'single' || !batchTab || !batchable) && selRow !== null &&
              (mode === 'single' ? shownDesk : shownDrawerDesk) !== null && (
                <ParamDetail
                  uid={target === BASE ? null : target}
                  paramKey={sel}
                  cols={(mode === 'single' ? shownDesk : shownDrawerDesk)!.cols}
                  cur={(mode === 'single' ? shownDesk : shownDrawerDesk)!.cur}
                  row={selRow}
                  param={selParam}
                  machine={
                    (mode === 'single'
                      ? book.machines.find((m) => m.id === (machineId ?? ''))
                      : book.machines.find((m) => m.id === baseMachineId)) ??
                    book.machines[0]!
                  }
                  words={words}
                  /* 对照：勾选的每一列一层（跨机型）；单版本自己按 `machine.versions` 拼 */
                  layers={detailLayers}
                  groupLabel={groupLabelOf(registry, selRow)}
                  /*
                   * 「组内顺序」（2026-11-02）：这一条在它所属分组里的同级清单，
                   * 按 `order` 排好（含自己）。2026-11-14 起与编辑定义框**共用同一个
                   * 排序模态框**（拖 / 输位次 / 撤销重做 / 恢复）。
                   */
                  siblings={siblingsOf(sel)}
                  sectionId={selParam?.sectionId}
                  sectionLabel={groupLabelOf(registry, selRow)}
                  onOrderApplied={applySectionOrder}
                  onClose={() => setSel(null)}
                  onLocate={sel === null ? undefined : () => locateParam(sel)}
                  /* 「属于 X 的子参数」点 X 就换到 X —— 那一行是入口，不是注解 */
                  onPick={(key) => setSel(key)}
                  /* 「编辑定义」：名称/单位/类型/范围/条件/弃用那条改的路（走草稿（保存后写入）） */
                  onEditDef={selParam ? () => setDefEditKey(sel) : undefined}
                  /* 「各版本取值」每一层各有一枚按钮 —— 进来的那一层就是它 */
                  onOpenGcode={(key, layerUid, layerMachineId) => {
                    const mid =
                      layerMachineId ??
                      (mode === 'single'
                        ? (machineId ?? '')
                        : (drawerDesk?.cols.find((c) => (c.versionUid ?? null) === layerUid)?.machineId ??
                          baseMachineId))
                    setGcodeOpen({ key, mid, uid: layerUid })
                  }}
                  onWriteLayer={(layerUid, next, layerMachineId) => {
                    const ownerMid =
                      layerMachineId ??
                      (mode === 'single'
                        ? (machineId ?? '')
                        : (drawerDesk?.cols.find((c) => (c.versionUid ?? null) === layerUid)?.machineId ??
                          baseMachineId))
                    const row = mode === 'single' ? rowOf(shownDesk, sel) : rowOf(shownDrawerDesk, sel)
                    const idx = cellIndexOf(mode === 'single' ? shownDesk : shownDrawerDesk, layerUid)
                    /* 跨机型时那一格来自矩阵（`detailLayers`）—— 基准机型的 desk 里没有它 */
                    const cell =
                      (detailLayers !== null
                        ? (detailLayers.find(
                            (l) =>
                              l.machineId === ownerMid &&
                              (l.base ? null : l.owner) === layerUid,
                          )?.cell ?? null)
                        : (row?.cells[idx] ?? null)) ?? null
                    writeValue(
                      row ?? selRow,
                      selParam,
                      ownerMid,
                      layerUid,
                      next,
                      'desk',
                      String(cell?.raw ?? ''),
                      cell,
                    )
                  }}
                />
              )}
            {mode === 'compare' && batchable && batchTab && (
              <BatchEdit
                param={selParam}
                words={words}
                targets={batchTargets}
                onLocate={sel === null ? undefined : () => locateParam(sel)}
                /* 被条件关着的那一行点「去改」：选中那个阻塞参数、翻回详情、滚到它那一行 */
                onGoFix={(key) => {
                  setSel(key)
                  setBatchTab(false)
                  const anchor = `t-mrow-${key}`
                  /* 被搜索 / 领域过滤挡着时先放开过滤，等这一帧画完再滚 */
                  if (document.getElementById(anchor) === null) {
                    if (q.trim() !== '') setQ('')
                    setTabSel(null)
                    window.setTimeout(() => locateAnchor(anchor, { row: true }), 0)
                    return
                  }
                  locateAnchor(anchor, { row: true })
                }}
                onApply={async (label, patches) => {
                  /*
                   * 批量是**一整块手势**（一次 apply、一次撤销）—— 走 quiet：
                   * 不再让外壳拨 `tick` 去把整页 desk / matrix 重取一遍，
                   * 要的那份派生由 `refresh` 顺带带回来（回来的是不是当前这一页，
                   * 与单格写值同一条判据）。
                   */
                  const r: Refresh = { page: 'matrix', cols: compareColRefs, tab: null, query: q }
                  const out = await onApply(label, patches, r, { quiet: true })
                  if (out.ok && out.matrix !== null && refreshKeyOf(r) === refreshKeyRef.current) {
                    setMatrix(out.matrix)
                  }
                  /* 批量那几个动作靠它决定要不要弹「已写 …」的回执 */
                  return out.ok
                }}
              />
            )}
          </aside>
        )}

        {/* 拖线只在钉住态给：浮出态那条线的落点算不到浮层右缘 */}
        {railPinned && (
          <SplitterC14
            bodyRef={bodyRef}
            side="rail"
            varName="--p-rail-w"
            width={railW}
            otherW={asideShown ? asideW : 0}
            defaultValue={RAIL.dft}
            min={RAIL.min}
            max={RAIL.max}
            onCommit={setRailW}
            label="机型版本栏"
            className={s.pSplitRail}
          />
        )}
        {asideShown && (
          <SplitterC14
            bodyRef={bodyRef}
            side="aside"
            varName="--p-aside-w"
            width={asideW}
            otherW={railPinned ? railW : 0}
            defaultValue={ASIDE.dft}
            min={ASIDE.min}
            max={ASIDE.max}
            onCommit={setAsideW}
            label="详情栏"
            className={s.pSplitAside}
          />
        )}
      </div>

      {/*
        多行 G-code 的模态框（C14 第八轮）。**只有行上那枚按钮能打开它**；
        经 ModalC14 落到外壳的 .shellBody（P1 的遮罩宿主约定）。
      */}
      {gcodeOpen !== null && gcodeParam !== null && gcodeCell !== null && (
        <GcodeModal
          param={gcodeParam}
          cell={gcodeCell}
          layerLabel={layerLabelOf(gcodeOpen.mid, gcodeOpen.uid)}
          disabled={!gcodeCell.editable}
          dirty={dirty}
          write={(next) => {
            const row =
              mode === 'single'
                ? rowOf(shownDesk, gcodeOpen.key)
                : (matrixOv?.rows.find((r) => r.key === gcodeOpen.key) ?? null)
            if (row) {
              writeValue(
                row,
                gcodeParam,
                gcodeOpen.mid,
                gcodeOpen.uid,
                next,
                mode === 'compare' ? 'matrix' : 'desk',
                String(gcodeCell.raw ?? ''),
                gcodeCell,
              )
            }
          }}
          onSave={onSave}
          onClose={() => setGcodeOpen(null)}
        />
      )}

      {/*
        分组抽屉（2026-11-01）：点卡片头进来。改名 / 换位 / 这一组的参数清单都在
        里头 —— 卡头不再放输入框与箭头（作者：「上下这些……很容易不小心点到的
        就是排序」）。里面每一次改动都只是**应用**：进草稿、进撤销栈，落盘仍归
        右上角那颗「保存」，点了「放弃」就全不算。

        ★ 2026-11-02 直接挂在 `.pBody` 里（它本来就有 `position: relative`），
        **不再 portal 到 `FieldLayer`**：抽屉要能被"抽屉开着时弹出的对话框"压住，
        而对话框住 `.shellBody` —— 两者必须同一条链上（谁后画谁在上），
        不能隔着一个 z 90 的层，也不能是两个并排的容器。
        理由与四次失败的过程见 `SectionDrawer.tsx` 的文件头。
      */}
        <SectionDrawer
          sectionId={drawerSection}
          /* 抽屉里那颗「当前」要跟页面看齐（未落盘的新组、组件卡都算） */
          markId={drawerAnchor}
          registry={registry}
          rows={drawerRows}
          params={registry?.params ?? []}
          onEditMembers={applyGroupMembers}
          /*
           * 「这一组现在页面上到底有没有东西」—— 作者 2026-11-01 的疑问是
           * 「主要页面看不到了，但抽屉能看到，所以他没有真的删掉？客户端会不会消费到一个空的」。
           * 两个计数都从页面自己那份派生里数（表头 74 条 / 真画出来的 18 行），
           * 抽屉据此把那句话写出来，而不是让人自己猜。
           */
          pageTotal={pageTotal}
          pageDrawn={pageDrawn}
          onRemove={(id) => void removeSection(id)}
          onAddSection={(tabId, label) => void addSection(tabId, label)}
          onClose={() => setDrawerSection(null)}
          onApply={(id, edit) => void saveSection(id, edit)}
          /* 点一条既选中它、也滚到那一行（抽屉留着 —— 顺手接着改这一组），要关自己按 × */
          onPickParam={setSel}
          /*
           * 行的锚点 `t-param-<key>`：卡片是按「全部 / 某个领域」画出来的，点之前先确认
           * 它在树上；不在就先切到「全部」（搜索态一并清掉），等新的 desk 回来再滚
           * —— 与 `commitDef` 之后的定位同一条路（`pendingLocateRef`）。
           */
          jumpToParam={(key) => {
            if (document.getElementById(`t-param-${key}`) !== null) {
              locateAnchor(`t-param-${key}`, { row: true })
              return
            }
            pendingLocateRef.current = `t-param-${key}`
            if (q.trim() !== '') setQ('')
            setTabSel(null)
          }}
          /* 抽屉里换一组看：抽屉跟着换内容，页面滚到那张卡上闪一下（等于在左树里点它） */
          onShowSection={(id) => {
            /*
             * 「当前」不动 —— 这里只是**看**另一组（抽屉内容跟着换）。上面的 effect 会在
             * `drawerSection` 变的那一刻把它拨成新的，所以先把「当前」钉回原值。
             */
            setDrawerAnchor(drawerSection)
            setDrawerSection(id)
            pendingLocateRef.current = `t-sec-${id}`
            /*
             * ★ 目标那一组**不一定在当前领域里**（「全部」之外只列一个 tab 的卡）。
             * 只 `setMetaTick` 不换 tab 的话，`desk` 重取回来还是同一个领域、同一批卡，
             * 那张卡根本没被画出来 ⇒ 锚点找不到 ⇒ 什么都没发生（作者：「点了没反应」）。
             * 这里与 `commitDef` 同一条口径：先切到它挂的那个页签，滚的动作由
             * `pendingLocateRef` 那个 effect 在**新的** `desk` 到了之后做。
             *
             * ★★ 但在「全部」下**不切 tab**（2026-11-01 作者第二次点明）：
             *   「我在全部的 tab 下点击，就不要切换 tab 只是滚动」—— 全部本来就列着
             *   每一个领域的卡，切过去等于把人从"看得见全部"的视图里踢出去。
             *   只有当前收窄在某个领域、而目标不在那个领域时才切。
             */
            const tabId = registry?.sections.find((x) => x.id === id)?.tabId ?? null
            if (tabId !== null && tabSel !== null && tabSel !== tabId) setTabSel(tabId)
            setMetaTick((t) => t + 1)
          }}
          /* 注册表里查不到时用卡片自己的名字兜底（组件卡的卡头也点得开） */
          fallbackLabel={
            shownDesk?.groups.find((g) => g.sectionId === drawerSection)?.label ?? null
          }
        />

      {/*
        参数定义编辑（2026-10-03）：名称/单位/值类型/控件/范围/步进/出厂默认/
        可选项（含逐条弃用）/适用机型/属于/前置条件/弃用。确认保存才落盘 ——
        落盘成功把改前/改后整包交给外壳压撤销栈；回来的注册表直接顶掉旧的，
        三处派生数据靠 `metaTick` 重取。
      */}
      {membersTab !== null && registry !== null && (
        <SectionMembersModal
          scope="tab"
          id={membersTab}
          label={registry.tabs.find((t) => t.id === membersTab)?.label ?? membersTab}
          params={registry.params}
          sections={registry.sections}
          onApply={applyGroupMembers}
          onClose={() => setMembersTab(null)}
          /*
           * 页签自己的两件事（上移 / 下移 / 删除）放在这一框里：那一框就是
           * 「这一页长什么样」的编辑入口，页签名也在里面。判据与后端同一套 ——
           * 只有空页签给删除按钮（参数没有第二个去处）。
           */
          tabOps={(() => {
            const mine = registry.sections.filter((x) => x.tabId === membersTab)
            const used = registry.params.filter((p) => mine.some((x) => x.id === p.sectionId)).length
            const at = registry.tabs.findIndex((t) => t.id === membersTab)
            return {
              at,
              total: registry.tabs.length,
              canRemove: used === 0 && mine.every((x) => x.component === null && !x.standalone),
              onMove: (dir: -1 | 1) => void moveTab(membersTab, dir),
              onRemove: () => void removeTab(membersTab),
            }
          })()}
        />
      )}

      {tabAdding && (
        <NewTabModal
          onCancel={() => setTabAdding(false)}
          onCreate={(label) => void addTab(label)}
        />
      )}

      {defEditKey !== null && defParam !== null && (
        <ParamDefModal
          paramKey={defEditKey}
          param={defParam}
          params={registry?.params ?? []}
          machineList={machineList}
          sections={registry?.sections ?? []}
          onClose={() => setDefEditKey(null)}
          onCommit={(edit) => commitDef(defEditKey, defParam, edit)}
          onStaleCleared={applyStaleCleared}
          onOrderApplied={applySectionOrder}
        />
      )}

      {/*
        参数行的右键菜单（2026-11-02）。排在所有模态框之后 —— 它自己 portal 到
        `FieldLayer`，层级跟这里无关；放最后只为读起来跟其它页一致。
      */}
      <ContextMenu at={rowMenu.at} entries={rowEntries} onClose={rowMenu.close} />
    </div>
  )
}

/* ---------- 页面内部的小件 ---------- */

/**
 * 「模式开关整卡收起」的收起态（C15 A2）：卡头照旧（哪一张卡、几项），
 * 下面只留**后端那一句** + 「仍然展开看」。
 *
 * 为什么留一句而不是整张卡消失：Rust 那两句（`family_off` / `group_off`）的注释
 * 就是为这件事写的 ——「列表里直接把那几项收起来，只留这一句」，并且词表里备好了
 * 「仍然展开看」这个入口（`words.relate.showAnyway`）。整张卡无声消失的话，
 * 人要翻半天才知道「原来有这么一组、只是现在不生效」。
 */
/**
 * 卡片头（2026-11-01 第三稿）。
 *
 * 作者看前两稿的原话：「我不要点击的改名，我要点击的打开右侧的抽屉呀，我在抽屉
 * 里面去做改名呀，上下这些因为如果标题就能上下的话，很容易不小心点到的就是排序。
 * 我希望都在抽屉里面，而且抽屉里面可以展示更多的信息。」
 *
 * 所以卡头**只留一件事**：点标题 = 打开右侧抽屉（改名 / 换位 / 这一组的参数清单
 * 全在里头）。卡头里不再有输入框、也不再有箭头 —— 一个误触就把顺序改了，
 * 那是这条界面上最不该发生的事。
 *
 * 锚点 `t-sec-<id>` 挂在 `<header>` 上：改完滚过去闪一下就是靠它（见 `locateAnchor`）。
 */
function GroupHead({
  group,
  onOpen,
  trailing,
}: {
  group: DeskGroup
  /** 点标题 = 打开分组抽屉 */
  onOpen: () => void
  /** 卡头右端别的东西（摊开的卡有颗「收起来」） */
  trailing?: ReactNode
}) {
  return (
    <header className={s.pGroupHead} id={`t-sec-${group.sectionId}`}>
      <button
        type="button"
        className={s.pGroupName}
        title="打开分组抽屉：改名、换位置、看这一组里都有什么"
        onClick={onOpen}
      >
        {group.label}
      </button>
      <em>{group.count}</em>
      {trailing}
    </header>
  )
}

function CollapsedCard({
  group,
  note,
  words,
  onOpen,
  onOpenDrawer,
}: {
  group: DeskGroup
  /** 后端那一句（`offNote` 优先，退回第一行的 `blockedHint`） */
  note: string
  words: Words
  onOpen: () => void
  onOpenDrawer: () => void
}) {
  return (
    <section className={s.pGroup} data-off="true">
      <GroupHead group={group} onOpen={onOpenDrawer} />
      <div className={s.pOffRow}>
        <span className={s.pOffNote} data-tip={note}>
          {note}
        </span>
        <span className={s.grow} />
        <button
          type="button"
          className={`${s.btn} ${s.btnSm}`}
          title="这几行现在都被条件关着 —— 摊开看它们（摊开了也还是改不动）"
          onClick={onOpen}
        >
          {words.relate.showAnyway}
        </button>
      </div>
    </section>
  )
}

function GroupCard({
  group,
  cur,
  uid,
  ownerMachineId,
  words,
  paramOf,
  sel,
  onPick,
  onWrite,
  onOpenGcode,
  onCollapse,
  onOpenDrawer,
  component,
  rowMenu,
}: {
  group: DeskGroup
  cur: number
  /** 当前正在改的层（正文行写值的目标层） */
  uid: string | null
  ownerMachineId: string
  words: Words
  paramOf: (key: string) => ParamView | null
  sel: string | null
  onPick: (key: string) => void
  onWrite: (
    row: Row,
    param: ParamView,
    ownerMachineId: string,
    layerUid: string | null,
    next: string | null,
    refreshKind?: 'desk' | 'matrix',
    beforeRaw?: string,
    /** 这一格（`writeValue` 要拿它的 `rest` 判"改回原值"） */
    cell?: Cell | null,
  ) => void
  onOpenGcode: (key: string) => void
  /**
   * 「收起来」（2026-10-03）：这张卡被条件关着、又被「仍然展开看」摊开时才有 ——
   * 点了回到收起态。条件已经变回来了（`offNote` 没了）就不给：正常卡没有「收」可言
   */
  onCollapse?: () => void
  /** 点卡头 = 打开分组抽屉（改名 / 换位 / 参数清单都在里头） */
  onOpenDrawer: () => void
  /** 这一组的整块组件名（`tower-map` 这种）；`null` = 普通参数分组 */
  component: string | null
  /**
   * 行右键要摊上去的属性（2026-11-02）。
   *
   * 收成一个**函数**而不是"已算好的 props"：右键的 `target` 是点下去才知道的，
   * 每个行 key 各自一份；`useContextMenu` 的 `triggerProps(key)` 本来就是这形状。
   */
  rowMenu: (key: string) => {
    tabIndex: number
    onContextMenu: (e: ReactMouseEvent<HTMLElement>) => void
    onKeyDown: (e: ReactKeyboardEvent<HTMLElement>) => void
  }
}) {
  /* 这一组卡里有没有画布、在哪一侧（`tower-map` 就是左图右参数） */
  const slot = slotSideOf(component)
  return (
    <section className={s.pGroup}>
      <GroupHead
        group={group}
        onOpen={onOpenDrawer}
        trailing={
          onCollapse !== undefined ? (
            <button
              type="button"
              className={s.pFoldBtn}
              title="这一卡现在一行都改不动 —— 收回去，要看了再摊开"
              onClick={onCollapse}
            >
              {words.relate.foldBack}
            </button>
          ) : undefined
        }
      />
      {/*
        带画布的组件卡：**行与占位块并排**，占位块在哪一侧由 `slotSideOf` 说
        （`tower-map` 在左 —— 客户端就是左图右参数）。占位块只占位置、不画内容，
        见 `ComponentSlot` 头注。
      */}
      <div className={slot !== null ? s.pGroupSplit : undefined} data-side={slot ?? undefined}>
        {slot !== null && (
          <div className={s.pGroupAside}>
            <ComponentSlot component={component ?? ''} />
          </div>
        )}
        <div className={s.pGroupRows}>
          {/* 空组也占一张卡：刚建的那一组就在这儿，只是里面还没有参数 */}
          {group.items.length === 0 && <p className={s.pGroupEmpty}>还没有参数</p>}
          {/* 父项在前、子项紧跟 —— 与后端排好的顺序一致 */}
          {group.items
            .flatMap((it) => [it.row, ...it.children])
            .map((row) => (
              <ParamLine
                key={row.key}
                row={row}
                cur={cur}
                uid={uid}
                ownerMachineId={ownerMachineId}
                words={words}
                param={paramOf(row.key)}
                on={sel === row.key}
                onPick={onPick}
                onWrite={onWrite}
                onOpenGcode={onOpenGcode}
                menu={rowMenu(row.key)}
              />
            ))}
        </div>
      </div>
    </section>
  )
}

/**
 * 一行。不适用的行控件照摆、置灰：位置一跳，人就记不住刚才在哪（B02 的规矩）。
 * 已弃用是第三档：不铺灰底、不说条件，名字划一条红线 + 右边换一枚「已弃用」。
 */
function ParamLine({
  row,
  cur,
  uid,
  ownerMachineId,
  words,
  param,
  on,
  onPick,
  onWrite,
  onOpenGcode,
  menu,
}: {
  row: Row
  cur: number
  /** 当前正在改的层。`null` = 机型基底 */
  uid: string | null
  ownerMachineId: string
  words: Words
  param: ParamView | null
  on: boolean
  onPick: (key: string) => void
  onWrite: (
    row: Row,
    param: ParamView,
    ownerMachineId: string,
    layerUid: string | null,
    next: string | null,
    refreshKind?: 'desk' | 'matrix',
    beforeRaw?: string,
    /** 这一格（`writeValue` 要拿它的 `rest` 判"改回原值"） */
    cell?: Cell | null,
  ) => void
  onOpenGcode: (key: string) => void
  /** 右键（`useContextMenu` 的 `triggerProps`）—— 摊在整行上，鼠标与 Shift+F10 都认 */
  menu: {
    tabIndex: number
    onContextMenu: (e: ReactMouseEvent<HTMLElement>) => void
    onKeyDown: (e: ReactKeyboardEvent<HTMLElement>) => void
  }
}) {
  const cell = row.cells[cur]
  if (!cell || !param) return null
  /* 弃用按**这一列的机型**判（`Row.deprecated` 是全机型口径，答不了这一格） */
  const dep = cell.deprecated
  const off = !cell.editable && !dep
  /* 「改之前的值」= 盘上那一份里这一格的**有效值**（后端已按版本→机型→出厂
     三层算好）。`null` 才算"没有可回的值"。 */
  const before = cell.rest ?? null

  return (
    <div
      /* 「去处理」定位的锚点（locateAnchor 按它滚 + 闪） */
      id={`t-param-${row.key}`}
      {...menu}
      className={`${s.pRow} ${on ? s.pRowOn : ''} ${off ? s.pRowOff : ''} ${dep ? s.pRowDep : ''}`}
      onClick={() => onPick(row.key)}
      data-tip={
        cell.editable
          ? (param.desc || undefined)
          : dep
            ? (words.paramDeprecated.explain ?? undefined)
            : (cell.blockedNote ?? undefined)
      }
    >
      <span className={s.pRowName} data-depth={row.depth}>
        <span className={s.pRowLabel}>
          {/* 一级一条引线：子参数比父参数再进一层（C14） */}
          {Array.from({ length: row.depth }, (_, i) => (
            <i key={i} className={s.pIndent} aria-hidden />
          ))}
          {/*
            状态**在名字前面**（2026-11-03 作者：「不要占位置，放到左边字段那边，
            靠颜色就行」）—— 一小枚点：蓝实心=自己钉着、蓝空心=跟随、红=已弃用、
            灰空心=不适用；全称与解释在悬停里（见 `StatusDot`）。
          */}
          <StatusDot cell={cell} words={words} dep={dep} />
          <span className={s.pRowText}>{row.label}</span>
          {/* 单位不写在这里：数字控件的单位本来就在框里（C14 第十四轮） */}
        </span>
      </span>
      <span
        /* G-code 那一行的控件列允许被压（见 .pRowCtlG） */
        className={`${s.pRowCtl} ${param.uiComponent === 'gcode' ? s.pRowCtlG : ''}`}
        onClick={(e) => e.stopPropagation()}
      >
        {/*
          橙色「改之前的值」（2026-10-19）—— 抄客户端 `ParamRow` 那枚 `↩ 已保存值`。
          **这一格改了才出来**（`cell.dirty`，含还没落库的乐观那几格）；点它 = 把这层
          写回**盘上钉着的那个值**（`Cell.rest`，草稿不算）。

          与右栏那颗蓝色「跟随 x」（写 `null` ⇒ 跟随基底，值变成**基底的当前值**）
          **不是一回事**：那颗是"交给基底"，这颗是"退回我上次保存的数"。
        */}
        {cell.dirty && before !== null && cell.editable && !dep && (
          <button
            type="button"
            className={s.pRevert}
            data-tip={`回到保存值 ${String(before)} —— 上次保存时这一层是多少。点了它就是「不跟随」（自己钉住这个数）；想让值跟着基底走，用右栏那颗「跟随」`}
            onClick={() =>
              onWrite(
                row,
                param,
                ownerMachineId,
                uid,
                /* **总是写回那个值**（2026-11-03 作者裁定）：这颗按钮叫「回到保存值」，
                   点完这一层就是 `rest` 那个数、并且**不跟随**。
                   以前这里还看 `restPinned`（这一层在保存的那一份里钉着没），没钉就
                   发 null 挂回继承 —— 于是"点保存值"反而变成了"跟随基底"，正是作者
                   报的「按了保存值它却变成了跟随值」。

                   `rest` 为 `null` 的说法不在这一支（按钮由 `before !== null` 挡着），
                   但仍用 `rawOf`：`String(null)` 会得到 `"null"`，数字控件回落成 0
                   （作者看到过的「点 ↩ 70 变 0」）。 */
                rawOf(cell.rest),
                undefined,
                String(cell.raw ?? ''),
                cell,
              )
            }
          >
            保存值 {rawOf(before)}
          </button>
        )}
        <CellEditor
          param={param}
          cell={cell}
          form="row"
          disabled={!cell.editable}
          /* `cell` 必须一起递下去：`writeValue` 要拿它的 `rest` 判"改回原值" */
          onWrite={(next) => onWrite(row, param, ownerMachineId, uid, next, undefined, undefined, cell)}
          onOpenGcode={() => onOpenGcode(row.key)}
        />
      </span>
      {/*
        ★ 右列那枚状态徽章**没了**（2026-11-03 作者：「不要在右侧占位置」）：
        跟不跟随 / 已弃用 / 不适用都收进名字前那枚点里，值那一格因此宽出一截。
        只有"要 X 才可改"那句还留在右边 —— 它是操作指引，不是状态。
      */}
      {!cell.editable && !dep && cell.blockedHint !== null && (
        <span className={s.pRowBlock} data-tip={cell.blockedNote ?? undefined}>
          {cell.blockedHint}
        </span>
      )}
    </div>
  )
}

/* ---------- 纯函数小工具 ---------- */

/**
 * 控件原文 → 注册表类型上的值。**改值与「固定」共用这一处**。
 *
 * 两处各写一遍的话，同一次输入在"写值"与"固定"两条路上会落成不同的 JSON：
 * `'true'` 与 `true`、`'74'` 与 `74` 在盘上是两种东西，
 * 而比较用的是 `serde_json::Value` 的相等 —— 差一位就变成"永远改不回去"。
 */
function valueOfRaw(raw: string, param: ParamView | null): unknown {
  if (param === null) return raw
  if (param.valueType === 'bool') return raw === 'true'
  if ((param.valueType === 'float' || param.valueType === 'int') && raw !== '') return Number(raw)
  return raw
}

/**
 * 「这条参数现在的定义」→ 一份整包载荷。`commitDef` 在保存前拿它拍**改前**
 * 快照 —— 外壳撤销栈上那条 meta 条目的改前半边就是它（改后半边是这次真正
 * 交上去的载荷）。
 */
function metaOf(p: ParamView): ParamMetaEdit {
  return {
    label: p.label,
    desc: p.desc,
    unit: p.unit,
    valueType: p.valueType,
    uiComponent: p.uiComponent,
    defaultValue: p.defaultValue,
    min: p.min,
    max: p.max,
    step: p.step,
    parentKey: p.parentKey,
    showWhen: p.showWhen,
    deprecated: p.deprecated,
    deprecatedMachines: [...p.deprecatedMachines],
    choices: p.choices,
    machineFilter: [...p.machineFilter],
    /* 改前它就在这一组 —— 撤销一次「搬组」就是把它搬回去（同组 = 后端不动） */
    sectionId: p.sectionId,
    /* 改前的位置也要带上：撤销一次「上移」要能回到原位 */
    order: p.order,
  }
}

/**
 * **整卡收起该怎么说**（C15 A2）。返回 `null` = 不收起来（照旧铺行）。
 *
 * 判据全是从后端收到的那几个字段读出来的，前端不重判可见性：
 *
 *   1. 这一卡里**每一行**在 `cur` 这一列都被条件关着（`Cell.blocked` 非空）。
 *      用 `blocked` 而不是 `!editable`：已弃用的行也不可编辑，但它不是「在等一个开关」
 *      —— 对弃用的行说「换个条件就能用」是假话（`derive.rs` 那条注释）。
 *      取不到格子的（列对不上）一律**不收**：看不准的时候别藏东西。
 *   2. 收起时那句话：`group.offNote`（后端为「整组被 section 级条件关掉」写好的整句）
 *      优先；否则退回第一行的 `blockedHint`（也是后端拼的整句：`要 X 等于 Y 才可改`）。
 *      两句都没有就**不收** —— 收了却说不清为什么，比留着灰行更糟。
 *   3. 搜索态（`searching`）永远不收：那份列表是人自己搜出来的。
 */
function collapseNoteOf(group: DeskGroup, cur: number, searching: boolean): string | null {
  if (searching) return null
  const cells = group.items
    .flatMap((it) => [it.row, ...it.children])
    .map((row) => row.cells[cur])
  if (cells.length === 0) return null
  if (cells.some((c) => c === undefined)) return null
  if (!cells.every((c) => (c?.blocked.length ?? 0) > 0)) return null
  const note = group.offNote ?? cells.find((c) => c?.blockedHint != null)?.blockedHint ?? null
  return note
}

/** 一屏当前的行数（分组卡里的行 = 顶层 + 子项） */
function countRows(desk: Desk): number {
  return desk.groups.reduce((n, g) => n + g.count, 0)
}

/** 在 desk 里找一行（顶层与子项都找） */
function rowOf(desk: Desk | null, key: string): Row | null {
  if (desk === null) return null
  for (const g of desk.groups) {
    for (const it of g.items) {
      if (it.row.key === key) return it.row
      const child = it.children.find((c) => c.key === key)
      if (child) return child
    }
  }
  return null
}

/** 层的列下标。`null`（基底）找不到时退回后端指认的当前列 */
function cellIndexOf(desk: Desk | null, uid: string | null): number {
  if (desk === null) return 0
  const i = desk.cols.findIndex((c) => (c.versionUid ?? null) === uid)
  return i >= 0 ? i : desk.cur
}

/** 对照矩阵里某一列的格子 */
function cellAtCol(matrix: Matrix, col: Col, key: string) {
  const row = matrix.rows.find((r) => r.key === key)
  const ci = matrix.cols.findIndex((c) => c.key === col.key)
  return row && ci >= 0 ? (row.cells[ci] ?? null) : null
}

/**
 * 对照矩阵的页签（后端矩阵不带 nav，计数从行本身数）。行源由调用处给 ——
 * 必须是**没被当前领域滤过**的那份（见 `tabCountRows`），不然页签会塌。
 */
function tabCountsOf(
  rows: Row[],
  registry: RegistryView | null,
): { id: string; label: string; count: number }[] {
  if (registry === null) return []
  const count = new Map<string, number>()
  for (const r of rows) {
    if (r.tabId === null) continue
    count.set(r.tabId, (count.get(r.tabId) ?? 0) + 1)
  }
  /* 空页签也要在这儿露头，否则新建完切到对照模式就找不到它（与后端 `desk_nav` 同一条） */
  const used = new Set(registry.params.map((p) => p.sectionId))
  return registry.tabs
    .map((t) => ({
      id: t.id,
      label: t.label,
      count: count.get(t.id) ?? 0,
      fresh: t.sections.every((s) => !used.has(s.id)),
    }))
    .filter((t) => t.count > 0 || t.fresh)
    .map(({ id, label, count: n }) => ({ id, label, count: n }))
}

/** 抽屉头的小字：「领域 · 分组」。tab 名要查注册表 */
function groupLabelOf(registry: RegistryView | null, row: Row): string {
  const tab = registry?.tabs.find((t) => t.id === row.tabId)?.label
  return [tab, row.sectionLabel].filter(Boolean).join(' · ')
}


