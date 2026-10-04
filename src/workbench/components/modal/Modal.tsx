/*
 * 居中模态框（A34）。任何稿都能用，和 `components/field`、`components/menu` 同级别。
 *
 * # 为什么单独做一个件
 *
 * 作者的原话：「还有很多模态框还没有居中」。散着写的话每处都要自己算居中，
 * 而这个项目的外壳有个陷阱（见下），算错一次就偏一次。一套实现就只有一种对错。
 *
 * # 三条硬约定
 *
 * 1. **居中用 flex，不用 `top:50%` + `translate(-50%,-50%)`。**
 *    后者在奇数像素上会把框摆在 0.5px 处，文字渲染发虚；而且外壳 `.window` 本来就带
 *    transform，再叠一层只会更难查。flex 居中是整数对齐的。
 *
 * 2. **`position: absolute`，绝不用 `fixed`。**
 *    `src/App.module.css` 的 `.window` 有一个永不为 none 的 transform，它是 fixed 的
 *    包含块 —— #33 那两个「点了没反应」的浮层就栽在这上面：量的是视口坐标、
 *    被当成局部坐标用，偏移一百多像素之后再被 overflow 裁掉。
 *
 * 3. **遮罩挂在调用方给的那一层（内容区），不是整页。**
 *    底栏与窗口右下角那个 resize 手柄还得点得着 —— #31 特意调过。
 *    所以这个组件只画 `absolute inset:0` 的遮罩，挂在哪由宿主的定位祖先决定。
 *
 * # 什么时候用它，什么时候用抽屉
 *
 *   模态框    看完就关、或者要当场做个决定（查看详情 / 确认 / 三步向导）
 *   右抽屉    看一眼还想接着改（编辑历史那种，之前定过它要用右抽屉）
 *
 * 判据是「关掉之后还要不要回来」，不是「内容多不多」。
 */

import { useEffect, useRef } from 'react'
import type { KeyboardEvent as ReactKeyboardEvent, ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { modalShortcutGate, modalStack } from './shortcutGate'
import s from './Modal.module.css'

export interface ModalProps {
  open: boolean
  title: string
  /** 标题下面那行小字 */
  subtitle?: string
  /** 底部一条。没有就不画那一条边 */
  footer?: ReactNode
  /**
   * 标题行右侧的空白位（关闭按钮左边）。确认框那种「想在头上放个小切换」的
   * （如生成前确认的「完整 / 对比」）用它，不用各自改外壳。
   */
  headerExtra?: ReactNode
  /**
   * 框的宽度档。`sm` 看一眼就关的（详情）· `md` 要读的（确认 / 表单）· `lg` 向导。
   * 不给自由数值 —— 三档已经够，给了数值各处就会长出十种宽度。
   */
  size?: 'sm' | 'md' | 'lg'
  /**
   * 点遮罩关不关。**有未保存内容的框要传 false** ——
   * 手滑点一下就把人家改的东西丢了是不可接受的。
   */
  closeOnScrim?: boolean
  /**
   * 遮罩挂在哪个 DOM 节点里（C13 交出 `.shellBody`：标题栏以下整层）。
   * 不给就**原地渲染** —— 遮罩盖住哪儿由最近的定位祖先决定（本组件原来的
   * 约定，A/B 轨都靠它）。给了就 `createPortal` 过去，遮罩与居中都跟着
   * 宿主走，与调用方写在哪一层无关。
   */
  host?: HTMLElement | null
  /** 页眉那枚「关闭」的悬停说明 —— 关闭不总是「放弃」，有草稿语义的稿要写清 */
  closeTitle?: string
  /**
   * 开着的时候**外壳的撤销/重做/保存快捷键要不要照常工作**（2026-10-03）。
   * 默认不 —— 框开着 Cmd+Z 却在改遮罩后面的草稿，就是「撤销穿透」；
   * 只给自己的写都进外壳栈的框（G-code 模态框）传 true。
   */
  shellShortcuts?: boolean
  onClose: () => void
  children: ReactNode
}

export default function Modal({
  open,
  title,
  subtitle,
  footer,
  headerExtra,
  size = 'md',
  closeOnScrim = true,
  host = null,
  closeTitle,
  shellShortcuts = false,
  onClose,
  children,
}: ModalProps) {
  const boxRef = useRef<HTMLDivElement>(null)
  /* 关掉之后焦点要还回去，不然键盘用户会掉到文档开头 */
  const returnTo = useRef<HTMLElement | null>(null)
  /** 开框时领的栈序号 —— Esc 只归最上面那层（嵌套框见 modalStack） */
  const stackToken = useRef<number | null>(null)

  const closeRef = useRef(onClose)
  closeRef.current = onClose

  /* 快捷键闸 + 栈序号：开一票、关一票 —— 外壳的 Cmd+Z / Cmd+S 在 blocking() 时装没听见 */
  useEffect(() => {
    if (!open) return
    modalShortcutGate.enter(shellShortcuts)
    const token = modalStack.push()
    stackToken.current = token
    return () => {
      modalShortcutGate.exit(shellShortcuts)
      modalStack.pop(token)
      stackToken.current = null
    }
  }, [open, shellShortcuts])

  useEffect(() => {
    if (!open) {
      const back = returnTo.current
      returnTo.current = null
      /* 元素可能已经不在了（列表重画过），所以要判一下 */
      if (back !== null && document.body.contains(back)) back.focus()
      return
    }
    const active = document.activeElement
    returnTo.current = active instanceof HTMLElement ? active : null
    /* 焦点进框里 —— 打开了却还停在外面，Esc 之外的键都打到背后去了 */
    boxRef.current?.focus()
  }, [open])

  useEffect(() => {
    if (!open) return
    const onKey = (e: globalThis.KeyboardEvent) => {
      if (e.key !== 'Escape') return
      /* 嵌套框只关最上面那层 —— 底下的框连着人家正在改的东西 */
      const token = stackToken.current
      if (token !== null && !modalStack.isTop(token)) return
      closeRef.current()
    }
    document.addEventListener('keydown', onKey)
    return () => document.removeEventListener('keydown', onKey)
  }, [open])

  if (!open) return null

  /** Tab 锁在框内：到最后一个就回到第一个，反之同理 */
  const onKeyDown = (e: ReactKeyboardEvent<HTMLDivElement>) => {
    if (e.key !== 'Tab') return
    const box = boxRef.current
    if (box === null) return
    const able = [
      ...box.querySelectorAll<HTMLElement>(
        'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
      ),
    ]
    if (able.length === 0) return
    const first = able[0]
    const last = able[able.length - 1]
    if (first === undefined || last === undefined) return
    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault()
      last.focus()
      return
    }
    if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault()
      first.focus()
    }
  }

  /** 遮罩那一块。`host` 给了就 portal 过去 —— 其余行为（焦点、Esc、Tab 锁）照旧 */
  const scrim = (
    <div
      className={s.scrim}
      role="presentation"
      onMouseDown={(e) => {
        /* 只认落在遮罩自己身上的那一下 —— 从框里开始拖到外面松手不算 */
        if (closeOnScrim && e.target === e.currentTarget) onClose()
      }}
    >
      <div
        ref={boxRef}
        className={s.box}
        data-size={size}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        tabIndex={-1}
        onKeyDown={onKeyDown}
      >
        <header className={s.head}>
          <span className={s.headText}>
            <strong className={s.title}>{title}</strong>
            {subtitle !== undefined && <span className={s.sub}>{subtitle}</span>}
          </span>
          {headerExtra !== undefined && <div className={s.headExtra}>{headerExtra}</div>}
          <button type="button" className={s.close} title={closeTitle} onClick={onClose}>
            关闭
          </button>
        </header>

        <div className={s.body}>{children}</div>

        {footer !== undefined && <footer className={s.foot}>{footer}</footer>}
      </div>
    </div>
  )

  return host !== null ? createPortal(scrim, host) : scrim
}
