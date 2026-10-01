/*
 * 自绘下拉（#33）。
 *
 * # 它替掉了三个实现
 *
 * #32 有两个下拉：`ChoiceDropdownV032`（密表里的枚举，弹层 fixed）与 `PickerDropdownV032`
 * （机型/版本，弹层 absolute、选项右边多一列小字）。两者是同一件东西的两份代码。
 * 这一稿只有这一个：多出来的那列小字做成 `FieldOption.note`，定位统一交给 `FieldPopover`。
 *
 * # 为什么不用原生 select
 *
 * 系统画的弹层改不了字号、改不了勾选样式，在密表里高度也对不上旁边的步进器。
 * 全仓库一个原生 `<select>` 都没有，这条从 #32 继续。
 *
 * # 箭头
 *
 * #32 用的是字符 `⌄`，它在不同字体下的位置和粗细都不稳。换成描边图形：lucide 的
 * chevron-down 路径（`m6 9 6 6 6-6`），12×12、stroke-width 2 —— 与参考项目同一个字形。
 * 项目没有 lucide-react 依赖，这一稿也不新增，只把路径内联。
 *
 * # 焦点不跟着选项跑
 *
 * 焦点始终留在触发器上，高亮用 `aria-activedescendant` 告诉读屏器。
 * 密表里每行一个下拉，焦点乱跳比不跳难用得多。
 */

import { useEffect, useRef, useState } from 'react'
import type { KeyboardEvent } from 'react'
import FieldPopover from './FieldPopover'
import type { FieldOption } from './types'
import s from './SelectField.module.css'

interface Props {
  label: string
  options: FieldOption[]
  value: string
  disabled?: boolean
  /** cell 档用 sm */
  size?: 'sm' | 'md'
  onChange: (next: string) => void
}

function Chevron({ up }: { up: boolean }) {
  return (
    <svg
      className={up ? s.caretUp : s.caret}
      width="12"
      height="12"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
    >
      <path d="m6 9 6 6 6-6" />
    </svg>
  )
}

export default function SelectField({
  label,
  options,
  value,
  disabled = false,
  size = 'md',
  onChange,
}: Props) {
  const [open, setOpen] = useState(false)
  const [active, setActive] = useState(0)
  const triggerRef = useRef<HTMLButtonElement>(null)
  const listRef = useRef<HTMLUListElement>(null)

  const at = options.findIndex((o) => o.value === value)
  const current = at < 0 ? undefined : options[at]
  const id = `select-${label}`

  /* 高亮项滚到可见 —— 选项多到列表自己滚的时候，键盘走到底下要跟得上 */
  useEffect(() => {
    if (!open) return
    const node = listRef.current?.querySelectorAll('li')[active]
    node?.scrollIntoView({ block: 'nearest' })
  }, [active, open])

  const openList = () => {
    setActive(at < 0 ? 0 : at)
    setOpen(true)
  }

  const close = () => {
    setOpen(false)
    triggerRef.current?.focus()
  }

  const pick = (next: string) => {
    onChange(next)
    setOpen(false)
    triggerRef.current?.focus()
  }

  const onKeyDown = (e: KeyboardEvent) => {
    if (disabled) return
    const last = options.length - 1
    if (e.key === 'Escape') {
      if (open) {
        e.preventDefault()
        close()
      }
      return
    }
    if (e.key === 'Tab') {
      if (open) setOpen(false) // 走到下一个控件时别留一个飘着的列表
      return
    }
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault()
      if (!open) {
        openList()
        return
      }
      const dir = e.key === 'ArrowDown' ? 1 : -1
      setActive((i) => (i + dir + options.length) % options.length)
      return
    }
    if (e.key === 'Home' || e.key === 'End') {
      if (!open) return
      e.preventDefault()
      setActive(e.key === 'Home' ? 0 : last)
      return
    }
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault()
      if (!open) {
        openList()
        return
      }
      const o = options[active]
      if (o !== undefined) pick(o.value)
    }
  }

  return (
    <span className={s.wrap} onKeyDown={onKeyDown}>
      <button
        ref={triggerRef}
        type="button"
        className={disabled ? s.triggerOff : s.trigger}
        data-size={size}
        aria-label={label}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-activedescendant={open ? `${id}-${active}` : undefined}
        disabled={disabled}
        onClick={() => (open ? setOpen(false) : openList())}
      >
        {/* 值不在选项里时照原文显示 —— 注册表里三处枚举的可选值是反推的，不许擅自改成第一项 */}
        <span className={s.text} data-dep={current?.deprecated ? '' : undefined}>
          {current?.label ?? value}
        </span>
        <Chevron up={open} />
      </button>

      {open && (
        <FieldPopover anchor={triggerRef.current} matchWidth onClose={close}>
          <ul ref={listRef} className={s.list} role="listbox" aria-label={label}>
            {options.map((o, i) => {
              const on = o.value === value
              return (
                <li key={o.value} role="presentation">
                  <button
                    type="button"
                    id={`${id}-${i}`}
                    role="option"
                    aria-selected={on}
                    className={i === active ? s.optActive : s.opt}
                    onMouseEnter={() => setActive(i)}
                    onClick={() => pick(o.value)}
                  >
                    <span className={s.check} aria-hidden>
                      {on ? '✓' : ''}
                    </span>
                    {/* 已弃用的那一档划一条线（C13）：与分段选择器同一个信号 */}
                    <span className={s.optLabel} data-dep={o.deprecated ? '' : undefined}>
                      {o.label}
                    </span>
                    {o.note !== undefined && <span className={s.note}>{o.note}</span>}
                  </button>
                </li>
              )
            })}
          </ul>
        </FieldPopover>
      )}
    </span>
  )
}
