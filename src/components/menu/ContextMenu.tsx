/*
 * 通用右键菜单（A34）。任何列表都能用，这一轮先用在用户端的预设页。
 *
 * # 一律 absolute，绝不用 fixed
 *
 * 预览外壳 `src/App.module.css` 的 `.window` 上有一个永不为 none 的 transform，
 * 它让 `.window` 成了 fixed 的包含块 —— #33 那两个「点了没反应」的浮层就栽在这上面：
 * 量的是视口坐标、被当成局部坐标用，偏移一百多像素之后再被 overflow 裁掉。
 * 所以坐标一律换算成**层内坐标**：`left = point.x - layer.left`。
 *
 * # 复用 FieldLayer，不自己造一个层
 *
 * `components/field` 的 `FieldLayer` 已经解决了三件事：portal 的落点、同层只留一个浮层、
 * 层自己的尺寸（翻边的判据）。再写一个层就等于同一件事有两套实现 ——
 * #33 定的「控件只能一套」对浮层同样成立。代价是宿主要包一层 `<FieldLayer>`，它本来就包了。
 *
 * # 量完才显形
 *
 * 翻边要先知道菜单自己多高，所以第一帧画在 (0,0) 且 visibility: hidden，量完再定位。
 *
 * # 二次确认长在菜单里
 *
 * 不另开一个对话框：确认层就在菜单的位置上，替换掉列表。理由是删除这件事发生在**某一行**上，
 * 把问句抬到屏幕正中会丢掉「问的是哪一行」这个上下文。
 * 确认层上**点外面不关** —— 不可逆的事必须显式选一个。
 */

import { useEffect, useLayoutEffect, useRef, useState } from 'react'
import type { KeyboardEvent as ReactKeyboardEvent } from 'react'
import { createPortal } from 'react-dom'
import { useFieldLayer } from '../field'
import { isSeparator } from './types'
import type { ContextMenuEntry, ContextMenuItem, ContextMenuPoint } from './types'
import s from './ContextMenu.module.css'

interface Props {
  /** null = 不画。视口坐标，由 `useContextMenu` 给 */
  at: ContextMenuPoint | null
  entries: ContextMenuEntry[]
  onClose: () => void
}

/** 贴边的安全边距。和 FieldPopover 用同一个数 */
const EDGE = 6

export default function ContextMenu({ at, entries, onClose }: Props) {
  const layer = useFieldLayer()
  const ref = useRef<HTMLDivElement>(null)
  const [box, setBox] = useState<{ left: number; top: number } | null>(null)
  const [confirming, setConfirming] = useState<ContextMenuItem | null>(null)

  /* onClose 每帧都是新函数，登记/卸载不该跟着重跑 */
  const closeRef = useRef(onClose)
  closeRef.current = onClose

  /* 关掉之后焦点要还回去，不然键盘用户会掉到文档开头 */
  const returnTo = useRef<HTMLElement | null>(null)

  const open = at !== null

  /* 同层只留一个浮层：登记自己，顺手关掉上一个（下拉开着时右键，下拉该消失） */
  useEffect(() => {
    if (!open) return
    return layer.open(() => closeRef.current())
  }, [layer, open])

  useEffect(() => {
    if (!open) {
      setConfirming(null)
      setBox(null)
      const back = returnTo.current
      returnTo.current = null
      /* 元素可能已经不在了（切了机型、列表重画），所以要判一下 */
      if (back !== null && document.body.contains(back)) back.focus()
      return
    }
    const active = document.activeElement
    returnTo.current = active instanceof HTMLElement ? active : null
  }, [open])

  const items = entries.filter((e): e is ContextMenuItem => !isSeparator(e))
  const enabled = items.filter((i) => i.disabled === undefined)

  useLayoutEffect(() => {
    const el = ref.current
    const host = layer.el
    if (el === null || host === null || at === null) return
    const l = host.getBoundingClientRect()
    const w = el.offsetWidth
    const h = el.offsetHeight

    /* 右边放不下就往左开，下面放不下就往上开 —— 翻边的判据是**层**的尺寸，不是视口 */
    let left = at.x - l.left
    if (left + w > l.width - EDGE) left = at.x - l.left - w
    left = Math.max(EDGE, Math.min(left, Math.max(EDGE, l.width - EDGE - w)))

    let top = at.y - l.top
    if (top + h > l.height - EDGE) top = at.y - l.top - h
    top = Math.max(EDGE, Math.min(top, Math.max(EDGE, l.height - EDGE - h)))

    setBox({ left, top })
  }, [at, layer.el, confirming, entries])

  /* 开着的时候滚动就关掉（跟着走要每帧重算，而停在错的位置是撒谎）。确认层不受滚动影响 */
  useEffect(() => {
    if (!open) return
    const onDown = (e: globalThis.MouseEvent) => {
      if (ref.current?.contains(e.target as Node) === true) return
      /* 确认层上点外面不关 —— 不可逆的事要显式选 */
      if (confirming !== null) return
      closeRef.current()
    }
    const onScroll = () => {
      if (confirming !== null) return
      closeRef.current()
    }
    window.addEventListener('mousedown', onDown)
    window.addEventListener('scroll', onScroll, true)
    window.addEventListener('resize', onScroll)
    return () => {
      window.removeEventListener('mousedown', onDown)
      window.removeEventListener('scroll', onScroll, true)
      window.removeEventListener('resize', onScroll)
    }
  }, [confirming, open])

  /* 开了就把焦点放到第一个能点的项上 —— 键盘用户打开菜单是为了马上用键盘选 */
  useEffect(() => {
    if (!open || box === null) return
    const first = ref.current?.querySelector<HTMLButtonElement>('button[data-on="1"]')
    first?.focus()
  }, [box, open, confirming])

  if (!open || layer.el === null) return null

  const pick = (item: ContextMenuItem) => {
    if (item.disabled !== undefined) return
    if (item.confirm !== undefined) {
      setConfirming(item)
      return
    }
    onClose()
    item.onSelect()
  }

  /** ↑↓ 在能点的项之间走（禁用项和分隔线都跳过），Esc 关，Tab 不陷进去 */
  const onKeyDown = (e: ReactKeyboardEvent<HTMLDivElement>) => {
    if (e.key === 'Escape') {
      e.preventDefault()
      onClose()
      return
    }
    if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp') return
    e.preventDefault()
    const btns = [...(ref.current?.querySelectorAll<HTMLButtonElement>('button[data-on="1"]') ?? [])]
    if (btns.length === 0) return
    const now = btns.findIndex((b) => b === document.activeElement)
    const step = e.key === 'ArrowDown' ? 1 : -1
    const next = now === -1 ? 0 : (now + step + btns.length) % btns.length
    btns[next]?.focus()
  }

  const body =
    confirming === null ? (
      <ul className={s.list} role="menu">
        {entries.map((entry, i) =>
          isSeparator(entry) ? (
            /* 分隔线没有 id，位置就是它的身份 */
            <li key={`sep-${String(i)}`} className={s.sep} role="separator" />
          ) : (
            <li key={entry.id} className={s.row} role="none">
              <button
                type="button"
                role="menuitem"
                className={entry.danger === true ? s.danger : s.item}
                data-on={entry.disabled === undefined ? '1' : '0'}
                disabled={entry.disabled !== undefined}
                title={entry.disabled}
                tabIndex={-1}
                onClick={() => pick(entry)}
              >
                {entry.label}
              </button>
            </li>
          ),
        )}
      </ul>
    ) : (
      <div className={s.ask} role="dialog" aria-label={confirming.confirm?.question ?? '确认'}>
        <p className={s.askHead}>{confirming.confirm?.question}</p>
        {confirming.confirm?.detail !== undefined && (
          <p className={s.askBody}>{confirming.confirm.detail}</p>
        )}
        <div className={s.askActs}>
          <button type="button" className={s.cancel} data-on="1" tabIndex={-1} onClick={onClose}>
            取消
          </button>
          <button
            type="button"
            className={s.ok}
            data-on="1"
            tabIndex={-1}
            onClick={() => {
              const go = confirming.onSelect
              onClose()
              go()
            }}
          >
            {confirming.confirm?.ok ?? '删除'}
          </button>
        </div>
      </div>
    )

  return createPortal(
    <div
      ref={ref}
      className={s.pop}
      data-ready={box !== null}
      style={box === null ? undefined : { left: box.left, top: box.top }}
      onKeyDown={onKeyDown}
    >
      {/* 一个能点的都没有时说一句，别给一个空框 */}
      {confirming === null && enabled.length === 0 && items.length > 0 && (
        <p className={s.none}>这一行没有可用的操作</p>
      )}
      {body}
    </div>,
    layer.el,
  )
}
