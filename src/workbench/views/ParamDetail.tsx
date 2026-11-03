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

import type { Cell, Col, MachineNode, ParamView, Row, Words } from '../api'
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
  /** 对照模式**不画这枚 ×**（那一栏顶上有一排页签，× 长在页签行右端）（P3） */
  hideClose?: boolean
  /** 换一个参数来看 —— 抽屉里「属于 X 的子参数」点 X 就去那儿 */
  onPick?: (key: string) => void
  /**
   * 「编辑定义」（2026-10-03）：名称 / 单位 / 值类型 / 范围 / 前置条件 / 弃用
   * 那些只读格子如今有一条改的路 —— 弹出定义编辑模态框（即时落盘）
   */
  onEditDef?: () => void
  /** 某一层那枚「打开编辑器」按钮 —— 每层各一枚，开的都是自己那层 */
  onOpenGcode?: (key: string, uid: string | null) => void
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
  /** 写某一层的值。`null` = 挂回继承（删键）。弃用闸在那条路上 */
  onWriteLayer: (uid: string | null, next: string | null) => void
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
  hideClose = false,
  onPick,
  onEditDef,
  onOpenGcode,
  onWriteLayer,
  siblings = null,
  onMoveOrder,
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

  /*
   * 已弃用（C14 §五）：抽屉是「这东西到底是什么」的地方，所以它必须在这里说清楚 ——
   * 作者：「我打开右侧的抽屉，里面它也应该显示弃用 什么什么呀」。
   * 判据只有一处，且**按当前这一层的机型**：`Cell.deprecated` / `Cell.deadChoices`
   * （含推定）。`Row.deprecated` 是全机型口径，只在拿不到格子时兜底。
   */
  const curCell = cellAt(uid)
  const deprecated = curCell?.deprecated ?? row.deprecated
  const deadSet = new Set(curCell?.deadChoices ?? [])
  const deadChoices = param.choices.filter(
    (c) => deadSet.has(String(c.value)) || c.deprecated || c.deprecatedMachines.includes(machine.id),
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
  const layerRow = (layerUid: string | null) => {
    const cell = cellAt(layerUid)
    if (!cell) return null
    /* 「改之前的值」= 盘上那一份里这一格的有效值（后端按版本→机型→出厂算好）。
       `null` 才是"没有可回的值"（见 `Cell.rest` 的注）。 */
    const before = cell.rest ?? null
    const on = cell.editable
    const isCur = layerUid === uid
    /* 「挂回继承」之后落到哪一层：版本层 → 机型基底那格；基底 → 出厂默认 */
    const below = layerUid !== null ? cellAt(null) : null
    return (
      <div key={layerUid ?? 'base'} className={`${s.pEdRow} ${isCur ? s.pEdRowOn : ''}`}>
        {/* identity：我是谁。徽章 + 版本身份，**一个整体**，不拆散、不折行 */}
        <span className={s.pEdIdentity}>
          <StatusTag cell={cell} words={words} short />
          <span
            className={s.pEdName}
            data-tip={`${machineLabel(layerUid)} —— ${on ? param.desc : (cell.blockedNote ?? '')}`}
          >
            {layerUid === null
              ? `${machine.display} 基底`
              : (machine.versions.find((v) => v.uid === layerUid)?.name ?? layerUid)}
            <em>
              {layerUid === null
                ? '改一次动一片'
                : (machine.versions.find((v) => v.uid === layerUid)?.versionId ?? undefined)}
            </em>
          </span>
        </span>
        {/* editor：怎么改。**一个整体**（控件 + ↶）—— 窄态整块挪到第二行右对齐 */}
        <span className={s.pEdEditor}>
          {on ? (
            <span className={s.pEdCtl}>
              {/*
                橙色「改之前的值」（2026-10-19）—— 与中间列表那一枚同款。
                点它 = 把这层写回**盘上钉着的值**（`cell.rest`）；右边那枚 `↶`
                是"挂回继承"（写 null 删键）—— 两条语义不同，两枚并存。
              */}
              {cell.dirty && before !== null && (
                <button
                  type="button"
                  className={s.pRevert}
                  data-tip={`恢复修改前的值：${String(before)}（这一层保存前是多少，不是「恢复默认」）`}
                  onClick={() =>
                    /* 盘上这一层本来钉着 ⇒ 写回那个值；没钉 ⇒ 写 null 删键挂回继承 */
                    onWriteLayer(layerUid, cell.restPinned ? String(cell.rest) : null)
                  }
                >
                  ↩ {String(before)}
                </button>
              )}
              {/* form="row"：与中间列表**同一个控件形态**（步进器、分段、开关长相一致） */}
              <CellEditor
                param={param}
                cell={cell}
                form="row"
                onWrite={(next) => onWriteLayer(layerUid, next)}
                onOpenGcode={onOpenGcode ? () => onOpenGcode(paramKey, layerUid) : undefined}
              />
            </span>
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
          {cell.own && on ? (
            <button
              type="button"
              className={s.pEdReset}
              data-tip={
                below
                  ? `恢复默认 —— 跟随${words.level.machine.label}，值 ${below.text}`
                  : '恢复出厂默认'
              }
              aria-label={`恢复 ${machineLabel(layerUid)} 的默认值`}
              onClick={() => onWriteLayer(layerUid, null)}
            >
              ↶
            </button>
          ) : (
            <span className={s.pEdResetPh} aria-hidden />
          )}
        </span>
      </div>
    )
  }

  const currentCell = row.cells[cur]
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
        {!hideClose && (
          <button type="button" className={s.pCardX} onClick={onClose} aria-label="收起">
            ×
          </button>
        )}
      </div>

      <div className={s.pDetailScroll}>
        {/* 各版本取值 = 编辑区本身。基底与每个版本各占一行，就地改 */}
        <section className={s.pDSection}>
          <div className={s.pDSecTitle}>各版本取值</div>
          <div className={s.pEdRows} ref={edRowsRef}>
            {layerRow(null)}
            {machine.versions.map((v) => layerRow(v.uid))}
          </div>
          <p className={s.cardNote}>
            每一行都能直接改；左侧选中的是 <b>{machineLabel(uid)}</b>
          </p>
        </section>

        {/* 基本信息：说明书该有的几行，没有键名。标题行右侧挂「编辑定义」——
            那些格子从这里从只读变成可改（即时落盘，判据见 ParamDefModal） */}
        <section className={s.pDSection}>
          <div className={s.pDSecTitleRow}>
            <span className={s.pDSecTitle}>基本信息</span>
            {onEditDef && (
              <button
                type="button"
                className={s.pDefEdit}
                title="改名称、单位、值类型、范围、出厂默认、前置条件、弃用 —— 即时落盘，不进草稿"
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
                <span className={s.pKvK}>跟着基底走</span>
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
