import { useState } from 'react'

import { useT } from '../../i18n'
import s from './TraceTag.module.css'

interface TraceTagProps {
  /** 这次失败在日志里的编号。`-` 表示这条错误没经过 Rust 的包装层 */
  traceId: string
}

/**
 * 错误旁边那行小字：traceId，点一下复制。
 *
 * 为什么要露出来：界面上的一句「连接失败」对排查毫无帮助，而日志里那一条有完整上下文。
 * 两者之间唯一的桥就是这个 id —— 不显示它，等于日志白写了。
 * 做成可点复制是因为它必然要被粘到别处（聊天窗口、issue）才有用，让人手抄 36 位 uuid 不现实。
 */
export default function TraceTag({ traceId }: TraceTagProps) {
  const [copied, setCopied] = useState(false)
  const tr = useT()

  if (!traceId || traceId === '-') return null

  const copy = () => {
    navigator.clipboard?.writeText(traceId).then(
      () => {
        setCopied(true)
        window.setTimeout(() => setCopied(false), 1200)
      },
      () => setCopied(false),
    )
  }

  return (
    <button type="button" className={s.tag} onClick={copy} data-tip={tr('trace.copy')}>
      {copied ? tr('trace.copied') : `#${traceId.slice(0, 8)}`}
    </button>
  )
}
