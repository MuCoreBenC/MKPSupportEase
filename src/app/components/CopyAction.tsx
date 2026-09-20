import { useEffect, useRef, useState } from 'react'
import s from './CopyAction.module.css'

interface CopyActionProps {
  text: string
  label: string
}

/** 文案恒定的复制按钮：宽度不跳，右侧侧滑淡入一个绿色「已复制」提示 */
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
      <button type="button" className={s.btn} onClick={copy}>
        {label}
      </button>
      <span className={s.hint} data-on={copied} aria-live="polite">
        已复制
      </span>
    </div>
  )
}
