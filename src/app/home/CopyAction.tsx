/*
 * 首页那颗「复制后处理脚本」。
 */

import { useEffect, useRef, useState } from 'react'
import s from './CopyAction.module.css'

interface CopyActionProps {
  text: string
  label: string
}

/**
 * 文案恒定的复制按钮：宽高永不跳 —— 尺寸只由原 label 撑出，
 * 「已复制」盖在原位直接显示（纯透明度，文字永不缩放/重采样，永远锐利）。
 *
 * 没有其他任何效果：点击 → 剪贴板 → 「✓ 已复制」显示 1.5s → 还原。
 */
export default function CopyAction({ text, label }: CopyActionProps) {
  const [copied, setCopied] = useState(false)
  const timer = useRef<number | null>(null)

  useEffect(
    () => () => {
      if (timer.current !== null) window.clearTimeout(timer.current)
    },
    [],
  )

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(text)
      setCopied(true)
      if (timer.current !== null) window.clearTimeout(timer.current)
      timer.current = window.setTimeout(() => setCopied(false), 1500)
    } catch {
      setCopied(false)
    }
  }

  return (
    <div className={s.wrap}>
      <button type="button" className={s.btn} data-copied={copied} onClick={copy}>
        <span className={s.label}>{label}</span>
        <span className={s.done} aria-live="polite">
          <svg className={s.tick} viewBox="0 0 12 12" width="11" height="11" aria-hidden="true">
            <path d="M2.2 6.6 5 9.4 9.8 3.2" pathLength={100} />
          </svg>
          已复制
        </span>
      </button>
    </div>
  )
}
