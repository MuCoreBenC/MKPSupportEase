/**
 * 视角一：参数矩阵（doc §8）。这一稿是整个工作台的技术核心。
 *
 * # 列不是「当前对象」，是一份可以同时看好几个的清单
 *
 * 上一版把「当前机型 + 当前版本」做成全局上下文，于是想比较三台机器的同一个字段，
 * 只能切过去记下来再切回来。这里列由树上的勾选给出，**可以同时看 16 列**。
 *
 * 列序与行序都由后端定（`wb_matrix`）：列照配方本顺序不按勾选顺序，行取并集不取交集。
 * 前端一个都不重排 —— 重排的话「同一份数据每次长得不一样」这件事会从后端漏到前端。
 *
 * **行序是四段键排出来的**（分类 → 组 → 父子 → 组内序）。这里只做一件事：
 * `sectionLabel` 与上一行不同时插一条分组表头。分组不是前端聚合出来的。
 *
 * # 表格：原生 table + 双向 sticky，没有虚拟滚动
 *
 * 74 行 × 最多 16 列，虚拟滚动带来的复杂度换不回什么。
 * `border-collapse: separate` 是必须的 —— `collapse` 会让 sticky 单元格的边框丢掉。
 *
 * # 一次点击不许让任何东西移动
 *
 * 这是上一稿最伤的地方：选中条按需插入、列头角标多一行、文本换成控件高度变一点 ——
 * 三处加起来，每点一下整张表就往下掉一截，于是下一次点击必然点偏。现在：
 *
 * - 点一格**不显示选中条**（那一格里正亮着输入框，自解释）
 * - 列/行的选中条**常驻占位**，没内容时只是看不见
 * - 「列编辑」占列头第三行原本那句话的位置，不新增行
 * - 文本与控件共用 `--w-cell-h`
 *
 * # 三种改法，一个写入口（doc §8.3）
 *
 * | 手势 | 触发 | 行为 | 撤销粒度 |
 * |---|---|---|---|
 * | 点格子 | `sel.kind === 'cell'` | 这一格升级成真控件 | 每改一格一条 |
 * | 点列头 | `sel.kind === 'col'` | **整列**同时可编辑 | 每改一格一条 |
 * | 点字段名 | `sel.kind === 'row'` | 行下面插一条批量条 → 预览 → 确认 | **N 列压一条** |
 *
 * 退出选中有四条路：`Esc`、点滚动区空白、再点一次同一个头、选中条上的退出链接。
 *
 * # 单元格四条互斥分支（顺序即优先级）
 *
 * 1. **不适用** —— 这台机型的 `machineFilter` 把这个字段排除了。它**没有这一项**。
 * 2. **G-code** —— 只报行数，内容走右抽屉。
 * 3. **开关** —— 布尔项常驻真开关，点一下直接改，不用先选中。
 * 4. **值** —— 后端格式化好的文本 + 来源徽章。
 *
 * 被上级条件关着的格子**照样能点**：点开的不是编辑器，是「谁把我关了」那句说明
 * 加一个跳过去的按钮。`disabled` 的按钮连 tooltip 都弹不出来，那等于让人猜。
 *
 * 判定全在后端（`Cell.kind` / `Cell.editable` / `Cell.blockedNote` / `Cell.jumpTo`）。
 */
import { Fragment, useCallback, useEffect, useMemo, useRef, useState } from 'react'

import {
  isAppError,
  wb,
  type BulkPreview,
  type Cell,
  type Col,
  type ColRef,
  type Matrix,
  type ParamView,
  type Patch,
  type RegistryView,
  type Row,
  type Words,
} from '../api'
import { FieldControl, Switch } from './FieldControl'

/** 选中：三种改法各一种，外加「什么都没选」 */
type Sel =
  | { kind: 'none' }
  | { kind: 'cell'; row: string; col: string }
  | { kind: 'col'; col: string }
  | { kind: 'row'; row: string }

/** 哪一格的「谁把我关了」说明正展开着 */
type Why = { row: string; col: string } | null

interface Props {
  cols: ColRef[]
  words: Words
  /** 草稿变了要重取。用 dirtyCount 当信号，比传回调简单且不会漏 */
  dirtyKey: number
  /** 从配方页跳过来时要定位的那一项。**null = 没人要求定位** */
  focusKey?: string | null
  /** 唯一写入口。`label` 是给撤销按钮显示的一句人话 */
  onApply: (label: string, patches: Patch[]) => Promise<unknown>
}

export function ParamsMatrix({ cols, words, dirtyKey, focusKey, onApply }: Props) {
  const [registry, setRegistry] = useState<RegistryView | null>(null)
  const [matrix, setMatrix] = useState<Matrix | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [tab, setTab] = useState<string | null>(null)
  const [query, setQuery] = useState('')
  const [sel, setSel] = useState<Sel>({ kind: 'none' })
  const [why, setWhy] = useState<Why>(null)
  const [gcode, setGcode] = useState<{ row: Row; col: Col; cell: Cell } | null>(null)
  /** 跳转目标：切了页签/清了搜索之后才知道那一行在不在，所以要等下一份矩阵 */
  const [pending, setPending] = useState<{ row: string; col: string } | null>(null)
  const scrollRef = useRef<HTMLDivElement | null>(null)

  useEffect(() => {
    void wb
      .registry()
      .then(setRegistry)
      .catch((e: unknown) => setError(isAppError(e) ? e.message : String(e)))
  }, [])

  const load = useCallback(() => {
    void wb
      .matrix(cols, tab, query)
      .then(setMatrix)
      .catch((e: unknown) => setError(isAppError(e) ? e.message : String(e)))
  }, [cols, tab, query])

  useEffect(load, [load, dirtyKey])

  const clear = useCallback(() => {
    setSel({ kind: 'none' })
    setWhy(null)
  }, [])

  /* Esc：抽屉开着优先关抽屉，其次收说明，最后清选中。**一层一层退**，
     一下全清掉会让「刚才在看的那句解释」跟着消失 */
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return
      if (gcode) {
        setGcode(null)
        return
      }
      if (why) {
        setWhy(null)
        return
      }
      setSel({ kind: 'none' })
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [gcode, why])

  const paramOf = useCallback(
    (key: string) => registry?.params.find((p) => p.key === key) ?? null,
    [registry],
  )

  /** 写一格。**层由列决定，前端不猜** */
  const setOne = useCallback(
    async (row: Row, col: Col, next: unknown) => {
      const owner = col.level === 'machine' ? col.machineId : (col.versionUid ?? '')
      await onApply(`${col.machine} ${col.label} · ${row.label}`, [
        { kind: 'setValue', level: col.level, owner, key: row.key, value: next },
      ])
      setWhy(null)
      // 列编辑要留着（整列连着改是它存在的理由），点一格改完就收
      setSel((s) => (s.kind === 'col' ? s : { kind: 'none' }))
    },
    [onApply],
  )

  /** 挂回继承 = 删掉这一层的这个键。**不是写回上一层的值** */
  const detach = useCallback(
    async (row: Row, col: Col) => {
      const owner = col.level === 'machine' ? col.machineId : (col.versionUid ?? '')
      await onApply(`${col.machine} ${col.label} · ${row.label} 挂回继承`, [
        { kind: 'setValue', level: col.level, owner, key: row.key, value: null },
      ])
    },
    [onApply],
  )

  /** 滚到某一行并选中它在这一列上的格子 */
  const focusCell = useCallback((rowKey: string, colKey: string) => {
    setSel({ kind: 'cell', row: rowKey, col: colKey })
    setWhy(null)
    const el = scrollRef.current?.querySelector(`[data-rowkey="${rowKey}"]`)
    el?.scrollIntoView({ block: 'center' })
  }, [])

  /**
   * 「去改那一项」。目标行可能**不在当前这一屏**：被分类页签滤掉了，或者被搜索滤掉了。
   * 那就先把过滤让开，等新的矩阵回来再选中 —— 直接选一个不在 DOM 里的格子等于什么都没发生
   */
  const jumpTo = useCallback(
    (rowKey: string, colKey: string) => {
      const here = matrix?.rows.some((r) => r.key === rowKey)
      if (here) {
        focusCell(rowKey, colKey)
        return
      }
      setQuery('')
      setTab(paramOf(rowKey)?.tabId ?? null)
      setPending({ row: rowKey, col: colKey })
    },
    [matrix, focusCell, paramOf],
  )

  useEffect(() => {
    if (!pending || !matrix) return
    if (!matrix.rows.some((r) => r.key === pending.row)) return
    focusCell(pending.row, pending.col)
    setPending(null)
  }, [pending, matrix, focusCell])

  /* 从配方页跳过来：那一项在这一屏里的话就滚过去。
     只滚不选中 —— 对比视角是「看」，替用户选中一格反而像替他动了什么 */
  useEffect(() => {
    if (!focusKey || !matrix) return
    const el = scrollRef.current?.querySelector(`[data-rowkey="${focusKey}"]`)
    el?.scrollIntoView({ block: 'center' })
  }, [focusKey, matrix])

  const searching = query.trim().length > 0
  const tabTitle = useMemo(
    () =>
      searching
        ? `搜索中 —— ${words.empty.matrixSearchSpansAllTabs}，分类过滤先让开`
        : undefined,
    [searching, words],
  )

  if (error) return <p className="wb-todo" data-tone="danger">{error}</p>

  return (
    <div className="wb-mx">
      <div className="wb-mx__bar">
        <div className="wb-mx__tabs" title={tabTitle}>
          <button
            type="button"
            className="wb-chip"
            data-on={tab === null ? 'yes' : undefined}
            data-off={searching ? 'yes' : undefined}
            onClick={() => {
              setTab(null)
              setQuery('')
            }}
          >
            全部
          </button>
          {registry?.tabs.map((t) => (
            <button
              key={t.id}
              type="button"
              className="wb-chip"
              data-on={tab === t.id ? 'yes' : undefined}
              data-off={searching ? 'yes' : undefined}
              /* 点任一页签先清空搜索：否则「我选了分类却还是看到别的分类的字段」 */
              onClick={() => {
                setTab(t.id)
                setQuery('')
              }}
            >
              {t.label}
            </button>
          ))}
        </div>

        <input
          className="wb-input"
          type="search"
          placeholder="搜字段名 / key / tomlKey / 分类 / 说明"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />

        {matrix && (
          <span className="wb-mx__count">
            显示 {matrix.rows.length} / 共 {matrix.totalRows}
          </span>
        )}
      </div>

      {matrix?.note && <p className="wb-mx__note">{matrix.note}</p>}

      {/* 选中条：**常驻占位**。只服务列编辑与批量 —— 点一格不需要它 */}
      <SelBar sel={sel} matrix={matrix} onClear={clear} />

      {matrix?.emptyReason ? (
        <p className="wb-todo">{matrix.emptyReason}</p>
      ) : (
        matrix && (
          <div
            className="wb-mx__scroll"
            ref={scrollRef}
            /* 点空白处退出选中。判 `target === currentTarget` 是为了
               不把「点格子」这一下也当成点空白 */
            onClick={(e) => e.target === e.currentTarget && clear()}
          >
            <table className="wb-mx__table">
              <thead>
                <tr>
                  <th className="wb-mx__corner">字段 / 对象</th>
                  {matrix.cols.map((c) => {
                    const on = sel.kind === 'col' && sel.col === c.key
                    return (
                      <th
                        key={c.key}
                        className="wb-mx__colhead"
                        data-on={on ? 'yes' : undefined}
                      >
                        <button
                          type="button"
                          className="wb-mx__headbtn"
                          title={
                            on
                              ? '再点一下退出列编辑（Esc 也行）'
                              : `点一下把整列切成可编辑：${c.machine} ${c.label}（写进${words.level[c.level].label}）`
                          }
                          onClick={() =>
                            setSel((s) =>
                              s.kind === 'col' && s.col === c.key
                                ? { kind: 'none' }
                                : { kind: 'col', col: c.key },
                            )
                          }
                        >
                          <span className="wb-mx__machine">{c.machine}</span>
                          <span className="wb-mx__label">{c.label}</span>
                          {/* 第三行**原位替换**，不新增行 —— 新增一行会把整表顶下去 */}
                          <span className="wb-mx__own">
                            {on ? (
                              <i className="wb-mx__colflag">列编辑 · 再点退出</i>
                            ) : (
                              `自有 ${c.own} · ${words.level[c.level].label}`
                            )}
                          </span>
                        </button>
                      </th>
                    )
                  })}
                </tr>
              </thead>
              <tbody>
                {matrix.rows.map((r, i) => (
                  <Fragment key={r.key}>
                    {/* 分组表头：组名与上一行不同就插一条。
                        **搜索时不插** —— 命中散落在各组，会变成一堆只有一行的组 */}
                    {!searching && r.sectionLabel !== matrix.rows[i - 1]?.sectionLabel && (
                      <tr className="wb-mx__group">
                        <th colSpan={matrix.cols.length + 1}>{r.sectionLabel}</th>
                      </tr>
                    )}
                    <RowView
                      row={r}
                      cols={matrix.cols}
                      sel={sel}
                      why={why}
                      words={words}
                      param={paramOf(r.key)}
                      showSection={searching}
                      onPickCell={(col) => {
                        setSel({ kind: 'cell', row: r.key, col })
                        setWhy(null)
                      }}
                      onPickRow={() =>
                        setSel((s) =>
                          s.kind === 'row' && s.row === r.key
                            ? { kind: 'none' }
                            : { kind: 'row', row: r.key },
                        )
                      }
                      onWhy={(col) =>
                        setWhy((w) =>
                          w && w.row === r.key && w.col === col ? null : { row: r.key, col },
                        )
                      }
                      onJump={(key, col) => jumpTo(key, col)}
                      onCommit={(col, v) => void setOne(r, col, v)}
                      onDetach={(col) => void detach(r, col)}
                      onCancel={() => setSel({ kind: 'none' })}
                      onOpenGcode={(col, cell) => setGcode({ row: r, col, cell })}
                      cols_ref={cols}
                      onApply={onApply}
                      onDone={clear}
                    />
                  </Fragment>
                ))}
              </tbody>
            </table>
          </div>
        )
      )}

      {gcode && (
        <GcodeDrawer
          row={gcode.row}
          col={gcode.col}
          cell={gcode.cell}
          param={paramOf(gcode.row.key)}
          onClose={() => setGcode(null)}
          onCommit={(v) => {
            void setOne(gcode.row, gcode.col, v)
            setGcode(null)
          }}
        />
      )}
    </div>
  )
}

/**
 * 选中条。**高度恒定**：没内容时 `visibility: hidden`，不是不渲染 ——
 * 按需插入会让整张表在每次点击时上下跳一截，下一次点击就点偏了。
 *
 * 点一格时它是空的：那一格里正亮着输入框，再说一遍「正在改 X」是多余的
 */
function SelBar({
  sel,
  matrix,
  onClear,
}: {
  sel: Sel
  matrix: Matrix | null
  onClear: () => void
}) {
  const show = sel.kind === 'col' || sel.kind === 'row'
  const col = matrix && sel.kind === 'col' ? matrix.cols.find((c) => c.key === sel.col) : undefined
  const row = matrix && sel.kind === 'row' ? matrix.rows.find((r) => r.key === sel.row) : undefined

  return (
    <div className="wb-selbar" data-empty={show ? undefined : 'yes'} aria-hidden={!show}>
      <span>
        {sel.kind === 'col'
          ? `整列可编辑：${col?.machine ?? ''} ${col?.label ?? ''}`
          : sel.kind === 'row'
            ? `一次改所有在看的列：${row?.label ?? ''}`
            : ''}
      </span>
      <button type="button" className="wb-link" onClick={onClear}>
        {sel.kind === 'row' ? '退出批量' : '退出列编辑'}
      </button>
    </div>
  )
}

interface RowProps {
  row: Row
  cols: Col[]
  sel: Sel
  why: Why
  words: Words
  param: ParamView | null
  /** 搜索中没有分组表头，组名改挂在行头上 */
  showSection: boolean
  onPickCell: (col: string) => void
  onPickRow: () => void
  onWhy: (col: string) => void
  onJump: (rowKey: string, col: string) => void
  onCommit: (col: Col, v: unknown) => void
  onDetach: (col: Col) => void
  onCancel: () => void
  onOpenGcode: (col: Col, cell: Cell) => void
  cols_ref: ColRef[]
  onApply: (label: string, patches: Patch[]) => Promise<unknown>
  onDone: () => void
}

function RowView(p: RowProps) {
  const { row, cols, sel, why, words, param } = p
  const bulkOpen = sel.kind === 'row' && sel.row === row.key

  return (
    <>
      <tr data-rowkey={row.key}>
        <th
          className="wb-mx__rowhead"
          data-on={bulkOpen ? 'yes' : undefined}
          data-depth={row.depth || undefined}
        >
          <button
            type="button"
            className="wb-mx__headbtn"
            title={`点一下一次改所有在看的列：${row.label}${row.desc ? `（${row.desc}）` : ''}`}
            onClick={p.onPickRow}
          >
            <span className="wb-mx__field">
              {row.label}
              {row.deprecated && <i className="wb-mx__dep">已废弃</i>}
            </span>
            <span className="wb-mx__key">{row.key}</span>
            {/* 归属与「受谁控制」**常驻**。「不知道是哪个选项导致它灰色」的正解是这两句
                在格子还没变灰的时候就已经在这儿了，而不是等灰了再去悬停 */}
            {p.showSection && <span className="wb-mx__rel">{row.sectionLabel}</span>}
            {row.parentNote && <span className="wb-mx__rel">{row.parentNote}</span>}
            {row.controlNote && <span className="wb-mx__rel">{row.controlNote}</span>}
          </button>
        </th>
        {row.cells.map((cell, i) => {
          const col = cols[i]
          if (!col) return null
          const editing =
            (sel.kind === 'cell' && sel.row === row.key && sel.col === col.key) ||
            (sel.kind === 'col' && sel.col === col.key)
          return (
            <CellView
              key={col.key}
              cell={cell}
              col={col}
              words={words}
              param={param}
              editing={editing && cell.editable && cell.kind === 'value'}
              colEdit={sel.kind === 'col' && sel.col === col.key}
              whyOpen={Boolean(why && why.row === row.key && why.col === col.key)}
              onPick={() => p.onPickCell(col.key)}
              onWhy={() => p.onWhy(col.key)}
              onJump={(key) => p.onJump(key, col.key)}
              onCommit={(v) => p.onCommit(col, v)}
              onDetach={() => p.onDetach(col)}
              onCancel={p.onCancel}
              onOpenGcode={() => p.onOpenGcode(col, cell)}
            />
          )
        })}
      </tr>
      {bulkOpen && (
        <tr>
          <td className="wb-bulk" colSpan={cols.length + 1}>
            <BulkBar
              row={row}
              param={param}
              cols={p.cols_ref}
              words={words}
              onApply={p.onApply}
              onDone={p.onDone}
            />
          </td>
        </tr>
      )}
    </>
  )
}

function CellView({
  cell,
  col,
  words,
  param,
  editing,
  colEdit,
  whyOpen,
  onPick,
  onWhy,
  onJump,
  onCommit,
  onDetach,
  onCancel,
  onOpenGcode,
}: {
  cell: Cell
  col: Col
  words: Words
  param: ParamView | null
  editing: boolean
  colEdit: boolean
  whyOpen: boolean
  onPick: () => void
  onWhy: () => void
  onJump: (rowKey: string) => void
  onCommit: (v: unknown) => void
  onDetach: () => void
  onCancel: () => void
  onOpenGcode: () => void
}) {
  /* 不适用：**没有这一项**，不是值为空 */
  if (cell.kind === 'notApplicable') {
    return (
      <td className="wb-mx__cell" data-kind="na" title={cell.reason ?? undefined}>
        <span className="wb-mx__slot">{cell.text}</span>
      </td>
    )
  }

  const originTitle = cell.origin
    ? `${cell.originLabel} · ${cell.originExplain}${cell.dirty ? ' · 已改动未保存' : ''}`
    : undefined
  const isSwitch = param?.uiComponent === 'switch'

  return (
    <td
      className="wb-mx__cell"
      data-kind={cell.kind}
      data-blocked={cell.editable ? undefined : 'yes'}
      data-dirty={cell.dirty ? 'yes' : undefined}
      data-editing={editing ? 'yes' : undefined}
      data-coledit={colEdit ? 'yes' : undefined}
    >
      <span className="wb-mx__slot">
        {cell.kind === 'gcode' ? (
          <button
            type="button"
            className="wb-mx__gcode"
            onClick={onOpenGcode}
            title="点开看全文并编辑 —— G-code 不在格子里改"
          >
            {cell.text}
          </button>
        ) : isSwitch ? (
          /* 开关常驻：改一个布尔项一下就够，不用先选中 */
          <Switch
            on={cell.raw === true}
            disabledReason={cell.editable ? null : cell.reason}
            onToggle={onCommit}
            onBlocked={onWhy}
          />
        ) : editing && param ? (
          <FieldControl
            param={param}
            value={cell.raw}
            form="cell"
            autoFocus
            onCommit={onCommit}
            onCancel={onCancel}
          />
        ) : (
          <button
            type="button"
            className="wb-mx__value"
            /* **不 disabled**：关着的格子也要点得开，点开的是「谁把我关了」那句说明。
               disabled 的按钮连 tooltip 都弹不出来，等于让人猜 */
            onClick={cell.editable ? onPick : onWhy}
            title={cell.editable ? '点一下改这一格' : '点一下看是谁把它关着的'}
          >
            {cell.text}
          </button>
        )}

        {cell.origin && (
          <span className="wb-mx__origin" data-origin={cell.origin} title={originTitle}>
            {words.origin[cell.origin].label}
          </span>
        )}

        {/* 挂回继承：**只在这一层自己写过时可点**。不可点时也给一句，别让人猜 */}
        <button
          type="button"
          className="wb-mx__detach"
          disabled={!cell.own}
          onClick={onDetach}
          title={
            cell.own
              ? `${words.disabled.detachReady}（这一层是${words.level[col.level].label}）`
              : words.disabled.detachNothing
          }
        >
          ⤺
        </button>
      </span>

      {/* 「谁把我关了」。整句由后端拼好（`blockedNote`），前端不组装；
          `jumpTo` 为空表示那一项在这台机型上不存在，这时候**不给按钮** */}
      {whyOpen && cell.blockedNote && (
        <span className="wb-mx__why">
          <span>{cell.blockedNote}</span>
          {cell.jumpTo && (
            <button
              type="button"
              className="wb-link"
              onClick={() => cell.jumpTo && onJump(cell.jumpTo)}
            >
              {words.relate.goFixIt}
            </button>
          )}
        </span>
      )}
    </td>
  )
}

/**
 * 批量条（doc §8.4 的五步 SOP 的第 3–5 步）。
 *
 * ```
 * 勾列（在树上）→ 点字段名 → 输入新值 → 看影响 → 确认，落一次撤销
 * ```
 *
 * 三种互斥形态：**G-code 拒绝**、**零可落列**、正常。
 * 「能落到哪几列」由后端 `wb_preview_bulk` 判 —— 前端判会和矩阵那边的判定分岔。
 */
function BulkBar({
  row,
  param,
  cols,
  words,
  onApply,
  onDone,
}: {
  row: Row
  param: ParamView | null
  cols: ColRef[]
  words: Words
  onApply: (label: string, patches: Patch[]) => Promise<unknown>
  onDone: () => void
}) {
  const [draft, setDraft] = useState<unknown>(param?.defaultValue ?? '')
  const [preview, setPreview] = useState<BulkPreview | null>(null)
  const [busy, setBusy] = useState(false)

  // G-code 一律拒绝，连控件都不给 —— 一段多行脚本被整体盖掉是不可逆的误操作
  if (row.gcode) {
    return <p className="wb-bulk__no">{words.disabled.bulkRefusesGcode}</p>
  }
  if (!param) return null

  return (
    <div className="wb-bulk__inner">
      <span className="wb-bulk__label">一次改所有在看的列：{row.label}</span>
      <FieldControl
        param={param}
        value={draft}
        form="row"
        onCommit={(v) => {
          setDraft(v)
          setPreview(null)
        }}
        onCancel={() => setPreview(null)}
      />
      <button
        type="button"
        className="wb-btn"
        disabled={busy}
        onClick={() => {
          setBusy(true)
          void wb
            .previewBulk(row.key, draft, cols)
            .then(setPreview)
            .finally(() => setBusy(false))
        }}
      >
        看影响
      </button>

      {preview &&
        (preview.effects.length === 0 ? (
          <p className="wb-bulk__no">这几列都没有这个字段，或都被上级条件关着</p>
        ) : (
          <div className="wb-bulk__preview">
            <p className="wb-bulk__note">
              改完先看影响再确认。{preview.effects.length} 列 ——
              基底列写进{words.level.machine.label}
              （会连带影响没自己写过这一项的版本）；版本列写进这个版本的
              {words.level.version.label}。
            </p>
            <table className="wb-diff">
              <thead>
                <tr>
                  <th>列</th>
                  <th>现在</th>
                  <th>改成</th>
                  <th>类型</th>
                </tr>
              </thead>
              <tbody>
                {preview.effects.map((e) => (
                  <tr key={e.col}>
                    <td>
                      {e.machine} {e.label}
                    </td>
                    <td className="wb-diff__del">{e.before}</td>
                    <td className="wb-diff__add">{e.after}</td>
                    <td>{words.bulkKind[e.kind].label}</td>
                  </tr>
                ))}
              </tbody>
            </table>

            {/* 跳过的也列出来 —— 否则「我勾了 6 列怎么只改了 4 列」变成一个谜 */}
            {preview.skipped.length > 0 && (
              <p className="wb-bulk__skip">
                跳过 {preview.skipped.length} 列：
                {preview.skipped
                  .map((s) => `${s.machine} ${s.label}（${s.reason}）`)
                  .join('；')}
              </p>
            )}

            <div className="wb-bulk__actions">
              <button type="button" className="wb-btn" onClick={onDone}>
                取消
              </button>
              <button
                type="button"
                className="wb-btn"
                data-tone="primary"
                disabled={busy}
                onClick={() => {
                  setBusy(true)
                  // **N 列压一条撤销**：一次手势一条，而不是 N 条
                  const patches: Patch[] = preview.effects.map((e) => ({
                    kind: 'setValue',
                    level: e.level,
                    owner: e.col,
                    key: row.key,
                    value: draft,
                  }))
                  void onApply(`批量改 ${row.label}（${patches.length} 列）`, patches)
                    .then(onDone)
                    .finally(() => setBusy(false))
                }}
              >
                确认，落一次撤销
              </button>
            </div>
          </div>
        ))}
    </div>
  )
}

/**
 * G-code 抽屉。**几十行脚本不在格子里改**：格子里放一个多行输入框会把行高撑坏，
 * 而缩成一行又看不见自己在改什么
 */
function GcodeDrawer({
  row,
  col,
  cell,
  param,
  onClose,
  onCommit,
}: {
  row: Row
  col: Col
  cell: Cell
  param: ParamView | null
  onClose: () => void
  onCommit: (v: string) => void
}) {
  const [text, setText] = useState(typeof cell.raw === 'string' ? cell.raw : '')

  return (
    <>
      {/* 遮罩接点击。absolute 不是 fixed —— 这套界面以后可能进带 transform 的外壳。
          Esc 在矩阵那边统一处理（抽屉优先），这里不再挂第二个监听 */}
      <div className="wb-backdrop" onClick={onClose} aria-hidden />
      <aside className="wb-drawer" role="dialog" aria-label={`${row.label} 的 G-code`}>
        <header className="wb-drawer__head">
          <span>
            {row.label} · {col.machine} {col.label}
          </span>
          <button type="button" className="wb-link" onClick={onClose}>
            关闭
          </button>
        </header>
        {param?.desc && <p className="wb-drawer__desc">{param.desc}</p>}
        <textarea
          className="wb-drawer__ta"
          value={text}
          spellCheck={false}
          onChange={(e) => setText(e.target.value)}
        />
        <footer className="wb-drawer__foot">
          <span className="wb-mx__count">{text.split('\n').length} 行</span>
          <button type="button" className="wb-btn" onClick={onClose}>
            取消
          </button>
          <button
            type="button"
            className="wb-btn"
            data-tone="primary"
            disabled={!cell.editable}
            title={cell.editable ? undefined : (cell.reason ?? undefined)}
            onClick={() => onCommit(text)}
          >
            保存这一段
          </button>
        </footer>
      </aside>
    </>
  )
}
