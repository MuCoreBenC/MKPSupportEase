/*
 * 带清空叉的搜索框。这一页有两个搜索框（tabs 行的「搜参数」、抽屉里的「搜预设名」），
 * 抽成一个组件是为了**两个叉长得一样** —— 各写一遍迟早会分叉。
 *
 * # 为什么要自己画那个叉
 *
 * `<input type="search">` 在 Chromium / WebView2 里自带一个
 * `::-webkit-search-cancel-button`。那是 **UA 样式里的位图**：
 *   - 形状、粗细、颜色全归浏览器，`--bbs-*` 一个都管不到
 *   - 深色皮肤下它还是那个灰位图，跟周围格格不入
 *   - 换平台就变样（Safari 圆底叉、Firefox 干脆没有）
 * 所以藏掉它，自己用两条 1.2px 的直线画一个 —— 细线交叉，跟着 currentColor 走。
 *
 * `type` 仍然留着 `search`：它带的语义（清空的 Esc、语音输入、历史）是有用的，
 * 只是那个按钮不要。
 */

import { useRef } from 'react'
import s from './BbsSearchBox.module.css'

interface Props {
  value: string
  placeholder: string
  /** 无障碍名，两个框各自不同（「搜参数」/「搜预设名」） */
  label: string
  /** 窄一点的那一档（抽屉里那个是满宽，tabs 行那个固定宽） */
  variant?: 'fill' | 'fixed'
  onChange: (next: string) => void
}

export default function BbsSearchBox({
  value,
  placeholder,
  label,
  variant = 'fixed',
  onChange,
}: Props) {
  const inputRef = useRef<HTMLInputElement>(null)

  const clear = () => {
    onChange('')
    /* 清完把焦点还回输入框：不然你得再点一下才能接着输 */
    inputRef.current?.focus()
  }

  return (
    <span className={s.box} data-variant={variant}>
      <input
        ref={inputRef}
        className={s.input}
        type="search"
        value={value}
        placeholder={placeholder}
        aria-label={label}
        onChange={(ev) => onChange(ev.target.value)}
        onKeyDown={(ev) => {
          /* Esc 清空。stopPropagation 是必须的：浮层态下 Esc 是「关抽屉」，
             焦点在框里时那一层不该抢 */
          if (ev.key === 'Escape') { ev.stopPropagation(); clear() }
        }}
      />
      {value && (
        <button type="button" className={s.clear} aria-label={`清空${label}`} onClick={clear}>
          {/* 两条 1.2px 的圆头直线。颜色走 currentColor，由 CSS 给 */}
          <svg viewBox="0 0 12 12" aria-hidden="true">
            <path
              d="M3.2 3.2 L8.8 8.8 M8.8 3.2 L3.2 8.8"
              stroke="currentColor"
              strokeWidth="1.2"
              strokeLinecap="round"
              fill="none"
            />
          </svg>
        </button>
      )}
    </span>
  )
}
