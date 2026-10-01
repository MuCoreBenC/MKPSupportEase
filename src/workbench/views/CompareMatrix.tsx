/*
 * 版本对照（C14 移植）——只回答「版本之间到底哪里不一样」：
 *
 *   单版本  我要改东西 → 一行只给「当前值 / 为什么」
 *   对照    我要理解差别 → 一行给 N 个版本，不一样的格子亮出来 + 可以整批改
 *
 * 与原型的分工：**差异判据在后端**——每格 `differs`、每行 `diffKeys`、
 * 「本机无此项」的 `notOwnKeys` 都是 `wb_matrix` 按基准机型算好的
 * （基准写死基准机型的机型基底，与勾选顺序无关，C14 第四轮）；
 * 前端只负责亮绿底、给状态列贴词（词来自 `words.matrixRow`）。
 *
 * 格子里的编辑态要铺满整格（C14 第四轮，`.cellEd` 的 CSS）；点空白收起编辑态，
 * 点在下拉浮层里不算空白（浮层 portal 到 FieldLayer，按 `data-popover` 认）。
 * 「点一下换下一个值」试过又被作者撤掉——**选项要能看见**，浮层开就行。
 */
import { useEffect, useRef, useState } from 'react'

import type { Col, Matrix, ParamView, Row, Words } from '../api'
import CellEditor from './CellEditor'
import s from '../c14.module.css'

interface Props {
  matrix: Matrix
  /** 基准机型（列头「跨机型」标记与行序都跟它走） */
  baseMachineId: string
  words: Words
  paramOf: (key: string) => ParamView | null
  /** **选中的那一个**参数（单选，C14 第十五轮）。详情与批量都读它 */
  sel: string | null
  /** 点参数名 → 详情页 */
  onPick: (key: string) => void
  /** 点行首那枚圆点 → 批量页（同一个「选中」，只是右侧翻到另一页签） */
  onPickBatch: (key: string) => void
  /** 写一个格子。层由那一列决定（基底列 = 机型层，其余 = 版本层） */
  onWriteCell: (col: Col, row: Row, param: ParamView, next: string) => void
}

/**
 * 能不能拿这个参数去批量（C14 第十五轮）：gcode 没有「设成同一个值」这回事；
 * 已弃用的**不给批量入口**——没有的入口好过灰掉的入口。
 */
const canBatch = (param: ParamView | null): boolean =>
  param !== null && param.uiComponent !== 'gcode' && !param.deprecated

export default function CompareMatrix({
  matrix,
  baseMachineId,
  words,
  paramOf,
  sel,
  onPick,
  onPickBatch,
  onWriteCell,
}: Props) {
  const [editing, setEditing] = useState<string | null>(null)
  /** 正在编辑的那一格。点外面时用它判断「这一下是不是点在格子里」 */
  const editRef = useRef<HTMLSpanElement | null>(null)

  /*
   * 参数行没有刷选、没有多选（C14 第十五轮）：一次只选一个参数。
   * 左树那套勾选是**版本列**的多选，在页面那层。
   */

  /* 点开这一格就把焦点交给里面的控件（C14 第十四轮） */
  useEffect(() => {
    if (editing === null) return
    const node = editRef.current?.querySelector<HTMLElement>(
      'input, textarea, button[aria-haspopup="listbox"]',
    )
    node?.focus()
  }, [editing])

  /* 点空白收起编辑态；浮层（data-popover）不算外面 */
  useEffect(() => {
    if (editing === null) return
    const onDown = (e: MouseEvent) => {
      const t = e.target as HTMLElement | null
      if (t === null) return
      if (editRef.current?.contains(t) === true) return
      if (t.closest('[data-popover]') !== null) return
      setEditing(null)
    }
    window.addEventListener('mousedown', onDown)
    return () => window.removeEventListener('mousedown', onDown)
  }, [editing])

  if (!matrix.rows.length) {
    return (
      <div className={`${s.card} ${s.sum}`} style={{ padding: 20 }}>
        没有可对照的参数 —— 换个分组，或者把「仅显示差异」关掉
      </div>
    )
  }

  const cols = matrix.cols

  return (
    <div className={`${s.card} ${s.matrix}`}>
      <div className={`${s.mRow} ${s.mHead}`}>
        <span className={s.pPick} />
        <span className={s.pKey}>参数</span>
        {cols.map((c) => (
          <span
            key={c.key}
            className={`${s.pCol} ${
              c.level === 'machine' && c.machineId === baseMachineId ? s.pColBase : ''
            } ${c.machineId !== baseMachineId ? s.pColCross : ''}`}
          >
            {c.label}
          </span>
        ))}
        <span className={s.pState}>状态</span>
      </div>

      {matrix.rows.map((row) => {
        const p = paramOf(row.key)
        const isDiff = matrix.diffKeys.includes(row.key)
        const own = !matrix.notOwnKeys.includes(row.key)
        /* 已弃用：名字划线，格子写「已弃用」而不是「不适用」（两件事） */
        const dep = row.deprecated
        return (
          <div key={row.key}>
            <div className={`${s.mRow} ${sel === row.key ? s.mRowOn : ''}`}>
              {/*
                行首那枚**单选圆点**（C14 第十五轮）：「一行就是对应的多个版本」。
                gcode / 弃用这类没有批量的行，这一格空着（没有入口好过灰入口）。
              */}
              <span className={s.pPick}>
                {canBatch(p) && (
                  <button
                    type="button"
                    className={s.pPickDot}
                    data-on={sel === row.key}
                    aria-pressed={sel === row.key}
                    title={`选中 ${row.label} —— 右侧「批量修改」把这个值写到多个版本`}
                    onClick={() => onPickBatch(row.key)}
                  />
                )}
              </span>
              <span className={s.pKey} data-depth={row.depth} data-dep={dep ? '' : undefined}>
                <button type="button" className={s.pKeyName} onClick={() => onPick(row.key)}>
                  {row.label}
                </button>
                {row.unit && <em className={s.pSub}>{row.unit}</em>}
              </span>

              {cols.map((c, ci) => {
                const cell = row.cells[ci]
                const use = cell.kind !== 'notApplicable'
                const id = `${c.key}|${row.key}`
                const open = editing === id
                const differs = own && cell.differs
                /* 弃用（且没被条件关着）的格子写「已弃用」，不是「不适用」——两件事 */
                const depOnly = dep && cell.blocked.length === 0
                return (
                  <span
                    key={c.key}
                    /* 能编辑的格子不参与刷选 —— 那上面按下是要进编辑态 */
                    data-nosel={cell.editable ? '' : undefined}
                    /* 编辑态那一格自己描一圈外框（C14 第十三轮） */
                    data-editing={open ? 'true' : undefined}
                    ref={open ? editRef : null}
                    className={`${s.cell} ${cell.editable ? '' : depOnly ? s.cellDep : s.cellOff} ${
                      cell.dirty ? s.cellDirty : ''
                    } ${differs ? s.cellDiff : ''} ${c.machineId !== baseMachineId ? s.cellCross : ''}`}
                    title={
                      !use
                        ? `${c.machine} 没有这个参数`
                        : cell.editable
                          ? (cell.diffTip ?? undefined)
                          : depOnly
                            ? (words.paramDeprecated.explain ?? undefined)
                            : (cell.blockedNote ?? undefined)
                    }
                    onClick={() => {
                      if (!cell.editable) return
                      /* 只在关闭态打开 —— 打开后再点的是控件，不能再把编辑器卸掉 */
                      if (!open && p) setEditing(id)
                    }}
                  >
                    {open && p ? (
                      <span className={s.cellEd}>
                        <CellEditor
                          param={p}
                          cell={cell}
                          form="cell"
                          onWrite={(next) => onWriteCell(c, row, p, next)}
                        />
                      </span>
                    ) : cell.editable ? (
                      <span className={s.cellText}>{cell.text}</span>
                    ) : (
                      <span className={s.offText}>
                        {depOnly ? words.paramDeprecated.label : use ? words.placeholder.notApplicable : '—'}
                      </span>
                    )}
                  </span>
                )
              })}

              <span className={s.pState}>
                {!own ? (
                  <span className={s.pTagNA} title={words.matrixRow.notOwn.explain ?? undefined}>
                    {words.matrixRow.notOwn.label}
                  </span>
                ) : isDiff ? (
                  <span className={s.pTagDiff} title={words.matrixRow.diff.explain ?? undefined}>
                    {words.matrixRow.diff.label}
                  </span>
                ) : (
                  <span className={s.pTagSame} title={words.matrixRow.same.explain ?? undefined}>
                    {words.matrixRow.same.label}
                  </span>
                )}
              </span>
            </div>
          </div>
        )
      })}
    </div>
  )
}
