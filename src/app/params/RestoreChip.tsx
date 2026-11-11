import type { ReactNode } from 'react'

import s from './RestoreChip.module.css'

interface Props {
  /** hover 提示：说清「还原成什么」 */
  title?: string
  disabled?: boolean
  onClick: () => void
  children: ReactNode
}

/**
 * 「↩ xxx」这类还原 chip 的**唯一实现**。
 *
 * 为什么值得抽出来：`↩` 的字形框天生偏上，和中文的视觉中线差 2px。
 * 之前 `GcodeBlock` 与 `ParamRow` 各写一遍（`↩ {text}`），于是只有一处调过、
 * 另一处继续偏上 —— 作者的原话是「别的地方也要一样呀，统一的、复用的」。
 *
 * 所以这里把三件事定死：图标与文字分开渲染、按钮自身 `inline-flex` 居中、
 * 图标单独下移 2px。以后无论谁再加一处还原 chip，用这个组件就自动对齐。
 */
export default function RestoreChip({ title, disabled = false, onClick, children }: Props) {
  return (
    <button
      type="button"
      className={s.chip}
      data-tip={title}
      disabled={disabled}
      onClick={onClick}
    >
      <span className={s.icon} aria-hidden="true">
        <svg width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden="true"><path d="M2.8 4.6h4.6a2.3 2.3 0 1 1 0 4.6H5.4" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round"/><path d="M5 2.5 2.8 4.6 5 6.7" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round"/></svg>
      </span>
      {children}
    </button>
  )
}
