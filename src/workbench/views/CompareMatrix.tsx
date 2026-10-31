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
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react'

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
  /**
   * 多行 G-code：**打开模态框**那一枚按钮的动作（C14 第十八轮那枚「开新窗口」）。
   *
   * 由参数台给（模态框挂在定位祖先上，不能挂在一行里）；不给就不画那枚按钮。
   * 单版本那一行一直是这么接的 —— 对照矩阵这一支最初漏了，于是格子里虽能就地改，
   * 却打不开全屏编辑器（2026-10-31 补上）。
   */
  onOpenGcode?: (col: Col, row: Row, param: ParamView) => void
}

/**
 * 能不能拿这个参数去批量（C14 第十五轮）：gcode 没有「设成同一个值」这回事；
 * **全局**弃用的不给批量入口 —— 没有的入口好过灰掉的入口。
 *
 * 只点名了部分机型的那种仍然给入口：勾的列里只有那几台会被后端拒，
 * 换一批机器照样能批量（拦在 `BatchEdit` 的手势前那一道，说得出是哪一档）。
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
  onOpenGcode,
}: Props) {
  const [editing, setEditing] = useState<string | null>(null)
  /** 正在编辑的那一格。点外面时用它判断「这一下是不是点在格子里」 */
  const editRef = useRef<HTMLSpanElement | null>(null)

  /**
   * G-code 浮窗的落点（2026-10-31 第三修）。
   *
   * 浮窗是 `position: fixed`：**只有脱离 `.matrix` 那个 `overflow: auto`**，
   * 它才能长到表格容器外面去。作者的原话：「还是被截断了呀，它可以出现到下面的呀」
   * —— 那一眼看到的正是「框的下沿与表格底边齐平」，下面明明还有空地。
   *
   * 位置由这一格自己的 `getBoundingClientRect()` 算，写回 `--gce-left/top`：
   *   · 右边界放不下 ⇒ 贴格子右沿向左展开；
   *   · 视口下方放不下 ⇒ 贴格子下沿向上展开；
   * 滚动（含表格自己的横向滚动）/ 窗口缩放时跟着走。
   */
  const placeGcode = useCallback(() => {
    const cell = editRef.current
    const wrap = cell?.querySelector<HTMLElement>('[class*="pGwrap"]')
    if (!cell || !wrap) return
    const box = cell.getBoundingClientRect()
    const w = wrap.offsetWidth
    const h = wrap.offsetHeight
    const vw = window.innerWidth
    const vh = window.innerHeight
    let left = box.left
    if (left + w > vw - 8) left = Math.max(8, Math.min(box.right - w, vw - w - 8))
    let top = box.top
    if (top + h > vh - 8) top = Math.max(8, box.bottom - h)
    cell.style.setProperty('--gce-left', `${Math.round(left)}px`)
    cell.style.setProperty('--gce-top', `${Math.round(top)}px`)
  }, [])

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

  /* 浮窗落点：每次提交后（含内容改动导致宽高变）都重算一次，绘制前完成 */
  useLayoutEffect(() => {
    if (editing === null) return
    placeGcode()
  })

  /* 滚动（表格内滚也算）/ 窗口缩放时跟着格子走；rAF 合并成每帧一次 */
  useEffect(() => {
    if (editing === null) return
    let raf = 0
    const soon = () => {
      if (raf !== 0) return
      raf = requestAnimationFrame(() => {
        raf = 0
        placeGcode()
      })
    }
    window.addEventListener('scroll', soon, true)
    window.addEventListener('resize', soon)
    return () => {
      if (raf !== 0) cancelAnimationFrame(raf)
      window.removeEventListener('scroll', soon, true)
      window.removeEventListener('resize', soon)
    }
  }, [editing, placeGcode])

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
                    data-tip={`选中 ${row.label} —— 右侧「批量修改」把这个值写到多个版本`}
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
                          /* G-code 那枚「打开编辑器」：与单版本行上那枚同一个动作 */
                          onOpenGcode={
                            onOpenGcode ? () => onOpenGcode(c, row, p) : undefined
                          }
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
                  <span className={s.pTagNA} data-tip={words.matrixRow.notOwn.explain ?? undefined}>
                    {words.matrixRow.notOwn.label}
                  </span>
                ) : isDiff ? (
                  <span className={s.pTagDiff} data-tip={words.matrixRow.diff.explain ?? undefined}>
                    {words.matrixRow.diff.label}
                  </span>
                ) : (
                  <span className={s.pTagSame} data-tip={words.matrixRow.same.explain ?? undefined}>
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
