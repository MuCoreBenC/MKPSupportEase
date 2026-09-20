import type { ReactNode } from 'react'
import s from './ui.module.css'

export function Segmented({
  options,
  value,
  onChange,
}: {
  options: string[]
  value: string
  onChange: (v: string) => void
}) {
  return (
    <div className={s.seg} role="tablist">
      {options.map((o) => (
        <button
          key={o}
          type="button"
          role="tab"
          aria-selected={o === value}
          className={s.segBtn}
          data-on={o === value}
          onClick={() => onChange(o)}
        >
          {o}
        </button>
      ))}
    </div>
  )
}

export function Badge({
  children,
  tone = 'muted',
}: {
  children: ReactNode
  tone?: 'new' | 'rec' | 'hot' | 'muted' | 'danger'
}) {
  return (
    <span className={s.badge} data-tone={tone}>
      {children}
    </span>
  )
}

export function Compare({ from, to }: { from: string; to: string }) {
  const changed = from !== to
  return (
    <span className={s.compare}>
      {from}
      <span aria-hidden="true">→</span>
      <b className={changed ? s.compareNew : undefined}>{to}</b>
    </span>
  )
}

export function Revert({ original, onClick }: { original: string; onClick: () => void }) {
  return (
    <button type="button" className={s.revert} onClick={onClick} title="还原为原值">
      <span aria-hidden="true">↰</span>
      {original}
    </button>
  )
}

export function Field({
  label,
  paramKey,
  help,
  children,
  extra,
}: {
  label: string
  paramKey?: string
  help?: string
  children: ReactNode
  extra?: ReactNode
}) {
  return (
    <div className={s.field}>
      <div className={s.fieldMain}>
        <span className={s.fieldLabel}>
          {label}
          {help && (
            <i className={s.help} title={help}>
              ?
            </i>
          )}
          {extra}
        </span>
        {paramKey && <span className={s.fieldKey}>{paramKey}</span>}
      </div>
      {children}
    </div>
  )
}

export function NumberField({
  value,
  unit,
  onChange,
}: {
  value: string
  unit?: string
  onChange: (v: string) => void
}) {
  return (
    <span className={s.num}>
      <input
        className={s.numInput}
        type="text"
        inputMode="decimal"
        value={value}
        onChange={(e) => onChange(e.target.value)}
      />
      <span className={s.unit}>{unit ?? ''}</span>
    </span>
  )
}

export function Btn({
  children,
  variant = 'default',
  disabled,
  onClick,
}: {
  children: ReactNode
  variant?: 'default' | 'primary' | 'ghost' | 'danger'
  disabled?: boolean
  onClick?: () => void
}) {
  const cls = [
    s.btn,
    variant === 'primary' ? s.btnPrimary : '',
    variant === 'ghost' ? s.btnGhost : '',
    variant === 'danger' ? s.btnDanger : '',
  ]
    .filter(Boolean)
    .join(' ')
  return (
    <button type="button" className={cls} disabled={disabled} onClick={onClick}>
      {children}
    </button>
  )
}
