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
 */
import { memo, useCallback, useEffect, useMemo, useRef, useState } from 'react'
import type { CSSProperties } from 'react'

import {
  isAppError,
  wb,
  type BookView,
  type Col,
  type ColRef,
  type Desk,
  type DeskGroup,
  type Matrix,
  type Patch,
  type Refresh,
  type RegistryView,
  type Row,
  type Words,
} from '../api'
import type { GotoFocus } from '../c14/types'
import { toasts } from '../c14/toast'
import GcodeModal from '../c14/GcodeModal'
import SplitterC14 from '../c14/SplitterC14'
import { useSplitWidth } from '../c14/useSplitWidth'
import { usePaintSelectC14 as usePaintSelect } from '../c14/usePaintSelect'
import type { ParamView } from '../api'
import BatchEdit from './BatchEdit'
import type { BatchTarget } from './BatchEdit'
import CellEditor from './CellEditor'
import CompareMatrix from './CompareMatrix'
import ParamDetail, { StatusTag } from './ParamDetail'
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
const RAIL = { dft: 176, min: 152, max: 320 }
const ASIDE = { dft: 320, min: 240, max: 620 }

interface Props {
  book: BookView
  words: Words
  /** 检查页 / 机型页「去处理」带过来的定位。挂载时读一次 */
  initialFocus?: GotoFocus | null
  /** 后端状态变过了就 +1（外壳每次写完会拨） */
  tick: number
  /** 草稿脏不脏（放弃 / 保存按钮） */
  dirty: boolean
  /** 唯一写入口（外壳的 run）。refresh 让后端顺带把那一页带回来 */
  onApply: (
    label: string,
    patches: Patch[],
    refresh?: Refresh,
  ) => Promise<{ desk: Desk | null; matrix: Matrix | null }>
  /** 外壳的保存 / 放弃 / 撤销 */
  onSave: () => void
  onDiscard: () => void
  onUndo: () => void
  /** 「回到版本页」—— 正在编辑某版时跳回机型与版本页 */
  onGoto: (view: string, focus?: GotoFocus) => void
}

export default memo(ParamsPage)

function ParamsPage({ book, words, initialFocus, tick, dirty, onApply, onSave, onDiscard, onUndo, onGoto }: Props) {
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

  const scrollRef = useRef<HTMLDivElement>(null)
  const bodyRef = useRef<HTMLDivElement>(null)
  const [railW, setRailW] = useSplitWidth('pRail', RAIL.dft, RAIL.min, RAIL.max)
  const [asideW, setAsideW] = useSplitWidth('pAside', ASIDE.dft, ASIDE.min, ASIDE.max)

  /*
   * 左 rail 的三种形态：collapsed 收起（默认）/ float 悬停浮出 / pinned 钉住。
   * 默认收起：两侧怎么变窄是这一稿的题目；但切版本仍然零点击 ——
   * 鼠标扫过把手，整棵树立刻浮出来。
   */
  const [railPinned, setRailPinned] = useState(false)
  const [railFloat, setRailFloat] = useState(false)
  const railOpen = railPinned || railFloat
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

  /* 注册表（参数的元数据）。开场取一次 */
  useEffect(() => {
    void wb
      .registry()
      .then(setRegistry)
      .catch((e: unknown) => setError(isAppError(e) ? e.message : String(e)))
  }, [])

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

  const compareColRefs: ColRef[] = useMemo(
    () =>
      compareCols.map((id) => {
        const { m, uid } = colSplit(id)
        return { machineId: m, versionUid: uid === '' ? null : uid }
      }),
    [compareCols],
  )

  /* 一屏配方台。tab / 搜索的过滤在后端（搜索跨分类，B02 的行为） */
  useEffect(() => {
    if (mode !== 'single' || machineId === null || target === null) {
      return
    }
    void wb
      .desk(machineId, target === BASE ? null : target, tabSel, q)
      .then(setDesk)
      .catch((e: unknown) => setError(isAppError(e) ? e.message : String(e)))
  }, [mode, machineId, target, tabSel, q, tick])

  /* 对照一屏：差异/行序/not_own 全部由后端按基准机型判（C14 第四轮） */
  useEffect(() => {
    if (mode !== 'compare' || compareColRefs.length === 0) {
      setMatrix(null)
      return
    }
    void wb
      .matrix(compareColRefs, tabSel, q, baseMachineId)
      .then(setMatrix)
      .catch((e: unknown) => setError(isAppError(e) ? e.message : String(e)))
  }, [mode, compareColRefs, tabSel, q, baseMachineId, tick])

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
  }, [mode, sel, baseMachineId, target, tick])

  /** 机器的中文名（对照模式的层标签用） */

  /** 层的中文名（撤销按钮的 label、G-code 模态框副标题共用） */
  const layerLabelOf = (mid: string, layerUid: string | null): string => {
    const m = book.machines.find((x) => x.id === mid)
    if (layerUid === null) return `${m?.display ?? mid} · ${words.level.machine.label}`
    const v = m?.versions.find((x) => x.uid === layerUid)
    return `${m?.display ?? mid} / ${v?.name ?? layerUid}`
  }

  /** 写值的统一入口：手势前拦弃用、按 valueType 归位，然后走外壳的 apply */
  const writeValue = async (
    row: Row,
    param: ParamView,
    ownerMachineId: string,
    layerUid: string | null,
    next: string | null,
    refreshKind: 'desk' | 'matrix' = 'desk',
    beforeRaw?: string,
  ) => {
    const before = beforeRaw ?? String(row.cells[0]?.raw ?? '')
    /*
     * 已弃用的不许写（C14 §五）。判据读的是后端字段：参数级 `param.deprecated`、
     * 选项级 `ChoiceView.deprecated`（推出来的并集）。闸本身在后端 patch 校验 ——
     * 这里拦在前面的意义只是「一句人话的 toast」，不是第二道判定。
     */
    if (next !== null) {
      if (param.deprecated) {
        toasts.push(`${param.label} ${words.disabled.deprecatedWriteBlocked}`)
        return
      }
      const dead = param.choices.find((c) => c.deprecated && String(c.value) === next)
      if (dead) {
        toasts.push(`${param.label} 的「${dead.label}」${words.disabled.deprecatedWriteBlocked}`)
        return
      }
    }

    /* 原文进原文出，但落库按注册表的值类型归位 —— 否则 true 和 'true' 是两回事 */
    let value: unknown = next
    if (next !== null) {
      if (param.valueType === 'bool') value = next === 'true'
      else if ((param.valueType === 'float' || param.valueType === 'int') && next !== '')
        value = Number(next)
    }

    const level = layerUid !== null ? 'version' : 'machine'
    const owner = layerUid ?? ownerMachineId
    const applyRefresh: Refresh =
      refreshKind === 'matrix'
        ? { page: 'matrix', cols: compareColRefs, tab: tabSel, query: q }
        : refresh
    const out = await onApply(`${layerLabelOf(ownerMachineId, layerUid)} · ${param.label}`, [
      { kind: 'setValue', level, owner, key: row.key, value },
    ], applyRefresh)
    if (refreshKind === 'matrix' && out.matrix) setMatrix(out.matrix)

    /* 一次点击 = 一次改动的提示（开关与枚举手滑改错是重灾区）；给一条能撤回的 */
    if (
      next !== null &&
      before !== next &&
      (param.uiComponent === 'switch' || param.choices.length > 0)
    ) {
      const label =
        param.choices.find((o) => String(o.value) === next)?.label ??
        (next === 'true' ? '开' : next === 'false' ? '关' : next)
      toasts.push(`${param.label} 改成 ${label}`, { label: '撤销', run: onUndo })
    }
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

  const switchMachine = (next: string) => {
    const m = book.machines.find((x) => x.id === next)
    setMachineId(next)
    setTarget(m?.versions[0]?.uid ?? BASE)
    setCompareCols([])
    setSel(null)
    setFocusUid(null)
    setBatchTab(false)
    setTabSel(null)
  }

  /** 换正在改的那一版。那枚「正在编辑」提示跟着作废 */
  const switchTarget = (next: string) => {
    setTarget(next)
    setSel(null)
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
  const selRow =
    sel !== null ? (mode === 'single' ? rowOf(desk, sel) : rowOf(drawerDesk, sel)) : null
  const batchable = selParam !== null && selParam.uiComponent !== 'gcode' && !selParam.deprecated
  const asideShown =
    sel !== null &&
    selParam !== null &&
    bodyReady &&
    (mode === 'single'
      ? selRow !== null
      : /* 对照：详情要基准机型的行；批量只要参数能批量 */
        (batchTab && batchable) || (!batchTab && selRow !== null))

  /** 批量目标：勾选列里的**版本列**（基底不进目标，C14 文件头第 1 条），可跨机型 */
  const batchTargets: BatchTarget[] = useMemo(
    () =>
      compareCols
        .filter((id) => colSplit(id).uid !== '')
        .map((id) => {
          const { m, uid } = colSplit(id)
          const machine = book.machines.find((x) => x.id === m)
          const v = machine?.versions.find((x) => x.uid === uid)
          return {
            /* id = 后端 BatchEffect.col 的记号（机型 id 或版本 uid = patch 的 owner），
               不是左树的 `机器|版本` 记号 —— 两套记号混用会让确认写入对不上列 */
            id: uid,
            name: `${machine?.display ?? m} · ${v?.name ?? uid}`,
            machineId: m,
            uid,
          }
        }),
    [compareCols, book.machines],
  )

  /* 模态框要的那几样，全部现推（值改一次它就跟着变，不存会过期的副本） */
  const gcodeParam = gcodeOpen !== null ? (paramOf(gcodeOpen.key) ?? null) : null
  const gcodeCell = useMemo(() => {
    if (gcodeOpen === null) return null
    if (mode === 'single') {
      const row = rowOf(desk, gcodeOpen.key)
      return row?.cells[cellIndexOf(desk, gcodeOpen.uid)] ?? null
    }
    const m = matrix
    if (m === null) return null
    const row = m.rows.find((r) => r.key === gcodeOpen.key)
    if (!row) return null
    const ci = m.cols.findIndex(
      (c) => c.machineId === gcodeOpen.mid && (c.versionUid ?? null) === gcodeOpen.uid,
    )
    return ci >= 0 ? (row.cells[ci] ?? null) : null
  }, [gcodeOpen, mode, desk, matrix])

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
    const mine = m.id === machineId
    const cid = colId(m.id, uid)
    const on = mode === 'single' ? mine && uid === target : compareSet.has(cid)
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
        {mode === 'compare' && mine && uid === target && (
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
  const shownMatrix = matrix
    ? diffOnly
      ? { ...matrix, rows: matrix.rows.filter((r) => matrix.diffKeys.includes(r.key)) }
      : matrix
    : null

  return (
    <div className={s.pPage}>
      {/* 页头一行：左 = 当前上下文，右 = 模式。版本选择在左树 —— 不抢视觉中心 */}
      <div className={s.pHead}>
        <button
          type="button"
          className={`${s.pRailHandle} ${railPinned ? s.pRailHandleOn : ''}`}
          data-open={railOpen}
          aria-expanded={railOpen}
          title={
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
              : `${machine.display} / ${target === BASE ? '机型基底' : (machine.versions.find((v) => v.uid === target)?.name ?? target)}`}
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
                      <input
                        ref={(el) => {
                          if (el) el.indeterminate = !railAll && compareCols.length > 0
                        }}
                        type="checkbox"
                        checked={railAll}
                        onChange={() =>
                          setCompareCols(railAll ? [] : [...railIds])
                        }
                      />
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
                    {railRow(m, BASE, '机型基底', '', '改一次动一片')}
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
            mode === 'single' && desk !== null ? (
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
                    {countRows(desk)} 条
                    {q.trim() ? ` · 搜索「${q.trim()}」跨全部分类` : ''}
                  </span>
                  <span className={s.grow} />
                  {/* 撤销 / 重做 / 未保存改动是全稿的动作，长在外壳状态栏上；
                      这里只留「放弃 / 保存」—— 只有这两件是「这一页在改的东西」 */}
                  <span className={s.pOpsBtns}>
                    <button type="button" className={`${s.btn} ${s.btnSm}`} disabled={!dirty} onClick={onDiscard}>
                      放弃
                    </button>
                    <button
                      type="button"
                      className={`${s.btn} ${s.btnPrimary} ${s.btnSm}`}
                      disabled={!dirty}
                      title={dirty ? undefined : words.disabled.nothingToSave}
                      onClick={onSave}
                    >
                      保存
                    </button>
                  </span>
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
                    全部 <em>{desk.total}</em>
                  </button>
                  {desk.nav.map((t) => (
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
                </div>

                <div className={s.pScroll} ref={scrollRef}>
                  {desk.groups.length > 0 ? (
                    /* 分组卡最多两列：这是参数工作区，不是仪表盘（作者对五列的判语） */
                    <div className={s.pGrid2}>
                      {desk.groups.map((g) => (
                        <GroupCard
                          key={g.sectionId}
                          group={g}
                          cur={desk.cur}
                          uid={target === BASE ? null : target}
                          ownerMachineId={machineId ?? ''}
                          words={words}
                          paramOf={paramOf}
                          sel={sel}
                          onPick={(key) => setSel(key)}
                          onWrite={writeValue}
                          onOpenGcode={(key) =>
                            setGcodeOpen({
                              key,
                              mid: machineId ?? '',
                              uid: target === BASE ? null : target,
                            })
                          }
                        />
                      ))}
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
                  <span className={s.pOpsBtns}>
                    <button type="button" className={`${s.btn} ${s.btnSm}`} disabled={!dirty} onClick={onDiscard}>
                      放弃
                    </button>
                    <button
                      type="button"
                      className={`${s.btn} ${s.btnPrimary} ${s.btnSm}`}
                      disabled={!dirty}
                      title={dirty ? undefined : words.disabled.nothingToSave}
                      onClick={onSave}
                    >
                      保存
                    </button>
                  </span>
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
                  {tabCountsOf(shownMatrix, registry).map((t) => (
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
                    onWriteCell={(col, row, param, next) => {
                      void writeValue(
                        row,
                        param,
                        col.machineId,
                        col.level === 'version' ? (col.versionUid ?? null) : null,
                        next,
                        'matrix',
                        String(cellAtCol(shownMatrix, col, row.key)?.raw ?? ''),
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
                  ×
                </button>
              </div>
            )}
            {sel !== null && (mode === 'single' || !batchTab || !batchable) && selRow !== null &&
              (mode === 'single' ? desk : drawerDesk) !== null && (
                <ParamDetail
                  uid={target === BASE ? null : target}
                  paramKey={sel}
                  cols={(mode === 'single' ? desk : drawerDesk)!.cols}
                  cur={(mode === 'single' ? desk : drawerDesk)!.cur}
                  row={selRow}
                  param={selParam}
                  machine={
                    (mode === 'single'
                      ? book.machines.find((m) => m.id === (machineId ?? ''))
                      : book.machines.find((m) => m.id === baseMachineId)) ??
                    book.machines[0]!
                  }
                  words={words}
                  groupLabel={groupLabelOf(registry, selRow)}
                  onClose={() => setSel(null)}
                  /* 「属于 X 的子参数」点 X 就换到 X —— 那一行是入口，不是注解 */
                  onPick={(key) => setSel(key)}
                  /* 「各版本取值」每一层各有一枚按钮 —— 进来的那一层就是它 */
                  onOpenGcode={(key, layerUid) => {
                    const mid =
                      mode === 'single'
                        ? (machineId ?? '')
                        : (drawerDesk?.cols.find((c) => (c.versionUid ?? null) === layerUid)?.machineId ??
                          baseMachineId)
                    setGcodeOpen({ key, mid, uid: layerUid })
                  }}
                  onWriteLayer={(layerUid, next) => {
                    const ownerMid =
                      mode === 'single'
                        ? (machineId ?? '')
                        : (drawerDesk?.cols.find((c) => (c.versionUid ?? null) === layerUid)?.machineId ??
                          baseMachineId)
                    const row = mode === 'single' ? rowOf(desk, sel) : rowOf(drawerDesk, sel)
                    const idx = cellIndexOf(mode === 'single' ? desk : drawerDesk, layerUid)
                    void writeValue(
                      row ?? selRow,
                      selParam,
                      ownerMid,
                      layerUid,
                      next,
                      'desk',
                      String(row?.cells[idx]?.raw ?? ''),
                    )
                  }}
                />
              )}
            {mode === 'compare' && batchable && batchTab && (
              <BatchEdit
                param={selParam}
                words={words}
                targets={batchTargets}
                onApply={async (label, patches) => {
                  await onApply(label, patches)
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
              mode === 'single' ? rowOf(desk, gcodeOpen.key) : (matrix?.rows.find((r) => r.key === gcodeOpen.key) ?? null)
            if (row) {
              void writeValue(
                row,
                gcodeParam,
                gcodeOpen.mid,
                gcodeOpen.uid,
                next,
                mode === 'compare' ? 'matrix' : 'desk',
                String(gcodeCell.raw ?? ''),
              )
            }
          }}
          onSave={onSave}
          onClose={() => setGcodeOpen(null)}
        />
      )}
    </div>
  )
}

/* ---------- 页面内部的小件 ---------- */

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
  ) => void
  onOpenGcode: (key: string) => void
}) {
  return (
    <section className={s.pGroup}>
      <header className={s.pGroupHead}>
        <span>{group.label}</span>
        <em>{group.count}</em>
      </header>
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
          />
        ))}
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
  ) => void
  onOpenGcode: (key: string) => void
}) {
  const cell = row.cells[cur]
  if (!cell || !param) return null
  const dep = row.deprecated
  const off = !cell.editable && !dep

  return (
    <div
      className={`${s.pRow} ${on ? s.pRowOn : ''} ${off ? s.pRowOff : ''} ${dep ? s.pRowDep : ''}`}
      onClick={() => onPick(row.key)}
      title={
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
          <span className={s.pRowText}>{row.label}</span>
          {/* 单位不写在这里：数字控件的单位本来就在框里（C14 第十四轮） */}
        </span>
      </span>
      <span
        /* G-code 那一行的控件列允许被压（见 .pRowCtlG） */
        className={`${s.pRowCtl} ${param.uiComponent === 'gcode' ? s.pRowCtlG : ''}`}
        onClick={(e) => e.stopPropagation()}
      >
        <CellEditor
          param={param}
          cell={cell}
          form="row"
          disabled={!cell.editable}
          onWrite={(next) => onWrite(row, param, ownerMachineId, uid, next)}
          onOpenGcode={() => onOpenGcode(row.key)}
        />
      </span>
      {cell.editable ? (
        <StatusTag cell={cell} words={words} />
      ) : dep ? (
        /* 「已弃用」借用状态徽章那副长相 —— 与「出厂默认 / 机型默认 / 本版修改 /
           已修改」占同一列，行尾的竖线不会因为它参差 */
        <span
          className={`${s.pStatus} ${s.pStatusDep}`}
          title={words.paramDeprecated.explain ?? undefined}
        >
          {words.paramDeprecated.label}
        </span>
      ) : (
        <span className={s.pRowBlock} title={cell.blockedNote ?? undefined}>
          {cell.blockedHint ?? words.placeholder.notApplicable}
        </span>
      )}
    </div>
  )
}

/* ---------- 纯函数小工具 ---------- */

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

/** 对照矩阵的页签（后端矩阵不带 nav，计数从行本身数） */
function tabCountsOf(
  matrix: Matrix,
  registry: RegistryView | null,
): { id: string; label: string; count: number }[] {
  if (registry === null) return []
  const count = new Map<string, number>()
  for (const r of matrix.rows) {
    if (r.tabId === null) continue
    count.set(r.tabId, (count.get(r.tabId) ?? 0) + 1)
  }
  return registry.tabs
    .map((t) => ({ id: t.id, label: t.label, count: count.get(t.id) ?? 0 }))
    .filter((t) => t.count > 0)
}

/** 抽屉头的小字：「领域 · 分组」。tab 名要查注册表 */
function groupLabelOf(registry: RegistryView | null, row: Row): string {
  const tab = registry?.tabs.find((t) => t.id === row.tabId)?.label
  return [tab, row.sectionLabel].filter(Boolean).join(' · ')
}
