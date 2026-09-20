import type { ReactNode } from 'react'
import s from './ui.module.css'

export function Modal({
  title,
  badge,
  wide,
  onClose,
  foot,
  children,
}: {
  title: string
  badge?: ReactNode
  wide?: boolean
  onClose: () => void
  foot?: ReactNode
  children: ReactNode
}) {
  return (
    <div className={s.scrim} role="presentation" onClick={onClose}>
      <div
        className={wide ? `${s.modal} ${s.modalWide}` : s.modal}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        onClick={(e) => e.stopPropagation()}
      >
        <header className={s.modalHead}>
          <h2 className={s.modalTitle}>{title}</h2>
          {badge}
          <button type="button" className={s.btnGhost} aria-label="关闭" onClick={onClose}>
            ×
          </button>
        </header>
        {children}
        {foot && <footer className={s.modalFoot}>{foot}</footer>}
      </div>
    </div>
  )
}

export function Progress({ percent }: { percent: number }) {
  return (
    <span className={s.bar}>
      <span className={s.barFill} style={{ width: `${Math.min(100, Math.max(0, percent))}%` }} />
    </span>
  )
}
