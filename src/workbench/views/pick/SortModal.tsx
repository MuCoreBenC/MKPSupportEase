/*
 * 组内排序（2026-11-14）。
 *
 * # 它替掉了什么
 *
 * 原来是「组内顺序」那颗自绘下拉：整组列在里面、每行右边两颗 ↑↓，而且**只有当前
 * 这一条能挪**（别的行箭头灰着，写一句「只能挪这一条」）。作者的原话：
 * 「我希望这个改排序的可以不只是改自己的，而且能不能不只是这种上下箭头的，难用，
 * 能不能再出一个模态框……可以方便拖拽排序，可以输入数字排序，可以撤销重做排序，
 * 可以恢复，就是放弃排序」。
 *
 * # 三条路落到同一个结果
 *
 *   拖拽     pointer 事件手写（WebView2 里 HTML5 `draggable` 的头像与 dragover 时序不稳）
 *   输位次   行首那个数字框，回车 / 失焦才认
 *   ↑↓      留着当无障碍兜底（键盘用户也有路走）
 *
 * 三者都只调 `useSortDraft` 的 `move` / `setPosition` —— 没有第二套排序数学。
 *
 * # 确认才落盘，而且是**单独一笔**
 *
 * 排序能挪到别人的位置。要塞进「这条参数的定义载荷」里，就得把别人的定义原样重抄
 * 一遍（抄漏一格 = 静默改坏一条参数），而且 N 条就是 N 次写盘。所以它走
 * `wb_set_section_order`：只提交最终 key 序列，顺序值由后端算，一次写盘一条撤销。
 *
 * 页脚左侧那句「顺序单独生效」说的就是这条 —— 用户得知道它不等父框那颗「确认」。
 */

import { useCallback, useEffect, useRef, useState } from 'react'
import type { PointerEvent as ReactPointerEvent } from 'react'

import { isAppError, wb } from '../../api'
import type { RegistryView } from '../../api'
import ModalC14 from '../../c14/ModalC14'
import { toasts } from '../../c14/toast'
import { useSortDraft } from './useSortDraft'
import s from './SortModal.module.css'

export interface SortRow {
  key: string
  label: string
  /** 名字后面那截浅色旁注（键名） */
  by?: string
}

export interface SortOrderButtonProps {
  sectionId: string
  /** 触发器上那截说明用的组名（「动作与挤出」） */
  sectionLabel: string
  /** 整组参数，**已按当前顺序排好**（含自己） */
  rows: SortRow[]
  /** 当前正在编辑的那一条（框里标一条细轨，触发器上写「第 N / 共 M 位」） */
  currentKey?: string
  /** 只读（分组只剩一条之类）：触发器禁用 */
  disabled?: boolean
  /** 落盘之后把新的注册表视图交回页面 */
  onApplied?: (view: RegistryView) => void
}

export default function SortOrderButton({
  sectionId,
  sectionLabel,
  rows,
  currentKey,
  disabled = false,
  onApplied,
}: SortOrderButtonProps) {
  const [open, setOpen] = useState(false)
  const at = currentKey === undefined ? -1 : rows.findIndex((r) => r.key === currentKey)
  const summary =
    at < 0 ? `共 ${rows.length} 项` : `第 ${at + 1} / ${rows.length} 位`

  return (
    <>
      <button
        type="button"
        className={s.trigger}
        aria-label="组内顺序"
        aria-haspopup="dialog"
        title={disabled ? '这一组只有一条，排不了' : '点开排这一组的先后 —— 可拖、可输位次'}
        disabled={disabled || rows.length < 2}
        onClick={() => setOpen(true)}
      >
        <span className={s.summary}>{summary}</span>
        <span className={s.caret} aria-hidden>
          <svg viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
            <path d="m4 6 4 4 4-4" />
          </svg>
        </span>
      </button>

      {open && (
        <SortDialog
          sectionId={sectionId}
          sectionLabel={sectionLabel}
          rows={rows}
          currentKey={currentKey}
          onApplied={onApplied}
          onClose={() => setOpen(false)}
        />
      )}
    </>
  )
}

interface DialogProps {
  sectionId: string
  sectionLabel: string
  rows: SortRow[]
  currentKey?: string
  onApplied?: (view: RegistryView) => void
  onClose: () => void
}

function SortDialog({
  sectionId,
  sectionLabel,
  rows,
  currentKey,
  onApplied,
  onClose,
}: DialogProps) {
  const draft = useSortDraft(rows.map((r) => r.key))
  const [busy, setBusy] = useState(false)
  /** 拖拽中的两格：从第几行拖到第几行（`over` 是**落地后的下标**） */
  const [drag, setDrag] = useState<{ from: number; over: number } | null>(null)
  /**
   * **落地动画中的那一笔**（2026-11-14）。
   *
   * 松手不能"清 transform + 立刻重排"两步走：清 transform 会让浮起行先弹回原位，
   * 下一帧 DOM 重排又把它搬到落点 —— 中间那一帧就是作者说的「跳一下 / 从上往下落」。
   * 这里先让浮起行带过渡滑到落点，动画结束才一次性重排并清掉全部临时样式。
   */
  const [landing, setLanding] = useState<{ from: number; over: number; dy: number } | null>(null)
  const landTimer = useRef<number | null>(null)
  /** 落地收尾动作：动画正常结束时由定时器调，中途又起拖则提前调一次 */
  const finishLanding = useRef<(() => void) | null>(null)
  const listRef = useRef<HTMLDivElement>(null)
  /*
   * 拖拽的"手感"全在 ref 里，**不走 React state**：
   * 手指每动一像素就 `setState` 的话，一帧里要排队等渲染跟确认，肉眼看就是"拖影/滞后"。
   * 这里只把 `transform` 直接写到那一行 DOM 上；落点下标变了才 setState 让别的行让位。
   */
  const dragLive = useRef<{
    from: number
    over: number
    startY: number
    /** 起拖时列表已滚了多少：滚过之后位移要把它算进去，不然落点会偏 */
    startScroll: number
    /** 真的动过手才算拖（按下不动 = 一次点击，不该进入拖拽态） */
    started: boolean
    el: HTMLElement
  } | null>(null)
  /** 行高固定 30px（与 CSS 同一处口径），让位与落点判定都按它算 */
  const ROW = 30
  /** 起拖阈值：手抖 5px 不算"我要拖" */
  const DRAG_SLOP = 5

  const rowOf = useCallback(
    (key: string): SortRow => rows.find((r) => r.key === key) ?? { key, label: key },
    [rows],
  )

  /* 框关了就把落地动画收干净，别留个定时器去改已经卸载的 DOM */
  useEffect(() => () => {
    if (landTimer.current !== null) window.clearTimeout(landTimer.current)
    finishLanding.current = null
  }, [])

  /* 框里的撤销 / 重做：外壳那套被 `modalShortcutGate` 拦着，这里自己听 */
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey)) return
      const k = e.key.toLowerCase()
      if (k === 'z' && !e.shiftKey) {
        e.preventDefault()
        draft.undo()
      } else if ((k === 'z' && e.shiftKey) || k === 'y') {
        e.preventDefault()
        draft.redo()
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [draft])

  const commit = async () => {
    if (busy || !draft.dirty) return
    setBusy(true)
    try {
      const view = await wb.setSectionOrder(sectionId, draft.order)
      onApplied?.(view)
      toasts.push(`「${sectionLabel}」的组内顺序已改 —— 撤销可退回`)
      onClose()
    } catch (e) {
      toasts.push(isAppError(e) ? e.message : String(e))
    } finally {
      setBusy(false)
    }
  }

  const order = draft.order

  /**
   * 落点 = 起点下标 + 「相对起点挪了几行」（四舍五入）。
   *
   * ★ 2026-11-14 作者：「如果我在原本的位置，我希望能停在原本的位置呀，
   *   你现在这个强制的帮我吸附到下一个位置」。
   *
   * 原来按**行的中线**判：起手点在整行中间，指针只要落在自己那行的下半，
   * `indexAt` 就直接返回 `i+1` —— 按下不动也被判成"要挪到下一格"。
   * 现在按**位移**算，没挪过半行就是 `from`：原位松手原样不动，
   * 这是天然的死区，不需要额外开关。
   */
  const overFrom = (live: NonNullable<typeof dragLive.current>, clientY: number): number => {
    const scroll = listRef.current?.scrollTop ?? 0
    const dy = clientY - live.startY + (scroll - live.startScroll)
    const steps = Math.round(dy / ROW)
    return Math.min(order.length - 1, Math.max(0, live.from + steps))
  }

  /** 拖到边上自动滚 —— 一屏放不下时，不滚就等于拖不过去 */
  const autoScroll = (y: number) => {
    const el = listRef.current
    if (el === null) return
    const r = el.getBoundingClientRect()
    if (y < r.top + 24) el.scrollTop -= 10
    else if (y > r.bottom - 24) el.scrollTop += 10
  }

  /*
   * **视觉状态的唯一来源**：拖动中与落地动画中都要按同一份 `from/over` 画，
   * 否则松手那一瞬间 `drag` 置空、让位立刻弹回，等落地动画走完又重排一次 ——
   * 作者说的「放下去排序好像有点乱」就是这两帧不同步。
   */
  const visual =
    drag ?? (landing === null ? null : { from: landing.from, over: landing.over })

  /**
   * 拖动 / 落地中显示的是**松手之后的位次**，不是各自原来的下标。
   * 否则行让位了、数字还写着 1/3/4/2/6，人会以为排序乱了。
   */
  const previewAt = (i: number): number => {
    if (visual === null) return i + 1
    const { from, over } = visual
    if (i === from) return over + 1
    if (from < over && i > from && i <= over) return i
    if (from > over && i < from && i >= over) return i + 2
    return i + 1
  }

  /** 其余行让位：被拖的行与落点之间的每一行，整体挪一格 */
  const shiftOf = (i: number): number => {
    if (visual === null || i === visual.from) return 0
    const { from, over } = visual
    if (from < over && i > from && i <= over) return -ROW
    if (from > over && i < from && i >= over) return ROW
    return 0
  }

  const startDrag = (e: ReactPointerEvent<HTMLDivElement>, i: number) => {
    /* 数字框 / ↑↓ 自己都有各自的活，别被整行拖抢走 */
    const tag = (e.target as HTMLElement).tagName
    if (tag === 'INPUT' || tag === 'BUTTON') return
    /*
     * **别把文字拖选出来**（作者实测：拖一下，"回抽速度 toolhead.MKP_…" 整片变蓝）。
     * `user-select: none` 只管得住静态，pointerdown 的默认动作仍会开一个选区；
     * 这里 preventDefault + 清掉已有选区 + 拖动期间整页禁选，三层一起才干净。
     */
    e.preventDefault()
    window.getSelection()?.removeAllRanges()
    document.body.style.userSelect = 'none'
    /* 上一次落地还没走完又开始拖：先把上一笔收干净，别留半段动画和旧 transform */
    if (finishLanding.current !== null) {
      finishLanding.current()
      finishLanding.current = null
    }
    const el = e.currentTarget
    el.setPointerCapture(e.pointerId)
    /* 只记起点，**不进拖拽态**：点一下 / 手抖都不该把行浮起来、把别人挪开 */
    dragLive.current = {
      from: i,
      over: i,
      startY: e.clientY,
      startScroll: listRef.current?.scrollTop ?? 0,
      started: false,
      el,
    }
  }

  const moveDrag = (e: ReactPointerEvent<HTMLDivElement>) => {
    const live = dragLive.current
    if (live === null) return
    /*
     * ① 跟手：直接写 transform，不等 React 渲染。
     *    **只动 Y**：原来那个 `scale(1.03)` 会把行撑宽 3%，左右边缘跟着动，
     *    看着就像"左右也在拖"（作者：「有点左右拖动的感觉」）。悬浮感改由
     *    实底绿带 + 描边 + 阴影给，尺寸一个像素都不变。
     */
    const dy = e.clientY - live.startY + ((listRef.current?.scrollTop ?? 0) - live.startScroll)
    if (!live.started) {
      if (Math.abs(dy) < DRAG_SLOP) return
      live.started = true
      setDrag({ from: live.from, over: live.from })
    }
    live.el.style.transform = `translate3d(0, ${dy}px, 0)`
    /* ② 落点：只当下标变了才 setState，其余行才有机会平滑让位 */
    autoScroll(e.clientY)
    const over = overFrom(live, e.clientY)
    if (over !== live.over) {
      live.over = over
      setDrag({ from: live.from, over })
    }
  }

  const endDrag = () => {
    const live = dragLive.current
    if (live === null) return
    dragLive.current = null
    document.body.style.userSelect = ''
    /* 没真拖过：一次点击，什么都不做（顺序、确认按钮的状态都保持原样） */
    if (!live.started) return

    /*
     * 落地：浮起行现在在 `from × 行高 + dy`，落点是 `over × 行高`。
     * 让它带着过渡直接滑到两者之差，**滑到位之前一行 DOM 都不重排**；
     * 其余行保持让位状态不动（它们已经在新位置上，只是还没写进 order）。
     */
    const dy = (live.over - live.from) * ROW
    setDrag(null)
    setLanding({ from: live.from, over: live.over, dy })
    /* 下一帧再改 transform：同一帧里改的话浏览器会把「起始值」也吃掉，过渡不触发 */
    const el = live.el
    window.requestAnimationFrame(() => {
      el.dataset.landing = 'true'
      el.style.transform = `translate3d(0, ${dy}px, 0)`
    })

    const finish = () => {
      if (landTimer.current !== null) {
        window.clearTimeout(landTimer.current)
        landTimer.current = null
      }
      finishLanding.current = null
      /*
       * 动画走完，一次性重排。
       *
       * ★ 这一帧必须**整列关掉 transform 过渡**：`.row` 平时带 140ms，
       * 清掉 `translateY(30px)` 让位值的同时 DOM 又重排，浏览器会把这两件事
       * 分开算 —— 行先按旧 transform 停一下，再滑回新布局，正是作者说的
       * 「从上往下落 / 排序有点乱」。`data-snap` 只在这一帧挂着。
       */
      const list = listRef.current
      list?.setAttribute('data-snap', 'true')
      delete el.dataset.landing
      el.style.transform = ''
      setLanding(null)
      if (live.over !== live.from) draft.move(live.from, live.over)
      window.requestAnimationFrame(() => {
        window.requestAnimationFrame(() => {
          list?.removeAttribute('data-snap')
        })
      })
    }
    finishLanding.current = finish
    landTimer.current = window.setTimeout(finish, 170)
  }

  return (
    <ModalC14
      open
      title={`组内顺序 · ${sectionLabel}`}
      subtitle={`共 ${order.length} 项 · 拖动、改数字、按 Alt+↑↓ 都行`}
      size="md"
      closeOnScrim={false}
      onClose={onClose}
      headerExtra={
        <div className={s.headTools}>
          <button
            type="button"
            className={s.iconBtn}
            title="撤销（Cmd/Ctrl+Z）"
            aria-label="撤销排序"
            disabled={!draft.canUndo}
            onClick={draft.undo}
          >
            <svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
              <path d="M3 7h6.5a3.5 3.5 0 0 1 0 7H6" />
              <path d="m6 4-3 3 3 3" />
            </svg>
          </button>
          <button
            type="button"
            className={s.iconBtn}
            title="重做（Cmd/Ctrl+Shift+Z）"
            aria-label="重做排序"
            disabled={!draft.canRedo}
            onClick={draft.redo}
          >
            <svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
              <path d="M13 7H6.5a3.5 3.5 0 0 0 0 7H10" />
              <path d="m10 4 3 3-3 3" />
            </svg>
          </button>
        </div>
      }
      footLead={
        <button
          type="button"
          className={s.linkBtn}
          disabled={!draft.dirty}
          onClick={draft.reset}
        >
          恢复
        </button>
      }
      footer={
        <>
          <span className={s.footNote}>顺序单独生效，不用等这一条的定义确认</span>
          <button type="button" className={s.btn} onClick={onClose}>
            取消
          </button>
          <button
            type="button"
            className={`${s.btn} ${s.btnPrimary}`}
            disabled={!draft.dirty || busy}
            onClick={() => void commit()}
          >
            {busy ? '保存中…' : '确认'}
          </button>
        </>
      }
    >
      <div className={s.list} ref={listRef} role="list" aria-label={`${sectionLabel}的组内顺序`}>
        {/*
         * 落点占位：其余行让位之后空出来的那一格，用**一整块背景色**填上。
         * 不再在某一行上沿画横线 —— 让位之后行已经不在原位，横线会落在错的缝里。
         */}
        {visual !== null && visual.over !== visual.from && (
          <div data-slot className={s.slot} style={{ top: visual.over * ROW }} aria-hidden />
        )}
        {order.map((key, i) => {
          const row = rowOf(key)
          const isDrag = drag !== null && drag.from === i
          /* 落地动画里那一行也由 JS 直接写 transform，别让 React 的 style 覆盖它 */
          const floating = isDrag || (landing !== null && landing.from === i)
          return (
            <div
              key={key}
              data-row
              role="listitem"
              className={s.row}
              data-drag={floating ? 'true' : undefined}
              /* 拖动 + 落地时只留「浮起的那条」和「落点占位」两块绿，当前项绿带先收掉 */
              data-self={key === currentKey && visual === null ? 'true' : undefined}
              /* 整行可拖（手柄只是给眼睛看的落点），数字框与箭头除外 */
              onPointerDown={(e) => startDrag(e, i)}
              onPointerMove={moveDrag}
              onPointerUp={endDrag}
              onPointerCancel={endDrag}
              style={floating ? undefined : { transform: shiftOf(i) === 0 ? undefined : `translateY(${shiftOf(i)}px)` }}
              onKeyDown={(e) => {
                if (!e.altKey) return
                if (e.key === 'ArrowUp' && i > 0) {
                  e.preventDefault()
                  draft.move(i, i - 1)
                } else if (e.key === 'ArrowDown' && i < order.length - 1) {
                  e.preventDefault()
                  draft.move(i, i + 1)
                }
              }}
            >
              {visual !== null ? (
                <span className={s.posPreview} aria-hidden>{previewAt(i)}</span>
              ) : (
                <input
                  key={`${key}-${i}`}
                  className={s.pos}
                  type="text"
                  inputMode="numeric"
                  aria-label={`${row.label} 的位次`}
                  defaultValue={String(i + 1)}
                  onBlur={(e) => {
                    const n = Number(e.target.value)
                    if (Number.isFinite(n)) draft.setPosition(key, n)
                  }}
                  onKeyDown={(e) => {
                    if (e.key !== 'Enter') return
                    e.preventDefault()
                    const n = Number((e.target as HTMLInputElement).value)
                    if (Number.isFinite(n)) draft.setPosition(key, n)
                    ;(e.target as HTMLInputElement).blur()
                  }}
                />
              )}
              <span className={s.label}>{row.label}</span>
              {row.by !== undefined && <span className={s.by}>{row.by}</span>}
              <span className={s.spacer} />
              <button
                type="button"
                className={s.arrow}
                aria-label={`${row.label} 上移一位`}
                disabled={i === 0}
                onClick={() => draft.move(i, i - 1)}
              >
                <svg viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
                  <path d="m4 10 4-4 4 4" />
                </svg>
              </button>
              <button
                type="button"
                className={s.arrow}
                aria-label={`${row.label} 下移一位`}
                disabled={i === order.length - 1}
                onClick={() => draft.move(i, i + 1)}
              >
                <svg viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
                  <path d="m4 6 4 4 4-4" />
                </svg>
              </button>
              <span
                className={s.handle}
                aria-hidden
              >
                <svg viewBox="0 0 16 16" width="12" height="12" fill="currentColor" aria-hidden>
                  <circle cx="6" cy="4" r="1.1" />
                  <circle cx="10" cy="4" r="1.1" />
                  <circle cx="6" cy="8" r="1.1" />
                  <circle cx="10" cy="8" r="1.1" />
                  <circle cx="6" cy="12" r="1.1" />
                  <circle cx="10" cy="12" r="1.1" />
                </svg>
              </span>
            </div>
          )
        })}
      </div>
    </ModalC14>
  )
}
