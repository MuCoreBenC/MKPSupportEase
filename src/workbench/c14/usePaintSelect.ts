/*
 * C14 · 列表刷选（paint selection —— 点住拖过去，经过哪行改哪行）
 *
 * 作者第一次验收时说得很清楚（转述）：他要的**不是** Photoshop / Blender 那种
 * 拉一个矩形框的框选，是列表里的连续拖选 ——
 *
 *   按下某一行 → 按住拖 → 鼠标经过哪一行，就把哪一行改掉
 *   不画矩形框；实时生效；向上向下都行；松手结束
 *
 *   按下时这一行**已选中** → 本次拖动沿途**取消**
 *   按下时这一行**没选中** → 本次拖动沿途**加入**
 *
 * 三条实现上的铁律（都是「会乱」的根源，别再省）：
 *
 *   1. **模式在 pointerdown 那一刻定死**，之后统一 ADD / REMOVE。
 *      绝不「每经过一行 toggle 一次」—— 来回拖过同一行会选中/取消/选中地跳。
 *      模式定了之后，同一行来回经过多少次结果都一样。
 *   2. **工作集也从 pointerdown 那一刻的选择出发**，边刷边在其上累积
 *      （经过过的行记在 `painted` 里，重复经过不再动手）。
 *   3. **4px 之内算点击**。没超过就不动手 —— 那一下留给行自己的 onClick
 *      （点一下 = 照旧只翻这一行）；一旦进入刷选，就把紧跟着的 click 吞掉，
 *      否则松手那一下会把起点那行又翻回去。
 *   4. **两次 mousemove 之间沿线补采样**（每 8px 一点）。拖得快时两个事件
 *      之间鼠标能掠过好几行，只看事件落点的那一行，中间的就漏了
 *      —— 作者实测「刷选的速度快一点，好像就没选上」，说的就是这个。
 *   5. **采样 ≠ 提交**。补采样只改本地工作集，**不逐点 setState**（上一版是
 *      每命中一行交一次，实测 13 行 = 13 次 setState = 12 次重渲染、16 列时
 *      掉 13 帧）。提交时机：手停下 120ms，或松手 —— 一次拖选 1～2 次。
 *   6. **实时反馈走 DOM，不走 React**（`preview`）。手势期间那一行该不该亮，
 *      直接改它的 class / `data-on` / `input.checked` —— 手一过就亮，零渲染。
 *      作者：「我拖的时候，左侧那个树状的列表应该实时显示出来」「人家 3D 都
 *      不会说这种卡顿，人家是优化过的，不是这种渲染」。实时与省是两件事，
 *      别拿「省」去换「实时」。
 *
 * 可选项靠 `data-sel` 认，不自管 DOM 清单（渲染出来的列表，再记一份就会打架）；
 * 不想被刷到的东西标 `data-nosel`（矩阵里能编辑的格子：按下是进编辑态，不是刷选）。
 * 命中用 `elementFromPoint` —— 拖到滚动边缘、列表自己滚了，它照样认得指到哪行。
 */

import { useEffect, useRef } from 'react'
import type { RefObject } from 'react'

interface Args {
  /** 容器：刷选只在它里面生效 */
  ref: RefObject<HTMLElement | null>
  /** 全部候选 id —— 只喂键盘的 A / Ctrl+I 用 */
  ids: string[]
  selected: Set<string>
  /** 选中集合变了调它（一次拖选 1～2 次，不是每行一次） */
  onChange: (next: Set<string>) => void
  /**
   * **手势期间的实时反馈**：直接改这一行的 DOM（加/去选中态），
   * 不走 React。作者的原话：「我拖的时候，它左侧的那个树状的列表应该实时显示
   * 出来」「人家 3D 都不会说这种卡顿，人家是优化过的，不是这种渲染」——
   * 实时反馈与重渲染是两件事：反馈交给 DOM，状态留给松手时那一次提交。
   *
   * 传进来的 `el` 是那一行的元素（`[data-sel]`），`on` 是按本次模式算出来的结果。
   */
  preview?: (el: HTMLElement, on: boolean) => void
  /** 关掉就不装监听（单选模式没有「选一批」这件事；树没渲染时也别装） */
  enabled?: boolean
  /** 可选项的标记属性，默认 `data-sel` */
  attr?: string
  /** 按下在这些东西上不进入刷选 */
  skip?: string
}

const SAME = (a: Set<string>, b: Set<string>): boolean => {
  if (a.size !== b.size) return false
  for (const x of a) if (!b.has(x)) return false
  return true
}

export function usePaintSelectC14({
  ref,
  ids,
  selected,
  onChange,
  preview,
  enabled = true,
  attr = 'data-sel',
  /** 勾选框**不排除**：从它上面按下拖过去也刷（不动 = 原样单击勾选） */
  skip = 'select,textarea,a,[data-nosel]',
}: Args): void {
  /*
   * 监听只装一次（省得每帧解绑重绑），所以它们读**最新的**那一份 ——
   * `selected` 每次渲染都在变，闭包里抓到的第一份永远是空的。
   */
  const latest = useRef({ ids, selected, onChange, preview })
  latest.current = { ids, selected, onChange, preview }

  useEffect(() => {
    const el = ref.current
    if (!el || !enabled) return

    /* 鼠标在容器里（或焦点在容器里）时键盘才生效 —— 否则搜索框里打个 a 就全选了 */
    let armed = false
    const arm = () => {
      armed = true
    }
    const disarm = () => {
      armed = false
    }

    let start: { x: number; y: number } | null = null
    /** 上一次采样点 —— 两次事件之间的行靠沿线补采样补齐 */
    let last: { x: number; y: number } | null = null
    let add = true
    /** 本次刷选的**工作集**：从按下那一刻的选择出发，边刷边改 */
    let current: Set<string> = new Set()
    let painted: Set<string> = new Set()
    let active = false
    /* 进入刷选就把紧跟着的那次 click 吃掉（见文件头第 3 条） */
    const swallow = (e: MouseEvent) => {
      e.stopPropagation()
      e.preventDefault()
    }

    /** 这一点落在哪一行上（`elementFromPoint` —— 列表自己滚了也认得） */
    const rowAt = (x: number, y: number): string | null => {
      const t = document.elementFromPoint(x, y)
      return t?.closest(`[${attr}]`)?.getAttribute(attr) ?? null
    }

    /** 按 id 找那一行的元素 —— 实时反馈直接改它 */
    const rowEl = (id: string): HTMLElement | null =>
      el.querySelector<HTMLElement>(`[${attr}="${CSS.escape(id)}"]`)

    /**
     * 把一行按本次模式改掉：**工作集 + 这一行的 DOM**。
     *
     * 注意它**不通知 React** —— 这是这一版最要紧的一条（C14 第十一轮）。
     *
     * 上一版是「每命中一行就 onChange 一次」：实测一次拖选 13 行 = 13 次
     * setState = 12 次矩阵重渲染，16 列时拖一下就掉 13 帧、最长帧 50ms
     * （作者的体感：「第一次正常，第二次开始卡」）。
     *
     * 而作者要的**实时反馈**跟「重渲染」本来就是两件事：
     *
     *   反馈   改这一行的 className / data-on / input.checked —— 手一过就亮
     *   状态   松手（或手停 120ms）提交一次，React 才渲染一次
     *
     * 手指的每一次移动只碰命中过的那几行 DOM，等于没有渲染成本；
     * 提交时 React 用真值重画一遍，画出来跟手势期间看到的一模一样。
     * 这就是作者说的「人家是优化过的，不是这种渲染」。
     */
    const paint = (id: string) => {
      painted.add(id)
      if (add) current.add(id)
      else current.delete(id)
      const node = rowEl(id)
      if (node !== null) latest.current.preview?.(node, add)
    }

    /**
     * 提交工作集（一次拖选 1～2 次）。
     *
     * 松手必交；**手停 120ms 也交一次** —— 慢拖时右侧那些靠状态派生的东西
     * （已选 N 项、批量面板）本来就该跟上，不该等到松手。
     */
    const flush = () => {
      if (SAME(current, latest.current.selected)) return
      latest.current.onChange(new Set(current))
    }

    let flushTimer: number | null = null
    const scheduleFlush = () => {
      if (flushTimer !== null) window.clearTimeout(flushTimer)
      flushTimer = window.setTimeout(() => {
        flushTimer = null
        flush()
      }, 120)
    }
    const stopFlush = () => {
      if (flushTimer !== null) window.clearTimeout(flushTimer)
      flushTimer = null
    }

    function onMove(e: MouseEvent) {
      if (start === null) return
      if (!active) {
        if (Math.abs(e.clientX - start.x) < 4 && Math.abs(e.clientY - start.y) < 4) return
        active = true
        painted = new Set()
        window.addEventListener('click', swallow, true)
      }
      /*
       * **沿「上一次的位置 → 现在」这条线段补采样**（作者实测：刷得快一点
       * 就有行没选上）。mousemove 两个事件之间鼠标可以掠过好几行 —— 只看
       * 事件落点的那一行，中间那些就漏了。每 8px 采一次点，落到哪行刷哪行；
       * 起点那一行也在段上，按着已选中的行往下拖时第一行照样被取消。
       *
       * 采样只改工作集，**不逐点提交** —— 提交的时机在 `scheduleFlush`。
       */
      const from = last ?? start
      const dx = e.clientX - from.x
      const dy = e.clientY - from.y
      const steps = Math.max(1, Math.ceil(Math.hypot(dx, dy) / 8))
      for (let i = 1; i <= steps; i++) {
        const id = rowAt(from.x + (dx * i) / steps, from.y + (dy * i) / steps)
        if (id !== null && !painted.has(id)) paint(id)
      }
      last = { x: e.clientX, y: e.clientY }
      scheduleFlush()
    }

    function onUp() {
      start = null
      last = null
      active = false
      stopFlush()
      flush()
      window.removeEventListener('mousemove', onMove)
      window.removeEventListener('mouseup', onUp)
      /* click 在 mouseup 之后才派发，所以监听器下一个 tick 再摘 */
      window.setTimeout(() => window.removeEventListener('click', swallow, true), 0)
    }

    const onDown = (e: MouseEvent) => {
      if (e.button !== 0) return
      const t = e.target as HTMLElement | null
      if (t?.closest(skip)) return
      /* 不在某一行上按下（空白、分组名）不进入刷选 —— 那里没有可刷的东西 */
      const id = t?.closest(`[${attr}]`)?.getAttribute(attr)
      if (id === null || id === undefined) return
      start = { x: e.clientX, y: e.clientY }
      /* 模式在这里定死：按下的行已选中 → 沿途取消；没选中 → 沿途加入 */
      add = !latest.current.selected.has(id)
      current = new Set(latest.current.selected)
      painted = new Set()
      active = false
      window.addEventListener('mousemove', onMove)
      window.addEventListener('mouseup', onUp)
    }

    const onKey = (e: KeyboardEvent) => {
      if (!armed) return
      const t = e.target as HTMLElement | null
      if (
        t !== null &&
        (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.tagName === 'SELECT' || t.isContentEditable)
      ) {
        return
      }
      const { ids: all, selected: cur, onChange: set } = latest.current
      if (e.key === 'a' || e.key === 'A') {
        /* Ctrl+A 是浏览器的「全选文本」，不抢；只认光杆 A */
        if (e.ctrlKey || e.metaKey || e.altKey) return
        const allOn = all.length > 0 && all.every((id) => cur.has(id))
        e.preventDefault()
        set(allOn ? new Set() : new Set(all))
        return
      }
      if ((e.key === 'i' || e.key === 'I') && (e.ctrlKey || e.metaKey)) {
        e.preventDefault()
        set(new Set(all.filter((id) => !cur.has(id))))
      }
    }

    el.addEventListener('mousedown', onDown)
    el.addEventListener('mouseenter', arm)
    el.addEventListener('mouseleave', disarm)
    el.addEventListener('focusin', arm)
    el.addEventListener('focusout', disarm)
    window.addEventListener('keydown', onKey)
    return () => {
      stopFlush()
      el.removeEventListener('mousedown', onDown)
      el.removeEventListener('mouseenter', arm)
      el.removeEventListener('mouseleave', disarm)
      el.removeEventListener('focusin', arm)
      el.removeEventListener('focusout', disarm)
      window.removeEventListener('keydown', onKey)
      window.removeEventListener('mousemove', onMove)
      window.removeEventListener('mouseup', onUp)
      window.removeEventListener('click', swallow, true)
    }
  }, [ref, enabled, attr, skip])
}
