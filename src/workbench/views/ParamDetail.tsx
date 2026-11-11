/*
 * 参数编辑区（右栏常驻 —— 页面的第二工作区）（C14 移植）。
 *
 * 交互模型（作者把这一稿说透了）：**不是「左边选版本 → 中间看参数 → 右边悬浮抽屉」，
 * 而是「版本 / 参数列表 / 参数编辑」三栏工作台。** 编辑一个参数时天然要面对
 * 它在多个版本上的取值 —— 所以「各版本取值」不是一张只读的展示表，
 * **每一行的值本身就是编辑控件**：点标准版那一行的 70，直接改标准版。
 * 不用关抽屉、回左树、切版本、再找回来。
 *
 * 数据全部由后端给：每层的格子来自 `Desk.cols` × `Row.cells`（一次 `wb_desk`
 * 就带全，不用为选一个参数再问一次）；「改了影响谁」来自 `Row.impact`；
 * 哪几档选项已弃用来自 `ChoiceView.deprecated`。前端不算业务。
 *
 * # C15 B1：类型提到「基本信息」里
 *
 * 值类型（`valueType`）/ 控件（`uiComponent`）/ 步进（`step`）原来住在最底下那一段
 * 「技术信息」—— 抽屉是「这东西到底是什么」的地方，而这三样正是答案的一部分，
 * 却压在整页最弱的档里（作者没找到它们）。C15 把它们提进「基本信息」，与
 * 「范围 / 可选项」「前置条件」并排；「技术信息」只留注册键（实现细节那一栏）。
 * 两样东西仍然全部来自后端注册表（`ParamView`），界面只换位置，不加判据。
 */

import { useEffect, useRef } from 'react'

import type { ReactNode } from 'react'

import type { Cell, Col, MachineNode, ParamView, Row, Words } from '../api'
/* 值 → TOML 原文的唯一转换点（`String(null)` 会得到 "null"，数字控件回落成 0） */
import { rawOf } from '../sheet/draftOverlay'
import CellEditor from './CellEditor'
import GroupPicker from './GroupPicker'
import type { GroupedGroup } from './GroupPicker'
import s from '../c14.module.css'

export type ParamStatusId = 'follow' | 'own'

/**
 * 两字缩写 —— 完整词（跟随基底 / 不跟随）在行内占得太宽，缩成两字，
 * 悬停仍是全称与解释。
 */
const ST_SHORT: Record<ParamStatusId, string> = {
  follow: '跟随',
  own: '不跟',
}

/**
 * 行上与编辑区共用同一枚状态标签 —— 两处说的是同一个事实，就不该有两份判据。
 * 词与解释都来自后端词表（`words.paramStatus`）。
 */
export function StatusTag({
  cell,
  words,
  short = false,
}: {
  cell: Cell | undefined
  words: Words
  short?: boolean
}) {
  /* 基底列不挂徽章（`badge` 为 null）—— 基底是根，谈不上跟不跟谁 */
  const id = cell?.badge ?? null
  if (id === null) return null
  const w = words.paramStatus[id]
  return (
    <span
      className={`${s.pStatus} ${ST_CLASS[id]}`}
      data-tip={short ? `${w.label} —— ${w.explain ?? ''}` : (w.explain ?? undefined)}
    >
      {short ? ST_SHORT[id] : w.label}
    </span>
  )
}

const ST_CLASS: Record<ParamStatusId, string> = {
  follow: s.stFollow,
  own: s.stOwn,
}

/**
 * **状态图标**（2026-11-03 作者）：列表行右列那枚 4 个字的状态徽章收进名字前这枚
 * 13px 的小图标 —— 一屏 70 多行，每行右边固定挂 60px，值那一格被压得很窄。
 *
 * 作者看过第一版（空心/实心圆点）说「我看不懂空心实心」⇒ 换成**形状不同**的图标，
 * 颜色只是第二层意思：
 *
 *   链条（灰）  跟随基底 —— 与上面连着，值从基底来
 *   图钉（蓝）  不跟随  —— 这一层自己钉着一个值（改基底不会带走它）
 *   禁止（红）  已弃用
 *   斜线（灰）  这台机型上不适用（前置条件不成立）
 *
 * 全称与解释在 `data-tip`（与徽章同一套 `words.paramStatus`；工作台 2026-11-03
 * 才把 `<TipLayer>` 挂上，此前所有 `data-tip` 都是死属性）。
 * **基底列不画**（`badge` 为 null）—— 基底是根，谈不上跟不跟谁。
 */
export function StatusDot({
  cell,
  words,
  dep,
}: {
  cell: Cell | undefined
  words: Words
  dep: boolean
}) {
  if (cell === undefined) return null
  const kind: 'own' | 'follow' | 'dep' | 'off' | null = dep
    ? 'dep'
    : !cell.editable
      ? 'off'
      : (cell.badge ?? null)
  if (kind === null) return null
  const tip =
    kind === 'dep'
      ? `${words.paramDeprecated.label} —— ${words.paramDeprecated.explain ?? ''}`
      : kind === 'off'
        ? (cell.blockedNote ?? words.placeholder.notApplicable)
        : `${words.paramStatus[kind].label} —— ${words.paramStatus[kind].explain ?? ''}`
  return (
    <span className={s.pDot} data-s={kind} data-tip={tip} role="img" aria-label={tip}>
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
        {MARK[kind]}
      </svg>
    </span>
  )
}

/** 四种状态各一个**形状不同**的图标（颜色见 `.pDot[data-s=…]`） */
const MARK: Record<'own' | 'follow' | 'dep' | 'off', ReactNode> = {
  /* 链条：与基底连着 */
  follow: (
    <>
      <path d="M9.6 14.4 14.4 9.6" />
      <path d="M8.6 11.9 6.7 13.8a3.4 3.4 0 0 0 4.8 4.8l1.9-1.9" />
      <path d="M15.4 12.1l1.9-1.9a3.4 3.4 0 0 0-4.8-4.8l-1.9 1.9" />
    </>
  ),
  /* 图钉：这一层自己钉着一个值 */
  own: (
    <>
      <path d="M12 14.5V21" />
      <path d="M8.9 3.5h6.2l-.8 4.1 2.5 2.5H7.2l2.5-2.5z" />
    </>
  ),
  /* 禁止：已弃用 */
  dep: (
    <>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M6 6l12 12" />
    </>
  ),
  /* 斜线：这台机型上不适用 */
  off: <path d="M5.5 18.5 18.5 5.5" />,
}

/**
 * 「各版本取值」里的一行。
 *
 * 单版本模式由本组件自己按 `machine.versions` 拼；**对照模式由调用方给**（勾选的
 * 每一列一层，可以跨机型）—— 那一屏里"版本"不止一台机器的，只看基准机型等于
 * 把另外几台勾掉的列藏起来。
 */
export interface DetailLayer {
  /** 写值用的 owner：版本 uid（`A1/STANDARD`）或机型 id */
  owner: string
  machineId: string
  /** 行内主文本：单版本 = `标准版`；对照 = `A1 · 标准版`（跨机型必须带机型） */
  name: string
  /** 完整身份，给 tooltip / aria 用：`A1 / 标准版`、`A1 基底` */
  title: string
  /** 版本 id（名字下面那行小字）。基底为 `null` */
  versionId: string | null
  /** 机型基底那一层 */
  base: boolean
  cell: Cell
}

interface Props {
  /** 左侧选中的工作上下文。`null` = 机型基底 */
  uid: string | null
  paramKey: string
  /** 这一机型的全部层与这一参数在各层的格子（来自 desk） */
  cols: Col[]
  cur: number
  row: Row
  param: ParamView
  machine: MachineNode
  words: Words
  /** 头部那一行小字：「领域 · 分组」。页面拼好给（tab 名要查注册表） */
  groupLabel: string
  /** 收起编辑区 —— 右栏按需出现，这个 × 就是「收起来」 */
  onClose: () => void
  /**
   * 滚到正文里这一条参数那一行（右栏遮着列表时，先看一眼再改）。
   * 不给就不画那颗按钮。
   */
  onLocate?: () => void
  /**
   * 对照模式：**勾选的每一列一层**（可跨机型）。给了就按它画「各版本取值」，
   * 不给就按 `machine.versions` 画（单版本模式）。
   */
  layers?: DetailLayer[] | null
  /** 对照模式**不画这枚 ×**（那一栏顶上有一排页签，× 长在页签行右端）（P3） */
  hideClose?: boolean
  /** 换一个参数来看 —— 抽屉里「属于 X 的子参数」点 X 就去那儿 */
  onPick?: (key: string) => void
  /**
   * 「编辑定义」（2026-10-03）：名称 / 单位 / 值类型 / 范围 / 前置条件 / 弃用
   * 那些只读格子如今有一条改的路 —— 弹出定义编辑模态框（走草稿（保存后写入））
   */
  onEditDef?: () => void
  /** 某一层那枚「打开编辑器」按钮 —— 每层各一枚，开的都是自己那层 */
  onOpenGcode?: (key: string, uid: string | null, machineId?: string) => void
  /**
   * 这一条在**它所属分组里**的同级清单（已按 `layout.order` 排好，含自己）。
   *
   * 给「组内顺序」那一格用（2026-11-02 作者：右栏也要一个，长得跟编辑定义框里
   * 那座下拉一样）。`null` / 空 = 算不出来（还没取到注册表），那一格就不画。
   */
  siblings?: { key: string; label: string }[] | null
  /**
   * 把这一条在组里上 / 下挪一位 —— 传 `-1` / `+1`。
   *
   * 与编辑定义框里的 `moveOrder` 是**同一条数据**（`[params.layout].order`），
   * 只是入口不同：那边先改前端草稿、按「确认」发一笔；这边每点一下箭头就发一笔。
   * **两边写到的都是影子树**（`workbench/.draft/presets/`），「放弃」都撤得掉 ——
   * 区别只在撤销栈的粒度（这边连挪三次 = 三笔）。
   */
  onMoveOrder?: (dir: -1 | 1) => void
  /**
   * 写某一层的值。`null` = 挂回继承。弃用闸在那条路上。
   *
   * `machineId` 只有对照模式跨机型时才有意义（那一层属于哪台机），单版本不用给。
   */
  onWriteLayer: (uid: string | null, next: string | null, machineId?: string) => void
}

export default function ParamDetail({
  uid,
  paramKey,
  cols,
  cur,
  row,
  param,
  machine,
  words,
  groupLabel,
  onClose,
  onLocate,
  hideClose = false,
  onPick,
  onEditDef,
  onOpenGcode,
  onWriteLayer,
  siblings = null,
  onMoveOrder,
  layers = null,
}: Props) {
  /*
   * 「各版本取值」的行形态跟着**这块面板的实际宽度**走（C14 第十二轮，
   * 作者把布局模型点名了：**不许自由换行，只许整行切成两种状态**）：
   *
   *   identity = 徽章 + 版本身份      editor = 控件 + ↶
   *
   *   宽（≥ 400）   identity | editor            —— 一行
   *   窄（默认 320） identity / editor（右对齐）  —— 两行，整行一起切
   *
   * 三条硬要求（作者原话）：
   *   1. **不许「半换行」** —— 两个**块**：要么并排，要么上下。
   *   2. **identity 不折行** —— 名字过长就省略号。
   *   3. **窄态输入框仍然右对齐** —— 保持参数编辑器的手感。
   *
   * 阈值 400/384 是量出来的。ResizeObserver 直接写 DOM 属性、不进 React ——
   * 拖右栏竖线时这几十行不参与重渲染。
   */
  const edRowsRef = useRef<HTMLDivElement>(null)
  useEffect(() => {
    const el = edRowsRef.current
    if (el === null) return
    const HI = 400
    const LO = 384
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
  }, [])

  /** 每层的格子。`cols` 与行的 `cells` 一一对应（后端保证） */
  const cellAt = (layerUid: string | null): Cell | undefined => {
    const i = cols.findIndex((c) => (c.versionUid ?? null) === layerUid)
    return i >= 0 ? row.cells[i] : undefined
  }

  const machineLabel = (layerUid: string | null): string =>
    layerUid === null
      ? `${machine.display} 基底`
      : `${machine.display} / ${machine.versions.find((v) => v.uid === layerUid)?.name ?? layerUid}`

  /**
   * 「各版本取值」要画的那几行。
   *
   * 对照模式读调用方给的 `layers`（勾选的每一列，可跨机型）；单版本按这一台的
   * 基底 + 各版本现拼。两种模式最后走同一个 `layerRow`。
   */
  const layerList: DetailLayer[] = (() => {
    if (layers !== null) return layers
    const out: DetailLayer[] = []
    const baseCell = cellAt(null)
    if (baseCell !== undefined) {
      out.push({
        owner: '',
        machineId: machine.id,
        name: `${machine.display} 基底`,
        title: `${machine.display} 基底`,
        versionId: null,
        base: true,
        cell: baseCell,
      })
    }
    for (const v of machine.versions) {
      const cell = cellAt(v.uid)
      if (cell !== undefined) {
        out.push({
          owner: v.uid,
          machineId: machine.id,
          /* 单版本模式下这一屏只有一台机，行内只写版本名（机型在页头已经说了） */
          name: v.name,
          title: `${machine.display} / ${v.name}`,
          versionId: v.versionId,
          base: false,
          cell,
        })
      }
    }
    return out
  })()

  /*
   * 已弃用（C14 §五）：抽屉是「这东西到底是什么」的地方，所以它必须在这里说清楚 ——
   * 作者：「我打开右侧的抽屉，里面它也应该显示弃用 什么什么呀」。
   * 判据只有一处，且**按当前这一层的机型**：`Cell.deprecated` / `Cell.deadChoices`
   * （含推定）。`Row.deprecated` 是全机型口径，只在拿不到格子时兜底。
   */
  const curCell =
    (uid === null
      ? layerList.find((l) => l.base)
      : layerList.find((l) => !l.base && l.owner === uid)
    )?.cell ?? cellAt(uid)
  const deprecated = curCell?.deprecated ?? row.deprecated
  const deadSet = new Set(curCell?.deadChoices ?? [])
  /* 对照模式这些行可能来自好几台机型 —— 任一台弃用这一档，就按弃用显示 */
  const involved = new Set(layerList.map((l) => l.machineId))
  const deadChoices = param.choices.filter(
    (c) =>
      deadSet.has(String(c.value)) ||
      c.deprecated ||
      c.deprecatedMachines.some((id) => involved.has(id)),
  )

  /**
   * 这一条参数是不是**枚举**：`choices` 只有在 string 档才是取值域 ——
   * 与 `CellEditor` 的控件分派、`validate.rs` 的枚举门同一道门。
   *
   * 「有 choices 就当枚举」是错的：`wiping.ironing_coverage_threshold` 是
   * float，挂着的 0/50/90 三条是「预设档」，这一条该说的是 `0~100 %`，
   * 控件也该是能填的百分比框，不是三选一。
   */
  const isEnum = param.valueType === 'string' && param.choices.length > 0

  /** 范围那一行的文本：枚举写可选项，数值写区间，都没有写「—」 */
  const rangeText = (() => {
    if (isEnum) return param.choices.map((c) => c.label).join(' / ')
    if (param.min !== null && param.max !== null) {
      /* 单位里带数字的（真数据里有 unit 就是 "0-255" 的）不再拼一次 */
      const unit = param.unit && !/\d/.test(param.unit) ? ` ${param.unit}` : ''
      return `${param.min}~${param.max}${unit}`
    }
    return '—'
  })()

  /**
   * 一层一行。`layerUid = null` 是机型基底 —— 它也直接可编辑：改一次动所有
   * 跟着它的版本，这层关系写进按钮的悬停提示里，不在界面上再摆一套说明。
   *
   * 行内顺序照作者画的最终样子：**徽章在最前**，名字跟上，控件、↶ 殿后。
   */
  const layerRow = (l: DetailLayer) => {
    const { cell } = l
    /* 「改之前的值」= 盘上那一份里这一格的有效值（后端按版本→机型→出厂算好）。
       `null` 才是"没有可回的值"（见 `Cell.rest` 的注）。 */
    const before = cell.rest ?? null
    const on = cell.editable
    const isCur = l.base ? uid === null : l.owner === uid
    /* 「挂回继承」之后落到哪一层：版本层 → **同一台机型**的基底那格；基底 → 出厂默认 */
    const below = l.base
      ? undefined
      : layerList.find((x) => x.base && x.machineId === l.machineId)?.cell
    /** 写这一层：基底写 `null` = 回出厂；版本写 `null` = 跟随基底 */
    const write = (next: string | null) =>
      onWriteLayer(l.base ? null : l.owner, next, l.machineId)
    /* 已经在跟随（徽章那档）：版本层才有意义，基底列 `badge` 恒 null */
    const following = cell.badge === 'follow'
    /* 「跟随」会跟到哪个数 —— **基底的当前值**（可能还没保存），不是保存值。
       **只取原文、不带单位**（作者 2026-11-03：「显示参数也不用显示单位」——
       单位在控件框里已经有一份） */
    const followTo = below != null ? rawOf(below.raw) : ''
    return (
      <div
        key={`${l.machineId}|${l.owner}`}
        className={`${s.pEdRow} ${isCur ? s.pEdRowOn : ''}`}
      >
        {/* identity：我是谁。徽章 + 版本身份，**一个整体**，不拆散、不折行 */}
        <span className={s.pEdIdentity}>
          <StatusTag cell={cell} words={words} short />
          <span
            className={s.pEdName}
            data-tip={`${l.title} —— ${on ? param.desc : (cell.blockedNote ?? '')}`}
          >
            {l.name}
            <em>{l.base ? '改一次动一片' : (l.versionId ?? undefined)}</em>
          </span>
        </span>
        {/* editor：怎么改。**一个整体**（两颗回去 + 控件）—— 窄态整块挪到第二行右对齐 */}
        <span className={s.pEdEditor}>
          {on ? (
            <>
              {/*
                **两颗「回去」的按钮，语义完全不同，名字必须写出来**（2026-11-03 作者裁定）。

                ① 橙色「保存值 3.85」= 写回 `Cell.rest`（**上次保存**那一刻的数）。
                   写值就等于"这一层自己钉住它" ⇒ 从此**不跟随**。只在脏的时候出现
                   （没有可回的值就没得点）。

                ② 蓝色「跟随 4.05」= 交回基底（写 null）。跟到的是**基底的当前值**，
                   不是保存值 —— 基底自己也可能有未保存的改动（作者 2026-11-03 问的
                   「为什么不是 3.85」就是这一条：3.85 是保存值，保存之后它就不存在了）。
                   基底那一行没有"跟谁"可言，那一位换成「出厂 4」= 删掉基底这一行、
                   回注册表的 `defaultValue`。

                ★ 排版（2026-11-03 作者：「不要在控件的后方，都放左边、固定」）：
                两颗**一律在控件左边**，`flex: none` 不参与压缩；文字只带数、不带单位
                （单位在控件框里已经有一份）。放不下就整块换行，不截字、不缩成图标。
                `↩` / `↶` 那类字形去掉了 —— 作者说"不要出现那么多奇怪的图标"。
              */}
              <span className={s.pEdActs}>
                {cell.dirty && before !== null && (
                  <button
                    type="button"
                    className={s.pRevert}
                    data-tip={`回到保存值 ${rawOf(before)} —— 上次保存时这一层是多少。点了它就是「不跟随」（这一层自己钉住这个数）`}
                    onClick={() =>
                      /* **总是写回那个值**：这颗叫「回到保存值」，点完就是那个数、且不跟随。
                         以前还看 `restPinned`，没钉着就发 null（= 挂回继承 = 跟随）——
                         于是"点保存值"变成了"跟随基底"（作者 2026-11-03 报的）。 */
                      write(rawOf(cell.rest))
                    }
                  >
                    保存值 {rawOf(before)}
                  </button>
                )}
                {!l.base ? (
                  <button
                    type="button"
                    className={s.pFollow}
                    disabled={following}
                    data-tip={
                      following
                        ? `已经在跟随 ${l.title} 的基底 —— 值就是基底那格（${followTo}），不用再点`
                        : `回到跟随的值 —— 跟到${words.level.machine.label}基底现在的 ${followTo}（基底自己还没保存的改动也算）`
                    }
                    aria-label={`让 ${l.title} 跟随基底`}
                    onClick={() => write(null)}
                  >
                    {following ? '已跟随' : `跟随 ${followTo}`}
                  </button>
                ) : cell.own ? (
                  <button
                    type="button"
                    className={s.pFollow}
                    data-kind="factory"
                    data-tip={`恢复出厂默认 ${rawOf(param.defaultValue)} —— 删掉基底这一行，值回到注册表里的 defaultValue`}
                    onClick={() => write(null)}
                  >
                    出厂 {rawOf(param.defaultValue)}
                  </button>
                ) : null}
              </span>
              {/* form="row"：与中间列表**同一个控件形态**（步进器、分段、开关长相一致） */}
              <span className={s.pEdCtl}>
                <CellEditor
                  param={param}
                  cell={cell}
                  form="row"
                  onWrite={(next) => write(next)}
                  onOpenGcode={
                    onOpenGcode
                      ? () => onOpenGcode(paramKey, l.base ? null : l.owner, l.machineId)
                      : undefined
                  }
                />
              </span>
            </>
          ) : deprecated ? (
            /* 弃用不说「要 X 才可改」——那也是假话（C14 §五） */
            <span
              className={`${s.pEdOff} ${s.pEdOffDep}`}
              data-tip={words.paramDeprecated.explain ?? undefined}
            >
              {words.paramDeprecated.label}
            </span>
          ) : (
            <span className={s.pEdOff} data-tip={cell.blockedNote ?? undefined}>
              {cell.blocked[0]?.need ?? words.placeholder.notApplicable}
            </span>
          )}
        </span>
      </div>
    )
  }

  const currentCell = curCell ?? row.cells[cur]
  /* 前置条件不成立时这里与行上写着同一句话（同一个来源：后端的 blockedHint） */
  const blocked = currentCell?.blocked[0] ?? null

  return (
    <div className={s.pDetail}>
      {/* 头：参数名 + 所属领域/分组 + 收起。右栏按需出现，× 就是「收起来」 */}
      <div className={s.pDHead}>
        <div className={s.pDHeadT}>
          {/* 名字上划一条线 —— 与列表行上那一条是同一个信号 */}
          <h3 data-dep={deprecated ? '' : undefined}>{param.label}</h3>
          <span className={s.cardNote}>
            {groupLabel}
            {deprecated && (
              <>
                {' · '}
                <span className={s.pDepTag} data-tip={words.paramDeprecated.explain ?? undefined}>
                  {words.paramDeprecated.label}
                </span>
              </>
            )}
          </span>
        </div>
        {onLocate !== undefined && (
          <button
            type="button"
            className={s.pMiniBtn}
            data-tip="滚到正文里这一条参数那一行"
            onClick={onLocate}
          >
            定位
          </button>
        )}
        {!hideClose && (
          <button type="button" className={s.pCardX} onClick={onClose} aria-label="收起">
            <svg width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden="true"><path d="M3 3l6 6M9 3l-6 6" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"/></svg>
          </button>
        )}
      </div>

      <div className={s.pDetailScroll}>
        {/* 各版本取值 = 编辑区本身。基底与每个版本各占一行，就地改 */}
        <section className={s.pDSection}>
          <div className={s.pDSecTitle}>各版本取值</div>
          <div className={s.pEdRows} ref={edRowsRef}>
            {layerList.map((l) => layerRow(l))}
          </div>
          <p className={s.cardNote}>
            {layers !== null
              ? '每一行都能直接改；这里列的是对照里勾选的每一列（可跨机型）'
              : <>每一行都能直接改；左侧选中的是 <b>{machineLabel(uid)}</b></>}
          </p>
        </section>

        {/* 基本信息：说明书该有的几行，没有键名。标题行右侧挂「编辑定义」——
            那些格子从这里从只读变成可改（走草稿（保存后写入），判据见 ParamDefModal） */}
        <section className={s.pDSection}>
          <div className={s.pDSecTitleRow}>
            <span className={s.pDSecTitle}>基本信息</span>
            {onEditDef && (
              <button
                type="button"
                className={s.pDefEdit}
                title="改名称、单位、值类型、范围、出厂默认、前置条件、弃用 —— 先进草稿，保存后写入"
                onClick={onEditDef}
              >
                编辑定义
              </button>
            )}
          </div>
          <div className={s.pKv}>
            {/*
              已弃用摆在「名称」**前面**：这一页最要紧的一条判断先说 ——
              读者看完这一行再往下读值，才不会以为那些值还能改。
            */}
            {deprecated && (
              <>
                <span className={s.pKvK}>状态</span>
                <span className={`${s.pKvV} ${s.pDepNote}`}>
                  <span className={s.pDepTag}>{words.paramDeprecated.label}</span>
                  {words.paramDeprecated.explain}
                </span>
              </>
            )}
            <span className={s.pKvK}>名称</span>
            <span className={s.pKvV}>{param.label}</span>
            <span className={s.pKvK}>单位</span>
            <span className={s.pKvV}>{param.unit ?? '—'}</span>
            {/*
              C15 B1：**类型 / 控件**从最底下那一段「技术信息」提上来，与「范围 /
              可选项」「前置条件」挨在一起 —— 「这东西是什么」该在一处说完。
              作者的原话是「比如 string，bool 之类的，都在抽屉里面显示的」；
              原来它们压在最弱的那一档（`技术信息`），作者没找到。
              两栏都是后端给的注册表原词（`ParamView.valueType` / `uiComponent`），
              前端不翻译、不按控件反推类型 —— 那正是这两栏存在的理由（见 contract.ts）。
            */}
            <span className={s.pKvK}>值类型</span>
            <span className={s.pKvV}>
              <span className={s.mono}>{param.valueType}</span>
            </span>
            <span className={s.pKvK}>控件</span>
            <span className={s.pKvV}>
              <span className={s.mono}>{param.uiComponent}</span>
              {param.step !== null && param.step !== undefined && ` · 步进 ${param.step}`}
            </span>
            <span className={s.pKvK}>{isEnum ? '可选项' : '范围'}</span>
            <span className={s.pKvV}>
              {/* 可选项那一行里，已弃用的那一档当场划掉 —— 与控件上那一条是同一个信号 */}
              {isEnum
                ? param.choices.flatMap((c, i) => [
                    i > 0 ? ' / ' : null,
                    <span
                      key={String(c.value)}
                      className={s.pDepStrike}
                      data-dep={
                        deadSet.has(String(c.value)) || c.deprecated ? '' : undefined
                      }
                    >
                      {c.label}
                    </span>,
                  ])
                : rangeText}
            </span>
            {deadChoices.length > 0 && (
              <>
                <span className={s.pKvK}>已弃用选项</span>
                <span className={`${s.pKvV} ${s.pDepNote}`}>
                  <span className={s.pDepTag}>{words.paramDeprecatedChoice.label}</span>
                  {deadChoices.map((c) => c.label).join('、')} ·{' '}
                  {words.paramDeprecatedChoice.explain}
                </span>
              </>
            )}
            <span className={s.pKvK}>出厂默认</span>
            <span className={s.pKvV}>{param.defaultText}</span>
            {row.parentKey && row.parentLabel && (
              <>
                <span className={s.pKvK}>属于</span>
                <span className={s.pKvV}>
                  {onPick ? (
                    <button
                      type="button"
                      className={s.pKvLink}
                      data-tip={`去看 ${row.parentLabel}`}
                      onClick={() => onPick(row.parentKey as string)}
                    >
                      {row.parentLabel}
                    </button>
                  ) : (
                    row.parentLabel
                  )}{' '}
                  的子参数
                </span>
              </>
            )}
            {blocked && (
              <>
                <span className={s.pKvK}>前置条件</span>
                <span className={s.pKvV}>{currentCell?.blockedHint ?? `要 ${blocked.need} 才可改`}</span>
              </>
            )}
          </div>
          <p className={s.pDesc}>{param.desc}</p>
        </section>

        {/* 作用域：它管多大范围、改了影响谁。判据在后端（`Row.impact`） */}
        <section className={s.pDSection}>
          <div className={s.pDSecTitle}>作用域</div>
          <div className={s.pKv}>
            <span className={s.pKvK}>适用机型</span>
            <span className={s.pKvV}>
              {param.machineFilter.length > 0 ? param.machineFilter.join('、') : '全部机型'}
            </span>
            <span className={s.pKvK}>改这里影响</span>
            <span className={s.pKvV}>
              {row.impact && row.impact.targets.length > 0
                ? row.impact.targets.join('、')
                : '—'}
            </span>
            {row.impact && row.impact.followers.length > 0 && (
              <>
                <span className={s.pKvK}>跟着基线走</span>
                <span className={s.pKvV}>{row.impact.followers.join('、')}</span>
              </>
            )}
          </div>
        </section>

        {/* 组内顺序（2026-11-02 作者：「这右侧也加一个组内排序的，就跟这些长得很像」）。
             与编辑定义框里那座下拉**同一个长相、同一条数据**（`[params.layout].order`）：
             展开后整组按当下顺序列出来，每行右边常驻上下箭头，就地挪。
             右栏不经过前端草稿：每点一下箭头发一条 `wb_set_param_meta`。
             但它进的是**影子树 + 事务**，右上角「保存」才写 `presets/`，「放弃」撤得掉。 */}
        {siblings !== null && siblings.length > 1 && onMoveOrder !== undefined && (() => {
          const at = siblings.findIndex((x) => x.key === param.key)
          const orderGroups: GroupedGroup[] = [
            {
              title: null,
              options: siblings.map((sib, i) => ({
                value: sib.key,
                label: sib.label,
                by: `第 ${i + 1} / ${siblings.length} 位`,
                ...(sib.key === param.key
                  ? { move: { up: at > 0, down: at >= 0 && at < siblings.length - 1, onMove: onMoveOrder } }
                  : { move: { up: false, down: false, onMove: () => undefined }, note: '只能挪这一条' }),
              })),
            },
          ]
          return (
            <section className={s.pDSection}>
              <div className={s.pDSecTitle}>组内顺序</div>
              <GroupPicker
                label="组内顺序"
                groups={orderGroups}
                value={param.key}
                onChange={() => undefined}
                width={300}
              />
              <p className={s.pDesc}>同一分组里的先后 — 客户端那一列也照这个排。</p>
            </section>
          )
        })()}

        {/* 技术信息：最底部、最弱的一档。**默认展开**（作者：就这些字，折叠反而
            多一下点击）—— 弱化靠位置与颜色，不靠藏起来。
            C15 B1 之后这里只剩「实现细节」那一栏：值类型 / 控件 / 步进已经提到
            「基本信息」去和范围、条件作伴了 —— 那几样是「这是什么」，
            注册键才是「它在程序里叫什么」。 */}
        <section className={`${s.pDSection} ${s.pDTech}`}>
          <div className={s.pDSecTitle}>技术信息</div>
          <div className={s.pKv}>
            <span className={s.pKvK}>注册键</span>
            <span className={`${s.pKvV} ${s.mono}`}>{param.key}</span>
          </div>
        </section>
      </div>
    </div>
  )
}
